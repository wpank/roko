//! HTTP server for the roko API.
//!
//! The [`ServerBuilder`] type is the main entrypoint for embedding the HTTP
//! server. [`run_server`] remains as a convenience wrapper for the current
//! CLI flow.
// Per-item #[allow] is preferred over blanket suppression.
// These lints are suppressed crate-wide because they are pervasive and stylistic
// across ~100 route modules and ~40K lines of HTTP handler code.
#![allow(
    missing_docs,
    clippy::derivable_impls,
    clippy::large_enum_variant,
    clippy::let_underscore_future,
    clippy::module_name_repetitions,
    clippy::needless_continue,
    clippy::needless_pass_by_ref_mut,
    clippy::struct_field_names,
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::trait_duplication_in_bounds
)]
// Re-export StateHub types from their canonical home in roko-runtime.
// These were previously path-included from roko-core via a fake
// `extern crate self as roko_core` alias. Task 104 moved them to
// roko-runtime where they can legally depend on EventBus.
pub use roko_runtime::{SharedStateHub, StateHub, StateHubSender};

/// Compatibility re-export so `roko_serve::state_hub::*` still resolves
/// for downstream crates that haven't migrated their imports yet.
pub mod state_hub {
    pub use roko_runtime::state_hub::*;
}

pub mod adapters;
pub mod agent_lifecycle;
pub mod auth_audit;
pub mod bench;
pub mod command_events;
pub mod config_watcher;
pub mod deploy;
pub mod dispatch;
pub mod dreams;
pub mod embedded;
pub mod endpoint;
pub mod error;
pub mod event_bus;
pub mod events;
pub mod extract;
pub mod feed_agents;
pub mod feedback;
pub mod fswatcher;
#[cfg(feature = "groups")]
pub mod group_runtime;
pub mod integrations;
pub mod job_runner;
pub mod jwks;
pub mod openapi;
mod operations;
pub mod parity;
pub mod plan_types;
pub mod projection_contract;
pub mod rbac;
pub mod relay;
pub mod retention;
pub mod routes;
pub mod runtime;
pub mod runtime_event_bridge;
pub mod sanitize;
pub mod scheduler;
pub mod service_factory;
pub mod showcase;
pub mod state;
// The relay bridge that consumes this needs `relay` (9220); the journal and
// status types also back `GET /api/subscriptions/relay/status`.
#[cfg_attr(not(feature = "relay"), allow(dead_code))]
mod subscription_relay;
mod telemetry_observer;
pub mod templates;
pub mod terminal;
pub mod trigger_runtime;
mod trigger_tls;
pub mod truth_map;
pub use service_factory::{ServiceBundle, ServiceConfig, ServiceFactory};

pub use crate::routes::reload_config_from_disk;
pub use crate::sanitize::sanitize_agent_content;

use std::collections::HashSet;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context as AnyhowContext, Result};
use axum::response::IntoResponse;
use roko_core::config::ServeConfig;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tower_http::trace::TraceLayer;
use tracing::{debug, info, warn};

use roko_core::Signal;
use roko_core::config::schema::RokoConfig;
use roko_core::config::showcase::PASSPHRASE_HASH_ENV;
use roko_core::connector::{ConnectorHealth, ConnectorInfo, ConnectorKind, ConnectorStatus};
use roko_core::dashboard_snapshot::DashboardEvent;
use roko_core::feed::{FeedAccess, FeedInfo, FeedKind};
use roko_core::foundation::EventConsumer;
use roko_core::{RuntimeEvent, WorkflowOutcome};
use roko_plugin::manifest::discover_plugins;
use roko_plugin::{CronEventSource, EventSource, FileWatchEventSource};

use crate::events::{ExecutionEvent, ServerEvent};
use runtime::CliRuntime;
use state::AppState;

/// Resolve the live-agent-output mode and startup log line from the configured
/// value and the effective listener bind address.
///
/// Trusted output is silently downgraded to `ToolSteps` on non-loopback binds
/// and a warning message is returned instead of the normal confirmation so the
/// operator knows the requested level was not applied.
fn live_agent_output_for_bind(
    configured: roko_core::config::serve::LiveAgentOutput,
    bind: &str,
) -> (roko_core::config::serve::LiveAgentOutput, String) {
    use roko_core::config::serve::LiveAgentOutput;
    let loopback = routes::bind_is_loopback(bind);
    let effective = configured.effective(loopback);
    let message = match effective {
        LiveAgentOutput::Trusted => "live agent output: trusted".to_string(),
        LiveAgentOutput::ToolSteps if configured == LiveAgentOutput::Trusted => format!(
            "live agent output: trusted ignored on non-loopback bind {bind}; streaming tool steps only"
        ),
        LiveAgentOutput::ToolSteps => "live agent output: tool_steps".to_string(),
    };
    (effective, message)
}

/// Inputs required to start the HTTP server.
pub struct ServerBuildConfig {
    /// Project working directory.
    pub workdir: PathBuf,
    /// Runtime bridge to CLI operations (run_once, status, dashboard).
    pub runtime: Arc<dyn CliRuntime>,
    /// Shared state hub to use for AppState and in-process runtimes.
    pub state_hub: Option<crate::SharedStateHub>,
    /// Full `roko.toml` schema configuration.
    pub roko_config: RokoConfig,
    /// Optional bind address override.
    pub bind: Option<String>,
    /// Optional port override.
    pub port: Option<u16>,
    /// Optional pre-created [`MetricRegistry`] shared with the runtime so
    /// that metrics collected during plan execution are visible on the
    /// `/metrics` endpoint (E09-T03).
    pub metrics: Option<Arc<roko_core::obs::metrics::MetricRegistry>>,
    /// When `true`, the server writes `.roko/runtime/serve.json` after
    /// binding so that CLI commands can discover the running server.
    /// Off by default; opt in with [`Self::with_advertised_endpoint`].
    pub advertise_endpoint: bool,
    /// Optional ephemeral launch token for the local-access auth flow.
    ///
    /// When set **and** `advertise_endpoint` is `true`, the server writes
    /// `.roko/runtime/serve.token` (mode 0600) alongside `serve.json` so that
    /// local CLI commands can authenticate without a full API key.
    /// The token is **never** written to `serve.json` or any log.
    pub launch_token: Option<String>,
}

impl ServerBuildConfig {
    /// Create a new server build configuration.
    pub fn new(
        workdir: PathBuf,
        runtime: Arc<dyn CliRuntime>,
        roko_config: RokoConfig,
        bind: Option<String>,
        port: Option<u16>,
    ) -> Self {
        Self {
            workdir,
            runtime,
            state_hub: None,
            roko_config,
            bind,
            port,
            metrics: None,
            advertise_endpoint: false,
            launch_token: None,
        }
    }

    /// Use a caller-provided state hub instead of constructing one inside AppState.
    #[must_use]
    pub fn with_state_hub(mut self, state_hub: crate::SharedStateHub) -> Self {
        self.state_hub = Some(state_hub);
        self
    }

    /// Share a pre-created [`MetricRegistry`] so the runtime and server expose
    /// the same counters on the `/metrics` endpoint (E09-T03).
    #[must_use]
    pub fn with_metrics(mut self, metrics: Arc<roko_core::obs::metrics::MetricRegistry>) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// Enable workspace endpoint advertisement.
    ///
    /// When this flag is set, [`ServerBuilder::start_background`] writes
    /// `.roko/runtime/serve.json` immediately after binding and removes it
    /// when the serve task exits.  Off by default so tests and embedded
    /// callers that do not own a real workspace do not touch the filesystem.
    #[must_use]
    pub fn with_advertised_endpoint(mut self) -> Self {
        self.advertise_endpoint = true;
        self
    }

    /// Attach an ephemeral launch token to this server configuration.
    ///
    /// When the server advertises its endpoint (`with_advertised_endpoint`),
    /// the token is written to `.roko/runtime/serve.token` (mode 0600) so that
    /// local CLI commands can authenticate without a pre-configured API key.
    /// The token is **never** written to `serve.json` or any log.
    ///
    /// The token is also stored on [`AppState::local_access`] for the
    /// duration of the server process.
    #[must_use]
    pub fn with_launch_token(mut self, token: String) -> Self {
        self.launch_token = Some(token);
        self
    }

    /// Resolve the effective bind address from CLI override or config.
    pub fn effective_bind(&self) -> &str {
        self.bind
            .as_deref()
            .unwrap_or(&self.roko_config.server.bind)
    }

    /// Resolve the effective port from CLI override or config.
    pub fn effective_port(&self) -> u16 {
        self.port.unwrap_or_else(|| {
            // Prefer [serve].port when [server].port is still the default.
            if self.roko_config.server.port == 6677 {
                self.roko_config
                    .serve
                    .port
                    .unwrap_or(self.roko_config.server.port)
            } else {
                self.roko_config.server.port
            }
        })
    }

    /// Resolve the effective address string (`bind:port`).
    pub fn effective_addr(&self) -> String {
        format!("{}:{}", self.effective_bind(), self.effective_port())
    }

    /// Persist the resolved listener bind into the configuration shared with
    /// request handlers. CLI overrides must be visible to authorization checks;
    /// retaining the file value here can otherwise misclassify a public listener
    /// as loopback (or reject a loopback override of a public config).
    fn synchronize_effective_bind(&mut self) -> String {
        let bind = self.effective_bind().to_string();
        self.roko_config.server.bind.clone_from(&bind);
        bind
    }
}

/// Resolve the bind socket when the `PORT` environment variable is in play.
///
/// Cloud platforms (Railway, Fly, etc.) set `PORT` to tell the server which
/// port to listen on, but they intentionally do **not** dictate the bind
/// address. Earlier behaviour silently rebound to `0.0.0.0`, which exposed
/// the API surface of every local-dev workflow that happened to have `PORT`
/// set in its shell. From T3-25 onwards we honour `PORT` for the port only;
/// the bind comes from `[server].bind` in `roko.toml` (default
/// `127.0.0.1`). Operators who actually want a public bind opt in by
/// setting `bind = "0.0.0.0"` in their config (and clearing the existing
/// `serve.acknowledge_public_risk` / `serve.auth.enabled` checks in
/// [`validate_bind_safety`]).
pub(crate) fn resolve_bind_with_port_env(
    config_bind: &str,
    cli_bind_override: Option<&str>,
    config_port: u16,
    cli_port_override: Option<u16>,
    port_env: Option<&str>,
) -> Result<(String, u16)> {
    let resolved_port = match port_env {
        Some(value) => value
            .parse::<u16>()
            .with_context(|| format!("PORT env var must be a valid u16 (got {value:?})"))?,
        None => cli_port_override.unwrap_or(config_port),
    };
    let resolved_bind = cli_bind_override.unwrap_or(config_bind).to_string();
    Ok((resolved_bind, resolved_port))
}

/// Builder for the HTTP server.
///
/// The builder keeps the resolved bind address, runtime config, and lazily
/// constructed application state together so the same server implementation
/// can be reused by the CLI and future embedders.
pub struct ServerBuilder {
    addr: String,
    config: ServerBuildConfig,
    state: Option<Arc<AppState>>,
}

impl ServerBuilder {
    /// Start a new server builder from the resolved runtime config.
    #[must_use]
    pub fn new(config: ServerBuildConfig) -> Self {
        let addr = config.effective_addr();
        Self {
            addr,
            config,
            state: None,
        }
    }

    /// Enable API-key authentication with the provided key.
    #[must_use]
    pub fn with_auth(mut self, key: impl Into<String>) -> Self {
        self.config.roko_config.serve.auth.enabled = true;
        self.config.roko_config.serve.auth.api_key = key.into();
        self
    }

    /// Start the server in the background and return the live state handle.
    ///
    /// The returned [`Arc<AppState>`] carries the [`SharedStateHub`] that the
    /// TUI or other in-process consumers can subscribe to.  The
    /// [`JoinHandle`] resolves when the server shuts down (e.g. because
    /// `state.cancel.cancel()` was called).
    #[allow(clippy::missing_panics_doc)]
    pub async fn start_background(mut self) -> Result<(Arc<AppState>, JoinHandle<Result<()>>)> {
        normalize_serve_dispatch_config(&mut self.config.roko_config)?;

        // AppState is the authorization source of truth after startup, so it
        // must carry the actual listener bind rather than the pre-override file
        // value. `PORT` may replace the port below, but never the resolved bind.
        let effective_bind = self.config.synchronize_effective_bind();

        // -- PORT env var override (Railway / cloud platforms) -------------
        // The `PORT` env var lets the platform pick a port; it does NOT imply
        // the operator wants a public bind. Per T3-25 we override only the
        // port and keep the bind from config (default `127.0.0.1`). Setting
        // `[server].bind = "0.0.0.0"` in `roko.toml` is the explicit opt-in.
        let port_env = std::env::var("PORT").ok();
        let addr = if let Some(value) = port_env.as_deref() {
            let (bind, port) = resolve_bind_with_port_env(
                &effective_bind,
                self.config.bind.as_deref(),
                self.config.roko_config.server.port,
                self.config.port,
                Some(value),
            )?;
            info!(
                "PORT env var detected ({port}); binding to {bind}:{port} \
                 (set `[server].bind = \"0.0.0.0\"` for a public bind)"
            );
            format!("{bind}:{port}")
        } else {
            self.addr.clone()
        };

        let workdir = self.config.workdir.clone();
        let runtime = Arc::clone(&self.config.runtime);
        let roko_config = self.config.roko_config.clone();
        let state_hub = self.config.state_hub.clone();
        let metrics = self.config.metrics.clone();
        if self.state.is_none() {
            self.state = Some(Arc::new(build_app_state(
                workdir,
                runtime,
                roko_config,
                state_hub,
                metrics,
                self.config.launch_token.clone(),
            )?));
        }
        let state = Arc::clone(
            self.state
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("server state not initialized"))?,
        );
        let roko_config = state.load_roko_config();
        validate_bind_safety(&addr, &roko_config.serve)?;
        let passphrase_hash = std::env::var(PASSPHRASE_HASH_ENV).ok();
        validate_showcase_mode(&roko_config, passphrase_hash.as_deref())?;
        state.configure_listener_security(&effective_bind, roko_config.serve.auth.enabled);
        let (live_setting, live_msg) =
            live_agent_output_for_bind(roko_config.serve.live_agent_output, &effective_bind);
        state.configure_live_agent_output(live_setting);
        println!("{live_msg}");
        info!("{live_msg}");
        if !roko_config.serve.auth.enabled {
            tracing::warn!(
                "roko serve is running WITHOUT authentication. Set [serve.auth] enabled = true in roko.toml to require API keys."
            );
        }
        warn_if_auth_misconfigured(&roko_config.serve.auth);

        // Conditionally initialize OTLP tracing export when the feature is
        // enabled and an endpoint is configured.
        #[cfg(feature = "otlp")]
        if let Some(endpoint) = &roko_config.serve.tracing.otlp_endpoint {
            init_otlp_tracing(
                endpoint,
                &roko_config.serve.tracing.service_name,
                roko_config.serve.tracing.sample_rate,
            );
        }

        if let Err(err) = state.restore_snapshot().await {
            warn!(error = %err, "failed to restore server state snapshot; starting fresh");
        }
        for feed_id in [
            "file-watch-roko-dir",
            "provider-health-feed",
            "episode-outcome-feed",
        ] {
            match state.runtime_feeds.start_registered(feed_id) {
                Ok(handle) => {
                    let _feed_bridge = state.feed_bus_bridge.spawn(handle.cell().subscribe());
                }
                Err(error) => {
                    warn!(feed_id, %error, "failed to start built-in runtime feed");
                }
            }
        }
        let dispatcher_roko_config = roko_config.as_ref().clone();
        let dispatcher = Arc::new(dispatch::TemplateAgentDispatcher::new(
            state.workdir.clone(),
            None,
            dispatcher_roko_config,
        ));
        tokio::spawn(dispatch::dispatch_loop(Arc::clone(&state), dispatcher));
        let _github_event_subscriber = events::start_github_event_subscriber(Arc::clone(&state));
        let _telemetry_producer_bridge = start_telemetry_producer_bridge(Arc::clone(&state));
        start_builtin_event_sources(Arc::clone(&state), roko_config.as_ref().clone());
        let _trigger_runtime = trigger_runtime::ensure_trigger_runtime(&state).await;
        if let Err(error) = state.gateway_http.gateway.spawn_gateway_loop() {
            warn!(%error, "failed to start E26 inference gateway handle loop");
        }
        let _gateway_batch_loop = state.gateway_http.spawn_batch_loop();
        let _config_watcher = config_watcher::start_config_watcher(Arc::clone(&state));
        let _feedback_loop = feedback::start_feedback_loop(Arc::clone(&state));
        let bridge_dedup = BridgeDedup::new();
        let _state_hub_bridge = start_state_hub_bridge(Arc::clone(&state), bridge_dedup.clone());
        let _orchestrator_bridge =
            start_orchestrator_event_bridge_dedup(Arc::clone(&state), bridge_dedup);
        let _state_saver = start_state_snapshot_saver(Arc::clone(&state));
        let _cascade_router_saver = start_cascade_router_saver(Arc::clone(&state));
        let _job_runner = job_runner::start_job_runner(Arc::clone(&state));
        let _cold_archival = start_cold_archival_timer(Arc::clone(&state));
        let _workspace_gc = start_workspace_gc(Arc::clone(&state));
        let _handle_gc = start_handle_gc(Arc::clone(&state));
        let _demurrage = start_demurrage_timer(Arc::clone(&state));
        let _block_watcher = start_block_watcher(Arc::clone(&state));

        // Load persisted deployments from disk.
        routes::load_persisted_deployments(&state).await;

        // Eagerly prime the JWKS cache if Privy auth is configured.
        if roko_config.serve.auth.privy_app_id.is_some() {
            state.jwks_cache.start_refresh_task();
            let jwks = Arc::clone(&state.jwks_cache);
            tokio::spawn(async move {
                jwks.prime().await;
            });
        }

        // Register workspace with relay if configured.
        #[cfg(feature = "relay")]
        let _relay_registration = relay::start_workspace_registration(
            self.config.roko_config.relay.clone(),
            self.config.port.unwrap_or(6677),
            Arc::clone(&state.agent_count),
            Arc::clone(&state.relay_health),
        );
        #[cfg(not(feature = "relay"))]
        if self.config.roko_config.relay.url.is_some() {
            warn!(
                "[relay] url ignored: rebuild roko with `--features relay` to register \
                 with the relay and bridge feeds to it"
            );
        }

        // Spawn feed agents publishing to the relay and local event bus.
        let _feed_agents = feed_agents::spawn_all(Arc::clone(&state));

        // Bridge feed agents to the relay: registers feeds and forwards ticks.
        #[cfg(feature = "relay")]
        let _feed_relay_bridge = start_feed_relay_bridge(Arc::clone(&state));

        // Register plugin webhook route scopes with the middleware so that
        // plugin-declared webhook endpoints are not misclassified as
        // "write:unclassified". Must happen before build_server_router.
        register_plugin_webhook_scopes(&state.workdir);

        let router = build_server_router(
            Arc::clone(&state),
            &roko_config.server.cors_origins,
            roko_config.server.unsafe_public_cors,
            roko_config.serve.auth.clone(),
        );
        let trigger_tls = trigger_tls::load(state.as_ref()).await?;

        let listener = TcpListener::bind(&addr)
            .await
            .with_context(|| format!("bind to {addr}"))?;
        if let Ok(local_addr) = listener.local_addr() {
            state
                .terminal_sessions
                .configure_server_env_from_addr(local_addr, roko_config.as_ref());
        }
        let telemetry_observer =
            telemetry_observer::start_periodic_telemetry_observer(state.as_ref());

        let scheme = if trigger_tls.is_some() {
            "https"
        } else {
            "http"
        };
        info!("roko server listening on {scheme}://{addr}");
        info!("workdir: {}", self.config.workdir.display());

        // Advertise this server in the workspace when the caller opted in
        // (off by default so tests and embedded callers do not touch the fs).
        let advertise_endpoint = self.config.advertise_endpoint;
        let endpoint_workdir = self.config.workdir.clone();
        if advertise_endpoint {
            if let Ok(local_addr) = listener.local_addr() {
                let ep = endpoint::ServeEndpoint {
                    pid: std::process::id(),
                    url: endpoint::local_url(local_addr, scheme),
                    workdir: endpoint_workdir.clone(),
                    started_at: chrono::Utc::now().to_rfc3339(),
                };
                if let Err(err) = endpoint::write_endpoint(&endpoint_workdir, &ep) {
                    warn!(%err, "failed to write serve endpoint file");
                }
                // Write the ephemeral launch token alongside serve.json (mode 0600).
                // The token never goes into serve.json — it is a separate file so
                // secrets are not accidentally included in endpoint payloads.
                if let Some(ref token) = self.config.launch_token {
                    if let Err(err) = endpoint::write_token(&endpoint_workdir, token) {
                        warn!(%err, "failed to write serve token file");
                    }
                }
            }
        }

