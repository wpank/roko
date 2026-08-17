//! JWKS caching and JWT verification for external identity providers.
//!
//! Keys are refreshed proactively and on an unknown `kid`. Refreshes are
//! coalesced so a rotated key cannot cause a request-driven refresh storm.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use futures::future::join_all;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use serde::{Deserialize, Serialize};
use tokio::sync::{Notify, RwLock};
use tokio::task::JoinHandle;

pub use roko_core::config::JwksProvider;

/// The Nunchi Privy application ID. Project-level constant, not a secret.
pub const NUNCHI_PRIVY_APP_ID: &str = "cmhw01vut003tjx0d5lmqc8zs";

/// Default JWKS endpoint for Privy.
pub const PRIVY_JWKS_URL: &str = "https://auth.privy.io/.well-known/jwks.json";

/// Cache TTL: keys are refreshed after this duration.
const CACHE_TTL: Duration = Duration::from_secs(60 * 60);

/// Staleness threshold at which validation emits a warning.
const MAX_STALE: Duration = Duration::from_secs(24 * 60 * 60);

/// Absolute staleness limit. Authentication fails closed beyond this age.
const FAIL_CLOSED_STALE: Duration = Duration::from_secs(48 * 60 * 60);

/// Default HTTP request timeout for JWKS fetches when none is configured.
const DEFAULT_JWKS_FETCH_TIMEOUT: Duration = Duration::from_secs(10);

fn privy_provider() -> JwksProvider {
    JwksProvider::new(PRIVY_JWKS_URL, "privy.io")
}

/// Operator-facing cache health summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct CacheHealth {
    /// Whether every configured provider has keys younger than [`CACHE_TTL`].
    pub fresh: bool,
    /// Total cached signing-key count across providers.
    pub key_count: usize,
    /// Age of the oldest provider key set, in seconds.
    pub age_secs: Option<u64>,
    /// Whether the oldest keys exceed the 24-hour warning threshold.
    pub stale: bool,
    /// Whether authentication is currently fail-closed due to no or >48h-old keys.
    pub fail_closed: bool,
}

/// Decoded claims from a Privy JWT.
#[derive(Debug, Clone, Deserialize)]
pub struct PrivyClaims {
    /// Privy user identifier (e.g. `did:privy:...`).
    pub sub: String,
    /// Token issuer.
    #[serde(default)]
    pub iss: String,
    /// Audience claim.
    #[serde(default)]
    pub aud: Option<serde_json::Value>,
    /// Workspace / organisation membership claim.
    #[serde(default)]
    pub org_id: Option<String>,
    /// Role within the workspace (e.g. `"admin"`, `"member"`, `"viewer"`).
    #[serde(default)]
    pub role: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct Jwk {
    kid: String,
    kty: String,
    crv: Option<String>,
    x: Option<String>,
    y: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct JwksResponse {
    keys: Vec<Jwk>,
}

struct CachedProviderKeys {
    provider: JwksProvider,
    keys: Vec<Jwk>,
    fetched_at: Instant,
}

enum CacheValidation {
    Valid(PrivyClaims),
    UnknownKid,
    Invalid,
}

/// Thread-safe multi-provider JWKS cache with automatic refresh.
pub struct JwksCache {
    http: reqwest::Client,
    providers: Vec<JwksProvider>,
    cache: RwLock<Vec<CachedProviderKeys>>,
    fetch_timeout: Duration,
    refreshing: AtomicBool,
    refresh_notify: Notify,
}

struct RefreshGuard<'a> {
    refreshing: &'a AtomicBool,
    notify: &'a Notify,
}

impl Drop for RefreshGuard<'_> {
    fn drop(&mut self) {
        self.refreshing.store(false, Ordering::Release);
        self.notify.notify_waiters();
    }
}

impl JwksCache {
    /// Create a cache for the default Privy provider.
    pub fn new(http: reqwest::Client) -> Self {
        Self::with_providers_and_timeout(http, vec![privy_provider()], DEFAULT_JWKS_FETCH_TIMEOUT)
    }

    /// Create a default-provider cache with a configurable fetch timeout.
    pub fn with_timeout(http: reqwest::Client, fetch_timeout: Duration) -> Self {
        Self::with_providers_and_timeout(http, vec![privy_provider()], fetch_timeout)
    }

