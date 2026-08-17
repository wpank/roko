# STAB_01: Fix `roko config mcp` unreachable panic

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-01`](../ISSUE-TRACKER.md#stab-01)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.01
- Priority: **P0**
- Effort: 1 hour
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_01 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The `ConfigCmd::Mcp` match arm in `config_cmd.rs` line 209 hits `unreachable!()` which panics.
However, investigation reveals this is actually intercepted in `dispatch_subcommand()` in
`main.rs` at line 2132, which calls `dispatch_mcp_cmd()` defined at line 2790. This function
handles `ConfigMcpCmd::List` and `ConfigMcpCmd::Test` and returns before reaching the
`unreachable!()`.

**Status re-assessment**: The MCP dispatch IS wired and functional. The `unreachable!()` is
defensive dead code because the arm is intercepted earlier. However, it is fragile -- if
`dispatch_subcommand` is refactored to not intercept MCP, the panic returns.

**Remaining fix**: Verify all `ConfigMcpCmd` variants are handled in `dispatch_mcp_cmd()`.
The `Add` and `Remove` subcommands (if they exist in the enum) may not have handlers.
Replace `unreachable!()` with a proper fallback that calls `dispatch_mcp_cmd` as a safety net.

## Exact Changes

1. In `config_cmd.rs`, replace the `unreachable!("mcp dispatched in dispatch_subcommand")` with
   a call to a shared MCP dispatch function, so both paths are safe:
   ```rust
   ConfigCmd::Mcp { cmd } => {
       let wd = resolve_workdir(cli);
       dispatch_mcp_cmd(&cmd, &wd)?;
       Ok(())
   }
   ```
2. Verify all `ConfigMcpCmd` variants (`List`, `Test`, `Add`, `Remove`) have handlers in
   `dispatch_mcp_cmd()`. Add stub handlers for any missing variants that return
   `Err(anyhow!("roko config mcp {variant} is not yet implemented"))`.
3. Add a test: `Cli::try_parse_from(["roko", "config", "mcp", "list"])` runs without panic.

## Design Guidance

The `unreachable!()` pattern in `config_cmd.rs` is used for 4 arms (Experiments, Plugins,
Secrets, MCP). All of these rely on interception in `dispatch_subcommand`. Consider making all
4 arms handle dispatch directly as a fallback, reducing fragility.

## Write Scope

- `crates/roko-cli/src/main.rs`
- `crates/roko-cli/src/commands/config_cmd.rs`

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

- [ ] `roko config mcp list` runs without panic (outputs servers or "no MCP config found")
- [ ] `roko config mcp test <name>` runs without panic
- [ ] No `unreachable!()` remains in the MCP arm of `dispatch_config`

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_01 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko config mcp list` runs without panic (outputs servers or "no MCP config found")
- `roko config mcp test <name>` runs without panic
- No `unreachable!()` remains in the MCP arm of `dispatch_config`
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_01 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
