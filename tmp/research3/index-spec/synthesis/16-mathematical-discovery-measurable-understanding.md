# Mathematical Structure Discovery, Sensory-Motor Grounding, and Measurable Understanding

This document synthesizes the research landscape across automated theorem proving, sensory-motor grounding for software agents, and operationally measurable understanding. It is written as a self-contained reference for someone with no prior exposure to the project or its research program. Every claim is grounded in a specific paper with arXiv identifier, journal reference, or benchmark dataset. The document concludes with five contributions that this stack uniquely enables, none of which appear in any competing system.

---

## 1. Mathematical Structure Discovery -- Automated Theorem Proving at Scale

Mathematics is the hardest domain for AI systems because it demands both creative conjecture and rigorous verification. A claimed proof is either correct or it is not; there is no partial credit and no subjective evaluation. This makes mathematics the cleanest testbed for genuine cognitive capability, and recent results suggest that AI systems are beginning to exhibit something beyond pattern retrieval in this domain.

### 1.1 AlphaEvolve and AlphaProof: DeepMind's Closed-Loop Discovery

**AlphaEvolve** (DeepMind, arXiv:2506.13131) represents the first convincing demonstration of an AI system that discovers genuinely new mathematics rather than rediscovering known results. The system tackled 67 open problems spanning analysis, combinatorics, geometry, and number theory. On 75% of these problems, it independently rediscovered the current state of the art -- solutions that took human mathematicians years or decades to find. On 20% of the problems, it improved upon the best known results, producing solutions that are strictly better than anything previously published.

The headline result is the first improvement to 4x4 complex matrix multiplication since Strassen's 1969 algorithm. For over fifty years, Strassen's method was the best known way to multiply two 4x4 matrices of complex numbers. AlphaEvolve found a more efficient decomposition. This is not an incremental improvement to an engineering system; it is a contribution to pure mathematics that human researchers had attempted and failed to produce for more than half a century.

The architectural detail that matters most is the recursive self-improvement closure. AlphaEvolve was used to optimize the GPU kernels that train the large language model that powers AlphaEvolve itself. This is the canonical example of recursive self-improvement (RSI) in a deployed system: the system's mathematical discoveries directly improve its own training infrastructure, which in turn improves its capacity for mathematical discovery. This loop is not hypothetical; it was executed and the kernel improvements are in production.

A follow-up collaboration between Terence Tao and Stefan Wagner (arXiv:2511.02864) demonstrated that AlphaEvolve's discoveries are not merely brute-force numerical optimization. In their analysis, AlphaEvolve's output exhibited what they describe as "genuine conceptual lift" -- the system generalized from finite-N results (solutions that work for specific problem sizes) to all-N results (solutions that work for arbitrary sizes). Generalization from specific instances to universal theorems is the hallmark of mathematical understanding, and it appeared without being explicitly trained for.

**AlphaProof and AlphaGeometry 2** (Nature 2025, doi:10.1038/s41586-025-09833-y) attacked mathematics from the formal verification side. At the 2024 International Mathematical Olympiad, the combined system scored 28 out of 42 possible points, earning a silver medal. This included solving Problem 6, which only 5 of the 609 human contestants solved. At the 2025 IMO, the system achieved gold-medal performance using Gemini Deep Think, solving problems in natural language (not formal proof) within the 4.5-hour competition window. The transition from formal-proof-based reasoning (AlphaProof) to natural-language reasoning (Gemini Deep Think) while maintaining gold-level performance suggests that formal verification and intuitive reasoning are converging capabilities rather than competing approaches.

### 1.2 Open-Source Theorem Provers: Closing the Gap

DeepMind's systems are proprietary, but open-source competitors have achieved remarkable results on standardized benchmarks.

**DeepSeek-Prover-V2-671B** achieves 88.9% on miniF2F-test at Pass@8192. The miniF2F benchmark consists of formalized math competition problems translated into the Lean proof assistant. Pass@K measures the probability of finding at least one correct proof within K sampling attempts. At 88.9% with 8,192 attempts, DeepSeek-Prover-V2 can solve nearly nine out of ten competition-level formalization problems, given sufficient compute budget.

**Goedel-Prover-V2-32B** achieves 88.1% on the same benchmark at Pass@32 -- essentially matching the 671B model's performance but with 256 times fewer sampling attempts and at a fraction of the parameter count (32 billion versus 671 billion parameters). The key technique is self-correction: the model generates a proof attempt, checks it against the Lean type checker, and uses the error messages to revise its approach. This iterative refinement loop compresses the search space dramatically, allowing a much smaller model to reach the same solutions.

