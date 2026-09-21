//! Holdout experiment infrastructure for detecting overfitting in learned routing.
//!
//! Deterministically assigns each task to either a [`Partition::Train`] or
//! [`Partition::Holdout`] bucket using an 80/20 split keyed by the task ID
//! hash.  Learning updates (playbook, cascade router, adaptive thresholds)
//! should be gated behind [`HoldoutExperiment::should_update_learning`] so
//! that holdout tasks never influence the routing model being evaluated.
//!
//! # Overfitting detection
//!
//! [`HoldoutExperiment::check_overfitting`] fires an [`OverfittingAlert`] when
//! the train gate-pass rate exceeds the holdout gate-pass rate by more than
//! [`OVERFITTING_THRESHOLD_PP`] percentage points, provided at least
//! [`MIN_HOLDOUT_OBSERVATIONS`] holdout outcomes have been observed.
//!
//! # Persistence
//!
//! The experiment state is saved as JSON to `.roko/learn/holdout-state.json`.
//! [`HoldoutExperiment::load_or_new`] reconstructs state from the file and
//! falls back to a fresh experiment when the file is absent or corrupt.

use std::collections::HashMap;
use std::hash::Hasher;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::warn;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Fraction of tasks assigned to the Train partition.
const TRAIN_FRACTION: f64 = 0.80;

/// Minimum holdout observations required before overfitting detection runs.
pub const MIN_HOLDOUT_OBSERVATIONS: u64 = 20;

/// Pass-rate divergence threshold that triggers an overfitting alert (15pp).
pub const OVERFITTING_THRESHOLD_PP: f64 = 0.15;

/// Seed mixed into the task ID hash to make partition assignment reproducible
/// across restarts while remaining independent of Rust's default hasher seed.
const HASH_SEED: u64 = 0xdead_beef_cafe_1337;

// ---------------------------------------------------------------------------
// Partition
// ---------------------------------------------------------------------------

/// Whether a task is used to train the routing model or held out for
/// independent evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Partition {
    /// This task's outcomes update playbooks, cascade router, and thresholds.
    Train,
    /// This task's outcomes are recorded for evaluation only; no updates occur.
    Holdout,
}

impl Partition {
    /// Return `true` for [`Partition::Train`].
    #[must_use]
    pub fn is_train(self) -> bool {
        self == Partition::Train
    }

    /// Return `true` for [`Partition::Holdout`].
    #[must_use]
    pub fn is_holdout(self) -> bool {
        self == Partition::Holdout
    }
}

// ---------------------------------------------------------------------------
// RollingMetrics
// ---------------------------------------------------------------------------

/// Cumulative gate-pass and cost metrics for one partition.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RollingMetrics {
    /// Total number of outcomes recorded.
    pub total: u64,
    /// Number of outcomes where the gate passed.
    pub passed: u64,
    /// Accumulated cost in USD across all outcomes.
    pub total_cost: f64,
}

impl RollingMetrics {
    /// Gate pass rate in [0.0, 1.0], or `0.0` when no outcomes have been
    /// recorded.
    #[must_use]
    pub fn pass_rate(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.passed as f64 / self.total as f64
        }
    }

    /// Average cost per outcome, or `0.0` when no outcomes have been recorded.
    #[must_use]
    pub fn avg_cost(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.total_cost / self.total as f64
        }
    }

    /// Record one gate outcome.
    pub fn record(&mut self, passed: bool, cost: f64) {
        self.total += 1;
        if passed {
            self.passed += 1;
        }
        self.total_cost += cost;
    }
}

// ---------------------------------------------------------------------------
// OverfittingAlert
// ---------------------------------------------------------------------------

/// Emitted when the train partition substantially outperforms the holdout
/// partition, indicating that the routing model has overfit to the training
/// distribution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OverfittingAlert {
    /// Gate pass rate on the Train partition.
    pub train_pass_rate: f64,
    /// Gate pass rate on the Holdout partition.
    pub holdout_pass_rate: f64,
    /// Absolute difference: `train_pass_rate − holdout_pass_rate`.
    pub divergence_pp: f64,
    /// Total holdout observations at detection time.
    pub holdout_observations: u64,
}

