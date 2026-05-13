# 04 — Markets

The investor-facing entry point for the Nunchi market story. Five docs (this index plus four content documents) covering the thesis, the strategy and narrative, the competitive landscape, and the benchmark-business expansion thesis. Every market figure, competitor name, valuation, ARR, paper, and regulatory citation in the four content docs is traceable to an external source.

For the technical architecture (Roko runtime, the Nunchi blockchain), see the `01-roko` and `02-daeji` folders in this documentation set. For the ISFR methodology, governance, regulatory, and partnership details, see the `03-isfr` folder.

---

## What Nunchi Is — One Paragraph

Nunchi is the company building the **Agent Coordination Plane**: infrastructure that separates how AI agents are coordinated from how they execute, the same structural move SDN made one layer up the network stack. Two products: **Roko**, an open-source Rust runtime (Apache 2.0, 18 crates, ~177,000 LOC) that runs on a developer's laptop today; and **the Nunchi blockchain**, a purpose-built sovereign EVM Layer 1 (Simplex consensus, ~50ms blocks via co-located Tokyo validators, ~300–500ms BFT finality) that anchors agent identity (ERC-8004), reputation (7-domain EMA + slashing), knowledge (HDC fingerprints), and settlement. The thesis: *"the model is the same; the system is the variable."* Cost reduction (10–30x stacked) is the wedge that gets developers in the door; the coordination plane — identity, reputation, shared knowledge, and on-chain audit trails — is the moat.

---

## Names and Conventions

- **Nunchi** — the company / brand. Use this consistently in external communication.
- **Nunchi blockchain** — the chain itself.
- **Daeji** — the testnet name. Internal only; do not use externally.
- **Roko** — the open-source Rust agent runtime.
- **agentchain** — umbrella term for the category combining runtime and chain. Use sparingly; *Agent Coordination Plane* remains the category name in investor-facing materials.
- **Korai** — legacy name for the chain. Should never appear in current materials except in an explicit historical-context line.
- **The earlier payment-rail analogy is RETIRED.** Stripe co-founded the x402 Foundation on April 2, 2026 and launched the Agentic Commerce Protocol with 60+ partners — that positioning would mean building toward an incumbent. Use the SDN analogy instead.

---

## Key Numbers (Verified)

Every figure below is traceable to an external source and verified in the briefing materials. If a number cannot be sourced, it is not in this table.

### Market and protocol substrate

| Number | What | Source |
|---|---|---|
| **97M / month** | MCP SDK monthly downloads | Linux Foundation; AAIF (governs MCP since December 9, 2025) |
| **150+** | A2A participating organizations | Google / Linux Foundation |
| **AAIF surpassed CNCF in 3 months** | MCP-governing foundation membership velocity (170+ orgs) | MCP Dev Summit NA, April 2026 |
| **January 29, 2026** | ERC-8004 mainnet date | eco.com |
| **~21,000–22,900** | ERC-8004 registrations within 2 weeks of mainnet, across BNB Chain, Base, Ethereum L1 | ChainUp / AInvest tracking. *(Registrations measure interest, not active usage.)* |
| **April 2, 2026** | Stripe co-founded x402 Foundation; launched Agentic Commerce Protocol with 60+ partners | Stripe / x402 Foundation |
| **~$50M / 165M txns** | x402 cumulative volume | Coinbase public dashboard, April 2026 |

### Identity and regulation

| Number | What | Source |
|---|---|---|
| **82:1 to 144:1** | Machine-to-human identity ratio in enterprise infrastructure | CyberArk 2025 / Entro Security 2025 |
| **August 2, 2026** | EU AI Act Article 50 enforcement date | EU Regulation 2024/1689 |
| **€15M or 3% of global turnover** | Article 50 transparency violation penalty | EU AI Act Article 99(4)(g) |
| **€35M or 7% of global turnover** | Prohibited-practice penalty | EU AI Act Article 99(4) |
| **35.7%** | EU managers prepared for Article 50 enforcement | Deloitte AI Regulation Survey, Q1 2026 (n=500) |
| **26.2%** | EU managers who have started concrete compliance activities | Deloitte AI Regulation Survey, Q1 2026 |

### Cost wedge and failure rates

