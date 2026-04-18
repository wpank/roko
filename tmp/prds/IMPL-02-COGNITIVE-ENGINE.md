# IMPL-02: Cognitive engine

Implements PRD-03. Target crates: `roko-gate`, `roko-learn`, `roko-daimon`, `roko-primitives`.

## Context

Roko is a Rust workspace at `/Users/will/dev/nunchi/roko/roko/` with 18 crates. It builds
agents that build themselves: read PRDs, generate plans, execute tasks via Claude agents,
validate with gates, and persist results.

This plan adds a prediction-error-driven cognitive engine that decides how much compute each
agent tick deserves. Today, every tick gets the same treatment. After this work, stable ticks
(repeated patterns, passing gates) suppress to T0 (no LLM call), while novel or failing ticks
escalate to T2 (full Opus). The result: 80%+ of ticks on a stable workload cost nothing, and
the system self-tunes via habituation, somatic markers, and adaptive thresholds.

### Tier model

The tier system already exists in `roko-primitives`:

| Tier | What happens | Model |
|------|-------------|-------|
| T0 | Suppress inference entirely | None (heuristics only) |
| T1 | Light inference | `claude-haiku-4-5` |
| T2 | Full inference | `claude-opus-4-6` (vitality >= 0.3) or `claude-sonnet-4` |

**Source**: `crates/roko-primitives/src/tier.rs`, lines 22-31. The `TierRouter::select_model`
function (line 67) maps tier + vitality to a model slug.

### Key existing code

| Component | File | What exists |
|-----------|------|-------------|
| Adaptive thresholds | `crates/roko-gate/src/adaptive_threshold.rs` | EMA + CUSUM + SPC + Hotelling per gate rung. 626 lines. |
| Cascade router | `crates/roko-learn/src/cascade_router.rs` | 3-stage model router (Static/Confidence/UCB1). |
| Active inference | `crates/roko-learn/src/active_inference.rs` | `BeliefState` over 90 latent states, EFE tier selection. |
| Somatic landscape | `crates/roko-daimon/src/lib.rs` | `SomaticLandscape` with k-d tree (`kiddo`), `SomaticMarker`, `query_somatic()`. |
| HDC vectors | `crates/roko-primitives/src/hdc.rs` | 10,240-bit binary vectors. XOR bind, majority bundle, Hamming similarity. |
| Tier routing | `crates/roko-primitives/src/tier.rs` | `InferenceTier` enum (T0/T1/T2), `TierRouter` model selection. |
| Threshold profiles | `crates/roko-gate/src/adaptive_threshold.rs` lines 74-163 | Domain profiles (coding/research/security) with per-rung priors. |
| SPC detectors | `crates/roko-gate/src/spc.rs` | CUSUM + EWMA Control Chart + BOCPD ensemble. |
| Hotelling detector | `crates/roko-gate/src/hotelling.rs` | Multi-gate joint anomaly detection (T-squared). |

---

## Phase 1: Prediction error system

**Goal**: Define the core data types that aggregate multi-source observations into a scalar
prediction error (PE). PE is the primary signal that drives tier selection.

### Task 1.1: Define `Observation` struct

**File**: `crates/roko-gate/src/cognitive.rs` (new file)

**Read first**:
- `crates/roko-gate/src/adaptive_threshold.rs` lines 27-55 (`RungStats` for prior art on tracking)
- `crates/roko-primitives/src/tier.rs` lines 22-31 (`InferenceTier` enum)

**Do**:
1. Create `crates/roko-gate/src/cognitive.rs`
2. Define the `Observation` struct:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    /// Which subsystem generated this observation.
    pub source: ObservationSource,
    /// Observed value, normalized to [0.0, 1.0].
    pub value: f64,
    /// Expected value (prediction), normalized to [0.0, 1.0].
    pub expected: f64,
    /// How much this observation contributes to the aggregate PE.
    /// Higher weight = this source matters more. Range [0.0, 1.0].
    pub weight: f64,
    /// Timestamp of the observation.
    pub observed_at: chrono::DateTime<chrono::Utc>,
}
```

3. Define `ObservationSource` enum:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ObservationSource {
    /// Gate pass/fail result.
    Gate { rung: u32 },
    /// Test suite result (pass count / total).
    TestSuite,
    /// Clippy lint count (normalized).
    Clippy,
    /// Diff size relative to expected.
    DiffSize,
    /// Cost relative to budget.
    CostBudget,
    /// Latency relative to SLA.
    LatencySla,
    /// Research source conflict rate.
    SourceConflict,
    /// External signal (e.g., chain event).
    External { tag: u32 },
}
```

4. Implement `Observation::prediction_error(&self) -> f64`:
   - Return `(self.value - self.expected).abs() * self.weight`

5. Register the module in `crates/roko-gate/src/lib.rs`:
   - Add `pub mod cognitive;`

**Test**: Unit test in `cognitive.rs`:
- Construct 3 observations with known values and weights
- Verify each `prediction_error()` returns the correct scalar
- Verify an observation where `value == expected` returns 0.0

- [ ] `Observation` struct defined with all fields
- [ ] `ObservationSource` enum covers gate, test, clippy, diff, cost, latency, research, external
- [ ] `prediction_error()` returns correct scalar
- [ ] Module registered in `lib.rs`
- [ ] Unit test passes

---

### Task 1.2: Define `PredictionErrorComputer`

**File**: `crates/roko-gate/src/cognitive.rs` (append)

**Read first**:
- Task 1.1 output (the `Observation` struct)
- `crates/roko-gate/src/adaptive_threshold.rs` lines 321-372 (`observe` method for EMA pattern)

