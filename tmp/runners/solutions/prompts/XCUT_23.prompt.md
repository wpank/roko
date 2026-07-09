# XCUT_23: Add API Version Header to roko serve

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#xcut-23`](../ISSUE-TRACKER.md#xcut-23)
- Source: `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` — Task 19.23
- Priority: **P3**
- Effort: 2 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: XCUT_23 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The HTTP control plane has ~85 routes with zero versioning. `crates/roko-serve/src/routes/middleware.rs` has no `X-Roko-API-Version` header, no `Accept-Version` handling, no version negotiation. Breaking changes to response schemas silently break dashboard clients, CLI callers, and external integrations.

## Exact Changes

1. Add `X-Roko-API-Version: 1` response header to all routes via Axum middleware layer.
2. Add `Accept-Version: 1` request header support (optional; defaults to latest).
3. In the OpenAPI spec (`openapi.rs`), set `info.version` to `"1.0.0"`.
4. Add version negotiation: if client sends `Accept-Version: 999` and server only supports 1, return `406 Not Acceptable`.
5. Document versioning policy: breaking changes require a new version; additive changes do not.

## Write Scope

- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/openapi.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] All responses include `X-Roko-API-Version: 1`
- [ ] `Accept-Version: 999` returns 406
- [ ] OpenAPI spec has version `1.0.0`
- [ ] Existing clients without version headers work unchanged

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: XCUT_23 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- All responses include `X-Roko-API-Version: 1`
- `Accept-Version: 999` returns 406
- OpenAPI spec has version `1.0.0`
- Existing clients without version headers work unchanged
- No files outside the Write Scope are modified.
- Commit message contains `tracker: XCUT_23 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
