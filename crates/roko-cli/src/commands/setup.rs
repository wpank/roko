//! `roko setup` — interactive workspace bootstrap wizard.
//!
//! Three paths:
//!
//! - Interactive (default): step-by-step with prompts.
//! - `--yes`: non-interactive, use first available provider.
//! - `--quick`: detect everything, write config, print summary — no prompts.
//!
//! Steps (interactive/yes paths):
//! 1. Detect available providers via `detect_auth_from_env()`
//! 2. If `NeedsSetup`: prompt for API key (or print instructions)
//! 3. Auto-select default model based on available provider
//! 4. Run `roko init` if `roko.toml` doesn't exist
//! 5. Run `roko doctor` to verify
//! 6. Print "next steps" message

use anyhow::Result;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use crate::Cli;
use roko_cli::auth_detect::{AuthMethod, detect_auth_from_env, version_probe};
use roko_cli::doctor::{CredentialProbe, DoctorOptions, run_doctor};
use roko_core::child_env::CredentialScrub;
use roko_core::provider_catalog::{ProviderAvailability, catalog, check_provider_availability};

use super::util::cmd_init;

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Run the setup wizard.
///
/// - `yes` — skip interactive prompts, accept all defaults.
/// - `quick` — detect every available provider, write all of them to
///   `roko.toml`, print a summary, skip doctor and prompts entirely.
pub(crate) async fn cmd_setup(
    cli: &Cli,
    workdir: Option<PathBuf>,
    yes: bool,
    quick: bool,
) -> Result<i32> {
    let workdir =
        workdir.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    if quick {
        return cmd_setup_quick(&workdir).await;
    }

    cmd_setup_interactive(cli, &workdir, yes).await
}

// ---------------------------------------------------------------------------
// --quick path
// ---------------------------------------------------------------------------

/// Detect everything, write config, print summary — no prompts.
async fn cmd_setup_quick(workdir: &std::path::Path) -> Result<i32> {
    println!("roko setup --quick");
    println!("==================\n");

    // ── Step 1: Detect CLIs ─────────────────────────────────────────────
    println!("[1/4] Detecting installed LLM CLIs...");
    let clis = detect_installed_clis();
    if clis.is_empty() {
        println!("  No LLM CLIs found on PATH.");
    } else {
        for (cmd, desc) in &clis {
            println!("  + {cmd}  ({desc})");
        }
    }

    // ── Step 2: Detect API keys ─────────────────────────────────────────
    println!("\n[2/4] Scanning environment for API keys...");
    let found_keys = detect_all_api_keys();
    let missing_keys = detect_missing_api_keys();

    if found_keys.is_empty() && clis.is_empty() {
        println!("  No providers detected.\n");
        print_provider_instructions();
        return Ok(1);
    }

    for (env_var, display_name, _kind) in &found_keys {
        println!("  + {env_var}  ({display_name})");
    }
    for (env_var, display_name, url) in &missing_keys {
        println!("  - {env_var} not set  ({display_name})  — get key: {url}");
    }

    // ── Step 3: Init + write config ─────────────────────────────────────
    println!("\n[3/4] Initializing workspace...");
    if roko_cli::init::needs_init(workdir) {
        cmd_init(Some(workdir.to_path_buf()), false, None, false).await?;
        println!("  Created .roko/ and roko.toml");
    } else {
        println!("  roko.toml already exists, skipping init.");
    }

    let toml_path = workdir.join("roko.toml");
    let added = roko_cli::config_cmd::add_detected_providers(&toml_path, &clis, &found_keys)?;
    if added == 0 {
        println!("  roko.toml already contains all detected providers.");
    } else {
        println!("  Wrote {added} provider(s) to roko.toml.");
    }

    // ── Step 4: Summary ─────────────────────────────────────────────────
    println!("\n[4/4] Summary:");
    let recommended = recommend_provider(&clis, &found_keys);
    if let Some(rec) = &recommended {
        println!("  Recommended provider: {rec}");
    }

    let total = clis.len() + found_keys.len();
    if total == 0 {
        println!("  No providers configured — set an API key and re-run.");
        println!();
        print_provider_instructions();
        return Ok(1);
    }

    println!("  {total} provider(s) available.");
    println!();
    println!("  Next:");
    println!("    roko run \"hello world\"             Run a quick test");
    println!("    roko run --plan \"describe a task\"  Plan + execute a task");
    println!("    roko doctor                         Full diagnostics");
    println!("    roko config models list             See all available models");

    Ok(0)
}

