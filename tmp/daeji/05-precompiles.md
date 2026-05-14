# Custom EVM Precompiles for Daeji

This document specifies three custom EVM precompiles for the daeji chain. Each
precompile is explained from first principles: what the underlying technology is,
why it cannot (or should not) be implemented as a standard Solidity smart contract,
and how the precompile works.

Each precompile exists to serve a specific need of the roko agent system. Roko is a
self-developing Rust toolkit (18 crates, ~177K lines of code) whose core loop is:
read a PRD, generate an implementation plan, dispatch LLM-powered agents to execute
tasks, validate results through a gate pipeline, persist outcomes, and learn from
them. As agents complete work, they generate knowledge -- observations about what
worked, what failed, and what patterns to reuse. That knowledge needs to be stored,
searched, and proven. These three precompiles make that possible on-chain at speeds
that Solidity cannot achieve.

---

## Background: What an EVM Precompile Is

The Ethereum Virtual Machine (EVM) is a stack-based virtual machine that executes
smart contract bytecode. When a smart contract is deployed, its source code (typically
written in Solidity) is compiled into EVM bytecode -- a sequence of low-level
instructions (opcodes) that the EVM interpreter executes one at a time.

EVM execution is metered by **gas**: each opcode has a gas cost, and a transaction
specifies a gas limit. If execution exceeds the limit, the transaction reverts.
Gas serves as both a denial-of-service protection mechanism and a fee model (users
pay gas fees to compensate validators for computation).

A **precompile** (also called a "precompiled contract") is a piece of native code
deployed at a fixed, well-known address. From the calling contract's perspective, a
precompile looks identical to any other contract: you `CALL` or `STATICCALL` its
address with ABI-encoded input data and receive ABI-encoded output data. The
difference is entirely internal to the EVM: instead of interpreting bytecode at that
address, the EVM hands the input bytes to a native (compiled) function, which returns
output bytes.

**Why precompiles exist.** Some operations are either impossible or prohibitively
expensive to implement in EVM bytecode:

- **Cryptographic operations** (elliptic curve pairings, modular exponentiation):
  These involve modular arithmetic over large prime fields. Implementing them in
  Solidity requires emulating big-integer math with 256-bit EVM words, costing
  millions of gas. As native code, they complete in microseconds.
- **Operations requiring chain internals**: Some data (like the state database's
  internal Merkle tree structure) is not accessible from the EVM execution context.
  A precompile runs inside the node implementation and can access anything the node
  has access to.

Ethereum mainnet has 9 standard precompiles at addresses 0x01 through 0x09:

| Address | Name | Purpose |
|---------|------|---------|
| 0x01 | ECRECOVER | Recover Ethereum address from ECDSA signature |
| 0x02 | SHA-256 | SHA-256 hash function |
| 0x03 | RIPEMD-160 | RIPEMD-160 hash function |
| 0x05 | MODEXP | Modular exponentiation (used in RSA verification) |
| 0x06-0x08 | BN256 operations | Elliptic curve operations for zkSNARK verification |

Custom chains can add their own precompiles at addresses beyond these.

---

## Background: What REVM Is

REVM is an EVM implementation written in Rust. It is used by several major projects:

- **Daeji**: as the EVM execution engine for smart contract processing.
- **Foundry** (Forge, Cast, Anvil): the most widely-used Solidity development and
  testing toolkit.
- **Reth**: a production Ethereum execution client (full node).

REVM is modular: its execution context is parameterized over a `Database` trait
(for state access) and a precompile set (for custom native operations). Adding a
custom precompile means writing a Rust struct that implements REVM's `Precompile`
trait and registering it at a chosen address:

```rust
// The REVM precompile trait (simplified):
pub trait Precompile: Send + Sync {
    fn run(
        &self,
        input: &Bytes,      // Raw bytes from the EVM CALL instruction
        gas_limit: u64,     // Gas budget available for this call
    ) -> PrecompileResult;  // Returns (gas_used, output_bytes) or an error
}
```

The precompile receives raw input bytes (ABI-encoded by the calling Solidity
contract), performs its computation using native Rust code with full access to the
node's state and internal data structures, and returns raw output bytes (which the
calling contract ABI-decodes).

Registration happens during executor initialization:

```rust
let mut precompiles = ContextPrecompiles::default(); // standard 0x01-0x09
precompiles.extend([(
    Address::from_low_u64_be(0x09),                    // custom address
    Arc::new(MyCustomPrecompile::new(node_state)),     // Rust implementation
)]);
```

---

## Precompile 0x09: HDC Similarity Search

### The Problem This Solves

Roko agents generate knowledge as they work. When an agent completes a task and the
gate pipeline passes, the system distills what the agent learned into a
`KnowledgeEntry` -- a structured record stored in the neuro knowledge store at
`.roko/neuro/knowledge.jsonl`. There are six kinds of knowledge:

| Kind | What it captures | Default half-life |
|------|-----------------|-------------------|
| **Insight** | A compact causal observation distilled from multiple raw episodes. Example: "Tokio runtime must be multi-threaded for this test suite." | 30 days (off-chain) / 7 days (on-chain) |
| **Heuristic** | A rule of thumb or learned tendency. Example: "Split files over 500 lines before refactoring." | 90 days / 15 days |
| **Warning** | A cautionary note about a failure mode. Example: "Never use --force-push on shared branches." | 1 hour / 3 minutes |
| **AntiKnowledge** | Negative knowledge -- what to avoid, what has failed, what was once believed true but is now known false. Includes a reference to the refuted insight. | 30 days / 15 days |
| **CausalLink** | A causal relationship between two observations. Example: "Increasing batch size causes OOM on CI runners." | 60 days / 15 days |
| **StrategyFragment** | A reusable approach fragment composable into a larger plan. Example: "For migration tasks: backup, migrate, validate, rollback-test." | 14 days / 15 days |

