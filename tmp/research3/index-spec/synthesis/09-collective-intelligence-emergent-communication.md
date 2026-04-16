# Collective Intelligence Scaling Laws, Emergent Communication, and Multi-Agent Coordination

This document synthesizes the current state of research (2024--2026) on how groups of AI agents scale, communicate, coordinate, and fail. It is written for someone with no prior context on multi-agent systems or the research literature. Every claim is traced to a specific paper with its arXiv identifier.

---

## 1. The 64-Agent Plateau: Empirical Scaling Laws

A natural question when building systems of multiple AI agents is: how many agents should you use? Intuition suggests more is better. The empirical evidence says otherwise.

**Dochkina** (Moscow Institute of Physics and Technology, arXiv:2603.28990, March 2026) conducted the most comprehensive multi-agent scaling study to date. The experimental design was unusually thorough: 8 different LLM models, agent counts ranging from 4 to 256, 8 distinct coordination protocols, and 4 levels of task difficulty, producing over 25,000 completed tasks. The central finding is a plateau: **scaling from 64 to 256 agents yields no statistically significant quality improvement.** The Kruskal-Wallis H-test gives H=1.84, p=0.61 -- meaning you cannot reject the null hypothesis that 64 and 256 agents produce identical quality. Measured quality scores remain stable in the range Q=[0.949, 0.967] with standard deviation approximately 0.04.

Three additional findings from the Dochkina study deserve attention:

**Protocol matters more than headcount.** The "Sequential Hybrid" protocol -- where agents work in sequence, each refining the previous agent's output, with the option to fork into parallel sub-teams for independent subtasks -- beats the "Coordinator" protocol (a single agent assigning and reviewing work) by +14% and "Shared Autonomy" (all agents working on a shared workspace simultaneously) by +44%. The effect size is large: Cohen's d=1.86 for Sequential Hybrid vs. Shared Autonomy. This means the way agents communicate determines quality far more than how many you deploy.

**Agents self-regulate at scale.** At N=256, 45% of agents voluntarily abstain from contributing -- they evaluate the current state of the work, conclude they have nothing useful to add, and remain silent. This is not a bug. It is endogenous cost optimization: agents that would add noise instead conserve resources. The system discovers its own effective team size.

**The practical recommendation is a hard cap.** Structure agent clusters at N=64 maximum. Above that threshold, shard the problem into independent sub-problems, each handled by its own cluster of up to 64 agents. This avoids the coordination overhead that eats any marginal quality gain from additional agents.

---

## 2. The Communication Density Threshold at rho=0.23

Communication density is the fraction of possible agent-to-agent communication links that are actually used. In a fully connected network of N agents, every agent talks to every other agent -- density is 1.0. In a ring network, each agent talks only to its two neighbors -- density is approximately 2/N. The question is: what density produces the best results?

Three independent research groups converge on the same answer: somewhere around 0.2--0.4, dramatically lower than full connectivity.

### 2.1 Ring Topology Matches Full Debate

