//! Pre-flight checks for `roko plan run`.
//!
//! Validates the workspace environment before entering the main event loop:
//! config loadability, LLM credentials, plan directory presence, Rust toolchain,
//! and stale workspace locks.

use std::path::Path;

use roko_core::config::schema::RokoConfig;

/// Status of an individual preflight check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreflightStatus {
    Pass,
    Warn,
    Fail,
}

impl PreflightStatus {
    /// Fixed-width label used in human-readable output.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Warn => "WARN",
            Self::Fail => "FAIL",
        }
    }
}

/// One named check in the preflight report.
#[derive(Debug, Clone)]
pub struct PreflightCheck {
    /// Short identifier for the check (e.g. `"config"`, `"credentials"`).
    pub name: &'static str,
    /// Whether the check passed, warned, or failed.
    pub status: PreflightStatus,
    /// Human-readable description of the result.
    pub message: String,
}

/// Run all preflight checks and return the results.
///
/// `config` is the already-loaded `RokoConfig` (or `None` if loading failed
/// earlier). `plans_dir` is the resolved plan directory passed to `plan run`.
/// `workdir` is the workspace root.
pub fn run_preflight_checks(
    config: Option<&RokoConfig>,
    plans_dir: &Path,
    workdir: &Path,
) -> Vec<PreflightCheck> {
    let mut checks = Vec::with_capacity(8);
    checks.push(check_config(config, workdir));
    checks.push(check_credentials(config, workdir));
    checks.push(check_disk_space(workdir));
    checks.push(check_git_state(workdir));
    checks.push(check_plans_dir(plans_dir));
    checks.push(check_declared_change_impact(plans_dir));
    checks.push(check_rust_toolchain());
    checks.push(check_stale_lock(workdir));
    checks
}

fn check_declared_change_impact(plans_dir: &Path) -> PreflightCheck {
    let mut manifests = Vec::new();
    if plans_dir.join("tasks.toml").is_file() {
        manifests.push(plans_dir.join("tasks.toml"));
    } else if let Ok(entries) = std::fs::read_dir(plans_dir) {
        manifests.extend(
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.path().join("tasks.toml"))
                .filter(|path| path.is_file()),
        );
    }
    manifests.sort();
    let mut diagnostics = Vec::new();
    for manifest in manifests {
        let Ok(tasks) = crate::task_parser::TasksFile::parse(&manifest) else {
            continue;
        };
        for task in tasks.tasks {
            let text = format!(
                "{} {}",
                task.title,
                task.description.as_deref().unwrap_or_default()
            )
            .to_ascii_lowercase();
            let likely_contract = [
                "public",
                "signature",
                "struct field",
                "enum",
                "trait",
                "serde",
                "schema",
                "re-export",
                "reexport",
                "api contract",
            ]
            .iter()
            .any(|term| text.contains(term));
            if !likely_contract {
                continue;
            }
            let acknowledged = task.context.as_ref().is_some_and(|context| {
                context
                    .impact_acknowledgement
                    .as_deref()
                    .is_some_and(|reason| !reason.trim().is_empty())
            });
            if acknowledged {
                continue;
            }
            let symbols_missing = task
                .context
                .as_ref()
                .is_none_or(|context| context.symbols.is_empty());
            if symbols_missing {
                diagnostics.push(format!(
                    "{}:{} declares a likely public/serialized contract change but no context.symbols; list changed symbols and known consumers",
                    tasks.meta.plan, task.id
                ));
            }
            if task.files.len() <= 1 {
                diagnostics.push(format!(
                    "{}:{} declares a likely cross-crate change with only {} planned file; confirm callers/re-exports or acknowledge the staged scope",
                    tasks.meta.plan,
                    task.id,
                    task.files.len()
                ));
            }
        }
    }
    if diagnostics.is_empty() {
        PreflightCheck {
            name: "impact",
            status: PreflightStatus::Pass,
            message: "declared high-impact task scopes are explicit".into(),
        }
    } else {
        let omitted = diagnostics.len().saturating_sub(3);
        let mut message = diagnostics
            .into_iter()
            .take(3)
            .collect::<Vec<_>>()
            .join("; ");
        if omitted > 0 {
            message.push_str(&format!("; and {omitted} more"));
        }
        PreflightCheck {
            name: "impact",
            status: PreflightStatus::Warn,
            message,
        }
    }
}

