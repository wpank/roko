# Daeji: A Custom Application-Specific Blockchain Built on Commonware Primitives

## 1. What Daeji Is

Daeji (internal codename "Kora") is a minimal, application-specific blockchain node written in Rust. It is purpose-built as a shared knowledge ledger and witness-anchoring chain for AI agent coordination. The critical distinction: Daeji is **not** a fork of go-ethereum, Reth, or any existing blockchain client. It is assembled from scratch by composing three independent components:

1. **Simplex BFT consensus** (from the Commonware library) for block agreement across validators
2. **REVM** (Rust Ethereum Virtual Machine, the same engine used by Foundry and Reth) for executing standard Ethereum smart contracts
3. **QMDB** (Quick Merkle Database, from Commonware) for authenticated state storage

This architecture means Daeji is EVM-compatible -- any Solidity contract, any Ethereum tooling (Foundry, MetaMask, Alloy, ethers.js, Hardhat) works against it without modification -- but the underlying consensus and storage layers are entirely different from what powers Ethereum mainnet. The chain exposes standard `eth_*` JSON-RPC methods plus a custom `kora_*` namespace for chain-specific operations.

**How this differs from traditional EVM chains:** A traditional EVM chain like Ethereum, Polygon, or Arbitrum uses either proof-of-work or proof-of-stake consensus with individual validator signatures, Merkle Patricia Tries (MPT) for state storage, and a monolithic node implementation. Daeji replaces all of this: BLS12-381 threshold signatures replace individual validator signatures, QMDB replaces MPT, and the node is composed from small, independent library crates rather than forked from a monolithic codebase. This composability is what enables the novel cryptographic features described in section 4.

**Standard devnet configuration:**

| Property | Value |
|---|---|
| Validator nodes | 4 |
| Secondary (follower) nodes | 1 |
| Threshold | 3-of-4 (any 3 validators finalize a block) |
| Chain ID | 1337 |
| Block time | ~400ms |
| Gas limit per block | 30,000,000 |
| Transaction type | EIP-1559 |
| RPC namespaces | `eth_`, `net_`, `web3_`, `kora_` |
| Repository | `github.com/Nunchi-trade/daeji` |

**What Daeji is for:** The chain serves three concrete purposes for AI agents built with the roko toolkit:

1. **Shared agent knowledge** -- Agents post knowledge entries (observations, heuristics, warnings) to an on-chain InsightBoard contract. Other agents query this shared ledger and inject relevant prior knowledge into their system prompts before starting tasks.
2. **Tamper-evident work products** -- After an agent completes a task and passes validation gates (compile, test, lint, review), a cryptographic hash of the full episode record is posted on-chain, creating an immutable audit trail.
3. **Novel cryptographic features** -- Because Daeji uses Commonware's threshold cryptography, it provides capabilities unavailable on standard EVM chains: verifiable random functions (VRF) from threshold signatures, binding timelock encryption (BTLE), compact 240-byte finality certificates, and deterministic simulation for testing.

---

## 2. What Commonware Is

Commonware is a Rust library of independent, composable blockchain building blocks created by Patrick O'Grady (formerly of Ava Labs, the company behind the Avalanche blockchain). It is explicitly an "anti-framework": rather than providing a monolithic blockchain node that you fork and customize, it provides 17 independent crates -- each implementing one primitive -- that you compose however you need.

**Repository:** `github.com/commonwarexyz/monorepo`
**Version used by Daeji:** 2026.4.0
**License:** Dual MIT / Apache-2.0

### Key Primitives

| Crate | What It Provides |
|---|---|
| `commonware-cryptography` | Ed25519 digital signatures (for P2P node identity), BLS12-381 threshold signatures (for consensus finalization and VRF), Verifiable Random Functions (VRF, for bias-resistant randomness) |
| `commonware-p2p` | Two P2P networking implementations sharing the same trait: `authenticated` (real TCP with Ed25519 handshakes, for production) and `simulated` (in-process message bus, for deterministic testing) |
| `commonware-consensus` | Simplex BFT -- a Byzantine Fault Tolerant consensus protocol with single-slot finality |
| `commonware-storage` | QMDB (Quick Merkle Database, for authenticated key-value state storage) and MMR (Merkle Mountain Range, an append-only cryptographic audit log) |
| `commonware-runtime` | Two async runtime implementations sharing the same trait: `tokio` (production, real I/O) and `deterministic` (single-threaded simulator, controllable time, for testing) |
| `commonware-codec` | Wire format for encoding and decoding consensus messages |
| `commonware-broadcast` | Ordered broadcast for multi-sequencer scenarios (DSMR -- Decoupled State Machine Replication) |
| `commonware-resolver` | Pluggable content-addressed storage for large data |

### Why Composable Primitives Matter

The core philosophy: every crate is independent. You can use the consensus without the P2P. You can use the P2P without the storage. There is no "commonware node" binary -- you build your own node by composing the pieces you need.

This matters for three reasons:

1. **Feature access.** Each primitive exposes capabilities that are deeply integrated into the chain's operation. Threshold cryptography is not bolted on after the fact -- it is the consensus mechanism itself. This means features like VRF-based randomness and BTLE are natural byproducts of normal block production, not add-on services.

2. **Testing.** The dual-implementation pattern (real TCP vs. simulated message bus, tokio vs. deterministic runtime) means the entire multi-validator blockchain network can run in a single process with deterministic, reproducible behavior. Given the same random seed, every run produces identical results. This enables testing strategies that are impossible with monolithic chain implementations.

