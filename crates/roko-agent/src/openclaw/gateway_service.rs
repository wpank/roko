//! Lifecycle manager for the OpenClaw gateway daemon.
//!
//! The OpenClaw gateway (`openclaw gateway`) is a long-running process
//! that listens on WebSocket port 18789 by default. It provides
//! multi-channel messaging integrations and acts as the backend for
//! the ACP bridge.
//!
//! This service is optional for the infer transport (which can run
//! in `--local` mode without a gateway), but required for the ACP
//! transport (PR-7).
//!
//! ## Lifecycle
//!
//! - `start()`: Spawn `openclaw gateway` as a background process.
//!   Wait up to 15 seconds for the WS port to respond.
//! - `stop()`: Send SIGTERM to the gateway process. Wait up to 10
//!   seconds for graceful shutdown, then SIGKILL.
//! - `healthcheck()`: Check if the gateway responds on its WS port
//!   via TCP connect.
//! - `status()`: Report whether the gateway process is running.
//! - `pid()`: Return the PID from the PID file, if available.
//!
//! ## Process management
//!
//! The gateway writes its PID to `~/.openclaw/gateway.pid`. This
//! service reads the PID file for status checks and stop operations.

use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use crate::harness::error::HarnessError;
use crate::harness::service::{HarnessService, ServiceStatus};

/// Default WebSocket port for the OpenClaw gateway.
const DEFAULT_WS_PORT: u16 = 18789;

/// Maximum time to wait for gateway startup.
const START_TIMEOUT: Duration = Duration::from_secs(15);

/// Maximum time to wait for graceful shutdown.
const STOP_TIMEOUT: Duration = Duration::from_secs(10);

/// Health check interval during startup polling.
const HEALTH_POLL_INTERVAL: Duration = Duration::from_millis(500);

/// OpenClaw gateway lifecycle manager.
///
/// Manages `openclaw gateway` as a background process. Spawns the
/// gateway on `start()`, sends SIGTERM on `stop()`, and checks WS
/// port health for `healthcheck()`.
pub struct OpenClawGatewayService {
    /// Path to the `openclaw` binary.
    binary: std::ffi::OsString,

    /// WebSocket port the gateway listens on.
    ws_port: u16,

    /// Path to the PID file (default: `~/.openclaw/gateway.pid`).
    pid_file: PathBuf,

    /// Cached PID of the spawned gateway process.
    /// Updated on `start()` and `stop()`. Read by `pid()`.
    cached_pid: AtomicU32,

    /// Cached endpoint string for `endpoint()` to return `Option<&str>`.
    endpoint_str: String,
}

impl OpenClawGatewayService {
    /// Create a new gateway service with default settings.
    pub fn new(binary: impl Into<std::ffi::OsString>) -> Self {
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| "/tmp".into());
        let ws_port = DEFAULT_WS_PORT;
        Self {
            binary: binary.into(),
            ws_port,
            pid_file: home.join(".openclaw").join("gateway.pid"),
            cached_pid: AtomicU32::new(0),
            endpoint_str: format!("ws://127.0.0.1:{ws_port}"),
        }
    }

    /// Set the WebSocket port.
    pub fn with_port(mut self, port: u16) -> Self {
        self.ws_port = port;
        self.endpoint_str = format!("ws://127.0.0.1:{port}");
        self
    }

    /// Set the PID file path.
    pub fn with_pid_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.pid_file = path.into();
        self
    }

    /// Read the PID from the PID file, if it exists and contains a
    /// valid PID of a running process.
    #[allow(unsafe_code)]
    fn read_pid_file(&self) -> Option<u32> {
        let content = std::fs::read_to_string(&self.pid_file).ok()?;
        let pid: u32 = content.trim().parse().ok()?;

        // Check if the process is actually running.
        #[cfg(unix)]
        {
            // SAFETY: kill(pid, 0) checks process existence without
            // sending a signal. This is a standard POSIX idiom.
            let alive = unsafe { libc::kill(pid as i32, 0) } == 0;
            if !alive {
                return None;
            }
        }

        Some(pid)
    }

    /// Check gateway health by attempting a TCP connection to the
    /// WebSocket port.
    ///
    /// A full WebSocket handshake is not performed -- a successful
    /// TCP connect is sufficient to confirm the gateway is listening.
    async fn check_port_open(&self) -> bool {
        let addr = format!("127.0.0.1:{}", self.ws_port);
        tokio::net::TcpStream::connect(&addr).await.is_ok()
    }

    /// Poll until the gateway is healthy or timeout is reached.
    async fn wait_for_healthy(&self, timeout: Duration) -> Result<(), HarnessError> {
        let start = Instant::now();
        loop {
            if self.check_port_open().await {
                return Ok(());
            }
            if start.elapsed() >= timeout {
                return Err(HarnessError::Timeout {
                    elapsed: start.elapsed(),
                    configured: timeout,
                });
            }
            tokio::time::sleep(HEALTH_POLL_INTERVAL).await;
        }
    }
}

