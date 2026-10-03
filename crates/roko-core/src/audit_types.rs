//! Plain audit data shared across crates (S05 §4.1, §4.5, §4.6, §5): the
//! verify-depth scale, audit labels, strata, routing trust estimates and the
//! `vs.label` row. roko-gate's audit module, roko-learn (DP4, DP5) and
//! roko-cli read them without depending on each other.

use serde::{Deserialize, Serialize};

/// The verify-depth scale (S05 §4.6). Each level adds checks to the one
/// below it.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum VerifyDepth {
    /// The authored `[[task.verify]]` steps and the existing gate ladder.
    #[default]
    V0,
    /// Adds clippy and the A1 tamper and vacuity diff, inline.
    V1,
    /// Adds the A2 clean re-run with the crate's full tests.
    V2,
    /// Adds B1 hidden tests, inline.
    V3,
    /// Adds B2 mutation and the B3 cross-family review.
    V4,
}

impl VerifyDepth {
    /// One level deeper, at most [`Self::V4`].
    #[must_use]
    pub const fn deeper(self) -> Self {
        match self {
            Self::V0 => Self::V1,
            Self::V1 => Self::V2,
            Self::V2 => Self::V3,
            Self::V3 | Self::V4 => Self::V4,
        }
    }

    /// One level shallower, at least [`Self::V0`].
    #[must_use]
    pub const fn shallower(self) -> Self {
        match self {
            Self::V0 | Self::V1 => Self::V0,
            Self::V2 => Self::V1,
            Self::V3 => Self::V2,
            Self::V4 => Self::V3,
        }
    }
}

/// What an audit found about one green unit (S05 §4.1). `None` means the
/// check behind the label errored or timed out; a label is never imputed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditLabels {
    /// False green: the artifact fails its spec.
    pub y: Option<bool>,
    /// Spec gaming: the pass depended on manipulating an oracle.
    pub g: Option<bool>,
    /// Weak oracle: the tests do not pin the change.
    pub w: Option<bool>,
}

/// The stratum a green unit is drawn and reported in (S05 §4.1).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Stratum {
    /// The task's type.
    pub task_type: String,
    /// The implementing model.
    pub model: String,
    /// The S09 registry arm id, or `prod` outside experiments.
    pub arm: String,
    /// The green verdict: `passed`, `unverified`, …
    pub verdict: String,
}

/// Routing trust in a (model, harness) pair (S05 §4.5): a Beta posterior on
/// its false-green rate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrustEstimate {
    /// The model.
    pub model: String,
    /// The harness it ran in.
    pub harness: String,
    /// The posterior's α.
    pub alpha: f64,
    /// The posterior's β.
    pub beta: f64,
    /// Kish's effective sample size behind it.
    pub n_eff: f64,
}

impl TrustEstimate {
    /// Beta(1 + θ̂·n_eff, 19 + (1 − θ̂)·n_eff), whose prior mean is 5%
    /// (S05 §4.5).
    #[must_use]
    pub fn from_estimate(model: &str, harness: &str, theta: f64, n_eff: f64) -> Self {
        Self {
            model: model.to_string(),
            harness: harness.to_string(),
            alpha: theta.mul_add(n_eff, 1.0),
            beta: (1.0 - theta).mul_add(n_eff, 19.0),
            n_eff,
        }
    }

    /// The posterior mean false-green rate.
    #[must_use]
    pub fn mean(&self) -> f64 {
        self.alpha / (self.alpha + self.beta)
    }
}

/// Where a `vs.label` row's label came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VsSource {
    /// Every attempt is labelled (experiments), at π = 1.
    Census,
    /// An attempt the lottery selected (production), at its logged π.
    Audit,
}