**Do**:
1. Define `PredictionErrorComputer`:

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PredictionErrorComputer {
    /// EMA of the aggregate PE over recent ticks.
    pub ema_pe: f64,
    /// EMA smoothing factor. Default 0.15.
    pub alpha: f64,
    /// Number of ticks processed.
    pub tick_count: u64,
    /// Per-source running weight for domain-specific tuning.
    pub source_weights: HashMap<String, f64>,
}
```

2. Implement `PredictionErrorComputer::compute(&mut self, observations: &[Observation]) -> f64`:
   - Sum `obs.prediction_error()` for all observations
   - Divide by `observations.len().max(1)` to get mean PE for this tick
   - Update `ema_pe` using EMA formula: `ema_pe = alpha * mean_pe + (1 - alpha) * ema_pe`
   - Increment `tick_count`
   - Return `ema_pe`

3. Implement `PredictionErrorComputer::raw_pe(observations: &[Observation]) -> f64`:
   - Static method, no EMA. Returns the raw mean PE for one batch.
   - Useful for testing and one-shot queries.

**Test**:
- Inject 10 observations with known PE values
- Verify `compute()` returns EMA that converges toward the mean
- Verify `raw_pe()` returns the exact mean for a single batch
- Verify that repeated identical observations cause EMA to converge

- [ ] `PredictionErrorComputer` struct defined
- [ ] `compute()` aggregates observations via weighted mean + EMA
- [ ] `raw_pe()` returns exact mean for one batch
- [ ] EMA converges toward steady state after repeated calls
- [ ] Unit tests pass

---

### Task 1.3: Domain-specific PE sources

**File**: `crates/roko-gate/src/cognitive.rs` (append)

**Read first**:
- Task 1.2 output
- `crates/roko-gate/src/adaptive_threshold.rs` lines 74-163 (`ThresholdProfile` for domain patterns)

**Do**:
1. Define `fn observations_from_gate_result(rung: u32, passed: bool, ema_pass_rate: f64) -> Observation`:
   - `value` = 1.0 if passed, 0.0 if failed
   - `expected` = `ema_pass_rate` (from `AdaptiveThresholds`)
   - `weight` = varies by rung: rung 0 (compile) = 0.3, rung 2 (test) = 0.5, others = 0.2
   - `source` = `ObservationSource::Gate { rung }`

2. Define `fn observations_from_cost(actual_usd: f64, budget_usd: f64) -> Observation`:
   - `value` = `(actual_usd / budget_usd).clamp(0.0, 1.0)`
   - `expected` = 0.5 (expected to use half the budget)
   - `weight` = 0.15
   - `source` = `ObservationSource::CostBudget`

3. Define `fn observations_from_latency(actual_ms: f64, sla_ms: f64) -> Observation`:
   - `value` = `(actual_ms / sla_ms).clamp(0.0, 1.0)`
   - `expected` = 0.5
   - `weight` = 0.1
   - `source` = `ObservationSource::LatencySla`

**Test**:
- Gate pass with high expected rate (0.95) produces low PE
- Gate fail with high expected rate produces high PE
- Cost at exactly 50% of budget produces PE = 0.0
- Cost at 100% of budget produces PE > 0

- [ ] `observations_from_gate_result` converts gate results to observations
- [ ] `observations_from_cost` converts cost data to observations
- [ ] `observations_from_latency` converts latency data to observations
- [ ] All domain converters produce correct PE for boundary cases
- [ ] Unit tests pass

---

### Task 1.4: Integration test for PE computation

**File**: `crates/roko-gate/tests/cognitive_integration.rs` (new file)

**Read first**:
- `crates/roko-gate/src/cognitive.rs` (all of tasks 1.1-1.3)
- `crates/roko-gate/src/adaptive_threshold.rs` lines 204-216 (`AdaptiveThresholds::new()`)

**Do**:
1. Create the integration test file
2. Build 4 scenarios:

**Scenario A: Stable workload**
- 20 gate passes at rung 0, 1, 2 (all passing)
- Cost at 40% of budget, latency at 30% of SLA
- Assert: aggregate PE < 0.15

**Scenario B: Single gate failure**
- 19 passes + 1 fail at rung 2 (test)
- Assert: PE spikes above 0.3 on the failure tick
- Assert: PE decays back below 0.2 after 5 more passing ticks

**Scenario C: Systematic degradation**
- 10 passes, then 10 fails at rung 2
- Assert: PE monotonically increases during the failure run
- Assert: Final PE > 0.4

**Scenario D: Cost pressure**
- All gates pass, but cost at 95% of budget
- Assert: PE > 0.1 (cost alone raises the floor)

3. Run with `cargo test -p roko-gate --test cognitive_integration`

- [ ] Integration test file created
- [ ] Scenario A: stable workload produces low PE
- [ ] Scenario B: single failure spikes PE, then decays
- [ ] Scenario C: systematic degradation raises PE monotonically
- [ ] Scenario D: cost pressure alone raises PE floor
- [ ] All scenarios pass

---

## Phase 2: Cognitive gate

**Goal**: Build the gate that maps PE to an inference tier (T0/T1/T2). This is the decision
boundary that controls compute spend per tick.

### Task 2.1: Define `CognitiveGate` trait and default implementation

**File**: `crates/roko-gate/src/cognitive.rs` (append)

**Read first**:
- `crates/roko-primitives/src/tier.rs` lines 22-31 (`InferenceTier`)
- `crates/roko-gate/src/adaptive_threshold.rs` lines 166-194 (`AdaptiveThresholds` struct)
- Task 1.2 output (`PredictionErrorComputer`)

**Do**:
1. Define the trait:

```rust
pub trait CognitiveGate: Send + Sync {
    /// Given the current PE state and observations, decide which tier to use.
    fn gate(
        &self,
        pe_computer: &PredictionErrorComputer,
        observations: &[Observation],
        thresholds: &AdaptiveThresholds,
    ) -> InferenceTier;
}
```

2. Define `DefaultCognitiveGate`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultCognitiveGate {
    /// PE below this -> T0 (suppress).
    pub t0_ceiling: f64,
    /// PE above this -> T2 (deliberate).
    pub t2_floor: f64,
    /// Everything between -> T1 (analyze).
}
```

