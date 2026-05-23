//! Tier 2 transport: `openclaw infer model run ... --json`.
//!
//! Wraps `ChildProcessRunner` to spawn `openclaw infer model run`
//! and parse the stable JSON envelope into an `AgentResult`.
//!
//! ## Token accounting
//!
//! OpenClaw's `infer --json` envelope does NOT include a `usage` field.
//! V1 uses character-count estimation only:
//!
//! - `input_tokens = max(prompt_chars / 4, 1)`
//! - `output_tokens = max(output_chars / 4, 1)`
//!
//! The episode log records `usage_estimated: true` to distinguish from
//! real token counts.
//!
//! ## Kill-on-timeout
//!
//! When the configured timeout elapses, `ChildProcessRunner` sends
//! SIGTERM to the child process group, waits 5 seconds, then SIGKILL.
//! The adapter returns `HarnessError::Timeout` so the orchestrator
//! can replan or route to a different provider.

use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::agent::{Agent, AgentResult, derived_output};
use crate::harness::capability::*;
use crate::harness::child_process_runner::ChildProcessRunner;
use crate::harness::events::HarnessEvent;
use crate::harness::service::HarnessService;
use crate::harness::{HarnessAdapter, HarnessCapabilities, ProbeError, TransportFlavor};
use roko_core::{Body, Context, Kind, Signal};

use super::config::OpenClawInferConfig;
use super::gateway_service::OpenClawGatewayService;
use super::infer_envelope::InferEventParser;
use super::probe;

/// OpenClaw infer adapter: executes `openclaw infer model run --json`
/// and parses the 8-field JSON envelope.
///
/// This adapter is stateless -- each `run()` call spawns a fresh
/// subprocess. The Node.js cold-start overhead is ~900ms p50.
///
/// ## Example
///
/// ```ignore
/// let config = OpenClawInferConfig::default();
/// let agent = OpenClawInferAgent::new(config)?;
/// let prompt = Signal::builder(Kind::Prompt).body(Body::text("hello")).build();
/// let result = agent.run(&prompt, &Context::now()).await;
/// assert!(result.success);
/// ```
pub struct OpenClawInferAgent {
    runner: ChildProcessRunner,
    config: OpenClawInferConfig,
    service: Option<Arc<OpenClawGatewayService>>,
    capabilities: HarnessCapabilities,
    name: String,
    /// Cached state_dir path so `state_dir()` can return `Option<&Path>`.
    state_dir_path: Option<PathBuf>,
}

impl OpenClawInferAgent {
    /// Construct a new `OpenClawInferAgent` from config.
    ///
    /// The `ChildProcessRunner` is configured with the binary path and
    /// timeout from the config. The working directory defaults to the
    /// current directory.
    pub fn new(config: OpenClawInferConfig) -> Result<Self, super::config::ConfigError> {
        let cwd = std::env::current_dir().unwrap_or_else(|_| "/tmp".into());
        let runner = ChildProcessRunner::new(&config.binary, cwd).with_timeout(config.timeout);

        let capabilities = HarnessCapabilities {
            one_shot: OneShotMode::CliCommand {
                subcommand: "infer model run",
                output: CliOutput::JsonEnvelope,
            },
            streaming: StreamingMode::None,
            session_resume: SessionResumeMode::None, // infer is stateless
            mcp_passthrough: McpMode::None,          // infer doesn't load tools/MCP
            tool_injection: ToolInjection::Opaque,   // bypass openclaw agent loop
            model_override: true,                    // --model provider/model
            multiplex_safe: true,                    // each call is independent
            cancel: CancelMode::KillChild,
            overhead_p50_ms: 900, // Node.js startup
        };

        let state_dir_path = std::env::var("HOME")
            .ok()
            .map(|h| PathBuf::from(h).join(".openclaw"));

        Ok(Self {
            runner,
            config,
            service: None,
            capabilities,
            name: "openclaw-infer".to_string(),
            state_dir_path,
        })
    }

    /// Attach an optional gateway service for lifecycle management.
    ///
    /// When set, `service()` returns a reference to this service,
    /// allowing the orchestrator to start/stop the OpenClaw gateway
    /// on demand.
    pub fn with_service(mut self, service: Arc<OpenClawGatewayService>) -> Self {
        self.service = Some(service);
        self
    }

    /// Extract the prompt text from an input signal.
    fn extract_prompt(input: &Signal) -> String {
        input.body.as_text().unwrap_or("(empty prompt)").to_string()
    }
}