        // Spawn chain-watcher if chain.rpc_url is configured (best-effort).
        // Redirect all subprocess output through a day-based rolling writer
        // under .roko/ so long-running serve sessions do not grow one unbounded file.
        if let Some(rpc_url) = self.config.roko_config.chain.rpc_url.as_deref() {
            let rpc = rpc_url.to_string();
            let log_dir = self.config.workdir.join(".roko");
            tokio::spawn(async move {
                let watcher = std::env::current_exe()
                    .ok()
                    .and_then(|p| p.parent().map(|d| d.join("roko-chain-watcher")))
                    .unwrap_or_else(|| std::path::PathBuf::from("roko-chain-watcher"));

                // Pipe subprocess output and relay through a daily rolling writer.
                let mut child = match tokio::process::Command::new(&watcher)
                    .arg("--rpc-url")
                    .arg(&rpc)
                    .env("ROKO_LOG", "warn")
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .spawn()
                {
                    Ok(child) => child,
                    Err(e) => {
                        tracing::debug!(error = %e, path = ?watcher, "chain-watcher not available");
                        return;
                    }
                };

                // Day-based rolling writer: chain-watcher.log.YYYY-MM-DD
                let rolling = tracing_appender::rolling::daily(&log_dir, "chain-watcher.log");
                let rolling = std::sync::Arc::new(std::sync::Mutex::new(rolling));

                // Relay stdout and stderr from the subprocess into the rolling log.
                let stdout = child.stdout.take();
                let stderr = child.stderr.take();

                let relay = |pipe: Option<tokio::process::ChildStdout>,
                             pipe_err: Option<tokio::process::ChildStderr>,
                             writer: std::sync::Arc<
                    std::sync::Mutex<tracing_appender::rolling::RollingFileAppender>,
                >| async move {
                    use tokio::io::AsyncBufReadExt;
                    let mut handles = Vec::new();
                    if let Some(out) = pipe {
                        let w = writer.clone();
                        handles.push(tokio::spawn(async move {
                            let reader = tokio::io::BufReader::new(out);
                            let mut lines = reader.lines();
                            while let Ok(Some(line)) = lines.next_line().await {
                                if let Ok(mut w) = w.lock() {
                                    use std::io::Write;
                                    let _ = writeln!(w, "{line}");
                                }
                            }
                        }));
                    }
                    if let Some(err) = pipe_err {
                        let w = writer;
                        handles.push(tokio::spawn(async move {
                            let reader = tokio::io::BufReader::new(err);
                            let mut lines = reader.lines();
                            while let Ok(Some(line)) = lines.next_line().await {
                                if let Ok(mut w) = w.lock() {
                                    use std::io::Write;
                                    let _ = writeln!(w, "{line}");
                                }
                            }
                        }));
                    }
                    for h in handles {
                        let _ = h.await;
                    }
                };

                relay(stdout, stderr, rolling).await;

                match child.wait().await {
                    Ok(s) => tracing::info!(exit = %s, "chain-watcher exited"),
                    Err(e) => {
                        tracing::debug!(error = %e, path = ?watcher, "chain-watcher wait failed")
                    }
                }
            });
        }

        let serve_state = Arc::clone(&state);
        let observer_cancel = state.cancel.clone();
        let handle = tokio::spawn(async move {
            let serve_result = if let Some(trigger_tls) = trigger_tls {
                trigger_tls::serve(listener, router, serve_state.cancel.clone(), trigger_tls).await
            } else {
                serve_until_cancelled(listener, router, serve_state).await
            };
            // Also cancel on an unexpected Axum exit, then join the observer
            // before reporting the server result. This prevents detached
            // telemetry tasks from outliving the production serve lifecycle.
            observer_cancel.cancel();
            if let Err(error) = telemetry_observer.await {
                warn!(%error, "periodic telemetry observer join failed");
            }
            serve_result?;
            // Remove the endpoint advertisement and token when we are the owner.
            // The token file must be removed first because remove_token_if_owned
            // reads serve.json to verify PID ownership.
            if advertise_endpoint {
                endpoint::remove_token_if_owned(&endpoint_workdir, std::process::id());
                endpoint::remove_endpoint_if_owned(&endpoint_workdir, std::process::id());
            }
            info!("server stopped");
            Ok(())
        });

        Ok((state, handle))
    }

    /// Bind and run the HTTP server until shutdown.
    ///
    /// # Errors
    ///
    /// Returns an error if the `PORT` environment variable is not a valid
    /// `u16`, the listener cannot bind, or the Axum server exits with an
    /// error.
    pub async fn run(self) -> Result<()> {
        let (state, handle) = self.start_background().await?;
        // Block on Ctrl-C to shut down.
        let _ = tokio::signal::ctrl_c().await;
        info!("received ctrl-c, shutting down");
        state.shutdown().await;
        // Wait for the server task to finish.
        match handle.await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(e),
            Err(e) => return Err(anyhow::anyhow!("server task panicked: {e}")),
        }
        Ok(())
    }
}

/// Start the HTTP server.
///
/// # Errors
///
/// Returns an error if config cannot be read or parsed, if the resolved
/// listener cannot bind, or if serving the Axum router fails.
pub async fn run_server(
    workdir: PathBuf,
    runtime: Arc<dyn CliRuntime>,
    bind: Option<String>,
    port: Option<u16>,
) -> Result<()> {
    let roko_config = roko_core::config::loader::load_config_unified(&workdir)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let config = ServerBuildConfig::new(workdir, runtime, roko_config, bind, port);
    ServerBuilder::new(config).run().await
}

/// Start the HTTP server in the background and return the live app state.
///
/// The returned [`Arc<AppState>`] carries the [`SharedStateHub`] that an
/// in-process TUI or other consumer can subscribe to.  The
/// [`JoinHandle`] resolves when the server shuts down.
///
/// Call `state.cancel.cancel()` or `state.shutdown().await` to stop the
/// server.
pub async fn start_server_background(
    workdir: PathBuf,
    runtime: Arc<dyn CliRuntime>,
    bind: Option<String>,
    port: Option<u16>,
) -> Result<(Arc<AppState>, JoinHandle<Result<()>>)> {
    let roko_config = roko_core::config::loader::load_config_unified(&workdir)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let config = ServerBuildConfig::new(workdir, runtime, roko_config, bind, port);
    ServerBuilder::new(config).start_background().await
}

/// Bridges WorkflowEngine RuntimeEvents to SharedStateHub as DashboardEvents.
struct DashboardEventBridge {
    state_hub: SharedStateHub,
}

impl DashboardEventBridge {
    fn new(state_hub: SharedStateHub) -> Self {
        Self { state_hub }
    }
}

impl EventConsumer for DashboardEventBridge {
    fn consume(&self, event: &RuntimeEvent) {
        let events = match event {
            RuntimeEvent::WorkflowStarted { run_id, prompt, .. } => {
                let plan_id = workflow_plan_id(run_id);
                let task_id = workflow_task_id(run_id);
                vec![
                    DashboardEvent::PlanStarted {
                        plan_id: plan_id.clone(),
                        tasks_total: 0,
                    },
                    DashboardEvent::TaskStarted {
                        plan_id,
                        task_id,
                        title: prompt.clone(),
                        phase: "workflow".into(),
                    },
                ]
            }
            RuntimeEvent::AgentSpawned {
                agent_id,
                role,
                model,
                ..
            } => vec![DashboardEvent::AgentSpawned {
                agent_id: agent_id.clone(),
                plan_id: String::new(),
                task_id: String::new(),
                attempt: 0,
                role: role.clone(),
                model: dashboard_model_label(model, agent_id),
                provider: String::new(),
            }],
            RuntimeEvent::AgentOutput {
                agent_id, chunk, ..
            } => vec![DashboardEvent::AgentOutput {
                agent_id: agent_id.clone(),
                plan_id: String::new(),
                task_id: String::new(),
                attempt: 0,
                content: chunk.clone(),
            }],
            RuntimeEvent::AgentCompleted { agent_id, .. } => {
                vec![DashboardEvent::AgentCompleted {
                    agent_id: agent_id.clone(),
                    plan_id: String::new(),
                    task_id: String::new(),
                    attempt: 0,
                }]
            }
            RuntimeEvent::PhaseTransition { run_id, from, to } => {
                vec![DashboardEvent::PhaseTransition {
                    plan_id: workflow_plan_id(run_id),
                    from: from.clone(),
                    to: to.clone(),
                }]
            }
            RuntimeEvent::GatePassed {
                run_id,
                gate_name,
                duration_ms,
            } => vec![
                DashboardEvent::GateResult {
                    plan_id: workflow_plan_id(run_id),
                    task_id: workflow_task_id(run_id),
                    gate: gate_name.clone(),
                    passed: true,
                    output_text: None,
                },
                workflow_gate_log_entry(run_id, gate_name, *duration_ms, true),
            ],
            RuntimeEvent::GateFailed {
                run_id,
                gate_name,
                duration_ms,
                ..
            } => vec![
                DashboardEvent::GateResult {
                    plan_id: workflow_plan_id(run_id),
                    task_id: workflow_task_id(run_id),
                    gate: gate_name.clone(),
                    passed: false,
                    output_text: None,
                },
                workflow_gate_log_entry(run_id, gate_name, *duration_ms, false),
            ],
            RuntimeEvent::WorkflowCompleted { run_id, outcome } => {
                let plan_id = workflow_plan_id(run_id);
                let success = matches!(outcome, WorkflowOutcome::Success { .. });
                vec![
                    DashboardEvent::TaskCompleted {
                        plan_id: plan_id.clone(),
                        task_id: workflow_task_id(run_id),
                        outcome: workflow_outcome_label(outcome),
                    },
                    DashboardEvent::PlanCompleted { plan_id, success },
                ]
            }
            _ => Vec::new(),
        };

        if !events.is_empty() {
            self.state_hub.publish_batch(events);
        }
    }
}

/// Create a DashboardEventBridge for attaching to WorkflowEngine instances.
#[must_use]
pub fn dashboard_event_bridge(state: &Arc<AppState>) -> Arc<dyn EventConsumer> {
    Arc::new(DashboardEventBridge::new(state.state_hub.clone()))
}

fn workflow_plan_id(run_id: &str) -> String {
    format!("wf-{}", run_id.chars().take(8).collect::<String>())
}

fn workflow_task_id(run_id: &str) -> String {
    format!("workflow-{}", run_id.chars().take(8).collect::<String>())
}

fn workflow_gate_log_entry(
    run_id: &str,
    gate_name: &str,
    duration_ms: u64,
    passed: bool,
) -> DashboardEvent {
    DashboardEvent::EventLogEntry {
        timestamp_ms: now_millis(),
        event_type: "gate_result".into(),
        plan_id: workflow_plan_id(run_id),
        task_id: gate_name.to_string(),
        message: format!(
            "{} {} ({}ms)",
            if passed { "PASS" } else { "FAIL" },
            gate_name,
            duration_ms
        ),
    }
}

fn workflow_outcome_label(outcome: &WorkflowOutcome) -> String {
    match outcome {
        WorkflowOutcome::Success { commit_hash } => commit_hash
            .as_ref()
            .map_or_else(|| "success".to_string(), |hash| format!("success ({hash})")),
        WorkflowOutcome::Halted { reason } => format!("halted: {reason}"),
        WorkflowOutcome::Cancelled => "cancelled".to_string(),
    }
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

/// Returns `true` when the host portion of `addr` resolves to a loopback address.
///
/// Handles both `127.0.0.1`, `::1`, and hostnames like `localhost`.
/// Returns `false` on parse errors (conservative: unknown = non-loopback).
fn is_loopback_addr(addr: &str) -> bool {
    let host = if let Some(rest) = addr.strip_prefix('[') {
        if let Some(bracket_end) = rest.find(']') {
            &rest[..bracket_end]
        } else {
            addr
        }
    } else if let Some(colon) = addr.rfind(':') {
        &addr[..colon]
    } else {
        addr
    };

    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        return ip.is_loopback();
    }

    host.eq_ignore_ascii_case("localhost")
}

/// Validate that a bind address is safe to expose.
///
/// Loopback addresses are always allowed. Public addresses require either
/// authentication or an explicit acknowledgement of the risk.
/// Emit a startup warning when auth is enabled but no credential is configured.
///
/// When `serve.auth.enabled = true` (the default) but no API key is set and no
/// Privy app ID is configured, every inbound request will be rejected with 401.
/// This is the safe posture, but it is almost always a misconfiguration that
/// the operator should know about immediately.
pub(crate) fn warn_if_auth_misconfigured(auth: &roko_core::config::ServeAuthConfig) {
    if !auth.enabled {
        return;
    }
    let has_legacy_key = !auth.api_key.is_empty();
    let has_named_keys = !auth.api_keys.is_empty();
    let has_privy =
        auth.privy_app_id.is_some() && crate::routes::middleware::privy_allow_list_configured(auth);
    if auth.privy_app_id.is_some() && !has_privy {
        tracing::warn!("{}", crate::routes::middleware::PRIVY_ALLOW_LIST_HINT);
    }
    let has_jwks = !auth.jwks_providers.is_empty();
    if !has_legacy_key && !has_named_keys && !has_privy && !has_jwks {
        tracing::warn!(
            "serve.auth.enabled = true but no API key or JWKS provider is configured. \
             Every /api/* request will be rejected with 401. \
             Add an API key with `roko config set serve.auth.api_key <secret>`, which \
             stores it as ROKO__SERVE__AUTH__API_KEY in .roko/.env, where agents cannot \
             read it; roko.toml may not hold it."
        );
    }
}

/// Refuse to start a showcase serve that is half configured (S11 §4.7, 9320).
///
/// With `[showcase] enabled = true`, serve starts only when auth is on in `enforce` mode,
/// `showcase.public_origin` is set, `passphrase_hash` (`ROKO_SHOWCASE_PASSPHRASE_HASH`) is an
/// Argon2id PHC string and no Privy app id is configured. Otherwise it would come up fail-open
/// while looking configured. The error names every rule that fails.
///
/// # Errors
///
/// Returns an error when showcase mode is on and a rule fails.
pub fn validate_showcase_mode(config: &RokoConfig, passphrase_hash: Option<&str>) -> Result<()> {
    let showcase = &config.showcase;
    if !showcase.enabled {
        return Ok(());
    }
    let auth = &config.serve.auth;
    let mut missing = Vec::new();
    if !auth.enabled {
        missing.push("serve.auth.enabled must be true");
    }
    if auth.enforcement_mode != roko_core::config::EnforcementMode::Enforce {
        missing.push("serve.auth.enforcement_mode must be \"enforce\"");
    }
    let origin = showcase.public_origin.as_deref().unwrap_or_default();
    if origin.trim().is_empty() {
        missing.push("showcase.public_origin must be set");
    }
    if !passphrase_hash.is_some_and(|hash| hash.starts_with("$argon2id$")) {
        missing
            .push("ROKO_SHOWCASE_PASSPHRASE_HASH must hold an Argon2id PHC string ($argon2id$...)");
    }
    if auth.privy_app_id.is_some() {
        missing.push("serve.auth.privy_app_id must not be set");
    }
    if missing.is_empty() {
        return Ok(());
    }
    anyhow::bail!(
        "showcase mode is half configured, so serve will not start: {}",
        missing.join("; ")
    );
}

pub fn validate_bind_safety(addr: &str, serve: &ServeConfig) -> Result<()> {
    if is_loopback_addr(addr) || serve.auth.enabled {
        return Ok(());
    }

    if serve.acknowledge_public_risk {
        warn!(
            addr = %addr,
            "binding to a public address without authentication; all routes will be network-accessible"
        );
        return Ok(());
    }

    anyhow::bail!(
        "Public bind requires `serve.auth.enabled = true` or `serve.acknowledge_public_risk = true`.\n\
         Attempted to bind to: {addr}\n\
         Set `[serve] auth.enabled = true` for authenticated public access, or\n\
         set `[serve] acknowledge_public_risk = true` to proceed anyway."
    );
}

/// Run the HTTP server against an already constructed [`AppState`].
///
/// # Errors
///
/// Returns an error if the listener cannot bind to `bind:port` or if the
/// Axum server exits with an error.
pub async fn run_server_with_state(state: Arc<AppState>, bind: &str, port: u16) -> Result<()> {
    // Validation is the first operation: an invalid caller-constructed state
    // must not restore snapshots, start workers/watchers, or touch a listener.
    let mut roko_config = state.load_roko_config().as_ref().clone();
    normalize_serve_dispatch_config(&mut roko_config)?;
    roko_config.server.bind = bind.to_string();
    state.store_roko_config(roko_config.clone());

    let addr = format!("{bind}:{port}");
    validate_bind_safety(&addr, &roko_config.serve)?;
    let passphrase_hash = std::env::var(PASSPHRASE_HASH_ENV).ok();
    validate_showcase_mode(&roko_config, passphrase_hash.as_deref())?;
    state.configure_listener_security(bind, roko_config.serve.auth.enabled);
    if !roko_config.serve.auth.enabled {
        tracing::warn!(
            "roko serve is running WITHOUT authentication. Set [serve.auth] enabled = true in roko.toml to require API keys."
        );
    }
    warn_if_auth_misconfigured(&roko_config.serve.auth);
    if let Err(err) = state.restore_snapshot().await {
        warn!(error = %err, "failed to restore server state snapshot; starting fresh");
    }
    let _github_event_subscriber = events::start_github_event_subscriber(Arc::clone(&state));
    let _telemetry_producer_bridge = start_telemetry_producer_bridge(Arc::clone(&state));
    start_builtin_event_sources(Arc::clone(&state), roko_config.clone());
    let _trigger_runtime = trigger_runtime::ensure_trigger_runtime(&state).await;
    let _config_watcher = config_watcher::start_config_watcher(Arc::clone(&state));
    // Both bridges share a BridgeDedup so they can run simultaneously without
    // creating a feedback loop (EventBus -> StateHub -> EventBus -> ...).
    let bridge_dedup = BridgeDedup::new();
    let _state_hub_bridge = start_state_hub_bridge(Arc::clone(&state), bridge_dedup.clone());
    let _orchestrator_bridge =
        start_orchestrator_event_bridge_dedup(Arc::clone(&state), bridge_dedup);
    let _state_saver = start_state_snapshot_saver(Arc::clone(&state));
    let _cascade_router_saver = start_cascade_router_saver(Arc::clone(&state));
    let _job_runner = job_runner::start_job_runner(Arc::clone(&state));
    let _cold_archival = start_cold_archival_timer(Arc::clone(&state));
    let router = build_server_router(
        Arc::clone(&state),
        &roko_config.server.cors_origins,
        roko_config.server.unsafe_public_cors,
        roko_config.serve.auth.clone(),
    );
    let trigger_tls = trigger_tls::load(state.as_ref()).await?;
    let listener = TcpListener::bind(&addr)
        .await
        .with_context(|| format!("bind to {addr}"))?;
    if let Ok(local_addr) = listener.local_addr() {
        state
            .terminal_sessions
            .configure_server_env_from_addr(local_addr, &roko_config);
    }
    let telemetry_observer = telemetry_observer::start_periodic_telemetry_observer(state.as_ref());

    let scheme = if trigger_tls.is_some() {
        "https"
    } else {
        "http"
    };
    info!("roko server listening on {scheme}://{addr}");
    info!("workdir: {}", state.workdir.display());

    let serve_result = if let Some(trigger_tls) = trigger_tls {
        trigger_tls::serve(listener, router, state.cancel.clone(), trigger_tls).await
    } else {
        serve_until_cancelled(listener, router, Arc::clone(&state)).await
    };
    if let Err(error) = state.runtime_feeds.stop_all().await {
        warn!(%error, "one or more runtime feeds failed to stop cleanly");
    }
    state.cancel.cancel();
    if let Err(error) = telemetry_observer.await {
        warn!(%error, "periodic telemetry observer join failed");
    }
    serve_result?;

    info!("server stopped");
    Ok(())
}

fn normalize_serve_dispatch_config(config: &mut RokoConfig) -> Result<()> {
    roko_core::config::loader::normalize_and_validate_dispatch_models(config)
        .context("validate model configuration before server startup")
}

