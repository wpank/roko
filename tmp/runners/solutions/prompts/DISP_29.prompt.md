# DISP_29: Expose Provider Health via HTTP API

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#disp-29`](../ISSUE-TRACKER.md#disp-29)
- Source: `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` — Task 3.29
- Priority: **P3**
- Effort: 3 hours
- Depends on: `DISP_15` (source 3.15)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: DISP_29 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The serve runtime at `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/providers.rs` already has provider-related routes. The serve state at `state.rs:377` already holds a `ProviderHealthTracker`. A `/api/providers/health` endpoint should expose the tracker's state.

## Exact Changes

1. Add `GET /api/providers/health` route handler
2. Query `state.provider_health` for all known providers
3. Return JSON:
   ```json
   {
     "providers": {
       "anthropic_api": { "state": "healthy", "consecutive_failures": 0, "total_attempts": 42, "total_successes": 41 },
       "cerebras_api": { "state": "unhealthy", "consecutive_failures": 3, "total_attempts": 10, "total_successes": 7 }
     }
   }
   ```
4. Add `pub fn snapshot(&self) -> HashMap<String, ProviderStatusSnapshot>` to `ProviderHealthTracker` to export current state
5. Wire the route in the serve router

## Design Guidance

The endpoint should be read-only and cheap. The `ProviderHealthTracker` uses a `RwLock` internally, so reading is non-blocking. Return only the fields that are useful for monitoring: state, failure count, success count, last success/failure timestamps.

## Write Scope

- `crates/roko-serve/src/routes/providers.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `curl localhost:6677/api/providers/health` returns valid JSON
- [ ] Health data reflects actual provider state

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: DISP_29 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `curl localhost:6677/api/providers/health` returns valid JSON
- Health data reflects actual provider state
- No files outside the Write Scope are modified.
- Commit message contains `tracker: DISP_29 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
