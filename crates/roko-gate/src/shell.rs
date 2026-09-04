//! `ShellGate` — runs an arbitrary shell command; passes on exit code 0.
//!
//! `ShellGate` is the simplest real gate. It's useful as a building block and
//! for bespoke checks (custom lints, site-specific invariants, pre-commit-style
//! hooks). It never consults the input signal's body beyond reading a
//! [`GatePayload`] if present (for `working_dir` and environment).

use crate::compile_errors::{render_failure_classification, structured_gate_failure};
use crate::payload::GatePayload;
use async_trait::async_trait;
use roko_core::{Context, Signal, Verdict, Verify};
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::time::timeout;

/// A gate that runs a fixed shell command; pass = exit code 0.
///
/// The signal's body (if a [`GatePayload`]) provides `working_dir` and
/// `extra_env`. If the body is missing or malformed, the gate uses the
/// current process's cwd and no extra env.
pub struct ShellGate {
    program: String,
    args: Vec<String>,
    timeout_ms: u64,
    name: String,
    /// Optional sender for live line-by-line output streaming.
    /// Each line from stdout/stderr is forwarded as it arrives.
    line_sink: Option<mpsc::UnboundedSender<String>>,
}

impl ShellGate {
    /// Construct a shell gate that runs `program` with `args`.
    #[must_use]
    pub fn new(program: impl Into<String>, args: Vec<String>) -> Self {
        let program = program.into();
        let name = format!("shell:{program}");
        Self {
            program,
            args,
            timeout_ms: 300_000, // 5 minutes
            name,
            line_sink: None,
        }
    }

    /// Override the timeout in milliseconds (default: 5 minutes).
    #[must_use]
    pub const fn with_timeout_ms(mut self, ms: u64) -> Self {
        self.timeout_ms = ms;
        self
    }

    /// Override the gate's display name.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Attach a line sink for live output streaming.
    ///
    /// Each line from stdout and stderr will be sent through this channel as
    /// it arrives, enabling real-time TUI display during gate execution.
    #[must_use]
    pub fn with_line_sink(mut self, sink: mpsc::UnboundedSender<String>) -> Self {
        self.line_sink = Some(sink);
        self
    }
}

impl roko_core::Cell for ShellGate {
    fn cell_id(&self) -> &str {
        "shell-gate"
    }
    fn cell_name(&self) -> &str {
        "ShellGate"
    }
    fn protocols(&self) -> Vec<roko_core::ProtocolId> {
        vec![roko_core::ProtocolId::Verify]
    }
}

