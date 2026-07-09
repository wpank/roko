# 05: Contract Deployment — Chain Profiles, Bootstrapping, ABI Generation

Chain-agnostic deployment of ISFR contracts. Everything is parameterized by `ChainProfile` — mirage-rs is the default dev profile, but daeji, a mainnet fork, or any EVM chain with ERC-8004/8183 contracts works by swapping the profile.

## Canonical Contracts (ABI Source)

All from `demo-ide/demo/contracts/src/`:

| Contract | Lines | What |
|----------|-------|------|
| `ISFROracle.sol` | 550 | v3.0 two-level four-class oracle, block-range voting, trust-weighted median, 256-epoch ring buffer, probation exclusion |
| `ISFRBountyPool.sol` | 182 | Per-range reward distribution, pro-rata by trust weight, async claiming |
| `WorkerRegistry.sol` | 238 | EMA reputation, 4 tiers (MIN_BOND=1000 ether), registration, probation |
| `RoleRegistry.sol` | 83 | RBAC — ADMIN, KEEPER, ORACLE roles, grant/revoke |
| `MockERC20.sol` | ~50 | Test token for bounty pool funding |

### Deployment Order (dependencies)

```
1. RoleRegistry         — no dependencies
2. MockERC20            — no dependencies (dev chains only)
3. WorkerRegistry       — depends on: RoleRegistry
4. ISFROracle           — depends on: WorkerRegistry, RoleRegistry
5. ISFRBountyPool       — depends on: ISFROracle, ERC20 token, RoleRegistry
```

## ChainProfile Abstraction

The core abstraction that makes everything chain-agnostic. A profile describes how to connect to a chain and where the contracts live.

**File:** `crates/roko-chain/src/chain_profile.rs` (new)

```rust
use alloy::primitives::Address;
use serde::{Deserialize, Serialize};

/// A chain deployment profile — all chain-specific configuration in one place.
///
/// Profiles can be:
/// - **Dev profiles** (mirage-rs): auto-deploy contracts, mock tokens, local RPC
/// - **Testnet profiles** (daeji devnet): pre-deployed contracts, testnet RPC
/// - **Production profiles** (daeji mainnet, Ethereum): known addresses, production RPC
///
/// # Usage
///
/// ```toml
/// # roko.toml — use the built-in mirage profile
/// [chain]
/// profile = "mirage"
///
/// # roko.toml — point to daeji
/// [chain]
/// profile = "custom"
/// chain_id = "daeji"
/// rpc_url = "ws://kora.nunchi.dev:8545"
///
/// [chain.contracts]
/// isfr_oracle = "0x..."
/// bounty_pool = "0x..."
/// worker_registry = "0x..."
/// role_registry = "0x..."
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainProfile {
    /// Human-readable profile name.
    pub name: String,
    /// Chain identifier — used for relay topic naming (`chain:{chain_id}`).
    pub chain_id: String,
    /// WebSocket RPC endpoint.
    pub rpc_url: String,
    /// HTTP RPC endpoint (for non-streaming calls). Optional.
    pub http_rpc_url: Option<String>,
    /// Whether this is a dev chain that auto-deploys contracts on startup.
    pub auto_deploy: bool,
    /// Known contract addresses. Empty if auto_deploy is true (filled at runtime).
    pub contracts: ContractAddresses,
    /// ERC-20 token used for bounty pool. On dev chains, this is a MockERC20.
    pub bounty_token: Option<Address>,
}

/// Contract addresses for ISFR + supporting contracts.
///
/// On dev chains (auto_deploy=true), these are populated at runtime after bootstrap.
/// On pre-deployed chains (daeji, mainnet), these are set in config.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContractAddresses {
    pub role_registry: Option<Address>,
    pub worker_registry: Option<Address>,
    pub isfr_oracle: Option<Address>,
    pub bounty_pool: Option<Address>,
    /// ERC-8004 agent registry (if present on this chain).
    pub agent_registry: Option<Address>,
    /// ERC-8183 job market (if present on this chain).
    pub job_market: Option<Address>,
}

impl ContractAddresses {
    /// Check that all required ISFR contracts are configured.
    pub fn validate_isfr(&self) -> Result<(), Vec<&str>> {
        let mut missing = Vec::new();
        if self.role_registry.is_none() { missing.push("role_registry"); }
        if self.worker_registry.is_none() { missing.push("worker_registry"); }
        if self.isfr_oracle.is_none() { missing.push("isfr_oracle"); }
        if self.bounty_pool.is_none() { missing.push("bounty_pool"); }
        if missing.is_empty() { Ok(()) } else { Err(missing) }
    }
}

