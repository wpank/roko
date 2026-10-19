//! The lifecycle of a project's `roko serve` (gap-fb1594): start it in the
//! background, tell whether it runs and which build it is, stop it and
//! restart it, for one project or for every serve on the machine.
//!
//! # Ours and alive
//!
//! A serve holds its workspace's lock (`.roko/runtime/roko.lock`, which
//! records its PID) for its whole life, writes `.roko/runtime/serve.json` once
//! its listener is bound, and answers `GET /health` on a loopback bind with the
//! `instance_id` that file records. [`check_workspace_serve`] looks at all
//! three, and everything that acts on a running serve goes through it:
//! `roko serve --detach|status|stop|restart`,
//! [`crate::serve_client::discover_workspace_server`] and
//! [`crate::doctor::serve_holding_workspace`]. Nothing here signals a process
//! that fails it.
//!
//! # Machine registry
//!
//! A serve also writes its record into `~/.roko/runtime/serves/`
//! ([`endpoint::registry_dir`]); `--all` reads it and prunes the entries of
//! serves that are gone.

use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::io::{self, Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use reqwest::header::HeaderMap;
use roko_serve::endpoint::{self, ServeArgs, ServeEndpoint};
use serde::Serialize;
use serde_json::{Value, json};

/// How long `/health` has to answer for a serve to count as alive.
pub const HEALTH_TIMEOUT: Duration = Duration::from_secs(1);

/// How long `--detach` waits by default for the serve it starts to answer.
pub const DEFAULT_DETACH_WAIT: Duration = Duration::from_secs(60);

/// How long `stop` waits by default for a serve to let go of its workspace
/// after SIGTERM, before SIGKILL.
pub const DEFAULT_STOP_TIMEOUT: Duration = Duration::from_secs(20);

/// How many lines of the serve log a failure carries.
pub const LOG_TAIL_LINES: usize = 20;

/// What the CLI says when no model provider is configured, which the log of
/// a detached serve that failed for it shows.
pub const NO_PROVIDER_MESSAGE: &str = "no LLM provider configured";

/// How long `/api/health` has to answer with the runs a serve has going.
const ACTIVE_RUNS_TIMEOUT: Duration = Duration::from_secs(3);

/// How long `<exe> --version` has to answer.
const VERSION_TIMEOUT: Duration = Duration::from_secs(5);

/// How long `--detach` gives a process that holds the workspace lock to show
/// itself as a starting serve before it reports the workspace `locked`.
const LOCK_GRACE: Duration = Duration::from_secs(3);

/// How long a stop waits for a process to go after SIGKILL.
const KILL_WAIT: Duration = Duration::from_secs(2);

/// How often waits look again.
const POLL: Duration = Duration::from_millis(100);

// ---------------------------------------------------------------------------
// Ours and alive
// ---------------------------------------------------------------------------

/// What one look at a workspace's serve found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ServeState {
    /// It runs and answers as itself.
    Running,
    /// No `serve.json`: no serve runs here.
    Stopped,
    /// `serve.json` names a process that is gone, or that does not hold the
    /// workspace: files a dead serve left.
    Stale,
    /// Its process is alive and holds the workspace, but `/health` does not
    /// answer as it within [`HEALTH_TIMEOUT`].
    Unresponsive,
}

impl ServeState {
    /// The state's name in `roko serve status --json`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Stopped => "stopped",
            Self::Stale => "stale",
            Self::Unresponsive => "unresponsive",
        }
    }
}

/// One look at a workspace's serve.
#[derive(Debug, Clone)]
pub struct ServeCheck {
    /// The workspace looked at.
    pub workdir: PathBuf,
    /// What was found.
    pub state: ServeState,
    /// The workspace's `serve.json`, when it has one.
    pub endpoint: Option<ServeEndpoint>,
    /// What `/health` answered, when it answered as this serve.
    pub health: Option<Value>,
}

/// Look at `workdir`'s serve, changing nothing: it runs when `serve.json`
/// exists, its PID is alive and holds the workspace lock, and `/health`
/// answers within [`HEALTH_TIMEOUT`] with the `instance_id` `serve.json`
/// records. A `serve.json` an older roko wrote records no instance id; its
/// serve counts when `/health` answers at all.
#[must_use]
pub fn check_workspace_serve(workdir: &Path) -> ServeCheck {
    let mut check = ServeCheck {
        workdir: workdir.to_path_buf(),
        state: ServeState::Stopped,
        endpoint: endpoint::read_endpoint(workdir),
        health: None,
    };
    let Some(served) = check.endpoint.as_ref() else {
        return check;
    };
    if !holds_workspace(workdir, served.pid) {
        check.state = ServeState::Stale;
        return check;
    }
    match get_json(&served.url, "/health", &HeaderMap::new(), HEALTH_TIMEOUT) {
        Some((_, health)) if answers_as(served, &health) => {
            check.state = ServeState::Running;
            check.health = Some(health);
        }
        _ => check.state = ServeState::Unresponsive,
    }
    check
}

/// `workdir`'s serve, when it runs: ours and alive ([`check_workspace_serve`]).
#[must_use]
pub fn running_workspace_serve(workdir: &Path) -> Option<ServeEndpoint> {
    let check = check_workspace_serve(workdir);
    if check.state == ServeState::Running {
        check.endpoint
    } else {
        None
    }
}

/// Whether `pid` is alive and holds `workdir`'s workspace lock, which a serve
/// takes before anything else and keeps for its whole life.
fn holds_workspace(workdir: &Path, pid: u32) -> bool {
    let roko_dir = workdir.join(".roko");
    process_is_alive(pid)
        && crate::workspace_lock::workspace_lock_is_held(&roko_dir)
        && crate::workspace_lock::workspace_lock_owner_pid(&roko_dir) == Some(pid)
}

/// Whether a `/health` answer comes from the serve `served` records.
fn answers_as(served: &ServeEndpoint, health: &Value) -> bool {
    if served.instance_id.is_empty() {
        // An older serve names no instance; its PID and lock matched already.
        health.get("status").is_some()
    } else {
        health.get("instance_id").and_then(Value::as_str) == Some(served.instance_id.as_str())
    }
}

/// Remove what the dead serve `pid` left in `workdir` — its token,
/// `serve.json` and registry entry — while they still name it. It takes the
/// workspace lock to do so, so a serve starting now keeps its own files;
/// `false` when another process holds the workspace and nothing was removed.
pub fn clean_stale_serve(workdir: &Path, pid: u32) -> bool {
    let Ok(_lock) = crate::workspace_lock::acquire_workspace_lock(&workdir.join(".roko")) else {
        return false;
    };
    endpoint::remove_advertisement_if_owned(workdir, endpoint::registry_dir().as_deref(), pid);
    true
}

