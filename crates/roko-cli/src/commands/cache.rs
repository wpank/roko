//! Workspace-local target, evidence, log, and context cache lifecycle.

use crate::{CacheCmd, Cli, EXIT_FAILURE, EXIT_SUCCESS, resolve_workdir};
use anyhow::{Context as _, Result};
use roko_fs::{CacheCleanupPolicy, CacheCleanupReport, cleanup_workspace_caches};
use std::path::PathBuf;
use std::time::Duration;

const MIB: u64 = 1024 * 1024;
const GIB: u64 = 1024 * MIB;

pub(crate) async fn cmd_cache(cli: &Cli, cmd: CacheCmd) -> Result<i32> {
    let (workdir, policy, apply) = match cmd {
        CacheCmd::Status { workdir } => (
            workdir.unwrap_or_else(|| resolve_workdir(cli)),
            CacheCleanupPolicy::default(),
            false,
        ),
        CacheCmd::Prune {
            apply,
            workdir,
            target_budget_gb,
            evidence_budget_mb,
            context_budget_mb,
            min_age_hours,
            max_evidence_age_days,
            keep_runs,
        } => {
            let policy = CacheCleanupPolicy {
                target_budget_bytes: target_budget_gb.saturating_mul(GIB),
                evidence_budget_bytes: evidence_budget_mb.saturating_mul(MIB),
                context_cache_budget_bytes: context_budget_mb.saturating_mul(MIB),
                min_incremental_age: Duration::from_secs(min_age_hours.saturating_mul(3600)),
                max_evidence_age: Duration::from_secs(
                    max_evidence_age_days.saturating_mul(86_400),
                ),
                preserve_evidence_runs: keep_runs,
                ..CacheCleanupPolicy::default()
            };
            (
                workdir.unwrap_or_else(|| resolve_workdir(cli)),
                policy,
                apply,
            )
        }
    };

    let report = cleanup_workspace_caches(&workdir, policy, apply)
        .await
        .with_context(|| format!("inspect caches under {}", workdir.display()))?;
    if cli.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_report(&workdir, &report);
    }
    Ok(if apply && report.failed_count > 0 {
        EXIT_FAILURE
    } else {
        EXIT_SUCCESS
    })
}

fn print_report(workdir: &PathBuf, report: &CacheCleanupReport) {
    println!("cache lifecycle: {}", workdir.display());
    println!("  target artifacts: {}", human_bytes(report.target_bytes));
    println!("  run evidence:     {}", human_bytes(report.evidence_bytes));
    println!("  context cache:    {}", human_bytes(report.context_cache_bytes));
    println!("  log archives:     {}", human_bytes(report.log_archive_bytes));
    println!("  eligible:         {}", human_bytes(report.eligible_bytes));
    if report.candidates.is_empty() {
        println!("  candidates:       none");
    } else {
        println!("  candidates:");
        for candidate in report.candidates.iter().take(50) {
            println!(
                "    {:>8}  risk={:?}  {}  {}",
                human_bytes(candidate.size_bytes),
                candidate.cold_build_risk,
                candidate.path.display(),
                candidate.reason
            );
        }
        if report.candidates.len() > 50 {
            println!("    ... {} more", report.candidates.len() - 50);
        }
    }
    if report.dry_run {
        println!("dry run: nothing deleted; use `roko cache prune --apply` to apply this plan");
    } else {
        println!(
            "reclaimed: {} across {} entries ({} skipped/failed)",
            human_bytes(report.reclaimed_bytes),
            report.removed_count,
            report.failed_count
        );
    }
    if !report.protected.is_empty() {
        println!("protected entries: {}", report.protected.len());
    }
}

fn human_bytes(bytes: u64) -> String {
    if bytes >= GIB {
        format!("{:.1} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.1} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= 1024 {
        format!("{:.1} KiB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}