fn build_server_router(
    state: Arc<AppState>,
    cors_origins: &[String],
    unsafe_public_cors: bool,
    api_auth: roko_core::config::ServeAuthConfig,
) -> axum::Router {
    // `routes::build_router` currently installs only the top-level SPA fallback.
    // Reset it here so the final fallback can distinguish API/WS typos from browser routes.
    let auth_enabled = api_auth.enabled;
    let api_router =
        routes::build_router(Arc::clone(&state), cors_origins, api_auth).reset_fallback();
    let fallback_router = axum::Router::new()
        .fallback(serve_api_or_spa_fallback)
        .layer(TraceLayer::new_for_http())
        .layer(routes::cors_layer(&routes::CorsPolicy {
            origins: cors_origins.to_vec(),
            unsafe_public: unsafe_public_cors,
            auth_enabled,
        }))
        .with_state(state);

    api_router.merge(fallback_router)
}

fn api_or_ws_path_requires_json_404(path: &str) -> bool {
    matches!(path, "/api" | "/ws" | "/roko-ws")
        || path.starts_with("/api/")
        || path.starts_with("/ws/")
        || path.starts_with("/roko-ws/")
}

pub(crate) async fn serve_api_or_spa_fallback(
    req: axum::extract::Request,
) -> axum::response::Response {
    let path = req.uri().path().to_string();
    if api_or_ws_path_requires_json_404(&path) {
        return (
            axum::http::StatusCode::NOT_FOUND,
            axum::Json(serde_json::json!({
                "error": "not_found",
                "message": format!("No route matches {path}"),
            })),
        )
            .into_response();
    }

    crate::embedded::serve_embedded(req).await
}

fn log_provider_credential_status(config: &RokoConfig) {
    let available = config.available_provider_ids();
    let mut missing: Vec<String> = config
        .effective_providers()
        .iter()
        .filter(|(_, p)| !config.is_provider_available(p))
        .map(|(id, _)| id.clone())
        .collect();
    missing.sort();
    info!(providers = ?available, "providers with API credentials (or CLI backends)");
    if !missing.is_empty() {
        info!(
            providers = ?missing,
            "providers missing credentials — models on these providers will not dispatch"
        );
    }
}

fn build_app_state(
    workdir: PathBuf,
    runtime: Arc<dyn CliRuntime>,
    mut roko_config: RokoConfig,
    state_hub: Option<crate::SharedStateHub>,
    metrics: Option<Arc<roko_core::obs::metrics::MetricRegistry>>,
    launch_token: Option<String>,
) -> anyhow::Result<AppState> {
    // Auto-enable auth when a stored Privy credential exists. Privy JWTs
    // themselves are accepted only when the operator sets
    // `serve.auth.privy_app_id` and an allow-list (see `try_privy_jwt`).
    if !roko_config.serve.auth.enabled {
        // Only auto-enable auth for non-loopback binds. Local dev (127.0.0.1 /
        // localhost) should respect the explicit `enabled = false` in roko.toml.
        let bind = &roko_config.server.bind;
        let is_loopback =
            bind == "127.0.0.1" || bind == "::1" || bind.eq_ignore_ascii_case("localhost");
        if !is_loopback {
            if let Ok(Some(cred)) = load_stored_credential() {
                if cred.get("method").and_then(|v| v.as_str()) == Some("privy") {
                    info!("Privy credential found — enabling auth for public bind");
                    roko_config.serve.auth.enabled = true;
                }
            }
        }
    }
    log_provider_credential_status(&roko_config);
    let deploy_backend = create_deploy_backend(&roko_config);
    let mut state = match state_hub {
        Some(state_hub) => {
            AppState::new_with_state_hub(workdir, runtime, roko_config, deploy_backend, state_hub)?
        }
        None => AppState::new(workdir, runtime, roko_config, deploy_backend)?,
    };

    // If the caller supplied a shared MetricRegistry (E09-T03), replace the
    // one that AppState created so the runtime and the /metrics endpoint
    // expose the same counters.
    if let Some(shared_metrics) = metrics {
        roko_core::obs::metrics::register_standard_metrics(&shared_metrics);
        crate::state::register_observability_foundation_metrics(&shared_metrics);
        state.metrics = shared_metrics;
    }

    // Install the ephemeral launch token for the local-access auth flow.
    // Replaces the default `LocalAccess::new(None)` set in AppState::new.
    if launch_token.is_some() {
        state.local_access = crate::state::LocalAccess::new(launch_token);
    }

    // AppState loaded the cascade router every surface shares (bug-012303)
    // from the persisted snapshot, so gateway selection reuses the learned
    // bandit state instead of rebuilding it on the first request.
    if let Some(router) = state.cascade_router.get_mut().as_ref() {
        let router_path = state.layout.cascade_router_path();
        let observations = router.total_observations();
        if observations > 0 {
            info!(
                observations = observations,
                path = %router_path.display(),
                "loaded persisted CascadeRouter"
            );
        } else {
            debug!(path = %router_path.display(), "initialized fresh CascadeRouter");
        }
    }

    let _ = state.state_hub.bootstrap_from_workdir(&state.workdir);
    // Hydrate remaining disk-backed surfaces as part of the recovered
    // generation. This intentionally emits no live events and preserves
    // restart provenance.
    let jobs = scan_marketplace_jobs(&state.workdir);
    if !jobs.is_empty() {
        info!(
            count = jobs.len(),
            "loaded existing marketplace jobs from disk"
        );
    }
    let knowledge = scan_knowledge_entries(&state.workdir);
    if !knowledge.is_empty() {
        info!(
            count = knowledge.len(),
            "loaded existing knowledge entries from neuro store"
        );
    }
    state.state_hub.hydrate_recovered_snapshot(|snapshot| {
        snapshot.marketplace_jobs = jobs;
        snapshot.knowledge_entries = knowledge;
    });

    // Seed connector and feed registries with default entries so routes
    // return real data instead of empty arrays (audit finding A3).
    seed_default_registries(&state);

    Ok(state)
}

/// Populate the connector and feed registries with default entries that
/// reflect the actual on-disk data files. Called once during server startup
/// before the `AppState` is shared behind an `Arc`.
///
/// Connectors registered:
/// - **filesystem**: the local `.roko/` data directory (`Database` kind)
/// - **neuro-store**: the durable knowledge store (`Database` kind)
///
/// Feeds registered:
/// - **signals**: `.roko/signals.jsonl` — raw signal log (`Raw` kind)
/// - **episodes**: `.roko/episodes.jsonl` — agent turn episodes (`Raw` kind)
/// - **efficiency**: `.roko/learn/efficiency.jsonl` — per-turn metrics (`Derived` kind)
/// - **knowledge**: neuro knowledge store entries (`Composite` kind)
fn seed_default_registries(state: &AppState) {
    // Use block_in_place so blocking_write doesn't panic inside a tokio runtime.
    // This is safe because build_app_state is called once at startup.
    tokio::task::block_in_place(|| seed_default_registries_inner(state));
}

fn seed_default_registries_inner(state: &AppState) {
    let now = chrono::Utc::now();
    let layout = &state.layout;

    // ── Connectors ────────────────────────────────────────────────────
    let mut connectors = state.connectors.blocking_write();

    let roko_root = layout.root().to_string_lossy().to_string();
    connectors.register(ConnectorInfo {
        name: "filesystem".to_string(),
        kind: ConnectorKind::Database,
        health: ConnectorHealth {
            status: ConnectorStatus::Connected,
            latency_ms: 0,
            last_check: now,
        },
        created_at: now,
        metadata: serde_json::json!({
            "description": "Local .roko/ data directory",
            "path": roko_root,
        }),
    });

    let neuro_path = layout.root().join("neuro");
    connectors.register(ConnectorInfo {
        name: "neuro-store".to_string(),
        kind: ConnectorKind::Database,
        health: ConnectorHealth {
            status: if neuro_path.exists() {
                ConnectorStatus::Connected
            } else {
                ConnectorStatus::Disconnected
            },
            latency_ms: 0,
            last_check: now,
        },
        created_at: now,
        metadata: serde_json::json!({
            "description": "Durable knowledge store (neuro)",
            "path": neuro_path.to_string_lossy(),
        }),
    });

    let connector_count = connectors.list().len();
    drop(connectors);

    // ── Feeds ─────────────────────────────────────────────────────────
    let mut feeds = state.feeds.blocking_write();

    let signals_path = layout.signals_path_with_fallback();
    feeds.register(FeedInfo {
        id: String::new(), // assigned by registry
        cell_id: String::new(),
        name: "signals".to_string(),
        kind: FeedKind::Raw,
        access: FeedAccess::Public,
        agent_id: "system".to_string(),
        description: "Raw signal log (.roko/signals.jsonl)".to_string(),
        schema: None,
        pricing: None,
        created_at: now,
    });

    let episodes_path = layout.episodes_path();
    feeds.register(FeedInfo {
        id: String::new(),
        cell_id: String::new(),
        name: "episodes".to_string(),
        kind: FeedKind::Raw,
        access: FeedAccess::Public,
        agent_id: "system".to_string(),
        description: "Agent turn episode log (.roko/episodes.jsonl)".to_string(),
        schema: None,
        pricing: None,
        created_at: now,
    });

    let efficiency_path = layout.efficiency_path();
    feeds.register(FeedInfo {
        id: String::new(),
        cell_id: String::new(),
        name: "efficiency".to_string(),
        kind: FeedKind::Derived,
        access: FeedAccess::Public,
        agent_id: "system".to_string(),
        description: "Per-turn efficiency metrics (.roko/learn/efficiency.jsonl)".to_string(),
        schema: None,
        pricing: None,
        created_at: now,
    });

    feeds.register(FeedInfo {
        id: String::new(),
        cell_id: String::new(),
        name: "knowledge".to_string(),
        kind: FeedKind::Composite,
        access: FeedAccess::Public,
        agent_id: "system".to_string(),
        description: "Durable knowledge entries from the neuro store".to_string(),
        schema: None,
        pricing: None,
        created_at: now,
    });

    let feed_count = feeds.list().len();
    drop(feeds);

    info!(
        connectors = connector_count,
        feeds = feed_count,
        signals_path = %signals_path.display(),
        episodes_path = %episodes_path.display(),
        efficiency_path = %efficiency_path.display(),
        "seeded default connector and feed registries"
    );
}

/// Scan `.roko/jobs/*.json` and return a vec of `MarketplaceJob`.
fn scan_marketplace_jobs(workdir: &Path) -> Vec<roko_core::MarketplaceJob> {
    let dir = workdir.join(".roko").join("jobs");
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };
    let mut jobs = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let data = match std::fs::read_to_string(&path) {
            Ok(d) => d,
            Err(_) => continue,
        };
        match serde_json::from_str::<roko_core::MarketplaceJob>(&data) {
            Ok(job) => jobs.push(job),
            Err(err) => {
                debug!(
                    path = %path.display(),
                    error = %err,
                    "skipping malformed job file during startup scan"
                );
            }
        }
    }
    jobs.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(b.id.cmp(&a.id)));
    jobs
}

