# 04 — Implementation Priorities

> Self-contained guide to what to build and in what order.
> Assumes no prior context. Read this to understand the full roadmap from current state to launch.

**Date**: 2026-04-26
**Codebase**: 18 crates, ~177K LOC Rust
**Status**: Core self-hosting loop works end-to-end. Protocol spec (v2) drafted.

---

## 1. What Exists Today

Roko is a Rust toolkit for building agents that build themselves. The core loop
(plan-execute-gate-persist) works end-to-end via CLI. The system can read PRDs,
generate implementation plans, execute tasks via Claude agents, validate results
through an 11-gate pipeline, persist state, and resume from snapshots.

### 1.1 Crate Map

| Crate | What | Status |
|---|---|---|
| `roko-core` | Signal + 6 verb traits (Store/Score/Verify/Route/Compose/React), types, config, errors | Kernel, stable |
| `roko-agent` | 5+ LLM backends (Claude CLI, Claude API, Codex, Cursor, OpenAI-compat, Ollama, Gemini, Perplexity), pools, MCP, tool loop, safety | Dispatch wired |
| `roko-agent-server` | Per-agent HTTP sidecar: `/message`, `/stream`, `/predictions`, `/research`, `/tasks` | Wired |
| `roko-orchestrator` | Plan DAG, parallel executor, merge queue, safety | Wired via orchestrate.rs |
| `roko-gate` | 11 gates, 7-rung pipeline, adaptive thresholds | Wired, called per-task |
| `roko-compose` | Prompt assembly, 9 templates, VCG auction (built but greedy path dominates), enrichment | Wired |
| `roko-learn` | Episodes, cascade router, experiments, efficiency, bandits | Fully wired |
| `roko-neuro` | Durable knowledge store, tiers, HDC fingerprints, distillation | Wired |
| `roko-dreams` | NREM/REM/Integration phases | Built, **not triggered at runtime** |
| `roko-conductor` | 10 watchers, circuit breaker, diagnosis | Wired into executor |
| `roko-runtime` | ProcessSupervisor, event bus, cancellation | Wired into PlanRunner |
| `roko-primitives` | HDC vectors, tier routing | Fully wired |
| `roko-daimon` | Affect engine, somatic markers, dispatch modulation | Wired per-task |
| `roko-serve` | ~85 HTTP routes, SSE, WebSocket on :6677 | Wired |
| `roko-cli` | All CLI subcommands + ratatui TUI (F1-F7 tabs) | Main entry point |
| `roko-fs` | FileSubstrate (JSONL), GC, layout | Stable |
| `roko-std` | 19 builtin tools, mock dispatcher | Stable |
| `roko-mcp-code` | Code-intelligence MCP server | Wired |
| `roko-index` | Parser + graph + HDC indexing | Built |
| `roko-chain` | Nunchi blockchain primitives: ChainClient, ChainWallet, TxSimGate, WalletGate, marketplace, validation registry | Partial (traits + mocks; awaiting Nunchi testnet) |

### 1.2 Working End-to-End Flows

These work today via `cargo run -p roko-cli`:

1. **Self-hosting loop**: `prd idea` -> `prd draft` -> `prd plan` -> `plan run` -> gate validate -> persist -> `plan run --resume`
2. **Research-enhanced planning**: `research enhance-prd` -> `prd plan` with research context
3. **Automatic replan**: Gate failure triggers `build_gate_failure_plan_revision`
4. **Auto-plan on publish**: `prd.auto_plan` config triggers plan generation when PRD is published
5. **Interactive monitoring**: `roko dashboard` TUI with F1-F7 tabs
6. **HTTP control plane**: `roko serve` exposes ~85 routes for external callers
7. **Agent sidecar**: `roko agent serve` with real LLM dispatch

### 1.3 What Is Built But Not Wired

These components exist as compiled code but are not called from any runtime path:

| Component | Where | Gap |
|---|---|---|
| VCG auction in composition | `vcg_allocate` built and exported | Greedy path dominates; VCG never selected at runtime |
| Dream cycle | `roko-dreams` crate | NREM/REM/Integration built; no cron/trigger fires them |
| Safety contracts | `AgentContract` wired | Falls back to permissive default when YAML missing |
| Cold substrate archival | `roko-fs` | Built but no trigger instantiates it |
| `force_backend` learning | Cascade router | Manual overrides do not feed back into router state |
| Knowledge-informed routing | `roko-neuro` store | Not consulted for model selection in CascadeRouter |

### 1.4 What Does Not Exist Yet (in the unified spec vocabulary)

- No **Observe protocol** (Lens system) -- monitoring is ad-hoc, not protocol-based
- No **Trigger protocol** -- triggers are hardcoded, not declarative
- No **Connect protocol** formalization -- connectors exist but are not a trait
- No **Bus trait** -- event bus is an implementation detail, not a kernel-level abstraction
- No **Graph authoring** -- plans are TOML task lists, not typed Graphs with edges
- No **Rack** abstraction -- no parameterized Graphs
- No **TypeSchema** validation at load time
- No **Pulse** as first-class type -- ephemeral events are `Envelope<E>`, not `Pulse`
- No **Loop 4** (structural adaptation / self-evolution)
- No **CalibrationPolicy** (predict-publish-correct)
- No Nunchi testnet or on-chain registries deployed (Phase 4+)