/// How many runs and plans `served` has going: `/api/health`'s
/// `active_runs + active_plans`, asked with the credentials a client of the
/// workspace sends ([`crate::serve_client::workspace_credentials`]); `None`
/// when it does not say.
#[must_use]
pub fn active_runs(workdir: &Path, served: &ServeEndpoint) -> Option<u64> {
    let mut credentials = crate::serve_client::workspace_credentials(workdir);
    if credentials.is_empty() {
        credentials.push(HeaderMap::new());
    }
    credentials.iter().find_map(|headers| {
        let (_, body) = get_json(&served.url, "/api/health", headers, ACTIVE_RUNS_TIMEOUT)?;
        let runs = body.get("active_runs")?.as_u64()?;
        let plans = body.get("active_plans")?.as_u64()?;
        Some(runs + plans)
    })
}

/// Whether the binary now at `served`'s `exe` path is another build than the
/// one running: its `--version` names another commit. `None` when that is
/// not known: no executable or commit recorded, or no answer.
#[must_use]
pub fn outdated(served: &ServeEndpoint) -> Option<bool> {
    let exe = served.exe.as_deref()?;
    let running = served.git_sha.as_deref()?;
    let installed = binary_git_sha(exe)?;
    Some(installed != running)
}

/// The commit `<exe> --version` names (`git <sha>`), within
/// [`VERSION_TIMEOUT`].
#[must_use]
pub fn binary_git_sha(exe: &Path) -> Option<String> {
    let mut child = Command::new(exe)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let deadline = Instant::now() + VERSION_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => break,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(20)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
    let mut version = String::new();
    child.stdout.take()?.read_to_string(&mut version).ok()?;
    parse_git_sha(&version)
}

/// The commit in a `roko --version` line: `roko 0.1.0 (rustc …, target, git
/// e31f4d52c)`.
fn parse_git_sha(version: &str) -> Option<String> {
    let (_, rest) = version.rsplit_once("git ")?;
    let sha: String = rest
        .chars()
        .take_while(char::is_ascii_alphanumeric)
        .collect();
    (!sha.is_empty() && sha != "unknown").then_some(sha)
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Why a lifecycle command failed, as a client branches on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    /// The workdir is not a directory.
    NotAWorkspace,
    /// The workdir is not set up for roko: `roko serve` would refuse it.
    NotInitialized,
    /// The flags do not go together.
    BadFlags,
    /// The serve exited before it answered.
    StartFailed,
    /// The serve did not answer in time.
    Timeout,
    /// Another roko process, not a serve, holds the workspace.
    Locked,
    /// Runs are going, and `--force` was not given.
    Busy,
    /// The serve holds its workspace but does not answer.
    Unresponsive,
}

impl ErrorCode {
    /// The code's name in JSON output.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotAWorkspace => "not_a_workspace",
            Self::NotInitialized => "not_initialized",
            Self::BadFlags => "bad_flags",
            Self::StartFailed => "start_failed",
            Self::Timeout => "timeout",
            Self::Locked => "locked",
            Self::Busy => "busy",
            Self::Unresponsive => "unresponsive",
        }
    }

    /// The exit status: 2 for a usage error, 1 for a runtime failure.
    #[must_use]
    pub const fn exit_code(self) -> i32 {
        match self {
            Self::NotAWorkspace | Self::NotInitialized | Self::BadFlags => 2,
            _ => 1,
        }
    }
}

/// A failed lifecycle command: its code, a message, and the last lines of the
/// serve log when a start failed.
#[derive(Debug, Clone)]
pub struct LifecycleError {
    /// What failed.
    pub code: ErrorCode,
    /// What to tell the person, with the fix when there is one.
    pub message: String,
    /// The last [`LOG_TAIL_LINES`] lines of the serve log; untrusted text.
    pub log_tail: Vec<String>,
}

impl LifecycleError {
    fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            log_tail: Vec::new(),
        }
    }

    fn with_log(mut self, log: &Path) -> Self {
        self.log_tail = log_tail(log, LOG_TAIL_LINES);
        self
    }

    /// The `error` object of a failed command's JSON.
    #[must_use]
    pub fn to_json(&self) -> Value {
        let mut error = json!({"code": self.code.as_str(), "message": self.message});
        if !self.log_tail.is_empty() {
            error["log_tail"] = json!(self.log_tail);
        }
        error
    }
}

impl std::fmt::Display for LifecycleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for LifecycleError {}

/// `workdir` as a canonical absolute path, or `not_a_workspace`.
///
/// # Errors
///
/// When `workdir` is not a directory.
pub fn canonical_workdir(workdir: &Path) -> Result<PathBuf, LifecycleError> {
    match workdir.canonicalize() {
        Ok(path) if path.is_dir() => Ok(path),
        _ => Err(LifecycleError::new(
            ErrorCode::NotAWorkspace,
            format!("{} is not a directory", workdir.display()),
        )),
    }
}

// ---------------------------------------------------------------------------
// roko serve --detach
// ---------------------------------------------------------------------------

/// What `roko serve --detach` starts.
#[derive(Debug, Clone)]
pub struct DetachOptions {
    /// The workspace.
    pub workdir: PathBuf,
    /// `--port`; the configured port when free, else one the OS picks.
    pub port: Option<u16>,
    /// `--bind`.
    pub bind: Option<String>,
    /// `--idle-exit` seconds; `None` or `0` never.
    pub idle_exit_secs: Option<u64>,
    /// `--enable-terminal`.
    pub enable_terminal: bool,
    /// `--config`, which the serve reads as its config file.
    pub config: Option<PathBuf>,
    /// How long to wait for the serve to answer.
    pub wait: Duration,
}

impl DetachOptions {
    /// Start `workdir`'s serve as `roko serve --detach` does without flags.
    #[must_use]
    pub fn new(workdir: impl Into<PathBuf>) -> Self {
        Self {
            workdir: workdir.into(),
            port: None,
            bind: None,
            idle_exit_secs: None,
            enable_terminal: false,
            config: None,
            wait: DEFAULT_DETACH_WAIT,
        }
    }
}

/// The serve `--detach` reports: one it started, or one it found running.
#[derive(Debug, Clone)]
pub struct Detached {
    /// Whether it started the serve now.
    pub started: bool,
    /// The serve.
    pub endpoint: ServeEndpoint,
    /// Where a detached serve writes its output.
    pub log: PathBuf,
}

impl Detached {
    /// The JSON `roko serve --detach --json` prints: the serve's
    /// `serve.json`, with `ok`, `started` and `log`.
    #[must_use]
    pub fn to_json(&self) -> Value {
        let mut value = serde_json::to_value(&self.endpoint).unwrap_or_else(|_| json!({}));
        if let Some(fields) = value.as_object_mut() {
            fields.insert("ok".to_string(), json!(true));
            fields.insert("started".to_string(), json!(self.started));
            fields.insert("log".to_string(), json!(self.log));
        }
        value
    }
}

/// Where a detached serve writes its output: `<workdir>/.roko/logs/serve.log`.
#[must_use]
pub fn serve_log_path(workdir: &Path) -> PathBuf {
    workdir.join(".roko").join("logs").join("serve.log")
}

