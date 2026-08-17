# ACPM_14: Add Research Workflow Template

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#acpm-14`](../ISSUE-TRACKER.md#acpm-14)
- Source: `tmp/solutions/roko/tasks/09-ACP-MCP.md` — Task 9.14
- Priority: **P1**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ACPM_14 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`WorkflowTemplate` at `pipeline.rs:97-104` has 3 variants. The Research template needs two new phases: Researching (agent queries sources) and Synthesizing (agent produces a summary). No gates or commits needed.

## Exact Changes

1. Add `Research` variant to `WorkflowTemplate`.
2. Add `Researching` and `Synthesizing` phases to `PipelinePhase`.
3. Add new actions: `SpawnResearcher { topic: String }`, `SpawnSynthesizer { research_output: String }`.
4. Add transitions:
   - `Pending + Start` (Research) -> `Researching` + `SpawnResearcher`
   - `Researching + AgentCompleted` -> `Synthesizing` + `SpawnSynthesizer`
   - `Synthesizing + AgentCompleted` -> `Complete` + `Done`
   - `Researching + AgentFailed` / `Synthesizing + AgentFailed` -> `Halted` + `Halt` (if no retries) or retry
5. Update `auto_select()`: prompts containing "research", "investigate", "analyze", "explain", "compare" (without implementation words like "implement", "fix", "add") trigger Research template.
6. Update `from_config()` to accept `"research"`.
7. `has_strategy()` -> false, `has_review()` -> false for Research.

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

- [ ] Unit test: Research template flows through Researching -> Synthesizing -> Complete
- [ ] Unit test: `auto_select("research the differences between X and Y")` -> Research
- [ ] Unit test: `auto_select("implement the research findings")` -> NOT Research (contains "implement")
- [ ] Existing template tests pass unchanged

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ACPM_14 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Unit test: Research template flows through Researching -> Synthesizing -> Complete
- Unit test: `auto_select("research the differences between X and Y")` -> Research
- Unit test: `auto_select("implement the research findings")` -> NOT Research (contains "implement")
- Existing template tests pass unchanged
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ACPM_14 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
