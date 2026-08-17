# Implementation Plans: Phased Execution with Acceptance Criteria

> Each plan is decomposed into tasks suitable for agent execution. Tasks include
> exact file paths, the specific function/method to modify, acceptance criteria
> that can be verified mechanically, and dependency ordering. Line numbers are
> approximate and should be confirmed at execution time.
>
> Core thesis from all analysis: **wire existing infrastructure into live paths,
> don't build new things.**

---

## PLAN 1: Wire Learning Into Runner v2 (Highest Impact)

**Problem**: 10 learning components built and working but only called from the
deprecated orchestrate.rs (22K lines). Runner v2 (the default path) records zero
durable feedback. The system cannot learn from 99% of its runs.

**Effort**: 2-3 days | **Impact**: Critical -- without this, every run is the first run
**Dependencies**: None

### Task 1.1: Wire Episode Logging

**File**: `crates/roko-cli/src/runner/event_loop.rs`
**What**: After each task attempt completes (success or failure), construct an `Episode`
and append to `.roko/episodes.jsonl`.

**Steps**:
1. Import `roko_learn::episode_logger::{EpisodeLogger, Episode, Usage, GateVerdict}` (already partially imported)
2. In `RunContext`, add field `episode_logger: EpisodeLogger`
3. Construct `EpisodeLogger::new(paths.roko_dir.join("episodes.jsonl"))` in `run()`
4. After `handle_agent_event()` resolves a task attempt, construct `Episode`:
   ```rust
   Episode {
       agent_id: agent_id.clone(),
       task_id: task_id.clone(),
       kind: role.to_string(),
       model: model_slug.clone(),
       backend: provider.to_string(),
       usage: Usage { input_tokens, output_tokens, cache_read: 0, cost_usd, wall_ms },
       gate_verdicts: gate_results.iter().map(|g| GateVerdict { ... }).collect(),
       hdc_fingerprint: None,
       metadata: Default::default(),
   }
   ```
5. Call `episode_logger.record(&episode)?`

**Acceptance criteria**:
- Run `roko plan run` on a test plan
- Verify `.roko/episodes.jsonl` has one entry per task attempt
- Each entry has non-zero `input_tokens` and `wall_ms`
- Gate verdicts are populated for tasks that reached the gate phase

### Task 1.2: Wire CascadeRouter Observations

**File**: `crates/roko-cli/src/runner/event_loop.rs`
**What**: After each agent dispatch completes, call `cascade_router.observe()` with
the routing context and outcome.

**Steps**:
1. Import `roko_learn::cascade_router::CascadeRouter`
2. Load router from `.roko/learn/cascade-router.json` via `CascadeRouter::load_or_new()`
3. In `RunContext`, add field `cascade_router: Arc<CascadeRouter>`
4. After task success/failure, construct `RoutingContext` with:
   - `role`, `task_category`, `complexity`, `iteration`, `crate_familiarity`, `prior_failure_count`
5. Call `cascade_router.observe(&context, &outcome)`
6. At run end, call `cascade_router.save()` to persist

**Acceptance criteria**:
- Run `roko plan run` twice
- Verify `.roko/learn/cascade-router.json` exists after first run
- Verify observation count > 0 after first run
- Verify observation count increases after second run

### Task 1.3: Wire AdaptiveThreshold Updates

**File**: `crates/roko-cli/src/runner/gate_dispatch.rs`
**What**: After each gate rung executes, call `thresholds.observe(rung, passed)`.

**Steps**:
1. Import `roko_gate::adaptive_threshold::AdaptiveThresholds`
2. Load from `.roko/learn/gate-thresholds.json` via `AdaptiveThresholds::load_or_new()`
3. Pass thresholds into `gate_dispatch` context
4. After each `GateVerdict` is produced, call `thresholds.observe(rung_index, verdict.passed)`
5. Wire adaptive skip: before running a gate, check `thresholds.should_skip_rung(rung)`
6. At run end, persist with `thresholds.save()`

**Acceptance criteria**:
- Run `roko plan run` and fail a gate
- Verify `.roko/learn/gate-thresholds.json` has rung entries
- Verify `total_observations > 0` for the failed rung
- After 20 consecutive passes on clippy, verify `should_skip_rung(1)` returns true

### Task 1.4: Wire Efficiency Events

**File**: `crates/roko-cli/src/runner/event_loop.rs`
**What**: After each agent turn, emit `AgentEfficiencyEvent` with token/cost/tool data.

