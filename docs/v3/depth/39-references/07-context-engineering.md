# 39-07 Context Engineering -- Annotated Reference Map

> Research foundations for context assembly, prompt optimization, RAG, and
> attention management in Roko's Composer and context pipeline.
>
> **v3 depth file** -- updated 2026-09-15. Added PEEK, VISTA, ECS, Scroll, ACE.

---

## Agentic Context Engineering

**[Zhang et al., 2026]** *ACE: Agentic Context Engineering.* ICLR 2026. arXiv:2510.04618.
Generator-Reflector-Curator cycle; +10.6% on AppWorld. Three-role architecture maps to Roko's compose-verify-persist loop.

**[Vijayvargiya & Lokesh, 2025]** *Efficient On-Device Agents via Adaptive Context Management.* arXiv:2511.03728.
Structured compression: 6x initial reduction, 10--25x growth rate reduction. Grounds structured context in `roko-compose`.

**[Kang et al., 2025]** *ACON: Optimizing Context Compression for Long-horizon LLM Agents.* arXiv:2510.00615.
Failure-driven compression: 26--54% peak token reduction. Learns which compressions preserve task-relevant information.

**[Lindenbauer et al., 2025]** *The Complexity Trap: Simple Observation Masking Is as Efficient as LLM Summarization for Agent Context Management.* NeurIPS 2025.
T0 suppression: halves cost while matching summarization quality. Validates the T0 probe approach.

---

## Context Attribution

**[Cohen-Wang et al., 2024]** *ContextCite: Attributing Model Generation to Context.* arXiv:2409.00729.
Sparse linear model with 64 ablation passes. Context pruned by measured attribution, not heuristic importance.

---

## Retrieval-Augmented Generation

**[Lewis et al., 2020]** *Retrieval-Augmented Generation for Knowledge-Intensive NLP Tasks.* NeurIPS 2020. arXiv:2005.11401.
RAG architecture foundation. Roko's per-tick context assembly is structural RAG.

**[Sarthi et al., 2024]** *RAPTOR: Recursive Abstractive Processing for Tree-Organized Retrieval.* ICLR 2024.
Hierarchical retrieval via recursive summarization. Informs multi-tier knowledge retrieval.

**[Gutierrez et al., 2024]** *HippoRAG: Neurobiologically-Inspired Long-Term Memory.* arXiv:2405.14831.
Hippocampal-inspired pattern separation and completion. Informs NeuroStore retrieval.

---

## Context Window Behavior

**[Liu et al., 2024]** *Lost in the Middle.* TACL 2024. arXiv:2307.03172.
U-shaped attention: models attend most to beginning and end. Grounds position-based prompt assembly.

**[Du et al., 2025]** *Context Length Alone Hurts LLM Performance Despite Perfect Retrieval.* EMNLP 2025.
13.9--85% degradation from unnecessary context. Mandates aggressive compression.

**[Shi et al., 2023]** *Large Language Models Can Be Easily Distracted by Irrelevant Context.* ICML 2023.
Irrelevant context actively degrades performance. Quality filtering is required, not optional.

**[Joren et al., 2025]** *Sufficient Context.* ICLR 2025.
Insufficient context makes models 6x worse than no context (10.2% to 66.1% incorrect). Motivates sufficiency checking.

---

## Prompt Engineering

**[Anthropic, 2025]** *Context Engineering for Agents.* anthropic.com.
Two-layer context: pre-loaded static + just-in-time retrieval. Roko: `roko.toml` static + per-tick RAG.

**[Karpathy, 2026]** *autoresearch.* GitHub.
Automated research context assembly. Informs the research agent.

**[Wei et al., 2022]** *Chain-of-Thought Prompting.* NeurIPS 2022. arXiv:2201.11903.
CoT as reasoning scaffold. StrategyFragment knowledge entries serve as CoT exemplars.

---

## Prompt Compression

**[Pan et al., 2024]** *LLMLingua-2.* ACL 2024.
Task-agnostic compression via data distillation. Applicable to context budget management.

**[Factory.ai, 2026]** *Evaluating Context Compression for Long-Context LLM Applications.* 2026.
Compression evaluation methodology. Informs quality metrics.

---

## 2026 Additions: PEEK, VISTA, ECS, Scroll, ACE extensions

**[Gu et al., 2026]** *PEEK: Context Map as an Orientation Cache for Long-Context LLM Agents.* arXiv:2605.19932.
Constant-token orientation cache with Distiller/Cartographer/Evictor. 93--145 fewer iterations, 1.7--5.8x lower cost. Target for 10th SystemPromptBuilder layer.

**[VISTA, 2026]** *LLM Agents Are Latent Context Managers: Eliciting Self-Managed Context via State Proprioception.* arXiv:2606.30005.
Proprioceptive context dashboard for agents. Informs self-awareness context injection alongside Named Surfaces.

**[Kim, 2026]** *ECS: Entropic Context Shaping.* arXiv:2601.11585.
Entropy-based context shaping provides information-theoretic context management.

**[Lin et al., 2026]** *Scroll: Context as Executable Environment.* arXiv:2608.21690.
Context window as executable environment. Frontier design for context management.

**[Ren et al., 2026]** *A Self-Evolving Framework for Efficient Terminal Agents via Observational Context Compression.* arXiv:2604.19572.
Self-evolving compression for terminal output. Informs gate output compression.

**[Mason, 2026]** *Arbiter: Detecting Interference in LLM Agent System Prompts.* arXiv:2603.08993.
Detecting interference between system prompt components. Informs 9-layer prompt quality assurance.

**[Li et al., 2025]** *Prompt Compression for Large Language Models: A Survey.* arXiv:2410.12388.
Comprehensive survey of prompt compression techniques.

**[Xu et al., 2025]** *Chain of Draft.* arXiv:2502.18600.
5-word intermediate reasoning steps. Ultra-compact reasoning format.

**[Xu et al., 2024]** *RECOMP.* ICLR 2024. arXiv:2310.04408.
Compressing retrieved documents before generation.

**[Murthy et al., 2025]** *Promptomatix: An Automatic Prompt Optimization Framework for Large Language Models.* arXiv:2507.14241.
Prompt optimization framework for structured prompts.

---

## Cross-References

- Self-learning: [06-self-learning-systems](./06-self-learning-systems.md)
- VCG attention auction: [21-mechanism-design](./21-mechanism-design.md)
- Agent harnesses: [14-agent-harnesses-and-tool-use](./14-agent-harnesses-and-tool-use.md)
