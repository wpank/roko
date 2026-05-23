//! OpenClaw integration for roko.
//!
//! OpenClaw is a Node.js multi-channel AI gateway with four programmatic
//! surfaces:
//!
//! - `openclaw infer <capability> ... --json` -- stable JSON envelope
//!   for headless inference. **Recommended Tier 2 transport.**
//! - `openclaw acp` -- ACP bridge over stdio to the OpenClaw Gateway.
//! - `openclaw mcp serve` -- MCP server (handled by roko's MCP plumbing).
//! - Gateway WebSocket protocol on port 18789 (opaque; deferred to v2).
//!
//! This module implements the Tier 2 (infer) transport and the gateway
//! service lifecycle. Tier 3 (ACP) is implemented in PR-7.

pub mod config;
pub mod gateway_service;
pub mod infer_agent;
pub mod infer_envelope;
pub mod probe;

// PR-7: ACP adapter
pub mod acp_agent;

pub use config::{ConfigError, OpenClawConfig, OpenClawInferConfig, TransportHint};
pub use gateway_service::OpenClawGatewayService;
pub use infer_agent::OpenClawInferAgent;
pub use infer_envelope::{InferEnvelope, InferError, InferEventParser, InferOutput};

pub use probe::probe_openclaw_infer;

// PR-7 re-exports
pub use acp_agent::{OpenClawAcpAgent, OpenClawAcpConfig};
