//! Adapter for the Google Gemini CLI subprocess provider.
//!
//! This adapter spawns the `gemini` binary (Google Gemini CLI) as a subprocess.
//! Unlike the Gemini API backends, this uses Google OAuth authentication rather
//! than an API key — users authenticate via `gemini /auth` beforehand.
//!
//! The adapter falls through to [`ExecAgent`](crate::ExecAgent) for legacy
//! one-shot callers. Runner-v2 uses Gemini's headless stream-JSON protocol and
//! authenticated per-task MCP configuration through its CLI dispatch adapter.
//!
//! # Configuration
//!
//! ```toml
//! [providers.gemini-cli]
//! kind = "gemini_cli"
//! # No api_key_env — uses Google OAuth via `gemini /auth`
//! # Optional: override the binary path
//! command = "gemini"   # default
//!
//! [models.gemini-3-flash]
//! provider = "gemini-cli"
//! slug = "gemini-3-flash-preview"
//! ```

use crate::Agent;
use crate::exec::ExecAgent;
use crate::provider::pre_flight::binary_on_path;
use crate::provider::{
    AgentCreationError, AgentOptions, ProviderAdapter, ProviderError, TurnCapEnforcement,
    configured_resource_limits, provider_credential_scrub,
};
use roko_core::agent::ProviderKind;
use roko_core::config::schema::{ModelProfile, ProviderConfig};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Default Gemini CLI binary name.
const DEFAULT_GEMINI_COMMAND: &str = "gemini";

/// The environment variable that points the Gemini CLI at its system
/// settings file, whose settings override the user's and the project's.
const SYSTEM_SETTINGS_ENV: &str = "GEMINI_CLI_SYSTEM_SETTINGS_PATH";

/// The Gemini CLI's own system settings file on this platform. This crate's
/// tests look at a path that never exists instead, so that they do not
/// depend on the host's settings.
fn default_system_settings() -> PathBuf {
    if cfg!(test) {
        return PathBuf::from("/nonexistent/roko-tests/gemini-cli-system-settings.json");
    }
    PathBuf::from(if cfg!(target_os = "macos") {
        "/Library/Application Support/GeminiCli/settings.json"
    } else if cfg!(windows) {
        r"C:\ProgramData\gemini-cli\settings.json"
    } else {
        "/etc/gemini-cli/settings.json"
    })
}

/// A system settings file for a run held to `max_turns` turns
/// (gap-130413): the system settings at `inherited` (else the platform's),
/// with `model.maxSessionTurns` at `max_turns`, or at the lower limit they
/// already set. The file is removed when the returned path is dropped.
fn turn_capped_settings(
    max_turns: u32,
    inherited: Option<&Path>,
) -> std::io::Result<tempfile::TempPath> {
    let source = inherited.map_or_else(default_system_settings, Path::to_path_buf);
    let settings = std::fs::read_to_string(&source)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok());
    let turns = settings
        .as_ref()
        .and_then(|settings| settings.pointer("/model/maxSessionTurns"))
        .and_then(Value::as_i64)
        .filter(|turns| *turns >= 0)
        .map_or(i64::from(max_turns), |turns| {
            turns.min(i64::from(max_turns))
        });
    let mut root = match settings {
        Some(Value::Object(root)) => root,
        _ => serde_json::Map::new(),
    };
    let model = root
        .entry("model")
        .or_insert_with(|| Value::Object(serde_json::Map::new()));
    if !model.is_object() {
        *model = Value::Object(serde_json::Map::new());
    }
    if let Value::Object(model) = model {
        model.insert("maxSessionTurns".to_string(), Value::from(turns));
    }
    let text = serde_json::to_vec_pretty(&Value::Object(root)).map_err(std::io::Error::other)?;
    let mut file = tempfile::Builder::new()
        .prefix("roko-gemini-settings-")
        .suffix(".json")
        .tempfile()?;
    std::io::Write::write_all(&mut file, &text)?;
    Ok(file.into_temp_path())
}

/// Gemini headless stream-JSON translation.
pub mod stream;

/// Provider adapter for the Gemini CLI subprocess.
pub struct GeminiCliAdapter;

impl ProviderAdapter for GeminiCliAdapter {
    fn kind(&self) -> ProviderKind {
        ProviderKind::GeminiCli
    }

