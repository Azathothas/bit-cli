<#
.SYNOPSIS
    Derive a BitTorrent client identity profile from that client's own tagged
    source, and refuse to guess when the source no longer says what it said.

.DESCRIPTION
    A client profile is not a string. What a tracker and a peer see is a peer
    id, a User-Agent, a set of query parameters in a fixed order, a key whose
    alphabet and width are the client's own, and a set of headers. Getting one
    of those wrong is what makes a mask fail on the second check.

    The defect this exists to catch is a profile copied from another emulator
    rather than derived from the client. Five projects share one profile format
    and no two of them agree on what it means. Four of the five never emit a
    qBittorrent `key` with a leading zero; libtorrent writes `key=%08X`, so a
    real one starts with `0` once in sixteen. Every one of them reproduced the
    format faithfully and the client not at all.

    So this reads the client's own repository at a tag, extracts the version
    constants and the identity construction, and asserts that the construction
    is still the one it knows how to read. When an upstream file moves or a
    line changes, it exits 1 and names what moved, rather than emitting a
    profile that describes a client that no longer exists.

        pwsh -NoProfile -File scripts/make-client-profile.ps1 -Client qbittorrent -Version 5.2.3
        pwsh -NoProfile -File scripts/make-client-profile.ps1 -Client transmission -Version 4.1.3 -Json
        pwsh -NoProfile -File scripts/make-client-profile.ps1 -SelfTest

    Exit codes follow the check-script contract: 0 the profile was derived and
    every guard held, 1 a guard failed or a value could not be extracted, 2 the
    run could not start, which here means `gh` is missing or the network is not
    reachable.

    Every fetch is a read of a public repository. Nothing is written anywhere
    but the path given by -Out, and nothing is announced anywhere.

.NOTES
    Ported from joal's `scripts/bittorrent-client-update-detector/`, Apache-2.0,
    at 90e710ba01ac6a8665eb352a612ce4e9581483c8. This is an independent
    implementation written from the observed behaviour of those two scripts;
    no line was copied. What it does differently is in `docs/reference-mining.md`
    and under T-234 in `TODO/peers.md`. The three that matter:

      - the version to character encoding is table driven and tested over its
        whole range, so a component of 10 or more produces `A` rather than two
        characters and a peer id one byte too long
      - every value the run extracts is used, rather than extracted, printed,
        and then replaced by a hardcoded template
      - the profile carries the peer wire surface as well as the announce, and
        says which fields were derived and which were left unknown
#>

[CmdletBinding()]
param(
    [ValidateSet('qbittorrent', 'transmission')]
    [string]$Client = 'qbittorrent',
    [string]$Version,
    [string]$Out,
    [switch]$Json,
    [switch]$SelfTest
)

$ErrorActionPreference = 'Stop'

# The peer id version alphabet. libtorrent, libtorrent-rakshasa and
# Transmission all encode one version component as one character: 0 to 9 then
# A to Z then a to z. Transmission calls the same table BASE62.
$VersionAlphabet = '0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz'

function ConvertTo-VersionChar {
    param([int]$Value)
    if ($Value -lt 0 -or $Value -ge $VersionAlphabet.Length) {
        throw "version component $Value has no single-character encoding"
    }
    return $VersionAlphabet[$Value]
}

function Get-VersionParts {
    param([string]$Text)
    $parts = $Text -split '\.'
    if ($parts.Count -lt 3) {
        throw "version '$Text' does not carry three components"
    }
    return @([int]$parts[0], [int]$parts[1], [int]$parts[2])
}

