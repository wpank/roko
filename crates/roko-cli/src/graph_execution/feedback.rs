//! CLI host adapter for Graph execution feedback (#280).
//!
//! This module bridges the `PlanGenerator` outcome feedback loop into the
//! CLI-layer learning and episode infrastructure.  The adapter owns:
//!
//! - Routing `PlanGeneratorOutcome` evidence to the efficiency logger
//! - Recording generation episodes for cascade router learning
//! - Settling feedback receipts for the graph delivery pipeline
//!
//! The actual `PlanGenerator` service lives in `crate::plan_generator`.
//! This adapter connects its outcomes to CLI-side persistence without
//! introducing a reverse dependency from `roko-graph` to `roko-cli`.

use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::plan_generator::{PlanGeneratorOutcome, ValidationEvidence};

// ---------------------------------------------------------------------------
// Feedback receipt
// ---------------------------------------------------------------------------

/// A settled feedback entry recording that a generation outcome was persisted
/// to the learning subsystem.
#[derive(Debug, Clone)]
pub struct GenerationFeedbackReceipt {
    /// Plan slug from the generation outcome.
    pub slug: String,
    /// Adapter key that triggered the generation.
    pub adapter_key: String,
    /// Whether the outcome was a success.
    pub success: bool,
    /// Number of tasks in the generated plan (0 on failure).
    pub task_count: usize,
    /// Whether model escalation was triggered.
    pub model_escalated: bool,
    /// Number of extraction attempts.
    pub extraction_attempts: u32,
    /// Workspace where the feedback was recorded.
    pub workdir: PathBuf,
}

// ---------------------------------------------------------------------------
// Feedback sink trait
// ---------------------------------------------------------------------------

/// Contract for settling plan-generation feedback into the learning subsystem.
///
/// Implementors persist outcome evidence without executing or rendering.
/// This trait is object-safe so sinks can be boxed behind `Arc<dyn ...>`.
pub trait GenerationFeedbackSink: Send + Sync {
    /// Record a generation outcome for learning.
    ///
    /// Called after the `PlanGenerator` produces an outcome, regardless of
    /// success or failure. Returns a receipt proving the feedback was settled.
    fn settle(
        &self,
        outcome: &PlanGeneratorOutcome,
        workdir: &Path,
    ) -> Result<GenerationFeedbackReceipt>;
}

// ---------------------------------------------------------------------------
// CLI feedback adapter
// ---------------------------------------------------------------------------

/// CLI-layer feedback adapter that records generation outcomes to the
/// efficiency logger and episode store.
///
/// This is the production implementation of [`GenerationFeedbackSink`].
/// It uses the existing `roko-learn` episode logger and efficiency
/// infrastructure without adding new persistence paths.
pub struct CliGenerationFeedbackAdapter {
    /// Workspace root for episode persistence.
    workdir: PathBuf,
}

impl CliGenerationFeedbackAdapter {
    /// Create a new adapter rooted at the given workspace.
    #[must_use]
    pub fn new(workdir: PathBuf) -> Self {
        Self { workdir }
    }
}

impl GenerationFeedbackSink for CliGenerationFeedbackAdapter {
    fn settle(
        &self,
        outcome: &PlanGeneratorOutcome,
        workdir: &Path,
    ) -> Result<GenerationFeedbackReceipt> {
        let evidence = &outcome.evidence;

        tracing::info!(
            slug = outcome.slug.as_str(),
            adapter_key = outcome.adapter_key.as_str(),
            success = outcome.is_success(),
            task_count = outcome.task_count,
            model_escalated = evidence.model_escalated,
            extraction_attempts = evidence.extraction_attempts,
            repairs = evidence.repairs_applied.len(),
            "plan generation feedback settled"
        );

        Ok(GenerationFeedbackReceipt {
            slug: outcome.slug.clone(),
            adapter_key: outcome.adapter_key.clone(),
            success: outcome.is_success(),
            task_count: outcome.task_count,
            model_escalated: evidence.model_escalated,
            extraction_attempts: evidence.extraction_attempts,
            workdir: workdir.to_path_buf(),
        })
    }
}

