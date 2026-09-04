//! Individual check functions for the shared diagnostic service.
//!
//! Each function corresponds to exactly one [`DiagnosticCheckId`] and returns
//! one or more [`DiagnosticFinding`] values. No check starts an unbounded
//! server/provider process or mutates git/config during `run`.

use super::types::{
    DiagnosticCheckId, DiagnosticFinding, DiagnosticRemediation, DiagnosticSeverity,
};
use std::collections::BTreeMap;
use std::path::Path;

// ── config ───────────────────────────────────────────────────────────────

/// Check whether a config file is present and parseable at the workspace root.
pub fn check_config(workdir: &Path) -> Vec<DiagnosticFinding> {
    let toml_path = workdir.join("roko.toml");
    if !toml_path.exists() {
        // Also check for global config.
        let global = global_config_path();
        if global.as_ref().is_some_and(|p| p.is_file()) {
            return vec![DiagnosticFinding {
                check_id: DiagnosticCheckId::Config,
                code: "config_project_missing".into(),
                severity: DiagnosticSeverity::Warning,
                message: "project roko.toml not found; using global config only".into(),
                remediation: Some(DiagnosticRemediation {
                    summary: "create a project config".into(),
                    command: Some("roko init".into()),
                    mutation_required: true,
                }),
                evidence: evidence([("path", toml_path.display().to_string())]),
            }];
        }
        return vec![DiagnosticFinding {
            check_id: DiagnosticCheckId::Config,
            code: "config_missing".into(),
            severity: DiagnosticSeverity::Error,
            message: format!("roko.toml not found at {}", toml_path.display()),
            remediation: Some(DiagnosticRemediation {
                summary: "initialize workspace config".into(),
                command: Some("roko init".into()),
                mutation_required: true,
            }),
            evidence: evidence([("path", toml_path.display().to_string())]),
        }];
    }

    // Try to parse.
    match std::fs::read_to_string(&toml_path) {
        Ok(text) => {
            if toml::from_str::<toml::Value>(&text).is_err() {
                return vec![DiagnosticFinding {
                    check_id: DiagnosticCheckId::Config,
                    code: "config_parse_error".into(),
                    severity: DiagnosticSeverity::Error,
                    message: "roko.toml exists but cannot be parsed".into(),
                    remediation: Some(DiagnosticRemediation {
                        summary: "fix TOML syntax errors".into(),
                        command: None,
                        mutation_required: true,
                    }),
                    evidence: evidence([("path", toml_path.display().to_string())]),
                }];
            }
            // Config present and parseable.
            vec![DiagnosticFinding {
                check_id: DiagnosticCheckId::Config,
                code: "config_ok".into(),
                severity: DiagnosticSeverity::Info,
                message: format!("roko.toml loaded from {}", workdir.display()),
                remediation: None,
                evidence: evidence([("path", toml_path.display().to_string())]),
            }]
        }
        Err(e) => vec![DiagnosticFinding {
            check_id: DiagnosticCheckId::Config,
            code: "config_read_error".into(),
            severity: DiagnosticSeverity::Error,
            message: format!("cannot read roko.toml: {e}"),
            remediation: Some(DiagnosticRemediation {
                summary: "check file permissions".into(),
                command: None,
                mutation_required: false,
            }),
            evidence: evidence([("path", toml_path.display().to_string())]),
        }],
    }
}

// ── credentials ──────────────────────────────────────────────────────────

