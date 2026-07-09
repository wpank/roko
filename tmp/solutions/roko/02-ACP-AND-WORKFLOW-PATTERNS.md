# ACP Protocol, Workflow Patterns, and Batch Processing

> Deep analysis of the ACP subsystem at `crates/roko-acp/src/`, workflow state
> machines, agent role definitions, batch processing protocols, and the path
> toward unified execution.

---

## 1. ACP Protocol

### 1.1 Wire Format

JSON-RPC 2.0 over stdio (newline-delimited JSON). Types defined in
`crates/roko-acp/src/types.rs`. The crate implements ACP spec version 0.12.2 (protocol
version 1), as declared by the constants `ACP_PROTOCOL_VERSION` and `ACP_SPEC_VERSION`.

**Critical constraint:** stdout = protocol channel. All diagnostic logging must go to files
or stderr. Any stray `println!` in the code path will corrupt the JSON-RPC stream and crash
the editor integration.

**Message types (`JsonRpcMessage` enum):**
- `Request` -- client-to-agent with an `id` for response correlation
- `Response` -- agent-to-client matching a previous request
- `Notification` -- fire-and-forget, no response expected

The `JsonRpcId` supports both numeric and string identifiers, plus a `Null` variant for
parse-level failures where no request ID is available.

### 1.2 Full Lifecycle

1. **Initialize:** Client sends `initialize` with protocol version + capabilities (filesystem,
   terminal, MCP). Agent responds with `InitializeResult` declaring its capabilities
   (`load_session`, `prompt_capabilities`, `mcp_capabilities`).

2. **Session creation:** Client sends `session/new` with optional `SessionNewParams` (session
   name, MCP servers). Agent responds with `SessionNewResult` containing:
   - Server-generated `session_id`
   - Available modes (`ModesInfo` with `current_mode_id` + `available_modes`)
   - 9 configuration dropdowns (`config_options`)

3. **Prompting:** Client sends `session/prompt` with content blocks (text, resource references,
   diffs). Agent streams `session/update` notifications:
   - `AgentMessageChunk` -- visible text output
   - `AgentThoughtChunk` -- internal reasoning (thinking)
   - `ToolCall` / `ToolCallUpdate` -- tool invocation progress
   - `Plan` -- structured plan entries with priority and status
   - `UsageUpdate` -- token/cost tracking
   - `ConfigOptionUpdate` -- dynamic config changes
   - `SessionInfoUpdate` -- session metadata
   - `AvailableCommandsUpdate` -- slash commands

4. **Config update:** Client sends `session/config/update` with option ID and new value.

5. **Cancellation:** Client sends `session/cancel` notification.

6. **Session list/load:** Client sends `session/list` or `session/load` for session management.

### 1.3 Bidirectional Requests (Agent -> Editor)

The ACP protocol allows the agent to request actions from the editor:

| Method | Direction | What |
|--------|-----------|------|
| `fs/read_text_file` | Agent -> Editor | Read file through editor's VFS |
| `fs/write_text_file` | Agent -> Editor | Write file through editor's VFS |
| `terminal/create` | Agent -> Editor | Create terminal for command execution |
| `terminal/output` | Agent -> Editor | Stream terminal output |
| `terminal/wait_for_exit` | Agent -> Editor | Wait for terminal command to finish |
| `session/request_permission` | Agent -> Editor | Prompt user for approval |
| `elicitation/create` | Agent -> Editor | Show structured form dialog |

This bidirectional model means the agent can use editor-mediated I/O instead of direct
filesystem access, enabling sandboxed execution.

### 1.4 Content Blocks

Three content block types defined in `ContentBlock` enum:

```rust
enum ContentBlock {
    Text { text: String },
    Resource { resource: ResourceRef },  // File URI reference
    Diff { path: String, diff: String }, // Unified diff
}
```

The `ResourceRef` currently only supports `File { uri: String }`. Image, audio, and embedded
context capabilities are declared in `PromptCapabilities` but not yet implemented as content
block variants.

### 1.5 Tool Call Updates

Tool calls flow through two update types:
- `ToolCall` -- initial declaration with `tool_call_id`, `title`, `kind` (Edit/Create/Delete/
  Terminal/Other), `status` (Pending/InProgress/Completed/Failed), and optional content