**Steps**:
1. Import `roko_learn::efficiency::{AgentEfficiencyEvent, EfficiencyWriter}`
2. Construct `EfficiencyWriter::new(paths.roko_dir.join("learn/efficiency.jsonl"))` in `run()`
3. After agent completion, construct event from `AgentCompletionSummary`:
   - `input_tokens`, `output_tokens`, `cost_usd`, `wall_ms`
   - `tools_available` vs `tools_used` counts
   - Letter grade from cost/quality heuristic
4. Call `writer.append(&event)?` (includes flush)

**Acceptance criteria**:
- Run `roko plan run`
- Verify `.roko/learn/efficiency.jsonl` has entries
- Each entry has non-zero token counts
- File grows with each agent call, not buffered to end

### Task 1.5: Wire Replan-on-Gate-Failure

**File**: `crates/roko-cli/src/runner/event_loop.rs`
**What**: When gate failures exhaust the autofix budget, trigger architectural replanning
instead of immediately failing the task.

**Steps**:
1. Import `roko_orchestrator::replan::{ReplanStrategy, PlanRevisionRequest}`
2. Track `gate_failure_count: HashMap<String, u32>` per plan in RunState
3. After gate failure where `iteration >= max_auto_fix_iterations`:
   - Check `config.learning.replan_on_gate_failure` (defaults to true)
   - If true and `gate_failure_count[plan_id] < 2`:
     - Construct `PlanRevisionRequest` with gate error context
     - Spawn a strategist agent to generate a revised approach
     - If revision succeeds, reset the task and retry with new prompt
     - If revision fails, proceed to Failed as before
4. Increment `gate_failure_count[plan_id]` on each replan attempt

**Acceptance criteria**:
- Create a task that consistently fails gates
- With `replan_on_gate_failure = true`, verify:
  - Task failure triggers replan (strategist agent spawned)
  - Replan is capped at 2 attempts per plan
  - After replan exhaustion, task fails normally
- With `replan_on_gate_failure = false`, verify: no replan, immediate failure

### Task 1.6: Wire Section Effectiveness

**File**: `crates/roko-cli/src/runner/event_loop.rs`
**What**: After each task completion, record which prompt sections contributed
to success/failure for future prompt optimization.

**Steps**:
1. Import `roko_learn::section_effect::SectionEffectivenessRegistry`
2. Load from `.roko/learn/section-effects.json`
3. After task success, record section IDs from prompt assembly as positive
4. After task failure, record section IDs as negative
5. At run end, persist the registry

**Acceptance criteria**:
- Run `roko plan run`
- Verify `.roko/learn/section-effects.json` has section entries
- Entries have `positive_count` > 0 for successful tasks

---

## PLAN 2: Consolidate Model Selection (Core Reliability)

**Problem**: 9+ dispatch paths with inconsistent behavior. User's configured
model/backend loaded then thrown away. `auth_detect.rs` ignores `roko.toml`.

**Effort**: 3-4 days | **Impact**: High -- config not respected = broken product
**Dependencies**: None (can run in parallel with Plan 1)

### Task 2.1: Create `ResolvedModelConfig` Unification

**File**: `crates/roko-core/src/agent/mod.rs` (extend existing `resolve_model()`)
**What**: Single function that resolves any model key to a fully-specified config.

**Steps**:
1. Extend `ResolvedModel` struct to include: `provider_kind`, `base_url`, `api_key_env`,
   `max_tokens`, `supports_tools`, `supports_thinking`
2. Make `resolve_model()` populate all fields from config
3. Replace all 8 hardcoded model strings in the codebase:
   - Search: `grep -rn '"claude-3' crates/ --include='*.rs' | grep -v target/ | grep -v test`
   - Replace each with `resolve_model(config, key).slug`

**Acceptance criteria**:
- `grep -rn '"claude-3-haiku\|"claude-sonnet\|"claude-3-5' crates/roko-cli/src/ | grep -v test | grep -v target/` returns 0 results
- All model references go through `resolve_model()`

### Task 2.2: Make `auth_detect.rs` Consult Config First

**File**: `crates/roko-cli/src/auth_detect.rs`
**What**: Check `config.agent.default_backend` first, then fall back to env scanning.

