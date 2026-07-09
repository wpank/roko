# WU-10: Config Integration

**Layer**: 3
**Depends on**: WU-7 (VerifiedChainClient), WU-8 (ThresholdBlsVerifier)
**Blocks**: WU-13, WU-14
**Estimated effort**: 2-3 hours
**Crates**: `crates/roko-core` (config), `crates/roko-chain` (factory)

---

## Overview

Extend `ChainConfig` in `crates/roko-core/src/config/chain.rs` to support multiple chain backends with consensus configuration. Add a factory function in `crates/roko-chain` that constructs `VerifiedChainClient` instances from config.

Currently, `ChainConfig` holds a single `rpc_url` + `chain_id`. We need to support:
```toml
[chain]
rpc_url = "https://rpc.moderato.tempo.xyz"
chain_id = 4217
default_backend = "tempo-moderato"

[chain.backends.tempo-moderato]
rpc_url = "https://rpc.moderato.tempo.xyz"
chain_id = 4217
consensus = "threshold_bls"
group_pubkey = "0xaa...48bytes..."

[chain.backends.tempo-mainnet]
rpc_url = "https://rpc.tempo.xyz"
chain_id = 4217
consensus = "threshold_bls"
group_pubkey = "0xbb...48bytes..."

[chain.backends.local-dev]
rpc_url = "http://127.0.0.1:8545"
chain_id = 31337
consensus = "rpc"  # no light client, just trust the RPC
```

**Backward compatible**: Existing `[chain]` with just `rpc_url` continues to work as a single "default" backend with `consensus = "rpc"`.

---

## Pre-read

- `crates/roko-core/src/config/chain.rs` — current `ChainConfig` struct (35 lines)
- `crates/roko-core/src/config/schema.rs` — `RokoConfig` where `chain: ChainConfig` is a field
- `crates/roko-chain/src/adapter.rs` — `ChainBackendConfig`, `ChainAdapter`, `adapter_for_consensus()` (from WU-1)
- `crates/roko-chain/src/verified_client.rs` — `VerifiedChainClient` (from WU-7)
- `crates/roko-chain/src/alloy_impl.rs` — `AlloyChainClient::http(url)` constructor

---

## Tasks

### 10.1 Add `ChainBackendsConfig` to `crates/roko-core/src/config/chain.rs`

Add below the existing `ChainConfig` struct:

```rust
/// Configuration for a single chain backend with consensus settings.
///
/// ```toml
/// [chain.backends.tempo-moderato]
/// rpc_url = "https://rpc.moderato.tempo.xyz"
/// chain_id = 4217
/// consensus = "threshold_bls"
/// group_pubkey = "0xaa..."
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct ChainBackendEntry {
    /// HTTP JSON-RPC endpoint.
    #[serde(default)]
    pub rpc_url: Option<String>,
    /// EVM chain ID.
    #[serde(default)]
    pub chain_id: Option<u64>,
    /// Consensus mechanism: "rpc", "threshold_bls", "playback".
    /// Defaults to "rpc" (no light client).
    #[serde(default = "default_consensus")]
    pub consensus: String,
    /// BLS group public key (hex, 0x-prefixed). Required for threshold_bls.
    #[serde(default)]
    pub group_pubkey: Option<String>,
    /// Path to playback JSONL file. Required for playback consensus.
    #[serde(default)]
    pub playback_file: Option<String>,
    /// Human-readable label for dashboards.
    #[serde(default)]
    pub label: Option<String>,
}

fn default_consensus() -> String {
    "rpc".to_string()
}
```

### 10.2 Add `backends` and `default_backend` to `ChainConfig`

Modify the existing `ChainConfig` to include:

```rust
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct ChainConfig {
    // ... existing fields unchanged ...

    /// Named chain backends with consensus configuration.
    /// If empty, a single "default" backend is synthesized from rpc_url/chain_id.
    #[serde(default)]
    pub backends: HashMap<String, ChainBackendEntry>,

    /// Which backend to use by default when no network is specified.
    /// Falls back to the first entry in `backends`, or the legacy rpc_url.
    #[serde(default)]
    pub default_backend: Option<String>,
}
```

Add the import:
```rust
use std::collections::HashMap;
```

### 10.3 Add `resolve_backends()` method to `ChainConfig`

This method normalizes the config into a unified backend map, supporting both legacy and new formats:

```rust
impl ChainConfig {
    /// Resolve all configured backends into a normalized map.
    ///
    /// If no explicit `[chain.backends.*]` sections exist, synthesizes a
    /// "default" backend from the legacy `rpc_url` / `chain_id` fields.
    pub fn resolve_backends(&self) -> HashMap<String, ChainBackendEntry> {
        if !self.backends.is_empty() {
            return self.backends.clone();
        }

        // Legacy fallback: single backend from top-level fields
        if let Some(url) = &self.rpc_url {
            let mut map = HashMap::new();
            map.insert("default".to_string(), ChainBackendEntry {
                rpc_url: Some(url.clone()),
                chain_id: self.chain_id,
                consensus: "rpc".to_string(),
                group_pubkey: None,
                playback_file: None,
                label: Some("default".to_string()),
            });
            map
        } else {
            HashMap::new()
        }
    }

