# Composition Stack Summary — Quick Reference

For agents working on batch `03`.

## Core Split

`roko-compose` owns prompt assembly, role templates, system-prompt layering, budget helpers, enrichment helpers, and the CLI-facing context-provider surface.

`roko-neuro` owns the real context assembly engine and knowledge retrieval machinery behind the higher-level compose APIs.

`roko-cli` owns the live runtime call sites that build prompts and attach context.

## Main Batch-03 Question

**Which composition subsystems are already built, but are either dead code or not the actual runtime path?**

## What Is Already Real

- `PromptComposer`,
- `SystemPromptBuilder`,
- `RoleSystemPromptSpec`,
- tier-aware `ContextProvider`,
- `ContextAssembler` in `roko-neuro`,
- tokenizer-backed `TokenCounter`,
- `SectionEffectivenessRegistry` feedback into builder caps,
- live prompt composition inside `orchestrate.rs`.

## What Exists But Is Not Really Live

- `budget_for()` and `adjusted_budget_for()` as runtime policy,
- `EnrichmentPipeline` and `LlmClient`,
- full cache-marker coverage,
- some role-template coverage,
- HDC dedup in live context pruning,
- the name/claim behind `ActiveInferenceScorer`.

## Runtime Reality To Keep In Mind

- the live prompt path is `RoleSystemPromptSpec` / `SystemPromptBuilder` / `PromptComposer`,
- the live context path is `ContextProvider` backed by `roko-neuro::ContextAssembler`,
- the enrichment library is not automatically the runtime enrichment path,
- not every documented subsystem needs to be activated in batch `03`; one real production path is enough to prove a contract.
