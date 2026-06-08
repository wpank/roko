# UX Subsystem Audit

## Executive Summary

Roko has ~93K LOC of UX infrastructure across 5 surfaces -- but the CLI is not
user-friendly, the user cannot be confident things are working, context framing
is poor for smaller agents, and task TOMLs get overloaded with too much
information for the agents that consume them.

The user's actual workflow today is: **aggregate docs, make multiple passes to
funnel them into mechanical plans with gates, then execute via agents.** This is
currently done manually with Claude externally -- Roko CLI does not support this
natively. The entire UX subsystem needs to be reoriented around this
aggregation-to-funnel-to-execute pattern.

Mori had an "okay-ish" approach where task TOMLs could be ingested and it would
update everything and the DAG automatically. Roko needs to go further: the CLI
itself should be the aggregation and funneling tool, not just the executor.

---

## Current UX Pain Points (User-Reported)

### Pain Point 1: CLI Is Not User-Friendly

The CLI has 60+ subcommands across 16 command groups. A new user (or even the
primary user) cannot discover what to do next. The help text is a wall of
commands with no guided workflow.

**Evidence from `main.rs`:**
- The `Command` enum has 30+ top-level variants
- Many take nested subcommands (PlanCmd, PrdCmd, AgentCmd, etc.)
- The `after_long_help` text is a flat list of command groups with no
  workflow guidance
- No "what should I do next?" affordance
- No interactive mode that guides through the aggregate-funnel-execute
  workflow

**What the user actually needs:**
```
$ roko
# Should detect workspace state and suggest next action:
#   "You have 3 PRDs in draft. Run `roko prd plan <slug>` to generate plans."
#   "You have 2 plans ready. Run `roko plan run plans/` to execute."
#   "Last run had 2 gate failures. Run `roko resume` to retry."
```

**Specific CLI usability gaps:**
1. No workflow-aware prompting -- bare `roko` drops into an interactive REPL
   with no guidance about what the workspace needs
2. No `roko next` command that inspects state and recommends the next step
3. No `roko ingest <docs>` command to start the aggregation phase
4. Five separate chat modes (`chat_inline.rs`, `chat.rs`, `run.rs`,
   `unified.rs`, `dispatch_direct.rs`) with different behaviors -- the user
   cannot predict which mode they are in
5. Error messages from agent dispatch are opaque -- "agent failed" with no
   guidance on what to check or retry
6. Plan/task output is flat text tables with no progress indication during
   long-running operations

### Pain Point 2: Cannot Be Sure Things Work

The user cannot determine whether a completed operation actually succeeded or
whether the system is in a consistent state.

**Evidence:**
- `roko plan run` produces output but does not clearly indicate whether
  all gates passed, which tasks succeeded vs. failed, or what the aggregate
  state is after the run
- `roko status` shows signal counts and episode counts but does not tell you
  "your last run passed 8/10 tasks, 2 need retry, here is what failed"
- Gate results are logged to `.roko/episodes.jsonl` but there is no CLI
  command to query them in a human-friendly way
- The TUI dashboard (F1) shows plan progress but only when the TUI is running;
  there is no equivalent for non-interactive CLI users
- No "dry run" mode that shows what would be executed without actually running
- No diff preview before plan execution

**What the user needs:**
```
$ roko status --last-run
Last run: 2026-04-29T14:23:05 (plan: self-hosting-sprint)
  Tasks: 8/10 completed
  Failed: T6 (gate: clippy), T9 (gate: test)
  Cost: $4.23 | Duration: 12m
  Retry: `roko resume` to retry failed tasks

$ roko plan run plans/ --dry-run
Would execute 10 tasks across 3 waves:
  Wave 0: T1, T2 (parallel, no deps)
  Wave 1: T3, T4, T5 (parallel, depend on T1)
  Wave 2: T6-T10 (sequential, exclusive files)
  Estimated cost: ~$5.00 | Estimated time: ~15min
  Proceed? [y/n]
```

### Pain Point 3: Context Framing Is Poor

The system prompt assembly (9-layer `SystemPromptBuilder`) produces prompts
that are comprehensive but not well-targeted. Context framing does not adapt
to what the agent actually needs for its specific task.

**Evidence from `orchestrate.rs`:**
- The prompt assembly path (`build_system_prompt_with_context_validated`) builds
  a monolithic prompt with up to 9 layers
- Context files are included based on task metadata (`context_files` field) but
  there is no intelligence about which parts of those files are relevant
- For tasks touching a single function, the agent receives the entire file plus
  multiple context files -- most of which are irrelevant
- The `effective_context_window_tokens` function caps the window but does not
  prioritize content within that window
