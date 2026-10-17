//! Live output of a task's verify steps (gap-f45a6d).
//!
//! Each line a verify step's command writes reaches the hub as
//! `gate_output_line` while the step runs, so a host can scroll a failing
//! test by as it happens. A chatty check is bounded twice: by rate (a burst,
//! then a steady number of lines per second) and by a cap per step, and each
//! line is cut to a length. What the bounds skip is counted, and the step's
//! last line, from roko itself (`stream: "roko"`), says how many. The step's
//! `gate_result` still carries the end of its output.

use std::time::Instant;

use roko_gate::OutputLine;
use tokio::sync::{mpsc, oneshot};

use crate::runner::tui_bridge::TuiBridge;

/// The most lines one step publishes.
const STEP_LINES: usize = 1_000;

/// Lines a step may publish at once before the rate applies.
const BURST_LINES: f64 = 80.0;

/// Lines a step may publish per second once its burst is spent.
const LINES_PER_SECOND: f64 = 40.0;

/// The most characters of a line it publishes.
const LINE_CHARS: usize = 500;

/// The stream of roko's own line about what it skipped.
const ROKO_STREAM: &str = "roko";

/// Forwards one verify step's output lines to the hub while it runs.
pub(super) struct LiveCheckOutput {
    sink: mpsc::UnboundedSender<OutputLine>,
    done: oneshot::Sender<()>,
    forwarder: tokio::task::JoinHandle<()>,
}

impl LiveCheckOutput {
    /// Start forwarding the lines of step `gate` of attempt `attempt` of
    /// `task_id` through `tui`.
    pub(super) fn start(
        tui: TuiBridge,
        plan_id: &str,
        task_id: &str,
        attempt: u32,
        gate: &str,
    ) -> Self {
        let (sink, mut lines) = mpsc::unbounded_channel::<OutputLine>();
        let (done, mut finished) = oneshot::channel::<()>();
        let step = Step {
            tui,
            plan_id: plan_id.to_string(),
            task_id: task_id.to_string(),
            attempt,
            gate: gate.to_string(),
        };
        let forwarder = tokio::spawn(async move {
            let mut budget = Budget::new(Instant::now());
            loop {
                tokio::select! {
                    line = lines.recv() => match line {
                        Some(line) => step.offer(&mut budget, &line),
                        None => break,
                    },
                    // The step ended (or its forwarder was dropped): what
                    // it wrote is already queued.
                    _ = &mut finished => {
                        while let Ok(line) = lines.try_recv() {
                            step.offer(&mut budget, &line);
                        }
                        break;
                    }
                }
            }
            step.report_skipped(budget.skipped);
        });
        Self {
            sink,
            done,
            forwarder,
        }
    }

    /// The sink the step's gate writes its lines to.
    pub(super) fn sink(&self) -> mpsc::UnboundedSender<OutputLine> {
        self.sink.clone()
    }

    /// Publish what the step wrote, and how many lines were not streamed,
    /// before its result is published.
    pub(super) async fn finish(self) {
        let Self {
            sink,
            done,
            forwarder,
        } = self;
        drop(sink);
        let _ = done.send(());
        if let Err(error) = forwarder.await {
            tracing::debug!(%error, "the check's output forwarder ended early");
        }
    }
}

impl super::GraphTaskDispatcher {
    /// The live output of verify step `gate` of attempt `attempt` of `task`,
    /// when a hub follows the run.
    pub(super) fn live_check_output(
        &self,
        spec: &super::TaskExecutionSpec,
        task: &super::TaskDef,
        attempt: u32,
        gate: &str,
    ) -> Option<LiveCheckOutput> {
        self.tui_bridge
            .as_ref()
            .map(|tui| LiveCheckOutput::start(tui.clone(), &spec.plan_id, &task.id, attempt, gate))
    }
}

/// Where a step's lines go.
struct Step {
    tui: TuiBridge,
    plan_id: String,
    task_id: String,
    attempt: u32,
    gate: String,
}

impl Step {
    /// Publish `line` when `budget` allows it now.
    fn offer(&self, budget: &mut Budget, line: &OutputLine) {
        if budget.take(Instant::now()) {
            let text: String = line.line.chars().take(LINE_CHARS).collect();
            self.publish(&text, line.stream.as_str());
        }
    }

