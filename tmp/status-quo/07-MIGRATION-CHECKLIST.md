# 07 — V1→V2 Migration Checklist

**Phase-by-phase checklist for migrating to V2 conventions and eliminating tech debt.**

---

## Guiding Principle

Incremental V2 adoption (Option 3 from `02-SPEC-EVOLUTION.md`): keep the working V1 runtime, adopt V2 concepts where they add value, decompose the monolith along the way.

---

## Phase 0: Foundation (No Runtime Changes)

Structural cleanup that enables everything else.

- [ ] **0.1** Decompose `orchestrate.rs` (23K LOC) into modules
  - [ ] Extract `plan_loader.rs` — TOML parsing, DAG construction
  - [ ] Extract `task_scheduler.rs` — parallel scheduling, dependency tracking
  - [ ] Extract `prompt_enricher.rs` — RoleSystemPromptSpec, context bidding, neuro queries
  - [ ] Extract `gate_runner.rs` — gate pipeline execution, adaptive thresholds
  - [ ] Extract `state_manager.rs` — snapshot, resume, persistence
  - [ ] Extract `replan.rs` — gate failure replan logic
  - [ ] Extract `metrics.rs` — C-factor computation, efficiency recording
  - [ ] Keep `orchestrate.rs` as thin coordinator calling into modules
- [ ] **0.2** Remove `WorkflowEngine` from `roko-runtime` (dead code)
- [ ] **0.3** Fix `roko-runtime` layer violations
  - [ ] Remove deps on `roko-learn`, `roko-compose`, `roko-gate`
  - [ ] Move dependent code up to `roko-orchestrator` or `roko-cli`
- [ ] **0.4** Break circular dependencies
  - [ ] Identify shared traits causing cycles between agent/learn/compose/neuro
  - [ ] Extract to `roko-core` or new `roko-traits` crate
  - [ ] Use trait objects at boundaries
- [ ] **0.5** Feature-gate Phase 2 code
  - [ ] `roko-dreams/phase2/` → `#[cfg(feature = "phase2")]`
  - [ ] `roko-daimon/phase2_stubs.rs` → `#[cfg(feature = "phase2")]`
  - [ ] `roko-chain/phase2/` → `#[cfg(feature = "phase2")]`

**Acceptance**: `cargo test --workspace` passes, no behavior changes, cleaner architecture.

---

## Phase 1: V2 Naming Completion

Finish the naming migration to V2 conventions.

- [ ] **1.1** Swap type alias direction: `Signal` primary, `Engram` as deprecated alias
- [ ] **1.2** Swap type alias direction: `Store` primary, `Substrate` as deprecated alias
- [ ] **1.3** Update remaining doc-comment references (2 known: grimoire→neuro, clade→fleet)
- [ ] **1.4** Standardize test files to use V2 names
- [ ] **1.5** Add `#[deprecated]` attributes to V1 type aliases
- [ ] **1.6** Update CLAUDE.md to reference V2 names as canonical

**Acceptance**: All public API uses V2 names. V1 aliases compile with deprecation warnings.

---

## Phase 2: Safety & Contracts

Harden the safety layer.

- [ ] **2.1** Ship default restrictive `AgentContract` YAML files
  - [ ] Define contracts for: planner, coder, researcher, reviewer roles
  - [ ] Place in `.roko/contracts/` or embed as defaults
- [ ] **2.2** Make missing contract an error in production mode (keep permissive for dev)
- [ ] **2.3** Audit tool whitelist per agent role
- [ ] **2.4** Add integration test: agent with restrictive contract is blocked from unauthorized tools

**Acceptance**: All standard agent roles have contracts. Production mode rejects contractless agents.

---

## Phase 3: Wiring Unwired Components

Connect built-but-unused functionality.

- [ ] **3.1** Wire VCG auction as composition option
  - [ ] Add config flag: `compose.strategy = "vcg" | "greedy"`
  - [ ] Default to greedy, allow VCG opt-in
- [ ] **3.2** Wire dream consolidation auto-trigger
  - [ ] Trigger after N plan completions, or on `roko daemon` schedule
  - [ ] Add config: `dreams.auto_trigger_every = 10` (plans)
- [ ] **3.3** Wire cold substrate archival
  - [ ] Integrate with `roko knowledge gc` command
  - [ ] Archive signals older than configurable threshold