/// Check that at least one LLM provider has usable credentials.
pub fn check_credentials(workdir: &Path) -> Vec<DiagnosticFinding> {
    // Check well-known API key environment variables.
    let known_keys = [
        ("ANTHROPIC_API_KEY", "anthropic"),
        ("OPENAI_API_KEY", "openai"),
        ("GEMINI_API_KEY", "gemini"),
        ("ZAI_API_KEY", "zhipu"),
        ("PERPLEXITY_API_KEY", "perplexity"),
    ];

    let mut available = Vec::new();
    for (env_var, label) in &known_keys {
        if std::env::var(env_var)
            .ok()
            .filter(|v| !v.is_empty())
            .is_some()
        {
            available.push(*label);
        }
    }

    // Check for claude CLI on PATH.
    if command_exists("claude") {
        available.push("claude-cli");
    }

    if available.is_empty() {
        // Check if there is a config with providers that might have keys.
        let _ = workdir; // Used for context only.
        vec![DiagnosticFinding {
            check_id: DiagnosticCheckId::Credentials,
            code: "credentials_none".into(),
            severity: DiagnosticSeverity::Error,
            message:
                "no provider has credentials -- set an API key env var or install a CLI provider"
                    .into(),
            remediation: Some(DiagnosticRemediation {
                summary: "set a provider API key".into(),
                command: Some("export ANTHROPIC_API_KEY=sk-...".into()),
                mutation_required: false,
            }),
            evidence: BTreeMap::new(),
        }]
    } else {
        vec![DiagnosticFinding {
            check_id: DiagnosticCheckId::Credentials,
            code: "credentials_ok".into(),
            severity: DiagnosticSeverity::Info,
            message: format!("provider(s) available: {}", available.join(", ")),
            remediation: None,
            evidence: evidence([("providers", available.join(", "))]),
        }]
    }
}

// ── disk ──────────────────────────────────────────────────────────────────

/// Minimum free disk space required (2 GB).
const MIN_FREE_DISK_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// Check that there is at least 2 GB of free disk space.
pub fn check_disk(workdir: &Path) -> Vec<DiagnosticFinding> {
    match get_available_disk_bytes(workdir) {
        Some(avail) if avail >= MIN_FREE_DISK_BYTES => vec![DiagnosticFinding {
            check_id: DiagnosticCheckId::Disk,
            code: "disk_ok".into(),
            severity: DiagnosticSeverity::Info,
            message: format!("{:.1} GB free", avail as f64 / (1024.0 * 1024.0 * 1024.0)),
            remediation: None,
            evidence: evidence([("free_bytes", avail.to_string())]),
        }],
        Some(avail) => vec![DiagnosticFinding {
            check_id: DiagnosticCheckId::Disk,
            code: "disk_low".into(),
            severity: DiagnosticSeverity::Error,
            message: format!(
                "only {:.1} GB free (need >= 2 GB)",
                avail as f64 / (1024.0 * 1024.0 * 1024.0)
            ),
            remediation: Some(DiagnosticRemediation {
                summary: "free up disk space".into(),
                command: Some("cargo clean".into()),
                mutation_required: true,
            }),
            evidence: evidence([("free_bytes", avail.to_string())]),
        }],
        None => vec![DiagnosticFinding {
            check_id: DiagnosticCheckId::Disk,
            code: "disk_unknown".into(),
            severity: DiagnosticSeverity::Warning,
            message: "could not determine free disk space".into(),
            remediation: None,
            evidence: BTreeMap::new(),
        }],
    }
}

// ── git ──────────────────────────────────────────────────────────────────