#[async_trait]
impl Verify for ShellGate {
    async fn verify(&self, signal: &Signal, _ctx: &Context) -> Verdict {
        let started = Instant::now();
        let payload: Option<GatePayload> = signal.body.as_json().ok();

        let mut cmd = Command::new(&self.program);
        cmd.args(&self.args);
        cmd.kill_on_drop(true);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        configure_child_process_group(&mut cmd);

        if let Some(ref p) = payload {
            cmd.current_dir(&p.working_dir);
            if let Some(ref tgt) = p.target_dir {
                cmd.env("CARGO_TARGET_DIR", tgt);
            }
            for (k, v) in &p.extra_env {
                cmd.env(k, v);
            }
        }

        let child = cmd.spawn();
        let mut child = match child {
            Ok(child) => child,
            Err(io_err) => {
                #[allow(clippy::cast_possible_truncation)]
                let elapsed = started.elapsed().as_millis() as u64;
                let reason = format!("spawn failed: {io_err}");
                let classification =
                    structured_gate_failure(&self.name, &reason, reason.clone(), elapsed);
                return Verdict::fail(&self.name, reason)
                    .with_error_digest(render_failure_classification(&classification))
                    .with_duration(elapsed);
            }
        };

        let child_pid = child.id();
        let stdout_pipe = child.stdout.take();
        let stderr_pipe = child.stderr.take();
        let line_sink = self.line_sink.clone();

        // Stream stdout and stderr line-by-line, forwarding each line through
        // the optional sink for live TUI display while accumulating the full
        // output for the final verdict.
        let stream_output = async {
            let mut stdout_buf = String::new();
            let mut stderr_buf = String::new();

            let stdout_task = {
                let sink = line_sink.clone();
                async move {
                    if let Some(pipe) = stdout_pipe {
                        let reader = BufReader::new(pipe);
                        let mut lines = reader.lines();
                        while let Ok(Some(line)) = lines.next_line().await {
                            if let Some(ref sink) = sink {
                                let _ = sink.send(line.clone());
                            }
                            stdout_buf.push_str(&line);
                            stdout_buf.push('\n');
                        }
                    }
                    stdout_buf
                }
            };

            let stderr_task = async move {
                if let Some(pipe) = stderr_pipe {
                    let reader = BufReader::new(pipe);
                    let mut lines = reader.lines();
                    while let Ok(Some(line)) = lines.next_line().await {
                        if let Some(ref line_sink) = line_sink {
                            let _ = line_sink.send(line.clone());
                        }
                        stderr_buf.push_str(&line);
                        stderr_buf.push('\n');
                    }
                }
                stderr_buf
            };

            let (stdout_out, stderr_out) = tokio::join!(stdout_task, stderr_task);
            let status = child.wait().await;
            (stdout_out, stderr_out, status)
        };

        let result = timeout(
            Duration::from_millis(self.timeout_ms),
            stream_output,
        )
        .await;

        #[allow(clippy::cast_possible_truncation)]
        let elapsed = started.elapsed().as_millis() as u64;

        match result {
            Err(_timeout) => {
                terminate_child_process_group(child_pid).await;
                let reason = format!("timed out after {} ms", self.timeout_ms);
                let classification =
                    structured_gate_failure(&self.name, &reason, reason.clone(), elapsed);
                Verdict::fail(&self.name, reason)
                    .with_error_digest(render_failure_classification(&classification))
                    .with_duration(elapsed)
            }
            Ok((stdout, stderr, Err(io_err))) => {
                let reason = format!("wait failed: {io_err}");
                let classification =
                    structured_gate_failure(&self.name, &reason, reason.clone(), elapsed);
                Verdict::fail(&self.name, reason)
                    .with_error_digest(render_failure_classification(&classification))
                    .with_duration(elapsed)
            }
            Ok((stdout, stderr, Ok(status))) => {
                let combined = if stderr.is_empty() {
                    stdout
                } else {
                    format!("{stdout}\n---stderr---\n{stderr}")
                };
                if status.success() {
                    Verdict::pass(&self.name)
                        .with_detail(combined)
                        .with_duration(elapsed)
                } else {
                    let code = status
                        .code()
                        .map_or_else(|| "terminated by signal".into(), |c| c.to_string());
                    let reason = format!("exit code: {code}");
                    let classification =
                        structured_gate_failure(&self.name, &combined, reason.clone(), elapsed);
                    Verdict::fail(&self.name, reason)
                        .with_detail(combined)
                        .with_error_digest(render_failure_classification(&classification))
                        .with_duration(elapsed)
                }
            }
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(unix)]
fn configure_child_process_group(cmd: &mut Command) {
    cmd.process_group(0);
}

#[cfg(not(unix))]
fn configure_child_process_group(_cmd: &mut Command) {}

#[cfg(unix)]
async fn terminate_child_process_group(child_pid: Option<u32>) {
    let Some(pid) = child_pid else {
        return;
    };
    let group_arg = format!("-{pid}");
    let _ = Command::new("kill")
        .arg("-TERM")
        .arg(&group_arg)
        .status()
        .await;
    tokio::time::sleep(Duration::from_millis(250)).await;
    let _ = Command::new("kill")
        .arg("-KILL")
        .arg(group_arg)
        .status()
        .await;
}

#[cfg(not(unix))]
async fn terminate_child_process_group(_child_pid: Option<u32>) {}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::{Body, Kind};

    fn scaled_test_timeout_ms(ms: u64) -> u64 {
        if std::env::var("CI").is_ok_and(|value| value == "true") {
            ms.saturating_mul(10)
        } else {
            ms
        }
    }

    fn empty_signal() -> Signal {
        Signal::builder(Kind::Task).body(Body::empty()).build()
    }

    #[tokio::test]
    async fn true_command_passes() {
        let gate = ShellGate::new("true", vec![]);
        let v = gate.verify(&empty_signal(), &Context::at(0)).await;
        assert!(v.passed);
        assert_eq!(v.gate, "shell:true");
    }

    #[tokio::test]
    async fn false_command_fails() {
        let gate = ShellGate::new("false", vec![]);
        let v = gate.verify(&empty_signal(), &Context::at(0)).await;
        assert!(!v.passed);
        assert!(v.reason.contains("exit code"));
        let digest = v.error_digest.as_deref().expect("structured digest");
        let classification: crate::compile_errors::GateFailureClassification =
            serde_json::from_str(digest).expect("digest parses");
        assert_eq!(classification.gate, "shell:false");
        assert_eq!(classification.summary, v.reason);
        assert_eq!(classification.duration_ms, Some(v.duration_ms));
    }

    #[tokio::test]
    async fn echo_command_captures_output() {
        let gate = ShellGate::new("echo", vec!["hello from gate".into()]);
        let v = gate.verify(&empty_signal(), &Context::at(0)).await;
        assert!(v.passed);
        let detail = v.detail.as_deref().unwrap();
        assert!(detail.contains("hello from gate"));
    }

    #[tokio::test]
    async fn nonexistent_command_fails_gracefully() {
        let gate = ShellGate::new("definitely_not_a_real_command_xyz", vec![]);
        let v = gate.verify(&empty_signal(), &Context::at(0)).await;
        assert!(!v.passed);
        assert!(v.reason.contains("spawn failed"));
    }

    #[tokio::test]
    async fn timeout_causes_failure() {
        let gate =
            ShellGate::new("sleep", vec!["10".into()]).with_timeout_ms(scaled_test_timeout_ms(100));
        let v = gate.verify(&empty_signal(), &Context::at(0)).await;
        assert!(!v.passed);
        assert!(v.reason.contains("timed out"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn timeout_kills_shell_descendants() {
        let tempdir = tempfile::tempdir().expect("tempdir should be created");
        let marker = tempdir.path().join("timeout-marker");
        let marker_arg = marker.to_string_lossy();
        assert!(!marker_arg.contains('\''));
        let command = format!("(sleep 1; touch '{marker_arg}') & wait");
        let gate = ShellGate::new("bash", vec!["-c".into(), command])
            .with_timeout_ms(scaled_test_timeout_ms(100));

        let v = gate.verify(&empty_signal(), &Context::at(0)).await;

        assert!(!v.passed);
        assert!(v.reason.contains("timed out"));
        tokio::time::sleep(Duration::from_millis(scaled_test_timeout_ms(1_500))).await;
        assert!(
            !marker.exists(),
            "timed-out shell descendants should not keep running"
        );
    }

    #[tokio::test]
    async fn records_duration() {
        let gate = ShellGate::new("true", vec![]);
        let v = gate.verify(&empty_signal(), &Context::at(0)).await;
        assert!(v.passed);
        // Duration should be non-zero but small.
        assert!(v.duration_ms < 5000);
    }

    #[tokio::test]
    async fn custom_name_appears_in_verdict() {
        let gate = ShellGate::new("true", vec![]).with_name("my_custom_gate");
        let v = gate.verify(&empty_signal(), &Context::at(0)).await;
        assert_eq!(v.gate, "my_custom_gate");
    }
}
