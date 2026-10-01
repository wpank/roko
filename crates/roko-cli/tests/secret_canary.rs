//! RG-3 / T036: secret canary testing.
//!
//! A canary secret in a workspace `.env` file must stay out of everything
//! roko writes. roko builds one scrubber per process from the secrets in its
//! `.env` files (those named like credentials, as `CANARY_SECRET` is) and
//! provider keys (`roko_fs::observability::RunScrubber::install`); its log
//! layers scrub with it, and its persistence writers redact its secrets from
//! what they write.
//!
//! - `canary_absent_from_every_file_after_plan_run` is the end-to-end check:
//!   a real `roko plan run` whose agent and verify command print the canary
//!   leaves no copy of it in any file, in stdout or in stderr.
//! - The other tests push contaminated records through the shared writers
//!   (episodes, the JSONL boundary the efficiency log uses, gate failures)
//!   with the production scrubber installed, and check the builder's rules.
//!
//! No live LLM runs: the plan run uses a scripted fake Claude CLI.
//!
//! `roko_fs::trace_sink::JsonlTraceSink`, `roko_fs::tool_audit::ToolAuditLog`
//! and `roko_core::transcript_store::TranscriptStore` have no production
//! writer yet, so no canary covers them. Each must gain one here when it is
//! wired into a run.

mod common;

use std::path::Path;
use std::sync::Arc;

use roko_core::obs::LogScrubber;
use roko_fs::observability::RunScrubber;
use roko_learn::episode_logger::{Episode, EpisodeLogger};
use tempfile::TempDir;

// ── Canary constants ──────────────────────────────────────────────────────────

/// Synthetic canary secret value. Long enough to count as a secret
/// (`roko_core::obs::scrub::MIN_SECRET_LEN`), and no built-in scrub pattern
/// matches it, so only the configured-secret scrubbing can remove it.
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

/// The `(name, value)` entries of a `.env` file, parsed the way roko loads
/// it at startup; empty when the file is missing.
fn env_file_entries(env_path: &Path) -> Vec<(String, String)> {
    if !env_path.is_file() {
        return Vec::new();
    }
    dotenvy::from_path_iter(env_path)
        .expect("read .env")
        .filter_map(Result::ok)
        .collect()
}

/// The scrubber roko builds from the given `.env` file
/// (`RunScrubber::build_from_env_file`), without installing it.
fn build_scrubber_from_env_file(env_path: &Path) -> Arc<LogScrubber> {
    RunScrubber::build_from_env_file(&env_file_entries(env_path))
}

/// Install the process's scrubber the way roko does at startup, from a
/// canary `.env` file, once per test binary. Every test here uses the same
/// canary, so tests running in parallel share it.
fn install_canary_scrubber() {
    static INSTALLED: std::sync::Once = std::sync::Once::new();
    INSTALLED.call_once(|| {
        let (_tmp, env_path) = setup_canary_workspace();
        RunScrubber::install(&env_file_entries(&env_path));
    });
}

/// The lines of the JSONL file at `path`, each parsed: scrubbing must leave
/// every record valid JSON.
fn read_jsonl(path: &Path) -> (String, Vec<serde_json::Value>) {
    let raw = std::fs::read_to_string(path).expect("read JSONL file");
    let records = raw
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("record still parses"))
        .collect();
    (raw, records)
}

// ── Test: .env loading registers canary ──────────────────────────────────────

/// Verify that the scrubber roko builds from `.roko/.env` redacts the canary
/// from any text that would appear in output sinks.
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