function Invoke-SelfTest {
    $failures = @()

    # The whole alphabet round trips, which is the property the ports lost.
    for ($i = 0; $i -lt $VersionAlphabet.Length; $i++) {
        if ((ConvertTo-VersionChar -Value $i) -ne $VersionAlphabet[$i]) {
            $failures += "alphabet index $i does not round trip"
        }
    }

    # A component of ten or more is one character, not two. joal's qBittorrent
    # script concatenates decimal, so 3.3.13 becomes -qB33130- and the prefix
    # is nine bytes. The real one is -qB33D0-.
    $cases = @(
        @{ v = '5.2.3';  code = 'qB'; want = '-qB5230-' },
        @{ v = '3.3.13'; code = 'qB'; want = '-qB33D0-' },
        @{ v = '3.3.16'; code = 'qB'; want = '-qB33G0-' },
        @{ v = '4.1.3';  code = 'TR'; want = '-TR4130-' },
        @{ v = '3.0.0';  code = 'TR'; want = '-TR3000-' }
    )
    foreach ($case in $cases) {
        $p = Get-VersionParts -Text $case.v
        $got = '-{0}{1}{2}{3}0-' -f $case.code,
            (ConvertTo-VersionChar $p[0]),
            (ConvertTo-VersionChar $p[1]),
            (ConvertTo-VersionChar $p[2])
        if ($got -ne $case.want) {
            $failures += "$($case.code) $($case.v): got $got, want $($case.want)"
        }
        if ($got.Length -ne 8) {
            $failures += "$($case.code) $($case.v): prefix is $($got.Length) bytes, not 8"
        }
    }

    # A key must be able to start with a zero. Every profile set read for T-234
    # guarantees it cannot, and libtorrent writes key=%08X.
    $sawLeadingZero = $false
    for ($i = 0; $i -lt 4096; $i++) {
        if ((New-LibtorrentKey).StartsWith('0')) { $sawLeadingZero = $true; break }
    }
    if (-not $sawLeadingZero) {
        $failures += "New-LibtorrentKey never produced a key with a leading zero in 4096 draws"
    }
    foreach ($i in 1..64) {
        $k = New-LibtorrentKey
        if ($k.Length -ne 8) { $failures += "key '$k' is not 8 characters"; break }
        if ($k -cmatch '[a-f]') { $failures += "key '$k' is not upper case"; break }
    }

    # Transmission's checksum digit makes the suffix sum a multiple of the base.
    foreach ($i in 1..64) {
        $id = New-TransmissionPeerId -Prefix '-TR4130-'
        if ($id.Length -ne 20) { $failures += "peer id '$id' is not 20 bytes"; break }
        $pool = '0123456789abcdefghijklmnopqrstuvwxyz'
        $total = 0
        foreach ($c in $id.Substring(8).ToCharArray()) { $total += $pool.IndexOf($c) }
        if (($total % 36) -ne 0) {
            $failures += "peer id '$id' suffix sums to $($total % 36) mod 36, not 0"
            break
        }
    }

    if ($failures.Count -gt 0) {
        foreach ($f in $failures) { Write-Host "  fail: $f" }
        Write-Host "make-client-profile: $($failures.Count) self-test failure(s)"
        return 1
    }
    Write-Host "make-client-profile: self-test passes"
    return 0
}

function New-LibtorrentKey {
    # libtorrent v2.0.11 src/http_tracker_connection.cpp:138 writes "&key=%08X",
    # so the value is a 32 bit integer in upper case hex, zero padded to eight,
    # and a leading zero is ordinary rather than impossible.
    $bytes = New-Object byte[] 4
    [System.Security.Cryptography.RandomNumberGenerator]::Fill($bytes)
    $value = [System.BitConverter]::ToUInt32($bytes, 0)
    return $value.ToString('X8')
}

function New-TransmissionPeerId {
    # Transmission 4.1.0 libtransmission/session.cc:196-206. Eleven characters
    # drawn from the pool, then one checksum character chosen so the whole
    # suffix sums to a multiple of the base.
    param([string]$Prefix)
    $pool = '0123456789abcdefghijklmnopqrstuvwxyz'
    $base = $pool.Length
    $suffixLength = 20 - $Prefix.Length
    $bytes = New-Object byte[] ($suffixLength - 1)
    [System.Security.Cryptography.RandomNumberGenerator]::Fill($bytes)
    $sb = New-Object System.Text.StringBuilder
    $total = 0
    foreach ($b in $bytes) {
        $v = [int]$b % $base
        $total += $v
        [void]$sb.Append($pool[$v])
    }
    $check = if (($total % $base) -ne 0) { $base - ($total % $base) } else { 0 }
    [void]$sb.Append($pool[$check])
    return $Prefix + $sb.ToString()
}

function Get-RepoFile {
    param([string]$Repo, [string]$Path, [string]$Ref)
    $raw = & gh api "repos/$Repo/contents/$Path`?ref=$Ref" --jq '.content' 2>$null
    if ($LASTEXITCODE -ne 0 -or -not $raw) { return $null }
    $joined = ($raw -join '') -replace '\s', ''
    try {
        return [System.Text.Encoding]::UTF8.GetString([System.Convert]::FromBase64String($joined))
    } catch {
        return $null
    }
}

# ---------------------------------------------------------------------------

if ($SelfTest) { exit (Invoke-SelfTest) }

if (-not (Get-Command gh -ErrorAction SilentlyContinue)) {
    Write-Host "make-client-profile: gh is not on PATH, so no source can be read"
    exit 2
}

$guardFailures = @()
$profile = $null

