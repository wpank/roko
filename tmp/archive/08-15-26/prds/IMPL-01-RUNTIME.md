# IMPL-01: Agent runtime extraction -- granular implementation plan

**Parent PRD:** PRD-02-AGENT-RUNTIME.md
**Date:** 2026-04-21
**Estimated tasks:** 30
**Estimated LOC delta:** +8,000 new, -0 removed (Phase 4 removes orchestrate.rs code later)

---

## What this document is

A step-by-step implementation plan for extracting Roko's agent runtime from a 19K-line monolith (`orchestrate.rs`) into a composable `Extension`-based architecture. Each task includes exact file paths, code patterns to follow, prompts for implementing agents, and concrete acceptance criteria.

You do not need prior knowledge of this codebase. Every task is self-contained.

---

## Project context

**Workspace root:** `/Users/will/dev/nunchi/roko/roko/`

Roko is an 18-crate Rust workspace (~177K LOC) for building agents that build themselves. The core loop reads PRDs, generates implementation plans, dispatches tasks to LLM-backed agents, validates results through gate pipelines, and persists outcomes as signals.

The agent runtime currently lives as a monolith:

| File | Lines | Methods | Fields |
|------|-------|---------|--------|
| `crates/roko-cli/src/orchestrate.rs` | 20,678 | 217 | 137+ on `PlanRunner` |

This plan extracts foundational types into `roko-runtime`, creates extension crates, and migrates `PlanRunner` to use them.

**Key existing files (read these before any task):**

| File | What it contains |
|------|-----------------|
| `crates/roko-runtime/src/lib.rs` | Current runtime crate root. Exports `lifecycle`, `event_bus`, `heartbeat`, `process`, `cancel`, `energy`, `metrics`. |
| `crates/roko-runtime/src/heartbeat.rs` | `HeartbeatSpeed` (Gamma/Theta/Delta), `CorticalState` (~21 atomic fields), `CorticalSnapshot`, `ClockConfig`, `HeartbeatPolicy`, `PersonalityPreset`, `BehavioralState`, `PlutchikLabel`, `Regime`. |
| `crates/roko-runtime/src/event_bus.rs` | `EventBus<E>`, `BusSender<E>`, `Envelope<E>`, `RokoEvent` enum, `global_event_bus()`. |
| `crates/roko-runtime/src/lifecycle.rs` | `AgentLifecycleState`, type-state `Agent<S>` with 8 phases (Unvalidated -> Ready), `LifecycleTransition`, `LifecycleHooks`, health probes. |
| `crates/roko-runtime/src/process.rs` | `ProcessSupervisor`, `ProcessHandle`, `SpawnConfig`, `SupervisionStrategy` (Erlang-style). |
| `crates/roko-runtime/src/energy.rs` | `EnergyPool`, `CognitiveMetabolism`, `OperationKind`, energy ledger. |
| `crates/roko-primitives/src/tier.rs` | `InferenceTier` (T0/T1/T2), `TierRouter::select_model()`. |
| `crates/roko-primitives/src/hdc.rs` | HDC hypervector operations. |
| `crates/roko-daimon/src/lib.rs` | Affect engine: PAD state, somatic markers, retrieval weights, behavioral modulation. |
| `crates/roko-dreams/src/lib.rs` | Dream cycle, hypnagogia, imagination, replay, threat rehearsal. |
| `crates/roko-conductor/src/lib.rs` | 10 watchers, circuit breaker, Yerkes-Dodson, federation, self-healing. |
| `crates/roko-learn/src/lib.rs` | Episode logger, playbooks, skill library, anomaly detection, latency tracking. |
| `crates/roko-neuro/src/lib.rs` | Knowledge store, tier progression, distillation. |
| `crates/roko-cli/src/orchestrate.rs` | `PlanRunner` (137+ fields). The monolith being decomposed. |

**Build commands:**

```bash
cd /Users/will/dev/nunchi/roko/roko
rustup update stable          # Need 1.91+
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --no-deps -- -D warnings
cargo +nightly fmt --all
```

---

## Phase 1: Foundation types (roko-runtime rewrite)

These tasks add new types to the existing `roko-runtime` crate. They do not remove or modify existing exports. Every existing import path continues to work.

---

### Task 1: Define `CognitiveTier` enum

- [ ] Create `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/cognitive.rs`

**Files to create:**
- `crates/roko-runtime/src/cognitive.rs`

**Files to modify:**
- `crates/roko-runtime/src/lib.rs` (add `pub mod cognitive;` and re-exports)

**What to do:**

1. Read `crates/roko-primitives/src/tier.rs` first. Note that `InferenceTier` already defines T0/T1/T2. `CognitiveTier` wraps this with domain-aware metadata.

2. Define in `cognitive.rs`:

```rust
use roko_primitives::tier::InferenceTier;
use serde::{Deserialize, Serialize};

/// Cognitive tier decision for a single heartbeat tick.
///
/// Wraps `InferenceTier` with the reasoning that produced the selection
/// and the prediction error that triggered it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CognitiveTier {
    /// The inference tier selected for this tick.
    pub tier: InferenceTier,
    /// Scalar prediction error that drove tier selection.
    pub prediction_error: f64,
    /// Per-extension PE components that contributed to the aggregate.
    pub pe_components: Vec<PredictionErrorComponent>,
    /// Why this tier was selected (threshold comparison result).
    pub reason: TierSelectionReason,
}

/// A single extension's contribution to aggregate prediction error.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PredictionErrorComponent {
    /// Extension name that produced this component.
    pub source: String,
    /// Raw prediction error value from this extension.
    pub value: f64,
    /// Weight applied to this component in aggregation.
    pub weight: f64,
}

/// Why a particular tier was selected.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TierSelectionReason {
    /// PE below T1 threshold. No LLM call needed.
    BelowThreshold { threshold: f64 },
    /// PE above T1 threshold but below T2 threshold.
    ModerateNovelty { t1_threshold: f64, t2_threshold: f64 },
    /// PE above T2 threshold. Full reasoning needed.
    HighNovelty { threshold: f64 },
    /// Budget constraints forced a lower tier.
    BudgetConstrained { requested: InferenceTier, budget_remaining_pct: f64 },
    /// Manual override from operator or conductor signal.
    Override { signal: String },
}
```

3. In `lib.rs`, add `pub mod cognitive;` after the existing module declarations. Add re-exports:

```rust
pub use cognitive::{CognitiveTier, PredictionErrorComponent, TierSelectionReason};
```

**Context/Prompt for implementing agent:**

> Read `crates/roko-primitives/src/tier.rs` to understand `InferenceTier` (T0/T1/T2). Read `crates/roko-runtime/src/heartbeat.rs` lines 228-280 to see how `CorticalState` already stores prediction accuracy and behavioral state. Create `crates/roko-runtime/src/cognitive.rs` with `CognitiveTier`, `PredictionErrorComponent`, and `TierSelectionReason`. Follow the serde patterns in `heartbeat.rs` (derive Serialize/Deserialize, use `#[serde(rename_all = "snake_case")]` on enums with data). Wire into `lib.rs` with `pub mod cognitive;` and re-exports.

**Acceptance:**
- `cargo build -p roko-runtime` succeeds
- `cargo test -p roko-runtime` passes (existing tests unchanged)
- `use roko_runtime::CognitiveTier;` resolves from another crate

---

### Task 2: Define `ExtensionLayer` enum

- [ ] Add to `crates/roko-runtime/src/cognitive.rs`

**Files to modify:**
- `crates/roko-runtime/src/cognitive.rs` (append)

**What to do:**

1. Read `crates/roko-runtime/src/lifecycle.rs` to see how enums are patterned (derive blocks, serde tags).

2. Append to `cognitive.rs`:

```rust
/// Layer ordering for extensions. Lower layers fire first.
///
/// Extensions declare their layer at registration time. The chain fires
/// hooks in layer order (Foundation first, Recovery last). Within a layer,
/// extensions fire in registration order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum ExtensionLayer {
    /// Core runtime services: heartbeat, energy, lifecycle. Always present.
    Foundation = 0,
    /// Knowledge and memory: neuro store, playbooks, context caches.
    Knowledge = 1,
    /// Affect and behavioral modulation: daimon, somatic markers.
    Affect = 2,
    /// Perception: chain subscriber, file watcher, event sources.
    Perception = 3,
    /// Analysis: conductor watchers, anomaly detection, stuck patterns.
    Analysis = 4,
    /// Action: tool dispatch, git operations, transactions.
    Action = 5,
    /// Learning: episode logger, skill extraction, threshold adaptation.
    Learning = 6,
    /// Recovery: dream cycles, consolidation, self-healing.
    Recovery = 7,
}

impl ExtensionLayer {
    /// Human-readable name for logging.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Foundation => "foundation",
            Self::Knowledge => "knowledge",
            Self::Affect => "affect",
            Self::Perception => "perception",
            Self::Analysis => "analysis",
            Self::Action => "action",
            Self::Learning => "learning",
            Self::Recovery => "recovery",
        }
    }
}
```

3. Add `ExtensionLayer` to the re-exports in `lib.rs`.

**Context/Prompt for implementing agent:**

> Open `crates/roko-runtime/src/cognitive.rs` (created in Task 1). Append `ExtensionLayer` as a `#[repr(u8)]` enum with 8 variants ordered Foundation(0) through Recovery(7). Derive `PartialOrd, Ord` so the chain can sort by layer. Add a `name()` method. Re-export from `lib.rs`.

**Acceptance:**
- `ExtensionLayer::Foundation < ExtensionLayer::Recovery` evaluates to `true`
- `serde_json::to_string(&ExtensionLayer::Perception)` produces `3`
- Add a unit test asserting layer ordering: `assert!(ExtensionLayer::Foundation < ExtensionLayer::Knowledge);` (repeat for all adjacent pairs)

---

### Task 3: Define `Extension` trait

- [ ] Create `crates/roko-runtime/src/extension.rs`

**Files to create:**
- `crates/roko-runtime/src/extension.rs`

**Files to modify:**
- `crates/roko-runtime/src/lib.rs` (add `pub mod extension;` and re-exports)

**What to do:**

1. Read `crates/roko-runtime/src/heartbeat.rs` lines 226-550 (`CorticalState` and its methods) to understand the shared perception surface that extensions read/write.

2. Read `crates/roko-runtime/src/cognitive.rs` to understand `CognitiveTier` and `ExtensionLayer`.

3. Read `crates/roko-runtime/src/cancel.rs` to understand `CancelToken`.

4. Define the trait:

```rust
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;

use crate::{
    cancel::CancelToken,
    cognitive::{CognitiveTier, ExtensionLayer, PredictionErrorComponent},
    heartbeat::CorticalState,
};

/// Metadata returned by an extension during registration.
#[derive(Debug, Clone)]
pub struct ExtensionDescriptor {
    /// Unique name (e.g., "heartbeat", "neuro", "chain-subscriber").
    pub name: String,
    /// Layer this extension belongs to.
    pub layer: ExtensionLayer,
    /// Names of extensions this one depends on. The chain validates that
    /// all dependencies are registered before this extension.
    pub depends_on: Vec<String>,
}

/// Observation produced during the OBSERVE step.
#[derive(Debug, Clone)]
pub struct Observation {
    /// Extension that produced this observation.
    pub source: String,
    /// Domain-specific observation payload serialized as JSON.
    pub payload: serde_json::Value,
    /// Salience score in [0.0, 1.0] for attention auction ranking.
    pub salience: f64,
}

/// Retrieval result produced during the RETRIEVE step.
#[derive(Debug, Clone)]
pub struct Retrieval {
    /// Extension that produced this retrieval.
    pub source: String,
    /// Retrieved content (text, structured data, etc.).
    pub content: String,
    /// Relevance score in [0.0, 1.0].
    pub relevance: f64,
    /// Token cost estimate for including this in the context window.
    pub token_estimate: usize,
}

/// Action proposed during SIMULATE, validated during VALIDATE, committed during EXECUTE.
#[derive(Debug, Clone)]
pub struct ProposedAction {
    /// Extension that proposed this action.
    pub source: String,
    /// Action type identifier (e.g., "tool_call", "git_commit", "tx_submit").
    pub action_type: String,
    /// Action payload serialized as JSON.
    pub payload: serde_json::Value,
    /// Confidence score in [0.0, 1.0].
    pub confidence: f64,
}

/// Outcome of an executed action.
#[derive(Debug, Clone)]
pub struct ActionOutcome {
    /// The action that was executed.
    pub action: ProposedAction,
    /// Whether the action succeeded.
    pub success: bool,
    /// Free-form result data.
    pub result: serde_json::Value,
    /// Wall-clock duration of the action.
    pub duration: std::time::Duration,
}

/// Verification result produced during the VERIFY step.
#[derive(Debug, Clone)]
pub struct Verification {
    /// Extension that produced this verification.
    pub source: String,
    /// Whether the verification passed.
    pub passed: bool,
    /// Details about what was checked.
    pub details: String,
}

/// Decision cycle record summarizing one heartbeat tick.
#[derive(Debug, Clone)]
pub struct DecisionCycleRecord {
    /// Monotonic tick number.
    pub tick: u64,
    /// Total wall-clock duration of this tick.
    pub duration: std::time::Duration,
    /// Tier selected for this tick.
    pub tier: CognitiveTier,
    /// Observations gathered.
    pub observations: Vec<Observation>,
    /// Retrievals gathered.
    pub retrievals: Vec<Retrieval>,
    /// Actions taken (empty for T0).
    pub actions: Vec<ActionOutcome>,
    /// Verifications performed (empty for T0).
    pub verifications: Vec<Verification>,
}

/// Extension trait: the plug-in interface for the heartbeat pipeline.
///
/// All methods have default no-op implementations. Extensions override
/// only the hooks they care about. The chain calls hooks in layer order
/// (Foundation first, Recovery last).
#[async_trait]
pub trait Extension: Send + Sync + 'static {
    /// Return metadata about this extension.
    fn descriptor(&self) -> ExtensionDescriptor;

    // ── Lifecycle hooks ───────────────────────────────────────────

    /// Called once when the extension is added to a running agent.
    /// Use for initialization that requires async (opening files, connecting).
    async fn on_start(&mut self, _cortical: &Arc<CorticalState>) -> Result<()> {
        Ok(())
    }

    /// Called once when the agent is shutting down.
    async fn on_stop(&mut self) -> Result<()> {
        Ok(())
    }

    /// Called when the agent transitions between lifecycle states.
    async fn on_lifecycle_transition(
        &mut self,
        _from: &crate::lifecycle::AgentLifecycleState,
        _to: &crate::lifecycle::AgentLifecycleState,
    ) -> Result<()> {
        Ok(())
    }

    // ── Heartbeat pipeline hooks (9-step) ─────────────────────────

    /// Step 1: OBSERVE. Read environment, return observations.
    async fn observe(
        &mut self,
        _cortical: &Arc<CorticalState>,
        _cancel: &CancelToken,
    ) -> Result<Vec<Observation>> {
        Ok(vec![])
    }

    /// Step 2: RETRIEVE. Query knowledge stores, return retrievals.
    async fn retrieve(
        &mut self,
        _observations: &[Observation],
        _cortical: &Arc<CorticalState>,
        _cancel: &CancelToken,
    ) -> Result<Vec<Retrieval>> {
        Ok(vec![])
    }

    /// Step 3: ANALYZE. Compute domain-specific prediction error.
    async fn analyze(
        &self,
        _observations: &[Observation],
        _retrievals: &[Retrieval],
        _cortical: &Arc<CorticalState>,
    ) -> Result<Option<PredictionErrorComponent>> {
        Ok(None)
    }

    /// Step 5: SIMULATE. Propose actions before committing.
    async fn simulate(
        &mut self,
        _observations: &[Observation],
        _retrievals: &[Retrieval],
        _tier: &CognitiveTier,
        _cancel: &CancelToken,
    ) -> Result<Vec<ProposedAction>> {
        Ok(vec![])
    }

    /// Step 6: VALIDATE. Check safety constraints on proposed actions.
    async fn validate(
        &self,
        _actions: &[ProposedAction],
        _cortical: &Arc<CorticalState>,
    ) -> Result<Vec<ProposedAction>> {
        Ok(vec![])
    }

    /// Step 7: EXECUTE. Perform validated actions.
    async fn execute(
        &mut self,
        _actions: &[ProposedAction],
        _tier: &CognitiveTier,
        _cancel: &CancelToken,
    ) -> Result<Vec<ActionOutcome>> {
        Ok(vec![])
    }

    /// Step 8: VERIFY. Confirm outcomes match expectations.
    async fn verify(
        &mut self,
        _outcomes: &[ActionOutcome],
        _cancel: &CancelToken,
    ) -> Result<Vec<Verification>> {
        Ok(vec![])
    }

    /// Step 9: REFLECT. Update internal state based on the completed cycle.
    async fn reflect(
        &mut self,
        _record: &DecisionCycleRecord,
        _cortical: &Arc<CorticalState>,
    ) -> Result<()> {
        Ok(())
    }

    // ── Frequency hooks ──────────────────────────────────────────

    /// Called on every gamma tick (fast perception cadence).
    async fn on_gamma_tick(&mut self, _tick_id: u64, _cortical: &Arc<CorticalState>) -> Result<()> {
        Ok(())
    }

    /// Called on every theta tick (reflective cadence).
    async fn on_theta_tick(&mut self, _tick_id: u64, _cortical: &Arc<CorticalState>) -> Result<()> {
        Ok(())
    }

    /// Called on every delta tick (consolidation cadence).
    async fn on_delta_tick(&mut self, _tick_id: u64, _cortical: &Arc<CorticalState>) -> Result<()> {
        Ok(())
    }

    // ── Event hooks ──────────────────────────────────────────────

    /// Called when a wakeup condition fires outside the normal cadence.
    async fn on_wakeup(
        &mut self,
        _condition: &crate::heartbeat::WakeupCondition,
        _cortical: &Arc<CorticalState>,
    ) -> Result<()> {
        Ok(())
    }

    /// Called when a cognitive signal is received.
    async fn on_cognitive_signal(
        &mut self,
        _signal: &crate::heartbeat::CognitiveSignal,
    ) -> Result<()> {
        Ok(())
    }

    // ── Budget hooks ─────────────────────────────────────────────

    /// Called when the energy budget changes significantly.
    async fn on_budget_change(&mut self, _remaining_pct: f64) -> Result<()> {
        Ok(())
    }

    /// Return the estimated cost of the next tick for this extension.
    fn estimated_tick_cost(&self) -> f64 {
        0.0
    }
}
```

5. Add `async-trait` to `roko-runtime`'s `Cargo.toml` dependencies if not already present.

6. Wire into `lib.rs`:

```rust
pub mod extension;
pub use extension::{
    ActionOutcome, DecisionCycleRecord, Extension, ExtensionDescriptor,
    Observation, ProposedAction, Retrieval, Verification,
};
```

**Context/Prompt for implementing agent:**

> Read `crates/roko-runtime/src/heartbeat.rs` (the `CorticalState` struct and its methods) and `crates/roko-runtime/src/cognitive.rs` (from Tasks 1-2). Create `crates/roko-runtime/src/extension.rs` with the `Extension` trait. The trait has 22 hook methods, all with default no-op implementations. Group them into lifecycle hooks (on_start, on_stop, on_lifecycle_transition), pipeline hooks (observe through reflect -- the 9-step heartbeat pipeline), frequency hooks (on_gamma/theta/delta_tick), event hooks (on_wakeup, on_cognitive_signal), and budget hooks (on_budget_change, estimated_tick_cost). Use `async_trait` for async methods. Define supporting types: `Observation`, `Retrieval`, `ProposedAction`, `ActionOutcome`, `Verification`, `DecisionCycleRecord`, `ExtensionDescriptor`. Wire into `lib.rs`.

**Acceptance:**
- `cargo build -p roko-runtime` succeeds
- A struct implementing `Extension` with only `fn descriptor()` compiles (all other methods default)
- `cargo doc -p roko-runtime --no-deps` generates docs for all 22 hook methods

---

### Task 4: Define `ExtensionChain`

- [ ] Create `crates/roko-runtime/src/chain.rs`

**Files to create:**
- `crates/roko-runtime/src/chain.rs`

**Files to modify:**
- `crates/roko-runtime/src/lib.rs` (add `pub mod chain;`)

**What to do:**

1. Read `crates/roko-runtime/src/extension.rs` (Task 3) for the `Extension` trait and `ExtensionDescriptor`.

2. Read `crates/roko-runtime/src/cognitive.rs` for `ExtensionLayer`.

3. Implement:

```rust
use std::sync::Arc;

use anyhow::{Result, bail};

use crate::{
    cancel::CancelToken,
    cognitive::{CognitiveTier, ExtensionLayer, PredictionErrorComponent},
    extension::*,
    heartbeat::CorticalState,
};

/// Ordered collection of extensions with dependency validation.
///
/// Extensions are sorted by layer (Foundation first) and within a layer
/// by registration order. The chain validates that all declared dependencies
/// are present before allowing the pipeline to run.
pub struct ExtensionChain {
    extensions: Vec<Box<dyn Extension>>,
    validated: bool,
}

impl ExtensionChain {
    /// Create an empty chain.
    pub fn new() -> Self {
        Self {
            extensions: Vec::new(),
            validated: false,
        }
    }

    /// Add an extension. Invalidates the chain (must call `validate()` again).
    pub fn add(&mut self, ext: impl Extension + 'static) {
        self.extensions.push(Box::new(ext));
        self.validated = false;
    }

    /// Sort extensions by layer, validate dependencies, mark as validated.
    ///
    /// Returns `Err` if any extension declares a dependency that is not
    /// registered in the chain.
    pub fn validate(&mut self) -> Result<()> {
        // Sort by layer, stable within layer (preserves registration order).
        self.extensions.sort_by_key(|ext| ext.descriptor().layer);

        // Build name set.
        let names: std::collections::HashSet<String> = self
            .extensions
            .iter()
            .map(|ext| ext.descriptor().name.clone())
            .collect();

        // Check all dependencies exist.
        for ext in &self.extensions {
            let desc = ext.descriptor();
            for dep in &desc.depends_on {
                if !names.contains(dep) {
                    bail!(
                        "extension '{}' depends on '{}', which is not registered",
                        desc.name,
                        dep,
                    );
                }
            }
        }

        // Check no duplicate names.
        if names.len() != self.extensions.len() {
            bail!("duplicate extension names detected");
        }

        self.validated = true;
        Ok(())
    }

    /// Number of registered extensions.
    pub fn len(&self) -> usize {
        self.extensions.len()
    }

    /// Whether the chain is empty.
    pub fn is_empty(&self) -> bool {
        self.extensions.is_empty()
    }

    /// List registered extension names in firing order.
    pub fn extension_names(&self) -> Vec<String> {
        self.extensions.iter().map(|e| e.descriptor().name.clone()).collect()
    }

    // ── Pipeline dispatch methods ────────────────────────────────

    /// Fire `on_start` on all extensions in layer order.
    pub async fn start_all(&mut self, cortical: &Arc<CorticalState>) -> Result<()> {
        self.ensure_validated()?;
        for ext in &mut self.extensions {
            ext.on_start(cortical).await?;
        }
        Ok(())
    }

    /// Fire `on_stop` on all extensions in reverse layer order.
    pub async fn stop_all(&mut self) -> Result<()> {
        for ext in self.extensions.iter_mut().rev() {
            ext.on_stop().await?;
        }
        Ok(())
    }

    /// Run OBSERVE step across all extensions. Returns flattened observations.
    pub async fn run_observe(
        &mut self,
        cortical: &Arc<CorticalState>,
        cancel: &CancelToken,
    ) -> Result<Vec<Observation>> {
        self.ensure_validated()?;
        let mut all = Vec::new();
        for ext in &mut self.extensions {
            let obs = ext.observe(cortical, cancel).await?;
            all.extend(obs);
        }
        Ok(all)
    }

    /// Run RETRIEVE step across all extensions.
    pub async fn run_retrieve(
        &mut self,
        observations: &[Observation],
        cortical: &Arc<CorticalState>,
        cancel: &CancelToken,
    ) -> Result<Vec<Retrieval>> {
        self.ensure_validated()?;
        let mut all = Vec::new();
        for ext in &mut self.extensions {
            let ret = ext.retrieve(observations, cortical, cancel).await?;
            all.extend(ret);
        }
        Ok(all)
    }

    /// Run ANALYZE step across all extensions. Returns PE components.
    pub async fn run_analyze(
        &mut self,
        observations: &[Observation],
        retrievals: &[Retrieval],
        cortical: &Arc<CorticalState>,
    ) -> Result<Vec<PredictionErrorComponent>> {
        self.ensure_validated()?;
        let mut components = Vec::new();
        for ext in &self.extensions {
            if let Some(pe) = ext.analyze(observations, retrievals, cortical).await? {
                components.push(pe);
            }
        }
        Ok(components)
    }

    /// Run SIMULATE step across all extensions.
    pub async fn run_simulate(
        &mut self,
        observations: &[Observation],
        retrievals: &[Retrieval],
        tier: &CognitiveTier,
        cancel: &CancelToken,
    ) -> Result<Vec<ProposedAction>> {
        self.ensure_validated()?;
        let mut all = Vec::new();
        for ext in &mut self.extensions {
            let actions = ext.simulate(observations, retrievals, tier, cancel).await?;
            all.extend(actions);
        }
        Ok(all)
    }

    /// Run VALIDATE step across all extensions.
    pub async fn run_validate(
        &mut self,
        actions: &[ProposedAction],
        cortical: &Arc<CorticalState>,
    ) -> Result<Vec<ProposedAction>> {
        self.ensure_validated()?;
        let mut validated = actions.to_vec();
        for ext in &self.extensions {
            validated = ext.validate(&validated, cortical).await?;
        }
        Ok(validated)
    }

    /// Run EXECUTE step across all extensions.
    pub async fn run_execute(
        &mut self,
        actions: &[ProposedAction],
        tier: &CognitiveTier,
        cancel: &CancelToken,
    ) -> Result<Vec<ActionOutcome>> {
        self.ensure_validated()?;
        let mut outcomes = Vec::new();
        for ext in &mut self.extensions {
            let out = ext.execute(actions, tier, cancel).await?;
            outcomes.extend(out);
        }
        Ok(outcomes)
    }

    /// Run VERIFY step across all extensions.
    pub async fn run_verify(
        &mut self,
        outcomes: &[ActionOutcome],
        cancel: &CancelToken,
    ) -> Result<Vec<Verification>> {
        self.ensure_validated()?;
        let mut all = Vec::new();
        for ext in &mut self.extensions {
            let v = ext.verify(outcomes, cancel).await?;
            all.extend(v);
        }
        Ok(all)
    }

    /// Run REFLECT step across all extensions.
    pub async fn run_reflect(
        &mut self,
        record: &DecisionCycleRecord,
        cortical: &Arc<CorticalState>,
    ) -> Result<()> {
        self.ensure_validated()?;
        for ext in &mut self.extensions {
            ext.reflect(record, cortical).await?;
        }
        Ok(())
    }

    fn ensure_validated(&self) -> Result<()> {
        if !self.validated {
            bail!("ExtensionChain must be validated before use (call .validate())");
        }
        Ok(())
    }
}

impl Default for ExtensionChain {
    fn default() -> Self {
        Self::new()
    }
}
```

