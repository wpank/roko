# UX Issues & Blockers

## Critical Issues

### I-UX01: CLI is not user-friendly -- no workflow guidance

**Severity:** Critical
**Affects:** All users, all phases

The CLI has 60+ subcommands across 16 command groups. Running `roko` with no
arguments drops into a REPL with no guidance. Running `roko --help` produces
a wall of text. There is no way to discover the recommended workflow.

**Evidence from `main.rs`:**
- `Command` enum: 30+ top-level variants (Init, Run, Status, Doctor,
  LayerCheck, Plan, Prd, Agent, Research, Knowledge, Learn, Job, Bench,
  Demo, Config, Index, Up, Serve, Acp, Daemon, Deploy, Worker, Dashboard,
  Login, Logout, Whoami, VisionLoop, Resume, Replay, History, Inject,
  Completions, New, Explain, Chat, Share)
- The `after_long_help` text groups commands but provides no workflow
  guidance: "Core workflow: init, run, status, doctor" does not tell the
  user what to do first
- No `roko next` or `roko what` command
- No workspace state inspection on startup

**User impact:** The user reports the CLI is not user-friendly. They cannot
discover what to do next. They resort to manually running Claude externally
and then copying artifacts into the Roko workspace.

**Root cause:** Commands were added organically as features were built. No
one designed the workflow-level UX. Each command works in isolation but they
do not compose into a guided experience.

**Resolution plan:**

1. Add `roko next` (or `roko what`) command:
   - Inspect workspace state: `.roko/` existence, PRDs, plans, executor
     snapshots, last run results, failed tasks
   - Output a prioritized list of suggested actions with full commands
   - File: new `crates/roko-cli/src/commands/next.rs`
   - Effort: Small (~200 LOC)

2. Make bare `roko` invoke `roko next` instead of the REPL:
   - Currently `unified.rs` handles bare invocation and starts the REPL
   - Change to: show `roko next` output, then offer REPL as an option
   - File: `crates/roko-cli/src/unified.rs` (modify), `main.rs` (modify)
   - Effort: Small (~50 LOC change)

3. Add workflow-aware help text:
   - Replace flat command listing with a guided workflow:
     "Step 1: roko init -- Set up workspace"
     "Step 2: roko ingest -- Aggregate context"
     "Step 3: roko funnel -- Refine into plan"
     "Step 4: roko plan run -- Execute"
   - File: `main.rs` `after_long_help`
   - Effort: Trivial

---

### I-UX02: Cannot be sure things work -- no clear verdicts

**Severity:** Critical
**Affects:** Execution phase, review phase

After `roko plan run` completes, the user cannot determine the aggregate
outcome without reading log files. The output streams agent text but does not
produce a clear summary.

**Evidence from `orchestrate.rs`:**
- The PlanRunner loop processes tasks sequentially/in parallel and logs
  results to `.roko/episodes.jsonl`
- Gate results are printed inline but scroll off screen during long runs
- No end-of-run summary is printed
- `roko status` shows signal/episode counts but not "your last run passed
  8/10 tasks"
- The `roko status --cfactor` flag computes C-Factor but not a human-readable
  run summary

**Evidence from `commands/plan.rs`:**
- `plan list` shows a table with ID, TITLE, PROGRESS, STATUS columns
- Progress is "done/total" from executor state -- but only if state exists
- No distinction between "never run" and "run and all passed" vs "run and
  some failed"

**User impact:** The user reports they "can't be sure things work." After a
run, they manually check `.roko/episodes.jsonl` and executor snapshots to
determine what happened. This is the opposite of user-friendly.

**Resolution plan:**

1. Add end-of-run summary to PlanRunner:
   - After all tasks complete (or on Ctrl-C), print:
     ```
     Run complete: sprint-42
       Passed: 8/10 tasks
       Failed: T6 (gate: clippy), T9 (gate: test)
       Skipped: 0
       Cost: $8.47 | Duration: 34min
       Resume: roko resume sprint-42
     ```
   - File: `crates/roko-cli/src/orchestrate.rs` (add summary function)
   - Effort: Medium (~300 LOC)