Default values: `t0_ceiling = 0.15`, `t2_floor = 0.40`.

3. Implement `CognitiveGate` for `DefaultCognitiveGate`:
   - If `pe_computer.ema_pe < self.t0_ceiling` -> `InferenceTier::T0`
   - If `pe_computer.ema_pe > self.t2_floor` -> `InferenceTier::T2`
   - Otherwise -> `InferenceTier::T1`

4. Add adaptive threshold integration:
   - If any rung has `cusum_shift_detected == true` (from `AdaptiveThresholds`), force minimum T1
   - If `thresholds.joint_anomaly_detected()`, force T2

**Test**:
- PE = 0.05 -> T0
- PE = 0.25 -> T1
- PE = 0.55 -> T2
- PE = 0.05 but CUSUM shift detected -> T1 (override)
- PE = 0.05 but joint anomaly -> T2 (override)

- [ ] `CognitiveGate` trait defined
- [ ] `DefaultCognitiveGate` implements the trait with configurable thresholds
- [ ] CUSUM shift forces minimum T1
- [ ] Joint anomaly forces T2
- [ ] Unit tests cover all tier boundaries and overrides

---

### Task 2.2: Wire adaptive thresholds into the cognitive gate

**File**: `crates/roko-gate/src/cognitive.rs` (modify `DefaultCognitiveGate`)

**Read first**:
- `crates/roko-gate/src/adaptive_threshold.rs` lines 291-313 (`threshold_for`, `override_for_role`)
- `crates/roko-gate/src/adaptive_threshold.rs` lines 557-611 (temperament adjustments)
- Task 2.1 output

**Do**:
1. Add `temperament: Option<Temperament>` field to `DefaultCognitiveGate`
2. When temperament is `Conservative`:
   - Lower `t0_ceiling` by 20% (harder to suppress)
   - Raise `t2_floor` by 10% (easier to deliberate -- no, harder to suppress means lower ceiling is correct but we also want to escalate more readily for conservative, so lower the t2_floor)
   - Correction: Conservative should be more cautious. Lower `t2_floor` by 15% so it escalates sooner.
3. When temperament is `Aggressive`:
   - Raise `t0_ceiling` by 25% (suppress more aggressively)
   - Raise `t2_floor` by 20% (harder to escalate)
4. When temperament is `Exploratory`:
   - Lower both thresholds by 10% (escalate more, observe more)

5. Add method `fn effective_thresholds(&self) -> (f64, f64)` that returns adjusted `(t0_ceiling, t2_floor)`.

**Test**:
- Conservative at PE = 0.13 (below default T0 ceiling 0.15) -> T1 (not suppressed)
- Aggressive at PE = 0.18 (above default T0 ceiling 0.15) -> T0 (still suppressed)
- Exploratory at PE = 0.35 -> T2 (escalated earlier than default)

- [ ] Temperament field added to `DefaultCognitiveGate`
- [ ] `effective_thresholds()` adjusts T0/T2 boundaries per temperament
- [ ] Conservative is harder to suppress, easier to escalate
- [ ] Aggressive suppresses more, escalates less
- [ ] Exploratory escalates earlier
- [ ] Unit tests pass for each temperament variant

---

### Task 2.3: Integration test for tier routing

**File**: `crates/roko-gate/tests/cognitive_integration.rs` (append)

**Read first**:
- Tasks 2.1, 2.2 output
- `crates/roko-primitives/src/tier.rs` lines 60-79 (`TierRouter::select_model`)

**Do**:
1. Add integration test: full pipeline from observations -> PE -> tier -> model selection
2. Scenarios:

**Scenario E: Stable workload -> T0 -> no model**
- 50 passing gate observations
- Assert: `gate()` returns `InferenceTier::T0`
- Assert: `TierRouter::select_model(T0, 1.0)` returns `None`

**Scenario F: Novel failure -> T2 -> Opus**
- 20 passes then 5 fails in quick succession
- Assert: `gate()` returns `InferenceTier::T2`
- Assert: `TierRouter::select_model(T2, 0.8)` returns `Some("claude-opus-4-6")`

**Scenario G: Low vitality T2 -> Sonnet**
- Same as F but vitality = 0.1
- Assert: `TierRouter::select_model(T2, 0.1)` returns `Some("claude-sonnet-4")`

3. Run: `cargo test -p roko-gate --test cognitive_integration`

- [ ] Scenario E: stable -> T0 -> None
- [ ] Scenario F: novel failure -> T2 -> Opus
- [ ] Scenario G: low vitality T2 -> Sonnet
- [ ] All integration tests pass

---

## Phase 3: Habituation

**Goal**: Repeated patterns should produce decreasing PE over time. This is how the system
learns that a pattern is "normal" and stops escalating for it.

### Task 3.1: Define `HabituationMask`

**File**: `crates/roko-gate/src/cognitive.rs` (append)

**Read first**:
- `crates/roko-primitives/src/hdc.rs` lines 1-30 (HDC vector basics, `HdcVector`)
- Task 1.1 output (`Observation`, `ObservationSource`)

