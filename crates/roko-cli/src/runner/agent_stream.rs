//! Agent process spawning and stream-JSON parsing.
//!
//! Spawns the configured CLI provider, parses stdout lines into
//! [`AgentEvent`]s, and sends them through a tokio mpsc channel.

use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, error, warn};

use roko_agent::process::{kill_tree, set_process_group};
use roko_core::defaults::DEFAULT_AGENT_TURN_LIMIT;

use crate::dispatch_v2::{CliDispatchProvider, CliDispatchRequest, CliProviderConfig};

use super::types::{AgentEvent, RunConfig};

/// Configuration for spawning a single agent.
#[derive(Debug, Clone)]
pub struct AgentSpawnConfig {
    /// The prompt to send to the agent.
    pub prompt: String,
    /// System prompt appended via --append-system-prompt.
    pub system_prompt: String,
    /// Model to use.
    pub model: String,
    /// Working directory for the agent.
    pub workdir: PathBuf,
    /// Maximum turns the agent can take.
    pub max_turns: u32,
    /// Optional reasoning effort hint for providers that support it.
    pub effort: Option<String>,
    /// Claude CLI binary path.
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
    pub async fn kill(mut self, grace: Duration) -> AgentTermination {
        let mut process_errors = Vec::new();

        let already_absent = matches!(self.child.try_wait(), Ok(Some(_)));
        if !already_absent {
            // Use roko-agent's kill_tree which handles process groups properly.
            if let Err(e) = kill_tree(&mut self.child, grace).await {
                warn!(pid = self.pid, err = %e, "error killing agent");
                process_errors.push(format!("process tree termination failed: {e}"));
            }
        }
        let process_confirmed = if already_absent {
            true
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

/// Spawn a configured CLI agent process and stream its output through the channel.
pub async fn spawn_agent(
    config: &AgentSpawnConfig,
    event_tx: mpsc::Sender<AgentEvent>,
) -> Result<AgentHandle> {
    let provider = config
        .cli_provider
        .clone()
        .unwrap_or_else(|| CliProviderConfig::from_legacy_runner_program(config.program.clone()));
    let invocation = provider.build_invocation(&CliDispatchRequest {
        prompt: config.prompt.clone(),
        system_prompt: config.system_prompt.clone(),
        model: config.model.clone(),
        workdir: config.workdir.clone(),
        max_turns: config.max_turns,
        effort: config.effort.clone(),
        dangerously_skip_permissions: config.dangerously_skip_permissions,
        mcp_config: config.mcp_config.clone(),
        resume_session: config.resume_session.clone(),
        env: Vec::new(),
        agent_id: config.agent_id.clone(),
    })?;

    let mut cmd = Command::new(&invocation.program);
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
        .with_context(|| format!("spawning {} CLI", invocation.event_provider))?;
    let pid = child
        .id()
        .context("agent process exited before PID could be read")?;

    // Register PID for orphan cleanup immediately, before spawning reader
    // tasks. If a panic occurs between here and the end of this function,
    // the cleanup handler will still find the PID.
    roko_agent::process::register_spawned_pid(pid);

    let _ = event_tx
        .send(AgentEvent::Started {
            agent_id: config.agent_id.clone(),
            provider: invocation.event_provider.clone(),
            model: invocation.model.clone(),
            pid: Some(pid),
        })
        .await;

    // Write prompt to stdin synchronously, then close it (matching mori's pattern).
    // Must complete BEFORE spawning reader tasks to avoid race conditions.
    if let Some(mut stdin) = child.stdin.take() {
        if let Err(e) = stdin.write_all(invocation.stdin.as_bytes()).await {
            error!(err = %e, "writing prompt to agent stdin");
        }
        drop(stdin); // EOF signals end of input to Claude CLI
    }

    // Spawn reader task for stdout.
    let stdout = child.stdout.take().context("agent stdout not captured")?;

    let agent_id = config.agent_id.clone();
    let stdout_tx = event_tx.clone();
    let reader_task = tokio::spawn(async move {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            for event in parse_stream_line(&line) {
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
        Some(tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if !line.trim().is_empty() {
                    debug!(stderr = %line, "agent stderr");
                    let _ = stderr_tx
                        .send(AgentEvent::Error {
                            message: line.to_string(),
                        })
                        .await;
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
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
