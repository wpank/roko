//! Atomic ingestion boundary for agent lifecycle observations.
//!
//! Agent runtimes report a complete sample after committing a lifecycle
//! mutation or finishing a real decision cycle. This store validates and
//! durably commits that sample before forwarding passive lifecycle events to
//! the StateHub. The monotonically increasing transport sequence makes
//! retries idempotent without suppressing later cycles with identical values.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use roko_core::{LensScope, ObservableEvent};

macro_rules! observed_enum {
    ($name:ident { $($variant:ident => $wire:literal),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
        #[serde(rename_all = "lowercase")]
        pub enum $name {
            $($variant),+
        }

        impl $name {
            const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $wire),+
                }
            }
        }
    };
}

observed_enum!(ObservedAgentRegime {
    Calm => "calm",
    Normal => "normal",
    Volatile => "volatile",
    Crisis => "crisis",
});

observed_enum!(ObservedAgentMode {
    Ephemeral => "ephemeral",
    Persistent => "persistent",
    Reactive => "reactive",
});

observed_enum!(ObservedVitalityPhase {
    Thriving => "thriving",
    Stable => "stable",
    Conservation => "conservation",
    Declining => "declining",
    Terminal => "terminal",
});

observed_enum!(ObservedLifecycleState {
    Provisioning => "provisioning",
    Active => "active",
    Dreaming => "dreaming",
    Terminal => "terminal",
});

/// Canonical state committed by an agent slot owner.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "state", rename_all = "lowercase")]
pub enum ObservedSlotState {
    Idle,
    Active,
    Blocked { reason: String },
    Completed,
}

impl ObservedSlotState {
    fn event_value(&self) -> String {
        match self {
            Self::Idle => "idle".to_string(),
            Self::Active => "active".to_string(),
            Self::Blocked { reason } => format!("blocked({reason})"),
            Self::Completed => "completed".to_string(),
        }
    }
}

/// One slot mutation completed during an agent decision cycle.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AgentSlotObservation {
    /// Stable slot name within the agent.
    pub slot: String,
    /// New state committed by the slot owner. A blocked state carries its reason.
    #[serde(flatten)]
    pub state: ObservedSlotState,
}

/// Complete post-mutation or post-cycle observation reported by an agent runtime.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AgentRuntimeObservation {
    /// Monotonically increasing transport sequence, scoped to the agent.
    pub sequence: u64,
    /// Regime active after this mutation or completed cycle.
    pub regime: ObservedAgentRegime,
    /// Normalized agent vitality in the inclusive range `[0, 1]`.
    pub vitality: f64,
    /// Agent mode active after this mutation or completed cycle.
    pub mode: ObservedAgentMode,
    /// Vitality phase derived from `vitality`.
    pub phase: ObservedVitalityPhase,
    /// Type-state lifecycle state active after this mutation or completed cycle.
    pub lifecycle_state: ObservedLifecycleState,
    /// Present only when this observation closes a real cognitive tick.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_tick: Option<CompletedAgentTick>,
    /// Explicit slot mutations committed during this cycle.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub slot_updates: Vec<AgentSlotObservation>,
}

/// Measurements available only after a real cognitive tick completes.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CompletedAgentTick {
    /// Normalized prediction error measured for the completed cycle.
    pub prediction_error: f64,
}

/// Result of committing one runtime observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct AgentObservationCommit {
    /// Transport sequence accepted by the store.
    pub sequence: u64,
    /// Number of lifecycle events emitted after the commit.
    pub emitted_events: usize,
    /// Whether this request was an exact transport retry.
    pub duplicate: bool,
}

/// Rejection returned before an observation mutates the stored baseline.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentObservationError {
    /// The observation violates the lifecycle payload contract.
    Invalid(String),
    /// The sequence is older than the last committed cycle.
    Stale { received: u64, committed: u64 },
    /// The sequence was already committed with different contents.
    ConflictingRetry { sequence: u64 },
    /// The durable baseline could not be read or committed.
    Persistence(String),
}

