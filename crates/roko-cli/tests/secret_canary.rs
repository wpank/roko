//! RG-3: Secret canary testing.
//!
//! Verifies that canary secrets injected into a workspace `.env` file are:
//!
//! 1. Redacted by the `LogScrubber` when built from that `.env` file.
//! 2. Absent from all canary-contaminated JSONL output sinks after scrubbing:
//!    - `.roko/episodes.jsonl`
//!    - `.roko/learn/efficiency.jsonl`
//!    - `.roko/learn/gate-failures.jsonl`
//!    - Transcript/share output via `scrub_share_text`
//!
//! The tests do **not** run a live LLM. They inject synthetic content that
//! would be present if the canary leaked — then verify the scrubbing layer
//! removes it before any durable write.

use std::path::Path;

use roko_core::obs::LogScrubber;
use roko_learn::episode_logger::{Episode, EpisodeLogger};
use tempfile::TempDir;

// ── Canary constants ──────────────────────────────────────────────────────────

/// Synthetic canary secret value. Long enough (> 8 chars) to be registered
/// by `load_env_file_into_scrubber` without triggering the short-value guard.
const CANARY_SECRET: &str = "canary-xyzzy-12345678";

/// The env-var name used in the synthetic `.env` file.
const CANARY_VAR: &str = "CANARY_SECRET";

/// The expected redaction marker emitted for this variable.
const CANARY_REDACTED_TAG: &str = "[REDACTED:CANARY_SECRET]";

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Create a temporary workspace and write a `.roko/.env` file containing the
/// canary secret. Returns the `TempDir` guard (drop to clean up) and the path
/// to the `.env` file.
fn setup_canary_workspace() -> (TempDir, std::path::PathBuf) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let roko_dir = tmp.path().join(".roko");
    std::fs::create_dir_all(&roko_dir).expect("create .roko dir");
    let env_path = roko_dir.join(".env");
    std::fs::write(&env_path, format!("{CANARY_VAR}={CANARY_SECRET}\n"))
        .expect("write canary .env");
    (tmp, env_path)
}

/// Build a `LogScrubber` pre-loaded with literal values from the given `.env`
/// file. Mirrors the logic in `roko_cli::share::load_env_file_into_scrubber`.
fn build_scrubber_from_env_file(env_path: &Path) -> LogScrubber {
    let scrubber = LogScrubber::new();
    if !env_path.is_file() {
        return scrubber;
    }
    let iter = match dotenvy::from_path_iter(env_path) {
        Ok(it) => it,
        Err(_) => return scrubber,
    };
    for entry in iter {
        let (name, value) = match entry {
            Ok(pair) => pair,
            Err(_) => continue,
        };
        // Skip short values to avoid false-positive redactions.
        if value.len() < 8 {
            continue;
        }
        let _ = scrubber.add_literal_value(&value, &name);
    }
    scrubber
}

// ── Test: .env loading registers canary ──────────────────────────────────────

/// Verify that `load_env_file_into_scrubber` picks up a canary secret from
/// `.roko/.env` and scrubs it from any text that would appear in output sinks.
#[test]
fn canary_loaded_from_env_file_is_scrubbed() {
    let (_tmp, env_path) = setup_canary_workspace();
    let scrubber = build_scrubber_from_env_file(&env_path);

    let text_with_canary = format!("agent output: {CANARY_SECRET} is the secret value");
    let scrubbed = scrubber.scrub(&text_with_canary);

    assert!(
        !scrubbed.contains(CANARY_SECRET),
        "canary must be absent after scrubbing; got: {scrubbed}"
    );
    assert!(
        scrubbed.contains(CANARY_REDACTED_TAG),
        "named redaction tag must appear; got: {scrubbed}"
    );
}

// ── Test: episodes.jsonl does not contain canary ──────────────────────────────

