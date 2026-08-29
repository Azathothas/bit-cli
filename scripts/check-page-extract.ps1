# Does link extraction find exactly what the proving ground says is there?
#
# The defect it exists to catch: `TODO/cli-surface.md` T-244 reads a web page
# and picks the torrent out of it, and every part of that is a judgement about
# markup. A `.torrent` in the link text rather than the href, a `.torrent.html`,
# a link in a comment, a `<base href>` that moves every relative URL, an
# unquoted attribute value: each one is a way to return a link that is not
# there or to miss one that is. Nothing about that is visible from a report
# that says "found 3 links".
#
# So this serves `scripts/make-page-fixture.ps1`'s six levels through
# `loopback-fileserver` and compares what `bit-cli` extracts against the answer
# recorded beside each page. Nothing here touches the network.
#
# What each case asserts, from the expected count:
#
#   0 links   the run is refused, `matches` is 0, and the message names
#             `--render` as the next thing to try
#   1 link    the run resolves the torrent and reports it, exit 0. A single
#             magnet is routed to the swarm resolver instead, which with
#             `--no-dht --no-lsd --no-tracker` and no peers refuses at once,
#             and that refusal is the proof the magnet was chosen
#   2 or more the run is refused with exit 4 and its JSON `page_links` is
#             compared **in order** against the recorded answer, URL, anchor
#             text and kind
#
# Usage:
#   pwsh scripts/check-page-extract.ps1
#   pwsh scripts/check-page-extract.ps1 -Json
#   pwsh scripts/check-page-extract.ps1 -Port 8099 -Keep
#
# Exit 0 when every case matched, 1 when one did not, 2 when it could not run.
#
# See TODO/cli-surface.md, T-244.

[CmdletBinding()]
param(
    [int]$Port = 8099,
    [switch]$Json,
    # Leave the fixture and the served directory in place afterwards.
    [switch]$Keep,
    [string]$Root = ".tmp/page-extract",
    [ValidateSet("debug", "release")]
    [string]$Profile = "release"
)

$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot

function Exit-With([int]$code, [string]$message) {
    [Console]::Error.WriteLine("check-page-extract: $message")
    exit $code
}

$exeDir = Join-Path $repo "target/$Profile"
$bit = Join-Path $exeDir "bit-cli.exe"
if (-not (Test-Path $bit)) { $bit = Join-Path $exeDir "bit-cli" }
$server = Join-Path $exeDir "examples/loopback-fileserver.exe"
if (-not (Test-Path $server)) { $server = Join-Path $exeDir "examples/loopback-fileserver" }
foreach ($p in @($bit, $server)) {
    if (-not (Test-Path $p)) {
        Exit-With 2 "$p is missing; run: cargo build --$Profile --bins --examples"
    }
}

$base = "http://127.0.0.1:$Port"
$fixtureRoot = if ([System.IO.Path]::IsPathRooted($Root)) { $Root } else { Join-Path $repo $Root }
if (Test-Path $fixtureRoot) { Remove-Item -Recurse -Force $fixtureRoot }
New-Item -ItemType Directory -Force -Path $fixtureRoot | Out-Null

& (Join-Path $PSScriptRoot 'make-page-fixture.ps1') -BaseUrl $base -Root $fixtureRoot *> $null
if ($LASTEXITCODE -ne 0) { Exit-With 2 "make-page-fixture failed" }

$indexPath = Join-Path $fixtureRoot 'index.json'
if (-not (Test-Path $indexPath)) { Exit-With 2 "no index.json under $fixtureRoot" }
$index = Get-Content -Raw $indexPath | ConvertFrom-Json