2. Add `roko status --last-run`:
   - Read the most recent executor snapshot
   - Show per-task pass/fail with gate output excerpts
   - Show cost breakdown and timing
   - File: `crates/roko-cli/src/commands/status.rs` (extend)
   - Effort: Medium (~200 LOC)

3. Add `--dry-run` flag to `roko plan run`:
   - Compute DAG, show execution waves, estimate cost/time
   - Do not dispatch any agents
   - File: `crates/roko-cli/src/orchestrate.rs` (modify)
   - Effort: Small (~150 LOC)

---

### I-UX03: Context framing is poor -- monolithic prompt assembly

**Severity:** Critical
**Affects:** Execution quality, cost efficiency

The SystemPromptBuilder produces prompts that include all 9 layers regardless
of task complexity. A trivial scaffolding task gets the same prompt structure
as a complex architectural redesign. This wastes context window tokens and
can confuse smaller models.

**Evidence from `orchestrate.rs`:**
- `build_system_prompt_with_context_validated` calls into the 9-layer
  SystemPromptBuilder with all context bidders active
- The `effective_context_window_tokens` function caps the total window but
  does not differentially allocate between layers
- Context bidders (`AttentionBidder` variants: Neuro, Task, Research) all
  compete for the same budget without considering task complexity
- A `complexity_band = "fast"` task gets research context and neuro
  knowledge hits that it does not need

**Evidence from `crates/roko-compose/src/system_prompt_builder.rs`:**
- The builder assembles sections: identity, role, conventions, task,
  context, tools, constraints, memory, meta
- No per-section budget that adapts to task complexity
- No "skip this section if task is simple" logic

**User impact:** Tasks assigned to smaller/faster models receive bloated
prompts. The agent either ignores the excess context (wasting tokens/money)
or gets confused by irrelevant instructions (degrading quality).

**Resolution plan:**

1. Add complexity-aware section budgets to SystemPromptBuilder:
   - Map `context_weight` to per-section token budgets:
     - `slim`: identity + role + task + constraints only (4K total)
     - `standard`: all sections, moderate budgets (16-32K total)
     - `deep`: all sections, generous budgets (64K+ total)
   - File: `crates/roko-compose/src/system_prompt_builder.rs` (modify)
   - Effort: Medium (~200 LOC)

2. Add section pruning for fast tasks:
   - When `complexity_band = "fast"`, skip: memory, meta, research context,
     neuro knowledge, prior task outputs
   - Keep: identity, role, task description, acceptance criteria, target
     files, constraints
   - File: `crates/roko-cli/src/dispatch_helpers.rs` (modify)
   - Effort: Small (~100 LOC)

3. Add intelligent file chunking:
   - When including a context file, extract only the relevant section
     (function, struct, impl block) rather than the whole file
   - Use `roko-index` symbol information when available
   - File: `crates/roko-compose/src/enrichment.rs` (extend)
   - Effort: Medium (~400 LOC)

---

### I-UX04: Task TOMLs get overloaded for smaller agents

**Severity:** Critical
**Affects:** Execution quality, small-model performance

Task TOMLs from Mori's format carry 20+ metadata fields per task. When the
dispatcher assembles the prompt, all these fields are serialized into the
agent's context. A haiku-class agent working on a trivial task receives:
`reasoning_level`, `speed_priority`, `quality_profile`, `context_weight`,
`preferred_model`, `preferred_provider`, `complexity_band`,
`escalate_on_retry`, `example_pattern` -- none of which it needs or can use.

**Evidence from Mori's tasks.toml format (MORI-REFERENCE.md):**
- Every task has 20+ fields regardless of complexity
- Fields like `reasoning_level = "low"` are routing hints for the dispatcher,
  not instructions for the agent
