//! A try's implementer that hands back while its task's checks still fail,
//! with turns left, is sent back to them within the try (bug-cbbaa8).
//!
//! In Undertow's phase 3 every strong try on T1 ran the spec tests, saw them
//! fail and handed back at 13 to 25 of its 120 turns. Each hand-off spent a
//! whole try, and each next try started from the tree it left: 131 of 132
//! assertions, then 77 of 149, then 0 of 2. Now the agent's tool loop asks
//! [`SendBack`] when the agent hands back: it runs the task's checks as the
//! agent's `run_tests` would, and while they fail it tells the agent what they
//! say, as the next try's feedback would, up to `[gates] hand_off_continues`
//! times. Then the try settles on its checks, as before.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use roko_agent::tool_loop::{HandOff, HandOffCheck};
use roko_core::tool::ToolContext;
use roko_std::tool::builtin::task_checks::{ChecksOutcome, TaskCheck, run_checks};

use super::*;
use crate::dispatch::prompt_builder::{GateFeedback, render_gate_feedback_sections};

/// What a try's agent loop asks when the agent hands back: the task's checks,
/// and how many more times the agent may be sent back to them.
#[derive(Debug)]
pub(super) struct SendBack {
    /// The task's verify steps.
    steps: Vec<crate::task_parser::VerifyStep>,
    /// The same steps as `run_tests` runs them.
    checks: Vec<TaskCheck>,
    /// How many times the agent may be sent back.
    bound: u32,
    /// How many times it was.
    sent: AtomicU32,
    /// Whether the loop asked: the agent handed back with turns left. An
    /// agent whose loop roko does not run never does.
    asked: AtomicBool,
}

impl SendBack {
    /// How many times the agent was sent back to its checks; `None` when its
    /// loop never asked, as for a CLI agent, which runs its own loop.
    pub(super) fn continues(&self) -> Option<u32> {
        self.asked
            .load(Ordering::Relaxed)
            .then(|| self.sent.load(Ordering::Relaxed))
    }

    /// What the agent is told the `sent`-th time it goes back, its check at
    /// `index` having ended as `verdict`.
    ///
    /// It is the step's entry in a retry's feedback, in its sections, under a
    /// heading that says the try goes on.
    fn note(&self, index: usize, verdict: &roko_core::Verdict, sent: u32) -> String {
        let step = &self.steps[index];
        let label = &self.checks[index].label;
        let entry = verification::step_failure(label, &step.command, step, verdict);
        let sections = GateFeedback::from_raw(&entry)
            .map_or(entry, |feedback| render_gate_feedback_sections(&feedback));
        let after = match self.bound.saturating_sub(sent) {
            0 => "This try does not go back again: when you hand back, its checks judge the work \
                  as it stands."
                .to_string(),
            1 => "It can go back once more; after that, its checks judge the work as it stands."
                .to_string(),
            left => format!(
                "It can go back {left} more times; after that, its checks judge the work as it \
                 stands."
            ),
        };
        format!(
            "# Your checks still fail\n\n\
             You handed back, but your task's check {label} still fails, so this try goes on. \
             Fix what it reports, run your checks again (`run_tests` with `\"build\": \
             \"checks\"`), and hand back once they pass. {after}\n\n{sections}"
        )
    }
}

#[async_trait]
impl HandOffCheck for SendBack {
    fn may_send_back(&self) -> bool {
        self.asked.store(true, Ordering::Relaxed);
        self.sent.load(Ordering::Relaxed) < self.bound
    }

    async fn check(&self, ctx: &ToolContext) -> HandOff {
        let budget = self
            .checks
            .iter()
            .map(|check| {
                check
                    .timeout_ms
                    .map_or(Duration::ZERO, Duration::from_millis)
            })
            .sum();
        let (index, verdict) = match run_checks(&self.checks, ctx, budget, None).await {
            ChecksOutcome::Passed(ran) => {
                return HandOff::Ends(format!("the task's {} checks pass", ran.len()));
            }
            ChecksOutcome::NotRun(why) => return HandOff::Ends(why),
            // As the gate tells a failed step's verdict (`ShellGate`).
            ChecksOutcome::Failed {
                index,
                code,
                stdout,
                stderr,
                ..
            } => {
                let code = code.map_or_else(|| "terminated by signal".into(), |c| c.to_string());
                let output = if stderr.is_empty() {
                    stdout
                } else {
                    format!("{stdout}\n---stderr---\n{stderr}")
                };
                let verdict = roko_core::Verdict::fail(
                    &self.checks[index].label,
                    format!("exit code: {code}"),
                )
                .with_detail(output);
                (index, verdict)
            }
            ChecksOutcome::TimedOut { index, limit } => {
                let verdict = roko_core::Verdict::fail(
                    &self.checks[index].label,
                    format!("did not finish within {} s", limit.as_secs()),
                );
                (index, verdict)
            }
        };
        let sent = self.sent.fetch_add(1, Ordering::Relaxed) + 1;
        HandOff::GoesOn(self.note(index, &verdict, sent))
    }
}

impl GraphTaskDispatcher {
    /// What the tool loop of an attempt at `task` asks when its agent hands
    /// back (bug-cbbaa8).
    ///
    /// An implementer with verify steps gets one, outside FAST mode, while
    /// `[gates] hand_off_continues` is above 0; any other task gets `None`.
    pub(super) fn send_back(&self, task: &TaskDef) -> Option<Arc<SendBack>> {
        let bound = self.config.gates.hand_off_continues;
        let implementer = task.role.as_deref().unwrap_or("implementer") == "implementer";
        (bound > 0 && implementer && self.fast.is_none() && !task.verify.is_empty()).then(|| {
            Arc::new(SendBack {
                steps: task.verify.clone(),
                checks: task_checks::checks_of(task),
                bound,
                sent: AtomicU32::new(0),
                asked: AtomicBool::new(false),
            })
        })
    }
}
