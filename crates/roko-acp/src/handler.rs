//! Main ACP dispatch loop.

use std::collections::VecDeque;
use std::future::Future;
use std::io::Write as _;
use std::path::Path;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll};

use anyhow::{Context, Result, anyhow};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::task::{JoinError, JoinHandle};
use tracing::{debug, error, info, warn};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;

use crate::{
    bridge_events::{BridgeEventsError, run_begun_prompt},
    config::AcpConfig,
    config_watch::ConfigWatcher,
    session::{AcpSession, CancelToken, SessionManager},
    transport::{StdioTransport, TransportError},
    types::{
        ACP_PROTOCOL_VERSION, ACP_SPEC_VERSION, AgentCapabilities, AgentInfo, ConfigUpdateParams,
        ConfigUpdateResult, INVALID_PARAMS, InitializeParams, InitializeResult, JsonRpcId,
        JsonRpcMessage, JsonRpcNotification, JsonRpcRequest, METHOD_NOT_FOUND, McpCapabilities,
        PARSE_ERROR, SESSION_NOT_FOUND, SessionCancelParams, SessionCloseParams, SessionLoadParams,
        SessionNewParams, SessionPromptParams, SessionSetModeParams,
        advertised_prompt_capabilities_for_model,
    },
};
use roko_core::agent::resolve_model;

/// Runs the ACP stdio server until stdin reaches EOF or a fatal transport error occurs.
pub async fn run_acp_server(config: AcpConfig) -> Result<()> {
    match run_acp_server_inner(config).await {
        Ok(()) => Ok(()),
        Err(e) => {
            // Send JSON-RPC error on stdout so the editor (e.g. Zed) can display it
            // instead of showing a silent "server shut down unexpectedly" message.
            // Not when stdout stopped taking writes: this write would block forever.
            let stdout_stalled = e.chain().any(|cause| {
                matches!(
                    cause.downcast_ref::<TransportError>(),
                    Some(TransportError::WriteTimeout { .. })
                )
            });
            if !stdout_stalled {
                let error_response = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": {
                        "code": -32603,
                        "message": format!("ACP server failed to start: {e:#}")
                    }
                });
                let _ = writeln!(std::io::stdout(), "{error_response}");
            }
            Err(e)
        }
    }
}

async fn run_acp_server_inner(config: AcpConfig) -> Result<()> {
    // Ensure .roko/ workspace directory exists before any file operations.
    let workdir = config
        .workdir
        .canonicalize()
        .unwrap_or_else(|_| config.workdir.clone());
    let roko_dir = workdir.join(".roko");
    if let Err(e) = std::fs::create_dir_all(&roko_dir) {
        // Non-fatal — we'll fall back to /tmp for logging.
        warn!("cannot create .roko/: {e}");
    }
    // Register the agents this server spawns under its workspace, as the CLI
    // does for its other commands; otherwise they land under the current
    // directory, where registry cleanup for the workspace cannot find them.
    roko_agent::process::set_registry_root(&workdir);

    let _guard = setup_file_logging(config.log_file())
        .or_else(|e| {
            // Fallback: log to /tmp if .roko/ is unavailable.
            let fallback =
                std::env::temp_dir().join(format!("roko-acp-{}.log", std::process::id()));
            warn!("{e}, falling back to {}", fallback.display());
            setup_file_logging(&fallback)
        })
        .with_context(|| "failed to initialize ACP logging")?;

    let mut transport = StdioTransport::new();
    run_acp_server_with_transport(config, &mut transport).await
}

