# Dream Routing Advice

> **v3 depth file** -- `/docs/v3/depth/10-dreams/dream-routing-advice.md`
> Implementation: `crates/roko-dreams/src/routing_advice.rs`
> Status: **Wired** -- `DreamRoutingAdvice`, `RoutingRecommendation`, `PatternSummary`,
> `generate_routing_advice`, `dream_advice_to_routing_bias`, `relevant_pattern_summaries`,
> staleness TTL, and persistence are live

---

## 1. What Dream Routing Advice Is

After each dream cycle, the consolidation phase generates model-routing advice
based on discovered patterns. This advice is persisted to
`.roko/learn/dream-routing-advice.json` and consulted by the runner at dispatch
time to bias model selection toward historically successful configurations.

The routing advice module bridges the dream subsystem and the CascadeRouter:
dreams discover which model/task pairings work well (or poorly), and the
advice file carries that knowledge into waking dispatch decisions.

---

## 2. Core Data Structures

### DreamRoutingAdvice

The top-level advice structure aggregates recommendations and pattern summaries
from a single dream cycle:

```rust
/// Routing recommendations produced by dream consolidation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DreamRoutingAdvice {
    /// When this advice was generated.
    pub generated_at: DateTime<Utc>,
    /// Dream report that produced this advice.
    pub source_dream_report: String,
    /// Individual model-routing recommendations.
    pub recommendations: Vec<RoutingRecommendation>,
    /// Discovered patterns ready for prompt/context injection.
    pub pattern_summaries: Vec<PatternSummary>,
}
```

### RoutingRecommendation

Each recommendation captures a specific model-routing signal derived from a
dream meta-pattern:

```rust
/// One routing recommendation derived from a dream meta-pattern.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoutingRecommendation {
    /// Task category this recommendation applies to.
    pub task_category: String,
    /// Complexity band this recommendation applies to.
    pub complexity_band: String,
    /// Model to prefer.
    pub recommended_model: String,
    /// Models to deprioritize for this task shape.
    pub deprioritize: Vec<String>,
    /// Confidence in 0.0..=1.0.
    pub confidence: f64,
    /// Number of episodes supporting the recommendation.
    pub supporting_episodes: usize,
    /// Success rate observed for the recommended model or task shape.
    pub recommended_model_success_rate: f64,
    /// Meta-pattern signature for deduplication.
    pub pattern_signature: u64,
}
```

### PatternSummary

Discovered patterns that are suitable for prompt injection into the agent's
waking context:

```rust
/// A discovered dream pattern suitable for prompt injection.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PatternSummary {
    /// Human-readable pattern description.
    pub description: String,
    /// Task categories where the pattern applies.
    pub applies_to: Vec<String>,
    /// Actionable guidance for a future agent.
    pub guidance: String,
    /// Confidence in 0.0..=1.0.
    pub confidence: f64,
    /// Meta-pattern signature for deduplication.
    pub signature: u64,
}
```

---

## 3. Advice Generation

The `generate_routing_advice` function transforms cross-episode consolidation
output into routing recommendations:

```rust
pub fn generate_routing_advice(
    report: &CrossEpisodeConsolidationReport,
    episodes: &[Episode],
    generated_at: DateTime<Utc>,
    source_dream_report: impl Into<String>,
) -> DreamRoutingAdvice
```

### Algorithm

For each meta-pattern in the consolidation report:

1. **Collect cluster episodes**: Map pattern episode indices to actual
   episodes. Skip clusters with fewer than 3 episodes (insufficient evidence).

2. **Compute success rate**: Count successful episodes in the cluster and
   divide by total cluster size.

3. **Identify majority fields**: For model, task category, and complexity
   band, determine the majority value (the value that appears in more than
   half the cluster). Uses strict majority: the field must appear in > 50%
   of cluster episodes to be considered a majority.

4. **Generate recommendation**: If the majority model has a clear signal:
   - Success rate < 0.4: recommend upgrading to the next tier model and
     deprioritize the current model
   - Success rate > 0.8: recommend the current model (it is reliable)
   - Between 0.4 and 0.8: no recommendation (insufficient signal)

5. **Generate pattern guidance**: Natural-language guidance string describing
   the observed model/task performance for prompt injection.

6. **Deduplicate**: Both recommendations and pattern summaries are
   deduplicated by task-category/complexity-band/model triple and by
   signature respectively, retaining the highest-confidence entry.

### Model Tier Escalation

When a model has low success rate (< 0.4), the recommendation suggests the
next tier:

```rust
fn next_tier_model(current: &str) -> String {
    let lower = current.to_ascii_lowercase();
    if lower.contains("haiku") {
        replace_model_tier(current, "haiku", "sonnet")
    } else if lower.contains("sonnet") {
        replace_model_tier(current, "sonnet", "opus")
    } else {
        current.to_string()
    }
}
```

Haiku escalates to Sonnet; Sonnet escalates to Opus. Unknown models are
returned unchanged.

### Pattern Guidance Generation

The guidance string provides natural-language context for prompt injection:

```rust
fn generate_pattern_guidance(
    majority_model: Option<&str>,
    success_rate: f64,
    episode_count: usize,
) -> String
```

Three guidance templates:
- Low success (< 0.4): "consider a more capable model or narrower context"
- High success (> 0.8): "this model/task pairing is reliable"
- Mixed (0.4-0.8): "verify edge cases early"

