//! `ClaudeCliAgent` — choose this for the Claude CLI path with Roko's system
//! prompt, tool allowlist, safety settings, and session-aware behavior.
//!
//! This is the runtime-facing adapter for the `claude` executable. It keeps
//! the wire-specific flag construction in one place instead of scattering
//! command-building logic across the CLI entrypoints. Prefer
//! [`ExecAgent`](crate::ExecAgent) only for generic stdin/stdout CLIs where
//! Claude-specific resume and tool-loop wiring are not needed.

use crate::agent::{Agent, AgentResult};
use crate::mcp::find_mcp_config;
use crate::process::{
    GRACE_STDIN_CLOSE_MS, ResourceLimits, apply_credential_scrub, benign_stderr_warn_once,
    classify_benign_stderr, config_file_env_names, confined_command, kill_tree,
    register_spawned_pid, set_process_group, unregister_pid,
};
use crate::provider::error_classify::{ATTEMPT_TIMEOUT_MARKER, detect_provider_exhaustion};
use crate::tool_loop::{StreamEvent, StreamEventKind};
use crate::usage::{Usage, UsageObservation, UsageSource};
use async_trait::async_trait;
use roko_core::agent::ProviderKind;
use roko_core::child_env::CredentialScrub;
use roko_core::config::model_registry::model_meta;
use roko_core::defaults::DEFAULT_REQUEST_TIMEOUT_MS;
use roko_core::{Body, Context, Kind, OperatingFrequency, Provenance, Signal};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::time::{Duration, timeout};

/// Build the Claude CLI `--settings` JSON payload with safety hooks.
///
/// Claude Code hook entries do not support per-hook condition fields. Keep the
/// filtering inside one command so ordinary Bash calls are allowed while the
/// destructive commands that should never be launched by a model in this
/// workspace are blocked.
#[must_use]
pub fn build_settings_json() -> String {
    serde_json::json!({
        "hooks": {
            "PreToolUse": [{
                "matcher": "Bash",
                "hooks": [{
                    "type": "command",
                    "command": r#"command -v python3 >/dev/null 2>&1 || exit 0
python3 -c 'import json, re, sys
try:
    data = json.load(sys.stdin)
except Exception:
    sys.exit(0)
tool_input = data.get("tool_input") or data.get("toolInput") or {}
command = tool_input.get("command") or data.get("command") or ""
if not isinstance(command, str):
    sys.exit(0)
checks = [
    (r"^\s*git\s+checkout(?:\s|$)", "BLOCKED: git checkout forbidden in plan worktrees"),
    (r"^\s*git\s+switch(?:\s|$)", "BLOCKED: git switch forbidden in plan worktrees"),
    (r"^\s*git\s+branch\s+-m(?:\s|$)", "BLOCKED: branch rename forbidden in plan worktrees"),
    (r"^\s*git\s+push(?:\s|$)", "BLOCKED: agents must not push - roko handles merges"),
    (r"(^|[;&|]\s*)rm\s+-[A-Za-z]*r[A-Za-z]*f[A-Za-z]*(?:\s|$)", "BLOCKED: destructive file deletion forbidden"),
    (r"(^|[;&|]\s*)rm\s+-[A-Za-z]*f[A-Za-z]*r[A-Za-z]*(?:\s|$)", "BLOCKED: destructive file deletion forbidden"),
    (r"(^|[;&|]\s*)rm\s+-[A-Za-z]*r[A-Za-z]*(?:\s|$)", "BLOCKED: destructive file deletion forbidden"),
]
for pattern, message in checks:
    if re.search(pattern, command):
        print(message, file=sys.stderr)
        sys.exit(2)
sys.exit(0)'"#
                }]
            }]
        }
    })
    .to_string()
}

/// Agent wrapper around the `claude` CLI.
#[derive(Debug, Clone)]
pub struct ClaudeCliAgent {
    program: PathBuf,
    current_dir: PathBuf,
    model: String,
    effort: String,
    fallback_model: Option<String>,
    bare_mode: bool,
    system_prompt: Option<String>,
    allowed_tools: Option<String>,
    disallowed_tools: Option<String>,
    max_turns: Option<u32>,
    settings_json: String,
    extra_args: Vec<String>,
    env: Vec<(String, String)>,
    credential_scrub: CredentialScrub,
    mcp_config: Option<PathBuf>,
    resume: Option<String>,
    dangerously_skip_permissions: bool,
    timeout_ms: u64,
    resource_limits: Option<ResourceLimits>,
    name: String,
}

impl ClaudeCliAgent {
    /// Construct a new Claude CLI agent rooted at `current_dir`.
    #[must_use]
    pub fn new(
        program: impl Into<PathBuf>,
        current_dir: impl Into<PathBuf>,
        model: impl Into<String>,
    ) -> Self {
        let model = model.into();
        Self {
            program: program.into(),
            current_dir: current_dir.into(),
            model: model.clone(),
            effort: "medium".to_string(),
            fallback_model: Some(roko_core::defaults::MODEL_FAST.to_string()),
            bare_mode: true,
            system_prompt: None,
            allowed_tools: None,
            disallowed_tools: None,
            max_turns: Some(OperatingFrequency::Theta.turn_limit()),
            settings_json: build_settings_json(),
            extra_args: Vec::new(),
            env: Vec::new(),
            credential_scrub: CredentialScrub::for_kind(ProviderKind::ClaudeCli),
            mcp_config: None,
            resume: None,
            dangerously_skip_permissions: true,
            timeout_ms: DEFAULT_REQUEST_TIMEOUT_MS,
            resource_limits: None,
            name: format!("claude-cli:{model}"),
        }
    }

    /// Override the display name used in traces.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Override the per-request timeout in milliseconds.
    #[must_use]
    pub const fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Apply OS resource limits to the Claude subprocess.
    #[must_use]
    pub fn with_resource_limits(mut self, limits: ResourceLimits) -> Self {
        self.resource_limits = Some(limits);
        self
    }

    /// Override the reasoning-effort label passed to Claude.
    #[must_use]
    pub fn with_effort(mut self, effort: impl Into<String>) -> Self {
        self.effort = effort.into();
        self
    }

    /// Override the fallback model passed to Claude.
    #[must_use]
    pub fn with_fallback_model(mut self, fallback_model: impl Into<String>) -> Self {
        self.fallback_model = Some(fallback_model.into());
        self
    }

    /// When enabled, replace Claude Code's built-in system prompt with the
    /// caller-supplied prompt (or an empty prompt when none was supplied).
    /// Disable this to append caller guidance to Claude's built-in prompt.
    #[must_use]
    pub const fn with_bare_mode(mut self, bare_mode: bool) -> Self {
        self.bare_mode = bare_mode;
        self
    }

    /// Attach a system prompt generated by `SystemPromptBuilder`.
    #[must_use]
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Attach a Claude tool allowlist, formatted as `Read,Edit,Bash`.
    #[must_use]
    pub fn with_tools(mut self, tools: impl Into<String>) -> Self {
        self.allowed_tools = Some(tools.into());
        self
    }

    /// Attach a Claude `--allowedTools` allowlist.
    #[must_use]
    pub fn with_allowed_tools(mut self, tools: impl Into<String>) -> Self {
        self.allowed_tools = Some(tools.into());
        self
    }

    /// Attach a Claude `--disallowed-tools` denylist.
    #[must_use]
    pub fn with_disallowed_tools(mut self, tools: impl Into<String>) -> Self {
        self.disallowed_tools = Some(tools.into());
        self
    }

    /// Set the maximum number of turns Claude may take.
    #[must_use]
    pub const fn with_max_turns(mut self, max_turns: u32) -> Self {
        self.max_turns = Some(max_turns);
        self
    }

    /// Override the settings JSON passed via `--settings`.
    #[must_use]
    pub fn with_settings_json(mut self, json: impl Into<String>) -> Self {
        self.settings_json = json.into();
        self
    }

