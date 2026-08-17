//! Arena definitions, attempt lifecycle, derived leaderboards, and prize escrow.
//!
//! This module is the deterministic local adapter for the arena contract surface.
//! It deliberately does not pretend to submit transactions: callers may persist a
//! registry snapshot locally and project its typed events onto a [`roko_core::Bus`].

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};

use roko_core::{Body, Bus, Kind, Pulse, Topic};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::phase2::u256;

static ARENA_SNAPSHOT_WRITE_NONCE: AtomicU64 = AtomicU64::new(0);
const SCORE_FIXED_POINT_SCALE: u128 = 1_000_000_000_000_000_000;

mod u256_string {
    use std::fmt;

    use serde::de::Visitor;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(value: &u128, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u128, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct U256Visitor;

        impl Visitor<'_> for U256Visitor {
            type Value = u128;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a decimal u128 string or unsigned integer")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                value.parse().map_err(E::custom)
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(u128::from(value))
            }

            fn visit_u128<E>(self, value: u128) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(value)
            }
        }

        deserializer.deserialize_any(U256Visitor)
    }
}

mod u256_pairs_string {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(values: &[(u128, u128)], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        values
            .iter()
            .map(|(left, right)| (left.to_string(), right.to_string()))
            .collect::<Vec<_>>()
            .serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<(u128, u128)>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let values = Vec::<(String, String)>::deserialize(deserializer)?;
        values
            .into_iter()
            .map(|(left, right)| {
                let left = left.parse().map_err(serde::de::Error::custom)?;
                let right = right.parse().map_err(serde::de::Error::custom)?;
                Ok((left, right))
            })
            .collect()
    }
}

/// Arena lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArenaState {
    /// Created, immutable scoring declaration recorded, not accepting attempts.
    Draft,
    /// Accepting attempts.
    Active,
    /// Temporarily not accepting attempts.
    Paused,
    /// Terminal state.
    Concluded,
}

/// Arena's measurement domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArenaCategory {
    /// Software implementation and maintenance.
    Coding,
    /// Trading performance.
    Trading,
    /// Forecast calibration.
    Prediction,
    /// Adversarial or strategic games.
    Games,
    /// Persuasive communication.
    Persuasion,
    /// Negotiation performance.
    Negotiation,
    /// Constraint optimization.
    Optimization,
    /// Research and synthesis.
    Research,
    /// An extension-defined category.
    UserCreated,
}

impl ArenaCategory {
    /// Map an arena category onto one of the reputation registry's seven domains.
    #[must_use]
    pub const fn to_reputation_domain(self) -> &'static str {
        match self {
            Self::Coding => "coding",
            Self::Trading => "chain",
            Self::Prediction | Self::Research => "research",
            Self::Games | Self::Persuasion | Self::Negotiation => "strategy",
            Self::Optimization => "operations",
            Self::UserCreated => "knowledge",
        }
    }
}

/// How tasks enter an arena.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskSource {
    /// A fixed, externally identified dataset.
    Static,
    /// A deterministic registered generator.
    Procedural,
    /// Curated user contributions.
    UserContributed,
    /// Tasks produced from prior failures.
    Adversarial,
}

/// A declared binary scoring criterion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BinaryCriterion {
    /// All declared gate verdicts must pass.
    AllGatesPass,
    /// An externally executed test suite must pass.
    TestSuitePass,
    /// A registered external oracle supplies the verdict.
    OracleVerdict,
}

/// A declared continuous metric.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContinuousMetric {
    /// Risk-adjusted trading return.
    SharpeRatio,
    /// Continuous ranked probability score.
    #[allow(clippy::upper_case_acronyms)]
    CRPS,
    /// Execution latency.
    Latency,
    /// Output quality per consumed token.
    TokenEfficiency,
    /// Registered extension metric.
    Custom(String),
}

/// Normalization applied by a continuous scoring Cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Normalization {
    /// Use the metric as-is.
    Identity,
    /// Scale by declared or observed minimum and maximum.
    MinMax,
    /// Standardize by population mean and deviation.
    ZScore,
    /// Use population percentile.
    Percentile,
}

/// One declarative component in a composite score.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoringComponent {
    /// Stable dimension name.
    pub name: String,
    /// Scoring function for the dimension.
    pub function: ScoringFunction,
    /// Importance hint for Pareto presentation; never used as a weighted sum.
    pub weight: f64,
    /// How raw values are normalized before Pareto comparison.
    pub normalization: Normalization,
}

/// Scoring declaration fixed when an arena is registered.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScoringFunction {
    /// Binary external verification.
    Binary(BinaryCriterion),
    /// A continuous externally measured metric.
    Continuous(ContinuousMetric),
    /// Conjunctive hard criteria plus Pareto soft dimensions.
    Composite(Vec<ScoringComponent>),
}

/// How completed attempts are aggregated for each agent.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AggregationRule {
    /// Average an agent's best N scores.
    BestOf(usize),
    /// Average the most recent N scores.
    AverageLastN(usize),
    /// Exponentially weighted moving average, oldest to newest.
    #[allow(clippy::upper_case_acronyms)]
    EWMA(f64),
    /// Median score.
    Median,
}

/// Externally verifiable ground truth. There is intentionally no LLM/self option.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroundTruthSource {
    /// Executable test suite.
    TestSuite,
    /// Chain state observed at a declared block.
    ChainState,
    /// Independent human review.
    HumanReview,
    /// Versioned benchmark dataset.
    BenchmarkDataset(String),
    /// Registered external oracle.
    ExternalOracle(String),
}

/// An arena's immutable measurement declaration and mutable lifecycle state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Arena {
    /// Stable arena identifier.
    pub id: [u8; 32],
    /// Human-readable name.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Measurement domain.
    pub category: ArenaCategory,
    /// Lifecycle state.
    pub state: ArenaState,
    /// Task source declaration.
    pub task_source: TaskSource,
    /// Immutable scoring declaration.
    pub scoring: ScoringFunction,
    /// Leaderboard aggregation rule.
    pub aggregation: AggregationRule,
    /// Reputation impact multiplier.
    #[serde(default = "default_arena_weight")]
    pub weight: f64,
    /// Creator's identity-registry identifier.
    #[serde(with = "u256_string")]
    pub creator_identity_id: u256,
    /// Authenticated service principal that registered the arena.
    ///
    /// Empty only for snapshots produced through the lower-level local API;
    /// the live HTTP service always binds this field to its auth context.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub creator_principal: String,
    /// Prize pool mirrored by local escrow.
    #[serde(with = "u256_string")]
    pub prize_pool_usdc: u256,
    /// Maximum attempts per agent; zero means unlimited.
    pub max_attempts_per_agent: u32,
    /// Minimum block distance between attempts.
    pub cooldown_blocks: u64,
    /// Deadline block; zero means no deadline.
    pub deadline_block: u64,
    /// Declared independent source of truth.
    pub ground_truth: GroundTruthSource,
}

/// Attempt lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptState {
    /// Accepted by the registry, waiting to run.
    Queued,
    /// Agent execution is in progress.
    Running,
    /// Output was submitted to independent verification.
    Evaluating,
    /// Independently graded and finalized.
    Completed,
    /// Execution or verification failed.
    Failed,
    /// Cancelled before completion.
    Cancelled,
    /// Removed for a rule violation.
    Disqualified,
}

/// One agent's attempt in an arena.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attempt {
    /// Stable attempt identifier.
    pub id: [u8; 32],
    /// Parent arena.
    pub arena_id: [u8; 32],
    /// Participating identity.
    #[serde(with = "u256_string")]
    pub agent_identity_id: u256,
    /// Authenticated service principal that started the attempt.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub participant_principal: String,
    /// Lifecycle state.
    pub state: AttemptState,
    /// Hash of the assigned task, once selected.
    pub task_hash: Option<[u8; 32]>,
    /// Hash of the submitted output.
    pub output_hash: Option<[u8; 32]>,
    /// Individual gate pass/fail results.
    pub gate_verdicts: Vec<bool>,
    /// Independently supplied score.
    pub score: Option<f64>,
    /// External evidence committed with terminal settlement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scoring_evidence: Option<ScoringEvidence>,
    /// External failure explanation for unsuccessful settlement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
    /// Block at which the attempt entered the registry.
    pub started_at_block: u64,
    /// Block at which it reached a terminal evaluated state.
    pub completed_at_block: Option<u64>,
}

/// Authenticated, content-addressed evidence supplied by an external scorer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoringEvidence {
    /// Ground-truth source used to produce the evidence.
    pub source: GroundTruthSource,
    /// Independent scorer's identity-registry identifier.
    #[serde(with = "u256_string")]
    pub scorer_identity_id: u256,
    /// Authenticated service principal that committed the evidence.
    pub scorer_principal: String,
    /// Hash of the immutable test report, oracle response, review, or chain proof.
    pub evidence_hash: [u8; 32],
    /// Submitted output hash that the external evidence evaluates.
    pub subject_output_hash: [u8; 32],
    /// Server-observed block at which the evidence was committed.
    pub observed_at_block: u64,
}

/// Atomic terminal result committed together with its scoring evidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum AttemptSettlement {
    /// Independent evaluation completed successfully.
    Completed {
        /// Normalized score.
        score: f64,
        /// Individual external gate results.
        gate_verdicts: Vec<bool>,
    },
    /// Execution or independent evaluation failed.
    Failed {
        /// Non-empty external failure reason.
        reason: String,
    },
}

/// One derived leaderboard row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    /// Agent identity.
    #[serde(with = "u256_string")]
    pub agent_identity_id: u256,
    /// One-indexed rank.
    pub rank: u32,
    /// Score derived through the arena's aggregation rule.
    pub aggregate_score: f64,
    /// Number of completed scored attempts.
    pub attempt_count: u32,
    /// Best individual score.
    pub best_score: f64,
    /// Latest completed attempt block.
    pub last_attempt_block: u64,
}

/// Derived, never persisted separately from attempts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Leaderboard {
    /// Arena being ranked.
    pub arena_id: [u8; 32],
    /// Entries sorted by score descending, then identity ascending.
    pub entries: Vec<LeaderboardEntry>,
    /// Registry block used for the projection.
    pub computed_at_block: u64,
}

/// Condition governing release of an arena prize pool.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseCondition {
    /// Split equally across the top N agents.
    TopN(usize),
    /// Split equally across all agents meeting a score threshold.
    AllAboveThreshold(f64),
    /// Split in proportion to non-negative aggregate score.
    ProportionalToScore,
}

/// Locally tracked arena prize escrow.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BountyEscrow {
    /// Arena funded by the escrow.
    pub arena_id: [u8; 32],
    /// Depositor identity.
    #[serde(with = "u256_string")]
    pub depositor_identity_id: u256,
    /// Amount locked in integer USDC base units.
    #[serde(with = "u256_string")]
    pub amount_usdc: u256,
    /// Whether the balance has been distributed or refunded.
    pub released: bool,
    /// Deterministic release rule declared at deposit time.
    pub release_condition: ReleaseCondition,
}

