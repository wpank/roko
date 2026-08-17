//! Supervised runtime for concrete [`Connect`](roko_core::connector::Connect) transports.
//!
//! The runtime deliberately shares the canonical [`ConnectorRegistry`] instead
//! of maintaining a second descriptor or health catalog. Private transport
//! configuration stays in managed entries and is never returned by status APIs.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use chrono::Utc;
use parking_lot::Mutex as ParkingMutex;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderName, HeaderValue};
use roko_core::connector::{
    Connect, ConnectConfig, ConnectHealthStatus, ConnectorHealth, ConnectorInfo, ConnectorKind,
    ConnectorManifest, ConnectorRegistry, ConnectorStatus, ExecuteRequest, ExecuteResponse,
    QueryRequest, QueryResponse, ReconnectStrategy,
};
use roko_core::{Result, RokoError};
use serde::Serialize;
use serde_json::{Value, json};
use tokio::sync::{Mutex, RwLock, watch};
use tokio::task::JoinHandle;

/// Hard upper bound for automatic reconnect attempts in one failure episode.
pub const MAX_RECONNECT_ATTEMPTS: u32 = 10;
/// Hard upper bound for any configured reconnect delay.
pub const MAX_RECONNECT_DELAY_MS: u64 = 300_000;
/// Hard upper bound for periodic health intervals.
pub const MAX_HEALTH_INTERVAL_SECS: u64 = 3_600;
/// Hard upper bound for HTTP request timeouts.
pub const MAX_HTTP_TIMEOUT_MS: u64 = 120_000;
/// Hard upper bound for one JSON request or response body.
pub const MAX_HTTP_JSON_BYTES: usize = 1_048_576;
/// Hard upper bound for a relative operation path.
pub const MAX_OPERATION_BYTES: usize = 1_024;
/// Hard upper bound for live managed connectors in one runtime.
pub const MAX_MANAGED_CONNECTORS: usize = 64;
/// Hard upper bound for the configured endpoint URL.
pub const MAX_ENDPOINT_BYTES: usize = 2_048;
/// Hard upper bound for all configured HTTP header names and values.
pub const MAX_HEADER_BYTES: usize = 65_536;
/// Hard upper bound for all encoded query parameter names and values.
pub const MAX_QUERY_BYTES: usize = 65_536;

/// Canonical descriptor/health registry shared with the control plane.
pub type SharedConnectorRegistry = Arc<RwLock<ConnectorRegistry>>;

/// Runtime bounds applied in addition to the connector manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ConnectorSupervisorOptions {
    /// Maximum reconnect attempts after a connection or health failure.
    pub max_reconnect_attempts: u32,
}

impl Default for ConnectorSupervisorOptions {
    fn default() -> Self {
        Self {
            max_reconnect_attempts: 3,
        }
    }
}

impl ConnectorSupervisorOptions {
    fn validate(self) -> Result<()> {
        if self.max_reconnect_attempts == 0 || self.max_reconnect_attempts > MAX_RECONNECT_ATTEMPTS
        {
            return Err(RokoError::invalid(format!(
                "max_reconnect_attempts must be between 1 and {MAX_RECONNECT_ATTEMPTS}"
            )));
        }
        Ok(())
    }
}

/// Current state of the bounded supervisor, separate from transport health.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectorSupervisorState {
    /// Connected and periodically health checked.
    Monitoring,
    /// Waiting for or executing a bounded reconnect attempt.
    Reconnecting,
    /// Automatic reconnect is disabled by the manifest.
    Manual,
    /// The consecutive reconnect budget has been consumed.
    Exhausted,
    /// The lifecycle was explicitly stopped.
    Stopped,
}

/// Public, secret-free supervision counters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConnectorSupervisionStatus {
    /// Current supervisor lifecycle state.
    pub state: ConnectorSupervisorState,
    /// Total reconnect attempts in this managed lifecycle.
    pub reconnect_attempts: u32,
    /// Failed attempts in the current failure episode.
    pub consecutive_failures: u32,
    /// Total failed health/connect observations in this lifecycle.
    pub health_failures: u32,
    /// Maximum failed attempts allowed in one episode.
    pub max_reconnect_attempts: u32,
    /// Number of explicit restarts requested by the operator.
    pub restart_count: u32,
    /// Sanitized failure class; never a raw transport error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

/// Public runtime view. It intentionally omits [`ConnectConfig`].
#[derive(Debug, Clone, Serialize)]
pub struct ConnectorRuntimeStatus {
    /// Canonical descriptor and health snapshot.
    pub connector: ConnectorInfo,
    /// Secret-free bounded supervision state.
    pub supervisor: ConnectorSupervisionStatus,
}

