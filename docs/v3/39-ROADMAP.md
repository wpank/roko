# 39. Roadmap

> **Implementation status**: This chapter collects all aspirational, deferred, and
> future-facing content in one place. Nothing here is a claim of current functionality.
> For current implementation state, see [00-INDEX.md](00-INDEX.md) and `.roko/GAPS.md`.
>
> Last updated: 2026-09-15

---

## 1. Approved Design Upgrades

Four research-backed architectural upgrades have been evaluated against 130+ papers from
2025-2026 surveys and approved for implementation. Each has a specific target subsystem and
a clear delta from the current design.

### 1.1 GRASP Regression-Gated Playbook Admission

**Problem.** The playbook store admits new when/then entries without checking whether they
degrade existing trajectories. This enables silent performance regression as the store
grows.

**Upgrade.** Each candidate playbook entry is tested against a balanced held-out probe
under a hard regression budget. Only entries with net positive improvement are admitted.
The original GRASP paper reports +48 points on MedAgentBench (40.6% to 88.8%).

**Target.** `roko-learn` playbook enrichment path.

**Related work.**
- GRASP (arXiv:2605.29668, May 2026) -- regression-gated admission
- SiriuS (arXiv:2502.04780) -- augment failed episodes rather than discarding
- SkillZip (arXiv:2608.11079) -- MDL compression to prevent unbounded playbook growth

### 1.2 Schema Distillation in Dream Consolidation

**Problem.** The dream system currently reorganizes stored records during idle periods but
does not distill generalizable schemas from episodic traces. Three independent research
groups demonstrated that consolidation should extract transferable structure, not merely
replay.

**Upgrade.** The dream cycle gains three capabilities:
1. Extract generalizable schemas from raw episodic traces (not just replay)
2. Update persistent knowledge representations (not just reorganize)
3. Use CLS-inspired fast/slow separation (Auto-Dreamer pattern)

**Target.** `roko-dreams` consolidation cycle.

**Related work.**
- Auto-Dreamer (arXiv:2605.20616, May 2026) -- learned consolidator
- Language Models Need Sleep (arXiv:2606.03979, June 2026) -- parametric distillation
- Do LMs Need Sleep? (arXiv:2605.26099, May 2026) -- offline recurrence
- TiMem (arXiv:2601.02845) -- temporal hierarchical consolidation
- Phasor Agents (arXiv:2601.04362) -- oscillatory sleep-staged learning

### 1.3 PEEK Orientation Cache (10th SystemPromptBuilder Layer)

**Problem.** Agents lose orientation over long sessions. There is no persistent summary of
accumulated knowledge in the prompt. The current 9-layer SystemPromptBuilder has no
mechanism for maintaining a running knowledge map.

**Upgrade.** Add a 10th layer to the SystemPromptBuilder -- a constant-token orientation
cache maintained by three modules:
- **Distiller**: extracts transferable knowledge from inference signals
- **Cartographer**: translates knowledge into structured map edits
- **Evictor**: enforces a fixed token budget via priority eviction

The PEEK paper reports 93-145 fewer iterations and 1.7-5.8x lower cost than ACE.

**Target.** `roko-compose` SystemPromptBuilder.

**Related work.**
- PEEK (arXiv:2605.19932, May 2026) -- orientation cache
- VISTA (arXiv:2606.30005) -- proprioceptive context dashboard
- ECS (arXiv:2601.11585) -- entropic context shaping
- Scroll (arXiv:2608.21690) -- context as executable environment

### 1.4 AgentPRM Continuous Gate Progress Signals

**Problem.** The 19 gates produce binary pass/fail verdicts. This makes it impossible to
distinguish "almost passed" from "completely failed", limiting the quality of replan
decisions.

**Upgrade.** Gates produce continuous progress signals alongside binary verdicts, using
Temporal Difference estimation and Generalized Advantage Estimation. This enables:
- Partial-success replanning (not just "try again")
- 8x compute efficiency for verification
- Test-time compute scaling

**Target.** `roko-gate` pipeline and `gate_dispatch.rs`.