/// `roko serve --detach`: make sure `options.workdir`'s serve runs, starting
/// it in the background when it does not, and return it once it answers.
/// Idempotent: a serve that runs already is returned with `started: false`.
///
/// The serve is this executable as `roko serve --workdir W --port P`, in its
/// own session, with its working directory in W, stdin from `/dev/null`, its
/// output appended to W's `serve.log`, and the environment without `PORT` or
/// the values this process took from `.env` files (the serve loads its own).
/// It waits up to `options.wait` for `serve.json` to name the child and for
/// `/health` to answer as it; a child that exits first, or is still silent
/// then, fails the start with the log's last lines.
///
/// # Errors
///
/// `not_a_workspace`, `not_initialized` (roko serve refuses W: no provider is
/// configured), `locked` (another roko process holds W), `start_failed`, or
/// `timeout`.
pub fn detach(options: &DetachOptions) -> Result<Detached, LifecycleError> {
    let workdir = canonical_workdir(&options.workdir)?;
    let log = serve_log_path(&workdir);
    let deadline = Instant::now() + options.wait;
    if let Some(running) = wait_for_existing_serve(&workdir, deadline)? {
        return Ok(Detached {
            started: false,
            endpoint: running,
            log,
        });
    }

    let mut port = choose_port(&workdir, options);
    let mut retried_port = false;
    loop {
        let log_start = std::fs::metadata(&log).map_or(0, |meta| meta.len());
        let mut child = spawn_serve(&workdir, port, options, &log).map_err(|error| {
            LifecycleError::new(
                ErrorCode::StartFailed,
                format!(
                    "could not start roko serve for {}: {error}",
                    workdir.display()
                ),
            )
        })?;
        match wait_for_child(&mut child, &workdir, deadline) {
            ChildWait::Answered(served) => {
                return Ok(Detached {
                    started: true,
                    endpoint: *served,
                    log,
                });
            }
            ChildWait::TimedOut => {
                stop_child(&mut child);
                return Err(LifecycleError::new(
                    ErrorCode::Timeout,
                    format!(
                        "roko serve for {} did not answer within {}s; it was stopped",
                        workdir.display(),
                        options.wait.as_secs()
                    ),
                )
                .with_log(&log));
            }
            ChildWait::Exited(status) => {
                let output = read_from(&log, log_start);
                // The configured port was taken between the check and the
                // bind, by another serve starting now: let the OS pick one.
                if options.port.is_none() && !retried_port && port != 0 && bind_failed(&output) {
                    retried_port = true;
                    port = 0;
                    continue;
                }
                return early_exit(&workdir, &log, &output, status, deadline).map(|running| {
                    Detached {
                        started: false,
                        endpoint: running,
                        log,
                    }
                });
            }
        }
    }
}

/// Before a start: the serve that runs in `workdir` already, waiting until
/// `deadline` for one that holds the workspace but does not answer yet. Files
/// a dead serve left are removed. `None` when no serve runs and the
/// workspace is free.
fn wait_for_existing_serve(
    workdir: &Path,
    deadline: Instant,
) -> Result<Option<ServeEndpoint>, LifecycleError> {
    loop {
        let check = check_workspace_serve(workdir);
        match check.state {
            ServeState::Running => return Ok(check.endpoint),
            ServeState::Unresponsive => {
                if Instant::now() >= deadline {
                    let pid = check.endpoint.map_or(0, |served| served.pid);
                    return Err(LifecycleError::new(
                        ErrorCode::Timeout,
                        format!(
                            "roko serve (PID {pid}) holds {} but /health does not answer; see \
                             {}, or stop it with `kill {pid}`",
                            workdir.display(),
                            serve_log_path(workdir).display()
                        ),
                    ));
                }
                std::thread::sleep(POLL);
            }
            ServeState::Stale => {
                if let Some(served) = check.endpoint {
                    clean_stale_serve(workdir, served.pid);
                }
                return free_workspace(workdir, deadline);
            }
            ServeState::Stopped => return free_workspace(workdir, deadline),
        }
    }
}

/// `None` when nothing holds `workdir`'s workspace; the serve when the holder
/// shows itself as a starting serve within [`LOCK_GRACE`]; else `locked`.
fn free_workspace(
    workdir: &Path,
    deadline: Instant,
) -> Result<Option<ServeEndpoint>, LifecycleError> {
    let roko_dir = workdir.join(".roko");
    if !crate::workspace_lock::workspace_lock_is_held(&roko_dir) {
        return Ok(None);
    }
    // A serve between taking the lock and writing serve.json shows itself in
    // a moment.
    if let Some(running) = wait_until_running(workdir, (Instant::now() + LOCK_GRACE).min(deadline))
    {
        return Ok(Some(running));
    }
    let holder = crate::workspace_lock::workspace_lock_owner_pid(&roko_dir)
        .map_or_else(|| "unknown".to_string(), |pid| pid.to_string());
    Err(LifecycleError::new(
        ErrorCode::Locked,
        format!(
            "another roko process (PID {holder}) holds {}, and it is not a roko serve (a plan \
             run?); wait for it to finish, then start roko serve",
            workdir.display()
        ),
    ))
}

/// The serve that runs in `workdir` by `deadline`, looking every [`POLL`].
fn wait_until_running(workdir: &Path, deadline: Instant) -> Option<ServeEndpoint> {
    loop {
        if let Some(running) = running_workspace_serve(workdir) {
            return Some(running);
        }
        if Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(POLL);
    }
}

/// How a started child turned out.
enum ChildWait {
    /// `serve.json` names it, and `/health` answers as it.
    Answered(Box<ServeEndpoint>),
    /// It exited first.
    Exited(std::process::ExitStatus),
    /// The deadline passed.
    TimedOut,
}

/// Wait until `deadline` for `child` to answer as `workdir`'s serve.
fn wait_for_child(child: &mut Child, workdir: &Path, deadline: Instant) -> ChildWait {
    loop {
        if let Ok(Some(status)) = child.try_wait() {
            return ChildWait::Exited(status);
        }
        let check = check_workspace_serve(workdir);
        if check.state == ServeState::Running
            && let Some(served) = check.endpoint
            && served.pid == child.id()
        {
            return ChildWait::Answered(Box::new(served));
        }
        if Instant::now() >= deadline {
            return ChildWait::TimedOut;
        }
        std::thread::sleep(POLL);
    }
}

/// Stop a child that did not answer: SIGTERM, then SIGKILL, and reap it.
fn stop_child(child: &mut Child) {
    let pid = child.id();
    terminate_pid_until(pid, Duration::from_secs(5), || {
        matches!(child.try_wait(), Ok(Some(_)))
    });
    let _ = child.kill();
    let _ = child.wait();
}

/// What a child that exited before it answered means: another serve that
/// started at the same moment runs now (returned), roko serve refuses the
/// workspace (`not_initialized`), another process holds it (`locked`), or the
/// start failed. `output` is what the child wrote to the log.
fn early_exit(
    workdir: &Path,
    log: &Path,
    output: &str,
    status: std::process::ExitStatus,
    deadline: Instant,
) -> Result<ServeEndpoint, LifecycleError> {
    if output.contains(NO_PROVIDER_MESSAGE) {
        return Err(LifecycleError::new(
            ErrorCode::NotInitialized,
            format!(
                "{} is not set up for roko: no LLM provider configured. Run `roko init` there, \
                 or put a provider's API key in ~/.roko/.env",
                workdir.display()
            ),
        )
        .with_log(log));
    }
    if crate::workspace_lock::workspace_lock_is_held(&workdir.join(".roko")) {
        return match free_workspace(workdir, deadline) {
            Ok(Some(running)) => Ok(running),
            Ok(None) => Err(start_failed(workdir, log, status)),
            Err(error) => Err(error.with_log(log)),
        };
    }
    Err(start_failed(workdir, log, status))
}

