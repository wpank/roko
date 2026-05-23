//! Probe for Hermes installation and gateway health.
//!
//! Checks:
//! 1. Binary exists on PATH via `<binary> --version`.
//! 2. Parses the version string.
//! 3. Optionally checks gateway health via HTTP GET `/health`.

use std::path::PathBuf;

use crate::harness::{HarnessProbe, ProbeError};

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
    // Step 1: Check binary exists via `<binary> --version`.
    let output = tokio::process::Command::new(binary)
        .arg("--version")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .await
        .map_err(|e| {
            ProbeError::Io(std::io::Error::new(
                e.kind(),
                format!("failed to run `{binary} --version`: {e}"),
            ))
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
    let binary_path = resolve_binary_path(binary).await;

    let mut probe = HarnessProbe::healthy(
        &version,
        binary_path.unwrap_or_else(|| PathBuf::from(binary)),
    );

    // Step 3: Optionally check gateway health.
    if let Some(endpoint) = gateway_endpoint {
        let health_url = format!("{}/health", endpoint.trim_end_matches('/'));
        match reqwest::Client::new()
            .get(&health_url)
            .timeout(std::time::Duration::from_secs(3))
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
    }

    Ok(probe)
}

/// Try to resolve the absolute path of a binary using `which`.
async fn resolve_binary_path(binary: &str) -> Option<PathBuf> {
    let output = tokio::process::Command::new("which")
        .arg(binary)
        .output()
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
}
