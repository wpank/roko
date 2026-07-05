# P12 — Demand-Driven Context Provider

> **Created**: 2026-04-08
> **Status**: In Progress
> **Priority**: P0 — required for roko to execute tasks well on cheaper models

## Problem

Tasks currently receive context in two ways:
1. Static `read_files`/`symbols`/`anti_patterns` from tasks.toml (surgical but manual)
2. The 6-layer system prompt (role identity + conventions + tools + domain + task + anti-patterns)

Neither adapts to the model tier. A Haiku task gets the same context as an Opus task.
Mori's approach (13-artifact enrichment + fixed per-role budgets) generated too much
irrelevant context. We need something better.

## Design: Three-Tier Context Provider

### Tier 1: Surgical (Haiku / Ollama / Gemma — mechanical tasks)
- Inlined file snippets (from `read_files` with line ranges)
- Symbol signatures (resolved via roko-index, not full files)
- Anti-patterns
- Verification commands
- **Budget: ~4K tokens**
- No enrichment artifacts. No plan context. The prompt IS the spec.

### Tier 2: Focused (Sonnet — focused/integrative tasks)
- Everything in Tier 1
- Task-scoped brief (not full plan brief — 3 questions: What/Why/How)
- Dependency graph excerpt (only this task's deps)
- Prior task outputs (if task has `depends_on`)
- **Budget: ~12K tokens**
- Enrichment: only the *slice* relevant to this task

### Tier 3: Full (Opus — architectural tasks)
- Everything in Tier 2
- Plan-level brief
- Cross-plan context
- Research memo (if exists in plan dir)
- Invariants / rubric
- **Budget: ~24K tokens**
- Full enrichment, but still scoped to relevance

### Ollama/Gemma Consideration
- Local models (gemma4:12b, llama3.1:8b) have smaller context windows (8K-32K)
- They don't support tool use reliably
- They get Tier 1 context ONLY — everything inline, no MCP tools
- The `OllamaAgent` already exists and works — we just need to ensure context fits
- Config: `tier_models.mechanical = "ollama/gemma4:12b"` in roko.toml

## Architecture

### New module: `roko-compose/src/context_provider.rs`

```rust
pub struct ContextProvider {
    workdir: PathBuf,
    symbol_index: Option<SymbolIndex>,  // from roko-index
}

pub struct ResolvedContext {
    pub sections: Vec<ContextSection>,
    pub total_tokens_estimate: usize,
}

pub struct ContextSection {
    pub name: String,           // "file:src/main.rs:40-80", "symbol:TaskDef", "brief"
    pub content: String,
    pub priority: u8,           // 1-5, dropped in order when over budget
    pub cache_layer: u8,        // for prefix-cache alignment
    pub source: ContextSource,  // InlineFile, SymbolSignature, TaskBrief, PlanBrief, etc.
}

pub enum ContextSource {
    InlineFile,
    SymbolSignature,
    TaskBrief,       // generated per-task
    PlanBrief,       // from plan dir brief.md
    PriorTaskOutput, // from completed dependency task
    ResearchMemo,    // from plan dir research.md
    Invariants,      // from plan dir rubric.md
    AntiPattern,
    Verification,
}
```

### New module: `roko-compose/src/task_brief.rs`

Generates a concise per-task brief answering:
1. **What**: Task title + files + acceptance (from tasks.toml)
2. **Why**: Extracted from plan.md — the sentences that reference this task's files
3. **How**: Existing patterns in codebase to follow + sibling task context

For mechanical tasks: skip entirely.
For focused tasks: generate with Haiku (mostly extraction).
For architectural tasks: use Sonnet to add cross-cutting reasoning.

### New module: `roko-compose/src/symbol_resolver.rs`

Given a list of symbol names, resolves to their signatures by:
1. First: check roko-index if available (fast, pre-parsed)
2. Fallback: grep for `pub (fn|struct|enum|trait|type) {name}` and extract signature

### Integration point: `orchestrate.rs::dispatch_agent`

Between task parsing and prompt composition, call:
```rust
let context = context_provider.resolve(
    &task_def,
    model_tier,   // derived from task.tier
    &plan_dir,
    &completed_tasks,
)?;
```

Then inject `context.sections` into the `PromptComposer` alongside the existing
role and task sections.

## Files to create/modify

| File | Action | What |
|------|--------|------|
| `crates/roko-compose/src/context_provider.rs` | **Create** | Core ContextProvider, ResolvedContext, tiered resolution |
| `crates/roko-compose/src/task_brief.rs` | **Create** | Per-task brief generator |
| `crates/roko-compose/src/symbol_resolver.rs` | **Create** | Symbol → signature resolver |
| `crates/roko-compose/src/lib.rs` | **Modify** | Export new modules |
| `crates/roko-cli/src/orchestrate.rs` | **Modify** | Wire ContextProvider into dispatch_agent |
| `crates/roko-cli/src/task_parser.rs` | **Modify** | Add `context_tier` field derivation |
| `crates/roko-cli/src/config.rs` | **Modify** | Add context provider config section |
| `crates/roko-compose/src/enrichment/step.rs` | **Modify** | Add Ollama backend to LlmBackend |

## Implementation order

1. ContextSection + ResolvedContext types
2. SymbolResolver (grep-based, no roko-index dependency)
3. TaskBrief generator
4. ContextProvider with tier-based assembly
5. Wire into orchestrate.rs dispatch_agent
6. Add config knobs (context budgets per tier)
7. Add Ollama/Gemma backend to enrichment LlmBackend
8. Tests

## Verification

```bash
cargo check -p roko-compose
cargo check -p roko-cli
cargo test -p roko-compose
cargo test -p roko-cli
cargo clippy --workspace --no-deps -- -D warnings
```