impl std::fmt::Display for AgentObservationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(message) => f.write_str(message),
            Self::Stale {
                received,
                committed,
            } => write!(
                f,
                "agent observation sequence {received} is older than committed sequence {committed}"
            ),
            Self::ConflictingRetry { sequence } => write!(
                f,
                "agent observation sequence {sequence} was already committed with different contents"
            ),
            Self::Persistence(message) => write!(f, "persist agent lifecycle state: {message}"),
        }
    }
}

impl std::error::Error for AgentObservationError {}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct AgentLifecycleState {
    last_observation: AgentRuntimeObservation,
    slots: BTreeMap<String, ObservedSlotState>,
}

/// Durable owner of the latest committed lifecycle sample for each agent.
pub struct AgentLifecycleStore {
    states: Mutex<HashMap<String, AgentLifecycleState>>,
    path: PathBuf,
    restore_error: Option<String>,
    telemetry: roko_runtime::StateHubSender,
}

impl AgentLifecycleStore {
    /// Open a store and restore its last committed baselines.
    #[must_use]
    pub fn open(path: impl Into<PathBuf>, telemetry: roko_runtime::StateHubSender) -> Self {
        let path = path.into();
        let (states, restore_error) = match load_states(&path) {
            Ok(states) => (states, None),
            Err(error) => {
                tracing::warn!(%error, path = %path.display(), "agent lifecycle baseline restore failed; observation ingestion disabled");
                (HashMap::new(), Some(error.to_string()))
            }
        };
        Self {
            states: Mutex::new(states),
            path,
            restore_error,
            telemetry,
        }
    }

    /// Validate and atomically commit a lifecycle mutation or completed cycle.
    ///
    /// All lifecycle events are emitted after the new baseline is committed.
    /// Telemetry delivery is passive and best-effort: a Lens queue failure is
    /// logged, but never rolls back real agent state or makes a retry emit the
    /// same transition twice.
    pub async fn observe(
        &self,
        agent: &str,
        observation: AgentRuntimeObservation,
    ) -> Result<AgentObservationCommit, AgentObservationError> {
        if let Some(error) = &self.restore_error {
            return Err(AgentObservationError::Persistence(format!(
                "baseline restore failed; restart after repairing `{}`: {error}",
                self.path.display()
            )));
        }
        validate_observation(agent, &observation)?;

        let mut states = self.states.lock().await;
        let events = match states.get(agent) {
            None => {
                validate_initial(&observation)?;
                initial_events(agent, &observation)
            }
            Some(previous) => {
                let committed_sequence = previous.last_observation.sequence;
                if observation.sequence < committed_sequence {
                    return Err(AgentObservationError::Stale {
                        received: observation.sequence,
                        committed: committed_sequence,
                    });
                }
                if observation.sequence == committed_sequence {
                    if observation == previous.last_observation {
                        return Ok(AgentObservationCommit {
                            sequence: observation.sequence,
                            emitted_events: 0,
                            duplicate: true,
                        });
                    }
                    return Err(AgentObservationError::ConflictingRetry {
                        sequence: observation.sequence,
                    });
                }

                validate_transition(previous, &observation)?;
                transition_events(agent, previous, &observation)
            }
        };

        let mut slots = states
            .get(agent)
            .map_or_else(BTreeMap::new, |state| state.slots.clone());
        for update in &observation.slot_updates {
            slots.insert(update.slot.clone(), update.state.clone());
        }
        let next_state = AgentLifecycleState {
            last_observation: observation.clone(),
            slots,
        };
        let mut next_states = states.clone();
        next_states.insert(agent.to_string(), next_state);
        persist_states(&self.path, &next_states)?;
        *states = next_states;
        drop(states);

        let ancestry = [LensScope::Agent(agent.to_string())];
        for event in &events {
            for error in self.telemetry.emit_observable(event, &ancestry) {
                tracing::warn!(%error, agent, sequence = observation.sequence, "agent lifecycle telemetry delivery failed");
            }
        }

        Ok(AgentObservationCommit {
            sequence: observation.sequence,
            emitted_events: events.len(),
            duplicate: false,
        })
    }
}

