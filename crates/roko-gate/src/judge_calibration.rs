//! Judge calibration utilities for [`crate::llm_judge_gate::LlmJudgeGate`].
//!
//! Provides golden-set loading, accuracy/precision/recall metrics, and
//! length-bias detection for offline analysis of judge quality.
//!
//! # Golden set
//!
//! Load a JSON array from a file with [`load_golden_set`], run the judge
//! against each example, collect `(predicted, actual)` pairs, then call
//! [`compute_calibration`] to get the full [`JudgeCalibration`] report.
//!
//! # Length bias
//!
//! [`detect_length_bias`] computes the Pearson correlation coefficient between
//! output length (bytes) and whether the judge passed. A coefficient near +1
//! means longer outputs almost always pass (length inflation bias); near -1
//! means longer outputs tend to fail; near 0 means length is not a signal.
//!
//! # Live telemetry
//!
//! [`append_calibration_record`] appends a single JSONL record to the
//! configured path so production runs accumulate a durable calibration log
//! at `.roko/learn/judge-calibration.jsonl`.

use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::time::SystemTime;

/// A labeled calibration example loaded from the golden set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationExample {
    /// Optional stable identifier for traceability.
    #[serde(default)]
    pub id: String,
    /// The task the agent was asked to perform (maps to [`JudgePayload::task_description`]).
    pub task_description: String,
    /// The agent's output / diff (maps to [`JudgePayload::diff`]).
    pub agent_output: String,
    /// The ground-truth verdict: `true` means the output should pass the judge.
    pub expected_verdict: bool,
    /// Human-readable justification for the expected verdict.
    #[serde(default)]
    pub reasoning: String,
}

/// Aggregate calibration metrics computed from a set of judge predictions.
///
/// All rates are in `[0.0, 1.0]`.  `NaN` is used when the denominator is
/// zero (e.g. `precision` is `NaN` when the judge never predicts pass).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JudgeCalibration {
    /// Fraction of predictions that match the expected verdict.
    pub accuracy: f64,
    /// `tp / (tp + fp)` — fraction of predicted-pass that should pass.
    pub precision: f64,
    /// `tp / (tp + fn)` — fraction of actual-pass that was predicted pass.
    pub recall: f64,
    /// `fp / (fp + tn)` — false positive rate (specificity complement).
    pub false_positive_rate: f64,
    /// `fn / (fn + tp)` — false negative rate (miss rate).
    pub false_negative_rate: f64,
    /// Pearson r between output length (bytes) and predicted pass, if computed.
    ///
    /// `None` when fewer than two examples are available or all lengths are
    /// identical (zero variance).
    pub length_bias: Option<f64>,
    /// Total number of examples evaluated.
    pub total_examples: usize,
    /// Number of examples where predicted == actual.
    pub correct: usize,
}

impl JudgeCalibration {
    /// F1 score: harmonic mean of precision and recall.
    ///
    /// Returns `NaN` when both precision and recall are `NaN` or zero.
    #[must_use]
    pub fn f1(&self) -> f64 {
        let p = self.precision;
        let r = self.recall;
        if (p + r) == 0.0 {
            return f64::NAN;
        }
        2.0 * p * r / (p + r)
    }
}

