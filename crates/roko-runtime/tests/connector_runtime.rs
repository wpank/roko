//! End-to-end coverage for concrete connector transports and supervision.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use roko_core::connector::{
    Connect, ConnectConfig, ConnectHealthStatus, ConnectorKind, ConnectorManifest,
    ConnectorRegistry, ConnectorStatus, ExecuteRequest, ExecuteResponse, QueryRequest,
    QueryResponse, ReconnectStrategy,
};
use roko_core::{Result, RokoError};
use roko_runtime::{
    ConnectorRuntime, ConnectorSupervisorOptions, ConnectorSupervisorState, MAX_HTTP_JSON_BYTES,
    MAX_RECONNECT_ATTEMPTS,
};
use serde_json::{Value, json};
use tokio::sync::{Notify, RwLock};

fn manifest(strategy: ReconnectStrategy) -> ConnectorManifest {
    ConnectorManifest {
        name: "example".to_owned(),
        kind: ConnectorKind::Api,
        version: "1".to_owned(),
        description: "test connector".to_owned(),
        config_schema: None,
        capabilities: vec!["query".to_owned(), "execute".to_owned()],
        health_interval_secs: 60,
        reconnect_strategy: strategy,
    }
}

fn config(endpoint: String) -> ConnectConfig {
    ConnectConfig {
        endpoint,
        auth: None,
        headers: None,
        timeout_ms: 2_000,
    }
}

fn runtime() -> (ConnectorRuntime, Arc<RwLock<ConnectorRegistry>>) {
    let registry = Arc::new(RwLock::new(ConnectorRegistry::new()));
    (ConnectorRuntime::new(Arc::clone(&registry)), registry)
}

#[derive(Clone)]
struct HttpState {
    auth_seen: Arc<AtomicBool>,
}

async fn root(State(state): State<HttpState>, headers: HeaderMap) -> StatusCode {
    if headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        == Some("Bearer transport-secret")
    {
        state.auth_seen.store(true, Ordering::SeqCst);
        StatusCode::NO_CONTENT
    } else {
        StatusCode::UNAUTHORIZED
    }
}

async fn lookup(Query(params): Query<HashMap<String, String>>) -> Json<Value> {
    Json(json!({ "id": params.get("id") }))
}

async fn mutate(Json(body): Json<Value>) -> Json<Value> {
    Json(json!({ "accepted": body }))
}

async fn large() -> String {
    "x".repeat(MAX_HTTP_JSON_BYTES + 1)
}

async fn spawn_http_server() -> (String, Arc<AtomicBool>, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind test HTTP server");
    let address = listener.local_addr().expect("test server address");
    let auth_seen = Arc::new(AtomicBool::new(false));
    let app = Router::new()
        .route("/", get(root))
        .route("/lookup", get(lookup))
        .route("/mutate", post(mutate))
        .route("/large", get(large))
        .with_state(HttpState {
            auth_seen: Arc::clone(&auth_seen),
        });
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve test HTTP");
    });
    (format!("http://{address}/"), auth_seen, task)
}

#[tokio::test]
async fn http_transport_checks_real_health_and_executes_without_exposing_secrets() {
    let (endpoint, auth_seen, server) = spawn_http_server().await;
    let (runtime, registry) = runtime();
    let mut connector_config = config(endpoint);
    connector_config.auth = Some("transport-secret".to_owned());
    connector_config.headers = Some(HashMap::from([(
        "x-private-token".to_owned(),
        "header-secret".to_owned(),
    )]));

    let status = runtime
        .register_http(
            manifest(ReconnectStrategy::Manual),
            connector_config,
            ConnectorSupervisorOptions::default(),
        )
        .await
        .expect("register healthy HTTP connector");
    assert_eq!(status.connector.health.status, ConnectorStatus::Connected);
    assert!(auth_seen.load(Ordering::SeqCst));
    assert_eq!(registry.read().await.healthy_count(), 1);

    let query = runtime
        .query(
            "example",
            QueryRequest {
                operation: "lookup".to_owned(),
                params: json!({ "id": "42" }),
            },
        )
        .await
        .expect("HTTP query");
    assert_eq!(query.data, json!({ "id": "42" }));

    let execution = runtime
        .execute(
            "example",
            ExecuteRequest {
                operation: "mutate".to_owned(),
                params: json!({ "value": 7 }),
            },
        )
        .await
        .expect("HTTP execute");
    assert_eq!(execution.result, json!({ "accepted": { "value": 7 } }));

    let status_json = serde_json::to_string(&status).expect("serialize status");
    let descriptor_json =
        serde_json::to_string(registry.read().await.get("example").expect("descriptor"))
            .expect("serialize descriptor");
    for secret in ["transport-secret", "header-secret", "x-private-token"] {
        assert!(!status_json.contains(secret));
        assert!(!descriptor_json.contains(secret));
    }

    let oversized = runtime
        .query(
            "example",
            QueryRequest {
                operation: "large".to_owned(),
                params: Value::Null,
            },
        )
        .await;
    assert!(oversized.is_err());

    runtime.shutdown().await;
    server.abort();
}