**Steps**:
1. Add `config: Option<&RokoConfig>` parameter to `detect_auth()`
2. If config is Some and `config.agent.default_backend` is non-empty, use that provider
3. Resolve the provider's API key from config or env var
4. Fall back to current env-scanning behavior only if config is None or backend is empty
5. Update all callers to pass config when available

**Acceptance criteria**:
- Set `default_backend = "zhipu"` and `ZHIPU_API_KEY=xxx` in env
- Run `roko run "hello"` -- verify ZhiPu is used (not Anthropic even if ANTHROPIC_API_KEY is set)
- Remove config setting -- verify env-scan fallback works as before

### Task 2.3: Unify max_tokens

**Files**: Multiple files in `crates/roko-cli/src/` and `crates/roko-agent/src/`
**What**: One default max_tokens (8192) with per-model override from config.

**Steps**:
1. Search: `grep -rn 'max_tokens.*=.*4096\|max_tokens.*=.*1024\|max_tokens.*=.*512' crates/ --include='*.rs'`
2. Replace each hardcoded value with `resolved_model.max_tokens`
3. Default to 8192 when model config has no max_tokens
4. Ensure `resolve_model()` populates max_tokens from model profile

**Acceptance criteria**:
- No hardcoded max_tokens values remain (except in test code)
- Model config `max_tokens` field is respected

### Task 2.4: Route All Dispatch Through Provider System

**Files**: `crates/roko-cli/src/run.rs`, `crates/roko-cli/src/chat_inline.rs`,
`crates/roko-cli/src/dispatch_direct.rs`
**What**: All dispatch paths use the provider system from `roko-agent`.

**Steps**:
1. `run.rs`: Replace inline dispatch logic with `ServiceFactory::build()` -> `ModelCallService`
2. `chat_inline.rs`: Replace inline HTTP client with `ModelCallService`
3. `dispatch_direct.rs`: Route through provider system instead of raw subprocess

**Acceptance criteria**:
- All three paths use the same model resolution and dispatch code
- Setting `default_model` in roko.toml affects all three paths identically

---

## PLAN 3: Fix P0 Blockers (Security + Crashes)

**Problem**: Runtime panic on `roko config mcp`, share routes bypass auth,
cloud deploys are unauthenticated.

**Effort**: 1 day | **Impact**: Critical -- crashes and security holes
**Dependencies**: None (do first, in parallel with Plans 1-2)

### Task 3.1: Wire ConfigCmd::Mcp Dispatch

**File**: `crates/roko-cli/src/commands/config_cmd.rs`
**What**: Replace `unreachable!()` with actual MCP config handling.

**Steps**:
1. Find the `ConfigCmd::Mcp` match arm (currently `unreachable!()`)
2. Add subcommand variants: `list`, `add <name> <command>`, `remove <name>`
3. `list`: Read `config.agent.mcp_servers` and format as table
4. `add`: Append to `[agent.mcp_servers]` section in roko.toml
5. `remove`: Remove entry from `[agent.mcp_servers]` section

**Acceptance criteria**:
- `roko config mcp list` shows configured MCP servers (no crash)
- `roko config mcp add test-server "npx test"` adds entry to roko.toml
- `roko config mcp remove test-server` removes it

### Task 3.2: Move Share Routes Inside Auth Middleware

**File**: `crates/roko-serve/src/routes/shared_runs.rs` and `crates/roko-serve/src/lib.rs`
**What**: Move `POST /api/runs/{id}/share` from public router group to protected router group.

**Steps**:
1. In `lib.rs`, find where share routes are mounted
2. Move the share route registration from the public `Router` to the protected `Router`
   (the one wrapped with auth middleware)
3. Keep `GET /api/shared/{token}` as public (read-only access via share token)

**Acceptance criteria**:
- Start `roko serve` with `api_auth.enabled = true`
- `POST /api/runs/test/share` without auth header returns 401
- `POST /api/runs/test/share` with valid auth header succeeds
- `GET /api/shared/{token}` works without auth (unchanged)

### Task 3.3: Auto-Provision Auth on Cloud Deploy

**File**: `crates/roko-cli/src/commands/deploy.rs` (or equivalent)
**What**: Generate random API key during deploy, enable auth, print key.

**Steps**:
1. In the deploy command handler, generate a random 32-byte hex API key
2. Set `api_auth.enabled = true` and `api_auth.keys = [generated_key]` in the
   deploy config
3. Print the key to stdout with a message: "Save this API key -- it will not be shown again"
4. Fail deployment if auth cannot be configured

