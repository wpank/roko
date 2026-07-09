# Information-Theoretic Limits, Fundamental Scaling Laws, and Time as a First-Class Primitive

## Introduction

This document addresses a question that most multi-agent system designs never confront: what are the hard mathematical walls that no amount of engineering can breach? Before committing to any architecture for coordinating autonomous agents — shared world models, hierarchical planning, distributed credit assignment, memory consolidation — a designer must understand which constraints are fundamental (imposed by information theory, computational complexity, and thermodynamics) and which are merely engineering challenges to be solved with better hardware or algorithms.

The answers turn out to be surprisingly concrete. We now have tight bounds on how many bits two agents must exchange to reach approximate agreement, provable no-free-lunch theorems showing that naive collaboration strategies can underperform even the worst individual agent, strict lower bounds on how much memory a system needs for optimal prediction, and phase-transition results showing that multi-agent coherence can collapse catastrophically when operating near critical density.

Alongside these impossibility results, a parallel development has made time-series forecasting and world modeling newly practical for agent systems. Foundation models trained on trillions of time-series tokens can now predict regime shifts in operational telemetry. Hierarchical reinforcement learning has found principled ways to discover temporal abstractions — not hand-engineered skill hierarchies, but learned decompositions of behavior into multi-scale options. And a recent metacontroller result from DeepMind demonstrates that agents can discover what timescale to reason at, rather than having timescales imposed externally.

This synthesis covers both sides: the walls you cannot break, and the tools you can use within those walls to build systems that reason about time as a first-class primitive.

---

## 1. Information-Theoretic Limits on Agent Coordination

### 1.1 The Complexity of Agreement

The single most important result for anyone designing multi-agent coordination is Scott Aaronson's "The Complexity of Agreement" (STOC 2005; arXiv cs/0406061). The theorem states: two Bayesian agents who share a common prior can reach epsilon-agreement on any posterior probability by exchanging O(1/epsilon-squared) bits, regardless of how much private information each agent holds internally.

Read that carefully. The amount of communication required depends only on the desired precision of agreement, not on the richness or complexity of each agent's internal knowledge. An agent that has processed a million documents and an agent that has processed ten can converge to within epsilon = 0.05 by exchanging roughly 400 bits — 50 bytes.

This has immediate architectural consequences. If a system represents each agent's state as a 10,240-bit hyperdimensional computing (HDC) vector (a common choice for holographic reduced representations), then each pairwise reconciliation transmits 25 times more data than the theoretical minimum for epsilon = 0.05 agreement. This is not necessarily wrong — there may be good engineering reasons for the overhead — but designers should understand that per-agent depth (how much an agent knows internally) is provably decoupled from coordination cost (how much agents need to tell each other to agree). Systems that scale coordination bandwidth linearly with agent knowledge are over-engineered by the Aaronson bound.

### 1.2 Collaboration Is Not Free: The Peng-Garg-Kleinberg No-Free-Lunch Theorem

The intuitive assumption that combining multiple agents always helps is formally false. Peng, Garg, and Kleinberg (arXiv 2411.15230; AAAI 2025) prove a no-free-lunch result for deterministic collaboration strategies: any collaboration strategy that does not always defer to a single agent will, on some input distribution, underperform the least accurate agent in the group.

This is a strong negative result. It does not say "collaboration sometimes fails to help." It says "collaboration sometimes makes things strictly worse than the worst individual." The key word is "deterministic" — the result applies to fixed voting rules, fixed routing strategies, and fixed ensemble methods.

The escape hatch is competence-region awareness. If the collaboration strategy can identify which agent is most competent for a given input region and route to that agent selectively, the NFL theorem does not apply (because such routing is not a fixed deterministic strategy — it is input-dependent). Any system that routes tasks to agents must therefore maintain an explicit model of each agent's competence regions and route based on predicted accuracy, not on generic confidence scores or majority voting. Blind ensembling is provably dangerous.

### 1.3 Credit Assignment: Wolpert's Difference Utility

