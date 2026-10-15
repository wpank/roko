//! Whether runs a chat host (Hermes, OpenClaw) starts over roko serve's
//! `/mcp` can start in a workspace, and `roko config preset chat-host`, the
//! one command that makes a workspace ready for them (gap-cf427e, gap-115bae).
//!
//! roko serve refuses such a run up front while the workspace has no
//! data-model boundary (`[agent.data_llm]`, or `[serve.mcp]
//! allow_without_data_llm` for local use) or no model roko's own tool loop
//! runs with its key set ([`RokoConfig::chat_run_refusal`]). The preset sets
//! the boundary in the workspace's `roko.toml` with `toml_edit`, so the
//! file's comments and layout stay as they are, and it never writes the
//! global `~/.roko/config.toml`. roko serve reads `roko.toml` again within
//! two seconds of a change, so it needs no restart.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context as _, Result, anyhow, bail};
use roko_core::config::DataLlmConfig;
use roko_core::config::loader::{LoadOptions, load_config_file};
use roko_core::config::schema::RokoConfig;
use serde::Serialize;

/// The comment above a table the preset adds to `roko.toml`.
const NOTE: &str = "For runs a chat host (Hermes, OpenClaw) starts: set by `roko config \
                    preset chat-host`";

/// The one setting `roko config preset chat-host` writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatHostEdit {
    /// `[agent.data_llm] model = <model>`: the data model that screens what
    /// a chat host's run reads.
    DataModel(String),
    /// `[serve.mcp] allow_without_data_llm = true`: for local use, such runs
    /// start without the data-model boundary.
    AllowWithoutDataModel,
}

impl ChatHostEdit {
    /// The dotted key the edit sets.
    #[must_use]
    pub const fn key(&self) -> &'static str {
        match self {
            Self::DataModel(_) => "agent.data_llm.model",
            Self::AllowWithoutDataModel => "serve.mcp.allow_without_data_llm",
        }
    }

    /// The value the key gets.
    fn value(&self) -> toml_edit::Value {
        match self {
            Self::DataModel(model) => toml_edit::Value::from(model.as_str()),
            Self::AllowWithoutDataModel => toml_edit::Value::from(true),
        }
    }

    /// `config` with the edit made, as roko serve sees it once `roko.toml`
    /// has it.
    fn apply(&self, config: &mut RokoConfig) {
        match self {
            Self::DataModel(model) => {
                let mut data_llm = config.agent.data_llm.clone().unwrap_or_default();
                data_llm.model.clone_from(model);
                config.agent.data_llm = Some(data_llm);
            }
            Self::AllowWithoutDataModel => config.serve.mcp.allow_without_data_llm = true,
        }
    }
}

/// What the preset changes in a workspace, before anything is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatHostPlan {
    /// The setting it writes; `None` when the workspace already has what
    /// chat hosts' runs need from it.
    pub edit: Option<ChatHostEdit>,
    /// Why that setting, or why none, in words for the person.
    pub why: String,
}

