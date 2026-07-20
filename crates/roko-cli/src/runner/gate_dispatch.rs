//! Verify dispatch — runs gate rungs as background tokio tasks and sends
//! results through a channel.

use std::fs::{File, OpenOptions};
use std::io::Read;
use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use futures::FutureExt;
use roko_core::config::GatesConfig;
use roko_core::{Body, Engram, EngramBuilder, Kind, Provenance, Verdict, Verify};
use roko_fs::RokoLayout;
use roko_gate::classify_gate_failure;
use roko_gate::rung_dispatch::{GatePipelineBuilder, RungExecutionConfig, RungExecutionInputs};
use roko_gate::{GatePayload, PlanComplexity, ShellGate};
use sha2::{Digest, Sha256};
use tokio::sync::{Semaphore, mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio::time::{Duration, timeout};
use tracing::{error, info};

use crate::task_parser::VerifyStep;

use super::types::{
    GateCompletion, GateCompletionKind, GateEffectRef, GateVerdictSummary, RunnerFailureKind,
    TaskAttemptRef,
};

/// Sentinel rung value for plan-level verification (not a per-task rung).
pub const RUNG_PLAN_VERIFY: u32 = 1000;
/// Sentinel rung value for post-merge regression gates.
pub const RUNG_MERGE: u32 = 1001;

#[derive(Debug, Clone, PartialEq, Eq)]
struct GateInputSnapshot(String, [u8; 32], bool);
const MAX_UNTRACKED_FILES: usize = 1024;
const MAX_UNTRACKED_FILE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_GATE_INPUT_BYTES: u64 = 32 * 1024 * 1024;
fn hash_part(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_le_bytes());
    hasher.update(bytes);
}
#[cfg(unix)]
fn metadata_unchanged(before: &std::fs::Metadata, after: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    before.file_type() == after.file_type()
        && before.len() == after.len()
        && before.modified().ok() == after.modified().ok()
        && before.dev() == after.dev()
        && before.ino() == after.ino()
}
fn gate_input_snapshot_blocking(workdir: &Path) -> Result<GateInputSnapshot, String> {
    #[cfg(not(unix))]
    return Err("stable gate input identity is unavailable on this platform".into());
    let git = |args: &[&str]| {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(workdir)
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .map_err(|error| error.to_string())?;
        if output.status.success() {
            Ok(output.stdout)
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    };
    let base_commit = String::from_utf8_lossy(&git(&["rev-parse", "HEAD"])?)
        .trim()
        .to_string();
    let diff = git(&["diff", "--binary", "HEAD", "--"])?;
    if diff.len() as u64 > MAX_GATE_INPUT_BYTES {
        return Err("tracked diff exceeds gate input byte limit".into());
    }
    let status = git(&[
        "status",
        "--porcelain=v1",
        "-z",
        "--ignored=matching",
        "-uall",
    ])?;
    roko_orchestrator::worktree::validate_workspace_file_kinds(workdir, &status)
        .map_err(|error| error.to_string())?;
    let untracked = git(&["ls-files", "--others", "--exclude-standard", "-z"])?;
    let mut hasher = Sha256::new();
    hash_part(&mut hasher, base_commit.as_bytes());
    hash_part(&mut hasher, &diff);
    let mut total_bytes = diff.len() as u64;
    for (index, raw_path) in untracked
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .enumerate()
    {
        if index >= MAX_UNTRACKED_FILES {
            return Err("untracked file count exceeds input limit".into());
        }
        let relative = std::str::from_utf8(raw_path).map_err(|error| error.to_string())?;
        let path = workdir.join(relative);
        let before = std::fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
        hash_part(&mut hasher, raw_path);
        if before.file_type().is_symlink() {
            let target_path = std::fs::read_link(&path).map_err(|error| error.to_string())?;
            let target = target_path.as_os_str().as_encoded_bytes();
            total_bytes = total_bytes.saturating_add(target.len() as u64);
            if target.len() as u64 > MAX_UNTRACKED_FILE_BYTES || total_bytes > MAX_GATE_INPUT_BYTES
            {
                return Err("untracked symlink exceeds input limit".into());
            }
            hasher.update([b'l']);
            hash_part(&mut hasher, target);
            if std::fs::read_link(&path).ok().as_ref() != Some(&target_path) {
                return Err("untracked symlink changed while hashing".into());
            }
        } else if before.is_file() {
            if before.len() > MAX_UNTRACKED_FILE_BYTES
                || total_bytes.saturating_add(before.len()) > MAX_GATE_INPUT_BYTES
            {
                return Err("untracked file exceeds input limit".into());
            }
            let mut options = OpenOptions::new();
            options.read(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
            }
            let mut file = options.open(&path).map_err(|error| error.to_string())?;
            let opened = file.metadata().map_err(|error| error.to_string())?;
            if !opened.is_file() || !metadata_unchanged(&before, &opened) {
                return Err("untracked file changed before hashing".into());
            }
            hasher.update([b'f']);
            hasher.update(before.len().to_le_bytes());
            let read_bytes = std::io::copy(&mut (&mut file).take(before.len() + 1), &mut hasher)
                .map_err(|error| error.to_string())?;
            let after = std::fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
            if read_bytes != before.len() || !metadata_unchanged(&before, &after) {
                return Err("untracked file changed while hashing".into());
            }
            total_bytes += read_bytes;
        } else {
            return Err("untracked path is not a regular file or symlink".into());
        }
    }
    let owned_diff: [u8; 32] = hasher.finalize().into();
    let has_owned_diff = !diff.is_empty() || !untracked.is_empty();
    Ok(GateInputSnapshot(base_commit, owned_diff, has_owned_diff))
}
async fn gate_input_snapshot(workdir: PathBuf) -> Result<GateInputSnapshot, String> {
    tokio::task::spawn_blocking(move || gate_input_snapshot_blocking(&workdir))
        .await
        .map_err(|error| error.to_string())?
}
async fn accepted_input_snapshot(
    workdir: PathBuf,
    expected_oid: &str,
) -> Result<GateInputSnapshot, String> {
    let snapshot = gate_input_snapshot(workdir).await?;
    (snapshot.0 == expected_oid && !snapshot.2)
        .then_some(snapshot)
        .ok_or_else(|| "accepted plan input differs from immutable commit".into())
}
fn raw_gate_name(name: &str) -> &str {
    name.strip_prefix("baseline+owned:")
        .or_else(|| name.strip_prefix("baseline:"))
        .or_else(|| name.strip_prefix("owned-diff:"))
        .or_else(|| name.strip_prefix("unattributed:"))
        .unwrap_or(name)
}
fn gate_failure_input(
    kind: GateCompletionKind,
    before: &GateInputSnapshot,
    baseline_failed_gates: Option<&[String]>,
    gate: &str,
) -> &'static str {
    match (kind, before.2, baseline_failed_gates) {
        (GateCompletionKind::Preflight, _, _) | (GateCompletionKind::Gate, false, _) => "baseline",
        (GateCompletionKind::Gate, true, Some(failures))
            if failures.iter().any(|name| name == raw_gate_name(gate)) =>
        {
            "baseline+owned"
        }
        (GateCompletionKind::Gate, true, Some(_)) => "owned-diff",
        (GateCompletionKind::Gate, true, None) => "unattributed",
        (GateCompletionKind::PlanVerify, _, _) => "accepted-plan",
        (GateCompletionKind::Merge, _, _) => "post-merge",
    }
}
macro_rules! proof_failure {
    ($gate:expr, $reason:expr, $digest:expr $(,)?) => {
        Verdict::fail($gate, $reason).with_error_digest($digest)
    };
}
/// Spawn a gate rung as a background task. Sends `GateCompletion` when done.
pub fn spawn_gate(
    effect: GateEffectRef,
    plan_id: String,
    task_id: String,
    rung: u32,
    workdir: PathBuf,
    gates_config: GatesConfig,
    complexity: PlanComplexity,
    verify_steps: Vec<VerifyStep>,
    baseline_failed_gates: Option<Vec<String>>,
    timeout_secs: u64,
    gate_tx: mpsc::Sender<GateCompletion>,
    gate_sem: Arc<Semaphore>,
    target_crates: Vec<String>,
) -> (JoinHandle<()>, oneshot::Sender<()>) {
    let (start_tx, start_rx) = oneshot::channel();
    let handle = tokio::spawn(async move {
        if start_rx.await.is_err() {
            return;
        }
        let failure_effect = effect.clone();
        let failure_plan = plan_id.clone();
        let failure_task = task_id.clone();
        let worker = AssertUnwindSafe(async move {
            let t_wait = Instant::now();
            let _permit = gate_sem
                .acquire_owned()
                .await
                .map_err(|_| "gate semaphore closed before acquisition".to_string())?;
            let wait_ms = t_wait.elapsed().as_millis() as u64;
            if wait_ms > 10 {
                info!(plan_id = %plan_id, task_id = %task_id, rung, wait_ms,
                    "gate semaphore acquired");
            }
            Ok::<_, String>(
                run_gate_once(
                    effect,
                    plan_id,
                    task_id,
                    rung,
                    workdir,
                    gates_config,
                    complexity,
                    verify_steps,
                    baseline_failed_gates,
                    timeout_secs,
                    target_crates,
                )
                .await,
            )
        })
        .catch_unwind()
        .await;
        let completion = match worker {
            Ok(Ok(completion)) => completion,
            Ok(Err(message)) => {
                failed_gate_completion(failure_effect, failure_plan, failure_task, rung, message)
            }
            Err(_) => failed_gate_completion(
                failure_effect,
                failure_plan,
                failure_task,
                rung,
                "gate producer panicked".to_string(),
            ),
        };

        if let Err(e) = gate_tx.send(completion).await {
            error!(err = %e, "failed to send gate completion — channel closed");
            return;
        }
    });
    (handle, start_tx)
}

