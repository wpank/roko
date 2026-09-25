//! Channel-to-reactive-agent binding (#428).
//!
//! [`ChannelBindingRouter`] ties a `[[channels]]` config table to the
//! [`ReactiveAgentSupervisor`] infrastructure: when a message arrives on a
//! bound platform channel, the router wakes the associated reactive agent by
//! publishing a [`Pulse`] on the workspace [`PulseBus`] with the canonical
//! topic `platform.message.<platform_id>.<channel_id>` and by injecting a
//! direct external wake into the agent's handle.
//!
//! # Design
//!
//! The router does **not** own any platform connections.  It sits between the
//! [`ChatBridge`] incoming-message stream and the [`ReactiveAgentHandle`]
//! registry:
//!
//! ```text
//!   ChatBridge::incoming() ──► ChannelBindingRouter::route()
//!                                       │
//!                    ┌──────────────────┴───────────────────┐
//!                    ▼                                       ▼
//!             PulseBus.publish(…)                ReactiveAgentHandle.wake(…)
//! ```
//!
//! # Usage
//!
//! ```rust,ignore
//! use std::sync::Arc;
//! use roko_runtime::channel_binding::ChannelBindingRouter;
//! use roko_runtime::reactive_agent::ReactiveAgentHandle;
//! use roko_core::config::channels::ChannelConfig;
//!
//! let mut router = ChannelBindingRouter::new(bus.clone());
//!
//! // Register the binding declared in roko.toml.
//! router.add_binding(channel_cfg, agent_handle);
//!
//! // When a message arrives on the platform, call:
//! router.route("slack-ops", "C01AB", "msg-42", "hello from Slack").await;
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use roko_core::config::channels::ChannelConfig;
use roko_core::{Body, Bus, Kind, Pulse, Topic};

use crate::pulse_bus::PulseBus;
use crate::reactive_agent::ReactiveAgentHandle;

// ---------------------------------------------------------------------------
// ChannelBindingRouter
// ---------------------------------------------------------------------------

/// Key used to look up a binding: `(platform_id, channel_id)`.
type BindingKey = (String, String);

/// Binding entry: optional agent handle + list of trigger binding names.
struct BindingEntry {
    agent_handle: Option<ReactiveAgentHandle>,
    trigger_bindings: Vec<String>,
}

/// Routes incoming platform messages to reactive agents and trigger bindings.
///
/// Cheaply cloneable via `Arc`.
pub struct ChannelBindingRouter {
    bus: Arc<PulseBus>,
    bindings: HashMap<BindingKey, BindingEntry>,
}

impl ChannelBindingRouter {
    /// Construct an empty router backed by `bus`.
    #[must_use]
    pub fn new(bus: Arc<PulseBus>) -> Self {
        Self {
            bus,
            bindings: HashMap::new(),
        }
    }

    /// Register a channel binding from config.
    ///
    /// `agent_handle` may be `None` when the channel only has trigger bindings
    /// (no agent wiring).
    pub fn add_binding(
        &mut self,
        config: &ChannelConfig,
        agent_handle: Option<ReactiveAgentHandle>,
    ) {
        let key = (config.platform_id.clone(), config.channel_id.clone());
        self.bindings.insert(
            key,
            BindingEntry {
                agent_handle,
                trigger_bindings: config.trigger_bindings.clone(),
            },
        );
    }

    /// Return the number of registered bindings.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    /// Return `true` if no bindings are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }

    /// Route an incoming message to any registered binding.
    ///
    /// 1. Publishes a pulse with topic
    ///    `platform.message.<platform_id>.<channel_id>` on the bus.
    /// 2. If an agent handle is registered for this `(platform_id, channel_id)`
    ///    pair, sends an external wake with the message text as payload.
    ///
    /// Does nothing (no error) if no binding is registered for the given key.
    pub async fn route(
        &self,
        platform_id: &str,
        channel_id: &str,
        message_id: &str,
        text: &str,
    ) {
        let topic_str = format!("platform.message.{platform_id}.{channel_id}");
        let topic = Topic::new(topic_str.clone());

        // Build a pulse payload carrying the message id.
        let body = Body::from_json(&serde_json::json!({
            "message_id": message_id,
            "platform_id": platform_id,
            "channel_id": channel_id,
        }))
        .unwrap_or(Body::empty());

        let pulse = Pulse::new(0, topic, Kind::Task, body);
        // Best-effort: ignore capacity errors from the bus.
        let _ = self.bus.publish(pulse);

        let key = (platform_id.to_string(), channel_id.to_string());
        if let Some(entry) = self.bindings.get(&key) {
            // Publish trigger-binding pulses.
            for trigger_name in &entry.trigger_bindings {
                let trigger_topic = Topic::new(format!("trigger.{trigger_name}.fired"));
                let trigger_body = Body::from_json(&serde_json::json!({
                    "source": "channel_message",
                    "platform_id": platform_id,
                    "channel_id": channel_id,
                }))
                .unwrap_or(Body::empty());
                let trigger_pulse = Pulse::new(0, trigger_topic, Kind::Task, trigger_body);
                let _ = self.bus.publish(trigger_pulse);
            }

            // Wake the agent directly.
            if let Some(handle) = &entry.agent_handle {
                let _ = handle.wake(Some(text.to_string()));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::config::channels::ChannelConfig;

    use crate::pulse_bus::PulseBus;

    fn slack_channel() -> ChannelConfig {
        ChannelConfig {
            platform_id: "slack-ops".to_string(),
            channel_id: "C01AB".to_string(),
            agent_name: Some("ops-agent".to_string()),
            trigger_bindings: vec!["deploy-hook".to_string()],
        }
    }

    #[test]
    fn router_starts_empty() {
        let bus = Arc::new(PulseBus::new(64));
        let router = ChannelBindingRouter::new(bus);
        assert!(router.is_empty());
        assert_eq!(router.len(), 0);
    }

    #[tokio::test]
    async fn add_binding_registers_entry() {
        let bus = Arc::new(PulseBus::new(64));
        let mut router = ChannelBindingRouter::new(bus);
        router.add_binding(&slack_channel(), None);
        assert_eq!(router.len(), 1);
    }

    #[tokio::test]
    async fn route_publishes_pulse_on_bus() {
        let bus = Arc::new(PulseBus::new(64));
        // Subscribe before routing so we can receive the pulse.
        let mut rx = bus
            .subscribe(roko_core::TopicFilter::Prefix("platform.message.".to_string()))
            .unwrap();

        let mut router = ChannelBindingRouter::new(bus);
        router.add_binding(&slack_channel(), None);

        router.route("slack-ops", "C01AB", "msg-1", "hello").await;

        // The pulse should have been delivered to the subscriber.
        let pulse = rx.recv().await.unwrap();
        assert!(pulse.topic.to_string().contains("slack-ops"));
        assert!(pulse.topic.to_string().contains("C01AB"));
    }

    #[tokio::test]
    async fn route_with_no_binding_does_not_panic() {
        let bus = Arc::new(PulseBus::new(64));
        let router = ChannelBindingRouter::new(bus);
        // This should be a no-op, not a panic.
        router.route("discord-main", "12345", "msg-99", "hi").await;
    }
}