David Wolpert's Collective Intelligence (COIN) framework and its Probability Collectives variant provide the theoretical foundation for multi-agent credit assignment. The central concept is Difference Utility (also called Wonderful Life Utility): an agent's reward is the global outcome minus the counterfactual global outcome where that agent is removed. This is formally: DU_i = G(a) - G(a_{-i}, c_i), where G is the global objective, a is the joint action, and c_i is a default action for agent i.

Empirical results across optimization benchmarks show that difference utility outperforms team-game reward (where every agent receives the same global reward) by factors of 100 to 1,000. The reason is that team-game reward gives each agent a noisy, diluted signal — the global outcome reflects everyone's actions, not the individual contribution. Difference utility isolates each agent's marginal impact.

This connects directly to Partial Information Decomposition (PID), which decomposes joint mutual information into redundant, unique, and synergistic components. An agent's PID-unique information — the predictive information that only it provides — maps onto Wolpert's difference utility. The design principle: reward agents on their PID-unique contribution to the joint outcome, not on the joint outcome itself.

### 1.4 Memory Floors: Crutchfield-Shalizi Epsilon-Machines

James Crutchfield and Cosma Shalizi's theory of computational mechanics establishes that every stationary stochastic process has a minimal sufficient statistic called the epsilon-machine. The statistical complexity C_mu of this epsilon-machine is the strict lower bound on the amount of memory any predictor needs to achieve optimal prediction of the process.

For agent memory systems — particularly those that consolidate experience through offline "dream" processing — this sets a hard floor. If the environment's statistical complexity is C_mu bits, then no compression scheme can reduce the agent's memory below C_mu bits without provably destroying predictive power. This is not an engineering limit; it is information-theoretic.

Practically, this constrains the minimum dimensionality of any compressed representation (such as HDC vectors) used for storing consolidated experience. If the environment generates processes with C_mu = 500 bits of statistical complexity, then HDC vectors shorter than 500 bits cannot faithfully represent the causal structure needed for optimal prediction.

### 1.5 The Information Bottleneck and Dream Consolidation

Naftali Tishby's Information Bottleneck (IB) principle provides the optimal framework for lossy compression: find a compressed representation T of input X that maximizes I(T; Y) (mutual information between the compression and the prediction target Y) while minimizing I(X; T) (mutual information between the input and the compression).

Any memory consolidation process — where an agent replays raw experience and distills it into compact knowledge — should be understood as walking down the IB curve. The replay process reduces I(replay; raw_experience) while attempting to preserve I(replay; future_reward). The IB framework provides an explicit generalization bound: the generalization gap is at most O(sqrt(2^{I(X;T)} / n)), where n is the number of training samples. This means over-compression (very low I(X;T)) improves generalization at the cost of prediction quality, and the optimal operating point depends on sample size.

### 1.6 Rational Inattention and Attention Budgets

Christopher Sims' Rational Inattention framework (for which he received the 2011 Nobel in Economics) models agents as having a finite capacity for processing information. The optimal policy under Shannon mutual-information cost is Boltzmann/softmax action selection with temperature inversely proportional to the attention budget — a result that has been extended by Bloedel, Denti, and Pomatto (October 2025) to general f-divergence cost functions.

For multi-agent systems, this yields a lower bound on total attention cost: the sum across N agents must be at least N times I(joint_state; agent_signal) minus the Slepian-Wolf slack (the savings from correlated observations). The Slepian-Wolf slack formalizes how much redundancy across agents can reduce total information processing cost — but it is bounded by the agents' mutual information, so the savings are limited.

### 1.7 Optimal Decision Policies Under Uncertainty

Tajima, Drugowitsch, and Pouget (Nature Communications, 2016) derive the optimal n-alternative choice policy under drift uncertainty: a collapsing posterior log-likelihood bound. As evidence accumulates, the decision threshold decreases monotonically. Humans achieve approximately 95% of the Bayes-optimal performance under this scheme.

