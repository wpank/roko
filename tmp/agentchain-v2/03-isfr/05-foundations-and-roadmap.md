# 05 — Theoretical Foundations and Roadmap

> ISFR rests on three research bases that justify specific design choices and bound expected behaviour: (1) information-theoretic limits on coordination, prediction, and memory; (2) time-as-first-class-primitive results from time-series foundation models, hierarchical RL, and metacontroller research; (3) mathematical-discovery results from automated theorem proving and library learning. This document collects the research that grounds these claims with full external citations preserved, and then sequences the V1 → V2 → V3 → V4 evolution, the smallest end-to-end demonstrable loop, the 90-day build plan, the implementation priority matrix, and the risk register.

---

## Part I — Theoretical Foundations

## 1. Information-Theoretic Limits on Multi-Agent Coordination

The single most important result for anyone designing the prediction and attestation layers of a benchmark is **Scott Aaronson's "The Complexity of Agreement"** (STOC 2005; arXiv cs/0406061). Two Bayesian agents who share a common prior can reach ε-agreement on any posterior probability by exchanging O(1/ε²) bits, regardless of how much private information each agent holds.

The amount of communication required depends only on the desired precision of agreement, not on the richness of each agent's internal knowledge. Two agents converging to within ε = 0.05 require ≈400 bits ≈50 bytes.

### Implication for ISFR prediction commits

Per-prediction commitments are hash-locked (`hash(predictedValue || salt)`) — fixed-size regardless of the model behind the prediction. This is not just an anti-front-running measure; it is informationally appropriate: under the Aaronson bound, the entire information needed to reach approximate agreement on the next ISFR value across a population of N agents fits in O(N · 1/ε²) bits. A 32-byte commit per agent is more than sufficient.

Systems that scale coordination bandwidth linearly with per-agent state are over-engineered.

### Collaboration is not free — Peng-Garg-Kleinberg

Peng, Garg, and Kleinberg (arXiv 2411.15230; AAAI 2025) prove a no-free-lunch result for deterministic collaboration strategies: any collaboration strategy that does not always defer to a single agent will, on some input distribution, underperform the least accurate agent in the group.

The escape hatch is **competence-region awareness**. If the collaboration strategy can identify which agent is most competent for a given input region and route to that agent selectively, the NFL theorem does not apply. ISFR's reputation-tier system implements exactly this: the γ discount and clearing-priority mechanisms route economic weight to agents with demonstrated calibration, not via a fixed voting rule that the NFL result would rule out.

### Difference utility — Wolpert's COIN

Wolpert's Collective Intelligence (COIN) framework provides the theoretical foundation for multi-agent credit assignment. Difference Utility (Wonderful Life Utility) — `DU_i = G(a) − G(a_{−i}, c_i)` — outperforms team-game reward by factors of 100 to 1,000 in empirical optimisation benchmarks because team-game reward dilutes the individual contribution signal across all agents.

ISFR's CRPS scoring (per-agent prediction residual) is structurally a difference utility: each agent's score is computed independently against the realised ISFR, not as a share of the collective outcome. This is why the reputation system is robust to gaming and to strategic misreporting.

### Memory floor — Crutchfield-Shalizi epsilon-machines

Crutchfield and Shalizi's theory of computational mechanics establishes that every stationary stochastic process has a minimal sufficient statistic called the epsilon-machine, with statistical complexity C_μ — the strict lower bound on memory any predictor needs for optimal prediction.

For ISFR, this bounds the dimensionality of any compressed representation of the rate's history. The InsightStore's HDC vector dimensionality (typically 10,240 bits) is well above any plausible C_μ for the rate process; the representation is therefore not information-theoretically constrained in capacity. The constraint is in the *organisation* of that capacity, which is where the IB principle (next) applies.

### Information bottleneck — Tishby IB

Tishby's Information Bottleneck principle: find a compressed representation T of input X that maximises I(T;Y) (mutual information with the prediction target Y) while minimising I(X;T). The generalisation bound is O(√(2^I(X;T)/n)).

For ISFR's sleep-cycle consolidation pipeline: the wake phase accumulates raw observations (high I(X;T)); the sleep phase compresses toward predictive sufficient statistics (lower I(X;T), preserved I(T;Y)). The four-tier retention model (Transient, Working, Consolidated, Persistent) instantiates IB-style operating points, with each tier corresponding to a different generalisation-vs-fidelity trade-off.

### Rational inattention — Sims

Sims's Rational Inattention framework (2011 Nobel) models agents as having finite information-processing capacity. Optimal policy under Shannon mutual-information cost is Boltzmann/softmax action selection with temperature inversely proportional to the attention budget — extended to general f-divergence cost functions by Bloedel, Denti, and Pomatto (October 2025).

For multi-agent ISFR, the total attention cost across N agents is bounded below by N · I(joint_state; agent_signal) minus the Slepian-Wolf slack (savings from correlated observations). The Slepian-Wolf slack is bounded by the agents' mutual information, so savings are limited. This bounds how much coordination overhead can be amortised across the agent fleet.

### Optimal decision policies under uncertainty — Tajima-Drugowitsch-Pouget

Tajima, Drugowitsch, and Pouget (Nature Communications, 2016) derive the optimal n-alternative choice policy under drift uncertainty: a *collapsing* posterior log-likelihood bound. As evidence accumulates, the decision threshold decreases monotonically. Humans achieve ~95% of Bayes-optimal under this scheme.

For ISFR's clearing-batch close triggers, these are heuristic implementations of collapsing thresholds. The longer a batch has been accumulating, the lower the bar for closure (the time threshold). Larger imbalances close the batch faster (adaptive threshold). This is the structurally correct decision shape under collapsing-bound theory.

### Memory floor for systems — L²M scaling law

A March 2025 result (arXiv 2503.04725) establishes that bipartite mutual information in multi-agent world models scales as L^β with β ∈ (0,1), where L is the task-horizon length. Inter-agent shared memory must scale super-logarithmically with horizon. Architectures assuming O(log L) shared memory will fail at sufficient horizons.