fn start_failed(workdir: &Path, log: &Path, status: std::process::ExitStatus) -> LifecycleError {
    LifecycleError::new(
        ErrorCode::StartFailed,
        format!(
            "roko serve for {} exited ({status}) before it answered; see {}",
            workdir.display(),
            log.display()
        ),
    )
    .with_log(log)
}

/// Whether a serve's output says its listener could not bind.
fn bind_failed(output: &str) -> bool {
    output.contains("bind to ")
        && (output.contains("in use")
            || output.contains("AddrInUse")
            || output.contains("os error 48"))
}

/// `--port`; else the configured port (`[server] port`, or `[serve] port`
/// while that is the default) when it is free on the bind address; else `0`,
/// so the OS picks one and `serve.json` records it.
fn choose_port(workdir: &Path, options: &DetachOptions) -> u16 {
    if let Some(port) = options.port {
        return port;
    }
    let config = roko_core::config::loader::load_config_unified(workdir).unwrap_or_default();
    let configured = if config.server.port == crate::DEFAULT_SERVE_PORT {
        config.serve.port.unwrap_or(config.server.port)
    } else {
        config.server.port
    };
    let bind = options.bind.as_deref().unwrap_or(&config.server.bind);
    if port_is_free(bind, configured) {
        configured
    } else {
        0
    }
}

/// Whether `port` can be bound on `bind` now.
#[must_use]
pub fn port_is_free(bind: &str, port: u16) -> bool {
    let host = bind
        .strip_prefix('[')
        .and_then(|host| host.strip_suffix(']'))
        .unwrap_or(bind);
    std::net::TcpListener::bind((host, port)).is_ok()
}

/// Start `roko serve` for `workdir` on `port`, detached, with its output
/// appended to `log`.
fn spawn_serve(
    workdir: &Path,
    port: u16,
    options: &DetachOptions,
    log: &Path,
) -> io::Result<Child> {
    let exe = std::env::current_exe().and_then(std::path::absolute)?;
    let mut args: Vec<OsString> = Vec::new();
    if let Some(config) = &options.config {
        args.extend([OsString::from("--config"), config.clone().into_os_string()]);
    }
    args.extend([
        OsString::from("serve"),
        OsString::from("--workdir"),
        workdir.as_os_str().to_os_string(),
        OsString::from("--port"),
        OsString::from(port.to_string()),
    ]);
    if let Some(bind) = &options.bind {
        args.extend([OsString::from("--bind"), OsString::from(bind)]);
    }
    if let Some(secs) = options.idle_exit_secs.filter(|secs| *secs > 0) {
        args.extend([
            OsString::from("--idle-exit"),
            OsString::from(secs.to_string()),
        ]);
    }
    if options.enable_terminal {
        args.push(OsString::from("--enable-terminal"));
    }
    let mut stdout = open_log(log)?;
    writeln!(
        stdout,
        "--- {} roko serve --detach: starting {} for {} on port {port}",
        chrono::Utc::now().to_rfc3339(),
        exe.display(),
        workdir.display()
    )?;
    let stderr = stdout.try_clone()?;
    spawn_detached(&DetachedCommand {
        program: &exe,
        args: &args,
        cwd: workdir,
        stdout,
        stderr,
        env_remove: &detached_env_removals(),
    })
}

