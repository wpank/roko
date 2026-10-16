//! The promotion gate that moves the self-model out of shadow mode: backlog task 6122.
//!
//! The self-model is "promoted only when calibrated" (P18). [`CalibrationGate::evaluate`] scores
//! the last W = 100 labelled outcomes of the current predictor version against S04 §4.7's bounds
//! on its essential variables: ECE ≤ 0.08, |calibration-in-the-large| ≤ 0.05, BSS ≥ 0.05 and
//! AUROC ≥ 0.65, and the pass rate of the attempts it routed ≥ π* − 0.05. Per decision 6102 it
//! calibrates on gate labels now and VS labels later, and the report only makes the model
//! eligible: a person promotes it. The circuit breaker (S04 §4.11.5) trips when the Brier score
//! over the last 50 outcomes is worse than the base rate's, or ECE exceeds 0.2, and an active
//! model then drops back to shadow ([`next_mode`]). A new predictor version restarts the window
//! (§4.11.6). No single run settles W outcomes, so the window is kept across runs in
//! [`WINDOW_FILE_NAME`] beside the state file ([`CalibrationWindow::load`],
//! [`CalibrationWindow::save_outcomes`]; gap-b38ed2). [`write_calibration`] exports the
//! `ev.m3.*` values S06 reads, atomically.
//!
//! Demotion for "no benefit after 150 exposed decisions" is S03's L-M3 loop audit, not this gate.

use std::collections::VecDeque;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::PredictorVersion;
use super::metrics::{
    ECE_BINS, Scored, auroc, base_rate, brier, brier_skill, calibration_in_the_large, ece,
};
use super::policy::TARGET;

/// The outcomes the gate scores, W (S04 §4.10).
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

/// The last [`WINDOW`] outcomes of one predictor version.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalibrationWindow {
    /// The version whose forecasts the window holds.
    pub version: PredictorVersion,
    outcomes: VecDeque<WindowOutcome>,
}

impl CalibrationWindow {
    /// An empty window for `version`.
    #[must_use]
    pub fn new(version: PredictorVersion) -> Self {
        Self {
            version,
            outcomes: VecDeque::with_capacity(WINDOW + 1),
        }
    }