/// The seven arena-to-learning data flow stages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlywheelStage {
    /// Capture the attempt trace.
    Trace,
    /// Independently grade the trace.
    Grade,
    /// Mine pairwise preferences.
    PreferenceMine,
    /// Cluster failed attempts.
    FailureCluster,
    /// Generate targeted curriculum.
    CurriculumGen,
    /// Extract falsifiable patterns.
    PatternExtract,
    /// Bootstrap new preferences.
    PreferenceBootstrap,
}

/// Output summary for one flywheel stage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlywheelResult {
    /// Completed stage.
    pub stage: FlywheelStage,
    /// Input records consumed.
    pub input_count: usize,
    /// Output records produced.
    pub output_count: usize,
    /// Durable artifact identifiers.
    pub artifacts: Vec<String>,
    /// Completion timestamp supplied by the learning runtime.
    pub completed_at: Option<u64>,
}

/// Current execution state of the external learning pipeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlywheelStatus {
    /// No execution in progress.
    Idle,
    /// The named stage is running.
    Running(FlywheelStage),
    /// All seven stages completed.
    Completed,
    /// External execution failed.
    Failed(String),
}

/// Arena-specific view of the external seven-step learning pipeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlywheelPipeline {
    /// Arena whose attempts feed this pipeline.
    pub arena_id: [u8; 32],
    /// Completed stage results.
    pub results: Vec<FlywheelResult>,
    /// Current status.
    pub status: FlywheelStatus,
}

/// Trace handed from arena execution to `roko-learn`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttemptTrace {
    /// Source attempt.
    pub attempt_id: [u8; 32],
    /// Optional durable episode identifier.
    pub episode_id: Option<String>,
    /// Independent gate verdicts.
    pub gate_verdicts: Vec<bool>,
    /// Named Pareto scoring dimensions.
    pub scoring_dimensions: Vec<(String, f64)>,
    /// Optional externally computed HDC fingerprint.
    pub hdc_fingerprint: Option<Vec<f64>>,
}

/// Reputation update produced by a completed attempt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArenaReputationEffect {
    /// Agent receiving the effect.
    #[serde(with = "u256_string")]
    pub agent_identity_id: u256,
    /// Reputation registry domain.
    pub domain: String,
    /// Signed score delta.
    pub delta: f64,
    /// Source arena.
    pub arena_id: [u8; 32],
    /// Source attempt.
    pub attempt_id: [u8; 32],
}

/// Typed arena events suitable for durable logs and Bus projection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum ArenaEvent {
    /// Arena declaration registered.
    ArenaCreated {
        /// Arena identifier.
        arena_id: [u8; 32],
        /// Declared name.
        name: String,
        /// Declared category.
        category: ArenaCategory,
    },
    /// Arena lifecycle changed.
    ArenaStateChanged {
        /// Arena identifier.
        arena_id: [u8; 32],
        /// Previous state.
        old_state: ArenaState,
        /// New state.
        new_state: ArenaState,
    },
    /// Attempt entered the registry.
    AttemptSubmitted {
        /// Arena identifier.
        arena_id: [u8; 32],
        /// Attempt identifier.
        attempt_id: [u8; 32],
        /// Agent identity.
        #[serde(with = "u256_string")]
        agent_identity_id: u256,
    },
    /// Attempt was completed by an independent verifier.
    AttemptCompleted {
        /// Arena identifier.
        arena_id: [u8; 32],
        /// Attempt identifier.
        attempt_id: [u8; 32],
        /// Final score.
        score: f64,
        /// Hash of the external scoring evidence.
        evidence_hash: [u8; 32],
    },
    /// Attempt failed evaluation or execution.
    AttemptFailed {
        /// Arena identifier.
        arena_id: [u8; 32],
        /// Attempt identifier.
        attempt_id: [u8; 32],
        /// Hash of the external failure evidence when settled by the service.
        #[serde(default)]
        evidence_hash: [u8; 32],
        /// External failure explanation.
        #[serde(default, skip_serializing_if = "String::is_empty")]
        reason: String,
    },
    /// Prize pool was locked locally pending a chain adapter.
    PrizeDeposited {
        /// Arena identifier.
        arena_id: [u8; 32],
        /// Deposited amount.
        #[serde(with = "u256_string")]
        amount_usdc: u256,
    },
    /// Prize pool was released according to its declaration.
    PrizeDistributed {
        /// Arena identifier.
        arena_id: [u8; 32],
        /// Deterministic payouts.
        #[serde(with = "u256_pairs_string")]
        payouts: Vec<(u256, u256)>,
    },
    /// Unreleased prize pool was refunded.
    PrizeRefunded {
        /// Arena identifier.
        arena_id: [u8; 32],
        /// Refunded amount.
        #[serde(with = "u256_string")]
        amount_usdc: u256,
    },
}

impl ArenaEvent {
    /// Stable Bus topic for this event.
    #[must_use]
    pub const fn topic(&self) -> &'static str {
        match self {
            Self::ArenaCreated { .. } => "arena.created",
            Self::ArenaStateChanged { .. } => "arena.state_changed",
            Self::AttemptSubmitted { .. } => "arena.attempt_submitted",
            Self::AttemptCompleted { .. } => "arena.attempt_completed",
            Self::AttemptFailed { .. } => "arena.attempt_failed",
            Self::PrizeDeposited { .. } => "arena.prize_deposited",
            Self::PrizeDistributed { .. } => "arena.prize_distributed",
            Self::PrizeRefunded { .. } => "arena.prize_refunded",
        }
    }

    /// Convert the event to a Pulse. The Bus assigns the final sequence number.
    ///
    /// # Errors
    ///
    /// Returns an error only if event serialization unexpectedly fails.
    pub fn to_pulse(&self) -> Result<Pulse, ArenaError> {
        let body = serde_json::to_value(self).map_err(|error| ArenaError::Persistence {
            message: error.to_string(),
        })?;
        Ok(Pulse::new(
            0,
            Topic::new(self.topic()),
            Kind::Custom("roko.arena.event".to_string()),
            Body::Json(body),
        ))
    }
}

/// Arena registry validation and lifecycle failures.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum ArenaError {
    /// Arena or attempt was not found.
    #[error("arena or attempt not found")]
    NotFound,
    /// Arena identifier is already registered.
    #[error("arena already exists")]
    DuplicateArena,
    /// Requested transition is invalid.
    #[error("invalid state transition")]
    InvalidState,
    /// Attempt was made before the cooldown elapsed.
    #[error("attempt cooldown remains active")]
    CooldownActive,
    /// Agent reached this arena's attempt limit.
    #[error("maximum attempts reached")]
    MaxAttemptsReached,
    /// Arena is not accepting attempts.
    #[error("arena is not active")]
    ArenaNotActive,
    /// Arena's declared deadline has passed.
    #[error("arena deadline has passed")]
    DeadlinePassed,
    /// Arena or escrow declaration failed validation.
    #[error("invalid arena declaration: {message}")]
    InvalidDeclaration {
        /// Validation detail.
        message: String,
    },
    /// Final scores are normalized before they enter ranking and reputation.
    #[error("score must be finite and within 0..=1")]
    InvalidScore,
    /// Human review needs an explicit independent evaluator.
    #[error("an explicit evaluator identity is required")]
    EvaluatorIdentityRequired,
    /// An agent may not grade its own attempt.
    #[error("an agent may not grade its own attempt")]
    SelfGrading,
    /// Authenticated principal is not permitted to perform this operation.
    #[error("authenticated principal is not authorized for this arena operation")]
    Unauthorized,
    /// External scoring evidence is missing or inconsistent.
    #[error("invalid external scoring evidence: {message}")]
    InvalidEvidence {
        /// Evidence validation detail.
        message: String,
    },
    /// No escrow exists for the arena.
    #[error("prize escrow was not found")]
    EscrowNotFound,
    /// Prize escrow already exists.
    #[error("prize escrow already exists")]
    DuplicateEscrow,
    /// A declared prize pool must be locked before an arena is activated.
    #[error("the declared prize pool has not been deposited into escrow")]
    PrizeEscrowRequired,
    /// Escrow balance has already been released.
    #[error("prize escrow was already released")]
    EscrowReleased,
    /// No leaderboard entries satisfy the release condition.
    #[error("no eligible prize recipients")]
    NoEligibleWinners,
    /// Local persistence failed.
    #[error("arena persistence failed: {message}")]
    Persistence {
        /// Filesystem or serialization detail.
        message: String,
    },
    /// Bus projection failed.
    #[error("arena event publication failed: {message}")]
    EventPublication {
        /// Bus error detail.
        message: String,
    },
}

