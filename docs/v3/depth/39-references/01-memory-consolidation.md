# 39-01 Memory Consolidation -- Annotated Reference Map

> Annotations are kept only for works whose descriptions were checked against the papers (listed in
> `tools/docs_integrity/content_audit.json` and in the banner of docs/v3/REFERENCES.md). The other entries are
> bare citations: their annotations were removed because they had not been checked against the papers.

> Research foundations for memory systems, forgetting as optimization, and knowledge
> tier progression in the Roko NeuroStore.
>
> **v3 depth file** -- updated 2026-09-15. Added Auto-Dreamer, FadeMem, MemPro.

---

## Complementary Learning Systems

**[McClelland, McNaughton & O'Reilly, 1995]** *Why There Are Complementary Learning Systems in the Hippocampus and Neocortex.* Psychological Review, 102(3), 419--457.

**[Kumaran, Hassabis & McClelland, 2016]** *What Learning Systems do Intelligent Agents Need?* Trends in Cognitive Sciences, 20(7), 512--534.

**[O'Reilly et al., 2014]** *Complementary Learning Systems.* Cognitive Science, 38(Suppl 1).

**[McCloskey & Cohen, 1989]** *Catastrophic Interference in Connectionist Networks.* Psychology of Learning and Motivation, 24, 109--165.

---

## Forgetting as Optimization

**[Richards & Frankland, 2017]** *The Persistence and Transience of Memory.* Neuron, 94(6), 1071--1084.

**[Ebbinghaus, 1885]** *Uber das Gedachtnis.* Leipzig.

**[Davis & Zhong, 2017]** *The Biology of Forgetting -- A Perspective.* Neuron, 95(3), 490--503.

**[Hardt, Nader & Nadel, 2013]** *Decay Happens: The Role of Active Forgetting in Memory.* Trends in Cognitive Sciences, 17(3), 111--120.

**[Nader et al., 2000]** *Fear Memories Require Protein Synthesis in the Amygdala for Reconsolidation.* Nature, 406, 722--726.

---

## Retrieval and Spacing Effects

**[Roediger & Karpicke, 2006]** *Test-Enhanced Learning.* Psychological Science, 17(3), 249--255.

**[Cepeda et al., 2006]** *Distributed Practice in Verbal Recall Tasks.* Psychological Bulletin, 132(3), 354--380.

---

## Prioritized Replay

**[Wilson & McNaughton, 1994]** *Reactivation of Hippocampal Ensemble Memories During Sleep.* Science, 265(5172), 676--679.

**[Mattar & Daw, 2018]** *Prioritized Memory Access Explains Planning and Hippocampal Replay.* Nature Neuroscience, 21(11), 1609--1617.

**[Schaul et al., 2016]** *Prioritized Experience Replay.* ICLR 2016. arXiv:1511.05952.

---

## Sleep-Dependent Consolidation

**[Stickgold, 2005]** *Sleep-Dependent Memory Consolidation.* Nature, 437, 1272--1278.

**[Born & Wilhelm, 2012]** *System Consolidation of Memory During Sleep.* Psychological Research, 76, 192--203.

**[Tononi & Cirelli, 2014]** *Sleep and the Price of Plasticity.* Neuron, 81(1), 12--34.

---

## Agent Memory Systems

**[Park et al., 2023]** *Generative Agents: Interactive Simulacra of Human Behavior.* UIST 2023. arXiv:2304.03442.

**[Chhikara et al., 2025]** *Mem0: Building Production-Ready AI Agents with Scalable Long-Term Memory.* arXiv:2504.19413.

**[Xu et al., 2025]** *A-MEM: Agentic Memory for LLM Agents.* arXiv:2502.12110.

**[Anokhin et al., 2024]** *AriGraph: Learning Knowledge Graph World Models with Episodic Memory.* arXiv:2407.04363.

**[Packer et al., 2023]** *MemGPT: Towards LLMs as Operating Systems.* arXiv:2310.08560.

**[Zhong et al., 2024]** *MemoryBank: Enhancing Large Language Models with Long-Term Memory.* AAAI 2024.

**[Xiong et al., 2025]** *How Memory Management Impacts LLM Agents: An Empirical Study of Experience-Following Behavior.* arXiv:2505.16067.

---

## Surveys and Taxonomies (2025--2026)

**[Hu et al., 2025]** *Memory in the Age of AI Agents: A Survey.* arXiv:2512.13564.
Factual, experiential, and working memory taxonomy. Validates NeuroStore's multi-type architecture.

**[Du et al., 2025]** *Rethinking Memory in LLM-based Agents.* arXiv:2505.00675.

**[Du, 2026]** *Memory for Autonomous LLM Agents: Mechanisms, Evaluation, and Emerging Frontiers.* arXiv:2603.07670.
Write-manage-read formalization with five mechanism families. Validates separation of memory management from inference.

**[Honda et al., 2025]** *Human-Like Remembering and Forgetting in LLM Agents: An ACT-R-Inspired Memory Architecture.* HAI 2025.

**[Pink et al., 2025]** *Position: Episodic Memory is the Missing Piece for Long-Term LLM Agents.* arXiv:2502.06975.

**[Logan, 2026]** *Continuum Memory Architectures for Long-Horizon Agents.* arXiv:2601.09913.

---

## 2026 Additions: Auto-Dreamer, FadeMem, MemPro

**[Ye et al., 2026]** *Auto-Dreamer.* arXiv:2605.20616.

**[FadeMem, 2026]** *FadeMem: Validates Ebbinghaus-Based Decay.* arXiv:2601.18642.
Empirical validation of Ebbinghaus-based decay curves for LLM agents. Directly supports `roko-neuro`'s existing half-life approach with independent experimental evidence.

**[Liu et al., 2026]** *MemPro: Agentic Memory Systems as Evolvable Programs.* arXiv:2606.00619.
Self-improving memory pipelines that evolve their own consolidation strategies. Aspirational target for memory system auto-improvement via the meta-learning loop.

---

## Knowledge Compression

**[Bartlett, 1932]** *Remembering: A Study in Experimental and Social Psychology.* Cambridge University Press.

---

## Cross-References

- Lifecycle-level forgetting: [00-lifecycle-and-finite-agency](./00-lifecycle-and-finite-agency.md)
- Sleep-dependent consolidation: [03-dreams-and-offline-learning](./03-dreams-and-offline-learning.md)
- Forgetting governance: [24-additions-2025-2026](./24-additions-2025-2026.md) -- see section 29
