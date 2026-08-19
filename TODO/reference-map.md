# Reference map

What is kept under `reference/`, why, and under what licence. `reference/` is
gitignored and is deleted in Phase B; this file is what survives it.

Every entry states the licence, because forgetting which pile a file came from
is how an AGPL function ends up in an MIT tree. See `TODO/licensing.md` for the
determinations.

Trimmed on 2026-08-19. Sizes after trimming:

```
reference/FluxDown      6.6M   296 files   AGPL-3.0, do not copy
reference/superseedr    4.8M   279 files   GPL-3.0-or-later, do not copy
reference/intermodal    735K   111 files   CC0-1.0, adapt directly
reference/rqbit         1.6M     2 files   the issue and PR corpus, data not code
reference/aria2-next    168K     1 file    the manual, documentation
```

What was deleted: the Flutter, Dart, Android, iOS, macOS, Linux desktop, web,
installer, packaging, promotion, and website trees from FluxDown; `assets/`,
`wix/`, `packaging/`, `docker*`, `agentic_*`, and the TUI tree from superseedr;
`book/`, `www/`, `bin/`, and `tmp/` from intermodal; and the `.git` directory
from all three, since none will be updated.

---

## reference/aria2-next — documentation

```
reference/aria2-next/aria2-next.rst
  The aria2-next manual, 4,805 lines. The parity source for section 9.
  Premise confirmed: 207 `.. option::` directives, zero web seed options. The
  only occurrence of "Web-seeding" is line 2595, inside aria2.addTorrent's RPC
  documentation. That gap is why this project exists.
  Documentation, quote sparingly with attribution.
```

## reference/rqbit — corpus, not code

```
reference/rqbit/issues.json
  262 issues, 91 open and 171 closed, fetched with gh on 2026-08-19. The input
  to the triage in the rest of TODO/. Data, not code.

reference/rqbit/prs.json
  346 pull requests with their changed file lists. Useful for finding which
  files a given subsystem lives in upstream. Data, not code.
```

Note that the `librqbit` source itself is not under `reference/`. It is a
crates.io dependency and its source is in the cargo registry cache, which is
where every claim in this file about "the pinned 9.0.0" was verified from.

## reference/intermodal — CC0-1.0, adapt directly

The one reference where copying is allowed. What has actually been adapted is
listed in `TODO/licensing.md`.

```
reference/intermodal/src/subcommand/torrent/create.rs
  3,196 lines. Torrent creation and the --allow <LINT> model. The basis for
  bit-cli create. CC0-1.0, adapt directly.

reference/intermodal/src/subcommand/torrent/piece_length.rs
  The piece length heuristic. CC0-1.0, adapt directly.

reference/intermodal/src/subcommand/torrent/verify.rs
  803 lines. Per-piece verification and reporting. CC0-1.0, adapt directly.

reference/intermodal/src/subcommand/torrent/show.rs
  636 lines. Metainfo summary rendering. CC0-1.0, adapt directly.

reference/intermodal/src/env.rs
  315 lines. Injects args, working directory, and the three streams into the
  program rather than reading globals. This is how section 0.11 is made
  testable. Adopted in crates/bit-cli/src/env.rs. CC0-1.0.

reference/intermodal/src/bytes.rs
  SI and binary size parsing. Adopted in bit-cli-core/src/units.rs. CC0-1.0.

reference/intermodal/src/table.rs
  Aligned output. Adopted in bit-cli/src/output.rs. CC0-1.0.

reference/intermodal/src/magnet_link.rs
  485 lines. Magnet parsing and rendering. CC0-1.0, adapt directly.

reference/intermodal/src/metainfo.rs
  460 lines. Metainfo types. CC0-1.0, adapt directly.

reference/intermodal/src/walker.rs
  Directory traversal honouring .gitignore, .ignore, and .git/info/exclude.
  Relevant to create --ignore. CC0-1.0, adapt directly.

reference/intermodal/benches/, reference/intermodal/justfile
  How they drive builds and measurements. CC0-1.0.
```

## reference/superseedr — GPL-3.0-or-later, do not copy

Read for architecture. Nothing here may be copied, translated, or closely
paraphrased into `bit-cli`.

