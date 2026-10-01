//! Cognitive event to session/update streaming.
//!
//! Bridges Roko's provider system (via `roko-agent`) to ACP
//! `session/update` notifications.
//! All cognitive workflow dispatch now goes through
//! [`crate::runner::run_with_workflow_engine`], which uses `ModelCallService`
//! for provider-agnostic model calls.

pub mod context;
pub mod cost;
pub mod dispatch;
pub mod experiments;
mod helpers;
pub mod permissions;
pub mod protocol;
pub mod provenance;
pub mod slash_commands;
pub mod tools;

#[cfg(test)]
mod tests;

// ── Re-exports (preserve public API) ────────────────────────────────

pub(crate) use context::resolve_context_items;
pub(crate) use context::{
    extract_prompt_text, extract_resource_uris, inject_image_parts, model_input_blocks_from_prompt,
    model_input_messages_from_wire, read_file_context,
};
pub use cost::calculate_cost_for_model_slug;
pub(crate) use cost::{
    acp_dispatch_succeeded, acp_efficiency_event, acp_routing_context, append_acp_episode,
    derive_acp_tool_capabilities, emit_acp_efficiency_event, truncate_assistant_history,
    truncate_to_title,
};
pub(crate) use dispatch::{run_anthropic_cognitive_task, run_openai_compat_cognitive_task};
pub(crate) use experiments::{
    AcpCascadeRequest, applicable_acp_experiment, assign_acp_experiment,
    cascade_router_model_slugs, cascade_select_model, mark_acp_experiment_dispatched,
    record_acp_experiment_outcome, record_cascade_observation, render_experiment_context,
    replace_experiment_section, resolve_acp_dispatch_model,
};
pub(crate) use helpers::{
    dispatch_failure_update, emit_dispatch_failure, map_event_to_update, roko_meta_update,
    send_cognitive_event, send_session_update, workflow_template_name,
};
pub use permissions::request_permission;
pub(crate) use permissions::request_permission_for_event;
pub use protocol::{
    BridgeEventsError, CognitiveEvent, PermissionReplyChannel, PermissionRequestPayload, Result,
    StreamResult,
};
pub(crate) use provenance::{
    build_provenance, emit_knowledge_card, emit_provenance_card, render_provenance_card,
};
pub(crate) use slash_commands::run_slash_command;
pub(crate) use tools::write_session_mcp_config;

// ── Imports for this module ─────────────────────────────────────────

use std::{
    path::Path,
    sync::{Arc, Mutex},
    time::Instant,
};

use roko_agent::safety::{DispatchSafetyContext, SafetyLayer, ViolationSeverity};
use roko_core::agent::{ProviderKind, resolve_model};
use roko_core::config::schema::{ModelProfile, RokoConfig};
use roko_core::foundation::{MessageRole, ModelInputMessage, validate_model_input_messages};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::mpsc,
};
use tracing::{debug, error, info, warn};

use crate::event_forward::AcpEventForwarder;
use crate::knowledge::{DispatchKnowledge, append_context, query_dispatch_knowledge};
use crate::runner::run_with_workflow_engine;
use crate::{
    session::{AcpSession, CancelToken},
    transport::{StdioTransport, TransportResult},
    types::{
        ContentBlock, CostInfo, JsonRpcMessage, SessionCancelParams, SessionPromptParams,
        SessionPromptResult, SessionUpdate, StopReason, advertised_prompt_capabilities_for_model,
        unsupported_prompt_content,
    },
};

/// Knowledge-card + context helper module alias (used by slash_commands).
pub(crate) mod knowledge_helpers {
    pub(crate) use super::provenance::emit_knowledge_card;
    pub(crate) use crate::knowledge::query_dispatch_knowledge;
}

// ── Core entry points ───────────────────────────────────────────────

