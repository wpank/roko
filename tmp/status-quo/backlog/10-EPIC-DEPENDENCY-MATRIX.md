# 10 -- Epic Dependency Matrix & Cross-Reference

> Navigation: [00-INDEX.md](00-INDEX.md) | [03-WORK-BREAKDOWN-EPICS.md](03-WORK-BREAKDOWN-EPICS.md) | [05-MASTER-CHECKLIST.md](05-MASTER-CHECKLIST.md)
> Repo HEAD: `5852c93c05` on `main` -- authored 2026-07-10
> Root: `/Users/will/dev/nunchi/roko/roko`

## 1. Epic Inventory

48 epics (E01--E48), 45 with authored `tasks.toml`, 3 in progress (E46--E48).

| # | Epic ID | Title | Tasks | Milestone | Status |
|--:|---------|-------|------:|-----------|--------|
| 1 | E01 | Execution Engine | 14 | M0 | tasks.toml authored |
| 2 | E02 | Storage Convergence | 12 | M1 | tasks.toml authored |
| 3 | E03 | Type Consolidation | 7 | M1 | tasks.toml authored |
| 4 | E04 | Security Perimeter | 19 | M0/M2 | tasks.toml authored |
| 5 | E05 | Gate Adaptivity Live | 8 | M0/M1 | tasks.toml authored |
| 6 | E06 | Compose / Prompt Unify | 9 | M1 | tasks.toml authored |
| 7 | E07 | Learning & Knowledge | 10 | M2 | tasks.toml authored |
| 8 | E08 | Conductor Supervision | 9 | M2 | tasks.toml authored |
| 9 | E09 | Observability | 11 | M2 | tasks.toml authored |
| 10 | E10 | Frontend / API Contract | 7 | M2 | tasks.toml authored |
| 11 | E11 | Chain / ISFR | 5 | M3+ | tasks.toml authored |
| 12 | E12 | Dead-Code Cleanup | 9 | M3+ | tasks.toml authored |
| 13 | E13 | v2 Spec-Debt (Lens) | 3 | M3+ | tasks.toml authored |
| 14 | E14 | Providers & Tools | 10 | M1 | tasks.toml authored |
| 15 | E15 | MCP Config & Passthrough | 7 | M1 | tasks.toml authored |
| 16 | E16 | PRD Self-Hosting | 2 | M1 | tasks.toml authored |
| 17 | E17 | ACP Completion | 6 | M2 | tasks.toml authored |
| 18 | E18 | Docs, Config, CI & Ops | 15 | M2 | tasks.toml authored |
| 19 | E19 | Signal Protocol | 10 | Phase 1 | tasks.toml authored |
| 20 | E20 | Cell Unification | 10 | Phase 1 | tasks.toml authored |
| 21 | E21 | Graph Engine | 10 | Phase 1 | tasks.toml authored |
| 22 | E22 | Execution Runtime | 10 | Phase 1 | tasks.toml authored |
| 23 | E23 | Agent Cognitive Autonomy | 10 | Phase 2 | tasks.toml authored |
| 24 | E24 | Memory Advanced | 10 | Phase 2 | tasks.toml authored |
| 25 | E25 | Learning Loops Advanced | 10 | Phase 2 | tasks.toml authored |
| 26 | E26 | Inference Gateway | 12 | Phase 2 | tasks.toml authored |
| 27 | E27 | Feeds System | 8 | Phase 2 | tasks.toml authored |
| 28 | E28 | Groups & Coordination | 8 | Phase 2 | tasks.toml authored |
| 29 | E29 | Connectivity & Relay | 9 | Phase 2 | tasks.toml authored |
| 30 | E30 | Extension System | 8 | Phase 2 | tasks.toml authored |
| 31 | E31 | Trigger System | 8 | Phase 2 | tasks.toml authored |
| 32 | E32 | Tool & Plugin Ecosystem | 8 | Phase 2 | tasks.toml authored |
| 33 | E33 | Telemetry & Lens | 9 | Phase 2 | tasks.toml authored |
| 34 | E34 | Security IFC | 8 | Phase 2 | tasks.toml authored |
| 35 | E35 | Auth Protocol | 8 | Phase 2 | tasks.toml authored |
| 36 | E36 | Payments | 8 | Phase 3 | tasks.toml authored |
| 37 | E37 | Surfaces | 9 | Phase 2+ | tasks.toml authored |
| 38 | E38 | Marketplace | 9 | Phase 3 | tasks.toml authored |
| 39 | E39 | Registries & Identity | 8 | Phase 3 | tasks.toml authored |
| 40 | E40 | Arenas & Evals | 8 | Phase 3 | tasks.toml authored |
| 41 | E41 | DeFi Products | 8 | Phase 3 | tasks.toml authored |
| 42 | E42 | Config Evolution | 8 | Phase 2 | tasks.toml authored |
| 43 | E43 | Deployment & Portability | 8 | Phase 2+ | tasks.toml authored |
| 44 | E44 | Cross-Cut Functors | 8 | Phase 2 | tasks.toml authored |
| 45 | E45 | Orchestrator Mori Parity | 10 | Phase 2 | tasks.toml authored |
| 46 | E46 | GitHub Workflow Integration | -- | TBD | directory only |
| 47 | E47 | Resource & Disk Management | -- | TBD | directory only |
| 48 | E48 | Rate Limit Budgeting | -- | TBD | directory only |