/// Runs the ACP server against an injected transport.
///
/// This loop is the only stdin reader. Each `session/prompt` runs as its own
/// task while the loop routes the client's responses and cancels to it and
/// answers other requests; a request for a session whose prompt is running
/// waits for that prompt.
pub async fn run_acp_server_with_transport<R, W>(
    config: AcpConfig,
    transport: &mut StdioTransport<R, W>,
) -> Result<()>
where
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin + Send + 'static,
{
    let (roko_config, config_load_warning) = config.load_roko_config_with_warning();
    if let Some(ref warning) = config_load_warning {
        warn!("{warning}");
    }
    if !config.workdir.join("roko.toml").is_file() {
        let global_note = match config.global_config_path.as_ref() {
            Some(path) if path.is_file() => {
                format!("using global config from {}", path.display())
            }
            Some(path) => {
                format!(
                    "--global-config was set to {} but that file does not exist; \
                     falling back to built-in defaults",
                    path.display()
                )
            }
            None => match roko_core::config::loader::global_config_path() {
                Some(implicit) if implicit.is_file() => {
                    format!("using implicit global config from {}", implicit.display())
                }
                _ => "no global config found; using built-in defaults only. \
                     Run `roko init` in this directory or pass --global-config ~/.roko/config.toml"
                    .to_owned(),
            },
        };
        warn!(
            workdir = %config.workdir.display(),
            "no roko.toml found in ACP workdir — {global_note}. \
             To configure: add roko.toml to the project root, \
             or set --global-config in your editor's ACP settings."
        );
    }
    info!(
        providers = roko_config.providers.len(),
        models = roko_config.models.len(),
        "loaded roko.toml configuration"
    );
    let provider_warning = check_provider_readiness(&roko_config);
    if let Some(ref warning) = provider_warning {
        warn!("{warning}");
    }

    // Collect all startup warnings for the initialize response.
    let startup_warnings: Vec<String> = [config_load_warning, provider_warning]
        .into_iter()
        .flatten()
        .collect();

    let mut sessions = SessionManager::new(config.workdir.clone(), roko_config);
    sessions.config_sources = config.config_sources();
    sessions.startup_warnings = startup_warnings;
    let mut config_watcher = ConfigWatcher::start(&config);

    // GC old persisted sessions at startup (7 days).
    sessions.gc_old_sessions(chrono::Duration::days(7));

    // Requests for a session whose prompt is running, in arrival order.
    let mut deferred: VecDeque<JsonRpcRequest> = VecDeque::new();
    let mut running: Vec<RunningPrompt> = Vec::new();
    loop {
        let ready = deferred
            .iter()
            .position(|request| !is_for_running_prompt(request, &running));
        if let Some(request) = ready.and_then(|position| deferred.remove(position)) {
            dispatch_request(
                transport,
                &mut sessions,
                &mut running,
                &mut deferred,
                request,
            )
            .await?;
            continue;
        }
        let inbound = tokio::select! {
            (prompt, outcome) = std::future::poll_fn(|cx| poll_finished(&mut running, cx)),
                if !running.is_empty() =>
            {
                finish_prompt(transport, &mut sessions, prompt, outcome).await?;
                deferred.extend(sessions.drain_deferred_requests());
                continue;
            }
            inbound = transport.read_message() => inbound,
        };
        let message = match inbound {
            Ok(Some(message)) => message,
            Ok(None) => {
                info!("stdin reached EOF; shutting down ACP server");
                // The client is gone: stop its prompts and let them settle.
                for prompt in &running {
                    prompt.cancel.cancel();
                }
                while !running.is_empty() {
                    let (prompt, outcome) =
                        std::future::poll_fn(|cx| poll_finished(&mut running, cx)).await;
                    let delivered = finish_prompt(transport, &mut sessions, prompt, outcome).await;
                    if let Err(error) = delivered {
                        debug!(error = %error, "prompt result not delivered after EOF");
                    }
                }
                return Ok(());
            }
            Err(TransportError::Json(error)) => {
                error!(error = %error, "failed to decode inbound JSON-RPC message");
                transport
                    .send_error(
                        JsonRpcId::Null,
                        PARSE_ERROR,
                        format!("failed to parse JSON-RPC message: {error}"),
                    )
                    .await
                    .context("failed to send JSON-RPC parse error response")?;
                continue;
            }
            Err(error) => return Err(error).context("failed to read ACP message"),
        };

        match message {
            JsonRpcMessage::Request(request) => {
                if config_watcher.changed() {
                    let (refreshed, reload_warning) = config.load_roko_config_with_warning();
                    if let Some(warning) = &reload_warning {
                        warn!("config reload warning: {warning}");
                    }
                    info!(
                        providers = refreshed.providers.len(),
                        models = refreshed.models.len(),
                        "ACP config changed; reloaded roko.toml"
                    );
                    let reload_provider_warning = check_provider_readiness(&refreshed);
                    if let Some(ref warning) = reload_provider_warning {
                        warn!("config reload: {warning}");
                    }
                    sessions.startup_warnings = [reload_warning, reload_provider_warning]
                        .into_iter()
                        .flatten()
                        .collect();
                    sessions.replace_roko_config(refreshed);

                    // Recompute configSources and notify the IDE if they changed.
                    let new_sources = config.config_sources();
                    let sources_changed = sessions.config_sources != new_sources;
                    sessions.config_sources = new_sources;

                    if sources_changed
                        && let Err(e) =
                            send_config_sources_notification(transport, &sessions.config_sources)
                                .await
                    {
                        warn!(
                            error = %e,
                            "failed to push configSources reload notification to IDE"
                        );
                    }

                    // Notify the IDE about updated config options for each
                    // active session so dropdowns reflect the new state.
                    let session_snapshots: Vec<(String, Vec<crate::types::ConfigOption>)> =
                        sessions.active_session_config_options();
                    for (session_id, options) in session_snapshots {
                        let options_value = serde_json::to_value(&options)
                            .unwrap_or_else(|_| serde_json::json!([]));
                        if let Err(e) =
                            send_config_options_notification(transport, &session_id, options_value)
                                .await
                        {
                            warn!(
                                session_id = %session_id,
                                error = %e,
                                "failed to push config reload notification to IDE"
                            );
                        }
                    }
                }
                dispatch_request(
                    transport,
                    &mut sessions,
                    &mut running,
                    &mut deferred,
                    request,
                )
                .await?;
            }
            JsonRpcMessage::Response(response) => {
                transport.handle_incoming_response(response);
            }
            JsonRpcMessage::Notification(notification) => {
                route_notification(&mut sessions, &running, notification);
            }
        }
    }
}

