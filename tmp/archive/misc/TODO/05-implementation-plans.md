# implementation-plans/ — Master Implementation Roadmap

**Directory**: `tmp/implementation-plans/`
**Status**: ACTIVE — critical wiring gaps remain
**Files**: 38 files (12 core plans + 22 model routing + 4 section files)

## Core Plans Summary

| Plan | Title | Status | Key Gap |
|------|-------|--------|---------|
| 01 | Agent Wiring | 70% | ClaudeAgent HTTP `system` field missing; ExecAgent fallback still used |
| 02 | System Prompt Integration | 0% wired | SystemPromptBuilder built but orchestrate.rs uses inline prompts |
| 03 | Safety & Hooks | 70% | SafetyLayer wired to Dispatcher, but Dispatcher not called from CLI |
| 04 | Orchestrator Pipeline | 95% | Done — full plan execution loop |
| 05 | Learning & Feedback | 100% | Done — all learning systems wired |
| 06 | Process Management | 40% | ProcessSupervisor built, not wired into orchestrate.rs |
| 07 | MCP Tool Wiring | Superseded | Partially wired (--mcp-config passed) |
| 08 | Observability | Superseded | TraceSink/MetricsSink exist, not initialized |
| 09 | TUI Dashboard | Superseded | DONE — ratatui wired, F1-F7 tabs |
| 10 | Golem Integration | Superseded | Deferred to 12b |
| 11 | Agent Dogfooding | 30% | Phase 0-2 done; phases 3-8 not started |
| 12a | Cognitive Layer | 20% scaffolded | Crates exist, distillation pipeline not wired |
| 12b | Chain Layer | 10% scaffolded | Intentionally deferred until Tier 1 |

## P0 Wiring Gaps (Blocking Full Self-Hosting)

### 1. SystemPromptBuilder not wired (Plan 02)

**Problem**: `orchestrate.rs` has inline `build_system_prompt()` (lines 17004-17070) with 1-2 sentence prompts per role. The full 9-layer `SystemPromptBuilder` exists but is never called.

**Fix**: Replace inline function with `SystemPromptBuilder::new(role_identity).with_conventions(...).with_domain(...)` calls.

- [ ] Replace `build_system_prompt()` in `orchestrate.rs` with `SystemPromptBuilder` calls
- [ ] Wire conventions detector from `roko-compose`
- [ ] Wire `agents_md` loader from `roko-compose`
- [ ] Use role-specific templates from `crates/roko-compose/src/templates/`

**Source files**:
- Builder: `crates/roko-compose/src/system_prompt_builder.rs` (1,983 lines)
- Templates: `crates/roko-compose/src/templates/` (9 templates)
- Inline prompts: `crates/roko-cli/src/orchestrate.rs:17004-17070`

### 2. ToolDispatcher not called from CLI (Plan 03)

**Problem**: `orchestrate.rs` calls `ExecAgent::run()` directly. `ToolDispatcher` with `SafetyLayer` is built and composed but never invoked from the CLI path.

**Fix**: Wire `ToolDispatcher` into the orchestrate.rs agent dispatch path.

