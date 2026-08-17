//! Cognitive loop Cell implementations for the Hot Graph execution model.
//!
//! The v2 cognitive loop is a Hot Graph with 7 typed Cells executing in sequence
//! each tick: Sense -> Assess -> Compose -> Act -> Verify -> Persist -> React.
//!
//! ## T0 Short-Circuit
//!
//! The most common case (~80% of ticks) is that nothing changed between ticks:
//! no new Signals, no new Pulses, no external input. In this case, SenseCell
//! emits an output Signal tagged with `"t0_short_circuit" = "true"`, which the
//! conditional edge in the graph definition uses to skip the expensive middle
//! cells (Assess -> Compose -> Act -> Verify -> Persist) and jump directly to React.
//!
//! ReactCell always runs. It handles maintenance tasks, event emission, and
//! housekeeping that should occur every tick regardless of whether a full
//! cognitive pass was needed.
//!
//! ## Short-Circuit Inhibition
//!
//! The short-circuit is suppressed (forced full tick) when:
//! - `budget_remaining` in `CellContext` is below `DEADLINE_BUDGET_THRESHOLD_USD`
//!   (approaching deadline -> run a full pass to flush pending work), OR
//! - The previous ReactCell output included a `"force_full_tick" = "true"` tag
//!   (React requested a follow-up full pass in the next iteration).

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use roko_core::{Body, Kind, PredictionRecord, ProtocolId, Signal, TypeSchema, error::Result};
use tracing::trace;

use crate::cell::{Cell, CellContext, CellVersion};

/// Budget threshold below which a T0 short-circuit is suppressed.
///
/// When `budget_remaining` drops below this value we force a full tick to
/// ensure pending work is flushed before the budget runs out.
const DEADLINE_BUDGET_THRESHOLD_USD: f64 = 0.05;

// ─── SenseCell ───────────────────────────────────────────────────────────────

/// Reads signals from Store and pulses from Bus; detects whether a full
/// cognitive tick is needed or if T0 short-circuit applies.
///
/// ## T0 Short-Circuit
///
/// If no new Signals or Pulses are available AND the tick is not deadline-
/// proximate AND no forced full tick was requested, SenseCell emits a single
/// Signal tagged with `"t0_short_circuit" = "true"`. Downstream conditional
/// edges route this directly to ReactCell, skipping the middle cognitive cells.
///
/// The `t0_count` atomic counter is incremented on every short-circuit tick
/// for observability via metrics.
///
/// Protocol: `Observe` (passive signal observation).
pub struct SenseCell {
    /// Monotonic count of T0 short-circuit ticks emitted.
    pub t0_count: Arc<AtomicU64>,
    /// Output type schema: produces agent messages (sensed material).
    output_schema: TypeSchema,
}

impl SenseCell {
    /// Create a new SenseCell with a fresh T0 counter.
    #[must_use]
    pub fn new() -> Self {
        Self {
            t0_count: Arc::new(AtomicU64::new(0)),
            output_schema: TypeSchema::OfKind(Kind::AgentMessage),
        }
    }

    /// Return the number of T0 short-circuit ticks emitted by this cell.
    #[must_use]
    pub fn t0_count(&self) -> u64 {
        self.t0_count.load(Ordering::Relaxed)
    }

    /// Determine whether a T0 short-circuit should fire for this tick.
    ///
    /// Returns `true` when all of the following hold:
    /// - The input signals are empty (no new Signals or Pulses were queued).
    /// - Budget is not approaching the deadline threshold.
    /// - No upstream signal carries a `"force_full_tick" = "true"` tag.
    fn should_short_circuit(input: &[Signal], ctx: &CellContext) -> bool {
        // Condition 1: no actionable input.
        if !input.is_empty() {
            return false;
        }

        // Condition 2: not deadline-proximate.
        if let Some(remaining) = ctx.budget_remaining
            && remaining < DEADLINE_BUDGET_THRESHOLD_USD
        {
            // Budget nearly exhausted -- run a full tick to flush pending work.
            return false;
        }

        // Condition 3: React did not request a forced full tick.
        let forced = input
            .iter()
            .any(|e| e.tags.get("force_full_tick").map(String::as_str) == Some("true"));
        if forced {
            return false;
        }

        true
    }
}

