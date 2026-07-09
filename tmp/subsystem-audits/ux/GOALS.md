# UX Goals

## Vision

Roko CLI should be the native tool for the user's actual workflow:
**aggregate context, funnel it through multiple refinement passes into
mechanical plans with gates, and execute via agents.** Today this workflow
is done manually with external Claude sessions. The CLI should own all three
phases.

The north star is not "Linear meets an agent orchestrator" -- it is **"the
tool I use instead of manually pasting context into Claude and then copying
task TOMLs out."**

---

## Design Principles

### DP-1: Workflow-First, Not Feature-First

The CLI should guide the user through the aggregate-funnel-execute workflow.
Every command should answer: "What phase of the workflow am I in, and what
should I do next?"

Bad: 60+ subcommands with no guidance.
Good: `roko next` tells you what to do. `roko` with no args shows workflow
state and suggests the next action.

### DP-2: Progressive Context Refinement

Context should narrow as the workflow progresses:

1. **Aggregate:** Broad -- ingest everything relevant
2. **Architecture pass:** Narrow to structure and components
3. **Gap pass:** Narrow to deltas between current and desired state
4. **Task pass:** Narrow to atomic work units
5. **Gate pass:** Narrow to testable acceptance criteria
6. **Execute:** Each task gets only its relevant context slice

At no point should an agent receive the entire aggregated corpus. Each pass
produces a distilled artifact that feeds the next pass.

### DP-3: Right-Sized Context for Right-Sized Models

Not all tasks need Opus. Not all tasks need the full context window.

| Task Complexity | Model Tier | Context Budget | Metadata Fields |
|---|---|---|---|
| Trivial (fix typo) | Haiku/Flash | 4K tokens | 5-6 fields |
| Fast (scaffold) | Sonnet | 16K tokens | 8-10 fields |
| Standard (implement) | Sonnet/Opus | 32K tokens | 12-15 fields |
| Complex (architect) | Opus | 64K+ tokens | 20+ fields |

The dispatcher should automatically scale context and metadata based on the
task's complexity band. A haiku agent doing `category = "scaffolding"` with
`complexity_band = "fast"` should get a slim prompt with only the essential
fields -- not the 20+ field monster that Mori's tasks.toml produced.

### DP-4: Confidence Through Visibility

The user should never wonder "did that work?" Every operation should produce
a clear verdict:

- `roko plan run` ends with a summary: 8/10 passed, 2 failed, here is why
- `roko status` shows the workspace health at a glance
- Failed tasks include the specific gate output, not just "failed"
- Cost is tracked per-task and shown inline
- `roko next` incorporates failure state: "2 tasks need retry"

### DP-5: Task TOML as Executable Specification

A task TOML should be a complete, self-contained specification that an agent
can execute without additional context beyond the files it references. This
means:

1. The `acceptance` field contains testable shell commands, not prose
2. The `files` field lists exactly the files to create or modify
3. The `context_files` field provides patterns to reference
4. The `depends_on` field ensures prior work is available

But the task should NOT contain routing metadata (model, provider, quality
profile) that the agent does not need to see. Routing metadata is for the
dispatcher, not the agent.

### DP-6: Mori's "Okay-ish" Approach, Improved

Mori's pattern was: you write a tasks.toml, ingest it with `mori plan ingest`,
and the system updates the DAG, schedules execution, and tracks progress. The
user could also edit the tasks.toml and re-ingest to update the plan mid-flight.

**What worked in Mori:**
- Task TOML as the canonical artifact
- Ingest-and-update loop (edit -> re-ingest -> DAG updates)
- Rich task metadata enabling intelligent routing
- Wave-based parallel execution from the DAG

**What was "okay-ish" in Mori:**
- The user still had to write the tasks.toml manually (or have Claude do it)
- No aggregation phase -- you started with tasks
- No funneling -- you jumped straight from idea to implementation plan
- No validation -- malformed tasks.toml caused runtime errors
- Task metadata was all-or-nothing: every task got 20+ fields

**What Roko should improve:**
- The CLI should help create the tasks.toml (funnel workflow)
- Tasks should be validated before execution
- Metadata should be tiered by complexity
- Context should be assembled per-task, not monolithically
- The ingest-and-update loop should survive mid-run edits

---

## Desired End State

### The Full Workflow in Roko