4. Wire into `lib.rs`: `pub mod chain;` and `pub use chain::ExtensionChain;`.

**Context/Prompt for implementing agent:**

> Read `crates/roko-runtime/src/extension.rs` for the `Extension` trait and its 22 methods. Create `crates/roko-runtime/src/chain.rs` with `ExtensionChain`. The chain holds a `Vec<Box<dyn Extension>>`, sorts by `ExtensionLayer` on `validate()`, checks that all declared `depends_on` names exist, and dispatches pipeline steps by iterating extensions in order. Each pipeline method (`run_observe`, `run_retrieve`, etc.) calls the corresponding `Extension` method on every extension and collects results. `run_validate` is special: it chains validators so each extension filters the action list.

**Acceptance:**
- Write a test: create a chain with two mock extensions (Foundation and Action layer), validate, run `run_observe`, assert both produce observations
- Write a test: chain with a missing dependency fails validation with descriptive error
- Write a test: chain with duplicate names fails validation

---

### Task 5: Extend `CorticalState` with pipeline fields

- [ ] Modify `crates/roko-runtime/src/heartbeat.rs`

**Files to modify:**
- `crates/roko-runtime/src/heartbeat.rs`

**What to do:**

1. Read `crates/roko-runtime/src/heartbeat.rs` lines 226-550 to understand the existing `CorticalState` struct. It already has 21 atomic fields.

2. Add 11 new atomic fields to `CorticalState` after the existing `compounding_momentum` field:

```rust
// ── Pipeline state (added for extension system) ────────────────
/// Current cognitive tier (0=T0, 1=T1, 2=T2).
cognitive_tier: AtomicU8,
/// Aggregate prediction error from the last tick.
prediction_error: AtomicU32,
/// Sleep pressure accumulator in [0.0, 1.0].
sleep_pressure: AtomicU32,
/// Ticks since last T2 escalation.
ticks_since_escalation: AtomicU32,
/// Total ticks executed since agent start.
total_ticks: AtomicU64,
/// Total cost in USD (stored as f32 bits).
total_cost_usd: AtomicU32,
/// Number of registered extensions.
extension_count: AtomicU16,
/// Energy pool remaining in [0.0, 1.0].
energy_remaining: AtomicU32,
/// Context window utilization in [0.0, 1.0].
context_utilization: AtomicU32,
/// Current DomainProfile id hash (first 4 bytes).
domain_hash: AtomicU32,
/// Whether the agent is currently executing an action (flag).
executing: AtomicU8,
```

3. Add corresponding accessor methods following the existing pattern (e.g., `set_cognitive_tier`/`cognitive_tier`, `set_prediction_error`/`prediction_error`, etc.).

4. Add the new fields to `CorticalSnapshot` and the `snapshot()` method.

5. Update `CorticalState::new()` to initialize the new fields to defaults.

**Context/Prompt for implementing agent:**

> Open `crates/roko-runtime/src/heartbeat.rs`. Find `struct CorticalState` (around line 228). It already has 21 atomic fields like `pleasure: AtomicU32`, `regime: AtomicU8`, etc. Add 11 new fields for pipeline state: `cognitive_tier`, `prediction_error`, `sleep_pressure`, `ticks_since_escalation`, `total_ticks`, `total_cost_usd`, `extension_count`, `energy_remaining`, `context_utilization`, `domain_hash`, `executing`. Follow the exact same pattern as existing fields (AtomicU32 for floats via `to_bits()`/`from_bits()`, AtomicU8 for small enums, AtomicU64 for counters). Add read/write methods matching the existing style (e.g., `pub fn set_sleep_pressure(&self, p: f32)` / `pub fn sleep_pressure(&self) -> f32`). Update `CorticalSnapshot` and `snapshot()`. Update `new()` with sensible defaults (0 for counters, 1.0 for energy, 0.0 for pressure).

**Acceptance:**
- All existing tests in `heartbeat.rs` still pass
- `state.set_prediction_error(0.75); assert!((state.prediction_error() - 0.75).abs() < 0.001);`
- `CorticalSnapshot` serializes to JSON including the new fields

---

### Task 6: Define `DomainProfile`

- [ ] Create `crates/roko-runtime/src/domain.rs`

**Files to create:**
- `crates/roko-runtime/src/domain.rs`

**Files to modify:**
- `crates/roko-runtime/src/lib.rs` (add `pub mod domain;` and re-exports)

**What to do:**

1. Read `crates/roko-runtime/src/heartbeat.rs` for `HeartbeatSpeed`, `ClockConfig`, `PersonalityPreset`.

2. Read `crates/roko-runtime/src/cognitive.rs` for `ExtensionLayer`.

3. Define:

```rust
use serde::{Deserialize, Serialize};

use crate::heartbeat::{ClockConfig, PersonalityPreset};

/// Which operating frequency governs the default gamma tick interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Frequency {
    /// Fast reactive loop (5-15s gamma). For environments with high event rates.
    /// Blockchain agents, market watchers, real-time monitors.
    Gamma,
    /// Medium reflective loop (30-120s gamma). For task-oriented work.
    /// Coding agents, research agents, plan executors.
    Theta,
    /// Slow consolidation loop (5-30min gamma). For background processes.
    /// Knowledge distillation, log analysis, archival.
    Delta,
}

impl Frequency {
    /// Default gamma interval for this frequency.
    pub const fn default_gamma_secs(self) -> u64 {
        match self {
            Self::Gamma => 5,
            Self::Theta => 60,
            Self::Delta => 300,
        }
    }
}

/// A domain profile configures an agent for a specific operating environment.
///
/// The profile determines which extensions load, what frequency the heartbeat
/// runs at, what personality preset initializes affect state, and what budget
/// limits apply.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DomainProfile {
    /// Profile name (e.g., "coding", "blockchain", "research").
    pub name: String,

    /// Operating frequency.
    pub frequency: Frequency,

    /// Personality preset for initial PAD state.
    pub personality: PersonalityPreset,

    /// Clock configuration overrides. If None, uses defaults for the frequency.
    pub clock_config: Option<ClockConfig>,

    /// Extension names to load for this profile.
    /// The runtime resolves these to concrete `Extension` instances at startup.
    pub extensions: Vec<String>,

    /// Daily budget limit in USD. 0.0 means no limit.
    pub daily_budget_usd: f64,

    /// Maximum concurrent actions this agent can execute.
    pub max_concurrent_actions: usize,

    /// Working directory for this agent's operations.
    pub workdir: Option<std::path::PathBuf>,

    /// Tool profile name to load (maps to roko-std tool sets).
    pub tool_profile: Option<String>,

    /// MCP server configuration path.
    pub mcp_config: Option<std::path::PathBuf>,

    /// Arbitrary metadata passed to extensions during initialization.
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

impl DomainProfile {
    /// Create a minimal coding profile.
    pub fn coding() -> Self {
        Self {
            name: "coding".into(),
            frequency: Frequency::Theta,
            personality: PersonalityPreset::Balanced,
            clock_config: None,
            extensions: vec![
                "heartbeat".into(),
                "context".into(),
                "learning".into(),
                "gate".into(),
                "conductor".into(),
            ],
            daily_budget_usd: 50.0,
            max_concurrent_actions: 1,
            workdir: None,
            tool_profile: Some("standard".into()),
            mcp_config: None,
            metadata: Default::default(),
        }
    }

    /// Create a minimal blockchain profile.
    pub fn blockchain() -> Self {
        Self {
            name: "blockchain".into(),
            frequency: Frequency::Gamma,
            personality: PersonalityPreset::Cautious,
            clock_config: None,
            extensions: vec![
                "heartbeat".into(),
                "chain-subscriber".into(),
                "risk".into(),
                "context".into(),
                "learning".into(),
            ],
            daily_budget_usd: 100.0,
            max_concurrent_actions: 1,
            workdir: None,
            tool_profile: Some("blockchain".into()),
            mcp_config: None,
            metadata: Default::default(),
        }
    }

    /// Create a minimal research profile.
    pub fn research() -> Self {
        Self {
            name: "research".into(),
            frequency: Frequency::Theta,
            personality: PersonalityPreset::Balanced,
            clock_config: None,
            extensions: vec![
                "heartbeat".into(),
                "knowledge-graph".into(),
                "context".into(),
                "learning".into(),
            ],
            daily_budget_usd: 30.0,
            max_concurrent_actions: 1,
            workdir: None,
            tool_profile: Some("research".into()),
            mcp_config: None,
            metadata: Default::default(),
        }
    }
}
```

4. Wire into `lib.rs`.

**Context/Prompt for implementing agent:**

> Read `crates/roko-runtime/src/heartbeat.rs` for `ClockConfig`, `PersonalityPreset`, and `HeartbeatSpeed`. Create `crates/roko-runtime/src/domain.rs` with `Frequency` (Gamma/Theta/Delta) and `DomainProfile`. The profile describes an agent's operating environment: which extensions to load, what frequency to tick at, personality preset, budget, tool profile, etc. Include three factory methods: `coding()`, `blockchain()`, `research()` with sensible defaults. Wire into `lib.rs`.

**Acceptance:**
- `DomainProfile::coding()` round-trips through serde_json
- `DomainProfile::blockchain().frequency == Frequency::Gamma`
- `DomainProfile::research().extensions` contains "knowledge-graph"

---

### Task 7: Rewrite `Agent<Phase>` type-state

- [ ] Create `crates/roko-runtime/src/agent.rs`

**Files to create:**
- `crates/roko-runtime/src/agent.rs`