/// Load knowledge entries from the neuro JSONL store and project them into
/// lightweight `KnowledgeBrowseEntry` summaries for the dashboard snapshot.
fn scan_knowledge_entries(
    workdir: &Path,
) -> Vec<roko_core::dashboard_snapshot::KnowledgeBrowseEntry> {
    let store = roko_neuro::knowledge_store::KnowledgeStore::for_workdir(workdir);
    let entries = match store.query("*", 200) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };
    entries
        .into_iter()
        .map(|entry| {
            let preview = if entry.content.len() > 200 {
                format!("{}…", &entry.content[..200])
            } else {
                entry.content.clone()
            };
            let tier_str = match entry.tier {
                roko_neuro::KnowledgeTier::Transient => "transient",
                roko_neuro::KnowledgeTier::Working => "working",
                roko_neuro::KnowledgeTier::Consolidated => "consolidated",
                roko_neuro::KnowledgeTier::Persistent => "persistent",
            };
            roko_core::dashboard_snapshot::KnowledgeBrowseEntry {
                id: entry.id,
                kind: entry.kind.as_str().to_string(),
                content_preview: preview,
                confidence: entry.confidence,
                tier: tier_str.to_string(),
                tags: entry.tags,
                created_at: entry.created_at,
                frozen: false,
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Bridge deduplication
// ---------------------------------------------------------------------------

/// Shared dedup state for the bidirectional EventBus <-> StateHub bridges.
///
/// When both `start_state_hub_bridge` (EventBus -> StateHub) and
/// `start_orchestrator_event_bridge` (StateHub -> EventBus) run simultaneously,
/// a naive setup creates an infinite loop:
///
/// ```text
/// REST handler -> EventBus -> Bridge A -> StateHub -> Bridge B -> EventBus -> ...
/// ```
///
/// `BridgeDedup` breaks the cycle by tracking which sequence numbers on each
/// bus were produced by a bridge. The other bridge skips those seqs.
///
/// The sets are bounded: once an entry is consumed (checked + removed) or the
/// set exceeds `MAX_TRACKED`, the oldest entries are drained.
#[derive(Clone)]
struct BridgeDedup {
    /// StateHub seqs produced by Bridge A (state_hub_bridge).
    /// Bridge B checks this before converting Dashboard -> Server.
    dashboard_seqs: Arc<StdMutex<HashSet<u64>>>,
    /// EventBus seqs produced by Bridge B (orchestrator_event_bridge).
    /// Bridge A checks this before converting Server -> Dashboard.
    server_seqs: Arc<StdMutex<HashSet<u64>>>,
}

impl BridgeDedup {
    /// Maximum tracked seqs per direction before we drain.
    const MAX_TRACKED: usize = 4096;

    fn new() -> Self {
        Self {
            dashboard_seqs: Arc::new(StdMutex::new(HashSet::new())),
            server_seqs: Arc::new(StdMutex::new(HashSet::new())),
        }
    }

    /// Record a StateHub seq as bridge-produced. Called by Bridge A.
    fn mark_dashboard_seq(&self, seq: u64) {
        let mut set = self
            .dashboard_seqs
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if set.len() >= Self::MAX_TRACKED {
            set.clear();
        }
        set.insert(seq);
    }

    /// Check if a StateHub seq was bridge-produced. Called by Bridge B.
    /// Returns true if the seq was bridged (and should be skipped).
    fn is_bridged_dashboard_seq(&self, seq: u64) -> bool {
        let mut set = self
            .dashboard_seqs
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        set.remove(&seq)
    }

    /// Record an EventBus seq as bridge-produced. Called by Bridge B.
    fn mark_server_seq(&self, seq: u64) {
        let mut set = self.server_seqs.lock().unwrap_or_else(|e| e.into_inner());
        if set.len() >= Self::MAX_TRACKED {
            set.clear();
        }
        set.insert(seq);
    }

    /// Check if an EventBus seq was bridge-produced. Called by Bridge A.
    /// Returns true if the seq was bridged (and should be skipped).
    fn is_bridged_server_seq(&self, seq: u64) -> bool {
        let mut set = self.server_seqs.lock().unwrap_or_else(|e| e.into_inner());
        set.remove(&seq)
    }
}

/// Fan durable server lifecycle transitions into the event-oriented Lens runtime.
///
/// This bridge deliberately consumes only one authoritative representation for
/// each transition. In particular, trigger observations come from the
/// post-persistence `TriggerLifecycle` event, never the compatibility
/// `TriggerFired` event, so one firing cannot be counted twice.
fn start_telemetry_producer_bridge(state: Arc<AppState>) -> JoinHandle<()> {
    let mut receiver = state.event_bus.subscribe();
    tokio::spawn(async move {
        loop {
            let envelope = tokio::select! {
                _ = state.cancel.cancelled() => break,
                event = receiver.recv() => match event {
                    Ok(envelope) => envelope,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                        warn!(skipped, "telemetry producer bridge lagged behind server event bus");
                        continue;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                },
            };

            let Some((event, ancestry)) = server_event_to_observable(&envelope.payload) else {
                continue;
            };
            for error in state.state_hub.emit_observable(&event, &ancestry) {
                warn!(%error, "server lifecycle telemetry delivery failed");
            }
        }
    })
}

fn server_event_to_observable(
    event: &ServerEvent,
) -> Option<(roko_core::ObservableEvent, Vec<roko_core::LensScope>)> {
    use roko_core::trigger::TriggerEventKind;
    use roko_core::{LensScope, ObservableEvent, Verdict};

    let observation = match event {
        ServerEvent::TriggerLifecycle { event } => {
            let observable = match event.kind {
                TriggerEventKind::Armed => ObservableEvent::TriggerArmed {
                    trigger: event.trigger_name.clone(),
                },
                TriggerEventKind::Fired => ObservableEvent::TriggerFired {
                    trigger: event.trigger_name.clone(),
                    graph: event.graph.clone(),
                },
                TriggerEventKind::Disarmed => ObservableEvent::TriggerDisarmed {
                    trigger: event.trigger_name.clone(),
                },
                _ => return None,
            };
            (observable, vec![LensScope::Graph(event.graph.clone())])
        }
        ServerEvent::GateResult {
            plan_id,
            task_id,
            gate,
            rung,
            passed,
        } => {
            let gate = gate_label_with_rung(gate, *rung);
            let verdict = if *passed {
                Verdict::pass(gate)
            } else {
                Verdict::fail(gate, "gate failed")
            };
            (
                ObservableEvent::VerifyPostResult {
                    block: task_id.clone(),
                    verdict,
                    reward: if *passed { 1.0 } else { 0.0 },
                    // `ServerEvent::GateResult` carries no evidence references;
                    // keep this empty instead of manufacturing identifiers.
                    evidence: Vec::new(),
                },
                vec![
                    LensScope::Cell(task_id.clone()),
                    LensScope::Graph(plan_id.clone()),
                ],
            )
        }
        _ => return None,
    };
    Some(observation)
}

fn start_state_hub_bridge(state: Arc<AppState>, dedup: BridgeDedup) -> JoinHandle<()> {
    let mut rx = state.event_bus.subscribe();
    let sender = state.state_hub.sender();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    // Skip events that were placed on the EventBus by the
                    // orchestrator bridge (Bridge B) to break the cycle.
                    if dedup.is_bridged_server_seq(envelope.seq) {
                        continue;
                    }
                    if let Some(event) = server_event_to_dashboard(&envelope.payload) {
                        let dashboard_seq = sender.publish(event);
                        dedup.mark_dashboard_seq(dashboard_seq);
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    warn!(skipped, "state hub bridge lagged behind server event bus");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    })
}

fn server_event_to_dashboard(event: &ServerEvent) -> Option<roko_core::DashboardEvent> {
    use roko_core::DashboardEvent;

    match event {
        ServerEvent::PlanStarted { plan_id } => Some(DashboardEvent::PlanStarted {
            plan_id: plan_id.clone(),
            tasks_total: 0,
        }),
        ServerEvent::PlanSetLoaded { plans } => Some(DashboardEvent::PlanSetLoaded {
            plans: plans.clone(),
        }),
        ServerEvent::PlanCompleted { plan_id, success } => Some(DashboardEvent::PlanCompleted {
            plan_id: plan_id.clone(),
            success: *success,
        }),
        ServerEvent::AgentSpawned {
            agent_id,
            role,
            model,
        } => Some(DashboardEvent::AgentSpawned {
            agent_id: agent_id.clone(),
            plan_id: String::new(),
            task_id: String::new(),
            attempt: 0,
            role: role.clone(),
            model: dashboard_model_label(model, agent_id),
            provider: String::new(),
        }),
        ServerEvent::AgentOutput {
            agent_id, content, ..
        } => Some(DashboardEvent::AgentOutput {
            agent_id: agent_id.clone(),
            plan_id: String::new(),
            task_id: String::new(),
            attempt: 0,
            content: content.clone(),
        }),
        ServerEvent::GateResult {
            plan_id,
            task_id,
            gate,
            rung,
            passed,
        } => Some(DashboardEvent::GateResult {
            plan_id: plan_id.clone(),
            task_id: task_id.clone(),
            gate: gate_label_with_rung(gate, *rung),
            passed: *passed,
            output_text: None,
        }),
        ServerEvent::Execution { plan_id, event } => match event {
            ExecutionEvent::TaskStarted {
                task_id,
                title,
                phase,
            } => Some(DashboardEvent::TaskStarted {
                plan_id: plan_id.clone(),
                task_id: task_id.clone(),
                title: title.clone(),
                phase: phase.clone(),
            }),
            ExecutionEvent::TaskCompleted { task_id, outcome } => {
                Some(DashboardEvent::TaskCompleted {
                    plan_id: plan_id.clone(),
                    task_id: task_id.clone(),
                    outcome: outcome.clone(),
                })
            }
            ExecutionEvent::TaskPhaseChanged {
                task_id,
                old_phase,
                new_phase,
            } => Some(DashboardEvent::TaskPhaseChanged {
                plan_id: plan_id.clone(),
                task_id: task_id.clone(),
                old_phase: old_phase.clone(),
                new_phase: new_phase.clone(),
            }),
            ExecutionEvent::GateResult {
                task_id,
                gate,
                passed,
                ..
            } => Some(DashboardEvent::GateResult {
                plan_id: plan_id.clone(),
                task_id: task_id.clone(),
                gate: gate.clone(),
                passed: *passed,
                output_text: None,
            }),
            ExecutionEvent::TaskBlocked {
                task_id,
                title,
                blocked_by,
                reason,
            } => Some(DashboardEvent::TaskBlocked {
                plan_id: plan_id.clone(),
                task_id: task_id.clone(),
                title: title.clone(),
                blocked_by: blocked_by.clone(),
                reason: reason.clone(),
            }),
            // Plans start and finish through the top-level plan events;
            // re-plans and watcher alerts have no dashboard event.
            ExecutionEvent::PlanStarted
            | ExecutionEvent::PlanCompleted { .. }
            | ExecutionEvent::ReplanTriggered { .. }
            | ExecutionEvent::WatcherAlert { .. } => None,
        },
        ServerEvent::PhaseTransition { plan_id, from, to } => {
            Some(DashboardEvent::PhaseTransition {
                plan_id: plan_id.clone(),
                from: from.clone(),
                to: to.clone(),
            })
        }
        ServerEvent::EfficiencyEvent {
            plan_id,
            task_id,
            metric,
            value,
        } => Some(DashboardEvent::EfficiencyEvent {
            plan_id: plan_id.clone(),
            task_id: task_id.clone(),
            metric: metric.clone(),
            value: *value,
        }),
        ServerEvent::JobExecutionStarted {
            job_id,
            job_type,
            agent_id,
        } => Some(DashboardEvent::JobExecutionStarted {
            job_id: job_id.clone(),
            job_type: job_type.clone(),
            agent_id: agent_id.clone(),
        }),
        ServerEvent::JobProgress {
            job_id,
            percent,
            message,
        } => Some(DashboardEvent::JobProgress {
            job_id: job_id.clone(),
            percent: *percent,
            message: message.clone(),
        }),
        ServerEvent::Error { message } => Some(DashboardEvent::Error {
            message: message.clone(),
        }),
        // Map one-shot runs as ephemeral plans so the TUI's plan/task views
        // show them: the plan the run's own task and agent events name.
        ServerEvent::RunStarted { run_id, .. } => Some(DashboardEvent::PlanStarted {
            plan_id: run_plan_id(run_id),
            tasks_total: 0,
        }),
        ServerEvent::RunCompleted {
            run_id, success, ..
        } => Some(DashboardEvent::PlanCompleted {
            plan_id: run_plan_id(run_id),
            success: *success,
        }),
        // Map agent lifecycle events from the supervisor.
        ServerEvent::AgentStarted { agent_id, .. } => Some(DashboardEvent::AgentSpawned {
            agent_id: agent_id.clone(),
            plan_id: String::new(),
            task_id: String::new(),
            attempt: 0,
            role: String::new(),
            model: dashboard_model_label("", agent_id),
            provider: String::new(),
        }),
        ServerEvent::AgentStopped { agent_id, .. } => Some(DashboardEvent::AgentCompleted {
            agent_id: agent_id.clone(),
            plan_id: String::new(),
            task_id: String::new(),
            attempt: 0,
        }),
        // Bridge bench events so the dashboard TUI / SSE clients see bench activity.
        ServerEvent::BenchRunStarted { bench_id, .. } => Some(DashboardEvent::PlanStarted {
            plan_id: format!("bench-{bench_id}"),
            tasks_total: 0,
        }),
        ServerEvent::BenchTaskStarted {
            bench_id,
            task_id,
            task_name,
            ..
        } => Some(DashboardEvent::TaskStarted {
            plan_id: format!("bench-{bench_id}"),
            task_id: task_id.clone(),
            title: task_name.clone(),
            phase: "dispatch".to_string(),
        }),
        ServerEvent::BenchTaskCompleted {
            bench_id, task_id, ..
        } => Some(DashboardEvent::TaskCompleted {
            plan_id: format!("bench-{bench_id}"),
            task_id: task_id.clone(),
            outcome: "completed".to_string(),
        }),
        ServerEvent::BenchRunCompleted { bench_id, .. } => Some(DashboardEvent::PlanCompleted {
            plan_id: format!("bench-{bench_id}"),
            success: true,
        }),
        ServerEvent::BenchProgress {
            bench_id,
            completed,
            total,
            cost_so_far,
        } => Some(DashboardEvent::EfficiencyEvent {
            plan_id: format!("bench-{bench_id}"),
            task_id: format!("{completed}/{total}"),
            metric: "cost_usd".to_string(),
            value: *cost_so_far,
        }),
        ServerEvent::ChainBlock {
            number,
            hash,
            parent_hash,
            timestamp,
            gas_used,
            gas_limit,
            tx_count,
            base_fee_per_gas,
        } => Some(DashboardEvent::ChainBlock {
            number: *number,
            hash: hash.clone(),
            parent_hash: parent_hash.clone(),
            timestamp: *timestamp,
            gas_used: *gas_used,
            gas_limit: *gas_limit,
            tx_count: *tx_count,
            base_fee_per_gas: *base_fee_per_gas,
        }),
        ServerEvent::ChainTx {
            block_number,
            tx_hash,
            from,
            to,
            value_wei,
            gas_used,
            method_sig,
            success,
        } => Some(DashboardEvent::ChainTx {
            block_number: *block_number,
            tx_hash: tx_hash.clone(),
            from: from.clone(),
            to: to.clone(),
            value_wei: value_wei.clone(),
            gas_used: *gas_used,
            method_sig: method_sig.clone(),
            success: *success,
        }),
        ServerEvent::ChainContractEvent {
            block_number,
            tx_hash,
            log_index,
            contract,
            event_name,
            decoded,
            ..
        } => Some(DashboardEvent::ChainContractEvent {
            block_number: *block_number,
            tx_hash: tx_hash.clone(),
            log_index: *log_index,
            contract: contract.clone(),
            event_name: event_name.clone(),
            decoded: decoded.clone(),
        }),
        ServerEvent::FeedTick {
            agent_id,
            feed_id,
            topic,
            payload,
            timestamp_ms,
        } => Some(DashboardEvent::FeedTick {
            agent_id: agent_id.clone(),
            feed_id: feed_id.clone(),
            topic: topic.clone(),
            payload: payload.clone(),
            timestamp_ms: *timestamp_ms,
        }),
        ServerEvent::FeedAgentOnline {
            agent_id,
            name,
            feed_count,
        } => Some(DashboardEvent::FeedAgentOnline {
            agent_id: agent_id.clone(),
            name: name.clone(),
            feed_count: *feed_count,
        }),
        ServerEvent::FeedAgentOffline { agent_id } => Some(DashboardEvent::FeedAgentOffline {
            agent_id: agent_id.clone(),
        }),
        // Not bridged (bug-bfdb9a): the dashboard has no event for these.
        // Task progress reaches it through `Execution`; the top-level
        // `TaskStarted`, `TaskCompleted` and `TaskFailed` have no producer.
        // The match lists every variant, so a new one fails to compile until
        // it is bridged or listed here.
        ServerEvent::AgentTrace { .. }
        | ServerEvent::Episode { .. }
        | ServerEvent::InferenceStarted { .. }
        | ServerEvent::InferenceCompleted { .. }
        | ServerEvent::InferenceFailed { .. }
        | ServerEvent::SomaticMarkerFired { .. }
        | ServerEvent::OperationStarted { .. }
        | ServerEvent::OperationCompleted { .. }
        | ServerEvent::DeploymentCreated { .. }
        | ServerEvent::DeploymentReady { .. }
        | ServerEvent::DeploymentFailed { .. }
        | ServerEvent::DeploymentTornDown { .. }
        | ServerEvent::JobCreated { .. }
        | ServerEvent::JobPostedToCandidate { .. }
        | ServerEvent::JobUpdated { .. }
        | ServerEvent::JobTransitioned { .. }
        | ServerEvent::WorkerTaskStarted { .. }
        | ServerEvent::WorkerTaskCompleted { .. }
        | ServerEvent::JobAgentOutput { .. }
        | ServerEvent::ChainTriageResult { .. }
        | ServerEvent::HeartbeatReceived { .. }
        | ServerEvent::TaskStarted { .. }
        | ServerEvent::TaskCompleted { .. }
        | ServerEvent::TaskFailed { .. }
        | ServerEvent::JobSubmitted { .. }
        | ServerEvent::JobEvaluated { .. }
        | ServerEvent::JobStateChanged { .. }
        | ServerEvent::Heartbeat { .. }
        | ServerEvent::TriggerFired { .. }
        | ServerEvent::TriggerLifecycle { .. }
        | ServerEvent::ServerShutdown
        | ServerEvent::WebhookReceived { .. }
        | ServerEvent::VisionLoopIteration { .. }
        | ServerEvent::VisionLoopCompleted { .. }
        | ServerEvent::ConfigReloaded { .. }
        | ServerEvent::StrategyReloaded { .. }
        | ServerEvent::BenchLearningEvent { .. }
        | ServerEvent::BenchRegressionReport { .. }
        | ServerEvent::MatrixRunStarted { .. }
        | ServerEvent::MatrixLaneCompleted { .. }
        | ServerEvent::MatrixRunCompleted { .. }
        | ServerEvent::BenchGateVerdict { .. }
        | ServerEvent::BenchTokenVelocity { .. }
        | ServerEvent::BenchAgentOutput { .. }
        | ServerEvent::SweRunStarted { .. }
        | ServerEvent::SweInstanceCompleted { .. }
        | ServerEvent::SweRunCompleted { .. }
        | ServerEvent::ChainLogObserved { .. }
        | ServerEvent::ChainFinalityUpdated { .. }
        | ServerEvent::ChainReorg { .. } => None,
    }
}

fn dashboard_model_label(model: &str, fallback: &str) -> String {
    let model = model.trim();
    if model.is_empty() {
        fallback.to_string()
    } else {
        model.to_string()
    }
}

/// The dashboard plan of the one-shot run `run_id`: `run-` and the first
/// eight characters of its id. The run's task and agent events and the plan
/// start and end the bridge makes of its `RunStarted` and `RunCompleted` name
/// the same plan (gap-8a1fb3).
pub(crate) fn run_plan_id(run_id: &str) -> String {
    format!("run-{}", run_id.get(..8).unwrap_or(run_id))
}

/// The dashboard label of a gate result that `ServerEvent::GateResult` sends
/// with its rung apart: the Graph path's `verify[i:phase]` form, so the rung
/// survives the bridge (gap-8a1fb3). A label that already names its rung
/// keeps it.
fn gate_label_with_rung(gate: &str, rung: u32) -> String {
    if gate_label_rung(gate).is_some() {
        gate.to_string()
    } else if gate.is_empty() {
        format!("verify[{rung}]")
    } else {
        format!("verify[{rung}:{gate}]")
    }
}

/// The rung a gate label names, as in `verify[3]` or `verify[3:test]`.
fn gate_label_rung(gate: &str) -> Option<u32> {
    let (_, index) = gate.strip_suffix(']')?.rsplit_once('[')?;
    index
        .split_once(':')
        .map_or(index, |(rung, _)| rung)
        .parse()
        .ok()
}

/// Bridge orchestrator events (`StateHub` -> `EventBus`) so SSE/WS clients
/// see gate results, task completions, and other events from `roko plan run`.
///
/// This is the reverse direction of [`start_state_hub_bridge`] which pushes
/// REST-triggered `ServerEvent`s into the `StateHub` for the TUI.
///
/// # Deduplication
///
/// When both bridges run simultaneously, a naive setup creates an infinite
/// loop: REST -> EventBus -> Bridge A -> StateHub -> Bridge B -> EventBus -> ...
///
/// The `dedup` parameter carries shared seq tracking: each bridge marks the
/// seqs it produces on the destination bus, and the other bridge skips those
/// seqs. Pass the same [`BridgeDedup`] instance to both bridges.
///
/// For backward compatibility with callers that only run this bridge (no
/// `start_state_hub_bridge`), a default no-dedup overload is provided.
#[doc(hidden)]
pub fn start_orchestrator_event_bridge(state: Arc<AppState>) -> JoinHandle<()> {
    start_orchestrator_event_bridge_dedup(state, BridgeDedup::new())
}

/// Like [`start_orchestrator_event_bridge`] but with shared dedup state.
fn start_orchestrator_event_bridge_dedup(
    state: Arc<AppState>,
    dedup: BridgeDedup,
) -> JoinHandle<()> {
    let mut rx = state.state_hub.subscribe_events();
    let bus = state.event_bus.clone();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    // Skip events that were placed on the StateHub by the
                    // state-hub bridge (Bridge A) to break the cycle.
                    if dedup.is_bridged_dashboard_seq(envelope.seq) {
                        continue;
                    }
                    if let Some(server_event) = dashboard_event_to_server(&envelope.payload) {
                        let server_seq = bus.publish(server_event);
                        dedup.mark_server_seq(server_seq);
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    warn!(n, "orchestrator bridge lagged behind state hub");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    })
}

/// Convert a [`DashboardEvent`] to a [`ServerEvent`] for SSE/WS delivery.
/// Inverse of [`server_event_to_dashboard`].
fn dashboard_event_to_server(event: &roko_core::DashboardEvent) -> Option<ServerEvent> {
    use roko_core::DashboardEvent;
    match event {
        DashboardEvent::PlanStarted { plan_id, .. } => Some(ServerEvent::PlanStarted {
            plan_id: plan_id.clone(),
        }),
        DashboardEvent::PlanCompleted { plan_id, success } => Some(ServerEvent::PlanCompleted {
            plan_id: plan_id.clone(),
            success: *success,
        }),
        DashboardEvent::PlanSetLoaded { plans } => Some(ServerEvent::PlanSetLoaded {
            plans: plans.clone(),
        }),
        DashboardEvent::TaskStarted {
            plan_id,
            task_id,
            title,
            phase,
        } => Some(ServerEvent::Execution {
            plan_id: plan_id.clone(),
            event: ExecutionEvent::TaskStarted {
                task_id: task_id.clone(),
                title: title.clone(),
                phase: phase.clone(),
            },
        }),
        DashboardEvent::TaskCompleted {
            plan_id,
            task_id,
            outcome,
        } => Some(ServerEvent::Execution {
            plan_id: plan_id.clone(),
            event: ExecutionEvent::TaskCompleted {
                task_id: task_id.clone(),
                outcome: outcome.clone(),
            },
        }),
        DashboardEvent::TaskPhaseChanged {
            plan_id,
            task_id,
            old_phase,
            new_phase,
        } => Some(ServerEvent::Execution {
            plan_id: plan_id.clone(),
            event: ExecutionEvent::TaskPhaseChanged {
                task_id: task_id.clone(),
                old_phase: old_phase.clone(),
                new_phase: new_phase.clone(),
            },
        }),
        DashboardEvent::AgentSpawned {
            agent_id,
            role,
            model,
            ..
        } => Some(ServerEvent::AgentSpawned {
            agent_id: agent_id.clone(),
            role: role.clone(),
            model: model.clone(),
        }),
        DashboardEvent::AgentOutput {
            agent_id, content, ..
        } => Some(ServerEvent::AgentOutput {
            agent_id: agent_id.clone(),
            run_id: None,
            content: content.clone(),
            done: false,
            metadata: None,
        }),
        DashboardEvent::GateResult {
            plan_id,
            task_id,
            gate,
            passed,
            ..
        } => Some(ServerEvent::GateResult {
            plan_id: plan_id.clone(),
            task_id: task_id.clone(),
            gate: gate.clone(),
            rung: gate_label_rung(gate).unwrap_or(0),
            passed: *passed,
        }),
        DashboardEvent::PhaseTransition { plan_id, from, to } => {
            Some(ServerEvent::PhaseTransition {
                plan_id: plan_id.clone(),
                from: from.clone(),
                to: to.clone(),
            })
        }
        DashboardEvent::EfficiencyEvent {
            plan_id,
            task_id,
            metric,
            value,
        } => Some(ServerEvent::EfficiencyEvent {
            plan_id: plan_id.clone(),
            task_id: task_id.clone(),
            metric: metric.clone(),
            value: *value,
        }),
        DashboardEvent::JobExecutionStarted {
            job_id,
            job_type,
            agent_id,
        } => Some(ServerEvent::JobExecutionStarted {
            job_id: job_id.clone(),
            job_type: job_type.clone(),
            agent_id: agent_id.clone(),
        }),
        DashboardEvent::JobProgress {
            job_id,
            percent,
            message,
        } => Some(ServerEvent::JobProgress {
            job_id: job_id.clone(),
            percent: *percent,
            message: message.clone(),
        }),
        DashboardEvent::Error { message } => Some(ServerEvent::Error {
            message: message.clone(),
        }),
        DashboardEvent::TaskBlocked {
            plan_id,
            task_id,
            title,
            blocked_by,
            reason,
        } => Some(ServerEvent::Execution {
            plan_id: plan_id.clone(),
            event: ExecutionEvent::TaskBlocked {
                task_id: task_id.clone(),
                title: title.clone(),
                blocked_by: blocked_by.clone(),
                reason: reason.clone(),
            },
        }),
        DashboardEvent::ChainBlock {
            number,
            hash,
            parent_hash,
            timestamp,
            gas_used,
            gas_limit,
            tx_count,
            base_fee_per_gas,
        } => Some(ServerEvent::ChainBlock {
            number: *number,
            hash: hash.clone(),
            parent_hash: parent_hash.clone(),
            timestamp: *timestamp,
            gas_used: *gas_used,
            gas_limit: *gas_limit,
            tx_count: *tx_count,
            base_fee_per_gas: *base_fee_per_gas,
        }),
        DashboardEvent::ChainTx {
            block_number,
            tx_hash,
            from,
            to,
            value_wei,
            gas_used,
            method_sig,
            success,
        } => Some(ServerEvent::ChainTx {
            block_number: *block_number,
            tx_hash: tx_hash.clone(),
            from: from.clone(),
            to: to.clone(),
            value_wei: value_wei.clone(),
            gas_used: *gas_used,
            method_sig: method_sig.clone(),
            success: *success,
        }),
        DashboardEvent::FeedTick {
            agent_id,
            feed_id,
            topic,
            payload,
            timestamp_ms,
        } => Some(ServerEvent::FeedTick {
            agent_id: agent_id.clone(),
            feed_id: feed_id.clone(),
            topic: topic.clone(),
            payload: payload.clone(),
            timestamp_ms: *timestamp_ms,
        }),
        DashboardEvent::FeedAgentOnline {
            agent_id,
            name,
            feed_count,
        } => Some(ServerEvent::FeedAgentOnline {
            agent_id: agent_id.clone(),
            name: name.clone(),
            feed_count: *feed_count,
        }),
        DashboardEvent::FeedAgentOffline { agent_id } => Some(ServerEvent::FeedAgentOffline {
            agent_id: agent_id.clone(),
        }),
        // Not bridged (bug-bfdb9a). `RunCompleted` names no run, which
        // `ServerEvent::RunCompleted` needs, and `ChainContractEvent` does not
        // say whether the raw log evidence was published.
        DashboardEvent::RunCompleted { .. } | DashboardEvent::ChainContractEvent { .. } => None,
        // Dashboard state the server stream has no event for: the dashboard's
        // own stream and snapshot carry it. The match lists every variant, so
        // a new one fails to compile until it is bridged or listed here.
        DashboardEvent::GateOutputLine { .. }
        | DashboardEvent::Diagnosis { .. }
        | DashboardEvent::ExperimentWinnersUpdated { .. }
        | DashboardEvent::CFactorTrendUpdated { .. }
        | DashboardEvent::ProjectionUpdated { .. }
        | DashboardEvent::EpisodeRecorded { .. }
        | DashboardEvent::TaskOutputAppended { .. }
        | DashboardEvent::EventLogEntry { .. }
        | DashboardEvent::CascadeRouterUpdated { .. }
        | DashboardEvent::GateThresholdsUpdated { .. }
        | DashboardEvent::AgentCompleted { .. }
        | DashboardEvent::MarketplaceJobsUpdated { .. }
        | DashboardEvent::KnowledgeEntriesUpdated { .. }
        | DashboardEvent::EfficiencyTrendUpdated { .. }
        | DashboardEvent::PaymentReceived { .. }
        | DashboardEvent::SettlementCompleted { .. }
        | DashboardEvent::InboxItemReceived { .. }
        | DashboardEvent::InboxApprove { .. }
        | DashboardEvent::InboxReject { .. }
        | DashboardEvent::InboxDefer { .. }
        | DashboardEvent::InboxDismiss { .. }
        | DashboardEvent::AgentHeartbeat { .. }
        | DashboardEvent::GateRungStarted { .. }
        | DashboardEvent::AffectUpdated { .. }
        | DashboardEvent::AgentTopologyUpdated { .. }
        | DashboardEvent::CriticalPathEtaUpdated { .. }
        | DashboardEvent::CostAnomaly { .. }
        | DashboardEvent::CrossCutCascade { .. }
        | DashboardEvent::LoopHealth { .. }
        | DashboardEvent::LoopTransition { .. }
        | DashboardEvent::SnapshotRebased { .. } => None,
    }
}

pub(crate) fn start_event_source_group(
    state: Arc<AppState>,
    sources: Vec<Box<dyn EventSource>>,
) -> JoinHandle<()> {
    if sources.is_empty() {
        return tokio::spawn(async {});
    }

    let cancel = CancellationToken::new();
    let cancel_for_shutdown = cancel.clone();
    let state_for_shutdown = Arc::clone(&state);
    tokio::spawn(async move {
        state_for_shutdown.cancel.cancelled().await;
        cancel_for_shutdown.cancel();
    });

    let (signal_tx, signal_rx) = mpsc::channel::<Signal>(256);
    tokio::spawn(signal_ingest_loop(
        Arc::clone(&state),
        signal_rx,
        cancel.clone(),
    ));

    for source in sources {
        let source_name = source.name().to_string();
        let source_kind = source.kind();
        let sender = signal_tx.clone();
        let cancel = cancel.clone();

        tokio::spawn(async move {
            if let Err(err) = source.start(sender, cancel).await {
                warn!(
                    source = %source_name,
                    kind = ?source_kind,
                    error = %err,
                    "event source stopped"
                );
            }
        });
    }

    tokio::spawn(async {})
}

fn start_state_snapshot_saver(state: Arc<AppState>) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            tokio::select! {
                _ = state.cancel.cancelled() => break,
                _ = interval.tick() => {}
            }
            if let Err(err) = state.save_snapshot().await {
                warn!(error = %err, "periodic server state snapshot save failed");
            }
        }
    })
}

