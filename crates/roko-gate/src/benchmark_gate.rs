//! `BenchmarkRegressionGate` — detects benchmark regressions.
//!
//! Compares current Criterion benchmark results against a stored baseline
//! to detect performance regressions.  The gate runs `cargo bench` with
//! `--message-format json` and parses the `benchmark-complete` JSON lines
//! that Criterion emits.  Baselines are stored under
//! `.roko/bench/baselines/<name>.json` (benchmark name → nanosecond mean).
//!
//! # Baseline lifecycle
//!
//! - **No baseline file**: run benchmarks, write results as the new baseline,
//!   return `Verdict::pass` with a note that the baseline was just established.
//! - **Baseline exists**: compare each benchmark's mean against the stored
//!   value.  Fail if any benchmark regresses beyond `threshold_pct`.
//!
//! # Criterion JSON format
//!
//! `cargo bench -- --message-format json` (or `cargo criterion --message-format
//! json`) emits one JSON object per line.  The lines we care about have
//! `"reason": "benchmark-complete"` and include a `"typical"` statistics block:
//!
//! ```json
//! {
//!   "reason": "benchmark-complete",
//!   "id": "engram_build",
//!   "typical": { "estimate": 123.45, "unit": "ns", ... },
//!   ...
//! }
//! ```
//!
//! All non-matching lines (compiler messages, `group-complete`, etc.) are
//! silently ignored.

use async_trait::async_trait;
use roko_core::{Context, Signal, Verify, Verdict};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::process::Command;
use tokio::time::timeout;

/// Threshold for regression detection (percentage slowdown allowed).
const DEFAULT_REGRESSION_THRESHOLD_PCT: f64 = 10.0;

/// Default timeout for running cargo bench (10 minutes).
const DEFAULT_BENCH_TIMEOUT_MS: u64 = 600_000;

// ─── Criterion JSON types ────────────────────────────────────────────────────

/// One `benchmark-complete` line emitted by Criterion's `--message-format json`.
///
/// Only the fields we actually need are deserialized; the rest are ignored via
/// `deny_unknown_fields = false` (the serde default).
#[derive(Debug, Deserialize)]
struct CriterionMessage {
    reason: String,
    /// The benchmark function name (e.g. "engram_build").
    id: Option<String>,
    /// The "typical" (representative) estimate for this benchmark run.
    typical: Option<CriterionEstimate>,
}

/// A single statistics estimate block inside a Criterion JSON message.
#[derive(Debug, Deserialize)]
struct CriterionEstimate {
    /// Point estimate in `unit` units.
    estimate: f64,
    /// Unit string: "ns" for nanoseconds, "us" for microseconds, etc.
    unit: Option<String>,
}

// ─── Public types ────────────────────────────────────────────────────────────

/// A single benchmark comparison result.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BenchmarkComparison {
    /// Benchmark name.
    pub name: String,
    /// Baseline time in nanoseconds.
    pub baseline_ns: f64,
    /// Current time in nanoseconds.
    pub current_ns: f64,
    /// Percentage change (positive = slower / regression).
    pub change_pct: f64,
}

/// A gate that detects benchmark regressions.
pub struct BenchmarkRegressionGate {
    name: String,
    /// Maximum allowed slowdown percentage before failing.
    threshold_pct: f64,
    /// Optional explicit cargo bench args (e.g. `["--bench", "hdc_bench"]`).
    bench_args: Vec<String>,
    /// Override for the baseline directory.  Defaults to
    /// `.roko/bench/baselines/` relative to the working directory.
    baseline_dir: Option<PathBuf>,
    /// Timeout for the `cargo bench` invocation.
    timeout_ms: u64,
}

