# Roko Workflow Implementation Plans

A complete set of detailed, self-contained implementation plans for everything in `tmp/workflow/` that is not yet fully implemented.

Each plan is written so a fresh agent with no prior context can execute it end-to-end. Each one includes: current code locations, exact signatures that already exist, file-by-file edits, anti-patterns to avoid, proof criteria, and dependencies on other plans.

---

## How To Use These Plans

1. **Read [00-INDEX.md](00-INDEX.md) (this file) first.** It tells you what is done, what is partial, and the order to attack remaining work.
2. **Read [`tmp/workflow/ANTI-PATTERNS.md`](../ANTI-PATTERNS.md).** Every plan references rules from this file. Violating them creates the exact mess we are trying to clean up.
3. **Pick a plan whose dependencies are satisfied.** Plans declare their dependencies explicitly at the top.
4. **Execute the plan in order**, leaving the proof checklist green at the end.
5. **When done, update this INDEX**: mark the plan COMPLETE, list the key commits or PRs, and surface any follow-up work created.

When in doubt, defer to the source of truth in `tmp/workflow/UNIFIED-IMPLEMENTATION-PLAN.md`. These plans flesh that out into actionable units, not replace it.

---

## Snapshot of Current Implementation Status

The audits below were performed on **2026-05-01** against `crates/`. Numbers (LOC) and signatures change over time — re-audit before you start a plan if it has been weeks since this index was written.

### Phase 0 — Foundation Services

| Plan | Status | Notes |
|------|--------|-------|
| [01-modelcallservice-completion.md](01-modelcallservice-completion.md) | **PARTIAL** | `ModelCaller` trait + `ModelCallService` exist (`roko-agent`); used by chat / ACP / serve. `roko run`, `roko plan run`, `agent_exec`, gate LLM judge, dreams, neuro distillation **still bypass**. `dispatch_direct` is feature-gated but lives. `stream()` has no true live streaming. |
| [02-prompt-assembly-completion.md](02-prompt-assembly-completion.md) | **PARTIAL** | `PromptAssembler` trait + `PromptAssemblyService` exist (`roko-compose`); wired into `EffectDriver` via `ServiceFactory`. ACP `runner.rs` and `session.rs` still use **inline format strings**. Chat uses `SystemPromptBuilder` directly without provenance. VCG auction not yet deleted. |
| [03-feedback-service-completion.md](03-feedback-service-completion.md) | **PARTIAL** | `FeedbackSink` + `FeedbackService` (`roko-learn`) wired into `WorkflowEngine`. Chat API path **not** attached to `FeedbackService`. `observe_multi_objective` is **only used in tests**. CLI has its own parallel `runtime_feedback::FeedbackSink` trait — naming collision and split sinks. |
| [04-persistence-service.md](04-persistence-service.md) | **NOT IMPLEMENTED** as unified service | `RunLedger` is **in-memory**; `WorkflowEngine` does not auto-checkpoint. Runner `persist.rs`/`resume.rs` provide JSONL recovery + fingerprint validation but only for the legacy `event_loop` plan runner. Three snapshot schemas (`RunStateSnapshot`, `ExecutorSnapshot`, `OrchestratorSnapshot`) coexist. |

### Phase 1 — Execution Engine

| Plan | Status | Notes |
|------|--------|-------|
| [05-pipeline-multi-task.md](05-pipeline-multi-task.md) | **PARTIAL** | `PipelineStateV2` handles single-prompt Express/Standard/Full. **Missing:** `PlanExecution` template, Verifying / DocRevision / Merging phases, FailureClassifier, replan ladder, `replan_max_per_plan` cap. |
| [06-task-scheduler-integration.md](06-task-scheduler-integration.md) | **PARTIAL** | `TaskScheduler` exists with DAG, file-overlap serialization, skip propagation. **Not used** by `WorkflowEngine` — `roko plan run` still uses `runner/event_loop.rs`. |
| [07-effect-driver-completion.md](07-effect-driver-completion.md) | **PARTIAL** | Dispatches `SpawnAgent`/`RunGates`/`Commit`/`SaveCheckpoint`. **Gate feedback always passed as empty `Vec`**. No `RunVerifySteps`, `SubmitMerge`, `SpawnScribe` actions. Multi-task fanout not implemented. Safety not threaded. |

