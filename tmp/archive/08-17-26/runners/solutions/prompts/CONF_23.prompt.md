# CONF_23: Auto-Provision Auth on Cloud Deploy

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#conf-23`](../ISSUE-TRACKER.md#conf-23)
- Source: `tmp/solutions/roko/tasks/16-CONFIG-AND-WIRING.md` — Task 16.23
- Priority: **P0**
- Effort: Medium
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: CONF_23 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`roko serve` binds to `0.0.0.0:6677` with auth disabled by default. Cloud deployments
(`roko deploy railway`) expose this publicly with no authentication. The
`acknowledge_public_risk` flag at `roko-serve/src/lib.rs:644` bypasses the auth warning
without actually enabling auth.

## Exact Changes

1. In `roko deploy railway/fly/docker`, auto-generate a random API key if none configured.
2. Set `api_auth.enabled = true` in the deploy config.
3. Print the generated API key to stdout so the user can save it.
4. Set the key as a Railway/Fly secret automatically.
5. In `roko serve`, if binding to `0.0.0.0` and auth is not enabled, print a
   prominent warning. The `acknowledge_public_risk` flag should ALSO check that
   auth is actually enabled, not just suppress the warning.

## Write Scope

- `crates/roko-cli/src/main.rs`
- `crates/roko-cli/src/commands/server.rs`
- `crates/roko-serve/src/lib.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/16-CONFIG-AND-WIRING.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `roko deploy railway` output includes an auto-generated API key.
- [ ] `acknowledge_public_risk = true` without `auth.enabled = true` still warns.

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: CONF_23 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko deploy railway` output includes an auto-generated API key.
- `acknowledge_public_risk = true` without `auth.enabled = true` still warns.
- No files outside the Write Scope are modified.
- Commit message contains `tracker: CONF_23 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