/// The verified-success label of one attempt (S05 §0.1, §5): the row of
/// `benchmarks/viabilitybench/schema/vs-label.schema.json`, which S09 scores.
/// The join key is S01's `attempt_key`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VsLabel {
    /// Always [`VsLabel::EV`].
    pub ev: String,
    /// The run.
    pub run_id: String,
    /// S01's attempt key.
    pub attempt_key: String,
    /// The task.
    pub task_id: String,
    /// The experiment's seed; `None` outside an experiment.
    pub seed: Option<i64>,
    /// The S09 registry arm id, or `prod`.
    pub arm: String,
    /// The M3 forecast made before dispatch, if any.
    pub prediction_id: Option<String>,
    /// Census or audit.
    pub vs_source: VsSource,
    /// The probability the attempt was labelled: 1.0 in census mode.
    pub pi: f64,
    /// The green verdict that made the attempt a lottery unit.
    pub verdict: Option<String>,
    /// The final commit or result tree the checks judged.
    pub final_commit: Option<String>,
    /// (a) The arm declared the task done and left a final commit.
    pub completion: Option<bool>,
    /// (b) The visible checks pass on a clean re-run.
    pub visible_clean: Option<bool>,
    /// (c) The hidden suite on the clean worktree.
    pub hidden: Option<HiddenResult>,
    /// (d) No confirmed tamper or gaming finding.
    pub integrity: Integrity,
    /// (e) F8 tasks only.
    pub honeypot: Option<Honeypot>,
    /// Strict VS: 1 only when every check is known and passes.
    pub vs: u8,
    /// VS without (d) integrity.
    pub vs_lenient: u8,
    /// A check VS needs could not run.
    pub unknown: bool,
    /// The attempt's attributable spend.
    pub cost_usd: LabelCosts,
    /// The rule that derived `vs` from the checks.
    pub label_rule: String,
    /// A planted known false green, which never enters a P1 table.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gold: Option<bool>,
    /// The audit checks' own results, for the per-check confusion table.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub battery: Option<serde_json::Value>,
}

impl VsLabel {
    /// The `ev` of every row.
    pub const EV: &'static str = "vs.label";
    /// S05 §0.1's strict rule.
    pub const LABEL_RULE: &'static str = "vs-1";
}

/// The hidden suite's result on the clean worktree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HiddenResult {
    /// The suite and its version, e.g. `truth:F4@v1`.
    pub suite: String,
    /// Tests run.
    pub n: u32,
    /// Tests that failed.
    pub failed: u32,
    /// `false` when `n` is a lower bound (the census record kept only the
    /// failed checks).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub n_known: Option<bool>,
}

/// The integrity check (A1): `g` is 1 with at least one finding.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Integrity {
    /// 0, 1, or `None` when the check could not run.
    pub g: Option<u8>,
    /// What was found.
    pub findings: Vec<LabelFinding>,
}

/// One integrity finding: its kind, and whatever detail came with it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LabelFinding {
    /// The A1 finding kind, e.g. `test_edit`.
    pub kind: String,
    /// Any further fields, such as `path`.
    #[serde(flatten)]
    pub detail: serde_json::Map<String, serde_json::Value>,
}

/// The F8 honeypot rule's two parts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Honeypot {
    /// The spec-correct behaviour.
    pub spec_correct: Option<bool>,
    /// The conflict was flagged.
    pub conflict_flagged: Option<bool>,
}

/// Attributable spend in USD; an unknown cost is `None`, never 0.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LabelCosts {
    /// The implementer, retries included.
    #[serde(rename = "impl")]
    pub implementer: Option<f64>,
    /// Escalations.
    pub escalation: Option<f64>,
    /// Spec refinement (S07).
    pub spec_refine: Option<f64>,
    /// Audit LLM calls.
    pub audit: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_census_row_round_trips_and_the_depth_scale_saturates() {
        let row = serde_json::json!({
            "ev": "vs.label", "run_id": "run-1", "attempt_key": "run-1:plan:F4-l3-0017:1",
            "task_id": "F4-l3-0017", "seed": 2, "arm": "roko_full", "prediction_id": null,
            "vs_source": "census", "pi": 1.0, "verdict": "passed", "final_commit": "9c1e",
            "completion": true, "visible_clean": true,
            "hidden": {"suite": "truth:F4@v1", "n": 14, "failed": 0},
            "integrity": {"g": 1, "findings": [{"kind": "test_edit", "path": "tests/a.py"}]},
            "honeypot": null, "vs": 0, "vs_lenient": 1, "unknown": false,
            "cost_usd": {"impl": 0.041, "escalation": 0.0, "spec_refine": null, "audit": 0.004},
            "label_rule": "vs-1"
        });
        let label: VsLabel = serde_json::from_value(row.clone()).expect("a vs.label row");
        assert_eq!(label.vs_source, VsSource::Census);
        assert_eq!(label.integrity.findings[0].detail["path"], "tests/a.py");
        assert_eq!(serde_json::to_value(&label).expect("serialize"), row);
        let mut extra = row;
        extra["surprise"] = serde_json::json!(1);
        assert!(serde_json::from_value::<VsLabel>(extra).is_err(), "the row is closed");

        assert_eq!(VerifyDepth::V4.deeper(), VerifyDepth::V4);
        assert_eq!(VerifyDepth::V0.shallower(), VerifyDepth::V0);
        assert!(VerifyDepth::V1 < VerifyDepth::V3);
        let prior = TrustEstimate::from_estimate("m", "roko", 0.0, 0.0);
        assert!((prior.mean() - 0.05).abs() < 1e-12);
    }
}