/// Print the preflight results in doctor-style format.
///
/// When all checks pass, no output is printed. When any check produces a
/// warning or failure, only non-passing checks are printed along with a
/// summary line.
///
/// Returns `true` if any check has [`PreflightStatus::Fail`].
pub fn print_preflight_results(checks: &[PreflightCheck]) -> bool {
    let any_fail = checks.iter().any(|c| c.status == PreflightStatus::Fail);
    let any_non_pass = checks.iter().any(|c| c.status != PreflightStatus::Pass);

    // Silent when all checks pass.
    if !any_non_pass {
        return false;
    }

    for check in checks {
        if check.status != PreflightStatus::Pass {
            eprintln!(
                "[{}] {}: {}",
                check.status.label(),
                check.name,
                check.message
            );
        }
    }

    let pass = checks
        .iter()
        .filter(|c| c.status == PreflightStatus::Pass)
        .count();
    let warn = checks
        .iter()
        .filter(|c| c.status == PreflightStatus::Warn)
        .count();
    let fail = checks
        .iter()
        .filter(|c| c.status == PreflightStatus::Fail)
        .count();
    eprintln!("preflight: {pass} passed, {warn} warnings, {fail} failed");
    any_fail
}

/// Async provider connectivity preflight — probes all configured providers.
///
/// HTTP providers get a HEAD probe with a 2 s timeout (reuses `doctor.rs` logic).
/// CLI providers check for the binary on `$PATH`.
/// Individual unreachable providers produce `Warn`; zero reachable produces a `Fail` summary.
pub async fn check_provider_connectivity(config: &RokoConfig) -> Vec<PreflightCheck> {
    use roko_core::agent::ProviderKind;
    use std::time::Duration;

    let mut checks = Vec::new();
    let mut reachable_count = 0u32;
    let providers = config.effective_providers();

    if providers.is_empty() {
        checks.push(PreflightCheck {
            name: "providers",
            status: PreflightStatus::Warn,
            message: "no providers configured".into(),
        });
        return checks;
    }

    let timeout = Duration::from_secs(2);
    let mut handles = Vec::new();

    for (provider_id, provider) in &providers {
        let kind = provider.kind;

        // CLI providers: check binary on PATH.
        let is_cli = matches!(
            kind,
            ProviderKind::ClaudeCli
                | ProviderKind::CodexCli
                | ProviderKind::CursorCli
                | ProviderKind::GeminiCli
                | ProviderKind::CursorAcp
                | ProviderKind::Hermes
                | ProviderKind::OpenClaw
        );

        if is_cli {
            let binary = match kind {
                ProviderKind::ClaudeCli => "claude",
                ProviderKind::CodexCli => "codex",
                ProviderKind::CursorCli | ProviderKind::CursorAcp => "cursor",
                ProviderKind::GeminiCli => "gemini",
                ProviderKind::Hermes => "hermes",
                ProviderKind::OpenClaw => "openclaw",
                _ => continue,
            };
            let found = std::process::Command::new("which")
                .arg(binary)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            checks.push(PreflightCheck {
                name: "providers",
                status: if found {
                    PreflightStatus::Pass
                } else {
                    PreflightStatus::Warn
                },
                message: if found {
                    format!("provider:{provider_id}: {binary} found on PATH")
                } else {
                    format!("provider:{provider_id}: {binary} not found on PATH")
                },
            });
            if found {
                reachable_count += 1;
            }
            continue;
        }

        // HTTP providers: probe endpoint.
        let endpoint = crate::doctor::endpoint_for_provider(provider);
        match endpoint {
            Some(url) => {
                let pid = provider_id.clone();
                handles.push(tokio::spawn(async move {
                    let result =
                        crate::doctor::probe_one_provider(pid.clone(), url, timeout).await;
                    (pid, result)
                }));
            }
            None => {
                checks.push(PreflightCheck {
                    name: "providers",
                    status: PreflightStatus::Warn,
                    message: format!("provider:{provider_id}: no endpoint configured"),
                });
            }
        }
    }

    // Await all HTTP probes.
    for handle in handles {
        if let Ok((pid, probe)) = handle.await {
            let passed = probe
                .http_status
                .map_or(false, |c| (200..400).contains(&c.into()));
            checks.push(PreflightCheck {
                name: "providers",
                status: if passed {
                    PreflightStatus::Pass
                } else {
                    PreflightStatus::Warn
                },
                message: if passed {
                    format!(
                        "provider:{pid}: {} ({}ms)",
                        probe
                            .http_status
                            .map_or("OK".to_string(), |c| format!("{c}")),
                        probe.latency_ms.unwrap_or(0)
                    )
                } else {
                    format!("provider:{pid}: {}", probe.message)
                },
            });
            if passed {
                reachable_count += 1;
            }
        }
    }

    // Summary check: fail if zero providers reachable.
    if reachable_count == 0 && !checks.is_empty() {
        checks.push(PreflightCheck {
            name: "providers",
            status: PreflightStatus::Fail,
            message: "no providers reachable \u{2014} all probes failed or timed out".into(),
        });
    }

    checks
}

