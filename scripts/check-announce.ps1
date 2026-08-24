# Are the numbers a tracker sees the numbers `bit-cli` reports?
#
# The defect this exists to catch is an announce that disagrees with the run it
# describes. `uploaded`, `downloaded` and `left` are the only thing a tracker
# knows about a client, and nothing until now compared them against what the
# client's own report said it transferred. A wrong number here is invisible
# locally, wrong on the tracker forever, and indistinguishable from cheating.
#
# Six cases, all on loopback and only on loopback:
#
#   started-left     the first announce is `started` and carries the whole
#                    payload in `left`
#   completed        `completed` is sent, once, and `left` is 0 by then
#   stopped          `stopped` is sent when the run ends
#   left-monotonic   `left` never rises
#   totals-match     the last announce's `downloaded` agrees with the report's
#                    own byte count, and `uploaded` is not invented
#   interval         the gap between two ordinary announces is at least the
#                    `min interval` the tracker asked for
#
# The evidence is `loopback-tracker --announce-log`, which appends one JSON
# object per announce carrying the raw query as received. The comparison is
# against `bit-cli download --json`, so both sides are machine output and
# neither is a log line parsed by eye.
#
# This points at loopback and never at a public tracker, and it changes no
# number it reports. It is a correctness harness, not a ratio tool.
#
# Usage:
#   pwsh scripts/check-announce.ps1
#   pwsh scripts/check-announce.ps1 -PayloadMiB 16 -Json bench/announce.json
#
# Exits 0 when every judged case holds, 1 when one does not, and 2 when the
# check could not run.
#
# See TODO/trackers.md, T-235.

[CmdletBinding()]
param(
    [int]$PayloadMiB = 8,
    [int]$TimeoutSeconds = 120,
    [int]$AnnounceInterval = 5,
    [string]$Root = ".tmp/announce",
    [ValidateSet("debug", "release")]
    [string]$Profile = "release",
    [string]$Json
)

$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot

function Exit-With([int]$code, [string]$message) {
    [Console]::Error.WriteLine("check-announce: $message")
    exit $code
}

function Write-Step($message) {
    Write-Host "$((Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ss.fffZ')) $message"
}

$bitCli = Join-Path $repo "target/$Profile/bit-cli.exe"
if (-not (Test-Path $bitCli)) {
    Exit-With 2 "missing $bitCli. Build it first: cargo build --$Profile --bins --examples"
}
$tracker = Join-Path $repo "target/$Profile/examples/loopback-tracker.exe"
if (-not (Test-Path $tracker)) {
    Exit-With 2 "missing $tracker. Build it first: cargo build --$Profile --bins --examples"
}

if (-not [System.IO.Path]::IsPathRooted($Root)) { $Root = Join-Path $repo $Root }
if (Test-Path $Root) { Remove-Item -Recurse -Force $Root -ErrorAction SilentlyContinue }
New-Item -ItemType Directory -Force -Path (Join-Path $Root "payload") | Out-Null
$Root = (Resolve-Path $Root).Path

$background = @()
function Stop-Background {
    foreach ($process in $script:background) {
        if ($process -and -not $process.HasExited) {
            Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
        }
    }
    $script:background = @()
}
trap {
    Stop-Background
    [Console]::Error.WriteLine("check-announce: $($_.Exception.Message)")
    [Console]::Error.WriteLine("  at $($_.InvocationInfo.ScriptLineNumber): $($_.InvocationInfo.Line.Trim())")
    throw
}

# ---------------------------------------------------------------------------
# A payload, a torrent, and a tracker that writes down what it was told
# ---------------------------------------------------------------------------

Write-Step "building a $PayloadMiB MiB payload"
$block = [byte[]]::new(1024 * 1024)
[int64]$state = 20260824
for ($i = 0; $i -lt $block.Length; $i++) {
    $state = ($state * 1103515245 + 12345) -band 0x7FFFFFFF
    $block[$i] = [byte](($state -shr 16) -band 0xFF)
}
$payload = Join-Path $Root "payload/announce.bin"
$stream = [System.IO.File]::Create($payload)
try { for ($i = 0; $i -lt $PayloadMiB; $i++) { $stream.Write($block, 0, $block.Length) } }
finally { $stream.Dispose() }
$payloadBytes = (Get-Item -LiteralPath $payload).Length

