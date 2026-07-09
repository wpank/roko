# STAB_02: Move share routes inside auth middleware

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-02`](../ISSUE-TRACKER.md#stab-02)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.02
- Priority: **P0**
- Effort: 1 hour
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_02 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The share route architecture is already partially correct. `shared_runs.rs` exports two
functions:
- `auth_routes()` (line 854): `POST /runs/{id}/share` -- mounted inside the auth layer at
  `routes/mod.rs` line 117.
- `public_routes()` (line 864): `GET /api/runs/{id}`, `GET /api/shared/{token}`,
  `GET /runs/{id}` -- mounted outside auth at line 170.

**Status re-assessment**: The `POST /runs/{id}/share` route IS behind the auth layer (merged
at line 117, which is inside the `protected` router). The public routes are read-only viewers.

The concern is: verify the actual Axum router nesting to confirm `auth_routes()` is inside
the auth middleware layer. In `routes/mod.rs`, the protected router (lines ~100-120) should
use `.layer(auth_middleware)`. The public router (lines ~160-175) does not.

## Exact Changes

1. Audit `routes/mod.rs` to confirm that the router block containing line 117
   (`shared_runs::auth_routes()`) is wrapped in the auth middleware layer.
2. Write an integration test: `POST /api/runs/test/share` without an auth header returns 401.
3. Write an integration test: `GET /api/shared/{token}` without auth returns 200 (public).
4. If the auth layer is NOT applied, move `shared_runs::auth_routes()` inside the auth
   middleware `.layer()` call.

## Design Guidance

Keep the two-function split (`auth_routes` / `public_routes`). This pattern is clean and
makes auth boundaries explicit. Other route modules should follow this pattern for any
mutation endpoints.

## Write Scope

- `crates/roko-serve/src/routes/shared_runs.rs`
- `crates/roko-serve/src/routes/mod.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `POST /api/runs/{id}/share` without auth header returns 401
- [ ] `GET /api/shared/{token}` without auth returns 200
- [ ] Integration test covers both cases

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_02 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `POST /api/runs/{id}/share` without auth header returns 401
- `GET /api/shared/{token}` without auth returns 200
- Integration test covers both cases
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_02 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
