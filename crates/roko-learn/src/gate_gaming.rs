//! Gate gaming detector — flags when agents game the gate system by passing
//! gates at an increasing rate while delivering lower-quality outputs.
//!
//! The detector maintains a per-model rolling window of 50 observations.
//! Gaming is flagged when the gate pass rate rises more than 15 percentage
//! points and the quality score falls more than 10 percentage points between
//! the first and second halves of the window.
//!
//! Alerts are logged with [`tracing::warn!`] and appended to
//! `.roko/learn/gate-gaming-alerts.jsonl`.
//!
//! Observations can carry a weight and count toward the pass rate, the
//! quality or both ([`GateGamingDetector::observe_weighted`], backlog 7128):
//! the audit worker adds each settled attempt's gate verdict at weight 1 and
//! each audited label, quality 1 − Y, at weight 1/π_i. The window then keeps
//! observations by weight, not by count, and splits its weight in halves.

use std::collections::{HashMap, VecDeque};
use std::io;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tracing::warn;

/// Default rolling window size (number of observations per model).
pub const DEFAULT_WINDOW_SIZE: usize = 50;

/// Minimum number of observations required before gaming detection runs.
///
/// Must equal `window_size` so both halves are fully populated.
const MIN_WINDOW_FOR_DETECTION: usize = DEFAULT_WINDOW_SIZE;

/// Gate pass-rate rise threshold in percentage points (0.15 = 15 pp).
const PASS_RATE_RISE_THRESHOLD: f64 = 0.15;

/// Quality score fall threshold in percentage points (0.10 = 10 pp).
const QUALITY_FALL_THRESHOLD: f64 = 0.10;

/// A single gate + quality observation for a model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GamingObservation {
    /// Whether the gate passed for this invocation.
    pub gate_passed: bool,
    /// Scalar quality score in [0.0, 1.0].
    ///
    /// The caller is responsible for computing this value.  A reasonable
    /// fallback when no c-factor quality is available is
    /// `gate_pass_rate * 0.5 + 0.5`.
    pub quality_score: f64,
    /// Wall-clock time the observation was recorded.
    pub timestamp: DateTime<Utc>,
    /// Its weight in the window: 1, or 1/π_i for an audited unit.
    #[serde(default = "unit_weight")]
    pub weight: f64,
    /// What it counts toward.
    #[serde(default)]
    pub observed: Observed,
}

/// What a [`GamingObservation`] counts toward.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Observed {
    /// The pass rate and the quality: a gate evaluation with a quality score.
    #[default]
    Both,
    /// The pass rate alone: a gate verdict whose quality is unknown.
    Gate,
    /// The quality alone: an audited unit's label.
    Quality,
}

const fn unit_weight() -> f64 {
    1.0
}

impl GamingObservation {
    /// Create an observation stamped at the current UTC time.
    #[must_use]
    pub fn now(gate_passed: bool, quality_score: f64) -> Self {
        Self {
            gate_passed,
            quality_score,
            timestamp: Utc::now(),
            weight: 1.0,
            observed: Observed::Both,
        }
    }
}

/// A detected gate-gaming alert for a specific model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GamingAlert {
    /// Model slug that triggered the alert.
    pub model_slug: String,
    /// Pass-rate change (second half minus first half).  Positive means rising.
    pub pass_rate_delta: f64,
    /// Quality change (second half minus first half).  Negative means falling.
    pub quality_delta: f64,
    /// Gate pass rate in the first half of the window.
    pub first_half_pass_rate: f64,
    /// Gate pass rate in the second half of the window.
    pub second_half_pass_rate: f64,
    /// Average quality score in the first half of the window.
    pub first_half_quality: f64,
    /// Average quality score in the second half of the window.
    pub second_half_quality: f64,
    /// Wall-clock time the alert was generated.
    pub timestamp: DateTime<Utc>,
}

impl GamingAlert {
    /// Return a human-readable one-line summary suitable for log output.
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "gate gaming detected for model `{}`: pass_rate +{:.1}pp, quality -{:.1}pp \
             (first_half: pass={:.1}% q={:.2}, second_half: pass={:.1}% q={:.2})",
            self.model_slug,
            self.pass_rate_delta * 100.0,
            -self.quality_delta * 100.0,
            self.first_half_pass_rate * 100.0,
            self.first_half_quality,
            self.second_half_pass_rate * 100.0,
            self.second_half_quality,
        )
    }
}