    /// Pass through additional CLI args before the canonical Claude flags.
    #[must_use]
    pub fn with_extra_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.extra_args.extend(args.into_iter().map(Into::into));
        self
    }

    /// Add a process environment variable.
    #[must_use]
    pub fn with_env_var(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    /// Replace the policy for which inherited credentials the subprocess
    /// loses (default: [`CredentialScrub::for_kind`] of `ClaudeCli`).
    #[must_use]
    pub fn with_credential_scrub(mut self, scrub: CredentialScrub) -> Self {
        self.credential_scrub = scrub;
        self
    }

    /// Attach an explicit MCP config path.
    #[must_use]
    pub fn with_mcp_config(mut self, path: impl Into<PathBuf>) -> Self {
        self.mcp_config = Some(path.into());
        self
    }

    /// Resume the given Claude session id.
    #[must_use]
    pub fn with_resume(mut self, session_id: impl Into<String>) -> Self {
        self.resume = Some(session_id.into());
        self
    }

    /// Resume a session id only when present.
    #[must_use]
    pub fn with_optional_resume(mut self, session_id: Option<String>) -> Self {
        self.resume = session_id.filter(|id| !id.trim().is_empty());
        self
    }

    /// Toggle `--dangerously-skip-permissions` for role-gated policy.
    #[must_use]
    pub const fn with_dangerously_skip_permissions(mut self, enabled: bool) -> Self {
        self.dangerously_skip_permissions = enabled;
        self
    }

    fn failure(&self, input: &Signal, reason: &str, started: Instant) -> AgentResult {
        let stream_usage = StreamUsage::default();
        self.failure_with_stream_usage(input, reason, started, &stream_usage)
    }

    fn failure_with_stream_usage(
        &self,
        input: &Signal,
        reason: &str,
        started: Instant,
        stream_usage: &StreamUsage,
    ) -> AgentResult {
        let wall_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        let mut output = input
            .derive(Kind::AgentOutput, Body::text(reason))
            .provenance(Provenance::agent(&self.name))
            .tag("agent", &self.name)
            .tag("failed", "true");
        if let Some(model) = stream_usage
            .model
            .as_deref()
            .filter(|model| !model.trim().is_empty())
        {
            output = output.tag("model", model);
        }
        if let Some(num_turns) = stream_usage.num_turns {
            output = output.tag("num_turns", num_turns.to_string());
        }
        let output = output.build();
        AgentResult::fail(output).with_usage_obs(self.usage_observation(stream_usage, wall_ms))
    }

    /// The run stopped at `--max-turns`: the final stream-json `result` has
    /// subtype `error_max_turns` (the CLI exits 1 with no text or stderr).
    fn turn_cap_hit(
        &self,
        stdout: &str,
        stderr: &str,
    ) -> Option<crate::provider::error_classify::TurnCapHit> {
        let result = [stdout, stderr].into_iter().find_map(|output| {
            output
                .lines()
                .filter_map(Self::parse_stream_event)
                .rfind(|event| event.get("type").and_then(Value::as_str) == Some("result"))
        })?;
        (result.get("subtype").and_then(Value::as_str) == Some("error_max_turns")).then(|| {
            crate::provider::error_classify::TurnCapHit {
                num_turns: result
                    .get("num_turns")
                    .and_then(Value::as_u64)
                    .and_then(|turns| u32::try_from(turns).ok()),
                cap: self.max_turns,
            }
        })
    }

    fn discovered_mcp_config(&self) -> Option<PathBuf> {
        if let Some(path) = &self.mcp_config {
            return Some(path.clone());
        }
        match find_mcp_config(&self.current_dir) {
            Some(Ok((path, _))) => Some(path),
            Some(Err(err)) => {
                tracing::warn!(agent = "claude-cli", "ignoring invalid MCP config: {err}");
                None
            }
            None => None,
        }
    }

    fn parse_stream_events(stdout: &str) -> Option<Vec<Value>> {
        let events: Vec<Value> = stdout
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .filter_map(Self::parse_stream_event)
            .collect();
        if events.is_empty() {
            None
        } else {
            Some(events)
        }
    }

    fn prompt_text_from_input(input: &Signal) -> Result<String, String> {
        input.body.as_text().map(str::to_string).or_else(|_| {
            serde_json::to_string(&input.body)
                .map_err(|e| format!("input body not readable as text or json: {e}"))
        })
    }

    fn build_command(&self) -> std::io::Result<Command> {
        let mut cmd = confined_command(&self.program, self.resource_limits.as_ref())?;
        cmd.args(&self.extra_args);
        // Claude removed `--bare`, but `--system-prompt` provides the part of
        // its contract Roko relies on: replacing Claude Code's built-in prompt
        // instead of paying for it in addition to Roko's canonical prompt.
        cmd.arg("--print")
            .arg("--verbose")
            .arg("--output-format")
            .arg("stream-json")
            .arg("--model")
            .arg(&self.model)
            .arg("--effort")
            .arg(&self.effort)
            .arg("--settings")
            .arg(&self.settings_json);
        if self.dangerously_skip_permissions {
            cmd.arg("--dangerously-skip-permissions");
        }
        if let Some(max_turns) = self.max_turns {
            cmd.arg("--max-turns").arg(max_turns.to_string());
        }

        if let Some(fallback_model) = &self.fallback_model
            && fallback_model != &self.model
        {
            cmd.arg("--fallback-model").arg(fallback_model);
        }
        if self.bare_mode {
            cmd.arg("--system-prompt")
                .arg(self.system_prompt.as_deref().unwrap_or_default());
        } else if let Some(system_prompt) = &self.system_prompt {
            cmd.arg("--append-system-prompt").arg(system_prompt);
        }
        // `Some("")` is deliberate: Claude documents an empty `--tools`
        // value as disabling all tools. Omitting the flag would instead make
        // a restricted fallback permissive.
        if let Some(tools) = &self.allowed_tools {
            cmd.arg("--tools").arg(tools);
        }
        if let Some(tools) = &self.disallowed_tools
            && !tools.is_empty()
        {
            cmd.arg("--disallowed-tools").arg(tools);
        }
        let mcp_config = self.discovered_mcp_config();
        if let Some(mcp_config) = &mcp_config {
            cmd.arg("--mcp-config").arg(mcp_config);
            cmd.arg("--strict-mcp-config");
        }
        if let Some(resume) = &self.resume {
            cmd.arg("--resume").arg(resume);
        }

        cmd.current_dir(&self.current_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        set_process_group(&mut cmd);

        // Other providers' keys and keys only roko loaded stay out of the
        // agent; MCP servers still get the variables their config names.
        let scrub = self.credential_scrub.clone().keep_all(
            mcp_config
                .as_deref()
                .map(config_file_env_names)
                .unwrap_or_default(),
        );
        apply_credential_scrub(&mut cmd, &scrub);
        for (key, value) in &self.env {
            cmd.env(key, value);
        }
        cmd.env("CARGO_INCREMENTAL", "0");
        cmd.env("CARGO_BUILD_JOBS", "2");
        // Prevent "nested session" detection when spawning from within Claude Code.
        cmd.env_remove("CLAUDECODE");
        Ok(cmd)
    }

    fn debug_enabled() -> bool {
        std::env::var_os("ROKO_DEBUG")
            .map(|value| {
                let value = value.to_string_lossy().trim().to_ascii_lowercase();
                matches!(value.as_str(), "1" | "true" | "yes" | "on")
            })
            .unwrap_or(false)
    }

    fn parse_stream_event(line: &str) -> Option<Value> {
        let value = serde_json::from_str::<Value>(line.trim()).ok()?;
        if value.get("type").and_then(Value::as_str).is_some() {
            Some(value)
        } else {
            None
        }
    }

    /// Usage from stream-json output.
    ///
    /// The final `result` event carries the provider-reported totals. A run
    /// killed before it (a timeout) has only its `assistant` events, whose
    /// usage becomes an [`UsageSource::Estimated`] partial total (see
    /// [`StreamedMessages`]); `fallback_model` prices messages naming no model.
    fn parse_stream_usage(output: &str, fallback_model: &str) -> StreamUsage {
        let mut usage = StreamUsage::default();
        let mut streamed = StreamedMessages::default();
        for line in output
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
        {
            let Some(event) = Self::parse_stream_event(line) else {
                continue;
            };
            let kind = event.get("type").and_then(Value::as_str);
            if kind == Some("assistant") {
                streamed.observe(&event);
                continue;
            }
            if kind != Some("result") {
                continue;
            }

            usage.source = UsageSource::ProviderReported;
            if let Some(num_turns) = event.get("num_turns").and_then(Value::as_u64) {
                usage.num_turns = Some(num_turns);
            }
            if let Some(model) = event
                .get("model")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|model| !model.is_empty())
            {
                usage.model = Some(model.to_string());
            }
            if let Some(cost) = event.get("total_cost_usd").and_then(Value::as_f64) {
                usage.cost_usd = Some(cost);
            }
            if let Some(result_usage) = event.get("usage") {
                Self::update_stream_usage_field(
                    &mut usage.input_tokens,
                    Self::stream_usage_u64(result_usage, &["input_tokens"]),
                );
                Self::update_stream_usage_field(
                    &mut usage.output_tokens,
                    Self::stream_usage_u64(result_usage, &["output_tokens"]),
                );
                Self::update_stream_usage_field(
                    &mut usage.cache_creation_tokens,
                    Self::stream_usage_u64(
                        result_usage,
                        &["cache_creation_input_tokens", "cache_creation_tokens"],
                    ),
                );
                Self::update_stream_usage_field(
                    &mut usage.cache_read_tokens,
                    Self::stream_usage_u64(
                        result_usage,
                        &["cache_read_input_tokens", "cache_read_tokens"],
                    ),
                );
            }
        }
        if usage.source == UsageSource::Unknown {
            return streamed.into_stream_usage(fallback_model);
        }
        usage
    }

    /// Canonical usage for a run, keeping the source of `stream_usage`:
    /// provider-reported, estimated from what a killed run streamed, or
    /// unknown.
    fn usage_observation(&self, stream_usage: &StreamUsage, wall_ms: u64) -> UsageObservation {
        UsageObservation {
            input_tokens: stream_usage.input_tokens,
            output_tokens: stream_usage.output_tokens,
            cache_creation_tokens: stream_usage.cache_creation_tokens,
            cache_read_tokens: stream_usage.cache_read_tokens,
            reasoning_tokens: None,
            cost_usd: stream_usage.cost_usd,
            source: stream_usage.source.clone(),
            model: stream_usage
                .model
                .clone()
                .or_else(|| Some(self.model.clone())),
            wall_ms,
        }
    }

    fn stream_usage_u64(usage: &Value, keys: &[&str]) -> Option<u64> {
        keys.iter()
            .find_map(|key| usage.get(*key).and_then(Value::as_u64))
    }

    fn update_stream_usage_field<T>(slot: &mut Option<T>, value: Option<T>) {
        if let Some(value) = value {
            *slot = Some(value);
        }
    }

    fn tool_summary(block: &Value) -> String {
        let name = block
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let mut summary = format!("tool: {name}");

        if let Some(input) = block.get("input") {
            for key in ["path", "file_path", "filename"] {
                if let Some(path) = input.get(key).and_then(Value::as_str) {
                    summary.push_str(&format!(" path={path}"));
                    return summary;
                }
            }
            if let Some(command) = input.get("command").and_then(Value::as_str) {
                summary.push_str(&format!(" command={command}"));
            }
        }

        summary
    }

    fn emit_stream_summary(
        agent_name: &str,
        event: &Value,
        text_bytes: &mut usize,
        tool_count: &mut usize,
    ) {
        match event.get("type").and_then(Value::as_str) {
            Some("assistant") => {
                if let Some(content) = event
                    .get("message")
                    .and_then(|m| m.get("content"))
                    .and_then(Value::as_array)
                {
                    for block in content {
                        if block.get("type").and_then(Value::as_str) == Some("tool_use") {
                            *tool_count += 1;
                            tracing::debug!(agent = %agent_name, "{}", Self::tool_summary(block));
                        }
                    }
                }
            }
            Some("content_block_start") => {
                if let Some(block) = event.get("content_block") {
                    match block.get("type").and_then(Value::as_str) {
                        Some("tool_use") => {
                            *tool_count += 1;
                            tracing::debug!(agent = %agent_name, "{}", Self::tool_summary(block));
                        }
                        Some("text") => {
                            tracing::debug!(agent = %agent_name, "generating text...");
                        }
                        _ => {}
                    }
                }
            }
            Some("content_block_delta") => {
                if let Some(delta) = event.get("delta")
                    && let Some(text) = delta.get("text").and_then(Value::as_str)
                {
                    *text_bytes += text.len();
                }
            }
            Some("result") => {
                let summary = if *tool_count > 0 {
                    format!("{text_bytes} bytes text, {tool_count} tool calls")
                } else {
                    format!("{text_bytes} bytes text")
                };
                tracing::info!(agent = %agent_name, "result received ({summary})");
            }
            _ => {}
        }
    }

    /// Human-readable reason for a non-zero exit.
    ///
    /// A usage-window refusal ("You've hit your session limit · resets 4pm")
    /// arrives as the stream-json `result` on stdout with nothing on stderr,
    /// so both are read; an exhaustion is rendered as
    /// [`crate::provider::ProviderError::ProviderExhausted`] so callers can
    /// quarantine the provider until its reset time.
    fn failure_reason(stdout: &str, stderr: &str) -> String {
        let human_stderr = stderr
            .lines()
            .filter(|line| Self::parse_stream_event(line).is_none())
            .collect::<Vec<_>>()
            .join("\n");
        let result_error =
            Self::result_error_text(stdout).or_else(|| Self::result_error_text(stderr));
        if let Some(exhaustion) = detect_provider_exhaustion(&human_stderr)
            .or_else(|| result_error.as_deref().and_then(detect_provider_exhaustion))
        {
            return exhaustion.into_error().to_string();
        }
        Self::first_human_stderr_line(stderr)
            .map(str::to_string)
            .or(result_error)
            .unwrap_or_else(|| "claude failed".to_string())
    }

    /// Text of the final stream-json `result` event when it reports an error.
    fn result_error_text(output: &str) -> Option<String> {
        output
            .lines()
            .filter_map(Self::parse_stream_event)
            .rfind(|event| event.get("type").and_then(Value::as_str) == Some("result"))
            .filter(|event| {
                event
                    .get("is_error")
                    .and_then(Value::as_bool)
                    .unwrap_or(true)
            })
            .and_then(|event| {
                event
                    .get("result")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|text| !text.is_empty())
                    .map(str::to_string)
            })
    }

    fn first_human_stderr_line(stderr: &str) -> Option<&str> {
        stderr.lines().find(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty()
                && Self::parse_stream_event(trimmed).is_none()
                && classify_benign_stderr(line).is_none()
        })
    }

    fn stream_requested_tool_use(events: &[Value]) -> bool {
        events
            .iter()
            .any(|event| match event.get("type").and_then(Value::as_str) {
                Some("assistant") => event
                    .get("message")
                    .and_then(|m| m.get("content"))
                    .and_then(Value::as_array)
                    .is_some_and(|content| {
                        content.iter().any(|block| {
                            block.get("type").and_then(Value::as_str) == Some("tool_use")
                        })
                    }),
                Some("content_block_start") => {
                    event
                        .get("content_block")
                        .and_then(|block| block.get("type").and_then(Value::as_str))
                        == Some("tool_use")
                }
                Some("tool") => true,
                _ => false,
            })
    }

    fn output_text(stdout: &str) -> String {
        Self::parse_stream_events(stdout).map_or_else(
            || stdout.trim().to_string(),
            |events| {
                let requested_tool_use = Self::stream_requested_tool_use(&events);
                let response = crate::translate::BackendResponse::StreamJson(events);
                let extracted = response.extract_text();
                if extracted.trim().is_empty() {
                    if requested_tool_use {
                        "assistant requested tool use".to_string()
                    } else {
                        String::new()
                    }
                } else {
                    extracted
                }
            },
        )
    }

    fn stderr_trace(&self, stderr: &str) -> Vec<Signal> {
        stderr
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                !trimmed.is_empty() && Self::parse_stream_event(trimmed).is_none()
            })
            .filter(|line| !self.warn_and_filter_benign(line))
            .map(|line| {
                Signal::builder(Kind::AgentMessage)
                    .body(Body::text(line))
                    .provenance(Provenance::agent(&self.name))
                    .tag("stream", "stderr")
                    .build()
            })
            .collect()
    }

    fn warn_and_filter_benign(&self, line: &str) -> bool {
        if let Some(benign) = classify_benign_stderr(line) {
            if benign_stderr_warn_once(benign.key) {
                tracing::warn!(agent = %self.name, "{}", benign.summary);
            }
            return true;
        }
        false
    }

    /// Translate a parsed stream-json `Value` into zero or more
    /// [`StreamEventKind`]s to forward to a streaming receiver.
    ///
    /// Handles:
    /// - `assistant` content blocks: `text` → `TextDelta`, `thinking` →
    ///   `ReasoningDelta`, `tool_use` → `ToolCallEnd`.
    /// - `tool` events (subtype `result`) → `ToolResult`.
    /// - `user` messages with `tool_result` blocks (older CLI format) →
    ///   `ToolResult`, with content flattened to a single text string.
    fn event_kinds_from_value(event: &Value) -> Vec<StreamEventKind> {
        let mut events = Vec::new();
        match event.get("type").and_then(Value::as_str) {
            Some("assistant") => {
                let Some(content) = event
                    .get("message")
                    .and_then(|m| m.get("content"))
                    .and_then(Value::as_array)
                else {
                    return events;
                };
                for block in content {
                    match block.get("type").and_then(Value::as_str) {
                        Some("text") => {
                            if let Some(text) = block.get("text").and_then(Value::as_str) {
                                events.push(StreamEventKind::TextDelta(text.to_string()));
                            }
                        }
                        Some("thinking") => {
                            if let Some(thinking) = block.get("thinking").and_then(Value::as_str) {
                                events.push(StreamEventKind::ReasoningDelta(thinking.to_string()));
                            }
                        }
                        Some("tool_use") => {
                            if let (Some(id), Some(name)) = (
                                block.get("id").and_then(Value::as_str),
                                block.get("name").and_then(Value::as_str),
                            ) {
                                let args = block.get("input").cloned().unwrap_or(Value::Null);
                                events.push(StreamEventKind::ToolCallEnd {
                                    id: id.to_string(),
                                    name: name.to_string(),
                                    args,
                                });
                            }
                        }
                        _ => {}
                    }
                }
            }
            Some("tool") => {
                // `tool` event with subtype "result" carries the tool output.
                let id = event
                    .get("tool_use_id")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let output = event
                    .get("content")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                events.push(StreamEventKind::ToolResult { id, output });
            }
            Some("user") => {
                // Older Claude CLI format: tool results arrive as a `user`
                // message with `tool_result` content blocks.
                let Some(content) = event
                    .get("message")
                    .and_then(|m| m.get("content"))
                    .and_then(Value::as_array)
                else {
                    return events;
                };
                for block in content {
                    if block.get("type").and_then(Value::as_str) != Some("tool_result") {
                        continue;
                    }
                    let id = block
                        .get("tool_use_id")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let output = match block.get("content") {
                        Some(Value::String(s)) => s.clone(),
                        Some(Value::Array(arr)) => arr
                            .iter()
                            .filter_map(|item| {
                                if item.get("type").and_then(Value::as_str) == Some("text") {
                                    item.get("text").and_then(Value::as_str).map(str::to_string)
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>()
                            .join("\n"),
                        _ => String::new(),
                    };
                    events.push(StreamEventKind::ToolResult { id, output });
                }
            }
            _ => {}
        }
        events
    }

    /// Core subprocess runner shared by [`run`](Self::run) and
    /// [`run_streaming`](Self::run_streaming).
    ///
    /// When `stream_tx` is `Some`, stream-json events are forwarded as
    /// [`StreamEvent`]s as each line arrives (live streaming). When `None`,
    /// the behaviour is identical to the original `run`.
    async fn run_impl(
        &self,
        input: &Signal,
        stream_tx: Option<mpsc::Sender<StreamEvent>>,
    ) -> AgentResult {
        let started = Instant::now();

        let prompt_text = match Self::prompt_text_from_input(input) {
            Ok(text) => text,
            Err(reason) => return self.failure(input, &reason, started),
        };

        let mut cmd = match self.build_command() {
            Ok(command) => command,
            Err(error) => {
                return self.failure(
                    input,
                    &format!("process confinement unavailable: {error}"),
                    started,
                );
            }
        };

        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(e) => {
                return self.failure(input, &format!("spawn failed: {e}"), started);
            }
        };
        let pid = child.id();
        if track_pids()
            && let Some(pid) = pid
        {
            register_spawned_pid(pid);
        }
        let stdout_pipe = child.stdout.take();
        let stderr_pipe = child.stderr.take();

        if let Some(mut stdin) = child.stdin.take()
            && let Err(e) = stdin.write_all(prompt_text.as_bytes()).await
        {
            let _ = kill_tree(&mut child, Duration::from_millis(GRACE_STDIN_CLOSE_MS)).await;
            if track_pids()
                && let Some(pid) = pid
            {
                unregister_pid(pid);
            }
            return self.failure(input, &format!("stdin write failed: {e}"), started);
        }

        tracing::info!(
            agent = %self.name,
            pid = pid.unwrap_or(0),
            timeout_s = self.timeout_ms / 1000,
            "agent started"
        );

        // Track activity across stdout and stderr for heartbeat messages.
        let has_activity = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let debug_enabled = Self::debug_enabled();

        // Stream stdout in real time, parsing stream-json events for progress
        // and forwarding StreamEventKind values when a sender is attached.
        let stdout_name = self.name.clone();
        let stdout_activity = has_activity.clone();
        let stdout_stream_tx = stream_tx;
        let stdout_handle = tokio::spawn(async move {
            let Some(pipe) = stdout_pipe else {
                return String::new();
            };
            let reader = BufReader::new(pipe);
            let mut lines = reader.lines();
            let mut collected = String::new();
            let mut text_bytes: usize = 0;
            let mut tool_count: usize = 0;

            while let Ok(Some(line)) = lines.next_line().await {
                collected.push_str(&line);
                collected.push('\n');
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                stdout_activity.store(true, std::sync::atomic::Ordering::Relaxed);
                if debug_enabled {
                    tracing::debug!("{line}");
                }

                // Parse stream-json events for progress reporting and optional
                // live-streaming to the upstream receiver.
                if let Some(event) = Self::parse_stream_event(trimmed) {
                    Self::emit_stream_summary(
                        &stdout_name,
                        &event,
                        &mut text_bytes,
                        &mut tool_count,
                    );
                    if let Some(tx) = &stdout_stream_tx {
                        for kind in Self::event_kinds_from_value(&event) {
                            // Ignore send errors: receiver may have dropped.
                            let _ = tx.send(StreamEvent::now(kind)).await;
                        }
                    }
                }
            }
            collected
        });

        // Stream stderr in real time. Raw stream JSON stays hidden in normal
        // mode, but debug mode echoes it verbatim for inspection.
        let agent_name = self.name.clone();
        let stderr_agent = self.clone();
        let stderr_activity = has_activity.clone();
        let stderr_handle = tokio::spawn(async move {
            let Some(pipe) = stderr_pipe else {
                return String::new();
            };
            let reader = BufReader::new(pipe);
            let mut lines = reader.lines();
            let mut collected = String::new();
            let mut text_bytes: usize = 0;
            let mut tool_count: usize = 0;
            while let Ok(Some(line)) = lines.next_line().await {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                stderr_activity.store(true, std::sync::atomic::Ordering::Relaxed);
                collected.push_str(&line);
                collected.push('\n');
                if let Some(event) = Self::parse_stream_event(trimmed) {
                    if debug_enabled {
                        tracing::debug!("{line}");
                    }
                    Self::emit_stream_summary(
                        &agent_name,
                        &event,
                        &mut text_bytes,
                        &mut tool_count,
                    );
                } else if debug_enabled {
                    tracing::debug!("{line}");
                } else if !stderr_agent.warn_and_filter_benign(&line) {
                    tracing::debug!(agent = %agent_name, "{line}");
                }
            }
            collected
        });

        // Heartbeat: print elapsed time every 15s when there's no other
        // output, so the user knows the agent is still running.
        let heartbeat_name = self.name.clone();
        let heartbeat_started = started;
        let heartbeat_activity = has_activity.clone();
        let heartbeat_handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(15));
            interval.tick().await; // skip immediate first tick
            loop {
                interval.tick().await;
                // Only print heartbeat when there's been no recent stdout/stderr activity.
                if !heartbeat_activity.swap(false, std::sync::atomic::Ordering::Relaxed) {
                    let elapsed = heartbeat_started.elapsed().as_secs();
                    tracing::debug!(agent = %heartbeat_name, elapsed_s = elapsed, "waiting for response");
                }
            }
        });

        let waited = match timeout(Duration::from_millis(self.timeout_ms), child.wait()).await {
            Ok(Ok(status)) => Ok(status),
            Ok(Err(e)) => Err(format!("wait failed: {e}")),
            Err(_) => Err(format!("{ATTEMPT_TIMEOUT_MARKER} {} ms", self.timeout_ms)),
        };
        heartbeat_handle.abort();
        let status = match waited {
            Ok(status) => status,
            Err(reason) => {
                let _ = kill_tree(&mut child, Duration::from_millis(GRACE_STDIN_CLOSE_MS)).await;
                if track_pids()
                    && let Some(pid) = pid
                {
                    unregister_pid(pid);
                }
                // Killed before its final `result` event: keep the usage it
                // streamed, so a long failed run is not recorded as free.
                let (stdout, stderr) = tokio::join!(
                    drain_killed_output(stdout_handle),
                    drain_killed_output(stderr_handle)
                );
                let stream_usage = Self::parse_stream_usage(&stdout, &self.model)
                    .merge(Self::parse_stream_usage(&stderr, &self.model));
                tracing::warn!(
                    agent = %self.name,
                    elapsed_s = started.elapsed().as_secs(),
                    streamed_cost_usd = stream_usage.cost_usd.unwrap_or_default(),
                    "agent {reason}"
                );
                return self.failure_with_stream_usage(input, &reason, started, &stream_usage);
            }
        };
        if track_pids()
            && let Some(pid) = pid
        {
            unregister_pid(pid);
        }

        let elapsed_secs = started.elapsed().as_secs();

        let stdout = stdout_handle.await.unwrap_or_default();
        let stderr = stderr_handle.await.unwrap_or_default();
        let wall_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        let stream_usage = Self::parse_stream_usage(&stdout, &self.model)
            .merge(Self::parse_stream_usage(&stderr, &self.model));

        if let Some(hit) = self.turn_cap_hit(&stdout, &stderr) {
            tracing::warn!(agent = %self.name, elapsed_s = elapsed_secs, "{hit}");
            return self.failure_with_stream_usage(input, &hit.to_string(), started, &stream_usage);
        }

        if !status.success() {
            let code = status
                .code()
                .map_or_else(|| "signal".to_string(), |c| c.to_string());
            tracing::warn!(agent = %self.name, exit_code = %code, elapsed_s = elapsed_secs, "agent failed");
            let reason = Self::failure_reason(&stdout, &stderr);
            return self.failure_with_stream_usage(
                input,
                &format!("exit {code}: {reason}"),
                started,
                &stream_usage,
            );
        }

        let text = {
            let text = Self::output_text(&stdout);
            if text.trim().is_empty() {
                Self::output_text(&stderr)
            } else {
                text
            }
        };
        if text.trim().is_empty() {
            tracing::warn!(
                agent = %self.name,
                elapsed_s = elapsed_secs,
                "agent finished but produced empty output"
            );
            return self.failure_with_stream_usage(
                input,
                "claude produced an empty response",
                started,
                &stream_usage,
            );
        }

        tracing::info!(
            agent = %self.name,
            elapsed_s = elapsed_secs,
            bytes = text.len(),
            "agent completed successfully"
        );

        let mut output_signal = input
            .derive(Kind::AgentOutput, Body::text(text))
            .provenance(Provenance::agent(&self.name))
            .tag("agent", &self.name)
            .tag(
                "model",
                stream_usage.model.as_deref().unwrap_or(&self.model),
            );
        if let Some(num_turns) = stream_usage.num_turns {
            output_signal = output_signal.tag("num_turns", num_turns.to_string());
        }
        let output_signal = output_signal.build();

        AgentResult::ok(output_signal)
            .with_trace(self.stderr_trace(&stderr))
            .with_usage_obs(self.usage_observation(&stream_usage, wall_ms))
    }
}

