# Roko — Learnings and Strategic Context

> **Purpose**: Self-contained briefing for a new Claude Code session or a new team member. Covers architecture, research, strategy, implementation, and risks. No prior context required.
> **Last updated**: 2026-04-26
> **Total**: 5 documents, ~4,500 lines

---

## What is Roko?

Roko is the **identity, reputation, and verifiable-similarity layer for the agent economy** -- an open-source protocol that ships the first production ZK-HDC primitive (verifiable Hamming-distance proofs over committed 10,240-bit hypervectors) and the first ERC-8004 agent identity that survives wallet transfer. It sits beneath A2A, MCP, x402, and Cloudflare Web Bot Auth as the trust layer for the regulated agent economy that EU AI Act Article 50 mandates from August 2, 2026. Cost reduction (10-30x) is the wedge; trust, identity, and cross-organization reputation are the moat.

The core is 3 fundamentals (Signal, Pulse/Bus, Cell, Graph), 9 protocols, 10 specializations, and a stack of HDC fingerprinting + on-chain identity (ERC-8004) + ZK attestation + stigmergic coordination that occupies a competitive quadrant no one else has assembled.

The **Nunchi blockchain** is a purpose-built EVM chain (sovereign EVM L1 with co-located Tokyo validators, Simplex consensus, 400ms blocks) for AI agent coordination. It provides native HDC similarity search at ~400 gas (20-100x cheaper than Solidity), ERC-8004 agent identities, demurrage token economics (1% annual decay -- knowledge IS currency), a 7-domain EMA reputation system, and the ERC-8183 job market. The Nunchi chain amplifies collective intelligence but is not required for individual agent operation -- agents that never touch the chain still benefit from the full Roko cognitive stack. ("Nunchi" is the canonical name; earlier docs may reference "Korai" or "Daeji.")

---

## Reading Order

| # | Document | What it covers | Read time |
|---|---|---|---|
| **01** | [ARCHITECTURE.md](01-ARCHITECTURE.md) | The full technical system — 3 fundamentals, 9 protocols, 10 specializations, two mediums, demurrage, predict-publish-correct, HDC, the Nunchi blockchain (HDC precompile, ERC-8004 agent identities, demurrage economics, ERC-8004 registries, 7-domain reputation, ERC-8183 job market, Valhalla privacy), design principles, compounding mechanisms. End-to-end execution trace. | 30 min |
| **02** | [RESEARCH-SYNTHESIS.md](02-RESEARCH-SYNTHESIS.md) | 80+ papers across 15 topics — coordination, active inference, self-evolution, knowledge, HDC, formal methods, safety, verification, ZK, collective intelligence, performance, scaling laws, competitive landscape, production economics, category creation. Full citations. | 35 min |
| **03** | [STRATEGY-AND-PITCH.md](03-STRATEGY-AND-PITCH.md) | Series A pitch narrative, market sizing, competitive moat, go-to-market (MCP playbook), fundraise narrative for a16z/Sequoia/Bessemer, 90-day launch sequence, counter-theses, regulatory timeline, naming and framing. Plus: the Nunchi blockchain as core narrative, a16z partner map (Casado/Aubakirova/Dixon), 3-minute demo script, 13-slide deck structure, Series A comps ($15-35M at $150-250M), three dangerous bear cases, last-14-days intel. | 30 min |
| **04** | [IMPLEMENTATION-PRIORITIES.md](04-IMPLEMENTATION-PRIORITIES.md) | What exists (18 crates, 177K LOC), what to build in what order (Phase 0-4 over 36 weeks, Phase 4+ Nunchi blockchain), how to prove the 10x cost claim, key dependencies and critical path. Nunchi phase covers testnet deployment, HDC precompile, six Solidity contracts, mirage-rs, and ChainWitness pipeline. | 25 min |

| **05** | [RISKS-AND-ANTIPATTERNS.md](05-RISKS-AND-ANTIPATTERNS.md) | Technical risks (diversity collapse, error amplification), security (90%+ ASR against defenses), regulatory (EU AI Act in 100 days), market (window closing), product (marketplace failures), anti-patterns with citations, mitigations. | 15 min |

---

## How to use these documents

### For a Claude Code session
Feed 01-ARCHITECTURE.md as primary context. Add 04-IMPLEMENTATION-PRIORITIES.md if working on code. Add 03-STRATEGY-AND-PITCH.md if working on positioning or fundraise material. Add 02-RESEARCH-SYNTHESIS.md if exploring research integration.

### For a new team member
Read in order: 01 → 03 → 04 → 05 → 02.

### For investor prep
Read 03-STRATEGY-AND-PITCH.md first, then scan 05-RISKS-AND-ANTIPATTERNS.md for the honest weaknesses section.

### For a technical deep-dive
Read 01-ARCHITECTURE.md, then 02-RESEARCH-SYNTHESIS.md, focusing on the topics relevant to your area.

---

## Key numbers (reference card)

