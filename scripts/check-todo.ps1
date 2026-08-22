# The two deep reviews, for the half a machine can do.
#
# TODO/RULES.md section 2 ends every session with two reviews: every claim
# against the code or the path it cites, and then a cold read looking for a doc
# contradicting another doc, an entry id that does not exist, a cited path that
# does not resolve, counts that no longer add up. Three of those four are
# mechanical, and doing them by hand is how they get skipped.
#
# On 2026-08-22 this would have caught two things that had been wrong for at
# least a session: TODO/INDEX.md's row for T-184 said `open` while its entry
# said `done`, and the priority table totalled 141 against 146 rows.
#
# What it checks:
#
#   1. Every INDEX row's status matches that entry's own `Status:` line.
#   2. Every entry in TODO/*.md has a row in INDEX.md, and every row has an
#      entry.
#   3. The counts prose and the priority table both agree with the rows.
#   4. Every `T-NNN` referenced from any TODO file is an entry that exists.
#   5. Every `TODO/<file>.md` and `(file.md)` link resolves.
#   6. Every `crates/...:NNN` citation resolves to a file with that many lines.
#   7. No file has a NUL byte in it. One got in on 2026-08-22 and `grep`
#      answered "Binary file TODO/trackers.md matches" instead of the line.
#
# What it does not check: whether a claim is true. That is the review this does
# not replace, and the point of doing the mechanical half in one second is to
# leave the time for the half that needs reading.
#
# Usage:
#   pwsh -NoProfile -File scripts/check-todo.ps1
#   pwsh -NoProfile -File scripts/check-todo.ps1 -Json
#
# Exit codes: 0 everything agrees, 1 something does not, 2 could not run.

[CmdletBinding()]
param([switch]$Json)

$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
$todo = Join-Path $repo "TODO"
if (-not (Test-Path $todo)) {
    [Console]::Error.WriteLine("check-todo: no TODO/ at $repo")
    exit 2
}

$problems = [System.Collections.ArrayList]::new()
function Problem([string]$kind, [string]$text) {
    [void]$problems.Add([ordered]@{ kind = $kind; detail = $text })
}

$files = @(Get-ChildItem -Path $todo -Filter *.md -File)

# ---------------------------------------------------------------------------
# 0. Bytes, before anything reads these as text
# ---------------------------------------------------------------------------
#
# A NUL byte in a tracked Markdown file makes `grep` call it binary and skip
# it, makes a diff unreadable, and hides whatever is around it from every text
# tool including this one. It got in on 2026-08-22 by way of a backslash-x-0-0
# escape written into a Python string that then interpreted the escape, in a
# sentence quoting a tracker's error message. This is one line to check and it
# is checked first, because everything below reads these files as text.
#
# `gates.ps1` has a `text` gate over the whole tracked tree, so these files are
# covered twice. Deliberately: this script is the mechanical half of the two
# reviews and gets run on its own, and a review that reads a file `grep` would
# have skipped is the review this is meant to catch.

foreach ($file in $files) {
    $bytes = [System.IO.File]::ReadAllBytes($file.FullName)
    $at = [System.Array]::IndexOf($bytes, [byte]0)
    if ($at -ge 0) {
        Problem "nul-byte" "$($file.Name) has a NUL byte at offset $at, so every text tool will treat it as binary"
    }
}

# ---------------------------------------------------------------------------
# Read every entry
# ---------------------------------------------------------------------------

$entries = @{}
foreach ($file in $files) {
    $current = $null
    $lineNo = 0
    foreach ($line in [System.IO.File]::ReadAllLines($file.FullName)) {
        $lineNo++
        if ($line -match '^###\s+(T-\d+)\b') {
            $current = $Matches[1]
            if ($entries.ContainsKey($current)) {
                Problem "duplicate-entry" "$current is defined in both $($entries[$current].file) and $($file.Name)"
            }
            else {
                $entries[$current] = [ordered]@{ file = $file.Name; line = $lineNo; status = $null }
            }
            continue
        }
        if ($current -and $null -eq $entries[$current].status -and $line -match '^Status:\s*(.+)$') {
            $entries[$current].status = $Matches[1].Trim()
        }
    }
}

