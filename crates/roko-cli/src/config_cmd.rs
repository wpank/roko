//! `roko config` subcommand group — setup wizard, provenance viewer, editor.
//!
//! These commands operate on the global (`~/.roko/config.toml`) and/or
//! project (`./roko.toml`) config files. The `init` wizard is the primary
//! onboarding path: it detects installed LLM CLIs and writes a working
//! global config with one interactive pass.

use crate::config::{
    ConfigPaths, DetectedCli, ResolvedConfig, Source, detect_clis, global_config_path,
    load_resolved_config, read_toml_file, resolve_paths, set_toml_dotted_key, write_toml_file,
};
use anyhow::{Context as _, Result, anyhow};
use roko_core::agent::ProviderKind;
use roko_core::config::GateRungConfig;
use roko_core::config::hot_reload::{self, ConfigChange, ConfigSection};
use roko_core::config::schema::{
    CURRENT_CONFIG_VERSION, CURRENT_SCHEMA_VERSION, ModelProfile, ProviderConfig, RokoConfig,
};
use roko_core::tool::{ToolFormat, profile_for_model};
use std::collections::BTreeSet;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const VALIDATION_REACHABILITY_TIMEOUT_SECS: u64 = 2;

/// Non-interactive inputs for `config init` (used by CI / tests).
#[derive(Clone, Debug, Default)]
pub struct WizardInputs {
    /// Pre-selected agent command (skip CLI picker).
    pub agent_command: Option<String>,
    /// Extra args for the agent (replaces the detected defaults).
    pub agent_args: Option<Vec<String>>,
    /// Preferred model slug, if the backend supports one.
    pub model: Option<String>,
    /// Token budget for prompt composition (`budget.prompt_token_budget`).
    pub token_budget: Option<usize>,
    /// Ignored: no config key sets the agent persona since `[prompt] role`
    /// was removed. The wizard says so when one is given.
    pub role: Option<String>,
    /// Whether to enable cargo compile+clippy gates by default.
    pub enable_gates: Option<bool>,
    /// Skip the final confirmation and write without asking.
    pub yes: bool,
}

/// Run the config-init wizard interactively and write a global config file.
///
/// `target` overrides the global path (for tests). `inputs` pre-fills
/// answers; any field left `None` triggers an interactive prompt.
pub fn run_init_wizard(target: Option<PathBuf>, inputs: &WizardInputs) -> Result<PathBuf> {
    let path = target
        .or_else(global_config_path)
        .ok_or_else(|| anyhow!("cannot determine global config path: HOME is not set"))?;
    println!("\nRoko config wizard");
    println!("==================");
    println!("Writing global config to: {}\n", path.display());

    // 1. Agent backend.
    let detected = detect_clis();
    let agent_command = resolve_agent_command(inputs.agent_command.clone(), &detected)?;
    let suggested_args = detected
        .iter()
        .find(|d| d.command == agent_command)
        .map(|d| d.default_args.clone())
        .unwrap_or_default();
    let agent_args = resolve_agent_args(inputs.agent_args.clone(), &agent_command, suggested_args)?;

    // 2. Token budget.
    let token_budget = match inputs.token_budget {
        Some(b) => b,
        None => prompt_usize("Token budget for prompt composition", 8000)?,
    };

    // 3. Default gates.
    let enable_gates = match inputs.enable_gates {
        Some(v) => v,
        None => prompt_bool("Enable default cargo gates (compile + clippy)?", false)?,
    };
    if inputs.role.is_some() {
        println!("note: the role text is not saved: no config key sets the agent persona");
    }

    // Build the wizard config as a raw TOML document so only explicitly set
    // keys appear in the output file (no default inflation). Every key is in
    // the current schema, so `roko config validate` accepts the file.
    let mut doc = toml::Value::Table(toml::map::Map::new());
    set_toml_dotted_key(&mut doc, "agent.command", &agent_command)?;
    if !agent_args.is_empty() {
        let args_json = serde_json::to_string(&agent_args).context("serialize agent args")?;
        set_toml_dotted_key(&mut doc, "agent.args", &args_json)?;
    }
    if let Some(model) = &inputs.model {
        set_toml_dotted_key(&mut doc, "agent.default_model", model)?;
    }
    set_toml_dotted_key(
        &mut doc,
        "budget.prompt_token_budget",
        &token_budget.to_string(),
    )?;
    if enable_gates {
        let rungs = default_cargo_gate_rungs();
        let rungs = toml::Value::try_from(rungs).context("serialize gates")?;
        let mut gates = toml::map::Map::new();
        gates.insert("rungs".to_string(), rungs);
        doc.as_table_mut()
            .ok_or_else(|| anyhow!("config root is not a table"))?
            .insert("gates".to_string(), toml::Value::Table(gates));
    }
    set_toml_dotted_key(&mut doc, "runner.plan_timeout_secs", "3600")?;
    set_toml_dotted_key(&mut doc, "serve.auth.enabled", "false")?;
    set_toml_dotted_key(&mut doc, "serve.auth.api_key", "")?;
    let rendered = toml::to_string_pretty(&doc).context("serialize config")?;

    println!("\n--- generated config ---");
    println!("{rendered}");
    println!("--- end config ---\n");

    if !inputs.yes && path.exists() {
        let diff_ok = prompt_bool(
            &format!("{} already exists. Overwrite?", path.display()),
            false,
        )?;
        if !diff_ok {
            return Err(anyhow!("cancelled"));
        }
    } else if !inputs.yes {
        let confirm = prompt_bool(&format!("Write to {}?", path.display()), true)?;
        if !confirm {
            return Err(anyhow!("cancelled"));
        }
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    std::fs::write(&path, rendered).with_context(|| format!("write {}", path.display()))?;
    println!("wrote {}", path.display());

    // Also create ~/.roko/.env with a commented template if it doesn't exist.
    if let Some(parent) = path.parent() {
        let env_path = parent.join(".env");
        if !env_path.exists() {
            let template = "\
# Roko environment — secrets and API keys.
# This file is loaded at startup by load_startup_env_files().
#
# ANTHROPIC_API_KEY=sk-ant-...
# GITHUB_TOKEN=ghp_...
# SLACK_BOT_TOKEN=xoxb-...
";
            std::fs::write(&env_path, template)
                .with_context(|| format!("write {}", env_path.display()))?;
            println!("wrote {}", env_path.display());
        }
    }

    Ok(path)
}

/// The compile and clippy rungs `config init` writes for cargo gates: the
/// commands `roko config migrate` gives the legacy cargo `[[gate]]` entries.
fn default_cargo_gate_rungs() -> Vec<GateRungConfig> {
    [
        ("compile", "cargo check --workspace"),
        (
            "clippy",
            "cargo clippy --workspace --no-deps -- -D warnings",
        ),
    ]
    .into_iter()
    .map(|(name, command)| GateRungConfig {
        name: name.to_string(),
        command: command.to_string(),
        timeout_secs: 600,
        required: true,
        parallel_with: Vec::new(),
        ..Default::default()
    })
    .collect()
}

/// Print the effective merged config with `[source]` tags on each field.
pub fn cmd_show(workdir: &Path) -> Result<()> {
    let resolved = load_resolved_config(workdir)?;
    print_resolved(&resolved);
    Ok(())
}

/// Print the fully-resolved config as TOML after global merge and env var overrides.
///
/// Secret values (API keys, tokens, credentials, extra headers) are redacted
/// before display so that `roko config show --effective` never leaks secrets
/// to stdout or logs.
pub fn cmd_show_effective(workdir: &Path) -> Result<()> {
    let config = roko_core::config::loader::load_config_unified(workdir)
        .map_err(|e| anyhow::anyhow!("load config: {e}"))?;
    let toml_str = roko_core::config::loader::serialize_effective_redacted(&config)
        .map_err(|e| anyhow::anyhow!("serialize config: {e}"))?;
    print!("{toml_str}");
    Ok(())
}

/// Print one section of the fully-resolved config as TOML, secrets redacted as
/// for `--effective`. `section` is a top-level table such as `agent` or a dotted
/// path such as `providers.anthropic`.
pub fn cmd_show_section(workdir: &Path, section: &str) -> Result<()> {
    let config = roko_core::config::loader::load_config_unified(workdir)
        .map_err(|e| anyhow::anyhow!("load config: {e}"))?;
    print!("{}", render_config_section(&config, section)?);
    Ok(())
}

/// The value at the dotted path `section` of the redacted effective config,
/// rendered as TOML under its own table header.
fn render_config_section(config: &RokoConfig, section: &str) -> Result<String> {
    let effective = roko_core::config::loader::serialize_effective_redacted(config)
        .map_err(|e| anyhow!("serialize config: {e}"))?;
    let root = toml::Value::Table(toml::from_str(&effective).context("parse the config")?);
    let keys: Vec<&str> = section.split('.').collect();
    let mut value = &root;
    for (depth, key) in keys.iter().enumerate() {
        let table = value
            .as_table()
            .ok_or_else(|| anyhow!("`{}` is a value, not a section", keys[..depth].join(".")))?;
        value = table.get(*key).ok_or_else(|| {
            let known: Vec<&str> = table.keys().map(String::as_str).collect();
            anyhow!(
                "the config has no section `{}`; the keys at that level are: {}",
                keys[..=depth].join("."),
                known.join(", ")
            )
        })?;
    }
    // Nest the value under its keys again so it prints with its table header.
    let mut wrapped = value.clone();
    for key in keys.iter().rev() {
        let mut table = toml::Table::new();
        table.insert(String::from(*key), wrapped);
        wrapped = toml::Value::Table(table);
    }
    toml::to_string_pretty(&wrapped).context("render the config section")
}

/// Print the resolved config paths (global + project + env override).
pub fn cmd_path(workdir: &Path) -> Result<()> {
    let resolved = load_resolved_config(workdir)?;
    let (global_display, global_exists) = match resolved.paths.global.as_deref() {
        Some(p) if p.is_file() => (p.display().to_string(), "exists"),
        Some(p) => (p.display().to_string(), "missing"),
        None => ("(unavailable)".to_string(), "missing"),
    };
    println!("global : {global_display} ({global_exists})");
    match &resolved.paths.project {
        Some(p) => println!("project: {}", p.display()),
        None => println!("project: (none)"),
    }
    if let Some(env) = &resolved.paths.env_override {
        println!("env    : {} (via ROKO_CONFIG)", env.display());
    }
    Ok(())
}

/// Print basic config health without mutating config files.
pub fn cmd_doctor(workdir: &Path) -> Result<()> {
    let paths = resolve_paths(workdir);
    let config_path = doctor_config_path(&paths, workdir);
    let config = match &config_path {
        Some(path) => {
            let text =
                fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
            RokoConfig::from_toml(&text).with_context(|| format!("parse {}", path.display()))?
        }
        None => RokoConfig::default(),
    };

    println!("config doctor");
    match &config_path {
        Some(path) => println!("config_path: {}", path.display()),
        None => println!("config_path: (none; using defaults)"),
    }
    println!("config_version: {}", config.config_version);
    println!("supported_config_version: {CURRENT_CONFIG_VERSION}");
    println!("schema_version: {}", config.schema_version);
    println!("supported_schema_version: {CURRENT_SCHEMA_VERSION}");
    println!("providers: {}", config.effective_providers().len());
    println!("models: {}", config.effective_models().len());
    println!(
        "dangerously_skip_permissions: {}",
        config.runner.dangerously_skip_permissions
    );

    // Settings users commonly tune that `plan run` never reads.
    let inert = crate::graph_task_dispatch::graph_engine_inert_settings(&config);
    if !inert.is_empty() {
        println!("---");
        for setting in &inert {
            println!(
                "[warn] {}: set, but no effect on `plan run` ({})",
                setting.key, setting.reason
            );
        }
    }

    // Run the shared diagnostic checks for config doctor (#279).
    use roko_execution::diagnostics::{
        DiagnosticCheckId, DiagnosticRequest, DiagnosticService, DiagnosticSeverity,
    };

    let selected = [
        DiagnosticCheckId::Config,
        DiagnosticCheckId::SchemaVersion,
        DiagnosticCheckId::Providers,
        DiagnosticCheckId::Models,
    ]
    .into_iter()
    .collect();

    let report = DiagnosticService::run(&DiagnosticRequest {
        workdir: workdir.to_path_buf(),
        selected,
        profile: None,
        allow_repairs: false,
    });

    if !report.findings.is_empty() {
        println!("---");
        for finding in &report.findings {
            let label = match finding.severity {
                DiagnosticSeverity::Info => "ok",
                DiagnosticSeverity::Warning => "warn",
                DiagnosticSeverity::Error => "FAIL",
            };
            println!("[{label}] {}: {}", finding.check_id, finding.message);
            if let Some(ref remediation) = finding.remediation {
                if let Some(ref cmd) = remediation.command {
                    println!("    fix: {cmd}");
                }
            }
        }
    }

    Ok(())
}

fn doctor_config_path(paths: &ConfigPaths, workdir: &Path) -> Option<PathBuf> {
    if let Some(path) = &paths.project {
        return Some(path.clone());
    }

    let direct = workdir.join("roko.toml");
    if direct.is_file() {
        return Some(direct);
    }

    if let Some(ref global) = paths.global {
        if global.is_file() {
            return Some(global.clone());
        }
    }

    None
}

/// Scan the active config file for `${VAR}` references and validate them.
///
/// All referenced env vars must exist and non-empty. `GITHUB_TOKEN` is
/// additionally validated against the GitHub API, and `SLACK_BOT_TOKEN`
/// against Slack's `auth.test` API.
pub fn cmd_check_secrets(workdir: &Path) -> Result<()> {
    let paths = resolve_paths(workdir);
    let config_path = secret_check_config_path(&paths)?;
    let text = std::fs::read_to_string(&config_path)
        .with_context(|| format!("read config {}", config_path.display()))?;
    let tokens = collect_env_tokens(&text)?;

    if tokens.is_empty() {
        println!("no `${{VAR}}` tokens found in {}", config_path.display());
        return Ok(());
    }

    println!(
        "checking {} referenced secret token(s) in {}",
        tokens.len(),
        config_path.display()
    );

    let client = reqwest::blocking::Client::builder()
        .user_agent("roko-cli/0.1")
        .timeout(Duration::from_secs(10))
        .build()
        .context("build HTTP client")?;

    let mut missing = Vec::new();
    let mut invalid = Vec::new();

    for token in tokens {
        print!("  {token}: ");
        match std::env::var(&token) {
            Ok(value) if !value.is_empty() => {
                if let Some(target) = secret_validation_target(&token) {
                    match validate_secret_token(&client, target, &value) {
                        Ok(()) => {
                            println!("valid ({})", target.label());
                        }
                        Err(err) => {
                            println!("invalid ({err})");
                            invalid.push(format!("{token}: {err}"));
                        }
                    }
                } else {
                    println!("present");
                }
            }
            _ => {
                println!("missing");
                missing.push(token);
            }
        }
    }

    if missing.is_empty() && invalid.is_empty() {
        println!("all referenced secret tokens are set and valid");
        return Ok(());
    }

    let mut message = String::from("secret check failed");
    if !missing.is_empty() {
        message.push_str(&format!("\nmissing: {}", missing.join(", ")));
    }
    if !invalid.is_empty() {
        message.push_str(&format!("\ninvalid: {}", invalid.join(", ")));
    }
    Err(anyhow!(message))
}

/// Validate the active `roko.toml` in phases: syntax, config paths, secrets,
/// schema, the core loader's invariants, and semantics.
pub async fn cmd_validate(workdir: &Path) -> Result<()> {
    let paths = resolve_paths(workdir);
    let config_path = validate_config_path(&paths, workdir)?;
    let text = match fs::read_to_string(&config_path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(anyhow!("config file not found: {}", config_path.display()));
        }
        Err(e) => {
            return Err(
                anyhow::Error::new(e).context(format!("read config {}", config_path.display()))
            );
        }
    };

    let parsed_value = match toml::from_str::<toml::Value>(&text) {
        Ok(v) => v,
        Err(err) => {
            print_phase_status("Phase 1: TOML syntax", false);
            println!("  ✗ {err}");
            println!();
            println!("Result: 0 warnings, 1 error");
            return Err(anyhow!("config validation failed"));
        }
    };
    print_phase_status("Phase 1: TOML syntax", true);

    // Phase 1b: validate all TOML paths against the RokoConfig schema.
    // Unknown paths are errors in `roko config validate`.
    let path_diagnostics = roko_core::config::loader::validate_known_config_paths(&parsed_value);
    if !path_diagnostics.is_empty() {
        print_phase_status("Phase 1b: Config path validation", false);
        for diag in &path_diagnostics {
            println!("  ✗ {}: {}", config_path.display(), diag.message);
        }
        println!();
        println!("Result: 0 warnings, {} errors", path_diagnostics.len());
        return Err(anyhow!("config validation failed"));
    }
    print_phase_status("Phase 1b: Config path validation", true);

    // Phase 1c: the loader refuses a config file agents can read while it
    // holds a secret, so validation does too.
    let secrets = roko_core::config::loader::refuse_readable_secrets(&config_path, &parsed_value);
    if let Err(err) = secrets {
        print_phase_status("Phase 1c: Secrets", false);
        println!("  ✗ {err}");
        println!();
        println!("Result: 0 warnings, 1 error");
        return Err(anyhow!("config validation failed"));
    }
    print_phase_status("Phase 1c: Secrets", true);

    let config = match toml::from_str::<RokoConfig>(&text) {
        Ok(config) => config,
        Err(err) => {
            print_phase_status("Phase 2: Schema validation", false);
            println!("  ✗ {err}");
            println!();
            println!("Result: 0 warnings, 1 error");
            return Err(anyhow!("config validation failed"));
        }
    };
    print_phase_status("Phase 2: Schema validation", true);

    // Phase 2b: the core loader, which every command loads roko.toml
    // through. It merges the global config and env overrides and then
    // enforces the cross-section invariants (budget ceilings, model ->
    // provider references), so a file it rejects must fail here too.
    let effective = match load_like_commands(&config_path, config.clone()) {
        Ok(effective) => effective,
        Err(err) => {
            print_phase_status("Phase 2b: Loader invariants", false);
            println!("  ✗ {err:#}");
            println!();
            println!("Result: 0 warnings, 1 error");
            return Err(anyhow!("config validation failed"));
        }
    };
    print_phase_status("Phase 2b: Loader invariants", true);

    let client = reqwest::Client::builder()
        .user_agent("roko-cli/0.1")
        .timeout(Duration::from_secs(VALIDATION_REACHABILITY_TIMEOUT_SECS))
        .build()
        .context("build validation HTTP client")?;
    let mut report = semantic_validate_config(&config, &client).await;
    if let Some(warning) = legacy_layout_warning(&config) {
        report.schema_warnings.push(warning);
    }
    report.schema_warnings.extend(loader_warnings(&effective));

    println!("Phase 3: Semantic validation:");
    print_warning_section("Schema warnings", &report.schema_warnings);
    print_warning_section("Field warnings", &report.field_warnings);
    print_warning_section("Migration warnings", &report.migration_warnings);
    print_semantic_result("API key env vars are set", &report.api_key_errors);

    println!();
    println!(
        "Summary: schema {} warnings, field {} warnings, migration {} warnings, {} errors",
        report.schema_warning_count(),
        report.field_warning_count(),
        report.migration_warning_count(),
        report.error_count()
    );
    println!(
        "Result: {} warnings, {} errors",
        report.warning_count(),
        report.error_count()
    );

    if report.error_count() == 0 {
        Ok(())
    } else {
        Err(anyhow!("config validation failed"))
    }
}

