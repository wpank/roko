# XCUT_28: Add Docker Compose for Development

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#xcut-28`](../ISSUE-TRACKER.md#xcut-28)
- Source: `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` — Task 19.28
- Priority: **P1**
- Effort: 3 hours
- Depends on: `XCUT_27` (source 19.27)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: XCUT_28 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

A `docker/docker-compose.yml` already exists in the workspace. No dev-specific compose file exists. No OTel collector or Jaeger service is configured for local trace viewing.

## Exact Changes

1. Review existing `docker/docker-compose.yml` and extend with:
   - `otel-collector`: `otel/opentelemetry-collector:latest`, receives OTLP on 4317.
   - `jaeger`: `jaegertracing/all-in-one:latest`, trace visualization on 16686.
2. Create `docker/docker-compose.dev.yml` with:
   - Hot reload via mounted source volume.
   - Debug logging level.
   - Exposed debug ports.
3. Add `scripts/dev-up.sh` that runs `docker compose -f docker/docker-compose.yml -f docker/docker-compose.dev.yml up`.
4. Ensure OTel collector forwards to Jaeger for local trace viewing.

## Write Scope

- `docker/docker-compose.yml`
- `docker/docker-compose.dev.yml`

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

- [ ] `docker compose -f docker/docker-compose.yml up` starts roko-serve
- [ ] `http://localhost:6677/api/health` returns healthy
- [ ] `http://localhost:16686` shows Jaeger UI (when OTel collector is configured)
- [ ] `docker compose down` stops all services cleanly

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: XCUT_28 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `docker compose -f docker/docker-compose.yml up` starts roko-serve
- `http://localhost:6677/api/health` returns healthy
- `http://localhost:16686` shows Jaeger UI (when OTel collector is configured)
- `docker compose down` stops all services cleanly
- No files outside the Write Scope are modified.
- Commit message contains `tracker: XCUT_28 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
