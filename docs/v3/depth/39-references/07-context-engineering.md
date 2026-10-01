# 39-07 Context Engineering -- Annotated Reference Map

> Annotations are kept only for works whose descriptions were checked against the papers (listed in
> `tools/docs_integrity/content_audit.json` and in the banner of docs/v3/REFERENCES.md). The other entries are
> bare citations: their annotations were removed because they had not been checked against the papers.

> Research foundations for context assembly, prompt optimization, RAG, and
> attention management in Roko's Composer and context pipeline.
>
> **v3 depth file** -- updated 2026-09-15. Added PEEK, VISTA, ECS, Scroll, ACE.

---

## Agentic Context Engineering

**[Zhang et al., 2026]** *ACE: Agentic Context Engineering.* ICLR 2026. arXiv:2510.04618.
Generator-Reflector-Curator cycle; +10.6% on AppWorld. Three-role architecture maps to Roko's compose-verify-persist loop.

**[Vijayvargiya & Lokesh, 2025]** *Efficient On-Device Agents via Adaptive Context Management.* arXiv:2511.03728.

**[Kang et al., 2025]** *ACON: Optimizing Context Compression for Long-horizon LLM Agents.* arXiv:2510.00615.

**[Lindenbauer et al., 2025]** *The Complexity Trap: Simple Observation Masking Is as Efficient as LLM Summarization for Agent Context Management.* NeurIPS 2025.

---

## Context Attribution

**[Cohen-Wang et al., 2024]** *ContextCite: Attributing Model Generation to Context.* arXiv:2409.00729.

---

## Retrieval-Augmented Generation

**[Lewis et al., 2020]** *Retrieval-Augmented Generation for Knowledge-Intensive NLP Tasks.* NeurIPS 2020. arXiv:2005.11401.

**[Sarthi et al., 2024]** *RAPTOR: Recursive Abstractive Processing for Tree-Organized Retrieval.* ICLR 2024.

**[Gutierrez et al., 2024]** *HippoRAG: Neurobiologically-Inspired Long-Term Memory.* arXiv:2405.14831.

---

## Context Window Behavior

**[Liu et al., 2024]** *Lost in the Middle.* TACL 2024. arXiv:2307.03172.
U-shaped attention: models attend most to beginning and end. Grounds position-based prompt assembly.

**[Du et al., 2025]** *Context Length Alone Hurts LLM Performance Despite Perfect Retrieval.* EMNLP 2025.

**[Shi et al., 2023]** *Large Language Models Can Be Easily Distracted by Irrelevant Context.* ICML 2023.

**[Joren et al., 2025]** *Sufficient Context.* ICLR 2025.

---

## Prompt Engineering

**[Anthropic, 2025]** *Context Engineering for Agents.* anthropic.com.

**[Karpathy, 2026]** *autoresearch.* GitHub.

**[Wei et al., 2022]** *Chain-of-Thought Prompting.* NeurIPS 2022. arXiv:2201.11903.

---

## Prompt Compression

**[Pan et al., 2024]** *LLMLingua-2.* ACL 2024.

**[Factory.ai, 2026]** *Evaluating Context Compression for Long-Context LLM Applications.* 2026.

---

## 2026 Additions: PEEK, VISTA, ECS, Scroll, ACE extensions

**[Gu et al., 2026]** *PEEK: Context Map as an Orientation Cache for Long-Context LLM Agents.* arXiv:2605.19932.
Constant-token orientation cache with Distiller/Cartographer/Evictor. 93--145 fewer iterations, 1.7--5.8x lower cost. Target for 10th SystemPromptBuilder layer.

**[VISTA, 2026]** *LLM Agents Are Latent Context Managers: Eliciting Self-Managed Context via State Proprioception.* arXiv:2606.30005.

**[Kim, 2026]** *ECS: Entropic Context Shaping.* arXiv:2601.11585.
Entropy-based context shaping provides information-theoretic context management.

**[Lin et al., 2026]** *Scroll: Context as Executable Environment.* arXiv:2608.21690.

**[Ren et al., 2026]** *A Self-Evolving Framework for Efficient Terminal Agents via Observational Context Compression.* arXiv:2604.19572.
Self-evolving compression for terminal output. Informs gate output compression.

**[Mason, 2026]** *Arbiter: Detecting Interference in LLM Agent System Prompts.* arXiv:2603.08993.

**[Li et al., 2025]** *Prompt Compression for Large Language Models: A Survey.* arXiv:2410.12388.

**[Xu et al., 2025]** *Chain of Draft.* arXiv:2502.18600.

**[Xu et al., 2024]** *RECOMP.* ICLR 2024. arXiv:2310.04408.

**[Murthy et al., 2025]** *Promptomatix: An Automatic Prompt Optimization Framework for Large Language Models.* arXiv:2507.14241.

---

## Cross-References

- Self-learning: [06-self-learning-systems](./06-self-learning-systems.md)
- VCG attention auction: [21-mechanism-design](./21-mechanism-design.md)
- Agent harnesses: [14-agent-harnesses-and-tool-use](./14-agent-harnesses-and-tool-use.md)
