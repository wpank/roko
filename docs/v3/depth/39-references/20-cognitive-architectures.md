# 39-20 Cognitive Architectures -- Annotated Reference Map

> Annotations are kept only for works whose descriptions were checked against the papers (listed in
> `tools/docs_integrity/content_audit.json` and in the banner of docs/v3/REFERENCES.md). The other entries are
> bare citations: their annotations were removed because they had not been checked against the papers.

> Research foundations for cognitive agent design, dual-process theory,
> classical cognitive architectures, and Global Workspace Theory as they
> apply to Roko's cognitive loop and tier routing.
>
> **v3 depth file** -- updated 2026-09-15. Added Mechanism-Level Review, Missing
> Knowledge Layer.

---

## Language Agent Architectures

**[Sumers et al., 2023]** *Cognitive Architectures for Language Agents (CoALA).* arXiv:2309.02427.
Modular memory, structured action spaces and a decision cycle of planning (proposal, evaluation, selection) then execution (§4). It defines no 9-step pipeline; that list is Roko's. Roko's universal loop extends CoALA with verification (Gate) and meta-cognition (Theta-frequency reflection).

**[Sumers et al., 2024]** *Cognitive Architectures for Language Agents.* Transactions on Machine Learning Research.
Extended CoALA treatment with updated cognitive architecture taxonomy. Provides the shared vocabulary for agent architecture comparisons.

---

## Dual-Process Theory

**[Kahneman, 2011]** *Thinking, Fast and Slow.* Farrar, Straus and Giroux.

**[Kahneman & Tversky, 1979]** *Prospect Theory: An Analysis of Decision Under Risk.* Econometrica, 47(2), 263--292.

**[Kahneman, 1973]** *Attention and Effort.* Prentice-Hall.

**[De Neys & Pennycook, 2019]** *Logic, Fast and Slow.* Current Directions in Psychological Science.

**[De Neys, 2018]** *Dual Process Theory 2.0.* Routledge.

---

## Classical Cognitive Architectures

**[Anderson, 1993]** *Rules of the Mind.* Lawrence Erlbaum Associates.

**[Anderson, 2007]** *How Can the Human Mind Occur in the Physical Universe?* Oxford University Press.

**[Laird, Newell & Rosenbloom, 1987]** *SOAR: An Architecture for General Intelligence.* Artificial Intelligence, 33(1), 1--64.

**[Laird, 2012]** *The Soar Cognitive Architecture.* MIT Press.

**[Sun, 2002]** *Duality of the Mind.* Lawrence Erlbaum Associates.

---

## Global Workspace Theory

**[Baars, 1988]** *A Cognitive Theory of Consciousness.* Cambridge University Press.

**[Franklin et al., 2014]** *LIDA: A Systems-level Architecture for Cognition, Emotion, and Learning.* IEEE Trans. Autonomous Mental Development, 6(1).

**[Maytié et al., 2025]** *Multimodal Dreaming: A Global Workspace Approach to World Model-Based Reinforcement Learning.* arXiv:2502.21142.

**[Nakanishi et al., 2025]** *Hypothesis on the Functional Advantages of the Selection-Broadcast Cycle Structure: Global Workspace Theory and Dealing with a Real-Time World.* arXiv:2505.13969.

---

## 2025--2026 Additions: Mechanism-Level Review, Missing Knowledge Layer

**[Roynard, 2025]** *The Missing Knowledge Layer in Cognitive Architectures for AI Agents.* arXiv:2604.11364.

---

## Cognitive Workspace and Episodic Memory

**[An, 2025]** *Cognitive Workspace: Active Memory Management for LLMs.* arXiv:2508.13171.
Context window as cognitive workspace with explicit read/write/evict operations. Mirrors NeuroStore's approach to context as a managed resource, not a passive buffer.

**[Fountas et al., 2025]** *EM-LLM: Human-Inspired Episodic Memory for Infinite Context LLMs.* ICLR 2025. arXiv:2407.09450.

---

## Agentic AI Surveys

**[Abou Ali & Dornaika, 2025]** *Agentic AI: A Comprehensive Survey.* Artificial Intelligence Review, Springer.

**[Wu et al., 2025]** *Cognitive LLMs: Toward Human-Like Artificial Intelligence by Integrating Cognitive Architectures and Large Language Models for Manufacturing Decision-Making.* Neurosymbolic Artificial Intelligence (SAGE Publications).

**[Arunkumar V et al., 2025]** *Agentic Artificial Intelligence (AI): Architectures, Taxonomies, and Evaluation of Large Language Model Agents.* arXiv:2601.12560.
Post-2022 paradigm shift from symbolic to neural orchestration. Validates neural-first design with structured cognitive overlays.

---

## Cross-References

- Dual-process in tier routing: depth/04-execution/
- Active inference: [16-active-inference](./16-active-inference.md)
- Cybernetics: [15-cybernetics-and-vsm](./15-cybernetics-and-vsm.md)