This has direct design implications for systems that use Expected Free Energy (EFE) or similar value-of-information metrics to decide when to stop deliberating and commit to an action. The theoretically optimal behavior is collapsing thresholds: the longer an agent has been thinking about a decision, the lower the bar should be for committing. Fixed thresholds are suboptimal. Any principled decision system should implement monotonically decreasing commitment thresholds as a function of elapsed deliberation time.

---

## 2. Phase Transitions and Critical Density

### 2.1 The Vicsek Phase Transition in Swarm Coherence

The Vicsek model (and its extensions by Mateo et al., arXiv 1409.7207; Brown, Bossomaier, and Barnett, arXiv 1710.06589) describes a second-order phase transition in multi-agent coherence as a function of noise and agent density. Below a critical density, agents move incoherently; above it, they spontaneously align. At the critical point, mutual information between agents diverges anomalously — meaning agents become maximally informative about each other's states.

The critical density in typical Vicsek-class models falls in the range rho = 0.2 to 0.3. A system operating at rho = 0.23 (a natural density for moderate-sized agent pools) sits squarely on the critical curve. This is simultaneously the point of maximum information transfer and maximum fragility. A small perturbation — adding noise, removing an agent, changing the communication topology — can cause catastrophic decoherence: the entire collective abruptly loses coherent behavior.

This is fundamentally a safety result, not an optimization result. Before scaling any multi-agent system to a new size, the team must run a phase-diagram sweep: vary the effective density parameter across the expected operating range and verify that the system does not cross a critical boundary. If it does, the system needs either damping (to stay subcritical) or explicit phase-stabilization mechanisms (to maintain coherence above criticality).

---

## 3. Fundamental Ceilings

### 3.1 Goedel, Hutter, and the Self-Improvement Ceiling

A self-improving system is provably bounded by the formal logic it uses to verify improvements. This follows from Goedel's incompleteness theorems and is made precise in Hutter's AIXI framework and Schmidhuber's Goedel Machine: a system using Peano Arithmetic as its proof system cannot verify the correctness of improvements that require reasoning beyond Peano Arithmetic.

The practical consequence is that any self-improving agent must ship an explicit formal axiomatization: its utility function, its priors over environments, and its proof system. Without this, there is no well-defined notion of "verified improvement," and the system is either doing unverified self-modification (unsafe) or not actually self-improving (just hill-climbing on heuristics).

### 3.2 Garrabrant Logical Induction

Scott Garrabrant's logical induction framework (arXiv 1609.03543) provides a computable algorithm whose belief sequence cannot be Dutch-booked by any polynomial-time trader. This is the strongest known coherence guarantee for bounded reasoners — agents that cannot compute all logical consequences of their beliefs.

For practical systems, Garrabrant induction provides the theoretical benchmark for what "rational belief updating under computational constraints" means. Any system that claims to maintain coherent beliefs over time should be evaluated against this standard: can a polynomial-time adversary construct a sequence of bets that extracts unbounded profit from the system's belief updates?

### 3.3 Thermodynamic Limits on Computation

Wolpert's stochastic thermodynamics results (PNAS 2024) establish exact equalities — not just inequalities — for entropy production in computation. The Landauer limit sets a floor: each bit erasure requires at least kT ln 2 of energy, approximately 0.018 electron-volts at 300 Kelvin (room temperature).

Current GPU operations consume energy roughly 8 to 10 orders of magnitude above this limit. This means thermodynamic limits are irrelevant for silicon-based systems today — the engineering overhead dominates by a factor of a hundred million or more. However, for neuromorphic computing architectures that approach physical limits, these bounds become binding constraints. Any roadmap that includes neuromorphic or reversible-computing hardware must account for the Landauer floor.

### 3.4 Lloyd's Ultimate Physical Limits

Seth Lloyd's calculation of the ultimate limits of computation gives 10^51 operations per second and 10^31 bits of memory per kilogram of matter per liter of volume. These are set by quantum mechanics (the Margolus-Levitin theorem for speed, the Bekenstein bound for memory). No engineering, however advanced, can exceed them.

