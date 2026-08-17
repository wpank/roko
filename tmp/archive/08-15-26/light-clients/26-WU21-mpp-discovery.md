# WU-21: MPP Service Discovery

**Layer**: 4
**Depends on**: WU-17 (MppClient), WU-18 (MPP Tool Handlers for the discover tool def)
**Blocks**: None
**Estimated effort**: 2-3 hours
**Crate**: `crates/roko-chain`
**Feature gate**: `mpp`

---

## Overview

Implement programmatic discovery of MPP-enabled services. Tempo's payments directory has 100+ services (OpenAI, Anthropic, Browserbase, etc.) but no public REST API for querying it. Discovery happens through two mechanisms:

1. **Per-service**: Fetch `GET {service_url}/openapi.json` — the normative spec (draft-payment-discovery-00). Each MPP service publishes its capabilities via OpenAPI with `x-payment-info` extensions.
2. **Directory scraping**: The curated service registry lives at `github.com/tempoxyz/mpp` in `schemas/services.ts`. We can vendor a snapshot as a fallback catalog.

---

## Pre-read

- `crates/roko-chain/src/mpp_client.rs` — `MppClient`, `MppConfig`, `VerifiedPayment` (WU-17)
- `crates/roko-chain/src/tools.rs` — `mpp_discover_tool_def()` (WU-18)
- `crates/roko-chain/src/types.rs` — `ChainError`
- `crates/roko-chain/src/lib.rs` — module registry
- `22-WU17-mpp-client.md` — MppClient design and dependency context
- `23-WU18-mpp-tools.md` — `chain.mpp_discover` tool definition and `handle_mpp_discover()` dispatch arm

---

## Tasks

### 21.1 Create `crates/roko-chain/src/mpp_discovery.rs`

**File**: `crates/roko-chain/src/mpp_discovery.rs`

The entire module is feature-gated behind `#[cfg(feature = "mpp")]`.

