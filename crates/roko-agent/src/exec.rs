//! `ExecAgent` — choose this for generic prompt-in/stdout CLIs that are not
//! Claude-specific and do not need a provider-aware protocol.
//!
//! Works with tools like `ollama run`, `mods`, `llm`, or just `cat` / `echo`
//! for testing. Prefer [`ClaudeCliAgent`](crate::ClaudeCliAgent) when the
//! backend is actually Claude Code and you need its richer tool, session, and
//! safety wiring. This is the lowest-common-denominator LLM integration.

use crate::agent::{Agent, AgentResult, derived_output};
use crate::process::{
    GRACE_SIGTERM_MS, GRACE_STDIN_CLOSE_MS, KillTreeOnDrop, ResourceLimits, apply_credential_scrub,
    benign_stderr_warn_once, classify_benign_stderr, confined_command, kill_tree,
    register_spawned_pid, set_process_group, unregister_pid,
};
use crate::provider::error_classify::{ProviderExhaustion, detect_provider_exhaustion};
use crate::safety::SafetyLayer;
use crate::usage::Usage;
use async_trait::async_trait;
use roko_core::child_env::CredentialScrub;
use roko_core::defaults::DEFAULT_REQUEST_TIMEOUT_MS;
use roko_core::tool::ToolResult;
use roko_core::{Body, Context, Kind, Provenance, Signal};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::sync::oneshot;
use tokio::time::timeout;
use tokio_util::task::AbortOnDropHandle;

// ── Codex operation policy ───────────────────────────────────────────────────

/// Codex CLI operation types that can be individually allowed or denied.
///
/// Codex's built-in file/shell/web operations are not roko tool calls; they
/// bypass [`crate::dispatcher::ToolDispatcher`] because Codex owns its own
/// tool loop. This policy type checks the JSONL event stream as Codex writes
/// it to enforce roko's deny/allow list at the operation level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodexOperationType {
    /// Shell/process execution (`command_execution` events).
    CommandExecution,
    /// File read/write (`file_change` events).
    FileChange,
}

impl CodexOperationType {
    /// Return the canonical Codex JSONL `item.type` string for this operation.
    #[must_use]
    pub fn as_item_type(&self) -> &'static str {
        match self {
            Self::CommandExecution => "command_execution",
            Self::FileChange => "file_change",
        }
    }

    /// Attempt to parse a Codex `item.type` string into an operation type.
    #[must_use]
    pub fn from_item_type(s: &str) -> Option<Self> {
        match s {
            "command_execution" => Some(Self::CommandExecution),
            "file_change" => Some(Self::FileChange),
            _ => None,
        }
    }

    /// Map roko canonical/provider tool names that correspond to this Codex
    /// operation type.  Used when deriving a policy from an [`AgentContract`].
    ///
    /// [`AgentContract`]: crate::safety::contract::AgentContract
    fn matches_tool_name(&self, name: &str) -> bool {
        let lower = name.to_lowercase();
        match self {
            Self::CommandExecution => {
                lower == "bash"
                    || lower == "shell"
                    || lower == "command_execution"
                    || lower == "run_command"
                    || lower == "execute"
            }
            Self::FileChange => {
                lower == "write_file"
                    || lower == "edit_file"
                    || lower == "multi_edit"
                    || lower == "apply_patch"
                    || lower == "create_file"
                    || lower == "file_change"
                    || lower == "notebook_edit"
            }
        }
    }
}

/// Pre-execution policy that governs which Codex CLI built-in operations roko
/// will accept in the response stream.
///
/// The broker is fail-closed: any operation type that is not explicitly allowed
/// (when an allowlist is set) or that is explicitly denied will cause the
/// entire agent turn to be rejected with a policy-violation error.
///
/// # Rationale
///
/// Codex runs its own internal tool loop and does not surface individual
/// tool-call approval points to the roko dispatcher. The only practical
/// enforcement boundary is the JSONL output stream that Codex emits as each
/// operation starts and completes. `ExecAgent` checks that stream while Codex
/// runs and stops the process at the first denied operation, which bounds
/// what the operation can do but cannot prevent it from starting.
///
/// For stronger pre-execution guarantees consider sandboxing the subprocess
/// at the OS level (namespaces, Landlock, Apple Sandbox) or using
/// `--dangerously-bypass-approvals-and-sandbox=false` (the default) so Codex
/// itself prompts before destructive actions.
#[derive(Debug, Clone, Default)]
pub struct CodexOperationPolicy {
    /// When `Some`, only the listed operation types are permitted.  All others
    /// are denied regardless of `denied`.
    pub allowed: Option<Vec<CodexOperationType>>,
    /// Operations that are always denied, even when `allowed` is `None`.
    pub denied: Vec<CodexOperationType>,
}

impl CodexOperationPolicy {
    /// Build a deny-all policy: every Codex operation type is rejected.
    #[must_use]
    pub fn deny_all() -> Self {
        Self {
            allowed: Some(Vec::new()),
            denied: Vec::new(),
        }
    }

    /// Build a permissive policy: all operation types are accepted.
    #[must_use]
    pub fn allow_all() -> Self {
        Self::default()
    }

    /// Derive a policy from an [`AgentContract`].
    ///
    /// - If the contract's `allowed_tools` is `Some([])` (explicit deny-all),
    ///   all Codex operations are denied.
    /// - If `allowed_tools` is `Some([…])`, only operations whose tool names
    ///   appear in the allowlist are permitted.
    /// - `ForbiddenTools` governance rules are always applied as a denylist.
    ///
    /// [`AgentContract`]: crate::safety::contract::AgentContract
    #[must_use]
    pub fn from_contract(contract: &crate::safety::contract::AgentContract) -> Self {
        let forbidden = contract.forbidden_tool_names();
        let denied: Vec<CodexOperationType> = ALL_CODEX_OPERATION_TYPES
            .iter()
            .filter(|op| forbidden.iter().any(|name| op.matches_tool_name(name)))
            .cloned()
            .collect();

        let allowed = contract.allowed_tools.as_ref().map(|tools| {
            ALL_CODEX_OPERATION_TYPES
                .iter()
                .filter(|op| {
                    // An operation is allowed when at least one of its
                    // corresponding tool names appears in the allowlist.
                    tools.iter().any(|name| op.matches_tool_name(name))
                })
                .cloned()
                .collect::<Vec<_>>()
        });

        Self { allowed, denied }
    }