/// Maps cognitive events to ACP `session/update` notifications and streams them to the editor.
/// Returns both the prompt result and the accumulated assistant response text.
pub async fn stream_events_to_editor<R, W>(
    transport: &mut StdioTransport<R, W>,
    session_id: &str,
    session: &mut AcpSession,
    workdir: &Path,
    mut events: mpsc::Receiver<CognitiveEvent>,
    cancel_token: &CancelToken,
) -> Result<StreamResult>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut assistant_text = String::new();
    let event_forwarder = AcpEventForwarder::from_env(session_id);

    loop {
        enum StreamAction {
            Cancelled,
            Event(Option<CognitiveEvent>),
            Inbound(TransportResult<Option<JsonRpcMessage>>),
        }

        let action = tokio::select! {
            biased;
            _ = cancel_token.cancelled() => StreamAction::Cancelled,
            maybe_event = events.recv() => StreamAction::Event(maybe_event),
            inbound = transport.read_message() => StreamAction::Inbound(inbound),
        };

        match action {
            StreamAction::Cancelled => {
                debug!(session_id, "ACP prompt cancelled while streaming events");
                return Ok(StreamResult {
                    prompt_result: SessionPromptResult {
                        stop_reason: StopReason::Cancelled,
                    },
                    assistant_text,
                    usage: None,
                });
            }
            StreamAction::Event(maybe_event) => {
                let Some(event) = maybe_event else {
                    warn!(
                        session_id,
                        "ACP event stream closed without an explicit completion event"
                    );
                    let stop_reason = if cancel_token.is_cancelled() {
                        StopReason::Cancelled
                    } else {
                        StopReason::EndTurn
                    };
                    return Ok(StreamResult {
                        prompt_result: SessionPromptResult { stop_reason },
                        assistant_text,
                        usage: None,
                    });
                };

                if let Some(forwarder) = event_forwarder.as_ref() {
                    forwarder.forward(&event);
                }

                match event {
                    CognitiveEvent::Complete { stop_reason, usage } => {
                        return Ok(StreamResult {
                            prompt_result: SessionPromptResult { stop_reason },
                            assistant_text,
                            usage,
                        });
                    }
                    CognitiveEvent::Failure { message } => {
                        let update = dispatch_failure_update(message);
                        send_session_update(transport, session_id, update).await?;
                        return Ok(StreamResult {
                            prompt_result: SessionPromptResult {
                                stop_reason: StopReason::EndTurn,
                            },
                            assistant_text,
                            usage: None,
                        });
                    }
                    CognitiveEvent::MaxTokens => {
                        return Ok(StreamResult {
                            prompt_result: SessionPromptResult {
                                stop_reason: StopReason::MaxTokens,
                            },
                            assistant_text,
                            usage: None,
                        });
                    }
                    CognitiveEvent::PermissionRequest { payload, reply } => {
                        let decision = request_permission_for_event(
                            transport,
                            session,
                            workdir,
                            &payload,
                            &reply,
                            cancel_token,
                        )
                        .await;
                        if !reply.reply(decision) {
                            warn!(
                                session_id,
                                "permission requester disappeared before receiving the decision"
                            );
                        }
                    }
                    CognitiveEvent::TokenChunk(ref text) => {
                        assistant_text.push_str(text);
                        if let Some(update) = map_event_to_update(event) {
                            send_session_update(transport, session_id, update).await?;
                        }
                    }
                    other => {
                        if let Some(update) = map_event_to_update(other) {
                            send_session_update(transport, session_id, update).await?;
                        }
                    }
                }
            }
            StreamAction::Inbound(inbound) => match inbound? {
                Some(JsonRpcMessage::Notification(notification))
                    if notification.method == "session/cancel" =>
                {
                    match serde_json::from_value::<SessionCancelParams>(
                        notification.params.unwrap_or(serde_json::Value::Null),
                    ) {
                        Ok(params) if params.session_id == session_id => {
                            cancel_token.cancel();
                        }
                        Ok(_) => {}
                        Err(error) => {
                            warn!(
                                session_id,
                                error = %error,
                                "received malformed session/cancel while prompt was active"
                            );
                        }
                    }
                }
                Some(JsonRpcMessage::Notification(notification)) => {
                    warn!(
                        session_id,
                        method = %notification.method,
                        "ignoring unsupported notification while prompt was active"
                    );
                }
                Some(JsonRpcMessage::Response(response)) => {
                    transport.handle_incoming_response(response);
                }
                Some(JsonRpcMessage::Request(request)) => {
                    warn!(
                        session_id,
                        method = %request.method,
                        "ignoring inbound request while prompt was active"
                    );
                }
                None => {
                    warn!(
                        session_id,
                        "ACP client disconnected while prompt was active"
                    );
                    return Ok(StreamResult {
                        prompt_result: SessionPromptResult {
                            stop_reason: StopReason::Cancelled,
                        },
                        assistant_text,
                        usage: None,
                    });
                }
            },
        }
    }
}

// ── Session prompt entry point ───────────────────────────────────────