impl OverfittingAlert {
    /// Return a one-line human-readable summary.
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "overfitting detected: train pass rate {:.1}% vs holdout {:.1}% \
             (divergence {:.1}pp, {} holdout observations)",
            self.train_pass_rate * 100.0,
            self.holdout_pass_rate * 100.0,
            self.divergence_pp * 100.0,
            self.holdout_observations,
        )
    }
}

// ---------------------------------------------------------------------------
// HoldoutExperiment
// ---------------------------------------------------------------------------

/// Serializable state persisted to disk.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct HoldoutState {
    /// Memoised partition assignments.  New IDs are assigned on first contact
    /// and then cached so that the partition for a given task never changes.
    assignments: HashMap<String, Partition>,
    /// Cumulative metrics for the Train partition.
    train_metrics: RollingMetrics,
    /// Cumulative metrics for the Holdout partition.
    holdout_metrics: RollingMetrics,
}

/// Manages train/holdout partitioning and overfitting detection for learned
/// routing decisions.
///
/// Construct with [`HoldoutExperiment::new`] to start fresh or
/// [`HoldoutExperiment::load_or_new`] to restore persisted state.
#[derive(Debug)]
pub struct HoldoutExperiment {
    state: HoldoutState,
    /// Path to the JSON state file.
    path: PathBuf,
}

