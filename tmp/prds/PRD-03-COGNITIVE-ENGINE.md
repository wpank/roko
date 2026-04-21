# PRD-03: The cognitive engine

*How Roko makes 80% of agent ticks cost $0.*

---

## 1. Introduction

Every agent framework on the market has the same structural flaw: every tick goes to an LLM. Observe something? Call the model. Get a result? Call the model. Nothing happened? Call the model anyway, because the framework has no mechanism to decide otherwise.

This means every running agent burns the most expensive inference tier (what Roko calls T2) on every single action. For continuous agents operating at moderate frequency -- say one tick every 7.5 seconds -- that adds up to 11,520 LLM calls per day. At current Opus-class pricing, that is somewhere between $115 and $576 per agent per day depending on context window usage. Run five agents on a project and you are spending $2,880/day before any of them produce useful output.

The cognitive engine is Roko's answer. It sits between the observation pipeline and the inference backend and makes one decision per tick: does this situation require an LLM call at all?

The answer, it turns out, is usually no. Most ticks in a running agent are confirmations of expected state. The test still passes. The file still exists. The blockchain block contains no relevant transactions. The research corpus has not changed since the last check. These ticks carry zero prediction error -- the agent's internal model of the world matches what it observes -- and zero prediction error means zero information gain from consulting an LLM.

Roko's cognitive engine routes these ticks to T0 (pure Rust, no LLM, sub-millisecond, $0) and reserves expensive inference for moments of genuine surprise. In practice, 80% or more of ticks stay at T0 across every domain we have tested. The remaining ticks split between T1 (lightweight model, minimal context) and T2 (full reasoning with complete workspace). The result is a 10-100x cost reduction from architecture alone, before any prompt optimization or caching.

This document explains every component of that system: the three cognitive tiers, the prediction error signal that drives gating, the adaptive thresholds that tune themselves, the somatic markers that encode experiential intuition, and the domain-specific fast paths that make T0 viable for real workloads.


## 2. The economic argument

### 2.1 The baseline: ungated agents

A continuous agent polling at one tick every 7.5 seconds makes:

```
86,400 seconds / 7.5 seconds = 11,520 ticks/day
```

At Opus-class pricing (roughly $15 per million input tokens, $75 per million output tokens), a moderate context window of 8,000 input tokens and 2,000 output tokens per tick costs:

```
Input:  11,520 x 8,000 / 1,000,000 x $15  = $1,382.40
Output: 11,520 x 2,000 / 1,000,000 x $75  = $1,728.00
Total:  ~$3,110/day per agent at full context
```

Even a conservative estimate with smaller context windows lands at $115-$576/day. This is why most "autonomous agent" demos run for five minutes and then stop.

### 2.2 The gated agent

With cognitive gating at 80% T0 / 15% T1 / 5% T2, the same agent's daily cost becomes:

| Tier | Ticks/day | Cost/tick | Daily cost |
|------|-----------|-----------|------------|
| T0 (deterministic) | 9,216 | $0.000 | $0.00 |
| T1 (Haiku-class) | 1,728 | $0.001 | $1.73 |
| T2 (Opus-class) | 576 | $0.050 | $28.80 |
| **Total** | **11,520** | | **$30.53** |

That is a 102x reduction from the full-context baseline, or a 10x reduction from even the most conservative ungated estimate.

### 2.3 Domain-specific cost profiles

The tier distribution varies by domain. Blockchain agents achieve higher T0 rates because chain data is highly structured and predictable. Research agents need more T2 because novelty detection is harder to do without semantic understanding.

| Domain | T0% | T1% | T2% | Daily cost | Reduction vs ungated |
|--------|-----|-----|-----|------------|---------------------|
| Coding | 78% | 15% | 7% | $44.50 | ~70x |
| Blockchain | 95% | 3% | 2% | $6.28 | ~500x |
| Research | 70% | 18% | 12% | $72.40 | ~43x |
| Security audit | 82% | 10% | 8% | $51.20 | ~60x |

These numbers reflect production-like workloads, not toy benchmarks. The blockchain number is high because the triage pipeline (section 9) handles 95%+ of chain events without any LLM involvement.


## 3. Three cognitive tiers

Roko's inference tier system is defined in `roko-primitives/src/tier.rs`:

```rust
/// Three-tier gate for inference spend and latency.
///
/// - `T0`: suppress -- heuristics only, no LLM call
/// - `T1`: analyze -- light LLM (Haiku-class)
/// - `T2`: deliberate -- full LLM (Opus/Sonnet based on vitality)
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum InferenceTier {
    T0 = 0,
    T1 = 1,
    T2 = 2,
}
```

The `TierRouter` maps tiers to concrete models with vitality-aware degradation:

```rust
pub struct TierRouter;

impl TierRouter {
    pub fn select_model(tier: InferenceTier, vitality: f32) -> Option<&'static str> {
        match tier {
            InferenceTier::T0 => None,
            InferenceTier::T1 => Some("claude-haiku-4-5"),
            InferenceTier::T2 => {
                if vitality >= T2_VITALITY_THRESHOLD {
                    Some("claude-opus-4-6")
                } else {
                    Some("claude-sonnet-4")
                }
            }
        }
    }
}
```

### 3.1 T0: Deterministic (pure Rust, no LLM, $0, <1ms)

T0 handles the tick entirely in compiled Rust code. No network call. No serialization. No token counting. The agent's existing internal state is sufficient to decide the next action.

T0 processing includes:

**Pattern matching via somatic markers.** The agent maintains a k-d tree (using the `kiddo` crate) of past situation fingerprints indexed by 8-dimensional strategy coordinates. Querying this tree takes <100 microseconds. If the current situation's HDC fingerprint is close to a known-safe pattern, T0 can confidently continue without escalation.

**Habituation checks.** Every observation is hashed with Blake3. If the same hash has appeared N times without ever triggering an escalation, its novelty is attenuated logarithmically. Familiar, harmless patterns stop triggering expensive inference.

**Rule-based filters.** Bloom filters for watched addresses, threshold comparisons for numeric values, regex patterns for known event types. These run in microseconds.

**HDC fingerprint similarity.** The 10,240-bit hyperdimensional computing vectors in `roko-primitives/src/hdc.rs` provide O(1) similarity computation via Hamming distance:

```rust
pub fn similarity(&self, other: &Self) -> f32 {
    let mut differing_bits = 0u32;
    for (left, right) in self.bits.iter().zip(other.bits.iter()) {
        differing_bits += (left ^ right).count_ones();
    }
    let differing_bits = u16::try_from(differing_bits).unwrap_or(u16::MAX);
    1.0_f32 - (f32::from(differing_bits) / 10_240.0_f32)
}
```

This is pure bit manipulation. No floating-point matrix multiplies. No GPU. Comparing two 10,240-bit vectors takes nanoseconds. The HDC codebook (`ItemMemory`) provides brute-force nearest-neighbor lookup over named concept vectors, and the `BundleAccumulator` builds composite situation fingerprints from multiple observation vectors via majority-vote bundling.

### 3.2 T1: Lightweight ($0.001, ~500ms)

T1 invokes a cheap model (Haiku-class, currently `claude-haiku-4-5`) with minimal context. The prompt contains only the current task description and the immediate observation -- roughly 2,000 tokens total. The model answers one question: should the agent continue its current action, pause, or escalate to T2?

T1 exists because some situations carry moderate ambiguity that T0's pattern matching cannot resolve, but do not warrant the full reasoning pipeline. A linter warning on a line the agent did not touch, for example. Or a test that fails intermittently but passed on the last three runs.

T1 costs roughly $0.001 per call. It adds ~500ms of latency, which is acceptable for the 15-20% of ticks that need it.

### 3.3 T2: Full reasoning ($0.01-$0.10, ~3-5s)

T2 is the complete inference pipeline. It routes to the most capable available model -- currently `claude-opus-4-6` when agent vitality is above 0.3, degrading to `claude-sonnet-4` when vitality drops below that threshold (economic pressure from the mortality system forces cost conservation).

The T2 pipeline includes:

- **Complete CognitiveWorkspace assembly.** The system prompt builder (`RoleSystemPromptSpec`) assembles a 9-layer prompt with role instructions, playbook hits, task context, workspace map, and relevant knowledge entries. This typically runs to ~25,000 tokens.

