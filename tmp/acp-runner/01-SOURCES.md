# Source Materials

## Primary References

### 1. Bardo/Mori Orchestrator (`/Users/will/dev/uniswap/bardo/`)
The reference implementation. 108K LOC Rust orchestrator that ran 171 plans across 36 crates. The gold standard for what multi-agent workflow execution looks like in production.

**Key files:**
| What | Path |
|------|------|
| Agent roles + tool restrictions | `apps/mori/src/agent/roles.rs` |
| Agent spawn (claude CLI) | `apps/mori/src/agent/connection.rs` lines 2444-2620 |
| Pipeline state machine | `apps/mori/src/orchestrator/pipeline.rs` |
| Parallel executor + PlanState | `apps/mori/src/orchestrator/executor.rs` |
| Plan DAG + wave discovery | `apps/mori/src/orchestrator/dag.rs` |
| Cross-plan task DAG | `apps/mori/src/orchestrator/unified_dag.rs` |
| Complexity classification | `apps/mori/src/orchestrator/complexity.rs` |
| Prompt assembly + budgeting | `apps/mori/src/orchestrator/prompts.rs` |
| Orchestrator state machine | `apps/mori/src/orchestrator/mod.rs` |
| Review schema | `apps/mori/src/orchestrator/review.rs` |
| App config + entry | `apps/mori/src/app/mod.rs` |
| Example plan tasks | `.mori/plans/01-workspace-scaffold/tasks.toml` |
| Example review tasks | `.mori/plans/01-workspace-scaffold/review-tasks.toml` |
| Example scribe tasks | `.mori/plans/01-workspace-scaffold/scribe-tasks.toml` |

**What it teaches:**
- Pipeline state machine pattern (Preflight → Strategist → Implementer → Gate → Review → Commit)
- Complexity-based pipeline selection (trivial/simple/standard/complex)
- Role-based tool restrictions (principle of least privilege)
- Cross-plan task DAG with dependency resolution
- Review feedback loop (reviewer finds issues → implementer retries)
- Express mode (skip strategy + review for speed)
- Prompt budget management per role
- Merge queue with dependency ordering

### 2. Workflow-v1 Design (`/Users/will/dev/nunchi/roko/roko/tmp/archive/workflow-v1/`)
13 files describing the long-term vision for composable workflow primitives.

**Key concepts:**
- Three primitives: Module (smallest unit), Workflow (state-graph composition), Trigger (event → workflow)
- Node kinds: Module, SubWorkflow, Branch, FanOut/FanIn, Loop, HumanInput, Wait, Slot, Noop
- Failure strategies: Fail, Retry, RetryWithEscalation, Skip, Compensate, Replan, HumanResolve
- Capability-typed security model (module ∩ workflow ∩ workspace)
- Trigger kinds: Manual, Cron, FileWatch, Webhook, GitHub, Slack, EventBus, WorkflowCompletion
- Budget enforcement with strategies (Cancel, SkipOptional, Downgrade, HumanInput)
- Resumability via run snapshots
- Macro promotion (expose workflow params as simple knobs)
- Slot filling (typed empty positions for customization)
- 62+ builtin workflow catalog

### 3. Visual Gate Design (`/Users/will/dev/nunchi/roko/roko/tmp/archive/visual-gate-v1/`)
5-tier verification system for UI work.

**Key concepts:**
- Deterministic-first pipeline (structural → WCAG → Web Vitals → APCA → Visual judgment)
- Per-attempt artifact tracking (screenshots, traces, console, network, layout)
- Retry feedback injection (structured failure → agent prompt)
- 9 cybernetic feedback loops
- Learning integration (cascade router, experiments, adaptive thresholds)

### 4. ACP Features Checklist (`/Users/will/dev/nunchi/roko/roko/tmp/acp-features/00-ACP-FEATURES.md`)
Current state of ACP implementation.

**What's done:** Core protocol, 6 providers, 22 models, 44 slash commands, config dropdowns, conversation history, mode-specific prompts, file context injection, session persistence.

**What's missing for runner:** Workflow engine integration, trigger system, cascade router in ACP dispatch, visual gates, progress streaming beyond single-turn.

### 5. Will's Workflow Patterns (`/Users/will/dev/nunchi/nunchi-dashboard/tmp/ux-refresh-context/doc-2a-will-workflow.md`)

**Will's 6-stage loop:**
1. Research → agents gather info
2. Synthesize → generate PRDs
3. Specify → concrete plans + tasks
4. Implement → agents execute in parallel
5. Verify → manual review + gates
6. Feedback → findings become new plans, loop repeats

**Key values:**
- Premium craftsmanship (Apple/Teenage Engineering benchmarks)
- Simplicity + depth
- Composability (modular, reusable, shareable)
- Self-learning (system improves autonomously)

### 6. Roko's Existing Infrastructure

Already built and wired in roko:

| Component | Where | Status |
|-----------|-------|--------|
| Plan DAG executor | `crates/roko-cli/src/orchestrate.rs` | Wired |
| Agent dispatch (5+ backends) | `crates/roko-agent/src/dispatcher/mod.rs` | Wired |
| 11 gates, 7-rung pipeline | `crates/roko-gate/` | Wired |
| Session persistence | `crates/roko-acp/src/session.rs` | Just added |
| Conversation history | `crates/roko-acp/src/session.rs` | Just added |
| Mode-specific system prompts | `crates/roko-acp/src/session.rs` | Just added |
| SystemPromptBuilder (9-layer) | `crates/roko-compose/src/system_prompt_builder.rs` | Wired |
| EpisodeLogger | `.roko/episodes.jsonl` | Wired |
| ProcessSupervisor | `crates/roko-runtime/` | Wired |
| CascadeRouter | `.roko/learn/cascade-router.json` | Wired |
| Adaptive gate thresholds | `.roko/learn/gate-thresholds.json` | Wired |
| Knowledge store | `crates/roko-neuro/` | Wired |
| HTTP control plane (~85 routes) | `crates/roko-serve/` | Wired |
| Interactive TUI | `crates/roko-cli/src/tui/` | Wired |
