//! Fail-closed resource and network policy for spawned provider processes.
//!
//! Unix targets install `setrlimit` calls in the child just before `exec`.
//! Network denial uses macOS Seatbelt or Linux firejail with seccomp. Other
//! platforms reject requested guarantees before spawn so configuration cannot
//! silently degrade to an unenforced policy.

use std::ffi::OsStr;
use std::io;
use std::path::PathBuf;

use roko_core::config::provider::{ProviderConfig, ProviderNetworkPolicy};
use serde::{Deserialize, Serialize};
use tokio::process::Command;

/// Optional process guarantees for provider subprocesses.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum CPU seconds before the OS delivers the platform limit signal.
    pub max_cpu_secs: Option<u64>,
    /// Maximum resident/address-space bytes, depending on platform support.
    pub max_rss_bytes: Option<u64>,
    /// Maximum process count available to the child under its real user ID.
    pub max_processes: Option<u64>,
    /// Network policy enforced for the provider process.
    pub network: ProviderNetworkPolicy,
}

impl ResourceLimits {
    /// Build subprocess resource limits from a provider configuration.
    #[must_use]
    pub fn from_provider_config(provider: &ProviderConfig) -> Option<Self> {
        let limits = provider.limits.as_ref()?;
        let resource_limits = Self {
            max_cpu_secs: limits.max_cpu_seconds,
            max_rss_bytes: limits.max_rss_bytes,
            max_processes: limits.max_processes,
            network: limits.network,
        };
        resource_limits.is_requested().then_some(resource_limits)
    }

    /// Whether at least one OS resource limit was requested.
    #[must_use]
    pub fn is_requested(&self) -> bool {
        self.max_cpu_secs.is_some()
            || self.max_rss_bytes.is_some()
            || self.max_processes.is_some()
            || self.network == ProviderNetworkPolicy::Deny
    }

    /// Validate values and confirm that this platform can enforce them.
    pub fn validate_for_current_platform(&self) -> io::Result<()> {
        if self.max_cpu_secs == Some(0)
            || self.max_rss_bytes == Some(0)
            || self.max_processes == Some(0)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "provider process resource limits must be greater than zero",
            ));
        }
        #[cfg(target_os = "macos")]
        if self.max_rss_bytes.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "macOS does not provide an enforceable per-process RSS/address-space rlimit",
            ));
        }
        #[cfg(not(unix))]
        if self.max_cpu_secs.is_some()
            || self.max_rss_bytes.is_some()
            || self.max_processes.is_some()
        {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "OS provider process limits are unavailable on this platform",
            ));
        }
        if self.network == ProviderNetworkPolicy::Deny {
            NetworkConfinement::detect().ensure_supported()?;
        }
        Ok(())
    }
}

/// Kernel-backed launcher used to deny all network access for a provider
/// subprocess. Fixed launcher paths avoid PATH substitution attacks.
#[derive(Debug, Clone, PartialEq, Eq)]
enum NetworkConfinement {
    MacOsSeatbelt { executable: PathBuf },
    LinuxFirejail { executable: PathBuf },
    Unsupported { reason: String },
}