struct SupervisorHandle {
    stop: watch::Sender<bool>,
    task: JoinHandle<()>,
}

struct AbortTaskOnDrop(Option<JoinHandle<()>>);

impl Drop for AbortTaskOnDrop {
    fn drop(&mut self) {
        if let Some(task) = self.0.take() {
            task.abort();
        }
    }
}

struct ManagedConnector {
    generation: u64,
    active: AtomicBool,
    connector: Mutex<Box<dyn Connect>>,
    config: ConnectConfig,
    manifest: ConnectorManifest,
    transport: &'static str,
    options: ConnectorSupervisorOptions,
    supervision: RwLock<ConnectorSupervisionStatus>,
    supervisor: ParkingMutex<Option<SupervisorHandle>>,
}

struct PreparedEntry(Option<Arc<ManagedConnector>>);

impl PreparedEntry {
    fn new(entry: Arc<ManagedConnector>) -> Self {
        Self(Some(entry))
    }

    fn disarm(&mut self) {
        self.0 = None;
    }
}

impl Drop for PreparedEntry {
    fn drop(&mut self) {
        if let Some(entry) = self.0.take() {
            spawn_cleanup(entry);
        }
    }
}

struct RestartRecovery {
    entry: Arc<ManagedConnector>,
    registry: SharedConnectorRegistry,
    armed: bool,
}

impl RestartRecovery {
    fn new(entry: Arc<ManagedConnector>, registry: SharedConnectorRegistry) -> Self {
        Self {
            entry,
            registry,
            armed: true,
        }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for RestartRecovery {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        let entry = Arc::clone(&self.entry);
        let registry = Arc::clone(&self.registry);
        tokio::spawn(async move {
            let connected = registry
                .read()
                .await
                .get(&entry.manifest.name)
                .and_then(|info| {
                    (info.metadata.get("generation").and_then(Value::as_u64)
                        == Some(entry.generation))
                    .then_some(info.health.status == ConnectorStatus::Connected)
                });
            let Some(connected) = connected else {
                return;
            };
            if !entry.active.load(Ordering::Acquire) {
                return;
            }
            let mut supervision = entry.supervision.write().await;
            supervision.state = if connected {
                ConnectorSupervisorState::Monitoring
            } else if matches!(entry.manifest.reconnect_strategy, ReconnectStrategy::Manual) {
                ConnectorSupervisorState::Manual
            } else {
                ConnectorSupervisorState::Reconnecting
            };
            drop(supervision);
            spawn_supervisor(Arc::clone(&entry), registry);
        });
    }
}

/// Owns concrete connector transports and their bounded health supervision.
pub struct ConnectorRuntime {
    registry: SharedConnectorRegistry,
    entries: RwLock<HashMap<String, Arc<ManagedConnector>>>,
    lifecycle: Mutex<()>,
    next_generation: AtomicU64,
}

impl ConnectorRuntime {
    /// Build a runtime backed by the shared canonical descriptor registry.
    #[must_use]
    pub fn new(registry: SharedConnectorRegistry) -> Self {
        Self {
            registry,
            entries: RwLock::new(HashMap::new()),
            lifecycle: Mutex::new(()),
            next_generation: AtomicU64::new(1),
        }
    }

    /// Register and start the built-in HTTP JSON transport.
    pub async fn register_http(
        &self,
        manifest: ConnectorManifest,
        config: ConnectConfig,
        options: ConnectorSupervisorOptions,
    ) -> Result<ConnectorRuntimeStatus> {
        if !matches!(
            manifest.kind,
            ConnectorKind::Api | ConnectorKind::Webhook | ConnectorKind::Exchange
        ) {
            return Err(RokoError::invalid(
                "http_json transport supports api, webhook, and exchange connector kinds",
            ));
        }
        HttpJsonConnector::validate_config(&config)?;
        self.register_connector(
            manifest,
            config,
            Box::new(HttpJsonConnector::new()),
            "http_json",
            options,
        )
        .await
    }