---

## 2. Phase 0: Launch Artifacts (the MCP Playbook)

**Timeline**: 4-6 weeks
**Goal**: Ship the artifacts that every successful protocol shipped on day one.
**Precedent**: MCP went from launch to 97M monthly SDK downloads in 13 months.
The template is spec + two SDKs + five demos + first-party host + anchor partners.

### 2.1 Required Artifacts

| # | Artifact | Size | Notes |
|---|---|---|---|
| 1 | **Spec document** | L | The unified spec (21 docs) exists in draft. Needs editorial pass, public hosting, versioning. Machine-parseable structure required (agents read it at startup). |
| 2 | **TypeScript SDK** | L | Reference implementation of Signal/Cell/Graph primitives. This is the MCP-set baseline -- anything less than TS + Python on day one feels half-finished. |
| 3 | **Python SDK** | L | Same scope as TypeScript. Python is the ML/data audience. |
| 4 | **5 demo integrations** | M each | Concrete, runnable demos that express the core differentiator in <=7 lines. Two agents must compose and produce a non-trivial result. |
| 5 | **First-party host** | M | Roko itself is the first-party host. `roko serve` on :6677 is the first consumer of the protocol. |
| 6 | **One-line analogy** | S | "USB-C for AI" did half the work for MCP. The analogy must make the current default feel obviously broken. Candidate: "Every agent run is a replayable, shareable trace URL." |
| 7 | **5 anchor partners** | -- | 3-5 adopters representing >50% of addressable market triggers self-reinforcement (Katz-Shapiro). Public quotes before launch. |
| 8 | **ACP registry submission** | S | Register roko as an ACP-compatible agent framework. A2A v1.0 is effectively unopposed as the cross-vendor agent bus. |

### 2.2 Demo Integration Targets

Pick five from:

| Integration | Why | Effort |
|---|---|---|
| Claude Code extension | Largest agent-coding surface. Distribution channel #1 in 2026. | M |
| Cursor extension | Second-largest. 40% of recent YC batch uses Supabase/Cursor. | M |
| VS Code extension | Broadest IDE reach. | M |
| GitHub Actions | CI/CD is a natural Graph execution surface. | S |
| Slack bot | Enterprise entry point. MCP has a Slack server. | M |
| Linear integration | Linear reports 24.4% agent-delegated work in April 2026. | M |
| Supabase template | Supabase's growth ($30M -> $70M ARR in 8 months) was AI-tool driven. | S |

### 2.3 Launch Sequence

Week 1-2: Finalize spec, begin SDK scaffolds, select anchor partners.
Week 2-4: SDK core (Signal/Cell/Graph types, serialize/deserialize, basic Graph execution).
Week 3-5: Demo integrations (parallel track).
Week 4-6: ACP registry, launch blog, documentation site, public announcement.

### 2.4 What NOT to Ship in Phase 0

- On-chain anything. Nunchi blockchain and ERC-8004 integration is Phase 4+.
- Marketplace. The local registry is sufficient for launch.
- L4 self-evolution. This is Phase 2+ and requires the Graph engine.
- Token economics. "No token speculation" is an anti-principle.

### 2.5 Dashboard Refocus for Pitch (1-2 Weeks)

The Nunchi app dashboard currently has 27+ pages across 7 sections (PULSE,
FLEET, FORGE, KNOWLEDGE, ARENA, MEASUREMENTS, TREASURY). The full dashboard
is the product; the pitch needs a focused story. This work runs in parallel
with SDK and demo development.

| Task | Target | Size |
|---|---|---|
| Strip app sidebar to 3-4 essential views for demo mode | `nunchi-dashboard/` | M |
| Embed terminal-based cost comparison in app (not just landing page) | `nunchi-dashboard/` | M |
| Update landing page Scaffold section with HAL-calibrated numbers ($44.86 -> $1.42) | `nunchi-dashboard/` | S |
| Update Anatomy section to unified vocabulary (or create simplified version) | `nunchi-dashboard/` | M |
| Add "Demo Mode" toggle that shows the pitch-optimized view | `nunchi-dashboard/` | M |

The pitch-optimized view should show only: Command Center (live chain +
agent fleet), Live Console (terminal output), and a new "Demo" view
(side-by-side cost comparison with the HAL $44.86 -> $1.42 beat). Everything
else should be accessible but hidden behind a toggle.

---

## 3. Phase 1: Core Protocol Implementation (Weeks 1-8)

**Goal**: Promote the unified vocabulary from spec to running code. Close the gap
between what the spec describes and what the runtime executes.

### 3.1 Priority Order

Tasks are ordered by dependency and impact. Items marked [PARALLEL] can run
concurrently with the previous item.

#### P1.1 — Pulse as First-Class Type

**Why first**: Every other protocol change depends on the Bus carrying typed Pulses.
The current `Envelope<E>` is untyped. Pulse is the ephemeral sibling of Signal.
Without it, predict-publish-correct has no transport.

| Task | Target | Size |
|---|---|---|
| Define `Pulse` struct in roko-core | `crates/roko-core/src/pulse.rs` | S |
| Add sequence numbering + ring buffer semantics | Same file | S |
| Migrate existing event bus `Envelope<E>` to Pulse | `crates/roko-runtime/src/event_bus.rs` | M |