- `ToolCallUpdate` -- status/content update for an existing call

This streaming model means the editor can show real-time progress for long-running tool calls
(e.g., test execution, file indexing).

### 1.6 Plan Entries

Plan updates carry structured `PlanEntry` items with `content` (text), `priority` (High/Medium/Low),
and `status` (Pending/InProgress/Completed). The editor renders these as a checklist, giving
the user visibility into the agent's work plan.

---

## 2. ACP State Machine (Pure, Zero I/O)

### 2.1 The Pipeline

Located at `crates/roko-acp/src/pipeline.rs`. This is the core innovation of the ACP subsystem:
a pure state machine with no I/O, no async, no side effects.

```
Pending -> Strategizing -> Implementing -> AutoFixing -> Gating -> Reviewing -> Committing -> Complete | Halted | Cancelled
```

### 2.2 Events and Actions

**Events (inputs to the state machine):**

| Event | Source | Meaning |
|-------|--------|---------|
| `Start` | Runner | Begin pipeline |
| `StrategyComplete { brief }` | Strategist agent | Strategy phase done |
| `StrategySkipped` | Template rule | Skip strategy |
| `AgentCompleted { output, files_changed }` | Any agent | Agent finished |
| `AgentFailed { error }` | Any agent | Agent crashed/timed out |
| `GatesPassed` | Gate runner | All gates pass |
| `GateFailed { gate, output }` | Gate runner | Gate failed |
| `ReviewApproved { summary }` | Reviewer agent | Code approved |
| `ReviewRevise { findings }` | Reviewer agent | Changes needed |
| `CommitDone { message }` | Git | Commit created |
| `Timeout` | Runner | Time budget exceeded |
| `BudgetExceeded` | Runner | Cost budget exceeded |
| `UserCancel` | User/editor | User cancelled |

**Actions (outputs from the state machine):**

| Action | Effect | Who performs it |
|--------|--------|----------------|
| `SpawnStrategist { prompt }` | Analyze prompt, produce brief | Runner |
| `SpawnImplementer { prompt, context }` | Write code | Runner |
| `SpawnAutoFixer { error_output }` | Fix gate errors | Runner |
| `RunGates` | Run compile/test/clippy | Runner |
| `SpawnReviewer { diff_context }` | Review changes | Runner |
| `Commit` | Create git commit | Runner |
| `Done` | Pipeline complete | Runner |
| `Halt { reason }` | Save state, stop | Runner |

### 2.3 Transition Table

The `step()` method is exhaustively pattern-matched with 16 explicit transitions:

| Current Phase | Event | Next Phase | Action |
|--------------|-------|------------|--------|
| Pending | Start (Full template) | Strategizing | SpawnStrategist |
| Pending | Start (Standard/Express) | Implementing | SpawnImplementer |
| Strategizing | StrategyComplete | Implementing | SpawnImplementer (with brief) |
| Strategizing | StrategySkipped | Implementing | SpawnImplementer |
| Implementing | AgentCompleted | Gating | RunGates |
| Implementing | AgentFailed (retries left) | Implementing | SpawnImplementer (with error) |
| Implementing | AgentFailed (no retries) | Halted | Halt |
| AutoFixing | AgentCompleted | Gating | RunGates |
| AutoFixing | AgentFailed (retries left) | Implementing | SpawnImplementer |
| AutoFixing | AgentFailed (no retries) | Halted | Halt |
| Gating | GatesPassed (has review) | Reviewing | SpawnReviewer |
| Gating | GatesPassed (no review) | Committing | Commit |
| Gating | GateFailed (retries left) | AutoFixing | SpawnAutoFixer |
| Gating | GateFailed (no retries) | Halted | Halt |
| Reviewing | ReviewApproved | Committing | Commit |
| Reviewing | ReviewRevise (retries left) | Implementing | SpawnImplementer |
| Reviewing | ReviewRevise (no retries) | Committing | Commit (accept with caveats) |
| Committing | CommitDone | Complete | Done |
| Any | UserCancel | Cancelled | Halt |
| Any | Timeout | Halted | Halt |
| Any | BudgetExceeded | Halted | Halt |

