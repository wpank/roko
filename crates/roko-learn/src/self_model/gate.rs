//! The promotion gate that moves the self-model out of shadow mode: backlog task 6122.
//!
//! The self-model is "promoted only when calibrated" (P18). [`CalibrationGate::evaluate`] scores
//! the last W labelled outcomes of the current predictor version against S04 §4.7's bounds on
//! its essential variables: ECE ≤ 0.08, |calibration-in-the-large| ≤ 0.05, BSS ≥ 0.05 and
//! AUROC ≥ 0.65, and, once it has routed one, the pass rate of the attempts it routed
//! ≥ π* − 0.05: it routes only while the gate is eligible (bug-92cc56). W is
//! `[self_model] n_min`, 100 by default (S04 §4.10), so a small workspace can opt into a shorter
//! window ([`CalibrationGate::of`]; gap-4c531f). The bounds stay as they are, and over a small W
//! they are noisy: with ten equal-mass bins, ECE over fewer than about 100 outcomes compares
//! single forecasts with their own 0/1 outcome, so it passes only sharp forecasts, and one
//! outcome moves every score. Per decision 6102 it calibrates on gate labels now and VS labels
//! later, and the report only makes the model eligible: a person promotes it. The circuit
//! breaker (S04 §4.11.5) trips when the Brier score over the last 50 outcomes is worse than the
//! base rate's, or ECE exceeds 0.2, and an active model then drops back to shadow
//! ([`next_mode`]). The breaker keeps its 50 outcomes whatever W is: the window keeps at least
//! [`WINDOW`] outcomes, so below a W of 50 the gate may hold before the breaker can trip. A new
//! predictor version restarts the window (§4.11.6). No single run settles W outcomes, so the
//! window is kept across runs in [`WINDOW_FILE_NAME`] beside the state file
//! ([`CalibrationWindow::load`], [`CalibrationWindow::save_outcomes`]; gap-b38ed2).
//! [`write_calibration`] exports the `ev.m3.*` values S06 reads, atomically.
//!
//! Demotion for "no benefit after 150 exposed decisions" is S03's L-M3 loop audit, not this gate.

use std::collections::VecDeque;
use std::io;
use std::path::{Path, PathBuf};

use roko_core::config::self_model::SelfModelConfig;
use serde::{Deserialize, Serialize};

use super::PredictorVersion;
use super::metrics::{
    ECE_BINS, Scored, auroc, base_rate, brier, brier_skill, calibration_in_the_large, ece,
};
use super::policy::TARGET;

/// The outcomes the gate scores by default, W (S04 §4.10); `[self_model] n_min` sets W
/// (gap-4c531f). A window keeps at least this many outcomes, whatever W is.
pub const WINDOW: usize = 100;
/// The ECE bound.
pub const ECE_MAX: f64 = 0.08;
/// The bound on |calibration-in-the-large|.
pub const CAL_IN_LARGE_MAX: f64 = 0.05;
/// The Brier skill floor.
pub const BSS_MIN: f64 = 0.05;
/// The AUROC floor.
pub const AUROC_MIN: f64 = 0.65;
/// How far the routed pass rate may fall below π*.
pub const ROUTE_PASS_MARGIN: f64 = 0.05;
/// The outcomes over which the breaker compares the Brier score with the base rate's.
pub const BREAKER_WINDOW: usize = 50;
/// The ECE that trips the breaker.
pub const BREAKER_ECE: f64 = 0.2;
/// The `ev.m3.*` export, under `.roko/`.
pub const CALIBRATION_FILE: &str = "learn/self-model/calibration.json";
/// `schema_version` of the export.
pub const CALIBRATION_SCHEMA: &str = "roko.self_model_calibration/1";
/// The window's file, in the directory of the self-model's state file (gap-b38ed2).
pub const WINDOW_FILE_NAME: &str = "window-v1.json";
/// `schema_version` of the window's file.
pub const WINDOW_SCHEMA: &str = "roko.self_model_window/1";

/// One labelled outcome in the gate's window.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WindowOutcome {
    /// The forecast probability of the label.
    pub p: f64,
    /// The label.
    pub y: bool,
    /// Its importance weight.
    pub w: f64,
    /// The self-model chose the attempt's arm, so it counts toward the routed pass rate.
    pub routed: bool,
}