/// Check prospective `roko.toml` text before a command writes it to `path`.
///
/// Runs every offline check of [`cmd_validate`]: TOML syntax, no secret in a
/// file agents can read, known config paths, the schema, the core loader
/// (global config merge, env overrides and its cross-section invariants) and
/// the provider/model semantic errors. Only the network probes, which can
/// only warn, are left out. Text that passes loads in every command and
/// passes `roko config validate`.
pub fn check_config_text(path: &Path, text: &str) -> Result<()> {
    let value = toml::from_str::<toml::Value>(text).context("invalid TOML")?;
    roko_core::config::loader::refuse_readable_secrets(path, &value)?;
    let unknown_paths = roko_core::config::loader::validate_known_config_paths(&value);
    if !unknown_paths.is_empty() {
        let messages = unknown_paths
            .iter()
            .map(|diag| diag.message.as_str())
            .collect::<Vec<_>>();
        return Err(anyhow!("unknown config paths: {}", messages.join("; ")));
    }
    let config = toml::from_str::<RokoConfig>(text).context("config does not match the schema")?;
    let errors = semantic_errors(&config);
    load_like_commands(path, config)?;
    if !errors.is_empty() {
        return Err(anyhow!(errors.join("; ")));
    }
    Ok(())
}

/// Write config `text` to `path` if it passes [`check_config_text`].
///
/// Commands that write `roko.toml` go through this, so none of them leaves a
/// config that `roko config validate` or the loader rejects: a rejected text
/// is not written and the file keeps its previous contents.
pub fn write_checked_config(path: &Path, text: &str) -> Result<()> {
    check_config_text(path, text).with_context(|| {
        format!(
            "refusing to write {}: `roko config validate` would reject it",
            path.display()
        )
    })?;
    roko_fs::atomic_write_bytes(path, text.as_bytes())
        .with_context(|| format!("write {}", path.display()))
}

/// Append the TOML text `block` to the config file at `path`, keeping its
/// existing text and comments, if the result passes [`check_config_text`].
pub fn append_checked_config(path: &Path, block: &str) -> Result<()> {
    let mut text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(block);
    write_checked_config(path, &text)
}

/// Add a `[providers.*]` block to the `roko.toml` at `path` for each
/// provider `roko setup` detected that the file does not configure yet, and
/// return how many were added.
///
/// `clis` holds `(command, description)` for LLM CLIs found on `PATH`, and
/// `api_keys` holds `(env var, display name, kind)` for API keys set in the
/// environment. The blocks carry only schema keys, and the file must pass
/// [`check_config_text`] afterwards or nothing is written.
pub fn add_detected_providers(
    path: &Path,
    clis: &[(String, String)],
    api_keys: &[(&str, &str, &str)],
) -> Result<usize> {
    let existing = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    // Parse rather than search the text: a commented-out block such as
    // `# [providers.claude_cli]` configures nothing.
    let parsed = toml::from_str::<toml::Value>(&existing)
        .with_context(|| format!("parse {}", path.display()))?;
    let mut configured = parsed
        .get("providers")
        .and_then(toml::Value::as_table)
        .map(|providers| providers.keys().cloned().collect::<BTreeSet<_>>())
        .unwrap_or_default();
    let mut appended = String::new();
    let mut count = 0usize;

    // CLI providers.
    for (cmd, _desc) in clis {
        let (provider_name, kind) = match cmd.as_str() {
            "claude" => ("claude_cli", "claude_cli"),
            "codex" => ("codex_cli", "codex_cli"),
            other => (other, "openai_compat"),
        };
        if !configured.insert(provider_name.to_string()) {
            continue;
        }
        appended.push_str(&format!("\n[providers.{provider_name}]\n"));
        appended.push_str(&format!("kind = \"{kind}\"\n"));
        appended.push_str(&format!("command = \"{cmd}\"\n"));
        count += 1;
    }

    // API key providers, named after their catalog entry.
    for (env_var, _display, kind) in api_keys {
        let entry = roko_core::provider_catalog::catalog()
            .iter()
            .find(|entry| entry.api_key_env == *env_var);
        let provider_name =
            entry.map_or_else(|| env_var.trim_end_matches("_API_KEY"), |entry| entry.id);
        if !configured.insert(provider_name.to_string()) {
            continue;
        }
        appended.push_str(&format!("\n[providers.{provider_name}]\n"));
        appended.push_str(&format!("kind = \"{kind}\"\n"));
        appended.push_str(&format!("api_key_env = \"{env_var}\"\n"));
        let base_url = entry.map_or("", |entry| entry.base_url);
        if !base_url.is_empty() {
            appended.push_str(&format!("base_url = \"{base_url}\"\n"));
        }
        count += 1;
    }

    if count > 0 {
        append_checked_config(path, &appended)?;
    }
    Ok(count)
}

/// Migrate a legacy project-local `roko.toml` into explicit provider/model tables.
pub fn cmd_migrate(workdir: &Path, dry_run: bool, yes: bool) -> Result<()> {
    let paths = resolve_paths(workdir);
    let config_path = validate_config_path(&paths, workdir)?;
    let text = match fs::read_to_string(&config_path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(anyhow!("config file not found: {}", config_path.display()));
        }
        Err(e) => {
            return Err(anyhow::Error::new(e).context(format!("read {}", config_path.display())));
        }
    };
    let plan = build_config_migration_plan(&text)?;

    match plan {
        ConfigMigrationPlan::AlreadyCurrent => {
            println!("roko.toml already uses [providers.*]; nothing to migrate");
            Ok(())
        }
        ConfigMigrationPlan::Legacy(plan) => {
            println!("Detected roko.toml version 1 (no [providers] section)");
            println!();
            println!("Proposed changes:");
            for line in render_migration_preview(&plan)? {
                println!("{line}");
            }

            if dry_run {
                println!();
                println!("[dry-run] no changes written");
                return Ok(());
            }

            println!();
            if !yes && !prompt_bool("Apply changes?", false)? {
                return Err(anyhow!("cancelled"));
            }

            fs::write(&config_path, &plan.rendered)
                .with_context(|| format!("write {}", config_path.display()))?;
            println!("updated {}", config_path.display());
            Ok(())
        }
    }
}

