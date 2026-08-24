# Consuming the output from a script

Two shapes, and they are for different jobs.

`--json` writes one document when the run ends. `--jsonl` writes one JSON
object per line as things happen. Both go to stdout, both are UTF-8 with no
BOM, and neither is affected by whether stdout is a terminal.

**Select by `type` or by `kind`, never by position.** A new event type can
appear in any release and a field can be added to any document. Nothing
promises that the third line is the one you want.

## What a run emits

From a real download of a three file, 1.47 MiB torrent from a loopback seeder:

```bash
bit-cli download album.torrent --dir out --report-interval 2s --jsonl
```

```text
      1 "type":"session_start"
      1 "type":"torrent_added"
      1 "type":"metadata_resolved"
     31 "type":"progress"
      1 "type":"torrent_completed"
      1 "type":"session_end"
```

The first line, in full:

```json
{"at":"2026-08-24T09:26:36.647Z","directory":"...\\leech","listen_addr":"[::]:6881","max_concurrent_downloads":1,"seq":0,"sources":1,"type":"session_start"}
```

Three fields on every event and they are the ones a consumer relies on:

| field | what it is for |
| --- | --- |
| `type` | what happened. Switch on this |
| `seq` | a monotonic counter from 0. A gap means a line was lost |
| `at` | ISO 8601 UTC with millisecond precision |

`listen_addr` on `session_start` is how a script learns the port when
`--port 0` was passed. Every acceptance script in `scripts/` reads it from
there rather than from a socket table, because a uTP listener is a UDP socket
and `Get-NetTCPConnection` cannot see it.

## Waiting for the port, in PowerShell

```powershell
$seed = Start-Process -FilePath bit-cli -NoNewWindow -PassThru -ArgumentList @(
    "seed", "album.torrent", "--data", "payload", "--port", "0", "--jsonl"
) -RedirectStandardOutput seed.out

$listen = $null
$deadline = (Get-Date).AddSeconds(60)
while (-not $listen -and (Get-Date) -lt $deadline) {
    foreach ($line in @(Get-Content seed.out -ErrorAction SilentlyContinue)) {
        if (-not $line -or -not $line.Trim().StartsWith("{")) { continue }
        try { $event = $line | ConvertFrom-Json } catch { continue }
        if ($event.listen_addr) { $listen = $event.listen_addr; break }
    }
    if (-not $listen) { Start-Sleep -Milliseconds 200 }
}
```

Two things that look like noise and are not. The `try`/`catch` around
`ConvertFrom-Json` handles a partially written final line, which happens when
the file is read while the process is still appending. And the loop waits on
**the condition** rather than sleeping a guessed number of seconds, which is
the rule in [`../../TODO/RULES.md`](../../TODO/RULES.md) section 5 that has
cost this repository seven red CI jobs.

## Reading the final document

```bash
bit-cli download album.torrent --dir out --json
```

Every byte figure is an object rather than a bare number:

```json
{
  "downloaded": { "bytes": 1543000, "human": "1.47 MiB" },
  "from_peers": { "bytes": 1543000, "human": "1.47 MiB" },
  "from_web_seeds": { "bytes": 0, "human": "0 B" },
  "from_resume": { "bytes": 0, "human": "0 B" }
}
```

**Read `.bytes` and never the pair.** The formatted string is beside the
integer rather than instead of it, which is
[`../../TODO/RULES.md`](../../TODO/RULES.md) section 5's output rule. A
consumer that parses `"1.47 MiB"` is parsing a presentation decision.

The three `from_*` figures add up to `downloaded` and are what says where the
bytes came from. A resumed download that charged its existing bytes to the
swarm was a real defect here, and these three fields are what made it visible.

## Exit codes are the other half

```bash
bit-cli download --not-a-flag
```

exits **2**, usage. Every code, its meaning, and whether a retry could succeed
are in [`../../man/bit-cli.json`](../../man/bit-cli.json) under `errors`, and
in [`../exit-codes.md`](../exit-codes.md).

**Read the exit code from the process that produced it, unpiped.** A check
piped into anything reports the pipeline's status.

## The schema

[`../schema.md`](../schema.md) documents every field of every document kind,
and `--schema-version` prints the version the binary emits. A field is added
without a version bump; a field is never removed or retyped without one.
