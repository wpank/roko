//! `DiagnosticService` -- the shared entry point for all diagnostic callers.
//!
//! Callers select a set of [`DiagnosticCheckId`] values via
//! [`DiagnosticRequest`] and receive a sorted [`DiagnosticReport`].
//! `run()` is read-only when `allow_repairs = false` (which Graph preflight
//! always passes). Repair operations use the separately named
//! [`apply_repair`] and reject absent explicit approval.

use super::checks;
use super::types::{
    DiagnosticCheckId, DiagnosticFinding, DiagnosticReport, DiagnosticRequest, now_ms,
};

/// Shared diagnostic service that consolidates all workspace health checks.
///
/// This is a stateless service -- each call to [`run`] executes the selected
/// checks against the provided workspace directory without retaining state
/// between calls.
pub struct DiagnosticService;

impl DiagnosticService {
    /// Run the selected diagnostic checks and return a sorted report.
    ///
    /// When `request.allow_repairs` is `false` (the default for Graph
    /// preflight), no check may mutate the workspace, git state, or
    /// configuration. All checks are side-effect-free reads.
    ///
    /// # Panics
    ///
    /// Does not panic.
    pub fn run(request: &DiagnosticRequest) -> DiagnosticReport {
        let started = now_ms();
        let mut findings = Vec::new();

        for check_id in &request.selected {
            let check_findings = run_single_check(*check_id, &request.workdir);
            findings.extend(check_findings);
        }

        DiagnosticReport::new(findings, started)
    }

    /// Apply a repair for a specific finding code.
    ///
    /// This is intentionally separate from `run()` and requires explicit
    /// approval. Returns an error if approval is not given or the finding
    /// code is not recognized.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - `approval` is `false` (repair rejected)
    /// - `finding_code` is not recognized
    /// - The repair operation itself fails
    pub fn apply_repair(
        finding_code: &str,
        approval: bool,
    ) -> Result<String, DiagnosticRepairError> {
        if !approval {
            return Err(DiagnosticRepairError::NotApproved);
        }

        // For now, no automated repairs are implemented. Each recognized
        // finding code maps to a manual remediation command in the finding's
        // `remediation.command` field. This method is the boundary where
        // future automated repairs will be added.
        Err(DiagnosticRepairError::UnrecognizedCode(
            finding_code.to_string(),
        ))
    }
}

/// Errors from [`DiagnosticService::apply_repair`].
#[derive(Debug, thiserror::Error)]
pub enum DiagnosticRepairError {
    /// The caller did not approve the repair.
    #[error("repair not approved: explicit approval is required")]
    NotApproved,
    /// The finding code is not recognized for automated repair.
    #[error("unrecognized finding code for repair: {0}")]
    UnrecognizedCode(String),
}

