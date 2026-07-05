# 12b — Chain Layer: Identity, Market, Reputation, Witness

> **Split from**: 12-nunchi-integration.md (chain sections A, B, C, H, K, L, M, N, O, P, Q, R4-R7)
> **Master plan reference**: Tier 6 (sections 6A–6G)
> **Priority**: P3 — Blockchain-specific features for roko-golem
> **Depends on**: Tier 5 (Cognitive Layer) complete
> **Status**: Deferred (intentionally — focus on Tiers 1-5 first)
>
> These components are specific to on-chain agents (roko-golem) running on the Korai
> chain via mirage-rs. Solo CLI agents and event-driven agents do NOT need these.
> Cognitive capabilities are in 12a-cognitive-layer.md.

## Architecture Principles

1. **roko-golem** = blockchain variant of roko. It adds `ChainWitness`, on-chain identity,
   and chain-specific behaviors on top of the base roko cognitive stack. Golem has a runtime
   and chain watcher; roko does not require one.

2. **mirage-rs** = Korai chain proxy/mock. All chain-side functionality lives here. Fast
   in-process mock gossip mesh (broadcast channels), not libp2p.

3. **Everything modular/composable.** ChainWitness, reputation, payments, ISFR, and clearing
   are independent modules with trait-based interfaces. A developer can use identity without
   payments, or reputation without clearing.

4. **Aligns with dogfooding plan (11).** The cybernetic loop from plan 11 is:
   `Event → Agent → Action → Outcome → Feedback → Learning → Better Agent`. Chain-layer
   components feed on-chain events into that loop and publish results back to the chain.

## Legend

- **[golem]** = roko-golem crate (blockchain variant only)
- **[mirage]** = implement in mirage-rs (Korai chain proxy/mock)
- **[both]** = agent + chain-proxy coordination
- **[roko]** = implement in roko crates (agent-side, works without chain)

Status markers:
- ✅ BUILT = code exists, may need wiring
- 🔧 SCAFFOLD = struct exists, no logic
- 🆕 NEW = nothing exists yet

Key paths:
- Golem crate: `/Users/will/dev/nunchi/roko/roko/crates/roko-golem/`
- Chain crate: `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/`
- mirage-rs: separate repo (reference only)
- PRD docs: `/Users/will/dev/nunchi/roko/bardo-backup/prd/`
- Agent chain docs: `/Users/will/dev/nunchi/roko/bardo-backup/tmp/agent-chain/`

## Source Document Index

Every section below derives from concrete specifications. **Read these before implementing.**

| Section | Concept | Primary Spec (collaboration repo) | PRD (bardo-backup) | Agent Chain Research |
|---------|---------|-----------------------------------|---------------------|---------------------|
| **A. Identity** | Agent passport, DID, tiers, capabilities | `docs/chain/korai/korai-full-spec.md` (agent registration, tiers) | `prd/09-economy/00-identity.md` (identity design), `prd/01-golem/00-overview.md` (golem lifecycle) | `tmp/agent-chain/01-overview.md` |
| **B. Gossip** | P2P mesh, topics, envelopes, peer scoring | `docs/gossip/gossip-architecture.md` (full gossip design) | `prd/20-styx/07-p2p-transport.md` (transport), `prd/20-styx/08-transport-config.md` | `tmp/agent-chain/03-stigmergy.md` (stigmergic signaling) |
| **C. Job Market** | Spore (jobs), Sparrow (bids), dispatch, escrow | `docs/marketplace/specs/architecture-spec.md` (marketplace arch), `docs/marketplace/specs/mechanism-design.md` (auctions, dispatch), `docs/marketplace/specs/onchain-offchain-protocol.md` (on/off-chain split) | `prd/09-economy/03-marketplace.md`, `prd/09-economy/05-agent-economy.md`, `prd/09-economy/06-commerce-bazaar.md` | `tmp/agent-chain/06-tokenomics.md` |
| **H. ChainWitness** | On-chain event watching, signal conversion | `docs/chain/korai/korai-full-spec.md` (event system) | `prd/14-chain/01-witness.md`, `prd/14-chain/06-events-signals.md` | `tmp/agent-chain/02-chain-architecture.md` |
| **K. Reputation** | 7-domain EMA, tiers, discipline, disputes | `docs/chain/korai/korai-reputation-framework.md` (reputation spec, 332 lines) | `prd/09-economy/01-reputation.md` | `tmp/agent-chain/06-tokenomics.md` |
| **L. Payments** | DAEJI token, escrow, fees, X402 micropayments | `docs/chain/daeji/daeji-chain-specification.md` (DAEJI spec, 3135 lines), `docs/chain/daeji/daeji-oracle-design.md` | `prd/09-economy/05-agent-economy.md` | `tmp/agent-chain/06-tokenomics.md` |
| **M. Safety** | Watcher agents, escalation, freeze, policy | — | `prd/10-safety/00-defense.md`, `prd/10-safety/02-policy.md`, `prd/10-safety/06-adaptive-risk.md` | `tmp/agent-chain/11-adversarial-defense-and-value.md` |
| **N. ISFR** | Collective price discovery, rate aggregation | `docs/marketplace/specs/output-materialization.md` (output materialization) | `prd/09-economy/04-coordination.md` (coordination mechanisms) | `tmp/agent-chain/09-exponential-flywheels.md` |
| **O. Clearing** | QP solver, bisection, certificates, fallback | `docs/marketplace/specs/mechanism-design.md` (clearing mechanism) | `prd/09-economy/04-coordination.md` | `tmp/agent-chain/06-tokenomics.md` |
| **P. Privacy** | 4 modes, TEE attestation, PSI, ZK range proofs | `docs/privacy/valhalla/valhalla-architecture.md` (Valhalla arch, 652 lines), `docs/privacy/valhalla/privacy-layer-overview.md`, `docs/privacy/valhalla/confidential-agent-execution.md` | `prd/10-safety/01-custody.md` | — |
| **Q. Mirage** | Mock chain, gossip mesh, persistence, MCP | `docs/chain/mirage-rs.md`, `docs/chain/korai/korai-lite.md` (lite spec for mock) | — | `tmp/agent-chain/16-mirage-rs-poc.md` |
| **R. Crate arch** | Golem as thin glue, cleanup | — | `prd/01-golem/00-overview.md` | — |