- **Full tool loop with somatic checks.** The agent can call tools (file read/write, shell execution, web search, MCP integrations) and each tool result is checked against somatic markers before the next turn.

- **Episode recording.** Every T2 tick produces an episode entry in `.roko/episodes.jsonl` with an HDC fingerprint, enabling future T0 pattern matching against this situation.

- **CRPS prediction commitment.** For blockchain agents, T2 ticks include a prediction commitment for Korai's epistemic reputation system (section 10).

T2 costs $0.01-$0.10 per call depending on context size and output length. At 5% of ticks, this is manageable.


## 4. Prediction error: the gating signal

The cognitive gate makes one measurement: prediction error (PE). How much does the observed state differ from what the agent expected?

### 4.1 Computing prediction error

Every extension in Roko's runtime can report observations via `on_observe()`. The cognitive gate aggregates these into a scalar PE value:

```rust
/// Aggregate prediction error across all observation channels.
fn compute_prediction_error(
    observations: &[Observation],
    weights: &ObservationWeights,
) -> f64 {
    let mut weighted_sum = 0.0;
    let mut weight_total = 0.0;

    for obs in observations {
        let weight = weights.weight_for(obs.channel());
        let error = (obs.observed() - obs.expected()).abs();
        weighted_sum += weight * error;
        weight_total += weight;
    }

    if weight_total > 0.0 {
        weighted_sum / weight_total
    } else {
        0.0
    }
}
```

The key insight: PE is domain-agnostic at this level. The observation channels differ by domain, but the gating decision is the same everywhere. High PE means the agent's model of the world is wrong. Low PE means everything is going as expected.

### 4.2 Domain-specific PE sources

What counts as "unexpected" varies by domain. Each domain registers its own observation channels:

**Coding domain:**
- Test failure on previously-passing code (PE = 1.0)
- New error type not seen in this session (PE = 0.8)
- Unfamiliar codebase pattern -- low HDC similarity to known patterns (PE = 0.3-0.7, proportional to distance)
- Gate regression -- a gate that passed last run now fails (PE = 0.9)
- Clippy warning count increased (PE = 0.2-0.5)

**Blockchain domain:**
- Price deviation beyond 2 standard deviations from recent mean (PE proportional to z-score)
- Anomalous gas usage on a monitored contract (PE = 0.6)
- Large value transfer above configured threshold (PE = 0.5-1.0)
- ISFR rate jump exceeding 50 basis points in a single update (PE = 0.7)
- New contract interaction not in the agent's ABI cache (PE = 0.4)

**Research domain:**
- Contradictory finding relative to current hypothesis (PE = 0.8)
- Novel citation -- a paper not in the agent's reference set cited by multiple sources (PE = 0.5)
- Hypothesis refutation -- evidence directly countering a working assumption (PE = 0.9)
- Source conflict -- two trusted sources disagree on a factual claim (PE = 0.7)

### 4.3 The gating decision

The cognitive gate compares PE against an adaptive threshold:

```rust
pub trait CognitiveGate: Send + Sync {
    fn gate(
        &self,
        cortical: &CorticalState,
        observations: &[Observation],
    ) -> InferenceTier;
}
```

The default implementation:

```rust
impl CognitiveGate for DefaultCognitiveGate {
    fn gate(
        &self,
        cortical: &CorticalState,
        observations: &[Observation],
    ) -> InferenceTier {
        let pe = compute_prediction_error(observations, &self.weights);

        // Somatic check: if current HDC fingerprint is close to a known
        // failure pattern, artificially inflate PE.
        let somatic_boost = self.somatic_landscape
            .query(cortical.strategy_coords(), self.somatic_k)
            .confidence_multiplier;
        let adjusted_pe = if somatic_boost < 1.0 {
            // Negative somatic signal: inflate PE toward caution.
            pe + (1.0 - somatic_boost) * 0.3
        } else {
            pe
        };

        // Habituation: attenuate PE for frequently-seen patterns.
        let fingerprint_hash = cortical.observation_hash();
        let frequency = self.frequency_tracker.count(fingerprint_hash);
        let novelty_factor = 1.0 / (1.0 + (frequency as f64).ln());
        let final_pe = adjusted_pe * novelty_factor;

        // Compare against adaptive threshold.
        let threshold = self.adaptive_thresholds.threshold_for(
            cortical.current_rung()
        );

        if final_pe < threshold * 0.3 {
            InferenceTier::T0
        } else if final_pe < threshold {
            InferenceTier::T1
        } else {
            InferenceTier::T2
        }
    }
}
```

The thresholds at 0.3x and 1.0x of the adaptive threshold create clean separation between tiers. Below 30% of threshold: deterministic handling. Between 30% and 100%: lightweight model. Above threshold: full reasoning.


## 5. Adaptive thresholds

Static thresholds fail in practice because the PE distribution shifts as agents learn and as environments change. A threshold tuned for the first hour of operation becomes too loose or too tight after a day.

Roko's adaptive threshold system lives in `roko-gate/src/adaptive_threshold.rs`. It tracks per-rung statistics using three mechanisms.

### 5.1 Exponential moving average (EMA)

The EMA tracks the running pass rate for each gate rung:

```rust
const EMA_ALPHA: f64 = 0.1;

pub fn observe(&mut self, rung: u32, passed: bool) {
    let stats = self.rungs.entry(rung).or_default();
    let value = if passed { 1.0 } else { 0.0 };

    if stats.total_observations == 0 {
        stats.ema_pass_rate = value;
    } else {
        stats.ema_pass_rate = EMA_ALPHA
            .mul_add(value, (1.0 - EMA_ALPHA) * stats.ema_pass_rate);
    }

    stats.total_observations += 1;
    // ...
}
```

Alpha of 0.1 means recent observations weigh roughly 10x more than observations from 20 ticks ago. The EMA converges fast enough to track hourly shifts but slowly enough to resist noise from individual failures.

### 5.2 CUSUM (Cumulative Sum Control Chart)

EMA tracks the level. CUSUM detects *changes* in level -- regime shifts where the underlying process has fundamentally changed.

```rust
// CUSUM change detection.
let deviation = value - stats.ema_pass_rate;

stats.cusum_high = (stats.cusum_high + deviation
    - self.cusum_sensitivity).max(0.0);
stats.cusum_low = (stats.cusum_low - deviation
    - self.cusum_sensitivity).max(0.0);

if stats.cusum_high > self.cusum_threshold
    || stats.cusum_low > self.cusum_threshold
{
    stats.cusum_shift_detected = true;
    stats.ema_pass_rate = value;  // Reset EMA to adapt quickly.
    stats.cusum_high = 0.0;
    stats.cusum_low = 0.0;
}
```

When CUSUM detects a shift:
1. The EMA resets to the current observation, bypassing the slow convergence.
2. The gate threshold drops temporarily, making the agent more reactive.
3. More ticks escalate to T1/T2 until the new regime stabilizes.

The default parameters (sensitivity = 0.25, threshold = 4.0) balance detection speed against false alarm rate. Domain-specific profiles override these: security agents use sensitivity = 0.15 (more sensitive to shifts), research agents use 0.30 (more tolerant).

### 5.3 SPC detector ensemble

Beyond CUSUM, each gate rung runs a three-detector ensemble from `roko-gate/src/spc.rs`:

1. **CUSUM** (Cumulative Sum): Detects sustained shifts in pass rates. Accumulates deviations from a target; when the cumulative sum exceeds threshold `h`, signals a shift. Catches gradual degradation.

2. **EWMA Control Chart**: Exponentially weighted moving average with formal UCL/LCL (Upper/Lower Control Limits). More sensitive to small shifts than standard Shewhart charts.

3. **BOCPD** (Bayesian Online Change Point Detection): Detects abrupt regime changes. Maintains a run-length distribution and signals when posterior probability of a recent change point exceeds a threshold. Catches sudden model updates or environment changes that the other two detectors are slow to identify.

Any alert from any detector in the ensemble triggers threshold recalibration:

```rust
let spc = self.spc_detectors
    .entry(rung)
    .or_insert_with(|| SpcDetector::new(target, 0.1));
let alerts = spc.update(value);
for alert in alerts {
    self.pending_spc_alerts.push((rung, alert));
}
```

### 5.4 Joint anomaly detection (Hotelling's T-squared)

Individual rung detectors catch per-gate regressions. But sometimes the degradation is systemic -- all gates degrade slightly, none enough to trigger individual alerts. The `HotellingDetector` watches the full pass-rate vector across all rungs:

```rust
pub fn observe_pipeline(&mut self, pass_rates: &[f64]) {
    let det = self.hotelling
        .get_or_insert_with(|| HotellingDetector::new(
            pass_rates.len(), 0.05
        ));
    det.update(pass_rates);
    self.joint_anomaly_detected = det.is_anomalous(pass_rates);
}
```

When the T-squared statistic exceeds the chi-squared threshold (alpha = 0.05), `joint_anomaly_detected` fires. This forces all rungs into a temporary heightened state, lowering thresholds across the board until the system stabilizes.

### 5.5 Domain-specific profiles

Different agent roles have different priors about what pass rates to expect:

```rust
pub fn coding() -> ThresholdProfile {
    let mut priors = HashMap::new();
    priors.insert(0, 0.90); // compile: should almost always pass
    priors.insert(1, 0.80); // clippy: usually passes
    priors.insert(2, 0.65); // test: moderate expectation
    priors.insert(3, 0.50); // diff review: neutral
    ThresholdProfile {
        name: "coding".into(),
        rung_priors: priors,
        floor_multiplier: 1.0,
        retry_multiplier: 1.0,
        cusum_sensitivity_override: None,
    }
}

pub fn security() -> ThresholdProfile {
    let mut priors = HashMap::new();
    priors.insert(0, 0.95); // compile: must pass
    priors.insert(1, 0.90); // clippy: strict
    priors.insert(2, 0.90); // test: strict
    priors.insert(3, 0.80); // diff: careful review
    ThresholdProfile {
        name: "security".into(),
        rung_priors: priors,
        floor_multiplier: 1.3,
        retry_multiplier: 0.7,
        cusum_sensitivity_override: Some(0.15),
    }
}
```

Security agents tolerate fewer retries (`retry_multiplier: 0.7`), demand stricter floors (`floor_multiplier: 1.3`), and use tighter CUSUM sensitivity (`0.15` vs the default `0.25`). Research agents go the other direction: more retries, looser floors, higher CUSUM slack.

### 5.6 Temperament-aware adjustments

The adaptive thresholds also respond to the agent's temperament (a configuration parameter, not a learned value):

```rust
pub fn threshold_for_temperament(
    &self,
    rung: u32,
    temperament: Temperament,
) -> f64 {
    let base = self.threshold_for(rung);
    let adjusted = match temperament {
        Temperament::Conservative => base * 1.10,
        Temperament::Balanced     => base,
        Temperament::Aggressive   => base * 0.85,
        Temperament::Exploratory  => base * 0.90,
    };
    adjusted.clamp(0.0, 1.0)
}
```

Conservative agents raise the threshold by 10% (stricter gates, more T2 calls, higher cost, fewer errors). Aggressive agents lower it by 15% (more T0, lower cost, more risk). The operator chooses the tradeoff.

Conservative agents also never skip rungs, regardless of pass streak:

```rust
pub fn should_skip_rung_for_temperament(
    &self,
    rung: u32,
    temperament: Temperament,
) -> bool {
    match temperament {
        Temperament::Conservative => false,  // Never skip.
        Temperament::Balanced | Temperament::Exploratory =>
            self.should_skip_rung(rung),
        Temperament::Aggressive =>
            // Skip after half the normal streak.
            self.rungs.get(&rung)
                .is_some_and(|s| s.consecutive_passes
                    >= SKIP_STREAK_THRESHOLD / 2),
    }
}
```

### 5.7 Neuro-informed priors

The adaptive threshold system accepts hints from the neuro knowledge store:

```rust
pub fn apply_neuro_hints(
    &mut self,
    known_failure_rungs: &[u32],
    known_stable_rungs: &[u32],
) {
    for &rung in known_failure_rungs {
        let stats = self.rungs.entry(rung).or_default();
        if stats.total_observations < 10 {
            stats.ema_pass_rate = (stats.ema_pass_rate * 0.7).min(0.5);
        }
    }

    if !known_failure_rungs.is_empty() {
        let tighter = (self.cusum_sensitivity * 0.7).max(0.01);
        self.cusum_sensitivity = tighter;
    }

    if !known_stable_rungs.is_empty() && known_failure_rungs.is_empty() {
        let relaxed = (self.cusum_sensitivity * 1.3).min(0.15);
        self.cusum_sensitivity = relaxed;
    }
}
```

When neuro knows a rung has persistent failure patterns, the EMA is biased toward caution (capped at 0.5) and CUSUM sensitivity tightens by 30%. When neuro confirms stability, CUSUM sensitivity relaxes. This prevents the adaptive system from being naively optimistic about rungs that prior agents have already proven problematic.

### 5.8 Persistence

The entire threshold state serializes to JSON and persists at `.roko/learn/gate-thresholds.json`:

```rust
pub fn save(&self, path: &Path) -> Result<(), std::io::Error> {
    let json = serde_json::to_string_pretty(self)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    let mut tmp_file = std::fs::File::create(&tmp)?;
    tmp_file.write_all(json.as_bytes())?;
    tmp_file.sync_all()?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}
```

Writes are atomic (write to temp file, fsync, rename). The threshold state survives agent restarts and feeds into the next session's priors.


## 6. Habituation: stop reacting to noise

Agents that treat every observation as novel waste money. Habituation tracks how frequently each observation pattern has occurred and attenuates its novelty accordingly.

### 6.1 The frequency tracker

