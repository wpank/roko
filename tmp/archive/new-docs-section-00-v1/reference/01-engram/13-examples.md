# Engram — Examples

> Ten worked examples from the simplest possible Engram to complex lineage chains with full provenance and scoring.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Builder](07-builder-pattern.md), [Struct reference](01-struct-reference.md)  
**Last reviewed**: 2026-04-19

---

## TL;DR

All examples below are runnable Rust snippets. They assume `use roko_core::*;`. The
simplest Engrams are a single `.kind().body().build()` chain. Complex examples add
lineage, provenance, decay, custom scoring, and multi-step chains.

---

## Example 1: Minimal AgentOutput

The simplest valid Engram: an agent response with all defaults.

```rust
<!-- source: crates/roko-core/examples/engram_examples.rs -->

let engram = EngramBuilder::new()
    .kind(Kind::AgentOutput)
    .body(Body::AgentOutput(AgentOutputBody {
        text: "Paris is the capital of France.".to_string(),
        model: "claude-3-7-sonnet".to_string(),
        prompt_tokens: 45,
        completion_tokens: 8,
        finished_normally: true,
    }))
    .build()?;

// id computed automatically: blake3(kind + body + defaults)
// fingerprint computed automatically
// score = Score::default() (all axes 0.5)
// decay = Decay::Demurrage(default)
// provenance = Provenance::anonymous()
// lineage = []
```

---

## Example 2: Tool Trace

Recording the result of a shell command:

```rust
<!-- source: crates/roko-core/examples/engram_examples.rs -->

let trace = EngramBuilder::new()
    .kind(Kind::ToolTrace)
    .body(Body::ToolTrace(ToolTraceBody {
        tool_name: "bash".to_string(),
        input_json: r#"{"command": "cargo test --lib"}"#.to_string(),
        output_json: r#"{"exit_code": 0, "stdout": "test result: ok. 376 passed"}"#.to_string(),
        duration_ms: 8_412,
        exit_code: 0,
        error: None,
    }))
    .provenance(Provenance {
        author: "roko-std-bash".to_string(),
        trust: TrustLevel::SelfVerified,
        tainted: false,
        custody: vec![],
    })
    .build()?;
```

---

## Example 3: Gate Verdict Derived from Agent Output

A GateVerdict that references its parent AgentOutput:

```rust
<!-- source: crates/roko-core/examples/engram_examples.rs -->

// First, build the agent output
let output = EngramBuilder::new()
    .kind(Kind::AgentOutput)
    .body(Body::AgentOutput(AgentOutputBody {
        text: "The Eiffel Tower is 300 meters tall.".to_string(),
        model: "gpt-4o".to_string(),
        prompt_tokens: 52,
        completion_tokens: 11,
        finished_normally: true,
    }))
    .build()?;

// Then, build the gate verdict referencing the output
let verdict = EngramBuilder::new()
    .kind(Kind::GateVerdict)
    .body(Body::GateVerdict(GateVerdictBody {
        passed: true,
        gate_name: "factual_accuracy".to_string(),
        confidence: 0.91,
        rationale: "Height is 300m to antenna tip; verified against trusted source.".to_string(),
        rung: 2,
    }))
    .parent(output.id)   // link to the output being judged
    .provenance(Provenance {
        author: "roko-gate-v1".to_string(),
        trust: TrustLevel::SelfVerified,
        tainted: false,
        custody: vec![],
    })
    .build()?;

assert_eq!(verdict.lineage, vec![output.id]);
```

---

## Example 4: Knowledge Entry with Custom Decay

A durable knowledge entry that stays warm if frequently retrieved:

```rust
<!-- source: crates/roko-core/examples/engram_examples.rs -->

let knowledge = EngramBuilder::new()
    .kind(Kind::KnowledgeEntry)
    .body(Body::KnowledgeEntry(KnowledgeEntryBody {
        text: "Rust's ownership system prevents data races at compile time.".to_string(),
        structured: None,
        domain_tags: vec!["rust".to_string(), "ownership".to_string(), "safety".to_string()],
        validation_tier: 2,
    }))
    .decay(Decay::Demurrage(DemurrageParams {
        balance: 1.0,
        idle_tax_per_day: 0.005,   // lose 0.5% per idle day
        reinforcement_per_use: 0.1, // gain 10% on each retrieval
    }))
    .provenance(Provenance {
        author: "neuro-extractor-v3".to_string(),
        trust: TrustLevel::PeerVerified,
        tainted: false,
        custody: vec![],
    })
    .score(Score {
        confidence: 0.95,
        novelty: 0.6,
        utility: 0.8,
        reputation: 0.75,
        precision: Some(0.9),
        salience: None,
        coherence: None,
    })
    .tag("domain", "rust")
    .tag("source", "roko-docs")
    .build()?;
```

---

## Example 5: Prediction with Horizon

A prediction about an upcoming test suite result:

```rust
<!-- source: crates/roko-core/examples/engram_examples.rs -->

use std::time::{SystemTime, UNIX_EPOCH, Duration};

let horizon_ms = SystemTime::now()
    .checked_add(Duration::from_secs(300))  // 5 minutes from now
    .unwrap()
    .duration_since(UNIX_EPOCH).unwrap()
    .as_millis() as i64;

let prediction = EngramBuilder::new()
    .kind(Kind::Prediction)
    .body(Body::Prediction(PredictionBody {
        text: "The refactored test suite will pass on the first run.".to_string(),
        predicted_value: r#"{"passed": true, "test_count": 376}"#.to_string(),
        horizon_ms,
        resolved: false,
        actual_value: None,
    }))
    .build()?;
```

---

## Example 6: Multi-Step Lineage Chain