fn validate_observation(
    agent: &str,
    observation: &AgentRuntimeObservation,
) -> Result<(), AgentObservationError> {
    validate_canonical_non_blank("agent", agent)?;
    if observation.sequence == 0 {
        return Err(AgentObservationError::Invalid(
            "agent observation sequence must be greater than zero".to_string(),
        ));
    }
    if observation.completed_tick.is_some_and(|tick| {
        !tick.prediction_error.is_finite() || !(0.0..=1.0).contains(&tick.prediction_error)
    }) {
        return Err(AgentObservationError::Invalid(
            "prediction_error must be finite and within [0, 1]".to_string(),
        ));
    }
    if !observation.vitality.is_finite() || !(0.0..=1.0).contains(&observation.vitality) {
        return Err(AgentObservationError::Invalid(
            "vitality must be finite and within [0, 1]".to_string(),
        ));
    }
    if phase_for_vitality(observation.vitality) != observation.phase {
        return Err(AgentObservationError::Invalid(format!(
            "phase `{}` does not match vitality {} (expected `{}`)",
            observation.phase.as_str(),
            observation.vitality,
            phase_for_vitality(observation.vitality).as_str()
        )));
    }

    let mut slots = HashSet::with_capacity(observation.slot_updates.len());
    for update in &observation.slot_updates {
        validate_canonical_non_blank("slot", &update.slot)?;
        if let ObservedSlotState::Blocked { reason } = &update.state {
            validate_canonical_non_blank("blocked slot reason", reason)?;
        }
        if !slots.insert(update.slot.as_str()) {
            return Err(AgentObservationError::Invalid(format!(
                "slot `{}` occurs more than once in one observation",
                update.slot
            )));
        }
    }
    Ok(())
}

fn validate_transition(
    previous: &AgentLifecycleState,
    observation: &AgentRuntimeObservation,
) -> Result<(), AgentObservationError> {
    let prior = &previous.last_observation;
    if observation.completed_tick.is_some()
        && prior.lifecycle_state != ObservedLifecycleState::Active
        && observation.lifecycle_state != ObservedLifecycleState::Active
    {
        return Err(AgentObservationError::Invalid(
            "a completed tick must begin or end in the active lifecycle state".to_string(),
        ));
    }
    if prior.regime != observation.regime && observation.completed_tick.is_none() {
        return Err(AgentObservationError::Invalid(
            "a regime change must be committed by a completed tick".to_string(),
        ));
    }
    if observation.vitality > prior.vitality {
        return Err(AgentObservationError::Invalid(format!(
            "vitality must not increase (committed {}, received {})",
            prior.vitality, observation.vitality
        )));
    }
    if phase_rank(observation.phase) < phase_rank(prior.phase) {
        return Err(AgentObservationError::Invalid(format!(
            "phase must not move backward from `{}` to `{}`",
            prior.phase.as_str(),
            observation.phase.as_str()
        )));
    }
    if prior.lifecycle_state != observation.lifecycle_state
        && !allowed_lifecycle_transition(prior.lifecycle_state, observation.lifecycle_state)
    {
        return Err(AgentObservationError::Invalid(format!(
            "invalid lifecycle transition from `{}` to `{}`",
            prior.lifecycle_state.as_str(),
            observation.lifecycle_state.as_str()
        )));
    }
    Ok(())
}

fn validate_initial(observation: &AgentRuntimeObservation) -> Result<(), AgentObservationError> {
    if observation.completed_tick.is_some()
        && observation.lifecycle_state != ObservedLifecycleState::Active
    {
        return Err(AgentObservationError::Invalid(
            "an initial completed tick requires the active lifecycle state".to_string(),
        ));
    }
    Ok(())
}

const fn phase_for_vitality(vitality: f64) -> ObservedVitalityPhase {
    if vitality > 0.7 {
        ObservedVitalityPhase::Thriving
    } else if vitality > 0.4 {
        ObservedVitalityPhase::Stable
    } else if vitality > 0.2 {
        ObservedVitalityPhase::Conservation
    } else if vitality > 0.05 {
        ObservedVitalityPhase::Declining
    } else {
        ObservedVitalityPhase::Terminal
    }
}

