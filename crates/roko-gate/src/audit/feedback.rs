//! Feedback from closed windows (S05 §4.6): DP3's strictness ladder and
//! the routing trust estimates DP4 reads.
//!
//! - [`Ladder`] holds a verify depth per task type, on S05's V0–V4 scale,
//!   in the vault's `ladder.json`; it is the single writer of verify depth
//!   (R.S04-7). After a window it steps a task type up one level when θ or
//!   γ is above its limit (θ_max, γ_max) on the evidence: LCB95 above the
//!   limit at any n_eff, or UCB95 above it once n_eff reaches the limit's
//!   minimum, [`min_n_eff`] (73 for θ_max = 0.05, 189 for γ_max = 0.02).
//!   Below that minimum even a window without events can show UCB95 above
//!   the limit, so without evidence the level holds (q-394b7f). The ladder
//!   steps a task type down one only after two windows in a row with
//!   UCB95(θ) < θ_max/2, which at θ_max = 0.05 takes an n_eff near 150 or
//!   more. It moves at most one step per window, never below V0 or an
//!   active M1 floor request, and every step appends `audit.policy_change`.
//!   A window without a known label of a task type leaves its level and its
//!   run of quiet windows as they are.
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

use super::estimate::Z95;
use super::ledger::{AuditEvent, AuditLedger, records};
use super::window::{GroupEstimate, LabelEstimate, WindowClose, due_windows};

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

/// The least n_eff from which UCB95 alone may step the ladder past `max`.
///
/// It is ceil(z²·(1 − max)/max) with z = [`Z95`], the least whole n_eff at
/// which a window without events has UCB95 = z²/(n_eff + z²) at or below
/// `max`. Below it, even a window without events can show UCB95 above
/// `max`, so UCB95 there is sampling noise, not evidence (q-394b7f). 73 for
/// θ_max = 0.05 and 189 for γ_max = 0.02; infinite for a `max` of 0, where
/// only LCB95 steps.
#[must_use]
pub fn min_n_eff(max: f64) -> f64 {
    (Z95 * Z95 * (1.0 - max) / max).ceil()
}

/// What a window showed of one rate of a task type: the 95% interval at
/// Kish's n_eff.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reading {
    /// LCB95, the interval's lower end.
    pub lcb: f64,
    /// UCB95, the interval's upper end.
    pub ucb: f64,
    /// Kish's n_eff of the units whose label is known.
    pub n_eff: f64,
}

impl Reading {
    /// The reading of one label's estimate, `None` without a known label.
    #[must_use]
    pub fn of(label: &LabelEstimate) -> Option<Self> {
        label.ucb().map(|ucb| Self {
            lcb: label.estimate.ci.0,
            ucb,
            n_eff: label.estimate.n_eff,
        })
    }

    /// Why rate `name` steps the ladder up past its limit `max`, if it does.
    ///
    /// LCB95 above `max` is evidence at any n_eff; UCB95 above `max` counts
    /// only from [`min_n_eff`] on. Otherwise the level holds.
    fn steps_up(self, name: &str, max: f64) -> Option<String> {
        let Self { lcb, ucb, n_eff } = self;
        if lcb > max {
            return Some(format!(
                "LCB95({name}) {lcb:.4} > {name}_max {max} at n_eff {n_eff:.1}"
            ));
        }
        let least = min_n_eff(max);
        (n_eff >= least && ucb > max).then(|| {
            format!("UCB95({name}) {ucb:.4} > {name}_max {max} at n_eff {n_eff:.1} ≥ {least}")
        })
    }
}

/// What a window showed of one task type: θ and γ, each `None` without a
/// known label.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Signal {
    /// θ, the false-green rate.
    pub theta: Option<Reading>,
    /// γ, the spec-gaming rate.
    pub gamma: Option<Reading>,
}

impl Signal {
    /// The signal of a task type's estimate.
    #[must_use]
    pub fn of(estimate: &GroupEstimate) -> Self {
        Self {
            theta: Reading::of(&estimate.theta),
            gamma: Reading::of(&estimate.gamma),
        }
    }
}

