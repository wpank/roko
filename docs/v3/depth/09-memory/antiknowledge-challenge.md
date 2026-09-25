# AntiKnowledge and the Challenge Mechanism

> **v3 depth -- 09-memory** | Source: v1/06-neuro/11

AntiKnowledge -- validated negative knowledge about what is wrong -- serves
as the epistemic immune system of Neuro, with a confidence floor of 0.3,
0.5x demurrage, and a challenge mechanism that links refutations to the
entries they contradict.

---

## The Challenge Mechanism

### Creating an AntiKnowledge entry

When an existing entry is contradicted by strong evidence -- typically a gate
failure or a direct observation:

```rust
let anti = KnowledgeEntry {
    id: format!("anti_{}", uuid::Uuid::new_v4()),
    kind: KnowledgeKind::AntiKnowledge,
    content: "Moving to async doesn't always improve throughput -- \
              CPU-bound workloads degrade due to task scheduling overhead".into(),
    confidence: 0.6,
    refuted_insight_id: Some("ke_original_async_insight".into()),
    refutation_evidence: Some("Benchmark showed 15% throughput regression".into()),
    ..Default::default()
};
```

### The refutation warning

```rust
impl KnowledgeEntry {
    pub fn refutation_warning(&self) -> Option<String> {
        if self.kind != KnowledgeKind::AntiKnowledge { return None; }
        let refuted_id = self.refuted_insight_id.as_deref()?.trim();
        if refuted_id.is_empty() { return None; }
        let evidence = self.refutation_evidence
            .as_deref()
            .unwrap_or(self.content.as_str())
            .trim()
            .trim_end_matches(|ch| matches!(ch, '.' | '!' | '?'));
        if evidence.is_empty() { return None; }
        Some(format!("Previous insight {refuted_id} was wrong because {evidence}."))
    }
}
```

### Retrieval integration

When context is assembled:
1. Retrieve relevant entries by topic
2. For each retrieved entry, check if any AntiKnowledge refutes it
3. If a refutation exists, attach the warning
4. The agent sees both the original claim and the counterevidence

---

## Special Decay Properties

### Confidence floor: 0.3

```rust
const ANTI_KNOWLEDGE_FLOOR: f64 = 0.3;

pub fn apply_decay(entry: &mut KnowledgeEntry, elapsed_days: f64) {
    let decay_factor = (-elapsed_days / entry.half_life_days).exp();
    let raw = entry.confidence * decay_factor;
    entry.confidence = if entry.kind == KnowledgeKind::AntiKnowledge {
        raw.max(ANTI_KNOWLEDGE_FLOOR)
    } else { raw };
}
```

### Half-speed demurrage (on-chain)

Standard: 1% annual. AntiKnowledge: 0.5% annual.

### GC exemption

AntiKnowledge entries with confidence >= 0.3 are exempt from the 0.05 GC
threshold.

---

## Reactive Checking

New candidates are compared against existing AntiKnowledge at ingestion:

```rust
pub fn reactive_anti_check(&self, candidate: &KnowledgeEntry) -> ReactiveCheckResult {
    // For each AntiKnowledge entry:
    //   if HDC_similarity(candidate, anti) > 0.526:
    //     flag candidate as potentially refuted
    //     require additional confirmation before promotion
}
```

---

## Memetic Evolution: Epistemic Parasites

### Dawkinsian fitness

```
W(E) = f * r * L   (fidelity * fecundity * longevity)
```

A **parasite** has high fitness but negative decision quality -- it is
frequently retrieved and persistent but actually harms outcomes.

### Detection

```rust
pub fn detect_parasites(entries, stats, outcomes) -> Vec<String> {
    // High fitness (retrieved often, persistent) AND
    // Negative decision quality (outcomes worse when used)
}
```

### Price equation diagnostics

```
delta(mean_fitness) = Cov(fitness, frequency) + E(delta_fitness)
```

- selection > 0: healthy knowledge base
- transmission < 0: distillation degrading quality
- selection < 0: bad entries preferentially selected

---

## Knowledge Immune System

### Level 1: Innate immunity (always active)

Bloom filter of known-bad fingerprints, max external confidence cap (0.7),
anomaly detection (mean similarity < 0.48 triggers quarantine).

### Level 2: Adaptive immunity (learned)

Corruption pattern prototypes bundled by infection vector category. Updated
during Dreams consolidation.

### Level 3: Active immunity (periodic audits)

Health audit during Dreams: parasite detection, Price equation diagnostics,
confirmation cascade risk scoring.

---

## Automatic Generation from Gate Failures

```rust
pub fn generate_anti_from_gate_failure(
    failed_gate: &GateResult,
    retrieved_entries: &[KnowledgeEntry],
    task_context: &str,
) -> Option<KnowledgeEntry> {
    // Attribute failure to highest-confidence retrieved entry
    // Create AntiKnowledge with refuted_insight_id set
    // Initial confidence: 0.5 (single failure), 0.6 (two), 0.7 (three+)
}
```

---

## Academic Foundations

- Dawkins, R. (1976). *The Selfish Gene*. Oxford. (Memetic replicator model)
- Price, G. R. (1970). "Selection and covariance." *Nature*, 227.
- Popper, K. (1963). *Conjectures and Refutations*. Routledge.
- Kahneman, D. & Tversky, A. (1979). "Prospect Theory." *Econometrica*.
- Kuhn, T. S. (1962). *The Structure of Scientific Revolutions*. Chicago.
- Proctor, R. N. (2008). *Agnotology*. Stanford.

---

## Cross-References

- `six-knowledge-types.md` -- AntiKnowledge as sixth type
- `ebbinghaus-decay-with-tier.md` -- decay mechanics
- `4-tier-distillation-pipeline.md` -- distillation interaction
