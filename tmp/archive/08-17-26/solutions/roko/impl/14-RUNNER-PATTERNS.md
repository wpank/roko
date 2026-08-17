# Implementation Plan 14: Runner Patterns -- Native Integration

> Bring the mega-parity runner's proven patterns into roko's core execution
> infrastructure. These patterns were validated across ~195 parallel batches,
> ~6 hours of wall time, and 177K LOC. The shell scripts worked; now the
> Rust crates must deliver the same behavior natively.
>
> **Source documents**: 22-RUNNER-LESSONS.md, 17-ORCH-PATTERNS.md, 01-LESSONS-AND-APPROACHES.md
>
> **Core thesis**: The mega-parity runner IS roko's self-hosting loop, in bash
> form. Every pattern here has been paid for in hours of operational pain.
> Wire them into the existing crates; do not reinvent.

---

## Existing Infrastructure (Do Not Rebuild)

| Component | Location | Status |
|---|---|---|
| WorktreeManager | `crates/roko-orchestrator/src/worktree.rs` | Built, branch naming + health checks + idle reclaim |
| MergeQueue | `crates/roko-orchestrator/src/merge_queue.rs` | Built, file-overlap-aware serialized merges |
| UnifiedTaskDag | `crates/roko-orchestrator/src/dag.rs` | Built, waves + CPM + fusion + file-overlap inference |
| TaskScheduler | `crates/roko-runtime/src/task_scheduler.rs` | Built, pure DAG resolver, max_parallel support |
| PostMergeRunner | `crates/roko-orchestrator/src/post_merge.rs` | Built, regression detection after merge |
| PipelineStateV2 | `crates/roko-runtime/src/pipeline_state.rs` | Built, pure state machine with iteration |
| EffectDriver | `crates/roko-runtime/src/effect_driver.rs` | Built, side-effect executor for pipeline |
| WorkflowEngine | `crates/roko-runtime/src/workflow_engine.rs` | Built, ties pipeline + effect driver |
| ReplanStrategy | `crates/roko-orchestrator/src/replan.rs` | Built, failure disposition + revision evidence |
| PromptAssemblyService | `crates/roko-compose/src/prompt_assembly_service.rs` | Built, 9-layer prompt builder |
| RuntimeSnapshot | `crates/roko-orchestrator/src/runtime_snapshot.rs` | Built, checkpoint/resume state |

---

## Phase 0: Worktree Isolation (Tasks 1-5)

The foundation: each task gets its own git worktree, never a shared working
directory. The mega-parity runner proved this eliminates file corruption,
stale reads, git index conflicts, and build cache invalidation.

### Task 1: Wire WorktreeManager into WorkflowEngine for per-task worktrees

**File**: `crates/roko-runtime/src/workflow_engine.rs`
**Also reads**: `crates/roko-orchestrator/src/worktree.rs`

**What**: When WorkflowEngine dispatches a task, allocate a worktree via
WorktreeManager and set the agent's `current_dir` to the worktree path.
Currently, all tasks share the main repo working directory.

**Steps**:
1. Add `worktree_manager: Option<Arc<WorktreeManager>>` field to `WorkflowEngine`
2. In `WorkflowRunConfig`, add `worktree_isolation: bool` (default false for backward compat)
3. When `worktree_isolation` is true and dispatching a task, call
   `worktree_manager.create(task_id, None).await?` to get a `WorktreeHandle`
4. Pass `handle.path` as the working directory to `EffectDriver::dispatch_agent()`
5. After task completes (success or failure), call `worktree_manager.touch(&task_id)`
   but do NOT remove the worktree (preserved for inspection per project rules)
6. Store active `WorktreeHandle`s in a `HashMap<String, WorktreeHandle>` for resume

**Acceptance criteria**:
- `roko plan run` with `worktree_isolation = true` creates worktrees under `.roko/worktrees/`
- Each task agent operates in its own worktree directory
- Worktrees survive after task completion (never auto-deleted)
- `roko plan run` without the flag works exactly as before (no regression)

### Task 2: Implement three-tier branch model in WorktreeManager

**File**: `crates/roko-orchestrator/src/worktree.rs`

**What**: Implement the source/integration/task branch hierarchy from the
mega-parity runner. Currently `format_branch_name()` creates task branches but
there is no integration branch concept.

**Steps**:
1. Add `create_integration_branch(&self, run_id: &str) -> Result<String>` method
   that creates a branch named `roko/run-{run_id}` from the current HEAD
2. Modify `create()` to accept an optional `base_branch: Option<&str>` parameter;
   when provided, fork the worktree from that branch instead of `self.config.base_branch`
3. Add `integration_branch: Option<String>` field to `WorktreeConfig` so all
   task worktrees in a run fork from the same integration branch
4. Add `backup_branch(&self, task_id: &str) -> Result<String>` that creates
   `roko/{run_id}-{task_id}-backup-{timestamp}` for retry preservation
5. Update `format_branch_name()` to use the pattern `roko/{run_id}-{task_id}`

**Acceptance criteria**:
- Integration branch is created once per plan run
- All task worktrees fork from the integration branch, not the source branch
- Backup branches are created on retry with timestamp suffix
- `git branch --list 'roko/*'` shows the expected hierarchy after a run

### Task 3: Wire serialized merge via MergeQueue into WorkflowEngine

**File**: `crates/roko-runtime/src/workflow_engine.rs`
**Also reads**: `crates/roko-orchestrator/src/merge_queue.rs`