/// A `session/prompt` running as its own task. Its session is out of the
/// manager until the task hands it back.
struct RunningPrompt {
    session_id: String,
    request_id: JsonRpcId,
    cancel: CancelToken,
    task: JoinHandle<FinishedPrompt>,
}

/// What a prompt task hands back: its session and the prompt's outcome.
type FinishedPrompt = (
    AcpSession,
    crate::bridge_events::Result<crate::types::SessionPromptResult>,
);

/// Polls the running prompt tasks and takes out the first one that finished.
fn poll_finished(
    running: &mut Vec<RunningPrompt>,
    cx: &mut TaskContext<'_>,
) -> Poll<(RunningPrompt, std::result::Result<FinishedPrompt, JoinError>)> {
    let finished = running
        .iter_mut()
        .enumerate()
        .find_map(|(index, prompt)| match Pin::new(&mut prompt.task).poll(cx) {
            Poll::Ready(outcome) => Some((index, outcome)),
            Poll::Pending => None,
        });
    match finished {
        Some((index, outcome)) => Poll::Ready((running.swap_remove(index), outcome)),
        None => Poll::Pending,
    }
}

/// The session a request or notification names, if any.
fn session_id_param(params: Option<&serde_json::Value>) -> Option<&str> {
    params?.get("sessionId")?.as_str()
}

fn is_for_running_prompt(request: &JsonRpcRequest, running: &[RunningPrompt]) -> bool {
    session_id_param(request.params.as_ref())
        .is_some_and(|id| running.iter().any(|prompt| prompt.session_id == id))
}

/// Handles one request beside the running prompts. A prompt starts as its own
/// task; a request for a session whose prompt is running waits for it.
async fn dispatch_request<R, W>(
    transport: &mut StdioTransport<R, W>,
    sessions: &mut SessionManager,
    running: &mut Vec<RunningPrompt>,
    deferred: &mut VecDeque<JsonRpcRequest>,
    request: JsonRpcRequest,
) -> Result<()>
where
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin + Send + 'static,
{
    if is_for_running_prompt(&request, running) {
        debug!(
            method = %request.method,
            "holding a request until its session's prompt finishes"
        );
        deferred.push_back(request);
        return Ok(());
    }
    if request.method == "session/prompt" {
        return start_prompt(transport, sessions, running, request).await;
    }
    handle_request(transport, sessions, request).await?;
    deferred.extend(sessions.drain_deferred_requests());
    Ok(())
}

