# What has happened upstream since the commit we vendored, and which of it
# matters here.
#
# Run this on every version bump and before every reconciliation. A vendored
# dependency stops being visible: nobody sees a release note, nobody sees the
# issue that names the bug we worked around, and the fork drifts from upstream
# with no record of what it drifted past. This is that record.
#
# For each upstream in vendor/upstream.json it asks GitHub for the releases,
# the commits, the pull requests and the issues that landed after our base
# commit, scores each against what `TODO/` says this repository is blocked on,
# and writes both a JSON record and a summary a person reads.
#
# It never writes to TODO/. What it produces is a candidate list, and turning a
# candidate into an entry is a judgement: see patches/TASKS.md.
#
# Usage:
#   pwsh scripts/upstream-scan.ps1
#   pwsh scripts/upstream-scan.ps1 -Upstream rqbit -Since 2026-01-01
#   pwsh scripts/upstream-scan.ps1 -FlaggedOnly
#
# Exits 0 when the scan completed, 2 when it could not run. A scan that finds
# things is not a failure: it is the normal case and the whole point.

[CmdletBinding()]
param(
    [string]$Upstream = "all",
    [string]$Since,
    [switch]$FlaggedOnly,
    [int]$MaxPages = 10,
    [string]$Manifest = "vendor/upstream.json",
    [string]$Out = "patches/scan"
)

$ErrorActionPreference = "Stop"
$repo = Split-Path -Parent $PSScriptRoot
Set-Location $repo

function Say([string]$text) {
    $at = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ss.fffZ")
    Write-Host "$at upstream-scan: $text"
}
function Exit-With([int]$code, [string]$text) { Say $text; exit $code }

if (-not (Get-Command gh -ErrorAction SilentlyContinue)) {
    Exit-With 2 "gh is not on PATH. This scan is entirely GitHub API calls."
}
if (-not (Test-Path $Manifest)) { Exit-With 2 "$Manifest is not there" }

# One `gh api` call, with backoff, because this makes hundreds of them.
#
# Three failures are worth retrying and no others. A 403 carrying "rate limit"
# is the secondary limit, which is not the hourly quota and clears in tens of
# seconds. A 429 is the same thing said properly. A 5xx is GitHub. Anything
# else, a 404 or a bad query, is a mistake here and retrying it just makes the
# same mistake more slowly.
#
# The wait doubles from two seconds and is capped, so a scan that hits the
# limit finishes late rather than hammering through it. `gh` itself does not
# back off: it returns the error and exits.
function Invoke-GhApi([string]$Path, [int]$Attempts = 6) {
    $wait = 2
    for ($try = 1; $try -le $Attempts; $try++) {
        $text = & gh api $Path 2>&1
        if ($LASTEXITCODE -eq 0) {
            try { return ($text | ConvertFrom-Json) }
            catch { Exit-With 2 "gh api $Path returned something that is not JSON" }
        }
        $message = ($text | Out-String)
        $retryable = $message -match "rate limit" -or $message -match "429" -or
                     $message -match "was submitted too quickly" -or $message -match "HTTP 5\d\d"
        if (-not $retryable) {
            Say "  gh api $Path failed and is not worth retrying: $($message.Trim())"
            return $null
        }
        if ($try -eq $Attempts) {
            Say "  gh api $Path still rate limited after $Attempts attempts, giving up on this call"
            return $null
        }
        Say "  rate limited, waiting $wait s (attempt $try of $Attempts)"
        Start-Sleep -Seconds $wait
        $wait = [math]::Min($wait * 2, 60)
    }
    $null
}

# A paged endpoint, to a bounded number of pages.
#
# Bounded on purpose. An upstream with ten years of closed issues is not a
# reason to make a thousand calls: what this scan is for is what changed since
# the base, and -Since narrows it further.
function Invoke-GhPaged([string]$Path, [int]$Pages) {
    $all = [System.Collections.ArrayList]::new()
    for ($page = 1; $page -le $Pages; $page++) {
        $joiner = if ($Path.Contains("?")) { "&" } else { "?" }
        $batch = Invoke-GhApi "$Path${joiner}per_page=100&page=$page"
        if ($null -eq $batch) { break }
        $items = @($batch)
        if ($items.Count -eq 0) { break }
        foreach ($item in $items) { [void]$all.Add($item) }
        if ($items.Count -lt 100) { break }
    }
    $all
}

