# Does the profile bit-cli impersonates still match a real browser?
#
# The defect it exists to catch: `TODO/cli-surface.md` T-244 ships a client
# that presents itself as a current Chrome, and `fingerprints/*.json` records
# what that client puts on the wire. Nothing compares either against a
# **browser**. A browser that changes its cipher list, its extension order, its
# HTTP/2 settings or its header set leaves this repository claiming a
# fingerprint nobody has, and the golden goes on passing because the golden is
# a record of ourselves.
#
# So this drives the browser this machine has at `loopback-tlsprobe`, reads
# what it emits, and compares it against `fingerprints/bit-cli-browser.json`
# field by field. The browser is the authority and the golden is the claim.
#
# **It recommends with proof**, which is the operator's requirement rather than
# a nicety: where the two disagree, the output carries the browser's own value
# in the shape the file that has to change wants, and names the browser and
# version it came from. A check that only says "your fingerprint changed" is
# half a tool.
#
# **With no browser it exits 2 and says so.** Most CI runners have none, and a
# check that fails a build because a machine has no Chrome is a check somebody
# disables. `crates/bit-cli-core/src/browser.rs` is the search and it names
# every path it looked at.
#
# **Header values are read here and nowhere else.** The probe records header
# names by default; `--header-values` is passed only for this one capture,
# where the client is a browser this script launched itself, into a throwaway
# profile, at a loopback port, having visited nothing. `cookie` and
# `authorization` are dropped even then. Nothing else in this repository ever
# asks for values.
#
# **A difference an open entry already names is recorded and not judged.**
# That is `scripts/check-close-wait.ps1`'s pattern: a check must not fail a
# build for a defect that is already written down and being worked on, and the
# other half of the rule is that the exemption comes off when the entry closes.
# `-Strict` judges every difference, which is what a session verifying a fix
# passes.
#
# Usage:
#   pwsh scripts/check-browser-fingerprint.ps1
#   pwsh scripts/check-browser-fingerprint.ps1 -Json
#   pwsh scripts/check-browser-fingerprint.ps1 -Out bench/browser-fingerprint.json
#   pwsh scripts/check-browser-fingerprint.ps1 -BrowserPath /path/to/chrome
#   pwsh scripts/check-browser-fingerprint.ps1 -Strict
#
# Exit 0 when the profile matches the browser apart from what an entry already
# names, 1 when it does not, and 2 when it could not run: no browser, no build,
# or the probe captured nothing.
#
# See TODO/cli-surface.md, T-244.

[CmdletBinding()]
param(
    [switch]$Json,
    [string]$Out = "",
    # An explicit browser, tried first and alone.
    [string]$BrowserPath = "",
    [ValidateSet("debug", "release")]
    [string]$Build = "release",
    [string]$GoldenDir = "fingerprints",
    # Seconds to let the browser run before it is killed.
    [int]$TimeoutSeconds = 25,
    # Judge every difference, including the ones an open entry already names.
    [switch]$Strict
)

$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot

function Exit-With([int]$code, [string]$message) {
    [Console]::Error.WriteLine("check-browser-fingerprint: $message")
    exit $code
}

$exeDir = Join-Path $repo "target/$Build/examples"
$probe = Join-Path $exeDir "loopback-tlsprobe.exe"
if (-not (Test-Path $probe)) { $probe = Join-Path $exeDir "loopback-tlsprobe" }
$finder = Join-Path $exeDir "browser-capture.exe"
if (-not (Test-Path $finder)) { $finder = Join-Path $exeDir "browser-capture" }
foreach ($p in @($probe, $finder)) {
    if (-not (Test-Path $p)) {
        Exit-With 2 "$p is missing; run: cargo build --$Build --bins --examples"
    }
}

$scratch = Join-Path $repo ".tmp/browser-fingerprint"
New-Item -ItemType Directory -Force -Path $scratch | Out-Null

