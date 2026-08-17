//! Two-layer exact and semantic inference cache.

use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use bytes::Bytes;
use dashmap::DashMap;
use regex::Regex;
use serde::Serialize;

use crate::{CacheRegime, GatewayResult, InferenceRequest, InferenceResponse, StopReason};

const DEFAULT_L1_CAPACITY: usize = 10_000;
const DEFAULT_L2_CAPACITY: usize = 5_000;
const L2_TTL: Duration = Duration::from_hours(2);
const SIMHASH_DISTANCE: u32 = 3;

/// Exact-cache value.
#[derive(Debug, Clone)]
pub struct CachedResponse {
    /// Serialized [`InferenceResponse`].
    pub body: Bytes,
    /// Cost of the original provider request.
    pub cost_usd: f64,
    /// Model that served the original request.
    pub model: String,
    /// Insertion time.
    pub cached_at: Instant,
    /// Regime-aware entry lifetime.
    pub effective_ttl: Duration,
}

impl CachedResponse {
    /// Decode the provider-neutral response body.
    pub fn response(&self) -> GatewayResult<InferenceResponse> {
        Ok(serde_json::from_slice(&self.body)?)
    }

    fn expired(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.cached_at) >= self.effective_ttl
    }
}

/// Semantic-cache value.
#[derive(Debug, Clone)]
pub struct SimHashEntry {
    /// Serialized response.
    pub response: Bytes,
    /// Cost of the original provider request.
    pub cost_usd: f64,
    /// Model that served the original request.
    pub model: String,
    /// Insertion time.
    pub created_at: Instant,
    /// Tenant/workspace namespace.
    pub namespace: String,
}

/// Which layer served a cache hit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheLayer {
    /// Exact normalized hash.
    L1,
    /// SimHash near match.
    L2,
}

/// Cache hit plus serving layer.
#[derive(Debug, Clone)]
pub struct CacheHit {
    /// Cached value.
    pub entry: CachedResponse,
    /// Layer that matched.
    pub layer: CacheLayer,
}

/// Aggregate cache telemetry.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct CacheStats {
    /// Exact hits.
    pub l1_hits: u64,
    /// Semantic hits.
    pub l2_hits: u64,
    /// Misses after both layers.
    pub misses: u64,
    /// Responses excluded by cache policy.
    pub excluded: u64,
}

#[derive(Default)]
struct L1State {
    entries: HashMap<[u8; 32], CachedResponse>,
    recency: VecDeque<[u8; 32]>,
}

/// Concurrent gateway cache with bounded L1 and L2 storage.
pub struct InferenceCache {
    l1: Mutex<L1State>,
    l2: DashMap<u64, SimHashEntry>,
    l1_capacity: usize,
    l2_capacity: usize,
    l1_hits: AtomicU64,
    l2_hits: AtomicU64,
    misses: AtomicU64,
    excluded: AtomicU64,
}

impl Default for InferenceCache {
    fn default() -> Self {
        Self::new(DEFAULT_L1_CAPACITY, DEFAULT_L2_CAPACITY)
    }
}

impl InferenceCache {
    /// Construct bounded exact and semantic layers.
    #[must_use]
    pub fn new(l1_capacity: usize, l2_capacity: usize) -> Self {
        Self {
            l1: Mutex::new(L1State::default()),
            l2: DashMap::new(),
            l1_capacity: l1_capacity.max(1),
            l2_capacity: l2_capacity.max(1),
            l1_hits: AtomicU64::new(0),
            l2_hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            excluded: AtomicU64::new(0),
        }
    }

    /// Look up an exact or namespace-isolated semantic match.
    #[must_use]
    pub fn lookup(&self, request: &InferenceRequest) -> Option<CachedResponse> {
        self.lookup_with_layer(request).map(|hit| hit.entry)
    }