**Test coverage:** 10 unit tests cover all major paths -- express, standard, full, gate failure,
review revise, user cancel, max iterations halt.

### 2.4 Pipeline State Fields

`PipelineState` carries everything the state machine needs for decisions:
- `phase: PipelinePhase` -- current state
- `template: WorkflowTemplate` -- Express/Standard/Full
- `iteration: u32` -- current loop count
- `max_iterations: u32` -- halt threshold
- `original_prompt: String` -- for re-dispatch
- `strategist_brief: Option<String>` -- from strategy phase
- `review_findings: Vec<String>` -- accumulated across iterations
- `last_gate_failure: Option<String>` -- for autofix context
- `files_changed: u32` -- from last implementation
- `commit_message: Option<String>` -- after commit

### 2.5 Why This Design Matters

The pure state machine + effect driver pattern is the only architecture in roko that scales:
- **Testable:** `step()` is a pure function with no dependencies. All 16 transitions tested.
- **Deterministic:** Same events always produce same actions. No race conditions.
- **Inspectable:** Full state serializable to JSON for debugging, snapshots, resume.
- **Composable:** Multiple state machines can run in parallel without interference.
- **Auditable:** Event log = complete execution history.

Compare this to `orchestrate.rs` (22K lines, 80+ fields on `PlanRunner`, tangled I/O and
state transitions) and it's clear which approach should win.

---

## 3. Workflow Templates

### 3.1 Three Built-in Templates

| Template | Flow | When Selected |
|----------|------|---------------|
| **Express** | Implement -> Gate -> Commit | Short prompts (<15 words) with "fix"/"typo"/"rename" |
| **Standard** | Implement -> Gate -> Review -> Commit | Default |
| **Full** | Strategy -> Implement -> Gate -> Review -> Commit | Long prompts (>50 words) or "refactor"/"architecture" |

### 3.2 Auto-Selection Logic

`WorkflowTemplate::auto_select(prompt)` in `crates/roko-acp/src/pipeline.rs`:

```
word_count < 15 AND contains(fix|typo|rename|update|bump)  ->  Express
word_count > 50 OR contains(files|modules|system|architecture|refactor)  ->  Full
otherwise  ->  Standard
```

**Limitation:** This is purely keyword/length based. No semantic analysis, no task complexity
estimation, no historical data. The `SkillSelector` in `crates/roko-agent/src/composition.rs`
has a richer routing model (by category, complexity band, reasoning level, speed priority,
quality profile) that could replace this.

### 3.3 Eight Planned Templates (Not Yet Implemented)

| # | Template | Flow | Status |
|---|----------|------|--------|
| 1 | Express | Implement -> Gate -> Commit | Implemented |
| 2 | Standard | Implement -> Gate -> QuickReview -> Commit | Implemented |
| 3 | Full | Strategy -> Implement -> Gate -> [Architect + Auditor + Scribe parallel] -> Verdict -> Commit | Partial (no parallel review) |
| 4 | Research | Research -> Synthesize -> [Optional Writer] | Not implemented |
| 5 | PRD-to-Ship | PRD -> Plan Generator -> [Per task: Standard/Full] -> Merge Queue | Not implemented |
| 6 | Review-Only | Git Diff -> [Architect + Auditor parallel] -> Verdict | Not implemented |
| 7 | Documentation | Changed Files -> Scribe -> Critic -> [Fix Loop] -> Commit | Not implemented |
| 8 | Custom | User-defined step sequence in TOML | Not implemented |

**Missing infrastructure for templates 4-8:**
- Parallel agent dispatch within a pipeline phase (Full template needs this)
- Sub-pipeline spawning (PRD-to-Ship needs nested Standard/Full)
- Read-only mode (Review-Only and Documentation need this)
- Custom step parsing from TOML

### 3.4 Complexity-Based Auto-Selection (Planned)

| Complexity | Files | Strategist | Reviews | Max Iterations |
|------------|-------|------------|---------|----------------|
| Trivial | 1 | No | No | 1 |
| Simple | 1-3 | No | No | 2 |
| Standard | 3-10 | No | QuickReviewer | 2 |
| Complex | 10+ | Yes | Full panel | 2 |

