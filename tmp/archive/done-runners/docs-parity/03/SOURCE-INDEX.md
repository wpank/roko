# SOURCE-INDEX — Verified Code Anchors

Current anchors for the `03` composition parity refresh.

Rechecked: 2026-04-18

Use `rg` if any file shifts again.

---

## Core Prompt Path

| File | Anchor | Why It Matters |
|------|--------|----------------|
| `crates/roko-core/src/traits.rs` | `143` | `Composer` trait |
| `crates/roko-compose/src/prompt.rs` | `828` | `PromptBuild` |
| `crates/roko-compose/src/role_prompts.rs` | `154` | `RoleSystemPromptSpec` |
| `crates/roko-compose/src/role_prompts.rs` | `280` | builder wiring from `RoleSystemPromptSpec` |
| `crates/roko-compose/src/role_prompts.rs` | `357` | `compose_with_budget()` |
| `crates/roko-compose/src/role_prompts.rs` | `391` | validated prompt build with context window |
| `crates/roko-compose/src/system_prompt_builder.rs` | `53` | `SystemPromptBuilder` |
| `crates/roko-compose/src/system_prompt_builder.rs` | `222` | `with_cache_markers()` |
| `crates/roko-compose/src/system_prompt_builder.rs` | `236` | `with_section_effectiveness(...)` |
| `crates/roko-compose/src/system_prompt_builder.rs` | `265` | `build_with_counter(...)` |
| `crates/roko-compose/src/system_prompt_builder.rs` | `336` | `build_sections()` |
| `crates/roko-cli/src/prompting.rs` | `25` | CLI `build_spec(...)` helper |
| `crates/roko-cli/src/prompting.rs` | `50` | `build_role_system_prompt(...)` |
| `crates/roko-cli/src/prompting.rs` | `60` | `build_role_system_prompt_validated(...)` |
| `crates/roko-cli/src/orchestrate.rs` | `11709` | `section_effectiveness_snapshot()` use on live path |
| `crates/roko-cli/src/orchestrate.rs` | `14263` | `build_system_prompt_with_context_validated(...)` |

---

## Templates And Budgets

| File | Anchor | Why It Matters |
|------|--------|----------------|
| `crates/roko-compose/src/templates/mod.rs` | `9` | template module list |
| `crates/roko-compose/src/templates/mod.rs` | `76` | `RolePromptTemplate` trait |
| `crates/roko-compose/src/templates/common.rs` | `17` | `PromptBudget` |
| `crates/roko-compose/src/templates/common.rs` | `44` | `budget_for()` |
| `crates/roko-compose/src/templates/common.rs` | `154` | `MCP_TOOLS_STANZA` |
| `crates/roko-compose/src/templates/implementer.rs` | `39` | `ImplementerTemplate` |
| `crates/roko-compose/src/templates/reviewer.rs` | `45` | `ReviewerTemplate` |
| `crates/roko-compose/src/templates/quick.rs` | `44` | `QuickReviewerTemplate` |
| `crates/roko-compose/src/templates/quick.rs` | `152` | `QuickFixTemplate` |
| `crates/roko-compose/src/templates/scribe.rs` | `56` | `ScribeTemplate` |
| `crates/roko-compose/src/templates/strategist.rs` | `46` | `StrategistTemplate` |
| `crates/roko-compose/src/templates/integration.rs` | `32` | `IntegrationTemplate` |
| `crates/roko-compose/src/templates/task_impl.rs` | `67` | `TaskImplTemplate` |
| `crates/roko-compose/src/role_prompts.rs` | `498` | `role_identity_for(...)` and inline fallbacks |
| `crates/roko-compose/src/budget.rs` | `23` | `Complexity` |
| `crates/roko-compose/src/budget.rs` | `66` | `adjusted_budget_for()` |

---

## Context And Enrichment

| File | Anchor | Why It Matters |
|------|--------|----------------|
| `crates/roko-compose/src/context_provider.rs` | `35` | `ContextTier` |
| `crates/roko-compose/src/context_provider.rs` | `168` | `ContextSection` |
| `crates/roko-compose/src/context_provider.rs` | `185` | `ResolvedContext` |
| `crates/roko-compose/src/context_provider.rs` | `297` | `ContextBudgets` |
| `crates/roko-compose/src/context_provider.rs` | `347` | `PlanArtifacts` |
| `crates/roko-compose/src/context_provider.rs` | `416` | `SiblingTask` |
| `crates/roko-compose/src/context_provider.rs` | `429` | `PriorTaskOutput` |
| `crates/roko-compose/src/context_provider.rs` | `442` | `ContextProvider` |
| `crates/roko-compose/src/enrichment/client.rs` | `13` | `LlmClient` |
| `crates/roko-compose/src/enrichment/pipeline.rs` | `30` | `EnrichmentPipeline` |
| `crates/roko-cli/src/orchestrate.rs` | `1293` | `EnrichmentRuntimeClient` |
| `crates/roko-cli/src/orchestrate.rs` | `6666` | enrichment step selection |
| `crates/roko-cli/src/orchestrate.rs` | `6694` | runtime `EnrichmentPipeline::new(...)` |
| `crates/roko-cli/src/orchestrate.rs` | `6761` | `handle_enriching(...)` |
| `crates/roko-neuro/src/context.rs` | `235` | `MARGINAL_VALUE_STOP_RATIO` |
| `crates/roko-neuro/src/context.rs` | `924` | semantic-similarity dedup helper |

Important correction:

- the real context engine is `crates/roko-neuro/src/context.rs`, not the `roko-compose/src/context_assembler.rs` shim.

---

## Learning Hooks

| File | Anchor | Why It Matters |
|------|--------|----------------|
| `crates/roko-learn/src/prompt_experiment.rs` | `395` | `ExperimentStore` |
| `crates/roko-learn/src/runtime_feedback.rs` | `383` | runtime loads `ExperimentStore` |
| `crates/roko-learn/src/runtime_feedback.rs` | `591` | experiment-store accessor |
| `crates/roko-learn/src/runtime_feedback.rs` | `597` | `section_effectiveness_snapshot()` |
| `crates/roko-cli/src/main.rs` | `5459` | CLI loads `ExperimentStore` |

Keep the claim small:

- prompt experiments and section-effectiveness persistence are wired,
- the full eval stack is not.

---

## Scorers

| File | Anchor | Why It Matters |
|------|--------|----------------|
| `crates/roko-compose/src/scorer.rs` | `21` | `SectionScorer` |
| `crates/roko-compose/src/scorer.rs` | `98` | `ActiveInferenceScorer` |
| `crates/roko-compose/src/scorer.rs` | `231` | local text-embedding helper |
| `crates/roko-compose/src/role_prompts.rs` | `475` | scorer selection |

Important correction:

- `ActiveInferenceScorer` is shipped code, but parity docs should describe it as a heuristic scorer unless/until formal active-inference machinery lands.