    /// Register an extension-provided transport behind the canonical contract.
    pub async fn register_connector(
        &self,
        manifest: ConnectorManifest,
        config: ConnectConfig,
        connector: Box<dyn Connect>,
        transport: &'static str,
        options: ConnectorSupervisorOptions,
    ) -> Result<ConnectorRuntimeStatus> {
        manifest.validate()?;
        config.validate()?;
        options.validate()?;
        validate_runtime_bounds(&manifest)?;

        let entry = Arc::new(ManagedConnector {
            generation: self.next_generation.fetch_add(1, Ordering::Relaxed),
            active: AtomicBool::new(true),
            connector: Mutex::new(connector),
            config,
            manifest: manifest.clone(),
            transport,
            options,
            supervision: RwLock::new(ConnectorSupervisionStatus {
                state: ConnectorSupervisorState::Monitoring,
                reconnect_attempts: 0,
                consecutive_failures: 0,
                health_failures: 0,
                max_reconnect_attempts: options.max_reconnect_attempts,
                restart_count: 0,
                last_error: None,
            }),
            supervisor: ParkingMutex::new(None),
        });
        // If this future is cancelled anywhere before publication, disconnect
        // the already-prepared concrete transport in a detached cleanup task.
        let mut prepared = PreparedEntry::new(Arc::clone(&entry));

        let health = connect_and_probe(&entry).await;
        set_state_after_probe(&entry, &health).await;

        // Connect and probe off-map. Only once the replacement is ready do we
        // briefly take lifecycle + map/registry locks and publish both views.
        // Cancellation before this point leaves the previous transport live.
        let supervision = entry.supervision.read().await.clone();
        let _lifecycle = self.lifecycle.lock().await;
        let mut entries = self.entries.write().await;
        let previous = entries.get(&manifest.name).cloned();
        if previous.is_none() && entries.len() >= MAX_MANAGED_CONNECTORS {
            return Err(RokoError::invalid(format!(
                "managed connector count must not exceed {MAX_MANAGED_CONNECTORS}"
            )));
        }
        let mut registry = self.registry.write().await;
        let created_at = registry
            .get(&manifest.name)
            .map(|info| info.created_at)
            .unwrap_or_else(Utc::now);
        // There are no cancellation points between deactivating the old
        // generation and publishing the replacement in both canonical views.
        if let Some(previous) = &previous {
            previous.active.store(false, Ordering::Release);
        }
        entries.insert(manifest.name.clone(), Arc::clone(&entry));
        registry.register(canonical_info(&entry, &health, supervision, created_at));
        drop(registry);
        drop(entries);
        spawn_supervisor(Arc::clone(&entry), Arc::clone(&self.registry));
        prepared.disarm();
        if let Some(previous) = previous {
            spawn_cleanup(previous);
        }
        self.status(&manifest.name).await
    }

    /// Return a secret-free live status view.
    pub async fn status(&self, name: &str) -> Result<ConnectorRuntimeStatus> {
        let entry = self.entry(name).await?;
        let connector = self
            .registry
            .read()
            .await
            .get(name)
            .cloned()
            .ok_or_else(|| RokoError::user(format!("connector '{name}' not found")))?;
        if !entry.active.load(Ordering::Acquire)
            || connector.metadata.get("generation").and_then(Value::as_u64)
                != Some(entry.generation)
        {
            return Err(RokoError::transport(
                "connector lifecycle changed while reading status",
            ));
        }
        let supervisor = entry.supervision.read().await.clone();
        Ok(ConnectorRuntimeStatus {
            connector,
            supervisor,
        })
    }

    /// Execute an immediate real transport health check.
    pub async fn refresh_health(&self, name: &str) -> Result<ConnectorRuntimeStatus> {
        let entry = self.entry(name).await?;
        let health = probe_health(&entry).await;
        set_state_after_probe(&entry, &health).await;
        update_canonical(&self.registry, &entry, health, None, false).await;
        self.status(name).await
    }

    /// Reset the retry budget and reconnect an existing transport.
    pub async fn restart(&self, name: &str) -> Result<ConnectorRuntimeStatus> {
        let _lifecycle = self.lifecycle.lock().await;
        let entry = self.entry(name).await?;
        let mut recovery = RestartRecovery::new(Arc::clone(&entry), Arc::clone(&self.registry));
        stop_entry(&entry).await;
        let mut supervision = entry.supervision.write().await;
        let mut registry = self.registry.write().await;
        if !entry.active.load(Ordering::Acquire)
            || registry
                .get(name)
                .and_then(|info| info.metadata.get("generation"))
                .and_then(Value::as_u64)
                != Some(entry.generation)
        {
            return Err(RokoError::transport(
                "connector lifecycle changed while restarting",
            ));
        }
        supervision.state = ConnectorSupervisorState::Reconnecting;
        supervision.reconnect_attempts = 0;
        supervision.consecutive_failures = 0;
        supervision.restart_count = supervision.restart_count.saturating_add(1);
        supervision.last_error = None;
        let created_at = registry
            .get(name)
            .map(|info| info.created_at)
            .unwrap_or_else(Utc::now);
        registry.register(canonical_info(
            &entry,
            &disconnected_health(None),
            supervision.clone(),
            created_at,
        ));
        drop(registry);
        drop(supervision);
        let _ = entry.connector.lock().await.disconnect().await;
        let health = connect_and_probe(&entry).await;
        set_state_after_probe(&entry, &health).await;
        update_canonical(&self.registry, &entry, health, None, false).await;
        spawn_supervisor(Arc::clone(&entry), Arc::clone(&self.registry));
        recovery.disarm();
        self.status(name).await
    }