- [ ] Create `ToolDispatcher` with `SafetyLayer` in `orchestrate.rs` init
- [ ] Route agent dispatch through `ToolDispatcher` instead of direct `ExecAgent::run()`
- [ ] Generate `--settings` JSON for Claude CLI (matching mori's `agent_hooks_settings()`)

**Source files**:
- Dispatcher: `crates/roko-agent/src/dispatcher/mod.rs` (1,237 lines)
- Safety: `crates/roko-agent/src/safety/mod.rs` (256 lines)
- Guards: `crates/roko-agent/src/safety/{bash_guard,git_guard,network_guard,path_guard}.rs`
- Orchestrator: `crates/roko-cli/src/orchestrate.rs:545+` (ExecAgent fallback)

### 3. ProcessSupervisor not wired (Plan 06)

**Problem**: `ProcessSupervisor` exists in `roko-runtime` but `orchestrate.rs` doesn't use it. Agent PIDs aren't tracked; no orphan reaping on shutdown.

- [ ] Wire `ProcessSupervisor` into `orchestrate.rs` PlanRunner init
- [ ] Register agent PIDs on spawn
- [ ] Implement orphan reaper on shutdown
- [ ] Add SIGTERM -> SIGKILL escalation

**Source files**:
- Supervisor: `crates/roko-runtime/src/process.rs` (300+ lines)
- Orchestrator spawn: `crates/roko-cli/src/orchestrate.rs:12775-12926`

## Model Routing Plans (modelrouting/)

| Plan | Title | Status |
|------|-------|--------|
| 01 | Architecture | Reference |
| 02 | Provider Registry | Partial (no TOML schema) |
| 03 | Provider Adapters | 80% (all adapters built) |
| 04 | Translator Extensions | Built |
| 05 | GLM Integration | DONE |
| 06 | Kimi Integration | DONE |
| 07 | OpenRouter Universal | DONE |
| 08 | Learning Loops | DONE |
| 09 | Cost Normalization | DONE |
| 10 | Model Experiments | DONE |
| 11 | Research Context | Reference |
| 12 | Advanced Patterns | NOT DONE |
| 13 | Architectural Gaps | Partial |
| 14 | Integration Refinements | Partial |
| 15 | Operational Surface | Partial |
| 16 | Production Hardening | Partial |
| 17 | Meta-Learning | NOT STARTED |
| 18 | Structural Cleanup | Partial |
| 19 | Implementation Guide | Reference |
| 20 | Perplexity Integration | DONE |
| 21 | Gemini Integration | DONE |
| 22 | Research APIs Backlog | Backlog |

Model routing checklist:

- [ ] Establish `ProviderRegistry` TOML schema (`[providers.*]` in roko.toml)
- [ ] Consolidate adapters under unified registry
- [ ] Wire `CascadeRouter` into active model selection decisions
- [ ] Implement advanced patterns (plan 12)
- [ ] Implement meta-learning (plan 17)

**Source files**:
- Adapters: `crates/roko-agent/src/{claude_agent,codex_agent,cursor_agent,openai_agent,ollama_agent,gemini,perplexity_integration,openrouter_integration}.rs`
- Router: `crates/roko-learn/src/cascade_router.rs`
- Tool loop: `crates/roko-agent/src/tool_loop/mod.rs` (1,500+ lines)
- Plans: `tmp/implementation-plans/modelrouting/`

## Plan 11 — Agent Dogfooding

| Phase | Status | What |
|-------|--------|------|
| 0 | DONE | Extract roko-serve, roko-plugin |
| 1 | Partial | Event ingress (webhooks exist, subscriptions incomplete) |
| 2 | Built | MCP servers (GitHub, Slack, Scripts crates exist) |
| 3 | NOT STARTED | 16 agent templates |
| 4 | NOT STARTED | Scheduler/watcher |
| 5 | NOT STARTED | `roko daemon start`, launchd |
| 6 | NOT STARTED | Multi-repo config |
| 7 | DONE | Learning loops (covered by plan 05) |
| 8 | NOT STARTED | Full autonomous PRD workflow |

- [ ] Build agent template schema and 16 templates
- [ ] Wire scheduler/file watcher (`crates/roko-serve/src/fswatcher.rs` exists)
- [ ] Implement `roko daemon start` command
- [ ] Build multi-repo subscription config
- [ ] Wire full PRD -> plan -> execute -> learn pipeline

## Source Files

- **Index**: `tmp/implementation-plans/00-INDEX.md`
- **Core plans**: `tmp/implementation-plans/01-agent-wiring.md` through `12b-chain-layer.md`
- **Model routing**: `tmp/implementation-plans/modelrouting/`
- **Phase details**: `tmp/implementation-plans/11-sections/`