/// Starts a `session/prompt` as its own task, which owns the session until it
/// finishes. The request loop keeps reading stdin and routes the client's
/// messages to the task.
async fn start_prompt<R, W>(
    transport: &mut StdioTransport<R, W>,
    sessions: &mut SessionManager,
    running: &mut Vec<RunningPrompt>,
    request: JsonRpcRequest,
) -> Result<()>
where
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin + Send + 'static,
{
    let JsonRpcRequest {
        id, method, params, ..
    } = request;
    let params: SessionPromptParams = match parse_params(params, &method) {
        Ok(params) => params,
        Err(error) => return send_error_response(transport, id, error).await,
    };
    let Some(mut session) = sessions.take_session(&params.session_id) else {
        return send_error_response(transport, id, session_not_found_error(&params.session_id))
            .await;
    };
    let workdir = sessions.workdir.clone();
    let roko_config = sessions.roko_config.clone();
    session.ensure_provider_runtime(&workdir, &roko_config);
    if !session.try_begin_prompt() {
        let busy = BridgeEventsError::SessionBusy(session.session_id.clone());
        sessions.insert_session(session);
        let error = busy
            .rpc_error()
            .unwrap_or_else(|| json_rpc_error(crate::types::INTERNAL_ERROR, busy.to_string()));
        return send_error_response(transport, id, error).await;
    }
    let session_id = session.session_id.clone();
    let cancel = session.cancel_token.clone();
    session.inbound_routed = true;
    let mut prompt_transport = transport.clone();
    let task = tokio::spawn(async move {
        let outcome = run_begun_prompt(
            &mut prompt_transport,
            &mut session,
            params,
            &workdir,
            &roko_config,
        )
        .await;
        session.inbound_routed = false;
        (session, outcome)
    });
    running.push(RunningPrompt {
        session_id,
        request_id: id,
        cancel,
        task,
    });
    Ok(())
}

/// Puts a finished prompt's session back and answers its request.
async fn finish_prompt<R, W>(
    transport: &mut StdioTransport<R, W>,
    sessions: &mut SessionManager,
    prompt: RunningPrompt,
    outcome: std::result::Result<FinishedPrompt, JoinError>,
) -> Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let (session, outcome) = match outcome {
        Ok(finished) => finished,
        Err(join_error) => {
            error!(
                session_id = %prompt.session_id,
                error = %join_error,
                "prompt task failed; its session is lost"
            );
            sessions.forget_taken_session(&prompt.session_id);
            let error = json_rpc_error(
                crate::types::INTERNAL_ERROR,
                format!("prompt task failed: {join_error}"),
            );
            return send_error_response(transport, prompt.request_id, error).await;
        }
    };
    let reloaded = sessions.insert_session(session);
    // Persist even when a post-dispatch transport/task error occurs: a
    // completed provider call may already have accrued billable cost.
    sessions.persist_session(&prompt.session_id);
    // A config reload while the prompt ran could not notify this session.
    if reloaded {
        let options = sessions
            .get_session(&prompt.session_id)
            .map(AcpSession::config_options)
            .unwrap_or_default();
        let options = serde_json::to_value(options).unwrap_or_else(|_| serde_json::json!([]));
        if let Err(e) =
            send_config_options_notification(transport, &prompt.session_id, options).await
        {
            warn!(
                session_id = %prompt.session_id,
                error = %e,
                "failed to push config reload notification to IDE"
            );
        }
    }
    match outcome {
        Ok(result) => send_success(transport, prompt.request_id, result).await,
        Err(error) => {
            if let Some(rpc_error) = error.rpc_error() {
                return send_error_response(transport, prompt.request_id, rpc_error).await;
            }
            Err(error).context("failed to handle ACP session prompt")
        }
    }
}

/// Routes a notification: a cancel for a session whose prompt is running goes
/// to that prompt, and the rest to the session manager.
fn route_notification(
    sessions: &mut SessionManager,
    running: &[RunningPrompt],
    notification: JsonRpcNotification,
) {
    if notification.method == "session/cancel"
        && let Some(id) = session_id_param(notification.params.as_ref())
        && let Some(prompt) = running.iter().find(|prompt| prompt.session_id == id)
    {
        prompt.cancel.cancel();
        return;
    }
    handle_notification(sessions, notification);
}

