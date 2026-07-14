# 09 — Unified Roadmap (E01-E48)

> **What this doc is:** the single execution plan for all 48 epics across 6 phases, from
> immediate bootstrap to long-horizon economy. It synthesizes `03-WORK-BREAKDOWN-EPICS.md`
> (E01-E18 status-quo findings), `00-INDEX.md` (E19-E48 v2 spec implementation), and
> `docs/v2/28-ROADMAP.md` (v2 phase/dependency definitions). It names the critical path,
> parallel tracks, resource estimates, and success criteria for every milestone.
>
> - Repo HEAD: `5852c93c05` on `main` -- authored 2026-07-10
> - Inputs: `03-WORK-BREAKDOWN-EPICS.md`, `00-INDEX.md`, `docs/v2/28-ROADMAP.md`,
>   `05-MASTER-CHECKLIST.md`, `04-EXECUTION-READINESS.md`

---

## 1. Phase Overview Table

All 48 epics, sorted by phase/milestone, with task counts and dependency gates.

| # | Epic | Title | Phase | Tasks | Depends On | Primary Crates |
|---|---|---|---|---:|---|---|
| E01 | Execution Engine | M0 Bootstrap | 10 | -- (root) | roko-cli |
| E04 | Security Perimeter | M0 subset / M1 full | 19 | P16, P22 | roko-serve, roko-agent |
| E05 | Gate Adaptivity Live | M0 min / M1 full | 8 | E01 | roko-cli, roko-gate |
| E46 | GitHub Workflow Integration | M1 | 8 | E18 | roko-cli, roko-serve |
| E47 | Resource & Disk Management | M1 | 8 | E09 | roko-fs, roko-serve |
| E48 | Rate-Limit Budgeting | M1 | 8 | E04, E14 | roko-serve, roko-agent |
| E03 | Type Consolidation | M1 | 7 | -- | roko-core |
| E02 | Storage Convergence | M1 | 12 | E03 (soft) | roko-fs, roko-serve |
| E06 | Compose / Prompt Unify | M1 | 9 | E01 | roko-compose |
| E14 | Providers & Tools | M1 | 7 | E01 | roko-std, roko-agent |
| E15 | MCP Config & Passthrough | M1 | 6 | -- | roko-mcp-code |
| E16 | PRD Self-Hosting | M1 | 2 | E01, E14 | roko-cli |
| E07 | Learning & Knowledge | M2 | 10 | E01 | roko-learn, roko-neuro |
| E08 | Conductor Supervision | M2 | 7 | E01 | roko-conductor |
| E09 | Observability | M2 | 9 | E01 | roko-conductor, roko-serve |
| E10 | Frontend / API Contract | M2 | 7 | E03 | demo/demo-app, roko-serve |
| E17 | ACP Completion | M2 | 6 | E04, E07, E15 | roko-acp |
| E18 | Docs, Config, CI & Ops | M2 | 13 | E01 | repo-wide |
| E33 | Telemetry & Lens | M2 | 9 | E09 | roko-conductor |
| E34 | Security IFC | M2 | 8 | E04 | roko-gate, roko-agent |
| E35 | Auth Protocol | M2 | 8 | E04 | roko-serve |
| E42 | Config Evolution | M2 | 8 | E19 | roko-core, roko-cli |
| E44 | Cross-Cut Functors | M2 | 8 | E19, E20 | roko-compose, roko-daimon |
| E45 | Orchestrator Mori Parity | M2 | 10 | E01, E12 | roko-cli |
| E11 | Chain / ISFR | M3+ | 5 | arch-core-queue | roko-chain |
| E12 | Dead-Code Cleanup | M3+ | 9 | E05, E06, E08 | repo-wide |
| E13 | v2 Spec-Debt (Lens) | M3+ | 3 | E09 | roko-core |
| E19 | Signal Protocol | Phase 1 | 10 | E01 | roko-core, roko-primitives |
| E20 | Cell Unification | Phase 1 | 10 | E01 | roko-core |
| E21 | Graph Engine | Phase 1 | 10 | E20 | roko-orchestrator |
| E22 | Execution Runtime | Phase 1 | 10 | E20, E21 | roko-orchestrator, roko-cli |
| E23 | Agent Cognitive Autonomy | Phase 2 | 10 | E19, E20 | roko-agent |
| E24 | Memory Advanced | Phase 2 | 10 | E07 | roko-neuro |
| E25 | Learning Loops Advanced | Phase 2 | 10 | E07 | roko-learn |
| E26 | Inference Gateway | Phase 2 | 12 | E14 | roko-agent |
| E27 | Feeds System | Phase 2 | 8 | E19, E20 | roko-core, roko-runtime |
| E28 | Groups & Coordination | Phase 2 | 8 | E20 | roko-runtime |
| E29 | Connectivity & Relay | Phase 2 | 9 | E04 | roko-agent, roko-runtime |
| E30 | Extension System | Phase 2 | 8 | E20 | roko-agent |
| E31 | Trigger System | Phase 2 | 8 | E08 | roko-runtime |
| E32 | Tool & Plugin Ecosystem | Phase 2 | 8 | E14, E15 | roko-std |
| E37 | Surfaces | Phase 2+ | 9 | E09, E33 | roko-cli, roko-serve |
| E43 | Deployment & Portability | Phase 2+ | 8 | E18 | roko-cli |
| E36 | Payments | Phase 3 | 8 | E11, E29 | roko-chain |
| E38 | Marketplace | Phase 3 | 9 | E36, E39 | roko-serve, roko-chain |
| E39 | Registries & Identity | Phase 3 | 8 | E11 | roko-chain |
| E40 | Arenas & Evals | Phase 3 | 8 | E25, E39 | roko-learn, roko-chain |
| E41 | DeFi Products | Phase 3 | 8 | E11, E39 | roko-chain |
| | | **TOTAL** | **460** | | |