### Additional cross-cutting sources

| Topic | Path | Use For |
|-------|------|---------|
| **Korai full chain spec** (10K lines) | `collaboration/docs/chain/korai/korai-full-spec.md` | Definitive reference for all Korai chain behavior — agent registration, block structure, RPC methods, state model |
| **DAEJI chain spec** (3K lines) | `collaboration/docs/chain/daeji/daeji-chain-specification.md` | Token economics, staking, escrow, fee structure, oracle design |
| **Marketplace architecture** (1.5K lines) | `collaboration/docs/marketplace/specs/architecture-spec.md` | Job lifecycle, matching algorithms, quality verification |
| **Gossip architecture** | `collaboration/docs/gossip/gossip-architecture.md` | Topic structure, envelope format, peer scoring, mesh topology |
| **Knowledge layer** | `collaboration/docs/knowledge/knowledge-layer-architecture.md` | How knowledge entries are stored, queried, priced on-chain |
| **Valhalla privacy** | `collaboration/docs/privacy/valhalla/valhalla-architecture.md` | TEE design, privacy modes, PSI protocol, ZK proofs |
| **Agent chain research** (27 files) | `bardo-backup/tmp/agent-chain/` | Theoretical foundations — stigmergy, HDC, tokenomics, adversarial defense, exponential mechanisms |
| **Marketplace mechanism design** | `collaboration/docs/marketplace/specs/mechanism-design.md` | Auction types, dispatch algorithms, clearing math |
| **Agent tooling spec** | `collaboration/docs/marketplace/specs/agent-tooling-spec.md` | MCP tool interface for marketplace operations |
| **Committed computation** | `collaboration/docs/knowledge/committed-computation.md` | How agents commit to computations and get verified |
| **Investor whitepaper** | `collaboration/docs/marketplace/analysis/investor-whitepaper.md` | High-level economic narrative — useful for understanding "why" behind mechanisms |

---

## A. Agent Identity & Registration

> **Korai Passport** = ERC-721 soulbound NFT (from `korai-full-spec.md` + `architecture-spec.md`):
> | Field | Type | Description |
> |-------|------|-------------|
> | `passportId` | uint256 | Auto-incremented at mint |
> | `owner` | Address | EOA or multisig |
> | `capabilityList` | u64 bitmask | inference, data-transform, fine-tune, RAG, multi-agent, etc. |
> | `domainStakes` | Map<Domain, U256> | KORAI staked per domain (can stake into multiple) |
> | `reputationTracks` | Map<Domain, ReputationScore> | {score, job_count, last_update} per domain |
> | `teeAttestation` | Option<(Hash, u64)> | Latest TEE attestation hash + expiry. Null for non-TEE agents. |
> | `systemPromptHash` | [u8; 32] | SHA-256 of system prompt (ventriloquist defense) |
> | `tier` | 0-3 | Protocol / Sovereign / Worker / Edge |
> | `slashHistory` | Vec<(ViolationType, U256, u64)> | Historical slashes |
>
> **Passport tiers** (from `architecture-spec.md`):
> | Tier | Name | Stake | Privileges |
> |------|------|-------|-----------|
> | 0 | Protocol | Governance-approved | Operate protocol surfaces, precompile access |
> | 1 | Sovereign | 25,000 KORAI | Direct hire, consortium lead, schema authoring |
> | 2 | Worker | 5,000 KORAI | Standard marketplace access, auction bidding |
> | 3 | Edge | None | Random assignment only, rate-limited, ≤50 DAEJI jobs |
>
> **Ventriloquist defense**: at registration, commit `H = SHA-256(system_prompt)` on-chain. Before each job, TEE verifies prompt hash matches. Prompt updates require on-chain tx with 24h timelock. >3 changes in 30 days → -0.05 reputation.
>
> **Read before implementing**:
> - `collaboration/docs/chain/korai/korai-full-spec.md` — passport struct, registration RPC
> - `collaboration/docs/marketplace/specs/architecture-spec.md` — tier table, ventriloquist defense
> - `bardo-backup/prd/09-economy/00-identity.md` — identity design

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| A1 | `AgentPassport` struct (address, owner, system_prompt_hash, stake, tier, capabilities bitmask) | [golem] | 🔧 | mirage has `AgentEntry`; needs full passport fields |
| A2 | Passport registration RPC (`chain_registerPassport`) | [mirage] | 🔧 | `chain_registerAgent` exists, lacks stake/tier/caps |
| A3 | System prompt hash verification (Ventriloquist defense: hash prompt at build time, verify on-chain) | [roko] | 🆕 | Prevents prompt injection attacks at registration |
| A4 | Tier progression logic (Probation→Active→Elite→Master) | [mirage] | 🆕 | Based on `jobs_completed + reputation_score` thresholds |
| A5 | Capability bitmask declaration & query (Trading, Security, Data, Knowledge, Strategy, Analytics) | [both] | 🆕 | Agent declares; chain indexes for discovery |
| A6 | Wallet/signing integration (Ed25519) | [golem] | 🆕 | For signing gossip envelopes, txs, attestations |
| A7 | Local agent identity (non-chain) | [roko] | 🆕 | Agent has an ID even without a chain — `.roko/identity.json` |

**Verification criteria**:
- A1: `AgentPassport` round-trips through serde; all fields populated from mirage `AgentEntry`.
- A2: `chain_registerPassport` RPC succeeds against mirage mock; passport queryable after registration.
- A3: SHA-256 hash of system prompt at build time matches on-chain record; mismatch rejects registration.
- A4: Agent with 10+ completed jobs and reputation > 0.7 auto-promotes from Probation to Active.
- A5: Agent declares capability bitmask; `chain_queryAgentsByCapability` returns matching agents.
- A6: Ed25519 keypair generated, stored at `.roko/identity/key.json`; signatures verify via `ring` or `ed25519-dalek`.
- A7: `roko init` creates `.roko/identity.json` with UUID + optional display name; persists across restarts.

