# 39-17 Process Reward Models -- Annotated Reference Map

> Research foundations for step-level verification, process supervision, and
> continuous gate signals in Roko's Gate pipeline.
>
> **v3 depth file** -- updated 2026-09-15. Added AgentPRM.

---

## Step-Level Verification

**[Lightman et al., 2024]** *Let's Verify Step by Step.* arXiv:2305.20050.
Process reward models that verify each reasoning step outperform outcome-only verification. Foundational for the Gate pipeline's per-step verification architecture. The 7-rung gate pipeline applies process-level verification at each stage.

**[Wei et al., 2022]** *Chain-of-Thought Prompting Elicits Reasoning in Large Language Models.* NeurIPS 2022. arXiv:2201.11903.
CoT makes reasoning steps explicit and individually verifiable. Without explicit steps, process verification is impossible. Grounds StrategyFragment knowledge as CoT exemplars.

**[Wang et al., 2023]** *Self-Consistency Improves Chain of Thought Reasoning.* ICLR 2023.
Process verification through multi-path consensus. When multiple reasoning paths agree, the conclusion is more reliable. Applicable to multi-agent verification in Roko.

---

## Generation-Verification Gap

**[Song et al., 2025]** *Mind the Gap: Examining the Self-Improvement Capabilities of Large Language Models.* ICLR 2025.
Self-improvement works only when verification ability exceeds generation ability. If the verifier is weaker than the generator, feedback is noise. Core architectural result validating the separation of agent (generator) and Gate (verifier). The Gate must be at least as capable as the agent at detecting errors.

**[Huang et al., 2024]** *Large Language Models Cannot Self-Correct Reasoning Yet.* ICLR 2024.
LLMs self-correcting without external feedback typically make answers worse. The model's assessment draws on the same biases that produced the original error. Foundational result motivating external verification in Roko's Gate system.

**[Pan et al., 2024]** *Spontaneous Reward Hacking in Iterative Self-Refinement.* arXiv:2407.04549.
When the same model generates and judges, it learns to produce outputs that score well on its own rubric without improving on the task. Validates the structural separation between agent and Gate.

---

## Agent Process Reward Models (2025--2026)

**[Xi et al., 2025]** *AgentPRM: Agent Process Reward Model.* arXiv:2511.08325. WWW 2026.
Continuous progress signals via temporal-difference estimation + Generalized Advantage Estimation. 8x compute efficiency for verification compared to outcome-only methods. Target for upgrading Roko's gate pipeline from binary pass/fail to continuous progress signals. The TD-based approach would allow gates to provide gradient information, not just accept/reject.

**[Agrawal et al., 2026]** *GEPA: Reflective Prompt Evolution.* ICLR (Oral) 2026. arXiv:2507.19457.
Process-level feedback for prompt evolution. Applicable to CascadeRouter optimization -- prompts evolve based on per-step verification outcomes rather than final task outcomes only.

**[Mukhal et al., 2025]** *ThinkPRM: Process Reward Model with Thinking Traces.* Working paper.
Process reward models augmented with thinking traces for richer verification signals. The thinking trace provides evidence for the verification decision, making gates more interpretable.

---

## Continuous and Partial Verification

**[Krsteski et al., 2026]** *Messier: Partial-Pass Scoring Rationale.* arXiv:2607.25891.
Rationale for partial-pass (continuous) verification scores rather than binary pass/fail. Informs the transition from binary gate outcomes to continuous gate signals in Roko's 7-rung pipeline.

**[Song et al., 2026]** *PACE: A Proxy for Agentic Capability Evaluation.* arXiv:2607.02032.
Proxy capability evaluation for efficient assessment without full task execution. Applicable to pre-dispatch capability estimation in the CascadeRouter.

---

## Search and Retrieval Verification

**[Jin et al., 2025]** *Search-R1: Training LLMs to Reason and Leverage Search Engines with Reinforcement Learning.* 2025. arXiv:2503.09516.
RL teaches dynamic search query generation with outcome-based rewards. Each search step is verified for relevance, not just the final answer. Applicable to research agent query refinement.

**[Xiong et al., 2025]** *Supervising the search process produces reliable and generalizable information-seeking agents.* 2025.
Multi-step retrieval as hierarchical MDP. DPO outperforms classical RL for retrieval optimization. Applicable to NeuroStore retrieval quality improvement.

---

## Foundational Verification Approaches

**[Cobbe et al., 2021]** *Training Verifiers to Solve Math Word Problems.* arXiv:2110.14168.
Outcome reward models (ORMs) trained on final answer correctness. Establishes the baseline against which process reward models improve. PRMs consistently outperform ORMs because they provide denser supervision.

**[Uesato et al., 2022]** *Solving Math Word Problems with Process- and Outcome-Based Feedback.* arXiv:2211.14275.
Direct comparison of process vs outcome feedback showing process feedback produces more reliable reasoning chains. Validates per-gate verification over final-outcome-only evaluation.

---

## Verification Scaling Laws

**[Snell et al., 2024]** *Scaling LLM Test-Time Compute Optimally Can Be More Effective Than Scaling Model Parameters.* arXiv:2408.03314.
Optimal allocation of test-time compute between generation and verification. Verification compute has diminishing returns but the optimal ratio varies by task difficulty. Informs how Roko allocates budget between agent dispatch and gate verification.

**[Brown et al., 2024]** *Large Language Monkeys: Scaling Inference Compute with Repeated Sampling.* arXiv:2407.21787.
Coverage scales with repeated sampling -- given enough attempts, even weak verifiers find correct solutions. Validates the retry-on-failure strategy in plan execution.

---

## Cross-References

- Self-learning: [06-self-learning-systems](./06-self-learning-systems.md)
- Agent harnesses: [14-agent-harnesses-and-tool-use](./14-agent-harnesses-and-tool-use.md)
- Gate pipeline design: depth/07-gates/
