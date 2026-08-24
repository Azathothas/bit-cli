#!/usr/bin/env python3
"""
RatioTracker - Ratio Manipulation Detection Test
Tracker: UNIT3D
Purpose: Verify if tracker validates announce data against actual transfer activity

Usage examples:
  python ratiotracker.py test.torrent
  python ratiotracker.py test.torrent --upload 5GB --cycles 3 --delay 10
  python ratiotracker.py test.torrent --client deluge --tests 1 2 --json
  python ratiotracker.py test.torrent -t 8 --sim-amount 50GB --sim-duration 2h --sim-interval 60
  python ratiotracker.py test.torrent --dry-run -v
"""

import argparse
import hashlib
import json
import logging
import random
import string
import sys
import time
import urllib.parse
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Optional

import bencodepy
import requests

__version__ = "1.1.0"

# ---------------------------------------------------------------------------
# Client profiles
# ---------------------------------------------------------------------------
CLIENT_PROFILES = {
    "utorrent":  {"prefix": "-UT3320-", "user_agent": "uTorrent/3320"},
    "qbit":      {"prefix": "-qB4710-", "user_agent": "qBittorrent/4.7.1"},
    "deluge":    {"prefix": "-DE2110-", "user_agent": "Deluge/2.1.1"},
    "transmission": {"prefix": "-TR4040-", "user_agent": "Transmission/4.0.4"},
    "libtorrent": {"prefix": "-LT2030-", "user_agent": "libtorrent/2.0.3"},
}

# ---------------------------------------------------------------------------
# Size helpers
# ---------------------------------------------------------------------------
SIZE_UNITS = {"B": 1, "KB": 1024, "MB": 1024**2, "GB": 1024**3, "TB": 1024**4}
DURATION_UNITS = {"S": 1, "M": 60, "H": 3600}


def parse_size(value: str) -> int:
    """Parse human-readable size string (e.g. '1GB', '500MB') to bytes."""
    value = value.strip().upper()
    for suffix, multiplier in sorted(SIZE_UNITS.items(), key=lambda x: -len(x[0])):
        if value.endswith(suffix):
            number = value[: -len(suffix)].strip()
            return int(float(number) * multiplier)
    return int(value)


def parse_duration(value: str) -> int:
    """Parse human-readable duration string (e.g. '30m', '1h', '90s') to seconds."""
    value = value.strip().upper()
    for suffix, multiplier in sorted(DURATION_UNITS.items(), key=lambda x: -len(x[0])):
        if value.endswith(suffix):
            number = value[: -len(suffix)].strip()
            return int(float(number) * multiplier)
    return int(value)


def format_duration(seconds: int) -> str:
    """Format seconds to human-readable string."""
    if seconds < 60:
        return f"{seconds}s"
    if seconds < 3600:
        m, s = divmod(seconds, 60)
        return f"{m}m{s}s" if s else f"{m}m"
    h, rem = divmod(seconds, 3600)
    m, s = divmod(rem, 60)
    parts = [f"{h}h"]
    if m:
        parts.append(f"{m}m")
    return "".join(parts)


def format_speed(bytes_per_sec: float) -> str:
    """Format bytes/s to human-readable speed."""
    return f"{format_size(int(bytes_per_sec))}/s"


def format_size(num_bytes: int) -> str:
    """Format bytes to human-readable string."""
    for unit in ("B", "KB", "MB", "GB", "TB"):
        if abs(num_bytes) < 1024:
            return f"{num_bytes:.1f} {unit}" if unit != "B" else f"{num_bytes} B"
        num_bytes /= 1024
    return f"{num_bytes:.1f} PB"

# ---------------------------------------------------------------------------
# Data classes
# ---------------------------------------------------------------------------

@dataclass
class TorrentInfo:
    announce_url: str
    info_hash: bytes
    name: str
    total_size: int
    piece_length: int
    num_pieces: int


@dataclass
class AnnounceResult:
    test_id: int
    test_name: str
    status_code: int
    response_size: int
    response_body: bytes
    url: str
    detected: bool = False
    error: Optional[str] = None

    def to_dict(self):
        d = {
            "test_id": self.test_id,
            "test_name": self.test_name,
            "status_code": self.status_code,
            "response_size": self.response_size,
            "response_preview": self.response_body[:200].decode(errors="replace"),
            "detected": self.detected,
            "error": self.error,
        }
        if hasattr(self, "_sim_steps"):
            d["simulation_steps"] = [s.to_dict() for s in self._sim_steps]
        return d


@dataclass
class TestSession:
    torrent: TorrentInfo
    results: list = field(default_factory=list)
    start_time: float = 0.0
    end_time: float = 0.0

    def to_dict(self):
        return {
            "torrent_name": self.torrent.name,
            "tracker": self.torrent.announce_url,
            "info_hash": self.torrent.info_hash.hex(),
            "total_size": self.torrent.total_size,
            "duration_s": round(self.end_time - self.start_time, 2),
            "results": [r.to_dict() for r in self.results],
            "verdict": self.verdict(),
        }

    def verdict(self) -> str:
        if not self.results:
            return "NO_TESTS_RUN"
        detected = any(r.detected for r in self.results)
        errors = any(r.error for r in self.results)
        if detected:
            return "DETECTED"
        if errors:
            return "PARTIAL_ERRORS"
        return "NOT_DETECTED"

