//! Concrete [`SettlementSink`] implementations for the 12-row completion
//! feedback pipeline (backlog #253).
//!
//! Each sink delegates to an existing store in `roko-learn`, `roko-daimon`,
//! `roko-conductor`, or the telemetry projection layer. No replacement stores
//! are created -- this module is purely an adapter layer.
//!
//! # Layer
//!
//! This module is layer 4 (CLI). It imports layer-3 types from `roko-learn`,
//! `roko-execution`, and `roko-graph`. No circular dependencies.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;

use roko_execution::FeedbackBundle;
use roko_execution::feedback::receipt::{ChoiceSource, TaskAttemptReceiptV1};
use roko_execution::feedback::settler::{FeedbackSettler, SINK_KEYS, SettlementSink, SinkError};

// ---------------------------------------------------------------------------
// Sink factory
// ---------------------------------------------------------------------------

/// Build a [`FeedbackSettler`] from a [`FeedbackBundle`].
///
/// Returns a settler wired with all 12 concrete sinks that delegate to
/// existing stores. The caller should persist the returned
/// [`SettlementLedger`] for resume/idempotency.
pub fn build_settler(feedback: &FeedbackBundle) -> FeedbackSettler {
    let learn_dir = feedback.learn_dir.clone();
    let health = Arc::clone(&feedback.health_registry);
    let cascade = feedback.cascade_router.clone();

    let sinks: Vec<Box<dyn SettlementSink>> = vec![
        // 0: attempt_receipt -- critical
        Box::new(AttemptReceiptSink {
            learn_dir: learn_dir.clone(),
        }),
        // 1: actual_cost -- critical
        Box::new(ActualCostSink {
            learn_dir: learn_dir.clone(),
        }),
        // 2: structured_audit -- critical
        Box::new(StructuredAuditSink {
            learn_dir: learn_dir.clone(),
        }),
        // 3: episode -- optional
        Box::new(EpisodeSink {
            learn_dir: learn_dir.clone(),
        }),
        // 4: efficiency -- optional
        Box::new(EfficiencySink {
            learn_dir: learn_dir.clone(),
        }),
        // 5: routing -- optional
        Box::new(RoutingSink {
            cascade_router: cascade,
        }),
        // 6: error_pattern -- optional (failed only)
        Box::new(ErrorPatternSink {
            learn_dir: learn_dir.clone(),
        }),
        // 7: playbook -- optional
        Box::new(PlaybookSink {
            learn_dir: learn_dir.clone(),
        }),
        // 8: knowledge -- optional
        Box::new(KnowledgeSink {
            learn_dir: learn_dir.clone(),
        }),
        // 9: daimon -- optional
        Box::new(DaimonSink {
            learn_dir: learn_dir.clone(),
        }),
        // 10: conductor -- optional
        Box::new(ConductorSink {
            health_registry: health,
        }),
        // 11: projection -- optional
        Box::new(ProjectionSink { learn_dir }),
    ];

    FeedbackSettler::new(sinks)
}

// ---------------------------------------------------------------------------
// Deduplication key helper
// ---------------------------------------------------------------------------

/// Deduplication key: `{receipt_id}:{sink_name}`.
fn dedup_key(receipt: &TaskAttemptReceiptV1, sink_key: &str) -> String {
    format!("{}:{}", receipt.idempotency_key, sink_key)
}

// ---------------------------------------------------------------------------
// Row 0: attempt_receipt (critical)
// ---------------------------------------------------------------------------

/// Persists the canonical receipt as a JSONL entry under the learn directory.
#[derive(Debug)]
struct AttemptReceiptSink {
    learn_dir: PathBuf,
}

#[async_trait]
impl SettlementSink for AttemptReceiptSink {
    fn sink_key(&self) -> &'static str {
        "attempt_receipt"
    }

    fn applicable(&self, _receipt: &TaskAttemptReceiptV1) -> bool {
        true
    }

    async fn settle(&self, receipt: &TaskAttemptReceiptV1) -> Result<(), SinkError> {
        let path = self.learn_dir.join("attempt-receipts.jsonl");
        let line = serde_json::to_string(receipt).map_err(|e| SinkError {
            sink_key: self.sink_key().to_string(),
            message: format!("serialize receipt: {e}"),
        })?;
        append_line(&path, &line).await.map_err(|e| SinkError {
            sink_key: self.sink_key().to_string(),
            message: format!("write receipt: {e}"),
        })
    }
}

