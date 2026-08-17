# XCUT_31: Add Container Health Monitoring Endpoint

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#xcut-31`](../ISSUE-TRACKER.md#xcut-31)
- Source: `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` — Task 19.31
- Priority: **P1**
- Effort: 3 hours
- Depends on: `XCUT_11` (source 19.11)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: XCUT_31 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The `/api/health` endpoint exists in `crates/roko-serve/src/routes/status/health.rs` and returns 200 OK, but does not report subsystem health. In containerized deployments, operators need to know event bus overflow state, learning subsystem health, filesystem space, and active agent count.

## Exact Changes

1. Extend `/api/health` to return subsystem health:
   ```json
   {
     "status": "healthy",
     "version": "0.5.0",
     "uptime_seconds": 3600,
     "subsystems": {
       "event_bus": { "status": "healthy", "overflow_count": 0 },
       "learning": { "status": "healthy", "episodes_count": 142 },
       "filesystem": { "status": "healthy", "free_bytes": 10737418240 },
       "agents": { "status": "healthy", "active_count": 2, "orphan_count": 0 }
     }
   }
   ```
2. Return HTTP 200 if all subsystems are healthy, 503 if any are degraded.
3. Add `/api/health/ready` (readiness probe) that returns 200 only when the server is fully initialized.
4. Add `/api/health/live` (liveness probe) that returns 200 as long as the process is running.
5. Configure Kubernetes/Docker health check to use `/api/health/ready`.

## Write Scope

- `crates/roko-serve/src/routes/status/health.rs`

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

- [ ] `/api/health` returns subsystem-level health information
- [ ] `/api/health/ready` returns 503 during initialization, 200 when ready
- [ ] `/api/health/live` always returns 200
- [ ] Subsystem health reflects actual state (not hardcoded)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: XCUT_31 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `/api/health` returns subsystem-level health information
- `/api/health/ready` returns 503 during initialization, 200 when ready
- `/api/health/live` always returns 200
- Subsystem health reflects actual state (not hardcoded)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: XCUT_31 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