**Acceptance criteria**:
- `roko deploy railway` prints an API key
- The deployed service rejects requests without the key
- The printed key works when used as `Authorization: Bearer <key>`

### Task 3.4: Add Scrubbing to CLI Gist Path

**File**: `crates/roko-cli/src/share.rs`
**What**: Apply `scrub_secrets()` before uploading to GitHub Gist.

**Steps**:
1. Import `roko_agent::safety::scrub::{ScrubPolicy, scrub_secrets}` (already available)
2. Before creating the Gist, run the transcript through `scrub_secrets(&transcript, &ScrubPolicy::default())`
3. The HTTP share path already does this -- match its behavior

**Acceptance criteria**:
- Create a run with an API key in the output (e.g., `echo ANTHROPIC_API_KEY=sk-test-123`)
- `roko run --share` should upload a Gist where `sk-test-123` is replaced with `[REDACTED]`

---

## PLAN 4: Make Streaming Work (UX)

**Problem**: `chat_inline.rs` drains streaming events silently. TUI stays in
spinner during agent work. Model name shows "-" in runner v2.

**Effort**: 2 days | **Impact**: High -- blind TUI = terrible UX
**Dependencies**: None

### Task 4.1: Forward Streaming Events to TUI

**File**: `crates/roko-cli/src/chat_inline.rs`
**What**: Replace silent drain with actual event processing.

**Steps**:
1. Find `while let Some(_event) = event_rx.recv().await {}`
2. Replace with event processing loop:
   ```rust
   while let Some(event) = event_rx.recv().await {
       match event {
           StreamEvent::ContentDelta(text) => {
               state_hub.push_dashboard_event(DashboardEvent::AgentOutput { text });
           }
           StreamEvent::ReasoningDelta(text) => {
               state_hub.push_dashboard_event(DashboardEvent::AgentReasoning { text });
           }
           StreamEvent::ToolCallDelta { name, .. } => {
               state_hub.push_dashboard_event(DashboardEvent::ToolCall { name });
           }
           StreamEvent::Usage(usage) => {
               state_hub.push_dashboard_event(DashboardEvent::TokenUpdate { usage });
           }
           _ => {}
       }
   }
   ```
3. Ensure StateHub distributes these events to the TUI's DashboardEventReceiver

**Acceptance criteria**:
- Run `roko chat`, type a prompt
- See streaming text appearing in the TUI as the agent generates output
- Token count updates in real-time during generation

### Task 4.2: Fix Model Name in Runner v2

**File**: `crates/roko-cli/src/runner/tui_bridge.rs`
**What**: Pass actual model slug from dispatch into TUI events.

**Steps**:
1. In `AgentDispatchOutcome`, ensure `model` field is populated (not empty string)
2. In `tui_bridge.rs`, when emitting `DashboardEvent::AgentStarted`, use the
   populated model field
3. Trace the model through: config -> dispatch -> AgentSpawnConfig -> TUI event

**Acceptance criteria**:
- Run `roko plan run`
- TUI shows actual model name (e.g., "claude-sonnet-4-20250514"), not "-"

---

## PLAN 5: Unify Gate Config (Consistency)

**Problem**: `roko init` writes `[[gate]]` arrays; `roko plan run` reads `[gates]`
tables. Init-generated gates are invisible to plan run.

**Effort**: 1 day | **Impact**: Medium -- config that does nothing is confusing
**Dependencies**: None

### Task 5.1: Accept Both Config Formats

**File**: `crates/roko-core/src/config/schema.rs` (or wherever `RokoConfig::from_toml()` lives)
**What**: Parse both `[[gate]]` and `[gates]` formats, normalize to one internal repr.

**Steps**:
1. In the TOML parser, check for both `[[gate]]` (array of tables) and `[gates]` (table)
2. If `[[gate]]` is present, convert to `GatesConfig` with `enabled_gates` derived from
   the gate array entries
3. If both are present, merge (array entries take precedence)
4. Emit a deprecation warning for `[[gate]]` format

**Acceptance criteria**:
- `roko init` produces config, `roko plan run` respects the generated gates
- Both `[[gate]]` and `[gates]` formats work
- `roko config show` displays gates regardless of source format

### Task 5.2: Update `roko init` to Write Runtime-Compatible Format

**File**: `crates/roko-cli/src/commands/init.rs`
**What**: Write `[gates]` format that the runtime parser reads.