/// Load all [`CalibrationExample`]s from a JSON file.
///
/// The file must contain a JSON array of objects matching [`CalibrationExample`].
///
/// # Errors
///
/// Returns an [`io::Error`] if the file cannot be read or the JSON is invalid.
pub fn load_golden_set(path: &Path) -> io::Result<Vec<CalibrationExample>> {
    let data = std::fs::read(path)?;
    serde_json::from_slice(&data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// Compute calibration metrics from a slice of `(predicted, actual)` pairs.
///
/// Each pair is `(judge_verdict, expected_verdict)` where `true` = pass.
///
/// Returns a [`JudgeCalibration`] with `length_bias: None`. To include length
/// bias, compute it separately with [`detect_length_bias`] and set the field.
///
/// # Panics
///
/// Does not panic; returns `accuracy = NaN` when `predictions` is empty.
#[must_use]
pub fn compute_calibration(predictions: &[(bool, bool)]) -> JudgeCalibration {
    let total = predictions.len();
    if total == 0 {
        return JudgeCalibration {
            accuracy: f64::NAN,
            precision: f64::NAN,
            recall: f64::NAN,
            false_positive_rate: f64::NAN,
            false_negative_rate: f64::NAN,
            length_bias: None,
            total_examples: 0,
            correct: 0,
        };
    }

    let mut tp = 0usize; // predicted pass, actually pass
    let mut fp = 0usize; // predicted pass, actually fail
    let mut tn = 0usize; // predicted fail, actually fail
    let mut fn_ = 0usize; // predicted fail, actually pass
    let mut correct = 0usize;

    for &(predicted, actual) in predictions {
        match (predicted, actual) {
            (true, true) => {
                tp += 1;
                correct += 1;
            }
            (true, false) => {
                fp += 1;
            }
            (false, false) => {
                tn += 1;
                correct += 1;
            }
            (false, true) => {
                fn_ += 1;
            }
        }
    }

    let accuracy = correct as f64 / total as f64;
    let precision = if tp + fp == 0 {
        f64::NAN
    } else {
        tp as f64 / (tp + fp) as f64
    };
    let recall = if tp + fn_ == 0 {
        f64::NAN
    } else {
        tp as f64 / (tp + fn_) as f64
    };
    let false_positive_rate = if fp + tn == 0 {
        f64::NAN
    } else {
        fp as f64 / (fp + tn) as f64
    };
    let false_negative_rate = if fn_ + tp == 0 {
        f64::NAN
    } else {
        fn_ as f64 / (fn_ + tp) as f64
    };

    JudgeCalibration {
        accuracy,
        precision,
        recall,
        false_positive_rate,
        false_negative_rate,
        length_bias: None,
        total_examples: total,
        correct,
    }
}

/// Compute the Pearson correlation coefficient between output length and judge
/// pass rate.
///
/// `examples` is a slice of `(output_length_bytes, judge_predicted_pass)`.
///
/// Returns `None` when:
/// - fewer than two data points are available, or
/// - all lengths are identical (zero variance in the X variable).
///
/// A positive value indicates longer outputs tend to pass (length-inflation
/// bias); a negative value indicates they tend to fail.
#[must_use]
pub fn detect_length_bias(examples: &[(usize, bool)]) -> Option<f64> {
    let n = examples.len();
    if n < 2 {
        return None;
    }

    let xs: Vec<f64> = examples.iter().map(|&(len, _)| len as f64).collect();
    let ys: Vec<f64> = examples
        .iter()
        .map(|&(_, pass)| if pass { 1.0 } else { 0.0 })
        .collect();

    let mean_x = xs.iter().sum::<f64>() / n as f64;
    let mean_y = ys.iter().sum::<f64>() / n as f64;

    let mut cov = 0.0f64;
    let mut var_x = 0.0f64;
    let mut var_y = 0.0f64;

    for i in 0..n {
        let dx = xs[i] - mean_x;
        let dy = ys[i] - mean_y;
        cov += dx * dy;
        var_x += dx * dx;
        var_y += dy * dy;
    }

    // Zero variance in X means all lengths are identical — correlation is
    // undefined.
    if var_x == 0.0 || var_y == 0.0 {
        return None;
    }

    Some(cov / (var_x.sqrt() * var_y.sqrt()))
}

/// A single judge decision record appended to the calibration JSONL log.
///
/// Written by [`append_calibration_record`] during live `verify()` calls so
/// the log at `.roko/learn/judge-calibration.jsonl` accumulates over time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationRecord {
    /// Stable task identifier (e.g. task slug or signal ID).
    pub task_id: String,
    /// Whether the judge passed (`true`) or failed (`false`).
    pub verdict: bool,
    /// Judge confidence / score in `[0, 1]`.
    pub confidence: f32,
    /// Byte length of the agent output / diff that was judged.
    pub output_length: usize,
    /// Unix timestamp in seconds.
    pub timestamp: u64,
}

/// Append a single [`CalibrationRecord`] as a JSONL line to `path`.
///
/// Creates the file (and parent directories) if they do not exist.
/// Errors are non-fatal for the caller — the gate should log but not fail.
///
/// # Errors
///
/// Returns `io::Error` on file-system or serialization failure.
pub fn append_calibration_record(path: &Path, record: &CalibrationRecord) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let line =
        serde_json::to_string(record).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(line.as_bytes())?;
    file.write_all(b"\n")?;
    Ok(())
}

/// Read all [`CalibrationRecord`]s from a JSONL file.
///
/// Skips blank lines and lines that fail to parse (non-fatal).
///
/// # Errors
///
/// Returns `io::Error` if the file cannot be opened.
pub fn load_calibration_log(path: &Path) -> io::Result<Vec<CalibrationRecord>> {
    let file = std::fs::File::open(path)?;
    let reader = io::BufReader::new(file);
    let mut out = Vec::new();
    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(rec) = serde_json::from_str::<CalibrationRecord>(trimmed) {
            out.push(rec);
        }
    }
    Ok(out)
}