**Related work.**
- AgentPRM (arXiv:2511.08325, Nov 2025; WWW 2026) -- process reward models
- Messier (arXiv:2607.25891) -- partial-pass scoring rationale
- PACE (arXiv:2607.02032) -- proxy capability evaluation

---

## 2. Active Backlog

Items with specifications and clear acceptance criteria. The full specced backlog is at
`tmp/backlog/00-INDEX.md` and `tmp/CONSOLIDATED-BACKLOG.md`. The numbers below are backlog
item IDs.

### 2.1 P0 -- Critical

| # | Item | Size | Notes |
|---|---|---|---|
| 17 | ACP stability hardening | L | 7 crash-path panics fixed (commit `815d96c90`). Broader P1 silent-failure and race items in the spec remain open |

### 2.2 P1 -- High

| # | Item | Size |
|---|---|---|
| 03 | Context injection scoping (per-role context sizing) | M |
| 04 | Compile auto-fix path (cargo fix before agent retry) | S |
| 18 | ACP spec upgrade v0.12 to v0.13 + bridge_events refactor | XL |
| 45 | ACP tool permission gate (plugin tier + command ceiling) | M |
| 56 | ACP single-agent chat: client capability declaration | M |
| 60 | Safety dispatch hardening (contract fail-open, optional SafetyLayer) | M |

### 2.3 P2 -- Medium (Selected)

| # | Item | Size |
|---|---|---|
| 20 | Event loop decomposition (23K-line god object) | XL |
| 01 | T0 reflex store runtime loop | M |
| 02 | Reactive agent mode (trigger-based wake/sleep) | L |
| 05 | Express mode (skip strategist for trivial fixes) | M |
| 15 | Post-gate reflection (real LLM call, not deterministic) | M |
| 37 | Multi-process locking (.roko/ concurrent writer safety) | S |
| 39 | ACP learning-pipeline parity (experiment receipts) | M |
| 47 | ConfigLayer elimination (~1,500 LOC legacy dual-loader) | L |
| 53 | Immune system adaptive screening (memory + provider visibility) | L |
| 54 | Graph Engine Runner-v2 parity (gates/replan/worktree/merge) | XL |
| 55 | AgentPool runtime integration (pool dispatch + warm reuse) | M |
| 61 | Agent dispatch consolidation (5 paths, safety/enrichment divergence) | XL |

### 2.4 Runner-v2 Removal

