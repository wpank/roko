# ISFR: A Regulated Benchmark Business for DeFi Rates

## Comprehensive Strategy Synthesis

---

## 1. The Benchmark Business Thesis

### What ISFR actually is

The Internet Secured Funding Rate (ISFR) is a composite benchmark index measuring the cost of secured funding across decentralized finance. It aggregates yield signals from lending protocols, staking, structured yield products, and perpetual funding rate venues into a single reference rate. The critical framing -- and the one that determines whether this business succeeds or fails -- is that ISFR is a **regulated benchmark business**, not oracle infrastructure. This distinction was the sharpest point of feedback from Praneeth Srikanti (Emergent Ventures) during a 65-minute pitch call on April 30, 2026, when he told the Nunchi team directly: "This is not just an Oracle network. A benchmark business requires trust, methodology, governance, and adoption -- not just technical infrastructure."

ISFR is to DeFi what SOFR is to traditional finance: the reference rate that interest rate swaps, perpetual futures, floating-rate notes, and structured products settle against. Traditional finance anchors approximately $668 trillion in interest rate derivative notional on benchmark rates. DeFi, which holds approximately $49.5 billion in lending TVL as of April 2026, has on-chain interest rate derivative TVL under $100 million. That six-order-of-magnitude gap is not a marketing failure. It is the absence of a foundational primitive. Without a benchmark rate, none of these instruments can be priced, hedged, or settled.

### The $4.5B+ index industry revenue model

The economics of benchmark businesses are among the most attractive in financial services. Combined index industry revenue across the three dominant firms exceeds $4.5 billion annually:

- **S&P Dow Jones Indices (SPDJI):** approximately $1.6 billion in revenue at 60%+ EBITDA margins
- **MSCI Index segment:** approximately $1.6 billion in revenue at **76% adjusted EBITDA margin** -- among the highest in any financial services vertical
- **FTSE Russell:** GBP 918 million in revenue

These margins exist because the toll-booth model scales with passive assets under management. US passive fund AUM surpassed active in late 2024 and reached approximately $19.1 trillion by October 2025. Once an ETF tracks an index, switching benchmarks is functionally impossible -- pension mandates, ERISA contracts, and ETF prospectuses are hardwired to specific benchmarks. State Street pays S&P DJI approximately 3 basis points of AUM plus $600,000 per year in flat fees for SPY alone, generating roughly $120 million annually from a single ETF. Index licensing fees represent 31--36% of all ETF expense ratios.

Bloomberg's $781 million acquisition of Barclays Risk Analytics and Index Solutions in August 2016 (rebranded Bloomberg Fixed Income Indices in August 2021) demonstrates that credibility in this space is buyable but expensive. Over 500 ETFs and more than $4.1 trillion in mutual fund and ETF assets benchmark to Bloomberg fixed-income indices today; approximately $2.3 trillion tracks the US Aggregate alone. Three administrator changes (Lehman to Barclays to Bloomberg) and the Agg still dominates because institutional mandates are hardwired.

### Why "benchmark business" and not "oracle"

The distinction is structural, not semantic. Oracle networks push external data onto a chain. A benchmark is computed by an independent administrator using a published, audited methodology, under regulatory oversight, with an independent oversight committee, and licensed to market participants on a recurring revenue basis. The LIBOR scandal -- $9 billion in fines, Tom Hayes's 14-year prison sentence -- taught the industry that submission-based rates from interested parties will be gamed. SOFR was designed to replace LIBOR by anchoring in the volume-weighted median of over $1 trillion in daily underlying repo transactions, collected under regulatory authority by the New York Federal Reserve with an OFR ex-officio observer.

