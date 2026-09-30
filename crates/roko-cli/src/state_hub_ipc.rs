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
//! On each new connection the client first sends one hello frame carrying the
//! hub token (see *Authentication*).  The server then sends one `snapshot`
//! frame (the current materialized state) and streams `event` frames for every
//! subsequent [`DashboardEvent`] published to the hub.  The client applies the
//! snapshot to a fresh in-process hub, then subscribes to the live event stream
//! from that hub — so the TUI sees zero-copy local state just like a co-located
//! run.
//!
//! ```json
//! { "token": "<contents of .roko/runtime/hub.token>" }
//! ```
//!
//! # Authentication
//!
//! Before it binds the socket, the server writes a random per-run token to
//! `.roko/runtime/hub.token`, created with mode `0600`.  A connection whose
//! first frame is missing, oversized, malformed or carries another token is
//! closed without a single frame being sent, so the event stream (plan and
//! task progress, agent output, cost events) reaches only callers that can read
//! that file.  This is the socket's counterpart of the launch token in
//! `.roko/runtime/serve.token`: it admits the processes of the user who
//! started the server.  It does not hold back other processes of that same
//! user, or root, which can read the token file; roko treats those as the
//! operator.  What it adds to the socket's own mode `0600`:
//!
//! - the socket is bound before it is chmod-ed, so for a moment it carries
//!   umask-default permissions; the token file never does;
//! - some platforms ignore permissions on socket files, while every platform
//!   enforces them on a regular file.
//!
//! # Socket path
//!
//! `.roko/runtime/hub.sock` — alongside the daemon socket.  The socket and
//! `hub.token` are created when the server starts and removed when it exits.
//!
//! # Fallback
//!
//! When no socket exists (or the connection fails), `try_connect_hub_ipc`
//! returns `None` and the caller falls back to the existing file-polling path.

#[cfg(unix)]
mod unix {
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::time::Duration;

    use anyhow::{Context as _, Result};
    use serde::de::DeserializeOwned;
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

    /// The first frame a client sends: the token it read from `hub.token`.
    ///
    /// No `Debug` derive, so the token cannot end up in a log line.
    #[derive(Serialize, Deserialize)]
    struct HubIpcHello {
        token: String,
    }

    /// Largest hello frame the server reads. A token is 64 bytes; the cap
    /// stops a client from making the server allocate a large buffer.
    const MAX_HELLO_FRAME_BYTES: u32 = 4096;

    /// How long the server waits for a client's hello before closing.
    const HELLO_TIMEOUT: Duration = Duration::from_secs(5);

