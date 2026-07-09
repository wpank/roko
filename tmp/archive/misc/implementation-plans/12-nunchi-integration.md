# ⚠️ SUPERSEDED — Split into [12a-cognitive-layer.md](12a-cognitive-layer.md) + [12b-chain-layer.md](12b-chain-layer.md)
>
> Cognitive sections (D,E,F,G,I,J) → 12a. Chain sections (A,B,C,H,K-Q) → 12b.
> This file retained for historical reference.

---

# 12 — Nunchi Platform Integration: Full Implementation List

> Generated 2026-04-08. Updated with PRD specs, research papers, dogfooding plan (11),
> and collaboration repo specs. Sources of truth: PRD docs in bardo-backup/prd/,
> collaboration repo specs, research papers in bardo-backup/.

## Architecture Principles

1. **roko-neuro** replaces grimoire. It's a standalone crate — memory/knowledge for ANY roko
   agent, not just blockchain agents. A solo agent running `roko plan run` benefits equally.

2. **roko-golem** = blockchain variant of roko. It adds `ChainWitness`, on-chain identity,
   and chain-specific behaviors on top of the base roko cognitive stack. Golem has a runtime
   and chain watcher; roko does not require one.

3. **Everything modular/composable.** Daimon, Dreams, Neuro, ChainWitness are independent
   crates (or well-separated modules) with trait-based interfaces. A developer can use
   Neuro without Daimon, or Dreams without a chain.

4. **mirage-rs** = Korai chain proxy/mock. All chain-side functionality lives here. Fast
   in-process mock gossip mesh (broadcast channels), not libp2p.

5. **Aligns with dogfooding plan (11).** The cybernetic loop from plan 11 is:
   `Event → Agent → Action → Outcome → Feedback → Learning → Better Agent`. Everything
   below feeds into that loop. C-factor and other metrics should be quantifiable from day 1.

## Legend

- **[roko]** = implement in roko crates (agent-side, works without chain)
- **[mirage]** = implement in mirage-rs (Korai chain proxy/mock)
- **[both]** = agent + chain-proxy coordination
- **[neuro]** = roko-neuro crate specifically
- **[golem]** = roko-golem crate (blockchain variant only)
- ✅ BUILT = code exists, may need wiring
- 🔧 SCAFFOLD = struct exists, no logic
- 🆕 NEW = nothing exists yet

---

## A. Agent Identity & Registration

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| A1 | `AgentPassport` struct (address, owner, system_prompt_hash, stake, tier, capabilities bitmask) | [golem] | 🔧 | mirage has `AgentEntry`; needs full passport fields |
| A2 | Passport registration RPC (`chain_registerPassport`) | [mirage] | 🔧 | `chain_registerAgent` exists, lacks stake/tier/caps |
| A3 | System prompt hash verification (Ventriloquist defense: hash prompt at build time, verify on-chain) | [roko] | 🆕 | Prevents prompt injection attacks at registration |
| A4 | Tier progression logic (Probation→Active→Elite→Master) | [mirage] | 🆕 | Based on `jobs_completed + reputation_score` thresholds |
| A5 | Capability bitmask declaration & query (Trading, Security, Data, Knowledge, Strategy, Analytics) | [both] | 🆕 | Agent declares; chain indexes for discovery |
| A6 | Wallet/signing integration (Ed25519) | [golem] | 🆕 | For signing gossip envelopes, txs, attestations |
| A7 | Local agent identity (non-chain) | [roko] | 🆕 | Agent has an ID even without a chain — `.roko/identity.json` |

## B. Gossip Mesh (P2P / Stigmergy Transport)

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| B1 | `GossipEnvelope` message format (version, topic, sender, timestamp, payload, sig) | [both] | 🆕 | Universal wrapper for all gossip |
| B2 | 8 topic subscriptions (txs, capabilities, reputation, spore/jobs, spore/deltas, spore/status, sparrow, isfr) | [mirage] | 🆕 | mirage mocks the mesh; agents subscribe |
| B3 | Message signing & validation (Ed25519) | [golem] | 🆕 | Sign outgoing, verify incoming |
| B4 | Heartbeat publishing (30-60s interval) | [golem] | 🔧 | `chain_agentHeartbeat` RPC exists; needs gossip envelope |
| B5 | 3-layer peer scoring (behavioral 0.4 + economic 0.4 + TEE 0.2) | [mirage] | 🆕 | Composite score per peer |
| B6 | In-process mock gossip mesh (tokio broadcast channels per topic) | [mirage] | 🆕 | Fast mock, no libp2p |

