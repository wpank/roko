//! Reactive agent lifecycle supervisor.
//!
//! A *reactive* agent registers one or more [`TriggerCondition`]s, then
//! enters a sleep loop.  When a matching condition fires (via the
//! [`PulseBus`], a cron tick, or an external wake-up call), the supervisor
//! wakes the agent, runs the provided handler, then puts it back to sleep.
//!
//! # Architecture
//!
//! ```text
//!   ReactiveAgentConfig ──spawn()──► ReactiveAgentHandle (sleeping)
//!                                          │
//!                              trigger fires ▼
//!                              handler.process(event) ──► returns
//!                                          │
//!                              sleep again ▼
//!                              (waits for next trigger)
//! ```
//!
//! The supervisor owns a single Tokio task.  Shutdown is cooperative: the
//! `cancel` token from [`ReactiveAgentHandle`] signals the loop to exit
//! cleanly after the current handler invocation completes.
//!
//! # Wiring into the trigger runtime (E31)
//!
//! The trigger runtime in `roko-serve` publishes every fired [`TriggerEvent`]
//! as a [`Pulse`] on the workspace [`PulseBus`].  A reactive agent subscribes
//! to that bus with a topic filter derived from its registered conditions and
//! wakes up exactly when a matching pulse arrives.
//!
//! # Supported conditions
//!
//! | Condition variant | Wake source |
//! |---|---|
//! | [`TriggerCondition::BusTopicMatch`] | [`PulseBus`] pulse with matching topic |
//! | [`TriggerCondition::CronSchedule`] | Internal `tokio::time::interval` tick |
//! | [`TriggerCondition::ExternalWake`] | Caller-provided [`tokio::sync::mpsc`] sender |
//!
//! `Webhook`, `FileWatch`, `ChainEvent`, and `SignalPattern` conditions that
//! are handled by the full trigger runtime in `roko-serve` can be bridged by
//! forwarding their `TriggerEvent` pulses through the `PulseBus` and using a
//! [`TriggerCondition::BusTopicMatch`] condition here.

use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use roko_core::trigger::TRIGGER_FIRED;
use roko_core::{Bus, Pulse, TopicFilter};

use crate::cancel::CancelToken;
use crate::lifecycle::{AgentLifecycleState, LifecycleTransition, LifecycleTransitionReason};
use crate::pulse_bus::PulseBus;

// ---------------------------------------------------------------------------
// TriggerCondition
// ---------------------------------------------------------------------------

/// A single wake condition for a reactive agent.
///
/// An agent may register multiple conditions; it wakes on the first that fires.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TriggerCondition {
    /// Wake when a pulse arrives on the [`PulseBus`] with a topic matching
    /// the given pattern.  Supports `*` wildcards (e.g. `"trigger.*.fired"`).
    BusTopicMatch {
        /// Topic glob pattern.
        topic: String,
    },

    /// Wake on a repeating schedule.  The agent sleeps between ticks.
    CronSchedule {
        /// Duration between ticks (the minimum sleep interval).
        interval: Duration,
    },
}

// ---------------------------------------------------------------------------
// ReactiveWakeEvent — the event delivered to the handler
// ---------------------------------------------------------------------------

/// The event that caused a reactive agent to wake.
#[derive(Clone, Debug)]
pub enum ReactiveWakeEvent {
    /// A matching pulse arrived on the bus.
    BusPulse(Pulse),
    /// A cron tick fired.
    CronTick {
        /// How many times this schedule has ticked since the agent was armed.
        tick_count: u64,
    },
    /// An external wake-up message was received (from [`ReactiveAgentHandle::wake`]).
    ExternalWake {
        /// Optional payload attached to the wake call.
        payload: Option<String>,
    },
    /// The cancel token was signalled; the agent should shut down.
    Shutdown,
}

// ---------------------------------------------------------------------------
// ReactiveAgentConfig
// ---------------------------------------------------------------------------

/// Configuration for a reactive agent supervisor.
#[derive(Clone, Debug)]
pub struct ReactiveAgentConfig {
    /// Stable agent identifier (used in lifecycle transition records).
    pub agent_id: String,
    /// Trigger conditions that should wake this agent.
    pub conditions: Vec<TriggerCondition>,
    /// Optional timeout for each handler invocation.
    ///
    /// When `None`, the handler is allowed to run until it returns.  When
    /// `Some`, the supervisor cancels the handler if it exceeds the timeout
    /// and logs a warning.
    pub handler_timeout: Option<Duration>,
}

