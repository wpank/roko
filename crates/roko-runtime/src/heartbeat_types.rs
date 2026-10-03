//! Heartbeat data types the rest of the runtime uses.
//!
//! They are the tick topics and speeds, the environmental regime, the
//! behavioural and emotion labels, and the event-bus payloads
//! (`HeartbeatTick`, `CognitiveSignal`, `WakeupCondition`).
//!
//! The cognitive clock itself (`heartbeat`, `theta_consumer` and
//! `delta_consumer`) is parked behind the `cognitive-clock` feature (9222);
//! `heartbeat` re-exports everything here, so its paths keep working.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Bus topic emitted for fast reactive heartbeat ticks.
pub const HEARTBEAT_GAMMA_TICK: &str = "heartbeat.gamma.tick";

/// Bus topic emitted for medium reflective heartbeat ticks.
pub const HEARTBEAT_THETA_TICK: &str = "heartbeat.theta.tick";

/// Bus topic emitted for slow consolidation heartbeat ticks.
pub const HEARTBEAT_DELTA_TICK: &str = "heartbeat.delta.tick";

/// Cognitive speed handled by the heartbeat clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeartbeatSpeed {
    /// Reactive perception and action cadence.
    Gamma,
    /// Reflective planning and calibration cadence.
    Theta,
    /// Offline consolidation cadence.
    Delta,
}

/// Configurable cognitive timescale for a pure `AdaptiveClock`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CognitiveTimescale {
    /// Fast reactive loop, normally configured in the 100–500 ms range.
    Gamma,
    /// Reflective loop, normally configured in the 500 ms–16 s range.
    Theta,
    /// Consolidation loop, normally configured in the 60–600 s range.
    Delta,
}

impl From<HeartbeatSpeed> for CognitiveTimescale {
    fn from(speed: HeartbeatSpeed) -> Self {
        match speed {
            HeartbeatSpeed::Gamma => Self::Gamma,
            HeartbeatSpeed::Theta => Self::Theta,
            HeartbeatSpeed::Delta => Self::Delta,
        }
    }
}

impl HeartbeatSpeed {
    /// Return the canonical bus topic for this speed.
    pub const fn topic(self) -> &'static str {
        match self {
            Self::Gamma => HEARTBEAT_GAMMA_TICK,
            Self::Theta => HEARTBEAT_THETA_TICK,
            Self::Delta => HEARTBEAT_DELTA_TICK,
        }
    }
}

/// Re-exported from [`roko_primitives::tier::InferenceTier`].
pub use roko_primitives::tier::InferenceTier;

/// Environmental regime used by the adaptive clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Regime {
    /// Low prediction error and stable external conditions.
    Calm,
    /// Expected day-to-day variation.
    Normal,
    /// Elevated volatility or repeated anomalies.
    Volatile,
    /// Critical conditions requiring near-continuous attention.
    Crisis,
}

impl Regime {
    /// Convert a compact atomic representation into a regime.
    pub const fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Calm,
            1 => Self::Normal,
            2 => Self::Volatile,
            _ => Self::Crisis,
        }
    }
}

impl From<Regime> for u8 {
    fn from(value: Regime) -> Self {
        match value {
            Regime::Calm => 0,
            Regime::Normal => 1,
            Regime::Volatile => 2,
            Regime::Crisis => 3,
        }
    }
}

/// Re-export the canonical PAD vector from [`roko_primitives`].
///
/// Previously the heartbeat module defined a local `f32` variant; the
/// canonical `f64` definition now lives in `roko_primitives::pad` and is
/// shared across the entire workspace.  The `CorticalState` atomic storage
/// narrows to `f32` at the read/write boundary (see `pad()` / `set_pad()`).
pub use roko_primitives::PadVector;

/// Personality preset used to initialize `CorticalState` affect signals.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersonalityPreset {
    /// Cautious startup: lower dominance and slightly elevated arousal.
    Cautious,
    /// Balanced startup with neutral PAD.
    Balanced,
    /// Aggressive startup: higher arousal and dominance.
    Aggressive,
    /// Explicit PAD values supplied by configuration.
    Custom(PadVector),
}

impl PersonalityPreset {
    /// Return the initial PAD vector for this preset.
    pub const fn pad(self) -> PadVector {
        match self {
            Self::Cautious => PadVector::new(-0.1, 0.1, -0.2),
            Self::Balanced => PadVector::neutral(),
            Self::Aggressive => PadVector::new(0.1, 0.3, 0.2),
            Self::Custom(pad) => pad,
        }
    }
}