fn failed_gate_completion(
    effect: GateEffectRef,
    plan_id: String,
    task_id: String,
    rung: u32,
    message: String,
) -> GateCompletion {
    GateCompletion {
        kind: effect.kind,
        attempt: Some(effect.attempt.clone()),
        effect: Some(effect),
        plan_id,
        task_id,
        rung,
        passed: false,
        failure_kind: Some(RunnerFailureKind::Resource),
        verdicts: Vec::new(),
        output: message,
        duration_ms: 0,
    }
}

/// Run a gate rung to completion and return its summary.
pub async fn run_gate_once(
    effect: GateEffectRef,
    plan_id: String,
    task_id: String,
    rung: u32,
    workdir: PathBuf,
    gates_config: GatesConfig,
    complexity: PlanComplexity,
    verify_steps: Vec<VerifyStep>,
    baseline_failed_gates: Option<Vec<String>>,
    timeout_secs: u64,
    target_crates: Vec<String>,
) -> GateCompletion {
    let start = Instant::now();
    let signal = gate_signal(&plan_id, &task_id, rung, &workdir, &target_crates);
    let ctx = roko_core::Context::now();
    let limit = Duration::from_secs(timeout_secs.max(1));

    info!(
        plan_id = %plan_id,
        task_id = %task_id,
        rung,
        timeout_secs,
        verify_step_count = verify_steps.len(),
        "gate rung starting"
    );

    let workdir_for_run = workdir.clone();
    let run = async {
        let inputs = RungExecutionInputs::default();
        let config = RungExecutionConfig {
            source_roots: Some(vec![workdir_for_run]),
            timeout_ms: Some(timeout_secs.saturating_mul(1000)),
            ..Default::default()
        };
        let pipeline = if gates_config.has_custom_rungs() {
            GatePipelineBuilder::from_config(&gates_config, complexity)
        } else {
            GatePipelineBuilder::from_config_with_execution(
                &gates_config,
                complexity,
                inputs,
                config,
            )
        };

        let task_verify_only = std::env::var("ROKO_TASK_VERIFY_ONLY").is_ok_and(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes"
            )
        });
        let mut verdicts = if task_verify_only {
            Vec::new()
        } else {
            vec![pipeline.verify(&signal, &ctx).await]
        };
        verdicts.extend(run_verify_steps(&signal, &ctx, &task_id, verify_steps).await);
        verdicts
    };

    let checked = async {
        let before = gate_input_snapshot(workdir.clone()).await?;
        let verdicts = run.await;
        let after = gate_input_snapshot(workdir.clone()).await?;
        Ok::<_, String>((before, after, verdicts))
    };
    let (input_before, mut verdicts) = match timeout(limit, checked).await {
        Ok(Ok((before, after, mut verdicts))) => {
            if before != after {
                verdicts.push(proof_failure!(
                    "unattributed:immutable-input",
                    format!(
                        "gate input changed during verification (base {} -> {})",
                        before.0, after.0
                    ),
                    "gate input mutation invalidates attribution",
                ));
            }
            (Some(before), verdicts)
        }
        Ok(Err(error)) => (
            None,
            vec![proof_failure!(
                "unattributed:input-snapshot",
                format!("could not prove immutable gate input: {error}"),
                "gate input identity unavailable",
            )],
        ),
        Err(_) => (
            None,
            vec![proof_failure!(
                format!("unattributed:gate-timeout:rung-{rung}"),
                format!("gate timed out after {timeout_secs}s"),
                format!("timeout: gate rung {rung} exceeded {timeout_secs}s"),
            )],
        ),
    };
    if let Some(before) = input_before.as_ref() {
        for verdict in verdicts
            .iter_mut()
            .filter(|verdict| !verdict.passed && !verdict.gate.starts_with("unattributed:"))
        {
            let input = gate_failure_input(
                effect.kind,
                before,
                baseline_failed_gates.as_deref(),
                &verdict.gate,
            );
            verdict.gate = format!("{input}:{}", verdict.gate);
            verdict.reason = format!("{input} failure: {}", verdict.reason);
        }
    }
    let duration_ms = start.elapsed().as_millis() as u64;

    let passed = verdicts.iter().all(|v| v.passed);
    let output = render_output(&verdicts);
    let failure_kind = (!passed).then(|| classify_failure_kind(&verdicts, &output));

    let summaries: Vec<GateVerdictSummary> = verdicts
        .iter()
        .map(|v| GateVerdictSummary {
            gate_name: v.gate.clone(),
            passed: v.passed,
            summary: v.reason.clone(),
            error_digest: v.error_digest.clone(),
            failure_kind: (!v.passed)
                .then(|| classify_failure_kind(std::slice::from_ref(v), &v.reason)),
        })
        .collect();

    let preview_limit = if passed { 500 } else { 4_000 };
    let output_preview: String = output.chars().take(preview_limit).collect();
    let verdict_names: Vec<&str> = summaries.iter().map(|v| v.gate_name.as_str()).collect();
    info!(
        plan_id = %plan_id,
        task_id = %task_id,
        rung,
        passed,
        duration_ms,
        verdict_count = summaries.len(),
        verdicts = ?verdict_names,
        output_preview = %output_preview,
        "gate completed"
    );

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
        duration_ms,
    }
}

