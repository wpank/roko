//! CLI subcommands for inspecting the custody audit chain.
//!
//! Provides `list`, `show`, and `verify` commands for operators to inspect
//! and validate the append-only custody record chain.
//!
//! Records are hash-chained: each entry carries a SHA-256 digest of its
//! canonical payload plus a `prev_hash` linking to the preceding record
//! ([`Custody::compute_hash`]). `cmd_custody_verify` recomputes the chain so
//! edited or reordered rows are detected.

use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use roko_agent::safety::provenance::{Custody, CustodyLogger};
use roko_fs::RokoLayout;

// ─── Hash-chain helpers ────────────────────────────────────────────

/// An exclusive lock on a custody log, held until it drops: the OS's
/// advisory lock on `<log>.lock`, so that appends from other processes wait
/// their turn instead of forking the chain (bug-2ae60f).
struct CustodyLock {
    _file: std::fs::File,
}

impl CustodyLock {
    /// Wait for the lock on the custody log at `log`.
    fn acquire(log: &Path) -> std::io::Result<Self> {
        if let Some(parent) = log.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut path = log.as_os_str().to_owned();
        path.push(".lock");
        let file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(PathBuf::from(path))?;
        file.lock()?;
        Ok(Self { _file: file })
    }
}

/// Append a custody record to the log with hash-chain fields populated, and
/// return the record's hash: the chain's new head.
///
/// Under the log's lock, [`CustodyLogger::log_chained`] links the record to
/// the last record's hash and seals it with [`Custody::compute_hash`].
pub fn log_chained(logger: &CustodyLogger, mut record: Custody) -> std::io::Result<String> {
    let _lock = CustodyLock::acquire(logger.path())?;
    logger.log_chained(&mut record)?;
    Ok(record.hash.unwrap_or_default())
}

/// Check `record`, line `idx` of a custody log, against the chain before it.
///
/// A sealed record's `prev_hash` must name `expected`, the hash of the sealed
/// record before it, and its hash must match its payload. A legacy record
/// without a hash restarts the chain. Pushes what is wrong onto `violations`,
/// moves `expected` on, and returns whether `record` is sealed.
fn check_chain_link(
    idx: usize,
    record: &Custody,
    expected: &mut Option<String>,
    violations: &mut Vec<String>,
) -> bool {
    let Some(stored_hash) = &record.hash else {
        *expected = None;
        return false;
    };
    // Verify prev_hash links to the previous record's hash.
    match (&record.prev_hash, &*expected) {
        (None, None) => { /* first chained record, OK */ }
        (Some(prev), Some(expected)) if prev == expected => { /* link OK */ }
        (None, Some(expected)) => {
            violations.push(format!(
                "line {idx}: chain break — prev_hash is missing, expected {:.16}...",
                expected
            ));
        }
        (Some(prev), None) => {
            violations.push(format!(
                "line {idx}: chain break — prev_hash is {:.16}... but no prior hash exists",
                prev
            ));
        }
        (Some(prev), Some(expected)) => {
            violations.push(format!(
                "line {idx}: chain break — prev_hash {:.16}... != expected {:.16}...",
                prev, expected
            ));
        }
    }
    // Recompute the hash and compare.
    let recomputed = record.compute_hash();
    if *stored_hash != recomputed {
        violations.push(format!(
            "line {idx}: hash mismatch — stored {:.16}... != recomputed {:.16}...",
            stored_hash, recomputed
        ));
    }
    *expected = Some(stored_hash.clone());
    true
}

/// What is wrong with the hash chain of the custody records `records`, in
/// order: empty when it verifies. `start` is the hash the first sealed record
/// must link to, `None` for the start of a log. `cmd_custody_verify` checks
/// the same links.
pub(crate) fn chain_violations(records: &[Custody], start: Option<String>) -> Vec<String> {
    let mut violations = Vec::new();
    let mut expected = start;
    for (idx, record) in records.iter().enumerate() {
        check_chain_link(idx, record, &mut expected, &mut violations);
    }
    violations
}

