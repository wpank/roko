# M164 — Wrap mirage-rs as Connect Cell

## Objective
Wrap the existing mirage-rs simulation engine as a Connect Cell in `roko-chain`. The `tx_sim_gate.rs` and `gate/mod.rs` already reference simulation for transaction pre-flight checks, but there is no unified `SimulationConnect` interface that exposes `fork(block_number)`, `execute(tx)`, and `inspect_state()` as a Cell. Create this Cell and wire it into the safety hook chain as a pre-flight simulation step that runs before any on-chain action is committed.

## Scope
- Crates: `roko-chain`
- Files:
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/simulation.rs` (new file)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/gate/tx_sim_gate.rs` (existing simulation gate)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/lib.rs` (re-export)
- Depth doc: `tmp/unified-depth/18-registries/08-simulation-and-liveness.md`

## Steps
1. Read existing simulation references in roko-chain:
   ```bash
   grep -n 'SimulationOutcome\|fork\|simulate\|TxSimGate\|mirage' /Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/gate/tx_sim_gate.rs | head -20
   grep -n 'simulation\|SimulationOutcome\|fork' /Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/heartbeat_ext.rs | head -15
   ```

2. Check mirage-rs availability in workspace:
   ```bash
   grep -rn 'mirage' /Users/will/dev/nunchi/roko/roko/Cargo.toml
   grep -rn 'mirage' /Users/will/dev/nunchi/roko/roko/crates/roko-chain/Cargo.toml
   ```

3. Create `simulation.rs` with the `SimulationConnect` Cell:
   ```rust
   /// Connect Cell wrapping mirage-rs for transaction simulation.
   ///
   /// Provides fork-execute-inspect pattern for pre-flight safety checks.
   /// Used by the safety hook chain to simulate transactions before commit.
   pub struct SimulationConnect {
       /// Block number at which the simulation fork was created
       fork_block: Option<u64>,
       /// Simulation results cache (tx_hash → outcome)
       results: HashMap<B256, SimulationOutcome>,
   }

   impl SimulationConnect {
       /// Fork the chain state at a given block number for simulation.
       pub async fn fork(&mut self, block_number: u64) -> Result<(), SimulationError> { ... }

       /// Execute a transaction against the forked state.
       pub async fn execute(&mut self, tx: &Transaction) -> Result<SimulationOutcome, SimulationError> { ... }

       /// Inspect state at a given address after simulation.
       pub async fn inspect_state(&self, address: Address) -> Result<StateSnapshot, SimulationError> { ... }

       /// Run full pre-flight check: fork → execute → validate outcome.
       pub async fn preflight(&mut self, tx: &Transaction, block: u64) -> Result<PreflightResult, SimulationError> {
           self.fork(block).await?;
           let outcome = self.execute(tx).await?;
           Ok(PreflightResult { outcome, fork_block: block })
       }
   }
   ```

4. Define result types:
   ```rust
   #[derive(Debug, Clone)]
   pub struct PreflightResult {
       pub outcome: SimulationOutcome,
       pub fork_block: u64,
   }

   #[derive(Debug, Clone)]
   pub struct StateSnapshot {
       pub address: Address,
       pub balance: U256,
       pub nonce: u64,
       pub storage: HashMap<B256, B256>,
   }

   #[derive(Debug, thiserror::Error)]
   pub enum SimulationError {
       #[error("no fork active — call fork() first")]
       NoActiveFork,
       #[error("simulation reverted: {reason}")]
       Reverted { reason: String },
       #[error("fork block {requested} is ahead of chain head {head}")]
       FutureBlock { requested: u64, head: u64 },
   }
   ```

5. Wire into safety hook chain — implement a trait that `ChainHeartbeatExtension` can call:
   ```rust
   /// Trait for pre-flight simulation in the safety chain.
   pub trait PreflightSimulator: Send + Sync {
       fn preflight(&self, tx: &Transaction, block: u64) -> impl Future<Output = Result<PreflightResult, SimulationError>>;
   }
   ```

6. Re-export from `lib.rs`:
   ```rust
   pub mod simulation;
   pub use simulation::{SimulationConnect, PreflightResult, SimulationError};
   ```

7. Write unit tests:
   - Fork creates simulation context
   - Execute without fork returns NoActiveFork error
   - Successful simulation returns outcome
   - Preflight runs full fork→execute pipeline
   - StateSnapshot captures address state correctly

## Verification
```bash
cargo check -p roko-chain
cargo clippy -p roko-chain --no-deps -- -D warnings
cargo test -p roko-chain -- simulation
```

## What NOT to do
- Do NOT import mirage-rs directly if it's not already a dependency — use trait abstractions that can be backed by mirage-rs later
- Do NOT modify existing `tx_sim_gate.rs` — SimulationConnect is a new higher-level abstraction
- Do NOT implement actual EVM execution — mock the simulation backend for now
- Do NOT add alloy types unless they're already imported in roko-chain
- Do NOT wire into orchestrate.rs — this is consumed by chain-internal safety hooks
