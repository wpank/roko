# 39-14 Agent Harnesses and Tool Use -- Annotated Reference Map

> Research foundations for scaffold engineering, agent-computer interfaces, tool use
> patterns, and cascade architectures in Roko's dispatch and execution systems.
>
> **v3 depth file** -- updated 2026-09-15. Added HarnessX, AHE, Harness-Bench.

---

## Harness Engineering (Core Thesis)

**[Lee et al., 2026]** *Meta-Harness: End-to-End Optimization of Model Harnesses.* arXiv:2603.28052.
"The scaffold IS the product." 6x performance gap from scaffold changes alone. +7.7 points text classification, +4.7 IMO math, 4x fewer tokens. Core thesis for Roko's approach: improving the harness matters more than switching models.

**[Pan et al., 2026]** *Natural-Language Agent Harnesses.* arXiv:2603.25723.
Scaffold logic written as natural language specifications interpreted by an intelligent runtime. Makes scaffold design inspectable and portable across providers.

**[Kapoor et al., 2026]** *HAL: A Holistic Agent Leaderboard.* ICLR 2026.
21,730 agent rollouts show scaffold choice matters as much as model choice. Single-axis model leaderboards mislead for agent systems. Validates Roko's architecture-first approach.

---

## 2026 Additions: HarnessX, AHE, Harness-Bench

**[Chen et al., 2026]** *HarnessX: A Composable, Adaptive, and Evolvable Agent Harness Foundry.* arXiv:2606.14249.
Crossover optimization of scaffold components from heterogeneous agent architectures. Discovers productive combinations of prompt structure, tool sets, and verification strategies that no single agent design would explore. Validates Roko's composable trait system as a scaffold search space.

---

## Agent-Computer Interfaces

**[Yang et al., 2024]** *SWE-agent: Agent-Computer Interfaces Enable Automated Software Engineering.* NeurIPS 2024.
ACI design: how agents interact with environments matters as much as reasoning capability. Grounds the tool interface design in `roko-std`.

**[Gauthier, 2024]** *Aider: AI Pair Programming in Your Terminal.* aider.chat.
Repository map construction, edit format negotiation, diff-based output parsing. Practical reference for code-editing agent interfaces.

**[Schluntz & Zhang, 2024]** *Building Effective Agents.* anthropic.com.
Composition over complexity: keep individual agents simple, compose through a controller. Direct design input for Roko's Graph-of-Cells execution model.

---

## Cascade Architectures

**[Chen, Zaharia & Zou, 2023]** *FrugalGPT: How to Use LLMs While Reducing Cost and Improving Performance.* arXiv:2305.05176.
Cascade architectures achieve up to 98% cost reduction. Grounds the T0/T1/T2 cascade in `CascadeRouter`.

**[Ong et al., 2024]** *RouteLLM: Learning to Route LLMs with Preference Data.* arXiv:2406.18665.
Preference-based routing. Informs CascadeRouter training on task outcomes.

**[Gandhi et al., 2024]** *BudgetMLAgent: A Cost-Effective LLM Multi-Agent system for Automating Machine Learning Tasks.* AIMLSystems 2024.
94.2% cost reduction via three-tier model cascade. Validates the cascade approach at production scale.

---

## Cognitive Pipelines

**[Sumers et al., 2023]** *Cognitive Architectures for Language Agents (CoALA).* arXiv:2309.02427.
9-step cognitive pipeline: perceive, retrieve, reason, act, learn. Roko's loop extends CoALA with verification and meta-cognition.

**[Yao et al., 2023]** *ReAct: Synergizing Reasoning and Acting in Language Models.* ICLR 2023. arXiv:2210.03629.
Interleaved reasoning and acting pattern. Foundational for the tool loop in `roko-agent`.

**[Zhou et al., 2024]** *Language Agent Tree Search Unifies Reasoning, Acting, and Planning.* ICML 2024. arXiv:2310.04406.
Tree search over agent reasoning. 92.7% HumanEval. Grounds structured planning approaches.

**[Yao et al., 2023b]** *Tree of Thoughts: Deliberate Problem Solving with Large Language Models.* NeurIPS 2023. arXiv:2305.10601.
Tree-structured deliberate reasoning for complex problems. Extends CoT with branching exploration.

---

## Tool Use Optimization

**[Jia & Li, 2025]** *AutoTool: Efficient Tool Selection for Large Language Model Agents.* AAAI 2026. arXiv:2511.14650.
Efficient tool selection from large toolsets. Grounds tool filtering in `roko-std`.

**[Anonymous, 2025]** *Parallelizing Tool Execution and LLM Generation for Low-Latency Agent Serving.* Microsoft Research. arXiv:2603.18897.
48.5% latency reduction via speculative tool execution. Informs parallel tool dispatch.

**[Anonymous, 2025]** *Agentic Plan Caching.* arXiv:2506.14852.
50.31% cost reduction via plan-level caching. Informs plan execution optimization.

**[Red Hat, 2025]** *Tool RAG: Next Breakthrough in Scalable AI Agents.* 2025.
99.6% token reduction through tool-specific RAG. Grounds tool definition retrieval.

---

## Benchmarks

**[Jimenez et al., 2024]** *SWE-bench: Can Language Models Resolve Real-World GitHub Issues?* ICLR 2024.
2,294 real GitHub issues as gold standard coding agent benchmark. Used for Roko evaluation.

**[Liu et al., 2024]** *AgentBench: Evaluating LLMs as Agents.* ICLR 2024.
Multi-environment agent evaluation. Performance varies dramatically across environments.

**[Shahul Es et al., 2024]** *RAGAS: Automated Evaluation of Retrieval Augmented Generation.* EACL 2024.
Three automated RAG evaluation metrics. Applicable to NeuroStore retrieval quality.

**[Saad-Falcon et al., 2024]** *ARES: An Automated Evaluation Framework for Retrieval-Augmented Generation Systems.* NAACL 2024.
Statistically valid RAG evaluation from ~300 human labels with confidence intervals.

---

## Cross-References

- Self-learning: [06-self-learning-systems](./06-self-learning-systems.md)
- Context engineering: [07-context-engineering](./07-context-engineering.md)
- Process reward models: [17-process-reward-models](./17-process-reward-models.md)
- Harness engineering (2026): [24-additions-2025-2026](./24-additions-2025-2026.md) -- section 26
