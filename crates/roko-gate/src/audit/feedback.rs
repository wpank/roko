//! Feedback from closed windows (S05 §4.6): DP3's strictness ladder and
//! the routing trust estimates DP4 reads.
//!
//! - [`Ladder`] holds a verify depth per task type, on S05's V0–V4 scale,
//!   in the vault's `ladder.json`; it is the single writer of verify depth
//!   (R.S04-7). After a window it steps a task type up one level when
//!   UCB95(θ) > θ_max or UCB95(γ) > γ_max, and down one only after two
//!   windows in a row with UCB95(θ) < θ_max/2. It moves at most one step
//!   per window, never below V0 or an active M1 floor request, and every
//!   step appends `audit.policy_change`. A window without a known label of
//!   a task type leaves its level and its run of quiet windows as they are.
//! - [`TrustBook`] holds the latest Beta posterior on each (model, harness)
//!   pair's false-green rate, Beta(1 + θ̂·n_eff, 19 + (1 − θ̂)·n_eff), in the
//!   vault's `trust.json`. The harness reads it at plan start; agents never
//!   see the vault.
//! - [`close_due_windows`], which the audit worker calls, closes every due
//!   window ([`super::window`]): each stratum's `audit.estimate`, the
//!   ladder's steps, the trust estimates, and last the window's summary,
//!   the `audit.estimate` without a stratum that marks it closed.

use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use roko_core::audit_home::AuditVault;
use roko_core::audit_types::{TrustEstimate, VerifyDepth};
use roko_core::config::audit::AuditConfig;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::ledger::{AuditEvent, AuditLedger, records};
use super::window::{GroupEstimate, WindowClose, due_windows};

/// The ladder's file in the workspace's vault directory.
pub const LADDER_FILE: &str = "ladder.json";
/// The trust estimates' file in the workspace's vault directory.
pub const TRUST_FILE: &str = "trust.json";
/// The lock window close holds, in the workspace's vault directory.
pub const FEEDBACK_LOCK: &str = ".feedback.lock";
/// The knob of a ladder step in `audit.policy_change`, before the task
/// type.
pub const DEPTH_KNOB: &str = "verify_depth:";
/// Quiet windows in a row before the ladder steps down.
pub const QUIET_WINDOWS: u32 = 2;

/// The rates the ladder holds below (`[audit] theta_max`, `gamma_max`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Limits {
    /// θ_max, the false-green rate.
    pub theta_max: f64,
    /// γ_max, the spec-gaming rate.
    pub gamma_max: f64,
}

impl Limits {
    /// `config`'s limits.
    #[must_use]
    pub const fn of(config: &AuditConfig) -> Self {
        Self {
            theta_max: config.theta_max,
            gamma_max: config.gamma_max,
        }
    }
}

/// What a window showed of one task type: the UCB95 of θ and γ, `None`
/// without a known label.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Signal {
    /// UCB95(θ).
    pub theta_ucb: Option<f64>,
    /// UCB95(γ).
    pub gamma_ucb: Option<f64>,
}

impl Signal {
    /// The signal of a task type's estimate.
    #[must_use]
    pub fn of(estimate: &GroupEstimate) -> Self {
        Self {
            theta_ucb: estimate.theta.ucb(),
            gamma_ucb: estimate.gamma.ucb(),
        }
    }
}

/// One task type's place on the ladder.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Rung {
    /// Its verify depth.
    pub level: VerifyDepth,
    /// Quiet windows in a row, with UCB95(θ) below θ_max/2.
    pub quiet: u32,
    /// The last window applied to it.
    pub window: Option<String>,
}

/// One step of the ladder.
#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    /// The task type.
    pub task_type: String,
    /// Its level before.
    pub from: VerifyDepth,
    /// Its level after.
    pub to: VerifyDepth,
    /// Why, as `audit.policy_change` records it.
    pub reason: String,
}

impl Step {
    /// The step as `audit.policy_change`, from `window`.
    #[must_use]
    pub fn event(&self, window: &str) -> AuditEvent {
        AuditEvent::PolicyChange {
            knob: format!("{DEPTH_KNOB}{}", self.task_type),
            from: json!(self.from),
            to: json!(self.to),
            reason: format!("{window}: {}", self.reason),
        }
    }
}

/// DP3's strictness ladder: a verify depth per task type (S05 §4.6).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Ladder {
    /// Each task type's rung.
    pub task_types: BTreeMap<String, Rung>,
}

