# SAFE_02: Wire `AgentContract` Into Runner v2 Dispatch

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#safe-02`](../ISSUE-TRACKER.md#safe-02)
- Source: `tmp/solutions/roko/tasks/17-SAFETY-SECURITY.md` — Task 17.2
- Priority: **P0**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: SAFE_02 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: Load the role-appropriate `AgentContract` at agent spawn time.
Apply contract constraints to the Claude CLI invocation: if the contract has
`allowed_tools`, pass them as `--allowedTools`; if the contract has
`ForbiddenTools`, add them to the `build_settings_json()` denylist.

## Exact Changes

1. Add `roko-agent` safety dependency if not already available in the CLI crate
2. When building an `AgentDispatchRequest` in the runner, determine the role
   from `TaskDef.role` (already available in the task TOML)
3. Call `AgentContract::load_for_role_with_mode(role, ContractLoadMode::RestrictedFallback)`
4. If contract has `allowed_tools: Some(tools)`, set `request.tools = tools.join(",")`
5. If contract has `ForbiddenTools` governance rules, merge them into the
   settings JSON hooks as additional `PreToolUse` blockers
6. If contract has `MaxTokensPerTurn(n)`, cap `request.max_turns` accordingly
7. Log the loaded contract at `tracing::info!` level: role, invariant count,
   governance rule count, allowed_tools count
8. Store the contract in the dispatch context for post-execution auditing

## Write Scope

- `crates/roko-cli/src/dispatch_v2.rs`
- `crates/roko-cli/src/runner/event_loop.rs`

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

- [ ] A task with `role = "reviewer"` spawns an agent without `edit_file` in its tool
- [ ] A task with `role = "auditor"` spawns an agent without write tools (auditor
- [ ] A task with `role = "implementer"` gets the full tool set minus network tools
- [ ] Contract load failures in `RestrictedFallback` mode log a warning but do not

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: SAFE_02 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- A task with `role = "reviewer"` spawns an agent without `edit_file` in its tool
- A task with `role = "auditor"` spawns an agent without write tools (auditor
- A task with `role = "implementer"` gets the full tool set minus network tools
- Contract load failures in `RestrictedFallback` mode log a warning but do not
- No files outside the Write Scope are modified.
- Commit message contains `tracker: SAFE_02 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
