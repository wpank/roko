//! Pre-flight checks for `roko plan run`.
//!
//! Validates the workspace environment before entering the main event loop:
//! config loadability, LLM credentials, plan directory presence, Rust toolchain,
//! and stale workspace locks.
//!
//! The seven shared checks (`config`, `credentials`, `disk`, `git`, `plans`,
//! `toolchain`, `lock`) are delegated to the shared [`roko_execution::diagnostics`]
//! service. The non-overlapping `impact` check remains local to this module.

use std::path::Path;

use roko_core::config::schema::RokoConfig;
use roko_execution::diagnostics::{
    DiagnosticCheckId, DiagnosticFinding, DiagnosticRequest, DiagnosticService, DiagnosticSeverity,
};

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

/// Convert a shared diagnostic severity to the preflight status.
fn severity_to_status(severity: DiagnosticSeverity) -> PreflightStatus {
    match severity {
        DiagnosticSeverity::Info => PreflightStatus::Pass,
        DiagnosticSeverity::Warning => PreflightStatus::Warn,
        DiagnosticSeverity::Error => PreflightStatus::Fail,
    }
}

/// Convert a shared diagnostic finding to a preflight check.
fn finding_to_preflight(finding: &DiagnosticFinding) -> PreflightCheck {
    PreflightCheck {
        name: finding.check_id.as_str(),
        status: severity_to_status(finding.severity),
        message: finding.message.clone(),
    }
}

/// Run all preflight checks and return the results.
///
/// `config` is the already-loaded `RokoConfig` (or `None` if loading failed
/// earlier). `plans_dir` is the resolved plan directory passed to `plan run`.
/// `workdir` is the workspace root.
///
/// The seven shared checks are delegated to the shared diagnostic service
/// per the fixed adapter matrix. The `impact` check remains local.
pub fn run_preflight_checks(
    _config: Option<&RokoConfig>,
    plans_dir: &Path,
    workdir: &Path,
) -> Vec<PreflightCheck> {
    // Run the shared checks via DiagnosticService.
    let selected = [
        DiagnosticCheckId::Config,
        DiagnosticCheckId::Credentials,
        DiagnosticCheckId::Disk,
        DiagnosticCheckId::Git,
        DiagnosticCheckId::Plans,
        DiagnosticCheckId::Toolchain,
        DiagnosticCheckId::Lock,
    ]
    .into_iter()
    .collect();

    let request = DiagnosticRequest {
        workdir: workdir.to_path_buf(),
        selected,
        profile: None,
        allow_repairs: false,
    };
    let report = DiagnosticService::run(&request);

    // Convert shared findings to PreflightCheck format, keeping one
    // finding per check ID (take the highest severity if multiple).
    let mut checks: Vec<PreflightCheck> = Vec::with_capacity(8);
    let ordered_ids = [
        DiagnosticCheckId::Config,
        DiagnosticCheckId::Credentials,
        DiagnosticCheckId::Disk,
        DiagnosticCheckId::Git,
        DiagnosticCheckId::Plans,
        DiagnosticCheckId::Toolchain,
        DiagnosticCheckId::Lock,
    ];

    for check_id in &ordered_ids {
        // Find the most severe finding for this check ID.
        let worst = report
            .findings
            .iter()
            .filter(|f| &f.check_id == check_id)
            .max_by_key(|f| f.severity);

        if let Some(finding) = worst {
            checks.push(finding_to_preflight(finding));
        }
    }

    // Add the non-overlapping local check.
    checks.push(check_declared_change_impact(plans_dir));

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
                    let result = crate::doctor::probe_one_provider(pid.clone(), url, timeout).await;
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

// ── Legacy individual checks (removed by #279) ──────────────────────────
//
// The following check functions were consolidated into the shared
// `roko_execution::diagnostics` service by backlog packet #279.
// Removed: check_config, check_credentials, check_disk_space,
// check_git_state, check_plans_dir, check_rust_toolchain, check_stale_lock,
// get_available_disk_bytes, is_pid_alive.
//
// These are now implemented in `roko-execution/src/diagnostics/checks.rs`
// and dispatched via `DiagnosticService::run()`.

// Tests for the old individual functions are also replaced; the canonical
// tests now live in `roko-execution/src/diagnostics/service.rs`.

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::config::schema::RokoConfig;
    use tempfile::tempdir;

    #[test]
    fn run_preflight_checks_returns_eight_checks() {
        let workdir = tempdir().expect("tempdir");
        let plans_dir = workdir.path().join("plans");
        std::fs::create_dir_all(&plans_dir).expect("create plans dir");
        let checks = run_preflight_checks(Some(&RokoConfig::default()), &plans_dir, workdir.path());
        // 7 shared checks + 1 local impact check = 8.
        assert_eq!(checks.len(), 8);
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
    fn preflight_status_labels() {
        assert_eq!(PreflightStatus::Pass.label(), "PASS");
        assert_eq!(PreflightStatus::Warn.label(), "WARN");
        assert_eq!(PreflightStatus::Fail.label(), "FAIL");
    }

    #[test]
    fn preflight_checks_include_shared_and_local() {
        let workdir = tempdir().expect("tempdir");
        let plans_dir = workdir.path().join("plans");
        std::fs::create_dir_all(&plans_dir).expect("create plans dir");
        let checks = run_preflight_checks(None, &plans_dir, workdir.path());
        // The last check should be the local 'impact' check.
        assert_eq!(checks.last().map(|c| c.name), Some("impact"));
        // Shared checks should include config, credentials, disk, git, plans, toolchain, lock.
        let shared_names: Vec<&str> = checks.iter().take(7).map(|c| c.name).collect();
        assert!(shared_names.contains(&"config"));
        assert!(shared_names.contains(&"credentials"));
        assert!(shared_names.contains(&"disk"));
        assert!(shared_names.contains(&"git"));
        assert!(shared_names.contains(&"plans"));
        assert!(shared_names.contains(&"toolchain"));
        assert!(shared_names.contains(&"lock"));
    }

    #[test]
    fn preflight_is_read_only() {
        // Spec acceptance: preflight failure launches zero provider/git mutation.
        let workdir = tempdir().expect("tempdir");
        let plans_dir = workdir.path().join("plans");
        std::fs::create_dir_all(&plans_dir).expect("create plans dir");
        // Write a marker file that must survive unchanged.
        let marker = workdir.path().join("marker.txt");
        std::fs::write(&marker, "before").expect("write marker");

        let _ = run_preflight_checks(None, &plans_dir, workdir.path());

        // Verify the workspace was not mutated.
        let contents = std::fs::read_to_string(&marker).expect("read marker");
        assert_eq!(
            contents, "before",
            "preflight must not mutate the workspace"
        );
    }

    #[test]
    fn preflight_severity_mapping_is_consistent() {
        // Verify that severity_to_status maps correctly for all variants.
        assert_eq!(
            severity_to_status(DiagnosticSeverity::Info),
            PreflightStatus::Pass
        );
        assert_eq!(
            severity_to_status(DiagnosticSeverity::Warning),
            PreflightStatus::Warn
        );
        assert_eq!(
            severity_to_status(DiagnosticSeverity::Error),
            PreflightStatus::Fail
        );
    }
}
