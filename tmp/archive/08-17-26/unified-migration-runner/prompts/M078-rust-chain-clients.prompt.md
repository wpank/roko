# M078 — Rust Clients for All Registries

**[BLOCKED:depth]** -- This item depends on `tmp/unified-depth/18-registries/` depth docs and M076/M077 (deployed contracts). Alloy-based clients require contract ABIs from compiled contracts.

## Objective
Implement typed Rust clients for each on-chain registry contract using Alloy. Each client mirrors the Solidity interface with strongly-typed methods. Clients handle RPC communication, transaction submission, event subscription, and error mapping. This enables the Rust runtime to interact with on-chain state.

## Scope
- Crates: `roko-chain`
- Files: `crates/roko-chain/src/clients/` (new directory with per-contract modules)
- Phase ref: `tmp/unified-migration/04-PHASE-3-ECONOMY.md` SS3.4
- Spec ref: `tmp/unified/18-ON-CHAIN-REGISTRIES.md`

## Steps
1. Check existing chain client code:
   ```bash
   grep -rn 'alloy\|Alloy\|contract\|Contract\|provider\|Provider' crates/roko-chain/src/ --include='*.rs' | head -15
   ls crates/roko-chain/src/ 2>/dev/null
   ```

2. Generate Alloy bindings from compiled contract ABIs:
   ```rust
   sol! {
       #[sol(rpc)]
       AgentPassport,
       "contracts/out/AgentPassport.sol/AgentPassport.json"
   }
   ```

3. Implement typed client wrappers for each contract:
   ```rust
   pub struct PassportClient { provider: Arc<Provider>, address: Address }
   pub struct ReputationClient { .. }
   pub struct InsightStoreClient { .. }
   pub struct PheromoneClient { .. }
   pub struct ArenaClient { .. }
   pub struct BountyClient { .. }
   ```

4. Each client provides:
   - Read methods (view functions): `get_passport(token_id)`, `get_reputation(agent, domain)`, etc.
   - Write methods (state-changing): `register_passport(name, capabilities)`, `submit_reputation(agent, domain, score)`, etc.
   - Event subscription: `subscribe_events() -> Stream<ContractEvent>`

5. Implement a `ChainConfig` that holds addresses and provider config:
   ```rust
   pub struct ChainConfig {
       pub rpc_url: String,
       pub addresses: ContractAddresses,
       pub signer: Option<SignerConfig>,
   }
   ```

6. Write tests using a fork of the testnet:
   - Register an agent passport and read it back
   - Submit reputation score and verify EMA update
   - Publish a knowledge Signal to InsightStore

## Verification
```bash
cargo check -p roko-chain
cargo clippy -p roko-chain --no-deps -- -D warnings
cargo test -p roko-chain -- clients  # requires testnet fork or mock
```

## What NOT to do
- Do NOT implement custom RPC handling -- use Alloy's built-in providers
- Do NOT proceed without compiled contract ABIs from M076
- Do NOT add transaction retry logic in this batch -- keep clients simple
- Do NOT hardcode addresses -- read from ChainConfig
