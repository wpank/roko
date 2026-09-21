//! Chaos engineering / fault injection framework for evaluation testing.
//!
//! This module is compiled only when the `chaos` feature flag is enabled.
//! Enable it with:
//! ```toml
//! [features]
//! chaos = []
//! ```
//! and activate at runtime by setting `ROKO_CHAOS_MODE=1`.
//!
//! # Design
//!
//! `FaultInjector` injects controlled faults into a temporary working directory
//! and records the injection timestamps. Callers call [`FaultInjector::mark_recovered`]
//! when they observe a passing [`roko_core::Verdict`], which closes the fault record and
//! computes recovery latency. The final [`AntifragilityReport`] compares chaos and baseline
//! pass rates to produce an antifragility index.
//!
//! **Safety**: all file-mutating methods target files supplied by the caller. Tests
//! MUST pass paths under a [`tempfile::TempDir`]; never the real workspace.

use chrono::{DateTime, Utc};
use std::{
    io,
    path::{Path, PathBuf},
};

// ─── Fault taxonomy ──────────────────────────────────────────────────────────

/// The kind of fault to inject.
#[derive(Debug, Clone, PartialEq)]
pub enum FaultType {
    /// Add `#[deprecated(note = "chaos-injection")]` above a `pub fn` or `pub struct`
    /// in the target source file.
    CompileWarningInjection,
    /// Sleep for `ms` milliseconds before the gate executes.
    ///
    /// The delay itself is recorded here; the actual `tokio::time::sleep` executes
    /// at the gate call site using the value stored in `FaultRecord`.
    GateDelay {
        /// Delay in milliseconds.
        ms: u64,
    },
    /// Send SIGKILL to the target agent process.
    ///
    /// Currently a stub — requires OS-level process tracking via
    /// `ProcessSupervisor`. Returns `Err` with `ErrorKind::Unsupported`.
    ProcessKill,
    /// Write 1 KiB of pseudo-random bytes at a random offset in the target file.
    WorktreeCorruption,
    /// Block outbound connections from the agent process.
    ///
    /// Currently a stub — requires OS-level firewall rules. Returns `Err` with
    /// `ErrorKind::Unsupported`.
    NetworkPartition,
}

// ─── Records ─────────────────────────────────────────────────────────────────

/// A single fault injection event with optional recovery timestamp.
#[derive(Debug, Clone)]
pub struct FaultRecord {
    /// Kind of fault that was injected.
    pub fault_type: FaultType,
    /// Wall-clock time at injection.
    pub injected_at: DateTime<Utc>,
    /// Wall-clock time when recovery was confirmed, or `None` if not yet recovered.
    pub recovered_at: Option<DateTime<Utc>>,
    /// Milliseconds from injection to recovery, or `None` if not yet recovered.
    pub recovery_time_ms: Option<u64>,
    /// Human-readable target description (file path, process ID, etc.).
    pub target: String,
}

// ─── Report ──────────────────────────────────────────────────────────────────

/// Summary statistics from a chaos evaluation run.
#[derive(Debug, Clone)]
pub struct AntifragilityReport {
    /// Gate pass rate observed without any faults (0.0–1.0).
    pub baseline_pass_rate: f64,
    /// Gate pass rate observed while faults were active (0.0–1.0).
    pub chaos_pass_rate: f64,
    /// `chaos_pass_rate / baseline_pass_rate`.
    ///
    /// Values > 1.0 indicate antifragility; values < 1.0 indicate fragility.
    /// Set to `f64::INFINITY` when `baseline_pass_rate` is zero.
    pub antifragility_index: f64,
    /// Total number of faults injected during the run.
    pub total_faults: usize,
    /// Mean recovery latency across all recovered faults, or `None` when no
    /// fault has a recovery timestamp.
    pub avg_recovery_ms: Option<u64>,
}

// ─── Injector ────────────────────────────────────────────────────────────────

/// Stateful fault injector tied to a single temporary working directory.
///
/// Construct with [`FaultInjector::new`], inject faults via the `inject_*` methods,
/// and collect results via [`FaultInjector::report`] and [`FaultInjector::records`].
///
/// # Isolation contract
///
/// `workdir` MUST be a path inside a `tempfile::TempDir` during tests. The injector
/// does not validate this; passing a real workspace path is a caller error.
pub struct FaultInjector {
    /// Temporary working directory used to scope file mutations.
    workdir: PathBuf,
    /// Ordered log of every injection event.
    fault_log: Vec<FaultRecord>,
}

