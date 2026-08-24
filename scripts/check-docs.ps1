<#
.SYNOPSIS
    Do README.md and docs/ still describe this tool, in this repository's voice?

.DESCRIPTION
    The defect this exists to catch is a document that was true when it was
    written. Three shapes of it, and all three are invisible to every other
    gate:

      - a link or a path that stopped resolving when something was renamed
      - a flag or a command an example names that the CLI does not have, which
        is worse than a dead link because the example still looks runnable
      - project history in a document about what the tool does, which is how a
        reference page turns into a diary

    It also enforces the prose rule mechanically: no emoji, no em dash, no C0
    control byte, and none of the banned vocabulary. The banned list is a data
    block at the top of this script so it can be extended without touching the
    logic.

    What it does NOT check is whether a claim is true. That is a reading, and
    `TODO/RULES.md` section 2 step 4 is where it belongs.

    `scripts/check-todo.ps1` covers `TODO/` and `patches/` and this covers
    `README.md` and `docs/`. They are separate scripts rather than one because
    the rules differ: a `TODO/` entry is allowed to carry history and a
    `docs/` page is not, and a `TODO/` entry may name a script that does not
    exist yet while a `docs/` page may not.

        pwsh -NoProfile -File scripts/check-docs.ps1
        pwsh -NoProfile -File scripts/check-docs.ps1 -Json bench/docs.json

    Exits 0 when everything holds, 1 when it does not, and 2 when the check
    could not run, which here means `man/bit-cli.json` is missing.
#>

[CmdletBinding()]
param(
    [string]$Json,
    [switch]$ListUrls
)

$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot

# ---------------------------------------------------------------------------
# The data blocks. Extend these rather than the logic below.
# ---------------------------------------------------------------------------

# Vocabulary that reads as though a language model wrote it. Matched
# case-insensitively on a word boundary.
$BannedWords = @(
    'delve', 'seamless', 'seamlessly', 'robust', 'powerful', 'cutting-edge',
    'state-of-the-art', 'game-changer', 'game changer', 'unlock', 'elevate',
    'streamline', 'effortless', 'rich set of', 'wide range of',
    'plethora', 'myriad', 'utilize', 'utilizing', 'leverages', 'leveraging'
)

# `harness` is deliberately not in the list above. This repository has test
# harnesses, a soak harness and an interop harness, and the noun is its own
# vocabulary. Only the verb is banned, below.

# Framings that carry no fact.
$BannedPhrases = @(
    "it's not just", 'it is not just', 'more than just',
    "in today's landscape", "in today's world",
    "it's worth noting that", 'it is worth noting that',
    "it's important to note that", 'it is important to note that',
    'that said,', 'at the end of the day', "let's take a look at",
    'in this section we will', 'in this section, we will',
    'this ensures that', 'by doing so,', 'needless to say'
)

# Project history in a document about what the tool does.
$HistoryMarkers = @(
    'a previous session', 'the previous session', 'this session',
    'last session', 'we decided', 'we chose', 'originally,', 'used to be',
    'it used to', 'they used to', 'has since been', 'was previously',
    'in an earlier version', 'before this change'
)

# `leverage` and `comprehensive` are context dependent: `leverage` as a noun is
# fine and `comprehensive` describing a measured coverage set is a fact. Only
# the verb and the praise are banned.
$BannedRegexes = @(
    @{ Pattern = '\bleverage(s|d)?\s+(the|a|an|our|its|this)\b'; Why = 'leverage as a verb' },
    @{ Pattern = '\bharness(es|ed|ing)?\s+(the|a|an|our|its|this)\b'; Why = 'harness as a verb' },
    @{ Pattern = '\b(a|the|our|its)\s+comprehensive\b';          Why = 'comprehensive as praise' },
    @{ Pattern = '\bfirst-class\s+(support|experience|citizen)\b'; Why = 'first-class as praise' }
)

# A bare session date in prose. A date inside a path, a filename or a code span
# is fine; one in a sentence is usually "it was broken until this date".
$DatePattern = '\b20\d{2}-\d{2}-\d{2}\b'

# Three documents are about **how this repository is worked on** rather than
# about what the tool does, so process vocabulary is their subject and a date
# beside a rule is the evidence for the rule. The narrative markers still
# apply to them; the bare-date rule does not.
#
# This is a scope decision rather than an exemption: the rule exists to keep a
# diary out of a reference page, and these three are not reference pages.
$ProcessDocs = @('docs/AGENTS.md', 'docs/reference-mining.md', 'docs/task-authoring.md')

