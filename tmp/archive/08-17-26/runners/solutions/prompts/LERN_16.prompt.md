# LERN_16: Wire Dream Cycle Automatic Trigger in `roko serve`

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#lern-16`](../ISSUE-TRACKER.md#lern-16)
- Source: `tmp/solutions/roko/tasks/07-LEARNING-FEEDBACK.md` — Task 7.16
- Priority: **P2**
- Effort: 3 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: LERN_16 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`start_dream_loop()` (at `dreams.rs:39`) is fully implemented: it spawns a background tokio task that checks `DreamLoopConfig.auto_dream`, runs `build_dream_cycle()`, and loops with `DREAM_CHECK_INTERVAL` (60s). `DreamLoopConfig` has `auto_dream: bool`, `interval`, `min_episodes_for_dream`, `agent` (model config).

The function exists but is never called from `roko serve` startup. The `roko-serve` library has the function exported but no startup code invokes it.

## Exact Changes

1. In `roko serve` startup (likely in `roko-serve/src/lib.rs` or the route builder that creates `AppState`), after constructing `AppState`, call `start_dream_loop(Arc::clone(&state), dream_config)`.
2. Load `DreamLoopConfig` from `roko.toml` config (add `[dreams]` section support if not present):
   ```toml
   [dreams]
   auto_dream = true
   interval_secs = 3600
   min_episodes = 20
   budget_usd = 0.10
   model = "claude-haiku-3-5"
   ```
3. Default `auto_dream = false` so existing deployments are not affected.
4. Store the `JoinHandle` from `start_dream_loop` so it can be cancelled on server shutdown.
5. Surface dream status in `roko status` output (last dream run timestamp, episode count since last dream).

## Write Scope

- `crates/roko-serve/src/lib.rs`
- `crates/roko-serve/src/dreams.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/07-LEARNING-FEEDBACK.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Start `roko serve` with `auto_dream = true`, verify dream loop starts (visible in logs)
- [ ] Dream cycle runs after sufficient episodes accumulate
- [ ] `auto_dream = false` (default) does not start the loop
- [ ] Server shutdown cancels the dream loop cleanly

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: LERN_16 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Start `roko serve` with `auto_dream = true`, verify dream loop starts (visible in logs)
- Dream cycle runs after sufficient episodes accumulate
- `auto_dream = false` (default) does not start the loop
- Server shutdown cancels the dream loop cleanly
- No files outside the Write Scope are modified.
- Commit message contains `tracker: LERN_16 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
