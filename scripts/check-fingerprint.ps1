# What does this client actually put on the wire?
#
# The defect it exists to catch: `TODO/cli-surface.md` T-244 fetches a source
# document while presenting as a browser, and every part of that presentation
# is invisible from inside the process. A client's own view of its handshake is
# the view it intended. The header set can be read from the code; the TLS
# `ClientHello` and the HTTP/2 SETTINGS frame are decided by `rustls` and `h2`
# and change when either is upgraded, silently and without a test failing.
#
# So this drives `bit-cli` at `loopback-tlsprobe`, reads the fingerprint off
# the wire, and compares it against a golden committed under `fingerprints/`.
# Nothing here touches the network.
#
# Two captures, because they need opposite things:
#
#   raw         no handshake is completed, so nothing has to be disabled to
#               reach it. This is where JA4 is read, and it is the JA4 that
#               ships: a client told to skip certificate verification can fall
#               back to a different `signature_algorithms` list, and the JA4
#               read through that handshake is not the one an origin sees.
#   plain       cleartext HTTP/1.1, which is where the header order is read.
#
# **The Akamai HTTP/2 fingerprint of this client is not captured here, and the
# reason is a good one.** It only exists after a TLS handshake completes and
# ALPN picks `h2`, and the probe's certificate is self signed with no CA behind
# it, so `bit-cli` refuses it. Reaching it would need `bit-cli` to stop
# verifying certificates, and a flag that does that is not worth adding to a
# shipping binary for a test. The probe reads the Akamai fingerprint of a
# client that will accept the certificate, which is how it was used to measure
# the impersonating candidate in T-244.
#
# **JA4 is asserted and JA3 is not.** JA4 sorts ciphers and extensions before
# hashing, so it survives a client that shuffles its extension order; JA3
# preserves wire order and flakes. JA3 is recorded for a reader and never
# compared.
#
# Usage:
#   pwsh scripts/check-fingerprint.ps1
#   pwsh scripts/check-fingerprint.ps1 -Update      # rewrite the goldens
#   pwsh scripts/check-fingerprint.ps1 -Json
#
# Exit 0 when every capture matched its golden, 1 when one did not, 2 when it
# could not run. With no golden present it records what it saw and exits 0,
# saying so: a check that has never been given an answer must not invent one.
#
# See TODO/cli-surface.md, T-244.

[CmdletBinding()]
param(
    [switch]$Json,
    # Rewrite the goldens from what was captured. A deliberate act: the point
    # of the check is that the fingerprint does not move without somebody
    # deciding it should.
    [switch]$Update,
    [ValidateSet("", "browser", "plain")]
    [string]$Profile = "",
    [string]$GoldenDir = "fingerprints",
    [ValidateSet("debug", "release")]
    [string]$Build = "release"
)

$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot

function Exit-With([int]$code, [string]$message) {
    [Console]::Error.WriteLine("check-fingerprint: $message")
    exit $code
}

$exeDir = Join-Path $repo "target/$Build"
$bit = Join-Path $exeDir "bit-cli.exe"
if (-not (Test-Path $bit)) { $bit = Join-Path $exeDir "bit-cli" }
$probe = Join-Path $exeDir "examples/loopback-tlsprobe.exe"
if (-not (Test-Path $probe)) { $probe = Join-Path $exeDir "examples/loopback-tlsprobe" }
foreach ($p in @($bit, $probe)) {
    if (-not (Test-Path $p)) {
        Exit-With 2 "$p is missing; run: cargo build --$Build --bins --examples"
    }
}

$goldenRoot = if ([System.IO.Path]::IsPathRooted($GoldenDir)) { $GoldenDir } else { Join-Path $repo $GoldenDir }
if ($Update) { New-Item -ItemType Directory -Force -Path $goldenRoot | Out-Null }
$scratch = Join-Path $repo ".tmp/fingerprint"
New-Item -ItemType Directory -Force -Path $scratch | Out-Null

# Start the probe, point one bit-cli run at it, and return the capture it made.
# `$Raw` decides which half of the fingerprint is reachable.
function Get-Capture([string]$profileName, [string]$Mode) {
    $tag = "$profileName-$Mode"
    $out = Join-Path $scratch "$tag-out.txt"
    $err = Join-Path $scratch "$tag-err.txt"
    $probeArgs = @('--once', '--json', '--port', '0')
    if ($Mode -eq 'raw') { $probeArgs += '--raw' }
    if ($Mode -eq 'plain') { $probeArgs += '--plain' }

    $p = Start-Process -FilePath $probe -ArgumentList $probeArgs -PassThru -NoNewWindow `
        -RedirectStandardOutput $out -RedirectStandardError $err

    # Wait on the fixture's own first line, never on a guessed duration.
    $url = $null
    for ($i = 0; $i -lt 100; $i++) {
        Start-Sleep -Milliseconds 100
        if (Test-Path $out) {
            $first = Get-Content $out -TotalCount 1 -ErrorAction SilentlyContinue
            if ($first) { $url = "$first".Trim(); break }
        }
        if ($p.HasExited) { break }
    }
    if (-not $url) {
        if (-not $p.HasExited) { $p | Stop-Process -Force -ErrorAction SilentlyContinue }
        return @{ error = "the probe never announced itself" }
    }

    # The fetch always fails: the certificate is camouflage and in raw mode
    # there is no handshake at all. The ClientHello is on the wire before
    # either of those matters, which is the whole point.
    $runOut = Join-Path $scratch "$tag-run.txt"
    $argv = @('info', "$url/one.torrent", '--page-client', $profileName, '--timeout', '10s')
    Start-Process -FilePath $bit -ArgumentList $argv -NoNewWindow -Wait `
        -RedirectStandardOutput $runOut -RedirectStandardError "$runOut.err" | Out-Null

    $p.WaitForExit(10000) | Out-Null
    if (-not $p.HasExited) { $p | Stop-Process -Force -ErrorAction SilentlyContinue }

    $lines = @(Get-Content $out -ErrorAction SilentlyContinue)
    if ($lines.Count -lt 2) { return @{ error = "the probe captured nothing" } }
    try {
        return @{ capture = ($lines[1] | ConvertFrom-Json) }
    } catch {
        return @{ error = "the probe's output is not JSON: $($lines[1])" }
    }
}

