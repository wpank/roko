# Local Infrastructure Guide: Running Daeji

This document explains how to run a local daeji blockchain devnet from scratch, with
particular attention to why an AI agent orchestration toolkit needs a local blockchain
and how the two systems connect at the infrastructure level. Every concept is defined as
it is introduced. No prior knowledge of daeji, commonware, roko, blockchain consensus,
or cryptographic key ceremonies is assumed.

---

## Why an AI Agent Toolkit Needs a Local Blockchain

This section explains the motivating context. If you already know what roko is and why
it needs a chain, skip to "What Is Daeji?" below.

### What Roko Is

Roko is a Rust toolkit for building AI agents that develop software autonomously. It is
roughly 177,000 lines of code organized into 18 crates. Its core loop works like this:

1. A human (or another agent) writes a PRD (product requirements document) describing
   what needs to be built.
2. An AI agent reads the PRD and generates an implementation plan -- a directed acyclic
   graph (DAG) of tasks, stored as a `tasks.toml` file.
3. A plan executor dispatches LLM-powered agents (Claude CLI, Claude API, Ollama,
   Gemini, Perplexity, OpenAI-compatible backends, and others) to work on each task.
4. After each task completes, a gate pipeline validates the result by running checks in
   sequence: compile, lint, test, symbol analysis, generated tests, property tests, and
   an LLM judge review. There are 7 rungs in the full pipeline; a task must pass all
   applicable rungs to be considered successful.
5. Results are persisted to an episode log (`.roko/episodes.jsonl`). If a task fails
   its gates, the system can automatically generate a revised plan and retry.
6. A learning layer records what worked: which models performed best for which task
   types (CascadeRouter), which prompt variants produced better results (ExperimentStore),
   and high-confidence observations extracted from successful runs (the neuro knowledge
   store).

The main orchestration loop lives in `crates/roko-cli/src/orchestrate.rs`. Everything
else feeds into or out of that loop: agents produce code, gates verify quality, learning
captures patterns, and knowledge informs future runs.

### Three Problems That Need a Chain

**Problem 1: Tamper-proof witness anchoring.** When an agent completes a task and passes
all gates, the only record is a line in a local JSONL file. Anyone with filesystem access
can edit or delete that record. There is no cryptographic proof that a particular agent
produced a particular result at a particular time. Roko already has a `ChainWitnessEngine`
in its `roko-chain` crate that computes `blake3(episode_data)` and can submit the hash
as on-chain calldata -- but it needs a chain to submit it to. With a local daeji devnet,
the witness hash is included in a finalized block signed by the consensus validators,
creating a tamper-evident record that anyone can verify.

**Problem 2: Cross-agent knowledge sharing.** Roko has a knowledge store called "neuro"
(`crates/roko-neuro/`) where agents persist learned observations. There are 6 kinds of
knowledge entries (Insight, Heuristic, Warning, AntiKnowledge, CausalLink,
StrategyFragment), each with a different temporal half-life. Today, this store is local
to a single machine. If you run two agent fleets on different machines, neither can
benefit from the other's learned knowledge. An on-chain InsightBoard contract lets agents
promote high-confidence local knowledge to a shared ledger, and other agents pull that
knowledge into their local stores.

**Problem 3: On-chain coordination.** Agents need a shared registry to announce their
presence, capabilities, and health status. An AgentRegistry contract provides this:
agents register with their Ed25519 public key and capabilities, send periodic heartbeat
transactions to prove they are alive, and can query the registry to discover other agents.
This is the foundation for coordinated multi-agent workflows where agents on different
machines work on related tasks.

### How Roko's ProcessSupervisor Manages Daeji Nodes

Roko has a component called `ProcessSupervisor` in `crates/roko-runtime/src/process.rs`.
It is a generic subprocess lifecycle manager that wraps `tokio::process::Child` with:

- **ProcessId**: a monotonically increasing identifier for each spawned process
- **SpawnConfig**: specifies the binary path, arguments, working directory, and
  environment variables for the child process
- **Cooperative shutdown**: sends SIGTERM, waits a configurable grace period (default 5
  seconds), then SIGKILL if the process has not exited
- **CancelToken integration**: ties child process lifetimes to a cancellation token, so
  when a plan run is interrupted or completes, all managed children are shut down
- **Bulk operations**: `shutdown_all()`, `kill_all()`, `reap_exited()` for managing
  groups of child processes
- **Drop guard**: when the supervisor is dropped with live children, it force-kills them
  to prevent orphaned processes

Today, ProcessSupervisor manages Claude CLI agent processes during plan execution. The
same infrastructure can manage daeji node processes: start 4 validator nodes before the
first task, monitor their health, and shut them all down when the plan completes. From
the supervisor's perspective, a daeji validator is just another child process with a
binary path (`kora`), arguments (`validator --data-dir ... --peers ... --chain-id 1337`),
and a health check (HTTP request to `kora_nodeStatus`).

### How Roko's Gate Pipeline Would Use the Local Chain

After an agent completes a task, roko's gate pipeline runs validation checks. The
pipeline is defined in `crates/roko-gate/` and has 7 rungs:

| Rung | Index | What It Checks |
|------|-------|----------------|
| Compile | 0 | `cargo build` succeeds |
| Lint | 1 | `cargo clippy` has no warnings |
| Test | 2 | `cargo test` passes |
| Symbol | 3 | Expected symbols exist in the compiled binary |
| GeneratedTest | 4 | Auto-generated test cases pass |
| PropertyTest | 5 | Property-based tests hold |
| Integration | 6 | LLM judge reviews the diff for correctness |

Plus standalone gates: DiffGate, CodeExecutionGate, BenchmarkRegressionGate,
SecurityScanGate, FormatCheckGate, and ShellGate.

With a local daeji devnet, two new gate capabilities become available:

1. **ChainWitness rung**: after all other rungs pass, the gate pipeline submits a
   `blake3(episode_data)` hash as calldata to the daeji chain. The roko-chain crate
   already implements this in its `witness` module: it constructs a `TxRequest` with
   destination address `0x00...c0`, calldata prefixed with `b"roko.attestation.witness:"`,
   signs it with the agent's wallet, submits via `eth_sendRawTransaction`, waits for
   the receipt, and records `{chain_id, tx_hash, block_number}` in the episode's
   `Attestation` struct. The `ChainWitnessEngine` also has a `verify_on_chain` method
   that re-checks the receipt to confirm the witness payload is present.