    /// Look up a response while retaining which cache layer matched.
    #[must_use]
    pub fn lookup_with_layer(&self, request: &InferenceRequest) -> Option<CacheHit> {
        let now = Instant::now();
        let exact_key = exact_key(request);
        if let Ok(mut l1) = self.l1.lock()
            && let Some(entry) = l1.entries.get(&exact_key).cloned()
        {
            if entry.expired(now) {
                l1.entries.remove(&exact_key);
                l1.recency.retain(|key| key != &exact_key);
            } else {
                touch(&mut l1.recency, exact_key);
                self.l1_hits.fetch_add(1, Ordering::Relaxed);
                return Some(CacheHit {
                    entry,
                    layer: CacheLayer::L1,
                });
            }
        }

        let fingerprint = simhash(&request.semantic_text());
        let mut best: Option<(u64, u32, SimHashEntry)> = None;
        let mut expired = Vec::new();
        for candidate in &self.l2 {
            if now.saturating_duration_since(candidate.created_at) >= L2_TTL {
                expired.push(*candidate.key());
                continue;
            }
            if candidate.namespace != request.metadata.namespace || candidate.model != request.model
            {
                continue;
            }
            let distance = hamming_distance(fingerprint, *candidate.key());
            if distance <= SIMHASH_DISTANCE
                && best
                    .as_ref()
                    .is_none_or(|(_, current, _)| distance < *current)
            {
                best = Some((*candidate.key(), distance, candidate.value().clone()));
            }
        }
        for key in expired {
            self.l2.remove(&key);
        }
        if let Some((_, _, entry)) = best {
            self.l2_hits.fetch_add(1, Ordering::Relaxed);
            return Some(CacheHit {
                entry: CachedResponse {
                    body: entry.response,
                    cost_usd: entry.cost_usd,
                    model: entry.model,
                    cached_at: entry.created_at,
                    effective_ttl: L2_TTL,
                },
                layer: CacheLayer::L2,
            });
        }
        self.misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    /// Store a successful response with zero explicit cost attribution.
    pub fn store(&self, request: &InferenceRequest, response: &InferenceResponse) {
        self.store_with_cost(request, response, 0.0);
    }

    /// Store a successful response in both layers.
    pub fn store_with_cost(
        &self,
        request: &InferenceRequest,
        response: &InferenceResponse,
        cost_usd: f64,
    ) {
        if !cacheable(response) {
            self.excluded.fetch_add(1, Ordering::Relaxed);
            return;
        }
        let Ok(body) = serde_json::to_vec(response) else {
            self.excluded.fetch_add(1, Ordering::Relaxed);
            return;
        };
        let body = Bytes::from(body);
        let now = Instant::now();
        let exact_key = exact_key(request);
        let exact = CachedResponse {
            body: body.clone(),
            cost_usd,
            model: response.model.clone(),
            cached_at: now,
            effective_ttl: ttl_for(request.metadata.regime),
        };
        if let Ok(mut l1) = self.l1.lock() {
            if l1.entries.insert(exact_key, exact).is_some() {
                l1.recency.retain(|key| key != &exact_key);
            }
            l1.recency.push_back(exact_key);
            while l1.entries.len() > self.l1_capacity {
                if let Some(oldest) = l1.recency.pop_front() {
                    l1.entries.remove(&oldest);
                }
            }
        }

        while self.l2.len() >= self.l2_capacity {
            let oldest = self
                .l2
                .iter()
                .min_by_key(|entry| entry.created_at)
                .map(|entry| *entry.key());
            if let Some(oldest) = oldest {
                self.l2.remove(&oldest);
            } else {
                break;
            }
        }
        self.l2.insert(
            simhash(&request.semantic_text()),
            SimHashEntry {
                response: body,
                cost_usd,
                model: response.model.clone(),
                created_at: now,
                namespace: request.metadata.namespace.clone(),
            },
        );
    }

    /// Current cache counters.
    #[must_use]
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            l1_hits: self.l1_hits.load(Ordering::Relaxed),
            l2_hits: self.l2_hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            excluded: self.excluded.load(Ordering::Relaxed),
        }
    }

    /// Current bounded layer sizes.
    #[must_use]
    pub fn sizes(&self) -> (usize, usize) {
        let l1 = self.l1.lock().map_or(0, |state| state.entries.len());
        (l1, self.l2.len())
    }
}

/// Regime-aware exact-cache TTL.
#[must_use]
pub const fn ttl_for(regime: CacheRegime) -> Duration {
    Duration::from_secs(match regime {
        CacheRegime::Normal => 3_600,
        CacheRegime::Calm => 7_200,
        CacheRegime::Volatile => 900,
        CacheRegime::Crisis => 300,
    })
}