/// The last outcomes of one predictor version: at most its [capacity](Self::capacity), which is
/// at least [`WINDOW`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalibrationWindow {
    /// The version whose forecasts the window holds.
    pub version: PredictorVersion,
    outcomes: VecDeque<WindowOutcome>,
    /// The most outcomes it keeps: the gate's W, never fewer than [`WINDOW`], so that the
    /// breaker always has its [`BREAKER_WINDOW`] (gap-4c531f). The run sets it
    /// ([`Self::keeping`]); the window's file does not hold it.
    #[serde(skip, default = "default_window")]
    capacity: usize,
}

/// The default W, [`WINDOW`]: a window's capacity and a gate's window when nothing sets them.
const fn default_window() -> usize {
    WINDOW
}

impl CalibrationWindow {
    /// An empty window for `version`, keeping the last [`WINDOW`] outcomes.
    #[must_use]
    pub fn new(version: PredictorVersion) -> Self {
        Self {
            version,
            outcomes: VecDeque::with_capacity(WINDOW + 1),
            capacity: WINDOW,
        }
    }

    /// The window keeping its last `capacity` outcomes, and never fewer than [`WINDOW`]: what
    /// a gate of that W needs ([`CalibrationGate::keeps`]). Older outcomes go.
    #[must_use]
    pub fn keeping(mut self, capacity: usize) -> Self {
        self.capacity = capacity.max(WINDOW);
        self.trim();
        self
    }

    /// The most outcomes the window keeps.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// Let the oldest outcomes go past the capacity.
    fn trim(&mut self) {
        let excess = self.outcomes.len().saturating_sub(self.capacity);
        self.outcomes.drain(..excess);
    }

    /// Add an outcome of a forecast by `version`. A new version restarts the window, since
    /// forecasts are never compared across versions.
    pub fn push(&mut self, version: &PredictorVersion, outcome: WindowOutcome) {
        if *version != self.version {
            self.version = version.clone();
            self.outcomes.clear();
        }
        self.outcomes.push_back(outcome);
        self.trim();
    }

    /// The outcomes held.
    #[must_use]
    pub fn len(&self) -> usize {
        self.outcomes.len()
    }

    /// Whether the window holds nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.outcomes.is_empty()
    }

    /// The last `n` outcomes as scored forecasts.
    fn scored(&self, n: usize) -> Vec<Scored> {
        let skip = self.outcomes.len().saturating_sub(n);
        self.outcomes
            .iter()
            .skip(skip)
            .map(|outcome| Scored::weighted(outcome.p, outcome.y, outcome.w))
            .collect()
    }

    /// The weighted pass rate of the attempts the self-model routed among the last `n`.
    fn route_pass(&self, n: usize) -> Option<f64> {
        let skip = self.outcomes.len().saturating_sub(n);
        let routed: Vec<Scored> = self
            .outcomes
            .iter()
            .skip(skip)
            .filter(|outcome| outcome.routed)
            .map(|outcome| Scored::weighted(outcome.p, outcome.y, outcome.w))
            .collect();
        base_rate(&routed)
    }

    /// The window kept in the file at `path` when it holds forecasts of `version`, else an empty
    /// window of `version`, either way keeping the last `capacity` outcomes ([`Self::keeping`]):
    /// a missing file, a window of another version (forecasts are never compared across
    /// versions) and a file of another schema start afresh. A file that does not parse is set
    /// aside, renamed `<name>.unreadable`, as an unreadable state file is, so the next save
    /// starts a new one.
    ///
    /// # Errors
    ///
    /// Returns an error when the file exists but cannot be read or set aside.
    pub fn load(path: &Path, version: &PredictorVersion, capacity: usize) -> io::Result<Self> {
        let window = match roko_fs::read_json_or_default_strict::<WindowFile>(path) {
            Ok(file) => {
                let current = file.schema_version == WINDOW_SCHEMA;
                let kept = file
                    .window
                    .filter(|window| current && window.version == *version);
                kept.unwrap_or_else(|| Self::new(version.clone()))
            }
            Err(error) if error.kind() == io::ErrorKind::InvalidData => {
                std::fs::rename(path, path.with_extension("json.unreadable"))?;
                Self::new(version.clone())
            }
            Err(error) => return Err(error),
        };
        Ok(window.keeping(capacity))
    }

    /// Add `outcomes`, each with the version that forecast it, to the window kept in the file at
    /// `path`, in one transaction under the file's sibling lock, so that a concurrent run's
    /// outcomes survive, and return the window as saved, keeping its last `capacity` outcomes
    /// ([`Self::keeping`]): `None` when the file holds none. Each outcome is
    /// [pushed](Self::push), so one of a new version restarts the window. With no outcomes the
    /// file is only read.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be read, parsed or written.
    pub fn save_outcomes(
        path: &Path,
        capacity: usize,
        outcomes: &[(PredictorVersion, WindowOutcome)],
    ) -> io::Result<Option<Self>> {
        roko_fs::with_locked_json_transaction::<WindowFile, _, io::Error, _>(path, |file| {
            if outcomes.is_empty() {
                let current = file.schema_version == WINDOW_SCHEMA;
                let saved = file.window.clone().filter(|_| current);
                return Ok(saved.map(|window| window.keeping(capacity)));
            }
            if file.schema_version != WINDOW_SCHEMA {
                *file = WindowFile {
                    schema_version: WINDOW_SCHEMA.to_string(),
                    window: None,
                };
            }
            let mut window = file.window.take().map(|window| window.keeping(capacity));
            for (version, outcome) in outcomes {
                window
                    .get_or_insert_with(|| Self::new(version.clone()).keeping(capacity))
                    .push(version, *outcome);
            }
            file.window.clone_from(&window);
            Ok(window)
        })
    }
}

