//! Shadow testing loop (Loop 12) — runs alternative configurations alongside
//! production tasks for safe A/B comparison.
//!
//! The [`ShadowRunner`] records paired outcomes from a production task and an
//! alternative ("shadow") configuration into `.roko/learn/shadow-results.jsonl`.
//! The runner integration point that actually forks a shadow dispatch is
//! intentionally **not** implemented here; that belongs in the plan runner.
//! This module owns the tracking infrastructure only.
//!
//! # Sampling
//!
//! [`ShadowRunner::should_shadow`] returns `true` every Nth call (configurable
//! via [`ShadowRunner::every_n`], default 5) using an atomic counter so it is
//! safe to call from concurrent contexts.
//!
//! # Superiority detection
//!
//! [`ShadowRunner::check_shadow_superiority`] fires a [`ShadowAlert`] when the
//! shadow configuration outperforms production by more than
//! [`SUPERIORITY_THRESHOLD_PP`] percentage points over at least
//! `min_results` (default [`DEFAULT_MIN_RESULTS`]) recorded pairs.
//!
//! # Runner integration note
//!
//! When the plan runner forks a shadow task it should:
//! 1. Call `should_shadow()` to decide whether to fork.
//! 2. Execute both the production and shadow tasks, recording wall-clock time
//!    and cost for each.
//! 3. Build a [`ShadowResult`] and call `record_result(result)`.
//! 4. Periodically call `check_shadow_superiority` and surface any
//!    [`ShadowAlert`] to the operator.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::fs::OpenOptions;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::warn;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Default sampling rate: run a shadow every 5th task.
pub const DEFAULT_EVERY_N: u32 = 5;

/// Default minimum number of paired results before superiority detection runs.
pub const DEFAULT_MIN_RESULTS: usize = 20;

/// Percentage-point margin above which shadow is considered superior.
///
/// A delta of 0.10 means the shadow pass rate must exceed the production pass
/// rate by more than 10 percentage points.
pub const SUPERIORITY_THRESHOLD_PP: f64 = 0.10;

// ---------------------------------------------------------------------------
// ShadowConfig
// ---------------------------------------------------------------------------

/// Which alternative model/prompt variant the shadow run uses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShadowConfig {
    /// Alternative model slug to dispatch the shadow task against.
    pub model_slug: String,
    /// Optional replacement for the system-prompt section used by the shadow.
    ///
    /// When `None` the shadow inherits the production system prompt and only
    /// the model differs.
    pub prompt_variant: Option<String>,
    /// Human-readable label for this shadow configuration, used in alerts and
    /// summaries (e.g. `"claude-opus-4-6-bare"`, `"flash-2.0-experimental"`).
    pub label: String,
}

// ---------------------------------------------------------------------------
// ShadowResult
// ---------------------------------------------------------------------------

/// A single paired outcome recorded after both production and shadow tasks
/// complete.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShadowResult {
    /// Stable task identifier within the plan.
    pub task_id: String,
    /// Plan the task belongs to.
    pub plan_id: String,
    /// Wall-clock time the pair was recorded.
    pub timestamp: DateTime<Utc>,
    /// Model used for the production run.
    pub production_model: String,
    /// Model used for the shadow run (from [`ShadowConfig::model_slug`]).
    pub shadow_model: String,
    /// Whether the production gate pipeline passed.
    pub production_passed: bool,
    /// Whether the shadow gate pipeline passed.
    pub shadow_passed: bool,
    /// Observed cost of the production run in USD.
    pub production_cost_usd: f64,
    /// Observed cost of the shadow run in USD.
    pub shadow_cost_usd: f64,
    /// Wall-clock duration of the production run in milliseconds.
    pub production_duration_ms: u64,
    /// Wall-clock duration of the shadow run in milliseconds.
    pub shadow_duration_ms: u64,
    /// Label copied from [`ShadowConfig::label`].
    pub label: String,
}

// ---------------------------------------------------------------------------
// ShadowAlert
// ---------------------------------------------------------------------------

/// Emitted when the shadow configuration has consistently outperformed
/// production by more than [`SUPERIORITY_THRESHOLD_PP`] percentage points.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShadowAlert {
    /// Gate pass rate of the shadow configuration in [0.0, 1.0].
    pub shadow_pass_rate: f64,
    /// Gate pass rate of the production configuration in [0.0, 1.0].
    pub production_pass_rate: f64,
    /// `shadow_pass_rate − production_pass_rate` (always positive when alert
    /// fires).
    pub delta_pp: f64,
    /// Number of paired results used to compute the rates.
    pub result_count: usize,
    /// Human-readable next-step recommendation.
    pub recommendation: String,
}