**Do**:
1. Define `HabituationMask`:

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HabituationMask {
    /// Blake3 hash of observation pattern -> encounter count.
    pattern_counts: HashMap<[u8; 32], u64>,
    /// Half-life: after this many encounters, PE attenuation reaches 50%.
    pub half_life: u64,
    /// Maximum attenuation factor. PE is multiplied by at least (1 - max_attenuation).
    pub max_attenuation: f64,
}
```

Default: `half_life = 10`, `max_attenuation = 0.8`.

2. Implement `HabituationMask::fingerprint(observations: &[Observation]) -> [u8; 32]`:
   - Hash the sorted `(source, value_bucket, expected_bucket)` tuples using Blake3
   - `value_bucket` = `(value * 10.0).floor() as u8` (quantize to 0.1 resolution)
   - This groups "close enough" observation patterns together

3. Implement `HabituationMask::attenuation(&mut self, observations: &[Observation]) -> f64`:
   - Compute fingerprint
   - Increment count for that fingerprint
   - Return attenuation factor: `min(max_attenuation, 1.0 - 0.5^(count / half_life))`
   - The PE should be multiplied by `(1.0 - attenuation)` before tier gating

4. Implement `HabituationMask::reset(&mut self, fingerprint: &[u8; 32])`:
   - Remove the fingerprint (sensitization after surprise)

**Dependency**: Add `blake3` to `crates/roko-gate/Cargo.toml`. Check if it is already a
workspace dependency in the root `Cargo.toml` first:
`grep blake3 /Users/will/dev/nunchi/roko/roko/Cargo.toml`

**Test**:
- First encounter: attenuation = 0.0 (no suppression)
- After `half_life` encounters: attenuation ~= 0.5
- After `3 * half_life` encounters: attenuation approaching `max_attenuation`
- After `reset()`: attenuation returns to 0.0
- Different observation patterns produce different fingerprints

- [ ] `HabituationMask` struct with Blake3 fingerprinting
- [ ] `fingerprint()` quantizes observations and hashes them
- [ ] `attenuation()` returns increasing suppression for repeated patterns
- [ ] `reset()` clears the count for a fingerprint (sensitization)
- [ ] `blake3` dependency added
- [ ] Unit tests pass

---

### Task 3.2: Wire habituation into PE computation

**File**: `crates/roko-gate/src/cognitive.rs` (modify `PredictionErrorComputer`)

**Read first**:
- Task 3.1 output
- Task 1.2 output (`PredictionErrorComputer::compute`)

**Do**:
1. Add `habituation: HabituationMask` field to `PredictionErrorComputer`
2. In `compute()`, after calculating raw mean PE:
   - Compute `attenuation = self.habituation.attenuation(observations)`
   - Multiply mean PE by `(1.0 - attenuation)` before applying EMA
3. Add `fn sensitize(&mut self, observations: &[Observation])`:
   - Computes fingerprint and calls `self.habituation.reset(&fingerprint)`
   - Called when a gate fails unexpectedly (PE spike should not be attenuated)

**Test**:
- 20 identical observation batches -> PE decreases each time
- After sensitize -> next identical batch produces full PE again
- Mixed batches (alternating patterns) -> each pattern habituates independently

- [ ] `HabituationMask` integrated into `PredictionErrorComputer`
- [ ] Repeated patterns produce decreasing PE
- [ ] `sensitize()` resets attenuation for a specific pattern
- [ ] Different patterns habituate independently
- [ ] Unit tests pass

---

### Task 3.3: Integration test for habituation

**File**: `crates/roko-gate/tests/cognitive_integration.rs` (append)

**Read first**:
- Tasks 3.1, 3.2 output

**Do**:
1. Scenario H: Repeated stable pattern
   - 30 ticks of identical gate results (all pass)
   - Assert: PE on tick 1 > PE on tick 15 > PE on tick 30
   - Assert: tier on tick 30 = T0 (suppressed)

2. Scenario I: Sensitization after failure
   - 20 ticks stable (habituated to T0)
   - Tick 21: gate fails -> call sensitize
   - Tick 22: same pattern as ticks 1-20 but now PE is back to full
   - Assert: tier on tick 22 >= T1

3. Run: `cargo test -p roko-gate --test cognitive_integration`

- [ ] Scenario H: habituation reduces PE over time
- [ ] Scenario I: sensitization restores full PE
- [ ] Integration tests pass

---

## Phase 4: Somatic integration

**Goal**: Past failures leave emotional traces (somatic markers) in the daimon's k-d tree.
When the current situation is geometrically close to a past failure, the cognitive gate
should escalate regardless of the raw PE.

### Task 4.1: Wire somatic marker query into the cognitive gate

**File**: `crates/roko-gate/src/cognitive.rs` (modify `DefaultCognitiveGate`)

**Read first**:
- `crates/roko-daimon/src/lib.rs` lines 1261-1280 (`SomaticMarker` struct)
- `crates/roko-daimon/src/lib.rs` lines 1364-1370 (`SomaticLandscape` struct)
- `crates/roko-daimon/src/lib.rs` lines 1452-1458 (`query` method)
- `crates/roko-daimon/src/lib.rs` lines 2105-2110 (`query_somatic` method)

**Do**:
1. Add a new `SomaticOverride` struct:

```rust
#[derive(Debug, Clone, Default)]
pub struct SomaticOverride {
    /// Average valence of nearest somatic markers. Negative = past failures.
    pub mean_valence: f64,
    /// Average intensity of nearest markers.
    pub mean_intensity: f64,
    /// Number of markers found within the query radius.
    pub neighbor_count: usize,
}
```

2. Add method to `DefaultCognitiveGate`:

```rust
pub fn apply_somatic_override(
    &self,
    base_tier: InferenceTier,
    somatic: &SomaticOverride,
) -> InferenceTier
```

Logic:
- If `somatic.mean_valence < -0.3` AND `somatic.mean_intensity > 0.5` AND `somatic.neighbor_count >= 2`:
  - Force at least T1
  - If `somatic.mean_valence < -0.6`: force T2
- Otherwise: return `base_tier` unchanged

3. Add a conversion function:

```rust
pub fn somatic_override_from_signal(signal: &roko_daimon::SomaticSignal) -> SomaticOverride
```

This bridges the daimon's `SomaticSignal` type to the gate's `SomaticOverride`.

**Dependency**: Add `roko-daimon` as an optional dependency of `roko-gate` behind a
`daimon` feature flag. Check current dependencies first:
`grep roko-daimon /Users/will/dev/nunchi/roko/roko/crates/roko-gate/Cargo.toml`

**Test**:
- No somatic data -> base tier unchanged
- Weak negative valence (-0.2) -> no override
- Strong negative valence (-0.5), high intensity (0.7), 3 neighbors -> T1 minimum
- Very strong negative valence (-0.8) -> T2 forced
- Positive valence -> no override

- [ ] `SomaticOverride` struct defined
- [ ] `apply_somatic_override` escalates tier based on past-failure proximity
- [ ] `somatic_override_from_signal` bridges daimon types to gate types
- [ ] Feature-gated `roko-daimon` dependency
- [ ] Unit tests cover all override boundaries

---

### Task 4.2: Integration test for somatic escalation

**File**: `crates/roko-gate/tests/cognitive_integration.rs` (append)

**Read first**:
- Task 4.1 output
- `crates/roko-daimon/src/lib.rs` lines 1396-1450 (`store_marker`, `add_marker`)

**Do**:
1. Scenario J: Past failure fingerprint forces escalation
   - Create a `SomaticLandscape`
   - Store 3 markers at strategy coordinates `[0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5]`
     with negative valence (-0.7) and high intensity (0.8)
   - Query at the same coordinates
   - Convert the `SomaticSignal` to `SomaticOverride`
   - Assert: override forces T2 even though raw PE is low (0.05)

2. Scenario K: Distant failure does not override
   - Same markers at `[0.5, ...]`
   - Query at `[0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]` (far away)
   - Assert: override returns base tier unchanged (no nearby markers)

3. Run: `cargo test -p roko-gate --test cognitive_integration`

- [ ] Scenario J: nearby negative markers force T2
- [ ] Scenario K: distant markers do not override
- [ ] Integration tests pass

---

## Phase 5: Triage pipeline (blockchain T0)

**Goal**: For blockchain/chain domains, most events are noise. A triage pipeline filters
95%+ of events at T0 using rules, anomaly detection, and Thompson sampling.

### Task 5.1: Define `TriagePipeline`

**File**: `crates/roko-gate/src/triage.rs` (new file)

**Read first**:
- `crates/roko-gate/src/cognitive.rs` (all of phases 1-4)
- `crates/roko-primitives/src/tier.rs` (tier model)

**Do**:
1. Create `crates/roko-gate/src/triage.rs`
2. Define the pipeline:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriagePipeline {
    pub stages: Vec<TriageStage>,
    /// Events processed.
    pub total_events: u64,
    /// Events suppressed at T0.
    pub suppressed_events: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriageStage {
    /// Rule-based filter using a bloom filter of known-safe patterns.
    RuleFilter {
        /// Blake3 hashes of safe event patterns.
        safe_patterns: HashSet<[u8; 32]>,
    },
    /// Anomaly detection: flag events whose features deviate from the running mean.
    AnomalyDetector {
        /// Running mean of event feature values.
        feature_means: Vec<f64>,
        /// Running variance.
        feature_variances: Vec<f64>,
        /// Number of observations.
        observations: u64,
        /// Z-score threshold for flagging anomalies.
        z_threshold: f64,
    },
    /// Enrichment: tag the event with metadata from recent context.
    Enrichment,
    /// Thompson sampling scorer: decide suppress vs escalate.
    ThompsonScorer {
        /// Beta(alpha, beta) for the suppress arm.
        suppress_alpha: f64,
        suppress_beta: f64,
        /// Beta(alpha, beta) for the escalate arm.
        escalate_alpha: f64,
        escalate_beta: f64,
    },
}
```