    /// Query a connected transport.
    pub async fn query(&self, name: &str, request: QueryRequest) -> Result<QueryResponse> {
        request.validate()?;
        let entry = self.connected_entry(name).await?;
        let result = entry.connector.lock().await.query(request).await;
        if let Err(error) = &result {
            mark_operation_failure(&self.registry, &entry, error).await;
        }
        result
    }

    /// Execute against a connected transport.
    pub async fn execute(&self, name: &str, request: ExecuteRequest) -> Result<ExecuteResponse> {
        request.validate()?;
        let entry = self.connected_entry(name).await?;
        let result = entry.connector.lock().await.execute(request).await;
        if let Err(error) = &result {
            mark_operation_failure(&self.registry, &entry, error).await;
        }
        result
    }

    /// Disconnect and remove one managed connector and its canonical descriptor.
    pub async fn unregister(&self, name: &str) -> Result<bool> {
        let _lifecycle = self.lifecycle.lock().await;
        let mut entries = self.entries.write().await;
        let Some(entry) = entries.get(name).cloned() else {
            return Ok(false);
        };
        let mut registry = self.registry.write().await;
        // Deactivation and removal are one cancellation-free publication step.
        entry.active.store(false, Ordering::Release);
        entries.remove(name);
        let deleted = registry
            .get(name)
            .and_then(|info| info.metadata.get("generation"))
            .and_then(Value::as_u64)
            == Some(entry.generation)
            && registry.unregister(name);
        drop(registry);
        drop(entries);
        spawn_cleanup(entry);
        Ok(deleted)
    }

    /// Stop all supervision tasks and gracefully disconnect every transport.
    pub async fn shutdown(&self) {
        let _lifecycle = self.lifecycle.lock().await;
        let mut entries_guard = self.entries.write().await;
        let mut registry = self.registry.write().await;
        // Empty the managed and canonical views atomically before any async
        // disconnect work, so cancellation cannot leave connected ghosts.
        let entries = std::mem::take(&mut *entries_guard);
        for (name, entry) in &entries {
            entry.active.store(false, Ordering::Release);
            let matches = registry
                .get(name)
                .and_then(|info| info.metadata.get("generation"))
                .and_then(Value::as_u64)
                == Some(entry.generation);
            if matches {
                registry.unregister(name);
            }
        }
        drop(registry);
        drop(entries_guard);
        for (name, entry) in entries {
            stop_entry(&entry).await;
            if let Err(error) = entry.connector.lock().await.disconnect().await {
                tracing::warn!(connector = %name, error = %safe_error(&error), "connector disconnect failed");
            }
        }
    }

    async fn entry(&self, name: &str) -> Result<Arc<ManagedConnector>> {
        self.entries
            .read()
            .await
            .get(name)
            .cloned()
            .ok_or_else(|| RokoError::user(format!("connector '{name}' not found")))
    }

    async fn connected_entry(&self, name: &str) -> Result<Arc<ManagedConnector>> {
        let entry = self.entry(name).await?;
        let connected = entry.active.load(Ordering::Acquire)
            && self.registry.read().await.get(name).is_some_and(|info| {
                info.health.status == ConnectorStatus::Connected
                    && info.metadata.get("generation").and_then(Value::as_u64)
                        == Some(entry.generation)
            });
        if !connected {
            return Err(RokoError::transport(format!(
                "connector '{name}' is not connected"
            )));
        }
        Ok(entry)
    }
}

fn validate_runtime_bounds(manifest: &ConnectorManifest) -> Result<()> {
    if manifest.name.len() > 64
        || !manifest
            .name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(RokoError::invalid(
            "connector name must be 1-64 ASCII letters, digits, '.', '-' or '_'",
        ));
    }
    if manifest.health_interval_secs > MAX_HEALTH_INTERVAL_SECS {
        return Err(RokoError::invalid(format!(
            "health_interval_secs must not exceed {MAX_HEALTH_INTERVAL_SECS}"
        )));
    }
    let delay = match manifest.reconnect_strategy {
        ReconnectStrategy::ExponentialBackoff { max_ms, .. } => max_ms,
        ReconnectStrategy::FixedInterval { interval_ms } => interval_ms,
        ReconnectStrategy::Manual => 0,
    };
    if delay > MAX_RECONNECT_DELAY_MS {
        return Err(RokoError::invalid(format!(
            "reconnect delay must not exceed {MAX_RECONNECT_DELAY_MS}ms"
        )));
    }
    Ok(())
}

