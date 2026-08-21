# The only sanctioned way to commit and push in this repository.
#
# Every rule this script enforces has cost a session at least once. They are
# written down in TODO/RULES.md section 4; this file is what makes them
# mechanical instead of remembered.
#
# What it enforces:
#
#   1. Author and committer are Azathothas <AjamX101@gmail.com>, set per
#      invocation with `-c`, so a machine with different global config still
#      produces the right commits.
#   2. No AI attribution: no Co-Authored-By naming a model or tool, no
#      "generated with" line, no tool name in the body. Refused, not stripped,
#      because silently editing a commit message is worse than refusing one.
#   3. Nothing under reference/ reaches main, even with -Force.
#   4. The gates run before the push, not after.
#   5. The corpus is pushed to the `references` branch so it survives a lost
#      machine without entering main's history.
#
# Usage:
#
#   # stage everything, commit, run the gates, push, sync the corpus
#   pwsh -NoProfile -File scripts/git-sync.ps1 -Message "Subject" -Body "..."
#
#   # commit only, no push
#   pwsh -NoProfile -File scripts/git-sync.ps1 -Message "Subject" -NoPush
#
#   # push what is already committed
#   pwsh -NoProfile -File scripts/git-sync.ps1 -PushOnly
#
#   # stage specific paths rather than everything
#   pwsh -NoProfile -File scripts/git-sync.ps1 -Message "Subject" -Path README.md,TODO/INDEX.md
#
#   # add one benchmark that IS the evidence for an entry
#   pwsh -NoProfile -File scripts/git-sync.ps1 -Message "Subject" -Evidence bench/soak-20260821T012428252Z.json
#
#   # check the rules without doing anything
#   pwsh -NoProfile -File scripts/git-sync.ps1 -Check
#
#   # get reference/ onto a fresh clone from the references branch
#   pwsh -NoProfile -File scripts/git-sync.ps1 -FetchReferences
#
# Exit codes: 0 all good, 1 a rule was broken or a gate failed, 2 the script
# could not run (not a git repository, no message, git missing).
#
# See TODO/RULES.md.

[CmdletBinding()]
param(
    # Commit subject. Required unless -PushOnly, -Check or -FetchReferences.
    [string]$Message,

    # Commit body. Blank line inserted between subject and body.
    [string]$Body,

    # Paths to stage. Default is everything tracked and untracked, minus
    # whatever .gitignore excludes.
    [string[]]$Path,

    # Paths to force-add past .gitignore. For a benchmark that IS the evidence
    # for a TODO entry. Refused for anything under reference/.
    [string[]]$Evidence,

    # Commit but do not push.
    [switch]$NoPush,

    # Push what is already committed. No staging, no commit.
    [switch]$PushOnly,

    # Run every check and report, change nothing.
    [switch]$Check,

    # Restore reference/ from the references branch into the working tree.
    [switch]$FetchReferences,

    # Skip the gates. For a documentation-only change where the tree is known
    # green. Recorded in the output so it is visible in a transcript.
    [switch]$SkipGates,

    # Do not sync the corpus to the references branch on this push.
    [switch]$NoReferences,

    [string]$Branch = "main",
    [string]$ReferenceBranch = "references"
)

$ErrorActionPreference = 'Stop'

# `git` writes progress to stderr on success, and
# $PSNativeCommandUseErrorActionPreference is false by default from pwsh 7.4,
# so stderr alone never terminates. Every git call below is checked on
# $LASTEXITCODE instead.
$script:RepoRoot = Split-Path -Parent $PSScriptRoot

$AuthorName = "Azathothas"
$AuthorEmail = "AjamX101@gmail.com"

function Write-Step([string]$text) {
    $stamp = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ss.fffZ")
    Write-Host "$stamp git-sync: $text"
}

function Exit-With([int]$code, [string]$text) {
    [Console]::Error.WriteLine("git-sync: $text")
    exit $code
}

