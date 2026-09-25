# 00-ARCH -- Performance and Numerical Stability

> **Parent**: [00-ARCHITECTURE](../../00-ARCHITECTURE.md)
>
> Roko performs numerical computation throughout: decay curves, scoring, HDC
> similarity, bandit arm selection, EMA threshold adaptation, cost normalization,
> prospect-theory valuation, E44 functor enrichment, and observability aggregation.
> This depth file specifies complexity, precision targets, normalization rules,
> NaN/Inf/underflow handling, instrumentation overhead budgets, and benchmark criteria.

---

## 1. Precision Targets

### 1.1 f32 vs. f64 Decision Matrix

| Domain | Type | Rationale |
|---|---|---|
| Decay weights | f32 | Range [0.0, 1.0], 7 decimal digits sufficient |
| Score axes | f32 | Range [-1.0, 1.0], 7 decimal digits sufficient |
| PAD vector | f32 | Range [-1.0, 1.0], psychometric precision |
| HDC vectors | u64 bitfield | Binary (0/1 per dimension); no floating point |
| Cost tracking | f64 | USD amounts accumulate; f32 loses cents past $16,777.216 |
| Bandit arm parameters | f64 | UCB/Thompson precision matters for convergence |
| EMA thresholds | f64 | Small alpha values (0.05) compound rounding errors in f32 |
| Prospect-theory values | f64 | Exponentiation with alpha=0.88 requires f64 for stability |
| Timestamps | i64 | Millisecond Unix timestamps; u64 would also work |
| Token counts | usize | Platform-native integer; no floating point |
| Metric counters | u64 | Monotonic counts should not round through floating-point |
| Histogram bucket bounds | f64 | Stable bucket boundaries must survive repeated serialization |
| Trace/log queue watermarks | usize | Queue depth is an integer occupancy, not an estimate |

**Rule**: Use f32 for per-Signal fields stored at high volume, f64 for aggregate
statistics and telemetry values that accumulate over time, and integers for counters,
token totals, timestamps, and queue depths.

### 1.2 Serialization Precision

```rust
fn round_f32(v: f32, decimals: u32) -> f32 {
    let factor = 10_f32.powi(decimals as i32);
    (v * factor).round() / factor
}
```

| Domain | Decimal places | Example |
|---|---|---|
| Decay weight | 6 | 0.707107 |
| Score axis | 4 | 0.8500 |
| Cost (USD) | 4 | 12.3456 |
| EMA threshold | 6 | 0.654321 |
| Exploration rate | 4 | 0.1000 |
| C-factor gauges | 4 | 0.8125 |
| Demurrage ratio | 6 | 0.002500 |
| Prospect-theory value | 4 | -0.4532 |

### 1.3 Telemetry Exposition Rules

| Surface | Internal | Exposed | Precision rule |
|---|---|---|---|
| Monotonic counters | u64 | integer sample | Never round through f32; reset only on restart |
| Duration histograms | u64 ns/ms, f64 bounds | seconds | Convert units once at exposition boundary |
| Cost metrics | f64 USD | decimal | 4 decimal places in logs and projections |
| Calibration gauges | f64 | decimal | 4 decimals for display, full precision in memory |
| Demurrage ratios | f64 | decimal | 6 decimals for display |
| Trace timestamps | i64/u64 monotonic | RFC3339 + monotonic delta | Wall clock for display; monotonic for ordering |

Telemetry-specific rules:

1. Percentile dashboards derive from histograms or StateHub projections, not from
   ad hoc rounded gauges
2. Histogram bucket boundaries are fixed at startup; do not synthesize buckets from
   live data
3. Durations in Prometheus-compatible metrics are exposed in seconds even if the
   runtime stores them in milliseconds or nanoseconds

---

## 2. Algorithm Complexity

### 2.1 Decay

| Operation | Time | Space | Notes |
|---|---|---|---|
| `Decay::apply(age_ms)` | O(1) | O(1) | Single `powf` or `exp` call |
| `Decay::is_alive(age_ms, threshold)` | O(1) | O(1) | Comparison after `apply` |
| `Substrate::prune(threshold)` | O(n) | O(1) | Linear scan, in-place removal |

**Decay formulas**:

```
HalfLife:    w(t) = 0.5^(t / half_life_ms)
Ebbinghaus:  w(t) = exp(-t / (strength * scale_ms))
TTL:         w(t) = if t < ttl_ms { 1.0 } else { 0.0 }
None:        w(t) = 1.0
```

### 2.2 Scoring

| Operation | Time | Space | Notes |
|---|---|---|---|
| `Score::effective()` | O(1) | O(1) | Weighted sum of 7 axes |
| `Scorer::score(datum)` | O(k) | O(1) | k = number of scoring rules |
| Batch score all data | O(n * k) | O(n) | n = Signals, k rules per item |

### 2.3 HDC Vectors

| Operation | Time | Space | Notes |
|---|---|---|---|
| Bind (XOR) | O(d/64) | O(d/64) | d = 10,240 bits = 160 u64s |
| Bundle (majority) | O(d * m) | O(d/64) | m = number of vectors |
| Hamming distance | O(d/64) | O(1) | `popcnt` on XOR result |
| Nearest neighbor (brute) | O(n * d/64) | O(1) | n = stored vectors |

**Performance note**: For d = 10,240 and n = 10,000 knowledge entries:

```
nearest-neighbor = 10,000 * 160 popcnt = 1.6M operations
on modern CPU (~10B ops/sec): ~0.16ms
```

No index structure is needed below 100,000 entries.

### 2.4 Bandit Selection

| Operation | Time | Space | Notes |
|---|---|---|---|
| UCB1 select | O(a) | O(a) | a = number of arms |
| Thompson sampling (Beta) | O(a) | O(a) | One Beta sample per arm |
| LinUCB select | O(a * d^2) | O(a * d^2) | d = context feature dimension |
| Bandit update | O(d^2) | O(1) | Matrix update for LinUCB |

For LinUCB with a = 10 arms and d = 8 features: select takes ~640 multiply-adds.

### 2.5 EMA Threshold Adaptation

```
new_threshold = alpha * sample + (1 - alpha) * old_threshold
```

| Operation | Time | Space | Notes |
|---|---|---|---|
| EMA update | O(1) | O(1) | Single multiply-add |
| Read threshold | O(1) | O(1) | Field access |
| Persist to JSON | O(r) | O(r) | r = number of rungs (7) |

### 2.6 Prompt Assembly

| Operation | Time | Space | Notes |
|---|---|---|---|
| Section collection | O(s) | O(s) | s = sections (~9) |
| Priority sort | O(s log s) | O(1) | In-place sort |
| Budget allocation | O(s) | O(s) | Single pass after sort |
| Token counting | O(t) | O(1) | t = total tokens |
| Full assembly | O(s log s + t) | O(s + t) | Dominated by token counting |

### 2.7 E44 Functor Enrichment

| Operation | Time | Space | Notes |
|---|---|---|---|
| EnrichedCell.execute() | O(f * n) | O(n) | f = functors (4), n = signals |
| MemoryFunctor query | O(k + h) | O(k + h) | k = keyword hits, h = HDC hits |
| DaimonFunctor assess | O(1) | O(1) | PAD read + somatic query |
| SafetyFunctor filter | O(n) | O(n) | Per-signal capability check |
| Gate-failure cascade | O(f * n) | O(n) | 6 natural transformations |

### 2.8 Cascade Router

| Operation | Time | Space | Notes |
|---|---|---|---|
| Candidate scoring | O(c) | O(c) | c = candidate models |
| Static table lookup | O(1) | O(r * k) | r = roles, k = complexity bands |
| Bandit arm selection | O(c) | O(c) | Falls through to bandit |
| Full route decision | O(c) | O(c) | Dominated by candidate scoring |

---

## 3. Normalization Rules

### 3.1 Score Normalization

All seven Score axes are in [-1.0, 1.0]. The `effective()` method:

```rust
pub fn effective(&self) -> f32 {
    const W: [f32; 7] = [0.25, 0.20, 0.15, 0.15, 0.10, 0.10, 0.05];
    let axes = [
        self.relevance, self.confidence, self.urgency,
        self.novelty, self.salience, self.coherence, self.surprise,
    ];
    axes.iter().zip(W.iter()).map(|(a, w)| a * w).sum::<f32>().clamp(-1.0, 1.0)
}
```

Weights sum to 1.0, so `effective()` is in [-1.0, 1.0].

### 3.2 Cost Normalization