## C. Job Market (Spore + Sparrow)

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| C1 | `SporeJob` posting & discovery | [mirage] | 🆕 | Job spec: reward, deadline, quality threshold, required capabilities |
| C2 | `BountySpec` with hiring models (fixed price, reverse auction, direct hire) | [mirage] | 🆕 | Full job definition |
| C3 | `SparrowBid` submission (price, ETA, confidence) | [golem] | 🆕 | Agent bids on discovered jobs |
| C4 | Power-of-two-choices dispatch (probe 2 random capable agents, assign least loaded) | [mirage] | 🆕 | Scheduler selection algorithm |
| C5 | Job lifecycle state machine (Created→Claimed→Running→Completed/Failed) | [both] | 🆕 | Timeout fallbacks at each state |
| C6 | `JobReceipt` with proof-of-execution (output_hash, execution_ms, gate results) | [both] | 🆕 | Agent submits; chain verifies |
| C7 | Escrow & settlement (budget lock → release on completion / slash on failure) | [mirage] | 🆕 | Mock DAEJI token escrow |
| C8 | Mining types (Genome, Verifier, Repair, Mechanism, Index, Memory) | [mirage] | 🆕 | 6 mining job categories for ecosystem maintenance |
| C9 | `DeltaArtifact` submission (before/after metrics, artifact hash) | [golem] | 🆕 | Mining solution format |

## D. Knowledge & Memory (roko-neuro)

> **Key principle**: Neuro is NOT blockchain-specific. Every roko agent gets a local knowledge
> store. The chain is just one possible sync/persistence backend. The distillation pipeline
> works identically for `roko plan run` and for a golem on Korai.

### D.1 Distillation Pipeline (PRD: 04-memory/01-grimoire.md)

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| D1 | **4-tier distillation**: Raw Episodes → Insights → Heuristics → PLAYBOOK | [neuro] | 🆕 | Core spec from PRD. Each tier compresses + validates |
| D2 | Episode → Insight extraction (pattern detection across episodes) | [neuro] | 🆕 | "When X happened, Y consistently followed" |
| D3 | Insight → Heuristic promotion (3+ confirmations → actionable rule) | [neuro] | 🆕 | Confidence threshold for promotion |
| D4 | Heuristic → PLAYBOOK compilation (top heuristics → `PLAYBOOK.md` action rules) | [neuro] | 🆕 | Human-readable + machine-parseable playbook |
| D5 | Temporal decay (half-life per knowledge type, configurable defaults) | [neuro] | 🆕 | Insights: 30d, Heuristics: 90d, Facts: 365d |
| D6 | Confirmation boost (independent validation extends weight by 1.5x) | [neuro] | 🆕 | `w_new = w_old * 1.5; half_life *= 1.2` |

### D.2 Knowledge Types & Storage

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| D7 | 6 knowledge types: Fact, Query, Strategy, Insight, Heuristic, AntiKnowledge | [neuro] | 🔧 | mirage has `KnowledgeKind`; neuro needs local equivalents |
| D8 | Local JSONL knowledge store (`.roko/neuro/knowledge.jsonl`) | [neuro] | 🆕 | Append-only, GC'd by decay |
| D9 | Knowledge entry struct (content, type, confidence, source_episodes, hdc_vector, created, half_life) | [neuro] | 🆕 | Core data model |
| D10 | AntiKnowledge (challenge mechanism: contradicts existing knowledge, 2x stake requirement on chain) | [neuro] | 🆕 | Locally: "this insight was wrong" with evidence |
| D11 | Knowledge query API (semantic search + temporal relevance + affect filters) | [neuro] | 🆕 | Used by context assembly |

