//! `ShellGate` — runs an arbitrary shell command; passes on exit code 0.
//!
//! `ShellGate` is the simplest real gate. It's useful as a building block and
//! for bespoke checks (custom lints, site-specific invariants, pre-commit-style
//! hooks). It never consults the input signal's body beyond reading a
//! [`GatePayload`] if present (for `working_dir` and environment).
//!
//! The command does not inherit roko's whole environment, which holds the
//! provider keys roko loads from its `.env` files: it gets the allowlisted
//! variables described in [`crate::gate_env`] plus the payload's explicit
//! ones.

use crate::compile_errors::{
    GateFailureClassification, classify_step_failure, render_failure_classification,
};
use crate::gate_env::{inherit_gate_env, inherit_gate_env_from};
use crate::payload::GatePayload;
use async_trait::async_trait;
use roko_core::{Context, Signal, Verdict, Verify};
use std::ffi::OsString;
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
    /// The verify phase the command checks, such as `test`, when known: it
    /// classifies a failure (`classify_step_failure`).
    phase: Option<String>,
    /// Optional sender for live line-by-line output streaming.
    /// Each line from stdout/stderr is forwarded as it arrives.
    line_sink: Option<mpsc::UnboundedSender<String>>,
    /// Variables inherited in place of roko's own environment, still
    /// filtered by the gate policy. `None` uses roko's environment.
    parent_env: Option<Vec<(String, OsString)>>,
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
            phase: None,
            line_sink: None,
            parent_env: None,
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

    /// Name the verify phase the command checks, such as `test`, so a
    /// failure classifies by it rather than by the gate's name.
    #[must_use]
    pub fn with_phase(mut self, phase: impl Into<String>) -> Self {
        self.phase = Some(phase.into());
        self
    }

    /// Classify a failure whose output is `output` for the verdict's error
    /// digest, by the gate's phase when it has one.
    fn classify_failure(
        &self,
        output: &str,
        summary: String,
        duration_ms: u64,
    ) -> GateFailureClassification {
        classify_step_failure(&self.name, self.phase.as_deref(), output)
            .with_summary(summary)
            .with_duration_ms(duration_ms)
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

    /// Inherit `vars` instead of roko's own environment. The gate policy
    /// still filters them, exactly as it filters roko's environment.
    #[must_use]
    pub fn with_parent_env<I, K, V>(mut self, vars: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<OsString>,
    {
        self.parent_env = Some(
            vars.into_iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect(),
        );
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

        let passthrough = payload
            .as_ref()
            .map_or(&[][..], |p| p.env_passthrough.as_slice());
        match &self.parent_env {
            Some(vars) => inherit_gate_env_from(&mut cmd, vars.iter().cloned(), passthrough),
            None => inherit_gate_env(&mut cmd, passthrough),
        }
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
                let classification = self.classify_failure(&reason, reason.clone(), elapsed);
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

        let result = timeout(Duration::from_millis(self.timeout_ms), stream_output).await;

        #[allow(clippy::cast_possible_truncation)]
        let elapsed = started.elapsed().as_millis() as u64;

        match result {
            Err(_timeout) => {
                terminate_child_process_group(child_pid).await;
                let reason = format!("timed out after {} ms", self.timeout_ms);
                let classification = self
                    .classify_failure(&reason, reason.clone(), elapsed)
                    .timed_out();
                Verdict::fail(&self.name, reason)
                    .with_error_digest(render_failure_classification(&classification))
                    .with_duration(elapsed)
            }
            Ok((_stdout, _stderr, Err(io_err))) => {
                let reason = format!("wait failed: {io_err}");
                let classification = self.classify_failure(&reason, reason.clone(), elapsed);
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
                    let classification = self.classify_failure(&combined, reason.clone(), elapsed);
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
        assert!(!crate::compile_errors::verdict_timed_out(&v));
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
        assert!(
            crate::compile_errors::verdict_timed_out(&v),
            "the digest records the timeout: {:?}",
            v.error_digest
        );
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

    #[cfg(unix)]
    #[tokio::test]
    async fn verify_command_cannot_see_provider_keys() {
        let path = std::env::var_os("PATH")
            .filter(|path| !path.is_empty())
            .unwrap_or_else(|| "/usr/bin:/bin".into());
        let payload = GatePayload::in_dir(std::env::temp_dir())
            .with_env("ROKO_GATE_TASK_ID", "T01")
            .with_env_passthrough(["DATABASE_URL"]);
        let signal = Signal::builder(Kind::Task)
            .body(Body::from_json(&payload).expect("payload serializes"))
            .build();
        let gate = ShellGate::new("bash", vec!["-c".into(), "env".into()]).with_parent_env([
            ("PATH", path),
            ("HOME", "/tmp/gate-home".into()),
            ("CARGO_HOME", "/tmp/gate-cargo".into()),
            ("ROKO_BIN", "/tmp/roko".into()),
            ("DATABASE_URL", "postgres://gate-db".into()),
            ("OPENAI_API_KEY", "sk-test-not-real".into()),
            ("ANTHROPIC_API_KEY", "sk-ant-test-not-real".into()),
            ("CARGO_REGISTRY_TOKEN", "cio-test-not-real".into()),
            ("UNRELATED_SETTING", "not-inherited".into()),
        ]);

        let v = gate.verify(&signal, &Context::at(0)).await;

        assert!(v.passed, "env should run: {}", v.reason);
        let detail = v.detail.as_deref().expect("env output");
        for leaked in [
            "sk-test-not-real",
            "sk-ant-test-not-real",
            "cio-test-not-real",
            "not-inherited",
        ] {
            assert!(!detail.contains(leaked), "{leaked} leaked:\n{detail}");
        }
        for kept in [
            "HOME=/tmp/gate-home",
            "CARGO_HOME=/tmp/gate-cargo",
            "ROKO_BIN=/tmp/roko",
            "DATABASE_URL=postgres://gate-db",
            "ROKO_GATE_TASK_ID=T01",
        ] {
            assert!(detail.contains(kept), "{kept} missing:\n{detail}");
        }
        assert!(detail.lines().any(|line| line.starts_with("PATH=/")));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn verify_command_can_run_cargo_with_the_allowlisted_env() {
        let gate = ShellGate::new("bash", vec!["-c".into(), "cargo --version".into()]);
        let v = gate.verify(&empty_signal(), &Context::at(0)).await;
        assert!(v.passed, "cargo should still be found: {}", v.reason);
    }

    #[tokio::test]
    async fn custom_name_appears_in_verdict() {
        let gate = ShellGate::new("true", vec![]).with_name("my_custom_gate");
        let v = gate.verify(&empty_signal(), &Context::at(0)).await;
        assert_eq!(v.gate, "my_custom_gate");
    }
}
