# XCUT_29: Improve Railway Deployment with Auth Provisioning

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#xcut-29`](../ISSUE-TRACKER.md#xcut-29)
- Source: `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` — Task 19.29
- Priority: **P1**
- Effort: 3 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: XCUT_29 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`roko deploy railway` exists but does not auto-provision auth. The ground truth document (section 7.4) notes that `dangerously_skip_permissions: true` is always set in plan mode (line 394 of `plan.rs`). Cloud deployments are unauthenticated by default. Plan 3.3 in `06-IMPLEMENTATION-PLANS.md` specifies the exact fix.

## Exact Changes

1. Auto-generate a 32-byte hex API key on Railway deploy, set as `ROKO_API_KEY` env var.
2. Set `api_auth.enabled = true` in the deployed config.
3. Print the API key to stdout once: "Save this API key -- it will not be shown again".
4. Configure Railway health check to `GET /api/health` with 30-second interval.
5. Set Railway memory limit to 2GB.
6. Add `--region` flag for Railway region selection.
7. Validate `RAILWAY_TOKEN` is set before attempting deployment.

## Write Scope

- `crates/roko-cli/src/commands/deploy.rs`

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

- [ ] `roko deploy railway` prints an API key
- [ ] Deployed service rejects unauthenticated requests (returns 401)
- [ ] Health check is configured and passing
- [ ] Missing `RAILWAY_TOKEN` produces a clear error message

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: XCUT_29 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko deploy railway` prints an API key
- Deployed service rejects unauthenticated requests (returns 401)
- Health check is configured and passing
- Missing `RAILWAY_TOKEN` produces a clear error message
- No files outside the Write Scope are modified.
- Commit message contains `tracker: XCUT_29 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
