//! Bounded subprocess runner for provider readiness and version probes.

use std::ffi::{OsStr, OsString};
use std::process::{Output, Stdio};
use std::time::Duration;

use roko_core::child_env::{self, CredentialScrub};

use crate::process::{ResourceLimits, confined_command};

use super::ProbeError;

/// Run one provider probe subprocess with the same process guarantees as the
/// provider workload and a hard wall-clock timeout.
///
/// The probe inherits roko's environment minus what `scrub` strips, as the
/// provider's own processes do (`CredentialScrub::for_kind`): `openclaw
/// doctor --repair` never sees a key the provider does not own.
pub(crate) async fn run_probe_command<I, S>(
    program: impl AsRef<OsStr>,
    args: I,
    limits: Option<&ResourceLimits>,
    timeout: Duration,
    scrub: &CredentialScrub,
) -> Result<Output, ProbeError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let inherited = child_env::process_env();
    run_probe_command_inheriting(program, args, limits, timeout, scrub, inherited).await
}

/// [`run_probe_command`] with `inherited` standing in for roko's own
/// environment.
async fn run_probe_command_inheriting<I, S>(
    program: impl AsRef<OsStr>,
    args: I,
    limits: Option<&ResourceLimits>,
    timeout: Duration,
    scrub: &CredentialScrub,
    inherited: Vec<(String, OsString)>,
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
    child_env::apply_credential_scrub_from(command.as_std_mut(), scrub, inherited);

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
        let scrub = CredentialScrub::default();
        let result = run_probe_command("/bin/sh", ["-c", "sleep 5"], None, timeout, &scrub).await;

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
            &CredentialScrub::default(),
        )
        .await
        .expect("run allowed probe");

        assert!(output.status.success());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn probe_command_env_excludes_provider_keys() {
        let path = std::env::var_os("PATH")
            .filter(|path| !path.is_empty())
            .unwrap_or_else(|| "/usr/bin:/bin".into());
        let mut parent = vec![("PATH".to_string(), path)];
        for (name, value) in [
            ("HOME", "/tmp/probe-home"),
            ("OPENAI_API_KEY", "sk-test-not-real"),
            ("ANTHROPIC_API_KEY", "sk-ant-test-not-real"),
            ("ROKO_SERVE_AUTH_API_KEY", "serve-test-not-real"),
        ] {
            parent.push((name.to_string(), OsString::from(value)));
        }

        let output = run_probe_command_inheriting(
            "/bin/sh",
            ["-c", "env"],
            None,
            Duration::from_secs(5),
            &CredentialScrub::default(),
            parent,
        )
        .await
        .expect("run env probe");

        let env = String::from_utf8_lossy(&output.stdout);
        for leaked in [
            "sk-test-not-real",
            "sk-ant-test-not-real",
            "serve-test-not-real",
        ] {
            assert!(!env.contains(leaked), "{leaked} leaked:\n{env}");
        }
        assert!(env.contains("HOME=/tmp/probe-home"), "HOME missing:\n{env}");
    }
}