Each knowledge entry carries:

- A **confidence** score (0.0 to 1.0).
- A **retention tier** (Transient, Working, Consolidated, or Persistent), which multiplies the half-life by 0.1x, 0.5x, 1.0x, or 5.0x respectively.
- **Source episodes** -- the episode IDs that contributed to this knowledge.
- **Tags** for categorical retrieval.
- A **balance** that decreases via demurrage over time and increases when the entry is retrieved, cited, or quoted.
- A **catalytic score** tracking how many new knowledge entries this one helped create.
- An **HDC fingerprint** -- a 10,240-bit binary vector encoding the entry's semantic content.

The HDC fingerprint is what makes similarity search possible. When a new agent is
dispatched for a task, roko queries the knowledge store for entries semantically
relevant to that task and injects them into the agent's 9-layer system prompt.
Today this happens off-chain (the `NeuroStore` in the `roko-neuro` crate reads
`.roko/neuro/knowledge.jsonl` and does the search in-process). On-chain, with
thousands of knowledge entries posted by many agents, the same search must happen
inside the EVM. That is what this precompile does.

### Background: What Hyperdimensional Computing (HDC) Is

Hyperdimensional Computing (HDC), also called vector symbolic architectures, is a
computational framework that represents information as high-dimensional binary
vectors. The core principles:

- Each concept, entity, or piece of knowledge is encoded as a **binary vector** of
  fixed length. In roko: **10,240 bits** (1,280 bytes), stored in Rust as
  `[u64; 160]` (the `HdcVector` type in `roko-primitives/src/hdc.rs`).

- **Similarity** between two concepts is measured by **Hamming distance**: the
  number of bit positions where two vectors differ. For 10,240-bit vectors: distance
  0 = identical, distance ~5,120 = unrelated (the expected distance between two
  random vectors), distance < ~2,048 = strong semantic similarity.

- Vectors are **combined** using bitwise operations:
  - **XOR bind** (`bind(&self, other)`) ties two concepts together into a pair representation.
    It is its own inverse: `bind(bind(a, b), b) == a`.
  - **Majority-vote bundle** (`bundle(vectors)`) merges multiple concepts into a
    single vector representing the set. At each bit position, the majority value
    among the input vectors wins.
  - **Cyclic permutation** (`permute(n)`) encodes sequence position (the n-th
    element of a list gets rotated by n before bundling).

- All operations are CPU-cache-friendly bit manipulation. No floating point, no
  matrix multiply, no GPU required.

**How roko generates HDC vectors.** Text content is encoded through a deterministic
pipeline:

1. Each byte of the input text seeds a pseudo-random `HdcVector` via FNV-1a hashing
   and splitmix64 expansion (`HdcVector::from_seed`).
2. Byte-level vectors are bound and permuted to capture sequence information.
3. The result is bundled via majority vote into a single 10,240-bit fingerprint.

This happens in `roko-learn/src/hdc_fingerprint.rs` (for episodes) and
`roko-primitives/src/hdc.rs` (for the core vector operations). The same encoding
is used for episode fingerprints, knowledge entry fingerprints, code pattern
matching in `roko-index`, and engram metadata in `roko-core`.

### Why HDC Similarity Search Cannot Be Done in Solidity

Computing Hamming distance requires two steps:

1. **XOR** the two vectors (producing a vector where each 1-bit marks a position
   where the vectors differ).
2. **Count the 1-bits** in the result (this operation is called "population count"
   or POPCNT).

**The POPCNT problem.** Modern CPUs have a hardware instruction (`POPCNT`) that
counts the 1-bits in a 64-bit word in a single clock cycle. The EVM has no
equivalent. Solidity's largest native integer type is `uint256` (256 bits). To count
1-bits in a `uint256`, you must implement the Hamming weight algorithm using a
series of shifts, masks, and additions -- approximately 50-100 gas per `uint256`
word.

**The scale problem.** A 10,240-bit vector spans 40 `uint256` words. One Hamming
distance computation: 40 XOR operations (~120 gas) plus 40 POPCNT emulations
(~2,000-4,000 gas). For a single comparison, this is manageable.

For a **similarity search** over the entire knowledge base:

| Entry count | Gas cost (Solidity) | Ethereum block gas limit (30M) |
|-------------|--------------------|---------------------------------|
| 1,000 | ~4,000,000 | Fits in one block |
| 10,000 | ~40,000,000 | Exceeds block limit |
| 100,000 | ~400,000,000 | Exceeds 13 blocks |

At 10,000 entries, the operation exceeds a single block's gas limit. It is
physically impossible to execute in one transaction.

**In native Rust**: roko's `HdcVector::similarity` method in
`roko-primitives/src/hdc.rs` computes Hamming distance as:

```rust
pub fn similarity(&self, other: &Self) -> f32 {
    let mut differing_bits = 0u32;
    for (left, right) in self.bits.iter().zip(other.bits.iter()) {
        differing_bits += (left ^ right).count_ones();
    }
    1.0_f32 - (f32::from(differing_bits) / 10_240.0_f32)
}
```

The `.count_ones()` call compiles to the CPU's native `POPCNT` instruction. With
AVX-512 (available on recent x86 processors), the compiler can vectorize this to
process 512 bits per instruction cycle. Scanning 100,000 entries takes approximately
**170 microseconds** on commodity server hardware.

The gap between "impossible in Solidity" and "170 microseconds in native code"
is why this requires a precompile.

### The Concrete Roko Workflow That Triggers This Precompile

Here is the end-to-end flow, from agent work to on-chain knowledge search:

**Step 1: Agent completes a task.**
An agent (e.g., a Claude-powered implementer) finishes work on a task in a plan.
The orchestrator in `orchestrate.rs` records an `Episode` -- a JSONL record at
`.roko/episodes.jsonl` containing: task_id, model used, prompt/completion tokens,
latency, gate verdicts, cost, success/failure, and an HDC fingerprint computed from
the prompt and outcome text.

