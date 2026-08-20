# Watch one long-lived process for a slope.
#
# `TODO/memory.md` T-040 is a report of RSS and open descriptors climbing until
# the process failed, over a run measured in days. `bit-cli seed` is the shape
# that reaches: a single process holding a payload, a listener, a tracker
# announce timer, and whatever peers turn up.
#
# The subject is one seeder. Everything else here exists to give it something
# to do, because an idle process is flat by construction and a flat line from
# an idle process says nothing about a busy one.
#
# Six workloads, so a slope names a subsystem rather than "the process":
#
#   idle      a seeder with no tracker and nothing connecting. The control.
#             Any slope here is the session's own timers or the sampler.
#   announce  a loopback tracker at a short interval. The reporter's growth
#             started after changing trackers, so this is the announce path on
#             its own. The tracker never expires a peer, so the peer list the
#             seeder is handed grows for the whole run, which is the shape a
#             busy public tracker has.
#   leech     real downloads against the seeder, one finishing and another
#             starting. Peer sessions arriving and leaving, with payload
#             moving and files opening.
#   steady    announce and leech together. The deployment shape, and the
#             default, because those are the two paths a seeder runs for days.
#   churn     connections that open and close without handshaking. This is
#             T-020's shape and the known positive: it strands sockets, so a
#             run with it should show a slope, which is what says the sampler
#             can see one.
#   all       steady plus churn.
#
# `all` is not the default and should not be the six-hour run. Churn strands
# sockets at about 30,000 handles an hour (measured, see TODO/memory.md), which
# is T-020 rather than T-040 and swamps every other series in the same chart.
# It also starves the leechers: the same run that completed 22 downloads in two
# minutes without churn completed 1 and failed 2 with it.
#
# Three series, sampled every -SampleSeconds from outside the process:
# resident memory, handle count, and TCP socket states. The seeder reports the
# first two itself in every `progress` event under `--jsonl`, and the summary
# checks the two against each other, because a sampler that disagrees with the
# subject is measuring something else.
#
# Usage:
#   pwsh scripts/soak.ps1                             six hours, the deployment
#   pwsh scripts/soak.ps1 -Minutes 20 -Workload churn
#   pwsh scripts/soak.ps1 -Minutes 360 -RssCeilingMiBPerHour 8
#
# Writes bench/soak-<timestamp>.csv with one row per sample and
# bench/soak-<timestamp>.json with the parameters, the slopes, and the verdict.
#
# Exits 0 when the run completed and every named ceiling held, 1 when a ceiling
# was passed or the seeder died, and 2 when the check could not run. With no
# ceiling named the slopes are recorded rather than judged, because T-040 is
# open and this script is what measures it.
#
# See TODO/memory.md, T-040.

[CmdletBinding()]
param(
    # Wall clock. T-040's acceptance is six hours, which is the default.
    [int]$Minutes = 360,
    [int]$SampleSeconds = 30,
    [ValidateSet("steady", "all", "idle", "announce", "leech", "churn")]
    [string]$Workload = "steady",
    # Small on purpose: the leech cycle rate is what matters, not the bytes.
    [int]$PayloadMiB = 16,
    # Downloads in flight against the seeder. Each one is a peer session that
    # connects, transfers, and leaves.
    [int]$Leechers = 2,
    # Connections per churn burst, and how many bursts run at once.
    [int]$ChurnConnections = 500,
    [int]$ChurnConcurrency = 32,
    # Slope ceilings. Zero records the number without judging it.
    [double]$RssCeilingMiBPerHour = 0,
    [double]$HandleCeilingPerHour = 0,
    [double]$CloseWaitCeilingPerHour = 0,
    [string]$Root = ".tmp/soak",
    [string]$ReportDir = "bench",
    [ValidateSet("debug", "release")]
    [string]$Profile = "release",
    [switch]$Keep
)

$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot

function Exit-With([int]$code, [string]$message) {
    [Console]::Error.WriteLine("soak: $message")
    exit $code
}