/// Per-model rolling-window gate gaming detector.
///
/// Call [`observe_and_detect`][Self::observe_and_detect] on every gate
/// evaluation to accumulate observations and check for gaming patterns.
/// Detected alerts are written to `alerts_path` (one JSON object per line).
#[derive(Debug)]
pub struct GateGamingDetector {
    /// Per-model observation ring buffers.
    observations: HashMap<String, VecDeque<GamingObservation>>,
    /// Number of observations kept per model.
    window_size: usize,
    /// Path to the append-only JSONL alert log.
    alerts_path: PathBuf,
}

impl GateGamingDetector {
    /// Create a detector using [`DEFAULT_WINDOW_SIZE`] that writes alerts to
    /// `alerts_path`.
    #[must_use]
    pub fn new(alerts_path: impl Into<PathBuf>) -> Self {
        Self {
            observations: HashMap::new(),
            window_size: DEFAULT_WINDOW_SIZE,
            alerts_path: alerts_path.into(),
        }
    }

    /// Create a detector with a custom window size.
    ///
    /// `window_size` is clamped to at least 2 so the split-half comparison
    /// is always valid.
    #[must_use]
    pub fn with_window_size(mut self, window_size: usize) -> Self {
        self.window_size = window_size.max(2);
        self
    }

    /// Return the configured window size.
    #[must_use]
    pub const fn window_size(&self) -> usize {
        self.window_size
    }

    /// Return the path where alerts are written.
    #[must_use]
    pub fn alerts_path(&self) -> &Path {
        &self.alerts_path
    }

    /// Add one observation for `model_slug`.
    ///
    /// Older observations are evicted once the window is full.
    pub fn observe(&mut self, model_slug: &str, gate_passed: bool, quality_score: f64) {
        self.observe_weighted(model_slug, Some(gate_passed), Some(quality_score), 1.0);
    }

    /// Add one observation for `model_slug` at `weight`: of the gate's
    /// verdict, of quality, or both (backlog 7128).
    ///
    /// The window keeps the newest observations whose weights sum to at least
    /// its size. An observation of neither, or of no positive weight, is
    /// ignored.
    pub fn observe_weighted(
        &mut self,
        model_slug: &str,
        gate_passed: Option<bool>,
        quality_score: Option<f64>,
        weight: f64,
    ) {
        let observed = match (gate_passed, quality_score) {
            (Some(_), Some(_)) => Observed::Both,
            (Some(_), None) => Observed::Gate,
            (None, Some(_)) => Observed::Quality,
            (None, None) => return,
        };
        if weight.is_nan() || weight <= 0.0 {
            return;
        }
        let size = self.window_size as f64;
        let window = self
            .observations
            .entry(model_slug.to_owned())
            .or_insert_with(|| VecDeque::with_capacity(self.window_size));
        window.push_back(GamingObservation {
            gate_passed: gate_passed.unwrap_or(false),
            quality_score: quality_score.unwrap_or(0.0),
            timestamp: Utc::now(),
            weight,
            observed,
        });
        let mut total: f64 = window.iter().map(|observation| observation.weight).sum();
        while let Some(front) = window.front()
            && total - front.weight >= size
        {
            total -= front.weight;
            window.pop_front();
        }
    }

    /// Check whether the current window for `model_slug` shows gaming.
    ///
    /// Returns `None` when the window is not yet full or no gaming is detected.
    /// The check does **not** write to disk; use
    /// [`observe_and_detect`][Self::observe_and_detect] for the combined path.
    #[must_use]
    pub fn detect(&self, model_slug: &str) -> Option<GamingAlert> {
        let window = self.observations.get(model_slug)?;
        let total: f64 = window.iter().map(|observation| observation.weight).sum();

        // Require a full window before making any comparison.
        if total < self.window_size.max(MIN_WINDOW_FOR_DETECTION) as f64 {
            return None;
        }

        // The halves split the window's weight.
        let observations: Vec<&GamingObservation> = window.iter().collect();
        let mut cumulative = 0.0;
        let half = observations
            .iter()
            .position(|observation| {
                cumulative += observation.weight;
                cumulative > total / 2.0
            })
            .unwrap_or(observations.len());

        let first_half = &observations[..half];
        let second_half = &observations[half..];

        let first_half_pass_rate = pass_rate(first_half);
        let second_half_pass_rate = pass_rate(second_half);
        let first_half_quality = avg_quality(first_half);
        let second_half_quality = avg_quality(second_half);

        let pass_rate_delta = second_half_pass_rate - first_half_pass_rate;
        let quality_delta = second_half_quality - first_half_quality;

        if pass_rate_delta > PASS_RATE_RISE_THRESHOLD && quality_delta < -QUALITY_FALL_THRESHOLD {
            Some(GamingAlert {
                model_slug: model_slug.to_owned(),
                pass_rate_delta,
                quality_delta,
                first_half_pass_rate,
                second_half_pass_rate,
                first_half_quality,
                second_half_quality,
                timestamp: Utc::now(),
            })
        } else {
            None
        }
    }

