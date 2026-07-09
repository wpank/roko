# Master Implementation Checklist

**Read AGENT-RULES.md before starting ANY task.**

Every task must pass its verification protocol before being marked complete.

---

## Document Map

| File | Content |
|------|---------|
| `AGENT-RULES.md` | 14 non-negotiable rules for executing agents |
| `00-CONTEXT.md` | Problem statement, root causes, architecture target, design decisions |
| `01-WAVE-A-ENGINE.md` | Engine convergence + event architecture (8 tasks) |
| `02-WAVE-B-WIRING.md` | Dead code wiring (7 tasks, all parallel) |
| `03-WAVE-C-CLI.md` | CLI redesign: 5 verbs using existing WorkflowEngine (6 tasks) |
| `04-WAVE-D-EVENTS.md` | RuntimeEvent extension + SseAdapter coverage (5 tasks) |
| `05-WAVE-E-DEMO.md` | Demo redesign: 5 scenarios + custom panels (8 tasks) |
| `CHECKLIST.md` | This file — master tracking |

---

## Milestone 1: First Working Demo (Critical Path)

These 6 tasks produce one scenario running end-to-end with live SSE panels:

- [ ] **A2+A3** Wire serve_runtime.rs state integration (StateHub + FeedbackFacade + Projection)
- [x] **A5+A6** HTTP ingest endpoint + HttpEventSink in roko-runtime
  - Implemented: route-level ingest tests, HttpEventSink batching/auth tests, 1000-event batch limit
  - Manual E2E: start `roko serve`, POST to `/api/events/ingest`, observe on SSE + JSONL
- [ ] **D1+D2** Extend RuntimeEvent variants + SseAdapter match arms
- [ ] **C1** `roko do` command using existing WorkflowEngine + ScopeResolver
- [ ] **E3** useOperationEvents hook (build on existing useBenchSSE/EventStreamContext)
- [ ] **E5** Pipeline scenario (1 terminal + PipelineStagesPanel) — simplest scenario

---

## Wave A: Engine Convergence + Event Architecture

### Prerequisite (cleanup)
- [ ] **A1** Port missing legacy features to v2 event_loop.rs
  - Port daimon/somatic modulation hooks
  - Port HDC fingerprint per-episode to FeedbackFacade sink
  - Delete: `workspace_context()`, `dispatch_and_record()` (dead code)
  - Verify: `cargo test --workspace` passes after ports

- [ ] **A4** Deprecate legacy PlanRunner
  - Already feature-gated OFF (`#[cfg(feature = "legacy-orchestrate")]`)
  - Add #[deprecated] attribute to PlanRunner struct
  - Verify: `grep -rn 'PlanRunner::new\|PlanRunner::from' crates/ --include='*.rs' | grep -v test | grep -v target/` → empty

### Event infrastructure
- [ ] **A2+A3** Wire serve_runtime.rs state integration
  - Add `state_hub: SharedStateHub` to RokoCliRuntime
  - Pass app_state.state_hub.clone() at construction
  - Wire FeedbackFacade (clone from commands/plan.rs:420-508)
  - Wire Projection
  - Verify: SSE stream shows events during serve-initiated plan run

- [x] **A5** Event ingest endpoint (POST /api/events/ingest)
  - Route accepts RuntimeEvent JSON (NOT ServerEvent)
  - Passes through EventConsumer pipeline (SseAdapter + JsonlLogger)
  - Batch variant for arrays (1000-event max)
  - Localhost-only by default; configurable via auth token or allowlist
  - Route-level tests: single 202, batch 202, batch >1000 error, SSE subscriber, JSONL logger, non-loopback rejection
  - Files: `crates/roko-serve/src/routes/event_ingest.rs`

- [x] **A6** HttpEventSink (generic, in roko-runtime)
  - Non-blocking mpsc + background POST task
  - Activated by ROKO_SERVE_URL env var
  - 50ms batching window, 32 event max batch
  - Tests: URL trim, bearer auth, batch size, non-blocking saturated channel, mock axum server
  - Files: `crates/roko-runtime/src/http_event_sink.rs`

- [x] **A7** PTY environment injection
  - terminal.rs injects ROKO_SERVE_URL, ROKO_SESSION_ID, ROKO_SERVER_AUTH_TOKEN
  - Status: implemented in `crates/roko-serve/src/terminal.rs`
  - Verify: `echo $ROKO_SERVE_URL` in PTY shows server URL

- [x] **A8** ACP event bridge
  - Thin adapter over HttpEventSink (from A6) -- CognitiveEvent->RuntimeEvent mapping
  - Tests: all CognitiveEvent variants mapped, summarize_content helpers, stop_reason_label coverage
  - Files: `crates/roko-acp/src/event_forward.rs`

