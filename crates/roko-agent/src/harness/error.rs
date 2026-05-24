//! Error types for the harness adapter layer.

use std::fmt;
use std::time::Duration;

/// Errors from harness operations.
///
/// Covers subprocess lifecycle (PR-1) and service lifecycle (PR-3).
#[derive(Debug)]
pub enum HarnessError {
    // ---- Subprocess lifecycle (PR-1) ----
    /// Harness process exited with an error code.
    ProcessExit {
        /// Exit code returned by the process, or `None` if it was killed by a signal.
        code: Option<i32>,
        /// Captured stderr output from the process.
        stderr: String,
    },
    /// Harness operation timed out.
    Timeout {
        /// How long the operation actually ran before being killed.
        elapsed: Duration,
        /// The configured timeout limit that was exceeded.
        configured: Duration,
    },
    /// Lost connection to the harness mid-operation.
    Disconnected {
        /// Identifier of the harness that disconnected.
        harness_id: String,
        /// Transport layer in use at disconnection (e.g. `"stdio"`, `"http"`).
        transport: String,
        /// Whether the disconnect occurred in the middle of a tool-loop turn.
        mid_turn: bool,
        /// Wall-clock time elapsed before the disconnect was detected.
        elapsed: Duration,
        /// Total bytes received from the harness before disconnection.
        bytes_received: u64,
        /// Human-readable description of the disconnection cause.
        reason: String,
    },
    /// I/O error during harness operation.
    Io(std::io::Error),
    /// Protocol-level error (malformed response, unexpected frame, etc.).
    Protocol(String),
    /// Harness runtime version is incompatible.
    RuntimeVersion(String),
    /// Configuration error.
    Config(String),
    // ---- Service lifecycle (PR-3) ----
    /// Service start timed out.
    ServiceStartTimeout {
        /// How long the startup probe ran before giving up.
        elapsed: Duration,
        /// The configured startup-timeout limit that was exceeded.
        configured: Duration,
    },
    /// Healthcheck failed.
    ServiceUnhealthy(String),
    /// Authentication error.
    Auth(String),
}

impl fmt::Display for HarnessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProcessExit { code, stderr } => {
                write!(f, "process exited with code {code:?}: {stderr}")
            }
            Self::Timeout {
                elapsed,
                configured,
            } => {
                write!(
                    f,
                    "timed out after {elapsed:?} (configured: {configured:?})"
                )
            }
            Self::Disconnected {
                harness_id, reason, ..
            } => {
                write!(f, "disconnected from {harness_id}: {reason}")
            }
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Protocol(msg) => write!(f, "protocol error: {msg}"),
            Self::RuntimeVersion(msg) => write!(f, "runtime version: {msg}"),
            Self::Config(msg) => write!(f, "config: {msg}"),
            Self::ServiceStartTimeout {
                elapsed,
                configured,
            } => {
                write!(
                    f,
                    "service start timed out after {elapsed:?} (configured: {configured:?})"
                )
            }
            Self::ServiceUnhealthy(msg) => write!(f, "healthcheck failed: {msg}"),
            Self::Auth(msg) => write!(f, "auth: {msg}"),
        }
    }
}