/// Serialize and scrub volatile request fields before exact hashing.
#[must_use]
pub fn normalize_request(request: &InferenceRequest) -> String {
    let mut request = request.clone();
    request.metadata.session_id.clear();
    request.metadata.agent_id.clear();
    request.metadata.budget_remaining = 0;
    request.metadata.tool_calls.clear();
    request.metadata.progress_marker = None;
    if let Some(tools) = &mut request.tools {
        tools.sort_by(|left, right| left.name.cmp(&right.name));
    }
    let mut value = serde_json::to_value(request).unwrap_or(serde_json::Value::Null);
    normalize_json(&mut value);
    serde_json::to_string(&value).unwrap_or_default()
}

fn normalize_json(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::String(text) => *text = scrub_volatile(text),
        serde_json::Value::Array(values) => values.iter_mut().for_each(normalize_json),
        serde_json::Value::Object(map) => map.values_mut().for_each(normalize_json),
        serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::Number(_) => {}
    }
}

fn scrub_volatile(text: &str) -> String {
    static UUID: OnceLock<Regex> = OnceLock::new();
    static TIMESTAMP: OnceLock<Regex> = OnceLock::new();
    static CCH: OnceLock<Regex> = OnceLock::new();
    let text = UUID
        .get_or_init(|| {
            Regex::new(r"(?i)\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b")
                .expect("static UUID regex")
        })
        .replace_all(text, "[UUID]");
    let text = TIMESTAMP
        .get_or_init(|| {
            Regex::new(r"\b\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})\b")
                .expect("static timestamp regex")
        })
        .replace_all(&text, "[TIMESTAMP]");
    let text = CCH
        .get_or_init(|| Regex::new(r"(?i)\bcch=[0-9a-f]+\b").expect("static cch regex"))
        .replace_all(&text, "cch=[HASH]");

    let mut normalized = Vec::new();
    let mut in_git_status = false;
    let mut emitted_git_status = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case("[git_status]") {
            if !emitted_git_status {
                normalized.push("[GIT_STATUS]");
                emitted_git_status = true;
            }
            continue;
        }
        if trimmed.eq_ignore_ascii_case("[git status]")
            || trimmed.eq_ignore_ascii_case("--- git status ---")
            || trimmed.starts_with("On branch ")
        {
            in_git_status = true;
            if !emitted_git_status {
                normalized.push("[GIT_STATUS]");
                emitted_git_status = true;
            }
            continue;
        }
        if in_git_status {
            if trimmed.eq_ignore_ascii_case("[/git status]")
                || trimmed.eq_ignore_ascii_case("--- end git status ---")
                || trimmed.is_empty()
            {
                in_git_status = false;
            }
            continue;
        }
        if trimmed.to_ascii_lowercase().starts_with("cwd:")
            || trimmed.to_ascii_lowercase().starts_with("date:")
        {
            continue;
        }
        normalized.push(line);
    }
    normalized.join("\n")
}

fn exact_key(request: &InferenceRequest) -> [u8; 32] {
    *blake3::hash(normalize_request(request).as_bytes()).as_bytes()
}

fn cacheable(response: &InferenceResponse) -> bool {
    response.stop_reason != StopReason::ToolUse && response.usage.output_tokens >= 3
}

fn touch(recency: &mut VecDeque<[u8; 32]>, key: [u8; 32]) {
    recency.retain(|candidate| candidate != &key);
    recency.push_back(key);
}

/// Compute a deterministic 64-bit SimHash over normalized word tokens.
#[must_use]
pub fn simhash(text: &str) -> u64 {
    let mut weights = [0_i32; 64];
    for token in text
        .split(|ch: char| !ch.is_alphanumeric() && ch != '_')
        .filter(|token| !token.is_empty())
        .map(str::to_ascii_lowercase)
    {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        token.hash(&mut hasher);
        let hash = hasher.finish();
        for (bit, weight) in weights.iter_mut().enumerate() {
            if hash & (1_u64 << bit) == 0 {
                *weight -= 1;
            } else {
                *weight += 1;
            }
        }
    }
    weights
        .iter()
        .enumerate()
        .fold(0_u64, |hash, (bit, weight)| {
            if *weight > 0 {
                hash | (1_u64 << bit)
            } else {
                hash
            }
        })
}

