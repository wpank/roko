//! The tool calls an attempt's live output showed (bug-264c41).
//!
//! A CLI provider such as the Claude CLI runs its tools itself: roko's tool
//! audit never sees them (it covers roko's own tool loop, gap-4d5e2d), and
//! the result bridge projects no tool events. The attempt's live-output tap
//! ([`GraphTaskDispatcher::live_output_tap`]) does see them: a tool step for
//! each call and, once the tap is trusted, the provider's tool result, which
//! says whether the call failed. The tap records them here, and the
//! attempt's efficiency row reads them once the tap has drained. Each call
//! keeps the target its tool step showed, so a verified attempt's calls can
//! become a playbook's steps (gap-89f985).
//!
//! The tap is best-effort: the provider boundary drops live events while the
//! tap's channel is full, so a burst can cost a call or its outcome.

use std::collections::BTreeMap;
use std::time::Duration;

use roko_agent::live_output::LiveAgentEvent;
use roko_agent::tool_loop::StreamEventKind;
use roko_core::dashboard_snapshot::{ATTEMPT_PHASE_TOOLS, ToolTime};

use super::*;
use crate::dispatch_v2::ToolCallRecord;

/// How long [`LiveToolCalls::finish`] waits for the tap to drain before it
/// reads what the tap recorded so far. The tap ends as soon as the dispatch
/// drops its live output, so this only bounds a sender something kept.
const TAP_DRAIN_WAIT: Duration = Duration::from_secs(2);

/// The tool calls one attempt's live output showed, in the order the
/// provider made them. Clones share one record.
#[derive(Clone, Default)]
pub(super) struct LiveToolCalls {
    state: Arc<parking_lot::Mutex<LiveToolCallsState>>,
}

#[derive(Default)]
struct LiveToolCallsState {
    calls: Vec<ToolCallRecord>,
    /// What each of `calls` acted on, as its tool step showed it (a path,
    /// the first line of a command); empty for a call whose step never
    /// arrived.
    targets: Vec<String>,
    /// When each of `calls` was announced and when its result came, in unix
    /// ms (gap-6a319c); `None` for an event that never arrived.
    spans: Vec<(Option<i64>, Option<i64>)>,
    /// When each model turn ended, in unix ms: the provider's `Done`.
    turn_ends: Vec<i64>,
    tap: Option<tokio::task::JoinHandle<()>>,
}

/// The time an attempt's provider call spent in tool calls, as its live
/// output showed it (gap-6a319c).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct ToolTiming {
    /// Time inside tool calls, overlapping calls counted once; `None` when a
    /// call's result never showed (an untrusted tap sees none).
    pub tools_ms: Option<u64>,
    /// The tool calls made.
    pub tool_calls: u32,
    /// The tools that took longest, the longest first.
    pub by_name: Vec<ToolTime>,
    /// The model turns that ended; `None` when none showed.
    pub model_calls: Option<u32>,
}

impl LiveToolCalls {
    /// Record `event`. A tool step opens a call; a tool result settles the
    /// earliest open call under its id, or is a call of its own when its
    /// step never arrived.
    pub(super) fn observe(&self, event: &LiveAgentEvent) {
        self.observe_at(event, chrono::Utc::now().timestamp_millis());
    }

    /// [`Self::observe`] the event as seen at `at` (unix ms).
    fn observe_at(&self, event: &LiveAgentEvent, at: i64) {
        let mut state = self.state.lock();
        match event {
            LiveAgentEvent::ToolStep { id, name, target } => {
                state.calls.push(ToolCallRecord {
                    id: id.clone(),
                    name: name.clone(),
                    succeeded: None,
                });
                state.targets.push(target.clone());
                state.spans.push((Some(at), None));
            }
            LiveAgentEvent::Unscreened(StreamEventKind::ToolResult { id, is_error, .. }) => {
                let succeeded = Some(!is_error);
                let open = state
                    .calls
                    .iter()
                    .position(|call| call.id == *id && call.succeeded.is_none());
                match open {
                    Some(index) => {
                        state.calls[index].succeeded = succeeded;
                        state.spans[index].1 = Some(at);
                    }
                    None => {
                        state.calls.push(ToolCallRecord {
                            id: id.clone(),
                            name: String::new(),
                            succeeded,
                        });
                        state.targets.push(String::new());
                        state.spans.push((None, Some(at)));
                    }
                }
            }
            LiveAgentEvent::Unscreened(StreamEventKind::Done { .. }) => {
                state.turn_ends.push(at);
            }
            _ => {}
        }
    }

