# HDC Knowledge Encoding

> **v3 depth -- 09-memory** | Source: v1/06-neuro/06

How knowledge entries are encoded as 10,240-bit HDC vectors for similarity
search, structured queries, and three-tier retrieval in Neuro.

Every Engram in Neuro carries a `fingerprint` field -- a 10,240-bit BSC
vector that encodes the record's semantic structure. The fingerprint is
produced by a deterministic default encoder at insert time, so similarity
search is native to the durable record rather than a side-table annotation.

---

## Encoding Pipeline

### Step 1: Concept vector generation

Map each concept to a deterministic hypervector via `HdcVector::from_seed()`.

### Step 2: Role vector assignment

Roles are named dimensions with fixed, deterministic vectors:
```rust
let role_kind    = HdcVector::from_seed(b"role:kind");
let role_content = HdcVector::from_seed(b"role:content");
let role_tag     = HdcVector::from_seed(b"role:tag");
let role_domain  = HdcVector::from_seed(b"role:domain");
```

### Step 3: Role-filler binding

Each attribute is encoded as a role-filler binding:
```rust
let kind_binding    = role_kind.bind(&HdcVector::from_seed(b"insight"));
let content_binding = role_content.bind(&text_fingerprint(&entry.content));
```

### Step 4: Tag encoding

Tags are bundled then bound to the tag role:
```rust
let tag_bundle = HdcVector::bundle(&tag_refs);
let tag_binding = role_tag.bind(&tag_bundle);
```

### Step 5: Final bundle

All role-filler bindings bundled into the entry's HDC vector:
```rust
let entry_hv = HdcVector::bundle(&[
    &kind_binding, &content_binding, &tag_binding,
]);
```

---

## Role Vector Registry

12 standard roles pre-populated at encoder construction:

| Role | Seed | Encodes |
|------|------|---------|
| `role:kind` | `b"role:kind"` | Knowledge type |
| `role:content` | `b"role:content"` | Content fingerprint |
| `role:tag` | `b"role:tag"` | Tag bundle |
| `role:domain` | `b"role:domain"` | Problem domain |
| `role:topic` | `b"role:topic"` | Specific topic |
| `role:risk_factor` | `b"role:risk_factor"` | What creates risk |
| `role:response` | `b"role:response"` | How to respond |
| `role:pattern` | `b"role:pattern"` | Observable signal |
| `role:severity` | `b"role:severity"` | Severity level |
| `role:temporal` | `b"role:temporal"` | Time dimension |
| `role:confidence` | `b"role:confidence"` | Certainty level |
| `role:source` | `b"role:source"` | Information source |

---

## Structured Queries

Because bind distributes over bundle, HDC enables **structured queries**:

**"What entries are about Rust?"**
```rust
let query = HdcVector::from_seed(b"role:domain")
    .bind(&HdcVector::from_seed(b"rust"));
// Compare against all stored entry vectors
```

**Unbinding for decomposition:**
```rust
let domain_signal = entry_hv.bind(&HdcVector::from_seed(b"role:domain"));
// Nearest codebook match reveals the entry's domain
```

**Multi-attribute queries:**
```rust
let combined = HdcVector::bundle(&[&kind_query, &topic_query]);
// Entries matching BOTH attributes have highest similarity
```

Threshold: 0.526 for cross-domain (Bonferroni-corrected for 100K); 0.52 for
within-domain (single-pair).

---

## CausalLink Encoding

CausalLinks use asymmetric permutation for directionality:

```rust
let cause_binding = role_cause.permute(1).bind(&hv_cause);
let effect_binding = role_effect.permute(2).bind(&hv_effect);
```

This ensures `CAUSE->EFFECT` differs from `EFFECT->CAUSE`.

---

## Code Symbol Fingerprinting (roko-index)

The `roko-index/src/hdc.rs` module uses trigram-based name encoding:

```rust
fn encode_name(name: &str) -> HdcVector {
    let padded = format!("__{name}__");
    let trigrams: Vec<HdcVector> = padded.as_bytes()
        .windows(3)
        .enumerate()
        .map(|(pos, trigram)| {
            HdcVector::from_seed(trigram).permute(pos)
        })
        .collect();
    HdcVector::bundle(&trigram_refs)
}
```

Names sharing substrings have moderate similarity: `parse_config` and
`parse_input` share `par`, `ars`, `rse` trigrams.

---

## Episode Compression

Bundle multiple entries into a summary vector for fast pre-filtering:

```rust
pub fn compress_episode(entries: &[&KnowledgeEntry]) -> Option<HdcVector> {
    let hvs: Vec<HdcVector> = entries.iter()
        .filter_map(|e| e.fingerprint.as_ref().map(|fp| fp.vector))
        .collect();
    if hvs.is_empty() { return None; }
    Some(HdcVector::bundle(&hvs.iter().collect::<Vec<_>>()))
}
```

Quality gate: mean pairwise similarity > 0.52 means entries are too similar
(bundle collapses to single concept). Below 0.45 means good diversity.

---

## What the Fingerprint Enables

Beyond nearest-neighbor lookup, the same vector supports:

- **Consensus**: Bundle fingerprints of multiple candidates to check
  convergence on the same structure
- **Analogy**: Bind role vectors and compare across domains for structural
  matches
- **Tier progression**: Cluster similar Insight fingerprints before
  promotion, promoting coherent neighborhoods rather than isolated entries

---

## Cross-References

- `hdc-vsa-foundations.md` -- mathematical foundations
- `hdc-operations.md` -- operation details
- `cross-domain-hdc-transfer.md` -- cross-domain structural analogy
- `false-positive-math.md` -- similarity threshold selection
