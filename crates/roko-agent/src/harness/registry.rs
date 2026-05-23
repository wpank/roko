//! Central registry for harness services (and optionally adapters).
//!
//! The registry's primary job is **service lifecycle**: it maps
//! `harness_id` to gateway service instances so they can be
//! started/stopped/probed collectively.
//!
//! It also supports an adapter map keyed by `(harness_id,
//! TransportFlavor)` for diagnostics/probing, but agent creation
//! for dispatch is handled by the provider adapters
//! (`HermesProviderAdapter`, `OpenClawProviderAdapter`), not here.
//!
//! A TTL-bounded probe cache prevents repeated `roko doctor` or
//! diagnostic checks from hammering harness binaries.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant};

use roko_core::config::schema::RokoConfig;

use super::capability::TransportFlavor;
use super::error::HarnessError;
use super::service::HarnessService;
use super::{HarnessAdapter, ProbeError};

// ---- Registry config -------------------------------------------------------

/// Configuration for the harness registry.
#[derive(Debug, Clone)]
pub struct RegistryConfig {
    /// How long probe results are cached before re-running.
    pub probe_ttl: Duration,
    /// Whether to automatically probe adapters when they are registered.
    pub auto_probe_on_register: bool,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            probe_ttl: Duration::from_secs(60),
            auto_probe_on_register: false,
        }
    }
}

// ---- Registry --------------------------------------------------------------

/// Central lookup table for harness adapters and services.
///
/// The registry's primary responsibility is **service lifecycle**:
/// registering gateway services so they can be started/stopped/probed
/// collectively (e.g. via [`start_services`](Self::start_services) or
/// [`probe_all`](Self::probe_all)).
///
/// The adapter map is available for ad-hoc queries (doctor, diagnostics)
/// but is **not** the dispatch path -- agent creation for dispatch is
/// handled by the provider adapters (`HermesProviderAdapter`,
/// `OpenClawProviderAdapter`), which own the tier-selection logic.
///
/// Services are keyed by `harness_id` alone because a harness runs at
/// most one daemon regardless of how many transports it exposes.
pub struct HarnessRegistry {
    /// Registered adapters, keyed by `(harness_id, transport)`.
    adapters: HashMap<(String, TransportFlavor), Arc<dyn HarnessAdapter>>,
    /// Registered services, keyed by `harness_id`.
    services: HashMap<String, Arc<dyn HarnessService>>,
    /// TTL-bounded probe cache, keyed by `"harness_id/transport"`.
    probes: TtlCache<String, ()>,
    /// Registry configuration.
    config: RegistryConfig,
}

