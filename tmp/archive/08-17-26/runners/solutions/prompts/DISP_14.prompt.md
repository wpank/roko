# DISP_14: Wire ProviderHealthTracker into ModelCallService

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#disp-14`](../ISSUE-TRACKER.md#disp-14)
- Source: `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` — Task 3.14
- Priority: **P1**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: DISP_14 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`ProviderHealthTracker` at `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/provider_health.rs:501` is a per-provider circuit breaker with `record_success()`, `record_failure()`, `is_healthy()`, and `filter_arms()`. It supports Healthy/Unhealthy/Probing states with configurable failure threshold (default 3) and recovery window (default 120s).

It is already used by:
- `roko-serve` state (`state.rs:377`)
- `roko-learn` runtime feedback (`runtime_feedback.rs:1251`)
- `roko-conductor` (`conductor.rs:70`)
- `roko-learn` model router (`model_router.rs:1241`)

But `ModelCallService` does not check provider health before dispatch. When a provider is down, every call fails immediately rather than trying a fallback.

## Exact Changes

1. Add a `health_tracker: Option<Arc<dyn HealthGate>>` field to `ModelCallService` where `HealthGate` is a new trait:
   ```rust
   pub trait HealthGate: Send + Sync {
       fn is_healthy(&self, provider_key: &str) -> bool;
       fn record_success(&self, provider_key: &str);
       fn record_failure(&self, provider_key: &str);
   }
   ```
2. Implement `HealthGate` for `ProviderHealthTracker` in `roko-learn` (trivial delegation)
3. Add `with_health_gate(gate: Arc<dyn HealthGate>)` builder method
4. In `ModelCallService::call()`, before dispatch:
   ```rust
   if let Some(gate) = &self.health_tracker {
       let provider_key = self.provider_key_for_model(&model);
       if !gate.is_healthy(&provider_key) {
           // Try fallback models
           for fb in &self.fallback_models {
               let fb_provider = self.provider_key_for_model(fb);
               if gate.is_healthy(&fb_provider) {
                   tracing::warn!(from = %model, to = %fb, "primary provider unhealthy, failing over");
                   model = fb.clone();
                   break;
               }
           }
       }
   }
   ```
5. After dispatch, record success/failure:
   ```rust
   if let Some(gate) = &self.health_tracker {
       if success { gate.record_success(&provider_key); }
       else { gate.record_failure(&provider_key); }
   }
   ```
6. Add a helper `provider_key_for_model(&self, model: &str) -> String` that maps model slug to provider key using `roko_core::agent::resolve_model` pattern matching

## Design Guidance

Use the trait approach to avoid a `roko-agent` -> `roko-learn` dependency. The `HealthGate` trait is minimal (3 methods). `ProviderHealthTracker` implements it in `roko-learn`. `roko-cli` wires them together when constructing `ModelCallService`. This matches the existing `ForceBackendOverrideRecorder` pattern.

## Write Scope

- `crates/roko-agent/src/model_call_service.rs`

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

- [ ] Unit test: when primary provider is marked unhealthy, dispatch uses fallback model
- [ ] Unit test: after successful call, health tracker records success
- [ ] Unit test: after failed call, health tracker records failure

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: DISP_14 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Unit test: when primary provider is marked unhealthy, dispatch uses fallback model
- Unit test: after successful call, health tracker records success
- Unit test: after failed call, health tracker records failure
- No files outside the Write Scope are modified.
- Commit message contains `tracker: DISP_14 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
