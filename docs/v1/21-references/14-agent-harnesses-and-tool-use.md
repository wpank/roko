# Agent Harnesses and Tool Use

> Annotations are kept only for works whose descriptions were checked against the papers (listed in
> `tools/docs_integrity/content_audit.json` and in the banner of docs/v3/REFERENCES.md). The other entries are
> bare citations: their annotations were removed because they had not been checked against the papers.

> **DEPRECATED (v1):** This document is part of the v1 specification and may be outdated. See [../../v2/](../../v2/) for the current reference.


> Academic foundations for agent scaffolding, harness engineering, tool interfaces, and coding agent systems that inform Roko's framework and harness layers.

**Topic**: [References](./INDEX.md)
**Prerequisites**: [Architecture](../00-architecture/INDEX.md), [Framework](../00-architecture/INDEX.md), [Harness](../04-verification/INDEX.md)
**Key sources**: `bardo-backup/tmp/mori-agents/12-references.md`, `bardo-backup/prd/shared/citations.md` §3

> **Implementation**: Reference

---

## Abstract

The Roko thesis — "the scaffold IS the product" — is grounded in empirical evidence that the same LLM performs 6x better or worse depending on its surrounding harness code. This section collects the agent scaffolding research: Meta-Harness (Lee et al. 2026), SWE-agent (Yang et al. 2024), coding agent evaluation (SWE-bench), and practical patterns for multi-agent orchestration. The core finding is that scaffold choice matters as much as model choice for agent performance.

---

## Harness Engineering

- Lee et al. (2026). Meta-Harness: End-to-End Optimization of Model Harnesses. arXiv:2603.28052.
  *Grounds: Core thesis — The 6x harness gap its §1 quotes is SWE-bench Mobile's result (Tian et al. 2026), not its own. +7.7 points text classification, +4.7 points IMO math, at 4x fewer tokens. A coding-agent proposer reads earlier harness candidates' code, scores and execution traces, proposes new harnesses, evaluates them and iterates (§3). The foundational paper for Roko's approach. Cross-referenced in [06-self-learning-systems.md](./06-self-learning-systems.md).*

- Pan et al. (2026). Natural-Language Agent Harnesses. arXiv:2603.25723.

- Kapoor, S. et al. (2026). HAL: A Holistic Agent Leaderboard. _ICLR_, 2026.

---

## Agent-Computer Interfaces

- Yang, J. et al. (2024). SWE-agent: Agent-Computer Interfaces Enable Automated Software Engineering. _NeurIPS_, 2024.

- Gauthier, P. (2024). Aider: AI Pair Programming in Your Terminal. aider.chat.

---

## Multi-Agent Orchestration

- Schluntz & Zhang (2024). Building Effective Agents. anthropic.com.

---

## Model Routing

- Chen, L., Zaharia, M., & Zou, J. (2023). FrugalGPT: How to Use Large Language Models While Reducing Cost and Improving Performance. arXiv:2305.05176.
  *Grounds: 16 T0 probes — cascade architectures can achieve up to 98% cost reduction while matching top-model quality. The key is intelligent routing. Grounds Roko's T0/T1/T2 cascade.*

- Ong et al. (2024). RouteLLM: Learning to Route LLMs with Preference Data. arXiv:2406.18665.


---

## Cognitive Architecture Integration

- Sumers, T.R., Yao, S., Narasimhan, K., & Griffiths, T.L. (2023). Cognitive Architectures for Language Agents (CoALA). arXiv:2309.02427.
  *Grounds: Roko's universal loop — CoALA's decision cycle (planning, then execution, §4.6) is the framework Roko's loop maps to; the 9-step list is Roko's own. Cross-referenced in [20-cognitive-architectures.md](./20-cognitive-architectures.md).*

- Park, J.S. et al. (2023). Generative Agents: Interactive Simulacra of Human Behavior. _UIST_, 2023. arXiv:2304.03442.

---

## Evaluation and Benchmarks

- Jimenez, C.E. et al. (2024). SWE-bench: Can Language Models Resolve Real-World GitHub Issues? _ICLR_, 2024.

- Shahul Es, S. et al. (2024). RAGAS: Automated Evaluation of Retrieval Augmented Generation. _EACL_, 2024.

- Saad-Falcon, J. et al. (2024). ARES: An Automated Evaluation Framework for Retrieval-Augmented Generation Systems. _NAACL_, 2024.

- Liu, X. et al. (2024). AgentBench: Evaluating LLMs as Agents. _ICLR_, 2024.

---

## Cross-References

- See [06-self-learning-systems.md](./06-self-learning-systems.md) for self-improvement mechanisms
- See [07-context-engineering.md](./07-context-engineering.md) for context assembly optimization
- See [20-cognitive-architectures.md](./20-cognitive-architectures.md) for CoALA and cognitive frameworks