### D.3 HDC Integration (Research: hdc-vsa.md, hdc-fingerprint.md)

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| D12 | HDC encoding for knowledge entries (text → 10,240-bit BSC vector) | [neuro] | ✅ | `bardo-primitives::HdcVector` exists + mirage `projection.rs` |
| D13 | Local HDC index (Hamming-distance search, <1ms retrieval) | [neuro] | ✅ | mirage `HdcIndex` + `HnswBinaryIndex`; roko has `hdc_clustering` |
| D14 | Wire HDC into episode fingerprinting (per plan 11 §7.4) | [roko] | 🆕 | `fingerprint(&signal.body)` → metadata |
| D15 | Wire HDC into knowledge retrieval (query → HDC → top-k → context) | [neuro] | 🆕 | Cosine/Hamming similarity search over knowledge store |
| D16 | Similarity-based template recommendation (plan 11 §7.4: no exact match → HDC suggest) | [roko] | 🆕 | `cosine_similarity > 0.7` threshold |
| D17 | HDC semantic neighborhoods (shift vectors for "related concepts") | [neuro] | 🆕 | Research: permutation/rotation = sequential, XOR = binding |
| D18 | HDC fingerprint for signals (plan 11 §7.4) | [roko] | 🆕 | Every webhook signal gets fingerprinted on ingress |

### D.4 Chain Sync (golem-specific)

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| D19 | Publish knowledge to chain (`InsightEntry` → mirage `InsightLedger`) | [golem] | 🆕 | Chain is a shared knowledge pool |
| D20 | Pull knowledge from chain (semantic query against shared ledger) | [golem] | 🆕 | Supplement local knowledge with network knowledge |
| D21 | Pheromone read/write for stigmergic coordination | [golem] | 🔧 | mirage has pheromone substrate; needs agent-side consumer |
| D22 | Pheromone weight dynamics (exponential decay + confirmation boost formula) | [mirage] | 🔧 | `Pheromone` struct has decay; needs full formula from PRD |

## E. Context Assembly (5-Stage Pipeline)

> PRD: 12-inference/04-context-engineering.md. Extends existing SystemPromptBuilder.

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| E1 | 5-stage pipeline: Query → Score → Deduplicate → Budget → Format | [roko] | 🔧 | SystemPromptBuilder does 6-layer assembly; needs scoring/dedup stages |
| E2 | **Active inference scoring** (pragmatic value × epistemic value, balanced by uncertainty) | [roko] | 🆕 | PRD formula: `score = track_record(entry) × belief_change(entry) / uncertainty` |
| E3 | **Attention-curve positioning** (Liu et al. U-shape: high-value at start+end of prompt) | [roko] | 🆕 | Reorder retrieved entries by attention curve |
| E4 | Affect-modulated retrieval (PAD state biases what knowledge is surfaced) | [roko] | 🆕 | Depends on Daimon (F1); high arousal → recent + action-oriented |
| E5 | Dynamic token budget (fit within model context window, prioritize by score) | [roko] | 🔧 | SystemPromptBuilder has layers; needs dynamic budget allocation |
| E6 | Neuro injection (pull from local knowledge store during context assembly) | [roko] | 🆕 | Bridge between roko-neuro and roko-compose |
| E7 | Chain state injection (current pheromones, reputation, market data from mirage) | [golem] | 🆕 | Golem-specific: real-time chain data in prompt |

## F. Daimon (Affect/Motivation Engine)

> PRD: 03-daimon/00-overview.md. Standalone module — any agent can have affect, not just golems.

### F.1 PAD Model (Mehrabian-Russell)

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| F1 | `PadVector` (Pleasure, Arousal, Dominance: each f32 in [-1, 1]) | [roko] | 🔧 | `DaimonEngine` stub in roko-golem; extract to standalone |
| F2 | 8 affect states from PAD octants (+P+A+D=Excited, -P+A-D=Anxious, etc.) | [roko] | 🆕 | Maps vector → named state |
| F3 | Appraisal triggers (task success/failure, gate results, reputation changes, time pressure) | [roko] | 🆕 | Events → PAD vector updates via appraisal rules |
| F4 | Decay toward baseline (affect decays to [0,0,0] with configurable half-life) | [roko] | 🆕 | Prevents permanent affect drift |
| F5 | Affect → behavior modulation table (state → risk tolerance, communication style, exploration rate) | [roko] | 🆕 | E.g., Anxious → conservative, lower exploration |