/// The window's file ([`WINDOW_FILE_NAME`]). A missing file holds no window.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
struct WindowFile {
    /// [`WINDOW_SCHEMA`].
    schema_version: String,
    /// The window of the version last saved.
    window: Option<CalibrationWindow>,
}

/// The gate's report on a window: the `ev.m3.*` values, whether the model may be promoted, and
/// which bounds failed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GateReport {
    /// The predictor version scored.
    pub predictor_version: PredictorVersion,
    /// W, the outcomes the gate scores, which an eligible window holds (gap-4c531f).
    pub window: usize,
    /// Outcomes scored: the window's last W, or all it holds when it holds fewer.
    pub n: usize,
    /// ECE over equal-mass bins.
    pub ece: Option<f64>,
    /// Calibration-in-the-large.
    pub cal_in_large: Option<f64>,
    /// Brier skill.
    pub bss: Option<f64>,
    /// AUROC.
    pub auroc: Option<f64>,
    /// The routed pass rate; `None` while the window holds no routed outcome, which fails no
    /// bound (bug-92cc56).
    pub route_pass: Option<f64>,
    /// The window has W outcomes and meets every bound.
    pub eligible: bool,
    /// The bounds that failed, in words.
    pub reasons: Vec<String>,
    /// The circuit breaker tripped.
    pub breaker_tripped: bool,
}

/// The calibration gate, at a target π*, over a window of W outcomes.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CalibrationGate {
    /// π*, the target the routed pass rate is held to.
    pub target: f64,
    /// W, the outcomes the gate scores and an eligible window must hold: `[self_model] n_min`
    /// (gap-4c531f). Read as at least 1.
    #[serde(default = "default_window")]
    pub window: usize,
}

impl Default for CalibrationGate {
    fn default() -> Self {
        Self {
            target: TARGET,
            window: WINDOW,
        }
    }
}

impl CalibrationGate {
    /// The gate `[self_model]` configures: π* = `target`, and W = `n_min` (gap-4c531f).
    #[must_use]
    pub fn of(settings: &SelfModelConfig) -> Self {
        Self {
            target: settings.target,
            window: (settings.n_min as usize).max(1),
        }
    }

    /// The outcomes a window must keep for this gate ([`CalibrationWindow::keeping`]): W, and
    /// never fewer than [`WINDOW`], which leaves the breaker its [`BREAKER_WINDOW`].
    #[must_use]
    pub fn keeps(&self) -> usize {
        self.window.max(WINDOW)
    }