/// Dispatch a single check ID to its implementation.
fn run_single_check(
    check_id: DiagnosticCheckId,
    workdir: &std::path::Path,
) -> Vec<DiagnosticFinding> {
    match check_id {
        DiagnosticCheckId::Config => checks::check_config(workdir),
        DiagnosticCheckId::Credentials => checks::check_credentials(workdir),
        DiagnosticCheckId::Disk => checks::check_disk(workdir),
        DiagnosticCheckId::Git => checks::check_git(workdir),
        DiagnosticCheckId::Plans => checks::check_plans(workdir),
        DiagnosticCheckId::Toolchain => checks::check_toolchain(workdir),
        DiagnosticCheckId::Lock => checks::check_lock(workdir),
        DiagnosticCheckId::Workspace => checks::check_workspace(workdir),
        DiagnosticCheckId::SchemaVersion => checks::check_schema_version(workdir),
        DiagnosticCheckId::Providers => checks::check_providers(workdir),
        DiagnosticCheckId::Models => checks::check_models(workdir),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::types::{DiagnosticCheckId, DiagnosticSeverity};
    use std::collections::BTreeSet;

    #[test]
    fn run_with_empty_selection_returns_empty_report() {
        let dir = tempfile::tempdir().expect("tempdir");
        let request = DiagnosticRequest {
            workdir: dir.path().to_path_buf(),
            selected: BTreeSet::new(),
            profile: None,
            allow_repairs: false,
        };
        let report = DiagnosticService::run(&request);
        assert!(report.findings.is_empty());
        assert!(report.started_at_ms <= report.completed_at_ms);
    }

    #[test]
    fn run_config_check_on_empty_dir() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut selected = BTreeSet::new();
        selected.insert(DiagnosticCheckId::Config);
        let request = DiagnosticRequest {
            workdir: dir.path().to_path_buf(),
            selected,
            profile: None,
            allow_repairs: false,
        };
        let report = DiagnosticService::run(&request);
        assert!(!report.findings.is_empty());
        // Config should be missing.
        assert!(report.findings.iter().any(|f| f.check_id == DiagnosticCheckId::Config
            && matches!(
                f.severity,
                DiagnosticSeverity::Error | DiagnosticSeverity::Warning
            )));
    }

    #[test]
    fn run_config_check_with_valid_toml() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("roko.toml"),
            "[agent]\nmodel = \"test\"\n",
        )
        .expect("write toml");
        let mut selected = BTreeSet::new();
        selected.insert(DiagnosticCheckId::Config);
        let request = DiagnosticRequest {
            workdir: dir.path().to_path_buf(),
            selected,
            profile: None,
            allow_repairs: false,
        };
        let report = DiagnosticService::run(&request);
        assert!(report.findings.iter().any(|f| f.code == "config_ok"));
    }

    #[test]
    fn run_config_check_with_invalid_toml() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("roko.toml"), "{{invalid}}")
            .expect("write bad toml");
        let mut selected = BTreeSet::new();
        selected.insert(DiagnosticCheckId::Config);
        let request = DiagnosticRequest {
            workdir: dir.path().to_path_buf(),
            selected,
            profile: None,
            allow_repairs: false,
        };
        let report = DiagnosticService::run(&request);
        assert!(report.has_errors());
        assert!(report.findings.iter().any(|f| f.code == "config_parse_error"));
    }

    #[test]
    fn run_workspace_check_on_empty_dir() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut selected = BTreeSet::new();
        selected.insert(DiagnosticCheckId::Workspace);
        let request = DiagnosticRequest {
            workdir: dir.path().to_path_buf(),
            selected,
            profile: None,
            allow_repairs: false,
        };
        let report = DiagnosticService::run(&request);
        assert!(report.has_errors());
        assert!(report.findings.iter().any(|f| f.code == "workspace_missing"));
    }

    #[test]
    fn run_workspace_check_with_roko_dir() {
        let dir = tempfile::tempdir().expect("tempdir");
        for sub in ["state", "learn", "memory", "prd"] {
            std::fs::create_dir_all(dir.path().join(".roko").join(sub))
                .expect("create dir");
        }
        let mut selected = BTreeSet::new();
        selected.insert(DiagnosticCheckId::Workspace);
        let request = DiagnosticRequest {
            workdir: dir.path().to_path_buf(),
            selected,
            profile: None,
            allow_repairs: false,
        };
        let report = DiagnosticService::run(&request);
        assert!(!report.has_errors());
        assert!(report.findings.iter().any(|f| f.code == "workspace_ok"));
    }

    #[test]
    fn run_lock_check_no_lock() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut selected = BTreeSet::new();
        selected.insert(DiagnosticCheckId::Lock);
        let request = DiagnosticRequest {
            workdir: dir.path().to_path_buf(),
            selected,
            profile: None,
            allow_repairs: false,
        };
        let report = DiagnosticService::run(&request);
        assert!(!report.has_errors());
        assert!(report.findings.iter().any(|f| f.code == "lock_absent"));
    }

    #[test]
    fn run_lock_check_stale_pid() {
        let dir = tempfile::tempdir().expect("tempdir");
        let lock_dir = dir.path().join(".roko").join("runtime");
        std::fs::create_dir_all(&lock_dir).expect("create dir");
        std::fs::write(lock_dir.join("roko.lock"), "999999999\n").expect("write lock");
        let mut selected = BTreeSet::new();
        selected.insert(DiagnosticCheckId::Lock);
        let request = DiagnosticRequest {
            workdir: dir.path().to_path_buf(),
            selected,
            profile: None,
            allow_repairs: false,
        };
        let report = DiagnosticService::run(&request);
        assert!(report.findings.iter().any(|f| f.code == "lock_stale"));
    }

    #[test]
    fn run_lock_check_current_pid() {
        let dir = tempfile::tempdir().expect("tempdir");
        let lock_dir = dir.path().join(".roko").join("runtime");
        std::fs::create_dir_all(&lock_dir).expect("create dir");
        let pid = std::process::id();
        std::fs::write(lock_dir.join("roko.lock"), format!("{pid}\n")).expect("write lock");
        let mut selected = BTreeSet::new();
        selected.insert(DiagnosticCheckId::Lock);
        let request = DiagnosticRequest {
            workdir: dir.path().to_path_buf(),
            selected,
            profile: None,
            allow_repairs: false,
        };
        let report = DiagnosticService::run(&request);
        assert!(!report.has_errors());
        assert!(report.findings.iter().any(|f| f.code == "lock_self"));
    }

    #[test]
    fn run_disk_check_on_dev_machine() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut selected = BTreeSet::new();
        selected.insert(DiagnosticCheckId::Disk);
        let request = DiagnosticRequest {
            workdir: dir.path().to_path_buf(),
            selected,
            profile: None,
            allow_repairs: false,
        };
        let report = DiagnosticService::run(&request);
        // Dev machines should have > 2 GB free.
        assert!(report.findings.iter().any(|f| f.code == "disk_ok"));
    }

    #[test]
    fn run_toolchain_check_on_dev_machine() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut selected = BTreeSet::new();
        selected.insert(DiagnosticCheckId::Toolchain);
        let request = DiagnosticRequest {
            workdir: dir.path().to_path_buf(),
            selected,
            profile: None,
            allow_repairs: false,
        };
        let report = DiagnosticService::run(&request);
        assert!(report.findings.iter().any(|f| f.code == "toolchain_ok"));
    }

    #[test]
    fn run_git_check_in_non_repo() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut selected = BTreeSet::new();
        selected.insert(DiagnosticCheckId::Git);
        let request = DiagnosticRequest {
            workdir: dir.path().to_path_buf(),
            selected,
            profile: None,
            allow_repairs: false,
        };
        let report = DiagnosticService::run(&request);
        assert!(report.findings.iter().any(|f| f.code == "git_not_repo"));
    }

    #[test]
    fn findings_are_sorted_by_check_id_code_message() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut selected = BTreeSet::new();
        selected.insert(DiagnosticCheckId::Workspace);
        selected.insert(DiagnosticCheckId::Config);
        selected.insert(DiagnosticCheckId::Lock);
        let request = DiagnosticRequest {
            workdir: dir.path().to_path_buf(),
            selected,
            profile: None,
            allow_repairs: false,
        };
        let report = DiagnosticService::run(&request);
        // Verify sorted: each finding's check_id should be <= the next.
        for window in report.findings.windows(2) {
            assert!(
                window[0].check_id <= window[1].check_id,
                "findings not sorted: {:?} > {:?}",
                window[0].check_id,
                window[1].check_id
            );
        }
    }

    #[test]
    fn apply_repair_rejects_without_approval() {
        let result = DiagnosticService::apply_repair("config_missing", false);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            DiagnosticRepairError::NotApproved
        ));
    }

    #[test]
    fn apply_repair_rejects_unknown_code() {
        let result = DiagnosticService::apply_repair("nonexistent_code", true);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            DiagnosticRepairError::UnrecognizedCode(_)
        ));
    }

    #[test]
    fn run_multiple_checks_preflight_matrix() {
        // Simulate preflight check selection: config, credentials, disk, git, plans, toolchain, lock
        let dir = tempfile::tempdir().expect("tempdir");
        let selected: BTreeSet<DiagnosticCheckId> = [
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
            workdir: dir.path().to_path_buf(),
            selected,
            profile: None,
            allow_repairs: false,
        };
        let report = DiagnosticService::run(&request);
        // Should have at least 7 findings (one per check).
        assert!(report.findings.len() >= 7);
    }

    #[test]
    fn run_config_doctor_matrix() {
        // Simulate config doctor check selection: config, schema_version, providers, models
        let dir = tempfile::tempdir().expect("tempdir");
        let selected: BTreeSet<DiagnosticCheckId> = [
            DiagnosticCheckId::Config,
            DiagnosticCheckId::SchemaVersion,
            DiagnosticCheckId::Providers,
            DiagnosticCheckId::Models,
        ]
        .into_iter()
        .collect();

        let request = DiagnosticRequest {
            workdir: dir.path().to_path_buf(),
            selected,
            profile: None,
            allow_repairs: false,
        };
        let report = DiagnosticService::run(&request);
        assert!(report.findings.len() >= 4);
    }

    #[test]
    fn run_chat_doctor_matrix() {
        // Simulate chat /doctor check selection: config, workspace, git, credentials
        let dir = tempfile::tempdir().expect("tempdir");
        let selected: BTreeSet<DiagnosticCheckId> = [
            DiagnosticCheckId::Config,
            DiagnosticCheckId::Workspace,
            DiagnosticCheckId::Git,
            DiagnosticCheckId::Credentials,
        ]
        .into_iter()
        .collect();

        let request = DiagnosticRequest {
            workdir: dir.path().to_path_buf(),
            selected,
            profile: None,
            allow_repairs: false,
        };
        let report = DiagnosticService::run(&request);
        assert!(report.findings.len() >= 4);
    }

    #[test]
    fn report_severity_counts() {
        let dir = tempfile::tempdir().expect("tempdir");
        // Run all checks on a bare tempdir -- will produce a mix of severities.
        let selected: BTreeSet<DiagnosticCheckId> =
            DiagnosticCheckId::ALL.iter().copied().collect();
        let request = DiagnosticRequest {
            workdir: dir.path().to_path_buf(),
            selected,
            profile: None,
            allow_repairs: false,
        };
        let report = DiagnosticService::run(&request);
        let (info, warn, error) = report.severity_counts();
        assert_eq!(info + warn + error, report.findings.len());
    }

    #[test]
    fn schema_version_check_with_version_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let roko_dir = dir.path().join(".roko");
        std::fs::create_dir_all(&roko_dir).expect("create .roko");
        std::fs::write(roko_dir.join("VERSION"), "3\n").expect("write version");

        let mut selected = BTreeSet::new();
        selected.insert(DiagnosticCheckId::SchemaVersion);
        let request = DiagnosticRequest {
            workdir: dir.path().to_path_buf(),
            selected,
            profile: None,
            allow_repairs: false,
        };
        let report = DiagnosticService::run(&request);
        assert!(report
            .findings
            .iter()
            .any(|f| f.code == "schema_version_current"));
    }

    #[test]
    fn plans_check_with_tasks_toml() {
        let dir = tempfile::tempdir().expect("tempdir");
        let plans_dir = dir.path().join("plans").join("test-plan");
        std::fs::create_dir_all(&plans_dir).expect("create plans dir");
        std::fs::write(plans_dir.join("tasks.toml"), "[meta]\nplan = \"test\"\n")
            .expect("write tasks.toml");

        let mut selected = BTreeSet::new();
        selected.insert(DiagnosticCheckId::Plans);
        let request = DiagnosticRequest {
            workdir: dir.path().to_path_buf(),
            selected,
            profile: None,
            allow_repairs: false,
        };
        let report = DiagnosticService::run(&request);
        assert!(!report.has_errors());
        assert!(report.findings.iter().any(|f| f.code == "plans_ok"));
    }

    // ── Fixed adapter matrix tests (#279) ────────────────────────────────

    #[test]
    fn check_id_all_contains_exactly_eleven_ids() {
        assert_eq!(
            DiagnosticCheckId::ALL.len(),
            11,
            "spec requires exactly 11 shared check IDs"
        );
        let all_set: BTreeSet<DiagnosticCheckId> =
            DiagnosticCheckId::ALL.iter().copied().collect();
        assert_eq!(
            all_set.len(),
            11,
            "ALL slice must not contain duplicates"
        );
    }

    #[test]
    fn check_id_as_str_matches_spec_names() {
        // The spec mandates exactly these 11 snake_case names.
        let expected = [
            "config",
            "credentials",
            "disk",
            "git",
            "lock",
            "models",
            "plans",
            "providers",
            "schema_version",
            "toolchain",
            "workspace",
        ];
        let mut actual: Vec<&str> = DiagnosticCheckId::ALL
            .iter()
            .map(|id| id.as_str())
            .collect();
        actual.sort();
        assert_eq!(actual, expected);
    }

    #[test]
    fn preflight_matrix_selects_seven_checks() {
        // Fixed adapter matrix: plan preflight selects exactly these 7.
        let preflight_ids: BTreeSet<DiagnosticCheckId> = [
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
        assert_eq!(preflight_ids.len(), 7);

        let dir = tempfile::tempdir().expect("tempdir");
        let request = DiagnosticRequest {
            workdir: dir.path().to_path_buf(),
            selected: preflight_ids.clone(),
            profile: None,
            allow_repairs: false,
        };
        let report = DiagnosticService::run(&request);

        // Every finding must belong to one of the preflight IDs.
        for finding in &report.findings {
            assert!(
                preflight_ids.contains(&finding.check_id),
                "unexpected check_id {:?} in preflight report",
                finding.check_id
            );
        }
    }

    #[test]
    fn config_doctor_matrix_selects_four_checks() {
        // Fixed adapter matrix: config doctor selects exactly these 4.
        let config_ids: BTreeSet<DiagnosticCheckId> = [
            DiagnosticCheckId::Config,
            DiagnosticCheckId::SchemaVersion,
            DiagnosticCheckId::Providers,
            DiagnosticCheckId::Models,
        ]
        .into_iter()
        .collect();
        assert_eq!(config_ids.len(), 4);

        let dir = tempfile::tempdir().expect("tempdir");
        let request = DiagnosticRequest {
            workdir: dir.path().to_path_buf(),
            selected: config_ids.clone(),
            profile: None,
            allow_repairs: false,
        };
        let report = DiagnosticService::run(&request);

        for finding in &report.findings {
            assert!(
                config_ids.contains(&finding.check_id),
                "unexpected check_id {:?} in config doctor report",
                finding.check_id
            );
        }
    }

    #[test]
    fn chat_doctor_matrix_selects_four_checks() {
        // Fixed adapter matrix: chat /doctor selects exactly these 4.
        let chat_ids: BTreeSet<DiagnosticCheckId> = [
            DiagnosticCheckId::Config,
            DiagnosticCheckId::Workspace,
            DiagnosticCheckId::Git,
            DiagnosticCheckId::Credentials,
        ]
        .into_iter()
        .collect();
        assert_eq!(chat_ids.len(), 4);

        let dir = tempfile::tempdir().expect("tempdir");
        let request = DiagnosticRequest {
            workdir: dir.path().to_path_buf(),
            selected: chat_ids.clone(),
            profile: None,
            allow_repairs: false,
        };
        let report = DiagnosticService::run(&request);

        for finding in &report.findings {
            assert!(
                chat_ids.contains(&finding.check_id),
                "unexpected check_id {:?} in chat doctor report",
                finding.check_id
            );
        }
    }

    #[test]
    fn full_doctor_matrix_runs_all_eleven() {
        // Fixed adapter matrix: full doctor runs all 11 shared checks.
        let all_ids: BTreeSet<DiagnosticCheckId> =
            DiagnosticCheckId::ALL.iter().copied().collect();
        assert_eq!(all_ids.len(), 11);

        let dir = tempfile::tempdir().expect("tempdir");
        let request = DiagnosticRequest {
            workdir: dir.path().to_path_buf(),
            selected: all_ids,
            profile: None,
            allow_repairs: false,
        };
        let report = DiagnosticService::run(&request);

        // Every check ID should produce at least one finding.
        let found_ids: BTreeSet<DiagnosticCheckId> =
            report.findings.iter().map(|f| f.check_id).collect();
        for id in DiagnosticCheckId::ALL {
            assert!(
                found_ids.contains(id),
                "check {:?} produced no findings in full doctor run",
                id
            );
        }
    }

    #[test]
    fn run_is_read_only_when_repairs_disabled() {
        // Spec: DiagnosticService::run is read-only when allow_repairs=false.
        // Graph preflight always passes false. Verify no mutations occur.
        let dir = tempfile::tempdir().expect("tempdir");
        let roko_dir = dir.path().join(".roko").join("runtime");
        std::fs::create_dir_all(&roko_dir).expect("create dir");
        // Place a stale lock file.
        std::fs::write(roko_dir.join("roko.lock"), "999999999\n").expect("write lock");

        let request = DiagnosticRequest {
            workdir: dir.path().to_path_buf(),
            selected: DiagnosticCheckId::ALL.iter().copied().collect(),
            profile: None,
            allow_repairs: false,
        };
        let _report = DiagnosticService::run(&request);

        // The lock file must still exist (no mutation).
        assert!(
            roko_dir.join("roko.lock").exists(),
            "run() with allow_repairs=false must not remove the stale lock"
        );
    }

    #[test]
    fn remediation_has_stable_codes_across_calls() {
        // Acceptance: all surfaces report stable codes.
        let dir = tempfile::tempdir().expect("tempdir");
        let request = DiagnosticRequest {
            workdir: dir.path().to_path_buf(),
            selected: DiagnosticCheckId::ALL.iter().copied().collect(),
            profile: None,
            allow_repairs: false,
        };

        let report1 = DiagnosticService::run(&request);
        let report2 = DiagnosticService::run(&request);

        let codes1: Vec<&str> = report1.findings.iter().map(|f| f.code.as_str()).collect();
        let codes2: Vec<&str> = report2.findings.iter().map(|f| f.code.as_str()).collect();
        assert_eq!(
            codes1, codes2,
            "stable codes must be deterministic across runs"
        );
    }
}
