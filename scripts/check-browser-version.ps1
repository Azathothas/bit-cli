# Is the browser bit-cli claims to be one that anybody still runs?
#
# The defect it exists to catch: `TODO/cli-surface.md` T-244 ships a client
# that presents itself as a current Chrome. A profile pinned to a browser
# nobody runs is a *correct* fingerprint of a browser that does not exist,
# which is its own tell, and nothing in the tree notices when that happens.
# Browsers ship every four weeks and this repository does not.
#
# So this asks the vendors what stable actually is and compares the answer
# against `BROWSER_MAJOR` in `crates/bit-cli-core/src/page.rs`, which is the
# one number the profile is pinned to.
#
# Three sources, all first-party and all documented endpoints meant to be read
# by a program:
#
#   chrome    versionhistory.googleapis.com, the Chrome release API
#   firefox   product-details.mozilla.org, the file Mozilla's own release
#             tooling reads
#   edge      edgeupdates.microsoft.com, the enterprise update feed
#
# **Every fetch is trapped on its own.** One dead endpoint degrades that field
# and leaves the others intact, because a check that reports nothing when one
# vendor has an outage is a check that teaches people to ignore it.
#
# It **recommends** rather than only reporting, which is the operator's
# requirement: when the profile is behind, the output carries the replacement
# `BROWSER_MAJOR`, the replacement `BROWSER_USER_AGENT` and the replacement
# `sec-ch-ua` value, in the shape `page.rs` wants, so patching is applying a
# diff rather than doing the work again.
#
# What it cannot recommend is the TLS `ClientHello`, which is a cipher and
# extension list rather than a version string.
# `scripts/check-browser-fingerprint.ps1` is the half that reads that off a
# real browser.
#
# Usage:
#   pwsh scripts/check-browser-version.ps1
#   pwsh scripts/check-browser-version.ps1 -Json
#   pwsh scripts/check-browser-version.ps1 -Out bench/browser-versions.json
#   pwsh scripts/check-browser-version.ps1 -MaxBehind 0   # judge, do not record
#
# Exit 0 when the profile is no more than `-MaxBehind` majors behind Chrome
# stable, 1 when it is further behind than that, and 2 when it could not run:
# no network, or every source failed.
#
# **`-MaxBehind` defaults to 2 and that is a decision.** Chrome ships a major
# every four weeks, so one is normal between sessions and two is a month and a
# half of not looking. Three is a profile nobody has read.
#
# See TODO/cli-surface.md, T-244.

[CmdletBinding()]
param(
    # Report as one JSON object on stdout instead of a table.
    [switch]$Json,
    # Also write the JSON here. A path under bench/ is the convention.
    [string]$Out = "",
    # How many majors behind Chrome stable the profile may be before this
    # fails. Absent, the default below is used.
    [int]$MaxBehind = 2,
    # Seconds to wait for each vendor.
    [int]$TimeoutSeconds = 20
)

$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot

function Exit-With([int]$code, [string]$message) {
    [Console]::Error.WriteLine("check-browser-version: $message")
    exit $code
}

# ---------------------------------------------------------------------------
# What the profile claims
# ---------------------------------------------------------------------------

$pageRs = Join-Path $repo "crates/bit-cli-core/src/page.rs"
if (-not (Test-Path $pageRs)) { Exit-With 2 "$pageRs is not there" }
$pageText = Get-Content -Raw $pageRs

$claimedMajor = $null
if ($pageText -match 'pub const BROWSER_MAJOR:\s*u32\s*=\s*(\d+)') {
    $claimedMajor = [int]$Matches[1]
}
if ($null -eq $claimedMajor) {
    Exit-With 2 "page.rs has no BROWSER_MAJOR to compare against"
}

$claimedAgent = ""
if ($pageText -match 'pub const BROWSER_USER_AGENT:\s*&str\s*=\s*"([^"]*)"') {
    $claimedAgent = $Matches[1] -replace '\s+', ' '
}

# ---------------------------------------------------------------------------
# What the vendors say
# ---------------------------------------------------------------------------

# One fetch, trapped. Returns a hashtable with either `value` or `error`, never
# both, so a caller never has to ask whether a null meant "absent" or "failed".
function Get-Json([string]$Url) {
    try {
        $response = Invoke-WebRequest -Uri $Url -TimeoutSec $TimeoutSeconds `
            -MaximumRedirection 3 -UseBasicParsing `
            -Headers @{ 'User-Agent' = 'bit-cli-version-check' }
        return @{ value = ($response.Content | ConvertFrom-Json) }
    } catch {
        return @{ error = $_.Exception.Message }
    }
}

function Get-Chrome {
    $answer = Get-Json "https://versionhistory.googleapis.com/v1/chrome/platforms/win/channels/stable/versions?pageSize=1"
    if ($answer.error) { return @{ browser = 'chrome'; error = $answer.error } }
    $version = $answer.value.versions[0].version
    if (-not $version) { return @{ browser = 'chrome'; error = "the response carried no version" } }
    @{ browser = 'chrome'; version = $version; major = [int]($version -split '\.')[0] }
}

function Get-Firefox {
    $answer = Get-Json "https://product-details.mozilla.org/1.0/firefox_versions.json"
    if ($answer.error) { return @{ browser = 'firefox'; error = $answer.error } }
    $version = $answer.value.LATEST_FIREFOX_VERSION
    if (-not $version) { return @{ browser = 'firefox'; error = "the response carried no LATEST_FIREFOX_VERSION" } }
    @{ browser = 'firefox'; version = $version; major = [int]($version -split '\.')[0] }
}

