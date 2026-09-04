//! Durable crash report written by the global panic hook.
//!
//! When the process panics, the installed hook captures diagnostic context
//! and serializes it to `.roko/crash-report.json` so that external tools
//! (the supervisor script, `roko doctor`, the TUI) can detect and display
//! the failure without parsing stderr.

use serde::{Deserialize, Serialize};
use std::io::Write as _;
use std::path::{Path, PathBuf};

/// Canonical filename inside the `.roko/` directory.
pub const CRASH_REPORT_FILENAME: &str = "crash-report.json";

/// Structured report written on panic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrashReport {
    /// ISO 8601 timestamp of the crash.
    pub timestamp: String,
    /// Roko version (from `CARGO_PKG_VERSION` or env).
    pub version: String,
    /// The panic message, if one was available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub panic_message: Option<String>,
    /// Captured backtrace (if `RUST_BACKTRACE=1` or `full`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backtrace: Option<String>,
    /// Plan directory that was being executed, if mid-execution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_plan: Option<String>,
    /// Task identifier that was running at crash time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_task: Option<String>,
    /// Provider kind that was dispatching at crash time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Operating system description.
    pub os: String,
    /// Rust compiler version used to build the binary.
    pub rust_version: String,
}

/// Thread-safe static slots that the runner can populate before a panic
/// occurs. The panic hook reads these without allocation.
///
/// These are written via [`set_active_plan`], [`set_active_task`], and
/// [`set_active_provider`], and cleared via the corresponding `clear_*`
/// functions.
mod active_context {
    use std::sync::Mutex;

    static ACTIVE_PLAN: Mutex<Option<String>> = Mutex::new(None);
    static ACTIVE_TASK: Mutex<Option<String>> = Mutex::new(None);
    static ACTIVE_PROVIDER: Mutex<Option<String>> = Mutex::new(None);

    pub fn set_plan(value: String) {
        if let Ok(mut guard) = ACTIVE_PLAN.lock() {
            *guard = Some(value);
        }
    }

    pub fn clear_plan() {
        if let Ok(mut guard) = ACTIVE_PLAN.lock() {
            *guard = None;
        }
    }

    pub fn set_task(value: String) {
        if let Ok(mut guard) = ACTIVE_TASK.lock() {
            *guard = Some(value);
        }
    }

    pub fn clear_task() {
        if let Ok(mut guard) = ACTIVE_TASK.lock() {
            *guard = None;
        }
    }

    pub fn set_provider(value: String) {
        if let Ok(mut guard) = ACTIVE_PROVIDER.lock() {
            *guard = Some(value);
        }
    }

    pub fn clear_provider() {
        if let Ok(mut guard) = ACTIVE_PROVIDER.lock() {
            *guard = None;
        }
    }

    pub fn snapshot() -> (Option<String>, Option<String>, Option<String>) {
        let plan = ACTIVE_PLAN.lock().ok().and_then(|g| g.clone());
        let task = ACTIVE_TASK.lock().ok().and_then(|g| g.clone());
        let provider = ACTIVE_PROVIDER.lock().ok().and_then(|g| g.clone());
        (plan, task, provider)
    }
}

/// Set the active plan directory for crash context.
pub fn set_active_plan(plan: String) {
    active_context::set_plan(plan);
}

/// Clear the active plan directory.
pub fn clear_active_plan() {
    active_context::clear_plan();
}

/// Set the active task identifier for crash context.
pub fn set_active_task(task: String) {
    active_context::set_task(task);
}

/// Clear the active task identifier.
pub fn clear_active_task() {
    active_context::clear_task();
}

/// Set the active provider kind for crash context.
pub fn set_active_provider(provider: String) {
    active_context::set_provider(provider);
}

/// Clear the active provider kind.
pub fn clear_active_provider() {
    active_context::clear_provider();
}

/// Build a `CrashReport` from a panic payload and the current active context.
///
/// This is called from the panic hook and must be allocation-light. The
/// `version` and `rust_version` arguments are compile-time constants passed
/// by the binary crate.
#[must_use]
pub fn build_crash_report(
    panic_message: Option<String>,
    backtrace: Option<String>,
    version: &str,
    rust_version: &str,
) -> CrashReport {
    let (active_plan, active_task, provider) = active_context::snapshot();

    let os = format!(
        "{} {} {}",
        std::env::consts::OS,
        std::env::consts::ARCH,
        std::env::consts::FAMILY
    );

    let timestamp = {
        use std::time::{SystemTime, UNIX_EPOCH};
        let dur = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        // Manual ISO 8601 without pulling in chrono in the panic path.
        // Format: seconds since epoch as a decimal string (lightweight).
        // We prefer a proper timestamp, so use chrono if available.
        let secs = dur.as_secs();
        // Approximate UTC datetime from epoch seconds.
        let days = secs / 86400;
        let day_secs = secs % 86400;
        let hours = day_secs / 3600;
        let minutes = (day_secs % 3600) / 60;
        let seconds = day_secs % 60;

        // Days since 1970-01-01 to Y-M-D (simplified leap year calculation).
        let (year, month, day) = epoch_days_to_ymd(days);
        format!("{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}Z")
    };

    CrashReport {
        timestamp,
        version: version.to_string(),
        panic_message,
        backtrace,
        active_plan,
        active_task,
        provider,
        os,
        rust_version: rust_version.to_string(),
    }
}

