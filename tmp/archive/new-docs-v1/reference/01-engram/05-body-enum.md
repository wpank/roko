# Engram — Body Enum

> The Body enum holds the typed payload for each Kind variant. One Body variant per Kind.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Kind](04-kind-enum.md), [Body type](../10-types/body/00-overview.md)  
**Used by**: all operators that process Engram content  
**Last reviewed**: 2026-04-19

---

## TL;DR

Body is an enum where each variant carries the typed payload for one Kind. The variant
discriminant must match the Kind field. Body content is included in the identity hash and
is immutable after construction. Operators dispatch on Kind first, then decode the Body.

---

## The Idea

Rather than a schema-less JSON blob, Body provides typed payloads per Kind. This means:

- Operators get compile-time guarantees about the fields they care about.
- The serialization format is determinate per variant.
- The canonical encoding for the ContentHash is well-defined for each case.

Each Body variant is a distinct struct carrying the minimum fields for that Kind. Operators
that only handle `GateVerdict` bodies never touch `AgentOutput` fields.

---

## Specification

```rust
<!-- source: crates/roko-core/src/body.rs -->

/// Typed payload for an Engram. Variant must match Kind.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Body {
    AgentOutput(AgentOutputBody),
    GateVerdict(GateVerdictBody),
    ToolTrace(ToolTraceBody),
    KnowledgeEntry(KnowledgeEntryBody),
    Prediction(PredictionBody),
    Observation(ObservationBody),
    Plan(PlanBody),
    Episode(EpisodeBody),
    Reflection(ReflectionBody),
    Pheromone(PheromoneBody),
    Metric(MetricBody),
    ContextAssembly(ContextAssemblyBody),
    ModelSelection(ModelSelectionBody),
    ErrorRecord(ErrorRecordBody),
    Custom(CustomBody),
}
```

### Body Sub-structs

