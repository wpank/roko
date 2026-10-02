//! `roko safety`: inspect and release immune isolation controls.
//!
//! The provider immune boundary writes an isolation control to
//! `<workdir>/.roko/immune/agent-controls.json` when it contains a provider
//! result, and denies that agent id before every later dispatch. These
//! commands show the controls in force and lift one, with an audit line in
//! `.roko/immune/agent-control-releases.jsonl`.

use std::path::PathBuf;

use anyhow::{Context as _, Result};
use clap::Subcommand;

use crate::{Cli, EXIT_FAILURE, EXIT_SUCCESS, resolve_workdir};

/// Immune isolation controls.
#[derive(Debug, Subcommand)]
pub(crate) enum SafetyCmd {
    /// List the immune isolation controls that deny agents before dispatch.
    Controls {
        /// Workspace root (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Release one agent's isolation control and record who did it and why.
    Release {
        /// Agent id the control denies, as `roko safety controls` lists it.
        agent_id: String,
        /// Why the control is released (recorded in the audit file).
        #[arg(long)]
        reason: String,
        /// Who releases it (default: $USER, else "operator").
        #[arg(long)]
        by: Option<String>,
        /// Workspace root (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
}

pub(crate) async fn cmd_safety(cli: &Cli, cmd: SafetyCmd) -> Result<i32> {
    match cmd {
        SafetyCmd::Controls { workdir } => {
            let workdir = workdir.unwrap_or_else(|| resolve_workdir(cli));
            let controls = roko_agent::list_agent_controls(&workdir).with_context(|| {
                format!("read isolation controls under {}", workdir.display())
            })?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&controls)?);
            } else if controls.is_empty() {
                println!("no isolation controls");
            } else {
                for control in &controls {
                    println!(
                        "{}  state={}  reason={}  expires={}  control={}",
                        control.agent_id,
                        control.state,
                        control.reason,
                        expiry_label(control.expires_at_ms),
                        control.control_id
                    );
                }
            }
            Ok(EXIT_SUCCESS)
        }
        SafetyCmd::Release {
            agent_id,
            reason,
            by,
            workdir,
        } => {
            let workdir = workdir.unwrap_or_else(|| resolve_workdir(cli));
            let by = by.unwrap_or_else(default_principal);
            let released = roko_agent::release_agent_control(&workdir, &agent_id, &by, &reason)
                .with_context(|| format!("release the isolation control of {agent_id}"))?;
            let audit_path = roko_agent::agent_control_releases_path(&workdir);
            if cli.json {
                let report = serde_json::json!({
                    "released": released,
                    "audit_path": audit_path,
                });
                println!("{}", serde_json::to_string_pretty(&report)?);
            }
            let Some(released) = released else {
                if !cli.json {
                    eprintln!("no isolation control for agent {agent_id}");
                }
                return Ok(EXIT_FAILURE);
            };
            if !cli.json {
                println!(
                    "released isolation control {} of {} (by {})",
                    released.control_id, released.agent_id, released.by
                );
                println!("audit record: {}", audit_path.display());
            }
            Ok(EXIT_SUCCESS)
        }
    }
}

/// When a control stops denying its agent: an RFC 3339 time, or `never` for
/// a control written before controls expired, which stays until released.
fn expiry_label(expires_at_ms: Option<u64>) -> String {
    let Some(expires_at_ms) = expires_at_ms else {
        return "never".to_string();
    };
    i64::try_from(expires_at_ms)
        .ok()
        .and_then(chrono::DateTime::<chrono::Utc>::from_timestamp_millis)
        .map_or_else(|| format!("{expires_at_ms}ms"), |at| at.to_rfc3339())
}

/// The releasing principal when `--by` is absent: `$USER`, else `operator`.
fn default_principal() -> String {
    std::env::var("USER")
        .ok()
        .filter(|user| !user.trim().is_empty())
        .unwrap_or_else(|| "operator".to_string())
}