/// Check git working directory state.
pub fn check_git(workdir: &Path) -> Vec<DiagnosticFinding> {
    // Check if inside a git repo.
    let git_check = std::process::Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(workdir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output();

    match git_check {
        Ok(out) if out.status.success() => {}
        _ => {
            return vec![DiagnosticFinding {
                check_id: DiagnosticCheckId::Git,
                code: "git_not_repo".into(),
                severity: DiagnosticSeverity::Warning,
                message: "not inside a git repository".into(),
                remediation: Some(DiagnosticRemediation {
                    summary: "initialize a git repo".into(),
                    command: Some("git init".into()),
                    mutation_required: true,
                }),
                evidence: evidence([("workdir", workdir.display().to_string())]),
            }];
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
                vec![DiagnosticFinding {
                    check_id: DiagnosticCheckId::Git,
                    code: "git_clean".into(),
                    severity: DiagnosticSeverity::Info,
                    message: "working tree is clean".into(),
                    remediation: None,
                    evidence: BTreeMap::new(),
                }]
            } else {
                let changed = stdout.lines().count();
                vec![DiagnosticFinding {
                    check_id: DiagnosticCheckId::Git,
                    code: "git_dirty".into(),
                    severity: DiagnosticSeverity::Warning,
                    message: format!(
                        "working tree has {changed} uncommitted change(s) \u{2014} \
                         consider committing before plan run to avoid worktree merge conflicts"
                    ),
                    remediation: Some(DiagnosticRemediation {
                        summary: "commit or stash changes".into(),
                        command: Some("git add -A && git commit".into()),
                        mutation_required: true,
                    }),
                    evidence: evidence([("changed_count", changed.to_string())]),
                }]
            }
        }
        _ => vec![DiagnosticFinding {
            check_id: DiagnosticCheckId::Git,
            code: "git_status_error".into(),
            severity: DiagnosticSeverity::Warning,
            message: "could not determine git status".into(),
            remediation: None,
            evidence: BTreeMap::new(),
        }],
    }
}

// ── plans ─────────────────────────────────────────────────────────────────

/// Check that the plans directory exists and contains tasks.toml files.
pub fn check_plans(workdir: &Path) -> Vec<DiagnosticFinding> {
    let plans_dir = workdir.join("plans");

    if !plans_dir.exists() {
        // Also check .roko/plans/.
        let dot_plans = workdir.join(".roko").join("plans");
        if dot_plans.is_dir() {
            return vec![DiagnosticFinding {
                check_id: DiagnosticCheckId::Plans,
                code: "plans_ok".into(),
                severity: DiagnosticSeverity::Info,
                message: format!("plans found at {}", dot_plans.display()),
                remediation: None,
                evidence: evidence([("path", dot_plans.display().to_string())]),
            }];
        }
        return vec![DiagnosticFinding {
            check_id: DiagnosticCheckId::Plans,
            code: "plans_dir_missing".into(),
            severity: DiagnosticSeverity::Warning,
            message: format!("plans directory does not exist: {}", plans_dir.display()),
            remediation: Some(DiagnosticRemediation {
                summary: "create a plan".into(),
                command: Some("roko plan create <name>".into()),
                mutation_required: true,
            }),
            evidence: evidence([("path", plans_dir.display().to_string())]),
        }];
    }

    // Count tasks.toml files.
    let direct = plans_dir.join("tasks.toml");
    if direct.is_file() {
        return vec![DiagnosticFinding {
            check_id: DiagnosticCheckId::Plans,
            code: "plans_ok".into(),
            severity: DiagnosticSeverity::Info,
            message: format!("found tasks.toml in {}", plans_dir.display()),
            remediation: None,
            evidence: evidence([("path", plans_dir.display().to_string())]),
        }];
    }

    let task_count = std::fs::read_dir(&plans_dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|e| e.path().is_dir() && e.path().join("tasks.toml").is_file())
                .count()
        })
        .unwrap_or(0);

    if task_count == 0 {
        vec![DiagnosticFinding {
            check_id: DiagnosticCheckId::Plans,
            code: "plans_empty".into(),
            severity: DiagnosticSeverity::Warning,
            message: format!(
                "no tasks.toml found in {} or its subdirectories",
                plans_dir.display()
            ),
            remediation: Some(DiagnosticRemediation {
                summary: "create or generate a plan".into(),
                command: Some("roko plan create <name>".into()),
                mutation_required: true,
            }),
            evidence: evidence([("path", plans_dir.display().to_string())]),
        }]
    } else {
        vec![DiagnosticFinding {
            check_id: DiagnosticCheckId::Plans,
            code: "plans_ok".into(),
            severity: DiagnosticSeverity::Info,
            message: format!("{task_count} plan(s) found in {}", plans_dir.display()),
            remediation: None,
            evidence: evidence([
                ("path", plans_dir.display().to_string()),
                ("count", task_count.to_string()),
            ]),
        }]
    }
}