```
normalized_cost_usd = input_tokens * price_per_input_token
                    + output_tokens * price_per_output_token
```

Prices stored as f64 in dollars per token (not per million tokens) to avoid
off-by-1000 errors.

### 3.3 Reward Normalization

```
quality = gate_pass_rate                     (in [0, 1])
cost    = 1.0 - (actual / budget)            (inverted: cheaper = higher)
latency = 1.0 - (actual / sla)              (inverted: faster = higher)

reward = quality_weight * quality + cost_weight * cost + latency_weight * latency
```

Clamped to [0.0, 1.0].

### 3.4 Prospect-Theory Valuation

The DaimonFunctor (E44) applies Kahneman & Tversky (1979) prospect-theory valuation:

```
V(x) = x^alpha           if x >= 0   (alpha = 0.88)
V(x) = -lambda|x|^alpha  if x < 0   (lambda = 2.25)
```

Output clamped to [-10.0, 10.0] to prevent extreme values.

---

## 4. NaN/Inf/Underflow Handling

### 4.1 Sources and Mitigations

| Operation | Problem | Mitigation |
|---|---|---|
| `0.0 / 0.0` | NaN | Check denominator before division |
| `exp(710.0_f64)` | Inf overflow | Clamp input to `exp()` at 700.0 |
| `1.0 / 0.0` | Inf | Check denominator; return default |
| `powf(0.5, 0.0/0.0)` | NaN propagation from half_life_ms=0 | Guard: `if half_life_ms == 0 { return 0.0; }` |
| Ebbinghaus with tiny strength | Underflow to 0.0 | Correct behavior (fully decayed) |
| Prospect value with extreme delta | Large magnitude | Clamp output to [-10.0, 10.0] |
| `(-x).powf(0.88)` with negative x | NaN from fractional power of negative | Guard: use `|x|` then negate |

### 4.2 Defensive Pattern

```rust
fn safe_compute(input: f32) -> f32 {
    let result = /* computation */;
    if result.is_nan() || result.is_infinite() {
        tracing::warn!(input, result = %result, "numerical anomaly, clamping");
        return DEFAULT_VALUE;
    }
    result.clamp(MIN, MAX)
}
```

Apply at computation boundaries, not at every intermediate step. Clamping intermediate
results can mask bugs.

### 4.3 Telemetry Failure Modes

| Failure mode | Cause | Mitigation |
|---|---|---|
| Negative rate after restart | Counter reset interpreted as decrease | Reset-aware counters; annotate restarts |
| Nonsensical latency histograms | Mixed ms/seconds units | Convert units once in the sink |
| Cardinality explosion | Unbounded identifiers as labels | Reject or hash into logs only |
| Sample bias from sink saturation | Log/trace queues drop under pressure | Expose drop counters and queue depth |
| False precision in replay | Partial Pulse history | Mark as `exact`, `reconstructed`, or `partial` |

---

## 5. Profiling Targets

### 5.1 Hot Paths

| Path | Budget | Notes |
|---|---|---|
| `Decay::apply()` | < 10ns | Inline candidate |
| `Score::effective()` | < 50ns | 7-float weighted sum |
| HDC Hamming distance | < 1us | 160 `popcnt` ops |
| Prompt assembly | < 5ms | Token counting dominates |
| Gate (compile) | < 60s | External subprocess |
| Gate (test) | < 300s | External subprocess |
| Cascade router select | < 100us | Candidate scoring + bandit |
| Episode log write | < 1ms | JSONL append |
| Metric counter increment | < 250ns | No heap allocation |
| EnrichedCell.execute() | < 10ms | Functor chain overhead |
| SafetyFunctor filter | < 100us | Per-signal cap check |
| StateHub telemetry fold | < 1ms per delta | Must not stall Bus consumers |
| Replay scan throughput | > 50,000 records/s | Postmortem and test replay |

**Instrumentation budget**: End-to-end observability overhead below 5% CPU on
Gamma-speed interactive turns, below 10% on Theta/Delta batch work.

### 5.2 Memory Budgets

