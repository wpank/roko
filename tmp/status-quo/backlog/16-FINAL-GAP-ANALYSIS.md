# 16 — Final Gap Analysis

> **Scope**: Verify ALL features from the v2 spec (docs/v2/01-28) and all user-requested
> capabilities are covered by the backlog epics (E01-E45 + any proposed additions).
> Repo HEAD: `5852c93c05` on `main` -- authored 2026-07-10.

---

## 1. v2 Spec-to-Epic Coverage Matrix

Every v2 document (01-28) is mapped below to the epic(s) that implement it.
Status: **COVERED** = at least one epic with tasks targeting the concept.
**PARTIAL** = concept is addressed but incomplete. **GAP** = no epic coverage.

| v2 Doc | Title | Primary Epic(s) | Status | Notes |
|---|---|---|---|---|
| **01** | Signal and Pulse | E19 (Signal Protocol) | COVERED | 10 tasks: Graduation, Pulse bridges, demurrage, HDC, IFC taint, Kind registry |
| **02** | Cell and Protocols | E20 (Cell Unification) | COVERED | 10 tasks: Cell supertrait, 9 protocols, TypeSchema, predict-publish-correct |
| **03** | Graph | E21 (Graph Engine) | COVERED | 10 tasks: typed edges, Hot Graphs, Workflow/Activity split, merge queue |
| **04** | Execution Engine | E22 (Execution Runtime) | COVERED | 10 tasks: 7 cognitive loop Cells, nested loops, T0 short-circuit, error taxonomy |
| **05** | Agent Runtime | E23 (Agent Cognitive Autonomy) | COVERED | 10 tasks: type-state, behavioral phases, CorticalState, EFE routing |
| **06** | Memory and Knowledge | E24 (Memory Advanced) + E07 | COVERED | E24: 10 tasks (heuristics, Allen intervals, resonator networks, dreams). E07: 10 tasks (LinUCB, HDC, knowledge income) |
| **07** | Learning Loops | E25 (Learning Loops Advanced) + E07 | COVERED | E25: 10 tasks (L3 HDC, L4 c-factor, variance inequality). E07: runtime wiring |
| **08** | Inference Gateway | E26 (Inference Gateway) | COVERED | 12 tasks: 9-stage pipeline, InferenceHandle, Batch API, CascadeRouter fallback |
| **09** | Feeds and Recipes | E27 (Feeds System) | COVERED | 8 tasks: Feed trait, registry, raw/derived/composite, recipes, marketplace |
| **10** | Groups and Coordination | E28 (Groups & Coordination) | COVERED | 8 tasks: Group as Space, 4 coordination modes, membership, pheromone fields |
| **11** | Connectivity and Relay | E29 (Connectivity & Relay) | COVERED | 9 tasks: Connect protocol, relay wire protocol, A2A cards, backpressure |
| **12** | Extension System | E30 (Extension System) | COVERED | 8 tasks: Extension trait, 22 hooks, CaMeL IFC, discovery/resolution |
| **13** | Trigger System | E31 (Trigger System) | COVERED | 8 tasks: Trigger as Cell, event source registry, bindings, debounce/filter |
| **14** | Tool Catalog | E32 (Tool & Plugin Ecosystem) + E14 | COVERED | E32: 8 tasks (Plugin SDK, dynamic loading, sandboxing). E14: 7 tasks (provider/tool correctness) |
| **15** | Telemetry | E33 (Telemetry & Lens) + E09 | COVERED | E33: 9 tasks (StateHub projections, Lens, c-factor). E09: 9 tasks (MetricRegistry, log rotation) |
| **16** | Security Model | E34 (Security IFC) + E04 | COVERED | E34: 8 tasks (taint lattice, immune system, corrigibility). E04: 19 tasks (P0 fixes, safety funnel) |
| **17** | Authentication | E35 (Auth Protocol) + E04 | COVERED | E35: 8 tasks (API key rotation, JWKS, team RBAC). E04: Privy JWT, rate limiter |
| **18** | Payments | E36 (Payments) | COVERED | 8 tasks: x402, MPP, reputation pricing, settlement batching |
| **19** | Configuration | E42 (Config Evolution) + E18 | COVERED | E42: 8 tasks (Config-as-Signal, schema versioning, 7 invariants). E18: dual-config collapse |
| **20** | Surfaces | E37 (Surfaces) + E10 | COVERED | E37: 9 tasks (Workbench, Inbox, Canvas, Minimap, Autonomy Slider). E10: 7 tasks (frontend contract) |
| **21** | Marketplace | E38 (Marketplace) | COVERED | 9 tasks: agent passport, TraceRank, publish/discover/fork, DAW composability |
| **22** | On-Chain Registries | E39 (Registries & Identity) | COVERED | 8 tasks: ERC-8004, ZK-HDC, InsightStore, gossip, job market |
| **23** | Arenas and Evals | E40 (Arenas & Evals) | COVERED | 8 tasks: 7-step flywheel, scoring functions, leaderboards, bounty escrow |
| **24** | DeFi Infrastructure | E41 (DeFi Products) + E11 | COVERED | E41: 8 tasks (VCG clearing, yield perpetuals, VenueAdapter). E11: 5 tasks (chain client, deploy) |
| **25** | Deployment | E43 (Deployment & Portability) + E18 | COVERED | E43: 8 tasks (brain export, daemon, secrets rotation). E18: Docker/CI/ops |
| **26** | Cross-Cut Functors | E44 (Cross-Cut Functors) | COVERED | 8 tasks: endofunctor algebra, natural transformations, VCG arbitration, safety wrapper |
| **27** | Orchestrator | E45 (Orchestrator Mori Parity) + E01 | COVERED | E45: 10 tasks (structured review, auto-fix, error sharing, warm spawn). E01: 10 tasks (engine bootstrap) |
| **28** | Roadmap | N/A (meta-doc) | COVERED | The roadmap itself is implemented via the phased epic structure (M0-M3+, Phase 1-3) |

