# DISP_18: Replace run_claude_cli() in ACP Runner with Provider Adapter

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#disp-18`](../ISSUE-TRACKER.md#disp-18)
- Source: `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` — Task 3.18
- Priority: **P2**
- Effort: 6 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: DISP_18 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`run_claude_cli()` at `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/runner.rs:1849` is the most bare-bones LLM invocation in the codebase. It spawns `claude --print --dangerously-skip-permissions` with no model flag, no streaming, no system prompt, and no feedback. Called from line 1629.

The `ClaudeCliAdapter` at `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/claude_cli.rs` handles all subprocess construction properly: model selection, system prompt injection, tool allowlist, MCP config, effort settings, safety hooks, streaming output, error classification, and usage tracking.

## Exact Changes

1. Add `roko-agent = { path = "../roko-agent" }` to `crates/roko-acp/Cargo.toml` if not already present
2. Thread `RokoConfig` through the ACP runner context. It may already be available via `PipelineConfig` or similar.
3. Replace the `run_claude_cli()` function body with:
   ```rust
   let agent = create_agent_for_model(config, &model_key, &AgentOptions {
       system_prompt: Some(system_prompt),
       workdir: Some(workdir.to_path_buf()),
       ..Default::default()
   })?;
   let engram = Engram::new(Kind::Text, Body::from(prompt));
   let ctx = Context::for_workdir(workdir);
   let result = agent.run(&engram, &ctx).await;
   ```
4. Update the caller at line 1629 to pass model and config
5. Remove the `Command::new("claude")` import if no longer used
6. Preserve the cancellation token integration (`cancel_token` parameter)

## Design Guidance

The ACP runner should not know about CLI subprocess details. It should request "run this prompt with this model" and get back a result. The provider adapter handles all the subprocess mechanics. If the ACP pipeline needs streaming output, use `AgentOptions { streaming: true }` and handle events through the existing `AgentRuntimeEvent` system.

Keep the `cancel_token` parameter -- wrap the agent call with `tokio::select!` against the cancellation.

## Write Scope

- `crates/roko-acp/src/runner.rs`
- `crates/roko-acp/Cargo.toml`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `grep -n 'Command::new("claude")' crates/roko-acp/src/runner.rs` returns zero results
- [ ] `grep -n 'create_agent_for_model' crates/roko-acp/src/runner.rs` shows at least one call
- [ ] ACP runner still works with cancellation

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: DISP_18 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `grep -n 'Command::new("claude")' crates/roko-acp/src/runner.rs` returns zero results
- `grep -n 'create_agent_for_model' crates/roko-acp/src/runner.rs` shows at least one call
- ACP runner still works with cancellation
- No files outside the Write Scope are modified.
- Commit message contains `tracker: DISP_18 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
