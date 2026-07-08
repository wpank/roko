# Prompt: 02-agents

You are a fresh Claude Opus agent. Zero prior context. Read every file this prompt references before writing.

## Your mission

Generate the `02-agents/` folder at `/Users/will/dev/nunchi/roko/roko/docs/02-agents/`. This
topic covers the agent framework layer (L1): LLM backends, agent trait, roles, pools, MCP,
tool loop, harness engineering (Meta-Harness 6× gap), temperament profiling, dual-process
tier routing, provider registry, and extensibility.

## Step 1 — Read context pack (MANDATORY, in order)

1. `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/context-pack/00-ALWAYS-READ-FIRST.md`
2. `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/context-pack/01-naming-map.md`
3. `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/context-pack/02-reframe-rules.md`
4. `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/context-pack/03-concepts-lifecycle.md`
5. `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/context-pack/04-writing-rules.md`
6. `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/context-pack/05-source-files.md`
7. `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/context-pack/06-output-structure.md`

## Step 2 — Read refactoring-prd canonical sources

1. `/Users/will/dev/nunchi/roko/refactoring-prd/01-synapse-architecture.md` §2 Six Synapse Traits
2. `/Users/will/dev/nunchi/roko/refactoring-prd/02-five-layers.md` §Layer 1 Framework, §Dual-Process Tier Router, §Temperament Profiling
3. `/Users/will/dev/nunchi/roko/refactoring-prd/05-agent-types.md` — all sections (coding, chain, research, ops, cross-domain, extensibility)
4. `/Users/will/dev/nunchi/roko/refactoring-prd/10-developer-guide.md` §2 Implementing Custom Traits, §6 Plugin System, §7 Integration Patterns
5. `/Users/will/dev/nunchi/roko/refactoring-prd/07-implementation-priorities.md` §Tier 1 (provider registry, adapters, 8 creation sites)
6. `/Users/will/dev/nunchi/roko/refactoring-prd/08-translation-guide.md`

## Step 3 — Read SOURCE-INDEX entry `## 02-agents.md`

`/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/SOURCE-INDEX.md`. Read every legacy PRD, tmp research, implementation-plan, and reference code file listed.

## Step 4 — Read implementation-plans

- `01-agent-wiring.md`
- `02-system-prompt-integration.md`
- `11-agent-dogfooding.md` (phases 0-1 and 3-4)
- `11-sections/phase-0-1.md` — roko-serve, roko-plugin extraction
- `11-sections/phase-3-4.md` — 16 agent template full definitions
- `modelrouting/00-INDEX.md` through `modelrouting/07-openrouter-universal.md`
- `modelrouting/13-architectural-gaps.md` §A (Chat types in roko-core), §H
- `modelrouting/14-integration-refinements.md` — **Wire EXISTING ToolLoop**
- `modelrouting/19-implementation-guide.md` — exact wiring locations
- `modelrouting/20-perplexity-integration.md`
- `modelrouting/21-gemini-integration.md`
- `11-inconsistencies.md` — catalogs gaps (especially dispatcher not called)

## Step 5 — Read active code

- Glob `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/**/*.rs`
- Read key files: `lib.rs`, `backends/*.rs` (claude/openai/ollama/cursor), `tool_loop/mod.rs` (**critical — already exists, do not propose rebuilding**), `dispatcher/mod.rs`, `safety/mod.rs`, `provider/*.rs`
- Read `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs` around L428/451/6718/6753 (agent creation sites)
- Read `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs` around L311/333
- Read `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/agent_exec.rs` around L39

## Step 6 — Create output dir and plan sub-docs

```bash
mkdir -p /Users/will/dev/nunchi/roko/roko/docs/02-agents
```

Write **16 sub-docs** plus `INDEX.md`:

