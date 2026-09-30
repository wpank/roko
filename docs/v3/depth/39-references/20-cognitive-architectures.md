# 39-20 Cognitive Architectures -- Annotated Reference Map

> Research foundations for cognitive agent design, dual-process theory,
> classical cognitive architectures, and Global Workspace Theory as they
> apply to Roko's cognitive loop and tier routing.
>
> **v3 depth file** -- updated 2026-09-15. Added Mechanism-Level Review, Missing
> Knowledge Layer.

---

## Language Agent Architectures

**[Sumers et al., 2023]** *Cognitive Architectures for Language Agents (CoALA).* arXiv:2309.02427.
9-step cognitive pipeline: perceive, retrieve, reason, act, learn. Roko's universal loop extends CoALA with verification (Gate) and meta-cognition (Theta-frequency reflection).

**[Sumers et al., 2024]** *Cognitive Architectures for Language Agents.* Transactions on Machine Learning Research.
Extended CoALA treatment with updated cognitive architecture taxonomy. Provides the shared vocabulary for agent architecture comparisons.

---

## Dual-Process Theory

**[Kahneman, 2011]** *Thinking, Fast and Slow.* Farrar, Straus and Giroux.
System 1 (fast, automatic) / System 2 (slow, deliberate). Roko implements as T0 (~80% of ticks, reflex), T1 (~15%, heuristic), T2 (~5%, full inference). The cascade router manages tier selection.

**[Kahneman & Tversky, 1979]** *Prospect Theory: An Analysis of Decision Under Risk.* Econometrica, 47(2), 263--292.
Asymmetric loss/gain evaluation. Informs the Daimon's asymmetric outcome response: losses weigh more heavily than equivalent gains in affect update.

**[Kahneman, 1973]** *Attention and Effort.* Prentice-Hall.
Attention as scarce computational resource. Grounds the VCG attention auction: context sections compete for limited attention budget.

**[De Neys & Pennycook, 2019]** *Logic, Fast and Slow.* Current Directions in Psychological Science.
Updated dual-process theory: System 1 and System 2 are not fully separate but interact. Informs T0/T1/T2 routing nuances -- T0 can escalate mid-processing.

**[De Neys, 2018]** *Dual Process Theory 2.0.* Routledge.
Comprehensive update to dual-process theory addressing the interaction between intuitive and deliberate processes.

---

## Classical Cognitive Architectures

**[Anderson, 1993]** *Rules of the Mind.* Lawrence Erlbaum Associates.
ACT-R: Adaptive Control of Thought-Rational. NeuroStore knowledge types (episodes, insights, heuristics, warnings) map to ACT-R declarative memory types. Retrieval is activation-based with recency and frequency.

**[Anderson, 2007]** *How Can the Human Mind Occur in the Physical Universe?* Oxford University Press.
Updated ACT-R theory placing cognitive architecture within physical universe constraints. Validates resource-bounded cognition design.

**[Laird, Newell & Rosenbloom, 1987]** *SOAR: An Architecture for General Intelligence.* Artificial Intelligence, 33(1), 1--64.
SOAR cycle: propose, decide, apply, learn. Maps to Roko's compose, act, verify, adapt. SOAR's chunking mechanism maps to knowledge tier promotion.

**[Laird, 2012]** *The Soar Cognitive Architecture.* MIT Press.
Comprehensive SOAR treatment. Provides detailed reference for the propose-decide-apply-learn cycle.

**[Sun, 2002]** *Duality of the Mind.* Lawrence Erlbaum Associates.
CLARION: dual-level architecture combining explicit and implicit knowledge. Validates the NeuroStore (explicit) + HDC (implicit) combination -- two representations serving different retrieval needs.

---

## Global Workspace Theory

**[Baars, 1988]** *A Cognitive Theory of Consciousness.* Cambridge University Press.
Global Workspace Theory: information becomes "conscious" when broadcast to multiple specialized processors. Informs broadcast-style information sharing in the Bus/Event system.

**[Franklin et al., 2014]** *LIDA: A Systems-level Architecture for Cognition, Emotion, and Learning.* IEEE Trans. Autonomous Mental Development, 6(1).
LIDA cognitive architecture integrating cognition, emotion, and learning in a Global Workspace framework. Validates the Daimon + NeuroStore + Learning integration.

**[Maytié et al., 2025]** *Multimodal Dreaming: A Global Workspace Approach to World Model-Based Reinforcement Learning.* arXiv:2502.21142.
Global Workspace Theory combined with world models (DreamerV3). Bridges GWT with offline learning. Validates the architectural intersection of Dreams and broadcast coordination.

**[Nakanishi et al., 2025]** *Hypothesis on the Functional Advantages of the Selection-Broadcast Cycle Structure: Global Workspace Theory and Dealing with a Real-Time World.* arXiv:2505.13969.
Functional advantages of the GWT-style selection-broadcast cycle. Validates broadcast mechanisms in multi-agent coordination.

---

## 2025--2026 Additions: Mechanism-Level Review, Missing Knowledge Layer

**[Roynard, 2025]** *The Missing Knowledge Layer in Cognitive Architectures for AI Agents.* arXiv:2604.11364.
Identifies a persistent gap in agent architectures: lack of a dedicated knowledge management layer between working memory (context window) and long-term storage (embeddings/databases). Validates NeuroStore as the architectural response to this gap -- a structured, tiered, actively managed knowledge layer.

---

## Cognitive Workspace and Episodic Memory

**[An, 2025]** *Cognitive Workspace: Active Memory Management for LLMs.* arXiv:2508.13171.
Context window as cognitive workspace with explicit read/write/evict operations. Mirrors NeuroStore's approach to context as a managed resource, not a passive buffer.

**[Fountas et al., 2025]** *EM-LLM: Human-Inspired Episodic Memory for Infinite Context LLMs.* ICLR 2025. arXiv:2407.09450.
Episodic memory enabling infinite effective context through retrieval. Provides practical techniques for the NeuroStore's episodic retrieval.

---

## Agentic AI Surveys

**[Anonymous, 2025]** *Agentic AI: A Comprehensive Survey.* Artificial Intelligence Review, Springer.
Six-module taxonomy validates Roko's modular architecture. Identifies paradigm shift from symbolic to neural orchestration post-2022.

**[Wu et al., 2025]** *Cognitive LLMs: Toward Human-Like Artificial Intelligence by Integrating Cognitive Architectures and Large Language Models for Manufacturing Decision-Making.* Neurosymbolic Artificial Intelligence (SAGE Publications).
ACT-R/SOAR + LLM integration validates layering cognitive principles onto LLMs.

**[Arunkumar V et al., 2025]** *Agentic Artificial Intelligence (AI): Architectures, Taxonomies, and Evaluation of Large Language Model Agents.* arXiv:2601.12560.
Post-2022 paradigm shift from symbolic to neural orchestration. Validates neural-first design with structured cognitive overlays.

---

## Cross-References

- Dual-process in tier routing: depth/04-execution/
- Active inference: [16-active-inference](./16-active-inference.md)
- Cybernetics: [15-cybernetics-and-vsm](./15-cybernetics-and-vsm.md)