# A status line is prose, not a token. Normalise to the five words the index
# uses, taking the first one that appears: "**done**, with the premise
# corrected below" is done, and "open, blocked" is blocked.
function Normalize([string]$status) {
    if (-not $status) { return $null }
    $plain = ($status -replace '\*', '').Trim().ToLowerInvariant()
    foreach ($word in @('deferred', 'blocked', 'partial', 'done', 'open')) {
        if ($plain -match "(^|\W)$word(\W|$)") { return $word }
    }
    return $plain
}

# ---------------------------------------------------------------------------
# Read the index rows
# ---------------------------------------------------------------------------

$indexPath = Join-Path $todo "INDEX.md"
$indexText = [System.IO.File]::ReadAllText($indexPath)
$rows = @{}
$rowOrder = [System.Collections.ArrayList]::new()
foreach ($line in ($indexText -split "`r?`n")) {
    if ($line -match '^\|\s*\[(T-\d+)\]\(([^)]+)\)\s*\|\s*([^|]+?)\s*\|\s*([^|]+?)\s*\|\s*([^|]+?)\s*\|') {
        $id = $Matches[1]
        if ($rows.ContainsKey($id)) { Problem "duplicate-row" "$id has more than one row in INDEX.md" }
        $rows[$id] = [ordered]@{
            file     = $Matches[2]
            priority = $Matches[3].Trim()
            status   = ($Matches[5] -replace '\*', '').Trim()
        }
        [void]$rowOrder.Add($id)
    }
}

# ---------------------------------------------------------------------------
# 1 and 2: rows and entries agree, and both exist
# ---------------------------------------------------------------------------

foreach ($id in $rows.Keys) {
    if (-not $entries.ContainsKey($id)) {
        Problem "row-without-entry" "$id has a row in INDEX.md and no `### $id` anywhere in TODO/"
        continue
    }
    $entryStatus = Normalize $entries[$id].status
    $rowStatus = $rows[$id].status
    if (-not $entryStatus) {
        Problem "entry-without-status" "$id in $($entries[$id].file) has no Status: line"
        continue
    }
    if ($entryStatus -ne $rowStatus) {
        Problem "status-mismatch" "$id : INDEX.md says '$rowStatus', $($entries[$id].file):$($entries[$id].line) says '$($entries[$id].status)'"
    }
    $linked = $rows[$id].file
    if ($linked -ne $entries[$id].file) {
        Problem "wrong-link" "$id : INDEX.md links to $linked, the entry is in $($entries[$id].file)"
    }
}
foreach ($id in $entries.Keys) {
    if (-not $rows.ContainsKey($id)) {
        Problem "entry-without-row" "$id is defined in $($entries[$id].file):$($entries[$id].line) with no row in INDEX.md"
    }
}

# ---------------------------------------------------------------------------
# 3: the counts
# ---------------------------------------------------------------------------

$byState = @{}
$byPriority = @{}
foreach ($id in $rows.Keys) {
    $state = $rows[$id].status
    $priority = $rows[$id].priority
    if (-not $byState.ContainsKey($state)) { $byState[$state] = 0 }
    $byState[$state]++
    $key = "$priority/$state"
    if (-not $byPriority.ContainsKey($key)) { $byPriority[$key] = 0 }
    $byPriority[$key]++
}
function Count([string]$state) { if ($byState.ContainsKey($state)) { $byState[$state] } else { 0 } }

$total = $rows.Count
if ($indexText -match '(?m)^(\d+) items:\s*(\d+) to work through, and (\d+) deferred') {
    $claimTotal = [int]$Matches[1]
    $claimWork = [int]$Matches[2]
    $claimDeferred = [int]$Matches[3]
    if ($claimTotal -ne $total) { Problem "count-prose" "the prose says $claimTotal items, the rows say $total" }
    if ($claimDeferred -ne (Count 'deferred')) { Problem "count-prose" "the prose says $claimDeferred deferred, the rows say $(Count 'deferred')" }
    if ($claimWork -ne ($total - (Count 'deferred'))) { Problem "count-prose" "the prose says $claimWork to work through, the rows say $($total - (Count 'deferred'))" }
}
else { Problem "count-prose" "INDEX.md has no '<N> items: <N> to work through' line to check" }

