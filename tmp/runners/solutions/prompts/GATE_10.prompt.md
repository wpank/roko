# GATE_10: Add gate budget tracking

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#gate-10`](../ISSUE-TRACKER.md#gate-10)
- Source: `tmp/solutions/roko/tasks/04-GATE-PIPELINE.md` — Task 4.10
- Priority: **P1**
- Effort: 2 hours
- Depends on: `GATE_02` (source 4.2)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: GATE_10 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

LLM judge invocations have no cost tracking (AP-10). Each judge call is a full LLM API call but no episode is recorded, no cost is attributed, and no limit prevents runaway invocations during replan loops.

## Exact Changes

1. Add a `GateBudget` struct to `crates/roko-core/src/foundation.rs`:
   ```rust
   #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
   pub struct GateBudget {
       pub max_judge_invocations: u32,
       pub current_judge_invocations: u32,
       pub max_cost_usd: f64,
       pub current_cost_usd: f64,
   }

   impl Default for GateBudget {
       fn default() -> Self {
           Self {
               max_judge_invocations: 10,
               current_judge_invocations: 0,
               max_cost_usd: 5.0,
               current_cost_usd: 0.0,
           }
       }
   }

   impl GateBudget {
       pub fn can_invoke_judge(&self) -> bool {
           self.current_judge_invocations < self.max_judge_invocations
               && self.current_cost_usd < self.max_cost_usd
       }
   }
   ```
2. Add optional `budget: Option<GateBudget>` field to `GateConfig`.
3. In GateService, before invoking the judge gate, check `budget.can_invoke_judge()`. If exhausted, return a skipped verdict with reason "gate budget exhausted".
4. After judge invocation, increment `budget.current_judge_invocations` and add estimated cost to `budget.current_cost_usd`.

## Write Scope

- `crates/roko-gate/src/gate_service.rs`
- `crates/roko-core/src/foundation.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/04-GATE-PIPELINE.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Judge gate is skipped when budget is exhausted
- [ ] Budget tracking increments on each judge invocation

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: GATE_10 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Judge gate is skipped when budget is exhausted
- Budget tracking increments on each judge invocation
- No files outside the Write Scope are modified.
- Commit message contains `tracker: GATE_10 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