**Step 2: Gate pipeline validates the work.**
The 7-rung gate pipeline (Compile, Lint, Test, Symbol, GeneratedTest, PropertyTest,
Integration) checks the agent's output. If it passes, the episode is marked
successful.

**Step 3: Knowledge is distilled.**
The `roko-neuro` distiller examines recent episodes and extracts knowledge entries.
For example, if three consecutive episodes show that splitting large files before
refactoring reduces gate failure rates, the distiller produces a Heuristic entry
with that observation. The entry gets an HDC fingerprint computed from its content
text using the same encoding pipeline as episodes.

**Step 4: Knowledge entry is posted to the chain.**
The entry is submitted to the `InsightLedger` contract on daeji. The transaction
includes: content hash (blake3 of the entry text), entry type (Insight=0,
Heuristic=1, Warning=2, AntiKnowledge=3, CausalLink=4, StrategyFragment=5),
half-life in blocks, confidence, and the raw 1,280-byte HDC fingerprint. The
contract stores this in its state storage.

**Step 5: A different agent needs relevant knowledge.**
Later, a different agent is dispatched for a new task. Before dispatch, the
orchestrator needs to populate the agent's system prompt with relevant knowledge.
Off-chain, this is done by the `NeuroStore` querying `.roko/neuro/`. On-chain, the
orchestrator (or the agent itself via its sidecar) submits a `STATICCALL` to the
HDC precompile at address `0x09` with:
- The query vector: the HDC fingerprint of the new task's description (1,280 bytes).
- top_k: how many results to return (e.g., 10).
- Filters: optionally restrict by entry type (only Heuristics and StrategyFragments)
  or minimum weight.

**Step 6: The precompile does fast Hamming search.**
The precompile reads its in-memory HDC index (rebuilt from InsightLedger state at
each block boundary), XORs the query against every stored vector, counts bits via
native POPCNT, and returns the top-k matches sorted by similarity. Each result
includes the entry ID, similarity score, weight, and a Merkle inclusion proof.

**Step 7: Results are injected into the agent's prompt.**
The top-k entries are fetched from the InsightLedger (by entry ID), formatted, and
injected into the agent's 9-layer system prompt as the "knowledge context" section --
the same role that `render_neuro_chunk` plays off-chain in orchestrate.rs today.

### Purpose

Perform a similarity search over all on-chain knowledge entries: given a query
hypervector, return the top-K most similar entries with their metadata and Merkle
inclusion proofs.

### Interface (ABI)

```
Input:  [query_vector: 1280 bytes][top_k: uint8][filters: bytes]
Output: [(similarity: uint16, entry_id: bytes32, weight: uint256, proof: bytes)][]
```

**Input fields:**

- `query_vector` (1,280 bytes): The 10,240-bit binary hypervector to search for.
  This is the same format as `HdcVector::to_bytes()` in roko-primitives: 160
  little-endian u64 words packed into 1,280 bytes.
- `top_k` (1 byte): Number of results to return (1-255).
- `filters` (variable length, optional): ABI-encoded filter criteria:
  - `entry_type_mask` (uint32): Bitmask of acceptable entry types. Bit 0 = Insight,
    bit 1 = Heuristic, bit 2 = Warning, bit 3 = AntiKnowledge, bit 4 = CausalLink,
    bit 5 = StrategyFragment. 0 = no filter (all types).
  - `min_weight` (uint256): Exclude entries below this weight threshold. 0 = no
    filter.
  - `max_age_blocks` (uint64): Exclude entries older than this many blocks. 0 = no
    filter.

**Output fields (per result, sorted by descending similarity):**

- `similarity` (uint16): 10,240 minus Hamming distance (higher = more similar; this
  inversion makes "higher is better" intuitive). Range: 0 to 10,240. This mirrors
  the `HdcVector::similarity` method's semantics but uses an integer scale instead
  of a float.
- `entry_id` (bytes32): Unique identifier of the matching knowledge entry in the
  InsightLedger contract.
- `weight` (uint256): The entry's current weight (a confidence/relevance score
  maintained by the knowledge ledger contract, analogous to the `balance` field in
  roko-neuro's `KnowledgeEntry`).
- `proof` (variable bytes): Merkle inclusion proof for this entry against the
  current block's state root. Allows a third party to verify the entry exists
  on-chain without trusting the responding node.

### Gas Cost: 50,000 (Fixed)

The gas cost does not scale with entry count because the native implementation's
performance is dominated by memory access patterns, not computation. The difference
between scanning 1,000 and 100,000 entries is negligible in native code (both
complete in microseconds).

For comparison: a single Solidity `SLOAD` (reading one 256-bit storage slot) costs
2,100 gas. The HDC precompile's 50,000 gas is equivalent to about 24 storage reads
-- a bargain for searching an entire knowledge base.

### Implementation Approach

