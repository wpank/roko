//! Shared subprocess lifecycle manager for one-shot and persistent agent harnesses.
//!
//! [`ChildProcessRunner`] encapsulates the common plumbing that every
//! CLI-based agent needs: spawn, pipe stdin, stream stdout/stderr
//! through an [`EventParser`], enforce timeouts, register/unregister
//! PIDs, and clean up via [`kill_tree`](crate::process::kill::kill_tree).

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::process::env::{AgentEnv, apply_agent_env};
use crate::process::group::set_process_group;
use crate::process::kill::kill_tree;
use crate::process::registry::{register_spawned_pid, unregister_pid};

use super::error::HarnessError;
use super::events::{EventParser, HarnessEvent};

/// Environment variable overrides for a child process.
#[derive(Clone, Debug, Default)]
pub struct ScrubbedEnv {
    /// Key-value pairs to inject into the child's environment.
    pub inject: Vec<(String, String)>,
    /// Keys to remove from the child's environment.
    pub remove: Vec<String>,
}

impl ScrubbedEnv {
    /// Apply the env overrides to a `Command`.
    pub fn apply(&self, cmd: &mut Command) {
        let agent_env = AgentEnv {
            vars: self.inject.iter().cloned().collect(),
            remove: self.remove.clone(),
            working_dir: None,
        };
        apply_agent_env(cmd, &agent_env);
    }
}

/// A handle to a spawned persistent child process.
///
/// Returned by [`ChildProcessRunner::spawn_persistent`]. The caller
/// owns the child and is responsible for driving its I/O.
pub struct SpawnedChild {
    /// The tokio child process.
    pub child: tokio::process::Child,
    /// PID of the spawned process.
    pub pid: Option<u32>,
}

impl SpawnedChild {
    /// Kill the child process and unregister its PID.
    pub async fn kill(&mut self) {
        if let Some(pid) = self.pid {
            let _ = kill_tree(&mut self.child, std::time::Duration::from_millis(1200)).await;
            if track_pids() {
                unregister_pid(pid);
            }
        }
    }
}

/// Whether to track PIDs in the global registry.
///
/// Disabled during `#[cfg(test)]` to avoid poisoning the global
/// registry from concurrent test runs.
const fn track_pids() -> bool {
    !cfg!(test)
}

/// Shared subprocess lifecycle manager.
///
/// Encapsulates spawn, stdin piping, stdout/stderr streaming through
/// an [`EventParser`], timeout enforcement, PID registration, and
/// cleanup. Each agent adapter constructs a `ChildProcessRunner` with
/// the harness binary path and working directory, then calls
/// [`run_one_shot()`] or [`spawn_persistent()`].
pub struct ChildProcessRunner {
    /// Path to the harness binary.
    program: PathBuf,
    /// Working directory for the child process.
    current_dir: PathBuf,
    /// Timeout for one-shot runs.
    timeout: Duration,
    /// Environment overrides.
    env: ScrubbedEnv,
    /// Agent name (for log messages).
    name: String,
    /// Whether to print heartbeat messages during long runs.
    heartbeat_enabled: bool,
}

impl ChildProcessRunner {
    /// Create a new runner for the given program and working directory.
    pub fn new(program: impl AsRef<OsStr>, current_dir: impl AsRef<Path>) -> Self {
        Self {
            program: PathBuf::from(program.as_ref()),
            current_dir: current_dir.as_ref().to_path_buf(),
            timeout: Duration::from_secs(600),
            env: ScrubbedEnv::default(),
            name: String::new(),
            heartbeat_enabled: true,
        }
    }

    /// Set the timeout for one-shot runs.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set environment overrides.
    pub fn with_env(mut self, env: ScrubbedEnv) -> Self {
        self.env = env;
        self
    }

    /// Set the agent name (used in log messages).
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Enable or disable heartbeat messages during long runs.
    pub fn with_heartbeat(mut self, enabled: bool) -> Self {
        self.heartbeat_enabled = enabled;
        self
    }

