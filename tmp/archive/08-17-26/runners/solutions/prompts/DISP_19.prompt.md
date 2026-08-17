# DISP_19: Replace run_claude_cognitive_task() in Bridge Events

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#disp-19`](../ISSUE-TRACKER.md#disp-19)
- Source: `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` — Task 3.19
- Priority: **P2**
- Effort: 5 hours
- Depends on: `DISP_18` (source 3.18)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: DISP_19 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`run_claude_cognitive_task()` at `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/bridge_events.rs:1110` spawns `claude --print --output-format stream-json --model <m> --system-prompt <sp>` as a direct subprocess. Better than `run_claude_cli()` (has model and system prompt) but still bypasses the provider system entirely.

`run_openai_compat_cognitive_task()` at line 1140 uses `resolve_model()` from RokoConfig but builds its own HTTP client instead of going through the provider adapter.

Both are called from the cognitive task dispatcher around line 945-972 which switches on provider kind.

## Exact Changes

1. Replace `run_claude_cognitive_task()` with a call through `create_agent_for_model()`:
   ```rust
   let agent = create_agent_for_model(config, model, &AgentOptions {
       system_prompt: Some(system_prompt.to_string()),
       workdir: Some(workdir.to_path_buf()),
       ..Default::default()
   })?;
   ```
2. Replace `run_openai_compat_cognitive_task()` similarly -- the provider adapter handles HTTP client construction and all provider-specific quirks
3. The cognitive task dispatcher at line 945-972 should no longer switch on provider kind -- `create_agent_for_model()` handles routing internally
4. Remove the direct `Command::new("claude")` call and the manual `reqwest::Client` construction
5. Parse the agent result into the expected cognitive task output format

## Design Guidance

The bridge events module should be provider-agnostic. It asks "run this prompt" and gets back a result. The provider adapter layer (7 adapters) handles all the provider-specific details. The cognitive task dispatcher should look like:
```rust
let agent = create_agent_for_model(config, model, &options)?;
let result = agent.run(&engram, &ctx).await?;
parse_cognitive_result(result)
```

No more switching on `ProviderKind` in the bridge.

## Write Scope

- `crates/roko-acp/src/bridge_events.rs`

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

- [ ] `grep -n 'Command::new("claude")' crates/roko-acp/src/bridge_events.rs` returns zero results
- [ ] `grep -n 'reqwest::Client' crates/roko-acp/src/bridge_events.rs` returns zero results (outside tests)
- [ ] Cognitive tasks work for both Claude and OpenAI-compatible models

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: DISP_19 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `grep -n 'Command::new("claude")' crates/roko-acp/src/bridge_events.rs` returns zero results
- `grep -n 'reqwest::Client' crates/roko-acp/src/bridge_events.rs` returns zero results (outside tests)
- Cognitive tasks work for both Claude and OpenAI-compatible models
- No files outside the Write Scope are modified.
- Commit message contains `tracker: DISP_19 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
