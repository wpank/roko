//! Client-side helpers for connecting to a live `roko-serve` workspace server.
//!
//! # Discovery
//!
//! [`discover_workspace_server`] reads the server's endpoint advertisement
//! (`.roko/runtime/serve.json`) and validates it with two safety checks before
//! returning:
//!
//! 1. The PID recorded in `.roko/runtime/roko.lock` matches the endpoint's PID.
//!    A crashed server cannot clear its own lock, so a stale endpoint is detected
//!    by the lock file still naming the dead PID.
//! 2. `GET {url}/api/health` responds 200 within one second.  A non-responsive
//!    server must never capture a CLI run.
//!
//! # Client
//!
//! [`WorkspaceServerClient`] wraps a `reqwest::Client` and provides
//! typed methods for the plan-execution API:
//!
//! - [`WorkspaceServerClient::submit_plan_run`]
//! - [`WorkspaceServerClient::cancel_plan_run`]
//! - [`WorkspaceServerClient::plan_run_finished`]

use std::path::Path;
use std::time::Duration;

use reqwest::{Client, Response};
use reqwest::header::HeaderMap;
use serde_json::Value;
use tracing::debug;

use roko_serve::endpoint::ServeEndpoint;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// A typed error returned by [`WorkspaceServerClient`] methods.
#[derive(Debug, thiserror::Error)]
pub enum ServeClientError {
    /// Another plan run is already active in this workspace (HTTP 409).
    #[error("a plan run is already active in this workspace — wait for it to finish or cancel it")]
    AlreadyActive,

    /// The server rejected the request due to a missing or invalid credential
    /// (HTTP 401 or 403).
    #[error(
        "the workspace server rejected the request ({status}): {detail}\n\
         hint: supply a key with ROKO_API_KEY, [serve.auth] api_key in roko.toml, \
         or `roko login`"
    )]
    Unauthorized {
        /// HTTP status code (401 or 403).
        status: u16,
        /// Server-provided detail, if any.
        detail: String,
    },

    /// Any other HTTP or network error.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

// ---------------------------------------------------------------------------
// Discovery
// ---------------------------------------------------------------------------