    /// Run a one-shot command, stream its output through a parser, and
    /// return the collected events.
    ///
    /// The process is spawned, optional stdin is piped, stdout/stderr
    /// are streamed line-by-line through the parser, and the process is
    /// awaited. On timeout, the process tree is killed.
    ///
    /// # Arguments
    ///
    /// * `args` - Command-line arguments.
    /// * `stdin_data` - Optional data to write to the child's stdin.
    /// * `parser` - Protocol-specific line parser.
    /// * `cancel` - Optional cancellation token (mpsc receiver).
    pub async fn run_one_shot(
        &self,
        args: &[&str],
        stdin_data: Option<&[u8]>,
        parser: &mut dyn EventParser,
        cancel: Option<mpsc::Receiver<()>>,
    ) -> Result<Vec<HarnessEvent>, HarnessError> {
        let started = Instant::now();

        let mut cmd = Command::new(&self.program);
        cmd.args(args)
            .current_dir(&self.current_dir)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        set_process_group(&mut cmd);
        self.env.apply(&mut cmd);

        let mut child = cmd.spawn().map_err(HarnessError::Io)?;

        let pid = child.id();
        if let Some(pid) = pid {
            if track_pids() {
                register_spawned_pid(pid);
            }
        }

        // Write stdin if provided.
        if let Some(data) = stdin_data {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(data).await;
                let _ = stdin.shutdown().await;
            }
        } else {
            // Close stdin immediately so the child doesn't block waiting.
            drop(child.stdin.take());
        }

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let mut events = Vec::new();
        let mut stderr_lines = Vec::new();

        // Stream stdout and stderr concurrently with timeout.
        let stream_result = tokio::time::timeout(self.timeout, async {
            let mut handles = Vec::new();

            // Stdout reader.
            if let Some(stdout) = stdout {
                let (tx, mut rx) = mpsc::channel::<String>(256);
                handles.push(tokio::spawn(async move {
                    let reader = BufReader::new(stdout);
                    let mut lines = reader.lines();
                    while let Ok(Some(line)) = lines.next_line().await {
                        if tx.send(line).await.is_err() {
                            break;
                        }
                    }
                }));

                // Stderr reader.
                let (tx_err, mut rx_err) = mpsc::channel::<String>(256);
                if let Some(stderr) = stderr {
                    handles.push(tokio::spawn(async move {
                        let reader = BufReader::new(stderr);
                        let mut lines = reader.lines();
                        while let Ok(Some(line)) = lines.next_line().await {
                            if tx_err.send(line).await.is_err() {
                                break;
                            }
                        }
                    }));
                }

                // Collect events from both streams.
                loop {
                    tokio::select! {
                        Some(line) = rx.recv() => {
                            events.extend(parser.parse_stdout_line(&line));
                        }
                        Some(line) = rx_err.recv() => {
                            stderr_lines.push(line.clone());
                            events.extend(parser.parse_stderr_line(&line));
                        }
                        else => break,
                    }
                }
            }

            // Wait for the child to exit.
            child.wait().await
        })
        .await;

        // Clean up PID registration.
        if let Some(pid) = pid {
            if track_pids() {
                unregister_pid(pid);
            }
        }

        // Finalize the parser.
        events.extend(parser.finalize());