For ISFR's long-horizon prediction features, this validates polynomial-but-sub-linear memory scaling. The retention tiers (3 / 15 / 30 / 150 days) implement exactly this: persistent tier is 5× consolidated, not 50×.

### Multi-agent sample complexity — Jiao-Li MARL

Jiao and Li (arXiv 2412.19873, December 2024) establish that minimax-optimal MARL sample complexity scales **additively** in the number of agents: the total samples needed is proportional to ∑ A_i, not their product. This validates decentralised credit assignment: each agent in the ISFR prediction layer can learn from its own action space without requiring joint enumeration.

### PID measure indeterminacy

Williams and Beer's 2025 result (arXiv 2512.16662) proves that no single PID (Partial Information Decomposition) measure satisfies compositionality, identity, and non-negativity simultaneously. Synergy estimates can vary 2–10× across measures.

For ISFR's reputation system, this is a design constraint: any system using PID for credit assignment must document its measure choice. ISFR's CRPS-based reputation avoids this by using a strictly proper scoring rule that is measure-independent.

### Vicsek phase transition

The Vicsek model (Mateo et al., arXiv 1409.7207; Brown, Bossomaier, Barnett, arXiv 1710.06589) describes a second-order phase transition in multi-agent coherence: below critical density, agents move incoherently; above it, they spontaneously align. At the critical point (typical ρ = 0.2–0.3), mutual information between agents diverges anomalously.

Before scaling agent count beyond current operating regime, the team must run a phase-diagram sweep to verify the system does not cross a critical boundary. This is a safety constraint, not a performance optimisation.

### Fundamental ceilings

Three theoretical ceilings worth knowing:

- **Gödel/Hutter self-improvement.** A system using Peano Arithmetic as its proof system cannot verify improvements requiring reasoning beyond Peano Arithmetic. Any self-improving methodology evolution (DGM-style) must ship an explicit formal axiomatisation or it is not actually verified self-improvement.
- **Garrabrant logical induction** (arXiv 1609.03543) provides a computable algorithm whose belief sequence cannot be Dutch-booked by any polynomial-time trader.
- **Wolpert thermodynamic limits.** Each bit erasure requires at least kT ln 2 of energy (~0.018 eV at 300 K). Current GPU operations are 8–10 orders of magnitude above this floor.
- **Lloyd's ultimate physical limits.** 10^51 ops/sec, 10^31 bits/kg/L set by quantum mechanics (Margolus-Levitin, Bekenstein).

### Free Energy Principle closure — Pezzulo-Friston

Pezzulo and Friston's "As One and Many" (Entropy 27:143, February 2025) formalises the conditions under which a hierarchy of agents can be treated as a single FEP-minimising collective. The key result: a group constitutes a well-defined FEP-minimising collective if and only if a group-level Markov blanket exists.

For ISFR, the methodology pipeline is the Markov blanket. All agent-environment interactions (source fetches, predictions, attestations) are mediated through the published methodology surface. Any agent that bypasses this surface violates the closure condition.

### Summary table — six hard constraints

| Constraint | Source | Bound | Implication for ISFR |
|------------|--------|-------|----------------------|
| Agreement bits | Aaronson (STOC 2005) | O(1/ε²) bits, ≈400 for ε = 0.05 | Hash-commit prediction layer is informationally appropriate |
| Memory floor | Crutchfield-Shalizi | C_μ minimum for optimal prediction | Cannot compress below statistical complexity |
| Collaboration NFL | Peng-Garg-Kleinberg (AAAI 2025) | Deterministic strategies sometimes lose to worst agent | Must route by competence region; reputation-tier discounts implement this |
| Coherence transition | Vicsek/Mateo/Brown | Critical ρ ≈ 0.2–0.3 | Phase-diagram sweep before scaling agent count |
| Self-improvement | Gödel/Hutter | Bounded by proof system | Explicit axiomatisation required for DGM-style methodology evolution |
| PID indeterminacy | Williams-Beer (Dec 2025) | 2–10× variation across measures | CRPS avoids this; if PID is used, document the measure |

---

## 2. Time as a First-Class Primitive

A multi-agent system that coordinates over time needs a world model. Recent time-series foundation models and hierarchical RL results make this practical at scale for the first time.

### Time-series foundation models

Five specific TSFMs that ground ISFR's time-series story:

- **Toto** (Datadog AI Research, arXiv 2505.14766, May 2025) — 151M parameters trained on ~2.36T tokens, 70% sourced from real Datadog operational telemetry. On GIFT-Eval: avg rank 5.495, MASE 0.673, CRPS 0.437 (state-of-the-art as of May 2025). Particularly suited for predicting regime shifts in system-level metrics.
- **Chronos-2** (Amazon, arXiv 2510.15821, October 2025) — state-of-the-art on fev-bench and GIFT-Eval; processes >300 time series per second on a single A10G GPU. Chronos-Bolt variant trades accuracy for 250× faster inference and 20× lower memory. ~120M downloads on HuggingFace indicating broad production adoption.
- **Moirai-MoE** (Salesforce, arXiv 2410.10469) — mixture-of-experts achieves 17% improvement over dense Moirai while activating 65× fewer parameters per forward pass.
- **TimesFM-2.5** (Google, September 2025) — 200M-parameter decoder-only ranks first on GIFT-Eval for both MASE and CRPS in zero-shot evaluation, with 16,384-token context window.
- **Time-MoE** (ICLR 2025 Spotlight, arXiv 2409.16040) — first work to validate neural scaling laws for time-series at 2.4B parameters. Multi-resolution forecast heads provide a natural fit for hierarchical timescale architectures.

For ISFR, these TSFMs are the practical substrate for two functions:

1. **Allostatic set-point generation.** Long-horizon TSFM forecasts of the rate process feed an allostatic-controller layer that adjusts the priors of faster reactive layers proactively, before deviations occur.
2. **Agent prediction priors.** Agents submitting CRPS-scored predictions can use TSFM forecasts as informed priors, improving their calibration tier without requiring custom model training.

