# XCUT_13: Make StateHub Snapshot Serializable for REST API

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#xcut-13`](../ISSUE-TRACKER.md#xcut-13)
- Source: `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` — Task 19.13
- Priority: **P7**
- Effort: 2 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: XCUT_13 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`DashboardSnapshot` is served via the REST API at `/api/status` (in `crates/roko-serve/src/routes/status/health.rs`) but the serialization is ad-hoc: some fields are skipped, some are transformed inline in the route handler. The snapshot already has `Serialize + Deserialize` on `DashboardEvent` (line 23 of `dashboard_snapshot.rs`) but the `DashboardSnapshot` struct itself may need verification that all fields are serializable.

## Exact Changes

1. Ensure all fields in `DashboardSnapshot` implement `Serialize + Deserialize`.
2. Add `schema_version: u8` field (start at 1) for forward compatibility.
3. Add `generated_at: DateTime<Utc>` field.
4. In the `/api/status` route, return `Json(snapshot)` directly instead of constructing an ad-hoc response.
5. Document the snapshot schema in `DashboardSnapshot` doc comments.

## Write Scope

- `crates/roko-core/src/dashboard_snapshot.rs`
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

- [ ] `GET /api/status` returns the full `DashboardSnapshot` with `schema_version: 1`
- [ ] `serde_json::to_string(&snapshot)` round-trips cleanly
- [ ] Existing TUI rendering is unaffected

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: XCUT_13 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `GET /api/status` returns the full `DashboardSnapshot` with `schema_version: 1`
- `serde_json::to_string(&snapshot)` round-trips cleanly
- Existing TUI rendering is unaffected
- No files outside the Write Scope are modified.
- Commit message contains `tracker: XCUT_13 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