impl ChainProfile {
    /// Built-in mirage-rs dev profile.
    pub fn mirage() -> Self {
        Self {
            name: "mirage".into(),
            chain_id: "mirage".into(),
            rpc_url: "ws://localhost:8545".into(),
            http_rpc_url: Some("http://localhost:8545".into()),
            auto_deploy: true,
            contracts: ContractAddresses::default(),
            bounty_token: None,
        }
    }

    /// Built-in daeji profile (requires pre-deployed contracts).
    pub fn daeji(rpc_url: &str) -> Self {
        Self {
            name: "daeji".into(),
            chain_id: "daeji".into(),
            rpc_url: rpc_url.into(),
            http_rpc_url: None,
            auto_deploy: false,
            contracts: ContractAddresses::default(), // must be filled from config
            bounty_token: None,
        }
    }

    /// The relay chain topic for this profile: `chain:{chain_id}`.
    pub fn chain_topic(&self) -> String {
        format!("chain:{}", self.chain_id)
    }

    /// Resolve a profile from config. Built-in names are expanded;
    /// custom profiles pass through with their explicit fields.
    pub fn from_config(config: &ChainConfig) -> Self {
        match config.profile.as_str() {
            "mirage" => {
                let mut p = Self::mirage();
                if let Some(url) = &config.rpc_url { p.rpc_url = url.clone(); }
                if let Some(contracts) = &config.contracts { p.contracts = contracts.clone(); }
                p
            }
            "daeji" => {
                let url = config.rpc_url.as_deref().unwrap_or("ws://localhost:26658");
                let mut p = Self::daeji(url);
                if let Some(contracts) = &config.contracts { p.contracts = contracts.clone(); }
                p
            }
            _ => Self {
                name: config.profile.clone(),
                chain_id: config.chain_id.clone().unwrap_or_else(|| config.profile.clone()),
                rpc_url: config.rpc_url.clone().unwrap_or_default(),
                http_rpc_url: config.http_rpc_url.clone(),
                auto_deploy: config.auto_deploy.unwrap_or(false),
                contracts: config.contracts.clone().unwrap_or_default(),
                bounty_token: None,
            },
        }
    }
}

/// The `[chain]` section of roko.toml.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    /// Profile name: "mirage", "daeji", or a custom name.
    pub profile: String,
    /// Override chain_id (defaults to profile name).
    pub chain_id: Option<String>,
    /// Override RPC URL.
    pub rpc_url: Option<String>,
    /// Override HTTP RPC URL.
    pub http_rpc_url: Option<String>,
    /// Whether to auto-deploy contracts (dev chains only).
    pub auto_deploy: Option<bool>,
    /// Pre-configured contract addresses.
    pub contracts: Option<ContractAddresses>,
}
```

### How Everything Uses ChainProfile

Every chain-touching component receives a `ChainProfile` instead of raw URLs/addresses:

| Component | Uses | From ChainProfile |
|-----------|------|--------------------|
| Chain watcher (relay `chain.rs`) | `rpc_url`, `chain_id` (topic) | `profile.rpc_url`, `profile.chain_topic()` |
| ISFRFeed | `chain_id` (topic subscription) | `profile.chain_topic()` |
| ISFRKeeper | `chain_id` (topic), `contracts.isfr_oracle` (submission) | `profile.chain_topic()`, `profile.contracts` |
| ISFRToolHandler | `rpc_url`, `contracts.*` (all reads/writes) | `profile.rpc_url`, `profile.contracts` |
| Mirage bootstrap | `auto_deploy` flag, populates `contracts` | Only when `profile.auto_deploy == true` |

## Bootstrap (Dev Chains Only)

Bootstrap deploys contracts and populates the profile's addresses. Only runs when `auto_deploy == true`.

**File:** `crates/roko-chain/src/isfr_bootstrap.rs` (new — shared, not mirage-specific)

```rust
/// Deploy ISFR contracts to a dev chain and return populated addresses.
///
/// This function is chain-agnostic — it works with mirage-rs, anvil, hardhat,
/// or any EVM chain where the deployer has sufficient funds.
pub async fn bootstrap_isfr(
    provider: &impl Provider,
    deployer: &Wallet,
) -> anyhow::Result<ContractAddresses> {
    // 1. Deploy RoleRegistry
    let role_registry = deploy_contract(provider, deployer, ROLE_REGISTRY_BYTECODE, &[]).await?;
    tracing::info!(address = %role_registry, "RoleRegistry deployed");

    // 2. Deploy MockERC20 (dev chains use mock tokens)
    let mock_erc20 = deploy_contract(
        provider, deployer,
        MOCK_ERC20_BYTECODE,
        &[Token::String("Test USDC".into()), Token::String("USDC".into())],
    ).await?;
    tracing::info!(address = %mock_erc20, "MockERC20 deployed");

    // 3. Deploy WorkerRegistry
    let worker_registry = deploy_contract(
        provider, deployer,
        WORKER_REGISTRY_BYTECODE,
        &[Token::Address(role_registry)],
    ).await?;
    tracing::info!(address = %worker_registry, "WorkerRegistry deployed");

    // 4. Deploy ISFROracle
    let isfr_oracle = deploy_contract(
        provider, deployer,
        ISFR_ORACLE_BYTECODE,
        &[
            Token::Address(worker_registry),
            Token::Address(role_registry),
        ],
    ).await?;
    tracing::info!(address = %isfr_oracle, "ISFROracle deployed");

    // 5. Deploy ISFRBountyPool
    let bounty_pool = deploy_contract(
        provider, deployer,
        ISFR_BOUNTY_POOL_BYTECODE,
        &[
            Token::Address(isfr_oracle),
            Token::Address(mock_erc20),
            Token::Address(role_registry),
        ],
    ).await?;
    tracing::info!(address = %bounty_pool, "ISFRBountyPool deployed");

    // 6. Grant roles
    grant_role(provider, deployer, &role_registry, "KEEPER_ROLE", deployer.address()).await?;
    grant_role(provider, deployer, &role_registry, "ORACLE_ROLE", isfr_oracle).await?;

    // 7. Fund bounty pool
    let fund_amount = parse_units("10000", 18)?;
    approve(provider, deployer, &mock_erc20, bounty_pool, fund_amount).await?;
    deposit_bounty(provider, deployer, &bounty_pool, fund_amount).await?;
    tracing::info!("ISFRBountyPool funded with 10,000 USDC");

    // 8. Register initial keeper
    register_worker(provider, deployer, &worker_registry).await?;
    tracing::info!("Deployer registered as initial keeper");

    Ok(ContractAddresses {
        role_registry: Some(role_registry),
        worker_registry: Some(worker_registry),
        isfr_oracle: Some(isfr_oracle),
        bounty_pool: Some(bounty_pool),
        agent_registry: None,
        job_market: None,
    })
}
```

### Mirage-rs Calls Bootstrap

In mirage's startup, it calls the shared bootstrap function:

```rust
// apps/mirage-rs/src/main.rs
use roko_chain::isfr_bootstrap::bootstrap_isfr;
use roko_chain::chain_profile::ChainProfile;