For all current and foreseeable systems, these limits are academic. They confirm that engineering, not physics, is the binding constraint on computational performance.

### 3.5 The L-squared-M Scaling Law

A recent scaling-law result (arXiv 2503.04725, March 2025) establishes that bipartite mutual information in multi-agent world models scales as L^beta where beta lies strictly between 0 and 1, with L being the task horizon length. The exponent beta being sub-linear but positive means that inter-agent world-model memory must scale super-logarithmically with task horizon.

Concretely: if agents need to coordinate over longer time horizons, the shared memory required grows as a power law, not a logarithm. Architectures that assume O(log L) shared memory will fail at sufficiently long horizons. The system must budget memory that grows polynomially with horizon length, though the sub-linear exponent means it grows more slowly than the horizon itself.

---

## 4. Multi-Agent Sample Complexity

### 4.1 Minimax-Optimal MARL Scaling

Jiao and Li (arXiv 2412.19873, December 2024) establish that minimax-optimal multi-agent reinforcement learning (MARL) sample complexity scales additively in the number of agents under generative-model access: the total samples needed is proportional to the sum of individual action-space sizes (sum_i A_i), not their product. This is a massive reduction — multiplicative scaling (product of action spaces) is the naive expectation and would be intractable for even moderate agent counts.

This result justifies decentralized credit assignment architectures. Since optimal sample complexity is additive, each agent can learn from its own action space without requiring joint enumeration over all agents' actions. The additive structure validates approaches like difference utility (Section 1.3) where each agent is evaluated on its marginal contribution.

### 4.2 PID Measure Indeterminacy

Williams and Beer's Partial Information Decomposition (PID) is the standard framework for decomposing joint mutual information into redundant, unique, and synergistic components. However, a recent result (arXiv 2512.16662, December 2025) proves that no single PID measure can simultaneously satisfy three natural desiderata: compositionality, identity, and non-negativity.

The practical consequence is severe: synergy estimates can vary by a factor of 2 to 10 depending on which redundancy measure is chosen (I_min, I_BROJA, I_CCS, etc.). Any system that uses PID for credit assignment, routing, or attention allocation must document which specific redundancy measure it uses and understand that switching measures can qualitatively change the system's behavior. There is no "correct" measure — the impossibility result guarantees this. The choice is a design decision with real consequences.

---

## 5. Free Energy Principle Closure

### 5.1 Nested Generative Models: "As One and Many"

Pezzulo and Friston ("As One and Many," Entropy 27:143, February 2025) formalize the conditions under which a hierarchy of agents can be treated as a single collective that minimizes variational free energy. The key result: a group of agents constitutes a well-defined FEP-minimizing collective if and only if a group-level Markov blanket exists — meaning the group's internal states are statistically separated from external states by a well-defined boundary of sensory and active states.

This is not a metaphor. It is a mathematical closure condition. If the Markov blanket exists, then the collective provably behaves as if it has beliefs about its environment and acts to confirm those beliefs (the FEP interpretation). If the blanket does not exist — if there are unmediated causal pathways between individual agents and the environment that bypass the group boundary — then the collective has no well-defined generative model and the FEP formalism does not apply.

The architectural implication is direct: the system must enforce the group-level Markov blanket. All agent-environment interactions must be mediated through a defined interface layer. Any agent that directly observes or acts on the environment without going through the group boundary violates the closure condition. This is not optional for systems that claim FEP-based coordination — it is the mathematical precondition.

---

## 6. Time-Series Foundation Models as World Models

A multi-agent system that coordinates over time needs a world model — a predictive representation of how the environment evolves. Recent time-series foundation models (TSFMs) and world models make this practical at scale for the first time.

### 6.1 Time-Series Foundation Models

