#![cfg(unix)]

//! gap-28ceb9: an ACP prompt fails over like a Graph task. When its planned
//! model's provider is out of usage, the prompt runs on the same slug on
//! another provider of its family in the same turn, and the refusing
//! provider is quarantined, so the next prompt goes straight to the other
//! provider. An explicit model selection pins the prompt.
//!
//! The providers are fake Claude CLIs that log each call.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use roko_acp::bridge_events::handle_session_prompt;
use roko_acp::session::AcpSession;
use roko_acp::transport::StdioTransport;
use roko_acp::types::{ContentBlock, SessionNewParams, SessionPromptParams};
use roko_core::ProviderKind;
use roko_core::config::schema::{ModelProfile, ProviderConfig, RokoConfig};
use tokio::io::{AsyncBufReadExt, BufReader, DuplexStream, duplex};
use tokio::time::timeout;

/// What the backup provider answers.
const ANSWER: &str = "answered by the backup provider";

/// A fake Claude CLI `dir/name` that appends a line to `dir/name.calls`,
/// prints `output` and exits with `status`.
fn fake_claude(dir: &Path, name: &str, output: &str, status: i32) -> PathBuf {
    let script = dir.join(name);
    let calls = dir.join(format!("{name}.calls"));
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\ncat >/dev/null\necho call >> '{}'\ncat <<'JSON'\n{output}\nJSON\n\
             exit {status}\n",
            calls.display()
        ),
    )
    .expect("write fake provider");
    let mut permissions = std::fs::metadata(&script)
        .expect("fake provider metadata")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&script, permissions).expect("make fake provider executable");
    script
}

fn calls(dir: &Path, name: &str) -> usize {
    std::fs::read_to_string(dir.join(format!("{name}.calls")))
        .unwrap_or_default()
        .lines()
        .count()
}

/// What the Claude CLI prints when it answers `text`.
fn answer_output(text: &str) -> String {
    let assistant = serde_json::json!({
        "type": "assistant",
        "message": {"content": [{"type": "text", "text": text}]},
    });
    format!("{assistant}\n{}", result_line(text, false))
}

/// The Claude CLI's final result line.
fn result_line(text: &str, is_error: bool) -> String {
    serde_json::json!({
        "type": "result",
        "subtype": "success",
        "is_error": is_error,
        "result": text,
        "model": "claude-sonnet-4-6",
        "total_cost_usd": 0.0,
        "usage": {"input_tokens": 10, "output_tokens": 5},
    })
    .to_string()
}

/// A workspace whose planned model, `primary`, runs on `limited-cli`, a fake
/// Claude CLI out of usage. Beside it, `backup-cli` answers with [`ANSWER`],
/// and `distill-cli` serves the default model, which background
/// distillation calls. A key in the environment would synthesize an
/// `anthropic` provider of the same family, so `[routing]` disables it.
fn workspace() -> (tempfile::TempDir, RokoConfig) {
    let workspace = tempfile::tempdir().expect("workspace");
    let dir = workspace.path();
    std::fs::create_dir_all(dir.join(".roko").join("learn")).expect("create .roko/learn");
    let refusal = result_line(
        "You’ve hit your session limit · resets 4pm (Europe/Berlin)",
        true,
    );
    let scripts = [
        ("limited-cli", fake_claude(dir, "limited-claude", &refusal, 1)),
        (
            "backup-cli",
            fake_claude(dir, "backup-claude", &answer_output(ANSWER), 0),
        ),
        (
            "distill-cli",
            fake_claude(dir, "distill-claude", &answer_output("{}"), 0),
        ),
    ];
    let mut config = RokoConfig::default();
    config.providers.clear();
    config.models.clear();
    for (id, script) in &scripts {
        config.providers.insert(
            (*id).to_string(),
            ProviderConfig {
                kind: ProviderKind::ClaudeCli,
                command: Some(script.display().to_string()),
                ..ProviderConfig::default()
            },
        );
    }
    for (key, provider, slug) in [
        ("primary", "limited-cli", "claude-sonnet-4-6"),
        ("distiller", "distill-cli", "claude-haiku-4-5"),
    ] {
        config.models.insert(
            key.to_string(),
            ModelProfile {
                provider: provider.to_string(),
                slug: slug.to_string(),
                context_window: 200_000,
                ..ModelProfile::default()
            },
        );
    }
    config.agent.default_model = "distiller".to_string();
    config.routing.disabled_providers = vec!["anthropic".to_string()];
    (workspace, config)
}

