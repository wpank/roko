//! Lifecycle management for a harness daemon.
//!
//! [`HarnessService`] manages the lifecycle of a harness's background
//! daemon process (start, stop, status, healthcheck). It is intentionally
//! separate from [`super::HarnessAdapter`] -- services live as long as
//! the daemon does, while adapters are constructed per-dispatch.

use async_trait::async_trait;
use std::fmt;
use std::path::PathBuf;

use super::error::HarnessError;

/// A harness daemon's lifecycle manager.
///
/// Distinct from [`super::HarnessAdapter`] -- services live as long as
/// the daemon does, adapters are constructed per-dispatch. A single
/// service may back multiple adapter instances (one per transport).
#[async_trait]
pub trait HarnessService: Send + Sync {
    /// Stable service name -- appears in logs and `roko doctor`.
    fn service_name(&self) -> &str;

    /// Start the daemon. Idempotent -- succeeds if already running.
    async fn start(&self) -> Result<(), HarnessError>;

    /// Stop the daemon. Idempotent -- succeeds if already stopped.
    async fn stop(&self) -> Result<(), HarnessError>;

    /// Cheap status check.
    async fn status(&self) -> ServiceStatus;

    /// Active health check.
    ///
    /// HTTP services typically hit `/health`; ACP services run an
    /// `initialize` request. Must return within a reasonable timeout
    /// (recommend 5s) or error.
    async fn healthcheck(&self) -> Result<(), HarnessError>;

    /// Where to reach the daemon, if it exposes an endpoint.
    /// `None` for in-process or stdio services.
    fn endpoint(&self) -> Option<&str>;

    /// OS process ID, if known and the daemon is running.
    fn pid(&self) -> Option<u32>;
}

/// Status of a harness daemon.
#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum ServiceStatus {
    /// Daemon is running and responding.
    Running,
    /// Daemon is not running.
    Stopped,
    /// Daemon is starting up (not yet responding to healthchecks).
    Starting,
    /// Status could not be determined.
    Unknown,
}

impl fmt::Display for ServiceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Running => f.write_str("running"),
            Self::Stopped => f.write_str("stopped"),
            Self::Starting => f.write_str("starting"),
            Self::Unknown => f.write_str("unknown"),
        }
    }
}

/// Where to reach a harness daemon.
#[derive(Clone, Debug)]
pub enum ServiceEndpoint {
    /// HTTP API endpoint.
    Http {
        /// Base URL (e.g. `http://127.0.0.1:8642`).
        base_url: String,
        /// Optional bearer token auth.
        auth: Option<BearerAuth>,
    },
    /// WebSocket endpoint.
    WebSocket {
        /// WebSocket URL (e.g. `ws://127.0.0.1:8642/ws`).
        url: String,
        /// Optional bearer token auth.
        auth: Option<BearerAuth>,
    },
    /// Stdio (no network endpoint).
    Stdio,
    /// Unix domain socket.
    Unix {
        /// Path to the socket file.
        socket_path: PathBuf,
    },
}

impl fmt::Display for ServiceEndpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Http { base_url, .. } => write!(f, "http: {base_url}"),
            Self::WebSocket { url, .. } => write!(f, "ws: {url}"),
            Self::Stdio => f.write_str("stdio"),
            Self::Unix { socket_path } => write!(f, "unix: {}", socket_path.display()),
        }
    }
}

/// Bearer token authentication for HTTP/WebSocket endpoints.
#[derive(Clone, Debug)]
pub struct BearerAuth {
    /// Environment variable name holding the token.
    pub token_env: String,
}

/// Result of a healthcheck.
#[derive(Clone, Debug)]
pub struct HealthReport {
    /// Whether the daemon is healthy and ready to serve requests.
    pub healthy: bool,
    /// Round-trip time of the healthcheck in milliseconds.
    pub rtt_ms: u32,
    /// Key-value diagnostic details (surfaced by `roko doctor`).
    pub details: Vec<(String, String)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_status_display() {
        assert_eq!(ServiceStatus::Running.to_string(), "running");
        assert_eq!(ServiceStatus::Stopped.to_string(), "stopped");
        assert_eq!(ServiceStatus::Starting.to_string(), "starting");
        assert_eq!(ServiceStatus::Unknown.to_string(), "unknown");
    }

    #[test]
    fn service_endpoint_display() {
        let ep = ServiceEndpoint::Http {
            base_url: "http://127.0.0.1:8642".to_string(),
            auth: None,
        };
        assert_eq!(ep.to_string(), "http: http://127.0.0.1:8642");
    }

    #[test]
    fn service_status_variants() {
        // Verify all four variants exist, are Copy, and have distinct Display output.
        let variants = [
            ServiceStatus::Running,
            ServiceStatus::Stopped,
            ServiceStatus::Starting,
            ServiceStatus::Unknown,
        ];
        let displays: Vec<String> = variants.iter().map(|v| v.to_string()).collect();
        assert_eq!(displays, vec!["running", "stopped", "starting", "unknown"]);

        // Verify PartialEq works across variants.
        assert_eq!(ServiceStatus::Running, ServiceStatus::Running);
        assert_ne!(ServiceStatus::Running, ServiceStatus::Stopped);

        // Verify Copy (assignment, not move).
        let a = ServiceStatus::Starting;
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn health_report_default_fields() {
        let report = HealthReport {
            healthy: true,
            rtt_ms: 12,
            details: vec![
                ("version".to_string(), "1.2.3".to_string()),
                ("uptime_s".to_string(), "3600".to_string()),
            ],
        };
        assert!(report.healthy);
        assert_eq!(report.rtt_ms, 12);
        assert_eq!(report.details.len(), 2);
        assert_eq!(report.details[0].0, "version");
        assert_eq!(report.details[0].1, "1.2.3");
    }

    #[test]
    fn service_endpoint_display_all_variants() {
        let http = ServiceEndpoint::Http {
            base_url: "http://localhost:9090".to_string(),
            auth: Some(BearerAuth {
                token_env: "MY_TOKEN".to_string(),
            }),
        };
        assert_eq!(http.to_string(), "http: http://localhost:9090");

        let ws = ServiceEndpoint::WebSocket {
            url: "ws://localhost:9090/ws".to_string(),
            auth: None,
        };
        assert_eq!(ws.to_string(), "ws: ws://localhost:9090/ws");

        let stdio = ServiceEndpoint::Stdio;
        assert_eq!(stdio.to_string(), "stdio");

        let unix = ServiceEndpoint::Unix {
            socket_path: PathBuf::from("/tmp/harness.sock"),
        };
        assert_eq!(unix.to_string(), "unix: /tmp/harness.sock");
    }
}
