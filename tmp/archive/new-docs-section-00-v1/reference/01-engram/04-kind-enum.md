# Engram — Kind Enum

> The Kind discriminant tells every operator what category of information an Engram represents and how to interpret its Body.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Kind type overview](../10-types/kind/00-overview.md)  
**Used by**: every operator that reads Engrams  
**Last reviewed**: 2026-04-19

---

## TL;DR

Kind is the Engram's category tag. It is a required field, included in the identity hash,
and determines which Body variant is valid for that Engram. When you write a Scorer, Gate,
or Router that processes only certain kinds of information, you match on Kind. The full
variant list, descriptions, and a decision tree for choosing the right Kind live in this
file.

---

## The Idea

Without Kind, consumers would have to inspect the Body to figure out what an Engram
represents. That couples the reader to the payload format. Kind separates "what is this"
from "what does it contain," so operators can filter and route Engrams without
deserializing the Body.

Kind also determines what operations are semantically meaningful on an Engram. Scoring
novelty is defined differently for a `KnowledgeEntry` versus a `GateVerdict`. Decay
schedules differ between ephemeral `ToolTrace` entries and long-lived `KnowledgeEntry`
entries. Kind carries that semantic intent.

---

## Specification

```rust
<!-- source: crates/roko-core/src/kind.rs -->

/// The category of information an Engram represents.
/// Determines how Body is interpreted and which operators apply.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Kind {
    /// Output produced by an agent in response to a prompt or task.
    AgentOutput,

    /// Verdict from a Gate: pass/fail + confidence + rationale.
    GateVerdict,

    /// Trace of a tool call: tool name, inputs, outputs, duration.
    ToolTrace,

    /// A durable knowledge entry (fact, rule, pattern, heuristic).
    KnowledgeEntry,

    /// A prediction about a future state or agent output.
    Prediction,

    /// A recorded observation from the environment or agent's sensors.
    Observation,

    /// A prompt or plan step emitted by the orchestrator.
    Plan,

    /// An episode record: a completed agent session summary.
    Episode,

    /// A reflection: the agent's self-assessment of a completed session.
    Reflection,

    /// A pheromone deposit: inter-agent stigmergic signal.
    Pheromone,

    /// A metric measurement at a point in time.
    Metric,

    /// A context assembly record: what context was built for a prompt.
    ContextAssembly,

    /// A model selection decision record.
    ModelSelection,

    /// An error or exception record.
    ErrorRecord,

    /// A custom kind defined by the application layer.
    /// Use sparingly; prefer extending the enum when the kind is stable.
    Custom(String),
}
```

---

## Variant Reference

### `Kind::AgentOutput`

An LLM agent's textual or structured response to a prompt. The Body carries the raw
text, the model used, and token counts.

**Typical decay**: Exponential or Demurrage — outputs become stale as context evolves.  
**Typical score**: High utility if the output passed gates; low if rejected.  
**Lineage**: References the `Plan` or `ContextAssembly` Engram that produced it.

---

### `Kind::GateVerdict`

The result of a Gate evaluation: pass/fail, confidence, which gate, and rationale.

**Typical decay**: Step decay — verdicts are valid until the next epoch; then expire.  
**Typical score**: High confidence in the verdict; utility reflects downstream impact.  
**Lineage**: References the `AgentOutput` being evaluated.

---

### `Kind::ToolTrace`

A record of a single tool invocation: tool name, JSON inputs, JSON outputs, wall-clock
duration, exit code.

**Typical decay**: Fast exponential — tool traces are transient diagnostics.  
**Typical score**: Utility = 1.0 if tool succeeded; 0.0 on error.  
**Lineage**: References the `AgentOutput` or `Plan` that triggered the tool.

---

### `Kind::KnowledgeEntry`

A durable fact, rule, pattern, or heuristic extracted from agent experience. The most
"long-lived" Kind — intended to persist for weeks or months in the Neuro substrate.

**Typical decay**: Demurrage (idle tax + reinforcement on use) or cold-tier freeze.  
**Typical score**: Confidence and novelty weighted heavily.  
**Lineage**: May reference the `AgentOutput` or `Episode` it was extracted from.

---

### `Kind::Prediction`

A prediction about a future agent output, gate verdict, or environment state. The
prediction is stored at emission time; when the predicted event occurs, the prediction's
score is updated with the actual outcome.

**Typical decay**: Exponential — predictions expire when the predicted horizon passes.  
**Typical score**: Confidence weighted by calibration history.  
**Lineage**: References the `Observation` or context that motivated the prediction.