#### P1.2 — Bus Trait Extraction

**Why second**: Bus is the transport fabric. Promoting it from implementation detail
to kernel trait (alongside Store) enables protocol-based observation and learning.

| Task | Target | Size |
|---|---|---|
| Define `Bus` trait in roko-core (publish, subscribe, topic taxonomy) | `crates/roko-core/src/traits.rs` | M |
| Implement default `InProcessBus` wrapping existing event bus | `crates/roko-runtime/src/bus.rs` | M |
| Wire Bus into orchestrate.rs (replace direct event_bus calls) | `crates/roko-cli/src/orchestrate.rs` | M |

#### P1.3 — Predict-Publish-Correct (CalibrationPolicy) [PARALLEL with P1.2]

**Why now**: This is the structural mechanism for all four learning loops.
Every Cell publishes its prediction as a Pulse, reality publishes the outcome,
a CalibrationPolicy joins by lineage and computes error.

| Task | Target | Size |
|---|---|---|
| Define `CalibrationPolicy` trait | `crates/roko-core/src/calibration.rs` | S |
| Implement for gate thresholds (L1 predict-publish-correct) | `crates/roko-gate/src/calibration.rs` | M |
| Implement for cascade router (L2 predict-publish-correct) | `crates/roko-learn/src/calibration.rs` | M |
| Wire into orchestrate.rs per-task dispatch | `crates/roko-cli/src/orchestrate.rs` | M |

#### P1.4 — Verify Protocol Redesign

**Why now**: Verify is load-bearing -- it is the reward function (continuous
`Verdict.reward`), the relabeling oracle, the safety boundary, and the economic
attestation. The current Gate trait lacks pre-action verification, continuous
reward, and typed evidence.

| Task | Target | Size |
|---|---|---|
| Add `verify_pre()` to Gate trait (pre-action check) | `crates/roko-core/src/traits.rs` | M |
| Add continuous `Verdict.reward: f64` to gate verdicts | `crates/roko-gate/src/verdict.rs` | S |
| Separate `EvidenceCollector` from `Criterion` | `crates/roko-gate/src/evidence.rs` | M |
| Implement conjunctive hard + Pareto soft (replace weighted-sum) | `crates/roko-gate/src/pipeline.rs` | L |
| Wire `verify_pre` into orchestrate.rs dispatch path | `crates/roko-cli/src/orchestrate.rs` | M |

#### P1.5 — Observe Protocol + 10 Built-In Lenses [PARALLEL with P1.4]

**Why now**: Monitoring exists but is not protocol-based. Lenses are read-only
views that never modify what they observe. StateHub projections consumed by TUI,
HTTP routes, and audit.

| Task | Target | Size |
|---|---|---|
| Define `Observe` trait in roko-core | `crates/roko-core/src/traits.rs` | S |
| Define `Lens` specialization struct | `crates/roko-core/src/lens.rs` | S |
| Implement 10 lenses: Agent, Plan, Gate, Router, Memory, Cost, Health, Error, Throughput, Dream | Various crates | M each |
| Wire Lenses into TUI dashboard | `crates/roko-cli/src/tui/` | M |
| Wire Lenses into HTTP routes | `crates/roko-serve/src/routes/` | M |

#### P1.6 — Hot Graph Execution

**Why**: Hot Graphs stay resident between firings and re-fire per tick. This is
what makes Loops (L1-L4) possible as runtime constructs, not batch jobs.

| Task | Target | Size |
|---|---|---|
| Add `hot = true` flag to Graph definition | `crates/roko-orchestrator/src/plan.rs` | S |
| Implement tick-driven re-execution for hot Graphs | `crates/roko-orchestrator/src/executor.rs` | L |
| Wire hot Graphs into `roko serve` startup | `crates/roko-serve/src/lib.rs` | M |

#### P1.7 — Workflow/Activity Split

**Why**: Deterministic orchestration (Workflow) must be separated from non-deterministic
execution (Activity). This is the Temporal pattern. Without it, replay and
deterministic testing are impossible.

| Task | Target | Size |
|---|---|---|
| Define Workflow (deterministic) vs Activity (non-deterministic) boundary | `crates/roko-orchestrator/src/graph/` | M |
| Ensure all LLM calls, filesystem writes, and network calls are Activities | `crates/roko-agent/src/` | L |
| Implement deterministic replay for Workflow-only paths | `crates/roko-orchestrator/src/replay.rs` | L |

### 3.2 Phase 1 Dependency Graph

```
P1.1 (Pulse) ──────> P1.2 (Bus) ──────> P1.3 (CalibrationPolicy)
                                              │
                                              v
                                         P1.6 (Hot Graph)
                                              │
                                              v
                                         P1.7 (Workflow/Activity)

P1.4 (Verify redesign)   [independent, parallel track]
P1.5 (Observe + Lenses)  [independent, parallel track]
```

P1.1 and P1.2 are sequential (Bus depends on Pulse).
P1.3 depends on P1.2 (CalibrationPolicy publishes/subscribes via Bus).
P1.4 and P1.5 are independent and can run in parallel with everything.
P1.6 depends on P1.2 (hot Graphs tick via Bus).
P1.7 depends on P1.6 (replay requires the execution engine distinction).

### 3.3 Phase 1 Success Criteria

