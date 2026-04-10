# Mori vs Roko: Side-by-Side Comparison

## TL;DR

Mori is a **monolithic, battle-tested** orchestrator with 171 plans worth of real execution. Roko has the same architectural bones but is organized into **separate crates** with cleaner separation. The key difference: mori hardcodes a lot (backend per role, budget per role, prompt assembly), while roko makes it configurable (providers in TOML, role manifests in TOML, 9-layer prompt builder). Roko also adds a second workflow mode (ACP pipeline) that mori doesn't have.

## Architecture

| Aspect | Mori | Roko |
|---|---|---|
| **Codebase** | 1 app (`apps/mori/`) + ~36 crates | 18 crates, cleaner separation |
| **Orchestration** | `apps/mori/src/orchestrator/` (many files) | `crates/roko-cli/src/orchestrate.rs` (1 big file) + `crates/roko-orchestrator/` |
| **Agent system** | `apps/mori/src/agent/` | `crates/roko-agent/` |
| **Gate system** | Inline in orchestrator | `crates/roko-gate/` (separate crate) |
| **Prompt system** | `prompts.rs` (~5500 lines) | `crates/roko-compose/` (separate crate, 9-layer builder) |
| **Learning** | Scattered across orchestrator | `crates/roko-learn/` (separate crate) |
| **Config** | `.mori/config.toml` | `roko.toml` |

## Roles

Both have the same 28 roles. The enum is essentially copy-pasted.

| Aspect | Mori | Roko |
|---|---|---|
| **Role count** | 28 | 28 (same) |
| **Role manifests** | Implicit (hardcoded in prompts.rs) | Formal TOML manifests for 6 core roles |
| **Backend per role** | Hardcoded (`fn backend()`) | Configurable via `[agent.roles]` and `[providers.*]` |
| **Budget per role** | Hardcoded (`fn claude_budget_usd()`) | Configurable via `[budget]` |
| **Priority per role** | Hardcoded (`fn role_priority()`) | Implicit from task ordering |

## Backends / Providers

| Mori | Roko |
|---|---|
| 3 backends: Claude, Codex, Cursor | 6 provider kinds: AnthropicApi, ClaudeCli, OpenAiCompat, CursorAcp, PerplexityApi, GeminiApi |
| Backend hardcoded per role | Provider resolved from model config |
| No API-direct option | Has direct HTTP API providers (Anthropic, OpenAI-compat) |
| Codex via app-server JSON-RPC | OpenAI-compat via HTTP (different protocol) |

## Pipeline State Machine

| Aspect | Mori | Roko (orchestrate.rs) | Roko (ACP pipeline) |
|---|---|---|---|
| **Input** | Plan directory with 15+ artifacts | Plan directory with plan.md + tasks.toml | Single prompt from editor |
| **Phases** | Preflight -> Strategist -> Implementer -> Gates -> Review -> Verdict -> DocRevision -> Commit | Queued -> Enriching -> Implementing -> Gating -> AutoFixing -> Verifying -> Reviewing -> DocRevision -> Merging -> Complete | Pending -> Strategizing -> Implementing -> AutoFixing -> Gating -> Reviewing -> Committing -> Complete |
| **Gate failure** | AutoFix (simple) or back to Implementer, max 3 | AutoFix up to 5 iterations, then replan | AutoFix then back to Implementing, up to max_iterations |
| **Review failure** | REVISE -> Implementer or QuickFix or DocRevision-only | ReviewRejected -> back to Implementing | ReviewRevise -> back to Implementing |
| **Complexity tiers** | Trivial/Simple/Standard/Complex | mechanical/focused/integrative/architectural | Express/Standard/Full |
| **State machine purity** | Yes (pipeline.rs) | Yes (executor/mod.rs) | Yes (pipeline.rs) |

## Task Format

| Field | Mori | Roko |
|---|---|---|
| `id`, `title`, `status` | Yes | Yes |
| `role` per task | No (hardcoded pipeline) | Yes |
| `tier` / complexity | `complexity_band` (fast/standard/complex) | `tier` (mechanical/focused/integrative/architectural) |
| `category` | scaffolding/impl/integration/etc. | Not present |
| `reasoning_level` | low/medium/high | Not present |
| `speed_priority` | latency/balanced/accuracy | Not present |
| `quality_profile` | pragmatic/balanced/hardened | Not present |
| `context_weight` | slim/standard/deep | Not present |
| `parallel_group` | A/B/C groups | DAG-based (depends_on) |
| `exclusive_files` | boolean per task | DAG config `infer_file_overlap` |
| `verify` per task | Not present | Per-task verification pipeline |
| `context` surgical | `context_files` list | Structured: files, symbols, search_patterns |
| `split_into` | Not present | Decomposition subtasks |
| `replan_strategy` | Not present | decompose/retry/escalate |
| `mcp_servers` per task | Not present | Per-task MCP |
| `allowed_tools` / `denied_tools` | Not present | Per-task tool scoping |

### Mori has more routing metadata, Roko has more execution metadata.

## Multi-Task Files Per Plan

| File | Mori | Roko |
|---|---|---|
| `tasks.toml` (implementation) | Yes | Yes |
| `review-tasks.toml` | Yes (Architect+Auditor) | No (role is per-task in tasks.toml) |
| `scribe-tasks.toml` | Yes (module docs, citations, formulas, diagrams) | No |
| `verify-tasks.toml` | Yes | No (verify steps are per-task in tasks.toml) |

