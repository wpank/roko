//! Probe for Hermes installation and gateway health.
//!
//! Checks:
//! 1. Binary exists on PATH via `<binary> --version`.
//! 2. Parses the version string.
//! 3. Optionally checks gateway health via HTTP GET `/health`.

use std::path::PathBuf;
use std::time::Duration;

use crate::harness::{HarnessProbe, ProbeError};
use crate::process::ResourceLimits;
use roko_core::config::provider::ProviderNetworkPolicy;

use crate::harness::probe_runner::run_probe_command;

/// Probe the Hermes installation.
///
/// Checks whether the `hermes` binary is reachable and, optionally,
/// whether the gateway HTTP server is healthy.
///
/// # Arguments
///
/// * `binary` -- Path or name of the Hermes binary (e.g., `"hermes"`).
/// * `gateway_endpoint` -- Optional gateway base URL to health-check
///   (e.g., `"http://localhost:8642"`).
pub async fn probe_hermes(
    binary: &str,
    gateway_endpoint: Option<&str>,
) -> Result<HarnessProbe, ProbeError> {
    probe_hermes_with_limits(binary, gateway_endpoint, None, Duration::from_secs(3)).await
}

/// Probe Hermes using the configured provider process limits and timeout.
pub async fn probe_hermes_with_limits(
    binary: &str,
    gateway_endpoint: Option<&str>,
    limits: Option<&ResourceLimits>,
    timeout: Duration,
) -> Result<HarnessProbe, ProbeError> {
    match tokio::time::timeout(
        timeout,
        probe_hermes_inner(binary, gateway_endpoint, limits, timeout),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => Err(ProbeError::Timeout(timeout)),
    }
}

async fn probe_hermes_inner(
    binary: &str,
    gateway_endpoint: Option<&str>,
    limits: Option<&ResourceLimits>,
    timeout: Duration,
) -> Result<HarnessProbe, ProbeError> {
    // Step 1: Check binary exists via `<binary> --version`.
    let output = run_probe_command(binary, ["--version"], limits, timeout)
        .await
        .map_err(|error| match error {
            ProbeError::Io(error) => ProbeError::Io(std::io::Error::new(
                error.kind(),
                format!("failed to run `{binary} --version`: {error}"),
            )),
            other => other,
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Ok(HarnessProbe::not_installed(format!(
            "`{binary} --version` exited with {}: {stderr}",
            output.status
        )));
    }

    // Step 2: Parse version string.
    let version_raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let version = version_raw
        .strip_prefix("hermes ")
        .or_else(|| version_raw.strip_prefix("hermes-agent "))
        .unwrap_or(&version_raw)
        .to_string();

    // Resolve absolute binary path via `which`.
    let binary_path = resolve_binary_path(binary, limits, timeout).await;

    let mut probe = HarnessProbe::healthy(
        &version,
        binary_path.unwrap_or_else(|| PathBuf::from(binary)),
    );

    // Step 3: Optionally check gateway health.
    if let Some(endpoint) = gateway_endpoint
        && !limits.is_some_and(|limits| limits.network == ProviderNetworkPolicy::Deny)
    {
        let health_url = format!("{}/health", endpoint.trim_end_matches('/'));
        match reqwest::Client::new()
            .get(&health_url)
            .timeout(timeout.min(Duration::from_secs(3)))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                probe.notes.push("gateway healthy".to_string());
            }
            Ok(resp) => {
                probe
                    .notes
                    .push(format!("gateway /health returned HTTP {}", resp.status()));
            }
            Err(e) => {
                probe
                    .notes
                    .push(format!("gateway /health unreachable: {e}"));
            }
        }
    } else if gateway_endpoint.is_some() {
        probe
            .notes
            .push("gateway health probe skipped by network-deny policy".to_string());
    }

    Ok(probe)
}

/// Try to resolve the absolute path of a binary using `which`.
async fn resolve_binary_path(
    binary: &str,
    limits: Option<&ResourceLimits>,
    timeout: Duration,
) -> Option<PathBuf> {
    let output = run_probe_command("which", [binary], limits, timeout)
        .await
        .ok()?;

    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Some(PathBuf::from(path));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn probe_with_nonexistent_binary() {
        // A binary that almost certainly does not exist.
        let result = probe_hermes("hermes_nonexistent_binary_12345", None).await;

        // Should return an Io error because the binary doesn't exist.
        assert!(result.is_err());
        match result {
            Err(ProbeError::Io(_)) => {} // expected
            other => panic!("expected ProbeError::Io, got {other:?}"),
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn configured_probe_timeout_bounds_version_subprocess() {
        use std::os::unix::fs::PermissionsExt;

        let directory = tempfile::tempdir().expect("temporary directory");
        let binary = directory.path().join("slow-hermes");
        std::fs::write(&binary, "#!/bin/sh\nsleep 5\n").expect("write probe binary");
        let mut permissions = std::fs::metadata(&binary)
            .expect("probe metadata")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&binary, permissions).expect("make probe executable");
        let timeout = Duration::from_millis(20);

        let result = probe_hermes_with_limits(&binary.to_string_lossy(), None, None, timeout).await;

        assert!(matches!(result, Err(ProbeError::Timeout(value)) if value == timeout));
    }
}
