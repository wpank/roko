use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::json;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, duplex, empty};
use tokio::sync::mpsc;

use roko_agent::dispatcher::{HandlerResolver, ToolDispatcher};
use roko_agent::rate_limit::ProviderRateLimiter;
use roko_core::agent::{AgentRole, ProviderKind, resolve_model};
use roko_core::config::DEFAULT_TTFT_TIMEOUT_MS;
use roko_core::config::schema::{ModelProfile, RokoConfig};
use roko_core::defaults::DEFAULT_CONNECT_TIMEOUT_MS;
use roko_core::defaults::DEFAULT_REQUEST_TIMEOUT_MS;
use roko_core::foundation::{MessageRole, ModelInputBlock, ModelStreamEvent, TokenUsage};
use roko_core::task::TaskCategory;
use roko_core::tool::{
    ToolCall, ToolContext, ToolError, ToolHandler, ToolPermission, ToolResult, VecToolRegistry,
};
use roko_learn::{
    cascade_router::CascadeRouter,
    episode_logger::{Episode, EpisodeLogger},
    model_router::RoutingContext,
    prompt_experiment::ExperimentStore,
    provider_health::ProviderHealthRegistry,
};
use roko_neuro::{KnowledgeKind, KnowledgeQueryHit, KnowledgeTier};

use crate::builtin_tools::acp_builtin_tools;
use crate::session::{AcpSession, CancelToken};
use crate::transport::StdioTransport;
use roko_learn::playbook::Playbook;

use crate::types::{
    ClientCapabilities, ContentBlock, JsonRpcNotification, McpServerStatus, PermissionAction,
    PermissionDecision, SESSION_BUDGET_EXCEEDED, SESSION_BUSY, SessionNewParams,
    SessionPromptParams, SessionPromptResult, SessionUpdate, StopReason, ToolCallKind,
    ToolCallStatus, UsageInfo, unsupported_prompt_content,
};

use super::context::*;
use super::cost::*;
use super::dispatch::*;
use super::experiments::*;
use super::helpers::*;
use super::permissions::*;
use super::provenance::*;
use super::slash_commands::*;
use super::tools::*;
use super::*;

fn test_session(model: &str, workflow: &str) -> AcpSession {
    let mut session = AcpSession::new(SessionNewParams {
        session_name: None,
        client_capabilities: None,
        model: None,
        provider: None,
        effort: None,
        mcp_servers: Vec::new(),
    });
    session.config_state.model = model.to_string();
    session.config_state.workflow = workflow.to_string();
    session
}

async fn reply_to_permission_request<C>(client: C, result: serde_json::Value)
where
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let mut reader = BufReader::new(client);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .await
        .expect("read permission request");
    let request: serde_json::Value = serde_json::from_str(&line).expect("parse request");
    let request_id = request["id"].clone();
    let mut client = reader.into_inner();
    let response = json!({
        "jsonrpc": "2.0",
        "id": request_id,
        "result": result,
    });
    let payload = serde_json::to_vec(&response).expect("serialize response");
    client
        .write_all(&payload)
        .await
        .expect("write response bytes");
    client.write_all(b"\n").await.expect("write newline");
    client.flush().await.expect("flush response");
}

#[test]
fn model_call_request_from_acp_messages_preserves_roles() {
    let request = model_call_request_from_acp_messages(
        "claude-sonnet-4-6",
        &[
            json!({"role": "system", "content": "system text"}),
            json!({"role": "user", "content": "hello"}),
            json!({"role": "assistant", "content": "hi"}),
        ],
        Vec::new(),
    )
    .expect("valid ACP messages");

    assert_eq!(request.model, "claude-sonnet-4-6");
    assert_eq!(request.caller.as_deref(), Some("acp"));
    assert_eq!(request.messages.len(), 3);
    assert_eq!(request.messages[0].role, MessageRole::System);
    assert_eq!(request.messages[0].content, "system text");
    assert_eq!(request.messages[1].role, MessageRole::User);
    assert_eq!(request.messages[1].content, "hello");
    assert_eq!(request.messages[2].role, MessageRole::Assistant);
    assert_eq!(request.messages[2].content, "hi");
}

#[test]
fn acp_prompt_conversions_preserve_text_image_diff_order() {
    let prompt = vec![
        ContentBlock::Text {
            text: "before".to_string(),
        },
        ContentBlock::Image {
            data: "aGVsbG8=".to_string(),
            mime_type: "image/png".to_string(),
        },
        ContentBlock::Diff {
            path: "src/lib.rs".to_string(),
            old_text: None,
            new_text: None,
            diff: Some("+added".to_string()),
        },
        ContentBlock::Text {
            text: "after".to_string(),
        },
    ];

    let anthropic = build_anthropic_content_parts(&prompt).expect("Anthropic parts");
    assert_eq!(anthropic[0]["text"], "before");
    assert_eq!(anthropic[1]["source"]["data"], "aGVsbG8=");
    assert_eq!(anthropic[2]["text"], "diff src/lib.rs:\n+added");
    assert_eq!(anthropic[3]["text"], "after");

    let openai = build_openai_content_parts(&prompt).expect("OpenAI parts");
    assert_eq!(openai[0]["text"], "before");
    assert_eq!(
        openai[1]["image_url"]["url"],
        "data:image/png;base64,aGVsbG8="
    );
    assert_eq!(openai[2]["text"], "diff src/lib.rs:\n+added");
    assert_eq!(openai[3]["text"], "after");
}

#[test]
fn acp_wire_round_trip_retains_multimodal_request_and_rejects_invalid_data() {
    let request = model_call_request_from_acp_messages(
        "gpt-4o",
        &[json!({
            "role": "user",
            "content": [
                {"type": "text", "text": "before"},
                {"type": "image_url", "image_url": {
                    "url": "data:image/webp;base64,aGVsbG8="
                }},
                {"type": "text", "text": "after"}
            ]
        })],
        Vec::new(),
    )
    .expect("valid image request");
    assert_eq!(request.input_messages.len(), 1);
    assert!(matches!(
        &request.input_messages[0].content[0],
        ModelInputBlock::Text { text } if text == "before"
    ));
    assert!(matches!(
        &request.input_messages[0].content[1],
        ModelInputBlock::Image { media_type, data }
            if media_type == "image/webp" && data == "aGVsbG8="
    ));
    assert!(matches!(
        &request.input_messages[0].content[2],
        ModelInputBlock::Text { text } if text == "after"
    ));

    let invalid = model_call_request_from_acp_messages(
        "gpt-4o",
        &[json!({
            "role": "user",
            "content": [{"type": "image_url", "image_url": {
                "url": "data:image/png;base64,not base64"
            }}]
        })],
        Vec::new(),
    );
    assert!(invalid.is_err());
}

#[test]
fn session_mcp_tool_names_are_provider_safe_and_unique() {
    let mut used = HashSet::new();

    let first = unique_tool_name(
        &format!(
            "{}_{}",
            sanitize_tool_segment("desktop.tools"),
            sanitize_tool_segment("read file")
        ),
        &mut used,
    );
    let second = unique_tool_name(
        &format!(
            "{}_{}",
            sanitize_tool_segment("desktop/tools"),
            sanitize_tool_segment("read:file")
        ),
        &mut used,
    );

    assert_eq!(first, "desktop_tools_read_file");
    assert_eq!(second, "desktop_tools_read_file_2");
    assert!(
        first
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
    );
    assert!(first.len() <= 64);
    assert!(second.len() <= 64);
}

#[tokio::test]
async fn anthropic_session_mcp_tools() {
    let server = r#"
        IFS= read -r initialize
        printf '%s\n' '{"jsonrpc":"2.0","id":1,"result":{"capabilities":{}}}'
        IFS= read -r list_tools
        printf '%s\n' '{"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"echo","description":"Echo input","inputSchema":{"type":"object"},"annotations":{"readOnlyHint":true}}]}}'
    "#;
    let session = AcpSession::new(SessionNewParams {
        session_name: None,
        client_capabilities: Some(ClientCapabilities {
            mcp_servers: Some(true),
            ..Default::default()
        }),
        model: Some("anthropic-test".to_string()),
        provider: Some("anthropic".to_string()),
        effort: None,
        mcp_servers: vec![crate::types::McpServerConfig {
            name: "fixture".to_string(),
            transport: crate::types::McpTransport::Stdio {
                command: "sh".to_string(),
                args: vec!["-c".to_string(), server.to_string()],
            },
            discovery_timeout_ms: Some(1_000),
        }],
    });
    let (event_sender, _event_receiver) = mpsc::channel(4);

    let (runtime, statuses) = setup_session_mcp_tools(
        &session.session_id,
        &session.mcp_servers,
        roko_agent::safety::capabilities::PluginTier::Sandboxed,
        event_sender,
    )
    .await;

    assert_eq!(statuses, vec![McpServerStatus::ready("fixture", 1)]);
    assert_eq!(runtime.tools.len(), 1);
    assert_eq!(runtime.tools[0].name, "fixture_echo");
    assert!(runtime.handlers.contains_key("fixture_echo"));
}

#[tokio::test]
async fn capabilities_reflect_session() {
    let tmp = tempfile::tempdir().expect("create tmpdir");
    let declined = ClientCapabilities {
        fs: Some(crate::types::FsCapabilities {
            read_text_file: true,
            write_text_file: false,
        }),
        terminal: Some(false),
        mcp_servers: Some(false),
    };
    let capabilities = derive_acp_tool_capabilities("code", &declined, false, &HashSet::new());
    assert!(capabilities.read);
    assert!(!capabilities.write);
    assert!(!capabilities.exec);

    let registry = Arc::new(VecToolRegistry::from_tools(acp_builtin_tools()));
    let resolver: Arc<dyn HandlerResolver> = Arc::new(AcpBuiltinHandlerResolver {
        handlers: HashMap::new(),
    });
    let dispatcher = ToolDispatcher::new(registry, resolver);
    let mut context = ToolContext::testing(tmp.path());
    context.capabilities = capabilities;

    for call in [
        ToolCall::new(
            "write-declined",
            "write_file",
            json!({"path": "blocked.txt", "content": "blocked"}),
        ),
        ToolCall::new("exec-declined", "bash", json!({"command": "true"})),
    ] {
        let result = dispatcher.dispatch(call, &context).await;
        assert!(matches!(
            result,
            ToolResult::Err(ToolError::PermissionDenied(message))
                if message.contains("role grants")
        ));
    }
    assert!(!tmp.path().join("blocked.txt").exists());

    let missing = derive_acp_tool_capabilities(
        "code",
        &ClientCapabilities::default(),
        false,
        &HashSet::new(),
    );
    assert_eq!(missing, ToolPermission::default());

    let elevated_client = ClientCapabilities {
        fs: Some(crate::types::FsCapabilities {
            read_text_file: true,
            write_text_file: true,
        }),
        terminal: Some(true),
        mcp_servers: Some(true),
    };
    let plan = derive_acp_tool_capabilities("plan", &elevated_client, true, &HashSet::new());
    assert!(plan.read);
    assert!(!plan.write);
    assert!(!plan.exec);
}

#[tokio::test]
async fn acp_conformance() {
    use roko_learn::prompt_experiment::{PromptExperiment, PromptVariant};

    let tmp = tempfile::tempdir().expect("tempdir");

    // Consent: the permission event must arrive before the handler can
    // mutate the worktree, and Reject must leave no side effect.
    let target = tmp.path().join("permission-rejected.txt");
    let (permission_tx, mut permission_rx) = mpsc::channel(4);
    let handler = AcpBuiltinToolHandler {
        tool_name: "write_file".into(),
        session_id: "conformance-session".into(),
        workdir: tmp.path().to_path_buf(),
        event_sender: permission_tx,
        role: "implementer".into(),
    };
    let context = ToolContext::testing(tmp.path());
    let write_task = tokio::spawn(async move {
        handler
            .execute(
                ToolCall::new(
                    "conformance-write",
                    "write_file",
                    json!({"path": "permission-rejected.txt", "content": "blocked"}),
                ),
                &context,
            )
            .await
    });
    let event = permission_rx.recv().await.expect("permission event");
    assert!(!target.exists(), "permission must precede the write");
    match event {
        CognitiveEvent::PermissionRequest { reply, .. } => {
            assert!(reply.reply(PermissionDecision::Reject));
        }
        other => panic!("expected permission request, got {other:?}"),
    }
    assert!(matches!(
        write_task.await.expect("join rejected write"),
        ToolResult::Err(ToolError::PermissionDenied(_))
    ));
    assert!(!target.exists(), "Reject must block the write");

    // Experiments: apply both content and model selection, then record the
    // outcome against the assigned experiment even when variant ids overlap.
    let experiment_path = tmp.path().join(".roko/learn/experiments.json");
    std::fs::create_dir_all(experiment_path.parent().expect("experiment parent"))
        .expect("create experiment parent");
    let variant = |content: &str, slug: Option<&str>| PromptVariant {
        id: "shared".into(),
        name: "shared".into(),
        section_name: "constraints".into(),
        content: content.into(),
        slug: slug.map(str::to_string),
        active: true,
    };
    let mut store = ExperimentStore::new();
    store.register(PromptExperiment::new(
        "exp-z",
        "other",
        vec![variant("wrong experiment", None)],
    ));
    store.register(PromptExperiment::new(
        "exp-a",
        "constraints",
        vec![variant("Use the ACP variant.", Some("vision-wire"))],
    ));
    store.save(&experiment_path).expect("save experiments");
    let assignment =
        assign_acp_experiment(&experiment_path, "code", "test-session").expect("assignment");
    let mut config = RokoConfig::default();
    config.models.insert(
        "vision-key".into(),
        ModelProfile {
            slug: "vision-wire".into(),
            supports_vision: true,
            ..ModelProfile::default()
        },
    );
    let (assignment, model_override) =
        applicable_acp_experiment(&config, "default", false, Some(assignment));
    let assignment = assignment.expect("applicable assignment");
    assert_eq!(model_override.as_deref(), Some("vision-key"));
    assert!(render_experiment_context(&assignment).contains("Use the ACP variant."));
    // P2-ACP-1: mark dispatched before settling so the assignment moves from
    // Prepared to Dispatched state; settle_attempt only counts Dispatched
    // assignments, matching the graph engine runner's three-phase protocol.
    let prompt_hash = roko_core::ContentHash::of(b"acp-conformance-test-prompt").to_hex();
    mark_acp_experiment_dispatched(&experiment_path, &assignment, &prompt_hash);
    record_acp_experiment_outcome(&experiment_path, &assignment, true)
        .expect("record scoped outcome");
    let recorded = ExperimentStore::load_or_new(&experiment_path);
    assert_eq!(
        recorded.get("exp-a").expect("exp-a").stats["shared"].trials,
        1
    );
    assert_eq!(
        recorded.get("exp-z").expect("exp-z").stats["shared"].trials,
        0
    );

    // MCP: an Anthropic-shaped session attachment discovers and exposes
    // the fixture tool without a network service.
    let server = r#"
        IFS= read -r initialize
        printf '%s\n' '{"jsonrpc":"2.0","id":1,"result":{"capabilities":{}}}'
        IFS= read -r list_tools
        printf '%s\n' '{"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"echo","inputSchema":{"type":"object"},"annotations":{"readOnlyHint":true}}]}}'
    "#;
    let mcp_session = AcpSession::new(SessionNewParams {
        session_name: None,
        client_capabilities: Some(ClientCapabilities {
            mcp_servers: Some(true),
            ..Default::default()
        }),
        model: Some("anthropic-test".into()),
        provider: Some("anthropic".into()),
        effort: None,
        mcp_servers: vec![crate::types::McpServerConfig {
            name: "fixture".into(),
            transport: crate::types::McpTransport::Stdio {
                command: "sh".into(),
                args: vec!["-c".into(), server.into()],
            },
            discovery_timeout_ms: Some(1_000),
        }],
    });
    let (mcp_tx, _mcp_rx) = mpsc::channel(4);
    let (runtime, statuses) = setup_session_mcp_tools(
        &mcp_session.session_id,
        &mcp_session.mcp_servers,
        roko_agent::safety::capabilities::PluginTier::Sandboxed,
        mcp_tx,
    )
    .await;
    assert_eq!(statuses, vec![McpServerStatus::ready("fixture", 1)]);
    assert_eq!(runtime.tools[0].name, "fixture_echo");
    assert!(runtime.handlers.contains_key("fixture_echo"));

    // Capabilities: advertised media support and the ToolContext ceiling
    // must agree with their enforcement helpers.
    let prompt_caps = crate::types::advertised_prompt_capabilities(true);
    let image = vec![ContentBlock::Image {
        data: "aGVsbG8=".into(),
        mime_type: "image/png".into(),
    }];
    let audio = vec![
        serde_json::from_value::<ContentBlock>(json!({
            "type": "audio", "data": "aGVsbG8=", "mimeType": "audio/wav"
        }))
        .expect("deserialize audio fail-closed"),
    ];
    assert!(prompt_caps.image && !prompt_caps.audio);
    assert!(unsupported_prompt_content(&image, &prompt_caps).is_none());
    assert!(build_anthropic_content_parts(&image).is_some());
    assert!(unsupported_prompt_content(&audio, &prompt_caps).is_some());

    let declined = ClientCapabilities {
        fs: Some(crate::types::FsCapabilities {
            read_text_file: true,
            write_text_file: false,
        }),
        terminal: Some(false),
        mcp_servers: Some(false),
    };
    let capabilities = derive_acp_tool_capabilities("code", &declined, false, &HashSet::new());
    let registry = Arc::new(VecToolRegistry::from_tools(acp_builtin_tools()));
    let resolver: Arc<dyn HandlerResolver> = Arc::new(AcpBuiltinHandlerResolver {
        handlers: HashMap::new(),
    });
    let dispatcher = ToolDispatcher::new(registry, resolver);
    let mut denied_context = ToolContext::testing(tmp.path());
    denied_context.capabilities = capabilities;
    let denied = dispatcher
        .dispatch(
            ToolCall::new(
                "capability-write",
                "write_file",
                json!({"path": "capability-blocked.txt", "content": "blocked"}),
            ),
            &denied_context,
        )
        .await;
    assert!(matches!(
        denied,
        ToolResult::Err(ToolError::PermissionDenied(_))
    ));
    assert!(!tmp.path().join("capability-blocked.txt").exists());
}

