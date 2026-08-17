# E1: Add ISFR Configuration to roko.toml Schema

## Context

`ChainConfig` and `RelayConfig` **already exist** in
`crates/roko-core/src/config/chain.rs` and are already fields on `RokoConfig` in
`crates/roko-core/src/config/schema.rs`:

```rust
// These already exist at lines 127-129 of schema.rs:
pub chain: ChainConfig,
pub relay: RelayConfig,
```

**Do NOT redefine `ChainConfig` or `RelayConfig`.** This task only:
1. Adds a `profile` field to the existing `ChainConfig` struct
2. Adds a new `ISFRSection` struct + `ISFRSourceConfig` struct to `chain.rs`
3. Adds a `pub isfr: ISFRSection` field to `RokoConfig`
4. Re-exports the new types from the config module

## Files to Modify

- `crates/roko-core/src/config/chain.rs` — add `ISFRSection`, `ISFRSourceConfig`, and
  the `profile` field to the existing `ChainConfig`
- `crates/roko-core/src/config/schema.rs` — add `pub isfr: ISFRSection` to `RokoConfig`
- `crates/roko-core/src/config/mod.rs` — add `ISFRSection`, `ISFRSourceConfig` to the
  `pub use schema::{...}` re-export list

## Pre-Check

```bash
# Verify existing ChainConfig and RelayConfig
grep -n "struct ChainConfig\|struct RelayConfig" \
  /Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/chain.rs

# Verify RokoConfig already has chain and relay fields (lines ~127-129 in schema.rs)
grep -n "pub chain:\|pub relay:\|pub isfr:" \
  /Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/schema.rs

# Confirm ISFRSection does not already exist
grep -n "ISFRSection\|ISFRSourceConfig" \
  /Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/chain.rs
```

## Implementation

### Step 1: Add `profile` field to the existing `ChainConfig`

In `crates/roko-core/src/config/chain.rs`, the existing `ChainConfig` currently has:
`rpc_url`, `chain_id`, `wallet_key`, `identity_registry`, `reputation_registry`,
`validation_registry`, `agent_registry`, `bounty_market`, `deployer`.

Add `profile` as the first field:

```rust
pub struct ChainConfig {
    /// Chain profile name: "mirage" (local dev), "daeji" (testnet), or custom.
    /// Resolves into a ChainProfile at runtime via ChainProfile::from_roko_config().
    #[serde(default = "default_chain_profile")]
    pub profile: String,
    // ... all existing fields unchanged ...
}

fn default_chain_profile() -> String {
    "mirage".to_string()
}
```

### Step 2: Add `ISFRSection` and `ISFRSourceConfig` to `chain.rs`

Append after the existing `RelayConfig` and its `Default` impl:

```rust
/// [isfr] section in roko.toml — ISFR keeper configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ISFRSection {
    /// Whether ISFR features are enabled (default: false).
    pub enabled: bool,
    /// Epoch duration in seconds (default: 28800 = 8 hours).
    pub epoch_duration_secs: u64,
    /// Source poll interval in seconds (default: 10).
    pub poll_interval_secs: u64,
    /// Minimum live source readings required to publish a composite (default: 2).
    pub min_submissions: u32,
    /// Outlier rejection sigma threshold (default: 3.0).
    pub outlier_sigma: f64,
    /// Rate source definitions.
    pub sources: Vec<ISFRSourceConfig>,
}

impl Default for ISFRSection {
    fn default() -> Self {
        Self {
            enabled: false,
            epoch_duration_secs: 28_800,
            poll_interval_secs: 10,
            min_submissions: 2,
            outlier_sigma: 3.0,
            sources: Vec::new(),
        }
    }
}

/// [[isfr.sources]] entry in roko.toml.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ISFRSourceConfig {
    /// Human-readable source name (e.g. "mock-aave-v3").
    pub name: String,
    /// Source kind: "mock", "aave_v3", "compound_v3", "ethena", "eth_staking".
    pub kind: String,
    /// Composite weight (0.0–1.0, default: 0.25).
    #[serde(default = "default_isfr_weight")]
    pub weight: f64,
    /// Rate class: "lending", "structured", "funding", "staking".
    pub class: String,
    /// Base rate in bps — mock sources only (e.g. 620 = 6.20%).
    #[serde(default)]
    pub rate_bps: u64,
    /// Rate jitter in bps — mock sources only.
    #[serde(default)]
    pub jitter_bps: u64,
    /// JSON-RPC endpoint — live sources only.
    #[serde(default)]
    pub rpc_url: Option<String>,
    /// Protocol pool/contract address — live sources only.
    #[serde(default)]
    pub pool_address: Option<String>,
}

fn default_isfr_weight() -> f64 {
    0.25
}
```

### Step 3: Add `isfr` field to `RokoConfig`