---

## Wave B: Dead Code Wiring (ALL PARALLEL — can overlap with Wave A)

- [ ] **B1** Wire TimeoutConfig — replace Duration::from_secs() hardcodes (~12 sites)
  - NO new parameter threading — RunConfig.roko_config already provides access
  - Verify: Custom `[timeouts] agent_dispatch_secs = 30` in roko.toml → agent times out at 30s

- [ ] **B2** Wire InlineTerminal as output pipeline (merges old B2+C5)
  - Replace 10 inline `if stream_to_stderr { eprintln!(...) }` with InlineTerminal calls
  - Use existing primitives: ToolCallBlock, DiffBlock, CostMeter
  - Verify: `roko plan run` shows structured ◆/│/└ format

- [ ] **B3** Wire GatePipeline configuration via GatePipelineBuilder
  - Build ComposedGatePipeline from `[[gates.rungs]]` TOML config
  - Use existing Verify trait (the gate plugin interface)
  - Keep select_rungs(complexity) as fallback when no custom config
  - Verify: Custom [[gates.rungs]] in roko.toml → only those gates run

- [ ] **B4** Wire AdaptiveBudget — replace budget_for() with adaptive version
  - Replace ~5 `budget_for(role)` calls with `adaptive_budget_for(role, context_window)`
  - Verify: haiku gets fewer tokens than opus for same task

- [ ] **B5** Consolidate layout boundary + wire Workspace (Phase 1)
  - Current decision: Workspace is public path boundary; RokoLayout remains roko-fs/internal migration catalog
  - Add missing accessors to Workspace before migrating callers
  - Wire into runner/, commands/plan.rs, serve_runtime.rs by subsystem
  - Verify: `grep '\.join(".roko' crates/roko-cli/src/runner/ ...` → empty

- [ ] **B6** Wire SafetyLayer on all backends (Exec, Gemini, Cursor)
  - Add SafetyLayer (non-optional) to all non-ToolDispatcher backends
  - Verify: Dangerous tool call blocked on non-ToolDispatcher backend

- [ ] **B7** Wire validate_against_schema() in plan loader
  - Call in plan_loader.rs after parsing + in `roko plan validate`
  - Verify: Invalid TOML (missing required 'id' field) → clear schema error

---

## Wave C: CLI Surface

