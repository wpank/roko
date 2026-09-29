# Distributed Context Engineering

> **Depth file for [06-COMPOSITION.md](../../06-COMPOSITION.md)**
> Source: `crates/roko-compose/` -- context assembly and isolation patterns
> v1 source: `docs/v1/03-composition/11-distributed-context-engineering.md`

---

## Overview

Distributed context engineering extends scaffold design beyond single-agent
prompt assembly to multi-agent systems where context must be managed across
parallel agents, shared knowledge stores, and coordinated execution plans.
The four fundamental strategies -- Write, Select, Compress, Isolate -- form a
complete basis for context management at any scale. Andrej Karpathy (2025)
articulated the key reframing: the real skill in building LLM applications
is not prompt engineering (phrasing instructions well) but context engineering
(managing the entire information environment the model sees).

The **ACE** framework (arXiv:2510.04618, Anthropic) formalized autonomous
context engineering as the baseline for agents that maintain their own working
memory. Roko's composition system extends ACE with the VCG auction for
multi-subsystem allocation and the PEEK orientation cache for session-persistent
context.

---

## 1. The Four Strategies

### 1.1 Write

**Definition:** Generating context that does not yet exist. Creating new
information to inject into the prompt.

**In Roko:**
- The enrichment pipeline WRITES 13 artifact types (briefs, decompositions,
  research memos) using the cheapest appropriate model per step
- The Strategist role WRITES plans and task breakdowns
- The knowledge store WRITES by distilling episodes into insights and heuristics
- The SystemPromptBuilder WRITES affect guidance from PAD state
- The Cartographer (PEEK 10th layer) WRITES structured orientation maps

Write is the most expensive strategy -- it requires an LLM call to generate new
content. The enrichment pipeline's model selection (Haiku for mechanical tasks,
Opus for research) is a cost optimization. The general principle: replace every
LLM call you can with a deterministic operation, and spend your LLM budget on
work that only language models can do.

### 1.2 Select

**Definition:** Choosing which existing information to include. Filtering from
a large candidate set to a small, high-value subset.

**In Roko:**
- Stage 2 (Score) of the 5-stage pipeline SELECTS candidates by composite score
- The ContextTier system SELECTS the appropriate amount of context per model
- The role template system SELECTS which sections each role receives
- The MVT stopping rule SELECTS when to stop searching for more candidates
- The VCG auction SELECTS the combination maximizing total value under budget

Select is the highest-leverage strategy. The empirical evidence is unambiguous:
including the wrong 1,000 tokens is worse than including no context at all
(Joren et al., ICLR 2025, "Sufficient Context"). Selection must be aggressive.

### 1.3 Compress

**Definition:** Reducing the size of existing information while preserving its
semantic content.

**In Roko:**
- The ContextAssembler's compress() method COMPRESSES lower-ranked chunks to
  short summaries
- History compaction COMPRESSES old conversation turns to summaries
- The hard_cap mechanism COMPRESSES sections by truncation
- The PromptBudget system COMPRESSES by allocating smaller budgets to
  lower-priority sections
- TACO-style rules (arXiv:2604.19572) COMPRESS gate output to actionable signal

Compression exists on a fidelity spectrum:
- **Lossless:** Reformatting, whitespace removal, deterministic extraction
- **Near-lossless:** LLMLingua-style token pruning (20x, minimal quality drop)
- **Lossy:** Haiku summarization (significant compression, some loss)
- **Extreme:** Gist tokens (Mu et al., NeurIPS 2023) -- entire prompts to
  special tokens

### 1.4 Isolate

**Definition:** Separating context into independent channels that do not
interfere with each other.

**In Roko:**
- Each agent session is ISOLATED -- no shared conversation history
- The cache layer system ISOLATES stable prefix from volatile suffix
- The role template system ISOLATES different roles' context needs
- Git worktrees ISOLATE each agent's filesystem view
- The "write for amnesia" principle enforces cold-start isolation