# What this repository is blocked on, in words that appear in an upstream title.
#
# Each term carries the entries it would move. The scan does not decide that a
# match matters; it decides that a person should look. A term too broad buries
# the list and a term too narrow misses the one release note that mattered, so
# these are the nouns the blockers actually use.
$interest = @(
    @{ term = "mse";                entries = "T-163" },
    @{ term = "encrypt";            entries = "T-163" },
    @{ term = "obfusc";             entries = "T-163" },
    @{ term = "holepunch";          entries = "T-102" },
    @{ term = "hole punch";         entries = "T-102" },
    @{ term = "bep 55";             entries = "T-102" },
    @{ term = "donthave";           entries = "T-167" },
    @{ term = "bep 54";             entries = "T-167" },
    @{ term = "extension message";  entries = "T-100, T-102, T-167" },
    @{ term = "peerconnectionhandler"; entries = "T-100, T-102, T-167" },
    @{ term = "close_wait";         entries = "T-020" },
    @{ term = "listener";           entries = "T-020" },
    @{ term = "handshake";          entries = "T-020" },
    @{ term = "accept";             entries = "T-020" },
    @{ term = "persistence";        entries = "T-016" },
    @{ term = "resume";             entries = "T-016" },
    @{ term = "fastresume";         entries = "T-016" },
    @{ term = "session_persistence"; entries = "T-016" },
    @{ term = "tracker";            entries = "T-022" },
    @{ term = "announce";           entries = "T-022" },
    @{ term = "ipv6";               entries = "T-022" },
    @{ term = "dual stack";         entries = "T-022" },
    @{ term = "storage";            entries = "T-132, T-188" },
    @{ term = "torrentstorage";     entries = "T-132" },
    @{ term = "peer id";            entries = "T-132" },
    @{ term = "only_files";         entries = "T-185, T-188" },
    @{ term = "select";             entries = "T-185, T-188" },
    @{ term = "memory";             entries = "T-040" },
    @{ term = "leak";               entries = "T-040" },
    @{ term = "bound";              entries = "T-040" },
    @{ term = "upload slot";        entries = "T-024" },
    @{ term = "choke";              entries = "T-024" }
)

function Get-Flags([string]$Text) {
    if (-not $Text) { return @() }
    $lower = $Text.ToLowerInvariant()
    $hits = [System.Collections.ArrayList]::new()
    foreach ($row in $interest) {
        if ($lower.Contains($row.term)) {
            [void]$hits.Add("$($row.term) -> $($row.entries)")
        }
    }
    $hits
}

