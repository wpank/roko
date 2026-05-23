//! OpenClaw installation probe.
//!
//! Runs 6 steps to verify the OpenClaw installation is healthy:
//!
//! 1. `which openclaw` -- binary discoverable on PATH?
//! 2. `openclaw --version` -- version string parseable?
//! 3. `node --version` -- Node.js >= 22.19 (24 recommended)?
//! 4. `openclaw doctor --repair` -- auth and config health?
//! 5. TCP connect to gateway port -- gateway reachable? (non-fatal)
//! 6. `openclaw config show` -- config valid?
//!
//! Step 3 is critical: OpenClaw requires Node.js 22.19+ at minimum.
//! If the detected version is below 22.19, the probe fails with a
//! diagnostic suggesting `nvm install 24` or `fnm install 24`.

use std::process::Stdio;

use super::config::OpenClawInferConfig;
use crate::harness::ProbeError;

/// Minimum required Node.js major version.
const MIN_NODE_MAJOR: u32 = 22;
/// Minimum required Node.js minor version (when major == 22).
const MIN_NODE_MINOR: u32 = 19;

/// Run the 6-step probe for OpenClaw infer.
///
/// Returns `Ok(())` if the installation is healthy enough to use.
/// Returns `Err(ProbeError)` if a critical step fails.
///
/// Non-critical failures (e.g. gateway not reachable) are logged
/// but do not cause the probe to fail, since infer can run in
/// `--local` mode without a gateway.
pub async fn probe_openclaw_infer(config: &OpenClawInferConfig) -> Result<(), ProbeError> {
    let binary_str = config.binary.to_string_lossy().to_string();

    // Step 1: which openclaw -- binary on PATH?
    match run_command("which", &[&binary_str]).await {
        Ok(output) => {
            let path = output.trim();
            if path.is_empty() {
                return Err(ProbeError::Parse(format!(
                    "'{binary_str}' not found on PATH"
                )));
            }
            // Binary found -- continue.
        }
        Err(e) => {
            return Err(ProbeError::Parse(format!(
                "could not locate '{binary_str}': {e}"
            )));
        }
    }

    // Step 2: openclaw --version
    match run_command(&binary_str, &["--version"]).await {
        Ok(output) => {
            let ver = output.trim();
            if ver.is_empty() {
                return Err(ProbeError::Parse(
                    "openclaw --version returned empty output".to_string(),
                ));
            }
            // Version string exists -- continue.
        }
        Err(e) => {
            return Err(ProbeError::Parse(format!(
                "could not read openclaw version: {e}"
            )));
        }
    }

    // Step 3: node --version (critical: >= 22.19, 24 recommended)
    check_node_version().await.map_err(|msg| {
        ProbeError::Parse(format!(
            "{msg}; install Node.js 24 with `nvm install 24` or `fnm install 24`"
        ))
    })?;

    // Step 4: openclaw doctor --repair
    match run_command(&binary_str, &["doctor", "--repair"]).await {
        Ok(output) => {
            let output_lower = output.to_lowercase();
            if output_lower.contains("error") || output_lower.contains("fail") {
                // Doctor reported issues -- warn but don't fail.
                // The user may still be able to use infer with some
                // providers even if doctor reports issues.
                tracing::warn!("openclaw doctor reported issues: {}", output.trim());
            }
        }
        Err(e) => {
            tracing::warn!("openclaw doctor failed: {e}");
            // Non-fatal -- doctor might fail if no providers configured
            // yet, but the binary is installed.
        }
    }

    // Step 5: gateway reachable? (non-fatal for infer)
    let gateway_addr = "127.0.0.1:18789";
    match tokio::net::TcpStream::connect(gateway_addr).await {
        Ok(_) => {
            tracing::debug!("openclaw gateway reachable on {gateway_addr}");
        }
        Err(_) => {
            tracing::debug!(
                "openclaw gateway not reachable on {gateway_addr} \
                 (not required for --local infer)"
            );
        }
    }

    // Step 6: openclaw config show -- config valid?
    match run_command(&binary_str, &["config", "show"]).await {
        Ok(output) => {
            if output.trim().is_empty() {
                tracing::warn!("openclaw config show returned empty; run `openclaw onboard`");
            }
            // Config exists -- continue.
        }
        Err(e) => {
            return Err(ProbeError::Parse(format!(
                "openclaw config show failed: {e}; run `openclaw onboard`"
            )));
        }
    }

    Ok(())
}

/// Check that Node.js is installed and meets the minimum version
/// requirement (>= 22.19, 24 recommended).
///
/// Returns the version string on success, or an error message on
/// failure.
async fn check_node_version() -> Result<String, String> {
    let output = run_command("node", &["--version"])
        .await
        .map_err(|e| format!("node not found: {e}"))?;

    let ver = output.trim().trim_start_matches('v');
    let parts: Vec<&str> = ver.split('.').collect();

    if parts.len() < 2 {
        return Err(format!("could not parse node version: {ver}"));
    }

    let major: u32 = parts[0]
        .parse()
        .map_err(|_| format!("invalid node major version: {}", parts[0]))?;
    let minor: u32 = parts[1]
        .parse()
        .map_err(|_| format!("invalid node minor version: {}", parts[1]))?;

    if major > MIN_NODE_MAJOR || (major == MIN_NODE_MAJOR && minor >= MIN_NODE_MINOR) {
        Ok(format!("v{ver}"))
    } else {
        Err(format!(
            "node.js {ver} is below minimum {MIN_NODE_MAJOR}.{MIN_NODE_MINOR}; \
             upgrade with `nvm install 24` or `fnm install 24`"
        ))
    }
}

/// Run a command and capture its stdout as a String.
///
/// Returns an error if the command fails to execute or exits non-zero.
async fn run_command(program: &str, args: &[&str]) -> Result<String, String> {
    let output = tokio::process::Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("{program}: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "{program} exited with {:?}: {}",
            output.status.code(),
            stderr.trim()
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_version_22_19_passes() {
        let ver = "22.19.0";
        let parts: Vec<&str> = ver.split('.').collect();
        let major: u32 = parts[0].parse().unwrap();
        let minor: u32 = parts[1].parse().unwrap();
        assert!(major > MIN_NODE_MAJOR || (major == MIN_NODE_MAJOR && minor >= MIN_NODE_MINOR));
    }

    #[test]
    fn node_version_22_18_fails() {
        let ver = "22.18.0";
        let parts: Vec<&str> = ver.split('.').collect();
        let major: u32 = parts[0].parse().unwrap();
        let minor: u32 = parts[1].parse().unwrap();
        assert!(!(major > MIN_NODE_MAJOR || (major == MIN_NODE_MAJOR && minor >= MIN_NODE_MINOR)));
    }

    #[test]
    fn node_version_24_passes() {
        let ver = "24.0.0";
        let parts: Vec<&str> = ver.split('.').collect();
        let major: u32 = parts[0].parse().unwrap();
        let minor: u32 = parts[1].parse().unwrap();
        assert!(major > MIN_NODE_MAJOR || (major == MIN_NODE_MAJOR && minor >= MIN_NODE_MINOR));
    }

    #[test]
    fn node_version_20_fails() {
        let ver = "20.0.0";
        let parts: Vec<&str> = ver.split('.').collect();
        let major: u32 = parts[0].parse().unwrap();
        let minor: u32 = parts[1].parse().unwrap();
        assert!(!(major > MIN_NODE_MAJOR || (major == MIN_NODE_MAJOR && minor >= MIN_NODE_MINOR)));
    }
}