```rust
// Location: daeji/crates/node/executor/src/precompiles/hdc.rs

pub struct HdcSearchPrecompile {
    // In-memory index of HDC vectors for all active knowledge entries.
    // Rebuilt at block boundaries from the InsightLedger contract's storage.
    index: Arc<RwLock<HdcIndex>>,
}

impl HdcIndex {
    /// Hamming distance using POPCNT on 64-bit words.
    /// This is the same algorithm as HdcVector::similarity in roko-primitives,
    /// but returns raw bit count instead of a normalized float.
    /// With AVX-512, this compiles to ~20 VPOPCOUNT instructions.
    fn hamming_distance(a: &[u64; 160], b: &[u64; 160]) -> u32 {
        let mut dist = 0u32;
        for i in 0..160 {
            dist += (a[i] ^ b[i]).count_ones();
        }
        dist
    }

    fn search(&self, query: &[u64; 160], top_k: usize) -> Vec<SearchResult> {
        // Score all entries by similarity
        let mut scored: Vec<_> = self.entries.iter()
            .map(|e| (10240 - Self::hamming_distance(query, &e.vector), e))
            .collect();
        // Partial sort for top-K (O(n log k), faster than full sort)
        scored.select_nth_unstable_by(top_k.min(scored.len()) - 1, |a, b| b.0.cmp(&a.0));
        scored.truncate(top_k);
        scored.sort_unstable_by(|a, b| b.0.cmp(&a.0));
        // ... encode results with Merkle proofs
    }
}

impl Precompile for HdcSearchPrecompile {
    fn run(&self, input: &[u8], gas_limit: u64) -> PrecompileResult {
        if gas_limit < 50_000 {
            return Err(PrecompileError::OutOfGas);
        }
        let query = parse_hdc_vector(&input[..1280]);
        let top_k = input[1280] as usize;
        let filters = parse_filters(&input[1281..]);
        let results = self.index.read().search(&query, top_k);
        Ok((50_000, encode_results_with_proofs(results)))
    }
}
```

### Dependencies

- **InsightLedger contract** (Solidity): Must be deployed first. This contract
  stores knowledge entries (content hash, hypervector, weight, metadata). The
  precompile reads from the same state storage slots the contract writes to. The
  Solidity entry types map 1:1 to roko-neuro's `KnowledgeKind` enum: 0=Insight,
  1=Heuristic, 2=Warning, 3=AntiKnowledge, 4=CausalLink, 5=StrategyFragment.
- **Off-chain HDC vector generation**: When an agent posts a knowledge entry, it
  precomputes the 10,240-bit HDC vector from the entry's text content using the
  same deterministic encoder that roko uses locally (`HdcVector::from_seed` and the
  fingerprinting pipeline in `roko-learn/src/hdc_fingerprint.rs`). The chain stores
  and searches vectors; it does not generate them.
- **Index rebuild strategy**: The `HdcIndex` is rebuilt at block boundaries from
  the state snapshot. At 100,000 entries x 1,280 bytes = 128 MB of vector data.
  Initial implementation: full rebuild per block. Optimization: incremental updates
  (invalidate only changed entries).

### Alternative: Contract-Only Approach

Without the precompile, search can be implemented in pure Solidity using categorical
tags instead of HDC similarity:

```solidity
mapping(bytes32 => bytes32[]) public entriesByTag;  // tag => entry IDs

function searchByTag(bytes32 tag) external view returns (bytes32[] memory) {
    return entriesByTag[tag];
}
```

This is deployable immediately with no chain changes. It is sufficient for small
knowledge bases (hundreds of entries with well-defined categories). It fails when:
entries do not fit neatly into predefined categories, the query is "find things
conceptually similar to X" rather than "find things tagged with Y", or entry count
grows beyond what tag enumeration handles efficiently. In roko's off-chain neuro
store, tag-based retrieval already works for small stores, but HDC similarity search
is what makes the knowledge system scale to thousands of entries generated by many
agents working concurrently.

---

## Precompile 0x0B: QMDB State Proofs

### The Problem This Solves

Roko agents need to verify that knowledge entries, episode anchors, and other
on-chain records actually exist without trusting a single RPC endpoint. In the
off-chain world, an agent reads `.roko/neuro/knowledge.jsonl` directly from the
filesystem -- trust comes from file ownership. On-chain, trust comes from
cryptographic proofs. An agent wants to answer: "Does this knowledge entry actually
exist at this block height?" The QMDB proof precompile generates a Merkle proof
that answers this question trustlessly, within the EVM execution context.

### Background: What QMDB Is

QMDB (Quick Merkle Database) is a purpose-built database for blockchain state
storage, developed in collaboration with LayerZero. It addresses two requirements
that conflict in standard databases:

**Fast state updates.** Blockchain state changes with every block. Each block may
modify thousands of storage slots (account balances, contract variables, nonces).
Updates must complete in milliseconds.

**Cryptographic state commitment.** The entire state must be summarized as a single
hash (the "state root") included in each block header. Computing this root requires
Merkleization: building a hash tree over all state entries. In a standard Merkle
tree, updating one leaf requires recomputing hashes along the path from leaf to root
-- O(log N) hash computations per update, each potentially requiring a random disk
read to fetch sibling hashes.

At scale (millions of storage slots), the combination of random I/O for Merkle
updates and sequential I/O for state writes creates a severe bottleneck. Ethereum's
state database (LevelDB or PebbleDB with a Merkle Patricia Trie) is one of the
primary performance limiters for node operators.

QMDB solves this with two design decisions:

- **O(1) SSD I/O per state update**: State changes are appended to a write-ahead
  log and batched. The on-disk structure is optimized for sequential writes, avoiding
  the random I/O that makes Merkle updates slow.
- **In-memory Merkleization**: The Merkle tree (internal nodes and hashes) is
  maintained in RAM. Computing the state root after a batch of updates requires no
  disk reads. This is feasible because the Merkle tree structure is much smaller
  than the state data itself.

### Background: What a Merkle Proof Is

A Merkle tree is a binary tree of cryptographic hashes:

- Each **leaf** is the hash of one data item (a storage key-value pair).
- Each **internal node** is the hash of its two children.
- The **root** is a single hash at the top that commits to every piece of data.

A **Merkle inclusion proof** proves that a specific data item is part of the tree
without revealing the entire tree. The proof consists of the sibling hashes along
the path from the target leaf to the root. Verification:

1. Hash the claimed data to compute the leaf hash.
2. Combine with the first sibling (from the proof) to compute the parent hash.
3. Continue up the tree, combining with each sibling from the proof.
4. Check that the final hash matches the known root (from the block header).