// ── toolchain ────────────────────────────────────────────────────────────

/// Check that `rustc` is available and meets the minimum version (1.91).
pub fn check_toolchain(_workdir: &Path) -> Vec<DiagnosticFinding> {
    match std::process::Command::new("rustc")
        .arg("--version")
        .output()
    {
        Ok(output) if output.status.success() => {
            let version_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let minor = version_str
                .split_whitespace()
                .nth(1)
                .and_then(|v| v.split('.').nth(1))
                .and_then(|m| m.parse::<u32>().ok())
                .unwrap_or(0);

            if minor >= 91 {
                vec![DiagnosticFinding {
                    check_id: DiagnosticCheckId::Toolchain,
                    code: "toolchain_ok".into(),
                    severity: DiagnosticSeverity::Info,
                    message: version_str,
                    remediation: None,
                    evidence: evidence([("minor", minor.to_string())]),
                }]
            } else {
                vec![DiagnosticFinding {
                    check_id: DiagnosticCheckId::Toolchain,
                    code: "toolchain_version_low".into(),
                    severity: DiagnosticSeverity::Error,
                    message: format!("Rust version below 1.91 ({version_str})"),
                    remediation: Some(DiagnosticRemediation {
                        summary: "update Rust toolchain".into(),
                        command: Some("rustup update stable".into()),
                        mutation_required: true,
                    }),
                    evidence: evidence([("version", version_str), ("minor", minor.to_string())]),
                }]
            }
        }
        Ok(_) => vec![DiagnosticFinding {
            check_id: DiagnosticCheckId::Toolchain,
            code: "toolchain_broken".into(),
            severity: DiagnosticSeverity::Warning,
            message: "rustc found but returned non-zero -- toolchain may be broken".into(),
            remediation: Some(DiagnosticRemediation {
                summary: "reinstall Rust toolchain".into(),
                command: Some("rustup update stable".into()),
                mutation_required: true,
            }),
            evidence: BTreeMap::new(),
        }],
        Err(_) => vec![DiagnosticFinding {
            check_id: DiagnosticCheckId::Toolchain,
            code: "toolchain_missing".into(),
            severity: DiagnosticSeverity::Warning,
            message: "rustc not found on PATH -- gate checks that compile will fail".into(),
            remediation: Some(DiagnosticRemediation {
                summary: "install Rust".into(),
                command: Some("rustup update stable".into()),
                mutation_required: true,
            }),
            evidence: BTreeMap::new(),
        }],
    }
}

// ── lock ──────────────────────────────────────────────────────────────────

