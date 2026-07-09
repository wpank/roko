# D2: ISFR Contract ABI Files and Bootstrap Function

## Context

The ISFR system depends on 5 Solidity contracts. This task:
1. Creates the `crates/roko-chain/abi/` directory with placeholder ABI JSON files
2. Creates `scripts/update-abi.sh` to regenerate ABIs from forge when available
3. Creates `crates/roko-chain/src/isfr_bootstrap.rs` with a documented deployment sequence

**Contract source**: `/Users/will/dev/nunchi/daeji/demo-ide/demo/contracts/src/`

**`ContractAddresses` type**: Already defined by task D1 in
`crates/roko-chain/src/chain_profile.rs`. Import from `crate::chain_profile`.

## Files to Create

- `crates/roko-chain/abi/ISFROracle.json` (NEW)
- `crates/roko-chain/abi/ISFRBountyPool.json` (NEW)
- `crates/roko-chain/abi/WorkerRegistry.json` (NEW)
- `crates/roko-chain/abi/RoleRegistry.json` (NEW)
- `crates/roko-chain/abi/MockERC20.json` (NEW)
- `crates/roko-chain/src/isfr_bootstrap.rs` (NEW)
- `scripts/update-abi.sh` (NEW)

## Files to Modify

- `crates/roko-chain/src/lib.rs` — add `pub mod isfr_bootstrap;`

## Pre-Check

```bash
# Check the abi directory does not exist yet.
ls /Users/will/dev/nunchi/roko/roko/crates/roko-chain/abi/ 2>/dev/null \
  && echo "EXISTS" || echo "does not exist — create it"

# Confirm ContractAddresses is defined by D1.
grep -n "struct ContractAddresses" \
  /Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/chain_profile.rs
```

## Implementation

### Step 1: Create ABI directory

```bash
mkdir -p /Users/will/dev/nunchi/roko/roko/crates/roko-chain/abi
```

### Step 2: Create `scripts/update-abi.sh`

```bash
#!/bin/bash
# Regenerate ABI files from daeji contracts source.
# Usage: ./scripts/update-abi.sh
# Run from workspace root: /Users/will/dev/nunchi/roko/roko/
set -euo pipefail

CONTRACTS_DIR="${DAEJI_CONTRACTS:-../daeji/demo-ide/demo/contracts}"
ABI_DIR="crates/roko-chain/abi"

if [ ! -d "$CONTRACTS_DIR" ]; then
    echo "Error: contracts not found at $CONTRACTS_DIR"
    echo "Set DAEJI_CONTRACTS env var or clone daeji alongside roko."
    exit 1
fi

echo "Building contracts..."
(cd "$CONTRACTS_DIR" && forge build)

echo "Extracting ABIs..."
for CONTRACT in ISFROracle ISFRBountyPool WorkerRegistry RoleRegistry MockERC20; do
    SRC="$CONTRACTS_DIR/out/${CONTRACT}.sol/${CONTRACT}.json"
    if [ -f "$SRC" ]; then
        jq '.abi' "$SRC" > "$ABI_DIR/${CONTRACT}.json"
        echo "  OK $CONTRACT"
    else
        echo "  SKIP $CONTRACT (not found: $SRC)"
    fi
done
echo "Done. ABIs in $ABI_DIR/"
```

Make it executable: `chmod +x scripts/update-abi.sh`

### Step 3: Create placeholder ABI files

Create each file with the minimal ABI needed for the bootstrap verifier and future alloy
bindings. These are replaced by running `scripts/update-abi.sh` once forge is available.

**`crates/roko-chain/abi/ISFROracle.json`**:
```json
[
  {"type":"function","name":"getCurrentRate","inputs":[],"outputs":[{"name":"","type":"uint256"}],"stateMutability":"view"},
  {"type":"function","name":"submitRate","inputs":[{"name":"compositeBps","type":"uint256"},{"name":"lendingBps","type":"uint256"},{"name":"structuredBps","type":"uint256"},{"name":"fundingBps","type":"uint256"},{"name":"stakingBps","type":"uint256"},{"name":"confidenceBps","type":"uint256"}],"outputs":[],"stateMutability":"nonpayable"},
  {"type":"function","name":"submitRateForRange","inputs":[{"name":"rangeStart","type":"uint256"},{"name":"rangeEnd","type":"uint256"},{"name":"compositeBps","type":"uint256"},{"name":"components","type":"uint256[4]"},{"name":"confidenceBps","type":"uint256"}],"outputs":[],"stateMutability":"nonpayable"},
  {"type":"function","name":"currentEpoch","inputs":[],"outputs":[{"name":"","type":"uint256"}],"stateMutability":"view"},
  {"type":"function","name":"getRateHistory","inputs":[{"name":"epoch","type":"uint256"}],"outputs":[{"name":"","type":"uint256"}],"stateMutability":"view"},
  {"type":"event","name":"RateSubmitted","inputs":[{"name":"keeper","type":"address","indexed":true},{"name":"epoch","type":"uint256","indexed":false},{"name":"compositeBps","type":"uint256","indexed":false}]},
  {"type":"event","name":"RangeClosed","inputs":[{"name":"rangeStart","type":"uint256","indexed":false},{"name":"rangeEnd","type":"uint256","indexed":false},{"name":"compositeBps","type":"uint256","indexed":false}]}
]
```

**`crates/roko-chain/abi/WorkerRegistry.json`**:
```json
[
  {"type":"function","name":"register","inputs":[{"name":"worker","type":"address"},{"name":"stake","type":"uint256"}],"outputs":[],"stateMutability":"nonpayable"},
  {"type":"function","name":"isRegistered","inputs":[{"name":"worker","type":"address"}],"outputs":[{"name":"","type":"bool"}],"stateMutability":"view"},
  {"type":"function","name":"workerCount","inputs":[],"outputs":[{"name":"","type":"uint256"}],"stateMutability":"view"},
  {"type":"event","name":"WorkerRegistered","inputs":[{"name":"worker","type":"address","indexed":true}]}
]
```

