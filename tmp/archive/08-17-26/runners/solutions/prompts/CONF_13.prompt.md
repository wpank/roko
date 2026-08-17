# CONF_13: Wire AnomalyDetector Into Live Paths

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#conf-13`](../ISSUE-TRACKER.md#conf-13)
- Source: `tmp/solutions/roko/tasks/16-CONFIG-AND-WIRING.md` — Task 16.13
- Priority: **P3**
- Effort: Small
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: CONF_13 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`AnomalyDetector` is defined in `crates/roko-learn/src/anomaly.rs` (also in
`roko-agent/src/task_runner.rs` and re-exported from `roko-agent/src/lib.rs:151`).
It detects prompt loops, cost spikes, and quality degradation. It is referenced in
`roko-cli/src/orchestrate.rs` and `roko-cli/src/learning_helpers.rs` but never
instantiated in the runner v2 event loop.

## Exact Changes

1. Create `AnomalyDetector` at session start in runner v2 event loop.
2. Before each dispatch, call `detector.check_prompt(prompt_hash)` for loop detection.
3. After each response, call `detector.check_cost(cost_usd)` for spike detection.
4. On anomaly: log at WARN level. Optionally record in episode's anomalies field.

## Write Scope

- `crates/roko-learn/src/anomaly.rs`
- `crates/roko-cli/src/runner/event_loop.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/16-CONFIG-AND-WIRING.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Dispatching the same prompt 5 times triggers a prompt-loop warning in logs.
- [ ] A response costing 10x the session average triggers a cost-spike warning.

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: CONF_13 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Dispatching the same prompt 5 times triggers a prompt-loop warning in logs.
- A response costing 10x the session average triggers a cost-spike warning.
- No files outside the Write Scope are modified.
- Commit message contains `tracker: CONF_13 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