#[test]
fn experiment_assignment_selects_applies_and_records_acp_variant() {
    use roko_learn::prompt_experiment::{PromptExperiment, PromptVariant};

    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().join(".roko/learn/experiments.json");
    std::fs::create_dir_all(path.parent().expect("experiment parent"))
        .expect("create experiment parent");

    let variant = |id: &str, content: &str, slug: Option<&str>| PromptVariant {
        id: id.to_string(),
        name: id.to_string(),
        section_name: "constraints".to_string(),
        content: content.to_string(),
        slug: slug.map(str::to_string),
        active: true,
    };
    let mut store = ExperimentStore::new();
    store.register(PromptExperiment::new(
        "exp-b",
        "style",
        vec![variant("b", "later", None)],
    ));
    store.register(PromptExperiment::new(
        "exp-a",
        "constraints",
        vec![variant(
            "a",
            "Use the selected constraint.",
            Some("vision-model"),
        )],
    ));
    store.save(&path).expect("save experiments");

    let assignment =
        assign_acp_experiment(&path, "code", "test-session-2").expect("active assignment");
    assert_eq!(assignment.experiment_id, "exp-a");
    assert_eq!(assignment.variant_id, "a");
    assert!(render_experiment_context(&assignment).contains("Use the selected constraint."));

    let mut config = RokoConfig::default();
    config.models.insert(
        "configured-vision".to_string(),
        ModelProfile {
            slug: "vision-model".to_string(),
            ..ModelProfile::default()
        },
    );
    assert_eq!(
        experiment_model_key(&config, &assignment).as_deref(),
        Some("configured-vision")
    );

    // P2-ACP-1: mark dispatched first so the assignment moves to Dispatched
    // state; the canonical receipt protocol (settle_attempt) only counts
    // Dispatched assignments toward trials, matching the runner's path.
    let prompt_hash = roko_core::ContentHash::of(b"test-session-2-prompt").to_hex();
    mark_acp_experiment_dispatched(&path, &assignment, &prompt_hash);
    record_acp_experiment_outcome(&path, &assignment, true).expect("record outcome");
    let recorded = ExperimentStore::load_or_new(&path);
    let stats = &recorded.get("exp-a").expect("experiment").stats["a"];
    assert_eq!(stats.trials, 1);
    assert_eq!(stats.successes, 1);
}

#[test]
fn concurrent_acp_and_external_experiment_writers_preserve_all_outcomes() {
    use roko_learn::prompt_experiment::{PromptExperiment, PromptVariant};

    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().join(".roko/learn/experiments.json");
    std::fs::create_dir_all(path.parent().expect("experiment parent"))
        .expect("create experiment parent");
    let mut store = ExperimentStore::new();
    store.register(PromptExperiment::new(
        "concurrent-exp",
        "constraints",
        vec![PromptVariant {
            id: "variant".to_string(),
            name: "Variant".to_string(),
            section_name: "constraints".to_string(),
            content: "Concurrent content".to_string(),
            slug: None,
            active: true,
        }],
    ));
    store.save(&path).expect("seed experiments");
    let assignment = AcpExperimentAssignment {
        experiment_id: "concurrent-exp".to_string(),
        variant_id: "variant".to_string(),
        section_name: "constraints".to_string(),
        content: "Concurrent content".to_string(),
        model_slug: None,
        attempt_key: None,
        prepared_assignment_ids: Vec::new(),
    };
    let barrier = Arc::new(std::sync::Barrier::new(3));

    let acp_path = path.clone();
    let acp_assignment = assignment.clone();
    let acp_barrier = barrier.clone();
    let acp_writer = std::thread::spawn(move || {
        acp_barrier.wait();
        for _ in 0..20 {
            record_acp_experiment_outcome(&acp_path, &acp_assignment, true)
                .expect("record ACP outcome");
        }
    });
    let external_path = path.clone();
    let external_barrier = barrier.clone();
    let external_writer = std::thread::spawn(move || {
        external_barrier.wait();
        for _ in 0..20 {
            ExperimentStore::transaction(&external_path, |store| {
                if !store.record_outcome_for_experiment("concurrent-exp", "variant", true) {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "missing concurrent experiment",
                    ));
                }
                Ok(())
            })
            .expect("record external outcome");
        }
    });
    barrier.wait();
    acp_writer.join().expect("ACP writer joins");
    external_writer.join().expect("external writer joins");

    let committed = ExperimentStore::load_or_new(&path);
    let stats = &committed.get("concurrent-exp").expect("experiment").stats["variant"];
    assert_eq!(stats.trials, 40);
    assert_eq!(stats.successes, 40);
}

// P2-ACP-1: Verify that the canonical receipt protocol does not double-count
// trials.  The graph engine runner calls settle_attempt as the sole stats writer;
// ACP must do the same when attempt_key is Some so both paths record exactly one
// trial per observed outcome.
#[test]
fn record_acp_experiment_outcome_does_not_double_count_trials_with_receipt_protocol() {
    use roko_learn::prompt_experiment::{PromptExperiment, PromptVariant};

    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().join(".roko/learn/experiments.json");
    std::fs::create_dir_all(path.parent().expect("experiment parent"))
        .expect("create experiment parent");

    let mut store = ExperimentStore::new();
    store.register(PromptExperiment::new(
        "no-dup-exp",
        "constraints",
        vec![PromptVariant {
            id: "v1".to_string(),
            name: "V1".to_string(),
            section_name: "constraints".to_string(),
            content: "No double count.".to_string(),
            slug: None,
            active: true,
        }],
    ));
    store.save(&path).expect("save experiments");

    // Full three-phase lifecycle: prepare → dispatch → settle.
    let assignment = assign_acp_experiment(&path, "code", "no-dup-session").expect("assignment");
    assert!(assignment.attempt_key.is_some(), "attempt_key must be set");

    let prompt_hash = roko_core::ContentHash::of(b"no-dup-prompt").to_hex();
    mark_acp_experiment_dispatched(&path, &assignment, &prompt_hash);

    record_acp_experiment_outcome(&path, &assignment, true).expect("record outcome");

    let reloaded = ExperimentStore::load_or_new(&path);
    let stats = &reloaded.get("no-dup-exp").expect("experiment").stats["v1"];
    assert_eq!(
        stats.trials, 1,
        "exactly one trial must be recorded; got {} (double-counting would yield 2)",
        stats.trials
    );
    assert_eq!(stats.successes, 1);
}

// P2-ACP-1: When the prepare phase failed (attempt_key is None), the legacy
// direct-mutation path must still record the outcome so results are never lost.
#[test]
fn record_acp_experiment_outcome_legacy_fallback_when_no_attempt_key() {
    use roko_learn::prompt_experiment::{PromptExperiment, PromptVariant};

    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().join(".roko/learn/experiments.json");
    std::fs::create_dir_all(path.parent().expect("experiment parent"))
        .expect("create experiment parent");

    let mut store = ExperimentStore::new();
    store.register(PromptExperiment::new(
        "fallback-exp",
        "constraints",
        vec![PromptVariant {
            id: "fv1".to_string(),
            name: "FV1".to_string(),
            section_name: "constraints".to_string(),
            content: "Fallback content.".to_string(),
            slug: None,
            active: true,
        }],
    ));
    store.save(&path).expect("save experiments");

    // Construct an assignment with no attempt_key to simulate a failed prepare.
    let assignment = AcpExperimentAssignment {
        experiment_id: "fallback-exp".to_string(),
        variant_id: "fv1".to_string(),
        section_name: "constraints".to_string(),
        content: "Fallback content.".to_string(),
        model_slug: None,
        attempt_key: None,
        prepared_assignment_ids: Vec::new(),
    };

    record_acp_experiment_outcome(&path, &assignment, true).expect("record outcome");

    let reloaded = ExperimentStore::load_or_new(&path);
    let stats = &reloaded.get("fallback-exp").expect("experiment").stats["fv1"];
    assert_eq!(
        stats.trials, 1,
        "legacy fallback must still record exactly one trial"
    );
    assert_eq!(stats.successes, 1);
}

#[test]
fn replace_experiment_section_replaces_named_canonical_section() {
    // P1-ACP-2: replace_experiment_section must replace the named section in
    // the system prompt rather than appending a new heading.
    let prompt =
        "Preamble.\n\n## Project Conventions\n\nOriginal conventions.\n\n## Other\n\nOther text.";
    let assignment = AcpExperimentAssignment {
        experiment_id: "exp".to_string(),
        variant_id: "v1".to_string(),
        section_name: "conventions".to_string(),
        content: "Experiment conventions content.".to_string(),
        model_slug: None,
        attempt_key: None,
        prepared_assignment_ids: Vec::new(),
    };
    let result = replace_experiment_section(prompt, &assignment);
    assert!(
        result.contains("Experiment conventions content."),
        "result should contain experiment content: {result}"
    );
    assert!(
        !result.contains("Original conventions."),
        "result should not contain original section: {result}"
    );
    assert!(
        result.contains("Other text."),
        "result should retain other sections: {result}"
    );
}

#[test]
fn replace_experiment_section_falls_back_to_append_when_heading_missing() {
    let prompt = "A prompt without a conventions section.";
    let assignment = AcpExperimentAssignment {
        experiment_id: "exp".to_string(),
        variant_id: "v1".to_string(),
        section_name: "conventions".to_string(),
        content: "Experiment content.".to_string(),
        model_slug: None,
        attempt_key: None,
        prepared_assignment_ids: Vec::new(),
    };
    let result = replace_experiment_section(prompt, &assignment);
    assert!(
        result.contains("Experiment content."),
        "fallback result should contain experiment content: {result}"
    );
    // Original prompt must still be present in the fallback path.
    assert!(
        result.contains("A prompt without a conventions section."),
        "fallback result should retain original prompt: {result}"
    );
}

#[test]
fn mark_acp_experiment_dispatched_records_dispatched_state_for_prepared_attempt() {
    use roko_learn::prompt_experiment::{ExperimentStatus, PromptExperiment, PromptVariant};

    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().join(".roko/learn/experiments.json");
    std::fs::create_dir_all(path.parent().expect("experiment parent"))
        .expect("create experiment parent");

    let mut store = ExperimentStore::new();
    store.register(PromptExperiment::new(
        "dispatch-exp",
        "conventions",
        vec![PromptVariant {
            id: "v1".to_string(),
            name: "V1".to_string(),
            section_name: "conventions".to_string(),
            content: "Content for dispatch test.".to_string(),
            slug: None,
            active: true,
        }],
    ));
    store.save(&path).expect("save experiments");

    // Assign and prepare a receipt.
    let assignment =
        assign_acp_experiment(&path, "code", "test-dispatch-session").expect("assignment");
    assert_eq!(assignment.experiment_id, "dispatch-exp");
    assert!(
        assignment.attempt_key.is_some(),
        "attempt key must be set after prepare"
    );
    assert!(
        !assignment.prepared_assignment_ids.is_empty(),
        "prepared_assignment_ids must be non-empty"
    );

    // Simulate what the pipeline path does: mark as dispatched.
    let prompt_hash = roko_core::ContentHash::of(b"simulated-pipeline-prompt").to_hex();
    mark_acp_experiment_dispatched(&path, &assignment, &prompt_hash);

    // Verify the assignment moved to Dispatched state in the store.
    let reloaded = ExperimentStore::load_or_new(&path);
    let exp = reloaded.get("dispatch-exp").expect("experiment");
    assert_eq!(exp.status, ExperimentStatus::Running);
    // The attempt bucket should exist; we verify via a successful settle.
    let attempt_key = assignment.attempt_key.as_ref().expect("attempt key");
    let settle_result = ExperimentStore::settle_attempt(
        &path,
        attempt_key,
        roko_learn::prompt_experiment::AssignmentSettlement::Observed { success: true },
    );
    assert!(
        settle_result.is_ok(),
        "settle_attempt should succeed after mark_attempt_dispatched: {settle_result:?}"
    );
}

#[test]
fn anthropic_model_call_config_routes_legacy_claude_to_anthropic_provider() {
    let mut roko_config = RokoConfig::default();
    roko_config.providers.insert(
        "anthropic".to_string(),
        roko_core::config::schema::ProviderConfig {
            kind: ProviderKind::AnthropicApi,
            base_url: Some("https://api.anthropic.com".to_string()),
            api_key_env: Some("TEST_ANTHROPIC_API_KEY".to_string()),
            command: None,
            args: None,
            timeout_ms: Some(DEFAULT_REQUEST_TIMEOUT_MS),
            ttft_timeout_ms: Some(DEFAULT_TTFT_TIMEOUT_MS),
            connect_timeout_ms: Some(DEFAULT_CONNECT_TIMEOUT_MS),
            extra_headers: None,
            max_concurrent: None,
            limits: None,
            require_confirmation: false,
        },
    );

    let config =
        anthropic_model_call_config(&roko_config, "claude-sonnet-4-6", "claude-sonnet-4-6")
            .expect("anthropic provider config");
    let resolved = resolve_model(&config, "claude-sonnet-4-6");

    assert_eq!(resolved.provider_kind, ProviderKind::AnthropicApi);
    assert_eq!(
        resolved
            .profile
            .as_ref()
            .map(|profile| profile.provider.as_str()),
        Some("anthropic")
    );
}

#[test]
fn anthropic_model_call_config_requires_explicit_provider_when_env_values_exist() {
    let mut roko_config = RokoConfig::default();
    roko_config.agent.env = Some(vec![
        ("ANTHROPIC_API_KEY".to_string(), "sk-test".to_string()),
        (
            "ANTHROPIC_BASE_URL".to_string(),
            "https://api.anthropic.com".to_string(),
        ),
    ]);

    assert!(
        !roko_config
            .effective_providers()
            .values()
            .any(|provider| provider.kind == ProviderKind::AnthropicApi)
    );
    assert!(
        anthropic_model_call_config(&roko_config, "claude-sonnet-4-6", "claude-sonnet-4-6")
            .is_none()
    );
}

#[tokio::test]
async fn model_stream_failed_event_emits_failure_event() {
    let (sender, mut receiver) = mpsc::channel(4);
    let mut state = ModelStreamForwardState::default();

    let error = forward_model_stream_event(
        "sess_model_stream",
        &sender,
        &mut state,
        ModelStreamEvent::Failed {
            error: "provider failed".to_string(),
        },
    )
    .await
    .expect_err("failed stream event should error");

    assert!(error.to_string().contains("provider failed"));
    match receiver.recv().await.expect("failure event") {
        CognitiveEvent::Failure { message } => {
            assert_eq!(message, "Error: model stream failed: provider failed");
        }
        other => panic!("expected failure event, got {other:?}"),
    }
}

#[tokio::test]
async fn model_stream_usage_and_completion_emit_typed_complete() {
    let (sender, mut receiver) = mpsc::channel(4);
    let mut state = ModelStreamForwardState::default();

    let forwarded = forward_model_stream_event(
        "sess_model_stream",
        &sender,
        &mut state,
        ModelStreamEvent::Usage {
            usage: TokenUsage {
                input_tokens: 11,
                output_tokens: 7,
                total_tokens: 18,
                cost_usd: 0.0,
            },
        },
    )
    .await
    .expect("usage event");
    assert_eq!(forwarded, ModelStreamForward::Continue);

    let forwarded = forward_model_stream_event(
        "sess_model_stream",
        &sender,
        &mut state,
        ModelStreamEvent::Completed {
            stop_reason: Some("max_tokens".to_string()),
        },
    )
    .await
    .expect("completed event");
    assert_eq!(forwarded, ModelStreamForward::Completed);

    match receiver.recv().await.expect("completion event") {
        CognitiveEvent::Complete { stop_reason, usage } => {
            assert_eq!(stop_reason, StopReason::MaxTokens);
            assert_eq!(
                usage,
                Some(UsageInfo {
                    total_tokens: 18,
                    input_tokens: 11,
                    output_tokens: 7,
                    thought_tokens: None,
                    cached_read_tokens: None,
                    cached_write_tokens: None,
                })
            );
        }
        other => panic!("expected completion event, got {other:?}"),
    }
}