```rust
<!-- source: crates/roko-core/src/body.rs -->

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AgentOutputBody {
    /// Raw text produced by the model.
    pub text: String,
    /// Model identifier (e.g. "claude-3-7-sonnet", "gpt-4o").
    pub model: String,
    /// Prompt tokens consumed.
    pub prompt_tokens: u32,
    /// Completion tokens produced.
    pub completion_tokens: u32,
    /// Whether the model indicated a stop reason (vs. token limit).
    pub finished_normally: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GateVerdictBody {
    /// true = passed, false = failed.
    pub passed: bool,
    /// Gate name / identifier.
    pub gate_name: String,
    /// Confidence in the verdict (0.0–1.0).
    pub confidence: f64,
    /// Human-readable rationale for the verdict.
    pub rationale: String,
    /// Rung level in the 6-rung gate pipeline (1–6).
    pub rung: u8,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolTraceBody {
    /// Tool name.
    pub tool_name: String,
    /// JSON-serialized tool input.
    pub input_json: String,
    /// JSON-serialized tool output.
    pub output_json: String,
    /// Wall-clock duration in milliseconds.
    pub duration_ms: u64,
    /// Exit code: 0 = success.
    pub exit_code: i32,
    /// Optional error message on failure.
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeEntryBody {
    /// The fact, rule, or heuristic in natural language.
    pub text: String,
    /// Optional structured representation (JSON).
    pub structured: Option<String>,
    /// Domain tags (e.g. ["rust", "async", "patterns"]).
    pub domain_tags: Vec<String>,
    /// Validation tier (0 = unvalidated, 1 = self-verified, 2 = peer-verified, 3 = chain-witnessed).
    pub validation_tier: u8,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PredictionBody {
    /// Natural language description of the prediction.
    pub text: String,
    /// Predicted value as JSON.
    pub predicted_value: String,
    /// Time horizon: Unix ms at which the prediction should be evaluated.
    pub horizon_ms: i64,
    /// Whether the prediction has been resolved.
    pub resolved: bool,
    /// Actual outcome after resolution (None until resolved).
    pub actual_value: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ObservationBody {
    /// Source of the observation (e.g. "file_watcher", "http_client", "test_runner").
    pub source: String,
    /// Observation content as JSON.
    pub content_json: String,
    /// Whether this observation was expected or surprising.
    pub was_expected: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlanBody {
    /// Plan step description.
    pub description: String,
    /// Step index in the plan graph.
    pub step_index: u32,
    /// Total steps in the plan.
    pub total_steps: u32,
    /// JSON-serialized step parameters.
    pub params_json: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EpisodeBody {
    /// Human-readable episode summary.
    pub summary: String,
    /// Number of steps taken.
    pub step_count: u32,
    /// Number of gate passes.
    pub gate_passes: u32,
    /// Number of gate failures.
    pub gate_failures: u32,
    /// Total tokens consumed.
    pub total_tokens: u64,
    /// Whether the episode objective was achieved.
    pub objective_achieved: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReflectionBody {
    /// What worked well (lessons learned).
    pub lessons: Vec<String>,
    /// What failed and why.
    pub failures: Vec<String>,
    /// Proposed adjustments for next time.
    pub adjustments: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PheromoneBody {
    /// Pheromone category (e.g. "Opportunity", "Wisdom", "Pattern", "Hazard").
    pub kind: String,
    /// Signal intensity (0.0–1.0).
    pub intensity: f64,
    /// Scope of the pheromone (e.g. "local", "mesh", "global").
    pub scope: String,
    /// Optional location hint (e.g. file path, URL, task id).
    pub location: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MetricBody {
    /// Metric name.
    pub name: String,
    /// Numeric value.
    pub value: f64,
    /// Unit (e.g. "ms", "tokens", "bytes").
    pub unit: String,
    /// Optional labels for multi-dimensional metrics.
    pub labels: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContextAssemblyBody {
    /// Engram ids included in the context window (ordered).
    pub included_ids: Vec<ContentHash>,
    /// Engram ids considered but excluded.
    pub excluded_ids: Vec<ContentHash>,
    /// Reason for exclusions (e.g. "score too low", "budget exceeded").
    pub exclusion_reasons: BTreeMap<String, String>,
    /// Total tokens in the assembled context.
    pub total_tokens: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelSelectionBody {
    /// Selected model identifier.
    pub model: String,
    /// Router that made the selection.
    pub router: String,
    /// Rationale for selection.
    pub rationale: String,
    /// Estimated cost tier (1 = cheapest).
    pub cost_tier: u8,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ErrorRecordBody {
    /// Subsystem that produced the error.
    pub subsystem: String,
    /// Error type name.
    pub error_type: String,
    /// Human-readable error message.
    pub message: String,
    /// BLAKE3 hash of the backtrace (for deduplication).
    pub backtrace_hash: Option<ContentHash>,
    /// Recovery action taken (if any).
    pub recovery_action: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CustomBody {
    /// Application-defined type tag.
    pub type_tag: String,
    /// JSON-serialized payload.
    pub payload_json: String,
}
```

---

## Serialization Rules

Body is serialized using `serde` with the variant tag included in the canonical encoding.
The canonical encoding for ContentHash computation uses:

1. The variant discriminant as a u32 (little-endian)
2. The variant's field bytes, in declaration order, each prefixed with its byte length as u32 LE

For the precise byte-level encoding, see
[`../10-types/body/02-serialization.md`](../10-types/body/02-serialization.md).

---

## Invariants

1. Body variant must match Kind (enforced at build time by EngramBuilder)
2. No Body variant is valid for multiple Kinds
3. Body is immutable after construction
4. `CustomBody::type_tag` must not be empty
5. JSON fields (`input_json`, `output_json`, `params_json`, `payload_json`) must be valid JSON

---

## Failure Modes

| Failure | Cause | Recovery |
|---------|-------|----------|
| Kind-Body mismatch | Body variant does not match Kind | `EngramBuilder::build()` returns `Err`; rejected at construction |
| Invalid JSON field | Body contains non-JSON in a JSON field | `EngramBuilder` validates JSON fields; returns `Err` |
| Empty type_tag | `CustomBody::type_tag.is_empty()` | `EngramBuilder` returns `Err` |

---

## See Also

- [`../10-types/body/00-overview.md`](../10-types/body/00-overview.md) — Body type folder
- [`04-kind-enum.md`](04-kind-enum.md) — Kind variants that pair with Body
- [`../10-types/body/02-serialization.md`](../10-types/body/02-serialization.md) — serialization spec
