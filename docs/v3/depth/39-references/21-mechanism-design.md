# 39-21 Mechanism Design -- Annotated Reference Map

> Research foundations for auction theory, incentive-compatible allocation,
> token economics, and agent marketplace design in Roko's VCG attention
> auction and economic subsystems.
>
> **v3 depth file** -- updated 2026-09-15. Added Duetting et al. (2024).

---

## VCG Mechanism

**[Vickrey, 1961]** *Counterspeculation, Auctions, and Competitive Sealed Tenders.* Journal of Finance, 16(1), 8--37.
Second-price auction guaranteeing truthful bidding. Foundational for the VCG attention auction: context sections bid for inclusion, and the mechanism ensures truthful valuation reporting.

**[Clarke, 1971]** *Multipart Pricing of Public Goods.* Public Choice, 11(1), 17--33.
Multi-item truthful pricing extending Vickrey to multiple goods. Enables simultaneous context section allocation -- multiple sections compete for limited context budget.

**[Groves, 1973]** *Incentives in Teams.* Econometrica, 41(4), 617--631.
Truthful revelation in teams. Subsystems (NeuroStore, task context, research context) truthfully reveal their context section valuations under VCG.

**[Milgrom, 2004]** *Putting Auction Theory to Work.* Cambridge University Press.
Comprehensive applied auction theory. Foundation for efficient VCG implementation and practical auction design in context allocation.

---

## Mechanism Design for Large Language Models (2024)

**[Duetting et al., 2024]** *Mechanism Design for Large Language Models.* ACM WWW Best Paper 2024. arXiv:2310.10826.
Token auction model for multi-LLM output generation. Multiple LLMs "bid" for tokens in generated output. Extends VCG to token granularity, showing that mechanism design principles apply at the finest level of LLM interaction. Directly informs the multi-provider dispatch architecture where providers compete for task allocation.

---

## Submodular Optimization

**[Nemhauser, Wolsey & Fisher, 1978]** *An Analysis of Approximations for Maximizing Submodular Set Functions.* Mathematical Programming, 14(1), 265--294.
Greedy submodular maximization with (1-1/e) approximation guarantee. Context selection is submodular: adding a context section has diminishing marginal value. The greedy VCG fallback achieves provably near-optimal context assembly.

---

## Algorithmic Game Theory

**[Nisan et al., 2007]** *Algorithmic Game Theory.* Cambridge University Press.
Computational aspects of game theory. Provides efficient VCG algorithms and analysis of computational mechanism design trade-offs.

---

## Dynamic Rating Systems

**[Glickman, 1999]** *Parameter Estimation in Large Dynamic Paired Comparison Experiments.* J. Royal Statistical Society C, 48(3), 377--394.
Glicko-2 dynamic rating with reliability intervals. Grounds reputation tracking in the Korai identity system -- each agent has a time-varying rating with confidence bounds.

**[Nasrulin et al., 2022]** *MeritRank: Sybil Tolerant Reputation.* arXiv:2207.09950.
Sybil-tolerant reputation system using network structure to resist fake identity attacks. Informs on-chain reputation design.

---

## Identity and Token Economics

**[Weyl, Ohlhaver & Buterin, 2022]** *Decentralized Society: Finding Web3's Soul.* SSRN.
Soulbound tokens as non-transferable identity. Grounds the Korai Passport: each agent's identity token is non-transferable, encoding capabilities and reputation.

**[Gesell, 1916]** *The Natural Economic Order.* 1916.
Demurrage: money that decays to encourage circulation. KORAI's 1% annual demurrage mirrors knowledge half-life -- currency that loses value over time encourages active use rather than hoarding.

---

## Commons Governance

**[Ostrom, 1990]** *Governing the Commons.* Cambridge University Press.
Commons governance without central authority. Eight design principles for managing shared resources. Informs Agent Group knowledge commons governance -- shared knowledge is a common-pool resource requiring institutional rules.

**[Williamson, 1979]** *Transaction-Cost Economics.* Journal of Law and Economics.
Institutional structures minimize transaction costs. The Agent Mesh reduces knowledge sharing costs by providing structured exchange channels.

---

## Information Economics

**[Bakos & Brynjolfsson, 1999]** *Bundling Information Goods.* Management Science.
Economics of bundling information goods. Informs how knowledge bundles (playbooks, skill libraries) are packaged for sharing.

---

## Agent Marketplace (2025)

**[Yang et al., 2025]** *Agent Exchange: Shaping the Future of AI Agent Economics.* arXiv:2507.03904.
RTB-inspired auction engine for agent task allocation with User-Side Platform, Agent-Side Platform, Agent Hubs, and Data Management Platform. Informs the Artifact Marketplace design in `roko-chain`.

---

## Neural Mechanism Design

**[Tacchetti et al., 2024]** *Deep mechanism design: Learning social and economic policies for human benefit.* PNAS 2024.
RL-trained neural networks create desirable mechanisms. Validates learned allocation mechanisms as alternatives to hand-designed VCG.

**[Curry et al., 2024]** *Automated Mechanism Design Survey.* ACM SIGecom Exchanges, 22(2).
Comprehensive survey of differentiable economics and neural auction design. Maps the landscape of automated mechanism design.

**[Zeng et al., 2025]** *Regularized Proportional Fairness Mechanism for Resource Allocation Without Money.* ICLR 2025. arXiv:2501.01111.
Fair allocation mechanisms. Informs fair context budget allocation across competing subsystems.

---

## Cross-References

- VCG auction in context assembly: [07-context-engineering](./07-context-engineering.md)
- Collective intelligence: [18-collective-intelligence](./18-collective-intelligence.md)
- Protocol standards: [22-protocol-standards](./22-protocol-standards.md)