**Result: All 28 v2 spec documents have corresponding epic coverage. No v2 doc is orphaned.**

---

## 2. Gap Analysis: User-Requested Capabilities

### 2.1 GitHub Integration

**Question**: Can roko agents create branches, PRs, review, merge, and manage issues?

**Current state**: The codebase has git worktree support built in `roko-orchestrator/src/worktree.rs`
and merge-branch logic in `runner/merge.rs`. E01-T07 wires per-plan worktree isolation.
E18 covers GitHub CI workflows (release, deny, docs-lint). However:

**GAP FOUND**: No epic explicitly covers the full GitHub workflow that a self-developing agent
needs: creating feature branches, opening pull requests, requesting reviews, merging PRs, or
managing GitHub issues programmatically. The existing MCP crates (`roko-mcp-github`) are
listed as "Partial" in CLAUDE.md but have no dedicated epic or tasks targeting them.

| Capability | Current State | Epic Coverage |
|---|---|---|
| Create branches | Built (worktree.rs) | E01-T07 (partial) |
| Open PRs | Not wired | **NONE** |
| PR review workflow | Not wired | **NONE** |
| Merge PRs | Not wired | **NONE** |
| Manage issues | Not wired | **NONE** |
| CI/CD workflows | Partial (.github/) | E18 (CI hygiene) |

**Recommendation**: New epic **E46 (GitHub Workflow Integration)** needed. Should cover:
- Wire `roko-mcp-github` for branch/PR/issue operations
- Add `roko pr create/review/merge` CLI subcommands or MCP tools
- Wire PR-based gate flow: agent creates branch, makes changes, opens PR, gates validate
- Issue-to-PRD pipeline: GitHub issues become `prd idea` inputs
- Estimated: 8-10 tasks, Phase 2 dependency on E01 + E15 (MCP)

### 2.2 Resource Awareness

**Question**: Does the backlog cover disk space management, artifact cleanup, and worktree management?

**Current state**: Several epics touch resource management tangentially:
- E02-T07: Retention for events.jsonl, logs, and *.bak.* files
- E02-T12: Cold-substrate archival move-not-copy (prune hot store after archive)
- E03-T06: Unified RetentionPolicy across 3 GC engines
- E09-T07: GC/cap events.jsonl (size-based or split)
- E12: Dead-code cleanup (~52K LOC removal)
- Knowledge GC exists: `roko knowledge gc` CLI command

