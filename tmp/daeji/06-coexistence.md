# Coexistence: Two EVM Chains for Different Purposes

This document explains why two separate EVM-compatible chains -- Daeji and Mirage-rs -- coexist in the same agent infrastructure, how agents interact with both, and what the long-term relationship between them looks like.

---

## What Roko Is

Roko is a self-developing Rust toolkit. 18 crates, approximately 177,000 lines of code. It reads PRDs (product requirements documents), generates implementation plans (DAGs of tasks), dispatches LLM-powered agents (Claude, Codex, Cursor, Gemini, Ollama) to execute those tasks, validates the results through a gate pipeline, persists outcomes as episodes, distills knowledge from episodes, and feeds that knowledge back into future agent dispatches. The core loop is: **PRD -> plan -> agent -> gate -> persist -> learn -> repeat**.

Roko develops itself. The CLI commands for this workflow are `roko prd`, `roko plan run`, `roko dashboard`, and `roko status`. The main orchestration logic lives in `crates/roko-cli/src/orchestrate.rs` -- a single large module that wires together the plan executor, agent dispatcher, gate pipeline, episode logger, knowledge store, and learning subsystems.

The crates most relevant to chain integration are:

- **roko-chain** (`crates/roko-chain/`): Alloy-based Ethereum client. Contains `AlloyChainClient` (JSON-RPC wrapper), `AlloyChainWallet` (key management, transaction signing), and `ChainWitnessEngine` (on-chain attestation via `witness_on_chain` / `verify_on_chain`). This is the existing chain integration code that both daeji and mirage-rs connect through.
- **roko-runtime** (`crates/roko-runtime/`): `ProcessSupervisor` for managing child processes (agent CLIs), `EventBus` for internal event dispatch, `CancelToken` for graceful shutdown. The supervisor tracks PIDs, escalates SIGTERM to SIGKILL, and integrates with the plan executor's cancellation flow.
- **roko-gate** (`crates/roko-gate/`): The 7-rung gate pipeline plus standalone gates. Each rung dispatches one or more concrete verification implementations.
- **roko-learn** (`crates/roko-learn/`): Episode logger, HDC fingerprinting, playbook extraction, model routing, efficiency tracking.
- **roko-neuro** (`crates/roko-neuro/`): Durable knowledge store. Six kinds of knowledge (Insight, Heuristic, Warning, AntiKnowledge, CausalLink, StrategyFragment), each with configurable half-lives, confidence scores, retention tiers (Transient, Working, Consolidated, Persistent), and HDC fingerprints for similarity search.

---

## What Each Chain Is

### Daeji: A Real Consensus Chain for Shared Agent State

Daeji (internally codenamed "Kora") is a minimal Ethereum Virtual Machine (EVM) blockchain built from scratch using the commonware library suite. It runs BFT consensus across multiple validators, meaning transactions are finalized by cryptographic agreement among a set of independent nodes.

Key properties:

- **Consensus**: Simplex BFT with BLS12-381 threshold signatures. In the devnet configuration, 4 validators participate, and any 3 of them (3-of-4 threshold) must agree to finalize a block. This means no single party controls what gets committed to the chain, and once a block is finalized, it cannot be reverted.
- **Execution**: REVM (the Rust EVM implementation also used by Foundry and Reth). Full Solidity smart contract support.
- **State storage**: QMDB (Quick Merkle Database), an authenticated key-value store. Every state commit produces a Merkle root, enabling cryptographic proofs that a particular value existed at a particular block height.
- **Chain ID**: 1337
- **Block time**: ~400ms (dependent on consensus round timing).
- **Deployment**: Docker Compose (4+ containers, one per validator) or native multi-process.
- **Use case**: A shared agent ledger. Agents post knowledge entries, anchor tamper-evident records of completed tasks ("witness anchoring"), register their identities, and read knowledge shared by other agents. The consensus mechanism guarantees these records are authentic and cannot be retroactively altered.
- **Custom features beyond a standard EVM chain**: threshold VRF (verifiable random function) -- every block header's `prevrandao` field contains a bias-resistant random number produced by the validator threshold signature, usable for fair randomness on-chain. QMDB state proofs -- prove that key K had value V at any historical block N. 240-byte finality certificates -- a single compact proof (48-byte group public key + 96-byte threshold signature) that a block is finalized, verifiable by any external system without running a light client. Deterministic simulation -- the full consensus network can be run in a single process for testing using commonware's deterministic runtime. Secondary/follower peers -- processes that replicate all blocks without voting, enabling real-time finalization event streaming.