#[async_trait]
impl HarnessService for OpenClawGatewayService {
    fn service_name(&self) -> &str {
        "openclaw-gateway"
    }

    /// Report whether the gateway process is running.
    ///
    /// Checks the PID file first, then the WS port. Returns:
    /// - `ServiceStatus::Running` if PID is alive and port is open
    /// - `ServiceStatus::Starting` if PID is alive but port not yet open
    /// - `ServiceStatus::Stopped` if no PID and port is closed
    /// - `ServiceStatus::Running` (pid=None) if port is open but no PID
    ///   file (externally managed gateway)
    async fn status(&self) -> ServiceStatus {
        if let Some(pid) = self.read_pid_file() {
            self.cached_pid.store(pid, Ordering::Relaxed);
            if self.check_port_open().await {
                ServiceStatus::Running
            } else {
                ServiceStatus::Starting
            }
        } else if self.check_port_open().await {
            // Port is open but no PID file -- externally managed gateway.
            ServiceStatus::Running
        } else {
            ServiceStatus::Stopped
        }
    }

    /// Spawn `openclaw gateway` as a background process.
    ///
    /// Idempotent: if already running, returns `Ok(())`.
    ///
    /// Steps:
    /// 1. Check if already running (idempotent).
    /// 2. Spawn `openclaw gateway` with stdout/stderr/stdin null.
    /// 3. Detach from parent process group (Unix).
    /// 4. Write PID to `~/.openclaw/gateway.pid`.
    /// 5. Poll WS port until healthy or 15s timeout.
    #[allow(unsafe_code)]
    async fn start(&self) -> Result<(), HarnessError> {
        // Idempotent: if already running, return success.
        if matches!(self.status().await, ServiceStatus::Running) {
            return Ok(());
        }

        // Spawn `openclaw gateway` as a background process.
        let mut cmd = tokio::process::Command::new(&self.binary);
        cmd.arg("gateway");
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::null());
        cmd.stdin(std::process::Stdio::null());

        // Detach from the parent process group so the gateway survives
        // roko's exit.
        #[cfg(unix)]
        {
            #[allow(unused_imports)]
            use std::os::unix::process::CommandExt;
            // SAFETY: setsid() is a standard POSIX call to create a new
            // session and detach from the parent process group.
            unsafe {
                cmd.pre_exec(|| {
                    libc::setsid();
                    Ok(())
                });
            }
        }

        let child = cmd.spawn().map_err(|e| {
            HarnessError::Protocol(format!("failed to start openclaw gateway: {e}"))
        })?;

        // Register the PID.
        if let Some(pid) = child.id() {
            self.cached_pid.store(pid, Ordering::Relaxed);
            if let Some(parent) = self.pid_file.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(&self.pid_file, pid.to_string());
        }

        // Wait for the gateway to become healthy.
        self.wait_for_healthy(START_TIMEOUT).await?;