3. **Customization without forking.** Adding a custom precompile to geth requires forking 600K+ lines of Go code. In Daeji, it means registering a Rust function with REVM's precompile registry -- a few lines of code, no fork needed.

---

## 3. Chain Architecture

### Consensus: Simplex BFT

Daeji uses Commonware's `threshold_simplex` consensus protocol. "Simplex" refers to the protocol's simplicity: a block goes through three network message hops to be finalized -- propose, vote, finalize. Once finalized, the block is permanent (single-slot finality -- no probabilistic confirmation waiting like Bitcoin or pre-merge Ethereum).

**BFT properties:** The protocol tolerates up to `f` Byzantine (malicious or faulty) validators out of `n` total, where `n >= 3f + 1`. In the standard 4-validator devnet, up to 1 validator can be offline or malicious and the chain still makes progress.

**Threshold signatures:** The validator set collectively holds a BLS12-381 threshold key. Each validator holds a share of the private key. Any 3-of-4 validators can combine their partial signatures to produce a valid block finalization signature, but no individual validator (and no outside observer) ever knows the complete private key. The group public key is 48 bytes. A threshold signature is 96 bytes.

**All-to-all connections:** Validators connect to each other via Commonware's authenticated P2P overlay. Every connection is mutually authenticated using Ed25519 keys -- peers prove their identity on connection. There is no unauthenticated path into the network.

**VRF output:** At each finalized view (consensus round), the threshold signature over the view number serves as a VRF output: deterministic, unpredictable, unbiasable, and verifiable against the 48-byte group public key. This value is placed in the block's `prevrandao`/`mixHash` field, making it accessible to smart contracts via `block.prevrandao` in Solidity.

### Block Structure

Daeji's block structure is minimal:

```rust
pub struct Block {
    pub parent: BlockId,
    pub height: u64,
    pub prevrandao: B256,     // VRF seed from threshold consensus
    pub state_root: StateRoot,
    pub txs: Vec<Tx>,
}
```

This is deliberately leaner than Ethereum's block header, which contains 15+ fields. The `prevrandao` field holds the threshold VRF output (stronger randomness guarantees than Ethereum's RANDAO). The `state_root` is computed by QMDB.

### Execution Model: The Dual-Plane Approach

Daeji operates on two planes:

1. **EVM plane (smart contracts).** Standard Solidity contracts deployed and executed via REVM. Any Ethereum-compatible bytecode works without modification. This is where application logic lives: the InsightBoard knowledge ledger, the AgentRegistry for agent identity, the BountyMarket for task exchange. Transactions follow the EIP-1559 fee model.

2. **Native plane (chain internals).** Rust code compiled directly into the node binary. This is where custom precompiles live, where consensus produces threshold signatures and VRF outputs, where QMDB generates state proofs. The native plane has access to everything the EVM plane does not: the Merkle tree structure, historical state, consensus signatures, P2P message channels.

The precompile mechanism is the bridge between these planes: a Solidity contract calls a precompile address, the EVM hands off to native Rust code, which returns results back to the EVM context.

### Storage: QMDB

QMDB (Quick Merkle Database) is purpose-built for blockchain state storage, developed in collaboration with LayerZero. It addresses two requirements that conflict in standard databases:

- **Fast updates:** State changes with every block (potentially thousands of storage slot writes). Updates must complete in milliseconds. QMDB achieves O(1) SSD I/O per state update via an append-only write-ahead log.
- **Cryptographic commitment:** The entire state must be summarized as a single hash (the state root) per block. QMDB maintains the Merkle tree entirely in RAM, computing roots without disk reads.

**Important limitation:** QMDB currently computes state roots via a transition hash (`keccak256(parent_root + serialized_changes)`), not a full Merkle Patricia Trie root. This means `eth_getProof` is not available -- you cannot generate standard Ethereum-style Merkle inclusion proofs against the state root. This is the single largest architectural gap relative to standard Ethereum (see section 9 for mitigation strategies).

### Distributed Key Generation (DKG)

Daeji supports two DKG modes for establishing the shared threshold key:

- **Trusted-dealer mode:** One party generates the full key and distributes shares. Faster, simpler, but requires trusting the dealer. Used for development.
- **Interactive Joint-Feldman mode:** All parties contribute randomness, verify each other's contributions. No single party learns the secret. Suitable for production deployments where parties may not trust each other.

---

## 4. Novel Commonware Features

These eight features are uniquely enabled by Daeji's use of Commonware primitives. None of them are available on standard EVM chains.

### 4.1 Binding Timelock Encryption (BTLE) for Agent Commitments

BTLE allows data to be encrypted such that it can only be decrypted after a specific future consensus view is finalized. No trusted third party is involved. Decryption happens automatically as a byproduct of consensus.

**How it works:**

1. **Encrypt.** Take plaintext and a target future view number V. Using the group public key and V, perform a pairing-based Identity-Based Encryption (IBE) operation over BLS12-381. The ciphertext can only be decrypted by someone possessing the threshold signature over V -- which does not exist yet because V has not been finalized.

2. **Post.** Publish the ciphertext on-chain. It is visible to everyone but unreadable. The poster cannot revoke or modify it.

3. **Automatic reveal.** When the chain reaches view V, validators finalize it as part of normal consensus, producing the threshold signature over V. This signature is the decryption key. It becomes public as soon as the view is finalized.

4. **Decrypt.** Anyone can now decrypt using the threshold signature from view V. No "reveal" transaction needed. No participant takes any deliberate action.

The key advantage over traditional commit-reveal schemes: in commit-reveal, a participant can refuse to reveal if the outcome is unfavorable. In BTLE, decryption is automatic and cannot be withheld.

