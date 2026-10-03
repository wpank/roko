//! M3: the calibrated self-model (S04).
//!
//! The self-model forecasts, for each routing arm, whether an attempt will pass its gates, how
//! likely a pass is a false green, and what the attempt will cost and how long it will take. It
//! learns from labelled attempts and stays in shadow mode until a person promotes it.
//!
//! This module holds the types every part shares. Each part has its own file, which the backlog
//! task named in its module comment fills: [`ingest`] and [`legacy`] read the data, [`metrics`]
//! scores forecasts, [`prior`], [`features`], [`logit`], [`cost`] and [`recal`] forecast, and
//! [`model`] combines them. The policies, replay, off-policy evaluation and the promotion gate
//! follow.

pub mod baselines;
pub mod cascade;
pub mod cost;
pub mod features;
pub mod gate;
pub mod ingest;
pub mod legacy;
pub mod logit;
pub mod metrics;
pub mod model;
pub mod ope;
pub mod policy;
pub mod prior;
pub mod recal;
pub mod replay;
pub mod spec_features;

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::telemetry::{AttemptKey, CostSource};

/// The harnesses an arm can run on (S04 §4.1), a fixed vocabulary. S09's arm registry maps
/// every experiment arm to one of them.
pub const HARNESSES: [&str; 6] = [
    "roko",
    "claude-code",
    "codex-cli",
    "gemini-cli",
    "mini-loop",
    "mini-swe-agent",
];

/// The harness that runs Roko's own gates, the only one whose arms carry a verify depth.
pub const ROKO_HARNESS: &str = "roko";

/// The deepest verify depth, V4 (S05 §4.6).
pub const MAX_VERIFY_DEPTH: u8 = 4;

/// How hard an arm works: mapped to the task turn limit and the thinking level.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Effort {
    /// Fewer turns, less thinking.
    Low,
    /// The role's defaults.
    #[default]
    Default,
    /// More turns, more thinking.
    High,
}

impl Effort {
    /// The effort's name in an arm key.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Default => "default",
            Self::High => "high",
        }
    }
}

impl fmt::Display for Effort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Effort {
    type Err = ArmKeyError;

    fn from_str(effort: &str) -> Result<Self, Self::Err> {
        match effort {
            "low" => Ok(Self::Low),
            "default" => Ok(Self::Default),
            "high" => Ok(Self::High),
            other => Err(ArmKeyError::UnknownEffort(other.to_string())),
        }
    }
}

/// Why a string is not an arm key.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ArmKeyError {
    /// The key is not `harness/provider/model@effort`, with an optional `#Vd`.
    #[error("not an arm key: {0:?} (expected harness/provider/model@effort[#Vd])")]
    Malformed(String),
    /// The harness is not one of [`HARNESSES`].
    #[error("unknown harness {0:?}")]
    UnknownHarness(String),
    /// The effort is not `low`, `default` or `high`.
    #[error("unknown effort {0:?} (expected low, default or high)")]
    UnknownEffort(String),
    /// The verify depth is not `V0` to `V4`.
    #[error("bad verify depth {0:?} (expected V0 to V4)")]
    BadDepth(String),
    /// A direct or CLI arm runs no Roko gate, so its key has no verify depth.
    #[error("{0:?} runs no Roko gate, so its key has no verify depth")]
    DepthOnDirectArm(String),
}

/// A routing arm (S04 §4.1): `harness/provider/model@effort`, plus `#Vd`, the verify depth
/// d ∈ 0–4 its gates run at, on Roko arms only. It serialises as its key string.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ArmKey {
    /// One of [`HARNESSES`].
    pub harness: String,
    /// The provider that serves the model.
    pub provider: String,
    /// The model slug; it may itself contain `/`.
    pub model: String,
    /// How hard the arm works.
    pub effort: Effort,
    /// The verify depth, on Roko arms only.
    pub depth: Option<u8>,
}

impl ArmKey {
    /// The Roko arm of `model` on `provider`, at the default effort and verify depth V0 (the
    /// authored verify plus the gate ladder).
    #[must_use]
    pub fn roko(provider: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            harness: ROKO_HARNESS.to_string(),
            provider: provider.into(),
            model: model.into(),
            effort: Effort::Default,
            depth: Some(0),
        }
    }
}

