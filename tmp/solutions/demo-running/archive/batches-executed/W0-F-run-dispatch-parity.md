# W0-F: Unify `roko run` Dispatch Paths (System Prompt + Tools + Playbooks)

**Priority**: P0 — `roko run` with non-Claude models is missing system prompt, tool filtering, and playbooks
**Effort**: 1.5 hours
**Files to modify**: 1-2 files
**Dependencies**: W0-D (routing fix)

## Problem

`dispatch_agent()` in `run.rs` has **5 separate dispatch paths** with wildly different capabilities:

| Path | Trigger | System Prompt | Tools CSV | Playbooks | MCP |
|------|---------|---------------|-----------|-----------|-----|
| A: Provider Routing | `use_provider_routing` | Yes | Yes | Yes | Yes |
| B: Anthropic API | `"claude" + api key` | Yes | Yes (in loop) | Yes | Yes |
| C: Claude CLI | `"claude"` | Yes | Yes | Yes | Yes |
| D: Ollama | `"ollama"` | Yes | Yes (in loop) | Yes | N/A |
| E: Known Protocol | `is_known_protocol_command()` | **NO** | **NO** | **NO** | **NO** |
| F: Generic Subprocess | fallback | **NO** | **NO** | **NO** | **NO** |

After W0-D fixes the routing decision, most traffic on Railway will go through Path A (provider routing), which is correct. But Paths E and F are still dangerously underconfigured for any scenario where they're reached.

The real fix is to ensure ALL paths get at minimum: system prompt, tool filtering by role, and playbook injection.

## Root Cause

Paths E and F at `run.rs:2000-2084` construct `SpawnAgentSpec` with:
```rust
system_prompt: None,    // No guidance
tools: None,            // All tools available (no role filtering)
mcp_config: None,       // No MCP
```

And they skip the `augment_system_prompt_for_strategy()` call that injects playbooks.

## Exact Code to Change

### Fix 1: Extract system prompt + tool building into a shared function

**File**: `crates/roko-cli/src/run.rs`

Currently, the system prompt building + playbook augmentation is inline in Path A (lines 1867-1878) and duplicated in Path B (lines 1909+). Extract to a helper:

```rust
/// Build the system prompt and tool CSV for any dispatch path.
async fn prepare_agent_context(
    config: &Config,
    workdir: &Path,
    prompt_text: &str,
    model: &str,
    strategy: Option<BenchStrategy>,
) -> (String, String, Vec<String>) {
    let tools_csv = claude_tool_allowlist(&config.prompt.role);
    let StrategyPromptAugmentation {
        system_prompt,
        injected_playbook_ids,
    } = augment_system_prompt_for_strategy(
        build_system_prompt(config, prompt_text, &tools_csv),
        workdir,
        &config.prompt.role,
        prompt_text,
        model,
        strategy,
    )
    .await;
    (system_prompt, tools_csv, injected_playbook_ids)
}
```

### Fix 2: Use the helper in Path E (known protocol)

**File**: `crates/roko-cli/src/run.rs` — lines 2000-2042

**Before** `let agent = spawn_agent_scoped(...)`, add:
```rust
        let (system_prompt, tools_csv, injected_playbook_ids) =
            prepare_agent_context(config, workdir, prompt_text, &model, strategy).await;
```

**Change the SpawnAgentSpec:**
```rust
            SpawnAgentSpec {
                model: model.clone(),
                command: Some(config.agent.command.clone()),
                timeout_ms: Some(config.agent.timeout_ms),
                system_prompt: Some(system_prompt),     // WAS: None
                cached_content: None,
                tools: Some(tools_csv),                 // WAS: None
                mcp_config: config.agent.mcp_config.clone(), // WAS: None
                working_dir: Some(workdir.to_path_buf()),
                env: config.agent.env.clone(),
                extra_args: config.agent.args.clone(),
                effort: Some(config.agent.effort.clone()),
                bare_mode: config.agent.bare_mode,
                dangerously_skip_permissions: false,
                name: String::new(),
                role: Some(normalized_role_label(&config.prompt.role)),
            },
```

**Change the DispatchOutcome:**
```rust
        Ok(DispatchOutcome {
            agent_result: agent.run(prompt, ctx).await,
            external_actions: Vec::new(),
            injected_playbook_ids,   // WAS: Vec::new()
            model_selection: resolved_cli_model.clone(),
        })
```

### Fix 3: Same treatment for Path F (generic subprocess)

Apply identical changes to the generic subprocess path at lines 2043-2084.

### Fix 4: Deduplicate Path A

Path A at lines 1861-1908 can now use the same helper, reducing the inline code.

## Why This Matters Beyond the Immediate Bug

Even after W0-D fixes routing to use Path A for Railway, the other paths remain broken for:
- Local development with non-standard agent commands
- Future provider kinds that might not match "claude"
- Any scenario where provider routing isn't available

Making all paths provide the same baseline context prevents this class of bug entirely.

## Agent Prompt

```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W0-F-run-dispatch-parity.md and implement all changes. Extract system prompt + tool CSV building into a helper function, then use it in all 5 dispatch paths in run.rs dispatch_agent(). The goal is that every path gets system_prompt, tools CSV, playbook injection, and MCP config. Do NOT run cargo build/test/clippy/fmt. Mark the checklist items as done.
```

## Commit

This batch is committed with Wave 0 (critical pipeline fixes). Do not commit individually.

## Checklist

- [x] Extract `prepare_agent_context()` helper function in `run.rs`
- [x] Path E (known protocol): add system_prompt, tools, mcp_config, playbooks
- [x] Path F (generic subprocess): add system_prompt, tools, mcp_config, playbooks
- [x] Path A (provider routing): use the shared helper
- [x] Verify: all dispatch paths pass system prompt to agent
- [x] Verify: all dispatch paths pass role-filtered tools CSV