/// Set a secret in `~/.roko/.env`, updating an existing key if present.
pub fn cmd_set_secret(name: &str, value: &str) -> Result<()> {
    let home = std::env::var_os("HOME").ok_or_else(|| anyhow!("HOME is not set"))?;
    let path = PathBuf::from(home).join(".roko").join(".env");
    write_secret_env_file(&path, name, value)?;
    println!("set {name} in {}", path.display());
    Ok(())
}

fn write_secret_env_file(path: &Path, name: &str, value: &str) -> Result<()> {
    if value.contains(['\n', '\r']) {
        return Err(anyhow!("the value of {name} must fit on one line"));
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }

    let existing = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(err) => return Err(err).with_context(|| format!("read {}", path.display())),
    };
    let rendered = upsert_env_assignment(&existing, name, &env_file_value(value));
    write_atomic_restricted(path, &rendered)?;
    Ok(())
}

/// `value` as a `.env` value that dotenv reads back unchanged: bare when it
/// is plain, else in single quotes, inside which dotenv expands no `$` and
/// no escape.
fn env_file_value(value: &str) -> String {
    if value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "-_.:/+=@,".contains(c))
    {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', r"'\''"))
    }
}

fn upsert_env_assignment(existing: &str, name: &str, value: &str) -> String {
    let mut lines = Vec::new();
    let mut replaced = false;
    let replacement = format!("{name}={value}");

    for line in existing.lines() {
        if env_assignment_name(line).as_deref() == Some(name) {
            lines.push(replacement.clone());
            replaced = true;
        } else {
            lines.push(line.to_string());
        }
    }

    if !replaced {
        lines.push(replacement);
    }

    lines.join("\n")
}

fn env_assignment_name(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let trimmed = trimmed.strip_prefix("export ").unwrap_or(trimmed);
    let (name, _) = trimmed.split_once('=')?;
    let name = name.trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

fn write_atomic_restricted(path: &Path, text: &str) -> Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    let file_name = path
        .file_name()
        .ok_or_else(|| anyhow!("path {} has no file name", path.display()))?;
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before UNIX_EPOCH")?
        .as_nanos();
    let tmp_path = parent.join(format!(".{}.{}.tmp", file_name.to_string_lossy(), unique));

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .truncate(true)
            .mode(0o600)
            .open(&tmp_path)
            .with_context(|| format!("create {}", tmp_path.display()))?;
        file.write_all(text.as_bytes())
            .with_context(|| format!("write {}", tmp_path.display()))?;
        file.sync_all()
            .with_context(|| format!("sync {}", tmp_path.display()))?;
    }
    #[cfg(not(unix))]
    {
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .truncate(true)
            .open(&tmp_path)
            .with_context(|| format!("create {}", tmp_path.display()))?;
        file.write_all(text.as_bytes())
            .with_context(|| format!("write {}", tmp_path.display()))?;
        file.sync_all()
            .with_context(|| format!("sync {}", tmp_path.display()))?;
    }

    fs::rename(&tmp_path, path).with_context(|| format!("replace {}", path.display()))?;
    Ok(())
}

/// Open `$EDITOR` on the global or project config file (creating it if needed).
pub fn cmd_edit(workdir: &Path, which: EditTarget) -> Result<()> {
    let resolved = load_resolved_config(workdir)?;
    let path = match which {
        EditTarget::Global => resolved
            .paths
            .global
            .ok_or_else(|| anyhow!("cannot determine global config path: HOME is not set"))?,
        EditTarget::Project => resolved
            .paths
            .project
            .unwrap_or_else(|| workdir.join("roko.toml")),
        EditTarget::Auto => resolved
            .paths
            .project
            .or(resolved.paths.global)
            .unwrap_or_else(|| workdir.join("roko.toml")),
    };

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    if !path.exists() {
        std::fs::write(&path, "# roko config\n")
            .with_context(|| format!("create {}", path.display()))?;
    }

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".into());
    let status = std::process::Command::new(&editor)
        .arg(&path)
        .status()
        .with_context(|| format!("spawn {editor}"))?;
    if !status.success() {
        return Err(anyhow!("{editor} exited non-zero"));
    }
    println!("edited {}", path.display());
    Ok(())
}

/// Set a single dotted-key value and write it to the chosen layer file.
///
/// See [`set_config_key`]: the key is written under its v2 name, and a
/// project `roko.toml` edit that `roko config validate` would reject is
/// refused. A secret such as `serve.auth.api_key` goes to `.roko/.env`
/// instead, whatever the target ([`set_secret_config_key`]).
pub fn cmd_set(workdir: &Path, target: EditTarget, key: &str, value: &str) -> Result<()> {
    // Only the paths are needed. Loading the config would stop `config set`
    // from repairing a file that no longer loads.
    let paths = resolve_paths(workdir);
    if let Some(stored) = set_secret_config_key(workdir, &paths, key, value)? {
        let StoredSecret {
            key,
            variable,
            env_file,
            removed_from,
        } = stored;
        println!(
            "stored {key} as {variable} in {}, which agents cannot read",
            env_file.display()
        );
        for path in removed_from {
            println!("removed {key} from {}", path.display());
        }
        let summary = format!("config set {key} (stored as {variable} in .roko/.env)");
        journal_config_set(workdir, &key, summary);
        return Ok(());
    }
    let path = match target {
        EditTarget::Global | EditTarget::Auto => paths
            .global
            .ok_or_else(|| anyhow!("cannot determine global config path: HOME is not set"))?,
        EditTarget::Project => paths.project.unwrap_or_else(|| workdir.join("roko.toml")),
    };

    let key = set_config_key(&path, target, key, value)?;
    println!("set {key} = {value} in {}", path.display());
    journal_config_set(workdir, &key, format!("config set {key} = {value}"));

    Ok(())
}

/// Append a `config set` entry to the config journal so changes can be traced.
fn journal_config_set(workdir: &Path, key: &str, summary: String) {
    let journal_path = workdir.join(".roko").join("config-journal.jsonl");
    let change = ConfigChange {
        section: ConfigSection::Other(key.split('.').next().unwrap_or(key).to_string()),
        summary,
    };
    if let Err(err) = hot_reload::append_config_journal(&journal_path, &[change], "config-set") {
        tracing::warn!(error = %err, "failed to append config journal entry");
    }
}

/// A secret that [`set_secret_config_key`] stored in `.roko/.env`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredSecret {
    /// The field, under its v2 name (`serve.auth.api_key`).
    pub key: String,
    /// The variable that now sets it (`ROKO__SERVE__AUTH__API_KEY`).
    pub variable: String,
    /// The `.roko/.env` file written.
    pub env_file: PathBuf,
    /// The config files agents can read that set the field, which no longer do.
    pub removed_from: Vec<PathBuf>,
}

/// Store a secret config field in the project's `.roko/.env`, as its `ROKO__`
/// variable, and remove the field from the config files agents can read.
///
/// roko loads `.roko/.env` at startup, and agents cannot read it; the loader
/// refuses a readable config file that holds a secret. `paths` are the config
/// files of `workdir`: `roko.toml`, the file `ROKO_CONFIG` names and the
/// legacy `~/.config/roko/config.toml`. A key file such as
/// `~/.roko/config.toml` is left as it is. The project is the directory of
/// its `roko.toml`, or `workdir` when there is none.
///
/// Returns `None` when `key = value` is no secret, for example an empty value
/// or a `${VAR}` reference, so the caller writes it to a config file.
///
/// # Errors
///
/// A secret that no `ROKO__` variable can set, such as a provider header, a
/// value that spans lines, or a file that cannot be read or written.
pub fn set_secret_config_key(
    workdir: &Path,
    paths: &ConfigPaths,
    key: &str,
    value: &str,
) -> Result<Option<StoredSecret>> {
    let key = v2_config_key(key);
    let mut probe = toml::Value::Table(toml::map::Map::new());
    set_toml_dotted_key(&mut probe, &key, value).with_context(|| format!("set {key}"))?;
    let fields = roko_core::config::loader::secret_fields(&probe);
    let Some(field) = fields.first() else {
        return Ok(None);
    };
    let variable = roko_core::config::loader::env_override_name(field)
        .filter(|_| *field == key)
        .ok_or_else(|| {
            anyhow!(
                "{field} is a secret, which roko keeps out of the config files agents can \
                 read: set it in .roko/.env instead, and give a provider header a ${{VAR}} \
                 reference to it"
            )
        })?;
    let project = paths.project.as_deref().and_then(Path::parent);
    let env_file = project.unwrap_or(workdir).join(".roko").join(".env");
    write_secret_env_file(&env_file, &variable, value)?;

    let mut removed_from = Vec::new();
    for path in [&paths.env_override, &paths.project, &paths.global]
        .into_iter()
        .flatten()
    {
        if roko_core::child_env::is_key_file(path) || !path.is_file() {
            continue;
        }
        let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        if let Some(rest) = remove_toml_key(&text, &key) {
            roko_fs::atomic_write_bytes(path, rest.as_bytes())
                .with_context(|| format!("write {}", path.display()))?;
            removed_from.push(path.clone());
        }
    }
    Ok(Some(StoredSecret {
        key,
        variable,
        env_file,
        removed_from,
    }))
}

/// The TOML document `text` without the dotted `key`, the rest kept as
/// written. `None` when `text` does not parse or does not set `key`.
fn remove_toml_key(text: &str, key: &str) -> Option<String> {
    let mut document = text.parse::<toml_edit::DocumentMut>().ok()?;
    let segments = key.split('.').collect::<Vec<_>>();
    remove_table_key(document.as_table_mut(), &segments).then(|| document.to_string())
}

fn remove_table_key(table: &mut dyn toml_edit::TableLike, segments: &[&str]) -> bool {
    match segments {
        [] => false,
        [leaf] => table.remove(leaf).is_some(),
        [parent, rest @ ..] => table
            .get_mut(parent)
            .and_then(toml_edit::Item::as_table_like_mut)
            .is_some_and(|child| remove_table_key(child, rest)),
    }
}

/// Set `key` to `value` in the config file at `path`, the `target` layer,
/// and return the key that was written.
///
/// A v1 key name that schema v2 renamed (`agent.model`) is written under its
/// v2 name (`agent.default_model`), so the file never gains a key that
/// validation rejects. A project file must pass [`check_config_text`] after
/// the edit, or it is not written.
pub fn set_config_key(path: &Path, target: EditTarget, key: &str, value: &str) -> Result<String> {
    let key = v2_config_key(key);
    let mut doc = if path.exists() {
        read_toml_file(path)?
    } else {
        toml::Value::Table(toml::map::Map::new())
    };
    set_toml_dotted_key(&mut doc, &key, value).with_context(|| format!("set {key} = {value}"))?;
    if target == EditTarget::Project {
        let text = toml::to_string_pretty(&doc).context("serialize config")?;
        write_checked_config(path, &text)?;
    } else {
        write_toml_file(path, &doc)?;
    }
    Ok(key)
}

/// The v2 name of a dotted config key (`agent.model` -> `agent.default_model`).
fn v2_config_key(key: &str) -> String {
    roko_core::config::loader::V1_RENAMED_KEYS
        .iter()
        .find(|(table, old, _)| key.split_once('.') == Some((*table, *old)))
        .map_or_else(
            || key.to_string(),
            |(table, _, new)| format!("{table}.{new}"),
        )
}

/// Which file `config edit` / `config set` should target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditTarget {
    /// `~/.roko/config.toml`.
    Global,
    /// `./roko.toml` (creating it if absent).
    Project,
    /// Project if one exists, else global.
    Auto,
}

// -----------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------

