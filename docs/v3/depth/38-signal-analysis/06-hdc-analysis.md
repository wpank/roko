# Hyperdimensional Signal Analysis

> **Parent:** [38-SIGNAL-ANALYSIS](../../38-SIGNAL-ANALYSIS.md) Section 6

---

## Pattern Algebra

### Role-filler composition

```rust
pub fn encode_observation(role: &HdcVector, filler: &HdcVector) -> HdcVector {
    role.xor(filler)
}

pub fn encode_ta_state(observations: &[(HdcVector, HdcVector)]) -> HdcVector {
    let bound: Vec<HdcVector> = observations.iter()
        .map(|(role, filler)| role.xor(filler))
        .collect();
    HdcVector::bundle(&bound)
}
```

### Temporal composition

```rust
pub fn encode_temporal_pattern(observations: &[HdcVector]) -> HdcVector {
    let permuted: Vec<HdcVector> = observations.iter()
        .enumerate()
        .map(|(i, obs)| obs.permute(i as u32))
        .collect();
    HdcVector::bundle(&permuted)
}
```

### Shift-invariant pattern matching

```rust
pub fn shift_invariant_match(
    pattern: &HdcVector,
    sequence: &[HdcVector],
    pattern_len: usize,
) -> (f64, usize) {
    let mut best_similarity = 0.0;
    let mut best_offset = 0;
    for offset in 0..=(sequence.len() - pattern_len) {
        let window = &sequence[offset..offset + pattern_len];
        let window_encoded = encode_temporal_pattern(window);
        let similarity = pattern.hamming_similarity(&window_encoded);
        if similarity > best_similarity {
            best_similarity = similarity;
            best_offset = offset;
        }
    }
    (best_similarity, best_offset)
}
```

## Domain Codebooks

### Coding codebook

```rust
pub struct CodingCodebook {
    // Event type roles
    pub commit: HdcVector,
    pub build: HdcVector,
    pub test_run: HdcVector,
    pub lint: HdcVector,
    pub benchmark: HdcVector,
    pub deploy: HdcVector,
    pub review: HdcVector,
    pub merge: HdcVector,

    // Metric roles
    pub complexity: HdcVector,
    pub coverage: HdcVector,
    pub pass_rate: HdcVector,
    pub build_time: HdcVector,
    pub error_count: HdcVector,
    pub churn_rate: HdcVector,

    // Scope roles
    pub file: HdcVector,
    pub module: HdcVector,
    pub crate_scope: HdcVector,
    pub workspace: HdcVector,

    // Numeric codebooks
    pub count_codebook: QuantizedCodebook,
    pub rate_codebook: QuantizedCodebook,
    pub duration_codebook: QuantizedCodebook,
}
```

All codebooks use the same HDC algebra (10,240-bit BSC, XOR bind, majority
bundle), so patterns from any domain can be compared directly via Hamming
similarity.

## Quantized Numeric Encoding

```rust
pub struct QuantizedCodebook {
    levels: Vec<HdcVector>,
    min: f64,
    max: f64,
    n_levels: usize,
}

impl QuantizedCodebook {
    pub fn encode(&self, value: f64) -> HdcVector {
        let normalized = (value - self.min) / (self.max - self.min);
        let level = (normalized * self.n_levels as f64).clamp(0.0, (self.n_levels - 1) as f64);
        let lower = level.floor() as usize;
        let upper = (lower + 1).min(self.n_levels - 1);
        let weight = level - lower as f64;
        self.levels[lower].weighted_bundle(&self.levels[upper], 1.0 - weight, weight)
    }
}
```

Thermometer construction: `flip_count = dim / (2 * n_levels)` ensures adjacent
levels have Hamming similarity ~= 1 - 1/(2*n_levels). Preserves ordinal
relationships.

| Parameter | Default | Range | Notes |
|---|---|---|---|
| `dim` | 10,240 | 1,024 - 65,536 | Multiple of 64 for SIMD. 160 u64 words. |
| `n_levels` | 64 | 8 - 256 | 64 gives ~1.5% resolution |
| `flip_count` | dim / (2 * n_levels) | derived | Controls inter-level similarity |

## Codebook Generation

```rust
pub struct CodebookGenerator {
    seed: [u8; 32],   // SHA-256 of domain name
    dim: usize,
}

impl CodebookGenerator {
    pub fn new(domain: &str, dim: usize) -> Self {
        Self { seed: sha256(domain.as_bytes()), dim }
    }

    pub fn generate_role(&self, index: u32) -> HdcVector {
        let mut key = self.seed.to_vec();
        key.extend_from_slice(&index.to_le_bytes());
        let mut rng = ChaCha20Rng::from_seed(sha256(&key));
        HdcVector::random(&mut rng, self.dim)
    }

    pub fn generate_quantized(
        &self, codebook_index: u32, n_levels: usize, min: f64, max: f64,
    ) -> QuantizedCodebook {
        let flip_count = self.dim / (2 * n_levels);
        let base = self.generate_role(codebook_index);
        let mut levels = vec![base];
        for k in 1..n_levels {
            let prev = &levels[k - 1];
            let mut rng = ChaCha20Rng::from_seed(
                sha256(&[&self.seed[..], &(codebook_index + k as u32).to_le_bytes()].concat())
            );
            levels.push(prev.flip_random_bits(flip_count, &mut rng));
        }
        QuantizedCodebook { levels, min, max, n_levels }
    }
}
```