async fn stop_entry(entry: &ManagedConnector) {
    let Some(handle) = entry.supervisor.lock().take() else {
        return;
    };
    let SupervisorHandle { stop, task } = handle;
    let _ = stop.send(true);
    let mut task = AbortTaskOnDrop(Some(task));
    if let Some(handle) = task.0.as_mut() {
        let _ = handle.await;
    }
    task.0 = None;
    entry.supervision.write().await.state = ConnectorSupervisorState::Stopped;
}

fn spawn_supervisor(entry: Arc<ManagedConnector>, registry: SharedConnectorRegistry) {
    let mut slot = entry.supervisor.lock();
    if slot.is_some() || !entry.active.load(Ordering::Acquire) {
        return;
    }
    let (stop, receiver) = watch::channel(false);
    let task_entry = Arc::clone(&entry);
    let task = tokio::spawn(async move {
        supervise(task_entry, registry, receiver).await;
    });
    *slot = Some(SupervisorHandle { stop, task });
}

fn spawn_cleanup(entry: Arc<ManagedConnector>) {
    tokio::spawn(async move {
        stop_entry(&entry).await;
        if let Err(error) = entry.connector.lock().await.disconnect().await {
            tracing::warn!(error = %safe_error(&error), "connector cleanup disconnect failed");
        }
    });
}

async fn supervise(
    entry: Arc<ManagedConnector>,
    registry: SharedConnectorRegistry,
    mut stop: watch::Receiver<bool>,
) {
    loop {
        if !entry.active.load(Ordering::Acquire) {
            break;
        }
        let connected = registry
            .read()
            .await
            .get(&entry.manifest.name)
            .is_some_and(|info| info.health.status == ConnectorStatus::Connected);

        if connected {
            if wait_or_stop(
                Duration::from_secs(entry.manifest.health_interval_secs),
                &mut stop,
            )
            .await
            {
                break;
            }
            if !entry.active.load(Ordering::Acquire) {
                break;
            }
            let health = probe_health(&entry).await;
            let still_connected = health.status == ConnectorStatus::Connected;
            set_state_after_probe(&entry, &health).await;
            update_canonical(&registry, &entry, health, None, false).await;
            if still_connected {
                continue;
            }
        }

        if matches!(entry.manifest.reconnect_strategy, ReconnectStrategy::Manual) {
            entry.supervision.write().await.state = ConnectorSupervisorState::Manual;
            let health = disconnected_health(None);
            update_canonical(&registry, &entry, health, None, false).await;
            let _ = stop.changed().await;
            break;
        }

        let consecutive = entry.supervision.read().await.consecutive_failures;
        if consecutive >= entry.options.max_reconnect_attempts {
            entry.supervision.write().await.state = ConnectorSupervisorState::Exhausted;
            let health = disconnected_health(None);
            update_canonical(&registry, &entry, health, None, false).await;
            break;
        }

        entry.supervision.write().await.state = ConnectorSupervisorState::Reconnecting;
        let delay = reconnect_delay(&entry.manifest.reconnect_strategy, consecutive);
        if wait_or_stop(delay, &mut stop).await {
            break;
        }
        if !entry.active.load(Ordering::Acquire) {
            break;
        }

        let _ = entry.connector.lock().await.disconnect().await;
        if !entry.active.load(Ordering::Acquire) {
            break;
        }
        let health = connect_and_probe(&entry).await;
        let succeeded = health.status == ConnectorStatus::Connected;
        {
            let mut supervision = entry.supervision.write().await;
            supervision.reconnect_attempts = supervision.reconnect_attempts.saturating_add(1);
            if succeeded {
                supervision.state = ConnectorSupervisorState::Monitoring;
                supervision.consecutive_failures = 0;
                supervision.last_error = None;
            } else {
                supervision.consecutive_failures =
                    supervision.consecutive_failures.saturating_add(1);
                supervision.health_failures = supervision.health_failures.saturating_add(1);
                supervision.last_error = health.error.clone();
            }
        }
        update_canonical(&registry, &entry, health, None, false).await;
    }
}

async fn wait_or_stop(duration: Duration, stop: &mut watch::Receiver<bool>) -> bool {
    if *stop.borrow() {
        return true;
    }
    tokio::select! {
        () = tokio::time::sleep(duration) => false,
        changed = stop.changed() => changed.is_err() || *stop.borrow(),
    }
}