function Get-Timestamp {
    (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ss.fffZ")
}

function Write-Step($message) {
    Write-Host "$(Get-Timestamp) $message"
}

$script:Background = @()

function Start-Child($path, $arguments, $tag) {
    $process = Start-Process -FilePath $path -WorkingDirectory $Root -NoNewWindow -PassThru `
        -ArgumentList $arguments `
        -RedirectStandardOutput (Join-Path $Root "$tag.out") `
        -RedirectStandardError (Join-Path $Root "$tag.err")
    $script:Background += $process
    $process
}

function Stop-Background {
    foreach ($process in $script:Background) {
        if ($process -and -not $process.HasExited) {
            Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
        }
    }
    $script:Background = @()
}

trap { Stop-Background; throw }

if (-not ($IsWindows -or $env:OS -eq "Windows_NT")) {
    Exit-With 2 "the socket series reads Get-NetTCPConnection, which is Windows only. On Linux read `ss -tan` instead."
}
$exe = ".exe"
$bitCli = Join-Path $repo "target/$Profile/bit-cli$exe"
$trackerExe = Join-Path $repo "target/$Profile/examples/loopback-tracker$exe"
$churnExe = Join-Path $repo "target/$Profile/examples/loopback-churn$exe"
foreach ($required in @($bitCli, $trackerExe, $churnExe)) {
    if (-not (Test-Path $required)) {
        Exit-With 2 "missing $required. Build it first: cargo build --$Profile --workspace --bins --examples"
    }
}
if ($Minutes -lt 1) { Exit-With 2 "-Minutes has to be at least 1." }
if ($SampleSeconds -lt 1) { Exit-With 2 "-SampleSeconds has to be at least 1." }

$wantAnnounce = $Workload -in @("steady", "all", "announce")
$wantLeech = $Workload -in @("steady", "all", "leech")
$wantChurn = $Workload -in @("all", "churn")

if (-not [System.IO.Path]::IsPathRooted($Root)) { $Root = Join-Path $repo $Root }
if (Test-Path $Root) { Remove-Item -Recurse -Force $Root -ErrorAction SilentlyContinue }
New-Item -ItemType Directory -Force -Path (Join-Path $Root "payload") | Out-Null
$Root = (Resolve-Path $Root).Path
if (-not [System.IO.Path]::IsPathRooted($ReportDir)) { $ReportDir = Join-Path $repo $ReportDir }
New-Item -ItemType Directory -Force -Path $ReportDir | Out-Null

# A six-hour run holds target/release/bit-cli.exe for six hours, and Windows
# will not let cargo replace a running executable, so a soak in the background
# would block every rebuild for as long as it lasts. It runs from its own copy
# instead. The binaries are statically linked, which is what makes a lone .exe
# enough: see scripts/check-static.ps1.
$bin = Join-Path $Root "bin"
New-Item -ItemType Directory -Force -Path $bin | Out-Null
foreach ($source in @($bitCli, $trackerExe, $churnExe)) {
    Copy-Item -Path $source -Destination $bin -Force
}
$bitCli = Join-Path $bin (Split-Path -Leaf $bitCli)
$trackerExe = Join-Path $bin (Split-Path -Leaf $trackerExe)
$churnExe = Join-Path $bin (Split-Path -Leaf $churnExe)

# ---------------------------------------------------------------------------
# A payload to serve
# ---------------------------------------------------------------------------

Write-Step "building a $PayloadMiB MiB payload"
$block = [byte[]]::new(1024 * 1024)
[int64]$state = 90210
for ($i = 0; $i -lt $block.Length; $i++) {
    $state = ($state * 1103515245 + 12345) -band 0x7FFFFFFF
    $block[$i] = [byte](($state -shr 16) -band 0xFF)
}
$stream = [System.IO.File]::Create((Join-Path $Root "payload/soak.bin"))
try { for ($i = 0; $i -lt $PayloadMiB; $i++) { $stream.Write($block, 0, $block.Length) } }
finally { $stream.Dispose() }

$torrent = Join-Path $Root "soak.torrent"
$announce = $null
$trackerProcess = $null
if ($wantAnnounce) {
    $trackerProcess = Start-Child $trackerExe @("--port", "0", "--interval", "5") "tracker"
    $deadline = (Get-Date).AddSeconds(15)
    while (-not $announce -and (Get-Date) -lt $deadline) {
        $line = Get-Content (Join-Path $Root "tracker.out") -TotalCount 1 -ErrorAction SilentlyContinue
        if ($line -and $line.Trim()) { $announce = $line.Trim() }
        if (-not $announce) { Start-Sleep -Milliseconds 100 }
    }
    if (-not $announce) { Exit-With 2 "the loopback tracker never printed its URL" }
    Write-Step "tracker at $announce"
}

$createArgs = @("create", (Join-Path $Root "payload"), "--name", "payload", "--piece-length", "1MiB",
    "--no-creation-date", "--output", $torrent, "--force", "--json")
if ($announce) { $createArgs += @("--announce", $announce) }
& $bitCli @createArgs 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) { Exit-With 2 "bit-cli create exited $LASTEXITCODE" }
$infoHash = (& $bitCli info $torrent --json | ConvertFrom-Json).info_hash
if (-not $infoHash) { Exit-With 2 "could not read the info hash" }

# ---------------------------------------------------------------------------
# The subject
# ---------------------------------------------------------------------------
#
# --seed-time outlives the sampling window, so the run ends because this
# script stops it rather than because the seeder gave up partway.

$seedTime = [int]($Minutes * 60) + 300
$seedArgs = @(
    "seed", $torrent, "--data", $Root, "--port", "0",
    "--no-dht", "--no-lsd",
    "--report-interval", "$($SampleSeconds)s",
    "--seed-time", "$($seedTime)s",
    "--jsonl"
)
if (-not $wantAnnounce) { $seedArgs += "--no-tracker" }
Write-Step "starting the seeder: $Workload for $Minutes minutes, sampling every ${SampleSeconds}s"
$seed = Start-Child $bitCli $seedArgs "seed"

$port = $null
$deadline = (Get-Date).AddSeconds(60)
while (-not $port -and (Get-Date) -lt $deadline) {
    if ($seed.HasExited) { Exit-With 2 "the seeder exited before it listened; see $Root/seed.err" }
    $port = (Get-NetTCPConnection -State Listen -OwningProcess $seed.Id -ErrorAction SilentlyContinue |
            Select-Object -First 1).LocalPort
    if (-not $port) { Start-Sleep -Milliseconds 250 }
}
if (-not $port) { Exit-With 2 "the seeder never opened a listening socket" }
Write-Step "seeder listening on 127.0.0.1:$port, pid $($seed.Id)"

# ---------------------------------------------------------------------------
# Load
# ---------------------------------------------------------------------------

$leechSlots = @{}
$leechDone = 0
$leechFailed = 0
$churnRuns = 0
$churnProcess = $null

$script:LoadErrors = 0

# Start a process, and treat a failure to start as load rather than as the end
# of the run.
#
# A six hour soak that dies at hour two has measured two hours, and the reasons
# it dies are not the reasons it is running: a redirected output file that the
# previous process has not finished releasing, a directory removal racing the
# next creation, a machine briefly out of handles. Windows releases a process
# handle some time after `HasExited` goes true, so restarting a leecher into
# the same output file is exactly that race, and it fired once here at 2.2
# hours into a six hour run under a parallel `cargo build`.
#
# So: three attempts with a short wait, and then a counted failure. The count
# is in the summary, because a run with a hundred of them is measuring
# something else.
function Start-Counted($block, $what) {
    for ($attempt = 1; $attempt -le 3; $attempt++) {
        try { return & $block }
        catch {
            if ($attempt -eq 3) {
                $script:LoadErrors++
                Write-Step "  could not start $what after 3 attempts: $($_.Exception.Message)"
                return $null
            }
            Start-Sleep -Milliseconds (200 * $attempt)
        }
    }
}

function Start-Leech($slot) {
    Start-Counted {
        $out = Join-Path $Root "leech-$slot"
        if (Test-Path $out) { Remove-Item -Recurse -Force $out -ErrorAction SilentlyContinue }
        New-Item -ItemType Directory -Force -Path $out | Out-Null
        Start-Process -FilePath $bitCli -WorkingDirectory $Root -NoNewWindow -PassThru -ArgumentList @(
            "download", $torrent, "--dir", $out,
            "--peer", "127.0.0.1:$port",
            "--no-dht", "--no-lsd", "--no-tracker",
            "--allow-overwrite", "--stop-after", "120s", "--json"
        ) -RedirectStandardOutput (Join-Path $Root "leech-$slot.out") `
            -RedirectStandardError (Join-Path $Root "leech-$slot.err")
    } "leecher $slot"
}