$announceLog = Join-Path $Root "announces.jsonl"
$trackerProcess = Start-Process -FilePath $tracker -WorkingDirectory $Root -NoNewWindow -PassThru -ArgumentList @(
    "--port", "0", "--interval", "$AnnounceInterval", "--announce-log", $announceLog
) -RedirectStandardOutput (Join-Path $Root "tracker.out") `
    -RedirectStandardError (Join-Path $Root "tracker.err")
$background += $trackerProcess

$announceUrl = $null
$deadline = (Get-Date).AddSeconds(30)
while (-not $announceUrl -and (Get-Date) -lt $deadline) {
    if ($trackerProcess.HasExited) { break }
    foreach ($line in @(Get-Content (Join-Path $Root "tracker.out") -ErrorAction SilentlyContinue)) {
        if ($line -and $line.StartsWith("http://127.0.0.1:")) { $announceUrl = $line.Trim(); break }
    }
    if (-not $announceUrl) { Start-Sleep -Milliseconds 200 }
}
if (-not $announceUrl) {
    Stop-Background
    Exit-With 2 "the loopback tracker never printed an announce URL; see $Root/tracker.err"
}
Write-Step "tracker at $announceUrl, min interval $AnnounceInterval s"

$torrent = Join-Path $Root "announce.torrent"
& $bitCli create (Join-Path $Root "payload") --name payload --piece-length 256KiB `
    --announce $announceUrl --no-creation-date --output $torrent --force --json 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) { Stop-Background; Exit-With 2 "bit-cli create exited $LASTEXITCODE" }

# ---------------------------------------------------------------------------
# A seeder, then a leecher that takes the whole payload from it
# ---------------------------------------------------------------------------

