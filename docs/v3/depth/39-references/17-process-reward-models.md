# 39-17 Process Reward Models -- Annotated Reference Map

> Annotations are kept only for works whose descriptions were checked against the papers (listed in
> `tools/docs_integrity/content_audit.json` and in the banner of docs/v3/REFERENCES.md). The other entries are
> bare citations: their annotations were removed because they had not been checked against the papers.

> Research foundations for step-level verification, process supervision, and
> continuous gate signals in Roko's Gate pipeline.
>
> **v3 depth file** -- updated 2026-09-15. Added AgentPRM.

---

## Step-Level Verification

**[Lightman et al., 2024]** *Let's Verify Step by Step.* arXiv:2305.20050.
Process reward models that verify each reasoning step outperform outcome-only verification. Foundational for the Gate pipeline's per-step verification architecture. The 7-rung gate pipeline applies process-level verification at each stage.

**[Wei et al., 2022]** *Chain-of-Thought Prompting Elicits Reasoning in Large Language Models.* NeurIPS 2022. arXiv:2201.11903.

**[Wang et al., 2023]** *Self-Consistency Improves Chain of Thought Reasoning.* ICLR 2023.

---

## Generation-Verification Gap

**[Song et al., 2025]** *Mind the Gap: Examining the Self-Improvement Capabilities of Large Language Models.* ICLR 2025.

**[Huang et al., 2024]** *Large Language Models Cannot Self-Correct Reasoning Yet.* ICLR 2024.

**[Pan et al., 2024]** *Spontaneous Reward Hacking in Iterative Self-Refinement.* arXiv:2407.04549.

---

## Agent Process Reward Models (2025--2026)

**[Xi et al., 2025]** *AgentPRM: Agent Process Reward Model.* arXiv:2511.08325. WWW 2026.
Continuous progress signals via temporal-difference estimation + Generalized Advantage Estimation. Over 8x more compute-efficient than baseline reward models (outcome reward models and step-value models) in Best-of-N search (§1, §4.2). Target for upgrading Roko's gate pipeline from binary pass/fail to continuous progress signals. The TD-based approach would allow gates to provide gradient information, not just accept/reject.

**[Agrawal et al., 2026]** *GEPA: Reflective Prompt Evolution.* ICLR (Oral) 2026. arXiv:2507.19457.

**[Mukhal et al., 2025]** *ThinkPRM: Process Reward Model with Thinking Traces.* Working paper.

---

## Continuous and Partial Verification

**[Krsteski et al., 2026]** *Messier: Partial-Pass Scoring Rationale.* arXiv:2607.25891.

**[Song et al., 2026]** *PACE: A Proxy for Agentic Capability Evaluation.* arXiv:2607.02032.

---

## Search and Retrieval Verification

**[Jin et al., 2025]** *Search-R1: Training LLMs to Reason and Leverage Search Engines with Reinforcement Learning.* 2025. arXiv:2503.09516.

**[Xiong et al., 2025]** *Supervising the search process produces reliable and generalizable information-seeking agents.* 2025.

---

## Foundational Verification Approaches

**[Cobbe et al., 2021]** *Training Verifiers to Solve Math Word Problems.* arXiv:2110.14168.

**[Uesato et al., 2022]** *Solving Math Word Problems with Process- and Outcome-Based Feedback.* arXiv:2211.14275.

---

## Verification Scaling Laws

**[Snell et al., 2024]** *Scaling LLM Test-Time Compute Optimally Can Be More Effective Than Scaling Model Parameters.* arXiv:2408.03314.

**[Brown et al., 2024]** *Large Language Monkeys: Scaling Inference Compute with Repeated Sampling.* arXiv:2407.21787.

---

## Cross-References

- Self-learning: [06-self-learning-systems](./06-self-learning-systems.md)
- Agent harnesses: [14-agent-harnesses-and-tool-use](./14-agent-harnesses-and-tool-use.md)
- Gate pipeline design: depth/07-gates/
