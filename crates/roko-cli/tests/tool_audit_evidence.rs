//! #384: Tool audit hardening — evidence collection.
//!
//! Runs the key tool-audit checks (secret canary, scrub adapter round-trip,
//! provider health snapshot) and writes a machine-readable evidence bundle to
//! `.roko/evidence/tool-audit/` in a temporary workspace.
//!
//! The evidence directory can be inspected after the test run for audit
//! purposes. The test asserts that all checks pass AND that the bundle file
//! was written correctly.

use std::io::Write;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use roko_core::obs::LogScrubber;
use roko_core::tool::{CorrelationEnvelope, ToolCall, ToolResult};
use roko_fs::tool_audit::{ScrubAuditAdapter, ToolAuditLog};
use roko_learn::provider_health::ProviderHealthRegistry;
use tempfile::TempDir;

// ── Constants ────────────────────────────────────────────────────────────────

const CANARY_SECRET: &str = "toolaudit-canary-xyzzy-99887766";
const CANARY_VAR: &str = "TOOL_AUDIT_CANARY";

// ── Helpers ───────────────────────────────────────────────────────────────────

fn setup_evidence_workspace() -> TempDir {
    let tmp = tempfile::tempdir().expect("tempdir");
    let roko_dir = tmp.path().join(".roko");
    let evidence_dir = roko_dir.join("evidence").join("tool-audit");
    std::fs::create_dir_all(&evidence_dir).expect("create evidence dir");

    // Write a synthetic .env with the canary.
    let env_path = roko_dir.join(".env");
    std::fs::write(
        &env_path,
        format!("{CANARY_VAR}={CANARY_SECRET}\nOTHER=short\n"),
    )
    .expect("write canary .env");

    tmp
}

fn build_scrubber(workspace: &std::path::Path) -> Arc<LogScrubber> {
    let scrubber = Arc::new(LogScrubber::new());
    let env_path = workspace.join(".roko").join(".env");
    if !env_path.is_file() {
        return scrubber;
    }
    let iter = match dotenvy::from_path_iter(&env_path) {
        Ok(it) => it,
        Err(_) => return scrubber,
    };
    for entry in iter.flatten() {
        let (name, value) = entry;
        if value.len() >= 8 {
            let _ = scrubber.add_literal_value(&value, &name);
        }
    }
    scrubber
}

// ── Check 1: secret canary — scrubber catches the canary value ───────────────

fn check_canary_scrubbing(scrubber: &LogScrubber) -> serde_json::Value {
    let start = Instant::now();

    let text_with_canary = format!("agent output contained: {CANARY_SECRET}");
    let scrubbed = scrubber.scrub(&text_with_canary);
    let canary_absent = !scrubbed.contains(CANARY_SECRET);

    let redacted_marker = format!("[REDACTED:{CANARY_VAR}]");
    let marker_present = scrubbed.contains(&redacted_marker);

    // "short" < 8 chars → must NOT be scrubbed even though "OTHER=short" is in .env.
    let short_val_text = "value short is present";
    let short_val_scrubbed = scrubber.scrub(short_val_text);
    let short_val_passes = short_val_scrubbed.contains("short");

    serde_json::json!({
        "check": "secret_canary",
        "pass": canary_absent && marker_present && short_val_passes,
        "canary_absent_after_scrub": canary_absent,
        "redaction_marker_present": marker_present,
        "short_value_not_scrubbed": short_val_passes,
        "duration_ms": start.elapsed().as_millis() as u64,
    })
}

// ── Check 2: scrub audit adapter round-trip ───────────────────────────────────

