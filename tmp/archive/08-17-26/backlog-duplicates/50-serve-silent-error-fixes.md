# Serve Silent Error Fixes

**Priority**: P2 — critical startup errors silently swallowed
**Size**: S (½ day)
**Crate**: `crates/roko-serve/`

---

## Problem

`crates/roko-serve/src/lib.rs` has 8 instances of `let _ =` that silently discard
errors. Two are critical:

1. **Line ~1084**: `let _ = state.state_hub.bootstrap_from_workdir()` — StateHub
   bootstrap recovery errors are silently ignored at startup. If the bootstrap fails,
   the server runs with no historical state, but no warning is emitted.

2. **Line ~3165**: `let _ = start_event_source_group()` — Event source startup errors
   are silently dropped. If configured event sources fail to initialize (bad config,
   missing credentials), the server runs without them and the operator has no
   indication.

The remaining 6 are lower severity (stderr pipe writes, async task joins, shutdown
signal waits, test cleanup) but should still be reviewed.

---

## Where to look

- `crates/roko-serve/src/lib.rs` — all 8 `let _ =` sites

---

## What to do

**Step 1.** For the two critical sites (StateHub bootstrap, event source startup),
replace `let _ =` with proper error handling:

```rust
// Before:
let _ = state.state_hub.bootstrap_from_workdir();

// After:
if let Err(e) = state.state_hub.bootstrap_from_workdir() {
    tracing::warn!(error = %e, "StateHub bootstrap failed; starting with empty state");
}
```

**Step 2.** For each of the remaining 6 sites, determine whether the error should be:
- Logged at `warn` level (if the caller should know about the failure)
- Left as `let _ =` with a comment explaining why the error is intentionally ignored

**Step 3.** Add `// intentionally ignored: <reason>` comments to any `let _ =` that
is kept after review.

---

## Acceptance criteria

- [ ] StateHub bootstrap errors logged at `warn` level
- [ ] Event source startup errors logged at `warn` level
- [ ] All 8 `let _ =` sites reviewed and either fixed or annotated
- [ ] No new `let _ =` sites on `Result` types without a comment
- [ ] All existing tests pass (`cargo test -p roko-serve`)

---

**Origin**: productionizing audit (2026-08-13)