**Grand total: 389 authored tasks** (E01--E45) + E46--E48 in progress.

---

## 2. Cross-Epic Dependency Edges (from `depends_on_plan`)

These are the explicit `depends_on_plan` references extracted from every `tasks.toml`.
Each row means "task X in epic A declares `depends_on_plan = [B]`", creating an
epic-level A --> B dependency edge.

### 2a. Raw edge list

| Downstream Epic | depends_on_plan target | Specific task(s) |
|-----------------|----------------------|------------------|
| E02 | E03 | E02-T08 |
| E04 | P16 | E04-T06 |
| E04 | P22 | E04-T14 |
| E05 | E02 | E05-T08 |
| E06 | E01 | E06-T01, E06-T02, E06-T06 |
| E07 | E01 | E07-T04, E07-T06, E07-T07, E07-T10 |
| E07 | P19 | E07-T09 |
| E10 | E01 | E10-T03 |
| E10 | E03 | E10-T05 |
| E12 | E03 | E12-T05 |
| E12 | E01, E04, E08 | E12-T06 |
| E12 | E05, E06, E08 | E12-T07 |
| E13 | E01 | E13-T03 |
| E14 | E01 | E14-T01 |
| E16 | P08 | E16-T1 |
| E16 | P23, P09 | E16-T2 |
| E17 | E04, P22 | E17-T01 |
| E17 | P19, E07 | E17-T02 |
| E17 | P25 | E17-T03 |
| E17 | P22 | E17-T04 |
| E17 | P28 | E17-T05 |
| E18 | E01 | E18-T10, E18-T11, E18-T12 |
| E21 | E20 | E21-T01, E21-T07 |
| E22 | E20 | E22-T01 |
| E22 | E21 | E22-T02, E22-T05, E22-T06, E22-T07 |

Non-epic plan references (Pxx tasks are pre-existing plans, not new epics):
P08, P09, P16, P19, P22, P23, P25, P28.

### 2b. Consolidated epic-to-epic dependency edges

Filtering only E-to-E edges (excluding Pxx plan references):

```
E01 <-- E05, E06, E07, E10, E12, E13, E14, E18
E02 <-- E05
E03 <-- E02, E10, E12
E04 <-- E12, E17
E05 <-- E12
E06 <-- E12
E07 <-- E17
E08 <-- E12
E20 <-- E21, E22
E21 <-- E22
```

### 2c. Inferred phase-gating dependencies (from 00-INDEX.md)

These are stated in the index/roadmap but not encoded as `depends_on_plan` in every task:

```
E01 --> E05, E06, E07, E08, E09, E14, E15, E16, E18  (M0 gates M1/M2)
E03 --> E02, E10                                       (type shapes)
E04 --> E17, E29, E34, E35                             (security perimeter)
E07 --> E24, E25                                       (learning foundations)
E08 --> E31                                            (conductor -> triggers)
E09 --> E33, E37                                       (observability -> lens/surfaces)
E11 --> E36, E39, E41                                  (chain -> economy)
E14 --> E26                                            (providers -> gateway)
E18 --> E43                                            (ops -> deployment)
E19 --> E23, E27, E42, E44                             (signal protocol -> phase 2)
E20 --> E21, E22, E23, E27, E28, E30, E44              (cell -> graph/runtime/infra)
E21 --> E22                                            (graph -> execution runtime)
E25, E39 --> E40                                       (learning+registries -> arenas)
E36, E39 --> E38                                       (payments+registries -> marketplace)
E01, E12 --> E45                                       (engine+cleanup -> mori parity)
```