impl BenchmarkRegressionGate {
    /// Create a benchmark regression gate with the default threshold.
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: "benchmark_regression".to_string(),
            threshold_pct: DEFAULT_REGRESSION_THRESHOLD_PCT,
            bench_args: Vec::new(),
            baseline_dir: None,
            timeout_ms: DEFAULT_BENCH_TIMEOUT_MS,
        }
    }

    /// Override the regression threshold percentage.
    #[must_use]
    pub fn with_threshold_pct(mut self, pct: f64) -> Self {
        self.threshold_pct = pct;
        self
    }

    /// Pass extra arguments to `cargo bench` (e.g. `--bench engram_bench`).
    #[must_use]
    pub fn with_bench_args(mut self, args: Vec<String>) -> Self {
        self.bench_args = args;
        self
    }

    /// Override the directory where baseline JSON files are stored.
    #[must_use]
    pub fn with_baseline_dir(mut self, dir: PathBuf) -> Self {
        self.baseline_dir = Some(dir);
        self
    }

    /// Override the timeout in milliseconds (default: 10 minutes).
    #[must_use]
    pub fn with_timeout_ms(mut self, ms: u64) -> Self {
        self.timeout_ms = ms;
        self
    }

    /// Resolve the baseline file path for a given gate name.
    fn baseline_path(&self, working_dir: &Path) -> PathBuf {
        let dir = self
            .baseline_dir
            .clone()
            .unwrap_or_else(|| working_dir.join(".roko/bench/baselines"));
        // Sanitize the gate name to make it safe as a filename.
        let safe_name = self.name.replace(['/', '\\', ':', ' '], "_");
        dir.join(format!("{safe_name}.json"))
    }
}

impl Default for BenchmarkRegressionGate {
    fn default() -> Self {
        Self::new()
    }
}

impl roko_core::Cell for BenchmarkRegressionGate {
    fn cell_id(&self) -> &str { "benchmark-gate" }
    fn cell_name(&self) -> &str { "BenchmarkRegressionGate" }
    fn protocols(&self) -> Vec<roko_core::ProtocolId> { vec![roko_core::ProtocolId::Verify] }
}