- `preferred_model` and `preferred_provider` are dispatcher metadata that
  should never reach the agent prompt

**Evidence from `orchestrate.rs`:**
- `task_def_to_input` converts the task TOML into an agent input
- The conversion includes all metadata fields in the prompt
- No filtering based on which fields the target model can use

**User impact:** Smaller agents produce lower-quality output because their
limited context window is consumed by irrelevant metadata. The user notices
tasks "getting overloaded with too much for smaller agents."

**Resolution plan:**

1. Separate routing metadata from agent-visible metadata:
   - Define two structs: `TaskRoutingMeta` (for dispatcher) and
     `TaskAgentInput` (for the agent prompt)
   - Routing fields: `preferred_model`, `preferred_provider`,
     `reasoning_level`, `speed_priority`, `quality_profile`,
     `context_weight`, `complexity_band`, `escalate_on_retry`
   - Agent fields: `id`, `title`, `description`, `files`,
     `depends_on`, `acceptance`, `context_files`, `tags`
   - Files: `crates/roko-orchestrator/src/` (new types),
     `crates/roko-cli/src/dispatch_helpers.rs` (modify)
   - Effort: Medium (~300 LOC)

2. Add task TOML tier validation:
   - `roko plan validate` checks that fast/trivial tasks do not have
     unnecessary complex-tier fields
   - Warn when a `complexity_band = "fast"` task has `reasoning_level`,
     `quality_profile`, or `example_pattern`
   - File: `crates/roko-cli/src/plan_validate.rs` (extend)
   - Effort: Small (~150 LOC)

3. Add task auto-splitting:
   - When a task has 8+ files, suggest splitting into subtasks
   - Each subtask touches 2-3 files
   - Dependencies are auto-generated based on file import chains
   - File: new `crates/roko-cli/src/commands/task.rs`
   - Effort: Medium (~400 LOC)

---

## Major Issues

### I-UX05: No aggregation or corpus management

**Severity:** Major
**Affects:** Aggregation phase

The user's workflow starts with gathering context: docs, source files,
requirements, research. Today this is done manually by pasting into a Claude
conversation. Roko has no command for this.

**What exists:**
- `roko research` can gather external information
- `roko knowledge query` can search the neuro store
- `roko prd` manages requirement documents
- But there is no unified "gather all relevant context" command

**Resolution:** Build `roko ingest` command that:
- Accepts file paths, globs, URLs, PRD slugs
- Builds a structured corpus (JSON) with token counts
- Stores in `.roko/corpora/<label>.json`
- Supports incremental updates (add to existing corpus)
- File: new `crates/roko-cli/src/commands/ingest.rs`
- Effort: Medium (~500 LOC)

---

### I-UX06: No multi-pass funnel workflow

**Severity:** Major
**Affects:** Funneling phase

The user makes 5+ passes over aggregated context to refine it into executable
plans. Today this is done entirely in external Claude sessions. Roko has no
native support for progressive refinement.

**What exists:**
- `roko prd plan <slug>` generates a plan from a PRD (single pass)
- `roko plan generate` generates a plan from a prompt (single pass)
- But neither supports multi-pass refinement with user checkpoints

**Resolution:** Build `roko funnel` command that:
- Takes a corpus label or inline sources
- Runs 5 passes: architecture, gaps, tasks, deps, gates
- Each pass is an agent call with structured output
- User reviews and approves each pass before continuing
- Saves checkpoints per pass for resume
- Final output is a validated tasks.toml
- File: new `crates/roko-cli/src/commands/funnel.rs`
- Effort: Large (~1,000 LOC)

---

### I-UX07: Five chat modes with zero shared rendering

**Severity:** Major
**Affects:** CLI UX consistency, maintenance burden

`chat_inline.rs` (4.1K), `chat.rs` (659), `run.rs` (1.5K), `unified.rs`,
`dispatch_direct.rs` each have independent event loops, output formatting,
cost display, and state management. Adding a feature (like tool output
rendering or cost tracking) requires changing up to 5 places.

