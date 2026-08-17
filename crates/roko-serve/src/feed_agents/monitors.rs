//! Meta monitoring agents.

use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use tokio::time::interval;

use super::{FeedAgent, FeedAgentContext, FeedDescriptor};

// ---------------------------------------------------------------------------
// #12 — Agent Monitor
// ---------------------------------------------------------------------------

pub struct AgentMonitorAgent;

impl FeedAgent for AgentMonitorAgent {
    fn agent_id(&self) -> &'static str {
        "agent-monitor"
    }
    fn display_name(&self) -> &'static str {
        "Agent Monitor"
    }
    fn capabilities(&self) -> Vec<&str> {
        vec!["meta", "monitoring", "agents"]
    }
    fn feeds(&self) -> Vec<FeedDescriptor> {
        vec![FeedDescriptor {
            feed_id: "meta-agents".into(),
            topic: "feed:meta:agents".into(),
            name: "Agent Monitor".into(),
            description: "Agent count and heartbeat status".into(),
            kind: "meta".into(),
            rate: "10s".into(),
            schema: None,
        }]
    }
    fn run(
        self: Arc<Self>,
        ctx: FeedAgentContext,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        Box::pin(async move {
            let mut tick = interval(Duration::from_secs(10));
            loop {
                tokio::select! {
                    _ = ctx.cancel.cancelled() => break,
                    _ = tick.tick() => {
                        let agent_count = ctx.state.agent_count.load(std::sync::atomic::Ordering::Relaxed);
                        let heartbeats_len = ctx.state.heartbeats.read().await.len();
                        let catalog = ctx.state.feed_agent_catalog.read().await;
                        let feed_agent_count = catalog.agents.len();
                        let total_feeds = catalog.feeds.len();
                        drop(catalog);

                        let payload = json!({
                            "relay_agent_count": agent_count,
                            "heartbeat_buffer_size": heartbeats_len,
                            "feed_agent_count": feed_agent_count,
                            "total_feeds": total_feeds,
                        });
                        ctx.publish_tick(
                            self.agent_id(),
                            "meta-agents",
                            "feed:meta:agents",
                            payload,
                        );
                    }
                }
            }
        })
    }
}

// ---------------------------------------------------------------------------
// #15 — Relay Stats Agent
// ---------------------------------------------------------------------------

pub struct RelayStatsAgent;

impl FeedAgent for RelayStatsAgent {
    fn agent_id(&self) -> &'static str {
        "relay-stats"
    }
    fn display_name(&self) -> &'static str {
        "Relay Stats"
    }
    fn capabilities(&self) -> Vec<&str> {
        vec!["meta", "relay", "stats"]
    }
    fn feeds(&self) -> Vec<FeedDescriptor> {
        vec![FeedDescriptor {
            feed_id: "meta-relay".into(),
            topic: "feed:meta:relay".into(),
            name: "Relay Stats".into(),
            description: "Relay connection health and throughput stats".into(),
            kind: "meta".into(),
            rate: "10s".into(),
            schema: None,
        }]
    }
    fn run(
        self: Arc<Self>,
        ctx: FeedAgentContext,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        Box::pin(async move {
            let mut tick = interval(Duration::from_secs(10));
            loop {
                tokio::select! {
                    _ = ctx.cancel.cancelled() => break,
                    _ = tick.tick() => {
                        let connection = {
                            let rh = ctx.state.relay_health.read();
                            serde_json::to_value(&rh.connection).unwrap_or_default()
                        };

                        let catalog = ctx.state.feed_agent_catalog.read().await;
                        let msgs_per_sec = catalog.messages_per_sec;
                        let total_feeds = catalog.feeds.len();
                        drop(catalog);

                        let payload = json!({
                            "relay_connection": connection,
                            "total_feeds": total_feeds,
                            "est_messages_per_sec": msgs_per_sec,
                        });
                        ctx.publish_tick(
                            self.agent_id(),
                            "meta-relay",
                            "feed:meta:relay",
                            payload,
                        );
                    }
                }
            }
        })
    }
}

// ---------------------------------------------------------------------------
// System heartbeat
// ---------------------------------------------------------------------------

pub struct SystemHeartbeatAgent;

impl FeedAgent for SystemHeartbeatAgent {
    fn agent_id(&self) -> &'static str {
        "system-heartbeat"
    }
    fn display_name(&self) -> &'static str {
        "System Heartbeat"
    }
    fn capabilities(&self) -> Vec<&str> {
        vec!["meta", "heartbeat", "health"]
    }
    fn feeds(&self) -> Vec<FeedDescriptor> {
        vec![FeedDescriptor {
            feed_id: "meta-heartbeat".into(),
            topic: "feed:meta:heartbeat".into(),
            name: "System Heartbeat".into(),
            description: "Aggregate system health: uptime, feed agent count, event throughput"
                .into(),
            kind: "meta".into(),
            rate: "5s".into(),
            schema: None,
        }]
    }
    fn run(
        self: Arc<Self>,
        ctx: FeedAgentContext,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        Box::pin(async move {
            let mut tick = interval(Duration::from_secs(5));
            let start = std::time::Instant::now();
            let mut tick_count: u64 = 0;
            loop {
                tokio::select! {
                    _ = ctx.cancel.cancelled() => break,
                    _ = tick.tick() => {
                        tick_count += 1;
                        let uptime_secs = start.elapsed().as_secs();
                        let catalog = ctx.state.feed_agent_catalog.read().await;
                        let active_agents = catalog.agents.len();
                        let total_feeds = catalog.feeds.len();
                        let msgs_per_sec = catalog.messages_per_sec;
                        drop(catalog);
                        let agent_count = ctx.state.agent_count.load(std::sync::atomic::Ordering::Relaxed);
                        let payload = json!({
                            "uptime_secs": uptime_secs,
                            "uptime_human": format!("{}h {}m {}s", uptime_secs / 3600, (uptime_secs % 3600) / 60, uptime_secs % 60),
                            "feed_agents_active": active_agents,
                            "total_feeds": total_feeds,
                            "relay_agents": agent_count,
                            "est_msgs_per_sec": msgs_per_sec,
                            "heartbeat_seq": tick_count,
                        });
                        ctx.publish_tick(self.agent_id(), "meta-heartbeat", "feed:meta:heartbeat", payload);
                    }
                }
            }
        })
    }
}