fn reconnect_delay(strategy: &ReconnectStrategy, failure_index: u32) -> Duration {
    let millis = match *strategy {
        ReconnectStrategy::ExponentialBackoff {
            base_ms,
            max_ms,
            jitter,
        } => {
            let factor = 1_u64.checked_shl(failure_index.min(63)).unwrap_or(u64::MAX);
            let bounded = base_ms.saturating_mul(factor).min(max_ms);
            if jitter && bounded > 1 {
                let nanos = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .subsec_nanos() as u64;
                (bounded / 2)
                    .saturating_add(nanos % (bounded / 2 + 1))
                    .min(max_ms)
            } else {
                bounded
            }
        }
        ReconnectStrategy::FixedInterval { interval_ms } => interval_ms,
        ReconnectStrategy::Manual => 0,
    };
    Duration::from_millis(millis)
}

async fn connect_and_probe(entry: &ManagedConnector) -> ConnectHealthStatus {
    let mut connector = entry.connector.lock().await;
    if let Err(error) = connector.connect(&entry.config).await {
        return disconnected_health(Some(safe_error(&error)));
    }
    match connector.health().await {
        Ok(health) => health,
        Err(error) => disconnected_health(Some(safe_error(&error))),
    }
}

async fn probe_health(entry: &ManagedConnector) -> ConnectHealthStatus {
    match entry.connector.lock().await.health().await {
        Ok(health) => health,
        Err(error) => disconnected_health(Some(safe_error(&error))),
    }
}

async fn set_state_after_probe(entry: &ManagedConnector, health: &ConnectHealthStatus) {
    let mut supervision = entry.supervision.write().await;
    if health.status == ConnectorStatus::Connected {
        supervision.state = ConnectorSupervisorState::Monitoring;
        supervision.consecutive_failures = 0;
        supervision.last_error = None;
        return;
    }
    supervision.health_failures = supervision.health_failures.saturating_add(1);
    supervision.state = if matches!(entry.manifest.reconnect_strategy, ReconnectStrategy::Manual) {
        ConnectorSupervisorState::Manual
    } else {
        ConnectorSupervisorState::Reconnecting
    };
    supervision.last_error = health
        .error
        .clone()
        .or_else(|| Some("transport health check failed".to_owned()));
}

async fn update_canonical(
    registry: &SharedConnectorRegistry,
    entry: &ManagedConnector,
    health: ConnectHealthStatus,
    created_at: Option<chrono::DateTime<Utc>>,
    force: bool,
) {
    let supervision = entry.supervision.read().await.clone();
    let mut registry = registry.write().await;
    if !force
        && (!entry.active.load(Ordering::Acquire)
            || registry
                .get(&entry.manifest.name)
                .and_then(|info| info.metadata.get("generation"))
                .and_then(Value::as_u64)
                != Some(entry.generation))
    {
        return;
    }
    let created_at = created_at
        .or_else(|| {
            registry
                .get(&entry.manifest.name)
                .map(|info| info.created_at)
        })
        .unwrap_or_else(Utc::now);
    registry.register(canonical_info(entry, &health, supervision, created_at));
}

fn canonical_info(
    entry: &ManagedConnector,
    health: &ConnectHealthStatus,
    supervision: ConnectorSupervisionStatus,
    created_at: chrono::DateTime<Utc>,
) -> ConnectorInfo {
    ConnectorInfo {
        name: entry.manifest.name.clone(),
        kind: entry.manifest.kind.clone(),
        health: ConnectorHealth {
            status: health.status.clone(),
            latency_ms: health.latency_ms,
            last_check: health.last_check,
        },
        created_at,
        metadata: json!({
            "generation": entry.generation,
            "transport": entry.transport,
            "supervisor": supervision,
        }),
    }
}

async fn mark_operation_failure(
    registry: &SharedConnectorRegistry,
    entry: &ManagedConnector,
    error: &RokoError,
) {
    let message = safe_error(error);
    entry.supervision.write().await.last_error = Some(message.clone());
    update_canonical(
        registry,
        entry,
        ConnectHealthStatus {
            status: ConnectorStatus::Degraded,
            latency_ms: 0,
            last_check: Utc::now(),
            error: Some(message),
        },
        None,
        false,
    )
    .await;
}

fn disconnected_health(error: Option<String>) -> ConnectHealthStatus {
    ConnectHealthStatus {
        status: ConnectorStatus::Disconnected,
        latency_ms: 0,
        last_check: Utc::now(),
        error,
    }
}

fn safe_error(error: &RokoError) -> String {
    match error {
        RokoError::Timeout { .. } => "transport timeout".to_owned(),
        RokoError::Invalid(_) | RokoError::User(_) => "connector rejected request".to_owned(),
        RokoError::RateLimited(_) => "transport rate limited".to_owned(),
        RokoError::PermissionDenied(_) => "transport authorization failed".to_owned(),
        _ => "transport unavailable".to_owned(),
    }
}