2. **ChainSimulate rung**: before sending anything to the real devnet, the gate spins up
   an in-process simulated daeji network (using commonware's deterministic runtime and
   simulated P2P -- no real network sockets, no Docker, runs in milliseconds). It
   replays the task's chain interactions against this simulation. If the simulation
   fails, the task is rejected before touching real chain state.

### What "Programmatic Devnet Bootstrap from Roko" Means

Roko already has the pieces needed to start a daeji devnet as part of an automated plan
run. Concretely:

1. **ProcessSupervisor spawns `keygen setup`** -- generates Ed25519 P2P keys, peers.json,
   and genesis.json into a timestamped directory under `.roko/state/daeji-devnet-<ts>/`.

2. **ProcessSupervisor spawns `keygen dkg-deal`** -- runs trusted-dealer DKG to generate
   BLS12-381 key shares for the 4 validators.

3. **ProcessSupervisor spawns 4 `kora validator` processes** -- each loads its keys from
   the generated directory and begins participating in consensus. The supervisor monitors
   them and ties their lifetime to the plan run's CancelToken.

4. **roko-chain's `AlloyChainClient` connects** -- the `[chain]` section of `roko.toml`
   tells the Alloy-backed JSON-RPC client where to find the devnet:
   ```toml
   [chain]
   rpc_url = "http://127.0.0.1:8545"
   chain_id = 1337
   wallet_key = "0xac09..."
   agent_registry = "0x9fE4..."
   ```
   `AlloyChainClient` implements the `ChainClient` trait (block headers, receipts, logs,
   storage reads, `eth_call`). `AlloyChainWallet` implements the `ChainWallet` trait
   (address, balance, nonce, sign-and-submit, receipt polling).

5. **roko-chain deploys contracts** -- Foundry's `forge script` or the Alloy wallet
   directly deploys AgentRegistry, InsightBoard, and BountyMarket contracts. The
   deployed addresses are written back to the running config.

6. **Agent registers and starts heartbeats** -- the agent calls
   `AgentRegistry.register(ed25519Pubkey, capabilities)` once, then sends
   `AgentRegistry.heartbeat()` every ~15 minutes via a periodic task.

7. **Plan execution proceeds** -- agents run tasks, gates validate, witnesses are
   anchored on-chain. When the plan completes or is interrupted, the CancelToken fires,
   ProcessSupervisor shuts down all validator processes, and the devnet stops cleanly.

Each plan run can get a fresh, isolated devnet. No chain state leaks between runs.

---

## What Is Daeji?

Daeji (internal codename "Kora") is a minimal blockchain node written in Rust. It is
not a fork of go-ethereum, reth, or any existing blockchain client. It is built from
scratch by combining three independent components:

1. **Simplex BFT consensus** (from the commonware library) -- a Byzantine Fault
   Tolerant protocol for agreeing on blocks. "Byzantine Fault Tolerant" means the
   protocol works correctly even if some of the participating nodes are malicious or
   faulty (up to a threshold). "Simplex" means a block goes through three network
   message hops to be finalized: propose, vote, finalize. Once a block is finalized,
   it is permanent and will never be reverted (single-slot finality). Consensus uses
   BLS12-381 threshold signatures: the validator set collectively holds a shared
   cryptographic signing key, and any T-of-N validators can combine their shares to
   produce a valid finalization signature. No individual validator ever knows the
   complete private key.

2. **REVM** (Rust Ethereum Virtual Machine) -- a Rust implementation of the Ethereum
   Virtual Machine. This is what executes smart contracts. Any standard Ethereum
   bytecode (compiled from Solidity, Vyper, etc.) works without modification. REVM
   produces the same results as go-ethereum's EVM implementation. It is also used by
   popular Ethereum development tools like Foundry.

3. **QMDB** (Quick Merkle Database, from the commonware library) -- a storage engine
   that holds the blockchain state (account balances, smart contract storage values)
   in a Merkle tree. A Merkle tree is a data structure where every entry contributes
   to a root hash. This allows compact cryptographic proofs: you can prove that a
   given account has a given balance (or that a contract's storage has a given value)
   by providing a small proof that chains up to the root hash, without revealing the
   entire database.

Because daeji uses REVM and exposes standard Ethereum JSON-RPC endpoints (`eth_`
namespace), any Ethereum-compatible tooling works against it: ethers-rs, alloy,
Foundry (forge, cast, anvil), MetaMask, Hardhat, etc. This is critical for roko
integration because roko's `roko-chain` crate uses Alloy (the standard Rust Ethereum
library, successor to ethers-rs) to talk to the chain.

**What commonware is:** Commonware is a Rust library of 17 independent, composable
blockchain primitives created by Patrick O'Grady (formerly of Ava Labs / Avalanche).
Repository: `github.com/commonwarexyz/monorepo`. It is explicitly the
"anti-framework": rather than providing a monolithic blockchain node you fork and
customize, it provides independent building blocks (cryptography, P2P networking,
consensus, storage, runtime) that you compose however you want. Daeji composes
simplex BFT + authenticated P2P + QMDB + the tokio runtime from commonware, plus
REVM from the broader Rust ecosystem.

**What a "devnet" means here:** Unlike development tools such as Anvil or Hardhat that
auto-mine blocks instantly with no real consensus, daeji runs real multi-node
consensus. A local devnet is a fully functional blockchain running on your machine
with multiple communicating validator processes. This makes setup slightly more
involved but means you are testing against the actual consensus and finality behavior
that a production deployment would exhibit. For roko, this matters because witness
anchoring depends on real finality -- a hash included in a block signed by a BFT
supermajority is a fundamentally stronger guarantee than a hash written to a
single-process auto-mined chain.

---

## Devnet Architecture

A standard local daeji devnet consists of:

- **4 validator nodes** -- each holds a BLS12-381 cryptographic "share" and
  participates in the simplex BFT consensus protocol: proposing blocks when it is the
  leader, voting on proposals from others, and contributing its signature share to
  finalize blocks.

- **1 secondary node** (also called a follower) -- connects to the peer-to-peer
  overlay network, receives and stores all finalized blocks, but never votes on
  proposals and never proposes blocks. A secondary node is useful as a read-only
  JSON-RPC endpoint that does not add load to the consensus process. Technically, a
  secondary node authenticates to the P2P network using an Ed25519 identity key (the
  same kind of key validators use for P2P identity, distinct from the BLS12-381
  consensus key) but is not included in the validator set. In the roko integration
  (Tier 2), roko itself runs a secondary peer to get push-based block finalization
  events instead of polling the JSON-RPC endpoint.