impl HarnessRegistry {
    /// Construct an empty registry with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(RegistryConfig::default())
    }

    /// Construct an empty registry with the given configuration.
    #[must_use]
    pub fn with_config(config: RegistryConfig) -> Self {
        let ttl = config.probe_ttl;
        Self {
            adapters: HashMap::new(),
            services: HashMap::new(),
            probes: TtlCache::new(ttl),
            config,
        }
    }

    /// Construct the registry from a parsed roko config.
    ///
    /// Reads `[providers.*]` blocks, identifies harness-type providers
    /// (`Hermes`, `OpenClaw`), and registers their **gateway services**
    /// for lifecycle management (start/stop/probe).
    ///
    /// Note: agent/adapter *instances* are NOT created here -- that is
    /// the responsibility of the provider adapters (`HermesProviderAdapter`,
    /// `OpenClawProviderAdapter`) which handle tier selection at dispatch
    /// time. This avoids duplicating the tier-selection logic.
    pub fn from_config(cfg: &RokoConfig) -> Result<Self, RegistryError> {
        let mut registry = Self::new();

        for (id, provider) in &cfg.providers {
            match provider.kind {
                roko_core::agent::ProviderKind::Hermes => {
                    tracing::info!(
                        provider_id = %id,
                        command = ?provider.command,
                        base_url = ?provider.base_url,
                        "discovered harness provider: hermes"
                    );
                    Self::register_hermes_services(&mut registry, provider);
                }
                roko_core::agent::ProviderKind::OpenClaw => {
                    tracing::info!(
                        provider_id = %id,
                        command = ?provider.command,
                        "discovered harness provider: openclaw"
                    );
                    Self::register_openclaw_services(&mut registry, provider);
                }
                _ => {
                    // Not a harness provider -- skip.
                }
            }
        }

        Ok(registry)
    }

    /// Register Hermes gateway service for lifecycle management.
    ///
    /// Only registers the [`HermesGatewayService`] when `base_url` is
    /// configured (indicating an HTTP gateway is expected). Agent
    /// instances are created by [`HermesProviderAdapter`] at dispatch
    /// time, not here.
    fn register_hermes_services(
        registry: &mut Self,
        provider: &roko_core::config::schema::ProviderConfig,
    ) {
        use crate::hermes::{HermesConfig, HermesGatewayService};

        // Only register the gateway service when an HTTP endpoint is configured.
        if provider.base_url.is_some() {
            let mut config = HermesConfig::from_provider_config(provider);
            let timeout = std::time::Duration::from_millis(provider.timeout_ms.unwrap_or(90_000));
            config.timeout = timeout;
            let svc = HermesGatewayService::new(config);
            if let Err(e) = registry.register_service(Arc::new(svc)) {
                tracing::warn!(error = %e, "failed to register hermes gateway service");
            }
        }
    }

    /// Register OpenClaw gateway service for lifecycle management.
    ///
    /// Agent instances are created by [`OpenClawProviderAdapter`] at
    /// dispatch time, not here.
    fn register_openclaw_services(
        registry: &mut Self,
        provider: &roko_core::config::schema::ProviderConfig,
    ) {
        use crate::openclaw::OpenClawGatewayService;

        let binary = provider
            .command
            .as_deref()
            .unwrap_or("openclaw")
            .to_string();

        let svc = OpenClawGatewayService::new(binary);
        if let Err(e) = registry.register_service(Arc::new(svc)) {
            tracing::warn!(error = %e, "failed to register openclaw gateway service");
        }
    }

    /// Register a new adapter.
    pub fn register(&mut self, adapter: Arc<dyn HarnessAdapter>) -> Result<(), RegistryError> {
        let key = (adapter.harness_id().to_string(), adapter.transport());
        if self.adapters.contains_key(&key) {
            return Err(RegistryError::DuplicateAdapter(format!(
                "{}/{}",
                key.0, key.1
            )));
        }
        self.adapters.insert(key, adapter);
        Ok(())
    }

    /// Register a service.
    pub fn register_service(
        &mut self,
        service: Arc<dyn HarnessService>,
    ) -> Result<(), RegistryError> {
        let name = service.service_name().to_string();
        if self.services.contains_key(&name) {
            return Err(RegistryError::DuplicateService(format!(
                "service '{name}' already registered"
            )));
        }
        self.services.insert(name, service);
        Ok(())
    }

    /// Look up an adapter by `(harness_id, transport)`.
    ///
    /// Used for diagnostics and probing -- NOT for dispatch. Agent
    /// creation for dispatch goes through the provider adapters.
    pub fn adapter(
        &self,
        id: &str,
        transport: TransportFlavor,
    ) -> Result<Arc<dyn HarnessAdapter>, RegistryError> {
        let key = (id.to_string(), transport);
        self.adapters
            .get(&key)
            .cloned()
            .ok_or_else(|| RegistryError::AdapterNotFound {
                id: id.to_string(),
                transport,
            })
    }

    /// Return all registered adapters (for diagnostics, not dispatch).
    pub fn all_adapters(&self) -> Vec<Arc<dyn HarnessAdapter>> {
        self.adapters.values().cloned().collect()
    }

    /// Return all adapters for a given provider/harness id (for diagnostics, not dispatch).
    pub fn adapters_for_provider(&self, harness_id: &str) -> Vec<Arc<dyn HarnessAdapter>> {
        self.adapters
            .iter()
            .filter(|((id, _), _)| id == harness_id)
            .map(|(_, adapter)| adapter.clone())
            .collect()
    }

    /// Look up a service by `harness_id`.
    pub fn service(&self, id: &str) -> Option<Arc<dyn HarnessService>> {
        self.services.get(id).cloned()
    }

    /// Number of registered adapters.
    #[must_use]
    pub fn adapter_count(&self) -> usize {
        self.adapters.len()
    }

    /// Number of registered services.
    #[must_use]
    pub fn service_count(&self) -> usize {
        self.services.len()
    }

    /// List all registered adapter keys.
    #[must_use]
    pub fn adapter_keys(&self) -> Vec<(String, TransportFlavor)> {
        self.adapters.keys().cloned().collect()
    }

    /// Invalidate all cached probe results.
    pub fn invalidate_cache(&mut self) {
        self.probes.clear();
    }

    /// Run all probes; return one report per registered adapter.
    pub async fn probe_all(&mut self) -> Vec<(String, TransportFlavor, Result<(), ProbeError>)> {
        let mut results = Vec::new();
        for ((id, transport), adapter) in &self.adapters {
            let cache_key = format!("{id}/{transport}");
            if self.probes.get(&cache_key).is_some() {
                results.push((id.clone(), *transport, Ok(())));
                continue;
            }
            let probe_result = adapter.probe().await;
            if probe_result.is_ok() {
                self.probes.insert(cache_key, ());
            }
            results.push((id.clone(), *transport, probe_result));
        }
        results
    }

    /// Start every registered service.
    pub async fn start_services(&self) -> Result<(), RegistryError> {
        for (name, service) in &self.services {
            match service.start().await {
                Ok(()) => {
                    tracing::info!(service_name = %name, "harness service started");
                }
                Err(e) => {
                    tracing::warn!(
                        service_name = %name,
                        error = %e,
                        "harness service start failed (non-fatal)"
                    );
                }
            }
        }
        Ok(())
    }
}