impl fmt::Display for ArmKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}/{}/{}@{}",
            self.harness, self.provider, self.model, self.effort
        )?;
        if let Some(depth) = self.depth {
            write!(f, "#V{depth}")?;
        }
        Ok(())
    }
}

impl FromStr for ArmKey {
    type Err = ArmKeyError;

    fn from_str(key: &str) -> Result<Self, Self::Err> {
        let malformed = || ArmKeyError::Malformed(key.to_string());
        let (body, depth) = match key.rsplit_once('#') {
            Some((body, depth)) => (body, Some(parse_depth(depth)?)),
            None => (key, None),
        };
        let (path, effort) = body.rsplit_once('@').ok_or_else(malformed)?;
        let parts: Vec<&str> = path.splitn(3, '/').collect();
        let [harness, provider, model] = parts[..] else {
            return Err(malformed());
        };
        if harness.is_empty() || provider.is_empty() || model.is_empty() {
            return Err(malformed());
        }
        if !HARNESSES.contains(&harness) {
            return Err(ArmKeyError::UnknownHarness(harness.to_string()));
        }
        if depth.is_some() && harness != ROKO_HARNESS {
            return Err(ArmKeyError::DepthOnDirectArm(harness.to_string()));
        }
        Ok(Self {
            harness: harness.to_string(),
            provider: provider.to_string(),
            model: model.to_string(),
            effort: effort.parse()?,
            depth,
        })
    }
}

/// The depth `d` of a `Vd` suffix.
fn parse_depth(depth: &str) -> Result<u8, ArmKeyError> {
    depth
        .strip_prefix('V')
        .and_then(|digits| digits.parse::<u8>().ok())
        .filter(|depth| *depth <= MAX_VERIFY_DEPTH)
        .ok_or_else(|| ArmKeyError::BadDepth(depth.to_string()))
}

impl TryFrom<String> for ArmKey {
    type Error = ArmKeyError;

    fn try_from(key: String) -> Result<Self, Self::Error> {
        key.parse()
    }
}

impl From<ArmKey> for String {
    fn from(arm: ArmKey) -> Self {
        arm.to_string()
    }
}

/// Where a label came from (S04 §4.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LabelSource {
    /// The agent's own success, before any gate ran: legacy rows only.
    PreGateSuccess,
    /// The gate verdict.
    GatePassed,
    /// S05's verified success: hidden tests plus integrity checks.
    Vs,
}

/// One attempt's outcome, as the self-model learns it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Label {
    /// The gate verdict: `forced_accept` is a fail, `unverified` is missing.
    pub y_gate: Option<bool>,
    /// S05's verified success, on audited attempts only.
    pub y_vs: Option<bool>,
    /// The label's importance weight: 1/π for an audit-only label (S05 DP5), else 1.
    pub weight: f64,
    /// Where the label came from.
    pub source: LabelSource,
}

/// One labelled attempt: the self-model's training and scoring unit (S04 §4.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Unit {
    /// The attempt's S01 key, the join key for forecasts and S05's labels.
    pub attempt_key: AttemptKey,
    /// The plan the task belongs to.
    pub plan_id: String,
    /// The task.
    pub task_id: String,
    /// The role dispatched.
    pub role: String,
    /// The task's difficulty tier.
    pub tier: String,
    /// The task family: the tier for plan tasks, `task.family` for benchmark rows.
    pub family: String,
    /// The arm that ran.
    pub arm: ArmKey,
    /// The 1-based attempt ordinal within the chain.
    pub attempt: u32,
    /// Whether an earlier attempt of the chain failed.
    pub prior_failure: bool,
    /// The previous attempt's failure class, when it failed.
    pub failure_class: Option<String>,
    /// The attempt's label.
    pub label: Label,
    /// Its tokens at the price snapshot's rates; `None` when unknown.
    pub api_equiv_usd: Option<f64>,
    /// Where its priced usage came from.
    pub cost_source: CostSource,
    /// Its wall time in seconds, when known.
    pub latency_s: Option<f64>,
    /// A provider failover substituted the model, so the unit is kept out of training.
    pub failover: bool,
}