- [ ] `Pulse` type exists in roko-core with sequence numbering
- [ ] `Bus` trait exists in roko-core; `InProcessBus` wraps existing event bus
- [ ] At least one CalibrationPolicy runs per-task (gate threshold or router)
- [ ] `verify_pre()` fires before every agent dispatch in orchestrate.rs
- [ ] `Verdict.reward` is a continuous f64, not just pass/fail
- [ ] 10 Lenses exist and are wired into TUI + HTTP
- [ ] A hot Graph stays resident and re-fires on tick
- [ ] All existing tests pass (no regressions)

---

## 4. Phase 2: Differentiation Surface (Weeks 8-16)

**Goal**: Build the systems that make roko structurally different from LangGraph,
CrewAI, and Microsoft Agent Framework. These are the primitives that compound.

### 4.1 EFE Routing (replacing LinUCB)

Expected Free Energy routing is the principled replacement for the current
LinUCB bandit in CascadeRouter. EFE balances epistemic value (information gain)
against pragmatic value (goal progress) conditioned on the current regime.

| Task | Target | Size |
|---|---|---|
| Define `EFEModel` with Bayesian posteriors | `crates/roko-learn/src/efe.rs` | L |
| Implement T0/T1/T2 gating as EFE bound evaluation | Same | L |
| Add regime conditioning (Crisis/Calm/Volatile shift priors) | Same | M |
| Migrate CascadeRouter from LinUCB to EFE | `crates/roko-learn/src/cascade_router.rs` | L |
| Wire `Route.feedback()` into orchestrate.rs post-task | `crates/roko-cli/src/orchestrate.rs` | S |

**Dependency**: P1.2 (Bus) for predict-publish-correct feedback.

### 4.2 Demurrage (replacing Ebbinghaus)

Attention-weighted retention replacing pure time decay. Signals decay via
holding cost unless actively used. Retrieval, citation, surprise, and gate-pass
restore balance. Self-trimming knowledge.

| Task | Target | Size |
|---|---|---|
| Implement demurrage balance on Signal | `crates/roko-core/src/signal.rs` | M |
| Add balance refresh on retrieval/citation/gate-pass | `crates/roko-neuro/src/demurrage.rs` | M |
| Implement cold threshold freeze + archive trigger | `crates/roko-neuro/src/archive.rs` | M |
| Wire into knowledge store queries | `crates/roko-neuro/src/store.rs` | M |

### 4.3 Heuristic Kind with Falsifiers

First-class Signal kind with when/then clauses and mandatory falsifiers.
A heuristic without a falsifier is an opinion, not knowledge.

| Task | Target | Size |
|---|---|---|
| Define `Heuristic` Signal kind with when/then + falsifier | `crates/roko-core/src/heuristic.rs` | M |
| Implement calibration tracking (prediction vs outcome) | `crates/roko-neuro/src/heuristic.rs` | M |
| Wire heuristic bids into CognitiveWorkspace VCG | `crates/roko-compose/src/cognitive_workspace.rs` | M |

### 4.4 CognitiveWorkspace (VCG + Section Effects)

Learnable context assembly. Context bidders compete in a VCG auction for prompt
space. Section effects (beta-distribution posteriors) track which context
sections correlate with gate success.

| Task | Target | Size |
|---|---|---|
| Implement `CognitiveWorkspace` struct | `crates/roko-compose/src/cognitive_workspace.rs` | L |
| Wire VCG auction (already built) as the allocation mechanism | Same | M |
| Implement section effect tracking (beta-distribution posteriors) | Same | M |
| Implement context bidders: Neuro, Task, Research, Heuristic, Playbook | Same | L |
| Wire into orchestrate.rs dispatch (replace current prompt assembly) | `crates/roko-cli/src/orchestrate.rs` | L |

**Dependency**: P4.2 (Demurrage) for bidder valuation; P4.3 (Heuristic) for heuristic bids.

### 4.5 Agent Vitality + Behavioral Phases

Agents have finite vitality (`remaining_budget / initial_budget`) that creates
behavioral phases: Thriving -> Stable -> Conservation -> Declining -> Terminal.
Economic pressure drives efficient resource use and knowledge transfer.

| Task | Target | Size |
|---|---|---|
| Define vitality scalar and phase thresholds | `crates/roko-agent/src/vitality.rs` | M |
| Implement phase-dependent behavior modulation | Same | M |
| Wire vitality into agent dispatch decisions | `crates/roko-cli/src/orchestrate.rs` | M |

### 4.6 Type-State Lifecycle

Compile-time enforced Agent state transitions. An Agent in `Created` state
cannot dispatch; an Agent in `Terminal` state cannot be restarted.

| Task | Target | Size |
|---|---|---|
| Define type-state states: Created, Initializing, Ready, Running, Paused, Terminal | `crates/roko-agent/src/lifecycle.rs` | M |
| Enforce transitions at compile time via phantom types or sealed traits | Same | L |

### 4.7 CorticalState (Lock-Free Atomics)

Lock-free atomic shared perception surface. Multiple agent slots read/write
CorticalState concurrently without locks.

| Task | Target | Size |
|---|---|---|
| Define `CorticalState` with atomic fields | `crates/roko-agent/src/cortical.rs` | L |
| Integrate with agent tick pipeline (read on tick, write after action) | `crates/roko-agent/src/pipeline.rs` | M |

