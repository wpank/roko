# 03 — ISFR (Index, Yield Perpetuals, and the Generalised Benchmark Framework)

> This folder owns ISFR — the **Implied Secured Funding Rate** index — together with the yield-perpetual product built on it, the generalised benchmark framework that ISFR is the lead instance of, and the benchmark-business and regulatory thesis. It is written for a first-time reader with no prior context.

---

## CRITICAL — Acronym Disambiguation

There are two different things called "ISFR" inside the Nunchi codebase. They share spelling and nothing else.

| Term | Full name | Owner | Where you find it |
|------|-----------|-------|-------------------|
| **ISFR** (the index) | Implied Secured Funding Rate | This folder | Oracle precompile, yield-perp contracts, the off-chain index service, validator computation |
| **ISFR_score** (the metric) | Internal Solvency & Funding Ratio | The chain folder (TEE clearing engine) | Per-agent scoreboard, risk manager, clearing engine internals |

**Throughout this folder, "ISFR" refers exclusively to the index.** When the internal scoreboard metric appears for cross-reference, it is always written as the full name *Internal Solvency & Funding Ratio* or `ISFR_score` so the reader cannot confuse the two.

A third historical expansion — *Intersubjective Fact Registry* — appears in a few Rust module-level doc comments for the on-chain ISFR registry. It refers to the same on-chain object as the index and is best read as legacy nomenclature for the agent-attestation surface, not a separate concept.

### Rule of thumb when reading code

| Context | Meaning |
|---------|---------|
| Oracle, precompile, validator vote, basis points, mark price | Index — *Implied Secured Funding Rate* |
| Yield perpetual contract, settlement rate, hedge | Index |
| Off-chain `isfr-service`, scrapers, Aave/Compound/Ethena/staking inputs | Index |
| Hierarchical market ID, weighted-median submission, agent rate observation | Index (via the on-chain registry, sometimes called *Intersubjective Fact Registry*) |
| Scoreboard, risk manager, per-agent solvency check, equity / margin ratio | Scoreboard metric — *Internal Solvency & Funding Ratio* (`ISFR_score`) |
| TEE clearing engine, parent process, parent/child architecture | Scoreboard metric |

If the file path or surrounding identifiers reference oracles, rates, benchmarks, or yield perpetuals, it is the index. If they reference scoreboards, risk managers, or per-agent health metrics inside the clearing engine, it is the solvency ratio.

---

## What ISFR Is in One Paragraph

ISFR is a composite benchmark index measuring the cost of secured funding across decentralised finance. It aggregates yield signals from collateralised lending (Aave V3, Compound V3), structured yield (Ethena sUSDe), perpetual funding (Hyperliquid ETH perp), and proof-of-stake validator yield (ETH Beacon Chain) into a single rate, computed by Nunchi blockchain validators every ~10 seconds and published on-chain via a dedicated precompile at address `0xA01`. ISFR is to DeFi what SOFR (Secured Overnight Financing Rate) is to traditional finance: the reference rate that derivative instruments — interest-rate swaps, perpetual futures, floating-rate notes — settle against. SOFR underpins ~$570T+ in instrument notional. ISFR is designed to occupy the same position in DeFi, where on-chain rate-derivative TVL is currently under $100M against $49.5B of unhedged DeFi lending exposure.

---

## Names

| Name | Meaning |
|------|---------|
| **Nunchi** | The brand. |
| **Nunchi blockchain** | The chain itself. |
| **Daeji** | The testnet name for the chain. |
| **Roko** | The agent runtime. |
| **agentchain** | Umbrella term spanning the chain, the runtime, and the benchmark framework. |
| **ISFR** (this folder) | The Implied Secured Funding Rate index. |
| **ISFR_score** (chain folder) | Internal Solvency & Funding Ratio — per-agent metric in the clearing engine. Never abbreviated to "ISFR" in this folder. |
| **NRIS** | Nunchi Reference Index Suite — the umbrella for ISFR plus the four sister indices (IAPI, IKQI, ISVI, IRRI). |

For historical context only: an earlier internal name for the chain, *Korai*, is now retired and does not appear in any current external artifact.

---

## Document List (5 Content Docs + This Index)

