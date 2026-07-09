# Composition Summary — PU03 Audit Refresh

Quick reference for agents working on `tmp/docs-parity/03`.

## Audit Posture

PU03 is a wiring audit, not a theory sprint.

- Prove the live composition path that actually ships.
- Document helper libraries and partial seams honestly.
- Defer VCG, MVT, distributed-context, and eval-theory work unless a wired-path bug makes them unavoidable.

## Wired Production Path

Treat this as the main contract unless code evidence says otherwise:

1. `roko-cli` dispatch builds task context and resolves tiered context in `crates/roko-cli/src/orchestrate.rs:11584`.
2. `ContextProvider::new` is instantiated at `crates/roko-cli/src/orchestrate.rs:11586`.
3. `ContextProvider::resolve` runs at `crates/roko-cli/src/orchestrate.rs:11611`.
4. Prompt text is assembled through `build_role_system_prompt` / `build_role_system_prompt_validated` in `crates/roko-cli/src/prompting.rs:50` and `:60`.
5. `RoleSystemPromptSpec` owns the typed role/task prompt contract in `crates/roko-compose/src/role_prompts.rs:155`.
6. `SystemPromptBuilder` owns layered system-prompt assembly in `crates/roko-compose/src/system_prompt_builder.rs:53`.
7. `PromptComposer` produces the final section-budgeted prompt, with `PromptBuild` at `crates/roko-compose/src/prompt.rs:828`.
8. The real context gather/rank engine is `roko-neuro::ContextAssembler` in `crates/roko-neuro/src/context.rs:221`.

## What Counts As Real In PU03

- `RoleSystemPromptSpec` is live and shared across CLI prompt entrypoints.
- `SystemPromptBuilder` is live for layered role/system assembly.
- `PromptComposer` is live for section budgeting and truncation.
- `ContextProvider` is the compose-owned interface to runtime context assembly.
- `ContextProvider` delegates into the real `roko-neuro::ContextAssembler`, not the thin re-export shim.
- Section-effectiveness hooks are real builder inputs, even if the audit does not expand them.

## Helper Paths To Describe Honestly

- `templates/common::budget_for()` is the base static budget table, while `budget::adjusted_budget_for()` adds a complexity layer. Audit the split; do not assume both are fully wired on every runtime path.
- `EnrichmentPipeline` is real library code in `crates/roko-compose/src/enrichment/pipeline.rs:30`, and `roko-cli` has an enrichment runtime seam at `crates/roko-cli/src/orchestrate.rs:1293` and `:6694`. That is plan-enrichment runtime, not the per-dispatch context path under this audit.
- HDC helpers such as `text_fingerprint` and `semantic_similarity` are present in `crates/roko-neuro/src/context.rs:17` and `:924`, but deeper dedup or distributed-context claims should be treated as future work unless the live path already calls them for the bug in question.
- `ActiveInferenceScorer` exists in `crates/roko-compose/src/scorer.rs:98`, but PU03 should treat it as a naming/contract honesty issue before treating it as a required learning-policy subsystem.

## Known Live Gaps

- `Researcher` and `Conductor` still use inline role strings in `crates/roko-compose/src/role_prompts.rs:518` and `:522`.
- `Refactorer` still reuses `TaskImplTemplate` identity at `crates/roko-compose/src/role_prompts.rs:517`.
- Cache markers are supported by the builder, but the audit should confirm actual call sites instead of assuming uniform runtime coverage.

## Working Rule

If a task cannot be justified by following the wired path from `orchestrate.rs` into `prompting.rs`, `role_prompts.rs`, `system_prompt_builder.rs`, `prompt.rs`, and `context.rs`, it is probably not PU03 work.
