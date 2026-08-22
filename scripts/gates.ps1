# Every gate, in one command, with one answer.
#
# A session runs these at the start to establish a baseline and after every
# change to keep one. Run by hand they are four commands whose output has to be
# read four different ways: `fmt` says nothing when it passes, `clippy` buries
# its verdict in a build log, `test` needs its failures filtered by test name
# rather than by the summary line, and `deny` says "ok" four times. This says
# one thing.
#
# Usage:
#   pwsh -NoProfile -File scripts/gates.ps1
#   pwsh -NoProfile -File scripts/gates.ps1 -Fix         # cargo fmt --all first
#   pwsh -NoProfile -File scripts/gates.ps1 -Fast        # skip deny and the build
#   pwsh -NoProfile -File scripts/gates.ps1 -Build       # also build the binaries
#   pwsh -NoProfile -File scripts/gates.ps1 -Json
#
# Exit codes: 0 every gate passed, 1 one did not, 2 the script could not run.
#
# Three things it does that running the commands by hand does not:
#
#   - Kills stray `bit-cli` and loopback-* processes first. A release binary
#     left running by an acceptance script holds its own executable open, and
#     the next build fails on a locked file with an error that names neither.
#     One exception, and it is load-bearing: a process running out of `.tmp/`
#     is spared, because `soak.ps1` copies its binaries there so that a six
#     hour run holds no build output open. Killing it would end T-040's
#     acceptance silently, which is the one measurement a session cannot redo.
#   - Filters test failures with `^test \S+ \.\.\. FAILED` and -CaseSensitive.
#     `-match 'FAILED'` matches "0 failed" in the summary line, so a flake's
#     name is lost exactly when it is needed. TODO/RULES.md section 5.
#   - Builds with `--bins --examples` when asked. `--examples` alone builds the
#     examples and no binaries, which is how a script comes to run yesterday's
#     `bit-cli.exe`. TODO/RULES.md section 5.
#   - Fails on a NUL byte in any tracked text file. Two were in this tree and
#     neither was noticed, because a file with one in it is what `grep` calls
#     binary and skips.
#   - Runs `check-todo.ps1`, so a push cannot carry a record that contradicts
#     the tree. `patches/TASKS.md` said two P0 entries were open for a session
#     after both closed, because nothing compared the two files.
#   - Prints the toolchain and warns when the stable it is using is behind the
#     one CI would install. Clippy gains lints with every release, so a green
#     run here on an older rustc is not a green clippy job there. It warns
#     rather than fails: a stale toolchain is not a reason to stop working.
#
# See TODO/RULES.md.

[CmdletBinding()]
param(
    # Run `cargo fmt --all` before checking, rather than failing on formatting.
    [switch]$Fix,
    # Skip `cargo deny` and the build. For the inner loop.
    [switch]$Fast,
    # Also `cargo build --release --bins --examples`.
    [switch]$Build,
    [switch]$Json
)

$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
Set-Location $repo

$results = [System.Collections.Specialized.OrderedDictionary]::new()
$failures = [System.Collections.ArrayList]::new()
$started = [System.Diagnostics.Stopwatch]::StartNew()

function Write-Step([string]$text) {
    $stamp = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ss.fffZ")
    Write-Host "$stamp gates: $text"
}

function Record([string]$name, [bool]$ok, [string]$detail) {
    $results[$name] = [ordered]@{ ok = $ok; detail = $detail }
    if (-not $ok) { [void]$failures.Add("$name`: $detail") }
    $verdict = if ($ok) { "ok" } else { "FAILED" }
    Write-Step "$name $verdict$(if ($detail) { " ($detail)" })"
}

# ---------------------------------------------------------------------------
# Stray processes
# ---------------------------------------------------------------------------

# A process running out of `.tmp/` is not stray and is not killed. `soak.ps1`
# copies the binaries it needs into `.tmp/soak/bin/` for exactly this reason:
# the copy holds no build output open, so nothing here is served by stopping
# it. T-040's acceptance is a six hour run, PROGRESS.md tells a session to
# start it early, and every gates run in between would otherwise end it. The
# run is the measurement, so losing it silently costs the whole session.
$tmpRoot = [System.IO.Path]::GetFullPath((Join-Path $repo ".tmp")) + [System.IO.Path]::DirectorySeparatorChar
$candidates = @(Get-Process bit-cli, loopback-fileserver, loopback-tracker, loopback-churn -ErrorAction SilentlyContinue)
$spared = @($candidates | Where-Object {
        $path = try { $_.Path } catch { $null }
        $path -and $path.StartsWith($tmpRoot, [StringComparison]::OrdinalIgnoreCase)
    })
$stray = @($candidates | Where-Object { $spared -notcontains $_ })
if ($spared.Count -gt 0) {
    Write-Step "leaving $($spared.Count) process(es) under .tmp/ alone, they hold no build output"
}
if ($stray.Count -gt 0) {
    Write-Step "stopping $($stray.Count) stray process(es) that would lock the build output"
    $stray | Stop-Process -Force -ErrorAction SilentlyContinue
}