impl std::error::Error for HarnessError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for HarnessError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harness_error_display_process_exit() {
        let err = HarnessError::ProcessExit {
            code: Some(1),
            stderr: "something failed".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("exited with code"));
        assert!(msg.contains("Some(1)"));
    }

    #[test]
    fn harness_error_display_timeout() {
        let err = HarnessError::Timeout {
            elapsed: Duration::from_secs(30),
            configured: Duration::from_secs(30),
        };
        assert!(err.to_string().contains("timed out"));
    }

    #[test]
    fn harness_error_display_disconnected() {
        let err = HarnessError::Disconnected {
            harness_id: "claude-cli".to_string(),
            transport: "stdio".to_string(),
            mid_turn: true,
            elapsed: Duration::from_secs(5),
            bytes_received: 1024,
            reason: "pipe broken".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("disconnected"));
        assert!(msg.contains("pipe broken"));
    }

    #[test]
    fn harness_error_display_io() {
        let err = HarnessError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file not found",
        ));
        assert!(err.to_string().contains("I/O error"));
    }

    #[test]
    fn harness_error_display_protocol() {
        let err = HarnessError::Protocol("bad json".to_string());
        assert!(err.to_string().contains("protocol error"));
        assert!(err.to_string().contains("bad json"));
    }

    #[test]
    fn harness_error_display_service_variants() {
        assert_eq!(
            HarnessError::ServiceStartTimeout {
                elapsed: Duration::from_secs(5),
                configured: Duration::from_secs(30),
            }
            .to_string(),
            "service start timed out after 5s (configured: 30s)"
        );
        assert_eq!(
            HarnessError::ServiceUnhealthy("connection refused".to_string()).to_string(),
            "healthcheck failed: connection refused"
        );
        assert_eq!(
            HarnessError::Auth("bad token".to_string()).to_string(),
            "auth: bad token"
        );
    }

    #[test]
    fn harness_error_display_all_variants() {
        // ProcessExit
        let err = HarnessError::ProcessExit {
            code: Some(42),
            stderr: "segfault".to_string(),
        };
        assert!(err.to_string().contains("42"));
        assert!(err.to_string().contains("segfault"));

        // ProcessExit with None code
        let err = HarnessError::ProcessExit {
            code: None,
            stderr: "killed".to_string(),
        };
        assert!(err.to_string().contains("None"));
        assert!(err.to_string().contains("killed"));

        // Timeout
        let err = HarnessError::Timeout {
            elapsed: Duration::from_millis(5500),
            configured: Duration::from_secs(10),
        };
        let msg = err.to_string();
        assert!(msg.contains("timed out"));
        assert!(msg.contains("5.5s"));
        assert!(msg.contains("10s"));

        // Disconnected
        let err = HarnessError::Disconnected {
            harness_id: "hermes".to_string(),
            transport: "http_openai".to_string(),
            mid_turn: false,
            elapsed: Duration::from_secs(2),
            bytes_received: 0,
            reason: "EOF".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("disconnected from hermes"));
        assert!(msg.contains("EOF"));

        // Io
        let err = HarnessError::Io(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "broken pipe",
        ));
        assert!(err.to_string().contains("I/O error"));
        assert!(err.to_string().contains("broken pipe"));

        // Protocol
        let err = HarnessError::Protocol("unexpected EOF in JSON frame".to_string());
        assert!(err.to_string().contains("protocol error"));
        assert!(err.to_string().contains("unexpected EOF"));

        // RuntimeVersion
        let err = HarnessError::RuntimeVersion("need >=2.0, got 1.3".to_string());
        assert!(err.to_string().contains("runtime version"));
        assert!(err.to_string().contains("need >=2.0"));

        // Config
        let err = HarnessError::Config("missing api_key_env".to_string());
        assert!(err.to_string().contains("config"));
        assert!(err.to_string().contains("missing api_key_env"));

        // ServiceStartTimeout
        let err = HarnessError::ServiceStartTimeout {
            elapsed: Duration::from_secs(30),
            configured: Duration::from_secs(30),
        };
        assert!(err.to_string().contains("service start timed out"));

        // ServiceUnhealthy
        let err = HarnessError::ServiceUnhealthy("port 8642 refused".to_string());
        assert!(err.to_string().contains("healthcheck failed"));

        // Auth
        let err = HarnessError::Auth("expired token".to_string());
        assert!(err.to_string().contains("auth"));
        assert!(err.to_string().contains("expired token"));
    }

    #[test]
    fn harness_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let harness_err: HarnessError = io_err.into();

        // Should be the Io variant
        assert!(matches!(harness_err, HarnessError::Io(_)));
        assert!(harness_err.to_string().contains("access denied"));

        // source() should return the inner io::Error
        use std::error::Error;
        let source = harness_err.source().expect("should have a source");
        assert!(source.to_string().contains("access denied"));
    }
}
