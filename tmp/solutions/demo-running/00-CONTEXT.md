# Full Scope: Architecture Redesign Context

## Executive Summary

46 backend batches shipped. 40% is dead code. The demo app is unchanged. The CLI has
35 subcommands nobody uses. The streaming infrastructure EXISTS but is disconnected.
The demo has 14 broken scenarios that should be 5.

This plan addresses ALL of it as one coherent redesign. The key insight from audit:
**most infrastructure already exists** — the work is primarily WIRING, not BUILDING.

---

## The 5 Problems (Root Causes, Not Symptoms)

### Problem 1: Disconnected Event Architecture

**Root cause**: When `roko serve` dispatches plan execution, `serve_runtime.rs:274`
creates a local `SharedStateHub` that is disconnected from `AppState.state_hub`.
The runner emits events into the void.

Additionally:
- `RuntimeEvent` (12 variants in roko-core) is the canonical event type
- `SseAdapter` (roko-serve) already converts `RuntimeEvent → SSE JSON` — just needs more match arms
- `DashboardEventBridge` (roko-serve) already converts `RuntimeEvent → DashboardEvent`
- ACP now has a `CognitiveEvent` -> `RuntimeEvent` bridge through `HttpEventSink`
- PTY terminals now inject `ROKO_SERVE_URL` so subprocesses can forward events
- **ServerEvent (55 variants) is redundant** — should be eliminated, not bridged

The infrastructure EXISTS. It's just never connected.

### Problem 2: Dual Execution Engine (NEARLY RESOLVED)

**Root cause**: `orchestrate.rs` (legacy) and `event_loop.rs` (v2) coexist.

**Current state** (discovered by audit):
- `orchestrate.rs` is already feature-gated: `#[cfg(feature = "legacy-orchestrate")]`
- It is NOT in default features — no production binary includes it
- `serve_runtime.rs` ALREADY uses v2 (`crate::runner::run()` at line 277)
- The only remaining work: port 2-3 features, then delete

### Problem 3: CLI Doesn't Match Mental Model

**Root cause**: 35+ subcommands at 3-4 levels. The user's model is "do something" not
"create a PRD, draft it, generate a plan, run the plan."

**Key discovery** (audit): `WorkflowEngine` ALREADY EXISTS in `roko-runtime` (1876 lines).
It has a `run()` method, is used by `shared_runs.rs`. The facade task (C1) does NOT need
to build a new one — it needs to expose the existing one via `roko do`.

### Problem 4: Demo App is Unchanged

**Root cause**: SCENARIO-REDESIGN.md was written but never decomposed into work.
14 old scenarios remain. No custom panels. No SSE consumption.

**Key discovery** (audit): SSE infrastructure already exists:
- `EventStreamContext` in roko-serve (manages SSE connections)
- `SseAdapter` converts RuntimeEvent to SSE format
- `useBenchSSE` React hook in demo-app (already consumes SSE for benchmarks)
- Build `useOperationEvents(opId, types[])` on existing infrastructure, don't replace it

### Problem 5: Dead Code Everywhere (40% of batches)

**Key discovery** (audit): Much of the "wiring" is simpler than originally scoped:
- `RunConfig` already holds `Arc<RokoConfig>` — config access requires NO new parameters
- Path architecture was revised after this source plan: `Workspace` (roko-core) is the
  public workspace path boundary; `RokoLayout` (roko-fs) remains a lower-level layout catalog
  during migration. See `tmp/taskrunner/AGENT.md` and task 004 for the current decision.
- Gate pipeline already has `Verify` trait + `ComposedGatePipeline` — use it, don't add dual-path
- `InlineTerminal` + 11 primitives are FULLY BUILT — C5 is pure wiring (~150-250 lines)

---

## Architecture After This Work

