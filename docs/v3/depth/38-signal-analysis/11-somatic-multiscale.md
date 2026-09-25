# Somatic Signal Analysis and Emergent Multiscale Intelligence

> **Parent:** [38-SIGNAL-ANALYSIS](../../38-SIGNAL-ANALYSIS.md) Section 11

---

## Part I: Somatic Signal Analysis

### Somatic Markers as HDC Bindings

```rust
pub struct SomaticMarker {
    pub marker_hv: HdcVector,       // BIND(pattern, affect)
    pub pattern_hv: HdcVector,
    pub affect_hv: HdcVector,
    pub pleasure: f64,
    pub arousal: f64,
    pub dominance: f64,
    pub strength: f64,
    pub episode_sources: Vec<ContentHash>,
    pub created_at_ms: i64,
}
```

### PAD Encoding in HDC

```rust
pub fn encode_pad(
    pleasure: f64, arousal: f64, dominance: f64,
    codebook: &AffectCodebook,
) -> HdcVector {
    let p = codebook.pleasure_role.xor(&codebook.value_codebook.encode(pleasure));
    let a = codebook.arousal_role.xor(&codebook.value_codebook.encode(arousal));
    let d = codebook.dominance_role.xor(&codebook.value_codebook.encode(dominance));
    HdcVector::bundle(&[p, a, d])
}

pub struct AffectCodebook {
    pub pleasure_role: HdcVector,
    pub arousal_role: HdcVector,
    pub dominance_role: HdcVector,
    pub value_codebook: QuantizedCodebook,  // range [-1.0, 1.0], 32 levels
}
```

### Somatic Retrieval

```rust
pub fn somatic_retrieval(
    pattern: &HdcVector,
    somatic_map: &[SomaticMarker],
    threshold: f64,
    contrarian_fraction: f64,  // 0.15 per Bower (1981)
) -> SomaticAssessment {
    // Find markers with similar pattern component
    let matches: Vec<(f64, &SomaticMarker)> = somatic_map.iter()
        .filter_map(|marker| {
            let pattern_component = marker.marker_hv.xor(&marker.affect_hv);
            let similarity = pattern.hamming_similarity(&pattern_component);
            if similarity > threshold { Some((similarity, marker)) } else { None }
        })
        .collect();

    // Aggregate affect, weighted by similarity and strength
    let total_weight: f64 = matches.iter().map(|(sim, m)| sim * m.strength).sum();
    let avg_pleasure = matches.iter()
        .map(|(sim, m)| m.pleasure * sim * m.strength / total_weight).sum::<f64>();
    // ... arousal, dominance similarly

    // Mandatory 15% contrarian retrieval
    let contrarian_count = (matches.len() as f64 * contrarian_fraction).ceil() as usize;
    let contrarian_markers = find_contrarian_markers(
        pattern, somatic_map, avg_pleasure, contrarian_count
    );

    SomaticAssessment {
        valence: avg_pleasure,
        arousal: avg_arousal,
        dominance: avg_dominance,
        confidence: total_weight / matches.len().max(1) as f64,
        n_matching_markers: matches.len(),
        contrarian_markers,
    }
}
```

Cost: ~63ns per marker. For 1,000 markers: ~63us. Runs BEFORE analytical
prediction (System 1, Kahneman 2011).

### Marker Formation

```rust
pub fn form_somatic_marker(
    pattern: &HdcVector,
    outcome: &PredictionAccuracy,
    codebook: &AffectCodebook,
) -> SomaticMarker {
    let pleasure = if outcome.accuracy > 0.7 { 0.8 } else { -0.6 };
    let arousal = outcome.residual.abs();
    let dominance = outcome.accuracy;
    let affect_hv = encode_pad(pleasure, arousal, dominance, codebook);
    let marker_hv = pattern.xor(&affect_hv);
    SomaticMarker { marker_hv, pattern_hv: pattern.clone(), affect_hv,
        pleasure, arousal, dominance, strength: 1.0, /* ... */ }
}
```

### Marker Decay and Reinforcement

```rust
pub fn update_marker_strength(marker: &mut SomaticMarker, elapsed_ms: i64, reinforcement: Option<f64>) {
    // Ebbinghaus decay
    let lambda = 0.001;
    marker.strength *= (-lambda * elapsed_ms as f64).exp();
    // Reinforcement on re-experience
    if let Some(r) = reinforcement {
        marker.strength = (marker.strength + r).min(5.0);
    }
}
```

### Somatic Index (k-d tree)

For >5,000 markers, use k-d tree in 8D (3 PAD + 5 PCA of pattern vector):

```rust
pub struct SomaticIndex {
    tree: KdTree<f64, usize, 8>,
    pca_matrix: [[f64; 5]; 10_240],
    markers: Vec<SomaticMarker>,
}
```

Build: O(n log n). Query: O(log n) average. ~10x faster than linear scan at
n = 10,000.

---

## Part II: Emergent Multiscale Intelligence