/// Behavioral state derived from PAD and recent outcomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BehavioralState {
    /// Balanced exploration and exploitation.
    Engaged,
    /// Repeated failures or low confidence require caution.
    Struggling,
    /// Success with low arousal may hide complacency.
    Coasting,
    /// Actively searching and tolerating uncertainty.
    Exploring,
    /// Deep-work state with high arousal and dominance.
    Focused,
    /// Low-arousal pre-consolidation state.
    Resting,
}

impl BehavioralState {
    /// Convert a compact atomic representation into a behavioral state.
    pub const fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Engaged,
            1 => Self::Struggling,
            2 => Self::Coasting,
            3 => Self::Exploring,
            4 => Self::Focused,
            _ => Self::Resting,
        }
    }
}

impl From<BehavioralState> for u8 {
    fn from(value: BehavioralState) -> Self {
        match value {
            BehavioralState::Engaged => 0,
            BehavioralState::Struggling => 1,
            BehavioralState::Coasting => 2,
            BehavioralState::Exploring => 3,
            BehavioralState::Focused => 4,
            BehavioralState::Resting => 5,
        }
    }
}

/// Primary emotion label stored in the shared perception surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlutchikLabel {
    /// Joy-like positive valence.
    Joy,
    /// Trust-like confidence signal.
    Trust,
    /// Fear-like threat signal.
    Fear,
    /// Surprise-like novelty signal.
    Surprise,
    /// Sadness-like negative valence.
    Sadness,
    /// Disgust-like rejection signal.
    Disgust,
    /// Anger-like blocked-goal signal.
    Anger,
    /// Anticipation-like opportunity signal.
    Anticipation,
}

impl PlutchikLabel {
    /// Convert a compact atomic representation into an emotion label.
    pub const fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Joy,
            1 => Self::Trust,
            2 => Self::Fear,
            3 => Self::Surprise,
            4 => Self::Sadness,
            5 => Self::Disgust,
            6 => Self::Anger,
            _ => Self::Anticipation,
        }
    }
}

impl From<PlutchikLabel> for u8 {
    fn from(value: PlutchikLabel) -> Self {
        match value {
            PlutchikLabel::Joy => 0,
            PlutchikLabel::Trust => 1,
            PlutchikLabel::Fear => 2,
            PlutchikLabel::Surprise => 3,
            PlutchikLabel::Sadness => 4,
            PlutchikLabel::Disgust => 5,
            PlutchikLabel::Anger => 6,
            PlutchikLabel::Anticipation => 7,
        }
    }
}

/// A heartbeat tick pulse published on the runtime bus.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeartbeatTick {
    /// Monotonic tick id for this policy instance.
    pub tick_id: u64,
    /// Cognitive speed of the tick.
    pub speed: HeartbeatSpeed,
    /// Canonical heartbeat topic.
    pub topic: String,
    /// UTC timestamp when the tick was emitted.
    pub emitted_at: DateTime<Utc>,
    /// Interval selected for this speed at emission time.
    pub interval_millis: u64,
    /// Current environmental regime.
    pub regime: Regime,
}

/// Inter-loop control signal produced by heartbeat policies and meta-cognition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CognitiveSignal {
    /// Stop all cognitive work as soon as possible.
    Shutdown,
    /// Pause lower-priority consolidation work.
    Pause,
    /// Resume paused work.
    Resume,
    /// Escalate to a stronger tier or request review.
    Escalate,
    /// Slow down and reduce thrashing.
    Cooldown,
    /// Reprioritize toward the provided target.
    Reprioritize(String),
    /// Inject a context note into the next deliberation.
    InjectContext(String),
    /// Seek novel information or alternatives.
    Explore,
}

impl CognitiveSignal {
    /// Return lower numeric values for higher-priority signals.
    pub const fn priority(&self) -> u8 {
        match self {
            Self::Shutdown => 1,
            Self::Pause => 2,
            Self::Escalate => 3,
            Self::Cooldown => 4,
            Self::Reprioritize(_) => 5,
            Self::InjectContext(_) => 6,
            Self::Explore => 7,
            Self::Resume => 8,
        }
    }
}

/// Condition that should emit an early gamma tick.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WakeupCondition {
    /// External intervention from a user or operator.
    UserIntervention,
    /// Internal intervention from a safety system.
    SafetyAlert,
    /// Coordination threat or opportunity signal.
    PheromoneAlert {
        /// Alert intensity in `[0.0, 1.0]`.
        intensity: f32,
    },
    /// Budget state changed enough to require attention.
    BudgetAlert,
    /// Scheduled event became due.
    ScheduledEvent {
        /// Event identifier supplied by the scheduler.
        event_id: String,
    },
}