function Start-Churn {
    Start-Counted {
        Start-Process -FilePath $churnExe -WorkingDirectory $Root -NoNewWindow -PassThru -ArgumentList @(
            "--peer", "127.0.0.1:$port",
            "--connections", "$ChurnConnections",
            "--concurrency", "$ChurnConcurrency",
            "--no-handshake"
        ) -RedirectStandardOutput (Join-Path $Root "churn.out") `
            -RedirectStandardError (Join-Path $Root "churn.err")
    } "churn"
}

# ---------------------------------------------------------------------------
# Slopes
# ---------------------------------------------------------------------------
#
# Least squares against elapsed hours, so the slope reads as "per hour" and a
# six-hour run and a twenty-minute one are comparable. R squared is reported
# beside it because a slope through noise is not a trend, and the two numbers
# together are what say whether the line is real.

function Get-Slope($rows, $column) {
    $n = $rows.Count
    if ($n -lt 2) { return $null }
    $sumX = 0.0; $sumY = 0.0; $sumXY = 0.0; $sumXX = 0.0
    foreach ($row in $rows) {
        $x = [double]$row.elapsed_s / 3600.0
        $y = [double]$row.$column
        $sumX += $x; $sumY += $y; $sumXY += ($x * $y); $sumXX += ($x * $x)
    }
    $denominator = ($n * $sumXX) - ($sumX * $sumX)
    if ([math]::Abs($denominator) -lt 1e-12) { return $null }
    $slope = (($n * $sumXY) - ($sumX * $sumY)) / $denominator
    $intercept = ($sumY - ($slope * $sumX)) / $n
    $meanY = $sumY / $n
    $ssTot = 0.0; $ssRes = 0.0
    foreach ($row in $rows) {
        $x = [double]$row.elapsed_s / 3600.0
        $y = [double]$row.$column
        $ssTot += [math]::Pow($y - $meanY, 2)
        $ssRes += [math]::Pow($y - ($intercept + ($slope * $x)), 2)
    }
    $r2 = if ($ssTot -gt 0) { 1.0 - ($ssRes / $ssTot) } else { $null }
    $values = @($rows | ForEach-Object { [double]$_.$column })
    [ordered]@{
        column         = $column
        samples        = $n
        first          = $values[0]
        last           = $values[$n - 1]
        min            = ($values | Measure-Object -Minimum).Minimum
        max            = ($values | Measure-Object -Maximum).Maximum
        mean           = [math]::Round(($values | Measure-Object -Average).Average, 2)
        slope_per_hour = [math]::Round($slope, 3)
        r_squared      = if ($null -eq $r2) { $null } else { [math]::Round($r2, 4) }
    }
}

# What the seeder says it cost, so the sampler can be checked against the
# subject. A sampler that disagrees with the process is measuring something
# else.
#
# This reads forward from where the last call stopped rather than re-reading
# the whole file, because a six hour run writes 720 progress events and
# re-parsing every one of them on every sample is work charged to the machine
# under measurement. A chunk can end mid-line, so the tail is held back until
# its newline arrives.

$script:SelfStream = $null
$script:SelfReader = $null
$script:SelfPending = ""
$script:SelfPeakRss = $null
$script:SelfHandles = $null
$script:SelfEvents = 0

function Update-SelfReported {
    try {
        if (-not $script:SelfReader) {
            $selfPath = Join-Path $Root "seed.out"
            if (-not (Test-Path $selfPath)) { return }
            $script:SelfStream = [System.IO.File]::Open(
                $selfPath, [System.IO.FileMode]::Open, [System.IO.FileAccess]::Read,
                [System.IO.FileShare]::ReadWrite)
            $script:SelfReader = [System.IO.StreamReader]::new($script:SelfStream)
        }
        $chunk = $script:SelfReader.ReadToEnd()
        if (-not $chunk) { return }
        $lines = ($script:SelfPending + $chunk) -split "`n"
        $script:SelfPending = $lines[-1]
        for ($index = 0; $index -lt $lines.Count - 1; $index++) {
            $text = $lines[$index].Trim()
            if (-not $text) { continue }
            $reported = $null
            try { $reported = $text | ConvertFrom-Json } catch { continue }
            if ($reported.type -ne "progress" -or -not $reported.process) { continue }
            $script:SelfEvents++
            if ($null -eq $script:SelfPeakRss -or $reported.process.peak_rss_bytes -gt $script:SelfPeakRss) {
                $script:SelfPeakRss = $reported.process.peak_rss_bytes
            }
            if ($null -eq $script:SelfHandles -or $reported.process.open_handles -gt $script:SelfHandles) {
                $script:SelfHandles = $reported.process.open_handles
            }
        }
    } catch { }
}