/// In-memory state machine with an explicit durable snapshot adapter.
#[derive(Debug, Clone, Default)]
pub struct ArenaRegistry {
    arenas: HashMap<[u8; 32], Arena>,
    attempts: HashMap<[u8; 32], Vec<Attempt>>,
    escrow: HashMap<[u8; 32], BountyEscrow>,
    current_block: u64,
    events: Vec<ArenaEvent>,
    projected_event_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ArenaSnapshot {
    schema_version: u32,
    arenas: Vec<Arena>,
    attempts: Vec<AttemptBucket>,
    escrow: Vec<BountyEscrow>,
    current_block: u64,
    events: Vec<ArenaEvent>,
    projected_event_count: usize,
    content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AttemptBucket {
    arena_id: [u8; 32],
    attempts: Vec<Attempt>,
}

impl ArenaRegistry {
    /// Create an empty local registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Advance the block used for lifecycle checks and derived timestamps.
    ///
    /// Stale observations are ignored so a caller cannot rewind deadlines or
    /// cooldown accounting through this monotonic state-machine boundary.
    pub fn set_block(&mut self, block: u64) {
        self.current_block = self.current_block.max(block);
    }

    /// Registry block used for the latest operation.
    #[must_use]
    pub const fn current_block(&self) -> u64 {
        self.current_block
    }

    /// Number of registered arenas.
    #[must_use]
    pub fn arena_count(&self) -> usize {
        self.arenas.len()
    }

    /// Read an arena by identifier.
    #[must_use]
    pub fn get_arena(&self, id: &[u8; 32]) -> Option<&Arena> {
        self.arenas.get(id)
    }

    /// List all arenas in deterministic identifier order.
    #[must_use]
    pub fn list_arenas(&self) -> Vec<&Arena> {
        let mut arenas = self.arenas.values().collect::<Vec<_>>();
        arenas.sort_by_key(|arena| arena.id);
        arenas
    }

    /// Register a validated arena. Its initial state is always Draft.
    ///
    /// # Errors
    ///
    /// Rejects duplicate identifiers and invalid declarative scoring fields.
    pub fn create_arena(&mut self, mut arena: Arena) -> Result<(), ArenaError> {
        if self.arenas.contains_key(&arena.id) {
            return Err(ArenaError::DuplicateArena);
        }
        validate_arena(&arena)?;
        arena.state = ArenaState::Draft;
        let event = ArenaEvent::ArenaCreated {
            arena_id: arena.id,
            name: arena.name.clone(),
            category: arena.category,
        };
        self.attempts.entry(arena.id).or_default();
        self.arenas.insert(arena.id, arena);
        self.events.push(event);
        Ok(())
    }

    /// Activate a Draft or resume a Paused arena.
    ///
    /// # Errors
    ///
    /// Returns [`ArenaError::InvalidState`] for any other transition.
    pub fn activate_arena(&mut self, id: &[u8; 32]) -> Result<(), ArenaError> {
        let arena = self.arenas.get(id).ok_or(ArenaError::NotFound)?;
        if arena.prize_pool_usdc != 0
            && !self
                .escrow
                .get(id)
                .is_some_and(|entry| !entry.released && entry.amount_usdc == arena.prize_pool_usdc)
        {
            return Err(ArenaError::PrizeEscrowRequired);
        }
        self.transition_arena(
            id,
            &[ArenaState::Draft, ArenaState::Paused],
            ArenaState::Active,
        )
    }

    /// Pause an Active arena.
    ///
    /// # Errors
    ///
    /// Returns [`ArenaError::InvalidState`] unless the arena is Active.
    pub fn pause_arena(&mut self, id: &[u8; 32]) -> Result<(), ArenaError> {
        self.transition_arena(id, &[ArenaState::Active], ArenaState::Paused)
    }

    /// Permanently conclude an Active or Paused arena.
    ///
    /// # Errors
    ///
    /// Returns [`ArenaError::InvalidState`] for Draft or Concluded arenas.
    pub fn conclude_arena(&mut self, id: &[u8; 32]) -> Result<(), ArenaError> {
        self.transition_arena(
            id,
            &[ArenaState::Active, ArenaState::Paused],
            ArenaState::Concluded,
        )
    }

    fn transition_arena(
        &mut self,
        id: &[u8; 32],
        allowed: &[ArenaState],
        next: ArenaState,
    ) -> Result<(), ArenaError> {
        let arena = self.arenas.get_mut(id).ok_or(ArenaError::NotFound)?;
        if !allowed.contains(&arena.state) {
            return Err(ArenaError::InvalidState);
        }
        let old_state = arena.state;
        arena.state = next;
        self.events.push(ArenaEvent::ArenaStateChanged {
            arena_id: *id,
            old_state,
            new_state: next,
        });
        Ok(())
    }

    /// Queue an attempt after enforcing state, deadline, count, and cooldown.
    ///
    /// # Errors
    ///
    /// Returns a lifecycle validation error when the attempt cannot begin.
    pub fn queue_attempt(
        &mut self,
        arena_id: &[u8; 32],
        agent_identity_id: u256,
    ) -> Result<Attempt, ArenaError> {
        self.queue_attempt_inner(arena_id, agent_identity_id, String::new())
    }

    /// Queue an attempt and bind it to an authenticated service principal.
    ///
    /// # Errors
    ///
    /// Returns a lifecycle validation error or rejects a blank principal.
    pub fn queue_attempt_for_principal(
        &mut self,
        arena_id: &[u8; 32],
        agent_identity_id: u256,
        participant_principal: String,
    ) -> Result<Attempt, ArenaError> {
        if participant_principal.trim().is_empty() {
            return Err(invalid_evidence(
                "attempt participant principal must not be blank",
            ));
        }
        self.queue_attempt_inner(arena_id, agent_identity_id, participant_principal)
    }

    fn queue_attempt_inner(
        &mut self,
        arena_id: &[u8; 32],
        agent_identity_id: u256,
        participant_principal: String,
    ) -> Result<Attempt, ArenaError> {
        self.validate_new_attempt(arena_id, agent_identity_id)?;
        let ordinal = self.attempts.get(arena_id).map_or(0_u64, |attempts| {
            u64::try_from(attempts.len()).unwrap_or(u64::MAX)
        });
        let id = attempt_id(arena_id, agent_identity_id, self.current_block, ordinal);
        let attempt = Attempt {
            id,
            arena_id: *arena_id,
            agent_identity_id,
            participant_principal,
            state: AttemptState::Queued,
            task_hash: None,
            output_hash: None,
            gate_verdicts: Vec::new(),
            score: None,
            scoring_evidence: None,
            failure_reason: None,
            started_at_block: self.current_block,
            completed_at_block: None,
        };
        self.attempts
            .entry(*arena_id)
            .or_default()
            .push(attempt.clone());
        self.events.push(ArenaEvent::AttemptSubmitted {
            arena_id: *arena_id,
            attempt_id: id,
            agent_identity_id,
        });
        Ok(attempt)
    }

    /// Transition a queued attempt to Running and attach its task hash.
    ///
    /// # Errors
    ///
    /// Returns NotFound or InvalidState.
    pub fn start_queued_attempt(
        &mut self,
        attempt_id: &[u8; 32],
        task_hash: Option<[u8; 32]>,
    ) -> Result<Attempt, ArenaError> {
        let attempt = self.find_attempt_mut(attempt_id)?;
        if attempt.state != AttemptState::Queued {
            return Err(ArenaError::InvalidState);
        }
        attempt.task_hash = task_hash;
        attempt.state = AttemptState::Running;
        Ok(attempt.clone())
    }

    /// Convenience operation that queues and immediately starts an attempt.
    ///
    /// # Errors
    ///
    /// Returns a lifecycle validation error when the attempt cannot begin.
    pub fn start_attempt(
        &mut self,
        arena_id: &[u8; 32],
        agent_identity_id: u256,
    ) -> Result<Attempt, ArenaError> {
        let queued = self.queue_attempt(arena_id, agent_identity_id)?;
        self.start_queued_attempt(&queued.id, None)
    }

    /// Queue and start an attempt for an authenticated service principal.
    ///
    /// # Errors
    ///
    /// Returns a lifecycle validation error when the attempt cannot begin.
    pub fn start_attempt_for_principal(
        &mut self,
        arena_id: &[u8; 32],
        agent_identity_id: u256,
        participant_principal: String,
        task_hash: Option<[u8; 32]>,
    ) -> Result<Attempt, ArenaError> {
        let queued =
            self.queue_attempt_for_principal(arena_id, agent_identity_id, participant_principal)?;
        self.start_queued_attempt(&queued.id, task_hash)
    }

    fn validate_new_attempt(
        &self,
        arena_id: &[u8; 32],
        agent_identity_id: u256,
    ) -> Result<(), ArenaError> {
        let arena = self.arenas.get(arena_id).ok_or(ArenaError::NotFound)?;
        if arena.state != ArenaState::Active {
            return Err(ArenaError::ArenaNotActive);
        }
        if arena.deadline_block != 0 && self.current_block > arena.deadline_block {
            return Err(ArenaError::DeadlinePassed);
        }
        let attempts = self.attempts.get(arena_id).map_or(&[][..], Vec::as_slice);
        let own = attempts
            .iter()
            .filter(|attempt| attempt.agent_identity_id == agent_identity_id)
            .collect::<Vec<_>>();
        if arena.max_attempts_per_agent != 0 && own.len() >= arena.max_attempts_per_agent as usize {
            return Err(ArenaError::MaxAttemptsReached);
        }
        if let Some(last) = own.iter().max_by_key(|attempt| attempt.started_at_block) {
            let Some(next_block) = last.started_at_block.checked_add(arena.cooldown_blocks) else {
                return Err(ArenaError::CooldownActive);
            };
            if self.current_block < next_block {
                return Err(ArenaError::CooldownActive);
            }
        }
        Ok(())
    }

    /// Submit output for independent evaluation.
    ///
    /// # Errors
    ///
    /// Returns NotFound or InvalidState.
    pub fn submit_attempt(&mut self, attempt_id: &[u8; 32]) -> Result<(), ArenaError> {
        self.submit_attempt_with_output(attempt_id, None)
    }

    /// Submit a content-addressed output for independent evaluation.
    ///
    /// # Errors
    ///
    /// Returns NotFound or InvalidState.
    pub fn submit_attempt_with_output(
        &mut self,
        attempt_id: &[u8; 32],
        output_hash: Option<[u8; 32]>,
    ) -> Result<(), ArenaError> {
        let attempt = self.find_attempt_mut(attempt_id)?;
        if attempt.state != AttemptState::Running {
            return Err(ArenaError::InvalidState);
        }
        attempt.output_hash = output_hash;
        attempt.state = AttemptState::Evaluating;
        Ok(())
    }

    /// Complete an automatically verifiable attempt.
    ///
    /// Human-review arenas must use [`Self::complete_attempt_by`] so that the
    /// registry can prove the evaluator was not the participating agent.
    ///
    /// # Errors
    ///
    /// Returns a state, score, or evaluator validation error.
    pub fn complete_attempt(
        &mut self,
        attempt_id: &[u8; 32],
        score: f64,
        gate_verdicts: Vec<bool>,
    ) -> Result<(), ArenaError> {
        let arena_id = self.find_attempt(attempt_id)?.arena_id;
        let arena = self.arenas.get(&arena_id).ok_or(ArenaError::NotFound)?;
        if arena.ground_truth == GroundTruthSource::HumanReview {
            return Err(ArenaError::EvaluatorIdentityRequired);
        }
        self.complete_attempt_inner(attempt_id, score, gate_verdicts)
    }

    /// Complete a human-reviewed attempt with an explicit independent identity.
    ///
    /// # Errors
    ///
    /// Rejects self-grading and all ordinary completion validation failures.
    pub fn complete_attempt_by(
        &mut self,
        attempt_id: &[u8; 32],
        evaluator_identity_id: u256,
        score: f64,
        gate_verdicts: Vec<bool>,
    ) -> Result<(), ArenaError> {
        let attempt = self.find_attempt(attempt_id)?;
        if evaluator_identity_id == attempt.agent_identity_id {
            return Err(ArenaError::SelfGrading);
        }
        self.complete_attempt_inner(attempt_id, score, gate_verdicts)
    }

    fn complete_attempt_inner(
        &mut self,
        attempt_id: &[u8; 32],
        score: f64,
        gate_verdicts: Vec<bool>,
    ) -> Result<(), ArenaError> {
        if !score.is_finite() || !(0.0..=1.0).contains(&score) {
            return Err(ArenaError::InvalidScore);
        }
        let current_block = self.current_block;
        let attempt = self.find_attempt_mut(attempt_id)?;
        if attempt.state != AttemptState::Evaluating {
            return Err(ArenaError::InvalidState);
        }
        attempt.state = AttemptState::Completed;
        attempt.score = Some(score);
        attempt.gate_verdicts = gate_verdicts;
        attempt.completed_at_block = Some(current_block);
        let event = ArenaEvent::AttemptCompleted {
            arena_id: attempt.arena_id,
            attempt_id: *attempt_id,
            score,
            evidence_hash: [0; 32],
        };
        self.events.push(event);
        Ok(())
    }

    /// Atomically terminalize an attempt together with external scoring evidence.
    ///
    /// The evidence source must exactly match the arena declaration, its hash
    /// must be non-zero, and the authenticated scorer must be independent from
    /// the participant. Callers should persist the registry immediately after
    /// this operation; the serve adapter rolls back the whole mutation when
    /// persistence fails.
    ///
    /// # Errors
    ///
    /// Returns a state, score, or external-evidence validation error.
    pub fn settle_attempt(
        &mut self,
        attempt_id: &[u8; 32],
        evidence: ScoringEvidence,
        settlement: AttemptSettlement,
    ) -> Result<Attempt, ArenaError> {
        let attempt = self.find_attempt(attempt_id)?;
        let arena = self
            .arenas
            .get(&attempt.arena_id)
            .ok_or(ArenaError::NotFound)?;
        validate_scoring_evidence(arena, attempt, &evidence, self.current_block)?;

        match &settlement {
            AttemptSettlement::Completed { score, .. } => {
                if attempt.state != AttemptState::Evaluating {
                    return Err(ArenaError::InvalidState);
                }
                if !score.is_finite() || !(0.0..=1.0).contains(score) {
                    return Err(ArenaError::InvalidScore);
                }
            }
            AttemptSettlement::Failed { reason } => {
                if !matches!(
                    attempt.state,
                    AttemptState::Running | AttemptState::Evaluating
                ) {
                    return Err(ArenaError::InvalidState);
                }
                if reason.trim().is_empty() {
                    return Err(invalid_evidence("failure reason must not be blank"));
                }
            }
        }

        let current_block = self.current_block;
        let attempt = self.find_attempt_mut(attempt_id)?;
        attempt.completed_at_block = Some(current_block);
        attempt.scoring_evidence = Some(evidence.clone());
        let event = match settlement {
            AttemptSettlement::Completed {
                score,
                gate_verdicts,
            } => {
                attempt.state = AttemptState::Completed;
                attempt.score = Some(score);
                attempt.gate_verdicts = gate_verdicts;
                attempt.failure_reason = None;
                ArenaEvent::AttemptCompleted {
                    arena_id: attempt.arena_id,
                    attempt_id: *attempt_id,
                    score,
                    evidence_hash: evidence.evidence_hash,
                }
            }
            AttemptSettlement::Failed { reason } => {
                attempt.state = AttemptState::Failed;
                attempt.score = None;
                attempt.gate_verdicts.clear();
                attempt.failure_reason = Some(reason.clone());
                ArenaEvent::AttemptFailed {
                    arena_id: attempt.arena_id,
                    attempt_id: *attempt_id,
                    evidence_hash: evidence.evidence_hash,
                    reason,
                }
            }
        };
        let settled = attempt.clone();
        self.events.push(event);
        Ok(settled)
    }

    /// Fail an attempt currently Running or Evaluating.
    ///
    /// # Errors
    ///
    /// Returns NotFound or InvalidState.
    pub fn fail_attempt(&mut self, attempt_id: &[u8; 32]) -> Result<(), ArenaError> {
        let current_block = self.current_block;
        let attempt = self.find_attempt_mut(attempt_id)?;
        if !matches!(
            attempt.state,
            AttemptState::Running | AttemptState::Evaluating
        ) {
            return Err(ArenaError::InvalidState);
        }
        attempt.state = AttemptState::Failed;
        attempt.completed_at_block = Some(current_block);
        let event = ArenaEvent::AttemptFailed {
            arena_id: attempt.arena_id,
            attempt_id: *attempt_id,
            evidence_hash: [0; 32],
            reason: String::new(),
        };
        self.events.push(event);
        Ok(())
    }

    /// Read all attempts for an arena.
    ///
    /// # Errors
    ///
    /// Returns NotFound when the arena does not exist.
    pub fn get_arena_attempts(&self, arena_id: &[u8; 32]) -> Result<&[Attempt], ArenaError> {
        if !self.arenas.contains_key(arena_id) {
            return Err(ArenaError::NotFound);
        }
        Ok(self.attempts.get(arena_id).map_or(&[], Vec::as_slice))
    }

    /// Read one agent's attempts in creation order.
    ///
    /// # Errors
    ///
    /// Returns NotFound when the arena does not exist.
    pub fn get_attempts_for_agent(
        &self,
        arena_id: &[u8; 32],
        agent_identity_id: u256,
    ) -> Result<Vec<&Attempt>, ArenaError> {
        Ok(self
            .get_arena_attempts(arena_id)?
            .iter()
            .filter(|attempt| attempt.agent_identity_id == agent_identity_id)
            .collect())
    }

    /// Read an attempt by identifier.
    #[must_use]
    pub fn get_attempt(&self, attempt_id: &[u8; 32]) -> Option<&Attempt> {
        self.find_attempt(attempt_id).ok()
    }

    fn find_attempt(&self, attempt_id: &[u8; 32]) -> Result<&Attempt, ArenaError> {
        self.attempts
            .values()
            .flat_map(|attempts| attempts.iter())
            .find(|attempt| &attempt.id == attempt_id)
            .ok_or(ArenaError::NotFound)
    }

    fn find_attempt_mut(&mut self, attempt_id: &[u8; 32]) -> Result<&mut Attempt, ArenaError> {
        self.attempts
            .values_mut()
            .flat_map(|attempts| attempts.iter_mut())
            .find(|attempt| &attempt.id == attempt_id)
            .ok_or(ArenaError::NotFound)
    }

    /// Recompute an arena leaderboard solely from completed attempts.
    ///
    /// # Errors
    ///
    /// Returns NotFound for an unknown arena.
    pub fn compute_leaderboard(&self, arena_id: &[u8; 32]) -> Result<Leaderboard, ArenaError> {
        let arena = self.arenas.get(arena_id).ok_or(ArenaError::NotFound)?;
        let mut by_agent: HashMap<u256, Vec<&Attempt>> = HashMap::new();
        for attempt in self.attempts.get(arena_id).into_iter().flatten() {
            if attempt.state == AttemptState::Completed && attempt.score.is_some() {
                by_agent
                    .entry(attempt.agent_identity_id)
                    .or_default()
                    .push(attempt);
            }
        }

        let mut entries = by_agent
            .into_iter()
            .map(|(agent_identity_id, mut attempts)| {
                attempts.sort_by_key(|attempt| attempt.completed_at_block.unwrap_or(u64::MAX));
                let scores = attempts
                    .iter()
                    .filter_map(|attempt| attempt.score)
                    .collect::<Vec<_>>();
                let aggregate_score = aggregate(&scores, arena.aggregation);
                let best_score = scores.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                let last_attempt_block = attempts
                    .iter()
                    .filter_map(|attempt| attempt.completed_at_block)
                    .max()
                    .unwrap_or(0);
                LeaderboardEntry {
                    agent_identity_id,
                    rank: 0,
                    aggregate_score,
                    attempt_count: u32::try_from(scores.len()).unwrap_or(u32::MAX),
                    best_score,
                    last_attempt_block,
                }
            })
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| {
            right
                .aggregate_score
                .partial_cmp(&left.aggregate_score)
                .unwrap_or(Ordering::Equal)
                .then_with(|| left.agent_identity_id.cmp(&right.agent_identity_id))
        });
        for (index, entry) in entries.iter_mut().enumerate() {
            entry.rank = u32::try_from(index + 1).unwrap_or(u32::MAX);
        }
        Ok(Leaderboard {
            arena_id: *arena_id,
            entries,
            computed_at_block: self.current_block,
        })
    }

    /// Convenience alias for a freshly derived leaderboard.
    ///
    /// # Errors
    ///
    /// Returns NotFound for an unknown arena.
    pub fn get_leaderboard(&self, arena_id: &[u8; 32]) -> Result<Leaderboard, ArenaError> {
        self.compute_leaderboard(arena_id)
    }

    /// Lock an arena prize in the local escrow adapter.
    ///
    /// # Errors
    ///
    /// Rejects missing/concluded arenas, duplicate escrow, zero amounts, and
    /// malformed release conditions.
    pub fn deposit_prize(
        &mut self,
        arena_id: &[u8; 32],
        depositor_identity_id: u256,
        amount_usdc: u256,
        release_condition: ReleaseCondition,
    ) -> Result<(), ArenaError> {
        if amount_usdc == 0 {
            return Err(invalid_declaration("prize amount must be non-zero"));
        }
        validate_release_condition(release_condition)?;
        if self.escrow.contains_key(arena_id) {
            return Err(ArenaError::DuplicateEscrow);
        }
        let arena = self.arenas.get_mut(arena_id).ok_or(ArenaError::NotFound)?;
        if !matches!(arena.state, ArenaState::Draft | ArenaState::Active) {
            return Err(ArenaError::InvalidState);
        }
        if arena.state == ArenaState::Active
            && self
                .attempts
                .get(arena_id)
                .is_some_and(|attempts| !attempts.is_empty())
        {
            return Err(ArenaError::InvalidState);
        }
        if arena.prize_pool_usdc != 0 && arena.prize_pool_usdc != amount_usdc {
            return Err(invalid_declaration(
                "escrow amount must equal the arena's declared prize pool",
            ));
        }
        arena.prize_pool_usdc = amount_usdc;
        self.escrow.insert(
            *arena_id,
            BountyEscrow {
                arena_id: *arena_id,
                depositor_identity_id,
                amount_usdc,
                released: false,
                release_condition,
            },
        );
        self.events.push(ArenaEvent::PrizeDeposited {
            arena_id: *arena_id,
            amount_usdc,
        });
        Ok(())
    }

    /// Read local prize escrow state.
    #[must_use]
    pub fn get_escrow(&self, arena_id: &[u8; 32]) -> Option<&BountyEscrow> {
        self.escrow.get(arena_id)
    }

    /// Distribute a concluded arena's prize pool from its derived leaderboard.
    ///
    /// # Errors
    ///
    /// Rejects unreleased lifecycle states, repeated release, or no eligible winners.
    pub fn distribute_prizes(
        &mut self,
        arena_id: &[u8; 32],
    ) -> Result<Vec<(u256, u256)>, ArenaError> {
        let arena = self.arenas.get(arena_id).ok_or(ArenaError::NotFound)?;
        if arena.state != ArenaState::Concluded {
            return Err(ArenaError::InvalidState);
        }
        let leaderboard = self.compute_leaderboard(arena_id)?;
        let escrow = self
            .escrow
            .get(arena_id)
            .ok_or(ArenaError::EscrowNotFound)?;
        if escrow.released {
            return Err(ArenaError::EscrowReleased);
        }
        let payouts = payouts_for(
            &leaderboard.entries,
            escrow.amount_usdc,
            escrow.release_condition,
        )?;
        self.escrow
            .get_mut(arena_id)
            .expect("escrow validated above")
            .released = true;
        self.events.push(ArenaEvent::PrizeDistributed {
            arena_id: *arena_id,
            payouts: payouts.clone(),
        });
        Ok(payouts)
    }

    /// Refund an unreleased prize pool after the arena concludes.
    ///
    /// # Errors
    ///
    /// Rejects non-concluded arenas, missing escrow, or repeated release.
    pub fn refund_prize(&mut self, arena_id: &[u8; 32]) -> Result<u256, ArenaError> {
        let arena = self.arenas.get(arena_id).ok_or(ArenaError::NotFound)?;
        if arena.state != ArenaState::Concluded {
            return Err(ArenaError::InvalidState);
        }
        let escrow = self
            .escrow
            .get_mut(arena_id)
            .ok_or(ArenaError::EscrowNotFound)?;
        if escrow.released {
            return Err(ArenaError::EscrowReleased);
        }
        escrow.released = true;
        let amount = escrow.amount_usdc;
        self.events.push(ArenaEvent::PrizeRefunded {
            arena_id: *arena_id,
            amount_usdc: amount,
        });
        Ok(amount)
    }

    /// Compute the reputation effect for one completed attempt.
    ///
    /// # Errors
    ///
    /// Returns NotFound, InvalidState, or InvalidScore.
    pub fn compute_reputation_effect(
        &self,
        arena_id: &[u8; 32],
        attempt_id: &[u8; 32],
    ) -> Result<ArenaReputationEffect, ArenaError> {
        let arena = self.arenas.get(arena_id).ok_or(ArenaError::NotFound)?;
        let attempt = self.find_attempt(attempt_id)?;
        if attempt.arena_id != *arena_id || attempt.state != AttemptState::Completed {
            return Err(ArenaError::InvalidState);
        }
        let score = attempt.score.ok_or(ArenaError::InvalidScore)?;
        Ok(ArenaReputationEffect {
            agent_identity_id: attempt.agent_identity_id,
            domain: arena.category.to_reputation_domain().to_string(),
            delta: (score - 0.5) * arena.weight,
            arena_id: *arena_id,
            attempt_id: *attempt_id,
        })
    }

    /// Events recorded in operation order.
    #[must_use]
    pub fn events(&self) -> &[ArenaEvent] {
        &self.events
    }

    /// Durable events not yet acknowledged by the service projection cursor.
    #[must_use]
    pub fn pending_events(&self) -> &[ArenaEvent] {
        &self.events[self.projected_event_count..]
    }

    /// Number of durable events acknowledged by the service projection.
    #[must_use]
    pub const fn projected_event_count(&self) -> usize {
        self.projected_event_count
    }

    /// Advance the durable event projection cursor after successful publication.
    ///
    /// # Errors
    ///
    /// Rejects cursor rewinds and positions beyond the durable event outbox.
    pub fn acknowledge_event_projection(&mut self, count: usize) -> Result<(), ArenaError> {
        if count < self.projected_event_count || count > self.events.len() {
            return Err(ArenaError::InvalidState);
        }
        self.projected_event_count = count;
        Ok(())
    }

    /// Publish all recorded events to a Bus and return assigned sequence numbers.
    ///
    /// # Errors
    ///
    /// Returns a serialization or Bus error without dropping recorded events.
    pub fn publish_events<B: Bus>(&self, bus: &B) -> Result<Vec<u64>, ArenaError> {
        self.events
            .iter()
            .map(|event| {
                let pulse = event.to_pulse()?;
                bus.publish(pulse)
                    .map_err(|error| ArenaError::EventPublication {
                        message: error.to_string(),
                    })
            })
            .collect()
    }

    /// Atomically persist the local adapter snapshot as JSON.
    ///
    /// # Errors
    ///
    /// Returns serialization or filesystem errors.
    pub fn persist(&self, path: impl AsRef<Path>) -> Result<(), ArenaError> {
        let path = path.as_ref();
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        if parent != Path::new(".") {
            fs::create_dir_all(parent).map_err(persistence_error)?;
        }
        let mut arenas = self.arenas.values().cloned().collect::<Vec<_>>();
        arenas.sort_by_key(|arena| arena.id);
        let mut attempts = self
            .attempts
            .iter()
            .map(|(arena_id, attempts)| AttemptBucket {
                arena_id: *arena_id,
                attempts: attempts.clone(),
            })
            .collect::<Vec<_>>();
        attempts.sort_by_key(|bucket| bucket.arena_id);
        let mut escrow = self.escrow.values().cloned().collect::<Vec<_>>();
        escrow.sort_by_key(|entry| entry.arena_id);
        let mut snapshot = ArenaSnapshot {
            schema_version: 2,
            arenas,
            attempts,
            escrow,
            current_block: self.current_block,
            events: self.events.clone(),
            projected_event_count: self.projected_event_count,
            content_hash: String::new(),
        };
        snapshot.content_hash = arena_snapshot_hash(&snapshot)?;
        let bytes = serde_json::to_vec_pretty(&snapshot).map_err(persistence_error)?;
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("arenas.json");
        let (temporary, mut file) = loop {
            let nonce = ARENA_SNAPSHOT_WRITE_NONCE.fetch_add(1, AtomicOrdering::Relaxed);
            let candidate =
                path.with_file_name(format!(".{file_name}.{}.{}.tmp", std::process::id(), nonce));
            match OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&candidate)
            {
                Ok(file) => break (candidate, file),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(persistence_error(error)),
            }
        };
        let result = (|| {
            file.write_all(&bytes).map_err(persistence_error)?;
            file.sync_all().map_err(persistence_error)?;
            drop(file);
            fs::rename(&temporary, path).map_err(persistence_error)?;
            // Directory sync makes the rename durable on filesystems that
            // support syncing directory entries. Some platforms reject it, so
            // opening/syncing the directory remains best effort.
            if let Ok(directory) = fs::File::open(parent) {
                let _ = directory.sync_all();
            }
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }

    /// Open a persisted registry, or create an empty registry when absent.
    ///
    /// # Errors
    ///
    /// Returns malformed JSON or filesystem errors.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ArenaError> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::new());
        }
        let bytes = fs::read(path).map_err(persistence_error)?;
        let snapshot: ArenaSnapshot = serde_json::from_slice(&bytes).map_err(persistence_error)?;
        validate_arena_snapshot_header(&snapshot)?;
        let mut arenas = HashMap::new();
        for arena in snapshot.arenas {
            validate_arena(&arena)?;
            if arenas.insert(arena.id, arena).is_some() {
                return Err(persistence_corruption("duplicate arena identifier"));
            }
        }
        let mut attempts = HashMap::new();
        let mut attempt_ids = HashSet::new();
        for bucket in snapshot.attempts {
            let arena = arenas.get(&bucket.arena_id).ok_or_else(|| {
                persistence_corruption("attempt bucket references an unknown arena")
            })?;
            if bucket
                .attempts
                .iter()
                .any(|attempt| attempt.arena_id != bucket.arena_id)
            {
                return Err(persistence_corruption(
                    "attempt is stored under the wrong arena",
                ));
            }
            if bucket
                .attempts
                .iter()
                .any(|attempt| !attempt_ids.insert(attempt.id))
            {
                return Err(persistence_corruption("duplicate attempt identifier"));
            }
            validate_snapshot_attempts(arena, &bucket.attempts, snapshot.current_block)?;
            if attempts.insert(bucket.arena_id, bucket.attempts).is_some() {
                return Err(persistence_corruption("duplicate attempt bucket"));
            }
        }
        if attempts.len() != arenas.len()
            || arenas
                .keys()
                .any(|arena_id| !attempts.contains_key(arena_id))
        {
            return Err(persistence_corruption(
                "snapshot must contain exactly one attempt bucket per arena",
            ));
        }
        let mut escrow = HashMap::new();
        for entry in snapshot.escrow {
            if !arenas.contains_key(&entry.arena_id) || entry.amount_usdc == 0 {
                return Err(persistence_corruption("invalid arena escrow entry"));
            }
            validate_release_condition(entry.release_condition)?;
            if escrow.insert(entry.arena_id, entry).is_some() {
                return Err(persistence_corruption("duplicate arena escrow entry"));
            }
        }
        for arena in arenas.values() {
            let entry = escrow.get(&arena.id);
            match (arena.prize_pool_usdc, arena.state, entry) {
                (0, _, None) => {}
                (0, _, Some(_)) => {
                    return Err(persistence_corruption(
                        "zero-prize arena contains an escrow entry",
                    ));
                }
                (amount, ArenaState::Draft, Some(entry))
                    if entry.amount_usdc == amount && !entry.released => {}
                (_, ArenaState::Draft, None) => {}
                (amount, ArenaState::Active | ArenaState::Paused, Some(entry))
                    if entry.amount_usdc == amount && !entry.released => {}
                (amount, ArenaState::Concluded, Some(entry)) if entry.amount_usdc == amount => {}
                _ => {
                    return Err(persistence_corruption(
                        "arena prize declaration and escrow lifecycle are inconsistent",
                    ));
                }
            }
        }
        Ok(Self {
            arenas,
            attempts,
            escrow,
            current_block: snapshot.current_block,
            events: snapshot.events,
            projected_event_count: snapshot.projected_event_count,
        })
    }
}