impl Default for SenseCell {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Cell for SenseCell {
    fn cell_id(&self) -> &'static str {
        "sense"
    }
    fn cell_name(&self) -> &'static str {
        "SenseCell"
    }
    fn cell_version(&self) -> CellVersion {
        (0, 1, 0)
    }
    fn protocols(&self) -> Vec<ProtocolId> {
        vec![ProtocolId::Observe]
    }
    fn estimated_cost(&self) -> Option<f64> {
        Some(0.0)
    }
    fn estimated_duration(&self) -> Option<Duration> {
        Some(Duration::from_millis(5))
    }
    fn output_schema(&self) -> Option<&TypeSchema> {
        Some(&self.output_schema)
    }

    /// Execute the Sense phase.
    ///
    /// When the T0 short-circuit condition is met (no new input, not deadline-
    /// proximate, no forced full tick), returns a single Signal tagged with
    /// `"t0_short_circuit" = "true"` and increments the T0 counter.
    ///
    /// Otherwise returns the input signals as-is (a real implementation would
    /// read from the Store and Bus and attach sensed material here).
    async fn execute(&self, input: Vec<Signal>, ctx: &CellContext) -> Result<Vec<Signal>> {
        if Self::should_short_circuit(&input, ctx) {
            self.t0_count.fetch_add(1, Ordering::Relaxed);
            trace!(
                t0_count = self.t0_count.load(Ordering::Relaxed),
                "SenseCell: T0 short-circuit -- no actionable input this tick"
            );

            let marker = Signal::builder(Kind::Custom("sense.tick".into()))
                .body(Body::text("t0: no actionable input"))
                .tag("t0_short_circuit", "true")
                .build();

            return Ok(vec![marker]);
        }

        // Full tick: pass through whatever was queued as sensed material.
        // A real implementation would query the Signal Store and Bus here.
        trace!(
            input_count = input.len(),
            "SenseCell: full tick -- {} input signals",
            input.len()
        );
        Ok(input)
    }
}

// ─── AssessCell ──────────────────────────────────────────────────────────────

/// Scores sensed signals for relevance and priority.
///
/// Protocol: `Score` (relevance scoring).
pub struct AssessCell {
    /// Input type schema: receives agent messages from SenseCell.
    input_schema: TypeSchema,
    /// Output type schema: produces scored agent messages.
    output_schema: TypeSchema,
    /// Number of completed predict-correct cycles received by this instance.
    calibration_observations: AtomicU64,
    /// Cumulative normalized prediction error stored as millionths.
    calibration_error_micros: AtomicU64,
}

impl AssessCell {
    /// Create a new AssessCell.
    #[must_use]
    pub fn new() -> Self {
        Self {
            input_schema: TypeSchema::OfKind(Kind::AgentMessage),
            output_schema: TypeSchema::OfKind(Kind::AgentMessage),
            calibration_observations: AtomicU64::new(0),
            calibration_error_micros: AtomicU64::new(0),
        }
    }

    /// Return the number of prediction corrections received by this instance.
    #[must_use]
    pub fn calibration_observations(&self) -> u64 {
        self.calibration_observations.load(Ordering::Relaxed)
    }

    fn output_count_error(prediction: &PredictionRecord, actual: &[Signal]) -> Option<f64> {
        let expected = prediction.predicted_outcome.get("output_count")?.as_u64()? as f64;
        let observed = actual.len() as f64;
        Some((expected - observed).abs() / expected.max(observed).max(1.0))
    }
}