impl NetworkConfinement {
    fn detect() -> Self {
        #[cfg(target_os = "macos")]
        {
            let executable = PathBuf::from("/usr/bin/sandbox-exec");
            if executable.is_file() {
                return Self::MacOsSeatbelt { executable };
            }
            Self::Unsupported {
                reason: "macOS Seatbelt launcher `/usr/bin/sandbox-exec` is unavailable"
                    .to_string(),
            }
        }

        #[cfg(target_os = "linux")]
        {
            for candidate in ["/usr/bin/firejail", "/usr/local/bin/firejail"] {
                let executable = PathBuf::from(candidate);
                if executable.is_file() {
                    return Self::LinuxFirejail { executable };
                }
            }
            Self::Unsupported {
                reason: "Linux network denial requires firejail with seccomp at `/usr/bin/firejail` or `/usr/local/bin/firejail`"
                    .to_string(),
            }
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        Self::Unsupported {
            reason: format!(
                "provider network isolation is not implemented for {}",
                std::env::consts::OS
            ),
        }
    }

    fn ensure_supported(&self) -> io::Result<()> {
        match self {
            Self::Unsupported { reason } => {
                Err(io::Error::new(io::ErrorKind::Unsupported, reason.clone()))
            }
            Self::MacOsSeatbelt { .. } | Self::LinuxFirejail { .. } => Ok(()),
        }
    }

    fn command(&self, program: &OsStr) -> io::Result<Command> {
        match self {
            Self::MacOsSeatbelt { executable } => {
                let mut command = Command::new(executable);
                command
                    .arg("-p")
                    .arg("(version 1)\n(allow default)\n(deny network*)\n")
                    .arg(program);
                Ok(command)
            }
            Self::LinuxFirejail { executable } => {
                let mut command = Command::new(executable);
                command.args([
                    "--quiet",
                    "--noprofile",
                    "--nonewprivs",
                    "--noroot",
                    "--caps.drop=all",
                    "--seccomp",
                    "--net=none",
                    "--",
                ]);
                command.arg(program);
                Ok(command)
            }
            Self::Unsupported { reason } => {
                Err(io::Error::new(io::ErrorKind::Unsupported, reason.clone()))
            }
        }
    }
}

/// Construct a provider command with every configured process guarantee
/// installed before spawn.
///
/// Network denial is implemented by a trusted kernel-confinement launcher;
/// Unix resource limits are installed on that launcher and inherited by the
/// provider. Any unavailable requested guarantee is returned as an error.
pub fn confined_command(
    program: impl AsRef<OsStr>,
    limits: Option<&ResourceLimits>,
) -> io::Result<Command> {
    let Some(limits) = limits else {
        return Ok(Command::new(program));
    };
    limits.validate_for_current_platform()?;

    let mut command = if limits.network == ProviderNetworkPolicy::Deny {
        NetworkConfinement::detect().command(program.as_ref())?
    } else {
        Command::new(program)
    };

    let mut os_limits = limits.clone();
    os_limits.network = ProviderNetworkPolicy::Allow;
    apply_resource_limits(&mut command, &os_limits)?;
    Ok(command)
}

/// Apply resource limits to a command before spawning it.
#[cfg(unix)]
#[allow(unsafe_code, clippy::cast_possible_truncation)]
pub fn apply_resource_limits(cmd: &mut Command, limits: &ResourceLimits) -> io::Result<()> {
    if limits.network == ProviderNetworkPolicy::Deny {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "network denial must be installed with confined_command",
        ));
    }
    limits.validate_for_current_platform()?;
    if !limits.is_requested() {
        return Ok(());
    }

    let limits = limits.clone();
    // SAFETY: `setrlimit` is called in the child process after fork and before
    // exec. The closure only captures plain integers and does not touch shared
    // process state.
    unsafe {
        cmd.pre_exec(move || {
            if let Some(cpu_secs) = limits.max_cpu_secs {
                let limit = libc::rlimit {
                    rlim_cur: cpu_secs as libc::rlim_t,
                    rlim_max: cpu_secs as libc::rlim_t,
                };
                if libc::setrlimit(libc::RLIMIT_CPU, &limit) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
            }

            if let Some(max_bytes) = limits.max_rss_bytes {
                let limit = libc::rlimit {
                    rlim_cur: max_bytes as libc::rlim_t,
                    rlim_max: max_bytes as libc::rlim_t,
                };
                // RLIMIT_AS is stricter than a resident-set-only ceiling:
                // RSS can never exceed virtual address space. This gives an
                // enforceable memory cap even on kernels where RLIMIT_RSS is
                // merely advisory or ignored.
                if libc::setrlimit(libc::RLIMIT_AS, &limit) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
            }

            if let Some(max_processes) = limits.max_processes {
                let limit = libc::rlimit {
                    rlim_cur: max_processes as libc::rlim_t,
                    rlim_max: max_processes as libc::rlim_t,
                };
                if libc::setrlimit(libc::RLIMIT_NPROC, &limit) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
            }

            Ok(())
        });
    }
    Ok(())
}