**GAP FOUND**: No epic addresses holistic resource awareness:

| Capability | Current State | Epic Coverage |
|---|---|---|
| Disk space monitoring | Not built | **NONE** |
| Worktree cleanup | Not wired | **NONE** (E01-T07 creates but doesn't clean up) |
| Artifact size budgets | Not built | **NONE** |
| Stale build cache cleanup | Not built | **NONE** |
| `.roko/` directory size management | Partial (per-file GC) | E02 (per-concern), E09-T07 (events) |
| Log rotation | E09-T05 (day-based) | COVERED |
| Knowledge GC | Built | COVERED (existing CLI) |

**Recommendation**: New epic **E47 (Resource & Disk Management)** needed. Should cover:
- Disk space monitoring and alerts (before runs, during runs)
- Worktree lifecycle: creation, cleanup after merge/abandon, stale worktree detection
- Artifact budget: max `.roko/` size, automatic cold archival trigger
- Build artifact cleanup (target/ directory management for worktrees)
- Estimated: 6-8 tasks, Phase 2 dependency on E01 + E02

### 2.3 Rate Limit Handling

**Question**: Does the backlog cover LLM provider rate limits, token budgets, and graceful degradation?

**Current state**:
- E14-T01..T07: Provider/tool dispatch correctness (P13 covers 429 retry fix)
- E04-T18: Per-API-key/per-IP rate limiter for roko-serve
- E01-T05: Agent concurrency configuration
- E26 (Inference Gateway): 9-stage pipeline includes cost tracking
- CascadeRouter: Exists with model routing, but no rate-limit awareness

**Partially covered but gaps remain**:

| Capability | Current State | Epic Coverage |
|---|---|---|
| HTTP 429 retry | Broken, P13 fixes | E14 + P13 (COVERED) |
| Per-provider rate limiting | Not built | **NONE** |
| Token budget per-plan/per-task | Not built | **NONE** (E26 has cost tracking but not budgets) |
| Graceful model fallback on quota | Not built | **NONE** (CascadeRouter routes by quality, not quota) |
| Cost ceiling / abort on overspend | Not wired | **NONE** |
| Daily/monthly spend tracking | Not wired | **NONE** |

**Recommendation**: New epic **E48 (Rate Limiting & Token Budgets)** needed. Should cover:
- Per-provider rate-limit tracking and backoff (not just HTTP retry)
- Token budget enforcement: per-plan and per-task ceilings
- Graceful cascade: when preferred provider hits quota, fall to next
- Cost tracking dashboard: daily/monthly spend, per-agent attribution
- Abort-on-overspend safety rail
- Estimated: 6-8 tasks, Phase 2 dependency on E14 + E26

### 2.4 Self-Development Workflow

**Question**: Can roko read PRDs, generate plans, execute, validate, learn, iterate via GitHub?

**Current end-to-end flow** (from CLAUDE.md and v2/28-ROADMAP):

| Step | CLI Command | Status | Epic Coverage |
|---|---|---|---|
| 1. Capture idea | `roko prd idea "..."` | Working | Existing |
| 2. Draft PRD | `roko prd draft new "slug"` | Working | Existing |
| 3. Research context | `roko research enhance-prd slug` | Working | E16 + P08/P09 |
| 4. Generate plan | `roko prd plan slug` | Working | E16 + P23 |
| 5. Execute plan | `roko plan run plans/` | **Broken (Graph default)** | **E01 (the fix)** |
| 6. Gate validation | Per-task gates | Working (but stubs pass) | E05 (honest gates) |
| 7. Persist results | Episodes, snapshots | Working | E02 (convergence) |
| 8. Resume if interrupted | `plan run --resume` | **Broken (Graph ignores)** | **E01-T02** |
| 9. Learn from results | Efficiency, routing | Partial (write-only) | E07 (close loops) |
| 10. Gate-failure replan | `build_gate_failure_plan_revision` | Prompt-append only | E01-T06 |
| 11. Create branch/PR | Not automated | **NOT WIRED** | **GAP (E46)** |
| 12. Iterate via CI | Manual | **NOT WIRED** | **GAP (E46)** |

**Result**: Steps 1-10 are covered by E01-E18. Steps 11-12 (the GitHub integration
piece) are the gap identified in section 2.1.

### 2.5 Missing v2 Subsystems Check

Cross-referencing the v2 "What Phase 0 Lacks" list (28-ROADMAP.md section 1.3) against epics:

| Missing Concept | v2 Doc | Epic | Status |
|---|---|---|---|
| Pulse/Bus kernel | 01 | E19 | COVERED |
| Predict-publish-correct | 02 | E20, E25 | COVERED |
| Demurrage on knowledge | 06 | E24 | COVERED |
| Heuristic kind | 06 | E24 | COVERED |
| EFE routing | 07 | E23, E25 | COVERED |
| Observe protocol (Lens) | 15 | E33, E13 | COVERED |
| Trigger protocol | 13 | E31 | COVERED |
| Connect protocol | 11 | E29 | COVERED |
| Graph authoring (typed) | 03 | E21 | COVERED |
| Rack abstraction | 03 | E21 | COVERED (part of Graph Engine) |
| TypeSchema validation | 02 | E20, E21 | COVERED |
| Dream cycle trigger (Loop 3) | 06 | E24 | COVERED |
| Loop 4 (structural adaptation) | 07 | E25 | COVERED |
| On-chain registries | 22 | E39 | COVERED |

**Result**: All missing v2 subsystems have epic coverage.

---

## 3. Summary of Gaps

| # | Gap | Severity | Recommendation |
|---|---|---|---|
| **G1** | GitHub workflow integration (branch/PR/issue/review/merge) | **High** | New epic E46 (8-10 tasks) |
| **G2** | Resource & disk management (monitoring, worktree cleanup, artifact budgets) | **Medium** | New epic E47 (6-8 tasks) |
| **G3** | Rate limiting & token budgets (per-provider quotas, cost ceilings, spend tracking) | **Medium** | New epic E48 (6-8 tasks) |

All three gaps are operational capabilities needed for safe, unattended self-hosting.
None are v2 spec concepts (they are infrastructure concerns), which explains why the
v2-derived epics E19-E45 don't cover them.

### What is NOT a gap

- **v2 spec coverage**: Complete. All 28 documents mapped to at least one epic.
- **Self-hosting core loop** (steps 1-10): Fully covered by E01-E18.
- **Log rotation and retention**: Covered by E02, E09.
- **Security perimeter**: Comprehensively covered by E04 (19 tasks) + E34 (8 tasks).
- **Dead-code cleanup**: Covered by E12 (9 tasks).
- **Mori parity**: Covered by E45 (10 tasks).
- **DeFi/chain**: Covered by E11, E39, E41.
- **MCP passthrough**: Covered by E15.

---

## 4. Grand Total After Gap Remediation

| Bucket | Epics | Tasks |
|---|---|---|
| Status-quo audit (E01-E18) | 18 | 149 |
| v2 spec implementation (E19-E45) | 27 | 240 |
| Proposed new epics (E46-E48) | 3 | ~24 |
| DOC reconciliation tasks | 6 plans | 71 |
| Existing executable plans (P08-P34 + side queues) | -- | 120 |
| Recovered architecture-core-queue | -- | 24 |
| **Grand total** | **48 epics** | **~628 tasks** |

---

## 5. Recommended Phasing for New Epics

| Epic | Phase | Dependencies | Rationale |
|---|---|---|---|
| E46 (GitHub Integration) | Phase 2 | E01, E15, E04 | Needs working execution + MCP + security before automated GitHub operations |
| E47 (Resource Management) | Phase 2 | E01, E02 | Needs working execution + storage convergence to manage resources intelligently |
| E48 (Rate Limiting) | Phase 2 | E14, E26 | Needs fixed providers + inference gateway architecture for quota-aware routing |

All three slot into the Phase 2 track alongside E27-E32 (Infrastructure) and can run
in parallel with them since they touch disjoint file surfaces.

---

_Back to the backlog index: [`00-INDEX.md`](00-INDEX.md)._