**Agent use cases:** Sealed-bid model selection among competing operators, commit-reveal for fair task claiming (preventing "claim sniping"), time-delayed knowledge reveals (first-mover advantage windows for discovering agents), and independent multi-agent verification for A/B experiments.

### 4.2 Threshold VRF for Unbiased Agent Selection

Every finalized block produces a bias-resistant random number derived from the BLS12-381 threshold signature. This is placed in the `prevrandao` field and accessible via `block.prevrandao` in Solidity.

**Properties (stronger than Ethereum's RANDAO):**

- **Bias-resistant:** No single validator controls the output. The threshold signature is deterministic for a given view number.
- **Unpredictable:** Until T validators produce their shares and the threshold signature is assembled, nobody can compute the output.
- **Verifiable:** Anyone can verify against the 48-byte group public key.
- **Free and per-block:** No oracle call, no additional transaction, no fee.

**Agent use cases:** Provably fair agent selection from qualified pools, verifiable A/B experiment assignment, VRF-seeded task shuffling for fairness, anti-gaming gate threshold jittering.

**Implementation status:** Already produced by consensus. Requires zero chain changes. Available today via `block.prevrandao` in Solidity or `eth_getBlockByNumber` off-chain.

### 4.3 Sub-Block-Time Agent Gossip (Separate from Chain Gossip)

Commonware's `p2p::authenticated` module enables a mesh network separate from the consensus P2P overlay. Agents can communicate directly without routing through the blockchain.

Each agent holds an Ed25519 keypair. When two agents connect, they perform a cryptographic handshake proving their identity. All subsequent messages are authenticated and encrypted.

**Agent use cases:** Real-time knowledge gossip between agents working on related tasks (sub-millisecond delivery vs. ~400ms chain finality), direct task coordination for parallel execution, dynamic fleet discovery.

This feature is independent of the chain -- it is a pure networking layer that agents use alongside on-chain interactions.

### 4.4 DKG-Based Private Multi-Agent Collaboration

The same DKG protocol that creates the validator threshold key can create ad-hoc threshold keys for groups of agents. A subset of agents can establish a shared secret without any single agent knowing the full key.

**Agent use cases:** Encrypted agent-to-agent channels where messages are decryptable only by the intended group, multi-party computation for joint knowledge distillation, privacy-preserving model evaluation where no single observer sees all results.

### 4.5 Proof-of-Work-Done and Proof-of-Learning

Daeji's finality certificates (see 4.7) can attest to on-chain records of completed work. The `ChainWitnessEngine` posts `blake3(episode_data)` to the chain after each task passes the gate pipeline. The finalized block containing this hash, combined with a finality certificate, constitutes a cryptographic proof that specific work was performed at a specific time.

Similarly, knowledge entries posted to the InsightBoard with confirmation counts from multiple independent agents constitute proof-of-learning: multiple agents independently verified that a piece of knowledge was correct and useful.

### 4.6 On-Chain Reputation with Threshold Attestation

Agent reputation scores (gate pass rates, efficiency metrics, knowledge contribution counts) stored on-chain are attested by the full validator set's threshold signature. This means an agent's reputation is not self-reported -- it is consensus-verified and tamper-evident.

**Agent use cases:** Portable agent reputation across deployments, reputation-gated task assignment, stake-weighted knowledge curation.

### 4.7 Light Clients via Threshold Signatures (Browser-Based)

A Commonware finality certificate contains approximately **240 bytes**:

- 48 bytes: the group public key (one BLS12-381 G1 point)
- 96 bytes: the threshold signature (one BLS12-381 G2 point)
- ~96 bytes: metadata (view number, state root, block hash)

Compare this to Ethereum's ~100KB finality proofs (512 validator public keys + participation bitmap + sync committee signatures + Merkle branches).

Verification is a single BLS12-381 pairing check: `e(signature, G2_generator) == e(H(message), group_public_key)`. This is one of the most well-optimized operations in blockchain cryptography.

**Agent use cases:** Gate verdict certification (prove "task X passed all 7 gate rungs at block N" to any external system in ~500 bytes), knowledge entry provenance proofs, portable agent reputation certificates, browser-based dashboards that verify chain state without trusting an RPC endpoint.

### 4.8 Native Knowledge Storage Precompiles

Daeji can host custom EVM precompiles -- native Rust functions at reserved addresses that execute inside the REVM interpreter. Three precompiles are designed for agent knowledge operations (see section 5 for full specifications). These enable operations that are computationally impossible in Solidity: searching 100,000 binary vectors by Hamming distance, generating historical Merkle proofs, and performing BLS12-381 pairing-based encryption.

---

## 5. Custom Precompiles

### Background: What a Precompile Is

A precompile (precompiled contract) is native code deployed at a fixed, well-known EVM address. From a calling contract's perspective, it looks identical to any other contract -- you `CALL` or `STATICCALL` its address with ABI-encoded input. Internally, the EVM hands the input bytes to a compiled Rust function instead of interpreting bytecode. Ethereum mainnet has 9 standard precompiles (0x01-0x09) for cryptographic operations like ECDSA recovery, SHA-256, and BN256 curve operations.

In REVM (the Rust EVM used by Daeji), precompiles implement the `Precompile` trait:

```rust
pub trait Precompile: Send + Sync {
    fn run(
        &self,
        input: &Bytes,      // Raw bytes from the CALL instruction
        gas_limit: u64,     // Gas budget
    ) -> PrecompileResult;  // Returns (gas_used, output_bytes) or error
}
```

### Precompile 0x09: HDC Similarity Search

**Address:** `0x09`
**Gas cost:** 50,000 (fixed, does not scale with entry count)
**Purpose:** Perform similarity search over all on-chain knowledge entries using Hyperdimensional Computing (HDC) vectors.

**Why it needs a precompile:** HDC vectors in this system are 10,240-bit binary vectors (1,280 bytes each, stored as `[u64; 160]`). Similarity is measured by Hamming distance: XOR two vectors and count the differing bits via hardware POPCNT. In native Rust with SIMD, scanning 100,000 entries takes ~170 microseconds. In Solidity, the same operation would cost ~400,000,000 gas at 10,000 entries -- exceeding 13 blocks' gas limits. At 100,000 entries, it is computationally impossible.

**Interface:**

```
Input:  [query_vector: 1280 bytes][top_k: uint8][filters: bytes]
Output: [(similarity: uint16, entry_id: bytes32, weight: uint256, proof: bytes)][]
```

Input fields:
- `query_vector` (1,280 bytes): The 10,240-bit binary hypervector to search for. Same format as `HdcVector::to_bytes()`: 160 little-endian u64 words.
- `top_k` (1 byte): Number of results to return (1-255).
- `filters` (variable, optional): `entry_type_mask` (uint32, bitmask: bit 0=Insight, 1=Heuristic, 2=Warning, 3=AntiKnowledge, 4=CausalLink, 5=StrategyFragment), `min_weight` (uint256), `max_age_blocks` (uint64).

Output fields (per result, sorted by descending similarity):
- `similarity` (uint16): 10,240 minus Hamming distance (higher = more similar). Range 0-10,240.
- `entry_id` (bytes32): Knowledge entry identifier in the InsightLedger contract.
- `weight` (uint256): Current weight (confidence/relevance score with decay).
- `proof` (bytes): Merkle inclusion proof against the current block's state root.

**Implementation:** The precompile maintains an in-memory `HdcIndex` rebuilt at block boundaries from InsightLedger contract state. The core loop is:

```rust
fn hamming_distance(a: &[u64; 160], b: &[u64; 160]) -> u32 {
    let mut dist = 0u32;
    for i in 0..160 {
        dist += (a[i] ^ b[i]).count_ones(); // Compiles to CPU POPCNT instruction
    }
    dist
}
```

**Dependency:** Requires the InsightLedger Solidity contract to be deployed first. The contract stores knowledge entries; the precompile reads from the same state storage slots.

**Contract-only alternative:** Tag-based retrieval (`mapping(bytes32 => bytes32[]) public entriesByTag`) works for small stores (hundreds of entries) but fails at scale and cannot do "find things conceptually similar to X."

### Precompile 0x0B: QMDB State Proofs

**Address:** `0x0B`
**Gas cost:** 30,000 (fixed)
**Purpose:** Generate Merkle inclusion or exclusion proofs for any storage key at any finalized block height.

**Why it needs a precompile:** The EVM provides smart contracts with a flat key-value view via `SLOAD`. A contract cannot access the Merkle tree structure underlying its storage, cannot access state from previous blocks, and cannot compute proofs against historical state roots. QMDB's internal structures are maintained by the node, not the EVM.

**Interface:**

```
Input:  [block_number: uint64][key: bytes32][proof_type: uint8]
Output: [exists: bool][value: bytes32][proof: bytes]
```

- `block_number` (uint64): Historical block to prove state at. Must be finalized.
- `key` (bytes32): Storage key to prove (follows standard EVM storage layout).
- `proof_type` (uint8): 0 = inclusion proof, 1 = exclusion proof.
- `proof` output: Serialized Merkle path verifiable against the state root in the block header.

**Dependency:** QMDB must be configured to retain historical Merkle tree data (not just current state). A pruning horizon (e.g., last 100,000 blocks) may be appropriate.

### Precompile 0x0C: BTLE Encryption/Decryption

**Address:** `0x0C`
**Gas cost:** 80,000 (both encrypt and decrypt)
**Purpose:** Native-speed BLS12-381 operations for Binding Timelock Encryption.

**Why it needs a precompile:** BLS12-381 pairing operations in pure Solidity cost 500,000-2,000,000 gas (381-bit modular arithmetic using 256-bit EVM words, Miller loop iterations, final exponentiation). In native Rust with the `blst` library: 1-2 milliseconds per pairing check.

**Interface:**

Encrypt (callable at any time):
```
Input:  [operation: uint8 = 0x00][target_view: uint64][plaintext: bytes]
Output: [ciphertext: bytes]
```

Decrypt (callable only after target_view is finalized):
```
Input:  [operation: uint8 = 0x01][target_view: uint64][ciphertext: bytes]
Output: [plaintext: bytes]
```

Ciphertext format: `[C1: 48 bytes (G1 point)][nonce: 12 bytes][encrypted_data: variable][auth_tag: 16 bytes]`. Uses a hybrid scheme: IBE for key exchange, ChaCha20-Poly1305 for bulk data encryption.

The encrypt operation derives the encryption key from the chain's group public key and the target view number via hash-to-G1 and a pairing computation. The decrypt operation looks up the VRF output (threshold signature) for the target view and uses it as decryption key material. If the target view is not yet finalized, decrypt reverts.

### Agent Communication Namespace (0xA10-0xA1F)

A reserved address range for future agent-specific precompiles. This namespace is designed to accommodate operations like direct agent-to-agent messaging verification, reputation attestation verification, and task-claim arbitration.

---

## 6. Agent-Chain Mapping

### What Goes On-Chain vs. Off-Chain

The system uses a hybrid architecture. Not everything belongs on the blockchain -- the design deliberately minimizes on-chain footprint.

**On-chain (the "anchor"):**

Per knowledge entry, approximately 71 bytes of contract storage:
- `contentHash` (bytes32, 32 bytes): BLAKE3 hash of the full content
- `poster` (address, 20 bytes): Ethereum address of the posting agent
- `timestamp` (uint64, 8 bytes): When the entry was posted
- `pheromone` (uint64, 8 bytes): Number of confirmations from other agents
- `entryType` (uint8, 1 byte): Which of the 6 knowledge types
- `halfLifeHrs` (uint16, 2 bytes): Decay rate in hours

Additionally, full content is emitted in event logs during the posting transaction (cheap to write, readable via `eth_getLogs`, not in mutable state).

**Off-chain (local to each agent):**

Each agent maintains its local neuro store -- an append-only JSONL file with the full `KnowledgeEntry` struct (~2-3 KB per entry) including: HDC vector (1,280 bytes), confidence score, tier, emotional tags, catalytic score, and all metadata.

HDC vectors are computed locally by each agent from entry content text -- never stored on-chain (at ~770,000 gas per entry for the 1,280-byte vector, on-chain storage would be prohibitively expensive).

### Knowledge Posting Flow

1. Agent completes a task and passes all 7 gate rungs (Compile, Lint, Test, Symbol, GeneratedTest, PropertyTest, LLM Judge).
2. The distillation hook examines the episode and extracts knowledge entries.
3. Entries meeting the promotion threshold (confidence >= 0.70, 3+ local confirmations, Consolidated or Persistent tier) are pushed to the InsightBoard contract on Daeji.
4. The transaction includes the content hash, entry type, half-life, and confidence. Full content is emitted as an event log.
5. Other agents discover new entries via `eth_getLogs`, compute HDC vectors locally from the content, and ingest them into their local neuro store at initial confidence 0.5 and Transient tier.

### Episode Anchoring Flow

1. Agent completes a task.
2. Gate pipeline validates the output.
3. `ChainWitnessEngine` computes `blake3(episode_json_bytes)`.
4. Submits a transaction to Daeji with calldata prefixed by `b"roko.attestation.witness:"` followed by the hash.
5. Transaction is mined and finalized.
6. The episode record gains a `chain_attestation` field: `{chain_id, tx_hash, block_number}`.

### Identity Registration

Agents register with the AgentRegistry contract by calling `register(ed25519Pubkey, capabilities)`. Periodic heartbeat transactions (`heartbeat()` every ~15 minutes) prove liveness. The registry provides on-chain discovery: any agent can query for other registered agents, their capabilities, and their last heartbeat timestamp.

### Confirmation Flow

When Agent B uses knowledge from the InsightBoard during task execution and the task succeeds (passes all gates), Agent B automatically confirms the entry on-chain by calling `confirm(entryId)`. This increments the entry's pheromone counter, increasing its weight for future retrievals. Entries that consistently prove useful across independent agents climb tiers; entries that are never confirmed decay and fade.

---

## 7. The Daeji Repo and Current State

### What Exists Today

The Daeji repository (`github.com/Nunchi-trade/daeji`) contains a working blockchain node with the following capabilities:

| Capability | Status |
|---|---|
| Simplex BFT consensus with BLS12-381 threshold signatures | Running |
| REVM execution (standard Ethereum bytecode) | Running |
| QMDB state storage | Running |
| 4-validator devnet via Docker Compose | Running |
| Secondary (follower) peer replication | Running |
| Standard `eth_*` JSON-RPC | Running |
| Custom `kora_nodeStatus` RPC | Running |
| VRF output in `prevrandao` field | Running |
| DKG (both trusted-dealer and Joint-Feldman) | Running |
| End-to-end test harness (deterministic simulation) | Running |
| Devnet tooling (`just trusted-devnet`, `just loadgen`, `just devnet-reset`) | Running |

### What Is Spec'd But Not Yet Built

| Feature | Status |
|---|---|
| Custom precompile registry (replacing `build_mainnet()`) | Spec'd, not implemented |
| HDC similarity search precompile at 0x09 | Spec'd, not implemented |
| QMDB state proof precompile at 0x0B | Spec'd, not implemented |
| BTLE encryption precompile at 0x0C | Spec'd, not implemented |
| WebSocket subscription support (`eth_subscribe`) | Spec'd, not implemented |
| Extended `kora_*` RPC namespace (`kora_vrfSeed`, `kora_recentBlocks`, `kora_consensusHealth`) | Spec'd, not implemented |
| Validator set resharing | Protocol exists in Commonware, not integrated into Daeji |
| DSMR / ordered broadcast integration | Crate exists in Commonware, not integrated |

### Known Issues in Current Daeji

Two critical bugs must be fixed before deploying Solidity contracts:

1. **`block.timestamp` is set to block height, not wall-clock Unix time.** At block 1000, `block.timestamp` returns `1000`, not a Unix timestamp. This breaks all Solidity timing logic: cooldown periods, timelocks, knowledge decay calculations. Fix: one-line change in `app.rs` to use `SystemTime::now()`.

2. **`BLOCKHASH` opcode returns zero for all inputs.** Smart contracts that call `blockhash(blockNumber)` get zero. Fix: add a ring buffer (`BlockHashCache`) storing the last 256 block hashes, pass lookups to this buffer instead of returning zero.

### Solidity Contracts (Built, Not Yet Deployed to Daeji)

Ten contracts exist in the roko repository, built and tested against local Anvil/mirage-rs:

| Contract | Purpose |
|---|---|
| `AgentRegistry.sol` | Agent identity, capabilities, heartbeat, liveness tracking |
| `InsightBoard.sol` | Knowledge curation: post content hashes, confirm entries, pheromone weighting |
| `IdentityRegistry.sol` | Soulbound identity passport -- 4 tiers, staking, timelocks |
| `ReputationRegistry.sol` | 7-domain exponential moving average reputation scores |
| `ValidationRegistry.sol` | Work proof + validator attestation |
| `BountyMarket.sol` | Bounty marketplace for cross-agent task exchange |
| `WorkerRegistry.sol` | Worker staking |
| `ConsortiumValidator.sol` | Consortium validation |
| `FeeDistributor.sol` | Fee distribution |
| `MockERC20.sol` | DAEJI test token (standard ERC-20) |

These contracts are chain-agnostic -- they compile to standard EVM bytecode and can be deployed to Daeji's RPC endpoint without modification.

---

## 8. Open Questions and Design Decisions

### 8.1 Token Economics

Should Daeji have a custom token with demurrage (balances that decay over time), or should agents use plain ETH? A `KoraiToken` implementation with lazy demurrage exists in Rust (`roko-chain/src/korai_token.rs`) but has never been deployed as a Solidity contract. The `MockERC20.sol` contract is a plain ERC-20 with no decay mechanics.

The core tension: a token enables fine-grained economic incentives (pay to post knowledge, earn for confirmations, stake for reputation), but adds complexity. For a single-operator development network, plain ETH is simpler. The token matters when multiple independent operators need real economic alignment.

**Sub-questions:** Minting policy (fixed supply vs. faucet vs. emission schedule with halving), demurrage implementation (lazy compute-on-read vs. eager decay every transfer), whether the on-chain token should mirror or replace the existing USD budget system in the orchestrator.

### 8.2 Knowledge Entry Storage Strategy

How much knowledge data should live on-chain? Three axes of decision:

- **Inline vs. external content:** Full text in contract storage (~600 gas/byte, ~600,000 gas for a 1,000-byte entry) vs. content hash only + full text in event logs (~50x cheaper).
- **HDC vectors on-chain vs. off-chain:** On-chain enables the HDC precompile to read directly from state but costs ~770,000 gas per entry (1,280 bytes). Off-chain means agents compute vectors locally.
- **Entry pruning:** No pruning (state grows forever), deterministic consensus-level pruning (complex to implement correctly), or lazy pruning (nodes prune local index but chain state retains everything).

Current recommendation: content hash on-chain, full content in events, HDC vectors off-chain, no pruning initially.

### 8.3 Agent Identity Model

Agents need two types of cryptographic identity:

- **secp256k1 key** for signing Ethereum transactions (interacting with Daeji contracts)
- **Ed25519 key** for authenticating to the P2P network (if running as a secondary peer or using Commonware P2P for agent-to-agent messaging)

Options: single Ed25519 key with a derived secp256k1 address (simpler management, one key backup), separate keys for each purpose (cleaner security boundary), or secp256k1 only (limits P2P capabilities). Currently, agents have no cryptographic identity at all -- they are identified by string names in `roko.toml`.

### 8.4 Secondary Peer vs. RPC Client

Should roko agents run a full secondary (follower) peer that joins the Daeji P2P network, or use a simple JSON-RPC HTTP client? The secondary peer provides real-time block delivery via `LedgerEvent` stream (no polling), direct P2P connectivity to validators, and lower latency. The RPC client is simpler, requires no P2P key management, and works with any Ethereum-compatible endpoint.

### 8.5 Precompile vs. Contract Boundary

At what knowledge entry count does HDC search need to transition from a contract-based approach (tag-based retrieval) to the native precompile? Estimates suggest the crossover is around 1,000-10,000 entries. Below 1,000, tags work fine. Above 10,000, Solidity-based vector search exceeds block gas limits.

### 8.6 Commonware Version Tracking

Must roko and Daeji use the same Commonware version? If roko imports Commonware crates directly (for Ed25519 agent identity, deterministic runtime for tests, P2P for agent mesh), version mismatches could cause wire-format incompatibilities. Options: monorepo (same workspace), pinned version (both pin to the same Commonware release), or separate repos with compatibility testing.

---

## 9. What Breaks and Risk Areas

### Critical: block.timestamp Bug

`block.timestamp` is set to block height, not Unix time. This breaks 8 Solidity contracts that use time-based logic. Impact examples:

- IdentityRegistry: `PROMPT_UPDATE_DELAY = 1 days` (86,400 seconds) never expires because height increments by 1, not 86,400.
- ReputationRegistry: EMA decay uses `(block.timestamp - lastUpdate) / DECAY_PERIOD` -- produces near-zero elapsed time.
- InsightBoard: Knowledge decay formula computes elapsed blocks instead of elapsed seconds, producing wildly wrong weights.

**Fix:** One-line change to use `SystemTime::now()` for timestamp. Low risk (validators tolerate slight clock drift).

### Critical: BLOCKHASH Returns Zero

The `block_hash_ref` closure always returns `B256::ZERO`. Breaks VRF seed derivation, commit-reveal schemes, and audit trail references that combine `prevrandao` with recent block hashes.

**Fix:** Add a `BlockHashCache` ring buffer storing the last 256 block hashes. Low risk, well-understood data structure.

### High: No Custom Precompile Registration

The executor calls `build_mainnet()` which loads only standard precompiles 0x01-0x09. No mechanism to add custom precompiles without modifying Daeji source. This blocks the HDC search precompile (the only operation that truly requires native code).

**Fix:** Replace `build_mainnet()` with a `KoraPrecompiles` registry that includes standard precompiles plus custom registrations.

### High: No WebSocket Subscriptions

`eth_subscribe` (the standard Ethereum mechanism for push-based event notifications) is not implemented. Agents must poll via `eth_getFilterChanges` or `eth_getLogs`, adding latency and load.

**Fix:** Add `#[subscription]` attributes to the jsonrpsee RPC trait implementation. Medium effort.

### Medium: QMDB State Root Is Not an MPT Root

QMDB's transition hash is not a Merkle Patricia Trie root. This means:

- No `eth_getProof` RPC method
- No standard Ethereum Merkle inclusion proofs
- Cross-chain verification requires either trusting the RPC endpoint or deploying an on-chain MMR (Merkle Mountain Range) contract for proof generation

**Mitigation:** Deploy a Commonware MMR contract for knowledge proofs. Accept the QMDB transition hash for everything else. This gives proofs where they matter (knowledge entries, witness anchors) without invasive chain modifications.

### Medium: HDC Vector On-Chain Cost

Storing a 1,280-byte HDC vector on-chain costs ~770,000 gas per entry. At the 30M gas block limit, at most ~39 entries with vectors can be posted per block.

**Mitigation:** Store vectors off-chain (computed locally by each agent from content text). The HDC precompile maintains its own in-memory index rebuilt at block boundaries.

### Low: Free Gas (No Real EIP-1559 Base Fee)

Daeji currently has no meaningful base fee mechanism. Transactions are effectively free. This removes spam protection -- any agent can flood the chain with zero-cost transactions. Acceptable for a private devnet; problematic if the network opens to untrusted participants.

### Low: Coinbase Always Zero

The `block.coinbase` field is always zero. No validator receives a reward address. This is cosmetic for now but would need fixing for any economic model.

---

## 10. What Kinds of Commonware Things Should Be Built

Based on the novel features, agent use cases, and current gaps, these are the highest-value Commonware primitives or chain features to build next, ordered by impact.

### 10.1 An MMR-Based State Proof Contract (Highest Priority)

The single biggest architectural gap is the inability to generate Merkle proofs against QMDB's state root. Commonware ships `commonware-storage::mmr` -- a Merkle Mountain Range implementation. Deploying this as a smart contract that agents append to on every knowledge post would give cryptographic proofs where they matter most: knowledge entry existence, witness anchor verification, and cross-fleet trust. This is the bridge between "trust the RPC endpoint" and "cryptographically verify."

### 10.2 A BTLE Rust Library

The cryptographic primitives for BTLE exist in Commonware's BLS12-381 implementation, but no standalone BTLE encryption/decryption crate exists yet. Building one (IBE encrypt/decrypt using BLS12-381 pairings, targeting future view numbers) would unlock sealed-bid model selection, fair task claiming, and time-delayed knowledge reveals. This is the highest-novelty feature in the Daeji stack.

### 10.3 Historical State Proof RPC Method

A `daeji_getStateProof(blockNumber, key)` RPC method that returns a QMDB Merkle proof against a specific historical block's state root. This does not require a precompile -- it can be an off-chain RPC endpoint. Combined with the MMR contract, this enables full trustless verification of any on-chain record.

### 10.4 Agent P2P Message Protocol

Commonware's P2P library handles connections and authentication. What is missing is a typed message protocol for agent-to-agent communication: knowledge gossip messages, task handoff signals, fleet discovery broadcasts, heartbeat messages. Defining this protocol (message types, serialization format, gossip semantics) would enable sub-block-time agent coordination.

### 10.5 Deterministic Test Harness as a Library

Daeji's end-to-end test infrastructure (`crates/e2e/`) runs a full multi-validator network in a single process with deterministic behavior. Exposing this as a library (adding a `[lib]` target to the crate) would let external code (like roko's gate pipeline) spin up a simulated Daeji network for testing agent-chain interactions -- no Docker, no network setup, runs in milliseconds as a Rust unit test.

