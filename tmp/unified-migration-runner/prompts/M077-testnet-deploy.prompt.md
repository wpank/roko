# M077 — Deploy to Nunchi Testnet

**[BLOCKED:depth]** -- This item depends on `tmp/unified-depth/18-registries/` depth docs and M076 (finalized contracts). The depth docs specify deployment order, environment configuration, and address management.

## Objective
Deploy all finalized Solidity contracts to the Nunchi testnet via foundry scripts. Verify deployment addresses are accessible and contracts function correctly on-chain. Configure the environment with deployed addresses for Rust client integration.

## Scope
- Crates: N/A (deployment scripts)
- Files: `contracts/deploy/`, `contracts/broadcast/`
- Phase ref: `tmp/unified-migration/04-PHASE-3-ECONOMY.md` SS3.4
- Spec ref: `tmp/unified/18-ON-CHAIN-REGISTRIES.md` SS8

## Steps
1. Verify contracts compile cleanly (M076 must be complete):
   ```bash
   cd contracts && forge build
   ```

2. Write deployment scripts in `contracts/deploy/Deploy.s.sol`:
   - Deploy in dependency order: DisputeResolver first (referenced by BountyMarket), then registries
   - Record all deployed addresses

3. Deploy to testnet:
   ```bash
   forge script contracts/deploy/Deploy.s.sol --rpc-url $NUNCHI_TESTNET_RPC --broadcast
   ```

4. Verify deployments:
   ```bash
   forge verify-contract <address> ContractName --chain nunchi-testnet
   ```

5. Create an address manifest file (`.roko/chain/addresses.toml`):
   ```toml
   [testnet]
   agent_passport = "0x..."
   reputation_registry = "0x..."
   insight_store = "0x..."
   # ...
   ```

6. Smoke test: call read methods on each deployed contract to verify they respond.

## Verification
```bash
cd contracts && forge test --fork-url $NUNCHI_TESTNET_RPC
# Verify each contract is accessible:
cast call <agent_passport_address> "name()(string)" --rpc-url $NUNCHI_TESTNET_RPC
```

## What NOT to do
- Do NOT deploy to mainnet
- Do NOT proceed without finalized contracts from M076
- Do NOT hardcode private keys in scripts -- use environment variables or foundry keystore
- Do NOT skip verification -- all contracts must be verified on the explorer
