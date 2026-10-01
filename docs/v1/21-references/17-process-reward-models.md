# Process Reward Models and Verification

> Annotations are kept only for works whose descriptions were checked against the papers (listed in
> `tools/docs_integrity/content_audit.json` and in the banner of docs/v3/REFERENCES.md). The other entries are
> bare citations: their annotations were removed because they had not been checked against the papers.

> **DEPRECATED (v1):** This document is part of the v1 specification and may be outdated. See [../../v2/](../../v2/) for the current reference.


> Academic foundations for step-level verification, generation-verification gaps, and process supervision in Roko's Gate pipeline.

**Topic**: [References](./INDEX.md)
**Prerequisites**: [Architecture](../00-architecture/INDEX.md), [Harness](../04-verification/INDEX.md)
**Key sources**: `bardo-backup/tmp/mori-agents/12-references.md`

> **Implementation**: Reference

---

## Abstract

Process Reward Models (PRMs) verify each reasoning step rather than only the final outcome. The key finding (Song et al. 2025) is that self-improvement works only when verification ability exceeds generation ability. Roko's Gate pipeline implements this principle: external verifiers (compiler, test suite, linters) are stronger than the LLM at determining correctness, so their verdicts drive learning. This section also covers agent-specific PRMs and retrieval-based RL.

---

## Step-Level Verification

- Lightman, H. et al. (2024). Let's Verify Step by Step. arXiv:2305.20050.
  *Grounds: Process reward models — step-level verification outperforms outcome-only verification for mathematical reasoning. Grounds the Gate pipeline's per-step verification: each gate checks a specific quality dimension (compilation, tests, linting, diff review) rather than a single holistic pass/fail.*

---

## Generation-Verification Gap

- Song, Y. et al. (2025). Mind the Gap: Examining the Self-Improvement Capabilities of Large Language Models. _ICLR_, 2025.

---

## Self-Correction Limits

- Huang, J. et al. (2024). Large Language Models Cannot Self-Correct Reasoning Yet. _ICLR_, 2024.

- Pan, A. et al. (2024). Spontaneous Reward Hacking in Iterative Self-Refinement. _ICML_, 2024.

---

## Agent Process Reward Models

- Agrawal, A. et al. (2026). GEPA: Reflective Prompt Evolution. _ICLR (Oral)_, 2026. arXiv:2507.19457.

---

## RL for Retrieval

- Jin, B. et al. (2025). Search-R1: Training LLMs to Reason and Leverage Search Engines with Reinforcement Learning. 2025.

- Xiong, W. et al. (2025). Supervising the search process produces reliable and generalizable information-seeking agents. 2025.

---

## Chain-of-Thought Verification

- Wei, J. et al. (2022). Chain-of-Thought Prompting Elicits Reasoning in Large Language Models. _NeurIPS_, 2022.

- Wang, X. et al. (2023). Self-Consistency Improves Chain of Thought Reasoning in Language Models. _ICLR_, 2023.

---

## Cross-References

- See [06-self-learning-systems.md](./06-self-learning-systems.md) for the full self-improvement context
- See [14-agent-harnesses-and-tool-use.md](./14-agent-harnesses-and-tool-use.md) for evaluation benchmarks
- See topic [03-harness](../04-verification/INDEX.md) for full Gate pipeline design