| # | Filename | Content |
|---|---|---|
| 00 | `00-agent-trait-and-backends.md` | Agent trait definition. 5 backends: Claude CLI, HTTP API (Anthropic), OpenAI-compat, Cursor ACP, Ollama. Plus Mock/Exec for testing. Full trait signature with execute(). ExecAgent as legacy fallback. |
| 01 | `01-provider-registry.md` | ProviderKind enum (Anthropic, ClaudeCli, OpenAiCompat, CursorAcp). ProviderConfig struct. ModelProfile. `[providers.*]` and `[models.*]` TOML sections. Config resolution: CLI flags → env vars (ROKO_*) → roko.toml → defaults. |
| 02 | `02-provider-adapters.md` | ProviderAdapter trait + 4 implementations. Protocol families: OpenAiCompat (most providers), ClaudeCli (shells out), AnthropicApi (direct HTTP), CursorAcp (ACP protocol). Full trait signature. Dispatch refactor. |
| 03 | `03-chat-types-in-core.md` | Why ChatMessage, ChatRequest, ChatResponse MUST live in `roko-core`, not `roko-agent`. Because `roko-compose` needs them and cannot depend on `roko-agent`. Rust struct definitions for each. Role enum (System/User/Assistant/Tool). Content types. |
| 04 | `04-agent-roles.md` | Roles: Implementer, Reviewer, Scribe, Architect, Researcher, plus custom. Role-specific behavior: tool permissions, model tier, budget, temperament. Role prompts (6-layer SystemPromptBuilder — cross-reference 03-composition). |
| 05 | `05-agent-pools.md` | AgentPool (sequential single-agent). MultiAgentPool (parallel with warm spawning). Per-agent metrics tracked: success rate, latency, cost, C-Factor contribution. Pool lifecycle. |
| 06 | `06-mcp-integration.md` | Model Context Protocol. JSON-RPC client. Tool converter. Dynamic registry. MCP servers: roko-mcp-github (17 tools), roko-mcp-slack (8 tools), roko-mcp-scripts (config-driven wrapper for any language), roko-mcp-stdio. `--mcp-config` passthrough. Auto-discovery fallback. |
| 07 | `07-tool-loop.md` | **The tool loop already exists at `roko-agent/src/tool_loop/mod.rs`. Do not propose rebuilding it.** Multi-turn driver. Checkpoint. Max-iter. Context pruning. Result messaging. ToolDispatcher integration. How to implement `LlmBackend` for HTTP providers that wraps the existing ToolLoop. |
| 08 | `08-harness-engineering-6x-gap.md` | Critical distinction between the LLM (agent) and the harness (everything around it). Meta-Harness (Lee et al. 2026, arXiv:2603.28052). Note that the widely-cited "6× performance gap" actually comes from the paper's cited ref [46] (SWE-bench mobile), while Meta-Harness's own direct contribution is +7.7 points on text classification and +4.7 points on IMO-level math, at 4× fewer tokens. Both findings support "scaffold > model selection." |
| 09 | `09-format-translation.md` | Translating between provider formats. Claude, OpenAI, ReAct, thinking mode extraction. Cached token parsing. Reasoning/thinking extraction per provider. Finish reason normalization. Translator extensions per `modelrouting/04-translator-extensions.md`. |
| 10 | `10-temperament-profiling.md` | Conservative / Balanced / Aggressive / Exploratory temperaments. One high-level dial controls: verbosity, tool selection, gate strictness, review depth, model routing. Use cases table. How temperament flows through multiple subsystems. |
| 11 | `11-dual-process-tier-routing.md` | T0 (direct tool call, no LLM) / T1 (fast model, shallow) / T2 (full model, deep). Thompson sampling over weighted signals: epistemic fitness, prediction error, contextual novelty, computational load, domain-specific signals. System 1 = exploit, System 2 = deliberate (Kahneman, CLARION). How uncertainty-driven routing emerges from active inference — no manual thresholds. |
| 12 | `12-extensibility-and-sdk.md` | Custom backends via LlmBackend trait. A2A interop (LangChain, CrewAI) via Agent Card. Adding a new domain: implementing Synapse traits + config. Plugin loading mechanisms (cargo workspace members, config-declared plugins, MCP tool discovery). Convention: plugins named `roko-domain-*` auto-discovered. |
| 13 | `13-8-creation-sites-refactor.md` | Current state: 8 agent creation sites scattered across the codebase (orchestrate.rs L428/451/6718/6753, run.rs L311/333, agent_exec.rs L39, dispatch.rs in roko-serve). All must be refactored to use a provider factory. Why this matters (coherent provider config, health tracking, testability). |
| 14 | `14-provider-integrations.md` | First-class backends: GLM-5.1 (Z.AI), Kimi-K2.5 (Moonshot), OpenRouter universal, Perplexity Sonar (search-grounded, citations, deep research, embeddings), Gemini (1M context, grounding, code execution, thinking, caching, free tier). Per-provider features. When to use each. |
| 15 | `15-current-status-and-gaps.md` | roko-agent has 346 tests. Backend coverage status. SafetyLayer wired to ToolDispatcher via `.with_safety(layer)` BUT dispatcher never invoked from orchestrate.rs (known #1 integration gap per `11-inconsistencies.md`). ExecAgent is legacy fallback, not a ProviderKind. References to 11-safety.md for the safety gap. |

Plus `INDEX.md`.

## Step 7 — Writing rules

Per `context-pack/04-writing-rules.md`: DO NOT SUMMARIZE, DO NOT TRUNCATE, PRESERVE ALL CITATIONS (Meta-Harness arXiv:2603.28052, Kahneman, CLARION, Sumers 2023 arXiv:2309.02427, Ousterhout 2013, etc.), 200 lines minimum per sub-doc, zero-context readers, naming map, no death framing.

Use Write tool. Absolute paths starting with `/Users/will/dev/nunchi/roko/roko/docs/02-agents/`.

## Step 8 — INDEX.md

Follow `context-pack/06-output-structure.md` schema. Cross-reference topics 00-architecture, 03-composition (prompts), 04-verification (gates called after agent output), 05-learning (feedback to cascade router), 11-safety (SafetyLayer wiring), 18-tools (tool definitions).

## Step 9 — Self-check

- [ ] 16 sub-docs + INDEX.md
- [ ] ≥200 lines per sub-doc, ≥4000 total
- [ ] No forbidden terms
- [ ] Meta-Harness citation present with the nuance about "6× vs +7.7/+4.7"
- [ ] Tool loop doc explicitly says "already exists, do not rebuild"
- [ ] Chat types doc explicitly explains "must live in roko-core, not roko-agent"
- [ ] ≥15 citations

## CRITICAL REMINDERS

- DO NOT SUMMARIZE. DO NOT TRUNCATE. PRESERVE ALL CITATIONS.
- Chat types go in `roko-core`, not `roko-agent` — this is a critical architectural constraint from modelrouting/13-architectural-gaps.md §A.
- ToolLoop already exists at `roko-agent/src/tool_loop/mod.rs`. Do NOT propose rebuilding it. Implement `LlmBackend` for HTTP providers and wrap the existing loop.
- ExecAgent is legacy fallback, not a `ProviderKind`.
- Apply naming map: golem→agent; mori→Roko Orchestrator; bardo→roko.
- Make clear the #1 integration gap: SafetyLayer wired to ToolDispatcher, but ToolDispatcher never called from orchestrate.rs.
- No death framing.
- Use Write tool. Don't ask questions. Continue.