// ── Individual checks ────────────────────────────────────────────────────

/// Check whether the config was loadable and minimally valid.
fn check_config(config: Option<&RokoConfig>, workdir: &Path) -> PreflightCheck {
    match config {
        Some(_) => PreflightCheck {
            name: "config",
            status: PreflightStatus::Pass,
            message: format!("roko.toml loaded from {}", workdir.display()),
        },
        None => {
            // Try to give a more specific error.
            let toml_path = workdir.join("roko.toml");
            if !toml_path.exists() {
                PreflightCheck {
                    name: "config",
                    status: PreflightStatus::Fail,
                    message: format!(
                        "roko.toml not found at {} — run `roko init`",
                        toml_path.display()
                    ),
                }
            } else {
                // File exists but failed to parse.
                let detail = match std::fs::read_to_string(&toml_path) {
                    Ok(content) => match RokoConfig::from_toml(&content) {
                        Ok(_) => "config loaded on retry — possible transient error".to_string(),
                        Err(e) => format!("parse error: {e}"),
                    },
                    Err(e) => format!("read error: {e}"),
                };
                PreflightCheck {
                    name: "config",
                    status: PreflightStatus::Fail,
                    message: format!("roko.toml exists but cannot be loaded: {detail}"),
                }
            }
        }
    }
}

/// Check that at least one LLM provider has usable credentials.
fn check_credentials(config: Option<&RokoConfig>, workdir: &Path) -> PreflightCheck {
    // Use the same config-aware detection that dispatch will use.
    let auth = crate::auth_detect::detect_auth_from_config(workdir);
    match auth {
        crate::auth_detect::AuthMethod::NeedsSetup => {
            // Provide a slightly different message when we have config.
            let hint = if config.is_some() {
                "no provider has credentials — set an API key env var or install a CLI provider"
            } else {
                "no config and no provider credentials found — run `roko config init`"
            };
            PreflightCheck {
                name: "credentials",
                status: PreflightStatus::Fail,
                message: hint.to_string(),
            }
        }
        other => PreflightCheck {
            name: "credentials",
            status: PreflightStatus::Pass,
            message: format!("provider available: {}", other.label()),
        },
    }
}