**`crates/roko-chain/abi/ISFRBountyPool.json`**:
```json
[
  {"type":"function","name":"fund","inputs":[{"name":"amount","type":"uint256"}],"outputs":[],"stateMutability":"nonpayable"},
  {"type":"function","name":"claim","inputs":[],"outputs":[],"stateMutability":"nonpayable"},
  {"type":"function","name":"balance","inputs":[],"outputs":[{"name":"","type":"uint256"}],"stateMutability":"view"},
  {"type":"event","name":"BountyClaimed","inputs":[{"name":"keeper","type":"address","indexed":true},{"name":"amount","type":"uint256","indexed":false}]}
]
```

**`crates/roko-chain/abi/RoleRegistry.json`**:
```json
[
  {"type":"function","name":"grantRole","inputs":[{"name":"role","type":"bytes32"},{"name":"account","type":"address"}],"outputs":[],"stateMutability":"nonpayable"},
  {"type":"function","name":"hasRole","inputs":[{"name":"role","type":"bytes32"},{"name":"account","type":"address"}],"outputs":[{"name":"","type":"bool"}],"stateMutability":"view"},
  {"type":"function","name":"revokeRole","inputs":[{"name":"role","type":"bytes32"},{"name":"account","type":"address"}],"outputs":[],"stateMutability":"nonpayable"}
]
```

**`crates/roko-chain/abi/MockERC20.json`**:
```json
[
  {"type":"function","name":"mint","inputs":[{"name":"to","type":"address"},{"name":"amount","type":"uint256"}],"outputs":[],"stateMutability":"nonpayable"},
  {"type":"function","name":"approve","inputs":[{"name":"spender","type":"address"},{"name":"amount","type":"uint256"}],"outputs":[{"name":"","type":"bool"}],"stateMutability":"nonpayable"},
  {"type":"function","name":"balanceOf","inputs":[{"name":"account","type":"address"}],"outputs":[{"name":"","type":"uint256"}],"stateMutability":"view"},
  {"type":"function","name":"transfer","inputs":[{"name":"to","type":"address"},{"name":"amount","type":"uint256"}],"outputs":[{"name":"","type":"bool"}],"stateMutability":"nonpayable"}
]
```

### Step 4: Create `crates/roko-chain/src/isfr_bootstrap.rs`

```rust
//! ISFR contract bootstrap — deployment sequence for dev chains.
//!
//! Used only when `ChainProfile::auto_deploy` is true (mirage profile).
//! Daeji and production chains use pre-deployed addresses from roko.toml.

use crate::chain_profile::ContractAddresses;
use tracing::info;

/// Deployment order (each contract depends on those above it):
/// 1. RoleRegistry  — no deps
/// 2. MockERC20     — no deps
/// 3. WorkerRegistry(roleRegistry)
/// 4. ISFROracle(workerRegistry, roleRegistry)
/// 5. ISFRBountyPool(isfrOracle, erc20, roleRegistry)
///
/// Post-deploy steps:
/// - Grant KEEPER_ROLE to deployer in RoleRegistry
/// - Register deployer in WorkerRegistry
/// - Mint 10,000 tokens to deployer via MockERC20
/// - Approve + fund ISFRBountyPool
///
/// Returns populated ContractAddresses on success.
pub async fn bootstrap_isfr(
    _rpc_url: &str,
    _deployer_key: &str,
) -> anyhow::Result<ContractAddresses> {
    info!("ISFR bootstrap: placeholder (alloy deployment not yet wired)");

    // TODO: Wire with alloy when chain integration is active.
    //
    // Pattern (alloy 1.x):
    //   let provider = ProviderBuilder::new()
    //       .with_recommended_fillers()
    //       .wallet(EthereumWallet::from(signer))
    //       .on_http(rpc_url.parse()?);
    //
    // Then deploy each contract in order using sol!() generated types.

    Ok(ContractAddresses::default())
}

/// Verify ISFR contracts are live at the configured addresses.
///
/// Calls `getCurrentRate()` on ISFROracle and `workerCount()` on WorkerRegistry
/// to confirm the contracts are responsive.
pub async fn verify_isfr_deployment(
    _rpc_url: &str,
    contracts: &ContractAddresses,
) -> anyhow::Result<()> {
    if contracts.isfr_oracle.is_none() {
        anyhow::bail!("ISFROracle address not configured");
    }
    if contracts.worker_registry.is_none() {
        anyhow::bail!("WorkerRegistry address not configured");
    }
    // TODO: Issue eth_call to getCurrentRate() / workerCount() once alloy is wired.
    info!("ISFR deployment verification: placeholder (contracts not yet called)");
    Ok(())
}
```

### Step 5: Add module to `crates/roko-chain/src/lib.rs`

After the `pub mod isfr;` line (alphabetical: `isfr_bootstrap` follows `isfr`):

```rust
pub mod isfr_bootstrap;
```

## Verification

```bash
cargo build -p roko-chain
ls /Users/will/dev/nunchi/roko/roko/crates/roko-chain/abi/
# Expected: 5 JSON files
```

## Cargo.toml Note

`isfr_bootstrap.rs` uses `tracing::info` and `anyhow::Result`. Verify these are in
roko-chain's Cargo.toml (D1 should have already added `anyhow`; `tracing` is needed by C1/C2):
```bash
grep -E "tracing|anyhow" crates/roko-chain/Cargo.toml
```

## Dependencies

- D1 (`ContractAddresses` type in `crates/roko-chain/src/chain_profile.rs`)