### Task count by phase

| Phase | Epics | Tasks | Cumulative |
|---|---:|---:|---:|
| M0 Bootstrap | 3 (core) | 37 | 37 |
| M1 Correctness | 9 | 78 | 115 |
| M2 Completeness | 12 | 103 | 218 |
| M3+ Long-horizon | 3 | 17 | 235 |
| Phase 1 Kernel | 4 | 40 | 275 |
| Phase 2 Agent/Infra | 12 | 111 | 386 |
| Phase 3 Economy | 5 | 41 | 427 |
| Phase 2+ Meta | 4 | 33 | 460 |

> Note: E01-E18 carry 149 tasks from status-quo audit findings. E19-E45 carry 240 tasks
> from v2 spec implementation. E46-E48 add 24 tasks for operational hygiene (GitHub, resources,
> rate limits). Plus ~120 tasks in pre-existing `plans/` queue (P08-P34) -- not double-counted.

---

## 2. Phase 0 -- Current State (What Works Today)

The plan-execute-gate-persist loop is **fully wired end-to-end**. The following subsystems
are live and operational at HEAD:

| Subsystem | Status | Key File |
|---|---|---|
| Plan discovery + DAG executor | Wired | `crates/roko-cli/src/orchestrate.rs` |
| Agent dispatch (Claude CLI + 8 backends) | Wired | `crates/roko-agent/src/dispatcher/mod.rs` |
| Safety layer (role auth, pre/post checks) | Wired | `crates/roko-agent/src/safety/` |
| Gate pipeline (compile, test, clippy, diff) | Wired | Called per-task from orchestrate.rs |
| Session persistence + resume | Wired | `.roko/state/executor.json`, `--resume` |
| PRD lifecycle (idea/draft/plan) | Wired | `roko prd` subcommands |
| Research agent | Wired | `roko research` subcommands |
| SystemPromptBuilder (9-layer prompts) | Wired | `RoleSystemPromptSpec` in orchestrate.rs |
| EpisodeLogger (turn recording) | Wired | `.roko/episodes.jsonl` |
| ProcessSupervisor (lifecycle) | Wired | `PlanRunner` tracks agents |
| MCP config passthrough | Wired | `agent.mcp_config` in roko.toml |
| CascadeRouter (model routing) | Wired | `.roko/learn/cascade-router.json` |
| Prompt experiments (A/B) | Wired | `.roko/learn/experiments.json` |
| Adaptive gate thresholds | Wired | EMA per rung |
| Interactive TUI (ratatui) | Wired | F1-F7 tabs, `roko dashboard` |
| HTTP control plane (~85 routes) | Wired | `roko serve` on :6677 |
| Per-agent sidecar (13 routes) | Wired | `crates/roko-agent-server/` |
| Code-intelligence MCP | Wired | `crates/roko-mcp-code/` |
| HDC fingerprint per-episode | Wired | Computed + stored |
| Gate failure replan | Wired | `build_gate_failure_plan_revision` |
| PRD auto-plan trigger | Wired | `prd_publish_subscriber` |

### Phase 0 critical gap

The default `roko plan run` engine is the **Graph dry-run stub**: it prints `SUCCESS` in ~2s,
spawns 0 agents, spends $0, changes no files. Until E01 flips the engine default, all
subsequent work must use `--engine runner-v2` explicitly. **E01 is the gate on everything.**

---

## 3. M0 -- Bootstrap (Self-Execution Becomes Possible)

**Goal:** Make bare `roko plan run plans/<x>` reliably execute a real plan with honest
pass/fail reporting.

**Scope:** E01 (core engine flip) + E04 subset (unattended-safe) + E05 minimum (no stub-pass).

### Epics

| Epic | Tasks | What it does |
|---|---:|---|
| **E01** Execution Engine | 10 | Flip default engine, wire resume, real DAG scheduler, worktree isolation, regression lock |
| **E04** Security Perimeter (subset) | ~6 | P16 deny-list + E04-T05/T06/T07 (block leaks, safety funnel on Claude-CLI, custody hash-chain) |
| **E05** Gate Adaptivity (minimum) | ~2 | E05-T02 (stubs -> Skipped not pass) + E05-T03 (skipped excluded from EMA) |

