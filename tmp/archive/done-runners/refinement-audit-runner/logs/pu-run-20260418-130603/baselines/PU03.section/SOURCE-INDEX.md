# SOURCE-INDEX — Verified Code Anchors For 03-Composition Parity

Code anchors used by batch `03`. These were re-checked against the current codebase on 2026-04-16.

Prefer `rg` over trusting any exact line number if the file has changed since then.

---

## Important Corrections

The previous source index was directionally useful but had several stale anchors. Use these corrections:

- `PromptBuild` is at `crates/roko-compose/src/prompt.rs:828`.
- `ContextTier` is at `crates/roko-compose/src/context_provider.rs:35`.
- `ContextBudgets` is at `context_provider.rs:297`.
- `ContextProvider` is at `context_provider.rs:442`.
- the real context engine is `crates/roko-neuro/src/context.rs:221`, not the 4-line `context_assembler.rs` shim.
- `EnrichmentPipeline` is at `crates/roko-compose/src/enrichment/pipeline.rs:29`.

---

## `crates/roko-core/src/`

| File | What | Section |
|------|------|---------|
| `traits.rs:143` | `Composer` trait | A.01 |

---

## `crates/roko-compose/src/`

### Core Composer

| File | What | Section |
|------|------|---------|
| `prompt.rs:27` | `SectionPriority` | A.03 |
| `prompt.rs:45` | `CacheLayer` | A.04, B.09 |
| `prompt.rs:64` | `Placement` | A.05, A.09 |
| `prompt.rs:77` | `AttentionBidder` | A.06, F.06 |
| `prompt.rs:99` | `PromptSection` | A.02 |
| `prompt.rs:828` | `PromptBuild` | A.10 |
| `prompt.rs` | `PromptComposer` implementation | A.07, A.09, E.06 |

### System Prompt Builder + Role Path

| File | What | Section |
|------|------|---------|
| `system_prompt_builder.rs:52` | `SystemPromptBuilder` | B.01 |
| `role_prompts.rs:42` | `TaskContext` | C.02 |
| `role_prompts.rs:154` | `RoleSystemPromptSpec` | C.01 |
| `role_prompts.rs:473` | `tool_allowlist_instructions()` | C.07 |
| `role_prompts.rs:485` | `role_identity_for()` | C.06, C.07 |

### Templates + Static Budgets

| File | What | Section |
|------|------|---------|
| `templates/mod.rs:76` | `RolePromptTemplate` trait | C.03 |
| `templates/common.rs:17` | `PromptBudget` | C.04, E.01 |
| `templates/common.rs:44` | `budget_for()` | C.05, E.01 |
| `templates/common.rs:154` | `MCP_TOOLS_STANZA` | C.08.6, P7 |
| `templates/assembly.rs` | `PromptAssembler` | C.08.6 |
| `templates/implementer.rs` | Implementer template hard caps | C.05 |
| `templates/reviewer.rs` | Reviewer / Combined reviewer variants | C.06 |
| `templates/quick.rs` | Quick reviewer + quick fix templates | C.06 |
| `templates/scribe.rs` | Scribe variants | C.06 |
| `templates/strategist.rs` | Strategist template | C.06 |
| `templates/task_impl.rs` | task-implementation / refactorer-adjacent path | C.06 |
| `templates/integration.rs` | integration template | C.06 |

### Budget Management + Token Counting

| File | What | Section |
|------|------|---------|
| `budget.rs:23` | `Complexity` | C.08, E.02 |
| `budget.rs:38` | `AdjustedBudget` | E.04 |
| `budget.rs:66` | `adjusted_budget_for()` | C.08, E.02 |
| `token_counter.rs:9` | `TokenCounter` | E.05 |
| `compaction.rs` | history compaction | F.08 |

### Context + Enrichment