async fn check_scrub_audit_adapter(
    workspace: &std::path::Path,
    scrubber: Arc<LogScrubber>,
) -> serde_json::Value {
    let start = Instant::now();
    let audit_path = workspace
        .join(".roko")
        .join("tool_audit_evidence_test.jsonl");

    let log = match ToolAuditLog::open_at(&audit_path).await {
        Ok(l) => Arc::new(l),
        Err(e) => {
            return serde_json::json!({
                "check": "scrub_audit_adapter",
                "pass": false,
                "error": format!("open_at failed: {e}"),
                "duration_ms": start.elapsed().as_millis() as u64,
            });
        }
    };
    let adapter = ScrubAuditAdapter::new(log, scrubber);

    // Admit a call whose arguments contain the canary — must be scrubbed.
    let call = ToolCall::new(
        "evidence-call-001",
        "fs.read_file",
        serde_json::json!({ "path": format!("/data/{CANARY_SECRET}/file.txt") }),
    );
    let _ = adapter
        .record_admit(&call, &CorrelationEnvelope::empty())
        .await;

    // Record a result with the canary in the text — must be scrubbed.
    let result = ToolResult::text(format!("file content: {CANARY_SECRET}"));
    let _ = adapter
        .record_result(&call, &result, &CorrelationEnvelope::empty())
        .await;

    let raw = match std::fs::read_to_string(&audit_path) {
        Ok(s) => s,
        Err(e) => {
            return serde_json::json!({
                "check": "scrub_audit_adapter",
                "pass": false,
                "error": format!("read back failed: {e}"),
                "duration_ms": start.elapsed().as_millis() as u64,
            });
        }
    };

    let canary_absent = !raw.contains(CANARY_SECRET);
    // AuditLine is tagged with "kind":"admit" / "kind":"result" via serde tag.
    let has_admit = raw.contains(r#""admit""#);
    let has_result = raw.contains(r#""result""#);

    serde_json::json!({
        "check": "scrub_audit_adapter",
        "pass": canary_absent && has_admit && has_result,
        "canary_absent_from_jsonl": canary_absent,
        "admit_line_written": has_admit,
        "result_line_written": has_result,
        "lines": raw.lines().count(),
        "duration_ms": start.elapsed().as_millis() as u64,
    })
}

// ── Check 3: provider health snapshot (static) ───────────────────────────────

fn check_provider_health_snapshot() -> serde_json::Value {
    let start = Instant::now();

    // Verify the provider health registry can be instantiated and its
    // snapshot round-trips through JSON.
    let registry = ProviderHealthRegistry::new();
    let snapshot = registry.snapshot();
    let serializable = serde_json::to_string(&snapshot).is_ok();

    serde_json::json!({
        "check": "provider_health_snapshot",
        "pass": serializable,
        "provider_count": snapshot.len(),
        "serializable": serializable,
        "duration_ms": start.elapsed().as_millis() as u64,
    })
}

// ── Evidence bundle writer ────────────────────────────────────────────────────

fn write_evidence_bundle(
    workspace: &std::path::Path,
    checks: &[serde_json::Value],
) -> std::path::PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let evidence_dir = workspace.join(".roko").join("evidence").join("tool-audit");
    std::fs::create_dir_all(&evidence_dir).expect("create evidence dir");

    let bundle_path = evidence_dir.join(format!("audit-{ts}.json"));
    let all_pass = checks.iter().all(|c| c["pass"].as_bool().unwrap_or(false));
    let bundle = serde_json::json!({
        "schema": "tool_audit_evidence/v1",
        "timestamp_secs": ts,
        "all_pass": all_pass,
        "checks": checks,
    });
    let content = serde_json::to_string_pretty(&bundle).expect("serialize evidence bundle");
    let mut f = std::fs::File::create(&bundle_path).expect("create evidence file");
    f.write_all(content.as_bytes()).expect("write evidence");
    bundle_path
}

// ── Main test ──────────────────────────────────────────────────────────────────

/// End-to-end evidence collection: runs all audit checks and writes the
/// result bundle to `.roko/evidence/tool-audit/`.
#[tokio::test]
async fn tool_audit_evidence_collection() {
    let tmp = setup_evidence_workspace();
    let workspace = tmp.path();
    let scrubber = build_scrubber(workspace);

    // Run all checks.
    let canary_check = check_canary_scrubbing(&scrubber);
    let adapter_check = check_scrub_audit_adapter(workspace, Arc::clone(&scrubber)).await;
    let health_check = check_provider_health_snapshot();

    let checks = vec![
        canary_check.clone(),
        adapter_check.clone(),
        health_check.clone(),
    ];

    // Write the evidence bundle.
    let bundle_path = write_evidence_bundle(workspace, &checks);
    assert!(bundle_path.exists(), "evidence bundle must be written");

    // Read back and verify structure.
    let raw = std::fs::read_to_string(&bundle_path).expect("read evidence bundle");
    let parsed: serde_json::Value = serde_json::from_str(&raw).expect("parse evidence bundle");
    assert_eq!(
        parsed["schema"].as_str(),
        Some("tool_audit_evidence/v1"),
        "bundle schema must match"
    );

    // Assert each check passes.
    assert!(
        canary_check["pass"].as_bool().unwrap_or(false),
        "secret canary check must pass; detail: {canary_check}"
    );
    assert!(
        adapter_check["pass"].as_bool().unwrap_or(false),
        "scrub audit adapter check must pass; detail: {adapter_check}"
    );
    assert!(
        health_check["pass"].as_bool().unwrap_or(false),
        "provider health snapshot check must pass; detail: {health_check}"
    );

    assert!(
        parsed["all_pass"].as_bool().unwrap_or(false),
        "all_pass must be true in the evidence bundle"
    );
}
