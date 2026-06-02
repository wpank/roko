# Next Phase: UX Overhaul + Demo Redesign

## Design Inputs

This document synthesizes proposals from:

- `SCENARIO-REDESIGN.md` — 5 focused demo scenarios with custom sidebar panels
- `tmp/subsystem-audits/05-01/42-workflow-redesign-suggestion.md` — "5 verbs" CLI (do/think/show/tune/undo)
- `tmp/solutions/roko/09-UX-WORKFLOW-VISION.md` — aggregate→funnel→execute
- `tmp/solutions/roko/15-UX-PLAN.md` — 6-phase UX overhaul
- `tmp/mori-diffs/36-WORKFLOW-ENTRYPOINT-ORCHESTRATION-AUDIT.md` — WorkflowEngine convergence
- `tmp/demo-req/IMPLEMENTATION-PLAN.md` — inline ratatui, Clack-style output, 18 primitives
- `tmp/workflow/implementation-plans/11-entry-point-convergence.md` — entry point unification
- `tmp/binary-issues/MASTER-INDEX.md` — 90+ issues including "confirmation theater"
- `tmp/dogfood/09-MAY6-DEMO-BUILD.md` — nunchi CLI wrapper for pitch demo

---

## Part 1: CLI Simplification

### Current Problem

The CLI has 35+ subcommands at 3-4 levels of nesting. Nobody uses the PRD pipeline
manually. Operators think in terms of intent ("add a health check") not in terms of
pipeline stages ("idea → draft → plan → run"). The real workflow is: describe what you
want, iterate. Everything else is friction.

### Proposed: 5 Primary Verbs

| Verb | Replaces | What It Does |
|------|----------|--------------|
| `roko do "<prompt>"` | `prd idea`, `prd draft`, `prd plan`, `plan run`, `run` | Intent-driven execution. Auto-classifies complexity (trivial to multi-agent). One command for everything. |
| `roko think "<topic>"` | `research *`, `prd draft`, `knowledge query` | Research and reasoning without action. Returns analysis, not code. |
| `roko show [what]` | `status`, `dashboard`, `learn *`, `plan list` | Inspect state. No args = overview. `show costs`, `show agents`, `show knowledge`. |
| `roko tune [what]` | `learn tune *`, `config set`, `config experiments` | Adjust behavior. `tune routing`, `tune gates`, `tune budget`. |
| `roko undo [what]` | (no equivalent today) | Revert last action, cancel in-progress work, rollback a plan. |

### Progressive Formality

`roko do` auto-classifies intent. The user never picks a formality level. The system does.

| Class | Trigger signal | Execution path |
|-------|---------------|----------------|
| **Trivial** | Typo fix, rename, single-line change | Direct dispatch, no plan, single model call |
| **Small** | Add a function, patch a config | Lightweight plan, 1-3 tasks, fast model |
| **Medium** | New feature, refactor a module | Full plan, 5-15 tasks, mixed models, gates |
| **Large** | Architectural change, new subsystem | PRD generation, research phase, multi-agent, full gate pipeline |
| **Ambiguous** | Unclear scope | Ask one clarifying question, then proceed |

Classification happens via a cheap pre-call (fast model, no tool use) on the raw prompt
before any heavy work starts. Misclassification is recoverable: the plan is shown before
execution and the user can say "this is bigger than I thought."

### Backwards Compatibility

All existing commands remain as aliases and continue to work. They print a non-blocking
hint line:

```
hint: try `roko do 'implement X'` instead (roko prd plan is still available)
```

Aliases are formally deprecated in the next minor version and removed in the next major.

---

## Part 2: Streaming and Real-Time

### Server-Side

`roko serve` gains three new SSE endpoints:

| Endpoint | Emits |
|----------|-------|
| `/api/v1/stream/plans/{id}` | Task status changes, gate results, cost deltas |
| `/api/v1/stream/agents/{id}` | Raw agent output lines as they are produced |
| `/api/v1/stream/costs` | Live cost accumulation across all active operations |

Each event is a JSON object with a `type` discriminant. The demo app and any external
dashboard subscribe via `EventSource`. The existing StateHub `DashboardEvent` broadcast
already handles the internal fanout; these endpoints are thin adapters over it.

### CLI-Side

Terminal output shifts from alternate-screen TUI to inline streaming:

- Tool calls shown as they execute (similar to Claude Code output)
- Clack-style spinners, checkmarks, and progress bars rendered inline
- Gate results printed as they complete, not buffered to the end
- `roko do` streams progress to the terminal in real time with no mode switching

The existing `roko dashboard` TUI stays for operators who want the full view. The CLI
default becomes the streaming inline format.

### Demo App

- `EventSource` connection per active operation replaces polling
- Sidebar numbers (cost, tokens, elapsed time) update live without a full re-render
- Terminal output panel streams via SSE rather than writing complete snapshots
- Progress indicators show gate completion and task completion percentages

---

## Part 3: Demo Redesign (5 Scenarios)

The current demo has 14 scenarios with a generic sidebar. The replacement has 5
scenarios, each with a custom React sidebar panel that displays the metric that matters
for that scenario's proof point.

### Scenario 1: Cost — The System Is the Variable

**Proof point**: 3-5x cost reduction through cascade routing and smart model selection.

**Command**: `roko do "add health check endpoint" --compare naive`

**What happens**: The same task runs twice in parallel — once with naive single-model
dispatch, once through the CascadeRouter. Both streams are visible simultaneously.

**Sidebar**: `CostComparisonPanel` — 3 columns (Naive / Cascade / Delta) with live
updating token counts, USD costs, and time-to-completion. A running percentage
reduction is shown as the gap grows.

**Duration**: ~90 seconds

### Scenario 2: Knowledge — Compounding Intelligence

**Proof point**: Shared knowledge means the thousandth agent joins smarter than the first.

**Command**: `roko do "add rate limiter"` (run twice, second run uses stored knowledge)

**What happens**: First run executes normally and persists findings to the neuro store.
Second run pulls those findings before dispatch and the agent reaches the implementation
faster.

**Sidebar**: `KnowledgeDeltaPanel` — knowledge items retrieved, confidence delta, tokens
saved vs. a cold run, estimated time saved. The panel shows specifically which knowledge
items were used so the mechanism is legible.

**Duration**: ~120 seconds

### Scenario 3: Coordination — Identity and Roles

**Proof point**: Multi-agent coordination with visible roles and message flow.

**Command**: `roko do "redesign auth system" --show-agents`

**What happens**: The system generates a plan, dispatches multiple agents with different
roles (Architect, Implementer, Reviewer), and routes subtasks between them. The
delegation is made visible rather than hidden inside the orchestrator.

**Sidebar**: `AgentFlowPanel` — agent role cards, directional arrows showing message
flow, per-agent task status, and a live feed of delegation events. Not a diagram that
was drawn ahead of time — it builds itself as the run progresses.

**Duration**: ~90 seconds

### Scenario 4: Memory — Durability and Learning

**Proof point**: The system gets faster and cheaper on repeated work.

**Command**: Run the same task twice with a visible comparison.

**What happens**: First run establishes a baseline. Second run uses routing history,
episode data, and stored patterns to complete faster and with fewer tokens.

**Sidebar**: `LearningCurvePanel` — cost and time curves across runs, routing adaptation
log (which models were tried vs. selected), and a projected cost curve extrapolated to N
future runs.

**Duration**: ~120 seconds (approximately 60 seconds per run)

### Scenario 5: Build — The Full Pipeline

**Proof point**: End-to-end self-hosting. The system reads intent, plans, executes,
validates, and persists.

**Command**: `roko do "implement <non-trivial feature>"` with no flags

**What happens**: The full pipeline runs: intent classification, optional research,
plan generation, multi-agent execution, gate validation, and persistence. Each stage
is visible as it transitions.

**Sidebar**: `PipelineStagesPanel` — stage-by-stage progress (Classify → Research →
Plan → Execute → Gate → Persist) with timing per stage, streaming logs from the active
stage, and a completion percentage. Gate results shown inline with pass/fail indicators.

**Duration**: ~120 seconds

### Sidebar Architecture

Each panel is a self-contained React component that:

- Subscribes to the relevant SSE stream on mount
- Manages its own local state (no global store writes)
- Receives an `operationId` prop and fetches initial state via REST
- Unsubscribes and resets on unmount

The scenario runner provides `operationId` to the panel. The panel is responsible for
everything visual from that point forward.

---

## Part 4: Wire Dead Code Before Building New Code

Several subsystems are built but not connected to the runtime. Wiring these costs less
than building new features and removes confusion about what actually runs.