    /// Returns `true` when `op` is permitted by this policy.
    ///
    /// Deny rules always win over allow rules (fail-closed).
    #[must_use]
    pub fn permits(&self, op: &CodexOperationType) -> bool {
        // Explicit deny wins unconditionally.
        if self.denied.contains(op) {
            return false;
        }
        // When an allowlist is set, the operation must appear in it.
        if let Some(ref allowed) = self.allowed {
            return allowed.contains(op);
        }
        true
    }

    /// Returns `true` when the policy has at least one constraint (i.e. is not
    /// trivially permissive).
    #[must_use]
    pub fn has_constraints(&self) -> bool {
        !self.denied.is_empty() || self.allowed.is_some()
    }
}

const ALL_CODEX_OPERATION_TYPES: &[CodexOperationType] = &[
    CodexOperationType::CommandExecution,
    CodexOperationType::FileChange,
];

// ── JSONL operation broker ───────────────────────────────────────────────────

/// Scan raw Codex JSONL output for operation types that violate `policy`.
///
/// Returns `Ok(())` when all observed operations are permitted, or `Err` with
/// a human-readable description of the first policy violation found.
///
/// `ExecAgent` runs this after Codex exits, as the last check before roko
/// persists or acts on the output. It catches the denials that
/// [`CodexStreamBroker`] had no chance to act on: one in a final line without
/// a newline, or one that arrived as Codex exited.
fn check_codex_output_against_policy(
    raw: &str,
    policy: &CodexOperationPolicy,
) -> Result<(), String> {
    if !policy.has_constraints() {
        return Ok(());
    }
    match raw
        .lines()
        .find_map(|line| codex_line_violation(line, policy))
    {
        Some(violation) => Err(violation),
        None => Ok(()),
    }
}

/// The violation in one line of Codex JSONL: a description of the operation
/// it starts or completes, when `policy` denies that operation.
fn codex_line_violation(line: &str, policy: &CodexOperationPolicy) -> Option<String> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    let event = serde_json::from_str::<serde_json::Value>(line).ok()?;
    let event_type = event.get("type").and_then(|v| v.as_str()).unwrap_or("");
    // Enforce on both item.started (pre-output) and item.completed (post-output).
    if event_type != "item.started" && event_type != "item.completed" {
        return None;
    }
    let item = event.get("item")?;
    let item_type = item.get("type").and_then(|v| v.as_str()).unwrap_or("");
    let op = CodexOperationType::from_item_type(item_type)?;
    if policy.permits(&op) {
        return None;
    }
    let detail = match op {
        CodexOperationType::CommandExecution => {
            let cmd = item
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("<unknown>");
            format!("command_execution denied by policy: {cmd}")
        }
        CodexOperationType::FileChange => {
            let paths: Vec<&str> = item
                .get("changes")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|c| c.get("path").and_then(|v| v.as_str()))
                        .collect()
                })
                .unwrap_or_default();
            if paths.is_empty() {
                "file_change denied by policy".to_string()
            } else {
                format!("file_change denied by policy: {}", paths.join(", "))
            }
        }
    };
    Some(detail)
}

/// Checks Codex's JSONL while the process runs, so that `ExecAgent` can stop
/// it when a denied operation starts rather than find the operation after
/// Codex exits.
struct CodexStreamBroker {
    policy: CodexOperationPolicy,
    /// Bytes of output already checked; they always end at a newline.
    checked: usize,
    /// Told the first denied operation; `None` once it has been.
    denied: Option<oneshot::Sender<String>>,
}

impl CodexStreamBroker {
    /// Check each complete line of `output` past the bytes already checked.
    fn check(&mut self, output: &[u8]) {
        while self.denied.is_some() {
            let rest = &output[self.checked..];
            let Some(end) = rest.iter().position(|byte| *byte == b'\n') else {
                return;
            };
            let line = String::from_utf8_lossy(&rest[..end]);
            self.checked += end + 1;
            if let Some(violation) = codex_line_violation(&line, &self.policy)
                && let Some(denied) = self.denied.take()
            {
                let _ = denied.send(violation);
            }
        }
    }
}

/// An agent that spawns a subprocess, pipes the input's text body to stdin,
/// and captures stdout as the output.
///
/// # Example
///
/// ```ignore
/// // Echo the prompt back (degenerate but demonstrates flow):
/// let agent = ExecAgent::new("cat", vec![], SafetyLayer::with_defaults());
/// let prompt = Signal::builder(Kind::Prompt).body(Body::text("ping")).build();
/// let result = agent.run(&prompt, &Context::now()).await;
/// assert_eq!(result.output.body.as_text().unwrap().trim(), "ping");
/// ```
pub struct ExecAgent {
    program: String,
    args: Vec<String>,
    env: Vec<(String, String)>,
    /// Which inherited provider credentials the subprocess loses.
    credential_scrub: CredentialScrub,
    current_dir: Option<PathBuf>,
    safety: SafetyLayer,
    timeout_ms: u64,
    kill_grace_ms: u64,
    resource_limits: Option<ResourceLimits>,
    name: String,
    /// Optional text prepended to stdin before the prompt, separated by
    /// `\n\n---\n\n`. Used by Codex CLI to pass a system prompt on stdin.
    stdin_prefix: Option<String>,
    /// When true, parse stdout as Codex CLI JSONL and extract `agent_message`
    /// text fields instead of returning raw JSONL.
    extract_codex_jsonl: bool,
    /// Optional operation-level policy broker for Codex CLI JSONL output.
    ///
    /// When set, each Codex operation event in the output stream is checked
    /// against this policy as it arrives. The first denied operation stops
    /// the process and fails the turn with a policy-violation error.
    /// Only meaningful when `extract_codex_jsonl` is `true`.
    codex_operation_policy: Option<CodexOperationPolicy>,
}

