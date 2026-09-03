//! Shared dispatch factory — owns the long-lived provider semaphores,
//! MCP runtime, rate limiter, and health registry.
//!
//! This is the layer-3 counterpart of `SharedAgentFactory` in `roko-cli`.
//! CLI code will re-export this type after migration (#244/#245).

use std::sync::Arc;

use roko_agent::mcp::McpRuntime;
use roko_agent::provider::{LocalToolRuntime, ProviderSemaphores};
use roko_agent::rate_limit::ProviderRateLimiter;
use roko_learn::cascade_router::CascadeRouter;
use roko_learn::provider_health::ProviderHealthRegistry;

/// Shared, reusable dispatch components constructed once per run.
///
/// Owns the expensive handles (semaphores, MCP, rate limiter, health registry)
/// that should survive across individual task dispatches.
#[derive(Debug)]
pub struct DispatchFactory {
    /// Per-provider concurrency semaphores.
    pub semaphores: Arc<ProviderSemaphores>,
    /// Pre-discovered MCP runtime (tools + clients), if available.
    pub mcp_runtime: Option<Arc<McpRuntime>>,
    /// Declarative plugin tool runtime, if loaded.
    pub local_tool_runtime: Option<Arc<LocalToolRuntime>>,
    /// Per-provider rate limiter shared across concurrent dispatches.
    pub rate_limiter: Arc<ProviderRateLimiter>,
    /// Provider health registry for circuit-breaker state.
    pub health_registry: Arc<ProviderHealthRegistry>,
    /// Cascade router for multi-model routing decisions.
    pub cascade_router: Option<Arc<CascadeRouter>>,
}

impl DispatchFactory {
    /// Create a minimal factory for testing (no MCP, no plugins).
    pub fn for_test() -> Self {
        Self {
            semaphores: Arc::new(ProviderSemaphores::new(&[])),
            mcp_runtime: None,
            local_tool_runtime: None,
            rate_limiter: Arc::new(ProviderRateLimiter::default()),
            health_registry: Arc::new(ProviderHealthRegistry::new()),
            cascade_router: None,
        }
    }
}