| Dead code | Location | Wire action |
|-----------|----------|-------------|
| `RunOutputSink` | `crates/roko-cli/src/agent_events.rs` | Call it in the agent dispatch path instead of dropping output |
| `Workspace` | `crates/roko-core/` | Replace scattered `workdir.join()` calls throughout orchestrate.rs |
| `GateRungConfig` | `crates/roko-gate/` | Load from config at startup instead of constructing inline |
| `TimeoutConfig` | `crates/roko-core/` | Read from `roko.toml` instead of using hardcoded durations |
| `AdaptiveBudget` | `crates/roko-learn/` | Replace static `budget_for()` calls with the adaptive version |

Each of these is a small, isolated change. Do them before Wave B starts.

---

## Part 5: Converge to One Execution Engine

There are currently two execution paths: `event_loop.rs` (v2, production) and
`orchestrate.rs / PlanRunner` (legacy, where most of the "wired" features live).
Both need to stay working during transition, but the goal is one path.

**Keep**: `event_loop.rs` as the canonical executor. It is the production path and the
one that receives new features going forward.

**Redirect**: `orchestrate.rs PlanRunner` becomes a thin adapter that calls
`WorkflowEngine`, which calls `event_loop.rs`. The "wired" features in orchestrate.rs
(EpisodeLogger, ProcessSupervisor, gate pipeline, etc.) migrate to the WorkflowEngine
layer one at a time.

**Unify entry points**: `roko run`, `roko plan run`, and the new `roko do` all go
through `WorkflowEngine → event_loop.rs`. No special-casing per entry point.

---

## Implementation Order

Work is sequenced so each wave unblocks the next. Nothing in Wave B depends on
Wave C being done; the waves can partially overlap.

### Wave A: Foundation

These changes are prerequisites for everything else. Estimated ~2,500 LOC Rust + ~500 LOC TypeScript.

1. `WorkflowEngine` facade — single public entry point for all operations, wraps event_loop.rs
2. SSE streaming endpoints in roko-serve — thin adapters over existing StateHub events
3. Wire dead code — RunOutputSink, TimeoutConfig, Workspace, GateRungConfig, AdaptiveBudget

### Wave B: CLI

These changes deliver the new CLI surface. Estimated ~1,500 LOC Rust.

4. `roko do` command — intent classifier pre-call, progressive formality, dispatch to WorkflowEngine
5. `roko show` command — unified inspect surface, replaces status/learn/plan-list
6. Streaming CLI output — Clack-style inline progress, no alternate screen by default
7. Old command deprecation hints — non-blocking one-liners on deprecated paths

### Wave C: Demo

These changes deliver the new demo experience. Estimated ~2,000 LOC TypeScript/React.

8. 5 new scenario runner modules (one per scenario above)
9. 5 custom sidebar panel components (CostComparison, KnowledgeDelta, AgentFlow, LearningCurve, PipelineStages)
10. SSE client integration in demo-app — EventSource hooks, reconnect logic
11. Archive old 14 scenarios (move to `src/lib/scenario-runners/archive/`, keep importable)

### Wave D: Polish

These changes complete the surface and clean up. Estimated ~800 LOC Rust.

12. `roko think`, `roko tune`, `roko undo` commands
13. Deprecation warnings on remaining old commands
14. Help text and completion scripts updated for new verb surface

---

## Estimated Scope

| Wave | LOC added | LOC removed/redirected |
|------|-----------|------------------------|
| A — Foundation | ~2,500 Rust + ~500 TS | ~300 (dead code cleanup) |
| B — CLI | ~1,500 Rust | ~200 (entry point consolidation) |
| C — Demo | ~2,000 TS/React | ~2,500 (old scenario files) |
| D — Polish | ~800 Rust | ~500 (alias boilerplate) |
| **Total** | **~7,300** | **~3,500** |

Net addition is roughly 3,800 LOC. The deleted code is not lost — the scenarios are
archived, not removed, and the old commands remain as aliases.

---

## Success Criteria

The phase is complete when:

- `roko do "add a health check"` runs end-to-end with no other commands
- Terminal output streams in real time with no mode switching
- The demo app connects via SSE and sidebar numbers update live
- All 5 scenarios run and each custom panel displays live data
- `roko status`, `roko plan run`, and `roko prd plan` still work (backwards compat)
- `cargo clippy --workspace --no-deps -- -D warnings` passes clean
