# 2025 Additions — Cross-Domain Research Frontiers

> Annotations are kept only for works whose descriptions were checked against the papers (listed in
> `tools/docs_integrity/content_audit.json` and in the banner of docs/v3/REFERENCES.md). The other entries are
> bare citations: their annotations were removed because they had not been checked against the papers.

> **DEPRECATED (v1):** This document is part of the v1 specification and may be outdated. See [../../v2/](../../v2/) for the current reference.


> Cutting-edge 2024-2025 research across cognitive architecture agents, multi-agent coordination, HDC applications, active inference, affect computing, knowledge consolidation, formal verification for AI, TDA for time series, mechanism design for AI economies, and stigmergy. Each citation links to the Roko subsystem it validates.

**Topic**: [References](./INDEX.md)
**Prerequisites**: All domain sub-docs
**Date**: April 2025 survey

> **Implementation**: Reference

---

## Abstract

This document catalogs ~60 papers from the 2024-2025 research frontier that strengthen, validate, or extend Roko's architectural foundations. These papers were discovered through systematic web search across ten research domains. Each paper is annotated with its relevance to specific Roko subsystems. Papers that are also added to their primary domain sub-doc are marked with cross-references. The research landscape shows clear convergence: sleep-inspired compute, active forgetting, emergent coordination in LLM collectives, and formal verification of AI systems are all moving from theoretical curiosities to engineering necessities — exactly the trajectory Roko's architecture anticipated.

---

## 1. Cognitive Architecture for LLM Agents

### Surveys and Taxonomies

- Agentic AI: A Comprehensive Survey (2025). _Artificial Intelligence Review_, Springer.

- Wu, S. et al. (2025). Cognitive LLMs: Toward Human-Like Artificial Intelligence by Integrating Cognitive Architectures and Large Language Models for Manufacturing Decision-Making for Manufacturing Decision-Making. _Neurosymbolic Artificial Intelligence (SAGE Publications)_.

- Agentic Artificial Intelligence (AI): Architectures, Taxonomies, and Evaluation of Large Language Model Agents (2025). arXiv:2601.12560.
  *Grounds: Agent taxonomy — a unified taxonomy of agents into Perception, Brain, Planning, Action, Tool Use and Collaboration (abstract, §3); the 21,730-rollout result is HAL's (Kapoor et al.), not this paper's. See [20-cognitive-architectures.md](./20-cognitive-architectures.md).*

### Cognitive Workspace

- An (2025). Active Memory Management for LLMs. arXiv:2508.13171.
  *Grounds: Context-as-workspace — active memory management with deliberate information curation and hierarchical cognitive buffers (abstract). Mirrors NeuroStore's approach to context as a managed resource, not a passive buffer. See [20-cognitive-architectures.md](./20-cognitive-architectures.md).*

- Position: Episodic Memory is the Missing Piece for Long-Term LLM Agents (2025). arXiv:2502.06975.

---

## 2. Multi-Agent Coordination and Emergent Collectives

### Emergence and Coordination

- Emergent Coordination in Multi-Agent Language Models (2025). arXiv:2510.05174.
  *Grounds: Dynamical emergence — information-theoretic framework measuring higher-order structure in multi-agent LLM systems. Identity-linked differentiation + theory-of-mind instructions produce collective intelligence. See [04-coordination-and-multi-agent.md](./04-coordination-and-multi-agent.md) and [18-collective-intelligence.md](./18-collective-intelligence.md).*

- Emergent Convergence in Multi-Agent LLM Annotation (2025). arXiv:2512.00047.

- Multi-Agent Language Models: Advancing Cooperation, Coordination, and Adaptation (2025). arXiv:2506.09331.

- Large Language Models Miss the Multi-Agent Mark (2025). arXiv:2505.21298.

- AgentsNet: Coordination and Collaborative Reasoning in Multi-Agent LLMs (2025). arXiv:2507.08616.
  *Grounds: Distributed coordination benchmark — probes up to 100 agents with problems from distributed computing theory. Provides evaluation methodology for Roko's Collective coordination at scale.*

- Multi-Agent Collaboration Mechanisms: A Survey of LLMs (2025). arXiv:2501.06322.

- LLM-Coordination: Evaluating Multi-agent Coordination Abilities (2025). _NAACL_, 2025.

- Agentic LLMs in the supply chain: towards autonomous multi-agent consensus-seeking (2025). _International Journal of Production Research_.

### Cooperation Game Theory

- Fontana, M. et al. (2024). Nicer Than Humans: How Do LLMs Behave in the Prisoner's Dilemma? arXiv:2406.13605.

