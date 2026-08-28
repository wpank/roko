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
    let mut checks = Vec::with_capacity(5);
    checks.push(check_config(config, workdir));
    checks.push(check_credentials(config, workdir));
    checks.push(check_plans_dir(plans_dir));
    checks.push(check_rust_toolchain());
    checks.push(check_stale_lock(workdir));
    checks
}

/// Print the preflight results in doctor-style format.
///
/// Returns `true` if any check has [`PreflightStatus::Fail`].
pub fn print_preflight_results(checks: &[PreflightCheck]) -> bool {
    let mut any_fail = false;
    for check in checks {
        if check.status == PreflightStatus::Fail {
            any_fail = true;
        }
        eprintln!("[{}] {}: {}", check.status.label(), check.name, check.message);
    }

    let pass = checks.iter().filter(|c| c.status == PreflightStatus::Pass).count();
    let warn = checks.iter().filter(|c| c.status == PreflightStatus::Warn).count();
    let fail = checks.iter().filter(|c| c.status == PreflightStatus::Fail).count();
    eprintln!("preflight: {pass} passed, {warn} warnings, {fail} failed");
    any_fail
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
            let version = String::from_utf8_lossy(&output.stdout)
                .trim()
                .to_string();
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
        assert!(result.message.contains("not found"), "message: {}", result.message);
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
        assert!(result.message.contains("does not exist"), "message: {}", result.message);
    }

    #[test]
    fn check_plans_dir_fail_when_empty() {
        let workdir = tempdir().expect("tempdir");
        let plans_dir = workdir.path().join("plans");
        std::fs::create_dir_all(&plans_dir).expect("create dir");
        let result = check_plans_dir(&plans_dir);
        assert_eq!(result.status, PreflightStatus::Fail);
        assert!(result.message.contains("no tasks.toml"), "message: {}", result.message);
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
        assert!(result.message.contains("1 plan(s)"), "message: {}", result.message);
    }

    #[test]
    fn check_rust_toolchain_pass_on_dev_machine() {
        // This test assumes the dev machine has rustc installed.
        let result = check_rust_toolchain();
        assert_eq!(result.status, PreflightStatus::Pass);
        assert!(result.message.contains("rustc"), "message: {}", result.message);
    }

    #[test]
    fn check_stale_lock_pass_when_no_lock() {
        let workdir = tempdir().expect("tempdir");
        let result = check_stale_lock(workdir.path());
        assert_eq!(result.status, PreflightStatus::Pass);
        assert!(result.message.contains("no existing"), "message: {}", result.message);
    }

    #[test]
    fn check_stale_lock_pass_when_empty_lock() {
        let workdir = tempdir().expect("tempdir");
        let lock_dir = workdir.path().join(".roko").join("runtime");
        std::fs::create_dir_all(&lock_dir).expect("create dir");
        std::fs::write(lock_dir.join("roko.lock"), "").expect("write empty lock");
        let result = check_stale_lock(workdir.path());
        assert_eq!(result.status, PreflightStatus::Pass);
        assert!(result.message.contains("empty"), "message: {}", result.message);
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
        assert!(result.message.contains("stale"), "message: {}", result.message);
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
        assert!(result.message.contains("current process"), "message: {}", result.message);
    }

    #[test]
    fn check_stale_lock_warn_when_non_numeric() {
        let workdir = tempdir().expect("tempdir");
        let lock_dir = workdir.path().join(".roko").join("runtime");
        std::fs::create_dir_all(&lock_dir).expect("create dir");
        std::fs::write(lock_dir.join("roko.lock"), "not-a-pid\n").expect("write lock");
        let result = check_stale_lock(workdir.path());
        assert_eq!(result.status, PreflightStatus::Warn);
        assert!(result.message.contains("non-numeric"), "message: {}", result.message);
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
    fn run_preflight_checks_returns_all_five_checks() {
        let workdir = tempdir().expect("tempdir");
        let plans_dir = workdir.path().join("plans");
        std::fs::create_dir_all(&plans_dir).expect("create plans dir");
        let config = RokoConfig::default();
        let checks = run_preflight_checks(Some(&config), &plans_dir, workdir.path());
        assert_eq!(checks.len(), 5);
        let names: Vec<&str> = checks.iter().map(|c| c.name).collect();
        assert_eq!(names, &["config", "credentials", "plans", "rust", "lock"]);
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