- **Threshold: 3-of-4** -- any 3 of the 4 validators can finalize a block. This means
  1 validator can be offline (or malicious) and the network continues to operate. If 2
  or more validators are down, the network stalls (no blocks are finalized) but does
  not produce incorrect results.

- **Chain ID: 1337** -- the Ethereum chain identifier. Returned by `eth_chainId` and
  included in every signed transaction to prevent replay attacks across chains. This
  must match the `chain_id` field in roko.toml's `[chain]` section.

- **Block time: ~400ms** -- the approximate time for one consensus round to complete
  and a block to be finalized. This is fast enough for interactive development (witness
  transactions confirm in under a second) but slow enough to represent realistic
  consensus behavior.

All 5 nodes run on your local machine. When using Docker Compose, they run in
containers. When using the native cargo path, they run as separate OS processes that
you manage in separate terminal windows (or, when integrated with roko, as child
processes managed by ProcessSupervisor).

---

## The DKG Ceremony

Before any validator node can start producing blocks, the validator set must perform a
**DKG -- Distributed Key Generation** ceremony. This section explains what DKG is, why
it is needed, and the two available modes.

### What Is DKG?

DKG (Distributed Key Generation) is a multi-party cryptographic protocol where N
participants collectively generate a shared secret key. The result is:

- **One "share" per validator** -- a piece of the shared secret that each validator
  stores privately.
- **One group public key** -- a single BLS12-381 public key (48 bytes) that corresponds
  to the shared secret. Anyone can verify signatures made by the group using only this
  public key.

The critical security property: after DKG completes, no individual validator (and no
outside observer) knows the complete private key. Only T-of-N validators combining
their shares can reconstruct the signing capability. (In the standard devnet: T=3,
N=4.)

### Why DKG Is Needed

Simplex BFT uses threshold signatures to finalize blocks. When a block is proposed and
enough validators vote for it, their individual vote signatures (each made with their
share) are combined into a single threshold signature. This combined signature is a
valid BLS12-381 signature under the group public key. Verifying finality requires only
the 48-byte group public key and the 96-byte threshold signature -- the verifier does
not need to know anything about the individual validators or their shares.

For roko, this means witness verification is compact: to prove that an episode hash was
anchored at block N, you need the block header (which includes the threshold signature)
and the group public key. The `ChainWitnessEngine::verify_on_chain` method in roko-chain
checks exactly this: it fetches the receipt, confirms the chain ID matches, confirms
the block number matches, and verifies the witness payload is in the transaction logs.

For this to work, each validator must hold a different share of the same underlying
secret. DKG is the process that generates and distributes these shares.

### Two DKG Modes

**Trusted-dealer DKG** (fast, for local development):

- A single process (the `keygen` binary, distributed with daeji) generates the
  complete secret key, splits it into 4 shares using Shamir's Secret Sharing, and
  writes each share to the appropriate validator's data directory.
- Fast: no interaction between nodes required. Runs in under a second.
- The "dealer" process briefly holds the complete secret before distributing shares
  and exiting. This is a security trade-off: anyone with access to the dealer's
  memory during generation could learn the full key.
- Use for: local development, continuous integration (CI), any environment where you
  trust the machine running setup. This is the recommended mode for roko's
  programmatic devnet bootstrap.
- Command: `keygen dkg-deal --validators 4 --threshold 3 --output-dir <dir>`

**Interactive Joint-Feldman DKG** (secure, for production-like environments):

- Each validator independently generates a random polynomial, broadcasts cryptographic
  commitments to all other validators, and receives encrypted partial shares from
  each peer. Through multiple rounds of communication, each validator ends up with
  its share of the collective secret.
- No single party ever holds the complete secret at any point -- not even the operator
  running the ceremony.
- Requires all 4 validator nodes to be running and communicating during setup.
- Slower: takes approximately 10-30 seconds due to multiple communication rounds.
- Use for: production-like testing, staging environments, any situation where you want
  to verify the full ceremony works or where trust assumptions matter.
- In Docker: `just devnet` (uses interactive DKG). In native: `keygen dkg-interactive`.

**For local development, trusted-dealer is recommended.** The DKG output (one key share
per validator plus the group public key) is stored in each node's data directory and
persists across restarts unless you explicitly wipe the data.

---

## Option A: Docker Compose (Recommended for CI and Staging)

Docker Compose starts all 5 nodes (4 validators + 1 secondary), runs the DKG ceremony
automatically, and wires up networking between containers.

### Prerequisites

- **Docker Engine 24+** and **Docker Compose V2** (`docker compose`, not the older
  `docker-compose` standalone binary). Docker Compose V2 is built into modern Docker
  Desktop and Docker Engine installations.

- **`just`** -- a command runner, similar to `make` but with a simpler syntax. It reads
  a `justfile` in the repository root and executes named recipes. Install it via:
  `cargo install just` (if you have the Rust toolchain) or `brew install just` (on
  macOS with Homebrew). The daeji repository's `justfile` defines all the devnet
  lifecycle commands.

- **Daeji repository cloned:**
  ```bash
  git clone https://github.com/Nunchi-trade/daeji
  cd daeji
  ```

### Quick Start

All commands are run from the daeji repository root:

```bash
# Fast start -- trusted-dealer DKG (single process generates all key shares)
just trusted-devnet

# Production-like start -- interactive Joint-Feldman DKG (no single party learns secret)
just devnet

# With observability -- adds Prometheus metrics scraping + Grafana dashboards
COMPOSE_PROFILES=observability just devnet
```

Both `just trusted-devnet` and `just devnet` execute three steps in sequence:

1. **`init-config` container** (runs once, then exits): Runs
   `keygen setup --validators 4 --secondary-peers 1 --threshold 3 --chain-id 1337`.
   This generates Ed25519 P2P identity keys for each node, writes `peers.json` (the
   P2P bootstrap peer list with all public keys and addresses), and writes
   `genesis.json` (the initial chain state with pre-funded account balances).

2. **`init-dkg` container** (runs once after init-config, then exits): Performs the DKG
   ceremony. In trusted mode, this runs
   `keygen dkg-deal --validators 4 --threshold 3` in a single process. In interactive
   mode, the validator containers participate in the Joint-Feldman protocol
   themselves. The output is one BLS12-381 key share per validator and the shared group
   public key.

