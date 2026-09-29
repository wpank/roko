# Composer Trait

> **Depth file for [06-COMPOSITION.md](../../06-COMPOSITION.md)**
> Source: `crates/roko-core/src/traits.rs` -- the `Compose` trait
> v1 source: `docs/v1/03-composition/00-composer-trait.md`

---

## Overview

The Composer trait defines the contract for assembling scored, budgeted context
into a single coherent prompt Signal. It is one of the 12 kernel traits in the
Synapse Architecture. Its distinguishing design decision is accepting a `Scorer`
reference at call time, making scoring an input to composition rather than a
separate upstream phase.

---

## 1. Trait Signature

```rust
// crates/roko-core/src/traits.rs

pub trait Compose: Send + Sync {
    fn compose(
        &self,
        signals: &[Signal],
        budget: &Budget,
        scorer: &dyn Score,
        ctx: &Context,
    ) -> Result<Signal>;
}
```

| Parameter | Type | Purpose |
|-----------|------|---------|
| `signals` | `&[Signal]` | Candidate context units to assemble |
| `budget` | `&Budget` | Hard constraints on output size |
| `scorer` | `&dyn Score` | Scoring function for ranking candidates |
| `ctx` | `&Context` | Ambient context (agent state, task metadata) |

**Returns:** A single `Signal` -- the assembled prompt, ready for LLM
consumption.

The trait is `Send + Sync`, allowing composers to be shared across threads in
parallel plan execution. It is synchronous -- composition is a CPU-bound
operation that must never perform I/O. Composers do not read files, query
databases, or call LLMs. They receive pre-gathered candidates and assemble them
under budget constraints.

---

## 2. The Signal: Content-Addressed Unit of Cognition

Every input and output of the Composer is a `Signal` (the primary type name;
`Engram` is a backward-compat alias via `pub type Engram = Signal`). A Signal is
a content-addressed, scored, decaying, lineage-tracked unit of cognition:

```rust
pub struct Engram {
    pub id: EngramId,           // Content-addressed hash (Blake3)
    pub body: Body,             // Payload (text, structured data, binary)
    pub score: Score,           // 7-axis quality assessment
    pub lineage: Lineage,       // DAG of parent engrams
    pub created_at: Timestamp,
    pub ttl: Option<Duration>,  // Time-to-live for decay
    pub tags: Vec<Tag>,         // Semantic labels
}
```

The 7-axis Score captures multiple quality dimensions:

```rust
pub struct Score {
    pub confidence: f64,    // [0,1] -- how certain is this information?
    pub novelty: f64,       // [0,1] -- how new/surprising is this?
    pub utility: f64,       // [0,1] -- how useful for the current task?
    pub reputation: f64,    // [0,1] -- trust in the source
    pub salience: f64,      // [0,1] -- how attention-worthy?
    pub coherence: f64,     // [0,1] -- internal consistency
    pub relevance: f64,     // [0,1] -- match to current query
}
```

The Composer receives a slice of scored Signals and produces a single output
Signal whose body contains the assembled prompt. The output Signal's lineage
field records which input Signals were included, providing full provenance for
every prompt.

---

## 3. The Budget Struct

```rust
pub struct Budget {
    pub max_tokens: usize,
    pub max_signals: usize,
    pub max_bytes: usize,
}
```

| Field | Purpose | Typical values |
|-------|---------|---------------|
| `max_tokens` | Hard cap on estimated token count of output | 4,000 -- 24,000 |
| `max_signals` | Maximum number of signals to include | 10 -- 50 |
| `max_bytes` | Byte-level cap (for binary payloads) | 100KB -- 1MB |

The three constraints work as a conjunction: all must be satisfied. The tightest
constraint wins. For text prompts, `max_tokens` is typically the binding
constraint. Token estimation uses the heuristic of approximately 4 bytes per
token (empirically calibrated across Anthropic and OpenAI tokenizers for English
text and source code).

