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
pub(crate) async fn cmd_tune(cli: &Cli, cmd: TuneCmd) -> Result<i32> {
    let workdir = tune_workdir(cli, &cmd);
    ensure_project_config(&workdir)?;

    let (label, edits): (&str, Vec<(&str, String)>) = match cmd {
        TuneCmd::Routing { .. } => (
            "routing",
            vec![
                ("routing.mode", "auto_override".to_string()),
                ("routing.algorithm", "linucb".to_string()),
                ("routing.fast_task_model", "claude-haiku-4-5".to_string()),
                (
                    "routing.standard_task_model",
                    "claude-sonnet-4-6".to_string(),
                ),
                ("routing.complex_task_model", "claude-opus-4-6".to_string()),
                ("routing.context_strategy", "hybrid".to_string()),
                ("routing.weights.quality", "0.55".to_string()),
                ("routing.weights.cost", "0.30".to_string()),
                ("routing.weights.latency", "0.15".to_string()),
            ],
        ),
        TuneCmd::Gates { .. } => (
            "gates",
            vec![
                ("gates.clippy_enabled", "true".to_string()),
                ("gates.skip_tests", "false".to_string()),
                ("gates.max_iterations", "2".to_string()),
            ],
        ),
        TuneCmd::Budget { .. } => (
            "budget",
            vec![
                ("budget.max_plan_usd", "10.0".to_string()),
                ("budget.max_turn_usd", "1.0".to_string()),
                ("budget.prompt_token_budget", "20000".to_string()),
            ],
        ),
        TuneCmd::Model { name, .. } => {
            let model = resolve_model_key(&workdir, &name)?;
            ("model", vec![("agent.default_model", model)])
        }
    };

    let pending = edits
        .iter()
        .map(|(key, value)| ((*key).to_string(), value.clone()))
        .collect::<HashMap<_, _>>();
    roko_cli::tui::config_meta::save_pending_edits(&workdir, &pending)
        .map_err(anyhow::Error::msg)?;

    let path = workdir.join("roko.toml");
    println!("tuned {label} in {}", path.display());
    for (key, value) in edits {
        println!("  {key} = {value}");
    }

    Ok(EXIT_SUCCESS)
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
            println!("dry-run: {label} preset for {target_label} config at {}", write_path.display());
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
    let config_dir = write_path
        .parent()
        .unwrap_or(&workdir);
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
            if !available.iter().any(|s| s == slug)
                && !config.models.contains_key(slug.as_str())
            {
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
            value: format!("{:.2}", routing.weights.quality),
        },
        PresetDiffEntry {
            key: "routing.weights.cost".into(),
            value: format!("{:.2}", routing.weights.cost),
        },
        PresetDiffEntry {
            key: "routing.weights.latency".into(),
            value: format!("{:.2}", routing.weights.latency),
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

fn tune_workdir(cli: &Cli, cmd: &TuneCmd) -> PathBuf {
    match cmd {
        TuneCmd::Routing { workdir }
        | TuneCmd::Gates { workdir }
        | TuneCmd::Budget { workdir }
        | TuneCmd::Model { workdir, .. } => workdir.clone(),
    }
    .unwrap_or_else(|| resolve_workdir(cli))
}

fn ensure_project_config(workdir: &Path) -> Result<()> {
    let path = workdir.join("roko.toml");
    if path.exists() {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let template = Config::default_toml_template(false)?;
    std::fs::write(&path, template).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

fn resolve_model_key(workdir: &Path, requested: &str) -> Result<String> {
    let requested = requested.trim();
    if requested.is_empty() {
        bail!("provide a model name");
    }

    let config = roko_core::config::loader::load_config_unified(workdir)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let requested_lower = requested.to_ascii_lowercase();
    let normalized = roko_cli::task_parser::normalize_model_alias(requested);
    let normalized_lower = normalized.to_ascii_lowercase();

    if config.models.contains_key(requested) {
        return Ok(requested.to_string());
    }
    if config.models.contains_key(normalized) {
        return Ok(normalized.to_string());
    }

    let mut matches = config
        .models
        .iter()
        .filter_map(|(key, profile)| {
            let key_lower = key.to_ascii_lowercase();
            let slug_lower = profile.slug.to_ascii_lowercase();
            (key_lower == requested_lower
                || key_lower == normalized_lower
                || slug_lower == requested_lower
                || slug_lower == normalized_lower)
                .then(|| key.clone())
        })
        .collect::<Vec<_>>();
    matches.sort();
    matches.dedup();
    if let Some(model_key) = matches.into_iter().next() {
        return Ok(model_key);
    }

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
}
