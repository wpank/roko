//! Bridge between runner state changes and the TUI dashboard.
//!
//! Wraps `StateHubSender` with convenience methods that publish
//! `DashboardEvent` variants for each significant runner event.

use crate::state_hub::StateHubSender;
use crate::tui::Tab;
use roko_core::dashboard_snapshot::{DashboardEvent, DiagnosisSummary};

/// Prefix for semantic live transcript records carried through the existing
/// StateHub `AgentOutput` event. Keeping this wire-compatible avoids a schema
/// migration while ensuring the TUI never has to scrape provider output text.
pub const STREAM_RECORD_PREFIX: &str = "\u{001e}roko.stream.v1 ";

use super::screenshot_collector::ScreenshotCollector;
use super::types::RunnerEvent;

/// Publishes runner events to the TUI / dashboard via `StateHub`.
#[derive(Clone)]
pub struct TuiBridge {
    sender: StateHubSender,
    screenshot_collector: Option<ScreenshotCollector>,
}

impl TuiBridge {
    /// Create a new bridge from a `StateHubSender`.
    pub fn new(sender: StateHubSender) -> Self {
        Self {
            sender,
            screenshot_collector: None,
        }
    }

    /// Attach a non-blocking continuous screenshot collector.
    #[must_use]
    pub fn with_screenshot_collector(mut self, collector: ScreenshotCollector) -> Self {
        self.screenshot_collector = Some(collector);
        self
    }

    /// A plan has started execution.
    pub fn plan_started(&self, plan_id: &str, tasks_total: usize) {
        self.sender.publish(DashboardEvent::PlanStarted {
            plan_id: plan_id.to_string(),
            tasks_total,
        });
        self.capture(
            "plan_started",
            Some(plan_id.to_string()),
            &[Tab::Dashboard, Tab::Plans],
        );
    }

    /// A plan has completed (successfully or not).
    pub fn plan_completed(&self, plan_id: &str, success: bool) {
        self.sender.publish(DashboardEvent::PlanCompleted {
            plan_id: plan_id.to_string(),
            success,
        });
        self.capture(
            "plan_completed",
            Some(format!(
                "{plan_id}:{}",
                if success { "passed" } else { "failed" }
            )),
            &[Tab::Dashboard, Tab::Plans],
        );
    }

    /// A task has started.
    pub fn task_started(&self, plan_id: &str, task_id: &str, title: &str, phase: &str) {
        self.sender.publish(DashboardEvent::TaskStarted {
            plan_id: plan_id.to_string(),
            task_id: task_id.to_string(),
            title: title.to_string(),
            phase: phase.to_string(),
        });
        self.capture(
            "task_started",
            Some(format!("{plan_id}/{task_id}")),
            &[Tab::Dashboard, Tab::Plans],
        );
    }

    /// A task changed phase.
    pub fn task_phase_changed(
        &self,
        plan_id: &str,
        task_id: &str,
        old_phase: &str,
        new_phase: &str,
    ) {
        self.sender.publish(DashboardEvent::TaskPhaseChanged {
            plan_id: plan_id.to_string(),
            task_id: task_id.to_string(),
            old_phase: old_phase.to_string(),
            new_phase: new_phase.to_string(),
        });
        self.capture(
            "task_phase_changed",
            Some(format!("{plan_id}/{task_id}:{old_phase}->{new_phase}")),
            &[Tab::Dashboard, Tab::Plans],
        );
    }

    /// A task has completed.
    pub fn task_completed(&self, plan_id: &str, task_id: &str, outcome: &str) {
        self.sender.publish(DashboardEvent::TaskCompleted {
            plan_id: plan_id.to_string(),
            task_id: task_id.to_string(),
            outcome: outcome.to_string(),
        });
        self.capture(
            "task_completed",
            Some(format!("{plan_id}/{task_id}:{outcome}")),
            &[Tab::Dashboard, Tab::Plans],
        );
    }