    /// Score the last W outcomes of `window` against every bound.
    #[must_use]
    pub fn evaluate(&self, window: &CalibrationWindow) -> GateReport {
        let w = self.window.max(1);
        let scored = window.scored(w);
        let n = scored.len();
        let ece = ece(&scored, ECE_BINS);
        let cal_in_large = calibration_in_the_large(&scored);
        let bss = brier_skill(&scored);
        let auroc = auroc(&scored);
        let route_pass = window.route_pass(w);
        let route_floor = self.target - ROUTE_PASS_MARGIN;
        let mut reasons = Vec::new();
        if n < w {
            reasons.push(format!("n = {n}, below the window of {w}"));
        }
        let mut bound = |value: Option<f64>, holds: &dyn Fn(f64) -> bool, rule: &str| match value {
            Some(value) if holds(value) => {}
            Some(value) => reasons.push(format!("{rule}: {value:.3}")),
            None => reasons.push(format!("{rule}: unknown")),
        };
        bound(ece, &|value| value <= ECE_MAX, "ECE above 0.08");
        bound(
            cal_in_large,
            &|value| value.abs() <= CAL_IN_LARGE_MAX,
            "|calibration-in-the-large| above 0.05",
        );
        bound(bss, &|value| value >= BSS_MIN, "Brier skill below 0.05");
        bound(auroc, &|value| value >= AUROC_MIN, "AUROC below 0.65");
        // The routed pass rate binds once the self-model has routed an attempt in the window;
        // unknown, it fails nothing. The model routes only while the gate is eligible, so a
        // bound on no routed attempt would keep every window ineligible, shadow and active
        // alike (bug-92cc56).
        if let Some(value) = route_pass
            && value < route_floor
        {
            reasons.push(format!("routed pass rate below π* − 0.05: {value:.3}"));
        }
        GateReport {
            predictor_version: window.version.clone(),
            window: w,
            n,
            ece,
            cal_in_large,
            bss,
            auroc,
            route_pass,
            eligible: reasons.is_empty(),
            reasons,
            breaker_tripped: self.breaker_tripped(window),
        }
    }

    /// Whether the circuit breaker trips: over the last [`BREAKER_WINDOW`] outcomes the Brier
    /// score is worse than the base rate's, or ECE exceeds [`BREAKER_ECE`]. It needs that many
    /// outcomes first.
    #[must_use]
    pub fn breaker_tripped(&self, window: &CalibrationWindow) -> bool {
        if window.len() < BREAKER_WINDOW {
            return false;
        }
        let recent = window.scored(BREAKER_WINDOW);
        let worse_than_base = match (brier(&recent), base_rate(&recent)) {
            (Some(brier), Some(rate)) => brier > rate * (1.0 - rate),
            _ => false,
        };
        let miscalibrated = ece(&recent, ECE_BINS).is_some_and(|value| value > BREAKER_ECE);
        worse_than_base || miscalibrated
    }
}

/// The self-model's routing mode (S04 §5's `[self_model] mode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// It does not run.
    Off,
    /// It forecasts and logs; routing ignores it.
    Shadow,
    /// It routes.
    Active,
}

/// The mode after `report`: an active model whose breaker tripped drops to shadow. The gate
/// never promotes: a person does (decision 6102).
#[must_use]
pub fn next_mode(current: Mode, report: &GateReport) -> Mode {
    if current == Mode::Active && report.breaker_tripped {
        Mode::Shadow
    } else {
        current
    }
}

/// `ev.m3.resolution`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
struct Resolution {
    bss: Option<f64>,
    auroc: Option<f64>,
}

/// The export's contents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct CalibrationExport {
    schema_version: String,
    predictor_version: PredictorVersion,
    /// W, the window the values are over (gap-4c531f).
    window: usize,
    n: usize,
    #[serde(rename = "ev.m3.ece")]
    ece: Option<f64>,
    #[serde(rename = "ev.m3.cal_in_large")]
    cal_in_large: Option<f64>,
    #[serde(rename = "ev.m3.resolution")]
    resolution: Resolution,
    #[serde(rename = "ev.m3.route_pass")]
    route_pass: Option<f64>,
    eligible: bool,
    reasons: Vec<String>,
    breaker_tripped: bool,
}