```rust
//! MPP service discovery via OpenAPI + `x-payment-info` extensions.
//!
//! Implements draft-payment-discovery-00: each MPP service publishes its
//! capabilities at `GET {base_url}/openapi.json` with pricing and payment
//! method metadata in `x-payment-info` extension fields.
//!
//! Also includes a vendored fallback catalog of ~50 known MPP services
//! from the tempoxyz/mpp registry.
//!
//! # Feature gate
//! Requires `mpp` feature: `cargo build -p roko-chain --features mpp`

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::types::ChainError;

// ── Types ────────────────────────────────────────────────────────────

/// Discovered service capabilities from OpenAPI + x-payment-info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredService {
    /// Base URL of the service.
    pub url: String,
    /// Service name (from OpenAPI info.title).
    pub name: Option<String>,
    /// Service description.
    pub description: Option<String>,
    /// Available endpoints with pricing.
    pub endpoints: Vec<DiscoveredEndpoint>,
    /// Accepted payment methods.
    pub payment_methods: Vec<PaymentMethod>,
    /// When this discovery was performed.
    pub discovered_at: String,
    /// Cache TTL (from Cache-Control, default 300s).
    pub cache_ttl_secs: u64,
}

/// A single discoverable endpoint with pricing metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredEndpoint {
    /// HTTP method + path (e.g., "POST /v1/chat/completions").
    pub route: String,
    /// Description.
    pub description: String,
    /// Price in base units (None = dynamic pricing).
    pub amount: Option<String>,
    /// Human-readable price hint (e.g., "$0.01/request").
    pub amount_hint: Option<String>,
    /// Payment intent: "charge" or "session".
    pub intent: String,
    /// Unit type (e.g., "request", "hour").
    pub unit_type: Option<String>,
}

/// A payment method accepted by a service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentMethod {
    /// Method type: "tempo" or "stripe".
    pub method: String,
    /// Currency: token address or "usd".
    pub currency: String,
    /// Decimal places for the currency.
    pub decimals: u32,
}

// ── Discovery Client ─────────────────────────────────────────────────

/// Service discovery client with LRU caching and fallback catalog.
pub struct MppDiscovery {
    /// HTTP client for fetching OpenAPI documents.
    http_client: reqwest::Client,
    /// LRU cache of discovered services (5-minute TTL per spec).
    cache: Arc<RwLock<HashMap<String, (DiscoveredService, Instant)>>>,
    /// Fallback catalog (vendored from tempoxyz/mpp schemas).
    fallback_catalog: Vec<ServiceCatalogEntry>,
}

impl MppDiscovery {
    /// Create a new discovery client with the default fallback catalog.
    pub fn new() -> Self {
        Self {
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
            cache: Arc::new(RwLock::new(HashMap::new())),
            fallback_catalog: crate::mpp_catalog::CATALOG.to_vec(),
        }
    }

    /// Create a discovery client with a custom HTTP client (for testing).
    pub fn with_http_client(http_client: reqwest::Client) -> Self {
        Self {
            http_client,
            cache: Arc::new(RwLock::new(HashMap::new())),
            fallback_catalog: crate::mpp_catalog::CATALOG.to_vec(),
        }
    }

    /// Discover a single service by fetching its OpenAPI document.
    ///
    /// Flow:
    /// 1. Check the LRU cache for a non-expired entry
    /// 2. If cache miss, fetch `GET {service_url}/openapi.json`
    /// 3. Parse the OpenAPI document for `x-payment-info` extensions
    /// 4. Extract endpoints, pricing, and payment methods
    /// 5. Cache the result with the TTL from Cache-Control (default 300s)
    ///
    /// # Arguments
    /// - `service_url`: Base URL of the service (e.g., "https://openai.mpp.tempo.xyz")
    ///
    /// # Errors
    /// Returns error if the service is unreachable, returns non-200, or
    /// the OpenAPI document cannot be parsed.
    pub async fn discover(&self, service_url: &str) -> Result<DiscoveredService, ChainError> {
        let normalized = service_url.trim_end_matches('/').to_string();

        // Check cache
        {
            let cache = self.cache.read().await;
            if let Some((service, cached_at)) = cache.get(&normalized) {
                let age = cached_at.elapsed().as_secs();
                if age < service.cache_ttl_secs {
                    return Ok(service.clone());
                }
            }
        }

        // Fetch OpenAPI document
        let openapi_url = format!("{normalized}/openapi.json");
        let response = self.http_client
            .get(&openapi_url)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| ChainError::Other(format!(
                "failed to fetch OpenAPI from {openapi_url}: {e}"
            )))?;

        // Extract Cache-Control TTL
        let cache_ttl_secs = parse_cache_control_max_age(
            response.headers().get("cache-control")
        ).unwrap_or(300);

        let status = response.status();
        if !status.is_success() {
            return Err(ChainError::Other(format!(
                "OpenAPI fetch returned {status} for {openapi_url}"
            )));
        }

        let body: serde_json::Value = response.json().await
            .map_err(|e| ChainError::Other(format!(
                "failed to parse OpenAPI JSON from {openapi_url}: {e}"
            )))?;

        // Parse the OpenAPI document
        let service = parse_openapi_document(&normalized, &body, cache_ttl_secs)?;

        // Update cache
        {
            let mut cache = self.cache.write().await;
            cache.insert(normalized, (service.clone(), Instant::now()));
        }

        Ok(service)
    }

    /// Search the fallback catalog by keyword.
    ///
    /// Matches against service name, id, and categories (case-insensitive).
    pub fn search_catalog(&self, query: &str) -> Vec<&ServiceCatalogEntry> {
        let q = query.to_lowercase();
        self.fallback_catalog
            .iter()
            .filter(|entry| {
                entry.id.to_lowercase().contains(&q)
                    || entry.name.to_lowercase().contains(&q)
                    || entry.categories.iter().any(|c| c.to_lowercase().contains(&q))
            })
            .collect()
    }

    /// List all services in the fallback catalog.
    pub fn list_catalog(&self) -> &[ServiceCatalogEntry] {
        &self.fallback_catalog
    }

    /// Invalidate a cached discovery result.
    pub async fn invalidate(&self, service_url: &str) {
        let normalized = service_url.trim_end_matches('/').to_string();
        let mut cache = self.cache.write().await;
        cache.remove(&normalized);
    }

    /// Clear the entire cache.
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }
}

// ── Catalog Entry ────────────────────────────────────────────────────

/// A service entry from the vendored fallback catalog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceCatalogEntry {
    /// Short identifier (e.g., "openai").
    pub id: String,
    /// Display name (e.g., "OpenAI").
    pub name: String,
    /// MPP service URL.
    pub url: String,
    /// Category tags.
    pub categories: Vec<String>,
    /// Default payment intent: "charge" or "session".
    pub intent: String,
}

// ── OpenAPI Parsing ──────────────────────────────────────────────────

/// Parse an OpenAPI document with x-payment-info extensions into a
/// `DiscoveredService`.
fn parse_openapi_document(
    service_url: &str,
    doc: &serde_json::Value,
    cache_ttl_secs: u64,
) -> Result<DiscoveredService, ChainError> {
    let info = doc.get("info").unwrap_or(&serde_json::Value::Null);
    let name = info.get("title").and_then(|v| v.as_str()).map(String::from);
    let description = info.get("description").and_then(|v| v.as_str()).map(String::from);

    // Parse endpoints from paths
    let mut endpoints = Vec::new();
    if let Some(paths) = doc.get("paths").and_then(|p| p.as_object()) {
        for (path, methods) in paths {
            if let Some(methods_obj) = methods.as_object() {
                for (method, operation) in methods_obj {
                    // Skip non-HTTP-method keys (e.g., "parameters", "summary")
                    let http_method = method.to_uppercase();
                    if !matches!(http_method.as_str(), "GET" | "POST" | "PUT" | "DELETE" | "PATCH") {
                        continue;
                    }

                    let route = format!("{http_method} {path}");
                    let desc = operation
                        .get("description")
                        .or_else(|| operation.get("summary"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    // Extract x-payment-info extension
                    let payment_info = operation.get("x-payment-info");
                    let (amount, amount_hint, intent, unit_type) =
                        parse_payment_info(payment_info);

                    endpoints.push(DiscoveredEndpoint {
                        route,
                        description: desc,
                        amount,
                        amount_hint,
                        intent: intent.unwrap_or_else(|| "charge".to_string()),
                        unit_type,
                    });
                }
            }
        }
    }

    // Parse payment methods from top-level x-payment-info or info extension
    let payment_methods = parse_payment_methods(doc);

    let discovered_at = chrono::Utc::now().to_rfc3339();

    Ok(DiscoveredService {
        url: service_url.to_string(),
        name,
        description,
        endpoints,
        payment_methods,
        discovered_at,
        cache_ttl_secs,
    })
}

/// Extract pricing fields from an `x-payment-info` extension object.
fn parse_payment_info(
    info: Option<&serde_json::Value>,
) -> (Option<String>, Option<String>, Option<String>, Option<String>) {
    let info = match info {
        Some(v) => v,
        None => return (None, None, None, None),
    };

    let amount = info.get("amount").and_then(|v| match v {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        _ => None,
    });

    let amount_hint = info
        .get("amount_hint")
        .or_else(|| info.get("amountHint"))
        .and_then(|v| v.as_str())
        .map(String::from);

    let intent = info
        .get("intent")
        .and_then(|v| v.as_str())
        .map(String::from);

    let unit_type = info
        .get("unit_type")
        .or_else(|| info.get("unitType"))
        .and_then(|v| v.as_str())
        .map(String::from);

    (amount, amount_hint, intent, unit_type)
}

/// Parse payment methods from the top-level or info-level x-payment-info.
fn parse_payment_methods(doc: &serde_json::Value) -> Vec<PaymentMethod> {
    // Check top-level x-payment-info.payment_methods
    let payment_info = doc
        .get("x-payment-info")
        .or_else(|| doc.get("info").and_then(|i| i.get("x-payment-info")));

    let methods_val = match payment_info {
        Some(info) => info.get("payment_methods").or_else(|| info.get("paymentMethods")),
        None => None,
    };

    match methods_val {
        Some(serde_json::Value::Array(arr)) => {
            arr.iter()
                .filter_map(|v| {
                    let method = v.get("method").and_then(|m| m.as_str())?;
                    let currency = v.get("currency").and_then(|c| c.as_str())?;
                    let decimals = v
                        .get("decimals")
                        .and_then(|d| d.as_u64())
                        .unwrap_or(6) as u32;
                    Some(PaymentMethod {
                        method: method.to_string(),
                        currency: currency.to_string(),
                        decimals,
                    })
                })
                .collect()
        }
        _ => Vec::new(),
    }
}

/// Parse max-age from a Cache-Control header value.
fn parse_cache_control_max_age(header: Option<&reqwest::header::HeaderValue>) -> Option<u64> {
    let val = header?.to_str().ok()?;
    for directive in val.split(',') {
        let directive = directive.trim();
        if let Some(age_str) = directive.strip_prefix("max-age=") {
            return age_str.trim().parse().ok();
        }
    }
    None
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    // ── DiscoveredService serde roundtrip ─────────────────────────

    #[test]
    fn discovered_service_serde_roundtrip() {
        let service = DiscoveredService {
            url: "https://openai.mpp.tempo.xyz".to_string(),
            name: Some("OpenAI".to_string()),
            description: Some("OpenAI API via MPP".to_string()),
            endpoints: vec![DiscoveredEndpoint {
                route: "POST /v1/chat/completions".to_string(),
                description: "Chat completion".to_string(),
                amount: Some("10000".to_string()),
                amount_hint: Some("$0.01/request".to_string()),
                intent: "charge".to_string(),
                unit_type: Some("request".to_string()),
            }],
            payment_methods: vec![PaymentMethod {
                method: "tempo".to_string(),
                currency: "0xUSDC".to_string(),
                decimals: 6,
            }],
            discovered_at: "2026-01-01T00:00:00Z".to_string(),
            cache_ttl_secs: 300,
        };
        let json = serde_json::to_string(&service).unwrap();
        let roundtripped: DiscoveredService = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtripped.url, service.url);
        assert_eq!(roundtripped.name.as_deref(), Some("OpenAI"));
        assert_eq!(roundtripped.endpoints.len(), 1);
        assert_eq!(roundtripped.endpoints[0].route, "POST /v1/chat/completions");
        assert_eq!(roundtripped.payment_methods.len(), 1);
        assert_eq!(roundtripped.payment_methods[0].method, "tempo");
        assert_eq!(roundtripped.cache_ttl_secs, 300);
    }

    #[test]
    fn discovered_endpoint_serde_roundtrip() {
        let endpoint = DiscoveredEndpoint {
            route: "GET /v1/models".to_string(),
            description: "List models".to_string(),
            amount: None,
            amount_hint: None,
            intent: "charge".to_string(),
            unit_type: None,
        };
        let json = serde_json::to_string(&endpoint).unwrap();
        let roundtripped: DiscoveredEndpoint = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtripped.route, "GET /v1/models");
        assert!(roundtripped.amount.is_none());
    }

    #[test]
    fn payment_method_serde_roundtrip() {
        let pm = PaymentMethod {
            method: "stripe".to_string(),
            currency: "usd".to_string(),
            decimals: 2,
        };
        let json = serde_json::to_string(&pm).unwrap();
        let roundtripped: PaymentMethod = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtripped.method, "stripe");
        assert_eq!(roundtripped.currency, "usd");
        assert_eq!(roundtripped.decimals, 2);
    }

    // ── OpenAPI parsing ──────────────────────────────────────────

    #[test]
    fn parse_openapi_with_payment_info() {
        let doc = serde_json::json!({
            "openapi": "3.0.0",
            "info": {
                "title": "Test Service",
                "description": "A test MPP service",
                "x-payment-info": {
                    "payment_methods": [
                        {
                            "method": "tempo",
                            "currency": "0xUSDC",
                            "decimals": 6
                        }
                    ]
                }
            },
            "paths": {
                "/v1/query": {
                    "post": {
                        "summary": "Run a query",
                        "x-payment-info": {
                            "amount": "50000",
                            "amount_hint": "$0.05/request",
                            "intent": "charge",
                            "unit_type": "request"
                        }
                    }
                }
            }
        });

        let service = parse_openapi_document("https://test.mpp.tempo.xyz", &doc, 300).unwrap();
        assert_eq!(service.name.as_deref(), Some("Test Service"));
        assert_eq!(service.description.as_deref(), Some("A test MPP service"));
        assert_eq!(service.endpoints.len(), 1);
        assert_eq!(service.endpoints[0].route, "POST /v1/query");
        assert_eq!(service.endpoints[0].amount.as_deref(), Some("50000"));
        assert_eq!(service.endpoints[0].amount_hint.as_deref(), Some("$0.05/request"));
        assert_eq!(service.endpoints[0].intent, "charge");
        assert_eq!(service.payment_methods.len(), 1);
        assert_eq!(service.payment_methods[0].method, "tempo");
    }

    #[test]
    fn parse_openapi_without_payment_info() {
        let doc = serde_json::json!({
            "openapi": "3.0.0",
            "info": { "title": "Plain Service" },
            "paths": {
                "/health": {
                    "get": { "summary": "Health check" }
                }
            }
        });

        let service = parse_openapi_document("https://plain.example.com", &doc, 300).unwrap();
        assert_eq!(service.name.as_deref(), Some("Plain Service"));
        assert_eq!(service.endpoints.len(), 1);
        assert!(service.endpoints[0].amount.is_none());
        assert_eq!(service.endpoints[0].intent, "charge"); // default
        assert!(service.payment_methods.is_empty());
    }

    #[test]
    fn parse_openapi_multiple_methods_and_paths() {
        let doc = serde_json::json!({
            "openapi": "3.0.0",
            "info": { "title": "Multi" },
            "paths": {
                "/v1/chat": {
                    "post": {
                        "summary": "Chat",
                        "x-payment-info": { "amount": "10000", "intent": "charge" }
                    }
                },
                "/v1/embed": {
                    "post": {
                        "summary": "Embed",
                        "x-payment-info": { "amount": "5000", "intent": "charge" }
                    }
                },
                "/v1/models": {
                    "get": { "summary": "List models" }
                }
            }
        });

        let service = parse_openapi_document("https://multi.mpp.tempo.xyz", &doc, 600).unwrap();
        assert_eq!(service.endpoints.len(), 3);
        assert_eq!(service.cache_ttl_secs, 600);
    }

    // ── Cache-Control parsing ────────────────────────────────────

    #[test]
    fn parse_cache_control_basic() {
        let hv = reqwest::header::HeaderValue::from_static("max-age=600");
        assert_eq!(parse_cache_control_max_age(Some(&hv)), Some(600));
    }

    #[test]
    fn parse_cache_control_with_directives() {
        let hv = reqwest::header::HeaderValue::from_static("public, max-age=120, must-revalidate");
        assert_eq!(parse_cache_control_max_age(Some(&hv)), Some(120));
    }

    #[test]
    fn parse_cache_control_none() {
        assert_eq!(parse_cache_control_max_age(None), None);
    }

    #[test]
    fn parse_cache_control_no_max_age() {
        let hv = reqwest::header::HeaderValue::from_static("no-cache");
        assert_eq!(parse_cache_control_max_age(Some(&hv)), None);
    }

    // ── Catalog search ───────────────────────────────────────────

    #[test]
    fn catalog_search_by_name() {
        let discovery = MppDiscovery::new();
        let results = discovery.search_catalog("openai");
        assert!(!results.is_empty());
        assert!(results.iter().any(|e| e.id == "openai"));
    }

    #[test]
    fn catalog_search_by_category() {
        let discovery = MppDiscovery::new();
        let results = discovery.search_catalog("ai");
        assert!(!results.is_empty());
    }

    #[test]
    fn catalog_search_case_insensitive() {
        let discovery = MppDiscovery::new();
        let results = discovery.search_catalog("ANTHROPIC");
        assert!(!results.is_empty());
        assert!(results.iter().any(|e| e.id == "anthropic"));
    }

    #[test]
    fn catalog_search_no_match() {
        let discovery = MppDiscovery::new();
        let results = discovery.search_catalog("zzz_nonexistent_service_zzz");
        assert!(results.is_empty());
    }

    #[test]
    fn catalog_list_returns_all() {
        let discovery = MppDiscovery::new();
        let all = discovery.list_catalog();
        assert!(all.len() >= 10, "expected at least 10 catalog entries, got {}", all.len());
    }

    // ── ServiceCatalogEntry serde ────────────────────────────────

    #[test]
    fn catalog_entry_serde_roundtrip() {
        let entry = ServiceCatalogEntry {
            id: "test".to_string(),
            name: "Test Service".to_string(),
            url: "https://test.mpp.tempo.xyz".to_string(),
            categories: vec!["testing".to_string()],
            intent: "charge".to_string(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let roundtripped: ServiceCatalogEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtripped.id, "test");
        assert_eq!(roundtripped.categories, vec!["testing"]);
    }
}
```