### Core command
- [~] **C1** `roko do` command with ScopeResolver
  - Uses existing WorkflowEngine from roko-runtime (1876 lines, already built)
  - ScopeResolver: LLM-first classification (haiku ~$0.001) with heuristic fallback
  - Extends existing PlanComplexity enum (don't create new Formality)
  - Flags: --plan, --yes, --ghost, --compare, --continue, --no-cascade
  - Status: wired through WorkflowEngine + heuristic ScopeResolver; PRD/plan pipelines and work-item resume not complete
  - Verify: `roko do "add a function"` auto-classifies, plans, executes, gates

### Independent commands (can start immediately)
- [ ] **C2** `roko show` command
  - Subcommands: costs, agents, knowledge, plans, learning, history, <work-id>, --live
  - Verify: `roko show` produces formatted overview with real .roko/ data

- [ ] **C3** `roko think` + `roko tune` commands
  - Verify: `roko tune model haiku` → actually writes config. `roko think` → returns analysis.

### After C1
- [ ] **C4** Work items (first-class, named, resumable)
  - Stored in `.roko/work/` as JSON
  - Records git commits (for undo)
  - Verify: Interrupt `roko do`, run `roko do` again → offers to resume by name

- [ ] **C5** `roko undo` command
  - Reverts commits from a work item
  - Verify: `roko undo` after `roko do` → changes reverted

- [ ] **C6** `POST /api/do` HTTP route
  - Calls WorkflowEngine::run(), returns work_item_id + stream_url
  - Verify: `curl POST /api/do` → 202 + events in SSE stream

---

## Wave D: Event Coverage

**IMPORTANT: D1 must precede D2. D2 extends SseAdapter for variants D1 defines.**

- [ ] **D1** Extend RuntimeEvent with ~8 new variants
  - InferenceStarted/Completed/Failed, AgentTrace, TaskFailed, RunStarted/Completed, KnowledgeIngested/Consumed
  - Verify: `cargo build --workspace` passes, unit tests for serialization

- [ ] **D2** Extend SseAdapter match arms for all new RuntimeEvent variants
  - Each variant → snake_case event type + JSON data
  - Verify: SSE stream shows new event types when triggered

- [ ] **D3** Inference tracking (InferenceObserver trait in roko-agent)
  - Trait in roko-agent, concrete impl in roko-cli that emits RuntimeEvent
  - Wire into dispatcher's LLM call path
  - Verify: `curl SSE | grep inference` shows events during LLM calls

- [ ] **D4** AgentTrace per-turn events
  - Emit from agent tool loop alongside TurnCompleted
  - Include: tool_calls, reasoning, usage
  - Verify: SSE shows agent_trace with tool_calls during plan execution

- [ ] **D5** Remaining high-value emissions
  - TaskFailed (on gate failure), RunStarted/Completed, KnowledgeIngested/Consumed
  - Verify: Each event type individually confirmed in SSE when triggered

---

## Wave E: Demo Redesign

### Prerequisites
- [ ] **E1** Fix terminal session fundamentals (resolveRoko, --repo removal, state reset)
  - Random UUID markers for resolveRoko, remove --repo injection, reset module state
  - Verify: All scenarios execute without "command not found" errors

- [ ] **E2** Archive 14 old scenarios
  - Move to `src/lib/scenario-runners/archive/`
  - Verify: Demo sidebar shows only 5 new scenarios

- [ ] **E3** SSE client infrastructure (useOperationEvents hook)
  - Build on existing EventStreamContext + useBenchSSE pattern
  - Filter by operation ID and event types
  - Verify: DevTools shows active EventSource, hook returns filtered events

### Scenarios (ALL PARALLEL after E1+E2+E3)
- [ ] **E4** Cost scenario + CostComparisonPanel
  - 2 panes (naive vs cascade) + 3-column comparison sidebar
  - Verify: Two panes run, sidebar shows live diverging costs, delta at end

- [ ] **E5** Pipeline scenario + PipelineStagesPanel
  - 1 terminal + stage progression sidebar
  - Verify: Stages light up progressively, tasks show pass/fail, gates inline

- [ ] **E6** Memory scenario + KnowledgeDeltaPanel
  - 2 panes (cold then warm, sequential) + comparison sidebar
  - Verify: Cold run generates knowledge, warm run consumes it, delta visible

- [ ] **E7** ISFR scenario + SwarmPanel
  - 4 panes + agent flow sidebar with directional arrows
  - Verify: 4 agents run, arrows show flow, ISFR rate computed

- [ ] **E8** Oracle scenario + OraclePanel
  - DeFi data collection: chain check → data agent → strategy agent
  - Requires mirage-rs with Ethereum mainnet fork
  - Verify: Chain connected, rates extracted, knowledge written, recommendation produced

---

## Parallelization Map

```
Time →
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Wave A (engine + events):
  A1+A4 (cleanup) ──────────┐
                             ├─→ A2+A3 (wire serve state)
  A5 (ingest endpoint) ─────┤
                             └─→ A6 (HttpEventSink) → A7 (PTY) → A8 (ACP)

Wave B (fully parallel, can START alongside Wave A):
  B1 ─── B2 ─── B3 ─── B4 ─── B5 ─── B6 ─── B7

Wave C (C1 needs A2+A3 done for streaming; C2/C3 independent):
  C1 (roko do) → C4 (work items) → C5 (undo)
  C1 → C6 (POST /api/do)
  C2 (roko show) ─── independent, start anytime
  C3 (think/tune) ── independent, start anytime

Wave D (can start after A2+A3, parallel with B and C):
  D1 (extend RuntimeEvent) → D2 (extend SseAdapter)
  D3 ─── D4 ─── D5  (all parallel after D2)

Wave E (after D2 and C1 exist):
  E1 + E2 + E3 ───→ E4 | E5 | E6 | E7 | E8 (all parallel)
```

---

## Success Criteria (The Demo Works)

The phase is COMPLETE when all of the following are true:

1. `roko do "add a health check"` runs end-to-end with streaming inline output
2. `curl -N http://localhost:6677/api/events/stream` shows 10+ event types during execution
3. `POST /api/do` triggers execution and returns work_item_id
4. Demo app connects via SSE, all 5 scenarios execute, custom panels update live
5. `roko show` displays formatted state overview with real data
6. `roko tune model haiku` actually changes the config (no confirmation theater)
7. `roko undo` reverts last work item's commits
8. Work items are named and resumable (`roko do` offers to continue)
9. Gates are configurable via roko.toml (`[[gates.rungs]]` section)
10. Timeouts are configurable via roko.toml (`[timeouts]` section)
11. Oracle scenario reads real on-chain data from mirage-rs fork
12. `cargo clippy --workspace --no-deps -- -D warnings` passes clean
13. `cargo test --workspace` passes clean