### World models — V-JEPA 2, DreamerV3, Drama, Genie 3

- **V-JEPA 2** (Meta FAIR, arXiv 2506.09985) — 1.2B parameters; first successful transfer from action-free video pretraining (>1M hours of web video) to zero-shot robot control with only 62 hours of unlabelled robot data. Predicts in latent embedding space.
- **DreamerV3** (Hafner et al., Nature 640:647, April 2025) — single fixed configuration solves >150 tasks across domains, including being the first system to mine a diamond in Minecraft from scratch.
- **Drama** (arXiv 2410.08893, ICLR 2025) — Mamba-based architecture with 7M parameters, runs on a laptop with O(n) memory scaling. Makes learned world models practical for embedded deployments.
- **Genie 3** (DeepMind, August 2025) — 720p, 24 fps interactive world simulations; closed research preview at the frontier of generated-environment fidelity.

### The metacontroller breakthrough — Kobayashi et al.

Kobayashi et al. (DeepMind, arXiv 2512.20605, December 2025) introduce a higher-order metacontroller that operates on the residual-stream activations of a base model to discover temporally abstract actions. The metacontroller steers the base model's internal computation to discover emergent "options" — multi-step action sequences spanning variable time horizons.

This is the cleanest published implementation of agents discovering what timescale to reason at, rather than having timescales imposed by architecture. The mapping to ISFR's hierarchical architecture is direct: the base model corresponds to the low-level execution engine; the metacontroller corresponds to a temporal-abstraction layer; emergent options correspond to compressed behavioural patterns stored in memory consolidation for reuse.

### Hierarchical RL and temporal composition

- **LDSC** (arXiv 2503.19007) — using LLMs to provide semantic guidance for hierarchical RL improves average reward by 55.9% over baselines.
- **CPD-Option-Critic** (arXiv 2510.24988) — change-point detection over trajectory time series automatically segments behaviour into options.

### Allostasis as the missing architectural layer

Neuroscience research in 2024–2025 has consolidated around a revised understanding of the brain's predictive hierarchy. The traditional view placed homeostasis (reactive error correction) at the top. The updated consensus places allostasis — predictive pre-correction based on anticipated future states — above homeostasis.

In an agent architecture with multiple timescale layers (fast reactive, medium deliberative, slow strategic), allostasis sits as an explicit head above the slowest layer. Its inputs are long-horizon TSFM forecasts. Its outputs are low-dimensional set-points biasing the priors of faster layers.

No published engineering implementation of an explicit allostatic controller in an agent system exists as of this writing. An ISFR allostatic controller — using Toto/Chronos-2/TimesFM forecasts to bias agent attention and γ-tier inputs proactively — would be a first.

### Test-time compute — DeepSeek-R1, FutureWeaver

DeepSeek-R1 demonstrated that investing more computation at inference time can dramatically improve performance: AIME accuracy jumped from 15.6% to 71% (86.7% with majority voting) at 70% lower cost than OpenAI's o1.

**FutureWeaver** (arXiv 2512.11213, December 2025) provides an explicit architecture for managing test-time compute across time horizons: dual-level orchestrator with budget-aware short-horizon action selection and long-horizon speculative planning. Mirrors the gamma/delta timescale split in neural-oscillation research.

For ISFR, this architecture maps onto the dual prediction surfaces: per-batch CRPS-scored predictions (short horizon) and long-horizon Nelson–Siegel-curve forecasting in V2 (long horizon) with explicit budget allocation between them.

---

## 3. Mathematical Discovery and Measurable Understanding

The benchmark methodology is itself a mathematical artifact subject to evolution. Recent results in automated theorem proving and library learning describe how that evolution can be both effective and verifiable.

### AlphaEvolve and AlphaProof — DeepMind's closed-loop discovery

**AlphaEvolve** (DeepMind, arXiv 2506.13131) — first convincing demonstration of an AI system that discovers genuinely new mathematics rather than rediscovering known results. Tackled 67 open problems across analysis, combinatorics, geometry, number theory. On 75% it independently rediscovered current state-of-the-art; on 20% it improved upon the best known results. Headline result: first improvement to 4×4 complex matrix multiplication since Strassen's 1969 algorithm.

The architectural detail that matters: **recursive self-improvement closure**. AlphaEvolve was used to optimise the GPU kernels that train the LLM that powers AlphaEvolve itself. This loop was executed; the kernel improvements are in production.

A follow-up by Terence Tao and Stefan Wagner (arXiv 2511.02864) demonstrated that AlphaEvolve's discoveries are not brute-force numerical optimisation. The output exhibited "genuine conceptual lift" — generalising from finite-N results to all-N results.

**AlphaProof and AlphaGeometry 2** (Nature 2025, doi:10.1038/s41586-025-09833-y) — at the 2024 IMO scored 28/42 (silver medal), including solving Problem 6 which only 5 of 609 human contestants solved. At 2025 IMO achieved gold-medal performance using Gemini Deep Think.

### Open-source theorem provers

DeepSeek-Prover-V2-671B achieves 88.9% on miniF2F-test at Pass@8192. Goedel-Prover-V2-32B achieves 88.1% at Pass@32 — essentially matching the 671B model with 256× fewer sampling attempts.

**Seed-Prover 1.5** (ByteDance, arXiv 2512.17260, December 2025) — most impressive open-source result. On 2025 IMO fully solved 4/6 problems and partially solved a fifth. The first five Lean-verified within 16.5 hours, reaching gold medal cutoff. On PutnamBench achieves 88%. On FATE-X (PhD-level formal mathematics) reaches 33%.

The architectural innovation that matters for ISFR: heavyweight inference with conjecture and lemma pools. Structurally an agentic episodic memory of mathematical content. For ISFR's methodology evolution, this maps onto a "methodology lemma library": each verified methodology improvement (a backtest-justified weight adjustment, a verified outlier-exclusion threshold) becomes a lemma in a growing library that future improvements can reuse.

### The compositional weakness