### F.2 Affect Integration

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| F6 | Affect signatures on episodes (every agent turn tagged with current PAD) | [roko] | 🆕 | Enriches episode logging (extends plan 11 §7.1) |
| F7 | Affect → SystemPromptBuilder (emotional state modifies prompt tone/focus) | [roko] | 🆕 | "You are under time pressure" vs "You have time to explore" |
| F8 | Affect → CascadeRouter (arousal level influences model selection: high arousal → faster model) | [roko] | 🆕 | Extends existing CascadeRouter |
| F9 | Affect persistence (`.roko/daimon/affect.json`, survives restart) | [roko] | 🆕 | Agent "wakes up" with residual affect |
| F10 | Chain events → appraisals (tx outcomes, slashing, reputation changes) | [golem] | 🆕 | ChainWitness feeds Daimon |

## G. Dreams (Offline Intelligence)

> PRD: 05-dreams/00-overview.md. Works for any agent — replays past episodes to extract learning.

### G.1 NREM Replay (v1)

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| G1 | Episode replay scheduler (trigger during idle time — no active tasks) | [roko] | 🆕 | When agent has nothing to do, it dreams |
| G2 | Re-evaluate past episodes with current knowledge (would I do this differently now?) | [roko] | 🆕 | Compare past decision vs current heuristics |
| G3 | Mistake identification (failed episodes → what went wrong → insight) | [roko] | 🆕 | Feed into Neuro distillation pipeline (D2) |
| G4 | Heuristic strengthening/weakening from replay (confirm or revise) | [roko] | 🆕 | Update confidence scores in knowledge store |
| G5 | Dreams output → Neuro (replay generates new Insights/Heuristics) | [roko] | 🆕 | Direct pipe: dreams → D1-D4 pipeline |

### G.2 REM Imagination (v2, later)

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| G6 | Counterfactual simulation (HDC vector shifting: "what if X had been different?") | [roko] | 🆕 | Use HDC permutation to explore semantic neighborhoods |
| G7 | Cross-episode consolidation (discover meta-patterns across unrelated episodes) | [roko] | 🆕 | HDC bundling of episode vectors → cluster detection |
| G8 | Novel strategy generation (combine heuristics from different domains) | [roko] | 🆕 | Cross-pollination of knowledge |

## H. ChainWitness (Golem-Specific)

> Golem-only. Watches on-chain events and feeds them into the cognitive stack.

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| H1 | Subscribe to relevant on-chain events (via mirage RPC or gossip) | [golem] | 🔧 | `ChainWitnessEngine` stub exists |
| H2 | Event → Signal conversion (chain events become roko Signals) | [golem] | 🆕 | Bridge chain world into roko signal graph |
| H3 | ChainWitness → Daimon feed (tx outcomes trigger appraisals) | [golem] | 🆕 | Profit → +Pleasure, Loss → -Pleasure+Arousal |
| H4 | ChainWitness → Neuro feed (witnessed patterns become knowledge) | [golem] | 🆕 | Observed market patterns → insights |
| H5 | Configurable event filters (which contracts/events to watch) | [golem] | 🆕 | Per-golem config in `roko.toml` |

## I. Operating Frequencies (3-Speed Cognition)

> PRD: cognitive architecture research. Not golem-specific — any agent benefits from multi-timescale thinking.

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| I1 | **Gamma loop** (~10s): reactive — perceive, retrieve, act | [roko] | 🆕 | Current orchestration loop is mostly gamma |
| I2 | **Theta loop** (~2-5min): strategic — re-plan, update goals, evaluate progress | [roko] | 🆕 | Periodic "step back and think about the plan" |
| I3 | **Delta loop** (~30min+): consolidation — dreams replay, knowledge distillation, meta-cognition | [roko] | 🆕 | Trigger dreams (G1), playbook compilation (D4) |
| I4 | Frequency scheduler (decides which loop to run based on context) | [roko] | 🆕 | Time-since-last-theta, idle-detection, etc. |
| I5 | Meta-cognition hook (agent reflects on its own cognitive state) | [roko] | 🆕 | "Am I stuck? Am I thrashing? Should I escalate?" |

## J. C-Factor (Collective Intelligence Metrics)

> Research: Woolley et al. (2010) — c-factor accounts for 43% of variance in group performance.
> Design: flexible — per-agent, per-fleet, chain-wide. Quantifiable from day 1.