/// The self-model's forecast for one candidate arm (S04 §5, `self_model.forecast`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandidateForecast {
    /// The arm.
    pub arm: ArmKey,
    /// P(every gate rung passes).
    pub p_gate: f64,
    /// P(a pass is a false green): P(VS = 0 | gates pass).
    pub p_fg: f64,
    /// P(verified success) = `p_gate`·(1 − `p_fg`): the routing target.
    pub p_vs: f64,
    /// A lower confidence bound on `p_vs`.
    pub p_vs_lcb: f64,
    /// The median attempt cost, in USD at the price snapshot.
    pub cost_q50: f64,
    /// The 90th-percentile attempt cost, in USD.
    pub cost_q90: f64,
    /// The median attempt latency, in seconds.
    pub lat_q50_s: f64,
    /// The 90th-percentile attempt latency, in seconds.
    pub lat_q90_s: f64,
    /// The forecast comes from L0 alone: the model has fewer outcomes than it needs (6116).
    pub cold_start: bool,
}

/// The version of a predictor: forecasts are never compared across versions (S04 §4.11.6).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PredictorVersion(
    /// The version string, `m3-<class>-<hash>`.
    pub String,
);

impl fmt::Display for PredictorVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A small deterministic generator for the self-model's synthetic tests (SplitMix64), so a
/// synthetic stream is the same on every machine.
#[cfg(test)]
pub(crate) struct TestRng(u64);

#[cfg(test)]
impl TestRng {
    pub(crate) const fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub(crate) fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in [0, 1).
    pub(crate) fn uniform(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1_u64 << 53) as f64
    }

    /// Standard normal (Box-Muller).
    pub(crate) fn normal(&mut self) -> f64 {
        let u1 = 1.0 - self.uniform();
        let u2 = self.uniform();
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }

    pub(crate) fn bernoulli(&mut self, p: f64) -> bool {
        self.uniform() < p
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arm_key_round_trips() {
        for key in [
            "roko/cerebras/gpt-oss-120b@default#V0",
            "roko/zai/glm-4.7@high#V4",
            "claude-code/anthropic/claude-sonnet-4-6@default",
            "mini-swe-agent/openrouter/openai/gpt-oss-120b@low",
        ] {
            let arm: ArmKey = key.parse().expect(key);
            assert_eq!(arm.to_string(), key);
            let json = serde_json::to_string(&arm).expect("serialise");
            assert_eq!(json, format!("\"{key}\""));
            let back: ArmKey = serde_json::from_str(&json).expect("deserialise");
            assert_eq!(back, arm);
        }
        let roko: ArmKey = "roko/zai/glm-4.7@high#V4".parse().expect("roko arm");
        assert_eq!(roko.depth, Some(4));
        assert_eq!(roko.effort, Effort::High);
        let direct: ArmKey = "mini-swe-agent/openrouter/openai/gpt-oss-120b@low"
            .parse()
            .expect("direct arm");
        assert_eq!(direct.depth, None);
        assert_eq!(direct.model, "openai/gpt-oss-120b");
        assert_eq!(
            ArmKey::roko("cerebras", "gpt-oss-120b").to_string(),
            "roko/cerebras/gpt-oss-120b@default#V0"
        );
    }

    #[test]
    fn arm_key_rejects_what_is_not_an_arm() {
        let error = |key: &str| key.parse::<ArmKey>().expect_err(key);
        for malformed in ["roko/cerebras@default", "roko/cerebras/m", "roko//m@low"] {
            assert_eq!(error(malformed), ArmKeyError::Malformed(malformed.into()));
        }
        assert_eq!(
            error("aider/p/m@default"),
            ArmKeyError::UnknownHarness("aider".into())
        );
        assert_eq!(
            error("roko/p/m@max#V0"),
            ArmKeyError::UnknownEffort("max".into())
        );
        assert_eq!(error("roko/p/m@low#V5"), ArmKeyError::BadDepth("V5".into()));
        assert_eq!(
            error("codex-cli/openai/gpt-5.5@high#V1"),
            ArmKeyError::DepthOnDirectArm("codex-cli".into())
        );
    }

    #[test]
    fn label_source_serde_names() {
        for (source, name) in [
            (LabelSource::PreGateSuccess, "\"pre_gate_success\""),
            (LabelSource::GatePassed, "\"gate_passed\""),
            (LabelSource::Vs, "\"vs\""),
        ] {
            assert_eq!(serde_json::to_string(&source).expect("serialise"), name);
            let back: LabelSource = serde_json::from_str(name).expect("deserialise");
            assert_eq!(back, source);
        }
    }
}