fn validate_arena_snapshot_header(snapshot: &ArenaSnapshot) -> Result<(), ArenaError> {
    if snapshot.schema_version != 2 {
        return Err(ArenaError::Persistence {
            message: format!(
                "unsupported arena snapshot schema version {}",
                snapshot.schema_version
            ),
        });
    }
    if snapshot.content_hash != arena_snapshot_hash(snapshot)? {
        return Err(persistence_corruption(
            "arena snapshot integrity hash does not match its content",
        ));
    }
    if snapshot.projected_event_count > snapshot.events.len() {
        return Err(persistence_corruption(
            "arena event projection cursor exceeds the durable outbox",
        ));
    }
    Ok(())
}

fn arena_snapshot_hash(snapshot: &ArenaSnapshot) -> Result<String, ArenaError> {
    let canonical = serde_json::to_vec(&(
        snapshot.schema_version,
        &snapshot.arenas,
        &snapshot.attempts,
        &snapshot.escrow,
        snapshot.current_block,
        &snapshot.events,
        snapshot.projected_event_count,
    ))
    .map_err(persistence_error)?;
    Ok(blake3::hash(&canonical).to_hex().to_string())
}

fn attempt_id(arena_id: &[u8; 32], agent_identity_id: u256, block: u64, ordinal: u64) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"roko-arena-attempt-v1");
    hasher.update(arena_id);
    hasher.update(&agent_identity_id.to_le_bytes());
    hasher.update(&block.to_le_bytes());
    hasher.update(&ordinal.to_le_bytes());
    *hasher.finalize().as_bytes()
}