/// Build a [`CalibrationRecord`] timestamp from the system clock.
///
/// Falls back to `0` if the clock is before the Unix epoch.
#[must_use]
pub fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // ── compute_calibration ─────────────────────────────────────────────────

    #[test]
    fn empty_predictions_returns_nan_accuracy() {
        let cal = compute_calibration(&[]);
        assert!(cal.accuracy.is_nan());
        assert_eq!(cal.total_examples, 0);
        assert_eq!(cal.correct, 0);
    }

    #[test]
    fn perfect_accuracy() {
        let preds = vec![(true, true), (false, false), (true, true), (false, false)];
        let cal = compute_calibration(&preds);
        assert!((cal.accuracy - 1.0).abs() < 1e-9);
        assert!((cal.precision - 1.0).abs() < 1e-9);
        assert!((cal.recall - 1.0).abs() < 1e-9);
        assert!((cal.false_positive_rate - 0.0).abs() < 1e-9);
        assert!((cal.false_negative_rate - 0.0).abs() < 1e-9);
        assert_eq!(cal.total_examples, 4);
        assert_eq!(cal.correct, 4);
    }

    #[test]
    fn always_wrong() {
        // Predict pass when should fail, predict fail when should pass.
        let preds = vec![(true, false), (false, true)];
        let cal = compute_calibration(&preds);
        assert!((cal.accuracy - 0.0).abs() < 1e-9);
        assert_eq!(cal.correct, 0);
    }

    #[test]
    fn precision_recall_computation() {
        // tp=2, fp=1, fn=1, tn=1  →  precision=2/3, recall=2/3
        let preds = vec![
            (true, true),   // tp
            (true, true),   // tp
            (true, false),  // fp
            (false, true),  // fn
            (false, false), // tn
        ];
        let cal = compute_calibration(&preds);
        let expected_prec = 2.0 / 3.0;
        let expected_rec = 2.0 / 3.0;
        assert!((cal.precision - expected_prec).abs() < 1e-9);
        assert!((cal.recall - expected_rec).abs() < 1e-9);
        // accuracy = (tp + tn) / total = 3 / 5
        assert!((cal.accuracy - 3.0 / 5.0).abs() < 1e-9);
    }

    #[test]
    fn false_positive_rate() {
        // fp=1, tn=3  →  fpr = 1/4 = 0.25
        let preds = vec![
            (true, false),  // fp
            (false, false), // tn
            (false, false), // tn
            (false, false), // tn
        ];
        let cal = compute_calibration(&preds);
        assert!((cal.false_positive_rate - 0.25).abs() < 1e-9);
    }

    #[test]
    fn false_negative_rate() {
        // fn=2, tp=2  →  fnr = 2/4 = 0.5
        let preds = vec![
            (true, true),  // tp
            (true, true),  // tp
            (false, true), // fn
            (false, true), // fn
        ];
        let cal = compute_calibration(&preds);
        assert!((cal.false_negative_rate - 0.5).abs() < 1e-9);
    }

    #[test]
    fn precision_nan_when_judge_never_predicts_pass() {
        let preds = vec![(false, true), (false, false)];
        let cal = compute_calibration(&preds);
        assert!(
            cal.precision.is_nan(),
            "precision should be NaN with no positive predictions"
        );
    }

    #[test]
    fn recall_nan_when_no_actual_positives() {
        let preds = vec![(true, false), (false, false)];
        let cal = compute_calibration(&preds);
        assert!(
            cal.recall.is_nan(),
            "recall should be NaN when no actual positives"
        );
    }

    #[test]
    fn f1_computation() {
        // precision = recall = 2/3  →  f1 = 2/3
        let preds = vec![
            (true, true),
            (true, true),
            (true, false),
            (false, true),
            (false, false),
        ];
        let cal = compute_calibration(&preds);
        let expected_f1 = 2.0 / 3.0;
        assert!((cal.f1() - expected_f1).abs() < 1e-9);
    }

    #[test]
    fn f1_nan_when_precision_and_recall_nan() {
        let preds = vec![(false, false)]; // no positives at all
        let cal = compute_calibration(&preds);
        // precision=NaN because no predicted positives, recall=NaN because no
        // actual positives → f1 = NaN
        let f1 = cal.f1();
        assert!(
            f1.is_nan() || f1 == 0.0,
            "f1 should be NaN or 0 with no positives"
        );
    }

    // ── detect_length_bias ──────────────────────────────────────────────────

    #[test]
    fn length_bias_none_with_one_example() {
        assert!(detect_length_bias(&[(100, true)]).is_none());
    }

    #[test]
    fn length_bias_none_with_identical_lengths() {
        let examples = vec![(50, true), (50, false), (50, true)];
        assert!(detect_length_bias(&examples).is_none());
    }

    #[test]
    fn length_bias_positive_correlation() {
        // Longer → pass, shorter → fail: strong positive correlation.
        let examples = vec![
            (10, false),
            (20, false),
            (30, false),
            (100, true),
            (200, true),
            (300, true),
        ];
        let r = detect_length_bias(&examples).expect("should compute correlation");
        assert!(r > 0.7, "expected strong positive correlation, got {r}");
    }

    #[test]
    fn length_bias_negative_correlation() {
        // Shorter → pass, longer → fail: strong negative correlation.
        let examples = vec![
            (10, true),
            (20, true),
            (30, true),
            (100, false),
            (200, false),
            (300, false),
        ];
        let r = detect_length_bias(&examples).expect("should compute correlation");
        assert!(r < -0.7, "expected strong negative correlation, got {r}");
    }

    #[test]
    fn length_bias_near_zero_for_uncorrelated() {
        // Same length for all true cases and all false cases but equal
        // representation means no length signal: r should be 0.
        let examples = vec![(100, true), (100, false), (100, true), (100, false)];
        // All lengths identical → None (zero variance).
        assert!(detect_length_bias(&examples).is_none());
    }

    #[test]
    fn length_bias_range_is_bounded() {
        let examples = vec![(10, true), (20, false), (30, true), (40, false)];
        let r = detect_length_bias(&examples).expect("correlation");
        assert!(
            r >= -1.0 && r <= 1.0,
            "Pearson r must be in [-1, 1], got {r}"
        );
    }

    // ── load_golden_set ─────────────────────────────────────────────────────

    #[test]
    fn load_golden_set_parses_fixture() {
        let fixture =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/judge-golden-set.json");
        let examples = load_golden_set(&fixture).expect("failed to load golden set");
        assert_eq!(examples.len(), 20, "golden set should contain 20 examples");

        // Verify mix of pass/fail.
        let pass_count = examples.iter().filter(|e| e.expected_verdict).count();
        let fail_count = examples.iter().filter(|e| !e.expected_verdict).count();
        assert!(pass_count > 0, "golden set must include passing examples");
        assert!(fail_count > 0, "golden set must include failing examples");
    }

    #[test]
    fn load_golden_set_fields_populated() {
        let fixture =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/judge-golden-set.json");
        let examples = load_golden_set(&fixture).expect("load golden set");
        for ex in &examples {
            assert!(
                !ex.task_description.is_empty(),
                "task_description must not be empty"
            );
            assert!(
                !ex.agent_output.is_empty(),
                "agent_output must not be empty"
            );
            assert!(
                !ex.reasoning.is_empty(),
                "reasoning must not be empty for id={}",
                ex.id
            );
        }
    }

    #[test]
    fn load_golden_set_missing_file_errors() {
        let result = load_golden_set(Path::new("/nonexistent/path/set.json"));
        assert!(result.is_err());
    }

    // ── append_calibration_record / load_calibration_log ────────────────────

    #[test]
    fn round_trip_calibration_record() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("learn").join("judge-calibration.jsonl");

        let rec1 = CalibrationRecord {
            task_id: "task-abc".to_string(),
            verdict: true,
            confidence: 0.87,
            output_length: 512,
            timestamp: 1_000_000,
        };
        let rec2 = CalibrationRecord {
            task_id: "task-xyz".to_string(),
            verdict: false,
            confidence: 0.42,
            output_length: 128,
            timestamp: 1_000_001,
        };

        append_calibration_record(&path, &rec1).expect("append rec1");
        append_calibration_record(&path, &rec2).expect("append rec2");

        let loaded = load_calibration_log(&path).expect("load log");
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].task_id, "task-abc");
        assert!(loaded[0].verdict);
        assert!((loaded[0].confidence - 0.87).abs() < 1e-5);
        assert_eq!(loaded[0].output_length, 512);
        assert_eq!(loaded[1].task_id, "task-xyz");
        assert!(!loaded[1].verdict);
    }

    #[test]
    fn append_creates_parent_dirs() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("a").join("b").join("c").join("cal.jsonl");
        let rec = CalibrationRecord {
            task_id: "t".to_string(),
            verdict: true,
            confidence: 1.0,
            output_length: 10,
            timestamp: 0,
        };
        append_calibration_record(&path, &rec).expect("should create parent dirs");
        assert!(path.exists());
    }

    #[test]
    fn load_log_missing_file_errors() {
        let result = load_calibration_log(Path::new("/nonexistent/cal.jsonl"));
        assert!(result.is_err());
    }

    #[test]
    fn load_log_skips_blank_lines() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("cal.jsonl");
        let rec = CalibrationRecord {
            task_id: "t".to_string(),
            verdict: false,
            confidence: 0.1,
            output_length: 5,
            timestamp: 42,
        };
        // Write a valid line, a blank, and another valid line manually.
        let json = serde_json::to_string(&rec).unwrap();
        std::fs::write(&path, format!("{json}\n\n{json}\n")).unwrap();
        let loaded = load_calibration_log(&path).unwrap();
        assert_eq!(loaded.len(), 2);
    }

    #[test]
    fn now_unix_secs_is_positive() {
        assert!(now_unix_secs() > 0);
    }
}