/// Open the serve log for appending, owner-only: it holds the portal line
/// with the launch token.
fn open_log(log: &Path) -> io::Result<File> {
    if let Some(parent) = log.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut options = OpenOptions::new();
    options.append(true).create(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    options.open(log)
}

/// The variables a detached serve must not inherit: `PORT`, which an
/// explicit port overrules anyway, and every value this process took from a
/// `.env` file, since the serve loads `~/.roko/.env` and its own workspace's
/// file itself, as a serve started in a terminal there would.
fn detached_env_removals() -> Vec<OsString> {
    let dotenv = roko_core::child_env::startup_dotenv();
    let mut names: Vec<OsString> = std::env::vars_os()
        .map(|(name, _)| name)
        .filter(|name| {
            name.to_str()
                .is_some_and(|name| dotenv.supplied_value(name))
        })
        .collect();
    names.push(OsString::from("PORT"));
    names
}

/// The text written to `log` after byte `start`.
fn read_from(log: &Path, start: u64) -> String {
    let Ok(mut file) = File::open(log) else {
        return String::new();
    };
    let mut bytes = Vec::new();
    if file.seek(SeekFrom::Start(start)).is_err() || file.read_to_end(&mut bytes).is_err() {
        return String::new();
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

/// The last `lines` lines of `path`, with launch tokens hidden.
#[must_use]
pub fn log_tail(path: &Path, lines: usize) -> Vec<String> {
    const TAIL_BYTES: u64 = 64 * 1024;
    let Ok(mut file) = File::open(path) else {
        return Vec::new();
    };
    let len = file.metadata().map_or(0, |meta| meta.len());
    let start = len.saturating_sub(TAIL_BYTES);
    let mut bytes = Vec::new();
    if file.seek(SeekFrom::Start(start)).is_err() || file.read_to_end(&mut bytes).is_err() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&bytes);
    let all: Vec<&str> = text.lines().collect();
    all[all.len().saturating_sub(lines)..]
        .iter()
        .map(|line| hide_launch_token(line))
        .collect()
}

/// `line` with the launch token of a portal URL (`#token=…`) hidden.
fn hide_launch_token(line: &str) -> String {
    match line.find("#token=") {
        Some(at) => {
            let end = line[at..]
                .find(char::is_whitespace)
                .map_or(line.len(), |offset| at + offset);
            format!("{}#token=…{}", &line[..at], &line[end..])
        }
        None => line.to_string(),
    }
}

// ---------------------------------------------------------------------------
// roko serve status
// ---------------------------------------------------------------------------

/// One serve as `roko serve status` reports it.
#[derive(Debug, Clone)]
pub struct ServeStatus {
    /// What the check found.
    pub check: ServeCheck,
    /// Whether the files a dead serve left were removed.
    pub cleaned: bool,
    /// Seconds since the serve started, when it runs.
    pub uptime_secs: Option<u64>,
    /// Runs and plans going, when it runs and says so.
    pub active_runs: Option<u64>,
    /// Whether the binary at its `exe` path is another build now.
    pub outdated: Option<bool>,
}

impl ServeStatus {
    /// Its object in `roko serve status --json`: the `serve.json` fields
    /// (only `workdir` for a stopped serve), plus `state`, `uptime_secs`,
    /// `active_runs` and `outdated`, and `cleaned` for a stale one.
    #[must_use]
    pub fn to_json(&self) -> Value {
        let mut value = self
            .check
            .endpoint
            .as_ref()
            .and_then(|served| serde_json::to_value(served).ok())
            .unwrap_or_else(|| json!({}));
        if let Some(fields) = value.as_object_mut() {
            fields.insert("workdir".to_string(), json!(self.check.workdir));
            fields.insert("state".to_string(), json!(self.check.state.as_str()));
            fields.insert("uptime_secs".to_string(), json!(self.uptime_secs));
            fields.insert("active_runs".to_string(), json!(self.active_runs));
            fields.insert("outdated".to_string(), json!(self.outdated));
            if self.check.state == ServeState::Stale {
                fields.insert("cleaned".to_string(), json!(self.cleaned));
            }
        }
        value
    }
}

/// `roko serve status` for `workdir`: what runs there, with its uptime, its
/// runs and whether its binary was replaced. Files a dead serve left are
/// removed ([`clean_stale_serve`]).
#[must_use]
pub fn status(workdir: &Path) -> ServeStatus {
    let check = check_workspace_serve(workdir);
    let mut status = ServeStatus {
        cleaned: false,
        uptime_secs: None,
        active_runs: None,
        outdated: None,
        check,
    };
    match (status.check.state, status.check.endpoint.as_ref()) {
        (ServeState::Running, Some(served)) => {
            status.uptime_secs = status
                .check
                .health
                .as_ref()
                .and_then(|health| health.get("uptime_secs"))
                .and_then(Value::as_u64);
            status.active_runs = active_runs(workdir, served);
            status.outdated = outdated(served);
        }
        (ServeState::Unresponsive, Some(served)) => status.outdated = outdated(served),
        (ServeState::Stale, Some(served)) => {
            status.cleaned = clean_stale_serve(workdir, served.pid);
            prune_registry_entry(workdir, served.pid);
        }
        _ => {}
    }
    status
}

/// The workspaces of every serve in the machine's registry, pruning the
/// entries of serves that are gone (their process is dead, or their
/// workspace no longer exists) and entries that cannot be read.
#[must_use]
pub fn registry_workdirs() -> Vec<PathBuf> {
    let Some(dir) = endpoint::registry_dir() else {
        return Vec::new();
    };
    let mut workdirs = Vec::new();
    for (path, record) in endpoint::read_registry(&dir) {
        let Some(served) = record else {
            let _ = std::fs::remove_file(&path);
            continue;
        };
        if !served.workdir.is_dir() {
            if !process_is_alive(served.pid) {
                let _ = std::fs::remove_file(&path);
            }
            continue;
        }
        if !workdirs.contains(&served.workdir) {
            workdirs.push(served.workdir);
        }
    }
    workdirs
}

/// Remove `workdir`'s registry entry while it records the dead serve `pid`.
fn prune_registry_entry(workdir: &Path, pid: u32) {
    if let Some(dir) = endpoint::registry_dir() {
        endpoint::remove_registry_entry_if_owned(&dir, workdir, pid);
    }
}

// ---------------------------------------------------------------------------
// roko serve stop / restart
// ---------------------------------------------------------------------------

/// How a stop of one workspace's serve ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopOutcome {
    /// It was stopped.
    Stopped {
        /// The serve's process.
        pid: u32,
    },
    /// No serve ran (files a dead one left were removed).
    NotRunning,
    /// Runs are going (or the serve does not say), and `--force` was not
    /// given: it was left alone.
    Busy {
        /// How many, when known.
        active_runs: Option<u64>,
    },
    /// Its process holds the workspace but does not answer: it was left
    /// alone, since nothing here signals a serve that fails the check.
    Unresponsive {
        /// The serve's process.
        pid: u32,
    },
}

/// Stop `workdir`'s serve: only one that runs ([`check_workspace_serve`]),
/// and a busy one (runs going, or unknown) only with `force`. SIGTERM, a wait
/// of up to `timeout` for it to let go of the workspace, then SIGKILL; what it
/// left behind is removed.
#[must_use]
pub fn stop(workdir: &Path, force: bool, timeout: Duration) -> StopOutcome {
    let check = check_workspace_serve(workdir);
    let Some(served) = check.endpoint else {
        return StopOutcome::NotRunning;
    };
    match check.state {
        ServeState::Stopped => StopOutcome::NotRunning,
        ServeState::Stale => {
            clean_stale_serve(workdir, served.pid);
            prune_registry_entry(workdir, served.pid);
            StopOutcome::NotRunning
        }
        ServeState::Unresponsive => StopOutcome::Unresponsive { pid: served.pid },
        ServeState::Running => {
            if !force {
                let active_runs = active_runs(workdir, &served);
                if active_runs != Some(0) {
                    return StopOutcome::Busy { active_runs };
                }
            }
            let pid = served.pid;
            terminate_pid_until(pid, timeout, || !holds_workspace(workdir, pid));
            // A serve stopped gracefully removed these already; one that was
            // killed did not.
            endpoint::remove_advertisement_if_owned(
                workdir,
                endpoint::registry_dir().as_deref(),
                pid,
            );
            StopOutcome::Stopped { pid }
        }
    }
}

/// How a restart of one workspace's serve ended.
#[derive(Debug, Clone)]
pub enum RestartOutcome {
    /// It was stopped and started again.
    Restarted(Box<Detached>),
    /// It was stopped, but did not start again.
    Failed(LifecycleError),
    /// It was not stopped: no serve ran, it is busy, or it does not answer.
    NotStopped(StopOutcome),
}

/// Restart `workdir`'s serve on this executable: [`stop`] it (same busy
/// rule), then [`detach`] it again with what it was started with — its bind,
/// `--idle-exit`, `--enable-terminal` and `--config`, and its port when that
/// is free again. A workspace whose serve does not run is not started.
#[must_use]
pub fn restart(workdir: &Path, force: bool, wait: Duration) -> RestartOutcome {
    let Some(served) = running_workspace_serve(workdir) else {
        return RestartOutcome::NotStopped(stop(workdir, force, DEFAULT_STOP_TIMEOUT));
    };
    match stop(workdir, force, DEFAULT_STOP_TIMEOUT) {
        StopOutcome::Stopped { .. } => {}
        other => return RestartOutcome::NotStopped(other),
    }
    match detach(&restart_options(workdir, &served, wait)) {
        Ok(detached) => RestartOutcome::Restarted(Box::new(detached)),
        Err(error) => RestartOutcome::Failed(error),
    }
}

/// The JSON of `roko serve stop` over `outcomes`, one per workspace, and its
/// exit status: `{"ok", "stopped": [{workdir, pid}], "not_running": [w],
/// "busy": [{workdir, active_runs}], "unresponsive": [{workdir, pid}]}`. A busy
/// or unresponsive serve is listed either way; for one workspace (`all` is
/// false) it also fails the command (`ok: false`, an `error`, exit 1), while
/// `--all` skips it.
#[must_use]
pub fn stop_report(outcomes: &[(PathBuf, StopOutcome)], all: bool) -> (Value, i32) {
    let mut report = json!({"ok": true, "stopped": [], "not_running": [], "busy": [],
                            "unresponsive": []});
    let mut error = None;
    for (workdir, outcome) in outcomes {
        let (list, entry) = match outcome {
            StopOutcome::Stopped { pid } => ("stopped", json!({"workdir": workdir, "pid": pid})),
            StopOutcome::NotRunning => ("not_running", json!(workdir)),
            StopOutcome::Busy { active_runs } => {
                error.get_or_insert_with(|| busy_error(workdir, *active_runs, "stopping"));
                (
                    "busy",
                    json!({"workdir": workdir, "active_runs": active_runs}),
                )
            }
            StopOutcome::Unresponsive { pid } => {
                error.get_or_insert_with(|| unresponsive_error(workdir, *pid));
                ("unresponsive", json!({"workdir": workdir, "pid": pid}))
            }
        };
        push(&mut report, list, entry);
    }
    finish(report, if all { None } else { error })
}

/// The JSON of `roko serve restart` over `outcomes` and its exit status:
/// `{"ok", "restarted": [<--detach objects>], "busy", "not_running",
/// "unresponsive", "failed": [{workdir, error}]}`, by [`stop_report`]'s rules;
/// a serve that did not start again fails the command, also with `--all`.
#[must_use]
pub fn restart_report(outcomes: &[(PathBuf, RestartOutcome)], all: bool) -> (Value, i32) {
    let mut report = json!({"ok": true, "restarted": [], "not_running": [], "busy": [],
                            "unresponsive": [], "failed": []});
    let mut refused = None;
    let mut failed: Option<LifecycleError> = None;
    for (workdir, outcome) in outcomes {
        let (list, entry) = match outcome {
            RestartOutcome::Restarted(detached) => ("restarted", detached.to_json()),
            RestartOutcome::Failed(error) => {
                failed.get_or_insert_with(|| error.clone());
                (
                    "failed",
                    json!({"workdir": workdir, "error": error.to_json()}),
                )
            }
            RestartOutcome::NotStopped(StopOutcome::Busy { active_runs }) => {
                refused.get_or_insert_with(|| busy_error(workdir, *active_runs, "restarting"));
                (
                    "busy",
                    json!({"workdir": workdir, "active_runs": active_runs}),
                )
            }
            RestartOutcome::NotStopped(StopOutcome::Unresponsive { pid }) => {
                refused.get_or_insert_with(|| unresponsive_error(workdir, *pid));
                ("unresponsive", json!({"workdir": workdir, "pid": pid}))
            }
            RestartOutcome::NotStopped(_) => ("not_running", json!(workdir)),
        };
        push(&mut report, list, entry);
    }
    finish(report, failed.or(if all { None } else { refused }))
}

/// Append `entry` to the array `list` of `report`.
fn push(report: &mut Value, list: &str, entry: Value) {
    if let Some(items) = report.get_mut(list).and_then(Value::as_array_mut) {
        items.push(entry);
    }
}

/// `report` with `ok` and `error` set from `error`, and its exit status.
fn finish(mut report: Value, error: Option<LifecycleError>) -> (Value, i32) {
    match error {
        None => (report, 0),
        Some(error) => {
            report["ok"] = json!(false);
            report["error"] = error.to_json();
            (report, error.code.exit_code())
        }
    }
}

fn busy_error(workdir: &Path, active_runs: Option<u64>, doing: &str) -> LifecycleError {
    let message = match active_runs {
        Some(1) => format!(
            "1 run is going in {}; {doing} roko serve stops it. Pass --force to do it anyway",
            workdir.display()
        ),
        Some(runs) => format!(
            "{runs} runs are going in {}; {doing} roko serve stops them. Pass --force to do it \
             anyway",
            workdir.display()
        ),
        None => format!(
            "roko serve in {} does not say whether runs are going; {doing} it would stop them. \
             Pass --force to do it anyway",
            workdir.display()
        ),
    };
    LifecycleError::new(ErrorCode::Busy, message)
}

fn unresponsive_error(workdir: &Path, pid: u32) -> LifecycleError {
    LifecycleError::new(
        ErrorCode::Unresponsive,
        format!(
            "roko serve (PID {pid}) holds {} but /health does not answer, so it was left alone; \
             see {}, or stop it with `kill {pid}`",
            workdir.display(),
            serve_log_path(workdir).display()
        ),
    )
}

/// How to start `served` again in `workdir`.
fn restart_options(workdir: &Path, served: &ServeEndpoint, wait: Duration) -> DetachOptions {
    let ServeArgs {
        bind,
        enable_terminal,
        config,
        ..
    } = &served.args;
    let bind = (!bind.is_empty()).then(|| bind.clone());
    let free =
        served.port != 0 && port_is_free(bind.as_deref().unwrap_or("127.0.0.1"), served.port);
    DetachOptions {
        workdir: workdir.to_path_buf(),
        port: free.then_some(served.port),
        bind,
        idle_exit_secs: served.idle_exit_secs,
        enable_terminal: *enable_terminal,
        config: config.clone(),
        wait,
    }
}

// ---------------------------------------------------------------------------
// Processes
// ---------------------------------------------------------------------------

/// What [`spawn_detached`] starts.
#[derive(Debug)]
pub struct DetachedCommand<'a> {
    /// The executable.
    pub program: &'a Path,
    /// Its arguments.
    pub args: &'a [OsString],
    /// Its working directory.
    pub cwd: &'a Path,
    /// Where its stdout goes.
    pub stdout: File,
    /// Where its stderr goes.
    pub stderr: File,
    /// Variables it does not inherit.
    pub env_remove: &'a [OsString],
}

