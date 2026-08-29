# Measuring in a throwaway container

Some things this repository has to measure cannot be measured on the machine
doing the measuring. A browser newer than the installed one, a
filesystem this host does not have, a libc this host does not use: each needs a
different machine, and waiting for CI to be that machine costs five minutes a
question.

A throwaway WSL2 distro is that machine, and it leaves nothing behind. Creating
one from `debian:bookworm-slim` and running a command in it is seconds; what
takes time is whatever the command does, which for installing a browser was a
few minutes.

**This page is a procedure, not a dependency.** Nothing in `scripts/` requires
a container to run, and no gate does. A check that can use one says so and
exits **2** when there is none, the same way
[`../scripts/check-browser-fingerprint.ps1`](../scripts/check-browser-fingerprint.ps1)
exits 2 when there is no browser. A machine with no container engine is not a
failing build.

## What is on this machine

| | |
| --- | --- |
| engine | podman 5.8.6, with a `podman-machine-default` WSL distro |
| distro tool | `wsl-ephemeral.ps1`, from `Azathothas/ToolKit` |
| networking | WSL2 in **NAT** mode, set deliberately in `.wslconfig` |

## Getting the tool, and the one rule about it

**Pin a commit, never a branch.** `main` moves, and a moved reference runs code
nobody reviewed.

```bash
gh api repos/Azathothas/ToolKit/commits/main --jq .sha
```

Fetch that exact revision to a file and run the file. Do not pipe a download
into a shell: a truncated transfer executes the prefix and leaves nothing to
inspect.

```bash
curl -sSL "https://raw.githubusercontent.com/Azathothas/ToolKit/<SHA>/scripts/powershell-windows/wsl-ephemeral.ps1" -o .tmp/wsl-ephemeral.ps1
```

```bash
pwsh -NoProfile -File .tmp/wsl-ephemeral.ps1 -Action List
```

`List` is the first thing to run and the last. It reports every distro the tool
made, every distro it will never touch, and any rootfs tarball a cancelled run
orphaned.

## Running something in one

```bash
pwsh -NoProfile -File .tmp/wsl-ephemeral.ps1 -Action New -Image debian:bookworm-slim -Name eph-bitcli-x -CommandB64 "$(base64 -w0 < script.sh)"
```

**Pass the command as `-CommandB64`.** It is not a preference. A command sent
as text is parsed by PowerShell before it reaches the distro: `$VAR` expands in
transit, a backtick opens a command substitution, and Windows PowerShell 5.1
drops a double quote out of a child process's argument list before the script
ever sees it. Base64 has no character any shell touches.

**Write the script with LF endings.** `-CommandFile` is read verbatim, so a
CRLF file makes `/bin/sh` read the carriage return as part of the last word on
every line.

**`/bin/sh` is dash on Debian.** `/dev/tcp` is a bash builtin and is not there;
call `bash -c` when you need it. `ip` is not in `-slim` images either; the
default gateway is in `/proc/net/route`.

## Reaching this host from inside one

WSL is in **NAT** mode here, so a distro does **not** reach the Windows
loopback. `localhost` inside the distro is the distro.

What it does reach is the host's WSL adapter address. Measured on this machine:

| | |
| --- | --- |
| gateway seen by the distro | `172.23.96.1`, from `/proc/net/route` |
| a listener bound there | accepts the connection |
| `127.0.0.1` on the host | not reachable |

That address is a Hyper-V internal network and is not reachable from the LAN,
which is why it is the right thing to bind a fixture to and `0.0.0.0` is not.
**Read it rather than writing it down**: it is assigned by WSL and the value
above is what this machine had on the day it was measured, not a constant.

```bash
awk '$2 == "00000000" { print $3 }' /proc/net/route
```

That is little-endian hex: `016017AC` is `172.23.96.1`.

## Getting a browser into one

Three ways, and the first is the one to reach for. The versions below are what
they gave when this was written; read them again rather than trusting them.

| source | version it gave | what it is |
| --- | --- | --- |
| **Chrome for Testing** | `Stable` 152.0.7977.64, `Beta` 153.0.8010.12, and `Dev` and `Canary` beyond | Google's own per-channel index of the builds it publishes for automation, with a download URL per platform |
| `debian:bookworm-slim` plus Google's apt repository | 152.0.7977.64 | whatever the distribution channel is serving |
| a third-party image | theirs | `selenium/standalone-chrome` and `mcr.microsoft.com/playwright` both ship a Chrome, and both pick the version |

```bash
curl -sS https://googlechromelabs.github.io/chrome-for-testing/last-known-good-versions.json
```

**Chrome for Testing is addressable by channel**, which is the property the
other two do not have: it reaches Beta, Dev and Canary as well as Stable, so a
profile can be captured **before** a version ships rather than after. A
distribution package gives whatever it gives.

**Download it into the distro, never onto the host.** Installing a browser on
somebody's machine is a system change nobody asked for; installing it into a
distro that is destroyed afterwards is not.

## Decommissioning, which is not optional

Every distro is removed in the same run that made it, and the run checks rather
than assuming.

```bash
pwsh -NoProfile -File .tmp/wsl-ephemeral.ps1 -Action Remove -Name eph-bitcli-x -Force
```

```bash
pwsh -NoProfile -File .tmp/wsl-ephemeral.ps1 -Action List
```

`-Ephemeral` on `New` does both in one call and destroys the distro even when
the command fails. Use it when the distro is wanted for exactly one command.

**A cancelled run can leave a rootfs tarball of several hundred MiB** in
`%LOCALAPPDATA%\wsl-ephemeral\`, because the cleanup is a `finally` and a hard
interrupt does not run one. `List` reports each with its size and the time it
was written; `Purge -Force` removes them. Read the time first: a `New` that is
running right now has its tarball in the same directory and nothing can tell
the two apart.

## What the tool will not do

The removal path is constrained four ways and every destructive call goes
through all of them: a fixed `eph-` prefix, a refusal for any name without it,
a protected list that includes `podman-machine-default` and the Docker and
Rancher distros, and a directory deletion confined to one base directory. A
mistake here cannot destroy the container runtime.

## Images cost more disk than they look like they do

The tool's own page measures an 8 MiB Alpine rootfs becoming a 76 MiB VHDX, and
a 74 MiB Debian one becoming 172 MiB: the cost is dominated by a fixed floor
rather than by a multiple of the input. It refuses an import it cannot fit
rather than leaving a half-written disk and a registered distro that does not
work. Those figures are its author's and the refusal has not been triggered
here; what was measured here is the removal, which left no distro and no
orphaned tarball.

## Leaving the engine as it was found

A session that pulls an image or creates a volume removes it. The engine is
shared with everything else on the machine and a session's leftovers are
somebody else's disk.

```bash
podman system df
```

That is the one number to read before finishing: `RECLAIMABLE` at 100 percent
of a large `SIZE` means something stopped cleaning up after itself.