fn validate_snapshot_attempts(
    arena: &Arena,
    attempts: &[Attempt],
    current_block: u64,
) -> Result<(), ArenaError> {
    if arena.state == ArenaState::Draft && !attempts.is_empty() {
        return Err(persistence_corruption("draft arena contains attempts"));
    }

    let mut per_agent: HashMap<u256, (usize, u64)> = HashMap::new();
    for (ordinal, attempt) in attempts.iter().enumerate() {
        let ordinal = u64::try_from(ordinal)
            .map_err(|_| persistence_corruption("attempt ordinal exceeds u64"))?;
        if attempt.id
            != attempt_id(
                &arena.id,
                attempt.agent_identity_id,
                attempt.started_at_block,
                ordinal,
            )
        {
            return Err(persistence_corruption(
                "attempt identifier does not match its creation fields",
            ));
        }
        if attempt.started_at_block > current_block
            || (arena.deadline_block != 0 && attempt.started_at_block > arena.deadline_block)
        {
            return Err(persistence_corruption(
                "attempt block is outside the arena snapshot bounds",
            ));
        }

        validate_snapshot_attempt_state(attempt, current_block)?;

        if let Some(evidence) = &attempt.scoring_evidence {
            validate_scoring_evidence(arena, attempt, evidence, current_block).map_err(
                |error| {
                    persistence_corruption(format!("invalid persisted scoring evidence: {error}"))
                },
            )?;
        }

        let agent_entry = per_agent
            .entry(attempt.agent_identity_id)
            .or_insert((0, attempt.started_at_block));
        if agent_entry.0 != 0 {
            let next_allowed = agent_entry
                .1
                .checked_add(arena.cooldown_blocks)
                .ok_or_else(|| persistence_corruption("attempt cooldown overflows block range"))?;
            if attempt.started_at_block < next_allowed {
                return Err(persistence_corruption(
                    "persisted attempts violate their arena cooldown",
                ));
            }
        }
        agent_entry.0 += 1;
        agent_entry.1 = attempt.started_at_block;
        if arena.max_attempts_per_agent != 0
            && agent_entry.0 > arena.max_attempts_per_agent as usize
        {
            return Err(persistence_corruption(
                "persisted attempts exceed their per-agent limit",
            ));
        }
    }
    Ok(())
}