### Execution order

```
1. E01-T01  Flip default engine               ──┐
2. E01-T02  Route `roko resume` to RunnerV2    ──┤
3. E01-T09  Regression test (bare default)     ──┘ serial, ~1 day
4. P16 + E04-T05/T06/T07  Safety enforcement  ──── parallel track, ~2 days
5. E05-T02 + E05-T03  Honest gate floor        ──── parallel track, ~1 day
6. Smoke test: 04-EXECUTION-READINESS §5       ──── M0 exit gate
```

### Exit criteria

- `roko plan run plans/<x>` (bare, no `--engine`) **spawns real agents and reports honest
  pass/fail**.
- After a run: `git status --porcelain` is non-empty, `.roko/episodes.jsonl` grew,
  `.roko/state/state-snapshot.json` was written.
- A complex-tier task with a failing verify check reports **fail/skip, not a stub-pass**.
- The `04-EXECUTION-READINESS §5` one-command smoke passes.

---

## 4. M1 -- Correctness & Convergence

**Goal:** Close the type/storage/compose/provider/MCP seams so the loop is correct end-to-end.
Add operational infrastructure (GitHub, resources, rate limits).

**Scope:** E02, E03, E05 full, E06, E14, E15, E16, E46, E47, E48.

### Epics

| Epic | Tasks | What it does |
|---|---:|---|
| **E03** Type Consolidation | 7 | Canonicalize 5 dup type families (`GateVerdict`, `DashboardSnapshot`, `RetentionPolicy`, etc.) |
| **E02** Storage Convergence | 12 | One canonical writer per `.roko/` store; fix empty dashboards |
| **E05** Gate Adaptivity (full) | 8 | Real rung inputs, per-rung EMA persisted, skipped != pass |
| **E06** Compose / Prompt Unify | 9 | Route Runner v2 through 12-slot builder; kill 4 parallel assemblers |
| **E14** Providers & Tools | 7 | Retries retry, tools survive, 37 advertised == executable |
| **E15** MCP Config & Passthrough | 6 | `{"mcpServers":{}}` normalizer + env + parity |
| **E16** PRD Self-Hosting | 2 | Close idea->draft->research->plan generative front-half |
| **E46** GitHub Workflow Integration | 8 | PR creation, issue tracking, CI status, branch management from within roko |
| **E47** Resource & Disk Management | 8 | Disk usage monitoring, log rotation, artifact cleanup, quota enforcement |
| **E48** Rate-Limit Budgeting | 8 | Per-provider rate limits, cost tracking, budget enforcement, backoff |

### Execution order (parallel tracks after M0)

```
Track A: E03 ──► E02 ──► E05 full ──► E06      Types -> Storage -> Gates -> Compose
Track B: E14 ──► E15 ──► E16                    Providers -> MCP -> PRD
Track C: E46, E47, E48                          Ops infra (parallel, independent)
```

### Exit criteria

- Writer-path == reader-path for verdicts/executor-state/thresholds/episodes (E02).
- Exactly one bare `struct GateVerdict` / `DashboardSnapshot` (E03).
- Default `plan run` exercises the 12-slot builder (E06-T03).
- Advertised builtins == executable handlers; one 429 no longer aborts a turn (E14).
- A `.mcp.json` server actually reaches the agent as `mcpServers` (E15-T1).
- `idea -> draft -> plan` produces a parseable `tasks.toml` with real `prd status` columns (E16).
- GitHub PRs can be created/tracked from the roko loop (E46).
- Disk quotas enforced, log rotation active (E47).
- Rate limits enforced per-provider, budget tracking live (E48).

---

## 5. M2 -- Completeness

**Goal:** Make the system observable, shippable, and ready for external users. Close the ACP
loop. Wire advanced telemetry.

**Scope:** E07-E10, E17-E18, E33-E35, E42, E44-E45.

### Epics

| Epic | Tasks | What it does |
|---|---:|---|
| **E07** Learning & Knowledge | 10 | LinUCB persists across restart, knowledge income, HDC on |
| **E08** Conductor Supervision | 7 | Wire anomaly supervision into live event loop (ghost-turn, compile-loop, cost-blowout) |
| **E09** Observability | 9 | Thread `MetricRegistry` into `RunConfig`, rotate logs, trim events firehose |
| **E10** Frontend / API Contract | 7 | Fix 4 frontend 404s, casing drift, double-SSE, replay-drop |
| **E17** ACP Completion | 6 | Make ACP turn consent-gated, learning-informed, MCP-equipped, honest |
| **E18** Docs, Config, CI & Ops | 13 | Fix CI gates, secrets, MSRV, then rewrite lying docs |
| **E33** Telemetry & Lens | 9 | 7 StateHub projections, Lens stacking, Observe protocol, c-factor |
| **E34** Security IFC | 8 | Taint lattice, immune system pipeline, 5-head corrigibility, sandbox |
| **E35** Auth Protocol | 8 | API key rotation, agent tokens, JWKS, team RBAC, audit trail |
| **E42** Config Evolution | 8 | Config-as-Signal, schema versioning, 7 invariants, hot-reload |
| **E44** Cross-Cut Functors | 8 | Endofunctor algebra (Memory/Daimon/Dreams), VCG arbitration |
| **E45** Orchestrator Mori Parity | 10 | Structured review, auto-fix, error sharing, reflection loop, warm spawn |