function Get-Edge {
    $answer = Get-Json "https://edgeupdates.microsoft.com/api/products?view=enterprise"
    if ($answer.error) { return @{ browser = 'edge'; error = $answer.error } }
    $stable = @($answer.value | Where-Object { $_.Product -eq 'Stable' })
    if ($stable.Count -eq 0) { return @{ browser = 'edge'; error = "the feed carried no Stable product" } }
    # The feed lists one release per platform and architecture. The highest
    # version across them is the release, and taking the first would make the
    # answer depend on the order Microsoft happens to serve.
    $versions = @($stable[0].Releases | ForEach-Object { $_.ProductVersion } | Where-Object { $_ })
    if ($versions.Count -eq 0) { return @{ browser = 'edge'; error = "the Stable product carried no release" } }
    $version = ($versions | Sort-Object { [version]$_ } -Descending)[0]
    @{ browser = 'edge'; version = $version; major = [int]($version -split '\.')[0] }
}

$sources = @((Get-Chrome), (Get-Firefox), (Get-Edge))
$reached = @($sources | Where-Object { -not $_.error })
if ($reached.Count -eq 0) {
    $why = ($sources | ForEach-Object { "$($_.browser): $($_.error)" }) -join '; '
    Exit-With 2 "no vendor answered. $why"
}

$chrome = $sources | Where-Object { $_.browser -eq 'chrome' } | Select-Object -First 1

# ---------------------------------------------------------------------------
# The verdict, and the replacement when there is one
# ---------------------------------------------------------------------------

$behind = $null
$pass = $true
$detail = ""
$recommend = $null

if ($chrome.error) {
    $detail = "chrome could not be reached, so nothing was judged: $($chrome.error)"
} else {
    $behind = $chrome.major - $claimedMajor
    if ($behind -gt $MaxBehind) {
        $pass = $false
        $detail = "the profile claims Chrome $claimedMajor and stable is $($chrome.major), which is $behind major(s) behind"
    } elseif ($behind -gt 0) {
        $detail = "the profile claims Chrome $claimedMajor and stable is $($chrome.major), within the $MaxBehind allowed"
    } elseif ($behind -lt 0) {
        $detail = "the profile claims Chrome $claimedMajor and stable is $($chrome.major), which is ahead of stable"
    } else {
        $detail = "the profile claims Chrome $claimedMajor and stable is $($chrome.major)"
    }

    if ($behind -ne 0) {
        $newMajor = $chrome.major
        $recommend = [ordered]@{
            # Exactly the three literals in page.rs that carry a version, in
            # the shape they are written there. What this cannot produce is
            # the ClientHello: see check-browser-fingerprint.ps1.
            file               = "crates/bit-cli-core/src/page.rs"
            browser_major      = $newMajor
            browser_user_agent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/$newMajor.0.0.0 Safari/537.36"
            sec_ch_ua          = "`"Not=A?Brand`";v=`"99`", `"Google Chrome`";v=`"$newMajor`", `"Chromium`";v=`"$newMajor`""
            unresolved         = @(
                "the TLS ClientHello, which is a cipher and extension list rather than a version string",
                "the HTTP/2 SETTINGS values, which are numbers a browser chooses rather than a version"
            )
            proof              = "https://versionhistory.googleapis.com/v1/chrome/platforms/win/channels/stable/versions?pageSize=1 answered $($chrome.version)"
            next               = "pwsh scripts/check-browser-fingerprint.ps1 -Recommend, on a machine with that Chrome installed"
        }
    }
}

$report = [ordered]@{
    schema    = "browser-versions/1"
    generated = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ss.fffZ")
    profile   = [ordered]@{
        browser_major      = $claimedMajor
        browser_user_agent = $claimedAgent
        source             = "crates/bit-cli-core/src/page.rs"
    }
    latest    = @($sources | ForEach-Object {
            $row = [ordered]@{ browser = $_.browser }
            if ($_.error) { $row.error = $_.error } else {
                $row.version = $_.version
                $row.major = $_.major
            }
            [pscustomobject]$row
        })
    behind    = $behind
    max_behind = $MaxBehind
    pass      = $pass
    detail    = $detail
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
    Write-Host ("profile  Chrome {0}" -f $claimedMajor)
    foreach ($s in $sources) {
        if ($s.error) {
            Write-Host ("{0,-9} unreachable: {1}" -f $s.browser, $s.error)
        } else {
            Write-Host ("{0,-9} {1}" -f $s.browser, $s.version)
        }
    }
    Write-Host ""
    Write-Host ("check-browser-version: {0}" -f $detail)
    if ($recommend) {
        Write-Host ""
        Write-Host "the replacement, for $($recommend.file):"
        Write-Host ("  pub const BROWSER_MAJOR: u32 = {0};" -f $recommend.browser_major)
        Write-Host ("  pub const BROWSER_USER_AGENT: &str = `"{0}`";" -f $recommend.browser_user_agent)
        Write-Host ("  sec-ch-ua: {0}" -f $recommend.sec_ch_ua)
        Write-Host "  still unresolved, and this cannot produce them:"
        foreach ($u in $recommend.unresolved) { Write-Host "    $u" }
        Write-Host ("  next: {0}" -f $recommend.next)
    }
}

if (-not $pass) { exit 1 }
exit 0