**Resolution:** Extract shared `ChatSession` that handles dispatch,
streaming, tool output rendering, and cost tracking. Each mode instantiates
it with different config (interactive vs. oneshot vs. agent).
- File: `crates/roko-cli/src/chat_session.rs` (extend)
- Effort: Large (~800 LOC refactor)

---

### I-UX08: TUI state + rendering is 11K LOC monolith

**Severity:** Major
**Affects:** TUI maintainability

`state.rs` (4,968 LOC) + `dashboard.rs` (6,382 LOC) = 11,350 LOC.
Adding new views (board, DAG) will make this worse.

**Resolution:** Decompose into per-tab state modules. Each tab owns its
slice of state. Central coordinator routes events.
- File: `crates/roko-cli/src/tui/state.rs` (split)
- Effort: Large (refactor)

---

### I-UX09: No task TOML validation

**Severity:** Major
**Affects:** Funneling phase, execution reliability

`roko plan validate` exists as a command but performs minimal checks. It does
not validate:
- Circular dependencies
- File conflict detection (two tasks modifying the same file in the same
  parallel group)
- Acceptance criteria format (are they executable shell commands?)
- Missing required fields per complexity tier
- Estimated cost/time consistency
- Context file existence

**Evidence from `crates/roko-cli/src/plan_validate.rs`:**
- Basic structural validation only
- No semantic validation of task relationships

**Resolution:**
1. Add circular dependency detection:
   - Build DAG from `depends_on`, detect cycles
   - Error: "Circular dependency: T3 -> T5 -> T3"
   - Effort: Small (~100 LOC)

2. Add file conflict detection:
   - Two tasks in the same `parallel_group` touching the same file is an
     error unless `exclusive_files = false`
   - Warning: "T4 and T6 both modify src/auth/login.rs in group B"
   - Effort: Small (~80 LOC)

3. Add acceptance criteria format check:
   - Acceptance strings that start with a shell command pattern are valid
   - Acceptance strings that are prose-only get a warning
   - Effort: Small (~60 LOC)

4. Add complexity tier consistency check:
   - `complexity_band = "fast"` + `reasoning_level = "high"` is a warning
   - `complexity_band = "complex"` without `acceptance` is an error
   - Effort: Small (~80 LOC)

---

### I-UX10: No task auto-splitting

**Severity:** Major
**Affects:** Funneling phase, small-model quality

When a task touches 8+ files, it is too large for a small model and often
too large for even Opus to execute reliably. The user has to manually split
it into subtasks.

**Resolution:**
1. Add `roko task split <plan> <task-id>` command:
   - Analyzes the task's `files` list
   - Groups files by directory and import relationships
   - Proposes 2-4 subtasks, each touching 2-3 files
   - Auto-generates `depends_on` relationships between subtasks
   - Asks for user approval before writing
   - File: new `crates/roko-cli/src/commands/task.rs`
   - Effort: Medium (~400 LOC)

2. Add automatic splitting during `roko funnel`:
   - After the task decomposition pass, check each task's file count
   - Auto-split tasks with 8+ files
   - Show the split for user approval
   - Effort: included in funnel command

---

## Medium Issues

### I-UX11: Inline primitives mostly unwired

**Severity:** Medium
**Affects:** Code rot, false confidence

11 inline primitives in `inline/primitives/` (~2.5K LOC). Only 4 are used
in live paths. The rest are only consumed in `bench_demo.rs`.

**Resolution:** Wire them to appropriate live paths or delete. Built-but-
unused code creates false confidence that features work.

---

### I-UX12: No cross-plan task DAG

**Severity:** Medium
**Affects:** Parallelism, scheduling

Mori's `UnifiedTaskDag` used `GlobalTaskId` (plan:task) for cross-plan
dependencies. Roko does not have this.

**Resolution:** Build cross-plan task DAG in `roko-orchestrator`. Use
GlobalTaskId pattern (plan:task).