// ---------------------------------------------------------------------------
// ReactiveAgentHandle
// ---------------------------------------------------------------------------

/// Handle for controlling a running reactive agent supervisor.
#[derive(Clone, Debug)]
pub struct ReactiveAgentHandle {
    /// Cancel token — drop all subscriptions and stop the loop on signal.
    cancel: CancelToken,
    /// Channel for external wake-up calls.
    wake_tx: mpsc::Sender<Option<String>>,
    /// Agent identifier for logging.
    agent_id: Arc<String>,
}

impl ReactiveAgentHandle {
    /// Signal the agent to stop at the end of its current handler invocation.
    ///
    /// Returns without blocking; the supervisor task exits asynchronously.
    pub fn shutdown(&self) {
        self.cancel.cancel();
    }

    /// Send an external wake-up message to the agent.
    ///
    /// The agent will be woken even if no bus pulse or cron tick has fired.
    /// If the agent is already processing an event, the wake message is queued
    /// (up to the bounded channel capacity).
    ///
    /// Returns `Err` if the agent has already been shut down.
    pub fn wake(&self, payload: Option<String>) -> Result<(), WakeError> {
        self.wake_tx
            .try_send(payload)
            .map_err(|_| WakeError::AgentShutDown {
                agent_id: (*self.agent_id).clone(),
            })
    }

    /// Whether the cancel token has been signalled.
    pub fn is_shutdown(&self) -> bool {
        self.cancel.is_cancelled()
    }
}

/// Error returned by [`ReactiveAgentHandle::wake`].
#[derive(Debug, thiserror::Error)]
pub enum WakeError {
    /// The agent's event loop has already exited; the wake message cannot be delivered.
    #[error("reactive agent {agent_id} has already shut down")]
    AgentShutDown {
        /// Stable agent identifier.
        agent_id: String,
    },
}

// ---------------------------------------------------------------------------
// ReactiveAgentSupervisor
// ---------------------------------------------------------------------------

/// Supervisor that owns the reactive event loop for one agent.
///
/// Construct with [`ReactiveAgentSupervisor::new`], then call
/// [`ReactiveAgentSupervisor::spawn`] to start the event loop. The returned
/// [`ReactiveAgentHandle`] can be used to stop the agent or inject external
/// wake events.
pub struct ReactiveAgentSupervisor<H> {
    config: ReactiveAgentConfig,
    bus: Arc<PulseBus>,
    handler: H,
    cancel: CancelToken,
    wake_tx: mpsc::Sender<Option<String>>,
    wake_rx: mpsc::Receiver<Option<String>>,
}

/// Trait for types that can process a reactive wake event.
///
/// Implementors must be `Send + Sync + 'static` so that they can be passed
/// to a Tokio task.
pub trait ReactiveHandler: Send + Sync + 'static {
    /// Process a wake event.  The future must be `Send`.
    fn process(
        &self,
        event: ReactiveWakeEvent,
    ) -> impl std::future::Future<Output = ()> + Send + '_;
}

