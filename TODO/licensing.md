# Licensing

`bit-cli` ships under **MIT** alone (decision 7.1). This file records the
determination for every project it reads, copies from, or depends on, and the
per-subsystem provenance for anything learned from a copyleft reference.

Determinations were made on 2026-08-19 against the trees under `reference/`,
which is gitignored and is deleted in Phase B.

---

## The four references

| Project | License | May copy code? | Verified from |
| --- | --- | --- | --- |
| `kist` | MIT OR Apache-2.0 | Yes, under MIT | `LICENSE-MIT` in the fork base |
| FluxDown | AGPL-3.0 | **No** | `reference/FluxDown/LICENSE` |
| superseedr | GPL-3.0-or-later | **No** | `reference/superseedr/LICENSE`, SPDX headers on every file |
| intermodal | CC0-1.0 | Yes, adapt directly | `reference/intermodal/LICENSE` |

### kist

Dual licensed MIT OR Apache-2.0, so taking its code under MIT alone is
permitted: a disjunctive dual license lets the recipient pick either half.

The MIT half requires the original copyright notice to survive. The exact
holder string, read from the upstream `LICENSE-MIT`, is:

```
Copyright (c) 2026 Rabindra Dhakal
```

**This corrects the original prompt**, which said the holder was "QaidVoid".
That is the GitHub account; the copyright line names a person. Use the string
above verbatim in `LICENSE`.

`LICENSE-APACHE` is deleted, because the Apache half of the disjunction is not
being exercised.

### FluxDown, AGPL-3.0

AGPL is the strongest copyleft in common use. Copying AGPL code into an MIT
tree relicenses the result and imposes network-use source disclosure on anyone
running it as a service, which is exactly the netdisk deployment this tool is
built for. So the boundary is hard:

- Read it for architecture, sequencing, data flow, failure handling, and
  naming. Learn from it freely.
- Do not copy, translate, transliterate, or closely paraphrase it. Not one
  function. Not "with variables renamed". Reading a file and then writing your
  own from memory of its structure is still a derivative work if the result
  mirrors it line for line.
- Do not vendor, submodule, or ship any part of it.

**What was taken so far: nothing.** No FluxDown file has been read during this
work beyond the directory listing and the licence. The one conclusion carried
across is architectural and was already recorded in the prompt as decision 7.6:
FluxDown deliberately does not set `panic = "abort"` because their download
manager relies on `catch_unwind` to survive a task panic. `bit-cli` reaches the
same conclusion for the same reason, recorded in a comment in the release
profile. A shared conclusion about a Cargo profile setting is not a derivative
work of anything.

When a subsystem is built after reading FluxDown, it gets a row here naming
what was read, what was concluded, what was built instead, and the benchmark or
differential test that shows the result holds up (rule 0.10).

| Subsystem | What was read | What was concluded | What was built | Evidence |
| --- | --- | --- | --- | --- |
| (none yet) | | | | |

### superseedr, GPL-3.0-or-later

Same treatment as FluxDown. Every file carries an SPDX header, so there is no
ambiguity about what is being looked at.

**What was taken so far: nothing.** `src/networking/web_seed_worker.rs` is
named in `TODO/webseed.md` as the reference for Candidate A-prime and has not
been opened. `src/synthetic_load.rs` is named in `TODO/bench.md` for the shape
of a load generator (warmup, bounded disk budget, adaptive step search, periodic
metrics) and has not been opened either. Those are descriptions from the
prompt, not from the source.

The same table applies when that changes.

| Subsystem | What was read | What was concluded | What was built | Evidence |
| --- | --- | --- | --- | --- |
| (none yet) | | | | |

### intermodal, CC0-1.0

CC0 is a public domain dedication, not a copyleft license. Its code may be
copied, adapted, and shipped inside an MIT project with no license obligation.
This is the one reference where copying is allowed.

Two caveats worth stating plainly:

