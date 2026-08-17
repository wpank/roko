# W0-A: Strip Tools from PRD Draft Generation (Speed Fix)

**Priority**: P0 — demo takes 4 minutes for `prd draft new`, should take 15-30 seconds
**Effort**: 30 minutes
**Files to modify**: 2 files
**Dependencies**: None

## Problem

`roko prd draft new` takes ~4 minutes (229 seconds) because the agent gets full tool access and does 3 iterations of read_file/write_file tool calls before producing output. The prompt explicitly encourages tool use: "If you have file tools, read the codebase to understand what exists and write the PRD directly."

From the logs:
- iteration=0: 4 read_file calls (agent exploring codebase)
- iteration=1: 1 write_file call (agent writes to file)
- iteration=2: 1 read_file call (agent re-reads to verify)
- iteration=3: stop, final_text=1793 chars

With tools stripped, the agent would produce output in a single LLM call (~10-30 seconds).

## Root Cause

### 1. `AgentExecOpts` has no tool restriction field

`crates/roko-cli/src/agent_exec.rs` line 25-45: The `AgentExecOpts` struct has no `tools` field. Line 142 hardcodes `tools: None` in `SpawnAgentSpec`, which means "use provider defaults" — and for OpenAI-compat providers, this means full tool access.

### 2. The prompt tells the agent to use tools

`crates/roko-cli/src/commands/prd.rs` lines 367-378 (system prompt) and 413-422 (task prompt) both say "If you have file tools, read the codebase...". This guarantees the agent will use tools when available.

### 3. For OpenAI-compat backends (glm51), tools = OpenAI function calling

The deployed demo uses `glm51` (Zhipu GLM-5.1) via OpenAI-compat. The tool loop in `crates/roko-agent/src/tool_loop/mod.rs` dispatches tool calls through the OpenAI-compat backend, which makes separate API calls for each tool iteration. Each iteration adds 30-60 seconds of latency.

## Exact Code to Change

### File 1: `crates/roko-cli/src/agent_exec.rs`

#### Change 1: Add `tools` field to `AgentExecOpts` (line 25-45)

**Add after `role` field (line 44):**
```rust
    /// Logical role used to scope safety policies and model routing.
    pub role: Option<&'a str>,
    /// Tool restriction. `Some("none")` disables all tools. `None` uses provider defaults.
    pub allowed_tools: Option<&'a str>,
```

#### Change 2: Thread `allowed_tools` to `SpawnAgentSpec` (line 134-152)

**Change line 142 from:**
```rust
            tools: None,
```

**To:**
```rust
            tools: opts.allowed_tools.map(str::to_string),
```

#### Change 3: Update ALL callers of `AgentExecOpts`

Search for all construction sites:
```bash
grep -rn 'AgentExecOpts {' crates/roko-cli/src/ --include='*.rs'
```

Each one needs `allowed_tools: None,` added (to preserve existing behavior). The key ones to add `allowed_tools: Some("none")` are the PRD and plan generation paths.

### File 2: `crates/roko-cli/src/commands/prd.rs`

#### Change 1: Rewrite PRD draft prompt (lines 367-378)

**Replace the system prompt (lines 367-378) from:**
```rust
                let system = roko_cli::prd::prd_agent_prompt(
                    &workdir,
                    &format!(
                        "Fill in the draft PRD at {path}. \
                         If you have file tools, read the codebase to understand what exists \
                         and write the PRD directly to {path}. \
                         If you do NOT have file tools, output the complete PRD markdown \
                         (with YAML frontmatter) as your response — do not wrap in code fences. \
                         Follow the PRD quality standards in your system prompt exactly.",
                        path = target.display()
                    ),
                );
```

**With:**
```rust
                let system = roko_cli::prd::prd_agent_prompt(
                    &workdir,
                    &format!(
                        "Fill in the draft PRD at {path}. \
                         Output the complete PRD markdown (with YAML frontmatter) as your response. \
                         Do NOT use file tools — they are not available. \
                         Do NOT wrap in code fences. \
                         Follow the PRD quality standards in your system prompt exactly.",
                        path = target.display()
                    ),
                );
```

#### Change 2: Rewrite PRD draft task prompt (lines 413-422)

