//! The whole-plan gate (gap-60233f): `[meta] verify`.
//!
//! A task's verify steps check that task. Once every task of a plan has
//! passed, the plan's `[meta] verify` steps check the plan's integrated
//! result: under `--worktree-per-task` the merge of its branch into the run's
//! batch branch, in the repository's regression checkout (the delivery's
//! regression check, see [`super::batch`]); otherwise the shared working tree
//! its tasks edited.
//! A plan whose tasks all passed but whose check fails does not succeed.
//!
//! A plan without `[meta] verify` in a Cargo workspace checks formatting,
//! lints and tests over the crates it affects: the ones its tasks write, and
//! every workspace crate that depends on them. That default never runs
//! `cargo test --workspace`. Other projects get no default.

use std::path::Path;
use std::time::Duration;

use serde::Serialize;

use super::plan_set::{Area, PlanFootprint};
use crate::runner::plan_loader::Plan;
use crate::task_parser::VerifyStep;

/// The steps that check `plan`'s integrated result: its `[meta] verify`
/// steps when it has some, else [`default_plan_verify`] of its `footprint`,
/// which is `None` outside a Cargo workspace.
#[must_use]
pub fn plan_verify_steps(plan: &Plan, footprint: Option<&PlanFootprint>) -> Vec<VerifyStep> {
    if !plan.tasks.meta.verify.is_empty() {
        return plan.tasks.meta.verify.clone();
    }
    footprint.map(default_plan_verify).unwrap_or_default()
}

/// The default whole-plan check in a Cargo workspace: `cargo fmt --check`,
/// then clippy with `-D warnings`, then `cargo test`, each over the crates
/// the plan affects. When the package graph is unknown, formatting and lints
/// cover the workspace and no tests run. A plan that affects no crate gets
/// no steps.
#[must_use]
pub fn default_plan_verify(footprint: &PlanFootprint) -> Vec<VerifyStep> {
    let all_packages = footprint.affects.contains(&Area::AllPackages);
    let packages = footprint
        .affects
        .iter()
        .filter_map(|area| match area {
            Area::Package { name, .. } => Some(format!("-p {name}")),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ");
    if all_packages {
        return vec![
            step("fmt", "cargo fmt --all -- --check"),
            step("lint", "cargo clippy --workspace --no-deps -- -D warnings"),
        ];
    }
    if packages.is_empty() {
        return Vec::new();
    }
    vec![
        step("fmt", format!("cargo fmt {packages} -- --check")),
        step(
            "lint",
            format!("cargo clippy {packages} --no-deps -- -D warnings"),
        ),
        step("test", format!("cargo test {packages}")),
    ]
}

fn step(phase: &str, command: impl Into<String>) -> VerifyStep {
    VerifyStep {
        phase: phase.to_string(),
        command: command.into(),
        fail_msg: None,
        timeout_ms: crate::task_parser::default_verify_timeout(),
        scope: Vec::new(),
    }
}

/// Why a plan's check failed: the step, and the end of its output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlanVerifyFailure {
    /// The failed step's phase.
    pub phase: String,
    /// The failed step's command.
    pub command: String,
    /// The last lines the step wrote, or why it did not finish.
    pub output: String,
}

impl std::fmt::Display for PlanVerifyFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "`{}` failed: {}", self.command, self.output)
    }
}

/// Run `steps` in order in `dir` (`sh -c`), each within its timeout, and
/// stop at the first that fails.
///
/// # Errors
///
/// The first step that exits unsuccessfully, cannot start, or runs out of
/// time.
pub async fn run_plan_verify(dir: &Path, steps: &[VerifyStep]) -> Result<(), PlanVerifyFailure> {
    for step in steps {
        let failure = |output: String| PlanVerifyFailure {
            phase: step.phase.clone(),
            command: step.command.clone(),
            output,
        };
        let run = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(&step.command)
            .current_dir(dir)
            .kill_on_drop(true)
            .output();
        let timeout = Duration::from_millis(step.timeout_ms.max(1));
        let output = match tokio::time::timeout(timeout, run).await {
            Ok(Ok(output)) => output,
            Ok(Err(error)) => return Err(failure(format!("could not start: {error}"))),
            Err(_) => {
                return Err(failure(format!("timed out after {}s", timeout.as_secs())));
            }
        };
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let text = if stderr.trim().is_empty() {
                stdout
            } else {
                stderr
            };
            let lines: Vec<&str> = text
                .lines()
                .filter(|line| !line.trim().is_empty())
                .collect();
            let tail = lines[lines.len().saturating_sub(5)..].join(" | ");
            return Err(failure(match step.fail_msg.as_deref() {
                Some(message) => format!("{message}: {tail}"),
                None => tail,
            }));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn verify_step(command: &str) -> VerifyStep {
        step("plan", command)
    }

    /// gap-60233f: the steps run in order, and the first that fails stops
    /// the check and says why.
    #[tokio::test]
    async fn plan_verify_stops_at_the_first_failed_step() {
        let dir = tempfile::tempdir().expect("tempdir");
        let steps = [
            verify_step("touch first"),
            verify_step("echo 'crate b no longer builds' >&2; exit 1"),
            verify_step("touch never"),
        ];

        let failure = run_plan_verify(dir.path(), &steps)
            .await
            .expect_err("the second step fails");

        assert_eq!(failure.command, steps[1].command);
        assert!(
            failure.output.contains("crate b no longer builds"),
            "{failure}"
        );
        assert!(dir.path().join("first").exists());
        assert!(!dir.path().join("never").exists());
        assert!(run_plan_verify(dir.path(), &steps[..1]).await.is_ok());
    }
}
