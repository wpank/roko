# Research Prompt 2: Deep-Dive — ISFR Benchmark Business, Narrow Index Strategy, and Agent-Benchmark Compounding

## Context

This is a follow-up research run. The first round (research1.md) established:

- **ISFR-YBS (Yield-Bearing Stablecoin Reference Rate)** as the recommended narrow wedge product — $50B+ addressable supply, no regulated administrator covers it, cleanest narrative for VCs and regulators
- **UK BMR Cat-6 authorization** as the regulatory path (same as CF Benchmarks in 2019-2020), with $1.5-3M Year 1-2 spend
- **Five named competitors**: IPOR/Fusion ($14M TVL, pivoted to vaults), Treehouse/TESR ($157M TVL, TREE 94% below ATH), CF Benchmarks (FCA-regulated, off-chain only, Kraken-owned), CESR/CoinDesk Indices (ETH staking), Silicon Data/SemiAnalysis (AI compute — deferred)
- **Agent-attested data sources** as the compound thesis — ERC-8004 + x402 + HDC precompile makes Korai the canonical L1 for regulated benchmark operation
- **10 Phase-1 partnership targets** named (Aave, CF Benchmarks, Lido, Pendle, CME, ISDA, Imperial IC3RE, Treehouse, Hyperliquid/Ethena, Bloomberg BISL)

The attached synthesis documents (01-06) contain the full context. Read them all before starting.

---

## What I need you to research now

### A. ISFR-YBS Deep Product Design

The first research round identified ISFR-YBS as the lead product but left the methodology at sketch level. I need a full product spec investigation:

1. **Yield-bearing stablecoin market map (May 2026)** — Complete the landscape. For every yield-bearing stablecoin with >$100M supply, document: issuer, current supply, yield mechanism (RWA/lending/basis/savings), current APY, audit status, redemption terms, chain deployments. Include at minimum: sUSDe, sUSDS, syrupUSDC, USDY, OUSG, USD0/USD0++, aUSDC, aUSDT, USDtb, USDM, sFRAX, AUSD. What am I missing?

2. **Methodology design** — How should the four sub-indices (YBS-T, YBS-L, YBS-D, YBS-S) actually be computed? Research how SOFR handles the weighted median, how VIX handles the model-free replication, how CESR/TESR handles validator consensus. What's the optimal: volume-weighted median, TVL-weighted mean, supply-weighted composite? What filtering (outlier exclusion, staleness, flash-loan protection) is standard?