**What**: After a task completes successfully, enqueue its changes for
serialized merge into the integration branch via MergeQueue. Currently,
MergeQueue is built but never called from the runtime.

**Steps**:
1. Add `merge_queue: Option<Arc<MergeQueue>>` field to `WorkflowEngine`
2. After a task reaches the `Completed` phase, collect its `files_changed` from
   the worktree diff: `git diff --name-only HEAD~1` in the worktree
3. Construct `MergeRequest { plan_id: task_id, branch_name, files_changed, priority }`
4. Call `merge_queue.enqueue(request)` to add to the queue
5. Spawn a background merge loop: `merge_queue.next_ready()` -> execute
   `git merge --no-ff <branch>` in the integration worktree -> `merge_queue.complete(id)`
6. On merge conflict, call `merge_queue.fail(id)` with the conflict details

**Acceptance criteria**:
- Task merges are serialized (no concurrent `git merge` operations)
- Tasks touching disjoint files can merge without waiting for each other
- Tasks touching overlapping files are serialized by the queue
- Merge conflicts are recorded and reported, not silently swallowed

### Task 4: Add worktree disk space monitoring

**File**: `crates/roko-orchestrator/src/worktree.rs`

**What**: Monitor available disk space before creating worktrees. The mega-parity
runner showed that 15 worktrees at 500MB each = 7.5GB, and cargo builds can
add 5-15GB per worktree. Pause dispatch when space is low.

**Steps**:
1. Add `check_disk_space(path: &Path) -> Result<DiskStatus>` function that
   returns available bytes at the given mount point (use `statvfs` on Unix)
2. Define `DiskStatus { available_bytes: u64, total_bytes: u64 }`
3. Add `min_disk_bytes: u64` to `WorktreeConfig` (default 5GB)
4. In `create()`, call `check_disk_space()` before creating. Return
   `WorktreeError::InsufficientDisk { available, required }` when below threshold
5. Add `estimate_worktree_size(&self) -> u64` method based on existing worktree
   sizes (or a configured default of 500MB)
6. Emit a `RuntimeEvent::DiskWarning` when available space < 2x the worktree estimate

**Acceptance criteria**:
- `WorktreeManager::create()` fails gracefully when disk is below threshold
- A warning event is emitted when disk space is low but not critical
- The disk check does not block or slow normal operations (< 1ms)

### Task 5: Add worktree cleanup utilities (non-destructive)

**File**: `crates/roko-orchestrator/src/worktree.rs`

**What**: Add methods for cleaning up worktree artifacts (build caches, temporary
files) without deleting worktrees or branches. The mega-parity runner showed
that incremental build caches are the primary disk consumer.

**Steps**:
1. Add `clean_build_cache(&self, task_id: &str) -> Result<u64>` that removes
   `target/` directories within the worktree, returns bytes freed
2. Add `clean_all_build_caches(&self) -> Result<u64>` for batch cleanup
3. Add `worktree_sizes(&self) -> Result<Vec<(String, u64)>>` for monitoring
4. Do NOT add any method that deletes worktrees or branches automatically
   (project rule: never delete worktrees or branches)
5. Add `stale_lock_cleanup(&self)` that removes `.git/index.lock` files older
   than `STALE_LOCK_SECS` (already partially implemented via `clear_stale_locks`)

**Acceptance criteria**:
- `clean_build_cache()` removes only `target/` directories, nothing else
- Worktree source files and git history are never touched
- `worktree_sizes()` returns accurate sizes for monitoring display

---

## Phase 1: Wave Gating (Tasks 6-10)

The core throughput optimization: defer compilation to wave boundaries instead
of per-task verification. 10-100x speed improvement measured in production.

### Task 6: Add WaveGatePhase to PipelineStateV2

**File**: `crates/roko-runtime/src/pipeline_state.rs`

**What**: Extend the pipeline state machine with a `WaveGating` phase that
accumulates completed tasks and triggers gates at wave boundaries instead of
per-task. Currently PipelineStateV2 gates every task individually.

**Steps**:
1. Add `WaveGating` variant to the `Phase` enum
2. Add `wave_gate_mode: WaveGateMode` to `WorkflowConfig`:
   ```rust
   pub enum WaveGateMode {
       PerTask,       // current behavior
       PerWave,       // gate after each wave completes
       Deferred,      // gate only at end of plan
   }
   ```
3. When `wave_gate_mode` is `PerWave`, the state machine transitions from
   `Implementing` to `WaveGating` only when all tasks in the current wave
   have reached `AgentCompleted`
4. In `WaveGating`, emit `PipelineOutput::RunGates` once for the entire wave
5. On gate success, transition to dispatching the next wave
6. On gate failure, use bisection logic to identify the offending task (see Task 10)

**Acceptance criteria**:
- `WaveGateMode::PerTask` produces identical behavior to current (no regression)
- `WaveGateMode::PerWave` runs gates once per wave, not per task
- Gate results are attributed to the wave, not individual tasks
- State machine transitions are covered by unit tests

### Task 7: Implement no-build context injection

**File**: `crates/roko-compose/src/prompt_assembly_service.rs`

**What**: When wave gating is active, inject a "do not compile" instruction
into the system prompt. The mega-parity runner proved this reduces task time
from 15-40 minutes to 1-5 minutes with ~95% agent compliance.

**Steps**:
1. Add a `build_policy: BuildPolicy` field to the prompt spec:
   ```rust
   pub enum BuildPolicy {
       Allowed,     // agent may compile and test
       Prohibited,  // agent must not run any build commands
   }
   ```