/// Returns a human-readable warning string if no configured provider has credentials,
/// or `None` if at least one provider appears ready for dispatch.
fn check_provider_readiness(config: &roko_core::config::schema::RokoConfig) -> Option<String> {
    use roko_core::ProviderKind;

    if config.providers.is_empty() {
        return Some(
            "no providers configured in roko.toml \u{2014} agent dispatch will fail; \
             set ANTHROPIC_API_KEY or install the claude CLI"
                .to_string(),
        );
    }
    for (_name, provider) in &config.providers {
        if provider.kind == ProviderKind::ClaudeCli
            || provider.kind == ProviderKind::Hermes
            || provider.kind == ProviderKind::OpenClaw
        {
            // CLI-based providers (ClaudeCli, Hermes, OpenClaw) don't need an API key env var.
            return None;
        }
        if let Some(env_var) = &provider.api_key_env
            && std::env::var(env_var)
                .ok()
                .filter(|k| !k.is_empty())
                .is_some()
        {
            return None;
        }
    }
    Some(
        "no provider has resolvable credentials \u{2014} check api_key_env vars in roko.toml \
         or set ANTHROPIC_API_KEY"
            .to_string(),
    )
}

async fn handle_request(
    transport: &mut StdioTransport<impl AsyncRead + Unpin, impl AsyncWrite + Unpin>,
    sessions: &mut SessionManager,
    request: JsonRpcRequest,
) -> Result<()> {
    let JsonRpcRequest {
        id, method, params, ..
    } = request;
    debug!(method = %method, request_id = ?id, "handling ACP request");

    match method.as_str() {
        "initialize" => {
            let _params: InitializeParams = match parse_params(params, &method) {
                Ok(params) => params,
                Err(error) => return send_error_response(transport, id, error).await,
            };
            // Determine image capability from default model's vision support
            let model_key = &sessions.roko_config.agent.default_model;
            let resolved = resolve_model(&sessions.roko_config, model_key);
            let prompt_capabilities = advertised_prompt_capabilities_for_model(
                resolved.provider_kind,
                resolved
                    .profile
                    .is_some_and(|profile| profile.supports_vision),
            );

            let result = InitializeResult {
                protocol_version: ACP_PROTOCOL_VERSION,
                agent_capabilities: AgentCapabilities {
                    load_session: true,
                    prompt_capabilities,
                    mcp_capabilities: McpCapabilities {
                        http: true,
                        sse: true,
                    },
                },
                auth_methods: Vec::new(),
                agent_info: Some(AgentInfo {
                    name: "roko".to_owned(),
                    version: env!("CARGO_PKG_VERSION").to_owned(),
                    title: Some("Roko".to_owned()),
                }),
                config_sources: sessions.config_sources.clone(),
                config_warnings: sessions.startup_warnings.clone(),
            };
            send_success(transport, id, result).await
        }
        "session/new" => {
            let params: SessionNewParams = match parse_params(params, &method) {
                Ok(params) => params,
                Err(error) => return send_error_response(transport, id, error).await,
            };
            let result = sessions.create_session(params);
            let session_id = result.session_id.clone();
            // Auto-detect bare mode from workspace; config `bare_mode = false`
            // is an explicit override to full mode.
            let config_override = if !sessions.roko_config.agent.bare_mode {
                Some(false)
            } else {
                None
            };
            let bare_mode = crate::session::resolve_bare_mode(config_override, &sessions.workdir);
            send_success(transport, id, result).await?;
            send_slash_commands_notification(transport, &session_id, bare_mode).await
        }
        "session/list" => {
            let result = sessions.list_sessions_with_persisted();
            send_success(transport, id, result).await
        }
        "session/load" => {
            let params: SessionLoadParams = match parse_params(params, &method) {
                Ok(params) => params,
                Err(error) => return send_error_response(transport, id, error).await,
            };
            let result = match sessions.load_session(&params.session_id) {
                Ok(result) => result,
                Err(_) => {
                    return send_error_response(
                        transport,
                        id,
                        session_not_found_error(&params.session_id),
                    )
                    .await;
                }
            };
            send_success(transport, id, result).await
        }
        "session/config/update" | "session/set_config_option" => {
            let params: ConfigUpdateParams = match parse_params(params, &method) {
                Ok(params) => params,
                Err(error) => return send_error_response(transport, id, error).await,
            };
            let roko_config = sessions.roko_config.clone();
            let session = match get_session_mut(sessions, &params.session_id) {
                Ok(session) => session,
                Err(error) => return send_error_response(transport, id, error).await,
            };
            debug!(
                session_id = %session.session_id,
                option_id = %params.option_id,
                new_value = %params.new_value,
                "received config update request"
            );
            if let Err(message) =
                session.update_config(&params.option_id, &params.new_value, &roko_config)
            {
                return send_error_response(transport, id, json_rpc_error(INVALID_PARAMS, message))
                    .await;
            }
            let result = ConfigUpdateResult {
                config_options: session.config_options(),
            };
            send_success(transport, id, result).await
        }
        "session/close" => {
            let params: SessionCloseParams = match parse_params(params, &method) {
                Ok(params) => params,
                Err(error) => return send_error_response(transport, id, error).await,
            };
            sessions.close_session(&params.session_id);
            send_success(transport, id, serde_json::json!({})).await
        }
        "session/resume" => {
            let params: SessionLoadParams = match parse_params(params, &method) {
                Ok(params) => params,
                Err(error) => return send_error_response(transport, id, error).await,
            };
            let result = match sessions.load_session(&params.session_id) {
                Ok(result) => result,
                Err(_) => {
                    return send_error_response(
                        transport,
                        id,
                        session_not_found_error(&params.session_id),
                    )
                    .await;
                }
            };
            let session_id = params.session_id.clone();
            let config_override = if !sessions.roko_config.agent.bare_mode {
                Some(false)
            } else {
                None
            };
            let bare_mode = crate::session::resolve_bare_mode(config_override, &sessions.workdir);
            send_success(transport, id, result).await?;
            send_slash_commands_notification(transport, &session_id, bare_mode).await?;
            if let Some(session) = sessions.get_session(&session_id) {
                let options = serde_json::to_value(session.config_options())
                    .unwrap_or_else(|_| serde_json::json!([]));
                send_config_options_notification(transport, &session_id, options).await?;
            }
            Ok(())
        }
        "session/set_mode" => {
            let params: SessionSetModeParams = match parse_params(params, &method) {
                Ok(params) => params,
                Err(error) => return send_error_response(transport, id, error).await,
            };
            let session = match get_session_mut(sessions, &params.session_id) {
                Ok(session) => session,
                Err(error) => return send_error_response(transport, id, error).await,
            };
            session.set_mode(params.mode_id);
            let result = ConfigUpdateResult {
                config_options: session.config_options(),
            };
            send_success(transport, id, result).await
        }
        _ => {
            let error = json_rpc_error(
                METHOD_NOT_FOUND,
                format!("method '{method}' is not supported"),
            );
            send_error_response(transport, id, error).await
        }
    }
}