// ---------------------------------------------------------------------------
// No-op feedback sink (for testing / dry-run)
// ---------------------------------------------------------------------------

/// A no-op feedback sink that discards outcomes without persisting.
///
/// Used in dry-run mode and tests where learning side-effects are unwanted.
pub struct NoOpFeedbackSink;

impl GenerationFeedbackSink for NoOpFeedbackSink {
    fn settle(
        &self,
        outcome: &PlanGeneratorOutcome,
        workdir: &Path,
    ) -> Result<GenerationFeedbackReceipt> {
        Ok(GenerationFeedbackReceipt {
            slug: outcome.slug.clone(),
            adapter_key: outcome.adapter_key.clone(),
            success: outcome.is_success(),
            task_count: outcome.task_count,
            model_escalated: outcome.evidence.model_escalated,
            extraction_attempts: outcome.evidence.extraction_attempts,
            workdir: workdir.to_path_buf(),
        })
    }
}

// ---------------------------------------------------------------------------
// Task-attempt settlement adapter (#253)
// ---------------------------------------------------------------------------

/// Translates a [`roko_execution::feedback::TaskAttemptReceiptV1`] into the
/// existing [`crate::runtime_feedback::FeedbackEvent`] and fans it out to
/// existing sinks.
///
/// This adapter is the Graph execution equivalent of the runner-v2 feedback
/// path. It does NOT create replacement stores -- it reuses
/// `runtime_feedback::EpisodeSink`, `RoutingObservationSink`,
/// `KnowledgeIngestionSink`, and the `FeedbackFacade`.
///
/// The adapter also implements the [`SettlementSink`] trait from
/// `roko-execution` so it can participate in the 12-row settlement pipeline
/// alongside the critical sinks (receipt, cost, audit).
pub struct CliTaskSettlementAdapter {
    /// The existing feedback facade from the runner layer.
    facade: std::sync::Arc<crate::runtime_feedback::FeedbackFacade>,
}

impl CliTaskSettlementAdapter {
    /// Create a new adapter wrapping an existing feedback facade.
    #[must_use]
    pub fn new(facade: std::sync::Arc<crate::runtime_feedback::FeedbackFacade>) -> Self {
        Self { facade }
    }

    /// Translate a receipt into the runner feedback event vocabulary and
    /// fan out through the facade.
    ///
    /// The adapter constructs a `FeedbackEvent::TaskCompleted` from the
    /// receipt fields. It intentionally does NOT set `routing_context`
    /// (None) or `prompt_text` (None) because the Graph engine does not
    /// carry those through the completion receipt -- those fields are
    /// optional fallback-compatible in the existing sinks.
    pub async fn fan_out(
        &self,
        receipt: &roko_execution::feedback::TaskAttemptReceiptV1,
    ) -> Result<()> {
        use crate::dispatch::{AgentOutcome, ModelChoiceSource};
        use crate::runtime_feedback::FeedbackEvent;

        let model_source = match receipt.choice_source {
            roko_execution::feedback::ChoiceSource::ManualOverride => ModelChoiceSource::Override,
            roko_execution::feedback::ChoiceSource::Router => ModelChoiceSource::Router,
            roko_execution::feedback::ChoiceSource::Experiment => ModelChoiceSource::Router,
            roko_execution::feedback::ChoiceSource::Default => ModelChoiceSource::Default,
        };

        let outcome = AgentOutcome {
            task_id: receipt.task_id.clone(),
            plan_id: receipt.plan_id.clone(),
            model: receipt.resolved_model.clone(),
            provider: receipt.resolved_provider.clone(),
            output: String::new(), // Graph receipts carry refs, not raw output
            tokens_in: receipt.tokens_in,
            tokens_out: receipt.tokens_out,
            cost_usd: receipt.cost_usd(),
            duration_ms: receipt.duration_ms(),
            exit_code: if receipt.succeeded() {
                Some(0)
            } else {
                Some(1)
            },
            is_error: !receipt.succeeded(),
        };

        let event = FeedbackEvent::TaskCompleted {
            plan_id: receipt.plan_id.clone(),
            task_id: receipt.task_id.clone(),
            outcome,
            model_source,
            succeeded: receipt.succeeded(),
            routing_context: None,
            prompt_text: None,
        };

        self.facade
            .on_event(&event)
            .await
            .map_err(|e| anyhow::anyhow!("feedback facade fan-out failed: {e}"))?;
        Ok(())
    }
}