// ---------------------------------------------------------------------------
// Interactive / --yes path
// ---------------------------------------------------------------------------

async fn cmd_setup_interactive(cli: &Cli, workdir: &std::path::Path, yes: bool) -> Result<i32> {
    println!("roko setup");
    println!("==========\n");

    // ── Step 1: Detect auth ─────────────────────────────────────────────
    println!("[1/5] Detecting available LLM providers...");

    // Show all detected and missing providers before deciding on the primary auth.
    let found_keys = detect_all_api_keys();
    let clis = detect_installed_clis();

    if !clis.is_empty() || !found_keys.is_empty() {
        for (cmd, desc) in &clis {
            println!("  + {cmd} CLI  ({desc})");
        }
        for (env_var, display_name, _) in &found_keys {
            println!("  + {env_var}  ({display_name})");
        }
    }

    let missing_keys = detect_missing_api_keys();
    if !missing_keys.is_empty() && !yes {
        println!("\n  The following providers are not configured:");
        for (env_var, display_name, url) in &missing_keys {
            println!("    {env_var} — {display_name}");
            println!("      Get a key at: {url}");
        }
    }

    let auth = detect_auth_from_env();
    let auth = match &auth {
        AuthMethod::NeedsSetup => {
            if yes {
                tracing::error!("no provider found; set an API key env var and re-run");
                return Ok(1);
            }
            // Interactive: prompt for API key
            prompt_for_api_key()?
        }
        other => {
            println!("\n  Primary provider: {}", other.label());
            other.clone()
        }
    };

    // ── Step 2: Show selected model ─────────────────────────────────────
    let model = default_model_for_auth(&auth);
    println!("\n[2/5] Default model: {model}");

    // ── Step 3: Init if needed ──────────────────────────────────────────
    if roko_cli::init::needs_init(workdir) {
        println!("\n[3/5] Initializing workspace...");
        cmd_init(Some(workdir.to_path_buf()), false, None, false).await?;
        println!("  Created .roko/ and roko.toml");
    } else {
        println!("\n[3/5] Workspace already initialized (roko.toml exists)");
    }

    // ── Step 3b: Offer to write provider entries for detected API keys ──
    let toml_path = workdir.join("roko.toml");
    if toml_path.is_file() {
        let detected_providers = detected_api_key_providers();
        if !detected_providers.is_empty() {
            let toml_contents = std::fs::read_to_string(&toml_path).unwrap_or_default();
            for (env_var, name, kind) in &detected_providers {
                let section_header = format!("[providers.{name}]");
                if toml_contents.contains(&section_header) {
                    continue;
                }
                if yes {
                    // Auto-add in non-interactive mode.
                    let entry = format!(
                        "\n{section_header}\nkind = \"{kind}\"\napi_key_env = \"{env_var}\"\n"
                    );
                    roko_cli::config_cmd::append_checked_config(&toml_path, &entry)?;
                    println!("  Added [providers.{name}] to roko.toml (detected {env_var})");
                } else {
                    print!("  Detected {env_var}. Add [providers.{name}] to roko.toml? [Y/n] ");
                    io::stdout().flush()?;
                    let stdin = io::stdin();
                    let line = stdin.lock().lines().next().unwrap_or(Ok(String::new()))?;
                    let answer = line.trim().to_lowercase();
                    if answer.is_empty() || answer == "y" || answer == "yes" {
                        let entry = format!(
                            "\n{section_header}\nkind = \"{kind}\"\napi_key_env = \"{env_var}\"\n"
                        );
                        roko_cli::config_cmd::append_checked_config(&toml_path, &entry)?;
                        println!("  Added [providers.{name}] to roko.toml");
                    }
                }
            }
        }
    }

    // ── Step 4: Doctor ──────────────────────────────────────────────────
    println!("\n[4/5] Running diagnostics...");
    let report = run_doctor(&DoctorOptions {
        workdir: workdir.to_path_buf(),
        config_override: cli.config.clone(),
        serve_url: None,
        credentials: CredentialProbe::Environment,
    })
    .await?;

    if report.healthy {
        println!("  All checks passed.");
    } else {
        println!("  Some checks need attention:");
        for check in &report.checks {
            if check.status == roko_cli::doctor::DoctorStatus::Fail {
                println!("    [fail] {}: {}", check.id, check.message);
                if let Some(fix) = &check.fix {
                    println!("           fix: {fix}");
                }
            }
        }
    }

    // ── Step 5: Next steps ──────────────────────────────────────────────
    println!("\n[5/5] Next steps:");
    println!("  roko run --plan \"describe your task\" Plan and execute a task end-to-end");
    println!("  roko \"describe your task\"           Run a one-shot task");
    println!("  roko models list                    See available models and routing");
    println!("  roko doctor                         Re-run diagnostics anytime");
    println!("  roko status                         Check workspace health");
    if matches!(auth, AuthMethod::NeedsSetup) {
        println!("\n  (Set an LLM provider key to enable agent dispatch)");
    }

    Ok(0)
}

