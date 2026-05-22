# Phase 3 — Autonomy, Safety, and Economy

> Full autonomous operation with CaMeL IFC, 5-head corrigibility, on-chain anchoring, arenas, brain export, and cross-agent knowledge sharing.

**Spec source**: `tmp/unified/21-ROADMAP.md` §4 (Phase 3)
**Dependencies**: Phase 2 complete (some items can start in parallel — noted below)
**Depth docs required**: Items marked **[BLOCKED:depth]** need `tmp/unified-depth/` docs written first

---

## 3.1 Extension System + CaMeL IFC

- [ ] **Formalize 8 Extension layers** — Define the 8 interception layers: L0 Foundation (PreExecute, PostExecute, OnError), L1 Perception (PreObserve, PostObserve), L2 Memory (PreRetrieve, PostRetrieve, PreStore, PostStore), L3 Cognition (PreReason, PostReason, PreCompose, PostCompose), L4 Action (PreExecuteTool, PostExecuteTool, OnToolError), L5 Social (PreCommunicate, PostCommunicate), L6 Meta (PreReflect, PostReflect), L7 Recovery (OnPanic, OnBudgetExhausted). Each hook receives data flow and returns modified data flow. **Verify**: an Extension in L4 can intercept a tool call, log it, and pass through unchanged.
  - Spec: `tmp/unified/08-EXTENSION-SYSTEM.md` §4-5
  - Code: `crates/roko-agent/src/extensions/` (refactor existing)

- [ ] **Define `CamelTag` types** — `CamelTag { capabilities: Capabilities, provenance: Vec<CellId>, taint_level: TaintLevel }`. `TaintLevel` enum: `Trusted`, `Local`, `External`, `Untrusted`. Tags attach to Signals flowing through Extensions. **Verify**: a Signal tagged `Untrusted` retains that tag through all Extension layers.
  - Spec: `tmp/unified/08-EXTENSION-SYSTEM.md` §3 (CaMeL IFC), `tmp/unified/17-SECURITY-MODEL.md` §3
  - Code: `crates/roko-core/src/camel.rs` (new)

- [ ] **Implement tag propagation** — Rule 1: input tags propagate to outputs. Rule 2: Extensions cannot elevate taint (Untrusted → Trusted is forbidden). Rule 3: decision enums (Route selection, Verify verdict) carry the tag of the data that influenced them. Rule 4: full audit trail of tag transitions as Signals. **Verify**: attempt to elevate Untrusted data to Trusted within an Extension → rejected with audit Signal emitted.
  - Spec: `tmp/unified/08-EXTENSION-SYSTEM.md` §3.2 (Propagation Rules)
  - Code: `crates/roko-agent/src/extensions/dispatch.rs`

- [ ] **Implement CaMeL Monitor (Verify Cell)** — A Verify Cell that checks CamelTag invariants on every Extension dispatch. Flags violations. Runs outside the modifiable surface (agent cannot modify its own CaMeL monitor). **Verify**: inject a tag violation, confirm CaMeL monitor catches it and emits alert Pulse.
  - Spec: `tmp/unified/17-SECURITY-MODEL.md` §3.3
  - Code: `crates/roko-gate/src/camel_monitor.rs` (new)

## 3.2 5-Head Corrigibility

- [ ] **Implement 5-head Verify chain** — Lexicographic safety ordering: Deference > Switch > Truth > Impact > Task. Each "head" is a Verify Cell that can veto. The chain runs sequentially — if any head vetoes, the action is blocked regardless of what lower heads say. Implemented as a Graph of 5 Verify Cells in series. **Verify**: a Task-optimal action that violates Deference is blocked. A Truth-optimal action that violates Switch is blocked.
  - Spec: `tmp/unified/17-SECURITY-MODEL.md` §4 (5-Head Corrigibility)
  - Code: `crates/roko-gate/src/corrigibility.rs` (new)

