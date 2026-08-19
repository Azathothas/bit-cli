# Check that a Windows build has no C runtime dependency.
#
# A statically linked bit-cli runs on a machine with no Visual C++
# redistributable installed. That is the whole point of +crt-static in
# .cargo/config.toml, and it is easy to lose: one dependency that links the
# dynamic CRT and the binary starts asking for VCRUNTIME140.dll on a machine
# that does not have it, which fails at process start with a dialog box rather
# than an error a script can read.
#
# Usage:
#   pwsh scripts/check-static.ps1
#   pwsh scripts/check-static.ps1 -Path target/release/bit-cli.exe
#
# Exits 0 when the import table is clean, 1 when it is not, and 2 when the
# check could not run.

[CmdletBinding()]
param(
    [string]$Path = "target/x86_64-pc-windows-msvc/release/bit-cli.exe"
)

$ErrorActionPreference = 'Stop'

# Write-Error is a terminating error under `Stop`, so a `Write-Error` followed
# by `exit 2` never reaches the exit and the caller sees 1. The exit codes in
# the header above are the contract, so failures go out this way instead.
function Exit-With([int]$code, [string]$message) {
    [Console]::Error.WriteLine("check-static: $message")
    exit $code
}

# Resolve a relative path against the repository root rather than the caller's
# working directory, so the script works from anywhere and from CI.
if (-not [System.IO.Path]::IsPathRooted($Path)) {
    $root = Split-Path -Parent $PSScriptRoot
    $Path = Join-Path $root $Path
}

if (-not (Test-Path $Path)) {
    Exit-With 2 "no binary at $Path. Build it first: cargo build --release --locked --target x86_64-pc-windows-msvc"
}

# dumpbin ships with the MSVC build tools and is not on PATH by default.
$dumpbin = Get-ChildItem -Path @(
    "${env:ProgramFiles(x86)}\Microsoft Visual Studio\*\*\VC\Tools\MSVC\*\bin\Hostx64\x64\dumpbin.exe",
    "${env:ProgramFiles}\Microsoft Visual Studio\*\*\VC\Tools\MSVC\*\bin\Hostx64\x64\dumpbin.exe"
) -ErrorAction SilentlyContinue | Select-Object -Last 1

if (-not $dumpbin) {
    Exit-With 2 "dumpbin not found. Install the Visual Studio build tools, or run this on a machine that has them."
}

$imports = & $dumpbin.FullName /dependents $Path |
    Select-String -Pattern '^\s+(\S+\.dll)\s*$' |
    ForEach-Object { $_.Matches[0].Groups[1].Value }

if (-not $imports) {
    Exit-With 2 "dumpbin reported no imports for $Path, which cannot be right"
}

# The C runtime, in every spelling that means "not statically linked".
# api-ms-win-crt-* are the CRT api-sets; api-ms-win-core-* are core OS
# api-sets and are fine.
$forbidden = $imports | Where-Object {
    $_ -match '^(vcruntime|msvcp|msvcr|ucrtbase|concrt)' -or $_ -match '^api-ms-win-crt-'
}

Write-Output "binary:  $Path"
Write-Output "size:    $((Get-Item $Path).Length) bytes"
Write-Output "imports:"
$imports | ForEach-Object { Write-Output "  $_" }

if ($forbidden) {
    Write-Output ""
    Exit-With 1 "the binary depends on the dynamic C runtime: $($forbidden -join ', ')"
}

Write-Output ""
Write-Output "static CRT confirmed: no VCRUNTIME, MSVCP, UCRT, or api-ms-win-crt-* import"
exit 0
