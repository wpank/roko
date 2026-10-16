//! `roko setup` — interactive workspace bootstrap wizard.
//!
//! Three paths:
//!
//! - Interactive (default): step-by-step with prompts.
//! - `--yes`: non-interactive, use first available provider.
//! - `--quick`: detect everything, write config, print summary — no prompts.
//!
//! Both create `~/.roko/.env`, the file roko loads API keys from before any
//! command runs, when it is missing: `roko init` writes only the workspace.
//!
//! Steps (interactive/yes paths):
//! 1. Detect available providers via `detect_auth_from_env()`
//! 2. If `NeedsSetup`: prompt for API key (or print instructions)
//! 3. Auto-select default model based on available provider
//! 4. Run `roko init` if `roko.toml` doesn't exist
//! 5. Run `roko doctor` to verify
//! 6. Print "next steps" message

use anyhow::{Context as _, Result};
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

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
    create_global_env();
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
        cmd_init(Some(workdir.to_path_buf()), false, None, None, false).await?;
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
    create_global_env();

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
        cmd_init(Some(workdir.to_path_buf()), false, None, None, false).await?;
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
        beside_serve: None,
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
    println!("  Keep keys in ~/.roko/.env, which every roko command loads, or export them.");
    println!("  After setting a key, re-run: roko setup --quick");
}

/// What `roko setup` writes to a new `~/.roko/.env`: only comments, so it
/// sets no key until the person fills one in.
const GLOBAL_ENV_TEMPLATE: &str = "\
# API keys roko loads before every command (then a workspace's .roko/.env).
# Uncomment a line and paste the key after `=`. Never put a key in roko.toml.
# `roko init` gives the models of each provider whose key is set rungs on the
# model ladder.
# OPENAI_API_KEY=
# ANTHROPIC_API_KEY=
# OPENROUTER_API_KEY=
# CEREBRAS_API_KEY=
# ZAI_API_KEY=
# MOONSHOT_API_KEY=
# GEMINI_API_KEY=
# PERPLEXITY_API_KEY=
";

/// Create `~/.roko/.env` when it is missing and say so. Every note about API
/// keys points at that file, and `roko init` writes only the workspace, so
/// the machine's setup is what creates it. A failure is only reported.
fn create_global_env() {
    let Some(home) = std::env::var_os("HOME").filter(|home| !home.is_empty()) else {
        return;
    };
    match ensure_global_env(Path::new(&home)) {
        Ok(Some(path)) => println!(
            "  Created {} for your API keys; it sets none yet",
            path.display()
        ),
        Ok(None) => {}
        Err(err) => println!("  note: could not create ~/.roko/.env: {err:#}"),
    }
}

/// Write [`GLOBAL_ENV_TEMPLATE`] to `home`'s `.roko/.env`, readable only by
/// its owner, unless the file exists. Returns its path when it was created.
fn ensure_global_env(home: &Path) -> Result<Option<PathBuf>> {
    let dir = home.join(".roko");
    let path = dir.join(".env");
    if path.exists() {
        return Ok(None);
    }
    std::fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut file = options
        .open(&path)
        .with_context(|| format!("create {}", path.display()))?;
    file.write_all(GLOBAL_ENV_TEMPLATE.as_bytes())
        .with_context(|| format!("write {}", path.display()))?;
    Ok(Some(path))
}

// ---------------------------------------------------------------------------
// Interactive helpers
// ---------------------------------------------------------------------------

/// Prompt for an API key when no provider was found, and keep it where
/// every roko command loads keys from, `~/.roko/.env` (bug-d32a50). On a
/// terminal the key is read without echo; it is never printed back, only
/// its last four characters, and it is loaded into this process so the
/// `roko init` that setup runs next gives its provider a ladder rung.
fn prompt_for_api_key() -> Result<AuthMethod> {
    let home = std::env::var_os("HOME")
        .filter(|home| !home.is_empty())
        .map(PathBuf::from);
    let stdin = io::stdin();
    let terminal = io::IsTerminal::is_terminal(&stdin);
    let (auth, saved) = prompt_for_api_key_with(
        &mut stdin.lock(),
        &mut io::stdout(),
        home.as_deref(),
        terminal,
    )?;
    if let Some((name, key)) = saved {
        // The startup loader already read ~/.roko/.env; this sets the one
        // variable, and never one the environment already has.
        dotenvy::from_read(io::Cursor::new(format!("{name}={key}\n")))
            .context("load the saved key")?;
    }
    Ok(auth)
}

/// The variables a typed key can be kept in, in the order setup offers them,
/// with the provider's name and the OpenAI-compatible endpoint of its API.
const KEY_PROVIDERS: &[(&str, &str, &str)] = &[
    ("OPENAI_API_KEY", "OpenAI", "https://api.openai.com/v1"),
    (
        "ANTHROPIC_API_KEY",
        "Anthropic",
        "https://api.anthropic.com",
    ),
    (
        "OPENROUTER_API_KEY",
        "OpenRouter",
        "https://openrouter.ai/api/v1",
    ),
    ("CEREBRAS_API_KEY", "Cerebras", "https://api.cerebras.ai/v1"),
    ("ZAI_API_KEY", "Z.ai", "https://api.z.ai/api/paas/v4"),
    ("MOONSHOT_API_KEY", "Moonshot", "https://api.moonshot.ai/v1"),
];