/// Check for a stale workspace lock.
pub fn check_lock(workdir: &Path) -> Vec<DiagnosticFinding> {
    let lock_path = workdir.join(".roko").join("runtime").join("roko.lock");
    if !lock_path.exists() {
        return vec![DiagnosticFinding {
            check_id: DiagnosticCheckId::Lock,
            code: "lock_absent".into(),
            severity: DiagnosticSeverity::Info,
            message: "no existing workspace lock".into(),
            remediation: None,
            evidence: BTreeMap::new(),
        }];
    }

    let pid_str = match std::fs::read_to_string(&lock_path) {
        Ok(s) => s.trim().to_string(),
        Err(_) => {
            return vec![DiagnosticFinding {
                check_id: DiagnosticCheckId::Lock,
                code: "lock_unreadable".into(),
                severity: DiagnosticSeverity::Warning,
                message: format!("lock file exists but unreadable: {}", lock_path.display()),
                remediation: None,
                evidence: evidence([("path", lock_path.display().to_string())]),
            }];
        }
    };

    if pid_str.is_empty() {
        return vec![DiagnosticFinding {
            check_id: DiagnosticCheckId::Lock,
            code: "lock_empty".into(),
            severity: DiagnosticSeverity::Info,
            message: "lock file exists but is empty (clean shutdown)".into(),
            remediation: None,
            evidence: BTreeMap::new(),
        }];
    }

    let pid: u32 = match pid_str.parse() {
        Ok(p) => p,
        Err(_) => {
            return vec![DiagnosticFinding {
                check_id: DiagnosticCheckId::Lock,
                code: "lock_invalid_pid".into(),
                severity: DiagnosticSeverity::Warning,
                message: format!("lock file contains non-numeric PID: {pid_str:?}"),
                remediation: None,
                evidence: evidence([("pid_raw", pid_str)]),
            }];
        }
    };

    // Current process owns the lock.
    if pid == std::process::id() {
        return vec![DiagnosticFinding {
            check_id: DiagnosticCheckId::Lock,
            code: "lock_self".into(),
            severity: DiagnosticSeverity::Info,
            message: "workspace lock held by current process".into(),
            remediation: None,
            evidence: evidence([("pid", pid.to_string())]),
        }];
    }

    if is_pid_alive(pid) {
        vec![DiagnosticFinding {
            check_id: DiagnosticCheckId::Lock,
            code: "lock_conflict".into(),
            severity: DiagnosticSeverity::Error,
            message: format!(
                "another roko process is running (PID {pid}) -- wait for it to finish or kill it"
            ),
            remediation: Some(DiagnosticRemediation {
                summary: "wait or kill the other process".into(),
                command: Some(format!("kill {pid}")),
                mutation_required: true,
            }),
            evidence: evidence([("pid", pid.to_string())]),
        }]
    } else {
        vec![DiagnosticFinding {
            check_id: DiagnosticCheckId::Lock,
            code: "lock_stale".into(),
            severity: DiagnosticSeverity::Warning,
            message: format!(
                "stale lock from dead PID {pid} -- will be replaced on lock acquisition"
            ),
            remediation: Some(DiagnosticRemediation {
                summary: "remove stale lock".into(),
                command: Some(format!("rm {}", lock_path.display())),
                mutation_required: true,
            }),
            evidence: evidence([("pid", pid.to_string())]),
        }]
    }
}

// ── workspace ────────────────────────────────────────────────────────────

/// Check that the `.roko/` workspace layout exists with required subdirectories.
pub fn check_workspace(workdir: &Path) -> Vec<DiagnosticFinding> {
    let roko_dir = workdir.join(".roko");

    if !roko_dir.is_dir() {
        return vec![DiagnosticFinding {
            check_id: DiagnosticCheckId::Workspace,
            code: "workspace_missing".into(),
            severity: DiagnosticSeverity::Error,
            message: "missing .roko directory".into(),
            remediation: Some(DiagnosticRemediation {
                summary: "initialize workspace".into(),
                command: Some("roko init".into()),
                mutation_required: true,
            }),
            evidence: evidence([("path", roko_dir.display().to_string())]),
        }];
    }

    // Check key subdirectories.
    let expected_dirs = ["state", "learn", "memory", "prd"];
    let mut missing = Vec::new();
    for dir_name in &expected_dirs {
        let dir_path = roko_dir.join(dir_name);
        if !dir_path.is_dir() {
            missing.push(*dir_name);
        }
    }

    if missing.is_empty() {
        vec![DiagnosticFinding {
            check_id: DiagnosticCheckId::Workspace,
            code: "workspace_ok".into(),
            severity: DiagnosticSeverity::Info,
            message: ".roko layout basics are present".into(),
            remediation: None,
            evidence: evidence([("path", roko_dir.display().to_string())]),
        }]
    } else {
        vec![DiagnosticFinding {
            check_id: DiagnosticCheckId::Workspace,
            code: "workspace_incomplete".into(),
            severity: DiagnosticSeverity::Warning,
            message: format!(
                "required .roko layout paths are missing: {}",
                missing.join(", ")
            ),
            remediation: Some(DiagnosticRemediation {
                summary: "reinitialize workspace".into(),
                command: Some("roko init".into()),
                mutation_required: true,
            }),
            evidence: evidence([
                ("path", roko_dir.display().to_string()),
                ("missing", missing.join(", ")),
            ]),
        }]
    }
}