/// Periodically save the cascade router every serve surface shares
/// (bug-012303), so what serve learns reaches other processes while it runs.
/// Each save merges into the snapshot on disk; shutdown saves once more.
fn start_cascade_router_saver(state: Arc<AppState>) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        interval.tick().await; // skip the first immediate tick
        loop {
            tokio::select! {
                _ = state.cancel.cancelled() => break,
                _ = interval.tick() => {}
            }
            if let Err(err) = state.save_cascade_router().await {
                warn!(error = %err, "periodic cascade router save failed");
            }
        }
    })
}

/// Periodic garbage collection of ephemeral workspaces.
///
/// Runs at the interval configured in `[server].workspace_gc_interval_secs`
/// (default 300s / 5 minutes). Each tick removes entries from
/// `AppState.ephemeral_workspaces` whose `created_at` is older than 1 hour,
/// deleting the corresponding filesystem directories and persisting the
/// updated registry.
fn start_workspace_gc(state: Arc<AppState>) -> JoinHandle<()> {
    const MAX_AGE_SECS: u64 = 3600;

    // Read the configured interval, clamping zero to 1 second to prevent busy-loop.
    let interval_secs = {
        let config = state.load_roko_config();
        let raw = config.server.workspace_gc_interval_secs;
        if raw == 0 { 1 } else { raw }
    };

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs));
        // Skip the first immediate tick — let the server warm up.
        interval.tick().await;

        loop {
            tokio::select! {
                _ = state.cancel.cancelled() => break,
                _ = interval.tick() => {}
            }

            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            let stale: Vec<crate::state::WorkspaceInfo> = {
                let map = state.ephemeral_workspaces.read().await;
                map.values()
                    .filter(|ws| now.saturating_sub(ws.created_at) > MAX_AGE_SECS)
                    .cloned()
                    .collect()
            };

            if stale.is_empty() {
                continue;
            }

            let mut removed = 0usize;
            for ws in &stale {
                if let Err(err) = tokio::fs::remove_dir_all(&ws.path).await {
                    warn!(
                        workspace_id = %ws.id,
                        path = %ws.path.display(),
                        error = %err,
                        "failed to remove stale ephemeral workspace directory"
                    );
                }
            }

            {
                let mut map = state.ephemeral_workspaces.write().await;
                for ws in &stale {
                    if map.remove(&ws.id).is_some() {
                        removed += 1;
                    }
                }
            }

            // Persist the updated registry after GC removals.
            if removed > 0 {
                if let Err(err) = state.persist_workspace_registry().await {
                    warn!(
                        error = %err,
                        "failed to persist workspace registry after GC"
                    );
                }
            }

            info!(
                count = removed,
                "workspace GC: removed {removed} stale ephemeral workspace(s)"
            );
        }
    })
}

/// Periodic GC for completed/failed operation handles (§15.6).
///
/// Without this, `active_runs`, `active_plans`, and `operations` HashMaps
/// grow unboundedly as completed JoinHandles accumulate.
fn start_handle_gc(state: Arc<AppState>) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        interval.tick().await; // skip first immediate tick

        loop {
            tokio::select! {
                _ = state.cancel.cancelled() => break,
                _ = interval.tick() => {}
            }
            state.gc_completed_handles().await;
        }
    })
}

/// Periodic knowledge demurrage: applies confidence/balance decay to knowledge
/// entries that have not been re-validated since the last pass.
///
/// The `DemurrageConsumer` drives the loop. Each heartbeat tick (40s) advances
/// the consumer's iteration counter. When `validation_interval` iterations
/// elapse (default 250 = ~2.9 hours), the consumer applies domain-specific
/// decay via its configured `domain_multipliers`. Entries below the archive
/// threshold are flagged for cold storage.
///
/// Failures are logged at debug level but never crash the server.
fn start_demurrage_timer(state: Arc<AppState>) -> JoinHandle<()> {
    use roko_runtime::demurrage_consumer::{
        DemurrageConsumer, DemurrageConsumerConfig, DemurrageEntry,
    };

    tokio::spawn(async move {
        let mut consumer = DemurrageConsumer::new(DemurrageConsumerConfig::default());
        // The consumer expects one tick per heartbeat iteration (~40s each).
        // validation_interval=250 means demurrage fires every ~2.9 hours.
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(40));

        // Skip the first immediate tick — let the server warm up.
        interval.tick().await;

        loop {
            tokio::select! {
                _ = state.cancel.cancelled() => break,
                _ = interval.tick() => {}
            }

            let store = roko_neuro::knowledge_store::KnowledgeStore::for_workdir(&state.workdir);

            // Read entries and convert to DemurrageEntry for the consumer.
            let entries = match store.read_all() {
                Ok(e) => e,
                Err(e) => {
                    debug!(error = %e, "demurrage: failed to read knowledge store");
                    continue;
                }
            };

            let demurrage_entries: Vec<DemurrageEntry> = entries
                .iter()
                .map(|e| {
                    // Use the first tag as the domain key for multiplier lookup;
                    // fall back to the knowledge kind name.
                    let domain = e
                        .tags
                        .first()
                        .cloned()
                        .unwrap_or_else(|| format!("{:?}", e.kind).to_lowercase());
                    DemurrageEntry {
                        id: e.id.clone(),
                        confidence: e.confidence,
                        domain,
                        last_validated_at: 0,
                        validated_since_last: false,
                    }
                })
                .collect();

            // Tick the consumer — it only fires demurrage when validation_interval elapses.
            let Some((updated_entries, event)) = consumer.tick(&demurrage_entries) else {
                continue;
            };

            debug!(
                iteration = event.iteration,
                entries_decayed = event.entries_decayed,
                entries_archived = event.entries_archived,
                total_confidence_lost = %format!("{:.4}", event.total_confidence_lost),
                "demurrage pass completed via consumer"
            );

            // Build a lookup of updated confidences by entry ID.
            let confidence_updates: std::collections::HashMap<&str, f64> = updated_entries
                .iter()
                .map(|e| (e.id.as_str(), e.confidence))
                .collect();

            // Persist confidence decay back to the store atomically.
            match store.update_entries(|entry| {
                if let Some(&new_conf) = confidence_updates.get(entry.id.as_str()) {
                    if (entry.confidence - new_conf).abs() > f64::EPSILON {
                        entry.confidence = new_conf;
                        return true;
                    }
                }
                false
            }) {
                Ok(n) => {
                    debug!(entries_updated = n, "demurrage: confidence decay persisted");
                }
                Err(e) => {
                    debug!(error = %e, "demurrage: failed to persist confidence decay");
                }
            }

            // Also run the balance-based demurrage (apply_demurrage uses elapsed time).
            // This ensures both confidence decay (consumer) and balance decay (store)
            // are applied together.
            let balances_before = entries
                .iter()
                .map(|entry| (entry.id.as_str(), entry.balance))
                .collect::<std::collections::HashMap<_, _>>();
            match store.apply_demurrage() {
                Ok(count) => match store.read_all() {
                    Ok(after) => {
                        let losses = after
                            .iter()
                            .filter_map(|entry| {
                                let before = balances_before.get(entry.id.as_str())?;
                                let loss = (*before - entry.balance).max(0.0);
                                (loss > 0.0).then_some((entry.id.as_str(), loss))
                            })
                            .collect::<Vec<_>>();
                        let total_balance_lost = losses.iter().map(|(_, loss)| *loss).sum::<f64>();
                        for (signal, loss) in losses {
                            emit_lens_observation(
                                &state,
                                roko_core::ObservableEvent::SignalDemurrageApplied(
                                    signal.to_string(),
                                    loss,
                                ),
                            );
                        }
                        emit_lens_observation(
                            &state,
                            roko_core::ObservableEvent::DemurrageApplied {
                                count,
                                total_balance_lost,
                            },
                        );
                    }
                    Err(e) => {
                        debug!(error = %e, "demurrage: failed to measure applied balance decay");
                    }
                },
                Err(e) => {
                    debug!(error = %e, "demurrage: balance decay failed");
                }
            }
        }
    })
}

fn emit_lens_observation(state: &AppState, event: roko_core::ObservableEvent) {
    for error in state
        .state_hub
        .emit_observable(&event, &[roko_core::LensScope::Global])
    {
        debug!(%error, "Lens observation delivery failed");
    }
}

/// Periodic cold archival: migrates aged-out signals from the hot substrate
/// (`.roko/signals.jsonl` / `FileSubstrate`) to compressed monthly JSONL
/// archives in `.roko/cold/`.
///
/// Runs every six hours (default) or at the interval specified by
/// `cold_storage.interval_secs`. Each tick:
///  1. Opens the hot `FileSubstrate`.
///  2. Queries for signals older than 7 days (default).
///  3. Batch-archives them to `ArchiveColdSubstrate`.
///  4. Applies retention compaction on observability artifacts.
///
/// Failures are logged but never crash the server.
fn start_cold_archival_timer(state: Arc<AppState>) -> JoinHandle<()> {
    let cold_cfg = state.load_roko_config().cold_storage.clone();

    if !cold_cfg.enabled {
        info!("cold archival timer: disabled via config");
        return tokio::spawn(async {});
    }

    // `tokio::time::interval` rejects zero. Treat a zero supplied by an old or
    // hand-written config as the smallest useful interval instead of crashing
    // server startup.
    let interval_secs = cold_cfg.interval_secs.max(1);
    let max_age_ms = cold_cfg.max_age_ms();
    let batch_size = cold_cfg.batch_size;

    info!(
        interval_secs,
        max_age_days = cold_cfg.max_age_days,
        batch_size,
        "cold archival timer: starting scheduled background task"
    );

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs));

        // Skip the first immediate tick — let the server warm up.
        interval.tick().await;

        loop {
            tokio::select! {
                _ = state.cancel.cancelled() => break,
                _ = interval.tick() => {}
            }

            let workdir = &state.workdir;
            let roko_dir = workdir.join(".roko");
            if !roko_dir.is_dir() {
                continue;
            }

            // -- Phase 1: cold-archive aged-out signals ----------------------
            match run_cold_archival_tick(&state.signal_store, max_age_ms, batch_size).await {
                Ok(0) => {
                    debug!("cold archival tick: no signals to archive");
                }
                Ok(n) => {
                    info!(
                        "cold archival tick: archived {n} signal(s) to {}",
                        roko_dir.join("cold").display()
                    );
                }
                Err(err) => {
                    warn!(error = %err, "cold archival tick failed");
                }
            }

            // -- Phase 2: apply retention compaction -------------------------
            let actions = retention::apply_retention(workdir, false);
            for action in &actions {
                info!(
                    artifact = %action.artifact,
                    action = ?action.action,
                    "retention compaction applied"
                );
            }
        }
    })
}

/// Execute a single cold-archival tick: query old signals from the hot
/// substrate, archive them to `.roko/cold/`, then prune them from the hot
/// store so they are not re-archived on the next tick.
///
/// Returns the number of signals archived, or an error.
async fn run_cold_archival_tick(
    signal_store: &crate::state::SignalStore,
    max_age_ms: i64,
    batch_size: usize,
) -> anyhow::Result<usize> {
    signal_store.archive_aged(max_age_ms, batch_size).await
}

/// Start the block watcher background task.
///
/// Polls the chain for new blocks, transactions, and contract events, then
/// publishes them to the event bus and updates `state.chain` ring buffers.
/// Returns a no-op handle if no chain client is configured.
#[cfg(feature = "alloy-backend")]
fn start_block_watcher(state: Arc<AppState>) -> JoinHandle<()> {
    use roko_chain::block_watcher::BlockWatcher;
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    let Some(client) = state.alloy_chain_client.as_ref() else {
        return tokio::spawn(async {});
    };

    let provider = client.provider();

    // Startup probe: quick TCP connect to check if the RPC endpoint is alive.
    // Avoids the 30-attempt seed loop (60s waste) when mirage is dead.
    {
        let rpc_url = state
            .load_roko_config()
            .chain
            .rpc_url
            .clone()
            .unwrap_or_default();
        if let Ok(parsed) = reqwest::Url::parse(&rpc_url) {
            let host = parsed.host_str().unwrap_or("127.0.0.1").to_string();
            let port = parsed.port().unwrap_or(8545);
            if let Ok(addr) = format!("{host}:{port}").parse::<std::net::SocketAddr>() {
                if std::net::TcpStream::connect_timeout(&addr, Duration::from_secs(2)).is_err() {
                    tracing::warn!(
                        rpc_url,
                        "block_watcher RPC startup probe failed; skipping watcher"
                    );
                    return tokio::spawn(async {});
                }
            }
        }
    }

    let watcher = BlockWatcher::new(provider, Duration::from_secs(2));

    // Bridge roko's CancelToken into a tokio-util CancellationToken.
    let cancel = CancellationToken::new();
    let bridge_cancel = cancel.clone();
    let state_bridge = Arc::clone(&state);
    tokio::spawn(async move {
        state_bridge.cancel.cancelled().await;
        bridge_cancel.cancel();
    });

    let state_publish = Arc::clone(&state);
    state.chain.watcher_running.store(true, Ordering::Relaxed);

    let publish_fn: roko_chain::block_watcher::PublishFn =
        Arc::new(move |topic: &str, payload: serde_json::Value| {
            publish_chain_watcher_payload(&state_publish, topic, payload);
        });

    let state_outer = Arc::clone(&state);
    tokio::spawn(async move {
        watcher.run(publish_fn, cancel).await;
        state_outer
            .chain
            .watcher_running
            .store(false, Ordering::Relaxed);
    })
}

#[cfg(not(feature = "alloy-backend"))]
fn start_block_watcher(_state: Arc<AppState>) -> JoinHandle<()> {
    tokio::spawn(async {})
}

/// Publish one block-watcher payload to the event bus and the chain state.
#[cfg(any(feature = "alloy-backend", all(test, feature = "chain")))]
fn publish_chain_watcher_payload(state: &Arc<AppState>, topic: &str, payload: serde_json::Value) {
    use roko_chain::chain_state::{
        BlockInfo, ChainReorgInfo, ContractEventInfo, RawLogInfo, TxInfo,
    };
    match topic {
        "chain.block" => {
            if let Ok(block) = serde_json::from_value::<BlockInfo>(payload) {
                state.event_bus.publish(ServerEvent::ChainBlock {
                    number: block.number,
                    hash: block.hash.clone(),
                    parent_hash: block.parent_hash.clone(),
                    timestamp: block.timestamp,
                    gas_used: block.gas_used,
                    gas_limit: block.gas_limit,
                    tx_count: block.tx_count,
                    base_fee_per_gas: block.base_fee_per_gas,
                });
                let chain_state = Arc::clone(&state.chain);
                tokio::spawn(async move { chain_state.push_block(block).await });
            }
        }
        "chain.tx" => {
            if let Ok(tx) = serde_json::from_value::<TxInfo>(payload) {
                state.event_bus.publish(ServerEvent::ChainTx {
                    block_number: tx.block_number,
                    tx_hash: tx.tx_hash.clone(),
                    from: tx.from.clone(),
                    to: tx.to.clone(),
                    value_wei: tx.value_wei.clone(),
                    gas_used: tx.gas_used,
                    method_sig: tx.method_sig.clone(),
                    success: tx.success,
                });
                let chain_state = Arc::clone(&state.chain);
                tokio::spawn(async move { chain_state.push_tx(tx).await });
            }
        }
        "chain.log" => {
            if let Ok(log) = serde_json::from_value::<RawLogInfo>(payload) {
                let chain_id = state.load_roko_config().chain.chain_id.unwrap_or_default();
                state.event_bus.publish(ServerEvent::ChainLogObserved {
                    chain_id,
                    block_number: log.block_number,
                    block_hash: log.block_hash,
                    tx_hash: log.tx_hash,
                    log_index: log.log_index,
                    contract: log.contract,
                    topics: log.topics,
                    data: log.data,
                    finality: roko_core::trigger::FinalityRequirement::Reversible,
                    removed: false,
                });
            }
        }
        "chain.event" => {
            if let Ok(evt) = serde_json::from_value::<ContractEventInfo>(payload) {
                state.event_bus.publish(ServerEvent::ChainContractEvent {
                    block_number: evt.block_number,
                    tx_hash: evt.tx_hash.clone(),
                    log_index: evt.log_index,
                    contract: evt.contract.clone(),
                    event_name: evt.event_name.clone(),
                    decoded: evt.decoded.clone(),
                    raw_evidence_available: evt.raw_evidence_available,
                });
                let chain_state = Arc::clone(&state.chain);
                tokio::spawn(async move { chain_state.push_event(evt).await });
            }
        }
        "chain.reorg" => {
            if let Ok(reorg) = serde_json::from_value::<ChainReorgInfo>(payload) {
                let chain_id = state.load_roko_config().chain.chain_id.unwrap_or_default();
                state.event_bus.publish(ServerEvent::ChainReorg {
                    chain_id,
                    orphaned_block_hashes: reorg.orphaned_block_hashes,
                });
            }
        }
        _ => {}
    }
}