---

## 3. Stigmergy and Swarm Intelligence

- Stigmergy: From Mathematical Modelling to Control (2024). _Proceedings of the Royal Society A_.

- Automatic Design of Stigmergy-Based Behaviours for Robot Swarms (2024). _Communications Engineering_, Nature.

- Stigmergy Facilitates Emergent Patterns in Academic Communication (2025). Research Square.

- Enhancing radioactive environment exploration with bio-inspired swarm robotics: A comparative analysis of Levy flight and stigmergy methods (2024). _Robotics and Autonomous Systems_.

---

## 4. HDC and Vector Symbolic Architecture Applications

- Heddes et al. (2024). HDC: A Framework for Stochastic Computation and Symbolic AI. _Journal of Big Data_.

- Hyperdimensional computing with holographic and adaptive encoder (2024). _Frontiers in AI_.

- HPVM-HDC: A Heterogeneous Programming System for Accelerating Hyperdimensional Computing (2024). arXiv:2410.15179.

- Hyperdimensional Computing in Biomedical Sciences (2025). PMC review.

- Optimal Hyperdimensional Representation for Learning and Cognitive Computation (2025). OpenReview.

- The Hyperdimensional Transform for Distributional Modeling (2025). _Neural Computing and Applications_.

---

## 5. Active Inference and Free Energy Principle

- Shafiei, A. et al. (2025). Distributionally Robust Free Energy Principle for Decision-Making. _Nature Communications_, 17, 707.

- Prakki (2024). Active Inference for Self-Organizing Multi-LLM Systems. arXiv:2412.10425.

- Synthetic Active Inference Agents, Part II (2024). arXiv:2306.02733.

- Free Energy Principle and Active Inference in Neural Language Models (2024). CEUR-WS Vol-3923.

---

## 6. Affective Computing and Emotion in Agents


- Intelligent Agents with Emotional Intelligence (2025). arXiv:2511.20657.

- Emotions in the Loop (2025). arXiv:2505.01542.

- CosmoCore: Affective Dream-Replay RL for Code Generation (2025). arXiv:2510.18895.

- Affective Computing and Emotional Data: Challenges in Privacy Regulations (2025). arXiv:2509.20153.

---

## 7. Knowledge Consolidation, Memory, and Forgetting

### Agent Memory Surveys

- Hu et al. (2025). Memory in the Age of AI Agents. arXiv:2512.13564.
  *Grounds: Memory taxonomy — factual, experiential, working memory distinguished. See [01-memory-consolidation.md](./01-memory-consolidation.md).*

- Du et al. (2025). Rethinking Memory in LLM-based Agents. arXiv:2505.00675.

- Memory for Autonomous LLM Agents (2026). arXiv:2603.07670.
  *Grounds: Write-manage-read formalization — five mechanism families for agent memory. See [01-memory-consolidation.md](./01-memory-consolidation.md).*

### Sleep-Inspired Memory

- Language Models Need Sleep (2025). OpenReview.

- NeuroDream (2025). SSRN:5377250.

- Fang et al. (2025). arXiv:2510.18866.
  *Grounds: Offline consolidation — up to 10.9% accuracy gain and up to 117x fewer tokens (v1 abstract). See [03-dreams-and-offline-learning.md](./03-dreams-and-offline-learning.md).*

- Xie (2025). arXiv:2603.14517.

### Human-Inspired Memory

- Honda et al. (2025). Human-Like Remembering and Forgetting in LLM Agents: An ACT-R-Inspired Memory Architecture. _HAI_, 2025.

- Enhancing Memory Retrieval in Generative Agents (2025). _Frontiers in Psychology_.

- Continuum Memory Architectures for Long-Horizon Agents (2026). arXiv:2601.09913.

---

## 8. Formal Verification for AI Safety


- Towards Guaranteed Safe AI (2024). arXiv:2405.06624.

- Model Checking Deep Neural Networks (2025). _Frontiers in Computer Science_.

- VNN-COMP 2024. International Verification Competition for Neural Networks.

- Formal Methods in Robot Policy Learning and Verification (2025). OpenReview.

---

## 9. TDA for Time Series and Anomaly Detection

- Topological data analysis and topological deep learning beyond persistent homology: a review (2025). _Artificial Intelligence Review_, Springer.

- Persistent Homology-Based Unsupervised Anomaly Detection (2025). OpenReview.

- Multivariate Time-Series Anomaly Detection based on Enhancing Graph Attention Networks with Topological Analysis (2024). arXiv:2408.13082.

