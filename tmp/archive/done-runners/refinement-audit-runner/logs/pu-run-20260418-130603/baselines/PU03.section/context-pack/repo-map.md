# Repo Map — Shared Composition Context

Quick reference for agents working on `03` composition parity.

## Workspace Root

`/Users/will/dev/nunchi/roko/roko/`

## High-Value Paths

| What | Path | Why It Matters In Batch 03 |
|------|------|----------------------------|
| Core composer | `crates/roko-compose/src/prompt.rs` | prompt assembly, ordering, prompt metadata |
| System prompt builder | `crates/roko-compose/src/system_prompt_builder.rs` | layered system-prompt construction |
| Role prompt glue | `crates/roko-compose/src/role_prompts.rs` | production role prompt path |
| Role templates | `crates/roko-compose/src/templates/` | static budgets and role coverage |
| Budget helpers | `crates/roko-compose/src/budget.rs` | dead-code budget policy seam |
| Token counter | `crates/roko-compose/src/token_counter.rs` | real tokenizer surface |
| Context provider | `crates/roko-compose/src/context_provider.rs` | live compose-owned context interface |
| Enrichment library | `crates/roko-compose/src/enrichment/` | dormant runtime activation target |
| Scorers | `crates/roko-compose/src/scorer.rs` | section scoring and misleading active-inference surface |
| Real context engine | `crates/roko-neuro/src/context.rs` | live assembly and HDC seam |
| Learning feedback | `crates/roko-learn/src/section_effect.rs`, `runtime_feedback.rs` | section-effectiveness runtime wiring |
| Runtime callsites | `crates/roko-cli/src/orchestrate.rs`, `prompting.rs`, `run.rs` | live prompt/context composition path |
| Composition docs | `docs/03-composition/` | source material being checked |
| Parity batch | `tmp/docs-parity/03/` | execution contract and findings |

## Important Corrections

Use these instead of older stale anchors:

- `ContextProvider` starts at `crates/roko-compose/src/context_provider.rs:442`, not near the top of the file.
- `ContextTier` is at `context_provider.rs:35`; `ContextBudgets` is at `:297`.
- the live context engine is `crates/roko-neuro/src/context.rs`, not the 4-line `context_assembler.rs` shim.
- `PromptBuild` is at `crates/roko-compose/src/prompt.rs:828`.
- `EnrichmentPipeline` is at `crates/roko-compose/src/enrichment/pipeline.rs:29`.

## Search Priorities

Before editing, search these first:

```bash
rg -n "budget_for\\(|adjusted_budget_for\\(|with_hard_cap\\(" crates/roko-compose crates/roko-cli
rg -n "EnrichmentPipeline::new|impl .*LlmClient" crates/roko-compose crates/roko-cli
rg -n "ContextProvider::new|ContextAssembler|text_fingerprint|semantic_similarity" crates/roko-compose crates/roko-neuro crates/roko-cli
rg -n "Researcher|Conductor|Refactorer|tool_allowlist_instructions" crates/roko-compose
rg -n "<!-- cache|MCP_TOOLS_STANZA|with_cache_markers" crates/roko-compose
rg -n "ActiveInferenceScorer|SectionScorer|PromptComposer::new" crates/roko-compose crates/roko-cli
```

## Build Commands

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --no-deps -- -D warnings
```

## Practical Rules

1. Make the live path stronger before reviving dormant helper layers.
2. Do not leave two sources of truth for prompt budgets.
3. If `orchestrate.rs` starts ballooning, prove one production path and stop.
4. If a task really belongs to learning or eval, record the handoff and stop.