#[async_trait]
impl Agent for OpenClawInferAgent {
    /// Execute `openclaw infer model run --prompt "..." --json` and
    /// return the parsed result as an `AgentResult`.
    ///
    /// ## Implementation steps
    ///
    /// 1. Extract prompt text from the input `Signal`.
    /// 2. Build argv via `config.build_argv(&prompt)`.
    /// 3. Convert `Vec<String>` to `Vec<&str>` for `run_one_shot`.
    /// 4. Create a mutable `InferEventParser`.
    /// 5. Call `runner.run_one_shot(args, None, &mut parser, None)`.
    /// 6. Process the returned `Vec<HarnessEvent>`.
    /// 7. Estimate token usage via `prompt_chars / 4`, `output_chars / 4`.
    /// 8. Convert to `AgentResult`.
    async fn run(&self, input: &Signal, _ctx: &Context) -> AgentResult {
        let prompt = Self::extract_prompt(input);
        let argv_owned = self.config.build_argv(&prompt);

        // Convert Vec<String> to Vec<&str> for run_one_shot's &[&str] parameter.
        let argv_refs: Vec<&str> = argv_owned.iter().map(|s| s.as_str()).collect();

        let mut parser = InferEventParser::new();

        // Spawn the child process and collect events.
        // - args: &[&str] of CLI arguments after the binary name
        // - stdin_data: None (infer uses --prompt, not stdin)
        // - parser: &mut InferEventParser
        // - cancel: None (orchestrator can wire cancellation in v2)
        let result = self
            .runner
            .run_one_shot(
                &argv_refs,
                None, // stdin_data: openclaw infer uses --prompt flag
                &mut parser,
                None, // cancel: no cancellation token for v1
            )
            .await;

        match result {
            Ok(events) => {
                // Process events returned by ChildProcessRunner.
                // The InferEventParser accumulates stdout and produces
                // events in finalize(), which run_one_shot calls
                // internally. The returned Vec contains all events
                // from parse_stdout_line, parse_stderr_line, and
                // finalize() combined.
                let mut final_output = String::new();
                let mut success = true;
                let mut error_msg = None;

                for event in &events {
                    match event {
                        HarnessEvent::Output(text) => {
                            final_output = text.clone();
                        }
                        HarnessEvent::Error(msg) => {
                            success = false;
                            error_msg = Some(msg.clone());
                        }
                        HarnessEvent::StopReason(reason) if reason == "error" => {
                            success = false;
                        }
                        // Usage, ToolCall, ToolProgress are not used by infer
                        _ => {}
                    }
                }

                // Estimate token usage from character counts since the
                // envelope has no usage field.
                let input_tokens = (prompt.len() as u64 / 4).max(1);
                let out_len = if success {
                    final_output.len()
                } else {
                    error_msg.as_ref().map_or(0, |m| m.len())
                };
                let output_tokens = (out_len as u64 / 4).max(1);

                let output_text = if success {
                    if final_output.is_empty() {
                        "(no output from openclaw infer)".to_string()
                    } else {
                        final_output
                    }
                } else {
                    error_msg.unwrap_or_else(|| "unknown openclaw infer error".to_string())
                };

                let output_signal =
                    derived_output(input, Kind::AgentOutput, Body::text(&output_text)).build();

                let usage = crate::usage::Usage {
                    input_tokens: u32::try_from(input_tokens).unwrap_or(u32::MAX),
                    output_tokens: u32::try_from(output_tokens).unwrap_or(u32::MAX),
                    cache_read_tokens: 0,
                    cache_create_tokens: 0,
                    cost_usd: 0.0,
                    wall_ms: 0,
                };

                let result = if success {
                    AgentResult::ok(output_signal)
                } else {
                    AgentResult::fail(output_signal)
                };

                result.with_usage(usage)
            }
            Err(err) => {
                let msg = format!("openclaw infer spawn failed: {err}");
                let output = derived_output(input, Kind::AgentOutput, Body::text(&msg)).build();
                AgentResult::fail(output)
            }
        }
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn backend_id(&self) -> &'static str {
        "openclaw-infer"
    }

    fn supports_streaming(&self) -> bool {
        false
    }
}

#[async_trait]
impl HarnessAdapter for OpenClawInferAgent {
    fn harness_id(&self) -> &str {
        "openclaw"
    }

    fn transport(&self) -> TransportFlavor {
        TransportFlavor::OneShotJson
    }

    fn capabilities(&self) -> &HarnessCapabilities {
        &self.capabilities
    }

    async fn probe(&self) -> Result<(), ProbeError> {
        probe::probe_openclaw_infer(&self.config).await
    }

    fn state_dir(&self) -> Option<&Path> {
        self.state_dir_path.as_deref()
    }

