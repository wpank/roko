//! 12-row serial feedback settlement pipeline (backlog #253).
//!
//! Each completed task attempt is settled through exactly 12 sinks in the
//! fixed order defined by the spec. The pipeline distinguishes **critical**
//! sinks (rows 0-2) from **optional** sinks (rows 3-11):
//!
//! - A critical sink failure stops settlement and returns an error.
//!   The provider result is committed but the task terminal state is NOT.
//! - An optional sink failure is recorded and settlement continues.
//!   The final result is `CompletedWithDegradation` listing every failure.
//!
//! # Idempotency
//!
//! Each `(idempotency_key, sink_key)` pair has a receipt in the settlement
//! ledger. On resume, the settler skips rows that already have `Settled`
//! status. The settler never calls the provider again; it only replays
//! from the first unsettled sink row.
//!
//! # Sink implementations
//!
//! The settler is sink-agnostic: each row is a boxed [`SettlementSink`].
//! Host adapters (CLI, serve) supply concrete sink instances that delegate
//! to existing `runtime_feedback`, `roko-learn`, `daimon`, `conductor`,
//! and projection code. No replacement stores are created.

use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use super::receipt::TaskAttemptReceiptV1;

// ---------------------------------------------------------------------------
// Sink keys
// ---------------------------------------------------------------------------

/// Fixed sink keys in settlement order. The index IS the row number.
pub const SINK_KEYS: [&str; 12] = [
    "attempt_receipt",   // 0 -- critical
    "actual_cost",       // 1 -- critical
    "structured_audit",  // 2 -- critical
    "episode",           // 3 -- optional
    "efficiency",        // 4 -- optional
    "routing",           // 5 -- optional
    "error_pattern",     // 6 -- optional (failed attempts only)
    "playbook",          // 7 -- optional
    "knowledge",         // 8 -- optional
    "daimon",            // 9 -- optional
    "conductor",         // 10 -- optional
    "projection",        // 11 -- optional
];

/// Number of critical sinks (rows 0-2).
pub const CRITICAL_SINK_COUNT: usize = 3;

// ---------------------------------------------------------------------------
// Settlement ledger types
// ---------------------------------------------------------------------------

/// State of one sink settlement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SinkSettlementState {
    /// Sink processing has been prepared but not yet invoked.
    Prepared,
    /// Sink was invoked successfully and acknowledged the write.
    Settled,
    /// Sink was skipped because it was not applicable (e.g. error_pattern
    /// for a successful attempt, or routing for a manual override).
    Skipped,
}

/// One row in the settlement ledger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SinkSettlementEntry {
    /// Sink key (one of [`SINK_KEYS`]).
    pub sink_key: String,
    /// Row index (0-11).
    pub row: usize,
    /// Current state.
    pub state: SinkSettlementState,
    /// Error from the last failed attempt, if any.
    #[serde(default)]
    pub last_error: Option<String>,
}

/// Complete settlement ledger for one receipt.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SettlementLedger {
    /// Keyed by sink key, preserving row order.
    pub entries: HashMap<String, SinkSettlementEntry>,
}

impl SettlementLedger {
    /// Create a fresh ledger with all 12 rows in `Prepared` state.
    #[must_use]
    pub fn fresh() -> Self {
        let entries = SINK_KEYS
            .iter()
            .enumerate()
            .map(|(i, key)| {
                (
                    (*key).to_string(),
                    SinkSettlementEntry {
                        sink_key: (*key).to_string(),
                        row: i,
                        state: SinkSettlementState::Prepared,
                        last_error: None,
                    },
                )
            })
            .collect();
        Self { entries }
    }

