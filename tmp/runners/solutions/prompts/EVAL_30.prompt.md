# EVAL_30: Curriculum from failures (WebRL pattern)

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#eval-30`](../ISSUE-TRACKER.md#eval-30)
- Source: `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` — Task 5.30
- Priority: **P2**
- Effort: 5 hours
- Depends on: `EVAL_05` (source 5.5)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: EVAL_30 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Cluster failed evaluation traces by judge rationale text. Generate synthetic tasks targeting failure modes. Integrates with existing `crates/roko-learn/src/curriculum.rs` and `crates/roko-learn/src/post_gate_reflection.rs`.

## Exact Changes

1. Define `CurriculumTask { id, source_cluster_id, cluster_size: u32, prompt, acceptance_criteria: Vec<String>, eval_profile: String, variants: Vec<CurriculumVariant>, priority: f64, status: CurriculumStatus }`.
2. Define `CurriculumStatus { Pending, InProgress, Promoted, Retained, Superseded }`.
3. Define `CurriculumVariant { description: String, edge_case: String }`.
4. Implement `cluster_failures(traces: &[EvalTrace], min_cluster_size: usize) -> Vec<FailureCluster>`: group by common finding rule_ids and criterion names. Simple text similarity (Jaccard over tokenized rationale) rather than embedding-based clustering (MVP).
5. Implement `generate_curriculum_tasks(cluster: &FailureCluster) -> Vec<CurriculumTask>`.

## Write Scope

- `crates/roko-eval/src/lib.rs`

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

- [ ] Test cluster creation from 5 failing traces with similar rule_ids
- [ ] Test curriculum task generation

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: EVAL_30 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Test cluster creation from 5 failing traces with similar rule_ids
- Test curriculum task generation
- No files outside the Write Scope are modified.
- Commit message contains `tracker: EVAL_30 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