| # | Document | What it covers |
|---|----------|----------------|
| 00 | `00-INDEX.md` | This file. Disambiguation note, names, document list, reading paths, current state of methodology. |
| 01 | `01-isfr-index.md` | Everything about the index itself: what it is, the SOFR comparison, V1 prototype methodology, canonical V1 two-level methodology, V2 self-calibrating roadmap (MSPE, Bates–Granger, Kalman, Nelson–Siegel), off-chain service architecture, on-chain validator pipeline, the `0xA01` precompile interface, four-state circuit breaker, confidence score, intra-block phase ordering. |
| 02 | `02-yield-perpetual.md` | The yield-perpetual instrument and its uses: contract spec, payoff and position semantics, mark-price formula across Live / Degraded / Stale / Halted states, funding rate (premium + carry), KKT-verified clearing, margining and liquidation, plus the *clearing profile* primitive, AAVE liquidation backstop, $10M DAO treasury hedging worked example, leveraged yield, sub-index-specific consumer profiles. |
| 03 | `03-market-and-framework.md` | Why this matters and how it generalises: ~$668T OTC IRD market, six-orders-of-magnitude on-chain gap, LIBOR-to-SOFR template, benchmark flywheel, ~$4.5B+ index-industry economics, ISFR-YBS narrow wedge, the `BenchmarkIndex` Rust trait, the five canonical indices (ISFR, IAPI, IKQI, ISVI, IRRI), the universal five-stage pipeline, NRIS umbrella, cross-domain applications. |
| 04 | `04-business-and-regulatory.md` | How to ship this as a regulated benchmark business: the agent-attestation pipeline (prediction commit/reveal, CRPS scoring, ERC-8004 identity, x402 payment, Eigen-AVS cross-attestation, Inspect AI evals), the "regulated benchmark business, not oracle" framing, SOFR/VIX/S&P 500 lessons, IBA / IOC / ARRC tripartite governance, four-tier revenue stack, partnership target list, IOSCO 19 Principles mapped to ISFR design, UK BMR Cat-6 application path, EU recognition, ISDA SPS Matrix. |
| 05 | `05-foundations-and-roadmap.md` | Theoretical grounding and what's next: information-theoretic limits (Aaronson, Crutchfield, Tishby IB, Sims, Vicsek, Pezzulo–Friston FEP closure), time-as-first-class-primitive (Toto, Chronos-2, TimesFM, world models, metacontroller, allostasis, FutureWeaver), mathematical discovery (AlphaEvolve, Seed-Prover, library learning), measurable understanding, V1 → V2 → V3 → V4 evolution, the smallest complete loop, 90-day build plan, implementation priority matrix, risk register. |

---

## Reading Paths

Choose one based on your background and what you're trying to learn.

### Path A — Financial reader (you understand SOFR, IRDs, perpetual futures; you want to know what's different here)

1. `01-isfr-index.md` §1 (definition) and §3 (SOFR comparison) — pin the concept.
2. `03-market-and-framework.md` §1–7 — read the market sizing first; this anchors why anything else matters.
3. `02-yield-perpetual.md` — the instrument spec, especially the carry-component half of the funding formula.
4. `01-isfr-index.md` §4.2 (canonical V1) and §4.5 (hybrid rate) — the methodology that distinguishes ISFR from a flat oracle feed.
5. `04-business-and-regulatory.md` — read this in full. The framing decision (benchmark business, not oracle) is the most important call in the project.

### Path B — Benchmark engineer (you build oracle/index/data infrastructure; you want to understand the design)

1. `00-INDEX.md` — pin the terminology before reading anything else.
2. `01-isfr-index.md` — the full methodology and on-chain publication in one document.
3. `03-market-and-framework.md` §10–17 — the `BenchmarkIndex` trait and the five-index NRIS. Read if you care about extending the pattern past ISFR.
4. `02-yield-perpetual.md` §4–6 — the mark-price and funding formulas. This is what consumes your data.
5. `05-foundations-and-roadmap.md` §1 (information-theoretic limits) — six hard constraints that bound what coordination, prediction, and memory operations are possible.

### Path C — Agent integrator (you build agents that participate in the prediction loop, attest to source data, or consume ISFR)

1. `01-isfr-index.md` §1–3 — minimum context.
2. `04-business-and-regulatory.md` §2–9 — the full agent-attestation pipeline, the prediction commit/reveal loop, CRPS scoring, reputation tiers, ERC-8004 / x402 / Eigen-AVS / Inspect AI integrations, the receipt envelope. This is your spec.
3. `03-market-and-framework.md` §16 (event integration) — how `BenchmarkUpdate` events flow to your agent.
4. `02-yield-perpetual.md` §8 (clearing profile), §15 (knowledge production), §17 (operational failure modes) — what your agent needs to handle.
5. `05-foundations-and-roadmap.md` §1.3 (Wolpert difference utility) and §4 (counterfactual-task accuracy gap) — the theoretical bases for the reputation system and the validation methodology your agent's predictions are scored against.

### Path D — Regulator or compliance reader

1. `00-INDEX.md` — pin terminology.
2. `01-isfr-index.md` §1, §3, §4 — what the index is and how it is computed.
3. `04-business-and-regulatory.md` — read in full. UK BMR Cat-6, IOSCO 19 Principles mapping, ARRC tripartite IOC, FCA application pipeline, EU recognition route, ISDA SPS Matrix, IBA corporate-structure template.
4. `05-foundations-and-roadmap.md` §6, §12 — V1 → V2 → V3 → V4 evolution and what "done" looks like at each stage.