- Change Point Detection in Financial Market Using Topological Data Analysis (2025). _Systems_, 13(10).

- Machine Learning of Time Series Using Persistent Homology (2025). _Scientific Reports_, Nature.

---

## 10. Mechanism Design for AI Economies

- Yang et al. (2025). arXiv:2507.03904.

- Duetting, P. et al. (2024). Mechanism Design for Large Language Models. _ACM WWW Best Paper_.

- Deep Mechanism Design (2024). _PNAS_.

- Automated Mechanism Design Survey (2024). _ACM SIGecom Exchanges_, 22(2).

---

## 11. Self-Learning and Reflection

- Ge et al., SAMULE: Self-Learning Agents Enhanced by Multi-level Reflection (2025). _EMNLP_, 2025. arXiv:2509.20562.

- MAR: Multi-Agent Reflexion (2025). arXiv:2512.20845.

- Self-Evolving LLMs via Continual Instruction Tuning (2025). arXiv:2509.18133.

- The Future of Continual Learning in the Era of Foundation Models (2025). arXiv:2506.03320.
  *Grounds: Continual learning — position paper arguing that continual learning remains essential for foundation models, in three directions: continual pre-training, continual fine-tuning, and continual compositionality and orchestration (§4). Validates NeuroStore's non-parametric approach to continual knowledge management.*

---

## Cross-cutting Themes

### Theme 1: Sleep and Offline Compute are Engineering Necessities

Five independent 2025 papers (Language Models Need Sleep, NeuroDream, LightMem, SleepGate, CosmoCore) demonstrate that offline consolidation — the Dreams subsystem — produces measurable improvements: 38% less forgetting, 17.6% better transfer, 10.9% accuracy gains, 117x token savings. This is no longer speculative neuroscience inspiration; it is empirical ML engineering.

### Theme 2: Active Forgetting Outperforms Passive Accumulation

Memory surveys (Liu 2025, Wang 2025) and SleepGate (2025) converge on the conclusion that forgetting is not a bug but a feature. Naive add-all memory degrades performance. The NeuroStore's Curator cycle — active pruning, confidence decay, tier-based retention — is validated by the latest agent memory research.

### Theme 3: Explicit Coordination Beats Implicit LLM Cooperation

Multiple 2025 papers (Large LLMs Miss the Multi-Agent Mark, AgentsNet, LLM-Coordination) show that LLMs fail at implicit coordination. Emergent coordination requires explicit mechanisms: pheromone fields, role prompting, theory-of-mind instructions. Roko's Pheromone Field and Agent Mesh are the right architectural response.

### Theme 4: Formal Verification is Moving from Theory to Practice

The formal verification community (VNN-COMP, Formal Methods for Safe AI) is producing practical tools for verifying AI system behavior. This trajectory aligns with Roko's Gate pipeline: structural verification (compiler, tests, lints) rather than LLM self-assessment.

### Theme 5: Active Inference Works for LLM Systems

DR-FREE (Shafiei 2025, Nature Communications) and Active Inference for Self-Organizing Multi-LLM Systems: A Bayesian Thermodynamic Approach to Adaptation (Koudahl 2024) demonstrate that active inference is not just a theoretical framework but a practical cognitive layer for LLM agents. Roko's EFE-based tier routing and context selection are validated by production-quality research.

---

## Cross-References

- See [01-memory-consolidation.md](./01-memory-consolidation.md) for agent memory surveys
- See [02-affective-computing.md](./02-affective-computing.md) for emotion-in-loop research
- See [03-dreams-and-offline-learning.md](./03-dreams-and-offline-learning.md) for sleep-inspired LLM architectures
- See [04-coordination-and-multi-agent.md](./04-coordination-and-multi-agent.md) for emergent coordination and stigmergy PDE
- See [05-biological-analogues.md](./05-biological-analogues.md) for swarm robotics
- See [06-self-learning-systems.md](./06-self-learning-systems.md) for multi-level reflection
- See [08-security-and-provenance.md](./08-security-and-provenance.md) for formal verification
- See [09-hdc-vsa.md](./09-hdc-vsa.md) for HDC frameworks
- See [12-signal-processing.md](./12-signal-processing.md) for TDA advances
- See [16-active-inference.md](./16-active-inference.md) for DR-FREE and multi-LLM active inference
- See [18-collective-intelligence.md](./18-collective-intelligence.md) for emergent LLM collectives
- See [20-cognitive-architectures.md](./20-cognitive-architectures.md) for agentic AI surveys
- See [21-mechanism-design.md](./21-mechanism-design.md) for agent marketplaces