fn print_resolved(r: &ResolvedConfig) {
    println!("effective config:");
    println!(
        "  agent.command      = {:?} {}",
        r.config.agent.command,
        r.sources.agent_command.tag()
    );
    println!(
        "  agent.args         = {:?} {}",
        r.config.agent.args,
        r.sources.agent_args.tag()
    );
    println!(
        "  agent.model        = {:?} {}",
        r.config.agent.model,
        r.sources.agent_model.tag()
    );
    println!(
        "  agent.effort       = {:?} {}",
        r.config.agent.effort,
        r.sources.agent_effort.tag()
    );
    println!(
        "  agent.bare_mode    = {} {}",
        r.config.agent.bare_mode,
        r.sources.agent_bare_mode.tag()
    );
    println!(
        "  agent.fallback_model = {:?} {}",
        r.config.agent.fallback_model,
        r.sources.agent_fallback_model.tag()
    );
    println!(
        "  agent.timeout_ms   = {} {}",
        r.config.agent.timeout_ms,
        r.sources.agent_timeout_ms.tag()
    );
    // Serialize providers to TOML and redact secret fields (api_key, tokens,
    // extra_headers values, etc.) before printing so that `roko config show`
    // never leaks literal API keys or bearer tokens to stdout.
    let providers_display = toml::to_string_pretty(&r.config.providers)
        .map(|s| roko_core::config::loader::redact_secrets_in_toml_str(&s))
        .unwrap_or_else(|_| "***REDACTED***".to_string());
    println!(
        "  providers         = {} {}",
        providers_display.trim(),
        r.sources.providers.tag()
    );
    let models_display = toml::to_string_pretty(&r.config.models)
        .unwrap_or_else(|_| format!("{:?}", r.config.models));
    println!(
        "  models            = {} {}",
        models_display.trim(),
        r.sources.models.tag()
    );
    println!(
        "  dreams.auto_dream  = {} {}",
        r.config.dreams.auto_dream,
        r.sources.dreams_auto_dream.tag()
    );
    println!(
        "  dreams.idle_threshold_mins = {} {}",
        r.config.dreams.idle_threshold_mins,
        r.sources.dreams_idle_threshold_mins.tag()
    );
    println!(
        "  dreams.min_episodes_for_dream = {} {}",
        r.config.dreams.min_episodes_for_dream,
        r.sources.dreams_min_episodes_for_dream.tag()
    );
    println!(
        "  dreams.scheduled_cron = {:?} {}",
        r.config.dreams.scheduled_cron,
        r.sources.dreams_scheduled_cron.tag()
    );
    println!(
        "  dreams.episode_count_trigger = {} {}",
        r.config.dreams.episode_count_trigger,
        r.sources.dreams_episode_count_trigger.tag()
    );
    println!(
        "  dreams.quality_gain = {} {}",
        r.config.dreams.quality_gain,
        r.sources.dreams_quality_gain.tag()
    );
    println!(
        "  dreams.quality_penalty = {} {}",
        r.config.dreams.quality_penalty,
        r.sources.dreams_quality_penalty.tag()
    );
    println!(
        "  gates              = {} entries {}",
        r.config.gates.len(),
        r.sources.gates.tag()
    );
    println!(
        "  runner.plan_timeout_secs = {} {}",
        r.config.runner.plan_timeout_secs,
        r.sources.runner_plan_timeout_secs.tag()
    );
    println!();
    println!("sources:");
    match r.paths.global.as_deref() {
        Some(p) => println!("  global : {}", p.display()),
        None => println!("  global : (unavailable)"),
    }
    match &r.paths.project {
        Some(p) => println!("  project: {}", p.display()),
        None => println!("  project: (none)"),
    }
    if let Some(env) = &r.paths.env_override {
        println!("  env    : {} (ROKO_CONFIG)", env.display());
    }
    let fully_default = r.sources.agent_command == Source::Default
        && r.sources.prompt_token_budget == Source::Default
        && r.sources.providers == Source::Default
        && r.sources.models == Source::Default
        && r.sources.dreams_auto_dream == Source::Default
        && r.sources.dreams_idle_threshold_mins == Source::Default
        && r.sources.dreams_min_episodes_for_dream == Source::Default
        && r.sources.dreams_scheduled_cron == Source::Default
        && r.sources.dreams_episode_count_trigger == Source::Default
        && r.sources.dreams_quality_gain == Source::Default
        && r.sources.dreams_quality_penalty == Source::Default
        && r.sources.runner_plan_timeout_secs == Source::Default;
    if fully_default {
        println!("\nhint: no config files found — run `roko config init` to set one up.");
    }
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
enum ConfigMigrationPlan {
    AlreadyCurrent,
    Legacy(LegacyConfigMigration),
}

#[derive(Debug)]
struct LegacyConfigMigration {
    provider_name: String,
    provider: ProviderConfig,
    models: Vec<(String, ModelProfile)>,
    rendered: String,
}

fn build_config_migration_plan(text: &str) -> Result<ConfigMigrationPlan> {
    let raw_value: toml::Value = toml::from_str(text).context("parse config toml")?;
    let mut raw = raw_value
        .as_table()
        .cloned()
        .ok_or_else(|| anyhow!("config root must be a TOML table"))?;

    if raw
        .get("providers")
        .and_then(toml::Value::as_table)
        .is_some_and(|providers| !providers.is_empty())
    {
        return Ok(ConfigMigrationPlan::AlreadyCurrent);
    }

    let config = RokoConfig::from_toml(text).context("parse roko config")?;
    let provider = legacy_provider_config(&config)?;
    let models = legacy_model_profiles(&config, &provider.0)?;

    let provider_value = toml::Value::try_from(provider.1.clone()).context("serialize provider")?;
    let mut providers_table = toml::map::Map::new();
    providers_table.insert(provider.0.clone(), provider_value);
    raw.insert("providers".to_string(), toml::Value::Table(providers_table));

    let mut models_table = toml::map::Map::new();
    for (model_key, profile) in &models {
        let value = toml::Value::try_from(profile.clone())
            .with_context(|| format!("serialize model '{model_key}'"))?;
        models_table.insert(model_key.clone(), value);
    }
    raw.insert("models".to_string(), toml::Value::Table(models_table));
    raw.insert(
        "schema_version".to_string(),
        toml::Value::Integer(i64::from(CURRENT_SCHEMA_VERSION)),
    );

    let rendered = toml::to_string_pretty(&toml::Value::Table(raw)).context("serialize config")?;
    Ok(ConfigMigrationPlan::Legacy(LegacyConfigMigration {
        provider_name: provider.0,
        provider: provider.1,
        models,
        rendered,
    }))
}

fn legacy_provider_config(config: &RokoConfig) -> Result<(String, ProviderConfig)> {
    let command = config
        .agent
        .command
        .as_deref()
        .map(str::trim)
        .filter(|command| !command.is_empty())
        .ok_or_else(|| anyhow!("legacy config is missing agent.command"))?;

    match command {
        "claude" => Ok((
            "claude_cli".to_string(),
            ProviderConfig {
                kind: ProviderKind::ClaudeCli,
                base_url: None,
                api_key_env: None,
                command: Some(command.to_string()),
                args: config.agent.args.clone(),
                timeout_ms: config.agent.timeout_ms,
                ttft_timeout_ms: None,
                connect_timeout_ms: None,
                extra_headers: None,
                max_concurrent: None,
                limits: None,
                require_confirmation: false,
                stream_usage: None,
            },
        )),
        "ollama" => Ok((
            "ollama".to_string(),
            ProviderConfig {
                kind: ProviderKind::OpenAiCompat,
                base_url: Some(
                    legacy_agent_env(config.agent.env.as_ref(), "OLLAMA_HOST")
                        .unwrap_or("http://localhost:11434")
                        .to_string(),
                ),
                api_key_env: None,
                command: None,
                args: None,
                timeout_ms: config.agent.timeout_ms,
                ttft_timeout_ms: None,
                connect_timeout_ms: None,
                extra_headers: None,
                max_concurrent: None,
                limits: None,
                require_confirmation: false,
                stream_usage: None,
            },
        )),
        other => Err(anyhow!(
            "legacy agent.command '{other}' cannot be migrated safely; only 'claude' and 'ollama' are supported"
        )),
    }
}

fn legacy_model_profiles(
    config: &RokoConfig,
    provider_name: &str,
) -> Result<Vec<(String, ModelProfile)>> {
    let mut model_keys = BTreeSet::new();

    let default_model = config.agent.default_model.trim();
    if !default_model.is_empty() {
        model_keys.insert(default_model.to_string());
    }

    for model in config.agent.tier_models.values() {
        let model = model.trim();
        if !model.is_empty() {
            model_keys.insert(model.to_string());
        }
    }

    if model_keys.is_empty() {
        return Err(anyhow!(
            "legacy config has no agent.model or agent.tier_models to migrate"
        ));
    }

    Ok(model_keys
        .into_iter()
        .map(|model_key| {
            (
                model_key.clone(),
                synthesized_legacy_model_profile(provider_name, &model_key),
            )
        })
        .collect())
}

fn synthesized_legacy_model_profile(provider_name: &str, slug: &str) -> ModelProfile {
    let tool_profile = profile_for_model(slug);
    let tool_format = match provider_name {
        "claude_cli" => ToolFormat::AnthropicBlocks,
        _ => ToolFormat::OpenAiJson,
    };
    let context_window = if matches!(tool_format, ToolFormat::AnthropicBlocks) {
        200_000
    } else {
        128_000
    };

    ModelProfile {
        provider: provider_name.to_string(),
        slug: slug.to_string(),
        context_window,
        max_output: None,
        supports_tools: tool_profile.supports_tools,
        supports_thinking: false,
        supports_vision: false,
        supports_web_search: false,
        supports_mcp_tools: false,
        supports_partial: false,
        provider_routing: None,
        tool_format: tool_format.as_str().to_string(),
        cost_input_per_m: None,
        cost_output_per_m: None,
        cost_cache_read_per_m: None,
        cost_cache_write_per_m: None,
        max_tools: Some(u32::from(tool_profile.max_tools_before_degrade)),
        tokenizer_ratio: None,
        ..Default::default()
    }
}

fn legacy_agent_env<'a>(env: Option<&'a Vec<(String, String)>>, key: &str) -> Option<&'a str> {
    env.and_then(|entries| {
        entries.iter().find_map(|(name, value)| {
            (name.trim().eq_ignore_ascii_case(key) && !value.trim().is_empty())
                .then_some(value.as_str())
        })
    })
}

fn render_migration_preview(plan: &LegacyConfigMigration) -> Result<Vec<String>> {
    let mut lines = Vec::new();
    lines.extend(render_prefixed_toml_block(single_provider_preview(
        &plan.provider_name,
        &plan.provider,
    )?));
    for (model_key, profile) in &plan.models {
        lines.push("  +".to_string());
        lines.extend(render_prefixed_toml_block(single_model_preview(
            model_key, profile,
        )?));
    }
    lines.push("  +".to_string());
    lines.push(format!("  + schema_version = {CURRENT_SCHEMA_VERSION}"));
    Ok(lines)
}

fn single_provider_preview(name: &str, provider: &ProviderConfig) -> Result<String> {
    let mut outer = toml::map::Map::new();
    let mut providers = toml::map::Map::new();
    providers.insert(
        name.to_string(),
        toml::Value::try_from(provider.clone()).context("serialize provider preview")?,
    );
    outer.insert("providers".to_string(), toml::Value::Table(providers));
    toml::to_string_pretty(&toml::Value::Table(outer)).context("serialize provider preview")
}

fn single_model_preview(model_key: &str, profile: &ModelProfile) -> Result<String> {
    let mut outer = toml::map::Map::new();
    let mut models = toml::map::Map::new();
    models.insert(
        model_key.to_string(),
        toml::Value::try_from(profile.clone()).context("serialize model preview")?,
    );
    outer.insert("models".to_string(), toml::Value::Table(models));
    toml::to_string_pretty(&toml::Value::Table(outer)).context("serialize model preview")
}

fn render_prefixed_toml_block(block: String) -> Vec<String> {
    block
        .trim()
        .lines()
        .map(|line| format!("  + {line}"))
        .collect()
}

#[derive(Debug, Default)]
struct SemanticValidationReport {
    schema_warnings: Vec<String>,
    field_warnings: Vec<String>,
    migration_warnings: Vec<String>,
    api_key_errors: Vec<String>,
}

impl SemanticValidationReport {
    fn error_count(&self) -> usize {
        self.api_key_errors.len()
    }

    fn schema_warning_count(&self) -> usize {
        self.schema_warnings.len()
    }

    fn field_warning_count(&self) -> usize {
        self.field_warnings.len()
    }

    fn migration_warning_count(&self) -> usize {
        self.migration_warnings.len()
    }

    fn warning_count(&self) -> usize {
        self.schema_warnings.len() + self.field_warnings.len() + self.migration_warnings.len()
    }
}

fn validate_config_path(paths: &ConfigPaths, workdir: &Path) -> Result<PathBuf> {
    if let Some(path) = &paths.project {
        return Ok(path.clone());
    }

    let direct = workdir.join("roko.toml");
    if direct.is_file() {
        return Ok(direct);
    }

    Err(anyhow!("no roko.toml found to validate"))
}

fn print_phase_status(label: &str, ok: bool) {
    let symbol = if ok { "✓" } else { "✗" };
    println!("{label:.<32} {symbol}");
}

fn print_warning_section(label: &str, warnings: &[String]) {
    if warnings.is_empty() {
        return;
    }

    println!("  {label}:");
    for warning in warnings {
        println!("    ⚠ {warning}");
    }
}