# The reader holds seed.out open, and -Keep off deletes the root at the end.
# Windows will not delete a file another handle is on, so this is called
# before the cleanup rather than left to the process exiting.
function Close-SelfReported {
    if ($script:SelfReader) { $script:SelfReader.Dispose(); $script:SelfReader = $null }
    if ($script:SelfStream) { $script:SelfStream.Dispose(); $script:SelfStream = $null }
}

# The summary is written after every sample, not only when the window ends, so
# a run killed at hour four leaves a report of four hours rather than a CSV
# somebody has to fit a line through by hand. `complete` is what says which of
# the two a reader is holding; nothing else about the shape changes, and the
# last write of a run that finished is the object this file always carried.
#
# Returns the slopes, the failures, and the verdict, so the caller prints what
# was written rather than computing it a second time.
function Write-SoakSummary([bool]$Complete) {
    $summaryRows = @($samples)
    $summarySlopes = [ordered]@{}
    foreach ($column in @("rss_bytes", "peak_rss_bytes", "handles", "threads",
            "tcp_total", "tcp_close_wait", "tcp_established")) {
        $summarySlopes[$column] = Get-Slope $summaryRows $column
    }

    $summaryHours = $clock.Elapsed.TotalHours
    $summaryFailures = [System.Collections.ArrayList]::new()
    if ($seedDied) { [void]$summaryFailures.Add("the seeder exited before the sampling window ended; see $Root/seed.err") }

    $summaryRss = if ($summarySlopes["rss_bytes"]) { [math]::Round($summarySlopes["rss_bytes"].slope_per_hour / 1MB, 3) } else { $null }
    if ($RssCeilingMiBPerHour -gt 0 -and $null -ne $summaryRss -and $summaryRss -gt $RssCeilingMiBPerHour) {
        [void]$summaryFailures.Add("resident memory grew $summaryRss MiB/hour, over the ceiling of $RssCeilingMiBPerHour")
    }
    if ($HandleCeilingPerHour -gt 0 -and $summarySlopes["handles"] -and
        $summarySlopes["handles"].slope_per_hour -gt $HandleCeilingPerHour) {
        [void]$summaryFailures.Add("handles grew $($summarySlopes["handles"].slope_per_hour)/hour, over the ceiling of $HandleCeilingPerHour")
    }
    if ($CloseWaitCeilingPerHour -gt 0 -and $summarySlopes["tcp_close_wait"] -and
        $summarySlopes["tcp_close_wait"].slope_per_hour -gt $CloseWaitCeilingPerHour) {
        [void]$summaryFailures.Add("CLOSE_WAIT grew $($summarySlopes["tcp_close_wait"].slope_per_hour)/hour, over the ceiling of $CloseWaitCeilingPerHour")
    }

    $summaryJudged = ($RssCeilingMiBPerHour -gt 0) -or ($HandleCeilingPerHour -gt 0) -or ($CloseWaitCeilingPerHour -gt 0)
    $summaryVerdict = switch ($true) {
        ($summaryFailures.Count -gt 0) { "$($summaryFailures.Count) ceiling(s) or the run itself did not hold"; break }
        (-not $Complete) { "in flight: $($summaryRows.Count) samples over $([math]::Round($summaryHours, 2)) of the $([math]::Round($Minutes / 60.0, 2)) hours asked for"; break }
        ($summaryJudged) { "every named ceiling held over $([math]::Round($summaryHours, 2)) hours"; break }
        default { "recorded, not judged: no ceiling was named"; break }
    }

    [ordered]@{
        kind             = "soak"
        schema_version   = "1"
        generated_at     = Get-Timestamp
        complete         = $Complete
        host             = [ordered]@{
            machine = [System.Environment]::MachineName
            os      = [System.Environment]::OSVersion.VersionString
            cpus    = [System.Environment]::ProcessorCount
        }
        parameters       = [ordered]@{
            minutes           = $Minutes
            sample_seconds    = $SampleSeconds
            workload          = $Workload
            payload_mib       = $PayloadMiB
            leechers          = $Leechers
            churn_connections = $ChurnConnections
            churn_concurrency = $ChurnConcurrency
            profile           = $Profile
            ceilings          = [ordered]@{
                rss_mib_per_hour    = $RssCeilingMiBPerHour
                handles_per_hour    = $HandleCeilingPerHour
                close_wait_per_hour = $CloseWaitCeilingPerHour
            }
        }
        info_hash        = $infoHash
        csv              = $csvPath
        elapsed_hours    = [math]::Round($summaryHours, 4)
        samples          = $summaryRows.Count
        cycles           = [ordered]@{
            leech_completed         = $leechDone
            leech_failed            = $leechFailed
            churn_runs              = $churnRuns
            churn_connections_total = $churnRuns * $ChurnConnections
            progress_events         = $script:SelfEvents
            # Samples or process starts that failed and were carried past
            # rather than ending the run.
            load_errors             = $script:LoadErrors
        }
        slopes           = $summarySlopes
        rss_mib_per_hour = $summaryRss
        self_reported    = [ordered]@{
            peak_rss_bytes = $script:SelfPeakRss
            open_handles   = $script:SelfHandles
        }
        seed_exited_early = $seedDied
        verdict          = $summaryVerdict
        failures         = @($summaryFailures)
        commands         = @(
            "$bitCli $($seedArgs -join ' ')",
            $(if ($wantChurn) { "$churnExe --peer 127.0.0.1:$port --connections $ChurnConnections --concurrency $ChurnConcurrency --no-handshake" } else { $null }),
            $(if ($wantLeech) { "$bitCli download $torrent --dir leech-N --peer 127.0.0.1:$port --no-dht --no-lsd --no-tracker --allow-overwrite --stop-after 120s --json" } else { $null }),
            $(if ($wantAnnounce) { "$trackerExe --port 0 --interval 5" } else { $null })
        ) | Where-Object { $_ }
        notes            = @(
            "The subject is the seeder. rss_bytes and handles are read from outside with Get-Process, and the seeder's own progress events carry the same two figures, so self_reported is the cross-check rather than a second measurement.",
            "slope_per_hour is least squares against elapsed hours. r_squared beside it says whether the line is a trend or noise: a large slope with a low r squared is a spike, not growth.",
            "peak_rss_bytes is a high-water mark rather than a level, so its slope is bounded below by zero and says nothing on its own. rss_bytes is the series that can fall as well as rise, and it is the one a leak shows in.",
            "The loopback tracker never expires a peer, so under -Workload announce or all the peer list handed to the seeder grows for the whole run. That is deliberate: it is the shape a busy tracker has, and it is the path T-040's report points at.",
            "complete is false while the run is still sampling. This file is rewritten after every sample, so a run that is killed leaves the report it had reached rather than nothing at all."
        )
    } | ConvertTo-Json -Depth 8 | Set-Content -Path $jsonPath -Encoding utf8NoBOM

    [ordered]@{
        slopes           = $summarySlopes
        failures         = $summaryFailures
        verdict          = $summaryVerdict
        hours            = $summaryHours
        rss_mib_per_hour = $summaryRss
    }
}

