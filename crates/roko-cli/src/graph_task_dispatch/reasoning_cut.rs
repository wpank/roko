//! What the hub hears when roko's tool loop stopped a reply that had only
//! reasoned (gap-f3f950).
//!
//! The tool loop stops a streamed reply once it has reasoned for 3/8 of its
//! output limit with no text and no tool call, and asks again: such a reply
//! is thrown away whole, so more reasoning only delays the ask. Once a try's
//! dispatch ends, [`GraphTaskDispatcher::publish_reasoning_cuts`] publishes
//! one `reasoning_cut` per stopped reply, from the turns the tool loop traced
//! (`agent.trace`): the tokens it reasoned, how long it ran, the seconds the
//! stop saved by estimate, and what the re-ask did.

use super::*;

impl GraphTaskDispatcher {
    /// Tell the hub about each reply of `dispatch`, attempt `attempt` of
    /// `plan_id`/`task_id`, that the tool loop stopped at its reasoning limit.
    pub(super) fn publish_reasoning_cuts(
        &self,
        plan_id: &str,
        task_id: &str,
        attempt: u32,
        dispatch: &crate::dispatch_v2::AgentResultDispatch,
    ) {
        let Some(tui) = &self.tui_bridge else {
            return;
        };
        for event in reasoning_cuts(plan_id, task_id, attempt, dispatch) {
            tui.publish_event(event);
        }
    }
}