### J.1 Core Metrics

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| J1 | **Information flow rate** (messages sent/received per unit time, latency) | [both] | 🆕 | Measure gossip propagation speed; locally measure signal throughput |
| J2 | **Turn-taking equality** (Gini coefficient of agent contributions) | [both] | 🆕 | Even participation = higher c-factor. Locally: are all agents productive? |
| J3 | **Social sensitivity proxy** (response quality to other agents' outputs) | [roko] | 🆕 | How well does agent incorporate context from others? |
| J4 | **Knowledge integration rate** (how fast shared insights get confirmed) | [both] | 🆕 | Track confirmation chains in Neuro |
| J5 | **Task diversity coverage** (are agents specializing effectively?) | [both] | 🆕 | Capability utilization vs overlap |
| J6 | **Convergence velocity** (time from divergent opinions to shared conclusion) | [both] | 🆕 | Measure via ISFR or knowledge agreement |

### J.2 Scoped Computation

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| J7 | Per-agent c-factor contribution score | [roko] | 🆕 | How much does this agent improve collective intelligence? |
| J8 | Per-fleet c-factor (across agents in a `roko plan run` session) | [roko] | 🆕 | Measurable today: multi-agent plan execution |
| J9 | Chain-wide c-factor (across all registered agents on mirage/Korai) | [mirage] | 🆕 | Aggregate metric, published periodically |
| J10 | C-factor → agent selection (prefer agents that improve collective c-factor) | [both] | 🆕 | Route tasks to agents that fill gaps |

### J.3 Dashboard Integration (extends plan 11 §7.5)

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| J11 | C-factor metrics endpoint (`GET /api/metrics/c_factor`) | [roko] | 🆕 | Per plan 11 cybernetic metrics dashboard |
| J12 | C-factor time-series tracking (`.roko/learn/c-factor.jsonl`) | [roko] | 🆕 | Historical trend for self-improvement velocity |
| J13 | C-factor visualization in TUI (plan 09) | [roko] | 🆕 | Show collective intelligence trends |

## K. Reputation System

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| K1 | 7 domain tracks (Trading, Predictions, Data, Security, Knowledge, Strategy, Analytics) | [mirage] | 🆕 | Per-domain EMA scores |
| K2 | EMA scoring with 30-day half-life | [mirage] | 🆕 | `score_new = α * outcome + (1-α) * score_old` |
| K3 | Tier-based trust multipliers (Probation=0.5x, Active=1x, Elite=1.5x, Master=2x) | [mirage] | 🆕 | Affects job eligibility + commission |
| K4 | Discipline protocol (penalty points → warning → probation → suspension) | [mirage] | 🆕 | Escalation ladder |
| K5 | Slashing schedule (2% abandoned job → 100% TEE violation) | [mirage] | 🆕 | 6 offense types |
| K6 | Local reputation tracking (agent monitors its own scores) | [roko] | 🆕 | Feeds into Daimon appraisals + context assembly |
| K7 | Reputation → peer scoring bridge (maps to gossip peer scoring B5) | [mirage] | 🆕 | High reputation = higher peer score |
| K8 | Dispute resolution state machine (file → panel → vote → appeal → finalize) | [mirage] | 🆕 | 3-agent panel, commit-reveal voting |

## L. Payments & Economics

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| L1 | Mock DAEJI token (staking, escrow, slashing ledger) | [mirage] | 🆕 | In-memory balance tracking |
| L2 | Job escrow lifecycle (lock → release/slash/refund) | [mirage] | 🆕 | Tied to job state machine (C5) |
| L3 | Fee structure (0.5% posting, 5% validation, 2% protocol) | [mirage] | 🆕 | Automatic deduction |
| L4 | X402 micropayment protocol (HTTP 402 wallet-sig for knowledge API access) | [mirage] | 🆕 | Monetize knowledge queries |
| L5 | Agent balance tracking & cost reporting | [roko] | 🔧 | Plan 11 §7.5 tracks `avg_cost_per_episode_cents` |

## M. Watcher & Safety

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| M1 | Watcher agent type (monitors other agents for policy compliance) | [mirage] | 🆕 | 5 check types: policy, behavioral, solvency, attestation, correlation |
| M2 | Escalation ladder (Advisory → Throttle → Freeze → Slash) | [mirage] | 🆕 | 2 independent watchers confirm High/Critical |
| M3 | Bounded safe actions (auto-trigger on threshold breach) | [mirage] | 🆕 | E.g., widen spreads on solvency drop |
| M4 | `GuardianFreeze` (lock agent state on critical violation) | [mirage] | 🆕 | Reversible by governance |
| M5 | `PolicyManifest` per agent (position limits, asset universe, max drawdown) | [both] | 🆕 | Agent declares, watcher enforces |
| M6 | Safety levels from plan 11 §8.8 (strict/normal/autonomous) | [roko] | 🆕 | Configurable in `roko.toml` |
| M7 | Circuit breaker (halt after N consecutive failures, cost budget cap) | [roko] | 🔧 | roko-conductor has circuit breaker; needs config wiring |

## N. ISFR (Collective Price Discovery)

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| N1 | `IsfrSubmission` (agent submits rate + confidence after clearing round) | [golem] | 🆕 | Published on korai/isfr topic |
| N2 | `IsfrAggregate` (median rate from 3+ submissions per market) | [mirage] | 🆕 | Chain computes + broadcasts |
| N3 | Agent consumes ISFR to update local pricing models | [golem] | 🆕 | Feeds into context assembly |

## O. Cooperative Clearing

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| O1 | Clearing engine (QP solver: minimize inventory cost) | [mirage] | 🆕 | Off-chain solve, on-chain verify |
| O2 | Soft-threshold analytical solution + bisection for λ* (O(80n) algorithm) | [mirage] | 🆕 | From collaboration spec |
| O3 | `ClearingCertificate` (KKT optimality proof, PU18 precision) | [mirage] | 🆕 | On-chain verification in O(n) |
| O4 | Agent submits clearing parameters (γ, c, I_min, I_max) sealed commitment | [golem] | 🆕 | Commit-reveal before solve |
| O5 | Fallback ladder (full clear → pruned → external hedge → safe mode) | [mirage] | 🆕 | Deterministic fallback chain |

## P. Privacy & TEE

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| P1 | 4 privacy modes (PUBLIC, OPERATOR_PRIVATE, HYBRID_CONFIDENTIAL, FULL_CONFIDENTIAL) | [both] | 🆕 | Per-knowledge-entry privacy level |
| P2 | TEE attestation stub (mock for dev, real for prod) | [mirage] | 🆕 | AWS Nitro / Intel TDX format |
| P3 | Private Set Intersection (position matching without revealing positions) | [golem] | 🆕 | X25519 DH + HMAC-SHA256 |
| P4 | Zero-knowledge range proofs (prove collateral > threshold without value) | [golem] | 🆕 | Bulletproofs over Ristretto255 |

## Q. Mirage-RS Infrastructure

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| Q1 | In-process gossip mesh mock (tokio broadcast::channel per topic) | [mirage] | 🆕 | Fast, no libp2p |
| Q2 | Mock consensus / auto block advancement (configurable interval) | [mirage] | 🔧 | `mirage_stepBlock` exists; needs auto-advance mode |
| Q3 | Persistent on-disk state (knowledge, pheromones, agents survive restart) | [mirage] | 🆕 | Currently all in-memory |
| Q4 | Multi-agent simulation mode (register N agents, run scenarios) | [mirage] | 🆕 | For testing collective behaviors + c-factor |
| Q5 | Event replay / time-travel debugging | [mirage] | 🔧 | Snapshot/revert exists; needs event log replay |
| Q6 | Aggregated metrics endpoints for dashboard | [mirage] | 🔧 | HTTP API exists; needs metrics rollups |
| Q7 | MCP server exposing chain operations (korai/knowledge/query, korai/marketplace/tasks, etc.) | [mirage] | 🆕 | From SDK spec: agents interact via MCP tools |

## R. Crate Architecture (Refactoring)

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| R1 | Create `roko-neuro` crate (extract from roko-golem grimoire scaffold + roko-learn relevant parts) | [neuro] | 🆕 | Knowledge store, distillation, HDC retrieval |
| R2 | Extract Daimon to `roko-daimon` or standalone module with trait interface | [roko] | 🆕 | `trait AffectEngine { fn appraise(&mut self, event) → PadVector }` |
| R3 | Extract Dreams to `roko-dreams` or standalone module | [roko] | 🆕 | `trait DreamEngine { fn replay(&mut self, episodes) → Vec<Insight> }` |
| R4 | Keep roko-golem as blockchain-variant assembly (imports neuro + daimon + dreams + chain_witness) | [golem] | 🔧 | Thin glue crate, not monolithic |
| R5 | Extract `roko-serve` from `roko-cli` (plan 11 §0.1) | [roko] | 🆕 | Reusable server library |
| R6 | Create `roko-plugin` crate (plan 11 §0.2: Integration/EventSource/FeedbackCollector traits) | [roko] | 🆕 | Integration SDK |
| R7 | Remove `mortality`, `hypnagogia` modules from roko-golem (death concepts removed) | [golem] | 🆕 | Clean up scaffold |

---

## Dependency Graph (Implementation Order)

```
Layer 0 (Foundation — no external deps, enables everything):
  R1-R7  Crate architecture refactoring
  A7     Local agent identity
  D7-D9  Knowledge types + storage (roko-neuro basics)
  D12-D13 HDC encoding + index (ALREADY BUILT, wire it)
  F1     PadVector struct

Layer 1 (Core cognitive — needs Layer 0):
  D1-D6   Distillation pipeline (episodes → insights → heuristics → playbook)
  D14-D18 HDC wiring (signals, episodes, knowledge retrieval)
  F2-F5   Daimon affect model + behavior modulation
  E1-E6   Context assembly pipeline (extends SystemPromptBuilder)
  J7-J8   Per-agent + per-fleet c-factor (measurable immediately)

Layer 2 (Learning + Dreams — needs Layer 1):
  G1-G5   Dreams v1 NREM replay
  I1-I4   3-speed cognition (gamma/theta/delta)
  F6-F9   Affect integration (episodes, prompts, routing, persistence)
  D10-D11 AntiKnowledge + knowledge query API
  J1-J6   C-factor core metrics
  J11-J13 Dashboard integration

Layer 3 (Chain infrastructure — needs Layer 0-1):
  Q1-Q7   Mirage infrastructure (gossip mock, persistence, metrics)
  A1-A6   Agent passport + identity
  B1-B6   Gossip mesh
  K1-K8   Reputation system
  L1-L5   Payments + economics

Layer 4 (Chain agent behavior — needs Layer 2-3):
  C1-C9   Job market (Spore + Sparrow)
  H1-H5   ChainWitness
  D19-D22 Knowledge chain sync + pheromones
  E7      Chain state context injection
  F10     Chain events → appraisals
  M1-M7   Watcher + safety

Layer 5 (Advanced collective — needs Layer 4):
  N1-N3   ISFR collective price discovery
  O1-O5   Cooperative clearing
  J9-J10  Chain-wide c-factor + agent selection
  G6-G8   Dreams v2 (counterfactuals, consolidation)
  I5      Meta-cognition
  P1-P4   Privacy + TEE
```

## Counts

| Category | Items | 🆕 | 🔧 | ✅ |
|----------|-------|-----|-----|-----|
| A. Identity | 7 | 5 | 1 | 1 |
| B. Gossip | 6 | 5 | 1 | 0 |
| C. Job Market | 9 | 9 | 0 | 0 |
| D. Knowledge (Neuro) | 22 | 16 | 3 | 3 |
| E. Context Assembly | 7 | 4 | 2 | 1 |
| F. Daimon | 10 | 9 | 1 | 0 |
| G. Dreams | 8 | 8 | 0 | 0 |
| H. ChainWitness | 5 | 4 | 1 | 0 |
| I. Operating Frequencies | 5 | 5 | 0 | 0 |
| J. C-Factor | 13 | 13 | 0 | 0 |
| K. Reputation | 8 | 8 | 0 | 0 |
| L. Payments | 5 | 4 | 1 | 0 |
| M. Safety | 7 | 5 | 2 | 0 |
| N. ISFR | 3 | 3 | 0 | 0 |
| O. Clearing | 5 | 5 | 0 | 0 |
| P. Privacy | 4 | 4 | 0 | 0 |
| Q. Mirage Infra | 7 | 3 | 4 | 0 |
| R. Crate Architecture | 7 | 6 | 1 | 0 |
| **TOTAL** | **131** | **116** | **17** | **5** |