**Replace:**
```rust
                let task_prompt = format!(
                    "Generate a complete PRD for: {title}. \
                     If you have file tools available, search the codebase to understand \
                     what exists and write the completed PRD to {path}. \
                     Otherwise, output the complete PRD markdown with YAML frontmatter. \
                     Include specific requirements, machine-verifiable acceptance criteria, \
                     and a design section.{context_suffix}",
                    path = target.display(),
                    context_suffix = context_suffix
                );
```

**With:**
```rust
                let task_prompt = format!(
                    "Generate a complete PRD for: {title}. \
                     Output the complete PRD markdown with YAML frontmatter. \
                     Include specific requirements, machine-verifiable acceptance criteria, \
                     and a design section.{context_suffix}",
                    context_suffix = context_suffix
                );
```

#### Change 3: Pass `allowed_tools: Some("none")` in the AgentExecOpts (line 428-437)

**Add to the `AgentExecOpts` construction:**
```rust
                let (exit_code, output) = run_agent_capture_silent(AgentExecOpts {
                    prompt: &task_prompt,
                    workdir: &workdir,
                    model: Some(model_key.as_str()),
                    effort: effort_ref,
                    system_prompt: Some(&system),
                    resume_session,
                    env_vars: &gw.vars,
                    role: Some("scribe"),
                    allowed_tools: Some("none"),  // ← ADD THIS
                })
                .await?;
```

### File 3: `crates/roko-cli/src/prd.rs` — Plan generation

#### Change: Pass `allowed_tools: Some("Read,Grep,Glob")` for plan generation (line 1019-1034)

Plan generation needs READ access (to understand codebase structure) but NOT write access. The prompt already says "Do NOT create files directly".

```rust
        let (exit_code, output) = run_agent_capture_logged(
            AgentExecOpts {
                prompt: &task_prompt,
                workdir: workdir_ref,
                model: model.or_else(|| resolved.config.agent.model.as_deref()),
                effort: Some(resolved.config.agent.effort.as_str()),
                system_prompt: Some(&system),
                resume_session: None,
                env_vars: &resolved.config.agent.env,
                role: Some("strategist"),
                allowed_tools: Some("Read,Grep,Glob"),  // ← READ-ONLY tools
            },
            AgentExecEpisode {
                task_kind: "prd-plan-generate",
                task_id: &task_id,
            },
        )
        .await?;
```

**Why read-only, not none**: The plan generation prompt says "You may read up to 5 codebase files to understand existing structure". This is useful — the agent produces better plans when it can see the actual crate structure. But write tools must be stripped so the agent outputs TOML to stdout (not via write_file).

### How tools get passed to the backend

In `crates/roko-cli/src/agent_spawn.rs` line 23, `SpawnAgentSpec.tools` is `Option<String>`. This flows to `AgentOptions.tools` (line 51) which flows to the backend.

For **OpenAI-compat backends** (glm51), check how `tools` is used:
```bash
grep -rn 'tools\|AgentOptions' crates/roko-agent/src/provider/openai_compat.rs | head -20
```

The `tools` field may need to map to either:
- An empty list of function definitions (to disable function calling)
- Or a filtered list of tool definitions

If `tools: Some("none")` doesn't work as a string, you may need to change the field type or add special handling in the OpenAI-compat backend to recognize "none" as "no tools".

## Agent Prompt

```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W0-A-strip-tools-prd-draft.md and implement all changes described in it. Add `allowed_tools` field to AgentExecOpts in agent_exec.rs, thread it through to SpawnAgentSpec, update all callers to pass None (default), and pass "none" for prd draft new and read-only tools for prd plan. Also rewrite the prompts in commands/prd.rs to remove "if you have file tools" language. Do NOT run cargo build/test/clippy/fmt — compilation is deferred. Mark the checklist items as done.
```

## Commit

This batch is committed with Wave 0 (critical speed fixes). Do not commit individually.

## Checklist

- [x] Add `allowed_tools: Option<&'a str>` to `AgentExecOpts`
- [x] Thread `allowed_tools` through to `SpawnAgentSpec.tools` in `run_agent_capture_impl()`
- [x] Update ALL `AgentExecOpts` construction sites to include `allowed_tools` field
- [x] Pass `allowed_tools: Some("none")` for `prd draft new`
- [x] Pass `allowed_tools: Some("Read,Grep,Glob")` for `prd plan` generation
- [x] Rewrite system prompt in commands/prd.rs to remove "if you have file tools" language
- [x] Rewrite task prompt in commands/prd.rs to remove file tool encouragement
- [ ] Verify the OpenAI-compat backend respects the tools parameter