---

## 4. Advice Consumption

### Cascade Routing Bias

The `dream_advice_to_routing_bias` function converts persisted advice into a
`RoutingBias` consumable by the `CascadeRouter`:

```rust
pub fn dream_advice_to_routing_bias(
    advice: &DreamRoutingAdvice,
    task_category: &str,
    complexity_band: &str,
) -> RoutingBias
```

Matching is case-insensitive and whitespace-normalized. Recommendations with
confidence < 0.5 or fewer than 3 supporting episodes are filtered out. The
returned `RoutingBias` contains:
- `deprioritize`: models to avoid for this task shape
- `reason`: human-readable explanation of why the bias was applied

### Pattern Summaries for Prompt Injection

The `relevant_pattern_summaries` function retrieves patterns applicable to
a given task category:

```rust
pub fn relevant_pattern_summaries<'a>(
    advice: &'a DreamRoutingAdvice,
    task_category: &str,
    min_confidence: f64,
    limit: usize,
) -> Vec<&'a PatternSummary>
```

Results are sorted by confidence (highest first) and truncated to `limit`.
These summaries are injected into the agent's system prompt during waking
dispatch, providing dream-derived guidance for the current task.

---

## 5. Persistence and Staleness

### File Location

Advice is persisted at `{workspace}/.roko/learn/dream-routing-advice.json`.

### Staleness TTL

Advice has a default staleness TTL of 1 hour:

```rust
pub const ROUTING_ADVICE_DEFAULT_TTL: chrono::TimeDelta = chrono::TimeDelta::hours(1);
```

When loading advice, if the `generated_at` timestamp is older than the TTL,
the loader returns an empty default instead:

```rust
pub fn load_dream_routing_advice_at_with_ttl(
    path: &Path,
    ttl: chrono::TimeDelta,
) -> Result<DreamRoutingAdvice> {
    // ...
    let age = Utc::now() - advice.generated_at;
    if age > ttl {
        tracing::debug!(
            generated_at = %advice.generated_at,
            age_secs = age.num_seconds(),
            ttl_secs = ttl.num_seconds(),
            "routing advice is stale, returning empty default"
        );
        return Ok(DreamRoutingAdvice::default());
    }
    Ok(advice)
}
```

This prevents stale dream advice from biasing routing decisions when the
agent's environment may have changed since the advice was generated.

### Save Path

Saving creates parent directories if needed and writes pretty-printed JSON:

```rust
pub fn save_dream_routing_advice(
    workdir: impl AsRef<Path>,
    advice: &DreamRoutingAdvice,
) -> Result<()>
```

---

## 6. Deduplication

Both recommendations and pattern summaries are deduplicated after generation.

### Recommendation Deduplication

Recommendations are deduplicated by the tuple (task_category, complexity_band,
recommended_model) after case-insensitive normalization. When duplicates exist,
the entry with the highest confidence (then highest supporting_episodes, then
lowest pattern_signature) is retained.

### Pattern Deduplication

Patterns are deduplicated by their `signature` field. The entry with the
highest confidence is retained.

---

## 7. Integration Points

### Dream Cycle Output

The `DreamCycle` calls `generate_routing_advice` after cross-episode
consolidation completes and includes the advice in the `DreamCycleReport`.
The `DreamRunner` then calls `save_dream_routing_advice` to persist it.

### Runner Dispatch Consumption

At dispatch time, the runner loads dream routing advice via
`load_dream_routing_advice` and converts it to a `RoutingBias` via
`dream_advice_to_routing_bias`. This bias is passed to the `CascadeRouter`
alongside other routing signals (provider health, experiment assignments,
manual overrides).

### Context Enrichment

Relevant pattern summaries are injected into the agent's system prompt via
`relevant_pattern_summaries`. The `SystemPromptBuilder` includes a dedicated
section for dream-derived context alongside playbook matches and knowledge
context.

---

## 8. Example Advice File

```json
{
  "generated_at": "2026-09-15T02:30:00Z",
  "source_dream_report": "dream-1726367400000.json",
  "recommendations": [
    {
      "task_category": "refactor",
      "complexity_band": "standard",
      "recommended_model": "claude-sonnet-4-20250514",
      "deprioritize": ["claude-haiku-4-5-20251001"],
      "confidence": 0.78,
      "supporting_episodes": 7,
      "recommended_model_success_rate": 0.86,
      "pattern_signature": 12345678901234
    }
  ],
  "pattern_summaries": [
    {
      "description": "Refactor tasks involving 3+ files cluster with mixed gate results",
      "applies_to": ["refactor"],
      "guidance": "Historical dream consolidation shows claude-sonnet-4-20250514 has an 86% success rate for this task shape across 7 episodes; this model/task pairing is reliable.",
      "confidence": 0.78,
      "signature": 12345678901234
    }
  ]
}
```

---

## 9. Cross-References

| Document | Relevance |
|----------|-----------|
| [nrem-replay.md](nrem-replay.md) | Replay clusters that feed advice generation |
| [consolidation-and-staging.md](consolidation-and-staging.md) | Integration phase where advice is produced |
| [cross-system-integration.md](cross-system-integration.md) | CascadeRouter integration details |
| [sleep-time-compute.md](sleep-time-compute.md) | Budget context for routing decisions |
| [three-phase-cycle.md](three-phase-cycle.md) | Dream cycle lifecycle that produces advice |