#[tokio::test]
async fn model_stream_cancelled_event_emits_cancelled_complete() {
    let (sender, mut receiver) = mpsc::channel(4);
    let mut state = ModelStreamForwardState::default();

    let forwarded = forward_model_stream_event(
        "sess_model_stream",
        &sender,
        &mut state,
        ModelStreamEvent::Cancelled,
    )
    .await
    .expect("cancelled event");
    assert_eq!(forwarded, ModelStreamForward::Completed);

    match receiver.recv().await.expect("completion event") {
        CognitiveEvent::Complete { stop_reason, usage } => {
            assert_eq!(stop_reason, StopReason::Cancelled);
            assert_eq!(usage, None);
        }
        other => panic!("expected completion event, got {other:?}"),
    }
}

#[tokio::test]
async fn send_session_update_emits_wrapped_payload() {
    let (client, server) = duplex(4096);
    let mut transport = StdioTransport::from_io(empty(), server);
    let mut reader = BufReader::new(client);

    send_session_update(
        &mut transport,
        "sess_wrapped",
        SessionUpdate::AgentMessageChunk {
            content: text_block("hello".to_owned()),
            _meta: None,
        },
    )
    .await
    .expect("send session update");

    let mut line = String::new();
    reader
        .read_line(&mut line)
        .await
        .expect("read notification line");
    let notification: JsonRpcNotification =
        serde_json::from_str(&line).expect("deserialize notification");

    assert_eq!(notification.method, "session/update");
    let params = notification.params.expect("params must be present");
    assert_eq!(params["sessionId"], json!("sess_wrapped"));
    // ACP spec requires updates nested under "update" key.
    let update = &params["update"];
    assert_eq!(update["sessionUpdate"], json!("agent_message_chunk"));
    assert_eq!(
        update["content"],
        json!({ "type": "text", "text": "hello" })
    );
}

/// The ACP v1 schema's definitions reachable from `SessionNotification`.
const ACP_SESSION_NOTIFICATION_SCHEMA: &str =
    include_str!("../../tests/fixtures/acp-v1-session-notification.schema.json");

/// Lists the ways `value` breaks `schema`, resolving `$ref`s against `defs`. Covers the
/// JSON Schema keywords that the ACP session-update definitions use, except `format`
/// and `minimum`.
fn acp_schema_errors(
    value: &serde_json::Value,
    schema: &serde_json::Value,
    defs: &serde_json::Value,
    path: &str,
) -> Vec<String> {
    let mut errors = Vec::new();
    if let Some(reference) = schema.get("$ref").and_then(serde_json::Value::as_str) {
        let name = reference.trim_start_matches("#/$defs/");
        let target = defs
            .get(name)
            .unwrap_or_else(|| panic!("schema has no definition for {reference}"));
        errors.extend(acp_schema_errors(value, target, defs, path));
    }
    if let Some(expected) = schema.get("const")
        && value != expected
    {
        errors.push(format!("{path}: expected {expected}, got {value}"));
    }
    if let Some(types) = schema.get("type") {
        let names: Vec<&str> = match types {
            serde_json::Value::Array(list) => {
                list.iter().filter_map(serde_json::Value::as_str).collect()
            }
            single => single.as_str().into_iter().collect(),
        };
        if !names.iter().any(|name| json_type_matches(value, name)) {
            errors.push(format!("{path}: {value} is not of type {types}"));
        }
    }
    if let Some(object) = value.as_object() {
        let required = schema["required"].as_array().into_iter().flatten();
        for name in required.filter_map(serde_json::Value::as_str) {
            if !object.contains_key(name) {
                errors.push(format!("{path}: missing required `{name}`"));
            }
        }
        for (name, property) in schema["properties"].as_object().into_iter().flatten() {
            if let Some(field) = object.get(name) {
                let field_path = format!("{path}.{name}");
                errors.extend(acp_schema_errors(field, property, defs, &field_path));
            }
        }
    }
    if let (Some(items), Some(array)) = (schema.get("items"), value.as_array()) {
        for (index, item) in array.iter().enumerate() {
            let item_path = format!("{path}[{index}]");
            errors.extend(acp_schema_errors(item, items, defs, &item_path));
        }
    }
    for branch in schema["allOf"].as_array().into_iter().flatten() {
        errors.extend(acp_schema_errors(value, branch, defs, path));
    }
    for (keyword, exactly_one) in [("anyOf", false), ("oneOf", true)] {
        if let Some(branches) = schema[keyword].as_array() {
            let matching = branches
                .iter()
                .filter(|branch| acp_schema_errors(value, branch, defs, path).is_empty())
                .count();
            if matching == 0 || (exactly_one && matching > 1) {
                errors.push(format!(
                    "{path}: {matching} {keyword} branches match {value}"
                ));
            }
        }
    }
    errors
}

fn json_type_matches(value: &serde_json::Value, json_type: &str) -> bool {
    match json_type {
        "object" => value.is_object(),
        "array" => value.is_array(),
        "string" => value.is_string(),
        "boolean" => value.is_boolean(),
        "integer" => value.is_i64() || value.is_u64(),
        "number" => value.is_number(),
        "null" => value.is_null(),
        other => panic!("unsupported JSON Schema type {other}"),
    }
}

#[test]
fn session_update_spec_conformance() {
    use crate::types::{
        CostInfo, PlanEntry, PlanStatus, Priority, SessionBudgetStatus, ToolCallLocation,
    };

    let schema: serde_json::Value =
        serde_json::from_str(ACP_SESSION_NOTIFICATION_SCHEMA).expect("parse ACP schema subset");
    let defs = &schema["$defs"];
    // Checks one update as `session/update` params and returns the update's JSON.
    let conforming = |update: SessionUpdate| {
        let params = json!({ "sessionId": "sess-1", "update": update });
        let errors = acp_schema_errors(&params, &defs["SessionNotification"], defs, "params");
        assert!(
            errors.is_empty(),
            "{params} is not a spec session/update: {errors:?}"
        );
        params["update"].clone()
    };
    let mapped = |event: CognitiveEvent| map_event_to_update(event).expect("maps to an update");

    for kind in [
        ToolCallKind::Read,
        ToolCallKind::Edit,
        ToolCallKind::Delete,
        ToolCallKind::Move,
        ToolCallKind::Search,
        ToolCallKind::Terminal,
        ToolCallKind::Think,
        ToolCallKind::Fetch,
        ToolCallKind::Other,
    ] {
        let value = serde_json::to_value(&kind).expect("serialize tool kind");
        let errors = acp_schema_errors(&value, &defs["ToolKind"], defs, "kind");
        assert!(
            errors.is_empty(),
            "{kind:?} is not a spec ToolKind: {errors:?}"
        );
    }

    conforming(mapped(CognitiveEvent::TokenChunk("hello".to_owned())));
    conforming(mapped(CognitiveEvent::ThinkingChunk("thinking".to_owned())));
    conforming(dispatch_failure_update("provider failed".to_owned()));
    conforming(mapped(CognitiveEvent::PlanUpdate {
        entries: vec![PlanEntry {
            content: "Write the test".to_owned(),
            priority: Priority::High,
            status: PlanStatus::InProgress,
        }],
    }));

    let started = conforming(mapped(CognitiveEvent::ToolCallStart {
        tool_call_id: "tc-1".to_owned(),
        title: "Write result.txt".to_owned(),
        kind: ToolCallKind::Edit,
        locations: Some(vec![ToolCallLocation {
            path: "/repo/result.txt".to_owned(),
            line: Some(3),
        }]),
    }));
    assert_eq!(started["kind"], json!("edit"));
    assert_eq!(
        started["locations"],
        json!([{ "path": "/repo/result.txt", "line": 3 }])
    );

    // Tool output keeps its text and diffs, in the spec's wrapped shapes.
    let completed = conforming(mapped(CognitiveEvent::ToolCallComplete {
        tool_call_id: "tc-1".to_owned(),
        status: ToolCallStatus::Completed,
        content: vec![
            text_block("wrote result.txt".to_owned()),
            ContentBlock::Diff {
                path: "/repo/result.txt".to_owned(),
                old_text: Some("old\n".to_owned()),
                new_text: Some("new\n".to_owned()),
                diff: None,
            },
            ContentBlock::Diff {
                path: "/repo/lib.rs".to_owned(),
                old_text: None,
                new_text: None,
                diff: Some("@@ -1 +1 @@\n-old\n+new\n".to_owned()),
            },
        ],
    }));
    assert_eq!(
        completed["content"],
        json!([
            { "type": "content", "content": { "type": "text", "text": "wrote result.txt" } },
            { "type": "diff", "path": "/repo/result.txt", "oldText": "old\n", "newText": "new\n" },
            {
                "type": "content",
                "content": { "type": "text", "text": "```diff\n@@ -1 +1 @@\n-old\n+new\n```" }
            }
        ])
    );

    // Roko's extensions ride under `_meta` on a spec update.
    let mcp = conforming(mapped(CognitiveEvent::McpStatus {
        statuses: vec![McpServerStatus::ready("github", 3)],
    }));
    assert_eq!(mcp["_meta"]["roko"]["mcpStatus"][0]["toolCount"], json!(3));
    let budget = conforming(roko_meta_update(
        "budget",
        &SessionBudgetStatus {
            cost_budget_usd: Some(1.0),
            accumulated_cost_usd: Some(0.25),
            budget_remaining_usd: Some(0.75),
        },
    ));
    assert_eq!(
        budget["_meta"]["roko"]["budget"]["budgetRemainingUsd"],
        json!(0.75)
    );

    let titled = conforming(SessionUpdate::SessionInfoUpdate {
        title: Some("Fix the login bug".to_owned()),
        _meta: None,
    });
    assert_eq!(titled["title"], json!("Fix the login bug"));
    conforming(SessionUpdate::UsageUpdate {
        used: 1_200,
        size: 200_000,
        cost: Some(CostInfo {
            amount: 0.01,
            currency: "USD".to_owned(),
        }),
    });
    conforming(SessionUpdate::AvailableCommandsUpdate {
        available_commands: crate::session::build_slash_commands(false),
    });

    // The check is not vacuous: the update roko used to send for MCP status fails it.
    let old = json!({
        "sessionId": "sess-1",
        "update": { "sessionUpdate": "mcp_status_update", "statuses": [] }
    });
    assert!(!acp_schema_errors(&old, &defs["SessionNotification"], defs, "params").is_empty());
}

#[tokio::test]
async fn stream_events_to_editor_emits_notifications_and_returns_completion() {
    let (client, server) = duplex(4096);
    let mut transport = StdioTransport::from_io(empty(), server);
    let mut reader = BufReader::new(client);
    let cancel_token = CancelToken::new();
    let mut session = test_session("test-model", "none");
    let (sender, receiver) = mpsc::channel(8);

    sender
        .send(CognitiveEvent::TokenChunk("hello".to_owned()))
        .await
        .expect("send token chunk");
    sender
        .send(CognitiveEvent::Complete {
            stop_reason: StopReason::EndTurn,
            usage: Some(UsageInfo {
                total_tokens: 12,
                input_tokens: 5,
                output_tokens: 7,
                thought_tokens: None,
                cached_read_tokens: None,
                cached_write_tokens: None,
            }),
        })
        .await
        .expect("send completion");
    drop(sender);

    let result = stream_events_to_editor(
        &mut transport,
        "sess_test",
        &mut session,
        Path::new("."),
        receiver,
        &cancel_token,
    )
    .await;
    let result = result.expect("stream should succeed");

    assert_eq!(result.prompt_result.stop_reason, StopReason::EndTurn);
    assert_eq!(
        result.usage.as_ref().map(|usage| usage.total_tokens),
        Some(12)
    );
    assert_eq!(
        result.usage.as_ref().map(|usage| usage.input_tokens),
        Some(5)
    );
    assert_eq!(
        result.usage.as_ref().map(|usage| usage.output_tokens),
        Some(7)
    );

    let mut line = String::new();
    reader
        .read_line(&mut line)
        .await
        .expect("read notification line");
    let notification: JsonRpcNotification =
        serde_json::from_str(&line).expect("deserialize notification");
    assert_eq!(notification.method, "session/update");
    let params = notification.params.expect("params must be present");
    assert_eq!(params["sessionId"], json!("sess_test"));
    let update = &params["update"];
    assert_eq!(update["sessionUpdate"], json!("agent_message_chunk"));
    assert_eq!(
        update["content"],
        json!({ "type": "text", "text": "hello" })
    );
}

#[tokio::test]
async fn stream_events_to_editor_emits_failure_status_without_normal_completion() {
    let (client, server) = duplex(4096);
    let mut transport = StdioTransport::from_io(empty(), server);
    let mut reader = BufReader::new(client);
    let cancel_token = CancelToken::new();
    let mut session = test_session("test-model", "none");
    let (sender, receiver) = mpsc::channel(8);

    sender
        .send(CognitiveEvent::Failure {
            message: "Error: provider returned 401".to_owned(),
        })
        .await
        .expect("send failure");
    sender
        .send(CognitiveEvent::Complete {
            stop_reason: StopReason::EndTurn,
            usage: None,
        })
        .await
        .expect("send normal completion after failure");
    drop(sender);

    let result = stream_events_to_editor(
        &mut transport,
        "sess_failure",
        &mut session,
        Path::new("."),
        receiver,
        &cancel_token,
    )
    .await;
    let result = result.expect("failure should still return a prompt result");

    assert_eq!(result.prompt_result.stop_reason, StopReason::EndTurn);
    assert_eq!(result.usage, None);

    let mut line = String::new();
    reader
        .read_line(&mut line)
        .await
        .expect("read notification line");
    let notification: JsonRpcNotification =
        serde_json::from_str(&line).expect("deserialize notification");
    assert_eq!(notification.method, "session/update");
    let params = notification.params.expect("params must be present");
    assert_eq!(params["sessionId"], json!("sess_failure"));
    let update = &params["update"];
    assert_eq!(update["sessionUpdate"], json!("agent_message_chunk"));
    assert_eq!(
        update["content"],
        json!({ "type": "text", "text": "Error: provider returned 401" })
    );
}

#[tokio::test]
async fn stream_events_to_editor_returns_cancelled_when_token_is_cancelled() {
    let (_client, server) = duplex(1024);
    let mut transport = StdioTransport::from_io(empty(), server);
    let cancel_token = CancelToken::new();
    let mut session = test_session("test-model", "none");
    let (_sender, receiver) = mpsc::channel(1);

    cancel_token.cancel();

    let result = stream_events_to_editor(
        &mut transport,
        "sess_cancel",
        &mut session,
        Path::new("."),
        receiver,
        &cancel_token,
    )
    .await
    .expect("cancelled prompt should still return a result");

    assert_eq!(result.prompt_result.stop_reason, StopReason::Cancelled);
}

#[tokio::test]
async fn handle_session_prompt_rejects_busy_sessions() {
    let (_client, server) = duplex(1024);
    let mut transport = StdioTransport::from_io(empty(), server);
    let mut session = AcpSession::new(SessionNewParams {
        session_name: None,
        client_capabilities: None,
        model: None,
        provider: None,
        effort: None,
        mcp_servers: Vec::new(),
    });
    let session_id = session.session_id.clone();
    session.begin_prompt();

    let roko_config = RokoConfig::default();
    let error = handle_session_prompt(
        &mut transport,
        &mut session,
        SessionPromptParams {
            session_id: session_id.clone(),
            prompt: vec![ContentBlock::Text {
                text: "busy".to_owned(),
            }],
            include_context: false,
        },
        Path::new("."),
        &roko_config,
    )
    .await
    .expect_err("busy session should be rejected");

    assert_eq!(
        error.rpc_error(),
        Some((
            SESSION_BUSY,
            format!("session '{session_id}' already has an active prompt")
        ))
    );
}

#[tokio::test]
async fn cost_budget_exhaustion_rejects_before_provider_dispatch() {
    let tmp = tempfile::tempdir().expect("create tmpdir");
    let mut transport = StdioTransport::from_io(empty(), tokio::io::sink());
    let mut session = test_session("model-a", "none");
    session.cost_budget_usd = Some(1.0);
    session.accumulated_cost_usd = 1.0;
    let session_id = session.session_id.clone();

    let error = handle_session_prompt(
        &mut transport,
        &mut session,
        SessionPromptParams {
            session_id,
            prompt: vec![ContentBlock::Text {
                text: "this must never dispatch".to_owned(),
            }],
            include_context: false,
        },
        tmp.path(),
        &RokoConfig::default(),
    )
    .await
    .expect_err("exhausted budget must reject the turn");

    let (code, message) = error.rpc_error().expect("structured budget error");
    assert_eq!(code, SESSION_BUDGET_EXCEEDED);
    assert!(message.contains("budget exceeded"));
    assert!(session.conversation_history.is_empty());
    assert!(!session.is_busy());
    assert!(!tmp.path().join(".roko/learn/efficiency.jsonl").exists());
}

#[test]
fn cost_budget_accumulates_exact_efficiency_event_cost() {
    let mut session = test_session("model-a", "none");
    session.cost_budget_usd = Some(1.0);
    let resolved = resolve_model(&RokoConfig::default(), "model-a");
    let event = acp_efficiency_event(
        &session.session_id,
        &resolved,
        Instant::now(),
        None,
        true,
        Some(0.375),
    );

    session.record_efficiency_cost(event.cost_usd);

    assert_eq!(event.cost_usd, 0.375);
    assert_eq!(session.accumulated_cost_usd, 0.375);
    assert_eq!(session.budget_status().budget_remaining_usd, Some(0.625));
}

