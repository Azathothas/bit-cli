# Does `seed --listener-check` see its own listener stop answering?
#
# This is the acceptance for `TODO/peers.md` T-020's second finding. The first
# finding was a panic and is fixed. The second is that `librqbit` 9.0.0's
# accept loop clears one queued handshake check per connection it accepts, so
# a run of peers that close before they handshake leaves a backlog and every
# peer after it waits behind one. The target then accepts TCP, answers no
# handshake, and goes on reporting itself as seeding. Nothing a supervisor
# watches says so: the process is alive and the port is open.
#
# `--listener-check` is what says so. Four cases:
#
#   healthy   A seeder nobody has poisoned. Every probe is answered, the run
#             is not stopped, and the probes leave nothing in `peer_detail`:
#             a probe completes a real handshake, so the session keeps a peer
#             row for it, and those rows are this process talking to itself.
#   poisoned  The same seeder, then `bench swarm --peers N --torrents 1`,
#             which is the load that leaves the backlog. The run must stop
#             with `"stopped": "listener_unhealthy"` and exit 17.
#   off       No `--listener-check`. The same poison, and the run carries on
#             to its own `--stop-after`, which is exit 9 and
#             `"stopped": "deadline"`. This is what proves the flag is what
#             stopped the run in `poisoned` rather than the poison stopping it
#             on its own.
#   recovery  How many incoming connections it takes to clear the backlog the
#             load left. This is the derivation for the threshold of three,
#             measured rather than argued.
#
# Usage:
#   pwsh scripts/check-listener.ps1
#   pwsh scripts/check-listener.ps1 -Poison 40
#
# Exits 0 when every case behaves as described, 1 when one does not, and 2
# when the check could not run. The record goes to
# bench/listener-<timestamp>.json.
#
# See TODO/peers.md, T-020.

[CmdletBinding()]
param(
    [int]$Poison = 20,
    [int]$PayloadMiB = 4,
    [string]$Interval = "2s",
    [string]$Root = ".tmp/listener",
    [string]$ReportDir = "bench",
    [ValidateSet("debug", "release")]
    [string]$Profile = "release",
    [switch]$Keep
)

$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot

$script:Seeder = $null

function Stop-Background {
    if ($script:Seeder -and -not $script:Seeder.HasExited) {
        try { $script:Seeder.Kill() } catch { }
    }
    $script:Seeder = $null
}

function Exit-With([int]$code, [string]$message) {
    [Console]::Error.WriteLine("check-listener: $message")
    Stop-Background
    exit $code
}

