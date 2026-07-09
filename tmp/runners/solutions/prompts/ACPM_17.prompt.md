# ACPM_17: Add Custom Workflow Template Parser

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#acpm-17`](../ISSUE-TRACKER.md#acpm-17)
- Source: `tmp/solutions/roko/tasks/09-ACP-MCP.md` — Task 9.17
- Priority: **P3**
- Effort: 5 hours
- Depends on: `ACPM_14` (source 9.14), `ACPM_15` (source 9.15), `ACPM_16` (source 9.16)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ACPM_17 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Users should be able to define custom step sequences in `roko.toml`. A custom template specifies an ordered list of phases and roles.

## Exact Changes

1. Add `Custom { steps: Vec<CustomStep> }` variant to `WorkflowTemplate`.
2. Define `CustomStep { phase: String, role: Option<String>, config: Option<serde_json::Value> }`.
3. Add `fn from_toml(table: &toml::Table) -> Result<WorkflowTemplate>` that parses:
   ```toml
   [[workflow.steps]]
   phase = "implement"
   role = "implementer"

   [[workflow.steps]]
   phase = "gate"

   [[workflow.steps]]
   phase = "review"
   role = "quick_reviewer"
   ```
4. Validate step sequence: must contain at least "implement" and "gate". Reject invalid phase names.
5. Map custom steps to `PipelinePhase` transitions dynamically using a step index counter.
6. Add `[workflow]` section to config schema in `roko-core`.

## Write Scope

- `crates/roko-acp/src/pipeline.rs`
- `crates/roko-core/src/config/schema.rs`

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

- [ ] Parse a 4-step custom workflow from TOML
- [ ] Reject TOML missing "implement" step
- [ ] Custom workflow executes phases in defined order

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ACPM_17 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Parse a 4-step custom workflow from TOML
- Reject TOML missing "implement" step
- Custom workflow executes phases in defined order
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ACPM_17 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