/// Number of differing bits between two SimHash fingerprints.
#[must_use]
pub const fn hamming_distance(left: u64, right: u64) -> u32 {
    (left ^ right).count_ones()
}

#[cfg(test)]
mod tests {
    use roko_core::foundation::MessageRole;
    use roko_core::tool::{ToolCategory, ToolDef, ToolPermission};

    use super::*;
    use crate::{InferenceMeta, Message, TokenUsage};

    fn request(text: &str) -> InferenceRequest {
        InferenceRequest {
            model: "claude-sonnet-4-6".into(),
            messages: vec![Message {
                role: MessageRole::User,
                content: text.into(),
            }],
            metadata: InferenceMeta {
                session_id: "s".into(),
                agent_id: "a".into(),
                budget_remaining: 10_000,
                ..InferenceMeta::default()
            },
            ..InferenceRequest::default()
        }
    }

    fn response(tokens: u64, stop_reason: StopReason) -> InferenceResponse {
        InferenceResponse {
            text: "cached response".into(),
            stop_reason,
            usage: TokenUsage {
                output_tokens: tokens,
                ..TokenUsage::default()
            },
            model: "claude-sonnet-4-6".into(),
            ..InferenceResponse::default()
        }
    }

    #[test]
    fn cache_normalization_strips_volatile_fields_and_sorts_tools() {
        let volatile = "Date: 2026-08-16\nCWD: /tmp/a\nid=550e8400-e29b-41d4-a716-446655440000 at 2026-08-16T12:30:00Z cch=deadbeef\n--- git status ---\nOn branch main\n M file.rs\n--- end git status ---\nwork";
        let stable = "id=[UUID] at [TIMESTAMP] cch=[HASH]\n[GIT_STATUS]\nwork";
        let mut left = request(volatile);
        left.tools = Some(vec![
            ToolDef::new("z", "z", ToolCategory::Read, ToolPermission::read_only()),
            ToolDef::new("a", "a", ToolCategory::Read, ToolPermission::read_only()),
        ]);
        let mut right = request(stable);
        right.tools = Some(vec![
            ToolDef::new("a", "a", ToolCategory::Read, ToolPermission::read_only()),
            ToolDef::new("z", "z", ToolCategory::Read, ToolPermission::read_only()),
        ]);
        assert_eq!(normalize_request(&left), normalize_request(&right));
    }

    #[test]
    fn cache_l1_ttl_and_exclusion_policy_are_enforced() {
        assert_eq!(ttl_for(CacheRegime::Normal), Duration::from_secs(3_600));
        assert_eq!(ttl_for(CacheRegime::Crisis), Duration::from_secs(300));
        let cache = InferenceCache::new(2, 2);
        let req = request("hello world");
        cache.store(&req, &response(2, StopReason::EndTurn));
        cache.store(&req, &response(10, StopReason::ToolUse));
        assert!(cache.lookup(&req).is_none());
        assert_eq!(cache.stats().excluded, 2);
    }

    #[test]
    fn cache_l1_then_l2_and_namespace_isolation() {
        let cache = InferenceCache::new(2, 2);
        let original = request("same semantic request");
        cache.store(&original, &response(10, StopReason::EndTurn));
        assert_eq!(
            cache.lookup_with_layer(&original).unwrap().layer,
            CacheLayer::L1
        );

        let mut near = original.clone();
        near.temperature = Some(0.2);
        assert_eq!(
            cache.lookup_with_layer(&near).unwrap().layer,
            CacheLayer::L2
        );

        near.metadata.namespace = "other-tenant".into();
        assert!(cache.lookup(&near).is_none());
    }

    #[test]
    fn cache_layers_evict_at_hard_capacity() {
        let cache = InferenceCache::new(2, 2);
        for index in 0..10 {
            let req = request(&format!("request {index}"));
            cache.store(&req, &response(10, StopReason::EndTurn));
        }
        assert_eq!(cache.sizes(), (2, 2));
    }
}
