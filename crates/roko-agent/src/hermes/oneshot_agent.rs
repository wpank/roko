//! Hermes one-shot CLI agent.
//!
//! Spawns `hermes` CLI commands via [`ChildProcessRunner`] for single-turn
//! prompt dispatch. Two flavors:
//!
//! - [`HermesFlavor::ChatQuiet`] -- `hermes chat -q "<prompt>" -Q --source roko`
//! - [`HermesFlavor::Z`] -- `hermes -z "<prompt>"` (minimal, no extra flags)

use std::path::Path;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use roko_core::{Context, Signal};

use crate::agent::{Agent, AgentResult};
use crate::harness::{
    CancelMode, ChildProcessRunner, CliOutput, HarnessCapabilities, HarnessEvent, McpMode,
    OneShotMode, ProbeError, SessionResumeMode, StreamingMode, ToolInjection, TransportFlavor,
    harness_events_to_agent_result,
};
use crate::harness::{EventParser, HarnessAdapter};

// ---------------------------------------------------------------------------
// HermesFlavor
// ---------------------------------------------------------------------------

/// Which CLI invocation style to use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HermesFlavor {
    /// `hermes chat -q "<prompt>" -Q --source roko`
    ChatQuiet,
    /// `hermes -z "<prompt>"`
    Z,
}

// ---------------------------------------------------------------------------
// HermesOneShotConfig
// ---------------------------------------------------------------------------

/// Configuration for the Hermes one-shot agent.
#[derive(Clone, Debug)]
pub struct HermesOneShotConfig {
    /// Path or name of the Hermes binary.
    pub binary: String,
    /// Which CLI invocation flavor to use.
    pub flavor: HermesFlavor,
    /// Extra CLI arguments appended after the prompt (ChatQuiet only).
    pub extra_args: Vec<String>,
    /// Source tag for attribution.
    pub source_tag: String,
    /// Optional model override (passed as `--model <model>`).
    pub model_override: Option<String>,
    /// Timeout for the subprocess.
    pub timeout: Duration,
}

impl Default for HermesOneShotConfig {
    fn default() -> Self {
        Self {
            binary: "hermes".to_string(),
            flavor: HermesFlavor::ChatQuiet,
            extra_args: vec![
                "--source".to_string(),
                "roko".to_string(),
                "--ignore-user-config".to_string(),
            ],
            source_tag: "roko".to_string(),
            model_override: None,
            timeout: Duration::from_secs(120),
        }
    }
}

// ---------------------------------------------------------------------------
// Parsers
// ---------------------------------------------------------------------------

/// Parser for `hermes -z` output: accumulates all stdout lines.
///
/// On `finalize()`, joins accumulated lines with `"\n"` and emits a single
/// [`HarnessEvent::Output`] followed by [`HarnessEvent::StopReason`].
struct HermesZParser {
    lines: Vec<String>,
}

impl HermesZParser {
    fn new() -> Self {
        Self { lines: Vec::new() }
    }
}

impl EventParser for HermesZParser {
    fn parse_stdout_line(&mut self, line: &str) -> Vec<HarnessEvent> {
        self.lines.push(line.to_string());
        vec![]
    }

    fn finalize(&mut self) -> Vec<HarnessEvent> {
        let joined = self.lines.join("\n");
        vec![
            HarnessEvent::Output(joined),
            HarnessEvent::StopReason("stop".into()),
        ]
    }
}

/// Parser for `hermes chat -q ... -Q` output: accumulates stdout and stderr.
///
/// On `finalize()`:
/// - Joins stdout lines with `"\n"` and emits [`HarnessEvent::Output`].
/// - If stderr is non-empty, emits [`HarnessEvent::Error`].
/// - Emits [`HarnessEvent::StopReason`].
struct HermesChatQuietParser {
    stdout_lines: Vec<String>,
    stderr_lines: Vec<String>,
}

impl HermesChatQuietParser {
    fn new() -> Self {
        Self {
            stdout_lines: Vec::new(),
            stderr_lines: Vec::new(),
        }
    }
}

impl EventParser for HermesChatQuietParser {
    fn parse_stdout_line(&mut self, line: &str) -> Vec<HarnessEvent> {
        self.stdout_lines.push(line.to_string());
        vec![]
    }

    fn parse_stderr_line(&mut self, line: &str) -> Vec<HarnessEvent> {
        self.stderr_lines.push(line.to_string());
        vec![]
    }