3. Implement `TriagePipeline::process(&mut self, event_features: &[f64]) -> InferenceTier`:
   - Stage 1 (RuleFilter): hash the features, check bloom filter. If safe -> T0.
   - Stage 2 (AnomalyDetector): compute z-score. If all features within threshold -> T0.
   - Stage 3 (Enrichment): no-op for now (placeholder for future context tagging).
   - Stage 4 (ThompsonScorer): use deterministic Thompson approximation
     (same as `LearningBidder` in `crates/roko-compose/src/auction.rs` line 159).
     If suppress wins -> T0. Otherwise -> T1.

4. Implement `TriagePipeline::update(&mut self, event_features: &[f64], outcome_was_important: bool)`:
   - Update anomaly detector means/variances
   - Update Thompson scorer posteriors

5. Register in `crates/roko-gate/src/lib.rs`: `pub mod triage;`

**Test**:
- Known-safe event pattern -> T0 at stage 1
- Normal event (within z-threshold) -> T0 at stage 2
- Anomalous event (z-score > threshold) -> passes to Thompson scorer
- After many unimportant events, Thompson scorer biases toward T0

- [ ] `TriagePipeline` with 4 stages defined
- [ ] `process()` runs events through all stages
- [ ] Rule filter catches known-safe patterns
- [ ] Anomaly detector flags outliers
- [ ] Thompson scorer learns to suppress unimportant events
- [ ] Module registered in `lib.rs`
- [ ] Unit tests pass

---

### Task 5.2: Integration test for triage

**File**: `crates/roko-gate/tests/triage_integration.rs` (new file)

**Read first**:
- Task 5.1 output

**Do**:
1. Scenario L: 95% suppression rate
   - Generate 1000 events: 950 from a "normal" distribution, 50 "anomalous"
   - Process all through the pipeline
   - Update the scorer: normal events were unimportant, anomalous were important
   - Assert: after training, >= 950 of 1000 new normal events return T0
   - Assert: >= 40 of 50 anomalous events return T1 or T2