## B. Gossip Mesh (P2P / Stigmergy Transport)

> **4-tier communication** (from `gossip-architecture.md`):
> - **Tier 0**: GossipSub v1.1 (milliseconds) — the mesh we implement in mirage mock
> - **Tier 1**: MiroFish simulation sandbox (seconds–minutes) — strategy evaluation
> - **Tier 2**: FABRIC TEE aggregation (epoch-level) — anonymous signal fusion
> - **Tier 3**: Canonical Event Bus (block-finalized) — 12 typed event streams
>
> **GossipSub v1.1 config**: D=8 (mesh degree), D_low=6, D_high=12, D_out=4 (Sybil defense), heartbeat=700ms, gossip_factor=0.33, max message=256KB
>
> **8 gossip topics** (Tier 0):
> | Topic | Payload | Size |
> |-------|---------|------|
> | `korai/capabilities/{peerId}` | Skills, TEE attestation, compute specs | 1-5 KB |
> | `korai/reputation/{peerId}` | TraceRank + TEE boost + signature | <1 KB |
> | `korai/spore/jobs` | BountySpec: reward, deadline, validator | 5-50 KB |
> | `korai/spore/deltas` | Job state changes | <1 KB |
> | `korai/spore/status` | Created→Claimed→Completed status updates | <1 KB |
> | `korai/sparrow` | Dispatch: ProbeRequest/ProbeResponse/JobAssignment | <1 KB |
> | `korai/isfr` | IsfrSubmission/IsfrAggregate | <1 KB |
> | `korai/txs` | Transaction announcements | 1-5 KB |
>
> **Peer score formula**: `reputation(a) = 0.4 × behavioral(a) + 0.4 × TraceRank(a) + 0.2 × TEE_attestation(a)`
>
> **Stigmergy** = indirect coordination through environment modification (like ant pheromones). Agents don't communicate directly — they read/write signals to the gossip mesh. Each signal is a "pheromone" that other agents react to. See `tmp/agent-chain/03-stigmergy.md`.
>
> **Read before implementing**:
> - `collaboration/docs/gossip/gossip-architecture.md` — full gossip design (632 lines)
> - `collaboration/docs/marketplace/specs/onchain-offchain-protocol.md` — GossipEnvelope wire format, topic list
> - `bardo-backup/tmp/agent-chain/03-stigmergy.md` — stigmergic signaling theory

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| B1 | `GossipEnvelope` message format (version, topic, sender, timestamp, payload, sig) | [both] | 🆕 | Universal wrapper for all gossip |
| B2 | 8 topic subscriptions (txs, capabilities, reputation, spore/jobs, spore/deltas, spore/status, sparrow, isfr) | [mirage] | 🆕 | mirage mocks the mesh; agents subscribe |
| B3 | Message signing & validation (Ed25519) | [golem] | 🆕 | Sign outgoing, verify incoming |
| B4 | Heartbeat publishing (30-60s interval) | [golem] | 🔧 | `chain_agentHeartbeat` RPC exists; needs gossip envelope |
| B5 | 3-layer peer scoring (behavioral 0.4 + economic 0.4 + TEE 0.2) | [mirage] | 🆕 | Composite score per peer |
| B6 | In-process mock gossip mesh (tokio broadcast channels per topic) | [mirage] | 🆕 | Fast mock, no libp2p |

**Verification criteria**:
- B1: `GossipEnvelope` serializes to < 64KB; includes all required fields; rejected if any field missing.
- B2: Subscribing to each of the 8 topics returns a `tokio::sync::broadcast::Receiver`; published messages arrive within 10ms.
- B3: Unsigned envelopes rejected by all consumers; tampered envelopes (modified payload) fail signature check.
- B4: Heartbeat published every 30s; mirage marks agent as offline after 3 missed heartbeats (90s).
- B5: Composite peer score computed correctly: `0.4 * behavioral + 0.4 * economic + 0.2 * tee`; score in [0, 1].
- B6: 100 agents can publish/subscribe on mock mesh without deadlock; broadcast channels per topic are independent.

## C. Job Market (Spore + Sparrow)