```
User
  │
  ├── CLI: `roko do "intent"`
  │     └── WorkflowEngine (exists in roko-runtime, 1876 lines)
  │           ├── ScopeResolver (extends PlanComplexity → Trivial/Small/Medium/Large)
  │           ├── event_loop.rs v2 (streaming, cross-task, gates)
  │           ├── GatePipeline (configurable via roko.toml, Verify trait)
  │           ├── FeedbackFacade (episodes, routing, knowledge)
  │           └── EventConsumer trait → [SseAdapter, DashboardBridge, JsonlLogger]
  │
  ├── HTTP: `roko serve`
  │     ├── POST /api/do → WorkflowEngine
  │     ├── GET /api/events/stream → SSE from RuntimeEvent via SseAdapter
  │     ├── POST /api/events/ingest ← subprocess/ACP forwarding
  │     └── 85 internal routes (unchanged)
  │
  ├── Demo: 5 scenarios with custom sidebar panels
  │     └── useOperationEvents(opId) → live updating panels
  │
  └── ACP: Editor integration
        └── HttpEventSink → POST /api/events/ingest → EventConsumer pipeline
```

Key properties:
1. **Single execution path**: event_loop.rs v2, used everywhere
2. **Single event type**: `RuntimeEvent` (12 variants), consumed by multiple adapters
3. **Universal event sink**: HTTP POST for out-of-process forwarding
4. **5 user commands**: do, think, show, tune, undo
5. **5 demo scenarios**: Cost, Pipeline, Memory, ISFR, Oracle (each with custom panel)

---

## Event Architecture (The Real Design)

The audit revealed the correct pattern already exists but isn't fully wired:

```
RuntimeEvent (roko-core, 12 variants)
  │
  ├── EventConsumer trait (roko-core/src/foundation.rs:421)
  │     ├── DashboardEventBridge → DashboardEvent → StateHub → TUI snapshot
  │     ├── SseAdapter → SSE JSON → EventSource clients
  │     ├── JsonlLogger → .roko/events.jsonl
  │     └── [NEW] HttpForwarder → POST /api/events/ingest (for subprocesses)
  │
  └── [NEW] Extend RuntimeEvent variants:
        Currently: 12 variants
        Target: ~20 variants (add InferenceStarted/Completed, AgentTrace, etc.)
```

**What we DON'T do**: Bridge DashboardEvent → ServerEvent. That was the old plan and it's
wrong — it inverts the dependency direction (StateHub is in roko-core, ServerEvent in roko-serve).
Instead, we extend RuntimeEvent and extend SseAdapter's match arms.

---

## Sequencing (2 Milestones)

### Milestone 1: First Working Demo (~6-8 critical tasks)

The minimum path to a single scenario running end-to-end with live SSE panels:

| # | Task | What | Est |
|---|------|------|-----|
| 1 | A3 | Wire AppState.state_hub into serve_runtime's RunConfig | 2-3h |
| 2 | A6+A7 | HTTP ingest endpoint + HttpEventSink in runner | 4-6h |
| 3 | Extend SseAdapter | Add RuntimeEvent variants + match arms (replaces D1+D2) | 3-4h |
| 4 | C3 | `roko do` command using existing WorkflowEngine | 3-4h |
| 5 | E8 | useOperationEvents hook (build on existing useBenchSSE) | 2-3h |
| 6 | E4 | Pipeline scenario (1 terminal + PipelineStagesPanel) | 4-6h |

**Total**: ~17-23 hours for first end-to-end demo

### Milestone 2: Full Implementation

Everything else: remaining scenarios, CLI commands, dead code wiring, full event coverage.

---

## Sequencing (5 Waves)

| Wave | What | Unblocks |
|------|------|----------|
| A: Engine + Events | Wire StateHub, ingest endpoint, HTTP sink, PTY env, ACP bridge | All streaming + demo |
| B: Dead Code Wiring | TimeoutConfig, GatePipeline config, AdaptiveBudget, Layout, SafetyLayer | Correctness, configurability |
| C: CLI Surface | `roko do/show/think/tune/undo`, inline output, work items | Demo scenarios |
| D: Event Coverage | Extend RuntimeEvent + SseAdapter, inference tracking, agent traces | Demo panel data |
| E: Demo Redesign | 5 scenarios, custom sidebar panels, SSE consumption | Presentation |

**Parallel opportunities:**
- Wave B is fully independent of A (different files, different concerns)
- Wave B tasks are all independent of each other (maximally parallel)
- Within Wave C, C4 is independent (reads state, no engine dependency)
- Within Wave E, scenarios E3-E7 are all parallel after E1+E2+E8

---

## Key Files (Current → After)

