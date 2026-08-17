# SAFE_16: Audit Log Retention Policy

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#safe-16`](../ISSUE-TRACKER.md#safe-16)
- Source: `tmp/solutions/roko/tasks/17-SAFETY-SECURITY.md` — Task 17.16
- Priority: **P2**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: SAFE_16 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: Implement configurable log retention for EU AI Act Article 26(6)
compliance (at least 6 months retention for automatic event logs).

## Exact Changes

1. Add `[safety.audit]` config section:
   ```toml
   [safety.audit]
   retention_days = 180
   max_file_size_mb = 100
   rotation_count = 10
   ```
2. On `roko serve` startup, check audit log age and warn if retention < 180 days
3. Add `roko audit gc` command that only removes logs older than `retention_days`
4. `roko doctor` warns if audit logging is disabled or retention < 180 days

## Write Scope

- `crates/roko-runtime/src/audit.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/17-SAFETY-SECURITY.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `roko audit gc` with `retention_days = 180` preserves logs from last 6 months
- [ ] `roko doctor` warns when `retention_days < 180`
- [ ] Retention config is loaded from roko.toml

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: SAFE_16 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko audit gc` with `retention_days = 180` preserves logs from last 6 months
- `roko doctor` warns when `retention_days < 180`
- Retention config is loaded from roko.toml
- No files outside the Write Scope are modified.
- Commit message contains `tracker: SAFE_16 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
