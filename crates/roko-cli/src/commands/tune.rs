//! tune command handler.
//!
//! This module handles both the legacy top-level `roko tune` command (deprecated,
//! now forwarding to `roko config preset`) and the new `roko config preset`
//! command that writes validated, resolved configuration presets.

use crate::*;
use std::collections::HashMap;
use std::io::IsTerminal;

// ── Legacy top-level `roko tune` (deprecated) ───────────────────────

/// `roko tune ...` — legacy preset writer, preserved for backward compatibility.
///
/// This command is deprecated. Users should migrate to `roko config preset`.
/// The deprecation warning is printed in main.rs dispatch.
///
/// Forwards to [`cmd_config_preset`] so that all writes go through the same
/// validated, resolved, consent-aware path.
pub(crate) async fn cmd_tune(cli: &Cli, cmd: TuneCmd) -> Result<i32> {
    let preset_cmd = tune_to_preset(cmd);
    cmd_config_preset(cli, preset_cmd).await
}

/// Convert a legacy `TuneCmd` to the canonical `ConfigPresetCmd`.
///
/// Uses `--yes` to preserve the legacy non-interactive behavior.
fn tune_to_preset(cmd: TuneCmd) -> ConfigPresetCmd {
    match cmd {
        TuneCmd::Routing { workdir } => ConfigPresetCmd::Routing {
            workdir,
            dry_run: false,
            yes: true,
            global: false,
            project: false,
        },
        TuneCmd::Gates { workdir } => ConfigPresetCmd::Gates {
            workdir,
            dry_run: false,
            yes: true,
            global: false,
            project: false,
        },
        TuneCmd::Budget { workdir } => ConfigPresetCmd::Budget {
            workdir,
            dry_run: false,
            yes: true,
            global: false,
            project: false,
        },
        TuneCmd::Model { name, workdir } => ConfigPresetCmd::Model {
            name,
            workdir,
            dry_run: false,
            yes: true,
            global: false,
            project: false,
        },
    }
}

// ── New `roko config preset` command ─────────────────────────────────

/// Typed diff entry for preset application.
#[derive(Clone, serde::Serialize)]
struct PresetDiffEntry {
    key: String,
    value: String,
}

/// JSON output for `config preset --json`.
#[derive(serde::Serialize)]
struct PresetDiffJson {
    preset: String,
    target: String,
    dry_run: bool,
    edits: Vec<PresetDiffEntry>,
}