    /// An agent has been spawned.
    pub fn agent_spawned(
        &self,
        agent_id: &str,
        plan_id: &str,
        task_id: &str,
        attempt: u32,
        role: &str,
        model: &str,
        provider: &str,
    ) {
        let role = if role.is_empty() { "impl" } else { role };
        self.sender.publish(DashboardEvent::AgentSpawned {
            agent_id: agent_id.to_string(),
            plan_id: plan_id.to_string(),
            task_id: task_id.to_string(),
            attempt,
            role: role.to_string(),
            model: model.to_string(),
            provider: provider.to_string(),
        });
        self.capture(
            "agent_spawned",
            Some(format!("{plan_id}/{task_id}:{agent_id}")),
            &[Tab::Dashboard, Tab::Agents],
        );
    }

    /// Agent produced text output (streamed).
    pub fn agent_output(
        &self,
        agent_id: &str,
        plan_id: &str,
        task_id: &str,
        attempt: u32,
        content: &str,
    ) {
        self.sender.publish(DashboardEvent::AgentOutput {
            agent_id: agent_id.to_string(),
            plan_id: plan_id.to_string(),
            task_id: task_id.to_string(),
            attempt,
            content: content.to_string(),
        });
    }

    /// Publish an assistant text delta as a semantic StateHub stream record.
    pub fn agent_text_delta(
        &self,
        agent_id: &str,
        plan_id: &str,
        task_id: &str,
        attempt: u32,
        text: &str,
    ) {
        self.publish_stream_record(
            agent_id,
            plan_id,
            task_id,
            attempt,
            "text",
            serde_json::json!({"text": text}),
        );
    }

    /// Publish a reasoning delta without degrading it to ordinary assistant text.
    pub fn agent_reasoning_delta(
        &self,
        agent_id: &str,
        plan_id: &str,
        task_id: &str,
        attempt: u32,
        text: &str,
    ) {
        self.publish_stream_record(
            agent_id,
            plan_id,
            task_id,
            attempt,
            "reasoning",
            serde_json::json!({"text": text}),
        );
    }

    /// Publish a tool invocation. Arguments are optional because some provider
    /// protocols stream them separately or omit them for safety.
    pub fn tool_call(
        &self,
        agent_id: &str,
        plan_id: &str,
        task_id: &str,
        attempt: u32,
        tool_id: &str,
        tool_name: &str,
    ) {
        self.publish_stream_record(
            agent_id,
            plan_id,
            task_id,
            attempt,
            "tool_start",
            serde_json::json!({"tool_id": tool_id, "tool": tool_name}),
        );
    }

    /// Publish a tool result correlated by provider call ID.
    pub fn tool_output(
        &self,
        agent_id: &str,
        plan_id: &str,
        task_id: &str,
        attempt: u32,
        tool_id: &str,
        output: &str,
    ) {
        self.publish_stream_record(
            agent_id,
            plan_id,
            task_id,
            attempt,
            "tool_result",
            serde_json::json!({"tool_id": tool_id, "output": output}),
        );
    }

    fn publish_stream_record(
        &self,
        agent_id: &str,
        plan_id: &str,
        task_id: &str,
        attempt: u32,
        kind: &str,
        payload: serde_json::Value,
    ) {
        let record = serde_json::json!({"kind": kind, "agent_id": agent_id, "plan_id": plan_id, "task_id": task_id, "attempt": attempt, "payload": payload});
        self.sender.publish(DashboardEvent::AgentOutput {
            agent_id: agent_id.to_string(),
            plan_id: plan_id.to_string(),
            task_id: task_id.to_string(),
            attempt,
            content: format!("{}{}", STREAM_RECORD_PREFIX, record),
        });
    }

    /// Agent has finished.
    pub fn agent_completed(&self, agent_id: &str, plan_id: &str, task_id: &str, attempt: u32) {
        self.sender.publish(DashboardEvent::AgentCompleted {
            agent_id: agent_id.to_string(),
            plan_id: plan_id.to_string(),
            task_id: task_id.to_string(),
            attempt,
        });
        self.capture(
            "agent_completed",
            Some(format!("{plan_id}/{task_id}:{agent_id}")),
            &[Tab::Dashboard, Tab::Agents],
        );
    }

    /// A single line of gate output (streamed).
    pub fn gate_output_line(&self, plan_id: &str, task_id: &str, gate: &str, line: &str) {
        self.sender.publish(DashboardEvent::GateOutputLine {
            plan_id: plan_id.to_string(),
            task_id: task_id.to_string(),
            gate: gate.to_string(),
            line: line.to_string(),
        });
    }