**Files to modify:**
- `crates/roko-runtime/src/lib.rs` (add `pub mod agent;` and re-exports)

**What to do:**

1. Read `crates/roko-runtime/src/lifecycle.rs` to see the existing `Agent<S>` type-state (8 phases: Unvalidated through Ready). This focuses on provisioning. The new agent type-state covers the full lifecycle including running and termination.

2. The existing lifecycle types stay. The new `agent.rs` defines a runtime-phase agent that starts where the provisioning pipeline ends.

```rust
use std::{marker::PhantomData, sync::Arc};

use crate::{
    cancel::CancelToken,
    chain::ExtensionChain,
    domain::DomainProfile,
    heartbeat::CorticalState,
};

/// Type-state marker: agent is being provisioned (extensions loading).
pub struct Provisioning;
/// Type-state marker: agent heartbeat is running.
pub struct Active;
/// Type-state marker: agent is in delta/dream consolidation.
pub struct Dreaming;
/// Type-state marker: agent is paused by operator or budget.
pub struct Suspended;
/// Type-state marker: agent has terminated.
pub struct Terminal;

/// Runtime agent in a specific lifecycle phase.
///
/// The type parameter `Phase` enforces valid operations at compile time:
/// - `Agent<Provisioning>` can add extensions and transition to Active
/// - `Agent<Active>` can run ticks and transition to Dreaming or Suspended
/// - `Agent<Dreaming>` can run consolidation and return to Active
/// - `Agent<Suspended>` can resume to Active or terminate
/// - `Agent<Terminal>` is a tombstone; no operations available
pub struct RuntimeAgent<Phase> {
    /// Unique agent identifier.
    id: String,
    /// Domain profile this agent was configured with.
    profile: DomainProfile,
    /// Extension chain (moved between phases).
    chain: ExtensionChain,
    /// Lock-free shared perception surface.
    cortical: Arc<CorticalState>,
    /// Cancellation token for cooperative shutdown.
    cancel: CancelToken,
    /// Phase marker.
    _phase: PhantomData<Phase>,
}

impl RuntimeAgent<Provisioning> {
    /// Create a new agent in the provisioning phase.
    pub fn new(id: impl Into<String>, profile: DomainProfile) -> Self {
        let cortical = Arc::new(CorticalState::new(profile.personality));
        Self {
            id: id.into(),
            profile,
            chain: ExtensionChain::new(),
            cortical,
            cancel: CancelToken::new(),
            _phase: PhantomData,
        }
    }

    /// Add an extension during provisioning.
    pub fn add_extension(&mut self, ext: impl crate::extension::Extension + 'static) {
        self.chain.add(ext);
    }

    /// Validate extensions and transition to Active.
    pub async fn activate(mut self) -> anyhow::Result<RuntimeAgent<Active>> {
        self.chain.validate()?;
        self.chain.start_all(&self.cortical).await?;
        Ok(RuntimeAgent {
            id: self.id,
            profile: self.profile,
            chain: self.chain,
            cortical: self.cortical,
            cancel: self.cancel,
            _phase: PhantomData,
        })
    }
}

impl RuntimeAgent<Active> {
    /// Access the agent's unique identifier.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Access the shared cortical state.
    pub fn cortical(&self) -> &Arc<CorticalState> {
        &self.cortical
    }

    /// Access the domain profile.
    pub fn profile(&self) -> &DomainProfile {
        &self.profile
    }

    /// Access the cancellation token.
    pub fn cancel(&self) -> &CancelToken {
        &self.cancel
    }

    /// Mutable access to the extension chain for running pipeline steps.
    pub fn chain_mut(&mut self) -> &mut ExtensionChain {
        &mut self.chain
    }

    /// Transition to the Dreaming phase for consolidation.
    pub fn enter_dream(self) -> RuntimeAgent<Dreaming> {
        RuntimeAgent {
            id: self.id,
            profile: self.profile,
            chain: self.chain,
            cortical: self.cortical,
            cancel: self.cancel,
            _phase: PhantomData,
        }
    }

    /// Transition to Suspended (operator pause or budget constraint).
    pub fn suspend(self) -> RuntimeAgent<Suspended> {
        RuntimeAgent {
            id: self.id,
            profile: self.profile,
            chain: self.chain,
            cortical: self.cortical,
            cancel: self.cancel,
            _phase: PhantomData,
        }
    }

    /// Shut down and transition to Terminal.
    pub async fn terminate(mut self) -> anyhow::Result<RuntimeAgent<Terminal>> {
        self.cancel.cancel();
        self.chain.stop_all().await?;
        Ok(RuntimeAgent {
            id: self.id,
            profile: self.profile,
            chain: self.chain,
            cortical: self.cortical,
            cancel: self.cancel,
            _phase: PhantomData,
        })
    }
}

impl RuntimeAgent<Dreaming> {
    /// Return to Active after consolidation.
    pub fn wake(self) -> RuntimeAgent<Active> {
        RuntimeAgent {
            id: self.id,
            profile: self.profile,
            chain: self.chain,
            cortical: self.cortical,
            cancel: self.cancel,
            _phase: PhantomData,
        }
    }
}

impl RuntimeAgent<Suspended> {
    /// Resume to Active.
    pub fn resume(self) -> RuntimeAgent<Active> {
        RuntimeAgent {
            id: self.id,
            profile: self.profile,
            chain: self.chain,
            cortical: self.cortical,
            cancel: self.cancel,
            _phase: PhantomData,
        }
    }

    /// Terminate from suspended state.
    pub async fn terminate(mut self) -> anyhow::Result<RuntimeAgent<Terminal>> {
        self.cancel.cancel();
        self.chain.stop_all().await?;
        Ok(RuntimeAgent {
            id: self.id,
            profile: self.profile,
            chain: self.chain,
            cortical: self.cortical,
            cancel: self.cancel,
            _phase: PhantomData,
        })
    }
}
```

**Context/Prompt for implementing agent:**

> Read `crates/roko-runtime/src/lifecycle.rs` for the existing provisioning type-state (`Agent<Unvalidated>` through `Agent<Ready>`). The existing type stays. Create `crates/roko-runtime/src/agent.rs` with `RuntimeAgent<Phase>` where Phase is one of: Provisioning, Active, Dreaming, Suspended, Terminal. Each phase allows only valid transitions enforced at compile time. `Provisioning` can `add_extension()` and `activate()`. `Active` can `enter_dream()`, `suspend()`, `terminate()`. `Dreaming` can `wake()`. `Suspended` can `resume()` or `terminate()`. Terminal is a tombstone with no methods.

**Acceptance:**
- `RuntimeAgent::<Provisioning>::new("test", DomainProfile::coding()).activate().await?` produces `RuntimeAgent<Active>`
- `RuntimeAgent<Terminal>` has no callable methods (verify by attempting to call `.id()` on a Terminal agent -- it should fail to compile)
- Write a test exercising the full lifecycle: Provisioning -> Active -> Dreaming -> Active -> Suspended -> Active -> Terminal

---

### Task 8: Define `HeartbeatPipeline`

- [ ] Create `crates/roko-runtime/src/pipeline.rs`

**Files to create:**
- `crates/roko-runtime/src/pipeline.rs`

**Files to modify:**
- `crates/roko-runtime/src/lib.rs` (add `pub mod pipeline;`)

**What to do:**

1. Read `crates/roko-runtime/src/chain.rs` (Task 4) for `ExtensionChain` and its `run_*` methods.

2. Read `crates/roko-runtime/src/heartbeat.rs` for `CorticalState`, `HeartbeatPolicy`, `InferenceTier`.

3. Read `crates/roko-runtime/src/cognitive.rs` for `CognitiveTier`, `TierSelectionReason`.

4. Implement the 9-step `execute_tick()` method from PRD-02 section 2. The pipeline orchestrates the extension chain through OBSERVE -> RETRIEVE -> ANALYZE -> GATE -> SIMULATE -> VALIDATE -> EXECUTE -> VERIFY -> REFLECT.

Key design points:
- Pipeline owns a mutable reference to `ExtensionChain` (borrowed per tick, not owned)
- T0 ticks short-circuit after GATE (skip SIMULATE through VERIFY)
- Aggregate PE from per-extension components using weighted average
- Tier selection uses two configurable thresholds (t1_threshold, t2_threshold)
- Returns `DecisionCycleRecord`

5. Include a `TierGate` struct with adaptive EWMA thresholds:

```rust
pub struct TierGate {
    t1_threshold: f64,
    t2_threshold: f64,
    ewma_alpha: f64,
}
```

**Context/Prompt for implementing agent:**

> Read `crates/roko-runtime/src/chain.rs` for `ExtensionChain` and `crates/roko-runtime/src/extension.rs` for `DecisionCycleRecord`. Create `crates/roko-runtime/src/pipeline.rs` with `HeartbeatPipeline` and `TierGate`. The pipeline's `execute_tick()` method runs the 9-step cognitive loop: OBSERVE, RETRIEVE, ANALYZE, GATE, SIMULATE, VALIDATE, EXECUTE, VERIFY, REFLECT. T0 ticks short-circuit after GATE. `TierGate` holds two EWMA-adapted thresholds for T1/T2 selection. After each tick, update thresholds: `threshold = alpha * observed_pe + (1 - alpha) * threshold`.

**Acceptance:**
- Write a test: pipeline with no extensions, PE = 0.0, returns T0 record with empty actions
- Write a test: pipeline with a mock extension that returns PE > t2_threshold, verify T2 selected
- Write a test: T0 tick completes in <1ms (no async work, no LLM)
- Verify `DecisionCycleRecord.tier.tier == InferenceTier::T0` for sub-threshold PE

---

### Task 9: Define `RuntimeEvent` types

- [ ] Create `crates/roko-runtime/src/runtime_event.rs`

**Files to create:**
- `crates/roko-runtime/src/runtime_event.rs`

**Files to modify:**
- `crates/roko-runtime/src/lib.rs` (add module and re-exports)

**What to do:**

1. Read `crates/roko-runtime/src/event_bus.rs` for the existing `RokoEvent` enum (8 variants).

2. The existing `RokoEvent` stays for backwards compatibility. Create new types that the extension system uses alongside it.

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    cognitive::ExtensionLayer,
    extension::DecisionCycleRecord,
};
use roko_primitives::tier::InferenceTier;

/// Source of a runtime event.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventSource {
    /// Event from the heartbeat pipeline itself.
    Pipeline,
    /// Event from a specific extension.
    Extension { name: String, layer: ExtensionLayer },
    /// Event from the conductor.
    Conductor,
    /// Event from the supervisor.
    Supervisor,
    /// Event from an external operator.
    Operator,
}

/// Filter for event subscriptions.
#[derive(Debug, Clone)]
pub enum EventFilter {
    /// Receive all events.
    All,
    /// Receive events from a specific source.
    Source(EventSource),
    /// Receive events from a specific extension layer.
    Layer(ExtensionLayer),
    /// Receive events matching a custom predicate.
    Custom(String),
}

/// Events produced by the extension-based runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuntimeEvent {
    /// A heartbeat tick completed.
    TickCompleted {
        agent_id: String,
        tick: u64,
        tier: InferenceTier,
        duration_ms: u64,
        prediction_error: f64,
        source: EventSource,
        at: DateTime<Utc>,
    },
    /// An extension produced an observation.
    ObservationRecorded {
        agent_id: String,
        extension: String,
        salience: f64,
        at: DateTime<Utc>,
    },
    /// An action was executed.
    ActionExecuted {
        agent_id: String,
        extension: String,
        action_type: String,
        success: bool,
        duration_ms: u64,
        at: DateTime<Utc>,
    },
    /// Tier was escalated from T0 to T1 or T2.
    TierEscalation {
        agent_id: String,
        from: InferenceTier,
        to: InferenceTier,
        prediction_error: f64,
        at: DateTime<Utc>,
    },
    /// Agent lifecycle changed.
    LifecycleChanged {
        agent_id: String,
        from_phase: String,
        to_phase: String,
        at: DateTime<Utc>,
    },
    /// Budget threshold crossed.
    BudgetAlert {
        agent_id: String,
        remaining_pct: f64,
        daily_spend_usd: f64,
        at: DateTime<Utc>,
    },
    /// Extension reported an error (non-fatal).
    ExtensionError {
        agent_id: String,
        extension: String,
        error: String,
        at: DateTime<Utc>,
    },
}
```

**Context/Prompt for implementing agent:**

> Read `crates/roko-runtime/src/event_bus.rs` for the existing `RokoEvent` enum. Do not modify it. Create `crates/roko-runtime/src/runtime_event.rs` with `RuntimeEvent` (new events for the extension system), `EventSource`, and `EventFilter`. These types will be used by the upgraded `EventFabric` in Task 10. Follow the same serde patterns as `RokoEvent`.

**Acceptance:**
- `RuntimeEvent::TickCompleted { .. }` round-trips through serde_json
- `EventSource::Extension { name: "gate".into(), layer: ExtensionLayer::Analysis }` serializes correctly

---

### Task 10: Upgrade `EventBus` with filtered subscriptions

- [ ] Create `crates/roko-runtime/src/event_fabric.rs`

**Files to create:**
- `crates/roko-runtime/src/event_fabric.rs`

**Files to modify:**
- `crates/roko-runtime/src/lib.rs` (add module)

**What to do:**

1. Read `crates/roko-runtime/src/event_bus.rs` for the existing `EventBus<E>`. Do not modify it -- it stays for backwards compatibility.

2. Create `EventFabric` that wraps `EventBus<RuntimeEvent>` and adds filtered subscription:

```rust
use std::sync::Arc;