/// Spawn plan-level verify steps as a background task.
pub fn spawn_plan_verify(
    effect: GateEffectRef,
    plan_id: String,
    workdir: PathBuf,
    expected_oid: String,
    verify_steps: Vec<(String, Vec<VerifyStep>)>,
    timeout_secs: u64,
    gate_tx: mpsc::Sender<GateCompletion>,
    gate_sem: Arc<Semaphore>,
) -> (JoinHandle<()>, oneshot::Sender<()>) {
    let (start_tx, start_rx) = oneshot::channel();
    let handle = tokio::spawn(async move {
        if start_rx.await.is_err() {
            return;
        }
        let failure_effect = effect.clone();
        let failure_plan = plan_id.clone();
        let worker = AssertUnwindSafe(async move {
            let t_wait = Instant::now();
            let _permit = gate_sem
                .acquire_owned()
                .await
                .map_err(|_| "plan verify semaphore closed before acquisition".to_string())?;
            let wait_ms = t_wait.elapsed().as_millis() as u64;
            if wait_ms > 10 {
                info!(
                    plan_id = %plan_id,
                    wait_ms,
                    "plan verify semaphore acquired"
                );
            }
            let start = Instant::now();
            let ctx = roko_core::Context::now();
            let limit = Duration::from_secs(timeout_secs.max(1));
            let plan_id_for_run = plan_id.clone();
            let workdir_for_run = workdir.clone();

            let run = async move {
                let before =
                    match accepted_input_snapshot(workdir_for_run.clone(), &expected_oid).await {
                        Ok(snapshot) => snapshot,
                        Err(error) => return vec![Verdict::fail("accepted-plan:input", error)],
                    };
                let mut all = Vec::new();
                for (task_id, steps) in verify_steps {
                    let signal = gate_signal(
                        &plan_id_for_run,
                        &task_id,
                        RUNG_PLAN_VERIFY,
                        &workdir_for_run,
                        &[], // plan-level verify runs workspace-wide
                    );
                    all.extend(run_verify_steps(&signal, &ctx, &task_id, steps).await);
                }
                if accepted_input_snapshot(workdir_for_run, &expected_oid).await != Ok(before) {
                    all.push(Verdict::fail(
                        "accepted-plan:immutable-input",
                        "accepted plan input changed during verification",
                    ));
                }
                all
            };

            let verdicts = match timeout(limit, run).await {
                Ok(verdicts) => verdicts,
                Err(_) => vec![
                    Verdict::fail(
                        "plan-verify-timeout",
                        format!("plan verify timed out after {timeout_secs}s"),
                    )
                    .with_error_digest(format!("timeout: plan verify exceeded {timeout_secs}s")),
                ],
            };
            let duration_ms = start.elapsed().as_millis() as u64;
            let passed = verdicts.iter().all(|v| v.passed);
            let output = render_output(&verdicts);
            let failure_kind = (!passed).then(|| classify_failure_kind(&verdicts, &output));
            let summaries = verdicts
                .iter()
                .map(|v| GateVerdictSummary {
                    gate_name: v.gate.clone(),
                    passed: v.passed,
                    summary: v.reason.clone(),
                    error_digest: v.error_digest.clone(),
                    failure_kind: (!v.passed)
                        .then(|| classify_failure_kind(std::slice::from_ref(v), &v.reason)),
                })
                .collect();

            info!(
                plan_id = %plan_id,
                passed,
                duration_ms,
                "plan verify completed"
            );

            Ok::<_, String>(GateCompletion {
                kind: GateCompletionKind::PlanVerify,
                attempt: Some(effect.attempt.clone()),
                effect: Some(effect),
                plan_id,
                task_id: "plan-verify".to_string(),
                rung: RUNG_PLAN_VERIFY,
                passed,
                failure_kind,
                verdicts: summaries,
                output,
                duration_ms,
            })
        })
        .catch_unwind()
        .await;
        let completion = match worker {
            Ok(Ok(completion)) => completion,
            Ok(Err(message)) => failed_gate_completion(
                failure_effect,
                failure_plan,
                "plan-verify".to_string(),
                RUNG_PLAN_VERIFY,
                message,
            ),
            Err(_) => failed_gate_completion(
                failure_effect,
                failure_plan,
                "plan-verify".to_string(),
                RUNG_PLAN_VERIFY,
                "plan verify producer panicked".to_string(),
            ),
        };

        if let Err(e) = gate_tx.send(completion).await {
            error!(err = %e, "failed to send plan verify completion — channel closed");
        }
    });
    (handle, start_tx)
}

