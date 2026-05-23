//! Capability negotiation types for harness adapters.
//!
//! [`TransportFlavor`] classifies how roko talks to the harness.
//! [`HarnessCapabilities`] declares what a harness can do at a given transport.
//! [`validate_for_task()`] checks capability-task compatibility at dispatch time.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Duration;

// ---- Transport flavors (5-tier) --------------------------------------------

/// Which transport tier roko uses to talk to the harness.
///
/// A single harness (e.g. Hermes) may expose multiple transports; each
/// gets its own adapter and `TransportFlavor` value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportFlavor {
    /// OpenAI-compatible HTTP API (Tier 1).
    #[serde(rename = "http_openai")]
    HttpOpenAi,
    /// OpenAI Responses API over HTTP (Tier 1 variant).
    HttpResponses,
    /// One-shot CLI invocation with JSON envelope on stdout (Tier 2).
    #[serde(rename = "oneshot_json")]
    OneShotJson,
    /// One-shot CLI invocation with plain-text output (Tier 2).
    #[serde(rename = "oneshot_plain")]
    OneShotPlain,
    /// ACP over stdio (Tier 3).
    #[serde(rename = "acp_stdio")]
    AcpStdio,
    /// TUI JSON-RPC over stdio (Tier 4, deferred to v2).
    #[serde(rename = "tui_jsonrpc")]
    TuiJsonRpc,
    /// MCP server mode -- harness acts as the MCP server (Tier 5).
    #[serde(rename = "mcp_server")]
    McpServer,
}

impl fmt::Display for TransportFlavor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HttpOpenAi => f.write_str("http_openai"),
            Self::HttpResponses => f.write_str("http_responses"),
            Self::OneShotJson => f.write_str("oneshot_json"),
            Self::OneShotPlain => f.write_str("oneshot_plain"),
            Self::AcpStdio => f.write_str("acp_stdio"),
            Self::TuiJsonRpc => f.write_str("tui_jsonrpc"),
            Self::McpServer => f.write_str("mcp_server"),
        }
    }
}

impl TransportFlavor {
    /// Loose string-to-enum conversion (for config parsing).
    #[must_use]
    pub fn from_str_loose(s: &str) -> Option<Self> {
        match s {
            "http_openai" | "http-openai" | "HttpOpenAi" => Some(Self::HttpOpenAi),
            "http_responses" | "http-responses" | "HttpResponses" => Some(Self::HttpResponses),
            "oneshot_json" | "oneshot-json" | "OneShotJson" => Some(Self::OneShotJson),
            "oneshot_plain" | "oneshot-plain" | "OneShotPlain" => Some(Self::OneShotPlain),
            "acp_stdio" | "acp-stdio" | "AcpStdio" => Some(Self::AcpStdio),
            "tui_jsonrpc" | "tui-jsonrpc" | "TuiJsonRpc" => Some(Self::TuiJsonRpc),
            "mcp_server" | "mcp-server" | "McpServer" => Some(Self::McpServer),
            _ => None,
        }
    }
}

// ---- Capability sub-enums --------------------------------------------------

/// How a single prompt is delivered to the harness.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OneShotMode {
    /// HTTP POST with the prompt in a request body.
    HttpJson { endpoint: &'static str },
    /// CLI subcommand prefix prepended before the prompt argument.
    CliCommand {
        subcommand: &'static str,
        output: CliOutput,
    },
    /// JSON-RPC over stdio (harness-specific, not ACP).
    StdioJsonRpc,
    /// ACP `session/prompt` request over stdio.
    Acp,
    /// PTY automation -- drive an interactive REPL.
    PtyAutomation,
    /// Genuinely interactive-only -- adapter refuses non-interactive dispatch.
    Unsupported,
}

/// How to parse the stdout output of a one-shot CLI invocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CliOutput {
    /// Plain UTF-8 text, no framing.
    PlainText,
    /// Single JSON object on stdout.
    JsonEnvelope,
    /// Newline-delimited JSON lines.
    NdJson,
    /// Claude-style `stream-json` format (JSON lines with event types).
    StreamJson,
}

/// What kind of streaming the harness supports at this transport.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StreamingMode {
    /// OpenAI-style SSE `chat.completion.chunk` events.
    SseChatCompletions,
    /// OpenAI Responses API SSE events.
    SseResponsesApi,
    /// Newline-delimited JSON over stdout.
    NdJson,
    /// Raw token bytes -- no event boundaries.
    RawTokens,
    /// No streaming at this transport.
    None,
}

