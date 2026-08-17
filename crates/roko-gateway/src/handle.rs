//! Agent-facing bounded-channel inference handle.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;
use futures::stream::BoxStream;
use tokio::sync::{mpsc, oneshot};
use tokio_stream::wrappers::ReceiverStream;

use crate::{
    AgentId, GatewayError, GatewayResult, InferenceChunk, InferenceClient, InferenceRequest,
    InferenceResponse,
};

/// Response transport selected by the handle call.
pub enum InferenceReply {
    /// One complete response.
    Complete(oneshot::Sender<GatewayResult<InferenceResponse>>),
    /// Incremental response items.
    Stream(mpsc::Sender<GatewayResult<InferenceChunk>>),
}

/// Internal request plus response channel.
pub struct InferenceEnvelope {
    /// Authoritative caller identity.
    pub agent_id: AgentId,
    /// Provider-neutral request.
    pub request: InferenceRequest,
    /// Per-handle remaining budget.
    pub(crate) budget: Arc<AtomicU64>,
    /// Response transport.
    pub reply: InferenceReply,
}

/// Cloneable inference capability containing only bounded-channel state.
#[derive(Clone)]
pub struct InferenceHandle {
    sender: mpsc::Sender<InferenceEnvelope>,
    agent_id: AgentId,
    budget: Arc<AtomicU64>,
}

impl InferenceHandle {
    /// Construct a handle from the gateway's bounded sender.
    #[must_use]
    pub fn new(
        sender: mpsc::Sender<InferenceEnvelope>,
        agent_id: AgentId,
        budget_microdollars: u64,
    ) -> Self {
        Self {
            sender,
            agent_id,
            budget: Arc::new(AtomicU64::new(budget_microdollars)),
        }
    }

    /// Send a request and await one completed response.
    pub async fn infer(&self, mut request: InferenceRequest) -> GatewayResult<InferenceResponse> {
        request.metadata.agent_id.clone_from(&self.agent_id);
        request.metadata.budget_remaining = self.remaining_budget();
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(InferenceEnvelope {
                agent_id: self.agent_id.clone(),
                request,
                budget: Arc::clone(&self.budget),
                reply: InferenceReply::Complete(sender),
            })
            .await
            .map_err(|_| GatewayError::ChannelClosed)?;
        receiver.await.map_err(|_| GatewayError::ChannelClosed)?
    }

    /// Send a request and receive incremental stream items.
    pub async fn infer_stream(
        &self,
        mut request: InferenceRequest,
    ) -> GatewayResult<BoxStream<'static, GatewayResult<InferenceChunk>>> {
        request.metadata.agent_id.clone_from(&self.agent_id);
        request.metadata.budget_remaining = self.remaining_budget();
        request.stream = true;
        let (sender, receiver) = mpsc::channel(64);
        self.sender
            .send(InferenceEnvelope {
                agent_id: self.agent_id.clone(),
                request,
                budget: Arc::clone(&self.budget),
                reply: InferenceReply::Stream(sender),
            })
            .await
            .map_err(|_| GatewayError::ChannelClosed)?;
        Ok(Box::pin(ReceiverStream::new(receiver)))
    }

    /// Remaining budget in microdollars.
    #[must_use]
    pub fn remaining_budget(&self) -> u64 {
        self.budget.load(Ordering::Relaxed)
    }
}

#[async_trait]
impl InferenceClient for InferenceHandle {
    async fn complete(&self, request: InferenceRequest) -> GatewayResult<InferenceResponse> {
        self.infer(request).await
    }

    async fn stream(
        &self,
        request: InferenceRequest,
    ) -> GatewayResult<BoxStream<'static, GatewayResult<InferenceChunk>>> {
        self.infer_stream(request).await
    }
}
