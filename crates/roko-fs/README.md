# roko-fs

Filesystem-backed `Substrate` for Roko. Append-only JSONL persistence with an in-memory query index.

## Install

```toml
[dependencies]
roko-fs = { path = "../roko-fs" }
roko-core = { path = "../roko-core" }
```

## What's inside

- **`FileSubstrate`** — persists signals to `<dir>/signals.jsonl`, replays on startup, serves queries from an in-memory index.
- **`JsonlTraceSink`** — append-only trace log for debugging.
- **`MetricsLog`** — structured counters written alongside signals.

## Example

```rust
use roko_fs::FileSubstrate;
use roko_core::{Signal, Substrate, Context, Query, Kind};

let sub = FileSubstrate::open(".roko").await?;   // replays existing log
sub.put(signal).await?;                          // appends one JSONL line

// State survives restart:
drop(sub);
let reopened = FileSubstrate::open(".roko").await?;
let all = reopened.query(&Query::of_kind(Kind::Episode), &Context::now()).await?;
```

## Why JSONL + in-memory index

- **Append-only** writes are crash-safe — a partial last line is skipped on replay.
- **JSONL** is grep-able, diff-able, human-readable. You can inspect `.roko/signals.jsonl` with `jq` and see exactly what happened.
- **In-memory index** gives the same query latency as `MemorySubstrate` (tens of MB per million signals).
- Swap in SQLite/sled later behind the same `Substrate` trait without touching callers.

## Compaction

`FileSubstrate::compact` rewrites the log, dropping superseded signals (e.g. ones fully subsumed by derived children with decayed-to-zero scores). Callers decide the policy; the substrate just enforces append-only semantics between compactions.

## Trace sinks

`JsonlTraceSink` writes structured trace events to a parallel file. Use for post-hoc debugging of loop_tick runs without interleaving with signals.
