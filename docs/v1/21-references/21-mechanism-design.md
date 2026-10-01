# Mechanism Design and Attention Economics

> Annotations are kept only for works whose descriptions were checked against the papers (listed in
> `tools/docs_integrity/content_audit.json` and in the banner of docs/v3/REFERENCES.md). The other entries are
> bare citations: their annotations were removed because they had not been checked against the papers.

> **DEPRECATED (v1):** This document is part of the v1 specification and may be outdated. See [../../v2/](../../v2/) for the current reference.


> Academic foundations for VCG auctions, reputation systems, incentive-compatible mechanisms, and attention allocation in Roko's context bidding and agent marketplace.

**Topic**: [References](./INDEX.md)
**Prerequisites**: [Architecture](../00-architecture/INDEX.md)
**Key sources**: `bardo-backup/prd/shared/citations.md` §34, `refactoring-prd/09-innovations.md` §II

> **Implementation**: Reference

---

## Abstract

Roko's VCG Attention Auction allocates limited context budget through incentive-compatible bidding. The Vickrey-Clarke-Groves mechanism guarantees truthful bidding: subsystems can't game the auction by inflating bids because they pay the second price. This section collects the mechanism design foundations, including auction theory, reputation systems, and the economic theory of attention.

---

## VCG Mechanism

- Vickrey, W. (1961). Counterspeculation, Auctions, and Competitive Sealed Tenders. _Journal of Finance_, 16(1), 8-37.

- Clarke, E.H. (1971). Multipart Pricing of Public Goods. _Public Choice_, 11(1), 17-33.

- Groves, T. (1973). Incentives in Teams. _Econometrica_, 41(4), 617-631.

---

## Auction Theory

- Milgrom, P. (2004). _Putting Auction Theory to Work_. Cambridge University Press.

- Duetting, P. et al. (2024). Mechanism Design for Large Language Models. _ACM WWW_, 2024. arXiv:2310.10826.

---

## Algorithmic Game Theory

- Nisan, N., Roughgarden, T., Tardos, E., & Vazirani, V.V. (2007). _Algorithmic Game Theory_. Cambridge University Press.

---

## Submodular Optimization

- Nemhauser, G.L., Wolsey, L.A., & Fisher, M.L. (1978). An Analysis of Approximations for Maximizing Submodular Set Functions. _Mathematical Programming_, 14(1), 265-294.

---

## Reputation Systems

- Glickman, M.E. (1999). Parameter Estimation in Large Dynamic Paired Comparison Experiments. _Journal of the Royal Statistical Society, Series C_, 48(3), 377-394.

- Nasrulin et al. (2022). Nasrulin, B. et al. MeritRank: Sybil Tolerant Reputation. arXiv:2207.09950.

- Soulbound Tokens (2022). Weyl, E.G., Ohlhaver, P., & Buterin, V. Decentralized Society: Finding Web3's Soul. _SSRN_.

---

## Vickrey Reputation-Adjusted Auction

- Reputation-adjusted scoring: `s_i = p_i × (1 + (1 - R_i))`. Payment = `s_second / (1 + (1 - R_winner))`.
  *Grounds: Spore/Sparrow marketplace — the Vickrey reputation-adjusted auction for the agent job market. Agents with higher reputation can bid lower and still win, creating positive incentives for reputation building.*

---

## Demurrage and Token Economics

- Gesell, S. (1916). _The Natural Economic Order_.

- Ostrom, E. (1990). _Governing the Commons_. Cambridge University Press.

---

## Knowledge Markets

- Williamson, O.E. (1979). Transaction-Cost Economics. _Journal of Law and Economics_.

- Bakos, Y. & Brynjolfsson, E. (1999). Bundling Information Goods. _Management Science_.

---

## Agent Marketplaces and AI Economies (2024-2025)

- Yang et al. (2025). Agent Exchange: Shaping the Future of AI Agent Economics. arXiv:2507.03904.

- Duetting, P. et al. (2024). Mechanism Design for Large Language Models. _ACM WWW Best Paper_, 2024. arXiv:2310.10826.

- Tacchetti et al. (2024). Learning Social and Economic Policies for Human Benefit. _PNAS_, 2024.

---

## Cross-References

- See [07-context-engineering.md](./07-context-engineering.md) for VCG in context assembly
- See [22-protocol-standards.md](./22-protocol-standards.md) for ERC-8004 and token standards
- See [10-market-microstructure.md](./10-market-microstructure.md) for DeFi-specific mechanisms