fn gate_signal(
    plan_id: &str,
    task_id: &str,
    rung: u32,
    workdir: &std::path::Path,
    target_crates: &[String],
) -> Engram {
    let attempt_sentinel = RokoLayout::for_project(workdir)
        .gate_attempts_dir()
        .join(format!(
            "{}-{}-{rung}.seen",
            sanitize_gate_env_segment(plan_id),
            sanitize_gate_env_segment(task_id)
        ));
    let payload = GatePayload::in_dir(workdir)
        .with_label(format!("{plan_id}:{task_id}:rung-{rung}"))
        .with_target_crates(target_crates.to_vec())
        .with_env("ROKO_GATE_PLAN_ID", plan_id)
        .with_env("ROKO_GATE_TASK_ID", task_id)
        .with_env("ROKO_GATE_RUNG", rung.to_string())
        .with_env(
            "ROKO_GATE_ATTEMPT_SENTINEL",
            attempt_sentinel.to_string_lossy().to_string(),
        );

    EngramBuilder::new(Kind::Task)
        .body(Body::from_json(&payload).unwrap_or_else(|_| Body::empty()))
        .provenance(Provenance::trusted("runner"))
        .tag("plan_id", plan_id.to_string())
        .tag("task_id", task_id.to_string())
        .tag("rung", rung.to_string())
        .build()
}

