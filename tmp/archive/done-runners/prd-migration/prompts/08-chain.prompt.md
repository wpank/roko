# Prompt: 08-chain

You are a fresh Claude Opus agent. Zero prior context. Read every file this prompt references before writing.

## Your mission

Generate `/Users/will/dev/nunchi/roko/roko/docs/08-chain/`. **This is the LARGEST topic.**
Covers Korai chain + mirage-rs + agent marketplace + HDC on-chain + KORAI/DAEJI tokenomics +
stigmergy theory + ChainClient/ChainWallet traits + chain intelligence + triage + the 6
Solidity contracts + exponential flywheels + Korai Passport (ERC-721 soulbound) + 4-tier
gossip + job market (Spore/Sparrow, 3 hiring models) + reputation framework (7-domain EMA)
+ clearing/settlement + Valhalla privacy (TEE/PSI/ZK) + x402 micropayments + chain agent
heartbeat mapping.

## Step 1 — Context pack (MANDATORY)

Read all 7 files in `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/context-pack/` in order.

## Step 2 — refactoring-prd canonical sources (READ IN FULL)

1. `/Users/will/dev/nunchi/roko/refactoring-prd/04-knowledge-and-mesh.md` — **full chain spec** (Korai, HDC on-chain, KORAI economics, ERC-8004, mesh, stigmergy, C-Factor, mirage-rs, backup/restore)
2. `/Users/will/dev/nunchi/roko/refactoring-prd/05-agent-types.md` §3 Chain Agent (9-step heartbeat mapping to universal loop, 3 custody modes)
3. `/Users/will/dev/nunchi/roko/refactoring-prd/09-innovations.md` §VI Collective Calibration (31.6× heuristic with caveats), §VIII x402 Micropayments, §XVI Knowledge Futures Market, §XIII Cross-Domain Resonance
4. `/Users/will/dev/nunchi/roko/refactoring-prd/07-implementation-priorities.md` §Tier 6 (Korai chain roadmap — deferred, blocked by Tier 5)
5. `/Users/will/dev/nunchi/roko/refactoring-prd/08-translation-guide.md`

## Step 3 — SOURCE-INDEX entry `## 08-chain.md`

Read every file. Key sources:
- All of `bardo-backup/prd/14-chain/` (00-architecture, 01-witness, 02-triage, 03-protocol-state, 04-chain-scope, 05-heartbeat-integration, 06-events-signals, 07-generative-views, 08-stream-api, 09-anomaly-detection)
- All of `bardo-backup/prd/15-dev/` (mirage-rs specs, debug UI)
- `bardo-backup/prd/01-golem/00-overview.md`
- `bardo-backup/prd/shared/chains.md`
- **All of `bardo-backup/tmp/agent-chain/`** (01-overview through 17-autonomous-eval-generation + README + research files: agent-chain-research2, agent-research2, context-quality-science, eval-research, exponential-mechanisms-research, proving-collective-intelligence, self-improvement-frameworks, harness-engineering) — **27 files**
- **All of `bardo-backup/tmp/agent-chain-new/`** (01-vision through 14-implementation) — 14 files
- All of `bardo-backup/tmp/hyperliquid/` (HyperEVM research)

## Step 4 — implementation-plans (CRITICAL — read full file)

- `roko/tmp/implementation-plans/12b-chain-layer.md` — **76 items, 11 sections**: Identity (Korai Passport, tiers, ventriloquist defense), Gossip (4-tier, 8 topics, GossipSub v1.1), Job Market (Spore/Sparrow, 3 hiring models, power-of-two-choices), ChainWitness, Reputation (7-domain EMA, tiers, disputes), Payments (DAEJI token, x402 micropayments), Safety, ISFR, Clearing (QP solver, bisection), Privacy (Valhalla TEE, PSI, ZK range proofs), Mirage, Crate cleanup
- `roko/tmp/implementation-plans/12-nunchi-integration.md` (historical split context)
- `roko/tmp/implementation-plans/10-golem-integration.md` (superseded but has context)

## Step 5 — active code

- Glob `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/**/*.rs`
- Glob `/Users/will/dev/nunchi/roko/roko/apps/mirage-rs/src/**/*.rs` (141 tests — major reference)
- Read key files from both.

## Step 6 — Output and sub-doc plan (LARGE TOPIC)

```bash
mkdir -p /Users/will/dev/nunchi/roko/roko/docs/08-chain
```

Write **25 sub-docs** plus `INDEX.md`. This topic is intentionally large — do not condense. Split further if needed.

