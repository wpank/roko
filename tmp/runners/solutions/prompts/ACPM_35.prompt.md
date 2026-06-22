# ACPM_35: Add DelegateExternal Action to Pipeline

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#acpm-35`](../ISSUE-TRACKER.md#acpm-35)
- Source: `tmp/solutions/roko/tasks/09-ACP-MCP.md` — Task 9.35
- Priority: **P3**
- Effort: 4 hours
- Depends on: `ACPM_34` (source 9.34)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ACPM_35 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The pipeline should be able to delegate sub-tasks to external agents via A2A. This adds an outbound path (Roko delegates to others) complementing the inbound path (others delegate to Roko).

## Exact Changes

1. Add `DelegateExternal { task: ExternalTaskSpec }` to `PipelineAction` where:
   ```rust
   pub struct ExternalTaskSpec {
       pub agent_url: String,
       pub skill: String,
       pub prompt: String,
   }
   ```
2. Add `ExternalDelegated` phase to `PipelinePhase`.
3. Add `ExternalCompleted { output: String }` and `ExternalFailed { error: String }` to `PipelineEvent`.
4. Transitions:
   - `DelegateExternal` -> `ExternalDelegated`
   - `ExternalCompleted` -> resume pipeline (back to Gating or next phase)
   - `ExternalFailed` -> `Halted` or retry
5. In the runner, implement delegation by POSTing to `{agent_url}/a2a/tasks/send` and polling `GET /a2a/tasks/:id` until completion.

## Write Scope

- `crates/roko-acp/src/pipeline.rs`
- `crates/roko-acp/src/runner.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/09-ACP-MCP.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Unit test: pipeline transitions through `ExternalDelegated` phase
- [ ] Unit test: external failure halts pipeline with clear reason
- [ ] Pipeline state machine remains pure (no I/O in `step()`)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ACPM_35 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Unit test: pipeline transitions through `ExternalDelegated` phase
- Unit test: external failure halts pipeline with clear reason
- Pipeline state machine remains pure (no I/O in `step()`)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ACPM_35 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
