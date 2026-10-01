# 39-06 Self-Learning Systems -- Annotated Reference Map

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
Cross-task experience extraction; insights accumulate across episodes. Grounds double-loop learning.

**[Wang et al., 2023]** *Voyager: An Open-Ended Embodied Agent.* arXiv:2305.16291.
Code-as-action skill library; 3.3x more unique behaviors. Grounds EvoSkills and persistent skill accumulation.

---

## Meta-Harness and Scaffold Self-Improvement

**[Lee et al., 2026]** *Meta-Harness: End-to-End Optimization of Model Harnesses.* arXiv:2603.28052.
6x performance gap from scaffold changes alone. Core thesis for Roko's approach: the scaffold IS the product.

**[Pan et al., 2026]** *Natural-Language Agent Harnesses.* arXiv:2603.25723.
Scaffold logic as natural language specifications. Makes scaffold design inspectable and portable.

**[Kapoor et al., 2026]** *HAL: A Holistic Agent Leaderboard.* ICLR 2026.
21,730 rollouts show scaffold choice matters as much as model choice.

---

## Prompt and Strategy Evolution

**[Guo et al., 2024]** *EvoPrompt.* arXiv:2309.08532.
GA prompt optimization; +25% on BBH tasks. Grounds evolutionary strategy selection.

**[Fernando et al., 2024]** *Promptbreeder: Self-Referential Self-Improvement.* arXiv:2309.16797.
Prompts evolving mutation operators. Grounds the meta-learning loop.

**[Khattab et al., 2024]** *DSPy: Compiling Declarative Language Model Calls.* ICLR 2024. arXiv:2310.03714.
Declarative signatures replace hand-crafted prompts. Influences prompt budget allocation.

**[Opsahl-Ong et al., 2024]** *MIPROv2.* EMNLP 2024.
Bayesian optimization for multi-stage LLM programs. Applicable to context assembly optimization.

---

## Architecture Search

**[Hu et al., 2025]** *Automated Design of Agentic Systems (ADAS).* ICLR 2025.
Meta-agent searching agent architecture space. Roko provides the composable trait system ADAS-style search operates over.

---

## Process Verification

**[Lightman et al., 2024]** *Let's Verify Step by Step.* arXiv:2305.20050.
Process reward models outperform outcome-only. Foundational for the Gate pipeline's per-step verification.

**[Song et al., 2025]** *Mind the Gap: Examining the Self-Improvement Capabilities of Large Language Models.* ICLR 2025.
Self-improvement works only when verification exceeds generation. Core architectural principle separating agent and Gate.

**[Huang et al., 2024]** *Large Language Models Cannot Self-Correct Reasoning Yet.* ICLR 2024.
Self-correction without external feedback worsens answers. Motivates external Gates.

**[Pan et al., 2024]** *Spontaneous Reward Hacking in Iterative Self-Refinement.* arXiv:2407.04549.
Same model as generator and judge leads to reward hacking. Validates generator-verifier separation.

---

## Triple-Loop Learning

**[Argyris & Schon, 1978]** *Organizational Learning.* Addison-Wesley.
Single/double/triple-loop learning. Maps to Gamma/Theta/Delta cognitive frequencies.

---

## Bandit-Based Optimization

**[Mishler, 2025]** *Bandits in your LLM Gateway: Improve LLM Applications Faster with Adaptive Experimentation (A/B Testing).* Working paper.
Bandit-based model routing in production gateways. Validates the CascadeRouter approach.

**[Hong et al., 2026]** *MASPOB: Bandit-Based Prompt Optimization for Multi-Agent Systems with Graph Neural Networks.* Working paper. arXiv:2603.02630.
Bandit optimization for multi-agent system prompts.

---

## Multi-Level Reflection (2025)

**[Ge et al., 2025]** *SAMULE: Self-Learning Agents Enhanced by Multi-level Reflection.* EMNLP 2025. arXiv:2509.20562.
Multi-level reflection outperforms single-trajectory Reflexion. Error clustering extracts insight from failures.

**[Ozer et al., 2025]** *MAR: Multi-Agent Reflexion.* arXiv:2512.20845.
Addresses Reflexion's single-agent limitations with multi-agent extension.

**[Kang et al., 2025]** *Self-Evolving LLMs via Continual Instruction Tuning.* arXiv:2509.18133.
Autonomous adaptation and cross-task knowledge integration. Maps to triple-loop learning.

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
Intrinsic metacognition required for genuine self-improvement. Validates the meta-cognition step (Step 9: Daimon.assess()).

---

## Cross-References

- Context self-improvement: [07-context-engineering](./07-context-engineering.md)
- Agent harnesses: [14-agent-harnesses-and-tool-use](./14-agent-harnesses-and-tool-use.md)
- Process reward models: [17-process-reward-models](./17-process-reward-models.md)
