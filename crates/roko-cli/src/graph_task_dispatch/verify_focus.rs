//! `[gates] mode = "focused"` on the Graph path (gap-1426e4).
//!
//! An attempt's authored `cargo test -p <package>` steps narrow to the one
//! Cargo target its changes touch, as the runner's focused gate mode scopes
//! them ([`scoped_test_command`]). Workspace rungs and every other step run
//! as written, and any doubt in the impact analysis leaves every step as
//! authored. A test that also fails on the plan run's start commit is
//! filtered by the baseline check either way.

use crate::runner::cargo_command::scoped_test_command;
use crate::runner::impact_analysis::ImpactReport;

use super::*;

impl GraphTaskDispatcher {
    /// `steps` for an attempt at `task` in `workdir`, with its authored Cargo
    /// test steps scoped ([`focus_steps`]) when `[gates] mode = "focused"`.
    pub(super) async fn focus_verify_steps(
        &self,
        workdir: &Path,
        task: &TaskDef,
        steps: Vec<(String, crate::task_parser::VerifyStep)>,
    ) -> Vec<(String, crate::task_parser::VerifyStep)> {
        let gates = &self.config.gates;
        if gates.mode != roko_core::config::GateMode::Focused {
            return steps;
        }
        let limit = std::time::Duration::from_millis(gates.impact_timeout_ms.max(100));
        let analysis = crate::runner::impact_analysis::analyze(workdir, &task.files, gates);
        match tokio::time::timeout(limit, analysis).await {
            Ok(report) if report.fallback_reason.is_none() => {
                focus_steps(workdir, &task.id, steps, &report)
            }
            Ok(report) => {
                tracing::info!(
                    task_id = %task.id,
                    reason = ?report.fallback_reason,
                    "focused verify keeps the authored steps"
                );
                steps
            }
            Err(_) => {
                tracing::info!(
                    task_id = %task.id,
                    "impact analysis timed out; focused verify keeps the authored steps"
                );
                steps
            }
        }
    }
}

/// `steps` with each authored step (`verify[..]`) that runs `cargo test -p
/// <package>` for `report`'s one impacted target narrowed to that target's
/// tests. Workspace rungs (`rung[..]`) stay as declared.
fn focus_steps(
    workdir: &Path,
    task_id: &str,
    steps: Vec<(String, crate::task_parser::VerifyStep)>,
    report: &ImpactReport,
) -> Vec<(String, crate::task_parser::VerifyStep)> {
    steps
        .into_iter()
        .map(|(label, mut step)| {
            if label.starts_with("verify[")
                && let Some(scoped) = scoped_test_command(workdir, &step.command, report)
            {
                tracing::info!(
                    task_id,
                    step = %label,
                    original_command = %step.command,
                    scoped_command = %scoped,
                    "focused verify scoped an authored Cargo test to the changed target"
                );
                step.command = scoped;
            }
            (label, step)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph_task_dispatch::tests::verify_step;
    use crate::runner::impact_analysis::{CargoTargetSelector, ImpactedTarget};

    /// gap-1426e4: under focused verify an authored `cargo test -p <package>`
    /// narrows to the one target the attempt changed. Workspace rungs and
    /// steps for another package run as written.
    #[test]
    fn focused_verify_scopes_an_authored_cargo_test_to_the_changed_target() {
        let report = ImpactReport {
            changed_files: vec!["crates/alpha/tests/api.rs".to_string()],
            targets: vec![ImpactedTarget {
                package: "alpha".to_string(),
                selector: CargoTargetSelector::Test("api".to_string()),
                required_features: Vec::new(),
            }],
            ..ImpactReport::default()
        };
        let steps: Vec<(String, crate::task_parser::VerifyStep)> = [
            ("verify[0:test]", "cargo test -p alpha"),
            ("verify[1:test]", "cargo test -p beta"),
            ("rung[test]", "cargo test -p alpha"),
        ]
        .into_iter()
        .map(|(label, command)| (label.to_string(), verify_step("test", command)))
        .collect();

        let commands: Vec<String> = focus_steps(Path::new("."), "T1", steps, &report)
            .into_iter()
            .map(|(_, step)| step.command)
            .collect();
        assert_eq!(
            commands,
            [
                "cargo test -p alpha --test api",
                "cargo test -p beta",
                "cargo test -p alpha",
            ]
        );
    }
}