#[async_trait]
impl Agent for ClaudeCliAgent {
    /// Run the agent without streaming. Delegates to [`run_impl`](Self::run_impl).
    async fn run(&self, input: &Signal, _ctx: &Context) -> AgentResult {
        self.run_impl(input, None).await
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn backend_id(&self) -> &'static str {
        "claude_cli"
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    /// Run the agent with live streaming. Each parsed stream-json event is
    /// forwarded as a [`StreamEvent`] via `event_tx` as it arrives; the final
    /// [`AgentResult`] is identical to what [`run`](Self::run) returns.
    async fn run_streaming(
        &self,
        input: &Signal,
        _ctx: &Context,
        event_tx: mpsc::Sender<StreamEvent>,
    ) -> AgentResult {
        self.run_impl(input, Some(event_tx)).await
    }
}

/// Parsed usage metadata from Claude CLI stream-json output: the final
/// `result` event ([`UsageSource::ProviderReported`]), or what a killed run
/// streamed before it ([`UsageSource::Estimated`]).
#[derive(Debug, Clone, PartialEq, Default)]
struct StreamUsage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    cache_creation_tokens: Option<u64>,
    cache_read_tokens: Option<u64>,
    cost_usd: Option<f64>,
    model: Option<String>,
    /// Agent turns the CLI reported in its final `result` event, or the
    /// main-loop messages a killed run streamed.
    num_turns: Option<u64>,
    source: UsageSource,
}