use crate::{
    event_bus::{BusSender, EventBus, Envelope},
    runtime_event::{EventFilter, EventSource, RuntimeEvent},
};

/// Upgraded event system with filtered subscriptions.
///
/// Wraps the existing `EventBus<RuntimeEvent>` and adds the ability to
/// subscribe with a filter. The filter is applied on the receive side,
/// not the send side (all events are broadcast to all subscribers;
/// filtering happens in the consumer).
pub struct EventFabric {
    bus: EventBus<RuntimeEvent>,
}

impl EventFabric {
    pub fn new(capacity: usize) -> Self {
        Self {
            bus: EventBus::new(capacity),
        }
    }

    pub fn emit(&self, event: RuntimeEvent) {
        self.bus.emit(event);
    }

    pub fn sender(&self) -> BusSender<RuntimeEvent> {
        self.bus.sender()
    }

    /// Subscribe with a filter. Returns a `FilteredSubscriber` that
    /// only yields events matching the filter.
    pub fn subscribe_filtered(&self, filter: EventFilter) -> FilteredSubscriber {
        FilteredSubscriber {
            rx: self.bus.subscribe(),
            filter,
        }
    }

    /// Unfiltered subscription (same as `EventBus::subscribe`).
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<Envelope<RuntimeEvent>> {
        self.bus.subscribe()
    }

    pub fn replay_from(&self, after_seq: u64) -> Vec<Envelope<RuntimeEvent>> {
        self.bus.replay_from(after_seq)
    }
}

pub struct FilteredSubscriber {
    rx: tokio::sync::broadcast::Receiver<Envelope<RuntimeEvent>>,
    filter: EventFilter,
}

impl FilteredSubscriber {
    /// Receive the next event matching the filter.
    /// Blocks until a matching event arrives or the channel closes.
    pub async fn recv(&mut self) -> Result<Envelope<RuntimeEvent>, tokio::sync::broadcast::error::RecvError> {
        loop {
            let envelope = self.rx.recv().await?;
            if self.matches(&envelope.payload) {
                return Ok(envelope);
            }
        }
    }

    fn matches(&self, event: &RuntimeEvent) -> bool {
        match &self.filter {
            EventFilter::All => true,
            EventFilter::Source(source) => event_source(event) == *source,
            EventFilter::Layer(layer) => {
                if let EventSource::Extension { layer: l, .. } = event_source(event) {
                    l == *layer
                } else {
                    false
                }
            }
            EventFilter::Custom(_) => true, // Custom filters need a predicate; placeholder
        }
    }
}

fn event_source(event: &RuntimeEvent) -> EventSource {
    match event {
        RuntimeEvent::TickCompleted { source, .. } => source.clone(),
        RuntimeEvent::ObservationRecorded { extension, .. } => EventSource::Extension {
            name: extension.clone(),
            layer: crate::cognitive::ExtensionLayer::Perception,
        },
        RuntimeEvent::ActionExecuted { extension, .. } => EventSource::Extension {
            name: extension.clone(),
            layer: crate::cognitive::ExtensionLayer::Action,
        },
        RuntimeEvent::TierEscalation { .. } => EventSource::Pipeline,
        RuntimeEvent::LifecycleChanged { .. } => EventSource::Pipeline,
        RuntimeEvent::BudgetAlert { .. } => EventSource::Pipeline,
        RuntimeEvent::ExtensionError { extension, .. } => EventSource::Extension {
            name: extension.clone(),
            layer: crate::cognitive::ExtensionLayer::Foundation,
        },
    }
}
```

**Context/Prompt for implementing agent:**

> Read `crates/roko-runtime/src/event_bus.rs` for the existing `EventBus<E>`. Do not modify it. Read `crates/roko-runtime/src/runtime_event.rs` (Task 9) for `RuntimeEvent`, `EventSource`, `EventFilter`. Create `crates/roko-runtime/src/event_fabric.rs` with `EventFabric` that wraps `EventBus<RuntimeEvent>` and adds `subscribe_filtered()`. The filter runs on the receive side: the subscriber loops internally, discarding non-matching events.

**Acceptance:**
- Write a test: emit 3 events from different sources, subscribe with `EventFilter::Source(Pipeline)`, verify only pipeline events received
- Write a test: unfiltered subscriber receives all events
- Existing `EventBus<RokoEvent>` tests still pass (nothing modified)

---

## Phase 2: Core extensions (roko-ext-core)

Phase 2 creates a new crate `roko-ext-core` with extensions that wrap existing crate functionality.

---

### Task 11: Create `roko-ext-core` crate scaffold

- [ ] Create `crates/roko-ext-core/`

**Files to create:**
- `crates/roko-ext-core/Cargo.toml`
- `crates/roko-ext-core/src/lib.rs`

**Files to modify:**
- `Cargo.toml` (workspace root -- add to `[workspace.members]`)

**What to do:**

1. Read the workspace root `Cargo.toml` to see the member list pattern.

2. Create the crate with dependencies on `roko-runtime`, `roko-compose`, `roko-daimon`, `roko-learn`, `roko-dreams`, `roko-conductor`, `roko-neuro`.

3. `lib.rs` declares the extension modules (to be filled in Tasks 12-15).

**Context/Prompt for implementing agent:**

> Read the workspace root `Cargo.toml` to see how other crates are declared. Create `crates/roko-ext-core/` with `Cargo.toml` depending on `roko-runtime` (for the Extension trait), `roko-compose`, `roko-daimon`, `roko-learn`, `roko-dreams`, `roko-conductor`, `roko-neuro`. Create `src/lib.rs` with `pub mod heartbeat_ext; pub mod context_ext; pub mod daimon_ext; pub mod learning_ext; pub mod dreams_ext;` and placeholder files for each. Add the crate to workspace members.

**Acceptance:**
- `cargo build -p roko-ext-core` succeeds
- `cargo test --workspace` still passes

---

### Task 12: Implement `HeartbeatExt`

- [ ] Create `crates/roko-ext-core/src/heartbeat_ext.rs`

**What to do:**

1. Read `crates/roko-runtime/src/heartbeat.rs` for `HeartbeatPolicy`, `ClockConfig`, `HeartbeatSpeed`.

2. Read `crates/roko-runtime/src/extension.rs` for the `Extension` trait.

3. Implement an extension that manages the heartbeat clock. It:
   - Registers at `ExtensionLayer::Foundation`
   - On `on_start`: initializes the `HeartbeatPolicy` with config from the `DomainProfile`
   - On `on_gamma_tick`: adjusts gamma interval based on anomaly count (calls `compute_gamma_interval`)
   - On `on_theta_tick`: adjusts theta interval based on regime (calls `compute_theta_interval`)
   - On `observe`: reads tick counters from `CorticalState`, returns observation with current tick rates
   - On `analyze`: contributes PE component based on tick frequency deviation from expected

**Context/Prompt for implementing agent:**

> Read `crates/roko-runtime/src/heartbeat.rs` (HeartbeatPolicy, compute_gamma_interval, compute_theta_interval) and `crates/roko-runtime/src/extension.rs` (Extension trait). Create `crates/roko-ext-core/src/heartbeat_ext.rs` implementing `Extension` for `HeartbeatExt`. This is a Foundation-layer extension that wraps `HeartbeatPolicy`. Override `on_start`, `on_gamma_tick`, `on_theta_tick`, `observe`, and `analyze`.

**Acceptance:**
- `HeartbeatExt` compiles and implements all required trait methods
- `descriptor().layer == ExtensionLayer::Foundation`
- `descriptor().depends_on` is empty (no dependencies)

---

### Task 13: Implement `ContextExt`

- [ ] Create `crates/roko-ext-core/src/context_ext.rs`

**What to do:**

1. Read `crates/roko-compose/src/system_prompt_builder.rs` for `SystemPromptBuilder`.

2. This extension wraps the context/prompt assembly from `roko-compose`:
   - Layer: `Knowledge`
   - On `retrieve`: assembles context from the workspace, returns retrievals with token estimates
   - On `reflect`: updates context caches based on what was useful this tick
   - Depends on: "heartbeat"

**Context/Prompt for implementing agent:**

> Read `crates/roko-compose/src/system_prompt_builder.rs` for the prompt assembly API. Create `crates/roko-ext-core/src/context_ext.rs` implementing `Extension` for `ContextExt`. Knowledge layer. Wraps `roko-compose` context assembly. Override `retrieve` (query relevant context) and `reflect` (update caches). Depends on "heartbeat".

**Acceptance:**
- `ContextExt` compiles
- `descriptor().layer == ExtensionLayer::Knowledge`
- `descriptor().depends_on == vec!["heartbeat"]`

---

### Task 14: Implement `DaimonExt`

- [ ] Create `crates/roko-ext-core/src/daimon_ext.rs`

**What to do:**

1. Read `crates/roko-daimon/src/lib.rs` for the affect engine API (PAD state, somatic markers).

2. This extension wraps daimon affect modeling:
   - Layer: `Affect`
   - On `on_start`: loads somatic marker database from disk
   - On `observe`: reads current PAD from `CorticalState`, returns observation with emotional context
   - On `analyze`: computes affect-based PE component (surprise from somatic marker mismatch)
   - On `reflect`: updates PAD based on action outcomes, stores new somatic markers
   - Depends on: "heartbeat", "context"

**Context/Prompt for implementing agent:**

> Read `crates/roko-daimon/src/lib.rs` for the affect engine (DaimonState, PAD vectors, somatic markers). Create `crates/roko-ext-core/src/daimon_ext.rs` implementing `Extension` for `DaimonExt`. Affect layer. Wraps `roko-daimon`. Override `on_start` (load markers), `observe` (read PAD), `analyze` (affect-based PE), `reflect` (update PAD + store markers). Depends on "heartbeat" and "context".

**Acceptance:**
- `DaimonExt` compiles
- `descriptor().layer == ExtensionLayer::Affect`
- Dependency chain: heartbeat -> context -> daimon (validated by ExtensionChain)

---

### Task 15: Implement `LearningExt`

- [ ] Create `crates/roko-ext-core/src/learning_ext.rs`

**What to do:**

1. Read `crates/roko-learn/src/lib.rs` for episode logger, playbooks, skill library.

2. This extension wraps the learning subsystem:
   - Layer: `Learning`
   - On `reflect`: logs episode, extracts skills, updates playbooks
   - On `on_theta_tick`: runs pattern discovery on accumulated episodes
   - Depends on: "heartbeat", "context"

**Context/Prompt for implementing agent:**

> Read `crates/roko-learn/src/lib.rs` for the learning API (episode_logger, playbook, skill_library, anomaly). Create `crates/roko-ext-core/src/learning_ext.rs` implementing `Extension` for `LearningExt`. Learning layer. Override `reflect` (log episode, extract skills) and `on_theta_tick` (pattern discovery). Depends on "heartbeat" and "context".

**Acceptance:**
- `LearningExt` compiles
- `descriptor().layer == ExtensionLayer::Learning`

---

### Task 16 (bonus): Implement `DreamsExt`

- [ ] Create `crates/roko-ext-core/src/dreams_ext.rs`

**What to do:**

1. Read `crates/roko-dreams/src/lib.rs` for dream cycle, hypnagogia, replay.

2. This extension manages sleep pressure and dream cycles:
   - Layer: `Recovery`
   - On `observe`: reads sleep pressure from `CorticalState`
   - On `on_delta_tick`: triggers dream cycle if sleep pressure exceeds threshold
   - On `reflect`: accumulates sleep pressure based on tick novelty
   - Depends on: "heartbeat", "learning"

**Context/Prompt for implementing agent:**

> Read `crates/roko-dreams/src/lib.rs` for the dream cycle API (DreamCycle, hypnagogia, replay). Create `crates/roko-ext-core/src/dreams_ext.rs` implementing `Extension` for `DreamsExt`. Recovery layer. Manages sleep pressure tracking and triggers dream consolidation on delta ticks. Depends on "heartbeat" and "learning".

**Acceptance:**
- `DreamsExt` compiles
- `descriptor().layer == ExtensionLayer::Recovery`
- Sleep pressure accumulates across ticks (write a unit test)

---

## Phase 3: Domain extensions

---

### Task 17: Create `roko-ext-code` crate

- [ ] Create `crates/roko-ext-code/`

**Files to create:**
- `crates/roko-ext-code/Cargo.toml`
- `crates/roko-ext-code/src/lib.rs`
- `crates/roko-ext-code/src/git_ext.rs`
- `crates/roko-ext-code/src/gate_ext.rs`
- `crates/roko-ext-code/src/conductor_ext.rs`

**What to do:**

1. `GitExt` (Perception layer): observes git status changes, proposes commits.
   - On `observe`: runs `git status --porcelain`, returns observations about dirty files
   - On `simulate`: proposes `git add`/`git commit` actions for changed files
   - Depends on: "heartbeat"

2. `GateExt` (Analysis layer): wraps the roko-gate pipeline.
   - On `verify`: runs compile/test/clippy gates on action outcomes
   - On `analyze`: computes PE from recent gate pass/fail ratio
   - Depends on: "heartbeat", "context"

3. `ConductorExt` (Analysis layer): wraps roko-conductor watchers.
   - On `analyze`: runs all 10 watchers, returns conductor-derived PE
   - On `on_cognitive_signal`: forwards signals to conductor for intervention decisions
   - Depends on: "heartbeat", "gate"

**Context/Prompt for implementing agent:**

> Create `crates/roko-ext-code/` with three extensions. Read `crates/roko-gate/src/lib.rs` for the gate pipeline. Read `crates/roko-conductor/src/lib.rs` for watchers and conductor. `GitExt` observes git state (Perception layer). `GateExt` verifies via compile/test/clippy (Analysis layer). `ConductorExt` runs anomaly watchers (Analysis layer). Wire all three into `lib.rs`. Add to workspace members.

**Acceptance:**
- `cargo build -p roko-ext-code` succeeds
- Each extension has correct layer and dependencies

---

### Task 18: Create `roko-ext-chain` crate

- [ ] Create `crates/roko-ext-chain/`

**Files to create:**
- `crates/roko-ext-chain/Cargo.toml`
- `crates/roko-ext-chain/src/lib.rs`
- `crates/roko-ext-chain/src/chain_subscriber_ext.rs`
- `crates/roko-ext-chain/src/risk_ext.rs`

**What to do:**

1. `ChainSubscriberExt` (Perception layer): subscribes to Korai blocks.
   - On `observe`: reads latest block from a subscription, returns observations about new transactions
   - On `analyze`: computes PE from price deviation and transaction novelty

2. `RiskExt` (Analysis layer): risk assessment for chain operations.
   - On `validate`: checks transaction safety against risk thresholds
   - On `simulate`: dry-runs transactions against a local fork

**Context/Prompt for implementing agent:**

> Create `crates/roko-ext-chain/` for blockchain-domain extensions. `ChainSubscriberExt` is a Perception-layer extension that observes new blocks. `RiskExt` is an Analysis-layer extension that validates transaction safety. These are stubs for now -- the chain integration is Phase 2+. Implement the Extension trait with placeholder logic that returns empty results.

**Acceptance:**
- `cargo build -p roko-ext-chain` succeeds
- Extensions have correct layer declarations

---

### Task 19: Create `roko-ext-research` crate

- [ ] Create `crates/roko-ext-research/`

**Files to create:**
- `crates/roko-ext-research/Cargo.toml`
- `crates/roko-ext-research/src/lib.rs`
- `crates/roko-ext-research/src/knowledge_graph_ext.rs`
- `crates/roko-ext-research/src/source_watcher_ext.rs`

**What to do:**

1. `KnowledgeGraphExt` (Knowledge layer): wraps roko-neuro knowledge store.
   - On `retrieve`: queries the NeuroStore for entries relevant to current observations
   - On `reflect`: promotes validated observations to knowledge entries

2. `SourceWatcherExt` (Perception layer): monitors external sources.
   - On `observe`: checks RSS feeds, API endpoints, or file changes for new information

**Context/Prompt for implementing agent:**

> Create `crates/roko-ext-research/` for research-domain extensions. Read `crates/roko-neuro/src/lib.rs` for the knowledge store API. `KnowledgeGraphExt` (Knowledge layer) wraps NeuroStore queries and knowledge promotion. `SourceWatcherExt` (Perception layer) monitors external information sources. Wire into `lib.rs`. Add to workspace members.

**Acceptance:**
- `cargo build -p roko-ext-research` succeeds
- `KnowledgeGraphExt` depends on "heartbeat"

---

## Phase 4: PlanRunner migration

---

### Task 20: Create `spawn_agent()` function

- [ ] Create `crates/roko-runtime/src/spawn.rs`

**Files to create:**
- `crates/roko-runtime/src/spawn.rs`

**What to do:**

1. Read `crates/roko-runtime/src/agent.rs` (Task 7) for `RuntimeAgent<Phase>`.
2. Read `crates/roko-runtime/src/domain.rs` (Task 6) for `DomainProfile`.

3. Create a function that builds a `RuntimeAgent<Active>` from a `DomainProfile`:

```rust
use anyhow::Result;
use crate::{agent::RuntimeAgent, domain::DomainProfile};