- No progressive disclosure: a simple "fix this typo" task gets the same
  context depth as a "redesign the auth system" task

**What the user needs:**
- Context that scales with task complexity: slim for trivial tasks, deep for
  architectural tasks
- Intelligent file chunking: only include the relevant sections of large files
- Research context integrated at task level, not just at plan level
- Prior task output feeding into subsequent task context (chain-of-work)

### Pain Point 4: Task TOMLs Get Overloaded

When the user (or an agent) generates task TOMLs, each task accumulates too
much metadata. Smaller, faster agents (sonnet, haiku) receive tasks with 20+
fields of metadata that they do not need and cannot effectively use. The
cognitive overhead degrades output quality for simpler tasks.

**Evidence from Mori's tasks.toml format:**
```toml
[[task]]
id = "T1"
title = "..."
status = "pending"
files = [...]
depends_on = []
parallel_group = "A"
exclusive_files = true
category = "scaffolding"
reasoning_level = "low"
speed_priority = "latency"
quality_profile = "pragmatic"
context_weight = "slim"
preferred_model = "claude-opus-4-6"
preferred_provider = "codex"
estimated_minutes = 5
complexity_band = "fast"
escalate_on_retry = false
tags = [...]
acceptance = [...]
context_files = [...]
example_pattern = "..."
```

This is 20+ fields. A haiku-class agent doing a simple scaffolding task does
not benefit from `reasoning_level`, `speed_priority`, `quality_profile`,
`context_weight`, or `example_pattern`. Those fields add noise and consume
context window tokens.

**What the user needs:**
- Task complexity tiers: "fast" tasks get 5-6 fields, "standard" tasks get
  12-15 fields, "complex" tasks get the full 20+ fields
- Smart context windowing: the dispatcher strips metadata fields that the
  target model cannot use
- Task TOML validation that warns when a fast/simple task has too many fields
- Task splitting: a complex task with 8+ files should be auto-split into
  subtasks that each touch 2-3 files

---

## The User's Actual Workflow (Not Supported Today)

The user's workflow has three distinct phases. Roko CLI should support each
one natively instead of forcing the user to do them manually with external
Claude sessions.

### Phase 1: Aggregation

The user gathers documents, research, code analysis, and requirements into a
coherent corpus. Today this is done by:
1. Reading relevant source files manually
2. Reading PRDs and prior research
3. Reading architecture docs and design decisions
4. Collecting all of this into a single Claude conversation

**What Roko should do:**
```
$ roko ingest docs/architecture.md docs/requirements.md crates/roko-core/src/
# Reads all inputs, builds a structured context corpus
# Stores in .roko/context/corpus-<hash>.json
# Shows: "Ingested 3 docs + 47 source files (152K tokens)"

$ roko ingest --from-prd system-prompt-wiring
# Reads the PRD, extracts referenced files, builds corpus automatically
```

### Phase 2: Funneling (Multiple Passes)

The user makes multiple passes over the aggregated corpus to progressively
refine it into mechanical, executable plans with clear gates. Each pass
narrows scope and increases specificity.

Pass 1: Architecture analysis -- understand the system, identify components
Pass 2: Gap analysis -- what exists vs. what needs to change
Pass 3: Task decomposition -- break into atomic, gate-checkable work units
Pass 4: Dependency analysis -- ordering, parallelism, file conflicts
Pass 5: Gate specification -- what must be true for each task to "pass"

Today the user does all 5 passes manually in a Claude conversation, then
copies the resulting tasks.toml into the Roko plans directory.

**What Roko should do:**
```
$ roko funnel --corpus corpus-<hash> --pass architecture
# Agent analyzes the corpus and produces an architecture summary
# Shows findings, asks for approval before continuing

$ roko funnel --corpus corpus-<hash> --pass gaps
# Agent compares architecture to requirements, identifies gaps
# Shows gap list, user confirms/edits

$ roko funnel --corpus corpus-<hash> --pass tasks
# Agent generates tasks.toml from the gaps
# Shows each task with brief rationale
# User can edit, split, reorder, approve

$ roko funnel --corpus corpus-<hash> --pass deps
# Agent analyzes file overlaps and import chains
# Suggests parallel groups and dependency ordering
# User confirms

$ roko funnel --corpus corpus-<hash> --pass gates
# Agent generates acceptance criteria and gate checkpoints
# Each task gets testable conditions
# User reviews and approves

# Or: all passes in sequence with checkpoints
$ roko funnel --corpus corpus-<hash> --all
```

### Phase 3: Execution

The user runs the plan. This phase is what Roko already does (mostly) with
`roko plan run`. But even here, the user needs more control:

- Pre-flight check: validate the plan before starting
- Progress monitoring without the TUI (for SSH sessions, CI, etc.)
- Per-task approval gates (for sensitive changes)
- Pause/resume at arbitrary points
- Cost cap enforcement

**What Roko should do:**
```
$ roko plan validate plans/
# Checks: all files exist, no circular deps, parallel groups consistent,
# acceptance criteria are testable shell commands, total estimated cost
# within budget

$ roko plan run plans/ --approve-each
# Pauses after each task for user approval before continuing

$ roko plan run plans/ --cost-cap 20.00
# Stops execution if total cost exceeds $20
```

---

## Current UX Surfaces (Updated Assessment)

### 1. TUI Dashboard (ratatui)

**Location:** `crates/roko-cli/src/tui/` (~42K LOC)
**Tabs:** F1-F10 (Dashboard, Plans, Agents, Git, Logs, Config, Inspect,
  Marketplace, Atelier, Learning)
**Status:** Wired but not usable as the primary workflow surface. The TUI is
an observation tool, not a workflow driver. You cannot aggregate, funnel, or
approve from the TUI -- you can only watch.

**Key gap:** The TUI should be the primary surface for the funnel phase,
showing each pass's output and allowing the user to approve/reject/edit
inline. Currently it is read-only for plan state.

### 2. Inline Chat Modes (5 separate implementations)

**Anti-pattern:** 5 separate chat event loops with zero shared rendering.

| Mode | Entry | LOC | Gap |
|---|---|---|---|
| Unified REPL | `roko` | 4,100 | No workflow guidance, no funnel support |
| One-shot | `roko "prompt"` | ~30 | No progress, no cost tracking |
| Agent chat | `roko chat` | 658 | Line-oriented, no rich output |
| Universal loop | `roko run` | 1,555 | No multi-pass, single prompt only |
| Dashboard | `roko dashboard` | (TUI) | Read-only observation |

**What should exist:** A single `ChatSession` that handles dispatch, streaming,
tool output rendering, and cost tracking. Each mode instantiates it with
different config. The funnel workflow should be a mode of this unified session.

### 3. HTTP Control Plane (~85 routes)

**Location:** `crates/roko-serve/src/routes/`
**Status:** Wired. Good API surface for programmatic access. But does not
expose the funnel workflow -- there are no endpoints for corpus management,
multi-pass refinement, or progressive context assembly.

### 4. ACP Protocol (Editor Integration)

**Location:** `crates/roko-acp/src/` (~7K LOC)
**Status:** Pure FSM over stdio JSON-RPC. Serial agents only. No task
awareness, no funnel support, no DAG visibility.

### 5. Demo Web/App

**Location:** `demo/demo-web/` + `demo/demo-app/`
**Status:** Demo-only. Not a production surface. Embedded in roko-serve
binary via rust-embed.

---

## Gap Analysis: What Matters Most

Ranked by impact on the user's actual workflow:

| Gap | Impact | Phase Affected | Effort |
|---|---|---|---|
| No aggregation/ingest command | Critical | Aggregation | Medium |
| No multi-pass funnel workflow | Critical | Funneling | Large |
| No task TOML validation | High | Funneling | Small |
| No smart context windowing | High | Execution | Medium |
| No task auto-splitting | High | Funneling/Exec | Medium |
| No `roko next` guidance | High | All | Small |
| No dry-run mode | Medium | Execution | Small |
| No per-task cost tracking | Medium | Execution | Small |
| No unified chat session | Medium | All | Medium |
| 5 chat modes, 0 shared code | Low-Med | Maintenance | Large |
| No board/kanban view | Low | Observation | Large |
| No cross-plan task DAG | Low | Execution | Large |

The top 5 gaps all relate to the aggregation-funnel-execute workflow.
Fixing the board view or unifying chat modes is less important than making
the core workflow native to the CLI.

---

## Anti-Patterns Specific to UX

### AP-UX1: Five chat modes, zero shared rendering

`chat_inline.rs`, `chat.rs`, `run.rs`, `unified.rs`, `dispatch_direct.rs`
each have independent event loops. Adding a feature requires changing 5 places.

### AP-UX2: Inline primitives built but mostly unwired

11 inline primitives in `inline/primitives/` (~2.5K LOC). Only `CostMeter`,
`StreamingState`, `GateBlockData`, and `RunBlockData` are used in live paths.
The rest exist only in `bench_demo.rs`.

### AP-UX3: TUI and web share no data model

TUI uses `TuiState` (4,968 LOC). Web uses React state from `hooks/useApi.ts`.
Serve uses `AppState`. No unified ViewModel.