    /// Say how many lines the bounds skipped, when they skipped any.
    fn report_skipped(&self, skipped: usize) {
        if skipped > 0 {
            let note = format!(
                "… {skipped} more line(s) of this check were not streamed; its result keeps the \
                 end of its output"
            );
            self.publish(&note, ROKO_STREAM);
        }
    }

    fn publish(&self, line: &str, stream: &str) {
        self.tui.gate_output_line(
            &self.plan_id,
            &self.task_id,
            self.attempt,
            &self.gate,
            line,
            stream,
        );
    }
}

/// A step's allowance of lines: a token bucket of [`BURST_LINES`] that
/// refills at [`LINES_PER_SECOND`], within [`STEP_LINES`] in all.
#[derive(Debug)]
struct Budget {
    tokens: f64,
    at: Instant,
    published: usize,
    skipped: usize,
}

impl Budget {
    fn new(now: Instant) -> Self {
        Self {
            tokens: BURST_LINES,
            at: now,
            published: 0,
            skipped: 0,
        }
    }

    /// Whether one more line may be published at `now`; counts it either
    /// way.
    fn take(&mut self, now: Instant) -> bool {
        let refill = now.saturating_duration_since(self.at).as_secs_f64() * LINES_PER_SECOND;
        self.tokens = (self.tokens + refill).min(BURST_LINES);
        self.at = now;
        if self.published < STEP_LINES && self.tokens >= 1.0 {
            self.tokens -= 1.0;
            self.published += 1;
            true
        } else {
            self.skipped += 1;
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use roko_gate::OutputStream;

    use super::*;

    /// The budget lets a burst through at once, then the steady rate, and
    /// never more than the cap in all.
    #[test]
    fn the_budget_allows_a_burst_then_a_rate_within_a_cap() {
        let start = Instant::now();
        let mut budget = Budget::new(start);
        let burst = (0..200).filter(|_| budget.take(start)).count();
        assert_eq!(burst, BURST_LINES as usize);
        let second = start + Duration::from_secs(1);
        let rate = (0..200).filter(|_| budget.take(second)).count();
        assert_eq!(rate, LINES_PER_SECOND as usize);
        assert_eq!(budget.skipped, 400 - burst - rate);

        let mut budget = Budget::new(start);
        let total = (0..STEP_LINES * 2)
            .filter(|index| budget.take(start + Duration::from_secs(*index as u64)))
            .count();
        assert_eq!(total, STEP_LINES);
    }

    /// gap-f45a6d: a chatty check publishes a bounded number of lines, each
    /// cut to its length, and one line from roko that counts the rest.
    #[tokio::test]
    async fn a_chatty_check_is_bounded() {
        let hub = crate::state_hub::StateHub::new(4_096);
        let mut events = hub.subscribe_events();
        let live = LiveCheckOutput::start(
            TuiBridge::new(hub.sender()),
            "plan",
            "T1",
            2,
            "verify[0:test]",
        );
        let sink = live.sink();
        let long = "x".repeat(LINE_CHARS * 2);
        for _ in 0..5_000 {
            sink.send(OutputLine::new(OutputStream::Stdout, &long))
                .expect("the forwarder listens");
        }
        drop(sink);
        live.finish().await;

        let mut lines = Vec::new();
        while let Ok(envelope) = events.try_recv() {
            if let roko_core::DashboardEvent::GateOutputLine {
                attempt,
                gate,
                line,
                stream,
                ..
            } = envelope.payload
            {
                assert_eq!((attempt, gate.as_str()), (2, "verify[0:test]"));
                lines.push((line, stream));
            }
        }
        let (note, stream) = lines.pop().expect("roko's note");
        assert_eq!(stream, "roko");
        let skipped: usize = note
            .split_whitespace()
            .nth(1)
            .and_then(|count| count.parse().ok())
            .unwrap_or_else(|| panic!("a count in {note:?}"));
        assert!(lines.len() < 200, "{} lines streamed", lines.len());
        assert_eq!(lines.len() + skipped, 5_000);
        assert!(
            lines
                .iter()
                .all(|(line, stream)| line.chars().count() == LINE_CHARS && stream == "stdout")
        );
    }
}
