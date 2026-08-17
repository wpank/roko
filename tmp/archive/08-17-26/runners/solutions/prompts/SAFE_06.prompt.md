# SAFE_06: Wire `NetworkPolicy` Into Dispatch

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#safe-06`](../ISSUE-TRACKER.md#safe-06)
- Source: `tmp/solutions/roko/tasks/17-SAFETY-SECURITY.md` — Task 17.6
- Priority: **P1**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: SAFE_06 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: `NetworkPolicy` has complete SSRF blocking (private IPs, link-local,
cloud metadata), scheme enforcement (HTTPS-only), and host allow/deny lists.
None of this is called from the dispatch path. Wire it into `build_settings_json()`
as `PreToolUse` hooks that block network commands matching denied patterns.

## Exact Changes

1. Add `PreToolUse` hooks that block `Bash(curl http://10.*)`,
   `Bash(curl http://127.*)`, `Bash(curl http://169.254.*)`,
   `Bash(curl http://192.168.*)`, `Bash(curl http://172.16.*)`
   and similar patterns for `wget`, `nc`, `telnet`
2. For agents with `NoNetworkAccess` invariant (reviewer, auditor, scribe),
   block ALL network commands: `curl *`, `wget *`, `http *`
3. Make the allowlist configurable via `[safety.network.allowlist]` in roko.toml
4. Default allowlist: `crates.io`, `docs.rs`, `github.com`, `api.github.com`,
   `registry.npmjs.org`

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

- [ ] An agent attempting `curl http://169.254.169.254/metadata` is blocked (SSRF)
- [ ] An agent with `NoNetworkAccess` attempting `curl https://example.com` is blocked
- [ ] `curl https://crates.io/api/v1/crates/serde` is allowed for roles that permit network
- [ ] Network blocks are visible in settings JSON hooks

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: SAFE_06 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- An agent attempting `curl http://169.254.169.254/metadata` is blocked (SSRF)
- An agent with `NoNetworkAccess` attempting `curl https://example.com` is blocked
- `curl https://crates.io/api/v1/crates/serde` is allowed for roles that permit network
- Network blocks are visible in settings JSON hooks
- No files outside the Write Scope are modified.
- Commit message contains `tracker: SAFE_06 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