Proof size is logarithmic: for N leaves, the proof contains log2(N) hashes. For
1 million leaves with 32-byte hashes: ~20 hashes = ~640 bytes.

A **Merkle exclusion proof** proves a key does NOT exist. The mechanism involves
showing the two adjacent keys that bound the absent key, proving there is no entry
between them.

### What "Historical" Means

Most blockchain nodes store only the **current** state. To answer "what was the value
of key K at block 500?" (when you are now at block 10,000), you would need either:
a full archive node storing snapshots at every block (enormous disk usage), or
replaying all transactions from block 500 forward (enormous computation).

QMDB retains historical state roots and the data needed to construct proofs against
them. A **historical state proof** proves "at block N, key K had value V" by
providing a Merkle proof against the state root committed in block N's header.

### Why QMDB State Proofs Need a Precompile

The EVM execution context provides smart contracts with a flat key-value view of
storage via the `SLOAD` opcode. A contract can read any of its own storage slots,
but it **cannot**:

- Access the Merkle tree structure underlying its storage (the tree is an
  implementation detail of the node, invisible to the EVM).
- Access state from previous blocks (only the current block's state is available
  during execution).
- Compute proofs against historical state roots.

QMDB's internal structures (the Merkle tree, historical roots, the append-only
update log) are maintained by the node software, not by the EVM. A precompile runs
inside the node and can pass proof requests directly to QMDB's internal APIs.

### The Concrete Roko Workflow That Triggers This Precompile

**Scenario: Agent verifies a knowledge entry exists on-chain.**

1. Agent B is dispatched for a task. The orchestrator's system prompt includes a
   knowledge entry that was originally distilled by Agent A and posted to the
   InsightLedger. The entry's metadata includes the daeji transaction hash and
   block number where it was posted.

2. Agent B (or the orchestrator on its behalf) wants to verify the entry actually
   exists before trusting it. It calls the QMDB proof precompile via `STATICCALL`
   at address `0x0B` with:
   - `block_number`: the block where the entry was posted.
   - `key`: the storage slot in the InsightLedger contract where the entry is stored
     (computed as `keccak256(abi.encode(mapping_slot, entry_id))`).
   - `proof_type`: 0 (inclusion proof).

3. The precompile traverses QMDB's in-memory Merkle tree for that historical block,
   constructs the proof, and returns: `exists=true`, `value` (the storage slot
   contents), and the Merkle path.

4. The proof can be verified off-chain by anyone who knows the block header's state
   root (public data via `eth_getBlockByNumber`). No trust in the RPC endpoint is
   required.

**Scenario: Verifying an episode anchor.**

After an agent completes work and the gate pipeline passes, roko anchors the episode
hash on daeji via the `ChainWitnessEngine` (in `roko-chain/src/witness.rs`). The
witness transaction contains `b"roko.attestation.witness:" ++ blake3(episode_json)`.
Later, to prove this witness exists at a specific block, the same QMDB proof flow
applies -- the precompile generates a proof against that block's state root.

### Purpose

Generate Merkle inclusion or exclusion proofs for any storage key at any finalized
block height.

### Interface (ABI)

```
Input:  [block_number: uint64][key: bytes32][proof_type: uint8]
Output: [exists: bool][value: bytes32][proof: bytes]
```

**Input fields:**

- `block_number` (uint64): The historical block height to prove state at. Must be
  a finalized block (reverts if the block is not yet finalized or does not exist).
- `key` (bytes32): The storage key to prove. Follows standard EVM storage layout:
  `keccak256(abi.encode(slot_number))` for simple slots, or the mapping-derived
  key for mappings.
- `proof_type` (uint8): 0 = inclusion proof (key exists, return value plus Merkle
  path), 1 = exclusion proof (key does not exist, return proof of absence).

**Output fields:**

- `exists` (bool): True for inclusion proofs, false for exclusion proofs.
- `value` (bytes32): The value at the key (zero for exclusion proofs).
- `proof` (variable bytes): Serialized Merkle path. Verifiable against the
  `state_root` in the block header for `block_number`. The block header is public
  data available via standard JSON-RPC (`eth_getBlockByNumber`), so verification
  can be performed by anyone -- on-chain, off-chain, or on a different chain.

### Gas Cost: 30,000 (Fixed)

Generating a Merkle proof requires traversing the tree from leaf to root.
Tree depth is logarithmic in the number of state entries (approximately 20-25 levels
for millions of entries). Each level requires one hash lookup. With QMDB's in-memory
Merkle tree, this completes in microseconds. The 30,000 gas cost is lower than the
HDC precompile because the computation is simpler (a single tree traversal versus
scanning an entire index).

### Implementation Approach

```rust
// Location: daeji/crates/node/executor/src/precompiles/state_proof.rs

pub struct QmdbProofPrecompile {
    // Reference to QMDB's historical state, including Merkle trees
    // at each finalized block height.
    db: Arc<QmdbState>,
}

impl Precompile for QmdbProofPrecompile {
    fn run(&self, input: &[u8], gas_limit: u64) -> PrecompileResult {
        if gas_limit < 30_000 {
            return Err(PrecompileError::OutOfGas);
        }
        if input.len() < 41 {
            return Err(PrecompileError::Other("input too short"));
        }

        let block_number = u64::from_be_bytes(input[..8].try_into().unwrap());
        let key: [u8; 32] = input[8..40].try_into().unwrap();
        let proof_type = input[40];

        // Verify the requested block is finalized
        if !self.db.is_finalized(block_number) {
            return Err(PrecompileError::Other("block not finalized"));
        }

        let proof = match proof_type {
            0 => self.db.inclusion_proof(block_number, &key),
            1 => self.db.exclusion_proof(block_number, &key),
            _ => return Err(PrecompileError::Other("invalid proof type")),
        };

        Ok((30_000, encode_proof(proof)))
    }
}
```

