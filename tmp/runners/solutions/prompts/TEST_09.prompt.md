# TEST_09: Serve HTTP API integration tests

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#test-09`](../ISSUE-TRACKER.md#test-09)
- Source: `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` — Task 15.9
- Priority: **P1**
- Effort: 5 hours
- Depends on: `TEST_01` (source 15.1)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: TEST_09 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

roko-serve has ~85 routes and 5 existing integration test files. The existing `api_integration.rs` uses `tower::ServiceExt::oneshot` for in-process HTTP testing (no real server). This pattern should be extended to cover more route groups.

## Exact Changes

1. Test status routes: `GET /api/status` returns valid JSON with expected fields
2. Test plan routes: `POST /api/plans`, `GET /api/plans`, `GET /api/plans/:id`
3. Test PRD routes: `GET /api/prds`, `POST /api/prds`, `GET /api/prds/:slug`
4. Test agent routes: `GET /api/agents`, `POST /api/agents`, agent lifecycle
5. Test job routes: `GET /api/jobs`, `POST /api/jobs`, `GET /api/jobs/:id`
6. Test config routes: `GET /api/config`, `GET /api/config/providers`, `GET /api/config/models`
7. Test learning routes: `GET /api/learning/episodes`, `GET /api/learning/router`, `GET /api/learning/efficiency`
8. Test auth middleware: requests without API key return 401 when auth enabled
9. Test auth bypass: requests without auth header succeed when auth disabled
10. Test SSE endpoint: `GET /api/events` returns SSE stream with `event:` prefix
11. Test OpenAPI spec: `GET /api/openapi.json` returns valid JSON
12. Test 404: unknown route returns 404 with JSON body

## Write Scope

- `crates/roko-serve/Cargo.toml`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] 12+ new tests, all passing
- [ ] Every major route group has at least one test
- [ ] Auth middleware tested in both enabled and disabled modes

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: TEST_09 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- 12+ new tests, all passing
- Every major route group has at least one test
- Auth middleware tested in both enabled and disabled modes
- No files outside the Write Scope are modified.
- Commit message contains `tracker: TEST_09 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