    fn finalize(&mut self) -> Vec<HarnessEvent> {
        let mut events = Vec::new();

        let joined_stdout = self.stdout_lines.join("\n");
        events.push(HarnessEvent::Output(joined_stdout));

        if !self.stderr_lines.is_empty() {
            let joined_stderr = self.stderr_lines.join("\n");
            events.push(HarnessEvent::Error(joined_stderr));
        }

        events.push(HarnessEvent::StopReason("stop".into()));
        events
    }
}

// ---------------------------------------------------------------------------
// HermesOneShotAgent
// ---------------------------------------------------------------------------

/// Hermes one-shot CLI agent.
///
/// Uses [`ChildProcessRunner`] to spawn `hermes` CLI one-shot commands
/// and collects the output via flavor-specific parsers.
pub struct HermesOneShotAgent {
    runner: ChildProcessRunner,
    config: HermesOneShotConfig,
    capabilities: HarnessCapabilities,
    name: String,
}

impl HermesOneShotAgent {
    /// Create a new Hermes one-shot agent from config.
    #[must_use]
    pub fn new(config: HermesOneShotConfig) -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let runner = ChildProcessRunner::new(&config.binary, &cwd)
            .with_timeout(config.timeout)
            .with_name("hermes-oneshot");

        let capabilities = Self::build_capabilities(config.flavor);

        Self {
            runner,
            config,
            capabilities,
            name: "hermes-oneshot".to_string(),
        }
    }

    /// Build the argument vector for the CLI invocation.
    fn build_argv(&self, prompt: &str) -> Vec<String> {
        match self.config.flavor {
            HermesFlavor::Z => {
                // Z flavor: just "-z" and the prompt, no extra flags.
                vec!["-z".to_string(), prompt.to_string()]
            }
            HermesFlavor::ChatQuiet => {
                let mut argv = vec![
                    "chat".to_string(),
                    "-q".to_string(),
                    prompt.to_string(),
                    "-Q".to_string(),
                ];

                // Append extra_args (e.g., --source roko --ignore-user-config).
                argv.extend(self.config.extra_args.iter().cloned());

                // Append model override if set.
                if let Some(ref model) = self.config.model_override {
                    argv.push("--model".to_string());
                    argv.push(model.clone());
                }

                argv
            }
        }
    }

    /// Build capabilities based on the flavor.
    fn build_capabilities(flavor: HermesFlavor) -> HarnessCapabilities {
        match flavor {
            HermesFlavor::ChatQuiet => HarnessCapabilities {
                one_shot: OneShotMode::CliCommand {
                    subcommand: "chat -q",
                    output: CliOutput::PlainText,
                },
                streaming: StreamingMode::None,
                session_resume: SessionResumeMode::None,
                mcp_passthrough: McpMode::None,
                tool_injection: ToolInjection::Opaque,
                model_override: true,
                multiplex_safe: true,
                cancel: CancelMode::KillChild,
                overhead_p50_ms: 600,
            },
            HermesFlavor::Z => HarnessCapabilities {
                one_shot: OneShotMode::CliCommand {
                    subcommand: "hermes -z",
                    output: CliOutput::PlainText,
                },
                streaming: StreamingMode::None,
                session_resume: SessionResumeMode::None,
                mcp_passthrough: McpMode::None,
                tool_injection: ToolInjection::Opaque,
                model_override: false,
                multiplex_safe: true,
                cancel: CancelMode::KillChild,
                overhead_p50_ms: 400,
            },
        }
    }
}

#[async_trait]
impl Agent for HermesOneShotAgent {
    async fn run(&self, input: &Signal, _ctx: &Context) -> AgentResult {
        let started = Instant::now();

        let prompt = match input.body.as_text() {
            Ok(s) => s.to_string(),
            Err(e) => {
                let output = input
                    .derive(
                        roko_core::Kind::AgentOutput,
                        roko_core::Body::text(format!("failed to extract prompt: {e}")),
                    )
                    .provenance(roko_core::Provenance::agent(&self.name))
                    .tag("agent", &self.name)
                    .tag("failed", "true")
                    .build();
                return AgentResult::fail(output);
            }
        };

        let argv = self.build_argv(&prompt);
        let argv_refs: Vec<&str> = argv.iter().map(String::as_str).collect();

        let mut parser: Box<dyn EventParser> = match self.config.flavor {
            HermesFlavor::Z => Box::new(HermesZParser::new()),
            HermesFlavor::ChatQuiet => Box::new(HermesChatQuietParser::new()),
        };

        let events = match self
            .runner
            .run_one_shot(&argv_refs, None, parser.as_mut(), None)
            .await
        {
            Ok(events) => events,
            Err(e) => {
                let output = input
                    .derive(
                        roko_core::Kind::AgentOutput,
                        roko_core::Body::text(format!("hermes oneshot error: {e}")),
                    )
                    .provenance(roko_core::Provenance::agent(&self.name))
                    .tag("agent", &self.name)
                    .tag("failed", "true")
                    .build();
                return AgentResult::fail(output);
            }
        };

        let wall_ms = started.elapsed().as_millis() as u64;
        harness_events_to_agent_result(&events, input, &self.name, wall_ms)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn backend_id(&self) -> &'static str {
        "hermes-oneshot"
    }