// ---------------------------------------------------------------------------
// Row 1: actual_cost (critical)
// ---------------------------------------------------------------------------

/// Records the actual provider cost into the cost log.
#[derive(Debug)]
struct ActualCostSink {
    learn_dir: PathBuf,
}

#[async_trait]
impl SettlementSink for ActualCostSink {
    fn sink_key(&self) -> &'static str {
        "actual_cost"
    }

    fn applicable(&self, _receipt: &TaskAttemptReceiptV1) -> bool {
        true
    }

    async fn settle(&self, receipt: &TaskAttemptReceiptV1) -> Result<(), SinkError> {
        let record = roko_learn::costs_db::CostRecord {
            timestamp: chrono::Utc::now().to_rfc3339(),
            model: receipt.resolved_model.clone(),
            provider: receipt.resolved_provider.clone(),
            role: String::new(),
            plan_id: receipt.plan_id.clone(),
            task_id: receipt.task_id.clone(),
            complexity_band: String::new(),
            input_tokens: receipt.tokens_in,
            output_tokens: receipt.tokens_out,
            cached_tokens: 0,
            cost_usd: receipt.cost_usd(),
            duration_ms: receipt.duration_ms(),
            success: receipt.succeeded(),
            session_id: receipt.run_id.clone(),
        };
        let path = self.learn_dir.join("costs.jsonl");
        let line = serde_json::to_string(&record).map_err(|e| SinkError {
            sink_key: self.sink_key().to_string(),
            message: format!("serialize cost: {e}"),
        })?;
        append_line(&path, &line).await.map_err(|e| SinkError {
            sink_key: self.sink_key().to_string(),
            message: format!("write cost: {e}"),
        })
    }
}

// ---------------------------------------------------------------------------
// Row 2: structured_audit (critical)
// ---------------------------------------------------------------------------

/// Emits a structured audit line to the audit JSONL.
#[derive(Debug)]
struct StructuredAuditSink {
    learn_dir: PathBuf,
}

#[async_trait]
impl SettlementSink for StructuredAuditSink {
    fn sink_key(&self) -> &'static str {
        "structured_audit"
    }

    fn applicable(&self, _receipt: &TaskAttemptReceiptV1) -> bool {
        true
    }

    async fn settle(&self, receipt: &TaskAttemptReceiptV1) -> Result<(), SinkError> {
        let audit = AuditLine {
            kind: "task_attempt_settled".to_string(),
            idempotency_key: receipt.idempotency_key.clone(),
            plan_id: receipt.plan_id.clone(),
            task_id: receipt.task_id.clone(),
            attempt: receipt.attempt,
            terminal_status: format!("{:?}", receipt.terminal_status),
            provider: receipt.resolved_provider.clone(),
            model: receipt.resolved_model.clone(),
            cost_micro_usd: receipt.actual_cost_micro_usd,
            tokens_in: receipt.tokens_in,
            tokens_out: receipt.tokens_out,
            duration_ms: receipt.duration_ms(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        let path = self.learn_dir.join("audit.jsonl");
        let line = serde_json::to_string(&audit).map_err(|e| SinkError {
            sink_key: self.sink_key().to_string(),
            message: format!("serialize audit: {e}"),
        })?;
        append_line(&path, &line).await.map_err(|e| SinkError {
            sink_key: self.sink_key().to_string(),
            message: format!("write audit: {e}"),
        })
    }
}

/// Structured audit line matching the spec's `AuditLine` input type.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct AuditLine {
    kind: String,
    idempotency_key: String,
    plan_id: String,
    task_id: String,
    attempt: u32,
    terminal_status: String,
    provider: String,
    model: String,
    cost_micro_usd: u64,
    tokens_in: u64,
    tokens_out: u64,
    duration_ms: u64,
    timestamp: String,
}

// ---------------------------------------------------------------------------
// Row 3: episode (optional)
// ---------------------------------------------------------------------------

/// Records an episode entry to episodes.jsonl.
#[derive(Debug)]
struct EpisodeSink {
    learn_dir: PathBuf,
}

