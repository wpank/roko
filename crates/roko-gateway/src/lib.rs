//! Centralized, provider-neutral inference gateway.
//!
//! Agents interact through a bounded channel while this crate applies routing,
//! cache, budget, safety-of-operation, provider, persistence, and accounting
//! policy around every model call. Hosts initialize [`InferenceGateway`], call
//! [`InferenceGateway::create_handle`], and start its bounded `process` loop
//! with [`InferenceGateway::spawn_gateway_loop`].

#![allow(clippy::module_name_repetitions)]

pub mod backpressure;
pub mod batch;
pub mod cache;
pub mod convergence;
pub mod cost_track;
pub mod error;
pub mod gateway;
pub mod handle;
pub mod http;
pub mod loop_detect;
pub mod output_budget;
pub mod provider;
pub mod thinking_cap;
pub mod tool_prune;
pub mod types;

pub use backpressure::{
    BackpressureConfig, BackpressureError, BackpressureGuard, BackpressurePermit,
    BackpressureStats, ProviderBackpressureStats, ProviderLimitConfig,
};
pub use batch::{
    BATCH_POLL_INTERVAL, BatchEntry, BatchProcessor, BatchQueue, BatchResult, BatchStatus,
    ClientBatchProcessor,
};
pub use cache::{
    CacheHit, CacheLayer, CacheStats, CachedResponse, InferenceCache, SimHashEntry,
    hamming_distance, normalize_request, simhash, ttl_for,
};
pub use convergence::{ConvergenceDetector, ConvergenceState, ConvergenceStats};
pub use cost_track::{CostAggregate, CostBreakdown, CostRecord, CostResult, CostTracker};
pub use error::{GatewayError, GatewayResult, ProviderFailureKind};
pub use gateway::{GatewayConfig, GatewayStats, InferenceGateway, PipelineStage};
pub use handle::{InferenceEnvelope, InferenceHandle, InferenceReply};
pub use http::{GatewayHttpState, gateway_routes};
pub use loop_detect::{LoopDetector, LoopStats, SessionLoopState};
pub use output_budget::{ModelOutputStats, OutputBudgetStats, OutputBudgeter};
pub use provider::{
    KeyRing, ModelCallerBackend, ProviderBackend, classify_provider_error, failed_stream,
};
pub use thinking_cap::{ThinkingCapStats, ThinkingCapper, apply_thinking_cap, default_budget};
pub use tool_prune::{PruneStats, ToolPruner};
pub use types::{
    AgentId, CacheRegime, InferenceChunk, InferenceMeta, InferenceRequest, InferenceResponse,
    Message, StopReason, ThinkingConfig, ThinkingMode, Tier, TokenUsage, ToolCallObservation,
    ToolSchema,
};

use async_trait::async_trait;
use futures::stream::BoxStream;

/// Loader-compatible graph definition for the nine-stage gateway pipeline.
pub const PIPELINE_GRAPH_TOML: &str = include_str!("../inference-gateway.toml");

/// Provider-neutral client contract used by agents and batch workers.
#[async_trait]
pub trait InferenceClient: Send + Sync {
    /// Complete one request through the full gateway pipeline.
    async fn complete(&self, request: InferenceRequest) -> GatewayResult<InferenceResponse>;

    /// Stream one request through the gateway.
    async fn stream(
        &self,
        request: InferenceRequest,
    ) -> GatewayResult<BoxStream<'static, GatewayResult<InferenceChunk>>>;
}

#[cfg(test)]
mod graph_tests {
    use super::*;

    #[test]
    fn pipeline_graph_toml_loads_as_nine_nodes_and_eight_edges() {
        let graph = roko_graph::loader::load_from_str(PIPELINE_GRAPH_TOML).unwrap();
        assert_eq!(graph.metadata.name, "inference-gateway");
        assert_eq!(graph.node_count(), 9);
        assert_eq!(graph.edge_count(), 8);
    }
}