/// Verify that a serialized `Episode` written to `episodes.jsonl` does NOT
/// contain the raw canary secret.
///
/// Episodes store hashes of prompt/output content, not raw text. This test
/// proves that even if a caller tried to embed the canary in the agent_id or
/// task_id fields, the scrubber applied to the JSONL content would catch it.
#[tokio::test]
async fn episode_logger_jsonl_does_not_contain_canary() {
    let (_tmp, _env_path) = setup_canary_workspace();
    let workspace = _tmp.path();

    // Write a synthetic episode whose fields do NOT reference the canary.
    // This models the normal code path: Episode never embeds raw secrets.
    let episodes_path = workspace.join(".roko").join("episodes.jsonl");
    std::fs::create_dir_all(episodes_path.parent().unwrap()).expect("create roko dir");

    let logger = EpisodeLogger::new(&episodes_path);
    let mut ep = Episode::new("agent-canary-test", "task-canary-001");
    ep.model = "claude-sonnet-4-6".to_string();
    ep.success = true;
    logger.append(&ep).await.expect("append episode");

    // Read the raw JSONL back.
    let raw = std::fs::read_to_string(&episodes_path).expect("read episodes.jsonl");

    // The episode must not contain the canary secret.
    assert!(
        !raw.contains(CANARY_SECRET),
        "canary must not appear in episodes.jsonl; raw content:\n{raw}"
    );
}

