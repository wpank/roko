//! Gate output rendering, failure classification, and pre-existing failure
//! filtering.
//!
//! Extracted from `gate_dispatch.rs` to separate verdict presentation from
//! gate execution.

use roko_core::Verdict;
use roko_gate::classify_gate_failure;
use tracing::info;

use super::types::{GateCompletionKind, GateVerdictSummary, RunnerFailureKind};
use super::gate_input::GateInputSnapshot;

// ── Attribution helpers ─────────────────────────────────────────────────

pub(super) fn raw_gate_name(name: &str) -> &str {
    name.strip_prefix("baseline+owned:")
        .or_else(|| name.strip_prefix("baseline:"))
        .or_else(|| name.strip_prefix("owned-diff:"))
        .or_else(|| name.strip_prefix("unattributed:"))
        .unwrap_or(name)
}

// ── Pre-existing failure filtering ──────────────────────────────────────

fn normalized_failure_fingerprint(digest: Option<&str>) -> Option<String> {
    fn remove_volatile(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(object) => {
                object.remove("duration_ms");
                for child in object.values_mut() {
                    remove_volatile(child);
                }
            }
            serde_json::Value::Array(array) => {
                for child in array {
                    remove_volatile(child);
                }
            }
            _ => {}
        }
    }
    let digest = digest?.trim();
    if digest.is_empty() {
        return None;
    }
    let mut value = serde_json::from_str::<serde_json::Value>(digest).ok()?;
    remove_volatile(&mut value);
    serde_json::to_string(&value).ok()
}

pub(super) fn filter_preexisting_failures(
    task_id: &str,
    verdicts: &mut [Verdict],
    baseline: Option<&[GateVerdictSummary]>,
) {
    let Some(baseline) = baseline else {
        return;
    };
    for verdict in verdicts
        .iter_mut()
        .filter(|verdict| !verdict.passed && !verdict.skipped)
    {
        let current_name = raw_gate_name(&verdict.gate);
        let current_fingerprint = normalized_failure_fingerprint(verdict.error_digest.as_deref());
        let unchanged = baseline.iter().any(|prior| {
            !prior.passed
                && raw_gate_name(&prior.gate_name) == current_name
                && current_fingerprint.is_some()
                && current_fingerprint
                    == normalized_failure_fingerprint(prior.error_digest.as_deref())
        });
        if unchanged {
            let original = verdict.gate.clone();
            verdict.passed = true;
            verdict.gate = format!("pre-existing-filtered:{original}");
            verdict.reason = "unchanged pre-existing verification failure filtered".into();
            info!(
                task_id,
                gate = %original,
                "filtered unchanged pre-existing gate failure"
            );
        }
    }
}

// ── Gate failure input attribution ──────────────────────────────────────

pub(super) fn gate_failure_input(
    kind: GateCompletionKind,
    before: &GateInputSnapshot,
    baseline_failed_gates: Option<&[GateVerdictSummary]>,
    gate: &str,
) -> &'static str {
    match (kind, before.2, baseline_failed_gates) {
        (GateCompletionKind::Preflight, _, _) | (GateCompletionKind::Gate, false, _) => "baseline",
        (GateCompletionKind::Gate, true, Some(failures))
            if failures
                .iter()
                .any(|failure| raw_gate_name(&failure.gate_name) == raw_gate_name(gate)) =>
        {
            "baseline+owned"
        }
        (GateCompletionKind::Gate, true, Some(_)) => "owned-diff",
        (GateCompletionKind::Gate, true, None) => "unattributed",
        (GateCompletionKind::PlanVerify, _, _) => "accepted-plan",
        (GateCompletionKind::Merge, _, _) => "post-merge",
    }
}

// ── Output rendering ────────────────────────────────────────────────────

pub(super) fn render_output(verdicts: &[Verdict]) -> String {
    verdicts
        .iter()
        .map(render_verdict_output)
        .collect::<Vec<_>>()
        .join("; ")
}

fn render_verdict_output(v: &Verdict) -> String {
    let status = if v.skipped {
        "SKIP"
    } else if v.passed {
        "pass"
    } else {
        "FAIL"
    };
    let detail = v.detail.as_deref().unwrap_or("").trim();
    let digest = v.error_digest.as_deref().unwrap_or("").trim();
    let reason = v.reason.trim();

    let message = if v.passed {
        first_non_empty([detail, reason, digest])
    } else if !detail.is_empty() && !digest.is_empty() {
        format!("{detail}\n\nclassification:\n{digest}")
    } else {
        first_non_empty([detail, reason, digest])
    };

    if message.is_empty() {
        format!("{}: {status}", v.gate)
    } else {
        format!("{}: {status} — {message}", v.gate)
    }
}

fn first_non_empty<const N: usize>(values: [&str; N]) -> String {
    values
        .into_iter()
        .find(|value| !value.is_empty())
        .unwrap_or("")
        .to_string()
}

// ── Failure classification ──────────────────────────────────────────────

pub(super) fn classify_failure_kind(verdicts: &[Verdict], output: &str) -> RunnerFailureKind {
    let combined = verdicts
        .iter()
        .filter(|v| !v.passed)
        .map(|v| {
            format!(
                "{}\n{}\n{}",
                v.reason,
                v.detail.as_deref().unwrap_or(""),
                v.error_digest.as_deref().unwrap_or("")
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let text = if combined.trim().is_empty() {
        output
    } else {
        &combined
    };
    let classification = classify_gate_failure("runner", text);
    let rendered = serde_json::to_string(&classification).unwrap_or_default();
    let fallback = RunnerFailureKind::from_output(text);
    match classification.recommended_action {
        roko_gate::GateFailureAction::Blocked => RunnerFailureKind::Resource,
        roko_gate::GateFailureAction::NeedsHuman => RunnerFailureKind::Permanent,
        roko_gate::GateFailureAction::NeedsReplan => RunnerFailureKind::Structural,
        roko_gate::GateFailureAction::Retry => {
            if rendered.contains("external_environment") {
                RunnerFailureKind::Transient
            } else {
                match fallback {
                    RunnerFailureKind::Resource | RunnerFailureKind::Transient => fallback,
                    RunnerFailureKind::Permanent
                    | RunnerFailureKind::Structural
                    | RunnerFailureKind::ContextOverflow
                    | RunnerFailureKind::Unknown => RunnerFailureKind::Structural,
                }
            }
        }
    }
}