/// Reject requested limits on platforms without an implementation.
#[cfg(not(unix))]
pub fn apply_resource_limits(_cmd: &mut Command, limits: &ResourceLimits) -> io::Result<()> {
    if limits.network == ProviderNetworkPolicy::Deny {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "network denial must be installed with confined_command",
        ));
    }
    limits.validate_for_current_platform()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_limit_is_rejected_before_spawn() {
        let limits = ResourceLimits {
            max_cpu_secs: Some(0),
            ..Default::default()
        };
        let error = limits
            .validate_for_current_platform()
            .expect_err("zero limit must fail closed");
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn unix_installs_cpu_memory_and_process_limits_before_exec() {
        let limits = ResourceLimits {
            max_cpu_secs: Some(60),
            #[cfg(not(target_os = "macos"))]
            max_rss_bytes: Some(8 * 1024 * 1024 * 1024),
            #[cfg(target_os = "macos")]
            max_rss_bytes: None,
            max_processes: Some(1),
            network: ProviderNetworkPolicy::Allow,
        };
        let mut command = Command::new("true");
        apply_resource_limits(&mut command, &limits).expect("configure resource limits");
        let status = command
            .status()
            .await
            .expect("spawn process with enforced limits");
        assert!(status.success());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_fails_closed_when_memory_enforcement_is_requested() {
        let limits = ResourceLimits {
            max_rss_bytes: Some(1024 * 1024 * 1024),
            ..Default::default()
        };
        let error = limits
            .validate_for_current_platform()
            .expect_err("macOS memory cap must not silently degrade");
        assert_eq!(error.kind(), io::ErrorKind::Unsupported);
    }

    #[test]
    fn explicit_network_allow_does_not_wrap_provider_command() {
        let limits = ResourceLimits::default();
        let command = confined_command("provider-binary", Some(&limits))
            .expect("allow policy does not require confinement backend");

        assert_eq!(
            command.as_std().get_program(),
            OsStr::new("provider-binary")
        );
    }

    #[test]
    fn denied_network_policy_builds_kernel_confinement_launchers() {
        let seatbelt = NetworkConfinement::MacOsSeatbelt {
            executable: PathBuf::from("/trusted/sandbox-exec"),
        }
        .command(OsStr::new("provider-binary"))
        .expect("build Seatbelt command");
        let seatbelt_args = seatbelt
            .as_std()
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert!(
            seatbelt_args
                .iter()
                .any(|arg| arg.contains("deny network*"))
        );
        assert_eq!(
            seatbelt_args.last().map(String::as_str),
            Some("provider-binary")
        );

        let firejail = NetworkConfinement::LinuxFirejail {
            executable: PathBuf::from("/trusted/firejail"),
        }
        .command(OsStr::new("provider-binary"))
        .expect("build firejail command");
        let firejail_args = firejail
            .as_std()
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert!(firejail_args.iter().any(|arg| arg == "--net=none"));
        assert!(firejail_args.iter().any(|arg| arg == "--seccomp"));
        assert_eq!(
            firejail_args.last().map(String::as_str),
            Some("provider-binary")
        );
    }

    #[test]
    fn unsupported_network_confinement_fails_closed() {
        let error = NetworkConfinement::Unsupported {
            reason: "test backend unavailable".to_string(),
        }
        .command(OsStr::new("provider-binary"))
        .expect_err("unsupported isolation must reject the command");

        assert_eq!(error.kind(), io::ErrorKind::Unsupported);
    }

    #[test]
    fn legacy_resource_only_api_rejects_network_policy() {
        let limits = ResourceLimits {
            network: ProviderNetworkPolicy::Deny,
            ..Default::default()
        };
        let mut command = Command::new("provider-binary");
        let error = apply_resource_limits(&mut command, &limits)
            .expect_err("resource-only API must not silently omit network denial");

        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    }

    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn seatbelt_denies_provider_loopback_network() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind loopback listener");
        let address = listener.local_addr().expect("listener address");
        let accept = tokio::spawn(async move {
            tokio::time::timeout(std::time::Duration::from_millis(750), listener.accept()).await
        });
        let limits = ResourceLimits {
            network: ProviderNetworkPolicy::Deny,
            ..Default::default()
        };
        let mut command = confined_command("/usr/bin/curl", Some(&limits))
            .expect("Seatbelt isolation is available");
        let status = command
            .args([
                "--silent",
                "--max-time",
                "0.5",
                &format!("http://{address}/"),
            ])
            .status()
            .await
            .expect("run isolated curl");

        assert!(!status.success());
        assert!(
            accept.await.expect("accept task").is_err(),
            "network-denied provider reached a loopback socket"
        );
    }
}
