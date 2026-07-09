# RNNR_08: Wire wave gate execution into runner event loop

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#rnnr-08`](../ISSUE-TRACKER.md#rnnr-08)
- Source: `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` — Task 14.8
- Priority: **??**
- Effort: ?
- Depends on: `RNNR_03` (source 14.3), `RNNR_06` (source 14.6)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: RNNR_08 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: When the state machine emits wave-level gate events, execute gates
against the integration branch (where all wave task merges accumulated), not
against individual worktrees.

## Exact Changes

1. Add `async fn run_wave_gate(integration_dir: &Path, gate_configs: &[GateConfig]) -> Vec<GateVerdict>`
   to `gate_dispatch.rs`
2. The method runs in the integration worktree directory, which has all merged
   task changes for the wave
3. Execute configured gates in order: compile -> clippy -> custom shell -> test
4. Collect all verdicts and return them as a batch
5. Track wave gate duration and emit it as a runner event
6. If any gate fails, include raw output for failure attribution (Task 14.10)

## Write Scope

- `crates/roko-cli/src/runner/event_loop.rs`
- `crates/roko-cli/src/runner/gate_dispatch.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Wave gates run in the integration worktree, not individual task worktrees
- [ ] Gate output includes enough information to identify which task caused failure
- [ ] Wave gate duration is tracked and reported
- [ ] All configured gates (compile, clippy, test, custom) are supported

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: RNNR_08 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Wave gates run in the integration worktree, not individual task worktrees
- Gate output includes enough information to identify which task caused failure
- Wave gate duration is tracked and reported
- All configured gates (compile, clippy, test, custom) are supported
- No files outside the Write Scope are modified.
- Commit message contains `tracker: RNNR_08 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
