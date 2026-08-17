# SAFE_13: Auto-Provision Auth on Cloud Deploy

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#safe-13`](../ISSUE-TRACKER.md#safe-13)
- Source: `tmp/solutions/roko/tasks/17-SAFETY-SECURITY.md` — Task 17.13
- Priority: **P0**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: SAFE_13 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: `roko deploy railway` (and fly, docker) creates a public deployment
without auth. Anyone who discovers the URL has full control of the agent runtime.
Generate a random API key during deploy and set `api_auth.enabled = true`.

## Exact Changes

1. In the deploy command, generate a random 32-byte API key:
   `hex::encode(rand::random::<[u8; 32]>())`
2. Set `api_auth.enabled = true` and `api_auth.keys = [{ hash = sha256(key) }]`
   in the deployed config
3. Print the API key to stdout: "Your API key: rk-{key}. Save this — it cannot
   be recovered."
4. Set the key as an environment variable in the deployment:
   `ROKO_API_KEY=rk-{key}`
5. Update `roko doctor` to warn when `api_auth.enabled = false` and the bind
   address is not loopback

## Write Scope

- `crates/roko-cli/src/commands/`

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

- [ ] `roko deploy railway` outputs an API key and sets auth enabled
- [ ] The deployed server rejects unauthenticated requests with 401
- [ ] `roko doctor` warns about disabled auth on non-loopback binds

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: SAFE_13 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko deploy railway` outputs an API key and sets auth enabled
- The deployed server rejects unauthenticated requests with 401
- `roko doctor` warns about disabled auth on non-loopback binds
- No files outside the Write Scope are modified.
- Commit message contains `tracker: SAFE_13 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