3. **4 validator containers + 1 secondary container** (long-running): Each validator
   loads its DKG share and begins participating in simplex BFT consensus. The
   secondary connects to the P2P overlay and replicates finalized blocks.

The difference between `trusted-devnet` and `devnet` is only in step 2: how the DKG
ceremony is performed.

### Lifecycle Commands

```bash
just devnet-down      # Stop all containers, but preserve Docker volumes.
                      # DKG state is kept on the volumes, so the next
                      # `just devnet` skips DKG re-run and starts faster.

just devnet-reset     # Stop all containers AND delete Docker volumes.
                      # Full reset: next start re-runs the DKG ceremony.

just devnet-logs      # Tail all container logs (all 5 nodes interleaved).
                      # Press Ctrl+C to stop following.

just devnet-stats     # Open a live terminal dashboard that polls
                      # kora_nodeStatus RPC on each validator and shows:
                      # currentView, finalizedCount, nullifiedCount,
                      # peerCount, isLeader.

just devnet-status    # Print container status + the full endpoint table.
```

### Endpoint Table

After the devnet is running, these endpoints are available on your local machine:

| Service | Address | Notes |
|---|---|---|
| validator-node0 JSON-RPC | `http://localhost:8550` | Primary JSON-RPC endpoint; use this for most operations |
| validator-node1 JSON-RPC | `http://localhost:8551` | |
| validator-node2 JSON-RPC | `http://localhost:8552` | |
| validator-node3 JSON-RPC | `http://localhost:8553` | |
| secondary-node0 JSON-RPC | `http://localhost:8554` | Read-only follower; all finalized blocks available |
| validator P2P (node0-3) | `localhost:30400-30403` | Commonware authenticated P2P overlay |
| secondary P2P | `localhost:30500` | Secondary follower P2P port |
| Prometheus metrics (node0-3) | `localhost:9000-9003` | Per-validator Prometheus scrape targets |
| Prometheus UI | `http://localhost:9090` | Only available with `COMPOSE_PROFILES=observability` |
| Grafana | `http://localhost:3000` | Only available with observability profile; login: `admin` / `admin` |

**Port note:** Ports 8550-8554 are used (not the Ethereum-conventional 8545) to avoid
collision with mirage-rs, a separate EVM fork simulator that roko uses on port 8545
for DeFi scenarios. If you are running daeji standalone without mirage-rs, you can
reconfigure the ports in the Docker Compose file
(`docker/compose/devnet.yaml`). When configuring `roko.toml`, set `rpc_url` to
whichever port your devnet uses.

### Docker Architecture in Detail

The Docker Compose file (`docker/compose/devnet.yaml`) orchestrates several container
types in a defined sequence:

**1. `init-config` container** (init container; runs once, then exits):

Runs: `keygen setup --validators 4 --secondary-peers 1 --threshold 3 --chain-id 1337`

Outputs written to shared Docker volumes:
- `node{0-3}/` -- one directory per validator, each containing that node's Ed25519
  P2P identity key (a private key file used to authenticate on the P2P network)
- `secondary0/` -- the secondary node's Ed25519 key directory
- `peers.json` -- the list of all nodes with their public keys and network addresses
  (see "Peers File" in the Configuration Reference below)
- `genesis.json` -- the initial chain state (see "Genesis File" in the Configuration
  Reference below)

**2. `init-dkg` container** (init container; runs once after init-config, then exits):

Runs the DKG ceremony. In trusted mode:
`keygen dkg-deal --validators 4 --threshold 3`

Outputs: one BLS12-381 key share file per validator node, plus the shared group
public key file. Written to the same shared Docker volumes as step 1.

In interactive mode: the validator containers themselves participate in the
Joint-Feldman DKG protocol, coordinated by the `init-dkg` container.

**3. Four `validator-node{0-3}` containers** (long-running):

Each loads its Ed25519 P2P key and BLS12-381 consensus share from its data volume,
connects to the other validators via the commonware authenticated P2P overlay, and
begins participating in simplex BFT consensus. Each exposes:
- One JSON-RPC port (8550-8553) for receiving transactions and answering queries
- One P2P port (30400-30403) for consensus message exchange with other validators

**4. One `secondary-node0` container** (long-running):

Connects to the P2P overlay using its Ed25519 key. Receives and stores all finalized
blocks from the validator network. Never participates in voting or block proposals.
Exposes JSON-RPC on port 8554 for read-heavy workloads that should not add consensus
load.

**5. Optional: `prometheus` + `grafana` containers** (enabled via
`COMPOSE_PROFILES=observability`):

Prometheus scrapes each validator's metrics endpoint (`localhost:9000-9003`) every 15
seconds. Grafana provides pre-built dashboards for: block finalization rate, consensus
view progression, nullification count (views where no block was finalized, which
indicates temporary consensus stalls), and peer connectivity.

---

## Option B: Native Cargo (Faster Iteration)

