//! Provider failover for ACP prompts (gap-28ceb9).
//!
//! A prompt's planned model is a preference, as on Graph runs: a provider
//! that cannot take the prompt is passed over before the call, and one that
//! refuses it with a usage exhaustion hands it to the next usable model in
//! the same turn. `roko_learn::provider_failover` holds the policy; an
//! explicit model selection or an experiment's model pins the prompt.
//!
//! The editor sees one turn. An attempt's events are forwarded as they come,
//! except an exhaustion failure that ends the attempt before it showed any of
//! its answer, which is held back while the next model runs.

use roko_agent::provider::error_classify::detect_provider_exhaustion;
use roko_core::agent::ProviderKind;
use roko_core::config::schema::RokoConfig;
use tokio::sync::mpsc;

use super::{CognitiveEvent, send_cognitive_event};

/// The config an ACP prompt's failover picks models from: `config` with the
/// Anthropic API providers that only a key in the environment synthesizes
/// disabled, since ACP's Anthropic path dispatches configured providers only.
pub(crate) fn failover_config(config: &RokoConfig) -> RokoConfig {
    let mut failover = config.clone();
    let synthesized: Vec<String> = config
        .effective_providers()
        .into_iter()
        .filter(|(id, provider)| {
            provider.kind == ProviderKind::AnthropicApi && !config.providers.contains_key(id)
        })
        .map(|(id, _)| id)
        .collect();
    failover.routing.disabled_providers.extend(synthesized);
    failover
}

/// Whether `event` shows the editor any of an attempt's answer, after which
/// the attempt's failure is the turn's.
fn shows_answer(event: &CognitiveEvent) -> bool {
    !matches!(
        event,
        CognitiveEvent::McpStatus { .. } | CognitiveEvent::PlanUpdate { .. }
    )
}

/// Forward one attempt's `events` to the editor's `sender`, holding back a
/// usage-exhaustion failure that ends the attempt before it showed any of its
/// answer, so the prompt can still move to another model. Returns the
/// held-back failure's message; the attempt's events after it are dropped.
pub(crate) async fn forward_attempt_events(
    mut events: mpsc::Receiver<CognitiveEvent>,
    sender: &mpsc::Sender<CognitiveEvent>,
) -> Option<String> {
    let mut answered = false;
    let mut withheld = None;
    while let Some(event) = events.recv().await {
        if withheld.is_some() {
            continue;
        }
        if let CognitiveEvent::Failure { message } = &event
            && !answered
            && detect_provider_exhaustion(message).is_some()
        {
            withheld = Some(message.clone());
            continue;
        }
        answered |= shows_answer(&event);
        send_cognitive_event(sender, event).await;
    }
    withheld
}

#[cfg(test)]
mod tests {
    use super::*;

    const SESSION_LIMIT: &str =
        "Error: model stream failed: You’ve hit your session limit · resets 4pm";

    /// What forwarding `events` holds back, and what reaches the editor.
    async fn forward(events: Vec<CognitiveEvent>) -> (Option<String>, Vec<CognitiveEvent>) {
        let (attempt_sender, attempt_events) = mpsc::channel(16);
        for event in events {
            attempt_sender.send(event).await.expect("queue the event");
        }
        drop(attempt_sender);
        let (sender, mut receiver) = mpsc::channel(16);
        let withheld = forward_attempt_events(attempt_events, &sender).await;
        drop(sender);
        let mut forwarded = Vec::new();
        while let Some(event) = receiver.recv().await {
            forwarded.push(event);
        }
        (withheld, forwarded)
    }

    /// An exhaustion that ends an attempt before its answer started is held
    /// back, with whatever the attempt sends after it.
    #[tokio::test]
    async fn a_leading_exhaustion_failure_is_held_back() {
        let (withheld, forwarded) = forward(vec![
            CognitiveEvent::McpStatus {
                statuses: Vec::new(),
            },
            CognitiveEvent::Failure {
                message: SESSION_LIMIT.to_string(),
            },
            CognitiveEvent::TokenChunk("late".to_string()),
        ])
        .await;

        assert_eq!(withheld.as_deref(), Some(SESSION_LIMIT));
        assert_eq!(forwarded.len(), 1);
        assert!(matches!(forwarded[0], CognitiveEvent::McpStatus { .. }));
    }

    /// Once the answer started, a failure is the turn's, and any failure that
    /// is no exhaustion is forwarded as it is.
    #[tokio::test]
    async fn other_failures_reach_the_editor() {
        let (withheld, forwarded) = forward(vec![
            CognitiveEvent::TokenChunk("partial".to_string()),
            CognitiveEvent::Failure {
                message: SESSION_LIMIT.to_string(),
            },
        ])
        .await;
        assert_eq!(withheld, None);
        assert_eq!(forwarded.len(), 2);
        assert!(matches!(forwarded[1], CognitiveEvent::Failure { .. }));

        let (withheld, forwarded) = forward(vec![CognitiveEvent::Failure {
            message: "Error: connection refused".to_string(),
        }])
        .await;
        assert_eq!(withheld, None);
        assert_eq!(forwarded.len(), 1);
    }

    /// The Anthropic API provider a key in the environment synthesizes is
    /// never a failover candidate; a configured one stays usable.
    #[test]
    fn failover_config_disables_only_synthesized_anthropic_providers() {
        let mut config = RokoConfig::default();
        config.providers.clear();
        let failover = failover_config(&config);
        let synthesized = config.effective_providers().contains_key("anthropic");
        assert_eq!(
            failover
                .routing
                .disabled_providers
                .contains(&"anthropic".to_string()),
            synthesized
        );

        config.providers.insert(
            "anthropic".to_string(),
            roko_core::config::schema::ProviderConfig {
                kind: ProviderKind::AnthropicApi,
                api_key_env: Some("PATH".to_string()),
                ..Default::default()
            },
        );
        let failover = failover_config(&config);
        assert!(
            !failover
                .routing
                .disabled_providers
                .contains(&"anthropic".to_string())
        );
    }
}
