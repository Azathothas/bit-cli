# Short flags

Short flags are scarce and a wrong one is worse than none, because a script
written from `aria2` muscle memory will silently do something else.

Two rules:

1. A short flag is assigned only where `aria2` already assigns that letter to
   the same concept, or where the letter is unclaimed by `aria2` and the
   meaning is obvious.
2. An `aria2` letter is never reassigned to a different concept.

`crates/bit-cli/src/cli.rs` carries a test,
`every_short_flag_is_documented_in_the_flags_table`, that reads this file's
table and fails when the two disagree. Adding a short flag without adding a row
here fails the build. Two more tests sit beside it: `no_short_flag_is_defined_twice`
rejects a letter used twice in one command, and `short_flags_never_contradict_aria2`
rejects an `aria2` letter reassigned to a different concept.

## The -v and -V question

`aria2` uses `-v` for `--version` and `-V` for `--check-integrity`. Most modern
tools use `-v` for verbosity.

**`bit-cli` uses `-v` for verbosity and gives `--version` no short form.**

`-V` keeps its `aria2` meaning, `--check-integrity`. Taking `-V` for
`--version` would have been the other way to resolve it, and it is the worse
one: it reassigns an `aria2` letter to a different concept, which rule 2
forbids. So `bit-cli -V` on a torrent means "check the integrity of what is on
disk", exactly as `aria2c -V` does, and `bit-cli --version` is spelled in full.

`clap` assigns `-V` to `--version` by default, so `disable_version_flag` is set
in `cli.rs` to stop it.

## Assigned

| Flag | Long form | Scope | `aria2` | Note |
| --- | --- | --- | --- | --- |
| `-c` | `--continue` | download | `-c` continue | Same concept. |
| `-d` | `--dir` | global | `-d` dir | Same concept. |
| `-h` | `--help` | global | `-h` help | Universal. |
| `-j` | `--max-concurrent-downloads` | download | `-j` max concurrent downloads | Same letter, narrower meaning: `bit-cli` has no queue across invocations, so this is parallelism inside one run. |
| `-l` | `--log-file` | global | `-l` log | Same concept. |
| `-O` | `--index-out` | download | `-O` index out | Same concept. Not implemented yet, see `TODO/cli-surface.md` T-116. |
| `-o` | `--out` | download | `-o` out | Same concept. |
| `-o` | `--output` | create, edit, man | unclaimed in this position | `-` means stdout, following `intermodal`. |
| `-q` | `--quiet` | global | `-q` quiet | Same concept. |
| `-u` | `--max-upload-rate` | download, seed | `-u` max upload limit | Same concept. |
| `-V` | `--check-integrity` | download | `-V` check integrity | Same concept. See above. |
| `-v` | `--verbose` | global | `-v` version | **Deliberate divergence.** See above. |

## Reserved and not assigned

These letters mean something specific in `aria2`. `bit-cli` does not use them
yet, and when it does they keep the meaning below.

| Flag | `aria2` meaning | Status here |
| --- | --- | --- |
| `-D` | daemon | Never. Phase C, decision 7.4. |
| `-i` | input file | Reserved. `TODO/cli-surface.md` T-114. |
| `-k` | min split size | Reserved. `TODO/performance.md` T-033. |
| `-M` | metalink file | Reserved. A Metalink is a positional source here, the way `.torrent` is: `bit-cli download release.meta4`. |
| `-m` | max tries | Reserved. |
| `-P` | parameterized uri | Reserved. |
| `-R` | remote time | Reserved. |
| `-S` | show files | Reserved. `bit-cli files` covers it as a verb. |
| `-s` | split | Reserved. `TODO/performance.md` T-033. |
| `-T` | torrent file | Reserved. `bit-cli` takes a positional source instead. |
| `-t` | timeout | Reserved. `--timeout` is global and long-only for now. |
| `-U` | user agent | Reserved. |
| `-x` | max connection per server | Reserved. `TODO/performance.md` T-033. |
| `-Z` | force sequential | Reserved. |

## Unclaimed by aria2

Free to use where the meaning is obvious. None is assigned today:

```
-A -B -C -E -F -G -H -I -J -K -L -N -Q -W -X -Y
-a -b -e -f -g -n -p -r -w -y -z
```