Deterministic from seed: all agents sharing a domain seed produce identical
codebooks without coordination.

## Cross-Domain Pattern Matching

```rust
pub fn detect_cross_domain_resonance(
    new_engram: &Engram,
    all_domain_knowledge: &[Engram],
    threshold: f64,
) -> Vec<CrossDomainInsight> {
    let new_hv = new_engram.hdc_vector();
    let new_domain = new_engram.domain();
    all_domain_knowledge.iter()
        .filter(|k| k.domain() != new_domain)
        .filter_map(|k| {
            let sim = new_hv.hamming_similarity(&k.hdc_vector());
            if sim > threshold {
                Some(CrossDomainInsight {
                    source_domain: new_domain.clone(),
                    target_domain: k.domain().clone(),
                    similarity: sim,
                })
            } else { None }
        })
        .collect()
}
```

Threshold 0.526: with 10,240-bit vectors, random similarity is 0.500 with
stddev ~0.005. Threshold at 5.26 sigma (p < 1e-7).

### Threshold calibration

```rust
pub fn calibrate_threshold(dim: usize, sigma_level: f64, n_samples: usize) -> (f64, f64, f64) {
    let mut similarities = Vec::with_capacity(n_samples);
    for _ in 0..n_samples {
        let a = HdcVector::random(&mut rng, dim);
        let b = HdcVector::random(&mut rng, dim);
        similarities.push(a.hamming_similarity(&b));
    }
    let mean = similarities.iter().sum::<f64>() / n_samples as f64;
    let variance = similarities.iter().map(|s| (s - mean).powi(2)).sum::<f64>() / n_samples as f64;
    let stddev = variance.sqrt();
    (mean, stddev, mean + sigma_level * stddev)
}
```

## Pattern Store

```rust
pub struct PatternStore {
    patterns: HashMap<OracleDomain, Vec<StoredPattern>>,
    cross_domain_index: Vec<(OracleDomain, HdcVector, ContentHash)>,
}

pub struct StoredPattern {
    pub vector: HdcVector,
    pub source_engram: ContentHash,
    pub outcome: Option<PredictionOutcome>,
    pub frequency: u64,
    pub reliability: f64,
}
```

### Pruning rules (applied in order)

1. **Unreliable**: reliability < 0.3 and frequency >= 10
2. **Stale**: not matched in max_staleness (default 72h)
3. **Redundant**: similarity > 0.95, keep higher reliability
4. **LRU eviction**: remove least-recently-matched to budget

```rust
pub struct PruneConfig {
    pub max_patterns_per_domain: usize,  // 100,000
    pub min_reliability: f64,            // 0.3
    pub min_frequency: u64,              // 10
    pub max_staleness: Duration,         // 72 hours
    pub dedup_threshold: f64,            // 0.95
}
```

### CBOR serialization

PatternStore persists to CBOR (RFC 8949). File size: ~1,325 bytes per pattern.
100K patterns ~= 130 MB.

## Dreams Integration

During Delta consolidation:
1. NREM replays high-value patterns, updates reliability (+0.05 agree, -0.10 disagree)
2. REM generates novel compositions via crossover with noise
3. Pruning removes unreliable patterns

Asymmetric update: trust is hard to earn and easy to lose.

## Test Criteria

- **Codebook determinism**: same domain + dim produces identical vectors
- **Quantized monotonicity**: for v1 < v2, similarity(encode(v1), encode(v2)) decreases with |v2 - v1|
- **Threshold calibration**: null distribution mean within 0.001 of 0.500
- **Cross-domain routing**: pattern stored by one oracle is retrievable via find_similar(domain: None)
- **CBOR round-trip**: deserialize(serialize(store)) produces identical store
- **Pruning correctness**: no surviving pattern violates thresholds

## Academic Foundations

- Kanerva, P. (2009). "Hyperdimensional Computing." *Cognitive Computation*, 1(2), 139-159.
- Kleyko, D., et al. (2022). "A Survey on Hyperdimensional Computing." *ACM Computing Surveys*, 54(6).
- Plate, T. A. (1995). "Holographic Reduced Representations." *IEEE Trans. Neural Networks*, 6(3).
- Frady, E. P., et al. (2018). "A Theory of Sequence Indexing." *Neural Computation*, 30(6).