let mut profile = ChainProfile::mirage();

if config.bootstrap.isfr {
    let addresses = bootstrap_isfr(&provider, &deployer).await?;
    profile.contracts = addresses;

    // Write populated profile to disk for other services
    let profile_path = data_dir.join("chain-profile.json");
    serde_json::to_writer_pretty(File::create(&profile_path)?, &profile)?;
    tracing::info!(path = %profile_path.display(), "Chain profile written");
}
```

### Daeji Uses Pre-Deployed Addresses

No bootstrap needed — addresses come from config:

```toml
# roko.toml — pointing to daeji
[chain]
profile = "daeji"
rpc_url = "ws://kora.nunchi.dev:8545"

[chain.contracts]
role_registry = "0x1234..."
worker_registry = "0x2345..."
isfr_oracle = "0x3456..."
bounty_pool = "0x4567..."
agent_registry = "0x5678..."
job_market = "0x6789..."
```

## ABI Generation

ABIs are checked in to avoid cross-repo build dependencies. They're the same regardless of which chain the contracts are deployed to.

### Checked-in ABI files

```
crates/roko-chain/abi/
├── ISFROracle.json
├── ISFRBountyPool.json
├── WorkerRegistry.json
├── RoleRegistry.json
└── MockERC20.json
```

### alloy `sol!` bindings

```rust
// crates/roko-chain/src/contracts/isfr_oracle.rs
alloy::sol! {
    #[sol(rpc)]
    ISFROracle,
    "abi/ISFROracle.json"
}
```

### Build Script

```bash
#!/bin/bash
# scripts/generate-isfr-abi.sh
# Run from repo root to regenerate ABIs from demo-ide

DEMO_IDE="../demo-ide/demo/contracts"
ABI_DIR="crates/roko-chain/abi"

cd "$DEMO_IDE" && forge build

for CONTRACT in ISFROracle ISFRBountyPool WorkerRegistry RoleRegistry MockERC20; do
    cp "out/${CONTRACT}.sol/${CONTRACT}.json" "../../roko/$ABI_DIR/${CONTRACT}.json"
done