// ─── CLI commands ──────────────────────────────────────────────────

/// List recent custody records, optionally limited to `limit` entries.
pub fn cmd_custody_list(workdir: &Path, limit: Option<usize>) -> Result<()> {
    let layout = RokoLayout::for_project(workdir);
    let logger = CustodyLogger::new(layout.custody_log());
    let records = logger
        .read_all()
        .map_err(|e| anyhow!("failed to read custody log: {e}"))?;

    if records.is_empty() {
        eprintln!(
            "No custody records found at {}",
            layout.custody_log().display()
        );
        return Ok(());
    }

    let display_records: Vec<&Custody> = if let Some(limit) = limit {
        records
            .iter()
            .rev()
            .take(limit)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    } else {
        records.iter().collect()
    };

    println!(
        "{:<6} {:<20} {:<24} {:<14} {:<12} {}",
        "#", "ACTION", "PRINCIPAL", "TAINT", "GATES", "WHEN"
    );
    println!("{}", "-".repeat(90));

    for (idx, record) in records.iter().enumerate() {
        if limit.is_some() && !display_records.contains(&record) {
            continue;
        }
        let taint = record
            .taint
            .as_ref()
            .map(|t| format!("{t:?}"))
            .unwrap_or_else(|| "none".to_string());
        let taint_display = if taint.len() > 12 {
            format!("{}...", &taint[..9])
        } else {
            taint
        };
        let gates = if record.gates_passed.is_empty() {
            "-".to_string()
        } else {
            format!("{}", record.gates_passed.len())
        };
        let when = chrono_format(record.when);
        let action_display = if record.action.len() > 18 {
            format!("{}...", &record.action[..15])
        } else {
            record.action.clone()
        };
        let principal_display = if record.principal.len() > 22 {
            format!("{}...", &record.principal[..19])
        } else {
            record.principal.clone()
        };
        println!(
            "{:<6} {:<20} {:<24} {:<14} {:<12} {}",
            idx, action_display, principal_display, taint_display, gates, when
        );
    }

    println!("\nTotal: {} records", records.len());
    Ok(())
}

/// Show a single custody record by index.
pub fn cmd_custody_show(workdir: &Path, index: usize) -> Result<()> {
    let layout = RokoLayout::for_project(workdir);
    let logger = CustodyLogger::new(layout.custody_log());
    let records = logger
        .read_all()
        .map_err(|e| anyhow!("failed to read custody log: {e}"))?;

    if records.is_empty() {
        anyhow::bail!(
            "No custody records found at {}",
            layout.custody_log().display()
        );
    }

    let record = records
        .get(index)
        .ok_or_else(|| anyhow!("record index {index} out of range (0..{})", records.len()))?;

    println!("Custody Record #{index}");
    println!("{}", "=".repeat(50));
    println!("Action:      {}", record.action);
    println!("Principal:   {}", record.principal);
    println!(
        "When:        {} ({})",
        chrono_format(record.when),
        record.when
    );
    println!(
        "Taint:       {}",
        record
            .taint
            .as_ref()
            .map(|t| format!("{t:?}"))
            .unwrap_or_else(|| "none".to_string())
    );
    println!(
        "Attestation: {}",
        record
            .attestation
            .as_ref()
            .map(|a| format!("{a:?}"))
            .unwrap_or_else(|| "none".to_string())
    );
    println!(
        "Result:      {}",
        record.result.as_deref().unwrap_or("none")
    );
    println!(
        "Witness:     {}",
        record.witness.as_deref().unwrap_or("none")
    );
    println!(
        "Simulation:  {}",
        record.simulation.as_deref().unwrap_or("none")
    );
    println!("Hash:        {}", record.hash.as_deref().unwrap_or("none"));
    println!(
        "Prev hash:   {}",
        record.prev_hash.as_deref().unwrap_or("none")
    );

    if !record.gates_passed.is_empty() {
        println!("\nGates passed:");
        for gate in &record.gates_passed {
            println!("  - {gate}");
        }
    }

    if !record.authorized.is_empty() {
        println!("\nAuthorization evidence:");
        for ev in &record.authorized {
            println!("  - [{:?}] {}", ev.source, ev.detail);
        }
    }

    if !record.why_heuristics.is_empty() {
        println!("\nHeuristics:");
        for h in &record.why_heuristics {
            println!("  - {h}");
        }
    }

    if !record.why_claims.is_empty() {
        println!("\nClaims:");
        for c in &record.why_claims {
            println!("  - {c}");
        }
    }

    Ok(())
}