impl StreamUsage {
    fn merge(mut self, other: Self) -> Self {
        match (&self.source, &other.source) {
            // A provider-reported total beats a partial estimate.
            (UsageSource::Unknown, _) | (UsageSource::Estimated, UsageSource::ProviderReported) => {
                return other;
            }
            (UsageSource::ProviderReported, UsageSource::ProviderReported) => {
                self.input_tokens = self.input_tokens.or(other.input_tokens);
                self.output_tokens = self.output_tokens.or(other.output_tokens);
                self.cache_creation_tokens =
                    self.cache_creation_tokens.or(other.cache_creation_tokens);
                self.cache_read_tokens = self.cache_read_tokens.or(other.cache_read_tokens);
                self.cost_usd = self.cost_usd.or(other.cost_usd);
                self.model = self.model.or(other.model);
                self.num_turns = self.num_turns.or(other.num_turns);
            }
            _ => {}
        }
        self
    }
}

/// Usage of the API messages a run streamed as stream-json `assistant`
/// events. The CLI emits one event per content block, each repeating its
/// message's usage, so a message id counts once (its largest values).
#[derive(Debug, Default)]
struct StreamedMessages {
    messages: Vec<StreamedMessage>,
    by_id: HashMap<String, usize>,
}

#[derive(Debug, Default)]
struct StreamedMessage {
    model: Option<String>,
    /// Main-loop message; a sub-agent's messages set `parent_tool_use_id`.
    top_level: bool,
    usage: Usage,
}