    /// Get the name of the default backend.
    pub fn default_backend_name(&self) -> Option<&str> {
        self.default_backend.as_deref().or_else(|| {
            if !self.backends.is_empty() {
                self.backends.keys().next().map(|s| s.as_str())
            } else if self.rpc_url.is_some() {
                Some("default")
            } else {
                None
            }
        })
    }
}
```

### 10.4 Create `crates/roko-chain/src/backend_factory.rs`

This module converts config into live `VerifiedChainClient` instances:

```rust
//! Backend factory — constructs [`VerifiedChainClient`] instances from configuration.
//!
//! Reads [`ChainBackendEntry`] structs (from roko.toml) and produces wired,
//! ready-to-use verified clients with the appropriate consensus verifier.

use std::collections::HashMap;
use std::sync::Arc;

use crate::adapter::{ChainBackendConfig, adapter_for_consensus};
use crate::client::ChainClient;
use crate::consensus::ConsensusVerifier;
use crate::types::ChainError;
use crate::verified_client::VerifiedChainClient;

/// A resolved, live chain backend.
pub struct LiveBackend {
    /// Human-readable name (key from config).
    pub name: String,
    /// The verified client wrapping this backend.
    pub client: Arc<VerifiedChainClient>,
    /// The raw RPC client (for non-verified access).
    pub rpc: Arc<dyn ChainClient>,
    /// The consensus verifier.
    pub verifier: Arc<dyn ConsensusVerifier>,
}

/// A pool of constructed chain backends, keyed by name.
pub struct BackendPool {
    backends: HashMap<String, LiveBackend>,
    default_name: Option<String>,
}

impl BackendPool {
    /// Get a backend by name.
    pub fn get(&self, name: &str) -> Option<&LiveBackend> {
        self.backends.get(name)
    }

    /// Get the default backend.
    pub fn default_backend(&self) -> Option<&LiveBackend> {
        self.default_name.as_deref().and_then(|n| self.backends.get(n))
    }

    /// Get the default verified client, or None if no backends configured.
    pub fn default_verified_client(&self) -> Option<Arc<VerifiedChainClient>> {
        self.default_backend().map(|b| Arc::clone(&b.client))
    }

    /// Get the default raw RPC client.
    pub fn default_rpc_client(&self) -> Option<Arc<dyn ChainClient>> {
        self.default_backend().map(|b| Arc::clone(&b.rpc))
    }

    /// Number of configured backends.
    pub fn len(&self) -> usize {
        self.backends.len()
    }

    /// Whether the pool is empty.
    pub fn is_empty(&self) -> bool {
        self.backends.is_empty()
    }

    /// Iterate over all backends.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &LiveBackend)> {
        self.backends.iter().map(|(k, v)| (k.as_str(), v))
    }
}

/// Construct a [`BackendPool`] from resolved backend entries.
///
/// `entries` is the output of `ChainConfig::resolve_backends()`.
/// `default_name` is from `ChainConfig::default_backend_name()`.
///
/// Backends that fail to construct log a warning and are skipped.
#[cfg(feature = "alloy-backend")]
pub fn build_backend_pool(
    entries: &HashMap<String, roko_core::config::chain::ChainBackendEntry>,
    default_name: Option<&str>,
) -> BackendPool {
    let mut backends = HashMap::new();

    for (name, entry) in entries {
        match build_single_backend(name, entry) {
            Ok(live) => {
                tracing::info!(
                    backend = name,
                    consensus = %live.verifier.mechanism(),
                    "chain backend initialized"
                );
                backends.insert(name.clone(), live);
            }
            Err(e) => {
                tracing::warn!(
                    backend = name,
                    error = %e,
                    "failed to initialize chain backend; skipping"
                );
            }
        }
    }

    BackendPool {
        default_name: default_name.map(|s| s.to_string()),
        backends,
    }
}

#[cfg(feature = "alloy-backend")]
fn build_single_backend(
    name: &str,
    entry: &roko_core::config::chain::ChainBackendEntry,
) -> Result<LiveBackend, ChainError> {
    let rpc_url = entry.rpc_url.as_deref()
        .ok_or_else(|| ChainError::Rpc(format!("backend '{name}': rpc_url is required")))?;
    let chain_id = entry.chain_id.unwrap_or(1);

    // Build the raw RPC client
    let rpc: Arc<dyn ChainClient> = Arc::new(
        crate::alloy_impl::AlloyChainClient::http(rpc_url)?
    );

    // Build the consensus verifier via the adapter system
    let adapter_config = ChainBackendConfig {
        name: name.to_string(),
        rpc_url: Some(rpc_url.to_string()),
        chain_id,
        consensus_type: entry.consensus.clone(),
        group_pubkey: entry.group_pubkey.clone(),
        playback_file: entry.playback_file.clone(),
    };

    let verifier = adapter_for_consensus(&entry.consensus)?
        .create_verifier(&adapter_config)?;

    // Wrap in VerifiedChainClient
    let verified = VerifiedChainClient::new(
        Arc::clone(&rpc),
        Arc::clone(&verifier),
        name,
        chain_id,
    );

    Ok(LiveBackend {
        name: name.to_string(),
        client: Arc::new(verified),
        rpc,
        verifier,
    })
}