    fn create_agent(
        &self,
        provider: &ProviderConfig,
        model: &ModelProfile,
        options: &AgentOptions,
    ) -> Result<Box<dyn Agent>, AgentCreationError> {
        if provider.kind != self.kind() {
            return Err(AgentCreationError::InvalidKind(provider.kind));
        }

        let command = provider
            .command
            .as_deref()
            .map(str::trim)
            .filter(|cmd| !cmd.is_empty())
            .unwrap_or(DEFAULT_GEMINI_COMMAND);

        // Pre-flight: the binary must be present on PATH before we attempt to
        // spawn. Report a clear error rather than falling back silently.
        if !binary_on_path(command) {
            return Err(AgentCreationError::BinaryNotFound(format!(
                "{command} (Gemini CLI — install from https://github.com/google-gemini/gemini-cli \
                 or configure providers.*.command in roko.toml)"
            )));
        }

        let working_dir = options
            .working_dir
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        let timeout_ms = options.effective_timeout_ms(provider.timeout_ms);

        // Build the argument list for `gemini -m <slug> -p`.
        // -m selects the model; -p signals non-interactive prompt mode (reads stdin).
        let mut extra_args = vec!["-m".to_string(), model.slug.clone(), "-p".to_string()];

        // Append provider-level args (e.g. `--sandbox`, `--debug`) before option-level args.
        if let Some(provider_args) = &provider.args {
            extra_args.extend(provider_args.iter().cloned());
        }
        extra_args.extend(options.extra_args.iter().cloned());

        let name = if options.name.is_empty() {
            format!("gemini-cli:{}", model.slug)
        } else {
            options.name.clone()
        };

        let mut agent = ExecAgent::new(
            command,
            extra_args,
            crate::safety::SafetyLayer::with_defaults().with_role("implementer"),
        )
        .with_timeout_ms(timeout_ms)
        .with_name(name)
        .with_current_dir(working_dir)
        .with_credential_scrub(provider_credential_scrub(provider, options));

        if let Some(limits) = configured_resource_limits(provider)? {
            agent = agent.with_resource_limits(limits);
        }

        for (key, value) in &options.env {
            agent = agent.with_env_var(key.clone(), value.clone());
        }
        if let Some(prompt) = &options.system_prompt {
            agent = agent.with_stdin_prefix(prompt.clone());
        }
        // The CLI has no turn flag: the turn cap goes in as its
        // `model.maxSessionTurns` setting, through a system settings file of
        // the run's own (gap-130413).
        if let Some(max_turns) = options.max_turns {
            let inherited = options
                .env
                .iter()
                .rev()
                .find(|(key, _)| key == SYSTEM_SETTINGS_ENV)
                .map(|(_, path)| PathBuf::from(path))
                .or_else(|| std::env::var_os(SYSTEM_SETTINGS_ENV).map(PathBuf::from));
            match turn_capped_settings(max_turns, inherited.as_deref()) {
                Ok(settings) => {
                    agent = agent
                        .with_env_var(SYSTEM_SETTINGS_ENV, settings.to_string_lossy().into_owned())
                        .with_scratch_file(settings);
                }
                Err(error) => tracing::warn!(
                    %error,
                    "the Gemini CLI's turn cap could not be written: only the attempt timeout \
                     bounds this run"
                ),
            }
        }

        Ok(Box::new(agent))
    }

    fn classify_error(&self, status: u16, body: &Value) -> ProviderError {
        super::error_classify::classify_cli_error(status, body, "gemini CLI")
    }