impl Ladder {
    /// The ladder kept at `path`; an empty one before its first step.
    ///
    /// # Errors
    ///
    /// The file cannot be read or does not parse.
    pub fn load(path: &Path) -> std::io::Result<Self> {
        read_json(path)
    }

    /// Keep the ladder at `path`.
    ///
    /// # Errors
    ///
    /// The file cannot be written.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        write_json(path, self)
    }

    /// The verify depth of `task_type`: V0 until the ladder steps it.
    #[must_use]
    pub fn level(&self, task_type: &str) -> VerifyDepth {
        self.task_types
            .get(task_type)
            .map_or(VerifyDepth::V0, |rung| rung.level)
    }

    /// Apply window `window`'s signals, at most one step per task type and
    /// none for a window already applied, with `floor` the lowest level a
    /// step down may reach. Returns the steps, in task-type order.
    pub fn apply(
        &mut self,
        window: &str,
        signals: &BTreeMap<String, Signal>,
        limits: Limits,
        floor: VerifyDepth,
    ) -> Vec<Step> {
        let mut steps = Vec::new();
        for (task_type, signal) in signals {
            let rung = self.task_types.entry(task_type.clone()).or_default();
            if rung.window.as_deref() == Some(window) {
                continue;
            }
            rung.window = Some(window.to_string());
            let from = rung.level;
            if let Some(reason) = step(rung, *signal, limits, floor) {
                steps.push(Step {
                    task_type: task_type.clone(),
                    from,
                    to: rung.level,
                    reason,
                });
            }
        }
        steps
    }
}

/// Move `rung` by one window's `signal`; the reason when its level moved.
fn step(rung: &mut Rung, signal: Signal, limits: Limits, floor: VerifyDepth) -> Option<String> {
    let theta_up = signal.theta_ucb.filter(|ucb| *ucb > limits.theta_max);
    let gamma_up = signal.gamma_ucb.filter(|ucb| *ucb > limits.gamma_max);
    if theta_up.is_some() || gamma_up.is_some() {
        rung.quiet = 0;
        let to = rung.level.deeper();
        if to == rung.level {
            return None;
        }
        rung.level = to;
        return Some(match theta_up {
            Some(ucb) => format!("UCB95(θ) {ucb:.4} > θ_max {}", limits.theta_max),
            None => format!(
                "UCB95(γ) {:.4} > γ_max {}",
                gamma_up.unwrap_or_default(),
                limits.gamma_max
            ),
        });
    }
    match signal.theta_ucb {
        None => return None,
        Some(ucb) if ucb < limits.theta_max / 2.0 => rung.quiet = rung.quiet.saturating_add(1),
        Some(_) => rung.quiet = 0,
    }
    if rung.quiet < QUIET_WINDOWS || rung.level <= floor {
        return None;
    }
    rung.quiet = 0;
    rung.level = rung.level.shallower().max(floor);
    Some(format!(
        "UCB95(θ) below θ_max/2 ({}) in {QUIET_WINDOWS} windows in a row",
        limits.theta_max / 2.0
    ))
}

/// The latest routing trust estimate of each (model, harness) pair
/// (S05 §4.5), which DP4 reads.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TrustBook {
    /// The window that last updated it.
    pub window: Option<String>,
    /// One estimate per pair, ordered by model and harness.
    pub estimates: Vec<TrustEstimate>,
}

impl TrustBook {
    /// The book kept at `path`; an empty one before the first window.
    ///
    /// # Errors
    ///
    /// The file cannot be read or does not parse.
    pub fn load(path: &Path) -> std::io::Result<Self> {
        read_json(path)
    }