# ---------------------------------------------------------------------------
# Sampling
# ---------------------------------------------------------------------------

$stamp = (Get-Date).ToUniversalTime().ToString("yyyyMMddTHHmmssfffZ")
$csvPath = Join-Path $ReportDir "soak-$stamp.csv"
$jsonPath = Join-Path $ReportDir "soak-$stamp.json"
$header = "sample,iso,elapsed_s,rss_bytes,peak_rss_bytes,handles,threads,cpu_ms," +
"tcp_total,tcp_established,tcp_listen,tcp_close_wait,tcp_time_wait,tcp_other," +
"leech_completed,leech_failed,churn_runs"
Set-Content -Path $csvPath -Value $header -Encoding utf8NoBOM

$samples = [System.Collections.ArrayList]::new()
$clock = [System.Diagnostics.Stopwatch]::StartNew()
$endAt = (Get-Date).AddMinutes($Minutes)
$sample = 0
$seedDied = $false

while ((Get-Date) -lt $endAt) {
    if ($seed.HasExited) { $seedDied = $true; break }

    # One sample is not the run. A transient failure here, a file still held
    # by a process that has exited or a directory removal racing its own
    # recreation, used to end a six hour soak at hour two: everything in this
    # loop ran under `$ErrorActionPreference = 'Stop'` with a trap above it.
    # It is counted and the loop carries on, and the count is in the summary,
    # because a run with a hundred of them is measuring something else.
    try {
        # Top up the load before sampling, so a sample never lands in the gap
        # between one leecher exiting and the next starting.
        if ($wantLeech) {
            for ($slot = 0; $slot -lt $Leechers; $slot++) {
                $running = $leechSlots[$slot]
                if ($running -and $running.HasExited) {
                    if ($running.ExitCode -eq 0) { $leechDone++ } else { $leechFailed++ }
                    $leechSlots[$slot] = $null
                    $running = $null
                }
                if (-not $running) { $leechSlots[$slot] = Start-Leech $slot }
            }
        }
        if ($wantChurn -and (-not $churnProcess -or $churnProcess.HasExited)) {
            if ($churnProcess) { $churnRuns++ }
            $churnProcess = Start-Churn
        }
    
        $seed.Refresh()
        $states = @{}
        foreach ($group in (Get-NetTCPConnection -OwningProcess $seed.Id -ErrorAction SilentlyContinue |
                    Group-Object State)) {
            $states[$group.Name] = $group.Count
        }
        $total = 0
        foreach ($count in $states.Values) { $total += $count }
        $named = 0
        foreach ($key in @("Established", "Listen", "CloseWait", "TimeWait")) {
            if ($states.ContainsKey($key)) { $named += $states[$key] }
        }
    
        $row = [ordered]@{
            sample           = $sample
            iso              = Get-Timestamp
            elapsed_s        = [int]($clock.Elapsed.TotalSeconds)
            rss_bytes        = $seed.WorkingSet64
            peak_rss_bytes   = $seed.PeakWorkingSet64
            handles          = $seed.HandleCount
            threads          = $seed.Threads.Count
            cpu_ms           = [int64]$seed.TotalProcessorTime.TotalMilliseconds
            tcp_total        = $total
            tcp_established  = if ($states.ContainsKey("Established")) { $states["Established"] } else { 0 }
            tcp_listen       = if ($states.ContainsKey("Listen")) { $states["Listen"] } else { 0 }
            tcp_close_wait   = if ($states.ContainsKey("CloseWait")) { $states["CloseWait"] } else { 0 }
            tcp_time_wait    = if ($states.ContainsKey("TimeWait")) { $states["TimeWait"] } else { 0 }
            tcp_other        = $total - $named
            leech_completed  = $leechDone
            leech_failed     = $leechFailed
            churn_runs       = $churnRuns
        }
        [void]$samples.Add($row)
        Add-Content -Path $csvPath -Encoding utf8NoBOM -Value (($row.Values | ForEach-Object { "$_" }) -join ",")
    
        # Rewrite the report now rather than only when the window ends, so a run
        # that is killed at hour four leaves four hours of slopes. See
        # Write-SoakSummary.
        Update-SelfReported
        [void](Write-SoakSummary $false)
    
        if ($sample % 10 -eq 0) {
            Write-Step ("  t+{0,6}s  rss {1,7:N1} MiB  handles {2,5}  sockets {3,5}  CW {4,5}  leech {5}" -f `
                    $row.elapsed_s, ($row.rss_bytes / 1MB), $row.handles, $row.tcp_total, $row.tcp_close_wait, $leechDone)
        }
        $sample++
    }
    catch {
        $script:LoadErrors++
        Write-Step "  sample $sample failed: $($_.Exception.Message)"
        $sample++
    }

    $nextAt = $endAt
    $due = (Get-Date).AddSeconds($SampleSeconds)
    if ($due -lt $nextAt) { $nextAt = $due }
    $wait = ($nextAt - (Get-Date)).TotalMilliseconds
    if ($wait -gt 0) { Start-Sleep -Milliseconds ([int]$wait) }
}

$clock.Stop()
Write-Step "sampling finished after $([int]$clock.Elapsed.TotalMinutes) minutes, $($samples.Count) samples"

if (-not $seed.HasExited) { Stop-Process -Id $seed.Id -Force -ErrorAction SilentlyContinue }
Start-Sleep -Milliseconds 500
Update-SelfReported
Close-SelfReported
Stop-Background

$summary = Write-SoakSummary $true
$slopes = $summary.slopes
$failures = $summary.failures
$hours = $summary.hours

Write-Host ""
Write-Host "workload:  $Workload for $([math]::Round($hours, 2)) hours, $($samples.Count) samples"
Write-Host "csv:       $csvPath"
Write-Host "report:    $jsonPath"
Write-Host ""
@($slopes.Keys) | ForEach-Object {
    $entry = $slopes[$_]
    if (-not $entry) { return }
    [pscustomobject][ordered]@{
        series     = $_
        first      = $entry.first
        last       = $entry.last
        max        = $entry.max
        "per hour" = $entry.slope_per_hour
        "r2"       = $entry.r_squared
    }
} | Format-Table -AutoSize | Out-String | Write-Host
Write-Host "leech cycles: $leechDone completed, $leechFailed failed. churn bursts: $churnRuns."
if ($script:LoadErrors -gt 0) {
    Write-Host "load errors carried past: $script:LoadErrors"
}
if ($null -ne $script:SelfPeakRss) {
    Write-Host "self reported: peak RSS $([math]::Round($script:SelfPeakRss / 1MB, 2)) MiB, $script:SelfHandles handles, over $($script:SelfEvents) progress events"
}
Write-Host "verdict: $($summary.verdict)"

if (-not $Keep) { Remove-Item -Recurse -Force $Root -ErrorAction SilentlyContinue }

if ($failures.Count -gt 0) {
    Write-Host ""
    foreach ($failure in $failures) { [Console]::Error.WriteLine("soak: $failure") }
    exit 1
}
exit 0
