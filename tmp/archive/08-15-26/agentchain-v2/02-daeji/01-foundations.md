# Foundations: How the Nunchi Blockchain Is Built

This document covers the chain from first principles: what it is, what it
is not, the Commonware primitives it is assembled from, the consensus
protocol that finalizes blocks, the cryptography that backs every
signature, the EVM execution engine, the state database, and the JSON-RPC
surface the rest of the world talks to it through. Everything is defined
as it appears.

---

## What the Chain Is

The Nunchi blockchain is a sovereign, application-specific Layer-1 chain
purpose-built for autonomous AI-agent coordination, shared knowledge,
and on-chain economic settlement. It is EVM-compatible: any Solidity
contract, any standard Ethereum tooling (Foundry, MetaMask, Alloy,
ethers.js, Hardhat, viem, web3.js) works against it without
modification.

The chain is **assembled from independent Rust primitives** supplied by
the Commonware library. It is **not a fork** of go-ethereum, Reth, or
any other existing Ethereum client. The three load-bearing components
are independent crates wired together in a small node binary:

- **Commonware's `threshold_simplex`** for Byzantine-Fault-Tolerant
  consensus with BLS12-381 threshold signatures.
- **REVM** (the Rust Ethereum Virtual Machine, the same engine used by
  Foundry and Reth) for executing Solidity bytecode.
- **Commonware's QMDB** (Quick Merkle Database) for authenticated state
  storage.

Above the base EVM the chain adds a small number of custom precompiles
for operations that are computationally infeasible at scale in pure
Solidity (hyperdimensional-vector similarity search, historical state
proofs, pairing-based encryption to a future consensus view). These are
covered in `02-precompiles-and-contracts.md`.

### The Daeji testnet

Daeji is the testnet name for the current devnet deployment of the
Nunchi blockchain. When this folder talks about "the current devnet",
it means Daeji. The chain itself is "the Nunchi blockchain"; the
testnet is "Daeji". Mainnet does not yet exist (see `05-roadmap.md`).

### Korai (historical note)

Older internal material refers to the chain as "Korai" or "Kora". The
current name is "the Nunchi blockchain"; treat references to Korai as
historical.

---

## Why Build a New Chain at All

Existing EVM chains were designed for human users interacting through
wallets and frontends. Autonomous agents that execute trades, manage
capital, and coordinate with each other have different requirements.
The Nunchi blockchain exists because no existing chain provides the
following capabilities simultaneously inside its base layer:

1. **Native vector-similarity search at consensus speed.** Agents
   represent operational knowledge as 10,240-bit binary
   "hyperdimensional vectors" and need to retrieve relevant prior
   knowledge by similarity. Pure-Solidity Hamming search is gas-
   prohibitive at meaningful scale; a native precompile completes the
   same operation in microseconds.
2. **Agents as first-class on-chain citizens.** Agent identity
   (ERC-8004), reputation, and capability bitmasks live as native
   primitives rather than ad-hoc patterns layered over user-account
   semantics.
3. **Threshold-cryptography byproducts.** Because consensus uses
   BLS12-381 threshold signatures, the chain produces a bias-resistant
   verifiable random value in every block, compact ~240-byte finality
   certificates, and the ability to encrypt data so it decrypts only
   when a specific future consensus view is finalized — all for free,
   without bolt-on protocols or external operators.
4. **Validator-computed market-data indices.** Source-protocol rates
   (lending, structured, funding, staking) are read by every validator
   independently and aggregated inside consensus, eliminating the
   separate-operator trust model used by external oracle networks.

The fuller treatment of (2) and (4) lives in `03-agent-systems.md` and
`04-defi-and-operations.md`. This document covers (1) and (3) at the
substrate level: where the primitives come from, how they fit
together, and why the assembled chain is more than the sum of its
parts.

---

## Commonware: The Composable Primitives

Commonware is a Rust library of independent, composable blockchain
building blocks, designed as an explicit "anti-framework". Rather than
providing a monolithic node binary that you fork and customize,
Commonware ships a set of small crates — each implementing one
primitive — that you compose into a node sized to your application.
The library is dual-licensed MIT/Apache-2.0; the chain pins to a
specific Commonware release at any given time so that wire-format
guarantees hold.

### Why composable primitives matter here

Three concrete features of the Nunchi blockchain depend directly on
Commonware's composability:

**1. Threshold cryptography is the consensus, not a bolt-on.** Because
finality is itself an aggregate BLS12-381 signature, four downstream
features come essentially for free: compact ~240-byte finality
certificates, a bias-resistant threshold VRF every block, Binding
Timelock Encryption to future views, and validator-set resharing that
preserves the same group public key across membership changes.

**2. Deterministic full-network simulation is the test harness.** Every
I/O primitive (runtime, P2P, storage) ships with a deterministic
in-process implementation that shares the exact same trait as its
production counterpart. A multi-validator network can run inside a
single OS process, in a single thread, with controllable time and a
fixed random seed. Given the same seed, every run produces identical
behavior — a strategy that is impossible with monolithic clients.

**3. Custom precompiles are first-class.** REVM is parameterized over
a `Database` trait and a precompile set. Adding a new precompile is a
few lines of Rust and a registration call; there is no upstream fork
to maintain.

### The crates the chain composes

| Crate | What it provides |
|---|---|
| `commonware-cryptography` | Ed25519 (P2P identity), BLS12-381 threshold signatures (consensus), threshold VRF, hash-to-curve. |
| `commonware-p2p` | Authenticated, encrypted overlay (`authenticated`) and a deterministic in-process simulator (`simulated`) sharing one trait. |
| `commonware-consensus` | The `threshold_simplex` protocol — Simplex BFT adapted to use BLS12-381 threshold signatures. |
| `commonware-storage` | QMDB authenticated key-value storage and a Merkle Mountain Range append-only log. |
| `commonware-runtime` | Two interchangeable async runtimes: `tokio` (production) and `deterministic` (single-thread simulator). |
| `commonware-codec` | A `derive`-style binary codec used for P2P messages and certificate serialization. |

A few additional Commonware crates exist (an ordered-broadcast
primitive supporting Decoupled State Machine Replication, a pluggable
content-addressed resolver) but are not yet integrated into block
building.

---

## Consensus: Simplex BFT with Threshold Signatures

### The consensus problem in one paragraph

In a distributed system with `N` validators, some of which may be slow,
offline, or outright malicious, "consensus" means all honest
validators agree on the same sequence of blocks despite those faults.
A Byzantine-Fault-Tolerant (BFT) protocol produces correct results
even when up to `f` of `N` validators misbehave, where `N >= 3f + 1`.
The chain uses **Simplex BFT** — a recent design with three message
hops to finality (Chan and Pass, IACR ePrint 2023/463) — adapted to
use BLS12-381 threshold signatures as the voting primitive.

### Three hops to finality

A block passes through three network hops:

1. **Propose.** The leader for the current view broadcasts a candidate
   block.
2. **Vote.** Each validator verifies the proposal and broadcasts a
   signed vote.
3. **Finalize.** When enough votes are collected (the threshold), they
   combine into a single aggregate threshold signature — the finality
   certificate.