    /// Keep the book at `path`.
    ///
    /// # Errors
    ///
    /// The file cannot be written.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        write_json(path, self)
    }

    /// Replace each pair's estimate with its `fresh` one from `window`.
    pub fn update(&mut self, window: &str, fresh: &[TrustEstimate]) {
        self.merge(fresh);
        self.window = Some(window.to_string());
    }

    /// DP6's trust downgrade (S05 §4.6): one more confirmed false green or
    /// gaming finding against (`model`, `harness`), on its estimate or on
    /// the prior Beta(1, 19). Returns the posterior mean before and after;
    /// the next window's estimate replaces it.
    pub fn downgrade(&mut self, model: &str, harness: &str) -> (f64, f64) {
        let kept = self
            .estimates
            .iter()
            .find(|estimate| estimate.model == model && estimate.harness == harness)
            .cloned();
        let mut estimate =
            kept.unwrap_or_else(|| TrustEstimate::from_estimate(model, harness, 0.0, 0.0));
        let before = estimate.mean();
        estimate.alpha += 1.0;
        estimate.n_eff += 1.0;
        let after = estimate.mean();
        self.merge(&[estimate]);
        (before, after)
    }

    /// Replace each pair's estimate with its one in `fresh`, keeping the
    /// book ordered by model and harness.
    fn merge(&mut self, fresh: &[TrustEstimate]) {
        let mut pairs = BTreeMap::new();
        for estimate in self.estimates.drain(..).chain(fresh.iter().cloned()) {
            let pair = (estimate.model.clone(), estimate.harness.clone());
            pairs.insert(pair, estimate);
        }
        self.estimates = pairs.into_values().collect();
    }
}

/// `vault`'s ladder file.
#[must_use]
pub fn ladder_path(vault: &AuditVault) -> PathBuf {
    vault.dir().join(LADDER_FILE)
}

/// `vault`'s trust file.
#[must_use]
pub fn trust_path(vault: &AuditVault) -> PathBuf {
    vault.dir().join(TRUST_FILE)
}

/// Close every window due at `now` in `ledger` (S05 §4.5, §4.6), with
/// `floor` the verify depth an active M1 floor request holds (V0 without
/// one); returns what it closed.
///
/// Each window appends its strata's `audit.estimate`, steps the ladder,
/// updates the trust book and appends its summary last, so a window cut
/// short is closed again whole; the ladder never applies one twice. One
/// process closes windows at a time.
///
/// # Errors
///
/// The lock, the ledger, the ladder or the trust book cannot be read or
/// written, or an estimate fails.
pub fn close_due_windows(
    ledger: &mut AuditLedger,
    vault: &AuditVault,
    config: &AuditConfig,
    floor: VerifyDepth,
    now: DateTime<Utc>,
) -> std::io::Result<Vec<WindowClose>> {
    let lock = OpenOptions::new()
        .create(true)
        .append(true)
        .open(vault.dir().join(FEEDBACK_LOCK))?;
    lock.lock()?;
    let closed = close_locked(ledger, vault, config, floor, now);
    lock.unlock()?;
    closed
}

fn close_locked(
    ledger: &mut AuditLedger,
    vault: &AuditVault,
    config: &AuditConfig,
    floor: VerifyDepth,
    now: DateTime<Utc>,
) -> std::io::Result<Vec<WindowClose>> {
    let all = records(ledger.dir())?;
    let mut closed = Vec::new();
    for window in due_windows(&all, config, now) {
        let close = window.close().map_err(std::io::Error::other)?;
        for (stratum, estimate) in &close.strata {
            ledger.append(AuditEvent::Estimate {
                window: close.id.clone(),
                stratum: Some(stratum.clone()),
                estimate: serde_json::to_value(estimate)?,
            })?;
        }
        let signals: BTreeMap<String, Signal> = close
            .task_types
            .iter()
            .map(|(task_type, estimate)| (task_type.clone(), Signal::of(estimate)))
            .collect();
        let mut ladder = Ladder::load(&ladder_path(vault))?;
        for step in ladder.apply(&close.id, &signals, Limits::of(config), floor) {
            ledger.append(step.event(&close.id))?;
        }
        ladder.save(&ladder_path(vault))?;
        let mut trust = TrustBook::load(&trust_path(vault))?;
        trust.update(&close.id, &close.trust);
        trust.save(&trust_path(vault))?;
        ledger.append(AuditEvent::Estimate {
            window: close.id.clone(),
            stratum: None,
            estimate: json!({
                "opened_at": close.opened_at,
                "last_at": close.last_at,
                "overall": close.overall,
                "task_types": close.task_types,
            }),
        })?;
        closed.push(close);
    }
    Ok(closed)
}

/// The JSON at `path`, or the default when there is no file.
fn read_json<T: DeserializeOwned + Default>(path: &Path) -> std::io::Result<T> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(serde_json::from_str(&text)?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(T::default()),
        Err(error) => Err(error),
    }
}

/// Write `value` to `path` whole: a sibling file renamed over it.
fn write_json<T: Serialize>(path: &Path, value: &T) -> std::io::Result<()> {
    let partial = path.with_extension("json.partial");
    std::fs::write(&partial, serde_json::to_string_pretty(value)?)?;
    std::fs::rename(&partial, path)
}