### 4.8 Somatic Markers

PAD affect model + prospect theory + k-d tree queries for <100us emotional
context retrieval. Somatic markers bias routing and composition decisions.

| Task | Target | Size |
|---|---|---|
| Extend DaimonState with PAD affect model | `crates/roko-daimon/src/somatic.rs` | M |
| Implement k-d tree for fast marker queries | Same | M |
| Wire somatic markers into EFE routing bias | `crates/roko-learn/src/efe.rs` | S |

### 4.9 Phase 2 Dependency Graph

```
P4.1 (EFE) ────────────────────────────────> P4.8 (Somatic markers bias EFE)
P4.2 (Demurrage) ──> P4.3 (Heuristic) ──> P4.4 (CognitiveWorkspace)
P4.5 (Vitality) ──> P4.6 (Type-state lifecycle)
P4.7 (CorticalState)  [independent]
```

P4.1-P4.3 and P4.5-P4.7 can run as two parallel tracks.
P4.4 depends on P4.2 and P4.3.
P4.8 depends on P4.1.

---

## 5. Phase 3: Distribution (Weeks 16-24)

**Goal**: Ship the infrastructure that makes roko a platform, not just a tool.
Third parties build on the protocol because not adopting is more expensive than adopting.

### 5.1 ACP Server (`roko acp`)

Register roko agents as ACP-compatible services. A2A v1.0 is effectively
unopposed as the cross-vendor agent bus heading into Q3 2026.

| Task | Target | Size |
|---|---|---|
| Implement A2A Agent Card generation from roko agent config | `crates/roko-serve/src/acp.rs` | M |
| Implement Signed Agent Cards (v1.0 requirement) | Same | M |
| Add `roko acp register/discover/invoke` CLI commands | `crates/roko-cli/src/acp.rs` | L |
| Wire ACP discovery into agent dispatch (discover external agents) | `crates/roko-agent/src/acp.rs` | M |

### 5.2 Package Ecosystem (5-Tier SPI)

Five tiers of extensibility with progressive capability and isolation:

| Tier | What | Isolation | Marketplace |
|---|---|---|---|
| 1: Composition | TOML-only Graphs, Racks, Profiles | None (pure composition) | Public |
| 2: WASM | Compiled WASM Cells | WASM sandbox (fuel + memory) | Public |
| 3: Script | Python/JS/shell Cells | Process isolation + path restriction | Verified publishers |
| 4: Extension | Rust Extension interceptors (22 hooks, 8 layers) | Code review + CaMeL IFC | Private only |
| 5: Rust | Compiled Rust Cells | Full trust | Local only |

| Task | Target | Size |
|---|---|---|
| Define Cell manifest format (TOML) | `crates/roko-core/src/manifest.rs` | M |
| Implement local Cell registry | `crates/roko-orchestrator/src/registry.rs` | M |
| Implement `roko marketplace publish/install/fork` | `crates/roko-cli/src/marketplace.rs` | L |
| Implement WASM sandbox (wasmtime, fuel metering, memory limits) | `crates/roko-orchestrator/src/wasm.rs` | L |
| Marketplace HTTP routes | `crates/roko-serve/src/routes/marketplace.rs` | M |

### 5.3 Marketplace Economics

| Revenue Band | Take-Rate | Precedent |
|---|---|---|
| First $1M lifetime creator revenue | **0%** | Shopify ($0 until you succeed) |
| Above $1M lifetime | **12-15%** | Unreal Engine (5% after $1M) |

The 0% band is per-creator, lifetime, non-retroactive. This directly addresses
the GPT Store failure mode (opaque rev-share, median creator earnings <$100/quarter).
All metrics mandatory and public: installs, active runs, fork count, gate pass rates,
mean cost per run, error rate.

### 5.4 Arena Framework

8 concrete arenas for measuring and proving capability:

| Arena | What It Measures | Benchmark |
|---|---|---|
| SWE-bench Pro | Code generation quality | Princeton-HAL dual-axis Pareto |
| Terminal-Bench | Terminal interaction capability | Side-by-side cost vs accuracy |
| Composability Arena | TTFW (time-to-first-workflow) vs LangGraph/CrewAI | Timed + pass^k reliability |
| c-factor Arena | Collective intelligence (>5 tasks, first-factor >40%) | Woolley methodology |
| Cost Arena | $/task at equal accuracy vs naive baselines | Dual-axis Pareto |
| Self-Improvement Arena | Voyager tech-tree + ablation | Skill growth + ablation collapse |
| Security Arena | Attack/defense against prompt injection, supply chain | Red-team/blue-team |
| Meta-Arena | Roko developing itself | Self-hosting loop throughput + quality |

| Task | Target | Size |
|---|---|---|
| Define Arena types and registry | `crates/roko-chain/src/arena.rs` | L |
| Implement eval harness with reproducible seeds | `crates/roko-gate/src/arena.rs` | L |
| Implement `roko arena run/submit/leaderboard` CLI | `crates/roko-cli/src/arena.rs` | L |

### 5.5 Brain Export/Import

Portable agent knowledge via Merkle-CRDT merge. An agent's knowledge store
is exportable (~100KB-1MB) and importable by another agent.