---

## 3. Epic Adjacency Matrix

Reading: row depends on column. `X` = explicit `depends_on_plan` in tasks.toml.
`i` = inferred from index/roadmap gating. `.` = no dependency.

```
          E01 E02 E03 E04 E05 E06 E07 E08 E09 E10 E11 E12 E13 E14 E15 E16 E17 E18
E01        .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .
E02        i   .   X   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .
E03        .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .
E04        .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .
E05        i   X   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .
E06        X   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .
E07        X   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .
E08        i   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .
E09        i   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .
E10        X   .   X   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .
E11        .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .
E12        X   .   X   X   X   X   .   X   .   .   .   .   .   .   .   .   .   .
E13        X   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .
E14        X   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .
E15        .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .
E16        i   .   .   .   .   .   .   .   .   .   .   .   .   i   .   .   .   .
E17        .   .   .   X   .   .   X   .   .   .   .   .   .   .   i   .   .   .
E18        X   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .

          E19 E20 E21 E22 E23 E24 E25 E26 E27 E28 E29 E30 E31 E32 E33 E34 E35
E19        .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .
E20        .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .
E21        .   X   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .
E22        .   X   X   .   .   .   .   .   .   .   .   .   .   .   .   .   .
E23        i   i   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .
E24        .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .
E25        .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .
E26        .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .
E27        i   i   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .
E28        .   i   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .
E29        .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .
E30        .   i   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .
E31        .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .
E32        .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .
E33        .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .
E34        .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .
E35        .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .   .

          E36 E37 E38 E39 E40 E41 E42 E43 E44 E45 E46 E47 E48
E36        .   .   .   .   .   .   .   .   .   .   .   .   .
E37        .   .   .   .   .   .   .   .   .   .   .   .   .
E38        i   .   .   i   .   .   .   .   .   .   .   .   .
E39        .   .   .   .   .   .   .   .   .   .   .   .   .
E40        .   .   .   i   .   .   .   .   .   .   .   .   .
E41        .   .   .   i   .   .   .   .   .   .   .   .   .
E42        .   .   .   .   .   .   .   .   .   .   .   .   .
E43        .   .   .   .   .   .   .   .   .   .   .   .   .
E44        .   .   .   .   .   .   .   .   .   .   .   .   .
E45        .   .   .   .   .   .   .   .   .   .   .   .   .
```

---

## 4. Topological Sort (Valid Execution Orders)

### 4a. Strict topological order (explicit `depends_on_plan` only)

Roots (no explicit upstream): E01, E03, E04, E08, E09, E11, E15, E19, E20,
E23--E45 (self-contained or inferred-only deps).

Valid execution waves:

```
Wave 0 (roots):     E01, E03, E04, E08, E09, E11, E15, E19, E20
Wave 1 (dep wave0): E02(E03), E05(E01), E06(E01), E07(E01), E10(E01,E03),
                    E13(E01), E14(E01), E18(E01), E21(E20)
Wave 2 (dep wave1): E05-T08(E02), E12(E01,E03,E04,E05,E06,E08),
                    E17(E04,E07), E22(E20,E21)
Wave 3 (dep wave2): E12-T07(E05,E06,E08), E12-T08(E12-T07)
```

### 4b. Full topological order (explicit + inferred)

Incorporating all inferred/roadmap dependencies:

```
Wave 0:  E01, E03, E04, E11, E15, E19, E20
Wave 1:  E02, E05, E06, E07, E08, E09, E14, E18, E21
Wave 2:  E10, E12(partial), E13, E16, E17, E22, E24, E25, E26
Wave 3:  E12(full), E23, E27, E28, E29, E30, E31, E32, E33, E34, E35, E42, E43, E44
Wave 4:  E36, E37, E39, E45
Wave 5:  E38, E40, E41
Wave 6:  E46, E47, E48 (TBD -- no deps authored yet)
```