### Dependencies

- **QMDB historical retention**: QMDB must be configured to retain historical
  Merkle tree data, not just the current state. This has storage implications: each
  block's tree delta must be retained. A pruning horizon (e.g., retain last 100,000
  blocks) may be appropriate.
- **Block finality tracking**: The precompile must verify the requested block is
  finalized before generating a proof, to prevent proofs against blocks that could
  later be reorganized.
- **No contract dependency**: Unlike the HDC precompile, this precompile operates
  on any storage key, not just knowledge entries. It is a general-purpose proof
  facility. This means it can prove the existence of episode witness anchors,
  agent registry entries, bounty market records, or any other on-chain state.

### Alternative: Off-Chain Proof Generation

Without the precompile, proofs can be generated off-chain:

1. Run a node with QMDB historical retention enabled.
2. Expose an RPC method: `daeji_getStateProof(blockNumber, key)`.
3. Return the proof to the requesting application.
4. If on-chain verification is needed, submit the proof to a Solidity verifier
   contract.

This works but introduces a trust assumption: the requesting contract trusts the
RPC endpoint to return a genuine proof. With the precompile, proof generation
happens within the EVM execution context (trustless, deterministic, part of the
state transition).

---

## Precompile 0x0C: BTLE Encryption/Decryption

### The Problem This Solves

Some roko agent operations require **commitment without early revelation**. For
example: when multiple agents compete for a bounty, each agent should commit to its
approach (model selection, strategy) before seeing what other agents chose. If Agent
A can see Agent B's model choice before committing its own, Agent A gains an unfair
advantage. BTLE (Binding Timelock Encryption) solves this: an agent encrypts its
commitment to a future consensus view. The ciphertext is posted on-chain immediately,
but the plaintext only becomes recoverable after that view is finalized.

In roko, this applies to:

- **Model selection in competitive dispatch**: The `CascadeRouter` in orchestrate.rs
  routes tasks to different models based on learned performance data. When multiple
  agents bid on the same bounty, each should commit to a model choice before seeing
  others' choices.
- **Strategy fragments in bounty markets**: An agent commits to a StrategyFragment
  (one of the six knowledge kinds) for solving a task. The commitment is encrypted;
  after the deadline view, all strategies are revealed simultaneously.
- **Sealed votes in agent governance**: If agents vote on knowledge entry quality
  (confirming or refuting entries), BTLE ensures votes are independent -- no agent
  can see how others voted before casting its own vote.

### Background: What BLS12-381 Pairings Are

BLS12-381 is an elliptic curve designed specifically for **pairing-based
cryptography**. A pairing (also called a bilinear map) is a mathematical function:

```
e: G1 x G2 -> GT
```

where G1, G2, and GT are mathematical groups defined over the curve. The pairing has
a critical algebraic property called **bilinearity**:

```
e(a * P, b * Q) = e(P, Q) ^ (a * b)
```

This property enables cryptographic constructions impossible with standard elliptic
curves:

- **BLS signatures**: Sign a message by computing `sigma = private_key * H(message)`
  (a point in G2). Verify by checking `e(generator, sigma) == e(public_key, H(message))`.
  The bilinearity property makes this equation hold.
- **Signature aggregation**: Multiple BLS signatures over different messages can be
  combined into one signature, verified with one pairing check.
- **Identity-Based Encryption (IBE)**: Encrypt to an arbitrary identity string
  (like a future block number) without needing a specific public key for that
  identity.
- **Zero-knowledge proofs**: Pairings are fundamental to zkSNARK proof systems.

The pairing computation involves:

- Elliptic curve point multiplication in both G1 and G2.
- A "Miller loop" (an iterative algorithm computing intermediate pairing values).
- A "final exponentiation" in the target group GT.

Each step involves modular arithmetic over a 381-bit prime field.

### Why BLS12-381 Pairings Are Expensive in Solidity

Ethereum mainnet has precompiles for BN256 (a different, older pairing-friendly
curve) at addresses 0x06-0x08. It does **not** have precompiles for BLS12-381.
EIP-2537 proposes adding them, but as of early 2026 this EIP has not been deployed
to mainnet.

Without a precompile, implementing BLS12-381 in Solidity requires:

- **381-bit modular arithmetic** using 256-bit EVM words. Each 381-bit
  multiplication requires multiple 256-bit multiplications with carry propagation.
- **Elliptic curve point operations** built on this field arithmetic: point
  addition, point doubling, scalar multiplication.
- **The Miller loop**: approximately 64 iterations, each involving multiple field
  multiplications and point operations.
- **Final exponentiation**: multiple field exponentiations with large exponents.

Estimated gas cost for a single pairing check in pure Solidity: **500,000 to
2,000,000 gas** (depending on optimization level). For BTLE, both encryption and
decryption require at least one pairing operation. At these gas costs, using BTLE
for routine agent operations (sealing bids, encrypting votes) would be
prohibitively expensive.

In native Rust with optimized libraries (such as `blst`, the fastest BLS12-381
implementation, maintained by Supranational): a single pairing check takes
approximately **1-2 milliseconds**. This three-to-four order of magnitude gap
between native performance and EVM emulation makes a precompile essential.

### The Concrete Roko Workflow That Triggers This Precompile

**Scenario: Sealed model selection for a competitive bounty.**

1. A bounty is posted to the BountyMarket contract on daeji. Three agents decide to
   compete for it.

2. Each agent chooses its model (e.g., Claude Opus 4.6, Codex, Gemini) and
   strategy. The agent constructs a commitment payload:
   `plaintext = abi.encode(agent_address, model_slug, strategy_hash)`.

3. The agent encrypts the commitment via the BTLE precompile (address `0x0C`,
   operation `0x00`), targeting a future view number (e.g., current view + 100,
   roughly 40 seconds at 400ms block time). The precompile returns a ciphertext.