impl ExecAgent {
    /// An agent that invokes `program` with `args`, piping input on stdin.
    #[must_use]
    pub fn new(program: impl Into<String>, args: Vec<String>, safety: SafetyLayer) -> Self {
        let program = program.into();
        let name = format!("exec:{program}");
        Self {
            program,
            args,
            env: Vec::new(),
            credential_scrub: CredentialScrub::default(),
            current_dir: None,
            safety,
            timeout_ms: DEFAULT_REQUEST_TIMEOUT_MS,
            kill_grace_ms: GRACE_SIGTERM_MS,
            resource_limits: None,
            name,
            stdin_prefix: None,
            extract_codex_jsonl: false,
            codex_operation_policy: None,
        }
    }

    /// Override the subprocess timeout in milliseconds (default 2 minutes).
    #[must_use]
    pub const fn with_timeout_ms(mut self, ms: u64) -> Self {
        self.timeout_ms = ms;
        self
    }

    /// Override the SIGTERM grace period used before SIGKILL on timeout.
    #[must_use]
    pub const fn with_kill_grace_ms(mut self, grace_ms: u64) -> Self {
        self.kill_grace_ms = grace_ms;
        self
    }

    /// Override the agent's display name.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Add an env var to the spawned subprocess (e.g. `OLLAMA_NOPROGRESS=1`).
    #[must_use]
    pub fn with_env_var(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    /// Add multiple env vars at once.
    #[must_use]
    pub fn with_env<I, K, V>(mut self, vars: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        for (k, v) in vars {
            self.env.push((k.into(), v.into()));
        }
        self
    }

    /// Replace the policy for which inherited credentials the subprocess
    /// loses. The default owns no provider credential: every known provider
    /// key, every name roko loaded from a `.env` file and roko's own
    /// credentials are stripped.
    #[must_use]
    pub fn with_credential_scrub(mut self, scrub: CredentialScrub) -> Self {
        self.credential_scrub = scrub;
        self
    }

    /// Run the subprocess from a specific working directory.
    #[must_use]
    pub fn with_current_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.current_dir = Some(dir.into());
        self
    }

    /// Apply OS resource limits to the spawned subprocess.
    #[must_use]
    pub fn with_resource_limits(mut self, limits: ResourceLimits) -> Self {
        self.resource_limits = Some(limits);
        self
    }

    /// Attach a safety layer to the subprocess runtime.
    #[must_use]
    pub fn with_safety_layer(mut self, safety: SafetyLayer) -> Self {
        self.safety = safety;
        self
    }

    /// Prepend text to stdin before the prompt, separated by `\n\n---\n\n`.
    ///
    /// Used by Codex CLI to pass a system prompt on stdin since it lacks a
    /// dedicated `--system-prompt` flag.
    #[must_use]
    pub fn with_stdin_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.stdin_prefix = Some(prefix.into());
        self
    }

    /// When enabled, parse stdout as Codex CLI JSONL (`--json` output) and
    /// extract `agent_message` text from `item.completed` events.
    #[must_use]
    pub const fn with_extract_codex_jsonl(mut self, extract: bool) -> Self {
        self.extract_codex_jsonl = extract;
        self
    }

    /// Attach a Codex operation policy broker.
    ///
    /// When set and `extract_codex_jsonl` is enabled, the raw JSONL output is
    /// checked for `command_execution` and `file_change` operation events as
    /// Codex writes it. The first operation that violates the policy stops
    /// the process, and the entire agent turn is rejected (fail-closed).
    ///
    /// Use [`CodexOperationPolicy::from_contract`] to derive a policy from an
    /// [`AgentContract`](crate::safety::contract::AgentContract).
    #[must_use]
    pub fn with_codex_operation_policy(mut self, policy: CodexOperationPolicy) -> Self {
        self.codex_operation_policy = Some(policy);
        self
    }
}

/// Extract agent message text from Codex CLI `--json` JSONL output.
///
/// Parses each line as JSON, looks for `item.completed` events with
/// `item.type == "agent_message"`, and concatenates the `item.text` fields.
fn extract_codex_text(jsonl: &str) -> String {
    use serde_json::Value;
    let mut text = String::new();
    for line in jsonl.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(event) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        // Extract text from: {"type":"item.completed","item":{"type":"agent_message","text":"..."}}
        if event.get("type").and_then(Value::as_str) == Some("item.completed")
            && let Some(item) = event.get("item")
            && item.get("type").and_then(Value::as_str) == Some("agent_message")
            && let Some(msg_text) = item.get("text").and_then(Value::as_str)
        {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(msg_text);
        }
    }
    text
}