/// The variable a key's prefix alone identifies, if it does: `sk-` alone is
/// shared by OpenAI, Moonshot and others, so such a key is asked about.
fn key_variable_by_prefix(key: &str) -> Option<&'static str> {
    [
        ("sk-ant-", "ANTHROPIC_API_KEY"),
        ("sk-or-", "OPENROUTER_API_KEY"),
        ("sk-proj-", "OPENAI_API_KEY"),
        ("csk-", "CEREBRAS_API_KEY"),
    ]
    .into_iter()
    .find(|(prefix, _)| key.starts_with(prefix))
    .map(|(_, name)| name)
}

/// `key` as setup may show it: its last four characters, when it is long
/// enough that they give nothing away.
fn masked_key(key: &str) -> String {
    let chars: Vec<char> = key.chars().collect();
    if chars.len() < 12 {
        return "…".to_string();
    }
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("…{tail}")
}

/// [`prompt_for_api_key`] over `input` and `out`, keeping the key in
/// `home`'s `.roko/.env`. `terminal` reads the key without echo from the
/// real terminal instead of `input`. Returns the auth the key gives and, when
/// it was saved, its variable and the key. It changes no environment.
fn prompt_for_api_key_with(
    input: &mut impl BufRead,
    out: &mut impl Write,
    home: Option<&Path>,
    terminal: bool,
) -> Result<(AuthMethod, Option<(&'static str, String)>)> {
    writeln!(out, "  No LLM provider detected.\n")?;
    writeln!(out, "  Options:")?;
    writeln!(
        out,
        "    1. Install Claude CLI: npm install -g @anthropic-ai/claude-cli && claude login"
    )?;
    writeln!(
        out,
        "    2. Paste an API key below; setup keeps it in ~/.roko/.env"
    )?;
    writeln!(out)?;
    write!(out, "  Enter API key (or press Enter to skip): ")?;
    out.flush()?;

    let key = if terminal {
        super::auth::read_password_from_terminal()?
    } else {
        let mut line = String::new();
        input.read_line(&mut line)?;
        line
    };
    let key = key.trim().to_string();
    if key.is_empty() {
        writeln!(out, "  Skipped. Put a key in ~/.roko/.env later.")?;
        return Ok((AuthMethod::NeedsSetup, None));
    }

    let name = match key_variable_by_prefix(&key) {
        Some(name) => name,
        None => {
            writeln!(
                out,
                "  Which provider is this key ({}) for?",
                masked_key(&key)
            )?;
            for (index, (name, provider, _)) in KEY_PROVIDERS.iter().enumerate() {
                writeln!(out, "    {}. {provider} ({name})", index + 1)?;
            }
            write!(out, "  Number: ")?;
            out.flush()?;
            let mut choice = String::new();
            input.read_line(&mut choice)?;
            let picked = choice
                .trim()
                .parse::<usize>()
                .ok()
                .and_then(|number| number.checked_sub(1))
                .and_then(|index| KEY_PROVIDERS.get(index));
            let Some(&(name, _, _)) = picked else {
                writeln!(
                    out,
                    "  Not saved: no provider chosen. Put the key in ~/.roko/.env."
                )?;
                return Ok((AuthMethod::NeedsSetup, None));
            };
            name
        }
    };
    let Some(home) = home else {
        writeln!(
            out,
            "  Not saved: HOME is not set. Put {name}=<your key> in ~/.roko/.env."
        )?;
        return Ok((AuthMethod::NeedsSetup, None));
    };
    let path = store_api_key(home, name, &key)?;
    writeln!(
        out,
        "  Saved {name} ({}) in {}, which every roko command loads.",
        masked_key(&key),
        path.display()
    )?;
    let auth = if name == "ANTHROPIC_API_KEY" {
        AuthMethod::AnthropicApi {
            key: key.clone(),
            model: None,
        }
    } else {
        let base_url = KEY_PROVIDERS
            .iter()
            .find(|(variable, _, _)| *variable == name)
            .map_or("https://api.openai.com/v1", |(_, _, url)| *url);
        AuthMethod::OpenAiCompat {
            key: key.clone(),
            base_url: base_url.to_string(),
            model: None,
        }
    };
    Ok((auth, Some((name, key))))
}

/// Set `name=key` in `home`'s `.roko/.env` and keep the file readable only
/// by its owner. The line that sets `name`, or the template's commented
/// `# name=`, is replaced; every other line stays. Returns the file's path.
fn store_api_key(home: &Path, name: &str, key: &str) -> Result<PathBuf> {
    ensure_global_env(home)?;
    let path = home.join(".roko").join(".env");
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let assignment = format!("{name}=");
    let mut replaced = false;
    let mut lines: Vec<String> = text
        .lines()
        .map(|line| {
            let bare = line.trim_start().trim_start_matches('#').trim_start();
            let bare = bare.strip_prefix("export ").unwrap_or(bare);
            if !replaced && bare.starts_with(&assignment) {
                replaced = true;
                format!("{name}={key}")
            } else {
                line.to_string()
            }
        })
        .collect();
    if !replaced {
        lines.push(format!("{name}={key}"));
    }
    let mut text = lines.join("\n");
    text.push('\n');
    write_owner_only(&path, text.as_bytes())?;
    Ok(path)
}

/// Replace `path` with `bytes` through a file created readable only by its
/// owner, so a key is never readable by others, not even for a moment.
fn write_owner_only(path: &Path, bytes: &[u8]) -> Result<()> {
    let name = path
        .file_name()
        .map_or_else(Default::default, |name| name.to_string_lossy());
    let staged = path.with_file_name(format!(".{name}.{}.tmp", std::process::id()));
    let _ = std::fs::remove_file(&staged);
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut file = options
        .open(&staged)
        .with_context(|| format!("create {}", staged.display()))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .with_context(|| format!("write {}", staged.display()))?;
    std::fs::rename(&staged, path).with_context(|| format!("replace {}", path.display()))
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

#[cfg(test)]
mod tests {
    use super::*;

    /// `roko setup` creates `~/.roko/.env` once, owner-only and setting no
    /// key, and never touches an existing one.
    #[test]
    fn setup_creates_the_global_env_file_once() {
        let home = tempfile::tempdir().expect("tempdir");
        let path = ensure_global_env(home.path())
            .expect("create")
            .expect("a new file");
        assert_eq!(path, home.path().join(".roko/.env"));
        let text = std::fs::read_to_string(&path).expect("read");
        assert!(
            text.lines().all(|line| line.starts_with('#')),
            "the template sets a variable: {text}"
        );
        assert!(text.contains("# OPENAI_API_KEY="));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mode = std::fs::metadata(&path).expect("stat").permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }

        std::fs::write(&path, "KEPT=1\n").expect("write");
        assert_eq!(ensure_global_env(home.path()).expect("existing"), None);
        assert_eq!(std::fs::read_to_string(&path).expect("read"), "KEPT=1\n");
    }

    /// bug-d32a50: a key typed into `roko setup` is never printed back, only
    /// its last four characters, and is kept in `~/.roko/.env`, owner-only,
    /// in the variable its prefix names or the one the person picks.
    #[test]
    fn setup_stores_a_typed_key_without_printing_it() {
        let home = tempfile::tempdir().expect("tempdir");
        let path = home.path().join(".roko/.env");
        let prompt = |typed: &str| {
            let mut input = io::Cursor::new(typed.to_string());
            let mut out = Vec::new();
            let result = prompt_for_api_key_with(&mut input, &mut out, Some(home.path()), false)
                .expect("prompt");
            (result, String::from_utf8(out).expect("utf8"))
        };

        let key = "sk-ant-api03-roko-init-secret-0123456789";
        let ((auth, saved), shown) = prompt(&format!("{key}\n"));
        assert!(!shown.contains("roko-init-secret"), "{shown}");
        assert!(shown.contains("ANTHROPIC_API_KEY (…6789)"), "{shown}");
        assert!(matches!(auth, AuthMethod::AnthropicApi { .. }));
        assert_eq!(saved, Some(("ANTHROPIC_API_KEY", key.to_string())));
        let text = std::fs::read_to_string(&path).expect("read");
        let set: Vec<&str> = text
            .lines()
            .filter(|line| line.contains("ANTHROPIC_API_KEY"))
            .collect();
        assert_eq!(set, [format!("ANTHROPIC_API_KEY={key}")]);
        assert!(
            text.contains("# OPENAI_API_KEY="),
            "the template's other lines stay"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mode = std::fs::metadata(&path).expect("stat").permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }

        // A key whose prefix names no one provider is asked about.
        let zai = "0f1e2d3c4b5a69788796a5b4c3d2e1f0.roko-init-zai";
        let ((auth, saved), shown) = prompt(&format!("{zai}\n5\n"));
        assert!(!shown.contains("roko-init-zai"), "{shown}");
        assert!(shown.contains("Z.ai (ZAI_API_KEY)"), "{shown}");
        assert!(
            matches!(&auth, AuthMethod::OpenAiCompat { base_url, .. } if base_url.contains("z.ai"))
        );
        assert_eq!(saved.map(|(name, _)| name), Some("ZAI_API_KEY"));
        let text = std::fs::read_to_string(&path).expect("read");
        assert!(text.contains(&format!("ZAI_API_KEY={zai}")));
        assert!(text.contains(&format!("ANTHROPIC_API_KEY={key}")));

        // No answer saves nothing, and nothing is shown.
        let ((auth, saved), shown) = prompt("sk-roko-init-ambiguous-0000\n\n");
        assert!(matches!(auth, AuthMethod::NeedsSetup));
        assert_eq!(saved, None);
        assert!(!shown.contains("roko-init-ambiguous"), "{shown}");
        assert!(
            !std::fs::read_to_string(&path)
                .expect("read")
                .contains("ambiguous")
        );
    }
}
