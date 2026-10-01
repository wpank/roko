# 39-14 Agent Harnesses and Tool Use -- Annotated Reference Map

> Annotations are kept only for works whose descriptions were checked against the papers (listed in
> `tools/docs_integrity/content_audit.json` and in the banner of docs/v3/REFERENCES.md). The other entries are
> bare citations: their annotations were removed because they had not been checked against the papers.

> Research foundations for scaffold engineering, agent-computer interfaces, tool use
> patterns, and cascade architectures in Roko's dispatch and execution systems.
>
> **v3 depth file** -- updated 2026-09-15. Added HarnessX, AHE, Harness-Bench.

---

## Harness Engineering (Core Thesis)

**[Lee et al., 2026]** *Meta-Harness: End-to-End Optimization of Model Harnesses.* arXiv:2603.28052.
 The 6x harness gap its §1 quotes is SWE-bench Mobile's result (Tian et al. 2026), not its own. +7.7 points text classification, +4.7 IMO math, 4x fewer tokens. Core thesis for Roko's approach (Roko's reading; the paper shows only that the harness matters, §1, §4): improving the harness can matter as much as switching models.

**[Pan et al., 2026]** *Natural-Language Agent Harnesses.* arXiv:2603.25723.

**[Kapoor et al., 2026]** *HAL: A Holistic Agent Leaderboard.* ICLR 2026.

---

## 2026 Additions: HarnessX, AHE, Harness-Bench

**[Chen et al., 2026]** *HarnessX: A Composable, Adaptive, and Evolvable Agent Harness Foundry.* arXiv:2606.14249.
Crossover optimization of scaffold components from heterogeneous agent architectures. Discovers productive combinations of prompt structure, tool sets, and verification strategies that no single agent design would explore. Validates Roko's composable trait system as a scaffold search space.

---

## Agent-Computer Interfaces

**[Yang et al., 2024]** *SWE-agent: Agent-Computer Interfaces Enable Automated Software Engineering.* NeurIPS 2024.

**[Gauthier, 2024]** *Aider: AI Pair Programming in Your Terminal.* aider.chat.

**[Schluntz & Zhang, 2024]** *Building Effective Agents.* anthropic.com.

---

## Cascade Architectures

**[Chen, Zaharia & Zou, 2023]** *FrugalGPT: How to Use LLMs While Reducing Cost and Improving Performance.* arXiv:2305.05176.
Cascade architectures achieve up to 98% cost reduction. Grounds the T0/T1/T2 cascade in `CascadeRouter`.

**[Ong et al., 2024]** *RouteLLM: Learning to Route LLMs with Preference Data.* arXiv:2406.18665.

**[Gandhi et al., 2024]** *BudgetMLAgent: A Cost-Effective LLM Multi-Agent system for Automating Machine Learning Tasks.* AIMLSystems 2024.

---

## Cognitive Pipelines

**[Sumers et al., 2023]** *Cognitive Architectures for Language Agents (CoALA).* arXiv:2309.02427.
Modular memory, structured action spaces and a decision cycle of planning (proposal, evaluation, selection) then execution (§4). It defines no 9-step pipeline; that list is Roko's. Roko's loop extends CoALA with verification and meta-cognition.

**[Yao et al., 2023]** *ReAct: Synergizing Reasoning and Acting in Language Models.* ICLR 2023. arXiv:2210.03629.

**[Zhou et al., 2024]** *Language Agent Tree Search Unifies Reasoning, Acting, and Planning.* ICML 2024. arXiv:2310.04406.

**[Yao et al., 2023b]** *Tree of Thoughts: Deliberate Problem Solving with Large Language Models.* NeurIPS 2023. arXiv:2305.10601.

---

## Tool Use Optimization

**[Jia & Li, 2025]** *AutoTool: Efficient Tool Selection for Large Language Model Agents.* AAAI 2026. arXiv:2511.14650.

**[Anonymous, 2025]** *Parallelizing Tool Execution and LLM Generation for Low-Latency Agent Serving.* Microsoft Research. arXiv:2603.18897.
48.5% shorter average task completion time via speculative tool execution (v1 abstract; 43.5% in the current version). Informs parallel tool dispatch.

**[Zhai et al., 2026]** *ToolCaching: Towards Efficient Caching for LLM Tool-calling.* arXiv:2601.15335.

**[Zhang et al., 2025]** *Agentic Plan Caching.* arXiv:2506.14852.

**[Red Hat, 2025]** *Tool RAG: Next Breakthrough in Scalable AI Agents.* 2025.

---

## Benchmarks

**[Jimenez et al., 2024]** *SWE-bench: Can Language Models Resolve Real-World GitHub Issues?* ICLR 2024.

**[Liu et al., 2024]** *AgentBench: Evaluating LLMs as Agents.* ICLR 2024.

**[Shahul Es et al., 2024]** *RAGAS: Automated Evaluation of Retrieval Augmented Generation.* EACL 2024.

**[Saad-Falcon et al., 2024]** *ARES: An Automated Evaluation Framework for Retrieval-Augmented Generation Systems.* NAACL 2024.

---

## Cross-References

- Self-learning: [06-self-learning-systems](./06-self-learning-systems.md)
- Context engineering: [07-context-engineering](./07-context-engineering.md)
- Process reward models: [17-process-reward-models](./17-process-reward-models.md)
- Harness engineering (2026): [24-additions-2025-2026](./24-additions-2025-2026.md) -- section 26