impl StreamedMessages {
    fn observe(&mut self, event: &Value) {
        let Some(message) = event.get("message") else {
            return;
        };
        let Some(usage) = message.get("usage").filter(|usage| usage.is_object()) else {
            return;
        };
        let index = match message.get("id").and_then(Value::as_str) {
            Some(id) => *self.by_id.entry(id.to_string()).or_insert_with(|| {
                self.messages.push(StreamedMessage::default());
                self.messages.len() - 1
            }),
            None => {
                self.messages.push(StreamedMessage::default());
                self.messages.len() - 1
            }
        };
        let streamed = &mut self.messages[index];
        if let Some(model) = message
            .get("model")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|model| !model.is_empty())
        {
            streamed.model = Some(model.to_string());
        }
        streamed.top_level = event.get("parent_tool_use_id").is_none_or(Value::is_null);
        let count = |keys: &[&str]| {
            ClaudeCliAgent::stream_usage_u64(usage, keys)
                .map_or(0, |count| u32::try_from(count).unwrap_or(u32::MAX))
        };
        let tokens = &mut streamed.usage;
        tokens.input_tokens = tokens.input_tokens.max(count(&["input_tokens"]));
        tokens.output_tokens = tokens.output_tokens.max(count(&["output_tokens"]));
        tokens.cache_create_tokens = tokens.cache_create_tokens.max(count(&[
            "cache_creation_input_tokens",
            "cache_creation_tokens",
        ]));
        tokens.cache_read_tokens = tokens
            .cache_read_tokens
            .max(count(&["cache_read_input_tokens", "cache_read_tokens"]));
    }

    /// The streamed totals as [`UsageSource::Estimated`] usage, each message
    /// priced from the model pricing table (`fallback_model` when it names
    /// none). Unknown when nothing was streamed.
    fn into_stream_usage(self, fallback_model: &str) -> StreamUsage {
        if self.messages.is_empty() {
            return StreamUsage::default();
        }
        let (mut input, mut output, mut cache_creation, mut cache_read) = (0_u64, 0, 0, 0);
        let mut cost_usd = None;
        for message in &self.messages {
            let mut usage = message.usage;
            input += u64::from(usage.input_tokens);
            output += u64::from(usage.output_tokens);
            cache_creation += u64::from(usage.cache_create_tokens);
            cache_read += u64::from(usage.cache_read_tokens);
            let model = message.model.as_deref().unwrap_or(fallback_model);
            if let Some(pricing) = model_meta(model).pricing {
                usage.fill_cost_from_pricing(
                    Some(pricing.input_per_m),
                    Some(pricing.output_per_m),
                    Some(pricing.cache_read_per_m),
                    Some(pricing.cache_write_per_m),
                );
                *cost_usd.get_or_insert(0.0) += f64::from(usage.cost_usd);
            }
        }
        let top_level = || self.messages.iter().filter(|message| message.top_level);
        StreamUsage {
            input_tokens: Some(input),
            output_tokens: Some(output),
            cache_creation_tokens: Some(cache_creation),
            cache_read_tokens: Some(cache_read),
            cost_usd,
            model: top_level().rev().find_map(|message| message.model.clone()),
            num_turns: Some(top_level().count() as u64),
            source: UsageSource::Estimated,
        }
    }
}

/// Longest wait for a killed run's output readers to reach end of file.
const KILLED_OUTPUT_DRAIN_MS: u64 = 2_000;

/// What `reader` collected from a killed run's pipe. A reader still blocked
/// after [`KILLED_OUTPUT_DRAIN_MS`] (a surviving descendant holds the pipe
/// open) is left behind, and its output is lost.
async fn drain_killed_output(reader: tokio::task::JoinHandle<String>) -> String {
    timeout(Duration::from_millis(KILLED_OUTPUT_DRAIN_MS), reader)
        .await
        .ok()
        .and_then(Result::ok)
        .unwrap_or_default()
}

