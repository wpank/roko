# Dreams and Offline Learning

> Annotations are kept only for works whose descriptions were checked against the papers (listed in
> `tools/docs_integrity/content_audit.json` and in the banner of docs/v3/REFERENCES.md). The other entries are
> bare citations: their annotations were removed because they had not been checked against the papers.

> **DEPRECATED (v1):** This document is part of the v1 specification and may be outdated. See [../../v2/](../../v2/) for the current reference.


> Academic foundations for offline consolidation, creative hypothesis generation, and sleep-time compute in the Roko Dreams subsystem.

**Topic**: [References](./INDEX.md)
**Prerequisites**: [Architecture](../00-architecture/INDEX.md), [Dreams](../10-dreams/INDEX.md)
**Key sources**: `bardo-backup/prd/02-mortality/14-research-foundations.md` §4, `bardo-backup/prd/shared/citations.md` §28

> **Implementation**: Reference

---

## Abstract

The brain dedicates 25-33% of its runtime to a state that prevents environmental interaction — an enormous evolutionary cost that must confer proportional benefits. The benefits are well-characterized: memory consolidation, emotional depotentiation, counterfactual hypothesis generation, and catastrophic forgetting prevention. For Roko agents, idle periods are not waste — they are budget for offline cognitive work. The Dreams subsystem implements three-phase consolidation: NREM replay (Mattar-Daw prioritized access), REM imagination (Boden creativity modes + Pearl SCM counterfactuals + Walker emotional depotentiation), and integration staging.

---

## Hypnagogia and Creative Insight

- Lacaux, C., Andrillon, T., Arnulf, I., & Oudiette, D. (2021). Sleep onset is a creative sweet spot. _Science Advances_, 7(50), eabj5866.

- Lacaux, C. et al. (2024). Embracing sleep-onset complexity: A Comprehensive Review of the N1 Stage. _Trends in Neurosciences_, 47(4), 273-288.

- Haar Horowitz, A. et al. (2020). Dormio: A Targeted Dream Incubation Device. _Consciousness and Cognition_, 83, 102938.

- Haar Horowitz, A., Cunningham, T.J., Maes, P., & Stickgold, R. (2023). Targeted Dream Incubation at Sleep Onset Increases Post-Sleep Creativity. _Scientific Reports_, 13, 7319.

---

## World Models and Imagined Trajectories

- Hafner, D. et al. (2025). DreamerV3: Mastering Diverse Domains through World Models. Working paper.

- Ha, D. & Schmidhuber, J. (2018). World Models. arXiv:1803.10122.

---

## Sleep-Time Compute

- Lin, B. et al. (2025). Sleep-time Compute: Beyond Inference Scaling at Test-Time. arXiv:2504.13171.
  *Grounds: Delta-frequency scheduling — the model thinks about a context offline, before queries arrive (§3). ~5× test-time compute reduction, up to 18% accuracy gain. For Roko, dream cycles are sleep-time compute — agents that process experiences during low-activity periods execute fewer expensive T2 inference calls during active work.*

---

## Wake-Sleep Learning

- Sorrenti et al. (2024). Wake-Sleep Consolidated Learning. arXiv:2401.08623.

---

## Replay and Experience Prioritization

- Wagner, U. et al. (2004). Sleep Inspires Insight. _Nature_, 427, 352-355.

- Chen et al. (2024). Enhancing LLM Agents for Code Generation with Possibility and Pass-rate Prioritized Experience Replay. arXiv:2410.12236.

- Wang, X. et al. (2024). Prioritized Generative Replay. arXiv:2410.18082.

- Van de Ven, G.M., Siegelmann, H.T., & Tolias, A.S. (2020). Brain-Inspired Replay for Continual Learning with Artificial Neural Networks. _Nature Communications_, 11, 4069.

---

## Creativity Theory

- Boden, M.A. (2004). _The Creative Mind: Myths and Mechanisms_. 2nd ed. Routledge.

---

## Causal Models for Counterfactuals

- Pearl, J. (2009). _Causality: Models, Reasoning, and Inference_. 2nd ed. Cambridge University Press.

---

## Hauntology and Experiential Traces

- Derrida, J. (1993). _Specters of Marx: The State of the Debt, the Work of Mourning and the New International_. Routledge (English translation 1994).

---

## Generative Agents and Reflection

- Park, J.S. et al. (2023). Generative Agents: Interactive Simulacra of Human Behavior. _UIST_, 2023. arXiv:2304.03442.

---

## Sleep-Inspired Architectures for LLMs (2025)

- Language Models Need Sleep (2025). Learning to Self-Modify and Consolidate Memories. OpenReview.

- Tutuncuoglu, B.T. (2025). NeuroDream: A Sleep-Inspired Memory Consolidation Framework for Artificial Neural Networks. SSRN:5377250.

- Fang et al. (2025). Lightweight and Efficient Memory-Augmented Generation. arXiv:2510.18866.
  *Grounds: Sleep-time memory update — offline consolidation procedure decouples memory management from online inference; up to 10.9% accuracy improvement with 117x token reduction. Validates Delta-frequency consolidation operating asynchronously from Gamma-frequency execution.*

- Xie (2025). Learning to Forget: Sleep-Inspired Memory Consolidation for Resolving Proactive Interference in LLMs. arXiv:2603.14517.

- Ravindran (2025). Affective Dream-Replay Reinforcement Learning for Code Generation. arXiv:2510.18895.

---

## Cross-References

- See [01-memory-consolidation.md](./01-memory-consolidation.md) for Wilson & McNaughton 1994, Mattar & Daw 2018, and other replay citations
- See [02-affective-computing.md](./02-affective-computing.md) for Walker & van der Helm 2009 emotional depotentiation
- See [13-philosophy.md](./13-philosophy.md) for Derrida 1993 in the philosophy context
- See topic [10-dreams](../10-dreams/INDEX.md) for full Dreams subsystem design
