# Memory Consolidation

> Annotations are kept only for works whose descriptions were checked against the papers (listed in
> `tools/docs_integrity/content_audit.json` and in the banner of docs/v3/REFERENCES.md). The other entries are
> bare citations: their annotations were removed because they had not been checked against the papers.

> **DEPRECATED (v1):** This document is part of the v1 specification and may be outdated. See [../../v2/](../../v2/) for the current reference.


> Academic foundations for memory systems, forgetting as optimization, and knowledge tier progression in the Roko framework.

**Topic**: [References](./INDEX.md)
**Prerequisites**: [Architecture](../00-architecture/INDEX.md), [Neuro](../06-neuro/INDEX.md)
**Key sources**: `bardo-backup/prd/04-memory/10-research.md`, `bardo-backup/prd/02-mortality/14-research-foundations.md`

> **Implementation**: Reference

---

## Abstract

Memory consolidation research provides the theoretical foundation for Roko's NeuroStore — the knowledge management subsystem that persists insights, heuristics, and warnings across tasks. The central finding across neuroscience, cognitive psychology, and AI memory systems is that **forgetting is not failure; it is regularization**. Active forgetting prevents overfitting to stale information, and selective consolidation (prioritized replay, spaced retrieval, CLS dual-store) produces more robust knowledge than naive accumulation. Every design decision in the NeuroStore traces to published research indexed here.

---

## Complementary Learning Systems

- McClelland, J.L., McNaughton, B.L., & O'Reilly, R.C. (1995). Why There Are Complementary Learning Systems in the Hippocampus and Neocortex: Insights from the Successes and Failures of Connectionist Models of Learning and Memory. _Psychological Review_, 102(3), 419-457.

- Kumaran, D., Hassabis, D., & McClelland, J.L. (2016). What Learning Systems do Intelligent Agents Need? Complementary Learning Systems Theory Updated. _Trends in Cognitive Sciences_, 20(7), 512-534.

- O'Reilly et al. (2014). Complementary Learning Systems. _Cognitive Science_, 38(Suppl 1).

- McCloskey, M. & Cohen, N.J. (1989). Catastrophic Interference in Connectionist Networks: The Sequential Learning Problem. _Psychology of Learning and Motivation_, 24, 109-165.

---

## Forgetting as Optimization

- Richards, B.A. & Frankland, P.W. (2017). The Persistence and Transience of Memory. _Neuron_, 94(6), 1071-1084.

- Ebbinghaus, H. (1885). _Über das Gedächtnis_ (Memory: A Contribution to Experimental Psychology). Leipzig.

- Davis, R.L. & Zhong, Y. (2017). The Biology of Forgetting — A Perspective. _Neuron_, 95(3), 490-503.

- Hardt, O., Nader, K., & Nadel, L. (2013). Decay Happens: The Role of Active Forgetting in Memory. _Trends in Cognitive Sciences_, 17(3), 111-120.

- Nader, K. et al. (2000). Fear Memories Require Protein Synthesis in the Amygdala for Reconsolidation after Retrieval. _Nature_, 406, 722-726.

---

## Retrieval and Spacing Effects

- Roediger, H.L. & Karpicke, J.D. (2006). Test-Enhanced Learning: Taking Memory Tests Improves Long-Term Retention. _Psychological Science_, 17(3), 249-255.

- Cepeda, N.J. et al. (2006). Distributed Practice in Verbal Recall Tasks: A Review and Quantitative Synthesis. _Psychological Bulletin_, 132(3), 354-380.

---

## Prioritized Replay

- Wilson, M.A. & McNaughton, B.L. (1994). Reactivation of Hippocampal Ensemble Memories During Sleep. _Science_, 265(5172), 676-679.

- Mattar, M.G. & Daw, N.D. (2018). Prioritized Memory Access Explains Planning and Hippocampal Replay. _Nature Neuroscience_, 21(11), 1609-1617.

- Schaul, T. et al. (2016). Prioritized Experience Replay. _ICLR_, 2016. arXiv:1511.05952.

---

## Sleep-Dependent Consolidation

- Stickgold, R. (2005). Sleep-Dependent Memory Consolidation. _Nature_, 437, 1272-1278.

- Born, J. & Wilhelm, I. (2012). System Consolidation of Memory During Sleep. _Psychological Research_, 76, 192-203.

- Tononi, G. & Cirelli, C. (2014). Sleep and the Price of Plasticity: From Synaptic and Cellular Homeostasis to Memory Consolidation and Integration. _Neuron_, 81(1), 12-34.

---

## Agent Memory Systems

- Park, J.S. et al. (2023). Generative Agents: Interactive Simulacra of Human Behavior. _UIST_, 2023. arXiv:2304.03442.

- Chhikara, P. et al. (2025). Mem0: Building Production-Ready AI Agents with Scalable Long-Term Memory. arXiv:2504.19413.

- Xu, W. et al. (2025). A-MEM: Agentic Memory for LLM Agents. arXiv:2502.12110.

- Anokhin, P. et al. (2024). AriGraph: Learning Knowledge Graph World Models with Episodic Memory for LLM Agents. arXiv:2407.04363.

- Packer, C. et al. (2023). MemGPT: Towards LLMs as Operating Systems. arXiv:2310.08560.

- Zhong, W. et al. (2024). MemoryBank: Enhancing Large Language Models with Long-Term Memory. _AAAI_, 2024.

- arXiv:2505.16067 (2025). How Memory Management Impacts LLM Agents: An Empirical Study of Experience-Following Behavior.

---

## Agent Memory Surveys and Taxonomies (2025)

- Hu et al. (2025). Memory in the Age of AI Agents: A Survey. arXiv:2512.13564.
  *Grounds: Agent memory taxonomy — finer-grained taxonomy distinguishing factual, experiential, and working memory. Analyzes how memory is formed, evolved, and retrieved over time. Validates NeuroStore's multi-type knowledge architecture.*

- Du et al. (2025). Rethinking Memory in LLM-based Agents: Representations, Operations, and Emerging Topics. arXiv:2505.00675.

- Memory for Autonomous LLM Agents (2026). Mechanisms, Evaluation, and Emerging Frontiers. arXiv:2603.07670.
  *Grounds: Agent memory formalization — formalizes agent memory as a write-manage-read loop coupled with perception and action. Five mechanism families: context-resident compression, retrieval-augmented stores, reflective self-improvement, hierarchical virtual context, and policy-learned management. Validates Roko's separation of memory management from inference.*

- Honda et al. (2025). Human-Like Remembering and Forgetting in LLM Agents: An ACT-R-Inspired Memory Architecture. _HAI_, 2025.

---

## Knowledge Compression and Transfer

- Bartlett, F.C. (1932). _Remembering: A Study in Experimental and Social Psychology_. Cambridge University Press.

---

## Cross-References

- See [00-lifecycle-and-finite-agency.md](./00-lifecycle-and-finite-agency.md) for lifecycle-level forgetting motivation
- See [03-dreams-and-offline-learning.md](./03-dreams-and-offline-learning.md) for sleep-dependent consolidation mechanisms
- See topic [06-neuro](../06-neuro/INDEX.md) for how these citations ground the NeuroStore implementation
- See topic [05-learning](../05-learning/INDEX.md) for the learning loop architecture
