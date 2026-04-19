# Consensus, Execution, And Precompiles Checklist

## Scope

Use this file for consensus/execution scaffolding and the first round of precompile interfaces.

## Implementation checklist

- [ ] Decide what lands in `roko-chain` vs `apps/mirage-rs`.
  - protocol/types/traits in `roko-chain`;
  - simulation/emulation and integration fixtures in `mirage-rs`.
- [ ] Define consensus and execution types without overcommitting to a runtime that does not exist yet.
- [ ] Build precompile interfaces as versioned contracts.
  - AgentPassport
  - nCLOB
  - INTENT
  - PROOF_LOG
  - AGENT_REASON
  - HTC
- [ ] Implement simulator-first coverage.
  - fake or emulated precompiles in mirage;
  - deterministic request/response fixtures;
  - failure codes and gas/error behavior documented.
- [ ] Keep validator-set and execution-engine work behind clear module boundaries so it can progress independently of economic features.

## Relevant files

- `crates/roko-chain/src/client.rs`
- `crates/roko-chain/src/wallet.rs`
- `crates/roko-chain/src/types.rs`
- `apps/mirage-rs/src/chain/mod.rs`
- `apps/mirage-rs/src/chain/agent.rs`
- `apps/mirage-rs/src/chain/task.rs`

## Verification checklist

- [ ] Precompile emulation tests exist per contract surface.
- [ ] Failure behavior is documented and tested.
- [ ] Consensus/execution types compile without dragging simulator-only code into `roko-chain`.

## Acceptance criteria

- Precompile surfaces are concrete enough for clients to code against.
- Mirage can exercise the interfaces before a real chain exists.
- Chain-core types and simulator-only helpers remain cleanly separated.