#[async_trait]
#[allow(clippy::too_many_lines)]
impl Agent for ExecAgent {
    async fn run(&self, input: &Signal, _ctx: &Context) -> AgentResult {
        let started = Instant::now();
        if let Err(err) = self.safety.check_exec_command(&self.program, &self.args) {
            return self.failure_signal(
                input,
                &format!("exec blocked by safety layer: {err}"),
                started,
            );
        }

        let prompt_text = match input.body.as_text() {
            Ok(s) => s.to_string(),
            Err(_) => {
                // Attempt JSON fallback: serialize body as JSON string.
                match serde_json::to_string(&input.body) {
                    Ok(s) => s,
                    Err(e) => {
                        return self.failure_signal(
                            input,
                            &format!("input body not readable as text or json: {e}"),
                            started,
                        );
                    }
                }
            }
        };

        // Spawn subprocess.
        let mut cmd = match confined_command(&self.program, self.resource_limits.as_ref()) {
            Ok(command) => command,
            Err(error) => {
                return self.failure_signal(
                    input,
                    &format!("process confinement unavailable: {error}"),
                    started,
                );
            }
        };
        cmd.args(&self.args);
        apply_credential_scrub(&mut cmd, &self.credential_scrub);
        for (k, v) in &self.env {
            cmd.env(k, v);
        }
        if let Some(dir) = &self.current_dir {
            cmd.current_dir(dir);
        }
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.kill_on_drop(true);
        set_process_group(&mut cmd);
        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                return self.failure_signal(input, &format!("spawn failed: {e}"), started);
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

        // Write prompt to stdin, then close it.
        // When a stdin_prefix is set (e.g. system prompt for Codex CLI),
        // prepend it with a separator so the agent sees both pieces.
        if let Some(mut stdin) = child.stdin.take() {
            let full_stdin = match &self.stdin_prefix {
                Some(prefix) if !prefix.trim().is_empty() => {
                    format!("{}\n\n---\n\n{}", prefix.trim(), prompt_text)
                }
                _ => prompt_text.clone(),
            };
            if let Err(e) = stdin.write_all(full_stdin.as_bytes()).await {
                let _ = kill_tree(&mut child, Duration::from_millis(GRACE_STDIN_CLOSE_MS)).await;
                if track_pids()
                    && let Some(pid) = pid
                {
                    unregister_pid(pid);
                }
                return self.failure_signal(input, &format!("stdin write failed: {e}"), started);
            }
            drop(stdin);
        }

        tracing::info!(
            agent = %self.name,
            pid = pid.unwrap_or(0),
            timeout_s = self.timeout_ms / 1000,
            "agent started"
        );

        let has_activity = Arc::new(AtomicBool::new(false));

        // Stream stdout in chunks, reporting progress without altering content.
        // A Codex run's operations are checked against its policy as they
        // arrive.
        let (denied_tx, mut denied_rx) = oneshot::channel();
        let mut broker = self.codex_stream_broker(denied_tx);
        let stdout_name = self.name.clone();
        let stdout_activity = has_activity.clone();
        let stdout_handle = tokio::spawn(async move {
            let Some(mut pipe) = stdout_pipe else {
                return String::new();
            };
            let mut buf = [0u8; 8192];
            let mut collected = Vec::new();
            let mut last_report: usize = 0;
            loop {
                match pipe.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        collected.extend_from_slice(&buf[..n]);
                        stdout_activity.store(true, Ordering::Relaxed);
                        if let Some(broker) = broker.as_mut() {
                            broker.check(&collected);
                        }
                        // Report progress every ~4KB.
                        if collected.len() - last_report >= 4096 || last_report == 0 {
                            tracing::debug!(
                                agent = %stdout_name,
                                bytes = collected.len(),
                                "receiving output"
                            );
                            last_report = collected.len();
                        }
                    }
                    Err(_) => break,
                }
            }
            String::from_utf8_lossy(&collected).into_owned()
        });

        // Stream stderr in real time.
        let stderr_name = self.name.clone();
        let stderr_activity = has_activity.clone();
        let stderr_handle = tokio::spawn(async move {
            let Some(pipe) = stderr_pipe else {
                return String::new();
            };
            let reader = BufReader::new(pipe);
            let mut lines = reader.lines();
            let mut collected = String::new();
            while let Ok(Some(line)) = lines.next_line().await {
                if !line.trim().is_empty() {
                    stderr_activity.store(true, Ordering::Relaxed);
                    tracing::debug!(agent = %stderr_name, "{line}");
                }
                collected.push_str(&line);
                collected.push('\n');
            }
            collected
        });

        // Heartbeat when no output activity, aborted however the run ends,
        // a dropped run included.
        let heartbeat_name = self.name.clone();
        let heartbeat_started = started;
        let heartbeat_activity = has_activity.clone();
        let heartbeat_handle = AbortOnDropHandle::new(tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(15));
            interval.tick().await;
            loop {
                interval.tick().await;
                if !heartbeat_activity.swap(false, Ordering::Relaxed) {
                    let elapsed = heartbeat_started.elapsed().as_secs();
                    tracing::debug!(agent = %heartbeat_name, elapsed_s = elapsed, "waiting for response");
                }
            }
        }));

        // Wait for exit with timeout. An operation the Codex policy denies
        // stops the process at once.
        let waited = tokio::select! {
            waited = timeout(Duration::from_millis(self.timeout_ms), child.wait()) => waited,
            Ok(violation) = &mut denied_rx => {
                heartbeat_handle.abort();
                let _ = kill_tree(&mut child, Duration::ZERO).await;
                if track_pids()
                    && let Some(pid) = pid
                {
                    unregister_pid(pid);
                }
                tracing::warn!(
                    agent = %self.name,
                    %violation,
                    "Codex operation denied by policy broker; process stopped"
                );
                return self.failure_signal(
                    input,
                    &format!("Codex operation policy violation: {violation}"),
                    started,
                );
            }
        };
        let status = match waited {
            Ok(Ok(status)) => status,
            Ok(Err(e)) => {
                heartbeat_handle.abort();
                if track_pids()
                    && let Some(pid) = pid
                {
                    unregister_pid(pid);
                }
                return self.failure_signal(input, &format!("wait failed: {e}"), started);
            }
            Err(_) => {
                heartbeat_handle.abort();
                let _ = kill_tree(&mut child, Duration::from_millis(self.kill_grace_ms)).await;
                if track_pids()
                    && let Some(pid) = pid
                {
                    unregister_pid(pid);
                }
                return self.failure_signal(
                    input,
                    &format!("timed out after {} ms", self.timeout_ms),
                    started,
                );
            }
        };
        if track_pids()
            && let Some(pid) = pid
        {
            unregister_pid(pid);
        }

        heartbeat_handle.abort();
        let elapsed_secs = started.elapsed().as_secs();

        // A process the agent started and left running can hold the output
        // pipes open, so the readers would never see EOF and the run would
        // hang after the agent exited (gap-5d3b82). Give them
        // `EXITED_OUTPUT_DRAIN_MS`, then end the run's process group and keep
        // what they read.
        let (mut stdout_handle, mut stderr_handle) = (stdout_handle, stderr_handle);
        let drained = timeout(Duration::from_millis(EXITED_OUTPUT_DRAIN_MS), async {
            tokio::join!(&mut stdout_handle, &mut stderr_handle)
        })
        .await;
        let (raw_stdout, raw_stderr) = match drained {
            Ok((stdout, stderr)) => (stdout.unwrap_or_default(), stderr.unwrap_or_default()),
            Err(_) => {
                tracing::warn!(
                    agent = %self.name,
                    "agent exited, a process it started still holds its output; ending its group"
                );
                // The root was reaped, so only its process group is
                // signalled: SIGTERM, then SIGKILL.
                let mut group = KillTreeOnDrop::new(pid);
                group.root_reaped();
                drop(group);
                tokio::join!(
                    drain_killed_output(stdout_handle),
                    drain_killed_output(stderr_handle)
                )
            }
        };

        // ── Codex operation policy broker ────────────────────────────────────
        // Scan the raw JSONL before extracting text.  The live check above
        // stops the process at a denied operation; this scan catches one it
        // had no chance to act on.  Fail-closed: any denied or
        // unrecognised-in-policy operation rejects the whole turn.
        if self.extract_codex_jsonl {
            if let Some(ref policy) = self.codex_operation_policy {
                if let Err(violation) = check_codex_output_against_policy(&raw_stdout, policy) {
                    tracing::warn!(
                        agent = %self.name,
                        %violation,
                        "Codex operation denied by policy broker"
                    );
                    return self.failure_signal(
                        input,
                        &format!("Codex operation policy violation: {violation}"),
                        started,
                    );
                }
            }
        }

        let stdout = if self.extract_codex_jsonl {
            let extracted = extract_codex_text(&raw_stdout);
            self.scrub_text(&extracted)
        } else {
            self.scrub_text(&raw_stdout)
        };
        let stderr = self.scrub_text(&raw_stderr);
        let wall_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);

        if !status.success() {
            let code = status
                .code()
                .map_or_else(|| "signal".into(), |c| c.to_string());
            tracing::warn!(agent = %self.name, exit_code = %code, elapsed_s = elapsed_secs, "agent failed");
            let reason = usage_exhaustion(&raw_stdout, &stderr).map_or_else(
                || first_line(&stderr).to_string(),
                |exhaustion| self.scrub_text(&exhaustion.into_error().to_string()),
            );
            return self.failure_signal(input, &format!("exit {code}: {reason}"), started);
        }

        if let Err(err) = self
            .safety
            .check_recovery(&ToolResult::text(stdout.clone()))
        {
            return self.failure_signal(
                input,
                &format!("exec result blocked by safety layer: {err}"),
                started,
            );
        }

        tracing::info!(
            agent = %self.name,
            elapsed_s = elapsed_secs,
            bytes = stdout.len(),
            "agent completed successfully"
        );

        let out_signal = derived_output(input, Kind::AgentOutput, Body::text(stdout.clone()))
            .provenance(Provenance::agent(&self.name))
            .tag("agent", &self.name)
            .tag("exit_code", "0")
            .build();

        // Trace: one signal per non-empty stderr line (as AgentMessage events).
        let trace = stderr_trace(&self.name, &stderr);

        AgentResult::ok(out_signal)
            .with_trace(trace)
            .with_usage(Usage {
                input_tokens: u32::try_from(prompt_text.len() / 4).unwrap_or(u32::MAX),
                output_tokens: u32::try_from(stdout.len() / 4).unwrap_or(u32::MAX),
                wall_ms,
                ..Default::default()
            })
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn backend_id(&self) -> &'static str {
        "exec"
    }

    fn supports_streaming(&self) -> bool {
        // We collect all output before returning — not streaming.
        false
    }
}

