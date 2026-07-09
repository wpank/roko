# Repo Map — PU03 Wired Composition Paths

Quick reference for agents working on the `03` audit refresh.

## Workspace Root

`/Users/will/dev/nunchi/roko/roko/`

## High-Value Paths

| What | Path | Why It Matters In PU03 |
|------|------|-------------------------|
| CLI prompt helpers | `crates/roko-cli/src/prompting.rs` | shared prompt-build entrypoints used by live dispatch code |
| Runtime dispatch path | `crates/roko-cli/src/orchestrate.rs` | where context resolution and prompt assembly actually meet |
| Run-mode prompt path | `crates/roko-cli/src/run.rs` | simpler CLI path that still uses the same role prompt builder |
| Role prompt contract | `crates/roko-compose/src/role_prompts.rs` | live `RoleSystemPromptSpec` path and role identity gaps |
| System prompt builder | `crates/roko-compose/src/system_prompt_builder.rs` | layered system-prompt construction and cache-marker support |
| Prompt composer | `crates/roko-compose/src/prompt.rs` | final section budgeting, truncation, and `PromptBuild` metadata |
| Context provider | `crates/roko-compose/src/context_provider.rs` | compose-owned runtime context interface and tier policy |
| Real context engine | `crates/roko-neuro/src/context.rs` | actual gather/rank logic behind `ContextProvider` |
| Budget helpers | `crates/roko-compose/src/templates/common.rs`, `crates/roko-compose/src/budget.rs` | base-vs-adjusted budget split that docs must describe accurately |
| Enrichment runtime seam | `crates/roko-compose/src/enrichment/`, `crates/roko-cli/src/orchestrate.rs` | real plan-enrichment code, but not the default dispatch context path |
| Scorers | `crates/roko-compose/src/scorer.rs` | place to audit naming and contract honesty |
| Composition docs | `docs/03-composition/` | source material being checked |
| Parity batch | `tmp/docs-parity/03/` | execution contract, findings, and owned refresh docs |

## Important Corrections

Use these anchors instead of older stale references:

- `ContextTier` is at `crates/roko-compose/src/context_provider.rs:35`.
- `ContextBudgets` is at `crates/roko-compose/src/context_provider.rs:297`.
- `ContextProvider` starts at `crates/roko-compose/src/context_provider.rs:442`.
- `PromptBuild` is at `crates/roko-compose/src/prompt.rs:828`.
- `RoleSystemPromptSpec` starts at `crates/roko-compose/src/role_prompts.rs:155`.
- `SystemPromptBuilder` starts at `crates/roko-compose/src/system_prompt_builder.rs:53`.
- `EnrichmentPipeline` starts at `crates/roko-compose/src/enrichment/pipeline.rs:30`.
- `ActiveInferenceScorer` starts at `crates/roko-compose/src/scorer.rs:98`.
- `ContextAssembler` starts at `crates/roko-neuro/src/context.rs:221`.
- `build_role_system_prompt` is at `crates/roko-cli/src/prompting.rs:50`.
- `build_role_system_prompt_validated` is at `crates/roko-cli/src/prompting.rs:60`.
- `ContextProvider::new(...)` is wired in `crates/roko-cli/src/orchestrate.rs:11586`.
- `context_provider.resolve(...)` is called in `crates/roko-cli/src/orchestrate.rs:11611`.
- The plan-enrichment runtime client starts at `crates/roko-cli/src/orchestrate.rs:1293`, and `EnrichmentPipeline::new(...)` is called at `:6694`.

## Search Priorities

Search these first before proposing edits:

```bash
rg -n "build_role_system_prompt|build_role_system_prompt_validated|RoleSystemPromptSpec" crates/roko-cli crates/roko-compose
rg -n "ContextProvider::new|resolve\\(|ContextAssembler|semantic_similarity|text_fingerprint" crates/roko-cli crates/roko-compose crates/roko-neuro
rg -n "budget_for\\(|adjusted_budget_for\\(|Complexity" crates/roko-compose crates/roko-cli
rg -n "Researcher|Conductor|Refactorer|tool_allowlist_instructions" crates/roko-compose/src/role_prompts.rs
rg -n "with_cache_markers|cache:system|cache:session|MCP_TOOLS_STANZA" crates/roko-compose/src
rg -n "EnrichmentPipeline::new|run_steps|EnrichmentRuntimeClient" crates/roko-cli/src/orchestrate.rs crates/roko-compose/src/enrichment
rg -n "ActiveInferenceScorer|SectionScorer" crates/roko-compose/src/scorer.rs crates/roko-compose/src/role_prompts.rs crates/roko-cli/src/orchestrate.rs
```

## Batch-Friendly Verification Defaults

Do not default to workspace-wide build/test loops for PU03. Prefer the smallest command that proves the live path:

```bash
cargo test -p roko-compose
cargo test -p roko-cli
cargo test -p roko-neuro
```

Add `rg` confirmation for the exact call sites or symbols you audited.

## Practical Rules

1. Follow the wired dispatch path before reading helper libraries.
2. Do not describe plan enrichment as if it were the default per-dispatch context path.
3. Treat HDC, distributed-context, and eval-harness work as handoff material unless a small live-path fix demands otherwise.
4. If a task needs more than targeted package tests plus a few `rg` checks, re-scope it.