### AP-UX4: Right-rail panels designed but not implemented

6 right-rail panels designed in the v2 ACP showcase. None are wired.

### AP-UX5: No task lifecycle UX

No UX for: creating tasks interactively, viewing task details with full
metadata, editing dependencies, visualizing the DAG, drag-reordering, or
batch operations.

### AP-UX6: Observation without action

The TUI dashboard is read-only. You can watch execution but cannot:
- Pause/resume from the dashboard
- Approve/reject a task result
- Edit a task's context or acceptance criteria
- Trigger a single task re-run
- Split a failing task into smaller pieces

This makes the dashboard a passive monitor, not a workflow tool.

### AP-UX7: No feedback loop visibility

When the learning subsystem adjusts model routing or gate thresholds, the
user has no way to see why a particular model was chosen for a task, or
why a gate threshold changed. The cascade router and adaptive thresholds
operate invisibly. The F10/Learning tab shows aggregate stats but not
per-decision explanations.

---

## Subsystem Interactions

```
User's Workflow:
  Aggregate ──> Funnel (multi-pass) ──> Execute ──> Review

             Today: manual + Claude         Today: roko plan run
             Roko should do all of this natively

Roko Subsystems Involved:

  [roko-cli]     CLI entry, chat modes, commands
       |
  [roko-compose] Prompt assembly, context bidders, enrichment
       |
  [roko-orchestrator] DAG computation, parallel executor
       |
  [roko-agent]   Agent dispatch, model routing, tool loop
       |
  [roko-gate]    Gate pipeline, verification
       |
  [roko-learn]   Episodes, playbooks, routing, experiments
       |
  [roko-neuro]   Knowledge store, context for enrichment
```

### Key Integration Points for the New Workflow

| New Feature | Depends On | Current State |
|---|---|---|
| `roko ingest` | roko-compose (corpus), roko-neuro | Not built |
| `roko funnel` | roko-agent (multi-pass), roko-compose | Not built |
| Task TOML validation | roko-orchestrator (plan types) | Not built |
| Smart context windowing | roko-compose (prompt builder) | Partial |
| Task auto-splitting | roko-orchestrator (DAG) | Not built |
| `roko next` | roko-cli (state inspection) | Not built |
| Dry-run mode | roko-orchestrator (executor) | Not built |
| Per-task approval | roko-runtime (event bus) | Not built |

---

## LOC Breakdown

| Component | LOC | Notes |
|---|---|---|
| TUI (fullscreen) | ~41,924 | 10 tabs, 21 widgets, 14 modals |
| Inline chat modes | ~6,921 | 5 modes with zero shared rendering |
| Inline primitives | ~2,506 | 11 components, mostly unwired |
| Inline support | ~1,475 | agent_events, markdown, styled, terminal |
| Serve routes | ~23,874 | All routes, not just UX-facing |
| Demo web (HTML) | ~7,299 | 7 standalone HTML files |
| Demo app (React) | ~1,859 | 18 .tsx files |
| ACP (editor UX) | ~6,963 | JSON-RPC + FSM |
| **Total UX-related** | **~92,821** | |

---

## Sources

Key source files verified during this audit:

- `crates/roko-cli/src/main.rs` -- CLI entry point, 30+ Command variants, 16 command groups
- `crates/roko-cli/src/chat_session.rs` -- ChatAgentSession, slash commands, tool call accumulation
- `crates/roko-cli/src/orchestrate.rs` -- PlanRunner, gate pipeline, prompt assembly, dispatch
- `crates/roko-cli/src/commands/plan.rs` -- plan list/show/create/run/validate handlers
- `crates/roko-cli/src/commands/prd.rs` -- PRD lifecycle commands
- `crates/roko-cli/src/chat_inline.rs` (4,100 LOC) -- Unified REPL
- `crates/roko-cli/src/chat.rs` (658 LOC), `run.rs` (1,555 LOC), `unified.rs` (203 LOC), `dispatch_direct.rs` (405 LOC)
- `crates/roko-acp/src/pipeline.rs` -- PipelinePhase enum (10 variants)
- `crates/roko-acp/src/session.rs` -- ACP session state management
- `crates/roko-acp/src/types.rs` -- JSON-RPC types, ACP protocol version
- `crates/roko-compose/src/system_prompt_builder.rs` -- 9-layer prompt assembly
- `crates/roko-serve/src/routes/` -- ~85 routes across 23 route files
- `crates/roko-orchestrator/src/` -- DAG computation, parallel executor, resource budget
- `crates/roko-learn/` -- episodes, playbooks, cascade router, experiments
