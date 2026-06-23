# INNO_20: Define SteeringAction primitives

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-20`](../ISSUE-TRACKER.md#inno-20)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.20
- Priority: **P2**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_20 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Interactive steering allows humans to redirect, skip, or adjust running agents
without stopping the plan.

## Exact Changes

1. Create `crates/roko-core/src/steering.rs`.
2. Define `SteeringAction` enum:
   - `Redirect { guidance: String, model_override: Option<String> }`
   - `Skip { reason: String }`
   - `Split { sub_tasks: Vec<String> }`
   - `BudgetAdjust { remaining_budget_usd: f64, model_preference: Option<String> }`
   - `InjectContext { content: String, priority: ContextPriority }`
   - `ReviewVerdict { task_id: String, verdict: Verdict, notes: String }`
3. Define `ContextPriority` enum: `Override`, `Append`, `Background`.
4. Define `ConfidenceThresholds` struct:
   - `auto_proceed: f64` (default 0.85)
   - `suggest_review: f64` (default 0.50)
   - `require_approval: f64` (default 0.50)
5. Define `SteeringAuditEntry` for the audit trail.
6. Implement serde for all types.
7. Add `pub mod steering;` to `crates/roko-core/src/lib.rs`.

## Write Scope

- `crates/roko-core/src/lib.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/11-INNOVATIONS.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] All types compile and serialize to/from JSON
- [ ] Unit test: round-trip serialization for each SteeringAction variant

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_20 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- All types compile and serialize to/from JSON
- Unit test: round-trip serialization for each SteeringAction variant
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_20 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