#[cfg(test)]
mod tests {
    use roko_core::audit_types::{AuditLabels, Stratum};
    use serde_json::Value;

    use super::super::window::WINDOW_PREFIX;
    use super::*;

    const LIMITS: Limits = Limits {
        theta_max: 0.05,
        gamma_max: 0.02,
    };

    const fn signal(theta_ucb: f64, gamma_ucb: f64) -> Signal {
        Signal {
            theta_ucb: Some(theta_ucb),
            gamma_ucb: Some(gamma_ucb),
        }
    }

    /// Apply `signal` to task type `code` in window `window`; the levels
    /// it stepped from and to.
    fn apply(
        ladder: &mut Ladder,
        window: u32,
        signal: Signal,
        floor: VerifyDepth,
    ) -> Vec<(VerifyDepth, VerifyDepth)> {
        let signals = BTreeMap::from([("code".to_string(), signal)]);
        let id = format!("{WINDOW_PREFIX}{window}-{window}");
        ladder
            .apply(&id, &signals, LIMITS, floor)
            .into_iter()
            .map(|step| (step.from, step.to))
            .collect()
    }

    #[test]
    fn ladder_steps_up_on_ucb_and_down_after_two_quiet_windows() {
        use VerifyDepth::{V0, V1, V2, V4};

        let loud = signal(0.20, 0.0);
        let gaming = signal(0.01, 0.05);
        let quiet = signal(0.01, 0.0);
        let middling = signal(0.03, 0.0);
        let mut ladder = Ladder::default();

        // Up on UCB95(θ) > θ_max, and at most one step per window.
        assert_eq!(apply(&mut ladder, 1, loud, V0), [(V0, V1)]);
        assert!(apply(&mut ladder, 1, loud, V0).is_empty(), "window 1 again");
        // Up on UCB95(γ) > γ_max alone.
        assert_eq!(apply(&mut ladder, 2, gaming, V0), [(V1, V2)]);

        // Down only after two quiet windows in a row: a window between
        // θ_max/2 and θ_max breaks the run.
        assert!(apply(&mut ladder, 3, quiet, V0).is_empty());
        assert!(apply(&mut ladder, 4, middling, V0).is_empty());
        assert!(apply(&mut ladder, 5, quiet, V0).is_empty());
        assert_eq!(apply(&mut ladder, 6, quiet, V0), [(V2, V1)]);
        // A window without a known label leaves the run as it is.
        let unknown = Signal::default();
        assert!(apply(&mut ladder, 7, quiet, V0).is_empty());
        assert!(apply(&mut ladder, 8, unknown, V0).is_empty());
        assert_eq!(apply(&mut ladder, 9, quiet, V0), [(V1, V0)]);

        // Never below V0.
        assert!(apply(&mut ladder, 10, quiet, V0).is_empty());
        assert!(apply(&mut ladder, 11, quiet, V0).is_empty());
        assert_eq!(ladder.level("code"), V0);

        // Never below an active M1 floor request: at V1 with a V1 floor,
        // quiet windows hold it, and it steps down once the floor goes.
        assert_eq!(apply(&mut ladder, 12, loud, V1), [(V0, V1)]);
        assert!(apply(&mut ladder, 13, quiet, V1).is_empty());
        assert!(apply(&mut ladder, 14, quiet, V1).is_empty());
        assert_eq!(ladder.level("code"), V1);
        assert_eq!(apply(&mut ladder, 15, quiet, V0), [(V1, V0)]);

        // V4 is the top.
        for window in 16..20 {
            apply(&mut ladder, window, loud, V0);
        }
        assert_eq!(ladder.level("code"), V4);
        assert!(apply(&mut ladder, 20, loud, V0).is_empty());

        // The ladder keeps its state, and a step names its window.
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join(LADDER_FILE);
        ladder.save(&path).expect("save");
        assert_eq!(Ladder::load(&path).expect("load"), ladder);
        let step = Step {
            task_type: "code".to_string(),
            from: V0,
            to: V1,
            reason: "UCB95(θ) 0.2 > θ_max 0.05".to_string(),
        };
        let AuditEvent::PolicyChange {
            knob,
            from,
            to,
            reason,
        } = step.event("window:1-9")
        else {
            panic!("a policy change");
        };
        assert_eq!(knob, "verify_depth:code");
        assert_eq!((from, to), (json!("V0"), json!("V1")));
        assert!(reason.starts_with("window:1-9: "), "{reason}");
    }

