//! Host-mergeable HTTP routes for inference, stats, and asynchronous batches.

use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::extract::{Path, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use serde::Serialize;

use crate::{
    BackpressureError, BatchQueue, BatchResult, ClientBatchProcessor, GatewayError, GatewayResult,
    GatewayStats, InferenceClient, InferenceGateway, InferenceRequest, InferenceResponse,
};

/// State owned by the gateway-specific HTTP router.
#[derive(Clone)]
pub struct GatewayHttpState {
    /// Live pipeline used for synchronous inference and stats.
    pub gateway: Arc<InferenceGateway>,
    /// Bounded asynchronous batch queue.
    pub batch: Arc<BatchQueue>,
}

impl GatewayHttpState {
    /// Create the standard 50-item/30-second batch queue, with drained items
    /// processed through the same gateway pipeline.
    #[must_use]
    pub fn new(gateway: Arc<InferenceGateway>) -> Self {
        let client: Arc<dyn InferenceClient> = gateway.clone();
        let batch = Arc::new(BatchQueue::with_processor(
            crate::batch::DEFAULT_FLUSH_INTERVAL,
            crate::batch::DEFAULT_FLUSH_SIZE,
            1_000,
            Arc::new(ClientBatchProcessor::new(client)),
        ));
        Self { gateway, batch }
    }

    /// Supply a native or otherwise custom batch queue.
    #[must_use]
    pub fn with_batch(gateway: Arc<InferenceGateway>, batch: Arc<BatchQueue>) -> Self {
        Self { gateway, batch }
    }

    /// Start the batch age timer. The host retains the returned task for
    /// shutdown coordination.
    pub fn spawn_batch_loop(&self) -> tokio::task::JoinHandle<()> {
        Arc::clone(&self.batch).spawn_auto_flush()
    }
}

/// Routes that a host can merge directly into its root Axum router.
///
/// Authentication is intentionally a host middleware concern, because the
/// host owns agent-token validation and identity injection.
pub fn gateway_routes(state: GatewayHttpState) -> Router {
    Router::new()
        .route("/api/gateway/inference", post(inference))
        .route("/api/gateway/stats", get(stats))
        .route("/api/gateway/batch/submit", post(batch_submit))
        .route("/api/gateway/batch/flush", post(batch_flush))
        .route("/api/gateway/batch/result/{id}", get(batch_result))
        .with_state(state)
}

async fn inference(
    State(state): State<GatewayHttpState>,
    Json(request): Json<InferenceRequest>,
) -> GatewayResult<Json<InferenceResponse>> {
    state.gateway.complete(request).await.map(Json)
}

async fn stats(State(state): State<GatewayHttpState>) -> Json<GatewayStats> {
    Json(state.gateway.stats())
}

#[derive(Debug, Serialize)]
struct BatchAccepted {
    custom_id: String,
}

async fn batch_submit(
    State(state): State<GatewayHttpState>,
    Json(request): Json<InferenceRequest>,
) -> GatewayResult<(StatusCode, Json<BatchAccepted>)> {
    let custom_id = state.batch.submit(request).await?;
    Ok((StatusCode::ACCEPTED, Json(BatchAccepted { custom_id })))
}

#[derive(Debug, Serialize)]
struct BatchFlushed {
    flushed: usize,
}

async fn batch_flush(State(state): State<GatewayHttpState>) -> GatewayResult<Json<BatchFlushed>> {
    let flushed = state.batch.flush().await?;
    Ok(Json(BatchFlushed { flushed }))
}

async fn batch_result(
    State(state): State<GatewayHttpState>,
    Path(custom_id): Path<String>,
) -> Result<Json<BatchResult>, StatusCode> {
    state
        .batch
        .get_result(&custom_id)
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: &'static str,
    message: String,
}

impl IntoResponse for GatewayError {
    fn into_response(self) -> Response {
        let retry_after = match &self {
            Self::Backpressure(error) => Some(error.retry_after_seconds()),
            _ => None,
        };
        let (status, code) = match &self {
            Self::Backpressure(BackpressureError::AgentQueueFull { .. }) => {
                (StatusCode::TOO_MANY_REQUESTS, "agent_queue_full")
            }
            Self::Backpressure(BackpressureError::ProviderFull { .. }) => {
                (StatusCode::SERVICE_UNAVAILABLE, "provider_queue_full")
            }
            Self::Backpressure(BackpressureError::GlobalOverload) => {
                (StatusCode::SERVICE_UNAVAILABLE, "gateway_overloaded")
            }
            Self::BudgetExceeded { .. } => (StatusCode::PAYMENT_REQUIRED, "budget_exceeded"),
            Self::ProvidersExhausted(_) | Self::NoProvider(_) => {
                (StatusCode::SERVICE_UNAVAILABLE, "providers_unavailable")
            }
            Self::Provider { .. } => (StatusCode::BAD_GATEWAY, "provider_failure"),
            Self::BatchQueueFull { .. } => (StatusCode::SERVICE_UNAVAILABLE, "batch_queue_full"),
            Self::ChannelClosed => (StatusCode::SERVICE_UNAVAILABLE, "gateway_unavailable"),
            Self::AlreadyStarted | Self::CacheDecode(_) | Self::EventLog(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "gateway_internal")
            }
        };
        let mut response = (
            status,
            Json(ErrorBody {
                error: code,
                message: self.to_string(),
            }),
        )
            .into_response();
        if let Some(seconds) = retry_after
            && let Ok(value) = HeaderValue::from_str(&seconds.to_string())
        {
            response.headers_mut().insert(header::RETRY_AFTER, value);
        }
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_backpressure_errors_keep_status_and_retry_after_contract() {
        let response = GatewayError::Backpressure(BackpressureError::AgentQueueFull {
            agent_id: "agent".into(),
        })
        .into_response();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(response.headers()[header::RETRY_AFTER], "2");

        let response =
            GatewayError::Backpressure(BackpressureError::GlobalOverload).into_response();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(response.headers()[header::RETRY_AFTER], "5");
    }
}
