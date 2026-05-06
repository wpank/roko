# Implementation Plan

## What We Have (Already Built)

| Component | Where | Usable As-Is |
|-----------|-------|--------------|
| ACP server (stdio, full protocol) | `crates/roko-acp/` | ✓ |
| Conversation history + persistence | `session.rs` | ✓ |
| Mode-specific system prompts | `session.rs` | ✓ |
| Provider dispatch (6 providers, 22 models) | `bridge_events.rs` | ✓ |
| 44 slash commands | `session.rs` + `bridge_events.rs` | ✓ |
| Plan updates (ACP `plan` session update) | `types.rs` | ✓ |
| Tool call cards (ACP `tool_call` update) | `types.rs` | ✓ |
| Plan executor + DAG | `roko-cli/src/orchestrate.rs` | ✓ (needs wiring) |
| Gate pipeline (11 gates, 7 rungs) | `roko-gate/` | ✓ (needs wiring) |
| Agent dispatch (Claude CLI + API) | `roko-agent/` | ✓ (needs wiring) |
| SystemPromptBuilder (9 layers) | `roko-compose/` | ✓ (needs wiring) |
| ProcessSupervisor | `roko-runtime/` | ✓ |
| CascadeRouter | `roko-learn/` | ✓ |
| Episode logging | orchestrate.rs | ✓ |

## What Needs To Be Built

### Phase 1: Workflow Runner Core (in `roko-acp`)

**Goal**: A single prompt can trigger a multi-phase pipeline instead of just a single agent call.

1. **WorkflowRun struct** in session.rs
   - Phase tracking, iteration count, task state
   - Persisted alongside session

2. **PipelineStateMachine** (new module: `pipeline.rs`)
   - State machine that receives events and emits actions
   - Handles: Strategize → Implement → Gate → Review → Commit
   - Failure transitions: gate fail → autofix/reimpl, review reject → reimpl

3. **`run_workflow_pipeline()`** in bridge_events.rs
   - Replaces direct `run_claude_cognitive_task` when workflow != "none"
   - Spawns agents sequentially per pipeline phase
   - Emits plan updates at each phase transition
   - Emits tool call cards for gates and reviews

4. **Config option**: `workflow` dropdown (express/standard/full/auto/none)
   - `none` = current behavior (single agent, no pipeline)
   - Others trigger the pipeline runner

5. **Slash commands**: `/workflow list/status/cancel/resume`

### Phase 2: Gate Integration

**Goal**: Gates run automatically between implementation and review.

1. Wire `roko-gate` crate's `run_gate_pipeline()` into the ACP pipeline
2. Emit `tool_call` updates for each gate step
3. On failure: format error output, pass to autofix/reimpl agent
4. Configurable gates per session (already have clippy/test toggles)

### Phase 3: Review Integration

**Goal**: Reviewer agent(s) run after gates pass, with structured verdict.

1. Add `reviewer` dispatch path (read-only, structured JSON output)
2. Parse review verdict, decide: approve → commit, revise → retry
3. Emit review findings as tool_call_update content
4. Feed review feedback into implementer retry prompt

### Phase 4: Multi-Task Plans

**Goal**: A plan with multiple tasks executes through the pipeline per-task.

1. Wire `roko-orchestrator` plan executor into ACP
2. Task DAG scheduling (respect `depends_on`, `max_parallel`)
3. Per-task pipeline execution
4. Plan-level progress updates (task completion counts)
5. Merge queue for ordered commits

### Phase 5: Custom Workflows

**Goal**: Users define pipeline templates in TOML.

1. Workflow template loading from `.roko/workflows/*.toml`
2. Template registry + discovery (for slash command completion)
3. Step-based execution with role/model/gate per step
4. Slot filling: user can swap out roles in a template

### Phase 6: Triggers

**Goal**: Workflows fire automatically from events.

1. File watch trigger (notify crate, already in TUI)
2. Manual trigger (slash command)
3. Workflow-completion trigger (chain A → B)
4. Background execution (no active session needed)
5. Result persistence to `.roko/runs/`

## File-Level Changes

### New Files
| File | What |
|------|------|
| `crates/roko-acp/src/pipeline.rs` | Pipeline state machine |
| `crates/roko-acp/src/workflow.rs` | WorkflowRun, templates, config |
| `crates/roko-acp/src/runner.rs` | Multi-phase execution loop |

### Modified Files
| File | Changes |
|------|---------|
| `crates/roko-acp/src/session.rs` | Add `active_run: Option<WorkflowRun>`, workflow config option |
| `crates/roko-acp/src/bridge_events.rs` | Route to `run_workflow_pipeline()` when workflow != none |
| `crates/roko-acp/src/handler.rs` | New request types for workflow control |
| `crates/roko-acp/Cargo.toml` | Add dependency on `roko-gate`, `roko-orchestrator` |

### Existing Code To Wire (Not Rebuild)

Critical principle from CLAUDE.md: **WIRE, don't build.**

| What exists | Where | How to wire |
|-------------|-------|-------------|
| Gate pipeline runner | `roko-gate/src/pipeline.rs` | Call from `pipeline.rs` at Gate phase |
| Agent dispatch | `roko-agent/src/dispatcher/` | Call from `runner.rs` per role |
| Plan executor | `roko-orchestrator/src/` | Call from `runner.rs` for multi-task |
| SystemPromptBuilder | `roko-compose/src/` | Build role-specific prompts per agent spawn |
| ProcessSupervisor | `roko-runtime/src/` | Track spawned agents, enable cancel |
| EpisodeLogger | orchestrate.rs patterns | Log per-agent episodes |
| CascadeRouter | `roko-learn/` | Select model per role |

## Priority Order

1. **Phase 1** (Workflow Runner Core) — enables the basic pipeline pattern in ACP
2. **Phase 2** (Gate Integration) — makes the pipeline actually verify code
3. **Phase 3** (Review Integration) — closes the feedback loop
4. **Phase 4** (Multi-Task Plans) — enables real plan execution through ACP
5. **Phase 5** (Custom Workflows) — user configurability
6. **Phase 6** (Triggers) — automation, background execution

Phase 1-3 together constitute the MVP: a single prompt → pipeline → verified committed code.

## Success Criteria

### MVP (Phase 1-3)
- [ ] User can select "standard" workflow in Zed config dropdown
- [ ] Prompt triggers: implement → compile gate → test gate → review → commit
- [ ] Live plan updates show progress through phases
- [ ] Gate failures trigger auto-fix retry (max 2)
- [ ] Review rejection triggers re-implementation with feedback
- [ ] Session persistence preserves workflow state across reconnects
- [ ] `/workflow status` shows current pipeline state

### Full (Phase 1-6)
- [ ] All 8 workflow templates functional
- [ ] Multi-task plan execution with dependency ordering
- [ ] Custom workflow templates loadable from TOML
- [ ] File watch triggers fire workflows automatically
- [ ] Background workflows persist results without active session
- [ ] Cost tracking across entire workflow run
- [ ] Learning feedback improves model selection per role