### 10.6 WebSocket Event Streaming

Adding `eth_subscribe` support via jsonrpsee `#[subscription]` attributes would eliminate the need for polling. This is particularly important for the knowledge sync loop: agents need to learn about new InsightBoard entries as soon as they are mined, not after a polling interval.

### 10.7 Certificate Export API

Commonware's consensus already produces finality certificates (threshold signatures over finalized blocks). What is missing is an API to export these certificates in a format that external verifiers can consume. A new RPC method (`kora_getCertificate(blockNumber)`) returning the 240-byte certificate would enable all cross-chain and light-client use cases.

### 10.8 DSMR Integration for Streaming Knowledge (Longer-Term)

Commonware's `broadcast` crate implements ordered broadcast (DSMR). Integrating this into Daeji would allow each agent to act as its own sequencer, broadcasting knowledge entries and telemetry in real-time. The consensus layer would only order tip references, not the full messages. This is a major architectural change but would dramatically increase throughput for knowledge-heavy workloads.

### What Is Missing from the Current Design

- **No cross-chain bridge primitives.** The 240-byte certificates are produced but there is no relay mechanism or verifier contract on a target chain. Building a minimal BLS12-381 pairing verifier contract (deployable to any EVM chain with BN256 precompiles or EIP-2537 support) would unlock cross-chain verification.
- **No agent key management standard.** Agents need both Ed25519 (P2P) and secp256k1 (transactions) keys. There is no key generation, storage, rotation, or backup protocol defined.
- **No knowledge garbage collection on-chain.** Entries decay but are never removed from contract storage. A deterministic pruning mechanism (consensus-level, not per-node) would prevent unbounded state growth.
- **No economic model for knowledge quality.** The confirmation/pheromone mechanism exists but there is no cost to posting bad knowledge and no reward for posting good knowledge (beyond the token question in section 8.1).