4. The agent posts the ciphertext to the `BtleVault` contract on daeji. All three
   agents do this independently, before the target view.

5. The target view is finalized by daeji's Simplex BFT consensus. Finalization
   produces a threshold VRF output (a BLS signature on the view number), which
   serves as the decryption key material.

6. Any party calls `BtleVault.revealCommitment(id)`, which internally calls the
   BTLE precompile (operation `0x01`) with the stored ciphertext and the
   now-finalized target view. The precompile looks up the VRF output and decrypts,
   returning the plaintext.

7. All three commitments are now revealed simultaneously. No agent could have seen
   others' choices before committing.

### Purpose

Provide native-speed BLS12-381 operations for Binding Timelock Encryption (BTLE).
BTLE allows data to be encrypted to a future consensus view, with automatic
decryption when that view is finalized (see doc 04, section 1 for the full BTLE
explanation). Two operations:

1. **Encrypt**: Given a future view number and plaintext, produce a ciphertext that
   can only be decrypted after that view is finalized.
2. **Decrypt**: Given a ciphertext and a finalized view number, recover the
   plaintext. Reverts if the view is not yet finalized.

### Interface (ABI)

**Encrypt** (callable at any time):

```
Input:  [operation: uint8 = 0x00][target_view: uint64][plaintext: bytes]
Output: [ciphertext: bytes]
```

- `operation` (1 byte): 0x00 for encrypt.
- `target_view` (uint64): The future view number. Ciphertext will be decryptable
  only after this view is finalized by consensus.
- `plaintext` (variable bytes): The data to encrypt.
- The encryption key is derived from the chain's group public key and the target
  view number using a hash-to-G1 operation followed by a pairing computation. The
  group public key is read from chain state (not passed as input).

**Decrypt** (callable only after target_view is finalized):

```
Input:  [operation: uint8 = 0x01][target_view: uint64][ciphertext: bytes]
Output: [plaintext: bytes]
```

- `operation` (1 byte): 0x01 for decrypt.
- `target_view` (uint64): The view number the ciphertext was encrypted to.
- `ciphertext` (variable bytes): The BTLE-encrypted data (output of the encrypt
  operation).
- The precompile looks up the VRF output (threshold signature) for `target_view`.
  If the view is not yet finalized, the call **reverts**.
- The VRF output is used as the decryption key material via the IBE decryption
  algorithm.

**Ciphertext format** (produced by encrypt, consumed by decrypt):

```
[C1: 48 bytes][nonce: 12 bytes][encrypted_data: variable][auth_tag: 16 bytes]
```

- `C1`: A BLS12-381 G1 point (the IBE ephemeral public value).
- `nonce`: Random nonce for symmetric encryption.
- `encrypted_data`: ChaCha20-Poly1305 ciphertext (hybrid scheme: IBE for key
  exchange, symmetric cipher for bulk data).
- `auth_tag`: Poly1305 authentication tag ensuring ciphertext integrity.

### Gas Cost: 80,000 (Both Encrypt and Decrypt)

Each operation involves one or more BLS12-381 pairing computations plus a
hash-to-curve operation plus symmetric encryption/decryption. In native Rust with
`blst`, the total completes in 1-2 milliseconds.

The 80,000 gas cost reflects:

- The operation is heavier than HDC search (50,000) or QMDB proofs (30,000) due to
  the pairing computation.
- It must be cheap enough that sealing a bid or encrypting a vote is economically
  feasible for routine agent operations.
- The Solidity alternative (500,000-2,000,000 gas) would make BTLE impractical.

### Implementation Approach

```rust
// Location: daeji/crates/node/executor/src/precompiles/btle.rs

pub struct BtlePrecompile {
    // The chain's 48-byte BLS12-381 group public key
    // (static, does not change even across validator resharing)
    group_pubkey: G2Affine,
    // Access to finalized VRF outputs (threshold signatures per view)
    vrf_store: Arc<dyn VrfStore>,
}

impl Precompile for BtlePrecompile {
    fn run(&self, input: &[u8], gas_limit: u64) -> PrecompileResult {
        if gas_limit < 80_000 {
            return Err(PrecompileError::OutOfGas);
        }
        match input[0] {
            0x00 => self.encrypt(&input[1..]),
            0x01 => self.decrypt(&input[1..]),
            _ => Err(PrecompileError::Other("unknown operation")),
        }
    }
}

impl BtlePrecompile {
    fn encrypt(&self, input: &[u8]) -> PrecompileResult {
        let target_view = u64::from_be_bytes(input[..8].try_into().unwrap());
        let plaintext = &input[8..];

        // IBE encryption:
        // 1. Hash target_view to a G1 point (the "identity")
        let q_id = hash_to_g1(&target_view.to_le_bytes());
        // 2. Choose random scalar r
        let r = random_scalar();
        // 3. C1 = r * G1_generator (ephemeral public value)
        let c1 = G1Affine::generator() * r;
        // 4. Derive symmetric key from pairing: e(q_id, group_pubkey)^r
        let key = sha256(pairing(&(q_id * r).into(), &self.group_pubkey));
        // 5. Symmetric encryption with ChaCha20-Poly1305
        let (nonce, ciphertext, tag) = chacha_encrypt(plaintext, &key);

        // Encode output
        Ok((80_000, encode_ciphertext(c1, nonce, ciphertext, tag)))
    }

    fn decrypt(&self, input: &[u8]) -> PrecompileResult {
        let target_view = u64::from_be_bytes(input[..8].try_into().unwrap());
        let ciphertext_blob = &input[8..];

        // Look up VRF output (threshold signature at target_view)
        // This IS the IBE private key for identity=target_view
        let sigma_v = self.vrf_store.vrf_output(target_view)
            .ok_or(PrecompileError::Other("view not yet finalized"))?;

        // Parse ciphertext components
        let (c1, nonce, encrypted, tag) = parse_ciphertext(ciphertext_blob);

        // IBE decryption:
        // e(C1, sigma_v) = e(r*G1, x*H(view)) = e(G1, H(view))^(rx)
        //                = e(H(view), group_pubkey)^r  (same key as encryption)
        let key = sha256(pairing(&c1, &sigma_v));
        let plaintext = chacha_decrypt(encrypted, &key, nonce, tag)?;

        Ok((80_000, plaintext))
    }
}
```