### Execution order

```
Wave 1 (after M1):
  E07 + E08 + E09     Learning + Conductor + Observability (parallel)
  E10                  Frontend (needs E03 from M1)
  E18 T01-T09          CI/config/ops fixes

Wave 2 (after Wave 1):
  E17                  ACP (needs E04 + E07 + E15)
  E18 T10-T13          Doc rewrites (need E01 + E18 fixes)
  E33                  Telemetry (needs E09)
  E34 + E35            Security IFC + Auth (need E04)

Wave 3 (after Wave 2):
  E42                  Config Evolution (needs E19 soft-dep, can start early)
  E44                  Cross-Cut Functors (needs E19 + E20 soft-dep)
  E45                  Mori Parity (needs E01 + E12)
```

### Exit criteria

- LinUCB survives restart; knowledge `balance > 0` (E07).
- Conductor aborts a ghost-turn loop before wall-clock (E08).
- `.roko/metrics/prometheus.txt` carries `roko_gate_verdicts_total`; logs rotate (E09).
- The 4 frontend 404s resolve; one SSE manager remains (E10).
- ACP turn is consent-gated + learning-informed + MCP-equipped (E17).
- Release tag runs clippy+test+`cargo deny`; docs pass grep-guard (E18).
- 7 StateHub projections consumed by TUI + HTTP + SSE (E33).
- Taint lattice propagates through extension hooks (E34).
- API key rotation works end-to-end (E35).

---

## 6. M3+ -- Long-Horizon Cleanup

**Goal:** Retire legacy code, recover the chain queue, resolve spec-debt.

**Scope:** E11, E12, E13.

### Epics

| Epic | Tasks | What it does |
|---|---:|---|
| **E11** Chain / ISFR | 5 | Recover `architecture-core-queue`, implement `get_logs`, 13-contract deploy parity |
| **E12** Dead-Code Cleanup | 9 | Delete ~52K-LOC legacy island (`orchestrate.rs` + `roko-orchestrator` + orphans) |
| **E13** v2 Spec-Debt (Lens) | 3 | Build `trait Lens` + `MetricRegistry` adapter; resolve Cell/Block naming |

### Gating rules

- **E12-T07** (delete orchestrate.rs) requires **E05 + E06 + E08** all landed (live value ported out).
- **E12-T06** (drop roko-orchestrator) requires **E01 + E04**.
- **E12-T05** (HDC de-dup) requires **E03**.
- **E13-T01** requires **E09-T09**.
- **E11-T01** recovers `architecture-core-queue` -- prerequisite for all Phase 3 chain work.
- **E11-T04** and **E03-T07** both delete the same `Engram` stub -- schedule in one wave only.

### Exit criteria

- `architecture-core-queue` recovered; `get_logs` real (E11).
- Legacy island deleted; workspace still green (E12).
- `rg 'trait Lens'` is non-zero; Cell/Block decision doc exists (E13).

---

## 7. Phase 1 -- Kernel Upgrade (E19-E22)

**Goal:** Promote Pulse/Bus to kernel-level, introduce Cell supertrait, build the typed Graph
engine, wire the cognitive execution runtime. This is the foundational refactoring that all
Phase 2 work builds on.

**Entry gate:** M1 exit green (E01 landed, core types stabilized).

### Epics

| Epic | Tasks | What it does |
|---|---:|---|
| **E19** Signal Protocol | 10 | Pulse/Bus kernel, Signal graduation, demurrage economics, HDC fingerprints, IFC taint, Kind registry |
| **E20** Cell Unification | 10 | Cell supertrait with 9 protocols, TypeSchema, predict-publish-correct, CellContext, CellRegistry |
| **E21** Graph Engine | 10 | Typed edge validation, Hot Graphs, Workflow/Activity split, parallel waves, snapshot/resume, merge queue |
| **E22** Execution Runtime | 10 | 7 cognitive loop Cells, nested gamma/theta/delta loops, T0 short-circuit, error taxonomy, budget, replay |

### Internal dependencies

```
E19 (Signal/Bus) ──────┐
                        ├──► E21 (Graph Engine) ──► E22 (Execution Runtime)
E20 (Cell Unification) ─┘
```

E19 and E20 are **parallel** -- they touch different subsystems (signal/bus vs cell/registry).
E21 needs both. E22 needs E21.

### Exit criteria

- `Pulse` struct exists as first-class type alongside `Engram`.
- `Bus` trait exists alongside `Substrate` with `BroadcastBus` implementation.
- Cell supertrait unifies all 9 protocol implementations.
- `TypeSchema` validation at Graph-load time.
- A Graph can be authored in TOML and executed via `roko plan run`.
- Hot Graph stays resident and re-fires per tick.
- 7 cognitive loop Cells wired into nested gamma/theta/delta loops.

