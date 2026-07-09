# Implementation Plan 08: UX & CLI

> Covers: `roko next` command, run summaries, dry-run mode, Context Packs (5-pass
> funnel), corpus management (`roko ingest`), task TOML validation and auto-splitting,
> interactive steering, smart context windowing for different model sizes, progressive
> context refinement, TUI improvements, Mori-style task ingestion.
>
> Source analysis: `15-UX-AUDIT.md`, `15-UX-GOALS.md`, `15-UX-ISSUES.md`,
> `15-UX-PLAN.md`, `15-UX-FEATURES.md`, `15-UX-MORI-REFERENCE.md`,
> `09-UX-WORKFLOW-VISION.md`, `11-CURRENT-STATE-GROUND-TRUTH.md`.
>
> Each task includes exact file paths, what to change, acceptance criteria, and
> dependency ordering. Line numbers are approximate and should be confirmed at
> execution time.

---

## Phase 0: Workflow Foundation (roko next, summaries, dry-run)

**Problem**: The CLI has 60+ subcommands. There is no workflow guidance, no end-of-run
verdicts, and no way to preview execution before committing resources.

**Effort**: ~1,200 LOC new code | **Impact**: Critical
**Dependencies**: None -- can start immediately


### T1: Add `roko next` command -- workspace state inspection

**Files**: new `crates/roko-cli/src/commands/next.rs`, modify `crates/roko-cli/src/commands/mod.rs`
**Complexity**: standard
**What**: Create a `next.rs` module that inspects workspace state and returns a
prioritized list of suggested actions. Check for:
- `.roko/` existence (suggest `roko init` if missing)
- PRDs in `.roko/prd/drafts/` and `.roko/prd/published/` (suggest `prd plan` if drafts exist)
- Plans in discovered plan directories (suggest `plan run` if pending plans exist)
- Executor snapshots in `.roko/state/executor.json` (suggest `resume` if failed tasks exist)
- Episode log `.roko/episodes.jsonl` (compute last-run outcome)
- Context packs in `.roko/packs/` (suggest next pass if incomplete packs exist)

Each suggestion includes the full CLI command to run and a one-line rationale.

**Steps**:
1. Create `next.rs` with `pub async fn cmd_next(workdir: &Path) -> Result<i32>`
2. Implement `WorkspaceState` struct: `has_roko_dir`, `draft_prds: Vec<String>`,
   `pending_plans: Vec<(String, usize, usize)>` (name, done, total),
   `failed_tasks: Vec<(String, String, String)>` (plan, task_id, gate),
   `incomplete_packs: Vec<(String, PackStatus)>`,
   `last_run: Option<RunSummary>`
3. Implement `inspect_workspace(workdir) -> WorkspaceState` that reads filesystem state
4. Implement `format_suggestions(state) -> Vec<Suggestion>` that produces prioritized actions
5. Print suggestions with styled output (green for ready, yellow for attention, red for failures)
6. Add `pub mod next;` to `commands/mod.rs`

**Acceptance criteria**:
- `roko next` in an uninitialized directory suggests `roko init`
- `roko next` in a workspace with draft PRDs suggests `roko prd plan <slug>`
- `roko next` after a failed plan run shows the failed tasks and suggests `roko resume`
- `roko next` with no pending work says "Nothing pending. Create a new idea with `roko prd idea`."

**Depends on**: --


### T2: Register `Next` command in CLI and wire dispatch

**Files**: modify `crates/roko-cli/src/main.rs`, modify `crates/roko-cli/src/commands/util.rs`
**Complexity**: fast
**What**: Add `Next` variant to the `Command` enum in `main.rs`. Wire the dispatch to
`cmd_next()` in `util.rs` or the relevant match arm.

**Steps**:
1. Add `/// Inspect workspace state and suggest what to do next.` + `Next` to the
   `Command` enum (place it near the top, in the "Core workflow" group)
2. Add match arm in the dispatch function: `Command::Next => commands::next::cmd_next(&workdir).await`
3. Update `after_long_help` text to list `next` in the workflow guidance section

**Acceptance criteria**:
- `roko next` invokes `cmd_next` and produces output
- `roko --help` lists `next` in the core workflow group
- `roko next --help` shows a description

**Depends on**: T1


### T3: Change bare `roko` invocation to show `next` output

**Files**: modify `crates/roko-cli/src/unified.rs` (~212 LOC), modify `crates/roko-cli/src/main.rs`
**Complexity**: fast
**What**: Currently bare `roko` with no arguments enters the interactive REPL via
`unified.rs`. Change it to show `roko next` output first, then offer to enter the
REPL as an option.

**Steps**:
1. In `main.rs`, when `Command` parsing results in no subcommand, call `cmd_next()` first
2. After printing suggestions, prompt: "Enter interactive mode? [y/n/q]"
3. If yes, proceed to `unified.rs` REPL. If no or quit, exit cleanly.
4. If `--batch` or non-TTY, skip the prompt and just print suggestions

**Acceptance criteria**:
- Running bare `roko` shows workspace state and suggestions before the REPL prompt
- Non-interactive sessions (piped, CI) get suggestions only, no REPL prompt
- `roko` with a subcommand bypasses the next output (existing behavior preserved)

**Depends on**: T1, T2


### T4: Add end-of-run summary to plan runner

**Files**: modify `crates/roko-cli/src/runner/event_loop.rs` (~3,136 LOC),
  new `crates/roko-cli/src/runner/run_summary.rs`
**Complexity**: standard
**What**: After all tasks complete (or on cancellation/Ctrl-C), print a structured
summary showing pass/fail counts, cost, duration, and resume command.

**Steps**:
1. Create `run_summary.rs` with `RunSummary` struct:
   ```
   plan_name, total_tasks, passed, failed, skipped,
   failed_details: Vec<(task_id, gate_name, excerpt)>,
   total_cost_usd: f64, duration: Duration, resume_cmd: Option<String>
   ```
2. Implement `RunSummary::from_executor_state(state, plan_name, start_time) -> Self`
3. Implement `RunSummary::display(&self)` with colored output:
   - Green header if all passed, red if any failed
   - Per-failed-task: task ID, gate name, first line of error
   - Cost and duration
   - Resume command if failed tasks exist
4. In `event_loop.rs`, after the main `tokio::select!` loop exits, construct and
   display the `RunSummary`
5. Also display on Ctrl-C (in the cancellation handler)

**Acceptance criteria**:
- `roko plan run plans/` ends with a summary block showing pass/fail/cost/duration
- Failed tasks include gate name and error excerpt
- Summary includes `roko resume <plan>` when tasks failed
- Ctrl-C produces a partial summary with "interrupted" status

**Depends on**: --


### T5: Add `roko status --last-run` flag

**Files**: modify `crates/roko-cli/src/commands/status.rs`
**Complexity**: standard
**What**: Read the most recent executor snapshot from `.roko/state/` and display a
detailed last-run summary.

**Steps**:
1. Add `--last-run` flag to the status command's clap args
2. When flag is present, load the latest executor snapshot from `.roko/state/executor.json`
3. Parse task states and gate results from the snapshot
4. Compute: passed/failed/skipped counts, per-task cost (from episodes.jsonl), duration
5. Display using `RunSummary` from T4 (reuse the type)
6. If no snapshot exists, print "No plan runs recorded. Run `roko plan run` first."

**Acceptance criteria**:
- `roko status --last-run` after a plan run shows the same summary as the end-of-run output
- `roko status --last-run` with no snapshot shows a helpful message
- Per-task cost is read from episodes.jsonl and displayed

**Depends on**: T4


### T6: Add `--dry-run` flag to `roko plan run`

**Files**: modify `crates/roko-cli/src/commands/plan.rs` (~1,317 LOC),
  modify `crates/roko-cli/src/runner/task_dag.rs` (~554 LOC)
**Complexity**: standard
**What**: Add `--dry-run` flag that computes the DAG, shows execution waves with
task listings, estimates cost/time, and exits without dispatching agents.

**Steps**:
1. Add `#[arg(long)] dry_run: bool` to `PlanRunArgs` in `plan.rs`
2. In the plan run handler, after loading plans and building the DAG, check `dry_run`
3. If dry-run: call `compute_execution_waves(dag)` which groups tasks by topological layer
4. Display each wave: wave number, task IDs, titles, parallel groups, file counts
5. Estimate cost: sum `estimated_minutes * cost_per_minute` per task (use cascade router
   historical averages if available, otherwise default $0.50/min)
6. Estimate time: sum of sequential waves' max task durations
7. Print total estimates and exit with code 0

**Acceptance criteria**:
- `roko plan run plans/ --dry-run` shows waves, tasks, and estimates without running anything
- Output includes wave grouping, parallelism information, and cost/time estimates
- No agents are spawned, no state is modified, no episodes are recorded
- Exit code is 0

**Depends on**: --


### T7: Update CLI help text with workflow guidance

**Files**: modify `crates/roko-cli/src/main.rs`
**Complexity**: trivial
**What**: Replace the flat `after_long_help` command listing with a guided workflow
section that shows the recommended command sequence.