/// Verify integrity of the custody chain.
///
/// Checks for:
/// - Empty custody log
/// - Parse errors (corrupted lines)
/// - Monotonic timestamps
/// - Missing required fields
/// - Hash chain integrity (prev_hash links + recomputed sha256 digests)
pub fn cmd_custody_verify(workdir: &Path) -> Result<()> {
    let layout = RokoLayout::for_project(workdir);
    let log_path = layout.custody_log();

    if !log_path.exists() {
        anyhow::bail!("No custody log found at {}", log_path.display());
    }

    let content = std::fs::read_to_string(&log_path)
        .map_err(|e| anyhow!("failed to read custody log: {e}"))?;

    let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.is_empty() {
        anyhow::bail!("Custody log is empty.");
    }

    let mut violations = Vec::new();
    let mut prev_when: Option<i64> = None;
    let mut valid_count = 0usize;
    let mut parse_errors = 0usize;
    let mut prev_hash_expected: Option<String> = None;
    let mut chain_checked = false;

    for (idx, line) in lines.iter().enumerate() {
        match serde_json::from_str::<Custody>(line) {
            Ok(record) => {
                valid_count += 1;

                // Check monotonic timestamps.
                if let Some(prev) = prev_when {
                    if record.when < prev {
                        violations.push(format!(
                            "line {idx}: timestamp regression ({} < {prev})",
                            record.when
                        ));
                    }
                }
                prev_when = Some(record.when);

                // Check required fields are non-empty.
                if record.action.is_empty() {
                    violations.push(format!("line {idx}: empty action field"));
                }
                if record.principal.is_empty() {
                    violations.push(format!("line {idx}: empty principal field"));
                }

                // ── Hash chain verification ──────────────────────────
                // Legacy records without hash fields are allowed; they
                // restart the chain.
                if check_chain_link(idx, &record, &mut prev_hash_expected, &mut violations) {
                    chain_checked = true;
                }
            }
            Err(e) => {
                parse_errors += 1;
                violations.push(format!("line {idx}: parse error: {e}"));
            }
        }
    }

    println!("Custody chain verification");
    println!("{}", "=".repeat(40));
    println!("Total lines:    {}", lines.len());
    println!("Valid records:  {valid_count}");
    println!("Parse errors:   {parse_errors}");
    println!(
        "Hash chain:     {}",
        if chain_checked {
            "verified"
        } else {
            "not present (legacy)"
        }
    );
    println!("Violations:     {}", violations.len());

    if violations.is_empty() {
        println!("\nChain integrity: OK");
        Ok(())
    } else {
        println!("\nViolations:");
        for v in &violations {
            println!("  - {v}");
        }
        Err(anyhow!("{} integrity violation(s) found", violations.len()))
    }
}

