//! RunnerProductionGateAdapter — bridges Runner-v2 gate parameters to the
//! shared [`ProductionGateRunner`] service.
//!
//! Extracted from `gate_dispatch.rs` to keep the adapter layer separate from
//! the inline gate execution pipeline.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use roko_core::config::GatesConfig;
use roko_gate::PlanComplexity;
use roko_gate::generated_test_gate::ArtifactStore as GeneratedArtifactStore;
use tracing::error;

use crate::task_parser::VerifyStep;

use super::gate_dispatch::{GateTaskContext, failed_gate_completion};
use super::types::{
    GateCompletion, GateCompletionKind, GateEffectRef, GateVerdictSummary, RunnerFailureKind,
};

// ═══════════════════════════════════════════════════════════════════════════
// RunnerProductionGateAdapter (#275)
// ═══════════════════════════════════════════════════════════════════════════

/// Adapter that converts Runner-v2 gate parameters into a
/// [`ProductionGateRequest`], calls the injected
/// [`ProductionGateRunner`], and converts the
/// [`ProductionGateVerdictV1`] back into a [`GateCompletion`].
///
/// This is the single point of conversion between the Runner-v2 types
/// (which own event-loop integration, attempt ownership, and TUI events)
/// and the shared production gate service (which owns rung selection,
/// execution, and verdict normalization).
///
/// ## Call-site manifest
///
/// Four production boundaries redirect through this adapter:
///
/// 1. `run_gate_once` -- delegates to `Self::run` instead of inline rung execution.
/// 2. `spawn_gate` worker body -- the spawned task calls `Self::run`.
/// 3. Preflight spawn branch in `event_loop.rs` -- injects the same shared service.
/// 4. Normal/plan-verify spawn branch in `event_loop.rs` -- injects the same shared service.
pub struct RunnerProductionGateAdapter {
    /// The injected shared gate service.
    service: Arc<dyn roko_gate::production_service::ProductionGateRunner>,
}

impl std::fmt::Debug for RunnerProductionGateAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunnerProductionGateAdapter")
            .finish_non_exhaustive()
    }
}

impl RunnerProductionGateAdapter {
    /// Create an adapter wrapping the given shared service.
    pub fn new(service: Arc<dyn roko_gate::production_service::ProductionGateRunner>) -> Self {
        Self { service }
    }

    /// Convert Runner-v2 parameters into a `ProductionGateRequest`.
    pub(crate) fn build_request(
        effect: &GateEffectRef,
        plan_id: &str,
        task_id: &str,
        workdir: &Path,
        gates_config: &GatesConfig,
        verify_steps: &[VerifyStep],
        timeout_secs: u64,
        target_crates: &[String],
        task_context: Option<&GateTaskContext>,
        cancel: tokio_util::sync::CancellationToken,
    ) -> roko_gate::ProductionGateRequest {
        // Convert CLI VerifyStep -> neutral VerifyStepSpec.
        let verify_step_specs: Vec<roko_gate::VerifyStepSpec> = verify_steps
            .iter()
            .map(|step| {
                roko_gate::VerifyStepSpec::from_command(&step.command)
                    .with_phase(&step.phase)
                    .with_timeout_ms(step.timeout_ms)
            })
            .collect();

        // Convert GateTaskContext -> GateTaskContextSpec.
        let task_context_spec = task_context
            .map(|ctx| roko_gate::GateTaskContextSpec {
                title: ctx.task_title.clone(),
                description: ctx.task_description.clone(),
                symbols: ctx.symbols.clone(),
                acceptance: ctx.acceptance.clone(),
            })
            .unwrap_or_default();

        // Compute workspace fingerprint synchronously from the workdir.
        let workspace_fingerprint = format!("{}:{}:{}", plan_id, task_id, effect.generation);

        roko_gate::ProductionGateRequest {
            run_id: format!("{}:{}", plan_id, effect.generation),
            plan_id: plan_id.to_string(),
            task_id: task_id.to_string(),
            attempt: effect.attempt.attempt,
            workspace: workdir.to_path_buf(),
            workspace_fingerprint,
            changed_files: target_crates.to_vec(),
            verify_steps: verify_step_specs,
            gates_config: gates_config.clone(),
            task_context: task_context_spec,
            timeout_secs,
            cancel,
            baseline_fingerprint: None,
            adaptive_thresholds: None,
        }
    }