# Differences an open entry already names, so this check records them rather
# than failing a build for them. One row per entry, and the row goes when the
# entry closes.
$known = @(
    @{
        field = "akamai"
        # Chrome opens stream 1 with a PRIORITY block; `h2` writes no priority
        # information at all and RFC 9113 deprecates it. Every other field of
        # the Akamai fingerprint matches.
        pattern = '^(?<head>.*\|)(?<priority>[^|]*)\|(?<tail>.*)$'
        entry = "T-262"
        why = "the PRIORITY field: Chrome opens stream 1 with one and h2 sends none"
    }
)

# Whether a difference is one an entry already names. For the Akamai
# fingerprint that means every field but the third agrees; anything else is a
# real disagreement even on the same line.
function Test-KnownAkamai([string]$claim, [string]$browser) {
    $a = $claim -split '\|'
    $b = $browser -split '\|'
    if ($a.Count -ne 4 -or $b.Count -ne 4) { return $false }
    return ($a[0] -ceq $b[0]) -and ($a[1] -ceq $b[1]) -and ($a[3] -ceq $b[3])
}

# ---------------------------------------------------------------------------
# Is there a browser at all? This is the case that has to work everywhere.
# ---------------------------------------------------------------------------

$findArgs = @('--json')
if ($BrowserPath) { $findArgs += @('--path', $BrowserPath) }
$findOut = Join-Path $scratch "find.json"
$findErr = Join-Path $scratch "find.err"
$find = Start-Process -FilePath $finder -ArgumentList $findArgs -PassThru -NoNewWindow -Wait `
    -RedirectStandardOutput $findOut -RedirectStandardError $findErr
if ($find.ExitCode -ne 0) {
    $why = (Get-Content -Raw $findErr -ErrorAction SilentlyContinue)
    if (-not $why) { $why = "no browser was found" }
    Exit-With 2 "$($why.Trim())"
}
$browser = Get-Content -Raw $findOut | ConvertFrom-Json

# ---------------------------------------------------------------------------
# What the browser puts on the wire
# ---------------------------------------------------------------------------

$probeOut = Join-Path $scratch "probe.txt"
$probeErr = Join-Path $scratch "probe.err"
if (Test-Path $probeOut) { Remove-Item -Force $probeOut }
$p = Start-Process -FilePath $probe -PassThru -NoNewWindow `
    -ArgumentList @('--once', '--json', '--port', '0', '--header-values') `
    -RedirectStandardOutput $probeOut -RedirectStandardError $probeErr

# Wait on the fixture's own first line, never on a guessed duration.
$url = $null
for ($i = 0; $i -lt 100; $i++) {
    Start-Sleep -Milliseconds 100
    if (Test-Path $probeOut) {
        $first = Get-Content $probeOut -TotalCount 1 -ErrorAction SilentlyContinue
        if ($first) { $url = "$first".Trim(); break }
    }
    if ($p.HasExited) { break }
}
if (-not $url) {
    if (-not $p.HasExited) { $p | Stop-Process -Force -ErrorAction SilentlyContinue }
    Exit-With 2 "the probe never announced itself"
}

$driveArgs = @('--url', "$url/", '--timeout', "$TimeoutSeconds")
if ($BrowserPath) { $driveArgs += @('--path', $BrowserPath) }
Start-Process -FilePath $finder -ArgumentList $driveArgs -NoNewWindow -Wait `
    -RedirectStandardOutput (Join-Path $scratch "drive.out") `
    -RedirectStandardError (Join-Path $scratch "drive.err") | Out-Null

$p.WaitForExit(10000) | Out-Null
if (-not $p.HasExited) { $p | Stop-Process -Force -ErrorAction SilentlyContinue }

$lines = @(Get-Content $probeOut -ErrorAction SilentlyContinue)
if ($lines.Count -lt 2) { Exit-With 2 "the probe captured nothing from the browser" }
try {
    $observed = $lines[1] | ConvertFrom-Json
} catch {
    Exit-With 2 "the probe's output is not JSON: $($lines[1])"
}
if (-not $observed.akamai) {
    Exit-With 2 "the browser completed no HTTP/2 request, so there is nothing to compare"
}

# ---------------------------------------------------------------------------
# What this repository claims
# ---------------------------------------------------------------------------

$goldenRoot = if ([System.IO.Path]::IsPathRooted($GoldenDir)) { $GoldenDir } else { Join-Path $repo $GoldenDir }
$goldenPath = Join-Path $goldenRoot "bit-cli-browser.json"
if (-not (Test-Path $goldenPath)) {
    Exit-With 2 "$goldenPath is not there; run scripts/check-fingerprint.ps1 -Update first"
}
$claim = (Get-Content -Raw $goldenPath | ConvertFrom-Json).fingerprint

$pageRs = Join-Path $repo "crates/bit-cli-core/src/page.rs"
$pageText = Get-Content -Raw $pageRs
$claimedMajor = $null
if ($pageText -match 'pub const BROWSER_MAJOR:\s*u32\s*=\s*(\d+)') { $claimedMajor = [int]$Matches[1] }

# ---------------------------------------------------------------------------
# Compare, field by field
# ---------------------------------------------------------------------------

$problems = @()

if ($claim.ja4 -cne $observed.ja4) {
    $problems += [ordered]@{
        field = "ja4"
        claim = $claim.ja4
        browser = $observed.ja4
        where = "crates/bit-cli-core/src/fetch.rs, through impit's fingerprint database"
    }
}
if ($claim.akamai -cne $observed.akamai) {
    $row = [ordered]@{
        field = "akamai"
        claim = $claim.akamai
        browser = $observed.akamai
        where = "crates/bit-cli-core/src/page.rs, BROWSER_H2_* and impit's fingerprint database"
        known = $null
    }
    if (-not $Strict -and (Test-KnownAkamai $claim.akamai $observed.akamai)) {
        $row.known = ($known | Where-Object { $_.field -eq 'akamai' } | Select-Object -First 1)
        $row.where = "$($row.known.entry): $($row.known.why)"
    }
    $problems += $row
}
$claimHeaders = @($claim.h2_headers)
$browserHeaders = @($observed.headers)
if (($claimHeaders -join '|') -cne ($browserHeaders -join '|')) {
    $problems += [ordered]@{
        field = "header order"
        claim = ($claimHeaders -join ', ')
        browser = ($browserHeaders -join ', ')
        where = "crates/bit-cli-core/src/page.rs, BROWSER_HEADERS"
    }
}

$browserMajor = $browser.major
if ($null -ne $browserMajor -and $null -ne $claimedMajor -and $browserMajor -ne $claimedMajor) {
    $problems += [ordered]@{
        field = "browser major"
        claim = "$claimedMajor"
        browser = "$browserMajor"
        where = "crates/bit-cli-core/src/page.rs, BROWSER_MAJOR"
    }
}

# The replacement, in the shape page.rs wants. Written whenever a capture
# happened, not only on a failure: a passing run's block is what a reader
# checks the file against by eye.
# A headless capture says `HeadlessChrome/151.0.0.0` where the browser a
# person runs says `Chrome/151.0.0.0`, and the same substitution reaches
# `sec-ch-ua` on some builds. Pasting the capture verbatim would ship a
# User-Agent that announces automation, which is the one thing this profile
# exists not to do, so the replacement is normalised and the substitution is
# reported beside it.
$headless = @()
$pairs = @($observed.header_pairs | ForEach-Object {
        $name = $_[0]
        $value = $_[1]
        if ($value -match 'HeadlessChrome') {
            $headless += $name
            $value = $value -replace 'HeadlessChrome', 'Chrome'
        }
        , @($name, $value)
    })
$rustHeaders = ($pairs | ForEach-Object {
        $name = $_[0]
        $value = ($_[1] -replace '\\', '\\\\') -replace '"', '\"'
        "    (`"$name`", `"$value`"),"
    }) -join "`n"