/// Whether and how a session can be resumed by identifier.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionResumeMode {
    /// `previous_response_id` chaining (Responses API).
    PreviousResponseId,
    /// Named conversation parameter.
    Conversation,
    /// CLI flag (e.g. `"--resume"`, `"--continue"`).
    CliFlag(&'static str),
    /// ACP `session/load` request.
    Acp,
    /// Not supported.
    None,
}

/// How MCP servers are made available to the harness.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum McpMode {
    /// Harness accepts MCP servers in the per-call `tools` array.
    PerCall,
    /// Harness reads MCP server configs from a file passed via flag.
    ConfigFile(&'static str),
    /// MCP only configured server-side on the harness -- roko cannot inject.
    ServerOnly,
    /// No MCP at this transport.
    None,
}

/// How roko-side tools are made available to the harness's tool loop.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolInjection {
    /// Tools listed in the per-request body (like OpenAI's `tools` array).
    PerCallTools,
    /// Tools defined ahead of time in a config file.
    ConfigFile,
    /// Tools come from MCP only.
    McpOnly,
    /// Adapter does not see or inject tools.
    Opaque,
}

/// How mid-turn cancellation works.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CancelMode {
    /// HTTP cancel endpoint (e.g. `/v1/runs/{id}/stop`).
    HttpEndpoint(&'static str),
    /// Subprocess SIGTERM / Drop.
    KillChild,
    /// ACP `session/cancel` request.
    AcpCancel,
    /// Not cancellable mid-turn.
    None,
}

// ---- Capabilities struct ---------------------------------------------------

/// Static description of what a harness can do at a given transport.
///
/// Conservative defaults -- when in doubt, declare unsupported and let
/// the orchestrator route the task elsewhere.
#[derive(Clone, Debug)]
pub struct HarnessCapabilities {
    /// How a single prompt is delivered.
    pub one_shot: OneShotMode,
    /// What kind of streaming, if any.
    pub streaming: StreamingMode,
    /// Can a session be resumed by id?
    pub session_resume: SessionResumeMode,
    /// How MCP is reached.
    pub mcp_passthrough: McpMode,
    /// How roko-side tools are made available.
    pub tool_injection: ToolInjection,
    /// Can the caller override the model per request?
    pub model_override: bool,
    /// Can multiple roko-side agents share this adapter concurrently?
    pub multiplex_safe: bool,
    /// Does the harness support inline cancellation?
    pub cancel: CancelMode,
    /// Approximate p50 latency overhead introduced by the harness layer.
    pub overhead_p50_ms: u32,
}

impl Default for HarnessCapabilities {
    fn default() -> Self {
        Self {
            one_shot: OneShotMode::Unsupported,
            streaming: StreamingMode::None,
            session_resume: SessionResumeMode::None,
            mcp_passthrough: McpMode::None,
            tool_injection: ToolInjection::Opaque,
            model_override: false,
            multiplex_safe: false,
            cancel: CancelMode::None,
            overhead_p50_ms: 0,
        }
    }
}

// ---- Task requirements -----------------------------------------------------

/// Requirements a task imposes on the dispatched harness adapter.
#[derive(Clone, Debug, Default)]
pub struct HarnessTaskRequirements {
    /// Task requires tool execution support.
    pub needs_tools: bool,
    /// Task requires real-time streaming output.
    pub needs_streaming: bool,
    /// Task requires MCP tool passthrough.
    pub needs_mcp: bool,
    /// Task requires session resume support.
    pub needs_session_resume: bool,
    /// Task requires mid-turn cancellation.
    pub needs_cancel: bool,
    /// Maximum timeout for the task.
    pub max_timeout: Option<Duration>,
    /// Task tolerates PTY overhead.
    pub allows_pty_overhead: bool,
}

// ---- Validation ------------------------------------------------------------

/// Error returned by [`validate_for_task`] when a capability mismatch
/// is detected.
#[derive(Debug, Clone)]
pub struct CapabilityMismatch {
    /// Adapter identifier.
    pub adapter: String,
    /// Transport the adapter speaks.
    pub transport: TransportFlavor,
    /// What the task needed.
    pub need: &'static str,
    /// Actionable hint for the operator.
    pub hint: &'static str,
}

impl fmt::Display for CapabilityMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}/{} cannot meet task requirement: {} -- {}",
            self.adapter, self.transport, self.need, self.hint
        )
    }
}