| Task | Target | Size |
|---|---|---|
| Implement Merkle-CRDT merge for knowledge store | `crates/roko-neuro/src/export.rs` | L |
| Implement `roko knowledge export/import` CLI | `crates/roko-cli/src/knowledge.rs` | M |
| Wire export format into `roko knowledge sync` | `crates/roko-neuro/src/sync.rs` | M |

---

## 6. Phase 4: Self-Evolution (Weeks 24-36)

**Goal**: Enable L4 self-evolution with safety guarantees.

### 6.1 L4 Self-Evolution

The most ambitious and most guarded capability. L4 modifies the system's own
structure, subject to human approval, c-factor gating, and Variance Inequality.

| Sub-component | What | Size |
|---|---|---|
| HGM Clade-Metaproductivity | Score variants by descendant performance, not just direct | L |
| CycleQD with HDC characterizations | Quality-Diversity search over system configurations | L |
| Verify-as-reward | Continuous `Verdict.reward` as fitness signal for evolutionary archive | M |
| c-factor measurement | PID-controlled collective intelligence as runtime observable | L |
| RecursiveSafetyMonitor | React-protocol Cell that monitors other Verify Cells | L |
| Approval workflow | Structural proposals -> safety check -> c-factor gate -> human review -> rollback on regression | L |

**Dependency**: All of Phase 1 and Phase 2. L4 requires Hot Graphs, EFE routing,
CognitiveWorkspace, and the Verify redesign.

---

## 6b. Phase 4+: Nunchi Blockchain (Weeks 36+)

**Goal**: Deploy the Nunchi blockchain -- a purpose-built EVM chain for AI agent
coordination -- anchoring identity, reputation, knowledge, and marketplace
economics on-chain. This phase executes after the core protocol is stable.

(Historical note: earlier documentation may reference "Korai" or "Daeji."
"Nunchi" is the canonical name for both the project and the blockchain.)

### 6b.1 Nunchi Testnet Deployment

Stand up the Nunchi testnet as a sovereign EVM L1 with Simplex consensus,
co-located Tokyo validators, and 400ms block time.

| Task | Target | Size |
|---|---|---|
| Configure sovereign EVM genesis with NUNCHI gas token | `contracts/deploy/` | L |
| Deploy initial validator set (co-located) | Infra | M |
| Validate 400ms block production | Integration tests | M |
| Set up block explorer and faucet | Infra | M |

### 6b.2 HDC Precompile

The core chain innovation: native 10,240-bit HDC vector operations at near-
native speed via the chain's HDC native precompile.

| Task | Target | Size |
|---|---|---|
| Implement `hdc_similarity` (pairwise, ~50 gas) | `contracts/precompile/hdc/` | M |
| Implement `hdc_topk` (K-nearest, ~400 gas target) | Same | L |
| Implement `hdc_bind` (XOR, ~30 gas) and `hdc_bundle` (majority, ~30+5N gas) | Same | M |
| Three-tier search architecture (Bloom fast reject, approximate coarse, exact top-K) | Same | L |
| Gas benchmarking: validate ~400 gas for top-K=20 against 1000 vectors | Benchmark suite | M |

### 6b.3 Six Solidity Contracts

| # | Contract | Purpose | Size |
|---|---|---|---|
| 1 | **AgentIdentity** (ERC-721, ERC-8004) | Transferable agent identity with capabilities, domain stakes, reputation tracks, prompt hash (on-chain tamper detection), tier classification, slash history | L |
| 2 | **ReputationRegistry** (ERC-8004 at 0xA200) | 7-domain EMA reputation with adaptive alpha, 30-day half-life decay, four discipline states | L |
| 3 | **ValidationRegistry** (ERC-8004 at 0xA300) | Gate verdicts, evidence hashes, dispute resolution records | M |
| 4 | **JobMarketplace** (ERC-8183) | Job posting, three hiring models (RandomVRF, Vickrey auction, DirectHire), escrow, lifecycle management | L |
| 5 | **KnowledgeLedger** | HDC-encoded knowledge entries with novelty-weighted posting rewards, duplicate detection, confirmation mechanics | L |
| 6 | **NunchiToken** (ERC-20 + demurrage) | 1% annual decay via lazy per-block computation, five earning mechanisms, spending for anti-spam | L |

### 6b.4 mirage-rs as Development Environment

mirage-rs (the in-process EVM simulator) already exists with 141 tests. Extend
it to emulate all Nunchi-specific features for local development.

| Task | Target | Size |
|---|---|---|
| Add `korai_*` RPC methods to mirage-rs (register, query, submit, heartbeat) | `apps/mirage-rs/` | M |
| Emulate HDC precompile in-process | Same | M |
| Add demurrage simulation (accelerated decay for testing) | Same | S |
| Validate API parity: mirage-rs vs real Nunchi testnet | Integration tests | M |

### 6b.5 ChainWitness Pipeline

Wire the chain intelligence pipeline that turns on-chain activity into
ordinary Bus Pulses for agent consumption.

| Task | Target | Size |
|---|---|---|
| Implement `ChainSubstrate` (Store trait for on-chain Signals) | `crates/roko-chain/src/substrate.rs` | L |
| Implement `ChainBus` (chain logs -> typed Pulses on Bus topics) | `crates/roko-chain/src/bus.rs` | L |
| Implement ChainWitness with Binary Fuse filter pre-screening | `crates/roko-chain/src/witness.rs` | L |
| Wire into orchestrate.rs so chain agents use standard cognitive loop | `crates/roko-cli/src/orchestrate.rs` | M |