### Dependencies

- **VRF store**: The decrypt path requires the threshold signature produced when
  each view is finalized. Daeji's `SeedReporter` already records these.
- **BLS12-381 library**: The `blst` crate (or commonware-cryptography's BLS
  wrappers) for pairing operations and hash-to-curve.
- **Group public key**: Daeji's 48-byte group public key, available at node
  initialization. Remains constant across validator set resharing.
- **BtleVault contract** (Solidity): A companion contract that stores posted
  ciphertexts and manages the commitment/reveal lifecycle:

```solidity
contract BtleVault {
    struct Commitment {
        address committer;
        uint64  targetView;
        bytes   ciphertext;
        uint256 blockPosted;
    }
    mapping(bytes32 => Commitment) public commitments;

    function postCommitment(uint64 targetView, bytes calldata ct) external {
        bytes32 id = keccak256(abi.encodePacked(msg.sender, targetView, ct));
        commitments[id] = Commitment(msg.sender, targetView, ct, block.number);
    }

    function revealCommitment(bytes32 id) external view returns (bytes memory) {
        Commitment storage c = commitments[id];
        bytes memory input = abi.encodePacked(uint8(0x01), c.targetView, c.ciphertext);
        (bool ok, bytes memory result) = address(0x0C).staticcall(input);
        require(ok, "view not yet finalized");
        return result;
    }
}
```

### Alternative: Contract-Only Approach

Without the precompile, BTLE can work in a degraded mode:

1. **Off-chain encryption, on-chain storage**: Agents perform IBE encryption
   off-chain using a Rust or JavaScript BLS12-381 library. They post the ciphertext
   on-chain as opaque bytes. Decryption also happens off-chain after fetching the
   VRF output via RPC. The contract stores ciphertexts and manages the auction/voting
   logic but cannot verify encryption correctness on-chain.

2. **Pure Solidity BLS12-381**: Implement pairing operations in Solidity. Feasible
   but costs 500,000-2,000,000 gas per operation, making BTLE economically
   impractical for routine use.

---

## Registry Precompile 0x08 (Deferred)

Earlier design documents specified an agent registry at precompile address 0x08.
After analysis, this is **deferred in favor of a standard Solidity contract**.

Registry operations (register an agent, send a heartbeat, look up an agent by ID)
are simple storage reads and writes that Solidity handles efficiently:

| Operation | Solidity gas cost | Frequency |
|-----------|------------------|-----------|
| Register (write 3-4 slots) | ~50,000-80,000 | Once per agent |
| Heartbeat (update 1 slot) | ~5,000 | Periodic (every N blocks) |
| Lookup (read 3-4 slots) | ~6,000-8,000 | Per-task |

At a 30M block gas limit, a single block can handle thousands of lookup operations.
For an agent system with hundreds of agents, this is not a bottleneck.

If the agent count scales to tens of thousands with per-transaction lookups becoming
a material fraction of block gas, a precompile can be added later. The contract's
external interface (function signatures) would not change; the internal
implementation would switch to a native in-memory hash map.

---

## Implementation Order

| Priority | Component | Type | Chain Changes | Prerequisite |
|----------|-----------|------|---------------|--------------|
| 1 | AgentRegistry | Solidity contract | None | None |
| 2 | InsightLedger (tag-based search) | Solidity contract | None | None |
| 3 | HDC precompile (0x09) | REVM precompile | New node binary | InsightLedger deployed |
| 4 | QMDB proof precompile (0x0B) | REVM precompile | New node binary | QMDB archive config |
| 5 | BTLE precompile (0x0C) | REVM precompile | New node binary | VRF store access, BtleVault contract |

Steps 1-2 are pure Solidity, deployable immediately with no chain changes.
Steps 3-5 require modifying the daeji node binary (adding Rust code to the REVM
executor) and shipping an updated chain node.

---

## File Layout in Daeji

Custom precompiles live alongside the executor:

```
daeji/crates/node/executor/src/
  executor.rs              # RevmExecutor -- registers precompiles during init
  precompiles/
    mod.rs                 # Re-exports; builds the ContextPrecompiles set
    hdc.rs                 # 0x09: HDC similarity search
    state_proof.rs         # 0x0B: QMDB historical state proofs
    btle.rs                # 0x0C: BTLE encryption/decryption
```

Registration in `RevmExecutor::new()`:

```rust
use crate::precompiles::{HdcSearchPrecompile, QmdbProofPrecompile, BtlePrecompile};

let mut precompiles = ContextPrecompiles::default(); // standard 0x01-0x09
precompiles.extend([
    (
        Address::from_low_u64_be(0x09),
        Arc::new(HdcSearchPrecompile::new(knowledge_index.clone())),
    ),
    (
        Address::from_low_u64_be(0x0B),
        Arc::new(QmdbProofPrecompile::new(qmdb_state.clone())),
    ),
    (
        Address::from_low_u64_be(0x0C),
        Arc::new(BtlePrecompile::new(consensus_state.clone(), group_pubkey)),
    ),
]);
```

Each precompile is instantiated with references to the node state it needs. These
references are read-only during EVM execution: precompiles do not mutate state
directly. State changes happen through normal EVM storage operations in the calling
contract.
