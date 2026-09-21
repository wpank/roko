//! Permission request/response channels for ACP consent flow.

use std::path::Path;
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncWrite};
use tracing::{debug, info, warn};

use crate::session::{AcpSession, CancelToken};
use crate::transport::StdioTransport;
use crate::types::{
    JsonRpcMessage, PermissionAction, PermissionDecision, PermissionOptionKind, PermissionOutcome,
    PermissionResponse, PermissionToolCall, RequestPermissionParams, SessionCancelParams,
};

use super::{PermissionReplyChannel, PermissionRequestPayload};

pub async fn request_permission<R, W>(
    transport: &mut StdioTransport<R, W>,
    session: &mut AcpSession,
    workdir: &Path,
    action: PermissionAction,
    title: &str,
    detail: &str,
) -> PermissionDecision
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    if session.is_pre_granted(&action) {
        debug!(
            session_id = %session.session_id,
            action = ?action,
            "permission pre-granted (always-allow)"
        );
        return PermissionDecision::Allow;
    }

    debug!(
        session_id = %session.session_id,
        action = ?action,
        title = %title,
        detail = %detail,
        "requesting permission from editor"
    );

    let tool_call_id = format!("perm-{}", uuid::Uuid::new_v4());
    let params = serde_json::to_value(RequestPermissionParams {
        session_id: session.session_id.clone(),
        tool_call: PermissionToolCall {
            tool_call_id: tool_call_id.clone(),
            title: format!("{title}: {detail}"),
        },
        options: PermissionOptionKind::standard_options(),
    })
    .unwrap_or_else(|error| {
        warn!(
            session_id = %session.session_id,
            action = ?action,
            error = %error,
            "failed to serialize permission request; sending null payload"
        );
        serde_json::Value::Null
    });

    let mut request_transport = transport.clone();
    let request_future = request_transport.send_request("session/request_permission", params);
    tokio::pin!(request_future);
    let timeout = tokio::time::sleep(std::time::Duration::from_secs(30));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            biased;
            response = &mut request_future => {
                match response {
                    Ok(json_response) => {
                        if let Some(error) = json_response.error.as_ref() {
                            warn!(
                                session_id = %session.session_id,
                                action = ?action,
                                code = error.code,
                                message = %error.message,
                                "permission request returned an error; defaulting to Reject"
                            );
                            return PermissionDecision::Reject;
                        }

                        let decision = json_response
                            .result
                            .as_ref()
                            .and_then(|value| serde_json::from_value::<PermissionResponse>(value.clone()).ok())
                            .map(|response| match response.outcome {
                                PermissionOutcome::Selected { ref option_id } => {
                                    PermissionOptionKind::decision_from_option_id(option_id)
                                        .unwrap_or(PermissionDecision::Reject)
                                }
                                PermissionOutcome::Cancelled => PermissionDecision::Reject,
                            })
                            .unwrap_or_else(|| {
                                warn!(
                                    session_id = %session.session_id,
                                    action = ?action,
                                    "permission response could not be parsed; defaulting to Reject"
                                );
                                PermissionDecision::Reject
                            });

                        if matches!(decision, PermissionDecision::AlwaysAllow) {
                            session.grant_always_allow(action.clone());
                            match AcpSession::save_workspace_trust(
                                workdir,
                                &session.always_allowed,
                            ) {
                                Ok(()) => info!(
                                    session_id = %session.session_id,
                                    action = ?action,
                                    "permission permanently granted (always-allow persisted)"
                                ),
                                Err(error) => warn!(
                                    session_id = %session.session_id,
                                    action = ?action,
                                    error = %error,
                                    "always-allow retained for this session but workspace persistence failed"
                                ),
                            }
                        }

                        return decision;
                    }
                    Err(error) => {
                        warn!(
                            session_id = %session.session_id,
                            action = ?action,
                            error = %error,
                            "permission request transport error; defaulting to Reject"
                        );
                        return PermissionDecision::Reject;
                    }
                }
            }
            inbound = transport.read_message() => {
                match inbound {
                    Ok(Some(JsonRpcMessage::Response(response))) => {
                        transport.handle_incoming_response(response);
                    }
                    Ok(Some(JsonRpcMessage::Notification(notification))) => {
                        if notification.method == "session/cancel" {
                            match serde_json::from_value::<SessionCancelParams>(
                                notification.params.unwrap_or(serde_json::Value::Null),
                            ) {
                                Ok(params) if params.session_id == session.session_id => {
                                    session.cancel_token.cancel();
                                    warn!(
                                        session_id = %session.session_id,
                                        "permission request cancelled by client; defaulting to Reject"
                                    );
                                    return PermissionDecision::Reject;
                                }
                                Ok(_) => {}
                                Err(error) => {
                                    warn!(
                                        session_id = %session.session_id,
                                        error = %error,
                                        "received malformed session/cancel while waiting for permission"
                                    );
                                }
                            }
                        } else {
                            debug!(
                                session_id = %session.session_id,
                                method = %notification.method,
                                "ignoring notification while waiting for permission"
                            );
                        }
                    }
                    Ok(Some(JsonRpcMessage::Request(request))) => {
                        warn!(
                            session_id = %session.session_id,
                            method = %request.method,
                            "ignoring inbound request while waiting for permission"
                        );
                    }
                    Ok(None) => {
                        warn!(
                            session_id = %session.session_id,
                            "ACP client disconnected while waiting for permission"
                        );
                        return PermissionDecision::Reject;
                    }
                    Err(error) => {
                        warn!(
                            session_id = %session.session_id,
                            error = %error,
                            "failed to read inbound message while waiting for permission; defaulting to Reject"
                        );
                        return PermissionDecision::Reject;
                    }
                }
            }
            _ = &mut timeout => {
                warn!(
                    session_id = %session.session_id,
                    action = ?action,
                    "permission request timed out after 30 seconds; defaulting to Reject"
                );
                return PermissionDecision::Reject;
            }
        }
    }
}

/// Runs the editor round-trip while respecting both the enclosing prompt and
/// tool-handler lifetimes. The handler-side receiver disappears when the tool
/// dispatcher times out, so observing that state prevents the parent stream
/// from waiting for the longer editor timeout after there is nobody to answer.
pub(crate) async fn request_permission_for_event<R, W>(
    transport: &mut StdioTransport<R, W>,
    session: &mut AcpSession,
    workdir: &Path,
    payload: &PermissionRequestPayload,
    reply: &PermissionReplyChannel,
    cancel_token: &CancelToken,
) -> PermissionDecision
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let session_id = session.session_id.clone();
    let request = request_permission(
        transport,
        session,
        workdir,
        payload.action.clone(),
        &payload.title,
        &payload.detail,
    );
    tokio::pin!(request);

    loop {
        if reply.receiver_is_closed() {
            warn!(
                session_id = %session_id,
                action = ?payload.action,
                "permission requester stopped waiting; abandoning editor request"
            );
            return PermissionDecision::Reject;
        }

        tokio::select! {
            decision = &mut request => return decision,
            _ = cancel_token.cancelled() => {
                warn!(
                    session_id = %session_id,
                    action = ?payload.action,
                    "ACP prompt cancelled while waiting for editor permission"
                );
                return PermissionDecision::Reject;
            }
            () = tokio::time::sleep(Duration::from_millis(25)) => {}
        }
    }
}