### Mirage-rs: An In-Process EVM Fork Simulator

Mirage-rs is a fundamentally different kind of chain. It is an in-process EVM fork simulator built on REVM (the same Rust EVM engine). It does not have consensus, validators, or a peer-to-peer network. It is a single Rust process that clones state from a real Ethereum chain and lets you execute transactions against that cloned state locally.

In the roko ecosystem, mirage-rs lives alongside the `roko-chain` crate. The existing `AlloyChainClient` and `AlloyChainWallet` types connect to mirage-rs through standard Ethereum JSON-RPC -- mirage-rs looks like any other Ethereum node from the client's perspective. The difference is entirely in what backs the RPC: not a real blockchain network, but an in-memory copy-on-write fork.

Key properties:

- **Consensus**: None. Mirage-rs uses an "auto-miner" that instantly produces a block for every submitted transaction. There is no finality delay, no validator set, no BFT voting. This is by design -- it is a simulator, not a real chain.
- **State model**: Copy-on-write over an upstream fork. When Mirage-rs receives a read request for a storage slot it has not seen before, it lazily fetches that slot from the upstream chain (e.g., Ethereum mainnet via an Alchemy or Infura RPC endpoint). Writes stay local -- they never propagate back to the upstream chain. This means Mirage-rs can simulate transactions against the exact state of mainnet at a given block, without actually affecting mainnet.
- **Chain ID**: 88888
- **Block time**: Configurable (50ms-1s), but effectively instant since blocks are mined on demand with every transaction submission.
- **Deployment**: Single process, single container. Currently deployed on Railway (a cloud platform for hosting containerized services).
- **Use case**: Answering "what if?" questions. What would happen if I executed this DeFi strategy against current mainnet state? What would this contract deployment look like? What are the gas costs of this transaction sequence? Mirage-rs simulates all of this without spending real ETH or risking real assets.
- **Custom features beyond raw REVM**: HDC index (Hyperdimensional Computing) -- a 10,240-bit binary vector index built from deployed contract storage, enabling semantic similarity search over on-chain state. InsightEntry -- in-memory knowledge entries with HDC vectors, confirmation counts, and half-life decay (a local, volatile knowledge store that does not persist across restarts). PheromoneField -- a stigmergic memory layer that all agents in the same process can read/write, where entries with more confirmations have higher weight. In-memory AgentRegistry. SimulationGate -- a gate rung that forks current state, executes transactions against the fork, validates the result, and discards the fork. HDC precompile at address `0xA0C` for in-process Hamming distance search.

---

## Why Both Exist (They Solve Different Problems)

These two chains serve fundamentally different purposes and are not interchangeable:

| Concern | Mirage-rs | Daeji |
|---|---|---|
| **What kind of chain** | In-process EVM simulator (single process, no consensus) | Real consensus chain (multi-validator BFT) |
| **Core guarantee** | Accurate simulation of real-world chain state | Tamper-evident shared ledger with cryptographic finality |
| **Consensus** | None (auto-miner, instant blocks) | Simplex BFT (threshold signatures, 3-of-4 agreement) |
| **Chain ID** | 88888 | 1337 |
| **State origin** | Copy-on-write over an upstream fork (e.g., Ethereum mainnet) | Fresh genesis, independent QMDB Merkle tree |
| **Primary use** | Fork mainnet state, simulate DeFi strategies, test contracts against real data | Agent knowledge sharing, witness anchoring, verifiable randomness |
| **Block time** | Configurable (50ms-1s), effectively instant | ~400ms (consensus-dependent) |
| **Persistence** | None (in-process memory; state is lost on restart) | Permanent (chain storage with Merkle proofs) |
| **Cross-agent visibility** | No (single process only) | Yes (all agents on the network see all state) |
| **Can fork mainnet state** | Yes (core capability) | No |
| **Cryptographic proofs** | No | Yes (QMDB Merkle proofs, threshold finality certificates) |
| **On-chain VRF** | No | Yes (via BLS threshold consensus) |
| **Deployment** | Railway (single container) | Docker Compose (4+ containers) or native multi-process |