#[tokio::test]
async fn request_permission_returns_allow_for_pregranted_action() {
    let mut transport = StdioTransport::from_io(empty(), tokio::io::sink());
    let mut session = AcpSession::new(SessionNewParams {
        session_name: Some("perm-test".to_string()),
        client_capabilities: None,
        model: None,
        provider: None,
        effort: None,
        mcp_servers: Vec::new(),
    });
    let action = crate::types::PermissionAction::FileEdit;
    session.grant_always_allow(action.clone());

    let decision = request_permission(
        &mut transport,
        &mut session,
        Path::new("."),
        action,
        "Allow code agent to edit files?",
        "The code agent may read and modify files.",
    )
    .await;

    assert_eq!(decision, PermissionDecision::Allow);
}

#[tokio::test]
async fn request_permission_persists_always_allow_decision() {
    let tmp = tempfile::tempdir().expect("create tmpdir");
    let workdir = tmp.path().to_path_buf();
    let mut session = AcpSession::new(SessionNewParams {
        session_name: Some("perm-test".to_string()),
        client_capabilities: None,
        model: None,
        provider: None,
        effort: None,
        mcp_servers: Vec::new(),
    });
    let action = crate::types::PermissionAction::FileEdit;

    let (client, server) = duplex(4096);
    let (server_reader, server_writer) = tokio::io::split(server);
    let mut transport = StdioTransport::from_io(server_reader, server_writer);
    let ((), decision) = tokio::join!(
        reply_to_permission_request(
            client,
            json!({ "outcome": { "type": "selected", "optionId": "allow_always" } })
        ),
        request_permission(
            &mut transport,
            &mut session,
            &workdir,
            action.clone(),
            "Allow code agent to edit files?",
            "The code agent may read and modify files.",
        ),
    );

    assert_eq!(decision, PermissionDecision::AlwaysAllow);
    assert!(session.always_allowed.contains(&action));
    assert!(AcpSession::load_workspace_trust(&workdir).contains(&action));

    // The persisted session grant suppresses the next equivalent prompt.
    // A transport with no readable client is intentional: reaching it
    // would reject immediately and prove the pre-grant was not consulted.
    let mut disconnected = StdioTransport::from_io(empty(), tokio::io::sink());
    let repeated = request_permission(
        &mut disconnected,
        &mut session,
        &workdir,
        action,
        "Allow code agent to edit files?",
        "The code agent may read and modify files.",
    )
    .await;
    assert_eq!(repeated, PermissionDecision::Allow);
}

#[tokio::test]
async fn stream_events_to_editor_routes_permission_request_to_editor() {
    let tmp = tempfile::tempdir().expect("create tmpdir");
    let workdir = tmp.path().to_path_buf();
    let mut session = test_session("test-model", "none");
    let session_id = session.session_id.clone();
    let cancel_token = session.cancel_token.clone();
    let (client, server) = duplex(4096);
    let (server_reader, server_writer) = tokio::io::split(server);
    let mut transport = StdioTransport::from_io(server_reader, server_writer);
    let (event_sender, event_receiver) = mpsc::channel(4);
    let (decision_sender, decision_receiver) = tokio::sync::oneshot::channel();

    event_sender
        .send(CognitiveEvent::PermissionRequest {
            payload: PermissionRequestPayload {
                action: PermissionAction::FileEdit,
                title: "Write result.txt".to_owned(),
                detail: "Allow this ACP turn to write the requested file?".to_owned(),
            },
            reply: PermissionReplyChannel::new(decision_sender),
        })
        .await
        .expect("queue permission event");
    event_sender
        .send(CognitiveEvent::Complete {
            stop_reason: StopReason::EndTurn,
            usage: None,
        })
        .await
        .expect("queue completion event");
    drop(event_sender);

    let ((), stream_result) = tokio::join!(
        reply_to_permission_request(
            client,
            json!({ "outcome": { "type": "selected", "optionId": "allow_once" } })
        ),
        stream_events_to_editor(
            &mut transport,
            &session_id,
            &mut session,
            &workdir,
            event_receiver,
            &cancel_token,
        ),
    );

    assert_eq!(
        decision_receiver.await.expect("receive editor decision"),
        PermissionDecision::Allow
    );
    assert_eq!(
        stream_result
            .expect("permission stream should complete")
            .prompt_result
            .stop_reason,
        StopReason::EndTurn
    );
}

#[tokio::test]
async fn request_permission_defaults_to_reject_on_malformed_response() {
    let tmp = tempfile::tempdir().expect("create tmpdir");
    let workdir = tmp.path().to_path_buf();
    let mut session = AcpSession::new(SessionNewParams {
        session_name: Some("perm-test".to_string()),
        client_capabilities: None,
        model: None,
        provider: None,
        effort: None,
        mcp_servers: Vec::new(),
    });
    let action = crate::types::PermissionAction::FileEdit;

    let (client, server) = duplex(4096);
    let (server_reader, server_writer) = tokio::io::split(server);
    let mut transport = StdioTransport::from_io(server_reader, server_writer);
    let ((), decision) = tokio::join!(
        reply_to_permission_request(client, json!({ "outcome": { "type": "cancelled" } })),
        request_permission(
            &mut transport,
            &mut session,
            &workdir,
            action.clone(),
            "Allow code agent to edit files?",
            "The code agent may read and modify files.",
        ),
    );

    assert_eq!(decision, PermissionDecision::Reject);
    assert!(!session.always_allowed.contains(&action));
    assert!(AcpSession::load_workspace_trust(&workdir).is_empty());
}

#[tokio::test]
async fn request_permission_accepts_spec_shaped_responses() {
    let action = PermissionAction::FileEdit;
    let cases = [
        (
            json!({ "outcome": "selected", "optionId": "allow_once" }),
            PermissionDecision::Allow,
        ),
        (
            json!({ "outcome": "selected", "optionId": "allow_always" }),
            PermissionDecision::AlwaysAllow,
        ),
        (
            json!({ "outcome": "cancelled" }),
            PermissionDecision::Reject,
        ),
    ];
    for (outcome, expected) in cases {
        let tmp = tempfile::tempdir().expect("create tmpdir");
        let mut session = test_session("test-model", "none");
        let (client, server) = duplex(4096);
        let (server_reader, server_writer) = tokio::io::split(server);
        let mut transport = StdioTransport::from_io(server_reader, server_writer);
        let ((), decision) = tokio::join!(
            reply_to_permission_request(client, json!({ "outcome": outcome })),
            request_permission(
                &mut transport,
                &mut session,
                tmp.path(),
                action.clone(),
                "Allow code agent to edit files?",
                "The code agent may read and modify files.",
            ),
        );

        // Only `allow_always` records a session grant and a workspace trust entry.
        let always = expected == PermissionDecision::AlwaysAllow;
        assert_eq!(decision, expected);
        assert_eq!(session.always_allowed.contains(&action), always);
        assert_eq!(
            AcpSession::load_workspace_trust(tmp.path()).contains(&action),
            always
        );
    }
}

#[tokio::test]
async fn append_acp_episode_records_single_dispatch_episode() {
    let tmp = tempfile::tempdir().expect("create tmpdir");
    let workdir = tmp.path();
    let session = test_session("claude-sonnet-4-6", "none");
    let roko_config = RokoConfig::default();
    let stream_result = StreamResult {
        prompt_result: SessionPromptResult {
            stop_reason: StopReason::EndTurn,
        },
        assistant_text: "hello from acp".to_string(),
        usage: Some(UsageInfo {
            total_tokens: 12,
            input_tokens: 5,
            output_tokens: 7,
            thought_tokens: None,
            cached_read_tokens: Some(2),
            cached_write_tokens: Some(1),
        }),
    };
    let dispatch_started = Instant::now();
    let cascade_selection = AcpCascadeSelection {
        model_key: "claude-sonnet-4-6".to_owned(),
        stage: "confidence".to_owned(),
    };

    tokio::time::sleep(std::time::Duration::from_millis(1)).await;

    append_acp_episode(
        &roko_config,
        workdir,
        &session,
        &session.config_state.model,
        "trim a file",
        &session.config_state.workflow,
        false,
        dispatch_started,
        Some(&stream_result),
        None,
        None,
        None,
        Some(&cascade_selection),
    )
    .await;

    let episodes_path = workdir.join(".roko").join("episodes.jsonl");
    let episodes = EpisodeLogger::read_all(&episodes_path)
        .await
        .expect("read episodes");

    assert_eq!(episodes.len(), 1);
    let episode = &episodes[0];
    assert_eq!(episode.kind, "acp-dispatch");
    assert_eq!(episode.agent_template, "code");
    assert_eq!(episode.task_id, session.session_id);
    assert_eq!(episode.extra.get("entry_point"), Some(&json!("acp")));
    assert_eq!(
        episode.extra.get("session_id"),
        Some(&json!(episode.task_id.clone()))
    );
    assert_eq!(
        episode.extra.get("cascade_selected_model"),
        Some(&json!("claude-sonnet-4-6"))
    );
    assert_eq!(
        episode.extra.get("cascade_stage"),
        Some(&json!("confidence"))
    );
    assert!(!episode.extra.contains_key("routing_mode"));
    assert!(episode.usage.wall_ms > 0);
    assert_eq!(episode.tokens_used, 12);
    assert_eq!(episode.usage.input_tokens, 5);
    assert_eq!(episode.usage.output_tokens, 7);
    assert_eq!(episode.usage.cache_read_tokens, 2);
    assert_eq!(episode.usage.cache_write_tokens, 1);
    assert!(episode.usage.cost_usd > 0.0);
    assert!(episode.usage.cost_usd_without_cache >= episode.usage.cost_usd);
    assert!(episode.success);
    assert_eq!(episode.failure_reason, None);
}

#[tokio::test]
async fn append_acp_episode_records_pipeline_kind() {
    let tmp = tempfile::tempdir().expect("create tmpdir");
    let workdir = tmp.path();
    let session = test_session("claude-sonnet-4-6", "express");
    let roko_config = RokoConfig::default();
    let stream_result = StreamResult {
        prompt_result: SessionPromptResult {
            stop_reason: StopReason::EndTurn,
        },
        assistant_text: "pipeline complete".to_string(),
        usage: None,
    };
    let dispatch_started = Instant::now();

    tokio::time::sleep(std::time::Duration::from_millis(1)).await;

    append_acp_episode(
        &roko_config,
        workdir,
        &session,
        &session.config_state.model,
        "wire ACP logging",
        &session.config_state.workflow,
        true,
        dispatch_started,
        Some(&stream_result),
        None,
        None,
        None,
        None,
    )
    .await;

    let episodes_path = workdir.join(".roko").join("episodes.jsonl");
    let episodes = EpisodeLogger::read_all(&episodes_path)
        .await
        .expect("read episodes");

    assert_eq!(episodes.len(), 1);
    let episode = &episodes[0];
    assert_eq!(episode.kind, "acp-pipeline-express");
    assert_eq!(episode.extra.get("workflow"), Some(&json!("express")));
    assert!(episode.success);
}

#[test]
fn acp_episodes_start_no_dream_when_dreams_are_off() {
    let tmp = tempfile::tempdir().expect("create tmpdir");
    let workdir = tmp.path();
    let roko_dir = workdir.join(".roko");
    std::fs::create_dir_all(&roko_dir).expect("create .roko");
    let episode_log: String = (0..12)
        .map(|i| {
            let episode = Episode::new("code", format!("acp-session-{i}"));
            serde_json::to_string(&episode).expect("serialize episode") + "\n"
        })
        .collect();
    std::fs::write(roko_dir.join("episodes.jsonl"), episode_log).expect("write episode log");
    assert!(!roko_dir.join("dreams").exists(), "no dream report yet");

    // Default config: 12 episodes and no dream report start no dream.
    let config = RokoConfig::default();
    assert!(!config.learning.dreams.trigger_on_acp_episodes);
    assert_eq!(acp_dream_due(workdir, &config), None);
    // No Tokio runtime runs this test, so spawning a dream would panic.
    maybe_spawn_dream_consolidation(workdir, &config);
    assert!(!roko_dir.join("dreams").exists());

    // The opt-in key starts one, with the threshold read from config.
    let mut opted_in = RokoConfig::default();
    opted_in.learning.dreams.trigger_on_acp_episodes = true;
    opted_in.learning.dreams.acp_episode_threshold = 10;
    assert_eq!(acp_dream_due(workdir, &opted_in), Some(12));

    opted_in.learning.dreams.acp_episode_threshold = 13;
    assert_eq!(acp_dream_due(workdir, &opted_in), None);
}

/// bug-31bca6: a dream stays due until it writes its report, so the trigger
/// starts no more than `learning.dreams.max_concurrent` dreams while they run.
#[test]
fn acp_starts_no_dream_while_one_is_running() {
    let tmp = tempfile::tempdir().expect("create tmpdir");
    let workdir = tmp.path();
    let roko_dir = workdir.join(".roko");
    std::fs::create_dir_all(&roko_dir).expect("create .roko");
    let episode_log: String = (0..3)
        .map(|i| {
            let episode = Episode::new("code", format!("acp-session-{i}"));
            serde_json::to_string(&episode).expect("serialize episode") + "\n"
        })
        .collect();
    std::fs::write(roko_dir.join("episodes.jsonl"), episode_log).expect("write episode log");

    let mut config = RokoConfig::default();
    config.learning.dreams.trigger_on_acp_episodes = true;
    config.learning.dreams.acp_episode_threshold = 2;
    assert_eq!(config.learning.dreams.max_concurrent, 1);

    // The first trigger takes the only slot; while that dream runs, the
    // next trigger finds the dream still due and starts none.
    let slots = DreamSlots::new();
    let (episodes, first) = claim_acp_dream(&slots, workdir, &config).expect("a dream is due");
    assert_eq!(episodes, 3);
    assert!(claim_acp_dream(&slots, workdir, &config).is_none());

    // A second slot admits one more dream, and no third.
    config.learning.dreams.max_concurrent = 2;
    let second = claim_acp_dream(&slots, workdir, &config).expect("a second slot is free");
    assert!(claim_acp_dream(&slots, workdir, &config).is_none());

    // Finished dreams free their slots. Zero counts as one.
    drop(first);
    drop(second);
    config.learning.dreams.max_concurrent = 0;
    let third = claim_acp_dream(&slots, workdir, &config).expect("the slots are free");
    assert!(claim_acp_dream(&slots, workdir, &config).is_none());
    drop(third);
}

#[test]
fn acp_routing_context_maps_modes_to_roles() {
    let tmp = tempfile::tempdir().expect("create tmpdir");
    let workdir = tmp.path();

    let plan = acp_routing_context("plan", "wire router feedback", "high", workdir);
    assert_eq!(plan.task_category, TaskCategory::Implementation);
    assert_eq!(plan.role, AgentRole::Strategist);
    assert_eq!(plan.thinking_level.as_deref(), Some("high"));

    let research = acp_routing_context("research", "find the source of truth", "medium", workdir);
    assert_eq!(research.task_category, TaskCategory::Research);
    assert_eq!(research.role, AgentRole::Researcher);

    let code = acp_routing_context("code", "edit file", "low", workdir);
    assert_eq!(code.task_category, TaskCategory::Implementation);
    assert_eq!(code.role, AgentRole::Implementer);
}

#[test]
fn acp_routing_context_loads_canonical_daimon_affect() {
    let tmp = tempfile::tempdir().expect("create tmpdir");
    let daimon_dir = tmp.path().join(".roko/daimon");
    std::fs::create_dir_all(&daimon_dir).expect("create daimon dir");
    std::fs::write(
        daimon_dir.join("affect.json"),
        serde_json::json!({
            "state": {
                "confidence": 0.23,
                "behavioral_state": "struggling"
            }
        })
        .to_string(),
    )
    .expect("write affect state");

    let context = acp_routing_context("code", "repair failure", "high", tmp.path());

    assert!((context.daimon_policy.affect_confidence - 0.23).abs() < f64::EPSILON);
    assert_eq!(
        context.daimon_policy.behavioral_state,
        roko_core::BehavioralState::Struggling
    );
}

#[test]
fn acp_dispatch_reward_distinguishes_success_and_failure() {
    assert_eq!(compute_acp_reward(false, 200, Some(120)), 0.0);
    assert!(compute_acp_reward(true, 1_000, Some(1_000)) > 0.9);
    assert!(compute_acp_reward(true, 20_000, None) >= 0.8);
}

#[test]
fn cascade_router_model_slugs_falls_back_when_config_is_empty() {
    let config = RokoConfig::default();
    let slugs = cascade_router_model_slugs(&config, "fallback-slug");
    assert_eq!(slugs, vec!["fallback-slug".to_string()]);
}

#[test]
fn cascade_router_model_slugs_are_deterministically_sorted() {
    let mut config = RokoConfig::default();
    config
        .models
        .insert("z-model".to_owned(), ModelProfile::default());
    config
        .models
        .insert("a-model".to_owned(), ModelProfile::default());

    assert_eq!(
        cascade_router_model_slugs(&config, "unused"),
        vec!["a-model".to_owned(), "z-model".to_owned()]
    );
}