- [ ] **Implement RecursiveSafetyMonitor** — Ensures the safety pipeline itself cannot be bypassed. Monitors Extension loading, Verify pipeline configuration, and capability grants for self-referential attacks (agent modifying its own safety checks). **Verify**: attempt to remove a corrigibility head at runtime → rejected.
  - Spec: `tmp/unified/17-SECURITY-MODEL.md` §5
  - Code: `crates/roko-gate/src/recursive_safety.rs` (new)

- [ ] **Wire corrigibility into Graph executor** — The 5-head chain wraps every Cell execution as a mandatory pre/post check. Cannot be removed by Graph authors. **Verify**: every Cell execution in a Graph passes through corrigibility check.
  - Spec: `tmp/unified/17-SECURITY-MODEL.md` §4.3
  - Code: `crates/roko-orchestrator/src/graph/safety.rs` (new)

## 3.3 Learning Loop 4 (Structural Self-Evolution)

- [ ] **Define structural change proposals** — `StructuralProposal { id, kind: ProposalKind, description, diff, evidence: Vec<SignalRef>, author: CellId }`. `ProposalKind`: `ModifyGraph`, `AddCell`, `RemoveCell`, `ChangeConfig`, `UpdateVerifyPipeline`. Proposals are Signals published on Bus. **Verify**: a StructuralProposal serializes/deserializes correctly.
  - Spec: `tmp/unified/10-LEARNING-LOOPS.md` §5 (Loop 4)
  - Code: `crates/roko-learn/src/structural.rs` (new)

- [ ] **Implement approval workflow via Agent Inbox** — L4 proposals appear in Agent Inbox as Urgent notifications. Human reviews evidence, approves or rejects. Approved proposals are applied to the workspace. Rejected proposals are archived with rejection reason. **Verify**: L4 proposal appears in Inbox, approval applies the change, rejection archives it.
  - Spec: `tmp/unified/10-LEARNING-LOOPS.md` §5.2
  - Code: `crates/roko-serve/src/routes/approvals.rs` (new)

- [ ] **Wire L4 into dream cycle** — During the dream Integration phase, the system reviews episode patterns and proposes structural improvements. Proposals go through approval workflow. **Verify**: after dream cycle, if episodes show recurring pattern, a structural proposal is generated.
  - Spec: `tmp/unified/10-LEARNING-LOOPS.md` §5.3
  - Code: `crates/roko-dreams/src/structural.rs` (new)

- [ ] **Variance Inequality enforcement** — L4 pauses when generator improves faster than verifier. Measured by comparing generator's predict-publish-correct improvement rate against verifier's accuracy. If verifier is not spectrally cleaner, L4 proposals are held. **Verify**: artificially degrade verifier accuracy, confirm L4 proposals stop.
  - Spec: `tmp/unified/10-LEARNING-LOOPS.md` §5.4
  - Code: `crates/roko-learn/src/structural.rs`

## 3.4 On-Chain Registry Deployment

**[BLOCKED:depth]** — Depends on `tmp/unified-depth/18-registries/` depth docs.

- [ ] **Finalize Solidity contracts** — AgentPassport (ERC-8004), ReputationRegistry (per-domain EMA), InsightStore (knowledge Signal publication), PheromoneRegistry (stigmergic coordination), ArenaRegistry, EvalRegistry, BountyMarket, DisputeResolver. **Verify**: all contracts compile with `forge build`. Unit tests pass with `forge test`.
  - Spec: `tmp/unified/18-ON-CHAIN-REGISTRIES.md` §2-7
  - Depth: `tmp/unified-depth/18-registries/` (pending)
  - Code: `contracts/src/`

- [ ] **Deploy to Nunchi testnet** — Deploy all contracts via foundry scripts. Verify deployment addresses. Configure environment. **Verify**: all contracts accessible at deployed addresses.
  - Spec: `tmp/unified/18-ON-CHAIN-REGISTRIES.md` §8
  - Code: `contracts/deploy/`

- [ ] **Implement Rust clients for all registries** — Alloy-based typed clients for each contract. Methods mirror Solidity interface. **Verify**: Rust client can register an agent, submit reputation, publish knowledge Signal, all on testnet.
  - Spec: `tmp/unified/18-ON-CHAIN-REGISTRIES.md`
  - Code: `crates/roko-chain/src/clients/` (new)