### 6b.6 Remaining Chain Infrastructure

| Task | Target | Size |
|---|---|---|
| ERC-8004 Identity Registry (0xA100) | `contracts/src/` | L |
| x402 micropayment flow for agent-to-agent transactions | `crates/roko-chain/src/x402.rs` | L |
| MSB-safe payment routing (through licensed stablecoin issuers) | `crates/roko-chain/src/compliance.rs` | L |
| Bionetta ZK-HDC (ZK proofs over HDC vectors) | `crates/roko-chain/src/zk_hdc.rs` | L |
| Valhalla privacy tiers (P0-P3) | `crates/roko-chain/src/privacy.rs` | L |
| ISFR collective price discovery + KKT clearing certificates | `crates/roko-chain/src/isfr.rs` | L |
| Multi-chain finality oracle (Ethereum, Nunchi, L2s) | `crates/roko-chain/src/finality.rs` | L |

### 6b.7 Phase 4+ Dependency Graph

```
Nunchi Testnet (6b.1)
    |
    |--> HDC Precompile (6b.2) [requires chain-native precompile deployment]
    |--> mirage-rs Extensions (6b.4) [parallel with testnet]
    |
    v
Six Contracts (6b.3) [requires testnet + HDC precompile]
    |
    |--> ChainWitness Pipeline (6b.5) [requires contracts deployed]
    |--> x402, ZK-HDC, Valhalla, ISFR (6b.6) [requires contracts]
```

### 6b.8 What Exists Today

- `roko-chain` crate: `ChainClient` trait, `ChainWallet` trait, `TxSimGate`,
  `WalletGate`, `MockChainClient`, `MockChainWallet` (52 tests passing)
- `mirage-rs`: In-process EVM simulator with fork mode, scenario engine,
  chain extensions, HTTP API, JSON-RPC server (141 tests passing)
- `contracts/broadcast/`: Foundry deployment artifacts (scaffold)

### 6b.9 Phase 4+ Success Criteria

- [ ] Nunchi testnet producing 400ms blocks with Simplex consensus
- [ ] HDC precompile benchmarked at ~400 gas for top-K=20
- [ ] All six contracts deployed and passing integration tests
- [ ] mirage-rs emulates all `korai_*` RPC methods with API parity
- [ ] ChainSubstrate and ChainBus wired into the cognitive loop
- [ ] At least one agent registers an ERC-8004 identity, posts knowledge, and completes
      an ERC-8183 job market job end-to-end on testnet

---

## 7. Cost-Reduction Proof

### 7.1 What to Measure

The credibility floor for cost claims in 2026 is **dual-axis cost-vs-accuracy
plotting on a public agent benchmark**. Princeton HAL (SWE-bench Verified Mini)
is the gold standard.

**Format**: "On SWE-bench Pro using mini-SWE-agent harness, roko resolves X% at
$Y/task vs baseline LangGraph at $Z/task -- Nx reduction at equal accuracy,
replicated 3x with seed variation."

### 7.2 How to Prove 10x

The 10-30x claim is defensible because the dominant cost in agent fleets is
structural, not algorithmic. The multiplication is:

| Lever | Reduction | Mechanism |
|---|---|---|
| **Prompt/KV-prefix caching** | 5x | Cached prefix at 0.10x input price (90% off). Break-even is 2 cache reads. ProjectDiscovery moved hit rate from 7% to 84% with a single refactor. |
| **Tier routing (EFE)** | 3x | T0 gating (pure Rust pattern matching) handles 80% of ticks at $0. T1 uses Sonnet/Haiku. T2 uses Opus only when EFE demands it. |
| **Gate-based waste elimination** | 2x | Pre-action `verify_pre()` vetoes doomed actions before spending tokens. Adaptive thresholds learn the pass/fail boundary. |

Stacked: 5x * 3x * 2x = 30x theoretical ceiling. Realistic deployment captures
roughly half multiplicatively, landing at 10-15x.

### 7.3 Vanity Metrics to Avoid

- Raw token counts without accuracy
- Cost reductions on saturated benchmarks (HumanEval, MMLU)
- Single-seed accuracy comparisons
- Vendor-provided benchmark numbers
- Semantic cache hit rates on non-FAQ workloads (production median is 30-50%, not the 86% from FAQ benchmarks)

### 7.4 Additional Proof Patterns

| Claim | Proof Method |
|---|---|
| Composability | TTFW (time-to-first-workflow) timed against LangGraph + CrewAI, plus pass^k reliability across topologies |
| Collective intelligence | Woolley c-factor battery: >=5 unrelated tasks, first-factor variance >40%, ratio to second factor >2x |
| Self-improvement | Voyager tech-tree + ablation: capability milestones + skill-library growth + ablation showing collapse when primitive removed |

---

## 8. Key Dependencies and Critical Path

### 8.1 What Cells What