$recommend = [ordered]@{
    from             = [ordered]@{
        path    = $browser.path
        version = $browser.version
        major   = $browser.major
    }
    ja4              = $observed.ja4
    ja4_r            = $observed.ja4_r
    akamai           = $observed.akamai
    header_order     = $browserHeaders
    browser_headers  = "pub const BROWSER_HEADERS: &[(&str, &str)] = &[`n$rustHeaders`n];"
    headless_rewritten = $headless
    note             = "The ClientHello is impit's fingerprint database rather than a literal in this tree, so a JA4 that has moved is a vendored file to reconcile, not a constant to edit. patches/UPSTREAM.md says which."
}

$judged = @($problems | Where-Object { -not $_.known })
$pass = $judged.Count -eq 0

$report = [ordered]@{
    schema    = "browser-fingerprint/1"
    generated = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ss.fffZ")
    browser   = $browser
    observed  = [ordered]@{
        ja4          = $observed.ja4
        ja4_r        = $observed.ja4_r
        ja3          = $observed.ja3
        akamai       = $observed.akamai
        headers      = $browserHeaders
        header_pairs = $pairs
    }
    claim     = [ordered]@{
        ja4        = $claim.ja4
        akamai     = $claim.akamai
        h2_headers = $claimHeaders
        major      = $claimedMajor
    }
    pass      = $pass
    strict    = [bool]$Strict
    problems  = $problems
    judged    = $judged.Count
    recommend = $recommend
}

