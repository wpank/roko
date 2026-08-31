//! Agent process spawning and stream-JSON parsing.
//!
//! Spawns the configured CLI provider, parses stdout lines into
//! [`AgentEvent`]s, and sends them through a tokio mpsc channel.

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use std::{fmt, future::Future};

use anyhow::{Context, Result};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, warn};

use roko_agent::process::{confined_command, kill_tree, set_process_group};
use roko_core::defaults::DEFAULT_AGENT_TURN_LIMIT;
use roko_core::obs::LogScrubber;

use crate::dispatch_v2::{
    CliDispatchProvider, CliDispatchRequest, CliPluginMcpConfig, CliProtocol, CliProviderConfig,
};

use super::types::{AgentEvent, RunConfig};

const FAST_AGENT_TURN_LIMIT: u32 = 6;

/// Cancellation and absolute deadline applied while a CLI runtime is being
/// materialized. Once [`AgentHandle`] is returned, ordinary attempt ownership
/// takes over.
#[derive(Clone)]
pub struct AgentStartupControl {
    pub deadline: tokio::time::Instant,
    pub cancel: CancellationToken,
    pub cleanup_grace: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentStartupInterruption {
    Deadline,
    Cancelled,
}

pub enum AgentStartupError {
    Failed(anyhow::Error),
    Interrupted {
        interruption: AgentStartupInterruption,
        cleanup_error: Option<String>,
        unconfirmed: Option<AgentHandle>,
    },
}

impl fmt::Debug for AgentStartupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AgentStartupError")
            .field("message", &self.to_string())
            .finish()
    }
}