async fn read_pipe_to_string<R>(pipe: &mut Option<R>) -> String
where
    R: AsyncRead + Unpin,
{
    let Some(reader) = pipe.as_mut() else {
        return String::new();
    };
    let mut bytes = Vec::new();
    if reader.read_to_end(&mut bytes).await.is_err() {
        return String::new();
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

const fn track_pids() -> bool {
    !cfg!(test)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::process::{Command as StdCommand, Stdio};
    use tempfile::tempdir;

    fn prompt(text: &str) -> Signal {
        Signal::builder(Kind::Prompt).body(Body::text(text)).build()
    }

    #[test]
    fn settings_json_contains_expected_hooks() {
        let value: Value = serde_json::from_str(&build_settings_json()).unwrap();
        let hooks = value
            .pointer("/hooks/PreToolUse/0/hooks")
            .and_then(Value::as_array)
            .expect("hooks array");
        assert_eq!(hooks.len(), 1);
        let hook = hooks.first().expect("single hook");
        assert!(
            hook.get("if").is_none(),
            "Claude hooks do not honor per-hook condition fields"
        );
        let command = hook
            .get("command")
            .and_then(Value::as_str)
            .expect("hook command");
        assert!(command.contains("tool_input"));
        assert!(command.contains("git\\s+checkout"));
        assert!(command.contains("git\\s+switch"));
        assert!(command.contains("git\\s+branch"));
        assert!(command.contains("git\\s+push"));
        assert!(command.contains("rm"));
    }

    #[test]
    fn settings_hook_allows_safe_bash_and_blocks_destructive_bash() {
        let value: Value = serde_json::from_str(&build_settings_json()).unwrap();
        let command = value
            .pointer("/hooks/PreToolUse/0/hooks/0/command")
            .and_then(Value::as_str)
            .expect("hook command");

        assert_eq!(run_hook_command(command, "echo ok").code(), Some(0));
        assert_eq!(
            run_hook_command(command, "git checkout main").code(),
            Some(2)
        );
    }

    fn run_hook_command(command: &str, bash_command: &str) -> std::process::ExitStatus {
        let mut child = StdCommand::new("sh")
            .arg("-c")
            .arg(command)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn hook command");
        let payload = serde_json::json!({
            "tool_input": {
                "command": bash_command,
            },
        })
        .to_string();
        child
            .stdin
            .as_mut()
            .expect("hook stdin")
            .write_all(payload.as_bytes())
            .expect("write hook payload");
        child.wait().expect("wait for hook")
    }

    #[test]
    fn parse_stream_usage_extracts_result_event_fields_and_model() {
        let usage = ClaudeCliAgent::parse_stream_usage(
            r#"{"type":"assistant","subtype":"message","message":{"content":[{"type":"text","text":"hello"}],"usage":{"input_tokens":999,"output_tokens":888,"cache_creation_input_tokens":777,"cache_read_input_tokens":666}}}
{"type":"result","session_id":"sess-1","model":"claude-sonnet-4-6","total_cost_usd":0.25,"usage":{"input_tokens":11,"output_tokens":22,"cache_creation_input_tokens":33,"cache_read_input_tokens":44}}"#,
            "claude-test-model",
        );
        assert_eq!(usage.source, UsageSource::ProviderReported);
        assert_eq!(usage.input_tokens, Some(11));
        assert_eq!(usage.output_tokens, Some(22));
        assert_eq!(usage.cache_creation_tokens, Some(33));
        assert_eq!(usage.cache_read_tokens, Some(44));
        assert_eq!(usage.cost_usd, Some(0.25));
        assert_eq!(usage.model.as_deref(), Some("claude-sonnet-4-6"));
    }

    #[test]
    fn parse_stream_usage_leaves_missing_fields_none_and_keeps_zeroes() {
        let usage = ClaudeCliAgent::parse_stream_usage(
            r#"{"type":"result","session_id":"sess-2","model":"claude-sonnet-4-6","total_cost_usd":0,"usage":{"input_tokens":0,"cache_read_input_tokens":5}}"#,
            "claude-test-model",
        );
        assert_eq!(usage.source, UsageSource::ProviderReported);
        assert_eq!(usage.input_tokens, Some(0));
        assert_eq!(usage.output_tokens, None);
        assert_eq!(usage.cache_creation_tokens, None);
        assert_eq!(usage.cache_read_tokens, Some(5));
        assert_eq!(usage.cost_usd, Some(0.0));
        assert_eq!(usage.model.as_deref(), Some("claude-sonnet-4-6"));
    }

    #[test]
    fn parse_stream_usage_accepts_cache_alias_fields() {
        let usage = ClaudeCliAgent::parse_stream_usage(
            r#"{"type":"result","session_id":"sess-3","model":"claude-sonnet-4-6","total_cost_usd":0.5,"usage":{"input_tokens":1,"output_tokens":2,"cache_creation_tokens":3,"cache_read_tokens":4}}"#,
            "claude-test-model",
        );
        assert_eq!(usage.cache_creation_tokens, Some(3));
        assert_eq!(usage.cache_read_tokens, Some(4));
        assert_eq!(usage.cost_usd, Some(0.5));
    }

    #[test]
    fn parse_stream_usage_stays_unknown_without_reported_usage() {
        let usage = ClaudeCliAgent::parse_stream_usage(
            r#"{"type":"assistant","subtype":"message","message":{"content":[{"type":"text","text":"hello"}],"usage":null}}
{"type":"tool","subtype":"result","tool_name":"Bash","tool_use_id":"tu_1","content":"done"}"#,
            "claude-sonnet-4-6",
        );
        assert_eq!(usage, StreamUsage::default());
    }

    #[test]
    fn parse_stream_usage_estimates_streamed_usage_without_result_event() {
        // One API message arrives as one `assistant` event per content block,
        // each repeating its usage; a sub-agent's message sets
        // `parent_tool_use_id`; msg_3 names no model.
        let usage = ClaudeCliAgent::parse_stream_usage(
            r#"{"type":"assistant","message":{"id":"msg_1","model":"claude-sonnet-4-6","content":[{"type":"text","text":"building"}],"usage":{"input_tokens":1000,"output_tokens":200,"cache_creation_input_tokens":3000,"cache_read_input_tokens":4000}},"parent_tool_use_id":null}
{"type":"assistant","message":{"id":"msg_1","model":"claude-sonnet-4-6","content":[{"type":"tool_use","id":"tu_1","name":"Task","input":{}}],"usage":{"input_tokens":1000,"output_tokens":200,"cache_creation_input_tokens":3000,"cache_read_input_tokens":4000}},"parent_tool_use_id":null}
{"type":"assistant","message":{"id":"msg_2","model":"claude-haiku-4-5","content":[{"type":"text","text":"sub-agent"}],"usage":{"input_tokens":500,"output_tokens":100}},"parent_tool_use_id":"tu_1"}
{"type":"assistant","message":{"id":"msg_3","content":[{"type":"text","text":"still going"}],"usage":{"input_tokens":10,"output_tokens":50,"cache_read_input_tokens":7000}}}"#,
            "claude-sonnet-4-6",
        );
        assert_eq!(usage.source, UsageSource::Estimated);
        assert_eq!(usage.input_tokens, Some(1_510));
        assert_eq!(usage.output_tokens, Some(350));
        assert_eq!(usage.cache_creation_tokens, Some(3_000));
        assert_eq!(usage.cache_read_tokens, Some(11_000));
        // Per million: Sonnet $3 in, $15 out, $0.30 cache read, $3.75 cache
        // write (msg_1, and msg_3 at the fallback model); Haiku $0.80 in,
        // $4 out (msg_2).
        let sonnet = 1_010.0 * 3.0 + 250.0 * 15.0 + 11_000.0 * 0.30 + 3_000.0 * 3.75;
        let haiku = 500.0 * 0.80 + 100.0 * 4.0;
        let cost = usage.cost_usd.expect("priced from the model table");
        assert!((cost - (sonnet + haiku) / 1e6).abs() < 1e-6, "{usage:?}");
        assert_eq!(usage.model.as_deref(), Some("claude-sonnet-4-6"));
        assert_eq!(usage.num_turns, Some(2), "main-loop messages only");
    }

    #[test]
    fn provider_reported_usage_beats_a_streamed_estimate() {
        let estimated = ClaudeCliAgent::parse_stream_usage(
            r#"{"type":"assistant","message":{"id":"msg_1","usage":{"input_tokens":1}}}"#,
            "claude-sonnet-4-6",
        );
        let reported = ClaudeCliAgent::parse_stream_usage(
            r#"{"type":"result","total_cost_usd":0.5,"usage":{"input_tokens":9}}"#,
            "claude-sonnet-4-6",
        );
        assert_eq!(estimated.source, UsageSource::Estimated);
        assert_eq!(estimated.clone().merge(reported.clone()), reported);
        assert_eq!(reported.clone().merge(estimated), reported);
    }

    #[test]
    fn command_preserves_explicit_deny_all_and_emits_denylist() {
        let command = ClaudeCliAgent::new("claude", ".", "claude-test-model")
            .with_allowed_tools("")
            .with_disallowed_tools("Bash,WebSearch")
            .build_command()
            .expect("build command");
        let args = command
            .as_std()
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        let tools = args
            .iter()
            .position(|arg| arg == "--tools")
            .expect("explicit deny-all flag");
        assert_eq!(args[tools + 1], "");
        let denied = args
            .iter()
            .position(|arg| arg == "--disallowed-tools")
            .expect("denylist flag");
        assert_eq!(args[denied + 1], "Bash,WebSearch");
    }

    #[test]
    fn bare_mode_replaces_the_builtin_prompt_even_without_roko_guidance() {
        let command = ClaudeCliAgent::new("claude", ".", "claude-test-model")
            .with_bare_mode(true)
            .build_command()
            .expect("build command");
        let args = command
            .as_std()
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        let system_prompt = args
            .iter()
            .position(|arg| arg == "--system-prompt")
            .expect("replacement prompt flag");
        assert_eq!(args[system_prompt + 1], "");
        assert!(!args.iter().any(|arg| arg == "--append-system-prompt"));
        assert!(!args.iter().any(|arg| arg == "--bare"));
    }

    #[test]
    fn full_mode_appends_roko_guidance_to_the_builtin_prompt() {
        let command = ClaudeCliAgent::new("claude", ".", "claude-test-model")
            .with_bare_mode(false)
            .with_system_prompt("system guidance")
            .build_command()
            .expect("build command");
        let args = command
            .as_std()
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        let appended = args
            .iter()
            .position(|arg| arg == "--append-system-prompt")
            .expect("append prompt flag");
        assert_eq!(args[appended + 1], "system guidance");
        assert!(!args.iter().any(|arg| arg == "--system-prompt"));
    }

    #[tokio::test]
    async fn runs_fake_claude_binary_and_passes_flags() {
        let tmp = tempdir().unwrap();
        let capture_args = tmp.path().join("args.txt");
        let capture_prompt = tmp.path().join("prompt.txt");
        let script = tmp.path().join("claude-fake.sh");
        let script_body = format!(
            r#"#!/bin/sh
set -eu
args_file="{args_file}"
prompt_file="{prompt_file}"
printf '%s\n' "$@" > "$args_file"
cat > "$prompt_file"
printf '%s\n' '{{"type":"content_block_delta","delta":{{"text":"hello"}}}}'
"#,
            args_file = capture_args.display(),
            prompt_file = capture_prompt.display(),
        );
        fs::write(&script, script_body).unwrap();
        #[cfg(unix)]
        {
            let mut perms = fs::metadata(&script).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&script, perms).unwrap();
        }

        let agent = ClaudeCliAgent::new(&script, tmp.path(), "claude-test-model")
            .with_system_prompt("system guidance")
            .with_allowed_tools("Read,Edit")
            .with_resume("session-123")
            .with_bare_mode(true);

        let result = agent.run(&prompt("hi there"), &Context::now()).await;
        assert!(result.success);
        assert_eq!(result.output.body.as_text().unwrap().trim(), "hello");

        let args_text = fs::read_to_string(&capture_args).unwrap();
        assert!(args_text.contains("--print"));
        assert!(args_text.contains("--verbose"));
        assert!(args_text.contains("--output-format"));
        assert!(args_text.contains("stream-json"));
        assert!(args_text.contains("--model"));
        assert!(args_text.contains("claude-test-model"));
        assert!(args_text.contains("--effort"));
        assert!(args_text.contains("medium"));
        assert!(args_text.contains("--max-turns"));
        assert!(args_text.contains("10"));
        assert!(args_text.contains("--system-prompt"));
        assert!(!args_text.contains("--append-system-prompt"));
        assert!(args_text.contains("system guidance"));
        assert!(args_text.contains("--settings"));
        assert!(args_text.contains("--dangerously-skip-permissions"));
        assert!(args_text.contains("--tools"));
        assert!(args_text.contains("Read,Edit"));
        assert!(args_text.contains("--resume"));
        assert!(args_text.contains("session-123"));

        let prompt_text = fs::read_to_string(&capture_prompt).unwrap();
        assert_eq!(prompt_text, "hi there");
    }

    #[tokio::test]
    async fn can_disable_dangerous_skip_permissions_flag() {
        let tmp = tempdir().unwrap();
        let capture_args = tmp.path().join("args.txt");
        let script = tmp.path().join("claude-fake.sh");
        let script_body = format!(
            r#"#!/bin/sh
set -eu
args_file="{args_file}"
printf '%s\n' "$@" > "$args_file"
cat >/dev/null
printf '%s\n' '{{"type":"content_block_delta","delta":{{"text":"ok"}}}}'
"#,
            args_file = capture_args.display(),
        );
        fs::write(&script, script_body).unwrap();
        #[cfg(unix)]
        {
            let mut perms = fs::metadata(&script).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&script, perms).unwrap();
        }

        let agent = ClaudeCliAgent::new(&script, tmp.path(), "claude-test-model")
            .with_dangerously_skip_permissions(false);
        let result = agent.run(&prompt("x"), &Context::now()).await;
        assert!(
            result.success,
            "{}",
            result.output.body.as_text().unwrap_or("unknown")
        );

        let args_text = fs::read_to_string(&capture_args).unwrap();
        assert!(!args_text.contains("--dangerously-skip-permissions"));
    }

    #[tokio::test]
    async fn optional_resume_none_omits_resume_flag() {
        let tmp = tempdir().unwrap();
        let capture_args = tmp.path().join("args.txt");
        let script = tmp.path().join("claude-fake.sh");
        let script_body = format!(
            r#"#!/bin/sh
set -eu
args_file="{args_file}"
printf '%s\n' "$@" > "$args_file"
cat >/dev/null
printf '%s\n' '{{"type":"content_block_delta","delta":{{"text":"ok"}}}}'
"#,
            args_file = capture_args.display(),
        );
        fs::write(&script, script_body).unwrap();
        #[cfg(unix)]
        {
            let mut perms = fs::metadata(&script).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&script, perms).unwrap();
        }

        let agent = ClaudeCliAgent::new(&script, tmp.path(), "claude-test-model")
            .with_optional_resume(None);
        let result = agent.run(&prompt("x"), &Context::now()).await;
        assert!(
            result.success,
            "{}",
            result.output.body.as_text().unwrap_or("unknown")
        );

        let args_text = fs::read_to_string(&capture_args).unwrap();
        assert!(!args_text.contains("--resume"));
    }

    #[tokio::test]
    async fn result_event_usage_is_threaded_into_agent_result() {
        let tmp = tempdir().unwrap();
        let script = tmp.path().join("claude-fake.sh");
        let script_body = r#"#!/bin/sh
set -eu
cat >/dev/null
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"hello"}}'
printf '%s\n' '{"type":"result","session_id":"sess-1","model":"claude-sonnet-4-6","total_cost_usd":0.25,"usage":{"input_tokens":11,"output_tokens":22,"cache_creation_input_tokens":33,"cache_read_input_tokens":44}}'
"#;
        fs::write(&script, script_body).unwrap();
        #[cfg(unix)]
        {
            let mut perms = fs::metadata(&script).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&script, perms).unwrap();
        }

        let agent = ClaudeCliAgent::new(&script, tmp.path(), "claude-test-model");
        let result = agent.run(&prompt("x"), &Context::now()).await;
        assert!(result.success);
        assert_eq!(result.output.body.as_text().unwrap().trim(), "hello");
        assert_eq!(result.output.tag("model"), Some("claude-sonnet-4-6"));
        assert_eq!(result.usage.input_tokens, 11);
        assert_eq!(result.usage.output_tokens, 22);
        assert_eq!(result.usage.cache_read_tokens, 44);
        assert_eq!(result.usage.cache_create_tokens, 33);
        assert!((result.usage.cost_usd - 0.25).abs() < 0.0001);
        let observation = result.usage_obs.expect("usage observation");
        assert_eq!(observation.source, UsageSource::ProviderReported);
        assert_eq!(observation.model.as_deref(), Some("claude-sonnet-4-6"));
    }

    #[tokio::test]
    async fn nonzero_exit_still_carries_result_event_usage() {
        let tmp = tempdir().unwrap();
        let script = tmp.path().join("claude-fake.sh");
        let script_body = r#"#!/bin/sh
set -eu
cat >/dev/null
printf '%s\n' '{"type":"result","session_id":"sess-2","model":"claude-sonnet-4-6","total_cost_usd":0.5,"usage":{"input_tokens":9,"output_tokens":8,"cache_creation_input_tokens":7,"cache_read_input_tokens":6}}'
exit 1
"#;
        fs::write(&script, script_body).unwrap();
        #[cfg(unix)]
        {
            let mut perms = fs::metadata(&script).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&script, perms).unwrap();
        }

        let agent = ClaudeCliAgent::new(&script, tmp.path(), "claude-test-model");
        let result = agent.run(&prompt("x"), &Context::now()).await;
        assert!(!result.success);
        assert_eq!(result.output.tag("model"), Some("claude-sonnet-4-6"));
        assert_eq!(result.usage.input_tokens, 9);
        assert_eq!(result.usage.output_tokens, 8);
        assert_eq!(result.usage.cache_read_tokens, 6);
        assert_eq!(result.usage.cache_create_tokens, 7);
        assert!((result.usage.cost_usd - 0.5).abs() < 0.0001);
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn a_timed_out_attempt_reports_the_usage_it_streamed() {
        let tmp = tempdir().unwrap();
        let script = tmp.path().join("claude-fake.sh");
        // Streams two API messages (the first as two content-block events),
        // then works past the timeout without reaching its `result` event.
        let script_body = r#"#!/bin/sh
cat >/dev/null
printf '%s\n' '{"type":"system","subtype":"init","session_id":"sess-t","model":"claude-sonnet-4-6"}'
printf '%s\n' '{"type":"assistant","message":{"id":"msg_1","model":"claude-sonnet-4-6","content":[{"type":"text","text":"building"}],"usage":{"input_tokens":1000,"output_tokens":200,"cache_creation_input_tokens":3000,"cache_read_input_tokens":4000}},"parent_tool_use_id":null}'
printf '%s\n' '{"type":"assistant","message":{"id":"msg_1","model":"claude-sonnet-4-6","content":[{"type":"tool_use","id":"tu_1","name":"Bash","input":{"command":"cargo build"}}],"usage":{"input_tokens":1000,"output_tokens":200,"cache_creation_input_tokens":3000,"cache_read_input_tokens":4000}},"parent_tool_use_id":null}'
printf '%s\n' '{"type":"assistant","message":{"id":"msg_2","model":"claude-sonnet-4-6","content":[{"type":"text","text":"still building"}],"usage":{"input_tokens":10,"output_tokens":50,"cache_creation_input_tokens":0,"cache_read_input_tokens":7000}},"parent_tool_use_id":null}'
sleep 30
"#;
        fs::write(&script, script_body).unwrap();
        let mut perms = fs::metadata(&script).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script, perms).unwrap();

        let agent =
            ClaudeCliAgent::new(&script, tmp.path(), "claude-sonnet-4-6").with_timeout_ms(1_000);
        let result = agent.run(&prompt("build it"), &Context::now()).await;

        assert!(!result.success);
        let text = result.output.body.as_text().expect("failure text");
        assert_eq!(text, "timed out after 1000 ms");
        assert!(crate::provider::error_classify::detect_attempt_timeout(
            text
        ));
        assert_eq!(result.output.tag("model"), Some("claude-sonnet-4-6"));
        assert_eq!(result.output.tag("num_turns"), Some("2"));
        assert_eq!(result.usage.input_tokens, 1_010);
        assert_eq!(result.usage.output_tokens, 250);
        assert_eq!(result.usage.cache_create_tokens, 3_000);
        assert_eq!(result.usage.cache_read_tokens, 11_000);
        // Sonnet per million: $3 in, $15 out, $0.30 cache read, $3.75 cache write.
        let expected = (1_010.0 * 3.0 + 250.0 * 15.0 + 11_000.0 * 0.30 + 3_000.0 * 3.75) / 1e6;
        assert!(
            (f64::from(result.usage.cost_usd) - expected).abs() < 1e-6,
            "{:?}",
            result.usage
        );
        let observation = result.usage_obs.expect("usage observation");
        assert_eq!(
            observation.source,
            UsageSource::Estimated,
            "a killed run's usage is partial"
        );
    }

    #[tokio::test]
    async fn benign_stderr_is_filtered_from_trace() {
        let tmp = tempdir().unwrap();
        let script = tmp.path().join("claude-fake.sh");
        let script_body = r#"#!/bin/sh
set -eu
cat >/dev/null
echo 'Claude CLI is starting up...' 1>&2
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"ok"}}'
"#;
        fs::write(&script, script_body).unwrap();
        #[cfg(unix)]
        {
            let mut perms = fs::metadata(&script).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&script, perms).unwrap();
        }

        let agent = ClaudeCliAgent::new(&script, tmp.path(), "claude-test-model");
        let result = agent.run(&prompt("x"), &Context::now()).await;
        assert!(
            result.success,
            "{}",
            result.output.body.as_text().unwrap_or("unknown")
        );
        assert!(result.trace.is_empty());
    }

    #[test]
    fn stderr_trace_skips_stream_json_lines() {
        let agent = ClaudeCliAgent::new("claude", ".", "claude-test-model");
        let trace = agent.stderr_trace(
            "unexpected stderr line\n{\"type\":\"content_block_delta\",\"delta\":{\"text\":\"ok\"}}\n",
        );
        assert_eq!(trace.len(), 1);
        assert_eq!(trace[0].body.as_text().unwrap(), "unexpected stderr line");
    }

    #[test]
    fn failure_reason_surfaces_session_limit_from_stdout_result() {
        let stdout = concat!(
            "{\"type\":\"system\",\"subtype\":\"init\",\"session_id\":\"s1\"}\n",
            "{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":true,",
            "\"result\":\"You\u{2019}ve hit your session limit \u{b7} resets 4pm (Europe/Berlin)\"}\n",
        );
        let reason = ClaudeCliAgent::failure_reason(stdout, "");
        assert!(
            reason.starts_with("provider usage exhausted: "),
            "unexpected reason: {reason}"
        );
        assert!(reason.contains("resets 4pm (Europe/Berlin)"), "{reason}");
        let exhaustion = detect_provider_exhaustion(&reason).expect("re-detectable");
        assert!(exhaustion.resets_at_ms.is_some());
    }

    #[test]
    fn failure_reason_prefers_stderr_then_result_then_generic() {
        assert_eq!(
            ClaudeCliAgent::failure_reason("", "Error: bad flag\n"),
            "Error: bad flag"
        );
        let stdout =
            "{\"type\":\"result\",\"is_error\":true,\"result\":\"Credit balance is too low\"}\n";
        assert_eq!(
            ClaudeCliAgent::failure_reason(stdout, ""),
            "Credit balance is too low"
        );
        assert_eq!(ClaudeCliAgent::failure_reason("", ""), "claude failed");
    }

    // ── event_kinds_from_value unit tests ─────────────────────────────

    #[test]
    fn event_kinds_text_block() {
        let event = serde_json::json!({
            "type": "assistant",
            "message": { "content": [{ "type": "text", "text": "hello" }] }
        });
        let kinds = ClaudeCliAgent::event_kinds_from_value(&event);
        assert_eq!(kinds.len(), 1);
        assert!(matches!(&kinds[0], StreamEventKind::TextDelta(t) if t == "hello"));
    }

    #[test]
    fn event_kinds_thinking_block() {
        let event = serde_json::json!({
            "type": "assistant",
            "message": { "content": [{ "type": "thinking", "thinking": "reasoning here" }] }
        });
        let kinds = ClaudeCliAgent::event_kinds_from_value(&event);
        assert_eq!(kinds.len(), 1);
        assert!(matches!(&kinds[0], StreamEventKind::ReasoningDelta(t) if t == "reasoning here"));
    }

    #[test]
    fn event_kinds_tool_use_block() {
        let event = serde_json::json!({
            "type": "assistant",
            "message": { "content": [{ "type": "tool_use", "id": "tu_1", "name": "Read", "input": { "path": "foo" } }] }
        });
        let kinds = ClaudeCliAgent::event_kinds_from_value(&event);
        assert_eq!(kinds.len(), 1);
        assert!(
            matches!(&kinds[0], StreamEventKind::ToolCallEnd { id, name, args }
            if id == "tu_1" && name == "Read" && args.get("path").and_then(Value::as_str) == Some("foo"))
        );
    }

    #[test]
    fn event_kinds_tool_event() {
        let event = serde_json::json!({
            "type": "tool",
            "subtype": "result",
            "tool_use_id": "tu_2",
            "tool_name": "Bash",
            "content": "output"
        });
        let kinds = ClaudeCliAgent::event_kinds_from_value(&event);
        assert_eq!(kinds.len(), 1);
        assert!(
            matches!(&kinds[0], StreamEventKind::ToolResult { id, output }
            if id == "tu_2" && output == "output")
        );
    }

    #[test]
    fn event_kinds_user_tool_result_string_content() {
        let event = serde_json::json!({
            "type": "user",
            "message": {
                "content": [{
                    "type": "tool_result",
                    "tool_use_id": "tu_3",
                    "content": "plain text result"
                }]
            }
        });
        let kinds = ClaudeCliAgent::event_kinds_from_value(&event);
        assert_eq!(kinds.len(), 1);
        assert!(
            matches!(&kinds[0], StreamEventKind::ToolResult { id, output }
            if id == "tu_3" && output == "plain text result")
        );
    }

    #[test]
    fn event_kinds_user_tool_result_array_content() {
        let event = serde_json::json!({
            "type": "user",
            "message": {
                "content": [{
                    "type": "tool_result",
                    "tool_use_id": "tu_4",
                    "content": [
                        { "type": "text", "text": "line one" },
                        { "type": "text", "text": "line two" }
                    ]
                }]
            }
        });
        let kinds = ClaudeCliAgent::event_kinds_from_value(&event);
        assert_eq!(kinds.len(), 1);
        assert!(
            matches!(&kinds[0], StreamEventKind::ToolResult { id, output }
            if id == "tu_4" && output == "line one\nline two")
        );
    }

    // ── run_streaming integration tests ───────────────────────────────

    #[tokio::test]
    #[cfg(unix)]
    async fn run_streaming_emits_events_in_order() {
        use tokio::sync::mpsc;
        let tmp = tempdir().unwrap();
        let script = tmp.path().join("claude-fake.sh");
        let script_body = r#"#!/bin/sh
cat >/dev/null
printf '%s\n' '{"type":"assistant","subtype":"message","message":{"content":[{"type":"text","text":"hello"},{"type":"tool_use","id":"tu_1","name":"Read","input":{"path":"foo"}}]}}'
printf '%s\n' '{"type":"tool","subtype":"result","tool_use_id":"tu_1","tool_name":"Read","content":"file contents"}'
printf '%s\n' '{"type":"result","subtype":"success","is_error":false,"result":"hello"}'
"#;
        fs::write(&script, script_body).unwrap();
        {
            let mut perms = fs::metadata(&script).unwrap().permissions();
            std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o755);
            fs::set_permissions(&script, perms).unwrap();
        }

        let agent = ClaudeCliAgent::new(&script, tmp.path(), "claude-test-model");
        let (tx, mut rx) = mpsc::channel(32);
        let result = agent
            .run_streaming(&prompt("hi"), &Context::now(), tx)
            .await;
        assert!(
            result.success,
            "{}",
            result.output.body.as_text().unwrap_or("unknown")
        );

        // Drain the channel.
        let mut kinds = Vec::new();
        while let Ok(event) = rx.try_recv() {
            kinds.push(event.kind);
        }

        // Verify order: TextDelta, ToolCallEnd, ToolResult (≥ those three in sequence).
        let text_pos = kinds
            .iter()
            .position(|k| matches!(k, StreamEventKind::TextDelta(t) if t == "hello"))
            .expect("TextDelta(hello) not found");
        let tool_end_pos = kinds
            .iter()
            .position(|k| {
                matches!(k, StreamEventKind::ToolCallEnd { id, name, .. } if id == "tu_1" && name == "Read")
            })
            .expect("ToolCallEnd not found");
        let tool_result_pos = kinds
            .iter()
            .position(|k| {
                matches!(k, StreamEventKind::ToolResult { id, output } if id == "tu_1" && output == "file contents")
            })
            .expect("ToolResult not found");

        assert!(
            text_pos < tool_end_pos,
            "TextDelta should precede ToolCallEnd"
        );
        assert!(
            tool_end_pos < tool_result_pos,
            "ToolCallEnd should precede ToolResult"
        );
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn run_and_run_streaming_agree_on_result() {
        use tokio::sync::mpsc;
        let tmp = tempdir().unwrap();
        let script = tmp.path().join("claude-fake.sh");
        let script_body = r#"#!/bin/sh
cat >/dev/null
printf '%s\n' '{"type":"assistant","subtype":"message","message":{"content":[{"type":"text","text":"response text"}]}}'
printf '%s\n' '{"type":"result","subtype":"success","is_error":false,"result":"response text","model":"claude-test-model","total_cost_usd":0.01,"usage":{"input_tokens":5,"output_tokens":3}}'
"#;
        fs::write(&script, script_body).unwrap();
        {
            let mut perms = fs::metadata(&script).unwrap().permissions();
            std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o755);
            fs::set_permissions(&script, perms).unwrap();
        }

        let run_result = ClaudeCliAgent::new(&script, tmp.path(), "claude-test-model")
            .run(&prompt("hi"), &Context::now())
            .await;

        let (tx, _rx) = mpsc::channel(32);
        let streaming_result = ClaudeCliAgent::new(&script, tmp.path(), "claude-test-model")
            .run_streaming(&prompt("hi"), &Context::now(), tx)
            .await;

        assert_eq!(run_result.success, streaming_result.success);
        assert_eq!(
            run_result.output.body.as_text().unwrap().trim(),
            streaming_result.output.body.as_text().unwrap().trim()
        );
        assert_eq!(
            run_result.usage.input_tokens,
            streaming_result.usage.input_tokens
        );
        assert_eq!(
            run_result.usage.output_tokens,
            streaming_result.usage.output_tokens
        );
    }
}