**Mirage-rs cannot replace Daeji** because it has no consensus: a single operator controls everything, there is no cryptographic finality, and records can be silently modified. Nothing stored in Mirage-rs is trustworthy to a third party.

**Daeji cannot replace Mirage-rs** because it does not fork external chain state. You cannot ask Daeji "what would happen if I swapped 100 ETH for USDC on Uniswap right now?" -- Daeji has its own independent state, not mainnet's. Mirage-rs can answer that question because it lazily fetches mainnet's exact storage slots.

---

## How Agents Interact With Both Chains

### The Orchestration Flow: Where Chain Interactions Happen

The dispatch cycle in `orchestrate.rs` follows this sequence for each task:

1. **Plan executor** picks the next ready task from the DAG.
2. **Knowledge query**: The `NeuroStore` is queried for entries relevant to the task. Today this reads `.roko/neuro/knowledge.jsonl`. With daeji, this would also query the InsightLedger contract for on-chain knowledge entries from other agents.
3. **System prompt assembly**: The 9-layer system prompt builder (`build_system_prompt` / `build_role_system_prompt` in `crates/roko-cli/src/prompting.rs`) composes: role identity, task description, plan context, knowledge entries (the neuro context sections from step 2), tool allowlist, safety constraints, gate expectations, strategy fragments, and daimon (affect engine) context.
4. **Agent dispatch**: The `CascadeRouter` selects a model and backend. The agent is spawned via `ProcessSupervisor` (which tracks the PID and registers a cancellation token). The agent runs the task using Claude CLI, Codex, or another backend.
5. **Gate pipeline**: The 7-rung pipeline validates the result:
   - Rung 0: **Compile** (`CompileGate` -- `cargo build`)
   - Rung 1: **Lint** (`ClippyGate` -- `cargo clippy -- -D warnings`)
   - Rung 2: **Test** (`TestGate` -- `cargo test`)
   - Rung 3: **Symbol** (`SymbolGate` -- verifies exported symbols match the manifest)
   - Rung 4: **GeneratedTest** (`GeneratedTestGate` + `VerifyChainGate` -- generates and runs behavioral tests)
   - Rung 5: **PropertyTest** (`PropertyTestGate` + `FactCheckGate` -- property-based testing)
   - Rung 6: **Integration** (`LlmJudgeGate` + `IntegrationGate` -- integration scenarios)
   Plus standalone gates (DiffGate, CodeExecutionGate, ShellGate, SecurityScanGate, etc.) invoked for specific scenarios.
   With both chains available, two new gate types extend this pipeline (see Pattern 4 below).
6. **Episode logging**: An `Episode` is recorded to `.roko/episodes.jsonl`. Episode fields include: `task_id`, `model` (which LLM was used), `backend` (which provider), `gate_verdicts` (per-rung pass/fail), `usage` (prompt tokens, completion tokens, latency, cost), `success`, `hdc_fingerprint` (computed from the prompt and outcome via `attach_episode_hdc_fingerprint` in orchestrate.rs), `turns`, `failure_reason`, `reflection`, and `reasoning_summary`.
7. **Knowledge distillation**: The neuro distiller examines recent episodes and extracts knowledge entries. These feed back into step 2 for future tasks.
8. **Chain anchoring** (with daeji): The episode hash is submitted to daeji via `ChainWitnessEngine::witness_on_chain`. The transaction contains `b"roko.attestation.witness:" ++ blake3(episode_json)`.

