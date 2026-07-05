# 06 -- Heartbeat: Work Batches

> **Scope**: Wire heartbeat ticks to DeFi consumers and build the 9-step decision pipeline.
> **Batches**: 4 | **Total effort**: M + XL + L + L

---

## Batch 6.1: Wire heartbeat clock to DeFi consumers

> **Effort**: M | **Depends on**: 1.2 (WS subscription) | **Crate**: roko-runtime
> **Branch**: `defi/batch-6.1-clock-wiring`

### Context

Roko's heartbeat clock works. `HeartbeatPolicy` in `roko-runtime/src/heartbeat.rs:724` spawns three concurrent `tokio::time::interval` loops -- gamma (5-15s), theta (15-120s), delta (300s) -- and publishes `HeartbeatTick` events on the `EventBus<RokoEvent>` (line 881). The `CorticalState` surface at line 228 carries 20+ atomic signals (PAD, regime, gas_gwei, resource_health). The clock configuration is adaptive: `compute_gamma_interval` (line 914) shortens gamma under anomaly load, `compute_theta_interval` (line 923) compresses theta under volatile/crisis regime.

The problem: no DeFi consumer subscribes to these ticks. The bus emits `RokoEvent::HeartbeatTick` events, but the only consumers are the TUI dashboard and the conductor's code-task watchers. Chain events (new blocks, price updates, position changes) do not feed the `CorticalState`, and tick consumers do not trigger DeFi actions.