2. When `BuildPolicy::Prohibited`, inject the following as a high-priority
   system prompt section (layer 1, before task description):
   ```
   IMPORTANT: Do NOT run `cargo build`, `cargo check`, `cargo test`, `cargo clippy`,
   or any other compilation command. The runner will verify your changes at the wave
   gate. Focus only on writing correct code.
   ```
3. Place this in layer 1 (not a context file) per the lesson that system prompt
   placement achieves 99% compliance vs 95% for context files
4. When `WaveGateMode::PerWave` or `Deferred`, automatically set `BuildPolicy::Prohibited`
5. Add an override `force_build_policy: Option<BuildPolicy>` in task config

**Acceptance criteria**:
- Agents dispatched under wave gating receive the no-build instruction
- The instruction appears early in the system prompt (layer 1)
- Agents dispatched without wave gating do NOT receive the instruction
- Per-task override allows specific tasks to build when needed

### Task 8: Wire wave gate execution in EffectDriver

**File**: `crates/roko-runtime/src/effect_driver.rs`
**Also reads**: `crates/roko-orchestrator/src/post_merge.rs`

**What**: When the state machine emits `RunGates` for a wave, execute the
gates against the integration branch (where all wave task merges accumulated),
not against individual worktrees.

**Steps**:
1. Add `run_wave_gate(&self, integration_dir: &Path, gate_configs: &[GateConfig]) -> Result<Vec<GateVerdict>>` method
2. The method runs in the integration worktree directory, which has all merged task changes
3. Execute configured gates in order: compile -> clippy -> custom shell -> test
4. Collect all verdicts and return them as a batch
5. Emit `RuntimeEvent::WaveGateResult { wave_index, verdicts, duration_ms }` per wave
6. If any gate fails, include the raw output for failure attribution (Task 10)

**Acceptance criteria**:
- Wave gates run in the integration worktree, not individual task worktrees
- Gate output includes enough information to identify which task caused a failure
- Wave gate duration is tracked and reported
- All configured gates (compile, clippy, test, custom) are supported

### Task 9: Add configurable gate deferral to plan run CLI

**File**: `crates/roko-cli/src/run.rs` (or wherever `plan run` is wired)
**Also reads**: `crates/roko-runtime/src/pipeline_state.rs`

**What**: Expose the wave gate mode as CLI flags on `roko plan run`, matching
the mega-parity runner's `--no-gate`, `--no-test` flags.

**Steps**:
1. Add `--gate-mode <per-task|per-wave|deferred>` flag (default: `per-task`)
2. Add `--no-gate` shorthand (equivalent to `--gate-mode deferred`)
3. Map CLI flags to `WorkflowConfig::wave_gate_mode`
4. Also add `--no-build` flag that forces `BuildPolicy::Prohibited` regardless
   of gate mode (for runs where you want manual verification only)
5. Add `[execution.gate_mode]` section to `roko.toml` for persistent defaults
6. CLI flags override TOML config

**Acceptance criteria**:
- `roko plan run --gate-mode per-wave` uses wave-level gating
- `roko plan run --no-gate` defers all gating to end of run
- `roko plan run` without flags uses per-task gating (backward compatible)
- Config from `roko.toml` is used when no CLI flag is provided

### Task 10: Implement wave gate failure bisection

**File**: `crates/roko-runtime/src/workflow_engine.rs`
**Also reads**: `crates/roko-orchestrator/src/dag.rs`

**What**: When a wave gate fails, determine which task(s) in the wave caused
the regression. The mega-parity runner used `git log` + bisect across merge
commits. Implement the same logic natively.

**Steps**:
1. Add `bisect_wave_failure(integration_dir: &Path, wave_task_ids: &[String], gate_configs: &[GateConfig]) -> Result<Vec<String>>`
2. Retrieve the list of merge commits in the wave from `git log --merges`
3. For each merge commit, check if reverting it fixes the gate failure:
   - `git revert --no-commit <merge_sha>` -> run gates -> `git reset --hard`
4. Return the task IDs whose merge commits, when reverted, fix the failure
5. If bisection finds the offending task(s), mark them for retry with
   the gate failure output as context
6. Log the bisection process via `RuntimeEvent::WaveBisection { wave, offending_tasks }`

**Acceptance criteria**:
- Bisection correctly identifies the task that introduced a compile error
- The offending task is retried with failure context from the gate output
- Non-offending tasks in the wave are not retried
- Bisection works for multiple simultaneous offenders in the same wave

---

## Phase 2: Context Handoff (Tasks 11-15)

The hardest problem: telling agent B what agent A changed. The cumulative
context pattern reduced merge conflicts by ~40%.

### Task 11: Build cumulative context section generator

**File**: `crates/roko-compose/src/prompt_assembly_service.rs` (new helper module)

**What**: Generate the "What Changed Before You" section that shows each task
what files were modified by prior tasks in the plan.

**Steps**:
1. Add `pub fn cumulative_context(completed_tasks: &[CompletedTaskSummary], token_budget: usize) -> String`
2. `CompletedTaskSummary` contains: `task_id`, `files_changed: Vec<(String, i32, i32)>` (path, lines_added, lines_removed), `brief_description`
3. Format as markdown:
   ```
   ## What Changed Before You
   Files modified by prior tasks in this plan:
   - `src/gate/compile.rs` (+45 -12): Added run_compile_gate, modified gate_pipeline
   - `src/lib.rs` (+3 -0): Added `pub mod gate;`
   ```