impl fmt::Display for AgentStartupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Failed(error) => write!(f, "{error}"),
            Self::Interrupted {
                interruption,
                cleanup_error,
                ..
            } => {
                write!(f, "CLI startup {interruption:?}")?;
                if let Some(error) = cleanup_error {
                    write!(f, "; cleanup failed: {error}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for AgentStartupError {}

async fn await_startup_step<T>(
    control: Option<&AgentStartupControl>,
    future: impl Future<Output = T>,
) -> std::result::Result<T, AgentStartupInterruption> {
    let Some(control) = control else {
        return Ok(future.await);
    };
    tokio::pin!(future);
    tokio::select! {
        biased;
        _ = control.cancel.cancelled() => Err(AgentStartupInterruption::Cancelled),
        _ = tokio::time::sleep_until(control.deadline) => Err(AgentStartupInterruption::Deadline),
        value = &mut future => Ok(value),
    }
}

async fn interrupt_startup_child(
    child: &mut Child,
    pid: u32,
    control: &AgentStartupControl,
) -> Option<String> {
    let result = kill_tree(child, control.cleanup_grace)
        .await
        .map_err(|error| error.to_string());
    let confirmed = child
        .try_wait()
        .map(|status| status.is_some())
        .map_err(|error| error.to_string());
    if confirmed == Ok(true) {
        roko_agent::process::unregister_pid(pid);
    }
    match (result, confirmed) {
        (Ok(()), Ok(true)) => None,
        (left, right) => Some(format!("kill_tree={left:?}, process_absent={right:?}")),
    }
}

/// Configuration for spawning a single agent.
#[derive(Debug, Clone)]
pub struct AgentSpawnConfig {
    /// The prompt to send to the agent.
    pub prompt: String,
    /// System prompt translated into the selected provider's input format.
    pub system_prompt: String,
    /// Model to use.
    pub model: String,
    /// Working directory for the agent.
    pub workdir: PathBuf,
    /// Maximum turns the agent can take.
    pub max_turns: u32,
    /// Optional reasoning effort hint for providers that support it.
    pub effort: Option<String>,
    /// Legacy/default CLI binary path.
    pub program: PathBuf,
    /// Whether to skip permission checks.
    pub dangerously_skip_permissions: bool,
    /// Optional MCP config path.
    pub mcp_config: Option<PathBuf>,
    /// Optional session ID to resume.
    pub resume_session: Option<String>,
    /// Agent identifier for logging.
    pub agent_id: String,
    /// Materialized CLI provider selected by provider/model resolution.
    pub cli_provider: Option<CliProviderConfig>,
    /// Tool names the agent must not invoke, derived from safety contracts.
    ///
    /// Set by the caller (event_loop) after contract loading — not populated
    /// by `from_run_config`. Passed through to `CliDispatchRequest`.
    pub disallowed_tools: Vec<String>,
    /// Binding tool allowlist derived from the role contract and task.
    ///
    /// `Some(vec![])` is intentionally distinct from `None`: it means the
    /// restricted fallback denied every tool and must be forwarded as such.
    pub allowed_tools: Option<Vec<String>>,
    /// Contract-scoped bridge for in-process declarative-plugin handlers.
    pub plugin_mcp: Option<CliPluginMcpConfig>,
    /// Extra environment variables injected into agent subprocesses (e.g.
    /// `CARGO_TARGET_DIR` for build-cache sharing with worktrees).
    pub extra_env: Vec<(String, String)>,
}

impl AgentSpawnConfig {
    /// Create a spawn config from a `RunConfig` and task-specific details.
    pub fn from_run_config(
        config: &RunConfig,
        prompt: String,
        system_prompt: String,
        model: String,
        agent_id: String,
    ) -> Self {
        Self {
            prompt,
            system_prompt,
            model,
            workdir: config.workdir.clone(),
            max_turns: DEFAULT_AGENT_TURN_LIMIT,
            effort: None,
            program: config.claude_program.clone(),
            dangerously_skip_permissions: config.dangerously_skip_permissions,
            mcp_config: config.mcp_config.clone(),
            resume_session: config.resume_session.clone(),
            agent_id,
            cli_provider: None,
            disallowed_tools: Vec::new(),
            allowed_tools: None,
            plugin_mcp: None,
            extra_env: Vec::new(),
        }
    }

    /// Attach a resolved CLI provider.
    #[must_use]
    pub fn with_cli_provider(mut self, provider: CliProviderConfig) -> Self {
        self.cli_provider = Some(provider);
        self
    }
}

/// Handle to a running agent process.
pub struct AgentHandle {
    /// PID of the agent process.
    pub pid: u32,
    /// The child process.
    child: Child,
    /// Task reading stdout lines.
    reader_task: Option<JoinHandle<()>>,
    /// Task reading stderr lines, when stderr was captured.
    stderr_reader_task: Option<JoinHandle<()>>,
    /// Keeps provider configuration alive until the subprocess is absent.
    _ephemeral_config_dir: Option<tempfile::TempDir>,
}

/// Result of attempting to terminate an agent and its stream readers.
#[must_use]
pub enum AgentTermination {
    /// The process exited and all reader tasks stopped intentionally.
    Confirmed { pid: u32 },
    /// Process or reader cleanup failed. Ownership is returned for retry.
    Failed {
        handle: AgentHandle,
        process_confirmed: bool,
        process_errors: Vec<String>,
        reader_errors: Vec<String>,
    },
}

/// Result of naturally waiting for an agent and all stream readers.
#[must_use]
pub enum AgentWait {
    /// The child is absent. Reader failures remain structured producer errors.
    Confirmed {
        pid: u32,
        exit_code: Option<i32>,
        reader_errors: Vec<String>,
    },
    /// Process absence was not confirmed. Ownership is returned to the caller.
    Unconfirmed {
        handle: AgentHandle,
        errors: Vec<String>,
    },
}

impl AgentHandle {
    /// Probe whether the child has exited without consuming this owned handle.
    pub fn is_finished(&mut self) -> std::io::Result<bool> {
        self.child.try_wait().map(|status| status.is_some())
    }

    /// Kill the agent and all descendants. Sends SIGTERM to the process group,
    /// waits for `grace`, then SIGKILL.
    pub async fn kill(self, grace: Duration) -> AgentTermination {
        self.kill_with_deadline(grace, None).await
    }

    /// Kill the agent without allowing process-tree settlement to outlive an
    /// outer automation deadline.  Timing out preserves this owned handle so
    /// the caller can durably retain the still-unconfirmed PID.
    pub async fn kill_until(
        self,
        grace: Duration,
        deadline: tokio::time::Instant,
    ) -> AgentTermination {
        self.kill_with_deadline(grace, Some(deadline)).await
    }

    async fn kill_with_deadline(
        mut self,
        grace: Duration,
        deadline: Option<tokio::time::Instant>,
    ) -> AgentTermination {
        let mut process_errors = Vec::new();

        let already_absent = matches!(self.child.try_wait(), Ok(Some(_)));
        let mut tree_cleanup_confirmed = already_absent;
        if !already_absent {
            // Use roko-agent's kill_tree which handles process groups properly.
            let result = if let Some(deadline) = deadline {
                match tokio::time::timeout_at(deadline, kill_tree(&mut self.child, grace)).await {
                    Ok(result) => result,
                    Err(_) => {
                        process_errors.push(
                            "process tree termination exceeded settlement deadline".to_string(),
                        );
                        Err(std::io::Error::new(
                            std::io::ErrorKind::TimedOut,
                            "process tree settlement deadline exceeded",
                        ))
                    }
                }
            } else {
                kill_tree(&mut self.child, grace).await
            };
            tree_cleanup_confirmed = result.is_ok();
            if let Err(e) = result {
                warn!(pid = self.pid, err = %e, "error killing agent");
                if process_errors.is_empty() {
                    process_errors.push(format!("process tree termination failed: {e}"));
                }
            }
        }
        let process_confirmed = if already_absent {
            true
        } else if !tree_cleanup_confirmed {
            false
        } else {
            match self.child.try_wait() {
                Ok(Some(_)) => true,
                Ok(None) => {
                    process_errors.push("process still running after kill_tree".to_string());
                    false
                }
                Err(e) => {
                    process_errors.push(format!("failed to confirm process exit: {e}"));
                    false
                }
            }
        };
        self.finish_kill(process_confirmed, process_errors).await
    }

    async fn finish_kill(
        mut self,
        process_confirmed: bool,
        process_errors: Vec<String>,
    ) -> AgentTermination {
        if !process_confirmed {
            return AgentTermination::Failed {
                handle: self,
                process_confirmed,
                process_errors,
                reader_errors: Vec::new(),
            };
        }

        roko_agent::process::unregister_pid(self.pid);
        let mut reader_errors = Vec::new();
        if let Some(reader_task) = &self.reader_task {
            reader_task.abort();
        }
        if let Some(stderr_reader_task) = &self.stderr_reader_task {
            stderr_reader_task.abort();
        }
        if let Some(reader_task) = self.reader_task.take() {
            collect_reader_result("stdout", reader_task.await, true, &mut reader_errors);
        }
        if let Some(stderr_reader_task) = self.stderr_reader_task.take() {
            collect_reader_result("stderr", stderr_reader_task.await, true, &mut reader_errors);
        }

        if process_errors.is_empty() && reader_errors.is_empty() {
            AgentTermination::Confirmed { pid: self.pid }
        } else {
            AgentTermination::Failed {
                handle: self,
                process_confirmed,
                process_errors,
                reader_errors,
            }
        }
    }

    /// Wait for the process to exit and return its exit code.
    pub async fn wait(mut self) -> AgentWait {
        let status = match self.child.wait().await {
            Ok(status) => status,
            Err(err) => {
                return AgentWait::Unconfirmed {
                    handle: self,
                    errors: vec![format!("failed to wait for agent process: {err}")],
                };
            }
        };
        let exit_code = status.code();
        // A successful wait proves process absence even if reader joining later
        // reports a separate supervision failure.
        roko_agent::process::unregister_pid(self.pid);
        let mut errors = Vec::new();
        if let Some(reader_task) = self.reader_task.take() {
            collect_reader_result("stdout", reader_task.await, false, &mut errors);
        }
        if let Some(stderr_reader_task) = self.stderr_reader_task.take() {
            collect_reader_result("stderr", stderr_reader_task.await, false, &mut errors);
        }
        AgentWait::Confirmed {
            pid: self.pid,
            exit_code,
            reader_errors: errors,
        }
    }
}

fn collect_reader_result(
    stream: &str,
    result: std::result::Result<(), tokio::task::JoinError>,
    allow_cancelled: bool,
    errors: &mut Vec<String>,
) {
    match result {
        Ok(()) => {}
        Err(err) if err.is_cancelled() && allow_cancelled => {}
        Err(err) => errors.push(format!("{stream} reader task failed: {err}")),
    }
}

/// Parse a single line of `--output-format stream-json` into `AgentEvent`(s).
///
/// Returns an empty vec for empty lines or unparseable content.
/// May return multiple events (e.g., a MessageDelta AND a TokenUsage from the
/// same assistant message).
pub fn parse_stream_line(line: &str) -> Vec<AgentEvent> {
    roko_agent::provider::claude_cli::stream::parse_stream_line(line)
}

/// Parse one line using the selected provider's stream protocol.
pub fn parse_provider_stream_line(protocol: CliProtocol, line: &str) -> Vec<AgentEvent> {
    match protocol {
        CliProtocol::ClaudeStreamJson => parse_stream_line(line),
        CliProtocol::CodexExecJson => {
            roko_agent::provider::codex_cli::stream::parse_stream_line(line)
        }
        CliProtocol::GeminiStreamJson => {
            roko_agent::provider::gemini_cli::stream::parse_stream_line(line)
        }
    }
}

/// Spawn a configured CLI agent process and stream its output through the channel.
pub async fn spawn_agent(
    config: &AgentSpawnConfig,
    event_tx: mpsc::Sender<AgentEvent>,
) -> Result<AgentHandle> {
    spawn_agent_controlled(config, event_tx, None)
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))
}

/// Spawn a CLI agent with an interruptible, process-tree-safe startup phase.
pub async fn spawn_agent_controlled(
    config: &AgentSpawnConfig,
    event_tx: mpsc::Sender<AgentEvent>,
    control: Option<&AgentStartupControl>,
) -> std::result::Result<AgentHandle, AgentStartupError> {
    let provider = config
        .cli_provider
        .clone()
        .unwrap_or_else(|| CliProviderConfig::from_legacy_runner_program(config.program.clone()));
    let max_turns = effective_agent_turn_limit(config.max_turns);
    let invocation = provider
        .build_invocation(&CliDispatchRequest {
            prompt: config.prompt.clone(),
            system_prompt: config.system_prompt.clone(),
            model: config.model.clone(),
            workdir: config.workdir.clone(),
            max_turns,
            effort: config.effort.clone(),
            dangerously_skip_permissions: config.dangerously_skip_permissions,
            mcp_config: config.mcp_config.clone(),
            resume_session: config.resume_session.clone(),
            env: config.extra_env.clone(),
            agent_id: config.agent_id.clone(),
            allowed_tools: config.allowed_tools.clone(),
            disallowed_tools: config.disallowed_tools.clone(),
            plugin_mcp: config.plugin_mcp.clone(),
        })
        .map_err(|error| AgentStartupError::Failed(error.into()))?;
    if invocation.turn_limit.effective_max_turns.is_some() {
        debug!(
            provider = %invocation.event_provider,
            requested_max_turns = invocation.turn_limit.requested_max_turns,
            "provider will enforce the native turn limit"
        );
    } else {
        warn!(
            provider = %invocation.event_provider,
            requested_max_turns = invocation.turn_limit.requested_max_turns,
            "provider has no native turn limit; runner wall-clock deadline is the binding bound"
        );
    }

    let mut ephemeral_config_dir = None;
    let ephemeral_config_env = if let Some(config) = &invocation.ephemeral_config {
        let directory = tempfile::Builder::new()
            .prefix("roko-cli-provider-")
            .tempdir()
            .context("creating temporary provider config directory")
            .map_err(AgentStartupError::Failed)?;
        let path = directory.path().join(&config.file_name);
        await_startup_step(control, tokio::fs::write(&path, config.contents.as_bytes()))
            .await
            .map_err(|interruption| AgentStartupError::Interrupted {
                interruption,
                cleanup_error: None,
                unconfirmed: None,
            })?
            .with_context(|| format!("writing temporary provider config {}", path.display()))
            .map_err(AgentStartupError::Failed)?;
        let env = Some((config.env_key.clone(), path));
        ephemeral_config_dir = Some(directory);
        env
    } else {
        None
    };

    let mut cmd = confined_command(&invocation.program, invocation.resource_limits.as_ref())
        .context("configuring provider process confinement")
        .map_err(AgentStartupError::Failed)?;
    cmd.current_dir(&invocation.workdir);
    cmd.args(&invocation.args);
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    cmd.kill_on_drop(true);

    // Environment.
    for (key, value) in &invocation.env {
        cmd.env(key, value);
    }
    for (key, value) in invocation.secret_env.iter() {
        cmd.env(key, value);
    }
    if let Some((key, path)) = &ephemeral_config_env {
        cmd.env(key, path);
    }
    // Unset all Claude Code env vars to prevent "nested session" detection
    // when spawning agents from within a Claude Code session.
    cmd.env_remove("CLAUDECODE");
    cmd.env_remove("CLAUDE_CODE_ENTRYPOINT");
    cmd.env_remove("CLAUDE_CODE_SSE_PORT");
    cmd.env_remove("CLAUDE_CODE_MAX_OUTPUT_TOKENS");
    cmd.env_remove("CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS");
    cmd.env_remove("CLAUDE_CODE_EFFORT_LEVEL");

    // Process group isolation.
    set_process_group(&mut cmd);
    let mut child = cmd
        .spawn()
        .with_context(|| format!("spawning {} CLI", invocation.event_provider))
        .map_err(AgentStartupError::Failed)?;
    let pid = child
        .id()
        .context("agent process exited before PID could be read")
        .map_err(AgentStartupError::Failed)?;

    // Register PID for orphan cleanup immediately, before spawning reader
    // tasks. If a panic occurs between here and the end of this function,
    // the cleanup handler will still find the PID.
    roko_agent::process::register_spawned_pid(pid);

    let started_send = event_tx.send(AgentEvent::Started {
        agent_id: config.agent_id.clone(),
        provider: invocation.event_provider.clone(),
        model: invocation.model.clone(),
        pid: Some(pid),
    });
    if let Err(interruption) = await_startup_step(control, started_send).await {
        let cleanup_error = match control {
            Some(control) => interrupt_startup_child(&mut child, pid, control).await,
            None => None,
        };
        return Err(AgentStartupError::Interrupted {
            interruption,
            unconfirmed: cleanup_error.as_ref().map(|_| AgentHandle {
                pid,
                child,
                reader_task: None,
                stderr_reader_task: None,
                _ephemeral_config_dir: ephemeral_config_dir,
            }),
            cleanup_error,
        });
    }

    // Write prompt to stdin synchronously, then close it (matching mori's pattern).
    // Must complete BEFORE spawning reader tasks to avoid race conditions.
    if let Some(mut stdin) = child.stdin.take() {
        match await_startup_step(control, stdin.write_all(invocation.stdin.as_bytes())).await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => error!(err = %e, "writing prompt to agent stdin"),
            Err(interruption) => {
                drop(stdin);
                let cleanup_error = match control {
                    Some(control) => interrupt_startup_child(&mut child, pid, control).await,
                    None => None,
                };
                return Err(AgentStartupError::Interrupted {
                    interruption,
                    unconfirmed: cleanup_error.as_ref().map(|_| AgentHandle {
                        pid,
                        child,
                        reader_task: None,
                        stderr_reader_task: None,
                        _ephemeral_config_dir: ephemeral_config_dir,
                    }),
                    cleanup_error,
                });
            }
        }
        drop(stdin); // EOF signals end of input to Claude CLI
    }

    // Spawn reader task for stdout.
    let stdout = match child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            let fallback = AgentStartupControl {
                deadline: tokio::time::Instant::now() + Duration::from_secs(3),
                cancel: CancellationToken::new(),
                cleanup_grace: Duration::from_secs(1),
            };
            let cleanup_error =
                interrupt_startup_child(&mut child, pid, control.unwrap_or(&fallback)).await;
            if let Some(cleanup_error) = cleanup_error {
                return Err(AgentStartupError::Interrupted {
                    // No provider protocol was established.  The meaningful
                    // fact here is that cleanup could not prove process-tree
                    // death, so ownership must retain the child for a later
                    // cancellation retry.  The caller does not terminalize an
                    // unconfirmed interruption based on this discriminator.
                    interruption: AgentStartupInterruption::Cancelled,
                    cleanup_error: Some(format!("agent stdout not captured; {cleanup_error}")),
                    unconfirmed: Some(AgentHandle {
                        pid,
                        child,
                        reader_task: None,
                        stderr_reader_task: None,
                        _ephemeral_config_dir: ephemeral_config_dir,
                    }),
                });
            }
            return Err(AgentStartupError::Failed(anyhow::anyhow!(
                "agent stdout not captured"
            )));
        }
    };

    let agent_id = config.agent_id.clone();
    let stdout_tx = event_tx.clone();
    let protocol = invocation.protocol;
    let scrubber = Arc::new(LogScrubber::new());
    let stdout_scrubber = Arc::clone(&scrubber);
    let reader_task = tokio::spawn(async move {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            let line = stdout_scrubber.scrub(&line);
            for event in parse_provider_stream_line(protocol, &line) {
                if stdout_tx.send(event).await.is_err() {
                    debug!(agent_id = %agent_id, "event channel closed, stopping reader");
                    return;
                }
            }
        }

        // Send Exited — we don't know the exit code yet, the event loop
        // will reap it from the child handle.
        let _ = stdout_tx.send(AgentEvent::Exited { exit_code: None }).await;
    });

    // Spawn stderr reader and surface it as durable agent events.
    let stderr_reader_task = if let Some(stderr) = child.stderr.take() {
        let stderr_tx = event_tx.clone();
        let stderr_scrubber = scrubber;
        Some(tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if !line.trim().is_empty() {
                    let line = stderr_scrubber.scrub(&line);
                    debug!(stderr = %line, "agent stderr");
                    let _ = stderr_tx.send(AgentEvent::Error { message: line }).await;
                }
            }
        }))
    } else {
        None
    };

    Ok(AgentHandle {
        pid,
        child,
        reader_task: Some(reader_task),
        stderr_reader_task,
        _ephemeral_config_dir: ephemeral_config_dir,
    })
}