/// Registry of extension constructors keyed by name.
pub type ExtensionFactory = Box<dyn Fn(&DomainProfile) -> Box<dyn crate::extension::Extension>>;

/// Build and activate an agent from a domain profile.
///
/// Resolves extension names from the profile to concrete instances using
/// the provided factories, validates the chain, and returns an Active agent.
pub async fn spawn_agent(
    id: impl Into<String>,
    profile: DomainProfile,
    factories: &std::collections::HashMap<String, ExtensionFactory>,
) -> Result<RuntimeAgent<crate::agent::Active>> {
    let mut agent = RuntimeAgent::new(id, profile.clone());

    for ext_name in &profile.extensions {
        let factory = factories
            .get(ext_name)
            .ok_or_else(|| anyhow::anyhow!("no factory for extension '{}'", ext_name))?;
        agent.add_extension(factory(&profile));
    }

    agent.activate().await
}
```

**Context/Prompt for implementing agent:**

> Read `crates/roko-runtime/src/agent.rs` (RuntimeAgent) and `crates/roko-runtime/src/domain.rs` (DomainProfile). Create `crates/roko-runtime/src/spawn.rs` with `spawn_agent()` that takes a profile and a map of extension factories, builds a `RuntimeAgent<Provisioning>`, adds extensions, and calls `activate()`. The factory pattern lets callers register domain-specific extensions without the runtime knowing about them.

**Acceptance:**
- `spawn_agent("test", DomainProfile::coding(), &factories).await?` returns `RuntimeAgent<Active>`
- Missing extension factory returns descriptive error

---

### Task 21: Create `dispatch_task()` function

- [ ] Create `crates/roko-runtime/src/dispatch.rs`

**Files to create:**
- `crates/roko-runtime/src/dispatch.rs`

**What to do:**

1. This is the bridge between the existing task-based PlanRunner and the new extension-based runtime.

2. `dispatch_task()` takes a running agent and a task envelope, injects the task as a high-priority observation, and runs one full tick:

```rust
use anyhow::Result;
use crate::{
    agent::{Active, RuntimeAgent},
    extension::{DecisionCycleRecord, Observation},
    pipeline::HeartbeatPipeline,
};

/// A task to inject into a running agent's next tick.
#[derive(Debug, Clone)]
pub struct TaskEnvelope {
    /// Task identifier from the plan.
    pub task_id: String,
    /// Task description / prompt.
    pub prompt: String,
    /// Priority (higher = more urgent).
    pub priority: u32,
    /// Maximum tier allowed for this task.
    pub max_tier: roko_primitives::tier::InferenceTier,
    /// Arbitrary metadata.
    pub metadata: serde_json::Value,
}

