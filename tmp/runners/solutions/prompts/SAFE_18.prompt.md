# SAFE_18: Security Configuration Validator

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#safe-18`](../ISSUE-TRACKER.md#safe-18)
- Source: `tmp/solutions/roko/tasks/17-SAFETY-SECURITY.md` — Task 17.18
- Priority: **P2**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: SAFE_18 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: Add `roko config validate-security` that checks the entire security
configuration for gaps. Wire into `roko doctor` as a security subsection.

## Exact Changes

1. Check contract coverage: all roles used in plans have contracts
2. Check `dangerously_skip_permissions` is not set in config
3. Check MCP servers: version pinned (no `@latest`)
4. Check audit logging: enabled, retention >= 180 days
5. Check auth: enabled if bind is not loopback
6. Check network policy: configured with explicit allowlist
7. Output a security posture report: pass/warn/fail per check
8. Add `--json` flag for machine-readable output
9. Wire the check list into `roko doctor` as a "Security" section

## Write Scope

- `crates/roko-cli/src/commands/config_cmd.rs`

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

- [ ] `roko config validate-security` produces a structured report
- [ ] Missing contracts for used roles trigger a warning
- [ ] Disabled audit logging triggers a failure
- [ ] `roko doctor` includes a security subsection with pass/warn/fail

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: SAFE_18 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko config validate-security` produces a structured report
- Missing contracts for used roles trigger a warning
- Disabled audit logging triggers a failure
- `roko doctor` includes a security subsection with pass/warn/fail
- No files outside the Write Scope are modified.
- Commit message contains `tracker: SAFE_18 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