---

## Current State of Methodology (April 2026)

| Layer | Status |
|-------|--------|
| **V1 prototype (off-chain Python service)** | Shipped. Equal-weight weighted median over four DeFi yield sources, FastAPI surface, JSONL storage, hourly cadence. |
| **V1 canonical (on-chain validator computation)** | In design, partial implementation. Two-level four-class median, stake-weighted aggregation, `0xA01` precompile, four-state circuit breaker. |
| **V2 self-calibrating** | In design. Leave-one-out MSPE confidence, Bates–Granger adaptive class weights, Kalman smoothing, Nelson–Siegel yield curve, computed volatility premium. |

The off-chain prototype and the canonical on-chain V1 currently coexist. V1 canonical is the source of truth once activated; the off-chain service narrows to a parallel reference rate used for cross-checking, dashboarding, and the public `/v1/` API. V2 activation requires no API change at the precompile surface; methodology evolution is gated by IOSCO Principle 12 (30-day public consultation, 7-day timelock on parameter changes, super-majority IOC vote on methodology changes).

---

## What Belongs to Other Folders

This folder describes only the index, the yield perpetual, the benchmark framework, the benchmark business, and the agent-attestation surface as it relates to the index. Three adjacent areas have their own folders:

- **The chain itself** — TEE clearing engine internals, oracle precompile internals, validator consensus, the per-agent `ISFR_score` (Internal Solvency & Funding Ratio) — owned by the chain folder. This folder describes the *contract surface* ISFR uses, not the chain internals.
- **The agent runtime (Roko)** — runtime architecture, signal/cell/graph kernel, knowledge store internals, foraging model, cascade router internals — owned by the agent-runtime folder. This folder describes how agents *participate* in the index, not how the runtime works.
- **Business strategy / GTM / VC pitch / competitive landscape** — broader narrative around Nunchi as a company, fundraising, market positioning at the platform level — owned by the markets folder. This folder includes the *benchmark business playbook* (`04-business-and-regulatory.md`) because that is a product-strategy lens specific to running an index, but defers broader narrative to the markets folder.

---

## Key External Citations Preserved

This folder preserves all primary external references. The most consequential are:

- **BIS Statistical Bulletin (December 2025)** and **BIS Triennial Central Bank Survey (2025)** — the ~$668T OTC IRD notional and $7.9T daily turnover figures that anchor the market thesis.
- **ISDA "Key Trends in the Size and Composition of OTC Derivatives Markets in the First Half of 2025"** (June 2025) — the OIS-share-of-IRD restructuring (66.6% as of 2024) and SOFR-OIS 11.8× growth (2021 → 2024).
- **Federal Reserve Bank of New York / ARRC** SOFR transition documentation — the LIBOR-to-SOFR template referenced throughout the design and governance argument.
- **IOSCO "Principles for Financial Benchmarks"** (FR07/13, July 2013) — the 19 principles ISFR's methodology and governance map to.
- **UK BMR Cat-6** — the regulatory category and authorisation path; CF Benchmarks (FCA FRN 847100) is the precedent.
- **ISDA Digital Asset Derivatives Definitions Settlement Price Source Matrix** — the institutional whitelist for crypto rate sources.
- **Gneiting & Raftery** "Strictly Proper Scoring Rules" (JASA 102(477), 2007) — the foundational result for CRPS.
- **Bates & Granger** "The Combination of Forecasts" (Operational Research Quarterly 20(4), 1969) — the V2 adaptive class-weight basis.
- **Boyd & Vandenberghe** *Convex Optimization* (Cambridge University Press, 2004) — the KKT necessity-and-sufficiency basis for cooperative clearing.
- **DeFiLlama protocol dashboards** (April 2026) — DeFi TVL figures.
- arXiv pre-prints throughout for the agent-attestation primitives, methodology evolution, time-series foundation models, mathematical discovery, and information-theoretic limits.

---

## Style and Provenance Notes

- All references to the chain are written as "the Nunchi blockchain" or "the chain"; legacy nomenclature is not exposed in any current artifact.
- Numerical figures (TVL, notional, regulatory cost) are sourced and timestamped where possible.
- "ISFR V1" appears in two senses across this folder: the prototype Python-service equal-weight model, and the canonical chain-implemented two-level four-class model. They are explicitly distinguished in `01-isfr-index.md` §4.
- The five canonical NRIS indices use *Internet ...* expansions (IAPI, IKQI, ISVI, IRRI) per the chain's `BenchmarkIndex` trait documentation. ISFR uses *Implied Secured Funding Rate* per the published-product convention.