# Run git and return its stdout. Terminates on a non-zero exit unless
# -AllowFail. Named $gitArgs rather than $args, because $args inside a function
# is an automatic variable that silently swallows a parameter of that name.
function Invoke-Git {
    param([string[]]$gitArgs, [switch]$AllowFail)
    $output = & git @gitArgs 2>&1
    $code = $LASTEXITCODE
    if ($code -ne 0 -and -not $AllowFail) {
        Exit-With 1 "git $($gitArgs -join ' ') failed with exit $code`n$output"
    }
    $script:LastGitExit = $code
    return ($output | Out-String)
}

# Same, with the identity pinned. Author and committer both, because
# `git commit --author` sets only the author.
function Invoke-GitAs {
    param([string[]]$gitArgs)
    $prefix = @(
        "-c", "user.name=$AuthorName",
        "-c", "user.email=$AuthorEmail",
        "-c", "committer.name=$AuthorName",
        "-c", "committer.email=$AuthorEmail"
    )
    return (Invoke-Git -gitArgs ($prefix + $gitArgs))
}

Set-Location $script:RepoRoot
Invoke-Git -gitArgs @("rev-parse", "--git-dir") | Out-Null

# ---------------------------------------------------------------------------
# Rule 2: no AI attribution
# ---------------------------------------------------------------------------
#
# -cmatch throughout where case is not the signal we want to ignore. These are
# case-insensitive on purpose: "co-authored-by" and "Co-Authored-By" are the
# same violation. The tool names are matched with word boundaries so a legitimate
# sentence mentioning a file called `claude.rs` would not trip, but an
# attribution line would.
$AttributionPatterns = @(
    '(?im)^\s*co-authored-by:',
    '(?i)generated\s+with\s+\[?claude',
    '(?i)\bgenerated\s+by\s+(claude|chatgpt|gpt-|copilot|cursor|codex|gemini|llm|an?\s+ai\b)',
    '(?i)\bwritten\s+by\s+(claude|chatgpt|gpt-|copilot|an?\s+ai\b)',
    '(?i)\bwith\s+assistance\s+from\s+(claude|chatgpt|copilot|an?\s+ai\b)',
    '(?i)\bclaude\s+(code|opus|sonnet|haiku)\b',
    '(?i)\banthropic\b',
    '(?i)^\s*(assisted|authored)-by:\s*(claude|chatgpt|copilot)',
    '(?i)\bnoreply@anthropic\.com\b',
    '(?i)🤖'
)

function Test-Attribution([string]$text) {
    $hits = @()
    foreach ($pattern in $AttributionPatterns) {
        if ($text -match $pattern) { $hits += $Matches[0].Trim() }
    }
    return $hits
}

# ---------------------------------------------------------------------------
# Rule 3: nothing under reference/ reaches main
# ---------------------------------------------------------------------------

function Get-ForbiddenStaged {
    $staged = (Invoke-Git -gitArgs @("diff", "--cached", "--name-only")) -split "`r?`n" |
        Where-Object { $_ -and $_.Trim() }
    return @($staged | Where-Object { $_ -match '^reference/' -or $_ -eq 'reference' })
}

# ---------------------------------------------------------------------------
# -FetchReferences
# ---------------------------------------------------------------------------

if ($FetchReferences) {
    Write-Step "fetching $ReferenceBranch from origin"
    Invoke-Git -gitArgs @("fetch", "origin", "${ReferenceBranch}:refs/remotes/origin/$ReferenceBranch") -AllowFail | Out-Null
    if ($script:LastGitExit -ne 0) {
        Exit-With 1 "origin has no '$ReferenceBranch' branch yet. Push one first with a normal run of this script."
    }
    Write-Step "restoring reference/ into the working tree"
    Invoke-Git -gitArgs @("checkout", "origin/$ReferenceBranch", "--", "reference") | Out-Null
    # `git checkout <ref> -- <path>` stages what it restores. reference/ must
    # never be staged on main, so unstage it immediately and leave the files.
    Invoke-Git -gitArgs @("reset", "--", "reference") -AllowFail | Out-Null
    $count = (Get-ChildItem -Recurse -File -Path (Join-Path $script:RepoRoot "reference") -ErrorAction SilentlyContinue).Count
    Write-Step "reference/ restored, $count files, unstaged"
    exit 0
}