/// Minimum free disk space required (2 GB).
const MIN_FREE_DISK_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// Check that there is at least 2 GB of free disk space on the workspace partition.
fn check_disk_space(workdir: &Path) -> PreflightCheck {
    match get_available_disk_bytes(workdir) {
        Some(avail) if avail >= MIN_FREE_DISK_BYTES => PreflightCheck {
            name: "disk",
            status: PreflightStatus::Pass,
            message: format!("{:.1} GB free", avail as f64 / (1024.0 * 1024.0 * 1024.0)),
        },
        Some(avail) => PreflightCheck {
            name: "disk",
            status: PreflightStatus::Fail,
            message: format!(
                "only {:.1} GB free (need >= 2 GB). Free up space or use --force to skip this check.",
                avail as f64 / (1024.0 * 1024.0 * 1024.0)
            ),
        },
        None => PreflightCheck {
            name: "disk",
            status: PreflightStatus::Warn,
            message: "could not determine free disk space".to_string(),
        },
    }
}

/// Query available disk bytes on the partition containing `path`.
///
/// Shells out to `df` on Unix; returns `None` on non-Unix or when the
/// command fails.
fn get_available_disk_bytes(path: &Path) -> Option<u64> {
    // `df -Pk <path>` outputs POSIX-portable 1024-byte blocks.
    // The "Available" column is the 4th field on the data line.
    let output = std::process::Command::new("df")
        .args(["-Pk"])
        .arg(path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let data_line = stdout.lines().nth(1)?;
    let avail_kb: u64 = data_line.split_whitespace().nth(3)?.parse().ok()?;
    Some(avail_kb * 1024)
}

/// Check git working directory state.
///
/// Warns (does not fail) if the tree has uncommitted changes. This catches the
/// common case where an operator forgot to commit before a plan run, which can
/// cause merge conflicts in worktrees.
fn check_git_state(workdir: &Path) -> PreflightCheck {
    // Check if we are in a git repo.
    let status_output = std::process::Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(workdir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output();

    match status_output {
        Ok(out) if out.status.success() => {}
        _ => {
            return PreflightCheck {
                name: "git",
                status: PreflightStatus::Warn,
                message: "not inside a git repository".to_string(),
            };
        }
    }

    // Check for uncommitted changes.
    let dirty = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(workdir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output();

    match dirty {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            if stdout.trim().is_empty() {
                PreflightCheck {
                    name: "git",
                    status: PreflightStatus::Pass,
                    message: "working tree is clean".to_string(),
                }
            } else {
                let changed = stdout.lines().count();
                PreflightCheck {
                    name: "git",
                    status: PreflightStatus::Warn,
                    message: format!(
                        "working tree has {changed} uncommitted change(s) \u{2014} \
                         consider committing before plan run to avoid worktree merge conflicts"
                    ),
                }
            }
        }
        _ => PreflightCheck {
            name: "git",
            status: PreflightStatus::Warn,
            message: "could not determine git status".to_string(),
        },
    }
}

/// Check that the plans directory exists and contains at least one tasks.toml.
fn check_plans_dir(plans_dir: &Path) -> PreflightCheck {
    if !plans_dir.exists() {
        return PreflightCheck {
            name: "plans",
            status: PreflightStatus::Fail,
            message: format!("plans directory does not exist: {}", plans_dir.display()),
        };
    }

    // Check for tasks.toml in the directory itself (single-plan case).
    if plans_dir.join("tasks.toml").is_file() {
        return PreflightCheck {
            name: "plans",
            status: PreflightStatus::Pass,
            message: format!("found tasks.toml in {}", plans_dir.display()),
        };
    }

    // Check subdirectories for tasks.toml files.
    let task_count = match std::fs::read_dir(plans_dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir() && e.path().join("tasks.toml").is_file())
            .count(),
        Err(e) => {
            return PreflightCheck {
                name: "plans",
                status: PreflightStatus::Fail,
                message: format!("cannot read plans directory: {e}"),
            };
        }
    };

    if task_count == 0 {
        PreflightCheck {
            name: "plans",
            status: PreflightStatus::Fail,
            message: format!(
                "no tasks.toml found in {} or its subdirectories",
                plans_dir.display()
            ),
        }
    } else {
        PreflightCheck {
            name: "plans",
            status: PreflightStatus::Pass,
            message: format!("{task_count} plan(s) found in {}", plans_dir.display()),
        }
    }
}

/// Check that `rustc` is available (gate pipeline requires it).
fn check_rust_toolchain() -> PreflightCheck {
    match std::process::Command::new("rustc")
        .arg("--version")
        .output()
    {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            PreflightCheck {
                name: "rust",
                status: PreflightStatus::Pass,
                message: version,
            }
        }
        Ok(_) => PreflightCheck {
            name: "rust",
            status: PreflightStatus::Warn,
            message: "rustc found but returned non-zero — toolchain may be broken".to_string(),
        },
        Err(_) => PreflightCheck {
            name: "rust",
            status: PreflightStatus::Warn,
            message: "rustc not found on PATH — gate checks that compile will fail".to_string(),
        },
    }
}

