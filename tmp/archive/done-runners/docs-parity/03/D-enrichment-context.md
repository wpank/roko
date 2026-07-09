# D — Enrichment And Context

Coverage for:

- `docs/03-composition/04-enrichment-pipeline-13-step.md`
- `docs/03-composition/08-5-stage-assembly-pipeline.md`

---

## Verdict

`rewrite`

The prior parity pass blurred together two different things:

- runtime prompt/context composition,
- enrichment pipeline execution.

They are related, but they are not the same path.

---

## Current State

### 1. Runtime prompt/context path

The live prompt-time path is:

- `ContextProvider` in `crates/roko-compose/src/context_provider.rs:442`
- `roko-neuro::ContextAssembler` in `crates/roko-neuro/src/context.rs`
- `build_system_prompt_with_context_validated(...)` in `crates/roko-cli/src/orchestrate.rs:14263-14295`
- `RoleSystemPromptSpec` / `SystemPromptBuilder` / `PromptComposer`

This is what the docs should describe as the active context path.

### 2. Enrichment pipeline path

`EnrichmentPipeline` is real code:

- `LlmClient` trait at `crates/roko-compose/src/enrichment/client.rs:13`
- `EnrichmentPipeline` at `crates/roko-compose/src/enrichment/pipeline.rs:30`
- CLI runtime client at `crates/roko-cli/src/orchestrate.rs:1293-1322`
- runtime construction at `crates/roko-cli/src/orchestrate.rs:6681-6707`

But that pipeline is used for the strategist enrichment phase, not as the default system-prompt composition path.

That distinction needs to stay explicit.

---

## What To Say In Present Tense

- Strategist enrichment is real and runtime-capable.
- Prompt-time context assembly is also real.
- These paths meet at the broader composition/orchestration layer, but they are not interchangeable.

---

## What To Stop Saying

- “The enrichment pipeline is the runtime enrichment path” without qualification.
- “The five-stage assembly pipeline is the canonical live path” if the actual runtime is now `ContextProvider` plus `ContextAssembler` plus builder/composer wiring.

---

## Narrow Gaps

- `EnrichmentPipeline` has runtime code, but parity should not assume it owns every prompt-time enrichment decision.
- HDC-aware similarity and dedup work exists in `roko-neuro/src/context.rs:924-931`, but that does not mean every doc claim about distributed or advanced context engineering is live.
- Keep the docs honest about which path is default and which path is phase-specific.

---

## Deferred

Defer from this section:

- distributed context engineering,
- multi-agent context mesh claims,
- eval-stack-backed enrichment optimization.

Follow-on code work here should be small and ownership-driven:

1. keep the strategist enrichment path clear,
2. keep the prompt-time context path clear,
3. do not merge them in docs unless the code actually merges them.