$jsonText = $report | ConvertTo-Json -Depth 8
if ($Out) {
    $outPath = if ([System.IO.Path]::IsPathRooted($Out)) { $Out } else { Join-Path $repo $Out }
    $parent = Split-Path -Parent $outPath
    if ($parent -and -not (Test-Path $parent)) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
    [System.IO.File]::WriteAllText($outPath, $jsonText)
}

if ($Json) {
    Write-Output $jsonText
} else {
    Write-Host ("browser  {0}" -f $browser.path)
    Write-Host ("version  {0}" -f $browser.version)
    Write-Host ""
    Write-Host ("  JA4     browser {0}" -f $observed.ja4)
    Write-Host ("          bit-cli {0}" -f $claim.ja4)
    Write-Host ("  akamai  browser {0}" -f $observed.akamai)
    Write-Host ("          bit-cli {0}" -f $claim.akamai)
    Write-Host ("  headers browser {0}" -f ($browserHeaders -join ', '))
    Write-Host ("          bit-cli {0}" -f ($claimHeaders -join ', '))
    Write-Host ""
    foreach ($problem in $problems) {
        $mark = if ($problem.known) { "note" } else { "FAIL" }
        Write-Host ("{0} {1}" -f $mark, $problem.field)
        Write-Host ("       claim   {0}" -f $problem.claim)
        Write-Host ("       browser {0}" -f $problem.browser)
        Write-Host ("       change  {0}" -f $problem.where)
    }
    if ($problems.Count -gt 0) { Write-Host "" }
    if ($pass) {
        $tail = if ($problems.Count -gt 0) { ", apart from what an entry already names" } else { "" }
        Write-Host "check-browser-fingerprint: the profile matches this browser$tail"
    } else {
        Write-Host "check-browser-fingerprint: $($judged.Count) field(s) disagree"
    }
    if (-not $pass -or $problems.Count -gt 0) {
        Write-Host ""
        Write-Host "the replacement, from $($browser.version):"
        Write-Host $recommend.browser_headers
        if ($headless.Count -gt 0) {
            Write-Host ""
            Write-Host ("HeadlessChrome was rewritten to Chrome in: {0}" -f ($headless -join ', '))
        }
        Write-Host ""
        Write-Host $recommend.note
    }
}

if (-not $pass) { exit 1 }
exit 0