impl Default for AssessCell {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Cell for AssessCell {
    fn cell_id(&self) -> &'static str {
        "assess"
    }
    fn cell_name(&self) -> &'static str {
        "AssessCell"
    }
    fn cell_version(&self) -> CellVersion {
        (0, 1, 0)
    }
    fn protocols(&self) -> Vec<ProtocolId> {
        vec![ProtocolId::Score]
    }
    fn estimated_cost(&self) -> Option<f64> {
        Some(0.001)
    }
    fn estimated_duration(&self) -> Option<Duration> {
        Some(Duration::from_millis(10))
    }
    fn input_schema(&self) -> Option<&TypeSchema> {
        Some(&self.input_schema)
    }
    fn output_schema(&self) -> Option<&TypeSchema> {
        Some(&self.output_schema)
    }

    fn predict(&self, input: &[Signal]) -> Option<PredictionRecord> {
        Some(PredictionRecord {
            cell_id: self.cell_id().to_string(),
            predicted_outcome: serde_json::json!({"output_count": input.len()}),
            confidence: 1.0,
            timestamp_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |duration| {
                    duration.as_millis().try_into().unwrap_or(i64::MAX)
                }),
        })
    }

    fn calibration_error(&self, prediction: &PredictionRecord, actual: &[Signal]) -> Option<f64> {
        Self::output_count_error(prediction, actual)
    }

    fn correct(&self, prediction: &PredictionRecord, actual: &[Signal]) {
        if let Some(error) = Self::output_count_error(prediction, actual) {
            self.calibration_observations
                .fetch_add(1, Ordering::Relaxed);
            self.calibration_error_micros.fetch_add(
                (error.clamp(0.0, 1.0) * 1_000_000.0).round() as u64,
                Ordering::Relaxed,
            );
        }
    }

    async fn execute(&self, input: Vec<Signal>, _ctx: &CellContext) -> Result<Vec<Signal>> {
        tracing::trace!(
            cell = "assess",
            input_count = input.len(),
            "AssessCell tick"
        );
        Ok(input)
    }
}

// ─── CognitiveComposeCell ────────────────────────────────────────────────────

/// Assembles the system prompt from scored signals and context.
///
/// Protocol: `Compose` (prompt assembly).
///
/// Named `CognitiveComposeCell` to avoid collision with `cells::compose::ComposeCell`.
pub struct CognitiveComposeCell {
    /// Input type schema: receives scored messages from AssessCell.
    input_schema: TypeSchema,
    /// Output type schema: produces a fully assembled prompt.
    output_schema: TypeSchema,
}

impl CognitiveComposeCell {
    /// Create a new CognitiveComposeCell.
    #[must_use]
    pub fn new() -> Self {
        Self {
            input_schema: TypeSchema::OfKind(Kind::AgentMessage),
            output_schema: TypeSchema::OfKind(Kind::Prompt),
        }
    }
}

impl Default for CognitiveComposeCell {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Cell for CognitiveComposeCell {
    fn cell_id(&self) -> &'static str {
        "compose"
    }
    fn cell_name(&self) -> &'static str {
        "CognitiveComposeCell"
    }
    fn cell_version(&self) -> CellVersion {
        (0, 1, 0)
    }
    fn protocols(&self) -> Vec<ProtocolId> {
        vec![ProtocolId::Compose]
    }
    fn estimated_cost(&self) -> Option<f64> {
        Some(0.005)
    }
    fn estimated_duration(&self) -> Option<Duration> {
        Some(Duration::from_millis(50))
    }
    fn input_schema(&self) -> Option<&TypeSchema> {
        Some(&self.input_schema)
    }
    fn output_schema(&self) -> Option<&TypeSchema> {
        Some(&self.output_schema)
    }

    async fn execute(&self, input: Vec<Signal>, _ctx: &CellContext) -> Result<Vec<Signal>> {
        tracing::trace!(
            cell = "compose",
            input_count = input.len(),
            "CognitiveComposeCell tick"
        );
        Ok(input)
    }
}

// ─── ActCell ─────────────────────────────────────────────────────────────────

/// Dispatches the composed prompt to an LLM agent and collects the response.
///
/// Protocol: `Connect` (external agent dispatch).
pub struct ActCell {
    /// Input type schema: receives a prompt from CognitiveComposeCell.
    input_schema: TypeSchema,
    /// Output type schema: produces an episode record.
    output_schema: TypeSchema,
}

