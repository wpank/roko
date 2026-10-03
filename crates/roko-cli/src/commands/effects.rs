//! `roko effects`: list the tool calls runs held for approval, and approve
//! or reject one (9132).
//!
//! A run whose outbound policy is `stage` holds each tool call that would
//! act on the outside world in `.roko/state/effect-holds/<run>/` instead of
//! running it (9131). `approve` replays the call once and runs the receipt
//! rungs of its task's pack; `reject` only records the decision. Both append
//! to `.roko/state/effects.jsonl` and remove the hold
//! (`roko_cli::effects_apply`).

use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use clap::Subcommand;
use roko_cli::effects_apply::{
    DecideError, EffectDecision, EffectOutcome, EffectRecord, McpEffectApplier, decide_effect,
    effects_report, list_holds, read_records,
};

use crate::{Cli, EXIT_FAILURE, EXIT_SUCCESS, resolve_workdir};

/// Staged outbound effects.
#[derive(Debug, Subcommand)]
pub(crate) enum EffectsCmd {
    /// List the effects that wait for a decision, and the recent decisions.
    List {
        /// Workspace root (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Show one effect: its hold while it waits, else its decision. The
    /// call's arguments are shown only for a waiting effect.
    Show {
        /// The effect's id, as `roko effects list` shows it.
        effect_id: String,
        /// Workspace root (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Approve an effect: replay its call once and check its receipt.
    Approve {
        /// The effect's id, as `roko effects list` shows it.
        effect_id: String,
        /// Why, recorded with the decision.
        #[arg(long)]
        note: Option<String>,
        /// Who approves (default: $USER, else "operator").
        #[arg(long)]
        by: Option<String>,
        /// Workspace root (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Reject an effect: its call never runs.
    Reject {
        /// The effect's id, as `roko effects list` shows it.
        effect_id: String,
        /// Why, recorded with the decision.
        #[arg(long)]
        note: Option<String>,
        /// Who rejects (default: $USER, else "operator").
        #[arg(long)]
        by: Option<String>,
        /// Workspace root (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
}

pub(crate) async fn cmd_effects(cli: &Cli, cmd: EffectsCmd) -> Result<i32> {
    match cmd {
        EffectsCmd::List { workdir } => {
            let workdir = workdir.unwrap_or_else(|| resolve_workdir(cli));
            list(cli, &workdir)
        }
        EffectsCmd::Show { effect_id, workdir } => {
            let workdir = workdir.unwrap_or_else(|| resolve_workdir(cli));
            show(cli, &workdir, &effect_id)
        }
        EffectsCmd::Approve {
            effect_id,
            note,
            by,
            workdir,
        } => decide(cli, workdir, &effect_id, true, note, by).await,
        EffectsCmd::Reject {
            effect_id,
            note,
            by,
            workdir,
        } => decide(cli, workdir, &effect_id, false, note, by).await,
    }
}

/// Print the waiting effects and the recent decisions. Arguments are never
/// printed here: they may carry secrets.
fn list(cli: &Cli, workdir: &Path) -> Result<i32> {
    if cli.json {
        let report = effects_report(workdir, None);
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(EXIT_SUCCESS);
    }
    let holds = list_holds(workdir);
    let records = read_records(workdir);
    if holds.is_empty() {
        println!("no effects wait for a decision");
    }
    for (_, hold) in &holds {
        println!(
            "{}  waiting  {}  run={}  task={}/{}  proposed={}",
            hold.effect_id, hold.tool, hold.run_id, hold.plan_id, hold.task_id, hold.proposed_at
        );
    }
    for record in records.iter().rev().take(20) {
        println!("{}", record_line(record));
    }
    Ok(EXIT_SUCCESS)
}

/// Print one effect: its hold, arguments included, while it waits, else its
/// decision.
fn show(cli: &Cli, workdir: &Path, effect_id: &str) -> Result<i32> {
    let waiting = list_holds(workdir)
        .into_iter()
        .find(|(_, hold)| hold.effect_id == effect_id);
    if let Some((path, hold)) = waiting {
        if cli.json {
            println!("{}", serde_json::to_string_pretty(&hold)?);
        } else {
            println!(
                "{}  waiting for a decision ({})",
                hold.effect_id,
                path.display()
            );
            println!("tool       {}", hold.tool);
            println!(
                "run        {} ({}/{})",
                hold.run_id, hold.plan_id, hold.task_id
            );
            println!("proposed   {}", hold.proposed_at);
            println!(
                "arguments  {}",
                serde_json::to_string_pretty(&hold.arguments)?
            );
        }
        return Ok(EXIT_SUCCESS);
    }
    let decided = read_records(workdir)
        .into_iter()
        .find(|record| record.effect_id == effect_id);
    let Some(record) = decided else {
        eprintln!("no effect {effect_id}");
        return Ok(EXIT_FAILURE);
    };
    if cli.json {
        println!("{}", serde_json::to_string_pretty(&record)?);
    } else {
        println!("{}", record_line(&record));
        if let Some(result) = &record.result {
            println!("result     {result}");
        }
        for receipt in &record.receipts {
            let verdict = if receipt.passed { "passed" } else { "failed" };
            println!(
                "receipt    {} {verdict}: {}",
                receipt.rung,
                receipt.output.trim()
            );
        }
    }
    Ok(EXIT_SUCCESS)
}

/// Approve or reject `effect_id` and print the record.
async fn decide(
    cli: &Cli,
    workdir: Option<PathBuf>,
    effect_id: &str,
    approve: bool,
    note: Option<String>,
    by: Option<String>,
) -> Result<i32> {
    let workdir = workdir.unwrap_or_else(|| resolve_workdir(cli));
    let config = roko_core::config::loader::load_config_unified(&workdir)
        .map_err(|error| anyhow::anyhow!("{error}"))
        .with_context(|| format!("load the config of {}", workdir.display()))?;
    let decision = EffectDecision {
        approve,
        note,
        decided_by: by.unwrap_or_else(default_principal),
    };
    let applier = McpEffectApplier::new(workdir.clone(), config.clone());
    match decide_effect(&workdir, &config, effect_id, decision, &applier).await {
        Ok(record) => {
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&record)?);
            } else {
                println!("{}", record_line(&record));
            }
            // A call that failed or may not have run, or a receipt that
            // does not show the effect, fails the command.
            let settled = matches!(
                record.outcome,
                EffectOutcome::Applied | EffectOutcome::Rejected
            );
            let shown = record.receipts.iter().all(|receipt| receipt.passed);
            Ok(if settled && shown {
                EXIT_SUCCESS
            } else {
                EXIT_FAILURE
            })
        }
        Err(error @ (DecideError::NotFound(_) | DecideError::AlreadyDecided(..))) => {
            eprintln!("{error}");
            Ok(EXIT_FAILURE)
        }
        Err(error) => Err(error).with_context(|| format!("decide effect {effect_id}")),
    }
}

/// One line for a decided effect.
fn record_line(record: &EffectRecord) -> String {
    let receipts = if record.receipts.is_empty() {
        String::new()
    } else {
        let passed = record.receipts.iter().filter(|r| r.passed).count();
        format!("  receipts={passed}/{}", record.receipts.len())
    };
    format!(
        "{}  {}  {}  by={}  at={}{receipts}",
        record.effect_id,
        record.outcome.label(),
        record.tool,
        record.decided_by,
        record.decided_at
    )
}

/// The deciding principal when `--by` is absent: `$USER`, else `operator`.
fn default_principal() -> String {
    std::env::var("USER")
        .ok()
        .filter(|user| !user.trim().is_empty())
        .unwrap_or_else(|| "operator".to_string())
}