#[test]
fn resolved_acp_dispatch_uses_the_cascade_config_key() {
    let mut config = RokoConfig::default();
    config.models.insert(
        "requested".to_owned(),
        ModelProfile {
            slug: "wire-requested".to_owned(),
            ..ModelProfile::default()
        },
    );
    config.models.insert(
        "selected".to_owned(),
        ModelProfile {
            slug: "wire-selected".to_owned(),
            ..ModelProfile::default()
        },
    );

    let selection = AcpCascadeSelection {
        model_key: "selected".to_owned(),
        stage: "confidence".to_owned(),
    };
    let (resolved, dispatch_key, retained) =
        resolve_acp_dispatch_model(&config, "requested", Some(selection.clone()));

    assert_eq!(dispatch_key, "selected");
    assert_eq!(resolved.model_key, "selected");
    assert_eq!(resolved.slug, "wire-selected");
    assert_eq!(retained, Some(selection));
}

#[test]
fn resolved_acp_dispatch_rejects_an_unconfigured_cascade_key() {
    let mut config = RokoConfig::default();
    config.models.insert(
        "requested".to_owned(),
        ModelProfile {
            slug: "wire-requested".to_owned(),
            ..ModelProfile::default()
        },
    );

    let (resolved, dispatch_key, retained) = resolve_acp_dispatch_model(
        &config,
        "requested",
        Some(AcpCascadeSelection {
            model_key: "not-configured".to_owned(),
            stage: "ucb".to_owned(),
        }),
    );

    assert_eq!(dispatch_key, "requested");
    assert_eq!(resolved.model_key, "requested");
    assert!(retained.is_none());
}

#[test]
fn calculate_cost_for_model_slug_handles_known_and_unknown_models() {
    let known = calculate_cost_for_model_slug("claude-sonnet-4-6", 1_000, 500, 250)
        .expect("known pricing should exist");
    assert!(known > 0.0);

    assert_eq!(
        calculate_cost_for_model_slug("definitely-not-a-real-model", 1_000, 500, 250),
        None
    );
}

#[test]
fn assistant_history_truncation_caps_bytes_and_preserves_boundaries() {
    let text = "é".repeat(6_000);
    let truncated = truncate_assistant_history(&text);
    let suffix = "...[truncated]";
    let prefix_len = truncated.len() - suffix.len();

    assert!(truncated.ends_with(suffix));
    assert!(truncated.len() <= MAX_HISTORY_ASSISTANT_BYTES + suffix.len());
    assert!(truncated.len() < text.len());
    assert!(truncated[..prefix_len].chars().all(|c| c == 'é'));
}

#[test]
fn tool_name_mapping() {
    assert_eq!(tool_name_to_kind("Edit"), ToolCallKind::Edit);
    assert_eq!(tool_name_to_kind("Write"), ToolCallKind::Edit);
    assert_eq!(tool_name_to_kind("Bash"), ToolCallKind::Terminal);
    assert_eq!(tool_name_to_kind("Read"), ToolCallKind::Other);
}

#[test]
fn extract_at_mentions_supports_embedded_mentions() {
    let mentions = extract_at_mentions("fix @src/main.rs and @branch-diff, not foo@bar.com");
    assert_eq!(mentions, vec!["src/main.rs", "branch-diff"]);
}

#[tokio::test]
async fn resolve_context_items_resolves_resource_and_path_mentions() {
    let tmp = tempfile::tempdir().expect("create tmpdir");
    let workdir = tmp.path();
    let file_path = workdir.join("src/main.rs");
    std::fs::create_dir_all(file_path.parent().expect("parent directory")).expect("create dirs");
    std::fs::write(&file_path, "fn main() {}\n").expect("write file");

    let prompt = vec![
        ContentBlock::Resource {
            resource: crate::types::ResourceRef::File {
                uri: format!("file://{}", file_path.display()),
            },
        },
        ContentBlock::Text {
            text: "check @src/main.rs".to_owned(),
        },
    ];

    let context = resolve_context_items(&prompt, workdir).await;
    assert!(context.contains("<file path=\"src/main.rs\">"));
    assert!(context.contains("--- src/main.rs ---"));
    assert!(context.contains("fn main() {}"));
}

#[test]
fn truncate_with_limit_is_char_safe() {
    let text = "é".repeat(20_000);
    let truncated = truncate_with_limit(&text, 32_768, "... [truncated]");
    let prefix_len = truncated.len() - "... [truncated]".len();

    assert!(truncated.ends_with("... [truncated]"));
    assert!(truncated.len() < text.len());
    assert!(truncated[..prefix_len].chars().all(|c| c == 'é'));
}

#[tokio::test]
async fn build_provenance_includes_all_source_types() {
    let tmp = tempfile::tempdir().expect("create tmpdir");
    let workdir = tmp.path();
    std::fs::create_dir_all(workdir.join(".roko").join("learn")).expect("create learn dir");

    let playbook = Playbook {
        id: "dispatch-chain".into(),
        name: "dispatch-chain".into(),
        goal: "Reuse the proven dispatch path for similar tasks".into(),
        when_pattern: Some("dispatch path".into()),
        steps: Vec::new(),
        success_count: 3,
        failure_count: 1,
        created_at_ms: 0,
        last_used_ms: Some(0),
    };

    let mut episode = Episode::new("agent-1", playbook.id.as_str()).succeeded();
    episode.kind = "agent_turn".into();
    episode.gate_verdicts = vec![
        roko_learn::episode_logger::EpisodeGateVerdict::new("compile", true),
        roko_learn::episode_logger::EpisodeGateVerdict::new("test", true),
    ];
    let logger = EpisodeLogger::new(workdir.join(".roko").join("episodes.jsonl"));
    logger.append(&episode).await.expect("append episode");

    let advice = roko_dreams::DreamRoutingAdvice {
        generated_at: chrono::Utc::now(),
        source_dream_report: "dream-report".into(),
        recommendations: Vec::new(),
        pattern_summaries: vec![roko_dreams::PatternSummary {
            description: "dispatch decisions should show the evidence chain".into(),
            applies_to: vec!["dispatch".into()],
            guidance: "surface the chain before strategist work starts".into(),
            confidence: 0.91,
            signature: 42,
        }],
    };
    std::fs::write(
        workdir
            .join(".roko")
            .join("learn")
            .join("dream-routing-advice.json"),
        serde_json::to_string(&advice).expect("serialize dream advice"),
    )
    .expect("write dream advice");

    let knowledge_hits = vec![KnowledgeQueryHit {
        entry: roko_neuro::KnowledgeEntry {
            id: "knowledge-1".into(),
            kind: KnowledgeKind::StrategyFragment,
            content: "Prefer the proven dispatcher path".into(),
            confidence: 0.9,
            tier: KnowledgeTier::Persistent,
            source_episodes: vec![playbook.id.clone()],
            tags: vec!["dispatch".into()],
            ..Default::default()
        },
        total_score: 0.85,
        breakdown: roko_neuro::KnowledgeQueryBreakdown {
            keyword_score: 1.0,
            effective_confidence: 0.9,
            recency_factor: 1.0,
            emotional_boost: 1.0,
            balance_freshness_boost: 0.0,
            hdc_similarity: None,
        },
    }];

    let chain = build_provenance(
        &knowledge_hits,
        &[playbook],
        "dispatch the request",
        workdir,
    )
    .await
    .expect("meaningful provenance");

    assert_eq!(chain.sources.len(), 4);
    assert!(matches!(
        chain.sources[0],
        ProvenanceSource::Playbook { .. }
    ));
    assert!(matches!(chain.sources[1], ProvenanceSource::Episode { .. }));
    assert!(matches!(
        chain.sources[2],
        ProvenanceSource::Knowledge { .. }
    ));
    assert!(matches!(
        chain.sources[3],
        ProvenanceSource::DreamPattern { .. }
    ));
    assert!(chain.confidence > 0.0);

    let card = render_provenance_card(&chain);
    assert!(card.contains("4 sources"));
    assert!(card.contains("Playbook `dispatch-chain`"));
    assert!(card.contains("Episode `dispatch-chain`"));
    assert!(card.contains("Knowledge [strategy_fragment/persistent]"));
    assert!(card.contains("Dream pattern"));
}

#[tokio::test]
async fn build_provenance_suppresses_trivial_knowledge_only_chains() {
    let tmp = tempfile::tempdir().expect("create tmpdir");
    let workdir = tmp.path();

    let knowledge_hits = vec![KnowledgeQueryHit {
        entry: roko_neuro::KnowledgeEntry {
            id: "knowledge-2".into(),
            kind: KnowledgeKind::Insight,
            content: "A lone idea without supporting history".into(),
            confidence: 0.5,
            tier: KnowledgeTier::Working,
            ..Default::default()
        },
        total_score: 0.4,
        breakdown: roko_neuro::KnowledgeQueryBreakdown {
            keyword_score: 1.0,
            effective_confidence: 0.5,
            recency_factor: 1.0,
            emotional_boost: 1.0,
            balance_freshness_boost: 0.0,
            hdc_similarity: None,
        },
    }];

    let chain = build_provenance(&knowledge_hits, &[], "small", workdir).await;
    assert!(chain.is_none());
}

#[tokio::test]
async fn permission_request_event_carries_reply_channel() {
    // Create a oneshot channel for the permission decision.
    let (tx, rx) = tokio::sync::oneshot::channel::<PermissionDecision>();

    // Build the PermissionRequest event.
    let payload = PermissionRequestPayload {
        action: PermissionAction::FileEdit,
        title: "Edit src/main.rs".into(),
        detail: "Replace println with tracing macro".into(),
    };
    let reply = PermissionReplyChannel::new(tx);
    let event = CognitiveEvent::PermissionRequest {
        payload: payload.clone(),
        reply: reply.clone(),
    };

    // Verify the event carries the expected payload fields.
    match &event {
        CognitiveEvent::PermissionRequest {
            payload: p,
            reply: r,
        } => {
            assert_eq!(p.action, PermissionAction::FileEdit);
            assert_eq!(p.title, "Edit src/main.rs");
            assert_eq!(p.detail, "Replace println with tracing macro");
            assert!(!r.is_consumed(), "reply channel should not be consumed yet");
        }
        _ => panic!("expected PermissionRequest variant"),
    }

    // Send the event through an mpsc channel (simulating the real flow).
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<CognitiveEvent>(4);
    event_tx.send(event).await.expect("send event");
    drop(event_tx);

    let received = event_rx.recv().await.expect("receive event");
    match received {
        CognitiveEvent::PermissionRequest { reply, .. } => {
            // Parent loop replies with Allow.
            assert!(reply.reply(PermissionDecision::Allow));
            // Second reply should fail (already consumed).
            assert!(!reply.reply(PermissionDecision::Reject));
            assert!(reply.is_consumed());
        }
        _ => panic!("expected PermissionRequest variant"),
    }

    // The tool loop receives the decision.
    let decision = rx.await.expect("receive decision");
    assert_eq!(decision, PermissionDecision::Allow);
}

#[tokio::test]
async fn permission_reply_channel_dropped_without_reply_gives_recv_error() {
    let (tx, rx) = tokio::sync::oneshot::channel::<PermissionDecision>();
    let reply = PermissionReplyChannel::new(tx);

    // Drop without replying — simulates parent loop crash / timeout.
    drop(reply);

    // The receiver should get an error (fail-closed).
    assert!(
        rx.await.is_err(),
        "dropped reply channel must produce RecvError"
    );
}

// ── AcpBuiltinToolHandler permission enforcement ─────────────────────────

#[tokio::test]
async fn acp_builtin_tool_handler_respects_denied_tools() {
    let (tx, _rx) = mpsc::channel(16);
    let handler = AcpBuiltinToolHandler {
        tool_name: "bash".into(),
        session_id: "test-session".into(),
        workdir: std::env::temp_dir(),
        event_sender: tx,
        role: "implementer".into(),
    };
    let call = ToolCall {
        id: "t1".into(),
        name: "bash".into(),
        arguments: serde_json::json!({"command": "echo hi"}),
        request_ts_ms: 0,
    };
    let mut ctx = ToolContext::testing(std::env::temp_dir());
    ctx.denied_tools = Some(vec!["bash".into()]);
    let result = handler.execute(call, &ctx).await;
    assert!(
        matches!(result, ToolResult::Err(_)),
        "denied tool must return ToolResult::Err"
    );
}

#[tokio::test]
async fn acp_builtin_tool_handler_respects_allowed_tools() {
    let (tx, _rx) = mpsc::channel(16);
    let handler = AcpBuiltinToolHandler {
        tool_name: "bash".into(),
        session_id: "test-session".into(),
        workdir: std::env::temp_dir(),
        event_sender: tx,
        role: "implementer".into(),
    };
    let call = ToolCall {
        id: "t1".into(),
        name: "bash".into(),
        arguments: serde_json::json!({"command": "echo hi"}),
        request_ts_ms: 0,
    };
    let mut ctx = ToolContext::testing(std::env::temp_dir());
    // bash is not in the allowed set — only read tools are.
    ctx.allowed_tools = Some(vec!["read_file".into(), "glob".into()]);
    let result = handler.execute(call, &ctx).await;
    assert!(
        matches!(result, ToolResult::Err(_)),
        "tool not in allowed set must return ToolResult::Err"
    );
}

#[tokio::test]
async fn acp_builtin_tool_handler_contract_denies_forbidden_tool() {
    // The implementer contract forbids web_fetch via ForbiddenTools governance.
    // The handler must deny the call before attempting execution.
    let (tx, _rx) = mpsc::channel(16);
    let handler = AcpBuiltinToolHandler {
        tool_name: "web_fetch".into(),
        session_id: "contract-deny-session".into(),
        workdir: std::env::temp_dir(),
        event_sender: tx,
        role: "implementer".into(),
    };
    let call = ToolCall {
        id: "c1".into(),
        name: "web_fetch".into(),
        arguments: serde_json::json!({"url": "https://example.com"}),
        request_ts_ms: 0,
    };
    let ctx = ToolContext::testing(std::env::temp_dir());
    let result = handler.execute(call, &ctx).await;
    assert!(
        matches!(result, ToolResult::Err(ToolError::PermissionDenied(_))),
        "AgentContract ForbiddenTools must deny web_fetch for implementer role, got {:?}",
        result,
    );
}

#[tokio::test]
async fn acp_builtin_tool_handler_unknown_role_falls_closed() {
    // An unrecognised role falls back to RestrictedFallback (empty allowed_tools).
    // Every tool must be denied.
    let (tx, _rx) = mpsc::channel(16);
    let handler = AcpBuiltinToolHandler {
        tool_name: "read_file".into(),
        session_id: "unknown-role-session".into(),
        workdir: std::env::temp_dir(),
        event_sender: tx,
        role: "completely-unknown-role-xyz".into(),
    };
    let call = ToolCall {
        id: "u1".into(),
        name: "read_file".into(),
        arguments: serde_json::json!({"path": "CLAUDE.md"}),
        request_ts_ms: 0,
    };
    let ctx = ToolContext::testing(std::env::temp_dir());
    let result = handler.execute(call, &ctx).await;
    assert!(
        matches!(result, ToolResult::Err(ToolError::PermissionDenied(_))),
        "Unknown role must fall closed (deny all tools), got {:?}",
        result,
    );
}

#[tokio::test]
async fn builtin_tool_permitted_in_default_code_mode() {
    // A new session starts in `code` mode, which loads the implementer contract.
    let session = test_session("test-model", "none");
    assert_eq!(session.config_state.agent_mode, "code");
    let role = acp_contract_role_for_mode(&session.config_state.agent_mode);
    assert_eq!(role, "implementer");

    let tmp = tempfile::tempdir().expect("create tmpdir");
    std::fs::write(tmp.path().join("notes.txt"), "read in code mode").expect("write fixture");
    let (tx, _rx) = mpsc::channel(16);
    let handler = AcpBuiltinToolHandler {
        tool_name: "read_file".into(),
        session_id: session.session_id.clone(),
        workdir: tmp.path().to_path_buf(),
        event_sender: tx,
        role,
    };
    let call = ToolCall {
        id: "code-mode-read".into(),
        name: "read_file".into(),
        arguments: json!({ "path": "notes.txt" }),
        request_ts_ms: 0,
    };
    let result = handler
        .execute(call, &ToolContext::testing(tmp.path()))
        .await;
    assert!(
        result.is_ok(),
        "read_file must run in code mode, got {result:?}"
    );
    assert_eq!(result.text_content(), "read in code mode");

    // The other modes load their own contracts, and unknown modes still fail closed.
    assert_eq!(acp_contract_role_for_mode("plan"), "strategist");
    assert_eq!(acp_contract_role_for_mode("research"), "researcher");
    assert_eq!(acp_contract_role_for_mode("unknown-mode"), "unknown-mode");
}