| Metric | Value | Source |
|---|---|---|
| Agent coordination failure rate | 41-86% | MAST, NeurIPS 2025 |
| Failures from coordination (not capability) | 79% | MAST |
| Cost reduction claim | 10-30x | Cache 5x + Route 3x + Gate 2x |
| MCP monthly SDK downloads | 97M | Linux Foundation |
| ERC-8004 mainnet registrations (week 1) | 30K+ | Ethereum mainnet |
| LangGraph monthly downloads | 90M | PyPI |
| Cursor ARR | ~$2B | Founder disclosure |
| Claude Code annualized run rate | ~$2.5B | Anthropic |
| EU AI Act enforcement | August 2, 2026 | EU regulation |
| Window before lock-in | 6-12 months | MCP/A2A/ERC-8004 convergence |
| HDC similarity search | <1 us | CPU POPCNT |
| Stigmergy density threshold | rho = 0.23 | arXiv:2512.10166 |
| Princeton single-agent-beats-multi | 64% of tasks | Princeton NLP |
| **Nunchi block time** | **400ms** | Sovereign EVM L1, Simplex consensus |
| **HDC precompile top-K gas** | **~400 gas** | 20-100x cheaper than Solidity |
| **HDC precompile pairwise gas** | **~50 gas** | Native precompile |
| **NUNCHI token annual demurrage** | **1%** | Lazy per-block decay |
| **Reputation domains** | **7** | coding, security, research, chain, knowledge, operations, strategy |
| **ERC-8183 job market hiring models** | **3** | RandomVRF, Vickrey auction, DirectHire |
| **Valhalla privacy tiers** | **4** | P0 public, P1 access-gated, P2 TEE, P3 ZK-sealed |
| **Series A target** | **$20-30M at $200-400M** | Research6 comp analysis |
| **HAL naive agent cost** | **$32-59/task** | SWE-bench Verified Mini, Princeton 2026 |
| **HAL optimized cost** | **~$1.42/task** | With Nunchi caching + routing |
| **ERC-8004 active agents** | **~80-150K** | 3 months post-mainnet |
| **x402 Foundation members** | **17** | Stripe, Coinbase, AWS, Visa, etc |
| **Machine:human identity ratio** | **82:1 to 144:1** | CyberArk / Entro 2025 |
| **ZK-HDC v1 proving time** | **<1 second** | Laptop, Circom + Groth16 |
| **Closest funded analog** | **Nava $8.3M** | Polychain + Archetype, April 2026 |

---

## Related document sets

These learnings synthesize insights from multiple source document sets:

| Set | Path | What's there |
|---|---|---|
| Unified spec (v2) | `tmp/unified/` (22 files) | The protocol specification — Signal/Cell/Graph + 9 protocols + 10 specializations |
| Unified depth | `tmp/unified-depth/` (22 dirs) | Algorithms, research backing, implementation detail per spec doc |
| Research docs | `tmp/unified/research{1-7}.md` | Seven rounds of deep research -- substrates through production reality, Series A intelligence, competitive reality check |
| DeFi gaps | `tmp/defi/gap/` (14 files) | DeFi-specific requirements and real-time domain generalization |
| Refinements | `tmp/refinements/` (36 files) | Architectural redesign proposals (two mediums, demurrage, c-factor) |
| Visual-gate2 | `tmp/visual-gate2/` (10 files) | Verification system redesign (evidence typing, BT judges, flywheel) |
| Run-anywhere | `tmp/run-anywhere/` (22 files) | WASM deployment, Merkle-CRDT sync, brain export |
| 04-21-26 | `tmp/04-21-26/` (117 files) | Operational specs — PRDs, generalizations, arenas, HDC integration |
| Workflow | `tmp/workflow/` (12 files) | Graph authoring, trigger system, execution engine |
| Architecture | `tmp/architecture/` (21 files) | Agent runtime, extensions, connectivity, knowledge |
| Core docs | `docs/` (422 files) | Deep per-system specs — 22 sections, 8.8MB |
| Nunchi chain docs | `docs/08-chain/` (25 files) | Chain vision, spec, HDC precompile, agent identities, token economics, ERC-8183 job market, reputation, privacy, gossip, clearing |
| Series A intelligence | `tmp/unified/research6.md` | a16z partner map, demo script, bear cases, comps, last-14-days intel |
| Competitive reality check | `tmp/unified/research7.md` | Pitch repositioning, x402/Stripe threat, 50ms/demurrage exposure, ZK-HDC status, design partners, ERC-8004 strategy |
| Nunchi dashboard | `/Users/will/dev/nunchi/nunchi-dashboard/` | Landing page (pitch deck), app dashboard, 27+ pages, ROSEDUST design system |
| Dashboard PRDs | `nunchi-dashboard/tmp/prds/` (5 files) | 360-item spec, strategy, page redesigns |
| Dashboard UX research | `nunchi-dashboard/tmp/ux-refresh-context/` (6 files) | Current state audit, optimal redesign, UX research (dopamine, foraging, composability) |
