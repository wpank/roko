//! Integration tests for the StateHub IPC socket round-trip.
//!
//! These tests are Unix-only (Unix sockets).  All `workdir`s are created under
//! `/tmp` so the socket path stays well within the 104-byte limit imposed by
//! most kernels for `AF_UNIX` paths.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt as _;
use std::time::Duration;

use roko_cli::state_hub::SharedStateHub;
use roko_cli::state_hub_ipc::{
    hub_socket_path, hub_token_path, start_hub_ipc_server, try_connect_hub_ipc,
};
use roko_core::dashboard_snapshot::DashboardEvent;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::UnixStream;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

/// Create a temp directory under `/tmp` to keep socket paths short.
fn tmp_workdir() -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix("hub_ipc_")
        .tempdir_in("/tmp")
        .expect("create tempdir under /tmp")
}

/// Send a hello frame: a 4-byte big-endian length, then `{"token": ...}`.
async fn send_hello(stream: &mut UnixStream, token: &str) {
    let body = serde_json::to_vec(&serde_json::json!({ "token": token })).expect("encode hello");
    let len = u32::try_from(body.len()).expect("hello length fits u32");
    stream
        .write_all(&len.to_be_bytes())
        .await
        .expect("write hello length");
    stream.write_all(&body).await.expect("write hello body");
}

/// Everything the server sends before it closes `stream`. A reset counts as
/// a close; the bytes read before it are kept.
async fn read_until_closed(mut stream: UnixStream) -> Vec<u8> {
    let mut received = Vec::new();
    let _ = timeout(Duration::from_secs(10), stream.read_to_end(&mut received))
        .await
        .expect("server closed the connection within 10 seconds");
    received
}

// ── T1 + T2: snapshot on connect + live event delivered ─────────────────────

/// Publishing a `PlanStarted` event *before* starting the server and then
/// connecting should give a mirror whose initial snapshot already contains
/// the plan.  A `TaskStarted` published *after* connecting should arrive at
/// the mirror within two seconds.
#[tokio::test]
async fn snapshot_preloaded_and_live_event_delivered() {
    let workdir = tmp_workdir();
    let hub = SharedStateHub::new_in_process();
    let shutdown = CancellationToken::new();

    // Publish PlanStarted into the server hub BEFORE the server is up.
    hub.publish(DashboardEvent::PlanStarted {
        plan_id: "test-plan".to_string(),
        tasks_total: 3,
    });

    let _server = start_hub_ipc_server(hub.clone(), workdir.path(), shutdown.clone())
        .expect("bind IPC server");

    // Connect — the mirror must see the pre-existing plan in its snapshot.
    let mirror = timeout(Duration::from_secs(5), try_connect_hub_ipc(workdir.path()))
        .await
        .expect("connect did not time out")
        .expect("mirror hub must be Some");

    // ── T1: snapshot already contains the plan ───────────────────────────────
    // Give the background reader task a moment to process the snapshot frame.
    tokio::time::sleep(Duration::from_millis(100)).await;
    let snap = mirror.current_snapshot();
    assert!(
        snap.plans.contains_key("test-plan"),
        "mirror snapshot must contain the plan that was published before connect; \
         got plans: {:?}",
        snap.plans.keys().collect::<Vec<_>>()
    );

    // ── T2: live event delivered within 2 s ──────────────────────────────────
    // Subscribe to the mirror's broadcast channel BEFORE publishing so we
    // don't race.
    let mut live_rx = mirror.subscribe_events();

    hub.publish(DashboardEvent::TaskStarted {
        plan_id: "test-plan".to_string(),
        task_id: "T01".to_string(),
        title: "First task".to_string(),
        phase: "implementing".to_string(),
    });

    let envelope = timeout(Duration::from_secs(2), live_rx.recv())
        .await
        .expect("TaskStarted arrived within 2 seconds")
        .expect("broadcast channel still open");

    match envelope.payload {
        DashboardEvent::TaskStarted {
            plan_id, task_id, ..
        } => {
            assert_eq!(plan_id, "test-plan");
            assert_eq!(task_id, "T01");
        }
        other => panic!("expected TaskStarted, got {other:?}"),
    }
}

// ── T3: socket mode is 0600 ──────────────────────────────────────────────────

#[tokio::test]
async fn socket_file_has_mode_0600() {
    let workdir = tmp_workdir();
    let hub = SharedStateHub::new_in_process();
    let shutdown = CancellationToken::new();

    let _server = start_hub_ipc_server(hub, workdir.path(), shutdown).expect("bind IPC server");

    let sock = hub_socket_path(workdir.path());
    let mode = std::fs::metadata(&sock)
        .unwrap_or_else(|e| panic!("stat {}: {e}", sock.display()))
        .permissions()
        .mode();

    // Mask off the file-type bits; keep only the 12 permission bits.
    let perm_bits = mode & 0o7777;
    assert_eq!(
        perm_bits, 0o600,
        "socket permissions should be 0600, got {perm_bits:o}"
    );
}

// ── T4: cancellation stops server and removes socket ────────────────────────