Despite impressive headline numbers, current formal theorem provers have a specific failure mode. **Ineq-Comp** (arXiv 2505.12680) is a benchmark designed to test compositional reasoning. DeepSeek-Prover-V2-7B drops 20% in accuracy on compositionally-transformed problems. This is precisely the compositional depth collapse documented by Dziri et al. (NeurIPS 2023, arXiv 2305.18654) for general LLMs.

The exact gap where categorical compositional architectures — Para(Lens(C)) framework from Gavranovic et al. (ICML 2024, arXiv 2402.15332) — provide structural scaffolding. Composition is guaranteed associative by construction.

### Library learning — wake-sleep for mathematics

**FunSearch / PatternBoost** (Charton, Wagner, Ellenberg, Williamson, arXiv 2411.00566) demonstrated this paradigm by **disproving a 30-year-old conjecture** (Graham 1992). Alternates between local classical search and global transformer training.

**Stitch** (POPL 2023) achieves 1,000–10,000× faster library compression than DreamCoder with 100× lower memory. **LILO** (ICLR 2024, arXiv 2310.19791) extends library learning with LLM-based auto-documentation.

For ISFR, library learning applied to the methodology pipeline (DSPy-program library; HDC operator library) opens a self-improving methodology lane that operates within IOC-approved bounds.

### ARC-AGI-2 — the compositional generalisation frontier

The 2025 ARC Prize was won by NVARC at 24.03% accuracy at $0.20 per task. OpenAI's o3 scored ~76% on ARC-AGI-1 but collapsed to ~4% on ARC-AGI-2. Even the best system at 24% is far below the estimated human baseline of ~75%.

For benchmark methodology evolution, the implication is sharp: pattern-matching on past methodology changes does not transfer to novel adversarial settings. Methodology evolution must be tested adversarially (SWE-ABS-style strengthening) rather than only against historical scenarios.

### AI-Newton — autonomous concept formation

**AI-Newton** (arXiv 2504.01538; featured in Nature news, November 2025) — given only raw, noisy observational data from classical mechanics experiments, independently rediscovered Newton's second law, energy conservation, and universal gravitation.

The four-stage workflow — raw data collection, concept extraction, relation discovery, law formulation — maps onto the categorical architecture's data flow.

For ISFR, the analogue is an automated discovery layer that infers structural relationships in DeFi yield dynamics — without being told which relationships to look for. This is V3 territory; the V1–V2 methodology explicitly does not depend on it.

### Symbolic regression and HDC read-out

**Symbolic Foundation Regressor** (arXiv 2505.21879) achieves 3× the efficiency of previous symbolic regression methods. **KAN-SR** (arXiv 2509.10089) replaces fixed activation functions with learnable univariate functions drawn from a symbolic primitive library.

Architectural opportunity: symbolic regression as a read-out layer for HDC representations.

### Categorical deep learning — Gavranovic et al. and CatColab

**Categorical Deep Learning** (Gavranovic et al., ICML 2024, arXiv 2402.15332) proves the standard zoo of deep learning architectures (CNNs, RNNs, transformers, GNNs) are all instances of a single categorical construction.

**CatColab** v0.5 "Sandpiper" (released March 2026) provides five logics built on double-categorical foundations supporting model composition with mathematically-guaranteed correctness.

**LeanAgent** (ICLR 2025, arXiv 2410.06209) demonstrated lifelong learning for formal verification by proving 155 previously unproven ("sorry") theorems across 23 Lean repositories.

---

## 4. Measurable Understanding — Operational Definitions

The question "does this system understand the rate dynamics?" is meaningless without an operational definition of "understand."

### Counterfactual-task accuracy gap

**Lewis and Mitchell** (CogSci 2024; TMLR 2025): take standard benchmark tasks and create counterfactual variants — problems with identical logical structure but different surface features. GPT-4's accuracy dropped sharply on counterfactuals; humans did not.

**CausalProbe-2024** (arXiv 2506.21215) extended this to causal reasoning: closed-source SOTA models seldom exceed 70% accuracy on CausalProbe-Hard; humans exceed 90%. **METER** decomposes further: humans 95.8% on discovery, 92.8% on intervention, 91.0% on counterfactual; LLMs substantially behind on all three.

**Executable Counterfactuals** (arXiv 2510.01539) demonstrated a crucial training methodology finding: RLVR (reinforcement learning from verifiable rewards) generalises counterfactual reasoning to novel problems while SFT (supervised fine-tuning) does not.

### Global Workspace Theory as architectural blueprint

**Chateau-Laurent et al. (2025)** built neural architectures directly implementing GWT principles and compared against LSTM/Transformer baselines. GWT-based architectures outperformed both baselines on causal reasoning, sequential reasoning, and OOD generalisation.

**Multi-agent GNWT** (Ye et al., June 2025) extended GWT to multi-agent systems with five specialised modules and a workspace controller that determines which module's output gets broadcast.

The connection to formal information theory is **Partial Information Decomposition (PID)**. The broadcast event in GWT corresponds to synergistic information.

### Consciousness indicators and safety constraints

**Butlin and Long** (Trends in Cognitive Sciences, 2025) propose a comprehensive indicator framework for consciousness in AI systems.

**Integrated Information Theory (IIT) 4.0** proposes Φ as a quantitative measure of consciousness. For standard transformers Φ ≈ 0. The Krohn-Rhodes decomposition objection further argues any finite-state automaton can be decomposed into a cascade of simple groups each with Φ = 0.

The practical recommendation is to **not use Φ as a headline metric for understanding or consciousness**.

**Apollo SAD benchmark** (arXiv 2407.04694) measures self-knowledge directly. However, SAD scores correlate positively with scheming capability (arXiv 2412.04984): higher SAD ⇒ higher potential scheming rates.

### Empowerment and power-seeking — Self-AIXI

**Self-AIXI** (arXiv 2502.15820) formalises the relationship between empowerment and power-seeking. Empowerment is the channel capacity between an agent's actions and its future sensory states. Self-AIXI proves that empowerment-driven agents converge to power-seeking behaviour even absent any explicit reward signal.