impl ExecAgent {
    /// The live check for a Codex run whose policy has constraints; `denied`
    /// is told the first operation it denies.
    fn codex_stream_broker(&self, denied: oneshot::Sender<String>) -> Option<CodexStreamBroker> {
        let policy = self.codex_operation_policy.as_ref()?;
        if !self.extract_codex_jsonl || !policy.has_constraints() {
            return None;
        }
        Some(CodexStreamBroker {
            policy: policy.clone(),
            checked: 0,
            denied: Some(denied),
        })
    }

    fn failure_signal(&self, input: &Signal, reason: &str, started: Instant) -> AgentResult {
        let wall_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        let output = derived_output(input, Kind::AgentOutput, Body::text(reason))
            .provenance(Provenance::agent(&self.name))
            .tag("agent", &self.name)
            .tag("failed", "true")
            .build();
        AgentResult::fail(output).with_usage(Usage {
            wall_ms,
            ..Default::default()
        })
    }

    fn scrub_text(&self, content: &str) -> String {
        self.safety.scrub_text(content)
    }
}

/// Usage-window refusal behind a failed exec ("You've hit your usage limit …
/// try again in 2 hours"): human stderr, or a Codex JSONL `error` /
/// `turn.failed` event on stdout. Other stdout events are agent output and are
/// never scanned, so quoted text cannot quarantine a provider.
fn usage_exhaustion(raw_stdout: &str, stderr: &str) -> Option<ProviderExhaustion> {
    detect_provider_exhaustion(stderr).or_else(|| {
        raw_stdout
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line.trim()).ok())
            .filter(|event| {
                matches!(
                    event.get("type").and_then(serde_json::Value::as_str),
                    Some("error" | "turn.failed")
                )
            })
            .find_map(|event| {
                event
                    .pointer("/message")
                    .or_else(|| event.pointer("/error/message"))
                    .and_then(serde_json::Value::as_str)
                    .and_then(detect_provider_exhaustion)
            })
    })
}

fn first_line(s: &str) -> &str {
    s.lines().next().unwrap_or(s)
}

const fn track_pids() -> bool {
    !cfg!(test)
}

/// How long a run's output readers may take to reach EOF after the agent
/// exited, before the run's process group is ended (gap-5d3b82).
#[cfg(not(test))]
const EXITED_OUTPUT_DRAIN_MS: u64 = 5_000;

/// Tests keep the grace short: a reader that only needed more time still
/// finishes within [`KILLED_OUTPUT_DRAIN_MS`].
#[cfg(test)]
const EXITED_OUTPUT_DRAIN_MS: u64 = 300;

/// How long the readers of an ended process group may take to finish.
const KILLED_OUTPUT_DRAIN_MS: u64 = 2_000;