4. When total tokens exceed `token_budget` (default 4000), truncate oldest
   task summaries first, keeping the most recent changes visible
5. For large files, use signature-only views (function name + params, no body)
6. Track byte count via `TokenCounter` from the compose crate

**Acceptance criteria**:
- Cumulative section is generated from completed task data
- Token budget is respected (never exceeds 4000 tokens default)
- Oldest entries are truncated first when budget is exceeded
- Format matches the mega-parity runner's cumulative section format

### Task 12: Wire cumulative context into agent dispatch

**File**: `crates/roko-runtime/src/effect_driver.rs`

**What**: Before dispatching each task's agent, generate the cumulative context
section from all previously completed tasks in the plan and inject it into
the prompt.

**Steps**:
1. Track `completed_summaries: Vec<CompletedTaskSummary>` in `EffectDriver` or
   pass it through the pipeline state
2. After each task completes, collect its changed files via `git diff --stat`
   in the task's worktree and append to `completed_summaries`
3. Before dispatching a new task, call `cumulative_context(&completed_summaries, 4000)`
4. Inject the result as a prompt section via `PromptAssemblyService` (layer 5,
   "contextual knowledge" layer)
5. Also inject the list of files the *current* task will modify (from task config)
   so the agent knows to check those files against prior changes

**Acceptance criteria**:
- Each dispatched agent receives a cumulative section with prior task changes
- The section grows as more tasks complete in the plan
- Token budget prevents the section from consuming too much context
- First task in a plan receives an empty cumulative section (no prior work)

### Task 13: Implement failure context accumulation for retries

**File**: `crates/roko-runtime/src/pipeline_state.rs`

**What**: When a task fails and is retried, accumulate structured failure
context (gate output, diff, error pattern) so the retry agent has full
information about what went wrong.

**Steps**:
1. Add `FailureContext` struct to pipeline_state:
   ```rust
   pub struct FailureContext {
       pub attempt: u32,
       pub gate_name: String,
       pub gate_output: String,         // truncated to 2000 chars
       pub diff_from_prior: String,     // what the agent changed
       pub error_pattern: Option<String>,
       pub suggested_fix: Option<String>,
   }
   ```
2. Add `failure_history: Vec<FailureContext>` to `PipelineStateV2`
3. On gate failure that triggers retry, construct `FailureContext` from the
   gate verdict and agent output
4. When dispatching the retry, include `failure_history` in the prompt as a
   "Previous Attempts" section
5. Format clearly: "Attempt 1 failed because: [gate output]. Your changes: [diff]."

**Acceptance criteria**:
- Retry attempts receive full context from all prior failures
- Failure context is truncated to prevent exceeding token budgets
- The failure history survives checkpoint/resume (serializable)
- Third attempt includes context from both attempt 1 and attempt 2

### Task 14: Implement structured handoff documents for multi-role workflows

**File**: `crates/roko-compose/src/prompt_assembly_service.rs`

**What**: For multi-pass workflows (strategist -> implementer -> reviewer),
generate structured handoff documents instead of passing raw strings.
PipelineStateV2 carries `strategist_brief` and `review_findings` as
unstructured strings; make them structured.

**Steps**:
1. Add `StrategyBrief` struct: `approach`, `key_constraints`, `files_to_modify`,
   `files_not_to_modify`, `estimated_complexity`
2. Add `ReviewFindings` struct: `must_fix: Vec<Finding>`, `nits: Vec<Finding>`
   where `Finding` has `file`, `line`, `description`
3. In `PromptAssemblyService`, add `format_strategy_brief(brief: &StrategyBrief) -> String`
   and `format_review_findings(findings: &ReviewFindings) -> String`
4. Update PipelineStateV2 to use these types instead of raw `Option<String>`
5. Parse agent output into these structures using regex patterns for common
   formats (numbered lists, file:line patterns)

**Acceptance criteria**:
- Strategist output is parsed into `StrategyBrief` with structured fields
- Review findings are parsed into `must_fix` and `nit` categories
- Implementer receives formatted brief with clear scope boundaries
- Fallback to raw string when parsing fails (no crash on unusual formats)

### Task 15: Add context-pack file support for shared agent knowledge

**File**: `crates/roko-compose/src/prompt_assembly_service.rs`

**What**: Support the mega-parity runner's context-pack pattern: a directory
of markdown files prepended to every agent prompt. Rules, architecture,
anti-patterns, performance contracts.

**Steps**:
1. Add `context_pack_dir: Option<PathBuf>` to `PromptAssemblyService` config
2. If set, read all `*.md` files from the directory, sorted by filename
   (00-RULES.md first, 05-NO-BUILD.md last)
3. Concatenate into a single context string with file separators
4. Inject as system prompt layer 2 (after role identity, before domain knowledge)
5. Track total token count; warn if context pack exceeds 8000 tokens
6. Support per-plan overrides: `context_pack_dir` in plan config overrides global
7. Add `[execution.context_pack_dir]` to `roko.toml`

**Acceptance criteria**:
- Files in `context-pack/` directory are injected into every agent prompt
- Files are ordered by filename (numeric prefix sorting)
- Total context pack size is logged for monitoring
- Warning emitted when pack exceeds 8000 tokens
- Per-plan override works when specified in plan config

---

## Phase 3: Anti-Pattern Pre-Gates (Tasks 16-19)

Fast grep-based checks that catch common LLM mistakes in milliseconds, before
any compilation. The mega-parity runner ran 10 anti-pattern checks; they caught
structural mistakes that compilation would miss.

### Task 16: Implement AntiPatternChecker with configurable rules