    /// Create a cache for one or more issuer-bound JWKS providers.
    pub fn with_providers(http: reqwest::Client, providers: Vec<JwksProvider>) -> Self {
        Self::with_providers_and_timeout(http, providers, DEFAULT_JWKS_FETCH_TIMEOUT)
    }

    /// Create a multi-provider cache with a configurable fetch timeout.
    pub fn with_providers_and_timeout(
        http: reqwest::Client,
        providers: Vec<JwksProvider>,
        fetch_timeout: Duration,
    ) -> Self {
        Self {
            http,
            providers,
            cache: RwLock::new(Vec::new()),
            fetch_timeout: if fetch_timeout.is_zero() {
                DEFAULT_JWKS_FETCH_TIMEOUT
            } else {
                fetch_timeout
            },
            refreshing: AtomicBool::new(false),
            refresh_notify: Notify::new(),
        }
    }

    /// Eagerly fetch all configured providers on startup.
    pub async fn prime(&self) {
        match self.refresh_jwks().await {
            Ok(()) => {
                let key_count = self.keys_count().await;
                tracing::info!(key_count, "JWKS cache primed");
            }
            Err(err) => {
                tracing::warn!(error = %err, "JWKS cache unavailable; JWT auth will fail closed until keys are fetched");
            }
        }
    }

    /// Spawn proactive refreshes every half TTL (30 minutes).
    ///
    /// The task holds only a weak cache reference, so dropping application
    /// state lets it exit without creating a detached lifetime leak.
    pub fn start_refresh_task(self: &Arc<Self>) -> JoinHandle<()> {
        self.start_refresh_task_with_interval(CACHE_TTL / 2)
    }

