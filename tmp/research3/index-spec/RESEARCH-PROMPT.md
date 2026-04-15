# Research Prompt: Nunchi — Net-New Opportunities, Market Gaps, and Scaling Strategies

## Context for the researcher

You are conducting deep research for Nunchi, a company building three interlocking products:

1. **Korai** — A sovereign EVM L1 blockchain purpose-built for autonomous AI agents. 50ms blocks, native HDC (hyperdimensional computing) precompiles, on-chain agent identity via ERC-8004, knowledge registries, and a cooperative clearing engine inside TEE enclaves. Built on Commonwealth/Quora fork of reth/revm.

2. **Roko** — A Rust agent toolkit (18 crates, ~177K LOC) where agents build themselves. Self-hosting loop: read PRDs → generate plans → execute via LLM agents (Claude, OpenAI, Gemini, Ollama, Codex) → validate through gate pipelines → persist → learn. Integrates with IDEs via Agent Client Protocol (ACP). 9-layer system prompt builder, cascade model router, adaptive gates, dream consolidation, episode logging.

3. **ISFR (Implied Secured Funding Rate)** — "The SOFR of DeFi." A weighted-median benchmark index across four DeFi yield sources (Aave V3, Compound V3, Ethena sUSDe, ETH staking). Published on-chain via validator consensus. Used as the reference rate for yield perpetual contracts — a continuous-funding perp whose underlying is a yield rate, not an asset price. Addresses the $665T TradFi interest rate derivatives market which has almost no DeFi equivalent.

The attached source documents (01-10) contain exhaustive technical detail on each subsystem. The VC call analysis (00) shows how a recent investor conversation went — what resonated, what didn't, and what gaps exist.

---

## What I need you to research

### A. Net-New Innovations and Novel Applications

Research and identify **10-15 novel applications, use cases, or product ideas** that could be built on top of this stack (Korai chain + Roko agents + ISFR benchmarks) that are NOT already described in the source documents. For each, provide:

- What the product/application is
- Why agents + blockchain is uniquely suited (vs. centralized alternatives)
- Market size or addressable opportunity
- Who the users/buyers are
- What existing infrastructure from the Nunchi stack it leverages
- Competitive moat or defensibility
- Time to build (weeks/months/quarters)

Focus on things that are:
- **Genuinely novel** — not rehashes of existing DeFi products with "agent" slapped on
- **High-impact** — could meaningfully move revenue, users, or strategic positioning
- **Exponential return potential** — network effects, compounding, or flywheel dynamics
- **Good fit for the stack** — leverages what Nunchi has already built (agent runtime, chain, HDC vectors, benchmark infrastructure, TEE clearing, identity registries)

Specific areas to explore:
1. **Agent-to-agent financial services** — What financial products only make sense when both counterparties are autonomous agents?
2. **Knowledge markets beyond DeFi** — The IKQI (Internet Knowledge Quality Index) concept applied to specific verticals (legal, medical, engineering, security)
3. **Cross-chain agent coordination** — Agents operating across multiple chains, using Korai as the coordination/identity layer
4. **Enterprise agent infrastructure** — Corporate use cases for agent identity, audit trails, and verifiable execution
5. **Prediction markets with agent participants** — Markets where agents are both participants and market makers
6. **Developer tooling as a wedge** — Roko/ACP as a developer adoption path that feeds back into the chain (the "Google strategy")
7. **Regulatory technology** — Agent compliance, audit chains, explainability as a product
8. **Insurance and risk** — Agent-underwritten insurance, parametric products, risk assessment markets
9. **Labor markets** — Agent hiring, reputation, and task marketplaces beyond the current Spore/Sparrow design
10. **Physical world integration** — IoT, robotics, supply chain, energy markets with agent coordination

### B. Underserved Market Opportunities

Research markets where:
1. There is clear demand for automation/agents but no credible decentralized infrastructure
2. Existing solutions are centralized and extractive (high take rates, lock-in, opacity)
3. The Nunchi stack's specific capabilities (TEE clearing, benchmark indices, agent identity, HDC similarity search) create a structural advantage
4. The market is large enough to matter ($1B+ TAM) but niche enough that incumbents aren't paying attention

Specifically investigate:
- What are the fastest-growing segments of the AI agent market in 2025-2026?
- Which industries are adopting AI agents fastest and what infrastructure are they using?
- Where are the pain points in current agent deployment (identity, trust, coordination, payment)?
- What does the competitive landscape look like for "agentic chains" or "AI x crypto" projects? (Bittensor, Fetch.ai, SingularityNET, Autonolas, etc.) Where are the gaps?
- What are VCs funding in the AI x crypto intersection right now? What thesis are they buying?

### C. Scaling and Adoption Strategies