**Toto** (Datadog AI Research, arXiv 2505.14766, May 2025) is a 151-million parameter TSFM trained on approximately 2.36 trillion tokens — the largest open time-series training corpus to date, with 70% sourced from real Datadog operational telemetry. On GIFT-Eval (the current standard benchmark), Toto achieves an average rank of 5.495, MASE of 0.673, and CRPS of 0.437, which is state-of-the-art as of May 2025. Its training on operational telemetry makes it particularly suited for predicting regime shifts in system-level metrics — CPU usage, memory pressure, latency spikes, error rates — the kind of signals an agent runtime generates continuously.

**Chronos-2** (Amazon, arXiv 2510.15821, October 2025) achieves state-of-the-art results on fev-bench and GIFT-Eval while processing over 300 time series per second on a single A10G GPU. The Chronos-Bolt variant trades some accuracy for 250 times faster inference and 20 times lower memory usage, making it viable for real-time edge deployment. The model has accumulated approximately 120 million downloads on HuggingFace, indicating broad production adoption.

**Moirai-MoE** (Salesforce, arXiv 2410.10469) applies mixture-of-experts to time-series forecasting, achieving 17% improvement over the dense Moirai baseline while activating 65 times fewer parameters per forward pass. This sparse-activation approach is particularly relevant for edge or resource-constrained deployments where per-inference compute budgets are tight.

**TimesFM-2.5** (Google, September 2025) uses a 200-million parameter decoder-only architecture that ranks first on GIFT-Eval for both MASE and CRPS in zero-shot evaluation, with a 16,384-token context window. The decoder-only architecture enables open-ended autoregressive rollouts — the model can generate arbitrarily long forecast trajectories, making it suitable for long-horizon planning scenarios.

**Time-MoE** (ICLR 2025 Spotlight, arXiv 2409.16040) is the first work to validate neural scaling laws for time-series at 2.4 billion parameters. Its multi-resolution forecast heads — producing predictions at multiple temporal granularities simultaneously — provide a natural fit for hierarchical timescale architectures where different system components operate at different temporal resolutions (fast reactive loops, medium deliberative cycles, slow strategic planning).

### 6.2 Video and World Models

**V-JEPA 2** (Meta FAIR, arXiv 2506.09985) is a 1.2-billion parameter model that demonstrates the first successful transfer from action-free video pretraining (over one million hours of web video) to zero-shot robot control (with only 62 hours of unlabeled robot data for fine-tuning). It predicts in latent embedding space rather than pixel space, avoiding the computational burden of high-dimensional pixel reconstruction. Conceptually, this is an implementation of the active-inference idea that agents should predict in the space of sufficient statistics, not raw observations.

**DreamerV3** (Hafner et al., Nature 640:647, April 2025) achieves a remarkable result: a single fixed configuration (same hyperparameters, same architecture) that solves over 150 tasks across different domains, including being the first system to mine a diamond in Minecraft from scratch. This validates the feasibility of general-purpose learned world models — the idea that one model can capture enough environmental dynamics to support diverse behavior without task-specific tuning.

**Drama** (arXiv 2410.08893, ICLR 2025) demonstrates that world models do not require massive compute. Using a Mamba-based architecture with 7 million parameters, it runs on a laptop with O(n) memory scaling (linear in sequence length, compared to O(n^2) for transformer-based alternatives). This makes learned world models practical for embedded or resource-constrained agent deployments.

**Genie 3** (DeepMind, August 2025) generates 720p, 24-frames-per-second interactive world simulations. It remains in closed research preview but represents the frontier of generated-environment fidelity.

### 6.3 The Metacontroller Breakthrough

Kobayashi et al. (DeepMind, arXiv 2512.20605, December 2025) introduce a higher-order metacontroller that operates on the residual-stream activations of a base model to discover temporally abstract actions. Rather than operating at the token level (where every prediction step represents the same fixed timescale), the metacontroller steers the base model's internal computation to discover emergent "options" — multi-step action sequences that span variable time horizons.