This batch wires the existing heartbeat into DeFi use. It connects chain event subscriptions (from batch 1.2's WS provider) to the `CorticalState` surface, implements `WakeupCondition` triggers from chain events, and adds DeFi-interval presets to `ClockConfig`.

**Deployment model**: The heartbeat clock runs in the roko control plane (Railway, always-on). For in-process agents (monitoring, research, risk-assessor, safety-guardian), the heartbeat loop runs directly in the control-plane process and ticks feed the agent's decision pipeline via in-memory channels. For isolated agents on Fly Machines (trading, coding), the heartbeat runs locally inside the Fly Machine and connects back to the shared `NeuroStore` via the control-plane's REST API (`POST /api/neuro/query`, `POST /api/neuro/ingest`). Both paths produce identical `DecisionCycleRecord` events that flow to the shared event bus.

### Read First

| File | Why |
|------|-----|
| `crates/roko-runtime/src/heartbeat.rs:596-644` | `ClockConfig` struct and defaults |
| `crates/roko-runtime/src/heartbeat.rs:700-722` | `WakeupCondition` enum -- built but no consumers |
| `crates/roko-runtime/src/heartbeat.rs:724-911` | `HeartbeatPolicy` -- the full clock implementation |
| `crates/roko-runtime/src/heartbeat.rs:226-365` | `CorticalState` -- the lock-free perception surface |
| `crates/roko-runtime/src/event_bus.rs:104-178` | `RokoEvent` enum -- available event types |
| `crates/roko-runtime/src/heartbeat_probes.rs:244-298` | `EngineState` -- metrics consumed by probes |

### Conventions

- **Module files**: one file per type, flat in `src/`, declared in `lib.rs` as `pub mod xxx;`
- **Errors**: use `thiserror` for domain errors, convert to `RokoError` at boundaries
- **Tests**: inline `#[cfg(test)] mod tests {}`, min 3 tests per new type
- **Lints**: `cargo clippy --no-deps -- -D warnings` must pass clean
- **Docs**: `///` on all public items, `//!` module-level doc in new files
- **Derives**: `#[derive(Debug, Clone)]` minimum; add `Serialize, Deserialize` if persisted
- **Mirage testing**: integration tests that need chain state should use ephemeral mirage-rs instances. Pattern: `spawn_mirage_test_instance(Some(rpc_url), Some(fork_block)).await?` from `apps/mirage-rs/src/integration.rs`. Connect via `MirageClient::new(instance.config()).await?`. Shut down with `instance.shutdown().await?`. Add `mirage-rs` as a dev-dependency feature gate.

### Work Items

**6.1.1 -- Add DeFi interval presets to ClockConfig**

Extend `ClockConfig` in `crates/roko-runtime/src/heartbeat.rs:596` with a factory method for DeFi-tuned intervals. Do not change the Default impl (it serves code-task orchestration). Add:

```rust
impl ClockConfig {
    /// DeFi-tuned intervals: faster gamma for price feeds, tighter theta for strategy.
    pub fn defi_preset() -> Self {
        Self {
            gamma_base_interval_secs: 2,
            gamma_min_interval_secs: 1,
            gamma_max_interval_secs: 5,
            theta_base_interval_secs: 15,
            theta_min_interval_secs: 5,
            theta_max_interval_secs: 60,
            theta_gamma_count: 5,
            delta_episode_threshold: 50,
            delta_idle_timeout_secs: 120,
            daily_budget_usd: 50.0,
            throttle_at_percent: 80,
            hard_stop_at_percent: 95,
            scheduler_poll_interval_millis: 500,
        }
    }
}
```

**6.1.2 -- Build chain event consumer**

Create `crates/roko-runtime/src/chain_consumer.rs`:

```rust
//! Chain event consumer that feeds CorticalState from chain subscriptions.

use std::sync::Arc;
use tokio::sync::mpsc;

use crate::heartbeat::{CorticalState, HeartbeatPolicy, Regime, WakeupCondition};

/// A chain event received from the WS subscription (batch 1.2).
#[derive(Debug, Clone)]
pub enum ChainEvent {
    /// New block with number and timestamp.
    NewBlock { number: u64, timestamp: u64 },
    /// Price update for a tracked asset.
    PriceUpdate { asset: String, price_usd: f64 },
    /// Gas price change.
    GasUpdate { gwei: f64 },
    /// Position health factor change.
    PositionHealth { position_id: String, health_factor: f64 },
}

/// Consumes chain events and updates CorticalState + triggers wakeups.
pub struct ChainEventConsumer {
    cortical: Arc<CorticalState>,
    heartbeat: Arc<HeartbeatPolicy>,
}

impl ChainEventConsumer {
    pub fn new(cortical: Arc<CorticalState>, heartbeat: Arc<HeartbeatPolicy>) -> Self {
        Self { cortical, heartbeat }
    }

    /// Process a single chain event. Updates CorticalState atomics and
    /// triggers a wakeup if the event is urgent.
    pub fn process(&self, event: &ChainEvent) {
        match event {
            ChainEvent::GasUpdate { gwei } => {
                self.cortical.set_gas_gwei(*gwei as f32);
                if *gwei > 500.0 {
                    self.heartbeat.wakeup(WakeupCondition::SafetyAlert);
                }
            }
            ChainEvent::PositionHealth { health_factor, .. } => {
                if *health_factor < 1.2 {
                    self.heartbeat.wakeup(WakeupCondition::SafetyAlert);
                }
            }
            _ => {}
        }
    }

    /// Run a consumer loop that reads from an mpsc channel.
    pub async fn run(self, mut rx: mpsc::Receiver<ChainEvent>) {
        while let Some(event) = rx.recv().await {
            self.process(&event);
        }
    }
}
```

**6.1.3 -- Add chain-domain WakeupCondition variants**

Extend the `WakeupCondition` enum in `crates/roko-runtime/src/heartbeat.rs:702` with DeFi-specific conditions:

```rust
/// Large price movement detected on a tracked asset.
PriceAnomaly {
    /// Asset identifier.
    asset: String,
    /// Percentage deviation from EMA.
    deviation_pct: f32,
},
/// Position health factor dropped below warning threshold.
LiquidationRisk {
    /// Position identifier.
    position_id: String,
    /// Current health factor.
    health_factor: f32,
},
```

These new variants are serializable and carry enough context for the tick pipeline to act on them without re-querying the chain.

**6.1.4 -- Wire module into lib.rs**

Add `pub mod chain_consumer;` to `crates/roko-runtime/src/lib.rs` (after line 49, alongside `heartbeat_probes`).

**Warning**: The `ChainEvent` type defined here is a runtime-internal event, not the external event type from batch 1.2's WS provider. The chain provider sends raw JSON; a thin adapter (built later) converts into `ChainEvent`. Do not make `ChainEvent` depend on alloy or ethers types.

### Wiring

- `crates/roko-runtime/src/lib.rs`: add `pub mod chain_consumer;`
- `crates/roko-runtime/src/heartbeat.rs`: add `defi_preset()` to `ClockConfig`, add two `WakeupCondition` variants
- `crates/roko-runtime/src/chain_consumer.rs`: new file

### Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::heartbeat::{ClockConfig, PersonalityPreset};

    #[test]
    fn test_defi_preset_faster_than_default() {
        let defi = ClockConfig::defi_preset();
        let default = ClockConfig::default();
        assert!(defi.gamma_base_interval_secs < default.gamma_base_interval_secs);
        assert!(defi.theta_base_interval_secs < default.theta_base_interval_secs);
    }

    #[test]
    fn test_gas_update_sets_cortical_state() {
        let cortical = Arc::new(CorticalState::new(PersonalityPreset::Balanced));
        // Process a GasUpdate event. Assert cortical.gas_gwei() reflects the new value.
    }

    #[test]
    fn test_high_gas_triggers_wakeup() {
        // Process a GasUpdate with gwei > 500.
        // Assert a HeartbeatTick::Gamma was emitted on the bus.
    }

    #[test]
    fn test_low_health_factor_triggers_wakeup() {
        // Process a PositionHealth event with health_factor < 1.2.
        // Assert wakeup fires.
    }
}
```

### Verification

```bash
cargo test -p roko-runtime -- chain_consumer
cargo test -p roko-runtime -- heartbeat
cargo clippy -p roko-runtime --no-deps -- -D warnings
cargo +nightly fmt --check -p roko-runtime
```

### Acceptance Criteria

- [ ] `ClockConfig::defi_preset()` returns intervals faster than `Default::default()`
- [ ] `ChainEventConsumer` updates `CorticalState::gas_gwei` from `GasUpdate` events
- [ ] Gas anomalies (>500 gwei) trigger `WakeupCondition::SafetyAlert` and emit early gamma
- [ ] Low health factors (<1.2) trigger `WakeupCondition::SafetyAlert`
- [ ] `PriceAnomaly` and `LiquidationRisk` wakeup variants serialize/deserialize
- [ ] Module wired into `roko-runtime/src/lib.rs`
- [ ] All tests pass, clippy clean, fmt clean

### Commit Message

```
feat(roko-runtime): wire heartbeat clock to DeFi chain event consumers
```

---

## Batch 6.2: 9-step decision pipeline

> **Effort**: XL | **Depends on**: 6.1, 5.1 (archetypes for delegation) | **Crate**: roko-runtime
> **Branch**: `defi/batch-6.2-decision-pipeline`

### Context

The heartbeat clock emits ticks. This batch builds what runs on each tick: the 9-step decision pipeline from the PRD.

The nine steps are: OBSERVE, RETRIEVE, ANALYZE, GATE, SIMULATE, VALIDATE, EXECUTE, VERIFY, REFLECT. Steps 5-8 are conditional. In a T0 tick (no LLM call, ~80% of ticks), only steps 1-4 and 9 execute. This tiered approach keeps ~80% of ticks at $0.00 inference cost.

The tier gating decision happens in step 4 (GATE). It consumes the prediction error computed in step 3 (ANALYZE) and routes to T0 (FSM rules, no model), T1 (cheap model like Haiku), or T2 (strong model like Sonnet/Opus). The threshold is adaptive: it modulates based on strategy confidence, affect arousal, and resource health from the `CorticalState`.

Existing infrastructure that this batch builds on:
- `HeartbeatProbe` trait in `roko-runtime/src/heartbeat_probes.rs:136` evaluates deterministic probes returning `f32` values
- `EngineState` at line 250 carries the metrics that probes consume (tracked assets, positions, gas, RSI, MACD)
- `InferenceTier` from `roko_primitives::tier` provides T0/T1/T2/T3 classification
- `CorticalState::snapshot()` at line 342 gives an eventually-consistent read of all cognitive signals
- `ChainHeartbeatExtension` in `roko-chain/src/heartbeat_ext.rs:151` provides SIMULATE + VALIDATE steps but is currently unwired

The pipeline is a struct with a `tick()` method. Each call to `tick()` runs all nine steps and returns a `DecisionCycleRecord`. The pipeline holds references to the probe registry, knowledge store (for RETRIEVE), archetype registry (for EXECUTE delegation), and chain extension (for SIMULATE/VALIDATE).

### Read First

| File | Why |
|------|-----|
| `crates/roko-runtime/src/heartbeat_probes.rs:126-199` | `HeartbeatProbe` trait and `EngineState` -- step 1 infrastructure |
| `crates/roko-runtime/src/heartbeat.rs:226-365` | `CorticalState` and `CorticalSnapshot` -- shared perception surface |
| `crates/roko-runtime/src/heartbeat.rs:54-56` | `InferenceTier` re-export from roko_primitives |
| `crates/roko-chain/src/heartbeat_ext.rs:151-194` | `ChainHeartbeatExtension::pre_act_check` -- SIMULATE+VALIDATE steps |
| `crates/roko-learn/src/budget.rs:1-40` | `BudgetGuardrail` -- budget enforcement |
| `crates/roko-runtime/src/heartbeat_attention.rs:1-73` | Attention auction -- context allocation primitives |
| `crates/roko-runtime/src/event_bus.rs:104-178` | `RokoEvent` variants -- what the pipeline can emit |

### Conventions

- **Module files**: one file per type, flat in `src/`, declared in `lib.rs` as `pub mod xxx;`
- **Errors**: use `thiserror` for domain errors, convert to `RokoError` at boundaries
- **Tests**: inline `#[cfg(test)] mod tests {}`, min 3 tests per new type
- **Lints**: `cargo clippy --no-deps -- -D warnings` must pass clean
- **Docs**: `///` on all public items, `//!` module-level doc in new files
- **Derives**: `#[derive(Debug, Clone)]` minimum; add `Serialize, Deserialize` if persisted
- **Mirage testing**: integration tests that need chain state should use ephemeral mirage-rs instances. Pattern: `spawn_mirage_test_instance(Some(rpc_url), Some(fork_block)).await?` from `apps/mirage-rs/src/integration.rs`. Connect via `MirageClient::new(instance.config()).await?`. Shut down with `instance.shutdown().await?`. Add `mirage-rs` as a dev-dependency feature gate.