    /// The tool time recorded between `start` and `end` (unix ms), the
    /// provider call's window. A call runs from its announcement, or from
    /// the end of the model turn that asked for it when that came later
    /// (roko's own tool loop announces a call before the turn ends), until
    /// its result.
    pub(super) fn timing(&self, start: i64, end: i64) -> ToolTiming {
        let state = self.state.lock();
        let mut runs = Vec::new();
        let mut by_name: BTreeMap<String, (u32, u64)> = BTreeMap::new();
        let mut unknown = false;
        for (call, &(announced, result)) in state.calls.iter().zip(&state.spans) {
            let Some(result) = result else {
                unknown = true;
                continue;
            };
            let turn_end = state.turn_ends.iter().copied().filter(|at| *at <= result).max();
            let Some(from) = announced.into_iter().chain(turn_end).max() else {
                unknown = true;
                continue;
            };
            let (from, to) = (from.clamp(start, end), result.clamp(start, end));
            let ms = u64::try_from(to - from).unwrap_or(0);
            runs.push((from, to));
            let name = if call.name.is_empty() {
                "unknown"
            } else {
                call.name.as_str()
            };
            let entry = by_name.entry(name.to_string()).or_default();
            entry.0 += 1;
            entry.1 += ms;
        }
        runs.sort_unstable();
        let mut covered = 0_u64;
        let mut reach = i64::MIN;
        for (from, to) in runs {
            let from = from.max(reach);
            if to > from {
                covered += u64::try_from(to - from).unwrap_or(0);
            }
            reach = reach.max(to);
        }
        let mut by_name: Vec<ToolTime> = by_name
            .into_iter()
            .map(|(tool, (calls, ms))| ToolTime { tool, calls, ms })
            .collect();
        by_name.sort_by(|a, b| b.ms.cmp(&a.ms).then_with(|| a.tool.cmp(&b.tool)));
        by_name.truncate(ATTEMPT_PHASE_TOOLS);
        ToolTiming {
            tools_ms: (!unknown).then_some(covered),
            tool_calls: u32::try_from(state.calls.len()).unwrap_or(u32::MAX),
            by_name,
            model_calls: u32::try_from(state.turn_ends.len())
                .ok()
                .filter(|turns| *turns > 0),
        }
    }

    /// The tap that reads the live output is `tap`: [`Self::finish`] waits
    /// for it.
    pub(super) fn attach(&self, tap: tokio::task::JoinHandle<()>) {
        self.state.lock().tap = Some(tap);
    }

    /// The calls recorded, once the tap has read everything the dispatch
    /// sent, or after [`TAP_DRAIN_WAIT`].
    pub(super) async fn finish(&self) -> Vec<ToolCallRecord> {
        self.drain().await;
        self.state.lock().calls.clone()
    }

    /// [`Self::finish`], each call with the target its tool step showed.
    pub(super) async fn finish_with_targets(&self) -> Vec<(ToolCallRecord, String)> {
        self.drain().await;
        let state = self.state.lock();
        state
            .calls
            .iter()
            .cloned()
            .zip(state.targets.iter().cloned())
            .collect()
    }