/// Start a process that outlives this one: in its own session
/// ([`roko_agent::process::detach_into_new_session`]), so neither this
/// process's exit nor a closed terminal stops it, with stdin from
/// `/dev/null`. The one detached spawn of the CLI (`roko serve --detach`,
/// `roko daemon start`).
///
/// # Errors
///
/// When the process cannot be started.
pub fn spawn_detached(command: &DetachedCommand<'_>) -> io::Result<Child> {
    let mut cmd = Command::new(command.program);
    cmd.args(command.args)
        .current_dir(command.cwd)
        .stdin(Stdio::null())
        .stdout(command.stdout.try_clone()?)
        .stderr(command.stderr.try_clone()?);
    for name in command.env_remove {
        cmd.env_remove(name);
    }
    roko_agent::process::detach_into_new_session(&mut cmd);
    cmd.spawn()
}

/// Whether a process with `pid` exists; one this user may not signal counts.
#[must_use]
#[allow(unsafe_code)]
pub fn process_is_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        let Ok(pid) = libc::pid_t::try_from(pid) else {
            return false;
        };
        if pid <= 0 {
            return false;
        }
        // SAFETY: kill(2) with signal 0 only checks that the process exists.
        let found = unsafe { libc::kill(pid, 0) } == 0;
        found || io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        false
    }
}

/// How [`terminate_pid_until`] ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Termination {
    /// The process was not running.
    NotRunning,
    /// It stopped after SIGTERM.
    Stopped,
    /// It got SIGKILL.
    Killed,
}