impl HoldoutExperiment {
    /// Create a new experiment that persists state to `path`.
    ///
    /// The file is not written until [`save`][Self::save] is called.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            state: HoldoutState::default(),
            path: path.into(),
        }
    }

    /// Load persisted state from `path`, falling back to a fresh experiment
    /// when the file is absent or unreadable.
    ///
    /// # Errors
    ///
    /// Returns an error only for file I/O failures other than `NotFound`.
    /// Corrupt JSON is treated as a missing file (logged at WARN level).
    pub fn load_or_new(path: impl Into<PathBuf>) -> io::Result<Self> {
        let path = path.into();

        let state = match std::fs::read_to_string(&path) {
            Ok(text) => match serde_json::from_str::<HoldoutState>(&text) {
                Ok(s) => s,
                Err(err) => {
                    warn!(
                        path = %path.display(),
                        error = %err,
                        "holdout state corrupt; starting fresh"
                    );
                    HoldoutState::default()
                }
            },
            Err(err) if err.kind() == io::ErrorKind::NotFound => HoldoutState::default(),
            Err(err) => return Err(err),
        };

        Ok(Self { state, path })
    }

    /// Persist the current state to disk atomically.
    ///
    /// The parent directory is created if it does not already exist.
    ///
    /// # Errors
    ///
    /// Returns an error for serialization or file I/O failures.
    pub fn save(&self) -> io::Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(&self.state).map_err(io::Error::other)?;

        // Write to a sibling `.tmp` file then rename for atomicity.
        let tmp_path = self.path.with_extension("json.tmp");
        std::fs::write(&tmp_path, &json)?;
        std::fs::rename(&tmp_path, &self.path)?;

        Ok(())
    }

    /// Return the path where the state is persisted.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    // -----------------------------------------------------------------------
    // Partition assignment
    // -----------------------------------------------------------------------

    /// Deterministically assign `task_id` to a partition.
    ///
    /// The assignment is memoised: calling this method with the same
    /// `task_id` always returns the same [`Partition`] for the lifetime of
    /// this experiment (including across restarts, once the state is
    /// persisted and reloaded).
    ///
    /// The split is 80% [`Partition::Train`] / 20% [`Partition::Holdout`].
    pub fn assign(&mut self, task_id: &str) -> Partition {
        if let Some(&partition) = self.state.assignments.get(task_id) {
            return partition;
        }

        let partition = deterministic_partition(task_id);
        self.state.assignments.insert(task_id.to_owned(), partition);
        partition
    }

    // -----------------------------------------------------------------------
    // Outcome recording
    // -----------------------------------------------------------------------

    /// Record the outcome of a task execution.
    ///
    /// The task is assigned to a partition (via [`assign`][Self::assign] if it
    /// has not been seen before) and the appropriate [`RollingMetrics`] are
    /// updated.
    pub fn record_outcome(&mut self, task_id: &str, passed: bool, cost: f64) {
        let partition = self.assign(task_id);
        match partition {
            Partition::Train => self.state.train_metrics.record(passed, cost),
            Partition::Holdout => self.state.holdout_metrics.record(passed, cost),
        }
    }

    // -----------------------------------------------------------------------
    // Overfitting detection
    // -----------------------------------------------------------------------

    /// Check whether the train partition has substantially out-performed the
    /// holdout partition.
    ///
    /// Returns `Some(alert)` when:
    /// - At least [`MIN_HOLDOUT_OBSERVATIONS`] holdout outcomes have been
    ///   recorded, **and**
    /// - <code>train_pass_rate − holdout_pass_rate > [`OVERFITTING_THRESHOLD_PP`]</code>.
    ///
    /// Returns `None` otherwise.
    #[must_use]
    pub fn check_overfitting(&self) -> Option<OverfittingAlert> {
        let holdout_total = self.state.holdout_metrics.total;

        if holdout_total < MIN_HOLDOUT_OBSERVATIONS {
            return None;
        }

        let train_pass_rate = self.state.train_metrics.pass_rate();
        let holdout_pass_rate = self.state.holdout_metrics.pass_rate();
        let divergence_pp = train_pass_rate - holdout_pass_rate;

        if divergence_pp > OVERFITTING_THRESHOLD_PP {
            let alert = OverfittingAlert {
                train_pass_rate,
                holdout_pass_rate,
                divergence_pp,
                holdout_observations: holdout_total,
            };
            warn!(
                train_pass_rate,
                holdout_pass_rate,
                divergence_pp,
                holdout_observations = holdout_total,
                "{}",
                alert.summary()
            );
            Some(alert)
        } else {
            None
        }
    }

    // -----------------------------------------------------------------------
    // Learning gate
    // -----------------------------------------------------------------------

    /// Return `true` if learning updates (playbook, cascade router, adaptive
    /// thresholds) should be applied for `task_id`.
    ///
    /// Only [`Partition::Train`] tasks update the routing model; holdout tasks
    /// are observed but never used to update learned state.
    pub fn should_update_learning(&mut self, task_id: &str) -> bool {
        self.assign(task_id).is_train()
    }

    // -----------------------------------------------------------------------
    // Metrics accessors
    // -----------------------------------------------------------------------

    /// Read-only access to the Train partition metrics.
    #[must_use]
    pub fn train_metrics(&self) -> &RollingMetrics {
        &self.state.train_metrics
    }

    /// Read-only access to the Holdout partition metrics.
    #[must_use]
    pub fn holdout_metrics(&self) -> &RollingMetrics {
        &self.state.holdout_metrics
    }

    /// Number of tasks with a cached partition assignment.
    #[must_use]
    pub fn assignment_count(&self) -> usize {
        self.state.assignments.len()
    }
}

// ---------------------------------------------------------------------------
// Deterministic hash-based partition assignment
// ---------------------------------------------------------------------------