**Li et al.** (Google, EMNLP 2024, arXiv:2406.11776) tested ring topology (each agent communicates only with its two immediate neighbors, giving approximately 0.22 communication density) against fully-connected debate (every agent reads every other agent's response) on GSM8K, MATH, and MMLU. Ring topology **matches or beats** fully-connected debate while saving 40--60% of tokens. The mechanism: in fully-connected debate, agents spend most of their context window reading what all other agents said. With ring topology, each agent reads only two perspectives and synthesizes them. Per-agent cognitive load is constant regardless of team size.

### 2.2 Logistic Saturation and Small-World Networks

**MacNet** (Qian et al., ICLR 2025, arXiv:2406.07155) tested networks with over 1,000 agents and found performance follows a **logistic saturation curve** with the knee at N approximately 32. The winning topology is the **small-world network** (Watts-Strogatz model, rewiring probability 0.1--0.3) -- a ring with random shortcut links that combines high local clustering with short average path length and the token efficiency of sparse communication.

### 2.3 DeepMind's Optimal Communication Density

**Kim et al.** (DeepMind, arXiv:2512.08296, December 2025) studied 180 agent configurations and measured optimal communication density at **c*=0.39 messages per agent per turn**. More communication degrades performance. They also quantified **error amplification**: independent multi-agent systems amplify errors 17.2x vs. a single agent; centralized systems amplify 4.4x. Sparse communication with local validation minimizes amplification -- direct motivation for topologies where small groups validate each other's work.

### 2.4 Convergence

Li et al.'s ring at density 0.22, MacNet's small-world optimum, and Kim et al.'s c*=0.39 all converge: **multi-agent systems should communicate sparsely, locally, and with short-path global shortcuts.** Fully-connected communication wastes tokens and degrades quality. Independent work without communication is fragile. The optimum lies closer to sparse than dense.

---

## 3. Heterogeneity Dominates Raw Agent Count

Even within the 64-agent budget, the composition of the team matters far more than its size.

**Yang et al.** (ICML 2026, arXiv:2602.03794) provide the information-theoretic foundation. They prove that the mutual information a multi-agent system can extract about a target variable Y given input X is bounded:

    I_MAS(n) <= H(Y|X)

where H(Y|X) is the conditional entropy of the target. The key is how quickly a system approaches this bound. Yang et al. show **geometric contraction**: the residual uncertainty after K effective information channels is bounded by

    residual <= e^(-alpha * K) * H(Y|X)

where alpha is the **complementarity rate** -- a measure of how different the agents' perspectives are. When agents are diverse (high alpha), each additional agent provides genuinely new information, and residual uncertainty drops exponentially. When agents are homogeneous (low alpha), each additional agent provides redundant information, and the exponential decay stalls.

The empirical consequence is dramatic: **2 diverse agents match or exceed 16 homogeneous agents** -- an 8x reduction in compute at equivalent performance. Diversity here means agents with different model architectures, different training data distributions, different prompting strategies, or different tool access. The complementarity rate alpha is the quantitative lever; raw agent count N is secondary.

### The K* Metric

Yang et al. introduce a practical measurement called **K***, the entropy effective rank of the Gram matrix of agent output embeddings. To compute K*: collect the outputs of all agents for a set of tasks, embed each output in a shared vector space, form the Gram matrix (pairwise cosine similarities), compute its eigenvalue spectrum, and take the exponential of the Shannon entropy of the normalized eigenvalues. K* measures how many "effectively independent perspectives" the agent team provides. A team of 16 identical agents might have K*=1.3; a team of 4 diverse agents might have K*=3.8.

K* is **label-free** -- no ground-truth answers needed -- making it directly usable as a runtime quality signal. If K* drops over time, the team needs different agent types, not more of the same. The ratio Kc*/Kw* (diversity on correct vs. incorrect paths) provides finer diagnostics: when wrong-path diversity exceeds correct-path diversity, the team is generating varied mistakes -- a signal to restructure.

---

## 4. Emergent Communication: How Agents Invent Their Own Languages

When agents interact repeatedly, they can develop compressed communication protocols that are more efficient than natural language. This phenomenon -- emergent communication -- has been studied in simplified settings since the 2010s, but 2024--2025 research demonstrates it with LLM agents at practical scale.

### 4.1 Agora: Self-Organizing Protocol Negotiation

**Marro et al.** (Oxford, arXiv:2410.11905, October 2024) built "Agora," a system where 100 LLM agents negotiate their own communication protocols. Instead of pre-defining message formats, agents propose, test, and iteratively refine JSON-schema-based routines for specific interaction types. Over the course of a task, agents converge on compressed protocols tailored to the problem at hand.

The result: **5x cost reduction** compared to agents communicating in unconstrained natural language. The emergent protocols strip away politeness markers, redundant context-setting, and verbose explanations, retaining only the information-theoretically necessary content. Protocols are stored as JSON-schema'd routines, each hashed (originally via SHA1) for lookup and versioning.

A natural extension is to replace cryptographic hashing with **similarity-addressable fingerprinting** -- encoding each protocol's semantics as a high-dimensional binary vector (a "hyperdimensional computing" or HDC fingerprint). Under this scheme, similar protocols produce similar fingerprints, so agents can find near-matches even across version drift or minor variations. This is impossible with SHA1, where a single-character change produces an entirely different hash.

### 4.2 Modular Composite Representations for Agent Communication

**Angioli, Kymn, Loutfi, and Kleyko** (arXiv:2511.09708, November 2025) provide the computational substrate for structured agent messages. Their Modular Composite Representations (MCR) framework achieves **3.08x faster execution and 2.68x lower energy consumption** compared to Binary Spatter Codes (the baseline for hyperdimensional computing binding and bundling). Combined with Agora-style protocol negotiation, agents can both invent compressed protocols and execute them at near-zero marginal cost per message.

### 4.3 Stigmergy and the Necessity of Decay

Stigmergy is indirect communication through the environment -- agents leave traces (like ant pheromones) that other agents read and respond to. In software systems, stigmergic communication means agents write to a shared data store (code repositories, databases, configuration files) rather than sending direct messages.

**Govcraft** (arXiv:2601.08129) formalizes stigmergic multi-agent coordination with mathematical rigor, producing several theorems. The most consequential is **Theorem 3 (Basin Separation)**: temporal decay of stigmergic traces is **mathematically necessary** to escape suboptimal basins. Without decay, early traces dominate the landscape permanently, and the system converges to whichever solution was explored first, regardless of quality. With decay, old traces fade, creating exploration pressure that allows the system to discover better solutions.

This result formalizes the Ebbinghaus forgetting curve -- the empirical observation from cognitive psychology that memory strength decays exponentially over time unless reinforced -- as a design requirement rather than a biological accident. Any multi-agent system using shared persistent state must implement active decay, or it will lock in to early (likely suboptimal) solutions.

### 4.4 The Population-Scale Trap

Not everything about emergent communication is positive. **Chaabouni et al.** (ICLR 2022) demonstrated that **scaling population size alone does NOT induce compositional communication protocols**. Compositionality -- the ability to combine atomic symbols into structured meanings, as human language does -- requires specific conditions beyond mere scale:

- **Heterogeneity**: agents must differ in their processing or representation, so that communication is genuinely necessary (homogeneous agents would converge on identical internal representations, making communication redundant).
- **Learning-speed asymmetry**: when some agents learn faster than others, the slower agents create pressure for the faster ones to communicate more clearly, driving the emergence of structured protocols.
- **Length cost**: Chaabouni et al. (2019) showed that penalizing message length is necessary to drive compression. Without length cost, agents develop verbose, non-compositional codes that are technically functional but do not generalize.

Additionally, Lossy Iterated Learning (arXiv:2511.18220, November 2025) extends Fano's inequality to show that even a **fraction of a bit** of channel capacity difference can flip a population from accumulating compositional knowledge to plateauing. The channel capacity through which agents communicate is a phase-transition parameter, not a continuous dial.

The practical implication: building a large fleet of identical agents and hoping they develop efficient communication is a recipe for wasted compute. Instead, deliberately engineer heterogeneity (different models, different tools, different roles) and communication bottlenecks (limited bandwidth, sparse topology) to create the pressure that drives compositional emergence.

---

## 5. PID Diagnostics: Measuring Coordination Health

How do you know whether a group of agents is actually coordinating, or merely coexisting? **Partial Information Decomposition (PID)** provides a mathematical framework for answering this question.

### 5.1 Synergy, Redundancy, and Unique Information

PID, following the Williams-Beer-Lizier-Mediano lattice, decomposes the information that a group of variables provides about a target into three components:

- **Redundancy**: information that multiple agents provide identically. High redundancy means agents are doing the same work and providing the same insights. It is waste.
- **Unique information**: information that only one specific agent provides. High unique information means agents are specialized and non-overlapping.
- **Synergy**: information that is only available from the *combination* of multiple agents -- no single agent provides it alone. High synergy means agents are genuinely coordinating to produce insights that none could produce individually.

**Riedl** (Northeastern University, arXiv:2510.05174, October 2025) applies PID to multi-agent systems by computing time-delayed mutual information over agent state trajectories. Rather than looking at agent outputs in isolation, Riedl tracks how agent states evolve over time and measures the information-theoretic structure of their joint dynamics.

The key finding: **synergy serves as an order parameter for coordination phase transitions.** When synergy crosses a threshold, the system transitions from a collection of independent agents to a genuinely coordinated collective. Below the threshold, agents may appear to be collaborating (they share a workspace, they exchange messages) but are producing no information that requires their combination. Above the threshold, the group produces emergent capabilities.

This gives a concrete monitoring strategy: compute PID metrics over rolling windows of agent activity. When synergy drops below a threshold and redundancy rises, the system is degenerating into redundant parallel work. The appropriate response might be to reduce the number of agents, increase agent diversity, or restructure the communication topology.

### 5.2 Sheaf Consensus: Distributed Agreement with Mathematical Guarantees

**Cellular sheaf theory** provides a complementary framework. A cellular sheaf assigns a data space to each agent and a "restriction map" to each communication link, specifying how local data relates across agents. Agreement is not "identical outputs" but "outputs consistent when translated through restriction maps."

**Hanks and Riess et al.** (arXiv:2504.02049 and arXiv:2510.00270, 2025) prove ADMM convergence for cellular-sheaf-based distributed optimization with **bounded asynchronous delays** and **Lyapunov-stable tracking** -- the system provably converges even when the target moves. Originally from control theory, the framework applies directly to heterogeneous multi-agent LLM coordination.

The key diagnostic: the first cohomology group H1 of the sheaf. When H1=0, all local consistencies compose globally. When H1 is nontrivial, agents appear pairwise consistent but the system is globally contradictory -- the signature of subtle coordination failures invisible to pairwise checking.

### 5.3 The Open Question: Sheaf Cohomology Equals PID Synergy?

An intriguing open research question connects these two frameworks: **does H1(sheaf)=0 hold if and only if PID synergy is positive?** If the sheaf has no global obstructions (H1=0), does that correspond to the presence of genuine synergistic coordination (synergy > 0)? If so, sheaf cohomology and PID synergy are measuring the same underlying phenomenon from different mathematical angles, and one can use whichever is computationally cheaper for a given monitoring task.

No published work has tested this correspondence empirically. Validating or refuting it would constitute a significant contribution to the theory of multi-agent coordination.

---

## 6. Failure Mode Catalog: The 14 Ways Multi-Agent Systems Break

Understanding how multi-agent systems fail is as important as understanding how they succeed.

**MAST** (Cemri et al., NeurIPS 2025, arXiv:2503.13657) provides the most systematic catalog to date: **14 distinct failure modes**, identified through extensive annotation of multi-agent task executions with **inter-annotator agreement kappa=0.88** (indicating strong human consensus on failure categorization).

These failure modes span multi-agent pathologies: mutual contradiction, redundant work, withheld critical information, misinterpreted coordinator instructions, incompatible output formats, infinite delegation loops, over-specialization, and failure to aggregate partial results.

Each failure mode maps to a detection mechanism: contradiction via sheaf cohomology obstructions, redundancy via PID spikes, delegation loops via graph cycle detection, format incompatibility via schema validation. Each detected failure triggers proportional consequences -- quality deductions, reputation penalties, or economic slashing.

---

## 7. Stigmergy-as-Immune-Tissue: Detecting Pathology in Shared Environments

When agents communicate through a shared persistent environment (stigmergy), that environment accumulates traces of all agent activity. This creates a unique opportunity: the environment itself can serve as an immune system, detecting pathological agent behavior the way biological immune systems detect pathogens.

The framework draws on **danger theory** from immunology (Matzinger 1994, refined through 2000s), which classifies signals into three categories:

- **PAMPs (Pathogen-Associated Molecular Patterns)**: known-bad signatures. In a multi-agent context, these are traces that match previously catalogued attack patterns -- prompt injections with known structure, outputs matching known hallucination patterns, activity patterns matching known Sybil coordination signatures.
- **Safe signals**: traces from healthy activity. Normal agent outputs, consistent with historical patterns, bearing valid provenance, within expected parameter ranges.
- **Danger signals**: traces that are not known-bad but indicate tissue damage -- anomalous activity patterns, unexpected state mutations, provenance gaps, output distributions that diverge sharply from historical baselines.

**Dendritic-cell-style temporal correlation** processes these signals by tracking the ratio of danger signals to safe signals over a sliding time window. A single anomalous trace is not actionable (it might be legitimate exploration). A sustained elevation triggers investigation.

**Auto-quarantine of necrotic regions** isolates sections of the shared environment where danger signals dominate, walling them off from agent writes pending review while preserving read access for context.

**Bond-slash on emission** adds economic consequences. Agents posting traces have a stake (deposited bond or accumulated reputation). If their trace is later classified as pathological, a portion is forfeited -- creating direct incentive for self-filtering.

Why stigmergic substrates uniquely enable this: in direct-messaging architectures, messages exist only between sender and receiver with no shared inspection surface. In stigmergic architectures, every agent action leaves a visible trace in the shared environment. The environment is simultaneously communication medium and diagnostic surface.

The threat motivating this design is real. **Prompt Infection** (ICLR 2025, arXiv:2410.07283) demonstrates self-propagating viral injection across multi-agent systems -- a single poisoned agent can contaminate others through shared context, and the contamination spreads geometrically. Multi-agent prompt injection is OWASP's #1 risk for LLM applications (2025 edition). Without an immune-like defense, stigmergic environments are trivially exploitable as infection vectors.

---

## 8. Sheaf-Consistent Reputation That Detects Emergent Deception

Multi-agent systems where agents have economic incentives (payments, reputation, competitive ranking) face a fundamental problem: agents will manipulate. Two recent papers make this concrete:

- **MAD-Spear** (arXiv:2507.13038, July 2025) demonstrates Sybil-coordinated manipulation where a group of colluding agents uses conformity pressure to herd honest agents toward desired conclusions.
- **AdapAM** (arXiv:2511.15292, November 2025) shows that strict black-box adaptive attacks against multi-agent reputation systems are now feasible -- attackers can learn to game reputation without access to the system's internals.

Detecting such manipulation requires comparing what agents *say* with what they *believe*. This is possible through several complementary mechanisms:

**Thought Communication latent extraction** (arXiv:2510.20733) refers to techniques for inferring an agent's internal reasoning state from its outputs -- not what it explicitly states, but what its output patterns reveal about its actual processing. When an agent's stated conclusions diverge from what its latent processing suggests, this is a flag for potential deception.

**Cellular sheaf consensus** (Hanks/Riess et al., arXiv:2504.02049 + arXiv:2510.00270, 2025) provides the detection framework. Construct a sheaf where each agent's data space includes both stated outputs and inferred latent state, with restriction maps encoding the expected relationship. When H1 is nontrivial, there is a **global obstruction** -- agents appear pairwise consistent but the system as a whole is contradictory. This is the signature of coordinated deception.

**PID synergy/redundancy diagnostics** (Riedl, arXiv:2510.05174) provide a complementary channel. Under manipulation, redundancy spikes (colluding agents echo each other) while synergy drops. Monitoring the synergy/redundancy ratio over time creates an early warning system for emerging collusion.

**On-chain reputation** anchors these diagnostics to economic consequences. Reputation scores on an append-only ledger are reduced proportionally when sheaf cohomology detects obstruction. Agents cannot erase deception flags.

The four mechanisms form defense in depth: latent extraction catches individual deception, sheaf cohomology catches coordinated deception, PID catches statistical patterns, and economic consequences create deterrence.

---

## 9. Implications for Agent System Design

The research surveyed above converges on several concrete design principles for multi-agent systems:

### 9.1 Hard-Cap Clusters at 64 Agents

Beyond 64 agents, quality gains are statistically indistinguishable from noise (Dochkina, arXiv:2603.28990). For problems requiring more agents, shard into independent sub-problems with minimal inter-cluster communication.

### 9.2 Use Ring or Small-World Topology, Not Fully Connected

Three independent studies agree: fully-connected communication wastes tokens and degrades performance. Small-world topology (ring plus random shortcuts, Watts-Strogatz rewiring probability 0.1--0.3) provides the best balance. Target communication density of 0.2--0.4.

### 9.3 Prioritize Diversity Over Scale

Two diverse agents match 16 homogeneous agents at 8x lower compute (Yang et al., arXiv:2602.03794). Monitor K* (entropy effective rank of output embedding Gram matrix) at runtime. Engineer diversity deliberately across model architectures, tool access, prompting strategies, and assigned roles.

### 9.4 Engineer Communication Bottlenecks to Drive Protocol Emergence

Compositional communication emerges from pressure, not abundance (Chaabouni et al., ICLR 2022; arXiv:2511.18220). Limit bandwidth, introduce heterogeneity, and let agents negotiate protocols (Agora, arXiv:2410.11905, achieves 5x cost reduction this way).

### 9.5 Implement Active Decay on Shared State

Temporal decay is mathematically necessary to escape suboptimal convergence in stigmergic systems (Govcraft Theorem 3, arXiv:2601.08129). Old information must lose weight unless actively reinforced.

### 9.6 Monitor Synergy and Redundancy as Health Metrics

Compute PID synergy and redundancy over rolling windows (Riedl, arXiv:2510.05174). Dropping synergy with rising redundancy signals coordination degeneration; trigger automatic restructuring.

### 9.7 Map Failure Modes to Economic Consequences

Use the MAST 14-failure-mode catalog (arXiv:2503.13657) as a detection checklist. Graduated responses: quality deductions for minor failures, reputation penalties for moderate failures, bond slashing for severe failures.

### 9.8 Build Deception Detection Into the Coordination Layer

Agents with economic incentives will manipulate (MAD-Spear arXiv:2507.13038, AdapAM arXiv:2511.15292). Build sheaf-cohomology / PID-synergy / latent-extraction detection into the coordination infrastructure from the start -- retrofitting is far more expensive than inclusion.

---

## 10. Summary of Key Papers

| Paper | Authors / Venue | arXiv ID | Key Finding |
|---|---|---|---|
| 64-agent plateau | Dochkina, MIPT | 2603.28990 | 64-to-256 agents: no quality gain (p=0.61); sequential hybrid +14% over coordinator |
| Ring vs. full debate | Li et al., Google, EMNLP 2024 | 2406.11776 | Ring (density ~0.22) matches full-connect at 40-60% token savings |
| MacNet scaling | Qian et al., ICLR 2025 | 2406.07155 | Logistic saturation at N~32; small-world topology wins |
| DeepMind scaling | Kim et al. | 2512.08296 | Optimal c*=0.39 msg/turn; 17.2x error amplification in independent multi-agent |
| Heterogeneity bound | Yang et al., ICML 2026 | 2602.03794 | 2 diverse agents >= 16 homogeneous; K* metric for runtime diversity |
| Agora protocols | Marro et al., Oxford | 2410.11905 | 100-agent self-organizing protocols; 5x cost reduction |
| Modular Composite Reps | Angioli/Kymn/Loutfi/Kleyko | 2511.09708 | 3.08x faster, 2.68x lower energy vs Binary Spatter Codes |
| Stigmergic decay | Govcraft | 2601.08129 | Theorem 3: decay necessary to escape suboptimal basins |
| Anti-compositional scaling | Chaabouni et al., ICLR 2022 | (published) | Population scale alone does not induce compositional protocols |
| Lossy Iterated Learning | (multiple) | 2511.18220 | Fraction of a bit flips accumulation vs. plateau |
| PID for MAS | Riedl, Northeastern | 2510.05174 | Synergy as coordination phase-transition order parameter |
| Sheaf ADMM consensus | Hanks/Riess et al. | 2504.02049, 2510.00270 | Lyapunov-stable convergence with bounded async delays |
| MAST failure catalog | Cemri et al., NeurIPS 2025 | 2503.13657 | 14 failure modes, kappa=0.88 inter-annotator agreement |
| MAD-Spear manipulation | (multiple) | 2507.13038 | Sybil conformity-driven herd manipulation |
| AdapAM adaptive attacks | (multiple) | 2511.15292 | Black-box adaptive attacks on multi-agent reputation |
| Prompt Infection | (multiple), ICLR 2025 | 2410.07283 | Self-propagating viral injection across MAS |

---

## 11. Open Research Questions

Several questions raised by this literature remain unanswered and represent opportunities for significant contributions:

1. **Does H1(sheaf)=0 correspond to PID synergy > 0?** If sheaf cohomology and PID synergy measure the same underlying coordination quality, this unification would simplify monitoring infrastructure and deepen theoretical understanding of multi-agent coordination.

2. **What is the optimal decay rate for stigmergic traces?** Govcraft proves decay is necessary but does not specify the optimal rate. Too fast, and agents lose useful context. Too slow, and the system locks into early solutions. The optimal rate likely depends on task complexity, agent count, and topology.

3. **Can K* predict coordination failures before they manifest?** If the diversity metric drops before quality drops, it could serve as a leading indicator, enabling preventive restructuring.

4. **Does the 64-agent plateau hold across all task types?** Dochkina tested four difficulty levels, but the task space is vast. Certain tasks (those requiring genuinely independent parallel work, like testing N independent modules) might benefit from larger teams. Others (those requiring tight sequential reasoning) might plateau even earlier.

5. **How do emergent protocols interact with adversarial agents?** If agents negotiate their own communication formats (Agora-style), can a malicious agent exploit protocol negotiation to inject misleading semantics? The intersection of emergent communication and adversarial robustness is largely unexplored.

These questions are independently publishable and, taken together, would constitute a comprehensive theory of multi-agent coordination grounded in both information theory and algebraic topology.