Running natively (without Docker) avoids container overhead and enables faster
edit-compile-test cycles when modifying daeji itself. You manage each node process
yourself (or let roko's ProcessSupervisor manage them).

### Prerequisites

- **Rust nightly toolchain:** Daeji uses nightly-only Rust features for BLS12-381
  optimizations. Install and set as default:
  ```bash
  rustup install nightly
  rustup default nightly
  ```

- **`just`** (task runner): `cargo install just` or `brew install just`

- **Daeji repository cloned:**
  ```bash
  git clone https://github.com/Nunchi-trade/daeji
  cd daeji
  ```

### Build

```bash
# From the daeji repository root:
just build
# Equivalent to: cargo build --release
# Produces three binaries in ./target/release/:
#   kora    -- the node binary (runs in validator or secondary mode)
#   keygen  -- key generation + DKG ceremony tool
#   loadgen -- EIP-1559 transaction load generator for stress testing
```

### The Three Binaries

Daeji produces three executable binaries:

1. **`kora`** -- the main node binary. Runs in one of three modes:
   - `kora validator` -- full consensus participant: proposes blocks, votes, signs
   - `kora secondary` -- read-only follower: receives blocks, serves RPC, no voting
   - `kora dkg` -- participates in interactive Joint-Feldman DKG ceremony

2. **`keygen`** -- a setup and key management utility:
   - `keygen setup` -- generates Ed25519 P2P keys, peers.json, and genesis.json for a
     new devnet
   - `keygen dkg-deal` -- runs trusted-dealer DKG (single process, fast, insecure)
   - `keygen dkg-interactive` -- coordinates interactive Joint-Feldman DKG

3. **`loadgen`** -- a load generator that creates, signs, and submits EIP-1559
   transactions to a running devnet for throughput testing. (EIP-1559 is the Ethereum
   transaction format that uses a base fee + priority fee pricing model.)

### Step-by-Step: Start a Local Devnet

**Step 1: Generate keys and genesis**

```bash
cargo run --release --bin keygen -- setup \
  --validators 4 \
  --secondary-peers 1 \
  --threshold 3 \
  --chain-id 1337 \
  --output-dir /tmp/kora-devnet
```

This writes the following to `/tmp/kora-devnet/`:
- `node{0-3}/` -- one directory per validator. Each contains that node's Ed25519 P2P
  identity private key.
- `secondary0/` -- the secondary node's Ed25519 key directory.
- `peers.json` -- the bootstrap peer list. Contains the public key and listen address
  of every node. Distributed to all nodes so they know how to find each other.
- `genesis.json` -- the initial chain state. Contains chain ID, genesis timestamp, and
  pre-funded account allocations.

**Step 2: Run the DKG ceremony**

```bash
# Trusted-dealer mode (fast, recommended for local development):
cargo run --release --bin keygen -- dkg-deal \
  --validators 4 \
  --threshold 3 \
  --output-dir /tmp/kora-devnet

# After this, each node{0-3}/ directory also contains a BLS12-381 key share file.
```

For interactive Joint-Feldman DKG, use `keygen dkg-interactive` instead. This requires
all 4 validators to already be running and reachable on the P2P network before the
ceremony begins -- so you would start the validators first (without consensus) in DKG
mode, run the ceremony, then restart them in validator mode.

**Step 3: Start the validator nodes**

Each validator runs as a separate OS process. Open 4 terminal windows:

```bash
# Terminal 1 (validator node0):
cargo run --release --bin kora -- validator \
  --data-dir /tmp/kora-devnet/node0 \
  --peers /tmp/kora-devnet/peers.json \
  --chain-id 1337

# Terminal 2 (validator node1):
cargo run --release --bin kora -- validator \
  --data-dir /tmp/kora-devnet/node1 \
  --peers /tmp/kora-devnet/peers.json \
  --chain-id 1337

# Terminal 3 (validator node2):
cargo run --release --bin kora -- validator \
  --data-dir /tmp/kora-devnet/node2 \
  --peers /tmp/kora-devnet/peers.json \
  --chain-id 1337

# Terminal 4 (validator node3):
cargo run --release --bin kora -- validator \
  --data-dir /tmp/kora-devnet/node3 \
  --peers /tmp/kora-devnet/peers.json \
  --chain-id 1337
```

Each validator starts up, reads its Ed25519 P2P key and BLS12-381 consensus share from
its `--data-dir`, connects to the other validators listed in `--peers`, and begins
participating in simplex BFT consensus. When at least 3 of the 4 are running, blocks
begin to finalize (because the threshold is 3-of-4).

**Step 4 (optional): Start the secondary peer**

```bash
# Terminal 5 (secondary follower):
cargo run --release --bin kora -- secondary \
  --data-dir /tmp/kora-devnet/secondary0 \
  --peers /tmp/kora-devnet/peers.json \
  --chain-id 1337
```

The secondary peer connects to the P2P overlay, receives all finalized blocks, but
never votes. Its JSON-RPC endpoint is available for read-heavy queries without adding
consensus load to the validators.

### Verify the Network Is Running

Query the custom `kora_nodeStatus` RPC method on any validator:

```bash
curl -s -X POST http://localhost:8550 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"kora_nodeStatus","params":[],"id":1}' \
  | jq .
```

Expected response:

```json
{
  "currentView": 42,
  "finalizedCount": 41,
  "nullifiedCount": 0,
  "peerCount": 3,
  "isLeader": false
}
```

Field meanings:
- `currentView` -- the consensus view (round) number. Increments with each consensus
  round, whether or not a block was finalized in that round.
- `finalizedCount` -- the number of blocks that have been finalized (permanently
  committed). If this number is incrementing, the network is producing blocks.
- `nullifiedCount` -- the number of views that passed without finalizing a block (e.g.,
  because the leader was offline or too few validators voted). Should be 0 in a
  healthy 4-validator devnet.
- `peerCount` -- the number of connected P2P peers (should be 3 for a validator in a
  4-node network, since it connects to the other 3).
- `isLeader` -- whether this node is currently the block proposer for the current view.
  Rotates among validators.

**If `finalizedCount` is incrementing, the network is live and producing blocks.**

### Standard Ethereum RPC

In addition to the custom `kora_nodeStatus`, daeji supports the standard Ethereum
JSON-RPC methods:

| Method | Purpose | Roko Usage |
|---|---|---|
| `eth_sendRawTransaction` | Submit a signed transaction | Witness anchoring, agent registration, heartbeats |
| `eth_call` | Read-only contract execution (no state change) | Query InsightBoard, AgentRegistry |
| `eth_estimateGas` | Estimate gas cost of a transaction | Pre-flight witness transactions |
| `eth_getBalance` | Get an account's ETH balance | Check agent wallet has funds |
| `eth_getCode` | Get a contract's deployed bytecode | Verify contracts are deployed |
| `eth_getStorageAt` | Read a contract's storage slot | Direct state inspection |
| `eth_getBlockByNumber` | Get a block by height | Read VRF seed from `mixHash` |
| `eth_getBlockByHash` | Get a block by hash | Witness verification |
| `eth_getTransactionReceipt` | Get the result of a submitted transaction | Confirm witness tx mined |
| `eth_getLogs` | Query event logs with filters | Watch InsightPosted, AgentRegistered events |
| `eth_chainId` | Returns 1337 | Sanity check; must match roko.toml |

These work with any standard Ethereum client library (ethers-rs, alloy, web3.js,
ethers.js, viem) and development tools (Foundry's `cast` and `forge`, Hardhat). Roko
uses Alloy exclusively via the `roko-chain` crate.

---

## E2E Test Harness (In-Process, No Docker)

Daeji ships an end-to-end test suite in `crates/e2e/` that runs a complete simulated
multi-validator consensus network within a single OS process. No Docker containers, no
real network sockets, no external setup required. All you need is the daeji source
code and a Rust test runner.

### How It Works

The key insight is that commonware provides two implementations of every I/O interface,
sharing the same Rust trait (interface):

**Runtime (async executor + timers):**
- `commonware_runtime::tokio` -- the production runtime. Uses tokio's async executor,
  real OS threads, real system clocks, real sockets. This is what runs in production.
- `commonware_runtime::deterministic` -- a single-threaded deterministic simulator.
  Replaces all I/O with in-memory equivalents. Time does not advance unless the test
  explicitly tells it to. All operations are reproducible given the same seed.

**P2P networking:**
- `commonware_p2p::authenticated` -- real TCP connections with Ed25519-authenticated
  handshakes. This is what runs in production.
- `commonware_p2p::simulated` -- an in-process message bus. Sending a "message" to a
  peer is a direct function call within the same process. Configurable latency, packet
  loss, and message reordering for fault injection.

The e2e test harness instantiates multiple validator instances within the same OS
process, each on its own deterministic runtime instance, and connects them via the
simulated P2P layer. The same consensus code that runs in production runs in the
tests -- only the I/O layer beneath it is swapped.

This means:
- **Tests run in seconds** (no container startup, no process spawning).
- **Every test is deterministic** given the same seed. Pass `--seed` to reproduce a
  specific failure.
- **Network conditions are programmable:** partitions, Byzantine validators, lossy
  links, and delayed messages are configured in the test setup, not by manipulating
  the OS network stack.
- **CI runs the full suite** as a single binary with no infrastructure dependencies.

### Running the Tests

**`cargo nextest`** is a Rust test runner (alternative to `cargo test`) that runs tests
in parallel, provides better output formatting, and supports per-test timeouts.
Install it with `cargo install cargo-nextest`.

```bash
# Run all e2e tests in the daeji workspace:
cargo nextest run --workspace --all-features

# Run a specific test by name:
cargo nextest run test_four_validators_reach_consensus

# Run with a fixed seed for reproducibility:
cargo nextest run test_four_validators_reach_consensus -- --seed 42

# Run with verbose output (shows consensus view progression in real time):
cargo nextest run --workspace --all-features --nocapture
```

You can also use standard `cargo test` if you do not have `cargo nextest` installed:

```bash
cargo test --workspace --all-features
```

### Using the Test Harness as a Library

The `TestHarness` struct in `crates/e2e/src/lib.rs` is designed to be imported as a
Rust library dependency from external crates. This is the intended integration point
for roko: it allows roko's gate pipeline to run chain-interaction tests without
starting real processes or Docker containers.

```rust
// Example: using the daeji test harness from an external crate
use daeji_e2e::{TestHarness, HarnessConfig};

#[tokio::test]
async fn my_contract_interaction_test() {
    // 1. Spin up a complete simulated blockchain (4 validators, threshold 3):
    let harness = TestHarness::new(HarnessConfig {
        validators: 4,
        threshold: 3,
        chain_id: 1337,
        seed: 12345,  // deterministic: same seed = same execution
    }).await;

    // 2. Deploy a smart contract:
    let contract_addr = harness.deploy(my_contract_bytecode, deployer_key).await;

    // 3. Submit a transaction to the contract:
    let receipt = harness.send_tx(TxRequest {
        to: contract_addr,
        data: calldata,
        ..Default::default()
    }).await;

    assert!(receipt.status == 1, "transaction succeeded");

    // 4. Read state from the contract:
    let value = harness.call(contract_addr, read_calldata).await;
    assert_eq!(value, expected_value);

    // 5. Tear down (automatic on drop, but explicit for clarity):
    harness.shutdown().await;
}
```

A full cycle (spin up network, deploy contract, submit transaction, read state, tear
down) typically completes in under 30 seconds. This makes the harness suitable for
use as a validation step in roko's gate pipeline -- specifically, as the backing
implementation for the ChainSimulate gate rung.

---

## Configuration Reference

### Roko's `[chain]` Configuration

Before diving into daeji's own configuration, here is how roko connects to the devnet.
The `[chain]` section of `roko.toml` (roko's master configuration file) tells the
roko-chain crate where to find the chain and how to sign transactions:

```toml
[chain]
# The JSON-RPC endpoint of a running daeji validator or secondary node.
# For Docker devnet with default ports: http://127.0.0.1:8550
# For native devnet or standalone: http://127.0.0.1:8545
rpc_url = "http://127.0.0.1:8545"

# The Ethereum chain identifier. Must match the chain_id used during devnet
# genesis generation. Included in every signed transaction and verified by
# ChainWitnessEngine during witness verification.
chain_id = 31337

# The agent's secp256k1 transaction signing key (hex, 32 bytes with 0x prefix).
# NOT the Ed25519 P2P key used by validators. For local development, this can
# be one of the well-known Foundry/Hardhat test keys. Generate a new one with:
# cast wallet new
wallet_key = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"

# Contract addresses, populated after deployment.
agent_registry = "0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0"
bounty_market = "0xDc64a140Aa3E981100a9becA4E685f962f0cF6C9"
```

When roko starts, it reads this config to instantiate:
- `AlloyChainClient` (implements the `ChainClient` trait): read-only operations --
  `block_number()`, `get_block_header()`, `get_receipt()`, `get_logs()`,
  `get_storage_at()`, `eth_call()`, `get_balance()`, `chain_id()`.
- `AlloyChainWallet` (implements the `ChainWallet` trait): sign-and-submit operations --
  `address()`, `balance()`, `nonce()`, `sign_and_submit()`, `wait_for_receipt()`.

Both traits are defined in the roko-chain crate and have mock implementations for testing.

### Node Configuration (TOML)

Each daeji node reads a configuration file named `config.toml` from its data
directory. Below is the format with every field explained:

```toml
# ---------------------------------------------------------------------------
# Chain Identity
# ---------------------------------------------------------------------------

# The Ethereum chain ID. Must match the value used during genesis generation
# and the --chain-id argument at startup. Included in every signed transaction
# to prevent cross-chain replay attacks (submitting a transaction signed for
# one chain on a different chain).
chain_id = 1337

# Filesystem path where this node stores its keys, DKG share, and blockchain
# data (blocks, state). Each node must have its own separate data directory.
data_dir = "/var/lib/kora"          # production path
# data_dir = "/tmp/kora-devnet/node0"  # local development path

# ---------------------------------------------------------------------------
# Consensus
# ---------------------------------------------------------------------------

[consensus]

# Path to this node's BLS12-381 threshold signature share file.
# Generated during the DKG ceremony. This is the cryptographic material
# that allows this validator to participate in block finalization.
# Only validators have this; secondary nodes do not.
validator_key = "/data/validator.key"

# Minimum number of validators whose signature shares must be combined
# to produce a valid finalization signature. For a 4-validator devnet
# with threshold 3: any 3 of 4 validators can finalize a block.
threshold = 3

# ---------------------------------------------------------------------------
# Network
# ---------------------------------------------------------------------------

[network]

# The address and port this node listens on for P2P connections from other
# nodes. "0.0.0.0" means listen on all network interfaces.
listen_addr = "0.0.0.0:30303"

# Bootstrap peers: the initial set of known nodes to connect to on startup.
# Format: "<hex-encoded-ed25519-public-key>@<host>:<port>"
# In practice, these are loaded from peers.json generated by `keygen setup`.
# The P2P layer authenticates every incoming connection against these public
# keys; connections from unknown keys are rejected.
bootstrap_peers = [
    "a1b2c3d4...@node1:30303",
    "e5f6a7b8...@node2:30303",
    "c9d0e1f2...@node3:30303",
]

# ---------------------------------------------------------------------------
# Execution (EVM)
# ---------------------------------------------------------------------------

[execution]

# Maximum gas allowed per block. Gas is the unit of computational cost in the
# EVM. 30,000,000 matches the current Ethereum mainnet default. Transactions
# that would push a block over this limit are deferred to the next block.
gas_limit = 30_000_000

# Target block time in seconds. The actual time depends on consensus round
# timing; this is a hint, not a guarantee. In practice, the devnet produces
# blocks roughly every 400ms regardless of this setting.
block_time = 2

# ---------------------------------------------------------------------------
# RPC (JSON-RPC Interface)
# ---------------------------------------------------------------------------

[rpc]

# Address and port for the HTTP JSON-RPC endpoint. Clients (wallets,
# scripts, other applications) connect here to send transactions and
# query blockchain state.
http_addr = "0.0.0.0:8545"

# Address and port for the WebSocket JSON-RPC endpoint. WebSocket supports
# long-lived connections and subscription-based event streaming (e.g.,
# newHeads, logs).
ws_addr = "0.0.0.0:8546"

# ---------------------------------------------------------------------------
# Metrics (Observability)
# ---------------------------------------------------------------------------

[metrics]

# Address and port for the Prometheus-compatible metrics endpoint.
# Prometheus (a monitoring tool) scrapes this endpoint periodically to
# collect time-series data about the node's operation: block rates,
# consensus round times, peer counts, etc.
addr = "0.0.0.0:9000"
```

### Genesis File (`genesis.json`)

The genesis file defines the initial state of the blockchain before any blocks are
produced. It is generated by `keygen setup` and should not be modified manually after
the DKG ceremony has been run (because validators must all agree on the same genesis).

```json
{
  "chain_id": 1337,
  "timestamp": 1714000000,
  "allocations": [
    {
      "address": "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266",
      "balance": "1000000000000000000000000"
    },
    {
      "address": "0x70997970C51812dc3A010C7d01b50e0d17dc79C8",
      "balance": "1000000000000000000000000"
    }
  ]
}
```

Field-by-field explanation:

- **`chain_id`** (integer): The Ethereum chain identifier. Must match the `--chain-id`
  argument used when starting nodes and the `chain_id` in each node's config.toml.
  Must also match the `chain_id` in roko.toml's `[chain]` section. Embedded in every
  signed transaction to prevent replay attacks.

- **`timestamp`** (integer): A Unix timestamp (seconds since January 1, 1970 00:00:00
  UTC) for the genesis block (block 0). Subsequent blocks should have increasing
  timestamps. Note: as of this writing, daeji does not yet correctly set
  `block.timestamp` on subsequent blocks (it remains zero); this is a known issue
  tracked in `10-daeji-changes.md`.

- **`allocations`** (array of objects): Pre-funded accounts. Each entry specifies an
  Ethereum address and a starting balance in wei. Wei is the smallest unit of ETH:
  1 ETH = 1,000,000,000,000,000,000 wei (1e18 wei). The example addresses shown
  above are the well-known Foundry/Hardhat default development accounts; using these
  makes the devnet compatible with standard Ethereum development tooling out of the
  box. The `wallet_key` in roko.toml's `[chain]` section should correspond to one of
  these pre-funded addresses so the agent has ETH to pay for gas when submitting
  witness transactions.

### Peers File (`peers.json`)

The peers file lists all nodes that are authorized to participate in the P2P overlay
network. It is generated by `keygen setup` and distributed to every node (validators
and secondaries alike).

```json
{
  "validators": [
    {
      "id": "node0",
      "pubkey": "a1b2c3d4e5f6...",
      "p2p_addr": "127.0.0.1:30400",
      "rpc_addr": "127.0.0.1:8550"
    },
    {
      "id": "node1",
      "pubkey": "e5f6a7b8c9d0...",
      "p2p_addr": "127.0.0.1:30401",
      "rpc_addr": "127.0.0.1:8551"
    },
    {
      "id": "node2",
      "pubkey": "c9d0e1f2a3b4...",
      "p2p_addr": "127.0.0.1:30402",
      "rpc_addr": "127.0.0.1:8552"
    },
    {
      "id": "node3",
      "pubkey": "f2a3b4c5d6e7...",
      "p2p_addr": "127.0.0.1:30403",
      "rpc_addr": "127.0.0.1:8553"
    }
  ],
  "secondary_peers": [
    {
      "id": "secondary0",
      "pubkey": "b4c5d6e7f8a9...",
      "p2p_addr": "127.0.0.1:30500",
      "rpc_addr": "127.0.0.1:8554"
    }
  ]
}
```

Field-by-field explanation:

- **`id`** (string): A human-readable identifier for the node. Used in logs and
  diagnostics.

- **`pubkey`** (string): The hex-encoded Ed25519 public key for this node. The P2P
  layer uses this to authenticate connections: when node A connects to node B, node B
  verifies that A's connection handshake is signed by one of the public keys in
  peers.json. Connections from unknown public keys are rejected. This is separate from
  the BLS12-381 consensus key -- Ed25519 is for P2P identity; BLS12-381 is for
  consensus signatures.

- **`p2p_addr`** (string): The IP address and port where this node listens for P2P
  connections from other nodes. In Docker Compose, these are container-internal
  hostnames (e.g., `node0:30303`). When running natively, use `127.0.0.1` with the
  appropriate port.

- **`rpc_addr`** (string): The IP address and port where this node's JSON-RPC endpoint
  is accessible. Used by external clients (wallets, scripts, other applications) and
  by monitoring tools. This is the address roko's AlloyChainClient connects to.

---

## Load Testing

Daeji ships a `loadgen` binary for testing throughput under realistic transaction
volumes. This generates EIP-1559 transactions (the modern Ethereum transaction format
with base fee + priority fee pricing), signs them with pre-funded test keys, and
submits them to a running devnet.

### Quick Load Tests

```bash
# 1,000 EIP-1559 transactions submitted to node0:
just loadtest

# 10,000 transactions from 50 funded accounts (stress test):
just stresstest
```

### Custom Load Test

```bash
cargo run --release --bin loadgen -- \
  --rpc-url http://127.0.0.1:8550 \
  --total-txs 5000 \
  --accounts 20 \
  --concurrency 50 \
  --chain-id 1337
```

Parameters:

- **`--rpc-url`** -- the JSON-RPC endpoint to submit transactions to. Can be any
  validator node.
- **`--total-txs`** -- total number of transactions to submit during the test.
- **`--accounts`** -- number of pre-funded sender accounts to use. Spreading
  transactions across multiple accounts avoids per-sender nonce contention (each
  Ethereum account has a monotonically increasing nonce; if all transactions come
  from one account, they must be strictly ordered).
- **`--concurrency`** -- number of concurrent in-flight transactions (submitted but
  not yet finalized). Higher concurrency increases throughput but also increases
  memory pressure on the node.
- **`--chain-id`** -- must match the running devnet (1337 for the standard devnet).

The loadgen binary reports:
- Transactions per second (TPS)
- Median and p99 finalization latency (time from submission to inclusion in a
  finalized block)
- Number of failed transactions (if any)

---

## Programmatic Devnet Bootstrap (from Roko)

This section describes the concrete steps by which roko starts and manages a daeji
devnet as part of its automated plan execution workflow.

### The Components Involved

**ProcessSupervisor** (`crates/roko-runtime/src/process.rs`): A generic subprocess
lifecycle manager. Key API surface:

```rust
// Create a supervisor tied to a cancellation token
let supervisor = ProcessSupervisor::new(cancel_token);

// Spawn a child process (returns a ProcessId)
let pid = supervisor.spawn(SpawnConfig {
    program: "/path/to/kora",
    args: &["validator", "--data-dir", "/tmp/devnet/node0", ...],
    working_dir: Some("/tmp/devnet"),
    env: &[],
}).await?;

// Graceful shutdown: SIGTERM, wait grace period, then SIGKILL
supervisor.shutdown(pid).await;

// Shut down all managed processes
supervisor.shutdown_all().await;

// Reap processes that have already exited
supervisor.reap_exited().await;

// Force kill all (no grace period)
supervisor.kill_all().await;
```

When the CancelToken fires (plan run completes, user presses Ctrl+C, or timeout
expires), the supervisor's drop guard force-kills any remaining children.

**AlloyChainClient / AlloyChainWallet** (`crates/roko-chain/`): Read-only client and
transaction signing wallet, both instantiated from `roko.toml`'s `[chain]` section.

**ChainWitnessEngine** (`crates/roko-chain/src/witness.rs`): Submits witness
transactions and verifies them against on-chain receipts.

### Shell Script Approach

For manual or CI-scripted devnet bootstrap without roko's orchestration:

```bash
# 1. Generate keys and genesis for a fresh, isolated devnet:
keygen setup \
  --validators 4 \
  --threshold 3 \
  --chain-id 42 \
  --output-dir .roko/state/daeji-testnet-$(date +%s)

# 2. Run the DKG ceremony:
keygen dkg-deal \
  --validators 4 \
  --threshold 3 \
  --output-dir .roko/state/daeji-testnet-$(date +%s)

# 3. Start validator nodes (roko's ProcessSupervisor manages these as child processes,
#    automatically stopping them when the plan run completes or is interrupted):
kora validator --data-dir .roko/state/daeji-testnet-<timestamp>/node0 --peers ... --chain-id 42
kora validator --data-dir .roko/state/daeji-testnet-<timestamp>/node1 --peers ... --chain-id 42
kora validator --data-dir .roko/state/daeji-testnet-<timestamp>/node2 --peers ... --chain-id 42
kora validator --data-dir .roko/state/daeji-testnet-<timestamp>/node3 --peers ... --chain-id 42
```

### Programmatic Approach (from orchestrate.rs)

When roko's plan executor starts a plan that has chain integration enabled, the
sequence in `orchestrate.rs` is:

```
1. Read [chain] config from roko.toml
2. Create a timestamped devnet directory under .roko/state/
3. ProcessSupervisor.spawn(keygen setup ...)     -> generates keys + genesis
4. ProcessSupervisor.spawn(keygen dkg-deal ...)  -> generates DKG shares
5. For each of 4 validators:
     ProcessSupervisor.spawn(kora validator ...) -> starts consensus
6. Wait for finalizedCount > 0 on any validator  -> network is live
7. AlloyChainClient::http(rpc_url)               -> connect read-only client
8. AlloyChainWallet::from_hex_key(...)           -> create signing wallet
9. forge script Deploy.s.sol --rpc-url ... --broadcast -> deploy contracts
10. AgentRegistry.register(pubkey, capabilities) -> register agent
11. Begin plan task execution loop:
      - Dispatch agent to task
      - Run gate pipeline (compile -> lint -> test -> ... -> chain witness)
      - Record episode with chain attestation
      - Post high-confidence knowledge to InsightBoard
      - Send heartbeat every ~15 minutes
12. Plan completes or CancelToken fires
13. ProcessSupervisor.shutdown_all()             -> stops all validators
```

### In-Process Approach (Preferred for Gate Validation)

The `TestHarness` in `crates/e2e/src/lib.rs` (described in the E2E Test Harness
section above) can be compiled as a library dependency and used directly from roko's
gate pipeline. This eliminates process management complexity: the entire simulated
daeji network lives in the same OS process as the gate, starts in milliseconds, and
tears down automatically when the gate function returns.

Typical use: add a `ChainSimulate` rung to roko's gate pipeline. Before marking
a task as passed, the gate:

1. Instantiates a `TestHarness` with a seed derived from the task hash (making the
   simulation reproducible for that specific task).
2. Deploys any smart contracts the task would interact with.
3. Replays the on-chain actions the task performed.
4. Asserts the resulting chain state matches expectations.
5. Shuts down the harness (automatic on drop).

If the simulation gate passes, the agent's chain interactions are verified correct
before any witness anchor is posted to a persistent devnet.