    /// Add an outcome of a forecast by `version`. A new version restarts the window, since
    /// forecasts are never compared across versions.
    pub fn push(&mut self, version: &PredictorVersion, outcome: WindowOutcome) {
        if *version != self.version {
            self.version = version.clone();
            self.outcomes.clear();
        }
        self.outcomes.push_back(outcome);
        if self.outcomes.len() > WINDOW {
            self.outcomes.pop_front();
        }
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

    /// The weighted pass rate of the attempts the self-model routed.
    fn route_pass(&self) -> Option<f64> {
        let routed: Vec<Scored> = self
            .outcomes
            .iter()
            .filter(|outcome| outcome.routed)
            .map(|outcome| Scored::weighted(outcome.p, outcome.y, outcome.w))
            .collect();
        base_rate(&routed)
    }

    /// The window kept in the file at `path` when it holds forecasts of `version`, else an empty
    /// window of `version`: a missing file, a window of another version (forecasts are never
    /// compared across versions) and a file of another schema start afresh. A file that does not
    /// parse is set aside, renamed `<name>.unreadable`, as an unreadable state file is, so the
    /// next save starts a new one.
    ///
    /// # Errors
    ///
    /// Returns an error when the file exists but cannot be read or set aside.
    pub fn load(path: &Path, version: &PredictorVersion) -> io::Result<Self> {
        match roko_fs::read_json_or_default_strict::<WindowFile>(path) {
            Ok(file) => {
                let current = file.schema_version == WINDOW_SCHEMA;
                let kept = file
                    .window
                    .filter(|window| current && window.version == *version);
                Ok(kept.unwrap_or_else(|| Self::new(version.clone())))
            }
            Err(error) if error.kind() == io::ErrorKind::InvalidData => {
                std::fs::rename(path, path.with_extension("json.unreadable"))?;
                Ok(Self::new(version.clone()))
            }
            Err(error) => Err(error),
        }
    }

    /// Add `outcomes`, each with the version that forecast it, to the window kept in the file at
    /// `path`, in one transaction under the file's sibling lock, so that a concurrent run's
    /// outcomes survive, and return the window as saved: `None` when the file holds none. Each
    /// outcome is [pushed](Self::push), so one of a new version restarts the window. With no
    /// outcomes the file is only read.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be read, parsed or written.
    pub fn save_outcomes(
        path: &Path,
        outcomes: &[(PredictorVersion, WindowOutcome)],
    ) -> io::Result<Option<Self>> {
        roko_fs::with_locked_json_transaction::<WindowFile, _, io::Error, _>(path, |file| {
            if outcomes.is_empty() {
                let current = file.schema_version == WINDOW_SCHEMA;
                return Ok(file.window.clone().filter(|_| current));
            }
            if file.schema_version != WINDOW_SCHEMA {
                *file = WindowFile {
                    schema_version: WINDOW_SCHEMA.to_string(),
                    window: None,
                };
            }
            for (version, outcome) in outcomes {
                file.window
                    .get_or_insert_with(|| Self::new(version.clone()))
                    .push(version, *outcome);
            }
            Ok(file.window.clone())
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
    /// Outcomes in the window.
    pub n: usize,
    /// ECE over equal-mass bins.
    pub ece: Option<f64>,
    /// Calibration-in-the-large.
    pub cal_in_large: Option<f64>,
    /// Brier skill.
    pub bss: Option<f64>,
    /// AUROC.
    pub auroc: Option<f64>,
    /// The routed pass rate.
    pub route_pass: Option<f64>,
    /// The window has W outcomes and meets every bound.
    pub eligible: bool,
    /// The bounds that failed, in words.
    pub reasons: Vec<String>,
    /// The circuit breaker tripped.
    pub breaker_tripped: bool,
}

/// The calibration gate, at a target π*.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CalibrationGate {
    /// π*, the target the routed pass rate is held to.
    pub target: f64,
}

impl Default for CalibrationGate {
    fn default() -> Self {
        Self { target: TARGET }
    }
}

impl CalibrationGate {
    /// Score `window` against every bound.
    #[must_use]
    pub fn evaluate(&self, window: &CalibrationWindow) -> GateReport {
        let scored = window.scored(WINDOW);
        let n = scored.len();
        let ece = ece(&scored, ECE_BINS);
        let cal_in_large = calibration_in_the_large(&scored);
        let bss = brier_skill(&scored);
        let auroc = auroc(&scored);
        let route_pass = window.route_pass();
        let route_floor = self.target - ROUTE_PASS_MARGIN;
        let mut reasons = Vec::new();
        if n < WINDOW {
            reasons.push(format!("n = {n}, below the window of {WINDOW}"));
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
        bound(
            route_pass,
            &|value| value >= route_floor,
            "routed pass rate below π* − 0.05",
        );
        GateReport {
            predictor_version: window.version.clone(),
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
            CalibrationWindow::save_outcomes(&path, &[]).expect("read"),
            None
        );
        assert!(!path.exists(), "nothing to save writes nothing");

        // Two runs save 60 outcomes each: 120, capped at W.
        let first = CalibrationWindow::save_outcomes(&path, &outcomes[..60]).expect("save");
        assert_eq!(first.map(|window| window.len()), Some(60));
        let second = CalibrationWindow::save_outcomes(&path, &outcomes[40..]).expect("save");
        let second = second.expect("the window as saved");
        assert_eq!(second.len(), WINDOW);
        let loaded = CalibrationWindow::load(&path, &a).expect("load");
        assert_eq!(loaded, second);
        assert_eq!(
            CalibrationWindow::save_outcomes(&path, &[]).expect("read"),
            Some(second)
        );

        // Another version's load starts afresh, and its first save restarts the window.
        let b = version("m3-l1-b");
        assert!(CalibrationWindow::load(&path, &b).expect("load").is_empty());
        let outcome = outcomes[0].1;
        let restarted = CalibrationWindow::save_outcomes(&path, &[(b.clone(), outcome)])
            .expect("save")
            .expect("the window as saved");
        assert_eq!(restarted.version, b);
        assert_eq!(restarted.len(), 1);

        // An unreadable file is set aside.
        std::fs::write(&path, "{not json").expect("corrupt the file");
        assert!(CalibrationWindow::load(&path, &b).expect("load").is_empty());
        assert!(!path.exists() && path.with_extension("json.unreadable").exists());
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