#[async_trait]
impl SettlementSink for EpisodeSink {
    fn sink_key(&self) -> &'static str {
        "episode"
    }

    fn applicable(&self, _receipt: &TaskAttemptReceiptV1) -> bool {
        true
    }

    async fn settle(&self, receipt: &TaskAttemptReceiptV1) -> Result<(), SinkError> {
        let mut episode =
            roko_learn::episode_logger::Episode::new(&receipt.resolved_provider, &receipt.task_id);
        episode.success = receipt.succeeded();
        episode.extra.insert(
            "idempotency_key".to_string(),
            serde_json::Value::String(receipt.idempotency_key.clone()),
        );
        // The episode logger lives under .roko/, not .roko/learn/.
        let roko_dir = self.learn_dir.parent().unwrap_or(&self.learn_dir);
        let logger =
            roko_learn::episode_logger::EpisodeLogger::new(roko_dir.join("episodes.jsonl"));
        logger.append(&episode).await.map_err(|e| SinkError {
            sink_key: self.sink_key().to_string(),
            message: format!("episode logger: {e}"),
        })
    }
}

// ---------------------------------------------------------------------------
// Row 4: efficiency (optional)
// ---------------------------------------------------------------------------

/// Records an efficiency summary to efficiency-summaries.jsonl.
#[derive(Debug)]
struct EfficiencySink {
    learn_dir: PathBuf,
}

#[async_trait]
impl SettlementSink for EfficiencySink {
    fn sink_key(&self) -> &'static str {
        "efficiency"
    }

    fn applicable(&self, _receipt: &TaskAttemptReceiptV1) -> bool {
        true
    }

    async fn settle(&self, receipt: &TaskAttemptReceiptV1) -> Result<(), SinkError> {
        let summary = EfficiencySummary {
            idempotency_key: receipt.idempotency_key.clone(),
            plan_id: receipt.plan_id.clone(),
            task_id: receipt.task_id.clone(),
            attempt: receipt.attempt,
            model: receipt.resolved_model.clone(),
            tokens_in: receipt.tokens_in,
            tokens_out: receipt.tokens_out,
            cost_usd: receipt.cost_usd(),
            duration_ms: receipt.duration_ms(),
            succeeded: receipt.succeeded(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        let path = self.learn_dir.join("efficiency-summaries.jsonl");
        let line = serde_json::to_string(&summary).map_err(|e| SinkError {
            sink_key: self.sink_key().to_string(),
            message: format!("serialize efficiency: {e}"),
        })?;
        append_line(&path, &line).await.map_err(|e| SinkError {
            sink_key: self.sink_key().to_string(),
            message: format!("write efficiency: {e}"),
        })
    }
}

/// Compact efficiency summary for the efficiency sink.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct EfficiencySummary {
    idempotency_key: String,
    plan_id: String,
    task_id: String,
    attempt: u32,
    model: String,
    tokens_in: u64,
    tokens_out: u64,
    cost_usd: f64,
    duration_ms: u64,
    succeeded: bool,
    timestamp: String,
}

// ---------------------------------------------------------------------------
// Row 5: routing (optional)
// ---------------------------------------------------------------------------

/// Updates the cascade router with the routing outcome. Skipped when the
/// choice source was a manual override or when no router is available.
#[derive(Debug)]
struct RoutingSink {
    cascade_router: Option<Arc<roko_learn::cascade_router::CascadeRouter>>,
}

#[async_trait]
impl SettlementSink for RoutingSink {
    fn sink_key(&self) -> &'static str {
        "routing"
    }

    fn applicable(&self, receipt: &TaskAttemptReceiptV1) -> bool {
        receipt.choice_source != ChoiceSource::ManualOverride && self.cascade_router.is_some()
    }

    async fn settle(&self, receipt: &TaskAttemptReceiptV1) -> Result<(), SinkError> {
        let router = self.cascade_router.as_ref().ok_or_else(|| SinkError {
            sink_key: self.sink_key().to_string(),
            message: "no cascade router".to_string(),
        })?;
        use roko_agent::model_call_service::ForceBackendOverrideRecorder;
        ForceBackendOverrideRecorder::record_override_outcome(
            router.as_ref(),
            &receipt.resolved_model,
            receipt.succeeded(),
        );
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Row 6: error_pattern (optional, failed only)
// ---------------------------------------------------------------------------

/// Records error patterns for failed attempts.
#[derive(Debug)]
struct ErrorPatternSink {
    learn_dir: PathBuf,
}

#[async_trait]
impl SettlementSink for ErrorPatternSink {
    fn sink_key(&self) -> &'static str {
        "error_pattern"
    }

    fn applicable(&self, receipt: &TaskAttemptReceiptV1) -> bool {
        !receipt.succeeded()
    }

    async fn settle(&self, receipt: &TaskAttemptReceiptV1) -> Result<(), SinkError> {
        let path = self.learn_dir.join("error-patterns.json");
        let observation = roko_learn::error_pattern_store::GateFailureObservation::new(
            dedup_key(receipt, "error_pattern"),
            &receipt.plan_id,
            Some(receipt.task_id.clone()),
            "completion",
            format!("{:?}", receipt.terminal_status),
            &receipt.error,
            roko_learn::error_pattern_store::GateFailureSource::RetryClassifier,
        );
        // ErrorPatternStore uses sync I/O; run in blocking context.
        let path_clone = path.clone();
        tokio::task::spawn_blocking(move || {
            let mut store = roko_learn::error_pattern_store::ErrorPatternStore::load(&path_clone);
            store.observe_gate_failure(observation);
            store.save(&path_clone)
        })
        .await
        .map_err(|e| SinkError {
            sink_key: self.sink_key().to_string(),
            message: format!("join error: {e}"),
        })?
        .map_err(|e| SinkError {
            sink_key: self.sink_key().to_string(),
            message: format!("save patterns: {e}"),
        })
    }
}

