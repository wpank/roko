# Prompt Assembly: Goals

## End State

A model-aware, budget-adaptive `PromptAssemblyService` where prompt content scales
to the model receiving it. Small models get surgical, focused prompts. Large models
get rich context. The system learns which sections help which roles and tunes itself
over time. All entry points converge on a single assembly path.

---

## Core Properties

### 1. Model-Aware Context Windowing

Every prompt is assembled against the target model's actual context window:
- **Surgical tier** (local models, Haiku): 4K token budget. Role identity +
  task + inline files + verification commands. No plan context, no research,
  no workspace map.
- **Focused tier** (Sonnet, GPT-4o-mini): 12K token budget. Surgical + task brief,
  dependency outputs, anti-patterns, playbook steps.
- **Full tier** (Opus, GPT-4, Gemini Pro): 24K token budget. Focused + plan brief,
  workspace map, PRD extract, cross-plan context, research memos.
- **Extended tier** (200K+ models, future): Up to 100K token budget. Full + raw
  source files, prior episode transcripts, multiple playbooks, dream
  consolidation insights.

The `ContextTier` system in `context_provider.rs` already defines
Surgical/Focused/Full with the right budgets. The goal is to wire it into the
main `SystemPromptBuilder` / `RoleSystemPromptSpec` path so that `dispatch_agent_with()`
consults the model slug before deciding how much context to inject.

**Tier selection logic:**
- Local models always get Surgical regardless of task complexity
- Task tier "mechanical" maps to Surgical
- Task tier "focused" or "integrative" maps to Focused
- Task tier "architectural" maps to Full
- The model's actual context window provides a hard ceiling

**Why tiers instead of proportional scaling:** Proportional scaling produces
mediocre prompts at every scale. A half-sized Opus prompt and a half-sized
Haiku prompt are both worse than purpose-built prompts for each tier. Tiers
allow qualitatively different prompt strategies: imperative instructions for
Surgical, collaborative context for Focused, comprehensive briefing for Full.

### 2. Progressive Context Refinement

Large document sets (PRDs, research memos, episode histories) are funneled into
focused prompts through a multi-stage pipeline:

**Stage 1: Broad retrieval.** Retrieve all potentially relevant content from
knowledge store, episode history, playbooks, code index. No budget constraint.
Uses `MultiPatchForager` to determine visitation order and stopping criteria.

**Stage 2: Relevance scoring.** Score each candidate by:
- Task-term overlap (information density via `information_density()`)
- Historical lift (`SectionInfluence.weights()` multipliers)
- Recency decay (via `SectionScorer` novelty component)
- Role-specific policy (ContextTier determines which categories are eligible)
- Active inference bias (via `PositionAttentionModel`)

**Stage 3: Budget allocation.** Knapsack over scored candidates under the
model's token budget. DensityGreedy for cold start; VCG when warm (after 10+
observations per bidder). Budget is partitioned by priority class:
- Critical sections: guaranteed allocation (up to 25% each)
- High sections: strong preference (up to 15% each)
- Normal sections: shared remaining budget
- Low sections: last to be allocated, first to be dropped

**Stage 4: Compaction.** Within each selected section, truncate or summarize
to fit the per-section cap. Strategy varies by content type:
- Conversation history: `compact_history()` with anchor preservation
- Documents: extractive summarization (sentence-level relevance)
- Code: symbol-level extraction (function signatures, struct definitions)
- Lists: priority truncation (keep first N items)

**Stage 5: Placement optimization.** Assign sections to Start/Middle/End
positions based on the U-curve attention model, placing the highest-value
sections at prompt edges where attention is highest. Uses `dynamic_placement()`
with model-specific attention curves when available.

### 3. Section Effectiveness Learning

Every dispatch records which sections were included. Every gate outcome records
success/failure. The `SectionInfluence` tracker computes causal lift per section.
Over time:
- Sections that consistently help get higher budget shares
- Sections that consistently hurt get demoted or dropped
- Section effects are tracked per-role (Implementer may benefit from different
  sections than Reviewer)