function Get-Timestamp {
    (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ss.fffZ")
}

function Write-Step($message) {
    Write-Host "$(Get-Timestamp) $message"
}

trap { Stop-Background; throw }

$exe = if ($IsWindows -or $env:OS -eq "Windows_NT") { ".exe" } else { "" }
$bitCli = Join-Path $repo "target/$Profile/bit-cli$exe"
if (-not (Test-Path $bitCli)) {
    Exit-With 2 "missing $bitCli. Build it first: cargo build --$Profile --workspace --bins --examples"
}

if (-not [System.IO.Path]::IsPathRooted($Root)) { $Root = Join-Path $repo $Root }
if (Test-Path $Root) { Remove-Item -Recurse -Force $Root -ErrorAction SilentlyContinue }
New-Item -ItemType Directory -Force -Path $Root | Out-Null
$Root = (Resolve-Path $Root).Path
if (-not [System.IO.Path]::IsPathRooted($ReportDir)) { $ReportDir = Join-Path $repo $ReportDir }
New-Item -ItemType Directory -Force -Path $ReportDir | Out-Null

# ---------------------------------------------------------------------------
# A payload, a torrent, and a seeder serving it
# ---------------------------------------------------------------------------

$serve = Join-Path $Root "serve"
New-Item -ItemType Directory -Force -Path $serve | Out-Null
Write-Step "building a $PayloadMiB MiB payload"
$payloadBytes = [byte[]]::new($PayloadMiB * 1024 * 1024)
[int64]$state = 29
for ($i = 0; $i -lt $payloadBytes.Length; $i++) {
    $state = ($state * 1103515245 + 12345) -band 0x7FFFFFFF
    $payloadBytes[$i] = [byte](($state -shr 16) -band 0xFF)
}
[System.IO.File]::WriteAllBytes((Join-Path $serve "payload.bin"), $payloadBytes)

# Through Start-Process with redirect files, like every other check script:
# whether a line on stderr ends the run otherwise depends on the host's pwsh
# version. See TODO/windows.md under T-075.
$torrent = Join-Path $Root "payload.torrent"
$createProc = Start-Process -FilePath $bitCli -ArgumentList @(
    "create", (Join-Path $serve "payload.bin"), "--piece-length", "256KiB",
    "--no-creation-date", "--output", $torrent, "--force", "--json"
) -PassThru -NoNewWindow -RedirectStandardOutput (Join-Path $Root "create.out") `
    -RedirectStandardError (Join-Path $Root "create.err")
$createProc.WaitForExit(60000) | Out-Null
if ($createProc.ExitCode -ne 0) {
    Exit-With 2 "bit-cli create exited $($createProc.ExitCode): $(Get-Content (Join-Path $Root 'create.err') -Raw)"
}

$script:SeederIndex = 0

# Port zero, and the port comes back out of the seeder's own event stream. A
# port this script picked could already be in use, and dialling it would
# measure whatever else was listening.
function Start-Seeder([string[]]$extra, [string]$stopAfter) {
    Stop-Background
    $script:SeederIndex++
    $tag = "seed-$($script:SeederIndex)"
    $script:SeedOut = Join-Path $Root "$tag.out"
    $script:SeedErr = Join-Path $Root "$tag.err"
    $arguments = @(
        "--jsonl", "seed", $torrent, "--dir", $serve, "--port", "0",
        "--no-tracker", "--no-dht", "--no-lsd", "--stop-after", $stopAfter,
        "--report-interval", "1s"
    ) + $extra
    $script:Seeder = Start-Process -FilePath $bitCli -ArgumentList $arguments `
        -PassThru -NoNewWindow -RedirectStandardOutput $script:SeedOut `
        -RedirectStandardError $script:SeedErr

    $addr = $null
    for ($attempt = 0; $attempt -lt 150; $attempt++) {
        Start-Sleep -Milliseconds 100
        if ($script:Seeder.HasExited) {
            Exit-With 2 "the seeder exited $($script:Seeder.ExitCode): $(Get-Content $script:SeedErr -Raw)"
        }
        foreach ($line in (Get-Content $script:SeedOut -ErrorAction SilentlyContinue)) {
            try { $event = $line | ConvertFrom-Json } catch { continue }
            if ($event.listen_addr) { $addr = $event.listen_addr }
        }
        if ($addr) { break }
    }
    if (-not $addr) { Exit-With 2 "the seeder never printed a listen address. stderr: $(Get-Content $script:SeedErr -Raw)" }
    $script:Target = "127.0.0.1:$(($addr -split ':')[-1])"
    Write-Step "seeder $($script:SeederIndex) on $($script:Target)"
    $script:Target
}

# Every `progress` event the seeder has written so far.
function Get-Progress {
    $events = @()
    foreach ($line in (Get-Content $script:SeedOut -ErrorAction SilentlyContinue)) {
        try { $event = $line | ConvertFrom-Json } catch { continue }
        if ($null -ne $event.uploaded_bytes) { $events += $event }
    }
    $events
}

# Wait for a condition rather than for a duration. A test that waits out a
# guessed number of seconds is asserting a scheduling outcome it does not
# control; see TODO/RULES.md section 5.
function Wait-For([scriptblock]$condition, [int]$seconds, [string]$what) {
    $deadline = (Get-Date).AddSeconds($seconds)
    while ((Get-Date) -lt $deadline) {
        if (& $condition) { return $true }
        Start-Sleep -Milliseconds 250
    }
    Write-Step "  timed out after ${seconds}s waiting for $what"
    $false
}

function Invoke-Poison([string]$name) {
    $report = Join-Path $Root "$name.json"
    $work = Join-Path $Root "work/$name"
    New-Item -ItemType Directory -Force -Path $work | Out-Null
    $arguments = @(
        "bench", "swarm", $script:Target, "--report", $report, "--format", "json",
        "--peers", "$Poison", "--torrents", "1", "--disk-budget", "256MiB",
        "--duration", "12s", "--warmup", "500ms", "--connect-timeout", "5s",
        "--dir", $work
    )
    $process = Start-Process -FilePath $bitCli -ArgumentList $arguments -PassThru -NoNewWindow `
        -RedirectStandardOutput (Join-Path $Root "$name.out") `
        -RedirectStandardError (Join-Path $Root "$name.err")
    if (-not $process.WaitForExit(120000)) { try { $process.Kill() } catch { } }
    if (Test-Path $report) { return (Get-Content $report -Raw | ConvertFrom-Json) }
    $null
}

$cases = @()
$failures = @()
function Add-Failure([string]$name, [string]$message) {
    $script:failures += "${name}: $message"
}

# ---------------------------------------------------------------------------
# healthy: nobody has poisoned it, so every probe is answered
# ---------------------------------------------------------------------------

Write-Step "case healthy (--listener-check $Interval, no load)"
Start-Seeder @("--listener-check", $Interval) "90s" | Out-Null

# Three answered probes is the same number the stop condition needs to see
# fail, so this is the exact counterpart of the case below.
$sawThree = Wait-For {
    $last = (Get-Progress) | Select-Object -Last 1
    $last -and $last.listener -and $last.listener.probes -ge 3
} 40 "three probes"
$healthySample = (Get-Progress) | Select-Object -Last 1
if (-not $sawThree) {
    Add-Failure "healthy" "the seeder made fewer than three probes in 40s, so the case measured nothing"
}
elseif (-not $healthySample.listener.healthy) {
    Add-Failure "healthy" "an unpoisoned seeder reported its own listener unhealthy: $($healthySample.listener | ConvertTo-Json -Compress)"
}
if ($healthySample.listener.consecutive_failures -ne 0) {
    Add-Failure "healthy" "$($healthySample.listener.consecutive_failures) consecutive failures against a seeder nothing had touched"
}
# The probes each complete a real handshake, so the session records a peer row
# for each. Those rows are this process, and the reported peer list drops them.
$probeRows = @($healthySample.peer_detail).Count
if ($probeRows -ne 0) {
    Add-Failure "healthy" "$probeRows peer rows after $($healthySample.listener.probes) probes; the probe's own rows are not being dropped"
}
if ($script:Seeder.HasExited) {
    Add-Failure "healthy" "the seeder exited $($script:Seeder.ExitCode) with nothing wrong with it"
}
$cases += [pscustomobject][ordered]@{
    case                 = "healthy"
    probes               = $healthySample.listener.probes
    failed               = $healthySample.listener.failed
    consecutive_failures = $healthySample.listener.consecutive_failures
    last_rtt_ms          = $healthySample.listener.last_rtt_ms
    peer_rows            = $probeRows
    peers_seen           = $healthySample.peers.seen
    still_running        = (-not $script:Seeder.HasExited)
}

# ---------------------------------------------------------------------------
# poisoned: the same seeder, and the load that leaves the backlog
# ---------------------------------------------------------------------------

Write-Step "case poisoned ($Poison connections for a torrent the seeder does not have)"
$load = Invoke-Poison "poison"
$exited = $script:Seeder.WaitForExit(90000)
$poisonExit = if ($exited) { $script:Seeder.ExitCode } else { $null }
if (-not $exited) {
    Add-Failure "poisoned" "the seeder did not stop within 90s of the load; --listener-check did not see the outage"
    Stop-Background
}
$final = $null
foreach ($line in (Get-Content $script:SeedOut -ErrorAction SilentlyContinue)) {
    try { $event = $line | ConvertFrom-Json } catch { continue }
    if ($event.stopped) { $final = $event }
}
if ($exited) {
    if ($poisonExit -ne 17) {
        Add-Failure "poisoned" "exited $poisonExit, expected 17. stderr: $(Get-Content $script:SeedErr -Raw)"
    }
    if (-not $final) {
        Add-Failure "poisoned" "the seeder wrote no report"
    }
    elseif ($final.stopped -ne "listener_unhealthy") {
        Add-Failure "poisoned" "stopped=$($final.stopped), expected listener_unhealthy"
    }
    elseif ($final.listener.consecutive_failures -lt 3) {
        Add-Failure "poisoned" "stopped after $($final.listener.consecutive_failures) consecutive failures, expected at least 3"
    }
}
$cases += [pscustomobject][ordered]@{
    case                 = "poisoned"
    load_connected       = $load.swarm.peers_connected
    load_handshaked      = $load.swarm.peers_handshaked
    exit_code            = $poisonExit
    stopped              = $final.stopped
    probes               = $final.listener.probes
    failed               = $final.listener.failed
    consecutive_failures = $final.listener.consecutive_failures
    last_failure         = $final.listener.last_failure
}
Stop-Background

# ---------------------------------------------------------------------------
# off: the flag is what stopped the run, not the load
# ---------------------------------------------------------------------------

Write-Step "case off (no --listener-check, same load)"
Start-Seeder @() "30s" | Out-Null
Invoke-Poison "poison_off" | Out-Null
$exitedOff = $script:Seeder.WaitForExit(90000)
$offExit = if ($exitedOff) { $script:Seeder.ExitCode } else { $null }
$offFinal = $null
foreach ($line in (Get-Content $script:SeedOut -ErrorAction SilentlyContinue)) {
    try { $event = $line | ConvertFrom-Json } catch { continue }
    if ($event.stopped) { $offFinal = $event }
}
if (-not $exitedOff) {
    Add-Failure "off" "the seeder never reached its own 30s deadline"
    Stop-Background
}
# 9 rather than 0: reaching `--stop-after` is `Stopped::Deadline`, which is a
# deadline that passed. What matters here is that it is not 17.
elseif ($offExit -ne 9) {
    Add-Failure "off" "exited $offExit, expected 9: without the flag the poison must not stop the run early"
}
elseif ($offFinal.stopped -ne "deadline") {
    Add-Failure "off" "stopped=$($offFinal.stopped), expected deadline"
}
# Absent rather than null, so a consumer selects on the key.
$offSample = (Get-Progress) | Select-Object -Last 1
if ($offSample -and $offSample.PSObject.Properties.Name -contains "listener" -and $null -ne $offSample.listener) {
    Add-Failure "off" "a run that did not ask for the check still reported a listener block"
}
$cases += [pscustomobject][ordered]@{
    case          = "off"
    exit_code     = $offExit
    stopped       = $offFinal.stopped
    listener_key  = ($null -ne $offSample.listener)
}
Stop-Background

# ---------------------------------------------------------------------------
# recovery: how much traffic it takes to clear the backlog
# ---------------------------------------------------------------------------
#
# This is the derivation for the threshold of three, so it is measured rather
# than asserted in prose. No `--listener-check` here: the probe would clear the
# backlog itself and this case is about what a real peer meets.
#
# `librqbit`'s accept loop drains its pending set through a `select!` arm whose
# pattern is `Some(Ok(..))`. A check that resolves to an error fails that
# pattern, which disables the arm for the rest of that `select!` call, so the
# loop cannot come round again until `accept` fires. One queued error therefore
# costs one incoming connection. A check that succeeds matches, so it ends an
# iteration without needing one.

Write-Step "case recovery (how many connections clear a $Poison connection backlog)"
Start-Seeder @() "300s" | Out-Null
$recoveryProbeDir = Join-Path $Root "work/recovery"
New-Item -ItemType Directory -Force -Path $recoveryProbeDir | Out-Null

function Invoke-Probe([int]$index) {
    $report = Join-Path $Root "recover$index.json"
    $work = Join-Path $recoveryProbeDir "$index"
    New-Item -ItemType Directory -Force -Path $work | Out-Null
    $process = Start-Process -FilePath $bitCli -ArgumentList @(
        "bench", "swarm", $script:Target, "--report", $report, "--format", "json",
        "--for", $torrent, "--peers", "1", "--disk-budget", "64MiB",
        "--duration", "5s", "--warmup", "200ms", "--dir", $work
    ) -PassThru -NoNewWindow -RedirectStandardOutput (Join-Path $Root "recover$index.out") `
        -RedirectStandardError (Join-Path $Root "recover$index.err")
    if (-not $process.WaitForExit(60000)) { try { $process.Kill() } catch { }; return -1 }
    if (-not (Test-Path $report)) { return -1 }
    (Get-Content $report -Raw | ConvertFrom-Json).swarm.peers_handshaked
}

