# D1: Implement ChainProfile Abstraction

## Context

ChainProfile is a runtime object that normalizes chain configuration into a single struct with
helper methods. It resolves the existing `ChainConfig` from roko.toml into concrete values
with profile-aware defaults. Switch profile = switch chain.

**Key insight**: `ChainConfig` already exists in `crates/roko-core/src/config/chain.rs` with
fields `rpc_url`, `chain_id`, `wallet_key`, and various registry addresses. ChainProfile is a
RUNTIME resolution layer on top of that config — it does NOT duplicate or replace `ChainConfig`.
The `profile` field (see E1) is added to `ChainConfig` by task E1.

**Do NOT define a second `ChainConfig` struct in this file.** That type lives in roko-core.

## Files to Create

- `crates/roko-chain/src/chain_profile.rs` (NEW)

## Files to Modify

- `crates/roko-chain/src/lib.rs` — add `pub mod chain_profile;`

## Pre-Check

```bash
# Verify the module does not already exist.
ls /Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/chain_profile.rs 2>/dev/null \
  && echo "EXISTS — skip creation"

# Verify ChainConfig in roko-core (already has rpc_url, chain_id, wallet_key, etc.)
grep -n "struct ChainConfig" \
  /Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/chain.rs
```

## Implementation

### Step 1: Create `crates/roko-chain/src/chain_profile.rs`

```rust
//! Chain profile — resolved runtime configuration for chain interactions.
//!
//! Resolves ChainConfig (from roko.toml) into concrete values with
//! profile-aware defaults. Switch profile = switch chain.
//!
//! This module does NOT redefine ChainConfig — that lives in roko-core.
//! ChainProfile is the resolved form used at runtime.

use serde::{Deserialize, Serialize};

/// Deployed contract addresses (resolved at runtime).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContractAddresses {
    pub role_registry: Option<String>,
    pub worker_registry: Option<String>,
    pub isfr_oracle: Option<String>,
    pub bounty_pool: Option<String>,
    pub bounty_token: Option<String>,
    pub agent_registry: Option<String>,
    pub job_market: Option<String>,
}

/// Resolved chain profile — concrete values ready for use.
///
/// Constructed via `ChainProfile::mirage()`, `ChainProfile::daeji()`, or
/// `ChainProfile::from_roko_config()` using the `[chain]` section of roko.toml.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainProfile {
    /// Profile name: "mirage", "daeji", or custom.
    pub name: String,
    /// Chain ID as a string (used for relay topic naming: `chain:{chain_id}`).
    pub chain_id: String,
    /// WebSocket RPC URL.
    pub rpc_ws_url: String,
    /// HTTP RPC URL (optional; derived from WS URL when absent).
    pub rpc_http_url: Option<String>,
    /// Whether to auto-deploy contracts on startup (dev chains only).
    pub auto_deploy: bool,
    /// Known contract addresses (populated after deployment or from config).
    pub contracts: ContractAddresses,
}

impl ChainProfile {
    /// Local development profile (mirage: anvil at localhost:8545).
    pub fn mirage() -> Self {
        Self {
            name: "mirage".to_string(),
            chain_id: "31337".to_string(),
            rpc_ws_url: "ws://localhost:8545".to_string(),
            rpc_http_url: Some("http://localhost:8545".to_string()),
            auto_deploy: true,
            contracts: ContractAddresses::default(),
        }
    }

    /// Daeji testnet profile (pre-deployed contracts, no auto-deploy).
    pub fn daeji(rpc_ws_url: &str) -> Self {
        Self {
            name: "daeji".to_string(),
            chain_id: "8004".to_string(),
            rpc_ws_url: rpc_ws_url.to_string(),
            rpc_http_url: None,
            auto_deploy: false,
            contracts: ContractAddresses::default(),
        }
    }

    /// Resolve a profile from the fields in roko.toml `[chain]`.
    ///
    /// `profile_name` comes from `ChainConfig::profile` (added by task E1).
    /// `rpc_url` is the existing `ChainConfig::rpc_url` field.
    /// `chain_id` is the existing `ChainConfig::chain_id` field.
    pub fn from_roko_config(
        profile_name: &str,
        rpc_url: Option<&str>,
        chain_id: Option<u64>,
    ) -> Self {
        match profile_name {
            "mirage" => {
                let mut p = Self::mirage();
                if let Some(url) = rpc_url {
                    p.rpc_ws_url = url.to_string();
                    p.rpc_http_url = Some(
                        url.replace("ws://", "http://")
                            .replace("wss://", "https://"),
                    );
                }
                if let Some(id) = chain_id {
                    p.chain_id = id.to_string();
                }
                p
            }
            "daeji" => {
                let rpc = rpc_url.unwrap_or("wss://rpc.daeji.dev/ws");
                let mut p = Self::daeji(rpc);
                if let Some(id) = chain_id {
                    p.chain_id = id.to_string();
                }
                p
            }
            _ => Self {
                name: profile_name.to_string(),
                chain_id: chain_id
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| "1".to_string()),
                rpc_ws_url: rpc_url.unwrap_or("ws://localhost:8545").to_string(),
                rpc_http_url: rpc_url.map(|u| {
                    u.replace("ws://", "http://")
                        .replace("wss://", "https://")
                }),
                auto_deploy: false,
                contracts: ContractAddresses::default(),
            },
        }
    }

    /// The relay pub/sub topic for chain events: `chain:{chain_id}`.
    pub fn chain_topic(&self) -> String {
        format!("chain:{}", self.chain_id)
    }

    /// HTTP RPC URL, falling back to the WS URL when no explicit HTTP URL is set.
    pub fn http_rpc(&self) -> &str {
        self.rpc_http_url.as_deref().unwrap_or(&self.rpc_ws_url)
    }

    /// Persist profile to disk (for address discovery after deploy).
    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        std::fs::write(path, json)
    }

    /// Load profile from disk.
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let data = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&data)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mirage_defaults() {
        let p = ChainProfile::mirage();
        assert_eq!(p.chain_id, "31337");
        assert!(p.auto_deploy);
        assert_eq!(p.chain_topic(), "chain:31337");
    }

    #[test]
    fn daeji_defaults() {
        let p = ChainProfile::daeji("wss://rpc.daeji.dev/ws");
        assert_eq!(p.chain_id, "8004");
        assert!(!p.auto_deploy);
        assert_eq!(p.chain_topic(), "chain:8004");
    }

    #[test]
    fn from_roko_config_mirage_override() {
        let p = ChainProfile::from_roko_config("mirage", Some("ws://custom:8545"), None);
        assert_eq!(p.rpc_ws_url, "ws://custom:8545");
        assert_eq!(p.rpc_http_url.unwrap(), "http://custom:8545");
    }

    #[test]
    fn from_roko_config_custom_profile() {
        let p = ChainProfile::from_roko_config("mainnet", Some("wss://eth.example.com"), Some(1));
        assert_eq!(p.name, "mainnet");
        assert_eq!(p.chain_id, "1");
        assert_eq!(p.chain_topic(), "chain:1");
        assert!(!p.auto_deploy);
    }

    #[test]
    fn http_rpc_falls_back_to_ws() {
        let mut p = ChainProfile::mirage();
        p.rpc_http_url = None;
        assert_eq!(p.http_rpc(), p.rpc_ws_url.as_str());
    }
}
```