// ---------------------------------------------------------------------------
// Provider detection helpers
// ---------------------------------------------------------------------------

/// Detect installed LLM CLI tools on PATH.
///
/// Returns `(command_name, human_description)` pairs.
fn detect_installed_clis() -> Vec<(String, String)> {
    let candidates: &[(&str, &str)] = &[
        ("claude", "Anthropic Claude CLI"),
        ("codex", "OpenAI Codex CLI"),
        ("ollama", "Ollama — local models"),
        ("mods", "charmbracelet/mods"),
        ("llm", "simonw/llm"),
        ("aichat", "aichat"),
    ];

    // A probed binary inherits no provider key, `.env`-loaded name or roko
    // credential.
    let scrub = CredentialScrub::default();
    candidates
        .iter()
        .filter(|(cmd, _)| {
            version_probe(cmd, &scrub)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        })
        .map(|(cmd, desc)| ((*cmd).to_string(), (*desc).to_string()))
        .collect()
}

/// Return all API key providers detected in the environment.
///
/// Returns `(env_var, display_name, kind_label)` triples.
fn detect_all_api_keys() -> Vec<(&'static str, &'static str, &'static str)> {
    catalog()
        .iter()
        .filter_map(|entry| {
            if entry.api_key_env.is_empty() {
                return None;
            }
            if check_provider_availability(entry) == ProviderAvailability::KeyFound {
                Some((entry.api_key_env, entry.display_name, entry.kind.label()))
            } else {
                None
            }
        })
        .collect()
}

/// Return all API key providers NOT detected in the environment.
///
/// Returns `(env_var, display_name, signup_url)` triples.
fn detect_missing_api_keys() -> Vec<(&'static str, &'static str, &'static str)> {
    // Map provider ids to signup URLs.
    let signup_urls: &[(&str, &str)] = &[
        ("anthropic", "https://console.anthropic.com/"),
        ("openai", "https://platform.openai.com/api-keys"),
        ("gemini", "https://aistudio.google.com/app/apikey"),
        ("deepseek", "https://platform.deepseek.com/"),
        ("cerebras", "https://cloud.cerebras.ai/"),
        ("openrouter", "https://openrouter.ai/keys"),
        ("xai", "https://console.x.ai/"),
        ("perplexity", "https://www.perplexity.ai/settings/api"),
    ];

    catalog()
        .iter()
        .filter_map(|entry| {
            if entry.api_key_env.is_empty() {
                return None; // Local providers don't need keys.
            }
            if check_provider_availability(entry) != ProviderAvailability::KeyMissing {
                return None;
            }
            let url = signup_urls
                .iter()
                .find(|(id, _)| *id == entry.id)
                .map(|(_, url)| *url)
                .unwrap_or("https://roko.dev/providers");
            Some((entry.api_key_env, entry.display_name, url))
        })
        .collect()
}

/// Return `(env_var, provider_name, kind)` for providers used in the
/// interactive path (narrower — only the ones we auto-add to toml).
fn detected_api_key_providers() -> Vec<(&'static str, &'static str, &'static str)> {
    catalog()
        .iter()
        .filter_map(|entry| {
            if entry.api_key_env.is_empty() {
                return None;
            }
            if check_provider_availability(entry) == ProviderAvailability::KeyFound {
                Some((entry.api_key_env, entry.id, entry.kind.label()))
            } else {
                None
            }
        })
        .collect()
}