```bash
# Phase 1: Aggregate
$ roko ingest docs/requirements.md crates/roko-core/src/ --label sprint-42
# Reads all inputs, builds structured corpus
# Output: .roko/corpora/sprint-42.json (152K tokens indexed)

# Phase 2: Funnel
$ roko funnel sprint-42
# Interactive multi-pass refinement:
#   Pass 1: Architecture analysis -> shows component map, asks for approval
#   Pass 2: Gap analysis -> shows what needs to change, asks for approval
#   Pass 3: Task decomposition -> generates tasks, asks for approval
#   Pass 4: Dependency analysis -> orders tasks, identifies parallelism
#   Pass 5: Gate specification -> adds acceptance criteria
# Each pass saves a checkpoint. User can resume from any pass.
# Output: plans/sprint-42/tasks.toml

# Phase 2b: Validate and tune
$ roko plan validate plans/sprint-42/
# Checks: circular deps, file conflicts, acceptance criteria format,
# estimated cost/time, complexity tier consistency
# Output: 10 tasks validated, 2 warnings (T6 has no acceptance criteria)

$ roko plan tune plans/sprint-42/
# Adjusts task metadata based on complexity band:
#   - Strips routing metadata from fast/trivial tasks
#   - Adds context_files suggestions based on import analysis
#   - Sets parallel_group based on file conflict analysis
#   - Estimates time based on similar past tasks (playbook recall)

# Phase 3: Execute
$ roko plan run plans/sprint-42/ --dry-run
# Shows execution plan without running:
#   Wave 0: T1, T2 (parallel, no deps) -- est. 5min, ~$1.00
#   Wave 1: T3-T5 (parallel, depend on T1) -- est. 12min, ~$3.00
#   Wave 2: T6-T10 (mixed) -- est. 20min, ~$5.00
#   Total: ~37min, ~$9.00

$ roko plan run plans/sprint-42/
# Executes with live progress:
#   [Wave 0] T1 implementing... T2 implementing...
#   [Wave 0] T1 done (gate: pass) | T2 done (gate: pass)
#   [Wave 1] T3 implementing... T4 implementing... T5 implementing...
#   ...
# Final summary:
#   8/10 passed | 2 failed (T6: clippy, T9: test)
#   Cost: $8.47 | Duration: 34min
#   Resume failed: `roko resume sprint-42`

# Phase 4: Iterate
$ roko resume sprint-42
# Retries failed tasks with gate failure context injected
```

### CLI State Awareness

When the user runs `roko` with no arguments, the CLI should inspect the
workspace and show contextual guidance:

```
$ roko
roko v0.1.0 | workspace: /Users/will/dev/nunchi/roko/roko

Current state:
  Corpora: 2 (sprint-42: 152K tokens, sprint-41: 89K tokens)
  Plans: 3 discovered
    sprint-42: 8/10 tasks done (2 failed)
    sprint-41: complete
    refactor-01: pending (never run)
  Learning: 247 episodes, 12 playbooks, 92% routing accuracy

Suggested next:
  roko resume sprint-42       Retry 2 failed tasks from sprint-42
  roko plan run refactor-01   Execute pending plan

[Enter interactive mode? y/n]
```

### Smart Context Windowing

Each task gets a context budget proportional to its complexity. The
`SystemPromptBuilder` should enforce this:

```
Task: T1 (complexity_band: fast, context_weight: slim)
  Context budget: 4,096 tokens
  Included: task description, acceptance criteria, target files
  Excluded: architecture overview, prior task outputs, research context

Task: T7 (complexity_band: complex, context_weight: deep)
  Context budget: 65,536 tokens
  Included: task description, acceptance criteria, target files,
    architecture overview, related prior task outputs, research context,
    playbook recall, neuro knowledge hits
```

The context bidders (AttentionBidder variants in orchestrate.rs) already
exist. They need to be constrained by the task's `context_weight` field
so that slim tasks get slim context.

### Task TOML Tiering

Tasks should have different levels of metadata based on complexity:

**Tier 1: Trivial (fix, typo, rename)**
```toml
[[task]]
id = "T1"
title = "Fix typo in README.md"
status = "pending"
files = ["README.md"]
acceptance = ["grep -q 'correct spelling' README.md"]
```

**Tier 2: Fast (scaffold, stub, config)**
```toml
[[task]]
id = "T2"
title = "Scaffold widget module"
status = "pending"
files = ["src/widgets/mod.rs", "src/widgets/button.rs"]
depends_on = []
category = "scaffolding"
complexity_band = "fast"
acceptance = ["cargo check -p my-crate"]
context_files = ["src/widgets/existing_widget.rs"]
```

**Tier 3: Standard (implement, integrate)**
```toml
[[task]]
id = "T3"
title = "Implement user authentication flow"
status = "pending"
files = ["src/auth/login.rs", "src/auth/session.rs", "src/auth/middleware.rs"]
depends_on = ["T1", "T2"]
parallel_group = "B"
exclusive_files = true
category = "implementation"
complexity_band = "standard"
estimated_minutes = 15
tags = ["auth", "security"]
acceptance = [
  "cargo test -p my-crate auth",
  "cargo clippy -p my-crate -- -D warnings",
]
context_files = ["src/auth/types.rs", "docs/auth-design.md"]
```