**Steps**:
1. Add a "Recommended workflow" section to `after_long_help`:
   ```
   Recommended workflow:
     1. roko next              See what to do
     2. roko prd idea "..."    Capture a work item
     3. roko prd draft new     Draft a PRD
     4. roko pack create       Aggregate context
     5. roko pack pipeline     Funnel into executable plan
     6. roko plan run          Execute with gates
     7. roko status --last-run Review results
   ```
2. Keep the existing command group listing below the workflow section

**Acceptance criteria**:
- `roko --help` shows a "Recommended workflow" section before the command groups
- The workflow lists commands in execution order with brief descriptions

**Depends on**: --

---

## Phase 1: Task Metadata Tiering + Smart Context Windowing

**Problem**: Every task gets the full 20+ field treatment and a monolithic 9-layer
prompt regardless of complexity. Smaller models receive bloated prompts that waste
tokens and degrade quality.

**Effort**: ~1,800 LOC new code | **Impact**: Critical for small-model quality
**Dependencies**: Phase 0 (for types shared with summaries)


### T8: Define `TaskSpec` and `TaskAgentInput` types

**Files**: new `crates/roko-orchestrator/src/task_spec.rs`,
  modify `crates/roko-orchestrator/src/lib.rs`
**Complexity**: standard
**What**: Create the two-struct approach for task metadata. `TaskSpec` holds all 20+ fields
on disk. `TaskAgentInput` holds only agent-visible fields, filtered by complexity tier.

**Steps**:
1. Define `ComplexityBand` enum: `Trivial`, `Fast`, `Standard`, `Complex` with `Ord` derive
2. Define `TaskSpec` struct with all fields from the Mori TOML format (see 15-UX-MORI-REFERENCE.md)
   - Agent-visible: `id`, `title`, `description`, `status`, `files`, `depends_on`,
     `acceptance`, `context_files`, `tags`
   - Conditional: `parallel_group`, `exclusive_files`, `category`, `estimated_minutes`,
     `example_pattern`
   - Routing-only: `preferred_model`, `preferred_provider`, `reasoning_level`,
     `speed_priority`, `quality_profile`, `context_weight`, `complexity_band`,
     `escalate_on_retry`
3. Define `TaskAgentInput` struct with agent-visible + conditional fields (all optional
   conditionals)
4. Implement `TaskSpec::to_agent_input(&self) -> TaskAgentInput` that strips routing metadata
   and omits conditional fields below the task's complexity tier
5. Implement `TaskSpec::from_toml_value(value: &toml::Value) -> Result<Self>` for parsing
6. Add `pub mod task_spec;` to `lib.rs`

**Acceptance criteria**:
- `ComplexityBand::Trivial.to_agent_input()` produces a struct with only 5-6 fields populated
- `ComplexityBand::Complex.to_agent_input()` includes all conditional fields
- Routing-only fields (`preferred_model`, `reasoning_level`, etc.) are never in `TaskAgentInput`
- Round-trip: `TaskSpec` -> TOML -> `TaskSpec` preserves all fields
- Unit tests for each tier

**Depends on**: --


### T9: Define `ContextBudget` type for per-section token allocation

**Files**: new `crates/roko-compose/src/context_budget.rs`,
  modify `crates/roko-compose/src/lib.rs`
**Complexity**: standard
**What**: Create a `ContextBudget` struct that allocates token budgets per prompt section
based on `ComplexityBand`. The `SystemPromptBuilder` will use this to cap each section.

**Steps**:
1. Define `ContextBudget` struct with fields per section: `total_tokens`, `identity_tokens`,
   `role_tokens`, `task_tokens`, `files_tokens`, `context_tokens`, `tools_tokens`,
   `constraints_tokens`, `memory_tokens`, `meta_tokens`
2. Implement `ContextBudget::for_complexity(band: ComplexityBand) -> Self` with tier presets:
   - Trivial: 4K total (200/500/2K/1K/0/300/96/0/0)
   - Fast: 16K total (300/1K/4K/8K/2K/500/384/0/200)
   - Standard: 32K total (500/2K/4K/16K/6K/1K/768/2K/500)
   - Complex: 64K+ total (500/2K/4K/32K/16K/2K/1K/4K/4K)
3. Implement `ContextBudget::section_budget(&self, section: &str) -> usize`
4. Implement `ContextBudget::should_include_section(&self, section: &str) -> bool`
   (returns false when section budget is 0)
5. Add `pub mod context_budget;` to `lib.rs`

**Acceptance criteria**:
- `ContextBudget::for_complexity(Trivial).should_include_section("memory")` returns false
- `ContextBudget::for_complexity(Complex).should_include_section("memory")` returns true
- All budgets sum to within 5% of `total_tokens`
- Unit tests verify each tier's section allocations

**Depends on**: T8 (imports ComplexityBand)


### T10: Wire `ContextBudget` into `SystemPromptBuilder`

**Files**: modify `crates/roko-compose/src/system_prompt_builder.rs` (~2,081 LOC)
**Complexity**: standard
**What**: Accept `ContextBudget` in the builder and enforce per-section token limits.
Sections that exceed their budget are truncated. Sections with zero budget are skipped.

**Steps**:
1. Add `context_budget: Option<ContextBudget>` field to `SystemPromptBuilder`
2. Add `pub fn with_context_budget(mut self, budget: ContextBudget) -> Self` builder method
3. In each `add_*_section()` method, check `context_budget.should_include_section(name)`
   before adding. If false, skip the section entirely.
4. After adding a section's content, check `context_budget.section_budget(name)` and
   truncate to that token count if exceeded (use existing token estimation)
5. Keep backward compatibility: when `context_budget` is None, current behavior is preserved
   (no truncation, no section skipping)

**Acceptance criteria**:
- Builder with `ContextBudget::for_complexity(Trivial)` produces a prompt with only
  identity, role, task, and constraints sections (no memory, no meta, no context)
- Builder with `ContextBudget::for_complexity(Complex)` includes all sections
- Builder with no budget set produces the same output as before (backward compatible)
- Section content that exceeds its budget is truncated with "... (truncated)" marker
- Integration test: build prompts for each tier, verify token counts are within budget

**Depends on**: T9


### T11: Wire `TaskSpec` into plan runner dispatch path

**Files**: modify `crates/roko-cli/src/runner/event_loop.rs`,
  modify `crates/roko-cli/src/runner/plan_loader.rs`
**Complexity**: standard
**What**: Parse task TOMLs into `TaskSpec` structs. When dispatching an agent, convert
to `TaskAgentInput` and use that for prompt assembly (stripping routing metadata).
Use routing fields to inform model selection.

**Steps**:
1. In `plan_loader.rs`, use `TaskSpec::from_toml_value()` instead of the current ad-hoc
   task parsing. Fall back to constructing `TaskSpec` with `Standard` complexity band
   for tasks that lack the new fields (backward compatibility).
2. In `event_loop.rs`, when building the agent prompt for a task, call
   `task_spec.to_agent_input()` and serialize that into the prompt instead of the full
   task metadata
3. Use `task_spec.preferred_model` and `task_spec.complexity_band` for model selection
   (feed into cascade router if available)
4. Use `task_spec.context_weight` to construct the `ContextBudget` passed to
   `SystemPromptBuilder`

**Acceptance criteria**:
- A task with `complexity_band = "fast"` dispatched to an agent has a prompt containing
  only 8-10 task metadata fields (no routing metadata)
- A task with `complexity_band = "complex"` includes all conditional fields
- Existing plans without `complexity_band` default to "standard" and work unchanged
- Model selection respects `preferred_model` when set

**Depends on**: T8, T10


### T12: Add section pruning for fast tasks in prompt assembly

**Files**: modify `crates/roko-compose/src/prompt_assembly_service.rs` (~1,048 LOC)
**Complexity**: fast
**What**: When assembling a prompt for a `slim` context weight task, skip memory,
meta, research context, neuro knowledge, and prior task outputs. Keep: identity,
role, task description, acceptance criteria, target files, constraints.

**Steps**:
1. Accept `context_weight: Option<String>` parameter in `assemble()` method
2. When `context_weight == Some("slim")`:
   - Skip `with_knowledge_context()` call
   - Skip `with_episode_context()` call
   - Skip `with_playbook_context()` call
   - Keep `with_tool_instructions()` (always needed)
3. When `context_weight == Some("deep")`:
   - Include all context with generous token allocations
4. Default ("standard"): current behavior preserved

**Acceptance criteria**:
- Assembling a prompt with `context_weight = "slim"` produces a shorter prompt that
  excludes knowledge, episodes, and playbooks
- Assembling with `context_weight = "deep"` includes everything with full budgets
- Assembling with no context_weight works exactly as before

**Depends on**: T9, T10

---

## Phase 2: Task TOML Validation

**Problem**: `roko plan validate` performs only minimal structural checks. Circular
dependencies, file conflicts, and malformed acceptance criteria are caught at runtime
(or not at all).

**Effort**: ~1,000 LOC new code | **Impact**: High
**Dependencies**: Phase 1 (T8 for TaskSpec types)


### T13: Define `ValidationReport` and validation check types

**Files**: new `crates/roko-orchestrator/src/validation.rs`,
  modify `crates/roko-orchestrator/src/lib.rs`
**Complexity**: standard
**What**: Create the validation engine types. A `ValidationReport` accumulates errors,
warnings, and info from a pipeline of checks.