# ---------------------------------------------------------------------------
# The toolchain, which is not a gate but decides what the gates can see
# ---------------------------------------------------------------------------
#
# CI pins `stable`, which moves. Clippy gains lints with every release, so a
# local toolchain a release behind passes a clippy CI then fails. That has
# happened: `clippy::chunks_exact_to_as_chunks` arrived in 1.98 and a push that
# was green here was red there, on nothing but the age of this machine's
# rustc. This warns rather than fails, because a red gate for a toolchain
# nobody has updated yet would stop work that is otherwise fine.

$toolchain = (& rustc --version 2>&1 | Out-String).Trim()
Write-Step "toolchain $toolchain"
if (-not $Fast -and (Get-Command rustup -ErrorAction SilentlyContinue)) {
    $check = & rustup check 2>&1 | Out-String
    $stale = $check -split "`n" | Where-Object {
        $_ -match '^stable-' -and $_ -match 'update available'
    }
    if ($stale) {
        # Only the toolchain in use matters. `rustup check` lists every one
        # installed, and a stale `windows-gnu` beside a current `windows-msvc`
        # is not a problem anybody has.
        $inUse = (& rustup show active-toolchain 2>&1 | Out-String).Trim()
        foreach ($line in $stale) {
            $name = ($line -split ' ')[0]
            if ($inUse -like "$name*") {
                Write-Step "WARNING: $($line.Trim())"
                Write-Step "WARNING: CI builds on stable, so a lint this rustc cannot see can still fail there. rustup update stable"
            }
        }
    }
}

# ---------------------------------------------------------------------------
# text
# ---------------------------------------------------------------------------
#
# A NUL byte in a tracked text file makes every text tool treat the file as
# binary. `grep` answers "Binary file X matches" instead of the line, a diff is
# unreadable, and whatever is around it is invisible to a review.
#
# Two got in and neither was noticed. `crates/bit-cli-core/src/torrent/bencode.rs`
# had three, in a byte-string literal written with the bytes themselves rather
# than escapes, since 2026-08-21. `TODO/trackers.md` had one on 2026-08-22, from
# a Python escape interpreted on the way to the file. Both are one line to
# check, and the check is here rather than in `check-todo.ps1` because it is
# the source tree that had the older one.
#
# Tracked files only, and only the ones meant to be text: `git ls-files` knows
# what is tracked, and the extension list is what this tree actually holds.

$binaryish = [System.Collections.ArrayList]::new()
$tracked = & git ls-files -- "*.rs" "*.md" "*.ps1" "*.toml" "*.yml" "*.jq" 2>$null
foreach ($relative in $tracked) {
    if (-not $relative) { continue }
    $path = Join-Path $repo $relative
    if (-not (Test-Path $path)) { continue }
    $bytes = [System.IO.File]::ReadAllBytes($path)
    $at = [System.Array]::IndexOf($bytes, [byte]0)
    if ($at -ge 0) { [void]$binaryish.Add("${relative}:$at") }
}
Record "text" ($binaryish.Count -eq 0) $(if ($binaryish.Count -eq 0) { "" }
    else { "NUL byte in $($binaryish -join ', ')" })

# ---------------------------------------------------------------------------
# record
# ---------------------------------------------------------------------------
#
# `TODO/` is the authoritative record and `patches/TASKS.md` is the ordered
# list of vendored work. Both are second copies of a status that lives in an
# entry, and a second copy is the thing that goes stale.
#
# It went stale, and this gate is what it cost. The session of 2026-08-22
# closed both P0 entries, wrote it into the entries, into `INDEX.md` and into
# `PROGRESS.md`, and pushed. `patches/TASKS.md` was rewritten afterwards and
# never committed, so HEAD went on saying `T-020 | P0 | open` while the entry
# beside it said `done`. The next session read the stale one first.
#
# `check-todo.ps1` compares them: every row against the entry it names, every
# count against the rows, and PROGRESS.md against what RULES.md section 2 step
# 2 says it must carry. That is a second, and it runs here so that a push
# cannot carry a record contradicting the tree it describes. It is not skipped
# by -Fast: it costs about three seconds, and it is the one gate here that
# catches a claim rather than a defect.

$todoArgs = @("-NoProfile", "-File", (Join-Path $PSScriptRoot "check-todo.ps1"))
$todoOut = (& pwsh @todoArgs 2>&1 | Out-String)
$todoOk = ($LASTEXITCODE -eq 0)
$todoDetail = ""
if (-not $todoOk) {
    $lines = @($todoOut -split "`r?`n" | Where-Object { $_ -match '^\s+\[' })
    $todoDetail = if ($lines.Count -gt 0) { ($lines[0].Trim()) } else { "see: pwsh -NoProfile -File scripts/check-todo.ps1" }
    if ($lines.Count -gt 1) { $todoDetail += " and $($lines.Count - 1) more" }
}
Record "record" $todoOk $todoDetail