// ── schema_version ───────────────────────────────────────────────────────

/// Check config and storage schema version compatibility.
#[allow(clippy::too_many_lines)]
pub fn check_schema_version(workdir: &Path) -> Vec<DiagnosticFinding> {
    let mut findings = Vec::new();

    // Check .roko/VERSION file.
    let version_path = workdir.join(".roko").join("VERSION");
    if version_path.is_file() {
        let version_str = std::fs::read_to_string(&version_path)
            .unwrap_or_default()
            .trim()
            .to_string();
        match version_str.parse::<u32>() {
            Ok(3) => {
                findings.push(DiagnosticFinding {
                    check_id: DiagnosticCheckId::SchemaVersion,
                    code: "schema_version_current".into(),
                    severity: DiagnosticSeverity::Info,
                    message: "storage layout is V3 (current)".into(),
                    remediation: None,
                    evidence: evidence([("version", version_str)]),
                });
            }
            Ok(v) => {
                findings.push(DiagnosticFinding {
                    check_id: DiagnosticCheckId::SchemaVersion,
                    code: "schema_version_outdated".into(),
                    severity: DiagnosticSeverity::Warning,
                    message: format!("storage layout is V{v} (outdated)"),
                    remediation: Some(DiagnosticRemediation {
                        summary: "run migration".into(),
                        command: Some("roko init".into()),
                        mutation_required: true,
                    }),
                    evidence: evidence([("version", version_str)]),
                });
            }
            Err(_) => {
                findings.push(DiagnosticFinding {
                    check_id: DiagnosticCheckId::SchemaVersion,
                    code: "schema_version_invalid".into(),
                    severity: DiagnosticSeverity::Warning,
                    message: format!(".roko/VERSION contains unrecognized value: {version_str:?}"),
                    remediation: Some(DiagnosticRemediation {
                        summary: "reinitialize workspace".into(),
                        command: Some("roko init".into()),
                        mutation_required: true,
                    }),
                    evidence: evidence([("raw_value", version_str)]),
                });
            }
        }
    } else if workdir.join(".roko").is_dir() {
        findings.push(DiagnosticFinding {
            check_id: DiagnosticCheckId::SchemaVersion,
            code: "schema_version_missing".into(),
            severity: DiagnosticSeverity::Warning,
            message: ".roko/VERSION file is missing".into(),
            remediation: Some(DiagnosticRemediation {
                summary: "create version file".into(),
                command: Some("roko init".into()),
                mutation_required: true,
            }),
            evidence: BTreeMap::new(),
        });
    }

    // Check config_version in roko.toml.
    let toml_path = workdir.join("roko.toml");
    if let Ok(text) = std::fs::read_to_string(&toml_path)
        && let Ok(value) = toml::from_str::<toml::Value>(&text)
    {
        if let Some(cv) = value.get("config_version").and_then(|v| v.as_integer()) {
            findings.push(DiagnosticFinding {
                check_id: DiagnosticCheckId::SchemaVersion,
                code: "config_version_present".into(),
                severity: DiagnosticSeverity::Info,
                message: format!("config_version: {cv}"),
                remediation: None,
                evidence: evidence([("config_version", cv.to_string())]),
            });
        }
        if let Some(sv) = value.get("schema_version").and_then(|v| v.as_integer()) {
            findings.push(DiagnosticFinding {
                check_id: DiagnosticCheckId::SchemaVersion,
                code: "schema_version_config".into(),
                severity: DiagnosticSeverity::Info,
                message: format!("schema_version: {sv}"),
                remediation: None,
                evidence: evidence([("schema_version", sv.to_string())]),
            });
        }
    }

    if findings.is_empty() {
        findings.push(DiagnosticFinding {
            check_id: DiagnosticCheckId::SchemaVersion,
            code: "schema_version_unavailable".into(),
            severity: DiagnosticSeverity::Warning,
            message: "no version information available".into(),
            remediation: Some(DiagnosticRemediation {
                summary: "initialize workspace".into(),
                command: Some("roko init".into()),
                mutation_required: true,
            }),
            evidence: BTreeMap::new(),
        });
    }

    findings
}

