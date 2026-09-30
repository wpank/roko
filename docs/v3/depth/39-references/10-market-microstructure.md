# 39-10 Market Microstructure -- Annotated Reference Map

> Research foundations for automated market making, liquidity provision, vault
> mechanisms, and DeFi protocol design in Roko's chain domain.
>
> **v3 depth file** -- updated 2026-09-15.

---

## AMM and LP Theory

**[Milionis et al., 2023]** *Automated Market Making and Loss-Versus-Rebalancing.* Journal of Financial Economics.
LVR formalizes the dominant cost of passive LP in AMMs. Grounds DeFi agent LP strategies in `roko-chain`.

**[Adams et al., 2024]** *UniswapX: Aggregating Automated Market Makers.* Uniswap Labs.
Dutch auction order protocol for MEV-protected swap execution.

**[Adams et al., 2025]** *am-AMM: Auction-Managed Automated Market Maker.* Uniswap Research.
Winning bidder controls pool fees and captures arbitrage.

**[Hasbrouck, Rivera & Saleh, 2025]** *An Economic Model of a Decentralized Exchange with Concentrated Liquidity.* Management Science.
Formal economic model of concentrated liquidity provision.

---

## Vault Mechanisms

**[ERC-4626, 2022]** *ERC-4626: Tokenized Vault Standard.* Ethereum EIPs.
Industry-standard vault interface for deposit/withdraw/share accounting.

**[ERC-7265, 2023]** *ERC-7265: Circuit Breaker Standard.* Ethereum EIPs.
Rate-limiting mechanism for DeFi protocols. Grounds circuit breaker patterns in `roko-conductor`.

**[ERC-7540, 2023]** *ERC-7540: Asynchronous Redemption Vaults.* Ethereum EIPs.
Async redemptions extending ERC-4626 with request/claim lifecycle.

---

## Risk and Decision Theory

**[Kelly, 1956]** *A New Interpretation of Information Rate.* Bell System Technical Journal, 35(4), 917--926.
Kelly criterion for optimal position sizing. Foundational for financial agent bet sizing.

**[Roy, 1952]** *Safety First and the Holding of Assets.* Econometrica, 20(3), 431--449.
Safety-first portfolio optimization. Grounds safety constraints in financial agents.

**[Peters, 2019]** *The Ergodicity Problem in Economics.* Nature Physics, 15(12), 1216--1221.
Non-ergodic returns require log-wealth maximization (Kelly). Grounds financial agent strategy.

**[Taleb, 2012]** *Antifragile: Things That Gain from Disorder.* Random House.
Systems with convex response improve under volatility. Design principle for robust agents.

**[Taleb & Douady, 2013]** *Mathematical Definition, Mapping, and Detection of (Anti)Fragility.* Quantitative Finance, 13(11), 1677--1689.
Formal definition of fragility as sensitivity to perturbation. Mathematical framework for robustness.

---

## MEV and Execution Quality

**[Daian et al., 2020]** *Flash Boys 2.0: Frontrunning in Decentralized Exchanges, Miner Extractable Value, and Consensus Instability.* IEEE S&P 2020.
MEV extraction creates adversarial execution environments. DeFi agents must account for sandwich attacks, frontrunning, and backrunning. Grounds the need for MEV-protected execution in agent strategies.

**[Flashbots, 2021]** *MEV-Explore: A Public Dashboard for MEV Data.* flashbots.net.
Transparency infrastructure for MEV measurement. Informs agent execution quality monitoring.

---

## Oracle and Price Feed Design

**[Chainlink, 2021]** *Chainlink 2.0: Next Steps in the Evolution of Decentralized Oracle Networks.* Whitepaper.
Decentralized oracle networks providing price feeds to smart contracts. Informs the data feed design for DeFi agent price signals.

---

## Behavioral Finance

**[Shiller, 2000]** *Irrational Exuberance.* Princeton University Press.
Behavioral biases in markets. Agents should not replicate human behavioral biases. Validates the Daimon's calibration-aware affect modulation preventing panic-driven decisions.

**[Kahneman & Tversky, 1979]** *Prospect Theory.* Econometrica, 47(2), 263--292.
Asymmetric loss/gain evaluation in human decision-making. Cross-referenced in [20-cognitive-architectures](./20-cognitive-architectures.md) for Daimon design.

---

## Liquidity and Market Making Theory

**[Kyle, 1985]** *Continuous Auctions and Insider Trading.* Econometrica, 53(6), 1315--1335.
Foundational model of informed trading and market making. Price impact as a function of order flow. Informs agent trade execution sizing.

**[Glosten & Milgrom, 1985]** *Bid, Ask and Transaction Prices in a Specialist Market with Heterogeneously Informed Traders.* Journal of Financial Economics, 14(1), 71--100.
Bid-ask spread as adverse selection cost. Informs how DeFi agents should quote and execute in adversarial environments.

---

## Cross-References

- Auction mechanisms: [21-mechanism-design](./21-mechanism-design.md)
- ERC standards: [22-protocol-standards](./22-protocol-standards.md)
- Online statistics: [11-streaming-algorithms](./11-streaming-algorithms.md)
