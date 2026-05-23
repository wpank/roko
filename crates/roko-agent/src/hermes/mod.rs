//! Hermes gateway harness adapters.
//!
//! Hermes is a multi-provider AI agent from Nous Research. This module
//! provides adapters for three transport tiers:
//!
//! - [`HermesHttpAgent`] -- Tier 1, OpenAI-compatible HTTP API via the gateway.
//! - `HermesOneShotAgent` -- Tier 2, CLI one-shot via `hermes -z` or `hermes chat -q -Q` (PR-5).
//! - `HermesAcpAgent` -- Tier 3, ACP over stdio via `hermes acp` (PR-5).

pub mod config;
pub mod gateway_service;
pub mod http_adapter;
pub mod probe;
pub mod tool_progress_inspector;

// PR-5: oneshot + ACP adapters
pub mod acp_agent;
pub mod oneshot_agent;

// Re-exports
pub use config::{CrashRecoveryConfig, HermesConfig};
pub use gateway_service::HermesGatewayService;
pub use http_adapter::HermesHttpAgent;
pub use probe::probe_hermes;
pub use tool_progress_inspector::ToolProgressInspector;

// PR-5 re-exports
pub use acp_agent::{HermesAcpAgent, HermesAcpConfig};
pub use oneshot_agent::{HermesFlavor, HermesOneShotAgent, HermesOneShotConfig};