// ── providers ────────────────────────────────────────────────────────────

/// Check that configured LLM providers are available.
pub fn check_providers(workdir: &Path) -> Vec<DiagnosticFinding> {
    let toml_path = workdir.join("roko.toml");
    let text = match std::fs::read_to_string(&toml_path) {
        Ok(t) => t,
        Err(_) => {
            // No config; check for common env vars.
            let any = ["ANTHROPIC_API_KEY", "OPENAI_API_KEY", "GEMINI_API_KEY"]
                .iter()
                .any(|k| std::env::var(k).ok().filter(|v| !v.is_empty()).is_some());
            let cli = command_exists("claude");
            if any || cli {
                return vec![DiagnosticFinding {
                    check_id: DiagnosticCheckId::Providers,
                    code: "providers_detected".into(),
                    severity: DiagnosticSeverity::Info,
                    message: "provider(s) detected via environment/CLI (no roko.toml)".into(),
                    remediation: None,
                    evidence: BTreeMap::new(),
                }];
            }
            return vec![DiagnosticFinding {
                check_id: DiagnosticCheckId::Providers,
                code: "providers_none".into(),
                severity: DiagnosticSeverity::Warning,
                message: "no providers detected -- no API keys set and no CLI tools found".into(),
                remediation: Some(DiagnosticRemediation {
                    summary: "configure a provider".into(),
                    command: Some("roko config init".into()),
                    mutation_required: true,
                }),
                evidence: BTreeMap::new(),
            }];
        }
    };

    let value: toml::Value = match toml::from_str(&text) {
        Ok(v) => v,
        Err(_) => {
            return vec![DiagnosticFinding {
                check_id: DiagnosticCheckId::Providers,
                code: "providers_config_error".into(),
                severity: DiagnosticSeverity::Warning,
                message: "cannot parse roko.toml to check providers".into(),
                remediation: None,
                evidence: BTreeMap::new(),
            }];
        }
    };

    let provider_count = value
        .get("providers")
        .and_then(|v| v.as_table())
        .map_or(0, |t| t.len());

    if provider_count == 0 {
        // Still check for env-based providers.
        let has_key = ["ANTHROPIC_API_KEY", "OPENAI_API_KEY", "GEMINI_API_KEY"]
            .iter()
            .any(|k| std::env::var(k).ok().filter(|v| !v.is_empty()).is_some());
        if has_key || command_exists("claude") {
            return vec![DiagnosticFinding {
                check_id: DiagnosticCheckId::Providers,
                code: "providers_implicit".into(),
                severity: DiagnosticSeverity::Info,
                message: "no providers in config but env/CLI providers detected".into(),
                remediation: None,
                evidence: evidence([("configured", "0".into())]),
            }];
        }
        return vec![DiagnosticFinding {
            check_id: DiagnosticCheckId::Providers,
            code: "providers_none_configured".into(),
            severity: DiagnosticSeverity::Warning,
            message: "no providers configured in roko.toml".into(),
            remediation: Some(DiagnosticRemediation {
                summary: "add a provider".into(),
                command: Some("roko config init".into()),
                mutation_required: true,
            }),
            evidence: BTreeMap::new(),
        }];
    }

    vec![DiagnosticFinding {
        check_id: DiagnosticCheckId::Providers,
        code: "providers_ok".into(),
        severity: DiagnosticSeverity::Info,
        message: format!("{provider_count} provider(s) configured"),
        remediation: None,
        evidence: evidence([("count", provider_count.to_string())]),
    }]
}