/// Run the supervised relay bridge for both durable subscription consumption
/// and optional feed publication. The consumer is active whenever a relay URL
/// is configured; it is intentionally not coupled to `feed_agents.enabled`.
#[cfg(feature = "relay")]
fn start_feed_relay_bridge(state: Arc<AppState>) -> Option<tokio::task::JoinHandle<()>> {
    use roko_agent_server::features::relay_client::{
        MAX_DESIRED_ROOMS, RelayClientConfig, RelayClientStatus, TopicHandler, connect,
    };
    use roko_agent_server::registration::{AgentCard, AgentCardEndpoints};
    use roko_agent_server::state::AgentSidecarState;

    let roko_config = state.load_roko_config();
    let raw_relay_url = roko_config.relay.url.clone()?;
    let publish_feeds = roko_config.feed_agents_enabled();

    // Normalize to base URL (strip path like /relay/agents/ws).
    let relay_url = crate::relay::normalize_relay_base_url(&raw_relay_url);
    info!(relay_url = %relay_url, publish_feeds, "starting durable relay subscription bridge");

    let state2 = Arc::clone(&state);
    Some(tokio::spawn(async move {
        let workspace_identity = stable_relay_workspace_identity(&state2.workdir);
        if publish_feeds {
            let publisher_state = Arc::clone(&state2);
            let publisher_url = relay_url.clone();
            let publisher_identity = workspace_identity.clone();
            tokio::spawn(async move {
                run_feed_relay_publisher(publisher_state, publisher_url, publisher_identity).await;
            });
        }

        let consumer_id = relay_consumer_id(&workspace_identity);
        let agent_state = Arc::new(AgentSidecarState::new(
            consumer_id.clone(),
            None,
            env!("CARGO_PKG_VERSION").to_string(),
            vec!["subscription_consumer".to_string()],
            None,
            None,
            None,
        ));

        let card = AgentCard {
            name: consumer_id,
            capabilities: vec!["subscription_consumer".to_string()],
            endpoints: AgentCardEndpoints {
                rest: None,
                websocket: None,
                a2a: None,
                mcp: None,
            },
            domain_tags: vec!["roko".to_string(), "subscription-consumer".to_string()],
            version: env!("CARGO_PKG_VERSION").to_string(),
        };

        let dispatcher: Arc<dyn dispatch::AgentDispatcher> =
            Arc::new(dispatch::TemplateAgentDispatcher::new(
                state2.workdir.clone(),
                None,
                state2.load_roko_config().as_ref().clone(),
            ));
        let topic_handler: Arc<dyn TopicHandler> =
            Arc::new(crate::subscription_relay::ServeTopicHandler::new(
                Arc::clone(&state2.subscription_relay),
                Arc::clone(&state2),
                dispatcher,
            ));
        let origin_hash = crate::subscription_relay::relay_origin_hash(&relay_url);
        'supervisor: loop {
            let plan = crate::subscription_relay::remote_subscription_plan(
                &state2.subscriptions,
                MAX_DESIRED_ROOMS,
            );
            state2
                .subscription_relay
                .set_subscription_plan_diagnostics(
                    plan.unsupported_triggers.clone(),
                    plan.capacity_rejected_rooms.clone(),
                )
                .await;

            if let Err(error) = state2
                .subscription_relay
                .guard_stream_change(&origin_hash, &plan.rooms)
                .await
            {
                let cursor = state2.subscription_relay.status().await.global_cursor;
                state2
                    .subscription_relay
                    .set_connection(
                        crate::subscription_relay::ServeRelayConnectionStatus::ReconciliationRequired {
                            snapshot_seq: cursor,
                        },
                    )
                    .await;
                warn!(%error, "relay stream change requires reconciliation");
                return;
            }

            if !plan.capacity_rejected_rooms.is_empty() || plan.rooms.is_empty() {
                let durable_cursor = state2.subscription_relay.status().await.global_cursor;
                let reason = if plan.rooms.is_empty() {
                    "no enabled exact relay subscription rooms are configured".to_string()
                } else {
                    format!(
                        "{} exact relay rooms exceed client capacity {MAX_DESIRED_ROOMS}",
                        plan.rooms.len()
                    )
                };
                state2
                    .subscription_relay
                    .set_connection(
                        crate::subscription_relay::ServeRelayConnectionStatus::Disconnected {
                            durable_cursor,
                            reason: Some(reason),
                        },
                    )
                    .await;
                tokio::select! {
                    _ = state2.cancel.cancelled() => return,
                    () = tokio::time::sleep(std::time::Duration::from_secs(2)) => {}
                }
                continue;
            }

            if let Err(error) = state2
                .subscription_relay
                .bind_stream(&origin_hash, &plan.rooms)
                .await
            {
                let cursor = state2.subscription_relay.status().await.global_cursor;
                state2
                    .subscription_relay
                    .set_connection(
                        crate::subscription_relay::ServeRelayConnectionStatus::ReconciliationRequired {
                            snapshot_seq: cursor,
                        },
                    )
                    .await;
                warn!(%error, "relay consumer stream binding requires reconciliation");
                return;
            }

            let relay_config = match RelayClientConfig::new(relay_url.clone())
                .with_initial_rooms(plan.rooms.clone())
            {
                Ok(config) => config,
                Err(error) => {
                    let durable_cursor = state2.subscription_relay.status().await.global_cursor;
                    state2
                        .subscription_relay
                        .set_connection(
                            crate::subscription_relay::ServeRelayConnectionStatus::Disconnected {
                                durable_cursor,
                                reason: Some(error.to_string()),
                            },
                        )
                        .await;
                    tokio::select! {
                        _ = state2.cancel.cancelled() => return,
                        () = tokio::time::sleep(std::time::Duration::from_secs(2)) => {}
                    }
                    continue;
                }
            };
            state2
                .subscription_relay
                .set_connection(crate::subscription_relay::ServeRelayConnectionStatus::Connecting)
                .await;
            let mut initial_attempt = 0u32;
            let handle = loop {
                let result = tokio::select! {
                    _ = state2.cancel.cancelled() => return,
                    result = connect(
                        relay_config.clone(),
                        Arc::clone(&agent_state),
                        card.clone(),
                        Some(Arc::clone(&topic_handler)),
                    ) => result,
                };
                match result {
                    Ok(handle) => break handle,
                    Err(error) => {
                        warn!(%error, %relay_url, initial_attempt, "relay consumer initial connect failed");
                        let durable_cursor = state2.subscription_relay.status().await.global_cursor;
                        state2
                            .subscription_relay
                            .set_connection(
                                crate::subscription_relay::ServeRelayConnectionStatus::Disconnected {
                                    durable_cursor,
                                    reason: Some(error.to_string()),
                                },
                            )
                            .await;
                        let delay = relay_initial_retry_delay(initial_attempt);
                        initial_attempt = initial_attempt.saturating_add(1);
                        tokio::select! {
                            _ = state2.cancel.cancelled() => return,
                            () = tokio::time::sleep(delay) => {}
                        }
                        let refreshed = crate::subscription_relay::remote_subscription_plan(
                            &state2.subscriptions,
                            MAX_DESIRED_ROOMS,
                        );
                        state2
                            .subscription_relay
                            .set_subscription_plan_diagnostics(
                                refreshed.unsupported_triggers.clone(),
                                refreshed.capacity_rejected_rooms.clone(),
                            )
                            .await;
                        if let Err(error) = state2
                            .subscription_relay
                            .guard_stream_change(&origin_hash, &refreshed.rooms)
                            .await
                        {
                            let cursor = state2.subscription_relay.status().await.global_cursor;
                            state2
                                .subscription_relay
                                .set_connection(
                                    crate::subscription_relay::ServeRelayConnectionStatus::ReconciliationRequired {
                                        snapshot_seq: cursor,
                                    },
                                )
                                .await;
                            warn!(%error, "relay stream change during connect requires reconciliation");
                            return;
                        }
                        if refreshed.rooms != plan.rooms
                            || !refreshed.capacity_rejected_rooms.is_empty()
                        {
                            continue 'supervisor;
                        }
                        state2
                            .subscription_relay
                            .set_connection(
                                crate::subscription_relay::ServeRelayConnectionStatus::Connecting,
                            )
                            .await;
                    }
                }
            };

            let mut status_rx = handle.subscribe_status();
            let initial_status = status_rx.borrow().clone();
            state2
                .subscription_relay
                .observe_client_status(initial_status)
                .await;
            let mut reconcile = tokio::time::interval(std::time::Duration::from_secs(2));
            reconcile.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tokio::select! {
                    _ = state2.cancel.cancelled() => {
                        handle.shutdown();
                        return;
                    },
                    changed = status_rx.changed() => {
                        if changed.is_err() {
                            break;
                        }
                        let status = status_rx.borrow().clone();
                        state2.subscription_relay.observe_client_status(status.clone()).await;
                        if matches!(
                            status,
                            RelayClientStatus::ReconciliationRequired { .. }
                                | RelayClientStatus::Superseded { .. }
                        ) {
                            handle.shutdown();
                            return;
                        }
                        if matches!(status, RelayClientStatus::Stopped) {
                            break;
                        }
                    }
                    _ = reconcile.tick() => {
                        let refreshed = crate::subscription_relay::remote_subscription_plan(
                            &state2.subscriptions,
                            MAX_DESIRED_ROOMS,
                        );
                        state2
                            .subscription_relay
                            .set_subscription_plan_diagnostics(
                                refreshed.unsupported_triggers.clone(),
                                refreshed.capacity_rejected_rooms.clone(),
                            )
                            .await;
                        if let Err(error) = state2
                            .subscription_relay
                            .guard_stream_change(&origin_hash, &refreshed.rooms)
                            .await
                        {
                            handle.shutdown();
                            let cursor = state2.subscription_relay.status().await.global_cursor;
                            state2
                                .subscription_relay
                                .set_connection(
                                    crate::subscription_relay::ServeRelayConnectionStatus::ReconciliationRequired {
                                        snapshot_seq: cursor,
                                    },
                                )
                                .await;
                            warn!(%error, "relay room-set change requires reconciliation");
                            return;
                        }
                        if refreshed.rooms != plan.rooms
                            || !refreshed.capacity_rejected_rooms.is_empty()
                        {
                            handle.shutdown();
                            if !refreshed.capacity_rejected_rooms.is_empty()
                                || refreshed.rooms.is_empty()
                            {
                                continue 'supervisor;
                            }
                            if let Err(error) = state2
                                .subscription_relay
                                .bind_stream(&origin_hash, &refreshed.rooms)
                                .await
                            {
                                let cursor = state2.subscription_relay.status().await.global_cursor;
                                state2
                                    .subscription_relay
                                    .set_connection(
                                        crate::subscription_relay::ServeRelayConnectionStatus::ReconciliationRequired {
                                            snapshot_seq: cursor,
                                        },
                                    )
                                    .await;
                                warn!(%error, "relay room-set change requires reconciliation");
                                return;
                            }
                            continue 'supervisor;
                        }
                    }
                }
            }
            handle.shutdown();
        }
    }))
}

#[cfg(feature = "relay")]
fn stable_relay_workspace_identity(workdir: &Path) -> String {
    let stable_path = std::fs::canonicalize(workdir).unwrap_or_else(|_| workdir.to_path_buf());
    let hash = blake3::hash(stable_path.to_string_lossy().as_bytes())
        .to_hex()
        .to_string();
    hash[..16].to_string()
}

#[cfg(feature = "relay")]
fn relay_consumer_id(workspace_identity: &str) -> String {
    format!("roko-serve-consumer-{workspace_identity}")
}

#[cfg(feature = "relay")]
fn relay_publisher_id(workspace_identity: &str) -> String {
    format!("roko-serve-publisher-{workspace_identity}")
}

#[cfg(feature = "relay")]
fn relay_initial_retry_delay(attempt: u32) -> std::time::Duration {
    let multiplier = 1u32.checked_shl(attempt.min(7)).unwrap_or(u32::MAX);
    std::time::Duration::from_millis(250)
        .saturating_mul(multiplier)
        .min(std::time::Duration::from_secs(30))
}