```
reference/superseedr/src/networking/web_seed_worker.rs
  141 lines. An in-process web seed as a virtual peer, over channels rather
  than a loopback socket: SuccessfullyConnected, then a PeerBitfield of all
  0xFF, then Unchoke, then BulkRequest batches served with ranged GETs. This is
  Candidate A-prime in the section 2.2 benchmark, and the reason that benchmark
  has a third option. Read this before T-002 in TODO/webseed.md.
  GPL-3.0, do not copy.

reference/superseedr/src/synthetic_load.rs
  5,748 lines. The reference for A3.11's bench swarm: warmup window, bounded
  disk budget, adaptive step search toward a target rate, periodic metrics.
  Read for the shape, not the code. GPL-3.0, do not copy.

reference/superseedr/src/integrations/cli.rs
  1,254 lines. A working headless CLI, and the argument surface for the load
  harness. Note StatusCommandMode { Snapshot, Follow, SetInterval, Stop }: one
  subcommand serving both a one-shot query and a stream. GPL-3.0, do not copy.

reference/superseedr/src/torrent_manager/merkle.rs
  BEP 52 v2 and hybrid merkle trees. Read before T-081 in
  TODO/create-seed.md. GPL-3.0, do not copy.

reference/superseedr/src/torrent_manager/piece_manager.rs
reference/superseedr/src/torrent_manager/block_manager.rs
  Piece and block scheduling. Relevant to T-032 in TODO/performance.md.
  GPL-3.0, do not copy.

reference/superseedr/src/networking/utp.rs
  BEP 29. Relevant to T-101 in TODO/bep-coverage.md. GPL-3.0, do not copy.

reference/superseedr/src/token_bucket.rs
  Rate limiting. GPL-3.0, do not copy.

reference/superseedr/src/resource_manager.rs
  Bounding descriptors and memory. Relevant to T-011 and T-040.
  GPL-3.0, do not copy.

reference/superseedr/src/integrity_scheduler.rs
  Background re-verification. GPL-3.0, do not copy.

reference/superseedr/src/fs_atomic.rs
  Atomic file operations. Relevant to the Windows finalize problem, T-070.
  GPL-3.0, do not copy.

reference/superseedr/src/tracker/client.rs
  Tracker protocol. bit-cli-core/src/tracker.rs was written from BEP 3, BEP 15,
  BEP 23, and BEP 48 directly, not from this file. GPL-3.0, do not copy.

reference/superseedr/src/peer_manager.rs
  Peer lifecycle. Relevant to T-020. GPL-3.0, do not copy.

reference/superseedr/src/telemetry/
  The observability model. Relevant to T-091. GPL-3.0, do not copy.

reference/superseedr/fuzz/, proptest-regressions/, integration_tests/
  Their fuzzing and property-testing practice, and a pytest integration suite.
  Worth copying as a practice, not as code. GPL-3.0, do not copy.
```

## reference/FluxDown — AGPL-3.0, do not copy

The strongest boundary in this file. AGPL network-use disclosure would attach
to the netdisk deployment.

```
reference/FluxDown/AGENTS.md
  Their conventions. Read freely. AGPL-3.0, do not copy.

reference/FluxDown/Cargo.toml
  The workspace. Records that they vendor a patched librqbit at
  native/engine/vendor/librqbit via [patch.crates-io], and that the vendored
  copy has no webseed module, so it is a different patch set from the
  StarCitizenToolBox series in section 2.2. AGPL-3.0.

reference/FluxDown/native/cli/src/exit.rs
reference/FluxDown/native/cli/src/format.rs
  Exit code design and output formatting. Read before revisiting bit-cli's
  exit table. AGPL-3.0, do not copy.

reference/FluxDown/native/engine/src/downloader.rs
reference/FluxDown/native/engine/src/download_manager.rs
reference/FluxDown/native/engine/src/bt_downloader.rs
  The core download path. download_manager is where their catch_unwind
  recovery lives, which is the reason for decision 7.6. AGPL-3.0, do not copy.

reference/FluxDown/native/engine/src/segment_coordinator.rs
reference/FluxDown/native/engine/src/segment_advisor.rs
  Multi-source segment assignment. Read before implementing --split for web
  seeds, T-033 in TODO/performance.md. AGPL-3.0, do not copy.

reference/FluxDown/native/engine/src/speed_limiter.rs
reference/FluxDown/native/engine/src/route_health.rs
  Rate limiting and per-route health. Relevant to source cooldown policy.
  AGPL-3.0, do not copy.

reference/FluxDown/native/engine/src/bt_sparse.rs
reference/FluxDown/native/engine/src/bt_partfile.rs
  Sparse files and part files. Relevant to T-012. AGPL-3.0, do not copy.

reference/FluxDown/native/engine/tests/bt_parts_reseed.rs
reference/FluxDown/native/engine/tests/bt_file_selection.rs
reference/FluxDown/native/engine/tests/bt_sparse_add.rs
  Their test shapes for the three areas above. AGPL-3.0, do not copy.

reference/FluxDown/native/engine/examples/
  headless_download, peer_probe, prod_download. AGPL-3.0, do not copy.

reference/FluxDown/native/engine/vendor/librqbit
  Their patched librqbit. Apache-2.0 in its own right, and vendored inside an
  AGPL tree. Read the diff against upstream to see what they needed to change;
  copying from it would need the Apache-2.0 notice requirements in
  TODO/licensing.md, and is not currently planned.
```

---

### T-122 reference/ is not deleted at the end of Phase B

Source:      PROMPT.md section 2.4
Category:    licensing
Priority:    P2
Effort:      S
Status:      open

Problem:     `reference/` holds two copyleft source trees inside the working
             directory of an MIT project. It is gitignored, so it cannot be
             committed by accident, but it is one `git add -f` away.
Relevance:   The safest copy of an AGPL tree is the one that is not there.
Approach:    Delete it when Phase B closes. Everything worth keeping is in this
             file and in `TODO/licensing.md`.
Acceptance:  `reference/` does not exist and this file still answers what was
             read and why.
