//! Cognitive loop Cell implementations.
//!
//! The cognitive loop is the core execution cycle: sense -> assess -> compose ->
//! act -> verify -> persist -> react. Each step is a typed Cell with declared
//! I/O schemas and protocol conformances.
//!
//! These Cells replace the `PassthroughCell` stubs for cognitive loop nodes.
//! The full execution logic is deferred -- for now each Cell passes input
//! through and logs its invocation (same behavior as PassthroughCell, but with
//! proper typing and protocol declarations).

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use roko_core::{Engram, Kind, ProtocolId, TypeSchema, error::Result};

use crate::cell::{Cell, CellContext, CellVersion};

// ─── SenseCell ───────────────────────────────────────────────────────────────

/// Reads signals from Store and pulses from Bus; detects whether a full
/// cognitive tick is needed or if T0 short-circuit applies.
///
/// Protocol: `Observe` (passive signal observation).
pub struct SenseCell {
    /// Monotonic counter for T0 short-circuit ticks (observability).
    pub t0_counter: AtomicU64,
    /// Output type schema: produces agent messages (sensed material).
    output_schema: TypeSchema,
}

impl SenseCell {
    /// Create a new SenseCell.
    #[must_use]
    pub fn new() -> Self {
        Self {
            t0_counter: AtomicU64::new(0),
            output_schema: TypeSchema::OfKind(Kind::AgentMessage),
        }
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

    async fn execute(&self, input: Vec<Engram>, _ctx: &CellContext) -> Result<Vec<Engram>> {
        tracing::trace!(cell = "sense", input_count = input.len(), "SenseCell tick");
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
}

impl AssessCell {
    /// Create a new AssessCell.
    #[must_use]
    pub fn new() -> Self {
        Self {
            input_schema: TypeSchema::OfKind(Kind::AgentMessage),
            output_schema: TypeSchema::OfKind(Kind::AgentMessage),
        }
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

    async fn execute(&self, input: Vec<Engram>, _ctx: &CellContext) -> Result<Vec<Engram>> {
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

    async fn execute(&self, input: Vec<Engram>, _ctx: &CellContext) -> Result<Vec<Engram>> {
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

    async fn execute(&self, input: Vec<Engram>, _ctx: &CellContext) -> Result<Vec<Engram>> {
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
        Some(Duration::from_secs(60))
    }
    fn input_schema(&self) -> Option<&TypeSchema> {
        Some(&self.input_schema)
    }
    fn output_schema(&self) -> Option<&TypeSchema> {
        Some(&self.output_schema)
    }

    async fn execute(&self, input: Vec<Engram>, _ctx: &CellContext) -> Result<Vec<Engram>> {
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

    async fn execute(&self, input: Vec<Engram>, _ctx: &CellContext) -> Result<Vec<Engram>> {
        tracing::trace!(
            cell = "persist",
            input_count = input.len(),
            "PersistCell tick"
        );
        Ok(input)
    }
}

// ─── ReactCell ───────────────────────────────────────────────────────────────

/// Emits events and applies reactive policies (feedback, alarms, replan).
/// Always runs, even during T0 short-circuit ticks.
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

    async fn execute(&self, input: Vec<Engram>, _ctx: &CellContext) -> Result<Vec<Engram>> {
        self.tick_counter.fetch_add(1, Ordering::Relaxed);
        tracing::trace!(cell = "react", input_count = input.len(), "ReactCell tick");
        Ok(input)
    }
}
