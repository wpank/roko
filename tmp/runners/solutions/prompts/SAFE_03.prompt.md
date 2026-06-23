# SAFE_03: Wire `AgentContract` Into WorkflowEngine (V2 `roko run` Path)

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#safe-03`](../ISSUE-TRACKER.md#safe-03)
- Source: `tmp/solutions/roko/tasks/17-SAFETY-SECURITY.md` — Task 17.3
- Priority: **P1**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: SAFE_03 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: The `WorkflowEngine` dispatches agents via `EffectDriver` and
`ModelCallService`. This path must also load and enforce contracts. The provider
options struct at `provider/mod.rs:534` already has a
`dangerously_skip_permissions: bool` field. Add contract enforcement alongside it.

## Exact Changes

1. In `EffectDriver`, when spawning an agent for a role, load the contract
2. Pass the contract's tool constraints through to the provider options
3. In `provider/claude_cli.rs:54`, the Claude CLI provider already reads
   `options.dangerously_skip_permissions`. Add parallel logic to read
   contract tool restrictions from the options
4. When the contract forbids a tool category, reflect it in the CLI args
5. Record contract enforcement in the feedback event

## Write Scope

- `crates/roko-runtime/src/effect_driver.rs`
- `crates/roko-agent/src/provider/claude_cli.rs`
- `crates/roko-agent/src/provider/mod.rs`

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

- [ ] `roko run "review this code"` dispatches a reviewer with read-only tools
- [ ] `roko run "fix this bug"` dispatches an implementer with edit tools allowed
- [ ] Contract violations are logged (not silently swallowed)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: SAFE_03 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko run "review this code"` dispatches a reviewer with read-only tools
- `roko run "fix this bug"` dispatches an implementer with edit tools allowed
- Contract violations are logged (not silently swallowed)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: SAFE_03 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