---

## 5. Critical Path Analysis

### 5a. Longest explicit dependency chain

```
E01 --> E05 --> E12-T07 --> E12-T08     (length 4)
E01 --> E06 --> E12-T07 --> E12-T08     (length 4)
E03 --> E02 --> E05-T08                 (length 3)
E20 --> E21 --> E22                     (length 3)
E01 --> E07 --> E17                     (length 3)
E04 --> E12-T06                         (length 2)
E04 --> E17                             (length 2)
```

**Critical path (longest chain through all 48 epics):**

```
E01 (Execution Engine, 14 tasks)
  --> E05 (Gate Adaptivity, 8 tasks)
    --> E12 (Dead-Code Cleanup, 9 tasks, gated by E05+E06+E08)

Total: 31 tasks on critical path, 3 sequential epic-level gates.
```

The E12 terminus requires all three of E05, E06, and E08 to complete.
Since E06 and E08 also depend on E01 (same root), the critical path fan-out is:

```
                 E01
                / | \
              E05 E06 E08
                \ | /
                 E12
                  |
              E12-T07 (delete orchestrate.rs)
                  |
              E12-T08 (remove legacy-orchestrate feature)
```

### 5b. Critical path with inferred dependencies

```
E01 --> E07 --> E17 --> (ACP fully operational)                       length 3
E20 --> E21 --> E22 --> (Phase 1 kernel complete)                     length 3
E01 --> E09 --> E33 --> E37 --> (surfaces operational)                length 4
E11 --> E39 --> E38/E40/E41 --> (economy operational)                 length 3
E19 --> E23 --> (agent autonomy)                                      length 2
E01 --> E12 --> E45 --> (mori parity)                                 length 3
```

**Longest inferred chain:**

```
E01 --> E09 --> E33 (Telemetry & Lens) --> E37 (Surfaces)
```
4 hops, ~40 tasks total on the sequential path.

---

## 6. Parallel Opportunity Analysis

### Phase M0 (Bootstrap)

| Track | Epics | Tasks | Parallel? |
|-------|-------|------:|-----------|
| Engine bootstrap | E01 | 14 | Serial prerequisite |
| Security P0s (subset) | E04 (P0 tasks only) | ~5 | Yes, with E01 |
| Gate minimum (subset) | E05 (T01-T02 only) | 2 | Yes, with E01 |

### Phase M1 (Stabilization)

After E01 completes, these are independent and fully parallelizable:

| Track | Epics | Tasks | Parallel? |
|-------|-------|------:|-----------|
| Types + Storage | E03, then E02 | 19 | Sequential pair, parallel to others |
| Providers & Tools | E14, E15 | 17 | Yes |
| Compose Unification | E06 | 9 | Yes |
| PRD Pipeline | E16 (after E14) | 2 | After E14 |
| Gate Adaptivity | E05 (full) | 8 | Yes |

**Maximum M1 parallelism: 4 tracks** (E03/E02, E14/E15, E06, E05)

### Phase M2 (Feature Complete)

| Track | Epics | Tasks | Parallel? |
|-------|-------|------:|-----------|
| Learning | E07 | 10 | Yes |
| Conductor | E08 | 9 | Yes |
| Observability | E09 | 11 | Yes |
| Frontend | E10 (after E03) | 7 | Yes |
| ACP | E17 (after E04+E07+E15) | 6 | After deps |
| Docs/CI/Ops | E18 | 15 | Mostly yes |
| Security full | E04 (remaining) | ~14 | Yes |

**Maximum M2 parallelism: 6 tracks** (E07, E08, E09, E10, E18, E04-rest)

### Phase 1 -- Kernel Upgrade

| Track | Epics | Tasks | Parallel? |
|-------|-------|------:|-----------|
| Signal Protocol | E19 | 10 | Independent root |
| Cell Unification | E20 | 10 | Independent root |
| Graph Engine | E21 (after E20) | 10 | After E20 |
| Execution Runtime | E22 (after E20, E21) | 10 | After E21 |

**Maximum Phase 1 parallelism: 2 tracks** (E19 || E20, then E21, then E22)

### Phase 2 -- Agent Cognition + Infrastructure