if ($Client -eq 'qbittorrent') {
    if (-not $Version) { $Version = '5.2.3' }
    $repo = 'qbittorrent/qBittorrent'
    $ref = "release-$Version"

    $versionFile = Get-RepoFile -Repo $repo -Path 'src/base/version.h.in' -Ref $ref
    if ($null -eq $versionFile) {
        Write-Host "make-client-profile: cannot read src/base/version.h.in at $ref"
        exit 2
    }

    # Read the version from the client rather than trusting the tag. joal's
    # script extracts these and then ignores them; here a disagreement is a
    # guard failure, because the tag and the constants naming different
    # versions is exactly the case a profile must not paper over.
    $constants = @{}
    foreach ($name in 'QBT_VERSION_MAJOR', 'QBT_VERSION_MINOR', 'QBT_VERSION_BUGFIX') {
        $m = [regex]::Match($versionFile, "(?m)^#define\s+$name\s+(\d+)")
        if (-not $m.Success) { $guardFailures += "$name is not in src/base/version.h.in at $ref" }
        else { $constants[$name] = [int]$m.Groups[1].Value }
    }

    if ($guardFailures.Count -eq 0) {
        $declared = '{0}.{1}.{2}' -f $constants['QBT_VERSION_MAJOR'], $constants['QBT_VERSION_MINOR'], $constants['QBT_VERSION_BUGFIX']
        if ($declared -ne $Version) {
            $guardFailures += "tag $ref carries version constants $declared"
        }
    }

    # qBittorrent moved the session implementation between 4.x and 5.x. Read
    # whichever exists and record which one answered.
    $sessionPath = $null
    $sessionText = $null
    foreach ($candidate in 'src/base/bittorrent/sessionimpl.cpp', 'src/base/bittorrent/session.cpp') {
        $text = Get-RepoFile -Repo $repo -Path $candidate -Ref $ref
        if ($null -ne $text) { $sessionPath = $candidate; $sessionText = $text; break }
    }
    if ($null -eq $sessionText) {
        $guardFailures += "no session implementation at either known path in $ref"
    } else {
        # The guard that matters: qBittorrent still builds its fingerprint from
        # the version constants through libtorrent, and still builds its
        # User-Agent from the same version string.
        if ($sessionText -notmatch 'peer_fingerprint|generate_fingerprint|PEER_ID') {
            $guardFailures += "$sessionPath no longer mentions a peer fingerprint"
        }
        if ($sessionText -notmatch 'USER_AGENT|user_agent') {
            $guardFailures += "$sessionPath no longer mentions a user agent"
        }
    }

    if ($guardFailures.Count -eq 0) {
        $p = Get-VersionParts -Text $declared
        $prefix = '-qB{0}{1}{2}0-' -f (ConvertTo-VersionChar $p[0]), (ConvertTo-VersionChar $p[1]), (ConvertTo-VersionChar $p[2])
        $profile = [ordered]@{
            name    = "qbittorrent-$declared"
            client  = 'qBittorrent'
            version = $declared
            derived_from = [ordered]@{
                repo    = $repo
                ref     = $ref
                files   = @('src/base/version.h.in', $sessionPath)
                engine  = 'libtorrent'
            }
            peer_id = [ordered]@{
                style   = 'azureus'
                prefix  = $prefix
                suffix  = [ordered]@{
                    kind    = 'charset'
                    charset = 'A-Za-z0-9_~()!.*-'
                    length  = 12
                }
                refresh = 'never'
            }
            tracker_http = [ordered]@{
                user_agent  = "qBittorrent/$declared"
                headers     = @('User-Agent', 'Accept-Encoding: gzip', 'Connection: close')
                query_order = @('info_hash', 'peer_id', 'port', 'uploaded', 'downloaded',
                                'left', 'corrupt', 'key', 'event', 'numwant', 'compact',
                                'no_peer_id', 'supportcrypto', 'redundant')
                key         = [ordered]@{
                    width          = 8
                    case           = 'upper'
                    leading_zero   = $true
                    refresh        = 'per_torrent'
                    source         = 'libtorrent src/http_tracker_connection.cpp, key=%08X'
                }
                numwant         = 200
                numwant_on_stop = 0
                encoder         = [ordered]@{
                    unreserved = 'A-Za-z0-9_~()!.*-'
                    hex_case   = 'lower'
                }
            }
            peer_wire = [ordered]@{
                note = 'not derived by this run: reserved bytes, the extension handshake and the message order after the handshake are read from a live client, not from a tag'
            }
        }
    }
}
elseif ($Client -eq 'transmission') {
    if (-not $Version) { $Version = '4.1.3' }
    $repo = 'transmission/transmission'
    $ref = $Version

    $cmake = Get-RepoFile -Repo $repo -Path 'CMakeLists.txt' -Ref $ref
    if ($null -eq $cmake) {
        Write-Host "make-client-profile: cannot read CMakeLists.txt at $ref"
        exit 2
    }

    $constants = @{}
    foreach ($name in 'TR_VERSION_MAJOR', 'TR_VERSION_MINOR', 'TR_VERSION_PATCH') {
        $m = [regex]::Match($cmake, "set\($name\s+`"(\d+)`"\)")
        if (-not $m.Success) { $guardFailures += "$name is not in CMakeLists.txt at $ref" }
        else { $constants[$name] = [int]$m.Groups[1].Value }
    }
    if ($cmake -notmatch 'set\(TR_SEMVER\s+"\$\{TR_VERSION_MAJOR\}\.\$\{TR_VERSION_MINOR\}\.\$\{TR_VERSION_PATCH\}"\)') {
        $guardFailures += 'TR_SEMVER is no longer major.minor.patch, so the User-Agent is no longer derivable from these three'
    }

    if ($guardFailures.Count -eq 0) {
        $declared = '{0}.{1}.{2}' -f $constants['TR_VERSION_MAJOR'], $constants['TR_VERSION_MINOR'], $constants['TR_VERSION_PATCH']
        if ($declared -ne $Version) {
            $guardFailures += "tag $ref carries version constants $declared"
        }
    }

    $session = Get-RepoFile -Repo $repo -Path 'libtransmission/session.cc' -Ref $ref
    if ($null -eq $session) {
        $guardFailures += 'libtransmission/session.cc is not at its known path'
    } else {
        # The checksum is the guard. Three of the four emulators read for T-234
        # get this wrong, and a tracker that validates it sees the difference.
        if ($session -notmatch '0123456789abcdefghijklmnopqrstuvwxyz') {
            $guardFailures += 'session.cc no longer carries the base 36 peer id pool'
        }
        if ($session -notmatch 'total\s*%\s*std::size\(Pool\)') {
            $guardFailures += 'session.cc no longer computes the peer id checksum the way this script reproduces'
        }
    }

    if ($guardFailures.Count -eq 0) {
        $p = Get-VersionParts -Text $declared
        $prefix = '-TR{0}{1}{2}0-' -f (ConvertTo-VersionChar $p[0]), (ConvertTo-VersionChar $p[1]), (ConvertTo-VersionChar $p[2])
        $profile = [ordered]@{
            name    = "transmission-$declared"
            client  = 'Transmission'
            version = $declared
            derived_from = [ordered]@{
                repo   = $repo
                ref    = $ref
                files  = @('CMakeLists.txt', 'libtransmission/session.cc')
                engine = 'libtransmission'
            }
            peer_id = [ordered]@{
                style   = 'azureus'
                prefix  = $prefix
                suffix  = [ordered]@{
                    kind      = 'pool_with_checksum'
                    pool      = '0123456789abcdefghijklmnopqrstuvwxyz'
                    base      = 36
                    length    = 12
                    checksum  = 'the whole suffix sums to a multiple of the base'
                }
                refresh = 'per_session'
                sample  = (New-TransmissionPeerId -Prefix $prefix)
            }
            tracker_http = [ordered]@{
                user_agent  = "Transmission/$declared"
                headers     = @('User-Agent', 'Accept: */*', 'Accept-Encoding: deflate, gzip')
                query_order = @('info_hash', 'peer_id', 'port', 'uploaded', 'downloaded',
                                'left', 'numwant', 'key', 'compact', 'supportcrypto',
                                'event', 'ipv6')
                key         = [ordered]@{
                    width        = 'variable'
                    case         = 'lower'
                    leading_zero = $false
                    refresh      = 'never'
                    source       = 'libtransmission announce_key, an integer rendered as hex'
                }
                numwant         = 80
                numwant_on_stop = 0
                encoder         = [ordered]@{
                    unreserved = 'A-Za-z0-9-'
                    hex_case   = 'lower'
                }
            }
            peer_wire = [ordered]@{
                note = 'not derived by this run: reserved bytes, the extension handshake and the message order after the handshake are read from a live client, not from a tag'
            }
        }
    }
}

if ($guardFailures.Count -gt 0) {
    if ($Json) {
        [ordered]@{ ok = $false; client = $Client; version = $Version; guard_failures = $guardFailures } |
            ConvertTo-Json -Depth 6
    } else {
        Write-Host "make-client-profile: $Client $Version could not be derived"
        foreach ($f in $guardFailures) { Write-Host "  guard: $f" }
        Write-Host "  nothing was written. The client changed how it builds its identity,"
        Write-Host "  or the tag does not exist. Read the files named above before editing this script."
    }
    exit 1
}

$text = ($profile | ConvertTo-Json -Depth 8)

if ($Out) {
    $dir = Split-Path -Parent $Out
    if ($dir -and -not (Test-Path $dir)) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
    [System.IO.File]::WriteAllText($Out, $text + "`n", (New-Object System.Text.UTF8Encoding $false))
    if (-not $Json) { Write-Host "make-client-profile: wrote $Out" }
}

if ($Json -or -not $Out) { $text }

exit 0