/// Inject a task into a running agent and execute one tick.
///
/// This is the transitional API that bridges PlanRunner (task-oriented)
/// with the extension runtime (tick-oriented). The task is converted to
/// a high-salience observation and injected into the next tick's OBSERVE
/// step.
pub async fn dispatch_task(
    agent: &mut RuntimeAgent<Active>,
    task: TaskEnvelope,
    pipeline: &mut HeartbeatPipeline,
) -> Result<DecisionCycleRecord> {
    // Convert task to observation with maximum salience.
    let task_obs = Observation {
        source: "task-dispatch".into(),
        payload: serde_json::json!({
            "task_id": task.task_id,
            "prompt": task.prompt,
            "priority": task.priority,
        }),
        salience: 1.0,
    };

    // Force T2 for dispatched tasks (they always need LLM reasoning).
    // The pipeline's gate will respect max_tier.
    pipeline.inject_observation(task_obs);
    pipeline.execute_tick(agent.chain_mut(), agent.cortical(), agent.cancel()).await
}
```

**Context/Prompt for implementing agent:**

> Read `crates/roko-runtime/src/agent.rs` (RuntimeAgent<Active>) and `crates/roko-runtime/src/pipeline.rs` (HeartbeatPipeline). Create `crates/roko-runtime/src/dispatch.rs` with `TaskEnvelope` and `dispatch_task()`. This bridges the old task-based model with the new tick-based model by converting a task into a high-salience observation and running one tick. Add `inject_observation()` to `HeartbeatPipeline` if not already present.

**Acceptance:**
- `dispatch_task()` compiles and returns a `DecisionCycleRecord`
- The task observation appears in the record's `observations` vector

---

### Task 22: Create `spawn_and_run_task()` transitional API

- [ ] Create `crates/roko-runtime/src/oneshot.rs`

**Files to create:**
- `crates/roko-runtime/src/oneshot.rs`

**What to do:**

1. This is the drop-in replacement for PlanRunner's current spawn-execute-die pattern:

```rust
/// Spawn an agent, dispatch one task, terminate.
///
/// This is the backwards-compatible transitional API. It creates an agent
/// from a profile, dispatches a single task, and shuts down. PlanRunner
/// uses this as a 1:1 replacement for its current spawn_process pattern.
pub async fn spawn_and_run_task(
    id: impl Into<String>,
    profile: DomainProfile,
    task: TaskEnvelope,
    factories: &std::collections::HashMap<String, ExtensionFactory>,
) -> Result<DecisionCycleRecord> {
    let mut agent = spawn_agent(id, profile, factories).await?;
    let mut pipeline = HeartbeatPipeline::default();
    let record = dispatch_task(&mut agent, task, &mut pipeline).await?;
    agent.terminate().await?;
    Ok(record)
}
```

**Context/Prompt for implementing agent:**

> Read `crates/roko-runtime/src/spawn.rs` (spawn_agent) and `crates/roko-runtime/src/dispatch.rs` (dispatch_task). Create `crates/roko-runtime/src/oneshot.rs` with `spawn_and_run_task()` that combines spawning, dispatching, and terminating into a single call. This is the transitional API that PlanRunner will use.

**Acceptance:**
- `spawn_and_run_task("test", DomainProfile::coding(), task, &factories).await?` returns a record
- Agent is terminated after the call (verify via cancel token)

---

### Task 23: Modify PlanRunner to use `spawn_and_run_task()`

- [ ] Modify `crates/roko-cli/src/orchestrate.rs`

**What to do:**

1. Read the current `dispatch_agent_with()` method in `orchestrate.rs`. Find where it calls `spawn_process()` or the equivalent.

2. Add a `use roko_runtime::oneshot::spawn_and_run_task;` import.

3. Create a method `PlanRunner::build_domain_profile()` that converts the current config + task metadata into a `DomainProfile`.

4. Create a method `PlanRunner::build_task_envelope()` that converts a plan task into a `TaskEnvelope`.

5. Replace the spawn-execute-die sequence with `spawn_and_run_task()`. Keep the existing method signature so all callers continue to work.

6. This is additive -- wrap the new path behind a config flag `use_extension_runtime: bool` so the old path remains available.

**Context/Prompt for implementing agent:**

> This is the critical migration task. Read `crates/roko-cli/src/orchestrate.rs` and find `dispatch_agent_with()` or the method that spawns agent processes. Read `crates/roko-runtime/src/oneshot.rs` for `spawn_and_run_task()`. Add a config flag `use_extension_runtime` (default false). When true, convert the task to a `TaskEnvelope`, build a `DomainProfile` from config, and call `spawn_and_run_task()` instead of the current spawn path. Keep the old path as fallback. This is a transitional change -- both paths coexist.

**Acceptance:**
- `cargo build -p roko-cli` succeeds
- `cargo test -p roko-cli` passes (old path still works by default)
- With `use_extension_runtime = true` in config, `roko plan run plans/` uses the new path

---

### Task 24: Add `roko agent start` CLI command

- [ ] Modify `crates/roko-cli/src/main.rs` (or wherever CLI commands are defined)

**What to do:**

1. Add a new subcommand `agent start --profile <name>` that spawns a persistent agent.

2. The command:
   - Reads the profile from config or uses a builtin (coding/blockchain/research)
   - Calls `spawn_agent()` with appropriate factories
   - Runs the heartbeat loop (`HeartbeatPolicy::run()`) in a tokio task
   - Listens for ctrl-c to trigger graceful shutdown
   - Prints tick summaries to stdout

**Context/Prompt for implementing agent:**

> Read `crates/roko-cli/src/main.rs` to see how existing subcommands (plan, prd, research) are registered. Add `roko agent start --profile coding` that spawns a persistent agent using `spawn_agent()`, runs the heartbeat loop, and prints tick summaries. Use the cancel token for ctrl-c handling. This is the first command that creates a long-running agent.

**Acceptance:**
- `cargo run -p roko-cli -- agent start --profile coding` starts and prints tick output
- Ctrl-C triggers graceful shutdown (extensions receive `on_stop`)
- `--profile blockchain` uses Gamma frequency (5s ticks)

---

### Task 25: Add persistent `roko chat` integration

- [ ] Modify `crates/roko-cli/src/chat.rs`

**What to do:**

1. Read the existing `chat.rs` to understand the current chat implementation.

2. Modify it to optionally attach to a running agent (started via `roko agent start`):
   - `roko chat --agent <id>` connects to the agent's event fabric
   - User messages are injected as `TaskEnvelope` with max priority
   - Agent responses come through the `ActionOutcome` events

3. If no agent is specified, the current behavior (spawn-execute-die) continues.

**Context/Prompt for implementing agent:**

> Read `crates/roko-cli/src/chat.rs` for the current chat implementation. Modify it to support `--agent <id>` that connects to a running persistent agent. Messages become `TaskEnvelope` entries injected via `dispatch_task()`. Responses come through the event fabric. Fall back to current behavior when `--agent` is not specified.

**Acceptance:**
- `roko chat` still works as before (backwards compatible)
- `roko agent start --profile coding` in one terminal, `roko chat --agent <id>` in another, messages flow

---

## Phase 5: Integration tests

---

### Task 26: Full lifecycle integration test

- [ ] Create `crates/roko-runtime/tests/lifecycle.rs`

**What to do:**

Write a test that exercises the complete agent lifecycle:

```rust
#[tokio::test]
async fn agent_full_lifecycle() {
    // 1. Create agent in Provisioning phase
    // 2. Add a mock extension that records hook calls
    // 3. Activate (verify on_start called)
    // 4. Run 3 ticks (verify observe/analyze/reflect called)
    // 5. Enter dream (verify phase transition)
    // 6. Wake (verify back to Active)
    // 7. Suspend (verify phase transition)
    // 8. Resume (verify back to Active)
    // 9. Terminate (verify on_stop called)
}
```

**Acceptance:**
- Test passes
- Mock extension records all hook invocations in order

---

### Task 27: Domain profile integration test

- [ ] Create `crates/roko-runtime/tests/domain_profile.rs`

**What to do:**

Test that `DomainProfile::blockchain()` produces an agent that ticks at Gamma frequency (5s) and selects T0 for low-PE ticks:

```rust
#[tokio::test]
async fn blockchain_profile_t0_ticks() {
    // Spawn with blockchain profile
    // Inject a low-PE observation
    // Run one tick
    // Assert tier == T0
    // Assert tick duration < 10ms
}
```

**Acceptance:**
- Test passes
- T0 tick completes in <10ms

---

### Task 28: Type-state compile-time enforcement test

- [ ] Create `crates/roko-runtime/tests/compile_fail/` with `trybuild` tests

**What to do:**

1. Add `trybuild` to dev-dependencies.

2. Write compile-fail tests verifying that invalid phase transitions fail at compile time:

```rust
// tests/compile_fail/terminal_no_methods.rs
fn main() {
    // This should fail to compile because Terminal has no methods
    let agent: roko_runtime::agent::RuntimeAgent<roko_runtime::agent::Terminal> = todo!();
    agent.id(); // ERROR: no method named `id` found
}
```

**Acceptance:**
- `cargo test -p roko-runtime` passes (trybuild verifies expected compilation errors)
- `Terminal.id()` fails to compile
- `Provisioning.enter_dream()` fails to compile

---

### Task 29: Extension chain ordering test

- [ ] Create `crates/roko-runtime/tests/chain_ordering.rs`

**What to do:**

Create 4 mock extensions at different layers. Validate that the chain fires hooks in layer order:

```rust
#[tokio::test]
async fn extensions_fire_in_layer_order() {
    // Create extensions at Recovery, Foundation, Action, Knowledge layers
    // Add to chain in random order
    // Validate and run observe
    // Assert invocation order: Foundation, Knowledge, Action, Recovery
}
```

**Acceptance:**
- Test passes
- Extensions always fire Foundation -> Knowledge -> ... -> Recovery regardless of registration order

---

### Task 30: CorticalState concurrent access test

- [ ] Create `crates/roko-runtime/tests/cortical_concurrent.rs`

**What to do:**

Spawn multiple tokio tasks that read and write CorticalState concurrently. Verify no panics, no data corruption, and snapshot consistency:

```rust
#[tokio::test]
async fn cortical_concurrent_reads_writes() {
    let state = Arc::new(CorticalState::default());
    let mut handles = Vec::new();

    // Spawn 10 writers
    for i in 0..10 {
        let s = state.clone();
        handles.push(tokio::spawn(async move {
            for j in 0..1000 {
                s.set_prediction_accuracy((i as f32 + j as f32) / 10000.0);
                s.set_regime(Regime::from_u8((j % 4) as u8));
                s.set_behavioral_state(BehavioralState::from_u8((j % 6) as u8));
            }
        }));
    }

    // Spawn 10 readers
    for _ in 0..10 {
        let s = state.clone();
        handles.push(tokio::spawn(async move {
            for _ in 0..1000 {
                let snap = s.snapshot();
                // Assert fields are in valid ranges
                assert!(snap.aggregate_accuracy >= 0.0 && snap.aggregate_accuracy <= 1.0);
            }
        }));
    }

    for h in handles {
        h.await.unwrap();
    }
}
```

**Acceptance:**
- Test passes with no panics
- All snapshot values are within valid ranges
- Runs in <1s

---

## 5-Phase migration plan

This plan overlaps two parallel workstreams (Stream A: wiring existing code, Stream B: new capabilities) with go/no-go gates between phases. Backwards compatibility is non-negotiable: `roko plan run` must work throughout.

### Phase M1 (Weeks 1-4): Wire existing HDC code

Wire the episode clustering, resonance detection, and somatic marker subsystems that already compile and have unit tests into the orchestration loop. No new types -- only call sites.

- Episode clustering triggers after 50 episodes (see IMPL-04 Task 1.1)
- Resonance detection feeds Lotka-Volterra dynamics (see IMPL-04 Task 1.2)
- Somatic markers record gate verdicts (see IMPL-04 Task 1.3)
- Neuro store queries run at dispatch time (see IMPL-04 Task 1.4)

**Go/no-go gate:** `cargo test --workspace` passes. Episode clustering produces `.roko/learn/clusters.json` on a 50-task plan.

### Phase M2 (Weeks 2-6): Build AgentRuntime stub + Extension trait + HeartbeatPipeline

Create the foundational types in `roko-runtime` (Tasks 1-10 of this plan). Stream A and Stream B run in parallel starting at Week 2.

- `CognitiveTier`, `ExtensionLayer`, `Extension` trait (Tasks 1-3)
- `ExtensionChain` with layer-ordered dispatch (Task 4)
- `CorticalState` pipeline fields (Task 5)
- `DomainProfile`, `RuntimeAgent`, `HeartbeatPipeline` (Tasks 6-8)
- `RuntimeEvent` types and `EventFabric` (Tasks 9-10)

**Go/no-go gate:** `cargo test -p roko-runtime` passes. `use roko_runtime::Extension;` resolves from `roko-cli`.

### Phase M3 (Weeks 4-8): Extract core extensions from orchestrate.rs

Build the extension crate scaffold (`roko-ext-core`) and extract the first 5 extensions from `orchestrate.rs`.

- DaimonExt: affect modulation, somatic markers (Task 12)
- LearningExt: episode logging, skill extraction, threshold adaptation (Task 13)
- ConductorExt: 10 watchers, circuit breaker, Yerkes-Dodson (Task 14)
- ContextExt: prompt assembly, attention bidders, VCG allocation (Task 15)
- EnergyExt: energy pool, cognitive metabolism (Task 16)

**Go/no-go gate:** Each extension passes its own test suite. All 5 extensions register and validate in an `ExtensionChain`.

### Phase M4 (Weeks 6-10): Rewrite PlanRunner to use AgentRuntime

Migrate `PlanRunner` from direct method calls to the extension pipeline. Feature-gated: `use_extension_runtime = true` in `roko.toml` enables the new path; `false` keeps the existing monolith.

- `spawn_agent` creates a `RuntimeAgent` with domain-appropriate extensions (Task 20)
- `dispatch_task` drives the HeartbeatPipeline (Task 21)
- `PlanRunner::run_plan` routes through the new agent system (Tasks 22-23)
- Bridge adapters let old code call new extensions and vice versa (Task 24)
- Configuration toggle in `roko.toml` (Task 25)

**Go/no-go gate:** `roko plan run plans/ --use-extension-runtime` produces identical results to `roko plan run plans/`. Side-by-side comparison of `.roko/episodes.jsonl` shows matching gate verdicts and episode counts.

### Phase M5 (Weeks 8-12): Integration testing, cleanup, documentation

- Full lifecycle integration test (Task 26)
- Domain profile integration test (Task 27)
- Type-state compile-time enforcement test (Task 28)
- Extension chain ordering test (Task 29)
- CorticalState concurrent access test (Task 30)
- Remove backwards-compat bridge adapters once verified
- Documentation: update CLAUDE.md, add architecture diagram

**Go/no-go gate:** All 30 tasks pass. `cargo test --workspace` passes. `cargo clippy --workspace --no-deps -- -D warnings` passes. Old monolith code paths marked `#[deprecated]`.

### 3 parallel workstreams

| Stream | Tasks | Weeks | Dependencies |
|--------|-------|-------|-------------|
| **A: Wire existing** | IMPL-04 Tasks 1.1-1.4, IMPL-01 Tasks 1-5 | 1-6 | None |
| **B: New capabilities** | IMPL-01 Tasks 6-16, domain extension crates | 2-8 | Tasks 1-3 |
| **C: Migration** | IMPL-01 Tasks 20-30 | 6-12 | Streams A + B |

Stream A and Stream B run in parallel. Stream C starts once both produce stable output.

---

## Inference gateway

The inference gateway routes LLM calls based on intent, caches responses at 3 tiers, and translates between provider formats. It sits between the `HeartbeatPipeline` and the agent dispatcher.

### Task IG-1: Define `InferenceIntent` struct