fn validate_snapshot_attempt_state(
    attempt: &Attempt,
    current_block: u64,
) -> Result<(), ArenaError> {
    match attempt.state {
        AttemptState::Queued | AttemptState::Running | AttemptState::Evaluating => {
            if attempt.score.is_some()
                || attempt.completed_at_block.is_some()
                || attempt.scoring_evidence.is_some()
                || attempt.failure_reason.is_some()
            {
                return Err(persistence_corruption(
                    "non-terminal attempt contains terminal result fields",
                ));
            }
        }
        AttemptState::Completed => {
            let score = attempt
                .score
                .ok_or_else(|| persistence_corruption("completed attempt is missing its score"))?;
            if !score.is_finite() || !(0.0..=1.0).contains(&score) {
                return Err(persistence_corruption(
                    "completed attempt contains an invalid normalized score",
                ));
            }
            if attempt.failure_reason.is_some() {
                return Err(persistence_corruption(
                    "completed attempt contains a failure reason",
                ));
            }
            validate_completion_block(attempt, current_block)?;
        }
        AttemptState::Failed | AttemptState::Cancelled | AttemptState::Disqualified => {
            if attempt.score.is_some() {
                return Err(persistence_corruption(
                    "unsuccessful attempt contains a score",
                ));
            }
            if attempt.state == AttemptState::Failed
                && attempt.scoring_evidence.is_some()
                && attempt
                    .failure_reason
                    .as_deref()
                    .is_none_or(|reason| reason.trim().is_empty())
            {
                return Err(persistence_corruption(
                    "externally settled failure is missing its reason",
                ));
            }
            validate_completion_block(attempt, current_block)?;
        }
    }
    Ok(())
}

fn validate_completion_block(attempt: &Attempt, current_block: u64) -> Result<(), ArenaError> {
    let completed_at = attempt.completed_at_block.ok_or_else(|| {
        persistence_corruption("terminal attempt is missing its completion block")
    })?;
    if completed_at < attempt.started_at_block || completed_at > current_block {
        return Err(persistence_corruption(
            "attempt completion block is outside the snapshot bounds",
        ));
    }
    Ok(())
}

const fn default_arena_weight() -> f64 {
    1.0
}

fn validate_arena(arena: &Arena) -> Result<(), ArenaError> {
    if arena.id == [0; 32] {
        return Err(invalid_declaration("arena id must be non-zero"));
    }
    if arena.name.trim().is_empty() {
        return Err(invalid_declaration("arena name must not be empty"));
    }
    if !arena.weight.is_finite() || arena.weight <= 0.0 {
        return Err(invalid_declaration(
            "arena weight must be finite and positive",
        ));
    }
    if matches!(
        &arena.ground_truth,
        GroundTruthSource::BenchmarkDataset(reference)
            | GroundTruthSource::ExternalOracle(reference)
            if reference.trim().is_empty()
    ) {
        return Err(invalid_declaration(
            "external ground-truth reference must not be empty",
        ));
    }
    validate_aggregation(arena.aggregation)?;
    validate_scoring(&arena.scoring)
}

fn validate_aggregation(rule: AggregationRule) -> Result<(), ArenaError> {
    match rule {
        AggregationRule::BestOf(0) | AggregationRule::AverageLastN(0) => {
            Err(invalid_declaration("aggregation window must be non-zero"))
        }
        AggregationRule::EWMA(alpha) if !alpha.is_finite() || !(0.0..=1.0).contains(&alpha) => Err(
            invalid_declaration("EWMA alpha must be finite and in (0, 1]"),
        ),
        AggregationRule::EWMA(0.0) => Err(invalid_declaration(
            "EWMA alpha must be finite and in (0, 1]",
        )),
        _ => Ok(()),
    }
}

fn validate_scoring(scoring: &ScoringFunction) -> Result<(), ArenaError> {
    if let ScoringFunction::Composite(components) = scoring {
        if components.is_empty() {
            return Err(invalid_declaration(
                "composite scoring needs at least one dimension",
            ));
        }
        for component in components {
            if component.name.trim().is_empty()
                || !component.weight.is_finite()
                || component.weight <= 0.0
            {
                return Err(invalid_declaration(
                    "scoring dimensions need a name and positive finite presentation weight",
                ));
            }
            validate_scoring(&component.function)?;
        }
    }
    Ok(())
}

fn validate_scoring_evidence(
    arena: &Arena,
    attempt: &Attempt,
    evidence: &ScoringEvidence,
    current_block: u64,
) -> Result<(), ArenaError> {
    if evidence.source != arena.ground_truth {
        return Err(invalid_evidence(
            "evidence source does not match the arena ground-truth declaration",
        ));
    }
    if evidence.evidence_hash == [0; 32] {
        return Err(invalid_evidence("evidence hash must be non-zero"));
    }
    if evidence.subject_output_hash == [0; 32]
        || attempt.output_hash != Some(evidence.subject_output_hash)
    {
        return Err(invalid_evidence(
            "evidence subject must match the submitted output hash",
        ));
    }
    if evidence.scorer_identity_id == 0 {
        return Err(invalid_evidence("scorer identity must be non-zero"));
    }
    if evidence.scorer_identity_id == attempt.agent_identity_id {
        return Err(ArenaError::SelfGrading);
    }
    if evidence.scorer_principal.trim().is_empty() {
        return Err(invalid_evidence("scorer principal must not be blank"));
    }
    if !attempt.participant_principal.is_empty()
        && evidence.scorer_principal == attempt.participant_principal
    {
        return Err(ArenaError::SelfGrading);
    }
    if evidence.observed_at_block < attempt.started_at_block
        || evidence.observed_at_block > current_block
    {
        return Err(invalid_evidence(
            "evidence observation block is outside the attempt lifecycle",
        ));
    }
    Ok(())
}

fn validate_release_condition(condition: ReleaseCondition) -> Result<(), ArenaError> {
    match condition {
        ReleaseCondition::TopN(0) => Err(invalid_declaration("TopN must be non-zero")),
        ReleaseCondition::AllAboveThreshold(value) if !value.is_finite() => {
            Err(invalid_declaration("release threshold must be finite"))
        }
        _ => Ok(()),
    }
}

fn invalid_declaration(message: impl Into<String>) -> ArenaError {
    ArenaError::InvalidDeclaration {
        message: message.into(),
    }
}

fn invalid_evidence(message: impl Into<String>) -> ArenaError {
    ArenaError::InvalidEvidence {
        message: message.into(),
    }
}

fn persistence_error(error: impl std::fmt::Display) -> ArenaError {
    ArenaError::Persistence {
        message: error.to_string(),
    }
}

fn persistence_corruption(message: impl Into<String>) -> ArenaError {
    ArenaError::Persistence {
        message: message.into(),
    }
}