impl Default for HarnessRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---- Registry errors -------------------------------------------------------

/// Errors from registry operations.
#[derive(Debug)]
pub enum RegistryError {
    /// No adapter registered for the given `(harness_id, transport)`.
    AdapterNotFound {
        id: String,
        transport: TransportFlavor,
    },
    /// An adapter with the same key is already registered.
    DuplicateAdapter(String),
    /// A service with the same id is already registered.
    DuplicateService(String),
    /// Service-level error.
    Service(HarnessError),
    /// Configuration error.
    Config(String),
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AdapterNotFound { id, transport } => {
                write!(f, "no adapter for harness '{id}' transport {transport}")
            }
            Self::DuplicateAdapter(msg) => write!(f, "duplicate adapter registration: {msg}"),
            Self::DuplicateService(msg) => write!(f, "duplicate service registration: {msg}"),
            Self::Service(e) => write!(f, "service: {e}"),
            Self::Config(msg) => write!(f, "config: {msg}"),
        }
    }
}

impl std::error::Error for RegistryError {}

impl From<HarnessError> for RegistryError {
    fn from(err: HarnessError) -> Self {
        Self::Service(err)
    }
}

// ---- TTL cache -------------------------------------------------------------

/// Simple TTL-bounded cache backed by a `HashMap`.
pub struct TtlCache<K, V> {
    entries: HashMap<K, (V, Instant)>,
    ttl: Duration,
}

impl<K: std::hash::Hash + Eq, V: Clone> TtlCache<K, V> {
    /// Create an empty cache with the given TTL.
    #[must_use]
    pub fn new(ttl: Duration) -> Self {
        Self {
            entries: HashMap::new(),
            ttl,
        }
    }

    /// Get a cached value if it exists and has not expired.
    #[must_use]
    pub fn get(&self, key: &K) -> Option<&V> {
        self.entries.get(key).and_then(|(value, inserted_at)| {
            if inserted_at.elapsed() < self.ttl {
                Some(value)
            } else {
                None
            }
        })
    }

    /// Insert or update a value.
    pub fn insert(&mut self, key: K, value: V) {
        self.entries.insert(key, (value, Instant::now()));
    }

    /// Remove all entries (expired or not).
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Remove expired entries.
    pub fn evict_expired(&mut self) {
        self.entries
            .retain(|_, (_, inserted_at)| inserted_at.elapsed() < self.ttl);
    }

    /// Number of entries (including possibly expired ones).
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the cache is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::{HarnessAdapter, HarnessCapabilities, ProbeError};
    use async_trait::async_trait;
    use std::sync::Arc;

    // ---- MockAdapter --------------------------------------------------------

    struct MockAdapter {
        id: &'static str,
        flavor: TransportFlavor,
        caps: HarnessCapabilities,
    }