**File**: `crates/roko-gate/src/anti_pattern.rs` (new file)

**What**: A fast, grep-based checker that scans agent output for known LLM
code-generation anti-patterns. Runs in milliseconds, no compilation needed.

**Steps**:
1. Define `AntiPatternRule`:
   ```rust
   pub struct AntiPatternRule {
       pub id: String,              // e.g. "AP-1"
       pub name: String,            // e.g. "stub_gate_silent_pass"
       pub pattern: Regex,          // compiled regex
       pub description: String,
       pub severity: Severity,      // Error, Warning
       pub file_glob: Option<String>, // restrict to matching files
       pub exemptions: Vec<String>,   // paths exempt from this rule
   }
   ```
2. Implement the 10 checks from the mega-parity runner:
   - AP-1: Stub gates that return pass (silent-pass)
   - AP-2: `block_on` in async code
   - AP-3: Duplicate trait definitions vs foundation.rs
   - AP-5: Raw `Command::new("claude")` shell-outs
   - AP-6: Inline prompt strings (`format!("You are a...")`)
   - AP-7: `std::sync::Mutex` held across `.await`
   - AP-8: Empty function bodies
   - AP-9: `unimplemented!/unreachable!` left behind
   - AP-10: Hardcoded localhost/port in non-test code
3. Add `AntiPatternChecker::check(files: &[PathBuf]) -> Vec<AntiPatternViolation>`
4. Support per-task exemptions via task config `ap_exemptions: ["AP-10"]`

**Acceptance criteria**:
- All 10 anti-pattern checks execute in < 100ms for a typical task diff
- Each violation includes: rule ID, file, line number, matched text
- Exemptions work per-task (AP-10 can be exempted for config files)
- False positive rate tracked via violation metadata

### Task 17: Wire AntiPatternChecker as a pre-gate in the pipeline

**File**: `crates/roko-runtime/src/effect_driver.rs`
**Also reads**: `crates/roko-gate/src/anti_pattern.rs`

**What**: Run anti-pattern checks after agent completion but before compilation
gates. This catches structural mistakes without waiting for `cargo check`.

**Steps**:
1. After `AgentCompleted` event, before transitioning to gate phase, run
   `AntiPatternChecker::check()` on the files changed by the agent
2. If any `Severity::Error` violations are found, treat as a gate failure:
   inject violation details into the retry context and re-dispatch
3. If only `Severity::Warning` violations, log them but continue to gates
4. In wave-gate mode, run anti-pattern checks per-task (they are fast enough)
   even when compilation is deferred to wave boundaries
5. Track AP check duration in `RuntimeEvent::AntiPatternCheck { duration_ms, violations }`

**Acceptance criteria**:
- Anti-pattern checks run on every task completion, regardless of gate mode
- Error-severity violations trigger immediate retry (no compilation wasted)
- Warning-severity violations are logged but do not block
- AP checks complete in < 100ms per task (measured and logged)

### Task 18: Add anti-pattern false-positive tracking and exemption learning

**File**: `crates/roko-gate/src/anti_pattern.rs`

**What**: Track false positive rates per rule and per file pattern. After N
false positives for a rule+file combination, auto-suggest an exemption.

**Steps**:
1. Add `AntiPatternStats` persisted to `.roko/learn/anti-pattern-stats.json`:
   ```rust
   pub struct AntiPatternStats {
       pub per_rule: HashMap<String, RuleStats>,
   }
   pub struct RuleStats {
       pub total_fires: u64,
       pub false_positives: u64,       // marked by human or retry success
       pub auto_exemptions: Vec<String>, // file patterns auto-exempted
   }
   ```
2. When a task succeeds on retry after AP failure, mark the prior AP firing
   as a potential false positive
3. When false positive rate for a rule+file exceeds 50% over 10+ firings,
   suggest an exemption (log at warn level)
4. Persist stats after each plan run

**Acceptance criteria**:
- False positive rate is tracked per rule
- Stats survive across runs (persisted to disk)
- Auto-exemption suggestions appear in logs when false positive rate is high
- Manual exemption override works via task config

### Task 19: Add custom anti-pattern rules via roko.toml

**File**: `crates/roko-gate/src/anti_pattern.rs`, `crates/roko-core/src/config/serve.rs`

**What**: Allow users to define custom anti-pattern rules in `roko.toml` in
addition to the built-in 10 rules.

**Steps**:
1. Add `[[anti_pattern]]` section to `roko.toml`:
   ```toml
   [[anti_pattern]]
   id = "AP-CUSTOM-1"
   name = "hardcoded_api_key"
   pattern = 'sk-[a-zA-Z0-9]{32,}'
   severity = "error"
   file_glob = "*.rs"
   ```
2. Parse custom rules alongside built-in rules in `AntiPatternChecker::new()`
3. Custom rules use the same `AntiPatternRule` struct and exemption system
4. Built-in rules can be disabled via `[anti_pattern_defaults] disable = ["AP-10"]`

**Acceptance criteria**:
- Custom rules defined in `roko.toml` are loaded and applied
- Built-in rules can be disabled per-project
- Custom rules participate in false-positive tracking
- Invalid regex in custom rules produces a clear error at config load time

---

## Phase 4: Resume and Result Tracking (Tasks 20-24)

The `--continue` pattern: any long-running process will crash. The ability to
resume from disk state (not memory state) is what makes the system reliable.

### Task 20: Implement result file tracking for manual intervention

**File**: `crates/roko-runtime/src/workflow_engine.rs`