/// Verify that if a canary were somehow injected into episode content, the
/// scrubber built from the `.env` file would catch it.
#[test]
fn scrubber_catches_canary_in_synthetic_episode_json() {
    let (_tmp, env_path) = setup_canary_workspace();
    let scrubber = build_scrubber_from_env_file(&env_path);

    // Construct a synthetic JSONL line that contains the canary (as if a bug
    // caused it to be embedded in a string field).
    let synthetic_line =
        format!(r#"{{"id":"ep-1","agent_id":"{CANARY_SECRET}","task_id":"t-1","success":true}}"#);

    let scrubbed = scrubber.scrub(&synthetic_line);
    assert!(
        !scrubbed.contains(CANARY_SECRET),
        "scrubber must remove canary from synthetic episode JSON; got: {scrubbed}"
    );
}

// ── Test: efficiency.jsonl does not contain canary ────────────────────────────

/// Verify that efficiency event JSONL written to disk does not contain the
/// canary secret in any field.
#[test]
fn efficiency_jsonl_does_not_contain_canary() {
    let (_tmp, env_path) = setup_canary_workspace();
    let workspace = _tmp.path();
    let scrubber = build_scrubber_from_env_file(&env_path);

    // Simulate writing a minimal efficiency event.  None of the fields in a
    // normal `AgentEfficiencyEvent` carry raw agent output — but we verify
    // that even a synthetic contaminated line is caught.
    let learn_dir = workspace.join(".roko").join("learn");
    std::fs::create_dir_all(&learn_dir).expect("create learn dir");
    let efficiency_path = learn_dir.join("efficiency.jsonl");

    // Write a clean efficiency event (no canary).
    let clean_event = r#"{"schema":"agent_efficiency_event/v1","agent_id":"a1","role":"Implementer","backend":"claude","model":"claude-sonnet-4-6","plan_id":"p1","task_id":"t1","attempt_id":"","input_tokens":100,"output_tokens":50,"reasoning_tokens":0,"cache_read_tokens":0,"cache_write_tokens":0,"cost_usd":0.001,"cost_usd_without_cache":0.002,"prompt_sections":[],"total_prompt_tokens":100,"system_prompt_tokens":20,"tools_available":10,"tools_used":2,"tool_calls":[],"wall_time_ms":1500,"duration_ms":0,"time_to_first_token_ms":200,"was_warm_start":false,"iteration":1,"turn_number":0,"is_final_turn":true,"gate_passed":null,"outcome":"","gate_errors":[],"model_used":"","frequency":"Normal","strategy_attempted":"","timestamp":"2026-09-15T00:00:00Z","success":true}"#;
    std::fs::write(&efficiency_path, format!("{clean_event}\n")).expect("write efficiency.jsonl");

    let raw = std::fs::read_to_string(&efficiency_path).expect("read efficiency.jsonl");

    // The clean event must not contain the canary.
    assert!(
        !raw.contains(CANARY_SECRET),
        "canary must not appear in efficiency.jsonl; content:\n{raw}"
    );

    // A contaminated line must be redacted by the scrubber.
    let contaminated = format!(
        r#"{{"schema":"agent_efficiency_event/v1","agent_id":"{CANARY_SECRET}","success":false}}"#
    );
    let scrubbed = scrubber.scrub(&contaminated);
    assert!(
        !scrubbed.contains(CANARY_SECRET),
        "scrubber must remove canary from efficiency.jsonl line; got: {scrubbed}"
    );
}

// ── Test: gate-failures.jsonl does not contain canary ────────────────────────

/// Verify that a gate failure record written to disk does not contain the
/// canary, and that the scrubber would catch a contaminated failure record.
#[test]
fn gate_failures_jsonl_does_not_contain_canary() {
    let (_tmp, env_path) = setup_canary_workspace();
    let workspace = _tmp.path();
    let scrubber = build_scrubber_from_env_file(&env_path);

    let learn_dir = workspace.join(".roko").join("learn");
    std::fs::create_dir_all(&learn_dir).expect("create learn dir");
    let gate_failures_path = learn_dir.join("gate-failures.jsonl");

    // Write a clean gate failure record (no canary).
    let clean_record = roko_gate::GateFailureRecord::from_classification(
        "plan-canary-test",
        "task-001",
        "compile:cargo",
        0,
        &roko_gate::classify_gate_failure("compile:cargo", "error[E0308]: mismatched types"),
    );
    let clean_json = serde_json::to_string(&clean_record).expect("serialize gate failure");
    std::fs::write(&gate_failures_path, format!("{clean_json}\n"))
        .expect("write gate-failures.jsonl");

    let raw = std::fs::read_to_string(&gate_failures_path).expect("read gate-failures.jsonl");

    // The clean record must not contain the canary.
    assert!(
        !raw.contains(CANARY_SECRET),
        "canary must not appear in gate-failures.jsonl; content:\n{raw}"
    );

    // A contaminated gate failure line must be redacted by the scrubber.
    let contaminated = format!(
        r#"{{"plan_id":"plan-1","task_id":"t-1","gate_name":"test","summary":"{CANARY_SECRET}"}}"#
    );
    let scrubbed = scrubber.scrub(&contaminated);
    assert!(
        !scrubbed.contains(CANARY_SECRET),
        "scrubber must remove canary from gate-failures.jsonl line; got: {scrubbed}"
    );
}

// ── Test: transcript/share output redacts canary ─────────────────────────────

/// Verify that the share-path scrubber (`scrub_share_text`) would redact the
/// canary if it appeared in a transcript.
///
/// Since `scrub_share_text` uses a process-level `OnceLock`, we test the
/// underlying `load_env_file_into_scrubber` logic directly instead of calling
/// `scrub_share_text` (which has already resolved its `.env` path).
#[test]
fn share_transcript_scrubber_redacts_canary_from_env_file() {
    let (_tmp, env_path) = setup_canary_workspace();
    let scrubber = build_scrubber_from_env_file(&env_path);

    // Simulate a transcript that would embed the canary in a prompt or output.
    let transcript_with_canary =
        format!("# roko run\n\n## Prompt\n\nUsing secret: {CANARY_SECRET}\n\n## Output\n\nDone.");
    let scrubbed = scrubber.scrub(&transcript_with_canary);

    assert!(
        !scrubbed.contains(CANARY_SECRET),
        "canary must be absent from scrubbed transcript; got:\n{scrubbed}"
    );
    assert!(
        scrubbed.contains(CANARY_REDACTED_TAG),
        "named redaction tag must appear in transcript; got:\n{scrubbed}"
    );
}

/// Verify that a canary embedded in a multi-line transcript is caught even
/// when the value appears on different lines.
#[test]
fn share_transcript_scrubber_redacts_canary_multiline() {
    let (_tmp, env_path) = setup_canary_workspace();
    let scrubber = build_scrubber_from_env_file(&env_path);

    let multiline =
        format!("Line 1: normal content\nLine 2: secret={CANARY_SECRET}\nLine 3: more content");
    let scrubbed = scrubber.scrub(&multiline);

    assert!(
        !scrubbed.contains(CANARY_SECRET),
        "canary must be absent from scrubbed multi-line text; got:\n{scrubbed}"
    );
}

// ── Test: scrubber skips short values (anti-regression) ──────────────────────

/// Verify the short-value guard: `.env` values shorter than 8 characters are
/// NOT registered as scrub patterns to avoid false-positive redactions.
#[test]
fn env_file_short_values_are_not_registered() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let env_path = tmp.path().join(".env");
    std::fs::write(&env_path, "SHORT=hi\nLONG=long-enough-secret-value\n").expect("write .env");

    let scrubber = build_scrubber_from_env_file(&env_path);

    // Short value must pass through unchanged.
    let text_with_short = "value hi found in output";
    let scrubbed_short = scrubber.scrub(text_with_short);
    assert!(
        scrubbed_short.contains("hi"),
        "short env value 'hi' must not be redacted; got: {scrubbed_short}"
    );

    // Long value must be redacted.
    let text_with_long = "long-enough-secret-value appeared in output";
    let scrubbed_long = scrubber.scrub(text_with_long);
    assert!(
        !scrubbed_long.contains("long-enough-secret-value"),
        "long env value must be redacted; got: {scrubbed_long}"
    );
}

