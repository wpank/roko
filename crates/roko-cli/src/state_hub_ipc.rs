//! Cross-process IPC layer for [`SharedStateHub`].
//!
//! Enables `roko dashboard` running in a second terminal to connect to the
//! live [`StateHub`] inside a running `roko plan run` process.
//!
//! # Protocol
//!
//! All frames are **length-prefixed JSON**: a 4-byte big-endian `u32` body
//! length followed by a UTF-8 JSON object.  Each message has a `"type"`
//! discriminator and a `"data"` payload:
//!
//! ```json
//! { "type": "snapshot", "data": { /* DashboardSnapshot */ } }
//! { "type": "event",    "data": { /* DashboardEvent   */ } }
//! ```
//!
//! On each new connection the server sends one `snapshot` frame (the current
//! materialized state) and then streams `event` frames for every subsequent
//! [`DashboardEvent`] published to the hub.  The client applies the snapshot
//! to a fresh in-process hub, then subscribes to the live event stream from
//! that hub — so the TUI sees zero-copy local state just like a co-located run.
//!
//! # Socket path
//!
//! `.roko/runtime/hub.sock` — alongside the daemon socket.  The file is
//! created when a plan run starts and removed when the process exits.
//!
//! # Fallback
//!
//! When no socket exists (or the connection fails), `try_connect_hub_ipc`
//! returns `None` and the caller falls back to the existing file-polling path.

#[cfg(unix)]
mod unix {
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use anyhow::{Context as _, Result};
    use serde::{Deserialize, Serialize};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{UnixListener, UnixStream};
    use tokio::task::JoinHandle;
    use tokio_util::sync::CancellationToken;
    use tracing::{debug, warn};

    use roko_core::dashboard_snapshot::{DashboardEvent, DashboardSnapshot};

    use crate::state_hub::SharedStateHub;

    // ── Wire protocol ─────────────────────────────────────────────────────────

    /// A framed IPC message with a type discriminator.
    #[derive(Debug, Serialize, Deserialize)]
    #[serde(tag = "type", content = "data", rename_all = "snake_case")]
    enum HubIpcMessage {
        /// Full materialized snapshot sent once on connect.
        ///
        /// Boxed to reduce the enum stack size (DashboardSnapshot is ~1360 bytes).
        Snapshot(Box<DashboardSnapshot>),
        /// One event to be applied incrementally after the snapshot.
        Event(DashboardEvent),
    }

    /// Write a length-prefixed JSON frame to `stream`.
    ///
    /// Format: 4-byte big-endian body length + UTF-8 JSON body.
    async fn write_frame(stream: &mut UnixStream, msg: &HubIpcMessage) -> Result<()> {
        let body = serde_json::to_vec(msg).context("serialize IPC frame")?;
        let len = u32::try_from(body.len()).context("frame too large")?;
        stream
            .write_all(&len.to_be_bytes())
            .await
            .context("write IPC frame length")?;
        stream
            .write_all(&body)
            .await
            .context("write IPC frame body")?;
        Ok(())
    }

    /// Read one length-prefixed JSON frame from `stream`.
    async fn read_frame(stream: &mut UnixStream) -> Result<HubIpcMessage> {
        let mut len_buf = [0u8; 4];
        stream
            .read_exact(&mut len_buf)
            .await
            .context("read IPC frame length")?;
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut body = vec![0u8; len];
        stream
            .read_exact(&mut body)
            .await
            .context("read IPC frame body")?;
        serde_json::from_slice(&body).context("deserialize IPC frame")
    }

    // ── Path helpers ─────────────────────────────────────────────────────────

    /// `.roko/runtime/hub.sock` — the well-known path for the StateHub IPC socket.
    pub fn hub_socket_path(workdir: &Path) -> PathBuf {
        workdir.join(".roko").join("runtime").join("hub.sock")
    }

    // ── Server ────────────────────────────────────────────────────────────────

    /// Start the StateHub IPC server.
    ///
    /// Binds `.roko/runtime/hub.sock`, accepts connections, sends the current
    /// snapshot once, then streams live `DashboardEvent`s until the connection
    /// is closed or `shutdown` is cancelled.
    ///
    /// Returns a `JoinHandle` for the accept loop.  The socket is removed when
    /// the task exits (either via shutdown or accept error).
    pub fn start_hub_ipc_server(
        hub: SharedStateHub,
        workdir: &Path,
        shutdown: CancellationToken,
    ) -> Result<JoinHandle<()>> {
        let socket_path = hub_socket_path(workdir);

        // Ensure the parent directory exists.
        if let Some(parent) = socket_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create {}", parent.display()))?;
        }

        // Remove a stale socket from a previous run.
        if socket_path.exists() {
            std::fs::remove_file(&socket_path)
                .with_context(|| format!("remove stale hub socket {}", socket_path.display()))?;
        }

        let listener = UnixListener::bind(&socket_path)
            .with_context(|| format!("bind hub socket {}", socket_path.display()))?;