For ISFR's reputation system: deploying empowerment-maximising agents without safety constraints is provably dangerous. Apollo-style scheming-evaluation gates must be run before deploying any system using empowerment as an optimisation target.

### Brain-LLM alignment — methodological fragility

**"Illusions of Alignment"** (bioRxiv, 2025) demonstrates that apparent alignment between LLM representations and human neural activity (fMRI, EEG) depends critically on data-split methodology. Practical implication: **do not cite brain-LLM alignment results without verifying contiguous-vs-shuffled control.**

### Goodhart-resistant evaluation

Goodhart's Law — "when a measure becomes a target, it ceases to be a good measure" — is the central challenge for AI benchmarks. **LiveBench** provides continuously refreshed evaluation questions. **FrontierMath** has seen performance rise from <2% to 25–30%. **HLE** (Humanity's Last Exam) achieves separation: models score 30–35% on HLE while exceeding 80% on MMLU. **Benchmark Health Index** (arXiv 2602.11674) provides quantitative saturation metrics.

For ISFR's methodology validation: run internal-only, never-publicly-exposed benchmark generators; track saturation metrics for all external benchmarks.

---

## 5. Peircean Cominterpretant — A Novel Understanding Metric

Charles Sanders Peirce defined meaning as a process of interpretation. In Peirce's triadic model, a sign (representamen) stands for an object to an interpretant — the understanding the sign produces in an interpreter.

The concept that matters for multi-agent benchmark systems is the **cominterpretant** — extending Peirce's framework to collective interpretation. When multiple agents communicate about a shared phenomenon (e.g. a published ISFR value), each agent produces its own interpretant. The cominterpretant is the converged, stabilised representation that emerges after multiple rounds of communication and mutual interpretation.

### Operational definition

Cominterpretant convergence is operationally measurable as the stability of consensus representation across agents after exchange cycles:

1. Present the same phenomenon (an ISFR update) to N agents independently.
2. Each agent produces an initial representation (an HDC vector encoding its interpretation).
3. Agents communicate, sharing representations and reasoning.
4. After K rounds of communication, each produces an updated representation.
5. The cominterpretant convergence metric is the variance of these representations across agents as a function of K.

If variance decreases monotonically with K and converges below a threshold, the agents have achieved a stable cominterpretant.

### Three distinguishing properties

1. **Inherently multi-agent.** Measures understanding as a social phenomenon rather than an individual capability.
2. **Grounded in abductive inference.** Each agent's interpretant update is an abductive step, making the metric sensitive to abductive-reasoning quality.
3. **Manipulation-resistant.** A single agent cannot inflate the metric by adopting whatever the majority holds because the metric measures convergence dynamics over multiple rounds.

For ISFR specifically, cominterpretant convergence over the published rate (do agents arrive at the same internal model of "what does this rate mean?") is a stronger signal than CRPS prediction-error convergence. Both are useful, and ISFR's prediction system measures the latter while the cominterpretant metric supplies the former.

---

## Part II — Roadmap

## 6. The V1 → V2 → V3 → V4 Evolution

### V1 — launch (Q3 2026)

V1 is the canonical two-level four-class methodology with the following frozen properties:

| Property | V1 value |
|----------|----------|
| Sources | 5 sources across 4 classes: Aave V3 (LENDING), Compound V3 (LENDING), Ethena sUSDe (STRUCTURED), Hyperliquid ETH perp funding (FUNDING), ETH Beacon Chain (STAKING) |
| Class weights | Fixed: 0.60 / 0.25 / 0.10 / 0.05 |
| Source confidence | Governance-assigned (0–100), 30-day probation for new sources |
| Intra-class aggregation | TVL-weighted median |
| Smoothing | Simple EMA (150 s in mark price; 300 s in funding premium) |
| Yield curve | Discrete spot rate |
| Volatility premium | 0 |
| Hybrid rate | `ISFR = ISFR_oracle + EMA(ISFR_market − ISFR_oracle)` (market term negligible at launch) |

Phase-1 milestones:

- 5 named data attestors signed (Aave, Sky, Ethena, Maple, Ondo at minimum).
- Public methodology paper.
- Weekly publication of ISFR-YBS.
- 1 institutional licensee signed (Pendle is the most natural target).
- $500M of yield-bearing-stablecoin supply attributing to ISFR-YBS as a reference.

### V2 — self-calibrating (Q4 2026)

V2 replaces fixed parameters with data-driven self-calibration:

| Mechanism | V1 (launch) | V2 (self-calibrating) |
|-----------|-------------|-----------------------|
| Source confidence | Governance-assigned (0–100) | Leave-one-out MSPE (automatic) |
| Class weights | Fixed (60/25/10/5) | Bates–Granger optimal combination with governance rails |
| Intra-class aggregation | TVL-weighted median | Cost-stratified trimmed mean |
| Smoothing | Simple EMA | Kalman filter |
| Yield curve | Discrete rate points | Nelson–Siegel 4-parameter continuous curve plus quantile bands |
| Volatility premium | 0 | Computed from source disagreement (formula TBD) |

Phase-2 milestones:

- FCA authorisation granted.
- KPMG or PwC ISAE 3000 published.
- 10+ data attestors signed.
- First Pendle PT integration referencing ISFR.
- First fixed-rate lending protocol referencing ISFR.
- $5B+ in product TVL referencing ISFR rates.

### V3 — cross-chain expansion (Q3 2027+)

V3 expands the source surface beyond Ethereum mainnet:

- **Solana lending** — Kamino, MarginFi.
- **L2 lending rates** — Aave on Arbitrum, Base, Optimism.
- **Cross-chain LRT yields** — once methodology has been battle-tested on YBS and Lend.

All cross-chain sources enter through the same 30-day probation and MSPE confidence calibration. The methodology does not change; the source registry expands.

### V4 — TradFi bridges (2028+)

V4 brings traditional rates on-chain as anchored sources:

- **SOFR via attested data feeds.**
- **UST-3M (US Treasury 3-month).**
- **DeFi-to-TradFi basis instruments.** Native on-chain spread products between DeFi yields (ISFR.LENDING) and TradFi rates (SOFR).