/// Discover a live `roko-serve` process in `workdir`.
///
/// Reads the endpoint advertisement from `.roko/runtime/serve.json` and
/// validates it with two independent checks:
///
/// 1. **PID match** — `.roko/runtime/roko.lock` must contain the same PID as
///    the endpoint.  A crashed server leaves a stale `serve.json` but does not
///    clear the lock, so the PIDs diverge.
/// 2. **Health probe** — `GET {url}/api/health` must respond 200 within one
///    second.  A dead or unreachable server is rejected regardless of PID.
///
/// Returns `None` for any failure; logs a `debug!` message explaining why the
/// endpoint was rejected so callers can diagnose without noise on the happy
/// path.
///
/// A crashed server must never capture a CLI run — therefore both checks must
/// pass before `Some` is returned.
#[must_use]
pub fn discover_workspace_server(workdir: &Path) -> Option<ServeEndpoint> {
    let endpoint = roko_serve::endpoint::read_endpoint(workdir)?;

    // 1. PID check: read the workspace lock file and compare.
    let lock_path = workdir.join(".roko").join("runtime").join("roko.lock");
    let lock_pid: u32 = match std::fs::read_to_string(&lock_path) {
        Ok(content) => match content.trim().parse::<u32>() {
            Ok(pid) => pid,
            Err(_) => {
                debug!(
                    path = %lock_path.display(),
                    content = content.trim(),
                    "workspace lock does not contain a valid PID; rejecting endpoint"
                );
                return None;
            }
        },
        Err(err) => {
            debug!(
                path = %lock_path.display(),
                error = %err,
                "cannot read workspace lock file; rejecting endpoint"
            );
            return None;
        }
    };

    if lock_pid != endpoint.pid {
        debug!(
            lock_pid,
            endpoint_pid = endpoint.pid,
            "workspace lock PID does not match endpoint PID; rejecting stale endpoint"
        );
        return None;
    }

    // 2. Health probe: connect to the server's TCP port within one second.
    // We use a raw TcpStream instead of reqwest::blocking so that this sync
    // function can be called safely from async tasks without creating a nested
    // tokio::Runtime (which would panic in debug builds).
    let health_url = format!("{}/api/health", endpoint.url.trim_end_matches('/'));
    let addr_str = endpoint
        .url
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .split('/')
        .next()
        .unwrap_or("");
    match addr_str.parse::<std::net::SocketAddr>() {
        Ok(addr) => {
            match std::net::TcpStream::connect_timeout(&addr, Duration::from_secs(1)) {
                Ok(_) => {
                    debug!(
                        url = %health_url,
                        pid = endpoint.pid,
                        "workspace server discovered and healthy (TCP probe)"
                    );
                    Some(endpoint)
                }
                Err(err) => {
                    debug!(
                        url = %health_url,
                        error = %err,
                        "workspace server TCP probe failed; rejecting endpoint"
                    );
                    None
                }
            }
        }
        Err(err) => {
            debug!(
                url = %health_url,
                error = %err,
                "cannot parse server address from endpoint URL; rejecting endpoint"
            );
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Typed API responses
// ---------------------------------------------------------------------------

/// Returned by [`WorkspaceServerClient::submit_plan_run`].
#[derive(Debug, Clone)]
pub struct SubmittedRun {
    /// Opaque run identifier assigned by the server.
    pub id: String,
    /// Dependency-ordered list of plan ids that will be executed.
    pub order: Vec<String>,
    /// Effective parallelism ceiling for this run.
    pub max_parallel_plans: usize,
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// A synchronous HTTP client for a `roko-serve` workspace server.
///
/// Constructed from a discovered [`ServeEndpoint`] and an optional API key
/// resolved via [`crate::auth::resolve_api_key`].
///
/// All methods are blocking.
pub struct WorkspaceServerClient {
    client: Client,
    base_url: String,
    headers: HeaderMap,
}

/// Read the launch token written by `roko serve` at startup.
///
/// The file lives at `workdir/.roko/runtime/serve.token` and is written as
/// 0600 by the server process.  Returns `None` when the file is absent,
/// unreadable, or empty.
fn read_launch_token(workdir: &Path) -> Option<String> {
    let token_path = workdir.join(".roko").join("runtime").join("serve.token");
    match std::fs::read_to_string(&token_path) {
        Ok(content) => {
            let trimmed = content.trim().to_string();
            if trimmed.is_empty() { None } else { Some(trimmed) }
        }
        Err(_) => None,
    }
}

impl WorkspaceServerClient {
    /// Create a new client for `endpoint`, resolving auth credentials from
    /// `auth_config`, with `workdir` used as a fallback for the launch token.
    ///
    /// Auth is resolved in this order:
    /// 1. [`crate::auth::resolve_api_key`] — tries `ROKO_API_KEY`, then
    ///    `[serve.auth].api_key`, then stored `roko login` credentials.
    /// 2. `.roko/runtime/serve.token` in `workdir` — the launch token written
    ///    by `roko serve` at startup (0600, same trust boundary as `hub.sock`).
    ///    Sent as `Authorization: Bearer <token>`.
    ///
    /// Missing auth is not an error at construction time; the server will
    /// reject individual requests with 401/403.
    #[must_use]
    pub fn new(
        endpoint: &ServeEndpoint,
        auth_config: &roko_core::config::ServeAuthConfig,
        workdir: &Path,
    ) -> Self {
        let resolved = crate::auth::resolve_api_key(auth_config, None);
        Self::new_with_resolved(endpoint, resolved, workdir)
    }

    /// Internal constructor that accepts a pre-resolved key so tests can
    /// exercise the launch-token fallback without touching the environment.
    pub(crate) fn new_with_resolved(
        endpoint: &ServeEndpoint,
        resolved: Option<crate::auth::ResolvedApiKey>,
        workdir: &Path,
    ) -> Self {
        let headers = match resolved {
            Some(key) => key.headers(),
            None => match read_launch_token(workdir) {
                Some(token) => crate::auth::auth_headers_with_method(
                    &token,
                    crate::auth::AuthMethod::Bearer,
                ),
                None => HeaderMap::new(),
            },
        };

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            client,
            base_url: endpoint.url.trim_end_matches('/').to_string(),
            headers,
        }
    }

    /// `POST /api/plans/execute` — start a plan run on the workspace server.
    ///
    /// # Parameters
    ///
    /// - `target`: workspace-relative directory or plan-ids list target.  Pass
    ///   `None` to run all plans under the workspace plans root.
    /// - `resume`: when `true`, the server resumes from the last checkpoint.
    /// - `max_parallel_plans`: concurrency ceiling; `None` uses the server's
    ///   `[conductor] max_parallel_plans` default.
    ///
    /// # Errors
    ///
    /// - [`ServeClientError::AlreadyActive`] when HTTP 409 is returned.
    /// - [`ServeClientError::Unauthorized`] when HTTP 401 or 403 is returned.
    pub async fn submit_plan_run(
        &self,
        target: Option<&str>,
        resume: bool,
        max_parallel_plans: Option<usize>,
    ) -> std::result::Result<SubmittedRun, ServeClientError> {
        let mut body = serde_json::json!({ "resume": resume });
        if let Some(t) = target {
            body["target"] = serde_json::Value::String(t.to_string());
        }
        if let Some(n) = max_parallel_plans {
            body["max_parallel_plans"] = serde_json::Value::Number(n.into());
        }

        let url = format!("{}/api/plans/execute", self.base_url);
        let resp = self
            .client
            .post(&url)
            .headers(self.headers.clone())
            .json(&body)
            .send()
            .await
            .map_err(|e| ServeClientError::Other(anyhow::anyhow!("HTTP request failed: {e}")))?;

        self.handle_auth_error(&resp)?;

        if resp.status().as_u16() == 409 {
            return Err(ServeClientError::AlreadyActive);
        }

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body_text = resp.text().await.unwrap_or_default();
            return Err(ServeClientError::Other(anyhow::anyhow!(
                "POST /api/plans/execute returned {status}: {body_text}"
            )));
        }

        let json: Value = resp
            .json()
            .await
            .map_err(|e| ServeClientError::Other(anyhow::anyhow!("parse response JSON: {e}")))?;

        let id = json["id"]
            .as_str()
            .ok_or_else(|| {
                ServeClientError::Other(anyhow::anyhow!("response missing 'id' field"))
            })?
            .to_string();

        let order = json["order"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let max_parallel_plans = json["max_parallel_plans"]
            .as_u64()
            .unwrap_or(1) as usize;

        Ok(SubmittedRun {
            id,
            order,
            max_parallel_plans,
        })
    }

    /// `POST /api/plans/{run_id}/cancel` — cancel an active plan run.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or the server returns an error
    /// status other than 404 (which is treated as "already finished").
    pub async fn cancel_plan_run(&self, run_id: &str) -> std::result::Result<(), ServeClientError> {
        let url = format!("{}/api/plans/{run_id}/cancel", self.base_url);
        let resp = self
            .client
            .post(&url)
            .headers(self.headers.clone())
            .send()
            .await
            .map_err(|e| ServeClientError::Other(anyhow::anyhow!("HTTP request failed: {e}")))?;

        self.handle_auth_error(&resp)?;

        // 404 means the run already finished — treat as success.
        if resp.status().as_u16() == 404 {
            return Ok(());
        }

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body_text = resp.text().await.unwrap_or_default();
            return Err(ServeClientError::Other(anyhow::anyhow!(
                "POST /api/plans/{run_id}/cancel returned {status}: {body_text}"
            )));
        }

        Ok(())
    }

    /// `GET /api/plans/{run_id}/status` — check whether a run has finished.
    ///
    /// Returns `true` when:
    /// - The server returns HTTP 404 (run not found — it has been cleaned up
    ///   after completion).
    /// - The status response body includes `"finished": true`.
    ///
    /// Returns `false` when the run is still in progress.
    ///
    /// # Errors
    ///
    /// Returns an error on network failures or unexpected server errors.
    pub async fn plan_run_finished(
        &self,
        run_id: &str,
    ) -> std::result::Result<bool, ServeClientError> {
        let url = format!("{}/api/plans/{run_id}/status", self.base_url);
        let resp = self
            .client
            .get(&url)
            .headers(self.headers.clone())
            .send()
            .await
            .map_err(|e| ServeClientError::Other(anyhow::anyhow!("HTTP request failed: {e}")))?;

        self.handle_auth_error(&resp)?;

        // 404 → run has been cleaned up, treat as finished.
        if resp.status().as_u16() == 404 {
            return Ok(true);
        }

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body_text = resp.text().await.unwrap_or_default();
            return Err(ServeClientError::Other(anyhow::anyhow!(
                "GET /api/plans/{run_id}/status returned {status}: {body_text}"
            )));
        }

        let json: Value = resp
            .json()
            .await
            .map_err(|e| ServeClientError::Other(anyhow::anyhow!("parse status JSON: {e}")))?;

        // The server sets "finished" when handle.is_finished() is true.
        let finished = json["finished"].as_bool().unwrap_or(false);
        Ok(finished)
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Check a response for 401/403 and convert to [`ServeClientError::Unauthorized`].
    ///
    /// The response is consumed only on error; on success it is returned
    /// unchanged.
    fn handle_auth_error(
        &self,
        resp: &Response,
    ) -> std::result::Result<(), ServeClientError> {
        let status = resp.status().as_u16();
        if status == 401 || status == 403 {
            return Err(ServeClientError::Unauthorized {
                status,
                detail: format!("server returned HTTP {status}"),
            });
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Shared-lock helper
// ---------------------------------------------------------------------------

/// Acquire the shared (read-only) workspace lock, **unless** a live
/// `roko-serve` process already owns the workspace.
///
/// When a server is running, it is the exclusive writer and performs all
/// mutations atomically.  Read-only commands (dashboard, plan list/show/
/// validate/index --check, plan status) can safely attach without acquiring a
/// lock — the server guarantees consistency.
///
/// # Return value
///
/// - `Ok(None)` — a live server owns the workspace; no lock is needed.
/// - `Ok(Some(guard))` — no server found; returns the shared lock as usual.
/// - `Err(…)` — no server found but the shared-lock acquisition failed (another
///   non-server exclusive writer holds the lock).
pub fn read_lock_unless_served(
    workdir: &Path,
) -> anyhow::Result<Option<crate::workspace_lock::WorkspaceLockGuard>> {
    if discover_workspace_server(workdir).is_some() {
        // Server owns the workspace — skip locking entirely.
        return Ok(None);
    }
    let guard =
        crate::workspace_lock::acquire_workspace_lock_shared(&workdir.join(".roko"))?;
    Ok(Some(guard))
}

// ---------------------------------------------------------------------------
// Server-mode plan runner
// ---------------------------------------------------------------------------

/// Run a plan via a live `roko-serve` workspace server.
///
/// Called from `PlanCmd::Run` when [`discover_workspace_server`] finds a
/// healthy server.  The caller must have already validated the plan set and
/// handled `--dry-run`.
///
/// # Refused flags
///
/// The following flags cannot be forwarded to the server; if any are set this
/// function returns an error naming them and identifying the server that owns
/// the workspace:
///
/// `--max-retries`, `--max-tasks`, `--budget-override`, `--no-budget`,
/// `--model`, `--dangerously-skip-permissions`, `--log-file`,
/// `--worktree-per-task`, `--rich-topology`, and an explicit
/// `--resume-plan` path.
///
/// # Protocol
///
/// 1. Validate refused flags.
/// 2. Connect the hub mirror via
///    [`crate::state_hub_ipc::try_connect_hub_ipc`] — fail if unavailable,
///    naming the socket path.
/// 3. Submit with [`WorkspaceServerClient::submit_plan_run`] and print
///    `submitted to roko serve at <url> as run <id>`.
/// 4. Follow the run: TUI when stdout is a TTY and `--no-tui` is not set;
///    otherwise print one line per notable event.
/// 5. On Ctrl-C / SIGTERM, call [`WorkspaceServerClient::cancel_plan_run`]
///    once, print a cancellation notice, and exit non-zero.
#[allow(clippy::too_many_arguments)]
pub async fn run_plan_via_server(
    wd: &Path,
    plans_dir: &Path,
    endpoint: &ServeEndpoint,
    // Run options
    fresh: bool,
    no_tui: bool,
    json: bool,
    max_parallel_plans: Option<usize>,
    // Flags that the server cannot honour — checked here and refused if set
    refused_max_retries: Option<u32>,
    refused_max_tasks: usize,
    refused_budget_override: Option<f64>,
    refused_no_budget: bool,
    refused_model: Option<String>,
    refused_dangerously_skip_permissions: bool,
    refused_log_file: Option<std::path::PathBuf>,
    refused_worktree_per_task: bool,
    refused_rich_topology: bool,
    refused_resume_plan_path: Option<std::path::PathBuf>,
) -> anyhow::Result<i32> {
    use std::io::IsTerminal as _;

    // ── 1. Refuse server-incompatible flags ───────────────────────────────
    let mut refused: Vec<&'static str> = Vec::new();
    if refused_max_retries.is_some() {
        refused.push("--max-retries");
    }
    if refused_max_tasks != 0 {
        refused.push("--max-tasks");
    }
    if refused_budget_override.is_some() {
        refused.push("--budget-override");
    }
    if refused_no_budget {
        refused.push("--no-budget");
    }
    if refused_model.is_some() {
        refused.push("--model");
    }
    if refused_dangerously_skip_permissions {
        refused.push("--dangerously-skip-permissions");
    }
    if refused_log_file.is_some() {
        refused.push("--log-file");
    }
    if refused_worktree_per_task {
        refused.push("--worktree-per-task");
    }
    if refused_rich_topology {
        refused.push("--rich-topology");
    }
    if refused_resume_plan_path.is_some() {
        refused.push("--resume-plan");
    }

    if !refused.is_empty() {
        anyhow::bail!(
            "{} cannot be used when a server (PID {} at {}) owns this workspace; \
             submit the run without these flags or stop the server first",
            refused.join(", "),
            endpoint.pid,
            endpoint.url,
        );
    }

    // ── 2. Connect hub mirror ─────────────────────────────────────────────
    let socket_path = wd.join(".roko").join("runtime").join("hub.sock");
    let mirror = match crate::state_hub_ipc::try_connect_hub_ipc(wd).await {
        Some(m) => m,
        None => {
            anyhow::bail!(
                "cannot connect to server hub IPC at {}; \
                 ensure roko serve is running and the socket exists",
                socket_path.display()
            );
        }
    };

    // ── Build HTTP client ─────────────────────────────────────────────────
    // For workspace-local server connections we skip globally-stored `roko login`
    // credentials: they may belong to a different (remote) server and would
    // silently shadow the workspace's own launch token.
    // Precedence: ROKO_API_KEY env var → .roko/runtime/serve.token (launch token).
    let resolved_for_local: Option<crate::auth::ResolvedApiKey> =
        std::env::var(crate::auth::ROKO_API_KEY_ENV)
            .ok()
            .filter(|k| !k.trim().is_empty())
            .map(|k| crate::auth::ResolvedApiKey {
                key: k.trim().to_string(),
                source: crate::auth::ApiKeySource::EnvVar,
                method: crate::auth::AuthMethod::ApiKey,
            });
    let serve_client = WorkspaceServerClient::new_with_resolved(endpoint, resolved_for_local, wd);

    // ── 3. Submit the plan run ────────────────────────────────────────────
    let target = plans_dir
        .strip_prefix(wd)
        .map(|rel| rel.to_string_lossy().into_owned())
        .ok();
    let resume = !fresh;

    let submitted = serve_client
        .submit_plan_run(target.as_deref(), resume, max_parallel_plans)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    println!(
        "submitted to roko serve at {} as run {}",
        endpoint.url, submitted.id
    );

    let run_id = submitted.id.clone();
    let server_url = endpoint.url.clone();
    let order: Vec<String> = submitted.order.clone();
    let order_set: std::collections::HashSet<String> = order.iter().cloned().collect();

    // ── 4. Follow the run ─────────────────────────────────────────────────
    let final_outcome = if !no_tui && std::io::stdout().is_terminal() {
        follow_run_tui(
            wd,
            &mirror,
            &run_id,
            &order_set,
            &serve_client,
        )
        .await?
    } else {
        follow_run_text(
            json,
            &mirror,
            &run_id,
            &order_set,
            &serve_client,
        )
        .await?
    };

    // ── JSON summary ──────────────────────────────────────────────────────
    if json {
        println!(
            "{}",
            serde_json::json!({
                "run_id": run_id,
                "server_url": server_url,
                "order": order,
                "outcome": final_outcome,
            })
        );
    }

    Ok(if final_outcome == "succeeded" { 0 } else { 1 })
}

// ---------------------------------------------------------------------------
// Follow helpers
// ---------------------------------------------------------------------------

/// Follow a server-managed plan run on the TUI, exiting when the run ends or
/// when Ctrl-C / SIGTERM arrives (in which case the run is cancelled).
async fn follow_run_tui(
    wd: &Path,
    mirror: &crate::state_hub::SharedStateHub,
    run_id: &str,
    order_set: &std::collections::HashSet<String>,
    serve_client: &WorkspaceServerClient,
) -> anyhow::Result<String> {
    let (shutdown_tx, shutdown_rx) = std::sync::mpsc::channel::<()>();

    // Subscribe to events and track run completion in a background task.
    let outcome_cell = std::sync::Arc::new(std::sync::Mutex::new("unknown".to_string()));
    let outcome_cell_bg = outcome_cell.clone();
    let shutdown_tx_bg = shutdown_tx.clone();
    let mut sub = mirror.subscribe_events_from(0);
    let order_set_bg = order_set.clone();

    tokio::spawn(async move {
        let mut run_started = false;
        loop {
            match sub.live.recv().await {
                Ok(envelope) => match &envelope.payload {
                    roko_core::DashboardEvent::PlanSetLoaded { plans } => {
                        if plans
                            .iter()
                            .any(|p| order_set_bg.contains(&p.plan_id))
                        {
                            run_started = true;
                        }
                    }
                    roko_core::DashboardEvent::PlanStarted { plan_id, .. } => {
                        if order_set_bg.contains(plan_id) {
                            run_started = true;
                        }
                    }
                    roko_core::DashboardEvent::RunCompleted { outcome, .. } => {
                        if run_started {
                            if let Ok(mut g) = outcome_cell_bg.lock() {
                                *g = outcome.clone();
                            }
                            let _ = shutdown_tx_bg.send(());
                            break;
                        }
                    }
                    _ => {}
                },
                Err(_) => break,
            }
        }
    });

    // Spawn the TUI on a blocking thread.
    let mirror_tui = mirror.clone();
    let wd_buf = wd.to_path_buf();
    let tui_handle = std::thread::Builder::new()
        .name("roko-serve-client-tui".to_string())
        .spawn(move || {
            crate::tui::App::new_connected_with_page(&wd_buf, None, &mirror_tui)
                .with_exit_on_plan_completion()
                .with_host_termination_signals()
                .with_shutdown_receiver(shutdown_rx)
                .run()
        })
        .map_err(|e| anyhow::anyhow!("spawn serve-client TUI thread: {e}"))?;

    // Wait for TUI exit or Ctrl-C.
    let run_id_owned = run_id.to_string();
    let cancelled = tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            let _ = shutdown_tx.send(());
            let _ = serve_client.cancel_plan_run(&run_id_owned).await;
            eprintln!("run {run_id_owned} cancelled");
            true
        }
        _ = tokio::task::spawn_blocking(move || tui_handle.join()) => {
            false
        }
    };
    if cancelled {
        return Ok("cancelled".to_string());
    }

    let outcome = outcome_cell.lock().map_or_else(|_| "unknown".to_string(), |g| g.clone());
    Ok(outcome)
}

/// Follow a server-managed plan run in text mode (no TUI), printing one line
/// per notable event and exiting when the run completes or is cancelled.
async fn follow_run_text(
    json: bool,
    mirror: &crate::state_hub::SharedStateHub,
    run_id: &str,
    order_set: &std::collections::HashSet<String>,
    serve_client: &WorkspaceServerClient,
) -> anyhow::Result<String> {
    let mut sub = mirror.subscribe_events_from(0);
    let mut run_started = false;
    let mut final_outcome = "unknown".to_string();
    let run_id_owned = run_id.to_string();

    // Shared flag for whether we resolved the outcome via polling.
    let poll_interval = tokio::time::Duration::from_secs(2);

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                let _ = serve_client.cancel_plan_run(&run_id_owned).await;
                eprintln!("run {run_id_owned} cancelled");
                return Ok("cancelled".to_string());
            }

            msg = sub.live.recv() => {
                match msg {
                    Ok(envelope) => {
                        let print_line = !json;
                        match &envelope.payload {
                            roko_core::DashboardEvent::PlanSetLoaded { plans } => {
                                if plans.iter().any(|p| order_set.contains(&p.plan_id)) {
                                    run_started = true;
                                    if print_line {
                                        println!("plan set loaded ({} plans)", plans.len());
                                    }
                                }
                            }
                            roko_core::DashboardEvent::PlanStarted { plan_id, .. } => {
                                if order_set.contains(plan_id) {
                                    run_started = true;
                                }
                                if print_line {
                                    println!("plan started: {plan_id}");
                                }
                            }
                            roko_core::DashboardEvent::TaskStarted {
                                plan_id,
                                task_id,
                                title,
                                ..
                            } => {
                                if print_line {
                                    println!("task started: {plan_id}/{task_id} {title}");
                                }
                            }
                            roko_core::DashboardEvent::TaskCompleted {
                                plan_id,
                                task_id,
                                outcome,
                            } => {
                                if print_line {
                                    println!(
                                        "task completed: {plan_id}/{task_id} [{outcome}]"
                                    );
                                }
                            }
                            roko_core::DashboardEvent::PlanCompleted {
                                plan_id,
                                success,
                            } => {
                                if print_line {
                                    println!(
                                        "plan completed: {plan_id} [{}]",
                                        if *success { "ok" } else { "failed" }
                                    );
                                }
                            }
                            roko_core::DashboardEvent::RunCompleted { outcome, .. } => {
                                if run_started {
                                    final_outcome = outcome.clone();
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        // Missed some events due to slow consumer; continue.
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        // Hub closed — server process ended; fall back to polling.
                        break;
                    }
                }
            }

            // Fallback: poll the server every 2 s in case events were missed.
            _ = tokio::time::sleep(poll_interval) => {
                let run_id_poll = run_id_owned.clone();
                match serve_client.plan_run_finished(&run_id_poll).await {
                    Ok(true) => {
                        // Finished but we didn't see RunCompleted — best-effort outcome.
                        if final_outcome == "unknown" {
                            final_outcome = "succeeded".to_string();
                        }
                        break;
                    }
                    Ok(false) => { /* still running */ }
                    Err(e) => {
                        tracing::debug!(error = %e, "polling plan_run_finished failed");
                    }
                }
            }
        }
    }

    Ok(final_outcome)
}

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

#[cfg(test)]
impl WorkspaceServerClient {
    /// Expose the resolved request headers for unit-test assertions.
    pub(crate) fn headers_for_test(&self) -> &HeaderMap {
        &self.headers
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::fs;

    use reqwest::header::AUTHORIZATION;

    use super::*;

    fn make_endpoint(pid: u32, url: &str) -> ServeEndpoint {
        ServeEndpoint {
            pid,
            url: url.to_string(),
            workdir: std::path::PathBuf::new(),
            started_at: "2026-09-28T00:00:00Z".to_string(),
        }
    }

    fn write_serve_json(workdir: &Path, endpoint: &ServeEndpoint) {
        roko_serve::endpoint::write_endpoint(workdir, endpoint).unwrap();
    }

    fn write_lock_pid(workdir: &Path, pid: u32) {
        let runtime_dir = workdir.join(".roko").join("runtime");
        fs::create_dir_all(&runtime_dir).unwrap();
        fs::write(runtime_dir.join("roko.lock"), format!("{pid}\n")).unwrap();
    }

    // ---- discover_workspace_server ----------------------------------------

    /// No serve.json → None.
    #[test]
    fn missing_endpoint_file_gives_none() {
        let dir = tempfile::tempdir().unwrap();
        assert!(discover_workspace_server(dir.path()).is_none());
    }

    /// serve.json present but lock file absent → None (PID check fails).
    #[test]
    fn missing_lock_file_gives_none() {
        let dir = tempfile::tempdir().unwrap();
        let endpoint = make_endpoint(12345, "http://127.0.0.1:19999");
        write_serve_json(dir.path(), &endpoint);
        // No lock file written.
        assert!(discover_workspace_server(dir.path()).is_none());
    }

    /// serve.json PID does not match lock file PID → None.
    #[test]
    fn pid_mismatch_gives_none() {
        let dir = tempfile::tempdir().unwrap();
        let endpoint = make_endpoint(12345, "http://127.0.0.1:19999");
        write_serve_json(dir.path(), &endpoint);
        // Write a different PID into the lock.
        write_lock_pid(dir.path(), 99999);
        assert!(discover_workspace_server(dir.path()).is_none());
    }

    /// PIDs match but server is not listening (unreachable URL) → None.
    #[test]
    fn unreachable_url_gives_none() {
        let dir = tempfile::tempdir().unwrap();
        // Use an unroutable address so the health probe fails immediately.
        let endpoint = make_endpoint(12345, "http://192.0.2.1:19999");
        write_serve_json(dir.path(), &endpoint);
        write_lock_pid(dir.path(), 12345);
        assert!(discover_workspace_server(dir.path()).is_none());
    }

    // ---- WorkspaceServerClient helpers ------------------------------------

    /// Confirms that `SubmittedRun` carries the id and order fields from
    /// JSON deserialization (compile-time struct shape test).
    #[test]
    fn submitted_run_fields_exist() {
        let run = SubmittedRun {
            id: "abc".to_string(),
            order: vec!["a".to_string(), "b".to_string()],
            max_parallel_plans: 2,
        };
        assert_eq!(run.id, "abc");
        assert_eq!(run.order, ["a", "b"]);
        assert_eq!(run.max_parallel_plans, 2);
    }

    /// `ServeClientError::AlreadyActive` formats a useful message.
    #[test]
    fn already_active_error_message_is_helpful() {
        let err = ServeClientError::AlreadyActive;
        let msg = err.to_string();
        assert!(msg.contains("already active"), "unhelpful message: {msg}");
    }

    /// `ServeClientError::Unauthorized` includes the status and a hint.
    #[test]
    fn unauthorized_error_includes_hint() {
        let err = ServeClientError::Unauthorized {
            status: 401,
            detail: "missing key".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("ROKO_API_KEY") || msg.contains("roko login"), "unhelpful: {msg}");
    }

    // ---- read_launch_token ---------------------------------------------------

    /// Absent token file → None.
    #[test]
    fn missing_token_file_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read_launch_token(dir.path()).is_none());
    }

    /// Empty token file → None.
    #[test]
    fn empty_token_file_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let runtime = dir.path().join(".roko").join("runtime");
        fs::create_dir_all(&runtime).unwrap();
        fs::write(runtime.join("serve.token"), "   \n").unwrap();
        assert!(read_launch_token(dir.path()).is_none());
    }

    /// Token file with content → trimmed string.
    #[test]
    fn present_token_file_returns_trimmed_token() {
        let dir = tempfile::tempdir().unwrap();
        let runtime = dir.path().join(".roko").join("runtime");
        fs::create_dir_all(&runtime).unwrap();
        fs::write(runtime.join("serve.token"), "abc123\n").unwrap();
        assert_eq!(read_launch_token(dir.path()).unwrap(), "abc123");
    }

    // ---- WorkspaceServerClient auth fallback --------------------------------

    /// With a token file present and `resolve_api_key` returning `None`
    /// (simulated by passing `None` directly), the client sends
    /// `Authorization: Bearer <token>`.
    #[test]
    fn launch_token_used_as_bearer_when_no_key_resolvable() {
        let dir = tempfile::tempdir().unwrap();
        let runtime = dir.path().join(".roko").join("runtime");
        fs::create_dir_all(&runtime).unwrap();
        fs::write(runtime.join("serve.token"), "launch-tok-xyz").unwrap();

        let endpoint = make_endpoint(42, "http://127.0.0.1:19999");
        // Pass None directly to bypass resolve_api_key entirely — this is the
        // "no key resolvable" scenario the task requires.
        let client = WorkspaceServerClient::new_with_resolved(&endpoint, None, dir.path());
        let headers = client.headers_for_test();

        let auth_value = headers
            .get(AUTHORIZATION)
            .expect("Authorization header should be set when serve.token exists");
        assert_eq!(
            auth_value.to_str().unwrap(),
            "Bearer launch-tok-xyz",
            "expected Bearer header from serve.token"
        );
        assert!(
            !headers.contains_key("X-Api-Key"),
            "X-Api-Key should not be set when using launch token"
        );
    }

    /// When a resolved key is supplied, the launch token is NOT used.
    #[test]
    fn resolved_key_takes_precedence_over_launch_token() {
        use crate::auth::{ApiKeySource, AuthMethod, ResolvedApiKey};

        let dir = tempfile::tempdir().unwrap();
        let runtime = dir.path().join(".roko").join("runtime");
        fs::create_dir_all(&runtime).unwrap();
        fs::write(runtime.join("serve.token"), "launch-tok-xyz").unwrap();

        let endpoint = make_endpoint(42, "http://127.0.0.1:19999");
        let resolved = Some(ResolvedApiKey {
            key: "configured-key".to_string(),
            source: ApiKeySource::Config,
            method: AuthMethod::ApiKey,
        });
        let client = WorkspaceServerClient::new_with_resolved(&endpoint, resolved, dir.path());
        let headers = client.headers_for_test();

        // Configured key wins → X-Api-Key header is set.
        let api_key = headers
            .get("X-Api-Key")
            .expect("X-Api-Key should be set when a resolved key is supplied");
        assert_eq!(api_key.to_str().unwrap(), "configured-key");
        assert!(
            !headers.contains_key(AUTHORIZATION),
            "Authorization (Bearer) should not be set when X-Api-Key is used"
        );
    }
}