### 21.2 Create `crates/roko-chain/src/mpp_catalog.rs`

**File**: `crates/roko-chain/src/mpp_catalog.rs`

Vendor a static snapshot of the top ~50 MPP services from `tempoxyz/mpp/schemas/services.ts`, converted to Rust. Feature-gated behind `#[cfg(feature = "mpp")]`.

```rust
//! Vendored fallback catalog of known MPP services.
//!
//! Sourced from `github.com/tempoxyz/mpp` schemas/services.ts (snapshot).
//! Used as a fallback when live OpenAPI discovery is unavailable.

use crate::mpp_discovery::ServiceCatalogEntry;

/// Static catalog of known MPP services.
///
/// This is a vendored snapshot — update periodically from the upstream
/// tempoxyz/mpp repository.
pub static CATALOG: &[ServiceCatalogEntry] = &[
    // ── AI / LLM ─────────────────────────────────────────────────
    ServiceCatalogEntry {
        id: "openai",
        name: "OpenAI",
        url: "https://openai.mpp.tempo.xyz",
        categories: &["ai", "llm"],
        intent: "charge",
    },
    ServiceCatalogEntry {
        id: "anthropic",
        name: "Anthropic",
        url: "https://anthropic.mpp.tempo.xyz",
        categories: &["ai", "llm"],
        intent: "charge",
    },
    ServiceCatalogEntry {
        id: "google-ai",
        name: "Google AI",
        url: "https://google-ai.mpp.tempo.xyz",
        categories: &["ai", "llm"],
        intent: "charge",
    },
    ServiceCatalogEntry {
        id: "mistral",
        name: "Mistral",
        url: "https://mistral.mpp.tempo.xyz",
        categories: &["ai", "llm"],
        intent: "charge",
    },
    ServiceCatalogEntry {
        id: "cohere",
        name: "Cohere",
        url: "https://cohere.mpp.tempo.xyz",
        categories: &["ai", "llm"],
        intent: "charge",
    },
    ServiceCatalogEntry {
        id: "groq",
        name: "Groq",
        url: "https://groq.mpp.tempo.xyz",
        categories: &["ai", "llm", "inference"],
        intent: "charge",
    },
    ServiceCatalogEntry {
        id: "together",
        name: "Together AI",
        url: "https://together.mpp.tempo.xyz",
        categories: &["ai", "llm", "inference"],
        intent: "charge",
    },
    ServiceCatalogEntry {
        id: "fireworks",
        name: "Fireworks AI",
        url: "https://fireworks.mpp.tempo.xyz",
        categories: &["ai", "inference"],
        intent: "charge",
    },
    ServiceCatalogEntry {
        id: "perplexity",
        name: "Perplexity",
        url: "https://perplexity.mpp.tempo.xyz",
        categories: &["ai", "search"],
        intent: "charge",
    },
    ServiceCatalogEntry {
        id: "replicate",
        name: "Replicate",
        url: "https://replicate.mpp.tempo.xyz",
        categories: &["ai", "inference"],
        intent: "charge",
    },

    // ── Browser / Automation ─────────────────────────────────────
    ServiceCatalogEntry {
        id: "browserbase",
        name: "Browserbase",
        url: "https://browserbase.mpp.tempo.xyz",
        categories: &["browser", "automation"],
        intent: "session",
    },
    ServiceCatalogEntry {
        id: "steel",
        name: "Steel",
        url: "https://steel.mpp.tempo.xyz",
        categories: &["browser", "automation"],
        intent: "session",
    },

    // ── Code / Dev Tools ─────────────────────────────────────────
    ServiceCatalogEntry {
        id: "e2b",
        name: "E2B",
        url: "https://e2b.mpp.tempo.xyz",
        categories: &["code", "sandbox"],
        intent: "session",
    },
    ServiceCatalogEntry {
        id: "replit",
        name: "Replit",
        url: "https://replit.mpp.tempo.xyz",
        categories: &["code", "ide"],
        intent: "session",
    },

    // ── Data / Search ────────────────────────────────────────────
    ServiceCatalogEntry {
        id: "exa",
        name: "Exa",
        url: "https://exa.mpp.tempo.xyz",
        categories: &["search", "data"],
        intent: "charge",
    },
    ServiceCatalogEntry {
        id: "tavily",
        name: "Tavily",
        url: "https://tavily.mpp.tempo.xyz",
        categories: &["search", "data"],
        intent: "charge",
    },
    ServiceCatalogEntry {
        id: "firecrawl",
        name: "Firecrawl",
        url: "https://firecrawl.mpp.tempo.xyz",
        categories: &["scraping", "data"],
        intent: "charge",
    },
    ServiceCatalogEntry {
        id: "spider",
        name: "Spider",
        url: "https://spider.mpp.tempo.xyz",
        categories: &["scraping", "data"],
        intent: "charge",
    },

    // ── Image / Video / Media ────────────────────────────────────
    ServiceCatalogEntry {
        id: "stability",
        name: "Stability AI",
        url: "https://stability.mpp.tempo.xyz",
        categories: &["ai", "image"],
        intent: "charge",
    },
    ServiceCatalogEntry {
        id: "fal",
        name: "Fal",
        url: "https://fal.mpp.tempo.xyz",
        categories: &["ai", "image", "inference"],
        intent: "charge",
    },
    ServiceCatalogEntry {
        id: "elevenlabs",
        name: "ElevenLabs",
        url: "https://elevenlabs.mpp.tempo.xyz",
        categories: &["ai", "audio", "tts"],
        intent: "charge",
    },
    ServiceCatalogEntry {
        id: "deepgram",
        name: "Deepgram",
        url: "https://deepgram.mpp.tempo.xyz",
        categories: &["ai", "audio", "stt"],
        intent: "charge",
    },
    ServiceCatalogEntry {
        id: "heygen",
        name: "HeyGen",
        url: "https://heygen.mpp.tempo.xyz",
        categories: &["ai", "video"],
        intent: "charge",
    },

    // ── Storage / Infra ──────────────────────────────────────────
    ServiceCatalogEntry {
        id: "pinata",
        name: "Pinata",
        url: "https://pinata.mpp.tempo.xyz",
        categories: &["storage", "ipfs"],
        intent: "charge",
    },
    ServiceCatalogEntry {
        id: "neon",
        name: "Neon",
        url: "https://neon.mpp.tempo.xyz",
        categories: &["database", "postgres"],
        intent: "session",
    },
    ServiceCatalogEntry {
        id: "supabase",
        name: "Supabase",
        url: "https://supabase.mpp.tempo.xyz",
        categories: &["database", "backend"],
        intent: "session",
    },
    ServiceCatalogEntry {
        id: "upstash",
        name: "Upstash",
        url: "https://upstash.mpp.tempo.xyz",
        categories: &["database", "redis", "kafka"],
        intent: "charge",
    },

    // ── Comms / Email ────────────────────────────────────────────
    ServiceCatalogEntry {
        id: "resend",
        name: "Resend",
        url: "https://resend.mpp.tempo.xyz",
        categories: &["email", "comms"],
        intent: "charge",
    },
    ServiceCatalogEntry {
        id: "twilio",
        name: "Twilio",
        url: "https://twilio.mpp.tempo.xyz",
        categories: &["sms", "comms"],
        intent: "charge",
    },
    ServiceCatalogEntry {
        id: "sendgrid",
        name: "SendGrid",
        url: "https://sendgrid.mpp.tempo.xyz",
        categories: &["email", "comms"],
        intent: "charge",
    },

    // ── Maps / Geo ───────────────────────────────────────────────
    ServiceCatalogEntry {
        id: "mapbox",
        name: "Mapbox",
        url: "https://mapbox.mpp.tempo.xyz",
        categories: &["maps", "geo"],
        intent: "charge",
    },

    // ── Auth / Identity ──────────────────────────────────────────
    ServiceCatalogEntry {
        id: "clerk",
        name: "Clerk",
        url: "https://clerk.mpp.tempo.xyz",
        categories: &["auth", "identity"],
        intent: "session",
    },

    // ── Monitoring / Observability ───────────────────────────────
    ServiceCatalogEntry {
        id: "sentry",
        name: "Sentry",
        url: "https://sentry.mpp.tempo.xyz",
        categories: &["monitoring", "errors"],
        intent: "charge",
    },
    ServiceCatalogEntry {
        id: "axiom",
        name: "Axiom",
        url: "https://axiom.mpp.tempo.xyz",
        categories: &["monitoring", "logs"],
        intent: "charge",
    },
    ServiceCatalogEntry {
        id: "posthog",
        name: "PostHog",
        url: "https://posthog.mpp.tempo.xyz",
        categories: &["analytics"],
        intent: "charge",
    },

    // ── Payments / Finance ───────────────────────────────────────
    ServiceCatalogEntry {
        id: "stripe",
        name: "Stripe",
        url: "https://stripe.mpp.tempo.xyz",
        categories: &["payments", "finance"],
        intent: "charge",
    },

    // ── Deployment / Hosting ─────────────────────────────────────
    ServiceCatalogEntry {
        id: "vercel",
        name: "Vercel",
        url: "https://vercel.mpp.tempo.xyz",
        categories: &["hosting", "deployment"],
        intent: "session",
    },
    ServiceCatalogEntry {
        id: "fly",
        name: "Fly.io",
        url: "https://fly.mpp.tempo.xyz",
        categories: &["hosting", "deployment"],
        intent: "session",
    },
    ServiceCatalogEntry {
        id: "railway",
        name: "Railway",
        url: "https://railway.mpp.tempo.xyz",
        categories: &["hosting", "deployment"],
        intent: "session",
    },
    ServiceCatalogEntry {
        id: "modal",
        name: "Modal",
        url: "https://modal.mpp.tempo.xyz",
        categories: &["compute", "serverless"],
        intent: "charge",
    },

    // ── Document / PDF ───────────────────────────────────────────
    ServiceCatalogEntry {
        id: "documenso",
        name: "Documenso",
        url: "https://documenso.mpp.tempo.xyz",
        categories: &["document", "signing"],
        intent: "charge",
    },
    ServiceCatalogEntry {
        id: "unstructured",
        name: "Unstructured",
        url: "https://unstructured.mpp.tempo.xyz",
        categories: &["document", "parsing"],
        intent: "charge",
    },

    // ── Vector / Embedding ───────────────────────────────────────
    ServiceCatalogEntry {
        id: "pinecone",
        name: "Pinecone",
        url: "https://pinecone.mpp.tempo.xyz",
        categories: &["vector", "database"],
        intent: "charge",
    },
    ServiceCatalogEntry {
        id: "weaviate",
        name: "Weaviate",
        url: "https://weaviate.mpp.tempo.xyz",
        categories: &["vector", "database"],
        intent: "charge",
    },
    ServiceCatalogEntry {
        id: "qdrant",
        name: "Qdrant",
        url: "https://qdrant.mpp.tempo.xyz",
        categories: &["vector", "database"],
        intent: "charge",
    },

    // ── Workflow / Integration ───────────────────────────────────
    ServiceCatalogEntry {
        id: "inngest",
        name: "Inngest",
        url: "https://inngest.mpp.tempo.xyz",
        categories: &["workflow", "queue"],
        intent: "charge",
    },
    ServiceCatalogEntry {
        id: "trigger",
        name: "Trigger.dev",
        url: "https://trigger.mpp.tempo.xyz",
        categories: &["workflow", "background"],
        intent: "charge",
    },

    // ── Security / Secrets ───────────────────────────────────────
    ServiceCatalogEntry {
        id: "infisical",
        name: "Infisical",
        url: "https://infisical.mpp.tempo.xyz",
        categories: &["secrets", "security"],
        intent: "session",
    },

    // ── CMS / Content ────────────────────────────────────────────
    ServiceCatalogEntry {
        id: "sanity",
        name: "Sanity",
        url: "https://sanity.mpp.tempo.xyz",
        categories: &["cms", "content"],
        intent: "charge",
    },
    ServiceCatalogEntry {
        id: "contentful",
        name: "Contentful",
        url: "https://contentful.mpp.tempo.xyz",
        categories: &["cms", "content"],
        intent: "charge",
    },
];
```