/// Check for a stale workspace lock (another runner might be holding it, or a
/// previous crash left a lock file with a dead PID).
fn check_stale_lock(workdir: &Path) -> PreflightCheck {
    let lock_path = workdir.join(".roko").join("runtime").join("roko.lock");
    if !lock_path.exists() {
        return PreflightCheck {
            name: "lock",
            status: PreflightStatus::Pass,
            message: "no existing workspace lock".to_string(),
        };
    }

    // Read the PID from the lock file.
    let pid_str = match std::fs::read_to_string(&lock_path) {
        Ok(s) => s.trim().to_string(),
        Err(_) => {
            return PreflightCheck {
                name: "lock",
                status: PreflightStatus::Warn,
                message: format!("lock file exists but unreadable: {}", lock_path.display()),
            };
        }
    };

    if pid_str.is_empty() {
        return PreflightCheck {
            name: "lock",
            status: PreflightStatus::Pass,
            message: "lock file exists but is empty (clean shutdown)".to_string(),
        };
    }

    let pid: u32 = match pid_str.parse() {
        Ok(p) => p,
        Err(_) => {
            return PreflightCheck {
                name: "lock",
                status: PreflightStatus::Warn,
                message: format!("lock file contains non-numeric PID: {pid_str:?}"),
            };
        }
    };

    // If the lock PID matches our own process, the lock was written by this
    // runner instance before calling preflight — not a conflict.
    if pid == std::process::id() {
        return PreflightCheck {
            name: "lock",
            status: PreflightStatus::Pass,
            message: "workspace lock held by current process".to_string(),
        };
    }

    if is_pid_alive(pid) {
        PreflightCheck {
            name: "lock",
            status: PreflightStatus::Fail,
            message: format!(
                "another roko process is running (PID {pid}) — wait for it to finish or kill it"
            ),
        }
    } else {
        PreflightCheck {
            name: "lock",
            status: PreflightStatus::Warn,
            message: format!(
                "stale lock from dead PID {pid} — will be replaced on lock acquisition"
            ),
        }
    }
}