impl ShadowAlert {
    /// Return a one-line human-readable summary.
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "shadow superiority: shadow pass rate {:.1}% vs production {:.1}% \
             (delta +{:.1}pp, {} results) — {}",
            self.shadow_pass_rate * 100.0,
            self.production_pass_rate * 100.0,
            self.delta_pp * 100.0,
            self.result_count,
            self.recommendation,
        )
    }
}

// ---------------------------------------------------------------------------
// ShadowSummary
// ---------------------------------------------------------------------------

/// Aggregate statistics computed over all recorded [`ShadowResult`]s.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShadowSummary {
    /// Total number of paired results.
    pub total_results: usize,
    /// Number of results where production passed.
    pub production_passed: usize,
    /// Number of results where shadow passed.
    pub shadow_passed: usize,
    /// Production gate pass rate in [0.0, 1.0].
    pub production_pass_rate: f64,
    /// Shadow gate pass rate in [0.0, 1.0].
    pub shadow_pass_rate: f64,
    /// Average production cost in USD.
    pub avg_production_cost_usd: f64,
    /// Average shadow cost in USD.
    pub avg_shadow_cost_usd: f64,
    /// Average production duration in milliseconds.
    pub avg_production_duration_ms: f64,
    /// Average shadow duration in milliseconds.
    pub avg_shadow_duration_ms: f64,
}

// ---------------------------------------------------------------------------
// ShadowRunner
// ---------------------------------------------------------------------------

/// Tracks shadow-testing results and surfaces superiority alerts.
///
/// # Construction
///
/// ```no_run
/// use roko_learn::shadow::{ShadowConfig, ShadowRunner};
///
/// let config = ShadowConfig {
///     model_slug: "claude-opus-4-6".to_string(),
///     prompt_variant: None,
///     label: "opus-4-6-shadow".to_string(),
/// };
/// let runner = ShadowRunner::new(
///     config,
///     "/workspace/.roko/learn/shadow-results.jsonl",
/// );
/// ```
pub struct ShadowRunner {
    /// Alternative model/prompt to shadow with.
    pub config: ShadowConfig,
    /// Path to the append-only JSONL result log.
    pub results_path: PathBuf,
    /// Invoke a shadow run every `every_n` tasks.
    pub every_n: u32,
    /// Monotonically-increasing task counter; incremented on each
    /// [`should_shadow`][Self::should_shadow] call.
    counter: AtomicU32,
}

impl std::fmt::Debug for ShadowRunner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShadowRunner")
            .field("config", &self.config)
            .field("results_path", &self.results_path)
            .field("every_n", &self.every_n)
            .field("counter", &self.counter.load(Ordering::Relaxed))
            .finish()
    }
}

impl ShadowRunner {
    /// Create a new runner that writes results to `results_path` and shadows
    /// every [`DEFAULT_EVERY_N`] tasks.
    #[must_use]
    pub fn new(config: ShadowConfig, results_path: impl Into<PathBuf>) -> Self {
        Self {
            config,
            results_path: results_path.into(),
            every_n: DEFAULT_EVERY_N,
            counter: AtomicU32::new(0),
        }
    }

    /// Override the sampling rate.
    ///
    /// The counter increments on every call to [`should_shadow`][Self::should_shadow]
    /// and a shadow is triggered when `counter % every_n == 0`.
    ///
    /// # Panics
    ///
    /// Panics if `every_n` is zero.
    #[must_use]
    pub fn with_every_n(mut self, every_n: u32) -> Self {
        assert!(every_n > 0, "every_n must be at least 1");
        self.every_n = every_n;
        self
    }

    /// Return the path where results are persisted.
    #[must_use]
    pub fn results_path(&self) -> &Path {
        &self.results_path
    }

    // -----------------------------------------------------------------------
    // Sampling decision
    // -----------------------------------------------------------------------

    /// Increment the task counter and return `true` every `every_n`-th call.
    ///
    /// The first call (counter = 1) returns `true` when `every_n == 1`.
    /// When `every_n == 5` the method returns `true` on calls 5, 10, 15, …
    ///
    /// Safe to call from concurrent contexts; the counter is atomic.
    #[must_use]
    pub fn should_shadow(&self) -> bool {
        // Increment *before* checking so the first shadow fires on call N
        // rather than call 0.
        let n = self.counter.fetch_add(1, Ordering::Relaxed) + 1;
        n.is_multiple_of(self.every_n)
    }