**What**: Write per-task `.result` files to disk as the sole coordination
mechanism. The mega-parity runner proved that simple files on disk enable
manual intervention at any point: mark a task as success, skip it, or
force a retry.

**Steps**:
1. Define result file location: `.roko/state/runs/{run_id}/{task_id}.result`
2. Write result file on each task status transition:
   ```json
   {"status": "success", "elapsed_ms": 12345, "commit": "abc123", "files_changed": 3}
   ```
3. Valid statuses: `in_progress`, `success`, `failed`, `blocked`, `skipped`, `success_noop`
4. On `--continue` resume, read all `.result` files and reconstruct task states
5. Support manual override: if a human writes `success` to a `.result` file,
   the scheduler treats that task as completed and unblocks dependents
6. Also write `.result.hash` with the commit SHA for cherry-pick support

**Acceptance criteria**:
- Each task produces a `.result` file at its designated path
- `--continue` reads result files and skips completed tasks
- Manually writing `success` to a result file unblocks dependent tasks on resume
- Result files are written atomically (write to tmp, rename)

### Task 21: Implement TaskDefFingerprint for mid-run edit detection

**File**: `crates/roko-runtime/src/pipeline_state.rs`

**What**: Hash task definitions so the runner can detect when tasks were edited
between runs. Prevents resuming with stale state when the plan has changed.

**Steps**:
1. Add `TaskDefFingerprint`:
   ```rust
   pub struct TaskDefFingerprint {
       pub task_id: String,
       pub hash: String,  // SHA-256 of task prompt + deps + scope
   }
   ```
2. Compute fingerprints at plan load time
3. Store fingerprints in the checkpoint file alongside task states
4. On `--continue` resume, compare stored fingerprints against current task defs
5. If a task's fingerprint changed, mark it as `Ready` (re-run needed) and
   log a warning: "Task {id} definition changed since last run, re-executing"
6. If dependencies of a changed task also need re-running, cascade the reset

**Acceptance criteria**:
- Task fingerprints are computed and stored in checkpoint files
- Editing a task's prompt between runs causes it to re-execute on resume
- Unchanged tasks are still skipped on resume
- Dependency cascading works: if task A changed and task B depends on A,
  both are re-run

### Task 22: Implement JSONL recovery for partial writes

**File**: `crates/roko-runtime/src/jsonl_logger.rs`

**What**: Make JSONL log files resilient to partial writes from crashes.
The mega-parity runner's `.result` files were sometimes truncated on crash;
the recovery logic needs to handle this.

**Steps**:
1. Add `recover_jsonl(path: &Path) -> Result<Vec<serde_json::Value>>` that
   reads a JSONL file line by line, skipping malformed lines
2. Log a warning for each skipped line: "Skipping malformed JSONL at line {n}"
3. Add `atomic_append(path: &Path, value: &impl Serialize) -> Result<()>` that
   writes to a temp file and uses `rename()` for atomicity (for small files)
   or appends with `fsync` (for JSONL append-only logs)
4. Use `atomic_append` for all result file writes
5. Update `WorkflowEngine` resume logic to use `recover_jsonl` when loading
   state from JSONL files

**Acceptance criteria**:
- Truncated JSONL lines are skipped with a warning, not a crash
- Complete lines before a truncation point are successfully recovered
- Result files are written with fsync to minimize data loss window
- Resume after a simulated crash (kill -9) recovers all complete entries

### Task 23: Add `--only` flag for selective task execution

**File**: `crates/roko-cli/src/run.rs`

**What**: Allow running only specific tasks from a plan, matching the
mega-parity runner's `--only A,B,C` flag.

**Steps**:
1. Add `--only <task_ids>` CLI flag (comma-separated list of task IDs)
2. When `--only` is set, filter the task DAG to include only the specified
   tasks and their transitive dependencies
3. Tasks not in the `--only` set are marked as `Skipped` in result files
4. Combine with `--continue`: `--only T5,T6 --continue` re-runs T5 and T6
   but skips everything else (even if previously failed)
5. Validate that all specified task IDs exist in the plan (error early)

**Acceptance criteria**:
- `roko plan run --only T5,T6` runs only T5 and T6 (and their dependencies)
- Tasks not in the `--only` list are skipped
- `--only` combined with `--continue` works correctly
- Invalid task IDs produce a clear error before execution starts

### Task 24: Add `--dry-run` flag for wave plan preview

**File**: `crates/roko-cli/src/run.rs`
**Also reads**: `crates/roko-orchestrator/src/dag.rs`

**What**: Show the wave structure and task execution plan without actually
running anything. Catches dependency errors and shows parallelism before
committing to a multi-hour run.

**Steps**:
1. Add `--dry-run` flag to `roko plan run`
2. Load the plan, build the DAG via `UnifiedTaskDag::build()`, compute waves
3. Display wave structure:
   ```
   Wave 0 (3 tasks, parallel):
     T1: "Wire episode logging" [mechanical, 2 files]
     T2: "Wire cascade router"  [mechanical, 1 file]
     T3: "Add efficiency events" [focused, 3 files]
   Wave 1 (2 tasks, parallel):
     T4: "Wire replan logic" [integrative, 4 files]  deps: T1, T2
     T5: "Add gate feedback" [focused, 2 files]      deps: T3
   ...
   Total: 5 tasks, 2 waves, max parallelism: 3
   Estimated time: ~15 min (wave gating) / ~5 min (deferred gating)
   ```