### Budget Derivation

Budgets are derived from the context tier and model context window:

| Context Tier | Token Budget | Use Case |
|-------------|-------------|----------|
| **Surgical** | ~4,000 | Haiku, Ollama, Gemma -- mechanical tasks |
| **Focused** | ~12,000 | Sonnet -- focused/integrative tasks |
| **Full** | ~24,000 | Opus -- architectural tasks |

The context tier is determined by `ContextTier::from_task_and_model()`, which
maps the task complexity band and model backend to the appropriate tier. Local
models (Ollama, Gemma, Llama, DeepSeek, Phi, StarCoder) always receive Surgical
tier regardless of task complexity, because they cannot reliably handle large
contexts or tools.

---

## 4. Why the Composer Takes a Scorer

The Composer trait's most distinctive design choice is accepting `&dyn Score` as
a parameter rather than consuming pre-scored signals. Three motivations:

### 4.1 Re-scoring During Assembly

Static pre-scoring assumes relevance is context-independent. It is not. A
signal's value depends on what else is in the prompt. If two signals contain
overlapping information, including both wastes budget. If one signal provides
definitions that another references, ordering matters. The Composer can re-score
signals during assembly to account for marginal value decreasing as similar
content is already included.

### 4.2 Scorer as Strategy

Different scoring strategies produce different compositions from the same
candidates. A priority-based scorer produces deterministic, predictable prompts.
An active-inference scorer produces adaptive prompts that explore when uncertain
and exploit when confident. By accepting the scorer as a parameter, the Composer
is decoupled from any specific scoring strategy. The caller chooses; the Composer
applies.

### 4.3 Testability

Accepting a scorer as a parameter makes composition fully testable. Unit tests
inject mock scorers returning predetermined values, verifying budget fitting,
priority dropping, and U-shape placement without real scoring infrastructure.

---

## 5. The Primary Implementation: PromptComposer

The `PromptComposer` in `crates/roko-compose/src/prompt.rs` is the primary
Composer implementation:

```rust
impl Compose for PromptComposer {
    fn compose(
        &self,
        signals: &[Signal],
        budget: &Budget,
        scorer: &dyn Score,
        ctx: &Context,
    ) -> Result<Signal> {
        // 1. Decode signals into PromptSections
        // 2. Score each section (VCG auto-select or manual bidders)
        // 3. Partition into Critical and Optional
        // 4. Sort by cache_layer ASC, priority DESC
        // 5. Greedy include under budget (Critical never dropped)
        // 6. Order by Placement (Start/Middle/End) for U-shape
        // 7. Concatenate with section headers and cache markers
        // 8. Return assembled prompt as a Signal
    }
}
```

The PromptComposer supports 9 `AttentionBidder` variants for VCG-based context
allocation, `LearningBidder` with Thompson sampling, HDC deduplication, and
`MultiPatchForager` for MVT-based retrieval. See
[prompt-composer.md](prompt-composer.md) for the full specification.

---

## 6. Composition in the Cognitive Loop

The Composer operates at a specific point in the universal cognitive loop:

```
PERCEIVE (Substrate.query)
    -> REMEMBER (Score.score)
        -> ATTEND (Route.select)
            -> **COMPOSE** (Compose.compose)  <- here
                -> ACT (Agent.execute)
                    -> VERIFY (Gate.verify)
                        -> ADAPT (React.decide)
```

The Composer receives the output of the Router (which selected which signals to
include) and the Scorer (which ranked them). It assembles these into the final
prompt that the Agent executes against.

In the production wiring, composition happens via
`RoleSystemPromptSpec::compose_with_budget()`, which builds the 9-layer system
prompt, applies role-specific budgets, and outputs the assembled prompt string.
The PromptComposer is invoked within this pipeline for final budget-fitting and
ordering.

---

## 7. Design Constraints

The Composer operates under several constraints:

1. **Synchronous only.** Composition must not perform I/O. All candidates are
   pre-gathered.