- The learning loop feeds into `LearningContextBidder` for relevance boosts
- `BudgetPredictor` converges token budgets toward actual usage per task type
- Influence data persists across runs in `.roko/learn/section-influence.json`
  and `.roko/learn/budget-predictor.json`

**Cold start behavior:** New installations use the static per-role budgets.
After 10+ task executions per feature key, `BudgetPredictor` provides
increasingly accurate budget estimates. After 20+ tasks per section per role,
`SectionInfluence` provides reliable lift measurements. The system gracefully
degrades to conservative defaults when data is insufficient.

**Guardrails on learning** (from `LearningContextBidder`):
- Minimum 20 included trials and 5 excluded trials before influence applies
- Maximum 25% of dispatch budget affected by learned controls
- Maximum 3 section priority promotions per dispatch
- Maximum 0.20 relevance boost per candidate
- 100-observation recency window for posterior calculations

### 4. Role Identity from Config

Role prompts loaded from TOML files, not `&'static str` constants. Benefits:
- Users can customize role behavior without recompiling
- A/B testing different role phrasings via the experiment system
- Organization-specific role definitions for enterprise deployments
- Role hot-reloading during long plan runs

**Format:**
```toml
# .roko/roles/implementer.toml
[role]
name = "Implementer"
identity = "You are the Implementer..."
reasoning_depth = "brief"
output_format = "code_with_explanation"

[role.tier_adjustments]
surgical = "Make the change described below. Do not explain."
focused = "Think through the approach briefly, then implement."
full = "Analyze the codebase structure, explain your approach, then implement."
```

### 5. All Callers Use One Service

Every entry point converges on `PromptAssemblyService.assemble()`:
- `roko run` -- already partially wired via `build_role_system_prompt_validated()`
- `roko chat` -- needs `PromptAssemblyService` integration with conversation
  compaction for long sessions
- `roko plan run` -- already fully wired through `dispatch_agent_with()` in
  `orchestrate.rs`, using the full 9-layer builder with effectiveness learning
- ACP runner -- replace inline format strings with `PromptAssemblyService` calls,
  using `ReviewerTemplate` for multi-role review
- Agent sidecar -- use `PromptAssemblyService` for `/message` endpoint in
  `roko-agent-server`

**Convergence benefit:** Bug fixes, prompt improvements, and learning feedback
apply uniformly across all entry points. Currently, improving the orchestrator
prompt does nothing for `roko chat` because they use completely different paths.

### 6. Zero Inline Prompt Strings

Every `format!()` prompt in `orchestrate.rs` and `roko-acp/runner.rs` is replaced
with a template call or `PromptAssemblyService` invocation. The only allowed prompt
construction is through the 9-layer builder.

**Locations to fix:**
- `orchestrate.rs` fallback task prompts (~8894, ~9347, ~9941, ~14014)
- `orchestrate.rs` gate failure retry hint (~11212)
- `orchestrate.rs` model escalation / replan prompts (~11285, ~11395)
- `orchestrate.rs` verification-failed fix prompt (~13080)
- `roko-acp/runner.rs` review variants (405-424)
- `roko-acp/runner.rs` multi-role review roles (525-541)

### 7. Budget Prediction from History

`BudgetPredictor` replaces static per-role budgets with learned values:
- EMA of actual token usage per (role x complexity x domain) triple
- 20% safety margin over EMA
- Failure inflation (1.3x on gate failure)
- Partial-match fallback for novel feature combinations
- Minimum floor of 1000 tokens (prevents degenerate budgets)
- Persistence to `.roko/learn/budget-predictor.json`

**Learning schedule:**
- First 10 tasks: use static per-role budgets
- Tasks 10-50: blend static and predicted (50/50 weighted average)
- Tasks 50+: use predicted budget with 20% safety margin
- On any gate failure: inflate the feature key's EMA by 1.3x

### 8. Cognitive Workspace Audit Trail

Every prompt assembly decision is recorded in a `CognitiveWorkspace` audit object:
- Which sections were included and at what priority
- Which sections were excluded and why (budget pressure, tier ineligibility,
  low effectiveness score)
