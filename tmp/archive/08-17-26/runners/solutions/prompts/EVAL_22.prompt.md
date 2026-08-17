# EVAL_22: Anchor store and rotation

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#eval-22`](../ISSUE-TRACKER.md#eval-22)
- Source: `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` — Task 5.22
- Priority: **P1**
- Effort: 5 hours
- Depends on: `EVAL_18` (source 5.18)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: EVAL_22 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The fixed-anchor protocol: always compare new_candidate vs prev_best_release. Anchors persist to `.roko/eval/anchors.json`. Bootstrapping protocol establishes first anchor via absolute scoring.

## Exact Changes

1. Define `JudgeAnchor { content_hash, established_at_ms, provenance: AnchorProvenance, artifact: ArtifactRef, elo: f64, comparison_count: u64 }`.
2. Define `AnchorProvenance { HumanApproved, GatePassed, ArenaWinner, Bootstrapped }`.
3. Define `AnchorRotationConfig { max_anchor_age_days: u32 (30), rotation_win_rate: f64 (0.8), rotation_min_evals: u32 (20), bootstrap_rotation_evals: u32 (10) }`.
4. Implement `AnchorStore` with methods: `get(task_id) -> Option<JudgeAnchor>`, `set(anchor)`, `rotate_if_due(config) -> bool`. Persists to `.roko/eval/anchors.json`.
5. Bootstrapping: when no anchor exists, accept candidate with provenance `Bootstrapped`. After `bootstrap_rotation_evals` subsequent evaluations, auto-promote best candidate.

## Write Scope

- `crates/roko-eval-judge/src/lib.rs`

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

- [ ] Test bootstrap -> rotate lifecycle
- [ ] Persistence round-trip test

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: EVAL_22 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Test bootstrap -> rotate lifecycle
- Persistence round-trip test
- No files outside the Write Scope are modified.
- Commit message contains `tracker: EVAL_22 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
