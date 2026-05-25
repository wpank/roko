# M076 — Finalize Solidity Contracts

**[BLOCKED:depth]** -- This item depends on `tmp/unified-depth/18-registries/` depth docs being written first. The depth docs specify: contract storage layouts, upgrade patterns, gas optimization targets, event schemas, and cross-contract call patterns.

## Objective
Finalize all Solidity contracts for the on-chain registry system: AgentPassport (ERC-8004), ReputationRegistry (per-domain EMA), InsightStore (knowledge Signal publication), PheromoneRegistry (stigmergic coordination), ArenaRegistry, EvalRegistry, BountyMarket, and DisputeResolver. All contracts must compile with `forge build` and pass unit tests with `forge test`. Blocked until depth docs provide storage layouts and gas targets.

## Scope
- Crates: N/A (Solidity)
- Files: `contracts/src/`
- Phase ref: `tmp/unified-migration/04-PHASE-3-ECONOMY.md` SS3.4
- Spec ref: `tmp/unified/18-ON-CHAIN-REGISTRIES.md` SS2-7
- Depth docs: `tmp/unified-depth/18-registries/` (pending)

## Steps
1. Check what Solidity contracts already exist:
   ```bash
   ls contracts/src/ 2>/dev/null
   find contracts/ -name '*.sol' 2>/dev/null | head -20
   ```

2. Read the existing contract code and foundry config:
   ```bash
   cat contracts/foundry.toml 2>/dev/null
   ```

3. For each contract, implement or finalize per the depth docs:
   - **AgentPassport.sol**: ERC-721 soulbound (non-transferable), fields per SS2 of doc-18
   - **ReputationRegistry.sol**: per-domain EMA with configurable half-life (default 30 days)
   - **InsightStore.sol**: knowledge Signal publication with demurrage and challenge mechanics
   - **PheromoneRegistry.sol**: stigmergic coordination pheromone deposit/decay
   - **ArenaRegistry.sol**: arena registration, task source, scoring function
   - **EvalRegistry.sol**: eval registration, ground truth sources
   - **BountyMarket.sol**: escrow, VCG matching, second-price auction, 4-level dispute
   - **DisputeResolver.sol**: arbiter -> court -> council -> DAO vote

4. Each contract must:
   - Compile with `forge build`
   - Have unit tests that pass with `forge test`
   - Emit events for all state changes (used by indexer in M081)
   - Include NatSpec documentation

5. Write deployment scripts in `contracts/deploy/`.

## Verification
```bash
cd contracts && forge build
cd contracts && forge test
```

## What NOT to do
- Do NOT deploy to any network yet -- that is M077
- Do NOT implement upgrade patterns until depth docs specify them
- Do NOT optimize gas until the depth docs provide targets
- Do NOT proceed without depth docs -- this item is blocked