- [ ] **Wire passport registration into Agent startup** — When an Agent starts in Active state, if no on-chain passport exists, register one. Store passport ID in Agent state. **Verify**: new Agent registers passport on first run. Subsequent runs reuse existing passport.
  - Code: `crates/roko-agent/src/passport.rs` (new)

- [ ] **Wire knowledge publication from Memory store** — When a Signal reaches Persistent tier, optionally publish its HDC fingerprint + metadata to InsightStore on-chain. Configurable opt-in per workspace. **Verify**: persistent Signal appears in on-chain InsightStore.
  - Code: `crates/roko-neuro/src/publish.rs` (new)

- [ ] **Implement event indexer** — Chain event indexer: subscribe to contract events, normalize into Pulses on Bus, store in PostgreSQL for query. REST API for indexed data. **Verify**: contract event emitted → Pulse on Bus → queryable via REST.
  - Spec: `tmp/unified/18-ON-CHAIN-REGISTRIES.md` §9
  - Code: `crates/roko-chain/src/indexer/` (new)

## 3.5 Arena System

**[BLOCKED:depth]** — Depends on `tmp/unified-depth/19-arenas/` depth docs.

- [ ] **Define Arena types** — `Arena { id, name, task_source, scoring_fn, leaderboard_config, ground_truth_source }`. 8 concrete arenas: Coding, Trading, Prediction, Research, Security Audit, Optimization, Agentic, MetaArena. **Verify**: arena types compile and serialize.
  - Spec: `tmp/unified/19-ARENAS-EVALS-BOUNTIES.md` §2-3
  - Code: `crates/roko-learn/src/arena/` (new)

- [ ] **Implement 7-step flywheel** — TRACE (record execution) → AUTO-GRADE (scoring function) → PREFERENCE-MINE (extract pairwise preferences) → FAILURE-CLUSTER (group failures by HDC similarity) → CURRICULUM-GEN (generate training tasks from failure clusters) → PATTERN-EXTRACT (extract Heuristic Signals from clusters) → PREFERENCE-BOOTSTRAP (create training data from preferences). **Verify**: feed 100 episodes into flywheel, confirm each step produces output.
  - Spec: `tmp/unified/19-ARENAS-EVALS-BOUNTIES.md` §4 (7-Step Flywheel)
  - Code: `crates/roko-learn/src/arena/flywheel.rs` (new)

- [ ] **Implement Eval protocol** — Ground truth sources: test suites, oracles, human review, chain state, benchmarks. Variance Inequality: verifier must be spectrally cleaner than generator. No LLM-judging-itself. **Verify**: eval correctly scores agent output against ground truth.
  - Spec: `tmp/unified/19-ARENAS-EVALS-BOUNTIES.md` §5
  - Code: `crates/roko-learn/src/arena/eval.rs` (new)

- [ ] **Implement Bounty system** — Escrow before execution. Task matching via VCG. Second-price auction for competitive bounties. Reputation settlement after completion. 4-level dispute resolution (arbiter → court → council → DAO vote). **Verify**: post bounty → agent claims → completes → escrow releases → reputation updated.
  - Spec: `tmp/unified/19-ARENAS-EVALS-BOUNTIES.md` §6-7
  - Code: `crates/roko-learn/src/arena/bounty.rs` (new)

- [ ] **Arena and Bounty HTTP routes** — `/api/arenas/`, `/api/bounties/`. Full CRUD + submission + scoring + leaderboard endpoints. **Verify**: HTTP API supports full arena and bounty lifecycle.
  - Code: `crates/roko-serve/src/routes/arenas.rs` (new), `crates/roko-serve/src/routes/bounties.rs` (new)

- [ ] **Cross-arena transfer detection** — Detect when skills learned in one arena transfer to another using HDC fingerprint correlation between arena episodes. Credit the originating arena. **Verify**: skill learned in Coding arena improves score in Security Audit arena → transfer detected and credited.
  - Spec: `tmp/unified/19-ARENAS-EVALS-BOUNTIES.md` §8
  - Code: `crates/roko-primitives/src/transfer.rs` (new)