fn aggregate(scores: &[f64], rule: AggregationRule) -> f64 {
    match rule {
        AggregationRule::BestOf(count) => {
            let mut ranked = scores.to_vec();
            ranked.sort_by(|left, right| right.partial_cmp(left).unwrap_or(Ordering::Equal));
            mean(&ranked[..ranked.len().min(count)])
        }
        AggregationRule::AverageLastN(count) => mean(&scores[scores.len().saturating_sub(count)..]),
        AggregationRule::EWMA(alpha) => scores
            .iter()
            .copied()
            .reduce(|prior, current| alpha.mul_add(current, (1.0 - alpha) * prior))
            .unwrap_or(0.0),
        AggregationRule::Median => {
            let mut sorted = scores.to_vec();
            sorted.sort_by(|left, right| left.partial_cmp(right).unwrap_or(Ordering::Equal));
            let middle = sorted.len() / 2;
            if sorted.len().is_multiple_of(2) {
                (sorted[middle - 1] + sorted[middle]) / 2.0
            } else {
                sorted[middle]
            }
        }
    }
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

fn payouts_for(
    entries: &[LeaderboardEntry],
    amount: u256,
    condition: ReleaseCondition,
) -> Result<Vec<(u256, u256)>, ArenaError> {
    let eligible = match condition {
        ReleaseCondition::TopN(count) => entries.iter().take(count).collect::<Vec<_>>(),
        ReleaseCondition::AllAboveThreshold(threshold) => entries
            .iter()
            .filter(|entry| entry.aggregate_score >= threshold)
            .collect::<Vec<_>>(),
        ReleaseCondition::ProportionalToScore => entries
            .iter()
            .filter(|entry| entry.aggregate_score > 0.0)
            .collect::<Vec<_>>(),
    };
    if eligible.is_empty() {
        return Err(ArenaError::NoEligibleWinners);
    }
    match condition {
        ReleaseCondition::TopN(_) | ReleaseCondition::AllAboveThreshold(_) => {
            let count = eligible.len() as u128;
            let base = amount / count;
            let remainder = amount % count;
            Ok(eligible
                .into_iter()
                .enumerate()
                .map(|(index, entry)| {
                    let bonus = u128::from((index as u128) < remainder);
                    (entry.agent_identity_id, base + bonus)
                })
                .collect())
        }
        ReleaseCondition::ProportionalToScore => {
            let weighted = eligible
                .into_iter()
                .filter_map(|entry| {
                    let weight = score_fixed_point(entry.aggregate_score);
                    (weight != 0).then_some((entry, weight))
                })
                .collect::<Vec<_>>();
            if weighted.is_empty() {
                return Err(ArenaError::NoEligibleWinners);
            }
            let total_weight = weighted.iter().try_fold(0_u128, |total, (_, weight)| {
                total
                    .checked_add(*weight)
                    .ok_or_else(|| invalid_declaration("proportional score weights exceed u128"))
            })?;
            let mut distributed = 0_u128;
            let mut payouts = Vec::with_capacity(weighted.len());
            let mut remainders = Vec::with_capacity(weighted.len());
            for (index, (entry, weight)) in weighted.iter().enumerate() {
                let (share, remainder) = mul_div_rem_u128(amount, *weight, total_weight)?;
                distributed = distributed
                    .checked_add(share)
                    .ok_or_else(|| invalid_declaration("proportional payout total overflowed"))?;
                payouts.push((entry.agent_identity_id, share));
                remainders.push((index, remainder));
            }
            let leftover = amount
                .checked_sub(distributed)
                .ok_or_else(|| invalid_declaration("proportional payouts exceeded escrow"))?;
            let leftover = usize::try_from(leftover).map_err(|_| {
                invalid_declaration("proportional payout remainder exceeds recipient count")
            })?;
            if leftover > remainders.len() {
                return Err(invalid_declaration(
                    "proportional payout remainder exceeds recipient count",
                ));
            }
            // Hamilton's largest-remainder method conserves the full escrow and
            // keeps every integer allocation within one base unit of its exact
            // fixed-point ratio. Leaderboard order breaks equal remainders.
            remainders
                .sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
            for (index, _) in remainders.into_iter().take(leftover) {
                payouts[index].1 += 1;
            }
            Ok(payouts)
        }
    }
}

fn score_fixed_point(score: f64) -> u128 {
    debug_assert!(score.is_finite() && (0.0..=1.0).contains(&score));
    (score * SCORE_FIXED_POINT_SCALE as f64).round() as u128
}

/// Return `floor(value * numerator / denominator)` and its exact division
/// remainder without ever narrowing `value` to a float or overflowing u128.
fn mul_div_rem_u128(
    value: u128,
    numerator: u128,
    denominator: u128,
) -> Result<(u128, u128), ArenaError> {
    if denominator == 0 || numerator > denominator {
        return Err(invalid_declaration("invalid proportional payout ratio"));
    }
    let (high, low) = full_mul_u128(value, numerator);
    div_u256_by_u128(high, low, denominator)
}

fn full_mul_u128(left: u128, right: u128) -> (u128, u128) {
    const LOW_MASK: u128 = u64::MAX as u128;
    let left_low = left & LOW_MASK;
    let left_high = left >> 64;
    let right_low = right & LOW_MASK;
    let right_high = right >> 64;

    let low_product = left_low * right_low;
    let cross_left = left_low * right_high;
    let cross_right = left_high * right_low;
    let high_product = left_high * right_high;
    let middle = (low_product >> 64) + (cross_left & LOW_MASK) + (cross_right & LOW_MASK);
    let low = (low_product & LOW_MASK) | ((middle & LOW_MASK) << 64);
    let high = high_product + (cross_left >> 64) + (cross_right >> 64) + (middle >> 64);
    (high, low)
}

fn div_u256_by_u128(high: u128, low: u128, denominator: u128) -> Result<(u128, u128), ArenaError> {
    let mut quotient = 0_u128;
    let mut remainder = 0_u128;
    for bit_index in (0_usize..256).rev() {
        let bit = if bit_index >= 128 {
            (high >> (bit_index - 128)) & 1
        } else {
            (low >> bit_index) & 1
        };
        let carried = remainder >> 127 != 0;
        let shifted = (remainder << 1) | bit;
        if carried || shifted >= denominator {
            remainder = shifted.wrapping_sub(denominator);
            if bit_index >= 128 {
                return Err(invalid_declaration(
                    "proportional payout quotient exceeds u128",
                ));
            }
            quotient |= 1_u128 << bit_index;
        } else {
            remainder = shifted;
        }
    }
    Ok((quotient, remainder))
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use roko_core::{MemoryBus, TopicFilter};

    use super::*;

    fn arena(id: u8, aggregation: AggregationRule) -> Arena {
        Arena {
            id: [id; 32],
            name: format!("arena-{id}"),
            description: "deterministic arena".to_string(),
            category: ArenaCategory::Coding,
            state: ArenaState::Active,
            task_source: TaskSource::Static,
            scoring: ScoringFunction::Binary(BinaryCriterion::TestSuitePass),
            aggregation,
            weight: 1.0,
            creator_identity_id: 99,
            creator_principal: String::new(),
            prize_pool_usdc: 0,
            max_attempts_per_agent: 0,
            cooldown_blocks: 0,
            deadline_block: 0,
            ground_truth: GroundTruthSource::TestSuite,
        }
    }

    fn active_registry(rule: AggregationRule) -> ArenaRegistry {
        let mut registry = ArenaRegistry::new();
        registry.create_arena(arena(1, rule)).unwrap();
        assert_eq!(
            registry.get_arena(&[1; 32]).unwrap().state,
            ArenaState::Draft
        );
        registry.activate_arena(&[1; 32]).unwrap();
        registry
    }

    fn complete(registry: &mut ArenaRegistry, agent: u256, score: f64, block: u64) {
        registry.set_block(block);
        let attempt = registry.start_attempt(&[1; 32], agent).unwrap();
        registry
            .submit_attempt_with_output(&attempt.id, Some([7; 32]))
            .unwrap();
        registry
            .complete_attempt(&attempt.id, score, vec![true])
            .unwrap();
    }

    #[test]
    fn arena_lifecycle_validates_transitions() {
        let mut registry = ArenaRegistry::new();
        registry
            .create_arena(arena(1, AggregationRule::Median))
            .unwrap();
        registry.activate_arena(&[1; 32]).unwrap();
        registry.pause_arena(&[1; 32]).unwrap();
        registry.activate_arena(&[1; 32]).unwrap();
        registry.conclude_arena(&[1; 32]).unwrap();
        assert_eq!(
            registry.activate_arena(&[1; 32]),
            Err(ArenaError::InvalidState)
        );
    }

    #[test]
    fn attempt_runs_full_queue_to_completed_lifecycle() {
        let mut registry = active_registry(AggregationRule::Median);
        let queued = registry.queue_attempt(&[1; 32], 7).unwrap();
        assert_eq!(queued.state, AttemptState::Queued);
        let running = registry
            .start_queued_attempt(&queued.id, Some([3; 32]))
            .unwrap();
        assert_eq!(running.state, AttemptState::Running);
        registry
            .submit_attempt_with_output(&queued.id, Some([4; 32]))
            .unwrap();
        registry
            .complete_attempt(&queued.id, 0.8, vec![true])
            .unwrap();
        assert_eq!(
            registry.get_attempt(&queued.id).unwrap().state,
            AttemptState::Completed
        );
    }

    #[test]
    fn cooldown_is_enforced_by_block() {
        let mut definition = arena(1, AggregationRule::Median);
        definition.cooldown_blocks = 5;
        let mut registry = ArenaRegistry::new();
        registry.create_arena(definition).unwrap();
        registry.activate_arena(&[1; 32]).unwrap();
        registry.set_block(10);
        registry.start_attempt(&[1; 32], 7).unwrap();
        registry.set_block(14);
        assert_eq!(
            registry.start_attempt(&[1; 32], 7),
            Err(ArenaError::CooldownActive)
        );
        registry.set_block(15);
        assert!(registry.start_attempt(&[1; 32], 7).is_ok());
    }

    #[test]
    fn cooldown_overflow_and_block_rewind_fail_closed() {
        let mut definition = arena(1, AggregationRule::Median);
        definition.cooldown_blocks = 1;
        let mut registry = ArenaRegistry::new();
        registry.create_arena(definition).unwrap();
        registry.activate_arena(&[1; 32]).unwrap();
        registry.set_block(u64::MAX);
        registry.start_attempt(&[1; 32], 7).unwrap();
        registry.set_block(0);
        assert_eq!(registry.current_block(), u64::MAX);
        assert_eq!(
            registry.start_attempt(&[1; 32], 7),
            Err(ArenaError::CooldownActive)
        );
    }

    #[test]
    fn maximum_attempts_are_enforced() {
        let mut definition = arena(1, AggregationRule::Median);
        definition.max_attempts_per_agent = 1;
        let mut registry = ArenaRegistry::new();
        registry.create_arena(definition).unwrap();
        registry.activate_arena(&[1; 32]).unwrap();
        registry.start_attempt(&[1; 32], 7).unwrap();
        assert_eq!(
            registry.start_attempt(&[1; 32], 7),
            Err(ArenaError::MaxAttemptsReached)
        );
        assert!(registry.start_attempt(&[1; 32], 8).is_ok());
    }

    #[test]
    fn best_of_averages_the_best_n_scores() {
        let mut registry = active_registry(AggregationRule::BestOf(2));
        complete(&mut registry, 7, 0.2, 1);
        complete(&mut registry, 7, 0.8, 2);
        complete(&mut registry, 7, 0.6, 3);
        complete(&mut registry, 8, 0.5, 4);
        let board = registry.compute_leaderboard(&[1; 32]).unwrap();
        assert_eq!(board.entries[0].agent_identity_id, 7);
        assert!((board.entries[0].aggregate_score - 0.7).abs() < 1e-12);
    }

    #[test]
    fn ewma_uses_attempt_completion_order() {
        let mut registry = active_registry(AggregationRule::EWMA(0.5));
        complete(&mut registry, 7, 0.2, 1);
        complete(&mut registry, 7, 0.8, 2);
        let board = registry.compute_leaderboard(&[1; 32]).unwrap();
        assert!((board.entries[0].aggregate_score - 0.5).abs() < 1e-12);
    }

    #[test]
    fn top_n_distribution_is_exact_and_deterministic() {
        let mut registry = active_registry(AggregationRule::Median);
        registry
            .deposit_prize(&[1; 32], 99, 101, ReleaseCondition::TopN(2))
            .unwrap();
        complete(&mut registry, 7, 0.9, 1);
        complete(&mut registry, 8, 0.8, 2);
        complete(&mut registry, 9, 0.7, 3);
        registry.conclude_arena(&[1; 32]).unwrap();
        assert_eq!(
            registry.distribute_prizes(&[1; 32]).unwrap(),
            vec![(7, 51), (8, 50)]
        );
    }

    #[test]
    fn proportional_distribution_is_exact_across_the_full_u128_range() {
        let mut registry = active_registry(AggregationRule::Median);
        registry
            .deposit_prize(
                &[1; 32],
                99,
                u128::MAX,
                ReleaseCondition::ProportionalToScore,
            )
            .unwrap();
        complete(&mut registry, 7, 0.7, 1);
        complete(&mut registry, 8, 0.2, 2);
        complete(&mut registry, 9, 0.1, 3);
        registry.conclude_arena(&[1; 32]).unwrap();

        let payouts = registry.distribute_prizes(&[1; 32]).unwrap();
        let quotient = u128::MAX / 10;
        let remainder = u128::MAX % 10;
        let expected = vec![
            (7, quotient * 7 + (remainder * 7) / 10 + 1),
            (8, quotient * 2 + (remainder * 2) / 10),
            (9, quotient + remainder / 10),
        ];
        assert_eq!(payouts, expected);
        assert_eq!(
            payouts.iter().map(|(_, amount)| amount).sum::<u128>(),
            u128::MAX
        );
    }

    #[test]
    fn empty_arena_prize_can_be_refunded_after_conclusion() {
        let mut registry = active_registry(AggregationRule::Median);
        registry
            .deposit_prize(&[1; 32], 99, 100, ReleaseCondition::TopN(1))
            .unwrap();
        registry.conclude_arena(&[1; 32]).unwrap();
        assert_eq!(registry.refund_prize(&[1; 32]).unwrap(), 100);
        assert!(registry.get_escrow(&[1; 32]).unwrap().released);
    }

    #[test]
    fn declared_prize_requires_matching_escrow_before_activation() {
        let mut definition = arena(1, AggregationRule::Median);
        definition.prize_pool_usdc = 100;
        let mut registry = ArenaRegistry::new();
        registry.create_arena(definition).unwrap();
        assert_eq!(
            registry.activate_arena(&[1; 32]),
            Err(ArenaError::PrizeEscrowRequired)
        );
        assert!(
            registry
                .deposit_prize(&[1; 32], 99, 99, ReleaseCondition::TopN(1))
                .is_err()
        );
        registry
            .deposit_prize(&[1; 32], 99, 100, ReleaseCondition::TopN(1))
            .unwrap();
        registry.activate_arena(&[1; 32]).unwrap();
    }

    #[test]
    fn human_review_requires_an_independent_evaluator() {
        let mut definition = arena(1, AggregationRule::Median);
        definition.ground_truth = GroundTruthSource::HumanReview;
        let mut registry = ArenaRegistry::new();
        registry.create_arena(definition).unwrap();
        registry.activate_arena(&[1; 32]).unwrap();
        let attempt = registry.start_attempt(&[1; 32], 7).unwrap();
        registry
            .submit_attempt_with_output(&attempt.id, Some([2; 32]))
            .unwrap();
        assert_eq!(
            registry.complete_attempt(&attempt.id, 0.8, vec![true]),
            Err(ArenaError::EvaluatorIdentityRequired)
        );
        assert_eq!(
            registry.complete_attempt_by(&attempt.id, 7, 0.8, vec![true]),
            Err(ArenaError::SelfGrading)
        );
        registry
            .complete_attempt_by(&attempt.id, 8, 0.8, vec![true])
            .unwrap();
    }

    #[test]
    fn external_settlement_binds_scorer_source_and_submitted_output() {
        let mut registry = active_registry(AggregationRule::Median);
        registry.set_block(1);
        let attempt = registry
            .start_attempt_for_principal(&[1; 32], 7, "participant".to_string(), Some([2; 32]))
            .unwrap();
        registry
            .submit_attempt_with_output(&attempt.id, Some([3; 32]))
            .unwrap();
        registry.set_block(2);
        let evidence = ScoringEvidence {
            source: GroundTruthSource::TestSuite,
            scorer_identity_id: 9,
            scorer_principal: "scorer".to_string(),
            evidence_hash: [4; 32],
            subject_output_hash: [3; 32],
            observed_at_block: 2,
        };

        let mut wrong_subject = evidence.clone();
        wrong_subject.subject_output_hash = [5; 32];
        assert!(matches!(
            registry.settle_attempt(
                &attempt.id,
                wrong_subject,
                AttemptSettlement::Completed {
                    score: 0.8,
                    gate_verdicts: vec![true],
                },
            ),
            Err(ArenaError::InvalidEvidence { .. })
        ));
        let mut self_graded = evidence.clone();
        self_graded.scorer_principal = "participant".to_string();
        assert_eq!(
            registry.settle_attempt(
                &attempt.id,
                self_graded,
                AttemptSettlement::Completed {
                    score: 0.8,
                    gate_verdicts: vec![true],
                },
            ),
            Err(ArenaError::SelfGrading)
        );
        let settled = registry
            .settle_attempt(
                &attempt.id,
                evidence,
                AttemptSettlement::Completed {
                    score: 0.8,
                    gate_verdicts: vec![true],
                },
            )
            .unwrap();
        assert_eq!(settled.state, AttemptState::Completed);
        assert_eq!(
            settled.scoring_evidence.unwrap().subject_output_hash,
            [3; 32]
        );

        registry.set_block(3);
        let failed = registry
            .start_attempt_for_principal(
                &[1; 32],
                8,
                "other-participant".to_string(),
                Some([6; 32]),
            )
            .unwrap();
        registry
            .submit_attempt_with_output(&failed.id, Some([7; 32]))
            .unwrap();
        let failed = registry
            .settle_attempt(
                &failed.id,
                ScoringEvidence {
                    source: GroundTruthSource::TestSuite,
                    scorer_identity_id: 9,
                    scorer_principal: "scorer".to_string(),
                    evidence_hash: [8; 32],
                    subject_output_hash: [7; 32],
                    observed_at_block: 3,
                },
                AttemptSettlement::Failed {
                    reason: "external gate failed".to_string(),
                },
            )
            .unwrap();
        assert_eq!(failed.state, AttemptState::Failed);
        assert_eq!(
            failed.failure_reason.as_deref(),
            Some("external gate failed")
        );
    }

    #[test]
    fn completion_rejects_non_normalized_or_non_finite_scores() {
        let mut registry = active_registry(AggregationRule::Median);
        let attempt = registry.start_attempt(&[1; 32], 7).unwrap();
        registry.submit_attempt(&attempt.id).unwrap();
        for invalid in [-0.1, 1.1, f64::NAN, f64::INFINITY] {
            assert_eq!(
                registry.complete_attempt(&attempt.id, invalid, vec![true]),
                Err(ArenaError::InvalidScore),
                "accepted {invalid:?}"
            );
        }
        registry
            .complete_attempt(&attempt.id, 1.0, vec![true])
            .unwrap();
        let board = registry.compute_leaderboard(&[1; 32]).unwrap();
        assert!(board.entries[0].aggregate_score.is_finite());
        assert!(
            registry
                .compute_reputation_effect(&[1; 32], &attempt.id)
                .unwrap()
                .delta
                .is_finite()
        );
    }

    #[test]
    fn completed_attempt_produces_domain_reputation_effect() {
        let mut registry = active_registry(AggregationRule::Median);
        complete(&mut registry, 7, 0.8, 1);
        let attempt = registry.get_arena_attempts(&[1; 32]).unwrap()[0].id;
        let effect = registry
            .compute_reputation_effect(&[1; 32], &attempt)
            .unwrap();
        assert_eq!(effect.domain, "coding");
        assert!((effect.delta - 0.3).abs() < 1e-12);
    }

    #[test]
    fn event_log_projects_to_arena_bus_topics() {
        let mut registry = active_registry(AggregationRule::Median);
        complete(&mut registry, 7, 0.8, 1);
        let bus = MemoryBus::new(64);
        let sequences = registry.publish_events(&bus).unwrap();
        assert_eq!(sequences.len(), registry.events().len());
        let received = bus.replay_from(0, Some(&TopicFilter::Prefix("arena.".to_string())));
        assert_eq!(received.len(), sequences.len());
        assert!(
            received
                .iter()
                .any(|pulse| pulse.topic == Topic::new("arena.attempt_completed"))
        );
    }

    #[test]
    fn durable_snapshot_round_trips_full_range_u256_and_events() {
        let mut definition = arena(1, AggregationRule::Median);
        definition.creator_identity_id = u128::MAX;
        let mut registry = ArenaRegistry::new();
        registry.create_arena(definition).unwrap();
        registry.activate_arena(&[1; 32]).unwrap();
        registry
            .deposit_prize(
                &[1; 32],
                u128::MAX - 1,
                u128::MAX,
                ReleaseCondition::TopN(1),
            )
            .unwrap();
        complete(&mut registry, u128::MAX - 2, 0.8, 1);
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("roko-arena-{}-{nonce}.json", std::process::id()));
        registry.persist(&path).unwrap();
        let reopened = ArenaRegistry::open(&path).unwrap();
        assert_eq!(reopened.arena_count(), 1);
        assert_eq!(
            reopened.get_arena(&[1; 32]).unwrap().creator_identity_id,
            u128::MAX
        );
        assert_eq!(
            reopened.get_arena(&[1; 32]).unwrap().prize_pool_usdc,
            u128::MAX
        );
        assert_eq!(
            reopened.get_escrow(&[1; 32]).unwrap().depositor_identity_id,
            u128::MAX - 1
        );
        assert_eq!(
            reopened.get_arena_attempts(&[1; 32]).unwrap()[0].agent_identity_id,
            u128::MAX - 2
        );
        assert_eq!(reopened.get_arena_attempts(&[1; 32]).unwrap().len(), 1);
        assert_eq!(reopened.events(), registry.events());
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn durable_snapshot_rejects_corruption_and_unknown_schema() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base =
            std::env::temp_dir().join(format!("roko-arena-invalid-{}-{nonce}", std::process::id()));
        let corrupt = base.with_extension("corrupt.json");
        fs::write(&corrupt, b"not-json").unwrap();
        assert!(matches!(
            ArenaRegistry::open(&corrupt),
            Err(ArenaError::Persistence { .. })
        ));
        fs::remove_file(corrupt).unwrap();

        let unsupported = base.with_extension("unsupported.json");
        fs::write(
            &unsupported,
            br#"{
                "schema_version": 99,
                "arenas": [],
                "attempts": [],
                "escrow": [],
                "current_block": 0,
                "events": []
            }"#,
        )
        .unwrap();
        assert!(matches!(
            ArenaRegistry::open(&unsupported),
            Err(ArenaError::Persistence { .. })
        ));
        fs::remove_file(unsupported).unwrap();
    }

    #[test]
    fn durable_snapshot_rejects_missing_active_escrow_and_incoherent_attempts() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "roko-arena-invariants-{}-{nonce}.json",
            std::process::id()
        ));

        let mut definition = arena(1, AggregationRule::Median);
        definition.prize_pool_usdc = 100;
        let mut registry = ArenaRegistry::new();
        registry.create_arena(definition).unwrap();
        registry
            .deposit_prize(&[1; 32], 99, 100, ReleaseCondition::TopN(1))
            .unwrap();
        registry.activate_arena(&[1; 32]).unwrap();
        complete(&mut registry, 7, 0.8, 1);
        registry.persist(&path).unwrap();
        let pristine = fs::read(&path).unwrap();

        let mut missing_escrow: serde_json::Value = serde_json::from_slice(&pristine).unwrap();
        missing_escrow["escrow"] = serde_json::json!([]);
        fs::write(&path, serde_json::to_vec(&missing_escrow).unwrap()).unwrap();
        assert!(matches!(
            ArenaRegistry::open(&path),
            Err(ArenaError::Persistence { .. })
        ));

        let mut incoherent_attempt: serde_json::Value = serde_json::from_slice(&pristine).unwrap();
        incoherent_attempt["attempts"][0]["attempts"][0]["score"] = serde_json::Value::Null;
        fs::write(&path, serde_json::to_vec(&incoherent_attempt).unwrap()).unwrap();
        assert!(matches!(
            ArenaRegistry::open(&path),
            Err(ArenaError::Persistence { .. })
        ));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn payout_events_round_trip_full_range_u256_as_decimal_strings() {
        let event = ArenaEvent::PrizeDistributed {
            arena_id: [1; 32],
            payouts: vec![(u128::MAX - 1, u128::MAX)],
        };
        let encoded = serde_json::to_string(&event).unwrap();
        assert!(encoded.contains(&format!("\"{}\"", u128::MAX)));
        let decoded: ArenaEvent = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, event);
    }

    #[test]
    fn malformed_declarative_scoring_is_rejected() {
        let mut definition = arena(1, AggregationRule::BestOf(0));
        definition.weight = f64::NAN;
        let mut registry = ArenaRegistry::new();
        assert!(matches!(
            registry.create_arena(definition),
            Err(ArenaError::InvalidDeclaration { .. })
        ));
    }
}