**Steps**:
1. Define `ValidationSeverity` enum: `Error`, `Warning`, `Info`
2. Define `ValidationIssue` struct: `severity`, `check`, `task_id: Option<String>`,
   `message: String`, `suggestion: Option<String>`
3. Define `ValidationCheck` enum: `CircularDependency`, `FileConflict`,
   `AcceptanceCriteriaFormat`, `ComplexityTierConsistency`, `MissingRequiredFields`,
   `FileExistence`, `DependencyReferenceCheck`, `CostEstimateReasonableness`
4. Define `ValidationReport` struct: `errors: Vec<ValidationIssue>`,
   `warnings: Vec<ValidationIssue>`, `info: Vec<ValidationIssue>`,
   `task_count: usize`, `wave_count: usize`, `cost_estimate: Option<f64>`
5. Implement `ValidationReport::has_errors(&self) -> bool`
6. Implement `ValidationReport::display(&self)` with colored output per severity
7. Add `pub mod validation;` to `lib.rs`

**Acceptance criteria**:
- `ValidationReport` can accumulate issues from multiple checks
- `has_errors()` returns true only for Error severity, not warnings
- Display output is color-coded: red for errors, yellow for warnings, blue for info

**Depends on**: --


### T14: Implement circular dependency detection

**Files**: modify `crates/roko-orchestrator/src/validation.rs`
**Complexity**: fast
**What**: Build a directed graph from task `depends_on` fields and detect cycles using
DFS with coloring (white/gray/black).

**Steps**:
1. Implement `check_circular_deps(tasks: &[TaskSpec]) -> Vec<ValidationIssue>`
2. Build adjacency list from `depends_on` fields
3. Run DFS cycle detection; when a cycle is found, trace the cycle path
4. Produce `ValidationIssue` with severity `Error`, message including the cycle path:
   "Circular dependency: T3 -> T5 -> T8 -> T3"
5. Also check for self-references (`depends_on` containing own `id`)

**Acceptance criteria**:
- Tasks with no cycles produce zero issues
- Tasks with a cycle produce an Error issue with the full cycle path
- Self-referencing tasks produce an Error issue
- Test with 3-node cycle and 5-node cycle

**Depends on**: T13


### T15: Implement file conflict detection

**Files**: modify `crates/roko-orchestrator/src/validation.rs`
**Complexity**: fast
**What**: Detect when two tasks in the same parallel group modify the same file
without `exclusive_files = true`.

**Steps**:
1. Implement `check_file_conflicts(tasks: &[TaskSpec]) -> Vec<ValidationIssue>`
2. Group tasks by `parallel_group`
3. Within each group, build a map of `file -> Vec<task_id>`
4. When a file appears in 2+ tasks within the same group:
   - If any of those tasks has `exclusive_files = false` (or unset): emit Warning
   - If all have `exclusive_files = true`: emit Info (handled by scheduler)
5. Message: "T4 and T6 both modify src/auth/login.rs in parallel group B"

**Acceptance criteria**:
- Two tasks in the same parallel_group touching the same file produce a Warning
- Tasks in different parallel_groups touching the same file produce no issue
- Tasks with `exclusive_files = true` in the same group produce Info (not Warning)

**Depends on**: T13


### T16: Implement acceptance criteria format check

**Files**: modify `crates/roko-orchestrator/src/validation.rs`
**Complexity**: fast
**What**: Check that acceptance criteria strings look like executable shell commands.
Warn on prose-only criteria that cannot be mechanically verified.

**Steps**:
1. Implement `check_acceptance_format(tasks: &[TaskSpec]) -> Vec<ValidationIssue>`
2. For each task's `acceptance` strings, check if they start with a recognized command
   pattern: `cargo`, `test`, `grep`, `diff`, `[`, `!`, `sh -c`, or contain `&&`/`||`/`|`
3. Criteria that are pure prose (no command-like patterns) get a Warning:
   "T3 acceptance[1] is prose, not a shell command. Consider: `cargo test -p crate auth`"
4. Tasks with `complexity_band = "complex"` and zero acceptance criteria get an Error:
   "T7 has no acceptance criteria but is marked complex"

**Acceptance criteria**:
- `"cargo test -p my-crate"` passes (no issue)
- `"All tests pass"` produces a Warning
- Complex task with empty `acceptance` produces an Error

**Depends on**: T13


### T17: Implement complexity tier consistency check

**Files**: modify `crates/roko-orchestrator/src/validation.rs`
**Complexity**: fast
**What**: Detect inconsistent combinations of complexity metadata.

**Steps**:
1. Implement `check_tier_consistency(tasks: &[TaskSpec]) -> Vec<ValidationIssue>`
2. Check for contradictions:
   - `complexity_band = "fast"` + `reasoning_level = "high"` -> Warning
   - `complexity_band = "trivial"` + `estimated_minutes > 15` -> Warning
   - `complexity_band = "complex"` + no `acceptance` -> Error
   - `complexity_band = "fast"` + 8+ files -> Warning ("consider splitting")
   - `context_weight = "slim"` + `complexity_band = "complex"` -> Warning

**Acceptance criteria**:
- Fast task with high reasoning_level produces a Warning
- Complex task with no acceptance produces an Error
- Consistent tasks produce zero issues

**Depends on**: T13, T8


### T18: Implement dependency reference and file existence checks

**Files**: modify `crates/roko-orchestrator/src/validation.rs`
**Complexity**: fast
**What**: Verify that all `depends_on` references point to existing task IDs, and
that `files` and `context_files` reference paths that exist in the workspace.

**Steps**:
1. Implement `check_references(tasks: &[TaskSpec], workdir: &Path) -> Vec<ValidationIssue>`
2. Build set of all task IDs. For each task's `depends_on`, verify target exists.
   Missing reference -> Error: "T5 depends on T99 which does not exist"
3. For each task's `files`, check `workdir.join(file).exists()` or that the parent directory
   exists (for files to be created). Missing parent -> Warning.
4. For each task's `context_files`, check existence. Missing -> Warning (not Error, as
   context files may be optional)

**Acceptance criteria**:
- `depends_on = ["T99"]` when T99 is not in the plan produces an Error
- `files = ["src/nonexistent/foo.rs"]` where `src/nonexistent/` does not exist produces a Warning
- `context_files = ["docs/missing.md"]` produces a Warning

**Depends on**: T13


### T19: Wire validation into `roko plan validate` and `plan run` pre-flight

**Files**: modify `crates/roko-cli/src/commands/plan.rs`,
  modify `crates/roko-cli/src/runner/event_loop.rs`
**Complexity**: standard
**What**: Call the full validation pipeline from `roko plan validate` and as a pre-flight
check in `roko plan run`. Errors block execution; warnings are displayed but do not block.

**Steps**:
1. In `plan.rs`, the `Validate` subcommand handler calls each validation check function
   on the loaded `TaskSpec` list, collects results into a `ValidationReport`, and displays it
2. Add `--skip-validation` flag to `PlanRunArgs`
3. In `event_loop.rs`, before entering the main execution loop, run validation unless
   `--skip-validation` is set. If `report.has_errors()`, print the report and exit with code 1.
   If only warnings, print them and continue.
4. If dry-run is also set, show validation results as part of the dry-run output

**Acceptance criteria**:
- `roko plan validate plans/` runs all checks and prints a report
- `roko plan run plans/` runs validation before execution and aborts on errors
- `roko plan run plans/ --skip-validation` bypasses validation
- Validation warnings during `plan run` are printed but execution continues

**Depends on**: T14, T15, T16, T17, T18, T6

---

## Phase 3: Task Auto-Splitting

**Problem**: Tasks with 8+ files are too large for smaller models. The user must
manually decompose them. The system should auto-propose splits.

**Effort**: ~1,100 LOC new code | **Impact**: High
**Dependencies**: Phase 2 (T13 for ValidationReport, T8 for TaskSpec)


### T20: Define `SplitProposal` type and file grouping logic

**Files**: new `crates/roko-orchestrator/src/task_split.rs`,
  modify `crates/roko-orchestrator/src/lib.rs`
**Complexity**: standard
**What**: Implement the core splitting algorithm. Group task files by directory
and/or import chain to produce subtask proposals.

**Steps**:
1. Define `SplitProposal` struct: `original_task_id`, `subtasks: Vec<TaskSpec>`,
   `merge_task: Option<TaskSpec>`, `rationale: String`
2. Implement `should_split(task: &TaskSpec) -> Option<SplitReason>`:
   - `SplitReason::TooManyFiles` if `files.len() >= 8`
   - `SplitReason::ContextOverflow` if estimated context exceeds tier budget
   - `SplitReason::TierMismatch` if fast/trivial tier with many files
3. Implement `split_by_directory(task: &TaskSpec) -> SplitProposal`:
   - Group files by parent directory
   - Each group becomes a subtask with the group's files
   - Generate subtask IDs as `{original_id}-sub{N}`
   - Set `depends_on` for subtasks to match original task's dependencies
   - Create a merge task `{original_id}-merge` that depends on all subtasks
   - Merge task inherits the original's `acceptance` criteria
4. Add `pub mod task_split;` to `lib.rs`