fn print_semantic_result(label: &str, errors: &[String]) {
    if errors.is_empty() {
        println!("  ✓ {label}");
        return;
    }

    for error in errors {
        println!("  ✗ {error}");
    }
}

fn legacy_layout_warning(config: &RokoConfig) -> Option<String> {
    // Only warn for genuinely legacy (v0/v1) configs.  A config_version >= 2
    // config with an empty [providers] table is perfectly valid — the user may
    // rely on CLI-based agents or env-var providers.
    if config.config_version <= 1 {
        if !config.providers.is_empty() {
            return Some(
                "roko.toml uses config version 1; run `roko config migrate` to upgrade".to_string(),
            );
        }
        return Some(
            "roko.toml uses config version 1 (no [providers] section)\n  hint: run `roko config migrate` to upgrade"
                .to_string(),
        );
    }

    None
}

/// Resolve a parsed `roko.toml` the way every command loads it: global config
/// merge, env overrides, interpolation, file secrets and the loader's
/// cross-section invariants.
fn load_like_commands(path: &Path, config: RokoConfig) -> Result<RokoConfig> {
    let effective = roko_core::config::loader::resolve_config_source(
        config,
        path,
        &roko_core::config::loader::LoadOptions::default(),
    )
    .map_err(|err| anyhow!("{err}"))?;
    // Commands then build their CLI config from it (`load_resolved_config`),
    // which validates the dream schedule and the daimon strategy space.
    crate::config::Config::from_roko_config(&effective)?;
    Ok(effective)
}

/// The invariant warnings the loader logs for a resolved config.
fn loader_warnings(config: &RokoConfig) -> Vec<String> {
    roko_core::config::validate_invariants(config)
        .into_iter()
        .filter(|result| result.severity == roko_core::config::InvariantSeverity::Warning)
        .map(|result| format!("{} (invariant {})", result.message, result.invariant_id))
        .collect()
}

/// The errors of the semantic phase of `roko config validate`.
///
/// They need no network access; the reachability probes only ever warn.
fn semantic_errors(config: &RokoConfig) -> Vec<String> {
    let mut providers = config.providers.iter().collect::<Vec<_>>();
    providers.sort_by(|a, b| a.0.cmp(b.0));
    let mut errors = providers
        .into_iter()
        .filter(|(_, provider)| {
            provider
                .api_key_env
                .as_deref()
                .is_some_and(|env_name| env_name.trim().is_empty())
        })
        .map(|(name, _)| format!("Provider '{name}' has an empty api_key_env value"))
        .collect::<Vec<_>>();
    errors.extend(
        roko_core::config::validate_provider_semantics(config)
            .into_iter()
            .filter(|finding| finding.severity == roko_core::config::InvariantSeverity::Error)
            .map(|finding| format!("[{}] {}", finding.code, finding.message)),
    );
    errors
}

async fn semantic_validate_config(
    config: &RokoConfig,
    client: &reqwest::Client,
) -> SemanticValidationReport {
    let mut report = SemanticValidationReport::default();
    // Use effective_providers for model-reference validation (includes synthesized),
    // but iterate config.providers for base_url reachability probes (synthesized
    // providers have hardcoded URLs that shouldn't be probed).
    let providers = config.effective_providers();
    let models = config.effective_models();

    let mut model_entries = config.models.iter().collect::<Vec<_>>();
    model_entries.sort_by(|a, b| a.0.cmp(b.0));
    for (model_key, profile) in model_entries {
        let provider_name = profile.provider.trim();
        if provider_name.is_empty() {
            report.schema_warnings.push(format!(
                "Model '{model_key}' has an empty provider reference"
            ));
            continue;
        }
        if !providers.contains_key(provider_name) {
            report.schema_warnings.push(format!(
                "Model '{model_key}' references missing provider '{provider_name}'"
            ));
        }
    }

    // A model reference resolves to a `[models]` key or a builtin model, as at
    // runtime; only a name that neither knows is missing.
    let resolves = |name: &str| {
        models.contains_key(name)
            || roko_core::config::model_registry::builtin_model(name).is_some()
    };

    let default_model = config.agent.default_model.trim();
    if !default_model.is_empty() && !resolves(default_model) {
        report.schema_warnings.push(format!(
            "agent.default_model references missing model '{default_model}'"
        ));
    }

    if let Some(fallback_model) = config.agent.fallback_model.as_deref() {
        let fallback_model = fallback_model.trim();
        if fallback_model.is_empty() {
            report
                .schema_warnings
                .push("agent.fallback_model must not be empty".to_string());
        } else if !resolves(fallback_model) {
            report.schema_warnings.push(format!(
                "agent.fallback_model references missing model '{fallback_model}'"
            ));
        }
    }

    let mut tier_models = config.agent.tier_models.iter().collect::<Vec<_>>();
    tier_models.sort_by(|a, b| a.0.cmp(b.0));
    for (tier, model) in tier_models {
        if tier.trim().is_empty() {
            report
                .schema_warnings
                .push("agent.tier_models contains an empty tier name".to_string());
        }
        if model.trim().is_empty() {
            report
                .schema_warnings
                .push(format!("agent.tier_models.{tier} must not be empty"));
        } else if !resolves(model.trim()) {
            report.schema_warnings.push(format!(
                "agent.tier_models.{tier} references missing model '{}'",
                model.trim()
            ));
        }
    }

    // Only probe user-declared providers (not synthesized ones with hardcoded URLs).
    let mut provider_entries = config.providers.iter().collect::<Vec<_>>();
    provider_entries.sort_by(|a, b| a.0.cmp(b.0));
    let mut unreachable_providers = BTreeSet::new();

    for (provider_name, provider) in provider_entries {
        if let Some(env_name) = provider
            .api_key_env
            .as_deref()
            .map(str::trim)
            .filter(|env_name| !env_name.is_empty())
        {
            let is_set = std::env::var(env_name)
                .ok()
                .is_some_and(|value| !value.trim().is_empty());
            if !is_set {
                // Demoted to warning: a provider with a missing key is
                // simply skipped at runtime. Only providers actually used
                // in routing cause failures, and that check happens at
                // dispatch time, not during static config validation.
                report.schema_warnings.push(format!(
                    "Provider '{provider_name}' env var '{env_name}' is not set \
                     (provider will be unavailable at runtime)"
                ));
            }
        }

        if let Some(base_url) = provider
            .base_url
            .as_deref()
            .map(str::trim)
            .filter(|base_url| !base_url.is_empty())
            && let Some(warning) = probe_validation_base_url(client, provider_name, base_url).await
        {
            unreachable_providers.insert(provider_name.to_string());
            report.field_warnings.push(warning);
        }
    }

    if !unreachable_providers.is_empty() {
        let mut model_entries = config.models.iter().collect::<Vec<_>>();
        model_entries.sort_by(|a, b| a.0.cmp(b.0));
        for (model_key, profile) in model_entries {
            if unreachable_providers.contains(profile.provider.trim()) {
                report.field_warnings.push(format!(
                    "Model '{model_key}' references provider '{}' which is unreachable",
                    profile.provider.trim()
                ));
            }
        }
    }

    // -- Harness-aware provider-kind checks (Hermes / OpenClaw) --
    for (provider_name, provider) in &config.providers {
        let has_base_url = provider
            .base_url
            .as_deref()
            .map(str::trim)
            .is_some_and(|u| !u.is_empty());
        let has_command = provider
            .command
            .as_deref()
            .map(str::trim)
            .is_some_and(|c| !c.is_empty());
        let args_contain_acp = provider
            .args
            .as_deref()
            .unwrap_or_default()
            .iter()
            .any(|a| a.trim() == "acp");

        match provider.kind {
            ProviderKind::Hermes => {
                if !has_base_url && !has_command {
                    report.field_warnings.push(format!(
                        "Provider '{provider_name}' (Hermes): no base_url or command set; \
                         will use default \"hermes\" binary on PATH"
                    ));
                }
                if has_command {
                    let cmd = provider.command.as_deref().unwrap_or_default().trim();
                    if !cmd.is_empty() && !command_exists_on_path(cmd) {
                        report.field_warnings.push(format!(
                            "Provider '{provider_name}' (Hermes): command \"{cmd}\" \
                             not found on PATH"
                        ));
                    }
                }
                if args_contain_acp && has_base_url {
                    report.field_warnings.push(format!(
                        "Provider '{provider_name}' (Hermes): args contain \"acp\" but \
                         base_url is also set; base_url takes precedence for HTTP transport"
                    ));
                }
            }
            ProviderKind::OpenClaw => {
                if !has_command {
                    report.field_warnings.push(format!(
                        "Provider '{provider_name}' (OpenClaw): no command set; \
                         will use default \"openclaw\" binary on PATH"
                    ));
                } else {
                    let cmd = provider.command.as_deref().unwrap_or_default().trim();
                    if !cmd.is_empty() && !command_exists_on_path(cmd) {
                        report.field_warnings.push(format!(
                            "Provider '{provider_name}' (OpenClaw): command \"{cmd}\" \
                             not found on PATH"
                        ));
                    }
                }
                if args_contain_acp && has_base_url {
                    report.field_warnings.push(format!(
                        "Provider '{provider_name}' (OpenClaw): args contain \"acp\" but \
                         base_url is also set; transport tier may be ambiguous"
                    ));
                }
            }
            _ => {}
        }
    }

    // Run roko-core provider/model semantic validation and merge findings.
    // Its errors come from `semantic_errors`, which config writers share.
    let semantic_findings = roko_core::config::validate_provider_semantics(config);
    for finding in semantic_findings {
        if finding.severity == roko_core::config::InvariantSeverity::Warning {
            report
                .field_warnings
                .push(format!("[{}] {}", finding.code, finding.message));
        }
    }
    report.api_key_errors = semantic_errors(config);

    report
}

async fn probe_validation_base_url(
    client: &reqwest::Client,
    provider_name: &str,
    base_url: &str,
) -> Option<String> {
    match client.head(base_url).send().await {
        Ok(_) => None,
        Err(err) if err.is_timeout() => Some(format!(
            "Provider '{provider_name}' base_url unreachable (timeout {}s)",
            VALIDATION_REACHABILITY_TIMEOUT_SECS
        )),
        Err(err) if err.is_builder() => Some(format!(
            "Provider '{provider_name}' base_url is invalid ({err})"
        )),
        Err(err) => Some(format!(
            "Provider '{provider_name}' base_url unreachable ({err})"
        )),
    }
}

/// Check whether `cmd` is resolvable on `PATH` using the system `which`.
///
/// Non-blocking in the async sense (spawns a short-lived process). Returns
/// `false` if the command is not found or if `which` itself cannot run.
fn command_exists_on_path(cmd: &str) -> bool {
    std::process::Command::new("which")
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SecretValidationTarget {
    GitHub,
    Slack,
}

impl SecretValidationTarget {
    const fn label(self) -> &'static str {
        match self {
            Self::GitHub => "GitHub API",
            Self::Slack => "Slack API",
        }
    }
}

fn secret_check_config_path(paths: &ConfigPaths) -> Result<PathBuf> {
    if let Some(path) = &paths.env_override {
        return Ok(path.clone());
    }
    if let Some(path) = &paths.project {
        return Ok(path.clone());
    }
    if let Some(ref global) = paths.global {
        if global.is_file() {
            return Ok(global.clone());
        }
    }
    Err(anyhow!("no config file found to check"))
}

fn collect_env_tokens(text: &str) -> Result<BTreeSet<String>> {
    let value: toml::Value = toml::from_str(text).context("parse config toml")?;
    let mut tokens = BTreeSet::new();
    collect_env_tokens_from_value(&value, &mut tokens);
    Ok(tokens)
}

fn collect_env_tokens_from_value(value: &toml::Value, tokens: &mut BTreeSet<String>) {
    match value {
        toml::Value::String(s) => collect_env_tokens_from_string(s, tokens),
        toml::Value::Array(items) => {
            for item in items {
                collect_env_tokens_from_value(item, tokens);
            }
        }
        toml::Value::Table(entries) => {
            for item in entries.values() {
                collect_env_tokens_from_value(item, tokens);
            }
        }
        _ => {}
    }
}

fn collect_env_tokens_from_string(input: &str, tokens: &mut BTreeSet<String>) {
    let mut rest = input;
    while let Some(start) = rest.find("${") {
        let after = &rest[start + 2..];
        let Some(end) = after.find('}') else {
            break;
        };
        let candidate = &after[..end];
        if !candidate.is_empty()
            && candidate
                .chars()
                .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
        {
            tokens.insert(candidate.to_string());
        }
        rest = &after[end + 1..];
    }
}