    /// A gate verdict.
    pub fn gate_result(&self, plan_id: &str, task_id: &str, gate: &str, passed: bool) {
        self.gate_result_with_output(plan_id, task_id, gate, passed, None);
    }

    /// A gate verdict with its captured output.
    pub fn gate_result_with_output(
        &self,
        plan_id: &str,
        task_id: &str,
        gate: &str,
        passed: bool,
        output_text: Option<&str>,
    ) {
        self.sender.publish(DashboardEvent::GateResult {
            plan_id: plan_id.to_string(),
            task_id: task_id.to_string(),
            gate: gate.to_string(),
            passed,
            output_text: output_text.map(str::to_string),
        });
    }

    /// Phase transition within a plan.
    pub fn phase_transition(&self, plan_id: &str, from: &str, to: &str) {
        self.sender.publish(DashboardEvent::PhaseTransition {
            plan_id: plan_id.to_string(),
            from: from.to_string(),
            to: to.to_string(),
        });
        self.capture(
            "phase_transition",
            Some(format!("{plan_id}:{from}->{to}")),
            &Tab::ALL,
        );
    }

    /// Efficiency metric for a task.
    pub fn efficiency_event(&self, plan_id: &str, task_id: &str, metric: &str, value: f64) {
        self.sender.publish(DashboardEvent::EfficiencyEvent {
            plan_id: plan_id.to_string(),
            task_id: task_id.to_string(),
            metric: metric.to_string(),
            value,
        });
    }

    /// Forward token usage to the dashboard.
    ///
    /// Publishes all four token counters (input, output, cache-read,
    /// cache-write) as individual `EfficiencyEvent`s so the snapshot
    /// accumulates them even when the output sink is `NoopSink`.
    pub fn token_usage(
        &self,
        plan_id: &str,
        task_id: &str,
        input_tokens: u64,
        output_tokens: u64,
        cache_read_tokens: u64,
        cache_write_tokens: u64,
    ) {
        self.efficiency_event(plan_id, task_id, "input_tokens", input_tokens as f64);
        self.efficiency_event(plan_id, task_id, "output_tokens", output_tokens as f64);
        self.efficiency_event(
            plan_id,
            task_id,
            "cache_read_tokens",
            cache_read_tokens as f64,
        );
        self.efficiency_event(
            plan_id,
            task_id,
            "cache_write_tokens",
            cache_write_tokens as f64,
        );
    }

    /// Error event.
    pub fn error(&self, message: &str) {
        self.sender.publish(DashboardEvent::Error {
            message: message.to_string(),
        });
        self.capture(
            "error",
            Some(message.to_string()),
            &[Tab::Dashboard, Tab::Logs],
        );
    }

    /// Publish a lightweight runner status line before the full lifecycle
    /// event stream is available (for example during cache warmup).
    pub fn status(&self, event_type: &str, message: &str) {
        self.sender.publish(DashboardEvent::EventLogEntry {
            timestamp_ms: timestamp_now_ms(),
            event_type: event_type.to_string(),
            plan_id: String::new(),
            task_id: String::new(),
            message: message.to_string(),
        });
        if event_type.starts_with("startup.") {
            self.capture(
                event_type,
                Some(message.to_string()),
                &[Tab::Dashboard, Tab::Logs],
            );
        }
    }

    /// Publish a typed runner lifecycle event into the dashboard event log.
    pub fn runner_event(&self, event: &RunnerEvent) {
        if let RunnerEvent::RunCompleted {
            outcome,
            duration_ms,
            cleanup_degraded,
            surviving_agent_ids,
            surviving_agent_pids,
            ..
        } = event
        {
            let outcome = match outcome {
                super::types::RunOutcome::Succeeded => "succeeded",
                super::types::RunOutcome::Failed => "failed",
                super::types::RunOutcome::Cancelled => "cancelled",
            };
            self.sender.publish(DashboardEvent::RunCompleted {
                outcome: outcome.to_string(),
                duration_ms: *duration_ms,
                cleanup_degraded: *cleanup_degraded,
                surviving_agent_ids: surviving_agent_ids.clone(),
                surviving_agent_pids: surviving_agent_pids.clone(),
            });
        }
        self.sender.publish(DashboardEvent::EventLogEntry {
            timestamp_ms: event.timestamp_ms(),
            event_type: event.event_type().to_string(),
            plan_id: event.plan_id().unwrap_or_default().to_string(),
            task_id: event.task_id().unwrap_or_default().to_string(),
            message: event.message(),
        });
        self.capture_runner_event(event);
    }