/// A stdio MCP server that lists one read-only `echo` tool and answers one call
/// to it. It creates the file named by its first argument when the call arrives.
const MCP_ECHO_FIXTURE: &str = r#"
    IFS= read -r initialize
    printf '%s\n' '{"jsonrpc":"2.0","id":1,"result":{"capabilities":{}}}'
    IFS= read -r list_tools
    printf '%s\n' '{"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"echo","inputSchema":{"type":"object"},"annotations":{"readOnlyHint":true}}]}}'
    IFS= read -r call || exit 0
    : > "$1"
    printf '%s\n' '{"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"echoed"}]}}'
"#;

/// Dispatches the fixture's `echo` tool through the dispatcher an ACP tool loop
/// builds for `mode`. Returns the result and whether the server got the call.
async fn dispatch_fixture_mcp_tool(mode: &str) -> (ToolResult, bool) {
    let tmp = tempfile::tempdir().expect("create tmpdir");
    let marker = tmp.path().join("tools-call-received");
    let servers = vec![crate::types::McpServerConfig {
        name: "fixture".into(),
        transport: crate::types::McpTransport::Stdio {
            command: "sh".into(),
            args: vec![
                "-c".into(),
                MCP_ECHO_FIXTURE.into(),
                "fixture".into(),
                marker.display().to_string(),
            ],
        },
        discovery_timeout_ms: Some(1_000),
    }];
    let (event_sender, _event_receiver) = mpsc::channel(16);
    let (runtime, statuses) = setup_session_mcp_tools(
        "mcp-contract-session",
        &servers,
        roko_agent::safety::capabilities::PluginTier::Sandboxed,
        event_sender,
    )
    .await;
    assert_eq!(statuses, vec![McpServerStatus::ready("fixture", 1)]);

    let registry = Arc::new(VecToolRegistry::from_tools(runtime.tools));
    let resolver: Arc<dyn HandlerResolver> = Arc::new(AcpMcpHandlerResolver {
        handlers: runtime.handlers,
    });
    let role = acp_contract_role_for_mode(mode);
    let safety = acp_tool_safety(&RokoConfig::default(), &role);
    let dispatcher = acp_tool_dispatcher(registry, resolver, safety);
    let call = ToolCall::new("mcp-contract-call", "fixture_echo", json!({}));
    let result = dispatcher
        .dispatch(call, &ToolContext::testing(tmp.path()))
        .await;
    (result, marker.exists())
}

#[tokio::test]
async fn mcp_tool_loop_allows_tool_permitted_by_role_contract() {
    // The default `code` mode loads the implementer contract, which permits the tool.
    let (result, received) = dispatch_fixture_mcp_tool("code").await;
    assert!(
        result.is_ok(),
        "code mode must run the MCP tool, got {result:?}"
    );
    assert!(result.text_content().contains("echoed"));
    assert!(received, "the MCP server must receive the call");
}

#[tokio::test]
async fn acp_tool_dispatcher_runs_builtin_tool_in_code_mode() {
    // The default dispatcher layer denies every tool; the role-scoped one admits
    // what the implementer contract permits.
    let tmp = tempfile::tempdir().expect("create tmpdir");
    std::fs::write(tmp.path().join("notes.txt"), "code mode dispatch").expect("write fixture");
    let role = acp_contract_role_for_mode("code");
    let (tx, _rx) = mpsc::channel(16);
    let mut handlers: HashMap<String, Arc<dyn ToolHandler>> = HashMap::new();
    handlers.insert(
        "read_file".to_owned(),
        Arc::new(AcpBuiltinToolHandler {
            tool_name: "read_file".into(),
            session_id: "dispatcher-code-mode".into(),
            workdir: tmp.path().to_path_buf(),
            event_sender: tx,
            role: role.clone(),
        }),
    );
    let registry = Arc::new(VecToolRegistry::from_tools(acp_builtin_tools()));
    let resolver: Arc<dyn HandlerResolver> = Arc::new(AcpBuiltinHandlerResolver { handlers });
    let safety = acp_tool_safety(&RokoConfig::default(), &role);
    let dispatcher = acp_tool_dispatcher(registry, resolver, safety);
    let call = ToolCall::new("read-1", "read_file", json!({ "path": "notes.txt" }));
    let result = dispatcher
        .dispatch(call, &ToolContext::testing(tmp.path()))
        .await;
    assert!(
        result.is_ok(),
        "read_file must pass the code-mode layer, got {result:?}"
    );
    assert!(result.text_content().contains("code mode dispatch"));
}

#[tokio::test]
async fn mcp_tool_loop_denies_tool_outside_role_contract() {
    // A mode with no bundled contract gets the deny-all restricted fallback.
    let (result, received) = dispatch_fixture_mcp_tool("unknown-mode").await;
    assert!(
        matches!(result, ToolResult::Err(ToolError::PermissionDenied(_))),
        "a tool outside the role contract must be denied, got {result:?}"
    );
    assert!(!received, "a denied call must never reach the MCP server");
}

#[tokio::test]
async fn permission_prompt_precedes_write() {
    let tmp = tempfile::tempdir().expect("create tmpdir");
    let target = tmp.path().join("permission-gated.txt");
    let mut session = test_session("test-model", "none");
    let session_id = session.session_id.clone();
    let cancel_token = session.cancel_token.clone();
    let (client, server) = duplex(4096);
    let (server_reader, server_writer) = tokio::io::split(server);
    let mut transport = StdioTransport::from_io(server_reader, server_writer);
    let (event_sender, event_receiver) = mpsc::channel(16);

    let handler = AcpBuiltinToolHandler {
        tool_name: "write_file".into(),
        session_id: session_id.clone(),
        workdir: tmp.path().to_path_buf(),
        event_sender: event_sender.clone(),
        role: "implementer".into(),
    };
    let context = ToolContext::testing(tmp.path());
    let handler_task = tokio::spawn(async move {
        handler
            .execute(
                ToolCall {
                    id: "wire-reject-write".into(),
                    name: "write_file".into(),
                    arguments: json!({
                        "path": "permission-gated.txt",
                        "content": "must not be written"
                    }),
                    request_ts_ms: 0,
                },
                &context,
            )
            .await
    });
    let completion_sender = event_sender.clone();
    let (editor_release_sender, editor_release_receiver) = tokio::sync::oneshot::channel();
    let completion_task = tokio::spawn(async move {
        let result = handler_task.await.expect("join permission-gated handler");
        completion_sender
            .send(CognitiveEvent::Complete {
                stop_reason: StopReason::EndTurn,
                usage: None,
            })
            .await
            .expect("queue completion after rejected tool");
        let _ = editor_release_sender.send(());
        result
    });
    drop(event_sender);

    let editor = async {
        let mut reader = BufReader::new(client);
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .await
            .expect("read outbound permission request");
        let request: serde_json::Value =
            serde_json::from_str(&line).expect("parse permission request");
        assert_eq!(request["method"], json!("session/request_permission"));
        assert!(!target.exists(), "permission must precede the write");

        let response = json!({
            "jsonrpc": "2.0",
            "id": request["id"].clone(),
            "result": {
                "outcome": { "type": "selected", "optionId": "reject_once" }
            }
        });
        let mut client = reader.into_inner();
        client
            .write_all(
                serde_json::to_string(&response)
                    .expect("serialize response")
                    .as_bytes(),
            )
            .await
            .expect("write permission rejection");
        client.write_all(b"\n").await.expect("write newline");
        client.flush().await.expect("flush permission rejection");
        let _ = editor_release_receiver.await;
    };

    let ((), stream_result) = tokio::join!(
        editor,
        stream_events_to_editor(
            &mut transport,
            &session_id,
            &mut session,
            tmp.path(),
            event_receiver,
            &cancel_token,
        )
    );
    let tool_result = completion_task.await.expect("join completion task");

    assert!(matches!(
        tool_result,
        ToolResult::Err(ToolError::PermissionDenied(_))
    ));
    assert!(
        !target.exists(),
        "wire-level rejection must block the write"
    );
    assert_eq!(
        stream_result
            .expect("permission stream should complete")
            .prompt_result
            .stop_reason,
        StopReason::EndTurn
    );
}

#[tokio::test]
async fn permission_wait_cancellation_is_bounded() {
    let tmp = tempfile::tempdir().expect("create tmpdir");
    let target = tmp.path().join("cancelled-permission.txt");
    let mut session = test_session("test-model", "none");
    let session_id = session.session_id.clone();
    let cancel_token = session.cancel_token.clone();
    let (client, server) = duplex(4096);
    let (server_reader, server_writer) = tokio::io::split(server);
    let mut transport = StdioTransport::from_io(server_reader, server_writer);
    let (event_sender, event_receiver) = mpsc::channel(16);

    let handler = AcpBuiltinToolHandler {
        tool_name: "write_file".into(),
        session_id: session_id.clone(),
        workdir: tmp.path().to_path_buf(),
        event_sender: event_sender.clone(),
        role: "implementer".into(),
    };
    let context = ToolContext::testing(tmp.path());
    let handler_task = tokio::spawn(async move {
        handler
            .execute(
                ToolCall {
                    id: "cancelled-write".into(),
                    name: "write_file".into(),
                    arguments: json!({
                        "path": "cancelled-permission.txt",
                        "content": "must not be written"
                    }),
                    request_ts_ms: 0,
                },
                &context,
            )
            .await
    });
    drop(event_sender);

    let editor_session_id = session_id.clone();
    let editor = async move {
        let mut reader = BufReader::new(client);
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .await
            .expect("read outbound permission request");
        let request: serde_json::Value =
            serde_json::from_str(&line).expect("parse permission request");
        assert_eq!(request["method"], json!("session/request_permission"));

        let cancel = json!({
            "jsonrpc": "2.0",
            "method": "session/cancel",
            "params": { "sessionId": editor_session_id }
        });
        let mut client = reader.into_inner();
        client
            .write_all(
                serde_json::to_string(&cancel)
                    .expect("serialize cancel")
                    .as_bytes(),
            )
            .await
            .expect("write session cancel");
        client.write_all(b"\n").await.expect("write newline");
        client.flush().await.expect("flush session cancel");
    };

    let joined = tokio::time::timeout(Duration::from_secs(1), async {
        tokio::join!(
            editor,
            stream_events_to_editor(
                &mut transport,
                &session_id,
                &mut session,
                tmp.path(),
                event_receiver,
                &cancel_token,
            )
        )
    })
    .await
    .expect("permission cancellation must not wait for the editor timeout");
    let ((), stream_result) = joined;
    let tool_result = handler_task.await.expect("join cancelled handler");

    assert!(matches!(
        tool_result,
        ToolResult::Err(ToolError::PermissionDenied(_))
    ));
    assert_eq!(
        stream_result
            .expect("cancelled permission stream should return a prompt result")
            .prompt_result
            .stop_reason,
        StopReason::Cancelled
    );
    assert!(cancel_token.is_cancelled());
    assert!(
        !target.exists(),
        "cancelled permission must block the write"
    );
}

#[tokio::test]
async fn permission_wait_abandons_when_requester_times_out() {
    let tmp = tempfile::tempdir().expect("create tmpdir");
    let mut session = test_session("test-model", "none");
    let session_id = session.session_id.clone();
    let cancel_token = session.cancel_token.clone();
    let (client, server) = duplex(4096);
    let (server_reader, server_writer) = tokio::io::split(server);
    let mut transport = StdioTransport::from_io(server_reader, server_writer);
    let (event_sender, event_receiver) = mpsc::channel(4);
    let (decision_sender, decision_receiver) = tokio::sync::oneshot::channel();
    let (request_seen_sender, request_seen_receiver) = tokio::sync::oneshot::channel();

    event_sender
        .send(CognitiveEvent::PermissionRequest {
            payload: PermissionRequestPayload {
                action: PermissionAction::FileCreate,
                title: "Write timed-out.txt".to_owned(),
                detail: "Allow this ACP turn to write the requested file?".to_owned(),
            },
            reply: PermissionReplyChannel::new(decision_sender),
        })
        .await
        .expect("queue permission event");
    event_sender
        .send(CognitiveEvent::Complete {
            stop_reason: StopReason::EndTurn,
            usage: None,
        })
        .await
        .expect("queue completion event");
    drop(event_sender);

    let editor = async move {
        let mut reader = BufReader::new(client);
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .await
            .expect("read outbound permission request");
        let request: serde_json::Value =
            serde_json::from_str(&line).expect("parse permission request");
        assert_eq!(request["method"], json!("session/request_permission"));
        request_seen_sender
            .send(())
            .expect("signal request observed");
        tokio::time::sleep(Duration::from_millis(100)).await;
    };
    let requester_timeout = async move {
        request_seen_receiver
            .await
            .expect("request should be observed");
        drop(decision_receiver);
    };

    let ((), (), stream_result) = tokio::time::timeout(Duration::from_secs(1), async {
        tokio::join!(
            editor,
            requester_timeout,
            stream_events_to_editor(
                &mut transport,
                &session_id,
                &mut session,
                tmp.path(),
                event_receiver,
                &cancel_token,
            )
        )
    })
    .await
    .expect("requester timeout must abandon the longer editor wait");

    assert_eq!(
        stream_result
            .expect("stream should continue after abandoned permission request")
            .prompt_result
            .stop_reason,
        StopReason::EndTurn
    );
}

#[tokio::test]
async fn acp_builtin_permission_decisions_gate_write() {
    let tmp = tempfile::tempdir().expect("create tmpdir");
    let target = tmp.path().join("permission-gated.txt");
    let call = || ToolCall {
        id: "permission-write".into(),
        name: "write_file".into(),
        arguments: json!({
            "path": "permission-gated.txt",
            "content": "written only after approval"
        }),
        request_ts_ms: 0,
    };

    // Reject: the request is observable before any side effect and the
    // target remains absent after the handler returns.
    let (reject_sender, mut reject_events) = mpsc::channel(16);
    let reject_handler = AcpBuiltinToolHandler {
        tool_name: "write_file".into(),
        session_id: "permission-reject".into(),
        workdir: tmp.path().to_path_buf(),
        event_sender: reject_sender,
        role: "implementer".into(),
    };
    let reject_context = ToolContext::testing(tmp.path());
    let reject_task =
        tokio::spawn(async move { reject_handler.execute(call(), &reject_context).await });
    let reject_reply = match reject_events.recv().await.expect("permission request") {
        CognitiveEvent::PermissionRequest { payload, reply } => {
            assert_eq!(payload.action, PermissionAction::FileCreate);
            assert!(!target.exists(), "permission must precede the write");
            reply
        }
        other => panic!("expected permission request, got {other:?}"),
    };
    assert!(reject_reply.reply(PermissionDecision::Reject));
    let rejected = reject_task.await.expect("join rejected write");
    assert!(matches!(
        rejected,
        ToolResult::Err(ToolError::PermissionDenied(_))
    ));
    assert!(!target.exists(), "rejected write must have no side effect");

    // Allow: execution resumes only after the positive decision.
    let (allow_sender, mut allow_events) = mpsc::channel(16);
    let allow_handler = AcpBuiltinToolHandler {
        tool_name: "write_file".into(),
        session_id: "permission-allow".into(),
        workdir: tmp.path().to_path_buf(),
        event_sender: allow_sender,
        role: "implementer".into(),
    };
    let allow_context = ToolContext::testing(tmp.path());
    let allow_task =
        tokio::spawn(async move { allow_handler.execute(call(), &allow_context).await });
    let allow_reply = match allow_events.recv().await.expect("permission request") {
        CognitiveEvent::PermissionRequest { reply, .. } => {
            assert!(!target.exists(), "permission must precede the write");
            reply
        }
        other => panic!("expected permission request, got {other:?}"),
    };
    assert!(allow_reply.reply(PermissionDecision::Allow));
    let allowed = allow_task.await.expect("join allowed write");
    assert!(allowed.is_ok());
    assert_eq!(
        tokio::fs::read_to_string(&target)
            .await
            .expect("allowed write should create target"),
        "written only after approval"
    );

    // A dropped parent reply is a prompt denial, not a hang or fail-open.
    tokio::fs::remove_file(&target)
        .await
        .expect("remove target before dropped-reply case");
    let (drop_sender, mut drop_events) = mpsc::channel(16);
    let drop_handler = AcpBuiltinToolHandler {
        tool_name: "write_file".into(),
        session_id: "permission-dropped".into(),
        workdir: tmp.path().to_path_buf(),
        event_sender: drop_sender,
        role: "implementer".into(),
    };
    let drop_context = ToolContext::testing(tmp.path());
    let drop_task = tokio::spawn(async move { drop_handler.execute(call(), &drop_context).await });
    match drop_events.recv().await.expect("permission request") {
        CognitiveEvent::PermissionRequest { reply, .. } => drop(reply),
        other => panic!("expected permission request, got {other:?}"),
    }
    let dropped = tokio::time::timeout(Duration::from_secs(1), drop_task)
        .await
        .expect("dropped reply must not hang")
        .expect("join dropped-reply write");
    assert!(matches!(
        dropped,
        ToolResult::Err(ToolError::PermissionDenied(_))
    ));
    assert!(!target.exists(), "dropped reply must not execute the write");
}