/// One task type's place on the ladder.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
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
    let theta_up = signal
        .theta
        .and_then(|theta| theta.steps_up("θ", limits.theta_max));
    let up = theta_up.or_else(|| {
        signal
            .gamma
            .and_then(|gamma| gamma.steps_up("γ", limits.gamma_max))
    });
    if let Some(reason) = up {
        rung.quiet = 0;
        let to = rung.level.deeper();
        if to == rung.level {
            return None;
        }
        rung.level = to;
        return Some(reason);
    }
    match signal.theta {
        None => return None,
        Some(theta) if theta.ucb < limits.theta_max / 2.0 => {
            rung.quiet = rung.quiet.saturating_add(1);
        }
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
        let kept = std::mem::take(&mut self.estimates);
        for estimate in kept.into_iter().chain(fresh.iter().cloned()) {
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

/// Close every window due at `now` in `ledger` (S05 §4.5, §4.6); returns
/// what it closed.
///
/// `floor` is the verify depth an active M1 floor request holds, V0 without
/// one. Each window appends its strata's `audit.estimate`, steps the ladder,
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

    use super::super::estimate::wilson;
    use super::super::window::{Audited, WINDOW_PREFIX, WindowUnit, group_estimate};
    use super::*;

    const LIMITS: Limits = Limits {
        theta_max: 0.05,
        gamma_max: 0.02,
    };

    /// What `audited` units at π = 1, `events` of them labelled 1, show of
    /// a rate.
    fn reading(events: u32, audited: u32) -> Reading {
        let n_eff = f64::from(audited);
        let (lcb, ucb) = wilson(f64::from(events) / n_eff, n_eff, Z95).expect("an interval");
        Reading { lcb, ucb, n_eff }
    }

    const fn signal(theta: Reading, gamma: Reading) -> Signal {
        Signal {
            theta: Some(theta),
            gamma: Some(gamma),
        }
    }

    /// Task type `code`'s signal from a window at ρ = 0.10: `audited` units
    /// the lottery drew at π = 0.1 and audited, each with nine green units
    /// it passed over; `false_greens` of them are false greens and `gaming`
    /// of them spec gaming.
    fn window_signal(audited: u32, false_greens: u32, gaming: u32) -> Signal {
        let stratum = Stratum {
            task_type: "code".to_string(),
            model: "glm-4.7".to_string(),
            arm: "prod".to_string(),
            verdict: "passed".to_string(),
        };
        let units: Vec<WindowUnit> = (0..audited * 10)
            .map(|n: u32| {
                let (drawn, selected) = (n / 10, n.is_multiple_of(10));
                let labels = AuditLabels {
                    y: Some(drawn < false_greens),
                    g: Some(drawn < gaming),
                    w: None,
                };
                WindowUnit {
                    seq: u64::from(n) + 1,
                    at: Utc::now(),
                    stratum: stratum.clone(),
                    pi: 0.1,
                    selected,
                    audit: selected.then_some(Audited {
                        labels,
                        phase_b: Some(true),
                        pi_eff: Some(0.1),
                    }),
                }
            })
            .collect();
        let group: Vec<&WindowUnit> = units.iter().collect();
        Signal::of(&group_estimate(&group).expect("an estimate"))
    }

    /// The vault of a workspace under `root`, and its ledger.
    fn vault_in(root: &Path) -> (AuditVault, AuditLedger) {
        let workspace = root.join("repo");
        std::fs::create_dir_all(&workspace).expect("mkdir");
        let home = root.join("vault");
        let vault = AuditVault::resolve_with(&workspace, Some(&home), None).expect("a vault");
        let ledger = AuditLedger::open(&vault).expect("a ledger");
        (vault, ledger)
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

        // Windows of 200 audited units, past both minimum n_effs, where no
        // LCB95 is above its limit: 6 false greens (UCB95(θ) 0.064), one
        // gaming finding (UCB95(γ) 0.028), nothing (UCB95 0.019), and one
        // false green (UCB95(θ) 0.028, between θ_max/2 and θ_max).
        let loud = signal(reading(6, 200), reading(0, 200));
        let gaming = signal(reading(0, 200), reading(1, 200));
        let quiet = signal(reading(0, 200), reading(0, 200));
        let middling = signal(reading(1, 200), reading(0, 200));
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

    #[test]
    fn ladder_holds_below_the_minimum_effective_sample() {
        use VerifyDepth::{V0, V1};

        // Each limit's minimum n_eff is the least at which a window without
        // events shows a UCB95 at or below the limit.
        assert_eq!(min_n_eff(LIMITS.theta_max), 73.0);
        assert_eq!(min_n_eff(LIMITS.gamma_max), 189.0);
        assert!(reading(0, 72).ucb > 0.05 && reading(0, 73).ucb <= 0.05);
        assert!(reading(0, 188).ucb > 0.02 && reading(0, 189).ucb <= 0.02);

        // A default window audits about 20 of its 200 green units. With no
        // false green and no gaming, UCB95(θ) and UCB95(γ) are 0.161, above
        // both limits, but n_eff 20 is short of both minimums: the level
        // holds window after window.
        let clean = window_signal(20, 0, 0);
        let theta = clean.theta.expect("θ is known");
        assert!((theta.n_eff - 20.0).abs() < 1e-9, "{theta:?}");
        assert!(theta.ucb > LIMITS.theta_max && theta.lcb <= LIMITS.theta_max);
        let mut ladder = Ladder::default();
        for window in 1..=5 {
            assert!(apply(&mut ladder, window, clean, V0).is_empty(), "{window}");
        }
        // So does a little noise in 20: one or two false greens (LCB95(θ)
        // 0.009 and 0.028) or one gaming finding (LCB95(γ) 0.009).
        let noise = [
            window_signal(20, 1, 0),
            window_signal(20, 2, 0),
            window_signal(20, 0, 1),
        ];
        for (window, noisy) in (6..).zip(noise) {
            assert!(apply(&mut ladder, window, noisy, V0).is_empty(), "{window}");
        }
        // Past θ's minimum but short of γ's, UCB95(γ) 0.037 is noise too.
        assert!(apply(&mut ladder, 9, window_signal(100, 0, 0), V0).is_empty());
        // One false green holds at n_eff 72 and steps at 73 (UCB95(θ) 0.075
        // and 0.074, LCB95(θ) 0.002 each): the minimum is where UCB95 counts.
        assert!(apply(&mut ladder, 10, window_signal(72, 1, 0), V0).is_empty());
        assert_eq!(ladder.level("code"), V0);
        let signals = BTreeMap::from([("code".to_string(), window_signal(73, 1, 0))]);
        let steps = ladder.apply("window:11-11", &signals, LIMITS, V0);
        let reasons: Vec<&str> = steps.iter().map(|step| step.reason.as_str()).collect();
        assert_eq!(reasons, ["UCB95(θ) 0.0736 > θ_max 0.05 at n_eff 73.0 ≥ 73"]);
        assert_eq!(ladder.level("code"), V1);

        // Below the minimum a window is not quiet either: clean windows of
        // 20 never step a level down.
        for window in 12..=15 {
            assert!(apply(&mut ladder, window, clean, V0).is_empty(), "{window}");
        }
        assert_eq!(ladder.level("code"), V1);

        // On the audit worker's path: two windows of 20 clean audited units
        // close with their estimates and without a step.
        let temp = tempfile::tempdir().expect("tempdir");
        let (vault, mut ledger) = vault_in(temp.path());
        let config = AuditConfig {
            window_units: 20,
            ..AuditConfig::default()
        };
        for n in 1..=40 {
            audited(&mut ledger, n, "glm-4.7", false);
        }
        let closed = close_due_windows(&mut ledger, &vault, &config, V0, Utc::now())
            .expect("the windows close");
        assert_eq!(closed.len(), 2);
        for close in &closed {
            let theta = Signal::of(&close.task_types["code"]).theta;
            let theta = theta.expect("θ is known");
            assert!((theta.n_eff - 20.0).abs() < 1e-9, "{theta:?}");
            assert!(theta.ucb > config.theta_max, "{theta:?}");
        }
        let ladder = Ladder::load(&ladder_path(&vault)).expect("the ladder");
        assert_eq!(ladder.level("code"), V0);
        let all = records(ledger.dir()).expect("records");
        let steps = all
            .iter()
            .filter(|record| matches!(record.event, AuditEvent::PolicyChange { .. }));
        assert_eq!(steps.count(), 0);
    }

    #[test]
    fn ladder_steps_up_below_the_floor_on_clear_evidence() {
        use VerifyDepth::{V0, V1, V2, V3};

        // 8 false greens in 20 audited units: n_eff 20 is short of θ's
        // minimum, but LCB95(θ) 0.219 is above θ_max.
        let eight = window_signal(20, 8, 0);
        let theta = eight.theta.expect("θ is known");
        assert!(theta.n_eff < min_n_eff(LIMITS.theta_max), "{theta:?}");
        let mut ladder = Ladder::default();
        let signals = BTreeMap::from([("code".to_string(), eight)]);
        let steps = ladder.apply("window:1-1", &signals, LIMITS, V0);
        assert_eq!(steps.len(), 1, "{steps:?}");
        assert_eq!((steps[0].from, steps[0].to), (V0, V1));
        assert_eq!(
            steps[0].reason,
            "LCB95(θ) 0.2188 > θ_max 0.05 at n_eff 20.0"
        );
        // Still one step per window.
        assert!(ladder.apply("window:1-1", &signals, LIMITS, V0).is_empty());

        // The least evidence that steps in 20 audited units: three false
        // greens (LCB95(θ) 0.052), or two gaming findings (LCB95(γ) 0.028).
        assert_eq!(
            apply(&mut ladder, 2, window_signal(20, 3, 0), V0),
            [(V1, V2)]
        );
        assert_eq!(
            apply(&mut ladder, 3, window_signal(20, 0, 2), V0),
            [(V2, V3)]
        );
        assert_eq!(ladder.level("code"), V3);
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
                risk_r: None,
                prediction_id: None,
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
        let (vault, mut ledger) = vault_in(temp.path());
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
                    window, stratum, ..
                } if *window == id => Some(stratum),
                _ => None,
            })
            .collect();
        assert_eq!(estimates.len(), 3, "two strata and the summary");
        assert!(estimates[2].is_none(), "the summary comes last");
        // Two false greens in three audited units are evidence at n_eff 3:
        // LCB95(θ) 0.208 > θ_max.
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