/// Stop `pid`: SIGTERM, then wait up to `timeout` for `stopped` to say so,
/// then SIGKILL — only while `pid` is still the process SIGTERM went to (its
/// start time is unchanged), so a recycled PID is never killed. The one
/// terminate-with-timeout of the CLI (`roko serve stop`, `roko dev`).
pub fn terminate_pid_until(
    pid: u32,
    timeout: Duration,
    mut stopped: impl FnMut() -> bool,
) -> Termination {
    if !process_is_alive(pid) {
        return Termination::NotRunning;
    }
    let started = roko_agent::process::process_identity(pid).map(|identity| identity.start);
    send_signal(pid, Signal::Term);
    let deadline = Instant::now() + timeout;
    loop {
        if stopped() {
            return Termination::Stopped;
        }
        if Instant::now() >= deadline {
            break;
        }
        std::thread::sleep(POLL);
    }
    let same_process = match started {
        Some(started) => roko_agent::process::process_identity(pid)
            .is_some_and(|identity| identity.start == started),
        None => process_is_alive(pid),
    };
    if !same_process {
        return Termination::Stopped;
    }
    send_signal(pid, Signal::Kill);
    let deadline = Instant::now() + KILL_WAIT;
    while !stopped() && Instant::now() < deadline {
        std::thread::sleep(POLL);
    }
    Termination::Killed
}

/// [`terminate_pid_until`] the process is gone.
pub fn terminate_pid(pid: u32, timeout: Duration) -> Termination {
    terminate_pid_until(pid, timeout, || !process_is_alive(pid))
}

/// A signal [`terminate_pid_until`] sends.
#[derive(Debug, Clone, Copy)]
enum Signal {
    Term,
    Kill,
}