| Track | Epics | Tasks | Parallel? |
|-------|-------|------:|-----------|
| Agent Cognition | E23 | 10 | After E19, E20 |
| Memory Advanced | E24 | 10 | After E07 |
| Learning Advanced | E25 | 10 | After E07 |
| Inference Gateway | E26 | 12 | After E14 |
| Feeds | E27 | 8 | After E19, E20 |
| Groups | E28 | 8 | After E20 |
| Connectivity | E29 | 9 | After E04 |
| Extensions | E30 | 8 | After E20 |
| Triggers | E31 | 8 | After E08 |
| Tools/Plugins | E32 | 8 | After E14, E15 |
| Telemetry/Lens | E33 | 9 | After E09 |
| Security IFC | E34 | 8 | After E04 |
| Auth Protocol | E35 | 8 | After E04 |
| Config Evolution | E42 | 8 | After E19 |
| Cross-Cut Functors | E44 | 8 | After E19, E20 |

**Maximum Phase 2 parallelism: 15 tracks** (all above are independent of each other)

### Phase 3 -- Economy

| Track | Epics | Tasks | Parallel? |
|-------|-------|------:|-----------|
| Payments | E36 | 8 | After E11, E29 |
| Registries | E39 | 8 | After E11 |
| Surfaces | E37 | 9 | After E09, E33 |
| Marketplace | E38 | 9 | After E36, E39 |
| Arenas | E40 | 8 | After E25, E39 |
| DeFi | E41 | 8 | After E11, E39 |

**Maximum Phase 3 parallelism: 4 tracks** (E36, E39, E37 first; then E38, E40, E41)

### Phase 2+ -- Meta

| Track | Epics | Tasks | Parallel? |
|-------|-------|------:|-----------|
| Deployment | E43 | 8 | After E18 |
| Mori Parity | E45 | 10 | After E01, E12 |
| Dead-Code Full | E12 (gated tasks) | ~4 | After E05+E06+E08 |
| Spec-Debt | E13 | 3 | After E01 |

---

## 7. Cross-Reference to v2 Spec Documents (docs/v2/)

