# 39-06 Self-Learning Systems -- Annotated Reference Map

> Annotations are kept only for works whose descriptions were checked against the papers (listed in
> `tools/docs_integrity/content_audit.json` and in the banner of docs/v3/REFERENCES.md). The other entries are
> bare citations: their annotations were removed because they had not been checked against the papers.

> Research foundations for agent self-improvement, experiential learning, skill
> evolution, and metacognitive loops in Roko's learning subsystems.
>
> **v3 depth file** -- updated 2026-09-15. Added GRASP, SiriuS, SkillZip, ReSkill.

---

## Verbal Self-Reflection

**[Shinn et al., 2023]** *Reflexion: Language Agents with Verbal Reinforcement Learning.* NeurIPS 2023. arXiv:2303.11366.
Verbal RL via stored self-reflection: 91% HumanEval. Foundational for the playbook enrichment system.

---

## Experiential Learning

**[Zhao et al., 2024]** *ExpeL: LLM Agents Are Experiential Learners.* arXiv:2308.10144.

**[Wang et al., 2023]** *Voyager: An Open-Ended Embodied Agent.* arXiv:2305.16291.

---

## Meta-Harness and Scaffold Self-Improvement

**[Lee et al., 2026]** *Meta-Harness: End-to-End Optimization of Model Harnesses.* arXiv:2603.28052.
The 6x harness gap its §1 quotes is SWE-bench Mobile's result (Tian et al. 2026), not its own. Core thesis for Roko's approach: the scaffold IS the product.

**[Pan et al., 2026]** *Natural-Language Agent Harnesses.* arXiv:2603.25723.

**[Kapoor et al., 2026]** *HAL: A Holistic Agent Leaderboard.* ICLR 2026.

---

## Prompt and Strategy Evolution

**[Guo et al., 2024]** *EvoPrompt.* arXiv:2309.08532.

**[Fernando et al., 2024]** *Promptbreeder: Self-Referential Self-Improvement.* arXiv:2309.16797.

**[Khattab et al., 2024]** *DSPy: Compiling Declarative Language Model Calls.* ICLR 2024. arXiv:2310.03714.

**[Opsahl-Ong et al., 2024]** *MIPROv2.* EMNLP 2024.

---

## Architecture Search

**[Hu et al., 2025]** *Automated Design of Agentic Systems (ADAS).* ICLR 2025.

---

## Process Verification

**[Lightman et al., 2024]** *Let's Verify Step by Step.* arXiv:2305.20050.
Process reward models outperform outcome-only. Foundational for the Gate pipeline's per-step verification.

**[Song et al., 2025]** *Mind the Gap: Examining the Self-Improvement Capabilities of Large Language Models.* ICLR 2025.

**[Huang et al., 2024]** *Large Language Models Cannot Self-Correct Reasoning Yet.* ICLR 2024.

**[Pan et al., 2024]** *Spontaneous Reward Hacking in Iterative Self-Refinement.* arXiv:2407.04549.

---

## Triple-Loop Learning

**[Argyris & Schon, 1978]** *Organizational Learning.* Addison-Wesley.

---

## Bandit-Based Optimization

**[Mishler, 2025]** *Bandits in your LLM Gateway: Improve LLM Applications Faster with Adaptive Experimentation (A/B Testing).* Working paper.

**[Hong et al., 2026]** *MASPOB: Bandit-Based Prompt Optimization for Multi-Agent Systems with Graph Neural Networks.* Working paper. arXiv:2603.02630.

---

## Multi-Level Reflection (2025)

**[Ge et al., 2025]** *SAMULE: Self-Learning Agents Enhanced by Multi-level Reflection.* EMNLP 2025. arXiv:2509.20562.

**[Ozer et al., 2025]** *MAR: Multi-Agent Reflexion.* arXiv:2512.20845.

**[Kang et al., 2025]** *Self-Evolving LLMs via Continual Instruction Tuning.* arXiv:2509.18133.

**[Bell et al., 2025]** *The Future of Continual Learning in the Era of Foundation Models.* arXiv:2506.03320.
Validates NeuroStore's non-parametric approach to continual knowledge management.

---

## 2025--2026 Additions: GRASP, SiriuS, SkillZip, ReSkill

**[GRASP, 2026]** *GRASP: Gated Regression-Aware Skill Proposer for Self-Improving LLM Agents.* arXiv:2605.29668.
Regression-gated admission for new strategies. 40.6% to 88.8% on MedAgentBench. Target for playbook admission quality gate.

**[Zhao et al., 2025]** *SiriuS: Self-improving Multi-agent Systems via Bootstrapped Reasoning.* arXiv:2502.04780.
Augment failed episodes rather than discarding them. Informs failure-based learning: failed runs generate negative-example knowledge entries.

**[Bai et al., 2026]** *SkillZip: MDL Compression for Skill Libraries.* arXiv:2608.11079.
Minimum Description Length compression prevents unbounded skill library growth. Target for EvoSkills pruning policy.

**[Zhang et al., 2025]** *Darwin Godel Machine: Open-Ended Evolution of Self-Improving Agents.* arXiv:2505.22954.
Open-ended self-improvement through evolutionary architecture search.

**[Liu & van der Schaar, 2025]** *Truly Self-Improving Agents Require Intrinsic Metacognitive Learning.* ICML 2025. arXiv:2506.05109.
Intrinsic metacognition required for genuine self-improvement. Motivates the meta-cognition step (Step 9: Daimon.assess()); it is a position paper, so this is Roko's mapping, not a validation.

---

## Cross-References

- Context self-improvement: [07-context-engineering](./07-context-engineering.md)
- Agent harnesses: [14-agent-harnesses-and-tool-use](./14-agent-harnesses-and-tool-use.md)
- Process reward models: [17-process-reward-models](./17-process-reward-models.md)