/// A session planned on `primary`; `explicit` says the editor chose it.
fn session(config: &RokoConfig, explicit: bool) -> AcpSession {
    let mut session = AcpSession::new_with_config(
        SessionNewParams {
            session_name: Some("failover".to_string()),
            client_capabilities: None,
            model: None,
            provider: None,
            effort: None,
            mcp_servers: Vec::new(),
        },
        config,
    );
    session.config_state.agent_mode = "code".to_string();
    session.config_state.model = "primary".to_string();
    session.config_state.workflow = "none".to_string();
    session.config_state.model_selection_explicit = explicit;
    session
}

/// Everything the editor was sent for one prompt in `session`, and the
/// prompt's error, if it failed.
async fn prompt(dir: &Path, config: &RokoConfig, session: &mut AcpSession) -> String {
    let (client_to_server, server_reader) = duplex(64 * 1024);
    let (server_writer, client_from_server) = duplex(64 * 1024);
    let _client_input = client_to_server;
    let mut transport = StdioTransport::from_io(server_reader, server_writer);
    let session_id = session.session_id.clone();
    let outcome = handle_session_prompt(
        &mut transport,
        session,
        SessionPromptParams {
            session_id,
            prompt: vec![ContentBlock::Text {
                text: "Say hello.".to_string(),
            }],
            include_context: false,
        },
        dir,
        config,
    )
    .await;
    drop(transport);
    let mut sent = read_all(client_from_server).await;
    if let Err(error) = outcome {
        sent.push_str(&format!("\nprompt error: {error}"));
    }
    sent
}

async fn read_all(output: DuplexStream) -> String {
    let mut reader = BufReader::new(output);
    let mut sent = String::new();
    loop {
        let mut line = String::new();
        match timeout(Duration::from_millis(500), reader.read_line(&mut line)).await {
            Ok(Ok(0) | Err(_)) | Err(_) => break,
            Ok(Ok(_)) => sent.push_str(&line),
        }
    }
    sent
}

/// The planned model's provider refuses with its session limit: the editor
/// gets the backup provider's answer in the same turn and never the refusal,
/// and the next prompt skips the quarantined provider without calling it.
#[tokio::test]
async fn an_acp_prompt_fails_over_from_an_exhausted_provider() {
    let (workspace, config) = workspace();
    let dir = workspace.path();
    let mut session = session(&config, false);

    let sent = prompt(dir, &config, &mut session).await;

    assert!(sent.contains(ANSWER), "{sent}");
    assert!(!sent.contains("session limit"), "{sent}");
    assert_eq!(
        (calls(dir, "limited-claude"), calls(dir, "backup-claude")),
        (1, 1)
    );

    let sent = prompt(dir, &config, &mut session).await;

    assert!(sent.contains(ANSWER), "{sent}");
    assert_eq!(
        (calls(dir, "limited-claude"), calls(dir, "backup-claude")),
        (1, 2)
    );
}

/// A model the editor chose pins the prompt: its provider's refusal reaches
/// the editor, and no other model runs.
#[tokio::test]
async fn an_explicitly_chosen_model_does_not_fail_over() {
    let (workspace, config) = workspace();
    let dir = workspace.path();
    let mut session = session(&config, true);

    let sent = prompt(dir, &config, &mut session).await;

    assert!(sent.contains("session limit"), "{sent}");
    assert!(!sent.contains(ANSWER), "{sent}");
    assert_eq!(calls(dir, "backup-claude"), 0);
}