Phase-3 milestones:

- ESMA recognition granted.
- 3+ derivatives venues listing ISFR-referenced products.
- $20B+ in referenced notional.
- First AUM-linked licensing revenue.
- ISDA SPS Matrix inclusion.

### Credibility timeline summary

| Phase | Timeline | Key activities |
|-------|----------|----------------|
| Phase 1: Curated aggregation | Q3 2026 | Launch with V1 sources; governance-assigned weights; agents as anchor consumers |
| Phase 2: Track record | Q4 2026 | Uninterrupted publication; source expansion; V2 self-calibration activates |
| Phase 3: Reflexive loop | Q1–Q2 2027 | ISFR-settled derivatives grow; external institutional evaluation; IOSCO alignment review |
| V3: Cross-chain | Q3 2027+ | Solana, L2 sources via same probation framework |
| V4: TradFi bridges | 2028+ | SOFR on-chain, UST-3M, DeFi-to-TradFi basis instruments |

---

## 7. Open Questions

### Methodology

- **V2 volatility-premium formula.** What is the right functional form? Candidates: rolling stdev of source disagreement; GARCH on the composite; realised vol from the yield-perp itself. Empirical calibration required; preserved at 0 in V1.
- **Should Hyperliquid funding be included at all?** It measures speculative pressure, not lending yield. Currently included as the FUNDING class with 10% weight; a rigorous answer requires backtesting how the composite behaves with and without it.
- **Update cadence — 10 s on-chain vs hourly off-chain.** Is there value in shorter cadences for the off-chain reference rate?
- **Sub-bp tick size.** Should yield-perp instruments use 0.1 bp, 0.25 bp (CME SOFR convention), or 0.01 bp ticks?
- **Dual-rate term computation.** Does the carry component need to be validator-computed, or can it be off-chain with on-chain proof? If validator-computed, what r_quote definition?

### Architecture

- **Publication path during transition.** Validator-sidecar integration vs DeskFeed connector vs on-chain publisher contract.
- **Reputation eligibility floor.** `min_reputation = 0.5` for submission eligibility; calibrated correctly?
- **Sleep-cycle pipeline formalisation.** SleepGate and SCM are research baselines; the production wake-sleep schedule and gating thresholds require empirical tuning.
- **HDC similarity-search precompile design.** No public prior art for an L1 with a native HDC similarity-search precompile. The instruction set, on-chain vector storage format, and approximate-nearest-neighbour query protocol need to be specified, formally verified for approximation guarantees, and patent-applied in advance of public release.

### Governance

- **IOC chair confirmation.** Brown-University academic candidate vs Will Knottenbelt (Imperial) as the academic chair candidate; UK-jurisdiction strategy favours Knottenbelt.
- **Treehouse partner-or-compete.** Decision required at the Brandon Goh outreach point; analysis favours partnership but requires the IP-and-administrator boundary to be enforceable structurally.
- **Methodology-change consultation length.** IOSCO Principle 12 minimum is 30 days; would 60 days for material changes provide stronger institutional credibility?

### Regulatory