    /// Convert a `ProductionGateVerdictV1` back into a `GateCompletion`.
    pub(crate) fn verdict_to_completion(
        effect: GateEffectRef,
        plan_id: String,
        task_id: String,
        rung: u32,
        verdict: &roko_gate::ProductionGateVerdictV1,
    ) -> GateCompletion {
        let passed = verdict.passed();

        // Map per-rung verdicts to GateVerdictSummary.
        let summaries: Vec<GateVerdictSummary> = verdict
            .rung_verdicts
            .iter()
            .map(|rv| {
                let failure_kind = if rv.skipped() || rv.passed() {
                    None
                } else {
                    rv.failure_classification
                        .as_ref()
                        .map(|fc| match fc.recommended_action {
                            roko_gate::GateFailureAction::Blocked => RunnerFailureKind::Resource,
                            roko_gate::GateFailureAction::NeedsHuman => {
                                RunnerFailureKind::Permanent
                            }
                            roko_gate::GateFailureAction::NeedsReplan => {
                                RunnerFailureKind::Structural
                            }
                            roko_gate::GateFailureAction::Retry => RunnerFailureKind::Transient,
                        })
                        .or(Some(RunnerFailureKind::Unknown))
                };
                GateVerdictSummary {
                    gate_name: rv.gate_name.clone(),
                    passed: rv.passed(),
                    skipped: rv.skipped(),
                    summary: rv.diagnostic.chars().take(500).collect(),
                    error_digest: rv
                        .failure_classification
                        .as_ref()
                        .map(|fc| format!("{:?}", fc.primary)),
                    failure_kind,
                    rung_index: Some(rv.rung.as_index()),
                }
            })
            .collect();

        let selected_rungs: Vec<String> = verdict
            .rung_verdicts
            .iter()
            .filter(|rv| !rv.skipped())
            .map(|rv| rv.rung.label().to_string())
            .collect();

        let failure_kind = if !passed {
            summaries
                .iter()
                .find_map(|s| s.failure_kind)
                .or(Some(RunnerFailureKind::Unknown))
        } else {
            None
        };

        // Collect output from rung diagnostics.
        let output: String = verdict
            .rung_verdicts
            .iter()
            .filter(|rv| !rv.diagnostic.is_empty())
            .map(|rv| format!("{}: {}", rv.gate_name, rv.diagnostic))
            .collect::<Vec<_>>()
            .join("; ");

        GateCompletion {
            kind: effect.kind,
            attempt: Some(effect.attempt.clone()),
            effect: Some(effect),
            plan_id,
            task_id,
            rung,
            passed,
            failure_kind,
            verdicts: summaries,
            output,
            duration_ms: verdict.total_duration.as_millis() as u64,
            selected_rungs,
        }
    }

    /// Run the production gate pipeline through the shared service and return
    /// a `GateCompletion` compatible with the Runner-v2 event loop.
    ///
    /// This is the primary entry point that replaces the inline execution in
    /// `run_gate_once`. The existing `run_gate_once` delegates to this method
    /// when a `RunnerProductionGateAdapter` is available.
    pub async fn run(
        &self,
        effect: GateEffectRef,
        plan_id: String,
        task_id: String,
        rung: u32,
        workdir: PathBuf,
        gates_config: GatesConfig,
        _complexity: PlanComplexity,
        verify_steps: Vec<VerifyStep>,
        _baseline_failed_gates: Option<Vec<GateVerdictSummary>>,
        timeout_secs: u64,
        target_crates: Vec<String>,
        task_context: Option<GateTaskContext>,
    ) -> GateCompletion {
        let cancel = tokio_util::sync::CancellationToken::new();
        let request = Self::build_request(
            &effect,
            &plan_id,
            &task_id,
            &workdir,
            &gates_config,
            &verify_steps,
            timeout_secs,
            &target_crates,
            task_context.as_ref(),
            cancel,
        );

        let progress = Arc::new(roko_gate::production_service::NoopProgressSink);
        match self.service.run(request, progress).await {
            Ok(verdict) => Self::verdict_to_completion(effect, plan_id, task_id, rung, &verdict),
            Err(err) => {
                error!(%err, "production gate service error");
                failed_gate_completion(
                    effect,
                    plan_id,
                    task_id,
                    rung,
                    format!("production gate service error: {err}"),
                )
            }
        }
    }
}

/// Create a default `RunnerProductionGateAdapter` with the production service.
///
/// Used by the event loop when no custom service is injected.
pub fn default_gate_adapter() -> RunnerProductionGateAdapter {
    RunnerProductionGateAdapter::new(Arc::new(
        roko_gate::production_service::ProductionGateService::new(),
    )
        as Arc<dyn roko_gate::production_service::ProductionGateRunner>)
}

// ── Generated-test artifact store ───────────────────────────────────────

/// Filesystem-backed store for generated test artifacts, keyed by plan.
#[derive(Clone, Debug)]
pub(crate) struct FsGeneratedArtifactStore {
    root: PathBuf,
}

impl FsGeneratedArtifactStore {
    pub(crate) fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn artifact_dir(&self) -> PathBuf {
        self.root.join("generated-tests")
    }

    pub(crate) fn matching_entries(&self, prefix: &str) -> Vec<String> {
        let dir = self.artifact_dir();
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return Vec::new();
        };

        let mut names: Vec<String> = entries
            .filter_map(std::result::Result::ok)
            .filter_map(|entry| {
                entry.file_type().ok().filter(|kind| kind.is_file())?;
                let name = entry.file_name().to_string_lossy().into_owned();
                let logical = format!("generated-tests/{name}");
                logical.starts_with(prefix).then_some(logical)
            })
            .collect();
        names.sort();
        names
    }
}

impl GeneratedArtifactStore for FsGeneratedArtifactStore {
    fn list(&self, _plan: &str, prefix: &str) -> Vec<String> {
        self.matching_entries(prefix)
    }

    fn read(&self, _plan: &str, name: &str) -> Option<Vec<u8>> {
        let relative = name.strip_prefix("generated-tests/")?;
        if relative.contains("..") || relative.contains('/') {
            return None;
        }
        std::fs::read(self.artifact_dir().join(relative)).ok()
    }
}