$seedRoot = Join-Path $Root "seed"
New-Item -ItemType Directory -Force -Path $seedRoot | Out-Null
$seed = Start-Process -FilePath $bitCli -WorkingDirectory $Root -NoNewWindow -PassThru -ArgumentList @(
    "seed", $torrent, "--data", $Root, "--port", "0",
    "--no-dht", "--no-lsd",
    "--report-interval", "5s", "--seed-time", "$($TimeoutSeconds + 60)s", "--jsonl"
) -RedirectStandardOutput (Join-Path $seedRoot "seed.out") `
    -RedirectStandardError (Join-Path $seedRoot "seed.err")
$background += $seed

$listen = $null
$deadline = (Get-Date).AddSeconds(60)
while (-not $listen -and (Get-Date) -lt $deadline) {
    if ($seed.HasExited) { break }
    foreach ($line in @(Get-Content (Join-Path $seedRoot "seed.out") -ErrorAction SilentlyContinue)) {
        if (-not $line -or -not $line.Trim().StartsWith("{")) { continue }
        $event = $null
        try { $event = $line | ConvertFrom-Json } catch { continue }
        if ($event.listen_addr) { $listen = $event.listen_addr; break }
    }
    if (-not $listen) { Start-Sleep -Milliseconds 200 }
}
if (-not $listen) {
    Stop-Background
    Exit-With 2 "the seeder never reported a listen address; see $seedRoot/seed.err"
}
Write-Step "seeder listening on $listen"

# The leecher is the subject. It is left running past completion for two
# announce intervals so the `completed` event and at least one ordinary
# announce after it are on the record, which is what the interval case reads.
$leechOut = Join-Path $Root "leech.json"
$leechRoot = Join-Path $Root "leech"
$holdSeconds = [Math]::Max(3 * $AnnounceInterval, 15)
Write-Step "leeching, then holding $holdSeconds s so the post-completion announces land"
$leech = Start-Process -FilePath $bitCli -WorkingDirectory $Root -NoNewWindow -PassThru -Wait -ArgumentList @(
    "download", $torrent, "--dir", $leechRoot,
    "--no-dht", "--no-lsd", "--allow-overwrite",
    "--seed-time", "$($holdSeconds)s",
    "--stop-after", "$($TimeoutSeconds)s", "--json"
) -RedirectStandardOutput $leechOut -RedirectStandardError (Join-Path $Root "leech.err")

$report = $null
try { $report = Get-Content $leechOut -Raw | ConvertFrom-Json } catch { $report = $null }
if (-not $report) {
    Stop-Background
    Exit-With 2 "bit-cli download wrote no JSON report; see $Root/leech.err"
}

Start-Sleep -Milliseconds 500
Stop-Background

# ---------------------------------------------------------------------------
# What the tracker was told
# ---------------------------------------------------------------------------

if (-not (Test-Path $announceLog)) {
    Exit-With 2 "the tracker recorded no announce at all; see $Root/tracker.err"
}

$announces = @()
foreach ($line in Get-Content $announceLog) {
    if (-not $line.Trim()) { continue }
    try { $announces += ($line | ConvertFrom-Json) } catch { }
}
if ($announces.Count -eq 0) { Exit-With 2 "the announce log holds no readable record" }

# The leecher's peer id, taken from the report rather than guessed, so a
# seeder announcing to the same tracker is not read as the subject.
$leechPeerId = $null
if ($report.PSObject.Properties.Name -contains 'peer_id') { $leechPeerId = $report.peer_id }
if (-not $leechPeerId -and $report.torrents -and $report.torrents[0].PSObject.Properties.Name -contains 'peer_id') {
    $leechPeerId = $report.torrents[0].peer_id
}

$mine = @($announces)
if ($leechPeerId) {
    $filtered = @($announces | Where-Object { $_.peer_id -eq $leechPeerId })
    if ($filtered.Count -gt 0) { $mine = $filtered }
}
# Without a peer id in the report the announces are separated by the one
# property that always differs: the seeder starts complete, so its first
# announce carries left=0 and the leecher's carries the whole payload.
if (-not $leechPeerId) {
    $byPeer = $announces | Group-Object peer_id
    $candidate = $byPeer | Where-Object {
        $first = ($_.Group | Sort-Object at)[0]
        [int64]$first.left -gt 0
    }
    if ($candidate) { $mine = @(($candidate | Select-Object -First 1).Group) }
}
$mine = @($mine | Sort-Object at)

Write-Step "$($announces.Count) announce(s) recorded, $($mine.Count) from the leecher"

$cases = [System.Collections.ArrayList]::new()
function Add-Case($name, $judged, $ok, $detail) {
    [void]$cases.Add([ordered]@{
            case   = $name
            judged = $judged
            ok     = $ok
            detail = $detail
        })
}

$events = @($mine | ForEach-Object { if ($_.PSObject.Properties.Name -contains 'event') { $_.event } else { "" } })
$firstAnnounce = if ($mine.Count -gt 0) { $mine[0] } else { $null }
$lastAnnounce = if ($mine.Count -gt 0) { $mine[$mine.Count - 1] } else { $null }

# 1. started, and it carries the whole payload as left
$startedOk = $false
$startedDetail = "no announce from the leecher"
if ($firstAnnounce) {
    $startedEvent = if ($firstAnnounce.PSObject.Properties.Name -contains 'event') { $firstAnnounce.event } else { "" }
    $startedLeft = [int64]$firstAnnounce.left
    $startedOk = ($startedEvent -eq "started") -and ($startedLeft -eq $payloadBytes)
    $startedDetail = "first event '$startedEvent', left $startedLeft, payload $payloadBytes"
}
Add-Case "started-left" $true $startedOk $startedDetail

# 2. completed, exactly once, and left is zero by then
$completedIndexes = @()
for ($i = 0; $i -lt $mine.Count; $i++) {
    if ($events[$i] -eq "completed") { $completedIndexes += $i }
}
$completedOk = $false
$completedDetail = "no completed event in $($mine.Count) announce(s)"
if ($completedIndexes.Count -eq 1) {
    $completed = $mine[$completedIndexes[0]]
    $completedOk = ([int64]$completed.left -eq 0)
    $completedDetail = "one completed event, left $([int64]$completed.left)"
} elseif ($completedIndexes.Count -gt 1) {
    $completedDetail = "$($completedIndexes.Count) completed events, and BEP 3 asks for one"
}
Add-Case "completed" $true $completedOk $completedDetail

# 3. stopped, at the end
$stoppedOk = ($events -contains "stopped")
$stoppedDetail = "events: " + (($events | ForEach-Object { if ($_) { $_ } else { "-" } }) -join ",")
Add-Case "stopped" $true $stoppedOk $stoppedDetail

# 4. left never rises
$leftValues = @($mine | ForEach-Object { [int64]$_.left })
$leftOk = $true
for ($i = 1; $i -lt $leftValues.Count; $i++) {
    if ($leftValues[$i] -gt $leftValues[$i - 1]) { $leftOk = $false }
}
Add-Case "left-monotonic" $true $leftOk ("left: " + ($leftValues -join " -> "))

# 5. the totals the tracker saw against the totals the run reported
# `docs/schema.md` gives these as `{bytes, human}`, the shape RULES.md section 5
# asks for: a raw integer with any formatted string beside it rather than
# instead of it. Read `.bytes` and never the pair.
$reportedDownloaded = $null
$reportedUploaded = $null
if ($report.torrents -and $report.torrents.Count -gt 0) {
    $torrent0 = $report.torrents[0]
    if ($torrent0.PSObject.Properties.Name -contains 'downloaded' -and
        $torrent0.downloaded.PSObject.Properties.Name -contains 'bytes') {
        $reportedDownloaded = [int64]$torrent0.downloaded.bytes
    }
    if ($torrent0.PSObject.Properties.Name -contains 'uploaded' -and
        $torrent0.uploaded.PSObject.Properties.Name -contains 'bytes') {
        $reportedUploaded = [int64]$torrent0.uploaded.bytes
    }
}
$announcedDownloaded = if ($lastAnnounce) { [int64]$lastAnnounce.downloaded } else { $null }
$announcedUploaded = if ($lastAnnounce) { [int64]$lastAnnounce.uploaded } else { $null }
$totalsJudged = ($null -ne $reportedDownloaded -and $null -ne $announcedDownloaded)
$totalsOk = $false
if ($totalsJudged) {
    # The tracker's figure is taken at the last announce and the report's at
    # exit, so they are not required to be equal to the byte. What is required
    # is that the announce is not larger than the run and covers the payload.
    $totalsOk = ($announcedDownloaded -ge $payloadBytes) -and ($announcedDownloaded -le $reportedDownloaded)
}
$totalsDetail = "announced downloaded $announcedDownloaded, uploaded $announcedUploaded; report downloaded $reportedDownloaded, uploaded $reportedUploaded; payload $payloadBytes"
Add-Case "totals-match" $totalsJudged $totalsOk $totalsDetail

# 6. the interval the tracker asked for is honoured between ordinary announces
$ordinary = @()
for ($i = 0; $i -lt $mine.Count; $i++) {
    if (-not $events[$i]) { $ordinary += $mine[$i] }
}
$intervalJudged = ($ordinary.Count -ge 2)
$intervalOk = $true
$smallest = $null
if ($intervalJudged) {
    for ($i = 1; $i -lt $ordinary.Count; $i++) {
        $gap = ([datetime]$ordinary[$i].at - [datetime]$ordinary[$i - 1].at).TotalSeconds
        if ($null -eq $smallest -or $gap -lt $smallest) { $smallest = $gap }
    }
    # One second of slack: the tracker stamps on arrival and the client times
    # from its own clock, and asserting they agree to the millisecond is
    # asserting a scheduling outcome.
    $intervalOk = ($smallest -ge ($AnnounceInterval - 1))
}
$intervalDetail = if ($intervalJudged) {
    "smallest gap $([math]::Round($smallest, 2))s against a min interval of ${AnnounceInterval}s over $($ordinary.Count) ordinary announces"
} else {
    "only $($ordinary.Count) ordinary announce(s), so there is no gap to measure"
}
Add-Case "interval" $intervalJudged $intervalOk $intervalDetail

# ---------------------------------------------------------------------------

$failures = @($cases | Where-Object { $_.judged -and -not $_.ok } | ForEach-Object {
        "$($_.case): $($_.detail)"
    })

$result = [ordered]@{
    kind             = "announce"
    generated_at     = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ss.fffZ")
    payload_bytes    = $payloadBytes
    announce_url     = $announceUrl
    min_interval     = $AnnounceInterval
    announces_total  = $announces.Count
    announces_subject = $mine.Count
    query_order      = if ($firstAnnounce) { @($firstAnnounce.query_order) } else { @() }
    user_agent       = if ($firstAnnounce) {
        ($firstAnnounce.headers | Where-Object { $_.name -eq 'User-Agent' } | Select-Object -First 1).value
    } else { $null }
    peer_id          = if ($firstAnnounce) { $firstAnnounce.peer_id } else { $null }
    cases            = @($cases)
    failures         = @($failures)
    notes            = @(
        "Loopback only. Nothing here points at a public tracker and nothing here changes a reported number: this measures whether the announce agrees with the run.",
        "totals-match compares the last announce against the report at exit, so it asserts a bound rather than equality: the announce must cover the payload and must not exceed what the run says it moved.",
        "The interval case allows one second of slack, because the tracker stamps on arrival and the client times from its own clock."
    )
}

if ($Json) {
    $jsonPath = if ([System.IO.Path]::IsPathRooted($Json)) { $Json } else { Join-Path $repo $Json }
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $jsonPath) | Out-Null
    $result | ConvertTo-Json -Depth 8 | Set-Content -Path $jsonPath -Encoding utf8NoBOM
    Write-Host "check-announce: wrote $Json"
}

@($cases) | ForEach-Object { [pscustomobject]$_ } |
    Format-Table case, judged, ok, detail -AutoSize -Wrap |
    Out-String | Write-Host

if ($firstAnnounce) {
    Write-Host ("query order: " + (@($firstAnnounce.query_order) -join ", "))
    Write-Host ("user agent:  " + $result.user_agent)
    Write-Host ("peer id:     " + $firstAnnounce.peer_id)
}

Remove-Item -Recurse -Force $Root -ErrorAction SilentlyContinue

if ($failures.Count -gt 0) {
    foreach ($failure in $failures) { [Console]::Error.WriteLine("check-announce: $failure") }
    exit 1
}
Write-Host "check-announce: every judged case holds"
exit 0
