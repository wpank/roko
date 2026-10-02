# PromptComposer: Priority Dropping and U-Shape Placement

> **Depth file for [06-COMPOSITION.md](../../06-COMPOSITION.md)**
> Source: `crates/roko-compose/src/prompt.rs`
> v1 source: `docs/v1/03-composition/01-prompt-composer.md`

---

## Overview

PromptComposer is the primary implementation of the Compose trait. It transforms
a collection of typed, prioritized prompt sections into a single budget-fitted,
cache-aligned prompt string. The core algorithm is a greedy knapsack with
priority partitioning: Critical sections are never dropped, optional sections
are included in priority order until the budget is exhausted, and the final
output is ordered by Placement hints to implement the U-shaped attention
optimization from Liu et al. (2023).

---

## 1. The PromptSection Data Model

Every piece of context that enters the Composer is wrapped in a `PromptSection`:

```rust
// crates/roko-compose/src/prompt.rs

pub struct PromptSection {
    pub name: String,
    pub content: String,
    pub priority: SectionPriority,
    pub cache_layer: CacheLayer,
    pub placement: Placement,
    pub hard_cap: Option<usize>,
    pub bidder: Option<AttentionBidder>,
    pub section_id: Option<String>,
    pub source: Option<AttentionBidder>,
    pub provenance: Option<String>,
    pub experiment_id: Option<String>,
}
```

### 1.1 SectionPriority

```rust
pub enum SectionPriority {
    Low = 1,      // Drop first when budget is tight
    Normal = 2,   // Standard -- included unless budget exhausted
    High = 3,     // Important -- included before Normal
    Critical = 4, // Never dropped, only truncated
}
```

Critical sections are the invariant core: role identity, safety constraints,
task description. They are never dropped, only truncated if necessary. This
guarantees the agent always knows what it is, what it should do, and what it
must not do.

### 1.2 CacheLayer

```rust
pub enum CacheLayer {
    System = 0,   // Identical across all tasks for this role
    Session = 1,  // Stable within a plan execution
    Task = 2,     // Stable within a single task's iterations
    Dynamic = 3,  // Unique per request
}
```

Lower-numbered layers appear first, forming a stable prefix for provider
KV-cache reuse:

- **System (0):** Role identity, conventions, tool definitions. Anthropic's
  prompt caching gives 90% input token cost discount on cache hits.
- **Session (1):** Workspace map, cross-plan context.
- **Task (2):** Plan content, task brief.
- **Dynamic (3):** Gate errors, iteration memory, affect guidance.

The BTreeMap requirement: cache hits require byte-identical content. All
serialization in cacheable layers uses `BTreeMap` for deterministic key
ordering.

### 1.3 Placement

```rust
pub enum Placement {
    Start,   // Highest attention zone (primacy)
    Middle,  // Lowest attention zone
    End,     // Second-highest attention zone (recency)
}
```

Implements the "Lost in the Middle" optimization (Liu et al. 2023): language
models attend most strongly to beginning and end of context, with degraded
attention (~30% loss) in the middle.

### 1.4 AttentionBidder

```rust
pub enum AttentionBidder {
    EpisodicMemory,     // Past task outcomes
    KnowledgeStore,     // Neuro insights, heuristics, warnings
    TaskContext,        // Task description, acceptance criteria
    FileContext,        // Source code, type signatures
    SafetySystem,       // Constraints, anti-patterns
    Enrichment,         // Briefs, research, decompositions
    Daimon,             // Affect guidance
    Collective,         // Mesh knowledge, cross-agent context
    LearningBidder,     // Thompson-sampling learned bids
}
```

Nine subsystem variants for VCG-based attention allocation. Each produces
candidate sections with associated bids.

---

## 2. The Assembly Algorithm

### Phase 1: Decode and Score

Candidate signals are decoded into `PromptSection` structs. Each section is
scored by the provided Scorer, producing a composite score from priority,
recency, relevance, and other dimensions.

### Phase 2: Partition

Sections are partitioned into two groups:
- **Critical:** `SectionPriority::Critical` -- guaranteed inclusion
- **Optional:** Everything else -- included by score order until budget exhausted

### Phase 3: Sort and Select

Optional sections are sorted by two keys:
1. **CacheLayer ascending** -- System before Session before Task before Dynamic
2. **SectionPriority descending** -- High before Normal before Low

Within each (CacheLayer, Priority) group, sections are ordered by
Scorer-assigned score descending. This produces a deterministic ordering that
maximizes prefix cache hits while respecting priority.

### Phase 4: Greedy Include

```
remaining_budget = budget.max_tokens
included = []

// Critical sections always included
for section in critical_sections:
    if estimate_tokens(section.content) <= remaining_budget:
        included.append(section)
        remaining_budget -= estimate_tokens(section.content)
    else:
        section.content = truncate_to_tokens(section.content, remaining_budget)
        included.append(section)
        remaining_budget = 0

// Optional sections by score order
for section in sorted_optional_sections:
    tokens = estimate_tokens(section.content)
    if tokens <= remaining_budget:
        if section.hard_cap and len(section.content) > section.hard_cap:
            section.content = truncate(section.content, section.hard_cap)
            tokens = estimate_tokens(section.content)
        included.append(section)
        remaining_budget -= tokens
    // else: drop this section
```

This is a greedy knapsack, not optimal. Greedy is chosen for:
1. **Speed:** O(n log n) sort + O(n) scan vs O(n x W) for DP knapsack
2. **Determinism:** Same input always produces same output
3. **Priority correctness:** Greedy with priority ordering always includes the
   most important sections

### Phase 5: U-Shape Ordering

```
final_order = [
    sections with Placement::Start,   // highest attention
    sections with Placement::Middle,   // lowest attention
    sections with Placement::End,      // second-highest attention
]
```