# A real torrent behind every single-link case, so "one link resolves" is
# proved by the report rather than by the absence of an error.
$files = Join-Path $fixtureRoot 'files'
New-Item -ItemType Directory -Force -Path $files | Out-Null
$payload = Join-Path $fixtureRoot 'payload.bin'
$bytes = New-Object byte[] 4096
for ($i = 0; $i -lt 4096; $i++) { $bytes[$i] = [byte]($i % 251) }
[System.IO.File]::WriteAllBytes($payload, $bytes)
& $bit create $payload --no-creation-date --no-created-by --piece-length 16KiB `
    -o (Join-Path $files 'only.torrent') --force *> $null
if ($LASTEXITCODE -ne 0) { Exit-With 2 "cannot build the fixture torrent" }
foreach ($name in @('in-source.torrent', 'first.torrent', 'second.torrent', 'real.torrent')) {
    Copy-Item (Join-Path $files 'only.torrent') (Join-Path $files $name) -Force
}

$serverOut = Join-Path $fixtureRoot 'server-out.txt'
$serverErr = Join-Path $fixtureRoot 'server-err.txt'
$proc = Start-Process -FilePath $server -ArgumentList '--root', $fixtureRoot, '--port', "$Port" `
    -PassThru -NoNewWindow -RedirectStandardOutput $serverOut -RedirectStandardError $serverErr

# Wait on the fixture's own contract: one base URL line on stdout before the
# first request is served. Never on a guessed duration.
$ready = $false
for ($i = 0; $i -lt 100; $i++) {
    Start-Sleep -Milliseconds 100
    if ((Test-Path $serverOut) -and (Get-Content $serverOut -TotalCount 1 -ErrorAction SilentlyContinue)) {
        $ready = $true
        break
    }
    if ($proc.HasExited) { break }
}
if (-not $ready) {
    if (-not $proc.HasExited) { $proc | Stop-Process -Force -ErrorAction SilentlyContinue }
    Exit-With 2 "the file server never announced itself on port $Port"
}