> **Note**: The `ServiceCatalogEntry` struct in the catalog uses `&'static str` and `&'static [&'static str]` for zero-allocation static data. The `ServiceCatalogEntry` in `mpp_discovery.rs` uses owned `String` and `Vec<String>`. The catalog version needs a separate struct definition or the discovery version needs a `From` impl. See task 21.2 note below.
>
> **Implementation note**: The catalog struct needs `&'static` lifetime fields for static data:
> ```rust
> /// Static catalog entry (borrows from static strings).
> pub struct ServiceCatalogEntry {
>     pub id: &'static str,
>     pub name: &'static str,
>     pub url: &'static str,
>     pub categories: &'static [&'static str],
>     pub intent: &'static str,
> }
> ```
> Add a `to_owned()` or `Into<mpp_discovery::ServiceCatalogEntry>` impl for the discovery client to use.

### 21.3 Wire discovery into the `chain.mpp_discover` tool handler (WU-18)

**File**: `crates/roko-cli/src/chain_handler.rs`

Update the `handle_mpp_discover()` method (added in WU-18) to use `MppDiscovery`:

```rust
async fn handle_mpp_discover(&self, args: &Value) -> ToolResult {
    let discovery = self.mpp_discovery.as_ref()
        .ok_or_else(|| ToolError::Other("MPP not configured — add [chain.mpp] to roko.toml".into()))?;
    let service_url = args["service_url"].as_str()
        .ok_or_else(|| ToolError::Other("missing 'service_url' parameter".into()))?;

    let service = discovery.discover(service_url).await
        .map_err(|e| ToolError::Other(format!("mpp_discover failed: {e}")))?;

    ToolResult::structured(serde_json::json!({
        "service_url": service.url,
        "name": service.name,
        "description": service.description,
        "endpoints": service.endpoints,
        "payment_methods": service.payment_methods,
        "discovered_at": service.discovered_at,
        "cache_ttl_secs": service.cache_ttl_secs,
    }))
}
```

