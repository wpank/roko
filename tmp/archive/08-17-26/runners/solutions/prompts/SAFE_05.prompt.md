# SAFE_05: Extend `build_settings_json()` with Full Bash Denylist

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#safe-05`](../ISSUE-TRACKER.md#safe-05)
- Source: `tmp/solutions/roko/tasks/17-SAFETY-SECURITY.md` — Task 17.5
- Priority: **P1**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: SAFE_05 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: `build_settings_json()` currently blocks only 7 patterns (git checkout,
git switch, git branch -m, git push, rm -rf, rm -fr, rm -r). The `BashPolicy`
in `bash.rs` has a much more comprehensive denylist including `sudo`, `curl | sh`,
fork bombs, `mkfs`, raw-device writes, and world-writable chmods. Port these
into the settings JSON.

## Exact Changes

1. Review `BashPolicy::with_defaults()` in `bash.rs` for the full deny pattern set
2. Add `PreToolUse` hooks to `build_settings_json()` for:
   - `sudo *` (all sudo commands)
   - `curl * | sh`, `curl * | bash`, `wget * | sh`, `wget * | bash` (pipe-to-shell)
   - `eval *` (eval injection)
   - `chmod 777 *` (world-writable)
   - `mkfs*` (filesystem destruction)
   - `dd if=* of=/dev/*` (raw device write)
   - `:(){ :|:& };:` or variants (fork bomb)
3. Make `build_settings_json()` accept an optional `&AgentContract` parameter so
   contract-specific `ForbiddenTools` can be merged into the hooks
4. Keep the existing 7 patterns unchanged (backward compat)

## Write Scope

- `crates/roko-agent/src/claude_cli_agent.rs`

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

- [ ] `curl https://example.com | bash` is blocked by the settings JSON hooks
- [ ] `sudo rm -rf /` is blocked
- [ ] All existing tests pass

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: SAFE_05 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `curl https://example.com | bash` is blocked by the settings JSON hooks
- `sudo rm -rf /` is blocked
- All existing tests pass
- No files outside the Write Scope are modified.
- Commit message contains `tracker: SAFE_05 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