#[cfg(feature = "relay")]
async fn run_feed_relay_publisher(
    state: Arc<AppState>,
    relay_url: String,
    workspace_identity: String,
) {
    use roko_agent_server::features::relay_client::{RelayClientConfig, connect};
    use roko_agent_server::registration::{AgentCard, AgentCardEndpoints};
    use roko_agent_server::state::AgentSidecarState;

    let publisher_id = relay_publisher_id(&workspace_identity);
    let agent_state = Arc::new(AgentSidecarState::new(
        publisher_id.clone(),
        None,
        env!("CARGO_PKG_VERSION").to_string(),
        vec!["feed_publisher".to_string()],
        None,
        None,
        None,
    ));
    let card = AgentCard {
        name: publisher_id,
        capabilities: vec!["feed_publisher".to_string()],
        endpoints: AgentCardEndpoints {
            rest: None,
            websocket: None,
            a2a: None,
            mcp: None,
        },
        domain_tags: vec!["roko".to_string(), "feed-publisher".to_string()],
        version: env!("CARGO_PKG_VERSION").to_string(),
    };
    let relay_config = RelayClientConfig::new(relay_url.clone());
    let mut attempt = 0u32;
    let handle = loop {
        let result = tokio::select! {
            _ = state.cancel.cancelled() => return,
            result = connect(
                relay_config.clone(),
                Arc::clone(&agent_state),
                card.clone(),
                None,
            ) => result,
        };
        match result {
            Ok(handle) => break handle,
            Err(error) => {
                warn!(%error, %relay_url, attempt, "feed relay publisher initial connect failed");
                let delay = relay_initial_retry_delay(attempt);
                attempt = attempt.saturating_add(1);
                tokio::select! {
                    _ = state.cancel.cancelled() => return,
                    () = tokio::time::sleep(delay) => {}
                }
            }
        }
    };

    let mut registered_feeds = HashSet::new();
    let mut rx = state.event_bus.subscribe();
    let mut reconcile = tokio::time::interval(std::time::Duration::from_secs(2));
    reconcile.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            _ = state.cancel.cancelled() => break,
            _ = reconcile.tick() => {
                let catalog = state.feed_agent_catalog.read().await;
                for feed in &catalog.feeds {
                    if registered_feeds.contains(&feed.feed_id) {
                        continue;
                    }
                    match handle.register_feed(
                        &feed.feed_id,
                        &feed.topic,
                        &feed.name,
                        &feed.description,
                        &feed.kind,
                        &feed.rate,
                    ) {
                        Ok(()) => {
                            registered_feeds.insert(feed.feed_id.clone());
                        }
                        Err(error) => {
                            debug!(feed_id = %feed.feed_id, %error, "feed relay registration was not admitted");
                        }
                    }
                }
            }
            envelope = rx.recv() => {
                match envelope {
                    Ok(env) => {
                        if let crate::events::ServerEvent::FeedTick { topic, payload, .. } = env.payload
                            && let Err(error) = handle.publish(&topic, "tick", payload)
                        {
                            debug!(%error, "feed relay publish failed");
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                        debug!(skipped, "feed relay publisher event bus lagged");
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }
    handle.shutdown();
}

#[cfg(test)]
#[cfg(feature = "relay")]
mod subscription_relay_bridge_tests {
    use super::*;

    #[test]
    fn workspace_identity_is_stable_and_roles_do_not_self_supersede() {
        let dir = tempfile::tempdir().expect("tempdir");
        let first = stable_relay_workspace_identity(dir.path());
        let second = stable_relay_workspace_identity(dir.path());
        assert_eq!(first, second);
        assert_eq!(first.len(), 16);
        assert_ne!(relay_consumer_id(&first), relay_publisher_id(&first));
    }

    #[test]
    fn relay_room_subscription_rejects_globs_and_invalid_names() {
        use crate::subscription_relay::is_exact_relay_room;
        assert!(is_exact_relay_room("feed.prices"));
        assert!(!is_exact_relay_room("feed.*"));
        assert!(!is_exact_relay_room("feed.price?"));
        assert!(!is_exact_relay_room("feed prices"));
        assert!(!is_exact_relay_room(""));
    }

    #[test]
    fn initial_retry_backoff_is_bounded() {
        assert_eq!(
            relay_initial_retry_delay(0),
            std::time::Duration::from_millis(250)
        );
        assert_eq!(
            relay_initial_retry_delay(1),
            std::time::Duration::from_millis(500)
        );
        assert_eq!(
            relay_initial_retry_delay(1_000),
            std::time::Duration::from_secs(30)
        );
    }
}

#[cfg(test)]
mod plan_set_event_mapping_tests {
    use super::*;

    #[test]
    fn plan_set_loaded_bridges_both_ways_with_one_wire_shape() {
        let dashboard = roko_core::DashboardEvent::PlanSetLoaded {
            plans: vec![roko_core::dashboard_snapshot::PlanSetEntry {
                plan_id: "01-a".into(),
                title: "01-a".into(),
                tasks_total: 10,
                ..Default::default()
            }],
        };

        let server = dashboard_event_to_server(&dashboard).expect("mapped to ServerEvent");
        let wire = serde_json::to_value(&server).expect("serialize server event");
        assert_eq!(wire["type"], "plan_set_loaded");
        assert_eq!(wire, serde_json::to_value(&dashboard).expect("serialize"));
        assert_eq!(server_event_to_dashboard(&server), Some(dashboard));
    }

    /// bug-bfdb9a: newer dashboard events cross the bridge both ways: a
    /// blocked task (gap-f59fe9), and chain and feed events.
    #[test]
    fn newer_dashboard_events_bridge_both_ways() {
        let events = [
            roko_core::DashboardEvent::TaskBlocked {
                plan_id: "p1".into(),
                task_id: "T4".into(),
                title: "Fourth".into(),
                blocked_by: Some("T1".into()),
                reason: "blocked by failed task 'T1'".into(),
            },
            roko_core::DashboardEvent::FeedAgentOnline {
                agent_id: "feed-1".into(),
                name: "prices".into(),
                feed_count: 2,
            },
            roko_core::DashboardEvent::FeedAgentOffline {
                agent_id: "feed-1".into(),
            },
            roko_core::DashboardEvent::ChainTx {
                block_number: 7,
                tx_hash: "0xabc".into(),
                from: "0x1".into(),
                to: None,
                value_wei: "0".into(),
                gas_used: 21_000,
                method_sig: None,
                success: true,
            },
        ];
        for dashboard in events {
            let server = dashboard_event_to_server(&dashboard)
                .unwrap_or_else(|| panic!("{dashboard:?} reaches the server stream"));
            assert_eq!(server_event_to_dashboard(&server), Some(dashboard));
        }
    }

    /// gap-8a1fb3: a gate result's rung crosses both bridges. A Graph run's
    /// label names it (`verify[i:phase]`); a server gate result sends it
    /// apart, and the dashboard gets it in the same label form.
    #[test]
    fn gate_rung_survives_both_bridges() {
        let graph = roko_core::DashboardEvent::GateResult {
            plan_id: "p1".into(),
            task_id: "T1".into(),
            gate: "verify[3:test]".into(),
            passed: false,
            output_text: None,
        };
        let server = dashboard_event_to_server(&graph).expect("reaches the server stream");
        assert!(
            matches!(server, ServerEvent::GateResult { rung: 3, .. }),
            "{server:?}"
        );
        assert_eq!(server_event_to_dashboard(&server), Some(graph));

        let named = ServerEvent::GateResult {
            plan_id: "p1".into(),
            task_id: "T1".into(),
            gate: "compile".into(),
            rung: 2,
            passed: true,
        };
        let Some(roko_core::DashboardEvent::GateResult { gate, .. }) =
            server_event_to_dashboard(&named)
        else {
            panic!("a server gate result reaches the dashboard");
        };
        assert_eq!(gate, "verify[2:compile]");
        assert_eq!(gate_label_rung(&gate), Some(2));
        assert_eq!(gate_label_rung("verify[0]"), Some(0));
        assert_eq!(gate_label_rung("rung[compile]"), None);
    }

    /// gap-8a1fb3: a one-shot run starts and ends the plan its task and agent
    /// events name, so that plan completes when the run does.
    #[test]
    fn one_shot_run_starts_and_ends_the_plan_its_events_name() {
        let run_id = "0123abcd-4567-89ef-0123-456789abcdef";
        let started = ServerEvent::RunStarted {
            run_id: run_id.into(),
            prompt: "say hi".into(),
            origin: None,
        };
        let completed = ServerEvent::RunCompleted {
            run_id: run_id.into(),
            success: true,
            verdict: Some(crate::state::RunState::Succeeded),
        };
        assert_eq!(run_plan_id(run_id), "run-0123abcd");
        assert_eq!(
            server_event_to_dashboard(&started),
            Some(roko_core::DashboardEvent::PlanStarted {
                plan_id: run_plan_id(run_id),
                tasks_total: 0,
            })
        );
        assert_eq!(
            server_event_to_dashboard(&completed),
            Some(roko_core::DashboardEvent::PlanCompleted {
                plan_id: run_plan_id(run_id),
                success: true,
            })
        );
        assert_eq!(run_plan_id("r1"), "run-r1");
    }
}

/// Discover plugin manifests in the standard search paths and register any
/// webhook trigger scopes with the middleware whitelist.
///
/// Search paths (in order):
/// 1. `<workdir>/.roko/extensions/`
/// 2. `<workdir>/plugins/`
/// 3. `<workdir>/.roko/plugins/`
///
/// This must be called before `build_server_router` so that the
/// `register_extension_route_scopes` `OnceLock` is set before the first
/// request arrives.
fn register_plugin_webhook_scopes(workdir: &std::path::Path) {
    let scan_dirs = [
        workdir.join(".roko").join("extensions"),
        workdir.join("plugins"),
        workdir.join(".roko").join("plugins"),
    ];

    let mut scopes: Vec<(String, String)> = Vec::new();

    for dir in &scan_dirs {
        match discover_plugins(dir) {
            Ok(plugins) => {
                for plugin in plugins {
                    let plugin_scopes = plugin.manifest.webhook_route_scopes();
                    if !plugin_scopes.is_empty() {
                        debug!(
                            plugin = %plugin.manifest.plugin.name,
                            count = plugin_scopes.len(),
                            "registering plugin webhook route scopes"
                        );
                        scopes.extend(plugin_scopes);
                    }
                }
            }
            Err(err) => {
                debug!(
                    dir = %dir.display(),
                    error = %err,
                    "failed to scan plugin directory for webhook scopes (skipping)"
                );
            }
        }
    }

    if !scopes.is_empty() {
        info!(
            count = scopes.len(),
            "registering plugin webhook route scopes"
        );
        routes::middleware::register_extension_route_scopes(scopes);
    }
}

fn start_builtin_event_sources(state: Arc<AppState>, roko_config: RokoConfig) {
    let mut sources: Vec<Box<dyn EventSource>> = Vec::new();

    if !roko_config.scheduler.is_empty() && scheduler::claim_scheduler_guard() {
        sources.push(Box::new(CronEventSource::from_config(
            roko_config.scheduler.clone(),
        )));
    } else if !roko_config.scheduler.is_empty() {
        debug!("scheduler already started elsewhere; skipping cron in event sources");
    }

    if !roko_config.watcher.is_empty() {
        sources.push(Box::new(FileWatchEventSource::from_config(
            roko_config.watcher.clone(),
        )));
    }

    if sources.is_empty() {
        return;
    }

    let _ = start_event_source_group(state, sources);
}

async fn signal_ingest_loop(
    state: Arc<AppState>,
    mut receiver: mpsc::Receiver<Signal>,
    cancel: CancellationToken,
) {
    loop {
        let maybe_signal = tokio::select! {
            _ = cancel.cancelled() => None,
            signal = receiver.recv() => signal,
        };
        let Some(signal) = maybe_signal else {
            break;
        };

        if let Err(err) = state.signal_store.put(signal.clone()).await {
            warn!(
                kind = %signal.kind,
                error = %err,
                "failed to persist event-source signal"
            );
            continue;
        }

        state
            .event_bus
            .publish(ServerEvent::WebhookReceived { signal });
    }
}

async fn shutdown_on_cancel(state: Arc<AppState>) {
    state.cancel.cancelled().await;
    state.shutdown().await;
}

/// How long graceful shutdown waits for open connections to finish. Event
/// streams end as soon as the server is cancelled (`routes::sse::until_shutdown`);
/// this bounds whatever else is still open, such as a slow request.
const SHUTDOWN_DRAIN_GRACE: std::time::Duration = std::time::Duration::from_secs(5);

/// Serve `router` until `state` is cancelled, then shut down gracefully, giving
/// open connections at most [`SHUTDOWN_DRAIN_GRACE`] to finish.
async fn serve_until_cancelled(
    listener: TcpListener,
    router: axum::Router,
    state: Arc<AppState>,
) -> Result<()> {
    let (draining_tx, draining_rx) = tokio::sync::oneshot::channel();
    let signal = async move {
        shutdown_on_cancel(state).await;
        let _ = draining_tx.send(());
    };
    let server = axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(signal)
    .into_future();
    drain_within(server, draining_rx, SHUTDOWN_DRAIN_GRACE)
        .await
        .context("axum server error")
}

/// Await `server`, but once `draining` fires, give it only `grace` more to
/// finish. Connections still open then are abandoned, so one client that
/// never disconnects cannot keep the process alive.
async fn drain_within<E>(
    server: impl Future<Output = std::result::Result<(), E>>,
    draining: tokio::sync::oneshot::Receiver<()>,
    grace: std::time::Duration,
) -> std::result::Result<(), E> {
    tokio::pin!(server);
    tokio::select! {
        result = &mut server => return result,
        Ok(()) = draining => {}
    }
    if let Ok(result) = tokio::time::timeout(grace, server).await {
        return result;
    }
    warn!(
        ?grace,
        "connections still open after the shutdown grace period; stopping without them"
    );
    Ok(())
}

/// Read `~/.roko/credentials.json` and return the "default" profile as a
/// raw JSON value. This avoids a dependency on roko-cli's `Credential` type.
fn load_stored_credential() -> Result<Option<serde_json::Value>> {
    let path = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".roko")
        .join("credentials.json");
    let data = match std::fs::read_to_string(&path) {
        Ok(d) => d,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let store: serde_json::Value = serde_json::from_str(&data)?;
    Ok(store.get("default").cloned())
}

fn create_deploy_backend(roko_config: &RokoConfig) -> Arc<dyn deploy::DeployBackend> {
    let dc = &roko_config.deploy;
    match deploy::create_backend(
        &dc.backend,
        dc.railway_api_token.as_deref(),
        dc.project_id.as_deref(),
        dc.environment_id.as_deref(),
    ) {
        Ok(b) => Arc::from(b),
        Err(e) => {
            warn!(
                "failed to create deploy backend '{}': {e}; falling back to manual",
                dc.backend
            );
            match deploy::create_backend("manual", None, None, None) {
                Ok(b) => Arc::from(b),
                Err(e2) => {
                    warn!("manual backend creation unexpectedly failed: {e2}; using default");
                    Arc::from(deploy::manual::ManualBackend::default())
                }
            }
        }
    }
}

// ── Optional OTLP tracing export ──────────────────────────────────────────

/// Initialize OTLP tracing export when the `otlp` feature is enabled and
/// `[serve.tracing].otlp_endpoint` is configured.
///
/// Called from [`ServerBuilder::start_background`] after loading the config.
/// Because the global tracing subscriber is already installed by the CLI
/// bootstrap code before `roko serve` runs, this function logs a warning and
/// returns without modifying the subscriber. A full OTLP integration would
/// require the tracing bootstrap to accept an optional OTLP layer at init
/// time -- that is deferred to a follow-up task.
#[cfg(feature = "otlp")]
fn init_otlp_tracing(endpoint: &str, service_name: &str, _sample_rate: f64) {
    // The tracing subscriber is typically already installed by the CLI entry
    // point before ServerBuilder::start_background is called. Attempting to
    // set a new global default here would panic. Instead, log that the config
    // was detected so operators know the config block is being read.
    //
    // A full implementation would:
    // 1. Accept an OTLP layer from the CLI bootstrap
    // 2. Compose it with the existing env-filter + fmt layers
    // 3. Set the composed subscriber as the global default
    //
    // For now, we validate the config is parsed and log the intent.
    tracing::info!(
        endpoint,
        service_name,
        "OTLP tracing export configured (layer installation deferred; \
         global subscriber already set by CLI bootstrap)"
    );
}

#[cfg(test)]
mod tests {
    use super::{
        RokoConfig, ServerBuildConfig, ServerBuilder, build_app_state, drain_within,
        resolve_bind_with_port_env, run_cold_archival_tick, run_server_with_state,
        serve_api_or_spa_fallback, start_telemetry_producer_bridge, validate_showcase_mode,
        warn_if_auth_misconfigured,
    };

    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode, header::CONTENT_TYPE};
    use roko_core::foundation::FeedbackEvent;
    use roko_gate::AdaptiveThresholds;
    use roko_learn::cascade_router::CascadeRouter;
    use roko_learn::model_router::{CONTEXT_DIM, RoutingContext};
    use serde_json::Value;
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};
    use tempfile::tempdir;

    use crate::runtime::NoOpRuntime;

    struct RecordingLifecycleLens {
        name: String,
        scope: roko_core::LensScope,
        observes: Vec<roko_core::ObservableEventKind>,
        seen: Arc<Mutex<Vec<(String, roko_core::ObservableEvent)>>>,
    }

    #[async_trait::async_trait]
    impl roko_core::TelemetryObserve for RecordingLifecycleLens {
        async fn observe(
            &self,
            event: &roko_core::ObservableEvent,
        ) -> roko_core::Result<Vec<roko_core::Signal>> {
            self.seen
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push((self.name.clone(), event.clone()));
            Ok(Vec::new())
        }

        fn observes(&self) -> &[roko_core::ObservableEventKind] {
            &self.observes
        }

        fn scope(&self) -> roko_core::LensScope {
            self.scope.clone()
        }
    }

    fn register_recording_lens(
        registry: &mut roko_core::LensRegistry,
        name: &str,
        scope: &str,
        observes: roko_core::ObservableEventKind,
    ) {
        registry
            .register_with_observes(
                roko_core::LensConfig {
                    name: name.to_string(),
                    block: "test:recording-lens".to_string(),
                    scope: scope.to_string(),
                    params: BTreeMap::new(),
                },
                vec![observes],
            )
            .expect("register recording lens");
    }

    async fn fallback_response(path: &str) -> axum::response::Response {
        let request = Request::builder()
            .uri(path)
            .body(Body::empty())
            .expect("build request");
        serve_api_or_spa_fallback(request).await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn durable_server_lifecycle_producers_reach_scoped_lenses_once() -> roko_core::Result<()>
    {
        use roko_core::trigger::{
            TriggerBinding, TriggerEvent, TriggerEventKind, TriggerKind, TriggerLifecycleEvent,
            TriggerSource,
        };
        use roko_core::{LensScope, ObservableEvent, ObservableEventKind, Verdict};
        use roko_runtime::{LensExecutor, LensQueueConfig};

        let workdir = tempdir().expect("tempdir");
        let hub = roko_runtime::SharedStateHub::new_in_process();
        let state = Arc::new(
            build_app_state(
                workdir.path().to_path_buf(),
                Arc::new(NoOpRuntime),
                roko_core::config::schema::RokoConfig::default(),
                Some(hub.clone()),
                None,
                None,
            )
            .expect("build state"),
        );

        let seen = Arc::new(Mutex::new(Vec::new()));
        let mut registry = roko_core::LensRegistry::new();
        register_recording_lens(
            &mut registry,
            "trigger-recorder",
            "graph:graph-a",
            ObservableEventKind::TriggerLifecycle,
        );
        register_recording_lens(
            &mut registry,
            "verify-recorder",
            "graph:plan-a",
            ObservableEventKind::VerifyLifecycle,
        );
        let mut executor = LensExecutor::new(registry.clone())?.with_projection(hub.sender());
        for registration in registry.registrations() {
            executor.register(
                registration.config.name.clone(),
                Arc::new(RecordingLifecycleLens {
                    name: registration.config.name.clone(),
                    scope: registration.scope.clone(),
                    observes: registration.observes.clone(),
                    seen: Arc::clone(&seen),
                }),
            )?;
        }
        let queue = executor.into_queued("server-producer-test", LensQueueConfig::default())?;
        let bridge = start_telemetry_producer_bridge(Arc::clone(&state));

        let binding = TriggerBinding::new("deploy", TriggerKind::Manual, "graph-a");
        for kind in [
            TriggerEventKind::Armed,
            TriggerEventKind::Fired,
            TriggerEventKind::Disarmed,
        ] {
            state
                .event_bus
                .publish(crate::events::ServerEvent::TriggerLifecycle {
                    event: TriggerLifecycleEvent::new(
                        &binding,
                        kind,
                        Some("trace-a".to_string()),
                        serde_json::json!({}),
                    ),
                });
        }
        state
            .event_bus
            .publish(crate::events::ServerEvent::TriggerFired {
                trigger_name: "deploy".to_string(),
                event: TriggerEvent::new(
                    "deploy".to_string(),
                    serde_json::json!({}),
                    TriggerSource::Manual {
                        user: "tester".to_string(),
                    },
                    "trace-a".to_string(),
                ),
            });
        state
            .event_bus
            .publish(crate::events::ServerEvent::GateResult {
                plan_id: "plan-a".to_string(),
                task_id: "task-a".to_string(),
                gate: "compile".to_string(),
                rung: 2,
                passed: true,
            });
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                if seen
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .len()
                    == 4
                {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("lifecycle observations");
        assert!(queue.wait_idle(std::time::Duration::from_secs(5)).await);

        let observations = seen
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        assert_eq!(
            observations.len(),
            4,
            "compatibility firing must not duplicate"
        );
        assert!(matches!(
            observations[0],
            (ref lens, ObservableEvent::TriggerArmed { ref trigger })
                if lens == "trigger-recorder" && trigger == "deploy"
        ));
        assert!(matches!(
            observations[1],
            (ref lens, ObservableEvent::TriggerFired { ref trigger, ref graph })
                if lens == "trigger-recorder" && trigger == "deploy" && graph == "graph-a"
        ));
        assert!(matches!(
            observations[2],
            (ref lens, ObservableEvent::TriggerDisarmed { ref trigger })
                if lens == "trigger-recorder" && trigger == "deploy"
        ));
        assert!(matches!(
            observations[3],
            (
                ref lens,
                ObservableEvent::VerifyPostResult {
                    ref block,
                    reward: 1.0,
                    ref verdict,
                    ref evidence,
                }
            ) if lens == "verify-recorder"
                && block == "task-a"
                && verdict == &Verdict::pass("verify[2:compile]")
                && evidence.is_empty()
        ));
        assert_eq!(
            observations[1].1.source_scope(),
            LensScope::Graph("graph-a".to_string())
        );

        state.cancel.cancel();
        bridge.await.expect("bridge shutdown");
        Ok::<(), roko_core::RokoError>(())
    }

    #[tokio::test]
    async fn api_paths_return_json_404() {
        let response = fallback_response("/api/nonexistent").await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .expect("content type");
        assert!(content_type.starts_with("application/json"));

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        let json: Value = serde_json::from_slice(&body).expect("parse body");
        assert_eq!(json["error"].as_str(), Some("not_found"));
        assert_eq!(
            json["message"].as_str(),
            Some("No route matches /api/nonexistent")
        );
    }

    #[tokio::test]
    async fn ws_paths_return_json_404() {
        let response = fallback_response("/ws/nonexistent").await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .expect("content type");
        assert!(content_type.starts_with("application/json"));
    }

    #[tokio::test]
    async fn non_api_paths_still_serve_spa_html() {
        let response = fallback_response("/nonexistent-page").await;
        assert_eq!(response.status(), StatusCode::OK);

        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .expect("content type");
        assert!(content_type.starts_with("text/html"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn build_app_state_loads_persisted_learning_state_and_falls_back_cleanly() {
        let persisted_dir = tempdir().expect("tempdir");
        let persisted_workdir = persisted_dir.path().to_path_buf();
        let persisted_learn_dir = persisted_workdir.join(".roko").join("learn");
        std::fs::create_dir_all(&persisted_learn_dir).expect("create learn dir");

        let thresholds_path = persisted_learn_dir.join("gate-thresholds.json");
        let mut thresholds = AdaptiveThresholds::new();
        thresholds.update(1, true);
        thresholds.save(&thresholds_path).expect("seed thresholds");

        let router_path = persisted_learn_dir.join("cascade-router.json");
        let router = CascadeRouter::new(vec!["claude-sonnet-4-6".to_string()]);
        router.observe(vec![0.0; CONTEXT_DIM], 0, 1.0);
        router.save(&router_path).expect("seed router");

        let mut config = roko_core::config::schema::RokoConfig::default();
        config.models.insert(
            "claude-sonnet".to_string(),
            roko_core::config::schema::ModelProfile {
                provider: "anthropic".to_string(),
                slug: "claude-sonnet-4-6".to_string(),
                ..Default::default()
            },
        );

        let persisted_state = build_app_state(
            persisted_workdir.clone(),
            Arc::new(NoOpRuntime),
            config.clone(),
            None,
            None,
            None,
        )
        .expect("build_app_state");

        let persisted_router = persisted_state.cascade_router.read().await;
        let persisted_router = persisted_router.as_ref().expect("router loaded");
        assert_eq!(persisted_router.total_observations(), 1);

        let thresholds_snapshot = persisted_state.state_hub.current_snapshot();
        let expected_thresholds =
            std::fs::read_to_string(&thresholds_path).expect("read seeded thresholds");
        assert_eq!(
            thresholds_snapshot.gate_thresholds_json,
            expected_thresholds
        );

        let fresh_dir = tempdir().expect("tempdir");
        let fresh_state = build_app_state(
            fresh_dir.path().to_path_buf(),
            Arc::new(NoOpRuntime),
            config,
            None,
            None,
            None,
        )
        .expect("build_app_state");

        let fresh_router = fresh_state.cascade_router.read().await;
        let fresh_router = fresh_router.as_ref().expect("fresh router initialized");
        assert_eq!(fresh_router.total_observations(), 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn production_construction_preserves_verified_restart_provenance() {
        let dir = tempdir().unwrap();
        let workdir = dir.path().to_path_buf();
        let state_dir = workdir.join(".roko/state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let executor = serde_json::json!({
            "schema_version": 1,
            "plan_states": {
                "restart": {
                    "plan_id": "restart",
                    "current_phase": {"kind": "implementing"},
                    "assigned_agents": []
                }
            },
            "queue_order": ["restart"],
            "speculative_executions": {},
            "timestamp_ms": 42
        });
        let snapshot = roko_runtime::StateSnapshot::new(
            42,
            executor.to_string(),
            serde_json::json!({
                "schema_version": 1,
                "executor": executor,
                "timestamp_ms": 42
            })
            .to_string(),
            serde_json::json!({
                "schema_version": 1,
                "run_id": "restart-run",
                "timestamp_ms": 42,
                "tasks_total": 0,
                "tasks_completed": 0,
                "tasks_failed": 0,
                "total_tokens_in": 0,
                "total_tokens_out": 0,
                "total_cost_usd": 0.0,
                "total_agent_calls": 0,
                "replan_ledger": {}
            })
            .to_string(),
            serde_json::json!({"rungs": {}}).to_string(),
        );
        std::fs::write(
            state_dir.join("state-snapshot.json"),
            serde_json::to_vec(&snapshot).unwrap(),
        )
        .unwrap();

        let state = build_app_state(
            workdir,
            Arc::new(NoOpRuntime),
            roko_core::config::schema::RokoConfig::default(),
            None,
            None,
            None,
        )
        .unwrap();
        let captured = state.state_hub.cursor_snapshot();
        assert!(captured.snapshot.plans.contains_key("restart"));
        assert!(captured.provenance.bootstrapped);
        assert!(captured.provenance.recovered);
        assert!(!captured.provenance.live_events_applied);
        assert_eq!(
            captured.provenance.source_status.as_deref(),
            Some("state_snapshot")
        );
        assert!(captured.provenance.runner.is_some());
        // bootstrap_from_workdir emits a SnapshotRebased event advancing the
        // cursor by one; the first live-observable sequence is therefore 1.
        assert_eq!(captured.next_seq, 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn shutdown_persists_cascade_router_state() {
        let dir = tempdir().expect("tempdir");
        let workdir = dir.path().to_path_buf();
        let state = build_app_state(
            workdir.clone(),
            Arc::new(NoOpRuntime),
            roko_core::config::schema::RokoConfig::default(),
            None,
            None,
            None,
        )
        .expect("build_app_state");

        let router = CascadeRouter::new(vec!["claude-sonnet-4-6".to_string()]);
        router.observe(vec![0.0; CONTEXT_DIM], 0, 1.0);
        {
            let mut guard = state.cascade_router.write().await;
            *guard = Some(Arc::new(router));
        }

        state.shutdown().await;

        let reloaded = CascadeRouter::load_or_new(
            &state.layout.cascade_router_path(),
            vec!["claude-sonnet-4-6".to_string()],
        );
        assert_eq!(reloaded.total_observations(), 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn shared_cascade_router_persists_gateway_and_feedback_observations() {
        // bug-012303: serve builds one cascade router. The inference gateway
        // routes with it, observations through any handle on it land in it,
        // and saving AppState's router persists them.
        let dir = tempdir().expect("tempdir");
        let model = "claude-sonnet-4-6";
        let mut config = roko_core::config::schema::RokoConfig::default();
        config.models.insert(
            "claude-sonnet".to_string(),
            roko_core::config::schema::ModelProfile {
                provider: "anthropic".to_string(),
                slug: model.to_string(),
                ..Default::default()
            },
        );
        let state = build_app_state(
            dir.path().to_path_buf(),
            Arc::new(NoOpRuntime),
            config,
            None,
            None,
            None,
        )
        .expect("build_app_state");
        let router = state
            .cascade_router
            .read()
            .await
            .clone()
            .expect("serve loads one cascade router");
        let gateway_router = state.gateway_http.gateway.cascade_router();
        assert!(
            Arc::ptr_eq(gateway_router, &router),
            "the gateway routes with the shared router"
        );

        // A failed attempt, observed through the gateway's handle.
        gateway_router.record_observation(&RoutingContext::default(), model, 0.0, false);
        // A successful model call, as the model-call service reports one.
        state
            .model_call_service
            .feedback_sink()
            .expect("model calls report feedback")
            .record(FeedbackEvent::ModelCall {
                run_id: None,
                request_id: None,
                prompt_section_ids: Vec::new(),
                knowledge_ids: Vec::new(),
                model: Some(model.to_string()),
                provider: Some("anthropic".to_string()),
                token_usage: None,
                cost: None,
                role: "implementer".to_string(),
                input_tokens: 100,
                output_tokens: 20,
                cost_usd: 0.01,
                latency_ms: 1_500,
                success: true,
                error_class: None,
                model_reported: None,
                attempt_key: None,
                cache_hit: false,
            })
            .await
            .expect("record the model call");
        assert_eq!(router.confidence_snapshot()[model], (2, 1));

        state.shutdown().await;

        let router_path = state.layout.cascade_router_path();
        let reloaded = CascadeRouter::load_or_new(&router_path, vec![model.to_string()]);
        assert_eq!(reloaded.confidence_snapshot()[model], (2, 1));
        assert_eq!(reloaded.total_observations(), 2);
        // The journaled model call is in the saved snapshot, and the save
        // truncated the journal, so a replay cannot count it twice.
        let learn_dir = state.layout.learn_dir();
        let segments_dir = learn_dir.join(roko_learn::wal::SEGMENTS_DIR);
        for segment in std::fs::read_dir(&segments_dir).expect("the journal's segments") {
            let path = segment.expect("segment").path();
            let entries = roko_learn::wal::replay_wal(&path).expect("read the segment");
            assert!(entries.is_empty(), "{} holds saved entries", path.display());
        }
    }

    #[tokio::test]
    async fn cold_archival_tick_moves_only_aged_signals_out_of_hot_storage() {
        use roko_core::{Body, ColdStore, Context, Kind, Query, Signal, Store};

        let dir = tempdir().expect("tempdir");
        let roko_dir = dir.path().join(".roko");
        let hot = roko_fs::FileSubstrate::open(&roko_dir)
            .await
            .expect("open hot substrate");
        let now_ms = chrono::Utc::now().timestamp_millis();
        let aged = Signal::builder(Kind::Metric)
            .body(Body::text("aged"))
            .created_at_ms(now_ms - 120_000)
            .build();
        let fresh = Signal::builder(Kind::Metric)
            .body(Body::text("fresh"))
            .created_at_ms(now_ms)
            .build();
        // Capture the stored IDs: put_if_absent attaches an HDC fingerprint
        // tag and recomputes the content hash, so the on-disk ID differs from
        // the pre-storage Signal::build() ID.
        let stored_aged_id = hot.put(aged.clone()).await.expect("store aged signal");
        let stored_fresh_id = hot.put(fresh.clone()).await.expect("store fresh signal");

        let hub = roko_runtime::StateHub::default_capacity();
        let signal_store = crate::state::SignalStore::new(roko_dir.clone(), hub.sender());
        let archived = run_cold_archival_tick(&signal_store, 60_000, 100)
            .await
            .expect("archive tick");
        assert_eq!(archived, 1);

        // The archival tick owns a separate FileSubstrate handle. Reopen the
        // hot store to validate the durable post-compaction state rather than
        // the original handle's deliberately independent in-memory index.
        let reopened_hot = roko_fs::FileSubstrate::open(&roko_dir)
            .await
            .expect("reopen hot substrate");
        let remaining = reopened_hot
            .query(&Query::all(), &Context::now())
            .await
            .expect("query hot substrate");
        assert_eq!(remaining.len(), 1);
        assert!(
            remaining.iter().any(|s| s.id == stored_fresh_id),
            "fresh signal must survive archival; got {:?}",
            remaining.iter().map(|s| s.id).collect::<Vec<_>>()
        );

        let cold = roko_fs::ArchiveColdSubstrate::open(roko_dir.join("cold"))
            .await
            .expect("open cold substrate");
        assert!(
            cold.contains(&stored_aged_id)
                .await
                .expect("query cold substrate")
        );
        assert!(
            !cold
                .contains(&stored_fresh_id)
                .await
                .expect("query cold substrate")
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn server_builder_rejects_ambiguous_models_before_startup() {
        let dir = tempdir().expect("tempdir");
        let mut config = roko_core::config::schema::RokoConfig::default();
        for key in ["first", "second"] {
            config.models.insert(
                key.to_string(),
                roko_core::config::schema::ModelProfile {
                    provider: "provider".to_string(),
                    slug: "duplicate-slug".to_string(),
                    ..Default::default()
                },
            );
        }
        config.agent.default_model = "first".to_string();

        let build = ServerBuildConfig::new(
            dir.path().to_path_buf(),
            Arc::new(NoOpRuntime),
            config,
            Some("127.0.0.1".to_string()),
            Some(0),
        );
        let error = match ServerBuilder::new(build).start_background().await {
            Err(error) => error,
            Ok((state, handle)) => {
                state.shutdown().await;
                let _ = handle.await;
                panic!("ambiguous server config unexpectedly started");
            }
        };

        assert!(
            error
                .to_string()
                .contains("validate model configuration before server startup"),
            "unexpected error: {error:#}"
        );
        let error_chain = format!("{error:#}");
        assert!(
            error_chain.contains("ambiguous model slug"),
            "unexpected error chain: {error_chain}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn run_server_with_state_rejects_invalid_config_before_side_effects() {
        fn snapshot_tree(root: &std::path::Path) -> Vec<(std::path::PathBuf, Vec<u8>)> {
            fn visit(
                root: &std::path::Path,
                path: &std::path::Path,
                snapshot: &mut Vec<(std::path::PathBuf, Vec<u8>)>,
            ) {
                let mut entries = std::fs::read_dir(path)
                    .expect("read test workdir")
                    .collect::<Result<Vec<_>, _>>()
                    .expect("collect test workdir");
                entries.sort_by_key(std::fs::DirEntry::path);
                for entry in entries {
                    let entry_path = entry.path();
                    if entry_path.is_dir() {
                        visit(root, &entry_path, snapshot);
                    } else {
                        snapshot.push((
                            entry_path
                                .strip_prefix(root)
                                .expect("path under root")
                                .to_path_buf(),
                            std::fs::read(&entry_path).expect("read test file"),
                        ));
                    }
                }
            }

            let mut snapshot = Vec::new();
            visit(root, root, &mut snapshot);
            snapshot
        }

        let dir = tempdir().expect("tempdir");
        let mut config = roko_core::config::schema::RokoConfig::default();
        for key in ["first", "second"] {
            config.models.insert(
                key.to_string(),
                roko_core::config::schema::ModelProfile {
                    provider: "provider".to_string(),
                    slug: "duplicate-slug".to_string(),
                    ..Default::default()
                },
            );
        }
        config.agent.default_model = "first".to_string();
        let state = Arc::new(
            build_app_state(
                dir.path().to_path_buf(),
                Arc::new(NoOpRuntime),
                config,
                None,
                None,
                None,
            )
            .expect("build app state"),
        );
        let before = snapshot_tree(dir.path());
        let probe = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("reserve probe port");
        let port = probe.local_addr().expect("probe address").port();
        drop(probe);

        let error = run_server_with_state(Arc::clone(&state), "127.0.0.1", port)
            .await
            .expect_err("invalid state must not start");
        assert!(
            format!("{error:#}").contains("ambiguous model slug"),
            "unexpected error: {error:#}"
        );

        tokio::task::yield_now().await;
        assert!(
            !state.cancel.is_cancelled(),
            "background shutdown was touched"
        );
        assert_eq!(state.supervisor.count().await, 0, "a process was started");
        assert_eq!(snapshot_tree(dir.path()), before, "workdir was mutated");
        let rebound = tokio::net::TcpListener::bind(("127.0.0.1", port))
            .await
            .expect("validation must happen before listener bind");
        drop(rebound);
    }

    /// backlog 2124: the serve lifecycle runs its periodic telemetry
    /// observer, keeps no Lens sample on disk, and shuts down.
    #[tokio::test(flavor = "multi_thread")]
    async fn run_server_with_state_persists_no_telemetry_and_shuts_down() {
        let dir = tempdir().expect("tempdir");
        let state = Arc::new(
            build_app_state(
                dir.path().to_path_buf(),
                Arc::new(NoOpRuntime),
                roko_core::config::schema::RokoConfig::default(),
                None,
                None,
                None,
            )
            .expect("build app state"),
        );
        let probe = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("reserve probe port");
        let port = probe.local_addr().expect("probe address").port();
        drop(probe);

        let server_state = Arc::clone(&state);
        let server = tokio::spawn(run_server_with_state(server_state, "127.0.0.1", port));
        let listening = tokio::time::timeout(std::time::Duration::from_secs(3), async {
            while tokio::net::TcpStream::connect(("127.0.0.1", port))
                .await
                .is_err()
            {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await;
        // The observer samples at once, then every 30 s.
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // Always cancel and join the production lifecycle before asserting so
        // a failed check cannot strand server background tasks in tests.
        state.shutdown().await;
        let server_result = tokio::time::timeout(std::time::Duration::from_secs(3), server)
            .await
            .expect("server did not stop after cancellation")
            .expect("server task panicked");
        server_result.expect("server returned an error");

        listening.expect("the server never listened");
        let samples = dir
            .path()
            .join(".roko/metrics/telemetry-observations.jsonl");
        assert!(!samples.exists(), "Lens samples are not persisted");
        assert!(state.cancel.is_cancelled());
    }

    /// A connection that never closes cannot keep a cancelled server alive:
    /// once draining starts, the server gets the grace period and no more.
    #[tokio::test(start_paused = true)]
    async fn shutdown_drain_abandons_connections_open_past_the_grace_period() {
        let grace = std::time::Duration::from_secs(5);
        let never_drains = std::future::pending::<std::io::Result<()>>;

        let (_draining_tx, not_draining) = tokio::sync::oneshot::channel();
        let serving = tokio::time::timeout(
            std::time::Duration::from_secs(3600),
            drain_within(never_drains(), not_draining, grace),
        )
        .await;
        assert!(serving.is_err(), "the grace period started before draining");

        let (draining_tx, draining) = tokio::sync::oneshot::channel();
        draining_tx.send(()).expect("start draining");
        let started = tokio::time::Instant::now();
        drain_within(never_drains(), draining, grace)
            .await
            .expect("an abandoned drain is not an error");
        assert_eq!(started.elapsed(), grace);
    }

    /// T3-25: a `PORT` env override must replace **only** the port, leaving
    /// the configured bind (default `127.0.0.1`) intact. Cloud platforms set
    /// `PORT` to choose a port; they do not (and should not) imply a public
    /// bind.
    #[test]
    fn port_env_override_keeps_loopback_bind_by_default() {
        let (bind, port) = resolve_bind_with_port_env(
            "127.0.0.1", // server.bind default
            None,        // no CLI --bind
            6677,        // server.port default
            None,        // no CLI --port
            Some("8080"),
        )
        .expect("resolve");
        assert_eq!(bind, "127.0.0.1");
        assert_eq!(port, 8080);
    }

    /// Operators who explicitly set `[server].bind = "0.0.0.0"` get the
    /// public bind they asked for, with the `PORT`-supplied port.
    #[test]
    fn port_env_override_respects_explicit_public_bind() {
        let (bind, port) =
            resolve_bind_with_port_env("0.0.0.0", None, 6677, None, Some("8080")).expect("resolve");
        assert_eq!(bind, "0.0.0.0");
        assert_eq!(port, 8080);
    }

    /// CLI overrides (the `bind`/`port` arguments threaded through
    /// `ServerBuildConfig`) take precedence over both config and the `PORT`
    /// env var's bind half — but `PORT` still wins for the port number when
    /// it is set.
    #[test]
    fn port_env_override_respects_cli_bind_override() {
        let (bind, port) =
            resolve_bind_with_port_env("127.0.0.1", Some("10.0.0.5"), 6677, Some(7777), Some("80"))
                .expect("resolve");
        assert_eq!(bind, "10.0.0.5");
        // PORT env still wins over the CLI port override (matches existing
        // semantics — cloud platforms set PORT *because* they pick the port).
        assert_eq!(port, 80);
    }

    /// Without `PORT`, both bind and port come from the CLI override (or
    /// fall through to config defaults).
    #[test]
    fn no_port_env_falls_back_to_cli_or_config() {
        let (bind, port) = resolve_bind_with_port_env("127.0.0.1", None, 6677, None, None)
            .expect("resolve fallback");
        assert_eq!(bind, "127.0.0.1");
        assert_eq!(port, 6677);

        let (bind, port) =
            resolve_bind_with_port_env("127.0.0.1", Some("0.0.0.0"), 6677, Some(9999), None)
                .expect("resolve overrides");
        assert_eq!(bind, "0.0.0.0");
        assert_eq!(port, 9999);
    }

    #[test]
    fn public_cli_bind_override_is_synchronized_into_shared_config() {
        let dir = tempdir().expect("tempdir");
        let mut config = roko_core::config::schema::RokoConfig::default();
        config.server.bind = "127.0.0.1".to_string();
        let mut build = ServerBuildConfig::new(
            dir.path().to_path_buf(),
            Arc::new(NoOpRuntime),
            config,
            Some("0.0.0.0".to_string()),
            None,
        );

        assert_eq!(build.synchronize_effective_bind(), "0.0.0.0");
        assert_eq!(build.roko_config.server.bind, "0.0.0.0");
    }

    #[test]
    fn loopback_cli_bind_override_is_synchronized_into_shared_config() {
        let dir = tempdir().expect("tempdir");
        let mut config = roko_core::config::schema::RokoConfig::default();
        config.server.bind = "0.0.0.0".to_string();
        let mut build = ServerBuildConfig::new(
            dir.path().to_path_buf(),
            Arc::new(NoOpRuntime),
            config,
            Some("127.0.0.1".to_string()),
            None,
        );

        assert_eq!(build.synchronize_effective_bind(), "127.0.0.1");
        assert_eq!(build.roko_config.server.bind, "127.0.0.1");
    }

    #[test]
    fn invalid_port_env_returns_error() {
        let err = resolve_bind_with_port_env("127.0.0.1", None, 6677, None, Some("not-a-port"))
            .expect_err("non-numeric PORT must fail");
        let msg = err.to_string();
        assert!(
            msg.contains("PORT env var must be a valid u16"),
            "unexpected error: {msg}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn public_bind_does_not_turn_on_privy_jwt_auth() {
        let workdir = tempdir().expect("tempdir");
        let mut config = roko_core::config::schema::RokoConfig::default();
        config.server.bind = "0.0.0.0".to_string();
        let state = build_app_state(
            workdir.path().to_path_buf(),
            Arc::new(NoOpRuntime),
            config,
            None,
            None,
            None,
        )
        .expect("build_app_state");

        let auth = state.load_roko_config().serve.auth.clone();
        assert!(auth.enabled);
        assert_eq!(auth.privy_app_id, None);
    }

    #[test]
    fn warn_if_auth_misconfigured_passes_when_disabled() {
        let auth = roko_core::config::ServeAuthConfig {
            enabled: false,
            ..Default::default()
        };
        // Should not panic — warn_if_auth_misconfigured returns early when disabled.
        warn_if_auth_misconfigured(&auth);
    }

    #[test]
    fn warn_if_auth_misconfigured_passes_when_legacy_key_set() {
        let auth = roko_core::config::ServeAuthConfig {
            enabled: true,
            api_key: "secret".to_string(),
            ..Default::default()
        };
        // Legacy key present — no warning path exercised, function should complete without panic.
        warn_if_auth_misconfigured(&auth);
    }

    #[test]
    fn warn_if_auth_misconfigured_passes_when_named_keys_present() {
        use roko_core::config::ApiKeyEntry;
        let auth = roko_core::config::ServeAuthConfig {
            enabled: true,
            api_keys: vec![ApiKeyEntry {
                name: "ci".to_string(),
                key_hash: "abc".to_string(),
                scope: "admin".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                expires_at: None,
                last_used_at: None,
                previous_key_hashes: Vec::new(),
            }],
            ..Default::default()
        };
        warn_if_auth_misconfigured(&auth);
    }

    /// 9320: with `[showcase] enabled = true`, serve refuses to start unless every rule holds:
    /// auth on and enforcing, a public origin, an Argon2id passphrase hash, and no Privy. Each
    /// case breaks one rule, and the error names it.
    #[test]
    fn showcase_mode_refuses_to_start_half_configured() {
        const HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$c2FsdHNhbHQ$aGFzaGhhc2hoYXNoaGFzaA";
        let mut ready = RokoConfig::default();
        ready.showcase.enabled = true;
        ready.showcase.public_origin = Some("https://roko-showcase.fly.dev".to_string());
        assert!(validate_showcase_mode(&ready, Some(HASH)).is_ok());
        // Showcase mode off: nothing to check.
        assert!(validate_showcase_mode(&RokoConfig::default(), None).is_ok());

        let mut auth_off = ready.clone();
        auth_off.serve.auth.enabled = false;
        let mut audit_only = ready.clone();
        audit_only.serve.auth.enforcement_mode = roko_core::config::EnforcementMode::Audit;
        let mut no_origin = ready.clone();
        no_origin.showcase.public_origin = None;
        let mut privy = ready.clone();
        privy.serve.auth.privy_app_id = Some("privy-app".to_string());
        let argon2i = Some("$argon2i$v=19$m=19456,t=2,p=1$c2FsdA$aGFzaA");
        let cases = [
            (auth_off, Some(HASH), "serve.auth.enabled"),
            (audit_only, Some(HASH), "serve.auth.enforcement_mode"),
            (no_origin, Some(HASH), "showcase.public_origin"),
            (ready.clone(), None, "ROKO_SHOWCASE_PASSPHRASE_HASH"),
            (ready.clone(), argon2i, "ROKO_SHOWCASE_PASSPHRASE_HASH"),
            (privy, Some(HASH), "serve.auth.privy_app_id"),
        ];
        for (config, hash, rule) in cases {
            let error = validate_showcase_mode(&config, hash).expect_err(rule);
            let message = error.to_string();
            assert!(message.contains(rule), "{message}");
            assert!(message.contains("half configured"), "{message}");
            assert_eq!(message.matches(" must ").count(), 1, "{message}");
        }
    }

    // ── live_agent_output_honours_the_bind ───────────────────────────────────

    #[test]
    fn live_agent_output_honours_the_bind() {
        use crate::live_agent_output_for_bind;
        use roko_core::config::serve::LiveAgentOutput;

        // Loopback + Trusted → effective Trusted, confirmation message.
        let (eff, msg) = live_agent_output_for_bind(LiveAgentOutput::Trusted, "127.0.0.1");
        assert_eq!(
            eff,
            LiveAgentOutput::Trusted,
            "loopback should allow Trusted"
        );
        assert_eq!(msg, "live agent output: trusted");

        // Non-loopback + Trusted → effective ToolSteps, warning message.
        let (eff, msg) = live_agent_output_for_bind(LiveAgentOutput::Trusted, "0.0.0.0");
        assert_eq!(
            eff,
            LiveAgentOutput::ToolSteps,
            "non-loopback must downgrade Trusted"
        );
        assert!(
            msg.contains("trusted ignored on non-loopback bind"),
            "expected warning in: {msg}"
        );
        assert!(
            msg.contains("0.0.0.0"),
            "bind address must appear in: {msg}"
        );

        // Default (ToolSteps) on loopback → ToolSteps, plain confirmation.
        let (eff, msg) = live_agent_output_for_bind(LiveAgentOutput::ToolSteps, "127.0.0.1");
        assert_eq!(eff, LiveAgentOutput::ToolSteps);
        assert_eq!(msg, "live agent output: tool_steps");

        // Default (ToolSteps) on non-loopback → ToolSteps (not a demotion, no warning).
        let (eff, msg) = live_agent_output_for_bind(LiveAgentOutput::ToolSteps, "0.0.0.0");
        assert_eq!(eff, LiveAgentOutput::ToolSteps);
        assert_eq!(msg, "live agent output: tool_steps");
    }
}
