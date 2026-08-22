# Do the committed manuals still describe the binary?
#
# `man/bit-cli.1`, `man/bit-cli.json` and `man/bit-cli.md` are generated from the clap
# definition
# by `bit-cli man`, and both are committed. A committed generated file is only
# worth having if something fails when it goes stale, so this regenerates both
# into a temporary directory and compares. Renaming a flag without regenerating
# fails the gates.
#
# Why the JSON exists at all: an agent driving this tool reads roff badly, so it
# guesses flag names, and a guessed flag is a run that fails on exit 2 or one
# that quietly does something else. `man/bit-cli.json` is the same surface in a
# shape a program can index, as a CLIspec 0.3 document. See `docs/man.md`.
#
# Usage:
#   pwsh scripts/check-man.ps1           # fail on drift
#   pwsh scripts/check-man.ps1 -Fix      # regenerate the committed copies
#
# Exits 0 when all three match, 1 when any has drifted, and 2 when the check could
# not run.
#
# The gate that actually holds this is `cargo test -p bit-cli --test man_is_current`,
# which renders from the crate being compiled rather than from a binary that may
# be older, and which runs on every platform CI builds. This script is how the
# files are regenerated, and a standalone check for a person.

[CmdletBinding()]
param(
    [switch]$Fix,
    [ValidateSet("debug", "release")]
    [string]$Profile = "release",
    [string]$ManDir = "man"
)

$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot

function Exit-With([int]$code, [string]$message) {
    [Console]::Error.WriteLine("check-man: $message")
    exit $code
}

$exe = Join-Path $repo "target/$Profile/bit-cli.exe"
if (-not (Test-Path $exe)) { $exe = Join-Path $repo "target/$Profile/bit-cli" }
if (-not (Test-Path $exe)) {
    Exit-With 2 "no bit-cli binary at target/$Profile. Run: cargo build --profile $Profile --bins"
}

$label = $ManDir
$manDir = Join-Path $repo $ManDir
New-Item -ItemType Directory -Force -Path $manDir | Out-Null

$work = Join-Path ([System.IO.Path]::GetTempPath()) "bit-cli-man-$(Get-Random)"
New-Item -ItemType Directory -Force -Path $work | Out-Null

try {
    $targets = @(
        @{ Name = "bit-cli.1"; Arguments = @("man") },
        @{ Name = "bit-cli.json"; Arguments = @("man", "--format", "json") },
        @{ Name = "bit-cli.md"; Arguments = @("man", "--format", "markdown") }
    )

    $drifted = @()
    foreach ($target in $targets) {
        $fresh = Join-Path $work $target.Name
        $committed = Join-Path $manDir $target.Name

        # Through Start-Process with redirect files, like every other check
        # script: whether a line on stderr ends the run otherwise depends on
        # the host's pwsh version. See TODO/windows.md under T-075.
        $err = Join-Path $work "$($target.Name).err"
        $proc = Start-Process -FilePath $exe -PassThru -NoNewWindow `
            -ArgumentList ($target.Arguments + @("--output", $fresh)) `
            -RedirectStandardOutput (Join-Path $work "$($target.Name).out") `
            -RedirectStandardError $err
        $proc.WaitForExit(120000) | Out-Null
        if ($proc.ExitCode -ne 0) {
            Exit-With 2 "bit-cli $($target.Arguments -join ' ') exited $($proc.ExitCode): $(Get-Content $err -Raw)"
        }
        if (-not (Test-Path $fresh)) {
            Exit-With 2 "bit-cli $($target.Arguments -join ' ') wrote nothing"
        }

        if ($Fix) {
            Copy-Item -LiteralPath $fresh -Destination $committed -Force
            Write-Host "check-man: wrote $label/$($target.Name)"
            continue
        }

        if (-not (Test-Path $committed)) {
            $drifted += "$label/$($target.Name) is missing"
            continue
        }

        # Compared as bytes. A line ending that changed is drift too: these are
        # committed files and a diff nobody can read is a diff nobody reviews.
        $a = [System.IO.File]::ReadAllBytes($fresh)
        $b = [System.IO.File]::ReadAllBytes($committed)
        if ($a.Length -ne $b.Length -or [System.Convert]::ToBase64String($a) -ne [System.Convert]::ToBase64String($b)) {
            $firstDiff = $null
            $limit = [math]::Min($a.Length, $b.Length)
            for ($i = 0; $i -lt $limit; $i++) {
                if ($a[$i] -ne $b[$i]) { $firstDiff = $i; break }
            }
            if ($null -eq $firstDiff) { $firstDiff = $limit }
            $drifted += "$label/$($target.Name) is stale: committed $($b.Length) bytes, generated $($a.Length), first difference at byte $firstDiff"
        }
    }

    if ($Fix) {
        Write-Host "check-man: regenerated"
        exit 0
    }

    if ($drifted.Count -gt 0) {
        foreach ($line in $drifted) { [Console]::Error.WriteLine("check-man: $line") }
        Exit-With 1 "run: pwsh -NoProfile -File scripts/check-man.ps1 -Fix"
    }

    Write-Host "check-man: bit-cli.1, bit-cli.json and bit-cli.md describe the binary"
    exit 0
}
finally {
    Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue
}