# An instant as ISO 8601 UTC, whatever ConvertFrom-Json made of it.
#
# It parses an ISO timestamp into a [DateTime], and interpolating one of those
# into a string uses the current culture: "02/10/2026 16:44:41" on this
# machine. Putting that in a `since=` query gives GitHub something it cannot
# read, and the retry loop then spends two and a half minutes on it. Every
# instant that reaches a URL or the record goes through here. TODO/RULES.md
# section 5 already says ISO 8601 UTC everywhere; this is where it was easiest
# to lose.
function Format-Iso($value) {
    if ($null -eq $value) { return $null }
    if ($value -is [datetime]) { return $value.ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ") }
    $parsed = [datetime]::MinValue
    if ([datetime]::TryParse([string]$value, [ref]$parsed)) {
        return $parsed.ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    }
    [string]$value
}

$doc = Get-Content $Manifest -Raw | ConvertFrom-Json
$selected = @($doc.upstreams | Where-Object { $Upstream -eq "all" -or $_.name -eq $Upstream })
if ($selected.Count -eq 0) { Exit-With 2 "no upstream named '$Upstream'" }

$stamp = (Get-Date).ToUniversalTime().ToString("yyyyMMddTHHmmssfffZ")
New-Item -ItemType Directory -Force -Path $Out | Out-Null
$record = [ordered]@{
    scanned_at = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ss.fffZ")
    since      = $Since
    upstreams  = [System.Collections.ArrayList]::new()
}

foreach ($up in $selected) {
    $slug = ([uri]$up.repository).AbsolutePath.Trim('/')
    Say "$($up.name): $slug, base $($up.base.Substring(0,12))"

    # The base commit's own date bounds every other query. Without it a scan
    # of a ten year old repository asks for everything.
    $baseCommit = Invoke-GhApi "repos/$slug/commits/$($up.base)"
    $baseDate = if ($baseCommit) { Format-Iso $baseCommit.commit.committer.date } else { $null }
    $cutoff = if ($Since) { Format-Iso $Since } elseif ($baseDate) { $baseDate } else { $null }
    if (-not $cutoff) {
        Say "  could not date the base commit; pass -Since to bound the scan"
        continue
    }
    Say "  looking at everything after $cutoff"

    # String comparison, and it is correct because both sides are ISO 8601 UTC
    # of the same width, which sorts the same way the instants do. A [DateTime]
    # against a string would compare a date to text.
    $releases = @(Invoke-GhPaged "repos/$slug/releases" 2 | Where-Object { $_.published_at -and (Format-Iso $_.published_at) -gt $cutoff })
    $commits = @(Invoke-GhPaged "repos/$slug/commits?since=$cutoff" $MaxPages)
    # `issues` returns pull requests too, which is why each is separated on
    # `pull_request` rather than asked for twice.
    $issuesAndPulls = @(Invoke-GhPaged "repos/$slug/issues?state=all&since=$cutoff&sort=updated&direction=desc" $MaxPages)
    $pulls = @($issuesAndPulls | Where-Object { $null -ne $_.pull_request })
    $issues = @($issuesAndPulls | Where-Object { $null -eq $_.pull_request })

    $rows = [System.Collections.ArrayList]::new()
    foreach ($item in $releases) {
        [void]$rows.Add([ordered]@{ kind = "release"; ref = $item.tag_name; title = $item.name; state = "published"; at = (Format-Iso $item.published_at); url = $item.html_url; flags = @(Get-Flags "$($item.name) $($item.body)") })
    }
    foreach ($item in $commits) {
        $subject = ($item.commit.message -split "`n")[0]
        [void]$rows.Add([ordered]@{ kind = "commit"; ref = $item.sha.Substring(0, 12); title = $subject; state = "merged"; at = (Format-Iso $item.commit.committer.date); url = $item.html_url; flags = @(Get-Flags $item.commit.message) })
    }
    foreach ($item in $pulls) {
        [void]$rows.Add([ordered]@{ kind = "pr"; ref = "#$($item.number)"; title = $item.title; state = $item.state; at = (Format-Iso $item.updated_at); url = $item.html_url; flags = @(Get-Flags "$($item.title) $($item.body)") })
    }
    foreach ($item in $issues) {
        [void]$rows.Add([ordered]@{ kind = "issue"; ref = "#$($item.number)"; title = $item.title; state = $item.state; at = (Format-Iso $item.updated_at); url = $item.html_url; flags = @(Get-Flags "$($item.title) $($item.body)") })
    }

    $flagged = @($rows | Where-Object { $_.flags.Count -gt 0 })
    Say "  $($releases.Count) release(s), $($commits.Count) commit(s), $($pulls.Count) pull request(s), $($issues.Count) issue(s); $($flagged.Count) flagged"

    [void]$record.upstreams.Add([ordered]@{
        name = $up.name; repository = $up.repository; base = $up.base
        base_committed_at = $baseDate; cutoff = $cutoff
        counts = [ordered]@{ releases = $releases.Count; commits = $commits.Count; pulls = $pulls.Count; issues = $issues.Count; flagged = $flagged.Count }
        rows = @($rows)
    })
}

$path = Join-Path $Out "upstream-$stamp.json"
$record | ConvertTo-Json -Depth 12 | Set-Content -Path $path -Encoding utf8
Say "record: $path"

Write-Host ""
foreach ($up in $record.upstreams) {
    Write-Host "$($up.name): $($up.counts.flagged) flagged of $($up.rows.Count) since $($up.cutoff)"
    $show = if ($FlaggedOnly) { @($up.rows | Where-Object { $_.flags.Count -gt 0 }) } else { @($up.rows) }
    foreach ($row in ($show | Sort-Object { $_.flags.Count } -Descending)) {
        $mark = if ($row.flags.Count -gt 0) { "*" } else { " " }
        $title = if ($row.title.Length -gt 68) { $row.title.Substring(0, 65) + "..." } else { $row.title }
        Write-Host ("  {0} {1,-7} {2,-9} {3}" -f $mark, $row.kind, $row.ref, $title)
        foreach ($flag in $row.flags) { Write-Host "              $flag" }
    }
    Write-Host ""
}
Write-Host "A flag says a person should look, not that anything is wrong."
Write-Host "Turning one into an entry is patches/TASKS.md's job."
exit 0