Add `mpp_discovery: Option<Arc<MppDiscovery>>` to `ChainToolHandler`.

Also add a catalog search variant — if `service_url` is not provided but `query` is, search the catalog:

```rust
// In handle_mpp_discover:
if let Some(query) = args.get("query").and_then(|v| v.as_str()) {
    let results = discovery.search_catalog(query);
    return ToolResult::structured(serde_json::json!({
        "catalog_results": results,
        "count": results.len(),
    }));
}
```

### 21.4 Knowledge store integration for discovered services

**File**: `crates/roko-cli/src/chain_handler.rs` (in `handle_mpp_discover()`)

After a successful discovery, persist the result to the knowledge store:

```rust
// After successful discovery, persist as knowledge
if let Some(ref neuro) = self.knowledge_store {
    let entry = KnowledgeEntry {
        kind: "mpp_service".to_string(),
        topic: format!("mpp:service:{}", service.url),
        content: serde_json::to_string(&service).unwrap_or_default(),
        confidence: 0.90,  // high but not cryptographic
        source: format!("mpp:discovery:{}", service.url),
    };
    if let Err(e) = neuro.ingest(entry).await {
        tracing::warn!(url = %service.url, err = %e, "failed to persist discovered service to knowledge store");
    }
}
```

This makes previously discovered services queryable via `roko knowledge query "mpp:service:*"`, enabling agents to recall known services without re-fetching.