Once the finality certificate exists the block is permanent. This is
**single-slot finality**: there is no probabilistic confirmation
waiting (Bitcoin's "wait six blocks") and no epoch-transition latency
(pre-merge Ethereum's two-epoch finality gap). Once a block is
finalized in its slot, it cannot be reverted by any subsequent
computation.

### Leader rotation

Leaders rotate deterministically by view number. In the current devnet
with 4 validators, leader election is `(view % 4) == validator_index`.
If the leader fails to produce a valid block within a timeout, the
view is *nullified* and the next view begins with the next leader.
Nullified views do not produce finalized blocks; the chain keeps
advancing by advancing the view number.

### What `(T, N)` buys you

With `N` validators and threshold `T`, the chain tolerates:

- Up to `N - T` validators offline (consensus continues as long as at
  least `T` are participating).
- Up to `T - 1` validators actively Byzantine (they cannot forge a
  valid threshold signature without `T` honest shares).

The current devnet runs `(T, N) = (3, 4)`: any 3 of the 4 validators
can finalize, 1 can be offline or malicious without halting progress,
and no incorrect block is ever finalized regardless.

---

## BLS12-381 Threshold Signatures

### The curve

BLS12-381 is a pairing-friendly elliptic curve. "Pairing-friendly"
means there exists an efficient bilinear map
`e : G1 × G2 → GT` between three elliptic-curve groups, with the
property `e(a · P, b · Q) = e(P, Q)^(a · b)`. This property is what
enables constructions impossible on standard elliptic curves:
signature aggregation, threshold signatures, and Identity-Based
Encryption (used by BTLE in `03-agent-systems.md`).

Key sizes:

- BLS group public key (a G1 point): **48 bytes**.
- BLS signature (a G2 point): **96 bytes**.
- Individual validator share (a scalar): **32 bytes**.

The `blst` library (Supranational's highly optimized BLS12-381
implementation, also used by Ethereum beacon-chain clients) provides
single-pairing verification in 1–2 milliseconds on modern hardware.

### How threshold signatures work

In a `(T, N)` threshold scheme, `N` participants each hold a **share**
of a private key. The full key is never assembled in one place. Any
`T` participants can combine partial signatures into a single
aggregate signature that verifies against one **group public key**.
Fewer than `T` participants learn nothing about the full key, even if
they pool their shares.

Verification reduces to a single pairing check:

```
e(signature, G2_generator) == e(H(message), group_public_key)
```

Anyone with the 48-byte group public key can verify a threshold
signature without knowing which specific validators cooperated. This
is what makes the chain's finality certificates compact: there is no
participation bitmap, no list of individual public keys, and no
validator-set tracking required by external verifiers.

---

## Distributed Key Generation

The validator group needs shares of a shared private key, but no
single party should ever hold the full key. **Distributed Key
Generation (DKG)** is the cryptographic protocol that establishes the
shares and the matching group public key. The chain ships two DKG
modes for different trust requirements.

### Trusted-dealer DKG (development default)

A single dealer process generates the complete secret, splits it into
`N` shares using Shamir's Secret Sharing, writes each share to the
corresponding validator's data directory, and exits. The dealer
process briefly holds the full secret in memory before distributing
shares.

- Runtime: well under one second.
- Trust: the host running the dealer must be trusted, since anyone
  with read access to its memory could learn the full key.
- Use case: local development, continuous integration, any environment
  where the setup machine itself is trusted.

### Interactive Joint-Feldman DKG (production-like)

Each validator independently generates a random polynomial, broadcasts
cryptographic commitments to all other validators, and receives
encrypted partial shares from each peer. Through multiple rounds of
exchange, each validator ends up with a share of the collective
secret — and **no single party, not even the operator running the
ceremony, ever holds the complete secret at any point**.

- Runtime: approximately 10–30 seconds due to multiple communication
  rounds.
- Trust: trust-minimized; no single party learns the key.
- Use case: production-like staging, any environment where trust
  assumptions matter.

Both modes produce the same artifacts: one BLS12-381 share per
validator (held privately by that validator) plus the 48-byte group
public key (used by anyone verifying a finality signature). The
shares persist across node restarts unless the data directory is
wiped deliberately.

---

## Threshold VRF: Randomness for Free

A **Verifiable Random Function (VRF)** produces a pseudorandom value
plus a proof that the value was correctly derived from a given
message using the key for a known public key. The properties that
matter:

- **Deterministic.** Same key + same message always produce the same
  output. The key holder cannot "try different outputs" and pick a
  favorable one.
- **Unpredictable.** Without the private key, the output is
  indistinguishable from random.
- **Verifiable.** Anyone with the public key checks the output is
  valid for the given message.

When the validator set finalizes a block at view `V`, the aggregate
threshold signature over `V` is a deterministic value — uniquely
determined by the group key and `V`. Because no single validator can
compute it without cooperation from at least `T - 1` others, no
validator can predict or bias the output. And because anyone can
verify the signature against the 48-byte group public key, it is a
VRF output in the strict cryptographic sense.

The chain places this output in the block's `prevrandao` / `mixHash`
field. Smart contracts read it as `block.prevrandao` in Solidity;
off-chain systems read it via `eth_getBlockByNumber`.

### Why this is stronger than Ethereum's RANDAO

Ethereum mainnet's `prevrandao` comes from RANDAO, an accumulator
mixed across an epoch by individual validators. The known weakness:
the last validator in an epoch can choose to withhold their block,
biasing the final accumulator value at the cost of forfeiting the
block reward. For high-value applications this bias is economically
attackable.

On the Nunchi blockchain, `prevrandao` is a single threshold
signature requiring `T` non-colluding validators to produce. The
input (view number) is fixed, the output is uniquely determined, and
manipulating it requires compromising `T` validators simultaneously
— the same threshold required to attack consensus itself. The two
concerns share one security budget rather than running on separate
trust models.

---

## REVM: The Execution Engine

### What it is

REVM is a standalone Rust implementation of the Ethereum Virtual
Machine. It is the same EVM engine used by Foundry (the Solidity
development toolkit) and Reth (a production Rust Ethereum client).
REVM takes compiled EVM bytecode, executes it, and produces state
changes (storage writes, balance transfers, event logs).

### Why REVM

- **Standard behavior.** REVM produces results identical to
  go-ethereum's EVM for any standard bytecode. Contracts deployed on
  the chain behave the same as they would on Ethereum mainnet
  (subject to the chain-specific differences enumerated in
  `02-precompiles-and-contracts.md`).
- **Modular.** REVM is parameterized over a `Database` trait (for
  state access) and a precompile set (for custom native operations).
  Replacing either is straightforward.
- **Rust-native.** No Go or C FFI layer, no cross-language build
  complexity.

### The EVM plane and the native plane

The chain operates on two planes that are strictly separated but
connected through reserved precompile addresses:

- **EVM plane.** What smart contracts see. Standard Ethereum
  semantics plus custom precompiles. Any Solidity contract works.
- **Native plane.** What the node maintains in Rust. Consensus,
  threshold signatures, VRF production, P2P, QMDB internals, and all
  chain-internal bookkeeping.

The two planes meet at the precompile interface. A Solidity contract
calls a reserved address (for example, `0x0B`); the EVM hands off to
a native Rust function with full access to chain internals. The
function returns raw output bytes back into the EVM context.

The deliberate boundary: **deploy as a contract first, migrate to a
precompile only if gas costs or latency become measurable
bottlenecks.** Contracts require zero chain modifications;
precompiles require coordinated node-binary changes that all
validators must adopt simultaneously.

---

## QMDB: The State Database

QMDB (Quick Merkle Database) is Commonware's authenticated key-value
store, designed to handle two requirements that conflict in standard
databases.

### The two requirements

**Fast updates.** Blockchain state changes with every block. Each
block may modify thousands of storage slots. Updates must complete in
milliseconds.

**Cryptographic commitment.** The entire state must be summarized as
a single hash (the state root) included in each block header.
Computing this root requires Merkleization — building a hash tree
over all state entries. In a standard Merkle tree, updating one leaf
requires `O(log N)` hash computations along the path from leaf to
root, each potentially a random disk read.

At scale (millions of storage slots) the random-I/O pattern of
Merkle updates is a primary performance limiter for full nodes.
Ethereum's state database (LevelDB/PebbleDB under a Merkle Patricia
Trie) sees this directly.

### QMDB's solution

- **`O(1)` SSD I/O per state update.** State changes are appended to a
  write-ahead log and batched. The on-disk structure is optimized for
  sequential writes, avoiding the random I/O pattern that makes Merkle
  updates slow.
- **In-memory Merkleization.** The Merkle tree (internal nodes and
  their hashes) lives entirely in RAM. Computing the state root after
  a batch of updates requires no disk reads.

The result is millions of entries with sub-millisecond update latency
and instant state-root computation.

### Current state-root model

The current devnet state root is computed by a deterministic
transition hash:

```
state_root(N) = keccak256("_KORA_STATE_TRANSITION_ROOT"
                          || state_root(N-1)
                          || serialized_changes_at_N)
```

This is **not** a Merkle Patricia Trie root. Two nodes processing the
same blocks in the same order produce the same state root, but the
root does not commit to state in a way that supports
Ethereum-style Merkle inclusion proofs via `eth_getProof`. This is
the single largest architectural gap relative to standard Ethereum.

Two mitigations are designed (covered in
`02-precompiles-and-contracts.md` and `03-agent-systems.md`): a
QMDB-historical-proofs precompile that exposes Merkle proofs against
any historical state root, and a Merkle Mountain Range contract that
covers the critical application-level state without requiring the
state-root format itself to change.

---

## The Block Structure

The block structure is deliberately minimal:

```rust
pub struct Block {
    pub parent: BlockId,
    pub height: u64,
    pub prevrandao: B256,     // VRF seed from threshold consensus
    pub state_root: StateRoot,
    pub txs: Vec<Tx>,
}
```

Notable omissions relative to a standard Ethereum block:

- No `difficulty` field (obsolete post-PoW; `prevrandao` replaces it).
- No `coinbase`/`beneficiary` field populated with a validator
  address (currently set to `Address::ZERO`; derivation from the
  proposing validator's Ed25519 key is on the roadmap).
- No separate receipts root or logs bloom at the protocol level.

Fields that are present: parent hash, monotonic height, the
threshold-VRF seed (`prevrandao`), the QMDB transition-hash state
root, and the list of EIP-1559 transactions in the block.

---

## Block Time and Validator Topology

### Observed block time

The current devnet finalizes blocks at approximately **400 ms**
cadence on a single host. Variance depends on consensus round timing,
leader liveness, and per-block transaction count. This is fast enough
for interactive development (a witness transaction confirms in well
under a second) and slow enough to represent realistic consensus
behavior.

### Target block time

The stated product target is **~50 ms** block time, achievable by
geographic co-location of validators in a single data center
(Tokyo in the original framing, modeled on the operational pattern
Hyperliquid uses). Co-location eliminates network round-trip latency
as the consensus bottleneck; Simplex BFT's commit path can sustain
sub-100 ms cadence once the latency floor is removed.

The 50 ms figure is an architectural target, not a measured devnet
number. The current ~400 ms describes single-host devnet operation;
neither has been validated end-to-end on a multi-machine
geographically-distributed deployment.

### Current devnet topology

- **4 validators** holding BLS12-381 threshold shares, participating
  in Simplex BFT (propose, vote, finalize). Each runs an Ed25519 key
  for P2P identity and a BLS share for consensus.
- **1 secondary peer** — a read-only follower authenticated to the
  P2P overlay. It replicates all finalized blocks but does not vote
  or propose. Useful as a non-consensus RPC endpoint and as the
  integration point for downstream consumers wanting push-based block
  notifications instead of polling.
- **Threshold:** 3-of-4 (any 3 of 4 validators can finalize a block;
  1 can be offline). Hardcoded as `(view % 4) == validator_index` in
  the current devnet; making the count configurable is a small change
  on the roadmap.

### Sequential execution today

Some design documents propose Block-STM parallel execution. The
current implementation deliberately does not include it: workload
shape at devnet scale does not justify the complexity (speculative
execution, conflict detection, rollback). Sequential execution is a
deliberate Phase-1 choice; parallel execution remains a Phase-3+
option if block utilization consistently exceeds a meaningful
fraction of the gas limit.

---

## Authenticated P2P

The peer-to-peer overlay (`commonware-p2p::authenticated`) is mutually
authenticated and encrypted. Every peer is identified by an Ed25519
public key; both sides of every connection prove they hold the
private key for their claimed public key (typically by signing a
challenge), and the resulting session is encrypted using keys derived
from the handshake. Connections from unknown public keys are
rejected.

This design has three direct consequences:

- Validators and secondaries cannot be impersonated.
- Messages cannot be tampered with in transit.
- Joining the network requires an explicit listing in `peers.json` —
  the bootstrap file generated during setup.

The deterministic test counterpart (`simulated`) replaces real TCP
with an in-process message bus that supports configurable link
latency, packet drop probability, partitions, and Byzantine
behavior — and shares the same trait, so the same application code
runs against either implementation without modification.

---

## A 5-Phase Block Model (Aspirational)

A more advanced 5-phase block-processing model appears in design
documents for production DeFi operations. Each block would process
operations in a fixed sequence so that liquidations always see fresh
oracle prices:

1. **Oracle phase.** Apply oracle tick: validators update any
   validator-computed indices (the ISFR rate is the canonical
   example; see `04-defi-and-operations.md`).
2. **Accrual phase.** Compute funding payments for open perpetual
   positions using the freshly updated oracle values.
3. **Liquidation phase.** Check positions against maintenance margin
   using the fresh mark prices.
4. **Trading phase.** Match new orders.
5. **Settlement phase.** Transfer funds and finalize.

This 5-phase ordering is a product-level design that sits on top of
the consensus layer; the current node does not yet enforce it as a
block-construction rule.

---

## JSON-RPC Surface

### Standard Ethereum methods

The chain exposes the standard `eth_*`, `net_*`, and `web3_*`
namespaces. Methods used for typical client work include:

| Method | Purpose |
|---|---|
| `eth_sendRawTransaction` | Submit a signed EIP-1559 transaction. |
| `eth_call` | Read-only contract execution. |
| `eth_estimateGas` | Estimate gas cost of a transaction. |
| `eth_getBalance` | Account ETH balance. |
| `eth_getTransactionCount` | Nonce for an address. |
| `eth_getCode` | Bytecode at a contract address. |
| `eth_getStorageAt` | Read a contract storage slot. |
| `eth_getBlockByNumber`, `eth_getBlockByHash` | Fetch a block (includes `mixHash` = VRF seed). |
| `eth_getTransactionReceipt` | Receipt of a mined transaction. |
| `eth_getLogs` | Query event logs by topic and address. |
| `eth_chainId` | Returns 1337 on the default devnet. |
| `eth_blockNumber` | Latest finalized block number. |
| `eth_gasPrice` | Current base fee. |

Any standard Ethereum tooling — Foundry (`cast`, `forge`, `anvil`),
Hardhat, MetaMask, Alloy, ethers.js, viem, web3.js — works against
these endpoints without modification.

### The `kora_*` namespace

A small set of chain-specific methods complements the standard
surface. The shipped one is:

| Method | Returns |
|---|---|
| `kora_nodeStatus` | `currentView`, `finalizedCount`, `nullifiedCount`, `peerCount`, `isLeader` |

`currentView` increments every Simplex round; `finalizedCount` is the
count of finalized blocks (if it is incrementing, the network is
live); `nullifiedCount` counts views that ended without finalizing
(should be near zero in a healthy 4-validator devnet); `peerCount`
counts authenticated P2P connections; `isLeader` indicates whether
this node is the proposer for the current view.

Additional `kora_*` methods are designed and queued (explicit VRF-seed
retrieval, recent-blocks summary, finality-certificate export,
consensus health superset, active-agent enumeration). See
`05-roadmap.md`.

### What is not yet wired

- **WebSocket subscriptions (`eth_subscribe`)** are not yet
  implemented. Clients currently must poll `eth_getLogs` or
  `eth_blockNumber`. The underlying RPC library natively supports
  subscriptions; the wiring is a queued change rather than a research
  problem.
- **`eth_getProof`** is not implemented. The QMDB transition-hash
  state-root model does not directly support Ethereum-style Merkle
  inclusion proofs. The mitigation is the QMDB-proofs precompile
  covered in `02-precompiles-and-contracts.md`.

---

## Architectural Layers in One Diagram

```
                  +---------------------------------+
                  |   Nunchi blockchain (the node)  |
                  +---------------------------------+
                  |   JSON-RPC: eth_* + kora_*      |
                  +---------------------------------+
                  |   EVM plane (REVM + Solidity)   |
                  |     + custom precompiles        |
                  |       0x09: HDC similarity      |
                  |       0x0B: QMDB state proofs   |
                  |       0x0C: BTLE encryption     |
                  |       0xA10-0xA1F: agent ns     |
                  +---------------------------------+
                  |   Native plane (Rust)           |
                  |     - Simplex BFT consensus     |
                  |     - BLS12-381 threshold sigs  |
                  |     - Threshold VRF (prevrandao)|
                  |     - QMDB state + proofs       |
                  |     - Authenticated P2P         |
                  +---------------------------------+
                  |   Commonware primitives         |
                  |   (independent crates,          |
                  |    anti-framework composition)  |
                  +---------------------------------+
```

Everything above the native plane is what smart contracts see.
Everything below the EVM plane is what the node implementation
maintains. Precompiles are the bridge: they are called like any other
contract (at a reserved EVM address) but execute as native Rust with
full access to chain internals.

---

## Summary Table: Current Devnet Parameters

| Property | Value |
|---|---|
| Consensus algorithm | Simplex BFT (`threshold_simplex` from Commonware) |
| Signature scheme | BLS12-381 threshold (3-of-4 in devnet) |
| Finality | Single-slot (once finalized, never reverted) |
| EVM execution engine | REVM (same engine used by Foundry and Reth) |
| State database | QMDB (Commonware's authenticated key-value store) |
| Chain ID (devnet) | 1337 |
| Block time (devnet, observed) | ~400 ms |
| Block time (target) | ~50 ms via co-located validators |
| Gas limit per block | 30,000,000 |
| Transaction format | EIP-1559 |
| RPC namespaces | `eth_*`, `net_*`, `web3_*`, `kora_*` |
| P2P layer | Commonware `authenticated` overlay (Ed25519 identity, mutual authentication, encrypted) |
| DKG modes | Trusted-dealer (dev) + interactive Joint-Feldman (prod-like) |
| BLS group public key | 48 bytes |
| BLS threshold signature | 96 bytes |
| Finality certificate | ~240 bytes (group key + threshold sig + metadata) |

These are the load-bearing primitives. Everything else in the chain
— the precompiles, the contracts, the agent systems, the DeFi
settlement layer — is built on top of them.