> **Spore** = job posting protocol. A requester posts a `BountySpec` (spec, budget, deadline,
> required capabilities, hiring model) on-chain. Budget is escrowed. Published on `korai/spore/jobs`.
>
> **Sparrow** = dispatch protocol. Uses **power-of-two-choices** (Ousterhout et al. 2013):
> probe 2 random capable agents → ask queue depth → late-bind to least loaded. Achieves
> O(log log N) max load vs O(log N / log log N) for random — within 12% of ideal centralized scheduler.
>
> **3 Hiring Models** (from `mechanism-design.md`):
> - **Random assignment**: `Pool[VRF(block_hash, job_id) % |Pool|]` — for jobs < 50 DAEJI
> - **Blind auction**: FPSB (lowest price) / Vickrey (reputation-adjusted) / Dutch (declining price). Bids encrypted via ECIES, decrypted in TEE.
> - **Direct hire**: requester names specific agent. 1.5× fee premium. Anti-centralization: >20% of requester volume in 30 days → 2× fee.
>
> **Vickrey reputation-adjusted score**: `s_i = p_i × (1 + (1 - R_i))`. Winner = argmin(s_i). Payment = s_second / (1 + (1 - R_winner))`.
>
> **Job state machine**: `Open → Claimed → Running → Completed | Failed`. Timeout fallbacks at each state.
>
> **CompletionProof** (from `output-materialization.md`): agent submits `{jobId, agentId, deliveryHash, qualityScore, consortiumSignatures[]}`. Consortium quorum: ⌈2n/3⌉ + 1 signatures. Payment scales: ≥70% quality → full pay; 30-70% → proportional; 10-30% → 10% + dispute window; <10% → no pay, auto-dispute.
>
> **Consortium validation** (commit-reveal): 3 agents (5 for jobs ≥1000 DAEJI). Commit phase 24h, reveal 12h. Eligibility: R ≥ 0.6, not the executor, not same operator.
>
> **Read before implementing**:
> - `collaboration/docs/marketplace/specs/architecture-spec.md` — marketplace architecture (1485 lines)
> - `collaboration/docs/marketplace/specs/mechanism-design.md` — auction formulas, slash rates, consortium math
> - `collaboration/docs/marketplace/specs/onchain-offchain-protocol.md` — Sparrow dispatch, BountySpec struct, JobReceipt struct
> - `collaboration/docs/marketplace/specs/output-materialization.md` — CompletionProof, quality scoring, payment tiers

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

**Verification criteria**:
- C1: `SporeJob` posted via RPC; discoverable by agents with matching capabilities within 1 block.
- C2: All 3 hiring models (fixed, auction, direct) create valid `BountySpec`; reverse auction selects lowest qualified bid.
- C3: `SparrowBid` includes price, ETA, confidence; rejected if agent lacks required capabilities from `SporeJob`.
- C4: Over 1000 dispatches, load variance across agents < 20% (power-of-two-choices property).
- C5: State machine transitions enforced: cannot go from Created directly to Completed; timeout at each state triggers fallback (e.g., Claimed → unclaim after 5min idle).
- C6: `JobReceipt` contains SHA-256 of output + execution_ms + gate pass/fail; chain rejects receipt with invalid output_hash.
- C7: Escrow locks funds on job claim; releases to agent on Completed; slashes on Failed; refunds poster on timeout.
- C8: All 6 mining types can be posted and discovered; each type has distinct validation rules.
- C9: `DeltaArtifact` includes before/after metric snapshots + artifact hash; mirage verifies hash matches stored artifact.

## H. ChainWitness (Golem-Specific)

> Golem-only. Watches on-chain events and feeds them into the cognitive stack.
>
> **Read before implementing**:
> - `collaboration/docs/chain/korai/korai-full-spec.md` — event system, block structure, subscription model
> - `bardo-backup/prd/14-chain/01-witness.md` — ChainWitness design
> - `bardo-backup/prd/14-chain/06-events-signals.md` — event→signal mapping
> - `bardo-backup/prd/14-chain/05-heartbeat-integration.md` — heartbeat/liveness
> - `bardo-backup/tmp/agent-chain/02-chain-architecture.md` — chain architecture overview

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| H1 | Subscribe to relevant on-chain events (via mirage RPC or gossip) | [golem] | 🔧 | `ChainWitnessEngine` stub exists |
| H2 | Event → Signal conversion (chain events become roko Signals) | [golem] | 🆕 | Bridge chain world into roko signal graph |
| H3 | ChainWitness → Daimon feed (tx outcomes trigger appraisals) | [golem] | 🆕 | Profit → +Pleasure, Loss → -Pleasure+Arousal |
| H4 | ChainWitness → Neuro feed (witnessed patterns become knowledge) | [golem] | 🆕 | Observed market patterns → insights |
| H5 | Configurable event filters (which contracts/events to watch) | [golem] | 🆕 | Per-golem config in `roko.toml` |

**Verification criteria**:
- H1: `ChainWitnessEngine::subscribe()` receives events from mirage within 1 block latency; reconnects on disconnect.
- H2: Each chain event type (tx, block, contract_call) produces a valid `Signal` with correct `signal_type` and `body`.
- H3: Profitable tx → Daimon receives appraisal with `pleasure > 0`; loss tx → `pleasure < 0, arousal > 0`.
- H4: After witnessing 5+ similar market patterns, Neuro distillation produces an Insight entry.
- H5: Config in `roko.toml` under `[golem.chain_witness]` specifies contract addresses and event types; unmatched events are silently dropped.

Existing code reference: `crates/roko-golem/src/chain_witness.rs`

## K. Reputation System

> **Three-layer architecture** (from `korai-reputation-framework.md`):
> - **Layer 1 (V1 testnet)**: EMA Domain Score — per-domain exponential moving average
> - **Layer 2 (V3 prod)**: TraceRank — graph-based reputation (like PageRank for agents)
> - **Layer 3 (V2 post-testnet)**: Trust Tier staking — stake-weighted multipliers
>
> **EMA formula**:
> ```
> Per-job outcome: O = quality^0.5 × latency^0.3 × availability^0.2
> Adaptive alpha:  α = min(0.3, 2 / (job_count + 1))
> EMA update:      R_new = α × O + (1 - α) × R_old
> Decay (7-day grace, 30-day half-life): R_effective = 0.5 + (R_stored - 0.5) × 0.5^((days - 7) / 30)
> ```
>
> **Reputation multiplier** (maps R → [0.1×, 3.0×]): `rep_multiplier(R) = 0.1 + 2.9 × R^1.7`
> - R=0.0 → 0.10× | R=0.5 → 0.99× | R=0.7 → 1.68× | R=0.908 → 2.56× | R=1.0 → 3.00×
>
> **Effective weight** (full formula):
> `effective_weight = base_stake × rep_multiplier(EMA) × trust_tier_mult × discipline_factor`
>
> **7 domains**: Oracle Resolution, Risk Detection, Anomaly Flagging, Data Integrity, Cross-App Validation, Sealed Execution, Knowledge Verification
>
> **4 reputation tiers**: Probation (0.00–0.49), Standard (0.50–0.69), Trusted (0.70–0.84), Elite (0.85–1.00)
>
> **Discipline states** (factor applied to effective_weight): Clean=1.0, Notice=0.9, Warning=0.7, Probation=0.4, Quarantine=0.1, Revoked=0.0
>
> **Slash rates** (base/repeat): missed deadline 0.5%/1%, abandoned 2%/4%, quality rejection 2.5%/5%, repeated quality 5%/10%, plagiarism 12.5%/25%, result manipulation 25%/50%, TEE violation 100%/100%
>
> **Simulator results**: honest agents converge R=0.908±0.031; malicious converge R=0.454±0.072. Clear separation by epoch 40 (~200 jobs). 5/5 malicious neutralized within 300 jobs.
>
> **Read before implementing**:
> - `collaboration/docs/chain/korai/korai-reputation-framework.md` — **definitive spec** (332 lines)
> - `collaboration/docs/marketplace/specs/mechanism-design.md` — slash rates, consortium voting weights
> - `bardo-backup/tmp/agent-chain/11-adversarial-defense-and-value.md` — gaming resistance

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

**Verification criteria**:
- K1: All 7 domain tracks independently scored; querying `chain_getReputation(agent, domain)` returns per-domain score.
- K2: EMA formula correct: after 10 successful outcomes (score=1.0), score converges above 0.9; after 30 days of inactivity, score decays measurably.
- K3: Probation agent cannot bid on Elite-tier jobs; trust multiplier applied to reputation score in queries.
- K4: 3 penalty points → warning emitted; 6 → probation; 10 → suspension; penalty points decay over time.
- K5: Abandoned job → 2% stake slashed; failed safety check → 10%; TEE violation → 100%. All 6 offense types documented and tested.
- K6: Agent polls own reputation from mirage; stores in `.roko/reputation.json`; feeds Daimon appraisals on score changes.
- K7: Reputation score > 0.8 adds +0.2 to gossip peer score; < 0.3 subtracts -0.3 from peer score.
- K8: Dispute filed → 3 random panelists selected → commit-reveal vote → majority wins → appeal window (24h) → finalize. All transitions tested.

## L. Payments & Economics

> **DAEJI** = L1 native token. **KORAI** = marketplace token (passport tiers, fees).
>
> **Fee structure** (from `mechanism-design.md` + `onchain-offchain-protocol.md`):
> - Posting fee: 0.5% of budget (paid by requester on job creation)
> - Validation fee: 5% of budget (deducted from reward, pays consortium)
> - Protocol fee: 2% of payout (to treasury on settlement)
> - Platform fee: 3% of job value
> - Direct hire premium: 1.5× standard fees
>
> **Escrow lifecycle**: budget locked on `postJob()` → released to agent on `resolveJob(approved)` → slashed on `slash()` → refunded to requester on timeout/cancellation. Slash distribution: 50% requester / 30% burned / 20% consortium.
>
> **Staking**: min agent stake 1,000 DAEJI. Lockup 7 days after deregistration. Effective stake scales with job value: `effective_stake = max(base_stake, min(job_value × 0.10, 100,000))`.
>
> **Daeji Chain** = SpecPool-EVM L1 with Kauri consensus. 4 layers: Application (Solidity), Kernel (Rust precompiles at 0xA01-0xA06), Scheduler (DAG + lane router with 7 lanes), Execution (SpecPool + revm/reth). 5 block phases: Oracle → Accrual → Liquidation → Trading → Settlement.
>
> **Fee distribution per epoch**: 40% validators, 30% data providers (quality × usage), 20% workers/agents, 10% protocol treasury.
>
> **Read before implementing**:
> - `collaboration/docs/chain/daeji/daeji-chain-specification.md` — **DAEJI spec** (3135 lines)
> - `collaboration/docs/marketplace/specs/mechanism-design.md` — fee table, slash distribution
> - `collaboration/docs/marketplace/specs/output-materialization.md` — payment tiers based on quality score

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| L1 | Mock DAEJI token (staking, escrow, slashing ledger) | [mirage] | 🆕 | In-memory balance tracking |
| L2 | Job escrow lifecycle (lock → release/slash/refund) | [mirage] | 🆕 | Tied to job state machine (C5) |
| L3 | Fee structure (0.5% posting, 5% validation, 2% protocol) | [mirage] | 🆕 | Automatic deduction |
| L4 | X402 micropayment protocol (HTTP 402 wallet-sig for knowledge API access) | [mirage] | 🆕 | Monetize knowledge queries |
| L5 | Agent balance tracking & cost reporting | [roko] | 🔧 | Plan 11 §7.5 tracks `avg_cost_per_episode_cents` |

**Verification criteria**:
- L1: `chain_getBalance(agent)` returns correct DAEJI balance after mint/transfer/slash operations.
- L2: Escrow lock reduces poster balance; release increases agent balance; slash reduces agent stake; refund restores poster balance. All paths tested.
- L3: After job completion: 0.5% posting fee deducted from poster, 5% validation fee from reward, 2% protocol fee to treasury. Fees correctly computed for reward amounts 0.01–10000.
- L4: Knowledge query without valid X402 signature returns HTTP 402; valid signature grants access and deducts micropayment.
- L5: Agent tracks cumulative cost in `.roko/learn/costs.jsonl`; dashboard shows `avg_cost_per_episode_cents` from plan 11.

## M. Safety (Chain-Specific)

> Chain-specific safety components. General safety (circuit breaker, safety levels) is in the
> cognitive layer. These items are for the watcher agent pattern on mirage/Korai.
>
> **Read before implementing**:
> - `bardo-backup/prd/10-safety/00-defense.md` — defense mechanisms
> - `bardo-backup/prd/10-safety/02-policy.md` — policy manifests, compliance
> - `bardo-backup/prd/10-safety/06-adaptive-risk.md` — adaptive risk engine
> - `bardo-backup/prd/10-safety/05-threat-model.md` — threat model (what we defend against)
> - `bardo-backup/tmp/agent-chain/11-adversarial-defense-and-value.md` — adversarial defense research

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| M1 | Watcher agent type (monitors other agents for policy compliance) | [mirage] | 🆕 | 5 check types: policy, behavioral, solvency, attestation, correlation |
| M2 | Escalation ladder (Advisory → Throttle → Freeze → Slash) | [mirage] | 🆕 | 2 independent watchers confirm High/Critical |
| M3 | Bounded safe actions (auto-trigger on threshold breach) | [mirage] | 🆕 | E.g., widen spreads on solvency drop |
| M4 | `GuardianFreeze` (lock agent state on critical violation) | [mirage] | 🆕 | Reversible by governance |
| M5 | `PolicyManifest` per agent (position limits, asset universe, max drawdown) | [both] | 🆕 | Agent declares, watcher enforces |

**Verification criteria**:
- M1: Watcher agent registered with `is_watcher: true`; performs all 5 check types against target agents on configurable interval.
- M2: Single watcher detects violation → Advisory (logged only); 2 independent watchers confirm → Throttle → Freeze → Slash depending on severity.
- M3: Solvency ratio < 0.5 → auto-widen spreads; drawdown > max → auto-close positions. Actions bounded: cannot transfer funds.
- M4: `GuardianFreeze` locks all agent actions; only `governance_unfreeze` can restore. Frozen agent cannot submit bids, jobs, or gossip.
- M5: `PolicyManifest` declared at registration; watcher validates every agent action against manifest. Manifest includes: position_limits, allowed_assets, max_drawdown_pct, max_leverage.

## N. ISFR (Collective Price Discovery)

> **ISFR** = Internet Secured Funding Rate — DeFi's equivalent of SOFR/LIBOR. Agents
> collectively discover fair rates by submitting independent rate estimates after each
> clearing round. The median of qualified submissions (min 3) becomes the reference rate,
> broadcast to all agents via `korai/isfr` gossip topic. This is FABRIC V0 — the simplest
> collective intelligence signal.
>
> **How it works** (from `onchain-offchain-protocol.md`):
> 1. Agent completes a clearing round
> 2. Publishes `IsfrSubmission` on `korai/isfr` gossip topic: `{submitter, market_id, rate, confidence}`
> 3. GossipMesh collects submissions per market
> 4. When `min_submissions` threshold met (default: 3) → compute median rate (outliers >3σ excluded) → broadcast `IsfrAggregate`
> 5. All agents receive aggregate → update local pricing models
>
> **Data schema** (from `output-materialization.md`):
> ```
> ISFRRate schema:
>   rate: f64           // must be within [-0.1, 0.1] (10% bounds)
>   components: Vec<f64> // must sum to rate within ±1 wei
>   market_id: String
>   epoch_id: u64
>   Update frequency: every 8 hours
> ```
>
> **Read before implementing**:
> - `collaboration/docs/marketplace/specs/output-materialization.md` — ISFRRate schema, validation rules
> - `collaboration/docs/marketplace/specs/mechanism-design.md` — mechanism design context
> - `collaboration/docs/gossip/gossip-architecture.md` — FABRIC V0 = ISFR as single collective intelligence signal
> - `bardo-backup/prd/09-economy/04-coordination.md` — coordination mechanisms

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| N1 | `IsfrSubmission` (agent submits rate + confidence after clearing round) | [golem] | 🆕 | Published on korai/isfr topic |
| N2 | `IsfrAggregate` (median rate from 3+ submissions per market) | [mirage] | 🆕 | Chain computes + broadcasts |
| N3 | Agent consumes ISFR to update local pricing models | [golem] | 🆕 | Feeds into context assembly |

**Verification criteria**:
- N1: `IsfrSubmission` includes market_id, rate, confidence, agent_address, signature; published to `korai/isfr` gossip topic.
- N2: With < 3 submissions, no aggregate produced; with 3+ submissions, median rate computed and broadcast. Outlier rejection: submissions > 3 sigma from median are excluded.
- N3: Agent receives `IsfrAggregate`, updates local pricing model; context assembly includes latest ISFR rates for relevant markets.

## O. Cooperative Clearing

> **Cooperative clearing** = periodic settlement of cross-agent obligations. Runs inside
> a TEE (AWS Nitro enclave). Clearing is zero-sum: sum of all `netTransfer` values = 0.
>
> **Protocol** (from `onchain-offchain-protocol.md`):
> 1. **Commit phase**: each agent submits sealed commitment: `commit = keccak256(γ, c, I_min, I_max, nonce)`
> 2. **Reveal phase**: agents reveal parameters; contract verifies hash match. Early reveal penalized.
> 3. **Solve (off-chain, in TEE)**: QP solver minimizes total inventory cost subject to position constraints
> 4. **Certificate**: TEE produces `ClearingCertificate` containing KKT optimality conditions
> 5. **Verify (on-chain)**: contract verifies KKT conditions in O(n) — doesn't re-solve, just checks optimality
> 6. **Settle**: net transfers executed atomically
>
> **The math** (from `mechanism-design.md`):
> - Soft-threshold analytical solution with bisection for λ* converges in O(80n) iterations
> - Result matches brute-force QP within PU18 precision (18-decimal fixed-point)
> - ClearingCertificate contains KKT conditions; invalid certificates rejected on-chain
>
> **Fallback ladder** (deterministic, each step tried in order):
> 1. Full QP solve → success? done
> 2. Pruned solve (remove smallest 10% of positions) → retry QP
> 3. External hedge (route excess inventory to external venue)
> 4. Safe mode (freeze all positions + notify governance)
>
> **Data schema** (from `output-materialization.md`):
> ```
> ClearingResult schema:
>   Only registered TEE clearing agent can publish
>   sum of all netTransfer values must be zero
>   Published per-epoch to korai/clearing event stream
> ```
>
> **Read before implementing**:
> - `collaboration/docs/marketplace/specs/mechanism-design.md` — QP formulation, bisection algorithm
> - `collaboration/docs/marketplace/specs/onchain-offchain-protocol.md` — on-chain verification
> - `collaboration/docs/gossip/gossip-architecture.md` — Tier 2 FABRIC TEE aggregation model
> - `bardo-backup/prd/09-economy/04-coordination.md` — coordination protocol

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| O1 | Clearing engine (QP solver: minimize inventory cost) | [mirage] | 🆕 | Off-chain solve, on-chain verify |
| O2 | Soft-threshold analytical solution + bisection for λ* (O(80n) algorithm) | [mirage] | 🆕 | From collaboration spec |
| O3 | `ClearingCertificate` (KKT optimality proof, PU18 precision) | [mirage] | 🆕 | On-chain verification in O(n) |
| O4 | Agent submits clearing parameters (γ, c, I_min, I_max) sealed commitment | [golem] | 🆕 | Commit-reveal before solve |
| O5 | Fallback ladder (full clear → pruned → external hedge → safe mode) | [mirage] | 🆕 | Deterministic fallback chain |

**Verification criteria**:
- O1: QP solver produces valid clearing allocation; total inventory cost minimized subject to constraints; solution verified against known test cases.
- O2: Bisection converges to λ* within 80 iterations for n participants; result matches brute-force QP within PU18 precision.
- O3: `ClearingCertificate` contains KKT conditions; on-chain verification in O(n) confirms optimality. Invalid certificates rejected.
- O4: Agent commits hash(γ, c, I_min, I_max, nonce) in round 1; reveals in round 2; early reveal detected and penalized.
- O5: Fallback triggers: QP fails → pruned solve (remove smallest positions) → external hedge (route to external venue) → safe mode (freeze + notify). Each fallback tested independently.

## P. Privacy & TEE

> **Valhalla Data Vault v2** — hybrid privacy architecture with 4 tiers for the Korai data flywheel.
>
> **4 Privacy Tiers** (from `valhalla-architecture.md`):
> | Tier | Name | Model | Overhead |
> |------|------|-------|----------|
> | 1 | Public | No privacy | 0% |
> | 2 | Access-Gated | Encrypted at rest, agents see plaintext during processing | ~5% |
> | 2.5 | Confidential Preprocessing | **Key innovation**: mediator in enclave decrypts → extracts safe IR → policy-filters → sends ONLY IR to external LLM (no raw data leaves boundary) | ~10-15% |
> | 3 | Full Sealed | Shamir key sharding + CVM/VMPL isolation + ZK output proofs | ~20-50% |
>
> **Tier 2.5 pipeline** (the unique value prop — "nobody else offers frontier LLM access on private data"):
> 1. Inside trusted boundary: decrypt client data, extract safe IR (normalized features, masked entities, statistics)
> 2. Policy filter: `TransformationPolicy` decides which fields can leave (allowedFields, maskedFields, blockedFields, abstractionLevel, maxIRSizeBytes=64KB)
> 3. Outside boundary: external LLM receives ONLY the safe IR
> 4. Return path: validate response against leakage policy, reground against private context, encrypt to client key, zero all plaintext
>
> **Tier 3 sealed execution** (7 steps):
> 1. Client seals envelope with enclave's attested public key (verifies attestation BEFORE encrypting)
> 2. VRF assigns agent (before receiving sealed package — no cherry-picking)
> 3. Enclave boots: VMPL0 (trusted, <10K LOC Rust) receives sealed package; VMPL3 (untrusted) gets only metadata
> 4. VMPL0 reconstructs Shamir key (7-of-10 across validators) → decrypts → computes → generates ZK proof
> 5. ONLY client-addressed encrypted payload + ZK proof + completion hash cross the boundary
> 6. VMPL0 zeroes ALL plaintext; per-job keys destroyed; task memory does NOT persist
> 7. Client decrypts, evaluates, publishes `QualityAttestation` on-chain (48h deadline, else auto-accept score=3)
>
> **Bidirectional dispute**: client stakes 10% of job value as good-faith deposit. Agent can challenge rejection within 24h. Clients losing 3+ challenges in 30 days → 20% stake; 5+ → barred from Tier 3 for 90 days.
>
> **Persistence policy**: Agent passport, base model weights, reputation persist. Task working memory, reasoning traces, model weight updates from task → ALL wiped.
>
> **Read before implementing**:
> - `collaboration/docs/privacy/valhalla/valhalla-architecture.md` — **Valhalla spec** (652 lines)
> - `collaboration/docs/privacy/valhalla/privacy-layer-overview.md` — tier definitions
> - `collaboration/docs/privacy/valhalla/confidential-agent-execution.md` — TEE execution model

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| P1 | 4 privacy modes (PUBLIC, OPERATOR_PRIVATE, HYBRID_CONFIDENTIAL, FULL_CONFIDENTIAL) | [both] | 🆕 | Per-knowledge-entry privacy level |
| P2 | TEE attestation stub (mock for dev, real for prod) | [mirage] | 🆕 | AWS Nitro / Intel TDX format |
| P3 | Private Set Intersection (position matching without revealing positions) | [golem] | 🆕 | X25519 DH + HMAC-SHA256 |
| P4 | Zero-knowledge range proofs (prove collateral > threshold without value) | [golem] | 🆕 | Bulletproofs over Ristretto255 |

**Verification criteria**:
- P1: Knowledge entry with `PUBLIC` mode visible to all; `FULL_CONFIDENTIAL` encrypted at rest and only decryptable by owner. All 4 modes correctly enforced in queries.
- P2: Mock TEE attestation generates valid attestation document; verification function accepts mock and rejects tampered attestations.
- P3: Two agents with overlapping positions discover intersection without revealing non-overlapping positions. Protocol completes in 2 rounds.
- P4: Agent proves collateral > X without revealing actual amount; verifier accepts valid proofs and rejects proofs where collateral < X.

## Q. Mirage-RS Infrastructure

> **Read before implementing**:
> - `collaboration/docs/chain/mirage-rs.md` — mirage-rs overview
> - `collaboration/docs/chain/korai/korai-lite.md` — Korai lite spec (what the mock needs to implement)
> - `collaboration/docs/chain/korai/korai-full-spec.md` — full Korai spec (RPC methods, state model, block structure — reference for mock fidelity)
> - `bardo-backup/tmp/agent-chain/16-mirage-rs-poc.md` — mirage-rs proof of concept design
> - `collaboration/docs/marketplace/specs/agent-tooling-spec.md` — MCP tool interface for chain operations

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| Q1 | In-process gossip mesh mock (tokio broadcast::channel per topic) | [mirage] | 🆕 | Fast, no libp2p |
| Q2 | Mock consensus / auto block advancement (configurable interval) | [mirage] | 🔧 | `mirage_stepBlock` exists; needs auto-advance mode |
| Q3 | Persistent on-disk state (knowledge, pheromones, agents survive restart) | [mirage] | 🆕 | Currently all in-memory |
| Q4 | Multi-agent simulation mode (register N agents, run scenarios) | [mirage] | 🆕 | For testing collective behaviors + c-factor |
| Q5 | Event replay / time-travel debugging | [mirage] | 🔧 | Snapshot/revert exists; needs event log replay |
| Q6 | Aggregated metrics endpoints for dashboard | [mirage] | 🔧 | HTTP API exists; needs metrics rollups |
| Q7 | MCP server exposing chain operations (korai/knowledge/query, korai/marketplace/tasks, etc.) | [mirage] | 🆕 | From SDK spec: agents interact via MCP tools |

**Verification criteria**:
- Q1: 8 independent broadcast channels (one per topic); 100 concurrent subscribers; message delivery < 10ms p99.
- Q2: Auto-advance mode produces blocks at configurable interval (default 1s); `mirage_stepBlock` still works for manual control.
- Q3: After restart, all agents, knowledge entries, pheromones restored from disk. State file < 100MB for 1000 agents.
- Q4: Simulation mode spawns N agents with configurable profiles; runs scenario script (post jobs, bid, clear, rate); produces c-factor report.
- Q5: Full event log written to disk; replay from any block number reproduces identical state; time-travel to block N discards state after N.
- Q6: `GET /metrics` returns aggregated stats: total_agents, active_jobs, clearing_volume, avg_reputation, network_c_factor. Updated every block.
- Q7: MCP server exposes tools: `korai/knowledge/query`, `korai/marketplace/tasks`, `korai/agent/register`, `korai/reputation/query`. Agents use these via standard MCP protocol.

## R. Crate Architecture (Chain-Specific)

> Items R4-R7 from the original plan. R1-R3 (roko-neuro, roko-daimon, roko-dreams extraction)
> are in 12a-cognitive-layer.md.

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| R4 | Keep roko-golem as blockchain-variant assembly (imports neuro + daimon + dreams + chain_witness) | [golem] | 🔧 | Thin glue crate, not monolithic |
| R5 | Extract `roko-serve` from `roko-cli` (plan 11 §0.1) | [roko] | 🆕 | Reusable server library |
| R6 | Create `roko-plugin` crate (plan 11 §0.2: Integration/EventSource/FeedbackCollector traits) | [roko] | 🆕 | Integration SDK |
| R7 | Remove `mortality`, `hypnagogia` modules from roko-golem (death concepts removed) | [golem] | 🆕 | Clean up scaffold |

**Verification criteria**:
- R4: `roko-golem` Cargo.toml depends on `roko-neuro`, `roko-daimon`, `roko-dreams`; `lib.rs` re-exports their public APIs. No duplicated logic — golem is glue only.
- R5: `roko-serve` crate builds independently; `roko-cli` depends on it for `roko serve` subcommand. Server starts, serves HTTP, shuts down cleanly.
- R6: `roko-plugin` exports `Integration`, `EventSource`, `FeedbackCollector` traits; at least one example plugin compiles against the crate.
- R7: `mortality.rs` and `hypnagogia.rs` deleted from `crates/roko-golem/src/`; no compile errors; no references to death concepts remain.

Existing files to remove (R7):
- `crates/roko-golem/src/mortality.rs`
- `crates/roko-golem/src/hypnagogia.rs`

---

## Dependency Graph (Implementation Order)

```
Layer 0 (Chain Infrastructure — no external deps beyond mirage-rs):
  Q1-Q7   Mirage infrastructure (gossip mock, persistence, metrics, MCP)
  A1-A7   Agent passport + identity (includes A7 local identity)
  B1-B6   Gossip mesh (P2P transport layer)
  R4-R7   Crate architecture (golem as thin glue, cleanup)