#[allow(unsafe_code)]
fn send_signal(pid: u32, signal: Signal) {
    #[cfg(unix)]
    {
        let Ok(pid) = libc::pid_t::try_from(pid) else {
            return;
        };
        if pid <= 0 {
            return;
        }
        let signal = match signal {
            Signal::Term => libc::SIGTERM,
            Signal::Kill => libc::SIGKILL,
        };
        // SAFETY: kill(2) sends `signal` to one positive PID; no memory is shared.
        unsafe {
            libc::kill(pid, signal);
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (pid, signal);
    }
}

// ---------------------------------------------------------------------------
// HTTP
// ---------------------------------------------------------------------------

/// `GET {base_url}{path}` with `headers` within `timeout`: the status and the
/// JSON body. On a thread of its own, so it runs the same from synchronous
/// code and from inside the async runtime.
fn get_json(
    base_url: &str,
    path: &str,
    headers: &HeaderMap,
    timeout: Duration,
) -> Option<(u16, Value)> {
    let url = format!("{}{path}", base_url.trim_end_matches('/'));
    let headers = headers.clone();
    std::thread::scope(|scope| {
        scope
            .spawn(move || {
                let client = reqwest::blocking::Client::builder()
                    .timeout(timeout)
                    .connect_timeout(timeout)
                    .no_proxy()
                    .build()
                    .ok()?;
                let response = client.get(&url).headers(headers).send().ok()?;
                let status = response.status().as_u16();
                let body = response.json::<Value>().ok()?;
                Some((status, body))
            })
            .join()
            .ok()
            .flatten()
    })
}

// ---------------------------------------------------------------------------
// Test support
// ---------------------------------------------------------------------------

/// A stand-in for a serve's `/health`, for the tests of the code that checks
/// one.
#[cfg(test)]
pub(crate) mod test_support {
    use std::io::{BufRead as _, BufReader, Write as _};
    use std::net::TcpListener;

    /// Answers `GET /health` with `{"status": "ok", "instance_id": …}` on a
    /// loopback port until it drops.
    pub(crate) struct FakeHealth {
        port: u16,
    }

    impl FakeHealth {
        /// Answer as the serve `instance_id`.
        pub(crate) fn answering(instance_id: &str) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind a loopback port");
            let port = listener.local_addr().expect("address").port();
            let body = serde_json::json!({"status": "ok", "instance_id": instance_id}).to_string();
            std::thread::spawn(move || {
                for stream in listener.incoming() {
                    let Ok(mut stream) = stream else { continue };
                    let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
                    let mut line = String::new();
                    while reader.read_line(&mut line).is_ok_and(|read| read > 2) {
                        line.clear();
                    }
                    let _ = write!(
                        stream,
                        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: \
                         {}\r\nconnection: close\r\n\r\n{body}",
                        body.len()
                    );
                }
            });
            Self { port }
        }

        /// Its base URL.
        pub(crate) fn url(&self) -> String {
            format!("http://127.0.0.1:{}", self.port)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::FakeHealth;
    use super::*;

    fn endpoint_for(workdir: &Path, pid: u32, url: &str, instance: &str) -> ServeEndpoint {
        ServeEndpoint {
            pid,
            url: url.to_string(),
            workdir: workdir.to_path_buf(),
            started_at: "2026-10-19T10:00:00+00:00".to_string(),
            instance_id: instance.to_string(),
            ..Default::default()
        }
    }

    /// gap-fb1594: a serve runs when its PID is alive and holds the lock and
    /// `/health` answers as it; a dead PID is stale, a holder whose `/health`
    /// answers as another instance is unresponsive.
    #[test]
    fn check_tells_running_stale_and_unresponsive_apart() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(check_workspace_serve(dir.path()).state, ServeState::Stopped);

        let health = FakeHealth::answering("me");
        let served = endpoint_for(dir.path(), u32::MAX - 1, &health.url(), "me");
        endpoint::write_endpoint(dir.path(), &served).unwrap();
        assert_eq!(check_workspace_serve(dir.path()).state, ServeState::Stale);

        let lock =
            crate::workspace_lock::acquire_workspace_lock(&dir.path().join(".roko")).unwrap();
        let served = endpoint_for(dir.path(), std::process::id(), &health.url(), "me");
        endpoint::write_endpoint(dir.path(), &served).unwrap();
        let check = check_workspace_serve(dir.path());
        assert_eq!(check.state, ServeState::Running);
        assert_eq!(check.endpoint, Some(served.clone()));
        assert_eq!(running_workspace_serve(dir.path()), Some(served));

        let other = endpoint_for(dir.path(), std::process::id(), &health.url(), "not-me");
        endpoint::write_endpoint(dir.path(), &other).unwrap();
        assert_eq!(
            check_workspace_serve(dir.path()).state,
            ServeState::Unresponsive
        );
        drop(lock);
        assert_eq!(
            check_workspace_serve(dir.path()).state,
            ServeState::Stale,
            "a live PID that does not hold the workspace is not its serve"
        );
    }

    /// gap-fb1594: an older serve records no instance id; its PID, the lock
    /// and an answer from `/health` are enough.
    #[test]
    fn a_serve_json_without_instance_id_counts_by_pid_lock_and_health() {
        let dir = tempfile::tempdir().unwrap();
        let _lock =
            crate::workspace_lock::acquire_workspace_lock(&dir.path().join(".roko")).unwrap();
        let health = FakeHealth::answering("whatever");
        let served = endpoint_for(dir.path(), std::process::id(), &health.url(), "");
        endpoint::write_endpoint(dir.path(), &served).unwrap();
        assert_eq!(check_workspace_serve(dir.path()).state, ServeState::Running);
    }

    /// gap-fb1594: status removes what a dead serve left, and says so; it
    /// leaves the files alone while another process holds the workspace.
    #[test]
    fn status_cleans_up_after_a_dead_serve() {
        let dir = tempfile::tempdir().unwrap();
        let served = endpoint_for(dir.path(), u32::MAX - 1, "http://127.0.0.1:9", "dead");
        endpoint::write_endpoint(dir.path(), &served).unwrap();
        endpoint::write_token(dir.path(), "token").unwrap();

        let holder =
            crate::workspace_lock::acquire_workspace_lock(&dir.path().join(".roko")).unwrap();
        let held = status(dir.path());
        assert_eq!(held.check.state, ServeState::Stale);
        assert!(!held.cleaned, "another process holds the workspace");
        assert!(endpoint::read_endpoint(dir.path()).is_some());
        drop(holder);

        let report = status(dir.path());
        assert_eq!(report.check.state, ServeState::Stale);
        assert!(report.cleaned);
        assert!(endpoint::read_endpoint(dir.path()).is_none());
        assert!(!endpoint::token_path(dir.path()).exists());
        let json = report.to_json();
        assert_eq!(json["state"], "stale");
        assert_eq!(json["cleaned"], true);
        assert_eq!(json["pid"], u32::MAX - 1);
        assert_eq!(json["active_runs"], Value::Null);

        let stopped = status(dir.path()).to_json();
        assert_eq!(stopped["state"], "stopped");
        assert_eq!(stopped["workdir"], json!(dir.path()));
    }

    /// gap-fb1594: stop leaves a serve that does not answer as itself alone,
    /// and reports a workspace without a serve as not running.
    #[test]
    fn stop_never_signals_a_serve_that_fails_the_check() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            stop(dir.path(), true, Duration::from_secs(1)),
            StopOutcome::NotRunning
        );
        let _lock =
            crate::workspace_lock::acquire_workspace_lock(&dir.path().join(".roko")).unwrap();
        let health = FakeHealth::answering("another");
        let served = endpoint_for(dir.path(), std::process::id(), &health.url(), "me");
        endpoint::write_endpoint(dir.path(), &served).unwrap();
        assert_eq!(
            stop(dir.path(), true, Duration::from_secs(1)),
            StopOutcome::Unresponsive {
                pid: std::process::id()
            }
        );
    }

    /// gap-fb1594: a busy serve is one with runs going, or one that does not
    /// say; without `--force` stop leaves it alone.
    #[test]
    fn stop_refuses_a_serve_whose_runs_are_unknown() {
        let dir = tempfile::tempdir().unwrap();
        let _lock =
            crate::workspace_lock::acquire_workspace_lock(&dir.path().join(".roko")).unwrap();
        // The fake answers /api/health without the run counts.
        let health = FakeHealth::answering("me");
        let served = endpoint_for(dir.path(), std::process::id(), &health.url(), "me");
        endpoint::write_endpoint(dir.path(), &served).unwrap();
        assert_eq!(active_runs(dir.path(), &served), None);
        assert_eq!(
            stop(dir.path(), false, Duration::from_secs(1)),
            StopOutcome::Busy { active_runs: None }
        );
        assert!(
            process_is_alive(std::process::id()),
            "nothing was signalled"
        );
    }

    #[test]
    fn version_line_names_the_commit() {
        assert_eq!(
            parse_git_sha(
                "roko 0.1.0 (rustc 1.91.0 (f8297e351 2025-10-28), aarch64-apple-darwin, git \
                 e31f4d52c)\n"
            ),
            Some("e31f4d52c".to_string())
        );
        assert_eq!(parse_git_sha("roko 0.1.0 (rustc, x, git unknown)"), None);
        assert_eq!(parse_git_sha("roko 0.1.0"), None);
    }

    #[test]
    fn log_tail_hides_launch_tokens_and_keeps_the_last_lines() {
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("serve.log");
        let mut text = String::new();
        for line in 0..30 {
            text.push_str(&format!("line {line}\n"));
        }
        text.push_str("portal: http://127.0.0.1:6801/#token=abc123 (open it)\n");
        std::fs::write(&log, text).unwrap();
        let tail = log_tail(&log, LOG_TAIL_LINES);
        assert_eq!(tail.len(), LOG_TAIL_LINES);
        assert_eq!(tail[0], "line 11");
        assert_eq!(
            tail.last().unwrap(),
            "portal: http://127.0.0.1:6801/#token=… (open it)"
        );
        assert!(log_tail(&dir.path().join("missing.log"), 5).is_empty());
    }

    #[test]
    fn a_bind_failure_is_told_apart_from_other_exits() {
        assert!(bind_failed(
            "error: bind to 127.0.0.1:6801: Address already in use (os error 48)"
        ));
        assert!(!bind_failed("error: no LLM provider configured."));
    }

    #[test]
    fn a_taken_port_is_not_free() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        assert!(!port_is_free("127.0.0.1", port));
        drop(listener);
    }

    /// gap-fb1594: a restart starts the serve with what it was started with,
    /// on its old port while that is free.
    #[test]
    fn restart_keeps_what_the_serve_was_started_with() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let taken = listener.local_addr().unwrap().port();
        let mut served = endpoint_for(Path::new("/w"), 1, "http://127.0.0.1:1", "x");
        served.port = taken;
        served.idle_exit_secs = Some(3600);
        served.args = ServeArgs {
            bind: "127.0.0.1".to_string(),
            port_requested: Some(0),
            enable_terminal: true,
            config: Some(PathBuf::from("/w/roko.toml")),
        };
        let options = restart_options(Path::new("/w"), &served, Duration::from_secs(9));
        assert_eq!(options.port, None, "the old port is taken");
        assert_eq!(options.bind.as_deref(), Some("127.0.0.1"));
        assert_eq!(options.idle_exit_secs, Some(3600));
        assert!(options.enable_terminal);
        assert_eq!(options.config, Some(PathBuf::from("/w/roko.toml")));
        assert_eq!(options.wait, Duration::from_secs(9));
        drop(listener);
        let options = restart_options(Path::new("/w"), &served, Duration::from_secs(9));
        assert_eq!(options.port, Some(taken), "the old port is free again");
    }

    #[test]
    fn terminate_reports_a_process_that_is_not_running() {
        assert_eq!(
            terminate_pid(u32::MAX - 1, Duration::from_millis(10)),
            Termination::NotRunning
        );
        assert!(process_is_alive(std::process::id()));
        assert!(!process_is_alive(0));
    }

    /// gap-fb1594: SIGTERM first; a process that stops on it is never killed.
    #[cfg(unix)]
    #[test]
    fn terminate_stops_a_child_with_sigterm() {
        let mut child = Command::new("sleep").arg("30").spawn().unwrap();
        let pid = child.id();
        let ended = terminate_pid_until(pid, Duration::from_secs(5), || {
            matches!(child.try_wait(), Ok(Some(_)))
        });
        assert_eq!(ended, Termination::Stopped);
        use std::os::unix::process::ExitStatusExt as _;
        assert_eq!(child.wait().unwrap().signal(), Some(libc::SIGTERM));
    }
}
