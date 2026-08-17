# STAB_11: Wire CascadeRouter to live callers

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-11`](../ISSUE-TRACKER.md#stab-11)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.11
- Priority: **P1**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_11 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`resolve_effective_model()` in `model_selection.rs` at line 140 accepts `Option<&CascadeRouter>`
as its 4th parameter. Every live caller passes `None` (verified by grep: no non-None calls
outside test code). The CascadeRouter is a LinUCB contextual bandit with 4-stage routing,
persistence at `.roko/learn/cascade-router.json`, and cost spike detection.

## Exact Changes

1. Create a helper function `load_or_create_cascade_router(roko_dir: &Path) -> CascadeRouter`:
   - Try to load from `.roko/learn/cascade-router.json`
   - If file doesn't exist or is corrupt, create a new router with default config
   - Log the loaded state (number of observations)
2. In `run.rs` (`cmd_run` or equivalent):
   - Load router at startup: `let router = load_or_create_cascade_router(&roko_dir);`
   - Pass `Some(&router)` to `resolve_effective_model()`
   - After each model call completes, call `router.observe(model, role, success, cost, latency)`
   - Persist router on graceful shutdown
3. In `chat_session.rs`:
   - Load router in session setup
   - Pass to model resolution
   - Observe after each turn
4. In `runner/event_loop.rs`:
   - Load router before entering the event loop
   - Pass to dispatch context
   - Observe after each task completion
   - Persist during periodic flush (every 5 tasks or 60 seconds)
5. Add the router to `ServiceFactory::build()` return type so it can be shared.

## Design Guidance

The router should be a shared `Arc<Mutex<CascadeRouter>>` to allow observation from multiple
async tasks. Persistence should use atomic file writes (write to `.tmp`, rename) to avoid
corruption on crash. The periodic flush interval should match the existing executor state
flush.

## Write Scope

- `crates/roko-cli/src/model_selection.rs`
- `crates/roko-cli/src/run.rs`
- `crates/roko-cli/src/chat_session.rs`
- `crates/roko-cli/src/runner/event_loop.rs`
- `crates/roko-learn/src/cascade_router.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `roko run "hello"` loads/creates cascade router
- [ ] After 2+ runs, `.roko/learn/cascade-router.json` has `observations > 0`
- [ ] Router observations include model name, success/failure, cost
- [ ] `roko plan run` also records observations

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_11 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko run "hello"` loads/creates cascade router
- After 2+ runs, `.roko/learn/cascade-router.json` has `observations > 0`
- Router observations include model name, success/failure, cost
- `roko plan run` also records observations
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_11 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