ISFR must copy three design principles directly from SOFR: (1) transaction-anchored, not opinion-anchored -- every input must be an observable on-chain event, not a panelist forecast; (2) volume-weighted median with a published filtering algorithm for outliers (SOFR's 20% bottom-volume specials filter is the template); (3) fallback language hard-coded before scale -- ARRC spent five years on fallback language and still needed Congressional legislation for tough-legacy contracts. ISFR should publish ISDA-style fallback templates from day one.

The VIX model teaches the venue-plus-benchmark vertical integration moat. Cboe owns both the SPX/SPXW options market and the VIX index trademark. VIX itself is uninvestable; all monetization happens at the futures, options, and ETP layer -- VIX futures (launched March 2004, approximately $6.1 billion monthly notional), VIX options (February 2006, over $3.1 billion August 2024 monthly), and volatility ETPs (UVXY at $290 million, SVIX, VXX, the post-Volmageddon SVXY at -0.5x). Cboe's 2003 redesign -- making VIX a model-free static-replication recipe rather than a Black-Scholes output -- is what made dealers able to hedge it. ISFR's methodology must be similarly replicable: a market maker should be able to construct a hedging portfolio from on-chain primitives.

ICE Benchmark Administration (IBA) is the corporate-structure template. IBA is separately incorporated (England company number 08457573), independently capitalized, with a majority-independent board, per-benchmark Oversight Committees, and externally-reviewed Statement of Compliance audited by EY. It administers the residual LIBOR (synthetic USD ceased September 30, 2024), LBMA Gold/Silver, and ICE Swap Rate. Even though owned by ICE, IBA is structurally walled off. This is exactly the pattern ISFR must adopt.

---

## 2. ISFR-YBS: The Narrow Wedge Product

### Why yield-bearing stablecoins are the right starting point

Praneeth Srikanti's other critical feedback was "narrow before broad" -- a focused index strategy over ambitious generalization. Five index candidates were scored across data availability, trust requirements, existing demand, competitive gap, and strategic fit (1--5 each, 25 maximum):

| Candidate | Data | Trust | Demand | Gap | Fit | Total |
|---|---|---|---|---|---|---|
| **Stablecoin yield (YBS)** | 4 | 3 | **5** | **5** | 3 | **20** |
| DeFi lending rates | **5** | **5** | 3 | 2 | 4 | 19 |
| LST/LRT yield | 4 | 3 | 4 | 3 | **5** | 19 |
| Perp funding | **5** | 3 | 4 | 2 | 3 | 17 |
| AI compute pricing | 3 | 2 | 4 | 2 | **5** | 16 |

ISFR-YBS (Yield-Bearing Stablecoin Reference Rate) scores highest on the two dimensions Praneeth weights most: existing demand and competitive gap. The yield-bearing stablecoin sector projects $50 billion or more in supply by end of 2026. **No regulated administrator publishes a benchmark for this segment.** DeFiLlama Yields and CoinGecko's category page are retail-grade and explicitly disclaim institutional use.

The current notable issuers and their scale:

- **Sky sUSDS:** approximately $5 billion market cap, 3.75--4.5% SSR
- **Ethena sUSDe:** USDe approximately $5.92 billion (Q1 2026), sUSDe APY approximately 3.72% (Messari)
- **Maple syrupUSDC:** approximately $2.6 billion
- **Ondo USDY:** $1.32 billion at approximately 3.69--5% yield; OUSG approximately $1.1 billion at approximately 3.75%
- **Aave aUSDC:** approximately $3.4 billion pool
- **Ethena USDtb:** $1.83 billion (BUIDL-backed)
- **Additional constituents:** Mountain USDM, Frax sFRAX, Usual USD0/USD0++, Agora AUSD

The branding narrative is "the MMF benchmark for crypto dollars" -- analogous to iMoneyNet or Crane Index in TradFi. Distribution is already happening at scale: Stripe/Privy plus sUSDS is pushing into 110 million wallets and 2,000 apps as of March 2026.

### Methodology design

The methodology is a supply-weighted composite with risk-tiered sub-indices:

- **ISFR-YBS-T** (T-bill / RWA-backed): USDY, OUSG, USD0, USDtb, RWA-portion of sUSDS
- **ISFR-YBS-L** (Lending-based): aUSDC, aUSDT, syrupUSDC, sFRAX
- **ISFR-YBS-D** (Delta-neutral / basis): sUSDe, Falcon, Resolv, Solv basis products
- **ISFR-YBS-S** (Savings-rate composites): sUSDS, sDAI

**Inclusion criteria:** ERC-4626 or equivalent standard, greater than $50 million sustained supply, public proof-of-reserves, redemption window under 14 days, two reputable audits. Daily fixing at 16:00 UTC plus rolling 7-day and 30-day series, with both gross and net-of-fees rates published. Per-asset constituent caps at 40% to prevent single-issuer dominance. Same-block flash-loan distortions filtered via EMA on debt levels. Published delisting protocol with 30-day notice.

### What was explicitly deferred and why

**LRT/staking:** Despite the highest strategic-fit score, Treehouse plus FalconX have a 6--12 month head start on staking-rate forward rate agreements (FRAs) with named institutional counterparties (Edge Capital, Mirana, Monarq). The April 2026 Kelp/rsETH exploit ($292 million drained, $236 million in cascaded bad debt across Aave, Compound, and Euler) makes LRT inclusion-criteria the most adversarial design problem in DeFi. Deferred to Phase 2 once methodology is battle-tested on YBS.

**Perp funding:** Coinglass, Laevitas, Velo, CoinAPI, Amberdata, Block Scholes, and CF Benchmarks KFRI all publish data products in this space. Ethena's perp backing of USDe fell from approximately 93% to approximately 11% of collateral over 2025, weakening the hero use-case. Deferred.

**AI compute pricing:** Silicon Data already publishes a Bloomberg-distributed daily H100/A100 index, backed by DRW and Jump Trading, claiming 80%+ coverage. SemiAnalysis publishes a ClusterMAX H100 1-year rental index. SF Compute raised a Series A at $300 million post-money in November 2025. Phase 3 candidate at the earliest.

---

## 3. ISFR-Lend.USDC: The Companion Product

The second product is ISFR-Lend.USDC -- a borrow-volume-weighted composite stablecoin lending rate across the major DeFi lending protocols:

- **Aave V3:** approximately $20--26 billion TVL (post-Kelp event)
- **Morpho Blue:** approximately $5.8--10 billion TVL
- **SparkLend:** approximately $2--3.4 billion TVL
- **Compound V3:** approximately $1.3 billion TVL
- **Fluid:** approximately $0.75--1 billion TVL

Aggregate DeFi lending TVL is approximately $75--80 billion per Coinbase/Eco data. The data is trivially machine-readable: `getReserveData()` on Aave/Spark forks, `getBorrowRate(utilization)` on Compound Comet, Morpho Blue per-market reads. This gives a 5/5 score on both data availability and trust dimensions.

The competitive gap here is shallower than YBS because IPOR exists, but IPOR's commercial pivot to a vault aggregator (see Section 4) demonstrates that publishing the index alone did not monetize. ISFR-Lend.USDC paired with YBS as the headline gives Pendle, Notional Exponent, Term Finance, Maple's institutional desk, and Gauntlet/Chaos Labs a clean reference rate and forms a complete "DeFi cost-of-capital" dashboard.

The framing is deliberate: ISFR-YBS is "the MMF benchmark for crypto dollars" while ISFR-Lend.USDC positions as "SOFR for DeFi credit." Together they make the methodology platform reusable and the institutional story complete.

---

## 4. Competitive Landscape

The credible-crypto-benchmark space is smaller than it looks. Only five entities clear the IOSCO + BMR + audited-oversight bar: CF Benchmarks (FCA, KPMG-audited, Kraken-owned), Bloomberg BISL, Compass Financial Technologies (now MSCI-owned, AMF-regulated), Vinter (now Kaiko, ESMA-regulated), and MarketVector (BaFin-regulated). None publishes a yield-bearing stablecoin benchmark. None publishes a cross-venue DeFi lending rate index.

### IPOR / Fusion

IPOR rebranded to Fusion in May 2025 (IPOR to FUSN 1:1 token swap; Fusion DAO; vault-aggregator pivot), effectively conceding the benchmark category. TVL stands at approximately $14 million unleveraged / $60 million total value managed -- 0.3% of Veda's scale. Over $4 billion in cumulative IRS volume was reported in 2023, but there has been no recent disclosure. The only funding round was a $5.55 million seed in April 2022; no fresh capital since. The pivot to vaults concedes the benchmark category entirely, and there is no yield curve product. Founder: Darren Camas. Status: possible acqui-hire target given the Fusion pivot and lack of capital.

### Treehouse / TESR

Treehouse raised at a $400 million valuation in April 2025 (approximately $20 million total funding: $18 million seed in March 2022 plus strategic round at $400 million valuation in April 2025). TREE token hit $610 million peak TVL at TGE but trades 94% below all-time high. Current TVL has fallen to $157 million. Their DOR (DeFi Offered Rate) remains a single-feed product -- TESR (ETH staking yield) only. The panelist set is small enough to invite LIBOR-collusion criticism. They use panelist-submitted forecasts with consensus methodology, a centralized "Operator" structure, and a Treehouse DAO post-TGE. FalconX FRA pilots are live with named institutional counterparties (Edge Capital, Mirana, Monarq), and there is CESR (CoinDesk Indices) integration. CEO: Brandon Goh.

### CF Benchmarks

CF Benchmarks is FCA-regulated (FRN 847100), KPMG ISAE 3000-audited, and owned by Kraken (acquired via Crypto Facilities in 2019, nine-figure deal). CEO: Sui Chung. They publish BIRC (Bitcoin Interest Rate Curve), KFRI (Kraken-only funding rate index), AUIRR (Aggregate Uncollateralized Interest Rate Risk), and a USDT IRC. Six of eleven spot BTC ETFs reference CF Benchmarks rates. Over $40 billion in referenced AUM. Approximately 99% of regulated derivatives reference their rates.

The critical limitation: every CF rate is off-chain-administered, on-chain-distributed. None aggregates cross-venue DeFi borrow data the way SOFR aggregates tri-party repo. CF Benchmarks has no DeFi lending or YBS product.

### ISFR's positioning

| Dimension | IPOR (Fusion) | Treehouse (TESR) | CF Benchmarks | ISFR (proposed) |
|---|---|---|---|---|
| Product | Vault aggregator (pivoted from benchmark) | Panel-consensus staking rate + tAssets + FRAs | FCA-regulated CeFi reference rates | DeFi-native YBS + Lending composites |
| Methodology | Block-over-block weighted average | Panelist-submitted forecasts | Multi-venue trade aggregation (off-chain) | Volume-weighted median, on-chain transactions |
| TVL/Notional | ~$14M / ~$60M TVM | $157M (was $610M peak) | >$40B referenced AUM | Target $5B Phase 2, $20B Phase 3 |
| Governance | Fusion DAO | Treehouse DAO + centralized Operator | FCA-regulated + KPMG + internal committee | Independent IOC, ARRC-style, FCA Cat-6 |
| Token | FUSN (1:1 from IPOR) | TREE (94% below ATH) | None (private, Kraken-owned) | None at admin layer |
| Regulation | None | None | UK FCA authorized | UK BMR Cat-6 (planned) |
| Coverage | USDT/USDC/DAI/stETH spot rates | ETH staking only | CeFi/derivatives-implied only | YBS (greenfield) + DeFi lending |

**The wedge is the intersection:** regulated administrator status (CF's strength, neither IPOR nor Treehouse has it) plus on-chain DeFi-native methodology (IPOR's original strength, CF does not have it) plus multi-asset coverage starting with the YBS gap (Treehouse only does staking). Each incumbent owns one corner; ISFR positions in the center.

The branding discipline is critical: avoid the "DeFi LIBOR" tagline that Treehouse markets heavily but has only delivered for staking. ISFR's framing is "MMF benchmark for crypto dollars, then SOFR for DeFi credit" -- concrete, narrow, and uncontested.

---

## 5. Governance Architecture

### Regulatory framework

ISFR is governed by the UK Benchmarks Regulation (BMR), not MiCA. MiCA Title V covers CASP services, not benchmarks; there is no MiCA reference-rate regime. This is strategically positive: BMR is mature regulation, and CASPs that consume ISFR rates absorb MiCA's compliance costs while CASP white papers must disclose reference-rate methodologies -- a built-in distribution channel for any BMR-compliant rate.

**UK BMR Cat-6** is the authorization category for significant interest-rate benchmarks. This is the same path CF Benchmarks took in 2019--2020. The EU BMR's "non-significant" tier is being removed from scope January 1, 2026 -- interest-rate benchmarks are automatically Cat-6 "significant," which narrows the competitive field rather than reducing ISFR's compliance burden.

### IOSCO 19 Principles

IOSCO compliance is a 19-principle, approximately GBP 300,000/year line item, not a strategic question. All 19 principles must be addressed via published policies:

Overall responsibility, oversight of third parties, conflicts management, control framework, internal oversight, design, data sufficiency, hierarchy of inputs, transparency, periodic review, methodology content, methodology change, transition, submitter code of conduct, internal data controls, complaints, audits, audit trail, and regulatory cooperation.

The audit trail must retain inputs, calculations, and decisions for five years (an inherent on-chain advantage for ISFR). MSCI adopted IOSCO voluntarily in 2014, four years before EU BMR. ISFR should publish an Adherence Statement on day one, with KPMG (CF Benchmarks's current auditor) or PwC (Argus's 14-year auditor) providing ISAE 3000 assurance.

### ARRC translation matrix and Independent Oversight Committee

The governance design adopts the ARRC tripartite structure verbatim: administrator (the ISFR entity), sponsoring/endorsing body (the IOC), and official-sector ex-officio observers. The proposed IOC composition:

1. **Independent academic chair:** Maurice Herlihy at Brown University is the strongest single candidate given his cross-chain atomic-commit and "adversarial commerce" work. Will Knottenbelt at Imperial College IC3RE is the natural UK alternative given the FCA-jurisdiction strategy.
2. **Lending-protocol seat:** Aave Labs plus Sky, post the BGD Labs / ACI wind-down through July 2026.
3. **Stablecoin-issuer seat:** Guy Young (Ethena Labs) or Rune Christensen (Sky/MakerDAO).
4. **LST/staking seat:** Lido via cyber-Fund's Vasiliy Shapovalov.
5. **Institutional/custody seat:** Coinbase Institutional, Anchorage Digital (Diogo Monica), or Fireblocks (Michael Shaulov).
6. **Big-4 audit seat (non-voting):** KPMG or PwC.
7. **Official-sector observers (non-voting, ARRC-style):** FCA Innovation Hub, CFTC LabCFTC, SEC FinHub, ESMA.
8. **End-user seat:** Pendle's TN (co-founder) as the largest natural index licensee.

All members under a Terms of Reference plus antitrust attestation modeled on the published ARRC TOR. Rotating seats with public charter, supermajority requirements for methodology changes, public consultation period of minimum 30 days for any material change.

### Corporate structure

Mirror IBA's structure with separate incorporation:

- **Nunchi ISFR Ltd.** (UK-domiciled, FCA-authorized, holds methodology IP and Part 4A permission for administering a benchmark)
- **Nunchi Inc.** (Delaware C-Corp, commercial sales and US distribution)
- **Korai Foundation** (chain governance, structurally separate from the benchmark administrator)

This is the same pattern CF Benchmarks (UK Ltd, Kraken-owned, US ops) and Bloomberg BISL use. Independent capitalization at the benchmark entity level is required by BMR and is the structural prerequisite for institutional credibility. No admin-entity employees may hold positions referencing the index.

### Regulatory standards bodies to engage

- **ISDA** (Digital Asset Derivatives Definitions Settlement Price Source Matrix -- the institutional whitelist for crypto rate sources; apply for inclusion as a Phase-1 priority)
- **GBBC** (PTDL Group, GSMI 7.0, Risk Mitigation Framework cohorts already include Kaiko, Dfns, Droit, Blockmosaic, Metrika)
- **ISO/TC 307** (chair Scott Farrell, secretariat Standards Australia; engage via national mirror committee -- ANSI in US, BSI in UK)

---

## 6. Phase-Gated Credibility Roadmap

### Phase 1: Narrow Launch and FCA Pre-Application (Months 0--9)

**Months 0--3:** Incorporate UK Ltd. and Delaware C-Corp. Engage UK regulatory counsel (Linklaters, Clifford Chance, Travers Smith, or specialists Bovill / Complyport). Draft Terms of Reference plus Antitrust Guidelines using the ARRC template.

**Months 1--6:** Author full ISFR-YBS methodology rulebook. Recruit Independent Oversight Committee with a named independent chair.

**Months 3--9:** Draft IOSCO 19-Principle adherence statement. Big-4 readiness review (KPMG given CF Benchmarks precedent). Build complaints, whistleblowing, and breach systems. FCA pre-application meeting. Submit Part 4A application via Connect for Cat-6 administrator of significant interest-rate benchmark.

**Phase 1 milestones:**
- 5 named data attestors signed (Aave, Sky, Ethena, Maple, Ondo at minimum)
- Public methodology paper
- Weekly publication of ISFR-YBS
- 1 institutional licensee signed (Pendle is the most natural target)
- $500 million of yield-bearing stablecoin supply attributing to ISFR-YBS as a reference

**External spend:** approximately GBP 40--100K legal + GBP 100--200K methodology/governance + GBP 150--300K IOSCO preparation + approximately GBP 10K FCA application fee.

### Phase 2: Composite Expansion, First Audit, First Derivatives (Months 9--24)

FCA Q&A process and 90-working-day determination clock. CF Benchmarks took approximately 12 months end-to-end in 2019--2020. Launch ISFR-Lend.USDC as the second composite. Convene a private DeFi Rates Committee with NY Fed-style ex-officio observers (FCA Innovation Hub, CFTC LabCFTC, SEC FinHub). First ISAE 3000 reasonable-assurance examination; publish on website (CF Benchmarks/Argus precedent). Apply for ISDA SPS Matrix inclusion.

**Phase 2 milestones:**
- FCA authorization granted
- KPMG or PwC ISAE 3000 published
- 10+ data attestors signed
- First Pendle PT integration referencing ISFR
- First fixed-rate lending protocol (Term Finance, Notional Exponent) referencing ISFR for floating-leg settlement
- $5 billion or more in product TVL referencing ISFR rates

**External spend:** GBP 200K FCA advisor + GBP 150--300K first audit + GBP 100K convening + GBP 250K compliance staff (2 FTE).

### Phase 3: Derivatives Market, EU Recognition, Licensing Revenue (Months 24--36)

ISFR-LRT and ISFR-Funding launched as Phase-3 composites once methodology has been battle-tested on YBS and Lend. File for ESMA recognition as third-country administrator -- CME/CBA achieved ESMA recognition in April 2026, which is the precedent. The third-country transitional regime has been extended to December 31, 2030, providing a clear window. This is what unlocks EU institutional users via the recognition/endorsement route.

License rate to a DCM/MTF for futures listing via CFTC Part 40 self-certification (CME's BRR is the playbook). License to ETF/ETP issuers. Korai-native ISFR perp futures and ISFR-referenced fixed-receivers go live.

**Phase 3 milestones:**
- ESMA recognition granted
- 3+ derivatives venues listing ISFR-referenced products
- $20 billion or more in referenced notional
- First AUM-linked licensing revenue
- ISDA SPS Matrix inclusion

**Total pre-revenue regulatory spend Years 1--2: $1.5--3.0 million.** Recurring compliance approximately $500,000 to $1 million per year thereafter. Statutory FCA determination is 90 working days, but realistic end-to-end timeline is 12--18 months.

---

## 7. Revenue Model

### Pricing structure: bps on referenced notional, not flat fees

The index industry's standard playbook applies directly. CME's Term SOFR program runs 7,000+ licenses to 1,800+ firms underpinning $2.6 trillion in loans and $660 billion in derivative hedges. MSCI's Index segment runs at approximately 76% adjusted EBITDA margin because the marginal cost of an additional licensee is near zero.

### Four-tier revenue stack

**Tier 1 -- AUM/notional licensing (the toll booth):** 0.5--3 basis points on yield-bearing stablecoin supply attributing ISFR-YBS as published reference. 0.5--2 basis points on notional outstanding of fixed-rate lending products (Term Finance, Notional Exponent), Pendle PT/YT markets, and structured products. At Phase-3 target of $20 billion referenced notional, this generates **$10--60 million per year** at index-industry margins.

**Tier 2 -- Recurring data subscriptions:** Institutional API tier at $25,000--100,000 per year per protocol risk engine, fund, or trading desk. Terminal-redistribution licenses to Bloomberg, Refinitiv, Kaiko, Coin Metrics, and Amberdata. Tiered public API access. Comparable: CF Benchmarks, Kaiko, and Coin Metrics all run six-figure-per-client institutional tiers.

**Tier 3 -- Derivatives venue revenue share:** For Korai-native ISFR perpetuals and fixed-receivers, capture exchange fee share following Cboe/CME precedent. VIX licensing economics are partially captured at the futures venue layer.

**Tier 4 -- Methodology/branding licensing:** Trademark licensing to ETP issuers under BMR's "commercial and reasonable" (FRAND) standard.

### What to avoid

Token-based admin revenue. TREE's 94% drawdown and FUSN's commercial pivot demonstrate that token-funded benchmark businesses lose institutional credibility. Keep ISFR Ltd. as a clean pure-software/data B2B entity. Any tokenization should sit at the Korai-chain or agent layer, structurally separate from the administrator. The administrator entity should have no token and no DAO governance -- this is the institutional credibility posture.

---

## 8. Partnership Target List: Phase-1 Priorities

The ten highest-leverage Phase-1 outreach targets, prioritized by data quality, distribution leverage, and regulatory legitimacy:

**1. Aave Labs** -- Stani Kulechov (CEO). Engage successor DAO service providers post-BGD/ACI wind-down via governance.aave.com. Largest underlying borrow-rate source (approximately $45 billion TVL). Without Aave, no credible "DeFi LIBOR."

**2. CF Benchmarks (Kraken)** -- Sui Chung (CEO). Either license CF infrastructure under FRN 847100 as a co-branded product or position as long-term acquirer (Crypto Facilities to Kraken precedent was nine-figures).

**3. Lido / cyber-Fund** -- Vasiliy Shapovalov and Konstantin Lomashuk via cyber.fund. LST yield is the floor rate for ETH-denominated fixed income. cyber-Fund is also a $100 million VC arm -- possible investment partner.

**4. Pendle Finance** -- TN (co-founder), via forum.pendle.finance. Largest yield-trading protocol ($1.96 billion TVL Q1 2026, $69.8 billion cumulative settled yield, approximately 90% category share). Boros funding-rate market and YBS PT/YT integrations make Pendle the most natural Phase-1 licensee.

**5. CME Group / CBA** -- Giovanni Vicioso (Global Head Crypto Products), Sean Tully (Financial and OTC). Listing path via Part 40 self-certification. CBA is the FCA-authorized administrator running Term SOFR -- direct precedent.

**6. ISDA Digital Assets Legal and Documentation Working Group** -- Katherine Tew Darras (General Counsel). SPS Matrix inclusion gates institutional adoption.

**7. Imperial College IC3RE** -- Will Knottenbelt (w.knottenbelt@imperial.ac.uk). UK academic legitimacy. Three-year sponsored-research model proven (Blockchain.com partnership). Aligns with FCA-jurisdiction strategy.

**8. Treehouse Finance** -- Brandon Goh (CEO). Operates competing DOR/TESR with FalconX FRA pilots and CESR integration. Partner-or-compete decision point (see Section 10 below).

**9. Hyperliquid Labs + Ethena Labs** -- Jeff Yan (Hyperliquid), Guy Young (Ethena). Funding-rate component data. sUSDe is a YBS-D constituent and Hyperliquid is the dominant on-chain perp DEX with hourly funding settlements.

**10. Bloomberg Index Services (BISL)** -- Index Relationship Manager team. Bloomberg Terminal distribution is the institutional default. BISL is itself UK BMR-regulated and routinely whitelabels compliant rates.

### Adjacent Phase-2 priorities

Morpho (Paul Frambot), Sky/MakerDAO (Rune Christensen), MarketVector (Martin Leinweber, BaFin-regulated, powers Coinbase 50 Index), Compass FT/MSCI (Pierre Kahn -- possible white-label partner now under MSCI), Maurice Herlihy at Brown for IOC chair, Stanford CBR's Joe Grundfest for regulatory advisory (former SEC commissioner), Coinbase Institutional (Greg Tusar), Anchorage Digital (Diogo Monica), Fireblocks (Michael Shaulov), and IPOR Labs (Darren Camas -- possible acqui-hire given the Fusion pivot and lack of fresh capital since April 2022).

---

## 9. Risk Register

### Risk 1: Manipulation (LIBOR scandal lessons)

Five governance failures enabled $9 billion in LIBOR fines: submission-based not transaction-based, the BBA was an industry trade group not an independent regulated entity, traders sat next to submitters, no audit trail, and panel banks had direct economic incentive to lie.

**Mitigation:** ISFR is transaction-anchored from day one (no panelist submissions). Independent UK Ltd. with majority-independent IOC. Cryptographic audit trail on Korai with five-year retention (structurally trivial on-chain). No admin-entity employees may hold positions referencing the index. Volume-weighted median (not mean) is empirically more manipulation-resistant. SOFR-style 20% specials filter handles outliers. Two-level aggregation (intra-class TVL-weighted median, then inter-class weighted sum) provides independent Byzantine fault tolerance at each layer. An attacker must corrupt 50%+ of source weight AND 50%+ of validator stake simultaneously; either layer alone stops the attack.

### Risk 2: Data quality

The Mango Markets exploit (October 2022, $117 million, MNGO pumped 2,300% within minutes; Pyth and Switchboard reported the manipulated price faithfully) and the Kelp/rsETH bridge drain (April 18, 2026, $292 million plus $236 million cascaded bad debt) demonstrate that even technically-correct oracles fail when underlying constituents are manipulated.

**Mitigation:** Hard inclusion criteria (greater than $50 million sustained supply, greater than 6-month live history, two reputable audits, public proof-of-reserves). Per-asset constituent caps at 40%. Same-block flash-loan distortions filtered via EMA on debt levels. Published delisting protocol with 30-day notice. 3-sigma outlier exclusion on validator submissions with recomputed median on filtered set. Source-level liveness timeouts (120 seconds for lending protocols, 24 hours for structured products, 30 minutes for staking).

### Risk 3: Governance capture

Treehouse's small panelist set is a structural risk. ARRC's 400+ firm participation is the antidote.

**Mitigation:** Rotating IOC seats with public charter. Supermajority requirements for methodology changes. Antitrust attestations modeled on ARRC's published TOR. Public consultation period of minimum 30 days for any material change. ARRC-style ex-officio regulator observers (FCA, CFTC, SEC, ESMA -- non-voting).

### Risk 4: Regulatory action

ISFR is BMR-regulated; the regulatory risk is failure to authorize, not adverse action against an authorized entity. CF Benchmarks's nine-figure acquisition by Kraken was partly a function of FRN 847100 being valuable.

**Mitigation:** Pre-application engagement with FCA Innovation Hub (and CFTC LabCFTC, SEC FinHub for US dialogue). Over-budget on legal and compliance in Year 1. Engage Big-4 (KPMG precedent) early for ISAE 3000 readiness review before formal application. EU BMR's removal of the "non-significant" tier from January 2026 narrows the competitive field while confirming ISFR's classification as a Cat-6 significant benchmark.

### Risk 5: Methodology contestation

Every published benchmark faces methodology critique; the question is whether the response process is institutionally credible.

**Mitigation:** Minimum annual methodology review per IOSCO Principle 10. Published change procedure with consultation per Principle 12. Named methodology committee with public minutes (redacted). Pre-announced rebalance and inclusion windows. Adversarial stress testing of the methodology itself. The methodology committee -- public rules, named members, change-control logs, advance-notice rebalance windows -- is itself the moat asset, not merely a compliance cost.

---

## 10. The Treehouse Partner-or-Compete Decision

Treehouse Finance is the most strategically ambiguous entry on the partnership list. They are simultaneously the closest competitor and the most valuable potential partner. The analysis strongly favors partnership.

### The case for partnership

**Complementary coverage, not overlapping:** TESR covers ETH staking yield exclusively. ISFR-YBS covers yield-bearing stablecoins. ISFR-Lend covers DeFi lending rates. A joint methodology between ISFR (lending + YBS) and TESR (staking) is materially more credible than either alone to institutional buyers who need complete DeFi rate coverage. The combined product would span all four primary yield generation mechanisms in DeFi: lending, staking, structured yield, and funding rates.

**Treehouse is weakened and ISFR has leverage:** TREE token trades 94% below all-time high. TVL has fallen from $610 million peak to $157 million. The panelist set is small enough to invite LIBOR-collusion criticism. Their DOR remains single-feed. A partnership offer from a team pursuing FCA authorization provides Treehouse with regulatory credibility they cannot achieve independently, while ISFR gains access to their FalconX FRA relationships (Edge Capital, Mirana, Monarq) and existing CESR integration.

**Head-to-head on staking is a losing fight:** Treehouse and CoinDesk Indices (CESR) already own the institutional staking-yield lane. Competing directly would require replicating 6--12 months of work with named institutional counterparties while diluting focus from the greenfield YBS opportunity.

**The combined story is more fundable:** For VCs like Praneeth Srikanti, a partnership narrative between ISFR and TESR -- "complete DeFi rate coverage under regulated methodology" -- is a stronger investment thesis than "we compete with Treehouse on one dimension while trying to build the others from scratch."

### The risk of partnership

Treehouse could use the partnership to extract regulatory legitimacy while retaining commercial control of the staking lane. The mitigation is structural: ISFR Ltd. must hold the BMR administrator authorization and methodology IP. Treehouse participates as a data provider and IOC member, not as a co-administrator. The corporate structure (separately incorporated Nunchi ISFR Ltd.) makes this boundary enforceable.

### Recommendation

Engage Brandon Goh (CEO) for a joint methodology discussion covering distinct lanes. Frame the conversation around what neither party can achieve alone: Treehouse cannot get FCA authorization at their current scale and governance maturity; ISFR cannot replicate their institutional staking-rate relationships. A "TESR + ISFR" composite methodology under a single FCA-authorized administrator is the strongest possible product for the institutional market. TREE's 94% drawdown and declining TVL mean the partnership window is open now but may close if Treehouse either recovers or is acquired.

---

## Appendix: Key Figures Referenced

| Entity/Person | Role/Context |
|---|---|
| Praneeth Srikanti | Partner, Emergent Ventures. Primary VC interlocutor. |
| John Doherty | CEO, Nunchi |
| Will Pankiewicz | Engineering, Nunchi |
| Jacob Gadikian | Chain Engineering, Nunchi (Korai) |
| Sui Chung | CEO, CF Benchmarks |
| Stani Kulechov | CEO, Aave Labs |
| Brandon Goh | CEO, Treehouse Finance |
| Guy Young | Founder, Ethena Labs |
| Rune Christensen | Founder, Sky/MakerDAO |
| Vasiliy Shapovalov | Lido / cyber-Fund |
| Konstantin Lomashuk | cyber-Fund ($100M VC arm) |
| TN | Co-founder, Pendle Finance |
| Jeff Yan | Founder, Hyperliquid Labs |
| Giovanni Vicioso | Global Head Crypto Products, CME Group |
| Sean Tully | Financial and OTC, CME Group |
| Katherine Tew Darras | General Counsel, ISDA |
| Maurice Herlihy | Brown University (IOC chair candidate) |
| Will Knottenbelt | Imperial College IC3RE (UK IOC chair alternative) |
| Joe Grundfest | Stanford CBR, former SEC Commissioner |
| Martin Leinweber | MarketVector (BaFin-regulated) |
| Pierre Kahn | Compass FT / MSCI |
| Paul Frambot | Founder, Morpho |
| Greg Tusar | Coinbase Institutional |
| Diogo Monica | Anchorage Digital |
| Michael Shaulov | Fireblocks |
| Darren Camas | IPOR Labs / Fusion |
| Scott Farrell | Chair, ISO/TC 307 |

| Financial Reference | Figure |
|---|---|
| Global interest rate derivative notional | ~$668 trillion |
| DeFi lending TVL | ~$49.5 billion |
| On-chain IR derivative TVL | <$100 million |
| Index industry annual revenue (SPDJI + MSCI + FTSE Russell) | >$4.5 billion |
| MSCI Index segment EBITDA margin | ~76% |
| SPY licensing revenue to S&P DJI | ~$120 million/year |
| US passive fund AUM | ~$19.1 trillion (Oct 2025) |
| Bloomberg acquisition of Barclays index business | $781 million (Aug 2016) |
| Yield-bearing stablecoin addressable supply | $50 billion+ (projected YE 2026) |
| Phase-3 referenced notional target | $20 billion+ |
| Phase-3 revenue range (0.5--3 bps on $20B) | $10--60 million/year |
| Year 1--2 regulatory spend | $1.5--3.0 million |
| Recurring compliance (post-authorization) | $500K--1M/year |
| FCA application fee | ~GBP 10,000 |
| IOSCO compliance annual cost | ~GBP 300,000/year |
| ISAE 3000 assurance annual cost | ~GBP 150--300K/year |
| Kelp/rsETH exploit (April 2026) | $292M drained + $236M cascaded |
| Mango Markets exploit (October 2022) | $117 million |
| LIBOR scandal total fines | $9 billion+ |
| CME Term SOFR licenses | 7,000+ licenses, 1,800+ firms |
| CME Term SOFR loan notional | $2.6 trillion |