Risk escalation: touching core crates or 3+ dependencies bumps complexity one tier.

**Not yet implemented.** Would require `estimate_complexity()` based on:
- File count from prompt analysis
- Dependency graph depth from `crates/roko-orchestrator/src/dag.rs`
- Historical data from episode logger

---

## 4. Agent Roles (10 Defined)

### 4.1 Role Definition Table

| Role | Can Edit | Can Run Cmds | Default Model | Purpose |
|------|----------|-------------|---------------|---------|
| Conductor | No | No | opus | Orchestration decisions |
| Strategist | No | No | opus | Analyze prompt, produce implementation brief |
| Implementer | Yes | Yes | sonnet | Write code |
| AutoFixer | Yes | Yes | haiku/sonnet | Fix gate errors |
| Researcher | No | Yes | sonnet/perplexity | Research topics, gather context |
| QuickReviewer | No | Yes | sonnet | Fast code review |
| Architect | No | Yes | opus | Deep architectural review |
| Auditor | No | Yes | opus | Security/correctness audit |
| Scribe | Yes (docs) | No | sonnet | Documentation |
| Critic | No | Yes | sonnet | Review critic |

### 4.2 Role Definitions in Code

Formal TOML manifests for 6 core roles in `crates/roko-core/src/builtin_roles/core_roles.toml`.

In the ACP runner (`crates/roko-acp/src/runner.rs`), roles are instantiated by the
`spawn_role_agent()` function which:
1. Resolves the model from config (or role default)
2. Builds a system prompt via `PromptAssemblyService` or the 9-layer builder
3. Configures tool permissions based on role (edit/run restrictions)
4. Sets MCP config if available
5. Applies safety hooks via `build_settings_json()` from `crates/roko-agent/src/claude_cli_agent.rs`

### 4.3 Safety Hooks (ClaudeCliAgent)

The `build_settings_json()` function generates a hooks configuration that blocks destructive
commands in plan worktrees:

| Hook | Blocks |
|------|--------|
| `Bash(git checkout *)` | Branch switching |
| `Bash(git switch *)` | Branch switching |
| `Bash(git branch -m *)` | Branch renaming |
| `Bash(git push *)` | Pushing (roko handles merges) |
| `Bash(rm -rf *)` | Destructive deletion |
| `Bash(rm -fr *)` | Destructive deletion |
| `Bash(rm -r *)` | Recursive deletion |

These hooks are injected via the `--settings` flag to the Claude CLI. They fire as PreToolUse
hooks on the Bash tool.

### 4.4 Role Escalation

The ACP pipeline supports model escalation on retry:

```
Attempt 1 (haiku) -> fail -> Attempt 2 (sonnet) -> fail -> Attempt 3 (opus)
```

This is implemented in the pipeline state machine: when `AgentFailed` fires and
`iteration < max_iterations`, the runner can spawn a new agent with a more capable model.
The model selection is handled by `CascadeRouter` in `crates/roko-learn/src/cascade_router.rs`.

---

## 5. Failure Handling

### 5.1 Gate Failure Ladder

1. **Simple compile error** -> AutoFixer (lightweight, fast model)
   - `PipelinePhase::Gating` + `GateFailed` -> `PipelinePhase::AutoFixing`
   - AutoFixer gets error output as context

2. **Complex compile error** -> back to Implementer with error context
   - `PipelinePhase::AutoFixing` + `AgentFailed` -> `PipelinePhase::Implementing`
   - Implementer gets both gate error and autofix failure

3. **Test failure** -> Implementer retry with test output
   - Same path as complex compile error

4. **Same error hash 2+ times** -> convergence stall -> escalate model or halt
   - Detected by `convergence` field on `ModelCallService`
   - `ConvergenceDetectionCell` tracks recent output hashes

### 5.2 Review Failure Paths

- **Quick-fixable** -> QuickFix -> re-gate
- **Complex** -> Implementer retry with structured review feedback
  - `ReviewRevise { findings }` -> accumulated in `review_findings: Vec<String>`
  - Findings passed as context to next SpawnImplementer