2. **Deterministic.** The same inputs must produce the same output. This is
   critical for prompt cache alignment -- if composition is non-deterministic,
   prefix caching fails.
3. **Budget-respecting.** The output must satisfy all Budget constraints. No
   exceptions.
4. **Critical sections survive.** Sections marked as Critical priority are never
   dropped, only truncated. This ensures safety instructions, role identity, and
   task description always appear.
5. **Lineage-preserving.** The output Signal's lineage must record which inputs
   were included, enabling provenance tracking and credit assignment.
6. **Placement-aware.** The Composer must respect Placement hints
   (Start/Middle/End) to implement U-shape attention optimization (Liu et al.
   2023).

---

## 8. Relationship to Other Traits

| Trait | Relationship to Composer |
|-------|-------------------------|
| **Substrate** | Provides raw signals from storage/sensors |
| **Score** | Ranks signals; passed as parameter to Composer |
| **Gate** | Validates composition output (does the prompt meet quality thresholds?) |
| **Route** | Selects which signals to include; upstream of Composer |
| **React** | Decides when to recompose (e.g., after gate failure) |
| **Bus** | Publishes composition events for telemetry |
| **Observe** | Records composition metrics through the Lens runtime |

The Composer is the convergence point: it receives output from Substrate
(candidates), Score (rankings), and Route (selection), and produces the input
for the Agent (assembled prompt). It is the most downstream trait before
execution.

---

## 9. Academic Foundations

**Compound AI Systems** [Zaharia et al., BAIR 2024]. The Composer embodies the
compound AI principle: state-of-the-art results come from composing multiple
components, not from single model calls. The 12-trait architecture is a compound
system where each trait is a composable module.

**CoALA: Cognitive Architectures for Language Agents** [Sumers et al. 2023].
Provides the theoretical framework: cognitive agents have a universal structure
(perception, memory, reasoning, action, reflection) with modular memory
components. The Composer maps to CoALA's "working memory assembly" phase --
constructing the agent's active context from long-term and episodic memory.

**DSPy: Programmatic Prompt Optimization** [Khattab et al. 2023]. Reframed
prompting as programming: define modules with typed signatures, compose them
into pipelines, and let a compiler optimize against a metric. The Compose
trait's typed signature (`signals x budget x scorer x ctx -> signal`) is
DSPy-compatible.

**Modular RAG** [Gao et al. 2023]. The evolution from Naive RAG
(retrieve-then-read) through Advanced RAG (query rewriting, re-ranking) to
Modular RAG (composable retrieval/generation/augmentation modules). The Composer
is the "augmentation" module in Modular RAG.

---

## 10. Implementation Status

| Aspect | Status |
|--------|--------|
| Trait definition in `roko-core` | **Shipped** |
| `PromptComposer` implementation | **Shipped** (18+ tests) |
| `SectionScorer` implementation | **Shipped** (6 tests) |
| Budget types | **Shipped** |
| ContextTier derivation | **Shipped** |
| VCG allocation with LearningBidder | **Shipped** |
| U-shape placement | **Shipped** |
| Active inference EFE scoring | **Designed, not wired** |
| Lineage tracking in output Signal | **Partial** |

---

## Cross-References

- [prompt-composer.md](prompt-composer.md) -- PromptComposer implementation
- [token-budget-management.md](token-budget-management.md) -- Budget derivation and allocation
- [lost-in-the-middle-u-shape.md](lost-in-the-middle-u-shape.md) -- U-shape attention
- [active-inference-context-selection.md](active-inference-context-selection.md) -- EFE scoring
- [5-stage-assembly-pipeline.md](5-stage-assembly-pipeline.md) -- Full assembly pipeline
- [vcg-attention-auction.md](vcg-attention-auction.md) -- VCG mechanism
- `crates/roko-compose/src/prompt.rs` -- PromptComposer source
- `crates/roko-core/src/traits.rs` -- Compose trait definition