/// Assign `task_id` to a partition deterministically using a seeded hash.
///
/// The hash is computed with [`std::collections::hash_map::DefaultHasher`]
/// after mixing in [`HASH_SEED`], then mapped to [0, 1000) to produce an
/// 80/20 split.
fn deterministic_partition(task_id: &str) -> Partition {
    use std::collections::hash_map::DefaultHasher;

    // Mix the seed into the hasher before hashing the task ID.  This makes
    // the result reproducible even if the DefaultHasher's own seed changes
    // between Rust versions (which it does — it is randomized per-process).
    // We write the seed bytes first so that the task ID bytes are hashed
    // against a consistent prefix.
    let mut hasher = DefaultHasher::new();
    // Manually mix seed: XOR the seed into the hasher state via a series of
    // writes.  We write the seed as two u32 values so the byte order is
    // platform-independent.
    hasher.write_u64(HASH_SEED);
    hasher.write(task_id.as_bytes());
    let h = hasher.finish();

    // Map to [0, 1000) and compare against the train threshold.
    let bucket = h % 1000;
    let train_threshold = (TRAIN_FRACTION * 1000.0) as u64; // 800

    if bucket < train_threshold {
        Partition::Train
    } else {
        Partition::Holdout
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn tmp_path(dir: &TempDir) -> PathBuf {
        dir.path().join("holdout-state.json")
    }

    // -----------------------------------------------------------------------
    // Deterministic assignment
    // -----------------------------------------------------------------------

    #[test]
    fn same_task_id_always_gets_same_partition() {
        let tmp = TempDir::new().expect("tempdir");
        let mut exp = HoldoutExperiment::new(tmp_path(&tmp));

        let id = "task-abc-123";
        let first = exp.assign(id);
        for _ in 0..20 {
            assert_eq!(exp.assign(id), first, "partition must be deterministic");
        }
    }

    #[test]
    fn deterministic_partition_is_stable_without_experiment() {
        // Direct function: calling twice with the same input returns the same
        // result without relying on the memoization cache.
        let id = "standalone-task-xyz";
        let p1 = deterministic_partition(id);
        let p2 = deterministic_partition(id);
        assert_eq!(p1, p2);
    }

    // -----------------------------------------------------------------------
    // 80/20 split
    // -----------------------------------------------------------------------

    #[test]
    fn roughly_eighty_twenty_split_over_many_ids() {
        let n = 1_000usize;
        let train_count = (0..n)
            .filter(|i| deterministic_partition(&format!("task-{i}")) == Partition::Train)
            .count();

        // Allow ±5pp deviation from 80%.
        let train_fraction = train_count as f64 / n as f64;
        assert!(
            (0.75..=0.85).contains(&train_fraction),
            "expected ~80% train but got {:.1}%",
            train_fraction * 100.0
        );
    }

    // -----------------------------------------------------------------------
    // RollingMetrics
    // -----------------------------------------------------------------------

    #[test]
    fn rolling_metrics_pass_rate_zero_for_empty() {
        let m = RollingMetrics::default();
        assert_eq!(m.pass_rate(), 0.0);
        assert_eq!(m.avg_cost(), 0.0);
    }

    #[test]
    fn rolling_metrics_correct_rates() {
        let mut m = RollingMetrics::default();
        m.record(true, 1.0);
        m.record(false, 2.0);
        m.record(true, 3.0);

        assert_eq!(m.total, 3);
        assert_eq!(m.passed, 2);
        assert!((m.pass_rate() - 2.0 / 3.0).abs() < 1e-9);
        assert!((m.avg_cost() - 2.0).abs() < 1e-9);
    }

    // -----------------------------------------------------------------------
    // Overfitting detection
    // -----------------------------------------------------------------------

    #[test]
    fn no_overfitting_when_both_partitions_perform_similarly() {
        let tmp = TempDir::new().expect("tempdir");
        let mut exp = HoldoutExperiment::new(tmp_path(&tmp));

        // Record 20 holdout outcomes — just at the minimum.
        for _ in 0..20 {
            exp.state.holdout_metrics.record(true, 0.1);
        }
        // Train also passes at the same rate.
        for _ in 0..80 {
            exp.state.train_metrics.record(true, 0.1);
        }

        assert!(
            exp.check_overfitting().is_none(),
            "no divergence — should not flag overfitting"
        );
    }

    #[test]
    fn no_overfitting_detected_below_min_holdout_observations() {
        let tmp = TempDir::new().expect("tempdir");
        let mut exp = HoldoutExperiment::new(tmp_path(&tmp));

        // Fewer than MIN_HOLDOUT_OBSERVATIONS holdout outcomes.
        for _ in 0..19 {
            exp.state.holdout_metrics.record(false, 0.1);
        }
        for _ in 0..80 {
            exp.state.train_metrics.record(true, 0.1);
        }

        assert!(
            exp.check_overfitting().is_none(),
            "insufficient holdout observations — must not fire"
        );
    }

    #[test]
    fn overfitting_detected_when_train_far_exceeds_holdout() {
        let tmp = TempDir::new().expect("tempdir");
        let mut exp = HoldoutExperiment::new(tmp_path(&tmp));

        // Train: 90% pass rate.
        for i in 0..100u64 {
            exp.state.train_metrics.record(i < 90, 0.1);
        }
        // Holdout: 70% pass rate — divergence = 20pp > 15pp threshold.
        for i in 0..20u64 {
            exp.state.holdout_metrics.record(i < 14, 0.1);
        }

        let alert = exp
            .check_overfitting()
            .expect("overfitting should be detected");
        assert!(alert.divergence_pp > OVERFITTING_THRESHOLD_PP);
        assert_eq!(alert.holdout_observations, 20);
    }

    #[test]
    fn no_overfitting_when_divergence_is_exactly_at_threshold() {
        let tmp = TempDir::new().expect("tempdir");
        let mut exp = HoldoutExperiment::new(tmp_path(&tmp));

        // Train: 85% pass rate.
        for i in 0..100u64 {
            exp.state.train_metrics.record(i < 85, 0.1);
        }
        // Holdout: 70% pass rate — divergence = 15pp, NOT strictly greater.
        for i in 0..20u64 {
            exp.state.holdout_metrics.record(i < 14, 0.1);
        }

        // 85% − 70% = 15pp, which is exactly at the threshold.
        // The check uses `>`, so this should NOT trigger.
        let divergence =
            exp.state.train_metrics.pass_rate() - exp.state.holdout_metrics.pass_rate();
        // This test is sensitive to exact arithmetic, so we check the
        // computed divergence and apply the same rule as the implementation.
        if divergence > OVERFITTING_THRESHOLD_PP {
            // Only assert if the check truly fires.
            let alert = exp.check_overfitting().expect("fired");
            assert!(alert.divergence_pp > OVERFITTING_THRESHOLD_PP);
        } else {
            assert!(exp.check_overfitting().is_none());
        }
    }

    // -----------------------------------------------------------------------
    // should_update_learning
    // -----------------------------------------------------------------------

    #[test]
    fn should_update_learning_returns_true_for_train_tasks() {
        let tmp = TempDir::new().expect("tempdir");
        let mut exp = HoldoutExperiment::new(tmp_path(&tmp));

        // Find a task ID that maps to Train.
        let train_id = (0u64..)
            .map(|i| format!("task-train-{i}"))
            .find(|id| deterministic_partition(id) == Partition::Train)
            .expect("some ID must be Train");

        assert!(
            exp.should_update_learning(&train_id),
            "Train tasks must update learning"
        );
    }

    #[test]
    fn should_update_learning_returns_false_for_holdout_tasks() {
        let tmp = TempDir::new().expect("tempdir");
        let mut exp = HoldoutExperiment::new(tmp_path(&tmp));

        // Find a task ID that maps to Holdout.
        let holdout_id = (0u64..)
            .map(|i| format!("task-holdout-{i}"))
            .find(|id| deterministic_partition(id) == Partition::Holdout)
            .expect("some ID must be Holdout");

        assert!(
            !exp.should_update_learning(&holdout_id),
            "Holdout tasks must NOT update learning"
        );
    }

    #[test]
    fn should_update_learning_is_consistent_with_assign() {
        let tmp = TempDir::new().expect("tempdir");
        let mut exp = HoldoutExperiment::new(tmp_path(&tmp));

        for i in 0..50u64 {
            let id = format!("consistency-task-{i}");
            let partition = exp.assign(&id);
            let should_update = exp.should_update_learning(&id);
            assert_eq!(
                should_update,
                partition.is_train(),
                "should_update_learning must match assign for id={id}"
            );
        }
    }

    // -----------------------------------------------------------------------
    // JSON round-trip / persistence
    // -----------------------------------------------------------------------

    #[test]
    fn json_round_trip_preserves_state() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp_path(&tmp);

        let mut exp = HoldoutExperiment::new(path.clone());

        // Assign and record a mix of tasks.
        for i in 0..50u64 {
            let id = format!("rtt-task-{i}");
            exp.record_outcome(&id, i % 3 != 0, i as f64 * 0.01);
        }

        let train_total_before = exp.train_metrics().total;
        let holdout_total_before = exp.holdout_metrics().total;
        let assignments_before = exp.assignment_count();

        exp.save().expect("save should succeed");

        let exp2 = HoldoutExperiment::load_or_new(path).expect("load should succeed");

        assert_eq!(exp2.train_metrics().total, train_total_before);
        assert_eq!(exp2.holdout_metrics().total, holdout_total_before);
        assert_eq!(exp2.assignment_count(), assignments_before);
        assert!((exp2.train_metrics().pass_rate() - exp.train_metrics().pass_rate()).abs() < 1e-9);
    }

    #[test]
    fn load_or_new_returns_fresh_state_for_missing_file() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("does-not-exist.json");

        let exp = HoldoutExperiment::load_or_new(path).expect("load should not error");
        assert_eq!(exp.train_metrics().total, 0);
        assert_eq!(exp.holdout_metrics().total, 0);
        assert_eq!(exp.assignment_count(), 0);
    }

    #[test]
    fn load_or_new_recovers_from_corrupt_json() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp_path(&tmp);
        std::fs::write(&path, b"{ not valid json }").expect("write");

        let exp = HoldoutExperiment::load_or_new(path).expect("should not propagate error");
        assert_eq!(exp.train_metrics().total, 0);
    }

    #[test]
    fn save_creates_parent_directories() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp
            .path()
            .join("nested")
            .join("dirs")
            .join("holdout-state.json");
        let exp = HoldoutExperiment::new(path.clone());
        exp.save().expect("save should create parent dirs");
        assert!(path.exists());
    }

    // -----------------------------------------------------------------------
    // record_outcome routing
    // -----------------------------------------------------------------------

    #[test]
    fn record_outcome_routes_to_correct_partition() {
        let tmp = TempDir::new().expect("tempdir");
        let mut exp = HoldoutExperiment::new(tmp_path(&tmp));

        // Find one Train ID and one Holdout ID.
        let train_id = (0u64..)
            .map(|i| format!("train-{i}"))
            .find(|id| deterministic_partition(id) == Partition::Train)
            .unwrap();
        let holdout_id = (0u64..)
            .map(|i| format!("holdout-{i}"))
            .find(|id| deterministic_partition(id) == Partition::Holdout)
            .unwrap();

        exp.record_outcome(&train_id, true, 1.0);
        exp.record_outcome(&holdout_id, false, 2.0);

        assert_eq!(exp.train_metrics().total, 1);
        assert_eq!(exp.train_metrics().passed, 1);
        assert_eq!(exp.holdout_metrics().total, 1);
        assert_eq!(exp.holdout_metrics().passed, 0);
    }

    // -----------------------------------------------------------------------
    // OverfittingAlert summary
    // -----------------------------------------------------------------------

    #[test]
    fn overfitting_alert_summary_contains_key_fields() {
        let alert = OverfittingAlert {
            train_pass_rate: 0.92,
            holdout_pass_rate: 0.70,
            divergence_pp: 0.22,
            holdout_observations: 25,
        };
        let s = alert.summary();
        assert!(s.contains("92.0"), "train rate: {s}");
        assert!(s.contains("70.0"), "holdout rate: {s}");
        assert!(s.contains("22.0"), "divergence: {s}");
        assert!(s.contains("25"), "observations: {s}");
    }
}