fn effective_agent_turn_limit(configured: u32) -> u32 {
    let configured = configured.max(1);
    if !env_flag_enabled("ROKO_FAST_MODE") {
        return configured;
    }
    let fast_limit = std::env::var("ROKO_FAST_MAX_AGENT_TURNS")
        .ok()
        .and_then(|value| value.trim().parse::<u32>().ok())
        .filter(|limit| *limit > 0)
        .unwrap_or(FAST_AGENT_TURN_LIMIT);
    configured.min(fast_limit)
}

fn env_flag_enabled(name: &str) -> bool {
    std::env::var(name).is_ok_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    fn scripted_agent(script: &str) -> (tempfile::TempDir, AgentSpawnConfig) {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir().expect("create agent test directory");
        let program = temp.path().join("test-agent");
        std::fs::write(&program, script).expect("write agent test script");
        let mut permissions = std::fs::metadata(&program)
            .expect("read agent test script metadata")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&program, permissions).expect("make agent test script executable");

        let config = AgentSpawnConfig {
            prompt: "test prompt".to_string(),
            system_prompt: String::new(),
            model: "test-model".to_string(),
            workdir: temp.path().to_path_buf(),
            max_turns: 1,
            effort: None,
            program: program.clone(),
            dangerously_skip_permissions: false,
            mcp_config: None,
            resume_session: None,
            agent_id: "test-agent".to_string(),
            cli_provider: Some(CliProviderConfig::claude("test-cli", program)),
            disallowed_tools: Vec::new(),
            allowed_tools: None,
            plugin_mcp: None,
            extra_env: Vec::new(),
        };
        (temp, config)
    }

    #[cfg(unix)]
    fn test_agent_handle(reader_task: JoinHandle<()>) -> AgentHandle {
        let mut command = Command::new("sh");
        command
            .arg("-c")
            .arg("sleep 30")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        set_process_group(&mut command);
        let child = command.spawn().expect("spawn test child");
        let pid = child.id().expect("test child pid");
        AgentHandle {
            pid,
            child,
            reader_task: Some(reader_task),
            stderr_reader_task: Some(tokio::spawn(std::future::pending())),
            _ephemeral_config_dir: None,
        }
    }

    #[cfg(unix)]
    fn completed_agent_handle(
        reader_task: JoinHandle<()>,
        stderr_reader_task: JoinHandle<()>,
    ) -> AgentHandle {
        let mut command = Command::new("sh");
        command
            .arg("-c")
            .arg("exit 7")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        set_process_group(&mut command);
        let child = command.spawn().expect("spawn completed test child");
        let pid = child.id().expect("test child pid");
        roko_agent::process::register_spawned_pid(pid);
        AgentHandle {
            pid,
            child,
            reader_task: Some(reader_task),
            stderr_reader_task: Some(stderr_reader_task),
            _ephemeral_config_dir: None,
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn kill_confirms_process_and_cancelled_readers() {
        let handle = test_agent_handle(tokio::spawn(std::future::pending()));
        let pid = handle.pid;

        assert!(matches!(
            handle.kill(Duration::from_millis(10)).await,
            AgentTermination::Confirmed { pid: confirmed } if confirmed == pid
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn kill_reports_panicked_reader_as_failure() {
        let reader_task = tokio::spawn(async { panic!("reader failed") });
        tokio::task::yield_now().await;
        let handle = test_agent_handle(reader_task);
        let pid = handle.pid;
        roko_agent::process::register_spawned_pid(pid);

        let AgentTermination::Failed {
            handle,
            process_confirmed,
            process_errors,
            reader_errors,
        } = handle.kill(Duration::from_millis(10)).await
        else {
            panic!("expected failed termination");
        };
        assert_eq!(handle.pid, pid);
        assert!(process_confirmed);
        assert!(process_errors.is_empty());
        assert!(!roko_agent::process::registered_pids().contains(&pid));
        assert!(
            reader_errors
                .iter()
                .any(|error| error.contains("stdout reader task failed"))
        );
        assert!(matches!(
            handle.kill(Duration::from_millis(10)).await,
            AgentTermination::Confirmed { pid: confirmed } if confirmed == pid
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn unconfirmed_process_retains_child_readers_and_registration_for_retry() {
        let handle = test_agent_handle(tokio::spawn(std::future::pending()));
        let pid = handle.pid;
        roko_agent::process::register_spawned_pid(pid);

        let AgentTermination::Failed {
            handle,
            process_confirmed,
            process_errors,
            reader_errors,
        } = handle
            .finish_kill(false, vec!["forced process error".to_string()])
            .await
        else {
            panic!("expected retryable termination failure");
        };
        assert!(!process_confirmed);
        assert_eq!(process_errors, vec!["forced process error"]);
        assert!(reader_errors.is_empty());
        assert!(handle.reader_task.is_some());
        assert!(handle.stderr_reader_task.is_some());
        assert!(roko_agent::process::registered_pids().contains(&pid));

        assert!(matches!(
            handle.kill(Duration::from_millis(10)).await,
            AgentTermination::Confirmed { pid: confirmed } if confirmed == pid
        ));
        assert!(!roko_agent::process::registered_pids().contains(&pid));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn wait_confirms_child_and_both_readers() {
        let handle = completed_agent_handle(tokio::spawn(async {}), tokio::spawn(async {}));
        let pid = handle.pid;
        assert!(matches!(
            handle.wait().await,
            AgentWait::Confirmed {
                pid: confirmed_pid,
                exit_code: Some(7),
                reader_errors,
            } if confirmed_pid == pid && reader_errors.is_empty()
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn wait_reports_reader_panic_after_confirmed_process_exit() {
        let reader = tokio::spawn(async { panic!("reader failed") });
        tokio::task::yield_now().await;
        let handle = completed_agent_handle(reader, tokio::spawn(async {}));
        let pid = handle.pid;
        let AgentWait::Confirmed {
            pid: confirmed_pid,
            reader_errors,
            ..
        } = handle.wait().await
        else {
            panic!("process absence must be distinguished from reader failure");
        };
        assert_eq!(confirmed_pid, pid);
        assert!(
            reader_errors
                .iter()
                .any(|error| error.contains("stdout reader task failed"))
        );
        assert!(!roko_agent::process::registered_pids().contains(&pid));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn wait_reports_stderr_reader_panic_after_confirmed_process_exit() {
        let stderr_reader = tokio::spawn(async { panic!("stderr reader failed") });
        tokio::task::yield_now().await;
        let handle = completed_agent_handle(tokio::spawn(async {}), stderr_reader);
        let AgentWait::Confirmed { reader_errors, .. } = handle.wait().await else {
            panic!("process absence must be distinguished from stderr reader failure");
        };
        assert!(
            reader_errors
                .iter()
                .any(|error| error.contains("stderr reader task failed"))
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn wait_reports_unexpected_reader_cancellation() {
        let reader = tokio::spawn(std::future::pending());
        reader.abort();
        let handle = completed_agent_handle(reader, tokio::spawn(async {}));
        let AgentWait::Confirmed { reader_errors, .. } = handle.wait().await else {
            panic!("child process should be confirmed absent");
        };
        assert!(
            reader_errors
                .iter()
                .any(|error| error.contains("stdout reader task failed"))
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn spawned_pid_is_registered_before_started_delivery_completes() {
        let (temp, config) = scripted_agent(
            "#!/bin/sh\nprintf '%s\\n' \"$$\" > \"$(dirname \"$0\")/pid\"\nsleep 30\n",
        );
        let pid_file = temp.path().join("pid");
        let (event_tx, mut event_rx) = mpsc::channel(1);
        event_tx
            .send(AgentEvent::Exited { exit_code: None })
            .await
            .expect("prefill event channel");

        let spawn_task = tokio::spawn(async move { spawn_agent(&config, event_tx).await });
        let observed_pid = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let Ok(contents) = std::fs::read_to_string(&pid_file)
                    && let Ok(pid) = contents.trim().parse::<u32>()
                    && roko_agent::process::registered_pids().contains(&pid)
                {
                    break pid;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .ok();

        assert!(matches!(
            event_rx.recv().await,
            Some(AgentEvent::Exited { exit_code: None })
        ));
        let handle = spawn_task
            .await
            .expect("spawn task must not panic")
            .expect("spawn test agent");
        let pid = handle.pid;
        let cleanup_confirmed = matches!(
            handle.kill(Duration::from_millis(10)).await,
            AgentTermination::Confirmed { pid: confirmed } if confirmed == pid
        );
        assert!(cleanup_confirmed, "test agent cleanup was not confirmed");
        assert_eq!(
            observed_pid,
            Some(pid),
            "spawned PID was not registered while Started delivery was blocked"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn closed_event_channel_terminates_stdout_reader_before_child_exit() {
        let (_temp, config) = scripted_agent(
            "#!/bin/sh\nprintf '%s\\n' '{\"type\":\"assistant\",\"subtype\":\"message\",\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"hello\"}],\"usage\":null}}'\nsleep 30\n",
        );
        let (event_tx, event_rx) = mpsc::channel(1);
        drop(event_rx);
        let mut handle = spawn_agent(&config, event_tx)
            .await
            .expect("spawn test agent");
        let pid = handle.pid;
        let mut reader_task = handle.reader_task.take().expect("stdout reader task");

        let reader_finished = tokio::time::timeout(Duration::from_secs(5), &mut reader_task)
            .await
            .is_ok();
        if !reader_finished {
            reader_task.abort();
            let _ = reader_task.await;
        }
        assert!(matches!(
            handle.kill(Duration::from_millis(10)).await,
            AgentTermination::Confirmed { pid: confirmed } if confirmed == pid
        ));
        assert!(
            reader_finished,
            "stdout reader remained attached to the live child after its event channel closed"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn gemini_spawn_materializes_auth_config_parses_events_and_cleans_up() {
        let (temp, mut config) = scripted_agent(
            r#"#!/bin/sh
set -eu
test -f "${GEMINI_CLI_SYSTEM_SETTINGS_PATH:?}"
test "${ROKO_PLUGIN_MCP_TOKEN:?}" = "signed-secret"
printf '%s' "$GEMINI_CLI_SYSTEM_SETTINGS_PATH" > "$PWD/settings-path"
cp "$GEMINI_CLI_SYSTEM_SETTINGS_PATH" "$PWD/settings-snapshot.json"
cat >/dev/null
printf '%s\n' '{"type":"init","session_id":"gemini-session","model":"gemini-2.5-pro"}'
printf '%s\n' '{"type":"message","role":"assistant","content":"calling plugin","delta":true}'
printf '%s\n' '{"type":"tool_use","tool_name":"demo.echo","tool_id":"call-1","parameters":{"text":"hi"}}'
printf '%s\n' '{"type":"tool_result","tool_id":"call-1","status":"success","output":"hi"}'
printf '%s\n' '{"type":"result","status":"success","stats":{"input_tokens":9,"output_tokens":4,"cached":2}}'
"#,
        );
        config.model = "gemini-2.5-pro".to_string();
        config.cli_provider = Some(CliProviderConfig::gemini(
            "gemini-cli",
            config.program.clone(),
        ));
        config.allowed_tools = Some(vec!["demo.echo".to_string()]);
        config.plugin_mcp = Some(CliPluginMcpConfig {
            server_name: "roko_plugins".to_string(),
            url: "http://127.0.0.1:43123/mcp".to_string(),
            bearer_token: "signed-secret".to_string(),
            tool_names: vec!["demo.echo".to_string()],
        });

        let (event_tx, mut event_rx) = mpsc::channel(32);
        let handle = spawn_agent(&config, event_tx)
            .await
            .expect("spawn fake Gemini CLI");
        assert!(matches!(
            handle.wait().await,
            AgentWait::Confirmed {
                exit_code: Some(0),
                ref reader_errors,
                ..
            } if reader_errors.is_empty()
        ));

        let events = std::iter::from_fn(|| event_rx.try_recv().ok()).collect::<Vec<_>>();
        assert!(events.iter().any(|event| matches!(
            event,
            AgentEvent::SystemInit { session_id, .. } if session_id == "gemini-session"
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            AgentEvent::ToolCall { id, name } if id == "call-1" && name == "demo.echo"
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            AgentEvent::ToolOutput { id, output } if id == "call-1" && output == "hi"
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            AgentEvent::TokenUsage {
                input_tokens: 9,
                output_tokens: 4,
                cache_read_tokens: 2,
                ..
            }
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            AgentEvent::TurnCompleted {
                is_error: false,
                ..
            }
        )));

        let settings_path =
            std::fs::read_to_string(temp.path().join("settings-path")).expect("settings path");
        assert!(
            !PathBuf::from(settings_path).exists(),
            "ephemeral Gemini settings must be deleted after process exit"
        );
        let settings = std::fs::read_to_string(temp.path().join("settings-snapshot.json")).unwrap();
        assert!(settings.contains("${ROKO_PLUGIN_MCP_TOKEN}"));
        assert!(!settings.contains("signed-secret"));
    }

    #[test]
    fn parse_system_event() {
        let line = r#"{"type":"system","subtype":"init","session_id":"abc-123","model":"claude-sonnet-4-6","tools":[]}"#;
        let event = parse_stream_line(line).into_iter().next().unwrap();
        match event {
            AgentEvent::SystemInit { session_id, model } => {
                assert_eq!(session_id, "abc-123");
                assert_eq!(model, "claude-sonnet-4-6");
            }
            _ => panic!("expected SystemInit"),
        }
    }

    #[test]
    fn parse_assistant_text() {
        let line = r#"{"type":"assistant","subtype":"message","message":{"content":[{"type":"text","text":"hello world"}],"usage":null}}"#;
        let event = parse_stream_line(line).into_iter().next().unwrap();
        match event {
            AgentEvent::MessageDelta { text } => {
                assert_eq!(text, "hello world");
            }
            _ => panic!("expected MessageDelta"),
        }
    }

    #[test]
    fn parse_assistant_tool_use() {
        let line = r#"{"type":"assistant","subtype":"message","message":{"content":[{"type":"tool_use","id":"tu_1","name":"Read","input":{"path":"foo"}}],"usage":null}}"#;
        let event = parse_stream_line(line).into_iter().next().unwrap();
        match event {
            AgentEvent::ToolCall { id, name } => {
                assert_eq!(id, "tu_1");
                assert_eq!(name, "Read");
            }
            _ => panic!("expected ToolCall"),
        }
    }

    #[test]
    fn parse_tool_event() {
        let line = r#"{"type":"tool","subtype":"result","tool_name":"Bash","tool_use_id":"tu_2","content":"output here"}"#;
        let event = parse_stream_line(line).into_iter().next().unwrap();
        match event {
            AgentEvent::ToolOutput { id, output } => {
                assert_eq!(id, "tu_2");
                assert_eq!(output, "output here");
            }
            _ => panic!("expected ToolOutput"),
        }
    }

    #[test]
    fn parse_result_event() {
        let line = r#"{"type":"result","session_id":"sess-1","total_cost_usd":0.05,"num_turns":3,"is_error":false}"#;
        let event = parse_stream_line(line).into_iter().next().unwrap();
        match event {
            AgentEvent::TurnCompleted {
                session_id,
                total_cost_usd,
                num_turns,
                is_error,
            } => {
                assert_eq!(session_id.unwrap(), "sess-1");
                assert!((total_cost_usd.unwrap() - 0.05).abs() < f64::EPSILON);
                assert_eq!(num_turns.unwrap(), 3);
                assert!(!is_error);
            }
            _ => panic!("expected TurnCompleted"),
        }
    }

    #[test]
    fn parse_empty_line() {
        assert!(parse_stream_line("").is_empty());
        assert!(parse_stream_line("   ").is_empty());
    }

    #[test]
    fn parse_malformed_json() {
        assert!(parse_stream_line("{not json}").is_empty());
    }

    #[test]
    fn tool_output_truncation() {
        let long_content = "x".repeat(5000);
        let line = format!(
            r#"{{"type":"tool","subtype":"result","tool_name":"Bash","tool_use_id":"tu_3","content":"{long_content}"}}"#
        );
        let event = parse_stream_line(&line).into_iter().next().unwrap();
        match event {
            AgentEvent::ToolOutput { output, .. } => {
                assert!(output.len() < 5000);
                assert!(output.ends_with("… [truncated]"));
            }
            _ => panic!("expected ToolOutput"),
        }
    }
}