$Problems = New-Object System.Collections.ArrayList
function Problem([string]$kind, [string]$message) {
    [void]$Problems.Add([ordered]@{ kind = $kind; message = $message })
}

# ---------------------------------------------------------------------------
# The command surface, so an example naming a flag can be checked against it
# ---------------------------------------------------------------------------

$manPath = Join-Path $repo 'man/bit-cli.json'
if (-not (Test-Path $manPath)) {
    [Console]::Error.WriteLine("check-docs: no man/bit-cli.json, so no flag can be checked. Run scripts/check-man.ps1 -Fix")
    exit 2
}

$man = Get-Content $manPath -Raw | ConvertFrom-Json
$knownFlags = New-Object System.Collections.Generic.HashSet[string]
$knownCommands = New-Object System.Collections.Generic.HashSet[string]
foreach ($arg in $man.global_args) { if ($arg.name) { [void]$knownFlags.Add($arg.name) } }
foreach ($command in $man.commands) {
    # "bit-cli webseed list" -> the leaf verb and the whole path
    $parts = $command.name -split '\s+'
    if ($parts.Count -gt 1) { [void]$knownCommands.Add($parts[1]) }
    if ($parts.Count -gt 2) { [void]$knownCommands.Add("$($parts[1]) $($parts[2])") }
    foreach ($arg in $command.args) { if ($arg.name) { [void]$knownFlags.Add($arg.name) } }
}

# Flags that belong to something else and legitimately appear in these docs.
$ForeignFlags = @(
    # cargo, rustup, gh, curl, aria2c, rqbit, and this repository's own scripts
    '--locked', '--release', '--workspace', '--all-features', '--all-targets',
    '--format-version', '--config', '--output-file', '--manifest-path',
    '--target-dir', '--no-cocomo', '--depth', '--jq', '--paginate', '--limit',
    '--bins', '--examples', '--check', '--fix', '--json', '--not-a-flag',
    '--bt-web-seed', '--range', '--data-binary', '--head', '--fail',
    '--force-with-lease', '--index-filter', '--no-verify', '--no-gpg-sign'
)

# ---------------------------------------------------------------------------
# The files
# ---------------------------------------------------------------------------

$files = @()
$readme = Join-Path $repo 'README.md'
if (Test-Path $readme) { $files += Get-Item $readme }
$docsDir = Join-Path $repo 'docs'
if (Test-Path $docsDir) { $files += Get-ChildItem -Path $docsDir -Filter *.md -Recurse -File }

if ($files.Count -eq 0) {
    [Console]::Error.WriteLine("check-docs: no README.md and no docs/*.md to check")
    exit 2
}

$externalUrls = New-Object System.Collections.Generic.HashSet[string]
$checkedLinks = 0
$checkedFlags = 0

