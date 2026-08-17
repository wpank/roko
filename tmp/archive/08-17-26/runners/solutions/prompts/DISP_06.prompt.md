# DISP_06: Record CascadeRouter Observations After Each ModelCallService Call

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#disp-06`](../ISSUE-TRACKER.md#disp-06)
- Source: `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` — Task 3.6
- Priority: **P0**
- Effort: 4 hours
- Depends on: `DISP_01` (source 3.1)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: DISP_06 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`ModelCallService` at `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/model_call_service.rs` already has a `cascade_router` field (line 84) typed as `Option<Arc<dyn ForceBackendOverrideRecorder>>`. This trait only has `record_override_outcome(model_slug, success)` -- it was designed for UX34 force_backend learning, not general routing observations.

`CascadeRouter::record_observation()` at `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs:961` takes `(ctx: &RoutingContext, model_slug: &str, reward: f64, success: bool)`. This requires a `RoutingContext` which carries the 17-dimensional feature vector.

The gap: `ModelCallService` records override outcomes but not general routing observations. After each successful `call()`, no observation is fed back to the router.

## Exact Changes

1. Define a new trait in `model_call_service.rs` (or extend `ForceBackendOverrideRecorder`):
   ```rust
   pub trait RoutingObserver: Send + Sync {
       fn record_observation(&self, model_slug: &str, success: bool, latency_ms: u64, cost_usd: f64);
       fn record_override_outcome(&self, model_slug: &str, success: bool) -> bool;
   }
   ```
2. Implement `RoutingObserver` for `CascadeRouter` in `roko-learn` by delegating to `record_observation()` with a default `RoutingContext` (zero features, or minimal features from available info)
3. Replace the `cascade_router: Option<Arc<dyn ForceBackendOverrideRecorder>>` field with `routing_observer: Option<Arc<dyn RoutingObserver>>`
4. In `ModelCallService::call()`, after the agent returns, call:
   ```rust
   if let Some(observer) = &self.routing_observer {
       observer.record_observation(&model, success, latency_ms, cost_usd);
   }
   ```
5. Update `with_cascade_router()` to accept `Arc<dyn RoutingObserver>` instead
6. Update all existing callers (search for `with_cascade_router` in the codebase)

## Design Guidance

The trait approach avoids adding a `roko-agent` -> `roko-learn` dependency. `RoutingObserver` is defined in `roko-agent`, implemented in `roko-learn`. The `roko-cli` layer wires them together. This is the existing pattern used for `ForceBackendOverrideRecorder`.

For the `RoutingContext`, a minimal version is acceptable initially (populate only the fields available: model slug, cost, latency). As more context is threaded through `ModelCallRequest`, richer features can be added.

## Write Scope

- `crates/roko-agent/src/model_call_service.rs`
- `crates/roko-learn/src/cascade_router.rs`

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

- [ ] After `roko run "echo hello"`, the router's observation count has increased (check JSON file)
- [ ] `ForceBackendOverrideRecorder` callers are migrated to `RoutingObserver`

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: DISP_06 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- After `roko run "echo hello"`, the router's observation count has increased (check JSON file)
- `ForceBackendOverrideRecorder` callers are migrated to `RoutingObserver`
- No files outside the Write Scope are modified.
- Commit message contains `tracker: DISP_06 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
