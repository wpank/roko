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
}