This is the cleanest published implementation of agents discovering what timescale to reason at, rather than having timescales imposed by architecture. The mapping to multi-agent systems is direct: the base model corresponds to the low-level execution engine (processing individual observations and actions), the metacontroller corresponds to a new temporal-abstraction layer (deciding when to intervene, when to let low-level execution proceed autonomously, and when to commit to multi-step plans), and the emergent options correspond to compressed behavioral patterns that can be stored in memory consolidation for reuse.

This result is architecturally significant because it unifies three previously separate concerns — temporal abstraction, option discovery, and model-based planning — into a single trained component.

### 6.4 Hierarchical RL and Temporal Composition

**LDSC** (arXiv 2503.19007) demonstrates that using large language models to provide semantic guidance for hierarchical reinforcement learning improves average reward by 55.9% over baselines. The LLM provides high-level subgoal proposals; the RL system learns low-level policies to achieve them. This validates the value of natural-language-mediated hierarchy, where high-level planning operates in language space and low-level execution operates in continuous control space.

**CPD-Option-Critic** (arXiv 2510.24988) uses change-point detection over trajectory time series to automatically segment behavior into options. Rather than hand-defining skill boundaries, the system identifies statistical change points in the agent's observation stream — moments where the trajectory statistics shift — and uses these as natural option boundaries. Applied to an agent runtime's tick stream, this would automatically discover behavioral modes and their transition points.

### 6.5 Allostasis as the Missing Architectural Layer

Neuroscience research in 2024-2025 has consolidated around a revised understanding of the brain's predictive hierarchy. The traditional view placed homeostasis (reactive error correction) at the top. The updated consensus places allostasis — predictive pre-correction based on anticipated future states — above homeostasis. Rather than waiting for a deviation and correcting it, allostatic systems forecast future deviations and adjust set-points proactively.

In an agent architecture with multiple timescale layers (fast reactive, medium deliberative, slow strategic), allostasis would sit as an explicit head above the slowest layer. Its inputs would be long-horizon forecasts from time-series foundation models (Toto, Chronos-2, TimesFM). Its outputs would be low-dimensional set-points that bias the priors of faster layers — adjusting the baseline expectations that drive reactive and deliberative behavior before deviations actually occur.

No published engineering implementation of an explicit allostatic controller in an agent system exists as of this writing. Implementing one would be a first, and it would require solving two integration problems: feeding TSFM outputs into a set-point generator, and feeding those set-points into the prior-specification mechanism of the faster layers.

### 6.6 Test-Time Compute

DeepSeek-R1 demonstrated that investing more computation at inference time can dramatically improve performance: AIME accuracy jumped from 15.6% to 71% (86.7% with majority voting) at 70% lower cost than OpenAI's o1. This establishes that the ratio of training compute to inference compute is a tunable design parameter with large effect sizes.

**FutureWeaver** (arXiv 2512.11213, December 2025) provides an explicit architecture for managing test-time compute across time horizons: a dual-level orchestrator with budget-aware short-horizon action selection and long-horizon speculative planning. The short-horizon component decides what to do next given the current budget; the long-horizon component invests compute in speculative reasoning about future states, subject to a budget constraint that prevents the system from deliberating indefinitely.

This mirrors the gamma/delta timescale split found in neural oscillation research: fast gamma oscillations (30-100 Hz) handle immediate perception and action, while slow delta oscillations (0.5-4 Hz) handle long-range integration and planning. A system implementing both time horizons with explicit budget allocation between them would recapitulate this biological architecture.

---

## 7. Design Constraints from Information Theory

The results surveyed above impose six hard constraints that any multi-agent system design must respect:

**Constraint 1: Aaronson — O(1/epsilon-squared) bits per reconciliation.** Two agents need only ~400 bits to reach 5% agreement, regardless of internal knowledge. Communication protocols that scale with per-agent state size (rather than desired agreement precision) are over-engineered. Budget pairwise reconciliation at O(1/epsilon-squared) bits; anything beyond that is engineering overhead, not information-theoretic necessity.