# ---------------------------------------------------------------------------
# Core functions
# ---------------------------------------------------------------------------

logger = logging.getLogger("ratiotracker")


def parse_torrent(path: Path) -> TorrentInfo:
    """Parse a .torrent file and extract metadata."""
    raw = path.read_bytes()
    torrent = bencodepy.decode(raw)
    info = torrent[b"info"]
    announce_url = torrent[b"announce"].decode()
    info_hash = hashlib.sha1(bencodepy.encode(info)).digest()
    name = info.get(b"name", b"unknown").decode(errors="replace")
    piece_length = info.get(b"piece length", 0)

    if b"length" in info:
        total_size = info[b"length"]
    else:
        total_size = sum(f[b"length"] for f in info.get(b"files", []))

    pieces_raw = info.get(b"pieces", b"")
    num_pieces = len(pieces_raw) // 20

    return TorrentInfo(
        announce_url=announce_url,
        info_hash=info_hash,
        name=name,
        total_size=total_size,
        piece_length=piece_length,
        num_pieces=num_pieces,
    )


def generate_peer_id(prefix: str) -> bytes:
    """Generate a peer ID with given client prefix."""
    suffix_len = 20 - len(prefix)
    suffix = "".join(random.choices(string.ascii_letters + string.digits, k=suffix_len))
    return f"{prefix}{suffix}"[:20].encode()


def do_announce(
    announce_url: str,
    info_hash_bytes: bytes,
    peer_id: bytes,
    user_agent: str,
    port: int,
    timeout: int,
    event: Optional[str] = None,
    uploaded: int = 0,
    downloaded: int = 0,
    left: int = 0,
    dry_run: bool = False,
) -> tuple[int, bytes, str]:
    """Send an announce request. Returns (status_code, content, url)."""
    ih = urllib.parse.quote(info_hash_bytes, safe="")
    pid = urllib.parse.quote(peer_id, safe="")
    params = urllib.parse.urlencode({
        "port": port,
        "uploaded": uploaded,
        "downloaded": downloaded,
        "left": left,
        "compact": 1,
        "numwant": 50,
        "key": "".join(random.choices("0123456789ABCDEF", k=8)),
        "corrupt": 0,
    })
    url = f"{announce_url}?info_hash={ih}&peer_id={pid}&{params}"
    if event:
        url += f"&event={event}"

    logger.debug("URL: %s", url)

    if dry_run:
        logger.info("[DRY-RUN] Would send GET %s", url[:120] + "...")
        return 0, b"[dry-run]", url

    resp = requests.get(url, headers={"User-Agent": user_agent}, timeout=timeout)
    return resp.status_code, resp.content, url


# ---------------------------------------------------------------------------
# Realistic speed simulation engine
# ---------------------------------------------------------------------------

