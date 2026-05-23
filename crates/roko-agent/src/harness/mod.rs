//! Shared primitives for agent harness adapters.
//!
//! A "harness" is a long-lived external program that wraps a backend,
//! runs its own tool loop, and exposes a higher-level interface (HTTP,
//! ACP, MCP, custom CLI). Examples: `claude`, `cursor agent`, `hermes`,
//! `openclaw`.
//!
//! This module provides:
//!
//! - [`HarnessAdapter`] -- the trait that every harness implementation
//!   satisfies. Sub-trait of [`crate::Agent`].
//! - [`HarnessService`] -- lifecycle manager for a harness daemon
//!   (start/stop/status/healthcheck), separate from the adapter.
//! - [`HarnessRegistry`] -- central lookup + probe cache.
//! - [`HarnessCapabilities`], [`TransportFlavor`] -- capability negotiation.
//! - [`ChildProcessRunner`] -- shared subprocess lifecycle manager.
//! - [`EventParser`], [`HarnessEvent`] -- protocol-specific line parsing.
//!
//! # Adding a new harness
//!
//! 1. Add a `crates/roko-agent/src/<name>/` directory.
//! 2. Implement `impl Agent + HarnessAdapter` for at least one
//!    `TransportFlavor`.
//! 3. Optionally implement `HarnessService` for daemon lifecycle.
//! 4. Add a variant to [`roko_core::agent::ProviderKind`].
//! 5. Register the constructor in [`HarnessRegistry::from_config`].

// PR-1: subprocess extraction
pub mod child_process_runner;
pub mod claude_parser;
pub mod error;
pub mod events;

// PR-2: shared ACP stdio client
pub mod acp_client;

// PR-3: foundation traits + registry
pub mod capability;
pub mod registry;
pub mod service;

// PR-1 re-exports
pub use child_process_runner::{ChildProcessRunner, ScrubbedEnv, SpawnedChild};
pub use claude_parser::ClaudeStreamJsonParser;
pub use error::HarnessError;
pub use events::{EventParser, HarnessEvent, harness_events_to_agent_result};

// PR-2 re-exports
pub use acp_client::{
    AcpError, AcpEvent, AcpInitResponse, AcpNotification, AcpPromptPayload, AcpPromptResult,
    AcpStdioClient, AcpStdioConfig, NewSessionOpts, SessionId,
};

// PR-3 re-exports
pub use capability::{
    CancelMode, CapabilityMismatch, CliOutput, HarnessCapabilities, HarnessTaskRequirements,
    McpMode, OneShotMode, SessionResumeMode, StreamingMode, ToolInjection, TransportFlavor,
    validate_for_task,
};
pub use registry::{HarnessRegistry, RegistryConfig, RegistryError};
pub use service::{BearerAuth, HarnessService, HealthReport, ServiceEndpoint, ServiceStatus};

use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::time::Duration;

// ---- HarnessAdapter trait --------------------------------------------------

/// A harness adapter is an [`Agent`](crate::Agent) that wraps an
/// external long-lived process or daemon.
///
/// Implementations carry **two extra things** beyond a plain `Agent`:
///
/// 1. Metadata -- capabilities, transport, harness-id -- that the
///    orchestrator consults at dispatch time.
/// 2. A pointer to an optional [`HarnessService`] that manages the
///    underlying daemon's lifecycle.
#[async_trait]
pub trait HarnessAdapter: super::Agent {
    /// Stable identifier. Appears in `roko.toml` as the provider id key
    /// and in episode logs / metrics.
    fn harness_id(&self) -> &str;

    /// Which transport this adapter speaks.
    fn transport(&self) -> TransportFlavor;

    /// Static description of what the harness can do at this transport.
    fn capabilities(&self) -> &HarnessCapabilities;

    /// Cheap install / auth / version probe.
    ///
    /// Must return within ~250ms on healthy installs.
    async fn probe(&self) -> Result<(), ProbeError>;

    /// Where the harness stores its state on disk.
    fn state_dir(&self) -> Option<&Path> {
        None
    }

    /// Lifecycle manager for the underlying daemon, if any.
    fn service(&self) -> Option<&dyn HarnessService> {
        None
    }
}

// ---- HarnessProbe ----------------------------------------------------------

/// Probe result metadata for a harness adapter.
#[derive(Clone, Debug)]
pub struct HarnessProbe {
    /// Is the harness binary discoverable on PATH?
    pub installed: bool,
    /// Version string from the harness, if available.
    pub version: Option<String>,
    /// Absolute path to the harness binary, if discovered.
    pub binary_path: Option<PathBuf>,
    /// Is the harness configured and authenticated?
    pub auth_ok: bool,
    /// If the adapter has a service, current daemon status.
    pub service_status: Option<ServiceStatus>,
    /// Operator-facing diagnostics, one line each.
    pub notes: Vec<String>,
}

impl HarnessProbe {
    /// Create a probe result for a harness that is not installed.
    #[must_use]
    pub fn not_installed(note: impl Into<String>) -> Self {
        Self {
            installed: false,
            version: None,
            binary_path: None,
            auth_ok: false,
            service_status: None,
            notes: vec![note.into()],
        }
    }

    /// Create a probe result for a healthy harness.
    #[must_use]
    pub fn healthy(version: impl Into<String>, binary_path: PathBuf) -> Self {
        Self {
            installed: true,
            version: Some(version.into()),
            binary_path: Some(binary_path),
            auth_ok: true,
            service_status: None,
            notes: Vec::new(),
        }
    }

    /// Whether the probe indicates the harness is ready for dispatch.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.installed && self.auth_ok
    }
}

// ---- ProbeError ------------------------------------------------------------

/// Errors from [`HarnessAdapter::probe`].
#[derive(Debug)]
pub enum ProbeError {
    /// Probe timed out.
    Timeout(Duration),
    /// I/O error during probe.
    Io(std::io::Error),
    /// Malformed probe output.
    Parse(String),
}

impl std::fmt::Display for ProbeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout(d) => write!(f, "probe timed out after {d:?}"),
            Self::Io(e) => write!(f, "io error: {e}"),
            Self::Parse(msg) => write!(f, "malformed probe output: {msg}"),
        }
    }
}

impl std::error::Error for ProbeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for ProbeError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_not_installed_has_correct_fields() {
        let probe = HarnessProbe::not_installed("hermes not found on PATH");
        assert!(!probe.installed);
        assert!(probe.version.is_none());
        assert!(probe.binary_path.is_none());
        assert!(!probe.auth_ok);
        assert!(probe.service_status.is_none());
        assert_eq!(probe.notes.len(), 1);
        assert_eq!(probe.notes[0], "hermes not found on PATH");
        assert!(!probe.is_ready());
    }

    #[test]
    fn probe_healthy_has_correct_fields() {
        let probe = HarnessProbe::healthy("1.0.0", PathBuf::from("/usr/local/bin/hermes"));
        assert!(probe.installed);
        assert_eq!(probe.version.as_deref(), Some("1.0.0"));
        assert_eq!(
            probe.binary_path.as_ref().map(|p| p.to_str().unwrap()),
            Some("/usr/local/bin/hermes")
        );
        assert!(probe.auth_ok);
        assert!(probe.notes.is_empty());
        assert!(probe.is_ready());
    }

    #[test]
    fn probe_error_display() {
        let err = ProbeError::Timeout(Duration::from_millis(250));
        assert_eq!(err.to_string(), "probe timed out after 250ms");

        let err = ProbeError::Parse("unexpected output".to_string());
        assert_eq!(err.to_string(), "malformed probe output: unexpected output");
    }
}