# ---------------------------------------------------------------------------
# -Check
# ---------------------------------------------------------------------------

if ($Check) {
    $problems = 0

    $forbidden = Get-ForbiddenStaged
    if ($forbidden.Count -gt 0) {
        [Console]::Error.WriteLine("git-sync: staged paths under reference/: $($forbidden -join ', ')")
        $problems++
    }
    else { Write-Step "no staged path under reference/" }

    if ($Message) {
        $hits = Test-Attribution "$Message`n$Body"
        if ($hits.Count -gt 0) {
            [Console]::Error.WriteLine("git-sync: attribution in the message: $($hits -join '; ')")
            $problems++
        }
        else { Write-Step "message carries no attribution" }
    }

    # The last commit on this branch, so a bad one that landed some other way
    # is still caught.
    $subject = (Invoke-Git -gitArgs @("log", "-1", "--pretty=%an <%ae>%n%B")).Trim()
    $hits = Test-Attribution $subject
    if ($hits.Count -gt 0) {
        [Console]::Error.WriteLine("git-sync: HEAD commit carries attribution: $($hits -join '; ')")
        $problems++
    }
    else { Write-Step "HEAD commit is clean" }

    $who = (Invoke-Git -gitArgs @("log", "-1", "--pretty=%an <%ae>|%cn <%ce>")).Trim()
    $expected = "$AuthorName <$AuthorEmail>"
    if ($who -ne "$expected|$expected") {
        [Console]::Error.WriteLine("git-sync: HEAD identity is '$who', expected '$expected|$expected'")
        $problems++
    }
    else { Write-Step "HEAD identity is $expected, author and committer" }

    if ($problems -gt 0) { exit 1 }
    Write-Step "all checks pass"
    exit 0
}

# ---------------------------------------------------------------------------
# The gates
# ---------------------------------------------------------------------------