fn handle_notification(sessions: &mut SessionManager, notification: JsonRpcNotification) {
    debug!(method = %notification.method, "handling ACP notification");

    match notification.method.as_str() {
        "session/cancel" => {
            let params: SessionCancelParams =
                match parse_params(notification.params, &notification.method) {
                    Ok(params) => params,
                    Err(error) => {
                        warn!(
                            method = %notification.method,
                            code = error.0,
                            message = %error.1,
                            "dropping malformed ACP notification"
                        );
                        return;
                    }
                };

            match sessions.get_session_mut(&params.session_id) {
                Some(session) => session.cancel(),
                None => {
                    warn!(session_id = %params.session_id, "received cancel for unknown ACP session")
                }
            }
        }
        _ => warn!(method = %notification.method, "ignoring unsupported ACP notification"),
    }
}

fn get_session_mut<'a>(
    sessions: &'a mut SessionManager,
    session_id: &str,
) -> std::result::Result<&'a mut crate::session::AcpSession, (i32, String)> {
    sessions
        .get_session_mut(session_id)
        .ok_or_else(|| session_not_found_error(session_id))
}

fn parse_params<T>(
    params: Option<serde_json::Value>,
    method: &str,
) -> std::result::Result<T, (i32, String)>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_value(params.unwrap_or(serde_json::Value::Null)).map_err(|error| {
        json_rpc_error(
            crate::types::INVALID_PARAMS,
            format!("invalid params for '{method}': {error}"),
        )
    })
}

async fn send_slash_commands_notification(
    transport: &mut StdioTransport<impl AsyncRead + Unpin, impl AsyncWrite + Unpin>,
    session_id: &str,
    bare_mode: bool,
) -> Result<()> {
    let commands = crate::session::build_slash_commands(bare_mode);
    let update = serde_json::json!({
        "sessionId": session_id,
        "update": {
            "sessionUpdate": "available_commands_update",
            "availableCommands": commands,
        }
    });
    transport
        .send_notification("session/update", update)
        .await
        .context("failed to send slash commands notification")
}