3. **Data sourcing reality check** — For each yield-bearing stablecoin, how exactly do you read the current yield on-chain? Is it a view function? An event? Does it require off-chain computation? Which are trivially readable (like Aave's `getReserveData()`) and which require complex derivation? This determines the trust budget.

4. **ISFR-YBS vs. existing yield aggregators** — How does this differ from DeFiLlama Yields, Coingecko yield pages, DefiRate, Yearn dashboards, Veda's strategy selection? What makes it a "benchmark" rather than a "data feed"? Research the precise legal/regulatory distinction under UK BMR.

5. **The $50B+ supply claim** — Validate or refine this number. What's the actual total supply of yield-bearing stablecoins as of May 2026? What's the growth trajectory? What's the realistic Phase-1 addressable supply (only those meeting inclusion criteria)?

### B. Regulatory and Governance Deep-Dive

6. **UK BMR Cat-6 application process** — Step-by-step: What forms are filed? What's the FCA's Connect portal process? What supporting documents are required? What's the actual timeline (not aspirational) based on CF Benchmarks, Argus, and other recent applicants? Are there any publicly available FCA determination letters or supervisory statements that reveal what the FCA actually looks for?

7. **IOSCO Principles gap analysis** — For each of the 19 principles, what specific policies/documents does an administrator need? Research what CF Benchmarks and MSCI have published publicly (adherence statements, methodology papers, governance frameworks). Can we find their actual published documents as templates?

8. **ISAE 3000 assurance** — What exactly does a Big-4 ISAE 3000 examination cover for a benchmark administrator? How long does it take? What's the cost range? Research KPMG's published work for CF Benchmarks and PwC's for Argus — any public reports?

9. **ISDA Settlement Price Source Matrix** — How does a new benchmark get added? What's the application process? Who decides? How long does it take? What precedents exist for crypto-native rates being added?

10. **EU BMR third-country recognition** — The transition period extends to 31 Dec 2030. What's the actual ESMA recognition process? What did CME/CBA do in April 2026? What documentation is required? Can a UK-authorized administrator serve EU users before ESMA recognition via the extended transition?

### C. Partnership Strategy Deep-Dive

11. **Pendle as Phase-1 licensee** — Research Pendle's current state in detail. What's their TVL, volume, token price? What's Boros (their funding rate market)? How do their PT/YT markets work with reference rates? What would a Pendle integration look like technically? Who to contact (TN is the co-founder — any public contact info)?

12. **CF Benchmarks partnership vs. competition** — Research CF Benchmarks in detail. What rates do they currently publish (BIRC, KFRI, AUIRR, CESR)? What's their methodology for each? Is co-branding realistic or would they view ISFR as competitive? What did the Crypto Facilities → Kraken acquisition look like? What's Sui Chung's public stance on DeFi-native rates?

13. **Treehouse partnership analysis** — Research Treehouse's current state. What happened after TGE? What's TREE trading at? What's their institutional pipeline (FalconX FRAs, Edge Capital, Mirana, Monarq)? How does TESR methodology actually work (panelist submissions, consensus mechanism)? What are the LIBOR-collusion criticisms leveled at panel-based rates?

14. **Aave Labs / DAO engagement** — Research the current state of Aave governance. What happened with BGD Labs and ACI wind-down? Who are the successor DAO service providers? What's the right path to get Aave as a data attestor? Any precedent for Aave providing data to third-party index providers?

15. **Academic partnerships** — Research Maurice Herlihy at Brown (cooperative clearing, cross-chain atomic commits). What's his publication record relevant to ISFR? Has he served on any financial oversight committees? Research Will Knottenbelt at Imperial IC3RE — what's his work on crypto benchmarks? Is there a published model for sponsored research partnerships with universities for benchmark governance?

### D. Competitive Intelligence

16. **IPOR → Fusion pivot deep-dive** — What exactly happened? When did the rebrand occur? What's the Fusion DAO structure? What's the IPOR→FUSN token swap? What's their current TVL, volume, and revenue? Is the benchmark product abandoned or dormant? Any signal they'd partner or sell?

17. **CESR and CoinDesk Indices** — Research the Composite Ether Staking Rate. Who administers it (CoinDesk Indices)? What's the methodology? How is it regulated? Who uses it? How would ISFR position against CESR for the staking lane?

18. **Silicon Data H100/A100 index** — Research this Bloomberg-distributed daily GPU compute index. Who's behind it? What's the methodology? How is it funded (DRW and Jump Trading backing)? What would it take to build an AI-compute variant on Korai? Is this a real competitor or potential partner for Phase 3?

19. **Block Scholes, Amberdata, Kaiko, Coin Metrics** — Research these as potential data distribution partners. What data products do they offer? What's their pricing? How do they work with benchmark administrators? Which would be the best fit for ISFR distribution?

20. **The Kelp/rsETH exploit implications** — Research the April 18, 2026 exploit in detail. $292M drained, $236M cascaded bad debt across Aave/Compound/Euler. What happened? How did it affect LRT inclusion criteria discussions? What methodology safeguards would prevent ISFR from being affected by a similar constituent failure?

### E. Revenue Model and Business Case

21. **Index licensing economics** — Research the actual economics of benchmark licensing in detail. What does State Street pay S&P for SPY? What does BlackRock pay MSCI for EEM? What does CME pay for Term SOFR licensing? What are the standard bps ranges for different product types (ETFs, futures, OTC derivatives, structured products)?

22. **Crypto benchmark revenue precedents** — Has any crypto-native benchmark generated meaningful licensing revenue? What's CF Benchmarks' estimated revenue? What does Kaiko charge for institutional data? What does Coin Metrics charge? Are there any public numbers?

23. **Business case for $20B referenced notional** — Is the Phase-3 target of $20B in referenced notional realistic? What's the current total notional of crypto rate derivatives? What's the growth rate? What comparable markets grew from zero to $20B and how long did it take?

24. **The "no token at admin layer" strategy** — Research how CF Benchmarks, Bloomberg BISL, and other regulated administrators structure their corporate entities. Confirm that token-based revenue at the benchmark entity level is structurally incompatible with institutional credibility. What's the right corporate structure?

### F. Agent-Benchmark Integration Specifics

25. **ERC-8004 current status** — What's the actual state of ERC-8004 in May 2026? How many agents are registered? Which chains? Who's implementing it? Is MetaMask still driving it? What's the roadmap?

26. **x402 current status** — What's the actual state of x402? The research1 doc claims 900K+ weekly settlements and Cloudflare/Visa/Google integrations. Verify these numbers. What's the latest from the x402 Foundation? What's the technical spec?

27. **DSPy 3.0 + GEPA** — Research the current state. The paper claims +10% over MIPROv2 on AIME-2025. What's the production readiness? Who's using it? How would it integrate with Roko's system prompt builder? Is it actually suitable for benchmark methodology pipelines?

28. **Inspect AI** — Research UK AISI's open-source eval framework. 200+ pre-built evals? Who's adopted it? How suitable is it for evaluating agent-attested data quality? What's the state of regulator adoption?

29. **Darwin Gödel Machine** — Research the current state. SWE-bench 20.0% → 50.0% is a remarkable claim. What's the actual methodology? Is it reproducible? What would DGM-style self-improvement look like applied to benchmark methodology evolution?

### G. What to Build and Prove (Concrete Artifacts)

For each, I need: scope, timeline, expected cost, and what it proves to investors:

30. **ISFR-YBS live data feed** — A public dashboard showing real-time yield-bearing stablecoin rates aggregated across sources. What's the minimum viable version? How long to build? What data sources are freely available vs. requiring partnerships?

31. **Methodology paper** — A publishable ISFR-YBS methodology whitepaper modeled on SOFR's published methodology. What sections does it need? What's the standard structure? How long is a typical benchmark methodology paper?

32. **Agent-attested data demo** — A working demonstration of a Roko agent fetching yield data, attesting it with ERC-8004, paying via x402, and posting to Korai. What's the minimum viable version?

33. **Backtesting study** — A historical ISFR-YBS calculation over the past 12-24 months, showing how the index would have performed. What data sources are available for backtesting? How do other benchmark administrators publish backtested histories?

34. **Competitive comparison dashboard** — A side-by-side comparison of ISFR-YBS vs. IPOR vs. TESR vs. CF Benchmarks rates, showing the gap that ISFR fills. What would this look like?

---

## Source Documents

Read ALL of these before starting research:

### Synthesis documents (in `synthesis/` folder):
- `01-isfr-benchmark-business-strategy.md` — Complete benchmark business strategy: ISFR-YBS wedge, governance, regulatory path, competitive landscape, revenue model, partnership targets, risk register
- `02-agent-benchmark-synergy.md` — How agent runtime compounds with benchmark business: 6 synergy mechanisms, key research papers, implementation priorities
- `03-architecture-paradigms.md` — v2 architecture: Signals/Cells/Graphs, 4 universal patterns, agent cognitive architecture, memory/learning, execution model
- `04-research-paradigms-competitive.md` — Competitive positioning: category naming, 5 key paradigms, moat ranking, market timing, opportunity layers
- `05-marketplace-payments-defi.md` — Marketplace, payments, DeFi: agent hiring models, x402/MPP payments, registries, arenas, yield perps, revenue capture
- `06-security-observability-deployment.md` — Security model, TEE integration, telemetry, deployment modes, audit trail, compliance

### Foundation documents (in parent `index-spec/` folder):
- `00` through `13` — The 14 self-contained reference docs from the previous research round (ISFR, yield perps, Korai blockchain, generalized benchmarks, TEE clearing, Roko runtime, oracle system, agent identity, knowledge engineering, DeFi integration, differentiation PRD, DKG use cases, Daeji/Commonware chain)

---

## Output format

Structure your research as:

1. **Executive Summary** (1 page) — Top 5 findings that change the strategy or open new opportunities
2. **Section A: ISFR-YBS Product Spec** — Full market map, methodology recommendation, data sourcing analysis
3. **Section B: Regulatory Playbook** — Step-by-step guide with real timelines, costs, and templates
4. **Section C: Partnership Intel** — For each target: current state, contact path, deal structure, risk
5. **Section D: Competitive Map** — Updated landscape with new intelligence
6. **Section E: Business Case** — Revenue projections grounded in real comparables
7. **Section F: Agent Integration** — Current state of each standard/tool, integration readiness
8. **Section G: Build Priorities** — Ranked list of artifacts with scope, cost, timeline, and investor impact

For each finding, flag:
- **Confirms** — validates something from research1
- **Updates** — refines or corrects something from research1
- **New** — net-new intelligence not in any prior document
- **Risk** — something that threatens the strategy

Be specific. Name companies, cite numbers, link to primary sources. Every claim should be traceable to a source. If you can't verify a claim from research1, flag it explicitly.