/// Generic HTTP transport: GET for queries, POST JSON for executions, and a
/// real GET of the configured base endpoint for connect/health.
pub struct HttpJsonConnector {
    client: Option<reqwest::Client>,
    endpoint: Option<reqwest::Url>,
}

impl HttpJsonConnector {
    /// Create a disconnected HTTP JSON transport.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            client: None,
            endpoint: None,
        }
    }

    fn validate_config(config: &ConnectConfig) -> Result<reqwest::Url> {
        config.validate()?;
        if config.endpoint.len() > MAX_ENDPOINT_BYTES {
            return Err(RokoError::invalid(format!(
                "connector endpoint must not exceed {MAX_ENDPOINT_BYTES} bytes"
            )));
        }
        if config.timeout_ms > MAX_HTTP_TIMEOUT_MS {
            return Err(RokoError::invalid(format!(
                "connector timeout_ms must not exceed {MAX_HTTP_TIMEOUT_MS}"
            )));
        }
        let endpoint = reqwest::Url::parse(&config.endpoint)
            .map_err(|_| RokoError::invalid("connector endpoint must be a valid HTTP URL"))?;
        if !matches!(endpoint.scheme(), "http" | "https") {
            return Err(RokoError::invalid(
                "http_json endpoint scheme must be http or https",
            ));
        }
        if !endpoint.username().is_empty() || endpoint.password().is_some() {
            return Err(RokoError::invalid(
                "connector credentials must use auth or headers, not URL userinfo",
            ));
        }
        if endpoint.query().is_some() || endpoint.fragment().is_some() {
            return Err(RokoError::invalid(
                "connector base endpoint must not contain query or fragment data",
            ));
        }
        if config
            .headers
            .as_ref()
            .is_some_and(|headers| headers.len() > 32)
        {
            return Err(RokoError::invalid(
                "connector headers must contain at most 32 entries",
            ));
        }
        if config.auth.as_ref().is_some_and(|auth| auth.len() > 8_192) {
            return Err(RokoError::invalid(
                "connector auth must not exceed 8192 bytes",
            ));
        }
        let header_bytes = config
            .headers
            .as_ref()
            .map(|headers| {
                headers.iter().fold(0_usize, |total, (name, value)| {
                    total.saturating_add(name.len()).saturating_add(value.len())
                })
            })
            .unwrap_or_default();
        if header_bytes > MAX_HEADER_BYTES {
            return Err(RokoError::invalid(format!(
                "connector headers must not exceed {MAX_HEADER_BYTES} total bytes"
            )));
        }
        Ok(endpoint)
    }

    fn operation_url(&self, operation: &str) -> Result<reqwest::Url> {
        if operation.len() > MAX_OPERATION_BYTES {
            return Err(RokoError::invalid(format!(
                "connector operation must not exceed {MAX_OPERATION_BYTES} bytes"
            )));
        }
        if operation.contains("://") || operation.split('/').any(|part| part == "..") {
            return Err(RokoError::invalid(
                "connector operation must be a relative path without '..'",
            ));
        }
        let base = self
            .endpoint
            .as_ref()
            .ok_or_else(|| RokoError::transport("connector is disconnected"))?;
        let joined = base
            .join(operation.trim_start_matches('/'))
            .map_err(|_| RokoError::invalid("connector operation is not a valid relative path"))?;
        if joined.scheme() != base.scheme()
            || joined.host_str() != base.host_str()
            || joined.port_or_known_default() != base.port_or_known_default()
        {
            return Err(RokoError::invalid(
                "connector operation must remain on the configured origin",
            ));
        }
        Ok(joined)
    }

    async fn probe(&self) -> Result<ConnectHealthStatus> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| RokoError::transport("connector is disconnected"))?;
        let endpoint = self
            .endpoint
            .as_ref()
            .ok_or_else(|| RokoError::transport("connector is disconnected"))?;
        let started = Instant::now();
        let response = client
            .get(endpoint.clone())
            .send()
            .await
            .map_err(|_| RokoError::transport("HTTP health request failed"))?;
        let latency_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        if !response.status().is_success() {
            return Err(RokoError::transport(format!(
                "HTTP health returned status {}",
                response.status().as_u16()
            )));
        }
        Ok(ConnectHealthStatus {
            status: ConnectorStatus::Connected,
            latency_ms,
            last_check: Utc::now(),
            error: None,
        })
    }

    async fn decode_json(mut response: reqwest::Response, operation: &str) -> Result<Value> {
        if response
            .content_length()
            .is_some_and(|length| length > MAX_HTTP_JSON_BYTES as u64)
        {
            return Err(RokoError::transport(format!(
                "HTTP {operation} response exceeded the byte limit"
            )));
        }
        let mut body = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| RokoError::transport(format!("HTTP {operation} body read failed")))?
        {
            if body.len().saturating_add(chunk.len()) > MAX_HTTP_JSON_BYTES {
                return Err(RokoError::transport(format!(
                    "HTTP {operation} response exceeded the byte limit"
                )));
            }
            body.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&body)
            .map_err(|_| RokoError::transport(format!("HTTP {operation} returned invalid JSON")))
    }
}