#[cfg(all(test, unix))]
mod turn_cap_tests {
    use super::*;
    use crate::provider::error_classify::{
        TurnCapHit, detect_provider_exhaustion, detect_turn_cap,
    };
    use std::os::unix::fs::PermissionsExt;

    fn fake_claude(dir: &std::path::Path, result_line: &str, exit_code: i32) -> PathBuf {
        let script = dir.join("claude-fake.sh");
        std::fs::write(
            &script,
            format!(
                "#!/bin/sh\ncat >/dev/null\nprintf '%s\\n' '{result_line}'\nexit {exit_code}\n"
            ),
        )
        .expect("write fake claude");
        let mut permissions = std::fs::metadata(&script).expect("metadata").permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&script, permissions).expect("make executable");
        script
    }

    fn prompt() -> Signal {
        Signal::builder(Kind::Prompt)
            .body(Body::text("do it"))
            .build()
    }

    #[tokio::test]
    async fn error_max_turns_is_a_turn_cap_hit_not_a_generic_failure() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let script = fake_claude(
            tmp.path(),
            r#"{"type":"result","subtype":"error_max_turns","is_error":true,"num_turns":2,"total_cost_usd":0.01}"#,
            1,
        );
        let agent = ClaudeCliAgent::new(&script, tmp.path(), "claude-test-model").with_max_turns(1);

        let result = agent.run(&prompt(), &Context::now()).await;
        assert!(!result.success);
        let text = result.output.body.as_text().expect("failure text");
        assert_eq!(
            detect_turn_cap(text),
            Some(TurnCapHit {
                num_turns: Some(2),
                cap: Some(1),
            }),
            "{text}"
        );
        assert!(detect_provider_exhaustion(text).is_none(), "{text}");
        assert_eq!(result.output.tag("num_turns"), Some("2"));
    }

    #[tokio::test]
    async fn successful_run_reports_the_cli_turn_count() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let script = fake_claude(
            tmp.path(),
            concat!(
                r#"{"type":"content_block_delta","delta":{"text":"done"}}"#,
                "\n",
                r#"{"type":"result","subtype":"success","is_error":false,"num_turns":3,"result":"done"}"#,
            ),
            0,
        );
        let agent = ClaudeCliAgent::new(&script, tmp.path(), "claude-test-model");

        let result = agent.run(&prompt(), &Context::now()).await;
        assert!(result.success, "{:?}", result.output.body.as_text());
        assert_eq!(result.output.tag("num_turns"), Some("3"));
    }
}