// ---------------------------------------------------------------------------
// Row 7: playbook (optional)
// ---------------------------------------------------------------------------

/// Updates playbook success/failure counters.
#[derive(Debug)]
struct PlaybookSink {
    learn_dir: PathBuf,
}

#[async_trait]
impl SettlementSink for PlaybookSink {
    fn sink_key(&self) -> &'static str {
        "playbook"
    }

    fn applicable(&self, _receipt: &TaskAttemptReceiptV1) -> bool {
        true
    }

    async fn settle(&self, receipt: &TaskAttemptReceiptV1) -> Result<(), SinkError> {
        let store = roko_learn::playbook::PlaybookStore::new(self.learn_dir.join("playbooks"));
        let playbook_id = format!("task-{}", receipt.task_id);
        store
            .record_outcome(&playbook_id, receipt.succeeded())
            .await
            .map_err(|e| SinkError {
                sink_key: self.sink_key().to_string(),
                message: format!("playbook outcome: {e}"),
            })?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Row 8: knowledge (optional)
// ---------------------------------------------------------------------------

/// Records knowledge ingestion seeds for successful attempts.
#[derive(Debug)]
struct KnowledgeSink {
    learn_dir: PathBuf,
}

#[async_trait]
impl SettlementSink for KnowledgeSink {
    fn sink_key(&self) -> &'static str {
        "knowledge"
    }

    fn applicable(&self, receipt: &TaskAttemptReceiptV1) -> bool {
        receipt.succeeded()
    }

    async fn settle(&self, receipt: &TaskAttemptReceiptV1) -> Result<(), SinkError> {
        let seed = KnowledgeSeed {
            idempotency_key: receipt.idempotency_key.clone(),
            plan_id: receipt.plan_id.clone(),
            task_id: receipt.task_id.clone(),
            provider: receipt.resolved_provider.clone(),
            model: receipt.resolved_model.clone(),
            changed_files: receipt.changed_files.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        let path = self.learn_dir.join("knowledge-seeds.jsonl");
        let line = serde_json::to_string(&seed).map_err(|e| SinkError {
            sink_key: self.sink_key().to_string(),
            message: format!("serialize seed: {e}"),
        })?;
        append_line(&path, &line).await.map_err(|e| SinkError {
            sink_key: self.sink_key().to_string(),
            message: format!("write seed: {e}"),
        })
    }
}

/// Knowledge ingestion seed matching the spec's `KnowledgeIngestion` type.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct KnowledgeSeed {
    idempotency_key: String,
    plan_id: String,
    task_id: String,
    provider: String,
    model: String,
    changed_files: Vec<String>,
    timestamp: String,
}

// ---------------------------------------------------------------------------
// Row 9: daimon (optional)
// ---------------------------------------------------------------------------

/// Feeds the affect engine with task/gate appraisal data.
#[derive(Debug)]
struct DaimonSink {
    learn_dir: PathBuf,
}