In `crates/roko-core/src/config/schema.rs`, find the block containing `pub chain:` and
`pub relay:` (currently lines ~127-129) and add the `isfr` field immediately after `relay`:

```rust
#[serde(default)]
pub chain: ChainConfig,
#[serde(default)]
pub relay: RelayConfig,
// ADD THIS:
/// ISFR keeper configuration.
#[serde(default)]
pub isfr: ISFRSection,
```

The import of `ISFRSection` is already covered by the `pub use super::chain::*;` wildcard
re-export at line 24 of `schema.rs`.

### Step 4: Add to `mod.rs` public re-exports

In `crates/roko-core/src/config/mod.rs`, the `pub use schema::{...}` list currently
includes `ChainConfig` and `RelayConfig`. Add the two new types to that list:

```rust
pub use schema::{
    // ... existing entries ...
    ChainConfig,
    ISFRSection,         // ADD
    ISFRSourceConfig,    // ADD
    RelayConfig,
    // ... rest unchanged ...
};
```

### Step 5: Add PartialEq to ChainConfig if missing

The existing `ChainConfig` derives `Default` but check whether it derives `PartialEq`.
If not, add it — the roundtrip test in `mod.rs` (`toml_serialize_roundtrip_default_config`)
requires all config types to implement `PartialEq`.

```rust
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct ChainConfig { ... }
```

## Example roko.toml

```toml
[chain]
profile = "mirage"
# rpc_url = "http://localhost:8545"

[relay]
url = "ws://localhost:9011/relay/agents/ws"

[isfr]
enabled = true
poll_interval_secs = 10
epoch_duration_secs = 28800
min_submissions = 2

[[isfr.sources]]
name = "mock-aave-v3"
kind = "mock"
weight = 0.30
class = "lending"
rate_bps = 620
jitter_bps = 15

[[isfr.sources]]
name = "mock-compound-v3"
kind = "mock"
weight = 0.25
class = "lending"
rate_bps = 580
jitter_bps = 10

[[isfr.sources]]
name = "mock-ethena-susde"
kind = "mock"
weight = 0.20
class = "structured"
rate_bps = 850
jitter_bps = 25

[[isfr.sources]]
name = "mock-eth-staking"
kind = "mock"
weight = 0.25
class = "staking"
rate_bps = 350
jitter_bps = 5
```

## Verification

```bash
cargo build -p roko-core
cargo test -p roko-core
```

The existing `toml_serialize_roundtrip_default_config` test in `mod.rs` will catch any
serialization regressions. Add these tests in `crates/roko-core/src/config/chain.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_isfr_section() {
        let toml_str = r#"
enabled = true
poll_interval_secs = 5

[[sources]]
name = "test"
kind = "mock"
weight = 1.0
class = "lending"
rate_bps = 500
"#;
        let section: ISFRSection = toml::from_str(toml_str).unwrap();
        assert!(section.enabled);
        assert_eq!(section.poll_interval_secs, 5);
        assert_eq!(section.sources.len(), 1);
        assert_eq!(section.sources[0].name, "test");
    }

    #[test]
    fn defaults_when_missing() {
        let section: ISFRSection = toml::from_str("").unwrap();
        assert!(!section.enabled);
        assert_eq!(section.epoch_duration_secs, 28_800);
        assert_eq!(section.poll_interval_secs, 10);
    }

    #[test]
    fn chain_config_profile_default() {
        let config: ChainConfig = toml::from_str("").unwrap();
        assert_eq!(config.profile, "mirage");
    }
}
```

Also verify the project's own `roko.toml` still loads:

```bash
cargo test -p roko-core project_roko_toml_loads_successfully
```

## Critical Notes

### toml dep in roko-core

The tests use `toml::from_str()`. Verify `toml` is in roko-core's Cargo.toml:
```bash
grep "toml" crates/roko-core/Cargo.toml
```

### Re-export path

The task says `pub use super::chain::*;` in schema.rs handles the import. Verify:
```bash
grep -n "use super::chain\|pub use.*chain" crates/roko-core/src/config/schema.rs
```

If schema.rs uses explicit named imports (not wildcard), you'll need to add
`ISFRSection` and `ISFRSourceConfig` to the `use` statement.

### PartialEq on all sub-types

`ISFRSection` contains `Vec<ISFRSourceConfig>` and `f64` fields. `PartialEq` on
structs with `f64` works but `Eq` does not. Do NOT add `Eq` derive to these types.

### Existing roundtrip test

The `toml_serialize_roundtrip_default_config` test in `config/mod.rs` serializes +
deserializes `RokoConfig::default()`. Since `ISFRSection` has `#[serde(default)]`,
this should work with no extra changes. But run the test to confirm.

## Dependencies

- None (pure config/schema work)