$servedBefore = Invoke-Probe 0
if ($servedBefore -ne 1) {
    Add-Failure "recovery" "the seeder did not serve a peer before the load, so the case measured nothing"
}
Invoke-Poison "poison_recovery" | Out-Null
$ceiling = 3 * $Poison + 10
$recovered = -1
for ($k = 1; $k -le $ceiling; $k++) {
    if ($script:Seeder.HasExited) { break }
    if ((Invoke-Probe $k) -ge 1) { $recovered = $k; break }
}
if ($recovered -eq 1) {
    Add-Failure "recovery" "the load left no backlog at all, so nothing here was measured"
}
elseif ($recovered -lt 0) {
    Add-Failure "recovery" "$ceiling connections did not clear a $Poison connection backlog"
}
Write-Step "  $recovered connections cleared a $Poison connection backlog"
$cases += [pscustomobject][ordered]@{
    case                    = "recovery"
    served_before           = $servedBefore
    poison_connections      = $Poison
    connections_to_recover  = $recovered
    ceiling                 = $ceiling
}
Stop-Background

# ---------------------------------------------------------------------------
# The record
# ---------------------------------------------------------------------------

$verdict = if ($failures.Count -eq 0) { "pass" } else { "fail" }
$stamp = (Get-Date).ToUniversalTime().ToString("yyyyMMddTHHmmssfffZ")
$reportPath = Join-Path $ReportDir "listener-$stamp.json"