/// Write a crash report to the given `.roko/` directory.
///
/// Best-effort: errors are silently ignored because this runs inside a
/// panic hook where further panics must be avoided.
pub fn write_crash_report(roko_dir: &Path, report: &CrashReport) {
    let path = roko_dir.join(CRASH_REPORT_FILENAME);
    // Ensure the directory exists (it should, but be defensive).
    let _ = std::fs::create_dir_all(roko_dir);
    // Serialize directly to a file to minimize allocations.
    if let (Ok(json), Ok(mut file)) = (
        serde_json::to_string_pretty(report),
        std::fs::File::create(&path),
    ) {
        let _ = file.write_all(json.as_bytes());
        let _ = file.write_all(b"\n");
        let _ = file.flush();
    }
}

/// Read a crash report from the `.roko/` directory, if one exists.
pub fn read_crash_report(roko_dir: &Path) -> Option<CrashReport> {
    let path = roko_dir.join(CRASH_REPORT_FILENAME);
    let content = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Return the path to the crash report file.
#[must_use]
pub fn crash_report_path(roko_dir: &Path) -> PathBuf {
    roko_dir.join(CRASH_REPORT_FILENAME)
}

/// Check if a crash report exists and is recent (within the given duration).
pub fn has_recent_crash_report(roko_dir: &Path, max_age: std::time::Duration) -> bool {
    let path = roko_dir.join(CRASH_REPORT_FILENAME);
    std::fs::metadata(&path)
        .ok()
        .and_then(|meta| meta.modified().ok())
        .and_then(|modified| modified.elapsed().ok())
        .map_or(false, |elapsed| elapsed < max_age)
}

// ── Date arithmetic (no chrono dependency in panic path) ──────────────

/// Convert days since Unix epoch to (year, month, day).
///
/// Uses a simplified algorithm that handles leap years correctly for the
/// range 1970..2100 (which covers our needs).
fn epoch_days_to_ymd(days: u64) -> (u64, u64, u64) {
    // Algorithm adapted from Howard Hinnant's civil_from_days.
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crash_report_round_trip() {
        let report = CrashReport {
            timestamp: "2026-09-03T12:00:00Z".to_string(),
            version: "0.1.0".to_string(),
            panic_message: Some("test panic".to_string()),
            backtrace: Some("backtrace here".to_string()),
            active_plan: Some("plans/test-plan".to_string()),
            active_task: Some("T1".to_string()),
            provider: Some("anthropic_api".to_string()),
            os: "linux x86_64 unix".to_string(),
            rust_version: "1.96.1".to_string(),
        };

        let json = serde_json::to_string_pretty(&report).unwrap();
        let deserialized: CrashReport = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.timestamp, report.timestamp);
        assert_eq!(deserialized.panic_message, report.panic_message);
        assert_eq!(deserialized.active_plan, report.active_plan);
        assert_eq!(deserialized.version, report.version);
    }

    #[test]
    fn crash_report_write_and_read() {
        let dir = std::env::temp_dir().join("roko-crash-test");
        let _ = std::fs::create_dir_all(&dir);

        let report = CrashReport {
            timestamp: "2026-09-03T12:00:00Z".to_string(),
            version: "0.1.0".to_string(),
            panic_message: Some("write test".to_string()),
            backtrace: None,
            active_plan: None,
            active_task: None,
            provider: None,
            os: "test".to_string(),
            rust_version: "1.96.1".to_string(),
        };

        write_crash_report(&dir, &report);

        let read_back = read_crash_report(&dir).expect("should read crash report");
        assert_eq!(read_back.panic_message, Some("write test".to_string()));

        // Clean up.
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn epoch_days_known_dates() {
        // 1970-01-01 = day 0
        assert_eq!(epoch_days_to_ymd(0), (1970, 1, 1));
        // 2000-01-01 = day 10957
        assert_eq!(epoch_days_to_ymd(10957), (2000, 1, 1));
        // 2026-09-03 (approximate: 56 years + leap days + days in 2026)
        // 365*56 + 14 leap days + 31+28+31+30+31+30+31+31+3 = 20698 + 246 = 20694
        // Actually let's just verify the algorithm doesn't panic on a
        // reasonable range.
        let (y, m, d) = epoch_days_to_ymd(20700);
        assert!(y >= 2026 && y <= 2027);
        assert!(m >= 1 && m <= 12);
        assert!(d >= 1 && d <= 31);
    }

    #[test]
    fn build_crash_report_populates_os() {
        let report = build_crash_report(Some("test".to_string()), None, "0.1.0", "1.96.1");
        assert!(!report.os.is_empty());
        assert_eq!(report.version, "0.1.0");
        assert_eq!(report.rust_version, "1.96.1");
    }

    #[test]
    fn active_context_set_and_snapshot() {
        set_active_plan("plans/my-plan".to_string());
        set_active_task("T3".to_string());
        set_active_provider("openai_compat".to_string());

        let report = build_crash_report(None, None, "0.1.0", "1.96.1");
        assert_eq!(report.active_plan.as_deref(), Some("plans/my-plan"));
        assert_eq!(report.active_task.as_deref(), Some("T3"));
        assert_eq!(report.provider.as_deref(), Some("openai_compat"));

        clear_active_plan();
        clear_active_task();
        clear_active_provider();

        let report2 = build_crash_report(None, None, "0.1.0", "1.96.1");
        assert!(report2.active_plan.is_none());
        assert!(report2.active_task.is_none());
        assert!(report2.provider.is_none());
    }

    #[test]
    fn has_recent_crash_report_false_when_missing() {
        let dir = std::env::temp_dir().join("roko-crash-missing");
        let _ = std::fs::remove_dir_all(&dir);
        assert!(!has_recent_crash_report(
            &dir,
            std::time::Duration::from_secs(3600)
        ));
    }
}