/// The `reasoning_cut` events of the turns `dispatch` traced.
fn reasoning_cuts(
    plan_id: &str,
    task_id: &str,
    attempt: u32,
    dispatch: &crate::dispatch_v2::AgentResultDispatch,
) -> Vec<roko_core::DashboardEvent> {
    let turns: Vec<&serde_json::Value> = dispatch
        .result
        .trace
        .iter()
        .filter(|signal| signal.kind.as_str() == "agent.trace")
        .filter_map(|signal| match &signal.body {
            roko_core::Body::Json(turn) => Some(turn),
            _ => None,
        })
        .collect();
    let max_output = dispatch
        .target
        .model_profile
        .as_ref()
        .and_then(|profile| profile.max_output)
        .unwrap_or_else(|| u64::from(roko_core::defaults::DEFAULT_MAX_OUTPUT_TOKENS));
    let limit_tokens = max_output * 3 / 8;
    let stopped = |turn: &serde_json::Value| {
        turn["finish_reason"] == roko_agent::streaming::REASONING_LIMIT_FINISH
    };
    let acted = |turn: &serde_json::Value| {
        turn["tool_calls"]
            .as_array()
            .is_some_and(|calls| !calls.is_empty())
    };
    turns
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, turn)| stopped(turn))
        .map(|(index, turn)| {
            let reasoning_tokens = turn["usage"]["output_tokens"].as_u64().unwrap_or(0);
            let secs = turn["duration_ms"].as_u64().map(|ms| ms as f64 / 1_000.0);
            // The rest of the output limit at the reply's own pace.
            let saved_secs = secs.filter(|_| reasoning_tokens > 0).map(|secs| {
                max_output.saturating_sub(reasoning_tokens) as f64 * secs / reasoning_tokens as f64
            });
            let reask = match turns.get(index + 1) {
                None => "ended",
                Some(next) if acted(next) => "acted",
                Some(next) if stopped(next) || next["finish_reason"] == "length" => "cut_again",
                Some(_) => "answered",
            };
            let model = turn["model"]
                .as_str()
                .map_or_else(|| dispatch.target.model_slug.clone(), str::to_string);
            let ran = secs.map_or_else(String::new, |secs| format!(" in {secs:.0} s"));
            let saved =
                saved_secs.map_or_else(String::new, |secs| format!(", about {secs:.0} s sooner"));
            let then = match reask {
                "acted" => "the re-ask made a tool call",
                "answered" => "the re-ask answered with text",
                "cut_again" => "the re-ask was cut again",
                _ => "the try ended before a re-ask",
            };
            roko_core::DashboardEvent::ReasoningCut {
                plan_id: plan_id.to_string(),
                task_id: task_id.to_string(),
                attempt,
                turn: turn["turn"]
                    .as_u64()
                    .and_then(|turn| u32::try_from(turn).ok())
                    .unwrap_or(0),
                model: model.clone(),
                reasoning_tokens,
                limit_tokens,
                secs,
                saved_secs,
                reask: reask.to_string(),
                detail: format!(
                    "{model} reasoned {reasoning_tokens} tokens{ran} with no tool call, past its \
                     limit of {limit_tokens}: roko stopped it and asked again{saved}; {then}"
                ),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use roko_core::agent::ProviderKind;
    use roko_core::config::schema::ModelProfile;
    use roko_core::{Body, Kind};

    use super::*;
    use crate::graph_task_dispatch::tests::{fixture_config, make_bare_dispatcher};
    use crate::state_hub::StateHub;

    /// A traced turn as the tool loop's `agent.trace` signal carries it.
    fn traced(turn: u32, finish: &str, output_tokens: u32, ms: u64, tool: Option<&str>) -> Signal {
        let tool_calls = tool.map_or_else(Vec::new, |name| {
            vec![serde_json::json!({ "name": name, "result_preview": "ok" })]
        });
        Signal::builder(Kind::Custom("agent.trace".to_string()))
            .body(Body::Json(serde_json::json!({
                "turn": turn,
                "model": "glm-4.7",
                "tool_calls": tool_calls,
                "finish_reason": finish,
                "duration_ms": ms,
                "usage": { "output_tokens": output_tokens },
            })))
            .build()
    }

    /// gap-f3f950: once a try's dispatch ends, the hub hears one
    /// `reasoning_cut` per reply the tool loop stopped at its reasoning limit:
    /// the tokens it reasoned, how long it ran, the seconds the stop saved by
    /// estimate, and what the re-ask did. A reply cut at the output limit is
    /// not one.
    #[tokio::test]
    async fn a_reasoning_cut_reaches_the_hub_with_what_the_reask_did() {
        let temp = tempfile::tempdir().expect("tempdir");
        let hub = StateHub::default_capacity();
        let dispatcher = make_bare_dispatcher(fixture_config(), temp.path())
            .await
            .with_tui_bridge(crate::runner::tui_bridge::TuiBridge::new(hub.sender()));
        let output = Signal::builder(Kind::AgentOutput)
            .body(Body::text("done"))
            .build();
        let dispatch = crate::dispatch_v2::AgentResultDispatch {
            target: crate::dispatch_v2::ProviderDispatchSpec {
                model_key: "glm-4-7".to_string(),
                model_slug: "glm-4.7".to_string(),
                provider_id: "zai".to_string(),
                provider_kind: ProviderKind::OpenAiCompat,
                model_profile: Some(ModelProfile {
                    slug: "glm-4.7".to_string(),
                    max_output: Some(32_768),
                    ..ModelProfile::default()
                }),
                provider_config: None,
                runtime: crate::dispatch_v2::ProviderRuntime::AgentResultBridge {
                    provider_kind: ProviderKind::OpenAiCompat,
                },
            },
            result: roko_agent::AgentResult::ok(output).with_trace(vec![
                traced(1, "tool_calls", 300, 9_000, Some("read_file")),
                // Stopped at 12,288 tokens after 330 s: the re-ask writes.
                traced(2, "reasoning_limit", 12_288, 330_000, None),
                traced(3, "tool_calls", 900, 30_000, Some("write_file")),
                // Cut at the output limit: not roko's stop.
                traced(4, "length", 32_768, 900_000, None),
                traced(5, "reasoning_limit", 12_288, 330_000, None),
            ]),
            events: Vec::new(),
            tool_calls: Vec::new(),
            tool_policy: None,
        };

        dispatcher.publish_reasoning_cuts("p1", "T1", 2, &dispatch);

        let cuts: Vec<roko_core::DashboardEvent> = hub
            .replay_from(0)
            .into_iter()
            .map(|envelope| envelope.payload)
            .filter(|event| matches!(event, roko_core::DashboardEvent::ReasoningCut { .. }))
            .collect();
        assert_eq!(cuts.len(), 2, "{cuts:?}");
        let roko_core::DashboardEvent::ReasoningCut {
            attempt,
            turn,
            model,
            reasoning_tokens,
            limit_tokens,
            secs,
            saved_secs,
            reask,
            detail,
            ..
        } = &cuts[0]
        else {
            unreachable!();
        };
        assert_eq!((*attempt, *turn), (2, 2));
        assert_eq!(model, "glm-4.7");
        assert_eq!((*reasoning_tokens, *limit_tokens), (12_288, 12_288));
        assert_eq!(*secs, Some(330.0));
        // The other 20,480 tokens of the limit at 12,288 tokens per 330 s.
        assert!(
            saved_secs.is_some_and(|saved| (saved - 550.0).abs() < 0.01),
            "{saved_secs:?}"
        );
        assert_eq!(reask, "acted");
        assert!(
            detail.contains("roko stopped it and asked again"),
            "{detail}"
        );
        let roko_core::DashboardEvent::ReasoningCut { turn, reask, .. } = &cuts[1] else {
            unreachable!();
        };
        assert_eq!((*turn, reask.as_str()), (5, "ended"));
    }
}