echo "ABIs generated at $ABI_DIR/"
```

## Address Discovery

Two mechanisms, both feeding into `ChainProfile`:

### 1. File-based (dev chains)

Bootstrap writes a `chain-profile.json` that other services read:

```rust
// The relay, roko serve, or any service that needs contract addresses:
let profile: ChainProfile = if Path::new(&profile_path).exists() {
    serde_json::from_reader(File::open(&profile_path)?)?
} else {
    ChainProfile::from_config(&config.chain)
};
```

### 2. Config-based (pre-deployed chains)

Addresses come from `roko.toml` `[chain.contracts]` section. No file discovery needed.

### 3. On-chain discovery (future)

For chains with an ERC-8004 registry, contract addresses could be discoverable on-chain via a well-known registry contract. Not implemented yet, but `ChainProfile` has the extension point:

```rust
impl ChainProfile {
    /// Attempt to discover contract addresses from an on-chain registry.
    pub async fn discover_contracts(&mut self, provider: &impl Provider) -> anyhow::Result<()> {
        if let Some(registry) = self.contracts.agent_registry {
            // Query the registry for known ISFR contract addresses
            // This is a future extension point
        }
        Ok(())
    }
}
```

## Verification

Chain-agnostic verification function — works against any profile:

```rust
async fn verify_isfr_deployment(
    provider: &impl Provider,
    profile: &ChainProfile,
) -> anyhow::Result<()> {
    let contracts = &profile.contracts;
    let oracle = contracts.isfr_oracle.ok_or_else(|| anyhow!("no ISFROracle address"))?;
    let registry = contracts.worker_registry.ok_or_else(|| anyhow!("no WorkerRegistry address"))?;

    // 1. Check ISFROracle is responding
    let rate = ISFROracle::new(oracle, provider).getCurrentRate().call().await;
    tracing::info!(result = ?rate, "ISFROracle.getCurrentRate()");

    // 2. Check WorkerRegistry is responding
    let worker_count = WorkerRegistry::new(registry, provider).workerCount().call().await;
    tracing::info!(count = ?worker_count, "WorkerRegistry.workerCount()");

    tracing::info!(
        chain_id = %profile.chain_id,
        oracle = %oracle,
        "ISFR deployment verified on {}",
        profile.name,
    );
    Ok(())
}
```

## Example Profiles

### Mirage (default dev)

```toml
[chain]
profile = "mirage"
# Everything else auto-configured: rpc_url = ws://localhost:8545, auto_deploy = true
```

### Daeji devnet

```toml
[chain]
profile = "daeji"
rpc_url = "ws://kora-devnet.nunchi.dev:8545"

[chain.contracts]
isfr_oracle = "0x..."
bounty_pool = "0x..."
worker_registry = "0x..."
role_registry = "0x..."
```

### Daeji mainnet

```toml
[chain]
profile = "daeji"
chain_id = "daeji-mainnet"
rpc_url = "wss://kora.nunchi.dev:8546"

[chain.contracts]
isfr_oracle = "0x..."
bounty_pool = "0x..."
worker_registry = "0x..."
role_registry = "0x..."
agent_registry = "0x..."
job_market = "0x..."
```

### Ethereum mainnet fork (anvil)

```toml
[chain]
profile = "custom"
chain_id = "eth-fork"
rpc_url = "ws://localhost:8545"
auto_deploy = true
# Contracts auto-deployed, real DeFi sources available via fork
```

## File Summary

| File | Action | Lines |
|------|--------|-------|
| `crates/roko-chain/src/chain_profile.rs` | NEW — ChainProfile, ContractAddresses, ChainConfig | ~150 |
| `crates/roko-chain/src/isfr_bootstrap.rs` | NEW — Chain-agnostic ISFR contract deployment | ~120 |
| `apps/mirage-rs/src/main.rs` | Wire bootstrap + profile writing | ~20 |
| `crates/roko-chain/abi/*.json` | NEW — Checked-in ABI files (5 contracts) | generated |
| `crates/roko-chain/src/contracts/isfr_oracle.rs` | NEW — alloy sol! bindings | ~20 |
| `crates/roko-chain/src/contracts/mod.rs` | NEW — module registration | ~10 |
| `scripts/generate-isfr-abi.sh` | NEW — ABI generation from demo-ide | ~15 |

## Testing

```bash
# Unit tests for ChainProfile resolution
cargo test -p roko-chain -- chain_profile

# Integration test: bootstrap on mirage
cargo test -p mirage-rs -- isfr_bootstrap --ignored

# Integration test: bootstrap on anvil fork
cargo test -p roko-chain -- isfr_bootstrap_anvil --ignored

# Verify ABIs are valid
cargo test -p roko-chain -- isfr_abi_parse

# End-to-end: deploy + submit rate + read back (any dev chain)
cargo test -p roko-chain -- isfr_e2e --ignored
```