if ($indexText -match '(?m)^(\d+) open, (\d+) partial, (\d+) blocked, (\d+) done\.') {
    $claimed = @{ open = [int]$Matches[1]; partial = [int]$Matches[2]; blocked = [int]$Matches[3]; done = [int]$Matches[4] }
    foreach ($state in $claimed.Keys) {
        if ($claimed[$state] -ne (Count $state)) {
            Problem "count-prose" "the prose says $($claimed[$state]) $state, the rows say $(Count $state)"
        }
    }
}
else { Problem "count-prose" "INDEX.md has no '<N> open, <N> partial, <N> blocked, <N> done.' line to check" }

# The priority table. `| P1 | 3 | 1 | 0 | 47 | 51 |` is open, partial, blocked,
# done, total.
$tableSeen = $false
foreach ($line in ($indexText -split "`r?`n")) {
    if ($line -match '^\|\s*(P[0-3])\s*\|\s*(\d+)\s*\|\s*(\d+)\s*\|\s*(\d+)\s*\|\s*(\d+)\s*\|\s*(\d+)\s*\|') {
        $tableSeen = $true
        $priority = $Matches[1]
        $want = @{
            open    = [int]$Matches[2]
            partial = [int]$Matches[3]
            blocked = [int]$Matches[4]
            done    = [int]$Matches[5]
        }
        $rowTotal = [int]$Matches[6]
        $sum = 0
        foreach ($state in $want.Keys) {
            $actual = if ($byPriority.ContainsKey("$priority/$state")) { $byPriority["$priority/$state"] } else { 0 }
            if ($want[$state] -ne $actual) {
                Problem "count-table" "$priority $state : the table says $($want[$state]), the rows say $actual"
            }
            $sum += $want[$state]
        }
        if ($sum -ne $rowTotal) { Problem "count-table" "$priority : the row sums to $sum and its total column says $rowTotal" }
    }
    elseif ($line -match '^\|\s*\*\*All\*\*\s*\|\s*\*\*(\d+)\*\*\s*\|\s*\*\*(\d+)\*\*\s*\|\s*\*\*(\d+)\*\*\s*\|\s*\*\*(\d+)\*\*\s*\|\s*\*\*(\d+)\*\*\s*\|') {
        $tableSeen = $true
        $want = @{ open = [int]$Matches[1]; partial = [int]$Matches[2]; blocked = [int]$Matches[3]; done = [int]$Matches[4] }
        foreach ($state in $want.Keys) {
            if ($want[$state] -ne (Count $state)) {
                Problem "count-table" "the All row says $($want[$state]) $state, the rows say $(Count $state)"
            }
        }
        if ([int]$Matches[5] -ne $total) { Problem "count-table" "the All row totals $($Matches[5]), the rows say $total" }
    }
}
if (-not $tableSeen) { Problem "count-table" "INDEX.md has no priority table to check" }

# ---------------------------------------------------------------------------
# 4, 5 and 6: references resolve
# ---------------------------------------------------------------------------

$known = [System.Collections.Generic.HashSet[string]]::new()
foreach ($id in $entries.Keys) { [void]$known.Add($id) }

# reference/ is gitignored and lives on the `references` branch, so a clone
# that has not fetched it cannot check a corpus citation. Absent is not a
# failure; it is one fewer thing this run can say.
$corpus = Join-Path $repo "reference"
$corpusPresent = Test-Path $corpus -PathType Container