---

## 11. Knowledge Layer Redesign

The knowledge layer redesign defines how agent-generated knowledge flows between the off-chain neuro store and the on-chain InsightBoard, creating a shared, validated repository of operational knowledge.

### The Problem: Siloed Agent Knowledge

AI agents produce enormous amounts of operational knowledge during task execution: what works, what fails, which approaches are effective for which problems. In a typical setup, this knowledge is:

1. **Ephemeral** -- stored in local files, lost on disk wipe or instance reset.
2. **Siloed** -- invisible to agents on other machines. Each fleet independently rediscovers what others already know.
3. **Unverified** -- no mechanism to distinguish good knowledge from noise without independent validation.

### The Hybrid On-Chain/Off-Chain Architecture

The redesign uses the hybrid split described in section 6:

- **On-chain:** Minimal metadata (71 bytes per entry: content hash, poster, timestamp, pheromone count, entry type, half-life).
- **Events:** Full content text emitted in transaction logs during posting.
- **Off-chain:** Complete `KnowledgeEntry` struct with HDC vectors, confidence scores, tier levels, emotional tags, and all rich metadata.

### Bidirectional Sync Protocol (NeuroChainSync)

**Push (local to chain):** Entries meeting promotion threshold (confidence >= 0.70, 3+ local confirmations, Consolidated or Persistent tier) are pushed to the InsightBoard contract. The transaction includes content hash, entry type, half-life, and confidence. Full content is emitted as an event.

