//! Integration tests for the StateHub IPC socket round-trip.
//!
//! These tests are Unix-only (Unix sockets).  All `workdir`s are created under
//! `/tmp` so the socket path stays well within the 104-byte limit imposed by
//! most kernels for `AF_UNIX` paths.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt as _;
use std::time::Duration;

use roko_cli::state_hub::SharedStateHub;
use roko_cli::state_hub_ipc::{hub_socket_path, start_hub_ipc_server, try_connect_hub_ipc};
use roko_core::dashboard_snapshot::DashboardEvent;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

/// Create a temp directory under `/tmp` to keep socket paths short.
fn tmp_workdir() -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix("hub_ipc_")
        .tempdir_in("/tmp")
        .expect("create tempdir under /tmp")
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

    let _server =
        start_hub_ipc_server(hub.clone(), workdir.path(), shutdown.clone())
            .expect("bind IPC server");

    // Connect — the mirror must see the pre-existing plan in its snapshot.
    let mirror = timeout(
        Duration::from_secs(5),
        try_connect_hub_ipc(workdir.path()),
    )
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

    let _server =
        start_hub_ipc_server(hub, workdir.path(), shutdown).expect("bind IPC server");

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