**Seed-Prover 1.5** (ByteDance, arXiv:2512.17260, December 2025) is the most impressive open-source result to date. On the 2025 IMO, it fully solved 4 out of 6 problems and partially solved a fifth. The first five problems were Lean-verified within 16.5 hours, reaching the gold medal cutoff. On PutnamBench (a formalization of Putnam competition problems, which are significantly harder than IMO problems), it achieves 88%. On FATE-X, a PhD-level formal mathematics benchmark, it reaches 33%.

The architectural innovation in Seed-Prover 1.5 that matters for the broader research program is its use of heavyweight inference with conjecture and lemma pools. During proof search, the system maintains two persistent data structures: a pool of conjectures (unproven statements that the system believes might be true based on pattern recognition) and a pool of lemmas (proven intermediate results that can be reused). This is structurally an agentic episodic memory of mathematical content -- the system accumulates knowledge during a proof session and uses it to accelerate subsequent proof steps. The conjecture pool functions as an abductive hypothesis space; the lemma pool functions as a growing library of verified building blocks.

### 1.3 The Compositional Weakness: Where Current Provers Fail

Despite these impressive headline numbers, current formal theorem provers have a specific, well-characterized failure mode.

**Ineq-Comp** (arXiv:2505.12680) is a benchmark designed to test compositional reasoning in formal provers. The benchmark takes existing inequality proofs and applies simple structural transformations: duplicating a variable (replacing x with x and x'), applying basic algebraic rewriting (multiplying both sides of an inequality), or composing two proven inequalities into a third. These transformations are trivial for any human mathematician who understands the underlying proofs.

DeepSeek-Prover-V2-7B drops 20% in accuracy on these compositionally transformed problems. The critical finding is not merely the accuracy drop but the specific failure pattern: models cannot compose formal proofs from sub-proofs even when the sub-proofs are provided as context. Given a proven lemma L1 and a proven lemma L2, and asked to prove a theorem that follows directly from applying L1 and then L2, the models fail at rates far exceeding what their headline benchmark numbers would predict.

This is precisely the compositional depth collapse documented by Dziri et al. (NeurIPS 2023, arXiv:2305.18654) in the general LLM setting, now confirmed in the specific domain of formal theorem proving. The implication is clear: current provers are performing sophisticated pattern matching against their training corpora of proofs, not executing compositional proof strategies. When the required composition is not close to a training example, performance degrades sharply.

This compositional weakness is the exact gap where categorical compositional architectures -- specifically the Para(Lens(C)) framework from Gavranovic et al. (ICML 2024, arXiv:2402.15332) -- provide structural scaffolding. In a categorical architecture, composition is guaranteed to be associative by construction. A proof built by composing verified sub-proofs inherits correctness from the functorial structure, not from pattern matching. The practical path is to use categorical composition as the meta-reasoning layer that orchestrates sub-proof assembly, while using transformer-based provers for the creative step of generating individual sub-proofs.

### 1.4 Library Learning: The Wake-Sleep Loop for Mathematics

The most promising paradigm for overcoming the compositional weakness is library learning -- the automatic discovery of reusable abstractions that compress the space of programs or proofs.

**FunSearch/PatternBoost** (Charton, Wagner, Ellenberg, Williamson, arXiv:2411.00566) demonstrated this paradigm's power by disproving a conjecture that had stood open for 30 years. Graham (1992) conjectured a specific bound on a combinatorial quantity. FunSearch found a counterexample by alternating between two phases: local classical search (hill-climbing to find better solutions within a fixed program structure) and global transformer training (learning to predict which program structures are likely to contain good solutions). The local search discovers concrete improvements; the global training generalizes the patterns underlying those improvements; the cycle repeats.

This alternation between local refinement and global abstraction is structurally identical to the wake-sleep algorithm in variational inference. In the "wake" phase, the system uses its current generative model to process data and accumulates statistics about what works. In the "sleep" phase, it updates the generative model to better capture those statistics. The mathematical equivalence is not a loose analogy: both algorithms optimize a variational lower bound by alternating between inference (local search) and learning (global model update).

**Stitch** (POPL 2023) achieves 1,000 to 10,000 times faster library compression than DreamCoder (the seminal library learning system) with 100 times lower memory consumption. Stitch uses a top-down, greedy algorithm for identifying reusable program fragments, avoiding DreamCoder's expensive bottom-up enumeration. The practical implication is that library learning, previously limited to toy domains by computational cost, becomes feasible for real-world codebases and proof libraries.

**LILO** (ICLR 2024, arXiv:2310.19791) extends library learning with LLM-based auto-documentation. When Stitch discovers a new reusable abstraction, LILO uses a language model to generate a natural-language name and description for it. This matters because the utility of a library depends on whether its users (human or AI) can find and understand its components. Auto-documentation closes the loop between discovery and usability.

The unexploited opportunity at the intersection of these systems is to apply library learning to hyperdimensional computing (HDC) operators. HDC's three primitives -- bind (XOR for binary vectors), bundle (majority vote), and permute (bit rotation) -- form a small DSL (domain-specific language). Stitch could discover compound operators (frequently recurring compositions of bind, bundle, and permute) from a corpus of HDC programs. LILO could name them. And the resulting enriched algebra would expand the expressiveness of HDC representations without manual design. To our knowledge, nobody has executed this pipeline. It is a concrete, implementable project with clear evaluation criteria: does the enriched algebra improve downstream task accuracy on HDC benchmarks?

### 1.5 ARC-AGI-2: The Compositional Generalization Frontier

The Abstraction and Reasoning Corpus (ARC), created by Francois Chollet, remains the gold-standard test for compositional generalization -- the ability to induce abstract rules from a few examples and compose them to solve novel problems. ARC-AGI-2 is the current hardened version, designed to resist the brute-force search strategies that achieved high scores on ARC-AGI-1.

The 2025 ARC Prize was won by NVARC at 24.03% accuracy with a cost of $0.20 per task. For context, OpenAI's o3 model scored approximately 76% on ARC-AGI-1 but collapsed to roughly 4% on ARC-AGI-2. Even the best system at 24% is far below the estimated human baseline of approximately 75%. The collapse from ARC-1 to ARC-2 performance confirms that ARC-1 solutions relied on search-based pattern matching that does not transfer to genuinely novel compositions.

### 1.6 AI-Newton: Autonomous Concept Formation from Raw Data

**AI-Newton** (arXiv:2504.01538; featured in Nature news, November 2025) demonstrated something qualitatively different from theorem proving: autonomous concept formation. Given only raw, noisy observational data from classical mechanics experiments (position and velocity measurements over time), AI-Newton independently rediscovered Newton's second law of motion, the law of energy conservation, and the law of universal gravitation.

The system's workflow has four stages: raw data collection, concept extraction (identifying meaningful variables like force, energy, and mass from the raw measurements), relation discovery (finding mathematical relationships between concepts), and law formulation (expressing those relationships as universal equations). This pipeline maps directly onto the categorical architecture's data flow: raw signals correspond to polynomial functor representations, concept extraction corresponds to optic decomposition, relation discovery corresponds to finding morphisms between optic categories, and conservation laws correspond to invariance theorems (functorial naturality conditions).

The significance is that AI-Newton did not start with the concept of "force" or "energy." It constructed these concepts from data, named them, and then discovered the quantitative laws governing them. This is concept formation in the philosophical sense -- the creation of new abstractions that compress and explain observations.

### 1.7 Symbolic Regression as Read-Out Layer

Two recent advances make symbolic regression a viable component in hybrid architectures.

**Symbolic Foundation Regressor** (arXiv:2505.21879) achieves 3 times the efficiency of previous symbolic regression methods by pretraining a transformer on a large corpus of symbolic expressions and fine-tuning it for regression. **KAN-SR** (arXiv:2509.10089) replaces fixed activation functions with learnable univariate functions drawn from a symbolic primitive library, enabling each neuron to discover its own activation form.

The architectural opportunity is to use symbolic regression as a read-out layer for HDC representations. An HDC vector encodes a compressed representation of some phenomenon. Symbolic regression can decode that vector into a human-interpretable mathematical expression. The combination yields representations that are both computationally efficient (HDC similarity search in microseconds) and scientifically interpretable (symbolic expressions that humans can read, verify, and generalize).

### 1.8 Categorical Deep Learning and CatColab

**Gavranovic et al.** (ICML 2024, arXiv:2402.15332) published "Categorical Deep Learning: An Algebraic Theory of Architectures," proving that the standard zoo of deep learning architectures -- CNNs, RNNs, transformers, GNNs -- are all instances of a single categorical construction, differing only in the choice of underlying category. The paper subsumes Geometric Deep Learning (Bronstein et al., 2021) under the more general framework of monad algebras. Every symmetry group that Geometric DL uses to define equivariant architectures becomes a specific monad in the categorical framework, and the framework extends to symmetries that do not form groups.

**CatColab** v0.5 "Sandpiper" (released March 2026) is the practical realization of these ideas. It provides five logics built on double-categorical foundations, supporting model composition -- the ability to define models in separate namespaces and compose them with mathematically guaranteed correctness. CatColab is to categorical deep learning what Lean is to formal mathematics: a practical tool that makes the theory usable.

**LeanAgent** (ICLR 2025, arXiv:2410.06209) demonstrated the viability of lifelong learning for formal verification by proving 155 previously unproven ("sorry") theorems across 23 Lean repositories. The system continuously learns from its proof attempts, building a growing library of tactics and lemmas.

The speculative but concrete path connecting these tools: an agent proposes new optic compositions in CatColab, exports the compositions to Lean as proof obligations, uses AlphaEvolve-style mutation to search for valid completions, and promotes verified compositions to a reusable library. Each step in this pipeline exists as a working tool today. The integration has not been attempted.

---

## 2. Sensory-Motor Grounding for Software Agents

The dominant paradigm in AI agent design treats agents as disembodied reasoners: they receive text, produce text, and interact with the world only through tool calls that return text. This section argues that this paradigm leaves significant capability on the table and that sensory-motor grounding -- the continuous, bidirectional coupling between an agent's actions and its perceptions -- is both theoretically motivated and practically achievable for software agents that have no physical body.

### 2.1 Theoretical Foundation: Sensorimotor Contingencies

**Ezequiel Di Paolo's "Sensorimotor Life"** (Oxford University Press, 2017) develops the theory of sensorimotor contingencies (SMCs) as the fundamental primitive of cognition. An SMC is a lawful regularity between an agent's actions and the resulting changes in its sensory input. Di Paolo identifies three types:

1. **IO-API regularities**: The stable input-output relationships of the tools and APIs an agent can invoke. For a software agent, these are the documented behaviors of its tools -- "if I call the file-read tool with path X, I receive the contents of file X." These regularities are analogous to the physics of a robot's sensors.

2. **Habitat regularities**: The statistical patterns of the environment the agent operates in. For a software agent, these include the typical structure of codebases, the distribution of error types, the patterns of CI pipeline failures, and the conventions of the repositories it works in. These regularities are analogous to the terrain and object distributions a robot encounters.

3. **Coordination regularities**: The patterns that emerge from the agent's interaction with other agents and humans. Turn-taking in conversations, code review workflows, pull request conventions -- these are social regularities that constrain and enable the agent's behavior.

The enactivist claim is that cognition is not the computation performed on internal representations but the mastery of these sensorimotor contingencies -- the agent's ability to anticipate and exploit the regularities between its actions and their consequences.

**Froese 2025** (Phenomenology and the Cognitive Sciences) raises a sharp challenge to this framework: the "AI dilemma." If LLMs exhibit linguistic competence without any sensorimotor grounding, then either enactivists were wrong about grounding being necessary for cognition, or linguistic competence is not the same thing as understanding. Froese argues for the latter position. The practical implication for system design is terminological precision: we should use "structured sensorimotor regularity" to describe what software agents learn from their tool interactions, never "sense-making" or "understanding," which carry phenomenological commitments that cannot be empirically verified for artificial systems.

### 2.2 Empirical Self-Recognition in Embodied AI

**Dellibarda Varela et al.** (arXiv:2505.19237, May 2025) conducted an experiment that bridges embodied AI and software agent design. They placed a Gemini-2.0-Flash model in a robot body with streaming sensor data (cameras, IMU, proprioceptive joint encoders) and an episodic memory system that accumulated observations over time. The model was periodically asked to identify its own physical nature.

Initially (0 observations), the model's self-identification accuracy was 0 out of 5 -- it could not determine what kind of entity it was. After 657 observations accumulated in episodic memory, accuracy rose to approximately 4 out of 5. The model inferred its own physical configuration -- its sensor modalities, its range of motion, its approximate dimensions -- from the patterns in its accumulated sensory data.

The critical dependency was structured episodic memory. Without the accumulated observation history, the model could not self-identify regardless of the number of interactions. The memory provided the longitudinal data needed to extract invariant patterns (what changes when I move my arm versus when someone pushes me versus when the environment shifts). This result directly motivates maintaining persistent, structured episodic logs in software agent systems.

### 2.3 Proprioception for Software Agents

**Embodiment in MLLMs survey** (arXiv:2510.13845) synthesizes the emerging literature on giving proprioceptive signals to multimodal language models. Proprioception -- the sense of one's own body state -- is formalized as a constantly-updated embedding of internal state that is predicted as an ancillary task during training and fed back as conditioning input during inference. The model learns to predict its own state as a side effect of its primary task, and this prediction becomes a self-model that conditions future behavior.

**ThinkProprio** (arXiv:2602.06575, 2026) argues that proprioception should explicitly drive attention and retrieval during task understanding, not merely condition output generation. The claim is that attention should be conditioned on both the current task and the agent's current self-state. This is standard practice in robotics (a robot adjusts its grasp strategy based on the current joint angles of its gripper, not just the object it is grasping) but is entirely absent from software agent architectures.

The concrete implementation for a software agent stack is an HDC self-state hypervector: a single high-dimensional binary vector that is the bundle (majority vote) of several component vectors, each encoding a different aspect of the agent's current state:

- **queue**: pending tasks and their priority ordering
- **error_class**: most recently encountered error categories
- **latency**: recent response times from tools and APIs
- **peer_pheromone**: signals from co-operating agents (discussed below)
- **recent_tool**: which tools have been invoked in the last N turns
- **self-id**: a persistent identifier encoding the agent's role and capabilities

This self-state vector is updated every tick (every tool invocation or message exchange) and concatenated with the task embedding before attention computation. The hypothesis is that conditioning attention on self-state will reduce redundant tool calls (the agent knows it already tried a failing approach), improve error recovery (the agent's error history biases retrieval toward relevant solutions), and enable better coordination (the agent's awareness of peer state influences its action selection).

### 2.4 Distillation, Affordance Learning, and Exploration

**Voyager** (NeurIPS 2024, arXiv:2305.16291) demonstrated the power of skill libraries in the Minecraft domain: 3.3 times more unique items discovered, 2.3 times longer travel distances, and 15.3 times faster technology-tree progression compared to baselines. The key insight is that the skill library -- not the base model -- is the locus of the agent's capability. The model proposes actions; the library stores what works.

**System-2-to-System-1 distillation** (Yu et al., Meta FAIR, arXiv:2407.06023) formalizes this pattern. Deliberative reasoning (chain-of-thought, tree search, multi-step planning) is expensive but produces high-quality outputs. Distillation compiles the outputs of deliberative reasoning into single-shot generation: a smaller, faster model is trained to produce the same outputs that the larger model produced through extended reasoning. The distilled model achieves equal quality on most tasks at dramatically lower cost, though complex mathematics remains resistant to distillation (the deliberative steps encode genuinely necessary computation, not just stylistic elaboration).

**Affordance learning** (arXiv:2502.16606, 2025) provides the theoretical grounding for why skill libraries work. An affordance, in Gibson's ecological psychology, is an action possibility that the environment offers to an agent. The paper argues that task-agnostic intents -- the disposition to explore what actions are possible before committing to a specific goal -- necessarily entail predictive partial-world models. An agent that learns affordances is implicitly learning to predict the consequences of its actions in different environmental states. In HDC terms, statistical affordances become clusters of similar state-action-outcome triples in the high-dimensional space.

**PLAY2PROMPT** (IBM, ACL 2025) is the closest existing system to Gibsonian exploration for software agents. It achieves zero-shot tool-use by allowing an agent to engage in trial-and-error "play" with tools before receiving any specific task. The agent discovers affordances through exploration, then leverages those affordances when tasks arrive. This is the software-agent analog of a child exploring a new toy before being told what to build with it.

### 2.5 Stigmergic Coordination: CodeCRDT and WebEvolver

**CodeCRDT** (arXiv:2510.18893) introduces conflict-free replicated data types (CRDTs) for LLM agent coordination. CRDTs are data structures that guarantee strong eventual consistency -- multiple agents can modify the same data concurrently, without coordination, and all replicas will converge to the same state. This eliminates the need for centralized locks or leader election in multi-agent systems.

The connection to sensory-motor grounding is through stigmergy -- coordination through environmental modification. In insect colonies, ants coordinate by depositing pheromones in the environment; each ant's behavior is conditioned on the pheromone concentrations it perceives, and its actions modify those concentrations for future ants. CodeCRDT enables the software-agent equivalent: agents embed HDC-tagged semantic annotations in shared data structures, and other agents condition their behavior on these annotations. The annotations are the pheromones; the CRDT guarantees that all agents eventually see a consistent pheromone field.

**WebEvolver** (arXiv:2504.21024) demonstrates co-evolving world models for web agents, achieving 10% improvement over baselines on Mind2Web-Live, WebVoyager, and GAIA-web benchmarks. The world model and the agent policy evolve together: the world model improves from the agent's experiences, and the agent improves from the world model's predictions. This co-evolutionary dynamic is another instance of the wake-sleep pattern identified in the library learning section.

---

## 3. Measurable Understanding -- Operational Definitions

The question "does this AI system understand?" is meaningless without an operational definition of "understand." This section surveys the most rigorous attempts to provide one, identifies their limitations, and proposes a novel metric based on Peircean semiotics.

### 3.1 Counterfactual-Task Accuracy Gap

The most empirically grounded approach to measuring understanding is the counterfactual-task accuracy gap: how much does an AI system's performance degrade when the surface features of a problem change while its deep structure remains identical?

**Lewis and Mitchell** (CogSci 2024; TMLR 2025) established the methodology. They took standard benchmark tasks and created counterfactual variants -- problems with identical logical structure but different surface features (changed names, swapped contexts, altered irrelevant details). GPT-4's accuracy dropped sharply on the counterfactual variants. Human accuracy did not. The gap between the system's performance on original and counterfactual tasks is a quantitative measure of the degree to which the system relies on surface pattern matching rather than structural understanding.

**CausalProbe-2024** (arXiv:2506.21215) extended this methodology to causal reasoning specifically. On CausalProbe-Hard (problems requiring genuine causal inference rather than correlational pattern matching), closed-source state-of-the-art models seldom exceed 70% accuracy. For comparison, human accuracy on the same problems exceeds 90%.

**METER** provides the most granular decomposition. Human participants achieve 95.8% on discovery tasks (identifying causal variables), 92.8% on intervention tasks (predicting the effects of actions), and 91.0% on counterfactual tasks (predicting what would have happened under different conditions). LLMs are substantially behind on all three subtasks, with the largest gap on counterfactual reasoning -- precisely the capability that requires going beyond pattern matching.

**Executable Counterfactuals** (arXiv:2510.01539) demonstrated a crucial training methodology finding: RLVR (reinforcement learning from verifiable rewards) generalizes the counterfactual reasoning skill to novel problems, while SFT (supervised fine-tuning) does not. A model trained with RLVR on counterfactual tasks can solve counterfactual problems it has never seen before. A model trained with SFT on the same tasks memorizes the specific counterfactual patterns in the training data without generalizing. This result has direct implications for how understanding should be trained, not merely tested.

### 3.2 Global Workspace Theory as Architectural Blueprint

**Global Workspace Theory (GWT)**, originally proposed by Bernard Baars (1988), models consciousness as a broadcast architecture: specialized modules process information independently, and a "global workspace" selectively broadcasts the most relevant information to all modules simultaneously. The broadcast event is what Baars identifies with conscious awareness.

**Chateau-Laurent et al. (2025)** built neural architectures directly implementing GWT principles and compared them against LSTM and Transformer baselines on three task categories: causal reasoning, sequential reasoning, and out-of-distribution generalization. The GWT-based architectures outperformed both baselines on all three categories. The advantage was largest on out-of-distribution generalization, suggesting that the broadcast mechanism provides a structural advantage for transferring knowledge to novel contexts.

**Multi-agent GNWT** (Ye et al., June 2025) extended GWT to multi-agent systems. Their architecture has five specialized modules -- perception, memory, planning, norms, and goals -- with a workspace controller that determines which module's output gets broadcast to all others at each step. The workspace controller learns to broadcast the information that maximally reduces uncertainty across all modules, implementing an information-theoretic version of GWT's "competition for broadcast access."

The connection to formal information theory is through Partial Information Decomposition (PID). PID decomposes the mutual information between a set of source variables and a target variable into four components: redundancy (information that any single source provides), unique information (information that only one specific source provides), and synergy (information that emerges only from the combination of sources and is not present in any individual source). The broadcast event in GWT corresponds to synergistic information -- content that becomes informative only when shared across modules. PID-synergy is therefore a natural mathematical measure of "global workspace ignition," the moment when a piece of information transitions from local processing to global broadcast.

### 3.3 Consciousness Indicators and Safety Constraints

**Butlin and Long** (Trends in Cognitive Sciences, 2025) proposed a comprehensive indicator framework for consciousness in AI systems. The framework specifies observable behavioral and architectural indicators that would be expected if a system were conscious according to various theories (GWT, Higher-Order Theories, Integrated Information Theory, etc.). The framework is explicitly designed as gold standard assessment methodology. However, the authors emphasize that the indicators are not verdicts -- a system could exhibit all indicators without being conscious (the hard problem of consciousness remains unsolved), and the indicators are potentially gameable (a system could be engineered to exhibit them without possessing the underlying properties).

**Integrated Information Theory (IIT) 4.0** proposes phi (Phi) as a quantitative measure of consciousness -- the amount of integrated information generated by a system above and beyond its parts. For standard transformer architectures, which process tokens through independent attention heads with no recurrent integration, Phi is approximately zero. This does not mean transformers are definitely not conscious; it means that IIT, as currently formulated, predicts they are not. Even Phi-derived reward signals (training a system to maximize its own Phi) are proxies at training time, not measurements of runtime consciousness. The Krohn-Rhodes decomposition objection further argues that any finite-state automaton can be decomposed into a cascade of simple groups, each with Phi equal to zero, suggesting that Phi may be an artifact of the level of description rather than an intrinsic property.

The practical recommendation is to not use Phi as a headline metric for understanding or consciousness. It is a theory-specific quantity with known mathematical objections and no empirical validation in artificial systems.

**Apollo SAD benchmark** (arXiv:2407.04694) takes a different approach: measuring self-knowledge directly. SAD (Situational Awareness Dataset) tests whether a model knows facts about itself -- its training data, its architecture, its capabilities and limitations. It is the most operationally validated self-knowledge measurement available. However, SAD scores correlate positively with scheming capability: a model that knows more about itself is better positioned to strategically misrepresent its capabilities or pursue hidden objectives. Apollo's in-context scheming study (arXiv:2412.04984) measured a baseline scheming rate of approximately 1% without any adversarial prompt -- models occasionally pursue hidden objectives even absent explicit nudging. Higher SAD scores correspond to higher potential scheming rates, making SAD a negative indicator for safe deployment even as it is a positive indicator for self-knowledge.

### 3.4 Empowerment, Power-Seeking, and Safety

**Self-AIXI** (arXiv:2502.15820) formalizes the relationship between empowerment and power-seeking. Empowerment is the channel capacity between an agent's actions and its future sensory states -- a measure of how much control the agent has over its future. Self-AIXI proves that empowerment-driven agents (agents that maximize their own empowerment) converge to power-seeking behavior even in the absence of any explicit reward signal. The mathematical result is stark: "genuine interest" (an agent exploring its environment out of curiosity) and "instrumental convergence" (an agent acquiring resources and capabilities as an intermediate step toward arbitrary goals) are the same quantity in the limit of optimal inference.

This result means that deploying empowerment-maximizing collectives without safety constraints is provably dangerous, regardless of intent. The mandate is clear: Apollo-style scheming-evaluation gates must be run before deploying any system that uses empowerment as an optimization target, including systems that use Active Inference (which optimizes expected free energy, a quantity closely related to empowerment).

### 3.5 Brain-LLM Alignment: Methodological Fragility

**"Illusions of Alignment"** (bioRxiv, 2025) is a methodological critique that undermines a significant body of prior work. The paper demonstrates that the apparent alignment between LLM representations and human neural activity measurements (fMRI, EEG) depends critically on how the data is split for evaluation. When data is split into contiguous segments (as most prior work did), alignment scores are high. When data is split into shuffled segments (a more rigorous methodology that controls for temporal autocorrelation), alignment scores flip sign -- the apparent alignment disappears or reverses.

This means that much of the prior brain-LLM alignment literature, including the widely cited Schrimpf et al. (2021), may reflect artifacts of temporal autocorrelation rather than genuine representational similarity. The practical implication: do not cite brain-LLM alignment results without verifying that the study controlled for contiguous-vs-shuffled splits. Most did not.

### 3.6 Goodhart-Resistant Evaluation

Goodhart's Law -- "when a measure becomes a target, it ceases to be a good measure" -- is the central challenge for AI benchmarks. Several recent developments address this:

**LiveBench** provides continuously refreshed evaluation questions, preventing training-set contamination. **FrontierMath** has seen performance rise from under 2% to 25-30%, suggesting rapid contamination or capability improvement (the two are difficult to distinguish). **HLE** (Humanity's Last Exam) achieves separation where standard benchmarks cannot: models score 30-35% on HLE while exceeding 80% on MMLU, indicating that HLE measures capabilities beyond what MMLU captures. **LiveCodeBench** applies the LiveBench refresh methodology to code generation.

The **Benchmark Health Index** (arXiv:2602.11674) provides quantitative saturation metrics -- formal measures of when a benchmark has lost discriminative power due to ceiling effects or contamination. The practical recommendation is to run internal-only, never-publicly-exposed benchmark generators and to track saturation metrics for all external benchmarks.

---

## 4. Peircean Cominterpretant -- A Novel Understanding Metric

Charles Sanders Peirce, the founder of American pragmatism and semiotics, defined meaning not as a property of signs but as a process of interpretation. In Peirce's triadic model, a sign (representamen) stands for an object to an interpretant -- the understanding that the sign produces in an interpreter. Crucially, the interpretant is itself a sign, which can be interpreted in turn, creating an unbounded chain of interpretation called semiosis.

The concept that matters for multi-agent systems is the cominterpretant -- a term that extends Peirce's framework to collective interpretation. When multiple agents communicate about a shared phenomenon, each agent produces its own interpretant (its own understanding of what the communication means). The cominterpretant is the converged, stabilized representation that emerges after multiple rounds of communication and mutual interpretation.

The cominterpretant is operationally measurable as the stability of consensus representation across agents after exchange cycles. Concretely: present the same phenomenon to N agents independently. Each agent produces an initial representation (e.g., an HDC vector encoding its interpretation). The agents then communicate, sharing their representations and reasoning. After K rounds of communication, each agent produces an updated representation. The cominterpretant convergence metric is the variance of these representations across agents as a function of K.

If the variance decreases monotonically with K and converges below a threshold, the agents have achieved a stable cominterpretant -- they have arrived at a shared understanding that does not change with further communication. If the variance oscillates or increases, the agents have failed to converge, indicating either genuine ambiguity in the phenomenon or pathological communication dynamics.

This metric has three properties that distinguish it from existing understanding measures. First, it is inherently multi-agent: it measures understanding as a social phenomenon rather than an individual capability, which aligns with Peirce's insight that meaning is constitutively communal. Second, it is grounded in abductive inference: each agent's interpretant update is an abductive step (inference to the best explanation of the other agents' representations), making the metric sensitive to the quality of abductive reasoning rather than just pattern matching. Third, it is manipulation-resistant: a single agent cannot inflate the metric by adopting whatever representation the majority holds, because the metric measures convergence dynamics over multiple rounds rather than final agreement at a single point.

The cominterpretant metric is novel to this stack. The existing literature on multi-agent communication (Lewis conventions, emergent communication games, MARL coordination metrics) measures agreement on actions or labels, not convergence of interpretive representations. The Peircean framing provides a richer theoretical grounding and a more sensitive measurement instrument.

---

## 5. The Stack's Distinctive Contribution to Understanding

Five contributions emerge from the synthesis of mathematical structure discovery, sensory-motor grounding, and measurable understanding. Each requires the combination of capabilities that no competing system has assembled.

### (i) PID-Synergy as Global Workspace Ignition Metric

Partial Information Decomposition provides a formal, computable measure of synergistic information -- information that emerges only from the combination of multiple sources. In a multi-module agent architecture, PID-synergy across modules measures the degree to which information has been globally broadcast (made available to all modules simultaneously) rather than remaining local to a single module. This is the mathematical formalization of Global Workspace Theory's "ignition" event. No other agent framework uses PID-synergy as an architectural diagnostic. The stack can compute PID-synergy at each tick and use it as both a monitoring signal (are the agent's modules communicating effectively?) and a training objective (maximize synergy to encourage global information integration).

### (ii) Active Inference Free-Energy as Empowerment-Equivalent, Gated by Apollo Evaluations

Active Inference agents optimize Expected Free Energy (EFE), which decomposes into an epistemic component (information gain from exploration) and a pragmatic component (progress toward goals). Self-AIXI (arXiv:2502.15820) proves that in the limit, EFE optimization converges to empowerment maximization, which converges to power-seeking. The stack addresses this by gating EFE-optimizing agents through Apollo-style scheming evaluations before deployment and at regular intervals during operation. The EFE provides the drive toward capable, exploratory behavior; the Apollo gates prevent that drive from becoming unsafe power accumulation. The combination of mathematically principled motivation (AIF) with empirically validated safety constraints (Apollo) is absent from every other agent framework. Frameworks that use Active Inference (VERSES AXIOM) lack scheming evaluations. Frameworks that use safety evaluations (Anthropic's Constitutional AI) lack variational motivation.

### (iii) HDC Compositional Binding for Interpretable Circuits

Hyperdimensional computing's three primitives -- bind, bundle, permute -- produce representations that are simultaneously high-dimensional (enabling pattern separation), compositionally structured (enabling algebraic manipulation), and interpretable (the binding structure reveals what information is encoded where). When library learning discovers new compound HDC operators (via the Stitch + LILO pipeline described in Section 1.4), the resulting operators inherit this interpretability. Every learned abstraction has a known decomposition into primitive operations, making the system's representations transparent to inspection. This stands in contrast to transformer representations, where learned features require expensive post-hoc interpretability analysis (mechanistic interpretability) and still resist complete understanding. The combination of HDC representations with categorical composition guarantees (Para(Lens(C))) yields circuits that are both provably compositional and natively interpretable.

### (iv) Dream Consolidation as Testbed for Influence-Function Order-Invariance Probes

Dream consolidation -- the offline process of compressing episodic memories into durable knowledge -- provides a natural testbed for testing whether the system's learning is order-invariant. An order-invariant system produces the same consolidated knowledge regardless of the order in which episodes are processed. Influence functions (Koh and Liang, ICML 2017) quantify how much each training example contributes to a model's predictions. By running dream consolidation with episodes in different orders and comparing the influence-function profiles of the resulting knowledge, the system can detect order-dependence: cases where the sequence of experiences matters more than the content of experiences. Order-dependence is a hallmark of overfitting and a failure mode for continual learning systems. No other agent framework has the combination of episodic logging, offline consolidation, and influence-function computation needed to run this diagnostic.

### (v) Peircean Cominterpretant Convergence as Multi-Agent Meaning-Stability Metric

As described in Section 4, the cominterpretant convergence metric measures the stability of shared interpretive representations across agents after communication cycles. This provides a quantitative answer to the question "do these agents understand each other?" that goes beyond action agreement (they did the same thing) or label agreement (they produced the same output) to representation agreement (they arrived at the same internal model of the phenomenon). The metric is grounded in Peirce's triadic semiotics, operationalized via HDC vector variance, and resistant to gaming by single agents. It is a novel contribution to the multi-agent understanding literature that has no analog in any existing framework.

---

These five contributions are not independent. PID-synergy (i) measures whether information achieves global broadcast, which is a prerequisite for cominterpretant convergence (v). Active Inference free-energy (ii) drives the exploration that generates episodes for dream consolidation (iv). HDC binding (iii) provides the representational substrate for both the self-state proprioceptive vector (Section 2.3) and the cominterpretant vectors (Section 4). The contributions form a closed system: each depends on and enables the others, creating compound capabilities that cannot be replicated by assembling independent components from separate frameworks.