Mori separates concerns into 4 task files per plan. Roko unifies into 1 task file with per-task role assignment.

## Prompts

| Aspect | Mori | Roko |
|---|---|---|
| **Location** | `prompts.rs` (5500 lines, all roles) | `crates/roko-compose/` (separate crate) |
| **Architecture** | Monolithic function per role | 9-layer `SystemPromptBuilder` + role templates |
| **Budget** | Per-role PromptBudget (plan, workspace, prd2, etc.) | Token budget profile per builder |
| **Cache alignment** | Manual layer markers | Builder cache tiers (System/Session/Task/Dynamic) |
| **Shared context** | SharedPlanContext (fixed ordering) | Builder composes per-role |
| **AGENTS.md** | Role-filtered global instructions | Not present (role identity in templates) |
| **Review criteria in implementer prompt** | Yes ("What Reviewers Will Check") | Via gate_expectations in role manifest |
| **Affect/emotion** | Not present | Layer 8 (daimon affect guidance) |
| **Pheromones** | Not present | Layer 3c (stigmergic signals) |
| **Anti-patterns** | Not present | Layer 7 |
| **Playbooks** | Learning pack in context | Layer 6 |

## Parallel Execution

| Aspect | Mori | Roko |
|---|---|---|
| **Plan-level parallelism** | Yes (max 5, default 3) | Yes (max 4, configured to 1 currently) |
| **Task-level parallelism** | Yes (parallel_group A/B/C) | Yes (DAG waves) |
| **Worktree isolation** | Yes (per-plan git worktrees) | Infrastructure exists but not tested at scale |
| **Multi-agent pool** | MultiAgentPool (warm spawning, instance scoping) | ProcessSupervisor (simpler) |
| **Merge queue** | Dependency-ordered, one at a time | Yes |
| **Speculative execution** | Not present | Yes (spawn backup after 2x expected time) |
| **Warm agents** | Yes (pre-spawn reviewers) | Config exists but not wired |

## Inter-Agent Communication

| Mechanism | Mori | Roko |
|---|---|---|
| **File-based context** | ContextInjector writes packs to worktree | Task output persistence (save/load) |
| **Review feedback** | Parsed from reviewer output, fed to next iteration | Same pattern |
| **Event bus** | AgentEvent enum (stream updates) | RokoEvent (plan revisions, gate verdicts) |
| **Conductor** | Full meta-agent (nudge, restart, force-advance) | Config exists, not fully wired |
| **Signals** | Not present | Engram system (typed, content-hashed) |
| **Pheromones** | Not present | Stigmergic coordination (time-decaying) |
| **Episodes** | Not present | Episode logging to `.roko/episodes.jsonl` |

## Learning

| Aspect | Mori | Roko |
|---|---|---|
| **Model routing** | 3-tier (fast/standard/complex) from config | CascadeRouter with LinUCB bandit + efficiency history |
| **Gate thresholds** | Static | Adaptive EMA in `.roko/learn/gate-thresholds.json` |
| **Prompt experiments** | Not present | A/B experiments in `.roko/learn/experiments.json` |
| **Efficiency tracking** | Not present | Per-turn events in `.roko/learn/efficiency.jsonl` |
| **Playbook extraction** | Not present | PatternExtractor role + playbook store |
| **Episode history** | Not present | Full episode log with HDC fingerprints |

## What Mori Has That Roko Doesn't

1. **171 real plans** with full artifacts (brief, decomposition, rubric, research, etc.)
2. **Battle-tested parallel execution** at scale (5 plans x 4 agents)
3. **4 separate task files per plan** (impl, review, scribe, verify) with rich type-specific fields
4. **Per-task routing metadata** (7 dimensions: category, reasoning, speed, quality, context, complexity, escalation)
5. **Milestone/queue system** for organizing plans into execution groups
6. **Warm agent pool** with pre-spawning
7. **Provider health tracking** with failure-based cooldown
8. **Conductor meta-agent** that actively intervenes (nudge, restart, force-advance)
9. **Refactorer** that runs batch cleanup every N plans
10. **Scribe task types** (module_doc, citation, formula, diagram, interaction)

## What Roko Has That Mori Doesn't

1. **ACP pipeline** (per-prompt workflow from editors)
2. **6 provider kinds** (vs 3 backends) -- direct API access, not just CLI
3. **Formal role manifests** in TOML (vs hardcoded)
4. **9-layer system prompt builder** with affect/pheromone/anti-pattern layers
5. **Signal/Engram system** for typed, content-hashed inter-agent communication
6. **Pheromone/stigmergic coordination** between agents
7. **Adaptive model routing** (LinUCB bandit vs static 3-tier)
8. **Prompt experiments** (A/B testing)
9. **Gate failure replan** (automatic plan revision)
10. **Speculative execution** (race backup agent)
11. **Per-task verification pipelines** (vs plan-level gates only)
12. **Per-task MCP and tool scoping**
13. **Knowledge store** (neuro) for durable cross-run knowledge
14. **Affect engine** (daimon) for emotional tone modulation
15. **HDC fingerprinting** per episode
