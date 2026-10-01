# Market Microstructure and DeFi Theory

> Annotations are kept only for works whose descriptions were checked against the papers (listed in
> `tools/docs_integrity/content_audit.json` and in the banner of docs/v3/REFERENCES.md). The other entries are
> bare citations: their annotations were removed because they had not been checked against the papers.

> **DEPRECATED (v1):** This document is part of the v1 specification and may be outdated. See [../../v2/](../../v2/) for the current reference.


> Academic foundations for automated market making, liquidity provision, vault mechanisms, and DeFi protocol design relevant to Roko's chain domain plugin.

**Topic**: [References](./INDEX.md)
**Prerequisites**: [Architecture](../00-architecture/INDEX.md)
**Key sources**: `bardo-backup/prd/shared/citations.md` §§1-2, §§14-20

> **Implementation**: Reference

---

## Abstract

Roko is domain-agnostic, but its first domain plugin is DeFi. This section collects the market microstructure research that grounds the chain agent's tools and strategies. These citations are chain-domain-specific — they inform the tools in the chain plugin, not the core cognitive architecture. The AMM/LP theory, vault mechanisms, and MEV protection research are preserved here for completeness.

---

## AMM and LP Theory

- Milionis, J., Moallemi, C., Roughgarden, T., & Zhang, A. (2023). Automated Market Making and Loss-Versus-Rebalancing. _Journal of Financial Economics_.

- Adams, H. et al. (2024). UniswapX: Aggregating Automated Market Makers. Uniswap Labs.

- Adams, H. et al. (2025). am-AMM: Auction-Managed Automated Market Maker. Uniswap Research.

- Hasbrouck, J., Rivera, T.J., & Saleh, F. (2025). An Economic Model of a Decentralized Exchange with Concentrated Liquidity. _Management Science_.

---

## Vault Mechanisms

- Ethereum Foundation (2022). ERC-4626: Tokenized Vault Standard. EIPs.

- Ethereum Foundation (2023). ERC-7265: Circuit Breaker Standard. EIPs.

- Ethereum Foundation (2023). ERC-7540: Asynchronous Redemption Vaults. EIPs.

---

## Risk and Decision Theory (Financial)

- Kelly, J.L. Jr. (1956). A New Interpretation of Information Rate. _Bell System Technical Journal_, 35(4), 917-926.

- Roy, A.D. (1952). Safety First and the Holding of Assets. _Econometrica_, 20(3), 431-449.

- Peters, O. (2019). The Ergodicity Problem in Economics. _Nature Physics_, 15(12), 1216-1221.

- Taleb, N.N. (2012). _Antifragile: Things That Gain from Disorder_. Random House.

- Taleb, N.N. & Douady, R. (2013). Mathematical Definition, Mapping, and Detection of (Anti)Fragility. _Quantitative Finance_, 13(11), 1677-1689.

---

## Cross-References

- See [21-mechanism-design.md](./21-mechanism-design.md) for Vickrey auctions and VCG mechanism
- See [22-protocol-standards.md](./22-protocol-standards.md) for ERC standards
- See [11-streaming-algorithms.md](./11-streaming-algorithms.md) for online statistics
