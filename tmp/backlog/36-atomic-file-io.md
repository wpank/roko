# Atomic File I/O

**Priority**: P2 — data corruption on crash, silent state loss
**Size**: S (1 day)
**Crates**: `crates/roko-cli/`, `crates/roko-learn/`, `crates/roko-fs/`

---

## Problem

State persistence across the codebase uses non-atomic file writes. Files like
`.roko/learn/cascade-router.json`, `.roko/learn/gate-thresholds.json`,
`.roko/state/state-snapshot.json`, and PRD files are written with direct `fs::write()`
calls. If the process crashes or is killed mid-write (SIGKILL, OOM, power loss), these
files can be left truncated or corrupted. This breaks resume, learning state, and
configuration on the next startup.

The write-to-temp-then-rename pattern (atomic write) is used in some places but not
consistently. The fix is to centralize this into a single utility and replace all
state-persistence writes.

---

## Where to look

- `crates/roko-fs/src/file_substrate.rs` — existing substrate I/O, may already have
  partial atomic patterns worth reusing
- `crates/roko-cli/src/runner/persist.rs` — state snapshot writes
- `crates/roko-cli/src/runner/state.rs` — runner state persistence
- `crates/roko-learn/src/costs_db.rs` — cascade router JSON persistence
- `crates/roko-learn/src/costs_log.rs` — cost log writes
- `crates/roko-learn/src/episode_logger.rs` — episode JSONL appends
- `crates/roko-learn/src/feedback_service.rs` — gate threshold JSON writes
- `crates/roko-learn/src/provider_health.rs` — provider health JSON writes
- `crates/roko-learn/src/runtime_feedback.rs` — efficiency JSONL writes

Search for `fs::write`, `std::fs::write`, and `File::create` across these crates to
find all direct write sites.

---

## What to do

**Step 1.** Create a utility function in `crates/roko-fs/` (or an appropriate shared
location):

```rust
/// Write `content` to `path` atomically.
/// Writes to a `.tmp` sibling, then renames over the target.
/// On Unix, rename is atomic within the same filesystem.
pub fn atomic_write(path: &Path, content: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, content)?;
    fs::rename(&tmp, path)?;
    Ok(())
}
```

Consider also an `atomic_write_async` variant for tokio contexts using
`tokio::fs::write` + `tokio::fs::rename`.

**Step 2.** Replace all state-persistence `fs::write` calls with the atomic utility.
Key files to protect (ordered by corruption impact):

1. `state-snapshot.json` — plan runner resume state; corruption loses all plan progress
2. `cascade-router.json` — learned model routing weights; corruption resets routing
3. `gate-thresholds.json` — adaptive gate EMA thresholds; corruption resets baselines
4. `provider-health.json` — provider health registry; corruption loses health history
5. `section-effects.json` — cross-cut functor state
6. `efficiency.jsonl` — per-turn efficiency events (append-only, but truncation loses tail)
7. `episodes.jsonl` — agent episode log (append-only, same truncation risk)

For append-only JSONL files, the atomic pattern is different: write the new line, then
`fsync` the file descriptor. Do not rewrite the entire file on each append.

**Step 3.** Handle the `.tmp` file cleanup edge case: on startup, if a `.tmp` sibling
exists but the target does not, rename it in (the write succeeded but rename failed). If
both exist, delete the `.tmp` (the previous write was incomplete).

---

## Acceptance criteria

- [ ] `atomic_write` utility exists and is tested (unit test: write, verify, simulate
  crash by skipping rename)
- [ ] `state-snapshot.json` writes use atomic write
- [ ] `cascade-router.json` writes use atomic write
- [ ] `gate-thresholds.json` writes use atomic write
- [ ] `provider-health.json` writes use atomic write
- [ ] JSONL appends use `fsync` after write
- [ ] Startup handles orphaned `.tmp` files gracefully
- [ ] All existing tests pass (`cargo test --workspace`)

### Verify

Kill roko mid-write (e.g., `kill -9` during `plan run` while state is being persisted).
Confirm state files are either the old version or the new version, never
corrupted/truncated. A simple test: start a plan run, send SIGKILL after 2 seconds,
restart with `--resume-plan`, confirm it resumes from valid state.

### Not in scope

- Multi-process locking (backlog item 37)
- JSONL rotation or compaction (already handled by `roko-fs` GC)
- Database-backed persistence (future consideration)

---

**Origin**: redesign-plan.md (Phase 1.3), infrastructure-audit.md
