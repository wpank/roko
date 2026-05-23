//! Lifecycle manager for the Hermes gateway daemon.
//!
//! The Hermes gateway (`hermes -p <profile> gateway run`) is a
//! long-running process that hosts the OpenAI-compatible HTTP API
//! server. This service manages its lifecycle:
//!
//! - `start()`: ensure prerequisites, spawn the gateway, poll for health.
//! - `stop()`: SIGTERM then SIGKILL.
//! - `healthcheck()`: GET /health.
//! - `status()`: check if the PID is alive + healthcheck.

use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use tokio::process::Command;

use crate::harness::{HarnessError, HarnessService, ServiceStatus};

use super::config::HermesConfig;

/// Lifecycle manager for the `hermes gateway` daemon.
pub struct HermesGatewayService {
    config: HermesConfig,
    http: reqwest::Client,
    /// PID of the managed gateway process, if started by this service.
    managed_pid: AtomicU32,
}

impl HermesGatewayService {
    /// Create a new gateway service from config.
    #[must_use]
    pub fn new(config: HermesConfig) -> Self {
        Self {
            config,
            http: crate::provider::shared_http_client(),
            managed_pid: AtomicU32::new(0),
        }
    }

    /// Returns `true` if this service manages the gateway process
    /// (i.e., it was started by `start()`, not by an external systemd unit
    /// or manual `hermes gateway run` invocation).
    #[must_use]
    pub fn is_managed(&self) -> bool {
        self.managed_pid.load(Ordering::Relaxed) != 0
    }

    /// The health endpoint URL.
    fn health_url(&self) -> String {
        let base = self.config.endpoint.trim_end_matches('/');
        format!("{base}/health")
    }

    /// Poll the health endpoint until it returns 200 or the timeout expires.
    async fn wait_for_health(&self, timeout: Duration) -> Result<(), HarnessError> {
        let deadline = tokio::time::Instant::now() + timeout;
        let url = self.health_url();
        let poll_interval = Duration::from_millis(250);

        loop {
            match self
                .http
                .get(&url)
                .timeout(Duration::from_secs(2))
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => return Ok(()),
                _ => {}
            }

            if tokio::time::Instant::now() >= deadline {
                return Err(HarnessError::ServiceStartTimeout {
                    elapsed: timeout,
                    configured: timeout,
                });
            }

            tokio::time::sleep(poll_interval).await;
        }
    }

    /// Check if a PID is alive using `kill(pid, 0)`.
    #[cfg(unix)]
    #[allow(unsafe_code)]
    fn is_pid_alive(pid: u32) -> bool {
        // SAFETY: kill(pid, 0) checks process existence without sending a signal.
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }

    #[cfg(not(unix))]
    fn is_pid_alive(_pid: u32) -> bool {
        // On non-Unix, assume alive if we have a PID.
        true
    }
}

#[async_trait]
impl HarnessService for HermesGatewayService {
    fn service_name(&self) -> &str {
        "hermes-gateway"
    }

    async fn start(&self) -> Result<(), HarnessError> {
        // 1. Check if already running.
        let health_url = self.health_url();
        if let Ok(resp) = self
            .http
            .get(&health_url)
            .timeout(Duration::from_secs(2))
            .send()
            .await
        {
            if resp.status().is_success() {
                tracing::info!("hermes gateway already running and healthy");
                return Ok(());
            }
        }

        // 2. Build the command.
        let mut cmd = Command::new(&self.config.binary);
        cmd.arg("gateway").arg("run");

        // Detach stdin so the gateway doesn't block on terminal input.
        cmd.stdin(std::process::Stdio::null());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        // Set process group so we can kill the entire tree.
        crate::process::group::set_process_group(&mut cmd);

        tracing::info!(
            binary = %self.config.binary,
            "starting hermes gateway"
        );

        let child = cmd.spawn().map_err(|e| {
            HarnessError::Io(std::io::Error::new(
                e.kind(),
                format!("failed to spawn hermes gateway: {e}"),
            ))
        })?;

        let pid = child.id().unwrap_or(0);
        self.managed_pid.store(pid, Ordering::Relaxed);

        // 3. Spawn a task to log stderr.
        if let Some(stderr) = child.stderr {
            tokio::spawn(async move {
                use tokio::io::{AsyncBufReadExt, BufReader};
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    tracing::debug!(target: "hermes-gateway", "{}", line);
                }
            });
        }

        // 4. Wait for health.
        let start_timeout = self.config.crash_recovery.health_timeout;
        self.wait_for_health(start_timeout).await?;