Research and propose:
1. **Go-to-market strategies** for an agentic blockchain — how do you get the first 100 agents on-chain? The first 1,000? The first 10,000?
2. **Developer adoption playbooks** — How did Ethereum, Solana, and Hyperliquid bootstrap their developer ecosystems? What can Nunchi learn?
3. **Market maker onboarding** — How do you convince market makers to run agents on a new chain? What incentives work? What's the pitch?
4. **The "benchmark business" playbook** — How did SOFR, VIX, and the S&P 500 index build credibility and adoption? What governance structures did they use? How long did it take?
5. **Academic-to-product pipeline** — How to turn the Brown (cooperative clearing) and USC (security templates) research into products that ship, not just papers
6. **Distribution through developer tools** — The ACP/IDE integration targets 50M JetBrains users. How do you convert developer tool users into chain participants? What's the funnel?
7. **Ecosystem incentive design** — Points, grants, hackathons, retroactive public goods funding — what actually works for bootstrapping a new chain ecosystem in 2026?

### D. VC Pitch Optimization

Based on the call analysis and the current state of the project:
1. **What are the 3-5 strongest "why now" arguments** for this project in the current market?
2. **What comparable exits or valuations** exist for projects in this category? (AI infra, DeFi infra, benchmark businesses)
3. **What metrics should Nunchi be tracking and presenting** to VCs at this stage?
4. **What proof points would be most compelling** to move from "interesting" to "investable"?
5. **How should Nunchi frame the competitive positioning** — as an L1, an agent platform, a benchmark business, or something else?
6. **What are the most common reasons VCs pass on AI x crypto projects** and how can Nunchi preemptively address them?
7. **What materials should be prepared before the next VC conversation?** (deck, one-pager, demo video, data room contents)

### E. Things to Prove and Build

For each of the following, suggest a concrete artifact, demo, or proof point that could be built in 1-4 weeks:
1. That ISFR is a credible benchmark (not just a technical implementation)
2. That agents on this chain produce measurably better outcomes than agents running independently
3. That the collective learning thesis works (agents get smarter from shared knowledge)
4. That yield perps have real demand (not just theoretical market sizing)
5. That the cooperative clearing mechanism produces better fills than continuous matching
6. That the developer tools (Roko, ACP) are compelling enough to attract non-crypto developers
7. That the chain performs under realistic adversarial conditions
8. That agent identity and reputation create real economic value

---

## Source Documents

The following documents are provided as context. They are self-contained and assume no prior knowledge of the project:

- `00-vc-call-analysis-ev-2026-04-30.md` — Analysis of today's VC call with Emergent Ventures (Praneeth Srikanti). What resonated, pushback received, gaps identified, materials to prepare.
- `01-isfr-index.md` — Complete ISFR specification: methodology, sources, computation, sub-indices, NRIS framework, V1→V2 evolution.
- `02-yield-perpetuals.md` — Yield perpetual instrument: funding/mark formulas, position semantics, AAVE backstop use case, $665T market thesis.
- `03-korai-blockchain.md` — Korai L1: consensus, ERC-8004 registries, precompiles, smart contracts, Hyperliquid relationship.
- `04-generalized-benchmarks.md` — BenchmarkIndex trait, all five indices (ISFR, IAPI, IKQI, ISVI, IRRI), NRIS framework.
- `05-tee-clearing.md` — TEE clearing engine: cooperative batch matching, KKT verification, security model.
- `06-roko-agent-runtime.md` — Roko toolkit: 18 crates, self-hosting loop, 7 LLM backends, 9-layer prompts, safety layer.
- `07-oracle-system.md` — Oracle infrastructure: validator computation, precompiles, prediction scoring, agent consumption.
- `08-agent-identity-economy.md` — Agent identity: ERC-8004 registries, hiring models, job marketplace, knowledge futures, reputation.
- `09-knowledge-context-engineering.md` — Knowledge system: neuro store, dreams, HDC vectors, collective intelligence, context assembly.
- `10-defi-integration.md` — DeFi stack: source protocols, smart contracts, agent-mediated trading, risk management, competitive landscape.
- `11-differentiation-prd.md` — "Coordination Primitives Ethereum and Rollups Cannot Run." Seven core primitives (P1-P7) that require Korai's architecture: browser/CLI light clients, big-chat agent control, DKG-private collaboration, proof-of-work-done, proof-of-learning, on-chain reputation. Plus eight extension primitives (V1-V8) for Q2-Q4. Includes investor kill lines and publication path.
- `12-dkg-agent-use-cases.md` — 29 net-new DKG-enabled agent capabilities (C13-C43) plus 3 Path 3 generalizations (P5-P7). Built on USC confidential-compute assessment. Covers private multi-agent collaboration, threshold-encrypted agent communication, confidential model inference, private reputation proofs, sealed-bid agent auctions, and 7 proposed precompile slots (0xA10-0xA1F agent-comm namespace). Surfaces 7 standout candidates with priority/effort matrix.
- `13-daeji-commonware-chain.md` — The actual chain implementation (Daeji), built from Commonware composable primitives (not a geth/reth fork). Covers: Simplex BFT consensus, BLS12-381 threshold signatures, dual-plane execution (native + EVM), 3 custom precompiles (HDC at 0x09, QMDB proofs at 0x0B, BTLE at 0x0C), 0xA10-0xA1F reserved agent namespace, sub-block-time agent gossip, DKG-based private collaboration, proof-of-work-done/proof-of-learning, knowledge layer redesign with NeuroChainSync. Current state: 4-node devnet running at ~400ms blocks, 10 Solidity contracts deployed. Open questions around token economics, knowledge storage strategy, and precompile boundaries.

