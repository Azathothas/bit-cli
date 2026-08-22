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
#   - Filters test failures with `^test \S+ \.\.\. FAILED` and -CaseSensitive.
#     `-match 'FAILED'` matches "0 failed" in the summary line, so a flake's
#     name is lost exactly when it is needed. TODO/RULES.md section 5.
#   - Builds with `--bins --examples` when asked. `--examples` alone builds the
#     examples and no binaries, which is how a script comes to run yesterday's
#     `bit-cli.exe`. TODO/RULES.md section 5.
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

$stray = @(Get-Process bit-cli, loopback-fileserver, loopback-tracker, loopback-churn -ErrorAction SilentlyContinue)
if ($stray.Count -gt 0) {
    Write-Step "stopping $($stray.Count) stray process(es) that would lock the build output"
    $stray | Stop-Process -Force -ErrorAction SilentlyContinue
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
