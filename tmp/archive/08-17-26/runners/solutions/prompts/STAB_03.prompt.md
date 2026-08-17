# STAB_03: Auto-provision auth on cloud deploy

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-03`](../ISSUE-TRACKER.md#stab-03)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.03
- Priority: **P0**
- Effort: 2 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_03 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`cmd_deploy_railway()` at line 181 of `server.rs` handles Railway deployment. It writes config
files and invokes the Railway CLI but does not generate or set an API key. The deployed server
binds to `0.0.0.0:6677` with auth disabled, making all ~85 routes publicly accessible.

Same applies to `cmd_deploy_fly()` (line 333) and `cmd_deploy_docker()`.

## Exact Changes

1. At the top of each deploy function (`cmd_deploy_railway`, `cmd_deploy_fly`, `cmd_deploy_docker`):
   - Generate a 32-byte random hex string: `use rand::Rng; let key: String = rand::thread_rng().sample_iter(...).take(32).map(|b| format!("{:02x}", b)).collect();`
   - Or use `uuid::Uuid::new_v4().to_string()` if simpler.
2. Set the API key as an environment variable in the deployment:
   - Railway: add `ROKO_API_KEY` to the Railway service variables via GraphQL or env file.
   - Fly: add to `fly.toml` `[env]` section or `flyctl secrets set`.
   - Docker: add to generated Dockerfile or docker-compose as env var.
3. Set `api_auth.enabled = true` in the generated `roko.toml` for the deployment.
4. Print the generated key with a prominent warning:
   ```
   API Key: {key}
   Save this API key. It will not be shown again.
   ```
5. Add `rand` to `roko-cli` dev-dependencies if not already present.

## Design Guidance

The key generation should be a shared utility function `generate_api_key() -> String` in
a common module (e.g., `config_helpers.rs`), callable from all deploy targets. Consider
adding `--no-auth` flag for deploy commands that explicitly opts out with a warning.

## Write Scope

- `crates/roko-cli/src/commands/server.rs`

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

- [ ] `roko deploy railway` output includes a generated API key
- [ ] Generated deployment config has `api_auth.enabled = true`
- [ ] `ROKO_API_KEY` environment variable is set in deployment

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_03 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko deploy railway` output includes a generated API key
- Generated deployment config has `api_auth.enabled = true`
- `ROKO_API_KEY` environment variable is set in deployment
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_03 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