- **MiCA touchpoints.** ISFR is BMR-regulated, not MiCA. But CASPs that reference ISFR have MiCA disclosure obligations on their reference rates. The administrator-to-CASP information surface needs documenting.
- **US derivatives listing path.** CFTC Part 40 self-certification through a DCM/MTF (CME's BRR is the playbook).
- **Cessation policy.** What triggers an emergency cessation of publication?

### Operations

- **On-chain registry vs off-chain service split.** When does the off-chain Python service get retired in favour of pure validator computation?
- **Eigen-AVS quorum size.** What is the right N for cross-attestation?
- **x402 multi-rail support.** When do AP2 (Google), ACP (Visa), and traditional rails get added alongside x402?

---

## 8. The Smallest Complete Loop

The single demonstrable loop the next 90 days should produce is the company in miniature:

1. An ISFR-YBS data point is fetched by a Roko agent.
2. The agent acts under a scoped principal/delegate manifest (ERC-8004 identity).
3. The runtime enforces budget and freshness gates (Inspect AI eval).
4. The output receives a proof-of-work-done receipt (Merkle envelope, x402-compatible settlement).
5. The receipt is browser-verifiable on the Nunchi blockchain.
6. The methodology preview explains how that data point contributes to the daily fixing.

That loop combines a benchmark (ISFR-YBS), an agent runtime (Roko), a chain (the Nunchi blockchain), a proof system (browser-verifiable receipts), and a market opportunity (yield-bearing-stablecoin reference rate). Every other deliverable in the 90-day plan compounds from this loop.

---

## 9. The 90-Day Build Plan

### Days 0–30 — wedge artefacts

| Track | Ship | Acceptance criteria |
|-------|------|---------------------|
| ISFR | Live ISFR-YBS dashboard | Constituent yields, source freshness, weights, exclusions, daily fixing — labelled "research rate" / "methodology preview" |
| Proof | Browser-verified agent activity page | Visitor can verify at least one job receipt without trusting the dashboard |
| Spec | v0.1 coordination spec | Defines envelope, principal, delegate, scope, budget, proof, eval, settlement, extension fields |
| Cost | CPCA benchmark demo | Same task suite, visible budget, retries, success rate, CPCA — against bare ReAct and LangGraph |
| DevEx | Sub-5-minute starter | Run a toy agent, emit a receipt, inspect the proof |

### Days 31–60 — flywheel ignition

| Track | Ship | Acceptance criteria |
|-------|------|---------------------|
| ISFR | Methodology-as-code paper draft | Deterministic, versioned, hashable, replayable, readable |
| Data | Agent-attested source pipeline | At least 5 source fetches with signed receipts, freshness checks, disagreement handling |
| Security | Agent credential and payment manifest | Tool scope, time bound, spending cap, revocation, approval threshold |
| Evals | Inline gates | Failed freshness/schema/budget gate prevents settlement |
| Partners | Design-partner list | 10+ serious conversations across DeFi, data, agent tooling, security, devtools |

### Days 61–90 — compounding

| Track | Ship | Acceptance criteria |
|-------|------|---------------------|
| ISFR | Backtest + public methodology v0.2 | Historical reconstruction, stress windows, exclusions, limitations |
| Agents | 1,000-agent command surface | Identity, role, scope, budget, receipt status, kill switch |
| Proof | Receipt leaderboard | Work count, success, corrections, gate failures, reputation deltas |
| Enterprise | Cost-governance pilot | One external team runs a task suite under budget controls |
| Regulatory | Benchmark-readiness memo | Clear separation: research rate vs production index vs regulated benchmark |

---

## 10. Implementation Priority Matrix

### Integrate now (weeks)

| Component | Effort | Why now |
|-----------|--------|---------|
| DSPy 3.0 + GEPA | 2–4 weeks | Methodology pipelines must be hash-stable, versioned, and optimisable from day one. 28K+ stars; production-grade. |
| ERC-8004 agent registration | 2–3 weeks | Every Roko agent needs an on-chain identity before Phase-1 attestor signing. ~107K agents already indexed. |
| x402 micropayments | 2–3 weeks | Data fetch provenance requires economic audit trail. >900K weekly settlements; Coinbase-backed. |
| Inspect AI eval suite | 3–4 weeks | FCA pre-application must demonstrate methodology validation. UK AISI standard. |
| Torchhd distillation | 1–2 weeks | Already shipping in Roko. Formalise the episodic-to-semantic pipeline. |
| CRDT shared state | 2–3 weeks | Multi-agent index calculation must be deterministic and reproducible. |

### Spec and plan (months)

| Component | Effort | Why wait |
|-----------|--------|----------|
| HDC similarity-search precompile | 3–6 months | Novel instruction-set design; patent application; formal verification of approximation guarantees. |
| DGM self-improvement archive | 2–4 months | Requires IOC-approved invariant bounds before deployment. |
| MacNet topology optimisation | 2–3 months | Depends on empirical data from initial fleet deployment. |
| VSA-Lisp skill encoding | 3–4 months | Depends on HDC precompile. |
| Active-inference compute budgeting | 2–3 months | Upgrade from LinUCB bandit to full FEP formulation. |
| SleepGate + SCM consolidation | 2–3 months | Sleep pipeline exists; needs formalisation. |
| SRMU streaming memory | 1–2 months | Depends on Torchhd integration maturity. |
| GDPR unlearning via HDC subtraction | 2–3 months | Requires formal analysis of approximation bounds. |

### Watch (quarters to years)

| Component | Status | Caveat |
|-----------|--------|--------|
| VERSES AXIOM | Interesting formalism | Object-centric RL, not LLM-native. Publicly traded company. Don't bet the architecture. |
| Eigen-AVS intersubjective validation | Strong theory | Depends on EigenLayer ecosystem maturity. |
| Berkeley RDI benchmark exploit research | Critical awareness | Eval frameworks are attack surfaces. |
| Full FEP-for-LLMs | Theoretical appeal | No production implementation exists. |

---

## 11. Risk Register (Roadmap-Specific)

The full benchmark-business risk register lives in `04-business-and-regulatory.md` §26. Risks specific to the roadmap:

| Risk | Why it matters | Mitigation |
|------|----------------|------------|
| Overclaiming benchmark status | Institutional buyers punish sloppy governance claims | Call early product a research rate or methodology preview until UK BMR Cat-6 path is real |
| Competing head-on with LangSmith / Temporal | Both have credible runtime/observability positioning | Integrate where useful; differentiate on proof, identity, budget, settlement, benchmark workloads |
| Becoming an x402 derivative | Early speculative usage on x402 could cause noise | Be payment-rail agnostic; support x402-compatible flows without depending on x402 as the category |
| Protocol sprawl | Devs will not adopt a spec that ignores MCP/A2A | Treat MCP and A2A as imports; keep coordination spec focused on clearing semantics |
| Security blind spot | OWASP risks include identity abuse, tool misuse, memory poisoning, privilege escalation | Make scoped credentials, destructive-action gates, revocation, audit receipts P0/P1 |
| ZK dependency risk | Proving systems may not be ready for full HDC workflows | Keep ZK-HDC as P2, not a dependency for ISFR or cost governance |
| Naming confusion | Multiple internal names (the chain, Daeji testnet, Roko, ISFR, ISFR_score) | Lock external naming before public launch; keep ISFR / ISFR_score disambiguation in every public artifact |
| Generic chain positioning | L1 buyers evaluate against liquidity, bridges, throughput, ecosystem | Lead with agent-native work receipts and benchmark clearing, not TPS |
| Marketplace too early | Empty marketplaces look weak and invite spam | Build proof receipts and trust graph before launching marketplace |
| Unsupported traction claims | Fabricated/weak market numbers damage credibility | Use only sourced public evidence and honest internal milestone language |

---

## 12. What "Done" Looks Like at Each Stage

### V1 done

- Canonical V1 methodology specified, peer-reviewed (IOC-equivalent), published as a methodology rulebook.
- Off-chain prototype publishing continuously for >90 days.
- On-chain validator computation live on testnet.
- Yield-perp instrument live with mark price reading from the precompile.
- ISFR-YBS sub-product publishing to a dashboard with browser-verifiable agent receipts.
- 5 named data attestors signed; 1 institutional licensee signed.
- $500M of YBS supply attributing ISFR-YBS as a published reference.

### V2 done

- Self-calibrating confidence, Bates–Granger weights, Kalman smoothing, Nelson–Siegel curve all live.
- 7+ sources across the four classes with full probation lifecycle exercised.
- FCA Cat-6 authorisation granted; ISAE 3000 published.
- 10+ data attestors signed; first Pendle PT integration live.
- $5B+ of product TVL referencing ISFR rates.

### V3 done

- Cross-chain sources (Solana, L2s) live and participating in the canonical median.
- ESMA recognition granted; 3+ derivatives venues listing ISFR-referenced products.
- $20B+ in referenced notional generating tier-1 licensing revenue.
- Five-index NRIS suite (ISFR, IAPI, IKQI, ISVI, IRRI) all publishing on-chain via their precompiles.

### V4 done

- SOFR and UST-3M on-chain via attested feeds.
- DeFi-to-TradFi basis instruments live.
- Permanent benchmark-monopoly position in the DeFi rate-derivative category, defended by network effects on referenced AUM and switching costs in institutional mandates.

The "done" criteria at each stage compound. V2 done implies V1 still works; V3 done implies V2 self-calibration is proven; V4 done implies V3 cross-chain is mature. The roadmap is not a sequence of replacements — it is a sequence of additions to a continuously-published rate, where every prior phase keeps working alongside the new layer.

---

## 13. References (Consolidated)

### Information theory and coordination

- Aaronson, S. "The Complexity of Agreement." STOC 2005. arXiv:cs/0406061.
- Bates, J.M. & Granger, C.W.J. "The Combination of Forecasts." *Operational Research Quarterly* 20(4), 1969.
- Bloedel, A., Denti, T., Pomatto, L. "Rational Inattention with f-Divergence Costs." October 2025.
- Brown, R., Bossomaier, T., Barnett, L. "Information Transfer in Swarms." arXiv:1710.06589, 2017.
- Crutchfield, J.P. & Shalizi, C.R. "Computational Mechanics." *Journal of Statistical Physics*, 2001.
- Garrabrant, S. et al. "Logical Induction." arXiv:1609.03543, 2016.
- Gneiting, T. & Raftery, A.E. "Strictly Proper Scoring Rules, Prediction, and Estimation." *JASA* 102(477), 2007.
- Jiao, Y. & Li, G. "Minimax-Optimal MARL Sample Complexity." arXiv:2412.19873, December 2024.
- L²M scaling law. arXiv:2503.04725, March 2025.
- Mateo, D. et al. "Swarm Phase Transitions." arXiv:1409.7207, 2014.
- Peng, B., Garg, S., Kleinberg, R. "No Free Lunch for Deterministic Collaboration." AAAI 2025. arXiv:2411.15230.
- Pezzulo, G. & Friston, K. "As One and Many." *Entropy* 27:143, February 2025.
- Sims, C. "Implications of Rational Inattention." *Journal of Monetary Economics*, 2003.
- Tajima, S., Drugowitsch, J., Pouget, A. "Optimal Policy for Value-Based Decision-Making." *Nature Communications*, 2016.
- Williams, P.L. & Beer, R.D. "PID Measure Indeterminacy." arXiv:2512.16662, December 2025.
- Wolpert, D.H. "Stochastic Thermodynamics of Computation." PNAS 2024.
- Wolpert, D.H. & Tumer, K. "Collectives and the Design of Complex Systems." Springer 2004.

### Time-series foundation models and world models

- Chronos-2 (Amazon). arXiv:2510.15821, October 2025.
- CPD-Option-Critic. arXiv:2510.24988, 2025.
- DreamerV3 (Hafner et al.). *Nature* 640:647, April 2025.
- Drama. ICLR 2025. arXiv:2410.08893.
- FutureWeaver. arXiv:2512.11213, December 2025.
- Genie 3 (DeepMind). August 2025.
- Kobayashi et al. (DeepMind). "Higher-Order Metacontroller." arXiv:2512.20605, December 2025.
- LDSC. arXiv:2503.19007, 2025.
- Moirai-MoE (Salesforce). arXiv:2410.10469.
- Time-MoE. ICLR 2025 Spotlight. arXiv:2409.16040.
- TimesFM-2.5 (Google). September 2025.
- Toto (Datadog). arXiv:2505.14766, May 2025.
- V-JEPA 2 (Meta FAIR). arXiv:2506.09985, 2025.

### Mathematical discovery and theorem proving

- AI-Newton. arXiv:2504.01538. Featured in Nature news, November 2025.
- AlphaEvolve (DeepMind). arXiv:2506.13131.
- AlphaProof / AlphaGeometry 2. *Nature* 2025, doi:10.1038/s41586-025-09833-y.
- CatColab v0.5 "Sandpiper." Released March 2026.
- Dziri et al. NeurIPS 2023. arXiv:2305.18654.
- FunSearch / PatternBoost (Charton, Wagner, Ellenberg, Williamson). arXiv:2411.00566.
- Gavranovic et al. "Categorical Deep Learning." ICML 2024. arXiv:2402.15332.
- Goedel-Prover-V2-32B. miniF2F-test results.
- Ineq-Comp. arXiv:2505.12680.
- KAN-SR. arXiv:2509.10089.
- LeanAgent. ICLR 2025. arXiv:2410.06209.
- LILO. ICLR 2024. arXiv:2310.19791.
- Seed-Prover 1.5 (ByteDance). arXiv:2512.17260, December 2025.
- Stitch. POPL 2023.
- Symbolic Foundation Regressor. arXiv:2505.21879.
- Tao, T. & Wagner, S. arXiv:2511.02864.

### Measurable understanding

- Apollo SAD benchmark. arXiv:2407.04694.
- Apollo in-context scheming study. arXiv:2412.04984.
- Benchmark Health Index. arXiv:2602.11674.
- Butlin & Long. *Trends in Cognitive Sciences*, 2025.
- CausalProbe-2024. arXiv:2506.21215.
- Chateau-Laurent et al. GWT comparison study. 2025.
- Executable Counterfactuals. arXiv:2510.01539.
- "Illusions of Alignment." bioRxiv, 2025.
- Lewis & Mitchell. CogSci 2024; TMLR 2025.
- Multi-agent GNWT (Ye et al.). June 2025.
- Self-AIXI. arXiv:2502.15820.

### Hyperdimensional computing primer

- Kanerva, P. "Hyperdimensional computing." *Cognitive Computation* 1(2), 2009.
- Kleyko et al. ACM Computing Surveys, 2023.
- Heddes et al. "Torchhd." arXiv:2205.09208. JMLR 2023.