    /// Cascade router state updated after observation.
    pub fn cascade_router_updated(&self, snapshot_json: &str) {
        self.sender.publish(DashboardEvent::CascadeRouterUpdated {
            snapshot_json: snapshot_json.to_string(),
        });
    }

    /// Adaptive gate thresholds updated.
    pub fn gate_thresholds_updated(&self, snapshot_json: &str) {
        self.sender.publish(DashboardEvent::GateThresholdsUpdated {
            snapshot_json: snapshot_json.to_string(),
        });
    }

    /// Experiment winners refreshed.
    pub fn experiment_winners_updated(&self, winners: Vec<roko_core::ExperimentWinnerSummary>) {
        self.sender
            .publish(DashboardEvent::ExperimentWinnersUpdated { winners });
    }

    /// C-factor trend buckets refreshed.
    pub fn cfactor_trend_updated(
        &self,
        buckets: Vec<roko_core::dashboard_snapshot::CFactorBucket>,
    ) {
        self.sender
            .publish(DashboardEvent::CFactorTrendUpdated { buckets });
    }

    /// Efficiency trend buckets refreshed.
    pub fn efficiency_trend_updated(
        &self,
        buckets: Vec<roko_core::dashboard_snapshot::EfficiencyBucket>,
    ) {
        self.sender
            .publish(DashboardEvent::EfficiencyTrendUpdated { buckets });
    }

    /// Publish a raw `DashboardEvent` that has no dedicated bridge helper.
    pub fn publish_event(&self, event: DashboardEvent) {
        self.sender.publish(event);
    }

    /// Model was selected for a task dispatch.
    pub fn model_selected(&self, plan_id: &str, task_id: &str, model: &str, source: &str) {
        self.sender.publish(DashboardEvent::EventLogEntry {
            timestamp_ms: timestamp_now_ms(),
            event_type: "model_selected".to_string(),
            plan_id: plan_id.to_string(),
            task_id: task_id.to_string(),
            message: format!("model={model} source={source}"),
        });
    }

    /// Publish a conductor diagnosis to the dashboard ring buffer.
    pub fn diagnosis(&self, summary: DiagnosisSummary) {
        self.sender.publish(DashboardEvent::Diagnosis { summary });
    }

    /// Daimon affect state was updated after a task turn.
    pub fn affect_updated(
        &self,
        pleasure: f64,
        arousal: f64,
        dominance: f64,
        behavioral_state: &str,
        confidence: f64,
        recent_markers: Vec<(String, f64)>,
        active_biases: Vec<String>,
    ) {
        self.sender.publish(DashboardEvent::AffectUpdated {
            pleasure,
            arousal,
            dominance,
            behavioral_state: behavioral_state.to_string(),
            confidence,
            recent_markers,
            active_biases,
        });
    }

    /// Extension hook fired.
    pub fn extension_hook(&self, plan_id: &str, task_id: &str, hook: &str, success: bool) {
        self.sender.publish(DashboardEvent::EventLogEntry {
            timestamp_ms: timestamp_now_ms(),
            event_type: "extension_hook".to_string(),
            plan_id: plan_id.to_string(),
            task_id: task_id.to_string(),
            message: format!("hook={hook} success={success}"),
        });
    }

    /// Periodic heartbeat from a running agent (item 108).
    ///
    /// Published every ~15 seconds while an agent API call is in progress so
    /// the TUI can show elapsed time even when no streaming tokens arrive.
    pub fn agent_heartbeat(&self, agent_id: &str, plan_id: &str, task_id: &str, elapsed_ms: u64) {
        self.sender.publish(DashboardEvent::AgentHeartbeat {
            agent_id: agent_id.to_string(),
            plan_id: plan_id.to_string(),
            task_id: task_id.to_string(),
            elapsed_ms,
        });
    }

    /// Critical-path ETA updated after a task state change.
    pub fn critical_path_eta(&self, plan_id: &str, eta_minutes: Option<u32>) {
        self.sender.publish(DashboardEvent::CriticalPathEtaUpdated {
            plan_id: plan_id.to_string(),
            eta_minutes,
        });
    }

