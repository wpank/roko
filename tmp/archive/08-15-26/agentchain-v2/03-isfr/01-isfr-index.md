# 01 — The ISFR Index

> ISFR (Implied Secured Funding Rate) is a composite benchmark index measuring the cost of secured funding across decentralized finance. It is computed by Nunchi blockchain validators every ~10 seconds and published on-chain via a dedicated precompile. This document covers what ISFR is, how it is computed (V1 prototype, canonical V1, and the V2 self-calibrating roadmap), how it is published on-chain, and how the four-state circuit-breaker model protects consumers.

---

## 1. What ISFR Is

ISFR is a single number — the composite cost, in basis points, of taking secured funding across the major DeFi yield mechanisms: collateralized lending, structured (delta-neutral) yield, perpetual funding, and proof-of-stake validator yield. It is computed by every Nunchi blockchain validator independently as part of block production, finalised via stake-weighted median across validator votes, and published on-chain at a fixed precompile address every 25 blocks (≈10 seconds at the chain's 400 ms block time) — 8,640 publications per day.

ISFR is to DeFi what SOFR (Secured Overnight Financing Rate) is to traditional finance: the reference rate that derivative instruments — interest rate swaps, perpetual futures, floating-rate notes — settle against. SOFR underpins approximately $570T+ in instrument notional. Its DeFi equivalent did not exist before ISFR.

The full expansion is **Implied Secured Funding Rate**. Some older internal documents and Rust module-level doc comments use the variants *Internet Secured Funding Rate* or *Intersubjective Fact Registry*. This folder treats *Implied* as canonical; all three expansions describe the same on-chain object, methodology, and publication surface.

### What ISFR is not

- **Not a lending rate.** It does not tell you what you will earn on any specific protocol. It is a composite measure of the broad market.
- **Not a third-party oracle feed.** It is not pushed onto the chain by an external operator (Chainlink, Pyth, API3). Each validator independently observes source rates and submits a vote; the chain finalises via stake-weighted median.
- **Not a governance-set parameter.** No DAO vote sets the rate. The methodology is fixed in the protocol specification. Governance can adjust source weights within bounds and add or remove sources via timelock, but cannot override the computed value.

---

## 2. The Disambiguation Note (Concise)

There are two distinct things called "ISFR" inside the Nunchi codebase. They share spelling and nothing else.

| Term | Full name | Where it lives |
|------|-----------|----------------|
| **ISFR** (the index) | Implied Secured Funding Rate | This folder. Oracle precompile, yield-perp contracts, off-chain index service, validator computation. |
| **ISFR_score** (the metric) | Internal Solvency & Funding Ratio | The chain folder (TEE clearing engine). Per-agent scoreboard, risk manager. |

Throughout this folder, **"ISFR" refers exclusively to the index**. When the per-agent scoreboard metric appears for cross-reference, it is always written as the full name *Internal Solvency & Funding Ratio* or `ISFR_score`. The full disambiguation note lives in `00-INDEX.md`.

The third historical expansion (*Intersubjective Fact Registry*) appears in some Rust module-level doc comments for the on-chain ISFR registry. It refers to the same on-chain object as the index and is best read as legacy nomenclature for the agent-attestation surface, not a separate concept.

---

## 3. SOFR — The Template ISFR Is Modelled On

In traditional finance the rates ecosystem depends on a benchmark by construction:

> **benchmark rate → derivative pricing → hedging → risk management → capital efficiency → market depth → lower borrowing costs**

Remove the benchmark and the entire chain collapses. SOFR is published daily by the Federal Reserve Bank of New York, computed from roughly $1–2T in daily overnight Treasury repo volume. It replaced LIBOR after the 2012 rate-rigging scandal in which traders at multiple global banks manipulated submission-based rates. The lesson: a benchmark that depends on voluntary submissions from interested parties will be gamed; SOFR was designed to be transaction-anchored and manipulation-resistant by construction.

The instruments that depend on SOFR add up to roughly $570T+:

| Instrument class | Notional outstanding |
|------------------|----------------------|
| Interest rate swaps | ~$400T |
| Futures and options (CME) | ~$120T |
| Floating-rate notes | ~$48T |
| Adjustable-rate mortgages | ~$2.5T |
| Corporate floating-rate debt | ~$1.8T |
| **Total** | **~$570T+** |

### SOFR vs ISFR — side-by-side

| Property | SOFR | ISFR |
|----------|------|------|
| Publisher | Federal Reserve Bank of New York | Nunchi blockchain validator set (decentralised) |
| Update frequency | Once daily (08:00 ET) | Every ~10 seconds (8,640/day) |
| Computation | Volume-weighted median of overnight Treasury repo | Two-level: TVL-weighted median per class, then weighted sum across classes; finalised via stake-weighted median across validator votes |
| Underlying volume | ~$1–2T daily repo | ~$49.5B DeFi lending TVL plus structured, funding, and staking sources |
| Trust model | Trust the Federal Reserve | Trust that >50% of source weight AND >50% of validator stake is honest |
| Availability | Weekdays only | 24/7/365 |
| Latency | T+1 | Real-time (~10 seconds) |
| Programmability | Not natively programmable | Native precompile; one opcode call |
| Circuit breakers | None | Four-state model with confidence-based triggers (Live, Degraded, Stale, Halted) |

ISFR's design explicitly inherits three principles from SOFR:

1. **Transaction-anchored, not opinion-anchored.** Every input is an observable on-chain event (a borrow rate, a funding settlement, a beacon-chain reward), not a panellist forecast.
2. **Volume-weighted median with a published outlier filter.** SOFR's bottom-20% specials filter is the template; ISFR's TVL-weighted median with confidence modulation and 3-sigma outlier exclusion is the equivalent.
3. **Fallback language hard-coded before scale.** ARRC spent five years on fallback language and still needed legislative action for tough-legacy contracts. ISFR publishes ISDA-style fallback templates as part of its day-one rulebook (see `04-business-and-regulatory.md`).

---

## 4. Methodology — Two Generations of "V1"

The codebase contains two distinct "V1" methodologies. They arose at different points in the project and now coexist:

- **Prototype V1 (off-chain Python service).** A flat, equal-weight composite of four DeFi yield sources combined via a weighted median. Sources: Aave V3 USDC supply APY, Compound V3 USDC supply APY, Hyperliquid ETH perpetual funding rate, Ethena sUSDe 7-day rolling yield. Weights: 0.25 each.
- **Canonical V1 (on-chain validator computation).** A two-level aggregation over four mutually exclusive source classes (LENDING, STRUCTURED, FUNDING, STAKING) with class-level weights of 0.60 / 0.25 / 0.10 / 0.05, finalised via stake-weighted median across validator votes. Sources: Aave V3 + Compound V3 (LENDING), Ethena sUSDe (STRUCTURED), Hyperliquid ETH perp funding (FUNDING), ETH Beacon Chain staking (STAKING).

When a downstream contract or document refers to "ISFR" without qualification, it means the canonical V1 (or, after activation, V2). The prototype's output is referred to as "the off-chain prototype rate."

### 4.1 Prototype V1 — flat equal-weight composite

The prototype is the simplest credible benchmark methodology that satisfies the multi-source, manipulation-resistant, transaction-anchored requirements.

| Source | What it measures | Weight | Update cadence |
|--------|------------------|--------|----------------|
| Aave V3 | USDC supply APY (Ethereum mainnet) | 0.25 | Per Ethereum block (~12 s) |
| Compound V3 | USDC supply APY (Ethereum mainnet) | 0.25 | Per Ethereum block (~12 s) |
| Hyperliquid | ETH perpetual funding rate (annualised) | 0.25 | Per funding interval (~8 h) |
| Ethena | sUSDe 7-day rolling yield | 0.25 | Daily rolling window |

Formula:

```
ISFR = Weighted_Median(Sources) + Volatility_Premium
V1: Volatility_Premium = 0
```

`Weighted_Median` sorts source rates ascending, accumulates source weights, and returns the value at which cumulative weight first reaches 0.5 of total weight. Ties are broken deterministically by source index. `Volatility_Premium` is reserved for V2; in V1 it is identically zero, so the published rate is exactly the raw weighted median.

#### Why a weighted median, not a weighted mean

Three reasons, each independently sufficient:

1. **Outlier resistance.** A flash loan that spikes one source's rate to 50% for one block barely moves the median. Run through a mean, the same event dominates the composite.
2. **Byzantine tolerance.** Up to 49% of total weight can be corrupted without affecting the median. The mean has no such tolerance.
3. **Convention consistency.** The Nunchi blockchain's validator oracle uses weighted median at both the source layer and the validator layer. Using the same primitive in the off-chain service avoids surprises.

#### Worked example (prototype V1)

Source rates at one update epoch:

| Source | Rate |
|--------|------|
| Aave V3 USDC supply APY | 4.50% |
| Compound V3 USDC supply APY | 5.80% |
| Ethena sUSDe 7-day | 6.20% |
| Hyperliquid ETH perp funding | 7.10% |

Sort ascending and walk cumulative weights:

| Rate | Weight | Cumulative |
|------|--------|------------|
| 4.50% | 0.25 | 0.25 |
| 5.80% | 0.25 | 0.50 |
| 6.20% | 0.25 | 0.75 |
| 7.10% | 0.25 | 1.00 |

Cumulative weight reaches 0.50 at the boundary between 5.80% and 6.20%. The published ISFR is the mean of the bracketing values: (5.80% + 6.20%) / 2 = **6.00%** (600 bps).

#### Manipulation comparison

If a flash loan spikes Aave's rate to 50.00% for one block, post-attack rates are 5.80%, 6.20%, 7.10%, 50.00%.

- **Median result:** Cumulative weight reaches 0.50 between 6.20% and 7.10%. ISFR = 6.65% — a transient 65 bps shift that corrects on the next update.
- **Mean result:** (5.80% + 6.20% + 7.10% + 50.00%) / 4 = 17.28% — an ~1,138 bps spike that nearly triples the index.

Every credible benchmark in financial history (SOFR, SONIA, €STR) uses a median or trimmed mean for the same reason.

### 4.2 Canonical V1 — two-level class aggregation

The canonical V1 is what Nunchi blockchain validators compute. It preserves the median primitive but introduces structure: sources are grouped into four mutually exclusive classes; aggregation runs at two levels; and a separate stake-weighted median across validator votes provides a second independent line of Byzantine defence.

| Class | What it measures | V1 sources | V2 candidates | Class weight | Rationale |
|-------|------------------|-----------|---------------|--------------|-----------|
| **LENDING** | Collateralised lending yield | Aave V3, Compound V3 | Morpho, Spark, Maker DSR | 0.60 | Most analogous to SOFR; deepest, most stable DeFi yield market; primary hedging target. |
| **STRUCTURED** | Multi-instrument strategy yield | Ethena sUSDe | Pendle PT yields, on-chain SOFR | 0.25 | Delta-neutral yield captures funding with dampening. |
| **FUNDING** | Perpetual futures funding rate | Hyperliquid ETH perp | dYdX, GMX | 0.10 | Real signal on speculative positioning; explicitly downscaled because of volatility. |
| **STAKING** | Proof-of-stake validator yield | ETH Beacon Chain | Lido stETH APR, rETH APR | 0.05 | Floor rate for the Ethereum economy; very stable. |

Hyperliquid's funding rate is consumed as a read-only data input. The Nunchi blockchain has no operational, settlement, or systemic dependency on any external exchange. The relocation of Hyperliquid from the lending basket (in the prototype) to a separate FUNDING class with a small weight is the most consequential change between prototype and canonical V1: funding rates spike for reasons unrelated to lending demand (liquidations, basis trades, speculative manias) and should not dominate a "secured funding" benchmark.

#### Level 1 — intra-class aggregation

Within each class, sources combine into a single class rate via a TVL-weighted median with confidence modulation:

```
effective_weight(source) = tvl(source) × (confidence(source) / 100)
```

`confidence(source)` is a 0–100 score that modulates a source's contribution. A new source enters with low confidence (typically 30) for a 30-day probation period, contributing only 30% of its TVL-proportional weight. Confidence is governance-assigned in V1 and transitions to leave-one-out MSPE auto-calibration in V2.

Computation:

1. Sort source rates ascending within the class.
2. Accumulate effective weights.
3. Return the value at which cumulative effective weight first reaches 50% of total class weight.

The TVL-weighted median tolerates up to 49% corrupted effective weight within each class.

#### Level 2 — inter-class aggregation

The composite ISFR is a deterministic weighted sum of the four class rates:

```
ISFR = 0.60 × LENDING + 0.25 × STRUCTURED + 0.10 × FUNDING + 0.05 × STAKING
```

The two-level design creates a structural firewall against volatility contamination. If FUNDING spikes to 200% during a speculative mania, it contributes at most `0.10 × 200% = 20` percentage points to the composite. The other three classes, holding 90% of the weight, anchor the rate near their stable levels.

#### Worked example (canonical V1)

| Class | Sources (TVL, confidence) | Source rates | TVL-weighted median |
|-------|---------------------------|--------------|---------------------|
| LENDING | Aave V3 ($23.5B, conf=95), Compound V3 ($2.1B, conf=90) | 6.20%, 5.80% | **6.20%** (Aave holds ≈92% of effective weight) |
| STRUCTURED | Ethena sUSDe ($5.2B, conf=85) | 7.10% | **7.10%** |
| FUNDING | Hyperliquid ETH perp ($1.8B OI, conf=70) | 12.40% | **12.40%** |
| STAKING | ETH Beacon Chain ($35B, conf=98) | 3.20% | **3.20%** |

```
ISFR = 0.60 × 6.20% + 0.25 × 7.10% + 0.10 × 12.40% + 0.05 × 3.20%
     = 3.720% + 1.775% + 1.240% + 0.160%
     = 6.895%
     ≈ 690 bps
```

Published this epoch:

| Index | Value |
|-------|-------|
| ISFR | 6.90% (690 bps) |
| ISFR.LENDING | 6.20% (620 bps) |
| ISFR.STRUCTURED | 7.10% (710 bps) |
| ISFR.FUNDING | 12.40% (1240 bps) |
| ISFR.STAKING | 3.20% (320 bps) |

The elevated FUNDING rate (12.40%) contributes only 124 bps to the composite — its 10% class weight bounds its influence.

#### Sub-indices are byproducts

Every computation round produces five values, all available via the oracle precompile at zero marginal cost: `ISFR`, `ISFR.LENDING`, `ISFR.STRUCTURED`, `ISFR.FUNDING`, `ISFR.STAKING`. A protocol hedging Aave supply rate risk specifically can reference `ISFR.LENDING` directly. A delta-neutral vault using Ethena monitors `ISFR.STRUCTURED`. The composite serves as the canonical settlement rate for yield perpetuals tracking aggregate DeFi yield.

### 4.3 Two-layer Byzantine tolerance

ISFR is defended at two independent layers. To corrupt the published value, an attacker must compromise both simultaneously.

**Layer 1 — sources (intra-class median).** The TVL-weighted median tolerates up to ⌊k/2⌋ corrupted sources in a class of k. With 2 sources in LENDING, an attacker must corrupt 1 to move the median; with 7 V2 sources in LENDING, they must corrupt 4. Effective-weight accounting (TVL × confidence) further raises the bar because TVL itself is hard to manipulate.

**Layer 2 — validators (stake-weighted median across votes).** Each validator independently computes its own ISFR and submits an `OracleVote`. The chain finalises via stake-weighted median across all validator submissions. This tolerates up to 49% compromised stake.

**Combined.** To shift ISFR to an arbitrary value, an attacker must control 50%+ of source weight in at least one class AND 50%+ of validator stake in the same epoch. Either layer alone stops the attack. The defence compounds: corrupting both is qualitatively harder than corrupting either.

A formal restatement of the validator-level aggregation: let $V = \{(v_i, s_i)\}$ be the set of validator votes where $v_i$ is the submitted ISFR value and $s_i$ is the validator's normalised stake weight ($\sum s_i = 1$). Sort votes by value. The aggregate ISFR is the value $v_j$ such that:

```
sum(s_i for i ≤ j) ≥ 0.50  AND  sum(s_i for i < j) < 0.50
```

If the boundary falls exactly between two votes, the aggregate interpolates linearly.

#### Outlier exclusion (3-sigma, two-pass)

The on-chain registry uses a two-pass outlier exclusion algorithm before computing the final stake-weighted median:

1. Compute an initial weighted median across all submissions.
2. Compute the weighted standard deviation around that median.
3. Exclude any submission more than `outlier_sigma` (default 3.0) standard deviations from the initial median.
4. Recompute the weighted median on the filtered set.

Reputation enters the weighting as `confidence × reputation`, where `reputation ∈ [0.0, 1.0]` per submitting agent (minimum `0.5` for eligibility). This prevents low-reputation Sybils from contributing signal.

#### Default chain configuration

| Constant | Value | Purpose |
|----------|-------|---------|
| `epoch_duration_secs` | 28,800 (8 h) | One clearing cycle |
| `max_kkt_residual` | 1e-6 | Certificate acceptance threshold |
| `min_submissions_for_clearing` | 2 | Minimum submissions before aggregation |
| `min_reputation` | 0.5 | Eligibility floor |
| `max_rate_bound` | 0.1 (10%) | Maximum absolute rate value accepted |
| `outlier_sigma` | 3.0 | Sigma multiplier for outlier exclusion |

Note that the 8-hour epoch is the *clearing cycle* duration (commit → reveal → solve → certificate → verify → settle), not the publication cadence. ISFR itself publishes every 25 blocks (≈10 seconds).

### 4.4 V2 — self-calibrating methodology

V1 is deliberately simple to maximise auditability. V2 replaces fixed parameters with data-driven self-calibration, every step of which retains a falsifiable backtest and on-chain provenance.

| Mechanism | V1 (launch) | V2 (self-calibrating) |
|-----------|-------------|-----------------------|
| Source confidence | Governance-assigned (0–100) | Leave-one-out MSPE (automatic) |
| Class weights | Fixed (60/25/10/5) | Bates–Granger optimal combination with governance rails |
| Intra-class aggregation | TVL-weighted median | Cost-stratified trimmed mean |
| Smoothing | Simple EMA | Kalman filter (separates measurement noise from process noise) |
| Yield curve | Discrete rate points | Nelson–Siegel 4-parameter continuous curve |
| Volatility premium | 0 | Computed from source disagreement (formula TBD) |

#### V2 self-calibrating source confidence (leave-one-out MSPE)

V1 confidence scores are governance-assigned during a 30-day probation period, then frozen. V2 replaces them with a leave-one-out Mean Squared Prediction Error:

```
ISFR_loo[s]   = aggregate(all sources except s)
residual[s][t] = source_rate[s][t] − ISFR_loo[s][t]
MSPE[s][t]    = λ × MSPE[s][t-1] + (1−λ) × residual[s][t]²
confidence[s][t] = 1 / (1 + MSPE[s][t] / MSPE_floor)
```

Leave-one-out breaks circularity: a source cannot inflate its own confidence by dominating the index. Sources that consistently agree with the consensus earn higher weight organically.

#### V2 adaptive class weights (Bates–Granger)

V1's fixed weights (60/25/10/5) are replaced with weights inversely proportional to each class's prediction error, bounded by governance rails:

```
MSPE_class[k][t] = λ × MSPE_class[k][t-1] + (1−λ) × (class_rate[k][t-1] − ISFR_realized[t])²
w_k_raw[t]       = 1 / MSPE_class[k][t]
w_k[t]           = clamp(w_k_raw[t] / Σ w_j_raw[t], w_min[k], w_max[k])
```

Governance sets bounds per class — for example, LENDING 30%–80%, FUNDING 0%–20%. The system finds optimal weights within those bounds from empirical data, following the forecast combination framework of Bates and Granger (*Operational Research Quarterly* 20(4), 1969). V1 weights become initial conditions, not permanent parameters.

#### V2 cost-stratified trim fractions

V2 replaces the flat TVL-weighted median with a trimmed mean per class, where the trim fraction reflects the economic cost of manipulating that source type:

| Class | Trim α | Rationale |
|-------|--------|-----------|
| LENDING | 0.15 | Manipulation requires actual capital deployment |
| STRUCTURED | 0.20 | Delta-neutral strategies can be unwound rapidly |
| FUNDING | 0.30 | Perpetual positioning can be cheaply manipulated via leverage |
| STAKING | 0.05 | Requires attacking Ethereum consensus |

A flat trim fraction ignores economic reality — STAKING (which requires attacking Ethereum's consensus to manipulate) gets the same defence as FUNDING (which requires only a leveraged perp position). Per-class trim allocates defence proportional to actual manipulation cost.

#### V2 Kalman filter smoothing

V2 replaces the simple EMA in the hybrid mark price formula with a Kalman filter that adapts to signal quality. An EMA treats all deviations as signal noise; a Kalman filter distinguishes measurement noise (source disagreement, stale data) from process noise (genuine rate movement). When oracle noise is high (sources disagree), the filter automatically upweights the clearing signal, and vice versa.

#### V2 Nelson–Siegel yield curve

V1 publishes a single spot rate per epoch. V2 publishes four parameters characterising the entire yield curve:

```
y(τ) = β₁ + β₂ · [(1 − e^(−λτ)) / (λτ)] + β₃ · [(1 − e^(−λτ)) / (λτ) − e^(−λτ)]
```

Where β₁ is the long-run level, β₂ the slope, β₃ the curvature, and λ the decay speed. Any consumer can reconstruct ISFR at any tenor τ from these four numbers. DeFi today has no native term structure; rates are quoted at a single tenor. Nelson–Siegel transforms discrete rate observations into a continuous curve.

#### Governance constraints on V2 source weights

- Maximum weight per source: 0.35 (no single source exceeds 35% influence).
- Minimum weight per source: 0.05 (no source is effectively zeroed without explicit removal).
- Weight changes require a 7-day timelock and a super-majority governance vote.
- Methodology changes require a minimum 30-day public consultation per IOSCO Principle 12.

### 4.5 Hybrid rate — oracle plus endogenous market discovery

Most benchmarks have a single source of truth. ISFR has two — and the interaction between them is a core design choice.

- **`ISFR_oracle` (the external anchor).** The oracle layer (sections above) measures external DeFi rates: validators scrape sources, compute the two-level aggregation, and produce `ISFR_oracle` via stake-weighted median of their votes.
- **`ISFR_market` (endogenous price discovery).** The clearing engine solves a Quadratic Programming problem each clearing round to match all buy and sell orders for rate exposure at the price that maximises total surplus. The clearing engine does not set the rate; the rate emerges from the mathematics of optimal allocation.

The block header publishes both. The canonical ISFR combines them:

```
ISFR = ISFR_oracle + EMA(ISFR_market − ISFR_oracle)
```

At launch with thin clearing liquidity, the EMA contribution is negligible: `ISFR ≈ ISFR_oracle`. As the yield-perp market deepens, `ISFR_market` becomes progressively more informative, and the benchmark naturally transitions toward endogenous price discovery without a binary cutover.

This convergence pattern mirrors how SOFR evolved. SOFR itself was published from April 2018; Term SOFR (a market-derived rate based on SOFR futures) was introduced in July 2021, more than three years later. Oracle-first, market-later sequencing is deliberate and historically validated.

The KKT-optimality basis for the clearing layer follows Boyd and Vandenberghe, *Convex Optimization* (Cambridge University Press, 2004): for convex programs the Karush–Kuhn–Tucker conditions are necessary and sufficient for global optimality, which makes the cooperative clearing solution provable in O(n) verification time.

---

## 5. Off-Chain Service Architecture

A standalone Python service computes the prototype ISFR rate and exposes it over HTTP. This service exists for development, testing, dashboarding, and as one of three transition-phase publication paths to the on-chain oracle.

The service is written in Python because the web3 RPC and HTTP libraries needed for source scraping are most mature there. A Rust rewrite is on the table if performance ever becomes the binding constraint; for the prototype's per-hour cadence, Python is adequate.

### Components

| Component | Purpose |
|-----------|---------|
| **Scheduler** | Triggers a calculation round every `ISFR_SCHEDULE_HOURS` (default 1 h). Idempotent — a missed tick due to restart is recovered on next boot. |
| **Source scrapers (×4)** | One per source. Aave V3 and Compound V3 scrapers query supply rates via the venue contracts on Ethereum mainnet over the configured RPC. The Hyperliquid scraper polls the venue's public funding-rate endpoint. The Ethena scraper computes the 7-day rolling sUSDe yield from the venue's reserve and supply state. |
| **Calculator** | Implements `weighted_median(values, weights)` over the four source rates with equal weights of 0.25 each. Volatility premium is identically zero in V1; the calculator's interface accepts a premium term so V2 activation requires no API change. |
| **Storage (JSONL)** | Append-only JSONL of `{timestamp, sources, rate, premium, computed}`. One file per day; 90 days hot retention. |
| **FastAPI / uvicorn** | Serves HTTP. `/v1/isfr/current`, `/v1/isfr/history`, `/health`. |

### Service identification

| Field | Value |
|-------|-------|
| Repository | `https://github.com/Nunchi-trade/isfr.git` |
| Language | Python |
| Default entry point (scheduler + API) | `python -m isfr.main` |
| API-only entry point | `uvicorn isfr.api:app --host 0.0.0.0 --port 8000` |
| Installation | `pip install -e ".[dev]"` |

### Configuration surface

| Variable | Default | Description |
|----------|---------|-------------|
| `ETH_RPC_URL` | `https://eth.llamarpc.com` | Ethereum RPC endpoint for the Aave and Compound scrapers. Production should pin to a private node. |
| `ISFR_DATA_DIR` | `data` | Directory for JSONL storage of historical rates. |
| `ISFR_SCHEDULE_HOURS` | `1` | Calculation interval in hours. |
| `ISFR_PORT` | `8000` | API server port. |

### HTTP API

| Endpoint | Method | Returns |
|----------|--------|---------|
| `/v1/isfr/current` | GET | Latest ISFR rate, last sources, computed timestamp, source health summary |
| `/v1/isfr/history` | GET, query `?days=N` (default 30) | Historical snapshots within the requested window |
| `/health` | GET | Liveness summary: scheduler heartbeat, last successful round, per-source freshness |

A representative `/v1/isfr/current` body:

```json
{
  "isfr_bps": 678,
  "computed_at": "2026-04-30T14:00:00Z",
  "sources": [
    {"name": "aave_v3_usdc", "rate_bps": 620, "stale": false, "last_seen": "2026-04-30T13:59:48Z"},
    {"name": "compound_v3_usdc", "rate_bps": 580, "stale": false, "last_seen": "2026-04-30T13:59:48Z"},
    {"name": "hyperliquid_eth_perp_funding", "rate_bps": 1240, "stale": false, "last_seen": "2026-04-30T13:00:00Z"},
    {"name": "ethena_susde_7d", "rate_bps": 710, "stale": false, "last_seen": "2026-04-30T13:00:00Z"}
  ],
  "weights": {"aave_v3_usdc": 0.25, "compound_v3_usdc": 0.25, "hyperliquid_eth_perp_funding": 0.25, "ethena_susde_7d": 0.25},
  "premium_bps": 0,
  "method": "prototype.v1.weighted_median"
}
```

### Three publication paths to the on-chain oracle

| Option | How it works | Pros | Cons |
|--------|--------------|------|------|
| **Validator-sidecar integration** | Each Nunchi blockchain validator runs an `oracle-sidecar` that polls `/v1/isfr/current` and includes the value in its per-validator `OracleVote` | Matches the chain's existing oracle architecture; stake-weighted median across validators is the existing aggregation primitive | Requires every validator to either run the index service themselves or trust a single endpoint |
| **DeskFeed connector** | A `FeedSource` trait implementation specifically for ISFR; uses authenticated HTTPS to a Nunchi-operated endpoint | Clean separation; matches the SOFR / UST-3M tier-2 pattern | Single point of trust until multiple endpoints are supported |
| **On-chain publisher contract** | A single signer posts ISFR values to a contract; the oracle reads from there | Simplest | Single-signer risk; defeats the multi-source median |

Recommendation: use the DeskFeed connector with multiple endpoints from V2 onwards; in V1 testnet, a single trusted endpoint is acceptable as a transition measure. Once canonical V1 is live in validators, the off-chain service becomes a parallel reference rate used for cross-checking, dashboarding, and the prototype `/v1/` API surface.

---

## 6. On-Chain Publication

ISFR is a consensus-level primitive on the Nunchi blockchain's Kernel Plane, not an application-layer contract. The publishing surface is a precompile at address `0xA01`, callable from any smart contract on the chain with fixed gas cost — as efficient as reading the block timestamp or chain ID.

This is the structural contrast with all other on-chain rate publications:

- **Chainlink, Pyth, API3** rely on a separate operator layer. If those operators are compromised, the data is compromised.
- **IPOR** is a contract call with variable gas reading from a published index.
- **ISFR** is a precompile reading from chain consensus state. It is as available as the chain itself.

### 6.1 Validator computation pipeline

Each validator runs the same pipeline every 25 blocks (≈10 seconds at the chain's 400 ms block time) — 8,640 publications per day:

1. **Pull source data** via RPC: Aave V3 reserves, Compound V3 markets, Hyperliquid funding endpoint, Ethena reserve state, ETH Beacon Chain epoch yields.
2. **Health check sources.** Apply latency, deviation, and availability thresholds. Stale or out-of-band sources are excluded from this round's intra-class computation.
3. **Compute Level 1 (intra-class).** TVL-weighted median per source class with confidence modulation. Produces four class rates.
4. **Compute Level 2 (inter-class).** Weighted sum: `ISFR = 0.60 × LENDING + 0.25 × STRUCTURED + 0.10 × FUNDING + 0.05 × STAKING`.
5. **Submit `OracleVote`.** Bundle the composite ISFR plus all four sub-indices into a vote, BLS-signed over `(value_bps, block_height)`, broadcast to the validator network.

The chain finalises via stake-weighted median across all submitted votes, applying the two-pass 3-sigma outlier exclusion described in §4.3. The result is committed to the block header.

```
struct OracleVote {
    value_bps: u32,           // The validator's computed ISFR in basis points
    block_height: u64,        // Block height this vote applies to
    signature: BlsSignature,  // BLS signature over (value_bps, block_height)
    validator_index: u32,     // Validator index in the current committee
}
```

#### Cadence and density

| Quantity | Value | Comparison |
|----------|-------|------------|
| Block time | 400 ms | — |
| ISFR cadence | 25 blocks (≈10 s) | 25× lighter than per-block computation |
| Publications per day | 8,640 | vs SOFR's 1/day, vs IPOR's ~96/day |

DeFi rates change over hours, not milliseconds. 10-second granularity is sufficient resolution for yield-perp mark-to-market and clearing while reducing validator workload 25× versus per-block computation.

### 6.2 Precompile interface

The full Solidity interface exposed at `0xA01`:

```solidity
/// ISFR Oracle Precompile at address 0xA01 on Nunchi blockchain Kernel Plane.
interface IISFROracle {
    /// Returns the current ISFR value in basis points and its publication state.
    function current() external view returns (uint32 valueBps, uint8 state);

    /// Returns ISFR + 4 sub-indices + confidence + counts.
    function currentRate() external view returns (
        uint256 isfr,           // composite rate in basis points
        uint256 lendingRate,    // ISFR.LENDING
        uint256 structuredRate, // ISFR.STRUCTURED
        uint256 fundingRate,    // ISFR.FUNDING
        uint256 stakingRate,    // ISFR.STAKING
        uint64  timestamp,
        uint8   confidence      // 0-100, validator agreement metric
    );

    /// Returns the full ISFR snapshot at a specific block height.
    /// Reverts if blockHeight is older than 90 days.
    function at(uint64 blockHeight) external view returns (ISFRSnapshot memory);

    /// Returns the time-weighted average ISFR between two block heights.
    function twap(uint64 startBlock, uint64 endBlock) external view returns (uint32 twapBps);

    /// Returns historical ISFR values (up to 30 days on-chain).
    function history(uint64 fromEpoch, uint64 toEpoch) external view returns (
        uint256[] memory rates,
        uint64[]  memory timestamps
    );

    /// Returns the number of sources currently reporting (0-4 in V1, 0-7+ in V2).
    function activeSources() external view returns (uint32);

    /// Returns the current confidence score in basis points (0-10000 = 0-100%).
    function confidence() external view returns (uint16 confidenceBps);

    /// Returns the rate of change since the previous update, in signed basis points.
    function delta() external view returns (int32 deltaBps);
}
```

### On-chain snapshot format

```solidity
struct ISFRSnapshot {
    uint32 valueBps;          // ISFR in basis points (e.g., 690 = 6.90%)
    uint64 blockHeight;       // Block at which this value was computed
    uint64 timestamp;         // Unix timestamp
    uint8  state;             // 0=Live, 1=Degraded, 2=Stale, 3=Halted
    uint16 confidenceBps;     // Validator confidence (0-10000)
    uint32 numSources;        // Number of active sources contributing
    uint32 numValidatorVotes; // Number of validator votes in this round
}
```

A single snapshot is therefore 35 bytes packed; 90 days of 10-second snapshots is ≈27 MB on-chain — modest by Layer-1 storage standards.

### 6.3 Publication states and circuit breakers

ISFR operates in one of four states at any time, determined automatically by the consensus layer based on data availability and validator agreement.

| State | Condition | ISFR behaviour | Yield-perp behaviour |
|-------|-----------|----------------|----------------------|
| **Live** | ≥3 sources reporting AND confidence ≥ 70% | Normal publication | Normal operations |
| **Degraded** | Exactly 2 sources OR confidence 50–70% | Rate published with wider confidence interval; warning flag set | Normal with warning flag; tight clearing-profile triggers may pause |
| **Stale** | Exactly 1 source OR 50–67% validator participation | Rate frozen at last Live/Degraded value | No new clearing-profile activations |
| **Halted** | 0 sources OR confidence < 50% OR consensus failure | No ISFR published | Liquidations paused; no new positions; emergency CLOB only |

#### Recovery hysteresis

Recovery from Degraded or Stale to Live requires confidence exceeding **80%** for **3 consecutive update periods (30 seconds)** before the transition. The 70%-down / 80%-up hysteresis prevents oscillation around the threshold.

#### Circuit-breaker event

When confidence drops below 70%:

1. ISFR state transitions from Live to Degraded (≥50%) or Halted (<50%).
2. The previous Live rate is cached as a fallback.
3. An `ISFRCircuitBreaker` event is emitted on-chain.
4. The clearing engine switches to wider spread limits (Degraded) or emergency CLOB mode (Halted).
5. Recovery requires the 80% / 30-second hysteresis above.

The four-state model is a deliberate design choice: there is no silent failure mode. The rate is either trustworthy or explicitly flagged as degraded.

### 6.4 Confidence score

Two equivalent definitions are used in different layers of the stack and produce the same operational behaviour.

**Stake-weighted within-σ definition.** Confidence is the percentage of total validator stake that submitted votes within one standard deviation of the stake-weighted median:

```
confidence = sum(s_i for all i where |v_i − median| ≤ σ) / sum(all s_i)
```

where σ is the stake-weighted standard deviation across all submitted votes. High confidence (≥90%) means validators substantially agree.

**Within-10bps definition.** Confidence is the percentage of total stake that submitted votes within 10 basis points of the finalised median (0–100 scale). This is the threshold-form used by consuming contracts that gate actions on a fixed precision.

Either definition produces a 0–100 (or 0–10000 bps) score that drives circuit-breaker state transitions, is published in the snapshot for consumer contracts to gate actions on, and is one of the two reputation inputs for validator oracle-mining rewards (validators within 1σ of the median get full rewards; outliers > 2σ from the median receive reduced rewards).

### 6.5 Source resilience

Validators perform health checks independently each computation round. There is no separate watcher service.

#### Per-source liveness timeouts

| Source | Expected update frequency | Liveness timeout |
|--------|--------------------------|------------------|
| Aave V3 | Every Ethereum block (~12 s) | 120 s (10 missed blocks) |
| Compound V3 | Every Ethereum block (~12 s) | 120 s |
| Ethena sUSDe | As funding settles (~8 h) | 24 h |
| ETH Beacon Chain | Per epoch (~6.4 min) | 30 min (5 missed epochs) |
| Hyperliquid ETH perp funding | Per funding interval (~8 h) | 16 h (2 missed intervals) |

#### Health-check thresholds

| Health metric | Threshold | Action |
|---------------|-----------|--------|
| Latency | >30 s since last update | Mark source degraded; exclude this round |
| Deviation | >3σ from peer sources in the same class | Exclude from intra-class computation |
| Availability | Source RPC unreachable for >60 s | Exclude; reweight remaining class sources |

#### Source failover

If all sources in a class go offline, that class's weight is redistributed proportionally to remaining healthy classes. For example, if FUNDING's sole source goes offline, its 10% is redistributed: LENDING gains +6.67%, STRUCTURED +2.78%, STAKING +0.56%. ISFR requires at least 3 healthy sources across classes for Live status; below 2, the oracle enters Stale.

This is the structural payoff of the four-class architecture: one class can fail completely without halting the index. A flat-source benchmark (the prototype methodology) does not have this property.

### 6.6 Access patterns for consumers

Three idiomatic call patterns:

```solidity
// 1. Mark-to-market read (yield perp engine)
(uint32 valueBps, uint8 state) = ISFROracle(0xA01).current();
require(state == 0, "ISFR not live");

// 2. Historical lookup (audit, dispute, settlement)
ISFRSnapshot memory snap = ISFROracle(0xA01).at(blockHeight);

// 3. TWAP for slow-moving consumers (ETF NAV, treasury hedging triggers)
uint32 twapBps = ISFROracle(0xA01).twap(startBlock, endBlock);
```

The TWAP read is particularly useful for treasury-grade consumers who want exposure to the rate's average behaviour over a window without being moved by transient spikes.

### 6.7 Phase ordering within a block

The Nunchi blockchain orders intra-block phases so that liquidations and funding always operate on the freshest possible mark prices:

```
Phase 1: ORACLE       → apply this round's stake-weighted median ISFR
Phase 2: ACCRUAL      → compute funding using fresh oracle prices
Phase 3: LIQUIDATION  → check margins using fresh mark prices
Phase 4: PERPS        → match orders
```

This ordering means liquidations cannot be triggered by stale ISFR. The combination of phase ordering and the Stale/Halted circuit-breakers eliminates the most common oracle-induced failure mode in DeFi history (cascading liquidations on stale prices).

---

## 7. Why This Methodology Is Credible

ISFR's design explicitly mirrors three principles drawn from SOFR and IOSCO:

1. **Transaction-anchored, not opinion-anchored.** Every input is an observable on-chain event (a borrow rate, a funding settlement, a beacon chain reward), not a panellist forecast. The LIBOR scandal — $9B+ in fines, Tom Hayes's 14-year sentence — was caused by panellist submission gaming. ISFR has no submitters in the LIBOR sense; it has validators reading public state.
2. **Volume-weighted median with a published filter.** SOFR's bottom-20% specials filter is the template; ISFR's TVL-weighted median with confidence modulation and 3-sigma outlier exclusion is the equivalent.
3. **Fallback language hard-coded before scale.** ARRC spent five years on fallback language and still needed legislative action for tough-legacy contracts. ISFR publishes ISDA-style fallback templates as part of its day-one rulebook.

The two-level architecture, sub-indices, and dual-median Byzantine tolerance combine to produce a benchmark that is structurally harder to manipulate than any flat-average alternative (IPOR), any single-venue oracle (Chainlink/Pyth feeds of one protocol's rate), or any survey-based rate (LIBOR-style). Governance constraints, time-locked weight changes, public consultation, and an Independent Oversight Committee provide the procedural complement to the structural guarantees.

---

## 8. References

- Bates, J.M. & Granger, C.W.J. "The Combination of Forecasts." *Operational Research Quarterly* 20(4), 1969.
- Boyd, S. & Vandenberghe, L. *Convex Optimization*. Cambridge University Press, 2004.
- Federal Reserve Bank of New York / Alternative Reference Rates Committee. "SOFR Transition" documentation.
- IOSCO. "Principles for Financial Benchmarks." FR07/13, July 2013.
- Kim, H. & Park, A. "Designing Funding Rates for Perpetual Futures." arXiv:2506.08573, 2025.
- "Optimal Benchmark Design under Costly Manipulation." arXiv:2506.22142, 2025.
- Gneiting, T. & Raftery, A.E. "Strictly Proper Scoring Rules, Prediction, and Estimation." *Journal of the American Statistical Association* 102(477), 2007.