**Steps**:
1. Change the init template to write `[gates]` instead of `[[gate]]`
2. Include `enabled = ["compile", "clippy", "test"]` as default
3. Include `max_rung` documentation in comments

**Acceptance criteria**:
- `roko init` in a fresh directory produces a roko.toml
- `roko plan run` in that directory uses the configured gates
- No silent config discarding

---

## PLAN 6: Ground Plan Generation in Repo Context (Quality)

**Problem**: Plans generated without repo awareness propose greenfield crates that
duplicate existing functionality.

**Effort**: 1-2 days | **Impact**: Medium -- bad plans waste agent time
**Dependencies**: None

### Task 6.1: Wire `build_repo_context` Into Plan Generation

**File**: `crates/roko-cli/src/commands/plan.rs`
**What**: Call `build_repo_context()` before agent dispatch in generate/regenerate.

**Steps**:
1. Import `repo_context::build_repo_context`
2. In `plan generate` handler, before dispatching the agent:
   - Call `build_repo_context(&workdir)` to get crate map, file structure, key types
   - Inject result as a prompt section: "## Repository Context\n\n{context}"
3. Do the same in `plan regenerate` and `prd plan` handlers

**Acceptance criteria**:
- `roko plan generate` for a task touching roko-agent includes crate map in prompt
- Generated plan references existing crates, not new ones
- `roko prd plan <slug>` also includes repo context

### Task 6.2: Inject Validation Diagnostics Into Regeneration

**File**: `crates/roko-cli/src/commands/plan.rs`
**What**: On validation failure, feed errors back to regeneration agent.

**Steps**:
1. After `plan regenerate` validates the generated plan:
2. If validation fails, construct a diagnostic prompt:
   ```
   The generated plan has validation errors:
   {errors}

   Please regenerate the plan fixing these issues. Keep all valid tasks unchanged.
   ```
3. Re-dispatch the agent with the diagnostic prompt (max 2 iterations)
4. If still failing after 2 iterations, save the plan with warnings

**Acceptance criteria**:
- Generate a plan with invalid file references
- `plan regenerate` detects the errors and re-dispatches
- Second attempt produces a corrected plan (or fails with clear message)

---

## PLAN 7: Novel Innovations for Orchestration

These are forward-looking improvements derived from the runner lessons and gap analysis.
Each is independent and can be prioritized separately.

### Innovation 7.1: Express Gate Mode (From Runner Lessons)

**Insight**: The parallel runner proved that deferred gating is 10-100x faster
with acceptable error rates. Roko should support this for simple tasks.

**Design**:
- Add `max_rung` field to per-task config in tasks.toml
- For tasks with `complexity = "trivial"`, default `max_rung = 0` (compile only)
- For tasks with `complexity = "simple"`, default `max_rung = 1` (compile + lint)
- Full gate pipeline only for `complexity = "standard"` or higher
- Anti-pattern checks (grep-based, millisecond latency) run on all tasks regardless

**File**: `crates/roko-gate/src/gate_service.rs` (already supports `max_rung`)
**File**: `crates/roko-cli/src/runner/gate_dispatch.rs` (set max_rung from task complexity)

**Acceptance criteria**:
- Trivial tasks run only compile gate
- Complex tasks run full pipeline
- Anti-pattern checks always run

### Innovation 7.2: Cumulative Context Section (From Runner Lessons)

**Insight**: The runner's cumulative section -- telling each agent what prior agents
changed -- reduced merge conflicts from 30% to 10%. Roko should adopt this.

**Design**:
- After each task completes, append a summary of changed files to a per-plan
  cumulative context document
- Use `git diff --stat` and signature-only views for large files
- Inject this as a Layer 3 domain context section for subsequent tasks
- Cap at 2000 tokens to avoid context bloat

**File**: `crates/roko-cli/src/runner/event_loop.rs` (collect changes after merge)
**File**: `crates/roko-compose/src/prompt_assembly_service.rs` (inject as context)

**Acceptance criteria**:
- Task B's prompt includes "Files changed by prior tasks: {list}"
- Git diff stat is included for each changed file
- Cumulative context grows with each completed task

### Innovation 7.3: Wave Gates for Multi-Task Plans (From Runner Lessons)

**Insight**: Per-task compilation takes 5-15 minutes. Wave gates (compile after
a batch of tasks complete) take 3-8 minutes. For plans with >5 tasks, this is
a significant speedup.

