# Mori Prompt System

All prompts defined in `apps/mori/src/orchestrator/prompts.rs` (~5500 lines).

## Prompt Budget System

Per-role character budgets control section sizing:

```rust
pub struct PromptBudget {
    pub plan: usize,          // Implementer: 50K, others: 50K
    pub workspace_map: usize, // Implementer: 20K, Reviewer: 6K
    pub prd2: usize,          // Scribe: 16K (heavy), others: 6-12K
    pub context: usize,       // 2-4K
    pub brief: usize,         // 4-8K
    pub reviews: usize,       // 2-3K
    pub instructions: usize,  // 4K
    pub file_context: usize,  // 0-8K
    pub skills: usize,        // 4-8K
}
```

Sections are priority-ranked (5=always include, 1=drop first) and sorted by cache layer for maximum prompt cache hits.

## Cache-Aligned Shared Context

All agents within a plan get an identical byte-prefix:

```rust
pub struct SharedPlanContext {
    pub system_prefix: String,   // AGENTS.md
    pub prd2_extract: String,
    pub plan_content: String,
    pub workspace_map: String,
    pub cross_plan_ctx: String,  // CONTEXT.md
    pub brief: String,
}
```

**Fixed ordering**: system -> plan -> prd2 -> workspace -> cross-plan. Comment: "This ordering is load-bearing for prompt caching -- do not rearrange."

Cache layer markers (`<!-- mori:layer:2 -->`) are inserted for the inference gateway.

## AGENTS.md (Global Agent Instructions)

Injected into every agent session. Key sections:

- **Repository Layout**: Explains all directories
- **Universal Rules**: No git ops (except MergeResolver), no prd2 mods, read workspace-map.md first
- **MCP Tools**: `search_code`, `get_symbol_context`, `get_file_ast`, `find_similar_patterns`
- **Implementer Protocol**:
  - Phase 0 (Orient): Read 11 files in order before writing code
  - Phase 1 (Implement): Per-unit loop: read sources -> read existing -> write -> test -> compile gate -> test gate -> docs -> checkpoint
  - Phase 2 (Completion): Append to CONTEXT.md, write completion report
- **Authority Chain**: Quick Reference > TOML acceptance criteria > Plan prose > PRD > Brief

Role-filtered using `<!-- role: role1,role2 -->` markers:
- Sections tagged "all" always included
- Other sections filtered by agent's role label
- Reduces ~6K tokens to ~1.5-3K per agent

## System Prompt (Common)

Every Claude agent gets via `--append-system-prompt`:

```
You are running inside Mori, an autonomous Rust code agent for the Bardo project ...

## Coding Standards
- Create all necessary files with proper mod declarations in lib.rs/main.rs
- Add `use` imports for all types before use. Check existing code patterns with search_code MCP tool.
- Add doc comments on all `pub` items. Never use `unwrap()` in library crates...

## Rules
- NEVER run `git checkout`, `git switch`, or `git branch -m`. You are in a plan worktree.
- NEVER add dependencies with { workspace = true } unless they exist in root Cargo.toml.
- Read the task description carefully. Implement ALL acceptance criteria.

{role_specific}
{tool_specific}
{artifact_specific}
Start from {preferred_context_entry} and widen only when that pack leaves a concrete ambiguity.
```

## Per-Role Prompts

### Strategist

```
You are the Strategist. Your job is to analyze the plan and produce a brief + structured task checklist.
```

**Receives**: workspace map, plan, cross-plan context, PRD2 extract
**Writes**: `brief.md` and `tasks.toml`
**On iteration 2+**: receives compressed feedback from prior reviews

### Implementer

```
Implement plan .mori/plans/{base}/plan.md ...
```

**Receives**:
- Plan content + PRD2
- "What Reviewers Will Check" section (Architect criteria + Auditor criteria)
- Self-validation instructions (cargo check, cargo test)
- Reconciliation guidance for working against a live repo
- Verify chain script (if exists)

### Task Implementer (Per-Task Version)

Per-task scoped version. Only receives:
- Assigned task's files and acceptance criteria
- Enhanced sections (types to define, formulas, imports, invariants)
- Sibling task awareness (parallel group members)
- Learning pack, research pack

**Strict scope**: "Implement ONLY this task's scope -- do not touch files outside your assigned list."

### Architect (Code Quality Reviewer)

```
You are the Architect. Review the implementation for code quality.
```

**Receives**: plan, brief, workspace map, PRD2, prior review (if iteration > 1), gate output
**Reviews**: review-tasks.toml and verify-tasks.toml
**Produces**: Structured TOML verdict block
**Issue format**: `[B-N]` (blocking bugs) with fix_hint

### Auditor (Spec Compliance Reviewer)

Similar to Architect but focused on spec compliance.
**Issue format**: `[S-N]` (spec violations)

### Quick Reviewer (Single-Pass)

```
You are the Quick Reviewer. Do a focused single-pass review of this implementation.
```

Replaces full Architect+Auditor+Scribe panel for Standard plans.
**Checks only**: correctness, API alignment, compilation, blocking omissions
**Must keep review under 500 words**
**Output**: Structured TOML verdict

### Scribe (Documentation Writer)

```
You are the Scribe. Write reference documentation for the implementation.
```

**Receives**: scribe-tasks.toml, skill injection (humanizer)
**7 required sections** in documentation
**Mandatory**: Mermaid diagrams, PRD2 citations, academic references
**Pre-submission checklist** included in prompt

### Critic (Documentation Reviewer)

```
You are the Critic. Review the Scribe's documentation for quality and spec fidelity.
```

**Checks**: completeness, accuracy, PRD2 fidelity, depth, cross-references, narrative, visual docs, voice/style

### Refactorer

Post-commit cleanup agent. Receives implementation diff and clippy output. Runs every N plans.

## Structured Verdict Format

Reviewers produce structured TOML output:

```toml
[verdict]
decision = "APPROVE"  # or "REVISE"

[[blocking]]
id = "B-1"
category = "correctness"
severity = "blocking"
description = "Missing null check on user input"
fix_hint = "Add `if input.is_none() { return Err(...) }` at line 42"
files = ["src/handler.rs"]

[[spec_violation]]
id = "S-1"
category = "spec_fidelity"
severity = "blocking"
description = "PRD requires pagination but endpoint returns all results"
fix_hint = "Add `limit` and `offset` query parameters"
```