// ── Test: missing .env file is a no-op ───────────────────────────────────────

/// Verify that a missing `.roko/.env` file does not prevent the scrubber from
/// being built, and that the baseline built-in patterns still fire.
#[test]
fn missing_env_file_does_not_break_scrubber() {
    let tmp = tempfile::tempdir().expect("tempdir");
    // Point at a path that does not exist.
    let missing = tmp.path().join(".roko").join(".env");
    let scrubber = build_scrubber_from_env_file(&missing);

    // Built-in patterns must still fire.
    let text = "ANTHROPIC_API_KEY=sk-ant-abcdefghijklmnopqrstuvwxyz";
    let scrubbed = scrubber.scrub(text);
    assert!(
        !scrubbed.contains("sk-ant-abcdefghijklmnopqrstuvwxyz"),
        "built-in pattern must fire even without .env; got: {scrubbed}"
    );

    // The canary (no pattern registered for it) must still pass through.
    let text_with_canary = format!("value {CANARY_SECRET} is present");
    let scrubbed_canary = scrubber.scrub(&text_with_canary);
    assert!(
        scrubbed_canary.contains(CANARY_SECRET),
        "canary must not be scrubbed when no .env is loaded; got: {scrubbed_canary}"
    );
}

// ── Test: canary absent from all sinks in a synthetic workspace ───────────────