| Current | Role | After |
|---------|------|-------|
| `crates/roko-cli/src/orchestrate.rs` (22K LOC) | Legacy engine (feature-gated OFF) | DELETED after port |
| `crates/roko-cli/src/runner/event_loop.rs` | v2 engine | Single canonical engine |
| `crates/roko-cli/src/serve_runtime.rs` | Serve→runner bridge (already uses v2) | Uses AppState.state_hub |
| `crates/roko-core/src/runtime_event.rs` | RuntimeEvent (12 variants) | Extended to ~20 variants |
| `crates/roko-core/src/foundation.rs` | EventConsumer trait | Unchanged |
| `crates/roko-serve/src/adapters.rs` | SseAdapter (RuntimeEvent → SSE) | Extended match arms |
| `crates/roko-serve/src/lib.rs` | DashboardEventBridge | Unchanged |
| `crates/roko-runtime/src/workflow_engine.rs` | WorkflowEngine (1876 lines) | Exposed via `roko do` |
| `crates/roko-core/src/workspace.rs` | Workspace path boundary | Public workspace path type |
| `crates/roko-fs/src/layout.rs` | RokoLayout (30+ accessors) | Lower-level layout catalog during migration |
| `crates/roko-acp/src/bridge_events.rs` | CognitiveEvent (8 variants) | + HttpEventSink bridge |
| `crates/roko-serve/src/terminal.rs` | PTY spawn | + ROKO_SERVE_URL injection |
| `demo/demo-app/src/` | 14 scenarios | 5 scenarios + custom panels |

---

## Critical Design Decisions

1. **RuntimeEvent is canonical** — NOT ServerEvent, NOT DashboardEvent. Those are projections.
2. **EventConsumer is the adapter pattern** — Each consumer (SSE, TUI, logger) implements the trait.
3. **WorkflowEngine already exists** — Don't build a new one. Expose the existing one.
4. **Workspace is the public path abstraction** — `RokoLayout` remains live for roko-fs
   internals and documented migration exceptions. This supersedes the older RokoLayout
   canonical guidance in this demo-running plan.
5. **PlanComplexity is the formality type** — NOT a new Formality enum. Extend the existing one.
6. **InlineTerminal is the output system** — NOT RunOutputSink. B2's StderrSink is skipped.
7. **GatePipeline has Verify trait** — NOT a dual-path dispatch. Use the existing plugin system.
8. **RunConfig holds Arc<RokoConfig>** — Config access requires NO new parameter threading.

---

## Sources Consumed

| Source | Key Finding | Covered By |
|--------|-------------|------------|
| `CURRENT-STATE.md` | 40% dead code, demo unchanged, CLI unusable | All waves |
| `WIRING-AUDIT.md` | 14 dead-code items cataloged with wiring instructions | Wave B |
| `BATCH-GAPS.md` | 18/56 fully done, 37 partial, 1 never started | Waves A, B |
| `CLI-REDESIGN.md` | 5-verb model, progressive formality, WorkflowEngine | Wave C |
| `SCENARIO-REDESIGN.md` | 5 scenarios with custom panels | Wave E |
| `SCENARIO-DETAILS.md` | Full specs per scenario (Oracle = DeFi, not health check) | Wave E |
| `06-STREAMING-DESIGN.md` | stderr streaming format, RunOutputSink design | Waves B, D |
| `TERMINAL-SESSION-REDESIGN.md` | PTY command resolution bugs (P0-P6) | Wave E (E1) |
| `NEXT-PHASE.md` | Wave A-D sequencing, LOC estimates | Sequencing |
| Code audit (18 agents) | Existing infrastructure discovery, merge opportunities | All waves |

### Sources Explicitly Superseded

| Source | Decision |
|--------|----------|
| `09-UX-WORKFLOW-VISION.md` (72KB) | Superseded by the 5-verb model in CLI-REDESIGN.md |
| `16-cli-tui-rendering-convergence.md` | Partially covered by C5 (inline output). Full TUI convergence OUT OF SCOPE. |
| `dogfood/09-MAY6-DEMO-BUILD.md` | Contradicts Wave E's web app approach. We follow Wave E. |

### Not Covered (Future Work)

| Source | Why Deferred |
|--------|-------------|
| `04-DEMO-UI-REDESIGN.md` | Lower priority than custom per-scenario panels |
| W8-A (blanket clippy suppression removal) | Mechanical, iterative. Not architecture. |
| W10-C (.roko/memory → .roko/learn path unification) | 3-file fix. Can be done alongside Wave B. |