### Work Items

**6.2.1 -- Define `DecisionCycleRecord`**

Create `crates/roko-runtime/src/decision_cycle.rs`:

```rust
//! Decision cycle record: the output of a single heartbeat tick pipeline.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::heartbeat::{HeartbeatSpeed, InferenceTier, Regime};

/// The output of one complete 9-step decision cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionCycleRecord {
    /// Tick sequence number.
    pub tick_id: u64,
    /// Tick speed (gamma/theta/delta).
    pub speed: HeartbeatSpeed,
    /// Regime at the time of the tick.
    pub regime: Regime,
    /// Computed prediction error (0.0..=1.0).
    pub prediction_error: f32,
    /// Selected inference tier.
    pub tier: InferenceTier,
    /// Whether an action was executed (steps 5-8 ran).
    pub acted: bool,
    /// Whether verification passed (step 8).
    pub verified: Option<bool>,
    /// Cost incurred by this tick in USD.
    pub cost_usd: f64,
    /// Total duration of the tick pipeline.
    pub duration_ms: u64,
    /// Per-step timing breakdown.
    pub step_timings: StepTimings,
    /// UTC timestamp when the tick started.
    pub started_at: DateTime<Utc>,
}

/// Timing breakdown for each step of the pipeline.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StepTimings {
    pub observe_ms: u64,
    pub retrieve_ms: u64,
    pub analyze_ms: u64,
    pub gate_ms: u64,
    pub simulate_ms: Option<u64>,
    pub validate_ms: Option<u64>,
    pub execute_ms: Option<u64>,
    pub verify_ms: Option<u64>,
    pub reflect_ms: u64,
}
```

**6.2.2 -- Define `PredictionError` computation**

Add to `crates/roko-runtime/src/decision_cycle.rs`:

```rust
/// Sources contributing to the aggregate prediction error.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PredictionErrorSources {
    /// Price divergence from causal model (weight: 0.30).
    pub price_divergence: f32,
    /// Regime change signal (weight: 0.40).
    pub regime_change: f32,
    /// Position health delta >10% (weight: 0.20).
    pub position_health_delta: f32,
    /// Probe anomaly count (weight: 0.05 each, max 2).
    pub probe_anomalies: f32,
}

impl PredictionErrorSources {
    /// Compute the weighted aggregate prediction error, capped at 1.0.
    pub fn aggregate(&self) -> f32 {
        let raw = self.price_divergence * 0.30
            + self.regime_change * 0.40
            + self.position_health_delta * 0.20
            + self.probe_anomalies * 0.10;
        raw.clamp(0.0, 1.0)
    }
}
```

**6.2.3 -- Build the `TickPipeline` struct**

Add to `crates/roko-runtime/src/decision_cycle.rs`:

```rust
use std::sync::Arc;
use std::time::Instant;

use crate::heartbeat::{CorticalState, HeartbeatTick};
use crate::heartbeat_probes::{EngineState, HeartbeatProbe};

/// Configuration for the tick pipeline's tier gating.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierGatingConfig {
    /// Base prediction error threshold for T0 -> T1 escalation.
    pub t0_t1_threshold: f32,
    /// Multiplier for T1 -> T2 escalation (threshold * multiplier).
    pub t1_t2_multiplier: f32,
}

impl Default for TierGatingConfig {
    fn default() -> Self {
        Self {
            t0_t1_threshold: 0.3,
            t1_t2_multiplier: 2.0,
        }
    }
}

/// The 9-step decision pipeline that runs on each heartbeat tick.
pub struct TickPipeline {
    /// Probes evaluated during OBSERVE.
    probes: Vec<Box<dyn HeartbeatProbe>>,
    /// Shared perception surface.
    cortical: Arc<CorticalState>,
    /// Tier gating configuration.
    gating: TierGatingConfig,
}

impl TickPipeline {
    pub fn new(
        probes: Vec<Box<dyn HeartbeatProbe>>,
        cortical: Arc<CorticalState>,
        gating: TierGatingConfig,
    ) -> Self {
        Self { probes, cortical, gating }
    }

    /// Run the full 9-step pipeline for a single tick.
    pub async fn tick(
        &self,
        heartbeat_tick: &HeartbeatTick,
        engine_state: &EngineState,
    ) -> DecisionCycleRecord {
        let started_at = Utc::now();
        let start = Instant::now();
        let mut timings = StepTimings::default();

        // Step 1: OBSERVE -- run probes, collect anomalies
        let step_start = Instant::now();
        let probe_results = self.observe(engine_state);
        timings.observe_ms = step_start.elapsed().as_millis() as u64;

        // Step 2: RETRIEVE -- pull knowledge
        // Query knowledge store for similar past market states.
        // Uses MirageTestHarness::get_pool_state() data encoded via MarketHdcEncoder (batch 10.1)
        // to find nearest-neighbor entries in NeuroStore.
        let step_start = Instant::now();
        self.retrieve();
        timings.retrieve_ms = step_start.elapsed().as_millis() as u64;

        // Step 3: ANALYZE -- compute prediction error
        let step_start = Instant::now();
        let pe_sources = self.analyze(&probe_results);
        let prediction_error = pe_sources.aggregate();
        timings.analyze_ms = step_start.elapsed().as_millis() as u64;

        // Step 4: GATE -- select inference tier
        let step_start = Instant::now();
        let tier = self.gate(prediction_error);
        timings.gate_ms = step_start.elapsed().as_millis() as u64;

        let mut acted = false;
        let mut verified = None;
        let mut cost_usd = 0.0;

        // Steps 5-8 run only for T1/T2
        if tier != InferenceTier::T0 {
            // Step 5: SIMULATE (T2 only) -- uses ephemeral mirage-rs
            if tier == InferenceTier::T2 || tier == InferenceTier::T3 {
                let step_start = Instant::now();
                // Delegate to MirageSimulator (impl TxSimulator) or
                // ChainHeartbeatExtension::pre_act_check
                timings.simulate_ms = Some(step_start.elapsed().as_millis() as u64);
            }

            // Step 6: VALIDATE
            // Run DeFiRiskEngine::check_limits() against current portfolio + proposed action.
            // If validation fails, demote to T0 (observe-only) and log the rejection reason.
            let step_start = Instant::now();
            timings.validate_ms = Some(step_start.elapsed().as_millis() as u64);

            // Step 7: EXECUTE
            // Dispatch the action through the agent's tool handler.
            // For DeFi: call VenueAdapter.swap/add_liquidity/remove_liquidity via ToolDispatcher.
            // The tool call goes through DeFiRiskEngine::simulate_trade() first (batch 4.1).
            let step_start = Instant::now();
            acted = true;
            timings.execute_ms = Some(step_start.elapsed().as_millis() as u64);

            // Step 8: VERIFY
            // Confirm the executed action matches the simulation.
            // Compare actual gas_used, output_amount, balance changes against SimulateResult.
            // If discrepancy > threshold, emit RiskAlert and trigger DeFi circuit breaker (batch 4.4).
            let step_start = Instant::now();
            verified = Some(true); // Placeholder
            timings.verify_ms = Some(step_start.elapsed().as_millis() as u64);
        }

        // Step 9: REFLECT
        let step_start = Instant::now();
        self.reflect(prediction_error, tier);
        timings.reflect_ms = step_start.elapsed().as_millis() as u64;

        DecisionCycleRecord {
            tick_id: heartbeat_tick.tick_id,
            speed: heartbeat_tick.speed,
            regime: heartbeat_tick.regime,
            prediction_error,
            tier,
            acted,
            verified,
            cost_usd,
            duration_ms: start.elapsed().as_millis() as u64,
            step_timings: timings,
            started_at,
        }
    }

    fn observe(&self, state: &EngineState) -> Vec<f32> {
        self.probes.iter().map(|p| p.evaluate(state)).collect()
    }

    fn retrieve(&self) {
        // Stub: will query neuro store in future batch
    }

    fn analyze(&self, probe_results: &[f32]) -> PredictionErrorSources {
        // Count anomalous probes (values > 0.7 as simple threshold)
        let anomaly_count = probe_results.iter().filter(|&&v| v > 0.7).count();
        PredictionErrorSources {
            probe_anomalies: (anomaly_count as f32 * 0.5).clamp(0.0, 1.0),
            ..Default::default()
        }
    }

    fn gate(&self, prediction_error: f32) -> InferenceTier {
        let snapshot = self.cortical.snapshot();
        let threshold = self.adaptive_threshold(&snapshot);
        if prediction_error < threshold {
            InferenceTier::T0
        } else if prediction_error < threshold * self.gating.t1_t2_multiplier {
            InferenceTier::T1
        } else {
            InferenceTier::T2
        }
    }

    fn adaptive_threshold(&self, snapshot: &crate::heartbeat::CorticalSnapshot) -> f32 {
        let base = self.gating.t0_t1_threshold;
        let arousal = snapshot.pad.arousal as f32;
        let arousal_factor = 1.0 - arousal.clamp(0.0, 1.0) * 0.2;
        let resource_factor = 1.0 + (1.0 - snapshot.resource_health) * 0.3;
        base * arousal_factor * resource_factor
    }

    fn reflect(&self, prediction_error: f32, tier: InferenceTier) {
        // Update CorticalState with latest prediction accuracy
        self.cortical.set_prediction_accuracy(1.0 - prediction_error);
    }
}
```

