# Runner 10-12 — Observability + Entry Points + Retirement

> **Give this entire file to a fresh agent.** These three plans form the convergence + cleanup phase.

---

## Context

Codebase: `/Users/will/dev/nunchi/roko/roko`. Goals:
- **Plan 10:** Unify two event enums (`RuntimeEvent` + `DashboardEvent`) and two JSONL files into one
- **Plan 11:** Route every entry point through `WorkflowEngine` or `ModelCallService`
- **Plan 12:** Delete ~100K LOC of dead/replaced code

**Read first:**

1. `tmp/workflow/implementation-plans/10-observability-projection.md`
2. `tmp/workflow/implementation-plans/11-entry-point-convergence.md`
3. `tmp/workflow/implementation-plans/12-retirement-deletion.md`
4. `crates/roko-core/src/runtime_event.rs` — `RuntimeEvent` enum
5. `crates/roko-runtime/src/projection.rs` — `RuntimeProjection`
6. `crates/roko-cli/src/commands/plan.rs` — plan run entry
7. `crates/roko-cli/src/agent_exec.rs` — PRD/research entry

---

## Phase A: Observability (Plan 10)

### A1: Extend `RuntimeEvent`

Add ~20 new variants per plan 10 step 1: `ToolCallStarted/Completed/OutputDelta`, `AgentThinkingDelta`, `PlanStarted/Completed`, `TaskStarted/Completed`, `MergeStarted/Succeeded/Failed`, `ReviewStarted/Approved/Revised`, `AutoFixStarted/Completed`, `ModelCallStarted/Completed`, `SafetyAlert/Warning`, `AgentBlocked`, `CostUpdate`, `PromptAssembled`.

### A2: Migrate `DashboardEvent` consumers

Search: `rg 'DashboardEvent' crates/ --type rust`. For each: replace with equivalent `RuntimeEvent`. Key consumers: TUI (`app.rs`, `state.rs`), SSE (`sse.rs`), WS (`ws.rs`).

### A3: One canonical JSONL

Pick `.roko/events.jsonl`. Delete all references to `.roko/runtime-events.jsonl`. Add migration at startup.

### A4: HTTP route consolidation

Replace multiple routes with: `/api/runs`, `/api/runs/{id}`, `/api/runs/{id}/events`, `/api/runs/{id}/transcript`.

### A5: TTL enforcement

Add `ProjectionEnvelope::is_stale(now)` and `load_or_refresh` to `crates/roko-runtime/src/projection.rs`.

### A6: TUI reads projection

Replace disk loading in `tui/dashboard.rs` with `RuntimeProjection::dashboard_view()`.

---

## Phase B: Entry Point Convergence (Plan 11)

### B1: Migrate `roko plan run`

Route to `WorkflowEngine::run(PlanExecution)` by default. Keep `--use-event-loop` fallback.

### B2: Migrate `agent_exec.rs` callers

Use `WorkflowEngine::run(Express)` for all PRD/research/plan-generate flows.

### B3: ACP default mode

Route through `WorkflowEngine::run(Express)` + `AcpEventBridge` consumer.

### B4: `roko chat` decision

Delete (preferred) or thin to 30-LOC wrapper over `run_unified_inline`.

### B5: HTTP assembly support

`POST /api/inference/complete` optional `assembly` body field triggers `PromptAssemblyService`.

### B6: Shared `auto_select`

Move `WorkflowConfig::auto_select(prompt)` to `crates/roko-runtime/src/pipeline_state.rs`.

---

## Phase C: Retirement (Plan 12)

**CRITICAL: Only start Phase C after Phases A+B are complete and soaking for 1+ week.**

Execute the 21-step deletion sequence from plan 12. Each step is its own PR. Key items:

1. Delete 12 noisy hooks (03-F)
2. Delete `runtime_feedback/` (03-H)
3. Delete `DashboardEvent`
4. Delete `extract_clean_text`
5. Delete `runner/event_loop.rs`, `task_dag.rs`, `persist.rs::RunStateSnapshot`
6. Delete `dispatch_direct.rs`
7. Extract unique features from `orchestrate.rs` (knowledge routing → `roko-learn`, skill extraction → `roko-learn`, gate classifier → `roko-runtime`)
8. Delete `orchestrate.rs` → `tmp/legacy/` backup
9. Remove `legacy-orchestrate` feature
10. Delete `roko-orchestrator` legacy components (UnifiedTaskDag, ParallelExecutor, coordination.rs, replan.rs)
11. Delete `roko-daimon/` crate
12. Delete `roko-compose/src/auction.rs`
13. Verify: `cargo build --workspace && cargo test --workspace`
14. Verify: LOC reduction ≥ 100K

---

## Verification

```bash
# One event enum
rg 'pub enum RuntimeEvent|pub enum DashboardEvent' crates/ --type rust
# only RuntimeEvent

# One JSONL
rg 'runtime-events.jsonl' crates/ --type rust
# returns 0

# No legacy feature gate
rg 'cfg.*legacy-orchestrate' crates/ --type rust
# returns 0

# LOC check
find crates/ -name '*.rs' | xargs wc -l | tail -1
# significantly less than baseline
```