/// An episode whose failure reason and reflection quote the canary, as they
/// would after an agent printed it, reaches `episodes.jsonl` through
/// `EpisodeLogger` with the canary redacted and the record intact.
#[tokio::test]
async fn episode_logger_jsonl_does_not_contain_canary() {
    install_canary_scrubber();
    let (tmp, _env_path) = setup_canary_workspace();
    let episodes_path = tmp.path().join(".roko").join("episodes.jsonl");

    let logger = EpisodeLogger::new(&episodes_path);
    let mut ep = Episode::new("agent-canary-test", "task-canary-001");
    ep.model = "claude-sonnet-4-6".to_string();
    ep.success = false;
    ep.failure_reason = Some(format!("verify printed {CANARY_SECRET}"));
    ep.reflection = Some(format!("the agent echoed {CANARY_VAR}={CANARY_SECRET}"));
    logger.append(&ep).await.expect("append episode");

    let (raw, records) = read_jsonl(&episodes_path);
    assert!(
        !raw.contains(CANARY_SECRET),
        "canary must not appear in episodes.jsonl"
    );
    assert_eq!(
        records[0]["failure_reason"],
        format!("verify printed {CANARY_REDACTED_TAG}")
    );
    // Only the secret is redacted: the record's ids survive.
    assert_eq!(records[0]["task_id"], "task-canary-001");
    assert_eq!(records[0]["model"], "claude-sonnet-4-6");
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

/// An efficiency event whose gate errors quote the canary reaches
/// `efficiency.jsonl` through the shared JSONL boundary the learning
/// runtime appends it with (`roko_fs::log_rotation::append_jsonl_line_sync`)
/// with the canary redacted.
#[test]
fn efficiency_jsonl_does_not_contain_canary() {
    install_canary_scrubber();
    let (tmp, _env_path) = setup_canary_workspace();
    let efficiency_path = tmp
        .path()
        .join(".roko")
        .join("learn")
        .join("efficiency.jsonl");

    let event = serde_json::json!({
        "schema": "agent_efficiency_event/v1",
        "agent_id": "a1",
        "model": "claude-sonnet-4-6",
        "task_id": "t1",
        "gate_errors": [format!("verify printed {CANARY_SECRET}")],
        "outcome": "fail",
        "cost_usd": 0.001,
    });
    roko_fs::log_rotation::append_jsonl_line_sync(
        &efficiency_path,
        event.to_string().as_bytes(),
        10,
    )
    .expect("append efficiency event");

    let (raw, records) = read_jsonl(&efficiency_path);
    assert!(
        !raw.contains(CANARY_SECRET),
        "canary must not appear in efficiency.jsonl"
    );
    assert_eq!(
        records[0]["gate_errors"][0],
        format!("verify printed {CANARY_REDACTED_TAG}")
    );
    assert_eq!(records[0]["cost_usd"], 0.001);
}

// ── Test: gate-failures.jsonl does not contain canary ────────────────────────

/// A gate failure classified from output that quotes the canary reaches
/// `gate-failures.jsonl` through the runner's JSONL writer
/// (`roko_cli::runner::persist::append_jsonl`) with the canary redacted.
#[test]
fn gate_failures_jsonl_does_not_contain_canary() {
    install_canary_scrubber();
    let (tmp, _env_path) = setup_canary_workspace();
    let gate_failures_path = tmp
        .path()
        .join(".roko")
        .join("learn")
        .join("gate-failures.jsonl");

    let record = roko_gate::GateFailureRecord::from_classification(
        "plan-canary-test",
        "task-001",
        "compile:cargo",
        0,
        &roko_gate::classify_gate_failure(
            "compile:cargo",
            &format!("error[E0308]: mismatched types\nkey = \"{CANARY_SECRET}\""),
        ),
    );
    assert!(
        serde_json::to_string(&record)
            .expect("serialize gate failure")
            .contains(CANARY_SECRET),
        "the classified record must quote the canary, or this test proves nothing"
    );
    roko_cli::runner::persist::append_jsonl(&gate_failures_path, &record)
        .expect("append gate failure");

    let (raw, records) = read_jsonl(&gate_failures_path);
    assert!(
        !raw.contains(CANARY_SECRET),
        "canary must not appear in gate-failures.jsonl"
    );
    assert!(raw.contains(CANARY_REDACTED_TAG));
    assert_eq!(records[0]["task_id"], "task-001");
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
/// NOT registered as scrub patterns to avoid false-positive redactions, and
/// neither is a setting whose name is no secret's (bug-cef888).
#[test]
fn env_file_short_values_are_not_registered() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let env_path = tmp.path().join(".env");
    std::fs::write(
        &env_path,
        "SHORT_TOKEN=hi\nLONG_TOKEN=long-enough-secret-value\n\
         LONG_SETTING=long-enough-plain-value\n",
    )
    .expect("write .env");

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

    // A setting stays readable.
    let text_with_setting = "long-enough-plain-value appeared in output";
    let scrubbed_setting = scrubber.scrub(text_with_setting);
    assert!(
        scrubbed_setting.contains("long-enough-plain-value"),
        "a setting must not be redacted; got: {scrubbed_setting}"
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

/// With the production scrubber installed, contaminated records written
/// through the shared writers of all three learning sinks leave no canary,
/// and a transcript scrubbed with the `.env` scrubber leaves none either.
#[tokio::test]
async fn canary_absent_from_all_output_sinks() {
    install_canary_scrubber();
    let (_tmp, env_path) = setup_canary_workspace();
    let workspace = _tmp.path();
    let scrubber = build_scrubber_from_env_file(&env_path);
    let roko_dir = workspace.join(".roko");
    let learn_dir = roko_dir.join("learn");

    // ── 1. episodes.jsonl ──────────────────────────────────────────
    let episodes_path = roko_dir.join("episodes.jsonl");
    let mut ep = Episode::new("agent-e2e-canary", "task-e2e-001");
    ep.reasoning_summary = Some(format!("found {CANARY_SECRET} in the environment"));
    EpisodeLogger::new(&episodes_path)
        .append(&ep)
        .await
        .expect("append episode");

    // ── 2. efficiency.jsonl ────────────────────────────────────────
    let efficiency_path = learn_dir.join("efficiency.jsonl");
    let event = serde_json::json!({ "agent_id": "a1", "outcome": CANARY_SECRET });
    roko_fs::log_rotation::append_jsonl_line_relaxed_sync(
        &efficiency_path,
        event.to_string().as_bytes(),
        10,
    )
    .expect("append efficiency event");

    // ── 3. gate-failures.jsonl ─────────────────────────────────────
    let gate_failures_path = learn_dir.join("gate-failures.jsonl");
    let record = roko_gate::GateFailureRecord::from_classification(
        "plan-e2e",
        "task-e2e-001",
        "test:cargo",
        1,
        &roko_gate::classify_gate_failure(
            "test:cargo",
            &format!("test failed: assertion `left == right` left: {CANARY_SECRET}"),
        ),
    );
    roko_cli::runner::persist::append_jsonl(&gate_failures_path, &record)
        .expect("append gate failure");

    for path in [&episodes_path, &efficiency_path, &gate_failures_path] {
        let (raw, _records) = read_jsonl(path);
        assert!(
            !raw.contains(CANARY_SECRET),
            "canary must be absent from {}",
            path.display()
        );
        assert!(
            raw.contains(CANARY_REDACTED_TAG),
            "{} must carry the redaction",
            path.display()
        );
    }

    // ── 4. transcript/share output ─────────────────────────────────
    let contaminated_transcript =
        format!("# roko run\n\n## Output\n\nSecret used: {CANARY_SECRET}\n\nDone.");
    assert!(
        !scrubber
            .scrub(&contaminated_transcript)
            .contains(CANARY_SECRET),
        "scrubber must catch canary in transcript"
    );
}

// ── Test: a real plan run leaves the canary in no file ───────────────────────

const PLAN_RUN_PLAN: &str = "canary-plan";

/// A fake Claude CLI that leaks the canary the way an agent that ran `env`
/// would: in its text, a tool call's input, the tool's result, its final
/// result and on stderr. It appends to `NOTES.md` so each attempt has a
/// diff, and copies its stream to `fixtures/stream.jsonl` so the test can
/// tell that it emitted the canary.
fn leaky_agent_script() -> String {
    format!(
        r##"#!/bin/sh
set -eu
cat >/dev/null
fixtures=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
printf 'attempt\n' >> NOTES.md
tee -a "$fixtures/stream.jsonl" <<'STREAM'
{{"type":"system","subtype":"init","session_id":"canary","model":"claude-sonnet-4-6","tools":["Bash"]}}
{{"type":"assistant","message":{{"id":"msg_1","model":"claude-sonnet-4-6","content":[{{"type":"text","text":"The environment holds {CANARY_SECRET}."}}],"usage":{{"input_tokens":10,"output_tokens":5}}}}}}
{{"type":"assistant","message":{{"id":"msg_2","model":"claude-sonnet-4-6","content":[{{"type":"tool_use","id":"toolu_canary","name":"Bash","input":{{"command":"echo {CANARY_SECRET}"}}}}],"usage":{{"input_tokens":10,"output_tokens":5}}}}}}
{{"type":"user","message":{{"content":[{{"type":"tool_result","tool_use_id":"toolu_canary","content":"{CANARY_SECRET}"}}]}}}}
{{"type":"result","subtype":"success","session_id":"canary","model":"claude-sonnet-4-6","result":"Done: the key is {CANARY_SECRET}.","total_cost_usd":0.001,"num_turns":2,"usage":{{"input_tokens":20,"output_tokens":10}},"is_error":false}}
STREAM
echo "warning: {CANARY_VAR}={CANARY_SECRET}" >&2
"##
    )
}

/// A verify command that fails and prints the canary, as a check would that
/// runs code an agent wrote with the key in it. It leaves a marker so the
/// test can tell that it ran.
fn leaky_verify_script() -> String {
    format!(
        "#!/bin/sh\n\
         fixtures=$(CDPATH= cd -- \"$(dirname -- \"$0\")\" && pwd)\n\
         : > \"$fixtures/verify.ran\"\n\
         echo \"check failed: NOTES.md mentions {CANARY_SECRET}\"\n\
         echo \"{CANARY_VAR}={CANARY_SECRET}\" >&2\n\
         exit 1\n"
    )
}

/// T1 passes its check; T2's check (`{fixtures}/leaky-verify.sh`) fails and
/// prints the canary, and T2 gets one retry, so gate failures and retry
/// prompts are written too.
fn leaky_plan_tasks() -> String {
    let task = |id: &str, depends_on: &str, verify: &str, retries: u32| {
        format!(
            r#"
[[task]]
id = "{id}"
title = "Update the notes ({id})"
description = "Append a line to NOTES.md."
role = "implementer"
status = "ready"
tier = "focused"
model_hint = "scripted"
files = ["NOTES.md"]
allowed_tools = []
denied_tools = []
mcp_servers = []
depends_on = [{depends_on}]
depends_on_plan = []
acceptance = []
verify = [{{ phase = "structural", command = "{verify}", fail_msg = "the notes check failed" }}]
timeout_secs = 30
max_retries = {retries}
"#
        )
    };
    format!(
        r#"[meta]
plan = "{PLAN_RUN_PLAN}"
iteration = 1
total = 2
done = 0
status = "ready"
max_parallel = 1
estimated_total_minutes = 1
skip_enrichment = true
{}{}"#,
        task("T1", "", "test -f NOTES.md", 0),
        task("T2", "\"T1\"", "sh {fixtures}/leaky-verify.sh", 1),
    )
}

/// End-to-end (T036): `roko plan run` with the canary in the workspace
/// `.roko/.env` and an agent and a check that both print it. No file roko
/// leaves behind, in the repository (`.roko/` and any plan worktree
/// included) or in its home directory, holds the canary, and neither do
/// stdout and stderr. The `.env` file itself is the one exception.
#[cfg(unix)]
#[test]
fn canary_absent_from_every_file_after_plan_run() {
    use common::{ScriptedPlanWorkspace, describe_leak, files_containing, mask_secret};

    let workspace = ScriptedPlanWorkspace::new(
        PLAN_RUN_PLAN,
        &leaky_plan_tasks(),
        &leaky_agent_script(),
        "",
    );
    common::write_executable(
        &workspace.fixtures.join("leaky-verify.sh"),
        &leaky_verify_script(),
    );

    let env_file = workspace.repo.join(".roko").join(".env");
    std::fs::create_dir_all(env_file.parent().expect(".roko")).expect("create .roko");
    std::fs::write(&env_file, format!("{CANARY_VAR}={CANARY_SECRET}\n")).expect("write .env");

    // Debug logging, so every log line an agent's output can reach is written.
    let output = workspace.run_plan(PLAN_RUN_PLAN, &[("ROKO_LOG", "roko=debug")]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let context = || {
        format!(
            "exit: {}\nstdout (masked):\n{}\nstderr (masked, last 4000 bytes):\n{}",
            output.status,
            mask_secret(&stdout, CANARY_SECRET),
            mask_secret(&stderr[stderr.len().saturating_sub(4000)..], CANARY_SECRET)
        )
    };

    // Not vacuous: the agent ran and emitted the canary, and so did the check.
    let stream = std::fs::read_to_string(workspace.fixtures.join("stream.jsonl"))
        .unwrap_or_else(|_| panic!("the fake agent never ran\n{}", context()));
    assert!(
        stream.contains(CANARY_SECRET),
        "the fake agent must emit the canary"
    );
    assert!(
        workspace.fixtures.join("verify.ran").exists(),
        "T2's check never ran\n{}",
        context()
    );

    assert!(
        !stdout.contains(CANARY_SECRET),
        "the canary reached stdout\n{}",
        context()
    );
    assert!(
        !stderr.contains(CANARY_SECRET),
        "the canary reached stderr\n{}",
        context()
    );

    let mut leaks = files_containing(&workspace.repo, CANARY_SECRET, &[env_file]);
    leaks.extend(files_containing(&workspace.home, CANARY_SECRET, &[]));
    assert!(
        leaks.is_empty(),
        "the canary reached {} file(s):\n{}",
        leaks.len(),
        leaks
            .iter()
            .map(|path| describe_leak(path, CANARY_SECRET))
            .collect::<Vec<_>>()
            .join("\n")
    );

    // Redaction must leave JSON intact: every JSON or JSONL file holding the
    // redaction tag still parses.
    for path in files_containing(&workspace.repo, CANARY_REDACTED_TAG, &[]) {
        let text = std::fs::read_to_string(&path).expect("read redacted file");
        match path.extension().and_then(|extension| extension.to_str()) {
            Some("json") => {
                serde_json::from_str::<serde_json::Value>(&text)
                    .unwrap_or_else(|error| panic!("{} no longer parses: {error}", path.display()));
            }
            Some("jsonl") => {
                for line in text.lines().filter(|line| !line.trim().is_empty()) {
                    serde_json::from_str::<serde_json::Value>(line).unwrap_or_else(|error| {
                        panic!("a line of {} no longer parses: {error}", path.display())
                    });
                }
            }
            _ => {}
        }
    }
}