**Acceptance criteria**:
- A task with 10 files across 3 directories produces 3 subtasks + 1 merge task
- Each subtask has 2-4 files from one directory
- Merge task depends on all subtasks and inherits parent acceptance criteria
- Subtask IDs follow the `{parent}-sub{N}` pattern

**Depends on**: T8


### T21: Implement import-chain-aware file grouping

**Files**: modify `crates/roko-orchestrator/src/task_split.rs`
**Complexity**: standard
**What**: Extend splitting to consider Rust `use`/`mod` import chains. Files that
import each other should stay in the same subtask.

**Steps**:
1. Implement `split_by_imports(task: &TaskSpec, workdir: &Path) -> SplitProposal`
2. For each file in `task.files`, parse `use` and `mod` statements (simple regex,
   not full AST -- sufficient for grouping heuristic)
3. Build an import graph: `file -> Vec<imported_file>` (only among task files)
4. Find connected components in the undirected import graph
5. Each connected component becomes a subtask
6. Files with no import relationships fall back to directory-based grouping
7. If roko-index is available, use symbol resolution for more accurate grouping

**Acceptance criteria**:
- Two files that `use` each other stay in the same subtask
- Files with no import relationship are grouped by directory
- Connected component algorithm correctly handles transitive imports
- Test with a 10-file task where 3 files form an import cluster

**Depends on**: T20


### T22: Implement acceptance criteria distribution for subtasks

**Files**: modify `crates/roko-orchestrator/src/task_split.rs`
**Complexity**: fast
**What**: When splitting, distribute parent acceptance criteria to subtasks
intelligently and ensure the merge task inherits all criteria.

**Steps**:
1. Implement `distribute_acceptance(parent: &TaskSpec, subtasks: &mut [TaskSpec])`
2. For each acceptance criterion, check if it references a specific file or crate:
   - If it contains a file path from a subtask's files, assign to that subtask
   - If it contains a crate name matching a subtask's files, assign to that subtask
   - If it is generic (e.g., "cargo test --workspace"), assign to merge task only
3. Add `"cargo check"` as default acceptance for subtasks that have no inherited criteria
4. Merge task always gets the full set of parent acceptance criteria

**Acceptance criteria**:
- `"cargo test -p my-crate auth"` assigned to the subtask containing auth files
- `"cargo clippy --workspace"` assigned to merge task only
- Every subtask has at least `"cargo check"` as acceptance

**Depends on**: T20


### T23: Add `roko task split <plan> <task-id>` CLI command

**Files**: new `crates/roko-cli/src/commands/task.rs`,
  modify `crates/roko-cli/src/main.rs`, modify `crates/roko-cli/src/commands/mod.rs`
**Complexity**: standard
**What**: Interactive CLI command that proposes a split and asks for approval before
writing the updated tasks.toml.

**Steps**:
1. Create `task.rs` with `TaskCmd` subcommand enum containing `Split { plan_dir, task_id }`
2. Load the plan's tasks.toml, parse into `Vec<TaskSpec>`
3. Find the target task, call `should_split()`, then `split_by_imports()` (or `split_by_directory()`
   as fallback)
4. Display the proposal: original task, proposed subtasks with file lists, merge task
5. Prompt: "Apply this split? [y/n/e(dit)]"
6. If yes: replace the original task with subtasks + merge task in the TOML, write to disk
7. If edit: open `$EDITOR` with the proposed TOML fragment, then apply
8. Register `Task` command variant in `main.rs`, add `pub mod task;` to `commands/mod.rs`

**Acceptance criteria**:
- `roko task split plans/sprint-42 T6` proposes a split and shows the preview
- Accepting the split updates tasks.toml with subtasks replacing the original
- The updated tasks.toml passes `roko plan validate`
- Rejecting the split leaves tasks.toml unchanged

**Depends on**: T20, T21, T22

---

## Phase 4: Context Packs & Corpus Management

**Problem**: The user's workflow starts with aggregating context from docs, code, and
research. Today this is done manually by pasting into external Claude sessions. The
CLI has no `ingest` or corpus management command.

**Effort**: ~2,800 LOC new code | **Impact**: Critical for workflow ownership
**Dependencies**: Phase 3 (T20 for splitting during decomposition)


### T24: Define Context Pack data model and storage

**Files**: new `crates/roko-cli/src/pack.rs`
**Complexity**: standard
**What**: Create the pack manifest, source tracking, and directory layout types.
Context packs are stored in `.roko/packs/<pack-id>/`.

**Steps**:
1. Define `PackStatus` enum: `Raw`, `Synthesized`, `Architected`, `Decomposed`,
   `Scoped`, `Executing`, `Done`
2. Define `PackManifest` struct: `pack: PackMeta`, `sources: PackSources`,
   `budget: PackBudget`, `passes: Vec<PassRecord>`, `approval: ApprovalConfig`
3. Define `PackSources`: `dirs: Vec<PathBuf>`, `files: Vec<PathBuf>`, `urls: Vec<String>`,
   `prd_slugs: Vec<String>`
4. Define `PackBudget`: `synthesis_budget`, `arch_budget`, `decompose_budget` (token counts)
5. Define `PassRecord`: `name`, `agent_role`, `model`, `input_tokens`, `output_tokens`,
   `output_file`, `timestamp`, `duration_secs`
6. Implement `create_pack()`, `load_manifest()`, `save_manifest()`, `list_packs()`,
   `add_sources()`
7. Directory layout: `.roko/packs/<id>/manifest.toml`, `raw/`, `scoping/`
8. Source linking: symlink source dirs/files into `raw/` (copy on symlink failure)

**Acceptance criteria**:
- `create_pack()` creates the directory structure with manifest
- `load_manifest()` round-trips through TOML serialization
- `list_packs()` discovers all packs sorted by creation date
- `add_sources()` adds new files/dirs to an existing pack

**Depends on**: --


### T25: Implement raw content collection and token estimation

**Files**: modify `crates/roko-cli/src/pack.rs`
**Complexity**: fast
**What**: Collect all text content from a pack's `raw/` directory, with token counting
for budget management.

**Steps**:
1. Implement `collect_raw_content(pack_dir: &Path) -> Result<(String, usize)>` that
   recursively reads all text files from `raw/`, returns concatenated content and token count
2. Implement `is_text_file(path: &Path) -> bool` checking extensions:
   md, txt, rs, toml, yaml, yml, json, ts, tsx, js, py, go, sh, html, css, sql
3. Implement `estimate_tokens(text: &str) -> usize` using ~4 chars/token heuristic
4. Add file separators: `\n--- {path} ---\n` between files for context

**Acceptance criteria**:
- Collecting a directory with 5 text files and 2 binary files returns content from only the 5
- Token estimate for 1000-char text is approximately 250
- File separators appear between each file's content

**Depends on**: T24


### T26: Add `roko pack create/list/status/add` CLI commands

**Files**: new `crates/roko-cli/src/commands/pack.rs`,
  modify `crates/roko-cli/src/main.rs`, modify `crates/roko-cli/src/commands/mod.rs`
**Complexity**: standard
**What**: Register the `Pack` command group with CRUD subcommands for context packs.

**Steps**:
1. Define `PackCmd` subcommand enum: `Create`, `List`, `Status`, `Add`, `Synthesize`,
   `Architect`, `Decompose`, `Scope`, `Execute`, `Pipeline`, `Show`, `Split`
2. Implement `cmd_pack(workdir, cmd)` with handlers for `Create`, `List`, `Status`, `Add`
3. Register `Pack { cmd: PackCmd }` variant in the `Command` enum in `main.rs`
4. Wire dispatch in the main match arm
5. Add `pub mod pack;` to `commands/mod.rs`

**Acceptance criteria**:
- `roko pack create "sprint-42" --from docs/ crates/roko-core/src/` creates a pack
- `roko pack list` shows all packs with status and pass count
- `roko pack status sprint-42` shows pack details including sources and passes
- `roko pack add sprint-42 extra-docs/` adds sources to an existing pack
- `roko pack --help` shows all subcommands

**Depends on**: T24, T25


### T27: Implement synthesis pass (Pass 1: compress raw material)

**Files**: new `crates/roko-cli/src/pack_pipeline.rs`
**Complexity**: standard
**What**: The synthesis pass uses a Researcher agent to compress the raw pack material
into a 10K-token design brief. For packs larger than the model's context window,
use a sliding window map-reduce approach.

**Steps**:
1. Implement `run_synthesis(workdir, pack_dir, manifest, model) -> Result<()>`
2. Collect raw content. If within budget, single-pass: dispatch Researcher agent with
   the full content as input, output as `pass-01-synthesis.md`
3. If over budget, implement sliding window:
   - Split content into chunks of `budget * 0.7` tokens (leave room for system prompt)
   - Dispatch Researcher agent per chunk to produce a chunk summary
   - If summaries fit in one window, synthesize them into final output
   - If not, recurse (map-reduce)
4. System prompt: "You are a research synthesizer. Compress the following material into
   a structured design brief. Preserve: key requirements, technical constraints, prior
   art references, and open questions. Target: ~10K tokens."
5. Update manifest with `PassRecord` and advance status to `Synthesized`
6. Wire into `PackCmd::Synthesize` handler in `commands/pack.rs`