impl FaultInjector {
    /// Create a new injector scoped to `workdir`.
    ///
    /// `workdir` should be a `tempfile::TempDir` path during tests. The directory
    /// must already exist; the injector does not create it.
    #[must_use]
    pub fn new(workdir: PathBuf) -> Self {
        Self {
            workdir,
            fault_log: Vec::new(),
        }
    }

    /// Return the working directory this injector is scoped to.
    #[must_use]
    pub fn workdir(&self) -> &Path {
        &self.workdir
    }

    /// Inject a compile warning by prepending `#[deprecated(note = "chaos-injection")]`
    /// above the first `pub fn` or `pub struct` found in `target_file`.
    ///
    /// If no eligible item is found the file is left unchanged and an `io::Error`
    /// with `ErrorKind::InvalidData` is returned.
    ///
    /// Returns the index of the new `FaultRecord` in the internal log, which can
    /// be passed to [`Self::mark_recovered`] once recovery is confirmed.
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` when the file cannot be read or written, or when no
    /// eligible `pub fn` / `pub struct` is found.
    pub fn inject_compile_warning(&mut self, target_file: &Path) -> io::Result<usize> {
        let src = std::fs::read_to_string(target_file)?;

        // Find the byte offset of the first `pub fn` or `pub struct` line.
        let insertion_line = src.lines().enumerate().find_map(|(idx, line)| {
            let trimmed = line.trim_start();
            if trimmed.starts_with("pub fn ")
                || trimmed.starts_with("pub struct ")
                || trimmed.starts_with("pub async fn ")
                || trimmed.starts_with("pub(crate) fn ")
                || trimmed.starts_with("pub(super) fn ")
            {
                Some(idx)
            } else {
                None
            }
        });

        let Some(line_index) = insertion_line else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "chaos: no eligible `pub fn` or `pub struct` found in {}",
                    target_file.display()
                ),
            ));
        };

        // Rebuild the file with the deprecated attribute prepended.
        let mut lines: Vec<&str> = src.lines().collect();
        let deprecated_attr = "#[deprecated(note = \"chaos-injection\")]";
        lines.insert(line_index, deprecated_attr);
        let patched = lines.join("\n");
        // Preserve a trailing newline if the original had one.
        let patched = if src.ends_with('\n') {
            format!("{patched}\n")
        } else {
            patched
        };

        std::fs::write(target_file, &patched)?;

        let idx = self.push_record(FaultRecord {
            fault_type: FaultType::CompileWarningInjection,
            injected_at: Utc::now(),
            recovered_at: None,
            recovery_time_ms: None,
            target: target_file.display().to_string(),
        });
        Ok(idx)
    }

    /// Record a gate delay fault.
    ///
    /// The actual `tokio::time::sleep(Duration::from_millis(delay_ms))` MUST be
    /// executed at the gate call site. This method only records the intent and
    /// returns the fault index.
    pub fn inject_gate_delay(&mut self, delay_ms: u64) -> usize {
        self.push_record(FaultRecord {
            fault_type: FaultType::GateDelay { ms: delay_ms },
            injected_at: Utc::now(),
            recovered_at: None,
            recovery_time_ms: None,
            target: format!("gate-delay-{delay_ms}ms"),
        })
    }

    /// Inject worktree corruption by writing 1 KiB of deterministic pseudo-random
    /// bytes at a random byte offset in `target_file`.
    ///
    /// Uses a simple LCG seeded from the current timestamp so the output is
    /// reproducible when the same seed is used — but varies across calls.
    ///
    /// Returns the fault record index.
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` when the file cannot be read or written, or when the
    /// file is empty.
    pub fn inject_worktree_corruption(&mut self, target_file: &Path) -> io::Result<usize> {
        let mut contents = std::fs::read(target_file)?;

        if contents.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("chaos: cannot corrupt empty file {}", target_file.display()),
            ));
        }

        // Deterministic pseudo-random via a minimal LCG seeded from nanos.
        let seed = {
            use std::time::{SystemTime, UNIX_EPOCH};
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0xDEAD_BEEF)
        };

        let mut lcg_state = seed as u64;
        let lcg_next = |state: &mut u64| -> u8 {
            // Numerical Recipes LCG parameters.
            *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            ((*state >> 24) & 0xFF) as u8
        };

        // Random offset within the file.
        let offset = lcg_next(&mut lcg_state) as usize % contents.len();

        // 1 KiB of garbage.
        const CORRUPTION_BYTES: usize = 1024;
        for i in 0..CORRUPTION_BYTES {
            let pos = (offset + i) % contents.len();
            contents[pos] = lcg_next(&mut lcg_state);
        }

        std::fs::write(target_file, &contents)?;

        let idx = self.push_record(FaultRecord {
            fault_type: FaultType::WorktreeCorruption,
            injected_at: Utc::now(),
            recovered_at: None,
            recovery_time_ms: None,
            target: target_file.display().to_string(),
        });
        Ok(idx)
    }

    /// Inject a process kill fault (stub).
    ///
    /// `target_pid` is the OS process identifier to kill. This method records the
    /// intent but does not perform the kill. Use `libc::kill(pid, libc::SIGKILL)`
    /// at the call site after resolving the PID from `ProcessSupervisor`.
    ///
    /// # Errors
    ///
    /// Always returns `Err(ErrorKind::Unsupported)` — OS-level process termination
    /// is not yet implemented in this stub.
    pub fn inject_process_kill(&mut self, target_pid: u32) -> io::Result<usize> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!(
                "chaos: ProcessKill for pid {target_pid} not yet implemented — \
                 use libc::kill(pid, libc::SIGKILL) at the call site"
            ),
        ))
    }

    /// Inject a network partition fault (stub).
    ///
    /// # Errors
    ///
    /// Always returns `Err(ErrorKind::Unsupported)` — OS-level firewall rules are
    /// not yet implemented in this stub.
    pub fn inject_network_partition(&mut self) -> io::Result<usize> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "chaos: NetworkPartition not yet implemented — \
             requires OS-level firewall rule installation",
        ))
    }

    /// Mark the fault at `fault_index` as recovered.
    ///
    /// Computes and stores `recovery_time_ms` as the milliseconds between
    /// `injected_at` and now. Silently ignores out-of-range indices or faults that
    /// were already marked recovered.
    pub fn mark_recovered(&mut self, fault_index: usize) {
        let now = Utc::now();
        if let Some(record) = self.fault_log.get_mut(fault_index) {
            if record.recovered_at.is_none() {
                let delta_ms = (now - record.injected_at).num_milliseconds().max(0) as u64;
                record.recovered_at = Some(now);
                record.recovery_time_ms = Some(delta_ms);
            }
        }
    }

    /// Compute an [`AntifragilityReport`] from the recorded faults.
    ///
    /// `baseline_pass_rate` and `chaos_pass_rate` are both in `[0.0, 1.0]`.
    /// The antifragility index is set to `f64::INFINITY` when `baseline_pass_rate`
    /// is zero to avoid division by zero.
    #[must_use]
    pub fn report(&self, baseline_pass_rate: f64, chaos_pass_rate: f64) -> AntifragilityReport {
        let antifragility_index = if baseline_pass_rate == 0.0 {
            f64::INFINITY
        } else {
            chaos_pass_rate / baseline_pass_rate
        };

        // Average over only those records that have a recovery timestamp.
        let recovered: Vec<u64> = self
            .fault_log
            .iter()
            .filter_map(|r| r.recovery_time_ms)
            .collect();

        let avg_recovery_ms = if recovered.is_empty() {
            None
        } else {
            let sum: u64 = recovered.iter().sum();
            Some(sum / recovered.len() as u64)
        };

        AntifragilityReport {
            baseline_pass_rate,
            chaos_pass_rate,
            antifragility_index,
            total_faults: self.fault_log.len(),
            avg_recovery_ms,
        }
    }

    /// Return a slice of all recorded fault events.
    #[must_use]
    pub fn records(&self) -> &[FaultRecord] {
        &self.fault_log
    }

    // ─── private ─────────────────────────────────────────────────────────

    /// Append `record` to the log and return its index.
    fn push_record(&mut self, record: FaultRecord) -> usize {
        let idx = self.fault_log.len();
        self.fault_log.push(record);
        idx
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_injector() -> (FaultInjector, TempDir) {
        let dir = TempDir::new().expect("tempdir");
        let injector = FaultInjector::new(dir.path().to_path_buf());
        (injector, dir)
    }

    // ─── CompileWarningInjection ─────────────────────────────────────────

    #[test]
    fn inject_compile_warning_prepends_deprecated_attr() {
        let (mut injector, dir) = make_injector();
        let file = dir.path().join("lib.rs");
        fs::write(
            &file,
            "// a module\npub fn hello() -> &'static str { \"world\" }\n",
        )
        .unwrap();

        let idx = injector
            .inject_compile_warning(&file)
            .expect("injection succeeds");

        let patched = fs::read_to_string(&file).unwrap();
        assert!(
            patched.contains("#[deprecated(note = \"chaos-injection\")]"),
            "attribute must be present in patched file"
        );
        // The attribute must appear before the pub fn line.
        let attr_pos = patched.find("#[deprecated").unwrap();
        let fn_pos = patched.find("pub fn hello").unwrap();
        assert!(attr_pos < fn_pos, "attribute must precede the fn");

        // Record was appended.
        assert_eq!(injector.records().len(), 1);
        assert_eq!(idx, 0);
        assert!(matches!(
            injector.records()[0].fault_type,
            FaultType::CompileWarningInjection
        ));
        assert!(injector.records()[0].recovered_at.is_none());
    }

    #[test]
    fn inject_compile_warning_recognises_pub_struct() {
        let (mut injector, dir) = make_injector();
        let file = dir.path().join("types.rs");
        fs::write(&file, "pub struct Config {\n    pub value: u32,\n}\n").unwrap();

        injector
            .inject_compile_warning(&file)
            .expect("injection succeeds on pub struct");

        let patched = fs::read_to_string(&file).unwrap();
        assert!(patched.contains("#[deprecated(note = \"chaos-injection\")]"));
        let attr_pos = patched.find("#[deprecated").unwrap();
        let struct_pos = patched.find("pub struct Config").unwrap();
        assert!(attr_pos < struct_pos);
    }

    #[test]
    fn inject_compile_warning_returns_error_when_no_pub_item_found() {
        let (mut injector, dir) = make_injector();
        let file = dir.path().join("private.rs");
        fs::write(&file, "fn private_fn() {}\n").unwrap();

        let result = injector.inject_compile_warning(&file);
        assert!(
            result.is_err(),
            "should fail when no pub fn or pub struct exists"
        );
        // No record should be appended on failure.
        assert_eq!(injector.records().len(), 0);
    }

    #[test]
    fn inject_compile_warning_preserves_trailing_newline() {
        let (mut injector, dir) = make_injector();
        let file = dir.path().join("newline.rs");
        let original = "pub fn foo() {}\n";
        fs::write(&file, original).unwrap();

        injector.inject_compile_warning(&file).unwrap();

        let patched = fs::read_to_string(&file).unwrap();
        assert!(
            patched.ends_with('\n'),
            "trailing newline must be preserved"
        );
    }

    #[test]
    fn inject_compile_warning_multiple_injections_append_records() {
        let (mut injector, dir) = make_injector();

        let file_a = dir.path().join("a.rs");
        let file_b = dir.path().join("b.rs");
        fs::write(&file_a, "pub fn alpha() {}\n").unwrap();
        fs::write(&file_b, "pub struct Beta;\n").unwrap();

        let idx_a = injector.inject_compile_warning(&file_a).unwrap();
        let idx_b = injector.inject_compile_warning(&file_b).unwrap();

        assert_eq!(idx_a, 0);
        assert_eq!(idx_b, 1);
        assert_eq!(injector.records().len(), 2);
    }

    // ─── GateDelay ───────────────────────────────────────────────────────

    #[test]
    fn inject_gate_delay_records_fault() {
        let (mut injector, _dir) = make_injector();

        let idx = injector.inject_gate_delay(250);

        assert_eq!(idx, 0);
        assert_eq!(injector.records().len(), 1);
        assert!(matches!(
            injector.records()[0].fault_type,
            FaultType::GateDelay { ms: 250 }
        ));
        assert!(injector.records()[0].target.contains("250ms"));
    }

    // ─── WorktreeCorruption ──────────────────────────────────────────────

    #[test]
    fn inject_worktree_corruption_changes_file_content() {
        let (mut injector, dir) = make_injector();
        let file = dir.path().join("data.bin");
        let original = vec![0u8; 4096];
        fs::write(&file, &original).unwrap();

        let idx = injector
            .inject_worktree_corruption(&file)
            .expect("corruption succeeds");

        let corrupted = fs::read(&file).unwrap();
        assert_eq!(corrupted.len(), original.len(), "file length unchanged");
        assert_ne!(corrupted, original, "content must differ after corruption");
        assert_eq!(idx, 0);
        assert!(matches!(
            injector.records()[0].fault_type,
            FaultType::WorktreeCorruption
        ));
    }

    #[test]
    fn inject_worktree_corruption_fails_on_empty_file() {
        let (mut injector, dir) = make_injector();
        let file = dir.path().join("empty.bin");
        fs::write(&file, b"").unwrap();

        let result = injector.inject_worktree_corruption(&file);
        assert!(result.is_err(), "should fail on empty file");
        assert_eq!(injector.records().len(), 0);
    }

    #[test]
    fn inject_worktree_corruption_leaves_file_same_length() {
        let (mut injector, dir) = make_injector();
        let file = dir.path().join("source.rs");
        let content = b"pub fn example() -> u32 { 42 }";
        fs::write(&file, content).unwrap();

        injector.inject_worktree_corruption(&file).unwrap();

        let result = fs::read(&file).unwrap();
        assert_eq!(
            result.len(),
            content.len(),
            "file length must not change after corruption"
        );
    }

    // ─── Stubs ───────────────────────────────────────────────────────────

    #[test]
    fn inject_process_kill_is_unsupported() {
        let (mut injector, _dir) = make_injector();
        let result = injector.inject_process_kill(12345);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::Unsupported);
        // Stub must not append a record.
        assert_eq!(injector.records().len(), 0);
    }

    #[test]
    fn inject_network_partition_is_unsupported() {
        let (mut injector, _dir) = make_injector();
        let result = injector.inject_network_partition();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::Unsupported);
        assert_eq!(injector.records().len(), 0);
    }

    // ─── Recovery ────────────────────────────────────────────────────────

    #[test]
    fn mark_recovered_sets_timestamp_and_duration() {
        let (mut injector, _dir) = make_injector();
        let idx = injector.inject_gate_delay(100);

        injector.mark_recovered(idx);

        let record = &injector.records()[idx];
        assert!(record.recovered_at.is_some(), "recovered_at must be set");
        assert!(
            record.recovery_time_ms.is_some(),
            "recovery_time_ms must be set"
        );
    }

    #[test]
    fn mark_recovered_is_idempotent() {
        let (mut injector, _dir) = make_injector();
        let idx = injector.inject_gate_delay(50);

        injector.mark_recovered(idx);
        let first_ts = injector.records()[idx].recovered_at;
        injector.mark_recovered(idx);
        let second_ts = injector.records()[idx].recovered_at;

        assert_eq!(
            first_ts, second_ts,
            "second call must not overwrite recovery timestamp"
        );
    }

    #[test]
    fn mark_recovered_ignores_out_of_range_index() {
        let (mut injector, _dir) = make_injector();
        // Should not panic.
        injector.mark_recovered(999);
        assert_eq!(injector.records().len(), 0);
    }

    // ─── AntifragilityReport ─────────────────────────────────────────────

    #[test]
    fn report_computes_antifragility_index() {
        let (mut injector, _dir) = make_injector();
        let idx = injector.inject_gate_delay(10);
        injector.mark_recovered(idx);

        let report = injector.report(0.8, 0.6);

        assert!((report.baseline_pass_rate - 0.8).abs() < f64::EPSILON);
        assert!((report.chaos_pass_rate - 0.6).abs() < f64::EPSILON);
        // 0.6 / 0.8 = 0.75
        assert!((report.antifragility_index - 0.75).abs() < 1e-10);
        assert_eq!(report.total_faults, 1);
        assert!(report.avg_recovery_ms.is_some());
    }

    #[test]
    fn report_infinity_when_baseline_is_zero() {
        let (injector, _dir) = make_injector();
        let report = injector.report(0.0, 0.5);
        assert!(report.antifragility_index.is_infinite());
    }

    #[test]
    fn report_no_avg_recovery_when_no_faults_recovered() {
        let (mut injector, _dir) = make_injector();
        injector.inject_gate_delay(200); // not marked recovered

        let report = injector.report(1.0, 1.0);
        assert!(
            report.avg_recovery_ms.is_none(),
            "avg_recovery_ms must be None when no faults recovered"
        );
        assert_eq!(report.total_faults, 1);
    }

    #[test]
    fn report_avg_recovery_averages_multiple_records() {
        let (mut injector, _dir) = make_injector();

        // Inject three gate delays and mark all recovered; set synthetic
        // recovery_time_ms values by directly mutating through mark_recovered
        // (recovery times will be near-zero since we call mark_recovered
        // immediately — we just verify the averaging logic with real calls).
        for _ in 0..3 {
            let idx = injector.inject_gate_delay(0);
            injector.mark_recovered(idx);
        }

        let report = injector.report(0.9, 0.9);
        assert_eq!(report.total_faults, 3);
        assert!(report.avg_recovery_ms.is_some());
    }

    #[test]
    fn report_antifragility_greater_than_one_is_valid() {
        let (injector, _dir) = make_injector();
        // chaos rate > baseline rate => antifragile
        let report = injector.report(0.5, 0.9);
        assert!(
            report.antifragility_index > 1.0,
            "index > 1.0 indicates antifragility"
        );
    }

    // ─── Workdir accessor ────────────────────────────────────────────────

    #[test]
    fn workdir_accessor_returns_configured_path() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().to_path_buf();
        let injector = FaultInjector::new(path.clone());
        assert_eq!(injector.workdir(), path.as_path());
    }
}