        tracing::info!(pid, "hermes gateway started and healthy");
        Ok(())
    }

    #[allow(unsafe_code)]
    async fn stop(&self) -> Result<(), HarnessError> {
        let pid = self.managed_pid.load(Ordering::Relaxed);
        if pid == 0 {
            tracing::info!("hermes gateway: no managed PID, nothing to stop");
            return Ok(());
        }

        tracing::info!(pid, "stopping hermes gateway");

        // SIGTERM first.
        #[cfg(unix)]
        // SAFETY: Sending SIGTERM to a known PID is standard process management.
        unsafe {
            libc::kill(pid as i32, libc::SIGTERM);
        }

        // Wait for up to 10s for the process to exit.
        let stop_timeout = Duration::from_secs(10);
        let deadline = tokio::time::Instant::now() + stop_timeout;
        loop {
            if !Self::is_pid_alive(pid) {
                break;
            }

            if tokio::time::Instant::now() >= deadline {
                // SIGKILL as last resort.
                #[cfg(unix)]
                {
                    tracing::warn!(pid, "hermes gateway did not stop in time, sending SIGKILL");
                    // SAFETY: Sending SIGKILL after SIGTERM timeout.
                    unsafe {
                        libc::kill(pid as i32, libc::SIGKILL);
                    }
                }
                break;
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        self.managed_pid.store(0, Ordering::Relaxed);
        tracing::info!("hermes gateway stopped");
        Ok(())
    }

    async fn status(&self) -> ServiceStatus {
        let pid = self.managed_pid.load(Ordering::Relaxed);

        // Check if the process is alive.
        if pid != 0 && !Self::is_pid_alive(pid) {
            self.managed_pid.store(0, Ordering::Relaxed);
            return ServiceStatus::Stopped;
        }

        // Try healthcheck to determine actual status.
        match self.healthcheck().await {
            Ok(()) => ServiceStatus::Running,
            Err(_) => {
                if pid != 0 {
                    // PID alive but healthcheck failed -- treat as starting.
                    ServiceStatus::Starting
                } else {
                    ServiceStatus::Stopped
                }
            }
        }
    }

    async fn healthcheck(&self) -> Result<(), HarnessError> {
        // GET /health
        let resp = self
            .http
            .get(self.health_url())
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| HarnessError::ServiceUnhealthy(format!("health request failed: {e}")))?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(HarnessError::ServiceUnhealthy(format!(
                "health check returned HTTP {}",
                resp.status()
            )))
        }
    }

    fn endpoint(&self) -> Option<&str> {
        Some(&self.config.endpoint)
    }

    fn pid(&self) -> Option<u32> {
        let pid = self.managed_pid.load(Ordering::Relaxed);
        if pid != 0 { Some(pid) } else { None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_name_is_hermes_gateway() {
        let config = HermesConfig::default();
        let svc = HermesGatewayService::new(config);
        assert_eq!(svc.service_name(), "hermes-gateway");
    }

    #[test]
    fn endpoint_returns_config_value() {
        let config = HermesConfig {
            endpoint: "http://10.0.0.1:9000".to_string(),
            ..Default::default()
        };
        let svc = HermesGatewayService::new(config);
        assert_eq!(svc.endpoint(), Some("http://10.0.0.1:9000"));
    }

    #[test]
    fn pid_returns_none_when_not_started() {
        let config = HermesConfig::default();
        let svc = HermesGatewayService::new(config);
        assert!(svc.pid().is_none());
    }

    #[test]
    fn is_managed_false_initially() {
        let config = HermesConfig::default();
        let svc = HermesGatewayService::new(config);
        assert!(!svc.is_managed());
    }

    #[test]
    fn health_url_format() {
        let config = HermesConfig {
            endpoint: "http://localhost:8642".to_string(),
            ..Default::default()
        };
        let svc = HermesGatewayService::new(config);
        assert_eq!(svc.health_url(), "http://localhost:8642/health");
    }

    #[test]
    fn health_url_strips_trailing_slash() {
        let config = HermesConfig {
            endpoint: "http://localhost:8642/".to_string(),
            ..Default::default()
        };
        let svc = HermesGatewayService::new(config);
        assert_eq!(svc.health_url(), "http://localhost:8642/health");
    }

    #[tokio::test]
    async fn status_is_stopped_when_no_pid() {
        let config = HermesConfig::default();
        let svc = HermesGatewayService::new(config);
        let status = svc.status().await;
        assert_eq!(status, ServiceStatus::Stopped);
    }
}
