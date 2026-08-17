# 32 — HTTP / Persistence Follow-Ups

`roko-serve` has good patterns (StateHub, atomic JSON writes, ArcSwap),
~6K LOC, ~85 routes. The audit identifies persistence duplication and
no transactional multi-file writes.

Source: subsystem-audits/http-persistence/AUDIT.md.

---

## Anti-Patterns

1. **No raw `std::fs::write` for JSON state.** Use the atomic
   write-then-rename helper.
2. **No two-file invariant without transaction.** If state spans two
   files, write to a temp dir then atomic rename.
3. **No serde manual derives where derive works.** Easy to drift.
4. **No persistence path that bypasses the StateHub.** New persisted
   state is a `DashboardSnapshot` field.

---

## Plan

### [ ] HP-1: Audit duplicate persistence helpers

```bash
rg 'fn write_json|fn save_json|atomic_write|write_atomically' crates/roko-serve/ crates/roko-runtime/
```

If multiple helpers exist (one per crate), consolidate into one shared
helper in `roko-runtime` or `roko-core`.

### [ ] HP-2: Add transactional multi-file write

**File**: `crates/roko-runtime/src/persistence.rs` (new or extend)

```rust
pub struct AtomicWriteSet {
    writes: Vec<(PathBuf, Vec<u8>)>,
}

impl AtomicWriteSet {
    pub fn add(&mut self, path: PathBuf, contents: Vec<u8>) { /* ... */ }

    pub async fn commit(self) -> Result<(), std::io::Error> {
        // Stage to temp dir
        let tmp = tempfile::TempDir::new()?;
        for (path, contents) in &self.writes {
            let staged = tmp.path().join(path.file_name().unwrap());
            tokio::fs::write(&staged, contents).await?;
        }
        // Atomic rename each into place
        for (path, _) in &self.writes {
            let staged = tmp.path().join(path.file_name().unwrap());
            tokio::fs::rename(&staged, path).await?;
        }
        Ok(())
    }
}
```

Use for state that crosses files (e.g. `executor.json` + `gates.json`).

### [ ] HP-3: Document the StateHub contract

The audit says "StateHub pattern is good" but undocumented. Add doc
comments explaining:

- Single writer per snapshot field.
- Many readers via `watch::Receiver`.
- Broadcast channel for transient events (DashboardEvent).
- Snapshots are deep cloned; readers don't hold writer locks.

### [ ] HP-4: Persistence consolidation

Move all `roko-serve`-specific persistence (jobs, plans, agents, etc.)
under one module: `crates/roko-serve/src/persistence/`. Subdivide by
domain.

---

## Combined Verification

```bash
cargo test -p roko-serve persistence --lib
cargo test -p roko-runtime atomic_write --lib
```

**Estimated effort**: 8-15 hours.
