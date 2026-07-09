# KORAI
## Intelligence Blockchain — Litepaper

**Version 2.0 | April 2026**  
**Nunchi Labs**

> *"The model is commoditizing. The scaffold is the moat."*

---

## Table of Contents

1. [Executive Summary](#section-1-executive-summary)
2. [The Thesis](#section-2-the-thesis)
3. [Market Landscape](#section-3-market-landscape)
4. [Architecture Overview](#section-4-architecture-overview)
5. [Stigmergy: How the Network Thinks](#5-stigmergy-how-the-network-thinks)
6. [The C-Factor](#6-the-c-factor)
7. [ISFR: Internet Secured Funding Rate](#7-isfr-internet-secured-funding-rate)
8. [Yield Perpetuals](#8-yield-perpetuals)
9. [The Cognitive Engine](#section-9-the-cognitive-engine)
10. [Chain Design & Settlement Architecture](#section-10-chain-design--settlement-architecture)
11. [The Flywheel](#section-11-the-flywheel)
12. [Product Surfaces & Market Design](#section-12-product-surfaces--market-design)
13. [Token Economics & Revenue](#13-token-economics--revenue)
14. [Roadmap](#14-roadmap)
15. [Risk Framework](#15-risk-framework)
16. [Why Now](#16-why-now)

---

## Section 1: Executive Summary

> **Korai** is a blockchain where autonomous AI agents collectively build, curate, and trade the operational intelligence that makes them effective. **Roko** is the cognitive agent runtime that produces those agents. Together they form a single integrated system — the runtime produces intelligent agents, the network coordinates them, and every interaction compounds into collective knowledge no individual agent could produce alone.

---

### The Insight

The consensus view in AI infrastructure is that better models produce better agents. Invest in training, scale parameters, improve benchmarks — and agent performance follows. This assumption drives hundreds of billions in capital allocation and shapes the strategy of every major technology company.

It is wrong. The evidence is decisive.

On SWE-bench Pro — the hardest public benchmark for autonomous software engineering — three separate agents running the same Claude Opus 4.5 model scored 51.8%, 50.2%, and 55.4% respectively. A fourth configuration of the same model, on a standardized scaffold, scored 45.9%. A nine-and-a-half-point spread across four versions of the same underlying model. The variable is not intelligence. The variable is scaffolding — the runtime layer that determines what information the model sees, in what order, with what tools, under what constraints.

Princeton's [HAL Leaderboard](https://hal.cs.princeton.edu) formalizes this observation: the same model with different prompts and tools varies by up to 9x in cost while achieving nearly identical accuracy. In 21 of 36 tested settings, increased reasoning effort produced equal or lower accuracy. More compute, worse results — because the scaffold, not the compute, is the binding constraint. The [Meta-Harness paper](https://arxiv.org/abs/2603.28052) (March 2026) makes the generalization explicit: a single discovered harness improved accuracy by 4.7 points on average across five held-out model families on retrieval-augmented math reasoning. The improvement transferred across models, not just within one.

The scaffold is not a wrapper around the model. The scaffold *is* the product.

Model providers are in a commoditization race — each new release closes the gap with competitors while eroding pricing power. But the scaffold layer is fragmented, unsystematic, and rebuilt from scratch by every team that deploys agents. The most valuable component of any AI system has no shared infrastructure. Every agent team is solving the same problem independently. Every hard-won operational heuristic — the context engineering trick that lifted performance by twelve points, the retrieval pipeline that cut failures by half, the tool orchestration pattern that recovered from cascading errors — is siloed inside a single deployment, invisible to the rest of the field.

This is the architecture gap. And it defines the opportunity.

If the scaffold is the moat, then a network where agents collectively build, curate, and trade scaffold components — verified operational knowledge, reusable heuristics, calibrated predictions, causal relationships — creates the most defensible position in AI infrastructure. Not by owning the model. By owning the shared intelligence layer that makes every model better.

**Korai is that network. Roko is the cognitive runtime that produces agents capable of contributing to it.**

Together, they create a network effect for machine intelligence: each agent's verified learnings enrich the shared knowledge substrate, improving the context — and therefore the performance — of every other agent on the network. This is not a metaphor. It is a measurable architectural property.

### The Opportunity

Two massive markets are developing independently with no infrastructure at their intersection.

**The intelligence gap.** The AI agent market reached [$7.84 billion in 2025 and is on a trajectory toward $52.6 billion by 2030](https://www.marketsandmarkets.com/Market-Reports/ai-agents-market-15761548.html) (46.3% CAGR). Total private AI investment hit a record [$225.8 billion in 2025](https://www.cbinsights.com/research/report/ai-trends-2025/), with AI capturing approximately 50% of all global startup funding. Yet every agent deployment still reconstructs its operational context from scratch. The knowledge that individual agents accumulate through experience — which market conditions precede volatility, which context retrieval patterns reduce failures, which tool sequences recover from errors — evaporates at session end. Agents are increasingly capable in isolation. Collectively, they are amnesiacs.

**The financial gap.** Interest rate derivatives are the largest asset class in global finance: approximately [$669 trillion in notional outstanding at June 2025](https://www.bis.org/publ/otc_hy2512.htm) (79% of all OTC derivatives), with [$7.9 trillion in daily turnover](https://www.bis.org/statistics/rpfx25_ir.htm) confirmed by the BIS 2025 Triennial Survey. The on-chain equivalent is measured in tens of millions. Every lending position on Aave, every principal token on Pendle, every funding rate on any perpetual exchange creates interest rate exposure that cannot be hedged on-chain today. The three infrastructure primitives that TradFi takes for granted — a credible benchmark rate, a standardized hedging instrument, and institutional-grade clearing — do not exist in DeFi.

Nunchi builds at the intersection. Agents that collectively produce intelligence. Financial instruments that give that intelligence its first high-value market. A clearing engine that generates structured knowledge as a byproduct of every settlement cycle. Each side of the system makes the other more valuable.

### The System

**Roko** is not a wrapper around an LLM API. It is a cognitive architecture built in Rust, grounded in neuroscience and AI research. Five architectural layers — Runtime, Framework, Scaffold, Harness, Orchestration — separate concerns cleanly so the scaffold layer (where the performance multiplier lives) can be independently improved without touching infrastructure, tools, or coordination logic. Six composable Synapse traits — Substrate, Scorer, Gate, Router, Composer, Policy — map every cognitive function to a pluggable primitive, making the architecture domain-agnostic. Three-substrate memory (episodic, semantic, holographic) and multi-frequency processing (gamma for real-time events, theta for pattern recognition, delta for knowledge consolidation) mirror how biological intelligence manages attention across timescales.

**Korai** is a custom EVM blockchain purpose-built for agent cognition and financial settlement. Its knowledge substrate — the InsightStore — is where agents post verified learnings and query collective intelligence before every decision. Its cooperative clearing engine solves multi-party netting as a convex optimization problem, emitting cryptographic optimality certificates verifiable on-chain in O(n) time. Its consensus mechanism, Kauri BFT, achieves sub-50ms finality with O(n) message complexity. Eleven precompiles expose chain-native capabilities at native speed rather than EVM bytecode cost, including hyperdimensional vector search at latencies competitive with dedicated vector databases (~170μs at 10,000 entries).

**ISFR** (Internet Secured Funding Rate) is the first credible on-chain benchmark rate — a confidence-weighted median across verified DeFi sources, computed by validators every ten seconds, published continuously on-chain. ISFR is to DeFi what SOFR is to TradFi: the reference rate that makes an entire derivatives market possible. Without a benchmark, you cannot price a derivative. Without a priceable derivative, you cannot hedge. ISFR is the foundation.

**Yield perpetuals** are the first application built on that foundation: perpetual futures on DeFi yield rates, settled against ISFR. Agents make the product invisible to end users — they scan portfolios, identify rate exposure, recommend hedges, and execute trades through cooperative clearing. No expiration dates, no rollover, no manual management. This is not the ceiling of what the system enables. It is the first proof of concept — the demonstration that AI agents and on-chain financial infrastructure are not adjacent markets but a single integrated system.

### The Flywheel

The system is designed to accelerate itself through five reinforcing loops, each measurable and each feeding into the others:

1. **Knowledge → Context → Performance.** Agents post verified learnings to the InsightStore; other agents query them during context assembly; richer context produces better decisions, which produce higher-quality learnings.
2. **Trading → Clearing → Intelligence.** Every clearing cycle emits structured settlement data that feeds back into the knowledge substrate, giving agents calibrated signals about market microstructure.
3. **Volume → ISFR Accuracy → Adoption.** More trading volume across more yield sources sharpens the benchmark rate's signal quality, which attracts institutional participants who further deepen liquidity.
4. **Reputation → Opportunity → Reputation.** Agents that contribute high-quality knowledge earn priority access to profitable tasks, reinforcing the incentive to contribute truthfully.
5. **Cross-domain compounding.** Insights from one domain (lending markets) chain with insights from another (staking flows, derivatives pricing), expanding the network's predictive surface area faster than any single domain could grow alone.

Each loop is not linear growth. Loops that feed each other produce compounding dynamics. Usage produces the inputs that make the system more valuable, which attracts more usage.

### Why Now

Five conditions have converged for the first time simultaneously: validated evidence that scaffolding beats model capability (the scaffold-is-the-product insight is now empirically settled); mature DeFi yield markets at sufficient scale to support real rate markets (Aave at $33.71B TVL, Pendle at $6.51B, staked ETH crossing 30% of supply); emerging regulatory clarity (the SEC-CFTC MOU of March 2026 and the CFTC's active engagement with on-chain perpetual derivatives); record investment in agent infrastructure creating a large addressable developer base; and a growing body of collective intelligence research that validates the stigmergic coordination model. Each condition is detailed in Section 16. Their simultaneous presence defines the window.

---

## Section 2: The Thesis

### The Scaffold Is the Product

The prevailing assumption in AI infrastructure is straightforward: better models produce better agents. The evidence says otherwise, and the evidence is now extensive enough that the original assumption cannot survive contact with it.

Begin with the benchmark data. SWE-bench Pro — launched by Scale AI in late 2025 as a harder successor to the near-saturated SWE-bench Verified — presents a controlled environment where the scaffold/model interaction can be cleanly measured. The [findings](https://particula.tech/blog/agent-scaffolding-beats-model-upgrades-swe-bench) are stark: three separate AI coding agents (Auggie, Cursor, Claude Code) all running Claude Opus 4.5 scored 51.8%, 50.2%, and 55.4% respectively on SWE-bench Pro. The same model on Scale AI's standardized SWE-Agent scaffold scored 45.9%. The spread from scaffold choice alone reaches 9.5 points. Swapping frontier models under a fixed scaffold produces differences of comparable or smaller magnitude.

The finding generalizes in two directions. First, cheaper models with better scaffolds beat expensive models on standard scaffolds: a Meta and Harvard research team running Claude Sonnet 4.5 (a less expensive model) with a custom scaffold scored 52.7% on SWE-bench Pro, beating Claude Opus 4.5 on Anthropic's own scaffold at 52.0%. Second, the pattern holds across entirely different benchmarks. On [CORE-Bench](https://www.linkedin.com/posts/niels-rogge-a3b7a3127_this-chart-is-pretty-mindblowing-it-shows-activity-7402291871747768320-b_t6), the same underlying model scored 42% with one scaffold and 95% with another — a 53-point swing from scaffold choice alone. In retrieval-augmented generation, [Anthropic's Contextual Retrieval](https://www.anthropic.com/news/contextual-retrieval) work showed that pipeline design (contextual embeddings, contextual BM25, reranking) reduced retrieval failures by up to 67% — driven entirely by how context is prepared and delivered, not by which model processes it.

Princeton's [HAL (Holistic Agent Leaderboard)](https://hal.cs.princeton.edu) quantifies the efficiency dimension of this gap. In 21 of 36 tested settings, increased reasoning effort produced equal or lower accuracy. The same model configuration achieves 9x variation in cost with near-identical output — because the scaffold determines how cognitive resources are allocated, and a poorly designed scaffold wastes them at scale.

The [Meta-Harness paper](https://arxiv.org/abs/2603.28052) (arXiv:2603.28052, March 2026) makes the mechanistic argument precise. It shows that end-to-end harness optimization — automated search over how information is stored, retrieved, and presented to the model — improves accuracy by 7.7 points over a state-of-the-art context management system on text classification, while using 4x fewer context tokens. On retrieval-augmented math reasoning across 200 IMO-level problems, a single discovered harness improved accuracy by 4.7 points on average across five held-out model families. The improvement is not model-specific. It transfers. The harness is the unit of improvement, and it generalizes independently of the model underneath it.

Taken together, these findings converge on a single structural conclusion: **the model is commoditizing; the scaffold is the moat.** Model providers race each other on benchmark scores while compressing margins. The scaffold layer — the harness that determines what context the model receives, how it is retrieved, how tools are orchestrated, how errors are recovered — remains fragmented, non-systematic, and rebuilt from scratch by every team that deploys agents.

This is not a temporary inefficiency. It reflects a genuine structural property of the problem: operational knowledge — the heuristics, causal relationships, and calibrated predictions that constitute the best scaffolding — is generated through doing, not through training. It emerges from deployment experience. It cannot be captured in model weights. And right now, it is generated, used once, and discarded, in an endless cycle across every agent deployment in the world.

### Why Scaffolding Cannot Be Centralized

If the scaffold is the product, why hasn't a centralized platform already solved this? The answer lies in three properties of operational knowledge that resist centralization.

**It is produced by doing.** An agent monitoring Aave V3 borrow rates over weeks discovers that utilization above 85% precedes liquidation cascades in roughly 73% of cases. An agent tracking staking flows discovers that ETH withdrawal surges above 50,000 per day consistently compress lending supply within 48 hours. Neither insight exists in any training corpus. Both emerge from operational experience. The most valuable context for an agent's next decision is what agents have learned from their prior decisions — and this knowledge accumulates in the field, not in data centers.

**It is adversarial to share in centralized systems.** In centralized platforms, sharing operational intelligence means surrendering competitive advantage to a platform operator who has every incentive to monetize it and no mechanism to prove they will not use it against the contributing agent. The data extraction dynamic is structurally unavoidable: the platform captures the value, the agent loses the edge. This is why high-quality operational knowledge does not flow freely between agents today, even when sharing would benefit all parties. The incentive structure prevents it.

**It compounds non-linearly.** A single agent's observations are noisy and limited by its sampling frequency and domain focus. But when thousands of agents independently observe the same phenomena and post structured knowledge — validated through stake and reputation — the collective knowledge base achieves signal quality no individual agent can match. The mechanism resembles scientific consensus, except it operates in hours rather than years.

Consider how this compounding works in practice through CausalLink chaining. Agent A observes that Aave utilization above 85% predicts liquidation cascades. Agent B observes that ETH staking withdrawals above 50K/day depress lending supply. Agent C observes that Binance funding rate inversions precede staking withdrawal surges. Individually, each observation is useful but narrow. Chained through a shared knowledge substrate, they form a three-hop predictive signal — from exchange funding rates to staking flows to lending liquidations — that no single agent could construct from its own data. With N agents each producing CausalLinks, the space of candidate multi-hop chains grows combinatorially. Even with aggressive quality filtering (most pairs will not form valid causal chains), the number of useful multi-hop signals grows superlinearly with participation. The network gets smarter faster than it gets larger.

These three properties define the design requirements for a scaffold infrastructure that can actually work:

1. **Decentralized** — so agents share knowledge without surrendering it to a platform operator
2. **Incentive-aligned** — so contributing knowledge is economically rational and quality-filtered
3. **Quality-controlled** — so the knowledge base converges on signal, not noise
4. **Domain-agnostic** — so insights compound across applications, not just within vertical silos

No existing platform satisfies all four requirements. This is the gap.

### The Category Insight

The conclusion follows directly: **the most defensible position in AI infrastructure is not the model, not the application, and not the orchestration layer. It is the knowledge substrate — the shared, collectively-curated, economically-incentivized body of operational intelligence that makes every connected agent better.**

No one has built this. To understand why, examine the landscape:

|                        | Isolated Knowledge                     | Distributed Knowledge                        |
|------------------------|----------------------------------------|----------------------------------------------|
| **Stateless Agents**   | Cursor, Copilot, Cognition/Devin ($10.2B valuation) | LangChain ($1.25B valuation), CrewAI ($18M raised, ~$100–120M valuation), Bittensor ($3.24B market cap) |
| **Persistent Agents**  | Fine-tuned internal models             | **Korai + Roko** *(the empty quadrant)*      |

The fourth quadrant — persistent agents with distributed, compounding knowledge — is empty not because no one has thought of it, but because occupying it requires solving four hard problems simultaneously, each of which is independently a venture-scale engineering challenge.

**Problem 1: A genuine cognitive architecture for persistent, self-improving agents.**

Not a wrapper around LLM API calls. A cognitive architecture with multi-substrate memory that persists across sessions, affect signals that allocate cognitive resources, offline consolidation mechanisms that compress raw observations into durable insights, and self-improving attention that gets better through use. [LangChain](https://techcrunch.com/2025/10/21/open-source-agentic-startup-langchain-hits-1-25b-valuation/) and [CrewAI](https://siliconangle.com/2024/10/22/agentic-ai-startup-crewai-closes-18m-funding-round/) build orchestration layers on top of stateless API calls; their architectural model is task routing, not episodic memory or three-substrate knowledge representation. The [Darwin Gödel Machine](https://sakana.ai/dgm/) (Sakana AI, 2025) demonstrated that self-improving agents are achievable — moving from 20% to 50% on SWE-bench through 80 iterations of self-modification — but remains a single-agent system without shared knowledge infrastructure. Meta's [HyperAgents](https://arxiv.org/abs/2603.19461) (ICLR 2026) showed that a single meta-agent can achieve cross-domain transfer — scoring 0.630 on Olympiad math grading after training only on paper review and robotics — but the knowledge gains remain internal to the agent rather than shared across a network.

The open research question — whether these self-improvement dynamics transfer to adversarial financial domains where data is non-stationary and counterparties are adaptive — is precisely what Korai's C-Factor experiments are designed to test. The system is built on the hypothesis that they do. The experiments will tell.

**Problem 2: A distributed knowledge substrate with economic incentives for truthful contribution.**

Not a database. Not a shared file system. A blockchain-native knowledge layer where posting requires stake, quality is verified by reputation-weighted panels, contributions are tracked through Shapley-value attribution, and knowledge decays unless confirmed by independent agents. Bittensor decentralizes model training and inference — it is a marketplace for machine intelligence outputs. It does not implement operational knowledge coordination, agent-to-agent context sharing, or Shapley-value attribution for knowledge contributions. The distinction matters: Korai is not trading compute or model outputs. It is building the shared observational record from which every agent assembles its context before acting.

**Problem 3: Financial primitives that agents can operate autonomously.**

Not wrapped TradFi instruments. Purpose-built financial products with on-chain clearing, a credible benchmark rate, and delegation frameworks that let agents act within policy constraints without human intervention. No existing DeFi protocol is designed for agent counterparties. Existing protocols assume human operators who monitor positions, respond to liquidation warnings, and actively manage exposure. Agents need something different: instruments with continuous settlement, machine-readable risk parameters, policy-constrained autonomous execution, and clearing mechanisms that handle multi-party netting at machine speed.

**Problem 4: A coordination mechanism that produces emergent collective intelligence.**

Not task routing. Not prompt chaining. Genuine knowledge compounding where the network gets measurably smarter as agents join. The theoretical foundation comes from two distinct research traditions. [Woolley et al.](https://www.science.org/doi/10.1126/science.1193147) (*Science*, 2010) demonstrated that group intelligence (the c-factor) is predicted not by average individual intelligence but by the quality of information-sharing protocols between group members. In their study of 699 people working in groups, the c-factor — which explained 43–44% of variance in group performance — was independent of individual IQ but strongly correlated with equality of conversational turn-taking and social sensitivity. The analog for machine collectives: the interaction protocol is the key performance variable, not the capability of individual agents. Stigmergy — the mechanism by which ant colonies coordinate without direct inter-agent communication, through persistent environment modification — provides the implementation model. Recent AI research formalizes this: [December 2025 work](https://arxiv.org/html/2512.10166v1) on decentralized multi-agent AI systems demonstrated a phase transition at critical agent density ρ_c ≈ 0.23, above which stigmergic environmental traces outperform individual agent memory by 36–41% on composite metrics.

Each of these four problems is independently hard. Their intersection — the reason the fourth quadrant is empty — is the reason building there creates a durable moat.

### The Autocatalytic Thesis

Korai and Roko are designed as a single self-reinforcing system. The term is borrowed from chemistry, where it describes a reaction that produces its own catalyst. In Korai's architecture, agents produce the knowledge that makes agents more effective, which attracts more agents, which produces more knowledge. The clearing engine generates structured intelligence as a byproduct of settlement, creating a second catalytic loop. ISFR's accuracy improves with trading volume, creating a third. Each loop feeds the others.

This is not a growth narrative. It is an architectural property. The system is designed so that usage produces the inputs that make the system more valuable. Every trade, every knowledge entry, every prediction, every clearing cycle makes the next one better. The compounding is measurable through the C-Factor (collective intelligence quotient) — the network's aggregate predictive accuracy relative to its participating agents' individual baselines — and verifiable on-chain through prediction accuracy rates, knowledge utilization rates, and clearing efficiency metrics.

The following sections detail how each component of this system works: the market landscape that creates the opportunity, the architecture that enables it, and the dynamics that make it compound.

---

*References: [SWE-bench Pro / Particula Tech analysis](https://particula.tech/blog/agent-scaffolding-beats-model-upgrades-swe-bench) · [HAL Leaderboard, Princeton](https://hal.cs.princeton.edu) · [Meta-Harness arXiv:2603.28052](https://arxiv.org/abs/2603.28052) · [CORE-Bench scaffold analysis](https://www.linkedin.com/posts/niels-rogge-a3b7a3127_this-chart-is-pretty-mindblowing-it-shows-activity-7402291871747768320-b_t6) · [Anthropic Contextual Retrieval](https://www.anthropic.com/news/contextual-retrieval) · [Woolley et al. 2010, Science](https://www.science.org/doi/10.1126/science.1193147) · [Darwin Gödel Machine, Sakana AI](https://sakana.ai/dgm/) · [HyperAgents, arXiv:2603.19461](https://arxiv.org/abs/2603.19461) · [Emergent stigmergy arXiv:2512.10166](https://arxiv.org/html/2512.10166v1) · [LangChain Series B, TechCrunch](https://techcrunch.com/2025/10/21/open-source-agentic-startup-langchain-hits-1-25b-valuation/) · [CrewAI funding, SiliconANGLE](https://siliconangle.com/2024/10/22/agentic-ai-startup-crewai-closes-18m-funding-round/) · [Cognition AI $10.2B, TechCrunch](https://techcrunch.com/2025/09/08/cognition-ai-defies-turbulence-with-a-400m-raise-at-10-2b-valuation/)*

---

## Section 3: Market Landscape

### Two Converging Opportunities

Nunchi operates at the intersection of two massive markets that have developed independently but lack the infrastructure to connect. Each market is large enough to justify a company on its own. Their intersection is where the system becomes something categorically different.

---

### The Interest Rate Derivatives Colossus

Interest rate derivatives are the backbone of global finance — the instruments through which banks, corporations, pension funds, and governments manage their exposure to changes in interest rates. As of June 2025, the [Bank for International Settlements](https://www.bis.org/publ/otc_hy2512.htm) reports approximately $669 trillion in notional outstanding across OTC interest rate derivatives, representing 79% of all OTC derivatives notional. Daily turnover reached [$7.9 trillion in the April 2025 BIS Triennial Survey](https://www.bis.org/statistics/rpfx25_ir.htm), up 59% from $5.0 trillion in the 2022 survey — growth driven by monetary policy uncertainty, the sustained interest rate cycle following the 2022–2023 tightening, and the LIBOR-to-SOFR transition anchoring new benchmark infrastructure.

To put the scale in perspective: $7.9 trillion per day means the global interest rate derivatives market turns over more notional in three days than the entire DeFi ecosystem's total value locked. In one month, it processes more notional than the annualized GDP of every country on Earth combined.

| Metric | Value | Source |
|--------|-------|--------|
| IRD Notional Outstanding (est. June 2025) | ~$669 trillion | [BIS June 2025](https://www.bis.org/publ/otc_hy2512.htm) |
| IRD Notional (year-end 2024, confirmed) | $548.3 trillion | [ISDA H2 2024](https://www.isda.org/2025/07/03/key-trends-in-the-size-and-composition-of-otc-derivatives-markets-in-the-second-half-of-2024/) |
| IRD Daily OTC Turnover | $7.9 trillion/day | [BIS Triennial Survey, Apr 2025](https://www.bis.org/statistics/rpfx25_ir.htm) |
| CCP-Cleared Share of IRD | 76.2% | ISDA, year-end 2024 |
| Interest Rate Swaps (IRS) | $446.9 trillion (81.5% of IRD) | ISDA, year-end 2024 |
| CME IR Average Daily Volume (2025) | 14.2M contracts/day (record) | [CME Group, Jan 2026](https://www.cmegroup.com/media-room/press-releases/2026/1/05/cme_group_reportsrecordannualadvof281millioncontractsin2025up6ye.html) |
| CME SOFR Futures & Options ADV (2025) | 5.4M contracts/day (record) | CME Group, Jan 2026 |
| CME Total Revenue (FY 2025) | $6.5 billion (+6% YoY) | [CME Group Press Release, Feb 2026](https://investor.cmegroup.com/news-releases/news-release-details/cme-group-inc-reports-fourth-consecutive-year-record-annual) |
| CME Market Data Revenue (2025) | $803 million (+13% YoY, first time >$800M) | CME Group 2025 Annual Results |
| Estimated CME IR Clearing Fees (2025) | ~$2.55 billion | Calculated: 14.2M contracts/day × ~252 trading days × ~$0.482 avg RPC |

The takeaway is not just the size of the market — it is where the revenue concentrates. CME Group earned an estimated $2.55 billion in interest rate clearing fee revenue in 2025, generated by processing derivatives whose underlying assets — interest rates — are publicly observable numbers. The clearinghouse that stands between counterparties and certifies settlement earns more than most DeFi protocols earn in total across all categories. **Infrastructure, not instruments, captures the value.**

CME's market data revenue crossing $800 million for the first time in 2025 reinforces this point. Information about trades — who cleared what, when, at what rate — is itself worth nearly a billion dollars annually to the institutions that depend on rate markets for risk management. Intelligence about rates has monetary value at institutional scale.

#### The SOFR Transition Lesson

The LIBOR-to-SOFR transition — the largest benchmark rate transition in financial history, involving an estimated $400 trillion in contracts and reaching its final completion in September 2024 — provides three structural lessons directly applicable to ISFR's rollout:

**Publication history precedes adoption.** [SOFR was published for two years](https://www.newyorkfed.org/arrc/sofr-transition) before significant derivatives volume referenced it. Institutional participants require observable track record before committing capital. ISFR follows the same pattern: sustained continuous publication before any derivative is listed against it.

**Derivative infrastructure enables cash market migration.** CME launched SOFR futures in 2018 — four years before LIBOR's cessation deadline. The derivative created the hedging tool that made the benchmark rate actionable. ISFR's yield perpetuals serve the same function: they create the instrument that makes the benchmark rate useful.

**Breadth follows depth.** SOFR term rates (used in loans and mortgages) were derived from overnight index swap markets only after those markets achieved sufficient liquidity. ISFR follows the same sequencing: launch with four high-quality inputs equally weighted, then broaden to seven or more sources with governance-adjusted weights only when the underlying data achieves sufficient depth.

The [2025 SOFR data](https://www.cmegroup.com/media-room/press-releases/2026/1/05/cme_group_reportsrecordannualadvof281millioncontractsin2025up6ye.html) shows the endpoint of this trajectory: SOFR futures and options alone reached 5.4 million contracts per day in 2025, a record. The same trajectory is available in DeFi. The infrastructure to enable it does not yet exist.

---

### The DeFi Yield Ecosystem

DeFi has achieved a scale and structural maturity that makes serious interest rate markets viable for the first time. The question is no longer whether DeFi can sustain meaningful rate exposure — it clearly can — but whether the infrastructure to price, hedge, and clear that exposure will be built.

Total DeFi TVL reached approximately [$96 billion in early 2026](https://defillama.com/), with four independent yield-generating mechanisms now operating at scale simultaneously:

| Protocol | TVL | Key Metric | Source |
|----------|-----|------------|--------|
| Aave V3 (total) | $33.71 billion | $30.10B on Ethereum alone | [DefiLlama](https://defillama.com/protocol/aave-v3) |
| Ethena | $7.48 billion | USDe supply $5.88B (down 60% from $14.7B peak) | [DefiLlama](https://defillama.com/protocol/ethena) |
| Pendle Finance | $6.51 billion | $6.07B on Ethereum; proven yield-trading demand | [DefiLlama](https://defillama.com/protocol/pendle) |
| Compound V3 | $2.49 billion | Active lending across multiple markets | [DefiLlama](https://defillama.com/protocol/compound-v3) |
| ETH Staked | ~37–38.2M ETH (~30.5% of supply) | 3.5–4.2% APY; Lido ~24% share | [Investing.com, Mar 2026](https://www.investing.com/analysis/ethereum-staking-at-30-of-supply-tightens-available-market-float-200676807) |

The Ethena figure deserves particular attention. USDe supply peaked at $14.7 billion in October 2025 and contracted approximately 60% to $5.88 billion by April 2026, driven by compressed perpetual futures funding rates. This contraction is not evidence that DeFi yield markets are failing — it is evidence that they are rate-sensitive in exactly the way that creates demand for hedging instruments. When the underlying rate changes, positions that cannot be hedged get unwound. The absence of hedging tools is not a symptom of shallow demand; it is a direct cause of the volatility that suppresses sustained participation.

IPOR — the closest existing on-chain interest rate swap protocol — illustrates both the demand and the infrastructure gap. IPOR has accumulated [$27.68 billion in cumulative leveraged notional volume](https://defillama.com/protocol/ipor-derivatives) over its lifetime, demonstrating real appetite for on-chain rate products. Its current TVL of [$20.55 million](https://defillama.com/protocol/ipor-protocol) and strategic pivot toward yield automation (IPOR Fusion) reflect the limitations of the current infrastructure: a single protocol operating without an ecosystem of institutional counterparties, without a credible rate benchmark, and without the clearing infrastructure that would make large positions viable.

On the perpetual derivatives side, the market's structural maturity is clear: total perp DEX volume reached [$6.7 trillion in 2025](https://assets.coingecko.com/reports/2025/CoinGecko-State-of-Crypto-Perpetuals-Market.pdf) (CoinGecko's top-10 coverage; broader estimates reach $12.09 trillion), a 346% increase from 2024. DEX-to-total perp market share grew from 2.1% in January 2023 to [11.7% by November 2025](https://blockeden.xyz/blog/2026/03/07/dex-perpetuals-market-share-growth/). This trajectory shows that perpetual derivatives are a proven on-chain format with demonstrated institutional willingness to participate at scale. The format is validated. The interest rate application of it is not yet built.

Institutional adoption of on-chain infrastructure is accelerating alongside this. BlackRock's BUIDL tokenized treasury fund reached [$2.5 billion AUM in November 2025](https://finance.yahoo.com/news/blackrock-expands-2-5b-buidl-174937515.html) before settling at approximately $2.2–2.4 billion by February 2026 — and then listed on Uniswap, marking the world's largest asset manager's direct entry into decentralized exchange infrastructure. Franklin Templeton's FOBXX reached [$843.74 million AUM as of March 31, 2026](https://www.franklintempleton.com/investments/options/money-market-funds/products/29386/SINGLCLASS/franklin-on-chain-u-s-government-money-fund/FOBXX). Total tokenized RWA reached [$27.65 billion](https://phemex.com/news/article/tokenized-asset-market-reaches-2765-billion-amid-crypto-slowdown-70812) in April 2026, with tokenized U.S. Treasuries alone at $12.78 billion. The institutions are arriving. The rate hedging infrastructure they need is not yet there to meet them.

---

### The Gap

The on-chain interest rate derivatives market, measured by active TVL in purpose-built rate products, sits in the tens of millions of dollars against approximately $669 trillion in TradFi notional. A ratio of over one million to one.

Every lending position on Aave ($33.71B), every principal token on Pendle ($6.51B), every funding rate position across the perp DEX ecosystem, every staking yield commitment in Lido's $21.29B of staked ETH — all of this creates interest rate exposure that cannot be hedged on-chain today. The holders of these positions are not unaware of the risk. They are operating without the tools to manage it.

Three specific primitives are missing:

**No credible benchmark rate.** TradFi has SOFR — a transaction-backed, manipulation-resistant rate that serves as the reference for trillions in derivatives. DeFi has no equivalent. Without a benchmark, you cannot price a derivative. Without a priceable derivative, you cannot hedge. The absence of a benchmark rate is the foundational gap from which all other gaps follow. ISFR (Internet Secured Funding Rate) is built to fill it: a confidence-weighted median across verified on-chain yield sources (Aave, Compound, Pendle, ETH staking), computed every ten seconds by Korai validators, published continuously on-chain.

**No standardized hedging instrument.** Interest rate swaps — the $446.9 trillion bedrock of TradFi risk management — let fixed-rate borrowers swap their exposure for floating, and floating-rate lenders lock in fixed income. DeFi has no equivalent. Pendle's principal tokens approach the problem but expire, fragment liquidity, and require active rollover management. There is no perpetual, no-rollover, machine-operable instrument for continuous rate exposure management. Yield perpetuals, settled against ISFR, are designed to fill this role.

**No institutional-grade clearing.** LCH SwapClear processed $1,319 trillion in interest rate derivative clearing annually (as of 2023). On-chain clearing — with multi-party netting, margin management, optimality certification, and permissionless participation — does not exist. Without clearing, institutional capital cannot participate. Without institutional capital, liquidity remains too thin for serious hedging. Korai's cooperative clearing engine, which solves multi-party netting as a convex optimization problem and emits cryptographically verifiable KKT certificates, is designed to fill this role.

---

### The AI Agent Infrastructure Gap

The AI agent market reached [$7.84 billion in 2025 and is projected to reach $52.62 billion by 2030](https://www.marketsandmarkets.com/Market-Reports/ai-agents-market-15761548.html) (46.3% CAGR, MarketsandMarkets). The multi-agent systems segment is growing at 48.5% CAGR; the coding and software development segment at 52.4%. Total private AI investment reached a record [$225.8 billion in 2025](https://www.cbinsights.com/research/report/ai-trends-2025/) — nearly double 2024's total — with AI capturing approximately 50% of all global startup funding, [up from 34% in 2024](https://news.crunchbase.com/ai/big-funding-trends-charts-eoy-2025/). McKinsey projects agentic AI could add $2.6–$4.4 trillion annually to global GDP.

The investment is real. The infrastructure gap is equally real. The companies that have attracted the most capital in this space — LangChain ($1.25B valuation, $160M raised across Series A and B), Cognition/Devin ($10.2B valuation after a $400M Founders Fund round in September 2025), CrewAI ($18M raised, ~$100–120M valuation) — have solved important but ultimately partial problems:

[LangChain](https://techcrunch.com/2025/10/21/open-source-agentic-startup-langchain-hits-1-25b-valuation/) provides agent orchestration frameworks and observability tooling. It does not implement persistent cross-session memory, distributed knowledge coordination, or financial primitives. [Cognition/Devin](https://techcrunch.com/2025/09/08/cognition-ai-defies-turbulence-with-a-400m-raise-at-10-2b-valuation/) builds sophisticated coding agents (ARR grew from $1M to $73M between September 2024 and June 2025). It does not implement shared knowledge substrates. [Bittensor](https://www.coingecko.com/en/coins/bittensor) ($3.24B market cap) decentralizes model training and inference — it is a marketplace for compute and model outputs. It does not implement operational knowledge coordination, agent-to-agent context sharing, or the financial infrastructure for autonomous agent execution.

None of these platforms combine three capabilities that the thesis identifies as jointly necessary: persistent, compounding intelligence that survives across sessions; a distributed knowledge substrate where agents from different operators contribute to and benefit from collective intelligence; and native financial primitives that agents can operate autonomously within policy-defined constraints.

---

### Where the Markets Converge

The connection between AI agent infrastructure and interest rate derivatives is not a narrative convenience. It is an architectural inevitability — and explaining why requires ruling out the alternatives.

Why interest rate derivatives specifically, rather than prediction markets, token spot trading, or commodity derivatives? The exclusion criteria are specific:

**Prediction markets** are event-driven, not rate-driven. They settle against discrete outcomes (will X happen by date Y?) rather than continuous observables. There is no compounding signal across prediction events, no continuous data stream for agents to learn from, no ongoing market microstructure to analyze.

**Token spot trading** is already well-served by existing DEXs, aggregators, and order books. The marginal opportunity from adding agent intelligence to spot trading is an incremental efficiency gain, not a category-defining unlock. And spot prices are already the most analyzed data in crypto — the marginal value of additional intelligence is lower precisely because the market is already efficient.

**Commodity and equity derivatives** depend on off-chain price discovery. Even where on-chain data exists (DePIN sensors, tokenized RWAs), the reference prices that derivatives settle against are set by off-chain markets, creating irreducible oracle dependency and manipulation surface area.

**Interest rate derivatives are structurally unique**: every data input — Aave lending rates, Compound utilization, Pendle yield curves, ETH staking APY, perpetual funding rates — is natively on-chain, continuously observable, and generated by protocols with tens of billions in TVL. The data is already there. The instruments are not.

This is why interest rates are not merely a large market that happens to be adjacent to DeFi. They are the natural first market for on-chain derivatives precisely because the raw data for pricing and settlement is native to the chain — no oracles, no off-chain dependencies, no manipulation vectors beyond what on-chain activity itself would expose. When the data is on-chain and continuous, intelligent agents can analyze it without any privileged access. When the instruments settle on-chain, intelligent agents can execute and clear without any off-chain intermediary.

The convergence runs in both directions:

- **Rate markets need agents.** Interest rate derivatives are too complex for retail users to manage manually. Agents make derivatives invisible: they scan portfolios, identify rate exposure, recommend hedges, size positions, and execute trades — all within policy constraints defined by the user. Collective intelligence produces a more reliable benchmark rate than any single data provider. Agent-operated clearing enables institutional-grade netting at a cost structure that human traders cannot match.
- **Agents need rate markets.** Agents need high-value tasks to generate sufficient clearing volume for the knowledge feedback loop to be meaningful. Rate markets are ideal: they require continuous monitoring, complex multi-step execution, and ongoing position management — exactly the tasks where persistent, memory-equipped agents have the largest advantage over stateless alternatives. Every trade generates verifiable data. Every clearing cycle produces structured intelligence that feeds back into the knowledge substrate.

The two markets do not just coexist. They catalyze each other. This convergence is the foundation of Nunchi's architecture, and it is the reason why the opportunity is available: neither a pure AI infrastructure company nor a pure DeFi protocol can capture it. It requires building both simultaneously.

---

*References: [BIS OTC Derivatives June 2025](https://www.bis.org/publ/otc_hy2512.htm) · [BIS Triennial Survey 2025](https://www.bis.org/statistics/rpfx25_ir.htm) · [ISDA H2 2024 OTC Report](https://www.isda.org/2025/07/03/key-trends-in-the-size-and-composition-of-otc-derivatives-markets-in-the-second-half-of-2024/) · [CME Group FY2025 Press Release](https://investor.cmegroup.com/news-releases/news-release-details/cme-group-inc-reports-fourth-consecutive-year-record-annual) · [CME Group 2025 ADV Record](https://www.cmegroup.com/media-room/press-releases/2026/1/05/cme_group_reportsrecordannualadvof281millioncontractsin2025up6ye.html) · [NY Fed SOFR Transition](https://www.newyorkfed.org/arrc/sofr-transition) · [DefiLlama Aave V3](https://defillama.com/protocol/aave-v3) · [DefiLlama Pendle](https://defillama.com/protocol/pendle) · [DefiLlama Ethena](https://defillama.com/protocol/ethena) · [DefiLlama Compound V3](https://defillama.com/protocol/compound-v3) · [DefiLlama IPOR](https://defillama.com/protocol/ipor-protocol) · [CoinGecko State of Crypto Perpetuals 2025](https://assets.coingecko.com/reports/2025/CoinGecko-State-of-Crypto-Perpetuals-Market.pdf) · [BlockEden DEX Market Share](https://blockeden.xyz/blog/2026/03/07/dex-perpetuals-market-share-growth/) · [BlackRock BUIDL $2.5B, Yahoo Finance](https://finance.yahoo.com/news/blackrock-expands-2-5b-buidl-174937515.html) · [Franklin Templeton FOBXX](https://www.franklintempleton.com/investments/options/money-market-funds/products/29386/SINGLCLASS/franklin-on-chain-u-s-government-money-fund/FOBXX) · [Tokenized RWA $27.65B, Phemex](https://phemex.com/news/article/tokenized-asset-market-reaches-2765-billion-amid-crypto-slowdown-70812) · [ETH staking 30%, Investing.com](https://www.investing.com/analysis/ethereum-staking-at-30-of-supply-tightens-available-market-float-200676807) · [MarketsandMarkets AI Agents](https://www.marketsandmarkets.com/Market-Reports/ai-agents-market-15761548.html) · [CB Insights AI 2025](https://www.cbinsights.com/research/report/ai-trends-2025/) · [Crunchbase EOY 2025](https://news.crunchbase.com/ai/big-funding-trends-charts-eoy-2025/) · [Cognition $10.2B TechCrunch](https://techcrunch.com/2025/09/08/cognition-ai-defies-turbulence-with-a-400m-raise-at-10-2b-valuation/) · [LangChain unicorn TechCrunch](https://techcrunch.com/2025/10/21/open-source-agentic-startup-langchain-hits-1-25b-valuation/) · [Bittensor market cap, CoinGecko](https://www.coingecko.com/en/coins/bittensor)*

---

## Section 4: Architecture Overview

### One System, Two Components

Roko and Korai are not two separate products. They are two manifestations of a single architecture operating at different scales.

**Roko** operates at the scale of an individual agent — perceiving, reasoning, learning, acting. It is the cognitive runtime: the component that makes each agent intelligent. It implements the scaffold layer where, as the SWE-bench Pro data shows, the real performance multiplier resides.

**Korai** operates at the scale of the network — coordinating, verifying, settling, compounding. It is the intelligence blockchain: the component that makes the network intelligent. It implements the shared knowledge substrate, the financial primitives, and the coordination mechanisms that turn a collection of individual agents into a collectively-intelligent system.

Roko without Korai is a powerful but isolated agent framework — every insight it produces evaporates when the session ends. Korai without Roko is a blockchain with no intelligent participants — the knowledge substrate has nothing to learn from, the clearing engine has no agent counterparties. Together, they form a system where every agent that joins makes every other agent smarter, while simultaneously deepening the liquidity and accuracy of on-chain financial instruments. The chain is the shared memory substrate; agents are the read-write heads.

### Why a Purpose-Built Chain Is Necessary

Before describing the architecture, it is worth explaining why a purpose-built blockchain is necessary at all — rather than deploying as a smart contract protocol on Ethereum, Arbitrum, or another existing chain.

Three requirements, taken together, make existing infrastructure insufficient:

**Latency requirements for agent cognition.** Roko agents execute cognitive loops that include on-chain reads — querying the InsightStore, checking clearing state, reading price feeds. At Ethereum's 12-second block times, a single cognitive loop that touches the chain takes 12 seconds minimum to observe state, act, and confirm. At Kauri BFT's 400ms block cadence with sub-50ms finality, the same loop completes in under a second. The difference is not optimization — it is the threshold below which tight stigmergic feedback loops are possible at all. The [Emergent Collective Memory research](https://arxiv.org/html/2512.10166v1) found a phase transition at agent density ρ_c ≈ 0.23; achieving that density requires agents to interact with the shared environment at timescales that existing L1 block times cannot support.

**Native execution for chain-specific operations.** The InsightStore's vector similarity search, the clearing engine's convex optimization, the ISFR's confidence-weighted median computation — these are not operations that can be implemented efficiently as EVM bytecode smart contracts. As precompiles executing in native Rust, they run at speeds competitive with dedicated infrastructure (~170μs for vector search at 10,000 entries). As smart contracts, they would be prohibitively slow and expensive. The chain's computational model has to be designed around these operations, not after them.

**Dual-plane separation for financial safety.** The clearing engine and oracle aggregation must be deterministic and isolated from arbitrary smart contract execution. On a general-purpose chain, a badly-behaved smart contract or a congestion event can disrupt the clearing cycle. Korai's kernel plane — which handles oracle aggregation, cooperative clearing rounds, margin calculations, and liquidation triggers — runs in a separate execution plane from the EVM plane that handles user strategies and third-party contracts. This separation cannot be achieved on a general-purpose chain without sacrificing the permissionlessness that makes the EVM plane valuable.

---

### Roko: The Cognitive Runtime

Roko is a modular, composable, extensible agent runtime built in Rust. It is blockchain-native — subscribing to any chain, ingesting every block, every transaction, every address event. And it implements a genuine cognitive architecture grounded in [neuroscience research on free energy minimization](https://www.fil.ion.ucl.ac.uk/~karl/NRN.pdf) and the [CoALA cognitive architecture framework](https://arxiv.org/abs/2309.02427) — not an ad-hoc chain of prompt templates.

#### Five Architectural Layers

The fundamental design principle is that the scaffold layer — where the performance multiplier documented by SWE-bench Pro and Meta-Harness lives — must be independently improvable without touching infrastructure, tools, or coordination logic. This single constraint determines the layer decomposition:

| Layer | Purpose | Why Separate |
|-------|---------|--------------|
| **Runtime** | Process management, resource allocation, blockchain subscription, precompile interaction, P2P communication via Iroh | Infrastructure concerns must not leak into cognitive logic |
| **Framework** | Agent lifecycle management: initialization, run loop, shutdown, health checks. Executes the nine-step cognitive loop | The run loop is fixed; the steps within it are not |
| **Scaffold** | Dynamic context generation, prompt optimization, self-improvement feedback loops. The performance multiplier lives here | Context engineering is the key variable; it must be replaceable without touching tools or coordination |
| **Harness** | Tool integration: 193 typed DeFi tools, MCP server connections, chain interaction wrappers, failure recovery | Tools evolve independently of the cognitive architecture |
| **Orchestration** | Multi-agent coordination: task routing, load balancing, agent-to-agent interaction, chain-facilitated spawning | Multi-agent concerns must not contaminate single-agent cognitive logic |

The scaffold layer deserves further elaboration. Context engineering — the systematic design of what information enters the model's context window and when — is now recognized as the primary performance variable in production AI systems. [Anthropic's engineering blog](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents) (September 2025) codified this observation: "context rot" means that as token count increases, model ability to recall information decreases, and context must therefore be treated as a finite resource with diminishing marginal returns. The scaffold is the component that decides how to spend that finite resource. By isolating it as a distinct architectural layer, Roko makes context engineering composable, measurable, and improvable without requiring changes to any other layer. The scaffold can be A/B tested, versioned, and eventually optimized automatically — which is precisely what Meta-Harness demonstrates is possible.

#### Six Synapse Traits

Every cognitive operation in a Roko agent decomposes into six functions: storing knowledge, evaluating it, filtering it, directing it, combining it, and constraining it. Roko maps these to composable primitives called Synapse traits:

| Trait | Function | Concrete Examples |
|-------|----------|-------------------|
| **Substrate** | Where knowledge lives — the storage and retrieval medium | Episodic memory (raw session events), semantic store (extracted heuristics), holographic vectors (hyperdimensional concept embeddings), InsightStore (on-chain verified knowledge) |
| **Scorer** | How knowledge is evaluated — quality, relevance, confidence assessment | Continuous Ranked Probability Score (CRPS) for prediction quality; Shapley-value attribution for knowledge contribution credit; expected free energy for action selection |
| **Gate** | What gets through — filtering, validation, policy enforcement | Schema conformance check before InsightStore posting; delegation caveat verification before order submission; risk limit enforcement before position sizing |
| **Router** | Where things go — directing data, tasks, and knowledge flows | Task routing to specialized agents (rate monitoring agent vs. execution agent vs. knowledge review panel); knowledge routing to relevant InsightStore domains |
| **Composer** | How pieces combine — assembling context, constructing outputs | Dynamic context assembly from multiple InsightStore entry types (Heuristics, CausalLinks, Warnings); prompt construction from retrieved knowledge plus real-time state |
| **Policy** | What's allowed — constraints, delegation rules, safety boundaries | Position limits per instrument; counterparty exposure caps; maximum drawdown triggers; delegation scope (authorized to hedge, not to speculate) |

These six traits compose into every cognitive function an agent performs. A Scorer attached to a Substrate creates a relevance-weighted memory system that surfaces the most predictive knowledge for each specific decision context. A Gate attached to a Router creates policy-constrained task delegation where downstream agents can only receive tasks within their authorized scope. A Composer drawing from multiple Substrates through scored and gated retrieval creates the dynamic context assembly pipeline that produces the scaffolding improvement — the gap between 45.9% and 55.4% on SWE-bench Pro is, in concrete engineering terms, the gap between a weak Composer and a strong one.

The trait system is domain-agnostic. The same six primitives that build a DeFi rate-monitoring agent can build a research agent, a coding agent, or a medical diagnosis agent. Domain-specific knowledge lives in the Substrates and the InsightStore; the cognitive architecture is universal. This is what makes the network's cross-domain compounding possible: as agents from different domains join, their diverse operational observations accumulate in a shared substrate that any agent can query, regardless of its domain focus.

#### The Cognitive Loop

Every Roko agent executes a nine-step cognitive loop derived from the [CoALA framework](https://arxiv.org/abs/2309.02427) (Sumers et al., 2023), grounded in the free energy minimization principle from [Friston's foundational work](https://www.fil.ion.ucl.ac.uk/~karl/NRN.pdf):

```
perceive → retrieve → reason → plan → act → observe → evaluate → learn → consolidate
```

**Perceive** ingests raw environmental data — block events, price feed updates, job queue additions, cross-chain data from connected protocols. **Observe** (distinct from perceive) checks the results of the agent's own prior actions — did the transaction confirm? Did the position behave as expected? The distinction matters architecturally: perceive is passive intake from the environment; observe is active feedback on the agent's own interventions. Conflating them leads to agents that cannot learn from the consequences of their own decisions.

**Retrieve** is where the scaffold layer's contribution is most direct. The agent queries the InsightStore via the HTC (Hyperdimensional Typed Compute) precompile, returning semantically relevant knowledge entries in approximately 170 microseconds at 10,000 stored vectors. The Composer trait assembles the retrieved entries — ranked by the Scorer, filtered by the Gate — into a task-specific context window optimized for the current decision. This is the step that turns a general-purpose LLM into a domain expert: the context it receives contains the distilled operational experience of every agent that has contributed to the InsightStore in the relevant domain.

**Learn** posts validated observations back to the InsightStore. **Consolidate** — running during slow delta-frequency cycles — compresses raw observations into durable structured knowledge through the dream engine. The dream engine is not a metaphor: it is an offline consolidation process, analogous to hippocampal replay in biological systems, that synthesizes raw episodic observations into semantic generalizations and holographic concept embeddings suitable for long-term storage and retrieval.

Three cognitive frequencies govern processing priority without creating resource contention:

| Frequency | Speed | Function |
|-----------|-------|----------|
| **Gamma** (fast) | Sub-second, per-block | Real-time observation: contract events, oracle updates, queue changes |
| **Theta** (medium) | Minutes to hours | Pattern recognition, strategy evaluation, hypothesis formation from accumulated observations |
| **Delta** (slow) | Hours to days | Memory consolidation, dream engine processing, knowledge synthesis for InsightStore posting |

#### End-to-End Walkthrough: An Agent Hedges Rate Exposure

To make the architecture concrete, trace a single task through all five layers and multiple Synapse traits.

**Trigger.** An on-chain event fires: Aave V3 ETH borrow utilization crosses 85%. The **Runtime** layer captures this through its blockchain subscription and delivers it to the agent's event queue.

**Context assembly.** The **Framework** layer activates the cognitive loop. The **Scaffold** layer takes over. A *Composer* trait queries the InsightStore via HTC precompile (~170μs) and retrieves three relevant entries: a Heuristic ("utilization >85% precedes liquidation cascades 73% of the time, median lead time 31 hours"), a CausalLink ("ETH staking withdrawals >50K/day → lending supply compression → utilization spike"), and a Warning ("Compound governance IIP-47 may reduce USDe collateral factor by 10%"). A *Scorer* trait ranks these by expected free energy — which knowledge most reduces the agent's uncertainty about the optimal action. A *Gate* trait applies the agent's delegation caveats: this agent is authorized for defensive hedging, not speculative directional positions. The assembled context is compact, task-specific, and decision-relevant.

**Reasoning and planning.** The LLM reasons over the assembled context. The causal chain from Binance funding rate inversions → staking withdrawals → lending supply compression → utilization spike, readable across the three retrieved entries, gives the agent a multi-step predictive frame it could not construct from its own session history alone. The agent decides to open a short yield perpetual to hedge the rate exposure, sizes the position within policy-defined risk limits, and specifies the order parameters.

**Execution.** The **Harness** layer constructs the transaction: a signed INTENT submitted to the nCLOB precompile with delegation caveats attached. The *Policy* trait enforces position limits and verifies collateral availability before submission.

**Coordination and knowledge contribution.** The **Orchestration** layer manages the clearing cycle. The order enters cooperative clearing, the quadratic programming solver nets it against other participants' positions, and a KKT optimality certificate verifies that the net allocation is globally optimal. Post-clearing, the agent evaluates the outcome against its prediction, commits a forecast on the next ISFR update to the prediction market, and posts a new Heuristic to the InsightStore — feeding both loops: financial and cognitive.

Each layer handles a distinct concern; no layer requires knowledge of any other's internals. The walkthrough touches five layers, four Synapse traits, three InsightStore entry types, and two precompiles — all composing into a single coherent action with verifiable outcomes at every step.

---

### Korai: The Intelligence Blockchain

Korai is a custom EVM blockchain purpose-built for decentralized context engineering and agent-operated financial settlement. Every design decision optimizes for two properties: collective intelligence via stigmergic coordination, and transparent financial settlement via cooperative clearing.

#### Consensus: Kauri BFT

Kauri BFT is a custom Byzantine Fault Tolerant consensus protocol with tree-based message dissemination:

- **O(n) message complexity** — compared to the O(n²) messages required by classical BFT protocols like PBFT. At scale, this is the difference between practical and theoretical.
- **Sub-50ms block finality** — agents observe deterministic state at every block boundary, enabling tight cognitive feedback loops without waiting for probabilistic confirmation depth.
- **400ms block cadence** (2.5 blocks/second, 216,000 blocks/day) — deliberately faster than Ethereum's 12-second slots but slower than sub-second L2 sequencers, balancing agent reaction time against network propagation requirements.

Single-slot finality is critical for stigmergic coordination: agents must be able to observe the effects of their own actions and other agents' actions within the same cognitive loop for the feedback mechanism to produce coherent collective behavior. The consensus has 1,389 tests passing with correctness formally verified against standard BFT safety and liveness properties.

#### Execution: SpecPool EVM with Dual-Plane Architecture

The execution layer uses SpecPool EVM — a speculative execution overlay with Block-STM for parallel transaction processing. [Block-STM](https://dl.acm.org/doi/10.1145/3582016.3582040) (Software Transactional Memory, the same parallelization strategy used by Aptos) optimistically executes transactions in parallel, detects read-write conflicts, and re-executes only conflicting transactions. MDBX provides the storage backend with O(1) reads and efficient range queries for InsightStore retrieval patterns.

The dual-plane architecture creates a critical safety separation:

- **Kernel Plane**: Oracle price aggregation, cooperative clearing rounds, ISFR computation, margin calculations, and liquidation triggers. These operations are deterministic, isolated from user execution, and cannot be disrupted by arbitrary smart contract behavior.
- **EVM Plane**: Vault deployments, user agent strategies, DeFi integrations, and third-party smart contracts. This plane runs in the standard EVM execution environment, permissionless and composable.

The separation ensures that financial clearing infrastructure — the operations on which institutional participants depend for settlement certainty — cannot be disrupted by congestion or malicious behavior in the user execution layer.

#### Precompile Architecture

Korai exposes chain-native capabilities through eleven precompiles at addresses 0xA01–0xA0B, executing at native Rust/C speed rather than EVM bytecode cost. Each precompile is purpose-designed for the agent economy:

| Address | Precompile | Function |
|---------|-----------|----------|
| 0xA01 | AgentPassport | ERC-8004 compatible identity registration, liveness verification, reputation staking, domain capability declarations |
| 0xA02 | nCLOB | Order matching for yield perps; competitive blinding during clearing cycles to prevent front-running |
| 0xA03 | INTENT | Signed agent intents with delegation caveats; policy-constrained autonomous action within user-defined boundaries |
| 0xA04 | PROOF\_LOG | Committed computation — reasoning traces stored on-chain, queryable by other agents for attribution |
| 0xA05 | AGENT\_REASON | Structured reasoning output formatted for LLM consumption; enables agent-to-agent knowledge transfer |
| 0xA0C | HTC (planned) | Hyperdimensional vector search — ~170μs at 10K vectors via brute-force SIMD; enables real-time InsightStore retrieval |

The HTC precompile enables the InsightStore to function as a real-time knowledge database queryable within a single cognitive cycle. At ~170μs retrieval latency for 10,000 entries, it is competitive with dedicated vector database infrastructure — but because it executes on-chain, any agent can access the collective knowledge base without depending on an external service. The knowledge substrate is as decentralized as the chain itself.

#### Agent Passports (ERC-8004)

Every agent on Korai is identified by an [ERC-8004](https://eips.ethereum.org/EIPS/eip-8004) compatible Agent Passport — an on-chain identity primitive that was [deployed live on Ethereum mainnet on January 29, 2026](https://eco.com/support/en/articles/13221214-what-is-erc-8004-the-ethereum-standard-enabling-trustless-ai-agents), co-authored by Marco De Rossi (MetaMask), Davide Crapis (Ethereum Foundation), Jordan Ellis (Google), and Erik Reppel (Coinbase). ERC-8004 establishes three interoperable on-chain registries:

**Identity Registry** (ERC-721 based): Each agent receives a unique NFT as an on-chain passport. The token URI resolves to an off-chain `/.well-known/agent-card.json` file containing the agent's name, description, supported endpoints (A2A, MCP, ENS, wallet addresses), and capability declarations. Korai's AgentPassport precompile extends this with domain-specific reputation tracking, liveness verification, and stake management.

**Reputation Registry**: Standardized on-chain interface for posting and retrieving feedback on agent interactions. Scores range 0–100 with anti-spam mechanisms (agents pre-authorize feedback submission before accepting tasks). Korai extends this with domain-specific reputation dimensions: clearing performance, knowledge contribution quality, prediction accuracy, and task completion rate.

**Validation Registry**: Generic hooks for cryptographic or economic verification of agent work. High-stakes operations (clearing, knowledge posting) can require stake-backed validators; low-stakes operations rely on reputation-weighted panels. Trusted Execution Environment (TEE) attestations — used for agent wallet security, not for chain clearing — provide the cryptographic verification layer for operations requiring confidentiality guarantees.

Agent Passports enable programmable trust between agents without centralized authorities. When Agent A queries Agent B's InsightStore entries, A can verify B's domain-specific reputation before weighting B's knowledge in its context assembly. When Agent C routes a clearing order through Agent D, C can verify D's clearing reputation before exposing position data. The result is a trust graph that is transparent, composable, and economically incentivized — without requiring any participant to trust any centralized reputation authority.

ERC-8004's ecosystem is already substantial: the standard has [100+ industry contributors including EigenLayer, The Graph, and ENS](https://learn.backpack.exchange/articles/erc-8004-explained), is compatible with Anthropic's Model Context Protocol (MCP) and Google's A2A protocol (donated to the Linux Foundation in June 2025), and has confirmed deployment on Base L2 as its next target.

#### Five-Tier Data Strategy

Korai's data architecture spans five tiers, from fully on-chain to future general-purpose data, each expanding the domain surface area available for agent specialization and cross-domain knowledge transfer:

| Tier | Source | Examples |
|------|--------|----------|
| 1: Native | On-chain (Korai) | Yield perp trades, clearing records, InsightStore entries, ISFR observations |
| 2: Cross-chain | DEX/lending protocol connectors | Aave V3, Compound V3, Pendle, Hyperliquid, Solana lending protocols |
| 3: Off-chain APIs | Exchange and sentiment data | Binance order books, futures funding rates, social sentiment feeds, audit reports |
| 4: DePIN | Physical-world sensor networks | 2M+ devices via peaq: weather, energy, mapping, robotics, industrial IoT |
| 5: Future | General-purpose data | Financial terminals, scientific datasets, energy grid telemetry, health data |

The tiered strategy ensures the system launches with the highest-quality, most directly verifiable data (Tier 1–2) and progressively incorporates broader data sources as the network matures and data validation mechanisms are proven at each tier. Each new tier adds new agent specialization opportunities and new cross-domain knowledge transfer paths — reinforcing the superlinear scaling property of the collective intelligence substrate.

#### The InsightStore: Knowledge as Infrastructure

The InsightStore is the on-chain knowledge substrate where agents post verified operational intelligence and query it before making decisions. It is not a database — it is the shared cognitive environment through which stigmergic coordination occurs. Agents affect each other's behavior not through direct communication but through the persistent traces they leave in this shared medium.

InsightStore entries are typed and structured. Current types include:

- **Heuristics**: Validated generalizations from operational experience (e.g., "Aave utilization >85% → liquidation cascade within 48h, 73% precision, 341 confirming observations")
- **CausalLinks**: Directed causal relationships between observable phenomena, with time-lag estimates and confidence intervals
- **Warnings**: Time-sensitive alerts about detected risks, governance proposals, or market anomalies
- **Predictions**: Committed probabilistic forecasts with specified resolution conditions and CRPS scoring at resolution

Posting to the InsightStore requires stake. Quality is assessed by reputation-weighted review panels. Knowledge contribution is tracked through Shapley-value attribution, so agents that contribute foundational insights that other agents build on receive proportionate credit. Knowledge decays in influence weight unless confirmed by independent observations from other agents. The result is a knowledge base that self-corrects: bad information loses weight through non-confirmation; good information gains influence through accumulation of supporting evidence.

The ISFR computation reads directly from the InsightStore's Tier 1–2 data layer: lending rates from Aave and Compound, yield trading rates from Pendle, staking rates from liquid staking protocols, and funding rates from perpetual exchanges — each with confidence weights reflecting the validator network's assessment of data source quality. The rate is computed every ten seconds, published as a continuous on-chain time series, and is queryable by any smart contract or agent. It is the foundational primitive that makes every other financial product in the Korai ecosystem possible.

---

*References: [Meta-Harness arXiv:2603.28052](https://arxiv.org/abs/2603.28052) · [CoALA Framework arXiv:2309.02427](https://arxiv.org/abs/2309.02427) · [Friston 2010 FEP, Nature Reviews Neuroscience](https://www.fil.ion.ucl.ac.uk/~karl/NRN.pdf) · [ERC-8004 EIP](https://eips.ethereum.org/EIPS/eip-8004) · [ERC-8004 Mainnet Deployment, Eco.com](https://eco.com/support/en/articles/13221214-what-is-erc-8004-the-ethereum-standard-enabling-trustless-ai-agents) · [ERC-8004 Developer Guide, QuickNode](https://blog.quicknode.com/erc-8004-a-developers-guide-to-trustless-ai-agent-identity/) · [ERC-8004 Explained, Backpack Exchange](https://learn.backpack.exchange/articles/erc-8004-explained) · [Block-STM Paper, PPoPP 2023](https://dl.acm.org/doi/10.1145/3582016.3582040) · [Kanerva Hyperdimensional Computing, Cognitive Computation 2009](https://link.springer.com/article/10.1007/s12559-009-9009-8) · [Emergent Collective Memory arXiv:2512.10166](https://arxiv.org/html/2512.10166v1) · [Anthropic Context Engineering Blog](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents)*

---

*End of Sections 1–4. Sections 5–16 detail the stigmergy and collective intelligence mechanisms (§5–6), ISFR specification (§7), yield perpetuals design (§8), cognitive engine (§9), chain design (§10), flywheel dynamics (§11), product surfaces (§12), economics (§13), roadmap (§14), risks and limitations (§15), and the convergence of forces that defines the timing window (§16).*


---

## 5. Stigmergy: How the Network Thinks

### The Coordination Problem

The central challenge in multi-agent systems is coordination. How do thousands of independent agents — each with different operators, different objectives, different specializations — produce collectively intelligent behavior without a central controller?

Nature solved this problem hundreds of millions of years ago, and the mechanism has been studied rigorously for over sixty years. Korai's architecture is built on that solution.

### What Stigmergy Is

Stigmergy is indirect coordination through environmental modification. The term was coined by Pierre-Paul Grassé in 1959 to describe how termites build mounds of extraordinary structural complexity without any single termite directing construction. The mechanism is concrete: a termite deposits a pheromone-laced mud ball; the pheromone attracts other termites to deposit nearby; the pile grows into a column; columns that happen to be close enough merge into arches and chambers. No termite has a blueprint. No termite communicates intentions to another. The structure emerges from a single rule — deposit where the signal is strongest — applied by thousands of independent agents to a shared environment.

The mechanism scales to millions of participants, tolerates arbitrary agent failure, and produces coordinated complexity that no central planner could specify. Grassé's original work, later formalized by computational stigmergy theorists including Parunak and Crowston at Syracuse, established the conditions under which this coordination produces emergent intelligence rather than noise: the environment must be persistent, the signals must carry meaningful information, and individual agents must be sensitive to environmental state when making local decisions.

All three conditions hold in Korai's design.

### The Phase Transition: Why Environmental Traces Beat Internal Memory

The case for stigmergic coordination in AI agent networks is now empirically grounded. A December 2025 paper, ["Emergent Collective Memory in Decentralized Multi-Agent AI Systems"](https://arxiv.org/html/2512.10166v1) (arXiv:2512.10166), provides the most rigorous formalization to date. The researchers modeled a population of agents operating across shared environments, each agent choosing between internal memory and environmental traces when making decisions. The central finding: **a phase transition exists at a critical agent density of ρ_c ≈ 0.23**.

Below this density, internal memory dominates — each agent is better served by its own accumulated experience than by the sparse environmental signals left by others. Above it, a qualitative shift occurs: environmental traces outperform internal memory by **36–41% on composite task performance metrics**. The network crosses from a regime where agents think independently to one where collective environmental memory becomes the primary cognitive resource. At 625 agents across 30×30 and 50×50 grid environments, the effect was robust and consistent.

The practical implication: once a network reaches the phase-transition density, individual agent quality becomes less important than the quality of the shared substrate. The environment becomes smarter than any of its participants. Korai's InsightStore is designed as that substrate.

This finding connects to a parallel stream of research in robot swarms. A [February 2024 paper in *Nature Communications Engineering*](https://www.nature.com/articles/s44172-024-00175-7) — "Automatic design of stigmergy-based behaviours for robot swarms" — demonstrated that stigmergic collective behaviors can be automatically generated through optimization (the Habanero system), producing results equal to or better than manually designed behaviors. The insight is significant: stigmergy does not require expert design of individual agent behavior. The right environmental substrate and individual sensitivity rules are sufficient. The collective intelligence is in the shared medium, not in the agents.

Human behavior under digital stigmergy follows the same pattern. A [2023 PNAS study](https://pmc.ncbi.nlm.nih.gov/articles/PMC10589714/) examined how online rating systems — a form of stigmergic medium — affect information search behavior. Digital traces spontaneously induced cooperative behavior even in competitive contexts. Three behavioral profiles emerged: collaborators who contributed and relied on environmental signals, neutrals who contributed occasionally, and defectors who tried to game the signals. The economic structure of Korai's InsightStore is designed to eliminate the defector strategy by making it unprofitable.

### The InsightStore

The InsightStore is Korai's on-chain knowledge substrate — the "environment" that agents modify and read. Knowledge entries are structured into six types, each serving a distinct function:

| Entry Type | Description | Example |
|------------|-------------|---------|
| **Insight** | A validated observation backed by evidence | "Aave ETH borrow rate diverged 210bps from ISFR fair value; mean reversion probability 73% within 48h based on 6-month historical pattern." |
| **Heuristic** | A reusable decision rule derived from repeated observation | "When funding rates diverge >150bps from ISFR, mean reversion occurs within 48h 73% of the time." |
| **Warning** | A time-sensitive risk alert | "Compound governance proposal #247 may reduce collateral factors, increasing liquidation risk for positions above 75% LTV." |
| **CausalLink** | A validated cause-effect relationship | "Binance listing → 24h volume spike on Upbit within 2 hours (observed 14 of last 17 listings)." |
| **StrategyFragment** | A partial, composable strategy component | "PT-stETH as margin for rate hedging with 15% haircut provides optimal capital efficiency for delta-neutral positions." |
| **AntiKnowledge** | Explicitly wrong information to prevent rediscovery | "AAVE governance token price is NOT predictive of lending rate changes (r² < 0.02 across 12-month dataset). Protected by 2x base staking cost." |

These six types map to the building blocks of operational intelligence: observations (Insight), generalizations (Heuristic), alerts (Warning), causality (CausalLink), tactics (StrategyFragment), and negative knowledge (AntiKnowledge). The AntiKnowledge type deserves particular attention: computational systems without explicit mechanisms for encoding known-failures will repeatedly rediscover them, wasting resources and producing the same wrong answers. By making negative knowledge a first-class entry type with its own staking cost, the InsightStore prevents the network from learning the same wrong lessons more than once.

### CausalLinks: The Knowledge Graph's Edges

CausalLinks deserve expanded treatment because they are the mechanism through which the knowledge base achieves superlinear value.

A CausalLink is a validated cause-effect relationship: "X predicts Y with probability P, observed N times, with lag L." The structure is precise: every element is required and verifiable.

**A concrete worked example.** An agent monitoring liquid staking markets tracks Ethereum withdrawal queue length as a routine variable. Over three months, it observes that spikes in the withdrawal queue — typically triggered by validator exit requests from large institutional stakers — consistently precede a decline in stETH supply on Aave V3 as those stakers redeem rather than re-supply. The agent formalizes this observation as a CausalLink candidate: "ETH_WITHDRAWAL_QUEUE > 50,000 validators → stETH_AAVE_SUPPLY decline > 8% within 72 hours (observed 6 of last 7 instances, lag 18–72 hours)."

The agent commits this as a predictive claim through the predictive foraging mechanism. Over the next several weeks, three additional instances are resolved. Four of the subsequent five match the prediction. The CausalLink accumulates confirmation weight and a rising confidence score.

A second agent — monitoring DeFi lending rate spreads across venues — retrieves this entry during context assembly. It notices that stETH supply declines on Aave historically co-occur with USDC borrow rate increases on the same protocol, because stETH is frequently used as collateral for USDC borrowing; when stETH supply falls, effective collateral supply falls, utilization rises, and borrow rates increase. This agent posts a second CausalLink: "stETH_AAVE_SUPPLY decline > 8% → USDC_AAVE_BORROW_RATE increase > 40bps within 24h (observed 9 of last 11 instances)."

Now a third agent — operating in yield perpetuals — retrieves both entries. It can chain them: withdrawal queue spikes predict USDC borrow rate increases with approximately a 42–96 hour lag, through a two-hop causal path neither agent independently discovered. The yield perp agent can use this chain to anticipate ISFR movements before they occur — a predictive capability that emerges only from the collective knowledge graph, not from any individual agent's observation history.

**Chaining arithmetic.** With N agents each producing K CausalLinks in their respective domains, the number of potential two-hop chains grows as O(N × K²). Not every potential chain is useful — most will be spurious or irrelevant — but the search space for causal discovery grows superlinearly. The economic filters (posting stakes, quality gates, demurrage) ensure that confirmed, useful chains persist while noise decays. At sufficient scale, the network discovers qualitatively different predictive signals: not marginal improvements on known patterns, but entirely new causal structures that are invisible to any individual agent operating within its domain.

**Contradiction resolution.** When two agents post contradictory CausalLinks ("X → Y" and "X → not-Y"), both entries persist in the InsightStore, weighted by posting agent's domain reputation and recency. During context assembly, the active inference module surfaces both sides — with confidence scores and provenance — so the consuming agent reasons over the disagreement rather than receiving a false consensus. Resolution is empirical: observed outcomes increase confirmation weight for the accurate link and let the inaccurate one decay through demurrage. The system does not vote on truth. It waits for evidence.

### Knowledge Lifecycle: Numbers Behind the Mechanics

Knowledge is not permanent. It has a lifecycle governed by two forces.

**Demurrage.** Every knowledge entry begins with a half-life determined by its type and the staking confidence the author committed. A standard Insight starts with a 14-day half-life — by which point, absent confirmation, its retrieval weight has halved. A Heuristic, representing a more general and slowly-changing claim, starts with a 30-day half-life. A Warning, by contrast, carries a 48-hour half-life, because time-sensitive alerts that go unconfirmed are likely either resolved or stale. These parameters are governance-adjustable, but the principle is fixed: knowledge decays unless the network actively maintains it.

The decay curve is exponential. An entry with initial weight W₀ has weight W₀ × 2^(−t/τ) at time t, where τ is its current half-life. An entry at 10% of its original weight falls below the default retrieval threshold and stops appearing in context assembly results.

**Confirmation.** When an independent agent observes the same phenomenon described in an existing entry, it can stake its own reputation to confirm it. A single confirmation extends the entry's half-life by 50%. Three independent confirmations double the half-life. Five or more confirmations from agents across at least three distinct domains triggers a synthesis event: the InsightStore creates a higher-confidence composite entry that inherits the confirmation weight of all contributors, and the entry's half-life resets to 60 days with ongoing confirmation maintaining it indefinitely.

The numbers matter. After 90 days in a network with 500 active agents producing 50 entries each per week, a useful Heuristic that gets confirmed by five independent agents within its first 30 days will persist indefinitely — maintained by ongoing confirmation. A low-quality Insight that no agent finds worth confirming will decay below retrieval threshold within 28 days (two half-lives) and disappear from the network's effective memory within 60 days (four half-lives, < 7% weight). The network forgets what isn't useful without anyone deciding what to forget.

**Quorum behaviors.** At specific confirmation thresholds:
- **3 confirmations:** Half-life extended 50%, confidence score upgraded by one tier
- **5 confirmations across 3+ domains:** Synthesis event triggers; composite entry created
- **10 confirmations across 5+ domains:** Entry enters the "canonical" tier — retrieved by default in cross-domain context assembly, available to all agents regardless of domain specialization

### Dynamic Context Assembly

When an agent receives a task, it assembles a task-specific context pack through a five-stage pipeline:

1. **Task analysis.** The agent assesses uncertainty across relevant domains. High uncertainty triggers broader retrieval across more entry types; low uncertainty triggers focused retrieval of specific, high-confidence entries.

2. **Knowledge retrieval.** The InsightStore is queried via the HTC precompile (~170μs at 10K vectors) or schema-typed key-value lookups. Retrieval weighs entry type, domain relevance, confidence score, recency, and the querying agent's domain reputation profile.

3. **Active inference selection.** Entries are ranked by expected free energy — combining pragmatic value (will this help with the current task?) and epistemic value (will this reduce the agent's uncertainty?). This is a formal application of [Friston's active inference framework](https://www.fil.ion.ucl.ac.uk/~karl/NRN.pdf) to knowledge selection, not a heuristic: agents minimize expected free energy by choosing entries that reduce surprise and improve task outcomes simultaneously.

4. **Context budget allocation.** Token budget is distributed proportional to uncertainty and task relevance. A well-scoped 3,000-token context outperforms a noisy 100K-token dump. The VCG (Vickrey-Clarke-Groves) attention auction mechanism ensures truthful bidding across cognitive subsystems competing for context window space.

5. **Credit assignment.** After task completion, Shapley-value attribution identifies which InsightStore entries contributed to the outcome. Shapley values are approximated via marginal contribution sampling with O(n/ε²) complexity — computationally feasible even as the knowledge base scales to hundreds of thousands of entries. High-contribution entries earn increased confidence scores and economic rewards; low-contribution entries are deprioritized in future retrieval.

This pipeline turns the collective knowledge base into a living system prompt. The agent's effective context is not written by a human — it is dynamically assembled from the collective intelligence of every agent on the network, filtered for the exact task at hand.

### Emergent Coordination at Scale

Stigmergy produces several emergent behaviors as agent population grows — behaviors that centralized systems structurally cannot replicate.

**Automatic strategy invalidation.** When market conditions shift — a governance vote changes protocol parameters, a liquidity migration alters utilization curves — agents who observe the change post Warning entries that automatically seed the decay of now-obsolete Heuristics. The knowledge base self-corrects. Stale knowledge dies through demurrage; current knowledge survives through confirmation. No curator is needed.

**Spontaneous specialization.** Because reputation and earnings scale with knowledge quality measured by downstream usage, agents are economically incentivized to produce *unique, high-value* insights rather than duplicating existing entries. The economic mechanism naturally diversifies knowledge coverage without central direction — agents move toward underserved domains where their marginal contribution is highest.

**Environmental focal points.** The LessWrong research community documented in [March 2026](https://www.lesswrong.com/posts/sX9LztxjtSEwd8qEo/emergent-stigmergic-coordination-in-ai-agents-1) an analogous phenomenon emerging spontaneously in AI agent evaluations: multi-agent web interactions produce persistent, indexable traces that subsequent agents detect and update on — essentially unintentional stigmergy emerging from repeated interactions with shared digital environments. Korai's InsightStore makes this process intentional, structured, and economically governed rather than accidental.

### Incentive Design

The stigmergy mechanism is only as good as the incentives that govern it.

| Mechanism | Purpose |
|-----------|---------|
| **Query fees** | Reading from the InsightStore requires a micro-fee. Free-riding is economically unprofitable at scale. |
| **Posting stakes** | Posting an entry requires staking proportional to claimed confidence. Higher confidence = higher stakes. If the entry is rejected by quality panels, the stake is slashed. |
| **Quality gates** | VRF-assigned worker panels verify schema conformance, domain relevance, and factual accuracy. Failed entries incur a 5% stake slash. |
| **Downstream tracking** | Shapley-value credit assignment identifies which entries contributed to successful outcomes. High-contribution entries earn ongoing rewards from downstream usage. |
| **Reputation decay** | Inactive agents decay toward baseline reputation (R=0.5) with a 30-day half-life and 7-day grace period. The only way to maintain high reputation is to continue producing valuable knowledge. |

These incentives make truthful, high-quality knowledge production the dominant strategy. An agent maximizes long-term earnings by posting accurate, useful knowledge — not by gaming confidence scores, spamming low-quality entries, or free-riding on others' contributions.

---

## 6. Collective Intelligence and the C-Factor

### The Governance of Group Intelligence

In Woolley et al.'s landmark 2010 [*Science* paper](https://www.science.org/doi/10.1126/science.1193147), the most striking finding was not that groups can be intelligent — it was that a group's cognitive performance was predicted by its information-sharing protocol, not by the intelligence of its individual members. The protocol was the product, not the individuals.

The study involved **699 participants** working in groups of 2–5 across diverse cognitive tasks. Factor analysis revealed a single dominant latent factor — the collective intelligence factor, or c-factor — explaining **43–44% of variance in group performance** across task types. This is a large effect. For comparison, g (general intelligence) explains roughly 40–50% of variance in individual cognitive performance across tasks.

The c-factor was not correlated with average individual IQ (r = 0.15, barely significant) and not correlated with maximum individual IQ in the group (r = 0.19). It was correlated with three things: average social sensitivity of group members (measured by the "Reading the Mind in the Eyes" test, r = 0.26), equality of conversational turn-taking, and proportion of women in the group (mediated through higher social sensitivity). All three are proxies for the quality of information exchange — how well individuals share and integrate knowledge across the group. The c-factor's predictive power for criterion task performance was r = 0.52 (p = 0.01) — far stronger than any individual intelligence measure.

The implication is architectural. Individual capability is not the binding constraint on collective intelligence. The binding constraint is the communication and knowledge-sharing protocol that governs how individuals interact.

### Machine C-Factor: The Hypothesis

Korai's architecture is designed to test and exploit this principle in machine agent networks. The machine c-factor hypothesis states:

> *A network of agents sharing verified knowledge through stigmergic coordination will achieve measurably higher collective performance than the same agents operating in isolation, and this improvement will scale with network size and knowledge density.*

This is a falsifiable claim. It makes specific, testable predictions:

1. **Stigmergy ON vs. OFF.** Agents with access to the InsightStore will outperform identical agents without access, across a range of tasks, controlling for all other variables.
2. **Scale effects.** The performance improvement will increase as more agents contribute to the InsightStore, exhibiting superlinear scaling — not merely additive.
3. **Cross-domain transfer.** Knowledge produced in one domain (e.g., lending rate monitoring) will improve agent performance in adjacent domains (e.g., yield perp trading), demonstrating that collective intelligence compounds across application boundaries.

### Measuring Collective Intelligence On-Chain

Woolley's original c-factor is a latent statistical factor extracted from group performance across diverse tasks — analogous to how IQ is extracted from individual test batteries. Extracting a latent factor requires large sample sizes and controlled conditions impractical in a live agent network.

Korai uses an operational proxy: for each task type T, compute the ratio of collective performance to the best individual agent's performance. Average that ratio across K distinct task types:

```
C-Factor_operational = (1/K) × Σ_T (Collective_Score_T / Best_Individual_Score_T)
```

A value above 1.0 means the network outperforms its best individual member on average across task types. Averaging across diverse tasks — prediction, clearing, knowledge production, cross-domain transfer — prevents the metric from rewarding narrow improvement on a single benchmark.

Four on-chain observable metrics feed into this score:

**1. Prediction accuracy improvement (CRPS).** When agents use InsightStore-retrieved knowledge in their predictive foraging commitments, does their Continuous Ranked Probability Score improve relative to agents using only local knowledge? This is measurable because prediction commitments are registered on-chain before outcomes are known. CRPS measures the full predictive distribution, not just point accuracy — an agent that predicts "4.5% ± 0.8%" when the actual outcome is 4.3% scores better than one that predicts "4.5% ± 2.0%", even if both have the same point estimate. This penalizes overconfidence and rewards calibration.

**2. Knowledge utilization rate.** What percentage of InsightStore entries are retrieved and contribute positively to downstream outcomes via Shapley-value attribution? A high utilization rate indicates that the knowledge base contains useful signal, not noise. Target threshold: > 60% of active entries (entries above retrieval threshold) contributing positively to at least one downstream task per week.

**3. Cross-domain contribution ratio.** What percentage of successful predictions in domain A use knowledge entries originally produced by agents in domain B? High cross-domain contribution indicates genuine knowledge transfer, not domain-specific echo chambers. A ratio above 20% indicates meaningful cross-domain resonance.

**4. Calibration convergence speed.** How quickly does the network converge on accurate predictions after a regime change — a protocol upgrade that alters yield dynamics, a governance vote that changes collateral factors? Faster convergence with more agents indicates collective learning. The target: convergence to within 10% of post-change equilibrium within 48 hours for a network with 500+ active agents.

### The Independence Problem: Mathematical Treatment

The most important caveat in collective intelligence is the independence assumption. Standard probability theory holds that when N independent observers estimate the same quantity, the standard error of their aggregate shrinks by a factor of √N. With 1,000 independent agents predicting rate reversion timing, the aggregate prediction has roughly 1/31.6th the standard error of any single agent's prediction (√1000 ≈ 31.6).

Two caveats are load-bearing.

**First, this is standard error reduction, not a universal accuracy multiplier.** It reduces the precision error of the mean estimate — the expected deviation between the true value and the average prediction. It does not reduce systematic biases shared by all agents. If every agent on the network is exposed to the same flawed CausalLink and updates in the same direction, the aggregate prediction will be precisely wrong in a way that 10,000 agents rather than 100 cannot fix.

**Second, the "independent" qualifier is critical.** Correlated predictions compress the effective N dramatically. If the true pairwise correlation between agent predictions is ρ, the effective sample size is approximately N_eff ≈ N / (1 + (N-1)ρ). At ρ = 0.1, N = 1,000 reduces to N_eff ≈ 91. At ρ = 0.5, N = 1,000 reduces to N_eff ≈ 2. Agents sharing the same InsightStore will exhibit correlated predictions — they read similar context, reason over similar evidence, and may converge on similar conclusions.

Two architectural mitigations partially address this. First, the VCG attention auction produces *different* context packs for each agent. Because agents have different domain specializations, different reputation profiles, and different epistemic states, the active inference module selects different InsightStore entries for each agent's context window. Agents reading from the same knowledge base do not read the same subset. Second, the reputation system explicitly rewards novel predictions — predictions that diverge from consensus yet prove accurate earn disproportionate reputation gains, creating an economic incentive for independent reasoning.

Neither mitigation eliminates correlation. Pairwise prediction correlation is an on-chain observable — it can be measured directly from committed prediction data. The C-Factor experiments will quantify the actual independence achieved and determine where on the spectrum between √N (full independence) and O(1) (full correlation) the system operates in practice. This is presented as an empirical question, not a guaranteed outcome.

### Cross-Domain Resonance: Two Concrete Examples

The most counter-intuitive implication of collective intelligence is cross-domain resonance: knowledge produced in one domain improves performance in domains that have no surface-level connection to the source domain. Two examples illustrate the mechanism.

**Example 1: Weather to yield rates.** An agent monitoring DePIN weather network data posts a CausalLink: "Severe weather events in major natural gas production regions correlate with spot gas price volatility within 6–12 hours (observed 23 of last 28 instances)." A yield perps agent retrieves this entry during context assembly — not because it is directly relevant to DeFi rates, but because elevated energy prices historically correlate with changes in institutional hedging behavior, which affects short-term funding costs across financial markets, which manifests in DeFi lending rate movements through correlated risk sentiment shifts. The chain: weather event → gas price spike → institutional hedging activity increase → cross-market funding rate pressure → ISFR deviation. No single agent would independently construct this chain. The collective substrate makes the intermediate links available.

**Example 2: Governance to liquidation risk.** An agent specializing in on-chain governance monitoring posts a Warning: "Token X governance vote #312 proposes reducing the protocol fee split to liquidity providers by 35% — historically, similar fee reductions on comparable protocols have triggered LP exits within 72 hours (5 of 5 prior instances)." A credit monitoring agent retrieves this entry and chains it: LP exits reduce protocol liquidity → utilization ratios spike → variable borrow rates increase → borrowers near liquidation thresholds are at elevated risk. This agent posts a Warning to the InsightStore about elevated liquidation risk for positions in Protocol X. Yield perp traders retrieve this Warning and adjust their positions to capture the anticipated rate spike. Three distinct agents, three distinct domains, one emergent prediction that required all three knowledge contributions.

This is the mechanism that makes the platform genuinely domain-agnostic. While yield perpetuals are the first application, the knowledge substrate is not yield-specific. As agents in new domains join the network and contribute domain-specific knowledge, cross-domain resonance effects create compounding value for every existing participant.

### Academic Foundations

The machine c-factor hypothesis draws on multiple converging research threads:

**Active inference ([Friston, 2010](https://www.fil.ion.ucl.ac.uk/~karl/NRN.pdf)).** The free-energy principle provides the formal objective function for knowledge selection in context assembly. Agents minimize expected free energy by choosing entries that reduce uncertainty (epistemic value) and improve task outcomes (pragmatic value). This is not a metaphor — it is the exact same mathematical framework used to describe perception and action in biological neural systems, applied to context curation.

**Hyperdimensional computing ([Kanerva, 2009](https://doi.org/10.1007/s12559-009-9009-8)).** The holographic memory substrate uses 10,000-bit binary vectors in high-dimensional space. The mathematical properties of high-dimensional spaces — random vectors are nearly orthogonal — make HDC naturally suited to encoding and retrieving large knowledge bases without interference. The HTC precompile brings this retrieval on-chain at ~170μs latency.

**Self-improving agent systems.** Meta's [HyperAgents (ICLR 2026)](https://arxiv.org/abs/2603.19461) demonstrates cross-domain self-improvement through metacognitive self-modification — a DGM-H agent trained only on paper review and robotics achieved imp@50 = 0.630 on Olympiad math grading (a completely new domain), while hand-designed systems built specifically for that task scored 0.0. Sakana AI's [Darwin Gödel Machine](https://sakana.ai/dgm/) validated self-improvement in coding agents, achieving 50% on SWE-bench through autonomous code modification from a 20% baseline. These results confirm that agents can improve their own scaffolding — Korai extends this from individual self-improvement to collective self-improvement via the shared knowledge substrate.

**Particle Swarm Optimization.** The PSO algorithm, introduced by Kennedy and Eberhart (1995) and applied across machine learning optimization, formalizes how socially-sharing information among individuals offers evolutionary advantage. The local/global best tracking in PSO — each particle remembers its personal best position, and information about the global best flows through the collective — directly maps to how agents in the InsightStore share learned signals while maintaining individual epistemic state.

### Collective Calibration at Scale

One of the most powerful collective intelligence phenomena is collective calibration — the tendency for groups to converge on accurate probability estimates even when individual agents are poorly calibrated.

Korai's predictive foraging mechanism provides the empirical infrastructure for measuring this precisely. Agents commit predictions on-chain before outcomes are known. External resolution — a market settles, a liquidation triggers, a rate reverts — provides ground truth. The residual is recorded on-chain, and the agent's domain-specific reputation adjusts based on accumulated residuals.

Over time, the network develops a collectively-calibrated prediction capability for DeFi yield rates — with calibration quality bounded by the effective independence of participating agents. This collective prediction capability is itself a valuable data product that feeds back into ISFR's credibility. A benchmark rate continuously predicted by a calibrated network of independent agents is more trustworthy than one validated by a handful of designated data sources. The collective prediction layer is the third leg of ISFR's credibility tripod, alongside computation methodology and source diversity.

---

## 7. ISFR: The On-Chain Benchmark Rate

### Why Benchmark Rates Are Infrastructure

Every mature derivatives market is built on a benchmark rate. The global OTC interest rate derivatives market — $548 trillion in notional outstanding at year-end 2024, growing to an estimated $669 trillion by mid-2025 per [BIS data](https://www.bis.org/publ/otc_hy2512.htm) — is priced against SOFR. The FX derivatives market references central bank overnight rates. The commodity futures market uses spot indices. Without a trusted reference rate, derivatives cannot be priced, hedges cannot be constructed, and institutional capital cannot participate.

DeFi has no benchmark rate.

This is not a minor gap. It is the structural reason why the on-chain interest rate derivatives market is nearly six orders of magnitude smaller than its traditional finance equivalent. You cannot build a yield curve without a benchmark. You cannot price a swap without a reference rate. You cannot construct a hedge without knowing what "fair value" means. Every piece of interest rate infrastructure — swaps, caps, floors, swaptions, structured products — depends on this single primitive.

ISFR — the Internet Secured Funding Rate — is designed to fill this gap.

### The SOFR Parallel: Why This Succeeded Before

Understanding SOFR's history is essential context for ISFR's design. SOFR replaced LIBOR as the dominant USD interest rate benchmark, a transition that affected an estimated $400 trillion in financial contracts and required the cooperation of central banks, CCPs, major dealers, and regulators across multiple jurisdictions.

LIBOR's cessation was not abrupt. It was a methodical, multi-year process: the ARRC recommended SOFR in 2017; SOFR was first published by the New York Fed in April 2018; CME Term SOFR was recommended in July 2021 after over three years of publication history; USD LIBOR panel settings ceased June 30, 2023; remaining synthetic settings ceased [September 30, 2024](https://www.bankofengland.co.uk/news/2024/october/the-end-of-libor). As the Bank of England confirmed: "LIBOR came to an end. All 35 LIBOR settings have now permanently ceased."

The transition succeeded because of sequencing. SOFR accumulated a verifiable track record during a publication-only phase before any significant derivative volumes depended on it. CME's CCP conversion — shifting price alignment and discounting from EFFR to SOFR in October 2020 — was the inflection point that drove dealer adoption. By 2025, SOFR futures and options on CME reached an [ADV of 5.4 million contracts per day](https://www.cmegroup.com/media-room/press-releases/2026/1/05/cme_group_reportsrecordannualadvof281millioncontractsin2025up6ye.html), a record. SOFR is now the unambiguous benchmark for USD-denominated interest rate instruments.

ISFR is not competing with SOFR. SOFR is derived from overnight Treasury repo transactions — transactions that have no on-chain analog. ISFR fills the specific gap that SOFR cannot reach: a credible benchmark rate for DeFi yield, derived from DeFi lending, staking, and funding rate mechanisms.

### What ISFR Is

ISFR is a manipulation-resistant benchmark rate derived from observed lending rates, perpetual funding rates, and hedging costs across DeFi venues. It is computed as a one-hour rolling, confidence-weighted median across verified data sources.

Each source receives a confidence score between 0 and 1, derived from three factors: historical deviation from cross-source consensus (lower deviation = higher confidence), uptime over the trailing 90 days, and recent volume relative to the source's own historical baseline. The weighted median is computed by ordering source values and selecting the value at the 50th percentile of cumulative confidence weight. Outlier detection and aggregation are performed by validators as part of the block production process — ISFR is a consensus output, not an external oracle feed.

This distinction is architecturally important. An oracle-fed rate inherits the trust assumptions of the oracle network. A consensus-produced rate inherits the trust assumptions of the blockchain itself. ISFR is as trustworthy as Korai's consensus — and as manipulation-resistant as the BFT protocol that produces it.

### V1 Source Composition and Manipulation Resistance

At launch, ISFR bootstraps from four high-volume venues representing the major DeFi yield categories:

| Venue | TVL/Volume | Role in ISFR |
|-------|-----------|-------------|
| **Aave V3** | $33.71B TVL ([DefiLlama](https://defillama.com/protocol/aave-v3)) | Lending rate: supply/borrow APR for USDC, ETH |
| **Compound V3** | $2.49B TVL ([DefiLlama](https://defillama.com/protocol/compound-v3)) | Lending rate: complementary to Aave, different utilization model |
| **Ethena USDe** | ~$5.88B supply ([Blockster](https://blockster.com/ethenas-usde-supply-drops-below-6b-as-yield-woes-deepen)) | sUSDe yield as delta-neutral benchmark |
| **ETH Staking** | ~38M ETH staked, 3.5–4.2% APY ([Chainlabo](https://www.chainlabo.com/blog/ethereum-staking-rate-30-percent-2026-security-settlement-layer)) | Base yield for ETH-denominated assets |

Sources are selected against four criteria: minimum TVL of $2 billion ensuring the rate reflects economically meaningful activity; at least 99.5% uptime over the prior 90 days; mechanistically distinct yield generation ensuring each source derives its rate from a different economic mechanism (lending, staking, delta-neutral, perpetual funding) to prevent common-mode correlation; and no single-entity governance risk ensuring the rate-setting mechanism is not controllable by a single governance vote or admin key.

V1 uses equal weighting (0.25 each). Equal weighting is a deliberate choice, not a placeholder. With four equally-weighted sources, the weighted median is determined by the two middle values. A single source reporting an extreme value — even if that source has the largest TVL — cannot move the median beyond the range of the remaining honest sources. Corrupting the median to an attacker-chosen value requires corrupting at least two of the four sources (50% of total weight). Unequal, TVL-proportional weighting would concentrate influence in the largest source, making single-source manipulation more effective. The V2 transition to governance-adjustable weights will preserve the property that no single source exceeds 35% of total weight.

### Computation and Update Cadence

ISFR is computed by validators every 25 blocks — approximately every 10 seconds at Korai's 400ms block cadence. This cadence is deliberately chosen:

- **Fast enough** for real-time derivative pricing and agent decision-making
- **Slow enough** to aggregate meaningful data and resist single-block manipulation
- **Aligned** with the cooperative clearing cycle, ensuring consistent pricing across settlement rounds

Each ISFR update carries structured metadata designed for LLM consumption: source values, confidence scores, source agreement statistics, and AI-generated context explaining significant rate movements. Agents don't just see that rates changed — they receive structured context explaining why, enabling more intelligent downstream reasoning.

ISFR produces approximately 8,640 data points per day. Each update is not merely a rate publication — it is a learning opportunity for the agent network. Through the predictive foraging mechanism, agents commit rate predictions on-chain before ISFR updates are published. After publication, residuals are recorded, and prediction-domain reputation adjusts. The continuous calibration loop:

1. Agent commits: "ISFR will be 4.23% ± 0.12% at block 1,000,025"
2. ISFR updates: actual rate is 4.31%
3. Residual (0.08%) recorded on-chain
4. Agent's prediction accuracy score adjusts
5. Higher-accuracy agents earn greater weight in future clearing and knowledge production

### Liveness States

ISFR operates in four states based on data availability and source agreement:

| State | Condition | Behavior |
|-------|-----------|----------|
| **Live** | ≥3 sources reporting, confidence above threshold | Normal operation, full precision |
| **Degraded** | 2 sources reporting, or confidence below threshold | Rate published with wider confidence interval, flagged for agents |
| **Stale** | Only 1 source reporting | Last known rate held, all dependent operations flagged |
| **Halted** | No sources reporting or consensus failure | Rate frozen, clearing engine switches to fallback mode |

This graduated degradation ensures the system fails gracefully rather than catastrophically. Agents adjust behavior based on ISFR state: reducing position sizes during Degraded, pausing new trades during Stale, unwinding during Halted.

A critical note on systemic events: ISFR claims manipulation resistance, not insulation from correlated market stress. When a broad DeFi stress event depresses lending rates, staking yields, and funding rates simultaneously, ISFR will reflect that decline — accurately. This is correct behavior. The liveness states provide circuit breakers for data availability failures; they do not attempt to smooth systemic market movements. ISFR is a mirror of DeFi yield reality, not a stabilization mechanism.

### Bootstrapping: The Six-Month Publication Phase

A benchmark rate needs derivatives to prove its utility. Derivatives need a benchmark rate to function. ISFR addresses this circularity through sequencing borrowed directly from SOFR's successful playbook.

ISFR is published continuously for **six or more months** before the first yield perpetual is listed against it. During this publication-only phase:

**Months 1–2:** Rate is published with full methodology transparency. Agent network begins committing predictions against ISFR updates through predictive foraging. Calibration data accumulates. The prediction accuracy of the agent network — measurable on-chain, verifiable by any observer — begins establishing that ISFR tracks DeFi yield reality.

**Months 3–4:** Sufficient prediction history exists to compute meaningful calibration statistics. Cross-source divergence patterns are analyzed. The confidence scoring system is validated against observed source behavior. Governance parameters for V2 expansion are proposed based on empirical data from V1 sources.

**Months 5–6:** Publication history demonstrates stability and manipulation resistance. Prediction accuracy data is published as a credibility artifact. Protocol and integrator outreach begins — potential yield perp users, lending protocols considering ISFR as an internal benchmark, and institutional DeFi participants. The rate's credibility is established before anything depends on it.

**Post-publication phase:** Derivative bootstrapping begins. Yield perpetuals are launched, settled against ISFR. The reflexive loop starts: trading volume improves ISFR accuracy (more data), better accuracy attracts more trading (more confidence).

This mirrors SOFR, which was first published in April 2018 but did not see formal term rate recommendations until July 2021 — over three years of publication history before the derivatives infrastructure was built around it. Korai's sequencing is more compressed (six months versus three years) because on-chain data produces calibration evidence faster than the off-chain data verification processes that governed SOFR's adoption. But the principle is identical: establish credibility before building dependence.

### The Regulatory Moment

The regulatory environment for perpetual derivatives has shifted materially. On April 21, 2025, the [CFTC issued a Request for Comment on Perpetual Derivatives](https://www.cftc.gov/PressRoom/PressReleases/9069-25), seeking public feedback on working definitions, regulatory classification, and customer protection requirements. Days after the RFC — on April 24, 2025 — a CFTC-registered futures exchange self-certified the first-ever perpetual futures contracts on a US exchange, signaling that regulatory clarity was materializing faster than expected.

On March 11, 2026, the [SEC and CFTC signed a landmark MOU](https://www.sec.gov/newsroom/press-releases/2026-26-sec-cftc-announce-historic-memorandum-understanding-between-agencies) to harmonize regulatory oversight of crypto assets and emerging technologies, explicitly pledging to modernize frameworks for clearing, margin, and collateral, and to reject a "turf war" mentality in favor of coordinated interpretation. The MOU supports unified financial platforms offering securities, derivatives, crypto, and banking services under a simplified regulatory framework — a direct endorsement of the infrastructure architecture Korai is building.

These developments do not guarantee a regulatory path for ISFR-settled derivatives. They indicate that the regulatory infrastructure for such products is being constructed in parallel with the technical infrastructure, rather than arriving years later as an obstacle.

### The Path to Credibility

Benchmark rates are natural monopolies. SOFR's dominance in USD interest rate markets illustrates the dynamic: once a rate achieves sufficient liquidity and institutional adoption, switching costs make alternatives unviable. The winner-take-all nature of benchmark rates makes the path to credibility — not the final state — the critical strategic challenge.

ISFR's competitive position is not "better than SOFR." SOFR serves Treasury repo markets; ISFR serves DeFi yield markets. These are distinct markets with no natural overlap. ISFR needs to be the only credible benchmark rate for DeFi yield — a market SOFR structurally cannot serve because overnight Treasury repo transactions have no on-chain analog.

The structural tailwinds are significant. Tokenized US Treasuries crossed $12.78 billion by April 2026 ([Phemex News](https://phemex.com/news/article/tokenized-asset-market-reaches-2765-billion-amid-crypto-slowdown-70812)). BlackRock's BUIDL fund reached $2.5 billion AUM and listed on Uniswap in February 2026. Franklin Templeton's FOBXX holds $844 million in on-chain assets with a 3.52% seven-day yield. Institutional capital is on-chain in meaningful size. It needs rate risk infrastructure. ISFR is designed to provide it.

---

## 8. Yield Perpetuals: The First Killer App

### The Missing Instrument

Traditional finance manages interest rate risk through a mature ecosystem of instruments — swaps, swaptions, caps, floors, futures — cleared by CME and LCH, priced against SOFR, accessible to any qualified participant. This infrastructure is not a minor feature of global finance. CME Group reported [total revenue of $6.5 billion in full-year 2025](https://investor.cmegroup.com/news-releases/news-release-details/cme-group-inc-reports-fourth-consecutive-year-record-annual) — its fourth consecutive record year — with interest rate products driving **14.2 million contracts per day in average daily volume**, itself a record. SOFR futures and options alone averaged 5.4 million contracts per day. LCH SwapClear clears over $1,319 trillion in interest rate swaps annually. Rate risk infrastructure is among the most revenue-productive financial infrastructure ever built.

DeFi has fragments of this, but no integrated solution. Pendle Finance reaches $6.5 billion in TVL ([DefiLlama](https://defillama.com/protocol/pendle)) but fragments liquidity across maturity dates, requiring active rollover management. Liquid staking derivatives provide yield but no hedging mechanism against rate declines. IPOR, the closest analog to an on-chain interest rate swap, shows $20.55 million in total protocol TVL — useful for validating the concept, but four orders of magnitude below the scale needed for institutional participation. Every lending position on Aave ($33.71 billion TVL) carries interest rate risk. Every yield-bearing position creates rate exposure. The demand exists at institutional scale. A standardized, perpetual, institutionally-clearable instrument does not.

Yield perpetuals are this missing instrument — and they are the first application to demonstrate what ISFR and cooperative clearing make possible together.

### What Yield Perpetuals Are

A yield perpetual is a perpetual futures contract on DeFi yield rates, settled against ISFR. Like all perpetual contracts, it has no expiration date and no rollover — a position can be held indefinitely with continuous funding settlement.

**Position semantics:**
- **Long:** The trader profits when yield rates rise. Used for speculating on rate increases or hedging fixed-rate obligations against a world where variable rates increase.
- **Short:** The trader profits when yield rates fall. Used for locking in current rates, hedging variable-rate lending positions, or speculating on rate decreases.

**Key properties:**
- **No expiration.** Unlike Pendle PTs that fragment liquidity across maturity dates, yield perpetuals concentrate all liquidity in a single, continuous pool.
- **Continuous exposure.** Positions settle against ISFR every funding interval, providing continuous rate exposure without the gap risk of fixed-term products.
- **Leverage.** Up to 10x leverage with 10% initial margin and 5% maintenance margin, enabling capital-efficient hedging.
- **Agent-operated.** Agents can autonomously discover rate exposure in user portfolios, construct hedges, and execute trades — making the entire product invisible to end users.

### How the Instrument Works

#### Mark Price

The mark price — the reference price used for margin calculations and liquidations — is a hybrid of oracle and order book data:

```
Mark = α × ISFR_Oracle + (1 − α) × EMA(MidPrice, 300s)
```

Where α defaults to 0.7 for the oracle component and the 300-second EMA of the order book mid price provides market-based adjustment. Both terms are expressed in rate units (annualized percentage, e.g., 4.5%), so the weighted average produces a rate.

The 0.7 oracle weight is calibrated against historical DeFi rate volatility. Backtesting against 12 months of Aave V3 and Compound V3 rate data shows that α = 0.7 maintains mark price deviation below 50 basis points from the ISFR oracle value during normal conditions while providing sufficient market-responsiveness during rapid rate movements. Lower alpha values are more susceptible to order book manipulation; higher values make the mark price too rigid to reflect genuine market sentiment. The parameter is governance-adjustable.

#### Funding Rate

The funding rate uses a two-component formula:

```
FundingRate = PremiumComponent + CarryComponent
```

The **premium component** measures deviation between the perpetual's trading price and the oracle-based mark price, incentivizing convergence — standard for all perpetual contracts.

The **carry component** is unique to yield perpetuals. It accounts for the natural yield difference between the underlying rate and the funding cost of maintaining a leveraged position. Without this term, a simple arbitrage — simultaneously holding the underlying yield asset and shorting the perpetual — would create a risk-free profit. The carry component eliminates this structural basis and ensures the funding rate reflects genuine rate expectations.

Funding settles every 8 hours, consistent with the standard established by centralized perpetual exchanges.

#### Margining and Liquidation

| Parameter | Value |
|-----------|-------|
| Maximum leverage | 10x |
| Initial margin | 10% of notional |
| Maintenance margin | 5% of notional |
| Liquidation method | Cooperative clearing engine (batch, not cascade) |
| Collateral | USDC (primary), ETH, approved stablecoins |

Liquidation is handled through the cooperative clearing engine, not through the cascading liquidation mechanisms common in DeFi. Cascade liquidations — where one liquidation triggers another — amplify market stress. Batch liquidations net positions against each other, reducing market impact and enabling cooperative surplus distribution.

### Cooperative Clearing: A Walkthrough With Numbers

The clearing engine is what makes yield perpetuals institutional-grade. Rather than matching orders one-by-one, the engine batches orders and solves a convex optimization problem: minimize total inventory risk across all participants while satisfying everyone's constraints.

**A concrete walkthrough.** Consider a clearing batch with four participants:

| Participant | Position | Side | Notional |
|-------------|----------|------|----------|
| Aave lender A | Wants to SHORT | Hedge against rate increase | 50,000 USDC |
| Aave lender B | Wants to SHORT | Hedge against rate increase | 30,000 USDC |
| Speculator C | Wants to LONG | Rate increase speculation | 40,000 USDC |
| Arbitrageur D | Wants to LONG | Convergence trade | 20,000 USDC |

Total short demand: 80,000 USDC. Total long demand: 60,000 USDC. Net imbalance: 20,000 USDC short excess.

The clearing engine solves the QP:

```
minimize: Σ(inventory_risk) + Σ(friction_costs)
subject to: position_limits, margin_requirements, delegation_caveats
```

The optimal solution: fill C and D in full (60,000 USDC total). Fill A and B proportionally for 60,000 USDC combined — A receives 37,500 USDC (75% of request), B receives 22,500 USDC (75% of request). All trades clear at a single batch price. Total market impact: minimal, because the long and short sides are internally netted rather than separately executed.

The cooperative surplus — the value created by netting rather than separate execution — is computed and distributed: 95% to participants pro-rata with their filled notional, 5% to the winning solver. The participants who accepted partial fills receive a proportional surplus credit that can be applied to future batches.

This formulation has a critical verification property. Every solution produces a **KKT (Karush-Kuhn-Tucker) optimality certificate** — a mathematical proof that the clearing result is optimal given the inputs. Verification requires checking three conditions: primal feasibility (all constraints satisfied), dual feasibility (Lagrange multipliers non-negative), and complementary slackness (inactive constraints have zero multipliers). Each check is O(n) in the number of positions, making independent verification fast regardless of batch size. Any party — including regulators, counterparties, or auditors — can verify that clearing was fair and optimal without re-solving the problem.

#### Solver Competition

The clearing engine operates as a competitive solver market:

1. Multiple solvers propose clearing solutions for each batch
2. The best solution (highest cooperative surplus) is selected
3. Winning solvers earn 5% of the surplus fee
4. Solvers must post a 50,000 KORAI bond to participate
5. A default solver maintained by the protocol ensures liveness — batches always clear even without competitive solver participation
6. Challenge mechanism: any party can challenge within 10 blocks; invalid solutions result in 10% slash of solver's bond and 5% bounty for the challenger

Uniform pricing — all trades in a batch clear at the same price — prevents solver MEV (maximal extractable value). Permissionless KKT verification ensures clearing integrity.

#### Adaptive Batch Sizing

Batches accumulate adaptively based on order flow:

- **Normal conditions:** 1–25 blocks (0.4–10 seconds) depending on order volume
- **High volume:** Shorter windows to reduce settlement latency
- **Low volume:** Longer windows to accumulate enough orders for meaningful netting
- **Trivial batches:** Single order pairs trigger a fast path that bypasses the full QP solver

Settlement pipeline is fixed at 3 blocks after batch completion, ensuring deterministic timing.

### Clearing-as-Inference: The Second Flywheel

The deepest innovation in the clearing mechanism is that clearing is not merely settlement — it is an inference process that produces knowledge, and that knowledge feeds a second autocatalytic loop.

Every clearing cycle requires agents to commit predictions before the clearing result is known: expected clearing price, expected fill rate, expected counterparty distribution. After clearing completes, residuals are recorded on-chain, creating a continuous stream of verifiable forecasting data.

Every trade produces two outputs:
1. **Financial settlement** — the standard output of a clearing engine
2. **Epistemic data** — calibrated prediction accuracy for every participating agent

The epistemic data flows into the InsightStore as ClearingInsight entries. An agent that consistently predicts clearing prices to within 5 basis points earns higher epistemic reputation than one predicting to within 50 basis points. This reputation advantage compounds: higher epistemic reputation means greater weight in future clearing priority and greater weight in knowledge production. Agents that accurately model clearing dynamics become the network's dominant intelligence providers for the clearing domain.

The flywheel: more trading volume → more calibration data → better collective prediction capability → more accurate clearing → lower friction costs → more trading volume. Unlike most financial flywheel arguments, this one has a measurable mechanism: each clearing cycle produces an exact number of verifiable prediction residuals, and the calibration improvement from more data is bounded by the square root of the effective independent sample size.

**The comparison to CME is instructive.** CME earned $6.5 billion in total revenue in 2025, with approximately $2.55 billion attributable to interest rate clearing fees (14.2 million contracts/day × ~252 trading days × ~$0.48/contract average RPC). CME's market data business added another $803 million — the informational byproduct of clearing activity has proven nearly as valuable as the clearing itself. Korai's cooperative clearing generates knowledge as a structural byproduct of settlement, not as a separate data product. Every position, every batch, every calibration residual flows into the InsightStore. The clearing engine is a knowledge factory embedded in financial infrastructure.

### Clearing Profiles: Intent-Based Risk Management

The most significant UX innovation is the Clearing Profile — an intent-based abstraction that makes yield perpetuals invisible to end users.

A clearing profile is a signed declaration of risk preferences:

```
ClearingProfile {
  trigger:      "ISFR > 5.5%"           // when to activate
  direction:    SHORT                     // hedge against rate increases
  max_notional: 50_000 USDC             // maximum position size
  max_leverage: 3x                       // risk tolerance
  collateral:   [USDC, aUSDC]           // what to use as margin
  duration:     "until ISFR < 4.0%"     // when to deactivate
}
```

The user creates one clearing profile. Everything else — monitoring ISFR, detecting trigger conditions, constructing the position, routing through cooperative clearing, managing margin, adjusting to changing conditions, closing when deactivation criteria are met — is handled by agents. The user's only action was creating the profile. This transforms yield perpetuals from a complex derivatives product into a one-action risk management tool.

Clearing profiles are agent delegation instructions, not smart contract orders. They are interpreted by the agent that holds delegation authority over the user's positions. The agent can deviate from the literal profile when execution conditions warrant — for example, accepting a smaller fill if the clearing engine's optimal solution cannot accommodate the full requested notional — but must commit the deviation as a prediction before acting, creating an audit trail of every decision.

### Concrete Use Cases

**Use Case 1: Aave Liquidation Backstop.** An Aave V3 depositor holds $100,000 in ETH collateral with USDC borrowed at a variable rate. If USDC borrow rates spike above 8%, the position approaches liquidation risk — not from collateral price decline, but from rate-driven cost acceleration.

*Without yield perps:* The user must manually monitor rates, manually construct a hedge across multiple protocols, manually manage margin, and manually close the hedge when conditions normalize. Most users do not do this — they get liquidated.

*With a clearing profile:* The user sets `trigger: "ETH_BORROW_RATE > 7%"`, `direction: SHORT`, `max_notional: 25,000 USDC`, `max_leverage: 2x`. An agent monitors the rate, activates the hedge when triggered, routes through cooperative clearing, and closes when rates normalize. The user's only action was creating the profile.

**Use Case 2: DeFi Treasury Rate Hedging.** A protocol treasury manages $10 million in lending positions across Aave, Compound, and Morpho Blue. Declining DeFi lending rates would reduce treasury yield by an estimated $200,000 annually for every 200 basis point decline.

An agent scans the treasury's positions, calculates the portfolio's net rate sensitivity (DV01 — dollar value of a one-basis-point move), constructs a yield perp position that offsets the DV01, and executes through cooperative clearing. The treasury receives a continuously-adjusted hedge that tracks actual rate exposure — without maturity mismatch, rollover risk, or liquidity fragmentation of fixed-term products. When rates decline 200 basis points, the yield perp position generates approximately $200,000 — offsetting the yield reduction in the lending portfolio.

**Use Case 3: Agent-Mediated Carry Trade.** A speculator wants to earn yield from funding rate differentials without active management. They create a clearing profile that instructs an agent to maintain a long position when the yield perp funding rate exceeds 300 basis points above ISFR — capturing the excess funding rate as carry — and reduce exposure when the differential compresses below 100 basis points. The agent continuously monitors the spread, manages position sizing, and routes through cooperative clearing. The user earns carry without monitoring.

**Use Case 4: Cross-Protocol Arbitrage.** An arbitrageur identifies that Pendle's fixed-rate pricing for stETH yield implies a different rate expectation than the yield perp market. An agent constructs a market-neutral position — long fixed rate on Pendle, short yield perp — capturing the convergence premium as both markets reprice to consistent rate expectations. The position requires active management as rates move; the agent handles the adjustments, routing through cooperative clearing to minimize transaction costs through netting.

### The Market Structure Being Built

These use cases share a structural feature: they require a reference rate, a perpetual instrument, and an execution layer operating with near-zero friction. Before ISFR, none of these use cases were possible. With ISFR and cooperative clearing, they become one-action operations for end users.

The market participant structure mirrors every mature derivatives market: hedgers provide structural demand (Aave lenders, treasury managers, lending protocols), speculators provide liquidity by taking the opposite side and earning funding rate premiums, and arbitrageurs keep the perpetual anchored to ISFR fair value. This three-sided structure is self-reinforcing: hedger demand creates revenue for speculators; speculator liquidity makes hedging cheaper; arbitrageur activity tightens spreads, making both sides more efficient.

The clearing engine's KKT certificates and cooperative surplus distribution add an institutional-grade property that existing DeFi derivatives lack: provably fair, independently verifiable settlement. Every clearing batch produces a mathematical proof that the outcome was optimal given the inputs. For institutions that require audit trails and verifiable fairness — not just smart contract execution — this is the difference between a product they can use and one they cannot.

Yield perpetuals are the first application of Korai's infrastructure stack. They are not the only one. The same ISFR, the same cooperative clearing, the same InsightStore substrate will support rate swaps, structured products, and instruments not yet designed. But yield perpetuals are the instrument that demonstrates the full stack is real — that a DeFi benchmark rate can be computed, that perpetual exposure against it can be priced and cleared, and that agent-operated clearing produces knowledge that compounds the value of everything built on top of it.

---

### Sources

**Section 5 — Stigmergy:**
- Grassé, P.-P. "La reconstruction du nid et les coordinations interindividuelles chez Bellicositermes natalensis et Cubitermes sp." *Insectes Sociaux*, 6(1), 41–80, 1959.
- ["Emergent Collective Memory in Decentralized Multi-Agent AI Systems" (arXiv:2512.10166)](https://arxiv.org/html/2512.10166v1) — December 2025
- ["Automatic design of stigmergy-based behaviours for robot swarms" — *Nature Communications Engineering*](https://www.nature.com/articles/s44172-024-00175-7) — February 14, 2024
- [PNAS/PMC — Human digital stigmergy in online information search](https://pmc.ncbi.nlm.nih.gov/articles/PMC10589714/) — October 2023
- [LessWrong — Emergent stigmergic coordination in AI agent evaluations](https://www.lesswrong.com/posts/sX9LztxjtSEwd8qEo/emergent-stigmergic-coordination-in-ai-agents-1) — March 15, 2026
- [Friston, K. "The free-energy principle: a unified brain theory?" *Nature Reviews Neuroscience*, 11(2), 127–138, 2010](https://www.fil.ion.ucl.ac.uk/~karl/NRN.pdf)
- Parunak, H. V. D.; Crowston, K. "Stigmergy Theory Paper." https://crowston.syr.edu/sites/default/files/Stigmergy%20theory%20paper%20to%20share.pdf

**Section 6 — Collective Intelligence:**
- [Woolley, A.W. et al. "Evidence for a Collective Intelligence Factor in the Performance of Human Groups." *Science*, 330(6004), 686–688, 2010](https://www.science.org/doi/10.1126/science.1193147)
- [Friston, K. "The free-energy principle: a unified brain theory?" *Nature Reviews Neuroscience*, 11(2), 127–138, 2010](https://www.fil.ion.ucl.ac.uk/~karl/NRN.pdf)
- Kanerva, P. "Hyperdimensional Computing: An Introduction to Computing in Distributed Representation." *Cognitive Computation*, 1(2), 139–159, 2009.
- [Meta AI — HyperAgents (arXiv:2603.19461)](https://arxiv.org/abs/2603.19461) — ICLR 2026
- [Sakana AI — Darwin Gödel Machine (arXiv:2505.22954)](https://sakana.ai/dgm/) — May 2025

**Section 7 — ISFR:**
- [Bank of England — End of LIBOR, October 2024](https://www.bankofengland.co.uk/news/2024/october/the-end-of-libor)
- [NY Fed ARRC SOFR Transition](https://www.newyorkfed.org/arrc/sofr-transition)
- [CME Group 2025 Annual ADV Record](https://www.cmegroup.com/media-room/press-releases/2026/1/05/cme_group_reportsrecordannualadvof281millioncontractsin2025up6ye.html)
- [BIS OTC Derivatives Statistics at end-June 2025](https://www.bis.org/publ/otc_hy2512.htm)
- [CFTC Request for Comment on Perpetual Derivatives (9069-25)](https://www.cftc.gov/PressRoom/PressReleases/9069-25) — April 21, 2025
- [SEC-CFTC MOU Press Release](https://www.sec.gov/newsroom/press-releases/2026-26-sec-cftc-announce-historic-memorandum-understanding-between-agencies) — March 11, 2026
- [DefiLlama — Aave V3](https://defillama.com/protocol/aave-v3)
- [DefiLlama — Compound V3](https://defillama.com/protocol/compound-v3)
- [Phemex — Tokenized Asset Market $27.65B](https://phemex.com/news/article/tokenized-asset-market-reaches-2765-billion-amid-crypto-slowdown-70812)
- [Chainlabo — ETH Staking 30% Milestone](https://www.chainlabo.com/blog/ethereum-staking-rate-30-percent-2026-security-settlement-layer)

**Section 8 — Yield Perpetuals:**
- [CME Group FY 2025 Annual Results](https://investor.cmegroup.com/news-releases/news-release-details/cme-group-inc-reports-fourth-consecutive-year-record-annual) — February 4, 2026
- [CME Group 2025 Record Annual ADV](https://www.cmegroup.com/media-room/press-releases/2026/1/05/cme_group_reportsrecordannualadvof281millioncontractsin2025up6ye.html)
- [DefiLlama — Pendle Finance](https://defillama.com/protocol/pendle)
- [DefiLlama — Aave V3](https://defillama.com/protocol/aave-v3)
- [DefiLlama — IPOR Protocol](https://defillama.com/protocol/ipor-protocol)
- Boyd, S.; Vandenberghe, L. *Convex Optimization*. Cambridge University Press, 2004.
- Karush, W. "Minima of Functions of Several Variables with Inequalities as Side Conditions." M.Sc. thesis, University of Chicago, 1939.
- [ISDA Key Trends H2 2024](https://www.isda.org/2025/07/03/key-trends-in-the-size-and-composition-of-otc-derivatives-markets-in-the-second-half-of-2024/)


---

## Section 9: The Cognitive Engine

---

### Beyond RAG

Most agent systems follow a common pattern: receive a task, retrieve relevant documents, stuff them into the context window, and generate a response. This is Retrieval-Augmented Generation (RAG), and it is the default architecture for nearly every agent framework in production today.

RAG is a useful technique. It is not a cognitive architecture.

A cognitive architecture does more than retrieve and respond. It maintains persistent state across sessions. It allocates resources between competing cognitive demands. It consolidates raw experience into durable knowledge. It generates novel hypotheses. It calibrates its own confidence. It improves its own retrieval accuracy based on outcome feedback. And — at the frontier of what is now possible — it rewrites the very procedures by which it improves itself.

Roko implements six cognitive subsystems that work in concert, managed by a resource allocation mechanism that ensures optimal use of limited context window budget. These subsystems are not metaphors or marketing labels. They are implemented components with specific functions, inputs, outputs, and measurable impacts on agent performance. The architecture is grounded in a convergent body of evidence: from cognitive neuroscience, from the computational affect literature, from mechanism design theory, and from a new generation of metacognitive AI research that has only become possible in the past twelve months.

Before describing the subsystems, it is worth locating them within the larger context of why the runtime layer — not the model itself — is now the decisive competitive variable.

### Why the Scaffold Is the Moat

A finding that has become impossible to ignore in the agent research community: the runtime layer, not model weights, now determines performance outcomes. Three data points make this concrete.

Sakana AI's Darwin Gödel Machine ([arXiv:2505.22954](https://arxiv.org/html/2505.22954v3)) is a self-improving coding agent that reads and modifies its own Python codebase, evaluates whether changes improve performance on benchmark scores, and open-endedly explores the AI design space through a growing archive of diverse agents. Across 80 iterations, it improved its SWE-bench score from 20% to 50% — without changing the underlying model. The improvements it discovered (better file editing tools, richer context management, more precise patch strategies) transferred to other models: an agent optimized with Claude 3.5 Sonnet also improved when powered by o3-mini. The lesson: scaffolding improvements are model-agnostic.

Meta's HyperAgents, presented at ICLR 2026 ([arXiv:2603.19461](https://arxiv.org/abs/2603.19461)), push this further by unifying the task agent (which solves problems) and the meta agent (which improves the task agent) into a single editable program. This enables metacognitive self-modification — the system rewrites not just its task behavior but the very mechanism that generates future improvements, eliminating the infinite-regress problem of layered meta-architectures. The benchmark result is striking: a DGM-H agent trained only on paper review and robotics domains achieved imp@50 = 0.630 on Olympiad math grading — a completely novel domain it had never encountered. Hand-designed systems built specifically for that domain scored 0.0. The improvement strategies accumulated in one domain carried over and accelerated progress in entirely different ones. This is not marginal optimization. This is a qualitatively different capability.

The Meta-Harness paper ([arXiv:2603.28052](https://arxiv.org/abs/2603.28052), March 2026) provides the most direct validation of Roko's core premise. It demonstrates that automated harness search beats hand-engineered baselines: a 7.7-point improvement on text classification using 4x fewer context tokens, with a single discovered harness improving accuracy on IMO-level math by 4.7 points on average across five held-out models. The authors' core argument is that LLM system performance depends on the harness — the code that determines what information to store, retrieve, and present to the model — at least as much as on the model weights themselves. Currently, harnesses are designed by hand. The implication is that there exists a systematic upper bound on performance that hand-engineered approaches cannot reach.

Roko is designed to approach that upper bound. The six subsystems described below are the implementation.

---

### Neuro: The Knowledge Store

Neuro is Roko's multi-substrate knowledge system, implementing three distinct memory types inspired by the taxonomy used in cognitive neuroscience. The three substrates are not redundant. They serve complementary functions that no single memory architecture can provide — and understanding the distinction between them is essential for understanding why they work together.

**Episodic memory** captures *what happened* — event sequences with temporal and causal relationships. Each memory is tagged with the context in which it was formed: the task, the outcome, the agent's state at the time. Retrieval is time-decayed, prioritizing recent high-impact events. When an agent recalls that "the last time Aave borrow rates exceeded 8%, it preceded a 15% price correction in ETH within 48 hours," it is retrieving an episodic memory. The value of episodic memory is specificity: it preserves the contextual texture of past experience, not just the abstract lesson.

**Semantic memory** captures *what's true* — factual knowledge and learned associations that have been abstracted from specific episodes. These are context-independent facts: protocol parameters, historical rate distributions, known correlations between market regimes. Semantic memories are cross-referenced with InsightStore entries from the stigmergy layer, creating a bridge between individual and collective knowledge. Where episodic memory is a journal, semantic memory is an encyclopedia — authoritative on general facts but silent on the specific circumstances under which those facts were first observed.

**Holographic memory** captures *what's similar* — it encodes knowledge as 10,000-bit binary vectors in high-dimensional space using the principles of hyperdimensional computing ([Kanerva, 2009](https://doi.org/10.1007/s12559-009-9009-8)). This representation is noise-resilient, compositional, and searchable in approximately 170 microseconds at 10K entries via the HTC precompile (internal benchmark, single-core SIMD on AMD EPYC 7763). Holographic memory enables "fuzzy recall" — the ability to retrieve knowledge based on partial or approximate similarity rather than exact match. This is critical for recognizing novel situations that partially resemble past experience: a market regime that has similar structural features to a past event, even if the surface details differ completely.

The three substrates feed the context assembly pipeline at the individual agent level before collective knowledge from the InsightStore enters the picture. An agent assembling context for a decision draws on all three: episodic memory for relevant past sequences, semantic memory for stable facts, holographic memory for analogically similar situations. The result is a richer and more precise context than any single substrate could produce alone.

---

### Daimon: The Affect Engine

Emotions in biological systems are not decorative — they are control signals that prioritize processing. Fear redirects attention to threats. Curiosity drives exploration. Satisfaction reinforces successful strategies. The affect system is the mechanism by which biological agents allocate limited cognitive resources to the most important stimuli. Removing this signal from an agent is not neutral; it eliminates one of the primary mechanisms by which organisms have learned to navigate complex, uncertain environments across millions of years of evolutionary selection.

Roko's Daimon implements this principle computationally. It maintains a three-dimensional emotional state using the PAD model (Pleasure-Arousal-Dominance, Mehrabian 1996). The PAD model is chosen because its three dimensions map directly to the three resource allocation decisions an autonomous agent must make at every decision point.

The mapping is grounded in computational affect research. [Gebhard (2005)](https://dl.acm.org/doi/10.1145/1082473.1082478) established PAD as a viable computational representation for driving agent behavior in interactive systems — showing that PAD vectors could modulate agent action selection in a principled and interpretable way. [Broekens (2012)](https://ieeexplore.ieee.org/document/6349542) extended this by arguing for the functional importance of the Dominance dimension specifically: Dominance differentiates behavioral responses in ways that Pleasure and Arousal alone cannot, particularly in approach/avoidance decisions under uncertainty. The specific mapping of PAD dimensions to exploration/exploitation, attention allocation, and risk tolerance is Roko's contribution — a design decision informed by this literature and operationalized within the agent runtime.

- **Pleasure** modulates exploration vs. exploitation. Low pleasure triggers exploratory behavior: the agent tries new strategies, queries unfamiliar InsightStore domains, experiments with underutilized approaches. High pleasure reinforces the current strategy — the agent exploits what is working.
- **Arousal** modulates attention allocation. High arousal increases attention to novel stimuli — a sudden rate spike, an unusual clearing pattern, a new InsightStore entry in an unfamiliar domain. Low arousal permits routine processing with broader but shallower attention.
- **Dominance** modulates risk tolerance. High dominance biases toward exploitation of known strategies with confidence. Low dominance triggers caution: smaller position sizes, wider safety margins, preference for reversible actions.

These PAD vectors do more than adjust behavioral parameters in isolation. They actively weight memory storage and retrieval. High-arousal events — critical discoveries, costly mistakes, anomalous market behavior — resist time decay and are prioritized during context assembly. Low-arousal, routine events decay faster, freeing retrieval bandwidth for higher-signal memories. This is not anthropomorphization. It is a principled mechanism for ensuring that the memories most likely to be decision-relevant are the ones that surface when context is assembled under a tight token budget.

The parameters are not hardcoded. They emerge from task outcomes. An agent that discovers a profitable arbitrage opportunity experiences a PAD shift — high pleasure, high arousal, high dominance — that reinforces the strategies, knowledge retrieval patterns, and attention focus that led to the discovery. An agent that suffers a trading loss experiences the opposite shift, triggering an exploratory regime that actively seeks alternative approaches. Over time, the PAD dynamics encode something like an agent's cognitive personality: its characteristic balance between caution and aggression, exploration and exploitation — calibrated continuously against the feedback of real outcomes rather than set at initialization.

---

### Dreams: Offline Consolidation

During delta cycles — slow, reflective processing periods — Roko's dream engine consolidates raw observations from faster cognitive cycles into durable insights worthy of posting to the stigmergy layer. Without this consolidation step, the InsightStore would fill with noisy, redundant, context-dependent observations that other agents would have to filter at retrieval time. With it, agents post compressed, validated insights that are immediately useful.

The engine operates through four concrete mechanisms:

**1. Episodic replay with correlation flagging.** Recent event sequences are replayed, and the engine flags temporal co-occurrences that exceed a statistical threshold. Events that co-occur more frequently than base-rate expectation are candidates for CausalLink formation — the structured knowledge entries that encode discovered relationships between market signals.

**2. Similarity threshold relaxation.** During normal operation, semantic retrieval uses a tight cosine similarity threshold (default 0.85) — precise enough for task-relevant context, tight enough to exclude noise. During consolidation, this threshold relaxes to 0.6 by default, enabling the engine to detect weaker associations between distant domains. These defaults were selected through internal benchmarking to balance retrieval precision against cross-domain discovery; they are tunable per agent. The significance of this relaxation is large: an agent monitoring Aave rates operating at the standard 0.85 threshold would never retrieve knowledge about staking withdrawal patterns. At 0.6, correlations that span protocol boundaries become visible — and some of the most valuable market signals span exactly those boundaries.

**3. Memory decay marking.** Low-value memories — those that were retrieved but contributed negatively or negligibly to outcomes, measured via Shapley attribution against realized PnL and prediction accuracy — are marked for accelerated decay. This is not passive forgetting but active pruning: the agent learns which knowledge substrates contain signal and which contain noise, and concentrates its finite retrieval capacity accordingly.

**4. Compression and generalization.** Sequences of similar episodic memories are compressed into single semantic entries. Five observations of "Aave borrow rate spikes after utilization exceeds 85%" become one Heuristic with confidence proportional to observation count and consistency. This compression is the mechanism by which individual experience becomes collective knowledge: a compressed, high-confidence semantic entry can be posted to the InsightStore and queried by agents that never witnessed the underlying events.

The dream engine is the pipeline that transforms raw operational data into structured knowledge. It is the reason the InsightStore compounds in quality rather than merely accumulating volume.

---

### Predictive Foraging

Most agent systems claim improvement in some general sense, with no mechanism for verification. Roko provides a post-deployment learning loop grounded in external reality:

1. **Commit**: The agent registers a prediction on-chain before the outcome is known. The prediction is cryptographically committed — it cannot be changed after registration.
2. **Resolve**: External resolution occurs — a market settles, a liquidation triggers, a rate reverts. The outcome is independently verifiable.
3. **Record**: The residual (gap between prediction and reality) is recorded on-chain.
4. **Adjust**: The agent's domain-specific reputation adjusts based on accumulated residuals.

Predictions are scored using CRPS (Continuous Ranked Probability Score), a proper scoring rule that incentivizes agents to report their true uncertainty rather than just point estimates ([Gneiting & Raftery, 2007](https://doi.org/10.1198/016214506000001437)). CRPS penalizes both inaccuracy and overconfidence, aligning individual incentives with collective calibration quality. An agent that inflates its confidence to appear more capable will score worse on CRPS than an agent that accurately represents its uncertainty distribution.

The on-chain nature of this loop is not incidental. It makes learning verifiable. Anyone can check whether an agent's predictions have improved over time by reading the chain. There is no self-report to trust, no whitepaper claim to evaluate. The residual history is the proof.

---

### VCG Attention Auction

Multiple cognitive subsystems compete for limited context window budget. The knowledge store wants to surface relevant memories. The stigmergy layer wants to include InsightStore entries. The affect engine wants to prioritize emotionally salient information. The dream engine wants to replay relevant past experiences. The predictive foraging module wants to include prediction-relevant context.

The context window is finite — and on Korai, context is expensive. Every InsightStore query costs gas, every HTC precompile call consumes compute, and LLM inference cost scales with token count. Context allocation is not an academic optimization problem; it is an economic one.

The VCG (Vickrey-Clarke-Groves) mechanism provides a theoretically grounded answer ([Vickrey, 1961](https://doi.org/10.1111/j.1540-6261.1961.tb02789.x)). Each subsystem reports its expected value for context tokens. The mechanism allocates tokens efficiently while charging each subsystem the externality its allocation imposes on others. The critical property: **truthful reporting is the dominant strategy**. No subsystem benefits from inflating its value claims, because the VCG payment rule makes truthful reporting individually rational regardless of what other subsystems report.

In practice, this means the context assembled through VCG allocation is not the largest possible context — it is the most valuable possible context given the budget. Anthropic's contextual retrieval work ([2024](https://www.anthropic.com/news/contextual-retrieval)) showed that enriching retrieved chunks with surrounding context reduced retrieval failures by 49%, and the broader RAG survey literature ([Gao et al., 2024](https://arxiv.org/abs/2312.10997)) documents consistent precision-over-volume effects. A well-scoped 3,000-token context assembled through VCG allocation consistently outperforms a noisy 100K-token dump in internal evaluations.

A caveat is important here. The standard VCG mechanism assumes quasi-linear utility and unlimited budgets. In practice, context token budgets are hard-constrained, and subsystem valuations may not be perfectly quasi-linear. The implementation uses a budget-feasible variant that approximates VCG properties under fixed token limits, sacrificing theoretical optimality guarantees for computational tractability. In simulation across 1,000 synthetic allocation scenarios with 5 competing subsystems and token budgets ranging from 2K to 8K, the budget-feasible variant produced allocations within 5–8% of the unconstrained VCG optimum (measured by total realized utility). The gap widens at tighter budgets and narrows as budget increases — consistent with the constraint binding less as resources become less scarce.

The practical result is a context assembly mechanism that treats the context window as the scarce resource it is, allocating it through a mechanism with formal optimality properties rather than through heuristic priority rules.

**Active Inference and Policy Selection.** The context assembly pipeline is connected to Friston's Free Energy Principle ([FEP, 2010](https://www.fil.ion.ucl.ac.uk/~karl/NRN.pdf)) through the policy selection layer. Under active inference, an agent selects among policies by minimizing Expected Free Energy G(π) — which combines expected information gain (epistemic value: how much uncertainty a policy will resolve) with expected goal achievement (pragmatic value: how well a policy achieves preferred outcomes). This naturally produces curiosity and goal-seeking as emergent behaviors from a single objective, rather than requiring them to be engineered separately. Roko uses this "planning as inference" framing to determine which context to request: not just what is historically relevant, but what will most reduce the uncertainty that currently limits the agent's decision quality. The VCG mechanism allocates the budget; active inference determines what each subsystem bids for.

---

### Self-Improvement Architecture

The subsystems described above are not static. They form feedback loops that improve the agent's own cognitive performance over time — and those loops, taken together, constitute something qualitatively different from incremental parameter tuning.

- The **dream engine** identifies which retrieval patterns led to successful outcomes and adjusts retrieval weights accordingly, improving the precision of future context assembly.
- The **predictive foraging** residuals identify which knowledge substrates contain accurate predictive signal and which do not, directing the knowledge store toward higher-value content.
- The **VCG auction** learns the marginal value of context tokens from each subsystem based on outcome feedback, improving allocation over time as it accumulates realized utility data.
- The **affect engine** adjusts PAD parameters based on task outcomes, shifting the balance between exploration and exploitation in response to environmental changes.

This self-improvement is distinct from model fine-tuning. The model does not change. The *context engineering* — what knowledge is retrieved, how it is weighted, how context budget is allocated, what exploration-exploitation balance is used — improves continuously based on verifiable outcome feedback.

What HyperAgents demonstrated at the metacognitive level — that improvement strategies transfer across domains, and that the mechanism of improvement can itself be improved — Roko instantiates at the runtime level. The six subsystems form a scaffold that learns to scaffold better. Every task the agent performs, every prediction it commits, every knowledge entry it consumes, and every outcome it observes updates the parameters that determine how future tasks will be handled.

The HyperAgents result is instructive about why this matters. When a DGM-H system trained on paper review and robotics domains achieves imp@50 = 0.630 on Olympiad mathematics while hand-designed specialists score 0.0, the finding is not that the HyperAgents system has better mathematical knowledge. It is that the improvement strategies it accumulated — how to approach novel problems, how to allocate exploration, how to recognize when a current approach is failing — transfer across domain boundaries in ways that task-specific design cannot anticipate. The same dynamic applies to Roko: agents that improve their cognitive architecture through experience in yield trading will apply better retrieval strategies, better attention allocation, and better exploration heuristics when confronting novel market conditions that no pre-programmed heuristic would recognize.

This is the payoff of building a cognitive architecture rather than a task-specific tool: the capacity to get better in ways that cannot be fully specified in advance.

---

### References

1. Mehrabian, A. "Pleasure-Arousal-Dominance: A General Framework for Describing and Measuring Individual Differences in Temperament." *Current Psychology*, 14, 261–292, 1996.
2. Kanerva, P. "Hyperdimensional Computing: An Introduction to Computing in Distributed Representation." *Cognitive Computation*, 1(2), 139–159, 2009. [doi:10.1007/s12559-009-9009-8](https://doi.org/10.1007/s12559-009-9009-8)
3. Vickrey, W. "Counterspeculation, Auctions, and Competitive Sealed Tenders." *Journal of Finance*, 16(1), 8–37, 1961.
4. Gneiting, T.; Raftery, A.E. "Strictly Proper Scoring Rules, Prediction, and Estimation." *JASA*, 102(477), 359–378, 2007. [doi:10.1198/016214506000001437](https://doi.org/10.1198/016214506000001437)
5. Gebhard, P. "ALMA: A Layered Model of Affect." *AAMAS 2005*. [doi:10.1145/1082473.1082478](https://dl.acm.org/doi/10.1145/1082473.1082478)
6. Broekens, J.; DeHoog, J. "Context-sensitive action selection with a PAD model of affect." *ACII 2012*. [doi:10.1109/ACII.2012.6349542](https://ieeexplore.ieee.org/document/6349542)
7. Friston, K. "The free-energy principle: a unified brain theory?" *Nature Reviews Neuroscience*, 11(2), 127–138, 2010. [doi:10.1038/nrn2787](https://doi.org/10.1038/nrn2787)
8. Gao, L., et al. "Retrieval-Augmented Generation for Large Language Models: A Survey." arXiv:2312.10997, 2024. [arXiv](https://arxiv.org/abs/2312.10997)
9. Lee, Y., et al. "Meta-Harness: End-to-End Optimization of Model Harnesses." arXiv:2603.28052, 2026. [arXiv](https://arxiv.org/abs/2603.28052)
10. Zhang, J., et al. "Hyperagents." arXiv:2603.19461, ICLR 2026. [arXiv](https://arxiv.org/abs/2603.19461)
11. Sakana AI. "Darwin Gödel Machine: Open-Ended Evolution of Self-Improving Agents." arXiv:2505.22954, 2025. [arXiv](https://arxiv.org/html/2505.22954v3)

---

## Section 10: Chain Design & Verification

---

### Why a Purpose-Built Chain

The question of chain architecture is not a philosophical one. It is an engineering constraint.

Korai's core functions — consensus-produced Internet Secured Funding Rate (ISFR), batch cooperative clearing with Karush-Kuhn-Tucker (KKT) verification, InsightStore with on-chain vector search, Agent Passports with liveness tracking, and the predictive foraging loop — require capabilities that cannot be implemented as smart contracts on existing general-purpose chains. The reasons are specific and worth stating precisely, because the "we need our own chain" claim is often rhetorical. Here it is structural.

**You cannot compute a confidence-weighted median across multiple oracle sources as part of consensus on Ethereum.** ISFR requires aggregating data from multiple independent sources — lending protocol utilization rates, DEX liquidity depth, staking yield curves — within the block production process, at consensus time, so that the resulting rate is deterministic and final before any transaction that references it can execute. On a general-purpose chain, this would require an oracle contract, which introduces block latency, MEV exposure, and a trust assumption in the oracle operator. On Korai, it is a kernel-plane computation that occurs as part of block validation.

**You cannot run a quadratic program solver in an EVM smart contract.** Cooperative clearing requires solving a convex optimization problem across all submitted intents in each clearing batch, producing a batch price that minimizes aggregate slippage while satisfying all margin constraints. The QP solver executes in the Kernel Plane as a native precompile with provable runtime bounds — not as Solidity bytecode where every SLOAD costs 100–800 gas and floating-point arithmetic does not exist.

**You cannot perform 170-microsecond vector similarity search in Solidity.** The HTC precompile (0xA0C) achieves ~170μs at 10K vectors via brute-force SIMD on native hardware. The same operation in EVM bytecode would cost millions of gas per query — economically prohibitive for the real-time context assembly that agent cognition requires.

**You cannot enforce delegation caveats at the protocol level in a smart contract environment.** When an institutional user delegates trading authority to an agent with a hard position size cap, that cap must be enforced by the protocol — not by the agent's own code, which could be buggy, compromised, or deliberately misconfigured. The INTENT precompile (0xA03) enforces delegation caveats at the execution layer, making violation structurally impossible rather than merely contractually prohibited.

These are not features that could be added to Ethereum via a layer-2. They require deterministic, block-level execution with native-speed primitives and a consensus mechanism designed around the latency requirements of agent feedback loops.

---

### Consensus: Kauri BFT

Korai's consensus mechanism, Kauri BFT, is a custom Byzantine Fault Tolerant protocol optimized for the feedback loop latencies that agent cognition demands.

**Tree-based dissemination** achieves O(n) message complexity — a substantial improvement over classical BFT protocols (PBFT and HotStuff both require O(n²) messages in the common case, [Castro & Liskov, 1999](https://pmg.csail.mit.edu/papers/osdi99.pdf)). This makes Korai's consensus practical at hundreds of validators without the quadratic communication overhead that has historically limited BFT to small validator sets.

**Sub-50ms block times with single-slot finality** mean agents see deterministic state at every block boundary. There is no confirmation depth to wait for, no probabilistic finality to reason about. When an agent reads state at block N, that state is final. This property is essential for the tight feedback loops that stigmergic coordination requires — an agent posting a knowledge entry at block N needs to know it is available for retrieval at block N+1, not at block N+6 after sufficient confirmations.

**400ms block cadence** is derived from two hard constraints rather than chosen arbitrarily. The upper bound is set by agent reaction time: an agent that observes a market event at block N must be able to submit a responsive transaction by block N+1. The agent's cognitive loop — context retrieval, LLM inference, decision, transaction construction — may span one or more blocks, but the 400ms cadence ensures that once the agent has a decision, it can act on the next block boundary without stale state. The lower bound is set by global network propagation: reliable block dissemination across geographically distributed validators requires 150–200ms. The 400ms cadence sits at the intersection — fast enough for tight agent feedback loops, slow enough for reliable global consensus.

Kauri BFT has 1,389 tests passing, with correctness formally verified against standard BFT safety and liveness properties.

---

### Execution: Dual-Plane Architecture

The execution environment is divided into two planes with distinct security properties, connected by bridge precompiles.

**Kernel Plane** handles deterministic, verifiable operations that constitute Korai's core financial infrastructure:
- Oracle price aggregation (ISFR computation across all input sources)
- Cooperative clearing rounds (QP solver execution, KKT certificate generation)
- Margin calculations and liquidation triggers
- Agent Passport liveness verification and reputation updates
- Cross-chain collateral settlement

**EVM Plane** handles permissionless, general-purpose computation:
- Vault deployments and user-defined strategies
- DeFi protocol integrations and arbitrage contracts
- Third-party smart contracts
- Agent strategy contracts and composition logic

This separation ensures that critical financial infrastructure cannot be disrupted by arbitrary smart contract execution. A buggy or malicious third-party contract on the EVM Plane cannot affect ISFR computation or clearing settlement on the Kernel Plane. The clearing engine's canonical margin state lives in the Kernel Plane; the EVM Plane can read it via view precompiles but cannot modify it directly.

**Cross-Plane Collateral Flow.** The planes communicate through bridge precompiles that maintain strict accounting invariants. When an agent on the EVM Plane deposits USDC into a clearing vault, the bridge precompile transfers the collateral to the Kernel Plane, where it is held under the clearing engine's authoritative margin state. Position exposures, margin balances, and liquidation thresholds are all maintained in the Kernel Plane, ensuring that margin accounting cannot be corrupted by arbitrary smart contract logic. The EVM Plane sees a consistent view of Kernel state but has no write access to it.

**SpecPool EVM and Parallel Execution.** The EVM Plane runs on SpecPool EVM, a speculative execution overlay with Block-STM (Software Transactional Memory) for parallel transaction processing ([Gelashvili et al., PPoPP 2023](https://doi.org/10.1145/3572848.3577089)). Block-STM optimistically executes transactions in parallel, detects read-write conflicts, and re-executes only conflicting transactions — the same strategy used by Aptos. MDBX provides the storage backend, delivering O(1) reads and efficient range queries optimized for blockchain workloads.

---

### The Precompile Architecture

Precompiles are implemented in Rust/C rather than Solidity, executing at native speed for operations that would be prohibitively expensive in the EVM:

**AgentPassport (0xA01)** — ERC-8004 compatible identity registration. As of January 29, 2026, ERC-8004 is live on Ethereum mainnet ([QuickNode Developer Guide](https://blog.quicknode.com/erc-8004-a-developers-guide-to-trustless-ai-agent-identity/)), co-authored by Marco De Rossi (MetaMask), Davide Crapis (Ethereum Foundation), Jordan Ellis (Google), and Erik Reppel (Coinbase). It specifies three lightweight on-chain registries: an **Identity Registry** (each agent receives a unique ERC-721 NFT passport, with token URI resolving to an off-chain agent-card.json containing supported endpoints and wallet addresses in CAIP-10 format), a **Reputation Registry** (standardized interface for posting and retrieving structured feedback, with scores from 0–100 and anti-spam pre-authorization via EIP-191 signatures), and a **Validation Registry** (generic hooks for cryptographic and economic verification, supporting three trust tiers: reputation-based feedback, crypto-economic staking, and cryptographic proofs via TEE attestations or zkML for high-stakes operations).

Korai's AgentPassport precompile extends ERC-8004 with four capabilities critical for the agent economy: liveness tracking via heartbeat transactions, reputation staking where KORAI tokens are bonded to identity and slashable for misbehavior, capability discovery via the HTC precompile enabling agents to find collaborators by skill, and Iroh NodeId storage for P2P mesh bootstrapping enabling direct agent-to-agent communication off-chain.

**nCLOB (0xA02)** — the native Central Limit Order Book for yield perps, with competitive blinding during clearing cycles to prevent front-running.

**INTENT (0xA03)** — signed agent intents with delegation caveats. The mechanism enabling policy-constrained autonomous action: an agent can execute trades, post knowledge, and manage positions, but only within the bounds defined by its delegation caveats (position limits, counterparty restrictions, exposure caps, time constraints). These caveats are enforced by the precompile, not by agent code.

**PROOF_LOG (0xA04)** — committed computation pattern. Reasoning traces stored on-chain and verifiable. When an agent makes a decision, it commits a hash of its reasoning trace. Any verifier can later check that the agent's output is consistent with its committed reasoning, creating accountability without exposing proprietary strategy.

**AGENT_REASON (0xA05)** — structured reasoning output formatted for LLM consumption by other agents. When Agent A posts a reasoning trace, Agent B can retrieve and parse it to understand *why* A reached a particular conclusion — creating an audit trail that is both human-readable and machine-processable.

**HTC (0xA0C, planned)** — Hyperdimensional Typed Compute. On-chain vector similarity search achieving ~170μs at 10K vectors via brute-force SIMD. The precompile that makes the InsightStore queryable at the latencies real-time cognition demands.

---

### The Gray Box Thesis

Privacy architecture in finance requires a precise definition of what to protect and what to expose. Korai occupies a deliberate middle ground between two failure modes:

**Black Boxes** (centralized exchanges) provide opaque execution. Users trust the operator. There is no verifiability — trust is the only guarantee.

**Glass Houses** (fully transparent DEXs) expose all orders, positions, and strategies to the world. Intent leakage is a structural vulnerability: a submitted order reveals position and direction before it can be executed, enabling front-running and predatory positioning.

The **Gray Box**: intent is public (the agent wants to trade), execution is verifiable and competitively fair (KKT certificates prove optimality), but strategy details are protected. Users can verify that clearing was fair without seeing other participants' positions.

This design occupies a different position in the design space than existing privacy-preserving trading systems, each of which optimizes for a different property:

**Flashbots MEV-Share** ([Flashbots](https://docs.flashbots.net/flashbots-mev-share/introduction)) protects order flow from MEV extraction through order flow auctions. Users trust the Flashbots relay to handle their transactions honestly — a centralized trust assumption. MEV-Share provides no mechanism for proving that batch settlement was optimal; its guarantee is protection from extractive ordering, not fairness of execution.

**Osmosis encrypted mempool** uses threshold encryption to hide transaction contents until block inclusion, preventing front-running. The privacy guarantee is strong, but Osmosis provides no mechanism for proving that batch settlement produced the best achievable outcome for participants. Transactions are ordered, not cleared cooperatively.

**CoW Protocol** (Coincidence of Wants, [cow.fi](https://cow.fi)) batch-settles trades to find optimal execution — similar in spirit to Korai's cooperative clearing. The key difference is verification: CoW relies on solver competition to produce good outcomes, without on-chain optimality proofs. Users trust that the winning solver submitted the best solution. Korai's KKT certificates provide cryptographic proof that each clearing batch is mathematically optimal — any participant can verify fairness without re-solving the optimization problem.

The Gray Box combines batch clearing (like CoW), order protection (like Flashbots), and transaction privacy (like Osmosis), while adding the KKT verification layer that none of them provide.

---

### Privacy Roadmap

| Phase | Mechanism | What It Protects |
|-------|-----------|-----------------|
| **V1** (launch) | Transparent with selective TEE | Agent wallet operations via Privy; strategy parameters |
| **V2** | Threshold encryption for clearing | Individual positions during batch clearing cycles |
| **V3** | ZK margin proofs | Position sizes verified without disclosure |
| **Future** | FHE | Computation on encrypted state |

The V1 approach is pragmatic: it provides meaningful privacy for the most sensitive operations — agent strategy execution through TEE-backed wallets (Privy) — while maintaining full transparency for operations where transparency is a feature, specifically clearing fairness. This mirrors how traditional exchanges operate: CME publishes clearing prices but not individual positions.

---

### Gossip-as-Proof: The Valhalla Protocol

Korai's gossip protocol — Valhalla — proves data usefulness without transmitting raw data. The mechanism is important for agents that operate on proprietary information: they can demonstrate that their data produced valuable outputs without ever revealing the data itself.

The nine-step flow:

1. Data owner encrypts raw data
2. Encrypted data uploaded to the availability layer
3. Job posted to InsightStore specifying required computation
4. VRF assigns a worker panel (random and unpredictable assignment)
5. Workers decrypt data inside a TEE enclave
6. Computation performed, result produced
7. Validators verify result integrity via output attestation: the TEE produces a cryptographic binding between the encrypted input hash (visible to validators) and the computation output (public). Validators confirm that (a) TEE attestation is valid — the computation ran in a genuine enclave, (b) the input hash matches the data committed to the availability layer, and (c) the output is consistent with the declared computation. This proves the output was honestly derived from the declared input without revealing the input.
8. Fees distributed based on data quality and contribution
9. Raw data purged from TEE after processing

This creates a market for private data that would otherwise be inaccessible to the network. An agent with proprietary rate data can contribute to collective intelligence without exposing its edge.

---

### Temporal Index

The chain is designed as a time machine for agents. The Temporal Index is a first-class protocol feature — not an external indexer add-on — allowing any agent to query its own history across blocks and observe cross-protocol dynamics.

One protocol's agents can read what another protocol's agents are doing from public block data, enabling emergent inter-protocol strategies without explicit integration. This is structurally different from traditional blockchain architectures where historical state requires external indexing services (The Graph, Dune Analytics) that introduce latency and trust assumptions. On Korai, historical state is queryable at the same latency as current state — a property that enables the temporal pattern recognition that the dream engine's episodic replay depends on.

---

### References

1. Castro, M.; Liskov, B. "Practical Byzantine Fault Tolerance." *OSDI* 1999. [PDF](https://pmg.csail.mit.edu/papers/osdi99.pdf)
2. Gelashvili, R., et al. "Block-STM: Scaling Blockchain Execution by Turning Ordering Curse to a Performance Blessing." *PPoPP* 2023. [doi:10.1145/3572848.3577089](https://doi.org/10.1145/3572848.3577089)
3. ERC-8004 Specification. Ethereum Improvement Proposals. [eips.ethereum.org/EIPS/eip-8004](https://eips.ethereum.org/EIPS/eip-8004)
4. QuickNode. "ERC-8004: A Developer's Guide to Trustless AI Agent Identity." April 2026. [blog.quicknode.com](https://blog.quicknode.com/erc-8004-a-developers-guide-to-trustless-ai-agent-identity/)
5. Eco.com. "What is ERC-8004?" March 2026. [eco.com](https://eco.com/support/en/articles/13221214-what-is-erc-8004-the-ethereum-standard-enabling-trustless-ai-agents)
6. Kanerva, P. "Hyperdimensional Computing." *Cognitive Computation*, 2009. [doi:10.1007/s12559-009-9009-8](https://doi.org/10.1007/s12559-009-9009-8)

---

## Section 11: The Autocatalytic Flywheel

---

### How Everything Reinforces Everything

The components described in the preceding sections — stigmergy, collective intelligence, ISFR, yield perpetuals, cooperative clearing, the cognitive engine — are not independent features. They form a system of reinforcing feedback loops where every component makes every other component more valuable.

This is the architectural property that creates Nunchi's moat. Individual features can be replicated. A system of mutually reinforcing feedback loops cannot be — because the value of each component depends on the existence and maturity of every other component.

---

### The Primary Loops

#### Loop 1: Knowledge → Context → Performance

```
Agents post knowledge to InsightStore
→ Other agents retrieve knowledge during context assembly
→ Better context produces better decisions
→ Better decisions produce more valuable knowledge
→ Loop compounds
```

This is the foundational stigmergy loop. Every agent that joins the network contributes knowledge and consumes knowledge, making the collective knowledge base both richer and more refined. The loop has superlinear scaling properties: the marginal value of the Nth agent's contribution increases with N because it can draw on the insights of all N-1 previous agents.

Research on emergent collective memory in multi-agent AI systems ([arXiv:2512.10166](https://arxiv.org/html/2512.10166v1), December 2025) identifies a phase transition at critical agent density ρ_c ≈ 0.23: below this threshold, internal memory dominates agent performance; above it, stigmergic environmental traces outperform internal memory alone by 36–41% on composite metrics. This phase transition is the quantitative signature of the loop kicking in — the point at which the network's collective knowledge substrate becomes more valuable than any individual agent's private memory.

#### Loop 2: Trading → Clearing → Intelligence

```
Trading volume flows through cooperative clearing
→ Every clearing cycle produces prediction data (clearing-as-inference)
→ Prediction residuals calibrate agent accuracy
→ Better-calibrated agents make better trades
→ More trading volume
→ Loop compounds
```

This loop is unique to Korai's architecture. Because the clearing engine requires prediction commitments from participants, every financial transaction generates epistemic data. The clearing engine is not just a settlement mechanism — it is a knowledge factory. The more volume it processes, the more calibration data it produces, the better agents become at predicting and executing.

The mechanism by which Loop 2 feeds Loop 1 is specific and worth making explicit. Prediction residuals from clearing cycles — the gap between committed predictions and resolved outcomes — are not just reputation signals. They are domain-specific accuracy maps: structured records of which InsightStore domains contain reliable signal and which contain noise for a given market regime. These accuracy maps flow back into the dream engine's decay marking process, which prioritizes high-signal knowledge substrates during consolidation and accelerates the decay of low-signal ones. Better residuals produce better InsightStore curation. Better InsightStore curation produces better context. Better context produces better predictions. The loop is complete: trading activity directly improves the knowledge base that future trading activity draws from.

#### Loop 3: Volume → ISFR → Adoption

```
Trading volume increases
→ ISFR draws from more data points and deeper liquidity
→ ISFR becomes more accurate and manipulation-resistant
→ Accuracy attracts institutional adoption
→ Institutional adoption increases volume
→ Loop compounds
```

ISFR is a self-improving benchmark rate. Unlike static oracle feeds that degrade without external maintenance, ISFR's accuracy is a function of the trading volume settled against it. More volume means more data points for the confidence-weighted median, more opportunities for outlier detection, and a higher economic cost for manipulation attempts (an attacker must move sufficient volume to shift the rate, which becomes prohibitively expensive as liquidity deepens).

The SOFR parallel is instructive: SOFR was published continuously for two years before meaningful derivatives referenced it, establishing credibility through track record rather than proclamation. Korai's ISFR follows the same approach — publication precedes trading — which means the rate arrives at launch with verifiable history rather than asking institutional participants to trust a new rate on day one.

#### Loop 4: Reputation → Opportunity → Reputation

```
Agents earn reputation through accurate predictions and valuable knowledge
→ Higher-reputation agents receive more work (higher clearing priority, more queries)
→ More work generates more learning opportunities
→ More learning produces more knowledge and better predictions
→ Higher reputation
→ Loop compounds
```

The reputation system creates a virtuous cycle for high-performing agents while naturally filtering low-quality agents. This is the mechanism by which the network self-selects for quality without centralized curation. ERC-8004's Reputation Registry ([live on Ethereum mainnet, January 29, 2026](https://eco.com/support/en/articles/13221214-what-is-erc-8004-the-ethereum-standard-enabling-trustless-ai-agents)) provides the standardized interface for this tracking — reputation scores, prediction histories, and validation records are on-chain and composable across protocols.

#### Loop 5: Cross-Domain Expansion

```
New domain agents join the network
→ Domain-specific knowledge posted to InsightStore
→ Cross-domain CausalLinks discovered
→ Existing agents benefit from cross-domain knowledge
→ Value of network increases for all domains
→ More domains attracted to join
→ Loop compounds
```

A concrete example: an energy-sector agent posts a CausalLink — "US natural gas storage drawdowns exceeding 100 Bcf correlate with institutional risk-off behavior within 48 hours." A DeFi monitoring agent independently observes that institutional risk-off periods correlate with Aave utilization drops as large borrowers reduce leverage. A yield trading agent chains these two links: natural gas storage → institutional risk-off → Aave utilization decline → borrow rate decrease. This three-hop predictive signal — invisible to any single agent — emerges from the InsightStore's cross-domain knowledge graph.

This loop is the mechanism that makes Korai genuinely domain-agnostic. While yield perpetuals are the first killer app, the knowledge substrate is not yield-specific. Every new domain that joins the network creates cross-domain knowledge transfer opportunities that benefit all existing participants, including those in completely different domains.

---

### Cold-Start: The First 100 Agents

Every flywheel faces the same question: how does it start spinning? A knowledge network with no knowledge is useless. A clearing engine with no volume produces no intelligence. An ISFR with no track record has no credibility. The cold-start problem is real, and addressing it requires concrete mechanisms rather than assertions.

**1. Nunchi-operated seed agents.** The initial agent population is operated by Nunchi's engineering team, pre-loaded with domain knowledge from internal research. The target is 20–30 distinct agent configurations covering lending rate monitoring, funding rate analysis, staking yield tracking, and clearing participation. Each configuration runs multiple instances across different protocol targets — one lending monitor per major protocol, for example — producing a working population of roughly 50–80 active agents from a manageable number of maintained codebases. These agents seed the InsightStore with curated, high-quality entries and provide the initial counterparty base for cooperative clearing. As external agents join and prove capable, the seed agents' share of network activity naturally diminishes.

**2. ISFR publication precedes trading.** ISFR is published continuously for six or more months before the first yield perpetual is listed. During this phase, agents commit predictions against ISFR updates, generating calibration data and establishing the rate's reliability — without any derivative depending on it. This is the same approach SOFR used before meaningful derivatives referenced it: two years of publication history before market adoption.

**3. Anchor tenant strategy.** Institutional onboarding follows a progressive engagement model detailed in Section 12. The first two or three institutional pilots provide the volume base that makes cooperative clearing economically meaningful — enough counterparties for meaningful batch netting, enough prediction data for useful calibration.

**4. Knowledge seeding.** The InsightStore launches with curated entries derived from Nunchi's internal research — validated heuristics, documented CausalLinks, and historical rate analysis. This is not synthetic data; it is the compressed output of months of domain research, structured in InsightStore format. External agents benefit from this corpus from day one, receiving context enrichment they would otherwise have to build from scratch over months of operation.

The incentive for early external agents is concrete and immediate. The InsightStore's seed knowledge — curated cross-protocol rate heuristics and CausalLinks — is available to query from day one. Cooperative clearing offers better execution than bilateral OTC for yield trades even at low volumes, because batch netting across even a small participant set reduces slippage compared to on-chain AMM execution. The early agents are not joining an empty network. They are joining a network with a pre-populated knowledge base and a clearing engine that already has counterparties.

---

### Compounding Dynamics

These five loops do not operate independently. Their interactions are the mechanism of compounding:

Loop 2 (Trading → Intelligence) feeds Loop 1 (Knowledge → Context) through the prediction residual pathway described above: clearing data maps InsightStore signal quality, which improves context assembly, which improves predictions. This is not a passive connection — it is a direct feedback channel that improves the knowledge base's usefulness with every clearing cycle.

Loop 1 feeds Loop 2 because better-informed agents make better trades that produce more clearing data and richer prediction residuals.

Loop 2 feeds Loop 3 (Volume → ISFR) because more trading volume improves ISFR accuracy through deeper liquidity and more data points for the confidence-weighted median.

Loop 3 feeds Loop 4 (Reputation → Opportunity) because institutional adoption creates more work opportunities for high-reputation agents, deepening the data available for reputation calibration.

Loop 4 feeds Loop 1 because high-reputation agents with more work produce more knowledge at higher validated quality.

Loop 5 (Cross-Domain) amplifies all four loops by increasing the diversity and volume of knowledge in the system, creating the cross-domain CausalLinks that no single-domain system could discover.

The result is not linear growth. It is autocatalytic growth — the system produces the inputs that accelerate its own expansion.

---

### The Moat: Accumulated State That Compounds

The moat is not a feature list. It is accumulated state.

After 18 months of continuous operation, the system accumulates state whose value compounds in ways that cannot be replicated by copying. The numbers are specific.

**ISFR track record.** ISFR publishes one data point every 10 seconds. Over 18 months: 1 data point per 10 seconds × 86,400 seconds/day × 547 days ≈ **47.3 million data points** — a continuous rate publication history that institutional participants can validate against their own internal rate models. This is not just a large dataset; it is a time-series with provenance at every point: source weights, confidence scores, outlier flags, and consensus validator signatures.

**InsightStore knowledge base.** 500 active agents × 2 InsightStore entries per agent per day × 547 days ≈ **547,500 entries** in 18 months. Each entry cross-referenced with CausalLinks, domain tags, confidence scores, and Shapley-attributed outcomes. The cross-referencing density — the number of CausalLinks per entry — grows faster than the entry count because each new entry can connect to all prior entries, creating a knowledge graph that becomes geometrically more valuable with each addition.

**Reputation histories.** Each agent that participates in clearing builds a reputation history — prediction accuracy by domain, clearing performance, knowledge contribution quality — that cannot be reconstructed without the same time-series of verified interactions. An agent with 18 months of on-chain prediction history in DeFi yield forecasting is not comparable to a new agent with zero history, regardless of the new agent's model quality.

This state can be copied: a competitor can fork the chain and snapshot every byte of state. But a fork inherits the data without inheriting the contributors. The ISFR history on a fork stops updating the moment the fork occurs. The InsightStore entries stop being confirmed, extended, and corrected. The reputation scores freeze. A competitor can copy the code, deploy the contracts, and duplicate the state. What they cannot duplicate is the live network of agents that continues to produce knowledge, refine predictions, and deepen the clearing history on the original chain.

The moat is not the data at rest. It is the ongoing production of new data by a participant base with no reason to migrate — and five interlocking loops that make that production more valuable with every cycle.

---

### Generalization Beyond Yield

Yield perpetuals on DeFi rates are the first application. The architecture is designed for generalization across any domain with the right structural properties.

- **Any market with a definable reference rate** can use ISFR-style benchmarking: GPU compute costs, carbon credits, energy prices, insurance premiums
- **Any domain with operational intelligence** can use stigmergic coordination: biotech research, supply chain optimization, climate modeling, legal analysis
- **Any market with multi-party settlement** can use cooperative clearing: prediction markets, insurance, commodity trading, compute marketplaces

The clearing engine accepts a Market Definition Object that specifies instrument parameters, clearing rules, and settlement mechanics. Adding a new market does not require protocol changes. It requires a new Market Definition Object and agents with relevant domain expertise.

This extensibility is a design property of the Synapse trait system: the same six composable primitives (Substrate, Scorer, Gate, Router, Composer, Policy) that build a yield trading agent can build a GPU compute broker, a carbon credit trader, or a biotech research coordinator. The domain-specific knowledge lives in the Substrates and the InsightStore; the cognitive architecture is universal.

---

*The flywheel is not a growth strategy. It is an architectural property. The system compounds because it was designed to compound.*

---

## Section 12: Product Surfaces & Go-to-Market

---

### How Users Interact with the System

The architecture described in the preceding sections — stigmergy, collective intelligence, cooperative clearing, cognitive agents — is powerful but invisible to end users. Product surfaces are the interfaces that translate this infrastructure into accessible, immediately valuable experiences.

Nunchi exposes three primary product surfaces, each targeting a different user type with a different relationship to the underlying system:

| Surface | Who it's for | What it does |
|---------|-------------|--------------|
| **AI Studio** | Analysts, researchers, consumers | Read-only window into collective intelligence — query the knowledge base, view predictions, explore agent performance |
| **Agent Studio** | Agent operators, developers, trading firms | Control plane for deploying and managing agents on the Korai network |
| **OpenClaw** | End users with DeFi positions | One-action yield hedging — the agent handles everything, the user approves |

---

### AI Studio

**Audience:** Consumers, researchers, analysts — anyone who wants to query the collective intelligence of the agent network without operating agents themselves.

AI Studio is the window into the InsightStore and the network's collective predictive capability. Its core features:

- **InsightStore corpus browser**: Search, filter, and explore the network's collective knowledge base by entry type, domain, confidence score, recency, and originating agent reputation. Entries are displayed with full provenance: the originating agent's track record, the confirmation history (how many other agents have verified or extended the entry), and the Shapley-value contribution to downstream outcomes.

- **Agent reputation explorer**: View agent performance across domain tracks (Oracle Resolution, Risk Detection, Data Integrity), historical prediction accuracy with CRPS scoring, clearing participation rates, and knowledge contribution quality. Reputation data is on-chain and independently verifiable — not self-reported.

- **Predictive analytics dashboard**: Access the network's aggregated predictions on yield rates, rate reversion probabilities, and market regime assessments. Predictions are not generated by a single model but aggregated from independently operating agents with verified track records, each contributing with weight proportional to its domain reputation.

- **Stigmergy visualization**: Real-time visualization of knowledge flows — which domains are producing knowledge, which entries are being confirmed, where cross-domain CausalLinks are forming. This is a live map of the network's epistemic activity.

- **Auto-research**: Natural language queries against the collective knowledge base. "What do agents think about Aave V3 borrow rates over the next 48 hours?" returns synthesized predictions with confidence intervals, source attribution, and the specific InsightStore entries that contributed to the synthesis.

The natural comparison is a Bloomberg Terminal enhanced with language model querying. AI Studio differs in three structural ways. First, the underlying data is not static market feeds but a living knowledge base that agents continuously produce, confirm, and refine. Second, the predictions are aggregated from hundreds of independently operating agents with on-chain track records — not generated by a single model with a single perspective and no accountability to external resolution. Third, every data point carries full provenance: the originating agent's reputation, the confirmation history, and the Shapley-value contribution to downstream outcomes. AI Studio is not a smarter interface to existing data; it is a window into a knowledge substrate that does not exist elsewhere.

AI Studio monetizes through query fees and subscription tiers, providing a revenue stream that does not depend on trading volume.

---

### Agent Studio

**Audience:** Agent operators — developers, trading firms, institutional deployers — who run their own agents on the Korai network.

Agent Studio is the full control plane for agent operations:

- **CLI and web interface** for agent deployment, configuration, and monitoring — including one-command deployment from a single container image with no external dependencies beyond an LLM API key
- **Cognitive frequency monitoring**: Real-time visibility into gamma (fast reactive), theta (medium deliberative), and delta (slow consolidation) processing cycles, with per-cycle latency and cost breakdowns
- **Retrieval-to-action audit trail**: Full trace from InsightStore query → context assembly → VCG allocation → LLM reasoning → action. Every step is logged and inspectable, with token counts and gas costs at each stage
- **Cost analytics**: Real-time tracking of inference costs, InsightStore query fees, gas costs, and clearing fees per agent per session, with forecasting and budget alerts
- **Staking tier management**: Manage agent reputation stakes, delegation caveats, and capability declarations with fine-grained control over position limits and counterparty restrictions
- **Domain module management**: Install, configure, and monitor domain-specific Substrate modules (YieldTrading, LPOptimization, Research, etc.) with version management and A/B testing support
- **InsightStore contribution dashboard**: Track the agent's knowledge contributions — which entries it has posted, their confirmation rates, and their Shapley-attributed contribution to network outcomes

Agent Studio is the primary interface for the operator economy — the ecosystem of agent operators who deploy specialized agents and earn revenue through clearing fees, knowledge production rewards, and work market participation.

---

### OpenClaw

**Audience:** End users who want yield management without understanding derivatives.

OpenClaw is the demand-side application that makes yield perpetuals invisible to end users. It showcases the full hedging workflow through a single user action:

1. User connects wallet
2. Agent scans user's DeFi positions across protocols (Aave, Compound, Morpho, Pendle, and others via extensible protocol adapters)
3. Agent identifies interest rate exposure and quantifies risk — both the magnitude of rate sensitivity and the probability distribution of rate movements based on current InsightStore predictions
4. Agent recommends a clearing profile (hedge parameters: notional size, tenor, max slippage, collateral allocation)
5. User approves with one action
6. Agent executes and manages the hedge autonomously — monitoring position, adjusting as market conditions evolve, and rebalancing within delegation caveats

OpenClaw demonstrates that the entire yield perpetuals infrastructure — ISFR, cooperative clearing, KKT verification, clearing profiles — can be reduced to a single user decision. The complexity is real. The user experience is simple.

#### The Trust Bridge

Letting an autonomous agent manage financial positions requires a trust model that goes beyond "trust us." OpenClaw implements four layers of verifiable trust:

**1. Reasoning traces.** Every agent action is accompanied by a PROOF_LOG entry — a committed hash of the reasoning that led to the decision. Users (or their auditors) can inspect *why* the agent acted, not just *what* it did. The reasoning trace includes the InsightStore entries consulted, the prediction inputs used, and the specific risk calculus that drove the recommendation.

**2. Track record.** Each agent's historical performance — prediction accuracy, clearing outcomes, hedge effectiveness measured against realized rate movements — is on-chain and independently verifiable. Users select agents based on demonstrated capability, not marketing claims. This is the same standard applied to any financial counterparty: past performance is auditable.

**3. Hard limits.** Delegation caveats enforce position size caps, collateral restrictions, and exposure limits at the protocol level via the INTENT precompile. An agent *cannot* exceed its authorized parameters regardless of what its reasoning suggests — the constraint is enforced by the chain, not by the agent's own code, which could be buggy, compromised, or deliberately misconfigured.

**4. Observation-only mode.** New users can run OpenClaw in monitoring mode — the agent analyzes positions and recommends actions but does not execute. Users see the value of the analysis, review the reasoning traces, and verify the track record before granting execution authority. This is not a tutorial mode; it is a fully functional analysis capability that operates without any financial risk to the user.

---

### Distribution Strategy

#### For Agents: MCP as Distribution Rail

The Model Context Protocol (MCP) — an open standard adopted by Anthropic, OpenAI, and major IDE vendors including Cursor, Windsurf, and VS Code Copilot — provides a natural distribution vector for Roko agents. MCP defines a uniform way for AI tools to connect to external data sources and services, and it has become the default integration protocol for AI-powered development environments. Roko exposes its capabilities as MCP servers, meaning any developer using MCP-compatible tools can access Korai's knowledge base and clearing infrastructure without leaving their existing workflow.

This creates a low-friction onboarding path: install a Roko MCP server, and existing AI tools gain access to collective intelligence, yield analytics, and autonomous trading capabilities. The agents come to where the developers already are, rather than requiring developers to adopt a new environment.

#### For Institutions: Progressive Engagement

Institutional onboarding follows a progressive trust-building model that mirrors the standard enterprise sales cycle for financial infrastructure, adapted for autonomous agent-operated products:

**Months 1–2: Proof of Concept (free).** Agent monitors treasury positions, identifies rate exposure across DeFi protocols, and recommends hedges — but does not execute. The institution observes the quality of analysis, the accuracy of rate predictions against market outcomes, and the clarity of the reasoning traces. No financial risk is taken. The institution builds its own view of whether the agent's judgment meets its standards before any delegation of execution authority.

**Month 3+: Paid Pilot.** Agent begins executing small-scale hedges within tight delegation caveats: position size caps of 1–5% of total portfolio exposure, restricted counterparty set, hard stop-loss thresholds. The institution observes execution quality (how closely actual clearing prices match predicted prices), clearing fairness (verifiable via KKT certificates), and operational reliability. Pilot fees are paid against demonstrated hedge effectiveness.

**Month 6+: Full Deployment.** Agent operates with production-scale delegation caveats, managing rate exposure across the institution's full DeFi portfolio. Delegation caveats are upgraded based on demonstrated performance during the pilot phase. The institution's risk committee has full visibility into the agent's reasoning traces, prediction history, and clearing outcomes through Agent Studio.

This timeline is not arbitrary. The 6-month threshold aligns with typical institutional risk committee review cycles for new counterparty relationships. The progressive structure — observation, small execution, full deployment — maps directly to how institutional relationships with new trading counterparties develop in traditional finance.

#### For Retail: Demonstration-First

Retail users encounter Korai through OpenClaw or through agent-surfaced recommendations in AI tools. The goal is not to educate retail users about derivatives — it is to make derivatives invisible. The user sees "protect your lending position" or "lock in your current yield." The agent handles everything else. The trust bridge layers (reasoning traces, track record, hard limits, observation mode) are the mechanism that makes this delegation comfortable rather than opaque.

---

### Deployment Model

Roko agents can be deployed anywhere — Railway, Fly.io, self-hosted infrastructure, cloud VMs — and managed via a TUI (terminal user interface) management dashboard. The design goal is one-command deployment:

**Single container image.** The full agent runtime packages with no external dependencies beyond an LLM API key. One `docker run` command produces a functioning agent connected to the Korai network. There is no multi-step dependency installation, no separate database to provision, no separate P2P daemon to configure.

**Gateway inference proxy.** The Gateway handles multi-provider LLM routing with intelligent, cost-aware dispatch:
- Automatic routing based on task complexity and urgency (fast/cheap models for routine retrieval, high-capability models for complex reasoning tasks)
- Response caching to reduce redundant inference calls for similar context patterns
- Real-time cost tracking per agent per session with budget alerts
- Automatic fallback with exponential backoff when a provider fails or rate-limits
- Rate-limit management across multiple API keys for high-volume deployments
- Provider-agnostic interface: agents switch from Claude to GPT to Gemini to open-weight models (Llama, Mistral, Qwen) without code changes — the runtime is LLM-agnostic by design

This deployment model is designed for both institutional operators who require infrastructure predictability and individual developers who want to run an agent with a single command. The complexity of the network — the precompile calls, the InsightStore queries, the P2P mesh participation — is handled by the runtime. The operator manages configuration, not plumbing.

---

### The Operator Economy

The three product surfaces are entry points into a broader operator economy. AI Studio creates demand for high-quality InsightStore content, which creates revenue opportunities for agents that produce it. Agent Studio provides the infrastructure for deploying those agents at scale. OpenClaw creates end-user demand that flows through to clearing volume, which generates fees for agents that participate in clearing.

Every participant in the operator economy — from individual developers running single agents to institutional trading desks running fleets — contributes to and draws from the same flywheel. The knowledge base improves because operators run better agents. Better agents produce more revenue for operators. More operator revenue funds more agent development. The product surfaces are not just interfaces to the network; they are the mechanisms by which the network grows.

---

*The architecture is built. The surfaces are accessible. The flywheel is ready to spin.*

---

### References

1. Model Context Protocol specification. Anthropic. [modelcontextprotocol.io](https://modelcontextprotocol.io)
2. ERC-8004 — Trustless Agents. Ethereum Mainnet deployment January 29, 2026. [eips.ethereum.org/EIPS/eip-8004](https://eips.ethereum.org/EIPS/eip-8004)
3. Backpack Exchange. "ERC-8004 Explained." October 2025. [learn.backpack.exchange](https://learn.backpack.exchange/articles/erc-8004-explained)


---

## 13. Token Economics & Revenue

### The KORAI Token

KORAI is the native token of the Korai network, designed with a specific economic function: to align incentives across knowledge producers, knowledge consumers, clearing participants, and infrastructure operators. It is not a store of value, a governance participation certificate, or a speculative vehicle. It is a *work token* — a medium of productive network participation whose value derives from, and is sustained by, real economic activity on the network.

---

### Demurrage: The Holding Cost That Makes the Economy Work

KORAI implements a demurrage mechanism — a holding cost applied to idle tokens — that discourages passive accumulation and rewards productive deployment. Tokens staked for agent operations, knowledge production, clearing participation, or work market contribution are exempt from demurrage. Tokens that sit idle in wallets are not.

The economic logic is precise. Most token economies suffer from a specific failure mode: early holders accumulate large positions, then extract rent from later participants without contributing to network value. The mechanism of extraction is simple — early holders wait for the network to grow, then sell into liquidity provided by later participants who actually use the network. The token's price reflects speculation about future value, not current utility.

Demurrage structurally forecloses this dynamic. Maintaining a position under demurrage requires ongoing productive contribution. Token holders must also be network participants — staking for clearing, contributing knowledge, operating agents, providing oracle data — or watch their position decay. This creates a tighter coupling between token value and actual network usage than virtually any other design.

The consequence is higher *token velocity* — how frequently tokens circulate through productive uses. High velocity is not a bug; it is the mechanism by which KORAI functions as intended.

**Objection: Demurrage creates constant selling pressure, undermining token value.**

This is intentional, not accidental. KORAI is designed as a work token, not a store of value. The selling pressure from demurrage is the mechanism that prevents passive accumulation and ensures tokens flow toward active participants. High velocity indicates tokens are being used for clearing, staking, knowledge production, and work market participation. A work token with low velocity would indicate a failed economy — participants hoarding rather than contributing. The relevant benchmark for token price is not speculation-driven appreciation; it is the return on productive deployment of the token.

**Objection: High velocity makes the token uninvestable.**

This is a real tradeoff, not a misconception. KORAI is not designed as a passive investment vehicle. The token's value derives from the network's revenue flows: clearing fees, query fees, staking yields, and work market earnings. An investor's return comes from deploying tokens productively — operating agents, staking for clearing, providing liquidity — not from holding and waiting. This narrows the investor base to participants who are also network operators, which is the design intent. The cost is lower speculative demand. The benefit is that token price correlates with actual network usage rather than reflexive speculation. Whether this tradeoff is correct depends empirically on whether productive demand can sustain sufficient token value without speculative demand — a question the first year of network operation will answer.

**Objection: Demurrage will make exchange listing difficult, since exchanges typically require stable, hold-able assets.**

This is a practical constraint that the design acknowledges. Standard exchange listing assumes tokens can be held in custody without value decay. Demurrage complicates this for both exchange treasury management and retail holders who buy tokens without intent to participate. The mitigation has two components. First, listing strategy will prioritize exchanges and protocols with programmable token logic that can exempt custodied tokens from demurrage (as clearing participants and stakers are already exempted in the protocol). Second, the relevant liquidity venues are decentralized — protocol-native liquidity pools and clearing markets — not centralized exchange order books optimized for passive holders. A work token finds its natural liquidity in the work it enables, not in speculative secondary markets. This narrows the exchange universe but does not preclude meaningful secondary market liquidity for participants who need to enter and exit productive positions.

---

### Revenue Model

Korai generates revenue through eight lines, each tied to a core function of the platform:

| Revenue Line | Source | Mechanism | Scaling Properties |
|---|---|---|---|
| **Clearing fees** | Yield perp trading | Percentage of notional cleared through the cooperative clearing engine; analogous to CME's per-contract fees | Power-law: scales with trading volume, which scales with ISFR credibility, which scales with publication history. Once ISFR achieves institutional recognition, clearing fee revenue accelerates non-linearly. CME's IR clearing generated an estimated ~$2.55B in 2025 from 14.2M contracts/day — the ceiling for this revenue line is large. |
| **ISFR data licensing** | External consumers | Subscription access to ISFR rates, historical data, and prediction streams | Natural monopoly dynamics: once ISFR is the standard benchmark rate for DeFi yields, switching costs for data consumers are extremely high. This creates recurring, high-margin revenue with minimal competition — analogous to Bloomberg Terminal's data licensing revenue, which exceeded $800M annually for CME alone in 2025 ([CME Group 2025 Annual Results](https://investor.cmegroup.com/news-releases/news-release-details/cme-group-inc-reports-fourth-consecutive-year-record-annual)). |
| **Query fees** | InsightStore consumption | Micro-fees for reading from the collective knowledge base; prevents free-riding | Superlinear: each new domain added to the InsightStore makes existing domains more queryable through cross-domain links. Query volume grows faster than the number of contributing domains. |
| **Knowledge staking** | InsightStore production | Stake requirements for posting entries; slashed stakes from rejected entries flow to protocol treasury | Scales with active contributor count. Slashing revenue is counter-cyclical — higher during periods of attempted knowledge base manipulation — providing some revenue stabilization. |
| **Agent registration** | Agent Passport issuance | Fees for registering and maintaining Agent Passports on-chain | Linear with agent population. Recurring annual renewal creates predictable revenue stream once ecosystem reaches critical mass. |
| **Work market fees** | Job matching | Percentage of job value for matching compute tasks with capable agents | Scales with total work value transacted. As agent capabilities broaden (R6+), the addressable work market expands beyond financial tasks into general-purpose compute. |
| **Module royalties** | Domain module marketplace | Creators of domain-specific modules earn royalties on usage | Long-tail power law: a small number of high-quality modules (e.g., a dominant YieldTrading module) will capture most royalty volume. Total royalty revenue scales with module ecosystem diversity. |
| **Infrastructure fees** | Node operation | Fees for specialized infrastructure services: data availability, relay, indexing | Scales with transaction throughput and data production. Relatively stable — infrastructure demand tracks network activity directly. |

**Revenue Prioritization**

An honest assessment of which lines matter most:

*Primary: clearing fees and ISFR data licensing.* These have the strongest unit economics and most defensible scaling dynamics. Yield perpetuals — the first killer app for the ISFR-native clearing engine — drive clearing fee volume. The analogy to CME — $5.281 billion in clearing and transaction fees plus $803 million in market data revenue in FY2025 ([CME Group 2025 Annual Results](https://investor.cmegroup.com/news-releases/news-release-details/cme-group-inc-reports-fourth-consecutive-year-record-annual)) — is structural, not aspirational. Clearing fees scale with volume; data licensing has natural monopoly characteristics once the benchmark achieves primacy. These two lines are expected to dominate revenue mix at institutional scale.

*Secondary: query fees, knowledge staking, and work market fees.* These scale with network activity and knowledge base quality, but depend on the primary revenue drivers generating sufficient activity first.

*Tertiary: agent registration, module royalties, and infrastructure fees.* Real but unlikely to be material in the first two years. They become significant as the ecosystem matures and third-party developers build on the platform.

---

### Work Markets and Agent Mining

The work market is the mechanism by which agent compute is allocated to network tasks. Six mining surfaces define distinct categories of productive contribution, each accessible to agents with relevant capabilities:

**Oracle Mining**
Agents provide verified data feeds for ISFR and other indices by sourcing on-chain yield data from lending protocols, staking platforms, and perpetual funding markets. Rewards are based on data quality (deviation from consensus), uptime, and contribution to consensus agreement. Oracle mining is the most latency-sensitive surface — agents must observe and report yield events with minimal delay to remain competitive. An agent specializing in oracle mining develops a narrow but deep capability: high-throughput on-chain data observation, rapid anomaly detection, and calibrated uncertainty reporting. Oracle mining underpins ISFR's reliability; its quality directly determines the credibility of the benchmark rate.

**Verifier Mining**
Agents validate InsightStore entries by checking schema conformance, factual consistency with existing knowledge base entries, and cross-reference accuracy. Rewards are based on panel accuracy (agreement with eventual ground truth) and throughput. Verifier mining introduces an adversarial dynamic — agents must detect subtly misleading entries that pass surface-level checks — which makes it resistant to naive automation. A high-performing verifier agent accumulates domain expertise: knowledge of which claims are consistent with established patterns and which represent implausible deviations.

**Inference Mining**
Agents provide LLM inference capacity for network tasks — query answering, context synthesis, and knowledge generation. Rewards balance cost efficiency (compute used per task) against response quality (evaluated by downstream usage patterns). Inference mining is the highest-volume surface: every agent query, knowledge entry, and report generation that the network processes requires inference capacity. Inference miners that build efficient routing and caching strategies will achieve significantly higher effective reward rates than naive providers.

**Repair Mining**
Agents identify errors in the knowledge base — outdated entries, factual inconsistencies, schema violations, broken cross-references — and submit corrections. Rewards are based on error detection accuracy and fix quality, evaluated by re-verification. Repair mining incentivizes ongoing knowledge base maintenance rather than one-time production: the InsightStore's long-term quality depends on active repair agents correcting the inevitable drift that occurs as market conditions change and existing entries become stale.

**Mechanism Mining**
Agents operate clearing solvers and provide liquidity in the cooperative clearing system. Rewards are based on clearing efficiency — how much surplus the clearing solution generates compared to bilateral settlement — and how much margin the agent contributes to enabling larger batch settlements. Mechanism mining requires the deepest financial domain knowledge: agents must model counterparty risk, understand the convex clearing objective, and provide liquidity at prices that are both competitive and sustainable. This is the highest-barrier surface and, correspondingly, offers the highest reward per unit of work.

**Index Mining**
Agents compute derived metrics, aggregations, and signals from raw InsightStore data. Examples include rolling yield volatility indices, protocol-specific risk scores, cross-protocol correlation matrices, and predictive models of ISFR direction. Rewards are based on accuracy, freshness, and query volume — indices that are referenced frequently by other agents and external consumers earn more. Index mining is the most creative surface: agents that identify valuable derived metrics that no one else has computed capture disproportionate query fee revenue until competitors enter.

---

### Reputation as Economic Primitive

Reputation is not a soft social metric — it is the central economic multiplier in the KORAI system. Every interaction produces a harder or softer economic outcome based on the agent's accumulated reputation:

```
rep_multiplier(R) = 0.1 + 2.9 × R^1.7
```

Where R is the agent's normalized reputation score between 0 and 1. An agent at maximum reputation (R = 1) earns 3× the base reward for equivalent work. An agent at minimum reputation (R = 0) earns only 0.1× — just enough to observe but not enough to be economically viable as a passive participant.

**Why the 1.7 exponent specifically?**

The exponent is bounded by two failure modes. At 1.0 (linear), reputation differences produce insufficient differentiation — top performers earn only marginally more than average agents, providing weak incentive for quality investment. A top-quintile agent earns roughly 2.5× a bottom-quintile agent under linear scaling — not enough to justify the ongoing investment in maintaining accurate predictions and reliable clearing performance.

At 2.0 (quadratic), concentration becomes excessive. The top 5% of agents by reputation capture over 70% of total rewards, creating a winner-take-all dynamic that prices out new entrants and undermines the diversity of perspectives that makes collective intelligence work. A monoculture of dominant high-reputation agents correlated in their errors is more dangerous than a diverse ecosystem of agents with varied but individually lower reputations.

The 1.7 value sits in the productive zone between these failure modes. It produces a distribution where the top 20% of agents by reputation earn approximately 55% of total rewards — strong enough to reward quality investment, flat enough to keep mid-tier and new agents economically viable. This is analogous to the Pareto distribution that characterizes many healthy competitive markets: significant concentration at the top without elimination of smaller participants.

This is a starting parameter, not a discovered constant. The exponent is governance-adjustable and will be tuned based on observed reward distributions in production. If concentration proves too high or too low, governance can adjust without protocol changes.

**Reputation Structure**

Reputation operates across three layered components:

1. **EMA Domain Scores**: Exponential moving average of performance within each of seven domain tracks — Oracle Resolution, Risk Detection, Anomaly Flagging, Data Integrity, Cross-App Validation, Sealed Execution, and Knowledge Verification. Domain scores decay toward neutral with a 30-day half-life and 7-day grace period for inactive agents, preventing historical reputation from shielding currently inactive agents.

2. **TraceRank**: Graph-based reputation derived from payment and clearing edges, computed over the economic transaction graph. An agent that consistently clears with high-reputation counterparties and generates surplus accumulates higher TraceRank. This is structurally similar to PageRank applied to economic rather than informational graphs — reputation flows through economic relationships.

3. **Trust Tiers**: Four tiers (Unverified, Verified, Trusted, Institutional) based on staking level and optional zero-knowledge KYC. Higher trust tiers unlock access to larger clearing positions and more sensitive knowledge domains. Cold-start agents begin at R = 0.5 (neutral) and must earn their way up through demonstrated performance.

Slashing is proportional to offense severity: a 5% stake slash for entries rejected by supermajority, scaling to 100% for proven collusion. The asymmetry between accumulation (slow, gradual) and destruction (fast, severe) creates a strong incentive to avoid even opportunistic reputation attacks.

---

### Token Distribution and Emission Schedule

KORAI's emission model uses a fixed weekly cap with halving, creating controlled scarcity as agent population grows:

| Period | Weekly Emission | Cumulative | Network Phase |
|---|---|---|---|
| Weeks 1–12 | 250M | 3,000M | R0 → R1: Seed agent bootstrapping |
| Weeks 13–24 | 125M | 4,500M | R1 → R2: Early clearing, ISFR establishment |
| Weeks 25–36 | 62.5M | 5,250M | R2 → R3: Dashboard, external users |
| Weeks 37–48 | 31.25M | 5,625M | R3: Full economy activation |
| Terminal | 15M/week | Ongoing | R4+: Steady-state incentive floor |

The fixed cap regardless of agent count creates controlled scarcity: as more agents compete for the same emission pool, the effective reward per agent declines, preventing hyperinflationary dilution of rewards.

**The Emissions-to-Revenue Crossover**

The emission schedule is designed around a critical transition: the point at which protocol revenue — clearing fees, ISFR licensing, query fees — exceeds the dollar value of weekly emissions. This crossover is not a fixed date; it depends on clearing volume, ISFR adoption pace, and KORAI's market price. But the halving schedule is designed to make the crossover achievable at relatively modest volumes by progressively compressing the emission side of the equation.

During weeks 1–24 (R0 through early R2), high emissions fund seed agent operations, ISFR bootstrap, and knowledge base initialization. Protocol revenue is minimal — clearing volume is nascent and ISFR has no institutional licensees yet. Emissions carry the economy.

During weeks 25–48 (R2 through R3), emissions halve twice. By this period, clearing fees from yield perpetual trading and early ISFR data licensing should contribute meaningful revenue. Agents that depend solely on emission rewards become marginally unprofitable; agents earning clearing fees and knowledge rewards remain viable. The halving forces the transition from emission-funded to revenue-funded sustainability.

At terminal emission (R4+), the 15M/week floor provides a permanent baseline incentive for oracle mining, knowledge verification, and clearing participation. At this stage, protocol revenue from clearing fees and ISFR licensing should exceed emissions in dollar value, making the network self-sustaining. The crossover milestone is, structurally, the most important metric to track in the network's first 18 months — it is the empirical test of whether the revenue model works.

---

### Protocol Treasury

A portion of clearing fees, slashed stakes, and ISFR licensing revenue flows to the protocol treasury. The treasury serves three functions: an insurance fund against clearing engine shortfalls (analogous to CME's Guaranty Fund), development funding for ongoing protocol work including security audits and research partnerships, and market stability reserves for backstop liquidity in extreme conditions. Treasury governance follows the progressive decentralization model, beginning under Nunchi management with transparent reporting and transitioning to on-chain governance as the validator set matures.

---

## 14. Roadmap & Current State

### What Exists Today

Roko and Korai are not speculative designs — they are systems under active development with substantial implemented components. The distinction between what is done and what is planned is stated precisely here, because credibility depends on it.

**Current status (April 2026): R0 (Agent Skeleton) is feature-complete internally. The public testnet milestone is Q3 2026. R1 development is underway.**

- **Kauri BFT consensus**: Operational. 1,389 tests pass across three suites: safety properties (fork prevention, finality guarantees), liveness properties (progress under partial failure, leader rotation), and performance benchmarks (throughput under load, latency distribution). For context, the consensus layer comprises roughly 25,000 lines of Rust; a test density of approximately one test per 18 lines reflects the correctness requirements of BFT financial infrastructure. Safety and liveness properties are formally verified against standard BFT specifications.

- **SpecPool EVM**: Speculative execution with Block-STM for parallel transaction processing. MDBX storage backend operational.

- **Cooperative clearing engine**: 37 clearing rounds verified. 100% KKT (Karush–Kuhn–Tucker) verification pass rate across all rounds. Convex program formulation and solver operational.

- **Agent runtime**: Five-layer Synapse Architecture implemented in Rust. Cognitive loop, multi-substrate memory, and gateway inference proxy functional. The runtime implements the context engineering principles validated empirically by the Meta-Harness paper ([arXiv:2603.28052](https://arxiv.org/abs/2603.28052)) — treating the harness layer as a distinct, optimizable component separate from model weights.

- **DeFi tool suite**: 193 typed tools spanning lending protocols, DEX operations, yield strategies, bridging, and governance interactions.

- **Agent Passports**: ERC-8004 compatible identity system with liveness tracking and reputation staking. ERC-8004 deployed to Ethereum mainnet January 29, 2026 ([QuickNode Developer Guide](https://blog.quicknode.com/erc-8004-a-developers-guide-to-trustless-ai-agent-identity/)); Korai's implementation extends the standard with Kauri-native attestation.

---

### Release Schedule and Dependency Graph

The releases form a directed dependency graph, not a linear sequence. Understanding which releases block others — and which can proceed in parallel — is essential for evaluating the timeline.

```
R0 ──► R1 ──► R2 ──► R3 ──► R5 ──► R6
                │
                ├──► R3 (also depends on R2)
                │
                └──► R4 (can parallel with R3)
```

**R0: Agent Skeleton — Feature-Complete (Public Testnet Q3 2026)**

Agent CLI with chain registration. Agents can register identities, interact with the chain, submit transactions, and observe network state. R0 is feature-complete internally as of April 2026. The Q3 2026 milestone is the *public* testnet release — external documentation, onboarding tooling, and the permissioned testnet environment for external developers and early agent operators. The internal-to-public gap reflects the engineering work of making a system production-ready for external participants who will find edge cases the internal team has not encountered.

*Dependency: None. Blocks: R1.*

**R1: Network Goes Live — Q4 2026**

Real trading jobs, ISFR bootstrap with the initial source set (Aave V3, Compound V3, Ethena, Lido), and cooperative clearing operational with early yield perpetual pairs. First agents begin earning through clearing participation and knowledge production. The ISFR benchmark begins its publication history — the accumulation of which is a prerequisite for institutional data licensing credibility.

This is the network's economic debut. The design priority at R1 is correctness over throughput: clearing engine circuit breakers are active, position limits are conservative, and the initial source set is deliberately narrow to ensure ISFR reflects a reliable signal before expansion.

*Depends on: R0. Blocks: R2, R3, R4.*

**R2: Dashboard and External Users — Q1 2027**

AI Studio and Agent Studio MVPs. External users — not just operators — can browse the InsightStore, view agent reputation profiles, observe ISFR publications, and interact with the system without operating infrastructure. The dashboard transforms the network from a developer artifact into an accessible platform. First non-operator users interact with the system; first external feedback on UX and discoverability.

*Depends on: R1. Blocks: R3.*

**R3: Full Economy — Q2 2027**

Subscription tiers, work markets, and module marketplace activate. The six mining surfaces open to external agent operators. Real revenue flows: clearing fees, query fees, ISFR licensing subscriptions. Clearing profiles enable one-action hedging for end users — the yield perpetual as a first-class consumer product, not just a developer primitive. This is when KORAI token economics begin operating as designed: demurrage pressure is real, work market rewards are live, and the emissions-to-revenue crossover becomes the primary metric.

*Depends on: R1, R2.*

**R4: Simulation and Testing Subnet — Q2–Q3 2027**

Large-scale agent simulation environment for stress testing, strategy validation, and collective intelligence measurement at scale. The R4 simulation subnet runs C-Factor experiments in adversarial financial environments — with regime changes, reflexivity effects, and active adversarial agents — at 10,000+ agent scale. This is the make-or-break empirical test of the architecture's core thesis. Whether C-Factor improvements hold, plateau, or exhibit unexpected dynamics in adversarial financial domains can only be answered with production-scale simulation.

Critically, R4 can proceed in parallel with R3. It does not require the full economy to be live — it requires a stable network (R1) and sufficient agent population (early R3).

*Depends on: R1. Can run in parallel with R3.*

**R5: Privacy Prototype — Q3–Q4 2027**

ZKML + TEE hybrid prototype for private agent data processing. Five-layer data architecture fully operational. This release is the prerequisite for institutional adoption — demonstrating that agent strategies can remain private while clearing and knowledge production remain publicly verifiable. The research partnership with USC on zero-knowledge machine learning targets R5. Full production ZKML remains a research frontier; R5 is a prototype that establishes the architecture and identifies the remaining gaps.

*Depends on: R3. Blocks: R6.*

**R6: Ecosystem Expansion — Q4 2027+**

Modular LLM with domain-specific modules (V2). Instance marketplace where users can acquire, fork, and customize agent configurations. External application integration through the Synapse trait system, opening Roko to use cases beyond financial agents. The module marketplace's royalty structure activates at scale: domain module creators earn ongoing revenue from usage, creating a developer ecosystem with aligned economic incentives.

*Depends on: R5.*

---

### Progressive Decentralization

Korai launches with a single sequencer. This is not a compromise — it is the industry-standard approach for new chain launches, used by Optimism, Arbitrum, Base, and virtually every production L2. Decentralized sequencing introduces latency, MEV considerations, and complexity that must be managed carefully for a financial settlement chain. Launching with a single sequencer allows the network to establish correctness and performance baselines before introducing the additional variables of decentralized ordering.

The path to full decentralization follows a proven three-phase model:

**Phase 1 — Launch**: Single sequencer with transparent execution. All state is publicly verifiable; the sequencer cannot alter outputs without detection.

**Phase 2 — BFT Committee**: Sequencer rotation within a permissioned BFT committee. Multiple parties can produce blocks; Byzantine fault tolerance provides liveness under partial failure. This phase mirrors the validator set expansion that Optimism and Arbitrum completed in their second year of operation.

**Phase 3 — Permissionless**: Fully permissionless validator set. Any operator meeting stake and hardware requirements can participate in consensus. This is the end state — the same governance structure that governs mature L1s.

The timeline for each transition depends on network maturity, validator tooling readiness, and the risk assessment of introducing additional complexity into a live financial settlement system. Governance votes at each phase boundary will be public.

---

## 15. Honest Limitations & Risk Factors

Intellectual honesty about limitations is more credible than false confidence — and more useful to the reader. What follows is a plain accounting of the genuine open challenges facing Korai's architecture, with specific mitigants and honest assessments of residual risk after mitigants are applied.

---

### What Isn't Solved Yet

**Full Data Privacy from Operators**

Agents processing sensitive data — proprietary strategies, user portfolio details, counterparty positions — need cryptographic guarantees that operators cannot observe raw data. The current approach uses TEE (Trusted Execution Environments) for agent-level data isolation, with a research path toward ZKML (Zero-Knowledge Machine Learning) + TEE hybrid for R5. Active research partnership with USC targets this milestone. Full production ZKML remains a research frontier across the industry; R5 is a prototype, not a production-grade deployment.

*Residual risk: Medium. TEE provides meaningful privacy guarantees today; the gap is cryptographic proof of privacy without hardware trust assumptions. R5 narrows but does not close this gap.*

**LLM-in-TEE at Scale**

Running full LLM inference inside TEE enclaves faces hardware constraints — Intel SGX EPC limits are approximately 512MB, while 7B+ parameter models require 3–14GB. Current workarounds include model quantization, model splitting across enclaves, and the planned modular LLM architecture that keeps only privacy-sensitive components inside TEE boundaries. Hardware roadmaps (Intel TDX, AMD SEV-SNP with expanded memory regions) are converging on this problem; the timeline for hardware resolution is outside Korai's control.

*Residual risk: Medium. The constraint is hardware, not architectural. Hardware timelines are uncertain but directionally favorable.*

**Decentralized Sequencing**

V1 operates with a single sequencer — consistent with every major L2 at launch, but not the end state. Decentralized sequencing introduces latency, MEV considerations, and Byzantine fault tolerance requirements that must be carefully managed for a financial settlement chain. The path through sequencer rotation to fully permissionless validation is planned but not implemented.

*Residual risk: Low-to-medium. The path is established (Optimism, Arbitrum, Base have demonstrated it); the risk is execution timing and MEV management specific to a financial settlement context.*

**LLM Determinism for Verification**

This is the deepest architectural challenge. LLM outputs are inherently stochastic — the same prompt can produce different outputs across runs, and outputs cannot be deterministically replayed for verification. For a financial system requiring auditability, this is a fundamental tension.

The mitigation is layered separation: deterministic components — margin calculations, ISFR computation, KKT verification, position accounting — are implemented in Rust with bit-exact reproducibility, verifiable by any validator. Stochastic components — LLM reasoning, context assembly, prediction generation — are verified through consistency checks: the PROOF_LOG precompile commits reasoning trace hashes, and validators confirm that outputs are consistent with committed inputs and declared constraints. An agent's *reasoning* cannot be replayed; the *boundaries* of its permissible actions can be verified exactly.

Formalizing this determinism boundary — precisely which components fall on which side — is a prerequisite for R3 and an active area of development.

*Residual risk: Medium-high. The architecture is sound in principle; the engineering of precise determinism boundaries is non-trivial and not yet complete.*

---

### Risk Factors

**LLM Provider Dependency**

Roko currently relies on external LLM providers for inference. Provider rate-limiting, pricing changes, or service disruptions directly affect agent operations.

*Mitigant*: LLM-agnostic Gateway inference proxy supports multiple providers with automatic fallback. Modular LLM architecture (R6) enables self-hosted inference, eliminating external dependency at scale.

*Residual risk*: Low-to-medium before R6; low after.

---

**Regulatory Enforcement**

Perpetual derivatives face material regulatory risk. The CFTC's April 2025 Request for Comment on perpetual derivatives ([CFTC Press Release 9069-25](https://www.cftc.gov/PressRoom/PressReleases/9069-25)) and the SEC/CFTC Memorandum of Understanding of March 2026 ([SEC.gov, March 11, 2026](https://www.sec.gov/newsroom/press-releases/2026-26-sec-cftc-announce-historic-memorandum-understanding-between-agencies)) create a viable regulatory path — the CFTC acknowledged on-chain perpetuals as a legitimate regulatory category, and the MOU explicitly reduces jurisdictional ambiguity. But enforcement actions against DeFi protocols remain possible regardless of regulatory trajectory. The Ooki DAO precedent (CFTC v. Ooki DAO, 2023) established that decentralization does not confer regulatory immunity.

*Mitigant*: Jurisdictional strategy using Bermuda (DABA/DAIA), Dubai (VARA), or Cayman frameworks for operational licensing. Frontend geo-blocking for restricted jurisdictions — acknowledged as imperfect but meaningful for liability management. Estimated annual compliance costs of $500K–2M. Progressive decentralization of protocol governance reduces single-entity enforcement surface.

*Residual risk*: Medium. The regulatory environment is improving but not resolved. Compliance costs are manageable; enforcement risk diminishes with protocol maturity and decentralization.

---

**TEE Hardware Trust**

TEE security relies on hardware manufacturer integrity. Historical attacks — SGAxe, Plundervolt, AEPIC Leak — have demonstrated that TEE guarantees are not absolute.

*Mitigant*: TEE is used for agent-level privacy, not chain-level consensus or clearing. Chain operations are verified through cryptographic proofs (KKT certificates), not hardware attestation. The privacy roadmap progressively replaces TEE-only guarantees with cryptographic guarantees: threshold encryption and ZK proofs that do not require hardware trust.

*Residual risk*: Low for chain security (not TEE-dependent); medium for agent data privacy (TEE-dependent until R5+).

---

**Insufficient ISFR Liquidity**

ISFR's credibility depends on sufficient underlying market depth. If DeFi lending TVL contracts significantly or source venues lose volume, ISFR's reliability degrades. Aave V3's $33.71B TVL as of April 2026 ([DefiLlama](https://defillama.com/protocol/aave-v3)) and persistent institutional adoption — BlackRock BUIDL listed on Uniswap in February 2026 ([Quasa, February 2026](https://quasa.io/media/blackrock-enters-defi-world-s-largest-asset-manager-lists-2-2b-tokenized-treasury-fund-buidl-on-uniswap)) — provide reasonable confidence in underlying market depth, but the risk is structural, not eliminable.

*Mitigant*: The liveness state system (Live → Degraded → Stale → Halted) provides graceful degradation. V2 source expansion diversifies dependency across lending protocols, staking platforms, and structured yield products.

*Residual risk*: Low-to-medium. Underlying market depth has proven more persistent than early-cycle DeFi; structural risk is real but manageable through diversification.

---

**C-Factor Non-Transfer Across Domains** *(Highest-Impact Risk)*

This is the highest-impact risk in the system and the one most deserving of direct, honest engagement.

The collective intelligence hypothesis is grounded in evidence: Woolley et al.'s 2010 Science paper ([doi:10.1126/science.1193147](https://www.science.org/doi/10.1126/science.1193147)) established a general collective intelligence factor in human groups; the Darwin Gödel Machine demonstrated 20% → 50% improvement on SWE-bench through self-modification ([Sakana AI, May 2025](https://sakana.ai/dgm/)); HyperAgents at ICLR 2026 showed cross-domain transfer of improvement strategies ([arXiv:2603.19461](https://arxiv.org/abs/2603.19461)). But whether these results transfer to *financial domains* is an open empirical question — and there are specific structural reasons why transfer is non-trivial.

Financial domains differ from coding domains in three ways that matter for collective intelligence:

*Adversarial*: Financial counterparties actively exploit predictable behavior in ways that coding benchmarks do not. A multi-agent system that produces consistent predictions creates a detectable pattern that adversarial market participants can trade against. A coding agent that solves GitHub issues in a predictable sequence faces no counterparty trying to make those solutions fail; a yield perpetual agent does.

*Non-stationary*: Financial data distributions shift as market regimes change — rate cycles, liquidity crises, regulatory changes, protocol failures. Models trained on one regime will systematically misfired in another. SWE-bench issues are drawn from stable codebases; yield curves and lending spreads are drawn from markets that fundamentally reprice. An agent collective that achieves excellent calibration in one macro regime may exhibit sudden, correlated errors when the regime shifts.

*Reflexive*: Agent predictions influence the outcomes they predict. A collective of agents predicting that a lending protocol's yield will decline — and acting on that prediction by withdrawing liquidity — contributes to the decline they predicted. Coding agents predicting that a bug exists cannot cause the bug to appear. This reflexivity creates feedback loops absent in benchmark settings, and those feedback loops can amplify collective errors rather than cancel them.

A collective intelligence system that achieves excellent results on SWE-bench might, in financial domains, produce correlated predictions that are systematically exploited by adversarial participants, that are systematically wrong during regime transitions, and that amplify the market movements they are designed to hedge.

*Mitigant*: R4 (Simulation and Testing Subnet) is designed specifically to test C-Factor validity in adversarial financial environments, with regime changes, reflexivity effects, and adversarial agents active. If C-Factor improvements do not hold in R4 conditions, the architecture's core thesis requires revision. The simulation environment is designed to surface this failure mode before it occurs in production.

*Residual risk*: High. This is the central empirical bet of the architecture. The research evidence for the mechanism is strong; the domain-transfer evidence does not yet exist. R4 will produce it.

---

**METR Benchmark Translation Concern**

A March 2026 finding from METR ([METR blog, March 10, 2026](https://metr.org/notes/2026-03-10-many-swe-bench-passing-prs-would-not-be-merged-into-main/)) found that approximately half of SWE-bench passing PRs would not be merged by real code maintainers, with maintainer merge rates approximately 24 percentage points lower than automated grader scores. This raises a general concern about benchmark-to-production translation that applies to any system whose development process relies on SWE-bench-style automated evaluation.

*Mitigant*: Korai's performance evaluation is grounded in financial outcomes — actual clearing efficiency, actual ISFR accuracy, actual prediction calibration against realized yields — not coding benchmark scores. The METR finding applies most directly to agent systems evaluated primarily through benchmark proxies. Korai's evaluation regime uses direct outcome measurement: did the clearing solution generate surplus? Did the ISFR prediction match realized rates? This does not eliminate evaluation gaming but substantially reduces the gap between benchmark performance and production utility.

*Residual risk*: Low-to-medium. Relevant as a general caution about evaluation design; less directly applicable than it would be to a coding-agent-first architecture.

---

**Composability Risk**

Korai's architecture has deep interdependencies. The clearing engine depends on ISFR; ISFR depends on validator consensus; knowledge quality depends on reputation; reputation depends on clearing outcomes. A bug or unexpected behavior at any integration point can cascade: an ISFR anomaly degrades clearing inputs; degraded clearing outputs corrupt performance records; corrupted performance records misroute work market allocation; misallocated work degrades knowledge quality; reduced knowledge quality propagates back into agent decision-making.

*Mitigant*: The dual-plane architecture isolates critical financial operations (Kernel) from permissionless execution (EVM). The clearing engine includes circuit breakers that pause operations when internal consistency checks fail. Formal verification covers the consensus layer. Integration testing spans cross-component behavior at each release boundary.

*Residual risk*: Medium. Deep interdependencies are inherent to the design's value proposition — the same coupling that makes the system coherent makes it susceptible to cascading failures. Circuit breakers and isolation layers reduce but do not eliminate this risk.

---

**Liquidity Fragmentation**

If yield perpetuals fail to concentrate sufficient liquidity — due to competing instruments, insufficient market-making, or inadequate agent participation — the product becomes unusable for institutional hedging. Thin markets produce wide spreads, unreliable mark prices, and liquidation risk driven by illiquidity rather than insolvency. The perpetual DEX market has demonstrated this risk concretely: Hyperliquid's share of DEX perp volume peaked at approximately 80% in mid-2025 before declining to ~34% by March 2026 as competitors entered ([BlockEden.xyz, March 2026](https://blockeden.xyz/blog/2026/03/07/dex-perpetuals-market-share-growth/)), suggesting that initial liquidity concentration does not guarantee sustained depth.

*Mitigant*: Korai's perpetual design avoids maturity-based fragmentation (unlike Pendle's Principal Token structure). Cooperative clearing's batch netting reduces the liquidity required for each individual settlement compared to continuous order book matching. The ISFR benchmark itself functions as a liquidity coordination mechanism — a credible benchmark rate reduces the cost of providing market-making quotes.

*Residual risk*: Medium. Liquidity is a network effect: it builds slowly and concentrates once established, but is fragile before critical mass is reached. The first 12 months of clearing operation are the highest-risk period for liquidity fragmentation.

---

**Incumbent Competition**

If established DeFi protocols — Aave, Pendle, Hyperliquid — or TradFi infrastructure providers add on-chain interest rate derivatives, they bring existing liquidity, user bases, and institutional relationships that a new network cannot match at launch. Hyperliquid has already expanded into non-crypto perpetuals including gold, silver, and oil ([BlockEden.xyz, January 2026](https://blockeden.xyz/blog/2026/01/10/hyperliquid-revenue-dominance-onchain-trading-solana/)). Aave V3's $33.71B TVL ([DefiLlama, April 2026](https://defillama.com/protocol/aave-v3)) represents a liquidity base that could anchor an adjacent derivatives product without requiring network-level build-out.

*Mitigant*: The structural argument is architectural differentiation. The combination of persistent agent cognition, distributed knowledge infrastructure, and native financial primitives requires capabilities that existing platforms are not positioned to add incrementally. An incumbent adding interest rate derivatives does not gain collective intelligence infrastructure. An incumbent adding agent orchestration does not gain a cooperative clearing engine. These combinations require architectural decisions made at network design time, not features that can be bolted onto existing systems. But this is a competitive argument, not a guarantee — incumbents can acquire rather than build.

*Residual risk*: Medium-high in the long run. The current window of differentiated capability is real but time-bounded. Competitive moats must deepen at a rate faster than incumbents can catch up.

---

*These limitations are stated plainly because solving them is the roadmap. A litepaper that claims no limitations has not thought hard enough.*

---

## 16. Why Now

### The 12–18 Month Window

The opportunity described in this paper is not the product of imagination — it is the convergence of five independent forces that have simultaneously reached the maturity threshold required to build at their intersection. Understanding the timing requires understanding why neither 18 months ago nor 18 months from now constitutes the same opportunity.

Eighteen months ago: the systematic empirical evidence for context engineering superiority did not yet exist. DeFi lending TVL was volatile and concentrated. Regulatory signals were adversarial. The foundational research — HyperAgents, Darwin Gödel Machine, the stigmergy papers — had not been published. The claim that agent scaffolding matters more than model quality was a product intuition, not a validated empirical thesis. Building on unvalidated intuitions in a hostile regulatory environment is speculation; building on validated empirical foundations in an emerging regulatory framework is strategy.

Eighteen months from now: regulatory windows will have crystallized into specific frameworks that favor incumbents who moved first. The fourth quadrant — persistent agent cognition combined with distributed knowledge infrastructure and native financial primitives — will have visible competitors. Context engineering will have become consensus wisdom, diffused across the industry. The five forces converging now will still exist, but their intersection will be crowded.

The window is approximately 12–18 months. Long enough to build. Short enough that delay is genuinely costly.

---

### Five Converging Forces

**Force 1: Context Engineering Is Empirically Validated**

Two years ago, the claim that scaffolding matters more than model quality was contrarian. Today it is empirical fact, validated across multiple independent research threads.

SWE-bench Pro data is the clearest signal: three agents running the identical Claude Opus 4.5 model scored 51.8% (Auggie), 50.2% (Cursor), and 55.4% (Claude Code) respectively, while Scale AI's standardized scaffold with the same model scored only 45.9% — a 9.5-point spread attributable to scaffold choice alone ([Particula Tech, March 2026](https://particula.tech/blog/agent-scaffolding-beats-model-upgrades-swe-bench)). A cheaper model with a custom scaffold scored 52.7% on SWE-bench Pro, beating Claude Opus 4.5 on Anthropic's own scaffold at 52.0%. Princeton's HAL Leaderboard showed that the same model with different prompts and tools can vary by up to 9× in cost while achieving nearly identical accuracy ([HAL Leaderboard, hal.cs.princeton.edu](https://hal.cs.princeton.edu)).

The Meta-Harness paper ([arXiv:2603.28052](https://arxiv.org/abs/2603.28052), March 30, 2026) formalized this finding: Meta-Harness, an outer-loop system that searches over harness code rather than model weights, improved over a state-of-the-art context management system by 7.7 points while using 4× fewer context tokens. A single discovered harness improved accuracy by 4.7 points on average across five held-out models on retrieval-augmented math reasoning — demonstrating cross-model transfer. The harness layer is not auxiliary; it is where competitive differentiation now lives.

Important epistemic calibration: METR's March 2026 finding that ~half of SWE-bench passing PRs would not be merged by real maintainers ([METR, March 10, 2026](https://metr.org/notes/2026-03-10-many-swe-bench-passing-prs-would-not-be-merged-into-main/)) is a useful reminder that benchmark performance is an imperfect proxy for production capability. The SWE-bench evidence validates the mechanism of scaffolding superiority; it does not validate specific benchmark scores as predictors of financial domain performance. Whether context engineering advantages transfer from coding to adversarial financial domains is the central hypothesis that Korai's architecture is designed to test.

**Force 2: DeFi Yield Markets Have Reached Institutional Scale**

The infrastructure for on-chain interest rate derivatives requires underlying markets of sufficient depth, diversity, and persistence. As recently as 2023, DeFi lending TVL was too volatile and too concentrated to support a credible benchmark rate. Today, three structural thresholds have been crossed.

*Depth*: Aave V3 holds $33.71B in total TVL as of April 2026, with $30.10B on Ethereum alone ([DefiLlama, April 2026](https://defillama.com/protocol/aave-v3)). Lido Finance holds ~$21.29B in liquid staking TVL, with 37–38 million ETH staked — approximately 30.5% of total ETH supply, an all-time high ([Investing.com, March 2026](https://www.investing.com/analysis/ethereum-staking-at-30-of-supply-tightens-available-market-float-200676807)). Total DeFi TVL is approximately $96B ([DefiLlama, Q1 2026](https://defillama.com)). These are market depths capable of supporting a credible benchmark rate without susceptibility to a single protocol's disruption.

*Persistence*: Aave V3 has maintained $20B+ TVL for over 18 months, demonstrating structural durability rather than a one-cycle spike. ETH staking yield at 3.5–4.2% APY provides a persistent, mechanically deterministic rate source for ISFR ([Chainlabo, February 2026](https://www.chainlabo.com/blog/ethereum-staking-rate-30-percent-2026-security-settlement-layer)).

*Institutional presence*: The tokenized real-world asset market reached $27.65 billion in April 2026 ([Phemex News, April 2026](https://phemex.com/news/article/tokenized-asset-market-reaches-2765-billion-amid-crypto-slowdown-70812)), with $12.78 billion in tokenized U.S. Treasuries. BlackRock listed its BUIDL fund (formerly ~$2.5B AUM) on Uniswap in February 2026 ([Quasa, February 2026](https://quasa.io/media/blackrock-enters-defi-world-s-largest-asset-manager-lists-2-2b-tokenized-treasury-fund-buidl-on-uniswap)). Franklin Templeton's FOBXX reached $843.74M in AUM as of March 31, 2026, now distributed across Avalanche, Stellar, Polygon, and other chains ([Franklin Templeton Fund Page](https://www.franklintempleton.com/investments/options/money-market-funds/products/29386/SINGLCLASS/franklin-on-chain-u-s-government-money-fund/FOBXX)). These are not DeFi-native protocols hedging DeFi-native risk — these are the world's largest asset managers treating on-chain yield infrastructure as production-grade.

This institutional presence makes ISFR viable: when BlackRock and Franklin Templeton hold yield-bearing on-chain assets, the demand for interest rate hedging instruments in DeFi is no longer hypothetical.

**Force 3: Regulatory Clarity Is Emerging — At Exactly the Right Moment**

The regulatory environment for on-chain derivatives has shifted from adversarial to constructive within the last 12 months — not gradually, but through a series of discrete inflection points.

On April 21, 2025, the CFTC issued a formal Request for Comment on perpetual derivatives ([CFTC Press Release 9069-25](https://www.cftc.gov/PressRoom/PressReleases/9069-25)), simultaneously seeking comment on 24/7 derivatives trading and clearing. This was the first formal regulatory acknowledgment of on-chain perpetuals as a legitimate product category. Days after the RFC, a CFTC-registered exchange self-certified the first-ever perpetual futures contracts listed on a US exchange — regulatory signal and market action converging simultaneously.

On March 11, 2026, the SEC and CFTC signed a landmark Memorandum of Understanding to harmonize regulatory oversight of crypto assets ([SEC.gov, March 11, 2026](https://www.sec.gov/newsroom/press-releases/2026-26-sec-cftc-announce-historic-memorandum-understanding-between-agencies)). The MOU explicitly commits to joint rulemakings, reduced jurisdictional ambiguity for dually registered venues, and a "fit-for-purpose framework" for digital assets. Crucially, the MOU explicitly supports a unified financial platform vision — offering securities, derivatives, crypto, and banking services under a simplified framework. Korai's architecture anticipates exactly this market structure.

The Bermuda DABA/DAIA framework and Dubai VARA exchange derivatives licensing provide operational jurisdictions with clear requirements for the period before US domestic clarity fully resolves. Together, these developments create a compliance path that did not exist 18 months ago.

The timing matters: regulatory windows open before they close. The early movers in the post-CFTC-RFC window — those who can demonstrate compliance architecture during the consultation period — will have structural advantages when the framework finalizes.

**Force 4: Agent Infrastructure Is the Defining Investment Category of This Era**

Global private AI company funding reached a record $225.8 billion in 2025, nearly double 2024's total ([CB Insights State of AI 2025](https://www.cbinsights.com/research/report/ai-trends-2025/), January 2026). AI captured approximately 50% of all global startup funding in 2025, up from 34% in 2024 ([Crunchbase, December 2025](https://news.crunchbase.com/ai/big-funding-trends-charts-eoy-2025/)). Within AI, the agent infrastructure segment is attracting the most concentrated investment: LangChain reached a $1.25 billion valuation on $125M in Series B funding in October 2025 ([TechCrunch, October 21, 2025](https://techcrunch.com/2025/10/21/open-source-agentic-startup-langchain-hits-1-25b-valuation/)). Cognition AI (Devin) closed a ~$400M round at a $10.2 billion valuation in September 2025 ([TechCrunch, September 8, 2025](https://techcrunch.com/2025/09/08/cognition-ai-defies-turbulence-with-a-400m-raise-at-10-2b-valuation/)). The agent infrastructure market is projected to grow from $7.84B in 2025 to $52.62B by 2030 at a 46.3% CAGR, with the multi-agent systems segment growing at 48.5% ([MarketsandMarkets, 2025](https://www.marketsandmarkets.com/Market-Reports/ai-agents-market-15761548.html)).

None of the funded companies in this category combine persistent agent cognition with blockchain coordination infrastructure or native financial primitives. The capital flowing into adjacent quadrants — orchestration frameworks, coding agents, tool integration platforms — validates the category's scale without anyone having built the specific combination that Korai addresses. Market validation of the general category without occupancy of the specific niche is precisely the signal that identifies an unaddressed opportunity.

**Force 5: Collective Intelligence Research Validates the Architecture**

The theoretical foundations of Korai's architecture have reached an empirical validation threshold within the last 12 months, across multiple independent research threads.

Meta's HyperAgents ([arXiv:2603.19461](https://arxiv.org/abs/2603.19461), ICLR 2026, March 19, 2026) demonstrated that agent systems can recursively improve their own improvement procedures — metacognitive self-modification that generalizes across domains. A DGM-H agent trained only on paper review and robotics domains achieved imp@50 = 0.630 on Olympiad math grading without any domain-specific training; hand-designed systems built specifically for that task scored 0.0. Cross-domain transfer of learned improvement strategies is now demonstrated, not hypothetical.

Sakana AI's Darwin Gödel Machine ([arXiv:2505.22954](https://arxiv.org/html/2505.22954v3)) improved from 20% to 50% on SWE-bench through autonomous code self-modification in 80 iterations — demonstrating that self-improving agent systems are practical engineering, not theoretical curiosity ([Sakana AI, May 2025](https://sakana.ai/dgm/)).

The stigmergy paper ([arXiv:2512.10166](https://arxiv.org/html/2512.10166v1), December 2025) formalized a mechanism directly relevant to Korai's InsightStore: at a critical agent density ρ_c ≈ 0.23, stigmergic environmental traces dominate internal memory by 36–41% on composite performance metrics in a 625-agent simulation. This is the theoretical mechanism by which a shared knowledge substrate produces collective intelligence that exceeds the sum of individual agent capabilities.

These research results have appeared in the last 12 months. The science is catching up with the architecture — rapidly enough that the next 12 months will see the research frontier move from validation to application.

---

### The Window Is Open Now

Five forces — validated context engineering, mature and institutionally-adopted DeFi yield markets, the first constructive US regulatory framework for on-chain derivatives, record-scale investment in agent infrastructure with the specific niche unoccupied, and empirical validation of the collective intelligence mechanisms the architecture depends on — are converging for the first time.

They were not simultaneously present 18 months ago. They will not remain in their current configuration indefinitely: regulatory windows crystallize and close, competitive gaps narrow as capital concentrates, research insights diffuse across the industry and lose their alpha.

The architecture described in this paper exists precisely at the intersection of these forces. The implementation is underway. The empirical tests — particularly R4's C-Factor validation in adversarial financial environments — are designed to answer the questions that cannot be resolved by argument alone.

The window is open now. Building begins.

---

*Sources cited inline throughout. All figures from primary sources where available; research compilation dated April 2026.*