foreach ($file in $files) {
    $relative = $file.FullName.Substring($repo.Length + 1) -replace '\\', '/'
    $lines = Get-Content -LiteralPath $file.FullName
    $text = ($lines -join "`n")

    # Which lines are inside a fenced code block. Prose rules do not apply
    # there: a banned word inside a quoted error message is the error message.
    $inFence = @($false) * ($lines.Count + 1)
    $fence = $false
    for ($i = 0; $i -lt $lines.Count; $i++) {
        if ($lines[$i] -match '^\s*(```|~~~)') { $fence = -not $fence; $inFence[$i] = $true; continue }
        $inFence[$i] = $fence
    }

    # The headings in this file, for anchor resolution.
    $anchors = New-Object System.Collections.Generic.HashSet[string]
    foreach ($line in $lines) {
        if ($line -match '^#{1,6}\s+(.*)$') {
            $slug = $Matches[1].ToLowerInvariant()
            $slug = $slug -replace '`', '' -replace '\[([^\]]*)\]\([^)]*\)', '$1'
            $slug = $slug -replace '[^a-z0-9 -]', ''
            $slug = ($slug.Trim() -replace '\s+', '-')
            [void]$anchors.Add($slug)
        }
    }

    for ($i = 0; $i -lt $lines.Count; $i++) {
        $line = $lines[$i]
        $lineNo = $i + 1

        # --- typography, by code point rather than by regex ------------------
        #
        # By code point on purpose. A character class holding an em dash or an
        # emoji would put those bytes in this file, and then the script that
        # bans them contains them. Nothing here is outside ASCII.
        $sawControl = $false; $sawDash = $false; $sawEmoji = $false; $sawGlyph = $false
        foreach ($ch in $line.ToCharArray()) {
            $code = [int]$ch
            if ($code -lt 32 -and $ch -ne "`t") { $sawControl = $true; continue }
            if ($code -eq 0x2014 -or $code -eq 0x2013) { $sawDash = $true; continue }
            # tick, cross, and their emoji-presentation forms
            if ($code -in 0x2713, 0x2714, 0x2717, 0x2718, 0x274C, 0x2705) { $sawGlyph = $true; continue }
            # arrows used as punctuation, dingbats, and the emoji planes
            if (($code -ge 0x2190 -and $code -le 0x21FF) -or
                ($code -ge 0x2600 -and $code -le 0x27BF) -or
                ($code -ge 0xFE00 -and $code -le 0xFE0F) -or
                ($code -ge 0xD800 -and $code -le 0xDBFF)) { $sawEmoji = $true }
        }
        if ($sawControl) { Problem 'control-byte' "$relative`:$lineNo carries a C0 control byte" }
        if ($sawDash)    { Problem 'em-dash' "$relative`:$lineNo uses an em or en dash. Rewrite the sentence" }
        if ($sawGlyph)   { Problem 'glyph' "$relative`:$lineNo uses a tick or cross glyph. A status word reads the same and copies" }
        if ($sawEmoji)   { Problem 'emoji' "$relative`:$lineNo carries an emoji or an arrow used as punctuation" }

        if ($inFence[$i]) { continue }

        # --- vocabulary ----------------------------------------------------
        foreach ($word in $BannedWords) {
            if ($line -imatch "\b$([regex]::Escape($word))\b") {
                Problem 'vocabulary' "$relative`:$lineNo uses '$word'"
            }
        }
        foreach ($phrase in $BannedPhrases) {
            if ($line -imatch [regex]::Escape($phrase)) {
                Problem 'framing' "$relative`:$lineNo uses '$phrase'"
            }
        }
        foreach ($rule in $BannedRegexes) {
            if ($line -imatch $rule.Pattern) {
                Problem 'vocabulary' "$relative`:$lineNo uses $($rule.Why)"
            }
        }

        # --- project history ------------------------------------------------
        foreach ($marker in $HistoryMarkers) {
            if ($line -imatch [regex]::Escape($marker)) {
                Problem 'history' "$relative`:$lineNo says '$marker'. docs/ describes what the tool does, not what the project did"
            }
        }
        # A bare date in prose, outside a path, a link target and a code span.
        if ($ProcessDocs -notcontains $relative) {
            $prose = $line -replace '`[^`]*`', '' -replace '\[[^\]]*\]\([^)]*\)', ''
            if ($prose -match $DatePattern -and $prose -notmatch 'soak-|bench/|\.csv|\.json') {
                Problem 'history' "$relative`:$lineNo carries a bare date in prose. A dated statement in a reference page is a session marker"
            }
        }

        # --- links ----------------------------------------------------------
        foreach ($m in [regex]::Matches($line, '\[[^\]]*\]\(([^)\s]+)\)')) {
            $target = $m.Groups[1].Value
            if ($target -match '^(https?|mailto):') {
                [void]$externalUrls.Add($target)
                if ($target -notmatch '^https?://[^/\s]+') {
                    Problem 'url' "$relative`:$lineNo has a malformed URL '$target'"
                }
                continue
            }
            if ($target.StartsWith('#')) {
                $anchor = $target.Substring(1).ToLowerInvariant()
                if (-not $anchors.Contains($anchor)) {
                    Problem 'dead-anchor' "$relative`:$lineNo links to '#$anchor', which is not a heading in this file"
                }
                continue
            }
            $path = $target
            $anchor = $null
            if ($path.Contains('#')) {
                $parts = $path -split '#', 2
                $path = $parts[0]; $anchor = $parts[1].ToLowerInvariant()
            }
            if (-not $path) { continue }
            $resolved = Join-Path $file.DirectoryName $path
            $checkedLinks++
            if (-not (Test-Path $resolved)) {
                Problem 'dead-link' "$relative`:$lineNo links to '$path', which does not resolve from $(Split-Path -Leaf $file.DirectoryName)/"
                continue
            }
            if ($anchor -and $path -match '\.md$') {
                $targetHeadings = New-Object System.Collections.Generic.HashSet[string]
                foreach ($h in (Get-Content -LiteralPath $resolved)) {
                    if ($h -match '^#{1,6}\s+(.*)$') {
                        $slug = $Matches[1].ToLowerInvariant() -replace '`', '' -replace '\[([^\]]*)\]\([^)]*\)', '$1'
                        $slug = $slug -replace '[^a-z0-9 -]', ''
                        [void]$targetHeadings.Add(($slug.Trim() -replace '\s+', '-'))
                    }
                }
                if (-not $targetHeadings.Contains($anchor)) {
                    Problem 'dead-anchor' "$relative`:$lineNo links to '$path#$anchor', and that heading is not in it"
                }
            }
        }

        # --- a bare scripts/ path --------------------------------------------
        foreach ($m in [regex]::Matches($line, '(?<![\w/-])(scripts/[A-Za-z0-9._-]+\.(?:ps1|jq))')) {
            $cited = $m.Groups[1].Value
            if (-not (Test-Path (Join-Path $repo $cited))) {
                Problem 'dead-path' "$relative`:$lineNo names $cited, which is not there"
            }
        }
    }

    # --- flags and commands, over the whole file including fences ----------
    # An example is exactly where a renamed flag hides, so code fences are
    # checked here even though the prose rules skip them.
    foreach ($m in [regex]::Matches($text, '(?m)^\s*(?:\$\s*)?bit-cli(?:\.exe)?\s+([^\r\n]*)')) {
        $invocation = $m.Groups[1].Value
        foreach ($fm in [regex]::Matches($invocation, '(?<![\w-])(--[a-z][a-z0-9-]+)')) {
            $flag = $fm.Groups[1].Value
            if ($ForeignFlags -contains $flag) { continue }
            $checkedFlags++
            if (-not $knownFlags.Contains($flag)) {
                Problem 'unknown-flag' "$relative names '$flag' in a bit-cli invocation, and man/bit-cli.json has no such flag"
            }
        }
        $verb = ($invocation -split '\s+' | Where-Object { $_ -and -not $_.StartsWith('-') } | Select-Object -First 1)
        if ($verb -and $verb -match '^[a-z][a-z-]*$' -and -not $knownCommands.Contains($verb)) {
            Problem 'unknown-command' "$relative names 'bit-cli $verb', and man/bit-cli.json has no such command"
        }
    }
}

