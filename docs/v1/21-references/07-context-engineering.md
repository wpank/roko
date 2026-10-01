# Context Engineering

> Annotations are kept only for works whose descriptions were checked against the papers (listed in
> `tools/docs_integrity/content_audit.json` and in the banner of docs/v3/REFERENCES.md). The other entries are
> bare citations: their annotations were removed because they had not been checked against the papers.

> **DEPRECATED (v1):** This document is part of the v1 specification and may be outdated. See [../../v2/](../../v2/) for the current reference.


> Academic foundations for context assembly, prompt optimization, retrieval-augmented generation, and attention management in Roko's Composer and context pipeline.

**Topic**: [References](./INDEX.md)
**Prerequisites**: [Architecture](../00-architecture/INDEX.md), [Scaffold](../01-orchestration/INDEX.md)
**Key sources**: `bardo-backup/prd/02-mortality/14-research-foundations.md` §8, `bardo-backup/prd/shared/citations.md` §27, `bardo-backup/tmp/mori-agents/12-references.md`

> **Implementation**: Reference

---

## Abstract

Context failures, not model failures, cause most agent breakdowns. For agents running long tasks, context assembly is the highest-leverage cognitive system. The research here establishes that effective context management requires active curation, not passive accumulation — and that the same resource pressure that shapes agent behavior also shapes what enters the context window. The 6x context reduction achievable through proper context engineering (CSO) directly reduces compute costs.

---

## Agentic Context Engineering

- Zhang, Q. et al. (2026). ACE: Agentic Context Engineering. _ICLR_, 2026. arXiv:2510.04618.
  *Grounds: Generator-Reflector-Curator cycle — treats context as an evolving playbook; +10.6% on AppWorld. Context assembly self-improves via cybernetic feedback. The three-role architecture (Generator creates, Reflector critiques, Curator compresses) maps directly to Roko's compose-verify-persist loop.*

- Vijayvargiya & Lokesh (2025). Efficient On-Device Agents via Adaptive Context Management. arXiv:2511.03728.

- Kang, S. et al. (2025). ACON: Optimizing Context Compression for Long-horizon LLM Agents. arXiv:2510.00615.

- Lindenbauer, T. et al. (2025). The Complexity Trap: Simple Observation Masking Is as Efficient as LLM Summarization for Agent Context Management. _NeurIPS_, 2025.

---

## Context Attribution

- Cohen-Wang, B., Shah, H., Georgiev, B., & Madry, A. (2024). ContextCite: Attributing Model Generation to Context. arXiv:2409.00729.

---

## Retrieval-Augmented Generation

- Lewis, P. et al. (2020). Retrieval-Augmented Generation for Knowledge-Intensive NLP Tasks. _NeurIPS_, 2020. arXiv:2005.11401.

- Sarthi, P. et al. (2024). RAPTOR: Recursive Abstractive Processing for Tree-Organized Retrieval. _ICLR_, 2024.

- Gutierrez et al. (2024). HippoRAG: Neurobiologically-Inspired Long-Term Memory for LLMs. arXiv:2405.14831.

---

## Context Window Behavior

- Liu, N.F. et al. (2024). Lost in the Middle: How Language Models Use Long Contexts. _TACL_, 2024. arXiv:2307.03172.
  *Grounds: Context position strategy — U-shaped performance curve: models use information at the beginning and end of the context best and degrade in the middle (§2.3). Directly motivates context assembly: highest-priority content at the beginning, second-highest at the end.*

- Du, Y. et al. (2025). Context Length Alone Hurts LLM Performance Despite Perfect Retrieval 13.9-85%. _EMNLP_, 2025.

- Shi, F. et al. (2023). Large Language Models Can Be Easily Distracted by Irrelevant Context. _ICML_, 2023.

- Joren, T. et al. (2025). Sufficient Context: A New Lens on Retrieval Augmented Generation Systems. _ICLR_, 2025.

---

## Prompt Engineering Foundations

- Anthropic (2025). Context Engineering for Agents. anthropic.com.

- Karpathy, A. (2026). autoresearch. GitHub.

- Wei, J. et al. (2022). Chain-of-Thought Prompting Elicits Reasoning in Large Language Models. _NeurIPS_, 2022. arXiv:2201.11903.

---

## Prompt Compression

- Pan, Z. et al. (2024). LLMLingua-2: Data Distillation for Efficient and Faithful Task-Agnostic Prompt Compression. _ACL_, 2024.

- Factory.ai (2026). Evaluating Context Compression for Long-Context LLM Applications. 2026.

---

## Cross-References

- See [06-self-learning-systems.md](./06-self-learning-systems.md) for ACE in the learning context
- See [14-agent-harnesses-and-tool-use.md](./14-agent-harnesses-and-tool-use.md) for Meta-Harness
- See [21-mechanism-design.md](./21-mechanism-design.md) for VCG attention auction
- See topic [02-scaffold](../01-orchestration/INDEX.md) for full Scaffold layer design
