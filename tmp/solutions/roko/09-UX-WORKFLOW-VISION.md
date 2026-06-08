# UX Workflow Vision: Aggregation, Funnel, Execute

How roko CLI should support the full research-to-ship workflow.

---

## Table of Contents

1. [The Problem](#1-the-problem)
2. [The Workflow Model](#2-the-workflow-model)
3. [Progressive Context Refinement](#3-progressive-context-refinement)
4. [Smart Task Decomposition](#4-smart-task-decomposition)
5. [Interactive Refinement](#5-interactive-refinement)
6. [ACP/MCP Integration](#6-acpmcp-integration)
7. [Implementation Plan](#7-implementation-plan)

---

## 1. The Problem

### 1.1 The Real User Workflow

The actual workflow for building non-trivial features is:

```
Aggregate → Funnel → Architecture → Decompose → Scope → Execute → Verify → Ship
```

**Aggregate**: Collect a messy pile of docs, research, prior art, reference implementations,
API specs, screenshots, conversations, and half-formed ideas into a working folder.

**Funnel**: Do multiple passes over that pile -- compress, synthesize, extract the signal.
Each pass reduces volume but increases density. A 50-file research folder becomes a
10-page design doc becomes a 2-page implementation spec.

**Architecture**: Turn the spec into decisions: what changes where, what depends on what,
what the boundaries are, what the acceptance criteria look like.

**Decompose**: Break the architecture into tasks. Each task should be small enough for a
single agent to execute, large enough to be meaningful.

**Scope**: For each task, determine exactly what context the executing agent needs. Not the
full research pile. Not even the full design doc. Just the surgical slice relevant to
_this_ task.

**Execute**: Run each task through an agent with the right model, the right context window,
the right tools, and the right verification.

**Verify**: Gates (compile, test, clippy, review) validate the output.

**Ship**: Commit, merge, close the loop.

### 1.2 Where Roko Falls Short Today

The machinery exists but the workflow is fragmented:

**Problem 1: No aggregation support.** The user manually assembles research into files,
copies them around, and hopes the agent's context window is big enough. There's no
structured way to say "here are 47 files of research -- synthesize them."

**Problem 2: Context framing is one-shot.** The `PromptAssemblyService` (9-layer builder)
assembles context once per task. There's no concept of progressive refinement -- taking a
large context, producing a compressed intermediate artifact, then using that artifact as
context for the next pass. The system treats every task as if it starts from scratch.

**Problem 3: Task TOMLs get overloaded.** A `TaskDef` has a `context` field with `read_files`,
`symbols`, and `snippets`. In practice, the operator stuffs everything they think the agent
might need into the task definition. For architectural tasks run by Opus, this is fine. For
mechanical tasks run by Haiku, it's waste -- the context overwhelms the useful signal.

**Problem 4: No automatic splitting.** When a task is too large for its assigned model's
context window, the system either truncates or fails. There's a `split_into` field on
`TaskDef` but it's declarative only -- the user has to pre-split manually. No runtime
decomposition happens.

**Problem 5: The PRD-to-plan gap is a cliff.** `roko prd plan <slug>` generates a `tasks.toml`
from a PRD in one shot. If the PRD is weak, the plan is weak. There's no iterative refinement
between "here's my research" and "here's an executable plan." The user either trusts the
one-shot output or manually edits the TOML.

**Problem 6: No workflow state across passes.** Each CLI invocation is stateless. The user
can't say "I ran a research pass, now funnel it, now decompose it" and have the system
track that progression. The `--resume` flag resumes execution state, not workflow state.

### 1.3 What This Document Proposes

A layered system that supports the full aggregation-to-ship workflow:

1. **Context Packs** -- structured containers for research material with progressive
   compression
2. **Funnel Passes** -- explicit pipeline stages that refine context with windowing for
   different model sizes
3. **Auto-decomposition** -- runtime task splitting based on context budget, model
   capability, and task complexity
4. **Interactive steering** -- TUI and CLI commands that let the user guide each stage
5. **Protocol integration** -- ACP and MCP hooks so the workflow works from editors, not
   just the terminal

---

## 2. The Workflow Model

### 2.1 The Funnel Pipeline

The core model is a **funnel** -- a sequence of passes that progressively refine raw
material into executable work. Each pass has:

- **Input**: a context pack (files, artifacts, prior pass outputs)
- **Agent**: a model+role appropriate for the task
- **Output**: a refined artifact (design doc, architecture spec, task plan)
- **Gate**: validation that the output meets quality thresholds

```
                    ┌─────────────────────────────────────────┐
                    │           CONTEXT PACK (raw)            │
                    │  research/, specs/, screenshots/, notes  │
                    │  ~500K tokens of unstructured material   │
                    └──────────────────┬──────────────────────┘
                                       │
                    ┌──────────────────▼──────────────────────┐
                    │         PASS 1: SYNTHESIS               │
                    │  Agent: Researcher (Opus)               │
                    │  Window: 200K tokens                    │
                    │  Output: design-brief.md (~10K tokens)  │
                    │  Gate: structure check, coverage check  │
                    └──────────────────┬──────────────────────┘
                                       │
                    ┌──────────────────▼──────────────────────┐
                    │         PASS 2: ARCHITECTURE            │
                    │  Agent: Architect (Opus)                │
                    │  Window: 100K tokens                    │
                    │  Input: design-brief + repo context     │
                    │  Output: architecture-spec.md (~5K)     │
                    │  Gate: grounding check, dep check       │
                    └──────────────────┬──────────────────────┘
                                       │
                    ┌──────────────────▼──────────────────────┐
                    │         PASS 3: DECOMPOSITION           │
                    │  Agent: Strategist (Opus/Sonnet)        │
                    │  Window: 50K tokens                     │
                    │  Input: arch-spec + repo context        │
                    │  Output: tasks.toml (N tasks)           │
                    │  Gate: structural validation, DAG check │
                    └──────────────────┬──────────────────────┘
                                       │
                    ┌──────────────────▼──────────────────────┐
                    │         PASS 4: SCOPING                 │
                    │  Per-task context windowing              │
                    │  Mechanical tasks: 8K-16K tokens        │
                    │  Focused tasks: 16K-50K tokens          │
                    │  Integrative tasks: 50K-100K tokens     │
                    └──────────────────┬──────────────────────┘
                                       │
                    ┌──────────────────▼──────────────────────┐
                    │         PASS 5: EXECUTION               │
                    │  Per-task: model selection, dispatch,    │
                    │  gates, retry/replan                     │
                    │  Existing orchestrate.rs machinery      │
                    └─────────────────────────────────────────┘
```

### 2.2 Context Packs

A **context pack** is a directory with metadata that the system can reason about:

```
.roko/packs/<pack-id>/
  manifest.toml          # pack metadata: sources, budget, status
  raw/                   # original input files (symlinks or copies)
  pass-01-synthesis.md   # output of synthesis pass
  pass-02-arch.md        # output of architecture pass
  pass-03-tasks.toml     # output of decomposition pass
  scoping/               # per-task context slices
    T01-context.md
    T02-context.md
    ...
```

The `manifest.toml`:

```toml
[pack]
id = "wire-mcp-server"
created = "2026-04-29T12:00:00Z"
status = "decomposed"      # raw | synthesized | architected | decomposed | executing | done

[sources]
# Where the raw material came from
dirs = ["research/mcp/", "specs/mcp-protocol/"]
files = ["notes/mcp-ideas.md", "reference/mcp-spec.json"]
urls = []

[budget]
# Token budget constraints for each pass
synthesis_budget = 200_000
arch_budget = 100_000
decompose_budget = 50_000

[passes]
# Record of completed passes
[[passes.completed]]
name = "synthesis"
agent = "researcher"
model = "claude-opus-4-6-20250514"
input_tokens = 187_432
output_tokens = 4_891
output = "pass-01-synthesis.md"
timestamp = "2026-04-29T12:05:00Z"
```

### 2.3 CLI Commands

New commands for the funnel workflow:

```bash
# Create a context pack from a folder of research
roko pack create "wire-mcp-server" --from research/mcp/ specs/mcp-protocol/

# Add more material to an existing pack
roko pack add wire-mcp-server notes/extra-research.md

# Run the synthesis pass (Researcher agent compresses raw material)
roko pack synthesize wire-mcp-server

# Run the architecture pass (Architect agent produces spec)
roko pack architect wire-mcp-server

# Run the decomposition pass (Strategist agent produces tasks.toml)
roko pack decompose wire-mcp-server

# Scope tasks: compute per-task context slices
roko pack scope wire-mcp-server

# Execute: delegates to existing plan run machinery
roko pack execute wire-mcp-server

# Run the full pipeline end to end
roko pack pipeline wire-mcp-server

# Show status of a pack
roko pack status wire-mcp-server

# List all packs
roko pack list

# Interactive: open pack in TUI for guided refinement
roko pack tui wire-mcp-server
```

Each command is idempotent and resumable. `pack pipeline` runs all passes in sequence,
stopping at any gate failure. The user can intervene, edit artifacts, then resume.

### 2.4 Integration with Existing Commands

The pack system sits on top of the existing PRD and plan infrastructure, not beside it:

```
roko pack create   →  creates .roko/packs/<id>/
roko pack synthesize  →  calls Researcher agent (same as roko research)
roko pack architect   →  calls Architect agent
roko pack decompose   →  calls Strategist agent → generates tasks.toml
                         (same format as roko prd plan output)
roko pack execute     →  delegates to roko plan run
```

A pack's `pass-03-tasks.toml` IS a standard plan. The pack just tracks the provenance
of how it was produced.

---

## 3. Progressive Context Refinement

### 3.1 The Context Window Problem

Different models have different context windows. Different task tiers need different
amounts of context. The current system doesn't handle this well:

| Model | Context Window | Best For |
|-------|---------------|----------|
| Haiku | 200K tokens | Mechanical: rename, move, boilerplate |
| Sonnet | 200K tokens | Focused: implement a function, write tests |
| Opus | 200K tokens | Integrative: multi-file refactor, architecture |

But context window size isn't the real constraint. The real constraint is **attention
degradation** -- models perform worse with more context, even when it fits. The sweet
spot for different task types:

| Task Tier | Effective Context Budget | Why |
|-----------|------------------------|-----|
| Mechanical | 4K-16K tokens | Task is simple; more context = more distraction |
| Focused | 16K-50K tokens | Needs local context (file + dependencies) |
| Integrative | 50K-100K tokens | Needs architectural context |
| Architectural | 100K-200K tokens | Needs full system understanding |

### 3.2 Context Windowing Strategy

For each task, the system computes a **context slice** -- the minimal context needed for
that specific task. The slice is computed from:

1. **Task definition**: the `files`, `symbols`, `snippets` fields from tasks.toml
2. **Dependency outputs**: outputs from upstream tasks that this task depends on
3. **Repo context**: workspace structure, conventions, related code
4. **Architecture artifacts**: the architecture spec from pass 2
5. **Knowledge store**: relevant entries from roko-neuro

The windowing algorithm:

```
function compute_context_slice(task, pack, repo):
    budget = tier_budget(task.tier)      # 16K for mechanical, 50K for focused, etc.

    sections = []

    # Layer 1: Task instructions (always included, ~500 tokens)
    sections.push(task.title + task.description)
    budget -= token_count(sections[0])

    # Layer 2: Target file contents (highest priority)
    for file in task.files:
        content = read_file(file)
        if token_count(content) <= budget * 0.4:
            sections.push(content)
            budget -= token_count(content)

    # Layer 3: Dependency outputs (what upstream tasks produced)
    for dep in task.depends_on:
        output = load_task_output(dep)
        summary = truncate_to(output, budget * 0.15)
        sections.push(summary)
        budget -= token_count(summary)

    # Layer 4: Architecture context (from pack pass-02)
    if pack.has_arch_spec():
        relevant = extract_relevant_sections(pack.arch_spec, task)
        sections.push(truncate_to(relevant, budget * 0.2))

    # Layer 5: Repo conventions + workspace map
    conventions = detect_conventions(repo.root)
    sections.push(truncate_to(conventions, budget * 0.1))

    # Layer 6: Knowledge store (anti-patterns, prior experience)
    knowledge = neuro_store.query(task.title, limit=3)
    sections.push(render_knowledge(knowledge, budget * 0.1))

    return assemble(sections)
```

### 3.3 Sliding Window for Large Inputs

When a context pack's raw material exceeds any single model's context window, the
synthesis pass uses a **sliding window**:

```
function synthesize_large_pack(pack, model):
    raw_files = sorted(pack.raw_files, by=relevance)
    window_size = model.context_window * 0.7  # leave room for system prompt + output

    chunk_summaries = []

    # Pass A: Chunk-level summaries
    for chunk in sliding_window(raw_files, window_size, overlap=0.1):
        summary = agent.summarize(chunk, role="researcher")
        chunk_summaries.push(summary)

    # Pass B: Cross-chunk synthesis
    if total_tokens(chunk_summaries) <= window_size:
        return agent.synthesize(chunk_summaries, role="researcher")
    else:
        # Recursive: summarize the summaries
        return synthesize_large_pack(
            pack_from(chunk_summaries),
            model
        )
```

This is the **map-reduce** pattern applied to context. Each level reduces volume by ~10x.
Three levels can handle 1000x the context window: a model with 200K tokens can
process ~200M tokens of raw material (though cost scales linearly).

### 3.4 Context Provenance Tracking

Every context slice records where each section came from:

```toml
[context_slice]
task_id = "T03"
total_tokens = 23_847

[[context_slice.sections]]
source = "task_definition"
tokens = 312
priority = "required"

[[context_slice.sections]]
source = "file:crates/roko-compose/src/system_prompt_builder.rs"
tokens = 8_921
priority = "high"
lines = "1-350"

[[context_slice.sections]]
source = "dependency_output:T01"
tokens = 2_103
priority = "medium"

[[context_slice.sections]]
source = "arch_spec:pass-02-arch.md#prompt-assembly"
tokens = 1_456
priority = "medium"

[[context_slice.sections]]
source = "knowledge:anti-pattern-duplicate-impl"
tokens = 342
priority = "low"
```

This lets the user inspect and edit what context each task sees. After a task fails, the
user can see "this task had 24K tokens of context, 37% was the target file" and decide
whether to adjust.

---

## 4. Smart Task Decomposition

### 4.1 Current State

The `TaskDef` struct in `crates/roko-cli/src/task_parser.rs` already has rich metadata:

- `tier`: mechanical / focused / integrative / architectural
- `model_hint`: suggested model
- `files`: target files
- `context`: read_files, symbols, snippets
- `depends_on`: task dependencies
- `split_into`: pre-declared subtask IDs
- `max_loc`: maximum lines of change
- `acceptance`: free-form criteria
- `acceptance_contract`: typed gate contract
- `verify`: per-task verification steps

The problem isn't the schema -- it's that decomposition is done once (by the strategist
agent) and never revised.

### 4.2 Auto-Split Algorithm

When a task is dispatched, the system checks whether it should be split:

```
function should_split(task, model):
    # Rule 1: Context exceeds effective budget
    context_tokens = compute_context_slice(task).total_tokens
    effective_budget = tier_budget(task.tier)
    if context_tokens > effective_budget * 1.5:
        return SplitReason::ContextOverflow

    # Rule 2: Too many files for the tier
    if task.tier == "mechanical" and len(task.files) > 3:
        return SplitReason::TooManyFiles
    if task.tier == "focused" and len(task.files) > 8:
        return SplitReason::TooManyFiles

    # Rule 3: max_loc suggests scope creep
    if task.max_loc and task.max_loc > tier_loc_limit(task.tier):
        return SplitReason::ScopeCreep

    # Rule 4: Historical data suggests this shape fails
    # (query learning system for similar task shapes)
    if error_pattern_store.has_pattern(task.shape()):
        return SplitReason::HistoricalFailure

    return None
```

When a task is split, the system generates subtasks:

```
function auto_split(task, reason):
    match reason:
        ContextOverflow:
            # Split by file groups
            file_groups = cluster_files(task.files, max_group_size=3)
            subtasks = []
            for (i, group) in file_groups:
                sub = TaskDef {
                    id: f"{task.id}-sub{i}",
                    title: f"{task.title} ({group.description})",
                    tier: demote_tier(task.tier),  # integrative → focused
                    files: group.files,
                    depends_on: task.depends_on,
                    context: extract_context_for(task.context, group.files),
                    verify: task.verify,  # inherit verification
                }
                subtasks.push(sub)
            # Add a merge task that depends on all subtasks
            subtasks.push(TaskDef {
                id: f"{task.id}-merge",
                title: f"Verify {task.title} integration",
                tier: "focused",
                depends_on: subtasks.iter().map(|s| s.id).collect(),
                verify: task.verify,
            })
            return subtasks

        TooManyFiles:
            # Split into per-file or per-module tasks
            return split_by_module(task)

        ScopeCreep:
            # Use strategist to re-decompose
            return agent_decompose(task, model="strategist")

        HistoricalFailure:
            # Query error patterns for the split strategy that worked
            strategy = error_pattern_store.best_split(task.shape())
            return apply_strategy(task, strategy)
```

### 4.3 Tier Demotion

When tasks are split, their tier is adjusted:

| Parent Tier | Subtask Tier | Rationale |
|-------------|-------------|-----------|
| Architectural | Integrative | Each subtask is a subsystem |
| Integrative | Focused | Each subtask is a module |
| Focused | Mechanical | Each subtask is a single file change |
| Mechanical | Mechanical | Already atomic; split by file only |

Tier demotion enables model demotion: a task originally planned for Opus can have its
subtasks run by Sonnet or even Haiku.

### 4.4 Acceptance Criteria Propagation

When a parent task has acceptance criteria, the system distributes them:

```
Parent task: "Wire SystemPromptBuilder into orchestrate.rs"
  Acceptance: ["cargo test passes", "builder is called per-task", "9 layers present"]

Split into:
  T1: "Add SystemPromptBuilder import and initialization"
    Acceptance: ["cargo check passes", "builder struct is constructed"]
  T2: "Wire builder into per-task dispatch"
    Acceptance: ["builder is called per-task"]
  T3: "Verify 9-layer assembly"
    Acceptance: ["9 layers present in generated prompt", "cargo test passes"]
  T-merge: "Integration verification"
    Acceptance: ["cargo test passes", "all acceptance criteria from parent"]
```

The merge task always inherits the full parent acceptance criteria. Subtask criteria are
derived by the strategist or by pattern matching.

### 4.5 DAG Update Protocol

When tasks are split at runtime, the executor DAG must be updated:

1. Mark the parent task as `split` (not `pending`, not `done`)
2. Insert subtasks into the DAG with the parent's position
3. Update any tasks that `depends_on` the parent to instead depend on the merge subtask
4. Persist the updated DAG to the executor snapshot
5. Emit a `PlanRevision` event so the TUI/dashboard can update

This is the hardest part. The existing `ParallelExecutor` in roko-orchestrator already
supports `PlanRevisionRequest` and `ReplanResult`, so the mechanism exists. The new part
is triggering it from context analysis rather than from gate failures.

---

## 5. Interactive Refinement

### 5.1 The Steering Problem

The current CLI is fire-and-forget: you run a command, it does everything, you see the
result. For simple tasks this is fine. For the funnel workflow, the user needs to steer:

- "This synthesis missed the security requirements. Re-synthesize with emphasis on auth."
- "Task T05 is too broad. Split it into per-crate tasks."
- "The architecture spec doesn't account for backward compatibility. Add a section."
- "I want to approve the task plan before execution starts."

### 5.2 CLI Steering Commands

Within a pack workflow, the user can intervene between passes:

```bash
# After synthesis: review and re-run with guidance
roko pack show wire-mcp-server --pass synthesis
roko pack edit wire-mcp-server pass-01-synthesis.md   # opens in $EDITOR
roko pack re-synthesize wire-mcp-server --focus "security,auth,permissions"

# After decomposition: review tasks, adjust, then scope
roko pack show wire-mcp-server --pass decompose
roko pack split wire-mcp-server T05 --by-module       # manual split
roko pack adjust wire-mcp-server T03 --tier focused   # change tier
roko pack scope wire-mcp-server                        # compute context slices

# During execution: pause, inspect, adjust
roko pack pause wire-mcp-server                        # pause at next task boundary
roko pack status wire-mcp-server                       # show progress
roko pack retry wire-mcp-server T07                    # retry a failed task
roko pack skip wire-mcp-server T07                     # skip a task
roko pack resume wire-mcp-server                       # continue execution
```

### 5.3 TUI Integration

The TUI dashboard (`crates/roko-cli/src/tui/`) gets a new tab for pack management:

**Pack Overview Tab (F8)**:
```
┌─ Packs ──────────────────────────────────────────────────────────────┐
│                                                                      │
│  wire-mcp-server        decomposed   5 passes   42 tasks            │
│  refactor-gate-system   synthesized  2 passes    0 tasks            │
│  add-openai-streaming   executing    5 passes   12 tasks  (3/12)    │
│                                                                      │
├─ wire-mcp-server ────────────────────────────────────────────────────┤
│                                                                      │
│  Pass 1: Synthesis     DONE   10K tokens   researcher/opus           │
│  Pass 2: Architecture  DONE    5K tokens   architect/opus            │
│  Pass 3: Decompose     DONE    3K tokens   strategist/sonnet         │
│  Pass 4: Scoping       DONE   42 slices    avg 23K tokens/task      │
│  Pass 5: Execution     ACTIVE 12/42 done   3 running  2 failed     │
│                                                                      │
│  [Enter] View pass detail  [s] Split task  [r] Retry  [p] Pause    │
└──────────────────────────────────────────────────────────────────────┘
```

**Task Context Inspector** (drill into a task):
```
┌─ T07: Wire knowledge store query ────────────────────────────────────┐
│                                                                      │
│  Tier: focused          Model: sonnet        Status: failed          │
│  Context: 34,218 tokens (budget: 50,000)                            │
│                                                                      │
│  Context Breakdown:                                                  │
│  ██████████░░░░░░░░  38%  target files (3 files)                    │
│  ████░░░░░░░░░░░░░░  15%  dependency outputs (T03, T05)            │
│  ███░░░░░░░░░░░░░░░  12%  architecture spec                        │
│  ██░░░░░░░░░░░░░░░░   8%  repo conventions                         │
│  █░░░░░░░░░░░░░░░░░   4%  knowledge store (2 entries)              │
│  ░░░░░░░░░░░░░░░░░░  23%  remaining budget                         │
│                                                                      │
│  Gate Results:                                                       │
│  compile  PASS   0.8s                                                │
│  test     FAIL   clippy found 3 warnings                             │
│  clippy   FAIL   unused import on line 47                            │
│                                                                      │
│  [c] View context  [g] Gate output  [s] Split  [r] Retry  [e] Edit │
└──────────────────────────────────────────────────────────────────────┘
```

### 5.4 Approval Gates

For high-stakes work, the user can require approval between passes:

```toml
# In manifest.toml
[approval]
require_before = ["execute"]   # pause before execution starts
auto_approve = ["synthesis", "architecture"]  # auto-advance these
```

Or via CLI:

```bash
roko pack pipeline wire-mcp-server --approve-before execute
```

When an approval gate is hit, the system pauses and notifies the user (terminal bell,
notification, or SSE event to the dashboard). The user reviews the artifacts, then:

```bash
roko pack approve wire-mcp-server     # approve and continue
roko pack reject wire-mcp-server      # reject; user edits artifacts manually
```

---

## 6. ACP/MCP Integration

### 6.1 ACP: Editor-Driven Workflow

The ACP protocol (`crates/roko-acp/`) enables editors to drive the workflow. Currently
ACP supports single-prompt workflows (Express/Standard/Full templates). The pack system
extends ACP with multi-pass awareness:

**New ACP Methods**:

```jsonc
// Create a pack from editor context
{
  "method": "pack/create",
  "params": {
    "name": "wire-mcp-server",
    "sources": [
      {"type": "file", "path": "research/mcp-spec.md"},
      {"type": "selection", "content": "...selected text from editor..."},
      {"type": "url", "url": "https://spec.modelcontextprotocol.io/"}
    ]
  }
}

// Run a specific pass
{
  "method": "pack/run_pass",
  "params": {
    "pack_id": "wire-mcp-server",
    "pass": "synthesize"
  }
}

// Get pack status with pass details
{
  "method": "pack/status",
  "params": { "pack_id": "wire-mcp-server" }
}

// Approve a pending pass
{
  "method": "pack/approve",
  "params": { "pack_id": "wire-mcp-server" }
}
```

**Session Updates**: pack progress is streamed as `session/update` notifications with a
new `pack_progress` content block type:

```jsonc
{
  "method": "session/update",
  "params": {
    "sessionId": "...",
    "type": "pack_progress",
    "data": {
      "pack_id": "wire-mcp-server",
      "pass": "architecture",
      "status": "running",
      "progress": 0.45,
      "tokens_used": 12_340
    }
  }
}
```

### 6.2 MCP: Tool-Driven Context

MCP servers provide tools that the agent uses during execution. The pack system integrates
with MCP in two ways:

**Context Tools**: MCP servers can provide context that feeds into the synthesis pass:

```toml
# In manifest.toml
[mcp_context]
servers = ["github", "jira", "confluence"]

# During synthesis, the researcher agent can call:
# - github:search_code("mcp server implementation")
# - jira:get_issue("ROKO-1234")
# - confluence:get_page("MCP Architecture")
```

**Execution Tools**: Per-task MCP server configuration inherited from the pack:

```toml
# In tasks.toml, tasks can specify which MCP servers they need
[[task]]
id = "T05"
title = "Implement MCP transport layer"
mcp_servers = ["github", "roko-code-intel"]
```

### 6.3 Existing Integration Points

The existing codebase already has the wiring for MCP passthrough:

- `agent.mcp_config` in `roko.toml` passes `--mcp-config` to the Claude CLI
  (`crates/roko-cli/src/agent_config.rs`)
- `TaskDef.mcp_servers` field exists in `crates/roko-cli/src/task_parser.rs`
- `McpConfig` and `McpServerConfig` types exist in `crates/roko-agent/src/mcp.rs`

The pack system reuses all of this. It only adds the ability to specify MCP context
sources at the pack level (for synthesis/architecture passes) in addition to the
per-task level (for execution).

---

## 7. Implementation Plan

### Phase 1: Context Packs (Foundation)

**Goal**: Users can create context packs, add material, and run synthesis passes.

**Timeline**: 2-3 weeks

**Files to create**:
- `crates/roko-cli/src/pack.rs` -- pack CRUD operations
- `crates/roko-cli/src/pack_pipeline.rs` -- pass execution logic
- `crates/roko-cli/src/commands/pack.rs` -- CLI command handlers

**Files to modify**:
- `crates/roko-cli/src/main.rs` -- add `Pack` subcommand
- `crates/roko-cli/src/commands/mod.rs` -- register pack module
- `crates/roko-cli/src/lib.rs` -- export pack module

#### Step 1.1: Pack Data Model

Create the pack manifest and directory structure:

```rust
// crates/roko-cli/src/pack.rs

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use anyhow::Result;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackStatus {
    Raw,
    Synthesized,
    Architected,
    Decomposed,
    Scoped,
    Executing,
    Done,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackManifest {
    pub pack: PackMeta,
    pub sources: PackSources,
    pub budget: PackBudget,
    #[serde(default)]
    pub passes: Vec<PassRecord>,
    #[serde(default)]
    pub approval: ApprovalConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackMeta {
    pub id: String,
    pub created: DateTime<Utc>,
    pub status: PackStatus,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackSources {
    #[serde(default)]
    pub dirs: Vec<PathBuf>,
    #[serde(default)]
    pub files: Vec<PathBuf>,
    #[serde(default)]
    pub urls: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackBudget {
    pub synthesis_budget: usize,    // tokens
    pub arch_budget: usize,
    pub decompose_budget: usize,
}

impl Default for PackBudget {
    fn default() -> Self {
        Self {
            synthesis_budget: 200_000,
            arch_budget: 100_000,
            decompose_budget: 50_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassRecord {
    pub name: String,
    pub agent_role: String,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub output_file: String,
    pub timestamp: DateTime<Utc>,
    pub duration_secs: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ApprovalConfig {
    #[serde(default)]
    pub require_before: Vec<String>,
    #[serde(default)]
    pub auto_approve: Vec<String>,
}

/// Create a new context pack.
pub fn create_pack(
    roko_dir: &Path,
    id: &str,
    sources: PackSources,
    description: Option<String>,
) -> Result<PathBuf> {
    let pack_dir = roko_dir.join("packs").join(id);
    std::fs::create_dir_all(&pack_dir)?;
    std::fs::create_dir_all(pack_dir.join("raw"))?;
    std::fs::create_dir_all(pack_dir.join("scoping"))?;

    let manifest = PackManifest {
        pack: PackMeta {
            id: id.to_string(),
            created: Utc::now(),
            status: PackStatus::Raw,
            description,
        },
        sources,
        budget: PackBudget::default(),
        passes: Vec::new(),
        approval: ApprovalConfig::default(),
    };

    let manifest_path = pack_dir.join("manifest.toml");
    let toml_str = toml::to_string_pretty(&manifest)?;
    std::fs::write(&manifest_path, toml_str)?;

    // Symlink or copy source files into raw/
    link_sources(&pack_dir, &manifest.sources)?;

    Ok(pack_dir)
}

/// List all packs in the workspace.
pub fn list_packs(roko_dir: &Path) -> Result<Vec<PackManifest>> {
    let packs_dir = roko_dir.join("packs");
    if !packs_dir.exists() {
        return Ok(Vec::new());
    }
    let mut manifests = Vec::new();
    for entry in std::fs::read_dir(&packs_dir)? {
        let entry = entry?;
        if entry.path().is_dir() {
            let manifest_path = entry.path().join("manifest.toml");
            if manifest_path.exists() {
                let content = std::fs::read_to_string(&manifest_path)?;
                let manifest: PackManifest = toml::from_str(&content)?;
                manifests.push(manifest);
            }
        }
    }
    manifests.sort_by(|a, b| b.pack.created.cmp(&a.pack.created));
    Ok(manifests)
}

fn link_sources(pack_dir: &Path, sources: &PackSources) -> Result<()> {
    let raw_dir = pack_dir.join("raw");
    for dir in &sources.dirs {
        if dir.exists() {
            let dest = raw_dir.join(dir.file_name().unwrap_or_default());
            // Symlink for efficiency; copy if symlink fails
            #[cfg(unix)]
            std::os::unix::fs::symlink(dir, &dest)
                .or_else(|_| copy_dir_recursive(dir, &dest))?;
            #[cfg(not(unix))]
            copy_dir_recursive(dir, &dest)?;
        }
    }
    for file in &sources.files {
        if file.exists() {
            let dest = raw_dir.join(file.file_name().unwrap_or_default());
            #[cfg(unix)]
            std::os::unix::fs::symlink(file, &dest)
                .or_else(|_| std::fs::copy(file, &dest).map(|_| ()))?;
            #[cfg(not(unix))]
            std::fs::copy(file, &dest)?;
        }
    }
    Ok(())
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let target = dest.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir_recursive(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}
```

**Acceptance criteria**:
- `roko pack create "test-pack" --from some-dir/` creates `.roko/packs/test-pack/` with
  manifest.toml and raw/ directory
- `roko pack list` shows all packs with status
- `roko pack status test-pack` shows pack details
- Unit tests for manifest serialization round-trip

#### Step 1.2: Synthesis Pass

Wire the synthesis pass using the existing agent dispatch infrastructure:

```rust
// crates/roko-cli/src/pack_pipeline.rs (synthesis portion)

use crate::agent_config::{command_from_config, load_gateway_env, model_from_config};
use crate::agent_exec::{AgentExecOpts, run_agent_capture_silent};

pub async fn run_synthesis(
    workdir: &Path,
    pack_dir: &Path,
    manifest: &mut PackManifest,
    model: Option<&str>,
) -> Result<()> {
    // 1. Collect all raw material
    let raw_content = collect_raw_content(&pack_dir.join("raw"))?;
    let total_tokens = estimate_tokens(&raw_content);

    // 2. If material fits in one window, single-pass synthesis
    let budget = manifest.budget.synthesis_budget;
    let output = if total_tokens <= budget {
        single_pass_synthesis(workdir, &raw_content, model).await?
    } else {
        // 3. If too large, sliding window map-reduce
        sliding_window_synthesis(workdir, &raw_content, budget, model).await?
    };

    // 4. Write output
    let output_file = "pass-01-synthesis.md";
    std::fs::write(pack_dir.join(output_file), &output)?;

    // 5. Update manifest
    manifest.passes.push(PassRecord {
        name: "synthesis".to_string(),
        agent_role: "researcher".to_string(),
        model: model.unwrap_or("default").to_string(),
        input_tokens: total_tokens as u64,
        output_tokens: estimate_tokens(&output) as u64,
        output_file: output_file.to_string(),
        timestamp: Utc::now(),
        duration_secs: 0.0, // filled by caller
    });
    manifest.pack.status = PackStatus::Synthesized;

    Ok(())
}

fn collect_raw_content(raw_dir: &Path) -> Result<String> {
    let mut content = String::new();
    collect_files_recursive(raw_dir, &mut content)?;
    Ok(content)
}

fn collect_files_recursive(dir: &Path, content: &mut String) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    let mut entries: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_files_recursive(&path, content)?;
        } else if is_text_file(&path) {
            use std::fmt::Write;
            writeln!(content, "\n--- {} ---\n", path.display())?;
            let file_content = std::fs::read_to_string(&path)
                .unwrap_or_else(|_| "(binary or unreadable)".to_string());
            content.push_str(&file_content);
        }
    }
    Ok(())
}

fn is_text_file(path: &Path) -> bool {
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    matches!(ext,
        "md" | "txt" | "rs" | "toml" | "yaml" | "yml" | "json"
        | "ts" | "tsx" | "js" | "jsx" | "py" | "go" | "sh"
        | "html" | "css" | "sql" | "proto" | "graphql"
    )
}

fn estimate_tokens(text: &str) -> usize {
    // Rough estimate: ~4 chars per token for English text
    text.len() / 4
}
```

**Acceptance criteria**:
- `roko pack synthesize test-pack` produces `pass-01-synthesis.md`
- Large packs (>200K tokens) use sliding window approach
- Manifest is updated with pass record
- Pack status advances to `synthesized`

#### Step 1.3: CLI Command Registration

```rust
// crates/roko-cli/src/commands/pack.rs

use crate::*;

pub(crate) async fn cmd_pack(cli: &Cli, cmd: PackCmd) -> Result<i32> {
    let workdir = resolve_workdir(cli);
    let roko_dir = workdir.join(".roko");

    match cmd {
        PackCmd::Create { name, from, description } => {
            let sources = roko_cli::pack::PackSources {
                dirs: from.clone(),
                files: Vec::new(),
                urls: Vec::new(),
            };
            let pack_dir = roko_cli::pack::create_pack(
                &roko_dir, &name, sources, description
            )?;
            println!("Created pack: {}", pack_dir.display());
            Ok(0)
        }
        PackCmd::List => {
            let packs = roko_cli::pack::list_packs(&roko_dir)?;
            if packs.is_empty() {
                println!("No packs. Create one: roko pack create \"name\" --from dir/");
            } else {
                for p in &packs {
                    println!("  {:30} {:15} {} passes",
                        p.pack.id, format!("{:?}", p.pack.status), p.passes.len());
                }
            }
            Ok(0)
        }
        PackCmd::Status { name } => {
            let manifest = roko_cli::pack::load_manifest(&roko_dir, &name)?;
            println!("Pack: {}", manifest.pack.id);
            println!("Status: {:?}", manifest.pack.status);
            println!("Created: {}", manifest.pack.created);
            for pass in &manifest.passes {
                println!("  Pass {}: {} ({}, {}K in / {}K out)",
                    pass.name, pass.agent_role, pass.model,
                    pass.input_tokens / 1000, pass.output_tokens / 1000);
            }
            Ok(0)
        }
        PackCmd::Synthesize { name } => {
            let mut manifest = roko_cli::pack::load_manifest(&roko_dir, &name)?;
            let pack_dir = roko_dir.join("packs").join(&name);
            let model = cli.model.as_deref();
            roko_cli::pack_pipeline::run_synthesis(
                &workdir, &pack_dir, &mut manifest, model
            ).await?;
            roko_cli::pack::save_manifest(&roko_dir, &name, &manifest)?;
            println!("Synthesis complete: {}/pass-01-synthesis.md", pack_dir.display());
            Ok(0)
        }
        // ... other subcommands follow the same pattern
    }
}
```

Add to `main.rs`:

```rust
// In the Command enum:
/// Manage context packs (aggregate, synthesize, decompose, execute).
Pack {
    #[command(subcommand)]
    cmd: PackCmd,
},

// PackCmd subcommands:
#[derive(Debug, Subcommand)]
enum PackCmd {
    /// Create a new context pack from directories or files.
    Create {
        name: String,
        #[arg(long, num_args = 1..)]
        from: Vec<PathBuf>,
        #[arg(long)]
        description: Option<String>,
    },
    /// List all context packs.
    List,
    /// Show detailed pack status.
    Status { name: String },
    /// Add sources to an existing pack.
    Add {
        name: String,
        #[arg(num_args = 1..)]
        sources: Vec<PathBuf>,
    },
    /// Run synthesis pass (compress raw material).
    Synthesize { name: String },
    /// Run architecture pass (produce spec from synthesis).
    Architect { name: String },
    /// Run decomposition pass (produce tasks.toml from spec).
    Decompose { name: String },
    /// Compute per-task context slices.
    Scope { name: String },
    /// Execute tasks (delegates to plan run).
    Execute { name: String },
    /// Run full pipeline end to end.
    Pipeline {
        name: String,
        #[arg(long)]
        approve_before: Option<Vec<String>>,
    },
    /// Show a specific pass output.
    Show {
        name: String,
        #[arg(long)]
        pass: Option<String>,
    },
    /// Manually split a task.
    Split {
        name: String,
        task_id: String,
        #[arg(long)]
        by_module: bool,
    },
}
```

**Acceptance criteria**:
- `roko pack --help` shows all subcommands with descriptions
- `roko pack create/list/status` work end-to-end
- `roko pack synthesize` dispatches to agent and produces output

---

### Phase 2: Full Funnel Pipeline (Architecture + Decomposition)

**Goal**: Complete the pipeline from synthesis through task decomposition with context
scoping.

**Timeline**: 2-3 weeks (after Phase 1)

**Files to create/modify**:
- `crates/roko-cli/src/pack_pipeline.rs` -- add architect, decompose, scope passes
- `crates/roko-cli/src/context_slicer.rs` -- per-task context windowing

#### Step 2.1: Architecture Pass

Reuse the Architect agent role (already defined in `roko-core`) to produce an
architecture spec from the synthesis output:

```rust
// In pack_pipeline.rs

pub async fn run_architecture(
    workdir: &Path,
    pack_dir: &Path,
    manifest: &mut PackManifest,
    model: Option<&str>,
) -> Result<()> {
    // Input: synthesis output + repo context
    let synthesis = std::fs::read_to_string(pack_dir.join("pass-01-synthesis.md"))?;
    let repo_context = crate::repo_context::build_repo_context(
        workdir,
        &extract_keywords(&synthesis),
    ).await?;

    let system_prompt = format!(
        "You are an expert software architect. Given a design brief and repository context, \
         produce an architecture specification that includes:\n\
         1. Component boundaries and responsibilities\n\
         2. Data flow and interfaces between components\n\
         3. File-level change manifest (which files to create/modify/delete)\n\
         4. Dependency ordering (what must be done before what)\n\
         5. Risk assessment and mitigation strategies\n\
         6. Acceptance criteria for the overall change\n\n\
         Be specific. Reference actual crate names, file paths, and function signatures.\n\n\
         Repository context:\n{repo_section}",
        repo_section = repo_context.to_prompt_section()
    );

    let task_prompt = format!(
        "Produce an architecture specification for the following design brief:\n\n{synthesis}"
    );

    let (exit_code, output) = dispatch_agent(
        workdir, &system_prompt, &task_prompt, "architect", model
    ).await?;

    if exit_code != 0 {
        anyhow::bail!("Architecture pass failed (exit {exit_code})");
    }

    let output_file = "pass-02-arch.md";
    std::fs::write(pack_dir.join(output_file), &output)?;

    manifest.pack.status = PackStatus::Architected;
    // ... record pass

    Ok(())
}
```

**Acceptance criteria**:
- `roko pack architect test-pack` reads synthesis output and produces arch spec
- Arch spec references actual crate names and file paths from repo context
- Manifest status advances to `architected`

#### Step 2.2: Decomposition Pass

Generate `tasks.toml` from the architecture spec. This reuses the existing `prd plan`
machinery but with the architecture spec as input instead of a PRD:

```rust
pub async fn run_decomposition(
    workdir: &Path,
    pack_dir: &Path,
    manifest: &mut PackManifest,
    model: Option<&str>,
) -> Result<()> {
    let arch_spec = std::fs::read_to_string(pack_dir.join("pass-02-arch.md"))?;

    // Use the same task generation prompt as prd plan, but from arch spec
    let task_toml = generate_tasks_from_spec(workdir, &arch_spec, model).await?;

    let output_file = "pass-03-tasks.toml";
    std::fs::write(pack_dir.join(output_file), &task_toml)?;

    // Validate the generated tasks
    let validation = roko_cli::task_parser::validate_tasks_toml(
        &pack_dir.join(output_file)
    )?;
    if !validation.issues.is_empty() {
        for issue in &validation.issues {
            eprintln!("  WARN: {}", issue);
        }
    }

    manifest.pack.status = PackStatus::Decomposed;
    Ok(())
}
```

**Acceptance criteria**:
- `roko pack decompose test-pack` generates valid tasks.toml
- Generated tasks have proper tiers, dependencies, file lists
- `roko plan validate` passes on the generated tasks.toml
- DAG has no cycles

#### Step 2.3: Context Scoping

The context slicer computes per-task context slices:

```rust
// crates/roko-cli/src/context_slicer.rs

use crate::task_parser::TaskDef;

/// Token budgets per task tier.
pub fn tier_budget(tier: &str) -> usize {
    match tier {
        "mechanical" => 16_000,
        "focused" => 50_000,
        "integrative" => 100_000,
        "architectural" => 200_000,
        _ => 50_000,
    }
}

/// Compute the context slice for a single task.
pub fn compute_context_slice(
    task: &TaskDef,
    arch_spec: Option<&str>,
    task_outputs: &HashMap<String, String>,
    repo_root: &Path,
    knowledge_entries: &[String],
) -> ContextSlice {
    let budget = tier_budget(&task.tier);
    let mut sections = Vec::new();
    let mut remaining = budget;

    // Layer 1: Task instructions (always included)
    let instructions = format_task_instructions(task);
    let instr_tokens = estimate_tokens(&instructions);
    sections.push(ContextSection {
        source: "task_definition".to_string(),
        content: instructions,
        tokens: instr_tokens,
        priority: Priority::Required,
    });
    remaining = remaining.saturating_sub(instr_tokens);

    // Layer 2: Target file contents (40% of remaining budget)
    let file_budget = remaining * 40 / 100;
    let file_sections = read_target_files(task, repo_root, file_budget);
    let file_tokens: usize = file_sections.iter().map(|s| s.tokens).sum();
    sections.extend(file_sections);
    remaining = remaining.saturating_sub(file_tokens);

    // Layer 3: Dependency outputs (15% of remaining)
    let dep_budget = remaining * 15 / 100;
    let dep_sections = collect_dependency_outputs(task, task_outputs, dep_budget);
    let dep_tokens: usize = dep_sections.iter().map(|s| s.tokens).sum();
    sections.extend(dep_sections);
    remaining = remaining.saturating_sub(dep_tokens);

    // Layer 4: Architecture context (20% of remaining)
    if let Some(spec) = arch_spec {
        let arch_budget = remaining * 20 / 100;
        let relevant = extract_relevant_arch_sections(spec, task, arch_budget);
        let arch_tokens = estimate_tokens(&relevant.content);
        sections.push(relevant);
        remaining = remaining.saturating_sub(arch_tokens);
    }

    // Layer 5: Knowledge store entries (10% of remaining)
    let knowledge_budget = remaining * 10 / 100;
    let knowledge = render_knowledge_entries(knowledge_entries, knowledge_budget);
    if !knowledge.content.is_empty() {
        let k_tokens = knowledge.tokens;
        sections.push(knowledge);
        remaining = remaining.saturating_sub(k_tokens);
    }

    ContextSlice {
        task_id: task.id.clone(),
        total_tokens: budget - remaining,
        budget,
        sections,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSlice {
    pub task_id: String,
    pub total_tokens: usize,
    pub budget: usize,
    pub sections: Vec<ContextSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSection {
    pub source: String,
    pub content: String,
    pub tokens: usize,
    pub priority: Priority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Priority {
    Required,
    High,
    Medium,
    Low,
}
```

**Acceptance criteria**:
- `roko pack scope test-pack` writes per-task context slices to `scoping/`
- Each slice stays within the tier budget
- Slices include provenance metadata
- Mechanical tasks get smaller context than integrative tasks

---

### Phase 3: Auto-Decomposition and Runtime Splitting

**Goal**: Tasks that are too large for their model/tier are automatically split at
dispatch time.

**Timeline**: 2-3 weeks (after Phase 2)

**Files to create/modify**:
- `crates/roko-cli/src/task_splitter.rs` -- split logic
- `crates/roko-cli/src/orchestrate.rs` -- hook splitter into dispatch loop
- `crates/roko-orchestrator/src/dag.rs` -- DAG mutation for split tasks

#### Step 3.1: Split Decision

```rust
// crates/roko-cli/src/task_splitter.rs

#[derive(Debug)]
pub enum SplitReason {
    ContextOverflow { actual: usize, budget: usize },
    TooManyFiles { count: usize, max: usize },
    ScopeCreep { loc: u32, max: u32 },
    HistoricalFailure { pattern: String },
}

pub fn should_split(task: &TaskDef, context_slice: &ContextSlice) -> Option<SplitReason> {
    let budget = tier_budget(&task.tier);

    // Check context overflow
    if context_slice.total_tokens > budget * 3 / 2 {
        return Some(SplitReason::ContextOverflow {
            actual: context_slice.total_tokens,
            budget,
        });
    }

    // Check file count
    let max_files = match task.tier.as_str() {
        "mechanical" => 3,
        "focused" => 8,
        "integrative" => 15,
        _ => 20,
    };
    if task.files.len() > max_files {
        return Some(SplitReason::TooManyFiles {
            count: task.files.len(),
            max: max_files,
        });
    }

    // Check max_loc
    if let Some(max_loc) = task.max_loc {
        let tier_limit = match task.tier.as_str() {
            "mechanical" => 100,
            "focused" => 300,
            "integrative" => 800,
            _ => 2000,
        };
        if max_loc > tier_limit {
            return Some(SplitReason::ScopeCreep {
                loc: max_loc,
                max: tier_limit,
            });
        }
    }

    None
}
```

#### Step 3.2: Split Execution

```rust
pub fn split_task(task: &TaskDef, reason: &SplitReason) -> Vec<TaskDef> {
    match reason {
        SplitReason::ContextOverflow { .. } | SplitReason::TooManyFiles { .. } => {
            split_by_files(task)
        }
        SplitReason::ScopeCreep { .. } => {
            split_by_scope(task)
        }
        SplitReason::HistoricalFailure { .. } => {
            split_by_files(task)  // conservative default
        }
    }
}

fn split_by_files(task: &TaskDef) -> Vec<TaskDef> {
    let max_files = match task.tier.as_str() {
        "mechanical" => 2,
        "focused" => 4,
        _ => 6,
    };

    let groups = cluster_files(&task.files, max_files);
    let child_tier = demote_tier(&task.tier);

    let mut subtasks: Vec<TaskDef> = groups.iter().enumerate().map(|(i, group)| {
        TaskDef {
            id: format!("{}-sub{}", task.id, i + 1),
            title: format!("{} ({})", task.title, describe_file_group(group)),
            tier: child_tier.clone(),
            files: group.clone(),
            depends_on: task.depends_on.clone(),
            verify: task.verify.clone(),
            model_hint: None,  // let cascade router decide
            ..task.clone()
        }
    }).collect();

    // Add merge/verification task
    let merge_deps: Vec<String> = subtasks.iter().map(|s| s.id.clone()).collect();
    subtasks.push(TaskDef {
        id: format!("{}-merge", task.id),
        title: format!("Verify {} integration", task.title),
        tier: "focused".to_string(),
        files: task.files.clone(),
        depends_on: merge_deps,
        verify: task.verify.clone(),
        acceptance: task.acceptance.clone(),
        ..task.clone()
    });

    subtasks
}

fn demote_tier(tier: &str) -> String {
    match tier {
        "architectural" => "integrative",
        "integrative" => "focused",
        "focused" => "mechanical",
        _ => "mechanical",
    }.to_string()
}
```

#### Step 3.3: Orchestrator Integration

Hook the splitter into the dispatch loop in `orchestrate.rs`:

```rust
// In the task dispatch path (orchestrate.rs), before calling the agent:

let context_slice = compute_context_slice(&task, arch_spec, &task_outputs, &workdir, &[]);

if let Some(reason) = should_split(&task, &context_slice) {
    info!("Auto-splitting task {} (reason: {:?})", task.id, reason);
    let subtasks = split_task(&task, &reason);

    // Update the DAG
    let revision = PlanRevisionRequest {
        original_task_id: task.id.clone(),
        replacement_tasks: subtasks.iter().map(|t| task_def_to_dag_task(t)).collect(),
        reason: format!("auto-split: {:?}", reason),
    };
    executor.apply_revision(revision)?;

    // Persist updated state
    persist_executor_snapshot(&executor, &snapshot_path)?;

    // Skip dispatching the original task; subtasks will be picked up by the executor
    continue;
}
```

**Acceptance criteria**:
- Tasks with >1.5x context budget are automatically split
- Subtasks have demoted tiers
- Merge tasks inherit parent acceptance criteria
- DAG is updated correctly (no orphan tasks, no broken deps)
- Executor snapshot reflects the split
- `roko plan run` works with runtime-split tasks

---

### Phase 4: Interactive Steering and TUI

**Goal**: Users can guide the pipeline interactively through TUI and CLI commands.

**Timeline**: 2 weeks (after Phase 3)

**Files to modify**:
- `crates/roko-cli/src/tui/dashboard.rs` -- add Pack tab
- `crates/roko-cli/src/commands/pack.rs` -- add steering commands
- `crates/roko-cli/src/tui/pack_view.rs` (new) -- pack-specific TUI views

**Acceptance criteria**:
- TUI shows pack overview with pass status
- Task context inspector shows token breakdown
- Users can split, retry, skip tasks from TUI
- Approval gates pause and wait for user input
- `roko pack pause/resume` work during execution

---

### Phase 5: ACP Protocol Extension

**Goal**: Editors can drive the funnel workflow through ACP.

**Timeline**: 2 weeks (after Phase 4)

**Files to modify**:
- `crates/roko-acp/src/handler.rs` -- add pack methods
- `crates/roko-acp/src/types.rs` -- add pack types
- `crates/roko-acp/src/session.rs` -- pack session state

**Acceptance criteria**:
- `pack/create`, `pack/run_pass`, `pack/status`, `pack/approve` ACP methods work
- Progress streams as `session/update` notifications
- Editor can create packs from file selections

---

### Phase 6: Learning Integration

**Goal**: The system learns from pack execution to improve future decomposition and
context scoping.

**Timeline**: 1-2 weeks (after Phase 5)

**What to wire**:
- After a task completes: record context slice utilization (how much of the context
  the agent actually used, measured by tool calls and file reads)
- After a split: record whether the split improved success rate
- Feed utilization data back into tier budgets (adaptive windowing)
- Feed split patterns back into the auto-split heuristics

**Files to modify**:
- `crates/roko-learn/src/efficiency.rs` -- add context utilization metrics
- `crates/roko-learn/src/section_effect.rs` -- track which context sections helped
- `crates/roko-cli/src/pack_pipeline.rs` -- record learning events

**Acceptance criteria**:
- `.roko/learn/context-utilization.jsonl` records per-task context usage
- Tier budgets adapt based on historical utilization
- `roko learn context` shows utilization statistics

---

## Summary of Changes by File

| File | Phase | Change |
|------|-------|--------|
| `crates/roko-cli/src/pack.rs` | P1 | New: pack data model, CRUD |
| `crates/roko-cli/src/pack_pipeline.rs` | P1-P2 | New: pass execution (synthesis, arch, decompose, scope) |
| `crates/roko-cli/src/commands/pack.rs` | P1 | New: CLI command handlers |
| `crates/roko-cli/src/context_slicer.rs` | P2 | New: per-task context windowing |
| `crates/roko-cli/src/task_splitter.rs` | P3 | New: auto-split logic |
| `crates/roko-cli/src/main.rs` | P1 | Mod: add Pack subcommand |
| `crates/roko-cli/src/commands/mod.rs` | P1 | Mod: register pack module |
| `crates/roko-cli/src/lib.rs` | P1 | Mod: export pack module |
| `crates/roko-cli/src/orchestrate.rs` | P3 | Mod: hook splitter into dispatch |
| `crates/roko-orchestrator/src/dag.rs` | P3 | Mod: DAG mutation for splits |
| `crates/roko-cli/src/tui/dashboard.rs` | P4 | Mod: add Pack tab |
| `crates/roko-cli/src/tui/pack_view.rs` | P4 | New: pack TUI views |
| `crates/roko-acp/src/handler.rs` | P5 | Mod: add pack ACP methods |
| `crates/roko-acp/src/types.rs` | P5 | Mod: add pack types |
| `crates/roko-learn/src/efficiency.rs` | P6 | Mod: context utilization metrics |

---

## Key Design Decisions

### Why "pack" and not "project" or "session"?

A pack is a **bounded container** for a specific piece of work. It's not the whole
project (that's the workspace). It's not a session (sessions are ephemeral; packs
persist and can be resumed days later). "Pack" implies collected material with a
purpose -- you pack for a trip, you pack context for a task.

### Why on-disk files instead of database?

The pack lives in `.roko/packs/` as files because:
1. Users can inspect and edit artifacts with any tool
2. Git tracks the progression naturally
3. No new dependencies (no SQLite, no embedded DB)
4. Consistent with the rest of roko (`.roko/prd/`, `.roko/plans/`, etc.)

### Why not extend the existing PRD workflow?

The PRD workflow (`roko prd`) assumes a single document progresses through states
(idea, draft, published). The pack workflow handles a *collection* of documents
being progressively refined. PRDs are one possible input to a pack, not the whole
workflow.

### Why funnel, not pipeline?

"Pipeline" implies fixed linear stages. "Funnel" implies progressive compression --
the same material, viewed at decreasing levels of detail. The user can re-enter the
funnel at any point (edit the synthesis, re-run architecture) without invalidating
everything downstream.

### Why client-side splitting instead of asking the model?

Auto-split uses heuristics (context budget, file count) rather than asking the model
to decompose because:
1. The model might be the wrong tier (asking Haiku to decompose is circular)
2. Heuristics are deterministic and fast (no API call)
3. The learning system can tune heuristics from historical data
4. Agent-based decomposition still happens in the Decomposition pass; runtime
   splitting is a safety net for tasks the agent misjudged

When heuristics don't suffice (ScopeCreep reason), the system falls back to
agent-based decomposition using a Strategist.

---

## Open Questions

1. **Pack garbage collection**: How long do packs persist? Auto-archive after
   execution completes? User-initiated cleanup only?

2. **Cross-pack dependencies**: Can one pack's output be another pack's input?
   (Probably yes, via the source mechanism, but the manifest format needs to
   support it.)

3. **Cost tracking across passes**: The pass record tracks tokens but not cost.
   Should packs have a total cost budget that triggers halting?

4. **Parallel passes**: Can synthesis of different source groups happen in parallel?
   The current model is sequential, but map-reduce implies parallelism.

5. **MCP context during synthesis**: Should MCP servers be available during the
   synthesis pass, or only during execution? If available, the agent might fetch
   additional context that wasn't in the raw material, which changes the
   reproducibility story.