Chain interaction can happen at steps 2 (read knowledge), 5 (simulation gate, witness gate), and 8 (episode anchoring). Mirage-rs serves step 5 (simulation). Daeji serves steps 2, 5, and 8.

### Configuration in roko.toml

Both chains are configured as separate entries in `roko.toml` (roko's main configuration file, located at the workspace root). The `purpose` field is informational and helps the orchestration layer decide which chain to use for which operation:

```toml
[chain.mirage]
rpc_url    = "http://localhost:8545"
chain_id   = 88888
purpose    = "fork_simulation"
# Optional: upstream fork source. If absent, mirage-rs starts from a fresh genesis.
fork_url   = "https://eth-mainnet.alchemyapi.io/v2/${ALCHEMY_KEY}"
fork_block = "latest"

[chain.daeji]
rpc_url   = "http://localhost:8550"    # different port to avoid collision
chain_id  = 1337
purpose   = "agent_ledger"
agent_key = "${DAEJI_AGENT_KEY}"       # secp256k1 private key for signing txs

[chain.daeji.contracts]
agent_registry = "0x..."
insight_board  = "0x..."
bounty_market  = "0x..."
```

Both chains expose standard Ethereum JSON-RPC interfaces, so the same `AlloyChainClient` / `AlloyChainWallet` types from the `roko-chain` crate work for both. The agent holds two provider instances, one per chain. The existing roko-chain code already supports this pattern -- `AlloyChainClient::http(url)` and `AlloyChainWallet::from_hex_key(url, key, chain_id)` are parameterized by URL and chain ID, so connecting to a second chain requires only a second instantiation.

---

## Port Allocation

Running both chains locally requires explicit port separation to avoid collisions. The default port 8545 is the standard Ethereum JSON-RPC port. The allocation below gives Mirage-rs the standard port (less configuration friction for existing DeFi tooling like Foundry and Hardhat) and shifts Daeji's validators up:

| Service | Port | Notes |
|---|---|---|
| Mirage-rs RPC | 8545 | Default JSON-RPC port; unchanged for DeFi tool compatibility |
| Mirage-rs agent-relay WebSocket | 9011 | Agent event stream sidecar |
| Daeji validator-node0 RPC | 8550 | Shifted up to avoid collision with Mirage-rs |
| Daeji validator-node1 RPC | 8551 | Second validator's RPC endpoint |
| Daeji validator-node2 RPC | 8552 | Third validator's RPC endpoint |
| Daeji validator-node3 RPC | 8553 | Fourth validator's RPC endpoint |
| Daeji secondary peer RPC | 8554 | Optional follower node for Roko agents (replicates blocks, no voting) |
| Daeji validator P2P | 30400-30403 | Authenticated P2P overlay; no collision (Mirage-rs has no P2P) |
| Daeji secondary peer P2P | 30500 | |
| Daeji Prometheus metrics | 9000-9003 | Per-validator metrics scrape targets |
| Prometheus UI | 9090 | Optional; requires `COMPOSE_PROFILES=observability` |
| Grafana | 3000 | Optional; admin/admin default credentials |
| Roko HTTP control plane | 6677 | Roko's HTTP API (~85 REST routes for orchestration, dashboard, agent management) |
| Roko agent sidecar | 6678 | Per-agent HTTP sidecar (`roko-agent-server`): /message, /stream, /research, /tasks |

**Alternative allocation**: if most agents interact primarily with Daeji rather than Mirage-rs, give Daeji the 8545-8548 range and shift Mirage-rs. The `roko.toml` configuration is the single source of truth -- changing ports requires only updating `rpc_url` values.

### Starting Both Locally

```bash
# Terminal 1: start mirage-rs (single process)
cargo run --release --bin mirage -- \
  --chain-id 88888 \
  --port 8545 \
  --fork-url "${FORK_RPC_URL}"

# Terminal 2: start daeji devnet (Docker Compose, 4 validators)
cd path/to/daeji
just trusted-devnet   # starts 4 validators on ports 8550-8553

# Terminal 3: start roko orchestrator
cargo run -p roko-cli -- serve   # HTTP control plane on :6677
```

---

## Interaction Patterns

These are the primary ways agents use both chains together. Each pattern exploits the unique strengths of both systems.

### Pattern 1: Simulate on Mirage-rs, Commit on Daeji

This is the most common dual-chain pattern. The agent uses Mirage-rs for safe, cost-free experimentation and Daeji for durable, shared record-keeping.

1. The agent forks current mainnet state on Mirage-rs (Mirage-rs lazily fetches any storage slots it needs from the upstream chain).
2. The agent simulates a DeFi strategy (e.g., a leveraged yield position or a multi-hop swap through Uniswap pools) against that forked state, executing N transactions.
3. The agent validates: expected profit, acceptable slippage, no reverts.
4. If the simulation passes: the agent computes `result_hash = blake3(strategy_params || strategy_result)` and posts a summary to Daeji's `InsightBoard` contract. The on-chain entry maps to roko-neuro's `KnowledgeEntry` struct:
   ```
   InsightBoard.post(
       contentHash = result_hash,
       entryType   = 5,        // StrategyFragment (one of 6 KnowledgeKind variants)
       halfLifeBlocks = 648000, // ~15 days at 2s/block (STRATEGY_FRAGMENT_HALF_LIFE_BLOCKS)
       content     = strategy_summary_json
   )
   ```
5. Other agents can query Daeji for proven strategies. Off-chain, the `NeuroStore` queries `.roko/neuro/knowledge.jsonl` and injects relevant entries into the 9-layer system prompt. On-chain, the same query happens via the HDC precompile (0x09) or `eth_getLogs` on the InsightBoard. Entries with high pheromone counts (many confirmations from different agents) indicate strategies that multiple agents found useful. In roko-neuro terms, this maps to the `confirmation_count` and `distinct_contexts` fields on `KnowledgeEntry`, which drive tier promotion (Transient -> Working at 2+ confirmations, Working -> Consolidated at 3+ distinct contexts).

The key insight: Mirage-rs provides the simulation capability (cheap, instant, against real mainnet state), while Daeji provides the record-keeping (tamper-evident, consensus-backed, visible to all agents).

### Pattern 2: Independent Chains, Shared Agent Identity

Each agent has a single identity that spans both chains, even though the chains use different port ranges and chain IDs:

- One **Ed25519 keypair** (from the commonware cryptography library) serves as the agent's P2P identity and is used for authentication in Daeji's network overlay.
- A separate **secp256k1 keypair** is used to sign Ethereum transactions on both chains. EIP-155 chain ID is included in the transaction signature, so signatures cannot be replayed from one chain to the other.
- The `AgentRegistry` contract on Daeji links both identities: it stores the agent's Ed25519 public key alongside the Ethereum address derived from the secp256k1 key.

In the roko-chain crate, the existing `AlloyChainWallet` handles this directly:

```rust
// In roko: one private key, two wallet instances for two chains.
// AlloyChainWallet::from_hex_key is already in crates/roko-chain/src/alloy_impl.rs.
let mirage_wallet = AlloyChainWallet::from_hex_key(
    "http://localhost:8545",
    &agent_key,
    88888,   // mirage chain ID
)?;

let daeji_wallet = AlloyChainWallet::from_hex_key(
    "http://localhost:8550",
    &agent_key,  // same private key
    1337,    // daeji chain ID
)?;
```

This means: when an agent posts a result to the `InsightBoard` on Daeji, other agents can verify that the same identity also ran the simulation on Mirage-rs, because the same Ethereum address signed transactions on both chains.

### Pattern 3: Daeji as Witness for Mirage-rs Simulations

This pattern creates a tamper-evident audit trail for simulations, establishing that an agent committed to test parameters before seeing results. It extends roko's existing `ChainWitnessEngine` (in `crates/roko-chain/src/witness.rs`) to the dual-chain setup.

1. The agent prepares simulation parameters and computes `params_hash = blake3(fork_block_number || strategy_params || account_address)`.
2. The agent anchors `params_hash` on Daeji **before** running the simulation. This uses the `ChainWitnessEngine`, which submits a transaction with `b"roko.attestation.witness:" ++ params_hash` as calldata and waits for it to be mined in a finalized block. This is the same witness mechanism that roko already uses for episode anchoring in orchestrate.rs (step 8 of the dispatch cycle).
3. The agent runs the simulation on Mirage-rs.
4. The agent anchors `result_hash = blake3(params_hash || simulation_result)` on Daeji after the simulation completes.
5. Verification: anyone can confirm that `params_hash` was committed at block N (before the simulation) and `result_hash` was committed at block M > N (after). Since the hash chain links them (`result_hash` includes `params_hash`), the agent could not have known the result before committing to the parameters.

This "Proof of Simulation" pattern establishes verifiable pre-commitment without any trusted third party. It matters for accountability: if an agent claims "I tested this strategy before deploying it," the Daeji witness records either confirm or refute that claim.

### Pattern 4: Gate Pipeline Using Both Chains

Roko validates the result of every agent task through its gate pipeline -- a sequence of automated checks called "rungs" that a task must pass before it is considered successful. The standard 7-rung pipeline is defined in `roko-gate/src/rung_selector.rs`:

```
Current 7-rung pipeline (roko-gate):
  Rung 0: Compile gate        (CompileGate -- cargo build)
  Rung 1: Lint gate            (ClippyGate -- cargo clippy -- -D warnings)
  Rung 2: Test gate            (TestGate -- cargo test)
  Rung 3: Symbol gate          (SymbolGate -- exported symbol manifest check)
  Rung 4: Generated test gate  (GeneratedTestGate + VerifyChainGate)
  Rung 5: Property test gate   (PropertyTestGate + FactCheckGate)
  Rung 6: Integration gate     (LlmJudgeGate + IntegrationGate)
```

Each rung returns a `GateResult` (pass/fail with evidence). Adaptive EMA thresholds per rung (stored in `.roko/learn/gate-thresholds.json`) control how strict each rung is. If any rung fails, roko can automatically generate a revised plan via `build_gate_failure_plan_revision` in orchestrate.rs.

With both chains available, two new gates extend this as **standalone gates** (not new rungs, but invoked alongside the pipeline for tasks that involve chain interactions):

```
Extended pipeline with chain gates:
  Rungs 0-6: (unchanged)
  Standalone: Simulation gate     (mirage-rs fork + replay)
  Standalone: Chain witness gate  (daeji anchor + confirm tx mined)
```

**Simulation gate (Mirage-rs)**: If a task involves changes to any code path that interacts with an EVM chain, the simulation gate forks the relevant chain state on Mirage-rs, replays the transactions that the changed code would generate, and verifies: no reverts, expected state changes, gas within budget. If any check fails, the gate fails and the task is re-queued for replanning (roko can automatically generate a revised plan when a gate fails -- this is the `learning_config.replan_on_gate_failure` path in orchestrate.rs).

```rust
// Follows the same GateRung trait pattern as existing gates in roko-gate.
pub struct SimulationGate {
    mirage_url: String,
    fork_url: Option<String>,
    scenarios: Vec<SimulationScenario>,  // derived from task metadata
}

impl GateRung for SimulationGate {
    async fn check(&self, ctx: &GateContext) -> GateResult {
        let mirage = MirageClient::new(&self.mirage_url, self.fork_url.as_deref())?;
        for scenario in &self.scenarios {
            let result = mirage.simulate(scenario).await?;
            if result.reverted {
                return GateResult::fail(format!(
                    "simulation revert: {} at step {}",
                    result.revert_reason, result.failed_step
                ));
            }
        }
        GateResult::pass()
    }
}
```

**Chain witness gate (Daeji)**: After all other gates pass, this gate anchors the episode hash on Daeji and verifies the transaction was mined within a timeout. The episode hash is computed as `blake3(episode_json)`, where `episode_json` is the full serialized `Episode` record (the struct defined in `roko-learn/src/episode_logger.rs` with its 25+ fields: task_id, model, backend, gate_verdicts, usage, hdc_fingerprint, etc.).

```rust
// Uses the existing AlloyChainClient and AlloyChainWallet from roko-chain.
pub struct ChainWitnessGate {
    chain_client: AlloyChainClient,
    wallet: AlloyChainWallet,
    timeout_ms: u64,
}

impl GateRung for ChainWitnessGate {
    async fn check(&self, ctx: &GateContext) -> GateResult {
        let mut attestation = ctx.attestation.clone();
        // witness_on_chain is the existing function in roko-chain/src/witness.rs.
        match witness_on_chain(&mut attestation, &self.wallet, &self.chain_client).await {
            Ok(tx_hash) => GateResult::pass_with_metadata(
                "chain_witness", tx_hash.as_str()
            ),
            Err(e) => GateResult::fail(format!("chain witness failed: {e}")),
        }
    }
}
```

These gates run in sequence: the simulation gate catches bugs cheaply (no real chain state is affected), and the witness gate commits the result durably only after the simulation passes.

### Pattern 5: Episode Flow End-to-End With Both Chains

This pattern traces a single episode from creation to on-chain anchoring, showing every system that touches it:

1. **Agent turn** (orchestrate.rs): An agent completes work on task T-42. The orchestrator creates an `Episode` struct:
   ```
   Episode {
       task_id: "T-42",
       model: "claude-opus-4-6",
       backend: "claude-cli",
       started_at: 2026-04-30T14:00:00Z,
       completed_at: 2026-04-30T14:02:30Z,
       duration_secs: 150.0,
       usage: Usage { prompt_tokens: 12000, completion_tokens: 4500, cost_usd: 0.08 },
       gate_verdicts: [Pass(Compile), Pass(Lint), Pass(Test)],
       success: true,
       hdc_fingerprint: Some("base64-encoded-1280-bytes"),
       ...
   }
   ```

2. **HDC fingerprinting** (orchestrate.rs, `attach_episode_hdc_fingerprint`): The prompt text and outcome are hashed through the HDC encoding pipeline (FNV-1a seed -> splitmix64 expansion -> bind/permute -> majority-vote bundle) to produce a 10,240-bit fingerprint. This uses `roko_learn::hdc_fingerprint::fingerprint_episode`. The fingerprint is stored in the episode's `hdc_fingerprint` field.

3. **Episode logging** (roko-learn `EpisodeLogger`): The episode is appended to `.roko/episodes.jsonl`.

4. **Simulation gate** (if applicable): If the task involved chain-interacting code, the SimulationGate forked mirage-rs state and replayed transactions. The gate verdict is already captured in `gate_verdicts`.

5. **Chain witness gate** (daeji): The `ChainWitnessGate` computes `blake3(episode_json)` and submits it to daeji via `witness_on_chain`. The transaction hash and block number are recorded in the gate result metadata.

6. **Knowledge distillation** (roko-neuro distiller): The distiller examines this episode (and recent episodes from the same task/plan) and may extract a `KnowledgeEntry`. For example, if this is the third consecutive episode where splitting large files before refactoring led to gate success, the distiller produces a Heuristic entry. The entry gets its own HDC fingerprint.

7. **Knowledge posted to daeji** (InsightLedger): The knowledge entry is submitted to daeji's InsightLedger contract with its content hash, type, half-life, and HDC fingerprint (1,280 bytes).

8. **Future agent dispatch queries daeji**: When a new agent is dispatched for a similar task, the orchestrator queries both the local NeuroStore and the on-chain InsightLedger (via the HDC precompile at 0x09) for relevant knowledge entries. Matches are injected into the agent's system prompt.

---

## Process Management: How ProcessSupervisor Handles Both Chains

Roko's `ProcessSupervisor` (in `crates/roko-runtime/src/process/`) already manages child processes for agent CLIs (Claude, Codex, etc.). It tracks PIDs, handles graceful shutdown (SIGTERM -> wait -> SIGKILL escalation), and integrates with `CancelToken` for coordinated teardown.

With daeji, the supervisor's scope expands:

| Process | Current | With Daeji |
|---|---|---|
| Agent CLI processes (Claude, Codex) | Managed by ProcessSupervisor | Unchanged |
| Mirage-rs | External (started separately) | Could be managed by ProcessSupervisor |
| Daeji validator nodes | N/A | Could be managed by ProcessSupervisor (devnet only) |
| Daeji secondary peer | N/A | Could be managed by ProcessSupervisor |

For devnet operation, the supervisor could start daeji validators as child processes (instead of requiring a separate `just trusted-devnet` command). This means `roko serve` would also bring up the chain infrastructure:

```toml
# roko.toml addition for managed chain processes
[chain.daeji.process]
binary    = "daeji-node"
args      = ["--validator", "--index", "0", "--rpc-port", "8550"]
replicas  = 4                    # start 4 validator processes
managed   = true                 # ProcessSupervisor tracks PIDs

[chain.mirage.process]
binary    = "mirage"
args      = ["--chain-id", "88888", "--port", "8545"]
managed   = true
```

In production, daeji validators run independently (Docker, bare metal) and roko connects via RPC. The supervisor only manages the agent processes.

---

## Migration Path

There is no active plan to replace Mirage-rs with Daeji or vice versa. They serve different purposes and are expected to coexist indefinitely. However, this section describes what a consolidation would look like if it were ever pursued.

### What Would Be Lost Without Mirage-rs

Mirage-rs's fork-from-mainnet capability is its core and irreplaceable feature. Daeji starts from a clean genesis and has no mechanism to load live Ethereum mainnet state. Without Mirage-rs, agents lose the ability to test strategies against real DeFi liquidity, real oracle prices, and real token balances. No amount of Daeji development can replicate this unless a mainnet fork mechanism were added to Daeji (significant engineering effort with no current plan).

### What Would Be Lost Without Daeji

Mirage-rs has no consensus -- a single operator controls everything. There is no cryptographic finality, no multi-party agreement, and no tamper-evidence. Records in Mirage-rs are not trustworthy to any external party. Without Daeji, agents lose the ability to share knowledge through a shared ledger that no single party controls. In roko terms: the `NeuroStore` would remain purely local (`.roko/neuro/` on each machine), and knowledge distilled by one agent could never be cryptographically verified by another. The `ChainWitnessEngine` would have nothing to anchor to. Episode hashes would exist only in local JSONL files.

### If Consolidation Were Pursued

**Prerequisites** (conditions that would need to be true before even evaluating):
1. DeFi simulation use case diminishes -- agents no longer need to test against mainnet state.
2. Daeji has persistent state with enough data to simulate realistic scenarios, OR
3. A mainnet fork mechanism is added to Daeji (significant engineering effort).

**Migration steps** (if prerequisites were met):

1. Move contracts from Mirage-rs's genesis allocations to Daeji's genesis, so they exist from block 0.
2. Move in-memory extensions (HDC index, InsightEntry, PheromoneField) to Daeji Solidity contracts or native precompiles, making them persistent and consensus-backed.
3. Replace Mirage-rs's auto-miner with Daeji's consensus for workloads that need real finality. Accept the ~400ms block time (a UX regression for rapid development iteration).
4. Keep Mirage-rs exclusively as a local development tool for fast-feedback contract iteration and Foundry test execution.

### Decision Criteria

The decision turns on one question: **do agents need cross-party trust for their knowledge entries?**

- If agents operate within a single trusted organization and knowledge entries are only consumed internally, Mirage-rs's auto-miner is sufficient. There is no adversary to worry about, and instant blocks are faster. The local `NeuroStore` at `.roko/neuro/` handles everything.
- If agents from different organizations or operators need to trust each other's knowledge entries (or if an audit trail must be credible to external parties), Daeji's BFT consensus is necessary. The multi-validator threshold signature is the only mechanism that provides this guarantee.

Until the cross-party trust requirement becomes pressing: run both.