#[tokio::test]
async fn cancellation_removes_socket_file() {
    let workdir = tmp_workdir();
    let hub = SharedStateHub::new_in_process();
    let shutdown = CancellationToken::new();

    let server =
        start_hub_ipc_server(hub, workdir.path(), shutdown.clone()).expect("bind IPC server");

    let sock = hub_socket_path(workdir.path());
    assert!(sock.exists(), "socket must exist before cancellation");

    // Cancel and wait for the server task to finish.
    shutdown.cancel();
    timeout(Duration::from_secs(5), server)
        .await
        .expect("server task finished within 5 seconds")
        .expect("server task did not panic");

    assert!(
        !sock.exists(),
        "socket file must be removed after server shutdown"
    );
    assert!(
        !hub_token_path(workdir.path()).exists(),
        "hub token must be removed after server shutdown"
    );
}

// ── T5: no socket → try_connect returns None ────────────────────────────────

#[tokio::test]
async fn no_socket_returns_none() {
    let workdir = tmp_workdir();
    // No server started — the socket file does not exist.
    let result = try_connect_hub_ipc(workdir.path()).await;
    assert!(
        result.is_none(),
        "expected None when no socket exists, got Some(…)"
    );
}

// ── T6: the event stream requires the hub token ─────────────────────────────

/// A client that sends no hello, or a hello with the wrong token, is
/// disconnected before the server sends it a single frame. A client that
/// presents the token from `hub.token` gets the snapshot.
#[tokio::test]
async fn hub_ipc_rejects_connection_without_token() {
    let workdir = tmp_workdir();
    let hub = SharedStateHub::new_in_process();
    let shutdown = CancellationToken::new();
    hub.publish(DashboardEvent::PlanStarted {
        plan_id: "secret-plan".to_string(),
        tasks_total: 1,
    });

    let _server =
        start_hub_ipc_server(hub, workdir.path(), shutdown.clone()).expect("bind IPC server");
    let sock = hub_socket_path(workdir.path());

    // No hello at all: the client closes its write half, as a client that
    // never heard of the token would after connecting.
    let mut silent = UnixStream::connect(&sock).await.expect("connect");
    silent.shutdown().await.expect("close write half");
    let received = read_until_closed(silent).await;
    assert!(
        received.is_empty(),
        "a client without a token must receive nothing; got {} bytes",
        received.len()
    );

    // A well-formed hello with the wrong token.
    let mut wrong = UnixStream::connect(&sock).await.expect("connect");
    send_hello(&mut wrong, "not-the-hub-token").await;
    let received = read_until_closed(wrong).await;
    assert!(
        received.is_empty(),
        "a client with a wrong token must receive nothing; got {} bytes",
        received.len()
    );

    // An oversized frame is refused before the server allocates for it.
    let mut oversized = UnixStream::connect(&sock).await.expect("connect");
    oversized
        .write_all(&u32::MAX.to_be_bytes())
        .await
        .expect("write oversized length");
    let received = read_until_closed(oversized).await;
    assert!(
        received.is_empty(),
        "a client with an oversized hello must receive nothing; got {} bytes",
        received.len()
    );

    // Control: the right token gets the snapshot, so the rejections above
    // came from the check and not from a server that was not serving.
    let token = std::fs::read_to_string(hub_token_path(workdir.path())).expect("read hub token");
    let mut authorized = UnixStream::connect(&sock).await.expect("connect");
    send_hello(&mut authorized, token.trim()).await;
    let mut len_buf = [0u8; 4];
    timeout(Duration::from_secs(5), authorized.read_exact(&mut len_buf))
        .await
        .expect("snapshot frame arrived within 5 seconds")
        .expect("read snapshot frame length");
    assert!(
        u32::from_be_bytes(len_buf) > 0,
        "an authorized client must receive a non-empty snapshot frame"
    );

    shutdown.cancel();
}

// ── T7: the token file is private and fresh per run ─────────────────────────

#[tokio::test]
async fn hub_token_file_has_mode_0600_and_changes_per_run() {
    let workdir = tmp_workdir();
    let token_path = hub_token_path(workdir.path());

    let first_shutdown = CancellationToken::new();
    let first = start_hub_ipc_server(
        SharedStateHub::new_in_process(),
        workdir.path(),
        first_shutdown.clone(),
    )
    .expect("bind IPC server");
    let mode = std::fs::metadata(&token_path)
        .unwrap_or_else(|e| panic!("stat {}: {e}", token_path.display()))
        .permissions()
        .mode();
    let perm_bits = mode & 0o7777;
    assert_eq!(
        perm_bits, 0o600,
        "hub token permissions should be 0600, got {perm_bits:o}"
    );
    let first_token = std::fs::read_to_string(&token_path).expect("read first token");
    assert!(
        first_token.trim().len() >= 32,
        "hub token must be a long random string"
    );
    first_shutdown.cancel();
    timeout(Duration::from_secs(5), first)
        .await
        .expect("first server finished within 5 seconds")
        .expect("first server did not panic");

    let second_shutdown = CancellationToken::new();
    let _second = start_hub_ipc_server(
        SharedStateHub::new_in_process(),
        workdir.path(),
        second_shutdown.clone(),
    )
    .expect("bind IPC server again");
    let second_token = std::fs::read_to_string(&token_path).expect("read second token");
    assert_ne!(
        first_token, second_token,
        "each server run must mint a new hub token"
    );
    second_shutdown.cancel();
}