### The 9 Signal Analysis Subsystems

| # | Subsystem | Contribution |
|---|---|---|
| 1 | HDC pattern algebra | Structural encoding + cross-domain matching |
| 2 | Adaptive signal metabolism | Evolutionary signal selection |
| 3 | Causal microstructure discovery | Causal reasoning (Pearl's 3 levels) |
| 4 | Predictive geometry (TDA) | Topological constraints |
| 5 | Resonant pattern ecosystem | Multi-signal competition |
| 6 | Adversarial robustness | Defense against manipulation |
| 7 | Somatic analysis | Pre-analytical "gut feelings" |
| 8 | Predictive foraging + active inference | Prediction-resolution-calibration |
| 9 | Sheaf-tropical geometry | Consistency + decision boundaries |

### Phi Computation

```rust
pub struct PhiComputer {
    subsystem_states: [SubsystemState; 9],
    flow_matrix: [[f64; 9]; 9],
}

impl PhiComputer {
    pub fn compute_phi(&self) -> PhiResult {
        let n = 9;
        let mut min_phi = f64::MAX;
        // Enumerate all 510 non-trivial bipartitions
        for mask in 1..(1u16 << n) - 1 {
            let complement = ((1u16 << n) - 1) ^ mask;
            let i_whole = self.integrated_information_whole();
            let i_a = self.integrated_information_part(&part_a);
            let i_b = self.integrated_information_part(&part_b);
            let delta_i = i_whole - i_a - i_b;
            if delta_i < min_phi { min_phi = delta_i; }
        }
        PhiResult { phi: min_phi, /* ... */ }
    }
}
```

510 bipartitions, O(81) each. Total: ~41K operations. Under 1ms.

### Information Flow Matrix

```rust
pub struct FlowMatrixComputer {
    pub lag_order: usize,     // 3
    pub window_size: usize,   // 100
    pub n_bins: usize,        // 10
}
```

Transfer entropy T(i -> j): reduction in uncertainty about j's next state
when knowing i's past, beyond j's own past.

### Partial Information Decomposition

```rust
pub struct PidAnalysis {
    pub redundancy: f64,
    pub unique_s1: f64,
    pub unique_s2: f64,
    pub synergy: f64,
}

impl PidAnalysis {
    pub fn compute(s1: &[f64], s2: &[f64], target: &[f64]) -> Self {
        let i_s1_t = mutual_information(s1, target);
        let i_s2_t = mutual_information(s2, target);
        let i_s1s2_t = joint_mutual_information(s1, s2, target);
        let redundancy = i_s1_t.min(i_s2_t);
        let unique_s1 = i_s1_t - redundancy;
        let unique_s2 = i_s2_t - redundancy;
        let synergy = i_s1s2_t - i_s1_t - i_s2_t + redundancy;
        PidAnalysis { redundancy, unique_s1, unique_s2, synergy }
    }
}
```

Synergy > 0.05 indicates meaningful emergent intelligence.

### Daimon Integration

```
THETA (every ~75s):
  Somatic assessment -> Daimon.pleasure += 0.3 * valence (if strong)

DELTA (hours):
  1. Compute 9x9 flow matrix
  2. Find MIB, compute Phi
  3. PID for all 36 pairs
  4. Phi > 0.5 -> Daimon.dominance += 0.2
  5. New synergy -> Daimon.arousal += 0.3
  6. Form somatic markers at synergistic boundaries
  7. Log to .roko/learn/phi.jsonl
```

## Error Handling

- Empty somatic map: neutral assessment (0, 0, 0, confidence=0)
- Zero weight in aggregation: neutral assessment, log warning
- Degenerate flow matrix: Phi = 0, MIB arbitrary
- Insufficient PID data: synergy = NaN, skip
- PCA failure (<5 patterns): reduce dims or linear scan

## Test Criteria

- **PAD round-trip**: encode then unbind recovers values within 0.1
- **Somatic retrieval**: positive marker for pattern A retrieves positive valence for similar patterns
- **Contrarian**: always returns ceil(n * 0.15) contrarian markers when available
- **Phi monotonicity**: adding strong connection does not decrease Phi
- **MIB exhaustiveness**: exactly 510 bipartitions evaluated
- **PID non-negativity**: redundancy, unique >= 0; synergy can be negative

## Academic Foundations

- Damasio, A. R. (1994). *Descartes' Error*.
- Mehrabian, A., & Russell, J. A. (1974). *An Approach to Environmental Psychology*.
- Bower, G. H. (1981). "Mood and Memory." *American Psychologist*, 36(2).
- Kahneman, D. (2011). *Thinking, Fast and Slow*.
- Tononi, G. (2004). "An information integration theory." *BMC Neuroscience*, 5(42).
- Tononi, G., et al. (2016). "Integrated information theory." *Nature Reviews Neuroscience*, 17(7).
- Williams, P. L., & Beer, R. D. (2010). "Nonnegative decomposition." arXiv:1004.2515.