#[async_trait]
impl SettlementSink for DaimonSink {
    fn sink_key(&self) -> &'static str {
        "daimon"
    }

    fn applicable(&self, _receipt: &TaskAttemptReceiptV1) -> bool {
        true
    }

    async fn settle(&self, receipt: &TaskAttemptReceiptV1) -> Result<(), SinkError> {
        let feedback = DaimonFeedbackRecord {
            idempotency_key: receipt.idempotency_key.clone(),
            task_id: receipt.task_id.clone(),
            succeeded: receipt.succeeded(),
            cost_usd: receipt.cost_usd(),
            duration_ms: receipt.duration_ms(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        let daimon_dir = self
            .learn_dir
            .parent()
            .unwrap_or(&self.learn_dir)
            .join("daimon");
        let path = daimon_dir.join("feedback.jsonl");
        // Ensure directory exists.
        if let Some(parent) = path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        let line = serde_json::to_string(&feedback).map_err(|e| SinkError {
            sink_key: self.sink_key().to_string(),
            message: format!("serialize daimon: {e}"),
        })?;
        append_line(&path, &line).await.map_err(|e| SinkError {
            sink_key: self.sink_key().to_string(),
            message: format!("write daimon: {e}"),
        })
    }
}

/// Daimon feedback record for affect state updates.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct DaimonFeedbackRecord {
    idempotency_key: String,
    task_id: String,
    succeeded: bool,
    cost_usd: f64,
    duration_ms: u64,
    timestamp: String,
}

// ---------------------------------------------------------------------------
// Row 10: conductor (optional)
// ---------------------------------------------------------------------------

/// Feeds conductor feedback for intervention policy learning.
#[derive(Debug)]
struct ConductorSink {
    health_registry: Arc<roko_learn::provider_health::ProviderHealthRegistry>,
}