/// The setting that gives `config`'s workspace a data-model boundary.
///
/// - With `allow_without_data_llm`: `[serve.mcp] allow_without_data_llm =
///   true`, unless the boundary is set or already waived.
/// - With `data_llm_model`: that model, once roko can call it over an API
///   with its key set.
/// - Otherwise, unless `[agent.data_llm]` already names such a model, the
///   first model of [`RokoConfig::tool_loop_models`] (the ladder's cheapest
///   such rung first, then the cheapest by configured price), or else the
///   default data model when roko can call it.
///
/// An error when no model can serve as the data model, or when the one
/// asked for cannot.
pub fn plan_chat_host(
    config: &RokoConfig,
    allow_without_data_llm: bool,
    data_llm_model: Option<&str>,
) -> Result<ChatHostPlan> {
    let current = config
        .agent
        .data_llm
        .as_ref()
        .map(|data_llm| data_llm.model.trim().to_string());
    let current_problem = current
        .as_deref()
        .and_then(|model| config.data_llm_model_problem(model));
    if allow_without_data_llm {
        if let (Some(model), Some(problem)) = (&current, &current_problem) {
            bail!(
                "[agent.data_llm] model `{model}` {problem}, which fails every agent roko's tool \
                 loop runs whatever [serve.mcp] says: fix or remove that table, or run this \
                 preset without --allow-without-data-llm to pick a model"
            );
        }
        if let Some(model) = &current {
            return Ok(ChatHostPlan {
                edit: None,
                why: format!("the data-model boundary is set: [agent.data_llm] uses `{model}`"),
            });
        }
        if config.serve.mcp.allow_without_data_llm {
            return Ok(ChatHostPlan {
                edit: None,
                why: "[serve.mcp] allow_without_data_llm is already true".to_string(),
            });
        }
        return Ok(ChatHostPlan {
            edit: Some(ChatHostEdit::AllowWithoutDataModel),
            why: "for local use: runs a chat host starts go without the data-model boundary, so \
                  what they read is not screened"
                .to_string(),
        });
    }
    if let Some(model) = data_llm_model
        .map(str::trim)
        .filter(|model| !model.is_empty())
    {
        if let Some(problem) = config.data_llm_model_problem(model) {
            bail!("--data-llm-model `{model}` {problem}: name a model roko calls over an API");
        }
        if current.as_deref() == Some(model) {
            return Ok(ChatHostPlan {
                edit: None,
                why: format!("[agent.data_llm] already uses `{model}`"),
            });
        }
        return Ok(ChatHostPlan {
            edit: Some(ChatHostEdit::DataModel(model.to_string())),
            why: "the data model you named".to_string(),
        });
    }
    if let (Some(model), None) = (&current, &current_problem) {
        return Ok(ChatHostPlan {
            edit: None,
            why: format!(
                "[agent.data_llm] already uses `{model}`, which roko calls over an API with its \
                 key set"
            ),
        });
    }
    let default_model = DataLlmConfig::default().model;
    let picked = config
        .tool_loop_models()
        .into_iter()
        .find(|model| config.data_llm_model_problem(model).is_none())
        .or_else(|| {
            config
                .data_llm_model_problem(&default_model)
                .is_none()
                .then_some(default_model)
        });
    let Some(model) = picked else {
        let reason = config
            .chat_run_refusal()
            .map(|refusal| refusal.message)
            .unwrap_or_default();
        bail!(
            "no model here can serve as the data model ({reason}). Add an API model with its key \
             and run this preset again; --allow-without-data-llm waives the boundary for local \
             use, but a chat host's run still needs such a model"
        );
    };
    let why = if ladder_runs(config, &model) {
        "the cheapest model of the ladder that roko calls over an API with its key set"
    } else {
        "the cheapest model roko calls over an API with its key set"
    };
    Ok(ChatHostPlan {
        edit: Some(ChatHostEdit::DataModel(model)),
        why: why.to_string(),
    })
}

/// Whether one of `config`'s ladder rungs runs `model`, a `[models.*]` key.
fn ladder_runs(config: &RokoConfig, model: &str) -> bool {
    let ladder = &config.routing.ladder;
    let slug = config.models.get(model).map(|profile| profile.slug.trim());
    ladder.enabled
        && ladder.all_rungs().any(|rung| {
            let rung = rung.model.trim();
            rung == model || slug == Some(rung)
        })
}

/// What `roko config preset chat-host` shows and writes for one workspace.
#[derive(Debug, Clone, Serialize)]
pub struct ChatHostPreset {
    /// The workspace's `roko.toml`, the only file the preset writes.
    pub path: PathBuf,
    /// The setting it writes, as its dotted key and value; `None` when
    /// nothing changes.
    pub edit: Option<ChatHostEditJson>,
    /// Why that setting, or why none.
    pub why: String,
    /// The change as a unified diff of `roko.toml`; empty when nothing
    /// changes.
    pub diff: String,
    /// Whether chat hosts can start runs here once the change is in.
    pub ready: bool,
    /// The refusal codes that still apply then (`no_tool_loop_model`, ...).
    pub refusals: Vec<&'static str>,
    /// roko serve's words for that refusal.
    pub refusal: Option<String>,
    /// `roko.toml` with the change.
    #[serde(skip)]
    pub text: String,
}

/// [`ChatHostEdit`] as `--json` shows it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ChatHostEditJson {
    /// The dotted key (`agent.data_llm.model`).
    pub key: &'static str,
    /// Its new value (`"gpt-4o-mini"`, `true`).
    pub value: serde_json::Value,
}

/// The preset for the workspace at `workdir`, from its `roko.toml` as roko
/// serve loads it (the global config merged in, environment overrides
/// applied), without writing anything.
pub fn prepare_chat_host_preset(
    workdir: &Path,
    allow_without_data_llm: bool,
    data_llm_model: Option<&str>,
) -> Result<ChatHostPreset> {
    prepare_with(
        workdir,
        &LoadOptions::default(),
        allow_without_data_llm,
        data_llm_model,
    )
}