    // -----------------------------------------------------------------------
    // Result persistence
    // -----------------------------------------------------------------------

    /// Append one [`ShadowResult`] as a JSON line to [`results_path`][Self::results_path].
    ///
    /// The parent directory is created if it does not exist.
    ///
    /// # Errors
    ///
    /// Returns an error for serialization or file I/O failures.
    pub async fn record_result(&self, result: ShadowResult) -> io::Result<()> {
        if let Some(parent) = self.results_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let mut line = serde_json::to_string(&result)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        line.push('\n');

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.results_path)
            .await?;

        file.write_all(line.as_bytes()).await?;
        file.sync_data().await?;

        Ok(())
    }

    /// Read all results from the JSONL file; malformed lines are skipped.
    ///
    /// Returns an empty vector when the file does not exist.
    ///
    /// # Errors
    ///
    /// Returns an error only for file open/read failures.
    pub async fn read_results(&self) -> io::Result<Vec<ShadowResult>> {
        read_shadow_results(&self.results_path).await
    }

    // -----------------------------------------------------------------------
    // Analysis
    // -----------------------------------------------------------------------

    /// Check whether the shadow configuration has consistently outperformed
    /// production.
    ///
    /// Returns `Some(alert)` when all of the following hold:
    /// - At least `min_results` paired results have been recorded.
    /// - <code>shadow_pass_rate − production_pass_rate > [`SUPERIORITY_THRESHOLD_PP`]</code>.
    ///
    /// Returns `None` otherwise. The check reads results from disk each time;
    /// callers should not call this on every task.
    ///
    /// # Errors
    ///
    /// Returns an error for file I/O failures.
    pub async fn check_shadow_superiority(
        &self,
        min_results: usize,
    ) -> io::Result<Option<ShadowAlert>> {
        let results = self.read_results().await?;

        if results.len() < min_results {
            return Ok(None);
        }

        let total = results.len() as f64;
        let production_passed = results.iter().filter(|r| r.production_passed).count();
        let shadow_passed = results.iter().filter(|r| r.shadow_passed).count();

        let production_pass_rate = production_passed as f64 / total;
        let shadow_pass_rate = shadow_passed as f64 / total;
        let delta_pp = shadow_pass_rate - production_pass_rate;

        if delta_pp > SUPERIORITY_THRESHOLD_PP {
            let recommendation = format!(
                "consider promoting '{}' (model: {}) to production — \
                 shadow has outperformed production by {:.1}pp over {} tasks",
                self.config.label,
                self.config.model_slug,
                delta_pp * 100.0,
                results.len(),
            );

            let alert = ShadowAlert {
                shadow_pass_rate,
                production_pass_rate,
                delta_pp,
                result_count: results.len(),
                recommendation,
            };

            warn!(
                shadow_pass_rate,
                production_pass_rate,
                delta_pp,
                result_count = results.len(),
                label = %self.config.label,
                model_slug = %self.config.model_slug,
                "{}",
                alert.summary(),
            );

            Ok(Some(alert))
        } else {
            Ok(None)
        }
    }

    /// Compute aggregate statistics over all recorded results.
    ///
    /// Returns a zeroed [`ShadowSummary`] when the JSONL file does not exist
    /// or is empty.
    ///
    /// # Errors
    ///
    /// Returns an error for file I/O failures.
    pub async fn summary(&self) -> io::Result<ShadowSummary> {
        let results = self.read_results().await?;
        Ok(compute_summary(&results))
    }
}

// ---------------------------------------------------------------------------
// Free-standing helpers
// ---------------------------------------------------------------------------

/// Read shadow results from `path`; malformed lines are skipped.
///
/// # Errors
///
/// Returns an error only for file open/read failures.
pub async fn read_shadow_results(path: &Path) -> io::Result<Vec<ShadowResult>> {
    let file = match tokio::fs::File::open(path).await {
        Ok(f) => f,
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
        if let Ok(result) = serde_json::from_str::<ShadowResult>(trimmed) {
            out.push(result);
        }
    }

    Ok(out)
}