async fn send_config_options_notification(
    transport: &mut StdioTransport<impl AsyncRead + Unpin, impl AsyncWrite + Unpin>,
    session_id: &str,
    config_options: serde_json::Value,
) -> Result<()> {
    let update = serde_json::json!({
        "sessionId": session_id,
        "update": {
            "sessionUpdate": "config_option_update",
            "configOptions": config_options,
        }
    });
    transport
        .send_notification("session/update", update)
        .await
        .context("failed to send config options notification")
}

/// Notify the IDE that the set of active config files has changed.
///
/// This is a server-level notification (not session-scoped) sent when a
/// config file is created, removed, or modified and the effective source
/// list differs from the previous reload. The IDE can use this to update
/// any status-bar indicator or "configure roko" prompt.
async fn send_config_sources_notification(
    transport: &mut StdioTransport<impl AsyncRead + Unpin, impl AsyncWrite + Unpin>,
    config_sources: &[String],
) -> Result<()> {
    let params = serde_json::json!({
        "configSources": config_sources,
    });
    transport
        .send_notification("server/config_sources_update", params)
        .await
        .context("failed to send configSources update notification")
}

fn json_rpc_error(code: i32, message: String) -> (i32, String) {
    (code, message)
}

fn session_not_found_error(session_id: &str) -> (i32, String) {
    json_rpc_error(
        SESSION_NOT_FOUND,
        format!("session '{session_id}' was not found"),
    )
}

async fn send_success<T>(
    transport: &mut StdioTransport<impl AsyncRead + Unpin, impl AsyncWrite + Unpin>,
    id: crate::types::JsonRpcId,
    result: T,
) -> Result<()>
where
    T: serde::Serialize,
{
    let value = serde_json::to_value(result).context("failed to serialize JSON-RPC result")?;
    transport
        .send_response(id, value)
        .await
        .context("failed to send JSON-RPC response")
}

async fn send_error_response(
    transport: &mut StdioTransport<impl AsyncRead + Unpin, impl AsyncWrite + Unpin>,
    id: crate::types::JsonRpcId,
    error: (i32, String),
) -> Result<()> {
    transport
        .send_error(id, error.0, error.1)
        .await
        .context("failed to send JSON-RPC error response")
}

fn setup_file_logging(log_file: &Path) -> Result<WorkerGuard> {
    if let Some(parent) = log_file.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create ACP log directory {}", parent.display()))?;
    }

    let file_name = log_file
        .file_name()
        .ok_or_else(|| anyhow!("ACP log file path must include a file name"))?;
    let directory = log_file.parent().unwrap_or_else(|| Path::new("."));

    let file_appender = tracing_appender::rolling::never(directory, file_name);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(non_blocking)
        .with_env_filter(EnvFilter::new("roko_acp=debug"))
        .finish();

    let _ = tracing::subscriber::set_global_default(subscriber);
    info!(
        protocol_version = ACP_PROTOCOL_VERSION,
        spec_version = ACP_SPEC_VERSION,
        log_file = %log_file.display(),
        "ACP logging initialized"
    );

    Ok(guard)
}

#[cfg(test)]
mod tests {
    use tokio::io::{AsyncBufReadExt, BufReader, DuplexStream, duplex};

    use super::*;

    #[test]
    fn parse_params_reports_invalid_payloads() {
        let error = parse_params::<InitializeParams>(
            Some(serde_json::json!({ "protocolVersion": "wrong" })),
            "initialize",
        )
        .expect_err("payload should be rejected");

        assert_eq!(error.0, crate::types::INVALID_PARAMS);
        assert!(error.1.contains("initialize"));
    }

    async fn read_json_line(reader: &mut BufReader<DuplexStream>) -> serde_json::Value {
        let mut line = String::new();
        reader.read_line(&mut line).await.expect("read line");
        serde_json::from_str(&line).expect("parse line")
    }