/// [`prepare_chat_host_preset`] with the loader's `options`.
fn prepare_with(
    workdir: &Path,
    options: &LoadOptions,
    allow_without_data_llm: bool,
    data_llm_model: Option<&str>,
) -> Result<ChatHostPreset> {
    let path = workdir.join("roko.toml");
    let before = std::fs::read_to_string(&path).with_context(|| {
        format!(
            "read {}: a chat host's runs need a roko workspace here (`roko init` makes one)",
            path.display()
        )
    })?;
    let config = load_config_file(&path, options)
        .map_err(|error| anyhow!("load {}: {error}", path.display()))?;
    let plan = plan_chat_host(&config, allow_without_data_llm, data_llm_model)?;
    let mut after_config = config;
    let text = match &plan.edit {
        Some(edit) => {
            edit.apply(&mut after_config);
            set_toml_key(&before, edit.key(), edit.value(), NOTE)?
        }
        None => before.clone(),
    };
    let refusal = after_config.chat_run_refusal();
    Ok(ChatHostPreset {
        diff: unified_diff("roko.toml", &before, &text),
        edit: plan.edit.as_ref().map(|edit| ChatHostEditJson {
            key: edit.key(),
            value: match edit {
                ChatHostEdit::DataModel(model) => serde_json::Value::from(model.as_str()),
                ChatHostEdit::AllowWithoutDataModel => serde_json::Value::Bool(true),
            },
        }),
        why: plan.why,
        ready: refusal.is_none(),
        refusals: refusal
            .as_ref()
            .map(|refusal| refusal.codes())
            .unwrap_or_default(),
        refusal: refusal.map(|refusal| refusal.message),
        path,
        text,
    })
}

/// Write `preset`'s change into its `roko.toml`, if it has one and
/// `roko config validate` would accept the result.
pub fn write_chat_host_preset(preset: &ChatHostPreset) -> Result<()> {
    if preset.diff.is_empty() {
        return Ok(());
    }
    crate::config_cmd::write_checked_config(&preset.path, &preset.text)
}

/// Whether git tracks the workspace's `roko.toml` and it differs from what
/// is committed: a run's merge into this checkout wants it committed.
#[must_use]
pub fn roko_toml_uncommitted(workdir: &Path) -> bool {
    let Ok(output) = Command::new("git")
        .arg("-C")
        .arg(workdir)
        .args(["status", "--porcelain", "--", "roko.toml"])
        .output()
    else {
        return false;
    };
    let status = String::from_utf8_lossy(&output.stdout);
    output.status.success()
        && status
            .lines()
            .any(|line| !line.trim().is_empty() && !line.starts_with("??"))
}

/// `text`, a `roko.toml`, with dotted `key` set to `value`, every other
/// line, comment and the layout kept (`toml_edit`). A table it adds is
/// placed after its parent's last table with `note` as a comment above it;
/// one it adds only to hold another stays implicit, with no header line. A
/// value it replaces keeps its comments.
fn set_toml_key(text: &str, key: &str, value: toml_edit::Value, note: &str) -> Result<String> {
    let mut document = text
        .parse::<toml_edit::DocumentMut>()
        .context("roko.toml is not valid TOML")?;
    let (tables, leaf) = key.rsplit_once('.').context("a dotted key")?;
    let names: Vec<&str> = tables.split('.').collect();
    let mut table = document.as_table_mut();
    for (index, name) in names.iter().enumerate() {
        if !table.contains_key(name) {
            let mut added = toml_edit::Table::new();
            if index + 1 == names.len() {
                added.decor_mut().set_prefix(format!("\n# {note}\n"));
            } else {
                added.set_implicit(true);
            }
            table.insert(name, toml_edit::Item::Table(added));
        }
        table = table
            .get_mut(name)
            .and_then(toml_edit::Item::as_table_mut)
            .with_context(|| {
                format!(
                    "`{}` in roko.toml is not a table; set {key} there by hand",
                    names[..=index].join(".")
                )
            })?;
    }
    // Replaced in place: `Table::insert` would reformat the key, dropping
    // the comment lines above it.
    if let Some(toml_edit::Item::Value(old)) = table.get_mut(leaf) {
        let decor = old.decor().clone();
        *old = value;
        *old.decor_mut() = decor;
    } else {
        table.insert(leaf, toml_edit::Item::Value(value));
    }
    Ok(document.to_string())
}