$profiles = if ($Profile) { @($Profile) } else { @('browser', 'plain') }
$results = @()

foreach ($name in $profiles) {
    $raw = Get-Capture $name 'raw'
    $plain = Get-Capture $name 'plain'
    if ($raw.error) { Exit-With 2 "$name raw capture: $($raw.error)" }
    if ($plain.error) { Exit-With 2 "$name plain capture: $($plain.error)" }

    $observed = [ordered]@{
        profile = $name
        # From the raw capture, which is the one that ships.
        ja4     = $raw.capture.ja4
        ja4_r   = $raw.capture.ja4_r
        # Recorded for a reader and never compared: JA3 preserves wire order.
        ja3     = $raw.capture.ja3
        # From the cleartext capture. `Host` is dropped: it carries the port
        # the probe happened to bind, so keeping it would make the golden
        # depend on a free port.
        headers = @($plain.capture.headers | Where-Object { $_ -ne 'host' })
    }

    $goldenPath = Join-Path $goldenRoot "bit-cli-$name.json"
    $row = [ordered]@{
        profile  = $name
        ja4      = $observed.ja4
        headers  = $observed.headers.Count
        golden   = (Test-Path $goldenPath)
        pass     = $true
        detail   = ""
        problems = @()
    }

    if ($Update) {
        $doc = [ordered]@{
            schema      = "fingerprint/1"
            captured    = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ss.fffZ")
            note        = "Captured off the wire by loopback-tlsprobe. ja4 and ja4_r come from a --raw capture, headers from a --plain one. ja3 is recorded and never asserted, because it preserves wire order and flakes."
            bit_cli     = (& $bit --version 2>$null | Select-Object -First 1)
            fingerprint = $observed
        }
        [System.IO.File]::WriteAllText($goldenPath, ($doc | ConvertTo-Json -Depth 6))
        $row.detail = "wrote $goldenPath"
        $row.golden = $true
    } elseif (-not (Test-Path $goldenPath)) {
        $row.detail = "no golden at $goldenPath, recorded only; pass -Update to write one"
    } else {
        $want = (Get-Content -Raw $goldenPath | ConvertFrom-Json).fingerprint
        $problems = @()
        if ($want.ja4 -cne $observed.ja4) {
            $problems += "ja4 want '$($want.ja4)' got '$($observed.ja4)'"
        }
        if ($want.ja4_r -cne $observed.ja4_r) {
            $problems += "ja4_r differs, which says where: want '$($want.ja4_r)' got '$($observed.ja4_r)'"
        }
        $wantHeaders = @($want.headers)
        if (($wantHeaders -join '|') -cne ($observed.headers -join '|')) {
            $problems += "header order want [$($wantHeaders -join ', ')] got [$($observed.headers -join ', ')]"
        }
        $row.problems = $problems
        $row.pass = $problems.Count -eq 0
        $row.detail = if ($problems.Count -eq 0) { "matches the golden" } else { $problems[0] }
    }

    $results += [pscustomobject]$row
    $mark = if ($row.pass) { "ok  " } else { "FAIL" }
    Write-Host ("{0} {1,-8} {2}" -f $mark, $name, $row.detail)
    Write-Host ("       JA4     {0}" -f $observed.ja4)
    Write-Host ("       headers {0}" -f ($observed.headers -join ', '))
    foreach ($problem in ($row.problems | Select-Object -Skip 1)) {
        Write-Host "       $problem"
    }
}

$failed = @($results | Where-Object { -not $_.pass })
$report = [ordered]@{
    schema    = "fingerprint-check/1"
    generated = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ss.fffZ")
    profiles  = $results.Count
    failed    = $failed.Count
    pass      = $failed.Count -eq 0
    results   = $results
}

if ($Json) {
    $report | ConvertTo-Json -Depth 6
} else {
    Write-Host ""
    Write-Host ("check-fingerprint: {0} profile(s), {1} failed" -f $results.Count, $failed.Count)
}

if ($failed.Count -gt 0) { exit 1 }
exit 0