**Pull (chain to local):** Agents scan `eth_getLogs` for new `InsightPosted` events from other agents. For each new entry, the agent: (a) fetches the full content from the event log, (b) computes the HDC vector locally, (c) ingests the entry into the local neuro store at initial confidence 0.5 and Transient tier, (d) subjects it to anti-knowledge conflict detection (HDC similarity > 0.9 against existing AntiKnowledge entries triggers rejection).

### Knowledge Entry Lifecycle

1. **Local discovery.** Agent discovers a fact during task execution (e.g., "this API requires Tokio 1.38+").
2. **Local storage.** Entry is stored in the neuro store as a Transient-tier entry with initial confidence.
3. **Local confirmation.** If the entry proves useful in subsequent local tasks (the task succeeds and passes all gates), its confirmation count increments and it may climb tiers (Transient -> Working -> Consolidated -> Persistent).
4. **Chain promotion.** Once the entry reaches Consolidated tier with sufficient confirmations and confidence, it is posted to the InsightBoard.
5. **Cross-agent discovery.** Other agents pull the entry from chain event logs.
6. **Cross-agent confirmation.** If the entry proves useful to other agents, they confirm it on-chain (incrementing the pheromone counter).
7. **Decay.** Entries that are not re-confirmed decay according to their half-life and tier multiplier. The formula: `weight = initial_weight * 0.5^(age / (half_life * tier_multiplier))`.