    /// A selected unit of task type `code` by `model` at π = 1, and its
    /// audit's labels, phase B drawn.
    fn audited(ledger: &mut AuditLedger, n: u32, model: &str, y: bool) {
        let attempt_key = format!("run:plan:t{n}:1");
        ledger
            .append(AuditEvent::Selection {
                sel_id: format!("sel-{n}"),
                attempt_key: attempt_key.clone(),
                run_id: "run".to_string(),
                task_id: format!("t{n}"),
                stratum: Stratum {
                    task_type: "code".to_string(),
                    model: model.to_string(),
                    arm: "prod".to_string(),
                    verdict: "passed".to_string(),
                },
                pi: 1.0,
                prf_u: "0x0000000000000000".to_string(),
                selected: true,
                base_tree: Some("base".to_string()),
                result_tree: Some("result".to_string()),
            })
            .expect("a selection");
        ledger
            .append(AuditEvent::Result {
                sel_id: format!("sel-{n}"),
                res_id: format!("res-{n}"),
                attempt_key,
                labels: AuditLabels {
                    y: Some(y),
                    g: Some(false),
                    w: None,
                },
                findings: Vec::new(),
                checks: json!({ "phase_b": { "pi_b": 1.0, "drawn": true } }),
                cost_usd: Some(0.0),
                cpu_secs: Some(1.0),
                pi_eff: Some(1.0),
            })
            .expect("a result");
    }

    #[test]
    fn window_close_logs_estimates_steps_the_ladder_and_publishes_trust() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = temp.path().join("repo");
        std::fs::create_dir_all(&workspace).expect("mkdir");
        let home = temp.path().join("vault");
        let vault = AuditVault::resolve_with(&workspace, Some(&home), None).expect("a vault");
        let mut ledger = AuditLedger::open(&vault).expect("a ledger");
        let config = AuditConfig {
            window_units: 3,
            ..AuditConfig::default()
        };
        audited(&mut ledger, 1, "glm-4.7", true);
        audited(&mut ledger, 2, "glm-4.7", true);
        let now = Utc::now();
        let none = close_due_windows(&mut ledger, &vault, &config, VerifyDepth::V0, now);
        assert!(none.expect("no window is due yet").is_empty());

        audited(&mut ledger, 3, "kimi-k2", false);
        let closed = close_due_windows(&mut ledger, &vault, &config, VerifyDepth::V0, now)
            .expect("the window closes");
        assert_eq!(closed.len(), 1);
        let id = closed[0].id.clone();
        assert!(id.starts_with(WINDOW_PREFIX), "{id}");

        let all = records(ledger.dir()).expect("records");
        let estimates: Vec<&Option<Stratum>> = all
            .iter()
            .filter_map(|record| match &record.event {
                AuditEvent::Estimate {
                    window,
                    stratum,
                    ..
                } if *window == id => Some(stratum),
                _ => None,
            })
            .collect();
        assert_eq!(estimates.len(), 3, "two strata and the summary");
        assert!(estimates[2].is_none(), "the summary comes last");
        let steps: Vec<(&str, &Value)> = all
            .iter()
            .filter_map(|record| match &record.event {
                AuditEvent::PolicyChange { knob, to, .. } => Some((knob.as_str(), to)),
                _ => None,
            })
            .collect();
        assert_eq!(steps, [("verify_depth:code", &json!("V1"))]);
        let ladder = Ladder::load(&ladder_path(&vault)).expect("the ladder");
        assert_eq!(ladder.level("code"), VerifyDepth::V1);

        let trust = TrustBook::load(&trust_path(&vault)).expect("the trust book");
        assert_eq!(trust.window.as_deref(), Some(id.as_str()));
        let pairs: Vec<(&str, &str)> = trust
            .estimates
            .iter()
            .map(|estimate| (estimate.model.as_str(), estimate.harness.as_str()))
            .collect();
        assert_eq!(pairs, [("glm-4.7", "roko"), ("kimi-k2", "roko")]);
        assert!(trust.estimates[0].mean() > trust.estimates[1].mean());

        // Closed, the window is not closed again.
        let again = close_due_windows(&mut ledger, &vault, &config, VerifyDepth::V0, now);
        assert!(again.expect("nothing is due").is_empty());
    }
}