| # | Filename | Content |
|---|---|---|
| 00 | `00-vision-and-framing.md` | Korai chain vision. Blockchain is ONE domain plugin, not the default. Three-level knowledge architecture: Korai Chain (global public) → Agent Mesh (peer/private) → Local Neuro Store (private). What goes on-chain vs off-chain. |
| 01 | `01-korai-chain-spec.md` | Korai mainnet: dedicated EVM for agent coordination, 400ms block time, agents as first-class citizens via ERC-8004. Daeji testnet. Block structure, state model, RPC methods. |
| 02 | `02-korai-token-economics.md` | KORAI token with 1% annual demurrage. DAEJI on testnet. Earning (registration mint, validated knowledge posting, confirmation, heartbeat, challenge defense). Spending (posting/anti-spam, querying, challenging). Quality incentives. |
| 03 | `03-hdc-on-chain-precompile.md` | 10,240-bit HDC vectors via native EVM precompile. ~400 gas for top-k=20 similarity search. 3-tier search (Bloom → approximate → exact). Same encoding used locally and on-chain — seamless transfer. Precompile is custom Korai feature, not mainnet Ethereum — needs benchmarking on Korai testnet. |
| 04 | `04-korai-passport-erc-721-soulbound.md` | Full struct from 12b §A: passportId, owner, capabilityList (u64 bitmask), domainStakes (Map<Domain, U256>), reputationTracks (Map<Domain, ReputationScore>), teeAttestation, systemPromptHash (ventriloquist defense), tier, slashHistory. 4 tiers: Protocol (governance-approved) / Sovereign (25K KORAI) / Worker (5K KORAI) / Edge (rate-limited, ≤50 DAEJI jobs). |
| 05 | `05-ventriloquist-defense.md` | At registration, commit `H = SHA-256(system_prompt)` on-chain. Before each job, TEE verifies prompt hash matches. Prompt updates require on-chain tx with 24h timelock. >3 changes in 30 days → -0.05 reputation. Prevents prompt injection attacks at registration. |
| 06 | `06-erc-8004-registries.md` | ERC-8004 identity (ERC-721 Agent Card with capabilities, endpoints, payment address), Reputation Registry (feedback authorization between agents; actual scoring off-chain), Validation Registry (agents request verification; validator contracts provide attestation; supports reputation, stake-secured re-execution, zkML proofs, TEE oracles). |
| 07 | `07-4-tier-gossip-architecture.md` | From 12b §B: Tier 0 GossipSub v1.1 (milliseconds) — the mesh mocked by mirage; Tier 1 MiroFish simulation sandbox (seconds-minutes) — strategy evaluation; Tier 2 FABRIC TEE aggregation (epoch-level) — anonymous signal fusion; Tier 3 Canonical Event Bus (block-finalized) — 12 typed event streams. GossipSub v1.1 config: D=8, D_low=6, D_high=12, D_out=4 (Sybil defense), heartbeat=700ms, gossip_factor=0.33, max message=256KB. |
| 08 | `08-eight-gossip-topics.md` | From 12b §B: korai/capabilities/{peerId}, korai/reputation/{peerId}, korai/spore/jobs, korai/spore/deltas, korai/spore/status, korai/sparrow, korai/isfr, korai/txs. Per-topic envelope size and payload structure. |
| 09 | `09-peer-scoring-3-layer.md` | Composite peer score: `reputation(a) = 0.4 × behavioral + 0.4 × TraceRank(a) + 0.2 × TEE_attestation(a)`. Score in [0, 1]. How each component is computed. |
| 10 | `10-spore-job-market.md` | Spore = job posting protocol. Requester posts BountySpec (spec, budget, deadline, required capabilities, hiring model) on-chain. Budget escrowed. Published on korai/spore/jobs. Job state machine: Open → Claimed → Running → Completed | Failed. Timeout fallbacks at each state. |
| 11 | `11-sparrow-power-of-two-choices.md` | Sparrow dispatch protocol: power-of-two-choices (Ousterhout et al. 2013). Probe 2 random capable agents → ask queue depth → late-bind to least loaded. Achieves O(log log N) max load vs O(log N / log log N) for random — within 12% of ideal centralized scheduler. |
| 12 | `12-three-hiring-models.md` | (a) Random VRF assignment: `Pool[VRF(block_hash, job_id) % |Pool|]` for jobs < 50 DAEJI. (b) Blind auction: FPSB (lowest price) / Vickrey reputation-adjusted / Dutch (declining price). Bids encrypted via ECIES, decrypted in TEE. (c) Direct hire: requester names specific agent. 1.5× fee premium. Anti-centralization: >20% of requester volume in 30 days → 2× fee. |
| 13 | `13-vickrey-reputation-auction.md` | Formula: `s_i = p_i × (1 + (1 - R_i))`. Winner = argmin. Payment = `s_second / (1 + (1 - R_winner))`. Truthfulness guarantee. Example calculation. |
| 14 | `14-reputation-system-7-domain.md` | From 12b §K: 7-domain EMA, tiers (Protocol/Sovereign/Worker/Edge), discipline, disputes. Per-domain stake. Slash history. Glicko-2 rating. Staking payoffs. |
| 15 | `15-chainwitness-event-watching.md` | From 12b §H: On-chain event watching, signal conversion, ABI decoder, event categorization. How chain events become Engrams. |
| 16 | `16-triage-curiosity-midas.md` | Chain intelligence. Curiosity scoring. MIDAS anomaly detection. Bayesian scoring. Block ingestion pipeline. |
| 17 | `17-chain-client-wallet-traits.md` | ChainClient and ChainWallet traits (`roko-chain`). 3 custody modes: Delegation (enclave keys), Embedded (ERC-4337 account abstraction), Local key (dev). Ed25519 keypair for signatures. |
| 18 | `18-mirage-rs-evm-simulator.md` | In-process EVM simulator. Korai proxy during development. JSON-RPC (full Ethereum RPC + custom `mirage_*` methods). Fork mode for mainnet Ethereum. Scenario engine. Copy-on-write branching. HDC precompile emulation. 141 tests. Port 8545. |
| 19 | `19-chain-agent-heartbeat.md` | From 05-agent-types.md §3: 9-step chain heartbeat maps to universal Synapse loop. OBSERVE (Substrate.query) → RETRIEVE (Scorer.score) → ANALYZE (Daimon cross-cut) → GATE (Router.select) → SIMULATE (mirage-rs pre-flight, domain-specific) → VALIDATE (PolicyCage, limits, Gate.verify) → EXECUTE (Agent.execute) → VERIFY (Gate.verify) → REFLECT (Policy.decide). The chain heartbeat adds SIMULATE and VALIDATE steps that don't exist in coding agents. |
| 20 | `20-x402-micropayments.md` | Coinbase x402 (Linux Foundation, AWS/Visa/Mastercard/Stripe). Per-API-call billing at < $0.001. Sub-second settlement (USDC on Base). Self-funding agent loop: KORAI earnings → USDC conversion → x402 compute → output → user pays agent → reinvest → accelerate. Agent-as-a-business. |
| 21 | `21-isfr-clearing-settlement.md` | ISFR (Intersubjective Fact Registry): collective price discovery, rate aggregation, disputed claim resolution, 3-arbitrator voting. Clearing: QP solver, bisection, certificates, fallback. Batch clearing, DVP, settlement finality. |
| 22 | `22-valhalla-privacy-layer.md` | From 12b §P: 4 privacy modes, TEE attestation, PSI protocol, ZK range proofs. Valhalla architecture. Confidential agent execution. |
| 23 | `23-knowledge-futures-market.md` | P3 deferred feature. On-chain escrow for committed knowledge production. Research agent publishes a Knowledge Future → operations agents purchase via x402 → escrow funds → delivery triggers release → non-delivery slashes stake. Predictive market for knowledge production. |
| 24 | `24-current-status-and-6-contracts.md` | 6 Solidity contracts: Agent Registry, Reputation, Marketplace, Escrow, KORAI token, Validation. Tier 6 deferred status (blocked by Tier 5 Agent Mesh). Solo agents and event-driven agents do NOT need the chain layer. Focus is Tiers 1-5 first. Capacity planning (400ms blocks × 2.5 blocks/sec × 1K agents × 1 entry/day = 1K entries/day fine; 10K+ needs capacity planning). |