    /// The Gemini CLI has no turn flag; its cap is the `model.maxSessionTurns`
    /// setting, which this adapter writes into the run's own system settings
    /// file (gap-130413). The CLI stops there, and the stop counts as a
    /// turn-cap hit.
    fn turn_cap_enforcement(&self, _provider: &ProviderConfig) -> TurnCapEnforcement {
        TurnCapEnforcement::Native
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::config::DEFAULT_TTFT_TIMEOUT_MS;

    fn gemini_cli_provider(command: Option<&str>) -> ProviderConfig {
        ProviderConfig {
            kind: ProviderKind::GeminiCli,
            base_url: None,
            api_key_env: None,
            command: command.map(str::to_string),
            args: None,
            timeout_ms: Some(30_000),
            ttft_timeout_ms: Some(DEFAULT_TTFT_TIMEOUT_MS),
            connect_timeout_ms: None,
            extra_headers: None,
            max_concurrent: None,
            limits: None,
            require_confirmation: false,
            stream_usage: None,
            billing: None,
        }
    }

    fn gemini_cli_model() -> ModelProfile {
        ModelProfile {
            provider: "gemini-cli".to_string(),
            slug: "gemini-3-flash-preview".to_string(),
            context_window: 1_048_576,
            max_output: Some(8_192),
            supports_tools: false,
            ..Default::default()
        }
    }

    #[test]
    fn gemini_cli_adapter_kind() {
        assert_eq!(GeminiCliAdapter.kind(), ProviderKind::GeminiCli);
    }

    #[test]
    fn gemini_cli_adapter_rejects_wrong_kind() {
        let provider = ProviderConfig {
            kind: ProviderKind::GeminiApi,
            ..gemini_cli_provider(None)
        };
        let result =
            GeminiCliAdapter.create_agent(&provider, &gemini_cli_model(), &AgentOptions::default());
        let Err(err) = result else {
            panic!("expected Err, got Ok");
        };
        assert!(matches!(err, AgentCreationError::InvalidKind(_)));
    }

    #[test]
    fn gemini_cli_adapter_missing_binary_returns_binary_not_found() {
        let provider = gemini_cli_provider(Some("roko-nonexistent-gemini-binary-xyz-090"));
        let result =
            GeminiCliAdapter.create_agent(&provider, &gemini_cli_model(), &AgentOptions::default());
        let Err(err) = result else {
            panic!("expected Err, got Ok");
        };
        if let AgentCreationError::BinaryNotFound(ref msg) = err {
            assert!(
                msg.contains("roko-nonexistent-gemini-binary-xyz-090"),
                "error message should include the missing binary name: {msg}"
            );
        } else {
            panic!("expected BinaryNotFound, got {err:?}");
        }
    }

    #[test]
    fn gemini_cli_adapter_classify_rate_limit() {
        let err = GeminiCliAdapter.classify_error(
            429,
            &serde_json::Value::String("rate limit exceeded".to_string()),
        );
        assert!(matches!(err, ProviderError::RateLimit { .. }));
    }

    #[test]
    fn gemini_cli_adapter_classify_auth_failure_unauthenticated_message() {
        let err = GeminiCliAdapter.classify_error(
            401,
            &serde_json::Value::String("not logged in — run `gemini /auth`".to_string()),
        );
        assert!(matches!(err, ProviderError::AuthFailure));
    }

    #[test]
    fn gemini_cli_adapter_classify_auth_failure_status_code() {
        let err = GeminiCliAdapter.classify_error(403, &serde_json::Value::Null);
        assert!(matches!(err, ProviderError::AuthFailure));
    }

    #[test]
    fn gemini_cli_adapter_classify_model_not_found() {
        let err = GeminiCliAdapter.classify_error(
            404,
            &serde_json::Value::String("model not found".to_string()),
        );
        assert!(matches!(err, ProviderError::ModelNotFound));
    }

    #[test]
    fn gemini_cli_adapter_classify_timeout() {
        let err = GeminiCliAdapter.classify_error(
            408,
            &serde_json::Value::String("request timed out".to_string()),
        );
        assert!(matches!(err, ProviderError::Timeout));
    }

    #[test]
    fn gemini_cli_adapter_classify_server_error() {
        let err = GeminiCliAdapter.classify_error(503, &serde_json::Value::Null);
        assert!(matches!(err, ProviderError::ServerError(503)));
    }

    /// gap-130413: a run with a turn cap gets it as the Gemini CLI's
    /// `model.maxSessionTurns`, in a system settings file of its own that
    /// keeps the system settings it would otherwise read and a lower limit
    /// they set, and is removed with the agent; the CLI's stop there is a
    /// turn-cap hit.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_gemini_cli_attempt_runs_under_roko_s_turn_cap() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::tempdir().expect("tempdir");
        // Prints the settings file it was pointed at, and what is in it.
        let script = tmp.path().join("gemini-fake.sh");
        std::fs::write(
            &script,
            "#!/bin/sh\ncat >/dev/null\nprintf '%s\\n' \"$GEMINI_CLI_SYSTEM_SETTINGS_PATH\"\ncat \"$GEMINI_CLI_SYSTEM_SETTINGS_PATH\"\n",
        )
        .expect("write fake gemini");
        let mut permissions = std::fs::metadata(&script).expect("metadata").permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&script, permissions).expect("make executable");
        let provider = gemini_cli_provider(Some(&script.display().to_string()));
        let prompt = roko_core::Signal::builder(roko_core::Kind::Prompt)
            .body(roko_core::Body::text("hi"))
            .build();
        let run = |options: AgentOptions| {
            let provider = provider.clone();
            let prompt = prompt.clone();
            async move {
                let agent = GeminiCliAdapter
                    .create_agent(&provider, &gemini_cli_model(), &options)
                    .expect("create agent");
                let result = agent.run(&prompt, &roko_core::Context::now()).await;
                assert!(result.success, "{:?}", result.output.body.as_text());
                let text = result.output.body.as_text().expect("output").to_string();
                drop(agent);
                let (path, settings) = text.split_once('\n').expect("path, then settings");
                let settings: Value = serde_json::from_str(settings).expect("settings JSON");
                (PathBuf::from(path), settings)
            }
        };

        let (path, settings) = run(AgentOptions {
            max_turns: Some(7),
            ..AgentOptions::default()
        })
        .await;
        assert_eq!(settings["model"]["maxSessionTurns"], 7, "{settings}");
        assert!(!path.exists(), "removed with the agent: {}", path.display());

        let system = tmp.path().join("system-settings.json");
        std::fs::write(
            &system,
            r#"{"general": {"vimMode": true}, "model": {"maxSessionTurns": 3}}"#,
        )
        .expect("write system settings");
        let (_, settings) = run(AgentOptions {
            max_turns: Some(7),
            env: vec![(
                SYSTEM_SETTINGS_ENV.to_string(),
                system.display().to_string(),
            )],
            ..AgentOptions::default()
        })
        .await;
        assert_eq!(settings["model"]["maxSessionTurns"], 3, "{settings}");
        assert_eq!(settings["general"]["vimMode"], true, "{settings}");

        let stop = "exit 53: Reached max session turns for this session. Increase the number of \
                    turns by specifying maxSessionTurns in settings.json.";
        assert!(super::super::error_classify::detect_turn_cap(stop).is_some());
        assert_eq!(
            GeminiCliAdapter.turn_cap_enforcement(&provider),
            TurnCapEnforcement::Native
        );
    }
}