    /// A gate rung started execution (item 108).
    ///
    /// Published before each rung in the gate pipeline so the TUI can show
    /// which rung is running and for how long.
    pub fn gate_rung_started(&self, plan_id: &str, task_id: &str, rung_name: &str) {
        self.sender.publish(DashboardEvent::GateRungStarted {
            plan_id: plan_id.to_string(),
            task_id: task_id.to_string(),
            rung_name: rung_name.to_string(),
        });
    }

    fn capture_runner_event(&self, event: &RunnerEvent) {
        let detail = event_detail(event);
        match event {
            RunnerEvent::RunStarted { .. } => {
                self.capture("run_started", detail, &Tab::ALL);
            }
            RunnerEvent::RunCompleted { .. } => {
                self.capture("completion", detail, &Tab::ALL);
            }
            RunnerEvent::GateCompleted { .. } => {
                self.capture(
                    "gate_completed",
                    detail,
                    &[Tab::Dashboard, Tab::Plans, Tab::Learning],
                );
            }
            RunnerEvent::MergeBackendCompleted { .. } => {
                self.capture(
                    "merge_completed",
                    detail,
                    &[Tab::Dashboard, Tab::Plans, Tab::Git],
                );
            }
            RunnerEvent::TaskAttemptCancellationFailed { .. }
            | RunnerEvent::TimeoutRecorded { .. }
            | RunnerEvent::BudgetExceeded { .. }
            | RunnerEvent::PlanCancelled { .. }
            | RunnerEvent::ConductorIntervention { .. } => {
                self.capture(event.event_type(), detail, &[Tab::Dashboard, Tab::Logs]);
            }
            RunnerEvent::BatchPause { .. } | RunnerEvent::BatchResume { .. } => {
                self.capture(event.event_type(), detail, &Tab::ALL);
            }
            RunnerEvent::ResumeMarker { .. }
            | RunnerEvent::PlanStarted { .. }
            | RunnerEvent::PlanCompleted { .. }
            | RunnerEvent::TaskAttemptStarted { .. }
            | RunnerEvent::TaskAttemptCompleted { .. }
            | RunnerEvent::TaskAttemptCancellationRequested { .. }
            | RunnerEvent::TimeoutSalvagedToGate { .. }
            | RunnerEvent::AgentDispatchStarted { .. }
            | RunnerEvent::AgentDispatchCompleted { .. }
            | RunnerEvent::AgentCompleted { .. }
            | RunnerEvent::GateDispatchStarted { .. }
            | RunnerEvent::PromptAssembled { .. }
            | RunnerEvent::RetryDecision { .. }
            | RunnerEvent::RunPaused { .. }
            | RunnerEvent::RunResumed { .. } => {}
        }
    }

    fn capture(&self, label: &str, detail: Option<String>, tabs: &[Tab]) {
        if let Some(collector) = &self.screenshot_collector {
            let _ = collector.capture_event(label, detail, tabs);
        }
    }
}

fn event_detail(event: &RunnerEvent) -> Option<String> {
    match (event.plan_id(), event.task_id()) {
        (Some(plan_id), Some(task_id)) => Some(format!("{plan_id}/{task_id}")),
        (Some(plan_id), None) => Some(plan_id.to_string()),
        (None, Some(task_id)) => Some(task_id.to_string()),
        (None, None) => Some(event.message()),
    }
}

fn timestamp_now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_hub::StateHub;

    #[test]
    fn semantic_stream_records_are_published_through_statehub() {
        let hub = StateHub::default_capacity();
        let bridge = TuiBridge::new(hub.sender());
        let mut subscription = hub.subscribe_events_from(0);
        bridge.agent_text_delta("a", "p", "t", 1, "hello");
        let event = subscription.live.try_recv().expect("live event");
        let DashboardEvent::AgentOutput { content, .. } = event.payload else {
            panic!("expected agent output event");
        };
        assert!(content.starts_with(STREAM_RECORD_PREFIX));
        let payload: serde_json::Value =
            serde_json::from_str(content.strip_prefix(STREAM_RECORD_PREFIX).expect("prefix"))
                .expect("record json");
        assert_eq!(payload["kind"], "text");
        assert_eq!(payload["payload"]["text"], "hello");
    }
}