class SpeedSimulator:
    """Generates realistic fluctuating transfer speeds mimicking a real BT client.

    Models:
    - Slow ramp-up phase at the start (peer discovery)
    - Sustained cruising speed with random jitter
    - Occasional micro-drops (peer churn, congestion)
    - Rare larger drops (peer disconnect)
    - Gradual taper at the end (fewer pieces left to share)
    """

    def __init__(
        self,
        target_bytes: int,
        duration_sec: int,
        jitter: float = 0.25,
        ramp_fraction: float = 0.10,
        taper_fraction: float = 0.08,
        micro_drop_chance: float = 0.12,
        big_drop_chance: float = 0.03,
    ):
        self.target_bytes = target_bytes
        self.duration_sec = duration_sec
        self.base_speed = target_bytes / duration_sec if duration_sec > 0 else 0  # bytes/s average
        self.jitter = jitter
        self.ramp_fraction = ramp_fraction
        self.taper_fraction = taper_fraction
        self.micro_drop_chance = micro_drop_chance
        self.big_drop_chance = big_drop_chance

    def speed_at(self, elapsed_sec: int) -> float:
        """Return a realistic instantaneous speed (bytes/s) at given elapsed time."""
        progress = elapsed_sec / self.duration_sec if self.duration_sec > 0 else 1.0
        progress = min(progress, 1.0)

        # Base multiplier from phase
        if progress < self.ramp_fraction:
            # Ramp-up: cubic ease-in from 5% to 100%
            t = progress / self.ramp_fraction
            phase_mult = 0.05 + 0.95 * (t ** 2)
        elif progress > (1.0 - self.taper_fraction):
            # Taper: slow down at the end
            t = (progress - (1.0 - self.taper_fraction)) / self.taper_fraction
            phase_mult = 1.0 - 0.4 * (t ** 1.5)
        else:
            phase_mult = 1.0

        # Random jitter (gaussian centered on 1.0)
        jitter_mult = max(0.1, random.gauss(1.0, self.jitter))

        # Occasional drops
        drop_mult = 1.0
        if random.random() < self.big_drop_chance:
            drop_mult = random.uniform(0.05, 0.3)
        elif random.random() < self.micro_drop_chance:
            drop_mult = random.uniform(0.4, 0.8)

        return max(0, self.base_speed * phase_mult * jitter_mult * drop_mult)

    def generate_plan(self, interval_sec: int) -> list[dict]:
        """Pre-compute the full announce plan: list of {elapsed, delta_bytes, total_uploaded, speed}."""
        steps = []
        total_uploaded = 0
        num_intervals = max(1, self.duration_sec // interval_sec)

        for i in range(num_intervals):
            elapsed = (i + 1) * interval_sec
            speed = self.speed_at(elapsed)
            delta = int(speed * interval_sec)

            # Don't overshoot
            if total_uploaded + delta > self.target_bytes:
                delta = self.target_bytes - total_uploaded

            total_uploaded += delta
            steps.append({
                "step": i + 1,
                "elapsed_sec": elapsed,
                "delta_bytes": delta,
                "total_uploaded": total_uploaded,
                "speed_bps": speed,
            })

            if total_uploaded >= self.target_bytes:
                break

        # Ensure we hit the exact target on the last step
        if steps and steps[-1]["total_uploaded"] < self.target_bytes:
            deficit = self.target_bytes - steps[-1]["total_uploaded"]
            steps[-1]["delta_bytes"] += deficit
            steps[-1]["total_uploaded"] = self.target_bytes

        return steps


@dataclass
class SimulationStep:
    step: int
    elapsed_sec: int
    delta_bytes: int
    total_uploaded: int
    speed_bps: float
    status_code: int
    response_size: int
    response_body: bytes
    detected: bool
    error: Optional[str] = None

    def to_dict(self):
        return {
            "step": self.step,
            "elapsed": self.elapsed_sec,
            "delta": self.delta_bytes,
            "total_uploaded": self.total_uploaded,
            "speed": format_speed(self.speed_bps),
            "status": self.status_code,
            "detected": self.detected,
            "error": self.error,
        }


def is_error_response(status: int, content: bytes) -> bool:
    """Heuristic to detect if the tracker rejected the announce."""
    if status == 0:
        return False  # dry-run
    if status != 200:
        return True
    try:
        decoded = bencodepy.decode(content)
        if b"failure reason" in decoded:
            return True
    except Exception:
        pass
    return False

# ---------------------------------------------------------------------------
# Test definitions
# ---------------------------------------------------------------------------

def test_fake_seed(session, peer_id, port, ua, timeout, dry_run, **_kw):
    """TEST 1: started announce with left=0 (fake seed, no actual data)."""
    t = session.torrent
    status, content, url = do_announce(
        t.announce_url, t.info_hash, peer_id, ua, port, timeout,
        event="started", left=0, dry_run=dry_run,
    )
    # Cleanup
    do_announce(
        t.announce_url, t.info_hash, peer_id, ua, port, timeout,
        event="stopped", uploaded=0, left=0, dry_run=dry_run,
    )
    return AnnounceResult(
        test_id=1, test_name="fake_seed",
        status_code=status, response_size=len(content),
        response_body=content, url=url,
        detected=is_error_response(status, content),
    )


def test_inflated_upload(session, peer_id, port, ua, timeout, dry_run, upload_amount, **_kw):
    """TEST 2: update announce with inflated uploaded bytes."""
    t = session.torrent
    # Register peer first
    do_announce(
        t.announce_url, t.info_hash, peer_id, ua, port, timeout,
        event="started", uploaded=0, left=0, dry_run=dry_run,
    )
    status, content, url = do_announce(
        t.announce_url, t.info_hash, peer_id, ua, port, timeout,
        uploaded=upload_amount, left=0, dry_run=dry_run,
    )
    # Cleanup
    do_announce(
        t.announce_url, t.info_hash, peer_id, ua, port, timeout,
        event="stopped", uploaded=upload_amount, left=0, dry_run=dry_run,
    )
    return AnnounceResult(
        test_id=2, test_name="inflated_upload",
        status_code=status, response_size=len(content),
        response_body=content, url=url,
        detected=is_error_response(status, content),
    )


def test_negative_downloaded(session, peer_id, port, ua, timeout, dry_run, negative_dl, **_kw):
    """TEST 3: announce with negative downloaded to reduce ratio debt."""
    t = session.torrent
    # Register peer first
    do_announce(
        t.announce_url, t.info_hash, peer_id, ua, port, timeout,
        event="started", uploaded=0, left=0, dry_run=dry_run,
    )
    status, content, url = do_announce(
        t.announce_url, t.info_hash, peer_id, ua, port, timeout,
        downloaded=-negative_dl, left=0, dry_run=dry_run,
    )
    # Cleanup
    do_announce(
        t.announce_url, t.info_hash, peer_id, ua, port, timeout,
        event="stopped", uploaded=0, left=0, dry_run=dry_run,
    )
    return AnnounceResult(
        test_id=3, test_name="negative_downloaded",
        status_code=status, response_size=len(content),
        response_body=content, url=url,
        detected=is_error_response(status, content),
    )


def test_huge_single_update(session, peer_id, port, ua, timeout, dry_run, bomb_size, **_kw):
    """TEST 4: single announce claiming a large amount uploaded in one shot."""
    t = session.torrent
    amount = bomb_size
    # Register peer first
    do_announce(
        t.announce_url, t.info_hash, peer_id, ua, port, timeout,
        event="started", uploaded=0, left=0, dry_run=dry_run,
    )
    status, content, url = do_announce(
        t.announce_url, t.info_hash, peer_id, ua, port, timeout,
        uploaded=amount, left=0, dry_run=dry_run,
    )
    # Cleanup
    do_announce(
        t.announce_url, t.info_hash, peer_id, ua, port, timeout,
        event="stopped", uploaded=amount, left=0, dry_run=dry_run,
    )
    return AnnounceResult(
        test_id=4, test_name="huge_single_update",
        status_code=status, response_size=len(content),
        response_body=content, url=url,
        detected=is_error_response(status, content),
    )


def test_impossible_speed(session, peer_id, port, ua, timeout, dry_run, delay, speed_amount, **_kw):
    """TEST 5: two rapid announces implying impossible transfer speed."""
    t = session.torrent
    # First: start with 0
    do_announce(
        t.announce_url, t.info_hash, peer_id, ua, port, timeout,
        event="started", uploaded=0, left=0, dry_run=dry_run,
    )
    sleep_time = max(1, delay)
    if not dry_run:
        logger.info("  Waiting %ds before second announce...", sleep_time)
        time.sleep(sleep_time)
    # Second: claim large upload after minimal delay
    amount = speed_amount
    status, content, url = do_announce(
        t.announce_url, t.info_hash, peer_id, ua, port, timeout,
        uploaded=amount, left=0, dry_run=dry_run,
    )
    return AnnounceResult(
        test_id=5, test_name="impossible_speed",
        status_code=status, response_size=len(content),
        response_body=content, url=url,
        detected=is_error_response(status, content),
    )


def test_multiple_peer_ids(session, peer_id, port, ua, timeout, dry_run, multi_seed_upload, multi_seed_count, **_kw):
    """TEST 6: multiple different peer IDs for the same torrent (multi-seed spoof)."""
    t = session.torrent
    profile = [p for p in CLIENT_PROFILES.values() if p["user_agent"] == ua]
    prefix = profile[0]["prefix"] if profile else "-UT3320-"

    results_all = []
    for i in range(multi_seed_count):
        pid = generate_peer_id(prefix)
        status, content, url = do_announce(
            t.announce_url, t.info_hash, pid, ua, port, timeout,
            event="started", uploaded=multi_seed_upload, left=0, dry_run=dry_run,
        )
        results_all.append((status, content))

    detected = any(is_error_response(s, c) for s, c in results_all)
    return AnnounceResult(
        test_id=6, test_name="multiple_peer_ids",
        status_code=results_all[-1][0], response_size=len(results_all[-1][1]),
        response_body=results_all[-1][1], url="[multiple requests]",
        detected=detected,
    )


def test_stopped(session, peer_id, port, ua, timeout, dry_run, upload_amount, **_kw):
    """TEST 7: stopped announce (cleanup)."""
    t = session.torrent
    # Register peer first
    do_announce(
        t.announce_url, t.info_hash, peer_id, ua, port, timeout,
        event="started", uploaded=0, left=0, dry_run=dry_run,
    )
    status, content, url = do_announce(
        t.announce_url, t.info_hash, peer_id, ua, port, timeout,
        event="stopped", uploaded=upload_amount, left=0, dry_run=dry_run,
    )
    return AnnounceResult(
        test_id=7, test_name="stopped",
        status_code=status, response_size=len(content),
        response_body=content, url=url,
        detected=is_error_response(status, content),
    )


def test_simulate(session, peer_id, port, ua, timeout, dry_run,
                   sim_amount, sim_duration, sim_interval, sim_jitter, **_kw):
    """TEST 8: Realistic transfer simulation with gradual announces."""
    t = session.torrent
    color = use_color(None)

    simulator = SpeedSimulator(
        target_bytes=sim_amount,
        duration_sec=sim_duration,
        jitter=sim_jitter,
    )
    plan = simulator.generate_plan(sim_interval)

    if not plan:
        return AnnounceResult(
            test_id=8, test_name="simulate",
            status_code=0, response_size=0,
            response_body=b"", url="",
            error="Empty simulation plan",
        )

    c, r, d, g, rd, y = ("", "", "", "", "", "")
    if color:
        c, r, d, g, rd, y = (CYAN, RESET, DIM, GREEN, RED, YELLOW)

    # Start event
    status, content, url = do_announce(
        t.announce_url, t.info_hash, peer_id, ua, port, timeout,
        event="started", uploaded=0, left=0, dry_run=dry_run,
    )
    if is_error_response(status, content):
        return AnnounceResult(
            test_id=8, test_name="simulate",
            status_code=status, response_size=len(content),
            response_body=content, url=url, detected=True,
        )

    print(f"\n    {c}Simulation:{r} {format_size(sim_amount)} over "
          f"{format_duration(sim_duration)} ({len(plan)} announces, "
          f"interval {sim_interval}s)")
    print(f"    {d}{'Step':>4}  {'Elapsed':>8}  {'Speed':>12}  {'Delta':>10}  "
          f"{'Total':>12}  {'Progress':>8}  Status{r}")
    print(f"    {d}{'─' * 75}{r}")

    sim_steps = []
    detected = False

    for step_plan in plan:
        if not dry_run:
            time.sleep(sim_interval)

        status, content, url = do_announce(
            t.announce_url, t.info_hash, peer_id, ua, port, timeout,
            uploaded=step_plan["total_uploaded"], left=0, dry_run=dry_run,
        )

        step_detected = is_error_response(status, content)
        if step_detected:
            detected = True

        step = SimulationStep(
            step=step_plan["step"],
            elapsed_sec=step_plan["elapsed_sec"],
            delta_bytes=step_plan["delta_bytes"],
            total_uploaded=step_plan["total_uploaded"],
            speed_bps=step_plan["speed_bps"],
            status_code=status,
            response_size=len(content),
            response_body=content,
            detected=step_detected,
        )
        sim_steps.append(step)

        # Live progress line
        pct = (step_plan["total_uploaded"] / sim_amount * 100) if sim_amount > 0 else 100
        bar_len = 20
        filled = int(bar_len * pct / 100)
        bar = "█" * filled + "░" * (bar_len - filled)
        status_str = f"{g}OK{r}" if not step_detected else f"{rd}FAIL{r}"

        print(f"    {step_plan['step']:>4}  "
              f"{format_duration(step_plan['elapsed_sec']):>8}  "
              f"{format_speed(step_plan['speed_bps']):>12}  "
              f"+{format_size(step_plan['delta_bytes']):>9}  "
              f"{format_size(step_plan['total_uploaded']):>12}  "
              f"{bar} {pct:5.1f}%  "
              f"{status_str}")

        if step_detected:
            print(f"    {rd}  ⚠ Tracker rejected announce at step {step_plan['step']}{r}")
            break

    # Stop event
    final_uploaded = sim_steps[-1].total_uploaded if sim_steps else 0
    do_announce(
        t.announce_url, t.info_hash, peer_id, ua, port, timeout,
        event="stopped", uploaded=final_uploaded, left=0, dry_run=dry_run,
    )

    # Build result with simulation details attached
    result = AnnounceResult(
        test_id=8, test_name="simulate",
        status_code=sim_steps[-1].status_code if sim_steps else 0,
        response_size=sim_steps[-1].response_size if sim_steps else 0,
        response_body=sim_steps[-1].response_body if sim_steps else b"",
        url="[simulation]",
        detected=detected,
    )
    # Attach detailed steps for JSON output
    result._sim_steps = sim_steps
    return result


ALL_TESTS = {
    1: ("Fake seed (left=0, no data)", test_fake_seed),
    2: ("Inflated upload claim", test_inflated_upload),
    3: ("Negative downloaded value", test_negative_downloaded),
    4: ("Huge single update (bomb)", test_huge_single_update),
    5: ("Impossible transfer speed", test_impossible_speed),
    6: ("Multiple peer IDs (multi-seed)", test_multiple_peer_ids),
    7: ("Stopped announce (cleanup)", test_stopped),
    8: ("Realistic transfer simulation", test_simulate),
}

# ---------------------------------------------------------------------------
# Output formatting
# ---------------------------------------------------------------------------

RESET = "\033[0m"
RED = "\033[91m"
GREEN = "\033[92m"
YELLOW = "\033[93m"
CYAN = "\033[96m"
BOLD = "\033[1m"
DIM = "\033[2m"


def use_color(force: Optional[bool]) -> bool:
    if force is True:
        return True
    if force is False:
        return False
    return sys.stdout.isatty()


def print_banner(color: bool):
    banner = r"""
__________         __  .__     ___________                     __                 
\______   \_____ _/  |_|__| ___\__    ___/___________    ____ |  | __ ___________ 
 |       _/\__  \\   __\  |/  _ \|    |  \_  __ \__  \ _/ ___\|  |/ // __ \_  __ \
 |    |   \ / __ \|  | |  (  <_> )    |   |  | \// __ \\  \___|    <\  ___/|  | \/
 |____|_  /(____  /__| |__|\____/|____|   |__|  (____  /\___  >__|_ \\___  >__|   
        \/      \/                                   \/     \/     \/    \/       
    """
    if color:
        print(f"{CYAN}{banner}{RESET}")
        print(f"  {DIM}Ratio Manipulation Detection Tester v{__version__}{RESET}\n")
    else:
        print(banner)
        print(f"  Ratio Manipulation Detection Tester v{__version__}\n")


def print_torrent_info(info: TorrentInfo, color: bool):
    c, r = (CYAN, RESET) if color else ("", "")
    print(f"  {c}Torrent:{r}  {info.name}")
    print(f"  {c}Tracker:{r}  {info.announce_url}")
    print(f"  {c}Hash:{r}     {info.info_hash.hex()}")
    print(f"  {c}Size:{r}     {format_size(info.total_size)}")
    print(f"  {c}Pieces:{r}   {info.num_pieces} x {format_size(info.piece_length)}")
    print()


def print_result(result: AnnounceResult, color: bool):
    if color:
        status_color = GREEN if not result.detected else RED
        tag = f"{status_color}{'DETECTED' if result.detected else 'PASSED'}{RESET}"
        print(f"  [{tag}] {BOLD}Test {result.test_id}{RESET}: {ALL_TESTS[result.test_id][0]}")
        if result.error:
            print(f"         {RED}Error: {result.error}{RESET}")
        else:
            print(f"         HTTP {result.status_code} — {result.response_size} bytes")
            preview = result.response_body[:120].decode(errors="replace")
            print(f"         {DIM}{preview}{RESET}")
    else:
        tag = "DETECTED" if result.detected else "PASSED"
        print(f"  [{tag}] Test {result.test_id}: {ALL_TESTS[result.test_id][0]}")
        if result.error:
            print(f"         Error: {result.error}")
        else:
            print(f"         HTTP {result.status_code} — {result.response_size} bytes")


def print_verdict(session: TestSession, color: bool):
    verdict = session.verdict()
    elapsed = round(session.end_time - session.start_time, 2)

    print()
    if color:
        print(f"{'=' * 50}")
        v_color = RED if verdict == "DETECTED" else (YELLOW if verdict == "PARTIAL_ERRORS" else GREEN)
        print(f"  {BOLD}VERDICT: {v_color}{verdict}{RESET}")
        print(f"  {DIM}Completed in {elapsed}s{RESET}")
        print(f"{'=' * 50}")
    else:
        print("=" * 50)
        print(f"  VERDICT: {verdict}")
        print(f"  Completed in {elapsed}s")
        print("=" * 50)

    if verdict == "NOT_DETECTED":
        print("\n  Tracker does NOT validate upload claims against actual transfer.")
        print("  Recommendation: implement server-side ratio verification.\n")
    elif verdict == "DETECTED":
        print("\n  Tracker HAS detection mechanisms in place.")
        print("  Some or all manipulation attempts were rejected.\n")

# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="ratiotracker",
        description="RatioTracker — test if your BitTorrent tracker detects ratio manipulation.",
        epilog=
            "━━━ CHEAT SHEET ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"
            "\n"
            "  Quick scan (all tests):     %(prog)s file.torrent\n"
            "  Specific tests only:        %(prog)s file.torrent -t 1 2 5\n"
            "  List tests:                 %(prog)s --list-tests\n"
            "\n"
            "  Simulate 50GB over 2h:      %(prog)s file.torrent -t 8 --sim-amount 50GB --sim-duration 2h\n"
            "  Simulate 10GB/30m, slow:    %(prog)s file.torrent -t 8 --sim-amount 10GB --sim-interval 60\n"
            "  Simulate with high jitter:  %(prog)s file.torrent -t 8 --sim-jitter 0.5\n"
            "\n"
            "  Emulate qBittorrent:        %(prog)s file.torrent -c qbit\n"
            "  Custom client:              %(prog)s file.torrent --user-agent 'Deluge/2.1' --peer-id-prefix '-DE2110-'\n"
            "\n"
            "  Repeat 5 times, 30s gap:    %(prog)s file.torrent --cycles 5 --delay 30\n"
            "  JSON report to file:        %(prog)s file.torrent --json -o report.json\n"
            "  Debug mode:                 %(prog)s file.torrent -vv --log-file debug.log\n"
            "  Preview without sending:    %(prog)s file.torrent --dry-run -v\n"
            "\n"
            "━━━ SIZE FORMATS ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"
            "  Bytes: 1024  |  KB: 500KB  |  MB: 100MB  |  GB: 10GB  |  TB: 1TB\n"
            "\n"
            "━━━ DURATION FORMATS ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"
            "  Seconds: 90s  |  Minutes: 30m  |  Hours: 2h\n"
            "\n"
            "━━━ CLIENTS ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"
            "  utorrent      uTorrent/3320      -UT3320-\n"
            "  qbit          qBittorrent/4.7.1  -qB4710-\n"
            "  deluge        Deluge/2.1.1       -DE2110-\n"
            "  transmission  Transmission/4.0.4 -TR4040-\n"
            "  libtorrent    libtorrent/2.0.3   -LT2030-\n"
            "\n"
            "━━━ TESTS ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"
            "  1  Fake seed         Start as seed without data\n"
            "  2  Inflated upload   Claim upload without transfer\n"
            "  3  Negative DL       Send --negative-dl (default 1GB) as negative downloaded\n"
            "  4  Size bomb         Single announce claiming --bomb-size (default 100GB)\n"
            "  5  Speed test        Claim --speed-amount (default 50GB) after minimal delay\n"
            "  6  Multi peer IDs    --multi-seed-count peers (default 3) claiming --multi-seed-upload each\n"
            "  7  Stop event        Clean stop announce\n"
            "  8  Simulation        Realistic gradual transfer over time\n"
            "\n"
            "━━━ EXIT CODES ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"
            "  0  At least one test passed undetected\n"
            "  1  All tests detected / rejected by tracker\n",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )

    parser.add_argument("torrent", type=Path, nargs="?", default=None,
                        help="Path to .torrent file")
    parser.add_argument("--version", action="version", version=f"%(prog)s {__version__}")

    # Test selection
    test_group = parser.add_argument_group("test selection")
    test_group.add_argument(
        "-t", "--tests", type=int, nargs="+", metavar="N",
        choices=list(range(1, 9)),
        help="Run only specific tests (1-8). Default: all",
    )
    test_group.add_argument(
        "--list-tests", action="store_true",
        help="List available tests and exit",
    )

    # Announce parameters
    announce_group = parser.add_argument_group("announce parameters")
    announce_group.add_argument(
        "-u", "--upload", type=str, default="1GB", metavar="SIZE",
        help="Upload amount to claim (e.g. 500MB, 1GB, 10GB). Default: 1GB",
    )
    announce_group.add_argument(
        "-d", "--download", type=str, default="0", metavar="SIZE",
        help="Download amount to claim. Default: 0",
    )
    announce_group.add_argument(
        "--left", type=str, default="0", metavar="SIZE",
        help="Bytes remaining. Default: 0 (pretend complete)",
    )
    announce_group.add_argument(
        "--bomb-size", type=str, default="100GB", metavar="SIZE",
        help="Upload amount for test 4 bomb (e.g. 50GB, 100GB, 1TB). Default: 100GB",
    )
    announce_group.add_argument(
        "--negative-dl", type=str, default="1GB", metavar="SIZE",
        help="Negative downloaded value for test 3 (e.g. 1GB, 5GB). Default: 1GB",
    )
    announce_group.add_argument(
        "--speed-amount", type=str, default="50GB", metavar="SIZE",
        help="Upload claimed in test 5 after minimal delay (e.g. 50GB, 100GB). Default: 50GB",
    )
    announce_group.add_argument(
        "--multi-seed-upload", type=str, default="1GB", metavar="SIZE",
        help="Upload claimed per peer in test 6 (e.g. 1GB, 5GB). Default: 1GB",
    )
    announce_group.add_argument(
        "--multi-seed-count", type=int, default=3, metavar="N",
        help="Number of fake peers in test 6. Default: 3",
    )
    announce_group.add_argument(
        "-p", "--port", type=int, default=None, metavar="PORT",
        help="Listening port to announce. Default: random 10000-60000",
    )

    # Client emulation
    client_group = parser.add_argument_group("client emulation")
    client_group.add_argument(
        "-c", "--client", choices=list(CLIENT_PROFILES.keys()), default="utorrent",
        help="BitTorrent client to emulate. Default: utorrent",
    )
    client_group.add_argument(
        "--user-agent", type=str, default=None, metavar="UA",
        help="Override User-Agent header (ignores --client for UA)",
    )
    client_group.add_argument(
        "--peer-id-prefix", type=str, default=None, metavar="PREFIX",
        help="Override peer ID prefix (e.g. '-qB4710-')",
    )

    # Simulation mode (test 8)
    sim_group = parser.add_argument_group(
        "simulation (test 8)",
        "Simulate a realistic transfer: gradual announces with fluctuating speeds"
    )
    sim_group.add_argument(
        "--sim-amount", type=str, default="10GB", metavar="SIZE",
        help="Total data to simulate uploading. Default: 10GB",
    )
    sim_group.add_argument(
        "--sim-duration", type=str, default="30m", metavar="DURATION",
        help="Time over which to spread the transfer (e.g. 30m, 1h, 90s). Default: 30m",
    )
    sim_group.add_argument(
        "--sim-interval", type=int, default=30, metavar="SEC",
        help="Seconds between each announce during simulation. Default: 30",
    )
    sim_group.add_argument(
        "--sim-jitter", type=float, default=0.25, metavar="RATIO",
        help="Speed jitter factor (0.0 = constant, 0.5 = high variance). Default: 0.25",
    )

    # Execution
    exec_group = parser.add_argument_group("execution")
    exec_group.add_argument(
        "--cycles", type=int, default=1, metavar="N",
        help="Repeat the test suite N times. Default: 1",
    )
    exec_group.add_argument(
        "--delay", type=int, default=5, metavar="SEC",
        help="Delay in seconds between cycles / speed tests. Default: 5",
    )
    exec_group.add_argument(
        "--timeout", type=int, default=30, metavar="SEC",
        help="HTTP request timeout in seconds. Default: 30",
    )
    exec_group.add_argument(
        "--dry-run", action="store_true",
        help="Show what would be sent without making requests",
    )

    # Output
    output_group = parser.add_argument_group("output")
    output_group.add_argument(
        "-v", "--verbose", action="count", default=0,
        help="Increase verbosity (-v, -vv)",
    )
    output_group.add_argument(
        "-q", "--quiet", action="store_true",
        help="Only output verdict (or JSON)",
    )
    output_group.add_argument(
        "--json", action="store_true",
        help="Output results as JSON",
    )
    output_group.add_argument(
        "--no-color", action="store_true",
        help="Disable colored output",
    )
    output_group.add_argument(
        "--color", action="store_true",
        help="Force colored output (even in pipes)",
    )
    output_group.add_argument(
        "-o", "--output", type=Path, default=None, metavar="FILE",
        help="Write results to file (JSON format)",
    )
    output_group.add_argument(
        "--log-file", type=Path, default=None, metavar="FILE",
        help="Write verbose log to file",
    )

    return parser


