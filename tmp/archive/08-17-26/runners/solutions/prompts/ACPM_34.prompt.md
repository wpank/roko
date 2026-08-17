# ACPM_34: Implement A2A Task Reception

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#acpm-34`](../ISSUE-TRACKER.md#acpm-34)
- Source: `tmp/solutions/roko/tasks/09-ACP-MCP.md` — Task 9.34
- Priority: **P2**
- Effort: 6 hours
- Depends on: `ACPM_33` (source 9.33)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ACPM_34 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

External agents submit tasks via `POST /a2a/tasks/send`. The task is mapped to an internal `WorkflowRun` and executed via the ACP pipeline runner.

## Exact Changes

1. Add `roko-a2a` dependency to `roko-serve/Cargo.toml`.
2. Create `routes/a2a.rs` with:
   - `POST /a2a/tasks/send` -- receives an `A2ATask`, extracts prompt from messages, selects template (from metadata or auto), executes via pipeline runner, returns task ID
   - `GET /a2a/tasks/:id` -- returns task status (maps pipeline phase to `TaskStatus`)
   - `POST /a2a/tasks/:id/cancel` -- cancels a running task via `CancelToken`
3. Map pipeline completion to A2A task status:
   - `Complete` -> `TaskStatus::Completed` with output as artifact
   - `Halted` / `Cancelled` -> `TaskStatus::Failed` with error detail
   - In-progress phases -> `TaskStatus::Working`
4. Store active tasks in `AppState` with `HashMap<String, A2ATaskState>`.
5. Add routes to `build_router()`.

## Write Scope

- `crates/roko-serve/src/routes/mod.rs`
- `crates/roko-serve/Cargo.toml`

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

- [ ] External agent can submit a coding task via `POST /a2a/tasks/send`
- [ ] Task status is retrievable via `GET /a2a/tasks/:id`
- [ ] Pipeline completion updates A2A task status
- [ ] Cancellation via `POST /a2a/tasks/:id/cancel` works

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ACPM_34 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- External agent can submit a coding task via `POST /a2a/tasks/send`
- Task status is retrievable via `GET /a2a/tasks/:id`
- Pipeline completion updates A2A task status
- Cancellation via `POST /a2a/tasks/:id/cancel` works
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ACPM_34 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