**Acceptance criteria**:
- `roko pack synthesize sprint-42` produces `pass-01-synthesis.md`
- Small packs (< budget) use single-pass synthesis
- Large packs (> budget) use sliding window with chunk summaries
- Manifest is updated with pass record including token counts
- Pack status advances to `Synthesized`

**Depends on**: T24, T25, T26


### T28: Implement architecture pass (Pass 2: produce spec)

**Files**: modify `crates/roko-cli/src/pack_pipeline.rs`
**Complexity**: standard
**What**: The architecture pass reads the synthesis output plus repo context and produces
an architecture specification with component boundaries, data flow, file manifest, and
dependency ordering.

**Steps**:
1. Implement `run_architecture(workdir, pack_dir, manifest, model) -> Result<()>`
2. Read `pass-01-synthesis.md` as input
3. Build repo context using existing `build_repo_context()` from `repo_context.rs`
4. System prompt: "You are a software architect. Given a design brief and repository
   context, produce an architecture spec including: component boundaries, data flow,
   file change manifest (create/modify/delete), dependency ordering, risk assessment."
5. Dispatch Architect agent, write output to `pass-02-arch.md`
6. Update manifest, advance status to `Architected`
7. Wire into `PackCmd::Architect` handler

**Acceptance criteria**:
- `roko pack architect sprint-42` reads synthesis output and produces arch spec
- Arch spec references actual file paths and crate names from the workspace
- Manifest status advances to `Architected`

**Depends on**: T27


### T29: Implement decomposition pass (Pass 3: generate tasks.toml)

**Files**: modify `crates/roko-cli/src/pack_pipeline.rs`
**Complexity**: standard
**What**: The decomposition pass reads the architecture spec and generates a `tasks.toml`
with tiered metadata. Auto-split any tasks with 8+ files.

**Steps**:
1. Implement `run_decomposition(workdir, pack_dir, manifest, model) -> Result<()>`
2. Read `pass-02-arch.md` as input
3. System prompt: "Generate a tasks.toml with [[task]] entries. Each task has: id, title,
   files, depends_on, acceptance (shell commands), complexity_band, category, context_files.
   Use complexity_band: trivial/fast/standard/complex based on task scope. Tasks with 4+
   files should be standard or complex. Keep acceptance criteria as executable shell commands."
4. Dispatch Strategist agent, parse output as TOML
5. Validate the generated tasks using the validation engine from Phase 2
6. Auto-split any tasks where `should_split()` returns true (from T20)
7. Write to `pass-03-tasks.toml`
8. Update manifest, advance status to `Decomposed`
9. Wire into `PackCmd::Decompose` handler

**Acceptance criteria**:
- `roko pack decompose sprint-42` generates valid tasks.toml
- Generated tasks have proper tiers, dependencies, and shell-command acceptance criteria
- Tasks with 8+ files are auto-split into subtasks
- `roko plan validate` passes on the generated tasks.toml

**Depends on**: T28, T20


### T30: Implement scoping pass (Pass 4: per-task context slicing)

**Files**: modify `crates/roko-cli/src/pack_pipeline.rs`,
  new `crates/roko-cli/src/context_slicer.rs`
**Complexity**: standard
**What**: For each task in the generated tasks.toml, compute a context slice --
the minimal context needed for that specific task, bounded by the task's tier budget.

**Steps**:
1. Create `context_slicer.rs` with `compute_context_slice(task, pack_dir, workdir) -> ContextSlice`
2. Define `ContextSlice`: `task_id`, `total_tokens`, `sections: Vec<ContextSection>`
3. Define `ContextSection`: `source` (task_def, file, dep_output, arch_spec, conventions,
   knowledge), `tokens`, `content`, `priority`
4. Implement the windowing algorithm from 09-UX-WORKFLOW-VISION.md section 3.2:
   - Layer 1: Task instructions (always, ~500 tokens)
   - Layer 2: Target file contents (high priority, up to 40% of budget)
   - Layer 3: Dependency outputs (medium, up to 15%)
   - Layer 4: Architecture context from pass-02 (medium, up to 20%)
   - Layer 5: Repo conventions (low, up to 10%)
   - Layer 6: Knowledge store hits (low, up to 10%)
5. Write each slice to `scoping/T{id}-context.toml`
6. Update manifest, advance status to `Scoped`

**Acceptance criteria**:
- Each task gets a context slice file in `scoping/`
- Trivial tasks get slices under 4K tokens
- Complex tasks get slices up to 64K tokens
- Context provenance is tracked per section (source + token count)
- Total tokens per slice respects the tier budget

**Depends on**: T29, T9


### T31: Implement `roko pack pipeline` (all passes in sequence)

**Files**: modify `crates/roko-cli/src/pack_pipeline.rs`,
  modify `crates/roko-cli/src/commands/pack.rs`
**Complexity**: fast
**What**: Run all 5 passes in sequence with optional approval gates between passes.
Resume from the last completed pass if interrupted.

**Steps**:
1. Implement `run_pipeline(workdir, pack_dir, manifest, model, approve_before) -> Result<()>`
2. Check `manifest.pack.status` to determine which passes are already complete
3. Run remaining passes in order: synthesis -> architecture -> decomposition -> scoping -> execute
4. Between each pass, if `approve_before` contains the next pass name or if the pack's
   `approval.require_before` contains it, prompt: "Pass N complete. Review output at
   {path}. Continue? [y/n/e(dit)]"
5. On "n", save checkpoint (manifest is already saved per-pass) and exit
6. On "e", open `$EDITOR` with the pass output, then continue
7. The "execute" step delegates to `roko plan run` with the generated tasks.toml

**Acceptance criteria**:
- `roko pack pipeline sprint-42` runs all passes end to end
- Interrupting mid-pipeline and re-running resumes from the last completed pass
- `--approve-before execute` pauses before execution for user review
- Each pass's output is visible as a file in the pack directory

**Depends on**: T27, T28, T29, T30

---

## Phase 5: Interactive Steering & Ingestion

**Problem**: The CLI is fire-and-forget. For the funnel workflow, users need to steer
between passes, edit artifacts, and re-run individual passes.

**Effort**: ~900 LOC new code | **Impact**: High
**Dependencies**: Phase 4 (T24-T31 for pack infrastructure)


### T32: Add `roko ingest` command (simplified pack creation)

**Files**: modify `crates/roko-cli/src/commands/pack.rs`,
  modify `crates/roko-cli/src/main.rs`
**Complexity**: fast
**What**: `roko ingest` is a convenience alias that creates a pack and immediately
starts the pipeline. Accepts file paths, globs, URLs, and PRD slugs.

**Steps**:
1. Add `Ingest` variant to `Command` enum with args: `label: String`,
   `sources: Vec<String>`, `--from-prd: Option<String>`, `--from-plan: Option<String>`
2. Implement handler:
   - If `--from-prd`, read the PRD file, extract referenced files/crates, build `PackSources`
   - If `--from-plan`, read the plan's tasks.toml, extract all `files` and `context_files`
   - Otherwise, partition sources into dirs, files, and URLs
3. Call `create_pack()` then `run_pipeline()` if the user confirms
4. Print: "Ingested N docs + M source files (X tokens). Pack: .roko/packs/{label}"

**Acceptance criteria**:
- `roko ingest sprint-42 docs/ crates/roko-core/src/` creates a pack and reports token count
- `roko ingest sprint-42 --from-prd system-prompt-wiring` extracts PRD references into a pack
- `roko ingest sprint-42 --from-plan plans/sprint-42/` extracts plan file references

**Depends on**: T24, T26


### T33: Implement per-pass re-run and editing

**Files**: modify `crates/roko-cli/src/commands/pack.rs`,
  modify `crates/roko-cli/src/pack_pipeline.rs`
**Complexity**: standard
**What**: Allow re-running individual passes and editing pass outputs. When a pass is
re-run, downstream passes are invalidated.

**Steps**:
1. Add `Rerun { name: String, pass: String, focus: Option<String> }` subcommand to `PackCmd`
2. When re-running a pass:
   - Delete output files for this pass and all downstream passes
   - Reset `manifest.pack.status` to the pass before the re-run target
   - Remove downstream `PassRecord` entries from manifest
   - Run the target pass (optionally with `--focus` injected into the system prompt)
3. Add `Edit { name: String, pass: String }` subcommand:
   - Open the pass output file in `$EDITOR`
   - After editing, prompt: "Re-validate downstream passes? [y/n]"
   - If yes, re-run validation on tasks.toml (if pass was decomposition)
4. Add `Show { name: String, pass: Option<String> }` subcommand:
   - Display the pass output (or all pass summaries if no pass specified)

**Acceptance criteria**:
- `roko pack rerun sprint-42 --pass synthesis --focus "security"` re-synthesizes with focus
- Downstream passes are invalidated when an upstream pass is re-run
- `roko pack edit sprint-42 --pass decompose` opens tasks.toml in $EDITOR
- `roko pack show sprint-42 --pass arch` displays the architecture spec

**Depends on**: T27, T28, T29, T31


### T34: Add `--approve-each` flag to `roko plan run`

**Files**: modify `crates/roko-cli/src/commands/plan.rs`,
  modify `crates/roko-cli/src/runner/event_loop.rs`
**Complexity**: standard
**What**: Pause after each task completes for user approval before proceeding.
Show the task result (gate verdicts, changes made) and prompt continue/retry/skip/abort.