**Tier 4: Complex (architect, redesign)**
```toml
[[task]]
id = "T4"
title = "Redesign the caching layer for distributed consistency"
status = "pending"
files = ["src/cache/distributed.rs", "src/cache/invalidation.rs", ...]
depends_on = ["T1", "T2", "T3"]
parallel_group = "C"
exclusive_files = true
category = "implementation"
reasoning_level = "high"
speed_priority = "accuracy"
quality_profile = "hardened"
context_weight = "deep"
complexity_band = "complex"
estimated_minutes = 45
escalate_on_retry = true
tags = ["cache", "distributed", "architecture"]
acceptance = [
  "cargo test -p my-crate cache -- --include-ignored",
  "cargo clippy -p my-crate -- -D warnings",
  "test -f docs/cache-architecture.md",
]
context_files = ["docs/distributed-cache-design.md", "src/cache/local.rs"]
example_pattern = "src/cache/local.rs"
```

The dispatcher reads `complexity_band` and strips irrelevant fields before
assembling the agent prompt. A haiku agent running T1 never sees
`reasoning_level`, `speed_priority`, or `quality_profile`.

---

## Feature Requirements (Workflow-Aligned)

### P0: Workflow Foundation

| Feature | Description | Phase |
|---|---|---|
| `roko next` | Inspect workspace state, suggest next action | All |
| `roko ingest` | Aggregate docs/code into a corpus | Aggregate |
| `roko funnel` | Multi-pass refinement of corpus into plan | Funnel |
| `roko plan validate` | Check plan for errors before execution | Funnel |
| Smart context windowing | Scale context to task complexity | Execute |
| Task metadata tiering | Strip irrelevant fields for simple tasks | Execute |
| Run summary | Clear pass/fail summary after `plan run` | Execute |

### P1: Execution Quality

| Feature | Description | Phase |
|---|---|---|
| `--dry-run` mode | Show execution plan without running | Execute |
| Per-task cost tracking | Show cost inline during execution | Execute |
| `--cost-cap` flag | Stop execution at budget limit | Execute |
| `--approve-each` flag | Pause for approval between tasks | Execute |
| Task auto-splitting | Split complex tasks into subtasks | Funnel |
| Gate failure context | Inject failure details into retry prompt | Execute |

### P2: Observation and Confidence

| Feature | Description | Phase |
|---|---|---|
| `roko status --last-run` | Detailed last-run summary | Review |
| Decision explanations | Show why a model was chosen, why a gate threshold changed | Execute |
| Playbook recall display | Show when a playbook is matched and applied | Execute |
| Progress output (non-TUI) | Inline progress for headless/SSH | Execute |

### P3: Advanced Workflow

| Feature | Description | Phase |
|---|---|---|
| Funnel checkpoints | Resume funnel from any pass | Funnel |
| Corpus caching | Reuse corpus across funnel runs | Aggregate |
| Cross-plan task DAG | Dependencies across plans | Execute |
| Board view (TUI) | Kanban-style task management | Observe |
| Web frontend | React app for board/task management | Observe |

---

## Key Properties

### KP-1: Workflow Continuity

The user should be able to stop at any point in the aggregate-funnel-execute
workflow and resume later. Every phase produces a persistent artifact:
- Aggregate: corpus JSON
- Funnel: per-pass checkpoint + final tasks.toml
- Execute: executor snapshot + episode log

### KP-2: Right-Sized Everything

Context, metadata, model, and cost should all scale with task complexity.
A $0.02 haiku task should not consume the same resources as a $2.00 opus task.

### KP-3: Confidence

After any operation, the user should know: did it work, what happened, and
what to do next. No ambiguous output. No silent failures.

### KP-4: Editability

The user should be able to edit artifacts (corpus, tasks.toml, acceptance
criteria) at any point and re-run from that point. The system should detect
edits and re-validate.

### KP-5: Composability

Each phase of the workflow should work independently. `roko plan run` should
work without `roko ingest` or `roko funnel` -- the user can bring their own
tasks.toml. But when used together, the phases should feed each other
seamlessly.

---

## Sources

Infrastructure that exists today and informs these goals:

- `crates/roko-cli/src/main.rs` -- CLI entry point, Command enum with 30+ variants
- `crates/roko-cli/src/orchestrate.rs` -- PlanRunner, gate pipeline, prompt assembly, state persistence
- `crates/roko-cli/src/chat_session.rs` -- ChatAgentSession, slash commands
- `crates/roko-cli/src/commands/plan.rs` -- plan list/show/create/run/validate handlers
- `crates/roko-cli/src/commands/prd.rs` -- PRD lifecycle (idea/draft/plan)
- `crates/roko-compose/src/system_prompt_builder.rs` -- 9-layer prompt assembly
- `crates/roko-compose/src/enrichment.rs` -- EnrichmentPipeline, StepSelector
- `crates/roko-orchestrator/src/` -- DAG computation, ParallelExecutor, ExecutorAction
- `crates/roko-learn/` -- episodes, playbooks, cascade router, efficiency events
- `crates/roko-neuro/` -- knowledge store, context assembly
- `crates/roko-gate/` -- 7-rung gate pipeline, adaptive thresholds