- **Docs-only** -> DocRevision only
- **Max iterations reached** -> commit anyway with caveats
  - The state machine accepts with caveats rather than halting

### 5.3 Global Interrupts

Three interrupts can fire from any phase:
- `UserCancel` -> Cancelled -> Halt
- `Timeout` -> Halted { reason: "Timeout in phase {phase}" } -> Halt
- `BudgetExceeded` -> Halted { reason: "Budget exceeded in phase {phase}" } -> Halt

The `CancelToken` in `crates/roko-acp/src/session.rs` provides cooperative cancellation:
- `cancelled: Arc<AtomicBool>` -- flag
- `notify: Arc<Notify>` -- wake waiters
- `cancel()` sets flag + notifies
- `is_cancelled()` checks flag
- `wait_cancelled()` async waits

---

## 6. Inter-Agent Communication

Agents do NOT communicate directly. This is a deliberate architectural decision.
Communication flows through five mechanisms:

### 6.1 File-Based Context Injection

Task outputs are persisted via `save_task_output()` and loaded via `load_prior_task_outputs()`.
Each agent's output is stored as a file in the plan's output directory. Subsequent agents
receive prior outputs as prompt context.

### 6.2 Review Feedback Loop

Reviewer output is parsed for structured verdicts via `parse_structured_review_verdict()` in
`crates/roko-gate/src/review_verdict.rs`. The parsed `ReviewVerdictContext` contains:
- Verdict (approve/revise/reject)
- Findings with severity (major/minor/nit)
- Suggested changes with file paths and line numbers

These findings are injected into the implementer's retry prompt.

### 6.3 Strategy Briefs

Strategist output is stored in `PipelineState.strategist_brief` and injected into the
implementer's `context` field when `SpawnImplementer` fires.

### 6.4 Gate Feedback

Structured `GateResult` from `crates/roko-acp/src/workflow.rs` with gate name, pass/fail,
output text, and duration. Failed gate output is stored in `PipelineState.last_gate_failure`
and injected into the autofix or implementer retry prompt.

### 6.5 Knowledge System

The `DispatchKnowledge` struct in `crates/roko-acp/src/knowledge.rs` queries two stores:
- **KnowledgeStore** from `roko-neuro` -- durable knowledge with tiers (Persistent/Consolidated/
  Working/Transient), kinds (Insight/Heuristic/AntiPattern/Warning/CausalLink/StrategyFragment),
  and relevance scoring
- **PlaybookStore** from `roko-learn` -- playbooks with steps, action kinds, expected signals,
  and success rates

Results are rendered both as:
- A visible card (shown in the editor UI as a tool call completion)
- Prompt context (injected into the system prompt for the agent)

### 6.6 Event Bus

`RokoEvent` variants published to TUI, HTTP SSE, and orchestrator subscribers via
`crates/roko-runtime/src/event_bus.rs`.

---

## 7. Batch Processing Protocol (Runner System)

### 7.1 Per-Batch Cycle

1. **Compose prompt** = context pack (5 files) + delegation guidance + batch-specific prompt
2. **Spawn Codex** in worktree with `codex exec --full-auto`
3. **Verify** (6 gates): scope gate, diff gate, required-terms gate, cargo check, clippy, tests
4. **Commit** if OK, backup + retry if not

### 7.2 Context Pack (~4000 tokens)

Five standard files loaded per batch:

| File | Content | Purpose |
|------|---------|---------|
| `00-ACP-RULES.md` | Agent behavior rules | Compliance |
| `01-ACP-PROTOCOL-PRIMER.md` | Protocol overview | API reference |
| `02-ROKO-ARCHITECTURE.md` | Crate map + layer structure | Navigation |
| `03-TYPE-REFERENCE.md` | Key type definitions | Code correctness |
| `04-EXISTING-PATTERNS.md` | Code patterns to follow | Style consistency |

### 7.3 Environment Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `ACP_MODEL` | `gpt-5.4` | Codex model |
| `ACP_REASONING` | `high` | Reasoning effort |
| `ACP_TIMEOUT` | `5400` | Per-batch timeout (90 min) |
| `ACP_MAX_RETRIES` | `2` | Retries per batch |

### 7.4 Recovery