foreach ($file in $files) {
    $text = [System.IO.File]::ReadAllText($file.FullName)
    $lineNo = 0
    foreach ($line in ($text -split "`r?`n")) {
        $lineNo++
        # A T-NNN that names no entry. Anchors and links both.
        foreach ($m in [regex]::Matches($line, '\bT-(\d{3})\b')) {
            $id = "T-$($m.Groups[1].Value)"
            if (-not $known.Contains($id)) {
                Problem "unknown-entry" "$($file.Name):$lineNo references $id, which is not an entry"
            }
        }
        # A markdown link to a sibling TODO file.
        foreach ($m in [regex]::Matches($line, '\]\((?<t>[A-Za-z0-9._-]+\.md)(?:#[^)]*)?\)')) {
            $target = $m.Groups['t'].Value
            if (-not (Test-Path (Join-Path $todo $target))) {
                Problem "dead-link" "$($file.Name):$lineNo links to $target, which is not in TODO/"
            }
        }
        # A citation into this tree, as `crates/a/b.rs:123`. The lookbehind is
        # load-bearing: without it `TorrentNG/crates/rt-storage/src/x.rs` from
        # the corpus matches from `crates/` and is reported as a path this
        # repository does not have, which is true and not the question.
        foreach ($m in [regex]::Matches($line, '(?<![\w./-])(?<p>(?:crates|scripts|docs)/[A-Za-z0-9._/-]+\.(?:rs|ps1|md|toml|json|jq|yml))(?::(?<l>\d+))?')) {
            $cited = $m.Groups['p'].Value
            # A path written with an ellipsis is deliberately abbreviated and
            # there is nothing to resolve.
            if ($cited -match '\.\.\.') { continue }
            $path = Join-Path $repo $cited
            if (-not (Test-Path $path)) {
                Problem "dead-path" "$($file.Name):$lineNo cites $cited, which is not there"
                continue
            }
            if ($m.Groups['l'].Success) {
                $count = (Get-Content -LiteralPath $path | Measure-Object -Line).Lines
                if ([int]$m.Groups['l'].Value -gt $count) {
                    Problem "dead-line" "$($file.Name):$lineNo cites ${cited}:$($m.Groups['l'].Value) and that file has $count lines"
                }
            }
        }
        # A citation into the corpus, as `TorrentNG/crates/a/b.rs:123`. Only
        # checkable when reference/ is on this machine, which is the case
        # TODO/RULES.md section 7 asks for: verify a path before citing it,
        # one Test-Path is the whole check.
        if ($corpusPresent) {
            # The path part must itself contain a directory. `torrent/x.rs`
            # is far more often this tree's `crates/bit-cli-core/src/torrent/`
            # written short than it is the corpus tree called `torrent`, and a
            # checker that cries wolf is a checker nobody runs.
            foreach ($m in [regex]::Matches($line, '(?<![\w./-])(?<r>[A-Za-z0-9_-]+)/(?<p>[A-Za-z0-9._-]+/[A-Za-z0-9._/-]+\.(?:rs|go|py|ts|js|md|toml|json))(?::(?<l>\d+))?')) {
                $tree = $m.Groups['r'].Value
                $treeRoot = Join-Path $corpus $tree
                if (-not (Test-Path $treeRoot -PathType Container)) { continue }
                $cited = "$tree/$($m.Groups['p'].Value)"
                if ($cited -match '\.\.\.') { continue }
                $path = Join-Path $corpus $cited
                if (-not (Test-Path $path)) {
                    Problem "dead-corpus-path" "$($file.Name):$lineNo cites $cited, which is not in reference/"
                    continue
                }
                if ($m.Groups['l'].Success) {
                    $count = (Get-Content -LiteralPath $path | Measure-Object -Line).Lines
                    if ([int]$m.Groups['l'].Value -gt $count) {
                        Problem "dead-corpus-line" "$($file.Name):$lineNo cites ${cited}:$($m.Groups['l'].Value) and that file has $count lines"
                    }
                }
            }
        }
    }
}

# ---------------------------------------------------------------------------
# Say it
# ---------------------------------------------------------------------------

if ($Json) {
    [ordered]@{
        kind           = "check-todo"
        schema_version = "1"
        generated_at   = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ss.fffZ")
        entries        = $entries.Count
        rows           = $rows.Count
        by_state       = $byState
        ok             = ($problems.Count -eq 0)
        problems       = @($problems)
    } | ConvertTo-Json -Depth 6
    exit $(if ($problems.Count -eq 0) { 0 } else { 1 })
}

Write-Host ""
Write-Host ("check-todo: {0} entries, {1} rows" -f $entries.Count, $rows.Count)
$order = @('open', 'partial', 'blocked', 'done', 'deferred')
Write-Host ("  states: " + (($order | Where-Object { $byState.ContainsKey($_) } | ForEach-Object { "$_ $($byState[$_])" }) -join ', '))
if ($problems.Count -eq 0) {
    Write-Host "  everything agrees"
    Write-Host ""
    exit 0
}
Write-Host ""
Write-Host "$($problems.Count) problem(s):"
foreach ($item in $problems) {
    Write-Host ("  [{0}] {1}" -f $item.kind, $item.detail)
}
Write-Host ""
exit 1