fn secret_validation_target(var: &str) -> Option<SecretValidationTarget> {
    match var {
        "GITHUB_TOKEN" | "GH_TOKEN" => Some(SecretValidationTarget::GitHub),
        "SLACK_BOT_TOKEN" | "SLACK_TOKEN" => Some(SecretValidationTarget::Slack),
        _ => None,
    }
}

fn validate_secret_token(
    client: &reqwest::blocking::Client,
    target: SecretValidationTarget,
    token: &str,
) -> Result<()> {
    match target {
        SecretValidationTarget::GitHub => validate_github_token(client, token),
        SecretValidationTarget::Slack => validate_slack_token(client, token),
    }
}

fn validate_github_token(client: &reqwest::blocking::Client, token: &str) -> Result<()> {
    let response = client
        .get("https://api.github.com/user")
        .bearer_auth(token)
        .send()
        .context("call GitHub API")?;
    if response.status().is_success() {
        return Ok(());
    }
    Err(anyhow!("GitHub API returned {}", response.status()))
}

fn validate_slack_token(client: &reqwest::blocking::Client, token: &str) -> Result<()> {
    let response = client
        .post("https://slack.com/api/auth.test")
        .bearer_auth(token)
        .send()
        .context("call Slack API")?;
    let status = response.status();
    let body: serde_json::Value = response.json().context("parse Slack API response")?;
    if body.get("ok").and_then(|value| value.as_bool()) == Some(true) {
        return Ok(());
    }
    let error = body
        .get("error")
        .and_then(|value| value.as_str())
        .unwrap_or("unknown error");
    Err(anyhow!("Slack API returned {status} ({error})"))
}

fn resolve_agent_command(preset: Option<String>, detected: &[DetectedCli]) -> Result<String> {
    if let Some(cmd) = preset {
        return Ok(cmd);
    }
    if detected.is_empty() {
        println!("no LLM CLIs found on PATH — defaulting to `cat` (echo smoke-test mode).");
        return Ok("cat".into());
    }
    println!("Detected agent backends:");
    for (i, d) in detected.iter().enumerate() {
        println!("  [{}] {} — {}", i + 1, d.command, d.description);
    }
    loop {
        let raw = prompt_string(
            "Pick an agent backend (number or name)",
            &detected[0].command,
        )?;
        if let Ok(idx) = raw.parse::<usize>() {
            if idx >= 1 && idx <= detected.len() {
                return Ok(detected[idx - 1].command.clone());
            }
        }
        if detected.iter().any(|d| d.command == raw) {
            return Ok(raw);
        }
        println!("  (didn't match — try again)");
    }
}

fn resolve_agent_args(
    preset: Option<Vec<String>>,
    command: &str,
    suggested: Vec<String>,
) -> Result<Vec<String>> {
    if let Some(args) = preset {
        return Ok(args);
    }
    // Special-case ollama: ask for model name.
    if command == "ollama" {
        let model = prompt_string(
            "Ollama model (e.g. llama3, codellama, qwen2.5-coder)",
            "llama3",
        )?;
        return Ok(vec!["run".into(), model]);
    }
    let suggested_str = if suggested.is_empty() {
        "(none)".to_string()
    } else {
        suggested.join(" ")
    };
    let raw = prompt_string(
        &format!("Args for `{command}` (space-separated, default: {suggested_str})"),
        &suggested.join(" "),
    )?;
    if raw.trim().is_empty() {
        return Ok(suggested);
    }
    Ok(raw.split_whitespace().map(String::from).collect())
}

fn prompt_string(label: &str, default: &str) -> Result<String> {
    print!("{label} [{default}]: ");
    io::stdout().flush().ok();
    let mut line = String::new();
    io::stdin()
        .lock()
        .read_line(&mut line)
        .context("read stdin")?;
    let trimmed = line.trim();
    Ok(if trimmed.is_empty() {
        default.into()
    } else {
        trimmed.into()
    })
}

fn prompt_usize(label: &str, default: usize) -> Result<usize> {
    loop {
        let raw = prompt_string(label, &default.to_string())?;
        match raw.parse::<usize>() {
            Ok(n) => return Ok(n),
            Err(_) => println!("  (not a number — try again)"),
        }
    }
}