**Steps**:
1. Add `#[arg(long)] approve_each: bool` to `PlanRunArgs`
2. In `event_loop.rs`, after a task completes and gate results are known:
   - If `approve_each`, print task summary: ID, title, gate verdicts, files changed
   - Prompt: "[c]ontinue / [r]etry / [s]kip / [a]bort"
   - Continue: proceed to next task
   - Retry: re-dispatch the same task with gate failure context
   - Skip: mark task as skipped, unblock dependents
   - Abort: save state and exit
3. In non-TTY mode, `--approve-each` is ignored with a warning

**Acceptance criteria**:
- `roko plan run plans/ --approve-each` pauses after each task
- User can retry a failed task inline without restarting the run
- User can skip a task and continue with dependents
- Aborting saves state for later `--resume`

**Depends on**: T4


### T35: Add `--cost-cap` flag to `roko plan run`

**Files**: modify `crates/roko-cli/src/commands/plan.rs`,
  modify `crates/roko-cli/src/runner/event_loop.rs`
**Complexity**: fast
**What**: Stop execution when total accumulated cost exceeds a specified dollar amount.

**Steps**:
1. Add `#[arg(long)] cost_cap: Option<f64>` to `PlanRunArgs`
2. In `event_loop.rs`, maintain a running `total_cost_usd: f64` counter
3. After each agent completion, add the task cost (from usage data) to the counter
4. Before dispatching the next task, check `total_cost_usd >= cost_cap`:
   - If exceeded, print: "Cost cap reached ($X.XX / $Y.YY). Stopping execution."
   - Save state and exit with code 2 (distinct from success=0 and error=1)
5. Show running cost in task completion messages

**Acceptance criteria**:
- `roko plan run plans/ --cost-cap 10.00` stops at $10
- Exit code is 2 (distinguishable from normal failure)
- State is saved for `--resume`
- Running cost is visible in per-task completion messages

**Depends on**: T4

---

## Phase 6: Mori-Style Task Ingestion & DAG Updates

**Problem**: Mori's ingest-and-update loop let users edit tasks.toml mid-run and have
the DAG update automatically. Roko does not support mid-flight plan editing.

**Effort**: ~1,000 LOC new code | **Impact**: Medium-High
**Dependencies**: Phase 2 (T19 for validation), Phase 3 (T20 for splitting)


### T36: Implement `roko plan ingest <dir>` for Mori-style task loading

**Files**: modify `crates/roko-cli/src/commands/plan.rs`,
  modify `crates/roko-cli/src/runner/plan_loader.rs`
**Complexity**: standard
**What**: Add an `Ingest` subcommand to `PlanCmd` that reads a directory of tasks.toml
files, validates them, builds the DAG, and sets up executor state -- similar to Mori's
ingestion. This is distinct from `plan run` which both ingests and executes.

**Steps**:
1. Add `Ingest { dir: PathBuf }` to `PlanCmd`
2. In the handler:
   - Discover all `tasks.toml` files recursively in the directory
   - Parse each into `Vec<TaskSpec>` via `plan_loader.rs`
   - Run full validation (Phase 2 validation engine)
   - Build the per-plan DAG and compute execution waves
   - Persist initial executor state to `.roko/state/executor.json`
3. Print summary: N plans, M tasks, K waves, estimated cost/time
4. If `--auto-split` flag is set, run auto-splitting (Phase 3) before persisting

**Acceptance criteria**:
- `roko plan ingest plans/` reads tasks.toml files, validates, and persists state
- Invalid TOMLs produce error output and prevent ingestion
- After ingestion, `roko plan run plans/ --resume` can execute without re-parsing
- `roko plan ingest plans/ --auto-split` splits large tasks before persisting

**Depends on**: T19, T8


### T37: Implement filesystem watcher for mid-run plan editing

**Files**: modify `crates/roko-cli/src/runner/event_loop.rs`,
  new `crates/roko-cli/src/runner/plan_watcher.rs`
**Complexity**: standard
**What**: Watch the plan directory for changes to tasks.toml files. When a change is
detected, pause execution, re-parse, diff against in-memory state, update the DAG
incrementally, and resume.

**Steps**:
1. Create `plan_watcher.rs` with `PlanWatcher` struct wrapping `notify::RecommendedWatcher`
   (already a dependency; used in `tui/fs_watch.rs`)
2. Watch all `tasks.toml` files in the plan directory
3. On change event, debounce (500ms), then:
   - Re-parse the changed tasks.toml into `Vec<TaskSpec>`
   - Diff against the in-memory task list: find added, removed, modified tasks
   - For added tasks: insert into DAG, compute new dependencies
   - For removed tasks: remove from DAG, unblock dependents (if pending)
   - For modified tasks: update metadata, recompute if status changed
   - Rebuild execution waves for the affected subgraph
4. In `event_loop.rs`, add a `plan_change` branch to the `tokio::select!`:
   - Receive change events from `PlanWatcher` via a channel
   - Apply the DAG update
   - Log: "Plan updated: +2 tasks, -1 task, DAG recomputed"

**Acceptance criteria**:
- Editing tasks.toml during a run updates the in-memory DAG without restarting
- Adding a new task to tasks.toml causes it to appear in the execution queue
- Removing a pending task removes it from the queue
- Changing a task's `depends_on` recomputes wave ordering
- The watcher does not trigger on status-only changes (which the runner itself writes)

**Depends on**: T36


### T38: Support cross-plan task references (GlobalTaskId)

**Files**: modify `crates/roko-orchestrator/src/dag.rs` (~2,557 LOC),
  modify `crates/roko-orchestrator/src/task_spec.rs`
**Complexity**: standard
**What**: Support Mori-style cross-plan dependencies using `"plan_id:task_id"` syntax
in `depends_on` fields. Build a unified DAG across all plans.

**Steps**:
1. Define `GlobalTaskId` struct: `plan_id: String`, `task_id: String` with `Display`
   rendering as `"plan_id:task_id"` and `FromStr` parsing
2. In `depends_on` parsing, detect the `:` separator: if present, parse as cross-plan
   reference; otherwise, treat as intra-plan reference
3. Extend `dag.rs` to accept cross-plan edges when building the unified DAG
4. In the executor, a task with cross-plan dependencies is blocked until the referenced
   task in the other plan completes
5. Validation: check that cross-plan references resolve to existing plan:task pairs

**Acceptance criteria**:
- `depends_on = ["09:T3"]` blocks until plan 09's T3 completes
- Invalid cross-plan references produce validation errors
- Cross-plan cycles are detected by the circular dependency checker (T14)

**Depends on**: T14, T8

---

## Phase 7: TUI Improvements

**Problem**: The TUI is an observation tool, not a workflow driver. Users cannot act
from the dashboard. Task details show gate results only, not full metadata.

**Effort**: ~1,200 LOC new code | **Impact**: Medium
**Dependencies**: Phase 1 (T8 for TaskSpec), Phase 4 (T24 for pack types)


### T39: Enrich task detail modal with full metadata

**Files**: modify `crates/roko-cli/src/tui/modals/task_detail.rs` (~177 LOC)
**Complexity**: standard
**What**: Extend the task detail modal to show all agent-visible fields, context
breakdown, and verbatim gate failure output.

**Steps**:
1. Read `TaskSpec` for the selected task (parse from tasks.toml or executor state)
2. Add sections to the modal:
   - **Metadata**: complexity band, category, estimated time, cost
   - **Files**: list of target files and context files
   - **Dependencies**: which tasks this depends on and which depend on it
   - **Acceptance**: formatted acceptance criteria with shell command highlighting
   - **Gate results**: verbatim gate output (compile errors, test failures, clippy warnings)
   - **Context budget**: bar chart showing token allocation per section (if context slice exists)
3. Add keybindings at the bottom: `[c] context` `[g] gate output` `[s] split` `[r] retry`
4. Increase modal width to accommodate the additional content

**Acceptance criteria**:
- Task detail modal shows complexity band, files, dependencies, and acceptance criteria
- Gate failure output is shown verbatim (not just pass/fail)
- Context budget breakdown is displayed as a bar chart if context slice data is available
- Modal renders correctly for all four complexity tiers

**Depends on**: T8


### T40: Add pause/resume and single-task retry from TUI

**Files**: modify `crates/roko-cli/src/tui/input.rs`,
  modify `crates/roko-cli/src/tui/state.rs`,
  modify `crates/roko-cli/src/runner/event_loop.rs`
**Complexity**: standard
**What**: Add keyboard shortcuts to pause/resume execution and retry a single failed
task from the TUI dashboard.

**Steps**:
1. Add `p` key handler in `input.rs` to toggle pause state:
   - Send a message to the event loop via the existing event channel
   - Update TUI state to show "PAUSED" in the header bar
2. In `event_loop.rs`, add a `pause` flag checked before dispatching new tasks:
   - When paused, do not dispatch new tasks but continue processing running ones
   - When unpaused, resume normal dispatch
3. Add `r` key handler on a focused failed task:
   - Send a retry request for the specific task to the event loop
   - The event loop resets the task status to pending and re-dispatches
4. Update header bar to show "PAUSED" state with the `p` toggle
5. Add `s` key handler to skip a focused pending/failed task