- [ ] **3.4** Wire event bus for internal communication
  - [ ] Replace direct function calls in orchestrate.rs with event emission
  - [ ] Subscribe learning subsystems to events
- [ ] **3.5** Wire agent pools for concurrent execution
  - [ ] Use pool for parallel task dispatch instead of spawning fresh agents
- [ ] **3.6** Wire knowledge-informed model routing
  - [ ] Query neuro store at CascadeRouter decision time
  - [ ] Factor past task performance into model selection

**Acceptance**: Each newly wired component has integration test + CLI verification.

---

## Phase 4: V2 Architecture Concepts (Incremental)

Adopt V2 concepts where they add clear value, without replacing the runtime.

- [ ] **4.1** Demurrage for knowledge store
  - [ ] Implement time-based decay in `roko-neuro`
  - [ ] Unused knowledge entries lose priority over time
  - [ ] GC can reclaim decayed entries
- [ ] **4.2** Predict-publish-correct for gate results
  - [ ] Before running gate, predict pass/fail based on history
  - [ ] Publish prediction, then correct based on actual result
  - [ ] Feed corrections into adaptive gate thresholds
- [ ] **4.3** EFE-informed routing
  - [ ] Compute Expected Free Energy for routing decisions
  - [ ] Integrate with CascadeRouter model selection
- [ ] **4.4** Cell model for agents
  - [ ] Wrap agents in `Cell` interface from `roko-graph`
  - [ ] Enable graph-based agent composition
  - [ ] Bridge: `tasks.toml` → Cell Graph → execution

**Acceptance**: Each concept has measurable improvement metric (better routing, lower waste, etc.)

---

## Phase 5: Graph Runtime Integration

The big V2 move: transition execution to the graph engine.

- [ ] **5.1** Bridge `tasks.toml` → `roko-graph` Cell Graph
  - [ ] Parser converts TOML tasks into Graph cells
  - [ ] DAG edges from task dependencies
- [ ] **5.2** Run Graph executor alongside PlanRunner (dual mode)
  - [ ] Config flag: `executor.engine = "plan_runner" | "graph"`
  - [ ] Graph executor calls same agent/gate/learn pipeline
- [ ] **5.3** Validate Graph executor produces same results as PlanRunner
  - [ ] Snapshot comparison tests
  - [ ] Run same plan through both engines
- [ ] **5.4** Implement Pulse/Bus kernel
  - [ ] Pulse scheduler drives graph execution
  - [ ] Bus routes events between cells
- [ ] **5.5** Migrate execution to Graph engine
  - [ ] Deprecate PlanRunner
  - [ ] Graph engine becomes default
- [ ] **5.6** Implement Graph-of-Graphs composition
  - [ ] Plans as sub-graphs within parent graphs
  - [ ] Recursive execution

**Acceptance**: `roko plan run` uses Graph engine with no regression.

---

## Phase 6: Polish & Cleanup

- [ ] **6.1** Audit and consolidate `roko-serve` routes
- [ ] **6.2** Add integration tests for undertested crates (agent-server, demo)
- [ ] **6.3** Archive stale `tmp/` documentation
- [ ] **6.4** Update `docs/v2/` to reflect actual implementation
- [ ] **6.5** Triage and resolve code TODOs/FIXMEs
- [ ] **6.6** Audit MCP crates (github, slack, scripts, stdio) — complete or remove
- [ ] **6.7** Update demo app to current API surface

---

## Progress Tracking

| Phase | Items | Done | % |
|-------|-------|------|---|
| Phase 0: Foundation | 5 | 0 | 0% |
| Phase 1: V2 Naming | 6 | 0 | 0% |
| Phase 2: Safety | 4 | 0 | 0% |
| Phase 3: Wiring | 6 | 0 | 0% |
| Phase 4: V2 Concepts | 4 | 0 | 0% |
| Phase 5: Graph Runtime | 6 | 0 | 0% |
| Phase 6: Polish | 7 | 0 | 0% |
| **Total** | **38** | **0** | **0%** |

## Recommended Execution Order

**Start with Phase 0** — it's pure cleanup with no behavior changes and unblocks everything else. Within Phase 0, item 0.1 (decompose orchestrate.rs) is the single highest-impact item.

Phases 1-3 can proceed in parallel after Phase 0.
Phase 4 requires Phase 0 + Phase 3.
Phase 5 requires Phase 4.
Phase 6 can proceed at any time.