impl ActCell {
    /// Create a new ActCell.
    #[must_use]
    pub fn new() -> Self {
        Self {
            input_schema: TypeSchema::OfKind(Kind::Prompt),
            output_schema: TypeSchema::OfKind(Kind::Episode),
        }
    }
}

impl Default for ActCell {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Cell for ActCell {
    fn cell_id(&self) -> &'static str {
        "act"
    }
    fn cell_name(&self) -> &'static str {
        "ActCell"
    }
    fn cell_version(&self) -> CellVersion {
        (0, 1, 0)
    }
    fn protocols(&self) -> Vec<ProtocolId> {
        vec![ProtocolId::Connect]
    }
    fn estimated_cost(&self) -> Option<f64> {
        Some(0.10)
    }
    fn estimated_duration(&self) -> Option<Duration> {
        Some(Duration::from_secs(30))
    }
    fn input_schema(&self) -> Option<&TypeSchema> {
        Some(&self.input_schema)
    }
    fn output_schema(&self) -> Option<&TypeSchema> {
        Some(&self.output_schema)
    }

    async fn execute(&self, input: Vec<Signal>, _ctx: &CellContext) -> Result<Vec<Signal>> {
        tracing::trace!(cell = "act", input_count = input.len(), "ActCell tick");
        Ok(input)
    }
}

// ─── VerifyCell ──────────────────────────────────────────────────────────────

/// Executes the gate pipeline (compile, test, clippy, diff) against the
/// agent's output.
///
/// Protocol: `Verify` (gate execution).
pub struct VerifyCell {
    /// Input type schema: receives an episode from ActCell.
    input_schema: TypeSchema,
    /// Output type schema: produces gate verdict.
    output_schema: TypeSchema,
}

impl VerifyCell {
    /// Create a new VerifyCell.
    #[must_use]
    pub fn new() -> Self {
        Self {
            input_schema: TypeSchema::OfKind(Kind::Episode),
            output_schema: TypeSchema::OfKind(Kind::GateVerdict),
        }
    }
}

impl Default for VerifyCell {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Cell for VerifyCell {
    fn cell_id(&self) -> &'static str {
        "verify"
    }
    fn cell_name(&self) -> &'static str {
        "VerifyCell"
    }
    fn cell_version(&self) -> CellVersion {
        (0, 1, 0)
    }
    fn protocols(&self) -> Vec<ProtocolId> {
        vec![ProtocolId::Verify]
    }
    fn estimated_cost(&self) -> Option<f64> {
        Some(0.0)
    }
    fn estimated_duration(&self) -> Option<Duration> {
        Some(Duration::from_mins(1))
    }
    fn input_schema(&self) -> Option<&TypeSchema> {
        Some(&self.input_schema)
    }
    fn output_schema(&self) -> Option<&TypeSchema> {
        Some(&self.output_schema)
    }

    async fn execute(&self, input: Vec<Signal>, _ctx: &CellContext) -> Result<Vec<Signal>> {
        tracing::trace!(
            cell = "verify",
            input_count = input.len(),
            "VerifyCell tick"
        );
        Ok(input)
    }
}

// ─── PersistCell ─────────────────────────────────────────────────────────────

/// Writes verified outputs to the durable signal store.
///
/// Protocol: `Store` (signal persistence).
pub struct PersistCell {
    /// Input type schema: receives gate verdict from VerifyCell.
    input_schema: TypeSchema,
}

impl PersistCell {
    /// Create a new PersistCell.
    #[must_use]
    pub fn new() -> Self {
        Self {
            input_schema: TypeSchema::OfKind(Kind::GateVerdict),
        }
    }
}

impl Default for PersistCell {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Cell for PersistCell {
    fn cell_id(&self) -> &'static str {
        "persist"
    }
    fn cell_name(&self) -> &'static str {
        "PersistCell"
    }
    fn cell_version(&self) -> CellVersion {
        (0, 1, 0)
    }
    fn protocols(&self) -> Vec<ProtocolId> {
        vec![ProtocolId::Store]
    }
    fn estimated_cost(&self) -> Option<f64> {
        Some(0.0)
    }
    fn estimated_duration(&self) -> Option<Duration> {
        Some(Duration::from_millis(10))
    }
    fn input_schema(&self) -> Option<&TypeSchema> {
        Some(&self.input_schema)
    }

    async fn execute(&self, input: Vec<Signal>, _ctx: &CellContext) -> Result<Vec<Signal>> {
        tracing::trace!(
            cell = "persist",
            input_count = input.len(),
            "PersistCell tick"
        );
        Ok(input)
    }
}