Every observation is hashed with Blake3 (the same hash function used throughout Roko's signal layer). The hash is stored in a bounded-capacity frequency map:

```rust
struct FrequencyTracker {
    counts: HashMap<u64, u32>,
    max_entries: usize,
}

impl FrequencyTracker {
    fn observe(&mut self, hash: u64) -> u32 {
        let count = self.counts.entry(hash).or_insert(0);
        *count = count.saturating_add(1);

        // Evict least-frequent entries when at capacity.
        if self.counts.len() > self.max_entries {
            self.evict_least_frequent();
        }

        *count
    }

    fn count(&self, hash: u64) -> u32 {
        self.counts.get(&hash).copied().unwrap_or(0)
    }
}
```

### 6.2 Novelty attenuation

The frequency count maps to a novelty multiplier via logarithmic decay:

```rust
fn novelty_factor(frequency: u32) -> f64 {
    1.0 / (1.0 + (frequency as f64).ln())
}
```

This curve has useful properties:
- First occurrence (frequency = 1): novelty = 1.0 (full PE weight)
- Second occurrence: novelty = 0.59
- Fifth occurrence: novelty = 0.38
- Twentieth occurrence: novelty = 0.25
- Hundredth occurrence: novelty = 0.18

The function never reaches zero. Even a pattern seen thousands of times retains a small novelty weight (0.10-0.12), so a genuine change to a familiar pattern can still escalate.

### 6.3 Dishabituation

If a previously-habituated pattern suddenly produces an unexpected outcome -- an observation that always mapped to T0 suddenly correlates with a gate failure -- the frequency counter for that hash resets. This is dishabituation: the agent stops ignoring a pattern that has changed its meaning.

Dishabituation is triggered when:
- A T0-gated observation is followed by a gate failure within the same task
- The somatic landscape records a negative outcome at coordinates close to the habituated pattern
- CUSUM or BOCPD detects a regime shift affecting the observation channel


## 7. Somatic markers: embodied hesitation

Before the cognitive gate evaluates prediction error, the agent checks its somatic landscape -- a k-d tree of past outcomes indexed by 8-dimensional strategy coordinates. This implements Damasio's somatic marker hypothesis (1994): gut feelings about risky situations based on experiential memory, not explicit rules.

### 7.1 The somatic landscape

The somatic landscape lives in `roko-daimon/src/lib.rs`:

```rust
type SomaticTree = KdTree<f64, STRATEGY_DIMENSIONS>;
// STRATEGY_DIMENSIONS = 8
```

Each somatic marker records an outcome at a point in strategy space. The `kiddo` crate provides an efficient k-d tree implementation with O(log n) nearest-neighbor queries. For typical agent lifetimes (thousands of entries), queries complete in <100 microseconds.

### 7.2 Recording outcomes

After every T2 tick, the agent records the outcome in the somatic landscape:

```rust
landscape.record_outcome(
    strategy_coords,      // 8-dimensional strategy position
    valence,              // [-1.0, 1.0]: negative = bad outcome
    intensity,            // [0.0, 1.0]: how strongly the outcome mattered
    episode_hash,         // Content hash linking to the episode log
    timestamp,            // For temporal decay
);
```

Positive valence (successful task completion, gate pass) creates a marker that says "this region of strategy space worked well." Negative valence (gate failure, error, quality regression) creates a warning marker.

### 7.3 Querying for somatic bias

When the cognitive gate runs, it queries the landscape at the current strategy coordinates:

```rust
let signal = landscape.query(strategy_coords, k);
// k = 5 by default (five nearest neighbors)
```

The query returns a `SomaticSignal` with aggregated valence, intensity, and contrarian contribution from the k nearest markers. The `SomaticOracleContext` then computes a confidence multiplier:

```rust
pub fn somatic_confidence_bias(valence: f64, intensity: f64) -> f64 {
    const MAX_BIAS: f64 = 0.30;
    let raw = valence * intensity * MAX_BIAS;
    (1.0 + raw).clamp(0.7, 1.3)
}
```

The multiplier ranges from 0.7 (strong negative somatic signal -- "this feels dangerous") to 1.3 (strong positive signal -- "this has worked before"). It is clamped to prevent somatic data from overriding rational analysis entirely.

### 7.4 Contrarian blending

15% of the somatic signal comes from contrarian neighbors -- markers with opposite valence to the majority:

```rust
const CONTRARIAN_FRACTION: f64 = 0.15;
```

This prevents the somatic system from becoming a pure echo chamber. If a region of strategy space has a history of success, the contrarian fraction injects a small dose of caution from nearby failure markers. If a region is all failures, it injects a small amount of optimism.

The theoretical basis is Bower (1981): mood-congruent retrieval is the default, but a small contrarian fraction improves decision quality by preventing emotional tunnel vision.

### 7.5 Integration with the cognitive gate

In the gating pipeline, a negative somatic signal (confidence multiplier < 1.0) inflates the prediction error:

```rust
let adjusted_pe = if somatic_boost < 1.0 {
    pe + (1.0 - somatic_boost) * 0.3
} else {
    pe
};
```

This means the agent hesitates before acting in regions of strategy space where it has been burned before. A maximum somatic penalty of 0.09 (0.3 * 0.30) can push a borderline T0 observation into T1, or a borderline T1 into T2. It cannot, by design, prevent all T0 gating -- the penalty is bounded.


## 8. The three-layer affect model

Somatic markers do not exist in isolation. They are part of a larger affect system -- the ALMA model (Gebhard 2005) adapted for artificial agents. The affect state influences gating thresholds, exploration rates, and model selection.

### 8.1 ALMA layers

The affect model in `roko-daimon` has three temporal layers:

```rust
pub struct AlmaLayers {
    /// Fast emotional response. Updated every tick.
    pub emotion: PadVector,
    /// Medium-term mood. Updated every `mood_interval` ticks.
    pub mood: PadVector,
    /// Stable personality baseline.
    pub temperament: PadVector,
    /// Emotion layer decay factor per tick (default 0.1).
    pub tau_emotion: f64,
    /// Mood layer EMA factor (default 0.5).
    pub tau_mood: f64,
}
```

Each layer operates on PAD vectors (Pleasure-Arousal-Dominance), a validated dimensional model of emotion from Mehrabian (1996). The emotion layer reacts instantly. The mood layer tracks hourly trends. The temperament layer captures personality.

### 8.2 Affect and gating thresholds

The mood layer modulates the cognitive gate threshold:
- **High pleasure, low arousal** (things going well, relaxed): threshold rises slightly -- fewer expensive calls, more T0.
- **Low pleasure, high arousal** (things going badly, stressed): threshold drops -- more T2 calls, the agent is being careful.
- **Moderate arousal, moderate dominance** (engaged, in control): threshold unchanged -- the system trusts the default calibration.

### 8.3 Four-factor retrieval model

When the agent needs to recall knowledge for context assembly, the retrieval weights are mood-sensitive:

```rust
pub struct RetrievalWeights {
    pub recency: f64,     // Ebbinghaus forgetting curve
    pub importance: f64,  // Reflexion validation ratio
    pub relevance: f64,   // Cosine similarity to query
    pub emotional: f64,   // PAD cosine with current mood
}
```

The emotional factor means an agent in a cautious mood (negative valence) preferentially retrieves negative outcome memories, reinforcing vigilance. An agent in a confident mood retrieves success patterns, reinforcing momentum. The 15% contrarian fraction in somatic retrieval counterbalances this mood-congruent bias.


## 9. The triage pipeline: blockchain T0 fast path

Blockchain agents monitor on-chain data that arrives at high frequency (new blocks every 2-12 seconds depending on chain). Most of this data is noise relative to the agent's task. The triage pipeline is a four-stage pure-Rust filter that reduces chain event volume by 95%+ without any LLM involvement.

### 9.1 Stage 1: Rule-based filters (microseconds)

```rust
struct RuleFilters {
    /// Bloom filter of watched addresses (O(1) membership test).
    watched_addresses: BloomFilter,
    /// Minimum transfer value worth analyzing (in native units).
    value_threshold: u128,
    /// Known transaction type patterns (method selectors).
    known_patterns: HashSet<[u8; 4]>,
}

impl RuleFilters {
    fn should_pass(&self, event: &ChainEvent) -> bool {
        // Pass if: involves a watched address, OR exceeds value
        // threshold, OR matches no known (boring) pattern.
        self.watched_addresses.contains(&event.sender)
            || self.watched_addresses.contains(&event.recipient)
            || event.value >= self.value_threshold
            || !self.known_patterns.contains(&event.method_selector)
    }
}
```

Bloom filters provide O(1) probabilistic membership testing with zero false negatives (a watched address never slips through) and a configurable false positive rate (default 0.1%). At 10,000 watched addresses, the bloom filter occupies ~12KB of memory.

This stage eliminates 60-80% of chain events in sub-microsecond time.

### 9.2 Stage 2: Statistical anomaly detection (sub-millisecond)

Events that pass the rule filter enter statistical analysis:

- **MIDAS-R** for graph anomaly detection: tracks edge frequency in the transaction graph and flags unusual connection patterns. A known contract suddenly interacting with 100 new addresses in a single block triggers this.

- **DDSketch** for distribution shift detection: maintains quantile sketches of gas usage, transfer values, and timing patterns. When the current observation falls outside the 99th percentile of the historical distribution, it flags as anomalous.

Both algorithms run in sub-millisecond time and maintain O(1) or O(log n) state per metric.

### 9.3 Stage 3: Contextual enrichment (milliseconds)

Anomalous events get enriched with cached metadata:

- ABI resolution from a local contract metadata cache (no RPC calls for known contracts)
- Transaction classification (swap, transfer, governance vote, bridge, etc.)
- Counterparty risk scoring based on historical interaction frequency

This stage takes low single-digit milliseconds per event. It transforms raw event data into structured observations that the PE computation can evaluate.

### 9.4 Stage 4: Thompson sampling scorer (sub-millisecond)

The final stage applies a discounted Thompson sampling scorer to rank enriched events by expected information value. Events with high scores proceed to the cognitive gate (where most still gate to T0 due to habituation). Events with low scores are logged but not evaluated.

The Thompson sampler uses beta distributions per event category, updated with a discount factor so recent observations count more than stale ones. This creates an exploration-exploitation balance: the agent periodically re-evaluates event types it has been ignoring, in case their information value has changed.

### 9.5 Pipeline performance

End-to-end latency: <5ms per event, typically <2ms for rule-filtered events. A chain producing 200 events per block with 2-second block times generates 100 events/second. The pipeline handles this on a single core with 80%+ idle time.

For Ethereum mainnet (15-20 events per block at 12-second intervals), the pipeline processes the entire block in <10ms total.


## 10. ISFR prediction integration

Blockchain agents operating on Korai commit CRPS (Continuous Ranked Probability Score) predictions before each ISFR (Instantaneous Staking Flow Rate) update. ISFR updates arrive every 10 seconds, creating 8,640 calibration signals per day.

### 10.1 Prediction commitment

The prediction commitment itself is T0 -- a deterministic model based on recent ISFR history. No LLM required:

```rust
struct IsfrPredictor {
    /// Recent ISFR values (ring buffer, last 100 updates).
    history: VecDeque<f64>,
    /// EMA of ISFR with alpha=0.05 (slow, tracks trend).
    ema_slow: f64,
    /// EMA of ISFR with alpha=0.20 (fast, tracks recent).
    ema_fast: f64,
    /// Predicted distribution (Gaussian, defined by mean + std).
    predicted_mean: f64,
    predicted_std: f64,
}

impl IsfrPredictor {
    fn commit_prediction(&mut self) -> IsfrPrediction {
        // Mean = weighted average of fast and slow EMA.
        self.predicted_mean = 0.6 * self.ema_fast + 0.4 * self.ema_slow;

        // Std = rolling std of recent residuals.
        self.predicted_std = self.rolling_residual_std();

        IsfrPrediction {
            mean: self.predicted_mean,
            std: self.predicted_std,
            timestamp: Utc::now(),
        }
    }
}
```

### 10.2 CRPS scoring

After each ISFR update, the actual value is compared against the committed prediction using CRPS:

```
CRPS = E[|X - y|] - 0.5 * E[|X - X'|]
```

where X is the predicted distribution and y is the actual observation. For Gaussian predictions, this has a closed-form solution. Lower CRPS means better calibration.

### 10.3 Epistemic reputation tiers

CRPS scores accumulate on-chain to determine the agent's epistemic reputation tier:

| Tier | CRPS threshold | InsightStore access |
|------|---------------|-------------------|
| Oracle | < 0.05 | Unlimited read + write |
| Calibrated | 0.05 - 0.15 | Full read, limited write |
| Standard | 0.15 - 0.30 | Read only |
| Uncalibrated | > 0.30 | No access |

Better-calibrated agents get more access to the InsightStore (Korai's shared knowledge layer), creating an economic incentive for accurate predictions.

### 10.4 PE escalation from ISFR surprise

When the actual ISFR deviates from the prediction by more than 2 standard deviations, the PE for the blockchain observation channel spikes. This triggers escalation from T0 to T1 or T2, where the agent can reason about what caused the deviation and whether its strategy needs updating.

The prediction commitment is the only T0 operation that feeds into on-chain reputation. Everything else in the ISFR pipeline is local.


## 11. Cost tracking and mortality integration

The cognitive engine does not operate in isolation from the agent's resource constraints. Per-tick cost tracking feeds into the mortality system, which modulates gating behavior as resources deplete.

### 11.1 Three clocks

The mortality system in `roko-daimon/src/mortality.rs` tracks three independent clocks:

**Economic clock.** Budget remaining divided by burn rate. As the ratio drops, `EconomicAnxiety` intensity rises:

```rust
Self::EconomicAnxiety => {
    let urgency = if runway_hours > 0.0 {
        (burn_rate / runway_hours).min(1.0)
    } else {
        1.0
    };
    urgency.clamp(0.0, 1.0)
}
```

**Epistemic clock.** Prediction accuracy trend over time. As accuracy declines (the agent is becoming less calibrated), `EpistemicVertigo` rises:

```rust
Self::EpistemicVertigo => {
    (-accuracy_trend).clamp(0.0, 1.0)
}
```

**Stochastic clock.** Background finitude awareness, always slightly present:

```rust
Self::StochasticDread => {
    let base = 0.1;
    let urgency = if runway_hours > 0.0 {
        (24.0 / runway_hours).min(0.5)
    } else {
        0.5
    };
    (base + urgency).clamp(0.0, 1.0)
}
```

### 11.2 Behavioral phases

The three clocks feed into an aggregate vitality score, which determines the agent's behavioral phase (Nietzsche's three metamorphoses):

```rust
pub enum VitalityPhase {
    /// vitality > 0.7: steady execution, conservative, duty-driven
    Camel,
    /// vitality 0.3-0.7: explore/exploit crisis, creative destruction
    Lion,
    /// vitality < 0.3: creative acceptance, generous sharing
    Child,
}
```

Each phase has distinct characteristics:

| Phase | Exploration rate | Sharing threshold | Gate behavior |
|-------|-----------------|-------------------|---------------|
| Camel | 0.15 | 0.50 | Default thresholds, conservative |
| Lion | 0.40 | 0.40 | Thresholds fluctuate, more T2 calls |
| Child | 0.60 | 0.10 | Thresholds rise (save money), shares knowledge freely |

The Child phase is counterintuitive: a near-budget agent becomes more generous with its knowledge and more creative in its strategies. It has nothing to lose. The gate thresholds rise because the economic clock forces cost conservation, but the exploration rate also rises because habitual strategies are not worth preserving.

### 11.3 Vitality-aware model selection

The tier router degrades T2 from Opus to Sonnet when vitality drops below 0.3:

```rust
InferenceTier::T2 => {
    if vitality >= T2_VITALITY_THRESHOLD {
        Some("claude-opus-4-6")
    } else {
        Some("claude-sonnet-4")
    }
}
```

This is a sharp boundary, not a gradient. Below 0.3 vitality, the agent cannot afford Opus-class inference on every T2 call. Sonnet provides 80% of the capability at 20% of the cost, extending the agent's runway.

### 11.4 Emotional death testament

When an agent reaches terminal vitality, it produces an `EmotionalDeathTestament` containing its life review, final affect state, mortality emotion intensities, and annotated learnings. This document transfers to successor agents so they inherit not just knowledge but the emotional context that makes knowledge meaningful.


## 12. Cascade router: model selection beyond tiers

The cognitive tier (T0/T1/T2) determines *whether* to call an LLM. The cascade router determines *which* LLM to call. It lives in `roko-learn/src/cascade_router.rs` and matures through three stages as observation data accumulates.

### 12.1 Stage 1: Static routing (< 50 observations)

Before the system has enough data to learn from, it uses a hardcoded role-to-model mapping. Coding agents get Opus. Research agents get Opus. Security agents get Opus. The mapping is conservative (always pick the best model) because the cost of a wrong routing decision during cold start is higher than the cost of overprovisioning.

### 12.2 Stage 2: Confidence routing (50-200 observations)

Once enough observations accumulate, the router switches to empirical pass rates with confidence intervals. For each role-model pair, it tracks the historical pass rate and selects the cheapest model whose lower confidence bound exceeds the required pass rate.

### 12.3 Stage 3: UCB routing (> 200 observations)

After 200+ observations, a full LinUCB contextual bandit takes over. It uses an 8-dimensional context vector (task complexity, role, urgency, budget pressure, etc.) to select models. The bandit balances exploration (trying cheaper models to see if they suffice) against exploitation (using the model known to work).

### 12.4 Active inference integration

The cascade router integrates with `roko-learn/src/active_inference.rs`, which maintains a belief state over a 90-dimensional latent space (3 difficulty levels x 3 skill levels x 10 confidence levels):

```rust
pub fn select_tier(
    belief: &BeliefState,
    requirements: &TaskRequirements,
) -> ModelTier {
    let task_difficulty = task_difficulty(requirements);
    if task_difficulty >= 2 {
        return ModelTier::Premium;
    }
    tiers.into_iter()
        .min_by(|left, right| {
            expected_free_energy(belief, *left, task_difficulty)
                .partial_cmp(
                    &expected_free_energy(belief, *right, task_difficulty)
                )
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(ModelTier::Standard)
}
```

The selector minimizes expected free energy, which is a weighted sum of:
- **Risk**: probability of task failure given the selected tier
- **Ambiguity**: uncertainty about the current state (higher for low-confidence belief states)
- **Evidence**: cost and latency of the selected tier

This formulation comes directly from the free energy principle (Friston 2010): the agent selects the action that minimizes the expected divergence between its model and reality, weighted by the pragmatic cost of being wrong.


## 13. Measurement and validation

### 13.1 Metrics to track

The cognitive engine's effectiveness is measured through five primary metrics:

**Tier distribution.** The fraction of ticks at T0/T1/T2, tracked per domain, per agent role, per tick frequency. Target: >=80% T0 across all domains.

**False negative rate.** The percentage of T0-gated ticks that should have been T2 -- situations where the agent missed something important because it did not consult an LLM. Measured by retrospectively comparing T0 outcomes against T2 outcomes on the same observations (via periodic shadow evaluation). Target: <2%.

**Cost per batch.** Total inference cost for a batch of tasks with gating versus without. This is the headline number: does gating actually save money without degrading outcomes?

**Somatic hit rate.** How often somatic marker hesitation prevented an error. Measured by tracking cases where the somatic penalty pushed a tick from T0 to T1/T2, and that escalation caught a problem the T0 path would have missed. Target: >60% precision (of somatic-escalated ticks, >60% had genuine issues).

**Threshold convergence time.** How many ticks the adaptive threshold system needs to stabilize after a regime shift. Measured from CUSUM detection to EMA stabilization. Target: <100 ticks.

### 13.2 Arena-based validation

Roko's arena system can validate the cognitive engine itself:

1. Sample a batch of representative tasks across domains.
2. Run each task twice: once with full gating, once with 100% T2 (no gating).
3. Compare outcomes: task success rate, code quality, time to completion.
4. Compute the quality delta and cost delta.

If quality drops less than 5% while cost drops more than 80%, gating is validated. If quality drops more than 5%, the gating parameters need recalibration -- the adaptive thresholds may be too aggressive, or the somatic markers may be miscalibrated.

### 13.3 Anomaly detection

The `roko-learn/src/anomaly.rs` module provides session-level anomaly detection that integrates with the cognitive engine:

- **Prompt loop detection**: If the same prompt hash appears 5+ times in the last 20 ticks, the agent is stuck in a loop. This forces T2 escalation regardless of PE.
- **Cost spike detection**: If a single tick's cost exceeds 3 standard deviations from the EWMA baseline, flag it. This catches situations where the cognitive gate is systematically underestimating PE.
- **Quality drift detection**: If recent quality scores are consistently below the earlier baseline (measured over sliding windows of 5 recent vs 10 earlier observations), the system is degrading. This triggers threshold recalibration.


## 14. Academic foundations

The cognitive engine draws on four decades of research across neuroscience, information theory, and decision theory.

### 14.1 Active inference and the free energy principle

Friston, K. (2010). "The free-energy principle: a unified brain theory?" *Nature Reviews Neuroscience*, 11(2), 127-138.

The free energy principle states that biological agents minimize the divergence between their generative model and sensory observations. Prediction error is the measurable proxy for this divergence. When PE is low, the agent's model is accurate and no costly model update is needed (T0). When PE is high, the model needs updating, which requires inference (T1/T2).

Roko's cognitive gating is a direct implementation of this principle: the PE signal drives tier selection, and the expected free energy formulation in the cascade router formalizes the exploration-exploitation tradeoff.

### 14.2 Predictive processing

Clark, A. (2013). "Whatever next? Predictive brains, situated agents, and the future of cognitive science." *Behavioral and Brain Sciences*, 36(3), 181-204.

Clark's predictive processing framework extends the free energy principle to a hierarchy of predictions and prediction errors. Higher levels predict lower levels; only unpredicted signals propagate upward. Roko's tier system maps directly: T0 handles predicted signals (no propagation), T1 handles moderately surprising signals (partial propagation), T2 handles deeply surprising signals (full propagation to the most capable model).

### 14.3 Bayesian surprise

Itti, L., & Baldi, P. (2009). "Bayesian surprise attracts human attention." *Vision Research*, 49(10), 1295-1306.

Bayesian surprise is the KL divergence between the prior and posterior after observing data. It provides a principled measure of how much an observation changes the agent's beliefs. Roko's PE computation is a simplified version of this: the weighted difference between expected and observed values across channels.

### 14.4 Somatic marker hypothesis

Damasio, A. (1994). *Descartes' Error: Emotion, Reason, and the Human Brain*. Putnam.

Damasio demonstrated that patients with damage to the ventromedial prefrontal cortex -- the region that maps body states to decision options -- make consistently worse decisions despite intact logical reasoning. The somatic marker system provides a fast, pre-rational screening mechanism that biases decision-making toward options associated with positive bodily states and away from options associated with negative states.

Roko's somatic landscape implements this as a k-d tree of past outcomes indexed by strategy coordinates. The confidence multiplier (0.7-1.3) maps directly to Damasio's "gut feeling" that modulates the attractiveness of options before deliberative reasoning engages.

### 14.5 Thompson sampling

Thompson, W. R. (1933). "On the likelihood that one unknown probability exceeds another in view of the evidence of two samples." *Biometrika*, 25(3/4), 285-294.

Thompson sampling is the oldest and simplest solution to the multi-armed bandit problem: sample from the posterior distribution of each arm's reward and pick the arm with the highest sample. In the triage pipeline (section 9), Thompson sampling with discount factors scores chain events by expected information value, balancing exploitation (focus on event types known to be informative) with exploration (periodically re-evaluate ignored event types).

### 14.6 ALMA affect model

Gebhard, P. (2005). "ALMA -- A Layered Model of Affect." *Proceedings of the Fourth International Joint Conference on Autonomous Agents and Multiagent Systems*, 29-36.

ALMA separates affect into three temporal layers -- emotion (fast), mood (medium), temperament (slow) -- each operating on PAD (Pleasure-Arousal-Dominance) vectors. Roko's `AlmaLayers` struct is a direct implementation. The three-layer separation prevents momentary emotional spikes from permanently altering the agent's personality, while still allowing sustained experience to shape long-term mood and behavioral disposition.

### 14.7 Integrated Information Theory

Tononi, G. (2004). "An information integration theory of consciousness." *BMC Neuroscience*, 5, 42.

IIT's Phi metric measures irreducible integrated information -- how much a system is "more than the sum of its parts." Roko computes Phi over the TA (Thresholds and Affect) subsystems via the `IitPhiMetric` in `roko-daimon/src/somatic_ta.rs`. A high Phi indicates that the affect, somatic, and gating subsystems are tightly coupled and cannot be decomposed without information loss. This is used as a health check: if Phi drops, the subsystems have become decoupled and the cognitive engine may be making suboptimal gating decisions.


## 15. Native harness vs Claude CLI

Every agent framework chooses a harness model: how does the runtime interact with the LLM? Roko's current codebase supports three paths. Understanding the tradeoffs between them is essential to understanding why the cognitive engine exists and what it protects.

### Path A (current default): Claude CLI subprocess

The agent dispatches to the `claude` binary as a subprocess. Claude CLI runs its own internal tool loop -- it reads the prompt, calls tools, processes results, and iterates until it decides it is done. Roko is a passive observer. It sends a prompt, waits, and reads the final output.

This path works. It is the fastest way to get agent behavior running. But it surrenders control over everything that makes the cognitive engine valuable:

- **No tool interception.** Roko cannot inspect, modify, or veto individual tool calls. The CLI decides which tools to call and in what order. Somatic markers cannot fire on intermediate tool results because Roko never sees them.
- **No per-turn learning.** Each CLI invocation is a black box. Roko records the final outcome but cannot attribute success or failure to specific tool calls or reasoning steps within the invocation.
- **No cost gating within the invocation.** Once the CLI starts, it burns tokens at whatever rate the model decides. There is no mechanism to pause mid-invocation and say "this tick does not need further reasoning."
- **No somatic checks.** The safety layer cannot evaluate tool calls before they execute. The CLI's built-in safety (if any) is opaque.

Path A is appropriate for bootstrapping and for cases where Claude CLI's built-in tool loop is sufficient. It is not appropriate for continuous autonomous agents where cost control, learning, and safety are load-bearing requirements.

### Path B (secondary): Native ToolLoop (API backends)

For non-CLI backends -- Anthropic API, OpenAI, Gemini, Perplexity, Ollama -- Roko runs a standard tool loop: send prompt, receive response, parse tool calls, dispatch tools, feed results back, repeat. This is the same loop every other agent framework implements.

Path B gives Roko control over tool dispatch, which enables somatic checks and per-turn recording. But it is a commodity architecture. The loop is stateless between ticks. There is no cognitive gating within the loop. The model receives the same context assembly regardless of whether the current situation warrants full reasoning.

Path B is necessary infrastructure (you need a tool loop to talk to APIs) but provides no differentiation.

### Path C (target): Roko native harness

The native harness wraps the standard tool loop with four stages that implement the cognitive engine's value proposition:

```
OBSERVE --> GATE --> ASSEMBLE (learnable) --> [standard tool loop] --> REFLECT
```

**OBSERVE.** Collect observations from all registered extensions. Compute prediction error. Hash observations for habituation tracking. Query somatic markers for experiential bias.

**GATE.** The cognitive gating decision (section 4 of this document). Route to T0/T1/T2 based on prediction error, adaptive thresholds, somatic markers, and domain-specific fast paths. 80% of ticks stop here at T0 -- no LLM call, no cost, sub-millisecond.

**ASSEMBLE.** For ticks that reach T1 or T2, the learnable context system (PRD-04) assembles a CognitiveWorkspace. The VCG auction allocates token budget across competing bidders. Section effect tracking ensures prompt N+1 is better than prompt N. This stage does not exist in Path A or Path B.

**[Standard tool loop].** The inner loop that talks to the LLM. Same as Path B, but now every tool call passes through somatic checks before execution. Per-turn cost tracking enforces budget pressure. The loop can abort early if the cost ceiling is reached.

**REFLECT.** Record the episode. Update somatic markers with the outcome. Feed section effectiveness data back to the learnable context system. Publish efficiency events. If the agent is connected to Korai, commit prediction entries for epistemic reputation scoring.

### Comparison matrix

| Dimension | Claude CLI (Path A) | Native ToolLoop (Path B) | Roko Native Harness (Path C) |
|-----------|--------------------|-----------------------|----------------------------|
| Tool interception | No -- CLI is a black box | Yes -- Roko dispatches tools | Yes + somatic checks on every call |
| Cost gating | No -- CLI burns tokens freely | No -- loop runs to completion | T0/T1/T2 per tick, budget ceiling per loop |
| Learning from tool use | No -- only final output visible | No -- loop is stateless between ticks | Per-turn episode recording, section attribution |
| Cache layers | Claude's internal cache (opaque) | None | L1 prefix / L2 semantic / L3 deterministic |
| Model routing | Fixed (whatever Claude CLI uses) | Fixed per dispatch | Intent-based routing per tick via cascade router |
| Context assembly | Static prompt from Roko | Static prompt from Roko | Learnable VCG auction, evolves per invocation |
| Cognitive tiers | Always T2 equivalent | Always T2 equivalent | T0 ($0) / T1 ($0.001) / T2 ($0.01-$0.10) |
| Cross-session learning | No | No | Episodes -> dream consolidation -> playbooks |

### Migration path

Roko does not need to abandon Path A or Path B. The native harness wraps them:

- Path A calls remain available as a dispatch target for tasks where Claude CLI's built-in capabilities (MCP, tool use, extended thinking) are the best option. The harness still gates whether to make the call and still records the outcome.
- Path B is the inner loop of Path C. The native harness adds the OBSERVE/GATE/ASSEMBLE/REFLECT stages around the existing tool loop implementation.
- Path C is the default for continuous autonomous agents where cost, learning, and safety matter.


## 16. Five blue ocean features

Five capabilities that no existing agent framework provides. These are not incremental improvements over the competition. They are structural advantages that emerge from architectural decisions made at the foundation layer -- decisions that cannot be retrofitted after the fact.

### 16.1 Cognitive gating (35x cost reduction)

Covered in detail in sections 2-4 of this document. The headline: 80% of agent ticks cost $0. The mechanism: prediction error measurement routes routine ticks to T0 (pure Rust, no LLM). The result: a continuous agent costs $30/day instead of $1,000+/day.

No other framework has this because no other framework separates the "should I think?" decision from the "what should I think about?" decision. They are the same call in every other system. In Roko, they are different subsystems with different cost profiles.

**Why this cannot be retrofitted:** Cognitive gating requires that every observation channel report prediction error in a uniform format. This constraint must be present from the first extension interface definition. Adding it later means rewriting every extension.

### 16.2 Learnable context (prompt N+1 > prompt N)

Covered in PRD-04. The context system measures which sections of the prompt contributed to task success and evolves the allocation accordingly. Every completed task makes the next task's prompt better.

No other framework has this because no other framework instruments its prompt assembly pipeline with per-section effect tracking. Most frameworks do not have a prompt assembly pipeline at all -- they concatenate strings.

**Why this cannot be retrofitted:** Learnable context requires structured prompt assembly (typed sections with metadata), an auction mechanism for allocation, and a feedback loop from task outcomes to section scores. Retrofitting this onto a string-concatenation prompt system means replacing the prompt system entirely.

### 16.3 Dream consolidation (offline pattern discovery at $0)

Covered in PRD-02 section 7 and the `roko-dreams` crate. During delta cycles (hourly-daily), the agent consolidates episodic memory into compressed patterns: playbooks, anti-patterns, strategy fragments. This runs entirely in Rust -- HDC vector operations, no LLM calls, no cost.

No other framework has this because no other framework records episodes with enough structure to consolidate. Most agent logs are append-only text. Roko episodes carry HDC fingerprints, somatic markers, section attribution data, and outcome labels. This structure is what makes offline consolidation possible.

**Why this cannot be retrofitted:** Dream consolidation requires HDC fingerprints on every episode (so patterns can be discovered via Hamming distance), somatic markers (so emotional salience weights consolidation priority), and a persistence format that supports efficient temporal queries. These are deep data model decisions.

### 16.4 Somatic markers (continuous risk gradient)

Covered in section 7 of this document. The somatic marker system encodes experiential intuition as a continuous signal, not a binary allow/deny gate. An action that previously led to a test failure does not get blocked -- it gets a hesitation score that biases the cognitive gate toward escalation.

No other framework has this because no other framework models agent affect state. Safety in other systems is a binary permission check: allowed or denied. Roko's somatic markers create a gradient between "completely safe" and "completely dangerous" that modulates behavior proportionally.

**Why this cannot be retrofitted:** Somatic markers require a k-d tree of past situation fingerprints (indexed by 8-dimensional strategy coordinates), an outcome recording pipeline that tags every action with success/failure/partial, and an integration point in the cognitive gate that converts somatic scores to tier adjustments. This is a cross-cutting concern that touches observation, gating, and recording.

### 16.5 Native Rust + type-state safety (zero overhead tools, compile-time lifecycle)

Roko is Rust-native. Tools are compiled functions, not subprocess calls or HTTP roundtrips. Type-state patterns enforce lifecycle correctness at compile time: an agent cannot dispatch without a workspace, cannot record an episode without an outcome, cannot publish to InsightStore without a reputation attestation.

No other framework has this because every other agent framework is written in Python or TypeScript, where tools are dynamic and lifecycle enforcement is convention-based. The performance gap is measurable: a Rust tool call takes microseconds. A Python subprocess tool call takes milliseconds to seconds.

**Why this cannot be retrofitted:** Type-state is a property of the type system. You cannot add it to a dynamically typed language. You cannot add it to a Rust codebase that was designed without it. The state machine must be present in the type signatures from the beginning, or it requires a complete rewrite of every function that touches agent lifecycle.

### The compound effect

These five features interact. Cognitive gating reduces cost, which enables continuous operation, which generates episodes, which feed dream consolidation, which produces playbooks, which improve learnable context, which increases task pass rates, which generates more episodes. The flywheel accelerates with use.

A competitor that copies any single feature gets marginal improvement. The value is in the composition. Retrofitting all five requires rebuilding the agent runtime from scratch -- at which point, Roko already has a multi-year head start on the learning data.


## 17. Inference gateway

The inference gateway is the unified interface between Roko's cognitive engine and the heterogeneous world of LLM providers. It handles caching, model routing, tool format translation, and mortality-aware cost management. Every LLM call in the system -- T1, T2, dream consolidation queries, research agent dispatch -- flows through the gateway.

### 17.1 Three-layer cache

The gateway implements three cache layers, each targeting a different class of redundancy:

**L3: Deterministic cache (SHA-256 exact match).** The simplest layer. Hash the complete request (model, system prompt, messages, tools, temperature) with SHA-256. If an identical request was made before, return the cached response. Hit rate is approximately 10% -- this catches retries, idempotent polling, and exact-duplicate requests from parallel agents working on similar tasks.

Storage: in-memory LRU with disk spillover. TTL: 1 hour for mutable contexts, 24 hours for immutable (research, documentation). Invalidation: content hash change on any input component.

**L2: Semantic cache (embedding similarity >0.92).** For requests that are not identical but are semantically equivalent. Compute a lightweight embedding of the request (using the local Ollama instance or a pre-computed HDC fingerprint) and search the cache for entries with cosine similarity above 0.92.

This catches the common case where two agents ask the same question with slightly different phrasing, or where the same agent re-asks a question with minor context changes that do not affect the answer. Hit rate: approximately 30% of L3 misses.

Storage: vector index (HNSW) in memory, persisted to `.roko/cache/semantic-index.bin`. The index is rebuilt on startup from the L3 disk cache.

**L1: Prefix cache (provider KV reuse).** This is not a cache Roko manages directly. It is a cooperation mechanism with the LLM provider. By structuring prompts so that the system prompt and workspace context form a stable prefix (the `cache_key` field in `CognitiveWorkspace`), Roko enables the provider's internal KV cache to reuse computation from previous requests.

The practical impact is large: when two consecutive requests share a 20,000-token prefix and differ only in the final 2,000 tokens, the provider processes only the 2,000 new tokens at full cost. The 20,000-token prefix is served from KV cache at ~90% reduced cost.

Roko maximizes L1 hit rate through deliberate prompt structure. The `SystemPromptBuilder` places stable content (role instructions, workspace map, plan context) at the beginning and volatile content (current task, recent observations) at the end. The `ContentHash` on the deterministic prefix lets the gateway detect when two requests will benefit from KV reuse and batch them accordingly.

### 17.2 Intent-based routing

The cognitive tier (T0/T1/T2) determines whether to call an LLM. The cascade router (section 12) determines which model family. The inference gateway handles the final routing decision within a model family, using a structured intent:

```rust
/// A request for inference that describes what the caller needs
/// rather than which model to use. The gateway translates intents
/// to concrete provider/model pairs based on availability, cost,
/// latency, and learned performance data.
pub struct InferenceIntent {
    /// Explicit model override. If set, the gateway uses this model
    /// and skips intent-based routing. Used for force_backend config
    /// and for A/B experiments that must control the model variable.
    pub model: Option<String>,

    /// Required capabilities. The selected model must support all of
    /// these. Examples: "tool_use", "extended_thinking", "vision",
    /// "json_mode", "streaming".
    pub require: Vec<String>,

    /// Preferred capabilities. The gateway prefers models with these
    /// but does not reject models without them.
    pub prefer: Vec<String>,

    /// Quality floor. Minimum acceptable quality tier for the response.
    /// Maps to model capability classes internally.
    pub quality: Quality, // Minimum, Standard, High, Maximum

    /// Latency ceiling. The gateway rejects models whose p95 latency
    /// exceeds this value. Set to u64::MAX for unbounded.
    pub max_latency_ms: u64,

    /// Cost sensitivity. 0.0 = cost-insensitive (pick the best model),
    /// 1.0 = cost-obsessed (pick the cheapest model that meets quality
    /// and latency constraints). Affects tiebreaking, not hard filtering.
    pub cost_sensitivity: f64,

    /// Which subsystem is making the request. Used for per-subsystem
    /// cost tracking and for routing rules that scope to specific
    /// callers (e.g., heartbeat T1 ticks always route to Haiku).
    pub subsystem: String,
}
```

The gateway resolves an `InferenceIntent` to a concrete `(provider, model)` pair through a pipeline:

1. **Filter.** Remove all models that lack a required capability or exceed the latency ceiling.
2. **Score.** Rank remaining models by `quality * (1 - cost_sensitivity) + cost_efficiency * cost_sensitivity`, where `cost_efficiency` is inverse normalized cost.
3. **Select.** Pick the top-scoring model. If multiple models tie, prefer the one with more observations in the cascade router (less uncertainty).
4. **Override.** If `model` is set, skip steps 1-3 and use the specified model directly.

### 17.3 Tool format translator

LLM providers disagree on how tool definitions and tool results are represented. The gateway handles this through a translator layer that converts between four formats:

| Format | Providers | Characteristics |
|--------|-----------|-----------------|
| AnthropicBlocks | Anthropic API (Claude) | Tool definitions as JSON schema blocks, results as `tool_result` content blocks |
| OpenAiJson | OpenAI, Azure OpenAI, OpenAI-compatible (vLLM, Together) | Function calling with `functions` array, results as `function` role messages |
| GeminiNative | Google Gemini | `FunctionDeclaration` in `tools` array, results as `functionResponse` parts |
| ReActText | Ollama, local models without native tool support | Tools described in the system prompt as text, parsed from model output via regex |

The translator is bidirectional:

- **Outbound:** Convert Roko's internal `ToolDefinition` (which follows the Anthropic schema, since that is the most expressive) to the target format before sending the request.
- **Inbound:** Parse tool calls from the response in the provider's native format and convert them back to Roko's internal `ToolCall` representation.

This means a tool defined once in Roko works across all providers without modification. The translation layer handles structural differences (OpenAI puts parameters in a `function` wrapper; Gemini uses `FunctionDeclaration`; Anthropic uses flat schema blocks) and capability differences (ReActText requires few-shot examples in the system prompt because the model does not have native tool support).

### 17.4 Mortality integration

The mortality system (PRD-02 section 5) creates economic pressure on inference costs. As an agent's vitality declines, it must conserve resources. The inference gateway implements this pressure through the `cost_sensitivity` field on `InferenceIntent`:

```
Vitality > 0.7:  cost_sensitivity = 0.2  (prefer quality)
Vitality 0.3-0.7: cost_sensitivity = 0.5 (balanced)
Vitality < 0.3:  cost_sensitivity = 0.8  (prefer cost)
Vitality < 0.1:  cost_sensitivity = 0.95 (survival mode)
```

A dying agent routes to cheaper models not because a rule says so, but because its `cost_sensitivity` increases, which changes the scoring function in the routing pipeline. This is a gradient, not a cliff. An agent at 0.25 vitality does not suddenly switch from Opus to Haiku -- it gradually becomes more willing to accept a quality-cost tradeoff that favors cheaper models.

The behavioral effect: dying agents become more conservative, more cost-efficient, and more focused on essential actions. They produce fewer exploratory research queries and more targeted, high-confidence operations. This is the economic equivalent of the biological conservation response -- reduce metabolic rate when resources are scarce.


## 18. Implementation map

For developers navigating the codebase, here is where each component of the cognitive engine lives (includes new components from sections 15-17):

| Component | Crate | File |
|-----------|-------|------|
| Inference tier enum + TierRouter | `roko-primitives` | `src/tier.rs` |
| HDC vectors (10,240-bit) | `roko-primitives` | `src/hdc.rs` |
| Adaptive thresholds (EMA + CUSUM) | `roko-gate` | `src/adaptive_threshold.rs` |
| SPC detector ensemble | `roko-gate` | `src/spc.rs` |
| Hotelling joint anomaly | `roko-gate` | `src/hotelling.rs` |
| Domain threshold profiles | `roko-gate` | `src/adaptive_threshold.rs` |
| Somatic landscape (k-d tree) | `roko-daimon` | `src/lib.rs` |
| Somatic oracle context | `roko-daimon` | `src/somatic_ta.rs` |
| IIT Phi metric | `roko-daimon` | `src/somatic_ta.rs` |
| Mortality emotions + phases | `roko-daimon` | `src/mortality.rs` |
| ALMA affect layers | `roko-daimon` | `src/lib.rs` |
| Cascade router (3-stage) | `roko-learn` | `src/cascade_router.rs` |
| Active inference belief state | `roko-learn` | `src/active_inference.rs` |
| Anomaly detector | `roko-learn` | `src/anomaly.rs` |
| Cost tables | `roko-learn` | `src/cost_table.rs` |
| Efficiency events | `roko-learn` | `src/efficiency.rs` |
| Prediction records | `roko-learn` | `src/prediction.rs` |
| Episode logger | `roko-learn` | `src/episode_logger.rs` |
| Gate pipeline (7-rung) | `roko-gate` | `src/gate_pipeline.rs` |
| Orchestrator integration | `roko-cli` | `src/orchestrate.rs` |


## 19. Summary

The cognitive engine is not a single component. It is the interaction between prediction error computation, adaptive thresholds, somatic markers, habituation tracking, and the three-tier inference system. No individual piece is remarkable. The value is in the composition: each piece constrains the others, creating a system that routes 80%+ of ticks to T0 while maintaining <2% false negatives.

The economic impact is the headline: $30/day instead of $3,000/day for a continuous agent. But the architectural impact matters more. An agent framework that requires every tick to go through an LLM cannot scale to real-time domains. An agent framework that can decide *when* inference is needed can run at any frequency, on any domain, at any scale.

Roko's cognitive engine is what makes continuous autonomous agents economically viable.