### Context Assembly Pipeline

Before each task dispatch, the `ContextAssembler` queries the knowledge store and assembles relevant entries for the agent's system prompt. The pipeline has 5 stages:

1. **Query:** Retrieve 50-200 candidate entries via HDC similarity search against the task description.
2. **Filter:** Remove entries below minimum confidence/weight thresholds. Apply active inference (Bayesian filtering based on the task domain).
3. **Rank:** Score remaining entries by a composite: HDC similarity (40%) + keyword relevance (30%) + predictive foraging utility (20%) + freshness (10%).
4. **Compress:** Select top entries that fit within a token budget (~800 tokens). Apply same-source diminishing returns (18% discount per same-source entry).
5. **Arrange:** Position entries following U-shaped attention research: most important entries at the very beginning and very end of the context block (where LLMs pay most attention); less critical entries in the middle.

### Predictive Foraging

Every knowledge retrieval is framed as a falsifiable prediction registered before task execution begins. The agent predicts "using entries A, B, C will help complete this task with score X in Y minutes." After execution, an external verifier (compiler, test suite, linter -- never the LLM itself) determines the actual outcome. The residual (predicted vs. actual) calibrates future retrieval. Entries that appear in successful predictions gain weight; entries that appear in failed predictions lose weight. This creates a self-correcting system where knowledge quality improves over time without human curation.

### Six Knowledge Types with Half-Lives

| Kind | Description | Off-Chain Half-Life | On-Chain Half-Life |
|---|---|---|---|
| Insight | Factual observation | 30 days | 7 days |
| Heuristic | Behavioral rule | 90 days | 15 days |
| Warning | Urgent transient condition | 1 hour | 3 minutes |
| AntiKnowledge | What failed / what to avoid | 30 days | 15 days |
| CausalLink | Cause-effect relationship | 60 days | 15 days |
| StrategyFragment | Reusable approach fragment | 14 days | 15 days |

### Four Retention Tiers

| Tier | Weight Multiplier | Promotion Threshold |
|---|---|---|
| Transient | 0.1x | (initial) |
| Working | 0.5x | 2+ confirmations |
| Consolidated | 1.0x | 3+ distinct contexts, confidence >= 0.70 |
| Persistent | 5.0x | Multiple independent cross-agent confirmations |

The on-chain half-lives are shorter than off-chain because the chain serves as a high-velocity shared memory -- entries need to prove their value quickly across multiple agents or fade. The off-chain store is more permissive because local knowledge does not impose costs on other agents.