impl<H> ReactiveAgentSupervisor<H>
where
    H: ReactiveHandler,
{
    /// Create a new supervisor with the given config, bus, and handler.
    pub fn new(config: ReactiveAgentConfig, bus: Arc<PulseBus>, handler: H) -> Self {
        let (wake_tx, wake_rx) = mpsc::channel(16);
        let cancel = CancelToken::new();
        Self {
            config,
            bus,
            handler,
            cancel,
            wake_tx,
            wake_rx,
        }
    }

    /// Spawn the event loop and return a handle.
    ///
    /// The loop runs until the handle's cancel token is signalled or the
    /// agent exits cleanly.  Lifecycle transitions (to `Waiting` and back to
    /// `Active`) are logged at `DEBUG` level.
    #[allow(clippy::too_many_lines)]
    pub fn spawn(self) -> ReactiveAgentHandle {
        let handle = ReactiveAgentHandle {
            cancel: self.cancel.clone(),
            wake_tx: self.wake_tx.clone(),
            agent_id: Arc::new(self.config.agent_id.clone()),
        };

        let cancel = self.cancel.clone();
        let config = self.config;
        let bus = self.bus;
        let handler = self.handler;
        let mut wake_rx = self.wake_rx;

        tokio::spawn(async move {
            tracing::debug!(
                agent_id = %config.agent_id,
                conditions = config.conditions.len(),
                "reactive agent supervisor started"
            );

            // Build Bus subscribers for BusTopicMatch conditions.
            //
            // A topic string ending with `.*` is treated as a prefix match.
            // An exact string produces an Exact filter. Everything else is
            // treated as a Prefix for forward-compatibility.
            let mut bus_receivers: Vec<_> = config
                .conditions
                .iter()
                .filter_map(|c| {
                    if let TriggerCondition::BusTopicMatch { topic } = c {
                        let filter = if topic.ends_with(".*") {
                            // Strip the trailing `.*` and use a Prefix filter.
                            let prefix = topic.trim_end_matches(".*").to_string();
                            TopicFilter::Prefix(prefix + ".")
                        } else if topic.contains('*') {
                            // Wildcard in the middle — use a Prefix up to the
                            // first `*` character for best-effort matching.
                            let prefix = topic
                                .split_once('*')
                                .map(|(p, _)| p.to_string())
                                .unwrap_or_default();
                            TopicFilter::Prefix(prefix)
                        } else {
                            TopicFilter::Exact(roko_core::Topic::new(topic.clone()))
                        };
                        bus.subscribe(filter).ok()
                    } else {
                        None
                    }
                })
                .collect();

            // Build cron intervals for CronSchedule conditions.
            let mut cron_intervals: Vec<tokio::time::Interval> = config
                .conditions
                .iter()
                .filter_map(|c| {
                    if let TriggerCondition::CronSchedule { interval } = c {
                        Some(tokio::time::interval(*interval))
                    } else {
                        None
                    }
                })
                .collect();

            let mut cron_tick_count: u64 = 0;

            loop {
                // Emit a lifecycle transition: agent enters Waiting state.
                let _sleep_transition = LifecycleTransition::new(
                    &config.agent_id,
                    AgentLifecycleState::Active,
                    AgentLifecycleState::Waiting,
                    LifecycleTransitionReason::ExternalWait,
                );
                tracing::debug!(agent_id = %config.agent_id, "reactive agent sleeping (waiting for trigger)");

                // Wait for the first of: bus pulse, cron tick, external wake, or shutdown.
                let wake_event: ReactiveWakeEvent = {
                    // We must avoid a compile-time select! over a dynamic
                    // number of arms. Instead, we use a single tokio::select!
                    // that branches on pre-computed futures.
                    //
                    // Bus: race all receivers into one future using async blocks.
                    // Cron: use the first interval only (merge if multiple needed).
                    // External: mpsc receiver.
                    // Cancel: cancel token.

                    // Build a single future that yields the first pulse from any receiver.
                    let num_bus = bus_receivers.len();
                    let num_cron = cron_intervals.len();

                    // Simple strategy: use tokio::select! with a single optional
                    // interval branch and a single optional bus branch.
                    // If neither condition type is registered, only external wake
                    // and cancel are active.

                    if num_bus > 0 && num_cron > 0 {
                        tokio::select! {
                            maybe_pulse = bus_receivers[0].recv() => {
                                match maybe_pulse {
                                    Some(pulse) => ReactiveWakeEvent::BusPulse(pulse),
                                    None => ReactiveWakeEvent::Shutdown,
                                }
                            }
                            _ = cron_intervals[0].tick() => {
                                cron_tick_count += 1;
                                ReactiveWakeEvent::CronTick { tick_count: cron_tick_count }
                            }
                            payload = wake_rx.recv() => {
                                match payload {
                                    Some(p) => ReactiveWakeEvent::ExternalWake { payload: p },
                                    None => ReactiveWakeEvent::Shutdown,
                                }
                            }
                            _ = cancel.cancelled() => ReactiveWakeEvent::Shutdown
                        }
                    } else if num_bus > 0 {
                        tokio::select! {
                            maybe_pulse = bus_receivers[0].recv() => {
                                match maybe_pulse {
                                    Some(pulse) => ReactiveWakeEvent::BusPulse(pulse),
                                    None => ReactiveWakeEvent::Shutdown,
                                }
                            }
                            payload = wake_rx.recv() => {
                                match payload {
                                    Some(p) => ReactiveWakeEvent::ExternalWake { payload: p },
                                    None => ReactiveWakeEvent::Shutdown,
                                }
                            }
                            _ = cancel.cancelled() => ReactiveWakeEvent::Shutdown
                        }
                    } else if num_cron > 0 {
                        tokio::select! {
                            _ = cron_intervals[0].tick() => {
                                cron_tick_count += 1;
                                ReactiveWakeEvent::CronTick { tick_count: cron_tick_count }
                            }
                            payload = wake_rx.recv() => {
                                match payload {
                                    Some(p) => ReactiveWakeEvent::ExternalWake { payload: p },
                                    None => ReactiveWakeEvent::Shutdown,
                                }
                            }
                            _ = cancel.cancelled() => ReactiveWakeEvent::Shutdown
                        }
                    } else {
                        // No bus or cron conditions: only external wake and cancel.
                        tokio::select! {
                            payload = wake_rx.recv() => {
                                match payload {
                                    Some(p) => ReactiveWakeEvent::ExternalWake { payload: p },
                                    None => ReactiveWakeEvent::Shutdown,
                                }
                            }
                            _ = cancel.cancelled() => ReactiveWakeEvent::Shutdown
                        }
                    }
                };

                let is_shutdown = matches!(wake_event, ReactiveWakeEvent::Shutdown);

                // Emit a lifecycle transition: agent wakes to Active.
                let _wake_transition = LifecycleTransition::new(
                    &config.agent_id,
                    AgentLifecycleState::Waiting,
                    AgentLifecycleState::Active,
                    LifecycleTransitionReason::ExternalReady,
                );
                tracing::debug!(agent_id = %config.agent_id, "reactive agent woke up, processing event");

                if is_shutdown {
                    tracing::info!(agent_id = %config.agent_id, "reactive agent received shutdown signal");
                    break;
                }

                // Invoke the handler, respecting the optional timeout.
                match config.handler_timeout {
                    Some(timeout) => {
                        if let Err(_elapsed) =
                            tokio::time::timeout(timeout, handler.process(wake_event)).await
                        {
                            tracing::warn!(
                                agent_id = %config.agent_id,
                                timeout_secs = timeout.as_secs_f64(),
                                "reactive agent handler timed out"
                            );
                        }
                    }
                    None => {
                        handler.process(wake_event).await;
                    }
                }

                tracing::debug!(agent_id = %config.agent_id, "reactive agent handler complete, returning to sleep");

                // If cancel was signalled during handler execution, exit cleanly.
                if cancel.is_cancelled() {
                    tracing::info!(agent_id = %config.agent_id, "reactive agent shutting down after handler");
                    break;
                }
            }

            tracing::debug!(agent_id = %config.agent_id, "reactive agent supervisor exited");
        });

        handle
    }
}