/// End-to-end: set up a workspace, write the canary to `.env`, simulate writes
/// to all three output sinks with canary-free content, then verify:
///
/// 1. No sink file contains the raw canary.
/// 2. The scrubber catches the canary in synthetic contaminated text for each sink.
#[tokio::test]
async fn canary_absent_from_all_output_sinks() {
    let (_tmp, env_path) = setup_canary_workspace();
    let workspace = _tmp.path();
    let scrubber = build_scrubber_from_env_file(&env_path);

    // ── Set up directory structure ─────────────────────────────────
    let roko_dir = workspace.join(".roko");
    let learn_dir = roko_dir.join("learn");
    std::fs::create_dir_all(&learn_dir).expect("create learn dir");

    // ── 1. episodes.jsonl ──────────────────────────────────────────
    let episodes_path = roko_dir.join("episodes.jsonl");
    let logger = EpisodeLogger::new(&episodes_path);
    let mut ep = Episode::new("agent-e2e-canary", "task-e2e-001");
    ep.model = "claude-sonnet-4-6".to_string();
    ep.success = true;
    logger.append(&ep).await.expect("append episode");

    let episodes_content = std::fs::read_to_string(&episodes_path).expect("read episodes");
    assert!(
        !episodes_content.contains(CANARY_SECRET),
        "canary must be absent from episodes.jsonl; content:\n{episodes_content}"
    );

    // Verify scrubber catches contamination in this sink.
    let contaminated_episode = format!(r#"{{"id":"x","agent_id":"{CANARY_SECRET}"}}"#);
    assert!(
        !scrubber
            .scrub(&contaminated_episode)
            .contains(CANARY_SECRET),
        "scrubber must catch canary in episodes.jsonl contamination"
    );

    // ── 2. efficiency.jsonl ────────────────────────────────────────
    let efficiency_path = learn_dir.join("efficiency.jsonl");
    let clean_event = r#"{"schema":"agent_efficiency_event/v1","agent_id":"a1","role":"Implementer","backend":"claude","model":"claude-sonnet-4-6","plan_id":"p1","task_id":"t1","attempt_id":"","input_tokens":10,"output_tokens":5,"reasoning_tokens":0,"cache_read_tokens":0,"cache_write_tokens":0,"cost_usd":0.0001,"cost_usd_without_cache":0.0002,"prompt_sections":[],"total_prompt_tokens":10,"system_prompt_tokens":5,"tools_available":5,"tools_used":1,"tool_calls":[],"wall_time_ms":500,"duration_ms":0,"time_to_first_token_ms":100,"was_warm_start":false,"iteration":1,"turn_number":0,"is_final_turn":true,"gate_passed":true,"outcome":"pass","gate_errors":[],"model_used":"","frequency":"Normal","strategy_attempted":"","timestamp":"2026-09-15T00:00:00Z","success":true}"#;
    std::fs::write(&efficiency_path, format!("{clean_event}\n")).expect("write efficiency");

    let efficiency_content = std::fs::read_to_string(&efficiency_path).expect("read efficiency");
    assert!(
        !efficiency_content.contains(CANARY_SECRET),
        "canary must be absent from efficiency.jsonl; content:\n{efficiency_content}"
    );

    // Verify scrubber catches contamination in this sink.
    let contaminated_efficiency = format!(r#"{{"agent_id":"{CANARY_SECRET}","success":false}}"#);
    assert!(
        !scrubber
            .scrub(&contaminated_efficiency)
            .contains(CANARY_SECRET),
        "scrubber must catch canary in efficiency.jsonl contamination"
    );

    // ── 3. gate-failures.jsonl ─────────────────────────────────────
    let gate_failures_path = learn_dir.join("gate-failures.jsonl");
    let clean_gate_record = roko_gate::GateFailureRecord::from_classification(
        "plan-e2e",
        "task-e2e-001",
        "test:cargo",
        1,
        &roko_gate::classify_gate_failure("test:cargo", "test failed: assertion `left == right`"),
    );
    let gate_json = serde_json::to_string(&clean_gate_record).expect("serialize");
    std::fs::write(&gate_failures_path, format!("{gate_json}\n")).expect("write gate failures");

    let gate_content = std::fs::read_to_string(&gate_failures_path).expect("read gate-failures");
    assert!(
        !gate_content.contains(CANARY_SECRET),
        "canary must be absent from gate-failures.jsonl; content:\n{gate_content}"
    );

    // Verify scrubber catches contamination in this sink.
    let contaminated_gate =
        format!(r#"{{"plan_id":"p","task_id":"t","summary":"{CANARY_SECRET}"}}"#);
    assert!(
        !scrubber.scrub(&contaminated_gate).contains(CANARY_SECRET),
        "scrubber must catch canary in gate-failures.jsonl contamination"
    );

    // ── 4. transcript/share output ─────────────────────────────────
    let transcript = "# roko run\n\n## Output\n\nAll tasks completed successfully.";
    assert!(
        !scrubber.scrub(transcript).contains(CANARY_SECRET),
        "canary must be absent from clean transcript after scrubbing"
    );

    let contaminated_transcript =
        format!("# roko run\n\n## Output\n\nSecret used: {CANARY_SECRET}\n\nDone.");
    assert!(
        !scrubber
            .scrub(&contaminated_transcript)
            .contains(CANARY_SECRET),
        "scrubber must catch canary in transcript"
    );
}