    impl MockAdapter {
        fn new(id: &'static str, flavor: TransportFlavor) -> Arc<Self> {
            Arc::new(Self {
                id,
                flavor,
                caps: HarnessCapabilities::default(),
            })
        }
    }

    #[async_trait]
    impl crate::agent::Agent for MockAdapter {
        fn name(&self) -> &str {
            self.id
        }

        async fn run(
            &self,
            _input: &roko_core::Signal,
            _ctx: &roko_core::Context,
        ) -> crate::agent::AgentResult {
            unimplemented!("MockAdapter::run is never called in registry tests")
        }
    }

    #[async_trait]
    impl HarnessAdapter for MockAdapter {
        fn harness_id(&self) -> &str {
            self.id
        }

        fn transport(&self) -> TransportFlavor {
            self.flavor
        }

        fn capabilities(&self) -> &HarnessCapabilities {
            &self.caps
        }

        async fn probe(&self) -> Result<(), ProbeError> {
            Ok(())
        }
    }

    // ---- TtlCache tests -----------------------------------------------------

    #[test]
    fn ttl_cache_get_returns_none_for_missing_key() {
        let cache: TtlCache<String, String> = TtlCache::new(Duration::from_secs(60));
        assert!(cache.get(&"missing".to_string()).is_none());
    }

    #[test]
    fn ttl_cache_insert_and_get() {
        let mut cache = TtlCache::new(Duration::from_secs(60));
        cache.insert("key".to_string(), "value".to_string());
        assert_eq!(cache.get(&"key".to_string()), Some(&"value".to_string()));
    }

    #[test]
    fn ttl_cache_evict_removes_expired() {
        let mut cache = TtlCache::new(Duration::from_millis(0));
        cache.insert("key".to_string(), "value".to_string());
        std::thread::sleep(Duration::from_millis(1));
        cache.evict_expired();
        assert!(cache.is_empty());
    }