**6.2.4 -- Add `RokoEvent::DecisionCycle` variant**

Extend `RokoEvent` in `crates/roko-runtime/src/event_bus.rs` to carry decision cycle records:

```rust
/// Emitted after a complete 9-step decision cycle finishes.
DecisionCycle(DecisionCycleRecord),
```

This import requires adding `use crate::decision_cycle::DecisionCycleRecord;` to the event_bus module.

**6.2.5 -- Add `MirageSimulator` for step 5 (SIMULATE)**

The SIMULATE step uses ephemeral mirage-rs instances for transaction simulation. Add to `crates/roko-runtime/src/decision_cycle.rs`:

> **Note**: `TxSimulator` is the trait defined at `crates/roko-chain/src/heartbeat_ext.rs:151`. `TxRequest` and `SimulateResult` are defined in Batch 0.1 (`crates/roko-chain/src/mirage_simulator.rs`). `MirageClient` comes from `apps/mirage-rs/src/integration.rs`. The `MirageSimulator` here is a thin wrapper; the full implementation lives in Batch 0.1. This work item wires it into the decision pipeline.

```rust
/// Transaction simulator backed by ephemeral mirage-rs instances.
/// Implements the TxSimulator trait so the tick pipeline can simulate
/// proposed transactions against forked chain state without broadcasting.
pub struct MirageSimulator {
    harness: MirageTestHarness,
}

impl MirageSimulator {
    /// Create a new simulator. Spawns an ephemeral mirage-rs instance
    /// forked at the current block. Caller must call `shutdown()` when done.
    pub async fn new(rpc_url: &str) -> ChainResult<Self> {
        let harness = MirageTestHarness::fork_at_block(rpc_url, None).await?;
        Ok(Self { harness })
    }
}

impl Drop for MirageSimulator {
    fn drop(&mut self) {
        // Best-effort shutdown. Prefer explicit shutdown() for deterministic cleanup.
        // Mirage process will be cleaned up by OS if drop runs without shutdown.
    }
}

#[async_trait]
impl TxSimulator for MirageSimulator {
    async fn simulate(&self, tx: &TxRequest) -> Result<SimulateResult> {
        let snapshot = self.client.evm_snapshot().await?;
        let receipt = self.client.eth_send_transaction(tx.clone()).await?;
        self.client.evm_revert(snapshot).await?;
        Ok(SimulateResult::from_receipt(receipt))
    }
}
```

The `MirageSimulator` is instantiated per-tick when step 5 runs. The mirage-rs instance is spawned ephemerally by the control plane (or by the Fly Machine for isolated agents) and shut down after the tick completes. This keeps simulation state isolated between ticks.

**6.2.6 -- Wire module into lib.rs**

Add `pub mod decision_cycle;` to `crates/roko-runtime/src/lib.rs`.

**Warning**: The `TickPipeline` is intentionally in `roko-runtime`, not `roko-conductor`. The conductor evaluates signal streams for intervention decisions. The tick pipeline is the core execution loop that produces signals. These are different responsibilities: the conductor reacts to pipeline outputs, not the other way around.

**Warning**: Steps 5-8 use placeholder logic in this batch. The full SIMULATE integration with `ChainHeartbeatExtension` and the EXECUTE integration with agent dispatch happen when the calling code (orchestrator or daemon loop) wires concrete implementations. The pipeline provides the skeleton and step ordering.

### Wiring

- `crates/roko-runtime/src/lib.rs`: add `pub mod decision_cycle;`
- `crates/roko-runtime/src/event_bus.rs`: add `DecisionCycle` variant to `RokoEvent`
- `crates/roko-runtime/src/decision_cycle.rs`: new file

### Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::heartbeat::*;
    use crate::heartbeat_probes::EngineState;

    #[test]
    fn test_prediction_error_aggregate() {
        let sources = PredictionErrorSources {
            price_divergence: 1.0,
            regime_change: 1.0,
            position_health_delta: 1.0,
            probe_anomalies: 1.0,
        };
        // 0.30 + 0.40 + 0.20 + 0.10 = 1.0, capped at 1.0
        assert!((sources.aggregate() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_prediction_error_weights_correct() {
        let sources = PredictionErrorSources {
            price_divergence: 0.5,
            regime_change: 0.0,
            position_health_delta: 0.0,
            probe_anomalies: 0.0,
        };
        assert!((sources.aggregate() - 0.15).abs() < 0.01); // 0.5 * 0.30
    }

    #[test]
    fn test_low_pe_gates_to_t0() {
        // Build a pipeline with default gating (threshold 0.3).
        // Provide an EngineState with calm conditions.
        // Assert tick() returns tier == T0.
    }

    #[test]
    fn test_high_pe_gates_to_t2() {
        // Build a pipeline with default gating.
        // Inject probes that all return anomalous values.
        // Assert tick() returns tier == T2.
    }

    #[test]
    fn test_t0_tick_skips_simulate_execute() {
        // Run a T0 tick. Assert step_timings.simulate_ms is None.
        // Assert step_timings.execute_ms is None.
        // Assert acted == false.
    }

    #[test]
    fn test_adaptive_threshold_increases_under_low_arousal() {
        // Set arousal to 0.0 (calm). Assert threshold is higher than base.
        // High threshold means more ticks go T0 (cost savings).
    }

    #[test]
    fn test_decision_cycle_record_serializes() {
        // Create a DecisionCycleRecord. Serialize to JSON. Deserialize back.
        // Assert round-trip equality.
    }
}
```

### Verification

```bash
cargo test -p roko-runtime -- decision_cycle
cargo test -p roko-runtime -- heartbeat
cargo clippy -p roko-runtime --no-deps -- -D warnings
cargo +nightly fmt --check -p roko-runtime
```

### Acceptance Criteria

- [ ] `DecisionCycleRecord` captures all 9 steps with per-step timing
- [ ] `PredictionErrorSources::aggregate()` produces weighted, capped [0.0, 1.0] values
- [ ] `TickPipeline::tick()` runs all 9 steps in order
- [ ] T0 ticks skip steps 5-8 (simulate/validate/execute/verify)
- [ ] T1/T2 ticks run steps 5-8
- [ ] Tier gating uses adaptive threshold modulated by arousal and resource health
- [ ] `RokoEvent::DecisionCycle` variant added to event bus
- [ ] Step 9 (REFLECT) updates `CorticalState::prediction_accuracy`
- [ ] All tests pass, clippy clean, fmt clean

### Commit Message

```
feat(roko-runtime): add 9-step decision pipeline with tier gating
```

---

## Batch 6.3: Regime detection and adaptive threshold

> **Effort**: L | **Depends on**: 3.1 (TA indicators for prediction error) | **Crate**: roko-runtime
> **Branch**: `defi/batch-6.3-regime-threshold`

### Context

The `Regime` enum exists in `roko-runtime/src/heartbeat.rs:58`: Calm, Normal, Volatile, Crisis. The `CorticalState` stores it as an `AtomicU8` at line 243, with `set_regime` and `regime()` accessors. The `HeartbeatPolicy` adjusts tick intervals based on regime via `compute_theta_interval` (line 923) and `adaptive_interval` (line 797).

What does not exist: the state machine that transitions between regimes based on observed conditions. The PRD specifies explicit transition rules:

- Calm -> Normal: prediction error crosses threshold for 2 consecutive ticks
- Normal -> Volatile: 3+ probe anomalies or regime change detected
- Volatile -> Crisis: position health below critical or circuit breaker warning
- Crisis -> Volatile: no critical anomalies for 10 consecutive ticks
- Volatile -> Normal: prediction error below threshold for 20 ticks
- Normal -> Calm: prediction error below 0.1 for 50 ticks

This batch builds the regime transition state machine and the full adaptive threshold computation. The adaptive threshold from the PRD is: `threshold = base * confidence_factor * mortality_factor * arousal_factor`, where confidence raises the threshold (coast when confident), mortality lowers it (attend when resources low), and arousal lowers it (attend when excited).

The TA indicators from batch 3.1 feed the price divergence component of prediction error. Without them, only probe anomalies and position health drive tier gating. This batch adds the price-divergence and regime-change signals to the `PredictionErrorSources` computation built in 6.2.

### Read First

| File | Why |
|------|-----|
| `crates/roko-runtime/src/heartbeat.rs:58-91` | `Regime` enum and conversions |
| `crates/roko-runtime/src/heartbeat.rs:226-365` | `CorticalState` -- where regime is stored |
| `crates/roko-runtime/src/decision_cycle.rs` | From batch 6.2 -- `PredictionErrorSources`, `TickPipeline::adaptive_threshold` |
| `crates/roko-runtime/src/heartbeat_probes.rs:24-48` | `RollingStats` -- windowed anomaly detection |
| `crates/roko-runtime/src/heartbeat.rs:914-934` | `compute_gamma_interval`, `compute_theta_interval` -- interval adjustment |

### Conventions

- **Module files**: one file per type, flat in `src/`, declared in `lib.rs` as `pub mod xxx;`
- **Errors**: use `thiserror` for domain errors, convert to `RokoError` at boundaries
- **Tests**: inline `#[cfg(test)] mod tests {}`, min 3 tests per new type
- **Lints**: `cargo clippy --no-deps -- -D warnings` must pass clean
- **Docs**: `///` on all public items, `//!` module-level doc in new files
- **Derives**: `#[derive(Debug, Clone)]` minimum; add `Serialize, Deserialize` if persisted
- **Mirage testing**: integration tests that need chain state should use ephemeral mirage-rs instances. Pattern: `spawn_mirage_test_instance(Some(rpc_url), Some(fork_block)).await?` from `apps/mirage-rs/src/integration.rs`. Connect via `MirageClient::new(instance.config()).await?`. Shut down with `instance.shutdown().await?`. Add `mirage-rs` as a dev-dependency feature gate.

### Work Items

**6.3.1 -- Build `RegimeStateMachine`**

Create `crates/roko-runtime/src/regime.rs`:

```rust
//! Regime transition state machine with hysteresis counters.

use serde::{Deserialize, Serialize};

use crate::heartbeat::Regime;

/// Configuration for regime transition thresholds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegimeTransitionConfig {
    /// Consecutive ticks above threshold to escalate Calm -> Normal.
    pub calm_to_normal_ticks: u32,
    /// Probe anomaly count to escalate Normal -> Volatile.
    pub normal_to_volatile_anomalies: u32,
    /// Health factor below which to escalate to Crisis.
    pub crisis_health_factor: f32,
    /// Consecutive clean ticks to de-escalate Crisis -> Volatile.
    pub crisis_to_volatile_clean_ticks: u32,
    /// Consecutive low-PE ticks to de-escalate Volatile -> Normal.
    pub volatile_to_normal_ticks: u32,
    /// PE threshold for Calm classification.
    pub calm_pe_threshold: f32,
    /// Consecutive sub-calm ticks to de-escalate Normal -> Calm.
    pub normal_to_calm_ticks: u32,
}

impl Default for RegimeTransitionConfig {
    fn default() -> Self {
        Self {
            calm_to_normal_ticks: 2,
            normal_to_volatile_anomalies: 3,
            crisis_health_factor: 1.1,
            crisis_to_volatile_clean_ticks: 10,
            volatile_to_normal_ticks: 20,
            calm_pe_threshold: 0.1,
            normal_to_calm_ticks: 50,
        }
    }
}

/// Tracks regime state with hysteresis counters to prevent flapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegimeStateMachine {
    current: Regime,
    config: RegimeTransitionConfig,
    /// Counter for consecutive ticks matching the escalation/de-escalation condition.
    consecutive_count: u32,
}

impl RegimeStateMachine {
    pub fn new(config: RegimeTransitionConfig) -> Self {
        Self {
            current: Regime::Calm,
            config,
            consecutive_count: 0,
        }
    }

    /// Feed a tick observation and return the (possibly changed) regime.
    pub fn update(
        &mut self,
        prediction_error: f32,
        anomaly_count: u32,
        min_health_factor: f32,
    ) -> Regime {
        let prev = self.current;
        let next = self.compute_transition(prediction_error, anomaly_count, min_health_factor);
        if next != prev {
            self.consecutive_count = 0;
        }
        self.current = next;
        next
    }

    /// Current regime.
    pub const fn current(&self) -> Regime {
        self.current
    }

    fn compute_transition(
        &mut self,
        pe: f32,
        anomalies: u32,
        health: f32,
    ) -> Regime {
        match self.current {
            Regime::Calm => {
                if pe >= self.config.calm_pe_threshold {
                    self.consecutive_count += 1;
                    if self.consecutive_count >= self.config.calm_to_normal_ticks {
                        return Regime::Normal;
                    }
                } else {
                    self.consecutive_count = 0;
                }
                Regime::Calm
            }
            Regime::Normal => {
                if anomalies >= self.config.normal_to_volatile_anomalies {
                    return Regime::Volatile;
                }
                if pe < self.config.calm_pe_threshold {
                    self.consecutive_count += 1;
                    if self.consecutive_count >= self.config.normal_to_calm_ticks {
                        return Regime::Calm;
                    }
                } else {
                    self.consecutive_count = 0;
                }
                Regime::Normal
            }
            Regime::Volatile => {
                if health < self.config.crisis_health_factor {
                    return Regime::Crisis;
                }
                if pe < self.config.calm_pe_threshold && anomalies == 0 {
                    self.consecutive_count += 1;
                    if self.consecutive_count >= self.config.volatile_to_normal_ticks {
                        return Regime::Normal;
                    }
                } else {
                    self.consecutive_count = 0;
                }
                Regime::Volatile
            }
            Regime::Crisis => {
                if anomalies == 0 && health >= self.config.crisis_health_factor {
                    self.consecutive_count += 1;
                    if self.consecutive_count >= self.config.crisis_to_volatile_clean_ticks {
                        return Regime::Volatile;
                    }
                } else {
                    self.consecutive_count = 0;
                }
                Regime::Crisis
            }
        }
    }
}
```

**6.3.2 -- Enrich `PredictionErrorSources` with price divergence**

Update the `TickPipeline::analyze` method from batch 6.2 to populate the `price_divergence` and `regime_change` fields. Price divergence comes from the `EngineState`'s tracked assets -- the maximum absolute deviation between `current_price` and `last_tick_price` across all tracked assets, normalized to [0.0, 1.0]. Regime change fires when the `RegimeStateMachine` transitions to a different regime on the current tick.

**6.3.3 -- Integrate `RegimeStateMachine` into `TickPipeline`**

Add a `RegimeStateMachine` field to `TickPipeline`. After step 3 (ANALYZE) computes prediction error:

1. Feed the PE, anomaly count, and minimum health factor to `RegimeStateMachine::update`
2. If regime changed, update `CorticalState::set_regime` and `HeartbeatPolicy::set_regime`
3. Set `PredictionErrorSources::regime_change` to 1.0 if regime changed, 0.0 otherwise

**6.3.4 -- Expand adaptive threshold to full PRD formula**

Replace the simplified `adaptive_threshold` in `TickPipeline` (batch 6.2) with:

```rust
fn adaptive_threshold(&self, snapshot: &CorticalSnapshot) -> f32 {
    let base = self.gating.t0_t1_threshold;
    let confidence_factor = 1.0 + snapshot.prediction_accuracy * 0.5;
    let resource_factor = 1.0 - (1.0 - snapshot.resource_health) * 0.3;
    let arousal_factor = 1.0 - snapshot.pad.arousal as f32 * 0.2;
    base * confidence_factor * resource_factor * arousal_factor
}
```

High confidence raises the threshold (coast). Low resources lower it (attend). High arousal lowers it (attend).

**6.3.5 -- Wire module into lib.rs**

Add `pub mod regime;` to `crates/roko-runtime/src/lib.rs`.

**Warning**: The regime state machine uses hysteresis counters, not raw threshold comparison. Without the counters, small oscillations around the threshold cause rapid regime flapping, which destabilizes interval computation and budget allocation. The counter values in the default config (2, 10, 20, 50 ticks) are calibrated for DeFi cadence -- at 2s theta intervals, 20 ticks = 40s of stability required to de-escalate from Volatile to Normal.

### Wiring

- `crates/roko-runtime/src/lib.rs`: add `pub mod regime;`
- `crates/roko-runtime/src/regime.rs`: new file
- `crates/roko-runtime/src/decision_cycle.rs`: integrate RegimeStateMachine, expand adaptive threshold

### Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calm_to_normal_requires_consecutive() {
        let mut sm = RegimeStateMachine::new(RegimeTransitionConfig::default());
        // Feed PE=0.5 once. Assert still Calm (needs 2 consecutive).
        sm.update(0.5, 0, 2.0);
        assert_eq!(sm.current(), Regime::Calm);
        // Feed PE=0.5 again. Assert transitions to Normal.
        sm.update(0.5, 0, 2.0);
        assert_eq!(sm.current(), Regime::Normal);
    }

    #[test]
    fn test_normal_to_volatile_on_anomalies() {
        let mut sm = RegimeStateMachine::new(RegimeTransitionConfig::default());
        // Move to Normal first.
        sm.update(0.5, 0, 2.0);
        sm.update(0.5, 0, 2.0);
        assert_eq!(sm.current(), Regime::Normal);
        // Feed 3 anomalies. Assert transitions to Volatile.
        sm.update(0.5, 3, 2.0);
        assert_eq!(sm.current(), Regime::Volatile);
    }

    #[test]
    fn test_crisis_deescalation_requires_10_clean_ticks() {
        let mut sm = RegimeStateMachine::new(RegimeTransitionConfig::default());
        // Force into Crisis state.
        sm.current = Regime::Crisis;
        // Feed 9 clean ticks. Assert still Crisis.
        for _ in 0..9 {
            sm.update(0.0, 0, 2.0);
        }
        assert_eq!(sm.current(), Regime::Crisis);
        // 10th clean tick. Assert transitions to Volatile.
        sm.update(0.0, 0, 2.0);
        assert_eq!(sm.current(), Regime::Volatile);
    }

    #[test]
    fn test_counter_resets_on_condition_break() {
        let mut sm = RegimeStateMachine::new(RegimeTransitionConfig::default());
        // Start escalating Calm -> Normal (need 2 consecutive).
        sm.update(0.5, 0, 2.0); // count = 1
        sm.update(0.01, 0, 2.0); // PE drops, count resets
        sm.update(0.5, 0, 2.0); // count = 1 again
        assert_eq!(sm.current(), Regime::Calm); // Still needs one more
    }

    #[test]
    fn test_adaptive_threshold_increases_with_confidence() {
        // High prediction_accuracy (high confidence) -> threshold rises.
        // More ticks route to T0 -> cost savings.
    }
}
```

### Verification

```bash
cargo test -p roko-runtime -- regime
cargo test -p roko-runtime -- decision_cycle
cargo clippy -p roko-runtime --no-deps -- -D warnings
cargo +nightly fmt --check -p roko-runtime
```

### Acceptance Criteria

- [ ] `RegimeStateMachine` transitions follow the PRD's state diagram
- [ ] Hysteresis counters prevent rapid regime flapping
- [ ] Counter resets when the qualifying condition breaks
- [ ] `PredictionErrorSources` includes price divergence from tracked assets
- [ ] Regime change signal fires when `RegimeStateMachine` transitions
- [ ] Adaptive threshold uses the full formula: base * confidence * resource * arousal
- [ ] Integration with `TickPipeline`: regime updates flow to `CorticalState`
- [ ] All tests pass, clippy clean, fmt clean

### Commit Message

```
feat(roko-runtime): add regime state machine and adaptive tier threshold
```

---

## Batch 6.4: DeFi conductor watchers

> **Effort**: L | **Depends on**: 6.1, 3.1 | **Crate**: roko-conductor
> **Branch**: `defi/batch-6.4-defi-watchers`

### Context

The `Conductor` in `roko-conductor/src/conductor.rs:59` runs 10 watchers. All 10 monitor code-task process health: ghost turns, review loops, compile failures, context window pressure, cost overruns, stuck patterns. None monitors DeFi market conditions.

The watchers implement `Policy` from `roko-core` (trait at `crates/roko-core/src/lib.rs`). Each watcher receives `&[Engram]` (the signal stream) and emits intervention signals. The conductor merges watcher outputs through an `InterventionPolicy` (line 63) and feeds them to the circuit breaker (line 65).

The existing watchers live in `crates/roko-conductor/src/watchers/` with one file per watcher. Each file exports a struct that implements `Policy`. The `Conductor::new()` method at line 97 registers all 10 default watchers in a `Vec<Box<dyn Policy>>`.

This batch adds 4 DeFi-specific watchers that produce `WatcherOutput` values with `Severity` levels. The watchers fit the existing conductor architecture -- they are additional `Box<dyn Policy>` entries in the watcher vec, producing signals that the existing `WorstSeverityPolicy` merges.

The watchers need chain data. Rather than depend on `roko-chain`, they consume `Engram` signals tagged with chain-domain kinds (price, gas, liquidity, health). The chain event consumer from batch 6.1 translates chain events into `Engram` signals on the bus, and the conductor reads them.

### Read First

| File | Why |
|------|-----|
| `crates/roko-conductor/src/conductor.rs:59-121` | `Conductor` struct and `new()` with default watchers |
| `crates/roko-conductor/src/watchers/mod.rs` | Watcher module declarations |
| `crates/roko-conductor/src/watchers/cost_overrun.rs` | Example watcher implementation pattern |
| `crates/roko-conductor/src/circuit_breaker.rs:1-60` | `CircuitBreaker`, `HoltForecaster` -- proactive tripping |
| `crates/roko-conductor/src/interventions.rs` (if exists) | `InterventionPolicy`, `WatcherOutput`, `Severity` |

### Conventions

- **Module files**: one file per type, flat in `src/`, declared in `lib.rs` as `pub mod xxx;`
- **Errors**: use `thiserror` for domain errors, convert to `RokoError` at boundaries
- **Tests**: inline `#[cfg(test)] mod tests {}`, min 3 tests per new type
- **Lints**: `cargo clippy --no-deps -- -D warnings` must pass clean
- **Docs**: `///` on all public items, `//!` module-level doc in new files
- **Derives**: `#[derive(Debug, Clone)]` minimum; add `Serialize, Deserialize` if persisted
- **Mirage testing**: integration tests that need chain state should use ephemeral mirage-rs instances. Pattern: `spawn_mirage_test_instance(Some(rpc_url), Some(fork_block)).await?` from `apps/mirage-rs/src/integration.rs`. Connect via `MirageClient::new(instance.config()).await?`. Shut down with `instance.shutdown().await?`. Add `mirage-rs` as a dev-dependency feature gate.