/// Write `report` to the `ev.m3.*` export under the `.roko` directory `roko_dir`, atomically
/// (a temporary file of its own, since runs export as they end, then a rename), and return its
/// path.
pub fn write_calibration(report: &GateReport, roko_dir: &Path) -> io::Result<PathBuf> {
    let path = roko_dir.join(CALIBRATION_FILE);
    let export = CalibrationExport {
        schema_version: CALIBRATION_SCHEMA.to_string(),
        predictor_version: report.predictor_version.clone(),
        window: report.window,
        n: report.n,
        ece: report.ece,
        cal_in_large: report.cal_in_large,
        resolution: Resolution {
            bss: report.bss,
            auroc: report.auroc,
        },
        route_pass: report.route_pass,
        eligible: report.eligible,
        reasons: report.reasons.clone(),
        breaker_tripped: report.breaker_tripped,
    };
    let json = serde_json::to_vec_pretty(&export).map_err(io::Error::other)?;
    roko_fs::atomic_write_bytes(&path, &json)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(name: &str) -> PredictorVersion {
        PredictorVersion(name.to_string())
    }

    /// A perfectly calibrated window: ten forecasts at each of 0.1 to 0.9 (and ten more at
    /// 0.5), 10·p of each group passing; the model routed the 0.7 to 0.9 groups. Each group's
    /// passes are spread through it, so the breaker's five-outcome bins see its rate too.
    fn calibrated() -> CalibrationWindow {
        let mut window = CalibrationWindow::new(version("m3-l1-a"));
        for p in [0.1_f64, 0.2, 0.3, 0.4, 0.5, 0.5, 0.6, 0.7, 0.8, 0.9] {
            let passes = (p * 10.0).round() as usize;
            for i in 0..10 {
                let outcome = WindowOutcome {
                    p,
                    y: (i + 1) * passes / 10 > i * passes / 10,
                    w: 1.0,
                    routed: p >= 0.7,
                };
                window.push(&version("m3-l1-a"), outcome);
            }
        }
        window
    }

    #[test]
    fn eligible_only_when_n_and_every_bound_are_met() {
        let gate = CalibrationGate::default();
        let window = calibrated();
        let report = gate.evaluate(&window);
        assert!(report.eligible, "{report:?}");
        assert_eq!(report.n, WINDOW);
        assert!(report.ece.is_some_and(|ece| ece < 1e-12), "{report:?}");
        assert!(report.bss.is_some_and(|bss| (bss - 0.24).abs() < 1e-9));
        assert!(
            report
                .auroc
                .is_some_and(|auroc| (auroc - 0.78).abs() < 1e-9)
        );
        assert!(
            report
                .route_pass
                .is_some_and(|rate| (rate - 0.8).abs() < 1e-9)
        );
        assert!(!report.breaker_tripped);

        // One outcome short of the window.
        let mut short = CalibrationWindow::new(version("m3-l1-a"));
        for outcome in window.outcomes.iter().skip(1) {
            short.push(&version("m3-l1-a"), *outcome);
        }
        let report = gate.evaluate(&short);
        assert!(!report.eligible);
        assert!(
            report
                .reasons
                .iter()
                .any(|reason| reason.starts_with("n = 99"))
        );

        // Forecasts 0.1 too high fail calibration-in-the-large, and the report says so.
        let mut high = CalibrationWindow::new(version("m3-l1-a"));
        for outcome in &window.outcomes {
            let shifted = WindowOutcome {
                p: (outcome.p + 0.1).min(1.0),
                ..*outcome
            };
            high.push(&version("m3-l1-a"), shifted);
        }
        let report = gate.evaluate(&high);
        assert!(!report.eligible);
        let calibration = "|calibration-in-the-large| above 0.05";
        assert!(
            report
                .reasons
                .iter()
                .any(|reason| reason.starts_with(calibration)),
            "{report:?}"
        );
    }

    #[test]
    fn circuit_breaker_drops_to_shadow_on_worse_than_base_rate() {
        let gate = CalibrationGate::default();
        let mut window = CalibrationWindow::new(version("m3-l1-a"));
        // Fifty forecasts that point the wrong way: Brier 0.81 against the base rate's 0.25.
        for i in 0..BREAKER_WINDOW {
            let passed = i % 2 == 1;
            let outcome = WindowOutcome {
                p: if passed { 0.1 } else { 0.9 },
                y: passed,
                w: 1.0,
                routed: true,
            };
            window.push(&version("m3-l1-a"), outcome);
        }
        let report = gate.evaluate(&window);
        assert!(report.breaker_tripped);
        assert!(!report.eligible);
        assert_eq!(next_mode(Mode::Active, &report), Mode::Shadow);
        assert_eq!(next_mode(Mode::Shadow, &report), Mode::Shadow);
        // A calibrated window keeps an active model active, and never promotes a shadow one.
        let good = gate.evaluate(&calibrated());
        assert_eq!(next_mode(Mode::Active, &good), Mode::Active);
        assert_eq!(next_mode(Mode::Shadow, &good), Mode::Shadow);
        // Fewer than fifty outcomes cannot trip it.
        let mut young = CalibrationWindow::new(version("m3-l1-a"));
        young.push(&version("m3-l1-a"), window.outcomes[0]);
        assert!(!gate.breaker_tripped(&young));
    }

    /// bug-92cc56: the self-model routes only while the gate is eligible, so its first window
    /// holds no routed outcome. A calibrated window settled in shadow, none of it routed, makes
    /// the gate eligible; once the model routes, the routed pass rate binds as before.
    #[test]
    fn calibration_gate_can_become_eligible_in_shadow() {
        let gate = CalibrationGate::default();
        let mut shadow = CalibrationWindow::new(version("m3-l1-a"));
        for outcome in &calibrated().outcomes {
            let unrouted = WindowOutcome {
                routed: false,
                ..*outcome
            };
            shadow.push(&version("m3-l1-a"), unrouted);
        }
        let report = gate.evaluate(&shadow);
        assert!(report.eligible, "{report:?}");
        assert_eq!(report.route_pass, None);

        // Routed attempts that pass below π* − 0.05 fail the bound.
        let mut routed = shadow.clone();
        for index in 0..10 {
            let outcome = WindowOutcome {
                p: 0.5,
                y: index < 5,
                w: 1.0,
                routed: true,
            };
            routed.push(&version("m3-l1-a"), outcome);
        }
        let report = gate.evaluate(&routed);
        assert!(!report.eligible, "{report:?}");
        assert_eq!(report.reasons, ["routed pass rate below π* − 0.05: 0.500"]);
    }

    #[test]
    fn the_window_resets_on_a_version_change() {
        let mut window = calibrated();
        assert_eq!(window.len(), WINDOW);
        let outcome = WindowOutcome {
            p: 0.5,
            y: true,
            w: 1.0,
            routed: false,
        };
        window.push(&version("m3-l1-b"), outcome);
        assert_eq!(window.len(), 1);
        assert_eq!(window.version, version("m3-l1-b"));
    }

    /// gap-b38ed2: saves of two runs add up in the window's file, capped at W; a load keeps a
    /// window of its own version only, and sets an unreadable file aside.
    #[test]
    fn the_window_file_keeps_each_runs_outcomes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(WINDOW_FILE_NAME);
        let a = version("m3-l1-a");
        let outcomes: Vec<(PredictorVersion, WindowOutcome)> = calibrated()
            .outcomes
            .iter()
            .map(|outcome| (a.clone(), *outcome))
            .collect();
        assert_eq!(
            CalibrationWindow::save_outcomes(&path, WINDOW, &[]).expect("read"),
            None
        );
        assert!(!path.exists(), "nothing to save writes nothing");

        // Two runs save 60 outcomes each: 120, capped at W.
        let first = CalibrationWindow::save_outcomes(&path, WINDOW, &outcomes[..60]).expect("save");
        assert_eq!(first.map(|window| window.len()), Some(60));
        let second =
            CalibrationWindow::save_outcomes(&path, WINDOW, &outcomes[40..]).expect("save");
        let second = second.expect("the window as saved");
        assert_eq!(second.len(), WINDOW);
        let loaded = CalibrationWindow::load(&path, &a, WINDOW).expect("load");
        assert_eq!(loaded, second);
        assert_eq!(
            CalibrationWindow::save_outcomes(&path, WINDOW, &[]).expect("read"),
            Some(second)
        );

        // Another version's load starts afresh, and its first save restarts the window.
        let b = version("m3-l1-b");
        assert!(
            CalibrationWindow::load(&path, &b, WINDOW)
                .expect("load")
                .is_empty()
        );
        let outcome = outcomes[0].1;
        let restarted = CalibrationWindow::save_outcomes(&path, WINDOW, &[(b.clone(), outcome)])
            .expect("save")
            .expect("the window as saved");
        assert_eq!(restarted.version, b);
        assert_eq!(restarted.len(), 1);

        // An unreadable file is set aside.
        std::fs::write(&path, "{not json").expect("corrupt the file");
        assert!(
            CalibrationWindow::load(&path, &b, WINDOW)
                .expect("load")
                .is_empty()
        );
        assert!(!path.exists() && path.with_extension("json.unreadable").exists());
    }

    /// Ten forecasts that are sharp and right: eight passes forecast at 0.96 and two failures
    /// at 0.04, each routed, the failures second and sixth. Over ten equal-mass bins of one
    /// forecast each, ECE is 0.04.
    fn sharp() -> Vec<WindowOutcome> {
        (0..10)
            .map(|index| {
                let passed = index != 1 && index != 5;
                WindowOutcome {
                    p: if passed { 0.96 } else { 0.04 },
                    y: passed,
                    w: 1.0,
                    routed: true,
                }
            })
            .collect()
    }

    /// gap-4c531f: the gate scores its own W. With W = 10, ten sharp forecasts are eligible,
    /// while the window keeps W = 100 outcomes, so the breaker keeps its fifty; the default W
    /// of 100 finds them short. With W = 150 the window and its file keep 150 outcomes, and the
    /// gate scores them all.
    #[test]
    fn the_gate_scores_a_window_of_its_own_w() {
        let a = version("m3-l1-a");
        let small = CalibrationGate {
            target: TARGET,
            window: 10,
        };
        assert_eq!(small.keeps(), WINDOW);
        let mut window = CalibrationWindow::new(a.clone()).keeping(small.keeps());
        for outcome in sharp() {
            window.push(&a, outcome);
        }
        let report = small.evaluate(&window);
        assert!(report.eligible, "{report:?}");
        assert_eq!((report.window, report.n), (10, 10));
        let report = CalibrationGate::default().evaluate(&window);
        assert!(!report.eligible);
        let short = "n = 10, below the window of 100";
        assert!(
            report.reasons.iter().any(|reason| reason == short),
            "{report:?}"
        );

        let large = CalibrationGate {
            target: TARGET,
            window: 150,
        };
        let outcomes: Vec<(PredictorVersion, WindowOutcome)> = (0..16)
            .flat_map(|_| sharp())
            .map(|outcome| (a.clone(), outcome))
            .collect();
        let mut window = CalibrationWindow::new(a.clone()).keeping(large.keeps());
        for (version, outcome) in &outcomes {
            window.push(version, *outcome);
        }
        assert_eq!(window.len(), 150, "160 pushed, the last 150 kept");
        let report = large.evaluate(&window);
        assert!(report.eligible, "{report:?}");
        assert_eq!((report.window, report.n), (150, 150));

        // The window's file keeps what the gate needs, and a smaller W's load keeps W = 100.
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(WINDOW_FILE_NAME);
        let saved = CalibrationWindow::save_outcomes(&path, large.keeps(), &outcomes)
            .expect("save")
            .expect("the window as saved");
        assert_eq!(saved, window);
        let loaded = CalibrationWindow::load(&path, &a, large.keeps()).expect("load");
        assert_eq!(loaded.len(), 150);
        let loaded = CalibrationWindow::load(&path, &a, small.keeps()).expect("load");
        assert_eq!(loaded.len(), WINDOW);
    }

    #[test]
    fn the_export_is_written_atomically() {
        let dir = tempfile::tempdir().expect("tempdir");
        let report = CalibrationGate::default().evaluate(&calibrated());
        let path = write_calibration(&report, dir.path()).expect("write the export");
        assert_eq!(path, dir.path().join(CALIBRATION_FILE));
        assert!(!path.with_extension("json.tmp").exists());
        let text = std::fs::read_to_string(&path).expect("read the export");
        let json: serde_json::Value = serde_json::from_str(&text).expect("json");
        assert_eq!(json["schema_version"], CALIBRATION_SCHEMA);
        assert_eq!(json["predictor_version"], "m3-l1-a");
        assert_eq!(json["eligible"], true);
        assert!(json["ev.m3.ece"].is_number());
        assert!(json["ev.m3.resolution"]["auroc"].is_number());
    }
}