---

## 8. Phase 2 -- Agent Cognition & Infrastructure (E23-E32, E37, E43)

**Goal:** Build the full agent cognitive stack, infrastructure, and surfaces on top of the
Phase 1 kernel.

**Entry gate:** Phase 1 exit green (Cell + Graph + Bus landed).

### Phase 2 -- Agent Cognition (E23-E26)

| Epic | Tasks | What it does | Depends On |
|---|---:|---|---|
| **E23** Agent Cognitive Autonomy | 10 | Type-state machine, behavioral phases, CorticalState, EFE routing, emergent goals | E19, E20 |
| **E24** Memory Advanced | 10 | Heuristics w/ falsifiers, Allen intervals, resonator networks, income, dream triggers | E07 |
| **E25** Learning Loops Advanced | 10 | L3 HDC defragmentation, L4 c-factor governance, experiment lifecycle, playbooks | E07 |
| **E26** Inference Gateway | 12 | 9-stage pipeline (loop detect -> cache -> prune -> budget -> think -> converge -> call -> store -> track) | E14 |

### Phase 2 -- Infrastructure (E27-E32)

| Epic | Tasks | What it does | Depends On |
|---|---:|---|---|
| **E27** Feeds System | 8 | Feed trait, registry, raw/derived/composite taxonomy, recipes, marketplace | E19, E20 |
| **E28** Groups & Coordination | 8 | Group as Space, 4 coordination modes, membership, pheromone fields | E20 |
| **E29** Connectivity & Relay | 9 | Connect protocol, relay wire protocol, A2A cards, reconnection FSM, backpressure | E04 |
| **E30** Extension System | 8 | Extension trait, 22 hooks, CaMeL IFC, discovery/resolution, circuit breaking | E20 |
| **E31** Trigger System | 8 | Trigger as Cell, event source registry, bindings, debounce/filter, Bus topics | E08 |
| **E32** Tool & Plugin Ecosystem | 8 | Plugin SDK (5-tier SPI), dynamic loading, capability binding, sandboxing | E14, E15 |

### Phase 2+ -- Operations & Meta (E33-E35, E37, E42-E45)

| Epic | Tasks | What it does | Depends On |
|---|---:|---|---|
| **E33** Telemetry & Lens | 9 | 7 StateHub projections, Lens stacking, Observe protocol, c-factor | E09 |
| **E34** Security IFC | 8 | Taint lattice, immune system, 5-head corrigibility, sandbox, quarantine | E04 |
| **E35** Auth Protocol | 8 | API key rotation, agent tokens, JWKS, team RBAC, relay tokens | E04 |
| **E37** Surfaces | 9 | 5 named surfaces (Workbench, Inbox, Canvas, Minimap, Autonomy Slider) | E09, E33 |
| **E43** Deployment & Portability | 8 | Brain export/import (Merkle-CRDT), daemon lifecycle, secrets rotation | E18 |

### Parallel tracks within Phase 2

```
Track A: E23 + E24 + E25           Agent cognition (parallel after E19/E20/E07)
Track B: E26                        Inference gateway (parallel, needs E14 only)
Track C: E27 + E28 + E29 + E30     Infrastructure (parallel after E19/E20/E04)
Track D: E31 + E32                  Triggers + Plugins (parallel after E08/E14/E15)
Track E: E37 + E43                  Surfaces + Deploy (parallel after E09/E18/E33)
```

### Exit criteria

- Type-state Agent enforces lifecycle at compile time (E23).
- Heuristic Signals have when/then/falsifier/calibration (E24).
- L3/L4 learning loops operational (E25).
- 9-stage inference pipeline live (E26).
- Feeds, Groups, Relay, Extensions, Triggers, Plugins all trait-based and wired (E27-E32).
- 5 named surfaces implemented in TUI (E37).
- Brain export produces ~100KB-1MB portable file (E43).

---

## 9. Phase 3 -- Economy (E36-E41)

**Goal:** On-chain identity, payments, marketplace, arenas, and DeFi products. This phase
requires chain infrastructure (E11) and the full agent/infra stack from Phase 2.

**Entry gate:** Phase 2 exit green + E11 chain recovery.

### Epics

| Epic | Tasks | What it does | Depends On |
|---|---:|---|---|
| **E36** Payments | 8 | x402 per-request, MPP session-based, reputation pricing, settlement batching | E11, E29 |
| **E38** Marketplace | 9 | Agent passport, TraceRank reputation, publish/discover/fork, Package SPI | E36, E39 |
| **E39** Registries & Identity | 8 | ERC-8004 transferable identity, ZK-HDC, on-chain InsightStore, gossip | E11 |
| **E40** Arenas & Evals | 8 | 7-step flywheel, scoring functions, leaderboards, bounty escrow | E25, E39 |
| **E41** DeFi Products | 8 | VCG clearing Cell, yield perpetuals, VenueAdapter, DeFiRiskEngine | E11, E39 |