**Acceptance criteria**:
- Pressing `p` during execution pauses dispatch (running tasks finish)
- Pressing `p` again resumes
- Pressing `r` on a failed task retries it with gate failure context injected
- Pressing `s` on a pending task skips it and unblocks dependents
- Header bar shows "PAUSED" when paused

**Depends on**: T4


### T41: Add complexity tier indicators to plan tree widget

**Files**: modify `crates/roko-cli/src/tui/pages/operations.rs` (or relevant plan tree widget)
**Complexity**: fast
**What**: Show complexity tier and cost in the plan tree's task list.

**Steps**:
1. Read `complexity_band` from each task's metadata
2. Add tier indicator after the task title: `[trivial]`, `[fast]`, `[std]`, `[complex]`
3. Color-code by tier: green for trivial/fast, yellow for standard, red for complex
4. Add model name and cost when available: `haiku $0.01`, `sonnet $0.23`, `opus $1.47`
5. Format: `T1 [fast]   Scaffold widget module       done  sonnet $0.23`

**Acceptance criteria**:
- Plan tree shows `[trivial]`/`[fast]`/`[std]`/`[complex]` per task
- Tier indicators are color-coded
- Cost is shown when episode data is available

**Depends on**: T8


### T42: Add pack progress view to dashboard

**Files**: new `crates/roko-cli/src/tui/pages/packs.rs`,
  modify `crates/roko-cli/src/tui/mod.rs`,
  modify `crates/roko-cli/src/tui/tabs.rs`
**Complexity**: standard
**What**: Add a new tab or subtab showing context pack status and funnel progress.

**Steps**:
1. Create `packs.rs` page that reads pack manifests from `.roko/packs/`
2. Left panel: list of packs with status indicators (color-coded by PackStatus)
3. Right panel (when a pack is selected): per-pass progress, token counts, agent/model used
4. Show pass flow: `Synthesis -> Architecture -> Decomposition -> Scoping -> Execution`
   with checkmarks for completed passes and a spinner for the active pass
5. Register the page in `tabs.rs` and `mod.rs`
6. Assign to a subtab or repurpose an existing tab (e.g., Atelier/F9)

**Acceptance criteria**:
- The TUI shows pack list with status
- Selecting a pack shows per-pass progress
- Completed passes show checkmarks, active pass shows spinner
- Token counts and durations are displayed per pass

**Depends on**: T24


### T43: Add cost tracking to TUI header bar

**Files**: modify `crates/roko-cli/src/tui/dashboard.rs` (or header widget)
**Complexity**: fast
**What**: Add running cost display to the header bar during plan execution.

**Steps**:
1. Track cumulative cost from episode/cost events in TUI state
2. Add cost display to the header bar: `$X.XX` with color: green if under estimate,
   yellow if near estimate, red if over estimate
3. Format: `roko  W1/3  sprint-42  ||||..  8/10  80%  $6.42  ETA:5m  F1-F10`
4. If `--cost-cap` is set, show as `$6.42/$10.00`

**Acceptance criteria**:
- Header bar shows running cost during plan execution
- Cost updates in real-time as tasks complete
- Cost cap is shown when configured

**Depends on**: T35


### T44: Wire batch review modal to orchestrator

**Files**: modify `crates/roko-cli/src/tui/modals/batch_review.rs` (~164 LOC),
  modify `crates/roko-cli/src/runner/event_loop.rs`
**Complexity**: fast
**What**: The batch review modal exists (164 LOC) but has no trigger. Wire it to
appear when a wave completes, showing wave results before proceeding.

**Steps**:
1. In `event_loop.rs`, detect when all tasks in a wave have completed
2. Emit a `WaveCompleted` event with the wave summary (passed/failed/skipped counts)
3. In the TUI, when `WaveCompleted` is received:
   - If `--approve-each` or a batch review is configured, show the `BatchReview` modal
   - Modal shows per-task results for the wave
   - User can approve (continue), retry failed tasks, or abort
4. If not in interactive mode, log the wave summary and continue

**Acceptance criteria**:
- Batch review modal appears at wave boundaries when configured
- Modal shows per-task pass/fail for the completed wave
- User can approve, retry, or abort from the modal
- Non-interactive mode logs wave summary without modal

**Depends on**: T40

---

## Phase 8: Progressive Context Refinement & Polish

**Problem**: The funnel passes produce artifacts but the execution phase does not use
per-task context slices. Prior task outputs do not feed into subsequent task context.

**Effort**: ~700 LOC new code | **Impact**: Medium
**Dependencies**: Phase 4 (T30 for context slicing)


### T45: Wire context slices into agent dispatch

**Files**: modify `crates/roko-cli/src/runner/event_loop.rs`,
  modify `crates/roko-compose/src/prompt_assembly_service.rs`
**Complexity**: standard
**What**: When a pack has scoped context slices, use the per-task slice instead of
monolithic context assembly. The slice provides exactly the context the task needs.

**Steps**:
1. In `event_loop.rs`, when dispatching a task, check if a context slice exists at
   `.roko/packs/<pack-id>/scoping/T{id}-context.toml`
2. If it exists, read the `ContextSlice` and construct the system prompt using only
   the sections in the slice (respecting their token budgets)
3. Pass the slice content to `PromptAssemblyService` as a pre-assembled context block
4. If no slice exists, fall back to the standard prompt assembly path (backward compatible)
5. Log which context source was used: "Using scoped context (34K tokens)" vs
   "Using standard assembly (estimated 45K tokens)"

**Acceptance criteria**:
- Tasks with context slices use the slice for prompt assembly
- Tasks without context slices use standard assembly (backward compatible)
- Log output shows which context source was used
- Prompt token count is within the slice's budget

**Depends on**: T30, T11


### T46: Feed prior task outputs into subsequent task context

**Files**: modify `crates/roko-cli/src/runner/event_loop.rs`,
  modify `crates/roko-cli/src/runner/state.rs`
**Complexity**: standard
**What**: When task B depends on task A, include a summary of task A's output in
task B's context. This enables chain-of-work reasoning.

**Steps**:
1. In `state.rs`, add `task_outputs: HashMap<String, String>` to executor state
2. After a task completes successfully, capture its output (the agent's final response
   or the git diff if available) and store in `task_outputs`
3. Truncate stored output to 2K tokens per task (use the estimation function)
4. When assembling context for a downstream task:
   - For each `depends_on` entry, look up the stored output
   - Include as: "Prior work (T{id}): {truncated output}"
   - Budget: up to 15% of the task's context budget for dependency outputs
5. Persist `task_outputs` in the executor snapshot for resume support

**Acceptance criteria**:
- Task B's prompt includes a summary of task A's output when B depends on A
- Prior task outputs are truncated to 2K tokens each
- Total dependency output context respects the 15% budget cap
- Outputs persist across resume

**Depends on**: T4, T9


### T47: Add inline progress output for non-TUI execution

**Files**: modify `crates/roko-cli/src/runner/event_loop.rs`
**Complexity**: fast
**What**: For headless/SSH sessions without TUI, show inline progress as tasks
complete. Update a single progress line (or append lines for non-TTY).

**Steps**:
1. Detect TTY vs non-TTY at run start
2. For TTY without TUI: use `\r` to overwrite a progress line:
   `[W1] T3 implementing... (3/10, $2.14, 5:23 elapsed)`
3. When a task completes, print a completion line:
   `[OK] T3: Scaffold widget module (sonnet, 23s, $0.23)`
   or `[FAIL] T6: Wire auth flow (opus, 45s, $1.47, gate: clippy)`
4. For non-TTY: print each completion as a new line (no `\r`)
5. At end of run, print the full RunSummary from T4

**Acceptance criteria**:
- TTY mode shows a live-updating progress line
- Each task completion is printed as a one-line summary
- Non-TTY mode (piped output, CI) prints one line per event with no control characters
- End-of-run summary appears in all modes

**Depends on**: T4

---

## Dependency Graph

```
Phase 0 (T1-T7): Foundation -- no dependencies
  |
  +---> Phase 1 (T8-T12): Metadata tiering + context windowing
  |       |
  |       +---> Phase 2 (T13-T19): Task TOML validation
  |       |       |
  |       |       +---> Phase 3 (T20-T23): Task auto-splitting
  |       |       |       |
  |       |       |       +---> Phase 4 (T24-T31): Context packs & corpus
  |       |       |               |
  |       |       |               +---> Phase 5 (T32-T35): Interactive steering
  |       |       |               |
  |       |       |               +---> Phase 8 (T45-T47): Progressive refinement
  |       |       |
  |       |       +---> Phase 6 (T36-T38): Mori-style ingestion
  |       |
  |       +---> Phase 7 (T39-T44): TUI improvements
  |
  Phase 5 (T34-T35): approve-each, cost-cap -- depends on T4 only
```

**Critical path**: T1-T4 -> T8-T11 -> T13-T19 -> T20-T23 -> T24-T31 -> T45-T47

**Parallel workstreams**:
- T39-T44 (TUI) can proceed after T8 is done
- T34-T35 (approve-each, cost-cap) can proceed after T4 is done
- T36-T38 (Mori-style ingestion) can proceed after T19 is done

---

## Effort Estimates

