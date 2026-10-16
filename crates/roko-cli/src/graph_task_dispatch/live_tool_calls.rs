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

use std::time::Duration;

use roko_agent::live_output::LiveAgentEvent;
use roko_agent::tool_loop::StreamEventKind;

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
    tap: Option<tokio::task::JoinHandle<()>>,
}

impl LiveToolCalls {
    /// Record `event`. A tool step opens a call; a tool result settles the
    /// earliest open call under its id, or is a call of its own when its
    /// step never arrived.
    pub(super) fn observe(&self, event: &LiveAgentEvent) {
        let mut state = self.state.lock();
        match event {
            LiveAgentEvent::ToolStep { id, name, target } => {
                state.calls.push(ToolCallRecord {
                    id: id.clone(),
                    name: name.clone(),
                    succeeded: None,
                });
                state.targets.push(target.clone());
            }
            LiveAgentEvent::Unscreened(StreamEventKind::ToolResult { id, is_error, .. }) => {
                let succeeded = Some(!is_error);
                let open = state
                    .calls
                    .iter_mut()
                    .find(|call| call.id == *id && call.succeeded.is_none());
                match open {
                    Some(call) => call.succeeded = succeeded,
                    None => {
                        state.calls.push(ToolCallRecord {
                            id: id.clone(),
                            name: String::new(),
                            succeeded,
                        });
                        state.targets.push(String::new());
                    }
                }
            }
            _ => {}
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