    /// Write a length-prefixed JSON frame to `stream`.
    ///
    /// Format: 4-byte big-endian body length + UTF-8 JSON body.
    async fn write_frame<T: Serialize>(stream: &mut UnixStream, msg: &T) -> Result<()> {
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
    ///
    /// A declared length above `max_len` fails before anything is allocated.
    async fn read_frame<T: DeserializeOwned>(stream: &mut UnixStream, max_len: u32) -> Result<T> {
        let mut len_buf = [0u8; 4];
        stream
            .read_exact(&mut len_buf)
            .await
            .context("read IPC frame length")?;
        let len = u32::from_be_bytes(len_buf);
        anyhow::ensure!(
            len <= max_len,
            "IPC frame of {len} bytes exceeds the {max_len}-byte limit"
        );
        let mut body = vec![0u8; len as usize];
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

    /// `.roko/runtime/hub.token` — the per-run token a client must present
    /// before the server sends it anything.
    pub fn hub_token_path(workdir: &Path) -> PathBuf {
        workdir.join(".roko").join("runtime").join("hub.token")
    }

    // ── Token ────────────────────────────────────────────────────────────────

    /// A fresh random token: two v4 UUIDs, as the serve launch token uses.
    fn mint_hub_token() -> String {
        format!(
            "{}{}",
            uuid::Uuid::new_v4().as_simple(),
            uuid::Uuid::new_v4().as_simple()
        )
    }

    /// Write `token` to `path`, readable and writable only by its owner.
    ///
    /// The file is created with mode `0600` under a temporary name and then
    /// renamed into place, so it is never wider than `0600`, even briefly.
    fn write_hub_token(path: &Path, token: &str) -> Result<()> {
        use std::io::Write as _;
        use std::os::unix::fs::OpenOptionsExt as _;

        let tmp = path.with_extension("token.tmp");
        // A leftover from a crashed start; `create_new` below refuses to
        // reuse it (or to follow a link planted in its place).
        let _ = std::fs::remove_file(&tmp);
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&tmp)
            .with_context(|| format!("create hub token {}", tmp.display()))?;
        file.write_all(token.as_bytes())
            .with_context(|| format!("write hub token {}", tmp.display()))?;
        drop(file);
        std::fs::rename(&tmp, path)
            .with_context(|| format!("install hub token {}", path.display()))?;
        Ok(())
    }

    /// Compare two tokens without stopping at the first differing byte.
    fn tokens_match(presented: &str, expected: &str) -> bool {
        let (presented, expected) = (presented.as_bytes(), expected.as_bytes());
        if presented.len() != expected.len() {
            return false;
        }
        let diff = presented
            .iter()
            .zip(expected)
            .fold(0u8, |diff, (lhs, rhs)| diff | (lhs ^ rhs));
        std::hint::black_box(diff) == 0
    }

    /// Read the client's hello and check the token it carries.
    ///
    /// Fails when no hello arrives within [`HELLO_TIMEOUT`], when the frame
    /// is oversized or malformed, and when the token does not match.
    async fn authenticate_client(stream: &mut UnixStream, expected: &str) -> Result<()> {
        let read = read_frame::<HubIpcHello>(stream, MAX_HELLO_FRAME_BYTES);
        let hello = tokio::time::timeout(HELLO_TIMEOUT, read)
            .await
            .context("no hub token within the handshake timeout")?
            .context("read hub hello")?;
        anyhow::ensure!(
            tokens_match(&hello.token, expected),
            "hub token does not match"
        );
        Ok(())
    }

    // ── Server ────────────────────────────────────────────────────────────────

    /// Start the StateHub IPC server.
    ///
    /// Writes a fresh token to `.roko/runtime/hub.token`, binds
    /// `.roko/runtime/hub.sock`, and accepts connections. A client that
    /// presents the token gets the current snapshot once, then live
    /// `DashboardEvent`s until the connection is closed or `shutdown` is
    /// cancelled; any other client is disconnected (see the module docs).
    ///
    /// Returns a `JoinHandle` for the accept loop.  The socket and the token
    /// file are removed when the task exits (either via shutdown or accept
    /// error).
    pub fn start_hub_ipc_server(
        hub: SharedStateHub,
        workdir: &Path,
        shutdown: CancellationToken,
    ) -> Result<JoinHandle<()>> {
        let socket_path = hub_socket_path(workdir);
        let token_path = hub_token_path(workdir);

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

        // Mint this run's token before binding, so every client that finds the
        // socket also finds the token it has to present.
        let token: Arc<str> = mint_hub_token().into();
        write_hub_token(&token_path, &token)?;

        let listener = UnixListener::bind(&socket_path)
            .with_context(|| format!("bind hub socket {}", socket_path.display()))?;

        // Restrict socket to owner-only (0600). The token handshake is the
        // check that holds while this chmod has not happened yet, and on
        // platforms that ignore socket-file permissions.
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
                        let token = Arc::clone(&token);
                        tokio::spawn(async move {
                            let result = handle_hub_connection(stream, hub_clone, &token).await;
                            if let Err(err) = result {
                                debug!(error = %err, "StateHub IPC connection closed");
                            }
                        });
                    }
                }
            }

            // Clean up the socket and token files on exit.
            let _ = tokio::fs::remove_file(&socket_path).await;
            let _ = tokio::fs::remove_file(&token_path).await;
        });

        Ok(handle)
    }

    /// Handle one accepted IPC connection.
    ///
    /// Checks the client's hub token, then sends a `snapshot` frame and
    /// streams `event` frames until the connection is dropped or the hub's
    /// broadcast channel lags. A client that fails the check is dropped
    /// before anything is sent to it.
    async fn handle_hub_connection(
        mut stream: UnixStream,
        hub: SharedStateHub,
        token: &str,
    ) -> Result<()> {
        if let Err(err) = authenticate_client(&mut stream, token).await {
            warn!(error = %err, "StateHub IPC client rejected");
            return Ok(());
        }

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
    /// If `.roko/runtime/hub.sock` exists and is connectable, presents the
    /// token from `.roko/runtime/hub.token` and returns a [`SharedStateHub`]
    /// pre-seeded with the full current snapshot and wired to stream live
    /// events from the remote hub into its in-process event bus.
    ///
    /// Returns `None` when no socket exists, the token file cannot be read,
    /// the connection is refused or rejected, or any protocol error occurs.
    /// The caller should fall back to file polling.
    pub async fn try_connect_hub_ipc(workdir: &Path) -> Option<SharedStateHub> {
        let socket_path = hub_socket_path(workdir);
        if !socket_path.exists() {
            return None;
        }

        match connect_hub_ipc(socket_path, hub_token_path(workdir)).await {
            Ok(hub) => Some(hub),
            Err(err) => {
                debug!(error = %err, "StateHub IPC connect failed; falling back to file polling");
                None
            }
        }
    }

    async fn connect_hub_ipc(socket_path: PathBuf, token_path: PathBuf) -> Result<SharedStateHub> {
        let token = tokio::fs::read_to_string(&token_path)
            .await
            .with_context(|| format!("read hub token {}", token_path.display()))?;
        let token = token.trim().to_string();

        let mut stream = UnixStream::connect(&socket_path)
            .await
            .with_context(|| format!("connect hub socket {}", socket_path.display()))?;

        write_frame(&mut stream, &HubIpcHello { token })
            .await
            .context("send hub token")?;

        // Read the first frame — must be a snapshot. The server rejects a
        // wrong token by closing the connection, which fails this read.
        let first: HubIpcMessage = read_frame(&mut stream, u32::MAX)
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
                match read_frame::<HubIpcMessage>(&mut stream, u32::MAX).await {
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
pub use unix::{hub_socket_path, hub_token_path, start_hub_ipc_server, try_connect_hub_ipc};

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