        match stream_result {
            Ok(Ok(status)) => {
                if status.success() {
                    Ok(events)
                } else {
                    Err(HarnessError::ProcessExit {
                        code: status.code(),
                        stderr: stderr_lines.join("\n"),
                    })
                }
            }
            Ok(Err(io_err)) => Err(HarnessError::Io(io_err)),
            Err(_timeout) => {
                // The child was moved into the timeout future.
                // `kill_on_drop(true)` ensures the process tree is cleaned up
                // when the future is dropped on timeout.
                Err(HarnessError::Timeout {
                    elapsed: started.elapsed(),
                    configured: self.timeout,
                })
            }
        }
    }

    /// Spawn a persistent child process and return a handle.
    ///
    /// Unlike [`run_one_shot`], this does NOT wait for the process to
    /// exit. The caller is responsible for driving I/O on the returned
    /// child's stdin/stdout/stderr.
    pub fn spawn_persistent(&self, args: &[&str]) -> Result<SpawnedChild, HarnessError> {
        let mut cmd = Command::new(&self.program);
        cmd.args(args)
            .current_dir(&self.current_dir)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        set_process_group(&mut cmd);
        self.env.apply(&mut cmd);

        let child = cmd.spawn().map_err(HarnessError::Io)?;

        let pid = child.id();
        if let Some(pid) = pid {
            if track_pids() {
                register_spawned_pid(pid);
            }
        }

        Ok(SpawnedChild { child, pid })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    /// Minimal parser that collects stdout lines as Output events.
    struct EchoParser;
    impl EventParser for EchoParser {
        fn parse_stdout_line(&mut self, line: &str) -> Vec<HarnessEvent> {
            vec![HarnessEvent::Output(line.to_string())]
        }
    }

    /// Parser that also collects stderr lines.
    struct EchoAllParser;
    impl EventParser for EchoAllParser {
        fn parse_stdout_line(&mut self, line: &str) -> Vec<HarnessEvent> {
            vec![HarnessEvent::Output(format!("stdout:{line}"))]
        }
        fn parse_stderr_line(&mut self, line: &str) -> Vec<HarnessEvent> {
            vec![HarnessEvent::Error(format!("stderr:{line}"))]
        }
    }

    fn write_script(path: &std::path::Path, body: &str) {
        fs::write(path, body).expect("write script");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(path).expect("script metadata").permissions();
            perms.set_mode(0o755);
            fs::set_permissions(path, perms).expect("chmod script");
        }
    }

    #[tokio::test]
    async fn run_one_shot_captures_stdout() {
        let dir = tempdir().unwrap();
        let script = dir.path().join("echo.sh");
        write_script(
            &script,
            "#!/bin/sh\necho 'hello from child'\necho 'second line'\n",
        );

        let runner = ChildProcessRunner::new(script.as_os_str(), dir.path()).with_heartbeat(false);
        let mut parser = EchoParser;
        let events = runner
            .run_one_shot(&[], None, &mut parser, None)
            .await
            .expect("run_one_shot should succeed");

        let texts: Vec<&str> = events
            .iter()
            .filter_map(|e| match e {
                HarnessEvent::Output(t) => Some(t.as_str()),
                _ => None,
            })
            .collect();
        assert!(
            texts.iter().any(|t| t.contains("hello from child")),
            "expected 'hello from child' in output, got: {texts:?}"
        );
        assert!(
            texts.iter().any(|t| t.contains("second line")),
            "expected 'second line' in output, got: {texts:?}"
        );
    }

    #[tokio::test]
    async fn run_one_shot_timeout_kills_process() {
        let dir = tempdir().unwrap();
        let script = dir.path().join("slow.sh");
        write_script(&script, "#!/bin/sh\nsleep 30\n");

        let runner = ChildProcessRunner::new(script.as_os_str(), dir.path())
            .with_timeout(Duration::from_millis(500))
            .with_heartbeat(false);
        let mut parser = EchoParser;
        let result = runner.run_one_shot(&[], None, &mut parser, None).await;

        assert!(result.is_err(), "should have timed out");
        let err = result.unwrap_err();
        match err {
            HarnessError::Timeout {
                elapsed,
                configured,
            } => {
                assert!(
                    elapsed >= Duration::from_millis(400),
                    "elapsed {elapsed:?} should be >= 400ms"
                );
                assert_eq!(configured, Duration::from_millis(500));
            }
            other => panic!("expected Timeout error, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn run_one_shot_nonzero_exit() {
        let dir = tempdir().unwrap();
        let script = dir.path().join("fail.sh");
        write_script(&script, "#!/bin/sh\necho 'about to fail' >&2\nexit 42\n");

        let runner = ChildProcessRunner::new(script.as_os_str(), dir.path()).with_heartbeat(false);
        let mut parser = EchoAllParser;
        let result = runner.run_one_shot(&[], None, &mut parser, None).await;

        assert!(result.is_err(), "should fail on non-zero exit");
        let err = result.unwrap_err();
        match err {
            HarnessError::ProcessExit { code, stderr } => {
                assert_eq!(code, Some(42));
                assert!(
                    stderr.contains("about to fail"),
                    "stderr should contain error message, got: {stderr:?}"
                );
            }
            other => panic!("expected ProcessExit error, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn scrubbed_env_injects_and_removes() {
        let dir = tempdir().unwrap();
        let script = dir.path().join("env.sh");
        write_script(
            &script,
            "#!/bin/sh\necho \"MY_VAR=$MY_VAR\"\necho \"HOME=${HOME:-unset}\"\n",
        );

        let env = ScrubbedEnv {
            inject: vec![("MY_VAR".into(), "hello_roko".into())],
            remove: vec!["HOME".into()],
        };
        let runner = ChildProcessRunner::new(script.as_os_str(), dir.path())
            .with_env(env)
            .with_heartbeat(false);
        let mut parser = EchoParser;
        let events = runner
            .run_one_shot(&[], None, &mut parser, None)
            .await
            .expect("run_one_shot should succeed");

        let texts: Vec<&str> = events
            .iter()
            .filter_map(|e| match e {
                HarnessEvent::Output(t) => Some(t.as_str()),
                _ => None,
            })
            .collect();
        assert!(
            texts.iter().any(|t| t.contains("MY_VAR=hello_roko")),
            "expected MY_VAR=hello_roko in output, got: {texts:?}"
        );
    }

    #[tokio::test]
    async fn run_one_shot_writes_stdin() {
        let dir = tempdir().unwrap();
        let script = dir.path().join("cat.sh");
        write_script(&script, "#!/bin/sh\ncat\n");

        let runner = ChildProcessRunner::new(script.as_os_str(), dir.path()).with_heartbeat(false);
        let mut parser = EchoParser;
        let stdin_data = b"data from stdin\nline two\n";
        let events = runner
            .run_one_shot(&[], Some(stdin_data), &mut parser, None)
            .await
            .expect("run_one_shot should succeed");

        let texts: Vec<&str> = events
            .iter()
            .filter_map(|e| match e {
                HarnessEvent::Output(t) => Some(t.as_str()),
                _ => None,
            })
            .collect();
        assert!(
            texts.iter().any(|t| t.contains("data from stdin")),
            "expected stdin data in output, got: {texts:?}"
        );
        assert!(
            texts.iter().any(|t| t.contains("line two")),
            "expected 'line two' in output, got: {texts:?}"
        );
    }

    #[tokio::test]
    async fn spawn_persistent_starts_process() {
        let dir = tempdir().unwrap();
        // Use `cat` as the persistent process: it stays alive waiting for stdin.
        let runner = ChildProcessRunner::new("cat", dir.path()).with_heartbeat(false);

        let mut spawned = runner
            .spawn_persistent(&[])
            .expect("spawn_persistent should succeed");

        // The process should have a valid PID immediately after spawn.
        assert!(
            spawned.pid.is_some(),
            "expected SpawnedChild.pid to be Some after a successful spawn"
        );

        // Clean up: kill the persistent process.
        spawned.kill().await;
    }

    #[tokio::test]
    async fn spawn_persistent_nonexistent_binary_returns_io_error() {
        let dir = tempdir().unwrap();
        let runner = ChildProcessRunner::new("__roko_nonexistent_binary__", dir.path())
            .with_heartbeat(false);

        let result = runner.spawn_persistent(&[]);

        assert!(
            result.is_err(),
            "expected an error when spawning a nonexistent binary"
        );
        assert!(
            matches!(result, Err(HarnessError::Io(_))),
            "expected HarnessError::Io for a nonexistent binary"
        );
    }
}