### Phase 2 — Routing

| Plan | Status | Notes |
|------|--------|-------|
| [08-cascade-router-integration.md](08-cascade-router-integration.md) | **PARTIAL** | `CascadeRouter` is fully built and persistent. `FeedbackService::observe_model_call` calls `router.observe(...)` for `ModelCall` events. **Multi-objective observation (`observe_multi_objective`)** still only fires from legacy orchestrate. `RoutingContext` not simplified to 6 features. `TaskRequirements` filtering not active in live paths. |

### Phase 3 — Safety

| Plan | Status | Notes |
|------|--------|-------|
| [09-safety-layer-wiring.md](09-safety-layer-wiring.md) | **NOT WIRED to unified engine** | `SafetyLayer` is rich and used by `ToolDispatcher`. Contract loading falls back to **`AgentContract::restricted`** (audit's "fail-open" claim is now stale). `EffectDriver` does not call pre/post-dispatch checks. `dangerously_skip_permissions` is partially per-role (`role_allows_dangerous_skip_permissions` in `run.rs`). Per-turn cumulative spend not enforced. |

### Phase 4 — Observability

| Plan | Status | Notes |
|------|--------|-------|
| [10-observability-projection.md](10-observability-projection.md) | **PARTIAL** | `RuntimeEvent` enum + `RuntimeEventEnvelope` exist (`roko-core`). `RuntimeProjection` exists (`roko-runtime`). Two persistent JSONL streams disagree: **`.roko/events.jsonl` (StateHub `DashboardEvent`)** vs **`.roko/runtime-events.jsonl` (workflow engine `RuntimeEvent`)**. HTTP routes `/api/dashboard/runs` (workflow projection) and `/api/runs/{id}` (transcript share) target different stores. |

### Phase 5 + 6 — Convergence + Retirement

| Plan | Status | Notes |
|------|--------|-------|
| [11-entry-point-convergence.md](11-entry-point-convergence.md) | **PARTIAL** | `roko run` (v2) and ACP go through `WorkflowEngine`. `roko plan run` still uses `runner/event_loop.rs`. `agent_exec` (PRD/research) uses `spawn_agent_scoped` directly. HTTP `/api/inference` does not assemble layered system prompts. |
| [12-retirement-deletion.md](12-retirement-deletion.md) | **PARTIAL** | `orchestrate.rs` (22,756 lines) is feature-gated behind `legacy-orchestrate`, **not deleted**. `runner/event_loop.rs` is still the active plan runner. `dispatch_direct.rs` deprecated and feature-gated. VCG auction, daimon PAD, pheromones, HDC fingerprints, `extract_clean_text` (246-line monster) all still present. |

### Phase 7 — Proof Runs

| Plan | Status | Notes |
|------|--------|-------|
| [18-proof-runs.md](18-proof-runs.md) | **NOT EXECUTED** as a coordinated suite | Some unit and integration tests exist for `WorkflowEngine`, gates, persistence. The 12-point proof matrix from the unified plan has not been executed end-to-end against a single build. |

### Cross-Cutting Plans

| Plan | Source Doc | Status | Notes |
|------|------------|--------|-------|
| [13-gate-pipeline-unification.md](13-gate-pipeline-unification.md) | `tmp/workflow/11-gate-pipeline-audit.md` | **PARTIAL** | 3 gate dispatch paths: `GateService` (used by `EffectDriver`), `rung_dispatch::run_canonical_rung` (used by orchestrate + runner), ACP `run_gates` inline. Rung-3+ semantics differ between `GateService` rung map and the canonical 7-rung table. LLM judge bypass via `AgentJudgeOracle` still in orchestrate. |
| [14-providers-action-plan.md](14-providers-action-plan.md) | `tmp/workflow/providers/` | **PARTIAL** | Tool output, session id, cost display, streaming all done for the **chat session** path. `auth_detect` does not yet accept `ResolvedRuntimeConfig`. Plan / orchestrate paths still use a separate stack. `OpenAiAgent` is non-streaming by design. |
| [15-cognitive-layer-cleanup.md](15-cognitive-layer-cleanup.md) | `tmp/workflow/14-cognitive-layer-audit.md` | **NOT STARTED** | Pheromones (~68K LOC) still present. Daimon PAD (~40K LOC) still present. HDC fingerprinting still emitted. Episode distillation still reads `ANTHROPIC_API_KEY` directly. |
| [16-cli-tui-rendering-convergence.md](16-cli-tui-rendering-convergence.md) | `tmp/workflow/10-cli-chat-tui-audit.md` | **PARTIAL** | Chat session unified more (one streaming/cost/tool path). Two chat loops in `chat_inline.rs` still exist. `extract_clean_text` (246 lines, 13 formats) still in `chat.rs`. `roko chat` (REPL) still parallel to `roko` (inline). TUI unaware of tool outputs. |
| [17-demo-completion.md](17-demo-completion.md) | `tmp/workflow/demo/` | **PARTIAL & STRATEGY DRIFTING** | Web demo app at `demo/demo-app/` is heavily built (15 scenarios, BlockTicker, ISFR). The CLI-first VC demo from `DEMO-FLOW.md` (predict → run → resume → share) is partially built. Decision needed: keep both tracks or pick one. |

---

## Dependency Graph

These dependencies are **strict**: do not start a plan before its dependencies are complete.

```
01 (ModelCallService) ───┬─→ 03 (FeedbackService) ──┐
                         │                          │
                         ├─→ 02 (PromptAssembly) ──┐│
                         │                         ││
                         ├─→ 13 (Gate)             ││
                         │                         ▼▼
                         └─→ 07 (EffectDriver) ←── (services)
                                       │
04 (Persistence) ─────────────────────→ ▼
                            05 (PipelineState multi-task)
                                       │
06 (TaskScheduler integration) ────────┤
                                       ▼
                                 11 (Entry-point convergence)
                                       │
08 (CascadeRouter) ────┐               │
09 (Safety wiring)  ───┤               │
10 (Observability) ────┴──────────────→ ▼
                                  12 (Retirement)
                                       │
                                       ▼
                                 18 (Proof runs)

Cross-cutting (parallelizable, after 01–04):
  14 (Providers/dispatch)
  15 (Cognitive cleanup)
  16 (CLI/TUI rendering)
  17 (Demo)
```

Recommended order:

1. **01** ModelCallService completion — unlocks every gate
2. **02** PromptAssembly completion (parallel with 01 once `caller`/`run_id` plumbing settles)
3. **03** FeedbackService completion
4. **04** PersistenceService
5. **07** EffectDriver completion (needs 01–04)
6. **05** + **06** Pipeline + TaskScheduler (parallel; both feed 11)
7. **13** Gate unification
8. **08** + **09** + **10** in any order
9. **11** Entry-point convergence
10. **14**, **15**, **16** anytime after their dependencies (most after 01)
11. **12** Retirement
12. **18** Proof runs
13. **17** Demo when CLI surface is stable

---

## What Each Plan Contains

Every plan in this folder follows the same template:

1. **Status snapshot** — what's done as of audit date
2. **Goal** — single sentence outcome  
3. **Why this exists** — the anti-pattern this plan eliminates  
4. **Existing code you must read first** — file paths + line ranges
5. **Existing signatures** — exact code already present
6. **Implementation steps** — file-by-file changes with code sketches
7. **Anti-patterns to avoid** — referencing `ANTI-PATTERNS.md`
8. **Things NOT to do** — concrete pitfalls discovered during the audit
9. **Tests / proof criteria** — measurable success
10. **Dependencies** — which plans must finish first
11. **Estimated effort** — rough sizing (S/M/L/XL)

---

## Key Source Files (Reference)

A fresh agent should keep these handy:

| Path | Why |
|------|-----|
| `crates/roko-core/src/foundation.rs` | All foundation traits: `ModelCaller`, `PromptAssembler`, `FeedbackSink`, `GateRunner`, `EventConsumer`, `EffectExecutor`, `AffectPolicy` |
| `crates/roko-core/src/runtime_event.rs` | `RuntimeEvent` enum + `RuntimeEventEnvelope` |
| `crates/roko-runtime/src/workflow_engine.rs` | The `WorkflowEngine` facade + `WorkflowRunConfig`/`WorkflowRunReport` |
| `crates/roko-runtime/src/effect_driver.rs` | `EffectDriver` + `EffectServices` |
| `crates/roko-runtime/src/pipeline_state.rs` | `PipelineStateV2` pure FSM + `WorkflowConfig` |
| `crates/roko-runtime/src/task_scheduler.rs` | DAG scheduler |
| `crates/roko-runtime/src/run_ledger.rs` | In-memory run ledger |
| `crates/roko-agent/src/model_call_service.rs` | The `ModelCaller` impl |
| `crates/roko-agent/src/safety/mod.rs` | `SafetyLayer` |
| `crates/roko-compose/src/prompt_assembly_service.rs` | The `PromptAssembler` impl |
| `crates/roko-compose/src/system_prompt_builder.rs` | 9-layer builder |
| `crates/roko-learn/src/feedback_service.rs` | The `FeedbackSink` impl |
| `crates/roko-learn/src/cascade_router.rs` | LinUCB router |
| `crates/roko-gate/src/gate_service.rs` | `GateService` impl |
| `crates/roko-gate/src/registry.rs` | `GateRegistry`, `GateSpec`, `GateKind` |
| `crates/roko-gate/src/rung_dispatch.rs` | Canonical 7-rung dispatch |
| `crates/roko-orchestrator/src/service_factory.rs` | The wiring point that builds all services |
| `crates/roko-cli/src/runner/event_loop.rs` | The legacy active plan runner (to be replaced) |
| `crates/roko-cli/src/orchestrate.rs` | The dead 22K-line monolith (feature-gated) |
| `crates/roko-acp/src/runner.rs` | ACP driver (still has inline review prompts) |
| `crates/roko-acp/src/session.rs` | ACP session (still has inline role prompts) |

---

## Definition Of Done For Any Plan

A plan is COMPLETE only when:

- [ ] All listed code changes landed in `main`
- [ ] Proof criteria pass (tests, scripts, manual checks listed in the plan)
- [ ] Anti-patterns enumerated in the plan return zero hits via `rg`
- [ ] Touched files compile cleanly (`cargo check --all-features`)
- [ ] No new TODO comments added without an associated issue or follow-up plan entry
- [ ] This INDEX is updated with the new status

---

## Reference: What Got Audited

Audited on **2026-05-01** via parallel `Task(explore)` subagents over the live `crates/` tree:

- **ModelCallService scope** → `roko-agent`, `roko-cli/src/dispatch*`, `roko-acp`, `roko-serve`
- **PromptAssembly + Feedback** → `roko-core/foundation.rs`, `roko-compose`, `roko-learn`, `roko-cli/runtime_feedback`
- **Pipeline + EffectDriver + TaskScheduler + Persistence** → `roko-runtime`, `roko-cli/runner`
- **Gates + Safety + Providers + Observability** → `roko-gate`, `roko-agent/safety`, `roko-cli/auth_detect`, `roko-runtime/projection`
- **Demo + Provider action plan** → `demo/demo-app`, `tmp/workflow/providers`

Re-audit before relying on these summaries if the codebase has moved significantly.
