# 33-02 -- Memory Functor (F_memory)

> **Parent**: [33-CROSS-CUTS](../../33-CROSS-CUTS.md)
>
> `MemoryFunctor` enriches the cognitive loop with durable knowledge from the
> `KnowledgeStore`. It injects retrieved entries on Sense and Compose, and applies
> gate-outcome feedback on React. This depth file covers the retrieval pipeline,
> gate feedback protocol, and short-circuit semantics.

**Authority**: `crates/roko-compose/src/memory_functor.rs`

---

## 1. Construction

```rust
pub struct MemoryFunctor {
    store: Arc<KnowledgeStore>,
    max_entries: usize,              // default: 10
    included: Mutex<HashMap<(String, String), Vec<String>>>,
    last_query_empty: AtomicBool,
}
```

The functor wraps a shared `Arc<KnowledgeStore>` without taking ownership. The
`max_entries` cap prevents knowledge injection from consuming the entire context
budget. The `included` map tracks which entries were in context for each (plan, task)
pair so that React feedback targets exactly the right entries.

---

## 2. Retrieval Pipeline

### 2.1 Topic Extraction

The retrieval topic is derived from input Signals. The functor collects text from
all `Kind::Task` Signals, concatenating `Body::Text` and `Body::Json` contents.
If no Task Signals are present, the context's `task_id` is used as the topic.

### 2.2 Keyword Retrieval

Without the `hdc` feature, retrieval uses `KnowledgeStore::query_hits()`, which
performs keyword matching and returns scored hits. Results are ordered by
`total_score` and capped at `max_entries`.

### 2.3 HDC Similarity Retrieval

With the `hdc` feature enabled, the functor runs both keyword and HDC retrieval:

1. Keyword hits are collected by entry ID.
2. An HDC fingerprint is computed from the topic text via `text_fingerprint()`.
3. `KnowledgeStore::query_similar()` returns similarity-scored hits.
4. Results are merged by entry ID, keeping the higher score.
5. The merged list is sorted by score (descending) and truncated.

This dual-path retrieval catches both exact keyword matches and semantic
similarities that keywords miss.

### 2.4 Signal Construction

Each matched entry becomes a `Kind::Insight` Signal with:

| Field | Source |
|-------|--------|
| `knowledge_id` | Entry ID |
| `content` | Entry content text |
| `source` | Entry source attribution |
| `tier` | KnowledgeTier label (transient/working/consolidated/persistent) |
| `retrieval` | Method used ("keyword" or "hdc") |
| `relevance` | Match score |
| `demurrage_balance` | Current entry balance |

The Signal's provenance carries the entry's `origin_taint` and `classification`
levels, propagating trust metadata through the IFC lattice.

---

## 3. Compose-Phase Bid Tags

On `LoopStep::Compose`, each result Signal is additionally tagged for cross-cut
arbitration:

| Tag | Value | Purpose |
|-----|-------|---------|
| `attention_bidder` | `"neuro"` | Identifies the bidder for prompt-budget allocation |
| `recommendation_source` | `"memory"` | Cross-cut ID for arbitration |
| `decision_kind` | `"compose"` | Decision type (VCG-eligible) |
| `decision_key` | `"prompt_context"` | Conflict group key |
| `priority_level` | `"2"` | Memory is priority 2 (below Daimon, above Dreams) |
| `recommendation_confidence` | Entry's confidence | Truthful bid value |
| `recommendation_value` | Entry ID | Concrete recommendation |

These tags make the entry parseable by `CrossCutRecommendation::from_signal()` and
eligible for priority/VCG resolution when it conflicts with a Dreams recommendation.

---

## 4. Gate Feedback (React Phase)

### 4.1 Verdict Detection

The post-enrichment hook on `LoopStep::React` scans output Signals in reverse order
for the first `Kind::GateVerdict`. The verdict is read from:

1. The `"verdict"` tag, if present.
2. The JSON body's `"passed"` boolean field.
3. The JSON body's `"verdict"` or `"status"` string field.

Label matching is case-insensitive: "pass"/"passed"/"success" map to true;
"fail"/"failed"/"failure" map to false.

### 4.2 Reinforcement (Gate Passed)

When the gate passed, the functor retrieves the entry IDs from the `included` map
(removing them atomically) and calls `KnowledgeStore::reinforce_batch()` with
`ReinforcementSignal::Gated`. This increases the entries' balance and confidence,
extending their effective lifespan against demurrage decay.

### 4.3 Weakening (Gate Failed)

When the gate failed, the functor:

1. Calls `score_prediction_utility()` with `correct = false` and `accuracy = 0.0`,
   reducing the entries' confidence.
2. Calls `batch_record_usage()` with `success = false` on a mutable clone of the
   store, recording unsuccessful usage in the prediction-utility state.

Both mutations decrease the entries' effective weight, making them less likely to be
retrieved in future queries and more susceptible to GC via demurrage.

---

## 5. Short-Circuit Semantics

`should_short_circuit()` returns true in two cases:

1. **Last query was empty**: the `AtomicBool` flag is set during `pre_enrich` when
   `query()` returns zero results.
2. **Store is empty**: `read_all()` returns an empty list (or errors).

This hint is an optimization for `EnrichedCell` callers. When true, the caller may
skip invoking this functor entirely, saving the overhead of store queries. The flag
is updated on every Sense/Compose invocation, so it tracks the live state.

---

## 6. Key Tests

| Test | What it verifies |
|------|-----------------|
| `sense_and_compose_query_real_store_with_metadata_and_neuro_bid` | Real store retrieval produces correct tier/retrieval tags and Neuro bid annotations |
| `react_reinforces_passes_and_weakens_failures_in_real_store` | Gate pass increases balance; gate fail decreases both balance and confidence |
| `empty_query_sets_short_circuit_hint_without_dropping_input` | Unrelated query sets the short-circuit flag without removing input Signals |

---

## References

- See [09-MEMORY](../../09-MEMORY.md) for the full KnowledgeStore specification.
- See [01-SIGNAL](../../01-SIGNAL.md) for Signal struct and provenance semantics.