    #[tokio::test]
    async fn bridge_under_load_answers_requests_while_a_prompt_runs() {
        let tmp = tempfile::tempdir().expect("create tmpdir");
        let mut sessions = SessionManager::new(
            tmp.path().to_path_buf(),
            roko_core::config::schema::RokoConfig::default(),
        );
        let new_session = |name: &str| SessionNewParams {
            session_name: Some(name.to_owned()),
            client_capabilities: None,
            model: None,
            provider: None,
            effort: None,
            mcp_servers: Vec::new(),
        };
        let busy = sessions.create_session(new_session("busy")).session_id;
        let idle = sessions.create_session(new_session("idle")).session_id;
        let (client, server) = duplex(64 * 1024);
        let (server_reader, server_writer) = tokio::io::split(server);
        let mut transport = StdioTransport::from_io(server_reader, server_writer);
        let mut reader = BufReader::new(client);

        // `busy` has a prompt running, which ends once it is cancelled.
        let busy_session = sessions.take_session(&busy).expect("busy session");
        let cancel = CancelToken::new();
        let prompt_cancel = cancel.clone();
        let task: JoinHandle<FinishedPrompt> = tokio::spawn(async move {
            prompt_cancel.cancelled().await;
            let result = crate::types::SessionPromptResult {
                stop_reason: crate::types::StopReason::Cancelled,
            };
            (busy_session, Ok(result))
        });
        let mut running = vec![RunningPrompt {
            session_id: busy.clone(),
            request_id: JsonRpcId::Number(1),
            cancel,
            task,
        }];
        let mut deferred = VecDeque::new();
        let request = |id: u64, method: &str, params: serde_json::Value| -> JsonRpcRequest {
            serde_json::from_value(serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": method,
                "params": params
            }))
            .expect("parse request")
        };
        let set_mode =
            |session_id: &str| serde_json::json!({ "sessionId": session_id, "modeId": "plan" });

        // A request for another session is answered while the prompt runs.
        let set_idle = request(2, "session/set_mode", set_mode(&idle));
        dispatch_request(
            &mut transport,
            &mut sessions,
            &mut running,
            &mut deferred,
            set_idle,
        )
        .await
        .expect("handle request");
        let response = read_json_line(&mut reader).await;
        assert_eq!(response["id"], serde_json::json!(2));
        assert!(response.get("error").is_none(), "got {response}");

        // So is a listing, which still shows the busy session.
        let list = request(3, "session/list", serde_json::json!({}));
        dispatch_request(
            &mut transport,
            &mut sessions,
            &mut running,
            &mut deferred,
            list,
        )
        .await
        .expect("handle list");
        let response = read_json_line(&mut reader).await;
        assert_eq!(response["id"], serde_json::json!(3));
        let listed = response["result"]["sessions"].as_array().expect("sessions");
        assert_eq!(listed.len(), 2, "got {response}");

        // A request for the busy session waits for its prompt.
        let set_busy = request(4, "session/set_mode", set_mode(&busy));
        dispatch_request(
            &mut transport,
            &mut sessions,
            &mut running,
            &mut deferred,
            set_busy,
        )
        .await
        .expect("hold request");
        assert_eq!(deferred.len(), 1);

        // The client's cancel reaches the prompt, which then finishes.
        let cancel_busy: JsonRpcNotification = serde_json::from_value(serde_json::json!({
            "jsonrpc": "2.0",
            "method": "session/cancel",
            "params": { "sessionId": busy }
        }))
        .expect("parse notification");
        route_notification(&mut sessions, &running, cancel_busy);
        let (prompt, outcome) = std::future::poll_fn(|cx| poll_finished(&mut running, cx)).await;
        finish_prompt(&mut transport, &mut sessions, prompt, outcome)
            .await
            .expect("finish prompt");
        let response = read_json_line(&mut reader).await;
        assert_eq!(response["id"], serde_json::json!(1));
        assert_eq!(response["result"]["stopReason"], "cancelled");
        assert!(sessions.get_session(&busy).is_some());

        // The held request goes next.
        let held = deferred.pop_front().expect("held request");
        assert!(!is_for_running_prompt(&held, &running));
        dispatch_request(
            &mut transport,
            &mut sessions,
            &mut running,
            &mut deferred,
            held,
        )
        .await
        .expect("handle held request");
        let response = read_json_line(&mut reader).await;
        assert_eq!(response["id"], serde_json::json!(4));
        assert!(response.get("error").is_none(), "got {response}");
    }
}