const fn phase_rank(phase: ObservedVitalityPhase) -> u8 {
    match phase {
        ObservedVitalityPhase::Thriving => 0,
        ObservedVitalityPhase::Stable => 1,
        ObservedVitalityPhase::Conservation => 2,
        ObservedVitalityPhase::Declining => 3,
        ObservedVitalityPhase::Terminal => 4,
    }
}

const fn allowed_lifecycle_transition(
    old: ObservedLifecycleState,
    new: ObservedLifecycleState,
) -> bool {
    matches!(
        (old, new),
        (
            ObservedLifecycleState::Provisioning,
            ObservedLifecycleState::Active
        ) | (
            ObservedLifecycleState::Active,
            ObservedLifecycleState::Dreaming | ObservedLifecycleState::Terminal
        ) | (
            ObservedLifecycleState::Dreaming,
            ObservedLifecycleState::Active | ObservedLifecycleState::Terminal
        )
    )
}

fn load_states(path: &Path) -> Result<HashMap<String, AgentLifecycleState>, AgentObservationError> {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|error| AgentObservationError::Persistence(error.to_string())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(HashMap::new()),
        Err(error) => Err(AgentObservationError::Persistence(error.to_string())),
    }
}

fn persist_states(
    path: &Path,
    states: &HashMap<String, AgentLifecycleState>,
) -> Result<(), AgentObservationError> {
    roko_fs::atomic_write_json(path, states)
        .map_err(|error| AgentObservationError::Persistence(error.to_string()))
}

fn validate_canonical_non_blank(label: &str, value: &str) -> Result<(), AgentObservationError> {
    if value.is_empty() || value.trim() != value {
        return Err(AgentObservationError::Invalid(format!(
            "{label} must be non-blank and must not have surrounding whitespace"
        )));
    }
    Ok(())
}

fn initial_events(agent: &str, observation: &AgentRuntimeObservation) -> Vec<ObservableEvent> {
    let mut events = changed_slot_events(agent, &BTreeMap::new(), &observation.slot_updates);
    if let Some(event) = tick_event(agent, observation) {
        events.push(event);
    }
    events
}

fn transition_events(
    agent: &str,
    previous: &AgentLifecycleState,
    observation: &AgentRuntimeObservation,
) -> Vec<ObservableEvent> {
    let prior = &previous.last_observation;
    let mut events = Vec::with_capacity(6 + observation.slot_updates.len());

    if prior.regime != observation.regime {
        events.push(ObservableEvent::AgentRegimeChange {
            agent: agent.to_string(),
            old: prior.regime.as_str().to_string(),
            new_regime: observation.regime.as_str().to_string(),
        });
    }
    if prior.mode != observation.mode {
        events.push(ObservableEvent::AgentModeChange {
            agent: agent.to_string(),
            old: prior.mode.as_str().to_string(),
            new_mode: observation.mode.as_str().to_string(),
        });
    }
    if prior.phase != observation.phase {
        events.push(ObservableEvent::AgentPhaseChange {
            agent: agent.to_string(),
            old: prior.phase.as_str().to_string(),
            new_phase: observation.phase.as_str().to_string(),
        });
    }
    if prior.lifecycle_state != observation.lifecycle_state {
        events.push(ObservableEvent::AgentStateTransition {
            agent: agent.to_string(),
            old: prior.lifecycle_state.as_str().to_string(),
            new_state: observation.lifecycle_state.as_str().to_string(),
        });
    }
    events.extend(changed_slot_events(
        agent,
        &previous.slots,
        &observation.slot_updates,
    ));
    if let Some(event) = tick_event(agent, observation) {
        events.push(event);
    }
    events
}

fn changed_slot_events(
    agent: &str,
    previous: &BTreeMap<String, ObservedSlotState>,
    updates: &[AgentSlotObservation],
) -> Vec<ObservableEvent> {
    let mut changed = updates
        .iter()
        .filter(|update| previous.get(&update.slot) != Some(&update.state))
        .collect::<Vec<_>>();
    changed.sort_unstable_by(|left, right| left.slot.cmp(&right.slot));
    changed
        .into_iter()
        .map(|update| ObservableEvent::AgentSlotUpdate {
            agent: agent.to_string(),
            slot: update.slot.clone(),
            state: update.state.event_value(),
        })
        .collect()
}