**Design**:
- Group tasks by execution wave (already computed by `UnifiedTaskDag::waves()`)
- Within a wave, run tasks without gates (only anti-pattern checks)
- At wave boundary, run full gate pipeline on the merged result
- If wave gate fails, identify which task's changes caused the failure (via
  `git bisect`-like approach on the wave's commits)
- Retry only the failing task, not the entire wave

**File**: `crates/roko-cli/src/runner/event_loop.rs` (wave boundary detection)
**File**: `crates/roko-cli/src/runner/gate_dispatch.rs` (deferred gate execution)

**Acceptance criteria**:
- Plan with 10 tasks in 3 waves: gates run 3 times (not 10)
- Wave gate failure identifies the specific task that broke compilation
- Total gate time reduced by 50%+ compared to per-task gating

### Innovation 7.4: Result File Coordination (From Runner Lessons)

**Insight**: The runner uses simple `.result` files as the primary coordination
mechanism, enabling manual intervention at any point. Roko's executor.json is
more complex but less operable.

**Design**:
- Alongside `executor.json`, write per-task status files:
  `.roko/state/tasks/{plan_id}/{task_id}.status` containing "queued", "running",
  "passed", "failed"
- These files are human-readable and human-writable
- Manual intervention: `echo "passed" > .roko/state/tasks/plan-1/task-3.status`
- On resume, reconcile file status with executor snapshot
- If file says "passed" but snapshot says "failed", trust the file (manual override)

**File**: `crates/roko-cli/src/runner/persist.rs` (write status files)
**File**: `crates/roko-cli/src/runner/resume.rs` (reconcile on resume)

**Acceptance criteria**:
- Each task has a `.status` file during execution
- Manual status override is respected on resume
- `ls .roko/state/tasks/` gives immediate visibility into run state

### Innovation 7.5: Activate VCG Auction for Context Allocation

**Insight**: The VCG auction in `roko-compose` is built but the greedy path dominates.
Activating it would produce more efficient prompt composition under tight token budgets.