**Constraint 2: Crutchfield — C_mu memory floor.** The statistical complexity of the environment sets an absolute minimum on agent memory. No compression scheme, however clever, can go below C_mu without destroying optimal prediction. Before choosing representation dimensionality, estimate the environment's statistical complexity. Any representation shorter than C_mu bits is provably insufficient.

**Constraint 3: Peng-Garg-Kleinberg — competence-region awareness required.** Deterministic collaboration without competence routing can underperform the worst individual agent. Task routing must be input-dependent and based on per-agent competence models. Fixed voting rules, fixed ensemble weights, and round-robin scheduling are all vulnerable to the NFL result.

**Constraint 4: Vicsek — rho = 0.23 sits on the critical curve, catastrophic decoherence risk.** Multi-agent coherence undergoes a phase transition. Operating near the critical density maximizes information transfer but also maximizes fragility. Before scaling agent count, run a phase-diagram sweep to map the coherence-decoherence boundary. Crossing it inadvertently is a safety failure, not a performance issue.

**Constraint 5: Goedel — self-improvement is bounded by the chosen logic.** A system cannot verify improvements beyond its proof system's expressive power. Ship an explicit axiomatization of utility, environment priors, and proof system. Without it, "self-improvement" is undefined. Formally, the system's self-modeling capacity is bounded by the consistency strength of its logical foundations.

**Constraint 6: PID — measure choice matters by a factor of 2 to 10.** There is no uniquely correct decomposition of joint information into redundant, unique, and synergistic components. The choice of PID measure is a design decision that quantitatively affects credit assignment, routing, and attention allocation. Document the chosen measure, understand its properties, and do not assume results transfer across measures.

### Summary Table

| Constraint | Source | Bound | Implication |
|---|---|---|---|
| Agreement bits | Aaronson (STOC 2005) | O(1/epsilon^2) bits, ~400 for epsilon=0.05 | Communication cost decoupled from agent complexity |
| Memory floor | Crutchfield-Shalizi | C_mu bits minimum | Cannot compress below statistical complexity |
| Collaboration NFL | Peng-Garg-Kleinberg (AAAI 2025) | Deterministic strategies sometimes lose to worst agent | Must route by competence region |
| Coherence transition | Vicsek/Mateo/Brown | Critical rho ~ 0.2-0.3 | Phase-diagram sweep before scaling |
| Self-improvement | Goedel/Hutter | Bounded by proof system | Explicit axiomatization required |
| PID indeterminacy | Williams-Beer (Dec 2025) | 2-10x variation across measures | Document and fix measure choice |

---

## 8. Synthesis: What These Results Mean Together

The information-theoretic results and the time-as-primitive results are not independent — they constrain each other in specific ways.

The Aaronson bound says coordination is cheap in bits. The Tishby IB bound says compression is costly in predictive power. Together, they imply that the right architecture compresses aggressively within each agent (riding down the IB curve until approaching the C_mu floor from Crutchfield), then exchanges only the minimal bits needed for agreement (the Aaronson bound). Fat inter-agent communication channels carrying full state representations are doubly wasteful: they transmit more than needed for agreement and they delay the compression that would improve generalization.

The Peng-Garg-Kleinberg NFL theorem and the Vicsek phase transition together constrain scaling strategy. NFL says you cannot scale by adding agents and voting; Vicsek says you cannot scale by adding agents at fixed density without risking decoherence. Competence-aware routing (the NFL escape) must be combined with phase-aware density management (the Vicsek constraint). The system must know not only which agent to route to, but whether the current agent density is safe.

The metacontroller result from Kobayashi et al. and the CPD-Option-Critic provide the mechanisms for making time a first-class primitive: learned temporal abstractions that emerge from data, not from hand-engineered timescale hierarchies. Combined with time-series foundation models for long-horizon forecasting and the allostatic-controller design for proactive set-point adjustment, these components form a complete stack for temporal reasoning — from fast reactive responses to slow strategic reorientation.