[pscustomobject][ordered]@{
    kind           = "listener_check"
    schema_version = "1"
    generated_at   = Get-Timestamp
    host           = [ordered]@{
        machine = $env:COMPUTERNAME
        os      = [System.Runtime.InteropServices.RuntimeInformation]::OSDescription
    }
    parameters     = [ordered]@{
        poison_connections = $Poison
        payload_mib        = $PayloadMiB
        interval           = $Interval
        profile            = $Profile
    }
    cases          = @($cases)
    verdict        = $verdict
    failures       = @($failures)
    notes          = @(
        "The probe completes a real handshake for a torrent the seeder holds rather than an unknown one. An unknown info hash is cheaper, and it is the wrong measurement: it resolves to an error inside the session, which adds an entry to the same backlog it is measuring. A completed handshake takes one off instead.",
        "That costs one peer row per probe, which librqbit keeps in a terminal state and never reclaims. The reported peer list drops them by the port the probe dialled from, which is the mechanism the web seed bridge already uses.",
        "Three failures in a row is derived, not picked. The accept loop clears one queued check per connection it accepts, so one failure means a backlog a real peer would have cleared by arriving, and three means the backlog outlived three connections.",
        "The off case exists because poisoned on its own does not prove the flag did anything: the load could have stopped the run by itself.",
        "recovery runs without the flag on purpose. The probe clears one queued check per answered handshake, so a seeder being probed is a seeder being repaired, and this case is about what a peer that is not us meets."
    )
} | ConvertTo-Json -Depth 10 | Set-Content -Path $reportPath -Encoding utf8NoBOM

Write-Host ""
$cases | Format-Table -AutoSize | Out-String -Width 200 | Write-Host
Write-Host "report:  $reportPath"
Write-Host "verdict: $verdict"

if (-not $Keep) { Remove-Item -Recurse -Force $Root -ErrorAction SilentlyContinue }

if ($failures.Count -gt 0) {
    Write-Host ""
    foreach ($failure in $failures) { [Console]::Error.WriteLine("check-listener: $failure") }
    exit 1
}
exit 0