function Invoke-Gates {
    if ($SkipGates) {
        Write-Step "GATES SKIPPED by -SkipGates. The push carries no proof the tree is green."
        return
    }

    Write-Step "cargo fmt --all --check"
    & cargo fmt --all --check
    if ($LASTEXITCODE -ne 0) { Exit-With 1 "cargo fmt --all --check failed. Run 'cargo fmt --all'." }

    Write-Step "cargo clippy --workspace --all-targets --all-features -- -D warnings"
    & cargo clippy --workspace --all-targets --all-features -- -D warnings
    if ($LASTEXITCODE -ne 0) { Exit-With 1 "clippy failed." }

    Write-Step "cargo test --workspace"
    $testLog = Join-Path ([System.IO.Path]::GetTempPath()) "bit-cli-git-sync-tests.txt"
    & cargo test --workspace 2>&1 | Tee-Object -FilePath $testLog | Out-Null
    $testExit = $LASTEXITCODE

    # Filter for the test name, not the summary line: -match is
    # case-insensitive, so 'FAILED' would match "0 failed" and a flake's name
    # would be lost. -cmatch and the leading 'test ' is the signal.
    $failed = @(Select-String -Path $testLog -Pattern '^test \S+ \.\.\. FAILED' -CaseSensitive |
        ForEach-Object { $_.Line.Trim() })
    if ($failed.Count -gt 0) {
        Exit-With 1 "$($failed.Count) test(s) failed:`n  $($failed -join "`n  ")"
    }
    if ($testExit -ne 0) { Exit-With 1 "cargo test --workspace exited $testExit with no named failure. See $testLog." }

    $passed = 0
    foreach ($line in (Select-String -Path $testLog -Pattern '^test result: ok\. (\d+) passed')) {
        $passed += [int]$line.Matches[0].Groups[1].Value
    }
    Write-Step "$passed tests passed, 0 failed"
}

# ---------------------------------------------------------------------------
# Sync the corpus to the references branch
# ---------------------------------------------------------------------------
#
# An orphan branch holding reference/ and nothing else. It is force-pushed
# every time, because it is a mirror of a working directory rather than a
# history: the corpus has no commits worth bisecting and a growing history of
# a 52 MB tree is a cost with no reader.
#
# Built in a temporary index so the working tree's index is never touched, and
# so a failure here cannot leave reference/ staged on main.

function Sync-References {
    $corpus = Join-Path $script:RepoRoot "reference"
    if (-not (Test-Path $corpus)) {
        Write-Step "no reference/ on disk, nothing to sync"
        return
    }

    $tempIndex = Join-Path ([System.IO.Path]::GetTempPath()) "bit-cli-references-index-$PID"
    if (Test-Path $tempIndex) { Remove-Item -Force $tempIndex }
    $previousIndex = $env:GIT_INDEX_FILE
    $env:GIT_INDEX_FILE = $tempIndex
    try {
        Write-Step "building the $ReferenceBranch tree from reference/"
        # --force because reference/ is gitignored on main, which is the point.
        # This index is thrown away, so it cannot leak into a main commit.
        Invoke-Git -gitArgs @("add", "--force", "--", "reference") | Out-Null
        $tree = (Invoke-Git -gitArgs @("write-tree")).Trim()
        if (-not $tree) { Exit-With 1 "could not write a tree for reference/" }

        $stamp = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ss.fffZ")
        $head = (Invoke-Git -gitArgs @("rev-parse", "--short", "HEAD")).Trim()
        $files = (Get-ChildItem -Recurse -File -Path $corpus).Count
        $commitMessage = @"
Corpus as of $stamp

Mirror of reference/ at main $head. $files files.
Not a history: this branch is force-pushed and holds only the current corpus.
See TODO/reference-map.md and reference/RESEARCH.md.
"@
        $commit = (Invoke-GitAs -gitArgs @("commit-tree", $tree, "-m", $commitMessage)).Trim()
        if (-not $commit) { Exit-With 1 "could not create the $ReferenceBranch commit" }

        Write-Step "pushing $files files to origin/$ReferenceBranch"
        Invoke-Git -gitArgs @("push", "--force", "origin", "${commit}:refs/heads/$ReferenceBranch") | Out-Null
        Write-Step "origin/$ReferenceBranch now holds $files files at $($commit.Substring(0,7))"
    }
    finally {
        if ($null -eq $previousIndex) { Remove-Item Env:GIT_INDEX_FILE -ErrorAction SilentlyContinue }
        else { $env:GIT_INDEX_FILE = $previousIndex }
        if (Test-Path $tempIndex) { Remove-Item -Force $tempIndex -ErrorAction SilentlyContinue }
    }
}

# ---------------------------------------------------------------------------
# Commit
# ---------------------------------------------------------------------------

if (-not $PushOnly) {
    if (-not $Message) { Exit-With 2 "-Message is required unless -PushOnly, -Check or -FetchReferences." }

    $hits = Test-Attribution "$Message`n$Body"
    if ($hits.Count -gt 0) {
        Exit-With 1 "the commit message carries AI attribution and will not be rewritten for you: $($hits -join '; '). Remove it and run again. See TODO/RULES.md section 4."
    }

    $onBranch = (Invoke-Git -gitArgs @("rev-parse", "--abbrev-ref", "HEAD")).Trim()
    if ($onBranch -ne $Branch) {
        Write-Step "on '$onBranch', not '$Branch'. Committing there."
    }

    if ($Path) {
        Write-Step "staging $($Path.Count) path(s)"
        Invoke-Git -gitArgs (@("add", "--") + $Path) | Out-Null
    }
    else {
        Write-Step "staging everything not ignored"
        Invoke-Git -gitArgs @("add", "-A") | Out-Null
    }

    if ($Evidence) {
        foreach ($item in $Evidence) {
            if ($item -match '^reference[/\\]') {
                Exit-With 1 "-Evidence '$item' is under reference/. That never enters main. See TODO/RULES.md section 4."
            }
            Write-Step "force-adding evidence: $item"
            Invoke-Git -gitArgs @("add", "--force", "--", $item) | Out-Null
        }
    }

    $forbidden = Get-ForbiddenStaged
    if ($forbidden.Count -gt 0) {
        Invoke-Git -gitArgs (@("reset", "--") + $forbidden) -AllowFail | Out-Null
        Exit-With 1 "refusing to commit paths under reference/: $($forbidden -join ', '). They have been unstaged. reference/ belongs on the '$ReferenceBranch' branch. See TODO/RULES.md section 4."
    }

    $staged = (Invoke-Git -gitArgs @("diff", "--cached", "--name-only")) -split "`r?`n" |
        Where-Object { $_ -and $_.Trim() }
    if ($staged.Count -eq 0) {
        Exit-With 1 "nothing staged, so there is nothing to commit."
    }
    Write-Step "$($staged.Count) file(s) staged"

    Invoke-Gates

    $full = if ($Body) { "$Message`n`n$Body" } else { $Message }
    $messageFile = Join-Path ([System.IO.Path]::GetTempPath()) "bit-cli-commit-$PID.txt"
    # UTF-8 without a BOM: pwsh 7 defaults to that, and a BOM ends up as three
    # bytes at the front of the subject line.
    [System.IO.File]::WriteAllText($messageFile, $full, (New-Object System.Text.UTF8Encoding($false)))
    try {
        Invoke-GitAs -gitArgs @("commit", "--file", $messageFile) | Out-Null
    }
    finally {
        Remove-Item -Force $messageFile -ErrorAction SilentlyContinue
    }

    $head = (Invoke-Git -gitArgs @("log", "-1", "--pretty=%h %s")).Trim()
    Write-Step "committed $head"

    $who = (Invoke-Git -gitArgs @("log", "-1", "--pretty=%an <%ae>|%cn <%ce>")).Trim()
    $expected = "$AuthorName <$AuthorEmail>"
    if ($who -ne "$expected|$expected") {
        Exit-With 1 "the commit landed with identity '$who' rather than '$expected'. Something overrode -c."
    }
}
else {
    Invoke-Gates
}

# ---------------------------------------------------------------------------
# Push
# ---------------------------------------------------------------------------

if ($NoPush) {
    Write-Step "-NoPush, stopping before the push"
    exit 0
}

$onBranch = (Invoke-Git -gitArgs @("rev-parse", "--abbrev-ref", "HEAD")).Trim()
Write-Step "pushing $onBranch to origin"
Invoke-Git -gitArgs @("push", "origin", $onBranch) | Out-Null
Write-Step "pushed"

if (-not $NoReferences) { Sync-References }

# The run the push started. A push that leaves CI red without an entry naming
# why is not finished, so print the handle rather than making the caller find it.
$gh = Get-Command gh -ErrorAction SilentlyContinue
if ($gh) {
    Write-Step "the run this push started, once it registers:"
    & gh run list --limit 1
    Write-Host ""
    Write-Host "  Watch it:  gh run watch `$(gh run list --limit 1 --json databaseId --jq '.[0].databaseId')"
    Write-Host "  Read it:   gh run view `$(gh run list --limit 1 --json databaseId --jq '.[0].databaseId')"
}
else {
    Write-Step "gh is not on PATH. Check CI by hand before calling this finished."
}

exit 0
