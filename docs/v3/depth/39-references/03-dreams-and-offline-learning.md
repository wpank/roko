# 39-03 Dreams and Offline Learning -- Annotated Reference Map

> Annotations are kept only for works whose descriptions were checked against the papers (listed in
> `tools/docs_integrity/content_audit.json` and in the banner of docs/v3/REFERENCES.md). The other entries are
> bare citations: their annotations were removed because they had not been checked against the papers.

> Research foundations for offline consolidation, creative hypothesis generation,
> and sleep-time compute in the Roko Dreams subsystem.
>
> **v3 depth file** -- updated 2026-09-15. Added sleep-time compute papers.

---

## Hypnagogia and Creative Insight

**[Lacaux et al., 2021]** *Sleep onset is a creative sweet spot.* Science Advances, 7(50), eabj5866.

**[Lacaux et al., 2024]** *Embracing sleep-onset complexity.* Trends in Neurosciences, 47(4), 273--288.

**[Haar Horowitz et al., 2020]** *Dormio: A Targeted Dream Incubation Device.* Consciousness and Cognition, 83, 102938.

**[Horowitz et al., 2023]** *Targeted Dream Incubation at Sleep Onset Increases Post-Sleep Creativity.* Scientific Reports, 13, 7319.

---

## World Models and Imagined Trajectories

**[Hafner et al., 2025]** *DreamerV3: Mastering Diverse Domains through World Models.* Working paper.

**[Ha & Schmidhuber, 2018]** *World Models.* arXiv:1803.10122.

---

## Sleep-Time Compute

**[Lin et al., 2025]** *Sleep-time Compute: Beyond Inference Scaling at Test-Time.* arXiv:2504.13171.
Lets a model think about a context offline, before queries arrive, so answering needs less test-time compute (§3): ~5x less test-time compute at equal accuracy on Stateful GSM-Symbolic and Stateful AIME, up to 13% and 18% higher accuracy with more sleep-time compute, and 2.5x lower cost per query when amortised over related queries (abstract, §5). Direct validation of the Dreams subsystem.

**[Sorrenti et al., 2024]** *Wake-Sleep Consolidated Learning.* arXiv:2401.08623.

---

## Replay and Experience Prioritization

**[Wagner et al., 2004]** *Sleep Inspires Insight.* Nature, 427, 352--355.

**[Chen et al., 2024]** *Enhancing LLM Agents for Code Generation with Possibility and Pass-rate Prioritized Experience Replay.* arXiv:2410.12236.

**[Wang et al., 2024]** *Prioritized Generative Replay.* arXiv:2410.18082.

**[Van de Ven et al., 2020]** *Brain-Inspired Replay for Continual Learning.* Nature Communications, 11, 4069.

---

## Creativity Theory

**[Boden, 2004]** *The Creative Mind: Myths and Mechanisms.* 2nd ed. Routledge.

---

## Causal Models for Counterfactuals

**[Pearl, 2009]** *Causality: Models, Reasoning, and Inference.* 2nd ed. Cambridge University Press.

---

## Hauntology

**[Derrida, 1993]** *Specters of Marx.* Routledge (English translation 1994).

---

## Sleep-Inspired LLM Architectures (2025--2026)

**[Behrouz et al., 2025]** *Language Models Need Sleep.* OpenReview.

**[Tutuncuoglu, 2025]** *NeuroDream: A Sleep-Inspired Memory Consolidation Framework.* SSRN:5377250.

**[Fang et al., 2025]** *LightMem: Lightweight and Efficient Memory-Augmented Generation.* arXiv:2510.18866.
Up to 10.9% accuracy gain and up to 117x fewer tokens in its v1 abstract (later versions report up to 7.7%/29.3% and 106x/117x online), with consolidation moved offline into a sleep-time update (§3.3). Validates Delta-frequency consolidation.

**[Xie, 2025]** *SleepGate: Learning to Forget.* arXiv:2603.14517.

**[Ravindran, 2025]** *CosmoCore: Affective Dream-Replay RL for Code Generation.* arXiv:2510.18895.

**[Ye et al., 2026]** *Auto-Dreamer.* arXiv:2605.20616.

**[Behrouz et al., 2026]** *Language Models Need Sleep: Learning to Self-Modify and Consolidate Memories.* arXiv:2606.03979.

**[Lee et al., 2026]** *Do Language Models Need Sleep? Offline Recurrence for Improved Online Inference.* arXiv:2605.26099.
Offline recurrent passes consolidate context into fast weights; longer sleep improves accuracy (§5, §6). Further validates the dream approach.

**[Li et al., 2026]** *TiMem: Temporal-Hierarchical Memory Consolidation for Long-Horizon Conversational Agents.* arXiv:2601.02845.

**[Phasor Agents, 2026]** *Phasor Agents: Oscillatory Graphs with Three-Factor Plasticity and Sleep-Staged Learning.* arXiv:2601.04362.

---

## Cross-References

- Replay citations: [01-memory-consolidation](./01-memory-consolidation.md)
- Emotional depotentiation: [02-affective-computing](./02-affective-computing.md)
- Derrida in philosophy: [13-philosophy](./13-philosophy.md)