#[tokio::test]
async fn slash_command_streaming_forwards_stdout_and_stderr_before_eof() {
    let (mut stdout_writer, stdout_reader) = duplex(1024);
    let (mut stderr_writer, stderr_reader) = duplex(1024);
    let (event_sender, mut event_receiver) = mpsc::channel(8);
    let cancel = CancelToken::new();
    let cancel_for_task = cancel.clone();
    let stream_task = tokio::spawn(async move {
        forward_slash_command_streams(
            "stream-test",
            stdout_reader,
            stderr_reader,
            &cancel_for_task,
            &event_sender,
        )
        .await
    });

    stdout_writer
        .write_all(b"first line\n")
        .await
        .expect("write stdout");
    stdout_writer.flush().await.expect("flush stdout");
    let first = tokio::time::timeout(Duration::from_secs(1), event_receiver.recv())
        .await
        .expect("stdout delivered before eof")
        .expect("stdout event");
    assert!(matches!(first, CognitiveEvent::TokenChunk(text) if text == "first line\n"));

    stderr_writer
        .write_all(b"warning\n")
        .await
        .expect("write stderr");
    stderr_writer.flush().await.expect("flush stderr");
    let second = tokio::time::timeout(Duration::from_secs(1), event_receiver.recv())
        .await
        .expect("stderr delivered before eof")
        .expect("stderr event");
    assert!(matches!(
        second,
        CognitiveEvent::TokenChunk(text) if text == "\x1b[2mwarning\x1b[0m\n"
    ));

    drop(stdout_writer);
    drop(stderr_writer);
    assert_eq!(
        stream_task.await.expect("stream task"),
        SlashCommandStreamOutcome::Completed { had_output: true }
    );
}

#[tokio::test]
async fn slash_command_streaming_preserves_unknown_progress_and_correlates_tasks() {
    let (mut stdout_writer, stdout_reader) = duplex(4096);
    let (stderr_writer, stderr_reader) = duplex(64);
    drop(stderr_writer);
    let (event_sender, mut event_receiver) = mpsc::channel(16);
    let cancel = CancelToken::new();
    let cancel_for_task = cancel.clone();
    let stream_task = tokio::spawn(async move {
        forward_slash_command_streams(
            "progress-test",
            stdout_reader,
            stderr_reader,
            &cancel_for_task,
            &event_sender,
        )
        .await
    });

    stdout_writer
        .write_all(
            b"ROKO_PROGRESS: {not-json}\nROKO_PROGRESS: {\"type\":\"future_event\"}\nROKO_PROGRESS: {\"type\":\"task_started\",\"task_id\":\"A\",\"title\":\"alpha\"}\nROKO_PROGRESS: {\"type\":\"task_started\",\"task_id\":\"B\",\"title\":\"beta\"}\nROKO_PROGRESS: {\"type\":\"task_completed\",\"task_id\":\"A\",\"completed\":1,\"total\":2}\n",
        )
        .await
        .expect("write progress lines");
    drop(stdout_writer);

    assert_eq!(
        stream_task.await.expect("stream task"),
        SlashCommandStreamOutcome::Completed { had_output: true }
    );

    let mut events = Vec::new();
    while let Ok(event) = event_receiver.try_recv() {
        events.push(event);
    }
    assert!(matches!(
        &events[0],
        CognitiveEvent::TokenChunk(text) if text == "ROKO_PROGRESS: {not-json}\n"
    ));
    assert!(matches!(
        &events[1],
        CognitiveEvent::TokenChunk(text) if text.contains("future_event")
    ));
    assert!(matches!(
        &events[2],
        CognitiveEvent::ToolCallStart { tool_call_id, .. } if tool_call_id == "progress-A-1"
    ));
    assert!(matches!(
        &events[3],
        CognitiveEvent::ToolCallStart { tool_call_id, .. } if tool_call_id == "progress-B-2"
    ));
    assert!(matches!(
        &events[4],
        CognitiveEvent::ToolCallComplete { tool_call_id, .. } if tool_call_id == "progress-A-1"
    ));
    assert!(matches!(
        &events[5],
        CognitiveEvent::ToolCallComplete {
            tool_call_id,
            status: ToolCallStatus::Failed,
            ..
        } if tool_call_id == "progress-B-2"
    ));
}

#[tokio::test]
async fn slash_command_streaming_failure_closes_attempt_before_retry() {
    let (mut stdout_writer, stdout_reader) = duplex(4096);
    let (stderr_writer, stderr_reader) = duplex(64);
    drop(stderr_writer);
    let (event_sender, mut event_receiver) = mpsc::channel(16);
    let cancel = CancelToken::new();
    let cancel_for_task = cancel.clone();
    let stream_task = tokio::spawn(async move {
        forward_slash_command_streams(
            "retry-test",
            stdout_reader,
            stderr_reader,
            &cancel_for_task,
            &event_sender,
        )
        .await
    });

    stdout_writer
        .write_all(
            b"ROKO_PROGRESS: {\"type\":\"task_started\",\"task_id\":\"A\",\"title\":\"first\"}\nROKO_PROGRESS: {\"type\":\"task_failed\",\"task_id\":\"A\",\"error\":\"retry me\"}\nROKO_PROGRESS: {\"type\":\"task_started\",\"task_id\":\"A\",\"title\":\"retry\"}\nROKO_PROGRESS: {\"type\":\"task_completed\",\"task_id\":\"A\",\"completed\":1,\"total\":1}\n",
        )
        .await
        .expect("write progress lines");
    drop(stdout_writer);

    assert_eq!(
        stream_task.await.expect("stream task"),
        SlashCommandStreamOutcome::Completed { had_output: true }
    );
    let mut events = Vec::new();
    while let Ok(event) = event_receiver.try_recv() {
        events.push(event);
    }
    assert!(matches!(
        &events[0],
        CognitiveEvent::ToolCallStart { tool_call_id, .. }
            if tool_call_id == "progress-A-1"
    ));
    assert!(matches!(
        &events[1],
        CognitiveEvent::ToolCallComplete {
            tool_call_id,
            status: ToolCallStatus::Failed,
            content,
        } if tool_call_id == "progress-A-1"
            && matches!(content.as_slice(), [ContentBlock::Text { text }] if text == "retry me")
    ));
    assert!(matches!(
        &events[2],
        CognitiveEvent::ToolCallStart { tool_call_id, .. }
            if tool_call_id == "progress-A-2"
    ));
    assert!(matches!(
        &events[3],
        CognitiveEvent::ToolCallComplete {
            tool_call_id,
            status: ToolCallStatus::Completed,
            ..
        } if tool_call_id == "progress-A-2"
    ));
}

#[tokio::test]
async fn slash_command_empty_output_emits_one_fallback_then_completion() {
    let (event_sender, mut event_receiver) = mpsc::channel(4);
    finish_slash_command_stream("plan-run", false, &event_sender).await;

    assert!(matches!(
        event_receiver.recv().await,
        Some(CognitiveEvent::TokenChunk(text)) if text == "/plan-run completed (no output)"
    ));
    assert!(matches!(
        event_receiver.recv().await,
        Some(CognitiveEvent::Complete { .. })
    ));
    assert!(event_receiver.try_recv().is_err());
}

#[tokio::test]
async fn slash_command_streaming_cancellation_does_not_emit_completion() {
    let (mut stdout_writer, stdout_reader) = duplex(512);
    let (_stderr_writer, stderr_reader) = duplex(64);
    let (event_sender, mut event_receiver) = mpsc::channel(8);
    let cancel = CancelToken::new();
    let cancel_for_task = cancel.clone();
    let stream_task = tokio::spawn(async move {
        forward_slash_command_streams(
            "cancel-test",
            stdout_reader,
            stderr_reader,
            &cancel_for_task,
            &event_sender,
        )
        .await
    });

    stdout_writer
        .write_all(
            b"ROKO_PROGRESS: {\"type\":\"task_started\",\"task_id\":\"A\",\"title\":\"alpha\"}\n",
        )
        .await
        .expect("write task start");
    assert!(matches!(
        event_receiver.recv().await,
        Some(CognitiveEvent::ToolCallStart { tool_call_id, .. })
            if tool_call_id == "progress-A-1"
    ));
    cancel.cancel();
    assert_eq!(
        stream_task.await.expect("stream task"),
        SlashCommandStreamOutcome::Cancelled
    );
    assert!(matches!(
        event_receiver.recv().await,
        Some(CognitiveEvent::ToolCallComplete {
            tool_call_id,
            status: ToolCallStatus::Failed,
            content,
        }) if tool_call_id == "progress-A-1"
            && matches!(content.as_slice(), [ContentBlock::Text { text }] if text == "cancelled")
    ));
    assert!(!matches!(
        event_receiver.try_recv(),
        Ok(CognitiveEvent::Complete { .. })
    ));
}

// ── T6: cascade_select_model tests ───────────────────────────────────────
//
// Tests that mutate the `ROKO_ACP_CASCADE_SELECT` env var share a module-level
// Mutex so they run serially and cannot interfere with each other.

static CASCADE_ENV_LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();

fn cascade_env_lock() -> &'static std::sync::Mutex<()> {
    CASCADE_ENV_LOCK.get_or_init(|| std::sync::Mutex::new(()))
}

fn test_provider_runtime(
    config: &RokoConfig,
) -> (Arc<ProviderHealthRegistry>, Arc<ProviderRateLimiter>) {
    let health = Arc::new(ProviderHealthRegistry::new());
    let health_checker: Arc<dyn roko_agent::rate_limit::ProviderHealthChecker> =
        Arc::clone(&health) as Arc<_>;
    let providers = config.effective_providers();
    let limiter = Arc::new(
        ProviderRateLimiter::from_provider_configs(
            roko_core::defaults::DEFAULT_PROVIDER_RPM,
            providers.iter(),
        )
        .with_health_registry(health_checker),
    );
    (health, limiter)
}

/// cascade_select_model returns None when the ROKO_ACP_CASCADE_SELECT env
/// var is absent, regardless of whether a router file exists.
#[test]
fn cascade_select_model_returns_none_without_env_var() {
    let _guard = cascade_env_lock().lock().expect("acquire env lock");
    // SAFETY: serialized by cascade_env_lock; no other test holds the lock
    // and mutates ROKO_ACP_CASCADE_SELECT concurrently.
    unsafe { std::env::remove_var("ROKO_ACP_CASCADE_SELECT") };

    let tmp = tempfile::tempdir().expect("create tmpdir");
    let workdir = tmp.path();
    let config = RokoConfig::default();
    let (health, limiter) = test_provider_runtime(&config);
    let result = cascade_select_model(AcpCascadeRequest {
        workdir,
        roko_config: &config,
        mode: "code",
        prompt: "fix a bug",
        effort: "medium",
        resolved_slug: "claude-sonnet-4-6",
        model_selection_explicit: false,
        provider_health: &health,
        rate_limiter: &limiter,
    });
    assert!(
        result.is_none(),
        "should return None when env var is not set"
    );
}

/// cascade_select_model returns None when the env var is set but the router
/// state file does not yet exist (cold start).
#[test]
fn cascade_select_model_returns_none_without_router_file() {
    let _guard = cascade_env_lock().lock().expect("acquire env lock");
    let tmp = tempfile::tempdir().expect("create tmpdir");
    let workdir = tmp.path();
    // The router path will not exist in the fresh tmpdir.
    let router_path = workdir
        .join(".roko")
        .join("learn")
        .join("cascade-router.json");
    assert!(!router_path.exists(), "router file should not exist yet");

    // SAFETY: serialized by cascade_env_lock; no other test holds the lock
    // and mutates ROKO_ACP_CASCADE_SELECT concurrently.
    unsafe { std::env::set_var("ROKO_ACP_CASCADE_SELECT", "1") };
    let config = RokoConfig::default();
    let (health, limiter) = test_provider_runtime(&config);
    let result = cascade_select_model(AcpCascadeRequest {
        workdir,
        roko_config: &config,
        mode: "code",
        prompt: "fix a bug",
        effort: "medium",
        resolved_slug: "claude-sonnet-4-6",
        model_selection_explicit: false,
        provider_health: &health,
        rate_limiter: &limiter,
    });
    unsafe { std::env::remove_var("ROKO_ACP_CASCADE_SELECT") };

    assert!(
        result.is_none(),
        "should return None when router file is absent"
    );
}

#[test]
fn cascade_select_model_requires_exact_opt_in_value() {
    let _guard = cascade_env_lock().lock().expect("acquire env lock");
    let tmp = tempfile::tempdir().expect("create tmpdir");
    let router_dir = tmp.path().join(".roko").join("learn");
    std::fs::create_dir_all(&router_dir).expect("create router dir");
    CascadeRouter::new(vec!["model-a".to_owned()])
        .save(&router_dir.join("cascade-router.json"))
        .expect("save router state");

    // SAFETY: serialized by cascade_env_lock.
    unsafe { std::env::set_var("ROKO_ACP_CASCADE_SELECT", "0") };
    let config = RokoConfig::default();
    let (health, limiter) = test_provider_runtime(&config);
    let result = cascade_select_model(AcpCascadeRequest {
        workdir: tmp.path(),
        roko_config: &config,
        mode: "code",
        prompt: "fix a bug",
        effort: "medium",
        resolved_slug: "model-a",
        model_selection_explicit: false,
        provider_health: &health,
        rate_limiter: &limiter,
    });
    unsafe { std::env::remove_var("ROKO_ACP_CASCADE_SELECT") };

    assert!(result.is_none(), "presence alone must not enable routing");
}

/// cascade_select_model returns Some when the env var is set and a valid
/// router state file exists.  The returned slug must be one of the model
/// keys known to the router.
#[test]
fn cascade_select_model_returns_model_with_router() {
    use roko_learn::cascade_router::CascadeRouter;

    let _guard = cascade_env_lock().lock().expect("acquire env lock");
    let tmp = tempfile::tempdir().expect("create tmpdir");
    let workdir = tmp.path();
    let router_dir = workdir.join(".roko").join("learn");
    std::fs::create_dir_all(&router_dir).expect("create router dir");
    let router_path = router_dir.join("cascade-router.json");

    // Build a minimal router with one model slug and persist it.
    let model_key = "claude-sonnet-4-6".to_string();
    let router = CascadeRouter::new(vec![model_key.clone()]);
    router.save(&router_path).expect("save router state");

    // SAFETY: serialized by cascade_env_lock; no other test holds the lock
    // and mutates ROKO_ACP_CASCADE_SELECT concurrently.
    unsafe { std::env::set_var("ROKO_ACP_CASCADE_SELECT", "1") };
    let config = RokoConfig::default();
    let (health, limiter) = test_provider_runtime(&config);
    let result = cascade_select_model(AcpCascadeRequest {
        workdir,
        roko_config: &config,
        mode: "code",
        prompt: "add unit tests",
        effort: "medium",
        resolved_slug: &model_key,
        model_selection_explicit: false,
        provider_health: &health,
        rate_limiter: &limiter,
    });
    unsafe { std::env::remove_var("ROKO_ACP_CASCADE_SELECT") };

    assert!(
        result.is_some(),
        "should return Some when router file exists"
    );
    let result = result.expect("selection");
    assert_eq!(result.model_key, model_key);
    assert_eq!(result.stage, "static");
}

#[tokio::test]
async fn rate_limit_provider_selection_prefers_healthy_capacity_and_honors_explicit_model() {
    let config = RokoConfig::from_toml(
        r#"
[agent]
default_model = "model-a"

[providers.provider-a]
kind = "openai_compat"
base_url = "https://a.example/v1"
[providers.provider-a.limits]
rpm = 1
tpm = 1000

[providers.provider-b]
kind = "openai_compat"
base_url = "https://b.example/v1"
[providers.provider-b.limits]
rpm = 100
tpm = 100000

[models.model-a]
provider = "provider-a"
slug = "wire-a"
context_window = 8192

[models.model-b]
provider = "provider-b"
slug = "wire-b"
context_window = 8192
"#,
    )
    .expect("parse provider selection config");
    let tmp = tempfile::tempdir().expect("create tmpdir");
    let router_dir = tmp.path().join(".roko/learn");
    std::fs::create_dir_all(&router_dir).expect("create router dir");
    CascadeRouter::new(vec!["model-a".to_owned(), "model-b".to_owned()])
        .save(&router_dir.join("cascade-router.json"))
        .expect("save router state");
    let (health, limiter) = test_provider_runtime(&config);

    // Consume provider-a's one-RPM configured window. ACP selection reads
    // this canonical limiter snapshot and retains provider-b as capacity.
    limiter.acquire("provider-a").await;

    let _guard = cascade_env_lock().lock().expect("acquire env lock");
    // SAFETY: serialized with every other cascade env mutation in this module.
    unsafe { std::env::set_var("ROKO_ACP_CASCADE_SELECT", "1") };
    let automatic = cascade_select_model(AcpCascadeRequest {
        workdir: tmp.path(),
        roko_config: &config,
        mode: "code",
        prompt: "fix a provider issue",
        effort: "medium",
        resolved_slug: "model-a",
        model_selection_explicit: false,
        provider_health: &health,
        rate_limiter: &limiter,
    })
    .expect("automatic adaptive selection");
    let explicit = cascade_select_model(AcpCascadeRequest {
        workdir: tmp.path(),
        roko_config: &config,
        mode: "code",
        prompt: "fix a provider issue",
        effort: "medium",
        resolved_slug: "model-a",
        model_selection_explicit: true,
        provider_health: &health,
        rate_limiter: &limiter,
    });
    let (degraded_health, fresh_limiter) = test_provider_runtime(&config);
    for _ in 0..3 {
        degraded_health.record_failure(
            "provider-a",
            roko_learn::provider_health::ErrorClass::RateLimit,
        );
    }
    let health_aware = cascade_select_model(AcpCascadeRequest {
        workdir: tmp.path(),
        roko_config: &config,
        mode: "code",
        prompt: "fix a provider issue",
        effort: "medium",
        resolved_slug: "model-a",
        model_selection_explicit: false,
        provider_health: &degraded_health,
        rate_limiter: &fresh_limiter,
    })
    .expect("health-aware adaptive selection");
    unsafe { std::env::remove_var("ROKO_ACP_CASCADE_SELECT") };

    assert_eq!(automatic.model_key, "model-b");
    assert_eq!(health_aware.model_key, "model-b");
    assert!(
        explicit.is_none(),
        "explicit model selection must bypass adaptation"
    );
}