    /// Whether all rows are in a terminal state (Settled or Skipped).
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.entries
            .values()
            .all(|e| matches!(e.state, SinkSettlementState::Settled | SinkSettlementState::Skipped))
    }

    /// The first row index that has not been settled or skipped.
    #[must_use]
    pub fn first_unsettled_row(&self) -> Option<usize> {
        for (i, key) in SINK_KEYS.iter().enumerate() {
            if let Some(entry) = self.entries.get(*key) {
                if entry.state == SinkSettlementState::Prepared {
                    return Some(i);
                }
            } else {
                return Some(i);
            }
        }
        None
    }

    /// Mark a sink row as settled.
    pub fn mark_settled(&mut self, sink_key: &str) {
        if let Some(entry) = self.entries.get_mut(sink_key) {
            entry.state = SinkSettlementState::Settled;
            entry.last_error = None;
        }
    }

    /// Mark a sink row as skipped.
    pub fn mark_skipped(&mut self, sink_key: &str) {
        if let Some(entry) = self.entries.get_mut(sink_key) {
            entry.state = SinkSettlementState::Skipped;
            entry.last_error = None;
        }
    }

    /// Record a failure for a sink row (keeps it in Prepared for retry).
    pub fn record_failure(&mut self, sink_key: &str, error: String) {
        if let Some(entry) = self.entries.get_mut(sink_key) {
            entry.last_error = Some(error);
        }
    }
}

// ---------------------------------------------------------------------------
// Sink trait
// ---------------------------------------------------------------------------

/// One settlement sink. Each row in the 12-row table has one implementation.
///
/// Sinks must be idempotent: if called twice with the same receipt and
/// idempotency key, the second call must be a no-op or return success.
#[async_trait::async_trait]
pub trait SettlementSink: Send + Sync + fmt::Debug {
    /// Stable sink key (one of [`SINK_KEYS`]).
    fn sink_key(&self) -> &'static str;

    /// Whether this sink should be invoked for the given receipt.
    ///
    /// For example, the `error_pattern` sink returns false for successful
    /// attempts, and the `routing` sink returns false for manual overrides.
    fn applicable(&self, receipt: &TaskAttemptReceiptV1) -> bool;

    /// Invoke the sink. Must be idempotent w.r.t. the receipt's
    /// `idempotency_key`.
    async fn settle(&self, receipt: &TaskAttemptReceiptV1) -> Result<(), SinkError>;
}

/// Error from a settlement sink invocation.
#[derive(Debug, Clone, thiserror::Error)]
#[error("sink '{sink_key}' failed: {message}")]
pub struct SinkError {
    /// Which sink failed.
    pub sink_key: String,
    /// Human-readable error message.
    pub message: String,
}

// ---------------------------------------------------------------------------
// Settlement outcome
// ---------------------------------------------------------------------------

/// Failure record for one sink.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SinkFailure {
    /// Sink key that failed.
    pub sink_key: String,
    /// Row index.
    pub row: usize,
    /// Error message.
    pub error: String,
}

impl fmt::Display for SinkFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[row {}] {}: {}", self.row, self.sink_key, self.error)
    }
}

/// Outcome of settling one receipt through the 12-row pipeline.
#[derive(Debug, Clone)]
pub enum SettlementOutcome {
    /// All sinks settled successfully (or were skipped as not applicable).
    FullySettled,
    /// All critical sinks settled but one or more optional sinks failed.
    CompletedWithDegradation(Vec<SinkFailure>),
    /// A critical sink (rows 0-2) failed. The task terminal state must
    /// NOT be committed.
    CriticalFailure(SinkFailure),
}

impl SettlementOutcome {
    /// Whether the outcome allows the task terminal state to be committed.
    #[must_use]
    pub fn allows_terminal_commit(&self) -> bool {
        !matches!(self, Self::CriticalFailure(_))
    }

    /// Whether any sinks had degraded delivery.
    #[must_use]
    pub fn has_degradation(&self) -> bool {
        matches!(self, Self::CompletedWithDegradation(_))
    }
}

// ---------------------------------------------------------------------------
// Settlement event
// ---------------------------------------------------------------------------

/// Event emitted after each sink row is settled or fails.
///
/// Consumers (graph event adapter, projection) subscribe to these for
/// parity comparison between Graph and Runner settlement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementEvent {
    /// The receipt's idempotency key.
    pub idempotency_key: String,
    /// Sink key.
    pub sink_key: String,
    /// Row index (0-11).
    pub row: usize,
    /// Whether settlement succeeded.
    pub settled: bool,
    /// Error message on failure.
    #[serde(default)]
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Settler
// ---------------------------------------------------------------------------