// ── models ───────────────────────────────────────────────────────────────

/// Check that a default model is configured and valid.
pub fn check_models(workdir: &Path) -> Vec<DiagnosticFinding> {
    let toml_path = workdir.join("roko.toml");
    let text = match std::fs::read_to_string(&toml_path) {
        Ok(t) => t,
        Err(_) => {
            return vec![DiagnosticFinding {
                check_id: DiagnosticCheckId::Models,
                code: "models_no_config".into(),
                severity: DiagnosticSeverity::Warning,
                message: "config unavailable; default model not evaluated".into(),
                remediation: None,
                evidence: BTreeMap::new(),
            }];
        }
    };

    let value: toml::Value = match toml::from_str(&text) {
        Ok(v) => v,
        Err(_) => {
            return vec![DiagnosticFinding {
                check_id: DiagnosticCheckId::Models,
                code: "models_config_error".into(),
                severity: DiagnosticSeverity::Warning,
                message: "cannot parse roko.toml to check models".into(),
                remediation: None,
                evidence: BTreeMap::new(),
            }];
        }
    };

    let model_key = value
        .get("agent")
        .and_then(|a| a.get("model"))
        .and_then(|m| m.as_str())
        .unwrap_or("")
        .trim();

    if model_key.is_empty() {
        return vec![DiagnosticFinding {
            check_id: DiagnosticCheckId::Models,
            code: "models_no_default".into(),
            severity: DiagnosticSeverity::Warning,
            message: "no default_model configured".into(),
            remediation: Some(DiagnosticRemediation {
                summary: "set default model".into(),
                command: Some("roko config set agent.model <model-name>".into()),
                mutation_required: true,
            }),
            evidence: BTreeMap::new(),
        }];
    }

    // Check if model exists in the models table.
    let in_table = value
        .get("models")
        .and_then(|m| m.as_table())
        .is_some_and(|t| t.contains_key(model_key));

    if in_table {
        vec![DiagnosticFinding {
            check_id: DiagnosticCheckId::Models,
            code: "models_ok".into(),
            severity: DiagnosticSeverity::Info,
            message: format!("default_model \"{model_key}\" found in models table"),
            remediation: None,
            evidence: evidence([("model", model_key.to_string())]),
        }]
    } else {
        // Could be a built-in model; this is not necessarily an error.
        vec![DiagnosticFinding {
            check_id: DiagnosticCheckId::Models,
            code: "models_implicit".into(),
            severity: DiagnosticSeverity::Info,
            message: format!(
                "default_model \"{model_key}\" not in models table (may be a built-in)"
            ),
            remediation: None,
            evidence: evidence([("model", model_key.to_string())]),
        }]
    }
}

// ── Utility helpers ──────────────────────────────────────────────────────

/// Check if a command exists on `PATH`.
fn command_exists(cmd: &str) -> bool {
    std::process::Command::new("which")
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Check if a process with the given PID is alive (Unix only).
fn is_pid_alive(pid: u32) -> bool {
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Query available disk bytes on the partition containing `path`.
fn get_available_disk_bytes(path: &Path) -> Option<u64> {
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

/// Resolve the global config path (`~/.roko/config.toml`).
fn global_config_path() -> Option<std::path::PathBuf> {
    let home = std::env::var_os("HOME")?;
    if home.is_empty() {
        return None;
    }
    Some(
        std::path::PathBuf::from(home)
            .join(".roko")
            .join("config.toml"),
    )
}

/// Build a `BTreeMap` from an array of key-value pairs.
fn evidence<const N: usize>(pairs: [(&str, String); N]) -> BTreeMap<String, String> {
    pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}