## 3.6 Brain Export and Import

- [ ] **Define brain export format** — Manifest + knowledge Signals (filtered by tier/date) + learning state (calibration, route posteriors, section effects) + episodes (optional). Target size: 100KB–1MB. Format: CBOR with Merkle tree over entries. **Verify**: export produces valid file under 1MB for a workspace with 10K Signals.
  - Spec: `tmp/unified/20-DEPLOYMENT.md` §4 (Brain Export)
  - Code: `crates/roko-neuro/src/brain/format.rs` (new)

- [ ] **Implement brain export with filters** — CLI: `roko knowledge export --min-tier=Working --since=30d --include-episodes`. Filters: minimum tier, date range, include/exclude episodes, include/exclude learning state. **Verify**: export with `--min-tier=Consolidated` excludes Transient and Working Signals.
  - Code: `crates/roko-neuro/src/brain/export.rs` (new)

- [ ] **Implement brain import with decay factor** — CLI: `roko knowledge import <file> --decay=0.5`. Imported Signals start with `balance * decay_factor`. Prevents imported knowledge from dominating local knowledge. Conflicts resolved by content hash (identical = skip, different = keep both with lineage link). **Verify**: import with decay 0.5 gives imported Signals half the balance of native ones.
  - Code: `crates/roko-neuro/src/brain/import.rs` (new)

- [ ] **Implement Merkle-CRDT sync** — Merkle tree over brain state (knowledge + learning). CRDT operations: GCounter (for citation counts), LWW-Register (for calibration state), Add-only set (for Signals). Two instances with divergent learning compute Merkle diff, exchange missing entries, merge via CRDT rules. **Verify**: two instances diverge for 100 operations, sync, converge to identical state.
  - Spec: `tmp/unified/20-DEPLOYMENT.md` §4.3
  - Code: `crates/roko-neuro/src/brain/merkle.rs` (new), `crates/roko-neuro/src/brain/crdt.rs` (new), `crates/roko-neuro/src/brain/sync.rs` (new)

## 3.7 Cross-Agent Knowledge Sharing

- [ ] **Knowledge Signal broadcast via relay** — When a Signal reaches Persistent tier with high confidence, optionally broadcast its HDC fingerprint + summary via the workspace relay (WebSocket/Iroh P2P). Receiving agents can request the full Signal if similarity to their active context exceeds threshold. **Verify**: Agent A publishes knowledge → Agent B receives fingerprint → requests full Signal → imports into local store.
  - Spec: `tmp/unified/12-CONNECTIVITY.md` §3 (Relay)
  - Code: `crates/roko-runtime/src/knowledge_sync.rs` (new)

- [ ] **On-chain knowledge discovery** — Query InsightStore for Signals similar to a local query via HDC precompile. Download matching Signals. Import with provenance tracking (on-chain source). **Verify**: query InsightStore with HDC vector → get matching Signals → import with chain provenance.
  - Code: `crates/roko-chain/src/knowledge_discovery.rs` (new)

## 3.8 Deployment

**[BLOCKED:depth]** — Depends on `tmp/unified-depth/20-deployment/` depth docs for cloud and WASM specifics.

- [ ] **WASM compilation target** — Compile roko-core + selected Cells to WASM via `wasm32-wasi`. Verify core types (Signal, Pulse, Cell) work in WASM context. **Verify**: a Cell compiled to WASM executes correctly in wasmtime.
  - Spec: `tmp/unified/20-DEPLOYMENT.md` §5
  - Code: workspace Cargo.toml (add wasm target)

- [ ] **Agent execution tiers** — T0: in-process (current default). T1: sidecar (existing roko-agent-server). T2: container (Docker). T3: VM (Firecracker, future). T4: cluster (k8s, future). Configuration in workspace.toml. **Verify**: an Agent can run as T0 (in-process) or T1 (sidecar) based on config.
  - Spec: `tmp/unified/20-DEPLOYMENT.md` §3
  - Code: `crates/roko-agent/src/`, `crates/roko-agent-server/src/`