#[async_trait]
impl Verify for BenchmarkRegressionGate {
    async fn verify(&self, _signal: &Signal, _ctx: &Context) -> Verdict {
        let started = Instant::now();
        let elapsed_ms = || {
            #[allow(clippy::cast_possible_truncation)]
            let ms = started.elapsed().as_millis() as u64;
            ms
        };

        let working_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let baseline_path = self.baseline_path(&working_dir);

        // Run `cargo bench -- --message-format json`
        let bench_output = run_cargo_bench(&self.bench_args, &working_dir, self.timeout_ms).await;
        let bench_output = match bench_output {
            Ok(out) => out,
            Err(e) => {
                return Verdict::fail(
                    &self.name,
                    format!("cargo bench failed: {e}"),
                )
                .with_duration(elapsed_ms());
            }
        };

        // Parse Criterion JSON output → benchmark name → nanosecond mean.
        let current = parse_criterion_json(&bench_output);
        if current.is_empty() {
            return Verdict::skip(
                &self.name,
                format!(
                    "no benchmark-complete messages found in cargo bench output \
                     (threshold={:.1}%); output may be empty or non-JSON",
                    self.threshold_pct
                ),
            )
            .with_duration(elapsed_ms());
        }

        // Read or establish baseline.
        match read_baseline(&baseline_path) {
            None => {
                // No baseline yet — write the current results as the first baseline.
                if let Err(e) = write_baseline(&baseline_path, &current) {
                    tracing::warn!(
                        gate = %self.name,
                        path = %baseline_path.display(),
                        error = %e,
                        "failed to write benchmark baseline"
                    );
                }
                let count = current.len();
                return Verdict::pass(&self.name)
                    .with_detail(format!(
                        "baseline established with {count} benchmark(s); \
                         future runs will compare against this baseline (threshold={:.1}%)",
                        self.threshold_pct
                    ))
                    .with_duration(elapsed_ms());
            }
            Some(baseline) => {
                // Compare current results against baseline.
                let comparisons = compare_results(&baseline, &current, self.threshold_pct);
                let regressions: Vec<&BenchmarkComparison> =
                    comparisons.iter().filter(|c| c.change_pct > self.threshold_pct).collect();

                if regressions.is_empty() {
                    let detail = format_comparison_summary(&comparisons, self.threshold_pct);
                    Verdict::pass(&self.name)
                        .with_detail(detail)
                        .with_duration(elapsed_ms())
                } else {
                    let reason = format_regression_reason(&regressions, self.threshold_pct);
                    let detail = format_comparison_summary(&comparisons, self.threshold_pct);
                    Verdict::fail(&self.name, reason)
                        .with_detail(detail)
                        .with_duration(elapsed_ms())
                }
            }
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

// ─── Core logic (all pure / synchronous — easy to unit-test) ─────────────────

/// Convert a Criterion estimate unit string + value to nanoseconds.
///
/// Criterion reports times in the most human-readable unit; we normalise to ns
/// so comparisons stay consistent regardless of benchmark speed.
fn to_nanoseconds(estimate: f64, unit: &str) -> f64 {
    match unit {
        "ns" => estimate,
        "us" | "μs" => estimate * 1_000.0,
        "ms" => estimate * 1_000_000.0,
        "s" => estimate * 1_000_000_000.0,
        // Unknown unit: treat as nanoseconds to avoid silent data loss.
        _ => {
            tracing::warn!(unit = %unit, "unknown criterion time unit; treating as ns");
            estimate
        }
    }
}

/// Parse Criterion `--message-format json` stdout into `{name → ns_mean}`.
///
/// Each line is an independent JSON object.  Lines that are not valid JSON,
/// or whose `"reason"` is not `"benchmark-complete"`, are silently skipped.
/// This matches how Criterion interleaves compiler messages with benchmark
/// results.
pub fn parse_criterion_json(output: &str) -> HashMap<String, f64> {
    let mut results = HashMap::new();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let msg: CriterionMessage = match serde_json::from_str(line) {
            Ok(m) => m,
            Err(_) => continue, // compiler diagnostic, blank line, etc.
        };

        if msg.reason != "benchmark-complete" {
            continue;
        }

        let Some(id) = msg.id else { continue };
        let Some(typical) = msg.typical else { continue };

        let unit = typical.unit.as_deref().unwrap_or("ns");
        let ns = to_nanoseconds(typical.estimate, unit);
        results.insert(id, ns);
    }

    results
}

/// Load a baseline file from `path`.  Returns `None` if the file does not
/// exist or cannot be parsed (treated as "no baseline").
pub fn read_baseline(path: &Path) -> Option<HashMap<String, f64>> {
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

/// Persist `data` (benchmark name → ns mean) to `path`, creating parent
/// directories as needed.
pub fn write_baseline<S: std::hash::BuildHasher>(
    path: &Path,
    data: &HashMap<String, f64, S>,
) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    std::fs::write(path, json)
}

/// Compare current results against baseline and return per-benchmark comparisons.
///
/// Benchmarks present in `current` but absent from `baseline` are included
/// with `change_pct = 0.0` (new benchmarks are not regressions).  Benchmarks
/// in `baseline` but absent from `current` are omitted (they may have been
/// renamed or removed).
pub fn compare_results<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
    baseline: &HashMap<String, f64, S1>,
    current: &HashMap<String, f64, S2>,
    _threshold_pct: f64,
) -> Vec<BenchmarkComparison> {
    let mut comparisons = Vec::new();

    for (name, &current_ns) in current {
        let (baseline_ns, change_pct) = match baseline.get(name) {
            Some(&b) if b > 0.0 => {
                let pct = ((current_ns - b) / b) * 100.0;
                (b, pct)
            }
            // Benchmark missing from baseline — treat as no change.
            _ => (current_ns, 0.0),
        };

        comparisons.push(BenchmarkComparison {
            name: name.clone(),
            baseline_ns,
            current_ns,
            change_pct,
        });
    }

    // Stable sort by name for deterministic output.
    comparisons.sort_by(|a, b| a.name.cmp(&b.name));
    comparisons
}

/// Format a one-line failure reason listing regressed benchmarks.
fn format_regression_reason(
    regressions: &[&BenchmarkComparison],
    threshold_pct: f64,
) -> String {
    let names: Vec<String> = regressions
        .iter()
        .map(|c| format!("{} (+{:.1}%)", c.name, c.change_pct))
        .collect();
    format!(
        "{} benchmark(s) regressed beyond {:.1}% threshold: {}",
        regressions.len(),
        threshold_pct,
        names.join(", ")
    )
}

/// Format a multi-line comparison table for the verdict detail.
fn format_comparison_summary(comparisons: &[BenchmarkComparison], threshold_pct: f64) -> String {
    let mut lines = vec![format!(
        "Benchmark regression check (threshold: {threshold_pct:.1}%):"
    )];
    for c in comparisons {
        let symbol = if c.change_pct > threshold_pct {
            "FAIL"
        } else if c.change_pct > 0.0 {
            "warn"
        } else {
            "pass"
        };
        lines.push(format!(
            "  [{symbol}] {}: baseline={:.1}ns current={:.1}ns change={:+.1}%",
            c.name, c.baseline_ns, c.current_ns, c.change_pct
        ));
    }
    lines.join("\n")
}

// ─── Subprocess runner ────────────────────────────────────────────────────────

/// Run `cargo bench -- --message-format json` and return combined stdout+stderr.
///
/// Criterion writes `benchmark-complete` lines to **stdout**; compiler messages
/// go to stderr.  We capture both and concatenate them because some Criterion
/// versions mix output streams.
async fn run_cargo_bench(
    extra_args: &[String],
    working_dir: &Path,
    timeout_ms: u64,
) -> Result<String, String> {
    let mut cmd = Command::new("cargo");
    cmd.arg("bench");
    for arg in extra_args {
        cmd.arg(arg);
    }
    // Pass Criterion-specific flags after the `--` separator.
    cmd.arg("--");
    cmd.arg("--message-format");
    cmd.arg("json");

    cmd.current_dir(working_dir);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    cmd.kill_on_drop(true);

    let fut = async {
        let output = cmd
            .output()
            .await
            .map_err(|e| format!("spawn failed: {e}"))?;

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

        if !output.status.success() {
            let code = output
                .status
                .code()
                .map_or_else(|| "signal".to_string(), |c| c.to_string());
            return Err(format!(
                "exit code {code}\n--- stderr ---\n{stderr}"
            ));
        }

        // Concatenate both streams — Criterion JSON appears on stdout but
        // some build output may contain JSON-like lines on stderr.
        let combined = if stderr.is_empty() {
            stdout
        } else {
            format!("{stdout}\n{stderr}")
        };
        Ok(combined)
    };

    timeout(Duration::from_millis(timeout_ms), fut)
        .await
        .map_err(|_| format!("cargo bench timed out after {timeout_ms} ms"))?
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // ── parse_criterion_json ─────────────────────────────────────────────────

    #[test]
    fn parse_benchmark_complete_ns() {
        let output = r#"{"reason":"benchmark-complete","id":"engram_build","report_directory":"/tmp/report","iteration_count":[100],"measured_values":[123.0],"unit":"ns","typical":{"estimate":123.45,"lower_bound":120.0,"upper_bound":130.0,"unit":"ns"}}"#;
        let results = parse_criterion_json(output);
        assert_eq!(results.len(), 1);
        let ns = results["engram_build"];
        assert!((ns - 123.45).abs() < 0.01, "expected ~123.45 ns, got {ns}");
    }

    #[test]
    fn parse_benchmark_complete_us_converts_to_ns() {
        let output = r#"{"reason":"benchmark-complete","id":"slow_bench","typical":{"estimate":1.5,"unit":"us"}}"#;
        let results = parse_criterion_json(output);
        let ns = results["slow_bench"];
        assert!((ns - 1500.0).abs() < 0.01, "expected 1500 ns, got {ns}");
    }

    #[test]
    fn parse_benchmark_complete_ms_converts_to_ns() {
        let output = r#"{"reason":"benchmark-complete","id":"very_slow","typical":{"estimate":2.0,"unit":"ms"}}"#;
        let results = parse_criterion_json(output);
        let ns = results["very_slow"];
        assert!((ns - 2_000_000.0).abs() < 1.0, "expected 2000000 ns, got {ns}");
    }

    #[test]
    fn parse_skips_non_benchmark_lines() {
        let output = r#"
{"reason":"compiler-message","message":{"code":null,"level":"warning","message":"unused import","spans":[]}}
{"reason":"group-complete","group_name":"my_group","benchmarks":["bench_a"]}
{"reason":"benchmark-complete","id":"bench_a","typical":{"estimate":42.0,"unit":"ns"}}
not json at all
{"broken": true
"#;
        let results = parse_criterion_json(output);
        assert_eq!(results.len(), 1);
        assert!(results.contains_key("bench_a"));
    }

    #[test]
    fn parse_multiple_benchmarks() {
        let output = [
            r#"{"reason":"benchmark-complete","id":"hdc_bind","typical":{"estimate":50.0,"unit":"ns"}}"#,
            r#"{"reason":"benchmark-complete","id":"hdc_similarity","typical":{"estimate":30.0,"unit":"ns"}}"#,
            r#"{"reason":"benchmark-complete","id":"hdc_from_seed","typical":{"estimate":200.0,"unit":"ns"}}"#,
        ]
        .join("\n");
        let results = parse_criterion_json(&output);
        assert_eq!(results.len(), 3);
        assert!((results["hdc_bind"] - 50.0).abs() < 0.01);
        assert!((results["hdc_similarity"] - 30.0).abs() < 0.01);
        assert!((results["hdc_from_seed"] - 200.0).abs() < 0.01);
    }

    #[test]
    fn parse_empty_output_returns_empty_map() {
        assert!(parse_criterion_json("").is_empty());
        assert!(parse_criterion_json("   \n\n  ").is_empty());
    }

    // ── to_nanoseconds ───────────────────────────────────────────────────────

    #[test]
    fn unit_conversion_ns() {
        assert!((to_nanoseconds(1.0, "ns") - 1.0).abs() < 1e-10);
    }

    #[test]
    fn unit_conversion_us() {
        assert!((to_nanoseconds(1.0, "us") - 1_000.0).abs() < 1e-10);
        assert!((to_nanoseconds(1.0, "μs") - 1_000.0).abs() < 1e-10);
    }

    #[test]
    fn unit_conversion_ms() {
        assert!((to_nanoseconds(1.0, "ms") - 1_000_000.0).abs() < 1e-10);
    }

    #[test]
    fn unit_conversion_s() {
        assert!((to_nanoseconds(1.0, "s") - 1_000_000_000.0).abs() < 1e-10);
    }

    // ── compare_results ──────────────────────────────────────────────────────

    #[test]
    fn no_regression_within_threshold() {
        let baseline: HashMap<String, f64> =
            [("bench_a".to_string(), 100.0), ("bench_b".to_string(), 200.0)]
                .into_iter()
                .collect();
        // 5% slower — below the 10% threshold.
        let current: HashMap<String, f64> =
            [("bench_a".to_string(), 105.0), ("bench_b".to_string(), 202.0)]
                .into_iter()
                .collect();
        let comparisons = compare_results(&baseline, &current, 10.0);
        let regressions: Vec<_> = comparisons
            .iter()
            .filter(|c| c.change_pct > 10.0)
            .collect();
        assert!(regressions.is_empty(), "expected no regressions");
    }

    #[test]
    fn regression_detected_above_threshold() {
        let baseline: HashMap<String, f64> =
            [("heavy_fn".to_string(), 100.0)].into_iter().collect();
        // 25% slower — well above 10% threshold.
        let current: HashMap<String, f64> =
            [("heavy_fn".to_string(), 125.0)].into_iter().collect();
        let comparisons = compare_results(&baseline, &current, 10.0);
        assert_eq!(comparisons.len(), 1);
        assert!((comparisons[0].change_pct - 25.0).abs() < 0.01);
    }

    #[test]
    fn improvement_is_not_a_regression() {
        let baseline: HashMap<String, f64> =
            [("fast_fn".to_string(), 100.0)].into_iter().collect();
        // 20% faster.
        let current: HashMap<String, f64> =
            [("fast_fn".to_string(), 80.0)].into_iter().collect();
        let comparisons = compare_results(&baseline, &current, 10.0);
        assert!(comparisons[0].change_pct < 0.0, "improvement should be negative");
        let regressions: Vec<_> = comparisons
            .iter()
            .filter(|c| c.change_pct > 10.0)
            .collect();
        assert!(regressions.is_empty());
    }

    #[test]
    fn new_benchmark_not_in_baseline_is_not_regression() {
        let baseline: HashMap<String, f64> =
            [("existing".to_string(), 100.0)].into_iter().collect();
        let current: HashMap<String, f64> = [
            ("existing".to_string(), 100.0),
            ("new_bench".to_string(), 500.0),
        ]
        .into_iter()
        .collect();
        let comparisons = compare_results(&baseline, &current, 10.0);
        let new_entry = comparisons.iter().find(|c| c.name == "new_bench").unwrap();
        assert!((new_entry.change_pct).abs() < 0.01, "new bench change_pct should be 0");
    }

    #[test]
    fn comparison_sorted_by_name() {
        let baseline: HashMap<String, f64> = [
            ("z_bench".to_string(), 100.0),
            ("a_bench".to_string(), 200.0),
            ("m_bench".to_string(), 150.0),
        ]
        .into_iter()
        .collect();
        let current = baseline.clone();
        let comparisons = compare_results(&baseline, &current, 10.0);
        let names: Vec<&str> = comparisons.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["a_bench", "m_bench", "z_bench"]);
    }

    // ── baseline I/O ─────────────────────────────────────────────────────────

    #[test]
    fn read_baseline_returns_none_for_missing_file() {
        let path = Path::new("/tmp/__roko_bench_nonexistent_path_xyz/baseline.json");
        assert!(read_baseline(path).is_none());
    }

    #[test]
    fn write_and_read_baseline_roundtrip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("baselines").join("test_gate.json");
        let data: HashMap<String, f64> = [
            ("bench_a".to_string(), 123.4),
            ("bench_b".to_string(), 567.8),
        ]
        .into_iter()
        .collect();

        write_baseline(&path, &data).expect("write baseline");
        let loaded = read_baseline(&path).expect("read baseline");

        assert!((loaded["bench_a"] - 123.4).abs() < 1e-6);
        assert!((loaded["bench_b"] - 567.8).abs() < 1e-6);
    }

