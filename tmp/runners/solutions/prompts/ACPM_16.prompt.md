# ACPM_16: Add Documentation Workflow Template

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#acpm-16`](../ISSUE-TRACKER.md#acpm-16)
- Source: `tmp/solutions/roko/tasks/09-ACP-MCP.md` — Task 9.16
- Priority: **P2**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ACPM_16 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Documentation template: Scribe writes docs, Critic reviews, fix loop, commit. The Scribe role has restricted write access (docs only).

## Exact Changes

1. Add `Documentation` variant to `WorkflowTemplate`.
2. Add `Scribing` and `Critiquing` phases to `PipelinePhase`.
3. Add new actions: `SpawnScribe { files: Vec<String>, context: String }`, `SpawnCritic { docs_diff: String }`.
4. Transitions:
   - `Pending + Start` (Documentation) -> `Scribing` + `SpawnScribe`
   - `Scribing + AgentCompleted` -> `Critiquing` + `SpawnCritic`
   - `Critiquing + ReviewApproved` -> `Committing` + `Commit`
   - `Critiquing + ReviewRevise` -> `Scribing` + `SpawnScribe` (with feedback, if iterations remain)
5. Update `auto_select()`: prompts with "document", "docs", "README", "changelog" trigger Documentation.
6. Update `from_config()` to accept `"documentation"`.

## Write Scope

- `crates/roko-acp/src/pipeline.rs`

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

- [ ] Unit test: Documentation template flows Scribing -> Critiquing -> Committing
- [ ] Unit test: critic rejection loops back to Scribing
- [ ] Existing template tests pass

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ACPM_16 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Unit test: Documentation template flows Scribing -> Critiquing -> Committing
- Unit test: critic rejection loops back to Scribing
- Existing template tests pass
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ACPM_16 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
