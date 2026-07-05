# 02 — System Prompt Integration

> **Priority**: 🔴 P0 — Agents run "naked" without role context
> **Parity sections**: §5 (per-role prompt templates), §4 (prompt composition)
> **Checklist ref**: `MORI-PARITY-CHECKLIST.md` §4, §5

## Problem statement

Roko has a sophisticated 6-layer `SystemPromptBuilder` (`/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/system_prompt_builder.rs`) and 9 role templates (`/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/templates/`), but **nothing calls them**.

- `ClaudeAgent` has no `system` field at all
- `ExecAgent` just pipes stdin→stdout
- `roko-cli/src/run.rs` uses `PromptComposer` for the *user prompt* but never builds a *system prompt*
- The default role prompt is literally `"You are a Roko agent."`

Meanwhile, Mori injects ~2K tokens of role-specific system prompt via `claude_system_prompt(role)` at `/Users/will/dev/uniswap/bardo/apps/mori/src/agent/connection.rs:427-500`.

## What exists (built but unused)

| Component | Path | Status |
|-----------|------|--------|
| SystemPromptBuilder (6 layers) | `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/system_prompt_builder.rs` | ✅ Implemented, ❌ Not called |
| Implementer template | `.../roko-compose/src/templates/implementer.rs` | ✅ Implemented, ❌ Not called |
| Reviewer template | `.../roko-compose/src/templates/reviewer.rs` | ✅ Implemented, ❌ Not called |
| Scribe template | `.../roko-compose/src/templates/scribe.rs` | ✅ Implemented, ❌ Not called |
| Strategist template | `.../roko-compose/src/templates/strategist.rs` | ✅ Implemented, ❌ Not called |
| Task impl template | `.../roko-compose/src/templates/task_impl.rs` | ✅ Implemented, ❌ Not called |
| Integration template | `.../roko-compose/src/templates/integration.rs` | ✅ Implemented, ❌ Not called |
| Quick template | `.../roko-compose/src/templates/quick.rs` | ✅ Implemented, ❌ Not called |
| Common utilities | `.../roko-compose/src/templates/common.rs` | ✅ Implemented, ❌ Not called |
| AGENTS.md loader | `.../roko-compose/src/agents_md.rs` | ✅ Implemented, ❌ Not called |
| Conventions detector | `.../roko-compose/src/conventions.rs` | ✅ Implemented, ❌ Not called |

## Mori's system prompt (what we need to match)

From `/Users/will/dev/uniswap/bardo/apps/mori/src/agent/connection.rs:427-500`:

```
1. Project identity ("You are running inside Mori...")
2. Coding standards (file creation, mod declarations, etc.)
3. Role-specific guidance (mori_role_guidance(role))
4. Tool-specific guidance (mori_tool_usage_guidance())
5. Artifact hints (mori_role_artifact_hint(role))
6. Rules (never git checkout, never add workspace deps, etc.)
```

Roko's `SystemPromptBuilder` has equivalent layers:
```
Layer 1: Role identity (who am I)
Layer 2: Conventions (coding standards)
Layer 3: Domain context (project knowledge)
Layer 4: Task context (current task)
Layer 5: Tool instructions (available tools)
Layer 6: Anti-patterns (what NOT to do)
```

## Checklist

### Wire SystemPromptBuilder into agent backends

- [ ] **2.1** Create `fn build_system_prompt(role: AgentRole, config: &RokoConfig) -> String` in a new file `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/role_prompts.rs`
  - Calls `SystemPromptBuilder::new(role_identity)` with role-specific identity text
  - `.with_conventions(...)` from auto-detected project conventions
  - `.with_domain(...)` from config or AGENTS.md
  - `.with_tool_instructions(...)` from role's tool allowlist
  - `.with_anti_patterns(...)` matching mori's Rules section
- [ ] **2.2** Map each `AgentRole` variant to a template module (Implementer → `templates::implementer`, etc.)
- [ ] **2.3** Add `system_prompt` field to `ClaudeAgent::MessagesRequest`
- [ ] **2.4** `ClaudeAgent::with_system_prompt()` builder method
- [ ] **2.5** `ClaudeCliAgent` (from plan 01) passes `--append-system-prompt`
- [ ] **2.6** `ExecAgent` prepends system prompt to stdin (for non-Claude backends)

### Wire into roko-cli entrypoint

- [ ] **2.7** `run.rs` calls `build_system_prompt()` using config's role + detected conventions
- [ ] **2.8** Replace default `"You are a Roko agent."` with the composed system prompt
- [ ] **2.9** Pass prompt budget from `config.prompt.budget` to truncation logic

### Wire into orchestrator

- [ ] **2.10** `roko-orchestrator` executor's `DispatchAgent` action includes role + task context
- [ ] **2.11** Orchestrator builds per-task system prompts using task_impl template + enrichment data
- [ ] **2.12** Enrichment sections (§3.1-3.13) get composed into Layer 3/4 of SystemPromptBuilder

### Acceptance tests

- [ ] **2.13** Test: `build_system_prompt(AgentRole::Implementer, ...)` produces prompt containing coding standards, tool guidance, and anti-patterns
- [ ] **2.14** Test: system prompt token count stays within config's `context_limit_k` budget
- [ ] **2.15** Test: each of the 9 role templates produces a non-empty, unique prompt