- Runtime logs at `tmp/acp-runner/logs/<run-id>/`
- Per-batch status in `.result` files
- Composed prompts saved as snapshots for debugging
- Worktree state backed up on failure
- Re-run failed batch: `bash run-acp.sh --continue last --only ACP05 --force`

---

## 8. Session Configuration (9 Dropdowns)

The ACP protocol exposes session configuration via `ConfigOption` structs with
`id`, `name`, `option_type` (Select/Toggle), `category`, `current_value`, and `options`.

| # | Dropdown | Options | Default |
|---|----------|---------|---------|
| 1 | Model | All models from roko.toml | Configured default |
| 2 | Effort | Low / Medium / High / Max | Medium |
| 3 | Temperament | Conservative / Balanced / Aggressive / Exploratory | Balanced |
| 4 | Routing | Auto / Manual | Auto |
| 5 | Clippy | On / Off | On |
| 6 | Tests | On / Off | On |
| 7 | Workflow | None / Express / Standard / Full / Auto | Auto |
| 8 | Review | None / Quick / Standard / Thorough | Standard |
| 9 | Retries | 1 / 2 / 3 | 2 |

These are built in `crates/roko-acp/src/session.rs` by the `build_config_options()` method and
sent to the client in the `SessionNewResult`. The client can update any option via
`session/config/update`, which triggers a `ConfigOptionUpdate` notification back to the client
with the new state.

---

## 9. ACP Runner Current Limitations

### 9.1 Architectural Limitations

1. **Single-prompt only** -- The ACP pipeline handles one prompt per session. Multi-task plans
   require the orchestrator (`orchestrate.rs`) or runner v2 (`workflow_engine.rs`). Extending
   ACP to handle multi-task plans would require adding DAG scheduling to the state machine.

2. **No DAG execution** -- Each pipeline runs linearly (strategy -> implement -> gate -> review).
   Parallel task execution within a plan is not supported.

3. **Serial agents only** -- One role per phase. The Full template's intended parallel review
   (Architect + Auditor + Scribe) is not implemented.

### 9.2 Integration Gaps

4. **Cost/token fields initialized but not always updated** -- `WorkflowRun.total_cost_usd`
   and `total_tokens` are `Option<f64>` / `Option<u64>`, starting as `None`. Updated when the
   provider reports usage, but some providers (Claude CLI) don't report per-turn usage.

5. **Prompt experiments not wired** -- The ACP runner does not participate in A/B prompt
   experiments from `crates/roko-learn/src/prompt_experiment.rs`.

6. **No section effectiveness tracking** -- The prompt assembly does not track which sections
   contributed to success/failure.

### 9.3 Learning Integration (Partially Wired)

The ACP runner in `crates/roko-acp/src/bridge_events.rs` now integrates with:
- Episode logging via `EpisodeLogger`
- Cascade router via `CascadeRouter`
- Knowledge queries via `query_dispatch_knowledge()`
- Playbook queries via `PlaybookStore`
- Cost tracking via `CostTable`

What's still missing:
- Prompt experiment recording
- Section effectiveness measurement
- Conductor bandit updates
- Full SPC threshold updates (only basic EMA is wired)

---

## 10. The Workflow Run Wrapper

`crates/roko-acp/src/workflow.rs` defines `WorkflowRun`, which wraps `PipelineState` with:
- `run_id: String` -- unique UUID-based identifier
- `started_at: DateTime<Utc>` -- creation timestamp
- `completed_at: Option<DateTime<Utc>>` -- terminal timestamp
- `total_cost_usd: Option<f64>` -- accumulated cost
- `total_tokens: Option<u64>` -- accumulated tokens
- `agents_spawned: u32` -- agent count

Plus helper types:
- `GateResult` -- per-gate outcome (name, pass/fail, output, duration_ms)
- `ReviewFinding` -- per-finding (severity, description, file, line)

### Status Summary Format

```
Active Workflow: Standard
Phase: Implementing (iteration 1/2)
Duration: 45s
Cost: $0.0234
Tokens: 12450
Agents spawned: 2
```

---

## 11. The Unified Implementation Plan (Converging 3 Runtimes)

### 11.1 Architecture Decision