#[async_trait]
impl SettlementSink for ConductorSink {
    fn sink_key(&self) -> &'static str {
        "conductor"
    }

    fn applicable(&self, _receipt: &TaskAttemptReceiptV1) -> bool {
        true
    }

    async fn settle(&self, receipt: &TaskAttemptReceiptV1) -> Result<(), SinkError> {
        if receipt.succeeded() {
            self.health_registry
                .record_success(&receipt.resolved_provider);
        } else {
            self.health_registry.record_failure(
                &receipt.resolved_provider,
                roko_learn::provider_health::ErrorClass::Unknown,
            );
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Row 11: projection (optional)
// ---------------------------------------------------------------------------

/// Emits a structured projection event for downstream dashboards and parity
/// comparison. Replayable from the durable audit log.
#[derive(Debug)]
struct ProjectionSink {
    learn_dir: PathBuf,
}

#[async_trait]
impl SettlementSink for ProjectionSink {
    fn sink_key(&self) -> &'static str {
        "projection"
    }

    fn applicable(&self, _receipt: &TaskAttemptReceiptV1) -> bool {
        true
    }

    async fn settle(&self, receipt: &TaskAttemptReceiptV1) -> Result<(), SinkError> {
        let event = ProjectionEvent {
            kind: "completion_settled".to_string(),
            idempotency_key: receipt.idempotency_key.clone(),
            plan_id: receipt.plan_id.clone(),
            task_id: receipt.task_id.clone(),
            attempt: receipt.attempt,
            terminal_status: format!("{:?}", receipt.terminal_status),
            cost_usd: receipt.cost_usd(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        let path = self.learn_dir.join("projections.jsonl");
        let line = serde_json::to_string(&event).map_err(|e| SinkError {
            sink_key: self.sink_key().to_string(),
            message: format!("serialize projection: {e}"),
        })?;
        append_line(&path, &line).await.map_err(|e| SinkError {
            sink_key: self.sink_key().to_string(),
            message: format!("write projection: {e}"),
        })
    }
}

/// Projection event for dashboards and parity comparison.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct ProjectionEvent {
    kind: String,
    idempotency_key: String,
    plan_id: String,
    task_id: String,
    attempt: u32,
    terminal_status: String,
    cost_usd: f64,
    timestamp: String,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Append a single line to a JSONL file, creating parent directories if needed.
async fn append_line(path: &std::path::Path, line: &str) -> Result<(), std::io::Error> {
    use tokio::io::AsyncWriteExt;

    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .await?;
    file.write_all(line.as_bytes()).await?;
    file.write_all(b"\n").await?;
    file.flush().await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// CompletionSinkResult (handoff to #254)
// ---------------------------------------------------------------------------

/// Result of settling one receipt, for handoff to completion delivery (#254).
///
/// The `drive_controller` host checks this and proceeds to delivery only if
/// all critical sinks (orders 0-2) settled. Optional sink failures produce
/// `CompletedWithDegradation` and do not block delivery.
#[derive(Debug, Clone)]
pub struct CompletionSinkResult {
    /// Sink keys that settled successfully.
    pub settled: Vec<String>,
    /// Sink failures (optional sinks only; critical failures abort).
    pub failed: Vec<roko_execution::feedback::settler::SinkFailure>,
}

impl CompletionSinkResult {
    /// Build from a [`SettlementOutcome`].
    pub fn from_outcome(outcome: &roko_execution::feedback::settler::SettlementOutcome) -> Self {
        use roko_execution::feedback::settler::SettlementOutcome;
        match outcome {
            SettlementOutcome::FullySettled => Self {
                settled: SINK_KEYS.iter().map(|k| k.to_string()).collect(),
                failed: Vec::new(),
            },
            SettlementOutcome::CompletedWithDegradation(failures) => {
                let failed_keys: std::collections::HashSet<&str> =
                    failures.iter().map(|f| f.sink_key.as_str()).collect();
                Self {
                    settled: SINK_KEYS
                        .iter()
                        .filter(|k| !failed_keys.contains(*k))
                        .map(|k| k.to_string())
                        .collect(),
                    failed: failures.clone(),
                }
            }
            SettlementOutcome::CriticalFailure(failure) => Self {
                settled: Vec::new(),
                failed: vec![failure.clone()],
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use roko_execution::feedback::receipt::AttemptTerminalStatus;
    use roko_execution::feedback::settler::{SettlementOutcome, SinkSettlementState};
    use tempfile::TempDir;

    fn test_receipt() -> TaskAttemptReceiptV1 {
        let mut r = TaskAttemptReceiptV1::new("run-1", "plan-a", "task-1", "node-1", 0);
        r.resolved_provider = "claude_cli".into();
        r.resolved_model = "claude-sonnet-4-6".into();
        r.choice_source = ChoiceSource::Router;
        r.terminal_status = AttemptTerminalStatus::Succeeded;
        r.tokens_in = 200;
        r.tokens_out = 80;
        r.actual_cost_micro_usd = 3_000_000;
        r.start_time_ms = 1000;
        r.end_time_ms = 5000;
        r
    }

    fn feedback_bundle(dir: &std::path::Path) -> FeedbackBundle {
        FeedbackBundle {
            learn_dir: dir.to_path_buf(),
            health_registry: Arc::new(roko_learn::provider_health::ProviderHealthRegistry::new()),
            cascade_router: None,
        }
    }

    #[tokio::test]
    async fn settle_all_sinks_happy_path() {
        let tmp = TempDir::new().unwrap();
        let learn_dir = tmp.path().join("learn");
        std::fs::create_dir_all(&learn_dir).unwrap();

        let bundle = feedback_bundle(&learn_dir);
        let settler = build_settler(&bundle);
        let receipt = test_receipt();

        let (outcome, ledger) = settler.settle(&receipt, None).await;

        assert!(
            outcome.allows_terminal_commit(),
            "expected terminal commit allowed, got {outcome:?}"
        );
        assert!(ledger.is_complete());

        // Verify files were created.
        assert!(learn_dir.join("attempt-receipts.jsonl").exists());
        assert!(learn_dir.join("costs.jsonl").exists());
        assert!(learn_dir.join("audit.jsonl").exists());
        assert!(learn_dir.join("efficiency-summaries.jsonl").exists());
        assert!(learn_dir.join("projections.jsonl").exists());
    }

    #[tokio::test]
    async fn error_pattern_skipped_for_success() {
        let tmp = TempDir::new().unwrap();
        let learn_dir = tmp.path().join("learn");
        std::fs::create_dir_all(&learn_dir).unwrap();

        let bundle = feedback_bundle(&learn_dir);
        let settler = build_settler(&bundle);
        let receipt = test_receipt();

        let (_, ledger) = settler.settle(&receipt, None).await;

        assert_eq!(
            ledger.entries["error_pattern"].state,
            SinkSettlementState::Skipped,
            "error_pattern should be skipped for success"
        );
    }

    #[tokio::test]
    async fn knowledge_skipped_for_failure() {
        let tmp = TempDir::new().unwrap();
        let learn_dir = tmp.path().join("learn");
        std::fs::create_dir_all(&learn_dir).unwrap();

        let bundle = feedback_bundle(&learn_dir);
        let settler = build_settler(&bundle);
        let mut receipt = test_receipt();
        receipt.terminal_status = AttemptTerminalStatus::GateFailed;

        let (_, ledger) = settler.settle(&receipt, None).await;

        assert_eq!(
            ledger.entries["knowledge"].state,
            SinkSettlementState::Skipped,
            "knowledge should be skipped for failed attempts"
        );
    }

    #[tokio::test]
    async fn routing_skipped_for_manual_override() {
        let tmp = TempDir::new().unwrap();
        let learn_dir = tmp.path().join("learn");
        std::fs::create_dir_all(&learn_dir).unwrap();

        let bundle = feedback_bundle(&learn_dir);
        let settler = build_settler(&bundle);
        let mut receipt = test_receipt();
        receipt.choice_source = ChoiceSource::ManualOverride;

        let (_, ledger) = settler.settle(&receipt, None).await;

        assert_eq!(
            ledger.entries["routing"].state,
            SinkSettlementState::Skipped,
            "routing should be skipped for manual overrides"
        );
    }

    #[tokio::test]
    async fn resume_skips_already_settled() {
        let tmp = TempDir::new().unwrap();
        let learn_dir = tmp.path().join("learn");
        std::fs::create_dir_all(&learn_dir).unwrap();

        let bundle = feedback_bundle(&learn_dir);
        let settler = build_settler(&bundle);
        let receipt = test_receipt();

        // First settle.
        let (outcome1, ledger1) = settler.settle(&receipt, None).await;
        assert!(outcome1.allows_terminal_commit());

        // Count lines in receipt file.
        let receipts_content =
            std::fs::read_to_string(learn_dir.join("attempt-receipts.jsonl")).unwrap();
        let lines_first: usize = receipts_content.lines().count();

        // Resume with completed ledger: no new writes.
        let (outcome2, _) = settler.settle(&receipt, Some(ledger1)).await;
        assert!(outcome2.allows_terminal_commit());

        let receipts_content2 =
            std::fs::read_to_string(learn_dir.join("attempt-receipts.jsonl")).unwrap();
        let lines_second: usize = receipts_content2.lines().count();

        assert_eq!(
            lines_first, lines_second,
            "resume should not write duplicate receipt lines"
        );
    }

    #[tokio::test]
    async fn exactly_once_across_retry() {
        let tmp = TempDir::new().unwrap();
        let learn_dir = tmp.path().join("learn");
        std::fs::create_dir_all(&learn_dir).unwrap();

        let bundle = feedback_bundle(&learn_dir);
        let settler = build_settler(&bundle);
        let receipt = test_receipt();

        // Settle fully.
        let (_, ledger) = settler.settle(&receipt, None).await;
        assert!(ledger.is_complete());

        // "Retry" with same receipt and completed ledger.
        let (outcome, ledger2) = settler.settle(&receipt, Some(ledger)).await;
        assert!(matches!(outcome, SettlementOutcome::FullySettled));
        assert!(ledger2.is_complete());

        // Receipt file should still have exactly 1 line.
        let content = std::fs::read_to_string(learn_dir.join("attempt-receipts.jsonl")).unwrap();
        assert_eq!(content.lines().count(), 1, "exactly one receipt line");
    }

    #[tokio::test]
    async fn completion_sink_result_from_fully_settled() {
        let outcome = SettlementOutcome::FullySettled;
        let result = CompletionSinkResult::from_outcome(&outcome);
        assert_eq!(result.settled.len(), 12);
        assert!(result.failed.is_empty());
    }

    #[tokio::test]
    async fn completion_sink_result_from_degradation() {
        let failures = vec![roko_execution::feedback::settler::SinkFailure {
            sink_key: "episode".to_string(),
            row: 3,
            error: "disk full".to_string(),
        }];
        let outcome = SettlementOutcome::CompletedWithDegradation(failures);
        let result = CompletionSinkResult::from_outcome(&outcome);
        assert_eq!(result.settled.len(), 11);
        assert_eq!(result.failed.len(), 1);
        assert_eq!(result.failed[0].sink_key, "episode");
    }

    #[tokio::test]
    async fn dedup_key_format() {
        let receipt = test_receipt();
        let key = dedup_key(&receipt, "episode");
        assert_eq!(key, "run-1:plan-a:task-1:0:episode");
    }
}