### Step 2: Add module to `crates/roko-chain/src/lib.rs`

The `lib.rs` file has an alphabetically-ordered `pub mod` block. Insert `chain_profile` near
`client` (both begin with `c`). The existing modules starting with `c` are:
- `pub mod client;`
- `pub mod collusion;`

Insert immediately before `pub mod client;`:

```rust
pub mod chain_profile;
```

## Integration Points

- **Relay chain watcher** (A4): `profile.rpc_ws_url` + `profile.chain_topic()`
- **ISFRFeed** (B1): subscribes to `profile.chain_topic()`
- **ISFRKeeper** (C2): `ISFRKeeperConfig::chain_id` set from `profile.chain_id`
- **Bootstrap** (D2): `profile.auto_deploy` controls whether to deploy
- **CLI isfr start** (E2): constructs profile via `ChainProfile::from_roko_config()`

## Cargo.toml Note

The `load()` method uses `anyhow::Result`. Verify `anyhow` is in roko-chain's Cargo.toml:
```bash
grep "anyhow" crates/roko-chain/Cargo.toml
```
If missing, add it (workspace syntax): `anyhow = { workspace = true }`

## Verification

```bash
cargo build -p roko-chain
cargo test -p roko-chain chain_profile
```

Expected: all 5 tests pass, no new warnings.

## Dependencies

- `serde`, `serde_json`, `anyhow` in roko-chain's Cargo.toml (verify present)