Isolation prevents context contamination -- where one agent's irrelevant context
pollutes another agent's prompt.

---

## 2. Three Levels of Context Engineering

### 2.1 Level 1: Local Context Engineering

Optimizing the context for a single agent on a single task.

| Technique | Strategy | Example |
|-----------|----------|---------|
| Priority-based section dropping | Select | Drop workspace map for Trivial tasks |
| U-shape placement | Select | Place critical content at prompt edges |
| Cache-aligned prefix ordering | Compress | Stable prefix for KV cache hits |
| Complexity-adaptive budgets | Select | Trivial -> 4K budget, Complex -> 24K |
| Affect-modulated content | Write | Inject urgency guidance from PAD state |

Level 1 is where most scaffold work happens. Roko's PromptComposer,
SystemPromptBuilder, and ContextAssembler all operate at Level 1.

### 2.2 Level 2: Allocation Context Engineering

Optimizing context allocation across multiple agents working on the same plan.

| Technique | Strategy | Example |
|-----------|----------|---------|
| Shared plan context | Isolate | Byte-identical prefix across agents in same plan |
| Role-specific budgets | Select | Implementer gets 8K file_context, Strategist gets 0 |
| Cross-agent iteration memory | Write | Gate errors from Agent A inform Agent B |
| Differential compression | Compress | Architect gets full code, QuickReviewer gets summary |

Level 2 requires orchestration awareness -- the scaffold must know about other
agents and their needs. Roko's `SharedPlanContext` and `RoleSystemPromptSpec`
operate at Level 2.

### 2.3 Level 3: Network Context Engineering

Optimizing context across agent collectives sharing a knowledge mesh.

| Technique | Strategy | Example |
|-----------|----------|---------|
| Stigmergic knowledge accumulation | Write | Agents deposit insights in shared Neuro store |
| Collective calibration | Select | Knowledge entries ranked by cross-agent track record |
| VCG attention auction | Select | Subsystems bid for context bandwidth |
| HDC-based retrieval | Select | Sub-50ns semantic search across collective knowledge |
| Knowledge distillation | Compress | Episodes -> insights -> heuristics -> playbook rules |
| Agent mesh sync | Isolate | Permissioned knowledge sharing across agents |

Level 3 is the target architecture -- a collective of agents that get smarter
over time because every task outcome feeds back into the shared knowledge store.

---

## 3. The Write-for-Amnesia Principle

Every agent session starts cold. No conversation history. No shared memory. No
implicit context. The files on disk are the only truth.

This is an isolation strategy with profound implications:

1. **All context must be explicit.** The agent cannot "remember" what a previous
   agent did. If the information is needed, it must be written to disk and
   injected into the prompt.

2. **Enrichment is pre-computation.** The enrichment pipeline creates artifacts
   BEFORE the agent session starts. The agent reads files, not memories.

3. **Iteration memory is structured.** When a task is retried after gate failure,
   the failure context is explicitly written to disk and injected. The agent
   does not "recall" the failure -- it reads about it.

4. **Cross-agent communication is file-based.** Agent A's output is written to
   disk. Agent B's prompt includes Agent A's output as a file. There is no
   message passing, no shared state, no implicit knowledge transfer.

This makes the system fully inspectable: if an agent produces bad output, you
can read its input files and see exactly what it saw. There is no hidden
context, no conversation history, no mystery.

---

## 4. The Meta-Harness Evaluation

Lee et al. (2026, arXiv:2603.28052) evaluated coding agents across scaffolds:

| Finding | Measurement | Implication |
|---------|-------------|-------------|
| **6x performance gap** from scaffold changes alone | Same model, different scaffolds | Scaffold > model quality |
| **4x fewer input tokens** in the best scaffolds | Token usage comparison | Better context engineering = less input |
| **Scaffold diversity matters** | Performance across task types | No single scaffold dominates all tasks |