#[tokio::test]
async fn failed_real_health_never_reports_connected() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind failing server");
    let address = listener.local_addr().expect("server address");
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            Router::new().route("/", get(|| async { StatusCode::SERVICE_UNAVAILABLE })),
        )
        .await
        .expect("serve failing HTTP");
    });
    let (runtime, registry) = runtime();
    let status = runtime
        .register_http(
            manifest(ReconnectStrategy::Manual),
            config(format!("http://{address}/")),
            ConnectorSupervisorOptions::default(),
        )
        .await
        .expect("register disconnected connector");
    assert_eq!(
        status.connector.health.status,
        ConnectorStatus::Disconnected
    );
    assert_eq!(status.supervisor.state, ConnectorSupervisorState::Manual);
    assert_eq!(registry.read().await.healthy_count(), 0);
    runtime.shutdown().await;
    server.abort();
}

struct ScriptedConnector {
    connect_calls: Arc<AtomicU32>,
    disconnect_calls: Arc<AtomicU32>,
    failures_remaining: Arc<AtomicU32>,
    connected: AtomicBool,
}

#[async_trait]
impl Connect for ScriptedConnector {
    async fn connect(&mut self, _config: &ConnectConfig) -> Result<()> {
        self.connect_calls.fetch_add(1, Ordering::SeqCst);
        let should_fail = self
            .failures_remaining
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |remaining| {
                if remaining > 0 {
                    Some(remaining - 1)
                } else {
                    None
                }
            })
            .is_ok();
        if should_fail {
            self.connected.store(false, Ordering::SeqCst);
            return Err(RokoError::transport(
                "upstream failed with credential=must-not-leak",
            ));
        }
        self.connected.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn query(&self, request: QueryRequest) -> Result<QueryResponse> {
        Ok(QueryResponse {
            data: request.params,
            latency_ms: 0,
        })
    }

    async fn execute(&self, request: ExecuteRequest) -> Result<ExecuteResponse> {
        Ok(ExecuteResponse {
            result: request.params,
            latency_ms: 0,
        })
    }

    async fn health(&self) -> Result<ConnectHealthStatus> {
        Ok(ConnectHealthStatus {
            status: if self.connected.load(Ordering::SeqCst) {
                ConnectorStatus::Connected
            } else {
                ConnectorStatus::Disconnected
            },
            latency_ms: 0,
            last_check: chrono::Utc::now(),
            error: None,
        })
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.disconnect_calls.fetch_add(1, Ordering::SeqCst);
        self.connected.store(false, Ordering::SeqCst);
        Ok(())
    }
}

fn scripted(
    failures: u32,
) -> (
    ScriptedConnector,
    Arc<AtomicU32>,
    Arc<AtomicU32>,
    Arc<AtomicU32>,
) {
    let connect_calls = Arc::new(AtomicU32::new(0));
    let disconnect_calls = Arc::new(AtomicU32::new(0));
    let failures_remaining = Arc::new(AtomicU32::new(failures));
    (
        ScriptedConnector {
            connect_calls: Arc::clone(&connect_calls),
            disconnect_calls: Arc::clone(&disconnect_calls),
            failures_remaining: Arc::clone(&failures_remaining),
            connected: AtomicBool::new(false),
        },
        connect_calls,
        disconnect_calls,
        failures_remaining,
    )
}

async fn wait_for_state(
    runtime: &ConnectorRuntime,
    expected: ConnectorSupervisorState,
) -> roko_runtime::ConnectorRuntimeStatus {
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let status = runtime.status("example").await.expect("runtime status");
            if status.supervisor.state == expected {
                return status;
            }
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
    })
    .await
    .expect("supervisor reached expected state")
}

