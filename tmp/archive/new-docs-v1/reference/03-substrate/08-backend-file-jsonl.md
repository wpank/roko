# Backend: JSONL File (`FileSubstrate`)

> `FileSubstrate` is the primary durable backend. It appends `Engram` records as newline-
> delimited JSON to a single `.jsonl` file, builds an in-memory index on open, and handles
> compaction.

**Status**: Shipping
**Crate**: `roko-fs`
**Depends on**: [Backends Overview](./07-backends-overview.md), [Trait Surface](./01-trait-surface.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

`FileSubstrate` is a simple, inspect-able durable backend. Open a file; each `put` appends a
JSON line; `get` and `query` serve from an in-memory index. On restart the index is rebuilt
from the file in O(n). Suitable for single-process agents with up to ~1 million records.

---

## Design Rationale

JSONL was chosen because:
1. **Inspect-ability** — any developer can `cat substrate.jsonl | jq .` to read the store.
2. **Append semantics** — no random-write fragmentation; writes are sequential (fast on SSD).
3. **Crash safety** — if the process dies mid-write, the partial line is ignored on reload
   (the parser skips malformed lines).
4. **Portability** — no embedded database dependency.

The downside is linear scan for `query` (mitigated by the in-memory index) and growth without
bound until compaction runs.

---

## File Format

Each line in the `.jsonl` file is a JSON-serialised `Engram`:

```
{"hash":"sha256:abc123...","kind":"Fact","body":{"text":"..."},"score":{...},"decay":{...},"fingerprint":"...","created_at":1713456789}\n
{"hash":"sha256:def456...","kind":"Episode","body":{...},...}\n
...
```

Fingerprints are stored as hex-encoded strings (for readability) or base64 (for compactness
— configurable). The HDC vector is included in full.

---

## Opening a FileSubstrate

```rust
// source: crates/roko-fs/src/lib.rs
use roko_fs::FileSubstrate;
use std::path::Path;

// Opens existing file or creates a new one.
let mut substrate = FileSubstrate::open(Path::new("./memory.jsonl"))?;
println!("Loaded {} records", substrate.len());
```
<!-- source: crates/roko-fs/src/lib.rs -->

On open, `FileSubstrate`:
1. Reads and parses every line in the file.
2. Skips malformed lines (logs a warning per skipped line).
3. Builds a `HashMap<ContentHash, Engram>` (the primary index) and an HDC index array
   (`Vec<(ContentHash, HdcFingerprint)>`) for `query_similar`.

Startup cost is O(n) reads + O(n) deserialisation + O(n) HDC index build.

---

## Write Path

`put(engram)`:
1. Serialise `engram` to a JSON string.
2. Append `json_line + "\n"` to the file (single `write` syscall + optional `fsync`).
3. Insert/replace in the in-memory `HashMap` and HDC index.

`fsync` is configurable (default: off for speed, on for maximum durability). With `fsync`
disabled, a power failure can lose the last ~4 KB of writes.

---

## Read Path

`get(hash)` — O(1) hashmap lookup.

`query(filter)` — O(n) scan of the in-memory hashmap, applying each filter field. Returns
matching records.

`query_similar(fp, k)` — O(n · D/64) scan of the HDC index array.

---

## Compaction

The JSONL log grows with every `put` (even updates write a new line; the old line is not
removed). Compaction rewrites the file to include only the current live records:

```rust
// source: crates/roko-fs/src/lib.rs
substrate.compact()?;
```
<!-- source: crates/roko-fs/src/lib.rs -->

Compaction:
1. Writes all in-memory records to a temp file.
2. Renames temp → original (atomic on POSIX).
3. The old file is gone; no partial state is visible to readers.

Compaction should be called after `prune` (which removes records from the in-memory index but
leaves tombstones in the file).

---

## Configuration

<!-- ADDED -->

| Parameter | Type | Default | Description |
|---|---|---|---|
| `path` | `PathBuf` | (required) | Path to the `.jsonl` file |
| `fsync_on_write` | `bool` | `false` | Call `fsync` after each append |
| `max_capacity` | `usize` | `usize::MAX` | Record count ceiling for pruning |
| `decay_floor` | `f32` | `0.0` | Prune records with balance ≤ this |
| `fingerprint_encoding` | `Hex \| Base64` | `Base64` | How to store fingerprints in JSON |

---

## Failure Modes

| Failure | Behaviour |
|---|---|
| Disk full on `put` | Returns `SubstrateError::Io`. The partially-written line is skipped on next open. |
| Malformed line on open | Skipped with a warning; remaining records are loaded. |
| File deleted while open | In-memory index still serves reads; next write fails with `SubstrateError::Io`. |
| Compaction interrupted | The temp file is left behind; the original is unmodified. |

---

## See Also

- [Backend: In-Memory](./09-backend-in-memory.md)
- [Pruning](./06-pruning.md)
- [Performance](./13-performance.md) — JSONL benchmarks

## Open Questions

- Should compaction run automatically after every `prune`, or always be called explicitly?
- Is a binary-format variant (MessagePack, FlatBuffers) worth shipping for agents with
  very large records?