| File | What | Section |
|------|------|---------|
| `context_assembler.rs` | 4-line shim re-export | D.08 |
| `context_provider.rs:35` | `ContextTier` | D.10, E.03 |
| `context_provider.rs:168` | `ContextSection` | D.11 |
| `context_provider.rs:185` | `ResolvedContext` | D.11 |
| `context_provider.rs:297` | `ContextBudgets` | D.10, E.03 |
| `context_provider.rs:347` | `PlanArtifacts` | D.11 |
| `context_provider.rs:416` | `SiblingTask` | D.11 |
| `context_provider.rs:429` | `PriorTaskOutput` | D.11 |
| `context_provider.rs:442` | `ContextProvider` | D.09 |
| `enrichment/config.rs:18` | `EnrichmentConfig` | D.05 |
| `enrichment/client.rs:13` | `LlmClient` trait | D.03 |
| `enrichment/pipeline.rs:29` | `EnrichmentPipeline<C>` | D.01 |
| `enrichment/step.rs:30` | `EnrichStep` | D.02 |
| `enrichment/direct_client.rs:177` | `DirectClient` | D.04 |
| `enrichment/batch_client.rs:153` | `BatchClient` | D.04 |

### Scorers

| File | What | Section |
|------|------|---------|
| `scorer.rs:21` | `SectionScorer` | F.01 |
| `scorer.rs:98` | `ActiveInferenceScorer` | F.02, P8 |

---

## `crates/roko-neuro/src/`

| File | What | Section |
|------|------|---------|
| `context.rs:148` | `PadState` | B.08, F.03 |
| `context.rs:221` | `ContextAssembler` | D.08, D.12 |
| `context.rs:235` | `MARGINAL_VALUE_STOP_RATIO` | F.07 |
| `context.rs:924` | `semantic_similarity()` using `text_fingerprint` | D.12 |
| `knowledge_store.rs:255` | `KnowledgeStore::query()` | F.05 |

Important note:

- the real context path is here, not in `roko-compose/src/context_assembler.rs`.

---

## `crates/roko-learn/src/`

| File | What | Section |
|------|------|---------|
| `section_effect.rs:13` | `DEFAULT_SECTION_EFFECTS_PATH` | B.10, E.06 |
| `section_effect.rs:114` | `SectionEffectivenessRegistry` | B.10, E.06 |
| `prompt_experiment.rs:349` | `ExperimentStore` | E.06 |
| `model_experiment.rs:207` | `ModelExperimentStore` | E.06 |
| `runtime_feedback.rs:597` | `section_effectiveness_snapshot()` | B.10, E.06 |

---

## `crates/roko-cli/src/`

| File | What | Section |
|------|------|---------|
| `prompting.rs:11` | `PromptBuildOptions` | A.10 |
| `run.rs:110` | `PromptComposer::new()` single-run path | A.07, E.02 |
| `run.rs:114` | flat `Budget::tokens(...)` path | E.02 |
| `orchestrate.rs:5717` | `handle_enriching()` | D.01, P5 |
| `orchestrate.rs:7191` | `NeuroStore::query()` | F.05 |
| `orchestrate.rs:10132` | `ContextProvider::new(...)` | D.09, E.03 |
| `orchestrate.rs:10255` | `section_effectiveness_snapshot()` call | B.10, E.06 |
| `orchestrate.rs:10261` | `build_system_prompt_with_context_validated(...)` | B.01, B.10 |
| `orchestrate.rs:10443` | `PromptComposer::new()` in main runtime path | A.07 |
| `orchestrate.rs:10487` | flat `Budget::tokens(self.config.prompt.token_budget)` | E.02 |
| `orchestrate.rs:12717` | `build_system_prompt_with_context_validated()` helper | B.01 |
| `orchestrate.rs:14000` | direct `role_identity_for(role)` use | C.07 |

Important negative findings:

- `adjusted_budget_for()` has no non-test runtime callers in `crates/roko-cli` or the production compose path.
- `EnrichmentPipeline::new(...)` has no non-test runtime callers in `crates/roko-cli`.

---

## Missing / Absent

These features still have no real code surface:

| Absent Feature | Why It Matters | Section |
|----------------|----------------|---------|
| `CompressionBudgetController` / `LayerCompressionConfig` / `CompressionMethod` | doc 02 §9 remains design-only | B.11 |
| `min_tokens` floor | doc 05's min-useful-context rule is unimplemented | E.06 |
| `BudgetPredictor` / `BudgetOutcome` / `SectionAllocationRecord` | dynamic budget prediction remains design-only | E.06 |
| RAGAS / CLEAR / CIV / Meta-Harness code | eval stack not started | F.10-F.12 |
| Level-3 distributed context machinery | distributed context remains conceptual | F.09 |