The 6x gap validates Roko's core premise: the scaffold IS the product. Investing
in better context engineering produces more improvement than upgrading to a more
expensive model. The 4x token reduction means better scaffolds are also cheaper.

---

## 5. The CLEAR Framework

The CLEAR framework (2025) defines five evaluation dimensions for AI systems:

| CLEAR Dimension | Context Engineering Impact |
|----------------|--------------------------|
| **Cost** | Better selection = fewer tokens = lower API bills |
| **Latency** | Smaller prompts = faster inference |
| **Efficacy** | Better context = higher task success rate |
| **Assurance** | Explicit context = inspectable, auditable |
| **Reliability** | Deterministic assembly = reproducible prompts |

CLEAR's most important finding: optimizing for efficacy alone produces systems
4.4-10.8x more expensive than co-optimizing for cost and efficacy. The four
context engineering strategies naturally co-optimize: Select reduces both cost
and noise, Compress reduces cost while preserving quality, Isolate improves
reliability, Write invests cost where it produces the highest return.

---

## 6. The RAGAS Evaluation Triad

RAGAS (Shahul Es et al., EACL 2024) defines three evaluation dimensions for
retrieval-augmented systems:

- **Faithfulness:** Does the agent's output match the provided context?
- **Answer Relevance:** Does the output address the task?
- **Context Relevance:** Is the retrieved context actually useful?

Most RAG systems optimize only for Answer Relevance and ignore Context
Relevance. Roko explicitly optimizes for Context Relevance through:
- The 5-stage pipeline's deduplication stage (remove redundant context)
- The MVT stopping rule (stop when marginal relevance drops)
- Priority-based dropping (remove low-value sections first)
- Complexity-adaptive budgets (exclude sections irrelevant to simple tasks)
- VCG auction diagnostics (detect when included context is low-value)

---

## 7. Contextual Influence Value

The Contextual Influence Value framework (Shanghai Jiao Tong University, 2025)
provides per-section impact measurement through leave-one-out analysis:

```
For each section S in the context pack:
    influence(S) = pass_rate_with_S - pass_rate_without_S

If influence(S) > 0:  S is valuable -- increase its budget allocation.
If influence(S) ~ 0:  S is neutral -- candidate for compression or dropping.
If influence(S) < 0:  S is harmful -- drop it (it introduces context rot).
```

Three evaluation dimensions per section:
- **Query-aware relevance:** Does the section relate to the task?
- **List-aware uniqueness:** Does the section provide new information not
  covered by other sections?
- **Generator-aware utility:** Does the specific model benefit from this section?

This framework enables targeted pruning -- removing sections that are redundant
or harmful rather than globally reducing context. It maps directly to Roko's
`SectionInfluence` struct in the budget learning system (see
`token-budget-management.md`).

---

## 8. Social Foraging

In multi-agent execution (parallel plan run with 5-20 agents), each agent
independently forages for context. Social foraging leverages the collective
retrieval patterns to improve individual performance.

### 8.1 Stigmergic Retrieval Signals

Inspired by ant pheromone trails, agents deposit retrieval signals after
successful task completion:

```rust
/// A retrieval signal deposited after a successful task.
pub struct RetrievalSignal {
    pub task_category: String,
    pub entry_id: String,
    pub relevance: f64,
    pub gate_passed: bool,
    pub timestamp: Timestamp,
    pub agent_id: String,
}
```

When Agent A queries the knowledge store for a cross-crate integration task and
finds entries X, Y, Z useful (gate pass on first attempt), that information is
valuable for Agent B working on a related task. Agent B's forager uses Agent A's
successful retrievals as "social information scent" -- boosting the score of
entries that were useful to similar agents.

### 8.2 Social Foraging Conditions

Social information is not always beneficial. Research (Mezey et al., PLOS
Computational Biology 2024) identifies when it helps:

| Condition | Social Signal Value |
|-----------|-------------------|
| Sparse, clustered knowledge | **High** -- social signals guide to relevant clusters |
| Uniform knowledge distribution | **Low** -- social signals add noise |
| High agent homogeneity (same role) | **High** -- same role needs same knowledge |
| Early in plan execution | **High** -- first agents scout for later agents |

### 8.3 Field Validation

A striking 2025 result (Science, doi:10.1126/science.ady1055): GPS tracking of
hunter-gatherer foragers demonstrated real-time adaptive social information use
at field scale. Foragers update patch-quality estimates based on others'
movements. This is the first empirical validation of social MVT outside
laboratory settings.

---

## 9. Academic Foundations

**Karpathy, A. (2025).** Articulated the context engineering framework. The
shift from "prompt engineering" to "context engineering" as the key skill.

**ACE** (arXiv:2510.04618). Autonomous Context Engineering. Anthropic's
baseline for agents that maintain their own working memory. Roko's PEEK
orientation cache targets 1.7-5.8x improvement over ACE.

**Lee et al. (2026), "Meta-Harness: Evaluating Coding Agents Across Scaffolds."**
arXiv:2603.28052. The 6x performance gap and 4x token reduction findings.

**Zaharia et al. (2024), "The Shift to Compound AI Systems."** BAIR.
State-of-the-art from composing components, not scaling single models.

**RAGAS** (Shahul Es et al., EACL 2024). Automated evaluation via Faithfulness,
Answer Relevance, Context Relevance.

**ARES** (Saad-Falcon et al., NAACL 2024). Statistical confidence intervals
for RAG evaluation from minimal human labels.

**CLEAR Framework** (2025). Five-dimensional evaluation: Cost, Latency,
Efficacy, Assurance, Reliability. Accuracy-only optimization is 4.4-10.8x
more expensive.

**AI Agents That Matter** (Kapoor et al., Princeton 2025). Minimum evaluation
bar: run each condition at least 5 times, report mean with confidence intervals.

**Contextual Influence Value** (Shanghai Jiao Tong University, 2025).
Leave-one-out per-section impact measurement for targeted context pruning.

**Mezey et al. (2024).** "Visual Social Information Use in Collective Foraging."
PLOS Computational Biology. Social information helps when resources are
heterogeneously distributed.

**Science (2025).** doi:10.1126/science.ady1055. First field-scale validation
of social MVT in hunter-gatherers.

---

## 10. Current Status and Gaps

| Aspect | Status |
|--------|--------|
| Write strategy (enrichment pipeline) | **Implemented** |
| Select strategy (priority dropping, tier budgets, VCG) | **Implemented** |
| Compress strategy (truncation, summary, TACO-style) | **Partially implemented** |
| Isolate strategy (session isolation, cache layers, worktrees) | **Implemented** |
| Level 1 (local) context engineering | **Implemented** |
| Level 2 (allocation) context engineering | **Implemented** |
| Level 3 (network) context engineering | **Partial** (knowledge store wired, social foraging designed) |
| RAGAS-style evaluation | **Not yet** |
| Contextual influence value tracking | **Not yet** |
| Meta-Harness benchmarking | **Not yet** |
| Social foraging signals | **Designed** |

---

## Cross-References

| File | Relationship |
|------|-------------|
| `enrichment-pipeline-13-step.md` | Write strategy implementation |
| `token-budget-management.md` | Select/Compress strategy, influence values |
| `lost-in-the-middle-u-shape.md` | Select strategy (placement optimization) |
| `5-stage-assembly-pipeline.md` | Full pipeline across all strategies |
| `vcg-attention-auction.md` | Level 3 allocation mechanism |
| `affect-modulated-retrieval.md` | Affect modulation of selection |
| `predictive-foraging-mvt.md` | Social foraging as Level 3 extension |
| [06-COMPOSITION.md](../../06-COMPOSITION.md) | Parent chapter |