// ─── ReactCell ───────────────────────────────────────────────────────────────

/// Emits lifecycle events, performs housekeeping, and optionally requests a
/// forced full tick for the next iteration.
///
/// ReactCell always runs, even on T0 short-circuit ticks. It is the only cell
/// that can set `"force_full_tick" = "true"` on its output Signals to force the
/// next tick to execute the full cognitive loop regardless of sensed material.
///
/// Protocols: `React` (reactive policy) + `Trigger` (event-driven triggers).
pub struct ReactCell {
    /// Monotonic counter for total ticks processed (observability).
    pub tick_counter: AtomicU64,
}

impl ReactCell {
    /// Create a new ReactCell.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tick_counter: AtomicU64::new(0),
        }
    }
}

impl Default for ReactCell {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Cell for ReactCell {
    fn cell_id(&self) -> &'static str {
        "react"
    }
    fn cell_name(&self) -> &'static str {
        "ReactCell"
    }
    fn cell_version(&self) -> CellVersion {
        (0, 1, 0)
    }
    fn protocols(&self) -> Vec<ProtocolId> {
        vec![ProtocolId::React, ProtocolId::Trigger]
    }
    fn estimated_cost(&self) -> Option<f64> {
        Some(0.0)
    }
    fn estimated_duration(&self) -> Option<Duration> {
        Some(Duration::from_millis(5))
    }

    async fn execute(&self, input: Vec<Signal>, _ctx: &CellContext) -> Result<Vec<Signal>> {
        self.tick_counter.fetch_add(1, Ordering::Relaxed);
        tracing::trace!(cell = "react", input_count = input.len(), "ReactCell tick");
        Ok(input)
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ctx() -> CellContext {
        CellContext::new()
    }

    fn make_ctx_with_budget(budget: f64) -> CellContext {
        CellContext::new().with_budget(budget)
    }

    fn make_signal(text: &str) -> Signal {
        Signal::builder(Kind::Task).body(Body::text(text)).build()
    }

    // ── T0 short-circuit fires on empty input ────────────────────────────────

    #[tokio::test]
    async fn sense_cell_t0_short_circuit_on_empty_input() {
        let cell = SenseCell::new();
        let ctx = make_ctx();

        let output = cell.execute(vec![], &ctx).await.unwrap();

        // Must emit exactly one marker signal.
        assert_eq!(output.len(), 1, "T0 short-circuit should emit one marker");
        let marker = &output[0];
        assert_eq!(
            marker.tags.get("t0_short_circuit").map(String::as_str),
            Some("true"),
            "marker must carry t0_short_circuit=true tag"
        );
    }

    // ── T0 counter increments on each short-circuit tick ─────────────────────

    #[tokio::test]
    async fn sense_cell_t0_counter_increments() {
        let cell = SenseCell::new();
        let ctx = make_ctx();

        assert_eq!(cell.t0_count(), 0);
        let _ = cell.execute(vec![], &ctx).await.unwrap();
        assert_eq!(cell.t0_count(), 1);
        let _ = cell.execute(vec![], &ctx).await.unwrap();
        assert_eq!(cell.t0_count(), 2);
    }

    // ── T0 short-circuit suppressed when input is non-empty ──────────────────

    #[tokio::test]
    async fn sense_cell_no_short_circuit_with_input() {
        let cell = SenseCell::new();
        let ctx = make_ctx();
        let input = vec![make_signal("new signal")];

        let output = cell.execute(input, &ctx).await.unwrap();

        // With non-empty input, SenseCell passes through (no T0 marker).
        assert_eq!(output.len(), 1);
        assert_eq!(
            output[0].tags.get("t0_short_circuit"),
            None,
            "non-empty input must not produce a T0 short-circuit"
        );
        assert_eq!(
            cell.t0_count(),
            0,
            "T0 counter must not increment on full tick"
        );
    }

    // ── T0 short-circuit suppressed when budget is near deadline ─────────────

    #[tokio::test]
    async fn sense_cell_no_short_circuit_near_deadline() {
        let cell = SenseCell::new();
        // Budget below the threshold forces a full tick.
        let ctx = make_ctx_with_budget(DEADLINE_BUDGET_THRESHOLD_USD - 0.001);

        let output = cell.execute(vec![], &ctx).await.unwrap();

        // Empty output (passthrough of empty input) -- no T0 marker, no T0 count.
        assert!(
            output
                .iter()
                .all(|e| e.tags.get("t0_short_circuit").map(String::as_str) != Some("true")),
            "deadline-proximate tick must not produce a T0 short-circuit"
        );
        assert_eq!(cell.t0_count(), 0);
    }

    // ── T0 short-circuit fires when budget is above threshold ────────────────

    #[tokio::test]
    async fn sense_cell_short_circuit_with_sufficient_budget() {
        let cell = SenseCell::new();
        // Budget well above the threshold -- T0 should still fire (empty input).
        let ctx = make_ctx_with_budget(1.00);

        let output = cell.execute(vec![], &ctx).await.unwrap();

        assert_eq!(output.len(), 1);
        assert_eq!(
            output[0].tags.get("t0_short_circuit").map(String::as_str),
            Some("true")
        );
        assert_eq!(cell.t0_count(), 1);
    }

    // ── All 7 cognitive cells implement Cell correctly ────────────────────────

    #[tokio::test]
    async fn all_cognitive_cells_passthrough_input() {
        let input = vec![make_signal("test signal")];
        let ctx = make_ctx();

        // All non-Sense cells simply pass input through in stub form.
        let assess = AssessCell::new();
        let compose = CognitiveComposeCell::new();
        let act = ActCell::new();
        let verify = VerifyCell::new();
        let persist = PersistCell::new();
        let react = ReactCell::new();

        for (name, result) in [
            ("AssessCell", assess.execute(input.clone(), &ctx).await),
            (
                "CognitiveComposeCell",
                compose.execute(input.clone(), &ctx).await,
            ),
            ("ActCell", act.execute(input.clone(), &ctx).await),
            ("VerifyCell", verify.execute(input.clone(), &ctx).await),
            ("PersistCell", persist.execute(input.clone(), &ctx).await),
            ("ReactCell", react.execute(input.clone(), &ctx).await),
        ] {
            let out = result.unwrap();
            assert_eq!(out.len(), 1, "{name} must pass through its input");
        }
    }

    // ── Protocol declarations ────────────────────────────────────────────────

    #[test]
    fn cognitive_cells_declare_correct_protocols() {
        assert_eq!(SenseCell::new().protocols(), vec![ProtocolId::Observe]);
        assert_eq!(AssessCell::new().protocols(), vec![ProtocolId::Score]);
        assert_eq!(
            CognitiveComposeCell::new().protocols(),
            vec![ProtocolId::Compose]
        );
        assert_eq!(ActCell::new().protocols(), vec![ProtocolId::Connect]);
        assert_eq!(VerifyCell::new().protocols(), vec![ProtocolId::Verify]);
        assert_eq!(PersistCell::new().protocols(), vec![ProtocolId::Store]);
        assert_eq!(
            ReactCell::new().protocols(),
            vec![ProtocolId::React, ProtocolId::Trigger]
        );
    }

    #[test]
    fn assess_cell_completes_predict_correct_calibration_cycle() {
        let cell = AssessCell::new();
        let input = vec![make_signal("first"), make_signal("second")];
        let prediction = cell.predict(&input).expect("prediction");
        assert_eq!(prediction.predicted_outcome["output_count"], 2);
        assert_eq!(cell.calibration_error(&prediction, &input), Some(0.0));
        cell.correct(&prediction, &input);
        assert_eq!(cell.calibration_observations(), 1);
    }
}
