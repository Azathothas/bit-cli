# Intermodal (imdl)

> **Note on this copy.** Trimmed for BitTorrent research: `www/` (the website),
> `bin/` (demo/changelog/contributor generators), `tmp/`, `Dockerfile`,
> `justfile`, `rustfmt.toml` and `CONTRIBUTING` were removed, along with the
> installation, packaging, chat and release sections of this file. Retained:
> `src/`, `book/`, `benches/`, `CHANGELOG.md`.

> **Manifest sweep.** All `Cargo.toml` / `Cargo.lock` / `go.mod` / `go.sum` /
> `package.json` / lock files and JS build config were removed corpus-wide, so
> this tree is for reading, not building. Passages below that reference them,
> or that give build or install instructions, are upstream prose left as
> written.
Intermodal is a user-friendly and featureful command-line BitTorrent metainfo
utility. The binary is called `imdl` and runs on Linux, Windows, and macOS.

At the moment, creation, viewing, and verification of `.torrent` files is
supported. The `book/` directory in this repository is the authoritative documentation:
`book/src/bittorrent/bep-support.md` (BEP support matrix),
`piece-length-selection.md`, `piece-length.md`,
`udp-tracker-protocol.md`, `metainfo-utilities.md`,
`distributing-large-data-sets.md`.



## Usage

Online documentation is available in the book, hosted
[here](https://imdl.io/book/).

### Commands

Adding `--help` to any command will print help text about how to use that
command, including detailed information about any command-line arguments it
accepts.

So, to get information about `imdl torrent create`, run `imdl torrent create
--help`.

Additionally, the same help text is available online in
[the book](https://imdl.io/book/).

### Examples

The intro to [the book](https://imdl.io/book/) has a few simple examples. Check
[the FAQ](https://imdl.io/book/faq.html) for more complex usage examples.

### FAQ

The [FAQ](https://imdl.io/book/faq.html) covers a variety of specific
use-cases. If there's a use case you think should be covered, feel free to open
[an issue](https://github.com/casey/intermodal/issues/new).


## Benchmarks

Performance benchmarks can be run with:

```shell
$ cargo bench --features bench
```

The benchmark framework used is [`criterion`](https://github.com/bheisler/criterion.rs).

The bench targets themselves are in the `benches` directory. These targets call benchmarking functions in `src/benches.rs`, which are only enabled when the `bench` feature is enabled.

## Unstable Features

To avoid premature stabilization and excessive version churn, unstable features
are unavailable unless the `--unstable` / `-u` flag is passed, for example
`imdl --unstable torrent create .`. Unstable features may be changed or removed
at any time.