/// Format a Unix-millis timestamp as a human-readable string.
fn chrono_format(millis: i64) -> String {
    use std::time::{Duration, UNIX_EPOCH};

    let secs = (millis / 1000) as u64;
    let nanos = ((millis % 1000) * 1_000_000) as u32;
    match UNIX_EPOCH.checked_add(Duration::new(secs, nanos)) {
        Some(time) => {
            let datetime: std::time::SystemTime = time;
            // Use a simple ISO-ish format without pulling in chrono.
            let elapsed = datetime
                .duration_since(UNIX_EPOCH)
                .unwrap_or(Duration::ZERO);
            let total_secs = elapsed.as_secs();
            let days = total_secs / 86400;
            let day_secs = total_secs % 86400;
            let hours = day_secs / 3600;
            let mins = (day_secs % 3600) / 60;
            let secs = day_secs % 60;
            // Approximate year/month/day from days since epoch.
            // Good enough for display; not calendar-accurate for leap seconds.
            format!("epoch+{days}d {hours:02}:{mins:02}:{secs:02}",)
        }
        None => format!("{millis}ms"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_agent::safety::provenance::{AttestationLevel, Taint};
    use tempfile::TempDir;

    fn setup_custody_log(tmp: &TempDir) -> CustodyLogger {
        let layout = RokoLayout::for_project(tmp.path());
        CustodyLogger::new(layout.custody_log())
    }

    #[test]
    fn list_empty_log_succeeds() {
        let tmp = TempDir::new().unwrap();
        // No custody log file exists; cmd_custody_list should not error.
        assert!(cmd_custody_list(tmp.path(), None).is_ok());
    }

    #[test]
    fn list_with_records() {
        let tmp = TempDir::new().unwrap();
        let logger = setup_custody_log(&tmp);
        log_chained(&logger, Custody::new("write_file", "agent-1", 1000, vec![])).unwrap();
        log_chained(
            &logger,
            Custody::new("bash", "agent-2", 2000, vec![]).with_taint(Taint::UserInput),
        )
        .unwrap();

        assert!(cmd_custody_list(tmp.path(), None).is_ok());
        assert!(cmd_custody_list(tmp.path(), Some(1)).is_ok());
    }

    #[test]
    fn show_record_by_index() {
        let tmp = TempDir::new().unwrap();
        let logger = setup_custody_log(&tmp);
        log_chained(
            &logger,
            Custody::new("edit_file", "agent-3", 3000, vec![])
                .with_attestation(AttestationLevel::LocalAgent)
                .with_gates_passed(vec!["compile".into(), "test".into()]),
        )
        .unwrap();

        assert!(cmd_custody_show(tmp.path(), 0).is_ok());
    }

    #[test]
    fn show_out_of_range_fails() {
        let tmp = TempDir::new().unwrap();
        let logger = setup_custody_log(&tmp);
        log_chained(&logger, Custody::new("test", "p", 100, vec![])).unwrap();

        assert!(cmd_custody_show(tmp.path(), 99).is_err());
    }

    #[test]
    fn verify_valid_chain_succeeds() {
        let tmp = TempDir::new().unwrap();
        let logger = setup_custody_log(&tmp);
        log_chained(&logger, Custody::new("action1", "agent-1", 1000, vec![])).unwrap();
        log_chained(&logger, Custody::new("action2", "agent-1", 2000, vec![])).unwrap();

        assert!(cmd_custody_verify(tmp.path()).is_ok());
    }

    #[test]
    fn verify_detects_timestamp_regression() {
        let tmp = TempDir::new().unwrap();
        let logger = setup_custody_log(&tmp);
        // Second record has an earlier timestamp.
        log_chained(&logger, Custody::new("action1", "agent-1", 2000, vec![])).unwrap();
        log_chained(&logger, Custody::new("action2", "agent-1", 1000, vec![])).unwrap();

        assert!(cmd_custody_verify(tmp.path()).is_err());
    }

    #[test]
    fn custody_verify_detects_tampered_chain() {
        let tmp = TempDir::new().unwrap();
        let logger = setup_custody_log(&tmp);

        // Write two properly chained records.
        log_chained(&logger, Custody::new("action1", "agent-1", 1000, vec![])).unwrap();
        log_chained(&logger, Custody::new("action2", "agent-1", 2000, vec![])).unwrap();

        // Sanity: chain is valid before tampering.
        assert!(cmd_custody_verify(tmp.path()).is_ok());

        // ── Tamper with the first record's action field ──
        let log_path = logger.path().to_path_buf();
        let content = std::fs::read_to_string(&log_path).unwrap();
        let tampered = content.replacen("action1", "HACKED!", 1);
        assert_ne!(content, tampered, "sanity: replacement happened");
        std::fs::write(&log_path, &tampered).unwrap();

        // Verify must now fail: the first record's hash won't match the
        // recomputed digest, and the second record's prev_hash will also
        // be wrong since it was computed from the original first record.
        let result = cmd_custody_verify(tmp.path());
        assert!(result.is_err(), "tampered chain must fail verification");
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("integrity violation"),
            "error should mention integrity violation, got: {err_msg}"
        );
    }

    #[test]
    fn chained_records_carry_hashes() {
        let tmp = TempDir::new().unwrap();
        let logger = setup_custody_log(&tmp);

        log_chained(&logger, Custody::new("a1", "p1", 100, vec![])).unwrap();
        log_chained(&logger, Custody::new("a2", "p2", 200, vec![])).unwrap();

        let records = logger.read_all().unwrap();
        assert_eq!(records.len(), 2);

        // First record has hash but no prev_hash.
        assert!(records[0].hash.is_some(), "first record should have hash");
        assert!(
            records[0].prev_hash.is_none(),
            "first record should have no prev_hash"
        );

        // Second record has both hash and prev_hash.
        assert!(records[1].hash.is_some(), "second record should have hash");
        assert!(
            records[1].prev_hash.is_some(),
            "second record should have prev_hash"
        );

        // Second record's prev_hash should match first record's hash.
        assert_eq!(
            records[1].prev_hash.as_deref(),
            records[0].hash.as_deref(),
            "prev_hash chain link mismatch"
        );
    }

    /// Records each child of `custody_appends_are_serialized_across_processes`
    /// appends.
    const CHILD_APPENDS: i64 = 25;

    /// The child half of `custody_appends_are_serialized_across_processes`:
    /// with `CUSTODY_TEST_LOG` set, it appends [`CHILD_APPENDS`] records to
    /// that log. Run on its own, it does nothing.
    #[test]
    #[ignore = "a child process of custody_appends_are_serialized_across_processes"]
    fn custody_append_child() {
        let Ok(log) = std::env::var("CUSTODY_TEST_LOG") else {
            return;
        };
        let writer = std::env::var("CUSTODY_TEST_WRITER").unwrap_or_default();
        let logger = CustodyLogger::new(log);
        for index in 0..CHILD_APPENDS {
            let record = Custody::new(format!("{writer}-{index}"), "child", index, vec![]);
            log_chained(&logger, record).expect("append a record");
        }
    }

    #[test]
    fn custody_appends_are_serialized_across_processes() {
        let tmp = TempDir::new().expect("tempdir");
        let log = tmp.path().join("custody.jsonl");
        let exe = std::env::current_exe().expect("the test binary");
        let children: Vec<std::process::Child> = ["a", "b", "c"]
            .into_iter()
            .map(|writer| {
                std::process::Command::new(&exe)
                    .args([
                        "--exact",
                        "custody::tests::custody_append_child",
                        "--ignored",
                    ])
                    .env("CUSTODY_TEST_LOG", &log)
                    .env("CUSTODY_TEST_WRITER", writer)
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()
                    .expect("start a child")
            })
            .collect();
        for mut child in children {
            assert!(child.wait().expect("a child").success());
        }

        // Three processes appended at once, and the chain did not fork.
        let records = CustodyLogger::new(&log).read_all().expect("custody log");
        assert_eq!(records.len(), 3 * CHILD_APPENDS as usize);
        assert_eq!(chain_violations(&records, None), Vec::<String>::new());
    }

    #[test]
    fn chrono_format_produces_readable_output() {
        // 2024-01-01T00:00:00Z in millis = 1704067200000
        let result = chrono_format(1704067200000);
        assert!(result.contains(':'));
    }
}