Within each placement group, CacheLayer ordering is preserved for cache
stability.

### Phase 6: Concatenate

Sections are concatenated with headers and cache-layer transition markers:

```xml
<!-- roko:layer:0 -->
<!-- roko:section:role_identity -->
{role identity content}

<!-- roko:section:conventions -->
{conventions content}

<!-- roko:layer:1 -->
<!-- roko:section:workspace_map -->
{workspace map content}

<!-- roko:layer:2 -->
<!-- roko:section:task_context -->
{task context}
```

These markers allow the inference gateway to place `cache_control` breakpoints.

---

## 3. Token Estimation

```rust
fn estimate_tokens(text: &str) -> usize {
    text.len() / 4
}
```

Deliberately conservative heuristic:
- English prose averages ~4.5 bytes/token
- Source code averages ~3.5 bytes/token
- 4.0 heuristic slightly overestimates prose, underestimates code
- Correct within +/-15%
- Takes <1 microsecond vs ~2ms for exact tokenization

The `TokenCounter` struct provides an alternative path with configurable
per-model estimates when higher accuracy is needed.

---

## 4. The PromptBuild Metadata

Each composition produces metadata alongside the assembled prompt:

```rust
pub struct PromptBuild {
    pub prompt: String,
    pub estimated_tokens: usize,
    pub sections_included: usize,
    pub sections_dropped: usize,
    pub dropped_names: Vec<String>,
    pub tokens_per_layer: HashMap<CacheLayer, usize>,
    pub section_metadata: Vec<PromptSectionAudit>,
    pub composition_manifest: Option<CompositionManifest>,
}
```

The `CompositionManifest` records VCG payments, candidate scores, and
included/excluded section metadata for diagnostic and learning purposes.

---

## 5. VCG Auto-Select

When the PromptComposer has registered `LearningBidder` instances per
`AttentionBidder` variant, it can automatically select sections using the VCG
mechanism rather than static priorities. The auto-select path:

1. Each registered bidder produces a bid via Thompson sampling
2. Bids are combined with relevance scores
3. The greedy knapsack runs on combined bid values
4. VCG payments are computed for diagnostic purposes
5. After gate outcome, bidders are updated via `update_bidders()`

This allows the composition system to learn which sections are valuable for
each task type over time.

---

## 6. HDC Deduplication

The PromptComposer supports HDC fingerprint-based deduplication:

```rust
pub fn with_hdc_dedup(mut self, threshold: f64) -> Self
```

When enabled, candidates with Hamming distance below the threshold (default
0.15) are treated as near-duplicates. The highest-scored duplicate is kept;
others are excluded. This prevents cluster domination where 15 near-identical
entries about the same topic consume the entire budget.

---

## 7. Critical Section Examples

| Section | Priority | Placement | Rationale |
|---------|----------|-----------|-----------|
| `role_identity` | Critical | Start | Agent must know its identity |
| `task_description` | Critical | Start | Agent must know what to do |
| `safety_constraints` | Critical | Start | Agent must know what not to do |
| `conventions` | Critical | Start | Agent must follow project patterns |
| `gate_errors` | High | End | Recent failures need recency attention |
| `iteration_memory` | High | End | Cross-iteration state prevents repeated mistakes |
| `task_brief` | High | Start | Detailed context for current task |
| `workspace_map` | Normal | Middle | Helpful but not always needed |
| `cross_plan_context` | Normal | Middle | Useful for integration tasks |
| `anti_patterns` | High | End | Prohibitions need recency attention |
| `affect_guidance` | Normal | End | Behavioral modulation |

---

## 8. Test Coverage

18+ tests in `crates/roko-compose/src/prompt.rs`:

- Budget enforcement: sections correctly dropped when budget exceeded
- Critical guarantee: Critical sections survive even when budget exhausted
- Cache-layer ordering: sections appear in System -> Session -> Task -> Dynamic
- Priority ordering: higher-priority sections appear first within layer
- Hard cap: sections truncated to hard_cap before budget fitting
- Token estimation: byte/4 heuristic produces expected values
- Empty input: empty section list produces empty output
- Single section: single Critical section survives any budget
- Metadata: PromptBuild correctly reports included/dropped counts
- VCG payment computation and manifest generation
- AttentionBidder registration and Thompson sampling updates
- HDC deduplication with configurable thresholds

---

## 9. Implementation Status

| Aspect | Status |
|--------|--------|
| Core assembly algorithm | **Shipped** |
| Priority enum with 4 levels | **Shipped** |
| CacheLayer enum | **Shipped** |
| Placement enum (Start/Middle/End) | **Shipped** |
| Hard cap truncation | **Shipped** |
| PromptBuild metadata | **Shipped** |
| Token estimation (byte/4) | **Shipped** |
| VCG auto-select with LearningBidder | **Shipped** |
| HDC deduplication | **Shipped** |
| MultiPatchForager integration | **Shipped** |
| CompositionManifest auditing | **Shipped** |
| Active inference re-scoring during assembly | **Designed, not wired** |

---

## Cross-References

- [composer-trait.md](composer-trait.md) -- Compose trait definition
- [system-prompt-builder-9-layer.md](system-prompt-builder-9-layer.md) -- 9-layer builder
- [token-budget-management.md](token-budget-management.md) -- Budget derivation
- [lost-in-the-middle-u-shape.md](lost-in-the-middle-u-shape.md) -- U-shape attention
- [vcg-attention-auction.md](vcg-attention-auction.md) -- VCG mechanism details
- [5-stage-assembly-pipeline.md](5-stage-assembly-pipeline.md) -- Full pipeline
- `crates/roko-compose/src/prompt.rs` -- Implementation source