        Ok(())
    }

    /// Stop the gateway process via SIGTERM, escalating to SIGKILL
    /// after 10 seconds.
    ///
    /// Idempotent: if already stopped, returns `Ok(())`.
    ///
    /// Steps:
    /// 1. Read PID from file. If no PID and port closed, return Ok.
    /// 2. Send SIGTERM to the PID.
    /// 3. Poll for up to 10s waiting for process exit.
    /// 4. If still alive after 10s, send SIGKILL.
    /// 5. Clean up PID file.
    #[allow(unsafe_code)]
    async fn stop(&self) -> Result<(), HarnessError> {
        let pid = match self.read_pid_file() {
            Some(pid) => pid,
            None => {
                if !self.check_port_open().await {
                    return Ok(()); // Already stopped.
                }
                return Err(HarnessError::Protocol(
                    "gateway is running but PID file is missing; \
                     stop it manually or remove the stale process"
                        .to_string(),
                ));
            }
        };

        // Send SIGTERM.
        #[cfg(unix)]
        {
            // SAFETY: Sending SIGTERM to a known PID. This is a
            // standard process management operation.
            unsafe {
                libc::kill(pid as i32, libc::SIGTERM);
            }
        }

        // Wait for graceful shutdown.
        let start = Instant::now();
        loop {
            if self.read_pid_file().is_none() || !self.check_port_open().await {
                break;
            }
            if start.elapsed() >= STOP_TIMEOUT {
                // Escalate to SIGKILL.
                #[cfg(unix)]
                {
                    // SAFETY: Sending SIGKILL after SIGTERM timeout.
                    unsafe {
                        libc::kill(pid as i32, libc::SIGKILL);
                    }
                }
                break;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }

        // Clean up PID file.
        let _ = std::fs::remove_file(&self.pid_file);
        self.cached_pid.store(0, Ordering::Relaxed);

        Ok(())
    }

    /// Check if the gateway is healthy by attempting a TCP connection
    /// to the WS port.
    ///
    /// Returns `Ok(())` if the port is open, `Err(HarnessError)` if not.
    async fn healthcheck(&self) -> Result<(), HarnessError> {
        if self.check_port_open().await {
            Ok(())
        } else {
            Err(HarnessError::Protocol(format!(
                "gateway not responding on ws://127.0.0.1:{}",
                self.ws_port
            )))
        }
    }

    /// Return the WS endpoint URL as a string.
    ///
    /// Always returns `Some("ws://127.0.0.1:<port>")`.
    fn endpoint(&self) -> Option<&str> {
        Some(&self.endpoint_str)
    }

    /// Return the PID of the gateway process, if known.
    ///
    /// Reads from the cached PID first, falls back to the PID file.
    fn pid(&self) -> Option<u32> {
        let cached = self.cached_pid.load(Ordering::Relaxed);
        if cached != 0 {
            return Some(cached);
        }
        self.read_pid_file()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_service_config() {
        let svc = OpenClawGatewayService::new("openclaw");
        assert_eq!(svc.ws_port, DEFAULT_WS_PORT);
        assert!(svc.pid_file.ends_with("gateway.pid"));
    }

    #[test]
    fn service_name() {
        let svc = OpenClawGatewayService::new("openclaw");
        assert_eq!(svc.service_name(), "openclaw-gateway");
    }

    #[test]
    fn with_port() {
        let svc = OpenClawGatewayService::new("openclaw").with_port(9999);
        assert_eq!(svc.ws_port, 9999);
        assert_eq!(svc.endpoint(), Some("ws://127.0.0.1:9999"));
    }

    #[test]
    fn endpoint_returns_ws_url() {
        let svc = OpenClawGatewayService::new("openclaw");
        let ep = svc.endpoint().unwrap();
        assert!(ep.starts_with("ws://127.0.0.1:"));
        assert!(ep.contains(&DEFAULT_WS_PORT.to_string()));
    }

    #[test]
    fn pid_returns_none_when_no_process() {
        let svc = OpenClawGatewayService::new("openclaw")
            .with_pid_file("/tmp/roko-test-nonexistent-gateway.pid");
        assert!(svc.pid().is_none());
    }

    #[tokio::test]
    async fn status_returns_stopped_when_nothing_running() {
        // Use a non-existent PID file and a port unlikely to be in use.
        let svc = OpenClawGatewayService::new("openclaw")
            .with_port(59999)
            .with_pid_file("/tmp/roko-test-nonexistent-gateway.pid");
        let status = svc.status().await;
        assert!(matches!(status, ServiceStatus::Stopped));
    }

    #[tokio::test]
    async fn stop_is_idempotent_when_already_stopped() {
        let svc = OpenClawGatewayService::new("openclaw")
            .with_port(59998)
            .with_pid_file("/tmp/roko-test-nonexistent-gateway-2.pid");
        let result = svc.stop().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn healthcheck_fails_when_port_closed() {
        let svc = OpenClawGatewayService::new("openclaw").with_port(59997);
        let result = svc.healthcheck().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn start_with_nonexistent_binary_returns_error() {
        let svc =
            OpenClawGatewayService::new("/nonexistent/path/to/openclaw-binary-that-does-not-exist")
                .with_port(59996)
                .with_pid_file("/tmp/roko-test-nonexistent-gw-start.pid");
        let result = svc.start().await;
        assert!(
            result.is_err(),
            "start() should fail when the binary doesn't exist"
        );
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("failed to start openclaw gateway"),
            "error should mention start failure, got: {err_msg}"
        );
    }

    #[tokio::test]
    async fn stop_idempotent_called_twice() {
        let svc = OpenClawGatewayService::new("openclaw")
            .with_port(59995)
            .with_pid_file("/tmp/roko-test-nonexistent-gw-stop2.pid");
        // Both calls should succeed when nothing is running.
        let r1 = svc.stop().await;
        let r2 = svc.stop().await;
        assert!(r1.is_ok(), "first stop() should succeed");
        assert!(r2.is_ok(), "second stop() should succeed");
    }

    #[tokio::test]
    async fn healthcheck_error_message_contains_port() {
        let svc = OpenClawGatewayService::new("openclaw").with_port(59994);
        let result = svc.healthcheck().await;
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("59994"),
            "healthcheck error should mention the port, got: {err_msg}"
        );
    }

    #[tokio::test]
    async fn pid_file_reading_with_nonexistent_file() {
        let svc = OpenClawGatewayService::new("openclaw")
            .with_pid_file("/tmp/roko-test-absolutely-nonexistent-pid-file-99999.pid");
        // read_pid_file should return None for a missing file.
        assert!(svc.read_pid_file().is_none());
        // pid() should also return None (falls back to read_pid_file).
        assert!(svc.pid().is_none());
    }

    #[tokio::test]
    async fn pid_file_reading_with_invalid_contents() {
        // Write a PID file with garbage contents.
        let pid_path = "/tmp/roko-test-invalid-pid-contents.pid";
        std::fs::write(pid_path, "not-a-number\n").expect("write test pid file");
        let svc = OpenClawGatewayService::new("openclaw").with_pid_file(pid_path);
        // Should return None because the contents can't be parsed as u32.
        assert!(svc.read_pid_file().is_none());
        // Clean up.
        let _ = std::fs::remove_file(pid_path);
    }

    #[tokio::test]
    async fn pid_file_reading_with_dead_pid() {
        // Write a PID file with a very high PID that almost certainly
        // doesn't correspond to a running process.
        let pid_path = "/tmp/roko-test-dead-pid.pid";
        std::fs::write(pid_path, format!("{}", u32::MAX - 1)).expect("write test pid file");
        let svc = OpenClawGatewayService::new("openclaw").with_pid_file(pid_path);
        // On Unix, kill(pid, 0) should return non-zero for a
        // non-existent process, so read_pid_file returns None.
        assert!(
            svc.read_pid_file().is_none(),
            "read_pid_file should return None for a dead PID"
        );
        // Clean up.
        let _ = std::fs::remove_file(pid_path);
    }

    #[tokio::test]
    async fn status_stopped_when_pid_file_missing_and_port_closed() {
        let svc = OpenClawGatewayService::new("openclaw")
            .with_port(59993)
            .with_pid_file("/tmp/roko-test-status-missing-pid.pid");
        let status = svc.status().await;
        assert_eq!(status, ServiceStatus::Stopped);
    }

    #[tokio::test]
    async fn cached_pid_starts_at_zero() {
        let svc = OpenClawGatewayService::new("openclaw");
        assert_eq!(svc.cached_pid.load(Ordering::Relaxed), 0);
        assert!(svc.pid().is_none());
    }
}