    /// Add an observation and immediately check for gaming.
    ///
    /// When gaming is detected the alert is:
    /// 1. Emitted as a structured `warn!` log.
    /// 2. Appended to the JSONL alert file asynchronously.
    ///
    /// # Errors
    ///
    /// Returns an error if the JSONL append fails.  The in-memory state is
    /// always updated regardless of I/O outcome.
    pub async fn observe_and_detect(
        &mut self,
        model_slug: &str,
        gate_passed: bool,
        quality_score: f64,
    ) -> io::Result<Option<GamingAlert>> {
        self.observe(model_slug, gate_passed, quality_score);

        let alert = match self.detect(model_slug) {
            Some(alert) => alert,
            None => return Ok(None),
        };

        warn!(
            model_slug = %alert.model_slug,
            pass_rate_delta = alert.pass_rate_delta,
            quality_delta = alert.quality_delta,
            first_half_pass_rate = alert.first_half_pass_rate,
            second_half_pass_rate = alert.second_half_pass_rate,
            first_half_quality = alert.first_half_quality,
            second_half_quality = alert.second_half_quality,
            "{}",
            alert.summary(),
        );

        self.append_alert(&alert).await?;

        Ok(Some(alert))
    }

    /// Append one [`GamingAlert`] as a JSON line to the alert file.
    ///
    /// The parent directory is created if it does not exist.
    ///
    /// # Errors
    ///
    /// Returns an error for serialization or file I/O failures.
    pub async fn append_alert(&self, alert: &GamingAlert) -> io::Result<()> {
        if let Some(parent) = self.alerts_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let mut line = serde_json::to_string(alert)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        line.push('\n');

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.alerts_path)
            .await?;

        file.write_all(line.as_bytes()).await?;
        file.flush().await?;
        file.sync_data().await?;

        Ok(())
    }

    /// Read all alerts from the JSONL file; malformed lines are skipped.
    ///
    /// # Errors
    ///
    /// Returns an error only for file open/read failures.
    pub async fn read_alerts(&self) -> io::Result<Vec<GamingAlert>> {
        read_gaming_alerts(&self.alerts_path).await
    }

    /// Return the number of observations currently buffered for `model_slug`.
    #[must_use]
    pub fn observation_count(&self, model_slug: &str) -> usize {
        self.observations
            .get(model_slug)
            .map(VecDeque::len)
            .unwrap_or(0)
    }
}

/// Read gaming alerts from a JSONL file.
///
/// Missing files produce an empty vector; malformed lines are skipped.
///
/// # Errors
///
/// Returns an error only for file open/read failures.
pub async fn read_gaming_alerts(path: &Path) -> io::Result<Vec<GamingAlert>> {
    use tokio::io::{AsyncBufReadExt, BufReader};

    let file = match tokio::fs::File::open(path).await {
        Ok(file) => file,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err),
    };

    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    let mut out = Vec::new();
    while let Some(line) = lines.next_line().await? {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(alert) = serde_json::from_str::<GamingAlert>(trimmed) {
            out.push(alert);
        }
    }

    Ok(out)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn pass_rate(obs: &[&GamingObservation]) -> f64 {
    weighted_mean(
        obs.iter()
            .filter(|o| o.observed != Observed::Quality)
            .map(|o| (o.weight, f64::from(u8::from(o.gate_passed)))),
    )
}

fn avg_quality(obs: &[&GamingObservation]) -> f64 {
    weighted_mean(
        obs.iter()
            .filter(|o| o.observed != Observed::Gate)
            .map(|o| (o.weight, o.quality_score)),
    )
}