# Run one command, returning stdout, stderr and the exit code read from the
# process rather than from a pipeline. docs/AGENTS.md: an exit code is read
# from the process that produced it, unpiped.
function Invoke-Bit([string[]]$argv) {
    $o = Join-Path $fixtureRoot "run-out.txt"
    $e = Join-Path $fixtureRoot "run-err.txt"
    $p = Start-Process -FilePath $bit -ArgumentList $argv -PassThru -NoNewWindow -Wait `
        -RedirectStandardOutput $o -RedirectStandardError $e
    return @{
        code   = $p.ExitCode
        stdout = (Get-Content -Raw $o -ErrorAction SilentlyContinue)
        stderr = (Get-Content -Raw $e -ErrorAction SilentlyContinue)
    }
}

# The info hash out of an `info --json` document, or $null when the document
# is not one. Reading the field rather than grepping the text: `--json` writes
# `info_hash` and the table writes `info hash`, and a check that matches the
# table passes on an empty document.
function Get-InfoHash([string]$stdout) {
    if (-not $stdout) { return $null }
    try {
        $doc = $stdout | ConvertFrom-Json
    } catch {
        return $null
    }
    foreach ($candidate in @($doc.info_hash, $doc.torrent.info_hash, $doc.infohash)) {
        if ($candidate) { return "$candidate" }
    }
    return $null
}

function Compare-Links($want, $got) {
    if ($null -eq $got) { return "no page_links in the JSON error" }
    if (@($want).Count -ne @($got).Count) {
        return "want $(@($want).Count) link(s), got $(@($got).Count)"
    }
    for ($i = 0; $i -lt @($want).Count; $i++) {
        $w = $want[$i]
        $g = $got[$i]
        if ($w.url -cne $g.url) { return "link $($i + 1): url want '$($w.url)' got '$($g.url)'" }
        if ($w.text -cne $g.text) { return "link $($i + 1): text want '$($w.text)' got '$($g.text)'" }
        if ($w.kind -cne $g.kind) { return "link $($i + 1): kind want '$($w.kind)' got '$($g.kind)'" }
    }
    return $null
}

$results = @()
try {
    foreach ($case in $index.cases) {
        $expected = Get-Content -Raw (Join-Path $fixtureRoot $case.expected) | ConvertFrom-Json
        $want = @($expected.static)
        $url = $case.url
        $row = [ordered]@{
            case           = $case.case
            level          = $case.level
            expected_static = $want.Count
            expected_rendered = @($expected.rendered).Count
            exit_code      = $null
            pass           = $false
            detail         = ""
        }

        $onlyMagnet = $want.Count -eq 1 -and $want[0].kind -eq 'magnet'
        $argv = @('info', $url, '--json')
        if ($onlyMagnet) { $argv += @('--no-dht', '--no-lsd', '--no-tracker', '--timeout', '5s') }
        $run = Invoke-Bit $argv
        $row.exit_code = $run.code
        $text = "$($run.stdout)`n$($run.stderr)"

        if ($want.Count -eq 0) {
            if ($run.code -ne 4) { $row.detail = "expected exit 4, got $($run.code)" }
            elseif ($text -notmatch '--render') { $row.detail = "the refusal does not name --render" }
            else { $row.pass = $true; $row.detail = "refused, and named --render" }
        } elseif ($want.Count -eq 1 -and -not $onlyMagnet) {
            if ($run.code -ne 0) {
                $row.detail = "expected exit 0, got $($run.code): $(($text -split "`n" | Where-Object { $_ } | Select-Object -First 1))"
            } elseif (-not (Get-InfoHash $run.stdout)) {
                $row.detail = "exit 0 but no torrent was reported"
            } else {
                $row.pass = $true
                $row.detail = "resolved the one link, info hash $(Get-InfoHash $run.stdout)"
            }
        } elseif ($onlyMagnet) {
            # The magnet was chosen and handed to the swarm resolver, which
            # with every discovery mechanism off has nowhere to ask.
            if ($run.code -ne 4) { $row.detail = "expected exit 4, got $($run.code)" }
            elseif ($text -notmatch [regex]::Escape($want[0].url.Substring(0, 40))) {
                $row.detail = "the refusal does not name the magnet that was chosen"
            } else {
                $row.pass = $true
                $row.detail = "the single magnet reached the swarm resolver"
            }
        } else {
            if ($run.code -ne 4) {
                $row.detail = "expected exit 4, got $($run.code)"
            } else {
                $doc = $null
                try { $doc = $run.stdout | ConvertFrom-Json } catch { }
                $problem = Compare-Links $want $doc.context.page_links
                if ($problem) { $row.detail = $problem }
                else {
                    $row.pass = $true
                    $row.detail = "$($want.Count) link(s), in order, with their anchor text"
                }
            }
        }

        $results += [pscustomobject]$row
        $mark = if ($row.pass) { "ok  " } else { "FAIL" }
        Write-Host ("{0} {1,-14} level {2}  {3}" -f $mark, $case.case, $case.level, $row.detail)
    }
} finally {
    if (-not $proc.HasExited) { $proc | Stop-Process -Force -ErrorAction SilentlyContinue }
    if (-not $Keep) { Remove-Item -Recurse -Force $fixtureRoot -ErrorAction SilentlyContinue }
}

# What only a rendered tier can reach. L0 to L3 must be identical between the
# tiers, and a difference there is a defect in the extractor rather than a
# property of the page.
$divergence = @()
foreach ($row in $results) {
    if ($row.expected_rendered -ne $row.expected_static) {
        $divergence += [ordered]@{
            case      = $row.case
            level     = $row.level
            static    = $row.expected_static
            rendered  = $row.expected_rendered
            only_rendered = $row.expected_rendered - $row.expected_static
        }
    }
}
$badDivergence = @($divergence | Where-Object { $_.level -le 3 })

$failed = @($results | Where-Object { -not $_.pass })
$pass = $failed.Count -eq 0 -and $badDivergence.Count -eq 0

$report = [ordered]@{
    schema      = "page-extract/1"
    generated   = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ss.fffZ")
    base_url    = $base
    cases       = $results.Count
    passed      = $results.Count - $failed.Count
    failed      = $failed.Count
    divergence  = $divergence
    pass        = $pass
    results     = $results
}

if ($Json) {
    $report | ConvertTo-Json -Depth 6
} else {
    Write-Host ""
    Write-Host ("check-page-extract: {0} case(s), {1} passed, {2} failed" -f `
            $results.Count, ($results.Count - $failed.Count), $failed.Count)
    if ($divergence.Count -gt 0) {
        Write-Host "  links only a rendered tier reaches:"
        foreach ($d in $divergence) {
            Write-Host ("    {0,-14} level {1}  static {2}  rendered {3}  (+{4})" -f `
                    $d.case, $d.level, $d.static, $d.rendered, $d.only_rendered)
        }
    }
    if ($badDivergence.Count -gt 0) {
        Write-Host "  a level at or below 3 differs between the tiers, which is an extractor defect"
    }
}

if (-not $pass) { exit 1 }
exit 0