    /// Wait for the tap to read everything the dispatch sent, at most
    /// [`TAP_DRAIN_WAIT`]. Only the first wait waits.
    async fn drain(&self) {
        let tap = self.state.lock().tap.take();
        if let Some(tap) = tap
            && tokio::time::timeout(TAP_DRAIN_WAIT, tap).await.is_err()
        {
            tracing::debug!("live output still open; reading the tool calls recorded so far");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(id: &str, name: &str) -> LiveAgentEvent {
        LiveAgentEvent::ToolStep {
            id: id.to_string(),
            name: name.to_string(),
            target: String::new(),
        }
    }

    fn result(id: &str, is_error: bool) -> LiveAgentEvent {
        LiveAgentEvent::Unscreened(StreamEventKind::ToolResult {
            id: id.to_string(),
            output: "out".to_string(),
            is_error,
        })
    }

    /// A step opens a call and its result settles it; a result whose step
    /// never arrived is a call of its own, and a call with no result keeps
    /// an unknown outcome. Other live events are not calls.
    #[test]
    fn live_tool_calls_pair_steps_with_results() {
        let calls = LiveToolCalls::default();
        for event in [
            step("tu_1", "Read"),
            LiveAgentEvent::Unscreened(StreamEventKind::TextDelta("thinking".to_string())),
            step("tu_2", "Bash"),
            result("tu_2", true),
            result("tu_1", false),
            result("tu_9", false),
            step("tu_3", "Grep"),
        ] {
            calls.observe(&event);
        }
        let recorded = calls.state.lock().calls.clone();
        let outcomes: Vec<(&str, &str, Option<bool>)> = recorded
            .iter()
            .map(|call| (call.id.as_str(), call.name.as_str(), call.succeeded))
            .collect();
        assert_eq!(
            outcomes,
            [
                ("tu_1", "Read", Some(true)),
                ("tu_2", "Bash", Some(false)),
                ("tu_9", "", Some(true)),
                ("tu_3", "Grep", None),
            ]
        );
    }

    /// `finish` waits for the tap to drain, so a result still in flight when
    /// the dispatch returned is counted.
    #[tokio::test]
    async fn live_tool_calls_finish_waits_for_the_tap() {
        let calls = LiveToolCalls::default();
        let (sink, mut events) = tokio::sync::mpsc::channel::<LiveAgentEvent>(4);
        let recorder = calls.clone();
        calls.attach(tokio::spawn(async move {
            while let Some(event) = events.recv().await {
                recorder.observe(&event);
            }
        }));
        sink.send(step("tu_1", "Read")).await.expect("send step");
        sink.send(result("tu_1", true)).await.expect("send result");
        drop(sink);

        let recorded = calls.finish().await;
        assert_eq!(
            recorded,
            [ToolCallRecord {
                id: "tu_1".to_string(),
                name: "Read".to_string(),
                succeeded: Some(false),
            }]
        );
    }

    /// gap-6a319c: a call runs from the end of the turn that asked for it
    /// (roko's tool loop announces it mid-turn) or its announcement, until
    /// its result; overlapping calls count once, and a call with no result
    /// leaves the tool time unknown.
    #[test]
    fn attempt_phases_tool_timing_counts_overlaps_once() {
        let calls = LiveToolCalls::default();
        let done = || LiveAgentEvent::Unscreened(StreamEventKind::Done {
            finish_reason: "tool_calls".to_string(),
        });
        for (at, event) in [
            (1_000, step("tu_1", "read_file")),
            (1_200, step("tu_2", "bash")),
            (1_500, done()),
            (1_700, result("tu_1", false)),
            (4_500, result("tu_2", false)),
            (6_000, done()),
            (6_100, step("tu_3", "bash")),
            (6_100, done()),
            (7_100, result("tu_3", true)),
            (9_000, done()),
        ] {
            calls.observe_at(&event, at);
        }

        let timing = calls.timing(900, 9_100);

        // tu_1 and tu_2 run 1500..1700 and 1500..4500 (3000 ms together),
        // tu_3 6100..7100.
        assert_eq!(timing.tools_ms, Some(4_000));
        assert_eq!(timing.tool_calls, 3);
        assert_eq!(timing.model_calls, Some(4));
        assert_eq!(
            timing.by_name,
            [
                ToolTime {
                    tool: "bash".to_string(),
                    calls: 2,
                    ms: 4_000,
                },
                ToolTime {
                    tool: "read_file".to_string(),
                    calls: 1,
                    ms: 200,
                },
            ]
        );

        calls.observe_at(&step("tu_4", "bash"), 9_050);
        assert_eq!(calls.timing(900, 9_100).tools_ms, None);
    }

    /// gap-89f985: each call keeps the target its tool step showed; a call
    /// whose step never arrived has none.
    #[tokio::test]
    async fn live_tool_calls_keep_each_steps_target() {
        let calls = LiveToolCalls::default();
        for event in [
            LiveAgentEvent::ToolStep {
                id: "tu_1".to_string(),
                name: "Read".to_string(),
                target: "src/lib.rs".to_string(),
            },
            result("tu_1", false),
            result("tu_9", true),
        ] {
            calls.observe(&event);
        }

        let recorded = calls.finish_with_targets().await;
        let targets: Vec<(&str, &str, Option<bool>)> = recorded
            .iter()
            .map(|(call, target)| (call.name.as_str(), target.as_str(), call.succeeded))
            .collect();
        assert_eq!(
            targets,
            [("Read", "src/lib.rs", Some(true)), ("", "", Some(false))]
        );
    }
}