# ---------------------------------------------------------------------------
# man
# ---------------------------------------------------------------------------
#
# `man/bit-cli.1`, `man/bit-cli.json` and `man/bit-cli.md` are generated from
# the clap definition and committed, so a reader can open them without building
# anything. A committed generated file is only worth having if something fails
# when it goes stale.
#
# The check that binds is `cargo test -p bit-cli --test man_is_current`, inside
# the `test` gate below: it renders from the crate being compiled, so it cannot
# compare against a stale binary, and it runs wherever CI builds. This line is
# here so a session that regenerates gets told what to run rather than reading
# a test name out of a failure, and it is skipped when there is no binary yet
# rather than failing on one that does not exist.
#
# -Fix regenerates them, the same as it formats.

$manExe = Join-Path $repo "target/release/bit-cli.exe"
if (-not (Test-Path $manExe)) { $manExe = Join-Path $repo "target/release/bit-cli" }
if ($Fast) {
    Write-Step "man skipped by -Fast"
}
elseif (-not (Test-Path $manExe)) {
    Write-Step "man skipped: no release binary yet, the test gate covers it"
}
else {
    $manArgs = @("-NoProfile", "-File", (Join-Path $PSScriptRoot "check-man.ps1"))
    if ($Fix) { $manArgs += "-Fix" }
    & pwsh @manArgs | Out-Null
    Record "man" ($LASTEXITCODE -eq 0) $(if ($LASTEXITCODE -eq 0) { "" }
        else { "run with -Fix, or: pwsh -NoProfile -File scripts/check-man.ps1 -Fix" })
}

# ---------------------------------------------------------------------------
# fmt
# ---------------------------------------------------------------------------

if ($Fix) {
    & cargo fmt --all
    Record "fmt" ($LASTEXITCODE -eq 0) "rewritten"
}
else {
    & cargo fmt --all --check | Out-Null
    Record "fmt" ($LASTEXITCODE -eq 0) $(if ($LASTEXITCODE -eq 0) { "" } else { "run with -Fix" })
}

# ---------------------------------------------------------------------------
# clippy
# ---------------------------------------------------------------------------

$clippyLog = Join-Path ([System.IO.Path]::GetTempPath()) "bit-cli-gates-clippy.txt"
& cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 |
    Tee-Object -FilePath $clippyLog | Out-Null
$clippyOk = $LASTEXITCODE -eq 0
$clippyCount = @(Select-String -Path $clippyLog -Pattern '^error' -CaseSensitive).Count
Record "clippy" $clippyOk $(if ($clippyOk) { "" } else { "$clippyCount error line(s), see $clippyLog" })

# ---------------------------------------------------------------------------
# test
# ---------------------------------------------------------------------------

$testLog = Join-Path ([System.IO.Path]::GetTempPath()) "bit-cli-gates-tests.txt"
& cargo test --workspace 2>&1 | Tee-Object -FilePath $testLog | Out-Null
$testExit = $LASTEXITCODE

$failed = @(Select-String -Path $testLog -Pattern '^test \S+ \.\.\. FAILED' -CaseSensitive |
    ForEach-Object { ($_.Line -split '\s+')[1] })
$passed = 0
foreach ($line in (Select-String -Path $testLog -Pattern '^test result: ok\. (\d+) passed')) {
    $passed += [int]$line.Matches[0].Groups[1].Value
}
$testOk = ($testExit -eq 0) -and ($failed.Count -eq 0)
$testDetail = if ($testOk) { "$passed passed" }
elseif ($failed.Count -gt 0) { "$($failed.Count) failed: $($failed -join ', ')" }
else { "exited $testExit with no named failure, see $testLog" }
Record "test" $testOk $testDetail

# ---------------------------------------------------------------------------
# deny
# ---------------------------------------------------------------------------

if (-not $Fast) {
    if (Get-Command cargo-deny -ErrorAction SilentlyContinue) {
        & cargo deny check 2>&1 | Out-Null
        Record "deny" ($LASTEXITCODE -eq 0) ""
    }
    else {
        Record "deny" $true "cargo-deny is not installed, so this is unmeasured"
    }
}

# ---------------------------------------------------------------------------
# build
# ---------------------------------------------------------------------------

if ($Build -and -not $Fast) {
    # --bins AND --examples. `--examples` alone builds no binaries, which is
    # how an acceptance script comes to measure a stale bit-cli.exe.
    & cargo build --release --bins --examples 2>&1 | Out-Null
    Record "build" ($LASTEXITCODE -eq 0) "release, bins and examples"
}

# ---------------------------------------------------------------------------
# Say it
# ---------------------------------------------------------------------------

$started.Stop()
$ok = $failures.Count -eq 0

if ($Json) {
    [ordered]@{
        kind           = "gates"
        schema_version = "1"
        generated_at   = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ss.fffZ")
        elapsed_ms     = $started.ElapsedMilliseconds
        ok             = $ok
        tests_passed   = $passed
        tests_failed   = @($failed)
        gates          = $results
        failures       = @($failures)
    } | ConvertTo-Json -Depth 6
    exit $(if ($ok) { 0 } else { 1 })
}

Write-Host ""
if ($ok) {
    Write-Host ("all gates pass: {0} tests, {1:n1}s" -f $passed, ($started.Elapsed.TotalSeconds))
    exit 0
}
Write-Host "gates failed:"
foreach ($item in $failures) { Write-Host "  $item" }
exit 1