**File to create:** `crates/roko-runtime/src/inference_gateway.rs`

**Read first:**
- `crates/roko-primitives/src/tier.rs` -- `InferenceTier`, `TierRouter`
- `crates/roko-learn/src/cascade_router.rs` -- existing model routing
- `crates/roko-agent/src/dispatcher/mod.rs` -- current dispatch path

**What to do:**

1. Define the intent struct:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceIntent {
    /// Preferred model class (e.g., "claude-opus-4-6", "claude-haiku-4-5").
    pub model: Option<String>,
    /// Minimum quality level: 0.0 (fast) to 1.0 (best).
    pub quality: f64,
    /// Maximum acceptable latency in milliseconds.
    pub latency_ms: u64,
    /// Cost sensitivity: 0.0 (no concern) to 1.0 (minimize cost).
    pub cost_sensitivity: f64,
    /// Which subsystem is making the request.
    pub subsystem: String,
    /// SHA-256 hash of the prompt for exact-match caching.
    pub prompt_hash: [u8; 32],
    /// Embedding vector for similarity caching (from HDC).
    pub prompt_embedding: Option<Vec<f32>>,
}
```

2. Wire into `lib.rs`: `pub mod inference_gateway;`

**Test:** Unit test: construct an `InferenceIntent`, serialize/deserialize round-trip.

- [ ] `InferenceIntent` struct defined with all fields
- [ ] Module registered in `lib.rs`
- [ ] Serde round-trip test passes

---

### Task IG-2: Implement L3 cache (SHA-256 exact match)

**File to modify:** `crates/roko-runtime/src/inference_gateway.rs`

**Read first:**
- Task IG-1 output

**What to do:**

1. Define `L3Cache`:

```rust
pub struct L3Cache {
    entries: HashMap<[u8; 32], CachedResponse>,
    max_entries: usize,
    ttl: Duration,
}

pub struct CachedResponse {
    pub response: String,
    pub model: String,
    pub cached_at: Instant,
    pub hit_count: u64,
}
```

2. Implement `L3Cache::get(&mut self, hash: &[u8; 32]) -> Option<&CachedResponse>`: return entry if within TTL.
3. Implement `L3Cache::put(&mut self, hash: [u8; 32], response: CachedResponse)`: insert, evict oldest if at capacity.
4. Implement `L3Cache::hit_rate(&self) -> f64`: track hits vs misses.

**Test:**
- Insert a response, retrieve by hash -> hit.
- Retrieve a missing hash -> miss.
- After TTL expires -> miss (entry evicted on next get).
- Cache hit rate tracks correctly.

- [ ] L3 cache stores exact-match responses keyed by SHA-256
- [ ] TTL expiry works
- [ ] LRU eviction at capacity
- [ ] Hit rate tracking

---

### Task IG-3: Implement L2 cache (embedding similarity >0.92)

**File to modify:** `crates/roko-runtime/src/inference_gateway.rs`

**Read first:**
- Task IG-2 output
- `crates/roko-primitives/src/hdc.rs` -- `HdcVector::similarity()`

**What to do:**

1. Define `L2Cache`:

```rust
pub struct L2Cache {
    entries: Vec<(Vec<f32>, CachedResponse)>,
    similarity_threshold: f64,
    max_entries: usize,
}
```

2. Implement `L2Cache::get(&self, embedding: &[f32]) -> Option<&CachedResponse>`: brute-force scan, return first entry with cosine similarity > `similarity_threshold` (default 0.92).
3. Implement `L2Cache::put(&mut self, embedding: Vec<f32>, response: CachedResponse)`.
4. If entry count exceeds `max_entries`, evict lowest-hit-count entry.

**Test:**
- Insert a response with embedding A. Query with embedding A' (similarity 0.95) -> hit.
- Query with embedding B (similarity 0.50) -> miss.
- Cache hit rate >30% on a workload of repeated similar prompts.

- [ ] L2 cache stores responses keyed by embedding vectors
- [ ] Similarity threshold configurable (default 0.92)
- [ ] Hit rate >30% on repeated-similar workloads

---

### Task IG-4: Implement L1 cache (prefix alignment for provider KV)

**File to modify:** `crates/roko-runtime/src/inference_gateway.rs`

**Read first:**
- Task IG-3 output
- Anthropic API documentation on prompt caching

**What to do:**

1. Define `L1Cache` -- this is not a response cache but a prefix-alignment cache that maximizes KV cache hits at the provider level.

```rust
pub struct L1Cache {
    /// Canonical prefix per role+domain. Prompts sharing this prefix
    /// benefit from provider-side KV cache reuse.
    canonical_prefixes: HashMap<String, String>,
    /// Track which prefixes are actively cached.
    active_prefixes: HashSet<String>,
}
```

2. Implement `L1Cache::align_prompt(&self, role: &str, prompt: &str) -> String`: rewrite prompt to start with the canonical prefix for this role, maximizing the shared prefix length.
3. Implement `L1Cache::register_prefix(&mut self, role: &str, prefix: String)`.
4. Track prefix reuse rate.

**Test:**
- Register a prefix for "coding" role. Align a prompt -> starts with the registered prefix.
- Prefix reuse rate >90% on a workload of coding tasks with the same system prompt.

- [ ] L1 prefix alignment maximizes provider KV cache hits
- [ ] Prefix reuse rate >90% on repeated tasks

---

### Task IG-5: Implement intent-based provider routing

**File to modify:** `crates/roko-runtime/src/inference_gateway.rs`

**Read first:**
- Tasks IG-1 through IG-4
- `crates/roko-learn/src/cascade_router.rs` -- existing CascadeRouter

**What to do:**

1. Define `InferenceGateway`:

```rust
pub struct InferenceGateway {
    l1: L1Cache,
    l2: L2Cache,
    l3: L3Cache,
    providers: Vec<ProviderConfig>,
}

pub struct ProviderConfig {
    pub name: String,
    pub model: String,
    pub quality: f64,
    pub latency_p50_ms: u64,
    pub cost_per_1k_tokens: f64,
}
```

2. Implement `InferenceGateway::route(&self, intent: &InferenceIntent) -> &ProviderConfig`: first-match-wins against provider list filtered by intent constraints (quality, latency, cost).
3. Implement `InferenceGateway::resolve(&mut self, intent: &InferenceIntent) -> Result<String>`:
   - Check L3 (exact hash) -> return cached if hit
   - Check L2 (embedding similarity) -> return cached if hit
   - Align prompt via L1
   - Route to provider
   - Call provider (via existing dispatcher)
   - Store result in L3 and L2
   - Return response

**Test:**
- High-quality intent routes to Opus provider.
- Low-latency intent routes to Haiku provider.
- Second identical request hits L3 cache.
- Similar-but-not-identical request hits L2 cache.

- [ ] Intent-based routing selects provider by quality/latency/cost
- [ ] 3-tier cache lookup (L3 -> L2 -> L1 alignment -> provider)
- [ ] First-match-wins routing

---

### Task IG-6: Implement Translator trait

**File to modify:** `crates/roko-runtime/src/inference_gateway.rs`

**Read first:**
- `crates/roko-agent/src/dispatcher/mod.rs` -- existing dispatch formats
- Task IG-5 output

**What to do:**

1. Define the translator:

```rust
pub trait Translator: Send + Sync {
    fn format(&self) -> TranslationFormat;
    fn encode(&self, prompt: &str, system: &str) -> serde_json::Value;
    fn decode(&self, response: &serde_json::Value) -> Result<String>;
}

#[derive(Debug, Clone, Copy)]
pub enum TranslationFormat {
    AnthropicBlocks,
    OpenAiJson,
    GeminiNative,
    ReActText,
}
```

2. Implement `AnthropicBlocksTranslator`: encode to Anthropic message format with system block.
3. Implement `OpenAiJsonTranslator`: encode to OpenAI chat completion format.
4. Implement `GeminiNativeTranslator`: encode to Gemini content format.
5. Implement `ReActTextTranslator`: encode to plain text ReAct format (for local/Ollama models).

**Test:**
- Each translator encodes a prompt and decodes a mock response without error.
- Anthropic format includes `role: "system"` block.
- OpenAI format includes `messages` array.

- [ ] `Translator` trait with 4 implementations
- [ ] Encode/decode round-trip for each format
- [ ] Formats: AnthropicBlocks, OpenAiJson, GeminiNative, ReActText

---

### Task IG-7: Cache performance validation

**File to create:** `crates/roko-runtime/tests/inference_cache.rs`

**Read first:**
- Tasks IG-2 through IG-5

**What to do:**

1. Generate a workload of 200 inference requests:
   - 50 exact duplicates (should hit L3)
   - 80 similar prompts (should hit L2 at >0.92 similarity)
   - 70 unique prompts (cache miss)
2. Assert: L3 hit rate >10% (exact match on the 50 duplicates after first miss)
3. Assert: L2 hit rate >30% (similar prompts hit after first member cached)
4. Assert: L1 prefix reuse rate >90% (same role prefixes)
5. Assert: total cache hit rate reduces provider calls by >40%

**Test:** `cargo test -p roko-runtime --test inference_cache`

- [ ] L3 hit rate >10% on repeated tasks
- [ ] L2 hit rate >30% on similar tasks
- [ ] L1 prefix reuse >90%
- [ ] Total provider call reduction >40%

---

## Overall acceptance criteria

- [ ] `cargo test -p roko-runtime --lib` passes with all new runtime tests
- [ ] `cargo test -p roko-ext-core --lib` passes
- [ ] `cargo test -p roko-ext-code --lib` passes
- [ ] `cargo test -p roko-ext-chain --lib` passes
- [ ] `cargo test -p roko-ext-research --lib` passes
- [ ] End-to-end: `roko agent start --profile coding` spawns a persistent agent that prints tick summaries
- [ ] End-to-end: `roko plan run plans/` still works with `use_extension_runtime = false` (backwards compat)
- [ ] End-to-end: `roko plan run plans/` works with `use_extension_runtime = true` (new path)
- [ ] Heartbeat tick completes in <10ms for T0 (no LLM call)
- [ ] CorticalState concurrent read/write has zero contention (verified via Task 30 test)
- [ ] All workspace tests pass: `cargo test --workspace`
- [ ] Clippy clean: `cargo clippy --workspace --no-deps -- -D warnings`
- [ ] Format clean: `cargo +nightly fmt --all --check`

---

## Dependency graph

```
Task 1  (CognitiveTier)
Task 2  (ExtensionLayer)       ← depends on Task 1
Task 3  (Extension trait)      ← depends on Tasks 1, 2
Task 4  (ExtensionChain)       ← depends on Task 3
Task 5  (CorticalState fields) ← independent (modifies existing)
Task 6  (DomainProfile)        ← independent
Task 7  (RuntimeAgent)         ← depends on Tasks 3, 4, 6
Task 8  (HeartbeatPipeline)    ← depends on Tasks 3, 4
Task 9  (RuntimeEvent types)   ← depends on Task 2
Task 10 (EventFabric)          ← depends on Task 9

Task 11 (roko-ext-core scaffold) ← depends on Tasks 1-10
Tasks 12-16 (core extensions)    ← depend on Task 11

Tasks 17-19 (domain extension crates) ← depend on Tasks 1-10

Task 20 (spawn_agent)      ← depends on Tasks 6, 7
Task 21 (dispatch_task)    ← depends on Tasks 7, 8
Task 22 (spawn_and_run_task) ← depends on Tasks 20, 21
Task 23 (PlanRunner migration) ← depends on Task 22
Task 24 (roko agent start)    ← depends on Task 20
Task 25 (roko chat integration) ← depends on Tasks 21, 24

Tasks 26-30 (integration tests) ← depend on all previous tasks
```

**Suggested execution order for parallelism:**

| Wave | Tasks | Why |
|------|-------|-----|
| 1 | 1, 2, 5, 6 | Independent foundation types |
| 2 | 3, 9 | Depend on wave 1 |
| 3 | 4, 7, 8, 10 | Depend on wave 2 |
| 4 | 11 | Crate scaffold |
| 5 | 12, 13, 14, 15, 16, 17, 18, 19 | All extensions in parallel |
| 6 | 20, 21 | Spawn + dispatch |
| 7 | 22 | Combines 20 + 21 |
| 8 | 23, 24, 25 | CLI integration |
| 9 | 26, 27, 28, 29, 30 | Integration tests |