**Design**:
- Replace greedy context allocation with `vcg_allocate()` when token budget is
  constrained (< 80% of model's max context)
- Each context source (knowledge, playbooks, prior outputs, pheromones) submits
  a bid based on estimated value and token cost
- VCG mechanism ensures truthful bidding and Pareto-optimal allocation
- Fall back to greedy when budget is not constrained (fast path)

**File**: `crates/roko-compose/src/prompt_assembly_service.rs` (switch to VCG)
**File**: `crates/roko-compose/src/auction.rs` (already built)

**Acceptance criteria**:
- With tight token budget, VCG selects higher-value sections
- With generous budget, greedy path remains (no regression)
- Prompt quality (measured by section effectiveness) improves under constrained budgets

### Innovation 7.6: LLM Judge Gate Implementation

**Insight**: Rung 6 (judge gate) is a stub. Implementing it provides semantic
code review beyond what compile/lint/test can catch.

**Design**:
- Replace `StubJudgeGate` with `LlmJudgeGate` that:
  1. Collects the `git diff` of changes
  2. Constructs a review prompt with diff, task description, and acceptance criteria
  3. Calls a model (configurable, default to a fast model like Cerebras or GPT-5.4-mini)
  4. Parses the response for pass/fail verdict with reasoning
  5. Returns a `GateVerdict` with the judge's assessment
- The judge gate should be opt-in (not enabled by default) because it adds LLM cost
- Configure via `[gates] judge_model = "cerebras-70b"` in roko.toml

**File**: `crates/roko-gate/src/gate_service.rs` (replace StubJudgeGate)
**File**: `crates/roko-gate/src/llm_judge_gate.rs` (implementation exists, wire it)

**Acceptance criteria**:
- `enabled_gates = ["compile", "clippy", "test", "judge"]` runs the LLM judge
- Judge produces a pass/fail verdict with reasoning
- Judge uses the configured model, not a hardcoded one
- Without `judge` in enabled_gates, no LLM call is made (zero cost)

---

## PLAN 8: Runner v2 Completion (Deprecate orchestrate.rs)

**Problem**: orchestrate.rs is 22K lines of dead code that contains the most
sophisticated features. Runner v2 is the live path but missing those features.

**Effort**: 5-7 days | **Impact**: High -- deleting 22K LOC of dead code
**Dependencies**: Plan 1 (learning wiring), Plan 2 (model unification)

### Task 8.1: Port Remaining Features to Runner v2

**After Plans 1-2 are complete**, the remaining features to port:
- Playbook extraction and injection (from successful tool-call sequences)
- Knowledge store querying (for system prompt enrichment)
- C-factor computation (collective intelligence metrics at run end)
- Crate familiarity tracking (per-agent expertise tracking)
- Daimon affect modulation (emotional state influencing dispatch)
- Enrichment pipeline (multi-step pre-dispatch context gathering)

Each of these follows the same pattern as Plan 1: construct the component in
`run()`, pass through `RunContext`, call at the appropriate event loop point.

### Task 8.2: Phase D -- Deprecate orchestrate.rs

**Steps**:
1. Rename `orchestrate.rs` to `orchestrate_legacy.rs`
2. Gate behind `#[cfg(feature = "legacy-orchestrate")]`
3. Remove from default compilation
4. Update CLI dispatch to never route to orchestrate.rs
5. Run full test suite to verify nothing depends on it

### Task 8.3: Phase E -- Align with Unified Spec

**Steps**:
1. Type renames where applicable (align with spec documents)
2. Activity recording format (structured events instead of free-form)
3. Event schema alignment (all events through RuntimeEvent enum)

### Task 8.4: Delete Dead Code

**Steps**:
1. Remove `orchestrate_legacy.rs` (22K lines)
2. Remove unused imports and dependencies
3. Run `cargo +nightly udeps` to find unused crate dependencies
4. Clean up `Cargo.toml` for roko-cli

**Acceptance criteria**:
- All existing `roko plan run` tests pass
- Learning files populated after runs (verified by Plan 1)
- No regression in gate handling
- `orchestrate.rs` is gone from the default build
- `cargo check --workspace` clean with 22K fewer lines

---

## Priority Matrix

| Priority | Plan | Impact | Effort | Parallel? |
|---|---|---|---|---|
| **P0** | Plan 3: P0 Blockers | Critical | 1 day | Yes (do immediately) |
| **P1** | Plan 1: Wire Learning | Critical | 2-3 days | Yes (with Plan 2, 3) |
| **P2** | Plan 2: Model Unification | High | 3-4 days | Yes (with Plan 1, 3) |
| **P3** | Plan 4: Streaming UX | High | 2 days | Yes (with Plan 1-3) |
| **P4** | Plan 5: Gate Config | Medium | 1 day | After Plan 3 |
| **P5** | Plan 6: Ground Plans | Medium | 1-2 days | After Plan 2 |
| **P6** | Plan 7: Innovations | Medium-High | Varies | After Plans 1-3 |
| **P7** | Plan 8: Deprecate Legacy | High | 5-7 days | After Plans 1-2 |

### Fast Track (30 min - 2 hours each, do in first day)

These are the smallest fixes with the highest impact-to-effort ratio:

| Fix | Time | Plan |
|---|---|---|
| Wire `ConfigCmd::Mcp` (kill the crash) | 30 min | Plan 3.1 |
| Move share routes inside auth | 30 min | Plan 3.2 |
| Fix model name "-" in TUI | 1 hour | Plan 4.2 |
| Forward streaming events (not drain) | 2 hours | Plan 4.1 |
| Add scrubbing to CLI Gist | 1 hour | Plan 3.4 |

### Batch Development Strategy (For Plans 7-8)

Based on runner lessons learned:
- Use worktree isolation per plan task
- Disable builds during implementation (anti-pattern checks only)
- Wave gates at plan group boundaries
- Context packs with cumulative change summaries
- `--continue` flag for disk-based resume
- Anti-pattern grep checks before commit
- Never delete worktrees (undo mechanism)
- Manual cherry-pick with auto-pick monitoring for integration

### Performance Optimization Roadmap

| Phase | Work | Savings | Effort |
|---|---|---|---|
| 0 | Shared HTTP client (already exists, not used everywhere) | 320-500ms | 2h |
| 1 | Express gate mode for trivial tasks | 500-2000ms | 4h |
| 2 | Memoize efficiency signals, batch substrate writes | 150-300ms | 4h |
| 3 | Pre-spawned warm agent pool | 200-500ms | 8h |
| 4 | VCG auction for prompt assembly (tight budgets only) | Variable | 4h |

**Target**: Fast API model (no gates) from ~880ms -> ~460ms.
With express gates: ~660ms (from 1.4-2.9s).
With wave gates on multi-task plans: 3-5x faster total gate time.
