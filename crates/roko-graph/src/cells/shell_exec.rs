//! The `shell.exec` and `verify.command` cells (9126): a graph that
//! `roko graph run` or a trigger starts runs an operator-authored shell
//! command.
//!
//! Graph files are operator-authored and trigger payloads are not, and the
//! cells keep that line: the command comes from the node's config alone, and
//! the node's input signals, a trigger's payload among them, reach it only as
//! a JSON file whose path is `ROKO_CELL_INPUT`, never inside the command.
//!
//! A node's config:
//!
//! - `command` (required), run by `sh -c`;
//! - `cwd`, a directory relative to the workspace that stays inside it;
//! - `timeout_secs`, 120 by default and at most 3600, after which the
//!   command's whole process group is killed;
//! - `env`, names of roko's environment variables the command inherits
//!   beyond the gate allow-list (`roko_core::child_env`).
//!
//! `shell.exec` outputs a `ProcessExit` signal with the exit code, the tails
//! of stdout and stderr with secrets scrubbed, and the duration; a non-zero
//! exit is the node's error. `verify.command` outputs a `GateVerdict` signal
//! instead, which passes on exit 0 and fails otherwise: a failed check is not
//! an error, which only a command that cannot run is. Both refuse to start in
//! a context whose capabilities lack shell execution.

use std::path::{Component, Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

use roko_core::error::{Result, RokoError};
use roko_core::{Body, Capability, Kind, ProtocolId, Signal};
use serde_json::json;

use crate::cell::{Cell, CellContext};

/// A command's time limit when its node names none, in seconds.
const DEFAULT_TIMEOUT_SECS: u64 = 120;

/// The longest time limit a node may name, in seconds.
const MAX_TIMEOUT_SECS: u64 = 3600;

/// The most of stdout and of stderr an output keeps, in bytes, from the end.
const TAIL_BYTES: usize = 4 * 1024;

/// Which of the two cells a [`ShellExecCell`] is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellExecMode {
    /// `shell.exec`: a non-zero exit is the node's error.
    Exec,
    /// `verify.command`: the exit is a pass or fail verdict.
    Verify,
}

/// A `shell.exec` or `verify.command` node's config.
#[derive(Clone, Debug)]
struct ShellExecConfig {
    command: String,
    cwd: Option<PathBuf>,
    timeout: Duration,
    env: Vec<String>,
}