### Work Items

**6.4.1 -- Build `PriceDeviationWatcher`**

Create `crates/roko-conductor/src/watchers/price_deviation.rs`:

```rust
//! Watcher for price deviation beyond band vs EMA.
//!
//! Scans the signal stream for price-update engrams and fires when the
//! deviation from the rolling EMA exceeds configured sigma thresholds.

use roko_core::{Body, Context, Engram, Kind, Policy};

/// Price deviation watcher configuration.
#[derive(Debug, Clone)]
pub struct PriceDeviationWatcher {
    /// Warning threshold in standard deviations.
    warning_sigma: f32,
    /// Critical threshold in standard deviations.
    critical_sigma: f32,
}

impl Default for PriceDeviationWatcher {
    fn default() -> Self {
        Self {
            warning_sigma: 2.0,
            critical_sigma: 3.0,
        }
    }
}

impl Policy for PriceDeviationWatcher {
    fn decide(&self, signals: &[Engram], _ctx: &Context) -> Vec<Engram> {
        // Scan signals for kind "price_update".
        // Compute deviation from rolling mean.
        // Emit Warning at 2sigma, Critical at 3sigma.
        vec![]
    }
}
```

**6.4.2 -- Build `GasAnomalyWatcher`**

Create `crates/roko-conductor/src/watchers/gas_anomaly.rs`:

```rust
//! Watcher for gas price spikes above median.

use roko_core::{Body, Context, Engram, Kind, Policy};

/// Gas anomaly watcher configuration.
#[derive(Debug, Clone)]
pub struct GasAnomalyWatcher {
    /// Warning threshold as a multiple of median gas price.
    warning_multiple: f32,
    /// Critical threshold as a multiple of median gas price.
    critical_multiple: f32,
}

impl Default for GasAnomalyWatcher {
    fn default() -> Self {
        Self {
            warning_multiple: 3.0,
            critical_multiple: 10.0,
        }
    }
}

impl Policy for GasAnomalyWatcher {
    fn decide(&self, signals: &[Engram], _ctx: &Context) -> Vec<Engram> {
        // Scan signals for kind "gas_update".
        // Compare to rolling median.
        // Emit Warning at 3x, Critical at 10x.
        vec![]
    }
}
```

**6.4.3 -- Build `PositionHealthWatcher`**

Create `crates/roko-conductor/src/watchers/position_health.rs`:

```rust
//! Watcher for position health factor approaching liquidation.

use roko_core::{Body, Context, Engram, Kind, Policy};

/// Position health watcher configuration.
#[derive(Debug, Clone)]
pub struct PositionHealthWatcher {
    /// Warning health factor threshold.
    warning_threshold: f32,
    /// Critical health factor threshold.
    critical_threshold: f32,
}

impl Default for PositionHealthWatcher {
    fn default() -> Self {
        Self {
            warning_threshold: 1.5,
            critical_threshold: 1.1,
        }
    }
}

impl Policy for PositionHealthWatcher {
    fn decide(&self, signals: &[Engram], _ctx: &Context) -> Vec<Engram> {
        // Scan signals for kind "position_health".
        // Emit Warning at health_factor < 1.5, Critical at < 1.1.
        vec![]
    }
}
```

**6.4.4 -- Build `LiquidityDropWatcher`**

Create `crates/roko-conductor/src/watchers/liquidity_drop.rs`:

```rust
//! Watcher for TVL decline on pools with open positions.

use roko_core::{Body, Context, Engram, Kind, Policy};

/// Liquidity drop watcher configuration.
#[derive(Debug, Clone)]
pub struct LiquidityDropWatcher {
    /// Warning threshold as a percentage decline.
    warning_pct: f32,
    /// Critical threshold as a percentage decline.
    critical_pct: f32,
}

impl Default for LiquidityDropWatcher {
    fn default() -> Self {
        Self {
            warning_pct: 10.0,
            critical_pct: 30.0,
        }
    }
}

impl Policy for LiquidityDropWatcher {
    fn decide(&self, signals: &[Engram], _ctx: &Context) -> Vec<Engram> {
        // Scan signals for kind "tvl_update".
        // Compare to previous reading.
        // Emit Warning at -10%, Critical at -30%.
        vec![]
    }
}
```

**6.4.5 -- Wire watchers into conductor**

Update `crates/roko-conductor/src/watchers/mod.rs` to declare the four new modules and re-export their types:

```rust
pub mod gas_anomaly;
pub mod liquidity_drop;
pub mod position_health;
pub mod price_deviation;

pub use gas_anomaly::GasAnomalyWatcher;
pub use liquidity_drop::LiquidityDropWatcher;
pub use position_health::PositionHealthWatcher;
pub use price_deviation::PriceDeviationWatcher;
```

**6.4.6 -- Add DeFi watchers to Conductor**

Add a `with_defi_watchers` builder method to `Conductor` in `conductor.rs`:

```rust
impl Conductor {
    /// Create a conductor with the default code-task watchers plus DeFi watchers.
    pub fn with_defi_watchers(mut self) -> Self {
        self.watchers.push(Box::new(PriceDeviationWatcher::default()));
        self.watchers.push(Box::new(GasAnomalyWatcher::default()));
        self.watchers.push(Box::new(PositionHealthWatcher::default()));
        self.watchers.push(Box::new(LiquidityDropWatcher::default()));
        self
    }
}
```

Do not change `Conductor::new()`. DeFi watchers are opt-in via the builder. Code-task users get the existing 10 watchers; DeFi users call `.with_defi_watchers()` for 14 total.

**Warning**: The four DeFi watchers consume engram signals tagged with specific `Kind` values (price_update, gas_update, position_health, tvl_update). These signal kinds must be emitted by the chain event consumer (batch 6.1) or by the heartbeat probes. If no signals of the right kind appear in the stream, the watchers produce no output -- they do not crash.

### Wiring

- `crates/roko-conductor/src/watchers/mod.rs`: add four module declarations and re-exports
- `crates/roko-conductor/src/watchers/price_deviation.rs`: new file
- `crates/roko-conductor/src/watchers/gas_anomaly.rs`: new file
- `crates/roko-conductor/src/watchers/position_health.rs`: new file
- `crates/roko-conductor/src/watchers/liquidity_drop.rs`: new file
- `crates/roko-conductor/src/conductor.rs`: add `with_defi_watchers` builder method

### Tests

```rust
// price_deviation.rs
#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::Policy;

    #[test]
    fn test_no_price_signals_no_output() {
        let w = PriceDeviationWatcher::default();
        let result = w.decide(&[], &Context::now());
        assert!(result.is_empty());
    }

    #[test]
    fn test_large_deviation_emits_critical() {
        // Build engrams with price_update kind showing 4-sigma deviation.
        // Assert the watcher emits a critical-severity intervention.
    }

    #[test]
    fn test_moderate_deviation_emits_warning() {
        // Build engrams with 2.5-sigma deviation.
        // Assert warning severity.
    }
}

// gas_anomaly.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gas_10x_median_emits_critical() {
        // Build gas_update engrams with 10x spike.
        // Assert critical severity.
    }
}

// position_health.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_below_1_1_emits_critical() {
        // Build position_health engram with factor 1.05.
        // Assert critical severity.
    }

    #[test]
    fn test_healthy_position_no_output() {
        // Build position_health engram with factor 2.0.
        // Assert no output.
    }
}

// liquidity_drop.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_30pct_tvl_drop_emits_critical() {
        // Build tvl_update engrams showing -35% decline.
        // Assert critical severity.
    }
}

// conductor.rs
#[cfg(test)]
mod tests {
    #[test]
    fn test_with_defi_watchers_adds_four() {
        let c = Conductor::new();
        assert_eq!(c.watcher_count(), 10);
        let c = c.with_defi_watchers();
        assert_eq!(c.watcher_count(), 14);
    }
}
```

### Verification

```bash
cargo test -p roko-conductor -- watchers
cargo test -p roko-conductor -- conductor
cargo clippy -p roko-conductor --no-deps -- -D warnings
cargo +nightly fmt --check -p roko-conductor
```

### Acceptance Criteria

- [ ] `PriceDeviationWatcher` emits Warning at 2-sigma, Critical at 3-sigma
- [ ] `GasAnomalyWatcher` emits Warning at 3x median, Critical at 10x
- [ ] `PositionHealthWatcher` emits Warning at factor < 1.5, Critical at < 1.1
- [ ] `LiquidityDropWatcher` emits Warning at -10%, Critical at -30%
- [ ] All four implement `Policy` and can be added to the conductor watcher vec
- [ ] `Conductor::with_defi_watchers()` adds 4 watchers to the existing 10
- [ ] Watchers produce no output when no matching signal kinds are in the stream
- [ ] `Conductor::new()` is unchanged -- DeFi watchers are opt-in
- [ ] All tests pass, clippy clean, fmt clean

### Commit Message

```
feat(roko-conductor): add DeFi watchers for price, gas, health, and liquidity
```

## Product Layer

> Maps this gap doc's capabilities to the 12 universal primitives defined in `docs/prd/23-universal-primitives.md`.

### Primitives Used

- **Feed**: `HeartbeatTickFeed` (periodic clock driving agent decision cycles -- configurable interval: 1s, 5s, 1m, etc.), `ChainEventFeed` (chain events merged into tick pipeline)
- **Recipe**: `CorticalState` aggregation pipeline (merges tick + chain events + indicator values into a single decision-cycle input)
- **Gate**: `EmergencyShutdownPolicy` (pre-action circuit breaker triggered by abnormal tick patterns or missed heartbeats)
- **Signal**: Tick signals (heartbeat events on PulseBus), emergency shutdown signals
- **Knowledge Entry**: `DecisionCycleRecord` (audit trail of each tick's inputs, decision, and outcome)

### Authoring Surfaces

- **Agent Composer Stage 8** -- heartbeat configuration: tick interval, event sources, decision pipeline
- **Feed Designer** -- configure heartbeat tick feed with interval, jitter, and backpressure settings
- **Recipe Editor** -- build CorticalState aggregation pipeline from tick + event sources

### Shareable Artifacts

- Heartbeat configurations (tick interval + event source presets for different trading frequencies)
- CorticalState pipeline templates (standard aggregation patterns for scalping, swing, position trading)

### Dashboard Visibility

- **Pulse > Heartbeat** -- live tick visualization, decision cycle timeline
- **Agent Detail > Heartbeat** -- per-agent tick health, missed beats, latency
- **Measurements > Decision Cycles** -- historical decision cycle records with outcome attribution