# ---------------------------------------------------------------------------
# README's tables must link to a doc that exists
# ---------------------------------------------------------------------------

$readmeText = Get-Content -LiteralPath $readme -Raw
foreach ($heading in @('## Features', '## Commands')) {
    if ($readmeText -notmatch [regex]::Escape($heading)) {
        Problem 'readme-shape' "README.md has no '$heading' section, and the table of doc links lives there"
        continue
    }
    $after = $readmeText.Substring($readmeText.IndexOf($heading) + $heading.Length)
    $next = $after.IndexOf("`n## ")
    if ($next -gt 0) { $after = $after.Substring(0, $next) }
    $rows = [regex]::Matches($after, '(?m)^\|(?!\s*-)(?!\s*(?:capability|command)\b).*\|$')
    foreach ($row in $rows) {
        if ($row.Value -notmatch '\]\(') {
            Problem 'readme-shape' "README.md's '$heading' table has a row with no link: $($row.Value.Trim())"
        }
    }
}

# ---------------------------------------------------------------------------

$urls = @($externalUrls | Sort-Object)
if ($ListUrls) {
    Write-Host "check-docs: $($urls.Count) external URL(s):"
    foreach ($u in $urls) { Write-Host "  $u" }
}

$result = [ordered]@{
    kind          = 'docs'
    generated_at  = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ss.fffZ')
    files         = $files.Count
    links_checked = $checkedLinks
    flags_checked = $checkedFlags
    external_urls = $urls
    problems      = @($Problems)
}

if ($Json) {
    $jsonPath = if ([System.IO.Path]::IsPathRooted($Json)) { $Json } else { Join-Path $repo $Json }
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $jsonPath) | Out-Null
    $result | ConvertTo-Json -Depth 6 | Set-Content -Path $jsonPath -Encoding utf8NoBOM
    Write-Host "check-docs: wrote $Json"
}

Write-Host "check-docs: $($files.Count) file(s), $checkedLinks link(s), $checkedFlags flag(s), $($urls.Count) external URL(s)"

if ($Problems.Count -gt 0) {
    Write-Host ""
    Write-Host "$($Problems.Count) problem(s):"
    foreach ($p in $Problems) { Write-Host "  [$($p.kind)] $($p.message)" }
    exit 1
}

Write-Host "check-docs: everything resolves and the prose rule holds"
exit 0