/// Non-alloy stub: always returns empty pool.
#[cfg(not(feature = "alloy-backend"))]
pub fn build_backend_pool(
    _entries: &HashMap<String, roko_core::config::chain::ChainBackendEntry>,
    _default_name: Option<&str>,
) -> BackendPool {
    BackendPool {
        backends: HashMap::new(),
        default_name: None,
    }
}
```

### 10.5 Register module in `lib.rs`

```rust
/// Backend factory — constructs VerifiedChainClient from config.
pub mod backend_factory;

pub use backend_factory::{BackendPool, LiveBackend, build_backend_pool};
```

### 10.6 Tests

Add tests to `backend_factory.rs`:

```rust
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn empty_entries_produce_empty_pool() {
        let pool = build_backend_pool(&HashMap::new(), None);
        assert!(pool.is_empty());
        assert!(pool.default_backend().is_none());
    }

    #[cfg(feature = "alloy-backend")]
    #[test]
    fn invalid_rpc_url_skipped_gracefully() {
        use roko_core::config::chain::ChainBackendEntry;

        let mut entries = HashMap::new();
        entries.insert("bad".to_string(), ChainBackendEntry {
            rpc_url: None, // missing = error
            chain_id: Some(1),
            consensus: "rpc".to_string(),
            ..Default::default()
        });

        let pool = build_backend_pool(&entries, Some("bad"));
        assert!(pool.is_empty()); // bad backend was skipped
    }
}
```

Also add tests for `ChainConfig::resolve_backends()` in `crates/roko-core/src/config/chain.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_backends_legacy_single_rpc() {
        let config = ChainConfig {
            rpc_url: Some("http://localhost:8545".into()),
            chain_id: Some(31337),
            ..Default::default()
        };
        let backends = config.resolve_backends();
        assert_eq!(backends.len(), 1);
        let default = backends.get("default").unwrap();
        assert_eq!(default.rpc_url.as_deref(), Some("http://localhost:8545"));
        assert_eq!(default.consensus, "rpc");
    }

    #[test]
    fn resolve_backends_empty_when_no_rpc() {
        let config = ChainConfig::default();
        assert!(config.resolve_backends().is_empty());
    }

    #[test]
    fn resolve_backends_prefers_explicit_over_legacy() {
        let mut backends = HashMap::new();
        backends.insert("tempo".to_string(), ChainBackendEntry {
            rpc_url: Some("https://rpc.tempo.xyz".into()),
            chain_id: Some(4217),
            consensus: "threshold_bls".to_string(),
            group_pubkey: Some("0xaa".repeat(48)),
            ..Default::default()
        });
        let config = ChainConfig {
            rpc_url: Some("http://localhost:8545".into()),
            backends,
            ..Default::default()
        };
        // Explicit backends win — legacy rpc_url is ignored
        let resolved = config.resolve_backends();
        assert_eq!(resolved.len(), 1);
        assert!(resolved.contains_key("tempo"));
    }

    #[test]
    fn default_backend_name_uses_explicit() {
        let config = ChainConfig {
            default_backend: Some("tempo".into()),
            ..Default::default()
        };
        assert_eq!(config.default_backend_name(), Some("tempo"));
    }

    #[test]
    fn default_backend_name_falls_back_to_legacy() {
        let config = ChainConfig {
            rpc_url: Some("http://localhost:8545".into()),
            ..Default::default()
        };
        assert_eq!(config.default_backend_name(), Some("default"));
    }
}
```

---

## Verification Checklist

- [ ] `ChainBackendEntry` struct added to `crates/roko-core/src/config/chain.rs`
- [ ] `ChainConfig` has `backends: HashMap<String, ChainBackendEntry>` and `default_backend: Option<String>`
- [ ] `resolve_backends()` synthesizes a "default" entry from legacy `rpc_url`
- [ ] `default_backend_name()` returns correct fallback chain
- [ ] `BackendPool` in `crates/roko-chain/src/backend_factory.rs` constructs live backends
- [ ] `build_backend_pool()` skips bad backends with warnings (not panics)
- [ ] Non-alloy build returns empty pool (no compile error)
- [ ] Module registered in `lib.rs`
- [ ] `cargo test -p roko-core` passes (config tests)
- [ ] `cargo test -p roko-chain` passes (factory tests)
- [ ] `cargo test --workspace` — no breakage
- [ ] Existing `roko.toml` files without `[chain.backends]` still parse correctly