2. Scenario M: Cold start
   - No training data
   - Assert: pipeline does not crash, returns reasonable defaults (T1 for unknown events)

3. Run: `cargo test -p roko-gate --test triage_integration`

- [ ] Scenario L: 95%+ suppression for normal events after training
- [ ] Scenario M: cold start returns sensible defaults
- [ ] Integration tests pass

---

## Phase 6: Cost tracking and mortality

**Goal**: Cost pressure feeds back into the cognitive gate. When the budget is running low
(low vitality), the gate becomes stricter -- more T0, fewer T2 calls.

### Task 6.1: Per-tick cost tracking

**File**: `crates/roko-gate/src/cognitive.rs` (append)

**Read first**:
- `crates/roko-primitives/src/tier.rs` lines 52-53 (`T2_VITALITY_THRESHOLD`)
- `crates/roko-learn/src/cascade_router.rs` lines 1-50 (cost-aware routing)

**Do**:
1. Define `CostTracker`:

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CostTracker {
    /// Total budget allocated (USD).
    pub total_budget_usd: f64,
    /// Total spent so far (USD).
    pub spent_usd: f64,
    /// Per-tick cost history (last 100 ticks).
    pub recent_costs: VecDeque<f64>,
    /// EMA of per-tick cost.
    pub ema_cost: f64,
}
```

2. Implement `CostTracker::record_tick(&mut self, cost_usd: f64)`:
   - Add to `spent_usd`
   - Push to `recent_costs` (cap at 100)
   - Update `ema_cost`

3. Implement `CostTracker::vitality(&self) -> f64`:
   - Return `(1.0 - spent_usd / total_budget_usd.max(0.01)).clamp(0.0, 1.0)`
   - This is the fraction of budget remaining.

4. Implement `CostTracker::adjust_gate_thresholds(&self, base_t0: f64, base_t2: f64) -> (f64, f64)`:
   - When vitality < 0.3: raise `t0_ceiling` by 30% (suppress more)
   - When vitality < 0.1: raise `t0_ceiling` by 60%, raise `t2_floor` by 40% (emergency conservation)
   - When vitality > 0.7: no adjustment (plenty of budget)

**Test**:
- 50% budget spent -> vitality 0.5, no threshold adjustment
- 80% budget spent -> vitality 0.2, T0 ceiling raised 30%
- 95% budget spent -> vitality 0.05, emergency conservation active
- Zero budget -> vitality 0.0, maximum conservation

- [ ] `CostTracker` struct defined
- [ ] `record_tick()` tracks spending and EMA
- [ ] `vitality()` returns budget fraction remaining
- [ ] `adjust_gate_thresholds()` raises T0 ceiling under budget pressure
- [ ] Emergency conservation kicks in below 10% vitality
- [ ] Unit tests pass

---

### Task 6.2: Wire cost tracker into the full cognitive pipeline

**File**: `crates/roko-gate/src/cognitive.rs` (append)

**Read first**:
- Task 6.1 output
- All previous phase outputs

**Do**:
1. Define `CognitivePipeline` that composes all components:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitivePipeline {
    pub pe_computer: PredictionErrorComputer,
    pub gate: DefaultCognitiveGate,
    pub cost_tracker: CostTracker,
    #[serde(skip)]
    pub somatic_override: Option<SomaticOverride>,
}
```

2. Implement `CognitivePipeline::decide(&mut self, observations: &[Observation], thresholds: &AdaptiveThresholds) -> InferenceTier`:
   - Compute PE via `pe_computer.compute(observations)`
   - Apply cost-based threshold adjustments
   - Call `gate.gate()` with adjusted thresholds
   - Apply somatic override if present
   - Return final tier

3. Implement `CognitivePipeline::save(&self, path: &Path)` and `load(path: &Path)`:
   - JSON persistence to `.roko/learn/cognitive-pipeline.json`

**Test**: Full end-to-end test:
- 100 ticks of mixed observations
- Verify tier decisions are consistent with PE, habituation, cost, and somatic state
- Verify persistence round-trip

- [ ] `CognitivePipeline` composes all components
- [ ] `decide()` runs the full pipeline: PE -> cost adjust -> gate -> somatic override
- [ ] Persistence round-trip works
- [ ] End-to-end test with 100 ticks

---

### Task 6.3: Final integration test

**File**: `crates/roko-gate/tests/cognitive_integration.rs` (append)

**Read first**:
- All phase outputs

**Do**:
1. Scenario N: Stable workload, 80%+ T0
   - 200 ticks, all gates pass, cost within budget, no somatic markers
   - Assert: >= 160 ticks assigned T0
   - Assert: remaining ticks are T1 (early ticks before habituation kicks in)

2. Scenario O: Mixed workload with failure recovery
   - 50 stable ticks -> T0
   - 10 failure ticks -> T2
   - 50 stable ticks -> T0 resumes (re-habituation)
   - Assert: tier transitions happen at the right boundaries

3. Scenario P: Budget exhaustion
   - 100 ticks with 80% budget consumed in first 50 ticks
   - Assert: T0 rate in ticks 51-100 is higher than ticks 1-50
   - Assert: no T2 calls in the last 20 ticks (emergency conservation)

4. Run: `cargo test -p roko-gate --test cognitive_integration`

- [ ] Scenario N: 80%+ T0 on stable workload
- [ ] Scenario O: correct tier transitions on failure/recovery
- [ ] Scenario P: budget exhaustion triggers conservation
- [ ] All integration tests pass

---

## Phase 7: Native harness integration

**Goal**: Replace the Claude CLI dispatch path with a native harness that wraps the ToolLoop with structured stages and somatic interception. This gives the cognitive engine direct control over every tool call, enabling somatic checks before execution and structured OBSERVE/GATE/ASSEMBLE/REFLECT stages per tick.