The Garrabrant logical induction result provides the coherence standard: beliefs should be updated in a way that no bounded adversary can exploit. The Goedel ceiling sets the self-improvement boundary: the system can only verify improvements within its proof system. Together with the Wolpert thermodynamic equalities (which constrain the physical cost of computation) and Lloyd's ultimate limits (which confirm that engineering, not physics, is the binding constraint), these results define the space within which practical agent systems must operate.

The L-squared-M scaling law and the Jiao-Li MARL sample-complexity result together determine memory and sample budgets. World-model memory must scale super-logarithmically with task horizon (L^beta, beta in (0,1)), but sample complexity scales only additively across agents (sum of individual action spaces, not their product). This means long-horizon coordination is memory-expensive but sample-efficient — the bottleneck is representation capacity, not data.

Finally, the FEP closure condition from Pezzulo and Friston provides the mathematical license for treating the entire agent hierarchy as a unified system, but only if the Markov blanket is enforced architecturally. Without it, the collective has no well-defined generative model, and techniques based on free-energy minimization (including EFE-based routing) lose their theoretical justification.

---

## References

- Aaronson, S. (2005). The Complexity of Agreement. STOC 2005. arXiv: cs/0406061.
- Peng, B., Garg, S., Kleinberg, R. (2025). No Free Lunch for Deterministic Collaboration. AAAI 2025. arXiv: 2411.15230.
- Wolpert, D.H., Tumer, K. (2004). Collectives and the Design of Complex Systems. Springer.
- Crutchfield, J.P., Shalizi, C.R. (2001). Computational Mechanics: Pattern and Prediction, Structure and Simplicity. Journal of Statistical Physics.
- Tishby, N., Pereira, F., Bialek, W. (2000). The Information Bottleneck Method. Proceedings of the 37th Allerton Conference.
- Sims, C. (2003). Implications of Rational Inattention. Journal of Monetary Economics.
- Bloedel, A., Denti, T., Pomatto, L. (2025). Rational Inattention with f-Divergence Costs. October 2025.
- Tajima, S., Drugowitsch, J., Pouget, A. (2016). Optimal Policy for Value-Based Decision-Making. Nature Communications.
- Mateo, D. et al. (2014). Swarm Phase Transitions. arXiv: 1409.7207.
- Brown, R., Bossomaier, T., Barnett, L. (2017). Information Transfer in Swarms. arXiv: 1710.06589.
- Garrabrant, S. et al. (2016). Logical Induction. arXiv: 1609.03543.
- Wolpert, D.H. (2024). Stochastic Thermodynamics of Computation. PNAS 2024.
- L-squared-M Scaling Law. (2025). arXiv: 2503.04725.
- Jiao, Y., Li, G. (2024). Minimax-Optimal MARL Sample Complexity. arXiv: 2412.19873.
- Williams, P.L., Beer, R.D. (2025). PID Measure Indeterminacy. arXiv: 2512.16662.
- Pezzulo, G., Friston, K. (2025). As One and Many. Entropy 27:143.
- Toto (Datadog). (2025). arXiv: 2505.14766.
- Chronos-2 (Amazon). (2025). arXiv: 2510.15821.
- Moirai-MoE (Salesforce). (2024). arXiv: 2410.10469.
- TimesFM-2.5 (Google). September 2025.
- Time-MoE. (2025). ICLR 2025 Spotlight. arXiv: 2409.16040.
- V-JEPA 2 (Meta FAIR). (2025). arXiv: 2506.09985.
- DreamerV3 (Hafner et al.). (2025). Nature 640:647.
- Drama. (2025). ICLR 2025. arXiv: 2410.08893.
- Genie 3 (DeepMind). August 2025.
- Kobayashi et al. (DeepMind). (2025). Higher-Order Metacontroller. arXiv: 2512.20605.
- LDSC. (2025). arXiv: 2503.19007.
- CPD-Option-Critic. (2025). arXiv: 2510.24988.
- FutureWeaver. (2025). arXiv: 2512.11213.