/// record_cascade_observation uses the config key (model_key_for_logging),
/// not the wire slug (resolved_for_logging.slug).  This test verifies the
/// exact-match path works: config keys that were registered with the router
/// are always found via model_index_for_slug regardless of family aliasing.
#[tokio::test]
async fn cascade_observation_updates_the_dispatched_config_key() {
    use roko_learn::cascade_router::CascadeRouter;

    let tmp = tempfile::tempdir().expect("create tmpdir");
    let workdir = tmp.path();
    let router_dir = workdir.join(".roko").join("learn");
    std::fs::create_dir_all(&router_dir).expect("create router dir");
    let router_path = router_dir.join("cascade-router.json");

    // Use a config key that has no slug_family alias so exact-match is the
    // only way it can be found.  A custom/short key like "my-model" will
    // not match any family heuristic.
    let config_key = "my-custom-model-key".to_string();
    let router = CascadeRouter::new(vec![config_key.clone()]);
    router.save(&router_path).expect("save initial router");

    record_cascade_observation(
        router_path.clone(),
        config_key.clone(),
        RoutingContext::default(),
        true,
        1_000,
        Some(250),
        vec![config_key.clone()],
    )
    .await
    .expect("observation task");

    let router_loaded = CascadeRouter::load_or_new(&router_path, vec![config_key.clone()]);
    let stats = router_loaded.observation_snapshot();
    assert_eq!(stats.get(&config_key).map(|entry| entry.trials), Some(1));
    assert_eq!(stats.get(&config_key).map(|entry| entry.successes), Some(1));
}

/// A failed dispatch is a trial without a success, and still reaches LinUCB
/// (bug-8da8ba).
#[tokio::test]
async fn cascade_observation_counts_failed_dispatch_as_failure() {
    use roko_learn::cascade_router::CascadeRouter;

    let tmp = tempfile::tempdir().expect("create tmpdir");
    let router_dir = tmp.path().join(".roko").join("learn");
    std::fs::create_dir_all(&router_dir).expect("create router dir");
    let router_path = router_dir.join("cascade-router.json");
    let config_key = "my-custom-model-key".to_string();

    record_cascade_observation(
        router_path.clone(),
        config_key.clone(),
        RoutingContext::default(),
        false,
        1_000,
        None,
        vec![config_key.clone()],
    )
    .await
    .expect("observation task");

    let router_loaded = CascadeRouter::load_or_new(&router_path, vec![config_key.clone()]);
    assert_eq!(router_loaded.total_observations(), 1);
    assert_eq!(
        router_loaded.confidence_snapshot().get(&config_key),
        Some(&(1, 0))
    );
}

// ── P2-ACP-3: client capability declaration ──────────────────────────────────

/// No capabilities declared → all tool flags must be false (safe text-only default).
/// This ensures a client that omits `clientCapabilities` cannot accidentally get
/// file/exec/network access.
#[test]
fn capability_negotiation_no_capabilities_yields_all_false() {
    let caps = derive_acp_tool_capabilities(
        "code",
        &ClientCapabilities::default(),
        false,
        &HashSet::new(),
    );
    assert!(
        !caps.read,
        "read must be false when no capabilities declared"
    );
    assert!(
        !caps.write,
        "write must be false when no capabilities declared"
    );
    assert!(
        !caps.exec,
        "exec must be false when no capabilities declared"
    );
    assert!(!caps.git, "git must be false when no capabilities declared");
    assert!(
        !caps.network,
        "network must be false when no capabilities declared"
    );
}

/// Declaring `fs.readTextFile = true` enables read access only; write remains
/// gated on `fs.writeTextFile`.
#[test]
fn capability_negotiation_read_only_fs_enables_read_not_write() {
    let client = ClientCapabilities {
        fs: Some(crate::types::FsCapabilities {
            read_text_file: true,
            write_text_file: false,
        }),
        terminal: None,
        mcp_servers: None,
    };
    let caps = derive_acp_tool_capabilities("code", &client, false, &HashSet::new());
    assert!(caps.read, "read_text_file=true must enable read");
    assert!(
        !caps.write,
        "write_text_file=false must keep write disabled"
    );
    assert!(!caps.exec, "exec must be false without terminal capability");
    assert!(!caps.git, "git must be false without terminal capability");
    assert!(!caps.network, "network must be false without mcp_servers");
}

/// Declaring both `fs.readTextFile` and `fs.writeTextFile` enables read + write.
#[test]
fn capability_negotiation_full_fs_enables_read_and_write() {
    let client = ClientCapabilities {
        fs: Some(crate::types::FsCapabilities {
            read_text_file: true,
            write_text_file: true,
        }),
        terminal: None,
        mcp_servers: None,
    };
    let caps = derive_acp_tool_capabilities("code", &client, false, &HashSet::new());
    assert!(caps.read, "read_text_file=true must enable read");
    assert!(caps.write, "write_text_file=true must enable write");
    assert!(!caps.exec, "exec must be false without terminal capability");
}

/// `terminal = true` enables exec access. Git access additionally requires the
/// role's permission ceiling to include git (only `MergeResolver` has that flag;
/// the `code` mode maps to `Implementer` which does not).
#[test]
fn capability_negotiation_terminal_enables_exec_not_git_for_implementer() {
    let client = ClientCapabilities {
        fs: None,
        terminal: Some(true),
        mcp_servers: None,
    };
    // "code" mode → Implementer role → role.git = false → git stays false even
    // with terminal capability declared.
    let caps = derive_acp_tool_capabilities("code", &client, false, &HashSet::new());
    assert!(caps.exec, "terminal=true must enable exec for Implementer");
    assert!(
        !caps.git,
        "git must be false for Implementer role (role.git = false)"
    );
    assert!(!caps.read, "read must still be false without fs capability");
}

/// `session/new` result includes `resolvedToolCapabilities` reflecting the
/// negotiated flags so the client does not need to re-derive them.
#[test]
fn session_new_result_includes_resolved_tool_capabilities() {
    // Session with full fs + terminal capabilities declared.
    let session = AcpSession::new(SessionNewParams {
        session_name: None,
        client_capabilities: Some(ClientCapabilities {
            fs: Some(crate::types::FsCapabilities {
                read_text_file: true,
                write_text_file: true,
            }),
            terminal: Some(true),
            mcp_servers: None,
        }),
        model: None,
        provider: None,
        effort: None,
        mcp_servers: Vec::new(),
    });
    let result = session.new_result();
    assert!(
        result.resolved_tool_capabilities.read,
        "read must be active"
    );
    assert!(
        result.resolved_tool_capabilities.write,
        "write must be active"
    );
    assert!(
        result.resolved_tool_capabilities.exec,
        "exec must be active"
    );
    // Default agent_mode is "code" (Implementer), whose role ceiling has git=false,
    // so git stays false regardless of the terminal capability.
    assert!(
        !result.resolved_tool_capabilities.git,
        "git must be false for Implementer role"
    );

    // Session with no capabilities declared → all-false safe defaults.
    let bare_session = AcpSession::new(SessionNewParams {
        session_name: None,
        client_capabilities: None,
        model: None,
        provider: None,
        effort: None,
        mcp_servers: Vec::new(),
    });
    let bare_result = bare_session.new_result();
    assert!(
        !bare_result.resolved_tool_capabilities.read,
        "no caps → read must be false"
    );
    assert!(
        !bare_result.resolved_tool_capabilities.write,
        "no caps → write must be false"
    );
    assert!(
        !bare_result.resolved_tool_capabilities.exec,
        "no caps → exec must be false"
    );
    assert!(
        !bare_result.resolved_tool_capabilities.git,
        "no caps → git must be false"
    );
    assert!(
        !bare_result.resolved_tool_capabilities.network,
        "no caps → network must be false"
    );
}

// ── P2-ACP-2: MCP crash resilience + tool matrix ─────────────────────────

/// MCP server that exits immediately after initialization — simulates a crash
/// during the tools/list phase. The session must not panic and must report a
/// failed status so callers can degrade gracefully.
#[tokio::test]
async fn mcp_server_crash_during_tools_list_produces_failed_status() {
    // The server sends the initialize response then exits, so the tools/list
    // request returns EOF on the client side (ToolsListFailed or similar).
    let server = r#"
        IFS= read -r initialize
        printf '%s\n' '{"jsonrpc":"2.0","id":1,"result":{"capabilities":{}}}'
        exit 0
    "#;
    let session = AcpSession::new(SessionNewParams {
        session_name: None,
        client_capabilities: Some(ClientCapabilities {
            mcp_servers: Some(true),
            ..Default::default()
        }),
        model: Some("crash-test".into()),
        provider: Some("anthropic".into()),
        effort: None,
        mcp_servers: vec![crate::types::McpServerConfig {
            name: "crash-server".into(),
            transport: crate::types::McpTransport::Stdio {
                command: "sh".into(),
                args: vec!["-c".into(), server.into()],
            },
            discovery_timeout_ms: Some(1_000),
        }],
    });
    let (event_sender, _event_receiver) = mpsc::channel(4);

    let (runtime, statuses) = setup_session_mcp_tools(
        &session.session_id,
        &session.mcp_servers,
        roko_agent::safety::capabilities::PluginTier::Sandboxed,
        event_sender,
    )
    .await;

    // A crash after initialize must produce a failed status — never a panic.
    assert_eq!(statuses.len(), 1, "exactly one status for one server");
    assert_ne!(
        statuses[0].status,
        crate::types::McpInitStatus::Ready,
        "crashed server must not report Ready"
    );
    assert!(
        runtime.tools.is_empty(),
        "no tools must be exposed from a crashed server"
    );
    assert!(
        runtime.handlers.is_empty(),
        "no handlers must be registered from a crashed server"
    );
}

/// MCP server that never responds — simulates a hang / freeze. The session
/// must time out and report a failed status rather than blocking indefinitely.
#[tokio::test]
async fn mcp_server_hang_during_initialize_times_out_gracefully() {
    // The server reads the initialize request but never responds.
    let server = r#"
        IFS= read -r initialize
        sleep 60
    "#;
    let session = AcpSession::new(SessionNewParams {
        session_name: None,
        client_capabilities: None,
        model: None,
        provider: None,
        effort: None,
        mcp_servers: vec![crate::types::McpServerConfig {
            name: "hang-server".into(),
            transport: crate::types::McpTransport::Stdio {
                command: "sh".into(),
                args: vec!["-c".into(), server.into()],
            },
            // Short timeout so the test stays fast.
            discovery_timeout_ms: Some(200),
        }],
    });
    let (event_sender, _event_receiver) = mpsc::channel(4);

    let start = std::time::Instant::now();
    let (runtime, statuses) = setup_session_mcp_tools(
        &session.session_id,
        &session.mcp_servers,
        roko_agent::safety::capabilities::PluginTier::Sandboxed,
        event_sender,
    )
    .await;
    let elapsed = start.elapsed();

    // Must resolve within a reasonable multiple of the configured timeout.
    assert!(
        elapsed.as_secs() < 5,
        "session setup must time out quickly, elapsed: {elapsed:?}"
    );
    assert_eq!(statuses.len(), 1);
    assert!(
        matches!(
            statuses[0].status,
            crate::types::McpInitStatus::InitializeTimeout
                | crate::types::McpInitStatus::InitializeFailed
        ),
        "hung server must report InitializeTimeout or InitializeFailed, got {:?}",
        statuses[0].status
    );
    assert!(runtime.tools.is_empty());
    assert!(runtime.handlers.is_empty());
}

/// Verify the builtin tool matrix: every tool exposed by `acp_builtin_tools()`
/// has a matching permission, a non-empty name, a non-empty description, and
/// the `derive_tool_permissions` function returns the expected capability flags.
#[test]
fn builtin_tool_matrix_permissions_and_definitions_are_correct() {
    use crate::builtin_tools::{
        acp_builtin_tools, compute_session_capabilities, derive_tool_permissions,
    };
    use roko_core::tool::ToolPermission;

    let tools = acp_builtin_tools();

    // Expected tool count: 9 (read_file, write_file, edit_file, glob, grep, bash, ls,
    // web_fetch, retrieve).  The RAG-16 `retrieve` tool was added after the initial 8.
    assert_eq!(
        tools.len(),
        9,
        "acp_builtin_tools must expose exactly 9 tools"
    );

    // Every tool must have a non-empty name and description.
    for tool in &tools {
        assert!(!tool.name.is_empty(), "tool must have a non-empty name");
        assert!(
            !tool.description.is_empty(),
            "tool '{}' must have a non-empty description",
            tool.name
        );
    }

    // Verify per-tool permission expectations.
    let cases: &[(&str, ToolPermission)] = &[
        ("read_file", ToolPermission::read_only()),
        ("glob", ToolPermission::read_only()),
        ("grep", ToolPermission::read_only()),
        ("ls", ToolPermission::read_only()),
        ("retrieve", ToolPermission::read_only()),
        ("write_file", ToolPermission::writes()),
        ("edit_file", ToolPermission::writes()),
        ("bash", ToolPermission::executes()),
        (
            "web_fetch",
            ToolPermission {
                read: true,
                write: false,
                exec: false,
                git: false,
                network: true,
            },
        ),
    ];

    for (name, expected) in cases {
        let got = derive_tool_permissions(name);
        assert_eq!(
            got, *expected,
            "derive_tool_permissions('{}') mismatch: got {got:?}, expected {expected:?}",
            name
        );
    }

    // Unknown tool names must fail closed (all false).
    let unknown = derive_tool_permissions("totally-unknown-tool-xyz");
    assert_eq!(
        unknown,
        ToolPermission {
            read: false,
            write: false,
            exec: false,
            git: false,
            network: false
        },
        "unknown tool must return all-false (fail-closed)"
    );

    // Session capabilities = union of all tool permissions.
    let session_caps = compute_session_capabilities(&tools);
    assert!(session_caps.read, "session must have read capability");
    assert!(session_caps.write, "session must have write capability");
    assert!(session_caps.exec, "session must have exec capability");
    assert!(session_caps.network, "session must have network capability");
}

/// Session setup with zero configured MCP servers must return an empty runtime
/// without panicking — nothing to do is a valid (non-error) outcome.
#[tokio::test]
async fn mcp_setup_with_no_servers_returns_empty_runtime() {
    let (event_sender, _event_receiver) = mpsc::channel(4);

    let (runtime, statuses) = setup_session_mcp_tools(
        "empty-session",
        &[],
        roko_agent::safety::capabilities::PluginTier::Sandboxed,
        event_sender,
    )
    .await;

    assert!(statuses.is_empty(), "no servers → no statuses");
    assert!(runtime.tools.is_empty(), "no servers → no tools");
    assert!(runtime.handlers.is_empty(), "no servers → no handlers");
}

/// MCP server that immediately exits without sending any data — simulates a
/// spawn-level failure (e.g. script returns 1 without output). The session
/// must not panic and must produce a failed status.
#[tokio::test]
async fn mcp_server_immediate_exit_without_response_reports_failed_status() {
    let server = r#"exit 1"#;
    let (event_sender, _event_receiver) = mpsc::channel(4);

    let (runtime, statuses) = setup_session_mcp_tools(
        "immediate-exit-session",
        &[crate::types::McpServerConfig {
            name: "exit-server".into(),
            transport: crate::types::McpTransport::Stdio {
                command: "sh".into(),
                args: vec!["-c".into(), server.into()],
            },
            discovery_timeout_ms: Some(1_000),
        }],
        roko_agent::safety::capabilities::PluginTier::Sandboxed,
        event_sender,
    )
    .await;

    assert_eq!(statuses.len(), 1);
    assert_ne!(
        statuses[0].status,
        crate::types::McpInitStatus::Ready,
        "server that exits without a response must not be Ready"
    );
    assert!(runtime.tools.is_empty());
}

/// `compute_session_capabilities` with an empty tool list must return all-false
/// (fail-closed), matching the documented contract.
#[test]
fn compute_session_capabilities_empty_tool_list_is_fail_closed() {
    use crate::builtin_tools::compute_session_capabilities;
    use roko_core::tool::ToolPermission;

    let caps = compute_session_capabilities(&[]);
    assert_eq!(
        caps,
        ToolPermission {
            read: false,
            write: false,
            exec: false,
            git: false,
            network: false
        },
        "empty tool list must produce all-false capabilities"
    );
}