### 21.5 Tests

Tests are inline in `mpp_discovery.rs` (see task 21.1 above). Additional integration tests:

**File**: `crates/roko-chain/src/mpp_discovery.rs` (append to existing `mod tests`)

```rust
    // ── Cache behavior ───────────────────────────────────────────

    #[tokio::test]
    async fn cache_is_populated_after_discover() {
        // This test requires a mock HTTP server (e.g., wiremock).
        // Placeholder for integration test when wiremock is added.
        //
        // let mock_server = MockServer::start().await;
        // Mock::given(method("GET")).and(path("/openapi.json"))
        //     .respond_with(ResponseTemplate::new(200)
        //         .set_body_json(sample_openapi_doc()))
        //     .mount(&mock_server).await;
        //
        // let discovery = MppDiscovery::with_http_client(reqwest::Client::new());
        // let result = discovery.discover(&mock_server.uri()).await;
        // assert!(result.is_ok());
        //
        // // Second call should hit cache (no HTTP request)
        // let cached = discovery.discover(&mock_server.uri()).await;
        // assert!(cached.is_ok());
    }

    #[tokio::test]
    async fn invalidate_removes_from_cache() {
        let discovery = MppDiscovery::new();
        // Pre-populate cache manually for testing
        {
            let mut cache = discovery.cache.write().await;
            cache.insert("https://test.example.com".to_string(), (
                DiscoveredService {
                    url: "https://test.example.com".to_string(),
                    name: Some("Test".to_string()),
                    description: None,
                    endpoints: vec![],
                    payment_methods: vec![],
                    discovered_at: "2026-01-01T00:00:00Z".to_string(),
                    cache_ttl_secs: 300,
                },
                std::time::Instant::now(),
            ));
        }

        // Verify it's in cache
        {
            let cache = discovery.cache.read().await;
            assert!(cache.contains_key("https://test.example.com"));
        }

        // Invalidate
        discovery.invalidate("https://test.example.com").await;

        // Verify it's gone
        {
            let cache = discovery.cache.read().await;
            assert!(!cache.contains_key("https://test.example.com"));
        }
    }

    #[tokio::test]
    async fn clear_cache_empties_all() {
        let discovery = MppDiscovery::new();
        {
            let mut cache = discovery.cache.write().await;
            cache.insert("a".to_string(), (
                DiscoveredService {
                    url: "a".to_string(),
                    name: None, description: None,
                    endpoints: vec![], payment_methods: vec![],
                    discovered_at: String::new(), cache_ttl_secs: 300,
                },
                std::time::Instant::now(),
            ));
            cache.insert("b".to_string(), (
                DiscoveredService {
                    url: "b".to_string(),
                    name: None, description: None,
                    endpoints: vec![], payment_methods: vec![],
                    discovered_at: String::new(), cache_ttl_secs: 300,
                },
                std::time::Instant::now(),
            ));
        }

        discovery.clear_cache().await;
        let cache = discovery.cache.read().await;
        assert!(cache.is_empty());
    }
```