        // Restrict socket to owner-only (0600): the event stream includes agent
        // output that [serve.auth] does not cover.
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(&socket_path, std::fs::Permissions::from_mode(0o600))
                .with_context(|| format!("chmod 0600 hub socket {}", socket_path.display()))?;
        }

        tracing::debug!(path = %socket_path.display(), "StateHub IPC server bound");

        let handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown.cancelled() => {
                        debug!("StateHub IPC server shutdown requested");
                        break;
                    }
                    result = listener.accept() => {
                        let (stream, _peer) = match result {
                            Ok(pair) => pair,
                            Err(err) => {
                                warn!(error = %err, "StateHub IPC accept failed; stopping server");
                                break;
                            }
                        };
                        let hub_clone = hub.clone();
                        tokio::spawn(async move {
                            if let Err(err) = handle_hub_connection(stream, hub_clone).await {
                                debug!(error = %err, "StateHub IPC connection closed");
                            }
                        });
                    }
                }
            }

            // Clean up socket file on exit.
            let _ = tokio::fs::remove_file(&socket_path).await;
        });

        Ok(handle)
    }

    /// Handle one accepted IPC connection.
    ///
    /// Sends a `snapshot` frame, then streams `event` frames until the
    /// connection is dropped or the hub's broadcast channel lags.
    async fn handle_hub_connection(mut stream: UnixStream, hub: SharedStateHub) -> Result<()> {
        // Atomically capture snapshot + subscribe to live events in one lock
        // cycle so no event can slip between snapshot and subscription.
        let subscription = hub.subscribe_events_from(hub.cursor_snapshot().next_seq);
        let snapshot = subscription.cursor.snapshot.clone();
        let mut live_rx = subscription.live;

        // Send the snapshot first.
        write_frame(&mut stream, &HubIpcMessage::Snapshot(Box::new(snapshot)))
            .await
            .context("send snapshot frame")?;

        // Replay any ring-buffered events the client missed between the
        // snapshot and now (typically zero for a fresh connection).
        for envelope in subscription.replay {
            write_frame(&mut stream, &HubIpcMessage::Event(envelope.payload))
                .await
                .context("send replay event frame")?;
        }

        // Stream live events.
        loop {
            match live_rx.recv().await {
                Ok(envelope) => {
                    if let Err(err) =
                        write_frame(&mut stream, &HubIpcMessage::Event(envelope.payload)).await
                    {
                        // Broken pipe / client disconnected — log at debug, not warn.
                        debug!(error = %err, "StateHub IPC stream write failed; closing connection");
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    // The client is too slow; the ring buffer wrapped.
                    // Re-snapshot to restore coherence and continue.
                    warn!(
                        skipped = n,
                        "StateHub IPC client lagged; re-sending snapshot"
                    );
                    let snapshot = hub.current_snapshot();
                    if let Err(err) =
                        write_frame(&mut stream, &HubIpcMessage::Snapshot(Box::new(snapshot))).await
                    {
                        debug!(error = %err, "StateHub IPC re-snapshot write failed; closing connection");
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    // Hub has been dropped; plan run ended.
                    debug!("StateHub IPC broadcast channel closed; closing connection");
                    break;
                }
            }
        }

        Ok(())
    }

    // ── Client ────────────────────────────────────────────────────────────────

    /// Attempt to connect to a live StateHub IPC server.
    ///
    /// If `.roko/runtime/hub.sock` exists and is connectable, returns a
    /// [`SharedStateHub`] pre-seeded with the full current snapshot and wired
    /// to stream live events from the remote hub into its in-process event bus.
    ///
    /// Returns `None` when no socket exists, the connection is refused, or any
    /// protocol error occurs.  The caller should fall back to file polling.
    pub async fn try_connect_hub_ipc(workdir: &Path) -> Option<SharedStateHub> {
        let socket_path = hub_socket_path(workdir);
        if !socket_path.exists() {
            return None;
        }

        match connect_hub_ipc(socket_path).await {
            Ok(hub) => Some(hub),
            Err(err) => {
                debug!(error = %err, "StateHub IPC connect failed; falling back to file polling");
                None
            }
        }
    }

    async fn connect_hub_ipc(socket_path: PathBuf) -> Result<SharedStateHub> {
        let mut stream = UnixStream::connect(&socket_path)
            .await
            .with_context(|| format!("connect hub socket {}", socket_path.display()))?;

        // Read the first frame — must be a snapshot.
        let first = read_frame(&mut stream)
            .await
            .context("read initial snapshot frame")?;

        let initial_snapshot = match first {
            HubIpcMessage::Snapshot(snap) => *snap,
            HubIpcMessage::Event(_) => {
                anyhow::bail!("expected snapshot frame, got event frame");
            }
        };

        // Build an in-process hub and seed it with the snapshot from the server.
        let hub = SharedStateHub::new_in_process();
        hub.apply_snapshot(initial_snapshot);

        // Spawn a background task that reads frames and publishes them into
        // the local hub so TUI consumers see live updates.
        let hub_clone = Arc::new(hub.clone());
        tokio::spawn(async move {
            loop {
                match read_frame(&mut stream).await {
                    Ok(HubIpcMessage::Event(event)) => {
                        hub_clone.sender().publish(event);
                    }
                    Ok(HubIpcMessage::Snapshot(snap)) => {
                        // Server sent a re-snapshot (client lagged).
                        hub_clone.apply_snapshot(*snap);
                    }
                    Err(err) => {
                        debug!(error = %err, "StateHub IPC reader stream ended");
                        break;
                    }
                }
            }
        });

        Ok(hub)
    }
}

// ── Public surface (unix-only) ───────────────────────────────────────────────

#[cfg(unix)]
pub use unix::{hub_socket_path, start_hub_ipc_server, try_connect_hub_ipc};

/// On non-Unix targets there is no socket support; the client always returns
/// `None` (file-polling fallback) and the server is a no-op.
#[cfg(not(unix))]
pub mod stubs {
    use std::path::Path;

    use crate::state_hub::SharedStateHub;

    pub async fn try_connect_hub_ipc(_workdir: &Path) -> Option<SharedStateHub> {
        None
    }
}

#[cfg(not(unix))]
pub use stubs::try_connect_hub_ipc;