### Internal dependencies

```
E11 (Chain) ──► E39 (Registries) ──► E38 (Marketplace)
                     │                      ▲
                     └──► E40 (Arenas)       │
                     └──► E41 (DeFi)         │
E29 (Relay) ──► E36 (Payments) ─────────────┘
E25 (Learning) ──► E40 (Arenas)
```

### Exit criteria

- Agent registers ERC-8004 identity, publishes knowledge, receives reputation attestation.
- x402 per-request payment works end-to-end (E36).
- Cells publishable to and installable from marketplace (E38).
- Arena 7-step flywheel runs end-to-end (E40).
- VCG clearing Cell produces allocation (E41).
- Variance Inequality enforced: L4 pauses when generator outpaces verifier (E40).

---

## 10. Critical Path Analysis

### Longest dependency chain (M0 through Phase 3)

The deepest chain traverses 7 epic-depths:

```
E01 (M0)
 └─► E14 (M1)
      └─► E15 (M1)
           └─► E32 (Phase 2: Plugin Ecosystem)
                └─► dependent on E14 + E15 landing
```

But the **binding critical path** that determines the latest milestone is:

```
E01  ──►  E05  ──►  E12-T07 (delete orchestrate.rs)
     ├──►  E06  ──►  E12-T07
     └──►  E08  ──►  E12-T07  ──►  E12-T08 (delete legacy feature)
```

This means M3+ cleanup cannot begin until three independent M1/M2 epics (E05, E06, E08)
all complete, which is the deepest chain at **4 epic-depths**.

For Phase 2-3, the binding chain is:

```
E01 ──► E19/E20 ──► E21 ──► E22 ──► (Phase 2 execution runtime)
E01 ──► E04 ──► E29 ──► E36 ──► E38  (Phase 3 economy)
E01 ──► E07 ──► E25 ──► E40          (Phase 3 arenas)
```

The **longest path** from root to leaf is:

```
E01 ──► E04 ──► E29 ──► E36 ──► E38 (Marketplace)
                                 ▲
                          E39 ───┘
                           ▲
                     E11 ──┘
```

This is **6 epic-depths** (E01 -> E04 -> E29 -> E36 -> E38, with E11 -> E39 -> E38 as
a parallel chain of equal length).

### Dependency DAG (ASCII)

```
                              ┌─────────────────┐
                              │  E01 ENGINE (M0)  │  root -- gates everything
                              └────────┬──────────┘
          ┌──────────┬─────────┬───────┼────────┬───────────┬────────────┬────────────┐
          ▼          ▼         ▼       ▼        ▼           ▼            ▼            ▼
       ┌──────┐  ┌──────┐  ┌──────┐ ┌──────┐ ┌──────┐  ┌───────┐  ┌───────┐  ┌───────┐
       │ E05  │  │ E06  │  │ E14  │ │ E15  │ │ E09  │  │ E07   │  │ E08   │  │ E16   │
       │gates │  │comp. │  │prov. │ │ MCP  │ │ obs  │  │learn  │  │cond.  │  │ PRD   │
       └──┬───┘  └──┬───┘  └──┬───┘ └──┬───┘ └──┬───┘  └──┬────┘  └──┬────┘  └───────┘
          │         │         │        │        │         │           │
          │         │         ▼        │        │         │           │
          │         │      E48(rate)   │        ▼         │           │
          │         │         │        │     E33(lens)    │           │
          │         │         │        │        │         │           │
    E03 ──┼─────────┼─────────┼────────┼────────┼─────────┼───────────┤
     │    │         │         │        │        │         │           │
     ▼    │         │         │        │        ▼         │           │
    E02   │         │         │        │     E37(surf)    │           │
     │    │         │         │        │                  │           │
    E10   │         │         │        │                  │           │
          │         │         │        │                  │           │
    E04 ──┼─────────┼─────────┼────────┼──────────────────┤           │
     │    │         │         │        │                  │           │
     │    │         │         │        ▼                  │           │
     │    │         │         │     E17(ACP)              │           │
     │    │         │         │    needs E04+E07+E15      │           │
     │    │         │         │                           │           │
     │    │         │         │                           │           │
     ├────┤─────────┤─────────┤                           │           │
     │    │         │         │                           │           │
     │    └────┬────┘         │                           │           │
     │    E12 ─┤──────────────┘                           │           │
     │ (needs E05+E06+E08)    │                           │           │
     │                        │                           │           │
     ▼                        │                           │           │
   E29(relay)                 │                           │           │
     │                        │                           │           │
     ▼                        │                           │           │
   E36(payments)              │                           │           │
     │                        │                           │           │
     ▼                        │                           │           │
   E38(marketplace) ◄───── E39(registries) ◄──── E11(chain)          │
     │                        │                                       │
     │                        ▼                                       │
     │                     E40(arenas) ◄──── E25(learning adv.) ◄── E07
     │                        │
     │                        ▼
     │                     E41(defi)
     │
   ┌─┴─────────────────────────────────────────────────────────┐
   │                    KERNEL UPGRADE                          │
   │  E19(Signal/Bus) ──┐                                      │
   │                     ├──► E21(Graph) ──► E22(Exec Runtime)  │
   │  E20(Cell Unify) ──┘                                      │
   │       │                                                    │
   │       ├──► E23(Agent Cognition)                            │
   │       ├──► E27(Feeds)  E28(Groups)  E30(Extensions)       │
   │       │                                                    │
   │  E08 ──► E31(Triggers)                                     │
   │  E14+E15 ──► E32(Plugins)  E26(Inference Gateway)         │
   └────────────────────────────────────────────────────────────┘
```