fn sanitize_gate_env_segment(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    if sanitized.is_empty() {
        "unknown".to_string()
    } else {
        sanitized
    }
}

async fn run_verify_steps(
    signal: &Engram,
    ctx: &roko_core::Context,
    task_id: &str,
    verify_steps: Vec<VerifyStep>,
) -> Vec<Verdict> {
    let mut verdicts = Vec::new();
    for (i, step) in verify_steps.iter().enumerate() {
        let step_start = Instant::now();
        let gate = verify_step_gate(task_id, step);
        let verdict = gate.verify(signal, ctx).await;
        info!(
            task_id = %task_id,
            step = i + 1,
            total_steps = verify_steps.len(),
            phase = %step.phase,
            command = %step.command,
            timeout_ms = step.timeout_ms,
            passed = verdict.passed,
            elapsed_ms = step_start.elapsed().as_millis() as u64,
            "verify step completed"
        );
        verdicts.push(verdict);
    }
    verdicts
}

fn verify_step_gate(task_id: &str, step: &VerifyStep) -> ShellGate {
    ShellGate::new(
        "bash",
        vec![
            "-o".into(),
            "pipefail".into(),
            "-c".into(),
            step.command.clone(),
        ],
    )
    .with_name(format!("task-verify:{}:{}", task_id, step.phase))
    .with_timeout_ms(step.timeout_ms)
}

fn render_output(verdicts: &[Verdict]) -> String {
    verdicts
        .iter()
        .map(render_verdict_output)
        .collect::<Vec<_>>()
        .join("; ")
}