    #[test]
    fn ttl_cache_clear_removes_all() {
        let mut cache = TtlCache::new(Duration::from_secs(60));
        cache.insert("a".to_string(), "1".to_string());
        cache.insert("b".to_string(), "2".to_string());
        assert_eq!(cache.len(), 2);
        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn registry_empty_has_zero_counts() {
        let registry = HarnessRegistry::new();
        assert_eq!(registry.adapter_count(), 0);
        assert_eq!(registry.service_count(), 0);
        assert!(
            registry
                .adapter("hermes", TransportFlavor::HttpOpenAi)
                .is_err()
        );
    }

    #[test]
    fn registry_from_config_empty_config_succeeds() {
        let config = RokoConfig::default();
        let registry = HarnessRegistry::from_config(&config).expect("from_config");
        assert_eq!(registry.adapter_count(), 0);
    }

    #[test]
    fn registry_all_adapters_empty() {
        let registry = HarnessRegistry::new();
        assert!(registry.all_adapters().is_empty());
    }

    #[test]
    fn registry_adapters_for_provider_empty() {
        let registry = HarnessRegistry::new();
        assert!(registry.adapters_for_provider("hermes").is_empty());
    }

    #[test]
    fn registry_invalidate_cache() {
        let mut registry = HarnessRegistry::new();
        registry.probes.insert("test/key".to_string(), ());
        assert!(!registry.probes.is_empty());
        registry.invalidate_cache();
        assert!(registry.probes.is_empty());
    }

    #[test]
    fn registry_with_config() {
        let config = RegistryConfig {
            probe_ttl: Duration::from_secs(120),
            auto_probe_on_register: true,
        };
        let registry = HarnessRegistry::with_config(config);
        assert_eq!(registry.config.probe_ttl, Duration::from_secs(120));
        assert!(registry.config.auto_probe_on_register);
    }

    // ---- register() + adapter() lookup --------------------------------------

    #[test]
    fn register_and_adapter_lookup_succeeds() {
        let mut registry = HarnessRegistry::new();
        let adapter = MockAdapter::new("hermes", TransportFlavor::HttpOpenAi);
        registry.register(adapter).expect("register");

        let found = registry
            .adapter("hermes", TransportFlavor::HttpOpenAi)
            .expect("lookup");
        assert_eq!(found.harness_id(), "hermes");
        assert_eq!(found.transport(), TransportFlavor::HttpOpenAi);
    }

    #[test]
    fn adapter_lookup_wrong_transport_returns_not_found() {
        let mut registry = HarnessRegistry::new();
        let adapter = MockAdapter::new("hermes", TransportFlavor::HttpOpenAi);
        registry.register(adapter).expect("register");

        let result = registry.adapter("hermes", TransportFlavor::OneShotJson);
        assert!(
            matches!(result, Err(RegistryError::AdapterNotFound { .. })),
            "expected AdapterNotFound"
        );
    }

    // ---- duplicate registration returns DuplicateAdapter --------------------

    #[test]
    fn register_duplicate_key_returns_duplicate_adapter_error() {
        let mut registry = HarnessRegistry::new();
        let a1 = MockAdapter::new("hermes", TransportFlavor::HttpOpenAi);
        let a2 = MockAdapter::new("hermes", TransportFlavor::HttpOpenAi);

        registry.register(a1).expect("first register");
        let err = registry.register(a2).unwrap_err();

        assert!(
            matches!(err, RegistryError::DuplicateAdapter(ref msg) if msg.contains("hermes")),
            "expected DuplicateAdapter containing 'hermes', got: {err}"
        );
    }

    // ---- all_adapters() -----------------------------------------------------

    #[test]
    fn all_adapters_returns_all_registered() {
        let mut registry = HarnessRegistry::new();
        let a1 = MockAdapter::new("hermes", TransportFlavor::HttpOpenAi);
        let a2 = MockAdapter::new("openclaw", TransportFlavor::OneShotJson);
        registry.register(a1).expect("register a1");
        registry.register(a2).expect("register a2");

        let all = registry.all_adapters();
        assert_eq!(all.len(), 2);
        let mut ids: Vec<&str> = all.iter().map(|a| a.harness_id()).collect();
        ids.sort_unstable();
        assert_eq!(ids, ["hermes", "openclaw"]);
    }

    // ---- adapters_for_provider() --------------------------------------------

    #[test]
    fn adapters_for_provider_returns_correct_subset() {
        let mut registry = HarnessRegistry::new();
        // Register two transports for "hermes" and one for "openclaw".
        registry
            .register(MockAdapter::new("hermes", TransportFlavor::HttpOpenAi))
            .expect("register hermes/http");
        registry
            .register(MockAdapter::new("hermes", TransportFlavor::OneShotJson))
            .expect("register hermes/oneshot");
        registry
            .register(MockAdapter::new("openclaw", TransportFlavor::OneShotPlain))
            .expect("register openclaw");

        let hermes_adapters = registry.adapters_for_provider("hermes");
        assert_eq!(hermes_adapters.len(), 2);
        assert!(hermes_adapters.iter().all(|a| a.harness_id() == "hermes"));

        let openclaw_adapters = registry.adapters_for_provider("openclaw");
        assert_eq!(openclaw_adapters.len(), 1);
        assert_eq!(openclaw_adapters[0].harness_id(), "openclaw");

        let missing = registry.adapters_for_provider("unknown");
        assert!(missing.is_empty());
    }

    // ---- adapter_count() increments after register --------------------------

    #[test]
    fn adapter_count_increments_after_register() {
        let mut registry = HarnessRegistry::new();
        assert_eq!(registry.adapter_count(), 0);

        registry
            .register(MockAdapter::new("hermes", TransportFlavor::HttpOpenAi))
            .expect("register 1");
        assert_eq!(registry.adapter_count(), 1);

        registry
            .register(MockAdapter::new("openclaw", TransportFlavor::OneShotJson))
            .expect("register 2");
        assert_eq!(registry.adapter_count(), 2);
    }

    // ---- adapter_keys() reflects registered keys ---------------------------

    #[test]
    fn adapter_keys_matches_registered_entries() {
        let mut registry = HarnessRegistry::new();
        registry
            .register(MockAdapter::new("hermes", TransportFlavor::HttpOpenAi))
            .expect("register");

        let keys = registry.adapter_keys();
        assert_eq!(keys.len(), 1);
        assert!(keys.contains(&("hermes".to_string(), TransportFlavor::HttpOpenAi)));
    }
}