    fn service(&self) -> Option<&dyn HarnessService> {
        self.service
            .as_ref()
            .map(|s| s.as_ref() as &dyn HarnessService)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capabilities_are_correct() {
        let agent = OpenClawInferAgent::new(OpenClawInferConfig::default()).unwrap();
        let caps = agent.capabilities();
        assert!(matches!(caps.one_shot, OneShotMode::CliCommand {
            subcommand: "infer model run",
            output: CliOutput::JsonEnvelope,
        }));
        assert!(matches!(caps.streaming, StreamingMode::None));
        assert!(matches!(caps.session_resume, SessionResumeMode::None));
        assert!(matches!(caps.mcp_passthrough, McpMode::None));
        assert!(matches!(caps.tool_injection, ToolInjection::Opaque));
        assert!(caps.model_override);
        assert!(caps.multiplex_safe);
        assert!(matches!(caps.cancel, CancelMode::KillChild));
        assert_eq!(caps.overhead_p50_ms, 900);
    }

    #[test]
    fn harness_id_is_openclaw() {
        let agent = OpenClawInferAgent::new(OpenClawInferConfig::default()).unwrap();
        assert_eq!(agent.harness_id(), "openclaw");
    }

    #[test]
    fn transport_is_oneshot_json() {
        let agent = OpenClawInferAgent::new(OpenClawInferConfig::default()).unwrap();
        assert_eq!(agent.transport(), TransportFlavor::OneShotJson);
    }

    #[test]
    fn backend_id_is_openclaw_infer() {
        let agent = OpenClawInferAgent::new(OpenClawInferConfig::default()).unwrap();
        assert_eq!(agent.backend_id(), "openclaw-infer");
    }

    #[test]
    fn does_not_support_streaming() {
        let agent = OpenClawInferAgent::new(OpenClawInferConfig::default()).unwrap();
        assert!(!agent.supports_streaming());
    }

    #[test]
    fn name_is_openclaw_infer() {
        let agent = OpenClawInferAgent::new(OpenClawInferConfig::default()).unwrap();
        assert_eq!(agent.name(), "openclaw-infer");
    }
}

#[cfg(test)]
mod fixture_tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Create a fake `openclaw` binary (shell script) that prints
    /// the given fixture content to stdout and exits with the given code.
    fn fake_openclaw_script(fixture: &str, exit_code: i32) -> NamedTempFile {
        let mut script = NamedTempFile::new().unwrap();
        writeln!(
            script,
            "#!/bin/sh\ncat <<'FIXTURE_EOF'\n{fixture}\nFIXTURE_EOF\nexit {exit_code}"
        )
        .unwrap();
        // Make executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = script.as_file().metadata().unwrap().permissions();
            perms.set_mode(0o755);
            script.as_file().set_permissions(perms).unwrap();
        }
        script
    }

    #[tokio::test]
    async fn fixture_model_run_basic() {
        let fixture = include_str!("../../tests/fixtures/openclaw/infer/model_run_basic.json");
        let script = fake_openclaw_script(fixture, 0);

        let config = OpenClawInferConfig {
            binary: script.path().as_os_str().to_owned(),
            ..Default::default()
        };
        let agent = OpenClawInferAgent::new(config).unwrap();

        let prompt = roko_core::Signal::builder(roko_core::Kind::Prompt)
            .body(roko_core::Body::text("What is the capital of France?"))
            .build();
        let result = agent.run(&prompt, &roko_core::Context::now()).await;

        assert!(result.success);
        let text = result.output.body.as_text().unwrap();
        assert!(text.contains("Paris"));
    }

    #[tokio::test]
    async fn fixture_model_run_auth_error() {
        let fixture = include_str!("../../tests/fixtures/openclaw/infer/model_run_auth_error.json");
        let script = fake_openclaw_script(fixture, 1);

        let config = OpenClawInferConfig {
            binary: script.path().as_os_str().to_owned(),
            ..Default::default()
        };
        let agent = OpenClawInferAgent::new(config).unwrap();

        let prompt = roko_core::Signal::builder(roko_core::Kind::Prompt)
            .body(roko_core::Body::text("hello"))
            .build();
        let result = agent.run(&prompt, &roko_core::Context::now()).await;

        assert!(!result.success);
    }

    #[tokio::test]
    async fn fixture_model_run_empty_output() {
        let fixture =
            include_str!("../../tests/fixtures/openclaw/infer/model_run_empty_output.json");
        let script = fake_openclaw_script(fixture, 0);

        let config = OpenClawInferConfig {
            binary: script.path().as_os_str().to_owned(),
            ..Default::default()
        };
        let agent = OpenClawInferAgent::new(config).unwrap();

        let prompt = roko_core::Signal::builder(roko_core::Kind::Prompt)
            .body(roko_core::Body::text("hello"))
            .build();
        let result = agent.run(&prompt, &roko_core::Context::now()).await;

        // Success but with "(no text output)" placeholder
        assert!(result.success);
    }

    #[tokio::test]
    async fn fixture_usage_estimation() {
        let fixture = include_str!("../../tests/fixtures/openclaw/infer/model_run_basic.json");
        let script = fake_openclaw_script(fixture, 0);

        let config = OpenClawInferConfig {
            binary: script.path().as_os_str().to_owned(),
            ..Default::default()
        };
        let agent = OpenClawInferAgent::new(config).unwrap();

        let prompt = roko_core::Signal::builder(roko_core::Kind::Prompt)
            .body(roko_core::Body::text("What is the capital of France?"))
            .build();
        let result = agent.run(&prompt, &roko_core::Context::now()).await;

        // Verify usage was estimated
        assert!(result.usage.input_tokens > 0, "input_tokens should be > 0");
        assert!(
            result.usage.output_tokens > 0,
            "output_tokens should be > 0"
        );
    }
}