1. **CC0 explicitly does not grant patent rights.** Section 4(a) of the CC0
   1.0 text: "No trademark or patent rights held by Affirmer are waived,
   abandoned, surrendered, licensed or otherwise affected by this document."
   Copying CC0 code carries no patent licence, express or implied. For a
   BitTorrent metainfo tool the practical risk is negligible, but the fact is
   recorded because "public domain" is often read as covering more than it does.
2. **Attribution is not required and is given anyway.** `intermodal` is
   credited in `THIRD_PARTY.md` because it costs nothing and it is accurate.

What was actually adapted from `intermodal`:

| Subsystem | From | What was taken |
| --- | --- | --- |
| `crates/bit-cli/src/env.rs` | `src/env.rs` | The pattern of injecting args, working directory, and the three streams into the program rather than reading globals. This is what makes the headless parity requirement in rule 0.11 testable rather than aspirational. The `bit-cli` implementation is written against `bit-cli`'s own types; the idea is theirs. |
| `crates/bit-cli-core/src/torrent/lint.rs` | `src/subcommand/torrent/create.rs` | The `--allow <LINT>` model: refuse at creation on conditions that are legal but usually mistakes, and require the lint to be named to proceed. Ten lints, each with a stable name usable in a script. |
| `crates/bit-cli/src/cli.rs`, `create` | `src/subcommand/torrent/create.rs` | The flag surface: `--announce-tier` building BEP 12 tiers, `--sort-by KEY:ORDER`, `--no-created-by`, `--no-creation-date`, `--glob` with a leading `!` for exclusion, `-o -` for stdout. |
| `crates/bit-cli-core/src/units.rs` | `src/bytes.rs` | Size parsing and formatting with binary units. |
| `crates/bit-cli/src/output.rs` | `src/table.rs` | Aligned table output. |

---

## Dependencies

### librqbit, Apache-2.0

`librqbit` and its sibling crates (`librqbit-core`, `librqbit-bencode`,
`librqbit-peer-protocol`, and the rest) are **Apache-2.0 only**, not dual
licensed. Copyright 2021 Igor Katson.

As an ordinary crates.io dependency this is fine: an MIT source tree may depend
on an Apache-2.0 crate. Two obligations follow, and both are on the binary
distribution rather than on the source:

1. Ship the Apache-2.0 licence text with any binary distribution.
2. Ship any `NOTICE` content the upstream provides.

Both go in `THIRD_PARTY.md` and in the release archives.

**If the section 2.2 benchmark concludes a patched `librqbit` should be
vendored** (Candidate B), three more obligations attach, from Apache-2.0
section 4:

- The vendored subtree keeps its own `LICENSE` intact.
- Every file that is modified carries a prominent notice saying it was changed.
- A `CHANGES` file inside the vendored directory records the modifications.

No fork exists today. `TODO/webseed.md` T-001 is the decision gate.

### Everything else

`THIRD_PARTY.md` is generated mechanically (`cargo about` or
`cargo bundle-licenses`) so it cannot drift from `Cargo.lock`, and regenerated
in CI.

---

### T-120 THIRD_PARTY.md is not generated

Source:      PROMPT.md A6
Category:    licensing
Priority:    P1
Effort:      S
Status:      **done**

Problem:     No `THIRD_PARTY.md` exists, so no binary distribution can carry
             the Apache-2.0 text `librqbit` requires.
Relevance:   It is a licence obligation on every release, not a nicety.
Approach:    `cargo install cargo-about`, a `about.toml` naming the accepted
             licences, and a CI job that regenerates and fails on drift. The
             accept list is the check that matters: a new dependency under a
             copyleft licence has to fail the build, not appear quietly in a
             generated file.
Acceptance:  `THIRD_PARTY.md` exists, carries the full Apache-2.0 text for
             `librqbit`, and CI fails when a dependency's licence is not on the
             accept list.