/// `roko config preset <subsystem>` — apply validated, resolved config presets.
///
/// Supports `--dry-run` to preview without writing, `--json` for structured
/// output, `--yes` to skip confirmation, and `--project`/`--global` to select
/// the config layer.
pub(crate) async fn cmd_config_preset(cli: &Cli, cmd: ConfigPresetCmd) -> Result<i32> {
    let (label, workdir, dry_run, yes, global) = preset_common_args(cli, &cmd);
    let json = cli.json;

    let target_label = if global { "global" } else { "project" };

    // Resolve the config write target.
    let write_path = if global {
        roko_core::config::loader::global_config_path()
            .ok_or_else(|| anyhow::anyhow!("cannot determine global config path (HOME unset)"))?
    } else {
        workdir.join("roko.toml")
    };

    // Build preset edits.
    let edits = match &cmd {
        ConfigPresetCmd::Gates { .. } => build_gates_preset(),
        ConfigPresetCmd::Routing { .. } => build_routing_preset(&workdir)?,
        ConfigPresetCmd::Budget { .. } => build_budget_preset(),
        ConfigPresetCmd::Model { name, .. } => {
            let resolved = resolve_model_key(&workdir, name)?;
            vec![PresetDiffEntry {
                key: "agent.default_model".into(),
                value: resolved,
            }]
        }
    };

    // Dry-run: show diff and exit.
    if dry_run {
        if json {
            let output = PresetDiffJson {
                preset: label.to_string(),
                target: target_label.to_string(),
                dry_run: true,
                edits: edits.clone(),
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            println!(
                "dry-run: {label} preset for {target_label} config at {}",
                write_path.display()
            );
            for entry in &edits {
                println!("  {} = {}", entry.key, entry.value);
            }
            println!("(no changes written)");
        }
        return Ok(EXIT_SUCCESS);
    }

    // Require consent in non-interactive mode unless --yes is set.
    if !yes && !std::io::stdin().is_terminal() {
        bail!(
            "non-interactive mode requires --yes to confirm preset writes. \
             Use --dry-run to preview changes."
        );
    }

    // Interactive confirmation unless --yes.
    if !yes {
        println!(
            "Apply {label} preset to {target_label} config at {}?",
            write_path.display()
        );
        for entry in &edits {
            println!("  {} = {}", entry.key, entry.value);
        }
        eprint!("Proceed? [y/N] ");
        let mut answer = String::new();
        std::io::stdin().read_line(&mut answer)?;
        if !answer.trim().eq_ignore_ascii_case("y") {
            println!("aborted");
            return Ok(EXIT_SUCCESS);
        }
    }

    // Ensure the target config file exists.
    if global {
        if let Some(parent) = write_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if !write_path.exists() {
            std::fs::write(&write_path, "")?;
        }
    } else {
        ensure_project_config(&workdir)?;
    }

    // Apply the edits atomically via the existing config editor.
    let pending = edits
        .iter()
        .map(|entry| (entry.key.clone(), entry.value.clone()))
        .collect::<HashMap<_, _>>();

    // save_pending_edits works on the directory containing roko.toml.
    let config_dir = write_path.parent().unwrap_or(&workdir);
    roko_cli::tui::config_meta::save_pending_edits(config_dir, &pending)
        .map_err(anyhow::Error::msg)?;

    if json {
        let output = PresetDiffJson {
            preset: label.to_string(),
            target: target_label.to_string(),
            dry_run: false,
            edits,
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!(
            "applied {label} preset to {target_label} config at {}",
            write_path.display()
        );
        for entry in &pending {
            println!("  {} = {}", entry.0, entry.1);
        }
    }

    Ok(EXIT_SUCCESS)
}

/// Extract common arguments from any `ConfigPresetCmd` variant.
fn preset_common_args<'a>(
    cli: &Cli,
    cmd: &'a ConfigPresetCmd,
) -> (&'static str, PathBuf, bool, bool, bool) {
    match cmd {
        ConfigPresetCmd::Gates {
            workdir,
            dry_run,
            yes,
            global,
            ..
        } => (
            "gates",
            workdir.clone().unwrap_or_else(|| resolve_workdir(cli)),
            *dry_run,
            *yes,
            *global,
        ),
        ConfigPresetCmd::Routing {
            workdir,
            dry_run,
            yes,
            global,
            ..
        } => (
            "routing",
            workdir.clone().unwrap_or_else(|| resolve_workdir(cli)),
            *dry_run,
            *yes,
            *global,
        ),
        ConfigPresetCmd::Budget {
            workdir,
            dry_run,
            yes,
            global,
            ..
        } => (
            "budget",
            workdir.clone().unwrap_or_else(|| resolve_workdir(cli)),
            *dry_run,
            *yes,
            *global,
        ),
        ConfigPresetCmd::Model {
            workdir,
            dry_run,
            yes,
            global,
            ..
        } => (
            "model",
            workdir.clone().unwrap_or_else(|| resolve_workdir(cli)),
            *dry_run,
            *yes,
            *global,
        ),
    }
}

/// Build gates preset edits.
fn build_gates_preset() -> Vec<PresetDiffEntry> {
    vec![
        PresetDiffEntry {
            key: "gates.clippy_enabled".into(),
            value: "true".into(),
        },
        PresetDiffEntry {
            key: "gates.skip_tests".into(),
            value: "false".into(),
        },
        PresetDiffEntry {
            key: "gates.max_iterations".into(),
            value: "2".into(),
        },
    ]
}

/// Build routing preset edits, resolving model slugs from configuration
/// rather than using hardcoded values. Each tier model must be resolvable
/// from the configured `[models.*]` table.
fn build_routing_preset(workdir: &Path) -> Result<Vec<PresetDiffEntry>> {
    let config = roko_core::config::loader::load_config_unified(workdir)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    // Use the current routing config values (which already come from the
    // user's config or defaults) as the preset source. This resolves from
    // configured models rather than hardcoding slugs.
    let routing = &config.routing;

    // Validate that the tier models are known if models are configured.
    if !config.models.is_empty() {
        let available = config.model_slugs_for_cascade();
        for (tier, slug) in [
            ("fast", &routing.fast_task_model),
            ("standard", &routing.standard_task_model),
            ("complex", &routing.complex_task_model),
        ] {
            if !available.iter().any(|s| s == slug) && !config.models.contains_key(slug.as_str()) {
                bail!(
                    "routing preset references {tier}_task_model = \"{slug}\" \
                     which is not in configured models. \
                     Available: {}. \
                     Configure the model first or adjust routing.{tier}_task_model.",
                    available.join(", ")
                );
            }
        }
    }

    Ok(vec![
        PresetDiffEntry {
            key: "routing.mode".into(),
            value: routing.mode.clone(),
        },
        PresetDiffEntry {
            key: "routing.fast_task_model".into(),
            value: routing.fast_task_model.clone(),
        },
        PresetDiffEntry {
            key: "routing.standard_task_model".into(),
            value: routing.standard_task_model.clone(),
        },
        PresetDiffEntry {
            key: "routing.complex_task_model".into(),
            value: routing.complex_task_model.clone(),
        },
        PresetDiffEntry {
            key: "routing.context_strategy".into(),
            value: routing.context_strategy.clone(),
        },
        PresetDiffEntry {
            key: "routing.weights.quality".into(),
            value: format!("{:.2}", routing.weights.default.quality),
        },
        PresetDiffEntry {
            key: "routing.weights.cost".into(),
            value: format!("{:.2}", routing.weights.default.cost),
        },
        PresetDiffEntry {
            key: "routing.weights.latency".into(),
            value: format!("{:.2}", routing.weights.default.latency),
        },
    ])
}

/// Build budget preset edits.
fn build_budget_preset() -> Vec<PresetDiffEntry> {
    vec![
        PresetDiffEntry {
            key: "budget.max_plan_usd".into(),
            value: "10.0".into(),
        },
        PresetDiffEntry {
            key: "budget.max_turn_usd".into(),
            value: "1.0".into(),
        },
        PresetDiffEntry {
            key: "budget.prompt_token_budget".into(),
            value: "20000".into(),
        },
    ]
}

// ── Shared helpers ──────────────────────────────────────────────────

fn ensure_project_config(workdir: &Path) -> Result<()> {
    let path = workdir.join("roko.toml");
    if path.exists() {
        return Ok(());
    }

    // The `roko init` template, checked like `roko config validate` before
    // it is written.
    let template = Config::default_toml_template(false)?;
    roko_cli::config_cmd::write_checked_config(&path, &template)?;
    Ok(())
}

/// Resolve a user-supplied model name to its canonical config key.
///
/// Delegates to the canonical `roko_core::agent::resolve_model` which handles
/// key lookup, slug matching, prefix matching, and builtin registry fallback.
/// This wrapper adds alias normalization and a user-friendly error when the
/// model is not found in a non-empty config.
fn resolve_model_key(workdir: &Path, requested: &str) -> Result<String> {
    let requested = requested.trim();
    if requested.is_empty() {
        bail!("provide a model name");
    }

    let config = roko_core::config::loader::load_config_unified(workdir)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    // Try the raw input first via the canonical resolver.
    let resolved = roko_core::agent::resolve_model(&config, requested);
    if resolved.profile.is_some() {
        return Ok(resolved.model_key);
    }

    // Try the normalized alias (e.g. "sonnet" -> "claude-sonnet-4").
    let normalized = roko_cli::task_parser::normalize_model_alias(requested);
    if normalized != requested {
        let resolved = roko_core::agent::resolve_model(&config, normalized);
        if resolved.profile.is_some() {
            return Ok(resolved.model_key);
        }
    }

    // No configured models at all: accept the normalized value as-is so that
    // the caller can write it to config even without a models table.
    if config.models.is_empty() {
        return Ok(normalized.to_string());
    }

    let mut keys = config.models.keys().cloned().collect::<Vec<_>>();
    keys.sort();
    bail!(
        "unknown model '{requested}'. configured models: {}",
        keys.join(", ")
    )
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_preset_gates_produces_expected_edits() {
        let edits = build_gates_preset();
        assert_eq!(edits.len(), 3);
        assert_eq!(edits[0].key, "gates.clippy_enabled");
        assert_eq!(edits[0].value, "true");
        assert_eq!(edits[1].key, "gates.skip_tests");
        assert_eq!(edits[1].value, "false");
        assert_eq!(edits[2].key, "gates.max_iterations");
        assert_eq!(edits[2].value, "2");
    }

    #[test]
    fn config_preset_budget_produces_expected_edits() {
        let edits = build_budget_preset();
        assert_eq!(edits.len(), 3);
        assert_eq!(edits[0].key, "budget.max_plan_usd");
        assert_eq!(edits[0].value, "10.0");
        assert_eq!(edits[1].key, "budget.max_turn_usd");
        assert_eq!(edits[1].value, "1.0");
        assert_eq!(edits[2].key, "budget.prompt_token_budget");
        assert_eq!(edits[2].value, "20000");
    }

    #[test]
    fn preset_diff_json_serializes_cleanly() {
        let output = PresetDiffJson {
            preset: "gates".into(),
            target: "project".into(),
            dry_run: true,
            edits: build_gates_preset(),
        };
        let json = serde_json::to_string_pretty(&output).unwrap();
        assert!(json.contains("\"preset\": \"gates\""));
        assert!(json.contains("\"dry_run\": true"));
        assert!(json.contains("gates.clippy_enabled"));
    }

    // ── tune_compat: legacy TuneCmd → ConfigPresetCmd forwarding ──

    #[test]
    fn tune_compat_routing_forwards_to_preset() {
        let cmd = TuneCmd::Routing { workdir: None };
        let preset = tune_to_preset(cmd);
        assert!(
            matches!(
                preset,
                ConfigPresetCmd::Routing {
                    yes: true,
                    dry_run: false,
                    global: false,
                    ..
                }
            ),
            "legacy tune routing must forward to config preset routing with --yes"
        );
    }

    #[test]
    fn tune_compat_gates_forwards_to_preset() {
        let cmd = TuneCmd::Gates { workdir: None };
        let preset = tune_to_preset(cmd);
        assert!(
            matches!(
                preset,
                ConfigPresetCmd::Gates {
                    yes: true,
                    dry_run: false,
                    global: false,
                    ..
                }
            ),
            "legacy tune gates must forward to config preset gates with --yes"
        );
    }

    #[test]
    fn tune_compat_budget_forwards_to_preset() {
        let cmd = TuneCmd::Budget {
            workdir: Some(PathBuf::from("/tmp/proj")),
        };
        let preset = tune_to_preset(cmd);
        assert!(
            matches!(
                preset,
                ConfigPresetCmd::Budget { workdir: Some(ref wd), yes: true, global: false, .. }
                if wd == std::path::Path::new("/tmp/proj")
            ),
            "legacy tune budget must forward workdir and set --yes"
        );
    }

    #[test]
    fn tune_compat_model_forwards_name_and_workdir() {
        let cmd = TuneCmd::Model {
            name: "sonnet".into(),
            workdir: None,
        };
        let preset = tune_to_preset(cmd);
        assert!(
            matches!(
                preset,
                ConfigPresetCmd::Model { ref name, yes: true, dry_run: false, global: false, .. }
                if name == "sonnet"
            ),
            "legacy tune model must forward name and set --yes"
        );
    }

    #[test]
    fn tune_compat_no_hardcoded_model_slugs() {
        // Verify no hardcoded model slugs appear anywhere in the legacy path.
        // The legacy cmd_tune now forwards to cmd_config_preset which uses
        // build_routing_preset (resolves from config) instead of hardcoded values.
        let source = include_str!("tune.rs");
        // The only model references should be in build_routing_preset which
        // reads from config, not in cmd_tune.
        let cmd_tune_section = source
            .split("pub(crate) async fn cmd_tune")
            .nth(1)
            .and_then(|s| s.split("pub(crate) async fn cmd_config_preset").next())
            .unwrap_or("");
        assert!(
            !cmd_tune_section.contains("claude-haiku"),
            "cmd_tune must not contain hardcoded claude-haiku slug"
        );
        assert!(
            !cmd_tune_section.contains("claude-sonnet"),
            "cmd_tune must not contain hardcoded claude-sonnet slug"
        );
        assert!(
            !cmd_tune_section.contains("claude-opus"),
            "cmd_tune must not contain hardcoded claude-opus slug"
        );
    }
}
