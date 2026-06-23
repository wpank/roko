# INNO_23: Add HTTP steering endpoints

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-23`](../ISSUE-TRACKER.md#inno-23)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.23
- Priority: **P2**
- Effort: 8 hours
- Depends on: `INNO_21` (source 11.21), `INNO_22` (source 11.22)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_23 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`roko-serve` at `crates/roko-serve/src/routes/` has ~85 routes. Steering
endpoints allow external clients (web dashboards, CI systems) to steer
running agents.

## Exact Changes

1. Create `crates/roko-serve/src/routes/steering.rs`.
2. `POST /api/steer/{task_id}` -- accept `SteeringAction` JSON body, send
   to steering channel.
3. `GET /api/confidence` -- return `Vec<ConfidenceReport>` for all active tasks.
4. `POST /api/approve/{task_id}` -- shorthand for `ReviewVerdict` with approve.
5. Wire into existing `roko-serve` router in `routes/mod.rs`.
6. Return 404 if task_id is not active, 409 if task already completed.
7. Respect existing auth middleware.

## Write Scope

- `crates/roko-serve/src/routes/mod.rs`

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

- [ ] `POST /api/steer/task-07 {"action":"redirect","guidance":"..."}` injects context into the running agent
- [ ] `GET /api/confidence` returns confidence scores for active tasks
- [ ] 401 returned for unauthenticated requests (when auth enabled)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_23 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `POST /api/steer/task-07 {"action":"redirect","guidance":"..."}` injects context into the running agent
- `GET /api/confidence` returns confidence scores for active tasks
- 401 returned for unauthenticated requests (when auth enabled)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_23 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