/// Handles a `session/prompt` request by running the cognitive task and streaming updates.
pub async fn handle_session_prompt<R, W>(
    transport: &mut StdioTransport<R, W>,
    session: &mut AcpSession,
    params: SessionPromptParams,
    workdir: &Path,
    roko_config: &RokoConfig,
) -> Result<SessionPromptResult>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    session.ensure_provider_runtime(workdir, roko_config);
    if !session.try_begin_prompt() {
        return Err(BridgeEventsError::SessionBusy(session.session_id.clone()));
    }

    let outcome =
        handle_session_prompt_inner(transport, session, params, workdir, roko_config).await;
    session.finish_prompt();
    outcome
}

async fn handle_session_prompt_inner<R, W>(
    transport: &mut StdioTransport<R, W>,
    session: &mut AcpSession,
    params: SessionPromptParams,
    workdir: &Path,
    roko_config: &RokoConfig,
) -> Result<SessionPromptResult>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let prompt_text = extract_prompt_text(&params.prompt);
    let prompt_has_images = params
        .prompt
        .iter()
        .any(|block| matches!(block, ContentBlock::Image { .. }));
    let model_key = session.config_state.model.clone();
    let is_slash_command = prompt_text.trim_start().starts_with('/');
    if !is_slash_command && session.cost_budget_exceeded() {
        return Err(BridgeEventsError::BudgetExceeded {
            cost_budget_usd: session.cost_budget_usd.unwrap_or_default(),
            accumulated_cost_usd: session.accumulated_cost_usd,
        });
    }
    let provider_health = session.provider_health_registry.as_ref().ok_or_else(|| {
        BridgeEventsError::Pipeline(anyhow::anyhow!(
            "provider health registry not initialized before prompt"
        ))
    })?;
    let provider_health = Arc::clone(provider_health);
    let provider_rate_limiter = session.provider_rate_limiter.as_ref().ok_or_else(|| {
        BridgeEventsError::Pipeline(anyhow::anyhow!(
            "provider rate limiter not initialized before prompt"
        ))
    })?;
    let provider_rate_limiter = Arc::clone(provider_rate_limiter);
    let experiment_path = workdir.join(".roko").join("learn").join("experiments.json");
    let experiment_assignment = if is_slash_command {
        None
    } else {
        // assign_acp_experiment acquires a std::sync::Mutex and reads from disk,
        // so run it on a blocking thread to avoid stalling the tokio runtime.
        let path = experiment_path.clone();
        let mode = session.config_state.agent_mode.clone();
        let sid = session.session_id.clone();
        tokio::task::spawn_blocking(move || assign_acp_experiment(&path, &mode, &sid))
            .await
            .unwrap_or(None)
    };
    let (experiment_assignment, experiment_model_key) = applicable_acp_experiment(
        roko_config,
        &model_key,
        session.config_state.model_selection_explicit,
        experiment_assignment,
    );
    let routing_model_key = experiment_model_key
        .clone()
        .unwrap_or_else(|| model_key.clone());
    let requested_resolved = resolve_model(roko_config, &routing_model_key);

    // Capture workflow config before model selection: cascade routing owns only
    // direct single-agent prompts. Slash commands and workflow pipelines retain
    // their explicitly selected/configured model behavior.
    let workflow_config = session.config_state.workflow.clone();
    let pipeline_template = if workflow_config == "auto" {
        Some(crate::pipeline::WorkflowTemplate::auto_select(&prompt_text))
    } else {
        crate::pipeline::WorkflowTemplate::from_config(&workflow_config)
    };

    let cascade_candidate = if !is_slash_command
        && pipeline_template.is_none()
        && !prompt_has_images
        && !session.config_state.model_selection_explicit
        && experiment_model_key.is_none()
    {
        cascade_select_model(AcpCascadeRequest {
            workdir,
            roko_config,
            mode: &session.config_state.agent_mode,
            prompt: &prompt_text,
            effort: &session.config_state.effort,
            resolved_slug: &requested_resolved.slug,
            model_selection_explicit: session.config_state.model_selection_explicit,
            provider_health: &provider_health,
            rate_limiter: &provider_rate_limiter,
        })
    } else {
        None
    };
    let (resolved, model_key_for_dispatch, cascade_selection) =
        resolve_acp_dispatch_model(roko_config, &routing_model_key, cascade_candidate);

    if let Some(assignment) = experiment_assignment.as_ref() {
        info!(
            experiment_id = %assignment.experiment_id,
            variant_id = %assignment.variant_id,
            section = %assignment.section_name,
            model_override = ?experiment_model_key,
            "assigned ACP experiment variant"
        );
    }

    let pipeline_accepts_images =
        pipeline_template.is_none() || std::env::var_os("ROKO_ACP_LEGACY").is_none();
    let prompt_capabilities = advertised_prompt_capabilities_for_model(
        resolved.provider_kind,
        !is_slash_command
            && pipeline_accepts_images
            && resolved
                .profile
                .as_ref()
                .is_some_and(|profile| profile.supports_vision),
    );
    if let Some(message) = unsupported_prompt_content(&params.prompt, &prompt_capabilities) {
        if let Some(assignment) = experiment_assignment.as_ref()
            && let Err(error) = record_acp_experiment_outcome(&experiment_path, assignment, false)
        {
            warn!(
                experiment_id = %assignment.experiment_id,
                variant_id = %assignment.variant_id,
                error = %error,
                "failed to persist rejected ACP experiment outcome"
            );
        }
        return Err(BridgeEventsError::UnsupportedPromptContent(
            message.to_string(),
        ));
    }

    if prompt_has_images {
        let probe = vec![ModelInputMessage::new(
            MessageRole::User,
            model_input_blocks_from_prompt(&params.prompt),
        )];
        validate_model_input_messages(&probe).map_err(|error| {
            BridgeEventsError::UnsupportedPromptContent(format!("invalid image input: {error}"))
        })?;
    }

    if let Some(selection) = cascade_selection.as_ref() {
        if model_key_for_dispatch == requested_resolved.model_key {
            debug!(
                requested_model = %model_key,
                selected_model = %model_key_for_dispatch,
                stage = %selection.stage,
                "cascade router retained requested model for ACP dispatch"
            );
        } else {
            info!(
                requested_model = %model_key,
                selected_model = %model_key_for_dispatch,
                stage = %selection.stage,
                reason = "adaptive cascade selection",
                "cascade router overriding model for ACP dispatch"
            );
        }
    }

    let resolved_for_logging = resolved.clone();

    debug!(
        session_id = %session.session_id,
        prompt_blocks = params.prompt.len(),
        prompt_chars = prompt_text.chars().count(),
        include_context = params.include_context,
        model_key = %model_key,
        workdir = %workdir.display(),
        "handling ACP session prompt"
    );

    if !is_slash_command {
        session.push_user_turn(prompt_text.clone());
    }

    // Permission is requested per-tool-call by the agent, not preemptively.

    let should_resolve_context = !is_slash_command && pipeline_template.is_none();

    let knowledge = if is_slash_command {
        DispatchKnowledge::default()
    } else {
        query_dispatch_knowledge(workdir, &prompt_text).await
    };
    let knowledge_context = knowledge.context_text();

    // Resolve context only for the single-agent path.
    // Resource blocks always resolve; @-mentions are only resolved when
    // prompt-time context is enabled.
    let file_context = if should_resolve_context {
        if params.include_context {
            resolve_context_items(&params.prompt, workdir).await
        } else {
            let uris = extract_resource_uris(&params.prompt);
            if uris.is_empty() {
                String::new()
            } else {
                read_file_context(&uris, workdir)
            }
        }
    } else {
        String::new()
    };

    // Get system prompt and history context for the single-agent path.
    let system_prompt = if should_resolve_context {
        session.build_system_prompt(workdir, &[], session.cached_conventions.as_deref())
    } else {
        String::new()
    };
    let _history_context = if should_resolve_context {
        session.build_history_context_for_cli()
    } else {
        String::new()
    };
    let messages = if should_resolve_context {
        // Build combined system prompt with resolved context.
        let mut full_system = system_prompt.clone();
        full_system = append_context(&full_system, &file_context);
        full_system = append_context(&full_system, &knowledge_context);
        if let Some(assignment) = experiment_assignment.as_ref() {
            full_system = replace_experiment_section(&full_system, assignment);
            // Mark the experiment as dispatched with the final prompt hash,
            // completing the Prepared -> Dispatched lifecycle transition.
            let prompt_hash = roko_core::ContentHash::of(full_system.as_bytes()).to_hex();
            mark_acp_experiment_dispatched(&experiment_path, assignment, &prompt_hash);
        }
        let mut msgs = session.build_messages_array(&full_system, &prompt_text);
        // If the prompt contains Image blocks, replace the last user message's
        // content with a multi-part content array in the appropriate format.
        inject_image_parts(&mut msgs, &params.prompt, resolved.provider_kind);
        msgs
    } else {
        // Pipeline path: build a minimal user-message array so that image blocks
        // are preserved for any downstream consumer that inspects `messages`.
        let dispatch_prompt = experiment_assignment.as_ref().map_or_else(
            || prompt_text.clone(),
            |assignment| append_context(&prompt_text, &render_experiment_context(assignment)),
        );
        let mut msgs = vec![serde_json::json!({"role": "user", "content": dispatch_prompt})];
        inject_image_parts(&mut msgs, &params.prompt, resolved.provider_kind);
        msgs
    };
    let input_messages = if prompt_has_images {
        model_input_messages_from_wire(&messages).map_err(|error| {
            BridgeEventsError::UnsupportedPromptContent(format!("invalid image input: {error}"))
        })?
    } else {
        Vec::new()
    };

    let (event_sender, event_receiver) = mpsc::channel(256);
    if !is_slash_command {
        emit_knowledge_card(&knowledge, &event_sender).await;
    }
    let provenance = if is_slash_command {
        None
    } else {
        build_provenance(&knowledge.hits, &knowledge.playbooks, &prompt_text, workdir).await
    };
    let provenance_card = provenance.as_ref().map(render_provenance_card);
    if !is_slash_command
        && pipeline_template.is_none()
        && let Some(chain) = provenance.as_ref()
    {
        emit_provenance_card(chain, &event_sender).await;
    }
    let stream_cancel_token = session.cancel_token.clone();
    let cancel_token = stream_cancel_token.clone();
    let stream_session_id = session.session_id.clone();
    let session_id = stream_session_id.clone();
    let worktree_before = crate::runner::WorktreeChangeSnapshot::capture(workdir);
    let workdir = workdir.to_path_buf();
    let workdir_for_logging = workdir.clone();
    let roko_config = roko_config.clone();
    let roko_config_for_logging = roko_config.clone();
    let prompt_text_for_logging = prompt_text.clone();
    let prompt_text_for_dispatch = experiment_assignment.as_ref().map_or_else(
        || prompt_text.clone(),
        |assignment| append_context(&prompt_text, &render_experiment_context(assignment)),
    );
    // P1-ACP-2: For the pipeline/workflow path, complete the Prepared ->
    // Dispatched lifecycle transition using the combined prompt text hash.
    // The single-agent path does this inside the `should_resolve_context`
    // branch above (where it also has the full system prompt available for
    // section replacement). The pipeline path only has the user prompt, so
    // the hash covers what it actually sends to the engine.
    if pipeline_template.is_some()
        && let Some(assignment) = experiment_assignment.as_ref()
    {
        let prompt_hash = roko_core::ContentHash::of(prompt_text_for_dispatch.as_bytes()).to_hex();
        mark_acp_experiment_dispatched(&experiment_path, assignment, &prompt_hash);
    }
    // The actual dispatched config key must drive provider construction,
    // episode/cost attribution, and the router observation arm.
    let model_key_for_logging = model_key_for_dispatch.clone();
    let cascade_selection_for_logging = cascade_selection.clone();
    let dispatch_started = Instant::now();
    let is_pipeline_dispatch = pipeline_template.is_some();

    let clippy_enabled = session.config_state.clippy_enabled;
    let tests_enabled = session.config_state.tests_enabled;
    let max_iterations = session.config_state.max_iterations;
    let review_strictness = session.config_state.review_strictness.clone();
    let session_mcp_servers = session.mcp_servers.clone();
    let session_mcp_config_path = session.mcp_config_path.clone();
    let session_tools_enabled = session.tools_enabled;
    let session_agent_role = session.config_state.agent_mode.clone();
    let session_tool_capabilities = derive_acp_tool_capabilities(
        &session.config_state.agent_mode,
        &session.client_capabilities,
        !session_mcp_servers.is_empty(),
        &session.always_allowed,
    );
    // Effort level from the IDE dropdown (low/medium/high/max). Passed to
    // config_with_session_effort() at dispatch time so the provider backend
    // sees it as `agent.default_effort`. See that function's doc comment for
    // the full effort dispatch flow.
    let session_effort = session.config_state.effort.clone();

    let shared_run = session.shared_run.clone();
    // SP-1: build a restrictive layer per dispatch; missing contracts fall closed.
    let pre_dispatch_violation = {
        let safety =
            SafetyLayer::from_config(&roko_config).with_role(&session.config_state.agent_mode);
        match safety.pre_dispatch_check_with_context(
            &session.session_id,
            "session-prompt",
            &session.config_state.agent_mode,
            &workdir,
            &DispatchSafetyContext::for_local_action(&prompt_text).with_network_requirement(true),
        ) {
            Ok(()) => None,
            Err(violation) => match violation.severity {
                ViolationSeverity::Block => {
                    error!(
                        session_id = %session.session_id,
                        violation = ?violation.violation_type,
                        message = %violation.message,
                        "ACP pre-dispatch safety check BLOCKED dispatch"
                    );
                    Some(violation)
                }
                ViolationSeverity::Warn => {
                    warn!(
                        session_id = %session.session_id,
                        violation = ?violation.violation_type,
                        message = %violation.message,
                        "ACP pre-dispatch safety warning"
                    );
                    None
                }
            },
        }
    };

    // Shared channel for the workflow engine path: the cognitive task writes the
    // WorkflowRunReport's actual cost (which was aggregated from AgentCompleted events)
    // here so that append_acp_episode can use it instead of the pricing-table estimate.
    let prompt_text_for_title = prompt_text.clone();

    let workflow_cost_sink: Arc<Mutex<Option<f64>>> = Arc::new(Mutex::new(None));
    let workflow_cost_sink_task = Arc::clone(&workflow_cost_sink);

    let cognitive_task = tokio::spawn(async move {
        if let Some(violation) = pre_dispatch_violation {
            let message = violation.message;
            let _ = event_sender
                .send(CognitiveEvent::TokenChunk(format!(
                    "Safety check blocked this action: {}",
                    message
                )))
                .await;
            let _ = event_sender
                .send(CognitiveEvent::Complete {
                    stop_reason: StopReason::EndTurn,
                    usage: None,
                })
                .await;
            return Err(anyhow::anyhow!("ACP pre-dispatch safety violation: {}", message).into());
        }

        if is_slash_command {
            return run_slash_command(
                &session_id,
                prompt_text_for_dispatch.trim(),
                &workdir,
                model_key_for_dispatch.clone(),
                cancel_token,
                event_sender,
                shared_run,
            )
            .await;
        }

        if let Some(template) = pipeline_template {
            if std::env::var_os("ROKO_ACP_LEGACY").is_some() {
                let legacy_run = shared_run.clone();
                let result = crate::runner::run_workflow_pipeline(
                    &session_id,
                    &prompt_text_for_dispatch,
                    knowledge_context.clone(),
                    provenance_card.clone(),
                    &workdir,
                    crate::runner::PipelineConfig {
                        template,
                        max_iterations,
                        clippy_enabled,
                        tests_enabled,
                        review_strictness,
                        model_slug: resolved.slug.clone(),
                        mcp_config: write_session_mcp_config(&session_mcp_servers, &workdir),
                        sandbox_level: roko_config.runner.sandbox_level,
                    },
                    cancel_token,
                    event_sender,
                    legacy_run.clone(),
                )
                .await;

                result?;

                let final_phase = legacy_run
                    .lock()
                    .await
                    .as_ref()
                    .map(|run| run.pipeline.phase.clone());

                return match final_phase {
                    Some(crate::pipeline::PipelinePhase::Complete) => Ok(()),
                    Some(crate::pipeline::PipelinePhase::Halted { reason }) => {
                        Err(anyhow::anyhow!("workflow pipeline halted: {reason}").into())
                    }
                    Some(crate::pipeline::PipelinePhase::Cancelled) => {
                        Err(anyhow::anyhow!("workflow pipeline cancelled").into())
                    }
                    Some(phase) => Err(anyhow::anyhow!(
                        "workflow pipeline ended in unexpected phase: {phase:?}"
                    )
                    .into()),
                    None => Err(anyhow::anyhow!(
                        "workflow pipeline completed without shared run state"
                    )
                    .into()),
                };
            }

            let mcp_config_path = write_session_mcp_config(&session_mcp_servers, &workdir);
            let report = run_with_workflow_engine(
                &session_id,
                &prompt_text_for_dispatch,
                &workdir,
                workflow_template_name(&template),
                crate::runner::GraphEngineOptions {
                    model_key: model_key_for_dispatch,
                    input_messages: input_messages.clone(),
                    mcp_config: mcp_config_path,
                    provenance_card,
                    route: crate::runner::AcpWorkflowRoute::LegacyDefault,
                },
                event_sender,
            )
            .await?;

            // Thread the actual cost from the report back to the main task so
            // append_acp_episode can record it instead of using the pricing-table estimate.
            if let Some(cost) = report.cost
                && let Ok(mut sink) = workflow_cost_sink_task.lock()
            {
                *sink = Some(cost);
            }

            if !report.success {
                return Err(anyhow::anyhow!(
                    "workflow engine reported unsuccessful run: {}",
                    report.output
                )
                .into());
            }

            return Ok(());
        }

        // Default: single-agent dispatch (workflow = "none").
        let provider_kind = resolved.provider_kind;

        info!(
            requested_model = %model_key,
            model_key = %model_key_for_dispatch,
            slug = %resolved.slug,
            provider_kind = ?provider_kind,
            "resolved model for ACP prompt"
        );

        match provider_kind {
            // AnthropicApi uses the dedicated Anthropic model caller path.
            // The provider must be present in explicit RokoConfig; ACP does
            // not synthesize providers from ANTHROPIC_API_KEY.
            ProviderKind::AnthropicApi => {
                run_anthropic_cognitive_task(
                    &session_id,
                    &messages,
                    &model_key_for_dispatch,
                    &resolved.slug,
                    &roko_config,
                    Arc::clone(&provider_health),
                    Arc::clone(&provider_rate_limiter),
                    &workdir,
                    &session_mcp_servers,
                    &session_effort,
                    session_tools_enabled,
                    session_tool_capabilities,
                    &session_agent_role,
                    cancel_token,
                    event_sender,
                )
                .await
            }
            // All other providers (ClaudeCli, OpenAiCompat, etc.) go through
            // ModelCallService which handles each provider kind natively.
            _ => {
                run_openai_compat_cognitive_task(
                    &session_id,
                    &messages,
                    &model_key_for_dispatch,
                    &roko_config,
                    Arc::clone(&provider_health),
                    Arc::clone(&provider_rate_limiter),
                    &workdir,
                    &session_mcp_servers,
                    session_mcp_config_path.as_deref(),
                    &session_effort,
                    session_tools_enabled,
                    session_tool_capabilities,
                    &session_agent_role,
                    cancel_token,
                    event_sender,
                )
                .await
            }
        }
    });

    let mut stream_result = stream_events_to_editor(
        transport,
        &stream_session_id,
        session,
        &workdir_for_logging,
        event_receiver,
        &stream_cancel_token,
    )
    .await;

    if !is_slash_command
        && let Ok(ref sr) = stream_result
        && let Some(usage) = sr.usage.as_ref()
    {
        let size = resolved_for_logging
            .profile
            .as_ref()
            .map(|profile| profile.context_window)
            .unwrap_or_else(|| ModelProfile::default().context_window);
        let update = SessionUpdate::UsageUpdate {
            used: usage.total_tokens,
            size,
            cost: calculate_cost_for_model_slug(
                &resolved_for_logging.slug,
                usage.input_tokens,
                usage.output_tokens,
                usage.cached_read_tokens.unwrap_or(0),
            )
            .map(|amount| CostInfo {
                amount,
                currency: "USD".to_string(),
            }),
        };
        if let Err(error) = send_session_update(transport, &session.session_id, update).await {
            warn!(
                session_id = %session.session_id,
                error = %error,
                "failed to send ACP usage update"
            );
        }
    }

    let task_result = cognitive_task.await;
    let (task_error, task_join_error) = match task_result {
        Ok(Ok(())) => (None, None),
        Ok(Err(e)) => {
            let error_text = e.to_string();
            if error_text.starts_with("ACP pre-dispatch safety violation:") {
                warn!(error = %error_text, "cognitive task blocked before dispatch");
            } else {
                error!(error = %error_text, "cognitive task failed");
            }
            (Some(error_text), None)
        }
        Err(join_error) => {
            let error_text = join_error.to_string();
            error!(error = %error_text, "cognitive task failed to join");
            (
                Some(error_text),
                Some(BridgeEventsError::TaskJoin(join_error)),
            )
        }
    };
    let stream_error = stream_result.as_ref().err().map(|err| err.to_string());

    let post_dispatch_block = if let Ok(ref sr) = stream_result
        && !sr.assistant_text.is_empty()
    {
        let changed_files = worktree_before.changed_files(&workdir_for_logging);
        let safety = SafetyLayer::from_config(&roko_config_for_logging)
            .with_role(&session.config_state.agent_mode);
        let violations = safety.post_dispatch_check(
            &session.session_id,
            "session-prompt",
            &session.config_state.agent_mode,
            &sr.assistant_text,
            &changed_files,
        );
        for v in &violations {
            match v.severity {
                ViolationSeverity::Warn | ViolationSeverity::Block => {
                    warn!(
                        session_id = %session.session_id,
                        violation = ?v.violation_type,
                        message = %v.message,
                        "ACP post-dispatch safety violation"
                    );
                }
            }
        }
        let block_messages = violations
            .iter()
            .filter(|violation| violation.severity == ViolationSeverity::Block)
            .map(|violation| format!("{}: {}", violation.violation_type, violation.message))
            .collect::<Vec<_>>();
        (!block_messages.is_empty()).then(|| block_messages.join("; "))
    } else {
        None
    };
    if let Some(block_message) = post_dispatch_block {
        stream_result = Err(BridgeEventsError::Pipeline(anyhow::anyhow!(
            "ACP post-dispatch safety block: {block_message}"
        )));
    }

    if !is_slash_command {
        // For the workflow engine path, the cognitive task wrote the actual provider cost
        // (from WorkflowRunReport) to workflow_cost_sink. Use it to override the
        // pricing-table estimate in append_acp_episode so the episode has accurate cost data.
        let cost_override = workflow_cost_sink.lock().ok().and_then(|g| *g);
        append_acp_episode(
            &roko_config_for_logging,
            &workdir_for_logging,
            session,
            &model_key_for_logging,
            &prompt_text_for_logging,
            &workflow_config,
            is_pipeline_dispatch,
            dispatch_started,
            stream_result.as_ref().ok(),
            task_error.as_deref(),
            stream_error.as_deref(),
            cost_override,
            cascade_selection_for_logging.as_ref(),
        )
        .await;

        let stream_result_ref = stream_result.as_ref().ok();
        let dispatch_succeeded = acp_dispatch_succeeded(
            stream_result_ref,
            task_error.as_deref(),
            stream_error.as_deref(),
        );
        let efficiency_event = acp_efficiency_event(
            &session.session_id,
            &resolved_for_logging,
            dispatch_started,
            stream_result_ref,
            dispatch_succeeded,
            cost_override,
        );
        session.record_efficiency_cost(efficiency_event.cost_usd);
        let budget_status = session.budget_status();
        if budget_status.cost_budget_usd.is_some()
            && let Err(error) = send_session_update(
                transport,
                &session.session_id,
                roko_meta_update("budget", &budget_status),
            )
            .await
        {
            warn!(
                session_id = %session.session_id,
                error = %error,
                "failed to send ACP budget status update"
            );
        }
        emit_acp_efficiency_event(&workdir_for_logging, efficiency_event);

        if let Some(assignment) = experiment_assignment.as_ref()
            && let Err(error) =
                record_acp_experiment_outcome(&experiment_path, assignment, dispatch_succeeded)
        {
            warn!(
                experiment_id = %assignment.experiment_id,
                variant_id = %assignment.variant_id,
                error = %error,
                "failed to persist ACP experiment outcome"
            );
        }

        if !is_pipeline_dispatch {
            let model_slugs =
                cascade_router_model_slugs(&roko_config_for_logging, &resolved_for_logging.slug);
            let routing_ctx = acp_routing_context(
                &session.config_state.agent_mode,
                &prompt_text_for_logging,
                &session.config_state.effort,
                &workdir_for_logging,
            );
            let output_tokens =
                stream_result_ref.and_then(|sr| sr.usage.as_ref().map(|usage| usage.output_tokens));
            // Observe only direct prompts here. Workflow services own their own
            // provider feedback and recording them again would train the wrong arm.
            drop(record_cascade_observation(
                workdir_for_logging
                    .join(".roko")
                    .join("learn")
                    .join("cascade-router.json"),
                model_key_for_logging.clone(),
                routing_ctx,
                dispatch_succeeded,
                dispatch_started.elapsed().as_millis() as u64,
                output_tokens,
                model_slugs,
            ));
        }
    }

    if let Some(join_error) = task_join_error {
        return Err(join_error);
    }

    // Auto-set session title from first user message.
    if session.session_name.is_none() && !is_slash_command {
        let title = truncate_to_title(&prompt_text_for_title, 60);
        session.session_name = Some(title.clone());
        let title_update = SessionUpdate::SessionInfoUpdate {
            title: Some(title),
            _meta: None,
        };
        if let Err(error) = send_session_update(transport, &session.session_id, title_update).await
        {
            warn!(
                session_id = %session.session_id,
                error = %error,
                "failed to send ACP session title update"
            );
        }
    }

    // Push assistant turn after streaming completes (skip slash commands).
    // If dispatch failed (empty assistant text), pop the user turn we pushed earlier
    // to prevent a dangling user message with no response in the history.
    match &stream_result {
        Ok(sr) if !is_slash_command && !sr.assistant_text.is_empty() => {
            session.push_assistant_turn(truncate_assistant_history(&sr.assistant_text));
        }
        _ if !is_slash_command => {
            session.conversation_history.pop();
        }
        _ => {}
    }

    stream_result.map(|sr| sr.prompt_result)
}