**Done.** `THIRD_PARTY.md` is generated from `Cargo.lock` by `cargo about`
against `about.toml` and `about.hbs`, covers **310 crates**, and carries the
Apache-2.0 text attributed to `librqbit 9.0.0` along with every other licence
that ships. It opens by naming the two dependencies that need saying out loud:
`librqbit`, Apache-2.0 only rather than dual licensed, and `intermodal`, CC0
and credited anyway.

```bash
cargo about generate --config about.toml --output-file THIRD_PARTY.md about.hbs
```

The `notices` job in `ci.yml` runs that on every push and does two things with
it. Generation itself is the gate: `cargo about` refuses a licence that is not
in `about.toml`'s `accepted` list, which is the same list `deny.toml` allows.
And the crates the regenerated file covers are compared against the committed
one, so a dependency added without regenerating fails there.

**The comparison is the set of crates, not the bytes, and that is deliberate.**
Regenerating here today produces a file 835 lines longer than the committed
one, with `librqbit`'s Apache-2.0 text repeated in four blocks instead of
grouped into one. The crate sets are identical, all 310 of them, so nothing is
missing and nothing is extra: what changed is how `cargo about` groups
identical texts. A byte-exact check would fail on a `cargo about` release and
make the notice file something that has to be regenerated to make CI green,
which is how a notice file stops being read.

**One thing found while establishing that**, worth knowing before anyone
regenerates and sees a diff. `librqbit` ships no licence file at its crate
root, so `cargo about` falls back to scanning the crate directory, and that
directory holds a `webui/` tree. `librqbit`'s build script runs `npm install`
in it under the `webui` feature, which `bit-cli` does not enable but something
else on a shared machine may have: this one has **147 licence files under
`webui/node_modules`** for the scan to find. Moving them aside changes nothing
about the drift above, so they are not its cause, but they are exactly the
kind of thing that makes a local regeneration disagree with a clean one. The
`notices` job builds nothing before it generates for that reason.

### T-121 No cargo-deny configuration

Source:      PROMPT.md A6, A7
Category:    licensing
Priority:    P1
Effort:      S
Status:      **done**

Problem:     `cargo deny check` is required in CI for licences, advisories,
             bans, and sources. There is no `deny.toml`.
Relevance:   It is the mechanism that stops a transitive AGPL dependency
             arriving unnoticed, which for this project is a licence incident
             rather than a lint.
Approach:    Allow MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Unicode,
             CC0-1.0, and Zlib. Deny everything else, including every GPL and
             AGPL variant. Every exception carries a comment saying why.
Acceptance:  `cargo deny check` passes, and adding a GPL dependency makes it
             fail.

**Done, and both halves of the acceptance are now a command rather than an
assertion.** `deny.toml` allows fourteen permissive licences and denies
everything else. Two of them carry their reason inline:
`CDLA-Permissive-2.0`, which covers the Mozilla CA bundle and is a data
licence rather than a code one, and `Apache-2.0`, whose obligation is on the
binary distribution and is met by `THIRD_PARTY.md`.

```
$ cargo deny check
advisories ok, bans ok, licenses ok, sources ok
```

The second half is the one that needed building. A configuration that passes
says the tree is clean today; it does not say the gate would catch a copyleft
dependency arriving tomorrow. `scripts/check-licence-gate.ps1` proves it does:

```
$ pwsh -NoProfile -File scripts/check-licence-gate.ps1
checking this repository against its own deny.toml
  passes
checking a tree with one GPL-3.0-or-later dependency
  rejected: license is not explicitly allowed

verdict: the tree is clean and a GPL dependency is refused
```

The probe is a throwaway crate depending on a local crate whose manifest
declares `GPL-3.0-or-later` and whose `lib.rs` is empty, checked against this
repository's own `deny.toml`. Nothing is downloaded and no network is touched:
a licence is a string in a manifest, and that string is what `cargo deny`
reads. The check requires the failure to name the licence, so a gate that
failed for some other reason does not pass for the right one by accident.

The `deny` job in `ci.yml` runs `cargo deny check licenses advisories bans
sources` through `EmbarkStudios/cargo-deny-action` on every push.