impl std::error::Error for CapabilityMismatch {}

/// Validate that an adapter can serve a given task.
///
/// Called by the dispatch resolver before dispatch. On mismatch, returns
/// a [`CapabilityMismatch`] carrying the specific reason and an
/// actionable hint.
pub fn validate_for_task(
    adapter: &dyn super::HarnessAdapter,
    task: &HarnessTaskRequirements,
) -> Result<(), CapabilityMismatch> {
    let c = adapter.capabilities();

    if task.needs_tools
        && matches!(c.tool_injection, ToolInjection::Opaque)
        && matches!(c.mcp_passthrough, McpMode::None | McpMode::ServerOnly)
    {
        return Err(CapabilityMismatch {
            adapter: adapter.harness_id().to_string(),
            transport: adapter.transport(),
            need: "tools",
            hint: "use a transport with PerCallTools, ConfigFile, or McpOnly tool injection",
        });
    }
    if task.needs_streaming && matches!(c.streaming, StreamingMode::None) {
        return Err(CapabilityMismatch {
            adapter: adapter.harness_id().to_string(),
            transport: adapter.transport(),
            need: "streaming",
            hint: "use a transport with SseChatCompletions or NdJson streaming",
        });
    }
    if task.needs_mcp && matches!(c.mcp_passthrough, McpMode::None | McpMode::ServerOnly) {
        return Err(CapabilityMismatch {
            adapter: adapter.harness_id().to_string(),
            transport: adapter.transport(),
            need: "mcp_passthrough",
            hint: "use a transport that supports per-call or config-file MCP injection",
        });
    }
    if task.needs_session_resume && matches!(c.session_resume, SessionResumeMode::None) {
        return Err(CapabilityMismatch {
            adapter: adapter.harness_id().to_string(),
            transport: adapter.transport(),
            need: "session_resume",
            hint: "use a transport that supports session resume (PreviousResponseId, CliFlag, or Acp)",
        });
    }
    if task.needs_cancel && matches!(c.cancel, CancelMode::None) {
        return Err(CapabilityMismatch {
            adapter: adapter.harness_id().to_string(),
            transport: adapter.transport(),
            need: "cancel",
            hint: "use a transport that supports cancellation (HttpEndpoint, KillChild, or AcpCancel)",
        });
    }
    if matches!(c.one_shot, OneShotMode::PtyAutomation) && !task.allows_pty_overhead {
        return Err(CapabilityMismatch {
            adapter: adapter.harness_id().to_string(),
            transport: adapter.transport(),
            need: "non_pty",
            hint: "this adapter requires PTY automation; set allows_pty_overhead = true or use a different transport",
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- MockAdapter for validate_for_task tests ----------------------------

    use async_trait::async_trait;

    struct MockAdapter {
        id: &'static str,
        flavor: TransportFlavor,
        caps: HarnessCapabilities,
    }

    impl MockAdapter {
        fn new(id: &'static str, flavor: TransportFlavor, caps: HarnessCapabilities) -> Self {
            Self { id, flavor, caps }
        }

        /// Adapter with fully-capable defaults: per-call tools, SSE streaming,
        /// MCP per-call, session resume via CLI flag, and KillChild cancel.
        fn capable(id: &'static str) -> Self {
            Self::new(id, TransportFlavor::HttpOpenAi, HarnessCapabilities {
                one_shot: OneShotMode::HttpJson {
                    endpoint: "/v1/chat/completions",
                },
                streaming: StreamingMode::SseChatCompletions,
                session_resume: SessionResumeMode::CliFlag("--resume"),
                mcp_passthrough: McpMode::PerCall,
                tool_injection: ToolInjection::PerCallTools,
                model_override: true,
                multiplex_safe: true,
                cancel: CancelMode::KillChild,
                overhead_p50_ms: 10,
            })
        }

        /// Adapter with the conservative default capabilities.
        fn conservative(id: &'static str) -> Self {
            Self::new(
                id,
                TransportFlavor::OneShotPlain,
                HarnessCapabilities::default(),
            )
        }
    }

    // Minimal Agent stub — validate_for_task only calls harness_id(),
    // transport() and capabilities(), so run() is never reached in tests.
    #[async_trait]
    impl crate::agent::Agent for MockAdapter {
        fn name(&self) -> &str {
            self.id
        }

        async fn run(
            &self,
            _input: &roko_core::Signal,
            _ctx: &roko_core::Context,
        ) -> crate::agent::AgentResult {
            unimplemented!("MockAdapter::run is never called in capability tests")
        }
    }

    #[async_trait]
    impl super::super::HarnessAdapter for MockAdapter {
        fn harness_id(&self) -> &str {
            self.id
        }

        fn transport(&self) -> TransportFlavor {
            self.flavor
        }

        fn capabilities(&self) -> &HarnessCapabilities {
            &self.caps
        }

        async fn probe(&self) -> Result<(), super::super::ProbeError> {
            Ok(())
        }
    }

    // Helper: build a requirements struct from a closure so each test can
    // flip exactly the fields it cares about.
    fn req_with(f: impl FnOnce(&mut HarnessTaskRequirements)) -> HarnessTaskRequirements {
        let mut r = HarnessTaskRequirements::default();
        f(&mut r);
        r
    }

    // ---- validate_for_task tests --------------------------------------------

    /// Constraint 1a: needs_tools + Opaque injection + McpMode::None → mismatch.
    #[test]
    fn needs_tools_opaque_no_mcp_is_mismatch() {
        let adapter = MockAdapter::conservative("hermes");
        let task = req_with(|r| r.needs_tools = true);
        let err = validate_for_task(&adapter, &task).unwrap_err();
        assert_eq!(err.need, "tools");
        assert_eq!(err.adapter, "hermes");
        assert_eq!(err.transport, TransportFlavor::OneShotPlain);
        assert!(err.hint.contains("PerCallTools"));
    }

    /// Constraint 1b: needs_tools + Opaque injection + McpMode::ServerOnly → mismatch
    /// (ServerOnly means roko cannot inject tools).
    #[test]
    fn needs_tools_opaque_server_only_mcp_is_mismatch() {
        let caps = HarnessCapabilities {
            tool_injection: ToolInjection::Opaque,
            mcp_passthrough: McpMode::ServerOnly,
            ..HarnessCapabilities::default()
        };
        let adapter = MockAdapter::new("cursor", TransportFlavor::AcpStdio, caps);
        let task = req_with(|r| r.needs_tools = true);
        let err = validate_for_task(&adapter, &task).unwrap_err();
        assert_eq!(err.need, "tools");
    }

    /// Constraint 1c: needs_tools + Opaque injection + McpMode::PerCall → OK
    /// (MCP provides tool passthrough even if direct injection is Opaque).
    #[test]
    fn needs_tools_opaque_with_per_call_mcp_is_ok() {
        let caps = HarnessCapabilities {
            tool_injection: ToolInjection::Opaque,
            mcp_passthrough: McpMode::PerCall,
            ..HarnessCapabilities::default()
        };
        let adapter = MockAdapter::new("cursor", TransportFlavor::AcpStdio, caps);
        let task = req_with(|r| r.needs_tools = true);
        assert!(validate_for_task(&adapter, &task).is_ok());
    }

    /// Constraint 1d: needs_tools + McpOnly injection (not Opaque) → OK.
    #[test]
    fn needs_tools_mcp_only_injection_is_ok() {
        let caps = HarnessCapabilities {
            tool_injection: ToolInjection::McpOnly,
            mcp_passthrough: McpMode::PerCall,
            ..HarnessCapabilities::default()
        };
        let adapter = MockAdapter::new("claude", TransportFlavor::HttpOpenAi, caps);
        let task = req_with(|r| r.needs_tools = true);
        assert!(validate_for_task(&adapter, &task).is_ok());
    }

    /// Constraint 2: needs_streaming + StreamingMode::None → mismatch.
    #[test]
    fn needs_streaming_none_is_mismatch() {
        let adapter = MockAdapter::conservative("ollama");
        let task = req_with(|r| r.needs_streaming = true);
        let err = validate_for_task(&adapter, &task).unwrap_err();
        assert_eq!(err.need, "streaming");
        assert_eq!(err.adapter, "ollama");
        assert!(err.hint.contains("SseChatCompletions") || err.hint.contains("NdJson"));
    }

    /// Constraint 2 satisfied: needs_streaming + SseChatCompletions → OK.
    #[test]
    fn needs_streaming_sse_is_ok() {
        let caps = HarnessCapabilities {
            streaming: StreamingMode::SseChatCompletions,
            ..HarnessCapabilities::default()
        };
        let adapter = MockAdapter::new("openai", TransportFlavor::HttpOpenAi, caps);
        let task = req_with(|r| r.needs_streaming = true);
        assert!(validate_for_task(&adapter, &task).is_ok());
    }

    /// Constraint 2 satisfied: needs_streaming + NdJson → OK.
    #[test]
    fn needs_streaming_ndjson_is_ok() {
        let caps = HarnessCapabilities {
            streaming: StreamingMode::NdJson,
            ..HarnessCapabilities::default()
        };
        let adapter = MockAdapter::new("claude-cli", TransportFlavor::OneShotJson, caps);
        let task = req_with(|r| r.needs_streaming = true);
        assert!(validate_for_task(&adapter, &task).is_ok());
    }

    /// Constraint 3: needs_mcp + McpMode::None → mismatch.
    #[test]
    fn needs_mcp_none_is_mismatch() {
        let adapter = MockAdapter::conservative("plain-cli");
        let task = req_with(|r| r.needs_mcp = true);
        let err = validate_for_task(&adapter, &task).unwrap_err();
        assert_eq!(err.need, "mcp_passthrough");
        assert!(err.hint.contains("per-call") || err.hint.contains("config-file"));
    }

    /// Constraint 3: needs_mcp + McpMode::ServerOnly → mismatch
    /// (roko cannot inject its own MCP servers).
    #[test]
    fn needs_mcp_server_only_is_mismatch() {
        let caps = HarnessCapabilities {
            mcp_passthrough: McpMode::ServerOnly,
            ..HarnessCapabilities::default()
        };
        let adapter = MockAdapter::new("locked", TransportFlavor::AcpStdio, caps);
        let task = req_with(|r| r.needs_mcp = true);
        let err = validate_for_task(&adapter, &task).unwrap_err();
        assert_eq!(err.need, "mcp_passthrough");
    }

    /// Constraint 3 satisfied: needs_mcp + McpMode::PerCall → OK.
    #[test]
    fn needs_mcp_per_call_is_ok() {
        let caps = HarnessCapabilities {
            mcp_passthrough: McpMode::PerCall,
            ..HarnessCapabilities::default()
        };
        let adapter = MockAdapter::new("claude-api", TransportFlavor::HttpOpenAi, caps);
        let task = req_with(|r| r.needs_mcp = true);
        assert!(validate_for_task(&adapter, &task).is_ok());
    }

    /// Constraint 3 satisfied: needs_mcp + McpMode::ConfigFile → OK.
    #[test]
    fn needs_mcp_config_file_is_ok() {
        let caps = HarnessCapabilities {
            mcp_passthrough: McpMode::ConfigFile("--mcp-config"),
            ..HarnessCapabilities::default()
        };
        let adapter = MockAdapter::new("claude-cli", TransportFlavor::OneShotJson, caps);
        let task = req_with(|r| r.needs_mcp = true);
        assert!(validate_for_task(&adapter, &task).is_ok());
    }

    /// Constraint 4: needs_session_resume + SessionResumeMode::None → mismatch.
    #[test]
    fn needs_session_resume_none_is_mismatch() {
        let adapter = MockAdapter::conservative("stateless");
        let task = req_with(|r| r.needs_session_resume = true);
        let err = validate_for_task(&adapter, &task).unwrap_err();
        assert_eq!(err.need, "session_resume");
        assert!(
            err.hint.contains("PreviousResponseId")
                || err.hint.contains("CliFlag")
                || err.hint.contains("Acp")
        );
    }

    /// Constraint 4 satisfied: needs_session_resume + PreviousResponseId → OK.
    #[test]
    fn needs_session_resume_previous_response_id_is_ok() {
        let caps = HarnessCapabilities {
            session_resume: SessionResumeMode::PreviousResponseId,
            ..HarnessCapabilities::default()
        };
        let adapter = MockAdapter::new("responses-api", TransportFlavor::HttpResponses, caps);
        let task = req_with(|r| r.needs_session_resume = true);
        assert!(validate_for_task(&adapter, &task).is_ok());
    }

    /// Constraint 4 satisfied: needs_session_resume + CliFlag → OK.
    #[test]
    fn needs_session_resume_cli_flag_is_ok() {
        let caps = HarnessCapabilities {
            session_resume: SessionResumeMode::CliFlag("--resume"),
            ..HarnessCapabilities::default()
        };
        let adapter = MockAdapter::new("claude-cli", TransportFlavor::OneShotJson, caps);
        let task = req_with(|r| r.needs_session_resume = true);
        assert!(validate_for_task(&adapter, &task).is_ok());
    }

    /// Constraint 4 satisfied: needs_session_resume + Acp → OK.
    #[test]
    fn needs_session_resume_acp_is_ok() {
        let caps = HarnessCapabilities {
            session_resume: SessionResumeMode::Acp,
            ..HarnessCapabilities::default()
        };
        let adapter = MockAdapter::new("cursor", TransportFlavor::AcpStdio, caps);
        let task = req_with(|r| r.needs_session_resume = true);
        assert!(validate_for_task(&adapter, &task).is_ok());
    }

    /// Constraint 5: needs_cancel + CancelMode::None → mismatch.
    #[test]
    fn needs_cancel_none_is_mismatch() {
        let adapter = MockAdapter::conservative("no-cancel");
        let task = req_with(|r| r.needs_cancel = true);
        let err = validate_for_task(&adapter, &task).unwrap_err();
        assert_eq!(err.need, "cancel");
        assert!(
            err.hint.contains("HttpEndpoint")
                || err.hint.contains("KillChild")
                || err.hint.contains("AcpCancel")
        );
    }

    /// Constraint 5 satisfied: needs_cancel + KillChild → OK.
    #[test]
    fn needs_cancel_kill_child_is_ok() {
        let caps = HarnessCapabilities {
            cancel: CancelMode::KillChild,
            ..HarnessCapabilities::default()
        };
        let adapter = MockAdapter::new("exec-agent", TransportFlavor::OneShotPlain, caps);
        let task = req_with(|r| r.needs_cancel = true);
        assert!(validate_for_task(&adapter, &task).is_ok());
    }

    /// Constraint 5 satisfied: needs_cancel + HttpEndpoint → OK.
    #[test]
    fn needs_cancel_http_endpoint_is_ok() {
        let caps = HarnessCapabilities {
            cancel: CancelMode::HttpEndpoint("/v1/runs/{id}/stop"),
            ..HarnessCapabilities::default()
        };
        let adapter = MockAdapter::new("hermes", TransportFlavor::HttpOpenAi, caps);
        let task = req_with(|r| r.needs_cancel = true);
        assert!(validate_for_task(&adapter, &task).is_ok());
    }

    /// Constraint 5 satisfied: needs_cancel + AcpCancel → OK.
    #[test]
    fn needs_cancel_acp_cancel_is_ok() {
        let caps = HarnessCapabilities {
            cancel: CancelMode::AcpCancel,
            ..HarnessCapabilities::default()
        };
        let adapter = MockAdapter::new("cursor", TransportFlavor::AcpStdio, caps);
        let task = req_with(|r| r.needs_cancel = true);
        assert!(validate_for_task(&adapter, &task).is_ok());
    }

    /// Constraint 6: PtyAutomation one_shot + !allows_pty_overhead → mismatch.
    #[test]
    fn pty_automation_without_allows_pty_overhead_is_mismatch() {
        let caps = HarnessCapabilities {
            one_shot: OneShotMode::PtyAutomation,
            ..HarnessCapabilities::default()
        };
        let adapter = MockAdapter::new("pty-driver", TransportFlavor::TuiJsonRpc, caps);
        // allows_pty_overhead defaults to false
        let task = HarnessTaskRequirements::default();
        let err = validate_for_task(&adapter, &task).unwrap_err();
        assert_eq!(err.need, "non_pty");
        assert_eq!(err.adapter, "pty-driver");
        assert!(err.hint.contains("allows_pty_overhead"));
    }

    /// Constraint 6 satisfied: PtyAutomation + allows_pty_overhead = true → OK.
    #[test]
    fn pty_automation_with_allows_pty_overhead_is_ok() {
        let caps = HarnessCapabilities {
            one_shot: OneShotMode::PtyAutomation,
            ..HarnessCapabilities::default()
        };
        let adapter = MockAdapter::new("pty-driver", TransportFlavor::TuiJsonRpc, caps);
        let task = req_with(|r| r.allows_pty_overhead = true);
        assert!(validate_for_task(&adapter, &task).is_ok());
    }

    /// Non-PTY one_shot modes do not trigger the PTY constraint regardless of
    /// allows_pty_overhead.
    #[test]
    fn non_pty_one_shot_ignores_allows_pty_overhead() {
        let caps = HarnessCapabilities {
            one_shot: OneShotMode::CliCommand {
                subcommand: "run",
                output: CliOutput::PlainText,
            },
            ..HarnessCapabilities::default()
        };
        let adapter = MockAdapter::new("plain", TransportFlavor::OneShotPlain, caps);
        // allows_pty_overhead = false but one_shot is not PTY
        let task = HarnessTaskRequirements::default();
        assert!(validate_for_task(&adapter, &task).is_ok());
    }

    /// Happy path: fully-capable adapter + task that needs everything → OK.
    #[test]
    fn fully_capable_adapter_satisfies_all_requirements() {
        let adapter = MockAdapter::capable("all-caps");
        let task = req_with(|r| {
            r.needs_tools = true;
            r.needs_streaming = true;
            r.needs_mcp = true;
            r.needs_session_resume = true;
            r.needs_cancel = true;
            r.allows_pty_overhead = true;
        });
        assert!(validate_for_task(&adapter, &task).is_ok());
    }

    /// Happy path: default (empty) requirements against any adapter → OK.
    #[test]
    fn default_requirements_always_pass() {
        let conservative = MockAdapter::conservative("bare");
        let capable = MockAdapter::capable("full");
        let task = HarnessTaskRequirements::default();
        assert!(validate_for_task(&conservative, &task).is_ok());
        assert!(validate_for_task(&capable, &task).is_ok());
    }

    /// Conservative (default) capabilities reject every non-trivial requirement.
    #[test]
    fn conservative_adapter_rejects_tools() {
        let adapter = MockAdapter::conservative("bare");
        let task = req_with(|r| r.needs_tools = true);
        assert!(validate_for_task(&adapter, &task).is_err());
    }

    #[test]
    fn conservative_adapter_rejects_streaming() {
        let adapter = MockAdapter::conservative("bare");
        let task = req_with(|r| r.needs_streaming = true);
        assert!(validate_for_task(&adapter, &task).is_err());
    }

    #[test]
    fn conservative_adapter_rejects_mcp() {
        let adapter = MockAdapter::conservative("bare");
        let task = req_with(|r| r.needs_mcp = true);
        assert!(validate_for_task(&adapter, &task).is_err());
    }

    #[test]
    fn conservative_adapter_rejects_session_resume() {
        let adapter = MockAdapter::conservative("bare");
        let task = req_with(|r| r.needs_session_resume = true);
        assert!(validate_for_task(&adapter, &task).is_err());
    }

    #[test]
    fn conservative_adapter_rejects_cancel() {
        let adapter = MockAdapter::conservative("bare");
        let task = req_with(|r| r.needs_cancel = true);
        assert!(validate_for_task(&adapter, &task).is_err());
    }

    /// CapabilityMismatch carries the correct adapter id and transport on every
    /// constraint path so the operator can identify the failing adapter.
    #[test]
    fn mismatch_carries_adapter_id_and_transport() {
        let caps = HarnessCapabilities {
            streaming: StreamingMode::None,
            ..HarnessCapabilities::default()
        };
        let adapter = MockAdapter::new("my-adapter", TransportFlavor::McpServer, caps);
        let task = req_with(|r| r.needs_streaming = true);
        let err = validate_for_task(&adapter, &task).unwrap_err();
        assert_eq!(err.adapter, "my-adapter");
        assert_eq!(err.transport, TransportFlavor::McpServer);
    }

    /// Checks are ordered: the first violated constraint is the one reported.
    /// If needs_tools and needs_streaming both fail, "tools" is reported first.
    #[test]
    fn first_failing_constraint_wins() {
        let adapter = MockAdapter::conservative("bare");
        let task = req_with(|r| {
            r.needs_tools = true;
            r.needs_streaming = true;
        });
        let err = validate_for_task(&adapter, &task).unwrap_err();
        // tools check comes before streaming in the function body
        assert_eq!(err.need, "tools");
    }

    #[test]
    fn transport_flavor_display_roundtrips() {
        let flavors = [
            (TransportFlavor::HttpOpenAi, "http_openai"),
            (TransportFlavor::HttpResponses, "http_responses"),
            (TransportFlavor::OneShotJson, "oneshot_json"),
            (TransportFlavor::OneShotPlain, "oneshot_plain"),
            (TransportFlavor::AcpStdio, "acp_stdio"),
            (TransportFlavor::TuiJsonRpc, "tui_jsonrpc"),
            (TransportFlavor::McpServer, "mcp_server"),
        ];
        for (flavor, expected) in &flavors {
            assert_eq!(flavor.to_string(), *expected);
            assert_eq!(TransportFlavor::from_str_loose(expected), Some(*flavor));
        }
    }

    #[test]
    fn transport_flavor_from_str_loose_rejects_unknown() {
        assert_eq!(TransportFlavor::from_str_loose("grpc"), None);
        assert_eq!(TransportFlavor::from_str_loose(""), None);
    }

    #[test]
    fn default_capabilities_are_conservative() {
        let caps = HarnessCapabilities::default();
        assert!(matches!(caps.one_shot, OneShotMode::Unsupported));
        assert!(matches!(caps.streaming, StreamingMode::None));
        assert!(matches!(caps.session_resume, SessionResumeMode::None));
        assert!(matches!(caps.mcp_passthrough, McpMode::None));
        assert!(matches!(caps.tool_injection, ToolInjection::Opaque));
        assert!(!caps.model_override);
        assert!(!caps.multiplex_safe);
        assert!(matches!(caps.cancel, CancelMode::None));
        assert_eq!(caps.overhead_p50_ms, 0);
    }

    #[test]
    fn transport_flavor_serde_roundtrips() {
        let flavor = TransportFlavor::HttpOpenAi;
        let json = serde_json::to_string(&flavor).expect("serialize");
        assert_eq!(json, r#""http_openai""#);
        let back: TransportFlavor = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, flavor);
    }

    #[test]
    fn harness_task_requirements_default_is_permissive() {
        let req = HarnessTaskRequirements::default();
        assert!(!req.needs_tools);
        assert!(!req.needs_streaming);
        assert!(!req.needs_mcp);
        assert!(!req.needs_session_resume);
        assert!(!req.needs_cancel);
        assert!(req.max_timeout.is_none());
        assert!(!req.allows_pty_overhead);
    }

    #[test]
    fn from_str_loose_accepts_kebab_case() {
        assert_eq!(
            TransportFlavor::from_str_loose("http-openai"),
            Some(TransportFlavor::HttpOpenAi)
        );
        assert_eq!(
            TransportFlavor::from_str_loose("http-responses"),
            Some(TransportFlavor::HttpResponses)
        );
        assert_eq!(
            TransportFlavor::from_str_loose("oneshot-json"),
            Some(TransportFlavor::OneShotJson)
        );
        assert_eq!(
            TransportFlavor::from_str_loose("oneshot-plain"),
            Some(TransportFlavor::OneShotPlain)
        );
        assert_eq!(
            TransportFlavor::from_str_loose("acp-stdio"),
            Some(TransportFlavor::AcpStdio)
        );
        assert_eq!(
            TransportFlavor::from_str_loose("tui-jsonrpc"),
            Some(TransportFlavor::TuiJsonRpc)
        );
        assert_eq!(
            TransportFlavor::from_str_loose("mcp-server"),
            Some(TransportFlavor::McpServer)
        );
    }

    #[test]
    fn from_str_loose_accepts_pascal_case() {
        assert_eq!(
            TransportFlavor::from_str_loose("HttpOpenAi"),
            Some(TransportFlavor::HttpOpenAi)
        );
        assert_eq!(
            TransportFlavor::from_str_loose("HttpResponses"),
            Some(TransportFlavor::HttpResponses)
        );
        assert_eq!(
            TransportFlavor::from_str_loose("OneShotJson"),
            Some(TransportFlavor::OneShotJson)
        );
        assert_eq!(
            TransportFlavor::from_str_loose("OneShotPlain"),
            Some(TransportFlavor::OneShotPlain)
        );
        assert_eq!(
            TransportFlavor::from_str_loose("AcpStdio"),
            Some(TransportFlavor::AcpStdio)
        );
        assert_eq!(
            TransportFlavor::from_str_loose("TuiJsonRpc"),
            Some(TransportFlavor::TuiJsonRpc)
        );
        assert_eq!(
            TransportFlavor::from_str_loose("McpServer"),
            Some(TransportFlavor::McpServer)
        );
    }

    #[test]
    fn capability_mismatch_display() {
        let mismatch = CapabilityMismatch {
            adapter: "hermes".to_string(),
            transport: TransportFlavor::HttpOpenAi,
            need: "streaming",
            hint: "use a transport with SSE support",
        };
        let msg = mismatch.to_string();
        assert_eq!(
            msg,
            "hermes/http_openai cannot meet task requirement: streaming -- use a transport with SSE support"
        );

        // Verify it implements std::error::Error
        let err: &dyn std::error::Error = &mismatch;
        assert!(err.source().is_none());
    }
}