### Task 7.1: Define `NativeHarness` struct

**File**: `crates/roko-gate/src/native_harness.rs` (new file)

**Read first**:
- `crates/roko-agent/src/dispatcher/mod.rs` -- `ToolLoop`, `dispatch_claude_api()`, tool call sequence
- `crates/roko-daimon/src/lib.rs` lines 1452-1458 (`query_somatic`)
- `crates/roko-gate/src/cognitive.rs` (all phases 1-6)
- `crates/roko-primitives/src/tier.rs` (`InferenceTier`)

**Do**:
1. Create `crates/roko-gate/src/native_harness.rs`
2. Define the harness:

```rust
#[derive(Debug)]
pub struct NativeHarness {
    /// The cognitive pipeline that decides tier per tick.
    pub cognitive: CognitivePipeline,
    /// Somatic landscape for pre-execution checks.
    pub somatic: Option<SomaticLandscape>,
    /// Adaptive thresholds for gate integration.
    pub thresholds: AdaptiveThresholds,
    /// Stage timings for the current tick.
    pub stage_timings: StageTiming,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StageTiming {
    pub observe_ms: u64,
    pub gate_ms: u64,
    pub assemble_ms: u64,
    pub reflect_ms: u64,
    pub total_ms: u64,
}

/// The four stages of a native harness tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HarnessStage {
    /// Collect observations from environment and prior state.
    Observe,
    /// Run cognitive gate to decide tier (T0/T1/T2).
    Gate,
    /// Assemble context and dispatch tool calls (with somatic intercept).
    Assemble,
    /// Reflect on outcomes, update PE, record episode.
    Reflect,
}
```

3. Register in `crates/roko-gate/src/lib.rs`: `pub mod native_harness;`

**Test**: Unit test: construct a `NativeHarness` with default `CognitivePipeline` and no somatic. Verify all fields accessible.

- [ ] `NativeHarness` struct defined with cognitive pipeline, somatic, thresholds, stage timings
- [ ] `HarnessStage` enum with 4 stages
- [ ] `StageTiming` tracks per-stage latency
- [ ] Module registered in `lib.rs`

---

### Task 7.2: Wire native harness as default dispatch path

**File**: `crates/roko-gate/src/native_harness.rs` (append)

**Read first**:
- Task 7.1 output
- `crates/roko-agent/src/dispatcher/mod.rs` -- `dispatch_claude_api()` for the API backend path
- `crates/roko-cli/src/orchestrate.rs` -- `dispatch_agent_with()`, the current dispatch entry point

**Do**:
1. Implement `NativeHarness::tick()`:

```rust
impl NativeHarness {
    pub async fn tick(
        &mut self,
        observations: &[Observation],
        tool_loop: &mut ToolLoop,
        context: &TickContext,
    ) -> Result<TickOutcome> {
        let start = Instant::now();

        // OBSERVE: collect observations
        let observe_start = Instant::now();
        let all_obs = self.collect_observations(observations, context);
        self.stage_timings.observe_ms = observe_start.elapsed().as_millis() as u64;

        // GATE: decide tier
        let gate_start = Instant::now();
        let tier = self.cognitive.decide(&all_obs, &self.thresholds);
        self.stage_timings.gate_ms = gate_start.elapsed().as_millis() as u64;

        // If T0: skip LLM call entirely
        if tier == InferenceTier::T0 {
            self.stage_timings.total_ms = start.elapsed().as_millis() as u64;
            return Ok(TickOutcome::Suppressed { tier, pe: self.cognitive.pe_computer.ema_pe });
        }

        // ASSEMBLE: build context + dispatch tool calls with somatic intercept
        let assemble_start = Instant::now();
        let result = self.assemble_and_dispatch(tool_loop, context, &tier).await?;
        self.stage_timings.assemble_ms = assemble_start.elapsed().as_millis() as u64;

        // REFLECT: update PE, record episode
        let reflect_start = Instant::now();
        self.reflect(&result, &all_obs);
        self.stage_timings.reflect_ms = reflect_start.elapsed().as_millis() as u64;

        self.stage_timings.total_ms = start.elapsed().as_millis() as u64;
        Ok(TickOutcome::Completed { tier, result, timings: self.stage_timings.clone() })
    }
}
```

2. Define `TickContext` struct: task description, domain, role, prior state.
3. Define `TickOutcome` enum: `Suppressed { tier, pe }`, `Completed { tier, result, timings }`.
4. Wire into `orchestrate.rs`: when the agent backend is an API backend (not Claude CLI), route through `NativeHarness::tick()` instead of the raw dispatcher.

**Files to modify**:
- `crates/roko-gate/src/native_harness.rs`
- `crates/roko-cli/src/orchestrate.rs` (add feature-gated path)

**Test**:
- With a low-PE state (all gates passing), `tick()` returns `Suppressed` at T0.
- With a high-PE state (gate failures), `tick()` returns `Completed` at T2.
- Stage timings are populated.

- [ ] `tick()` implements the 4-stage pipeline
- [ ] T0 suppression skips LLM call
- [ ] `TickContext` and `TickOutcome` defined
- [ ] Wired into orchestrate.rs for API backends
- [ ] Stage timings tracked

---

### Task 7.3: Implement somatic check on every tool call

**File**: `crates/roko-gate/src/native_harness.rs` (append)

**Read first**:
- Task 7.2 output
- `crates/roko-daimon/src/lib.rs` lines 2105-2110 (`query_somatic`)
- Task 4.1 (`SomaticOverride`, `apply_somatic_override`)

**Do**:
1. Implement `NativeHarness::somatic_intercept()`:

```rust
impl NativeHarness {
    /// Check the somatic landscape before executing a tool call.
    /// Returns None if safe to proceed, or Some(warning) if past failures
    /// are geometrically close to the current situation.
    pub fn somatic_intercept(
        &self,
        tool_name: &str,
        tool_args: &serde_json::Value,
        task_domain: &str,
    ) -> Option<SomaticWarning> {
        let landscape = self.somatic.as_ref()?;

        // Encode the tool call + domain into strategy coordinates
        let coords = encode_tool_context(tool_name, tool_args, task_domain);

        // Query the k-d tree for nearby markers
        let signal = landscape.query_somatic(&coords, 5);

        // If nearby markers have strong negative valence, warn
        if signal.mean_valence < -0.3 && signal.mean_intensity > 0.5 && signal.neighbor_count >= 2 {
            Some(SomaticWarning {
                mean_valence: signal.mean_valence,
                mean_intensity: signal.mean_intensity,
                neighbor_count: signal.neighbor_count,
                tool_name: tool_name.to_string(),
                recommendation: if signal.mean_valence < -0.6 {
                    SomaticAction::Block
                } else {
                    SomaticAction::WarnAndProceed
                },
            })
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct SomaticWarning {
    pub mean_valence: f64,
    pub mean_intensity: f64,
    pub neighbor_count: usize,
    pub tool_name: String,
    pub recommendation: SomaticAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SomaticAction {
    /// Safe to proceed.
    Proceed,
    /// Past failures nearby -- proceed with elevated tier.
    WarnAndProceed,
    /// Strong negative signal -- block the tool call and escalate to T2.
    Block,
}
```

2. Wire `somatic_intercept()` into the `assemble_and_dispatch()` method: before each tool call, query the somatic landscape. If `Block`, skip the tool call and report the block. If `WarnAndProceed`, escalate tier to T1 minimum and log the warning.

3. Implement `encode_tool_context()`: hash tool name + first 3 argument keys into strategy coordinates using `HdcVector::from_seed` projected to 8 floats.

**Dependency**: Add `roko-daimon` as an optional dependency of `roko-gate` (check if already added from Task 4.1).

**Test**:
- No somatic landscape -> no intercept (all calls proceed).
- Landscape with negative markers at the coordinates matching `write_file` + `critical-path` -> `SomaticAction::Block`.
- Landscape with mild negative markers -> `SomaticAction::WarnAndProceed`.
- Landscape with positive markers -> no warning.

- [ ] `somatic_intercept()` queries k-d tree before each tool call
- [ ] `Block` prevents tool execution and escalates tier
- [ ] `WarnAndProceed` logs warning but allows execution
- [ ] `encode_tool_context()` maps tool+args+domain to strategy coordinates

---

### Task 7.4: Integration test for native harness

**File**: `crates/roko-gate/tests/native_harness_integration.rs` (new file)

**Read first**:
- Tasks 7.1 through 7.3
- All cognitive pipeline tasks (phases 1-6)

**Do**:

1. **Scenario Q: Stable workload through native harness**
   - Create a `NativeHarness` with default pipeline
   - Run 50 ticks with all-passing observations
   - Assert: >40 ticks return `Suppressed` (T0) after habituation kicks in
   - Assert: stage timings show observe + gate < 5ms for T0 ticks

2. **Scenario R: Somatic intercept blocks dangerous tool call**
   - Populate somatic landscape with negative markers for `write_file` + `production` domain
   - Run a tick that produces a `write_file` tool call in `production` domain
   - Assert: somatic intercept returns `Block`
   - Assert: tick outcome does not include the blocked tool call

3. **Scenario S: Full pipeline with tier escalation**
   - Run 20 stable ticks (habituate to T0)
   - Inject a gate failure on tick 21
   - Assert: tick 21 escalates to T2
   - Assert: PE spikes, then decays over subsequent stable ticks
   - Assert: T0 resumes within 10 ticks

4. Run: `cargo test -p roko-gate --test native_harness_integration`

- [ ] Scenario Q: >80% T0 on stable workload, <5ms per T0 tick
- [ ] Scenario R: somatic intercept blocks dangerous tool calls
- [ ] Scenario S: failure -> T2 escalation -> recovery -> T0
- [ ] All integration tests pass

---

## Acceptance criteria

- [ ] PE computation produces correct scalar from multi-source observations
- [ ] Gate correctly routes: low PE -> T0, medium -> T1, high -> T2
- [ ] Habituation reduces PE for repeated patterns
- [ ] Somatic markers increase PE for past-failure fingerprints
- [ ] Triage pipeline filters 95%+ chain events at T0
- [ ] Cost tracking adjusts gate thresholds under budget pressure
- [ ] End-to-end: agent with gating uses 80%+ T0 ticks on stable workload
- [ ] All structs are `Serialize + Deserialize` for persistence
- [ ] All new code has doc comments
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] `cargo test -p roko-gate` passes

## Files created or modified

| File | Action |
|------|--------|
| `crates/roko-gate/src/cognitive.rs` | **New**. Core PE, gate, habituation, cost, pipeline. |
| `crates/roko-gate/src/triage.rs` | **New**. Blockchain T0 triage pipeline. |
| `crates/roko-gate/src/lib.rs` | **Modified**. Register `cognitive` and `triage` modules. |
| `crates/roko-gate/Cargo.toml` | **Modified**. Add `blake3` and optional `roko-daimon` dep. |
| `crates/roko-gate/tests/cognitive_integration.rs` | **New**. Integration tests for phases 1-4, 6. |
| `crates/roko-gate/tests/triage_integration.rs` | **New**. Integration tests for phase 5. |

## Build and test commands

```bash
# Build
cd /Users/will/dev/nunchi/roko/roko
cargo build -p roko-gate

# Unit tests
cargo test -p roko-gate

# Integration tests
cargo test -p roko-gate --test cognitive_integration
cargo test -p roko-gate --test triage_integration

# Full workspace check
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
```