| Number | What | Source |
|---|---|---|
| **50x** | Cost variation between agents at similar accuracy on the same tasks | Princeton HAL (ICLR 2026, arXiv:2510.11977; $40K spent, 21,730 rollouts, 9 benchmarks, Weave cost integration) |
| **41–86%** | Multi-agent deployment failure rate | MAST taxonomy, Berkeley AI Safety, NeurIPS 2025 (arXiv:2503.13657; 1,642 production traces across 7 frameworks) |
| **79%** | Failures originating from coordination, not model capability | MAST |
| **14 distinct failure modes, κ=0.88** | MAST inter-annotator agreement | MAST |
| **64%** | Tasks on which a single well-tooled agent matches or outperforms multi-agent ensembles | Princeton NLP, 2025 |
| **45% / 17x** | Single-agent accuracy threshold past which adding agents reduces accuracy ("17x error trap") | Google DeepMind, December 2025 |
| **~15x more tokens** | Multi-agent vs single-agent on the same task | Anthropic multi-agent paper, June 2025 |
| **PlanCraft scaling** | Centralized multi-agent –50.4%, decentralized –41.4%, independent –70.0% vs single-agent baseline | PlanCraft paper |
| **11–14%** | Enterprise agent pilots reaching production at scale | Gartner / McKinsey 2025 surveys |
| **$720 → $18 (~40x)** | Same coding task, naive Opus-only API vs optimized multi-model routing | Cline / Uber analysis, April 24, 2026 |
| **85% / 95%** | Cost reduction retaining quality on standard benchmarks | RouteLLM, Princeton, arXiv:2406.18665 |
| **99.8%** | Anthropic prompt cache hit rate on a specific internal pipeline | Anthropic April 23, 2026 postmortem |
| **92%** | Claude Code cache hit rate (third-party measurement) | LMCache, December 2025 |
| **7% → 74% → 84%** | ProjectDiscovery cache hit progression across 9.8B cached tokens | ProjectDiscovery engineering blog |
| **ρ_c ≈ 0.230** | Critical agent density above which trace-based (stigmergic) coordination beats memory-based by 36–41% | arXiv:2512.10166 |

### Funding and comparables

| Number | What | Source |
|---|---|---|
| **$20–30M at $200–400M post-money** | Nunchi Series A target | Internal; benchmarked against R6 comp analysis |
| **$8.3M seed (April 14, 2026)** | Nava Labs (closest funded analog), Polychain + Archetype co-led | Fortune (April 14, 2026 exclusive) |
| **$5B Series D ($220M, Aug 12 2025)** | Temporal; round led by Sarah Wang and Raghu Raghuram (NOT Casado) | Reuters, August 12, 2025 |
| **$6.4B (Feb 2025)** | IBM acquisition of HashiCorp, all-cash | SEC filings |
| **$1.25B (Series B, Oct 2025)** | LangChain valuation; $125M Series B | Crunchbase; Forbes |
| **~$73M ARR at $10.2B post-money (140x)** | Cognition / Devin, $500M Series B March 2025 | The Information, March 24, 2025 |
| **15% (3 of 20 tasks)** | Devin task completion in independent eval | Answer.AI, January 2025 |
| **659 PRs / best week** | Cognition self-reported Devin dogfood (up from 154 in best week 2025) | Cognition Substack |
| **$100M+ ARR within ~6 months of launch** | Lovable; fastest 0-to-$100M in SaaS history before Cursor surpassed it | The Information, August 2025 |
| **$1B+ ARR by mid-2025** | Cursor; fastest SaaS to $100M ARR ever (~12 months) | Reuters, June 5, 2025; founder disclosure |
| **$300M Series B, Casado-led** | Braintrust valuation (corrected from earlier $800M figure) | TechCrunch, May 2025 |
| **$22M Series A (2026)** | Mastra; ~22K stars (1.5K → 7.5K in one HN week), 300K weekly npm downloads | Spark |
| **$7M seed (April 16, 2026)** | Capsule Security | Public announcement |
| **$5M seed** | t54 Labs (Ripple + Franklin Templeton) | Public announcement |
| **$56M @ $500M pre-product** | /dev/agents (Index + CapitalG) | Public announcement |
| **$65M raised** | Sycamore | Public announcement |
| **$38M (October 2025)** | Keycard (a16z, Aubakirova co-led) | a16z portfolio |
| **$85M (rumored Cisco $250–350M)** | Astrix | Public reporting |
| **$13.8M (February 2025)** | Olas / Autonolas | Public announcement |
| **$359M committed** | 0G Labs | Public reporting |
| **$500M at $5B (October 2025)** | Tempo (mainnet March 18, 2026) | Public announcement |
| **$220M ARR at $4.15B valuation (July 2025)** | Vanta — comparable Series A precedent at the regulatory-deadline play | TechCrunch, July 22, 2025 |
| **$8.3M seed** | Nava — comparable adjacent play Nunchi differentiates against | (above) |
| **$2.66B across 44 rounds YTD 2026** | Macro AI infrastructure funding (vs $1.09B / 71 rounds same period 2025) | Internal market scan |
| **$750M (April 22, 2026)** | Google Cloud agentic partner fund | Google Cloud announcement |

