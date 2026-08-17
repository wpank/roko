//! Bounded subprocess runner for provider readiness and version probes.

use std::ffi::OsStr;
use std::process::{Output, Stdio};
use std::time::Duration;

use crate::process::{ResourceLimits, confined_command};

use super::ProbeError;

/// Run one provider probe subprocess with the same process guarantees as the
/// provider workload and a hard wall-clock timeout.
pub(crate) async fn run_probe_command<I, S>(
    program: impl AsRef<OsStr>,
    args: I,
    limits: Option<&ResourceLimits>,
    timeout: Duration,
) -> Result<Output, ProbeError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut command = confined_command(program, limits)?;
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    match tokio::time::timeout(timeout, command.output()).await {
        Ok(output) => output.map_err(ProbeError::Io),
        Err(_) => Err(ProbeError::Timeout(timeout)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[tokio::test]
    async fn probe_subprocess_obeys_wall_clock_timeout() {
        let timeout = Duration::from_millis(20);
        let result = run_probe_command("/bin/sh", ["-c", "sleep 5"], None, timeout).await;

        assert!(matches!(result, Err(ProbeError::Timeout(value)) if value == timeout));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn probe_subprocess_accepts_explicit_network_allow_policy() {
        let limits = ResourceLimits::default();
        let output = run_probe_command(
            "/usr/bin/true",
            std::iter::empty::<&str>(),
            Some(&limits),
            Duration::from_secs(1),
        )
        .await
        .expect("run allowed probe");

        assert!(output.status.success());
    }
}