impl Default for HttpJsonConnector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Connect for HttpJsonConnector {
    async fn connect(&mut self, config: &ConnectConfig) -> Result<()> {
        let endpoint = Self::validate_config(config)?;
        let mut headers = HeaderMap::new();
        if let Some(configured) = &config.headers {
            for (name, value) in configured {
                let name = HeaderName::from_bytes(name.as_bytes())
                    .map_err(|_| RokoError::invalid("connector header name is invalid"))?;
                let value = HeaderValue::from_str(value)
                    .map_err(|_| RokoError::invalid("connector header value is invalid"))?;
                headers.insert(name, value);
            }
        }
        if let Some(auth) = &config.auth {
            let value = HeaderValue::from_str(&format!("Bearer {auth}"))
                .map_err(|_| RokoError::invalid("connector auth value is invalid"))?;
            headers.insert(AUTHORIZATION, value);
        }
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_millis(config.timeout_ms))
            .build()
            .map_err(|_| RokoError::invalid("connector HTTP client configuration is invalid"))?;
        self.client = Some(client);
        self.endpoint = Some(endpoint);
        Ok(())
    }

    async fn query(&self, request: QueryRequest) -> Result<QueryResponse> {
        request.validate()?;
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| RokoError::transport("connector is disconnected"))?;
        let mut url = self.operation_url(&request.operation)?;
        match request.params {
            Value::Null => {}
            Value::Object(params) => {
                if params.len() > 64 {
                    return Err(RokoError::invalid(
                        "connector query params must contain at most 64 entries",
                    ));
                }
                let mut pairs = url.query_pairs_mut();
                let mut total_bytes = 0_usize;
                for (name, value) in params {
                    let value = match value {
                        Value::String(value) => value,
                        Value::Null => continue,
                        value => value.to_string(),
                    };
                    if name.len().saturating_add(value.len()) > 8_192 {
                        return Err(RokoError::invalid(
                            "connector query parameter exceeds the byte limit",
                        ));
                    }
                    total_bytes = total_bytes
                        .saturating_add(name.len())
                        .saturating_add(value.len());
                    if total_bytes > MAX_QUERY_BYTES {
                        return Err(RokoError::invalid(format!(
                            "connector query params must not exceed {MAX_QUERY_BYTES} total bytes"
                        )));
                    }
                    pairs.append_pair(&name, &value);
                }
            }
            _ => {
                return Err(RokoError::invalid(
                    "connector query params must be a JSON object or null",
                ));
            }
        }
        let started = Instant::now();
        let response = client
            .get(url)
            .send()
            .await
            .map_err(|_| RokoError::transport("HTTP query failed"))?;
        if !response.status().is_success() {
            return Err(RokoError::transport(format!(
                "HTTP query returned status {}",
                response.status().as_u16()
            )));
        }
        let data = Self::decode_json(response, "query").await?;
        Ok(QueryResponse {
            data,
            latency_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        })
    }

    async fn execute(&self, request: ExecuteRequest) -> Result<ExecuteResponse> {
        request.validate()?;
        if serde_json::to_vec(&request.params)?.len() > MAX_HTTP_JSON_BYTES {
            return Err(RokoError::invalid(format!(
                "connector execute params must not exceed {MAX_HTTP_JSON_BYTES} bytes"
            )));
        }
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| RokoError::transport("connector is disconnected"))?;
        let url = self.operation_url(&request.operation)?;
        let started = Instant::now();
        let response = client
            .post(url)
            .json(&request.params)
            .send()
            .await
            .map_err(|_| RokoError::transport("HTTP execute failed"))?;
        if !response.status().is_success() {
            return Err(RokoError::transport(format!(
                "HTTP execute returned status {}",
                response.status().as_u16()
            )));
        }
        let result = Self::decode_json(response, "execute").await?;
        Ok(ExecuteResponse {
            result,
            latency_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        })
    }

    async fn health(&self) -> Result<ConnectHealthStatus> {
        self.probe().await
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.client = None;
        self.endpoint = None;
        Ok(())
    }
}