/// Callback for per-row settlement events.
pub type SettlementEventCallback = Box<dyn Fn(&SettlementEvent) + Send + Sync>;

/// The serial 12-row settlement pipeline.
///
/// Constructed with the ordered list of sink implementations. The settler
/// drives settlement through each row, maintaining the ledger and emitting
/// events.
pub struct FeedbackSettler {
    sinks: Vec<Box<dyn SettlementSink>>,
    event_callback: Option<SettlementEventCallback>,
}

impl fmt::Debug for FeedbackSettler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FeedbackSettler")
            .field("sinks", &self.sinks.len())
            .finish()
    }
}

impl FeedbackSettler {
    /// Construct a settler from an ordered list of sinks.
    ///
    /// # Panics
    ///
    /// Panics if `sinks.len() != 12` or if the sink keys don't match
    /// [`SINK_KEYS`] in order.
    #[must_use]
    pub fn new(sinks: Vec<Box<dyn SettlementSink>>) -> Self {
        assert_eq!(
            sinks.len(),
            SINK_KEYS.len(),
            "settler requires exactly 12 sinks, got {}",
            sinks.len()
        );
        for (i, sink) in sinks.iter().enumerate() {
            assert_eq!(
                sink.sink_key(),
                SINK_KEYS[i],
                "sink at row {i} has key '{}', expected '{}'",
                sink.sink_key(),
                SINK_KEYS[i]
            );
        }
        Self {
            sinks,
            event_callback: None,
        }
    }

    /// Attach an event callback for settlement progress.
    #[must_use]
    pub fn with_event_callback(mut self, cb: SettlementEventCallback) -> Self {
        self.event_callback = Some(cb);
        self
    }

    /// Settle one receipt through all 12 rows.
    ///
    /// If `ledger` is `Some`, settlement resumes from the first unsettled
    /// row (idempotent replay). Otherwise a fresh ledger is created.
    ///
    /// The returned ledger should be persisted by the caller for resume.
    pub async fn settle(
        &self,
        receipt: &TaskAttemptReceiptV1,
        ledger: Option<SettlementLedger>,
    ) -> (SettlementOutcome, SettlementLedger) {
        let mut ledger = ledger.unwrap_or_else(SettlementLedger::fresh);
        let start_row = ledger.first_unsettled_row().unwrap_or(SINK_KEYS.len());
        let mut optional_failures: Vec<SinkFailure> = Vec::new();

        for row in start_row..SINK_KEYS.len() {
            let sink = &self.sinks[row];
            let sink_key = SINK_KEYS[row];

            // Check applicability.
            if !sink.applicable(receipt) {
                ledger.mark_skipped(sink_key);
                self.emit_event(&SettlementEvent {
                    idempotency_key: receipt.idempotency_key.clone(),
                    sink_key: sink_key.to_string(),
                    row,
                    settled: true,
                    error: None,
                });
                continue;
            }

            // Mark as prepared (write before invoke).
            // The ledger entry was already Prepared from fresh(), but this
            // makes the intent explicit for resumed ledgers.

            // Invoke the sink.
            match sink.settle(receipt).await {
                Ok(()) => {
                    ledger.mark_settled(sink_key);
                    self.emit_event(&SettlementEvent {
                        idempotency_key: receipt.idempotency_key.clone(),
                        sink_key: sink_key.to_string(),
                        row,
                        settled: true,
                        error: None,
                    });
                }
                Err(err) => {
                    let error_msg = err.message.clone();
                    ledger.record_failure(sink_key, error_msg.clone());

                    self.emit_event(&SettlementEvent {
                        idempotency_key: receipt.idempotency_key.clone(),
                        sink_key: sink_key.to_string(),
                        row,
                        settled: false,
                        error: Some(error_msg.clone()),
                    });

                    if row < CRITICAL_SINK_COUNT {
                        // Critical failure: stop settlement.
                        let failure = SinkFailure {
                            sink_key: sink_key.to_string(),
                            row,
                            error: error_msg,
                        };
                        return (SettlementOutcome::CriticalFailure(failure), ledger);
                    }

                    // Optional failure: record and continue.
                    optional_failures.push(SinkFailure {
                        sink_key: sink_key.to_string(),
                        row,
                        error: error_msg,
                    });
                }
            }
        }

        let outcome = if optional_failures.is_empty() {
            SettlementOutcome::FullySettled
        } else {
            SettlementOutcome::CompletedWithDegradation(optional_failures)
        };

        (outcome, ledger)
    }