| Epic | Primary v2 Doc(s) | Section Coverage |
|------|-------------------|------------------|
| E01 | 04-EXECUTION.md, 27-ORCHESTRATOR.md | Execution engine, plan runner, DAG executor |
| E02 | (cross-cutting storage) | `.roko/` layout, signal/gate/episode stores |
| E03 | 01-SIGNAL.md, 02-CELL.md | Engram, DashboardEvent, GateVerdict, ToolResult shapes |
| E04 | 16-SECURITY.md, 17-AUTH.md | Safety funnel, capability tokens, audit chain |
| E05 | 04-EXECUTION.md (gates) | Gate rungs, adaptive thresholds, rung stats |
| E06 | 05-AGENT.md (prompts) | SystemPromptBuilder, 12-slot compose, role templates |
| E07 | 07-LEARNING.md, 06-MEMORY.md | Episodes, playbooks, LinUCB, knowledge income |
| E08 | 03-GRAPH.md (conductor) | Conductor watchers, circuit breaker, routing bias |
| E09 | 15-TELEMETRY.md | MetricRegistry, StateHub, event log, tracing |
| E10 | 20-SURFACES.md | Frontend wire contract, SSE, WebSocket, dashboard |
| E11 | 22-REGISTRIES.md, 24-DEFI.md | Chain client, ISFR, contracts, deploy parity |
| E12 | (housekeeping) | Legacy orchestrate.rs deletion, dead code |
| E13 | 15-TELEMETRY.md (Lens) | Lens trait, MetricRegistry adapter, Cell/Block naming |
| E14 | 14-TOOLS.md, 08-GATEWAY.md | Provider backends, tool handlers, retry logic |
| E15 | 14-TOOLS.md (MCP) | MCP config normalization, passthrough, env |
| E16 | 27-ORCHESTRATOR.md (PRD) | PRD pipeline, idea-draft-research-plan loop |
| E17 | ACP-INTEGRATION-GUIDE.md | ACP permission gate, learning, MCP in editor |
| E18 | 19-CONFIG.md, 25-DEPLOYMENT.md | MSRV, CI, secrets, docs, Dockerfile |
| E19 | 01-SIGNAL.md | Signal graduation, Pulse, demurrage, Kind registry |
| E20 | 02-CELL.md | Cell supertrait, TypeSchema, CellRegistry |
| E21 | 03-GRAPH.md | Typed edges, Hot Graphs, Workflow/Activity, merge queue |
| E22 | 04-EXECUTION.md | Cognitive loop Cells, nested loops, T0 short-circuit |
| E23 | 05-AGENT.md | Type-state machine, CorticalState, EFE routing |
| E24 | 06-MEMORY.md | Heuristics, Allen intervals, resonator networks, dreams |
| E25 | 07-LEARNING.md | L3 HDC defrag, L4 c-factor, experiment lifecycle |
| E26 | 08-GATEWAY.md | 9-stage inference pipeline, Batch API |
| E27 | 09-FEEDS.md | Feed trait, registry, recipes, marketplace |
| E28 | 10-GROUPS.md | Group-as-Space, coordination modes, pheromone fields |
| E29 | 11-CONNECTIVITY.md | Connect protocol, relay wire, A2A cards |
| E30 | 12-EXTENSIONS.md | Extension trait, hooks, CaMeL IFC, lifecycle |
| E31 | 13-TRIGGERS.md | Trigger-as-Cell, event sources, bindings, debounce |
| E32 | 14-TOOLS.md | Plugin SDK, dynamic loading, sandboxing |
| E33 | 15-TELEMETRY.md | StateHub projections, Lens stacking, Observe protocol |
| E34 | 16-SECURITY.md | Taint lattice, immune system, corrigibility, sandbox |
| E35 | 17-AUTH.md | API key rotation, JWKS, team RBAC, relay tokens |
| E36 | 18-PAYMENTS.md | x402, MPP, reputation pricing, settlement |
| E37 | 20-SURFACES.md | Workbench, Inbox, Canvas, Minimap, Autonomy Slider |
| E38 | 21-MARKETPLACE.md | Agent passport, TraceRank, publish/discover/fork |
| E39 | 22-REGISTRIES.md | ERC-8004, ZK-HDC, on-chain InsightStore |
| E40 | 23-ARENAS.md | 7-step flywheel, scoring, leaderboards, bounty escrow |
| E41 | 24-DEFI.md | VCG clearing, yield perpetuals, DeFiRiskEngine |
| E42 | 19-CONFIG.md | Config-as-Signal, schema versioning, hot-reload |
| E43 | 25-DEPLOYMENT.md | Brain export/import, daemon, secrets rotation |
| E44 | 26-CROSS-CUTS.md | Endofunctor algebra, natural transformations, VCG |
| E45 | 27-ORCHESTRATOR.md | Structured review, auto-fix, reflection loop |
| E46 | (GitHub CI/CD) | GitHub workflow integration (TBD) |
| E47 | 25-DEPLOYMENT.md | Resource and disk management (TBD) |
| E48 | 08-GATEWAY.md, 19-CONFIG.md | Rate limit budgeting (TBD) |

---

## 8. Cross-Reference to Crates