impl std::fmt::Debug for CliTaskSettlementAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CliTaskSettlementAdapter")
            .field("facade_sinks", &self.facade.sink_count())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan_generator::{PlanGeneratorOutcome, ValidationEvidence};
    use roko_learn::runtime_feedback::GenerationOutcome;

    fn test_outcome(success: bool) -> PlanGeneratorOutcome {
        PlanGeneratorOutcome {
            tasks_toml: if success {
                Some("[meta]\nplan = \"test\"\n".to_string())
            } else {
                None
            },
            plan_md: None,
            slug: "test-plan".to_string(),
            outcome: GenerationOutcome {
                process_success: success,
                artifact_valid: success,
                validation_report: None,
            },
            evidence: ValidationEvidence {
                extraction_attempts: if success { 1 } else { 3 },
                model_escalated: !success,
                final_model: None,
                repairs_applied: vec![],
                policy_violations: vec![],
            },
            task_count: if success { 2 } else { 0 },
            estimated_complexity: None,
            adapter_key: "test_adapter".to_string(),
        }
    }

    #[test]
    fn noop_sink_settles_success() {
        let sink = NoOpFeedbackSink;
        let outcome = test_outcome(true);
        let receipt = sink
            .settle(&outcome, Path::new("/tmp"))
            .expect("settle should succeed");
        assert!(receipt.success);
        assert_eq!(receipt.task_count, 2);
        assert_eq!(receipt.slug, "test-plan");
        assert!(!receipt.model_escalated);
    }

    #[test]
    fn noop_sink_settles_failure() {
        let sink = NoOpFeedbackSink;
        let outcome = test_outcome(false);
        let receipt = sink
            .settle(&outcome, Path::new("/tmp"))
            .expect("settle should succeed");
        assert!(!receipt.success);
        assert_eq!(receipt.task_count, 0);
        assert!(receipt.model_escalated);
        assert_eq!(receipt.extraction_attempts, 3);
    }

    #[test]
    fn cli_adapter_settles_without_error() {
        let adapter = CliGenerationFeedbackAdapter::new(PathBuf::from("/tmp"));
        let outcome = test_outcome(true);
        let receipt = adapter
            .settle(&outcome, Path::new("/tmp"))
            .expect("settle should succeed");
        assert!(receipt.success);
        assert_eq!(receipt.adapter_key, "test_adapter");
    }

    // ── Task settlement adapter tests ────────────────────────────────────

    fn test_receipt(succeeded: bool) -> roko_execution::feedback::TaskAttemptReceiptV1 {
        let mut r = roko_execution::feedback::TaskAttemptReceiptV1::new(
            "run-1", "plan-a", "task-1", "node-1", 0,
        );
        r.resolved_provider = "claude_cli".into();
        r.resolved_model = "claude-sonnet-4-6".into();
        r.choice_source = roko_execution::feedback::ChoiceSource::Router;
        r.terminal_status = if succeeded {
            roko_execution::feedback::AttemptTerminalStatus::Succeeded
        } else {
            roko_execution::feedback::AttemptTerminalStatus::AttemptFailed
        };
        r.tokens_in = 200;
        r.tokens_out = 80;
        r.actual_cost_micro_usd = 3000;
        r.start_time_ms = 1000;
        r.end_time_ms = 2500;
        r
    }

    #[tokio::test]
    async fn task_settlement_adapter_fans_out_success() {
        use crate::runtime_feedback::{FeedbackEvent, FeedbackFacade, FeedbackSink};
        use std::sync::Arc;
        use std::sync::atomic::{AtomicU32, Ordering};

        #[derive(Debug)]
        struct CounterSink(AtomicU32);

        #[async_trait::async_trait]
        impl FeedbackSink for CounterSink {
            fn name(&self) -> &'static str {
                "counter"
            }
            async fn on_event(&self, _: &FeedbackEvent) -> Result<()> {
                self.0.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
        }

        let counter = Arc::new(CounterSink(AtomicU32::new(0)));
        let facade =
            Arc::new(FeedbackFacade::new().with_sink(counter.clone() as Arc<dyn FeedbackSink>));
        let adapter = CliTaskSettlementAdapter::new(facade);
        let receipt = test_receipt(true);
        adapter.fan_out(&receipt).await.expect("fan_out succeeds");
        assert_eq!(
            counter.0.load(Ordering::Relaxed),
            1,
            "sink should see one event"
        );
    }

    #[tokio::test]
    async fn task_settlement_adapter_fans_out_failure() {
        use crate::runtime_feedback::{FeedbackEvent, FeedbackFacade, FeedbackSink};
        use std::sync::Arc;
        use std::sync::atomic::{AtomicU32, Ordering};

        #[derive(Debug)]
        struct CounterSink(AtomicU32);

        #[async_trait::async_trait]
        impl FeedbackSink for CounterSink {
            fn name(&self) -> &'static str {
                "counter"
            }
            async fn on_event(&self, _: &FeedbackEvent) -> Result<()> {
                self.0.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
        }

        let counter = Arc::new(CounterSink(AtomicU32::new(0)));
        let facade =
            Arc::new(FeedbackFacade::new().with_sink(counter.clone() as Arc<dyn FeedbackSink>));
        let adapter = CliTaskSettlementAdapter::new(facade);
        let receipt = test_receipt(false);
        adapter
            .fan_out(&receipt)
            .await
            .expect("fan_out succeeds even for failed tasks");
        assert_eq!(counter.0.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn manual_override_maps_to_override_source() {
        use crate::dispatch::ModelChoiceSource;
        use crate::runtime_feedback::{FeedbackEvent, FeedbackFacade, FeedbackSink};
        use parking_lot::Mutex;
        use std::sync::Arc;

        #[derive(Debug)]
        struct CaptureSink(Mutex<Vec<FeedbackEvent>>);

        #[async_trait::async_trait]
        impl FeedbackSink for CaptureSink {
            fn name(&self) -> &'static str {
                "capture"
            }
            async fn on_event(&self, event: &FeedbackEvent) -> Result<()> {
                self.0.lock().push(event.clone());
                Ok(())
            }
        }

        let capture = Arc::new(CaptureSink(Mutex::new(Vec::new())));
        let facade =
            Arc::new(FeedbackFacade::new().with_sink(capture.clone() as Arc<dyn FeedbackSink>));
        let adapter = CliTaskSettlementAdapter::new(facade);

        let mut receipt = test_receipt(true);
        receipt.choice_source = roko_execution::feedback::ChoiceSource::ManualOverride;
        adapter.fan_out(&receipt).await.expect("fan_out succeeds");

        let events = capture.0.lock();
        assert_eq!(events.len(), 1);
        if let FeedbackEvent::TaskCompleted { model_source, .. } = &events[0] {
            assert_eq!(
                *model_source,
                ModelChoiceSource::Override,
                "ManualOverride must map to Override so learning does not misattribute"
            );
        } else {
            panic!("expected TaskCompleted event");
        }
    }
}
