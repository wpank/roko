# STAB_05: Fix `acknowledge_public_risk` bypass

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-05`](../ISSUE-TRACKER.md#stab-05)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.05
- Priority: **P0**
- Effort: 1 hour
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_05 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The current logic at line 644 of `lib.rs`:
```rust
if serve.acknowledge_public_risk {
    warn!(addr = %addr, "binding to a public address without authentication; all routes will be network-accessible");
    return Ok(());
}
anyhow::bail!("Public bind requires `serve.auth.enabled = true` or `serve.acknowledge_public_risk = true`.");
```

This means `acknowledge_public_risk = true` suppresses the bind error and allows a public
server WITHOUT auth. The user sees a log warning but no auth is enforced.

## Exact Changes

1. When `acknowledge_public_risk = true` AND `api_auth.enabled = false`:
   - Log a WARNING: "Public risk acknowledged but auth is NOT enabled. All routes accessible without authentication."
   - Print a prominent banner to stderr (not just tracing):
     ```
     ======================================
     WARNING: NO AUTHENTICATION ENABLED
     Server is publicly accessible at {addr}
     Set [serve.auth] enabled = true for security
     ======================================
     ```
   - Allow the bind (current behavior) -- user explicitly opted in.
2. When binding to non-localhost AND neither `auth.enabled` nor `acknowledge_public_risk`:
   - Bail with the existing error message (current behavior, correct).
3. When `auth.enabled = true`:
   - Allow bind regardless of `acknowledge_public_risk` (current behavior, correct).

## Design Guidance

The current behavior is actually defensible -- it requires explicit opt-in to run without
auth. The fix is about making the consequences MORE visible, not changing the logic. The
warning should be impossible to miss (stderr banner, not just log line).

## Write Scope

- `crates/roko-serve/src/lib.rs`

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

- [ ] `acknowledge_public_risk = true` with no auth shows prominent WARNING banner
- [ ] Without either flag, public bind fails with error
- [ ] With `auth.enabled = true`, bind succeeds normally

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_05 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `acknowledge_public_risk = true` with no auth shows prominent WARNING banner
- Without either flag, public bind fails with error
- With `auth.enabled = true`, bind succeeds normally
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_05 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
