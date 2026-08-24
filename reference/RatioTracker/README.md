<div align="center">

```
__________         __  .__     ___________                     __                 
\______   \_____ _/  |_|__| ___\__    ___/___________    ____ |  | __ ___________ 
 |       _/\__  \\   __\  |/  _ \|    |  \_  __ \__  \ _/ ___\|  |/ // __ \_  __ \
 |    |   \ / __ \|  | |  (  <_> )    |   |  | \// __ \\  \___|    <\  ___/|  | \/
 |____|_  /(____  /__| |__|\____/|____|   |__|  (____  /\___  >__|_ \\___  >__|   
        \/      \/                                   \/     \/     \/    \/       
```

**RatioTracker is a command-line security auditing tool that checks whether a BitTorrent tracker correctly validates announce data and enforces ratio integrity against actual transfer activity.**

[![Python](https://img.shields.io/badge/Python-3.11+-3776AB?style=flat-square&logo=python&logoColor=white)](https://python.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-22c55e?style=flat-square)](LICENSE)
[![Security Testing](https://img.shields.io/badge/Purpose-Security%20Testing-ef4444?style=flat-square)]()
[![Authorized Use Only](https://img.shields.io/badge/Use-Authorized%20Only-f59e0b?style=flat-square)]()

</div>

---

## Quick Start

```bash
pip install -r requirements.txt

# Run the full test suite
python ratiotracker.py file.torrent

# Dry run — preview without sending any requests
python ratiotracker.py file.torrent --dry-run -v
```

---

## Test Suite

Eight tests designed to probe common tracker validation failures. Every test is **self-contained** — it registers its own peer (`started`), runs the attack, and cleans up (`stopped`), so each test works reliably both alone (`-t 2`) and as part of the full suite.

| # | Test | What it checks | Key option |
|:-:|------|----------------|------------|
| `1` | **Fake seed** | Announces `left=0` without ever having downloaded | — |
| `2` | **Inflated upload** | Claims uploaded bytes with no actual transfer | `--upload` |
| `3` | **Negative DL** | Sends negative `downloaded` value to reduce ratio debt | `--negative-dl` |
| `4` | **Size bomb** | Single announce claiming a massive upload instantly | `--bomb-size` |
| `5` | **Impossible speed** | Two rapid announces implying physically unrealistic speeds | `--speed-amount` |
| `6` | **Multi peer IDs** | Multiple fake peers seeding the same torrent simultaneously | `--multi-seed-count`, `--multi-seed-upload` |
| `7` | **Stop event** | Clean `stopped` announce sequence | `--upload` |
| `8` | **Simulation** | Realistic gradual transfer with fluctuating speeds | `--sim-*` options |

```bash
# Target specific tests
python ratiotracker.py file.torrent -t 1 2 5

# List all available tests
python ratiotracker.py --list-tests
```

---

## Realistic Simulation (Test 8)

The simulation engine models a real BitTorrent session — not just a static announce, but a full transfer lifecycle with organic speed variations.

```
Timeline ──────────────────────────────────────────────────────────────────>
         |  Ramp-up  |           Cruising            |  Taper  |
Speed    |  ↗↗↗↗↗↗  |  ~~~~~~~~~~~~~~~~~~~▼~~~▼▼~~~~  |  ↘↘↘↘  |
         └──────────────────────────────────────────────────────
```

| Phase | Share | Behavior |
|-------|------:|---------|
| Ramp-up | ~10% | Speed climbs progressively (peer discovery) |
| Cruising | ~82% | Stable speed with Gaussian jitter |
| Micro-drop | 12% chance | 20-60% speed reduction (congestion) |
| Big drop | 3% chance | 70-95% speed reduction (peer disconnect) |
| Taper | ~8% | Gradual slowdown as fewer pieces remain |

```bash
# 10 GB over 30 minutes (default)
python ratiotracker.py file.torrent -t 8

# 1 TB over 1 hour, announce every 30s
python ratiotracker.py file.torrent -t 8 --sim-amount 1TB --sim-duration 1h --sim-interval 30

# 50 GB over 2 hours with high variance
python ratiotracker.py file.torrent -t 8 --sim-amount 50GB --sim-duration 2h --sim-jitter 0.5
```

### Simulation options

| Option | Default | Description |
|--------|:-------:|-------------|
| `--sim-amount` | `10GB` | Total data volume to simulate |
| `--sim-duration` | `30m` | Total duration of the session |
| `--sim-interval` | `30` | Seconds between each announce |
| `--sim-jitter` | `0.25` | Speed variance (`0.0` = constant, `0.5` = chaotic) |

---

## Announce Parameters

Every value is fully configurable — nothing is hardcoded:

| Option | Default | Used by | Description |
|--------|:-------:|:-------:|-------------|
| `-u`, `--upload` | `1GB` | Tests 2, 7 | Upload amount to claim |
| `-d`, `--download` | `0` | All | Download amount to claim |
| `--left` | `0` | All | Bytes remaining (0 = pretend complete) |
| `--bomb-size` | `100GB` | Test 4 | Upload claimed in the single-announce bomb |
| `--negative-dl` | `1GB` | Test 3 | Absolute value sent as negative downloaded |
| `--speed-amount` | `50GB` | Test 5 | Upload claimed after minimal delay |
| `--multi-seed-upload` | `1GB` | Test 6 | Upload claimed per fake peer |
| `--multi-seed-count` | `3` | Test 6 | Number of fake peers to spawn |

```bash
# Custom values for specific tests
python ratiotracker.py file.torrent -t 3 --negative-dl 5GB
python ratiotracker.py file.torrent -t 4 --bomb-size 1TB
python ratiotracker.py file.torrent -t 5 --speed-amount 100GB
python ratiotracker.py file.torrent -t 6 --multi-seed-count 5 --multi-seed-upload 10GB
```

---

## Client Emulation

Spoof any major BitTorrent client's peer ID and User-Agent:

```bash
python ratiotracker.py file.torrent -c qbit
python ratiotracker.py file.torrent -c deluge
```

| Client | User-Agent | Peer ID prefix |
|--------|-----------|:--------------:|
| `utorrent` | uTorrent/3320 | `-UT3320-` |
| `qbit` | qBittorrent/4.7.1 | `-qB4710-` |
| `deluge` | Deluge/2.1.1 | `-DE2110-` |
| `transmission` | Transmission/4.0.4 | `-TR4040-` |
| `libtorrent` | libtorrent/2.0.3 | `-LT2030-` |

```bash
# Fully custom client identity
python ratiotracker.py file.torrent --user-agent "MyClient/1.0" --peer-id-prefix "-MC1000-"
```

---

## Output & Reporting

```bash
# JSON output
python ratiotracker.py file.torrent --json

# Save full report to file
python ratiotracker.py file.torrent --json -o report.json

# Quiet mode — verdict only
python ratiotracker.py file.torrent -q

# Verbose debug logging
python ratiotracker.py file.torrent -vv --log-file debug.log
```

---

## Advanced Usage

```bash
# Repeat the suite 5 times with a 30s gap between cycles
python ratiotracker.py file.torrent --cycles 5 --delay 30

# Lock to a specific port
python ratiotracker.py file.torrent -p 51413
```

### Size & Duration formats

| Type | Examples |
|------|---------|
| Size | `1024` (bytes) - `500KB` - `100MB` - `10GB` - `1TB` |
| Duration | `90s` - `30m` - `1h` |

---

## Exit Codes

| Code | Meaning |
|:----:|---------|
| `0` | At least one test passed undetected by the tracker |
| `1` | All tests were detected and rejected |

---

## Disclaimer

This tool is intended for **authorized security testing only**.
Only use it on trackers you own or have **explicit written permission** to test.
Unauthorized use may violate terms of service and applicable laws.

---

<div align="center">

MIT License - Built for tracker operators and security researchers

</div>