/// Check whether a process with the given PID is alive.
///
/// Uses `kill -0 <pid>` via a subprocess on Unix, which sends no signal but
/// checks for the process's existence (exit 0 = alive, non-zero = dead).
fn is_pid_alive(pid: u32) -> bool {
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn check_config_pass_when_config_present() {
        let config = RokoConfig::default();
        let workdir = tempdir().expect("tempdir");
        let result = check_config(Some(&config), workdir.path());
        assert_eq!(result.status, PreflightStatus::Pass);
        assert_eq!(result.name, "config");
    }

    #[test]
    fn check_config_fail_when_no_config_and_no_file() {
        let workdir = tempdir().expect("tempdir");
        let result = check_config(None, workdir.path());
        assert_eq!(result.status, PreflightStatus::Fail);
        assert!(
            result.message.contains("not found"),
            "message: {}",
            result.message
        );
    }

    #[test]
    fn check_config_fail_when_no_config_but_file_exists_with_bad_content() {
        let workdir = tempdir().expect("tempdir");
        std::fs::write(workdir.path().join("roko.toml"), "{{invalid toml}}")
            .expect("write bad toml");
        let result = check_config(None, workdir.path());
        assert_eq!(result.status, PreflightStatus::Fail);
        assert!(
            result.message.contains("parse error") || result.message.contains("cannot be loaded"),
            "message: {}",
            result.message
        );
    }

    #[test]
    fn check_plans_dir_fail_when_missing() {
        let workdir = tempdir().expect("tempdir");
        let plans_dir = workdir.path().join("nonexistent");
        let result = check_plans_dir(&plans_dir);
        assert_eq!(result.status, PreflightStatus::Fail);
        assert!(
            result.message.contains("does not exist"),
            "message: {}",
            result.message
        );
    }

    #[test]
    fn check_plans_dir_fail_when_empty() {
        let workdir = tempdir().expect("tempdir");
        let plans_dir = workdir.path().join("plans");
        std::fs::create_dir_all(&plans_dir).expect("create dir");
        let result = check_plans_dir(&plans_dir);
        assert_eq!(result.status, PreflightStatus::Fail);
        assert!(
            result.message.contains("no tasks.toml"),
            "message: {}",
            result.message
        );
    }

    #[test]
    fn check_plans_dir_pass_with_direct_tasks_toml() {
        let workdir = tempdir().expect("tempdir");
        let plans_dir = workdir.path().join("my-plan");
        std::fs::create_dir_all(&plans_dir).expect("create dir");
        std::fs::write(plans_dir.join("tasks.toml"), "[meta]\nplan = \"test\"\n")
            .expect("write tasks.toml");
        let result = check_plans_dir(&plans_dir);
        assert_eq!(result.status, PreflightStatus::Pass);
    }

    #[test]
    fn check_plans_dir_pass_with_subdirectory_plans() {
        let workdir = tempdir().expect("tempdir");
        let plans_dir = workdir.path().join("plans");
        let sub_plan = plans_dir.join("alpha");
        std::fs::create_dir_all(&sub_plan).expect("create dir");
        std::fs::write(sub_plan.join("tasks.toml"), "[meta]\nplan = \"alpha\"\n")
            .expect("write tasks.toml");
        let result = check_plans_dir(&plans_dir);
        assert_eq!(result.status, PreflightStatus::Pass);
        assert!(
            result.message.contains("1 plan(s)"),
            "message: {}",
            result.message
        );
    }

    #[test]
    fn check_rust_toolchain_pass_on_dev_machine() {
        // This test assumes the dev machine has rustc installed.
        let result = check_rust_toolchain();
        assert_eq!(result.status, PreflightStatus::Pass);
        assert!(
            result.message.contains("rustc"),
            "message: {}",
            result.message
        );
    }

    #[test]
    fn check_stale_lock_pass_when_no_lock() {
        let workdir = tempdir().expect("tempdir");
        let result = check_stale_lock(workdir.path());
        assert_eq!(result.status, PreflightStatus::Pass);
        assert!(
            result.message.contains("no existing"),
            "message: {}",
            result.message
        );
    }

    #[test]
    fn check_stale_lock_pass_when_empty_lock() {
        let workdir = tempdir().expect("tempdir");
        let lock_dir = workdir.path().join(".roko").join("runtime");
        std::fs::create_dir_all(&lock_dir).expect("create dir");
        std::fs::write(lock_dir.join("roko.lock"), "").expect("write empty lock");
        let result = check_stale_lock(workdir.path());
        assert_eq!(result.status, PreflightStatus::Pass);
        assert!(
            result.message.contains("empty"),
            "message: {}",
            result.message
        );
    }

    #[test]
    fn check_stale_lock_warn_when_dead_pid() {
        let workdir = tempdir().expect("tempdir");
        let lock_dir = workdir.path().join(".roko").join("runtime");
        std::fs::create_dir_all(&lock_dir).expect("create dir");
        // PID 999999999 is extremely unlikely to be alive.
        std::fs::write(lock_dir.join("roko.lock"), "999999999\n").expect("write lock");
        let result = check_stale_lock(workdir.path());
        assert_eq!(result.status, PreflightStatus::Warn);
        assert!(
            result.message.contains("stale"),
            "message: {}",
            result.message
        );
    }

    #[test]
    fn check_stale_lock_pass_when_current_pid() {
        let workdir = tempdir().expect("tempdir");
        let lock_dir = workdir.path().join(".roko").join("runtime");
        std::fs::create_dir_all(&lock_dir).expect("create dir");
        let pid = std::process::id();
        std::fs::write(lock_dir.join("roko.lock"), format!("{pid}\n")).expect("write lock");
        let result = check_stale_lock(workdir.path());
        // Current process PID should be recognized as our own lock, not a conflict.
        assert_eq!(result.status, PreflightStatus::Pass);
        assert!(
            result.message.contains("current process"),
            "message: {}",
            result.message
        );
    }

    #[test]
    fn check_stale_lock_warn_when_non_numeric() {
        let workdir = tempdir().expect("tempdir");
        let lock_dir = workdir.path().join(".roko").join("runtime");
        std::fs::create_dir_all(&lock_dir).expect("create dir");
        std::fs::write(lock_dir.join("roko.lock"), "not-a-pid\n").expect("write lock");
        let result = check_stale_lock(workdir.path());
        assert_eq!(result.status, PreflightStatus::Warn);
        assert!(
            result.message.contains("non-numeric"),
            "message: {}",
            result.message
        );
    }

    #[test]
    fn preflight_status_labels() {
        assert_eq!(PreflightStatus::Pass.label(), "PASS");
        assert_eq!(PreflightStatus::Warn.label(), "WARN");
        assert_eq!(PreflightStatus::Fail.label(), "FAIL");
    }

    #[test]
    fn print_preflight_results_returns_true_on_failure() {
        let checks = vec![
            PreflightCheck {
                name: "good",
                status: PreflightStatus::Pass,
                message: "all fine".to_string(),
            },
            PreflightCheck {
                name: "bad",
                status: PreflightStatus::Fail,
                message: "broken".to_string(),
            },
        ];
        assert!(print_preflight_results(&checks));
    }

    #[test]
    fn print_preflight_results_returns_false_when_all_pass() {
        let checks = vec![
            PreflightCheck {
                name: "a",
                status: PreflightStatus::Pass,
                message: "ok".to_string(),
            },
            PreflightCheck {
                name: "b",
                status: PreflightStatus::Warn,
                message: "warn".to_string(),
            },
        ];
        assert!(!print_preflight_results(&checks));
    }

    #[test]
    fn run_preflight_checks_returns_all_eight_checks() {
        let workdir = tempdir().expect("tempdir");
        let plans_dir = workdir.path().join("plans");
        std::fs::create_dir_all(&plans_dir).expect("create plans dir");
        let config = RokoConfig::default();
        let checks = run_preflight_checks(Some(&config), &plans_dir, workdir.path());
        assert_eq!(checks.len(), 8);
        let names: Vec<&str> = checks.iter().map(|c| c.name).collect();
        assert_eq!(
            names,
            &[
                "config",
                "credentials",
                "disk",
                "git",
                "plans",
                "impact",
                "rust",
                "lock"
            ]
        );
    }

    #[test]
    fn check_disk_space_pass_on_dev_machine() {
        // Dev machines should have > 2 GB free.
        let workdir = tempdir().expect("tempdir");
        let result = check_disk_space(workdir.path());
        assert_eq!(result.status, PreflightStatus::Pass);
        assert!(
            result.message.contains("GB free"),
            "message: {}",
            result.message
        );
    }

    #[test]
    fn check_git_state_in_non_repo() {
        // A tempdir won't be a git repo.
        let workdir = tempdir().expect("tempdir");
        let result = check_git_state(workdir.path());
        assert_eq!(result.status, PreflightStatus::Warn);
        assert!(
            result.message.contains("not inside"),
            "message: {}",
            result.message
        );
    }

    #[test]
    fn is_pid_alive_for_current_process() {
        assert!(is_pid_alive(std::process::id()));
    }

    #[test]
    fn is_pid_alive_for_dead_process() {
        // PID 999999999 should not exist on any sane system.
        assert!(!is_pid_alive(999_999_999));
    }
}