    fn supports_streaming(&self) -> bool {
        false
    }
}

#[async_trait]
impl HarnessAdapter for HermesOneShotAgent {
    fn harness_id(&self) -> &str {
        "hermes"
    }

    fn transport(&self) -> TransportFlavor {
        TransportFlavor::OneShotPlain
    }

    fn capabilities(&self) -> &HarnessCapabilities {
        &self.capabilities
    }

    async fn probe(&self) -> Result<(), ProbeError> {
        crate::hermes::probe::probe_hermes(&self.config.binary, None)
            .await
            .map(|_| ())
    }

    fn state_dir(&self) -> Option<&Path> {
        None
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Config and flavor tests -------------------------------------------

    #[test]
    fn flavor_chat_quiet_default() {
        let config = HermesOneShotConfig::default();
        assert_eq!(config.flavor, HermesFlavor::ChatQuiet);
    }

    #[test]
    fn flavor_z() {
        let config = HermesOneShotConfig {
            flavor: HermesFlavor::Z,
            ..Default::default()
        };
        assert_eq!(config.flavor, HermesFlavor::Z);
    }

    #[test]
    fn default_config() {
        let config = HermesOneShotConfig::default();
        assert_eq!(config.binary, "hermes");
        assert_eq!(config.flavor, HermesFlavor::ChatQuiet);
        assert_eq!(
            config.extra_args,
            vec!["--source", "roko", "--ignore-user-config"]
        );
        assert_eq!(config.source_tag, "roko");
        assert!(config.model_override.is_none());
        assert_eq!(config.timeout, Duration::from_secs(120));
    }

    // ---- build_argv tests --------------------------------------------------

    #[test]
    fn build_argv_chat_quiet_basic() {
        let config = HermesOneShotConfig::default();
        let agent = HermesOneShotAgent::new(config);
        let argv = agent.build_argv("hello world");
        assert_eq!(
            argv,
            vec![
                "chat",
                "-q",
                "hello world",
                "-Q",
                "--source",
                "roko",
                "--ignore-user-config",
            ]
        );
    }

    #[test]
    fn build_argv_chat_quiet_with_model() {
        let config = HermesOneShotConfig {
            model_override: Some("claude-sonnet-4-20250514".to_string()),
            ..Default::default()
        };
        let agent = HermesOneShotAgent::new(config);
        let argv = agent.build_argv("test prompt");
        assert_eq!(
            argv,
            vec![
                "chat",
                "-q",
                "test prompt",
                "-Q",
                "--source",
                "roko",
                "--ignore-user-config",
                "--model",
                "claude-sonnet-4-20250514",
            ]
        );
    }

    #[test]
    fn build_argv_z_no_extra_flags() {
        let config = HermesOneShotConfig {
            flavor: HermesFlavor::Z,
            ..Default::default()
        };
        let agent = HermesOneShotAgent::new(config);
        let argv = agent.build_argv("quick question");

        // Z flavor must only have "-z" and the prompt -- no --source,
        // no --ignore-user-config, no other extra flags.
        assert_eq!(argv, vec!["-z", "quick question"]);
        assert!(!argv.contains(&"--source".to_string()));
        assert!(!argv.contains(&"--ignore-user-config".to_string()));
    }

    #[test]
    fn build_argv_z_ignores_model_override() {
        // Z flavor does not support --model; even if model_override is set
        // in the config, the argv must NOT contain it.
        let config = HermesOneShotConfig {
            flavor: HermesFlavor::Z,
            model_override: Some("claude-opus-4-20250514".to_string()),
            ..Default::default()
        };
        let agent = HermesOneShotAgent::new(config);
        let argv = agent.build_argv("test");

        assert_eq!(argv, vec!["-z", "test"]);
        assert!(!argv.contains(&"--model".to_string()));
        assert!(!argv.contains(&"claude-opus-4-20250514".to_string()));
    }

    #[test]
    fn build_argv_chat_quiet_custom_extra_args() {
        let config = HermesOneShotConfig {
            extra_args: vec!["--verbose".to_string(), "--no-cache".to_string()],
            model_override: None,
            ..Default::default()
        };
        let agent = HermesOneShotAgent::new(config);
        let argv = agent.build_argv("prompt");
        assert_eq!(
            argv,
            vec!["chat", "-q", "prompt", "-Q", "--verbose", "--no-cache"]
        );
    }

    #[test]
    fn build_argv_chat_quiet_empty_extra_args() {
        let config = HermesOneShotConfig {
            extra_args: vec![],
            model_override: None,
            ..Default::default()
        };
        let agent = HermesOneShotAgent::new(config);
        let argv = agent.build_argv("hello");
        assert_eq!(argv, vec!["chat", "-q", "hello", "-Q"]);
    }

    #[test]
    fn build_argv_chat_quiet_empty_prompt() {
        let config = HermesOneShotConfig::default();
        let agent = HermesOneShotAgent::new(config);
        let argv = agent.build_argv("");
        // Empty prompt is still passed as the argument
        assert_eq!(argv[2], "");
    }

    // ---- Parser tests ------------------------------------------------------

    #[test]
    fn z_parser_basic() {
        let mut parser = HermesZParser::new();
        assert!(parser.parse_stdout_line("line one").is_empty());
        assert!(parser.parse_stdout_line("line two").is_empty());

        let events = parser.finalize();
        assert_eq!(events.len(), 2);

        match &events[0] {
            HarnessEvent::Output(text) => assert_eq!(text, "line one\nline two"),
            other => panic!("expected Output, got {other:?}"),
        }
        match &events[1] {
            HarnessEvent::StopReason(reason) => assert_eq!(reason, "stop"),
            other => panic!("expected StopReason, got {other:?}"),
        }
    }

    #[test]
    fn z_parser_empty() {
        let mut parser = HermesZParser::new();
        let events = parser.finalize();
        assert_eq!(events.len(), 2);

        match &events[0] {
            HarnessEvent::Output(text) => assert_eq!(text, ""),
            other => panic!("expected empty Output, got {other:?}"),
        }
        match &events[1] {
            HarnessEvent::StopReason(reason) => assert_eq!(reason, "stop"),
            other => panic!("expected StopReason, got {other:?}"),
        }
    }

    #[test]
    fn chat_quiet_parser_basic() {
        let mut parser = HermesChatQuietParser::new();
        assert!(parser.parse_stdout_line("response line 1").is_empty());
        assert!(parser.parse_stdout_line("response line 2").is_empty());

        let events = parser.finalize();
        assert_eq!(events.len(), 2); // Output + StopReason, no stderr

        match &events[0] {
            HarnessEvent::Output(text) => {
                assert_eq!(text, "response line 1\nresponse line 2");
            }
            other => panic!("expected Output, got {other:?}"),
        }
        match &events[1] {
            HarnessEvent::StopReason(reason) => assert_eq!(reason, "stop"),
            other => panic!("expected StopReason, got {other:?}"),
        }
    }

    #[test]
    fn chat_quiet_parser_with_stderr() {
        let mut parser = HermesChatQuietParser::new();
        assert!(parser.parse_stdout_line("output").is_empty());
        assert!(parser.parse_stderr_line("warn: something").is_empty());
        assert!(parser.parse_stderr_line("warn: another").is_empty());

        let events = parser.finalize();
        assert_eq!(events.len(), 3); // Output + Error + StopReason

        match &events[0] {
            HarnessEvent::Output(text) => assert_eq!(text, "output"),
            other => panic!("expected Output, got {other:?}"),
        }
        match &events[1] {
            HarnessEvent::Error(text) => {
                assert_eq!(text, "warn: something\nwarn: another");
            }
            other => panic!("expected Error, got {other:?}"),
        }
        match &events[2] {
            HarnessEvent::StopReason(reason) => assert_eq!(reason, "stop"),
            other => panic!("expected StopReason, got {other:?}"),
        }
    }

    #[test]
    fn z_parser_single_line() {
        let mut parser = HermesZParser::new();
        assert!(parser.parse_stdout_line("only one line").is_empty());
        let events = parser.finalize();
        assert_eq!(events.len(), 2);
        match &events[0] {
            HarnessEvent::Output(text) => assert_eq!(text, "only one line"),
            other => panic!("expected Output, got {other:?}"),
        }
    }

    #[test]
    fn z_parser_multiline_preserves_order() {
        let mut parser = HermesZParser::new();
        for i in 0..5 {
            assert!(parser.parse_stdout_line(&format!("line {i}")).is_empty());
        }
        let events = parser.finalize();
        match &events[0] {
            HarnessEvent::Output(text) => {
                assert_eq!(text, "line 0\nline 1\nline 2\nline 3\nline 4");
            }
            other => panic!("expected Output, got {other:?}"),
        }
    }

    #[test]
    fn chat_quiet_parser_empty() {
        let mut parser = HermesChatQuietParser::new();
        let events = parser.finalize();
        // No stderr => Output + StopReason only
        assert_eq!(events.len(), 2);
        match &events[0] {
            HarnessEvent::Output(text) => assert_eq!(text, ""),
            other => panic!("expected empty Output, got {other:?}"),
        }
        match &events[1] {
            HarnessEvent::StopReason(reason) => assert_eq!(reason, "stop"),
            other => panic!("expected StopReason, got {other:?}"),
        }
    }

    #[test]
    fn chat_quiet_parser_only_stderr() {
        let mut parser = HermesChatQuietParser::new();
        assert!(
            parser
                .parse_stderr_line("error: something broke")
                .is_empty()
        );
        let events = parser.finalize();
        // Empty stdout output + error + stop reason
        assert_eq!(events.len(), 3);
        match &events[0] {
            HarnessEvent::Output(text) => assert_eq!(text, ""),
            other => panic!("expected empty Output, got {other:?}"),
        }
        match &events[1] {
            HarnessEvent::Error(text) => assert_eq!(text, "error: something broke"),
            other => panic!("expected Error, got {other:?}"),
        }
    }

    // ---- Capability tests --------------------------------------------------

    #[test]
    fn capabilities_chat_quiet() {
        let config = HermesOneShotConfig::default(); // ChatQuiet
        let agent = HermesOneShotAgent::new(config);
        let caps = agent.capabilities();

        assert!(matches!(
            caps.one_shot,
            OneShotMode::CliCommand {
                subcommand: "chat -q",
                output: CliOutput::PlainText,
            }
        ));
        assert!(matches!(caps.streaming, StreamingMode::None));
        assert!(matches!(caps.session_resume, SessionResumeMode::None));
        assert!(matches!(caps.mcp_passthrough, McpMode::None));
        assert!(matches!(caps.tool_injection, ToolInjection::Opaque));
        assert!(caps.model_override);
        assert!(caps.multiplex_safe);
        assert!(matches!(caps.cancel, CancelMode::KillChild));
        assert_eq!(caps.overhead_p50_ms, 600);
    }

    #[test]
    fn capabilities_z() {
        let config = HermesOneShotConfig {
            flavor: HermesFlavor::Z,
            ..Default::default()
        };
        let agent = HermesOneShotAgent::new(config);
        let caps = agent.capabilities();

        assert!(matches!(
            caps.one_shot,
            OneShotMode::CliCommand {
                subcommand: "hermes -z",
                output: CliOutput::PlainText,
            }
        ));
        assert!(matches!(caps.streaming, StreamingMode::None));
        assert!(matches!(caps.session_resume, SessionResumeMode::None));
        assert!(matches!(caps.mcp_passthrough, McpMode::None));
        assert!(matches!(caps.tool_injection, ToolInjection::Opaque));
        assert!(!caps.model_override); // Z does not support --model
        assert!(caps.multiplex_safe);
        assert!(matches!(caps.cancel, CancelMode::KillChild));
        assert_eq!(caps.overhead_p50_ms, 400);
    }

    // ---- Harness metadata tests --------------------------------------------

    #[test]
    fn harness_metadata() {
        let config = HermesOneShotConfig::default();
        let agent = HermesOneShotAgent::new(config);

        assert_eq!(agent.harness_id(), "hermes");
        assert_eq!(agent.transport(), TransportFlavor::OneShotPlain);
        assert_eq!(agent.backend_id(), "hermes-oneshot");
        assert_eq!(agent.name(), "hermes-oneshot");
        assert!(!agent.supports_streaming());
    }
}