fn render_verdict_output(v: &Verdict) -> String {
    let status = if v.passed { "pass" } else { "FAIL" };
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

fn classify_failure_kind(verdicts: &[Verdict], output: &str) -> RunnerFailureKind {
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
                    | RunnerFailureKind::Unknown => RunnerFailureKind::Structural,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for args in [
            vec!["init", "-b", "main"],
            vec!["config", "user.name", "Roko Test"],
            vec!["config", "user.email", "roko@example.invalid"],
            vec!["commit", "--allow-empty", "-m", "base"],
        ] {
            let output = std::process::Command::new("git")
                .args(args)
                .current_dir(dir.path())
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "git setup failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        dir
    }

    fn gate_effect(kind: GateCompletionKind) -> GateEffectRef {
        GateEffectRef {
            attempt: TaskAttemptRef::new("plan", "task", 1),
            kind,
            rung: 1,
            generation: 1,
        }
    }

    fn barrier_gate() -> (
        JoinHandle<()>,
        oneshot::Sender<()>,
        mpsc::Receiver<GateCompletion>,
    ) {
        let dir = tempfile::tempdir().expect("tempdir");
        let workdir = dir.keep();
        let (tx, rx) = mpsc::channel(1);
        let effect = GateEffectRef {
            attempt: TaskAttemptRef::new("plan", "task", 1),
            kind: GateCompletionKind::Gate,
            rung: 1,
            generation: 99,
        };
        let (handle, start) = spawn_gate(
            effect,
            "plan".to_string(),
            "task".to_string(),
            1,
            workdir,
            GatesConfig::default(),
            PlanComplexity::Trivial,
            Vec::new(),
            None,
            1,
            tx,
            Arc::new(Semaphore::new(1)),
            Vec::new(),
        );
        (handle, start, rx)
    }

    #[tokio::test]
    async fn gate_producer_waits_for_owner_start_barrier() {
        let (handle, start, mut rx) = barrier_gate();
        tokio::task::yield_now().await;
        assert!(matches!(
            rx.try_recv(),
            Err(mpsc::error::TryRecvError::Empty)
        ));
        drop(start);
        handle.await.expect("barrier cancellation should be clean");
    }

    #[tokio::test]
    async fn gate_start_reports_failure_after_producer_abort() {
        let (handle, start, _rx) = barrier_gate();
        handle.abort();
        let _ = handle.await;
        assert!(start.send(()).is_err());
    }

    #[tokio::test]
    async fn plan_verify_is_barriered_and_preserves_exact_effect() {
        let shared_root = git_repo();
        std::fs::write(shared_root.path().join("unrelated.txt"), b"dirty root\n").unwrap();
        let dir = git_repo();
        let expected_oid = String::from_utf8_lossy(
            &std::process::Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(dir.path())
                .output()
                .unwrap()
                .stdout,
        )
        .trim()
        .to_string();
        let effect = GateEffectRef {
            attempt: TaskAttemptRef::new("plan-a", "plan-verify", 1),
            kind: GateCompletionKind::PlanVerify,
            rung: RUNG_PLAN_VERIFY,
            generation: 501,
        };
        let (tx, mut rx) = mpsc::channel(1);
        let (handle, start) = spawn_plan_verify(
            effect.clone(),
            "plan-a".to_string(),
            dir.path().to_path_buf(),
            expected_oid,
            Vec::new(),
            1,
            tx,
            Arc::new(Semaphore::new(1)),
        );
        tokio::task::yield_now().await;
        assert!(matches!(
            rx.try_recv(),
            Err(mpsc::error::TryRecvError::Empty)
        ));
        start.send(()).unwrap();
        let completion = rx.recv().await.unwrap();
        handle.await.unwrap();
        assert!(completion.passed);
        assert_eq!(completion.effect, Some(effect));
        assert!(shared_root.path().join("unrelated.txt").exists());
    }

    #[tokio::test]
    async fn closed_plan_verify_semaphore_emits_exact_resource_failure() {
        let dir = tempfile::tempdir().unwrap();
        let effect = GateEffectRef {
            attempt: TaskAttemptRef::new("plan-b", "plan-verify", 1),
            kind: GateCompletionKind::PlanVerify,
            rung: RUNG_PLAN_VERIFY,
            generation: 502,
        };
        let semaphore = Arc::new(Semaphore::new(0));
        semaphore.close();
        let (tx, mut rx) = mpsc::channel(1);
        let (handle, start) = spawn_plan_verify(
            effect.clone(),
            "plan-b".to_string(),
            dir.path().to_path_buf(),
            "unused".to_string(),
            Vec::new(),
            1,
            tx,
            semaphore,
        );
        start.send(()).unwrap();
        let completion = rx.recv().await.unwrap();
        handle.await.unwrap();
        assert!(!completion.passed);
        assert_eq!(completion.failure_kind, Some(RunnerFailureKind::Resource));
        assert_eq!(completion.effect, Some(effect));
    }

    #[tokio::test]
    async fn closed_semaphore_emits_exact_failed_preflight_completion() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (tx, mut rx) = mpsc::channel(1);
        let semaphore = Arc::new(Semaphore::new(1));
        semaphore.close();
        let effect = GateEffectRef {
            attempt: TaskAttemptRef::new("plan", "task", 2),
            kind: GateCompletionKind::Preflight,
            rung: 3,
            generation: 101,
        };
        let (handle, start) = spawn_gate(
            effect.clone(),
            "plan".to_string(),
            "task".to_string(),
            3,
            dir.path().to_path_buf(),
            GatesConfig::default(),
            PlanComplexity::Trivial,
            Vec::new(),
            None,
            1,
            tx,
            semaphore,
            Vec::new(),
        );

        start.send(()).expect("owner starts producer");
        let completion = rx.recv().await.expect("structured failure completion");
        handle.await.expect("supervisor exits cleanly");
        assert!(!completion.passed);
        assert_eq!(completion.kind, GateCompletionKind::Preflight);
        assert_eq!(completion.attempt.as_ref(), Some(&effect.attempt));
        assert_eq!(completion.effect.as_ref(), Some(&effect));
        assert_eq!(completion.failure_kind, Some(RunnerFailureKind::Resource));
        assert!(completion.output.contains("semaphore closed"));
    }

    #[test]
    fn retry_recommended_gate_digest_remains_retryable() {
        let digest = r#"{
  "gate": "task-verify:C01:structural",
  "primary": "unknown",
  "failure_kind": "permanent",
  "retry_policy": {
    "retryable": true,
    "cooldown_secs": 0,
    "include_error_digest": true,
    "generate_reflection": true,
    "regenerate_verify": false
  },
  "summary": "exit code: 1",
  "classes": ["unknown"],
  "compile_errors": [],
  "error_count": 0,
  "warning_count": 0,
  "cargo_fix_candidate": false,
  "agent_retry_needed": true,
  "recommended_action": "retry",
  "replan_candidate": false,
  "blocking_findings": [],
  "duration_ms": 10,
  "raw_excerpt": ""
}"#;
        let verdict =
            Verdict::fail("task-verify:C01:structural", "exit code: 1").with_error_digest(digest);

        let kind = classify_failure_kind(&[verdict], "");

        assert_eq!(kind, RunnerFailureKind::Structural);
        assert!(kind.is_retryable());
    }

    #[test]
    fn failed_gate_output_prefers_command_detail_before_classification() {
        let verdict = Verdict::fail("task-verify:V03:test", "exit code: 1")
            .with_detail("failures:\n    workspace_tests::regression\n")
            .with_error_digest(r#"{"recommended_action":"retry"}"#);

        let rendered = render_output(&[verdict]);

        assert!(rendered.contains("workspace_tests::regression"));
        assert!(rendered.contains("classification:"));
        assert!(
            rendered.find("workspace_tests::regression").unwrap()
                < rendered.find("classification:").unwrap()
        );
    }

    #[tokio::test]
    async fn verify_steps_fail_when_a_piped_command_fails_before_tail() {
        let tempdir = tempfile::tempdir().expect("tempdir should be created");
        let signal = gate_signal("plan", "task", 2, tempdir.path(), &[]);
        let ctx = roko_core::Context::now();
        let step = VerifyStep {
            phase: "test".to_string(),
            command: "false | tail -1".to_string(),
            fail_msg: None,
            timeout_ms: 10_000,
        };

        let verdicts = run_verify_steps(&signal, &ctx, "T01", vec![step]).await;

        assert_eq!(verdicts.first().map(|verdict| verdict.passed), Some(false));
    }

    #[tokio::test]
    async fn verify_steps_pass() {
        let tempdir = tempfile::tempdir().expect("tempdir should be created");
        let signal = gate_signal("plan", "task", 2, tempdir.path(), &[]);
        let ctx = roko_core::Context::now();
        let step = VerifyStep {
            phase: "structural".to_string(),
            command: "true".to_string(),
            fail_msg: None,
            timeout_ms: 10_000,
        };

        let verdicts = run_verify_steps(&signal, &ctx, "T01", vec![step]).await;

        assert_eq!(verdicts.first().map(|verdict| verdict.passed), Some(true));
    }

    #[tokio::test]
    async fn failures_identify_actual_baseline_and_owned_diff_inputs() {
        for (owned, expected) in [(false, "baseline:"), (true, "owned-diff:")] {
            let dir = git_repo();
            if owned {
                std::fs::write(dir.path().join("candidate.txt"), b"owned\n").unwrap();
            }
            let completion = run_gate_once(
                gate_effect(GateCompletionKind::Gate),
                "plan".into(),
                "task".into(),
                1,
                dir.path().to_path_buf(),
                GatesConfig::default(),
                PlanComplexity::Trivial,
                vec![VerifyStep {
                    phase: "test".into(),
                    command: "false".into(),
                    fail_msg: None,
                    timeout_ms: 10_000,
                }],
                Some(Vec::new()),
                10,
                Vec::new(),
            )
            .await;
            assert!(!completion.passed);
            assert!(
                completion
                    .verdicts
                    .iter()
                    .filter(|verdict| !verdict.passed)
                    .all(|verdict| verdict.gate_name.starts_with(expected))
            );
        }
    }

    #[tokio::test]
    async fn gate_fails_closed_when_verification_mutates_owned_input() {
        let dir = git_repo();
        std::fs::write(dir.path().join("tracked.txt"), b"before\n").unwrap();
        for args in [
            vec!["add", "tracked.txt"],
            vec!["commit", "-m", "tracked input"],
        ] {
            assert!(
                std::process::Command::new("git")
                    .args(args)
                    .current_dir(dir.path())
                    .status()
                    .unwrap()
                    .success()
            );
        }
        let completion = run_gate_once(
            gate_effect(GateCompletionKind::Gate),
            "plan".into(),
            "task".into(),
            1,
            dir.path().to_path_buf(),
            GatesConfig::default(),
            PlanComplexity::Trivial,
            vec![VerifyStep {
                phase: "test".into(),
                command: "printf 'after\\n' > tracked.txt".into(),
                fail_msg: None,
                timeout_ms: 10_000,
            }],
            Some(Vec::new()),
            10,
            Vec::new(),
        )
        .await;

        assert!(!completion.passed);
        assert!(
            completion
                .verdicts
                .iter()
                .any(|verdict| verdict.gate_name == "unattributed:immutable-input")
        );
    }

    #[tokio::test]
    async fn failure_reproduced_on_baseline_retains_both_identities() {
        let dir = git_repo();
        let step = VerifyStep {
            phase: "test".into(),
            command: "false".into(),
            fail_msg: None,
            timeout_ms: 10_000,
        };
        let baseline = run_gate_once(
            gate_effect(GateCompletionKind::Preflight),
            "plan".into(),
            "task".into(),
            1,
            dir.path().to_path_buf(),
            GatesConfig::default(),
            PlanComplexity::Trivial,
            vec![step.clone()],
            None,
            10,
            Vec::new(),
        )
        .await;
        let baseline_failures = baseline
            .verdicts
            .iter()
            .filter(|verdict| !verdict.passed)
            .map(|verdict| raw_gate_name(&verdict.gate_name).to_string())
            .collect();
        std::fs::write(dir.path().join("candidate.txt"), b"owned\n").unwrap();
        let candidate = run_gate_once(
            gate_effect(GateCompletionKind::Gate),
            "plan".into(),
            "task".into(),
            1,
            dir.path().to_path_buf(),
            GatesConfig::default(),
            PlanComplexity::Trivial,
            vec![step],
            Some(baseline_failures),
            10,
            Vec::new(),
        )
        .await;
        assert!(candidate.verdicts.iter().any(|verdict| {
            !verdict.passed && verdict.gate_name.starts_with("baseline+owned:")
        }));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn equal_content_symlink_target_swap_invalidates_gate_input() {
        use std::os::unix::fs::symlink;

        let dir = git_repo();
        std::fs::write(dir.path().join("a"), b"same\n").unwrap();
        std::fs::write(dir.path().join("b"), b"same\n").unwrap();
        symlink("a", dir.path().join("input")).unwrap();
        let completion = run_gate_once(
            gate_effect(GateCompletionKind::Gate),
            "plan".into(),
            "task".into(),
            1,
            dir.path().to_path_buf(),
            GatesConfig::default(),
            PlanComplexity::Trivial,
            vec![VerifyStep {
                phase: "test".into(),
                command: "ln -sfn b input".into(),
                fail_msg: None,
                timeout_ms: 10_000,
            }],
            Some(Vec::new()),
            10,
            Vec::new(),
        )
        .await;
        assert!(!completion.passed);
        assert!(
            completion
                .verdicts
                .iter()
                .any(|verdict| { verdict.gate_name == "unattributed:immutable-input" })
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlink_cycle_is_not_traversed() {
        use std::os::unix::fs::symlink;

        let dir = git_repo();
        symlink(".", dir.path().join("cycle")).unwrap();
        gate_input_snapshot_blocking(dir.path()).expect("symlink cycle remains a link input");
    }

    #[cfg(unix)]
    #[test]
    fn equal_kind_len_mtime_inode_replacement_is_detected() {
        use std::os::unix::fs::MetadataExt;

        let dir = git_repo();
        let input = dir.path().join("input");
        let replacement = dir.path().join("replacement");
        std::fs::write(&input, b"aaaa").unwrap();
        std::fs::write(&replacement, b"bbbb").unwrap();
        let before = std::fs::symlink_metadata(&input).unwrap();
        OpenOptions::new()
            .write(true)
            .open(&replacement)
            .unwrap()
            .set_times(std::fs::FileTimes::new().set_modified(before.modified().unwrap()))
            .unwrap();
        std::fs::rename(&replacement, &input).unwrap();
        let after = std::fs::symlink_metadata(&input).unwrap();

        assert_eq!(before.file_type(), after.file_type());
        assert_eq!(before.len(), after.len());
        assert_eq!(before.modified().unwrap(), after.modified().unwrap());
        assert_eq!(before.dev(), after.dev());
        assert_ne!(before.ino(), after.ino());
        assert!(!metadata_unchanged(&before, &after));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn bare_fifo_is_rejected_without_scanning_ignored_build_artifacts() {
        let dir = git_repo();
        std::fs::write(dir.path().join(".gitignore"), "ignored-build/\n").unwrap();
        std::fs::create_dir(dir.path().join("ignored-build")).unwrap();
        assert!(
            std::process::Command::new("mkfifo")
                .arg("ignored-build/cache.fifo")
                .current_dir(dir.path())
                .status()
                .unwrap()
                .success()
        );
        gate_input_snapshot_blocking(dir.path()).expect("ignored artifacts are pruned");
        assert!(
            std::process::Command::new("mkfifo")
                .arg("input.fifo")
                .current_dir(dir.path())
                .status()
                .unwrap()
                .success()
        );
        assert!(
            gate_input_snapshot_blocking(dir.path())
                .unwrap_err()
                .contains("non-file input")
        );
        let completion = tokio::time::timeout(
            Duration::from_secs(2),
            run_gate_once(
                gate_effect(GateCompletionKind::Gate),
                "plan".into(),
                "task".into(),
                1,
                dir.path().to_path_buf(),
                GatesConfig::default(),
                PlanComplexity::Trivial,
                Vec::new(),
                Some(Vec::new()),
                1,
                Vec::new(),
            ),
        )
        .await
        .expect("FIFO fingerprinting must not block");
        assert!(!completion.passed);
        assert!(completion.duration_ms < 2_000, "{completion:#?}");
        assert!(completion.verdicts.iter().any(|verdict| {
            verdict.gate_name == "unattributed:input-snapshot"
                && verdict.summary.contains("non-file input")
        }));
    }

    #[tokio::test]
    async fn untracked_size_and_count_limits_are_deterministic() {
        let oversized = git_repo();
        File::create(oversized.path().join("large.bin"))
            .unwrap()
            .set_len(MAX_UNTRACKED_FILE_BYTES + 1)
            .unwrap();
        assert!(
            gate_input_snapshot_blocking(oversized.path())
                .unwrap_err()
                .contains("exceeds input limit")
        );
        let completion = run_gate_once(
            gate_effect(GateCompletionKind::Gate),
            "plan".into(),
            "task".into(),
            1,
            oversized.path().to_path_buf(),
            GatesConfig::default(),
            PlanComplexity::Trivial,
            Vec::new(),
            Some(Vec::new()),
            1,
            Vec::new(),
        )
        .await;
        assert!(
            completion
                .verdicts
                .iter()
                .any(|verdict| { verdict.gate_name == "unattributed:input-snapshot" })
        );

        let counted = git_repo();
        for index in 0..MAX_UNTRACKED_FILES {
            File::create(counted.path().join(format!("item-{index:04}"))).unwrap();
        }
        gate_input_snapshot_blocking(counted.path()).expect("count boundary is accepted");
        File::create(counted.path().join("one-too-many")).unwrap();
        assert!(
            gate_input_snapshot_blocking(counted.path())
                .unwrap_err()
                .contains("untracked file count")
        );
    }
}