def main():
    parser = build_parser()
    args = parser.parse_args()

    # --list-tests
    if args.list_tests:
        print("Available tests:")
        for tid, (desc, _) in ALL_TESTS.items():
            print(f"  {tid}. {desc}")
        sys.exit(0)

    if args.torrent is None:
        parser.error("the following arguments are required: torrent")

    # Logging setup
    log_level = logging.WARNING
    if args.verbose >= 2:
        log_level = logging.DEBUG
    elif args.verbose >= 1:
        log_level = logging.INFO

    handlers = [logging.StreamHandler()]
    if args.log_file:
        handlers.append(logging.FileHandler(args.log_file))

    logging.basicConfig(
        level=log_level,
        format="%(asctime)s [%(levelname)s] %(message)s",
        handlers=handlers,
    )

    # Color
    color_flag = None
    if args.no_color:
        color_flag = False
    elif args.color:
        color_flag = True
    color = use_color(color_flag)

    # Validate torrent path
    if not args.torrent.exists():
        print(f"Error: file not found: {args.torrent}", file=sys.stderr)
        sys.exit(1)

    # Parse torrent
    try:
        torrent_info = parse_torrent(args.torrent)
    except Exception as e:
        print(f"Error parsing torrent file: {e}", file=sys.stderr)
        sys.exit(1)

    # Resolve client profile
    profile = CLIENT_PROFILES[args.client]
    prefix = args.peer_id_prefix or profile["prefix"]
    ua = args.user_agent or profile["user_agent"]
    port = args.port or random.randint(10000, 60000)
    upload_amount = parse_size(args.upload)
    download_amount = parse_size(args.download)
    left_amount = parse_size(args.left)
    bomb_size = parse_size(args.bomb_size)
    negative_dl = parse_size(args.negative_dl)
    speed_amount = parse_size(args.speed_amount)
    multi_seed_upload = parse_size(args.multi_seed_upload)
    multi_seed_count = args.multi_seed_count
    sim_amount = parse_size(args.sim_amount)
    sim_duration = parse_duration(args.sim_duration)
    sim_interval = args.sim_interval

    # Select tests
    test_ids = args.tests or list(ALL_TESTS.keys())

    # Banner
    if not args.quiet and not args.json:
        print_banner(color)
        print_torrent_info(torrent_info, color)
        print(f"  Client:  {args.client} ({ua})")
        print(f"  Upload:  {format_size(upload_amount)}")
        print(f"  Port:    {port}")
        print(f"  Tests:   {test_ids}")
        print(f"  Cycles:  {args.cycles}")
        if 8 in test_ids:
            print(f"  Sim:     {format_size(sim_amount)} over {format_duration(sim_duration)} "
                  f"(interval {sim_interval}s, jitter {args.sim_jitter})")
        if args.dry_run:
            b, r = (YELLOW, RESET) if color else ("", "")
            print(f"  {b}Mode:    DRY-RUN{r}")
        print()

    # Run
    all_sessions = []

    for cycle in range(1, args.cycles + 1):
        if args.cycles > 1 and not args.quiet and not args.json:
            print(f"--- Cycle {cycle}/{args.cycles} ---")

        peer_id = generate_peer_id(prefix)
        session = TestSession(torrent=torrent_info, start_time=time.time())

        test_kwargs = {
            "session": session,
            "peer_id": peer_id,
            "port": port,
            "ua": ua,
            "timeout": args.timeout,
            "dry_run": args.dry_run,
            "upload_amount": upload_amount,
            "download_amount": download_amount,
            "left_amount": left_amount,
            "bomb_size": bomb_size,
            "negative_dl": negative_dl,
            "speed_amount": speed_amount,
            "multi_seed_upload": multi_seed_upload,
            "multi_seed_count": multi_seed_count,
            "delay": args.delay,
            "sim_amount": sim_amount,
            "sim_duration": sim_duration,
            "sim_interval": sim_interval,
            "sim_jitter": args.sim_jitter,
        }

        for tid in test_ids:
            desc, test_fn = ALL_TESTS[tid]
            if not args.quiet and not args.json:
                print(f"\n  Running test {tid}: {desc}...")

            try:
                result = test_fn(**test_kwargs)
                session.results.append(result)
                if not args.quiet and not args.json:
                    print_result(result, color)
            except requests.RequestException as e:
                err_result = AnnounceResult(
                    test_id=tid, test_name=desc,
                    status_code=0, response_size=0,
                    response_body=b"", url="",
                    detected=False, error=str(e),
                )
                session.results.append(err_result)
                if not args.quiet and not args.json:
                    print_result(err_result, color)
                logger.error("Request failed for test %d: %s", tid, e)

        session.end_time = time.time()
        all_sessions.append(session)

        if cycle < args.cycles and args.delay > 0:
            if not args.quiet and not args.json:
                print(f"\n  Waiting {args.delay}s before next cycle...")
            time.sleep(args.delay)

    # Output
    if args.json:
        output_data = {
            "version": __version__,
            "sessions": [s.to_dict() for s in all_sessions],
        }
        json_str = json.dumps(output_data, indent=2)
        print(json_str)
    elif not args.quiet:
        for s in all_sessions:
            print_verdict(s, color)
    else:
        for s in all_sessions:
            print(s.verdict())

    # File output
    if args.output:
        output_data = {
            "version": __version__,
            "sessions": [s.to_dict() for s in all_sessions],
        }
        args.output.write_text(json.dumps(output_data, indent=2))
        if not args.quiet and not args.json:
            print(f"\n  Results saved to {args.output}")

    # Exit code: 0 if any test passed undetected, 1 if all detected
    final_verdict = all_sessions[-1].verdict() if all_sessions else "NO_TESTS_RUN"
    sys.exit(0 if final_verdict != "DETECTED" else 1)


if __name__ == "__main__":
    main()