### Benchmark business

| Number | What | Source |
|---|---|---|
| **$665.8 trillion** | OTC interest-rate-derivatives notional | BIS Statistical Bulletin (December 2025); BIS Triennial Central Bank Survey (2025) |
| **$7.9T daily turnover** | OTC IRD market | BIS Triennial 2025 |
| **<$100M** | On-chain interest-rate products | Estimate |
| **>1,000,000:1** | TradFi-vs-on-chain rate-derivatives gap | Computed |
| **$4.5B+ annual** | Combined index-industry revenue (SPDJI ~$1.6B / MSCI ~$1.6B / FTSE Russell £918M) | Public filings |
| **76%** | MSCI Index segment adjusted EBITDA margin | Public filings |
| **$19.1T** | US passive-fund AUM (October 2025) | Public industry data |
| **$400M valuation (April 2025)** | Treehouse Protocol; ~$20M total funding | Public reporting |
| **94% below ATH** | TREE token price; TVL fallen from $610M peak to $157M | Public market data |
| **$40M annualized** | Pendle Boros yield-tokenization revenue at 2025 peak ($13.4B TVL) | Public reporting |
| **6 of 11 spot BTC ETFs reference; >$40B AUM** | CF Benchmarks (FCA FRN 847100, KPMG-audited, Kraken-owned) | Public regulatory filings |
| **CME Term SOFR program** | 7,000+ licenses to 1,800+ firms underpinning $2.6T in loans and $660B in derivative hedges | CME public materials |

---

## Glossary

- **Agent Coordination Plane** — the category Nunchi defines. The infrastructure layer that separates agent coordination from agent execution, the same way SDN separated network control from forwarding.
- **Cooperative Clearing** — the architectural noun for what the coordination plane does: centralized matching, netting, and settlement of obligations between agent counterparties. Borrowed deliberately from CME / DTCC / LCH.
- **HDC** — Hyperdimensional Computing. 10,240-bit binary vectors with constant-time POPCNT similarity. Native precompile at chain address `0xA01`.
- **ZK-HDC** — Zero-knowledge proofs of Hamming distance over committed hypervectors. Circom + Groth16 + Poseidon-2; sub-second laptop proving; ~250K gas to verify on chain.
- **ERC-8004** — On-chain agent identity standard (Identity, Reputation, Validation registries). Mainnet January 29, 2026.
- **ERC-8183** — On-chain agent job-market protocol.
- **MCP** — Model Context Protocol (Anthropic; donated to Linux Foundation December 9, 2025; AAIF governance). Tool layer.
- **A2A** — Agent-to-Agent protocol (Google; Linux Foundation; v1.0 with Signed Agent Cards). Coordination layer.
- **x402** — HTTP 402 payment protocol (Coinbase; Stripe co-founded x402 Foundation April 2, 2026). Payment layer.
- **CascadeRouter** — Roko's adaptive model-routing component (Thompson sampling + LinUCB contextual bandits).
- **NeuroStore** — Roko's durable knowledge layer; agents deposit and query knowledge entries with HDC fingerprints; entries decay via Ebbinghaus-style forgetting.
- **HAL benchmark** — Princeton Holistic Agent Leaderboard (ICLR 2026, arXiv:2510.11977). 9 benchmarks, $40K, 21,730 rollouts, integrated cost tracking via Weave.
- **MAST taxonomy** — Berkeley multi-agent failure taxonomy (NeurIPS 2025, arXiv:2503.13657). 14 distinct failure modes, 1,642 production traces, κ=0.88.
- **Independent Oversight Committee (IOC)** — the governance body for a benchmark administrator; tripartite ARRC structure (administrator → IOC → official-sector ex-officio observers).
- **UK BMR Cat-6** — Lowest-friction regulated benchmark category under the UK Benchmarks Regulation. The path CF Benchmarks took.