impl ShellExecConfig {
    /// The config in `node`, or what is wrong with it.
    fn parse(node: &toml::Value) -> std::result::Result<Self, String> {
        let command = node
            .get("command")
            .and_then(toml::Value::as_str)
            .filter(|command| !command.trim().is_empty())
            .ok_or("it needs a `command`")?
            .to_string();
        let cwd = match node.get("cwd").map(toml::Value::as_str) {
            None => None,
            Some(None) => return Err("its `cwd` must be text".to_string()),
            Some(Some(cwd)) => {
                let cwd = PathBuf::from(cwd);
                let climbs = cwd
                    .components()
                    .any(|part| matches!(part, Component::ParentDir));
                if cwd.is_absolute() || climbs {
                    return Err(format!(
                        "its `cwd` {} must stay inside the workspace",
                        cwd.display()
                    ));
                }
                Some(cwd)
            }
        };
        let timeout_secs = match node.get("timeout_secs") {
            None => DEFAULT_TIMEOUT_SECS,
            Some(value) => value
                .as_integer()
                .and_then(|secs| u64::try_from(secs).ok())
                .filter(|secs| (1..=MAX_TIMEOUT_SECS).contains(secs))
                .ok_or(format!(
                    "its `timeout_secs` must be from 1 to {MAX_TIMEOUT_SECS}"
                ))?,
        };
        let env = node
            .get("env")
            .and_then(toml::Value::as_array)
            .map(|names| {
                names
                    .iter()
                    .filter_map(toml::Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        Ok(Self {
            command,
            cwd,
            timeout: Duration::from_secs(timeout_secs),
            env,
        })
    }
}

/// The `shell.exec` cell for a node whose config is `node`, as the default
/// registry builds it.
#[must_use]
pub fn exec_cell(node: toml::Value) -> Box<dyn Cell> {
    Box::new(ShellExecCell::new(ShellExecMode::Exec, &node))
}

/// The `verify.command` cell for a node whose config is `node`.
#[must_use]
pub fn verify_cell(node: toml::Value) -> Box<dyn Cell> {
    Box::new(ShellExecCell::new(ShellExecMode::Verify, &node))
}

/// A `shell.exec` or `verify.command` cell ([module docs](self)).
pub struct ShellExecCell {
    mode: ShellExecMode,
    config: std::result::Result<ShellExecConfig, String>,
    workdir: Option<PathBuf>,
}

impl ShellExecCell {
    /// The cell of `mode` for a node whose config is `node`. A config that is
    /// wrong fails the node when it runs.
    #[must_use]
    pub fn new(mode: ShellExecMode, node: &toml::Value) -> Self {
        Self {
            mode,
            config: ShellExecConfig::parse(node),
            workdir: None,
        }
    }

    /// Run in `workdir` instead of the process's working directory.
    #[must_use]
    pub fn with_workdir(mut self, workdir: PathBuf) -> Self {
        self.workdir = Some(workdir);
        self
    }

    /// The cell's type in a graph file.
    const fn cell_type(&self) -> &'static str {
        match self.mode {
            ShellExecMode::Exec => "shell.exec",
            ShellExecMode::Verify => "verify.command",
        }
    }
}

#[async_trait::async_trait]
impl Cell for ShellExecCell {
    fn cell_id(&self) -> &str {
        self.cell_type()
    }

    fn cell_name(&self) -> &str {
        match self.mode {
            ShellExecMode::Exec => "ShellExecCell",
            ShellExecMode::Verify => "VerifyCommandCell",
        }
    }

    fn protocols(&self) -> Vec<ProtocolId> {
        match self.mode {
            ShellExecMode::Exec => vec![ProtocolId::Connect],
            ShellExecMode::Verify => vec![ProtocolId::Verify],
        }
    }

    fn estimated_cost(&self) -> Option<f64> {
        None
    }

    async fn execute(&self, input: Vec<Signal>, ctx: &CellContext) -> Result<Vec<Signal>> {
        let cell = self.cell_type();
        if let Some(capabilities) = &ctx.capabilities
            && !capabilities.contains(Capability::Execute)
        {
            return Err(RokoError::invalid(format!(
                "cell '{cell}' requires capability {}",
                Capability::Execute
            )));
        }
        let config = self
            .config
            .as_ref()
            .map_err(|problem| RokoError::invalid(format!("cell '{cell}': {problem}")))?;
        let workdir = match &self.workdir {
            Some(workdir) => workdir.clone(),
            None => std::env::current_dir()
                .map_err(|error| RokoError::invalid(format!("cell '{cell}': {error}")))?,
        };
        let dir = config
            .cwd
            .as_ref()
            .map_or_else(|| workdir.clone(), |cwd| workdir.join(cwd));
        let input_file = InputFile::write(&input)
            .map_err(|error| RokoError::invalid(format!("cell '{cell}': {error}")))?;
        let ran = run(config, &dir, &input_file.0).await;
        drop(input_file);
        let (ran, timed_out) = match ran {
            Ok(ran) => (ran, false),
            Err(Failure::TimedOut(ran)) => (ran, true),
            Err(Failure::Spawn(error)) => {
                return Err(RokoError::invalid(format!("cell '{cell}': {error}")));
            }
        };
        let passed = !timed_out && ran.exit_code == Some(0);
        let mut body = json!({
            "exit_code": ran.exit_code,
            "timed_out": timed_out,
            "stdout_tail": tail(&ran.stdout),
            "stderr_tail": tail(&ran.stderr),
            "duration_ms": ran.duration_ms,
        });
        if self.mode == ShellExecMode::Verify {
            body["passed"] = json!(passed);
            let verdict = Signal::builder(Kind::GateVerdict)
                .body(Body::Json(body))
                .tag("cell", cell)
                .build();
            return Ok(vec![verdict]);
        }
        if !passed {
            let ended = if timed_out {
                format!(
                    "ran past its {} s limit and was killed",
                    config.timeout.as_secs()
                )
            } else {
                format!("exited with code {}", ran.exit_code.unwrap_or(-1))
            };
            return Err(RokoError::Verify {
                gate: cell.to_string(),
                message: format!("`{}` {ended}: {}", config.command, tail(&ran.stderr)),
            });
        }
        let exit = Signal::builder(Kind::ProcessExit)
            .body(Body::Json(body))
            .tag("cell", cell)
            .build();
        Ok(vec![exit])
    }
}

/// The node's input signals as a JSON file, removed when dropped.
struct InputFile(PathBuf);

impl InputFile {
    fn write(input: &[Signal]) -> std::result::Result<Self, String> {
        let name = format!("roko-cell-input-{}.json", uuid::Uuid::new_v4().simple());
        let path = std::env::temp_dir().join(name);
        let json = serde_json::to_vec(input).map_err(|error| error.to_string())?;
        std::fs::write(&path, json).map_err(|error| format!("{}: {error}", path.display()))?;
        Ok(Self(path))
    }
}

impl Drop for InputFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// What a run of the command gave.
struct Ran {
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
    duration_ms: u64,
}

/// Why a run gave no exit.
enum Failure {
    /// The command could not start.
    Spawn(String),
    /// The command ran past its time limit, and its process group was
    /// killed: what it had written by then.
    TimedOut(Ran),
}

/// Run `config`'s command by `sh -c` in `dir`, with `ROKO_CELL_INPUT` naming
/// `input`, in a process group of its own that a timeout kills whole.
async fn run(
    config: &ShellExecConfig,
    dir: &Path,
    input: &Path,
) -> std::result::Result<Ran, Failure> {
    let mut command = std::process::Command::new("sh");
    command
        .arg("-c")
        .arg(&config.command)
        .current_dir(dir)
        .env("ROKO_CELL_INPUT", input);
    roko_core::child_env::apply_gate_env(
        &mut command,
        roko_core::child_env::process_env(),
        &config.env,
    );
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt as _;
        command.process_group(0);
    }
    let mut command = tokio::process::Command::from(command);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let started = Instant::now();
    let child = command
        .spawn()
        .map_err(|error| Failure::Spawn(format!("cannot start `sh`: {error}")))?;
    let group = child.id();
    let elapsed = || u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    match tokio::time::timeout(config.timeout, child.wait_with_output()).await {
        Ok(Ok(output)) => Ok(Ran {
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            duration_ms: elapsed(),
        }),
        Ok(Err(error)) => Err(Failure::Spawn(format!(
            "the command failed to run: {error}"
        ))),
        Err(_) => {
            if let Some(group) = group {
                kill_group(group);
            }
            Err(Failure::TimedOut(Ran {
                exit_code: None,
                stdout: String::new(),
                stderr: String::new(),
                duration_ms: elapsed(),
            }))
        }
    }
}

/// Kill process group `group`: the command and everything it started.
fn kill_group(group: u32) {
    let target = format!("-{group}");
    let _ = std::process::Command::new("kill")
        .args(["-KILL", "--", &target])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

/// The last [`TAIL_BYTES`] of `text`, with the process's secrets scrubbed.
fn tail(text: &str) -> String {
    let start = text.ceil_char_boundary(text.len().saturating_sub(TAIL_BYTES));
    roko_core::obs::scrub::scrub_secrets(&text[start..]).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A cell of `mode` in `workdir` for the node config `config`, TOML.
    fn cell(mode: ShellExecMode, config: &str, workdir: &Path) -> ShellExecCell {
        let node: toml::Value = toml::from_str(config).expect("node config");
        ShellExecCell::new(mode, &node).with_workdir(workdir.to_path_buf())
    }

    /// 9126: a payload holding `$(touch pwned)` reaches the command only as
    /// the file `ROKO_CELL_INPUT` names, so no file is made; a non-zero exit
    /// fails a `shell.exec` node with its code; a timeout kills the command's
    /// whole process group; and `verify.command` turns exit 1 into a fail
    /// verdict, not an error.
    #[tokio::test]
    async fn shell_exec_cell_runs_command_without_interpolating_payload() {
        let temp = tempfile::tempdir().expect("tempdir");
        let ctx = CellContext::new();
        let payload = Signal::builder(Kind::Task)
            .body(Body::text("$(touch pwned)"))
            .build();
        let copy = cell(
            ShellExecMode::Exec,
            r#"command = 'cat "$ROKO_CELL_INPUT" > seen.json'"#,
            temp.path(),
        );
        let output = copy
            .execute(vec![payload], &ctx)
            .await
            .expect("the copy runs");
        assert_eq!(output[0].kind, Kind::ProcessExit);
        assert!(!temp.path().join("pwned").exists(), "the payload ran");
        let seen = std::fs::read_to_string(temp.path().join("seen.json")).expect("seen");
        assert!(seen.contains("$(touch pwned)"), "{seen}");

        let failing = cell(ShellExecMode::Exec, "command = 'exit 3'", temp.path());
        let error = failing.execute(Vec::new(), &ctx).await.expect_err("exit 3");
        assert!(error.to_string().contains("exited with code 3"), "{error}");

        let slow = cell(
            ShellExecMode::Exec,
            "command = '(sleep 2; touch late) & wait'\ntimeout_secs = 1",
            temp.path(),
        );
        let error = slow.execute(Vec::new(), &ctx).await.expect_err("a timeout");
        assert!(error.to_string().contains("was killed"), "{error}");
        tokio::time::sleep(Duration::from_millis(2_500)).await;
        assert!(!temp.path().join("late").exists(), "the group lived on");

        let check = cell(ShellExecMode::Verify, "command = 'exit 1'", temp.path());
        let verdict = check.execute(Vec::new(), &ctx).await.expect("a verdict");
        assert_eq!(verdict[0].kind, Kind::GateVerdict);
        let body: serde_json::Value = verdict[0].body.as_json().expect("a JSON verdict");
        assert_eq!(body["passed"], false, "{body}");
        assert_eq!(body["exit_code"], 1, "{body}");

        let read_only = roko_core::CapabilitySet::from([Capability::ReadFs]);
        let confined = CellContext::new().with_capabilities(read_only);
        let refused = check.execute(Vec::new(), &confined).await;
        assert!(refused.is_err(), "{refused:?}");
    }
}