/// Compute aggregate stats over a slice of results.
fn compute_summary(results: &[ShadowResult]) -> ShadowSummary {
    let total = results.len();

    if total == 0 {
        return ShadowSummary {
            total_results: 0,
            production_passed: 0,
            shadow_passed: 0,
            production_pass_rate: 0.0,
            shadow_pass_rate: 0.0,
            avg_production_cost_usd: 0.0,
            avg_shadow_cost_usd: 0.0,
            avg_production_duration_ms: 0.0,
            avg_shadow_duration_ms: 0.0,
        };
    }

    let production_passed = results.iter().filter(|r| r.production_passed).count();
    let shadow_passed = results.iter().filter(|r| r.shadow_passed).count();

    let total_f = total as f64;
    let avg_production_cost_usd =
        results.iter().map(|r| r.production_cost_usd).sum::<f64>() / total_f;
    let avg_shadow_cost_usd =
        results.iter().map(|r| r.shadow_cost_usd).sum::<f64>() / total_f;
    let avg_production_duration_ms =
        results.iter().map(|r| r.production_duration_ms as f64).sum::<f64>() / total_f;
    let avg_shadow_duration_ms =
        results.iter().map(|r| r.shadow_duration_ms as f64).sum::<f64>() / total_f;

    ShadowSummary {
        total_results: total,
        production_passed,
        shadow_passed,
        production_pass_rate: production_passed as f64 / total_f,
        shadow_pass_rate: shadow_passed as f64 / total_f,
        avg_production_cost_usd,
        avg_shadow_cost_usd,
        avg_production_duration_ms,
        avg_shadow_duration_ms,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn config() -> ShadowConfig {
        ShadowConfig {
            model_slug: "claude-opus-4-6".to_string(),
            prompt_variant: None,
            label: "opus-shadow".to_string(),
        }
    }

    fn make_runner(dir: &TempDir) -> ShadowRunner {
        let path = dir.path().join("shadow-results.jsonl");
        ShadowRunner::new(config(), path)
    }

    fn result(task_id: &str, production_passed: bool, shadow_passed: bool) -> ShadowResult {
        ShadowResult {
            task_id: task_id.to_string(),
            plan_id: "plan-001".to_string(),
            timestamp: Utc::now(),
            production_model: "claude-sonnet-4-6".to_string(),
            shadow_model: "claude-opus-4-6".to_string(),
            production_passed,
            shadow_passed,
            production_cost_usd: 0.05,
            shadow_cost_usd: 0.08,
            production_duration_ms: 3_000,
            shadow_duration_ms: 5_000,
            label: "opus-shadow".to_string(),
        }
    }

    // -----------------------------------------------------------------------
    // should_shadow — fires every Nth call
    // -----------------------------------------------------------------------

    #[test]
    fn should_shadow_fires_on_every_nth_call_default_five() {
        let tmp = TempDir::new().expect("tempdir");
        let runner = make_runner(&tmp);

        // First 4 calls should not shadow.
        for i in 1..=4 {
            assert!(
                !runner.should_shadow(),
                "call {i}: expected false before 5th"
            );
        }
        // 5th call should shadow.
        assert!(runner.should_shadow(), "call 5: expected true");

        // Next 4 calls should not shadow.
        for i in 6..=9 {
            assert!(!runner.should_shadow(), "call {i}: expected false");
        }
        // 10th call should shadow.
        assert!(runner.should_shadow(), "call 10: expected true");
    }

    #[test]
    fn should_shadow_fires_every_call_when_every_n_is_one() {
        let tmp = TempDir::new().expect("tempdir");
        let runner = make_runner(&tmp).with_every_n(1);

        for i in 1..=10 {
            assert!(runner.should_shadow(), "call {i}: expected true with every_n=1");
        }
    }

    #[test]
    fn should_shadow_fires_on_correct_multiple_with_every_n_three() {
        let tmp = TempDir::new().expect("tempdir");
        let runner = make_runner(&tmp).with_every_n(3);

        // Calls: 1→false, 2→false, 3→true, 4→false, 5→false, 6→true.
        let expected = [false, false, true, false, false, true, false, false, true];
        for (i, &exp) in expected.iter().enumerate() {
            assert_eq!(
                runner.should_shadow(),
                exp,
                "call {}: expected {exp}",
                i + 1
            );
        }
    }

    // -----------------------------------------------------------------------
    // JSONL write / read round-trip
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn record_result_and_read_results_round_trip() {
        let tmp = TempDir::new().expect("tempdir");
        let runner = make_runner(&tmp);

        let r1 = result("task-001", true, true);
        let r2 = result("task-002", true, false);

        runner.record_result(r1.clone()).await.expect("record r1");
        runner.record_result(r2.clone()).await.expect("record r2");

        let all = runner.read_results().await.expect("read");
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].task_id, r1.task_id);
        assert_eq!(all[1].task_id, r2.task_id);
        assert_eq!(all[0].production_passed, r1.production_passed);
        assert_eq!(all[1].shadow_passed, r2.shadow_passed);
    }

    #[tokio::test]
    async fn read_results_returns_empty_for_missing_file() {
        let tmp = TempDir::new().expect("tempdir");
        let runner = make_runner(&tmp);
        let results = runner.read_results().await.expect("should not error");
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn read_results_skips_malformed_lines() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("shadow-results.jsonl");

        let r = result("task-abc", true, true);
        let good = serde_json::to_string(&r).expect("serialize");
        let content = format!("{good}\n{{bad json\n{good}\n");
        tokio::fs::write(&path, content).await.expect("write");

        let runner = ShadowRunner::new(config(), &path);
        let results = runner.read_results().await.expect("read");
        assert_eq!(results.len(), 2, "two good lines expected");
    }

    #[tokio::test]
    async fn record_result_creates_parent_directories() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("nested").join("dirs").join("shadow-results.jsonl");
        let runner = ShadowRunner::new(config(), &path);

        runner.record_result(result("task-nested", true, true)).await.expect("record");
        assert!(path.exists(), "JSONL file should be created");
    }

    // -----------------------------------------------------------------------
    // No alert when both perform similarly
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn no_alert_when_shadow_and_production_perform_similarly() {
        let tmp = TempDir::new().expect("tempdir");
        let runner = make_runner(&tmp);

        // 20 results: production passes 80%, shadow passes 80%.
        for i in 0..20 {
            let r = result(&format!("task-{i:03}"), i < 16, i < 16);
            runner.record_result(r).await.expect("record");
        }

        let alert = runner
            .check_shadow_superiority(DEFAULT_MIN_RESULTS)
            .await
            .expect("io");
        assert!(alert.is_none(), "equal pass rates should not trigger alert");
    }

    #[tokio::test]
    async fn no_alert_when_shadow_is_only_slightly_better() {
        let tmp = TempDir::new().expect("tempdir");
        let runner = make_runner(&tmp);

        // production: 80% pass, shadow: 88% pass — delta = 8pp < 10pp threshold.
        for i in 0..25 {
            let prod = i < 20; // 80%
            let shad = i < 22; // 88%
            runner.record_result(result(&format!("task-{i:03}"), prod, shad)).await.expect("record");
        }

        let alert = runner
            .check_shadow_superiority(DEFAULT_MIN_RESULTS)
            .await
            .expect("io");
        assert!(alert.is_none(), "8pp delta is below 10pp threshold");
    }

    #[tokio::test]
    async fn no_alert_when_fewer_than_min_results() {
        let tmp = TempDir::new().expect("tempdir");
        let runner = make_runner(&tmp);

        // Only 10 results — well below default min of 20.
        for i in 0..10 {
            // Shadow passes everything, production passes nothing — extreme gap.
            runner.record_result(result(&format!("task-{i:03}"), false, true)).await.expect("record");
        }

        let alert = runner
            .check_shadow_superiority(DEFAULT_MIN_RESULTS)
            .await
            .expect("io");
        assert!(
            alert.is_none(),
            "insufficient results should not fire alert"
        );
    }

    // -----------------------------------------------------------------------
    // Alert fires when shadow consistently outperforms
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn alert_fires_when_shadow_consistently_outperforms_by_more_than_10pp() {
        let tmp = TempDir::new().expect("tempdir");
        let runner = make_runner(&tmp);

        // production: 60% pass, shadow: 90% pass — delta = 30pp > 10pp threshold.
        for i in 0..20 {
            let prod = i < 12; // 60%
            let shad = i < 18; // 90%
            runner.record_result(result(&format!("task-{i:03}"), prod, shad)).await.expect("record");
        }

        let alert = runner
            .check_shadow_superiority(DEFAULT_MIN_RESULTS)
            .await
            .expect("io")
            .expect("expected superiority alert");

        assert!(
            alert.delta_pp > SUPERIORITY_THRESHOLD_PP,
            "delta_pp {} should exceed threshold {}",
            alert.delta_pp,
            SUPERIORITY_THRESHOLD_PP
        );
        assert_eq!(alert.result_count, 20);
        assert!(!alert.recommendation.is_empty());
    }

    #[tokio::test]
    async fn alert_delta_pp_is_accurate() {
        let tmp = TempDir::new().expect("tempdir");
        let runner = make_runner(&tmp);

        // 20 results: production passes 10 (50%), shadow passes 16 (80%) → delta 30pp.
        for i in 0..20 {
            runner
                .record_result(result(&format!("task-{i:03}"), i < 10, i < 16))
                .await
                .expect("record");
        }

        let alert = runner
            .check_shadow_superiority(DEFAULT_MIN_RESULTS)
            .await
            .expect("io")
            .expect("alert expected");

        assert!((alert.production_pass_rate - 0.5).abs() < 1e-9);
        assert!((alert.shadow_pass_rate - 0.8).abs() < 1e-9);
        assert!((alert.delta_pp - 0.3).abs() < 1e-9);
    }

    // -----------------------------------------------------------------------
    // Summary stats computation
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn summary_returns_zero_for_empty_results() {
        let tmp = TempDir::new().expect("tempdir");
        let runner = make_runner(&tmp);

        let s = runner.summary().await.expect("io");
        assert_eq!(s.total_results, 0);
        assert_eq!(s.production_pass_rate, 0.0);
        assert_eq!(s.shadow_pass_rate, 0.0);
        assert_eq!(s.avg_production_cost_usd, 0.0);
        assert_eq!(s.avg_shadow_cost_usd, 0.0);
    }

    #[tokio::test]
    async fn summary_computes_correct_aggregate_stats() {
        let tmp = TempDir::new().expect("tempdir");
        let runner = make_runner(&tmp);

        // 4 results: 3 prod pass, 2 shadow pass.
        let data = [
            ("t1", true, true, 0.10, 0.20, 1_000u64, 2_000u64),
            ("t2", true, false, 0.20, 0.10, 3_000u64, 1_000u64),
            ("t3", true, true, 0.30, 0.40, 2_000u64, 3_000u64),
            ("t4", false, false, 0.05, 0.05, 500u64, 500u64),
        ];

        for (id, pp, sp, pc, sc, pd, sd) in data {
            runner
                .record_result(ShadowResult {
                    task_id: id.to_string(),
                    plan_id: "plan-x".to_string(),
                    timestamp: Utc::now(),
                    production_model: "prod-model".to_string(),
                    shadow_model: "shadow-model".to_string(),
                    production_passed: pp,
                    shadow_passed: sp,
                    production_cost_usd: pc,
                    shadow_cost_usd: sc,
                    production_duration_ms: pd,
                    shadow_duration_ms: sd,
                    label: "test".to_string(),
                })
                .await
                .expect("record");
        }

        let s = runner.summary().await.expect("io");
        assert_eq!(s.total_results, 4);
        assert_eq!(s.production_passed, 3);
        assert_eq!(s.shadow_passed, 2);
        assert!((s.production_pass_rate - 0.75).abs() < 1e-9);
        assert!((s.shadow_pass_rate - 0.50).abs() < 1e-9);

        // avg_production_cost = (0.10 + 0.20 + 0.30 + 0.05) / 4 = 0.1625
        assert!((s.avg_production_cost_usd - 0.1625).abs() < 1e-9);
        // avg_shadow_cost = (0.20 + 0.10 + 0.40 + 0.05) / 4 = 0.1875
        assert!((s.avg_shadow_cost_usd - 0.1875).abs() < 1e-9);
        // avg_production_duration = (1000 + 3000 + 2000 + 500) / 4 = 1625 ms
        assert!((s.avg_production_duration_ms - 1625.0).abs() < 1e-9);
        // avg_shadow_duration = (2000 + 1000 + 3000 + 500) / 4 = 1625 ms
        assert!((s.avg_shadow_duration_ms - 1625.0).abs() < 1e-9);
    }

    // -----------------------------------------------------------------------
    // ShadowAlert summary helper
    // -----------------------------------------------------------------------

    #[test]
    fn shadow_alert_summary_contains_key_fields() {
        let alert = ShadowAlert {
            shadow_pass_rate: 0.90,
            production_pass_rate: 0.70,
            delta_pp: 0.20,
            result_count: 25,
            recommendation: "promote the shadow config".to_string(),
        };
        let s = alert.summary();
        assert!(s.contains("90.0"), "summary: {s}");
        assert!(s.contains("70.0"), "summary: {s}");
        assert!(s.contains("20.0"), "summary: {s}");
        assert!(s.contains("25"), "summary: {s}");
        assert!(s.contains("promote"), "summary: {s}");
    }
}