---

## Document List

This folder contains five documents. Read them in this order if you are starting from zero.

| # | File | Audience | What it covers |
|---|---|---|---|
| 00 | `00-INDEX.md` (this file) | Anyone | Entry point, names, key-numbers table, glossary, reading paths |
| 01 | `01-thesis.md` | First-time reader | What Nunchi is (30s / 3min / 30min). Buyer pain (production-agent gap). Positioning as Agent Coordination Plane. The locked protocol substrate. The regulatory clock. The cost cliff. The 6–12 month protocol-window claim |
| 02 | `02-strategy-and-narrative.md` | Pitch preparation | The pitch arc. The four primitives. The SDN analogy. The Cinderella wedge. The boundary statements to volunteer. The anti-pattern checklist. What investors should walk away believing |
| 03 | `03-competitive-landscape.md` | Competitive context | Layer-by-layer competitive map. Per-competitor analysis with verified funding and ARR. Keycard adjacency. Nava comparable. Vanta comparable. Casado-associated portfolio. Macro funding context |
| 04 | `04-benchmark-business-thesis.md` | Long-term expansion | ISFR as a regulated benchmark business (not oracle infrastructure). SOFR / VIX / S&P 500 credibility playbook. Methodology / governance / licensing / administration. Comparable benchmark businesses (CF Benchmarks, CoinDesk Indices, Treehouse partner-or-compete). UK BMR Cat-6 path. How it ladders into the Nunchi flywheel |

The `archive/` subfolder contains older drafts that have been superseded; do not pull from it.

---

## Reading Paths

### For an investor evaluating Nunchi cold

1. `00-INDEX.md` (this file) — 5 minutes. Get oriented on names, numbers, and category claim.
2. `01-thesis.md` — 20 minutes. The full thesis: company + product + buyer pain + positioning + why-now.
3. `02-strategy-and-narrative.md §1–§5` — 10 minutes. The pitch arc, four primitives, wedge math, moat layers.
4. `03-competitive-landscape.md §3` — skim. Per-competitor analysis as needed for partner FAQ.
5. `04-benchmark-business-thesis.md §1, §6, §8, §9` — 10 minutes only if "what's the really big vision?" comes up.

### For an engineer onboarding to Nunchi

1. `00-INDEX.md` — 5 minutes. Names, glossary, and the key-numbers table.
2. `01-thesis.md §2 (3-minute version) and §3.4 (chain primitives)` — 10 minutes. What Roko and the Nunchi blockchain are and how they fit together.
3. Then jump to the `01-roko` folder for the runtime architecture and the `02-daeji` folder for the chain spec.
4. Return to `02-strategy-and-narrative.md §3 (four primitives)` to see how the engineering surface maps to the pitch.

### For a competitive analyst

1. `00-INDEX.md` — 5 minutes. Verified key-numbers table is the spine.
2. `03-competitive-landscape.md` — 30 minutes. Layer map, empty quadrant, per-competitor analysis, summary table.
3. `01-thesis.md §4 (positioning)` — 10 minutes. Why Nunchi exits the saturated "trust layer" and "Agent OS" categories and the retired payment-rail analogy.
4. `02-strategy-and-narrative.md §13 (boundary volunteers)` — 10 minutes. The 30-second answers vs. Temporal, Keycard, Inngest.

### For a GTM lead

1. `00-INDEX.md` — 5 minutes. Get the verified numbers.
2. `01-thesis.md §3.2 (buyer pain) and §5.2 (regulatory clock)` — 15 minutes. Who buys, why now, what triggers procurement.
3. `02-strategy-and-narrative.md §9 (the beachhead) and §10 (the two wedges)` — 10 minutes. Pricing structure, 90-day target, services-attached-to-software model.
4. `03-competitive-landscape.md §3 (Cursor / Devin / Linear ecosystem) and §3 (a16z portfolio design partner targets)` — 15 minutes. Specific accounts, integration-target rationale, anti-customer list.
5. `04-benchmark-business-thesis.md §17 (Series A pitch treatment)` — 5 minutes. Where ISFR appears in the deck (slide 9 only) and where it does not.