### 21.6 Register modules in lib.rs

**File**: `crates/roko-chain/src/lib.rs`

Add:

```rust
#[cfg(feature = "mpp")]
pub mod mpp_discovery;

#[cfg(feature = "mpp")]
pub mod mpp_catalog;

#[cfg(feature = "mpp")]
pub use mpp_discovery::{MppDiscovery, DiscoveredService, DiscoveredEndpoint, PaymentMethod, ServiceCatalogEntry};
```

---

## Verification Checklist

- [ ] `MppDiscovery` struct with `discover()`, `search_catalog()`, `list_catalog()` methods
- [ ] `DiscoveredService`, `DiscoveredEndpoint`, `PaymentMethod` types serialize/deserialize correctly
- [ ] OpenAPI parser extracts `x-payment-info` extensions from both operation-level and top-level
- [ ] Cache respects TTL from Cache-Control header (default 300s)
- [ ] `invalidate()` and `clear_cache()` work correctly
- [ ] `ServiceCatalogEntry` static catalog has ~50 entries covering major categories
- [ ] Catalog search is case-insensitive, matches against id/name/categories
- [ ] `ChainToolHandler` updated with `mpp_discovery` field
- [ ] `handle_mpp_discover()` supports both URL discovery and catalog search
- [ ] Knowledge store integration persists discovered services
- [ ] Both modules registered in `lib.rs` under `mpp` feature gate
- [ ] `cargo build -p roko-chain --features mpp`
- [ ] `cargo test -p roko-chain --features mpp`
- [ ] `cargo clippy -p roko-chain --features mpp --no-deps -- -D warnings`
- [ ] `cargo test --workspace` — no breakage

---

## Open Questions

1. **Catalog update cadence**: How often should the vendored catalog be refreshed? The upstream `tempoxyz/mpp` repo changes frequently as new services are added. A CI job could auto-vendor, or we could fetch the catalog at startup as a third discovery mechanism.

2. **Discovery caching strategy**: The current in-memory `HashMap` is simple but doesn't survive process restarts. Should discovered services be persisted to disk (e.g., `.roko/cache/mpp-discovery.json`) for faster cold starts? The knowledge store integration (21.4) partially addresses this, but the discovery client's own cache is ephemeral.

3. **OpenAPI spec version**: The parser currently handles OpenAPI 3.0.x. Should it also support OpenAPI 3.1.x (which uses JSON Schema 2020-12) or Swagger 2.0 documents? Most MPP services likely use 3.0, but edge cases may exist.

4. **Rate limiting on discovery**: Should the discovery client rate-limit how frequently it re-fetches a service's OpenAPI doc? The cache TTL handles normal cases, but a misbehaving caller could `invalidate()` + `discover()` in a tight loop.