---

### F. DKG and Differentiation Deep-Dive

Based on documents 11 and 12, research:
1. **DKG (Distributed Key Generation) for agents** — What other projects are using DKG/threshold cryptography for agent privacy? How mature is the field? What are the real-world applications beyond crypto?
2. **Confidential multi-agent coordination** — Where is private agent-to-agent communication a hard requirement (not just nice-to-have)? Healthcare, legal, financial, military, enterprise?
3. **The 7 core primitives (P1-P7)** — For each, research whether anyone else is building this and what the competitive landscape looks like
4. **The 29 DKG use cases (C13-C43)** — Which of these have the highest real-world demand right now? Which are most novel?
5. **Precompile-level privacy** — How does embedding DKG at the chain level compare to application-level solutions (Secret Network, Oasis, Aztec)?
6. **"Things Ethereum cannot run"** — What's the strongest version of this argument? What are the most compelling examples of coordination primitives that genuinely require a purpose-built chain?
7. **Enterprise adoption of confidential compute for AI** — What are enterprises actually deploying (TEEs, MPC, FHE, DKG)? What do they care about most (compliance, IP protection, competitive intelligence)?

### G. Commonware and Chain Primitive Research

Based on document 13, the Daeji chain is NOW RUNNING on a 4-node devnet (~400ms blocks). This makes the following research urgent and actionable:

1. **Commonware ecosystem** — What else is being built on Commonware? Who are the other teams? What primitives exist that Daeji isn't using yet? What's on the Commonware roadmap?
2. **What to build next on Commonware** — Given the 8 novel features already spec'd (BTLE, threshold VRF, agent gossip, DKG collaboration, proof-of-work-done, proof-of-learning, threshold reputation, light clients), what additional primitives would unlock the highest-value use cases? Prioritize by impact and feasibility.
3. **Simplex BFT competitive analysis** — How does Simplex BFT compare to other modern consensus protocols (CometBFT, Narwhal/Bullshark, Mysticeti, HotStuff-2)? What are the tradeoffs? Where does it excel for agent workloads specifically?
4. **Custom precompile design patterns** — What are the best practices for designing custom EVM precompiles? What gas cost models work? How do other chains (Avalanche, Arbitrum Stylus, Sei) handle custom precompiles?
5. **Agent-native chain features** — What chain-level features would make the biggest difference for autonomous agent coordination? Sub-block messaging? Native vector search? Threshold-encrypted shared state? Programmable transaction ordering?
6. **Application-specific chain vs. rollup tradeoffs** — When does building a sovereign L1 from primitives (Commonware approach) beat deploying as a rollup (on Ethereum, Celestia, etc.)? What are the adoption implications? What do VCs/developers prefer?
7. **QMDB and authenticated storage** — How does QMDB compare to other authenticated storage solutions (Merkle Patricia Trie, Verkle trees, Jellyfish Merkle Tree)? What are the implications of not having MPT compatibility?
8. **Threshold cryptography applications for AI agents** — Beyond DKG for private collaboration, what other threshold crypto applications make sense for agent systems? Threshold decryption for model weights? Threshold signing for multi-agent transactions? Secret sharing for API keys?
9. **Knowledge storage on-chain vs. off-chain** — What's the right split? What do other "knowledge chains" do (OriginTrail DKG, The Graph, Ceramic)? What's the gas cost reality for storing knowledge entries on-chain?
10. **Devnet to mainnet path** — What are the typical steps and timelines for going from a 4-node devnet to a production mainnet? What security audits, stress tests, and hardening steps are standard? What do investors expect to see?

---

## Output format

Structure your research as:
1. **Executive Summary** (1 page) — Top 5 highest-conviction opportunities with estimated impact
2. **Section A: Novel Applications** — Each opportunity as a mini-brief (half page each)
3. **Section B: Market Analysis** — Market maps, competitive gaps, demand signals
4. **Section C: Scaling Playbooks** — Concrete strategies with examples from comparable projects
5. **Section D: VC Strategy** — Positioning, metrics, proof points, materials list
6. **Section E: Build Plan** — Prioritized list of artifacts to build, each with scope, timeline, and expected impact on fundraising/adoption

For each recommendation, rate:
- **Impact**: How much it moves the needle (1-5)
- **Feasibility**: How hard is it to execute given what's already built (1-5)
- **Urgency**: How time-sensitive is the opportunity (1-5)

Be specific. Name companies, cite numbers, reference real market data. Avoid generic "agents are the future" platitudes. I want actionable intelligence.