```
Phase 0 (Launch Artifacts)
    |
    | spec + SDKs + demos -- these are independent of code changes
    |
    v
Phase 1 (Core Protocol)
    |
    |-- P1.1 Pulse -> P1.2 Bus -> P1.3 CalibrationPolicy -> P1.6 Hot Graph -> P1.7 Workflow/Activity
    |-- P1.4 Verify redesign  [parallel]
    |-- P1.5 Observe + Lenses [parallel]
    |
    v
Phase 2 (Differentiation)
    |
    |-- P4.1 EFE routing ---------> P4.8 Somatic markers
    |-- P4.2 Demurrage -> P4.3 Heuristic -> P4.4 CognitiveWorkspace
    |-- P4.5 Vitality -> P4.6 Type-state
    |-- P4.7 CorticalState [independent]
    |
    v
Phase 3 (Distribution)
    |
    |-- P5.1 ACP server [independent]
    |-- P5.2 Package ecosystem -> P5.3 Marketplace economics
    |-- P5.4 Arena framework [independent]
    |-- P5.5 Brain export [independent]
    |
    v
Phase 4 (Self-Evolution)
    |
    |-- P6.1 L4 self-evolution (depends on Phase 1 + Phase 2)
    |
    v
Phase 4+ (Nunchi Blockchain)
    |
    |-- Testnet -> HDC precompile -> Six contracts -> ChainWitness pipeline
    |-- x402, ZK-HDC, Valhalla, ISFR (after contracts deployed)
    |-- Multi-chain finality [independent]
```

### 8.2 Critical Path

The longest dependency chain:

```
P1.1 Pulse (1w)
  -> P1.2 Bus (2w)
    -> P1.3 CalibrationPolicy (2w)
      -> P1.6 Hot Graph (2w)
        -> P1.7 Workflow/Activity (2w)
          -> P4.4 CognitiveWorkspace (3w)
            -> P6.4 L4 Self-Evolution (6w)

Total critical path: ~18 weeks
```

### 8.3 What Can Be Parallelized

**Three independent tracks** can run from Phase 1 onward:

1. **Protocol track**: Pulse -> Bus -> CalibrationPolicy -> Hot Graph -> Workflow/Activity
2. **Verify track**: Verify redesign -> Observe + Lenses (both independent of Bus)
3. **Launch track**: Spec finalization, SDK development, demo integrations

From Phase 2 onward:

4. **Routing track**: EFE -> Somatic markers
5. **Knowledge track**: Demurrage -> Heuristic -> CognitiveWorkspace
6. **Agent track**: Vitality -> Type-state -> CorticalState

From Phase 3 onward:

7. **Distribution track**: ACP server, Arena framework, Brain export (all independent)
8. **Marketplace track**: Package ecosystem -> Marketplace economics

### 8.4 Time-Sensitive Dependencies

| Deadline | What | Impact |
|---|---|---|
| August 2, 2026 | EU AI Act high-risk enforcement | Must have: per-agent identity, tamper-evident logs, policy-as-code, kill switches |
| June 30, 2026 | Colorado AI Act effective | Must have: impact assessment capability, NIST AI RMF alignment |
| ~6-12 months | MCP + A2A + ERC-8004 lock-in | Must ship spec + SDKs + demos before standards solidify |
| Q3 2026 | LangGraph 1.0 GA + Microsoft Agent Framework 1.0 GA | Enterprise SDK choice is bimodal and hardening now |

### 8.5 One-Person vs Team Sequencing

If working solo, the priority stack is:

1. Phase 0 launch artifacts (highest leverage: sets the adoption curve)
2. P1.1-P1.2 (Pulse + Bus) -- enables everything else
3. P1.4 (Verify redesign) -- safety is load-bearing
4. P4.1 (EFE routing) -- the single biggest cost-reduction lever
5. P4.4 (CognitiveWorkspace) -- the single biggest quality lever
6. P5.2 (Package ecosystem) -- enables third-party contribution

If working with a team (3-4 engineers), run tracks 1-3 in parallel from day one,
add tracks 4-6 at week 8.

---

## 9. Summary

| Phase | Timeline | Deliverable | Success Metric |
|---|---|---|---|
| 0: Launch | Weeks 0-6 | Spec + 2 SDKs + 5 demos + 5 partners | First external integration running |
| 1: Core Protocol | Weeks 1-8 | Pulse, Bus, CalibrationPolicy, Verify, Observe, Hot Graph | predict-publish-correct running per-task |
| 2: Differentiation | Weeks 8-16 | EFE, Demurrage, Heuristic, CognitiveWorkspace, Vitality | 10x cost reduction provable on SWE-bench Pro |
| 3: Distribution | Weeks 16-24 | ACP, Marketplace, Arena, Brain export | First marketplace artifact published by third party |
| 4: Self-Evolution | Weeks 24-36 | L4, c-factor, RecursiveSafetyMonitor | System improves itself through verified feedback loops |
| 4+: Nunchi Blockchain | Weeks 36+ | Testnet, HDC precompile, 6 contracts, ChainWitness, x402, ZK-HDC | Agent registers ERC-8004 identity, posts knowledge, completes ERC-8183 job on testnet |

The binding constraint is not model capability -- it is coordination cost.
Every phase reduces the structural cost of agent coordination. The protocol wins
by making that cost an order of magnitude lower than the alternatives, with
verifiable proof, and by shipping the boring artifacts (spec, SDKs, demos,
conformance kits) that every successful protocol shipped before it.
