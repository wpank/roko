# EVAL_06: Bridge adapter -- `BridgeGateService`

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#eval-06`](../ISSUE-TRACKER.md#eval-06)
- Source: `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` — Task 5.6
- Priority: **P0**
- Effort: 8 hours
- Depends on: `EVAL_01` (source 5.1), `EVAL_04` (source 5.4), `EVAL_05` (source 5.5)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: EVAL_06 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The migration from `roko-gate` to `roko-eval` must be incremental. `BridgeGateService` wraps the existing `GateService` at `crates/roko-gate/src/gate_service.rs` and intercepts gate names that have been migrated to the new system. For migrated gates, it runs through `EvalService` and projects the `EvalTrace` back to `GateVerdict` for backward compatibility. For non-migrated gates, it delegates to the inner `GateService` unchanged.

The `GateRunner` trait is defined at `crates/roko-core/src/foundation.rs:311-320`:
```rust
#[async_trait]
pub trait GateRunner: Send + Sync {
    async fn run_gates(&self, config: GateConfig) -> Result<GateReport>;
}
```

## Exact Changes

1. Add `roko-eval = { path = "../roko-eval" }` to `crates/roko-gate/Cargo.toml`.
2. Define `BridgeGateService`:
   ```rust
   pub struct BridgeGateService {
       legacy: GateService,
       eval_service: Option<Arc<EvalService>>,
       migrated: HashSet<String>,
   }
   impl BridgeGateService {
       pub fn new(legacy: GateService) -> Self;
       pub fn with_eval_service(self, svc: Arc<EvalService>) -> Self;
       pub fn migrate_gate(mut self, name: &str) -> Self;
   }
   ```
3. Implement `GateRunner` for `BridgeGateService`:
   - Split `config.enabled_gates` into migrated and non-migrated.
   - For non-migrated gates: build a sub-`GateConfig` and delegate to `self.legacy.run_gates()`.
   - For migrated gates: construct `ArtifactRef` from `config.workdir`, run through `self.eval_service.evaluate()`, project `EvalTrace` to `Vec<GateVerdict>` via a `From` impl.
   - Merge verdicts in rung order.
4. Implement `From<&EvalTrace> for Vec<GateVerdict>`:
   - Each `CriterionResult` becomes a `GateVerdict` with `gate_name = criterion_name`, `passed`, `output = summary of findings`, `duration_ms`.
   - The overall `EvalVerdict` is not a separate verdict -- it is the conjunction of its parts.
5. Add `pub mod bridge;` to `crates/roko-gate/src/lib.rs` and `pub use bridge::BridgeGateService;`.

## Design Guidance

The bridge is the zero-regression guarantee. With no migrated gates (`migrated.is_empty()`), it must behave **identically** to `GateService`. Test this by running the exact same `GateConfig` through both and asserting verdict-for-verdict equivalence. The bridge does NOT change the `GateRunner` trait or `GateReport` struct -- it produces the same output type.

## Write Scope

- `crates/roko-gate/src/lib.rs`
- `crates/roko-gate/Cargo.toml`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Test: `BridgeGateService` with no migrations behaves identically to `GateService`
- [ ] Test: a migrated gate produces a `GateVerdict` matching the legacy gate's pass/fail

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: EVAL_06 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Test: `BridgeGateService` with no migrations behaves identically to `GateService`
- Test: a migrated gate produces a `GateVerdict` matching the legacy gate's pass/fail
- No files outside the Write Scope are modified.
- Commit message contains `tracker: EVAL_06 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