async fn wait_for_counter(counter: &AtomicU32, expected: u32) {
    tokio::time::timeout(Duration::from_secs(1), async {
        while counter.load(Ordering::SeqCst) < expected {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("counter reached expected value");
}

#[tokio::test]
async fn supervisor_recovers_then_explicit_restart_resets_the_failure_budget() {
    let (runtime, _) = runtime();
    let (connector, connect_calls, disconnect_calls, _) = scripted(1);
    let status = runtime
        .register_connector(
            manifest(ReconnectStrategy::FixedInterval { interval_ms: 1 }),
            config("http://unused.test".to_owned()),
            Box::new(connector),
            "scripted",
            ConnectorSupervisorOptions {
                max_reconnect_attempts: 2,
            },
        )
        .await
        .expect("register scripted connector");
    assert_eq!(
        status.connector.health.status,
        ConnectorStatus::Disconnected
    );

    let recovered = wait_for_state(&runtime, ConnectorSupervisorState::Monitoring).await;
    assert_eq!(
        recovered.connector.health.status,
        ConnectorStatus::Connected
    );
    assert_eq!(recovered.supervisor.reconnect_attempts, 1);
    assert_eq!(connect_calls.load(Ordering::SeqCst), 2);

    let restarted = runtime.restart("example").await.expect("restart connector");
    assert_eq!(
        restarted.connector.health.status,
        ConnectorStatus::Connected
    );
    assert_eq!(restarted.supervisor.restart_count, 1);
    assert_eq!(restarted.supervisor.reconnect_attempts, 0);
    assert!(disconnect_calls.load(Ordering::SeqCst) >= 2);
    runtime.shutdown().await;
}

#[tokio::test]
async fn reconnect_attempts_stop_exactly_at_the_configured_bound() {
    let (runtime, _) = runtime();
    let (connector, connect_calls, _, _) = scripted(u32::MAX);
    runtime
        .register_connector(
            manifest(ReconnectStrategy::FixedInterval { interval_ms: 1 }),
            config("http://unused.test".to_owned()),
            Box::new(connector),
            "scripted",
            ConnectorSupervisorOptions {
                max_reconnect_attempts: 2,
            },
        )
        .await
        .expect("register always-failing connector");

    let exhausted = wait_for_state(&runtime, ConnectorSupervisorState::Exhausted).await;
    assert_eq!(exhausted.supervisor.reconnect_attempts, 2);
    assert_eq!(exhausted.supervisor.consecutive_failures, 2);
    assert_eq!(
        exhausted.supervisor.last_error.as_deref(),
        Some("transport unavailable")
    );
    assert_eq!(connect_calls.load(Ordering::SeqCst), 3);
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert_eq!(connect_calls.load(Ordering::SeqCst), 3);
    let encoded = serde_json::to_string(&exhausted).expect("serialize exhausted status");
    assert!(!encoded.contains("must-not-leak"));
    runtime.shutdown().await;
}

#[tokio::test]
async fn cancelled_replacement_or_unregister_keeps_the_published_generation_live() {
    let (runtime_value, registry) = runtime();
    let runtime = Arc::new(runtime_value);
    let (initial, _, _, _) = scripted(0);
    let initial_status = runtime
        .register_connector(
            manifest(ReconnectStrategy::Manual),
            config("http://unused.test".to_owned()),
            Box::new(initial),
            "scripted",
            ConnectorSupervisorOptions::default(),
        )
        .await
        .expect("register initial connector");
    let initial_generation = initial_status.connector.metadata["generation"]
        .as_u64()
        .expect("initial generation");

    // Force replacement to suspend after preparing the new connector but
    // before its cancellation-free deactivation/publication step.
    let registry_guard = registry.write().await;
    let replacement_runtime = Arc::clone(&runtime);
    let (replacement, replacement_connects, replacement_disconnects, _) = scripted(0);
    let mut replacement_task = tokio::spawn(async move {
        replacement_runtime
            .register_connector(
                manifest(ReconnectStrategy::Manual),
                config("http://replacement.test".to_owned()),
                Box::new(replacement),
                "scripted",
                ConnectorSupervisorOptions::default(),
            )
            .await
    });
    wait_for_counter(&replacement_connects, 1).await;
    assert!(
        tokio::time::timeout(Duration::from_millis(50), &mut replacement_task)
            .await
            .is_err(),
        "prepared replacement must be suspended on the held registry lock"
    );
    replacement_task.abort();
    let _ = replacement_task.await;
    drop(registry_guard);
    wait_for_counter(&replacement_disconnects, 1).await;

    let after_replacement = runtime
        .status("example")
        .await
        .expect("initial status survives");
    assert_eq!(
        after_replacement.connector.metadata["generation"].as_u64(),
        Some(initial_generation)
    );
    assert_eq!(
        after_replacement.connector.health.status,
        ConnectorStatus::Connected
    );

    // The same invariant applies when deletion is cancelled while waiting for
    // the canonical registry lock.
    let registry_guard = registry.write().await;
    let unregister_runtime = Arc::clone(&runtime);
    let mut unregister_task =
        tokio::spawn(async move { unregister_runtime.unregister("example").await });
    assert!(
        tokio::time::timeout(Duration::from_millis(50), &mut unregister_task)
            .await
            .is_err(),
        "unregister must be suspended on the held registry lock"
    );
    unregister_task.abort();
    let _ = unregister_task.await;
    drop(registry_guard);

    let after_unregister = runtime
        .status("example")
        .await
        .expect("live status survives");
    assert_eq!(
        after_unregister.connector.metadata["generation"].as_u64(),
        Some(initial_generation)
    );
    runtime
        .query(
            "example",
            QueryRequest {
                operation: "still-live".to_owned(),
                params: json!({ "ok": true }),
            },
        )
        .await
        .expect("published generation remains queryable");
    runtime.shutdown().await;
}

struct RestartBlockingConnector {
    connect_calls: AtomicU32,
    restart_started: Arc<Notify>,
    release_restart: Arc<Notify>,
    connected: AtomicBool,
}

#[async_trait]
impl Connect for RestartBlockingConnector {
    async fn connect(&mut self, _config: &ConnectConfig) -> Result<()> {
        let call = self.connect_calls.fetch_add(1, Ordering::SeqCst);
        if call > 0 {
            self.connected.store(false, Ordering::SeqCst);
            self.restart_started.notify_one();
            self.release_restart.notified().await;
        }
        self.connected.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn query(&self, request: QueryRequest) -> Result<QueryResponse> {
        Ok(QueryResponse {
            data: request.params,
            latency_ms: 0,
        })
    }

    async fn execute(&self, request: ExecuteRequest) -> Result<ExecuteResponse> {
        Ok(ExecuteResponse {
            result: request.params,
            latency_ms: 0,
        })
    }

    async fn health(&self) -> Result<ConnectHealthStatus> {
        Ok(ConnectHealthStatus {
            status: if self.connected.load(Ordering::SeqCst) {
                ConnectorStatus::Connected
            } else {
                ConnectorStatus::Disconnected
            },
            latency_ms: 0,
            last_check: chrono::Utc::now(),
            error: None,
        })
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.connected.store(false, Ordering::SeqCst);
        Ok(())
    }
}

#[tokio::test]
async fn cancelled_restart_never_leaves_a_connected_canonical_ghost() {
    let (runtime_value, _) = runtime();
    let runtime = Arc::new(runtime_value);
    let restart_started = Arc::new(Notify::new());
    let release_restart = Arc::new(Notify::new());
    runtime
        .register_connector(
            manifest(ReconnectStrategy::Manual),
            config("http://unused.test".to_owned()),
            Box::new(RestartBlockingConnector {
                connect_calls: AtomicU32::new(0),
                restart_started: Arc::clone(&restart_started),
                release_restart,
                connected: AtomicBool::new(false),
            }),
            "restart_blocking",
            ConnectorSupervisorOptions::default(),
        )
        .await
        .expect("register restart connector");

    let restart_runtime = Arc::clone(&runtime);
    let restart_task = tokio::spawn(async move { restart_runtime.restart("example").await });
    tokio::time::timeout(Duration::from_secs(1), restart_started.notified())
        .await
        .expect("restart reached the concrete connect operation");
    let during_restart = runtime.status("example").await.expect("restart status");
    assert_eq!(
        during_restart.connector.health.status,
        ConnectorStatus::Disconnected
    );

    restart_task.abort();
    let _ = restart_task.await;
    tokio::task::yield_now().await;
    let cancelled = runtime
        .status("example")
        .await
        .expect("cancelled restart status");
    assert_eq!(
        cancelled.connector.health.status,
        ConnectorStatus::Disconnected
    );
    assert!(
        runtime
            .query(
                "example",
                QueryRequest {
                    operation: "must-not-run".to_owned(),
                    params: Value::Null,
                },
            )
            .await
            .is_err()
    );
    runtime.shutdown().await;
}

#[tokio::test]
async fn runtime_rejects_unbounded_or_secret_bearing_http_configuration() {
    let (runtime, _) = runtime();
    let too_many = runtime
        .register_http(
            manifest(ReconnectStrategy::Manual),
            config("http://127.0.0.1/".to_owned()),
            ConnectorSupervisorOptions {
                max_reconnect_attempts: MAX_RECONNECT_ATTEMPTS + 1,
            },
        )
        .await;
    assert!(too_many.is_err());

    for endpoint in [
        "https://user:secret@example.test/",
        "https://example.test/?api_key=secret",
        "file:///tmp/connector",
    ] {
        let result = runtime
            .register_http(
                manifest(ReconnectStrategy::Manual),
                config(endpoint.to_owned()),
                ConnectorSupervisorOptions::default(),
            )
            .await;
        assert!(result.is_err(), "endpoint should be rejected: {endpoint}");
    }

    let secret_config = ConnectConfig {
        endpoint: "https://example.test/".to_owned(),
        auth: Some("auth-secret".to_owned()),
        headers: Some(HashMap::from([(
            "authorization".to_owned(),
            "header-secret".to_owned(),
        )])),
        timeout_ms: 100,
    };
    let encoded = serde_json::to_string(&secret_config).expect("serialize config");
    let debugged = format!("{secret_config:?}");
    for secret in ["auth-secret", "header-secret", "example.test"] {
        assert!(!encoded.contains(secret) || secret == "example.test");
        assert!(!debugged.contains(secret));
    }
}