4. Include critical path analysis from `UnifiedTaskDag::critical_path()`
5. Include file overlap warnings (tasks in same wave touching same files)
6. Exit 0 after display (no execution)

**Acceptance criteria**:
- Wave structure is displayed with task details and dependencies
- Critical path is highlighted
- File overlap warnings are shown for potential merge conflicts
- No git operations or agent dispatches occur during dry run

---

## Phase 5: DAG Scheduling and Merge Integration (Tasks 25-30)

Advanced scheduling patterns from the mega-parity runner: critical path
optimization, fan-out priority, auto-cherry-pick, and parallel execution
with serialized merges.

### Task 25: Wire critical path priority into TaskScheduler

**File**: `crates/roko-runtime/src/task_scheduler.rs`
**Also reads**: `crates/roko-orchestrator/src/dag.rs`

**What**: When multiple tasks are ready to dispatch, prioritize critical path
tasks. The DAG's CPM analysis identifies zero-slack tasks that determine
total execution time.

**Steps**:
1. Add `priority: TaskPriority` to `SchedulableTask`:
   ```rust
   pub struct TaskPriority {
       pub critical_path: bool,   // zero slack in CPM analysis
       pub fan_out: usize,        // number of downstream dependents
       pub tier: u8,              // 0=mechanical, 1=focused, 2=integrative, 3=architectural
   }
   ```
2. In `next_batch()`, sort ready tasks by: critical_path (desc), fan_out (desc), tier (asc)
3. Critical path tasks are dispatched first because they cannot afford failure delay
4. High fan-out tasks are dispatched next because they unblock the most work
5. Lower-tier tasks go before higher-tier (mechanical tasks are faster to complete)
6. Accept `TaskPriority` from `UnifiedTaskDag::critical_path_info()` at scheduler construction

**Acceptance criteria**:
- Critical path tasks are always dispatched before non-critical tasks
- Fan-out priority breaks ties among non-critical tasks
- Priority does not affect correctness (only dispatch order)
- Dispatch order is deterministic for the same DAG

### Task 26: Implement auto-cherry-pick conveyor belt

**File**: `crates/roko-orchestrator/src/merge_queue.rs` (extend)

**What**: Background process that watches for completed task branches and
cherry-picks them into a target branch. The mega-parity runner's "conveyor
belt" pattern: agents produce, picker integrates, human reviews.

**Steps**:
1. Add `AutoPickConfig`:
   ```rust
   pub struct AutoPickConfig {
       pub target_branch: String,      // branch to cherry-pick into
       pub interval_secs: u64,         // polling interval (default 90)
       pub auto_resolve: bool,         // accept --theirs on conflict
       pub verify_after_pick: bool,    // run cargo check after each cycle
   }
   ```
2. Add `spawn_auto_pick(config: AutoPickConfig, merge_queue: Arc<MergeQueue>) -> JoinHandle`
   that polls for completed merges and cherry-picks to the target branch
3. On conflict: if `auto_resolve` is true, use `git checkout --theirs . && git add -A`;
   otherwise mark as needing manual resolution
4. Save pick state to `.roko/state/auto-pick.json` (survives restart)
5. Emit `RuntimeEvent::CherryPick { task_id, status, conflict }` for monitoring

**Acceptance criteria**:
- Completed task changes are automatically cherry-picked to the target branch
- Conflict resolution respects `auto_resolve` config
- Pick state survives process restart
- Cherry-pick progress is visible via events/dashboard

### Task 27: Implement model escalation on repeated failure

**File**: `crates/roko-runtime/src/effect_driver.rs`

**What**: When a cheap model fails the same gate repeatedly, escalate to a
stronger model. The mega-parity runner found that most mechanical tasks
succeed on first try with a cheap model, but 5-10% need a stronger one.

**Steps**:
1. Add `model_escalation: ModelEscalation` to `WorkflowConfig`:
   ```rust
   pub struct ModelEscalation {
       pub enabled: bool,
       pub escalation_after: u32,      // attempts before escalating (default 2)
       pub cheap_model: String,        // starting model
       pub strong_model: String,       // escalation target
   }
   ```
2. In `EffectDriver`, track attempt count per task
3. When attempt count exceeds `escalation_after`, switch the model for the
   next dispatch: override `default_model` with `strong_model`
4. Log the escalation: "Task {id}: escalating from {cheap} to {strong} after {n} failures"
5. Integrate with CascadeRouter: record the escalation as a routing observation
   so the router learns which task types need stronger models

**Acceptance criteria**:
- First 2 attempts use the cheap model
- Third attempt uses the strong model
- Escalation is logged and trackable
- CascadeRouter receives the escalation observation for future routing

### Task 28: Wire chain fusion from DAG into TaskScheduler

**File**: `crates/roko-runtime/src/task_scheduler.rs`
**Also reads**: `crates/roko-orchestrator/src/dag.rs`

**What**: Wire `UnifiedTaskDag::fuse_linear_chains()` into the scheduler so
linear sequences of mechanical tasks are collapsed into single dispatch
units. Reduces wave count and dispatch overhead.

**Steps**:
1. Add `fusion_config: Option<FusionConfig>` to `TaskScheduler::new()`
2. When fusion is enabled, call `dag.fuse_linear_chains(&fusion_config)` before
   converting to `SchedulableTask`s
3. Fused tasks get combined prompts: "Step 1: [task A prompt]. Step 2: [task B prompt]."
4. Fused tasks inherit the union of all constituent task file scopes
5. If any step in a fused task fails, the entire fused unit fails (but
   individual step results are tracked for diagnostics)