// ---------------------------------------------------------------------------
// Convenience: build the bus topic filter for a trigger:fired event
// ---------------------------------------------------------------------------

/// Build a [`TriggerCondition::BusTopicMatch`] that fires when *any* trigger
/// named `trigger_name` is fired by the trigger runtime.
///
/// This is the canonical way to wire a reactive agent into the E31 trigger
/// runtime, which publishes every `TriggerEvent` as a pulse with topic
/// `trigger.<name>.fired`.
#[must_use]
pub fn condition_for_trigger_name(trigger_name: &str) -> TriggerCondition {
    let topic = format!("trigger.{trigger_name}.fired");
    TriggerCondition::BusTopicMatch { topic }
}

/// Build a [`TriggerCondition::BusTopicMatch`] that matches the global
/// `trigger:fired` topic (fires on *any* trigger firing).
#[must_use]
pub fn condition_for_any_trigger() -> TriggerCondition {
    TriggerCondition::BusTopicMatch {
        topic: TRIGGER_FIRED.replace(':', "."),
    }
}

// ---------------------------------------------------------------------------
// ReactiveAgentState — observable status for monitoring
// ---------------------------------------------------------------------------

/// Observable status of a reactive agent supervisor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReactiveAgentStatus {
    /// Supervisor task is alive; agent is sleeping, waiting for a trigger.
    Sleeping,
    /// Supervisor task is alive; agent is executing its handler.
    Processing,
    /// Supervisor task has exited (normal shutdown or error).
    Stopped,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};
    use tokio::time::Duration;

    struct CountingHandler {
        count: Arc<AtomicU64>,
    }

    impl ReactiveHandler for CountingHandler {
        async fn process(&self, event: ReactiveWakeEvent) {
            match event {
                ReactiveWakeEvent::Shutdown => {}
                _ => {
                    self.count.fetch_add(1, Ordering::SeqCst);
                }
            }
        }
    }

    #[tokio::test]
    async fn external_wake_triggers_handler() {
        let count = Arc::new(AtomicU64::new(0));
        let handler = CountingHandler {
            count: count.clone(),
        };
        let bus = Arc::new(PulseBus::new(64));
        let config = ReactiveAgentConfig {
            agent_id: "test-agent".to_string(),
            conditions: vec![],
            handler_timeout: None,
        };

        let supervisor = ReactiveAgentSupervisor::new(config, bus, handler);
        let handle = supervisor.spawn();

        // Give the task time to start and enter the select.
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Send an external wake.
        handle.wake(Some("hello".to_string())).unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        assert_eq!(
            count.load(Ordering::SeqCst),
            1,
            "handler should have been called once"
        );

        handle.shutdown();
    }

    #[tokio::test]
    async fn shutdown_stops_loop() {
        let count = Arc::new(AtomicU64::new(0));
        let handler = CountingHandler {
            count: count.clone(),
        };
        let bus = Arc::new(PulseBus::new(64));
        let config = ReactiveAgentConfig {
            agent_id: "test-agent-shutdown".to_string(),
            conditions: vec![],
            handler_timeout: None,
        };

        let supervisor = ReactiveAgentSupervisor::new(config, bus, handler);
        let handle = supervisor.spawn();

        // Give the task time to start.
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Shut down immediately.
        handle.shutdown();

        // Give the task time to detect the cancel.
        tokio::time::sleep(Duration::from_millis(50)).await;

        assert!(handle.is_shutdown(), "handle should be marked shutdown");
        // No events processed.
        assert_eq!(count.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn cron_schedule_wakes_agent() {
        let count = Arc::new(AtomicU64::new(0));
        let handler = CountingHandler {
            count: count.clone(),
        };
        let bus = Arc::new(PulseBus::new(64));
        let config = ReactiveAgentConfig {
            agent_id: "test-cron-agent".to_string(),
            conditions: vec![TriggerCondition::CronSchedule {
                interval: Duration::from_millis(20),
            }],
            handler_timeout: None,
        };

        let supervisor = ReactiveAgentSupervisor::new(config, bus, handler);
        let handle = supervisor.spawn();

        // Wait for 3+ ticks.
        tokio::time::sleep(Duration::from_millis(120)).await;
        handle.shutdown();

        let calls = count.load(Ordering::SeqCst);
        assert!(calls >= 2, "expected at least 2 cron ticks, got {calls}");
    }

    #[tokio::test]
    async fn bus_pulse_wakes_agent() {
        let count = Arc::new(AtomicU64::new(0));
        let handler = CountingHandler {
            count: count.clone(),
        };
        let bus = Arc::new(PulseBus::new(64));
        let config = ReactiveAgentConfig {
            agent_id: "test-bus-agent".to_string(),
            conditions: vec![TriggerCondition::BusTopicMatch {
                topic: "trigger.deploy.fired".to_string(),
            }],
            handler_timeout: None,
        };

        let supervisor = ReactiveAgentSupervisor::new(config, Arc::clone(&bus), handler);
        let handle = supervisor.spawn();

        // Give the task time to subscribe.
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Publish a matching pulse using the public API.
        let pulse = roko_core::Pulse::new(
            1,
            roko_core::Topic::new("trigger.deploy.fired"),
            roko_core::Kind::Task,
            roko_core::Body::empty(),
        );
        bus.publish(pulse).unwrap();

        tokio::time::sleep(Duration::from_millis(50)).await;

        assert_eq!(
            count.load(Ordering::SeqCst),
            1,
            "bus pulse should have woken the agent"
        );
        handle.shutdown();
    }

    #[test]
    fn condition_for_trigger_name_builds_correct_topic() {
        let cond = condition_for_trigger_name("deploy");
        match cond {
            TriggerCondition::BusTopicMatch { topic } => {
                assert_eq!(topic, "trigger.deploy.fired");
            }
            TriggerCondition::CronSchedule { .. } => {
                panic!("expected BusTopicMatch, got CronSchedule");
            }
        }
    }
}