Plus `INDEX.md`.

## Step 7-9 — Rules, INDEX, self-check

Per context-pack rules. **This topic is expected to be very large — 25 sub-docs × 500-1500 lines each = 12,000-37,500 total lines. DO NOT shrink this — the source material is dense.** Preserve every citation. Minimum 30 citation-like patterns.

Cross-reference topics 00-architecture, 06-neuro (HDC encoding shared), 13-coordination (stigmergy, mesh), 14-identity-economy (identity, reputation, x402), 11-safety (Valhalla privacy), 04-verification (chain gates).

## CRITICAL REMINDERS

- **THIS IS THE LARGEST TOPIC. DO NOT SHRINK IT. 25 sub-docs, not fewer.**
- DO NOT SUMMARIZE. DO NOT TRUNCATE. PRESERVE ALL 30+ CITATIONS.
- Blockchain is **one domain plugin**, not the default framing. Make this clear at the top of the INDEX and throughout.
- Rename: GNOS → KORAI/DAEJI; styx → Mesh; clade → collective/mesh; golem → agent; bardo → roko; mori → Roko Orchestrator.
- The 31.6× collective calibration is a **heuristic**, not a theorem. Explicit caveats.
- Tier 6 is **deferred** (blocked by Tier 5). Solo/event-driven agents don't need the chain layer.
- Use Write tool. Don't ask questions. Continue until all 25 sub-docs + INDEX exist.