A three-step chain: Observation → KnowledgeEntry → AgentOutput → GateVerdict:

```rust
<!-- source: crates/roko-core/examples/engram_examples.rs -->

// Step 1: External observation
let obs = EngramBuilder::new()
    .kind(Kind::Observation)
    .body(Body::Observation(ObservationBody {
        source: "test_runner".to_string(),
        content_json: r#"{"result": "FAILED", "test": "test_memory_safety", "error": "use-after-free"}"#.to_string(),
        was_expected: false,
    }))
    .build()?;

// Step 2: Extract knowledge from observation
let knowledge = EngramBuilder::new()
    .kind(Kind::KnowledgeEntry)
    .body(Body::KnowledgeEntry(KnowledgeEntryBody {
        text: "test_memory_safety fails with use-after-free in the buffer module.".to_string(),
        structured: None,
        domain_tags: vec!["memory-safety".to_string(), "bugs".to_string()],
        validation_tier: 1,
    }))
    .parent(obs.id)
    .build()?;

// Step 3: Agent output referencing the knowledge
let output = EngramBuilder::new()
    .kind(Kind::AgentOutput)
    .body(Body::AgentOutput(AgentOutputBody {
        text: "Fix: add drop check to BufferGuard to prevent use-after-free.".to_string(),
        model: "claude-3-7-sonnet".to_string(),
        prompt_tokens: 1024,
        completion_tokens: 48,
        finished_normally: true,
    }))
    .parent(knowledge.id)
    .build()?;

// Step 4: Gate verdict
let verdict = EngramBuilder::new()
    .kind(Kind::GateVerdict)
    .body(Body::GateVerdict(GateVerdictBody {
        passed: true,
        gate_name: "code_compiles".to_string(),
        confidence: 0.99,
        rationale: "cargo build succeeds with the proposed fix.".to_string(),
        rung: 1,
    }))
    .parent(output.id)
    .build()?;

// Full chain: verdict → output → knowledge → obs
assert_eq!(verdict.lineage, vec![output.id]);
assert_eq!(output.lineage, vec![knowledge.id]);
assert_eq!(knowledge.lineage, vec![obs.id]);
assert!(obs.lineage.is_empty());
```

---

## Example 7: Pheromone Deposit

An inter-agent signal marking a successful strategy:

```rust
<!-- source: crates/roko-core/examples/engram_examples.rs -->

let pheromone = EngramBuilder::new()
    .kind(Kind::Pheromone)
    .body(Body::Pheromone(PheromoneBody {
        kind: "Wisdom".to_string(),
        intensity: 0.8,
        scope: "mesh".to_string(),
        location: Some("task/code-review/rust-ownership".to_string()),
    }))
    .decay(Decay::Exponential(ExponentialDecayParams {
        half_life_secs: 3600.0, // 1-hour half-life
    }))
    .parent(verdict.id)  // references the successful gate verdict
    .build()?;
```

---

## Example 8: Episode Summary

End-of-session summary with performance metrics:

```rust
<!-- source: crates/roko-core/examples/engram_examples.rs -->

let episode = EngramBuilder::new()
    .kind(Kind::Episode)
    .body(Body::Episode(EpisodeBody {
        summary: "Reviewed PR #4421: ownership fix in buffer module. 3 iterations.".to_string(),
        step_count: 3,
        gate_passes: 7,
        gate_failures: 2,
        total_tokens: 8_192,
        objective_achieved: true,
    }))
    .lineage(vec![obs.id, knowledge.id, output.id, verdict.id])
    .score(Score {
        confidence: 1.0,
        novelty: 0.3,
        utility: 0.9,
        reputation: 0.75,
        precision: None,
        salience: None,
        coherence: None,
    })
    .build()?;
```

---

## Example 9: Error Record with Custody

An error with a chain-of-custody record:

```rust
<!-- source: crates/roko-core/examples/engram_examples.rs -->

let error_record = EngramBuilder::new()
    .kind(Kind::ErrorRecord)
    .body(Body::ErrorRecord(ErrorRecordBody {
        subsystem: "roko-fs".to_string(),
        error_type: "SubstrateWriteError".to_string(),
        message: "JSONL shard file exceeded 100MB limit.".to_string(),
        backtrace_hash: None,
        recovery_action: Some("Rotated shard, continuing write.".to_string()),
    }))
    .provenance(Provenance {
        author: "roko-fs-shard-manager".to_string(),
        trust: TrustLevel::LocalAgent,
        tainted: false,
        custody: vec![
            CustodyRecord {
                timestamp_ms: now_ms(),
                actor: "shard-manager".to_string(),
                action: "shard_rotation".to_string(),
            }
        ],
    })
    .build()?;
```

---

## Example 10: Deserialization and Verification

Round-trip: serialize to JSONL, deserialize, verify hash:

```rust
<!-- source: crates/roko-core/examples/engram_examples.rs -->

let original = EngramBuilder::new()
    .kind(Kind::KnowledgeEntry)
    .body(Body::KnowledgeEntry(KnowledgeEntryBody {
        text: "Content-addressed data is immutable by design.".to_string(),
        structured: None,
        domain_tags: vec!["architecture".to_string()],
        validation_tier: 1,
    }))
    .build()?;

// Serialize to JSONL
let json_line = serde_json::to_string(&original)?;

// Deserialize
let deserialized: Engram = serde_json::from_str(&json_line)?;

// Verify identity preserved
assert_eq!(original.id, deserialized.id);
assert!(deserialized.verify_id());
```

---

## See Also

- [`07-builder-pattern.md`](07-builder-pattern.md) — complete builder API
- [`12-invariants.md`](12-invariants.md) — what each example enforces
- [`14-api-reference.md`](14-api-reference.md) — full API surface