| Epic | Primary Crate(s) Modified |
|------|--------------------------|
| E01 | `roko-cli` |
| E02 | `roko-cli`, `roko-fs` |
| E03 | `roko-core`, `roko-chain` |
| E04 | `roko-serve`, `roko-agent`, `roko-cli`, `roko-core`, `roko-acp` |
| E05 | `roko-cli`, `roko-gate`, `roko-core`, `roko-runtime` |
| E06 | `roko-cli` |
| E07 | `roko-cli`, `roko-learn`, `roko-neuro` |
| E08 | `roko-cli`, `roko-conductor` (consumer) |
| E09 | `roko-cli`, `roko-runtime`, `roko-serve`, `roko-fs`, `agent-relay`, `roko-chain-watcher` |
| E10 | `roko-serve`, `demo/demo-app` |
| E11 | `roko-chain`, `contracts/` |
| E12 | `roko-cli`, `roko-orchestrator` (delete), `roko-plugin` (retire), `roko-runtime`, `roko-index` |
| E13 | `roko-core` |
| E14 | `roko-std`, `roko-agent` |
| E15 | `roko-cli`, `roko-acp` |
| E16 | `roko-agent`, `roko-cli` |
| E17 | `roko-acp` |
| E18 | `docs/`, `.github/`, `Cargo.toml`, `Dockerfile` |
| E19 | `roko-core` |
| E20 | `roko-core`, `roko-std` |
| E21 | `roko-graph` (new) |
| E22 | `roko-graph` |
| E23 | `roko-agent`, `roko-cli`, `roko-daimon`, `roko-learn`, `roko-runtime` |
| E24 | `roko-neuro`, `roko-dreams` |
| E25 | `roko-cli`, `roko-learn` |
| E26 | `roko-gateway` (new) |
| E27 | `roko-cli`, `roko-core`, `roko-serve` |
| E28 | `roko-core`, `roko-compose`, `roko-serve` |
| E29 | `roko-core` (relay wire) |
| E30 | `roko-core`, `roko-cli` |
| E31 | `roko-core`, `roko-cli`, `roko-conductor`, `roko-fs`, `roko-serve` |
| E32 | `roko-std`, `roko-agent`, `roko-cli`, `roko-plugin` |
| E33 | `roko-core`, `roko-runtime`, `roko-serve` |
| E34 | `roko-core`, `roko-agent`, `roko-orchestrator` |
| E35 | `roko-serve` |
| E36 | `roko-core`, `roko-chain`, `roko-learn`, `roko-serve` |
| E37 | `roko-core`, `roko-cli`, `roko-serve` |
| E38 | `roko-core`, `roko-chain`, `roko-cli`, `roko-serve` |
| E39 | `roko-chain` |
| E40 | `roko-chain`, `roko-serve` |
| E41 | `roko-core`, `roko-compose`, `roko-daimon`, `roko-serve` |
| E42 | `roko-core` |
| E43 | `roko-cli`, `roko-neuro` |
| E44 | `roko-cli`, `roko-compose` |
| E45 | `roko-cli`, `roko-learn`, `roko-neuro` |

---

## 9. Risk Register (Blast Radius Analysis)

### 9a. Most-depended-on epics (fan-out)

Epics with the most downstream dependents. Failure or delay here has the
highest blast radius.

| Rank | Epic | Direct dependents | Indirect dependents | Total fan-out | Risk |
|-----:|------|:-----------------:|:-------------------:|:-------------:|------|
| 1 | **E01** | 12 | 36 | 48 | **CRITICAL** -- gates everything; M0 bootstrap |
| 2 | **E20** | 7 | 15+ | 22+ | **HIGH** -- Cell unification gates Phase 1+2 kernel |
| 3 | **E03** | 3 | 8 | 11 | **HIGH** -- type shapes unblock storage + frontend |
| 4 | **E04** | 5 | 10+ | 15+ | **HIGH** -- security gates ACP, connectivity, IFC, auth |
| 5 | **E19** | 5 | 10+ | 15+ | **MEDIUM** -- signal protocol gates Phase 2 infra |
| 6 | **E21** | 1 | 5+ | 6+ | **MEDIUM** -- graph engine gates execution runtime |
| 7 | **E11** | 4 | 8 | 12 | **MEDIUM** -- chain gates economy (Phase 3) |
| 8 | **E09** | 2 | 4 | 6 | **MEDIUM** -- observability gates telemetry + surfaces |
| 9 | **E07** | 3 | 6 | 9 | **MEDIUM** -- learning gates ACP, memory, advanced learning |
| 10 | **E08** | 2 | 4 | 6 | **LOW** -- conductor gates dead-code + triggers only |

### 9b. Most-dependent epics (fan-in)

Epics that depend on the most upstream prerequisites. These are bottleneck
risks -- they cannot start until many things complete.

| Rank | Epic | Direct upstream deps | Risk |
|-----:|------|:--------------------:|------|
| 1 | **E12** | 6 (E01,E03,E04,E05,E06,E08) | **CRITICAL** -- maximally gated |
| 2 | **E17** | 5 (E04,E07,E15,P19,P22) | **HIGH** -- ACP needs security + learning + MCP |
| 3 | **E22** | 2 (E20,E21) | **MEDIUM** -- sequential Phase 1 chain |
| 4 | **E38** | 2 (E36,E39) | **MEDIUM** -- marketplace needs payments + registries |
| 5 | **E45** | 2 (E01,E12) | **MEDIUM** -- parity needs engine + cleanup |
| 6 | **E05** | 2 (E01,E02) | **LOW** -- E02 dep is soft/partial |