- Knowledge entry IDs that contributed to the prompt
- Section effectiveness scores applied
- Budget allocation details (DensityGreedy vs VCG, per-section caps)
- Model choice and tier selection rationale

This trail enables:
- Debugging prompt quality issues (why did the model fail?)
- Offline analysis of prompt composition patterns
- Automated detection of prompt assembly regressions
- User-facing transparency ("here is what context your agent received")

---

## What Exists Today

| Capability | Status | Where |
|---|---|---|
| 9+ layer SystemPromptBuilder | **Live** | `system_prompt_builder.rs` |
| 11 role templates | **Live** | `templates/*.rs` |
| Token budgeting (DensityGreedy) | **Live** | `prompt.rs`, `strategy.rs` |
| VCG auction | **Live** (rarely activates) | `auction.rs` |
| Knowledge/playbook/anti-pattern injection | **Live** (plan run path only) | `prompt_assembly_service.rs`, `orchestrate.rs` |
| Section effectiveness tracking | **Live** (plan run path only) | `orchestrate.rs` |
| Demand-driven context tiers | **Built, not wired** | `context_provider.rs` |
| BudgetPredictor | **Built, not wired** | `budget_predictor.rs` |
| SectionInfluence | **Built, not wired** | `budget_predictor.rs` |
| Position attention model | **Built, default only** | `attention.rs` |
| Multi-patch foraging | **Built, not wired** | `foraging.rs` |
| Conversation compaction | **Built, not wired** | `compaction.rs` |
| ContextBidderRegistry | **Built, partially wired** | `context_provider.rs` |
| CognitiveWorkspace audit | **Live** | `cognitive_workspace.rs` |
| LearningContextBidder | **Built, partially wired** | `context_provider.rs` |
| ContextAssembler | **Re-export** | `context_assembler.rs` -> roko-neuro |

---

## From v2 UX Showcase (9 Scenarios)

- **Knowledge injection labeling** (all): KnowledgeCard footer shows "injected
  via SystemPromptBuilder L7 of 9" -- users can see which builder layer injected
  neuro store content.
- **Mode-specific system prompts** (architect, follow, debug): Architect = "senior
  reviewer, structured output (Strong/Concerns/Blocker)". Research = "read-only,
  trace and explain". Teach = "step-through reasoning, explain every decision".
- **Embedded resource injection** (architect): "@rfc/0042-rate-limit.md (4.2 kB)"
  as context pill -- external docs injected into architect prompt context.
- **Role-specific output formats** (pipeline, pair): STR outputs strategy brief.
  REV outputs structured review. ARC outputs Strong Points / Concerns / Blockers.
  AUD outputs compliance findings.
- **Cross-agent context** (pair, tournament): Reviewer receives implementer's
  code + diffs as context. Synthesis agent receives all N approach diffs.
- **Playbook-informed prompts** (incident, pipeline): P1 triage playbook steps
  injected as context. RS256 migration playbook injected.
- **Episode-informed prompts** (incident, tournament): Past similar episodes
  injected: "Last 5xx spike: nil deref after refactor."

### Data Feeds Required

- `InjectionLayer` -- layer_number, layer_name, content_source, token_count
- `ModeTemplate` -- mode -> system_prompt_template, output_format, restrictions
- `EmbeddedResource` -- file_path, size, type (rfc/spec/doc), injected_as_context
- `RoleOutputFormat` -- role -> expected_output_structure (architect -> Strong/Concerns/Blockers)

---

## Gap Summary

### Critical (blocks model-aware assembly)

1. **ContextTier not consulted by SystemPromptBuilder.** The tier system defines
   4K/12K/24K budgets per model class, but the main builder path uses static
   per-role budgets that can total 117K characters. Need to wire
   `ContextTier::from_task_and_model()` into `dispatch_agent_with()` and use
   the tier budget as the outer envelope.

2. **BudgetPredictor not wired.** The predictor is fully built and tested but
   never called. Wire `predictor.predict()` into `PromptAssemblyService` to
   replace the static `token_budget` field.

