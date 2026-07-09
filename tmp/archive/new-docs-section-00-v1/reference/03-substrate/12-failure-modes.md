# Failure Modes

> What can go wrong in `Substrate`, how each backend responds, and what the caller must do.
> Covers disk full, data corruption, and schema migration scenarios.

**Status**: Shipping
**Crate**: `roko-core`, `roko-fs`
**Depends on**: [Invariants](./11-invariants.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

`Substrate` propagates I/O and serialization errors via `SubstrateError`. Disk-full errors on
`put` are recoverable (the partial write is skipped on reload). Corruption is handled by
skipping malformed lines. Schema migrations require explicit migration tooling.

---

## Failure Catalogue

### F1 — Disk Full

**Scenario**: `put` is called, but the filesystem has no space.

**Behaviour**:
- `put` returns `SubstrateError::Io(std::io::ErrorKind::StorageFull)`.
- The partially-written line (if any) is skipped on next open (JSONL parser ignores malformed
  lines).
- The in-memory index is **not** updated — the record is as if it was never stored.

**Recovery**:
- Free disk space.
- Retry the `put`.
- If the store is over capacity, call `prune` to free space, then retry.

**Caller responsibility**: The cognitive loop should catch `SubstrateError::Io` in the STORE
step and emit a warning metric (`substrate.write.error`). The loop should not abort — one
failed write is not fatal.

---

### F2 — Corruption (Malformed Line)

**Scenario**: A line in the JSONL file is truncated or contains invalid JSON (e.g., from a
power failure mid-write).

**Behaviour**:
- On `FileSubstrate::open`, malformed lines are skipped with a `tracing::warn!` log.
- Valid lines before and after the malformed line are loaded normally.
- `len()` reflects only the successfully loaded records.

**Recovery**:
- Automatic — the store self-heals on next open by skipping the bad line.
- Run `substrate.compact()` to rewrite the file without the malformed line.
- If many lines are corrupted (catastrophic failure), restore from backup.

---

### F3 — Schema Migration

**Scenario**: A new version of `Engram` adds a field. Old JSONL records do not have this
field.

**Behaviour**:
- `serde`'s `#[serde(default)]` attribute on new fields means old records deserialise
  successfully with the new field set to its default value.
- Records written by newer code that remove a field are readable by older code if the
  removed field has `#[serde(skip_serializing_if = "is_none")]`.

**Breaking migrations** (rename a field, change a type):
- There is no automatic migration.
- A migration script must read the old JSONL, transform each record, and write a new JSONL.
- The migration script is in `roko-fs/tools/migrate.rs`.

---

### F4 — `prune` Interrupted (Crash Mid-Prune)

**Scenario**: The process crashes during a `prune` call.

**Behaviour** (FileSubstrate):
- The JSONL file is not modified during `prune` — `prune` only updates the in-memory index.
- The removed records are still in the file.
- On next open, all records (including those that were pruned) are reloaded.
- The next `prune` call re-evicts them.

**Net effect**: No data loss. The only cost is re-loading records that will be immediately
re-pruned.

---

### F5 — File Deleted While Open

**Scenario**: Another process deletes the JSONL file while `FileSubstrate` has it open.

**Behaviour**:
- Reads continue to work from the in-memory index (the file descriptor remains valid on POSIX
  until closed).
- The next `put` succeeds if the file descriptor is still open for writing (POSIX).
- On next open, the file is missing → `FileSubstrate::open` returns
  `SubstrateError::Io(NotFound)`.

**Recovery**: Restore the file from backup or accept data loss and recreate.

---

### F6 — `MemorySubstrate` OOM

**Scenario**: `put` is called so many times that the `HashMap` exhausts available RAM.

**Behaviour**: Rust's allocator panics (or returns `null` in `no_std` contexts).

**Recovery**: Set `max_capacity` on `MemorySubstrate` to prevent unbounded growth. Call
`prune` regularly. If the agent genuinely needs more records than fit in RAM, switch to
`FileSubstrate`.

---

<!-- ADDED -->
### F7 — Fingerprint Dimension Mismatch

**Scenario**: An `Engram` was stored with a fingerprint of dimension D1, but the current
build uses D2 ≠ D1.

**Behaviour**:
- `query_similar` returns `SubstrateError::Backend("fingerprint dimension mismatch")`.
- `put` and `get` continue to work normally (fingerprints are stored as opaque byte arrays).

**Recovery**: Re-derive all fingerprints using `substrate.reindex()` (planned, not yet
shipped). Until then, the HDC index must be rebuilt by clearing and re-inserting all records.

---

## Error Propagation Guidance

| Call site | On `SubstrateError::Io` | On `SubstrateError::Serialization` |
|---|---|---|
| Cognitive loop STORE step | Log warning, continue loop | Log error, drop the engram |
| Cognitive loop RECALL step | Log warning, use empty recall | Log error, use empty recall |
| Explicit `prune` call | Log warning, retry later | Should not occur — prune only removes |
| Tests | `unwrap()` / `expect()` is fine | Same |

---

## See Also

- [Invariants](./11-invariants.md)
- [Performance](./13-performance.md)
- [Pruning](./06-pruning.md) — F4 in depth

## Open Questions

- Should `FileSubstrate` support a WAL (write-ahead log) mode for stronger crash safety?
- Should a `SubstrateError::Corruption` variant be added to distinguish malformed data from
  transient I/O errors?