### 9c. Crate hotspots

Crates modified by the most epics -- merge conflict and coordination risk.

| Rank | Crate | Epics touching it | Count |
|-----:|-------|-------------------|------:|
| 1 | `roko-cli` | E01,E02,E04,E05,E06,E07,E08,E09,E12,E15,E16,E23,E25,E27,E30,E31,E32,E37,E38,E43,E44,E45 | 22 |
| 2 | `roko-core` | E03,E04,E05,E13,E19,E20,E27,E28,E29,E30,E31,E33,E34,E36,E37,E38,E41,E42 | 18 |
| 3 | `roko-serve` | E04,E09,E10,E27,E31,E33,E35,E36,E37,E38,E40,E41 | 12 |
| 4 | `roko-agent` | E04,E14,E16,E23,E32,E34 | 6 |
| 5 | `roko-chain` | E03,E11,E36,E38,E39,E40 | 6 |
| 6 | `roko-learn` | E07,E23,E25,E36,E45 | 5 |

---

## 10. Dependency DAG (ASCII)

```
                                     E01 (M0 root, 14 tasks)
                                      |
         .----------------------------+-----------------------------------.
         |           |           |         |          |          |         |
        E05         E06         E07       E08        E09       E14       E18
      gates-live  compose    learn-know  conductor   obs     prov/tool  docs/ops
         |           |           |         |          |          |
         |           |           |         |          |         E15
         |           |           |         |          |        mcp-cfg
         |           |           |         |          |          |
  E03    |           |           |         |          |         E16
  types  |           |           |         |          |        prd-self
   |     |           |           |         |          |
   +--E02|           |           |         |          |
   |  storage        |           |         |          |
   |     |           |           |         |          |
   +--E10|           |           |         |          |
   | frontend        |           |         |          |
   |                 |           |         |          |
   +--E12-T05        |           |         |          |
   |   (HDC dedup)   |           |         |          |
   |                 |           |         |          |
   '----E12(full)----'--------E12-T07------'          |
        dead-code      (needs E05+E06+E08)            |
            |                                          |
           E45                                        E33
        mori-parity                                 tel-lens
                                                       |
  E04 (Security Perimeter)                            E37
   |                                                surfaces
   +--E17 (ACP, +E07,E15)
   +--E29 (Connectivity)
   +--E34 (Security IFC)
   +--E35 (Auth Protocol)
   +--E12-T06 (safety dedup)

  E19 (Signal)                    E20 (Cell)
   |                               |
   +--E23 (Agent Cognition,+E20)   +--E21 (Graph)
   +--E27 (Feeds, +E20)           |    |
   +--E42 (Config Evolution)      |   E22 (Execution Runtime)
   +--E44 (Cross-Cuts, +E20)      |
                                   +--E23, E27, E28, E30, E44

  E11 (Chain/ISFR)       E07 --> E24 (Memory Adv)
   |                     E07 --> E25 (Learning Adv)
   +--E36 (Payments,+E29)     E25+E39 --> E40 (Arenas)
   +--E39 (Registries)
   +--E41 (DeFi,+E39)   E36+E39 --> E38 (Marketplace)

  E13 (Spec-Debt, M3+, gates nothing)
  E46/E47/E48 (TBD, no deps yet)
```

---

## 11. Summary Statistics

| Metric | Value |
|--------|------:|
| Total epics | 48 |
| Epics with authored tasks.toml | 45 |
| Epics pending authoring | 3 (E46, E47, E48) |
| Total authored tasks | 389 |
| Explicit cross-epic dependency edges | 17 |
| Inferred dependency edges | ~25 |
| Root epics (no upstream deps) | 7 (E01, E03, E04, E11, E15, E19, E20) |
| Terminal epics (no downstream deps) | ~20 |
| Maximum parallelism (Phase 2) | 15 simultaneous tracks |
| Critical path length | 4 hops (E01-->E05-->E12-T07-->E12-T08) |
| Highest fan-out epic | E01 (48 total dependents) |
| Highest fan-in epic | E12 (6 direct upstream deps) |
| Most-touched crate | roko-cli (22 epics) |

---

_Back to [00-INDEX.md](00-INDEX.md) | [03-WORK-BREAKDOWN-EPICS.md](03-WORK-BREAKDOWN-EPICS.md)_