---

### I-UX13: ACP does not know about tasks

**Severity:** Medium
**Affects:** Editor integration

ACP runs a pure FSM over stdio. It has no awareness of the Board/Epic/Task
hierarchy. It cannot show "which task am I working on" in the editor.

**Resolution:** Extend ACP session (`session.rs`) with task context. When
dispatched for a task, include task metadata.

---

### I-UX14: No queue/batch management

**Severity:** Medium
**Affects:** Large-scale execution

Mori had named queues, milestones, batch pause/resume. Roko has `plan run`
which processes a directory. No queue concept, no batching.

The batch review TUI modal (`modals/batch_review.rs`, 164 LOC) exists but
has no orchestrator trigger.

**Resolution:** Add queue manager and batch controller. Wire the existing
`BatchReview` modal to orchestrator batch completion events.

---

## Low Issues

### I-UX15: Verification test grid missing

Mori showed individual verification tests (CG1-CG7, LS1-LS16, SV1-SV8,
UT1-UT27) with per-test status. Roko shows gate verdicts as pass/fail blocks.

### I-UX16: Theme hardcoded

ROSEDUST palette is hardcoded. High-contrast variant exists but is not
user-selectable.

### I-UX17: No fixture lifecycle management

Mori had fixture manifests with healthchecks. Roko does not manage test
fixtures.

### I-UX18: Right-rail panels designed but unwired

6 right-rail panels designed in ACP showcase. None are wired.

---

## Cross-Subsystem Dependencies

| UX Feature | Depends On | Status |
|---|---|---|
| `roko ingest` | roko-compose (corpus assembly) | Not built |
| `roko funnel` | roko-agent (multi-pass dispatch) | Not built |
| Smart context windowing | roko-compose (SystemPromptBuilder) | Partially built |
| Task metadata tiering | roko-orchestrator (task types) | Not built |
| Task auto-splitting | roko-orchestrator (DAG), roko-index | Not built |
| `roko next` | roko-cli (state inspection) | Not built |
| Dry-run mode | roko-orchestrator (executor) | Not built |
| Run summary | roko-cli (orchestrate.rs) | Not built |
| Task validation | roko-orchestrator (plan types) | Partially built |
| Model routing viz | roko-learn (CascadeRouter) | Built, wired from dead code |
| Knowledge panel | roko-neuro (knowledge store) | Wired |
| Gate results | roko-gate (pipeline) | Wired |
| Cost tracking | roko-agent (CostMeter) | Partial (inline only) |
| Research enrichment | roko-agent (research commands) | Wired |
| Episode replay | roko-learn (episode logger) | Wired |
| DAG execution | roko-orchestrator (PlanRunner) | Wired |

---

## Sources

- `crates/roko-cli/src/main.rs` -- Command enum, CLI structure
- `crates/roko-cli/src/orchestrate.rs` -- PlanRunner, dispatch, gate pipeline
- `crates/roko-cli/src/chat_session.rs` -- ChatAgentSession
- `crates/roko-cli/src/commands/plan.rs` -- plan handlers
- `crates/roko-cli/src/commands/prd.rs` -- PRD lifecycle
- `crates/roko-cli/src/plan_validate.rs` -- plan validation (minimal)
- `crates/roko-cli/src/dispatch_helpers.rs` -- prompt assembly helpers
- `crates/roko-cli/src/unified.rs` -- bare `roko` invocation handler
- `crates/roko-compose/src/system_prompt_builder.rs` -- 9-layer builder
- `crates/roko-orchestrator/src/` -- DAG, executor, task types
- `crates/roko-acp/src/session.rs` -- ACP session state
- `crates/roko-cli/src/tui/modals/task_detail.rs` -- 177 LOC, gate results only
- `crates/roko-cli/src/tui/modals/batch_review.rs` -- 164 LOC, no trigger
- `crates/roko-cli/src/tui/state.rs` -- 4,968 LOC monolith