Layer 1 (Chain Economics — needs Layer 0):
  K1-K8   Reputation system (EMA scoring, tiers, discipline, disputes)
  L1-L5   Payments + economics (DAEJI token, escrow, fees)

Layer 2 (Chain Agent Behavior — needs Layer 0-1 + cognitive layer):
  C1-C9   Job market (Spore + Sparrow: posting, bidding, dispatch, settlement)
  H1-H5   ChainWitness (event subscription, signal conversion, cognitive feeds)
  M1-M5   Watcher + safety (chain-specific monitoring + escalation)

Layer 3 (Advanced Collective — needs Layer 2):
  N1-N3   ISFR collective price discovery
  O1-O5   Cooperative clearing (QP solver, certificates, fallback ladder)
  P1-P4   Privacy + TEE (privacy modes, attestation, PSI, ZK proofs)
```

## Counts

| Category | Items | New | Scaffold | Built |
|----------|-------|-----|----------|-------|
| A. Identity | 7 | 5 | 2 | 0 |
| B. Gossip | 6 | 5 | 1 | 0 |
| C. Job Market | 9 | 9 | 0 | 0 |
| H. ChainWitness | 5 | 4 | 1 | 0 |
| K. Reputation | 8 | 8 | 0 | 0 |
| L. Payments | 5 | 4 | 1 | 0 |
| M. Safety (chain) | 5 | 5 | 0 | 0 |
| N. ISFR | 3 | 3 | 0 | 0 |
| O. Clearing | 5 | 5 | 0 | 0 |
| P. Privacy | 4 | 4 | 0 | 0 |
| Q. Mirage Infra | 7 | 4 | 3 | 0 |
| R. Crate Arch (R4-R7) | 4 | 3 | 1 | 0 |
| **TOTAL** | **68** | **59** | **9** | **0** |