6. Default `FusionConfig`: max_chain_length=3, same_tier_only=true

**Acceptance criteria**:
- Linear chains of mechanical tasks are fused into single dispatch units
- Fused tasks produce correct combined prompts
- Fusion reduces total wave count (verified via `--dry-run`)
- Fused task failure is attributed to the failing step

### Task 29: Add parallel execution with --continue support

**File**: `crates/roko-runtime/src/workflow_engine.rs`

**What**: When running with `--continue` after a partial failure, re-execute
only failed/blocked tasks while preserving successful results. Support
parallel re-execution of independent failed tasks.

**Steps**:
1. On resume, load result files (Task 20) and task fingerprints (Task 21)
2. Classify tasks: `success` -> skip, `failed` -> re-execute, `blocked` -> check
   if dependency is now resolved, `in_progress` -> treat as failed (stale)
3. Rebuild the DAG with only tasks that need re-execution
4. Dispatch re-execution tasks in parallel where dependencies allow
5. Merge re-executed tasks into the integration branch via the merge queue
6. Support multiple `--continue` cycles: each cycle picks up where the last left off
7. Clear stale worktree locks before re-execution (Task 5)

**Acceptance criteria**:
- `--continue` skips successful tasks and re-runs failed ones
- Previously blocked tasks are re-evaluated against current dependency state
- Parallel re-execution works for independent failed tasks
- Multiple consecutive `--continue` cycles converge to all-success

### Task 30: Add `--pause` flag for inter-wave inspection

**File**: `crates/roko-runtime/src/workflow_engine.rs`

**What**: Pause between waves so a human can inspect the merged result, fix
issues, and resume. The mega-parity runner showed this is essential for
large plans where wave gate failures need human judgment.

**Steps**:
1. Add `--pause` flag to `roko plan run`
2. After each wave completes and is merged, print a summary:
   ```
   Wave 2 complete: 5/5 tasks succeeded, 3 files modified
   Integration branch: roko/run-20260429-030528
   Press Enter to continue, or 's' to stop...
   ```
3. Wait for user input before dispatching the next wave
4. On 's': save checkpoint and exit cleanly (resumable with `--continue`)
5. While paused, the human can inspect the integration branch, run manual
   tests, or edit files
6. After resume, re-read the integration branch state (human may have
   made changes)

**Acceptance criteria**:
- `--pause` stops execution between waves and waits for input
- User can inspect and modify the integration branch during the pause
- Pressing Enter resumes execution from the current state
- Pressing 's' saves state and exits cleanly
- Changes made during pause are visible to subsequent waves

---

## Dependency Graph

```
Phase 0 (Worktree):
  T1 (WorktreeManager wire)
  T2 (three-tier branches) -- depends on T1
  T3 (serialized merge) -- depends on T1, T2
  T4 (disk monitoring) -- independent
  T5 (cleanup utilities) -- independent

Phase 1 (Wave Gating):
  T6 (WaveGatePhase) -- independent
  T7 (no-build injection) -- depends on T6
  T8 (wave gate execution) -- depends on T3, T6
  T9 (CLI flags) -- depends on T6
  T10 (bisection) -- depends on T8

Phase 2 (Context Handoff):
  T11 (cumulative context) -- independent
  T12 (wire into dispatch) -- depends on T1, T11
  T13 (failure context) -- independent
  T14 (structured handoff) -- depends on T11
  T15 (context-pack files) -- independent

Phase 3 (Anti-Pattern Pre-Gates):
  T16 (AntiPatternChecker) -- independent
  T17 (wire as pre-gate) -- depends on T16
  T18 (false-positive tracking) -- depends on T16
  T19 (custom rules via toml) -- depends on T16

Phase 4 (Resume and Result Tracking):
  T20 (result files) -- depends on T1
  T21 (task fingerprints) -- independent
  T22 (JSONL recovery) -- independent
  T23 (--only flag) -- depends on T20
  T24 (--dry-run) -- independent

Phase 5 (DAG Scheduling):
  T25 (critical path priority) -- independent
  T26 (auto-cherry-pick) -- depends on T3, T20
  T27 (model escalation) -- depends on T13
  T28 (chain fusion) -- depends on T25
  T29 (--continue parallel) -- depends on T20, T21, T22
  T30 (--pause flag) -- depends on T20
```

## Key Numbers (From Operational Data)

| Metric | Value | Source |
|---|---|---|
| Per-task time (with build) | 15-40 min | Mega-parity runner |
| Per-task time (no build) | 1-5 min | Mega-parity runner |
| Wave gate (cargo check) | 3-8 min | 18-crate workspace |
| Worktree creation | ~2 sec | git worktree add |
| Worktree size | ~500 MB | Source only |
| Merge conflict rate | ~30% | Large runs, shared files |
| AP false positive rate | 2-3% | Mostly AP-10 |
| Agent compliance (no-build) | 95-99% | System prompt > context file |
| Post-run fix-up errors | 10-30 | After 195 batches, no-build |
| Optimal parallelism | 15 | MacBook Pro, 20 API workers |

## Decision Matrix

| Plan Size | Recommended Config |
|---|---|
| 1-5 tasks | `--gate-mode per-task` (low overhead, immediate feedback) |
| 5-20 tasks | `--gate-mode per-wave` (10x faster, good safety) |
| 20+ tasks | `--gate-mode deferred --no-build` (maximum speed) |
| Mechanical tasks | Cheap model, no audit, wave gates |
| Architectural tasks | Strong model, per-task gates, structured handoff |