/// Recommend a provider based on what's available.
///
/// Priority: claude CLI > ANTHROPIC_API_KEY > anything else with a key > none.
fn recommend_provider(
    clis: &[(String, String)],
    found_keys: &[(&'static str, &'static str, &'static str)],
) -> Option<String> {
    if clis.iter().any(|(cmd, _)| cmd == "claude") {
        return Some("claude CLI (fastest for local use, no API costs)".to_string());
    }
    if clis.iter().any(|(cmd, _)| cmd == "codex") {
        return Some("codex CLI".to_string());
    }
    if found_keys
        .iter()
        .any(|(env, _, _)| *env == "ANTHROPIC_API_KEY")
    {
        return Some("Anthropic API (claude-sonnet-4-6)".to_string());
    }
    if let Some((env, name, _)) = found_keys.first() {
        return Some(format!("{name} ({env})"));
    }
    None
}

/// Print instructions for getting API keys when no providers are detected.
fn print_provider_instructions() {
    println!("  To get started, set at least one of:");
    println!();
    println!("  Option 1 — Claude CLI (recommended, free tier available):");
    println!("    npm install -g @anthropic-ai/claude-cli");
    println!("    claude login");
    println!();
    println!("  Option 2 — Anthropic API:");
    println!("    export ANTHROPIC_API_KEY=sk-ant-...   # https://console.anthropic.com/");
    println!();
    println!("  Option 3 — OpenAI API:");
    println!("    export OPENAI_API_KEY=sk-...          # https://platform.openai.com/api-keys");
    println!();
    println!("  Option 4 — Free/cheap alternatives:");
    println!("    export GEMINI_API_KEY=...             # https://aistudio.google.com/app/apikey");
    println!("    export CEREBRAS_API_KEY=...           # https://cloud.cerebras.ai/ (free)");
    println!("    export DEEPSEEK_API_KEY=...           # https://platform.deepseek.com/");
    println!();
    println!("  After setting a key, re-run: roko setup --quick");
}

// ---------------------------------------------------------------------------
// Interactive helpers
// ---------------------------------------------------------------------------

/// Prompt the user to set up an API key interactively.
fn prompt_for_api_key() -> Result<AuthMethod> {
    println!("  No LLM provider detected.\n");
    println!("  Options:");
    println!("    1. Install Claude CLI: npm install -g @anthropic-ai/claude-cli && claude login");
    println!("    2. Set ANTHROPIC_API_KEY=sk-ant-...");
    println!("    3. Set OPENAI_API_KEY=sk-...");
    println!("    4. Set GEMINI_API_KEY=...     (free tier available)");
    println!("    5. Set CEREBRAS_API_KEY=...   (free, fast inference)");
    println!();
    print!("  Enter API key (or press Enter to skip): ");
    io::stdout().flush()?;

    let stdin = io::stdin();
    let line = stdin.lock().lines().next().unwrap_or(Ok(String::new()))?;
    let key = line.trim().to_string();

    if key.is_empty() {
        println!("  Skipped. You can set an env var later.");
        return Ok(AuthMethod::NeedsSetup);
    }

    // Guess provider from key prefix.
    if key.starts_with("sk-ant-") {
        println!("  Detected Anthropic key. Set in your shell:");
        println!("    export ANTHROPIC_API_KEY={key}");
        Ok(AuthMethod::AnthropicApi { key, model: None })
    } else {
        println!("  Detected OpenAI-compatible key. Set in your shell:");
        println!("    export OPENAI_API_KEY={key}");
        Ok(AuthMethod::OpenAiCompat {
            key,
            base_url: "https://api.openai.com/v1".to_string(),
            model: None,
        })
    }
}

/// Pick a sensible default model based on detected auth.
fn default_model_for_auth(auth: &AuthMethod) -> &'static str {
    match auth {
        AuthMethod::ClaudeCli
        | AuthMethod::CliProvider { .. }
        | AuthMethod::AnthropicApi { .. } => "claude-sonnet-4-6",
        AuthMethod::OpenAiCompat { .. } => "gpt-4.1-mini",
        AuthMethod::NeedsSetup => "claude-sonnet-4-6",
    }
}