| Phase | Tasks | New LOC | Description |
|---|---|---|---|
| Phase 0: Foundation | T1-T7 | ~1,200 | `roko next`, run summaries, dry-run, help text |
| Phase 1: Metadata + Context | T8-T12 | ~1,800 | TaskSpec, ContextBudget, windowing, section pruning |
| Phase 2: Validation | T13-T19 | ~1,000 | Circular deps, file conflicts, acceptance format, pre-flight |
| Phase 3: Auto-splitting | T20-T23 | ~1,100 | File grouping, import chains, acceptance distribution, CLI |
| Phase 4: Context Packs | T24-T31 | ~2,800 | Pack data model, 4 funnel passes, pipeline, corpus mgmt |
| Phase 5: Interactive Steering | T32-T35 | ~900 | `roko ingest`, per-pass re-run, approve-each, cost-cap |
| Phase 6: Mori Ingestion | T36-T38 | ~1,000 | Plan ingest, filesystem watcher, cross-plan GlobalTaskId |
| Phase 7: TUI Improvements | T39-T44 | ~1,200 | Task detail, pause/resume, tier indicators, pack view, cost, batch review |
| Phase 8: Progressive Refinement | T45-T47 | ~700 | Context slice wiring, prior task outputs, inline progress |
| **Total** | **T1-T47** | **~11,700** | |

---

## Risk Mitigations

### R1: Another parallel universe of types (TaskSpec vs existing task types)

**Risk**: `TaskSpec` becomes yet another task struct alongside existing types in
roko-orchestrator and the plan runner.
**Mitigation**: `TaskSpec` replaces existing ad-hoc task parsing. Migration: update
`plan_loader.rs` to parse into `TaskSpec`, deprecate the old path. Do not add
`TaskSpec` as a second type -- make it THE type.

### R2: Funnel workflow is too expensive (5 agent calls per funnel run)

**Risk**: At ~$2 per Opus call, a full funnel costs $10 before any execution.
**Mitigation**: Use Sonnet for passes 1-4 (synthesis, architecture, decomposition,
scoping). Use Opus only for execution. Allow `--model` override per pass. Allow
skipping passes that the user does manually. Total funnel cost target: ~$2-4.

### R3: Context windowing breaks working prompts

**Risk**: Existing prompts that work with full context may break when context is
reduced for fast tasks.
**Mitigation**: Default all existing tasks to `Standard` complexity band (current
behavior). Only apply slim/deep context for tasks that explicitly set `context_weight`
or `complexity_band`. Backward compatibility is preserved by default.

### R4: Validation blocks legitimate runs

**Risk**: Overly strict validation prevents runs on valid-but-unusual plans.
**Mitigation**: Only `Error` severity blocks execution. `Warning` severity is shown
but does not block. `--skip-validation` flag is always available.

### R5: Filesystem watcher race conditions during mid-run editing

**Risk**: Editing tasks.toml while a task is running could cause inconsistent state.
**Mitigation**: Debounce watcher events (500ms). Never modify a task that is currently
executing. Only apply changes to pending/blocked tasks. Lock the running task's entry.

### R6: Pack pipeline produces low-quality tasks.toml

**Risk**: Agent-generated tasks.toml from the decomposition pass may be poorly structured.
**Mitigation**: Run full validation (Phase 2) on the generated tasks.toml automatically.
Show validation results to the user. Allow editing before execution. The pipeline is
a tool, not a replacement for judgment.

---

## File Paths Summary

### New Files

| File | Phase | Purpose |
|---|---|---|
| `crates/roko-cli/src/commands/next.rs` | 0 | `roko next` command |
| `crates/roko-cli/src/runner/run_summary.rs` | 0 | End-of-run summary type |
| `crates/roko-orchestrator/src/task_spec.rs` | 1 | TaskSpec + TaskAgentInput types |
| `crates/roko-compose/src/context_budget.rs` | 1 | ContextBudget per-section allocation |
| `crates/roko-orchestrator/src/validation.rs` | 2 | Validation engine and checks |
| `crates/roko-orchestrator/src/task_split.rs` | 3 | Task splitting engine |
| `crates/roko-cli/src/commands/task.rs` | 3 | `roko task split` command |
| `crates/roko-cli/src/pack.rs` | 4 | Context pack data model |
| `crates/roko-cli/src/pack_pipeline.rs` | 4 | Funnel pass execution |
| `crates/roko-cli/src/commands/pack.rs` | 4 | `roko pack` CLI commands |
| `crates/roko-cli/src/context_slicer.rs` | 4 | Per-task context windowing |
| `crates/roko-cli/src/runner/plan_watcher.rs` | 6 | Filesystem watcher for mid-run editing |
| `crates/roko-cli/src/tui/pages/packs.rs` | 7 | Pack progress TUI page |

### Modified Files

| File | Phase(s) | Change |
|---|---|---|
| `crates/roko-cli/src/main.rs` | 0,3,4,5 | Add Command variants (Next, Task, Pack, Ingest) |
| `crates/roko-cli/src/commands/mod.rs` | 0,3,4 | Re-export new modules |
| `crates/roko-cli/src/commands/plan.rs` | 0,2,5,6 | --dry-run, --approve-each, --cost-cap, Ingest, validation wiring |
| `crates/roko-cli/src/commands/status.rs` | 0 | --last-run flag |
| `crates/roko-cli/src/unified.rs` | 0 | Bare invocation shows `next` |
| `crates/roko-cli/src/runner/event_loop.rs` | 0,1,2,5,6,7,8 | Run summary, TaskSpec dispatch, validation pre-flight, approve-each, cost-cap, pause, watcher, context slices, prior outputs, inline progress |
| `crates/roko-cli/src/runner/plan_loader.rs` | 1,6 | Parse into TaskSpec, ingest command |
| `crates/roko-cli/src/runner/task_dag.rs` | 0 | Execution wave computation for dry-run |
| `crates/roko-cli/src/runner/state.rs` | 8 | Task output storage |
| `crates/roko-orchestrator/src/lib.rs` | 1,2,3 | Re-export new modules |
| `crates/roko-orchestrator/src/dag.rs` | 6 | GlobalTaskId cross-plan edges |
| `crates/roko-compose/src/lib.rs` | 1 | Re-export context_budget |
| `crates/roko-compose/src/system_prompt_builder.rs` | 1 | ContextBudget enforcement |
| `crates/roko-compose/src/prompt_assembly_service.rs` | 1,8 | Section pruning, context slice wiring |
| `crates/roko-cli/src/tui/modals/task_detail.rs` | 7 | Full metadata display |
| `crates/roko-cli/src/tui/modals/batch_review.rs` | 7 | Wire to orchestrator |
| `crates/roko-cli/src/tui/input.rs` | 7 | Pause/resume/retry keybindings |
| `crates/roko-cli/src/tui/state.rs` | 7 | Pause state, cost tracking |
| `crates/roko-cli/src/tui/dashboard.rs` | 7 | Cost in header bar |
| `crates/roko-cli/src/tui/tabs.rs` | 7 | Register packs page |
| `crates/roko-cli/src/tui/mod.rs` | 7 | Register packs page |
| `crates/roko-cli/src/tui/pages/operations.rs` | 7 | Tier indicators in plan tree |

---

## Sources

Existing infrastructure this plan builds on:

- `crates/roko-cli/src/main.rs` (~4,423 LOC) -- CLI entry, Command enum
- `crates/roko-cli/src/runner/event_loop.rs` (~3,136 LOC) -- Plan runner V2 event loop
- `crates/roko-cli/src/runner/plan_loader.rs` (~153 LOC) -- Plan loading and parsing
- `crates/roko-cli/src/runner/task_dag.rs` (~554 LOC) -- Task DAG computation
- `crates/roko-cli/src/runner/state.rs` (~511 LOC) -- Executor state
- `crates/roko-cli/src/runner/persist.rs` (~472 LOC) -- State persistence
- `crates/roko-cli/src/runner/gate_dispatch.rs` (~323 LOC) -- Gate execution
- `crates/roko-cli/src/commands/plan.rs` (~1,317 LOC) -- Plan command handlers
- `crates/roko-cli/src/commands/status.rs` -- Status command
- `crates/roko-cli/src/unified.rs` (~212 LOC) -- Bare invocation handler
- `crates/roko-cli/src/chat_session.rs` (~2,747 LOC) -- Chat session
- `crates/roko-cli/src/repo_context.rs` -- Repository context builder
- `crates/roko-compose/src/system_prompt_builder.rs` (~2,081 LOC) -- 9-layer prompt builder
- `crates/roko-compose/src/prompt_assembly_service.rs` (~1,048 LOC) -- Prompt assembly
- `crates/roko-orchestrator/src/dag.rs` (~2,557 LOC) -- DAG computation
- `crates/roko-orchestrator/src/lib.rs` (~108 LOC) -- Orchestrator exports
- `crates/roko-runtime/src/workflow_engine.rs` (~1,678 LOC) -- V2 workflow engine
- `crates/roko-cli/src/tui/modals/task_detail.rs` -- Task detail modal (extend)
- `crates/roko-cli/src/tui/modals/batch_review.rs` (~164 LOC) -- Batch review (wire)
- `crates/roko-cli/src/tui/fs_watch.rs` -- Existing filesystem watcher (reference)