/// The weighted mean of `(weight, value)` pairs; NaN without any weight, so
/// a half with nothing to compare raises no alert.
fn weighted_mean(pairs: impl Iterator<Item = (f64, f64)>) -> f64 {
    let (weight, sum) = pairs.fold((0.0, 0.0), |(weight, sum), (w, value)| {
        (weight + w, w.mul_add(value, sum))
    });
    if weight > 0.0 { sum / weight } else { f64::NAN }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    fn make_detector(tmp: &TempDir) -> GateGamingDetector {
        let path = tmp.path().join("gate-gaming-alerts.jsonl");
        GateGamingDetector::new(path)
    }

    // Fill the first half of the window with normal observations, then fill
    // the second half simulating gaming (pass rate up, quality down).
    fn fill_gaming_pattern(
        detector: &mut GateGamingDetector,
        model: &str,
        window_size: usize,
        first_half_pass_rate: f64,
        first_half_quality: f64,
        second_half_pass_rate: f64,
        second_half_quality: f64,
    ) {
        let half = window_size / 2;

        // First half: lower pass rate, higher quality.
        for i in 0..half {
            let gate_passed = (i as f64 / half as f64) < first_half_pass_rate;
            detector.observe(model, gate_passed, first_half_quality);
        }

        // Second half: higher pass rate, lower quality.
        for i in 0..half {
            let gate_passed = (i as f64 / half as f64) < second_half_pass_rate;
            detector.observe(model, gate_passed, second_half_quality);
        }
    }

    // -----------------------------------------------------------------------
    // Normal operation — no alert
    // -----------------------------------------------------------------------

    #[test]
    fn no_alert_for_stable_pass_rate_and_quality() {
        let tmp = TempDir::new().expect("tempdir");
        let mut detector = make_detector(&tmp);
        let model = "claude-sonnet-4-6";

        fill_gaming_pattern(
            &mut detector,
            model,
            DEFAULT_WINDOW_SIZE,
            0.7,
            0.75,
            0.7,
            0.75,
        );

        let alert = detector.detect(model);
        assert!(
            alert.is_none(),
            "expected no alert for stable metrics, got {alert:?}"
        );
    }

    #[test]
    fn no_alert_when_quality_rises_with_pass_rate() {
        let tmp = TempDir::new().expect("tempdir");
        let mut detector = make_detector(&tmp);
        let model = "gpt-4o";

        // Both improve — this is genuine improvement, not gaming.
        fill_gaming_pattern(
            &mut detector,
            model,
            DEFAULT_WINDOW_SIZE,
            0.5,
            0.6,
            0.75,
            0.8,
        );

        let alert = detector.detect(model);
        assert!(alert.is_none(), "improvement is not gaming: {alert:?}");
    }

    #[test]
    fn no_alert_when_pass_rate_rise_is_below_threshold() {
        let tmp = TempDir::new().expect("tempdir");
        let mut detector = make_detector(&tmp);
        let model = "gemini-2.0-flash";

        // Pass rate rises only ~10pp (below 15pp threshold), quality falls ~12pp.
        fill_gaming_pattern(
            &mut detector,
            model,
            DEFAULT_WINDOW_SIZE,
            0.6,
            0.7,
            0.7,
            0.58,
        );

        let alert = detector.detect(model);
        assert!(
            alert.is_none(),
            "10pp pass-rate rise should not trigger alert: {alert:?}"
        );
    }

    #[test]
    fn no_alert_when_quality_fall_is_below_threshold() {
        let tmp = TempDir::new().expect("tempdir");
        let mut detector = make_detector(&tmp);
        let model = "llama-3.3-70b";

        // Pass rate rises ~20pp but quality only falls ~5pp (below 10pp threshold).
        fill_gaming_pattern(
            &mut detector,
            model,
            DEFAULT_WINDOW_SIZE,
            0.5,
            0.7,
            0.7,
            0.65,
        );

        let alert = detector.detect(model);
        assert!(
            alert.is_none(),
            "5pp quality fall should not trigger alert: {alert:?}"
        );
    }

    // -----------------------------------------------------------------------
    // Gaming detected
    // -----------------------------------------------------------------------

    #[test]
    fn alert_fires_when_pass_rate_rises_and_quality_falls() {
        let tmp = TempDir::new().expect("tempdir");
        let mut detector = make_detector(&tmp);
        let model = "suspicious-model-v1";

        // Pass rate rises ~30pp, quality falls ~20pp — clearly gaming.
        fill_gaming_pattern(
            &mut detector,
            model,
            DEFAULT_WINDOW_SIZE,
            0.4,
            0.75,
            0.7,
            0.55,
        );

        let alert = detector.detect(model).expect("expected gaming alert");
        assert_eq!(alert.model_slug, model);
        assert!(
            alert.pass_rate_delta > PASS_RATE_RISE_THRESHOLD,
            "pass_rate_delta={} must be > {}",
            alert.pass_rate_delta,
            PASS_RATE_RISE_THRESHOLD
        );
        assert!(
            alert.quality_delta < -QUALITY_FALL_THRESHOLD,
            "quality_delta={} must be < -{}",
            alert.quality_delta,
            QUALITY_FALL_THRESHOLD
        );
    }

    #[test]
    fn alert_fields_reflect_correct_halves() {
        let tmp = TempDir::new().expect("tempdir");
        let mut detector = make_detector(&tmp);
        let model = "gamer-model";

        // First half: all gates pass (1.0), quality = 0.9.
        // Second half: all gates pass (1.0), quality = 0.5.
        // Pass rate delta = 0, so this should NOT trigger (quality alone is not enough).
        let half = DEFAULT_WINDOW_SIZE / 2;
        for _ in 0..half {
            detector.observe(model, true, 0.9);
        }
        for _ in 0..half {
            detector.observe(model, true, 0.5);
        }
        // No pass rate rise → no gaming.
        assert!(
            detector.detect(model).is_none(),
            "equal pass rates should not trigger"
        );
    }

    #[test]
    fn alert_fires_at_exactly_threshold_boundaries() {
        let tmp = TempDir::new().expect("tempdir");
        let mut detector = make_detector(&tmp);
        let model = "boundary-model";

        // Construct observations where pass_rate_delta is just above 0.15
        // and quality_delta is just below -0.10.
        let half = DEFAULT_WINDOW_SIZE / 2;

        // First half: 40% pass rate, quality = 0.70.
        for i in 0..half {
            let gate_passed = i < (half * 2 / 5); // 40%
            detector.observe(model, gate_passed, 0.70);
        }
        // Second half: 60% pass rate (+20pp), quality = 0.55 (-15pp).
        for i in 0..half {
            let gate_passed = i < (half * 3 / 5); // 60%
            detector.observe(model, gate_passed, 0.55);
        }

        let alert = detector
            .detect(model)
            .expect("should detect gaming at threshold");
        assert!(alert.pass_rate_delta > PASS_RATE_RISE_THRESHOLD);
        assert!(alert.quality_delta < -QUALITY_FALL_THRESHOLD);
    }

    // -----------------------------------------------------------------------
    // Window too small — no alert
    // -----------------------------------------------------------------------

    #[test]
    fn no_alert_when_window_not_full() {
        let tmp = TempDir::new().expect("tempdir");
        let mut detector = make_detector(&tmp);
        let model = "starving-model";

        // Add only half the required observations.
        for _ in 0..(DEFAULT_WINDOW_SIZE / 2) {
            detector.observe(model, true, 0.1);
        }

        let alert = detector.detect(model);
        assert!(
            alert.is_none(),
            "should not fire with insufficient observations: {alert:?}"
        );
    }

    #[test]
    fn no_alert_for_unknown_model() {
        let tmp = TempDir::new().expect("tempdir");
        let detector = make_detector(&tmp);
        assert!(detector.detect("does-not-exist").is_none());
    }

    #[test]
    fn observation_count_tracks_correctly() {
        let tmp = TempDir::new().expect("tempdir");
        let mut detector = make_detector(&tmp);
        let model = "count-model";

        assert_eq!(detector.observation_count(model), 0);

        for i in 1..=5 {
            detector.observe(model, true, 0.8);
            assert_eq!(detector.observation_count(model), i);
        }
    }

    // -----------------------------------------------------------------------
    // Window sliding — old observations evicted
    // -----------------------------------------------------------------------

    #[test]
    fn window_slides_and_evicts_old_observations() {
        let tmp = TempDir::new().expect("tempdir");
        let mut detector = make_detector(&tmp);
        let model = "slide-model";

        // Fill a full gaming pattern (should trigger).
        fill_gaming_pattern(
            &mut detector,
            model,
            DEFAULT_WINDOW_SIZE,
            0.3,
            0.80,
            0.65,
            0.55,
        );
        assert!(
            detector.detect(model).is_some(),
            "should detect gaming before slide"
        );

        // Now slide the window: push `window_size` observations with stable,
        // high-quality, consistent pass rate — gaming should no longer be detected.
        let half = DEFAULT_WINDOW_SIZE / 2;
        for _ in 0..half {
            detector.observe(model, true, 0.85);
        }
        for _ in 0..half {
            detector.observe(model, true, 0.85);
        }

        let alert = detector.detect(model);
        assert!(
            alert.is_none(),
            "after stable slide, gaming should not be detected: {alert:?}"
        );
    }

    // -----------------------------------------------------------------------
    // JSONL persistence
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn observe_and_detect_writes_alert_to_jsonl() {
        let tmp = TempDir::new().expect("tempdir");
        let mut detector = make_detector(&tmp);
        let model = "jsonl-model";

        fill_gaming_pattern(
            &mut detector,
            model,
            DEFAULT_WINDOW_SIZE,
            0.3,
            0.80,
            0.65,
            0.55,
        );

        // One final observation triggers detection + write.
        let alert = detector
            .observe_and_detect(model, true, 0.50)
            .await
            .expect("io should succeed");

        assert!(
            alert.is_some(),
            "expected gaming alert from observe_and_detect"
        );

        let alerts = detector
            .read_alerts()
            .await
            .expect("should read back alerts");
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].model_slug, model);
    }

    #[tokio::test]
    async fn read_gaming_alerts_returns_empty_for_missing_file() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("nonexistent.jsonl");
        let alerts = read_gaming_alerts(&path).await.expect("should not error");
        assert!(alerts.is_empty());
    }

    #[tokio::test]
    async fn read_gaming_alerts_skips_malformed_lines() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("alerts.jsonl");

        let alert = GamingAlert {
            model_slug: "m".to_string(),
            pass_rate_delta: 0.2,
            quality_delta: -0.15,
            first_half_pass_rate: 0.4,
            second_half_pass_rate: 0.6,
            first_half_quality: 0.75,
            second_half_quality: 0.60,
            timestamp: Utc::now(),
        };

        let good_line = serde_json::to_string(&alert).expect("serialize");
        let content = format!("{good_line}\n{{bad json\n{good_line}\n");
        tokio::fs::write(&path, content).await.expect("write");

        let alerts = read_gaming_alerts(&path).await.expect("read");
        assert_eq!(alerts.len(), 2);
    }

    // -----------------------------------------------------------------------
    // GamingAlert helper
    // -----------------------------------------------------------------------

    #[test]
    fn gaming_alert_summary_contains_key_fields() {
        let alert = GamingAlert {
            model_slug: "test-model".to_string(),
            pass_rate_delta: 0.20,
            quality_delta: -0.15,
            first_half_pass_rate: 0.40,
            second_half_pass_rate: 0.60,
            first_half_quality: 0.75,
            second_half_quality: 0.60,
            timestamp: Utc::now(),
        };

        let summary = alert.summary();
        assert!(summary.contains("test-model"), "summary: {summary}");
        assert!(summary.contains("20.0"), "summary: {summary}");
        assert!(summary.contains("15.0"), "summary: {summary}");
    }

    // -----------------------------------------------------------------------
    // Custom window size
    // -----------------------------------------------------------------------

    #[test]
    fn custom_window_size_respected() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("alerts.jsonl");
        let window = 10;
        let mut detector = GateGamingDetector::new(path).with_window_size(window);

        assert_eq!(detector.window_size(), window);

        let model = "small-window";

        // Fill gaming pattern in the smaller window.
        fill_gaming_pattern(&mut detector, model, window, 0.3, 0.80, 0.65, 0.55);

        // Observation count should not exceed the window.
        assert_eq!(detector.observation_count(model), window);
    }

    #[test]
    fn window_size_minimum_is_two() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("alerts.jsonl");
        let detector = GateGamingDetector::new(path).with_window_size(0);
        assert_eq!(detector.window_size(), 2);
    }
}