---

### `Kind::Observation`

A raw sensing event from the environment: file system change, HTTP response, metric
reading, code test result. Observations are the substrate's ground truth input.

**Typical decay**: Fast exponential — observations age quickly.  
**Typical score**: High novelty if first observation of a pattern; low if routine.  
**Lineage**: Empty (root Engrams).

---

### `Kind::Plan`

A plan step, task decomposition, or prompt template emitted by the orchestrator. The
Body carries the plan graph or step descriptor.

**Typical decay**: Step decay tied to the plan's lifecycle.  
**Typical score**: Utility reflects whether the plan step succeeded.  
**Lineage**: References the parent `Plan` or the `AgentOutput` that requested planning.

---

### `Kind::Episode`

A summary of a completed agent session: task, steps taken, tools used, final verdict,
performance metrics.

**Typical decay**: Slow exponential or Demurrage — episodes are valuable for learning.  
**Typical score**: Utility reflects episode success rate.  
**Lineage**: References all `GateVerdict` and `AgentOutput` Engrams from the session.

---

### `Kind::Reflection`

A self-assessment produced by the agent at episode end: what worked, what failed, what
to do differently. Input for the `roko-learn` playbook extractor.

**Typical decay**: Long-lived; similar to KnowledgeEntry.  
**Lineage**: References the `Episode` being reflected on.

---

### `Kind::Pheromone`

An inter-agent stigmergic signal, inspired by ant colony optimization. Agents deposit
pheromones to communicate successful paths; the substrate's decay causes weak signals
to fade and reinforcement keeps strong signals alive.

**Typical decay**: Fast exponential (standard ACO-style evaporation).  
**Lineage**: References the `AgentOutput` or `GateVerdict` that triggered the deposit.

---

### `Kind::Metric`

A numeric measurement snapshot: latency, token count, gate pass rate, memory usage, etc.

**Typical decay**: Fast step decay — metrics are time-series data.  
**Typical score**: Utility based on whether the metric is within expected ranges.

---

### `Kind::ContextAssembly`

A record of what context was assembled for a particular prompt: which Engrams were
retrieved, what was included, what was excluded, and why.

**Typical decay**: Tied to the `AgentOutput` it produced.  
**Lineage**: References all Engrams that contributed to the context window.

---

### `Kind::ModelSelection`

A record of which LLM model was selected for a task, by which router, and why.

**Typical decay**: Fast — model selection decisions are transient diagnostics.  
**Lineage**: References the `Plan` or `AgentOutput` that required a model.

---

### `Kind::ErrorRecord`

A structured error report: subsystem, error type, message, backtrace hash, recovery action.

**Typical decay**: Moderate — errors feed into calibration and learning.  
**Lineage**: References the Engram that was being processed when the error occurred.

---

### `Kind::Custom(String)`

An application-defined Kind for domain-specific use cases. Use when the application layer
needs to distinguish a new category that has no appropriate standard variant. Avoid
proliferating custom kinds — if a kind stabilizes, add it to the enum.

Custom kinds participate in all standard operations: scoring, decay, provenance, lineage.
Operators that do not recognize a Custom kind should pass through without processing.

---

## Decision Tree: Which Kind to Use?

```
Is the Engram produced by an LLM call?
├─ Yes: AgentOutput
└─ No:
   Is it a judgment about another Engram?
   ├─ Yes (gate pass/fail): GateVerdict
   └─ No:
      Is it from calling an external tool?
      ├─ Yes: ToolTrace
      └─ No:
         Is it a durable fact/rule/pattern?
         ├─ Yes: KnowledgeEntry
         └─ No:
            Is it a forward-looking statement?
            ├─ Yes: Prediction
            └─ No:
               Is it from external sensing?
               ├─ Yes: Observation
               └─ No: use Plan, Episode, Reflection, Pheromone, Metric, or Custom as appropriate
```

---

## Invariants

1. `body` variant must match `kind` (enforced by `EngramBuilder`)
2. `Kind::Custom(s)` where `s.is_empty()` is invalid
3. Kind is included in the identity hash; changing Kind produces a new Engram

---

## See Also

- [`../10-types/kind/01-variants.md`](../10-types/kind/01-variants.md) — full variant reference (type folder)
- [`../10-types/kind/02-compositional-kinds.md`](../10-types/kind/02-compositional-kinds.md) — how Kinds compose
- [`05-body-enum.md`](05-body-enum.md) — Body variants that correspond to each Kind