/// A unified diff of `before` and `after` under `label`: one hunk around
/// the lines that differ, with three lines of context, or nothing when they
/// are equal. The preset changes one place in the file, so one hunk shows
/// all of it.
fn unified_diff(label: &str, before: &str, after: &str) -> String {
    let old: Vec<&str> = before.lines().collect();
    let new: Vec<&str> = after.lines().collect();
    let prefix = old.iter().zip(&new).take_while(|(a, b)| a == b).count();
    let suffix = old[prefix..]
        .iter()
        .rev()
        .zip(new[prefix..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    if prefix == old.len() && prefix == new.len() {
        return String::new();
    }
    let start = prefix.saturating_sub(3);
    let old_end = (old.len() - suffix + 3).min(old.len());
    let new_end = (new.len() - suffix + 3).min(new.len());
    let mut diff = format!(
        "--- {label}\n+++ {label}\n@@ -{},{} +{},{} @@\n",
        start + 1,
        old_end - start,
        start + 1,
        new_end - start
    );
    for line in &old[start..prefix] {
        diff.push_str(&format!(" {line}\n"));
    }
    for line in &old[prefix..old.len() - suffix] {
        diff.push_str(&format!("-{line}\n"));
    }
    for line in &new[prefix..new.len() - suffix] {
        diff.push_str(&format!("+{line}\n"));
    }
    for line in &old[old.len() - suffix..old_end] {
        diff.push_str(&format!(" {line}\n"));
    }
    diff
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::agent::ProviderKind;
    use roko_core::config::schema::{ModelProfile, ProviderConfig};

    /// The loader's options for these tests: this file alone, with no
    /// global config and no environment overrides.
    fn file_only() -> LoadOptions {
        LoadOptions {
            merge_global: false,
            apply_env_overrides: false,
            apply_hierarchical_env: false,
            strict_validation: false,
        }
    }

    /// A workspace `roko.toml` with comments whose agents run on the Claude
    /// CLI, plus `api-mini` on an OpenAI-compatible API whose key is `PATH`
    /// (always set) and the ladder `cheap = api-mini`, `top = cli-sonnet`.
    /// The providers roko would make from API keys in the environment are
    /// configured without a key, so none of them can serve.
    const ROKO_TOML: &str = r#"# Will's workspace: keep this comment.
config_version = 2
schema_version = 2

[project]
name = "chat-host-preset" # and this one

[agent]
default_model = "cli-sonnet"

[providers.claude_cli]
kind = "claude_cli"
command = "claude"

[providers.api]
kind = "openai_compat"
base_url = "http://127.0.0.1:9/v1"
api_key_env = "PATH"

[providers.anthropic]
kind = "anthropic_api"
api_key_env = "ROKO_TEST_CHAT_HOST_KEY_NEVER_SET"

[providers.openai]
kind = "openai_compat"
api_key_env = "ROKO_TEST_CHAT_HOST_KEY_NEVER_SET"

[providers.gemini]
kind = "gemini_api"
api_key_env = "ROKO_TEST_CHAT_HOST_KEY_NEVER_SET"

[providers.perplexity]
kind = "perplexity_api"
api_key_env = "ROKO_TEST_CHAT_HOST_KEY_NEVER_SET"

[models.cli-sonnet]
provider = "claude_cli"
slug = "claude-sonnet-4-6"

[models.api-mini]
provider = "api"
slug = "api-mini-1"
cost_input_per_m = 0.5
cost_output_per_m = 0.5

[routing]
fallback_models = []

[routing.ladder]
rungs = [{ name = "cheap", model = "api-mini" }, { name = "top", model = "cli-sonnet" }]

# The end of the file: keep this comment too.
"#;

    fn workspace(text: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("roko.toml"), text).expect("write roko.toml");
        dir
    }

    /// gap-115bae: the preset gives a workspace without the data-model
    /// boundary `[agent.data_llm]` with the cheapest model of the ladder
    /// that roko calls over an API, shown first as a diff; the written
    /// `roko.toml` keeps every comment, and chat hosts' runs may start.
    #[test]
    fn chat_host_preset_sets_the_data_model_and_keeps_comments() {
        let dir = workspace(ROKO_TOML);
        let preset = prepare_with(dir.path(), &file_only(), false, None).expect("a preset");
        assert_eq!(
            preset.edit,
            Some(ChatHostEditJson {
                key: "agent.data_llm.model",
                value: serde_json::json!("api-mini"),
            })
        );
        assert!(
            preset.why.contains("cheapest model of the ladder"),
            "{}",
            preset.why
        );
        assert!(preset.ready, "{preset:?}");
        assert!(preset.refusals.is_empty());
        assert!(
            preset.diff.starts_with("--- roko.toml\n+++ roko.toml\n@@ "),
            "{}",
            preset.diff
        );
        assert!(
            preset
                .diff
                .contains("\n+[agent.data_llm]\n+model = \"api-mini\"\n"),
            "{}",
            preset.diff
        );
        assert!(
            !preset
                .diff
                .lines()
                .any(|line| line.starts_with('-') && !line.starts_with("---"))
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join("roko.toml"))
                .ok()
                .as_deref(),
            Some(ROKO_TOML),
            "nothing written yet"
        );

        write_chat_host_preset(&preset).expect("written");
        let written = std::fs::read_to_string(dir.path().join("roko.toml")).expect("read");
        for kept in [
            "# Will's workspace: keep this comment.",
            "name = \"chat-host-preset\" # and this one",
            "# The end of the file: keep this comment too.",
        ] {
            assert!(written.contains(kept), "{kept}: {written}");
        }
        assert_eq!(written, preset.text);
        let at = |needle: &str| written.find(needle).expect(needle);
        assert!(at("[agent]") < at("[agent.data_llm]"), "{written}");
        assert!(
            at("[agent.data_llm]") < at("[providers.claude_cli]"),
            "{written}"
        );

        let config = load_config_file(&dir.path().join("roko.toml"), &file_only()).expect("loads");
        assert_eq!(
            config
                .agent
                .data_llm
                .map(|data_llm| data_llm.model)
                .as_deref(),
            Some("api-mini")
        );
        let again = prepare_with(dir.path(), &file_only(), false, None).expect("a preset");
        assert_eq!(again.edit, None, "{again:?}");
        assert!(again.diff.is_empty());
        assert!(again.ready);
    }

    /// With `--allow-without-data-llm` the preset waives the boundary in an
    /// existing `[serve]` table; a workspace whose only agents run their own
    /// tools stays refused, for `no_tool_loop_model`, and no model can be
    /// the data model there.
    #[test]
    fn chat_host_preset_waives_the_boundary_and_says_what_still_refuses() {
        let cli_only = ROKO_TOML
            .replace(
                "api_key_env = \"PATH\"",
                "api_key_env = \"ROKO_TEST_CHAT_HOST_KEY_NEVER_SET\"",
            )
            .replace(
                "# The end of the file",
                "[serve]\nport = 6799\n\n# The end of the file",
            );
        let dir = workspace(&cli_only);
        let preset = prepare_with(dir.path(), &file_only(), true, None).expect("a preset");
        assert_eq!(
            preset.edit.as_ref().map(|edit| edit.key),
            Some("serve.mcp.allow_without_data_llm")
        );
        assert!(
            preset
                .diff
                .contains("+[serve.mcp]\n+allow_without_data_llm = true\n"),
            "{}",
            preset.diff
        );
        assert!(!preset.ready);
        assert_eq!(preset.refusals, ["no_tool_loop_model"]);
        let refusal = preset.refusal.as_deref().unwrap_or_default();
        assert!(
            refusal.contains("ROKO_TEST_CHAT_HOST_KEY_NEVER_SET"),
            "{refusal}"
        );

        let error = prepare_with(dir.path(), &file_only(), false, None).expect_err("no data model");
        assert!(
            error.to_string().contains("can serve as the data model"),
            "{error}"
        );
    }

    /// The model asked for wins once roko can call it; one on the Claude
    /// CLI cannot be the data model, and a broken `[agent.data_llm]` is
    /// replaced, not waived.
    #[test]
    fn chat_host_preset_checks_the_data_model() {
        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        for (id, kind, key) in [
            ("claude_cli", ProviderKind::ClaudeCli, None),
            ("api", ProviderKind::OpenAiCompat, Some("PATH")),
            (
                "anthropic",
                ProviderKind::AnthropicApi,
                Some("ROKO_TEST_CHAT_HOST_KEY_NEVER_SET"),
            ),
            (
                "openai",
                ProviderKind::OpenAiCompat,
                Some("ROKO_TEST_CHAT_HOST_KEY_NEVER_SET"),
            ),
            (
                "gemini",
                ProviderKind::GeminiApi,
                Some("ROKO_TEST_CHAT_HOST_KEY_NEVER_SET"),
            ),
            (
                "perplexity",
                ProviderKind::PerplexityApi,
                Some("ROKO_TEST_CHAT_HOST_KEY_NEVER_SET"),
            ),
        ] {
            let provider = ProviderConfig {
                kind,
                api_key_env: key.map(str::to_string),
                command: (kind == ProviderKind::ClaudeCli).then(|| "claude".to_string()),
                ..ProviderConfig::default()
            };
            config.providers.insert(id.to_string(), provider);
        }
        for (key, provider, slug, price) in [
            ("cli-sonnet", "claude_cli", "claude-sonnet-4-6", None),
            ("api-dear", "api", "api-dear-1", Some(9.0)),
            ("api-cheap", "api", "api-cheap-1", Some(0.1)),
        ] {
            let profile = ModelProfile {
                provider: provider.to_string(),
                slug: slug.to_string(),
                supports_tools: true,
                cost_input_per_m: price,
                cost_output_per_m: price,
                ..ModelProfile::default()
            };
            config.models.insert(key.to_string(), profile);
        }
        config.routing.ladder.enabled = false;
        config.routing.fallback_models.clear();
        config.agent.default_model = "cli-sonnet".to_string();

        let plan = plan_chat_host(&config, false, None).expect("a plan");
        assert_eq!(
            plan.edit,
            Some(ChatHostEdit::DataModel("api-cheap".to_string()))
        );
        assert!(
            plan.why.contains("cheapest model roko calls"),
            "{}",
            plan.why
        );
        let plan = plan_chat_host(&config, false, Some("api-dear")).expect("a plan");
        assert_eq!(
            plan.edit,
            Some(ChatHostEdit::DataModel("api-dear".to_string()))
        );
        let error = plan_chat_host(&config, false, Some("cli-sonnet")).expect_err("the CLI");
        assert!(
            error.to_string().contains("cannot call over an API"),
            "{error}"
        );

        config.agent.data_llm = Some(DataLlmConfig {
            model: "cli-sonnet".to_string(),
            ..DataLlmConfig::default()
        });
        let plan = plan_chat_host(&config, false, None).expect("a plan");
        assert_eq!(
            plan.edit,
            Some(ChatHostEdit::DataModel("api-cheap".to_string()))
        );
        let error = plan_chat_host(&config, true, None).expect_err("a broken table");
        assert!(
            error.to_string().contains("fix or remove that table"),
            "{error}"
        );
    }

    /// The edit keeps a replaced value's comment and adds an implicit
    /// parent table only to hold the one it adds.
    #[test]
    fn set_toml_key_keeps_comments_and_adds_only_what_it_needs() {
        let text = "[agent.data_llm]\nmodel = \"old\" # the data model\nmax_tokens = 9\n";
        let edited = set_toml_key(
            text,
            "agent.data_llm.model",
            toml_edit::Value::from("new"),
            NOTE,
        )
        .expect("edited");
        assert_eq!(
            edited,
            "[agent.data_llm]\nmodel = \"new\" # the data model\nmax_tokens = 9\n"
        );

        let text = "[serve.mcp]\n# for local use only\nallow_without_data_llm = false # off\n";
        let edited = set_toml_key(
            text,
            "serve.mcp.allow_without_data_llm",
            toml_edit::Value::from(true),
            NOTE,
        )
        .expect("edited");
        assert_eq!(
            edited,
            "[serve.mcp]\n# for local use only\nallow_without_data_llm = true # off\n"
        );

        let edited = set_toml_key(
            "# top\nconfig_version = 2\n",
            "serve.mcp.allow_without_data_llm",
            toml_edit::Value::from(true),
            NOTE,
        )
        .expect("edited");
        assert_eq!(
            edited,
            format!(
                "# top\nconfig_version = 2\n\n# {NOTE}\n[serve.mcp]\nallow_without_data_llm = true\n"
            )
        );

        let error = set_toml_key(
            "agent = 1\n",
            "agent.data_llm.model",
            toml_edit::Value::from("m"),
            NOTE,
        )
        .expect_err("not a table");
        assert!(
            error
                .to_string()
                .contains("set agent.data_llm.model there by hand"),
            "{error}"
        );
    }
}