| Component | Target | Notes |
|---|---|---|
| Signal header (in memory) | < 1 KB | Body is variable |
| HDC vector | 1.25 KB | 10,240 bits = 1,280 bytes |
| Knowledge entry | < 2 KB | HDC + metadata |
| Episode record | < 4 KB | Compressed JSON |
| Config (loaded) | < 64 KB | All structs combined |
| Cascade router state | < 256 KB | All arm parameters |
| Metrics registry | < 16 MB/process | Active series + buckets |
| Log queue | 8-32 MB/process | Bounded; drop low-priority first |
| Trace export queue | 8-32 MB/process | Bounded; sustained overflow degrades readiness |
| Replay index cache | < 64 MB/process | Seek episode windows without full scans |

### 5.3 Disk Budgets

| File | Growth rate | Target max | Rotation |
|---|---|---|---|
| `engrams.jsonl` | ~1 KB/Signal | 100 MB | Demurrage/decay prune |
| `episodes.jsonl` | ~2 KB/episode | 50 MB | Retain `episode_retention_days` |
| `efficiency.jsonl` | ~500 B/event | 20 MB | Monthly rotation |
| `cascade-router.json` | Rewritten | < 1 MB | Single file |
| `gate-thresholds.json` | Rewritten | < 10 KB | Single file |
| `experiments.json` | Grows with experiments | < 5 MB | Archive concluded |
| `telemetry.log.jsonl` | 1-10 MB/hour | 7 days local | Rotate by size and age |

---

## 6. Benchmark Suite

```rust
#[bench]
fn bench_decay_apply_halflife(b: &mut Bencher) {
    let d = Decay::HalfLife { half_life_ms: 86_400_000 };
    b.iter(|| d.apply(black_box(3_600_000)));
}

#[bench]
fn bench_decay_apply_ebbinghaus(b: &mut Bencher) {
    let d = Decay::Ebbinghaus { strength: 1.0, scale_ms: 86_400_000 };
    b.iter(|| d.apply(black_box(3_600_000)));
}

#[bench]
fn bench_score_effective(b: &mut Bencher) {
    let s = Score { relevance: 0.8, confidence: 0.9, urgency: 0.5,
                    novelty: 0.3, salience: 0.6, coherence: 0.7, surprise: 0.2 };
    b.iter(|| black_box(&s).effective());
}

#[bench]
fn bench_hdc_hamming_10240(b: &mut Bencher) {
    let a = vec![0u64; 160];
    let c = vec![u64::MAX; 160];
    b.iter(|| hamming_distance(black_box(&a), black_box(&c)));
}

#[bench]
fn bench_prospect_value(b: &mut Bencher) {
    b.iter(|| {
        let x = black_box(-0.5_f64);
        let alpha = 0.88;
        let lambda = 2.25;
        -lambda * x.abs().powf(alpha)
    });
}
```

---

## 7. Test Criteria

1. `Decay::apply()` produces results within 1e-6 of the mathematical formula
2. `Score::effective()` with all axes at 1.0 returns 1.0; at -1.0 returns -1.0
3. No function produces NaN or Inf when given valid inputs (proptest)
4. `Decay::apply()` with `half_life_ms = 0` returns 0.0 (not NaN or Inf)
5. `Decay::Ebbinghaus` with `strength = 0.0` returns 0.0 (not NaN)
6. Cost normalization of 0 tokens returns 0.0 USD
7. EMA with alpha = 0.0 returns old value; alpha = 1.0 returns new sample
8. HDC Hamming of identical vectors is 0; of complementary vectors is dimensionality
9. Benchmark regression: all hot paths within 2x of baseline
10. Prospect-theory value: losses weighted > 2x equivalent gains
11. EnrichedCell functor ordering matches documented stack discipline
12. Metrics expose stable base units: seconds, USD, integers
13. Histogram bucket families are fixed and monotonic
14. Metric label validation rejects high-cardinality identifiers
15. Log and trace queues remain bounded under sink failure

---

## Cross-References

- [00-ARCHITECTURE](../../00-ARCHITECTURE.md) -- Parent chapter
- [cognitive-cross-cuts.md](cognitive-cross-cuts.md) -- Functor performance characteristics
- [decay-variants-and-tier-matrix.md](decay-variants-and-tier-matrix.md) -- Decay math
- [score-7-axis-appraisal.md](score-7-axis-appraisal.md) -- Score normalization
- `crates/roko-core/src/decay.rs` -- Decay implementation
- `crates/roko-core/src/engram.rs` -- Signal structure
- `crates/roko-core/src/score.rs` -- Score representation
- `crates/roko-primitives/src/hdc/` -- HDC operations
