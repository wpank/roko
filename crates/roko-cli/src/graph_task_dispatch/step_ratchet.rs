//! Each task's verify step history across its attempts (gap-6dba88).
//!
//! A verify step that passed on an earlier attempt and fails now is a
//! regression: the thrash of a retry loop that fixes one check and breaks
//! another. Steps are told apart by their label, which carries their index
//! and phase, and a hash of their command, never by gate rung, so the order
//! a task authors its steps in is never a regression. The history lives in
//! the retry feedback book beside the Graph checkpoint: a resumed run keeps
//! it, and a fresh run or the task's pass drops it. It is advisory only. It
//! changes neither which steps run nor whether the task passes.

use std::collections::BTreeSet;

use sha2::{Digest, Sha256};

use super::*;

/// A verify step's identity across a task's attempts: its label (`verify[i]`,
/// `verify[i:phase]` or `rung[name]`) and a hash of its command.
pub(super) fn step_identity(label: &str, command: &str) -> String {
    let digest = Sha256::digest(command.trim().as_bytes());
    let hash: String = digest
        .iter()
        .take(6)
        .map(|byte| format!("{byte:02x}"))
        .collect();
    format!("{label}#{hash}")
}

/// The labels of the failing steps of `ran` (`(label, identity, passed)` for
/// each step that ran) whose identity `passed_before` holds: the steps an
/// earlier attempt passed that fail now.
pub(super) fn step_regressions(
    ran: &[(String, String, bool)],
    passed_before: &BTreeSet<String>,
) -> Vec<String> {
    ran.iter()
        .filter(|(_, identity, passed)| !passed && passed_before.contains(identity))
        .map(|(label, _, _)| label.clone())
        .collect()
}

/// `diagnosis`, led by a line naming the steps of `regressed`, for the next
/// attempt's retry feedback, which shows the diagnosis ahead of the errors.
pub(super) fn regression_note(regressed: &[String], diagnosis: &str) -> String {
    if regressed.is_empty() {
        return diagnosis.to_string();
    }
    let steps = regressed.join(", ");
    format!(
        "Regression: {steps} passed on an earlier attempt and fails now, so the last change \
         broke it.\n{diagnosis}"
    )
}

impl GraphTaskDispatcher {
    /// Check this attempt's verify run against the steps `task` passed on its
    /// earlier attempts, then add this run's passes to that history. Returns
    /// the labels of the steps that passed before and fail now. Each is
    /// logged and shown on the dashboard, and the caller's retry feedback
    /// names them ([`regression_note`]).
    ///
    /// `step_outcomes` are `(phase, passed)` of the steps that ran, the first
    /// of `steps`: verify stops at the first failure.
    pub(super) fn settle_step_regressions(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        steps: &[(String, crate::task_parser::VerifyStep)],
        step_outcomes: &[(String, bool)],
    ) -> Vec<String> {
        let ran: Vec<(String, String, bool)> = steps
            .iter()
            .zip(step_outcomes)
            .map(|((label, step), (_, passed))| {
                (label.clone(), step_identity(label, &step.command), *passed)
            })
            .collect();
        let book = &self.gate_retry_context;
        let regressed = step_regressions(&ran, &book.passed_steps(&spec.plan_id, &task.id));
        let passed_now = ran
            .into_iter()
            .filter(|(_, _, passed)| *passed)
            .map(|(_, identity, _)| identity);
        book.record_passed_steps(&spec.plan_id, &task.id, passed_now);
        for label in &regressed {
            tracing::warn!(
                plan_id = %spec.plan_id,
                task_id = %task.id,
                step = %label,
                "verify step passed on an earlier attempt and fails now: the last change broke it"
            );
            if let Some(tui) = &self.tui_bridge {
                tui.gate_regression(&spec.plan_id, &task.id, label);
            }
        }
        regressed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `(label, identity, passed)` for steps that ran with these outcomes,
    /// each step's command being its label.
    fn ran(outcomes: &[(&str, bool)]) -> Vec<(String, String, bool)> {
        outcomes
            .iter()
            .map(|(label, passed)| ((*label).to_string(), step_identity(label, label), *passed))
            .collect()
    }

    /// The identities of the steps of `ran` that passed.
    fn passes(ran: &[(String, String, bool)]) -> BTreeSet<String> {
        ran.iter()
            .filter(|(_, _, passed)| *passed)
            .map(|(_, identity, _)| identity.clone())
            .collect()
    }

    #[test]
    fn a_step_that_passed_on_an_earlier_attempt_and_fails_now_is_a_regression() {
        // Attempt 1 fixes compile and fails the tests; attempt 2 fixes the
        // tests and breaks compile again.
        let first = ran(&[("verify[0:compile]", true), ("verify[1:test]", false)]);
        assert!(step_regressions(&first, &BTreeSet::new()).is_empty());
        let second = ran(&[("verify[0:compile]", false)]);
        assert_eq!(step_regressions(&second, &passes(&first)), ["verify[0:compile]"]);

        // The same label with another command is another step.
        let edited = vec![(
            "verify[0:compile]".to_string(),
            step_identity("verify[0:compile]", "cargo check --all-targets"),
            false,
        )];
        assert!(step_regressions(&edited, &passes(&first)).is_empty());

        let note = regression_note(&["verify[0:compile]".to_string()], "a missing import");
        assert!(
            note.starts_with("Regression: verify[0:compile] passed on an earlier attempt"),
            "{note}"
        );
        assert!(note.ends_with("\na missing import"), "{note}");
        assert_eq!(regression_note(&[], "a missing import"), "a missing import");
    }

    #[test]
    fn step_order_within_an_attempt_is_not_a_regression() {
        // `test` passes, then `clippy` fails, in one attempt: clippy never
        // passed, whatever rung it sits on.
        let attempt = ran(&[("verify[0:test]", true), ("verify[1:clippy]", false)]);
        assert!(step_regressions(&attempt, &BTreeSet::new()).is_empty());
        // Nor on the next attempt, which fails clippy again.
        assert!(step_regressions(&attempt, &passes(&attempt)).is_empty());
    }

    /// The history outlives the process for the same checkpoint run, and the
    /// task's pass drops it.
    #[test]
    fn passed_steps_survive_a_resume_and_a_pass_clears_them() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("retry-feedback.json");
        let book = retry_feedback::RetryFeedbackBook::default();
        book.attach("plan-a", path.clone(), "run-1");
        book.record_passed_steps("plan-a", "T1", ["verify[0:compile]#abc".to_string()]);

        let resumed = retry_feedback::RetryFeedbackBook::default();
        resumed.attach("plan-a", path.clone(), "run-1");
        assert_eq!(
            resumed.passed_steps("plan-a", "T1"),
            BTreeSet::from(["verify[0:compile]#abc".to_string()])
        );
        let fresh = retry_feedback::RetryFeedbackBook::default();
        fresh.attach("plan-a", path.clone(), "run-2");
        assert!(fresh.passed_steps("plan-a", "T1").is_empty());

        resumed.clear("plan-a", "T1");
        assert!(resumed.passed_steps("plan-a", "T1").is_empty());
        assert!(!path.exists(), "nothing is left to keep");
    }
}
