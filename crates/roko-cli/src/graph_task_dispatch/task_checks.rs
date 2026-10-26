//! A task's checks, left for its agent's `run_tests` with `build: "checks"`
//! (gap-770cb5), while the attempt's dispatch runs.

use roko_std::tool::builtin::task_checks::{TaskCheck, task_check_runs_path, task_checks_path};

use super::*;

/// `task`'s verify steps as its agent's `run_tests` runs them, named as their
/// gate results name them.
pub(super) fn checks_of(task: &TaskDef) -> Vec<TaskCheck> {
    task.verify
        .iter()
        .enumerate()
        .map(|(index, step)| TaskCheck {
            label: verification::verify_step_label(index, &step.phase),
            command: step.command.clone(),
            timeout_ms: Some(step.timeout_ms),
        })
        .collect()
}

/// The checks file an attempt left in the workspace, and the record of its
/// checks' runs beside it (gap-56f761); both removed when dropped, once the
/// attempt is judged.
pub(super) struct LeftChecks {
    checks: Option<PathBuf>,
    runs: Option<PathBuf>,
}

impl Drop for LeftChecks {
    fn drop(&mut self) {
        for path in [self.checks.take(), self.runs.take()].into_iter().flatten() {
            let _ = std::fs::remove_file(path);
        }
    }
}

impl GraphTaskDispatcher {
    /// Leave `task`'s verify steps, named as their gate results name them,
    /// where the agent of the attempt `attempt_key` finds them: under the
    /// workspace's `.roko/state/`, outside the checkout it writes. A task with
    /// none leaves nothing, and a file that cannot be written leaves the agent
    /// without the mode, as before.
    pub(super) fn leave_task_checks(&self, task: &TaskDef, attempt_key: &str) -> LeftChecks {
        if task.verify.is_empty() {
            return LeftChecks {
                checks: None,
                runs: None,
            };
        }
        let checks = checks_of(task);
        let path = task_checks_path(&self.workdir, attempt_key);
        // Runs an earlier process of the same attempt recorded are not this
        // dispatch's.
        let runs = task_check_runs_path(&self.workdir, attempt_key);
        let _ = std::fs::remove_file(&runs);
        let written = serde_json::to_vec(&checks)
            .map_err(std::io::Error::other)
            .and_then(|bytes| {
                if let Some(dir) = path.parent() {
                    std::fs::create_dir_all(dir)?;
                }
                std::fs::write(&path, bytes)
            });
        match written {
            Ok(()) => LeftChecks {
                checks: Some(path),
                runs: Some(runs),
            },
            Err(error) => {
                tracing::debug!(%error, "the task's checks were not left for run_tests");
                LeftChecks {
                    checks: None,
                    runs: None,
                }
            }
        }
    }
}