3. **SectionInfluence not wired.** Section lift data is computed but not fed
   back into budget allocation. Wire `influence.weights()` into the builder's
   per-section caps.

### Important (improves quality)

4. **roko chat bypasses builder entirely.** `dispatch_direct.rs` sends bare
   prompts. Need to route through `PromptAssemblyService`.

5. **ACP inline prompts.** `run_multi_role_review()` duplicates reviewer
   template functionality. Replace with `ReviewerTemplate` calls.

6. **Per-model attention curves empty.** `ModelAttentionCurves` has no fitted
   data. Need to populate curves for Claude Opus, Sonnet, Haiku and
   GPT-4/4o-mini from placement experiments.

7. **MultiPatchForager not instantiated.** Foraging optimization is tested but
   the dispatch path uses direct queries. Wire the forager into context retrieval.

### Nice to Have (future improvement)

8. **Role identity from TOML config.** Currently `&'static str`.
9. **Conversation compaction in roko chat.** `compact_history()` is ready.
10. **VCG payment utility.** Payments are diagnostic-only. Consider removing or
    clearly marking as non-functional.
11. **Mode-specific prompt templates** (architect, research, debug modes).
12. **External resource embedding** (docs as prompt context).
13. **Cross-agent context injection** (findings from parallel agents).

---

## Measurement Criteria

### Prompt Quality Metrics

- **Gate pass rate by tier:** Surgical tier should achieve >= 70% first-attempt
  gate pass rate for mechanical tasks. Focused >= 80%. Full >= 85%.
- **Token efficiency:** Average system prompt size / model context window should
  be <= 15% for Focused and Full tiers, <= 30% for Surgical tier.
- **Learning convergence:** After 50 tasks, `BudgetPredictor` estimates should
  be within 30% of actual usage for 80%+ of task types.
- **Section influence significance:** After 100 tasks, `SectionInfluence` should
  identify at least 2 sections with statistically significant positive lift
  and at least 1 with negative lift.

### Entry Point Coverage

- **Builder usage rate:** 100% of dispatches go through `PromptAssemblyService`
  or `SystemPromptBuilder`. Zero dispatches with empty system prompts.
- **Template usage rate:** 100% of role-specific prompts use role templates.
  Zero inline `format!()` role descriptions.

### Performance

- **Assembly latency:** `PromptAssemblyService.assemble()` completes in < 50ms
  for Surgical tier, < 200ms for Full tier (excluding knowledge store queries).
- **Memory:** Per-dispatch memory allocation for prompt assembly < 1MB.

---

## Sources

- `crates/roko-compose/src/system_prompt_builder.rs` -- 10-layer structure, build methods
- `crates/roko-compose/src/prompt_assembly_service.rs` -- PromptAssemblyService, full pipeline
- `crates/roko-compose/src/context_provider.rs` -- ContextTier, is_local_model(), budgets, LearningContextBidder
- `crates/roko-compose/src/budget_predictor.rs` -- BudgetPredictor, SectionInfluence
- `crates/roko-compose/src/budget.rs` -- Complexity enum, adjusted_budget_for()
- `crates/roko-compose/src/attention.rs` -- PositionAttentionModel, ModelAttentionCurves
- `crates/roko-compose/src/foraging.rs` -- MultiPatchForager, should_stop_searching()
- `crates/roko-compose/src/compaction.rs` -- compact_history(), CompactionPolicy
- `crates/roko-compose/src/cognitive_workspace.rs` -- CognitiveWorkspace audit trail
- `crates/roko-compose/src/strategy.rs` -- DEFAULT_VCG_WARMUP_OBSERVATIONS = 10
- `crates/roko-compose/src/templates/common.rs` -- PromptBudget, budget_for()
- `crates/roko-compose/src/templates/mod.rs` -- 11 Template structs
- `crates/roko-cli/src/orchestrate.rs` -- dispatch_agent_with(), live path
- `crates/roko-cli/src/dispatch_direct.rs` -- no system prompt
- `crates/roko-acp/src/runner.rs` -- inline role prompts