---

## 11. Parallel Tracks

These track groupings are **file-disjoint** and can safely run concurrently in separate
worktrees.

### M0-M1 parallel tracks (after E01)

| Track | Epics | Primary files | Notes |
|---|---|---|---|
| **A -- Security** | E04 | `roko-serve` middleware, `roko-agent/safety`, `roko-acp` | M0 subset first, rest M1/M2 |
| **B -- Providers/MCP** | E14, E15, E16 | `roko-std/tool/*`, `roko-agent/provider`, `roko-mcp-code` | Hot dispatch-path |
| **C -- Types/Storage** | E03, E02, E05, E06 | `roko-core`, `roko-fs`, `roko-serve` readers | E03 must lead |
| **D -- Ops Infra** | E46, E47, E48 | `.github/`, `roko-fs/gc`, `roko-serve/rate_limit` | Independent |

### M2 parallel tracks

| Track | Epics | Notes |
|---|---|---|
| **E -- Learning** | E07, E08 | Conductor + learning loops |
| **F -- Observability** | E09, E10, E33 | Metrics, frontend, telemetry |
| **G -- Security adv.** | E34, E35 | IFC + auth (needs E04) |
| **H -- Docs/Ops** | E18, E42, E43 | CI, config, deploy |

### Phase 1-2 parallel tracks

| Track | Epics | Notes |
|---|---|---|
| **I -- Kernel** | E19, E20, E21, E22 | Serial internally (E19/E20 parallel, then E21, then E22) |
| **J -- Cognition** | E23, E24, E25 | After E19/E20 + E07 |
| **K -- Infra** | E26, E27, E28, E29, E30, E31, E32 | Most are independent after E20 |
| **L -- Surfaces** | E37 | After E33 |

### Phase 3 parallel tracks

| Track | Epics | Notes |
|---|---|---|
| **M -- Identity** | E39, E40, E41 | After E11 |
| **N -- Payments** | E36, E38 | After E29 + E39 |

### File-exclusivity caution

E02, E03, and E04 all touch `roko-serve` routes/readers. Keep E02 reader repoints and E04
middleware edits in **dependency order** (E03 signature changes precede E02 consumers), never
as siblings in one parallel group.

---

## 12. Resource Requirements

### Estimated LOC per phase

| Phase | New LOC | Modified LOC | Net delta | Notes |
|---|---:|---:|---:|---|
| M0 Bootstrap | ~500 | ~300 | +200 | Mostly flipping defaults + tests |
| M1 Correctness | ~3,000 | ~4,000 | -1,000 | De-dup reduces LOC net |
| M2 Completeness | ~6,000 | ~5,000 | +1,000 | New surfaces, learning, docs |
| M3+ Cleanup | ~200 | ~500 | **-52,000** | Legacy island deletion |
| Phase 1 Kernel | ~12,000 | ~5,000 | +7,000 | Bus, Cell, Graph, Runtime |
| Phase 2 Agent/Infra | ~25,000 | ~8,000 | +17,000 | 12 epics, all new subsystems |
| Phase 3 Economy | ~15,000 | ~3,000 | +12,000 | Chain integration, marketplace |
| **Total** | **~62,000** | **~26,000** | **-16,000** | Net shrink from M3+ cleanup |

### Agent-hours per phase (estimated)

Assuming roko self-hosting with `roko plan run` executing tasks via Claude agents:

| Phase | Tasks | Est. agent-hours | Est. model cost (USD) | Calendar time |
|---|---:|---:|---:|---|
| M0 Bootstrap | 18 | 4-8 | $20-50 | 1-2 days |
| M1 Correctness | 78 | 30-60 | $150-400 | 1-2 weeks |
| M2 Completeness | 103 | 50-100 | $250-600 | 2-4 weeks |
| M3+ Cleanup | 17 | 8-15 | $40-100 | 3-5 days |
| Phase 1 Kernel | 40 | 40-80 | $200-500 | 2-3 weeks |
| Phase 2 Agent/Infra | 111 | 80-160 | $400-1,000 | 4-8 weeks |
| Phase 3 Economy | 41 | 40-80 | $200-500 | 2-4 weeks |
| **Total** | **460** | **250-500** | **$1,300-3,200** | **~3-5 months** |

> These estimates assume Claude Opus-class models for architectural tasks and Sonnet-class
> for mechanical/focused tasks. Actual costs depend on model routing (CascadeRouter) and
> gate pass rates. The `efficiency.jsonl` log will provide real numbers after M0.