    fn emit_event(&self, event: &SettlementEvent) {
        if let Some(cb) = &self.event_callback {
            cb(event);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    use super::*;
    use crate::feedback::receipt::{AttemptTerminalStatus, ChoiceSource};

    /// A test sink that optionally fails.
    #[derive(Debug)]
    struct TestSink {
        key: &'static str,
        fail: bool,
        call_count: Arc<AtomicU32>,
    }

    impl TestSink {
        fn new(key: &'static str, fail: bool) -> Self {
            Self {
                key,
                fail,
                call_count: Arc::new(AtomicU32::new(0)),
            }
        }

        fn calls(&self) -> u32 {
            self.call_count.load(Ordering::Relaxed)
        }
    }

    #[async_trait::async_trait]
    impl SettlementSink for TestSink {
        fn sink_key(&self) -> &'static str {
            self.key
        }

        fn applicable(&self, _receipt: &TaskAttemptReceiptV1) -> bool {
            true
        }

        async fn settle(&self, _receipt: &TaskAttemptReceiptV1) -> Result<(), SinkError> {
            self.call_count.fetch_add(1, Ordering::Relaxed);
            if self.fail {
                Err(SinkError {
                    sink_key: self.key.to_string(),
                    message: format!("test failure in {}", self.key),
                })
            } else {
                Ok(())
            }
        }
    }

    /// A sink that skips non-failed attempts (like error_pattern).
    #[derive(Debug)]
    struct FailedOnlySink {
        key: &'static str,
        call_count: Arc<AtomicU32>,
    }

    #[async_trait::async_trait]
    impl SettlementSink for FailedOnlySink {
        fn sink_key(&self) -> &'static str {
            self.key
        }

        fn applicable(&self, receipt: &TaskAttemptReceiptV1) -> bool {
            !receipt.succeeded()
        }

        async fn settle(&self, _receipt: &TaskAttemptReceiptV1) -> Result<(), SinkError> {
            self.call_count.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }

    fn test_receipt() -> TaskAttemptReceiptV1 {
        let mut r = TaskAttemptReceiptV1::new("run-1", "plan-a", "task-1", "node-1", 0);
        r.resolved_provider = "claude_cli".into();
        r.resolved_model = "claude-sonnet-4-6".into();
        r.choice_source = ChoiceSource::Router;
        r.terminal_status = AttemptTerminalStatus::Succeeded;
        r.tokens_in = 200;
        r.tokens_out = 80;
        r.actual_cost_micro_usd = 3000;
        r
    }

    fn all_passing_sinks() -> Vec<Box<dyn SettlementSink>> {
        SINK_KEYS
            .iter()
            .map(|key| Box::new(TestSink::new(key, false)) as Box<dyn SettlementSink>)
            .collect()
    }

    fn sinks_with_failure(fail_key: &'static str) -> Vec<Box<dyn SettlementSink>> {
        SINK_KEYS
            .iter()
            .map(|key| {
                Box::new(TestSink::new(key, *key == fail_key)) as Box<dyn SettlementSink>
            })
            .collect()
    }

    #[tokio::test]
    async fn fully_settled_happy_path() {
        let settler = FeedbackSettler::new(all_passing_sinks());
        let receipt = test_receipt();
        let (outcome, ledger) = settler.settle(&receipt, None).await;

        assert!(
            matches!(outcome, SettlementOutcome::FullySettled),
            "expected FullySettled, got {outcome:?}"
        );
        assert!(outcome.allows_terminal_commit());
        assert!(!outcome.has_degradation());
        assert!(ledger.is_complete());
    }

    #[tokio::test]
    async fn critical_failure_stops_settlement() {
        let settler = FeedbackSettler::new(sinks_with_failure("actual_cost"));
        let receipt = test_receipt();
        let (outcome, ledger) = settler.settle(&receipt, None).await;

        assert!(
            matches!(outcome, SettlementOutcome::CriticalFailure(ref f) if f.sink_key == "actual_cost"),
            "expected CriticalFailure on actual_cost, got {outcome:?}"
        );
        assert!(!outcome.allows_terminal_commit());
        assert!(!ledger.is_complete());

        // Row 0 (attempt_receipt) should be settled.
        assert_eq!(
            ledger.entries["attempt_receipt"].state,
            SinkSettlementState::Settled
        );
        // Row 1 (actual_cost) should still be Prepared (failed).
        assert_eq!(
            ledger.entries["actual_cost"].state,
            SinkSettlementState::Prepared
        );
        assert!(ledger.entries["actual_cost"].last_error.is_some());
        // Row 2+ should not have been reached.
        assert_eq!(
            ledger.entries["structured_audit"].state,
            SinkSettlementState::Prepared
        );
    }

    #[tokio::test]
    async fn optional_failure_continues_with_degradation() {
        let settler = FeedbackSettler::new(sinks_with_failure("episode"));
        let receipt = test_receipt();
        let (outcome, ledger) = settler.settle(&receipt, None).await;

        assert!(
            matches!(outcome, SettlementOutcome::CompletedWithDegradation(ref failures) if failures.len() == 1),
            "expected CompletedWithDegradation with 1 failure, got {outcome:?}"
        );
        assert!(outcome.allows_terminal_commit());
        assert!(outcome.has_degradation());

        // All rows except episode should be settled.
        for key in &SINK_KEYS {
            if *key == "episode" {
                // Episode stays Prepared with error.
                assert_eq!(ledger.entries[*key].state, SinkSettlementState::Prepared);
            } else {
                assert_eq!(
                    ledger.entries[*key].state,
                    SinkSettlementState::Settled,
                    "sink {key} should be settled"
                );
            }
        }
    }

    #[tokio::test]
    async fn resume_skips_already_settled_rows() {
        let sinks = all_passing_sinks();
        // Get shared call counters before moving sinks into the settler.
        let counters: Vec<_> = sinks
            .iter()
            .map(|s| {
                // Safety: we know these are TestSink instances.
                let test_sink = s.as_ref() as *const dyn SettlementSink as *const TestSink;
                unsafe { (*test_sink).call_count.clone() }
            })
            .collect();

        let settler = FeedbackSettler::new(sinks);
        let receipt = test_receipt();

        // First settlement.
        let (_, ledger) = settler.settle(&receipt, None).await;
        assert!(ledger.is_complete());

        // All 12 sinks called exactly once.
        for (i, counter) in counters.iter().enumerate() {
            assert_eq!(
                counter.load(Ordering::Relaxed),
                1,
                "sink {} should be called once on first settle",
                SINK_KEYS[i]
            );
        }

        // Resume with the completed ledger: no sinks should be called again.
        let (outcome, _) = settler.settle(&receipt, Some(ledger)).await;
        assert!(matches!(outcome, SettlementOutcome::FullySettled));

        for (i, counter) in counters.iter().enumerate() {
            assert_eq!(
                counter.load(Ordering::Relaxed),
                1,
                "sink {} should NOT be called again on resume",
                SINK_KEYS[i]
            );
        }
    }

    #[tokio::test]
    async fn resume_restarts_at_first_unsettled() {
        let settler = FeedbackSettler::new(all_passing_sinks());
        let receipt = test_receipt();

        // Create a ledger with first 5 rows settled.
        let mut ledger = SettlementLedger::fresh();
        for key in &SINK_KEYS[..5] {
            ledger.mark_settled(key);
        }
        assert_eq!(ledger.first_unsettled_row(), Some(5));

        let (outcome, final_ledger) = settler.settle(&receipt, Some(ledger)).await;
        assert!(matches!(outcome, SettlementOutcome::FullySettled));
        assert!(final_ledger.is_complete());
    }

    #[tokio::test]
    async fn not_applicable_sink_is_skipped() {
        let error_pattern_counter = Arc::new(AtomicU32::new(0));

        let mut sinks: Vec<Box<dyn SettlementSink>> = Vec::new();
        for (i, key) in SINK_KEYS.iter().enumerate() {
            if i == 6 {
                // error_pattern only fires for failed attempts.
                sinks.push(Box::new(FailedOnlySink {
                    key,
                    call_count: error_pattern_counter.clone(),
                }));
            } else {
                sinks.push(Box::new(TestSink::new(key, false)));
            }
        }

        let settler = FeedbackSettler::new(sinks);
        // Successful receipt: error_pattern should be skipped.
        let receipt = test_receipt();
        let (outcome, ledger) = settler.settle(&receipt, None).await;

        assert!(matches!(outcome, SettlementOutcome::FullySettled));
        assert_eq!(
            error_pattern_counter.load(Ordering::Relaxed),
            0,
            "error_pattern should not be called for success"
        );
        assert_eq!(
            ledger.entries["error_pattern"].state,
            SinkSettlementState::Skipped
        );
    }

    #[tokio::test]
    async fn events_emitted_for_each_row() {
        let events = Arc::new(parking_lot::Mutex::new(Vec::new()));
        let events_clone = events.clone();

        let settler = FeedbackSettler::new(all_passing_sinks()).with_event_callback(Box::new(
            move |event| {
                events_clone.lock().push(event.clone());
            },
        ));
        let receipt = test_receipt();
        let _ = settler.settle(&receipt, None).await;

        let captured = events.lock();
        assert_eq!(captured.len(), 12, "should emit one event per row");
        for (i, event) in captured.iter().enumerate() {
            assert_eq!(event.row, i);
            assert_eq!(event.sink_key, SINK_KEYS[i]);
            assert!(event.settled);
            assert!(event.error.is_none());
        }
    }

    #[test]
    fn ledger_serde_roundtrip() {
        let mut ledger = SettlementLedger::fresh();
        ledger.mark_settled("attempt_receipt");
        ledger.mark_skipped("error_pattern");
        ledger.record_failure("episode", "write failed".into());

        let json = serde_json::to_string(&ledger).expect("serialize");
        let back: SettlementLedger = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(
            back.entries["attempt_receipt"].state,
            SinkSettlementState::Settled
        );
        assert_eq!(
            back.entries["error_pattern"].state,
            SinkSettlementState::Skipped
        );
        assert_eq!(
            back.entries["episode"].last_error.as_deref(),
            Some("write failed")
        );
    }

    #[test]
    fn first_unsettled_row_fresh_ledger() {
        let ledger = SettlementLedger::fresh();
        assert_eq!(ledger.first_unsettled_row(), Some(0));
    }

    #[test]
    fn first_unsettled_row_partial() {
        let mut ledger = SettlementLedger::fresh();
        ledger.mark_settled("attempt_receipt");
        ledger.mark_settled("actual_cost");
        assert_eq!(ledger.first_unsettled_row(), Some(2));
    }

    #[test]
    fn first_unsettled_row_complete() {
        let mut ledger = SettlementLedger::fresh();
        for key in &SINK_KEYS {
            ledger.mark_settled(key);
        }
        assert!(ledger.first_unsettled_row().is_none());
        assert!(ledger.is_complete());
    }

    #[test]
    fn sink_error_display() {
        let err = SinkError {
            sink_key: "episode".into(),
            message: "disk full".into(),
        };
        assert_eq!(err.to_string(), "sink 'episode' failed: disk full");
    }

    #[test]
    fn sink_failure_display() {
        let f = SinkFailure {
            sink_key: "routing".into(),
            row: 5,
            error: "router unavailable".into(),
        };
        assert_eq!(f.to_string(), "[row 5] routing: router unavailable");
    }
}