    // ── gate metadata ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn benchmark_gate_default() {
        let gate = BenchmarkRegressionGate::default();
        assert_eq!(gate.name(), "benchmark_regression");
    }

    #[test]
    fn custom_threshold_stored() {
        let gate = BenchmarkRegressionGate::new().with_threshold_pct(5.0);
        assert!((gate.threshold_pct - 5.0).abs() < 1e-10);
    }

    // ── format helpers ───────────────────────────────────────────────────────

    #[test]
    fn regression_reason_lists_benchmarks() {
        let regressions = vec![
            BenchmarkComparison {
                name: "slow_fn".to_string(),
                baseline_ns: 100.0,
                current_ns: 120.0,
                change_pct: 20.0,
            },
        ];
        let refs: Vec<&BenchmarkComparison> = regressions.iter().collect();
        let reason = format_regression_reason(&refs, 10.0);
        assert!(reason.contains("1 benchmark(s)"));
        assert!(reason.contains("slow_fn"));
        assert!(reason.contains("20.0%"));
        assert!(reason.contains("10.0%"));
    }

    #[test]
    fn comparison_summary_shows_pass_and_fail() {
        let comparisons = vec![
            BenchmarkComparison {
                name: "ok_bench".to_string(),
                baseline_ns: 100.0,
                current_ns: 100.0,
                change_pct: 0.0,
            },
            BenchmarkComparison {
                name: "bad_bench".to_string(),
                baseline_ns: 100.0,
                current_ns: 125.0,
                change_pct: 25.0,
            },
        ];
        let summary = format_comparison_summary(&comparisons, 10.0);
        assert!(summary.contains("[pass] ok_bench"));
        assert!(summary.contains("[FAIL] bad_bench"));
    }

    // ── end-to-end logic (no real cargo bench invocation) ────────────────────

    /// Simulate what verify() does internally without running cargo bench.
    #[test]
    fn end_to_end_regression_detection() {
        let baseline: HashMap<String, f64> =
            [("my_fn".to_string(), 100.0)].into_iter().collect();

        // 15% regression — above the 10% threshold.
        let current_output = r#"{"reason":"benchmark-complete","id":"my_fn","typical":{"estimate":115.0,"unit":"ns"}}"#;
        let current = parse_criterion_json(current_output);
        let comparisons = compare_results(&baseline, &current, 10.0);
        let regressions: Vec<_> = comparisons
            .iter()
            .filter(|c| c.change_pct > 10.0)
            .collect();

        assert_eq!(regressions.len(), 1);
        assert!((regressions[0].change_pct - 15.0).abs() < 0.1);
    }
}
