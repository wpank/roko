# Cognitive Architectures

> Annotations are kept only for works whose descriptions were checked against the papers (listed in
> `tools/docs_integrity/content_audit.json` and in the banner of docs/v3/REFERENCES.md). The other entries are
> bare citations: their annotations were removed because they had not been checked against the papers.

> **DEPRECATED (v1):** This document is part of the v1 specification and may be outdated. See [../../v2/](../../v2/) for the current reference.


> Academic foundations for cognitive agent architectures, dual-process theory, and computational cognitive science that inform Roko's Synapse Architecture.

**Topic**: [References](./INDEX.md)
**Prerequisites**: [Architecture](../00-architecture/INDEX.md)
**Key sources**: `bardo-backup/prd/shared/citations.md` §3, `bardo-backup/tmp/agent-chain/08-references.md`

> **Implementation**: Reference

---

## Abstract

Roko's universal cognitive loop is not an ad-hoc design but an implementation of established cognitive architecture principles. CoALA (Sumers et al. 2023) provides the 9-step pipeline. Kahneman's System 1/System 2 grounds the dual-process T0/T1/T2 cascade. CLARION's dual-level architecture validates the combination of explicit (declarative) and implicit (procedural) knowledge. ACT-R and SOAR provide decades of validated cognitive architecture design that inform Roko's approach.

---

## CoALA: Cognitive Architectures for Language Agents

- Sumers, T.R., Yao, S., Narasimhan, K., & Griffiths, T.L. (2023). Cognitive Architectures for Language Agents (CoALA). arXiv:2309.02427.
  *Grounds: Decision cycle — CoALA defines modular memory, action spaces and a decision cycle of planning (proposal, evaluation, selection) then execution (§4); the 9-step pipeline is Roko's own. Roko's universal loop (PERCEIVE → EVALUATE → ATTEND → INTEGRATE → ACT → VERIFY → PERSIST → ADAPT → META-COGNIZE) extends CoALA with explicit verification (Gate) and meta-cognition (Daimon).*

- Sumers, T.R. et al. (2024). Cognitive Architectures for Language Agents. _Transactions on Machine Learning Research_, 2024.

---

## Dual-Process Theory

- Kahneman, D. (2011). _Thinking, Fast and Slow_. Farrar, Straus and Giroux.

- Kahneman, D. & Tversky, A. (1979). Prospect Theory: An Analysis of Decision Under Risk. _Econometrica_, 47(2), 263-292.

- Kahneman, D. (1973). _Attention and Effort_. Prentice-Hall.

---

## Classical Cognitive Architectures

- Anderson, J.R. (1993). _Rules of the Mind_. Lawrence Erlbaum Associates.

- Laird, J.E., Newell, A., & Rosenbloom, P.S. (1987). SOAR: An Architecture for General Intelligence. _Artificial Intelligence_, 33(1), 1-64.

- Sun, R. (2002). Duality of the Mind: A Bottom-Up Approach Toward Cognition. Lawrence Erlbaum Associates.

---

## Cognitive Load and Rational Inattention

- Sims, C.A. (2003). Implications of Rational Inattention. _Journal of Monetary Economics_, 50(3), 665-690.

---

## Cognitive Workspace

- An (2025). Active Memory Management for LLMs. arXiv:2508.13171.
  *Grounds: Active memory — active memory management with deliberate information curation, hierarchical cognitive buffers and task-driven context optimization; 58.6% memory reuse versus 0% for RAG (abstract, §6). Informs the Composer's context management.*

---

## Episodic Memory for LLMs

- Fountas, Z. et al. (2025). EM-LLM: Human-Inspired Episodic Memory for Infinite Context LLMs. _ICLR_, 2025.

---

## Complementary Learning Systems

- McClelland, J.L., McNaughton, B.L., & O'Reilly, R.C. (1995). Why There Are Complementary Learning Systems in the Hippocampus and Neocortex. _Psychological Review_, 102(3), 419-457.

---

## Agentic AI Architecture Surveys (2025)

- Agentic AI: A Comprehensive Survey of Architectures, Applications, and Future Directions (2025). _Artificial Intelligence Review_, Springer, 2025.

- Wu, S. et al. (2025). Cognitive LLMs: Toward Human-Like Artificial Intelligence by Integrating Cognitive Architectures and Large Language Models. _SAGE Journals_, 2025.

- Agentic Artificial Intelligence (AI): Architectures, Taxonomies, and Evaluation of Large Language Model Agents of LLM Agents (2025). arXiv:2601.12560.
  *Grounds: Agent taxonomy — a unified taxonomy of agents into Perception, Brain, Planning, Action, Tool Use and Collaboration (abstract, §3).*

---

## Cross-References

- See [01-memory-consolidation.md](./01-memory-consolidation.md) for CLS and memory systems
- See [02-affective-computing.md](./02-affective-computing.md) for PAD and somatic markers
- See [15-cybernetics-and-vsm.md](./15-cybernetics-and-vsm.md) for cybernetic regulation
- See [16-active-inference.md](./16-active-inference.md) for active inference