    fn start_refresh_task_with_interval(self: &Arc<Self>, interval: Duration) -> JoinHandle<()> {
        let cache = Arc::downgrade(self);
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            ticker.tick().await;
            loop {
                ticker.tick().await;
                let Some(cache) = cache.upgrade() else {
                    break;
                };
                if let Err(err) = cache.refresh_jwks().await {
                    tracing::warn!(error = %err, "proactive JWKS refresh failed; retaining stale keys");
                }
            }
        })
    }

    /// Return whether every configured provider has a non-empty, TTL-fresh key set.
    pub async fn is_fresh(&self) -> bool {
        let cache = self.cache.read().await;
        !self.providers.is_empty()
            && cache.len() == self.providers.len()
            && cache
                .iter()
                .all(|entry| !entry.keys.is_empty() && entry.fetched_at.elapsed() < CACHE_TTL)
    }

    /// Return the total number of cached keys.
    pub async fn keys_count(&self) -> usize {
        self.cache
            .read()
            .await
            .iter()
            .map(|entry| entry.keys.len())
            .sum()
    }

    /// Return the oldest provider refresh instant (the conservative cache age).
    pub async fn last_refresh(&self) -> Option<Instant> {
        self.cache
            .read()
            .await
            .iter()
            .map(|entry| entry.fetched_at)
            .min()
    }

    /// Return an operator-facing cache health snapshot.
    pub async fn cache_health(&self) -> CacheHealth {
        let cache = self.cache.read().await;
        let age = cache
            .iter()
            .map(|entry| entry.fetched_at)
            .min()
            .map(|instant| instant.elapsed());
        let key_count = cache.iter().map(|entry| entry.keys.len()).sum();
        let fresh = !self.providers.is_empty()
            && cache.len() == self.providers.len()
            && cache
                .iter()
                .all(|entry| !entry.keys.is_empty() && entry.fetched_at.elapsed() < CACHE_TTL);
        CacheHealth {
            fresh,
            key_count,
            age_secs: age.map(|duration| duration.as_secs()),
            stale: age.is_some_and(|duration| duration > MAX_STALE),
            fail_closed: key_count == 0 || age.is_some_and(|duration| duration > FAIL_CLOSED_STALE),
        }
    }

    /// Validate a JWT against cached keys and the provider associated with each key.
    ///
    /// Returns `Some(claims)` on success, `None` if verification fails. An
    /// unknown key ID triggers one coalesced refresh and one retry.
    pub async fn validate(&self, token: &str, privy_app_id: &str) -> Option<PrivyClaims> {
        let header = decode_header(token).ok()?;
        let kid = header.kid.as_deref()?;

        let refreshed_for_staleness = self.ensure_fresh().await;
        if !self.staleness_allows_validation().await {
            return None;
        }

        match self.try_validate_with_cache(token, kid, privy_app_id).await {
            CacheValidation::Valid(claims) => Some(claims),
            CacheValidation::Invalid => None,
            CacheValidation::UnknownKid => {
                if !refreshed_for_staleness {
                    tracing::info!(
                        kid,
                        "JWT kid not present in JWKS cache; refreshing once for rotation"
                    );
                    if let Err(err) = self.refresh_jwks().await {
                        tracing::warn!(kid, error = %err, "JWKS rotation refresh failed");
                    }
                }
                if !self.staleness_allows_validation().await {
                    return None;
                }
                match self.try_validate_with_cache(token, kid, privy_app_id).await {
                    CacheValidation::Valid(claims) => Some(claims),
                    CacheValidation::UnknownKid | CacheValidation::Invalid => None,
                }
            }
        }
    }

    async fn try_validate_with_cache(
        &self,
        token: &str,
        kid: &str,
        privy_app_id: &str,
    ) -> CacheValidation {
        let cache = self.cache.read().await;
        let mut found_kid = false;
        for entry in cache.iter() {
            for jwk in entry.keys.iter().filter(|key| key.kid == kid) {
                found_kid = true;
                let Some(decoding_key) = ec_decoding_key(jwk) else {
                    continue;
                };
                let mut validation = Validation::new(Algorithm::ES256);
                validation.set_audience(&[privy_app_id]);
                validation.set_issuer(&[entry.provider.expected_issuer.as_str()]);
                validation.required_spec_claims.insert("iss".to_string());
                if let Ok(token_data) = decode::<PrivyClaims>(token, &decoding_key, &validation) {
                    return CacheValidation::Valid(token_data.claims);
                }
            }
        }
        if found_kid {
            CacheValidation::Invalid
        } else {
            CacheValidation::UnknownKid
        }
    }

    /// Refresh stale keys and report whether this validation already made a
    /// network attempt, preventing an immediate second fetch on a `kid` miss.
    async fn ensure_fresh(&self) -> bool {
        if !self.is_fresh().await {
            if let Err(err) = self.refresh_jwks().await {
                tracing::warn!(error = %err, "JWKS refresh failed; checking retained keys");
            }
            true
        } else {
            false
        }
    }

    async fn staleness_allows_validation(&self) -> bool {
        let health = self.cache_health().await;
        if health.key_count == 0 {
            tracing::warn!("JWKS cache has no keys; rejecting JWT");
            return false;
        }
        let age_secs = health.age_secs.unwrap_or_default();
        if health.fail_closed {
            tracing::warn!(
                age_secs,
                key_count = health.key_count,
                fail_closed_after_secs = FAIL_CLOSED_STALE.as_secs(),
                "JWKS cache exceeded fail-closed staleness limit; rejecting JWT"
            );
            return false;
        }
        if age_secs > MAX_STALE.as_secs() {
            tracing::warn!(
                age_secs,
                key_count = health.key_count,
                stale_after_secs = MAX_STALE.as_secs(),
                "JWKS cache is stale; validating with retained keys"
            );
        }
        true
    }

    /// Coalesce simultaneous refresh requests into one network operation.
    async fn refresh_jwks(&self) -> Result<(), String> {
        let notified = self.refresh_notify.notified();
        tokio::pin!(notified);
        notified.as_mut().enable();
        if self
            .refreshing
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            notified.await;
            return if self.keys_count().await > 0 {
                Ok(())
            } else {
                Err("concurrent JWKS refresh did not populate any keys".to_string())
            };
        }

        let _guard = RefreshGuard {
            refreshing: &self.refreshing,
            notify: &self.refresh_notify,
        };
        self.fetch_and_merge_providers().await
    }

    async fn fetch_and_merge_providers(&self) -> Result<(), String> {
        if self.providers.is_empty() {
            return Err("no JWKS providers configured".to_string());
        }

        let results = join_all(self.providers.iter().cloned().map(|provider| async move {
            let keys = self.fetch_provider(&provider).await?;
            Ok::<_, String>((provider, keys))
        }))
        .await;

        let mut successful = Vec::new();
        let mut errors = Vec::new();
        for result in results {
            match result {
                Ok(value) => successful.push(value),
                Err(error) => errors.push(error),
            }
        }

        if successful.is_empty() {
            return Err(errors.join("; "));
        }

        let now = Instant::now();
        let mut cache = self.cache.write().await;
        for (provider, keys) in successful {
            if let Some(entry) = cache.iter_mut().find(|entry| entry.provider == provider) {
                entry.keys = keys;
                entry.fetched_at = now;
            } else {
                cache.push(CachedProviderKeys {
                    provider,
                    keys,
                    fetched_at: now,
                });
            }
        }
        drop(cache);

        for error in errors {
            tracing::warn!(error = %error, "JWKS provider refresh failed; retaining its stale keys");
        }
        Ok(())
    }

    async fn fetch_provider(&self, provider: &JwksProvider) -> Result<Vec<Jwk>, String> {
        let response = self
            .http
            .get(&provider.url)
            .timeout(self.fetch_timeout)
            .send()
            .await
            .map_err(|error| format!("{}: JWKS fetch failed: {error}", provider.url))?;

        if !response.status().is_success() {
            return Err(format!(
                "{}: JWKS endpoint returned {}",
                provider.url,
                response.status()
            ));
        }

        let jwks: JwksResponse = response
            .json()
            .await
            .map_err(|error| format!("{}: JWKS parse failed: {error}", provider.url))?;
        if jwks.keys.is_empty() {
            return Err(format!("{}: JWKS endpoint returned no keys", provider.url));
        }
        Ok(jwks.keys)
    }
}