/// What `reader` collected from an ended run's pipe. A reader still blocked
/// after [`KILLED_OUTPUT_DRAIN_MS`] is left behind, and its output is lost.
async fn drain_killed_output(reader: tokio::task::JoinHandle<String>) -> String {
    timeout(Duration::from_millis(KILLED_OUTPUT_DRAIN_MS), reader)
        .await
        .ok()
        .and_then(Result::ok)
        .unwrap_or_default()
}

fn maybe_warn_and_filter_benign(name: &str, line: &str) -> bool {
    if let Some(benign) = classify_benign_stderr(line) {
        if benign_stderr_warn_once(benign.key) {
            tracing::warn!(agent = %name, "{}", benign.summary);
        }
        return true;
    }
    false
}

fn stderr_trace(name: &str, stderr: &str) -> Vec<Signal> {
    stderr
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter(|line| !maybe_warn_and_filter_benign(name, line))
        .map(|line| {
            Signal::builder(Kind::AgentMessage)
                .body(Body::text(line))
                .provenance(Provenance::agent(name))
                .tag("stream", "stderr")
                .build()
        })
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn prompt(text: &str) -> Signal {
        Signal::builder(Kind::Prompt).body(Body::text(text)).build()
    }

    fn exec_agent(program: impl Into<String>, args: Vec<String>) -> ExecAgent {
        ExecAgent::new(program, args, SafetyLayer::with_defaults())
    }

    #[test]
    fn usage_exhaustion_reads_codex_error_events_but_not_agent_output() {
        let failed = concat!(
            "{\"type\":\"item.completed\",\"item\":{\"type\":\"agent_message\",\"text\":\"ok\"}}\n",
            "{\"type\":\"turn.failed\",\"error\":{\"message\":\"You've hit your usage limit. ",
            "Upgrade to Pro or try again in 2 hours 5 minutes.\"}}\n",
        );
        let exhaustion = usage_exhaustion(failed, "").expect("codex usage limit");
        assert!(
            exhaustion
                .message
                .starts_with("You've hit your usage limit")
        );
        assert!(exhaustion.resets_at_ms.is_some());

        let quoted = "{\"type\":\"item.completed\",\"item\":{\"type\":\"agent_message\",\
                      \"text\":\"You've hit your usage limit\"}}\n";
        assert!(usage_exhaustion(quoted, "").is_none());
        assert!(usage_exhaustion("", "error: connection reset").is_none());
    }

    /// A dropped run leaves no task of its own behind: its heartbeat stops
    /// with it, and its output readers end once its process is killed.
    #[tokio::test]
    #[cfg(unix)]
    async fn a_dropped_exec_run_stops_its_heartbeat() {
        let temp = tempfile::tempdir().expect("tempdir");
        let started = temp.path().join("started");
        let agent = exec_agent(
            "sh",
            vec![
                "-c".to_string(),
                format!("touch '{}'; exec sleep 30", started.display()),
            ],
        );
        let run = tokio::spawn(async move {
            let ctx = Context::now();
            agent.run(&prompt("x"), &ctx).await
        });
        for _ in 0..400 {
            if started.exists() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        assert!(started.exists(), "the command started");
        let metrics = tokio::runtime::Handle::current().metrics();

        run.abort();
        assert!(run.await.expect_err("the run is cancelled").is_cancelled());

        let mut alive = metrics.num_alive_tasks();
        for _ in 0..200 {
            if alive == 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
            alive = metrics.num_alive_tasks();
        }
        assert_eq!(alive, 0, "a task of the dropped run outlived it");
    }

    /// gap-5d3b82: the agent exits while a process it started still holds
    /// its output open. The run ends soon after the exit with what the agent
    /// printed, and that process is killed.
    #[tokio::test]
    #[cfg(unix)]
    async fn exited_exec_agent_with_open_stdout_does_not_hang() {
        let temp = tempfile::tempdir().expect("tempdir");
        let holder = temp.path().join("holder.pid");
        let agent = exec_agent(
            "sh",
            vec![
                "-c".to_string(),
                format!("sleep 600 & echo $! > '{}'; echo done", holder.display()),
            ],
        );

        let result = timeout(
            Duration::from_secs(30),
            agent.run(&prompt("x"), &Context::now()),
        )
        .await
        .expect("the run ends although a process it started holds its output");
        assert!(result.success, "{:?}", result.output.body.as_text());
        assert_eq!(result.output.body.as_text().unwrap().trim(), "done");

        let pid = std::fs::read_to_string(&holder)
            .expect("the holder's pid")
            .trim()
            .to_string();
        let mut alive = true;
        for _ in 0..50 {
            alive = std::process::Command::new("kill")
                .args(["-0", &pid])
                .stderr(Stdio::null())
                .status()
                .is_ok_and(|status| status.success());
            if !alive {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        assert!(!alive, "process {pid} still holds the run's output");
    }

    #[tokio::test]
    async fn failed_exec_reports_usage_exhaustion() {
        let agent = exec_agent(
            "sh",
            vec![
                "-c".into(),
                "echo \"You've hit your usage limit. Try again in 3 hours.\" >&2; exit 1".into(),
            ],
        );
        let result = agent.run(&prompt(""), &Context::now()).await;
        assert!(!result.success);
        let text = result.output.body.as_text().unwrap();
        assert!(
            text.starts_with("exit 1: provider usage exhausted: You've hit your usage limit"),
            "{text}"
        );
    }

    #[tokio::test]
    async fn env_vars_reach_subprocess() {
        let agent = exec_agent("sh", vec!["-c".into(), "echo $ROKO_TEST_VAR".into()])
            .with_env_var("ROKO_TEST_VAR", "hello_from_env");
        let result = agent.run(&prompt(""), &Context::now()).await;
        assert!(result.success);
        let out = result.output.body.as_text().unwrap();
        assert_eq!(out.trim(), "hello_from_env");
    }

    #[tokio::test]
    async fn with_env_accepts_iterator() {
        let agent = exec_agent("sh", vec!["-c".into(), "echo $A-$B".into()])
            .with_env([("A", "alpha"), ("B", "beta")]);
        let result = agent.run(&prompt(""), &Context::now()).await;
        assert!(result.success);
        assert_eq!(result.output.body.as_text().unwrap().trim(), "alpha-beta");
    }

    #[tokio::test]
    async fn current_dir_reaches_subprocess() {
        let temp = tempfile::tempdir().expect("tempdir");
        let agent = exec_agent("pwd", vec![]).with_current_dir(temp.path());
        let result = agent.run(&prompt(""), &Context::now()).await;
        assert!(result.success);
        let expected = std::fs::canonicalize(temp.path()).expect("canonical tempdir");
        let actual = std::fs::canonicalize(result.output.body.as_text().unwrap().trim())
            .expect("canonical subprocess pwd");
        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn cat_echoes_prompt() {
        let agent = exec_agent("cat", vec![]);
        let result = agent.run(&prompt("roundtrip text"), &Context::now()).await;
        assert!(result.success);
        let out = result.output.body.as_text().unwrap();
        assert_eq!(out, "roundtrip text");
    }

    #[tokio::test]
    async fn exit_code_failure_marks_result_failed() {
        // `false` always exits non-zero.
        let agent = exec_agent("false", vec![]);
        let result = agent.run(&prompt("x"), &Context::now()).await;
        assert!(!result.success);
        assert_eq!(result.output.tag("failed"), Some("true"));
    }

    #[tokio::test]
    async fn nonexistent_binary_fails_gracefully() {
        let agent = exec_agent("definitely_not_a_real_binary_xyz", vec![]);
        let result = agent.run(&prompt("x"), &Context::now()).await;
        assert!(!result.success);
        assert!(
            result
                .output
                .body
                .as_text()
                .unwrap()
                .contains("spawn failed")
        );
    }

    #[tokio::test(start_paused = true)]
    async fn timeout_fails_result() {
        let agent = exec_agent("sleep", vec!["10".into()]).with_timeout_ms(100);
        let input = prompt("x");
        let ctx = Context::now();
        let run = tokio::spawn(async move { agent.run(&input, &ctx).await });
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_millis(100)).await;
        let result = run.await.expect("exec task should join");
        assert!(!result.success);
        assert!(result.output.body.as_text().unwrap().contains("timed out"));
    }

    #[tokio::test]
    async fn output_tracks_input_as_lineage() {
        let agent = exec_agent("cat", vec![]);
        let input = prompt("lineage test");
        let input_id = input.id;
        let result = agent.run(&input, &Context::now()).await;
        assert!(result.success);
        assert_eq!(result.output.lineage, vec![input_id]);
    }

    #[tokio::test]
    async fn usage_has_estimated_tokens() {
        let agent = exec_agent("cat", vec![]);
        let text = "x".repeat(400); // ~100 tokens
        let result = agent.run(&prompt(&text), &Context::now()).await;
        assert!(result.success);
        assert!(result.usage.input_tokens >= 90);
        assert!(result.usage.input_tokens <= 110);
        assert!(result.usage.wall_ms > 0);
    }

    #[tokio::test]
    async fn stderr_becomes_trace_signals() {
        // Use `sh -c` to emit both stdout and stderr.
        let agent = exec_agent(
            "sh",
            vec!["-c".into(), "echo hello; echo 'warning: x' 1>&2".into()],
        );
        let result = agent.run(&prompt(""), &Context::now()).await;
        assert!(result.success);
        assert!(result.output.body.as_text().unwrap().contains("hello"));
        assert_eq!(result.trace.len(), 1);
        assert_eq!(result.trace[0].kind, Kind::AgentMessage);
        assert!(result.trace[0].body.as_text().unwrap().contains("warning"));
    }

    #[tokio::test]
    async fn safety_blocks_dangerous_shell_before_spawn() {
        let temp = tempfile::tempdir().expect("tempdir");
        let sentinel = temp.path().join("sentinel");
        let command = format!("touch {}; rm -rf /", sentinel.display());
        let agent = exec_agent("sh", vec!["-c".into(), command])
            .with_current_dir(temp.path())
            .with_safety_layer(SafetyLayer::with_defaults());
        let result = agent.run(&prompt(""), &Context::now()).await;
        assert!(!result.success);
        assert!(
            result
                .output
                .body
                .as_text()
                .unwrap()
                .contains("blocked by safety layer")
        );
        assert!(!sentinel.exists());
    }

    #[tokio::test]
    async fn safety_allows_safe_shell_wrapper() {
        let agent = exec_agent("sh", vec!["-c".into(), "echo ok".into()])
            .with_safety_layer(SafetyLayer::with_defaults());
        let result = agent.run(&prompt(""), &Context::now()).await;
        assert!(result.success);
        assert_eq!(result.output.body.as_text().unwrap().trim(), "ok");
    }

    #[tokio::test]
    async fn safety_scrubs_stdout_and_stderr() {
        let secret = "sk-ant-api03-abcdefghij1234567890abcdefghij1234567890abcdefghij1234567890abcdefghij1234-AAAAAA";
        let command = format!("printf '%s' '{secret}'; printf '%s\\n' '{secret}' 1>&2");
        let agent = exec_agent("sh", vec!["-c".into(), command])
            .with_safety_layer(SafetyLayer::with_defaults());
        let result = agent.run(&prompt(""), &Context::now()).await;
        assert!(result.success);
        let output = result.output.body.as_text().unwrap();
        assert!(!output.contains(secret));
        assert!(output.contains("[REDACTED]"));
        assert_eq!(result.trace.len(), 1);
        let stderr = result.trace[0].body.as_text().unwrap();
        assert!(!stderr.contains(secret));
        assert!(stderr.contains("[REDACTED]"));
    }

    #[tokio::test]
    async fn safety_blocks_direct_git_force_push_before_spawn() {
        let agent = exec_agent(
            "git",
            vec![
                "push".into(),
                "--force".into(),
                "origin".into(),
                "main".into(),
            ],
        )
        .with_safety_layer(SafetyLayer::with_defaults());
        let result = agent.run(&prompt(""), &Context::now()).await;
        assert!(!result.success);
        assert!(
            result
                .output
                .body
                .as_text()
                .unwrap()
                .contains("blocked by safety layer")
        );
    }

    #[tokio::test]
    async fn benign_stderr_is_suppressed_from_trace() {
        let agent = exec_agent(
            "sh",
            vec![
                "-c".into(),
                "echo ok; echo 'Claude CLI is starting up...' 1>&2".into(),
            ],
        );
        let result = agent.run(&prompt(""), &Context::now()).await;
        assert!(result.success);
        assert!(result.trace.is_empty());
    }

    #[test]
    fn codex_policy_from_contract_maps_tool_names_to_operations() {
        use crate::safety::contract::{AgentContract, GovernanceRule};

        let forbids_bash = AgentContract {
            governance: vec![GovernanceRule::ForbiddenTools(vec!["Bash".into()])],
            ..AgentContract::default()
        };
        let policy = CodexOperationPolicy::from_contract(&forbids_bash);
        assert!(!policy.permits(&CodexOperationType::CommandExecution));
        assert!(policy.permits(&CodexOperationType::FileChange));

        // The live run's allowlist: no command tool, one edit tool.
        let allowlist = AgentContract {
            allowed_tools: Some(vec!["grep".into(), "read_file".into(), "write_file".into()]),
            ..AgentContract::default()
        };
        let policy = CodexOperationPolicy::from_contract(&allowlist);
        assert!(!policy.permits(&CodexOperationType::CommandExecution));
        assert!(policy.permits(&CodexOperationType::FileChange));

        let restricted = CodexOperationPolicy::from_contract(&AgentContract::restricted("x"));
        assert!(!restricted.permits(&CodexOperationType::CommandExecution));
        assert!(!restricted.permits(&CodexOperationType::FileChange));

        let open = CodexOperationPolicy::from_contract(&AgentContract::default());
        assert!(!open.has_constraints());
        assert!(open.permits(&CodexOperationType::CommandExecution));
    }

    #[test]
    fn codex_policy_denial_wins_over_the_allowlist() {
        let policy = CodexOperationPolicy {
            allowed: Some(vec![CodexOperationType::CommandExecution]),
            denied: vec![CodexOperationType::CommandExecution],
        };
        assert!(!policy.permits(&CodexOperationType::CommandExecution));
        assert!(!policy.permits(&CodexOperationType::FileChange));
        assert!(CodexOperationPolicy::allow_all().permits(&CodexOperationType::FileChange));
        assert!(!CodexOperationPolicy::deny_all().permits(&CodexOperationType::FileChange));
    }

    #[test]
    fn codex_output_scan_reports_the_first_denied_operation() {
        let policy = CodexOperationPolicy {
            allowed: None,
            denied: vec![CodexOperationType::FileChange],
        };
        let output = concat!(
            "not json\n",
            r#"{"type":"item.started","item":{"type":"command_execution","command":"ls"}}"#,
            "\n",
            r#"{"type":"item.completed","item":{"type":"file_change","#,
            r#""changes":[{"path":"src/a.rs"},{"path":"src/b.rs"}]}}"#,
            "\n",
        );
        let violation = check_codex_output_against_policy(output, &policy).unwrap_err();
        assert_eq!(violation, "file_change denied by policy: src/a.rs, src/b.rs");

        let allow_all = CodexOperationPolicy::allow_all();
        assert!(check_codex_output_against_policy(output, &allow_all).is_ok());
        let message = r#"{"type":"item.completed","item":{"type":"agent_message","text":"hi"}}"#;
        let deny_all = CodexOperationPolicy::deny_all();
        assert!(check_codex_output_against_policy(message, &deny_all).is_ok());
    }

    #[test]
    fn codex_stream_broker_checks_each_complete_line_once() {
        let (denied_tx, mut denied_rx) = oneshot::channel();
        let mut broker = CodexStreamBroker {
            policy: CodexOperationPolicy::deny_all(),
            checked: 0,
            denied: Some(denied_tx),
        };
        let line = r#"{"type":"item.started","item":{"type":"file_change","changes":[]}}"#;
        let mut output = line.as_bytes()[..20].to_vec();
        broker.check(&output);
        assert!(denied_rx.try_recv().is_err(), "half a line is not checked");

        output.extend_from_slice(&line.as_bytes()[20..]);
        output.push(b'\n');
        broker.check(&output);
        assert_eq!(denied_rx.try_recv().unwrap(), "file_change denied by policy");
        assert_eq!(broker.checked, output.len());
        assert!(broker.denied.is_none());
    }

    /// gap-baab0a: the broker stops Codex when an operation its contract
    /// denies starts, rather than reading the output after Codex exits.
    #[tokio::test]
    #[cfg(unix)]
    async fn codex_restrictive_contract_is_enforced() {
        use crate::safety::contract::{AgentContract, GovernanceRule};

        let temp = tempfile::tempdir().expect("tempdir");
        let pid_file = temp.path().join("pid");
        let started = concat!(
            r#"{"type":"item.started","item":{"id":"item_0","type":"command_execution","#,
            r#""command":"git fsck","status":"in_progress"}}"#,
        );
        let script = format!(
            "echo $$ > '{}'; printf '%s\\n' '{started}'; exec sleep 30",
            pid_file.display()
        );
        let contract = AgentContract {
            role: "reviewer".into(),
            governance: vec![GovernanceRule::ForbiddenTools(vec!["bash".into()])],
            ..AgentContract::default()
        };
        let agent = exec_agent("sh", vec!["-c".into(), script])
            .with_timeout_ms(10_000)
            .with_extract_codex_jsonl(true)
            .with_codex_operation_policy(CodexOperationPolicy::from_contract(&contract));

        let run_started = Instant::now();
        let result = agent.run(&prompt("x"), &Context::now()).await;

        assert!(!result.success);
        let text = result.output.body.as_text().unwrap();
        assert!(text.contains("Codex operation policy violation"), "{text}");
        assert!(text.contains("git fsck"), "{text}");
        assert!(run_started.elapsed() < Duration::from_secs(5));
        let pid = std::fs::read_to_string(&pid_file).expect("pid file");
        let pid: u32 = pid.trim().parse().expect("pid");
        let identity = crate::process::identity::process_identity(pid);
        assert!(identity.is_none(), "codex {pid} still runs: {identity:?}");
    }
}