---

## 13. Success Criteria Per Phase

### M0 -- Bootstrap

- [ ] `roko plan run plans/<x>` (bare default) spawns real agents
- [ ] Honest pass/fail: failing verify -> fail, not stub-pass
- [ ] State snapshot written; episodes logged; files changed
- [ ] Safety funnel invoked on default Claude-CLI path
- [ ] Custody hash-chain computes real hashes

### M1 -- Correctness

- [ ] One bare `struct GateVerdict` in entire codebase
- [ ] Writer-path == reader-path for all `.roko/` stores
- [ ] 12-slot prompt builder used by default `plan run`
- [ ] All 37 advertised builtins are executable
- [ ] MCP config reaches agent correctly
- [ ] `idea -> draft -> plan` produces valid `tasks.toml`
- [ ] GitHub integration operational
- [ ] Rate limits + disk quotas enforced

### M2 -- Completeness

- [ ] LinUCB survives restart; knowledge `balance > 0`
- [ ] Conductor detects and aborts ghost-turn loops
- [ ] Prometheus metrics exported; logs rotate
- [ ] Frontend 404s resolved; single SSE manager
- [ ] ACP turn: consent-gated + learning-informed + MCP-equipped
- [ ] CI pipeline: clippy + test + `cargo deny` on release tag
- [ ] 7 StateHub projections consumed by all surfaces
- [ ] API key rotation works end-to-end

### M3+ -- Cleanup

- [ ] `architecture-core-queue` recovered into `plans/`
- [ ] Legacy island deleted (~52K LOC); workspace green
- [ ] `trait Lens` exists; Cell/Block naming decision documented

### Phase 1 -- Kernel

- [ ] `Pulse` and `Bus` are kernel-level types/traits
- [ ] Cell supertrait unifies 9 protocols
- [ ] TypeSchema validates Graph edges at load time
- [ ] Graph TOML authored and executed via `roko plan run`
- [ ] Hot Graph stays resident and re-fires per tick
- [ ] 7 cognitive loop Cells in nested gamma/theta/delta loops
- [ ] Predict-publish-correct structural via Bus pub/sub
- [ ] Demurrage: knowledge Signals decay; retrieval restores balance

### Phase 2 -- Agent/Infra

- [ ] Type-state Agent enforces lifecycle at compile time
- [ ] EFE routing replaces LinUCB in CascadeRouter
- [ ] 9-stage inference pipeline live
- [ ] L3 HDC defragmentation + L4 c-factor governance operational
- [ ] Feeds, Groups, Relay, Extensions, Triggers, Plugins all wired
- [ ] 5-tier SPI: all tiers load and run
- [ ] 5 named surfaces implemented in TUI
- [ ] Brain export/import produces portable ~100KB-1MB file

### Phase 3 -- Economy

- [ ] Agent registers ERC-8004 identity on-chain
- [ ] x402 per-request payment works end-to-end
- [ ] Cells publishable/installable from marketplace
- [ ] Arena 7-step flywheel runs end-to-end
- [ ] VCG clearing Cell produces allocation
- [ ] Variance Inequality: L4 pauses when generator outpaces verifier
- [ ] Two Agents discover each other's knowledge through relay

---

## 14. Quick Reference: Recommended Execution Order

For a human or agent starting from scratch:

```
 1. E01-T01/T02/T09    Flip engine + regression test               M0 (day 1)
 2. E04 subset + E05 min  Safety + honest gates                    M0 (day 2)
 3. E03 -> E02          Types then storage                         M1 (week 1)
 4. E14 -> E15 -> E16   Providers -> MCP -> PRD                   M1 (week 1, parallel)
 5. E05 full -> E06     Gates -> Compose                           M1 (week 2)
 6. E46, E47, E48       Ops infra                                  M1 (week 2, parallel)
 7. E01 remainder       DAG, worktree, gate enrichment             M1 (week 2)
 8. E07 + E08 + E09     Learning + Conductor + Observability       M2 (week 3)
 9. E10 + E18           Frontend + Docs/CI                         M2 (week 3, parallel)
10. E17                 ACP completion                             M2 (week 4)
11. E33-E35             Telemetry + Security + Auth                M2 (week 4, parallel)
12. E42, E44, E45       Config + Functors + Mori parity            M2 (week 5)
13. E11, E12, E13       Chain + Cleanup + Spec-debt                M3+ (week 6)
14. E19/E20 -> E21 -> E22  Kernel upgrade                         Phase 1 (weeks 7-9)
15. E23-E32             Agent cognition + Infrastructure           Phase 2 (weeks 10-15)
16. E37, E43            Surfaces + Deploy                          Phase 2+ (weeks 14-16)
17. E36, E38-E41        Economy                                   Phase 3 (weeks 17-20)
```

---

_Back to index: [`00-INDEX.md`](00-INDEX.md) -- Master roadmap: [`03-WORK-BREAKDOWN-EPICS.md`](03-WORK-BREAKDOWN-EPICS.md)_