fn ec_decoding_key(jwk: &Jwk) -> Option<DecodingKey> {
    if jwk.kty != "EC" || jwk.crv.as_deref() != Some("P-256") {
        return None;
    }
    DecodingKey::from_ec_components(jwk.x.as_deref()?, jwk.y.as_deref()?).ok()
}

/// Create a default-provider cache wrapped in [`Arc`].
pub fn new_jwks_cache(http: reqwest::Client) -> Arc<JwksCache> {
    Arc::new(JwksCache::new(http))
}

/// Create a default-provider cache with a configurable fetch timeout.
pub fn new_jwks_cache_with_timeout(
    http: reqwest::Client,
    fetch_timeout: Duration,
) -> Arc<JwksCache> {
    Arc::new(JwksCache::with_timeout(http, fetch_timeout))
}

/// Create a multi-provider cache with a configurable fetch timeout.
pub fn new_jwks_cache_with_providers(
    http: reqwest::Client,
    providers: Vec<JwksProvider>,
    fetch_timeout: Duration,
) -> Arc<JwksCache> {
    Arc::new(JwksCache::with_providers_and_timeout(
        http,
        providers,
        fetch_timeout,
    ))
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use axum::extract::State;
    use axum::routing::get;
    use axum::{Json, Router};
    use jsonwebtoken::{EncodingKey, Header, encode};
    use serde::Serialize;
    use serde_json::{Value, json};

    use super::*;

    const TEST_APP_ID: &str = "test-app";
    const TEST_X: &str = "w7JAoU_gJbZJvV-zCOvU9yFJq0FNC_edCMRM78P8eQQ";
    const TEST_Y: &str = "wQg1EytcsEmGrM70Gb53oluoDbVhCZ3Uq3hHMslHVb4";
    const TEST_PRIVATE_KEY: &str = r#"-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgWTFfCGljY6aw3Hrt
kHmPRiazukxPLb6ilpRAewjW8nihRANCAATDskChT+Altkm9X7MI69T3IUmrQU0L
950IxEzvw/x5BMEINRMrXLBJhqzO9Bm+d6JbqA21YQmd1Kt4RzLJR1W+
-----END PRIVATE KEY-----"#;

    #[derive(Clone)]
    struct TestServerState {
        responses: Arc<Vec<Value>>,
        hits: Arc<AtomicUsize>,
        delay: Duration,
    }

    #[derive(Serialize)]
    struct TestClaims<'a> {
        sub: &'a str,
        iss: &'a str,
        aud: &'a str,
        exp: u64,
    }

    fn jwks(kid: &str) -> Value {
        json!({
            "keys": [{
                "kid": kid,
                "kty": "EC",
                "crv": "P-256",
                "x": TEST_X,
                "y": TEST_Y
            }]
        })
    }

    fn token(kid: &str, issuer: &str) -> String {
        let mut header = Header::new(Algorithm::ES256);
        header.kid = Some(kid.to_string());
        let claims = TestClaims {
            sub: "did:test:user",
            iss: issuer,
            aud: TEST_APP_ID,
            exp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock after epoch")
                .as_secs()
                + 3600,
        };
        encode(
            &header,
            &claims,
            &EncodingKey::from_ec_pem(TEST_PRIVATE_KEY.as_bytes()).expect("test EC key"),
        )
        .expect("encode test JWT")
    }

    fn corrupt_signature(token: &str) -> String {
        let mut parts = token.split('.').map(str::to_string).collect::<Vec<_>>();
        let replacement = if parts[2].starts_with('A') { "B" } else { "A" };
        parts[2].replace_range(..1, replacement);
        parts.join(".")
    }

    async fn test_jwks_handler(State(state): State<TestServerState>) -> Json<Value> {
        if !state.delay.is_zero() {
            tokio::time::sleep(state.delay).await;
        }
        let hit = state.hits.fetch_add(1, Ordering::SeqCst);
        let index = hit.min(state.responses.len().saturating_sub(1));
        Json(state.responses[index].clone())
    }

    async fn spawn_jwks_server(
        responses: Vec<Value>,
        delay: Duration,
    ) -> (String, Arc<AtomicUsize>, JoinHandle<()>) {
        assert!(!responses.is_empty());
        let hits = Arc::new(AtomicUsize::new(0));
        let state = TestServerState {
            responses: Arc::new(responses),
            hits: Arc::clone(&hits),
            delay,
        };
        let app = Router::new()
            .route("/jwks", get(test_jwks_handler))
            .with_state(state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test JWKS server");
        let address = listener.local_addr().expect("test server address");
        let handle = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("serve test JWKS endpoint");
        });
        (format!("http://{address}/jwks"), hits, handle)
    }

    fn cache_for(url: impl Into<String>, issuer: &str) -> JwksCache {
        JwksCache::with_providers_and_timeout(
            reqwest::Client::new(),
            vec![JwksProvider::new(url, issuer)],
            Duration::from_millis(250),
        )
    }

    async fn seed(cache: &JwksCache, provider: JwksProvider, kid: &str, age: Duration) {
        cache.cache.write().await.push(CachedProviderKeys {
            provider,
            keys: serde_json::from_value::<JwksResponse>(jwks(kid))
                .expect("test JWKS")
                .keys,
            fetched_at: Instant::now() - age,
        });
    }

    #[tokio::test]
    async fn health_reports_fresh_stale_and_fail_closed_states() {
        let provider = JwksProvider::new("http://127.0.0.1:1/jwks", "issuer-a");
        let cache = cache_for(&provider.url, &provider.expected_issuer);
        let empty = cache.cache_health().await;
        assert!(!empty.fresh);
        assert_eq!(empty.key_count, 0);
        assert!(empty.fail_closed);

        seed(&cache, provider, "key-a", Duration::from_secs(1)).await;
        let fresh = cache.cache_health().await;
        assert!(fresh.fresh);
        assert!(!fresh.stale);
        assert!(!fresh.fail_closed);

        cache.cache.write().await[0].fetched_at =
            Instant::now() - MAX_STALE - Duration::from_secs(1);
        let stale = cache.cache_health().await;
        assert!(!stale.fresh);
        assert!(stale.stale);
        assert!(!stale.fail_closed);

        cache.cache.write().await[0].fetched_at =
            Instant::now() - FAIL_CLOSED_STALE - Duration::from_secs(1);
        assert!(cache.cache_health().await.fail_closed);
    }

    #[tokio::test]
    async fn unknown_kid_refreshes_once_but_bad_signature_does_not() {
        let (url, hits, server) =
            spawn_jwks_server(vec![jwks("old-key"), jwks("rotated-key")], Duration::ZERO).await;
        let cache = cache_for(url, "issuer-a");
        cache.prime().await;

        let rotated = token("rotated-key", "issuer-a");
        let claims = cache
            .validate(&rotated, TEST_APP_ID)
            .await
            .expect("rotated key should validate after refresh");
        assert_eq!(claims.sub, "did:test:user");
        assert_eq!(hits.load(Ordering::SeqCst), 2);

        let corrupted = corrupt_signature(&rotated);
        assert!(cache.validate(&corrupted, TEST_APP_ID).await.is_none());
        assert_eq!(hits.load(Ordering::SeqCst), 2);
        server.abort();
    }

    #[tokio::test]
    async fn expected_issuer_is_enforced_per_provider() {
        let (url, hits, server) = spawn_jwks_server(vec![jwks("shared-key")], Duration::ZERO).await;
        let cache = cache_for(url, "issuer-a");
        cache.prime().await;

        assert!(
            cache
                .validate(&token("shared-key", "issuer-b"), TEST_APP_ID)
                .await
                .is_none()
        );
        assert!(
            cache
                .validate(&token("shared-key", "issuer-a"), TEST_APP_ID)
                .await
                .is_some()
        );
        assert_eq!(hits.load(Ordering::SeqCst), 1);
        server.abort();
    }

    #[tokio::test]
    async fn partial_provider_failure_retains_working_provider() {
        let (good_url, _hits, server) =
            spawn_jwks_server(vec![jwks("good-key")], Duration::ZERO).await;
        let dead_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("reserve dead provider port");
        let dead_url = format!(
            "http://{}/jwks",
            dead_listener.local_addr().expect("dead provider address")
        );
        drop(dead_listener);
        let cache = JwksCache::with_providers_and_timeout(
            reqwest::Client::new(),
            vec![
                JwksProvider::new(dead_url, "issuer-dead"),
                JwksProvider::new(good_url, "issuer-good"),
            ],
            Duration::from_millis(250),
        );

        cache.prime().await;
        assert_eq!(cache.keys_count().await, 1);
        assert!(!cache.is_fresh().await);
        assert!(
            cache
                .validate(&token("good-key", "issuer-good"), TEST_APP_ID)
                .await
                .is_some()
        );
        server.abort();
    }

    #[tokio::test]
    async fn stale_keys_survive_network_failure_until_48_hour_limit() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("reserve dead provider port");
        let url = format!(
            "http://{}/jwks",
            listener.local_addr().expect("dead provider address")
        );
        drop(listener);
        let provider = JwksProvider::new(url, "issuer-a");
        let cache = cache_for(&provider.url, &provider.expected_issuer);
        seed(
            &cache,
            provider,
            "stale-key",
            MAX_STALE + Duration::from_secs(1),
        )
        .await;
        let signed = token("stale-key", "issuer-a");
        assert!(cache.validate(&signed, TEST_APP_ID).await.is_some());

        cache.cache.write().await[0].fetched_at =
            Instant::now() - FAIL_CLOSED_STALE - Duration::from_secs(1);
        assert!(cache.validate(&signed, TEST_APP_ID).await.is_none());
    }

    #[tokio::test]
    async fn simultaneous_refreshes_are_coalesced() {
        let (url, hits, server) =
            spawn_jwks_server(vec![jwks("key-a")], Duration::from_millis(50)).await;
        let cache = Arc::new(cache_for(url, "issuer-a"));
        let refreshes = (0..12).map(|_| {
            let cache = Arc::clone(&cache);
            tokio::spawn(async move { cache.refresh_jwks().await })
        });
        for result in join_all(refreshes).await {
            result.expect("refresh task").expect("refresh result");
        }
        assert_eq!(hits.load(Ordering::SeqCst), 1);
        server.abort();
    }

    #[tokio::test]
    async fn proactive_refresh_task_fetches_without_validation_traffic() {
        let (url, hits, server) = spawn_jwks_server(vec![jwks("key-a")], Duration::ZERO).await;
        let cache = Arc::new(cache_for(url, "issuer-a"));
        let refresh = cache.start_refresh_task_with_interval(Duration::from_millis(10));
        tokio::time::timeout(Duration::from_secs(1), async {
            while hits.load(Ordering::SeqCst) == 0 {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("proactive refresh did not run");
        assert_eq!(cache.keys_count().await, 1);
        refresh.abort();
        server.abort();
    }
}
