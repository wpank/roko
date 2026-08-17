# SAFE_01: Change `dangerously_skip_permissions` Default to `false`

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#safe-01`](../ISSUE-TRACKER.md#safe-01)
- Source: `tmp/solutions/roko/tasks/17-SAFETY-SECURITY.md` — Task 17.1
- Priority: **P0**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: SAFE_01 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: The `ClaudeCliAgent::new()` constructor sets `dangerously_skip_permissions: true`
unconditionally. `RunConfig::default()` does the same. `chat_session.rs` hardcodes
the flag. Change all defaults to `false`. Add a `--skip-permissions` CLI flag that
explicitly opts in with a `tracing::warn!` when used.

## Exact Changes

1. In `claude_cli_agent.rs` line 128, change `dangerously_skip_permissions: true` to `false`
2. In `runner/types.rs` lines 1337 and 1382, change both `dangerously_skip_permissions: true` to `false`
3. In `chat_session.rs` line 1053, make the `--dangerously-skip-permissions` arg conditional
   on a `skip_permissions` config value, not hardcoded
4. Add `--skip-permissions` flag to the `plan run` clap args in `commands/plan.rs`
5. When `--skip-permissions` is passed, set `RunConfig.dangerously_skip_permissions = true`
   and emit `tracing::warn!("running with --skip-permissions: all agent permission checks bypassed")`
6. Update existing tests that depend on the permissive default: search for
   `with_dangerously_skip_permissions(false)` assertions and adjust

## Write Scope

- `crates/roko-agent/src/claude_cli_agent.rs`
- `crates/roko-cli/src/runner/types.rs`
- `crates/roko-cli/src/chat_session.rs`

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

- [ ] `cargo run -p roko-cli -- plan run plans/` does NOT pass `--dangerously-skip-permissions` to the Claude CLI
- [ ] `cargo run -p roko-cli -- plan run plans/ --skip-permissions` passes it and logs a warning
- [ ] `cargo run -p roko-cli -- chat` does NOT hardcode the flag
- [ ] Unit tests that explicitly test the flag continue to pass
- [ ] `RunConfig::default()` has `dangerously_skip_permissions: false`

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: SAFE_01 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Unit tests that explicitly test the flag continue to pass
- `RunConfig::default()` has `dangerously_skip_permissions: false`
- No files outside the Write Scope are modified.
- Commit message contains `tracker: SAFE_01 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