Runner-v2 is retained as `--engine legacy` for one release cycle after Graph became the
sole engine (#260/#276, 2026-09-05). The decision to remove it has been made. Implementation
should be combined with event_loop.rs decomposition (backlog #20) to extract reusable
subsystems (gate dispatch, persist, snapshot writer, merge, branch cleanup) before
deleting the legacy code path. The remaining ~23K lines of `event_loop.rs` become a thin
orchestrator over extracted modules.

---

## 3. Code Quality

### 3.1 Duplicate Type Families (~14)

Near-duplicate types across crates create serialization mismatches and maintenance burden.
Consolidation should happen opportunistically as each subsystem is documented for v3:

- DashboardSnapshot / DashboardData (two data models during plan run)
- StateHub types (multiple definitions)
- AgentState variants
- TaskStatus enums
- GateFeedback types
- EventBus implementations (47 distinct event enums, 4 EventBus structs, 2 bus trait systems)
- Cell/Node types
- Plan/Workflow types

Five families are already resolved (DashboardSnapshot, StateHub, GateVerdict,
RetentionPolicy, Engram). The remaining ~14 require case-by-case consolidation or explicit
semantic separation.

### 3.2 Dead Code Cleanup

| Location | LOC | Status |
|----------|-----|--------|
| `roko-acp/src/runner.rs` staging code | 460 | Confirmed dead (DC-08). Remove |
| `roko-serve` blanket `#![allow(dead_code)]` | unknown | Remove suppression, fix exposed dead code |
| `roko-conductor/src/federation.rs` | 376 | Already removed |
| `roko-std` ~52 todo/stub matches | -- | Verify if legitimate fallbacks |
| `roko-dreams` ~31 stub matches | -- | Verify if legitimate deferred |
| `roko explain` 12 stale entries | -- | Update to reference current code locations |

### 3.3 Half-Implemented Features

All eight have scaffolding/types but lack final wiring:

| Feature | What is Missing |
|---------|-----------------|
| T0 Reflex Store | Runtime sleep/wake loop, trigger registration |
| Compile Auto-Fix | Final wiring to agent dispatch |
| Express Mode | Final wiring |
| Post-Gate Reflection | Real LLM call (currently deterministic) |
| Warm Agent Spawning | Real pre-spawned processes (currently placeholders) |
| Multi-Process Locking | Shared/read-only locks |
| HDC Prompt Assembly | Propagate feature flag to downstream consumers |
| MCP Protocol Version | Align server and client versions |

### 3.4 CLI Surface Fixes

| Item | What |
|------|------|
| `--json` flag | Silently ignored on ~15+ commands. Implement JSON output or remove the flag |
| `--role` flag | Hardcoded in research/PRD commands. Make it use the specified role |
| `config set --global` | Flag exists but is silently ignored |
| `roko inject` | Stub that accepts args but never sends signals. Wire or remove |
| `roko new` | 6/9 scaffold types generate non-compiling code (stale Signal rename) |
| `roko explain` | 12 entries reference deleted `orchestrate.rs` |
| `status` daemon check | Code exists but is unreachable |

### 3.5 Config Issues

| Issue | Fix |
|-------|-----|
| `[github]` in init template not recognized | Add to config schema (done) |
| `[resources]` in init template not recognized | Add to config schema (done) |
| Budget enforcement zero defaults | Set sensible non-zero defaults or document the zero-default |

---

## 4. Chain Deprecation

The blockchain coupling in `roko-chain` (29,683 LOC, 37 files) is being deprecated.
Good algorithms are extracted to server-backed crates; blockchain infrastructure is
removed. Full plan at `tmp/docs-audit/09-CHAIN-DEPRECATION-PLAN.md`.

### 4.1 Phase 1: Extract Algorithms (~8 modules)

Strip all blockchain types (Address to String/Uuid, u256 to u64, block_number to
timestamp) and move pure algorithms to appropriate crates:

| Module | Destination | What is Preserved |
|--------|-------------|-------------------|
| `reputation_registry.rs` (~1,600 LOC) | `roko-core` | 7-domain EMA scoring, 30-day half-life decay, 4 discipline states, slash rates, recovery paths, pricing tiers |
| `arena.rs` + `arena_flywheel.rs` (~2,800 LOC) | `roko-core` types, `roko-serve` routes | Lifecycle, attempts, leaderboards, scoring, prizes, event outbox, cooldown/deadline enforcement |
| `agent_registry.rs` (~1,800 LOC) | `roko-core` | 10 capability bits, passport tiers, delegation with caveats, prompt hash commitment |
| `trace_rank.rs` (~150 LOC) | `roko-learn` | PageRank power iteration, fork attribution, blend weight with EMA reputation |
| `collusion.rs` (~200 LOC) | `roko-learn` | Clique detection, mutual-ratio flagging, reputation feedback dilution |
| `knowledge_registry.rs` (~900 LOC) | `roko-core` types, `roko-serve` routes | Publish/validate/challenge lifecycle, HDC fingerprint discovery, staleness auto-transition |
| `validation_registry.rs` (~150 LOC) | `roko-gate` or `roko-core` | Gate-based proof acceptance, duplicate rejection, accept/reject tallying |
| `identity_economy_identity.rs` (PPR/Sybil) | `roko-learn` | PersonalizedPageRank, SybilRankDetector, collusion ring detection |

### 4.2 Phase 2: Remove Blockchain Infrastructure

- Remove `chain` feature from default compilation in `roko-cli`, `roko-serve`, `roko-agent-server`
- Delete ~21 chain-specific source files (chain handlers, chain routes, DeFi routes, EVM tools)
- Remove chain config (`ChainConfig`, `FeedAgentsConfig`, `chain_rpc`)
- Remove alloy/revm/k256 from workspace dependencies
- Exclude `mirage-rs` from default workspace builds
- Remove 17 chain-domain tools from the tool registry
- Remove Korai token, x402, ERC-8004 references

### 4.3 Phase 3: Update Documentation

- CLAUDE.md: mark `roko-chain` deprecated, update route/crate counts
- GAPS.md: close E38-E41 Phase 2+ items as deprecated
- v3 docs: chapter 37 becomes "Shared Economy" (extracted algorithms only, no blockchain)

**Estimated effort.** Phase 1 (extract): L (2-3 sessions). Phase 2 (remove): M (1-2
sessions). Phase 3 (docs): S (1 session).

---

## 5. Research Frontiers

Research directions identified during the 2025-2026 paper surveys. These are not approved
upgrades (Section 1) but are tracked as aspirational targets that may influence future
design.

### 5.1 Latent-Space Agent Communication

Replace natural-language inter-agent messages with latent-space representations. LatentMAS
(arXiv:2511.20639, ICML 2026 Spotlight) demonstrates 4x speed improvement and 14.6%
accuracy gain. Relevant to roko's pheromone communication in agent groups. The current Bus
and group message systems use text; latent encodings could reduce bandwidth and improve
coordination fidelity.

### 5.2 Federated Skill Sharing

Share learned agent skills across workspaces without sharing raw data. FederatedSkill
(arXiv:2606.03143) uses semantic skill diffs for federated learning with 44.4%
improvement. Relevant to cross-workspace knowledge transfer via the existing relay and
group systems. Would require extending the current knowledge sync protocol with
differential skill representations.

### 5.3 Evolvable Memory Pipelines

Let the memory system evolve its own structure rather than using fixed tiers. MemPro
(arXiv:2606.00619) proposes evolvable memory pipelines where the system discovers optimal
memory organization through experience. This is an aspirational target for `roko-neuro`
self-improvement, complementing the existing fixed Transient/Working/Consolidated/Persistent
tier hierarchy.

Additional memory research to track:
- FluxMem (arXiv:2602.14038) -- adaptive memory structure selection via Beta Mixture Model
- Memory Worth (arXiv:2604.12007) -- outcome-grounded forgetting as principled alternative
  to time-based decay
- FadeMem (arXiv:2601.18642) -- empirical validation of Ebbinghaus-based decay (supports
  roko-neuro's existing approach)

### 5.4 Harness Auto-Evolution

Automate optimization of the agent harness itself. Two independent lines of work:
- HarnessX / Meta-Harness (arXiv:2603.28052) -- +7.7 points on text classification from
  harness optimization alone at 4x fewer tokens
- AHE (Autonomous Harness Evolution) -- self-modifying agent scaffolding

These validate roko's scaffold thesis and suggest the harness optimization loop (currently
manual via PRDs and plans) could eventually become automated.

### 5.5 eBPF Kernel Enforcement

Use eBPF programs for kernel-level agent safety enforcement. ActPlane (arXiv:2606.25189)
demonstrates deployable substrate for tool cooldown and isolation. This would complement
roko's current five-level sandbox policy with OS-kernel enforcement, providing a hardware-
backed guarantee layer beneath the existing software-based safety checks.

### 5.6 Capture-Checking Safety

Apply capture checking (as in Scala 3's capture calculus) to agent safety.
"Tracking Capabilities" (arXiv:2603.00991, Best Paper ACM CAIS 2026) proposes type-system
enforcement of capability boundaries. This complements roko's existing runtime capability
system with static analysis guarantees, potentially catching capability violations at
compile time rather than runtime.

### 5.7 Additional Research Pointers

| Topic | Paper | Relevance |
|-------|-------|-----------|
| Semantic taint tracking | NeuroTaint (arXiv:2604.23374) | Extension to roko's classical IFC model |
| SMT behavioral specs | VIGIL (arXiv:2606.26524) | Complement to gate pipeline |
| Coordination failure analysis | arXiv:2605.03310 | 41-87% of failures are coordination, not capability |
| Terminal output compression | TACO (arXiv:2604.19572) | Self-evolving compression for gate output |
| Prompt interference detection | Arbiter (arXiv:2603.08993) | Prompt quality assurance for SystemPromptBuilder |

---

## 6. Nous Research Integration

A research and integration study covering the Nous Research ecosystem is maintained at
`tmp/nous-research/`. It contains:

- **Reference material**: 93-repo ecosystem catalog, model family tree, Hermes tool-calling
  format, Psyche decentralized training architecture, Portal API and x402 payment flow
- **Roko integrations**: Hermes provider wiring (backlog #172), 10 battle-tested agent
  patterns, training data export pipelines (gate to Atropos, dream to corpus, immune to
  classifier), federated routing via Iroh P2P, knowledge-to-LoRA distillation, HDC
  deduplication oracle for Psyche
- **Standalone projects**: 14 ranked by impact and feasibility. Top 3: hermes-rs Rust SDK
  (4-6 weeks), ratchet-ci quality gate tool (2-4 weeks), gate-eval-harness standalone
  evaluator (4-6 weeks)
- **Evals deep dive**: 13 documents covering Atropos RLVR, benchmark landscape, judge
  calibration, failure analysis, eval statistics, and benchmark design

The Hermes provider is already a supported provider kind in `roko-agent`. Deeper integration
(training data export, federated routing, arena tournaments) is tracked as backlog items
#172-#177.

---

## 7. Deferred Product Work

These items are recognized product gaps where the contract or backend is complete but the
user-facing runtime is not. They are tracked in `.roko/GAPS.md` and are not blocked by
design decisions.

### 7.1 Full Named-Surface TUI Rendering

E37 (Named Surfaces) is 9/9 against its contract/backend manifest. Five typed projections
(Workbench, Inbox, Canvas, Minimap, Autonomy), five StateHub-backed HTTP routes, and the
legacy-tab mapping are implemented. What remains:
- Legacy TUI does not render every named surface
- SurfaceEvents do not enter a command path
- No production Inbox publisher or action consumer is wired
- `pending_human` has no live source
- Autonomy config has no live store
- Canvas uses dashboard plan IDs
- Minimap uses deterministic layout rather than HDC coordinates

### 7.2 Native Agent-to-E33 Telemetry

E33 (Telemetry Lens) is runtime-complete at the ingress boundary with all 39 protocol
variants having production evidence. E23 (Cognitive Autonomy) is 10/10. The remaining gap:
native Agent owners do not publish the E33 observation payload directly. The observation
server validates canonical regimes, legal lifecycle transitions, monotonic vitality, and
transport sequences -- agents need to emit these observations through their lifecycle
rather than relying solely on external instrumentation.

### 7.3 WIT/Component WASM Hostcalls

E32 (Tool/Plugin Ecosystem) is 8/8 against its manifest. Signed dependency graphs, bounded
WASM hooks, strict admission, verified relay/install, and current CLI/MCP targets are live.
What remains:
- WIT/Component-model Store and Bus hostcalls for WASM plugins
- OpenClaw and legacy one-shot parity

### 7.4 Additional Relay Transports

E29 (Connectivity/Relay) has one supervised HTTP JSON adapter with bounded health/reconnect
and durable exact-room subscription execution. What remains:
- Additional transport backends beyond HTTP JSON
- Startup discovery protocol
- MCP/A2A/x402 execution integration
- Finality and reorg processing
- Dashboard integration for relay status

### 7.5 Fresh Dogfood Proof

The first dogfood run (2026-08-13) exposed four serial blockers (config merge, stale
snapshot, fsmonitor, scheduler deadlock). All four have regression fixes. A pre-execution
proof (2026-09-15) verified the full pipeline with a prebuilt binary. The full live
agent-dispatch rerun of the complete self-hosting workflow has not been performed; it is
blocked by a dirty-tree build failure from the `SnapshotRebased` arm gap in
`output_sink.rs`. Proof documentation is at `tmp/refactoring-audit/P1-06-DOGFOOD-PROOF.md`.

### 7.6 Serve Experiment Parity

Runner-v2 has full durable prompt-experiment lifecycle (assignment, dispatch, settlement).
ACP now has canonical section replacement (commit `37f86fa81`). Serve still injects
ephemeral context instead of using canonical section replacement under durable receipts.
A three-phase fix is designed at `tmp/refactoring-audit/P1-07-EXPERIMENT-PARITY.md` but not
yet implemented.

### 7.7 Provider-Internal Security Visibility

E34 (Security/IFC) is 8/8 strict. The immune Graph screens canonical provider primary
outputs and all host-visible tool results. The remaining gap is visibility into provider-
owned internal calls/results, provider trace Signals, broad semantic/adaptive immune memory,
and externally anchored whole-ledger authenticity. These are outside the primary-output
boundary by design.

### 7.8 Chain/Economy Runtime Integration

Local state machines for marketplace (E38), registries (E39), arenas (E40), and DeFi (E41)
are implemented as tested contract/stub tranches. Production boundaries remain for each:
- Durable storage and search for artifact marketplace
- Deployed contracts, gossip transport, and ABI decoding for registries
- Eval orchestration, the seven-stage flywheel, and on-chain settlement for arenas
- Risk engine, venue execution, and durable adapters for DeFi

These are tracked under the chain deprecation plan (Section 4). Algorithms will be
extracted; blockchain coupling will be removed. Server-backed versions of reputation,
arenas, and identity will continue.

### 7.9 Recursive Safety at Scale

R04 provides bounded meta-agent lineage with non-widening authority and single-use
artifact-bound acceptance. What remains as product work:
- Loop 4 structural adaptation
- ADAS/HGM-style autonomous generation of new agents
- Autonomous execution of generated agent specifications
- Continuous recursive-safety wrapping for every Flow

### 7.10 Additional Deferred Items

| Item | Current State | What Remains |
|------|---------------|--------------|
| Graph approval channel | `--approval` fails before workspace lock | Interactive approval protocol for Graph engine |
| AgentPool runtime instantiation | Pool/multi-pool management built, TUI modal exists | No runtime instantiation in runner |
| Bus-reactive dream scheduling | Cron/idle/episode-count triggers live | Bus-reactive and intensive-backlog controls |
| Event system unification | 47 event enums, 4 EventBus structs | Converge to single event system (four-phase migration plan at `tmp/refactoring-audit/P1-10-EVENTBUS-AUDIT.md`) |
| StateHub cursor atomicity | `SnapshotRebased` event added | Cursor-atomic SSE typed capture, single immutable resume generation |
| TUI disk I/O elimination | 3 functions read files every frame | Cache file reads, refresh only on change |

---

## 8. Crate Lifecycle

Two workspace crates are scheduled for removal and one for exclusion:

| Crate | LOC | Decision | Reason |
|-------|-----|----------|--------|
| `roko-mcp-slack` | 1,948 | Remove | Never wired |
| `roko-mcp-scripts` | 766 | Remove | Never wired, duplicates bash/shell tools |
| `roko-demo` | 5,838 | Exclude from default builds | Zero callers, depends on alloy |

`roko-chain` lifecycle is covered in Section 4.

---

## Cross-References

| Topic | Canonical Location |
|-------|-------------------|
| Current implementation status | `.roko/GAPS.md` |
| Full specced backlog | `tmp/backlog/00-INDEX.md` |
| Consolidated backlog | `tmp/CONSOLIDATED-BACKLOG.md` |
| TUI parity remaining items | `tmp/tui-parity/` |
| Chain deprecation plan | `tmp/docs-audit/09-CHAIN-DEPRECATION-PLAN.md` |
| Research design upgrades | `tmp/docs-audit/11-RESEARCH-DESIGN-UPGRADES.md` |
| Tech debt registry | `tmp/docs-audit/07-TECH-DEBT.md` |
| Nous research integration | `tmp/nous-research/INDEX.md` |
| EventBus migration plan | `tmp/refactoring-audit/P1-10-EVENTBUS-AUDIT.md` |
| Experiment parity design | `tmp/refactoring-audit/P1-07-EXPERIMENT-PARITY.md` |
| Dogfood proof | `tmp/refactoring-audit/P1-06-DOGFOOD-PROOF.md` |
