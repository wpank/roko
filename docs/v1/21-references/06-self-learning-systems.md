# Self-Learning Systems

> Annotations are kept only for works whose descriptions were checked against the papers (listed in
> `tools/docs_integrity/content_audit.json` and in the banner of docs/v3/REFERENCES.md). The other entries are
> bare citations: their annotations were removed because they had not been checked against the papers.

> **DEPRECATED (v1):** This document is part of the v1 specification and may be outdated. See [../../v2/](../../v2/) for the current reference.


> Academic foundations for agent self-improvement, experiential learning, skill evolution, and metacognitive loops in Roko's learning subsystems.

**Topic**: [References](./INDEX.md)
**Prerequisites**: [Architecture](../00-architecture/INDEX.md), [Learning](../05-learning/INDEX.md)
**Key sources**: `bardo-backup/prd/02-mortality/14-research-foundations.md` §7, `bardo-backup/tmp/mori-agents/12-references.md`

> **Implementation**: Reference

---

## Abstract

An agent that does not improve is an expensive cron job. The research here establishes how agents improve without human retraining — through verbal self-reflection (Reflexion), cross-episode experience extraction (ExpeL), code-as-action skill libraries (Voyager), and metacognitive loops that improve the learning process itself (ACE, Argyris). The critical finding: these mechanisms must be architecturally integrated, not bolted on. The triple-loop (execution, strategy, meta) maps to Roko's Gamma (reactive), Theta (reflective), and Delta (consolidation) frequencies.

---

## Verbal Self-Reflection

- Shinn, N. et al. (2023). Reflexion: Language Agents with Verbal Reinforcement Learning. _NeurIPS_, 2023. arXiv:2303.11366.
  *Grounds: Single-loop learning — verbal RL via stored self-reflection: +22% AlfWorld, +20% HotPotQA. Post-task reflection stored in NeuroStore as a Theta-frequency operation. Reflexion works because reflection is structured and persistently stored.*

---

## Experiential Learning

- Zhao, A. et al. (2024). ExpeL: LLM Agents Are Experiential Learners. arXiv:2308.10144.

- Wang, G. et al. (2023). Voyager: An Open-Ended Embodied Agent with Large Language Models. arXiv:2305.16291.

---

## Meta-Harness and Scaffold Self-Improvement

- Lee et al. (2026). Meta-Harness: End-to-End Optimization of Model Harnesses. arXiv:2603.28052.
  *Grounds: "The scaffold IS the product" thesis — The 6x harness gap its §1 quotes is SWE-bench Mobile's result (Tian et al. 2026), not its own. +7.7 points on text classification, +4.7 on IMO math, at 4x fewer tokens. The foundational paper for Roko's harness engineering approach.*

- Pan et al. (2026). Natural-Language Agent Harnesses. arXiv:2603.25723.

- Kapoor, S. et al. (2026). HAL: A Holistic Agent Leaderboard. _ICLR_, 2026.

---

## Prompt and Strategy Evolution

- Guo, Q. et al. (2024). EvoPrompt: Connecting Large Language Models with Evolutionary Algorithms Yields Powerful Prompt Optimizers. arXiv:2309.08532.

- Fernando, C. et al. (2024). Promptbreeder: Self-Referential Self-Improvement via Prompt Evolution. arXiv:2309.16797.

- Khattab, O. et al. (2024). DSPy: Compiling Declarative Language Model Calls into Self-Improving Pipelines. _ICLR_, 2024. arXiv:2310.03714.

- Opsahl-Ong, K. et al. (2024). MIPROv2: Optimizing Instructions and Demonstrations for Multi-Stage Language Model Programs. _EMNLP_, 2024.

---

## Architecture Search

- Hu, S. et al. (2025). Automated Design of Agentic Systems (ADAS). _ICLR_, 2025.

---

## Process Reward Models

- Lightman, H. et al. (2024). Let's Verify Step by Step. arXiv:2305.20050.
  *Grounds: Step-level verification — process reward models that verify each reasoning step outperform outcome-only verification. Grounds the Gate pipeline's per-step verification architecture. Cross-referenced in [17-process-reward-models.md](./17-process-reward-models.md).*

- Song, Y. et al. (2025). Mind the Gap: Examining the Self-Improvement Capabilities of Large Language Models. _ICLR_, 2025.

---

## Self-Correction Limitations

- Huang, J. et al. (2024). Large Language Models Cannot Self-Correct Reasoning Yet. _ICLR_, 2024.

- Pan, A. et al. (2024). Spontaneous Reward Hacking in Iterative Self-Refinement. _ICML_, 2024.

---

## Triple-Loop Learning

- Argyris, C. & Schön, D. (1978). _Organizational Learning_. Addison-Wesley.

---

## Bandit-Based Optimization

- TensorZero (2025). Track-and-Stop Optimal Bandits in an LLM Gateway. 2025.

- MASPOB (2026). MASPOB: Bandit-Based Prompt Optimization for Multi-Agent Systems with Graph Neural Networks. arXiv, 2026.


---

## Multi-Level Reflection (2025)

- Ge et al. (2025). Self-Learning Agents Enhanced by Multi-level Reflection. _EMNLP_, 2025. arXiv:2509.20562.

- MAR (2025). Multi-Agent Reflexion Improves Reasoning Abilities in LLMs. arXiv:2512.20845.

---

## Cross-References

- See [07-context-engineering.md](./07-context-engineering.md) for ACE, CSO, ACON context self-improvement
- See [14-agent-harnesses-and-tool-use.md](./14-agent-harnesses-and-tool-use.md) for SWE-agent, Aider, and harness engineering
- See [17-process-reward-models.md](./17-process-reward-models.md) for Lightman and AgentPRM
- See topic [06-learning](../05-learning/INDEX.md) for full learning subsystem design