fn tick_event(agent: &str, observation: &AgentRuntimeObservation) -> Option<ObservableEvent> {
    let completed_tick = observation.completed_tick?;
    Some(ObservableEvent::AgentTick {
        agent: agent.to_string(),
        regime: observation.regime.as_str().to_string(),
        prediction_error: completed_tick.prediction_error,
        vitality: observation.vitality,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex as StdMutex};
    use std::time::Duration;

    use roko_core::{
        LensConfig, LensRegistry, LensScope, ObservableEvent, ObservableEventKind, Signal,
        TelemetryObserve,
    };
    use roko_runtime::{LensExecutor, LensQueueConfig};
    use tempfile::tempdir;

    use super::*;

    struct RecordingAgentLens {
        scope: LensScope,
        seen: Arc<StdMutex<Vec<ObservableEvent>>>,
    }

    #[async_trait::async_trait]
    impl TelemetryObserve for RecordingAgentLens {
        async fn observe(&self, event: &ObservableEvent) -> roko_core::Result<Vec<Signal>> {
            self.seen
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(event.clone());
            Ok(Vec::new())
        }

        fn observes(&self) -> &[ObservableEventKind] {
            const OBSERVES: &[ObservableEventKind] = &[ObservableEventKind::AgentLifecycle];
            OBSERVES
        }

        fn scope(&self) -> LensScope {
            self.scope.clone()
        }
    }

    fn observation(sequence: u64) -> AgentRuntimeObservation {
        AgentRuntimeObservation {
            sequence,
            regime: ObservedAgentRegime::Calm,
            vitality: 0.8,
            mode: ObservedAgentMode::Ephemeral,
            phase: ObservedVitalityPhase::Thriving,
            lifecycle_state: ObservedLifecycleState::Provisioning,
            completed_tick: None,
            slot_updates: Vec::new(),
        }
    }

    fn recording_runtime(
        hub: &roko_runtime::StateHub,
    ) -> roko_core::Result<(
        roko_runtime::QueuedLensExecutor,
        Arc<StdMutex<Vec<ObservableEvent>>>,
        Arc<StdMutex<Vec<ObservableEvent>>>,
    )> {
        let mut registry = LensRegistry::new();
        for (name, scope) in [
            ("agent-a-recorder", "agent:agent-a"),
            ("agent-b-recorder", "agent:agent-b"),
        ] {
            registry.register_with_observes(
                LensConfig {
                    name: name.to_string(),
                    block: format!("test:{name}"),
                    scope: scope.to_string(),
                    params: BTreeMap::new(),
                },
                vec![ObservableEventKind::AgentLifecycle],
            )?;
        }
        let seen_a = Arc::new(StdMutex::new(Vec::new()));
        let seen_b = Arc::new(StdMutex::new(Vec::new()));
        let mut executor = LensExecutor::new(registry)?.with_projection(hub.sender());
        executor.register(
            "agent-a-recorder",
            Arc::new(RecordingAgentLens {
                scope: LensScope::Agent("agent-a".to_string()),
                seen: Arc::clone(&seen_a),
            }),
        )?;
        executor.register(
            "agent-b-recorder",
            Arc::new(RecordingAgentLens {
                scope: LensScope::Agent("agent-b".to_string()),
                seen: Arc::clone(&seen_b),
            }),
        )?;
        let queue = executor.into_queued("agent-lifecycle-test", LensQueueConfig::default())?;
        Ok((queue, seen_a, seen_b))
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn production_store_emits_all_agent_lifecycle_deltas_in_exact_order_and_scope()
    -> roko_core::Result<()> {
        let dir = tempdir().expect("tempdir");
        let hub = roko_runtime::StateHub::default_capacity();
        let (queue, seen_a, seen_b) = recording_runtime(&hub)?;
        let store = AgentLifecycleStore::open(dir.path().join("lifecycle.json"), hub.sender());

        let baseline = store
            .observe("agent-a", observation(1))
            .await
            .expect("seed provisioning baseline");
        assert_eq!(baseline.emitted_events, 0);

        let mut changed = observation(2);
        changed.regime = ObservedAgentRegime::Normal;
        changed.vitality = 0.7;
        changed.mode = ObservedAgentMode::Persistent;
        changed.phase = ObservedVitalityPhase::Stable;
        changed.lifecycle_state = ObservedLifecycleState::Active;
        changed.completed_tick = Some(CompletedAgentTick {
            prediction_error: 0.25,
        });
        changed.slot_updates = vec![
            AgentSlotObservation {
                slot: "zeta".to_string(),
                state: ObservedSlotState::Active,
            },
            AgentSlotObservation {
                slot: "alpha".to_string(),
                state: ObservedSlotState::Blocked {
                    reason: "waiting".to_string(),
                },
            },
        ];
        let commit = store
            .observe("agent-a", changed.clone())
            .await
            .expect("commit real lifecycle deltas");
        assert_eq!(commit.emitted_events, 7);
        assert!(!commit.duplicate);

        assert!(queue.wait_idle(Duration::from_secs(2)).await);
        let events = seen_a
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        assert_eq!(events.len(), 7);
        assert!(matches!(
            &events[0],
            ObservableEvent::AgentRegimeChange { agent, old, new_regime }
                if agent == "agent-a" && old == "calm" && new_regime == "normal"
        ));
        assert!(matches!(
            &events[1],
            ObservableEvent::AgentModeChange { agent, old, new_mode }
                if agent == "agent-a" && old == "ephemeral" && new_mode == "persistent"
        ));
        assert!(matches!(
            &events[2],
            ObservableEvent::AgentPhaseChange { agent, old, new_phase }
                if agent == "agent-a" && old == "thriving" && new_phase == "stable"
        ));
        assert!(matches!(
            &events[3],
            ObservableEvent::AgentStateTransition { agent, old, new_state }
                if agent == "agent-a" && old == "provisioning" && new_state == "active"
        ));
        assert!(matches!(
            &events[4],
            ObservableEvent::AgentSlotUpdate { agent, slot, state }
                if agent == "agent-a" && slot == "alpha" && state == "blocked(waiting)"
        ));
        assert!(matches!(
            &events[5],
            ObservableEvent::AgentSlotUpdate { agent, slot, state }
                if agent == "agent-a" && slot == "zeta" && state == "active"
        ));
        assert!(matches!(
            &events[6],
            ObservableEvent::AgentTick { agent, regime, prediction_error, vitality }
                if agent == "agent-a" && regime == "normal"
                    && *prediction_error == 0.25 && *vitality == 0.7
        ));
        assert!(
            events
                .iter()
                .all(|event| { event.source_scope() == LensScope::Agent("agent-a".to_string()) })
        );
        assert!(
            seen_b
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .is_empty(),
            "a different Agent Lens must not receive agent-a observations"
        );

        let mut no_op = changed;
        no_op.sequence = 3;
        no_op.completed_tick = None;
        assert_eq!(
            store
                .observe("agent-a", no_op)
                .await
                .expect("same-state slot reports are no-ops")
                .emitted_events,
            0
        );
        assert!(queue.wait_idle(Duration::from_secs(2)).await);
        assert_eq!(
            seen_a
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .len(),
            7
        );
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn retries_noops_invalid_transitions_and_persistence_failures_emit_nothing() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("lifecycle.json");
        let hub = roko_runtime::StateHub::default_capacity();
        let (queue, seen, _) = recording_runtime(&hub).expect("recording runtime");
        let store = AgentLifecycleStore::open(&path, hub.sender());

        let mut active = observation(1);
        active.lifecycle_state = ObservedLifecycleState::Active;
        active.completed_tick = Some(CompletedAgentTick {
            prediction_error: 0.1,
        });
        assert_eq!(
            store
                .observe("agent-a", active.clone())
                .await
                .expect("initial active tick")
                .emitted_events,
            1
        );
        assert!(queue.wait_idle(Duration::from_secs(2)).await);
        seen.lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();

        let retry = store
            .observe("agent-a", active.clone())
            .await
            .expect("exact retry");
        assert!(retry.duplicate);
        assert_eq!(retry.emitted_events, 0);
        drop(store);

        let reopened = AgentLifecycleStore::open(&path, hub.sender());
        assert!(
            reopened
                .observe("agent-a", active.clone())
                .await
                .expect("restart retry")
                .duplicate
        );

        let mut later_tick = active.clone();
        later_tick.sequence = 2;
        assert_eq!(
            reopened
                .observe("agent-a", later_tick.clone())
                .await
                .expect("distinct real tick with identical values")
                .emitted_events,
            1
        );

        let mut transition_only = later_tick.clone();
        transition_only.sequence = 3;
        transition_only.mode = ObservedAgentMode::Reactive;
        transition_only.completed_tick = None;
        assert_eq!(
            reopened
                .observe("agent-a", transition_only.clone())
                .await
                .expect("mode transition without a cognitive tick")
                .emitted_events,
            1
        );

        let mut conflict = transition_only.clone();
        conflict.mode = ObservedAgentMode::Persistent;
        assert!(matches!(
            reopened.observe("agent-a", conflict).await,
            Err(AgentObservationError::ConflictingRetry { sequence: 3 })
        ));
        assert!(matches!(
            reopened.observe("agent-a", later_tick).await,
            Err(AgentObservationError::Stale { .. })
        ));

        let mut invalid_phase = transition_only.clone();
        invalid_phase.sequence = 4;
        invalid_phase.phase = ObservedVitalityPhase::Stable;
        assert!(matches!(
            reopened.observe("agent-a", invalid_phase).await,
            Err(AgentObservationError::Invalid(_))
        ));
        let mut corrected = transition_only.clone();
        corrected.sequence = 4;
        corrected.mode = ObservedAgentMode::Persistent;
        assert_eq!(
            reopened
                .observe("agent-a", corrected.clone())
                .await
                .expect("invalid request did not advance baseline")
                .emitted_events,
            1
        );

        let mut vitality_increase = corrected.clone();
        vitality_increase.sequence = 5;
        vitality_increase.vitality = 0.9;
        assert!(matches!(
            reopened.observe("agent-a", vitality_increase).await,
            Err(AgentObservationError::Invalid(_))
        ));
        let mut phase_forward = corrected.clone();
        phase_forward.sequence = 5;
        phase_forward.vitality = 0.4;
        phase_forward.phase = ObservedVitalityPhase::Conservation;
        assert_eq!(
            reopened
                .observe("agent-a", phase_forward.clone())
                .await
                .expect("legal vitality decline")
                .emitted_events,
            1
        );

        let mut illegal_state = phase_forward.clone();
        illegal_state.sequence = 6;
        illegal_state.lifecycle_state = ObservedLifecycleState::Provisioning;
        assert!(matches!(
            reopened.observe("agent-a", illegal_state).await,
            Err(AgentObservationError::Invalid(_))
        ));
        let mut dreaming = phase_forward.clone();
        dreaming.sequence = 6;
        dreaming.lifecycle_state = ObservedLifecycleState::Dreaming;
        assert_eq!(
            reopened
                .observe("agent-a", dreaming.clone())
                .await
                .expect("active to dreaming")
                .emitted_events,
            1
        );

        let mut impossible_terminal_tick = dreaming.clone();
        impossible_terminal_tick.sequence = 7;
        impossible_terminal_tick.lifecycle_state = ObservedLifecycleState::Terminal;
        impossible_terminal_tick.completed_tick = Some(CompletedAgentTick {
            prediction_error: 0.2,
        });
        assert!(matches!(
            reopened.observe("agent-a", impossible_terminal_tick).await,
            Err(AgentObservationError::Invalid(_))
        ));
        let mut terminal = dreaming;
        terminal.sequence = 7;
        terminal.lifecycle_state = ObservedLifecycleState::Terminal;
        assert_eq!(
            reopened
                .observe("agent-a", terminal)
                .await
                .expect("dreaming to terminal without fabricated tick")
                .emitted_events,
            1
        );

        assert!(queue.wait_idle(Duration::from_secs(2)).await);
        let events = seen
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        assert_eq!(events.len(), 6);
        assert!(matches!(events[0], ObservableEvent::AgentTick { .. }));
        assert!(matches!(events[1], ObservableEvent::AgentModeChange { .. }));
        assert!(matches!(events[2], ObservableEvent::AgentModeChange { .. }));
        assert!(matches!(
            events[3],
            ObservableEvent::AgentPhaseChange { .. }
        ));
        assert!(matches!(
            events[4],
            ObservableEvent::AgentStateTransition { .. }
        ));
        assert!(matches!(
            events[5],
            ObservableEvent::AgentStateTransition { .. }
        ));
    }

    #[tokio::test]
    async fn payload_and_durable_write_failures_do_not_advance_or_emit() {
        let dir = tempdir().expect("tempdir");
        let hub = roko_runtime::StateHub::default_capacity();
        let parent = dir.path().join("state-parent");
        std::fs::create_dir(&parent).expect("create state parent");
        let path = parent.join("lifecycle.json");
        let store = AgentLifecycleStore::open(&path, hub.sender());
        std::fs::remove_dir(&parent).expect("remove empty state parent");
        std::fs::write(&parent, b"blocks directory creation").expect("create parent blocker");

        let mut active = observation(1);
        active.lifecycle_state = ObservedLifecycleState::Active;
        active.completed_tick = Some(CompletedAgentTick {
            prediction_error: 0.2,
        });
        assert!(matches!(
            store.observe("agent-a", active.clone()).await,
            Err(AgentObservationError::Persistence(_))
        ));
        std::fs::remove_file(&parent).expect("remove parent blocker");
        std::fs::create_dir(&parent).expect("restore state parent");
        assert_eq!(
            store
                .observe("agent-a", active)
                .await
                .expect("same sequence succeeds after storage recovers")
                .emitted_events,
            1
        );

        let corrupt_path = dir.path().join("corrupt.json");
        std::fs::write(&corrupt_path, b"not json").expect("write corrupt baseline");
        let disabled = AgentLifecycleStore::open(&corrupt_path, hub.sender());
        assert!(matches!(
            disabled.observe("agent-b", observation(1)).await,
            Err(AgentObservationError::Persistence(_))
        ));
        assert_eq!(
            std::fs::read(&corrupt_path).expect("corrupt file preserved"),
            b"not json"
        );
    }

    #[test]
    fn canonical_payload_validation_covers_phase_boundaries_slots_and_tick_state() {
        assert_eq!(
            phase_for_vitality(0.700_001),
            ObservedVitalityPhase::Thriving
        );
        assert_eq!(phase_for_vitality(0.7), ObservedVitalityPhase::Stable);
        assert_eq!(phase_for_vitality(0.4), ObservedVitalityPhase::Conservation);
        assert_eq!(phase_for_vitality(0.2), ObservedVitalityPhase::Declining);
        assert_eq!(phase_for_vitality(0.05), ObservedVitalityPhase::Terminal);

        let mut duplicate_slots = observation(1);
        duplicate_slots.slot_updates = vec![
            AgentSlotObservation {
                slot: "worker".to_string(),
                state: ObservedSlotState::Active,
            },
            AgentSlotObservation {
                slot: "worker".to_string(),
                state: ObservedSlotState::Completed,
            },
        ];
        assert!(validate_observation("agent-a", &duplicate_slots).is_err());

        let mut blank_reason = observation(1);
        blank_reason.slot_updates = vec![AgentSlotObservation {
            slot: "worker".to_string(),
            state: ObservedSlotState::Blocked {
                reason: " ".to_string(),
            },
        }];
        assert!(validate_observation("agent-a", &blank_reason).is_err());

        let mut terminal_tick = observation(1);
        terminal_tick.vitality = 0.05;
        terminal_tick.phase = ObservedVitalityPhase::Terminal;
        terminal_tick.lifecycle_state = ObservedLifecycleState::Terminal;
        terminal_tick.completed_tick = Some(CompletedAgentTick {
            prediction_error: 0.1,
        });
        assert!(validate_initial(&terminal_tick).is_err());
    }
}
