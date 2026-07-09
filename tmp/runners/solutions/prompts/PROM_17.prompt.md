# PROM_17: Replace Orchestrate.rs Inline Prompts with Template Calls

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#prom-17`](../ISSUE-TRACKER.md#prom-17)
- Source: `tmp/solutions/roko/tasks/06-PROMPT-ASSEMBLY.md` — Task 6.17
- Priority: **??**
- Effort: 3-4 days | **Impact**: Medium-High
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PROM_17 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: VCG warmup threshold of 10 observations per bidder is rarely
reached. DensityGreedy dominates. MultiPatchForager is built but context
retrieval uses direct queries.

## Exact Changes

1. Identify all inline prompt sites:
   ```bash
   grep -n 'format!.*Plan:.*Task:.*Implement\|format!.*Retry\|format!.*escalat' crates/roko-cli/src/orchestrate.rs
   ```
   Known sites: lines ~9399, ~9846, ~10437, ~14131, ~14558
2. Create helper functions or template structs for:
   - `fn fallback_task_prompt(plan_id: &str, task_id: &str) -> String`
   - `fn gate_failure_retry_hint(gate_error: &str) -> String`
   - `fn model_escalation_prompt(prior_model: &str, target_model: &str) -> String`
   - `fn replan_prompt(task_context: &str, failure_reason: &str) -> String`
3. Replace each `format!()` site with the appropriate helper call
4. Place helpers in a dedicated module or in the existing prompting module

## Write Scope

_None — this is a documentation/verification-only batch._

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/06-PROMPT-ASSEMBLY.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `grep -c 'format!.*Plan:.*Task:.*Implement' crates/roko-cli/src/orchestrate.rs` returns 0
- [ ] Gate failure retry still works end-to-end
- [ ] Model escalation still works end-to-end

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PROM_17 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `grep -c 'format!.*Plan:.*Task:.*Implement' crates/roko-cli/src/orchestrate.rs` returns 0
- Gate failure retry still works end-to-end
- Model escalation still works end-to-end
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PROM_17 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