The ACP state machine's pure `step()` function is the correct foundation. All three runtimes
should converge on this pattern:

| Runtime | Should become |
|---------|--------------|
| ACP pipeline | **Foundation** -- extend with multi-task support |
| Runner v2 | **Adaptor** -- thin wrapper calling ACP pipeline per-task |
| orchestrate.rs | **Deprecated** -- extract valuable features, delete the rest |

### 11.2 Seven Phases (80+ Tasks)

**Phase 0 -- Foundation Services**
- `ModelCallService` (one trait for all model calls) -- **exists** at
  `crates/roko-agent/src/model_call_service.rs`
- `PromptAssemblyService` (merging runner v2's assembler with 9-layer builder) -- **exists** at
  `crates/roko-compose/src/prompt_assembly_service.rs`
- `FeedbackService` (one event stream feeding all learning components) -- **exists** at
  `crates/roko-learn/src/feedback_service.rs`
- `PersistenceService` (crash-safe state management) -- partially at
  `crates/roko-runtime/src/pipeline_state.rs`

**Phase 1 -- Execution Engine**
- Extend ACP's pure state machine to handle multi-task plans
- DAG-aware scheduling extracted from event loop
- Replace 3000-line tokio::select! loop

**Phase 2 -- Model Routing Integration**
- Wire `CascadeRouter` into runner v2 dispatch
- Connect `ModelCallService` to `AdaptiveThresholds`
- Knowledge-informed model selection via neuro store

**Phase 3 -- Safety Wiring**
- Unify safety hook injection across all runtimes
- `build_settings_json()` called from every agent spawn path
- Agent contract enforcement (currently falls back to permissive default)

**Phase 4 -- Observability**
- Episode recording from runner v2
- Cost tracking from all paths
- Unified event bus

**Phase 5 -- Entry Point Convergence**
- All CLI commands use the same dispatch path
- Remove `dispatch_direct.rs` (currently 800 LOC of mixed concerns)
- Remove `auth_detect.rs` bypass

**Phase 6 -- Dead Code Retirement**
- Delete orchestrate.rs (22K LOC)
- Delete redundant dispatch paths
- Expected deletions: ~110K LOC
- Expected replacements: ~5K LOC

**Phase 7 -- Proof Runs**
- 12 proof runs validating all workflow templates
- Regression test suite

### 11.3 Design Principles

1. **Pure state machine + side-effect driver** (ACP got this right)
2. **One dispatch path for every model call** (ModelCallService)
3. **Structured prompt assembly through 9-layer builder** (PromptAssemblyService)
4. **Feedback as normalized event stream** (FeedbackService)
5. **Preserve what's valuable, delete what's overengineered** (VCG auction payments, daimon
   PAD model, pheromone system, HDC fingerprints)

---

## 12. Missing Innovations

### 12.1 Wave-Level Gating for Multi-Agent Workflows

The mega-parity runner proved that wave-level gating (compile after N agents finish) is 10x
more efficient than per-agent gating. The ACP pipeline does not support this.

**Implementation path:**
1. Add `WaveAccumulating` phase to `PipelinePhase`
2. Add `WaveComplete { agents_done: u32 }` event to `PipelineEvent`
3. `step()` transitions `WaveAccumulating -> Gating` when all wave agents finish
4. Runner tracks wave membership via `BTreeMap<WaveId, Vec<AgentId>>`

### 12.2 Speculative Execution

Start the next wave's agents before the current wave's gates finish. If gates fail, cancel
the speculative agents. This overlaps gate latency with agent dispatch.

**Risk:** Wasted compute if gates fail. Mitigated by:
- Only speculate when gate pass rate >90% (from `AdaptiveThresholds`)
- Cancel speculative agents immediately on gate failure
- Use cheaper models for speculative agents

### 12.3 Agent Context Windowing

Instead of giving each agent the full codebase context, compute a minimal "context window"
based on the files the agent is likely to touch. The `ContextAssembler` in
`crates/roko-neuro/src/episode_completion.rs` already computes relevance scores.

### 12.4 Feedback-Driven Template Selection

Use historical episode data to learn which template works best for which prompt type.
The `CascadeRouter` already does this for model selection; extend to template selection.