fn prompt_bool(label: &str, default: bool) -> Result<bool> {
    let hint = if default { "Y/n" } else { "y/N" };
    loop {
        print!("{label} [{hint}]: ");
        io::stdout().flush().ok();
        let mut line = String::new();
        io::stdin()
            .lock()
            .read_line(&mut line)
            .context("read stdin")?;
        let trimmed = line.trim().to_ascii_lowercase();
        if trimmed.is_empty() {
            return Ok(default);
        }
        match trimmed.as_str() {
            "y" | "yes" => return Ok(true),
            "n" | "no" => return Ok(false),
            _ => println!("  (enter y/n)"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use roko_core::agent::ProviderKind;
    use roko_core::config::schema::{ModelProfile, ProviderConfig};
    use std::collections::HashMap;

    /// Helper: create an empty TOML doc to test set_toml_dotted_key.
    fn empty_doc() -> toml::Value {
        toml::Value::Table(toml::map::Map::new())
    }

    #[test]
    fn set_dotted_key_sets_agent_command() {
        let mut doc = empty_doc();
        set_toml_dotted_key(&mut doc, "agent.command", "ollama").unwrap();
        assert_eq!(doc["agent"]["command"].as_str().unwrap(), "ollama");
    }

    #[tokio::test]
    async fn validate_rejects_invalid_dream_schedule_before_semantic_checks() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("roko.toml"),
            r#"
[dreams]
scheduled_cron = "invalid cron"
"#,
        )
        .unwrap();

        let err = cmd_validate(dir.path()).await.unwrap_err();

        assert_eq!(err.to_string(), "config validation failed");
    }

    /// bug-8465a2: `config validate` passed budget tables the core loader
    /// rejected, and the loader rejected an uncapped turn beneath a finite
    /// plan cap. Validation now runs the loader, and 0 means no cap in both.
    #[tokio::test]
    async fn config_validate_matches_the_core_loader_on_budgets() {
        let loader_options = roko_core::config::loader::LoadOptions {
            merge_global: false,
            apply_env_overrides: false,
            apply_hierarchical_env: false,
            strict_validation: false,
        };
        for (budget, valid) in [
            // The old README example: plan and task caps, no turn cap.
            ("[budget]\nmax_plan_usd = 10\nmax_task_usd = 1\n", true),
            (
                "[budget]\nmax_plan_usd = 10.0\nmax_task_usd = 1.0\nmax_turn_usd = 0.5\n",
                true,
            ),
            // A turn cap above the plan cap contradicts it.
            ("[budget]\nmax_plan_usd = 1.0\nmax_turn_usd = 2.0\n", false),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("roko.toml");
            fs::write(&path, budget).unwrap();

            let loaded = roko_core::config::loader::load_config_file(&path, &loader_options);
            assert_eq!(loaded.is_ok(), valid, "loader on {budget:?}: {loaded:?}");
            let validated = cmd_validate(dir.path()).await;
            assert_eq!(
                validated.is_ok(),
                valid,
                "config validate on {budget:?}: {validated:?}"
            );
        }
    }

    /// bug-1c93b4: `config set --project agent.default_model X` wrote the
    /// legacy `agent.model` key, which validation rejects, and v2 keys such
    /// as `budget.max_plan_usd` were refused as unknown.
    #[tokio::test]
    async fn config_set_project_keeps_the_file_valid() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("roko.toml");
        crate::init::write_init_config(dir.path(), false, crate::init::InitProvider::ClaudeCli)
            .unwrap();

        let set = |key: &str, value: &str| cmd_set(dir.path(), EditTarget::Project, key, value);
        set("agent.default_model", "claude-opus-4-6").unwrap();
        set("budget.max_plan_usd", "10").unwrap();
        // A v1 name is written under its v2 name.
        set("agent.effort", "high").unwrap();

        let doc = read_toml_file(&path).unwrap();
        let agent = &doc["agent"];
        assert_eq!(agent["default_model"].as_str(), Some("claude-opus-4-6"));
        assert_eq!(agent["default_effort"].as_str(), Some("high"));
        assert!(agent.get("model").is_none(), "legacy key written");
        assert!(agent.get("effort").is_none(), "legacy key written");
        assert_eq!(doc["budget"]["max_plan_usd"].as_float(), Some(10.0));
        cmd_validate(dir.path())
            .await
            .expect("config validate accepts the edited file");

        // An edit that validation would reject is refused, and the file is
        // left as it was: a $20 turn cap cannot sit beneath a $10 plan cap.
        let before = fs::read_to_string(&path).unwrap();
        let err = set("budget.max_turn_usd", "20").unwrap_err();
        assert!(format!("{err:#}").contains("invariant 1"), "{err:#}");
        assert_eq!(fs::read_to_string(&path).unwrap(), before);
    }

    /// bug-131421: in an empty directory `roko setup --quick` skipped init,
    /// because roko's own log had already created `.roko/`, and so wrote no
    /// roko.toml; after `roko init` it wrote `providers.*.default_model`,
    /// a key that validation rejects.
    #[test]
    fn setup_quick_in_empty_dir_writes_a_valid_config() {
        let dir = tempfile::tempdir().unwrap();
        // roko's own log creates `.roko/` before setup runs.
        fs::create_dir_all(dir.path().join(".roko")).unwrap();
        assert!(crate::init::needs_init(dir.path()));

        // What `roko init` writes when neither claude nor a key is found;
        // its provider blocks are commented out.
        let unconfigured = crate::init::InitProvider::Unconfigured;
        crate::init::write_init_config(dir.path(), false, unconfigured).unwrap();
        assert!(!crate::init::needs_init(dir.path()));

        let path = dir.path().join("roko.toml");
        let clis = [("claude".to_string(), "Anthropic Claude CLI".to_string())];
        let api_keys = [("ANTHROPIC_API_KEY", "Anthropic", "anthropic_api")];
        let first = add_detected_providers(&path, &clis, &api_keys).unwrap();
        let second = add_detected_providers(&path, &clis, &api_keys).unwrap();
        assert_eq!((first, second), (2, 0));

        let doc = read_toml_file(&path).unwrap();
        let claude = &doc["providers"]["claude_cli"];
        let anthropic = &doc["providers"]["anthropic"];
        assert_eq!(claude["command"].as_str(), Some("claude"));
        assert_eq!(anthropic["kind"].as_str(), Some("anthropic_api"));
        assert!(anthropic.get("default_model").is_none());
        // The offline checks of `roko config validate`, without its probes.
        check_config_text(&path, &fs::read_to_string(&path).unwrap())
            .expect("setup leaves a roko.toml that validation accepts");
    }

    /// bug-8d7d18: `config preset`, `tune` and the TUI's config and effects
    /// saves wrote roko.toml without the check that `config set` runs. Each
    /// library writer now refuses a result that validation rejects and keeps
    /// the old file.
    #[test]
    fn every_roko_toml_writer_checks_before_writing() {
        use crate::tui::config_meta::save_pending_edits;
        use crate::tui::effects_config::{EffectsPreset, save_preset_to_root};

        fn turn_cap(usd: &str) -> HashMap<String, String> {
            HashMap::from([("budget.max_turn_usd".to_string(), usd.to_string())])
        }

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("roko.toml");
        let valid = "[budget]\nmax_plan_usd = 10.0\n";
        fs::write(&path, valid).unwrap();

        // The TUI config editor and `roko config preset`: a $20 turn cap
        // above the $10 plan cap fails config invariant 1.
        let err = save_pending_edits(dir.path(), &turn_cap("20")).unwrap_err();
        assert!(err.contains("invariant 1"), "{err}");
        assert_eq!(fs::read_to_string(&path).unwrap(), valid);
        // A valid edit still saves.
        save_pending_edits(dir.path(), &turn_cap("2")).unwrap();
        let saved = fs::read_to_string(&path).unwrap();
        assert!(saved.contains("max_turn_usd = 2.0"), "{saved}");

        // The TUI effects preset: a save into a config that fails the check
        // is refused too.
        let invalid = "[budget]\nmax_plan_usd = 1.0\nmax_turn_usd = 2.0\n";
        fs::write(&path, invalid).unwrap();
        let err = save_preset_to_root(dir.path(), EffectsPreset::Full).unwrap_err();
        assert!(err.contains("invariant 1"), "{err}");
        assert_eq!(fs::read_to_string(&path).unwrap(), invalid);
    }

    #[test]
    fn set_dotted_key_sets_prompt_budget() {
        let mut doc = empty_doc();
        set_toml_dotted_key(&mut doc, "budget.prompt_token_budget", "12345").unwrap();
        let budget = doc["budget"]["prompt_token_budget"].as_integer();
        assert_eq!(budget, Some(12_345));
    }

    #[test]
    fn set_dotted_key_sets_tools_deny() {
        let mut doc = empty_doc();
        set_toml_dotted_key(&mut doc, "tools.deny", r#"["write_file","bash"]"#).unwrap();
        let arr = doc["tools"]["deny"].as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0].as_str().unwrap(), "write_file");
        assert_eq!(arr[1].as_str().unwrap(), "bash");
    }

    #[test]
    fn set_dotted_key_rejects_unknown() {
        let mut doc = empty_doc();
        assert!(set_toml_dotted_key(&mut doc, "bogus.key", "x").is_err());
    }

    #[test]
    fn set_dotted_key_parses_args_as_json_array() {
        let mut doc = empty_doc();
        set_toml_dotted_key(&mut doc, "agent.args", r#"["run","llama3"]"#).unwrap();
        let arr = doc["agent"]["args"].as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0].as_str().unwrap(), "run");
        assert_eq!(arr[1].as_str().unwrap(), "llama3");
    }

    #[test]
    fn set_dotted_key_parses_args_as_whitespace() {
        let mut doc = empty_doc();
        set_toml_dotted_key(&mut doc, "agent.args", "run llama3").unwrap();
        let arr = doc["agent"]["args"].as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0].as_str().unwrap(), "run");
        assert_eq!(arr[1].as_str().unwrap(), "llama3");
    }

    #[test]
    fn wizard_writes_global_with_yes_flag() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("config.toml");
        let inputs = WizardInputs {
            agent_command: Some("cat".into()),
            agent_args: Some(vec![]),
            model: None,
            token_budget: Some(4000),
            role: Some("test role".into()),
            enable_gates: Some(false),
            yes: true,
        };
        let written = run_init_wizard(Some(path.clone()), &inputs).unwrap();
        assert_eq!(written, path);
        assert!(path.exists());
        // Read back the file as raw TOML and verify the wizard-set keys.
        let doc = read_toml_file(&path).unwrap();
        assert_eq!(doc["agent"]["command"].as_str().unwrap(), "cat");
        let budget = doc["budget"]["prompt_token_budget"].as_integer();
        assert_eq!(budget, Some(4000));
        assert_eq!(doc["serve"]["auth"]["enabled"].as_bool().unwrap(), false);
        assert_eq!(doc["serve"]["auth"]["api_key"].as_str().unwrap(), "");
    }

    /// bug-49a966: the wizard writes only keys of the current schema, so the
    /// file it writes passes path validation and parses strictly. It used to
    /// write v1 keys: `[executor]`, `[tools]`, `[prompt]` and `[[gate]]`.
    #[test]
    fn init_wizard_output_validates_against_the_core_schema() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("config.toml");
        let inputs = WizardInputs {
            agent_command: Some("cat".into()),
            agent_args: Some(vec!["--quiet".into()]),
            model: Some("claude-haiku-4-5".into()),
            token_budget: Some(4000),
            role: Some("test role".into()),
            enable_gates: Some(true),
            yes: true,
        };
        run_init_wizard(Some(path.clone()), &inputs).expect("run the wizard");

        let text = fs::read_to_string(&path).expect("read the written config");
        let value: toml::Value = text.parse().expect("parse the written config");
        let unknown = roko_core::config::loader::validate_known_config_paths(&value);
        assert!(unknown.is_empty(), "unknown keys: {unknown:?}\n{text}");
        let config = RokoConfig::from_toml(&text).expect("the written config parses");
        assert_eq!(config.agent.command.as_deref(), Some("cat"));
        assert_eq!(config.agent.default_model, "claude-haiku-4-5");
        assert_eq!(config.budget.prompt_token_budget, 4000);
        let rungs = config
            .gates
            .custom_rungs
            .iter()
            .map(|rung| rung.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(rungs, ["compile", "clippy"]);
    }

    #[test]
    fn set_dotted_key_sets_serve_auth() {
        let mut doc = empty_doc();
        set_toml_dotted_key(&mut doc, "serve.auth.enabled", "true").unwrap();
        set_toml_dotted_key(&mut doc, "serve.auth.api_key", "secret").unwrap();
        assert!(doc["serve"]["auth"]["enabled"].as_bool().unwrap());
        assert_eq!(doc["serve"]["auth"]["api_key"].as_str().unwrap(), "secret");
    }

    #[test]
    fn set_dotted_key_sets_serve_deploy() {
        let mut doc = empty_doc();
        set_toml_dotted_key(&mut doc, "serve.deploy.provider", "fly").unwrap();
        set_toml_dotted_key(
            &mut doc,
            "serve.deploy.environment",
            r#"["GITHUB_TOKEN", "SLACK_BOT_TOKEN"]"#,
        )
        .unwrap();
        set_toml_dotted_key(
            &mut doc,
            "serve.deploy.webhooks",
            r#"[{"provider":"github","owner":"nunchi","repo":"roko"}]"#,
        )
        .unwrap();
        assert_eq!(doc["serve"]["deploy"]["provider"].as_str().unwrap(), "fly");
        let env = doc["serve"]["deploy"]["environment"].as_array().unwrap();
        assert_eq!(env.len(), 2);
        assert_eq!(env[0].as_str().unwrap(), "GITHUB_TOKEN");
        let webhooks = doc["serve"]["deploy"]["webhooks"].as_array().unwrap();
        assert_eq!(webhooks[0]["provider"].as_str().unwrap(), "github");
        assert_eq!(webhooks[0]["owner"].as_str().unwrap(), "nunchi");
        assert_eq!(webhooks[0]["repo"].as_str().unwrap(), "roko");
    }

    #[test]
    fn collect_env_tokens_dedupes_nested_strings() {
        let text = r#"
            [agent]
            command = "runner-${GITHUB_TOKEN}"
            args = ["--flag=${SLACK_BOT_TOKEN}", "plain", "again-${GITHUB_TOKEN}"]

            [prompt]
            role = "use ${ANTHROPIC_API_KEY}"
        "#;
        let tokens = collect_env_tokens(text).unwrap();
        assert_eq!(
            tokens.into_iter().collect::<Vec<_>>(),
            vec![
                "ANTHROPIC_API_KEY".to_string(),
                "GITHUB_TOKEN".to_string(),
                "SLACK_BOT_TOKEN".to_string(),
            ]
        );
    }

    #[test]
    fn secret_validation_target_recognizes_known_tokens() {
        assert!(matches!(
            secret_validation_target("GITHUB_TOKEN"),
            Some(SecretValidationTarget::GitHub)
        ));
        assert!(matches!(
            secret_validation_target("SLACK_BOT_TOKEN"),
            Some(SecretValidationTarget::Slack)
        ));
        assert!(secret_validation_target("ANTHROPIC_API_KEY").is_none());
    }

    #[test]
    fn upsert_env_assignment_appends_new_key() {
        let rendered = upsert_env_assignment("EXISTING=1\n# comment", "NEW_SECRET", "value");
        assert_eq!(rendered, "EXISTING=1\n# comment\nNEW_SECRET=value");
    }

    #[test]
    fn upsert_env_assignment_updates_existing_key() {
        let rendered = upsert_env_assignment("NAME=old\nOTHER=keep", "NAME", "fresh");
        assert_eq!(rendered, "NAME=fresh\nOTHER=keep");
    }

    #[cfg(unix)]
    #[test]
    fn write_secret_env_file_creates_restricted_file() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".roko").join(".env");
        write_secret_env_file(&path, "TOKEN", "abc123").unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        assert_eq!(fs::read_to_string(&path).unwrap(), "TOKEN=abc123");
    }

    /// bug-524a3b: `roko config set serve.auth.api_key` wrote the key to a
    /// config file, `roko.toml` with `--project`, which agents read and the
    /// loader now refuses. The key goes to `.roko/.env` as the `ROKO__`
    /// variable that sets it, whatever the target, and leaves the readable
    /// config files; a key file keeps what it holds.
    #[test]
    fn config_set_writes_a_serve_secret_to_the_env_file() {
        let dir = tempfile::tempdir().unwrap();
        let workdir = dir.path();
        let write = |path: &Path, text: &str| {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        };
        let project = workdir.join("roko.toml");
        write(
            &project,
            "# serve settings\n[serve.auth]\nenabled = true\napi_key = \"sk-old\"\n",
        );
        let legacy = workdir.join("home/.config/roko/config.toml");
        write(&legacy, "[serve.auth]\napi_key = \"sk-old\"\n");
        let key_file = workdir.join("home/.roko/config.toml");
        write(&key_file, "[serve.auth]\napi_key = \"sk-kept\"\n");
        let opts = roko_core::config::loader::LoadOptions {
            merge_global: false,
            apply_env_overrides: false,
            apply_hierarchical_env: false,
            strict_validation: false,
        };
        let load = |path: &Path| roko_core::config::loader::load_config_file(path, &opts);
        assert!(matches!(
            load(&project),
            Err(roko_core::config::LoadConfigError::SecretInConfig { .. })
        ));

        let paths = ConfigPaths {
            global: Some(legacy.clone()),
            project: Some(project.clone()),
            env_override: Some(key_file.clone()),
        };
        let stored = set_secret_config_key(workdir, &paths, "serve.auth.api_key", "sk-new $1")
            .unwrap()
            .expect("serve.auth.api_key is a secret");
        assert_eq!(stored.variable, "ROKO__SERVE__AUTH__API_KEY");
        assert_eq!(stored.env_file, workdir.join(".roko").join(".env"));
        assert_eq!(stored.removed_from, [project.clone(), legacy.clone()]);

        // roko reads the value back from .roko/.env as it was given.
        let vars = dotenvy::from_path_iter(&stored.env_file)
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let expected = (
            "ROKO__SERVE__AUTH__API_KEY".to_string(),
            "sk-new $1".to_string(),
        );
        assert_eq!(vars, [expected]);

        // roko.toml keeps the rest as written and loads again, and a checked
        // write does not put the key back.
        assert_eq!(
            fs::read_to_string(&project).unwrap(),
            "# serve settings\n[serve.auth]\nenabled = true\n"
        );
        assert!(load(&project).unwrap().serve.auth.enabled);
        let secret_text = "[serve.auth]\napi_key = \"sk-new\"\n";
        assert!(check_config_text(&project, secret_text).is_err());
        assert!(!fs::read_to_string(&legacy).unwrap().contains("sk-old"));
        assert!(fs::read_to_string(&key_file).unwrap().contains("sk-kept"));

        // No secret: an empty value, a reference, or another field.
        for (key, value) in [
            ("serve.auth.api_key", ""),
            ("serve.auth.api_key", "${SERVE_KEY}"),
            ("agent.default_model", "sk-looking-model"),
            ("budget.prompt_token_budget", "5000"),
        ] {
            let outcome = set_secret_config_key(workdir, &paths, key, value).unwrap();
            assert_eq!(outcome, None, "{key} = {value}");
        }

        // A secret that no variable can set is refused and written nowhere.
        let header = r#"{"Authorization": "Bearer sk-header"}"#;
        let error = set_secret_config_key(workdir, &paths, "providers.x.extra_headers", header)
            .expect_err("a provider header");
        assert!(format!("{error:#}").contains("${VAR}"), "{error:#}");
        let env_text = fs::read_to_string(&stored.env_file).unwrap();
        assert!(!env_text.contains("sk-header"), "{env_text}");
    }

    #[test]
    fn build_config_migration_plan_synthesizes_supported_legacy_claude_config() {
        let text = r#"
[agent]
command = "claude"
args = ["--print", "--output-format", "stream-json"]
model = "claude-sonnet-4-6"
timeout_ms = 300000

[agent.tier_models]
mechanical = "claude-haiku-4-5"
"#;

        let plan = build_config_migration_plan(text).unwrap();
        let ConfigMigrationPlan::Legacy(plan) = plan else {
            panic!("expected legacy migration plan");
        };

        assert_eq!(plan.provider_name, "claude_cli");
        assert_eq!(plan.provider.kind, ProviderKind::ClaudeCli);
        assert_eq!(plan.provider.command.as_deref(), Some("claude"));
        assert_eq!(
            plan.provider.args.as_deref(),
            Some(
                &[
                    "--print".to_string(),
                    "--output-format".to_string(),
                    "stream-json".to_string(),
                ][..]
            )
        );
        assert_eq!(plan.provider.timeout_ms, Some(300_000));
        assert_eq!(plan.models.len(), 2);
        assert_eq!(plan.models[0].0, "claude-haiku-4-5");
        assert_eq!(plan.models[0].1.provider, "claude_cli");
        assert_eq!(plan.models[0].1.tool_format, "anthropic_blocks");
        assert_eq!(plan.models[1].0, "claude-sonnet-4-6");
        assert_eq!(plan.models[1].1.provider, "claude_cli");

        let migrated = RokoConfig::from_toml(&plan.rendered).unwrap();
        assert_eq!(migrated.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(migrated.providers.len(), 1);
        assert_eq!(migrated.models.len(), 2);

        let legacy = Config::parse_toml(&plan.rendered).unwrap();
        assert_eq!(legacy.agent.command, "claude");
        assert_eq!(
            legacy.agent.args,
            vec![
                "--print".to_string(),
                "--output-format".to_string(),
                "stream-json".to_string(),
            ]
        );
        assert_eq!(legacy.agent.model.as_deref(), Some("claude-sonnet-4-6"));
    }

    #[test]
    fn build_config_migration_plan_rejects_unsupported_legacy_backend() {
        let text = r#"
[agent]
command = "mods"
model = "gpt-5"
"#;

        let err = build_config_migration_plan(text).unwrap_err();
        assert!(
            err.to_string()
                .contains("legacy agent.command 'mods' cannot be migrated safely")
        );
    }

    #[test]
    fn build_config_migration_plan_noops_for_current_provider_registry() {
        let text = r#"
schema_version = 2

[agent]
command = "claude"
model = "claude-sonnet-4-6"

[providers.claude_cli]
kind = "claude_cli"
command = "claude"
"#;

        let plan = build_config_migration_plan(text).unwrap();
        assert!(matches!(plan, ConfigMigrationPlan::AlreadyCurrent));
    }

    #[tokio::test]
    async fn semantic_validate_reports_missing_provider_reference() {
        let client = reqwest::Client::builder().build().unwrap();
        let mut config = RokoConfig::default();
        config.agent.default_model = "kimi-k2-5".to_string();
        config.models.insert(
            "kimi-k2-5".to_string(),
            ModelProfile {
                provider: "moonshot".to_string(),
                slug: "kimi-k2.5".to_string(),
                context_window: 256_000,
                max_output: None,
                supports_tools: true,
                supports_thinking: false,
                supports_vision: false,
                supports_web_search: false,
                supports_mcp_tools: false,
                supports_partial: false,
                provider_routing: None,
                tool_format: "openai_json".to_string(),
                cost_input_per_m: None,
                cost_output_per_m: None,
                cost_cache_read_per_m: None,
                cost_cache_write_per_m: None,
                max_tools: None,
                tokenizer_ratio: None,
                ..Default::default()
            },
        );

        let report = semantic_validate_config(&config, &client).await;
        // The CLI-level check produces 1 schema_warning; the core
        // `validate_provider_semantics` also fires model.unknown_provider as
        // an Error, so we expect 1 error + 1 schema_warning.
        assert_eq!(report.error_count(), 1);
        assert_eq!(report.schema_warning_count(), 1);
        assert_eq!(report.field_warning_count(), 0);
        assert_eq!(
            report.schema_warnings,
            vec!["Model 'kimi-k2-5' references missing provider 'moonshot'".to_string()]
        );
    }

    #[tokio::test]
    async fn semantic_validate_reports_missing_api_key_env_var() {
        let client = reqwest::Client::builder().build().unwrap();
        let env_name = "ROKO_TEST_VALIDATE_API_KEY_THAT_SHOULD_NOT_EXIST";

        let mut config = RokoConfig::default();
        config.agent.default_model.clear();
        config.providers.insert(
            "moonshot".to_string(),
            ProviderConfig {
                kind: ProviderKind::OpenAiCompat,
                base_url: None,
                api_key_env: Some(env_name.to_string()),
                command: None,
                args: None,
                timeout_ms: None,
                ttft_timeout_ms: None,
                connect_timeout_ms: None,
                extra_headers: None,
                max_concurrent: None,
                limits: None,
                require_confirmation: false,
                stream_usage: None,
            },
        );

        let report = semantic_validate_config(&config, &client).await;

        // Missing API key is now a warning (not an error): the provider is
        // simply unavailable at runtime rather than making config invalid.
        assert_eq!(report.error_count(), 0);
        assert_eq!(report.schema_warning_count(), 1);
        assert!(
            report.schema_warnings[0].contains("moonshot"),
            "expected moonshot in warning: {:?}",
            report.schema_warnings
        );
        assert!(
            report.schema_warnings[0].contains(env_name),
            "expected env var name in warning: {:?}",
            report.schema_warnings
        );
    }

    #[tokio::test]
    async fn semantic_validate_reports_missing_fallback_model_reference() {
        let client = reqwest::Client::builder().build().unwrap();
        let mut config = RokoConfig::default();
        config.agent.default_model.clear();
        config.agent.fallback_model = Some("missing-model".to_string());

        let report = semantic_validate_config(&config, &client).await;

        // The CLI-level check produces 1 schema_warning; the core
        // `validate_provider_semantics` also fires routing.unresolved_model
        // as an Error for agent.fallback_model.
        assert_eq!(report.error_count(), 1);
        assert_eq!(report.schema_warning_count(), 1);
        assert_eq!(report.field_warning_count(), 0);
        assert_eq!(
            report.schema_warnings,
            vec!["agent.fallback_model references missing model 'missing-model'".to_string()]
        );
    }

    #[tokio::test]
    async fn semantic_validate_warns_for_legacy_default_tier_and_fallback_models() {
        let client = reqwest::Client::builder().build().unwrap();
        let mut config = RokoConfig::default();
        config.models.clear();
        // Names that neither `[models]` nor the builtin models know.
        config.agent.default_model = "no-default".to_string();
        config
            .agent
            .tier_models
            .insert("mechanical".to_string(), "no-tier-model".to_string());
        config.agent.fallback_model = Some("no-fallback".to_string());

        let report = semantic_validate_config(&config, &client).await;

        assert!(
            report
                .schema_warnings
                .contains(&"agent.default_model references missing model 'no-default'".to_string())
        );
        assert!(
            report.schema_warnings.contains(
                &"agent.fallback_model references missing model 'no-fallback'".to_string()
            )
        );
        assert!(report.schema_warnings.contains(
            &"agent.tier_models.mechanical references missing model 'no-tier-model'".to_string()
        ));
    }

    /// bug-c1950e: a builtin model needs no `[models]` entry, yet validation
    /// warned that it was missing.
    #[tokio::test]
    async fn validate_accepts_a_builtin_default_model() {
        let client = reqwest::Client::builder().build().unwrap();
        let mut config = RokoConfig::default();
        config.models.clear();
        config.agent.default_model = "claude-sonnet-4-6".to_string();
        config.agent.fallback_model = Some("claude-haiku-4-5".to_string());
        // A builtin alias resolves too.
        config
            .agent
            .tier_models
            .insert("mechanical".to_string(), "haiku".to_string());

        let report = semantic_validate_config(&config, &client).await;
        let model_warnings = report
            .schema_warnings
            .iter()
            .filter(|warning| warning.contains("references missing model"))
            .collect::<Vec<_>>();
        assert!(model_warnings.is_empty(), "{model_warnings:?}");

        // A name that neither `[models]` nor the builtins know still warns.
        config.agent.default_model = "not-a-model".to_string();
        let report = semantic_validate_config(&config, &client).await;
        let missing = "agent.default_model references missing model 'not-a-model'".to_string();
        assert!(
            report.schema_warnings.contains(&missing),
            "{:?}",
            report.schema_warnings
        );
    }

    #[tokio::test]
    async fn semantic_validate_warns_when_provider_is_unreachable() {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(100))
            .build()
            .unwrap();
        let mut config = RokoConfig::default();
        config.agent.default_model = "kimi-k2-5".to_string();
        config.providers = indexmap::IndexMap::from([(
            "moonshot".to_string(),
            ProviderConfig {
                kind: ProviderKind::OpenAiCompat,
                base_url: Some("http://127.0.0.1:9".to_string()),
                api_key_env: None,
                command: None,
                args: None,
                timeout_ms: None,
                ttft_timeout_ms: None,
                connect_timeout_ms: None,
                extra_headers: None,
                max_concurrent: None,
                limits: None,
                require_confirmation: false,
                stream_usage: None,
            },
        )]);
        config.models.insert(
            "kimi-k2-5".to_string(),
            ModelProfile {
                provider: "moonshot".to_string(),
                slug: "kimi-k2.5".to_string(),
                context_window: 256_000,
                max_output: None,
                supports_tools: true,
                supports_thinking: false,
                supports_vision: false,
                supports_web_search: false,
                supports_mcp_tools: false,
                supports_partial: false,
                provider_routing: None,
                tool_format: "openai_json".to_string(),
                cost_input_per_m: None,
                cost_output_per_m: None,
                cost_cache_read_per_m: None,
                cost_cache_write_per_m: None,
                max_tools: None,
                tokenizer_ratio: None,
                ..Default::default()
            },
        );

        let report = semantic_validate_config(&config, &client).await;

        assert_eq!(report.error_count(), 0);
        assert_eq!(report.warning_count(), 2);
        assert_eq!(report.schema_warning_count(), 0);
        assert_eq!(report.field_warning_count(), 2);
        assert!(
            report
                .field_warnings
                .iter()
                .any(|warning| warning.contains("Provider 'moonshot' base_url unreachable"))
        );
        assert_eq!(
            report.field_warnings[1],
            "Model 'kimi-k2-5' references provider 'moonshot' which is unreachable"
        );
    }

    #[test]
    fn legacy_layout_warning_reports_version_one_only() {
        let mut legacy = RokoConfig::default();
        legacy.config_version = 1;
        assert_eq!(
            legacy_layout_warning(&legacy).as_deref(),
            Some(
                "roko.toml uses config version 1 (no [providers] section)\n  hint: run `roko config migrate` to upgrade"
            )
        );

        // A v2 config with empty providers should NOT warn — empty providers
        // is a valid configuration (user may rely on CLI agents or env vars).
        let current = RokoConfig::default();
        assert_eq!(legacy_layout_warning(&current).as_deref(), None);
    }

    /// Proves that `cmd_show_effective` uses the redacted serialization path
    /// so that interpolated secrets never appear in stdout.
    #[test]
    fn config_show_effective_redacts_interpolated_secret() {
        // Build a config with a secret-bearing provider, exactly as the
        // loader would produce after `interpolate_env_vars` + `resolve_file_secrets`.
        let mut config = RokoConfig::default();
        let mut headers = HashMap::new();
        headers.insert(
            "authorization".to_string(),
            "Bearer sk-live-INTERPOLATED".to_string(),
        );
        config.providers.insert(
            "acme".into(),
            ProviderConfig {
                kind: ProviderKind::OpenAiCompat,
                base_url: Some("https://acme.test/v1".into()),
                api_key_env: Some("ACME_API_KEY".into()),
                command: None,
                args: None,
                timeout_ms: None,
                ttft_timeout_ms: None,
                connect_timeout_ms: None,
                extra_headers: Some(headers),
                max_concurrent: None,
                limits: None,
                require_confirmation: false,
                stream_usage: None,
            },
        );

        // Serialize through the same redacted path used by cmd_show_effective.
        let output = roko_core::config::loader::serialize_effective_redacted(&config)
            .expect("redacted serialization");

        // The literal secret values must NOT appear in the output.
        assert!(
            !output.contains("sk-live-INTERPOLATED"),
            "interpolated header secret leaked in effective config output"
        );
        assert!(
            output.contains("ACME_API_KEY"),
            "api_key_env metadata was incorrectly redacted"
        );

        // The redaction marker must be present.
        assert!(
            output.contains(roko_core::config::loader::REDACTED_MARKER),
            "redaction marker absent from effective config output"
        );

        // Non-secret fields (base_url) must remain visible.
        assert!(
            output.contains("https://acme.test/v1"),
            "base_url was incorrectly redacted"
        );
    }

    #[test]
    fn config_show_section_prints_only_that_section() {
        let text = r#"
schema_version = 2

[serve.auth]
enabled = true
api_key = "roko-secret-api-key"

[dreams]
auto_dream = true
"#;
        let config = RokoConfig::from_toml(text).expect("parse config");
        let render = |section: &str| render_config_section(&config, section).unwrap();

        let dreams = render("dreams");
        assert!(dreams.starts_with("[dreams]\n"), "{dreams}");
        assert!(dreams.contains("auto_dream = true"), "{dreams}");
        assert!(!dreams.contains("[serve"), "{dreams}");

        let auth = render("serve.auth");
        assert!(auth.starts_with("[serve.auth]\n"), "{auth}");
        assert!(!auth.contains("roko-secret-api-key"), "{auth}");

        assert_eq!(render("dreams.auto_dream"), "[dreams]\nauto_dream = true\n");
    }

    #[test]
    fn config_show_section_names_the_keys_of_an_unknown_section() {
        let config = RokoConfig::default();
        let error = render_config_section(&config, "agnet").unwrap_err();
        let message = error.to_string();
        assert!(message.contains("`agnet`"), "{message}");
        assert!(message.contains("agent"), "{message}");
    }
}
