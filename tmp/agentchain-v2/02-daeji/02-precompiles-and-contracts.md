# Precompiles and Smart Contracts

This document covers the chain's two extension surfaces: the **custom EVM
precompiles** (native Rust functions at reserved EVM addresses) and the
**Solidity contract suite** (production-intent application code that
runs in REVM). Together they make up the on-chain application surface
that everything else in the agentchain stack composes against.

The two prerequisite chain-side fixes that gate full contract deployment
to the live devnet are documented at the end (`block.timestamp` and
`BLOCKHASH`) — they are described as current devnet limitations rather
than blockers because the design is decided and queued.

---

## What an EVM Precompile Is

A **precompile** is a piece of native code at a fixed, well-known EVM
address. To the calling contract it looks like any other contract: you
`CALL` or `STATICCALL` the address with ABI-encoded input and receive
ABI-encoded output. Internally the EVM hands off to a compiled Rust
function rather than interpreting bytecode. The function runs inside
the node with full access to chain state, historical data, consensus
signatures, and anything else the node maintains.

Precompiles exist for two reasons:

- **Cryptographic operations.** Elliptic-curve pairings, modular
  exponentiation, large-integer arithmetic. Emulating these in Solidity
  costs millions of gas; native code completes them in microseconds.
- **Operations that need chain internals.** The Merkle-tree structure
  inside QMDB, consensus VRF outputs, the validator group's public
  key — none of these are accessible to a Solidity contract. A
  precompile runs inside the node and reads them directly.

Standard Ethereum has 9 precompiles at addresses `0x01`–`0x09`
(ECRECOVER, SHA-256, RIPEMD-160, identity, MODEXP, BN256 curve
operations, Blake2). The Nunchi blockchain reserves three additional
agent-specific precompiles plus a small reserved range for future agent
operations.

---

## Precompile Address Map

| Address | Name | Category | Status |
|---|---|---|---|
| `0x09` (overload) | HDC Similarity Search | Agent knowledge | Designed; pending precompile-registry wiring |
| `0x0B` | QMDB Historical State Proofs | State attestation | Designed; pending precompile-registry wiring |
| `0x0C` | BTLE Encryption / Decryption | Threshold cryptography | Designed; pending precompile-registry wiring |
| `0xA0_` | Index publication page | Validator-aggregated indices | `0xA01` reserved for the ISFR oracle |
| `0xA10`–`0xA1F` | Agent communication namespace | Agent operations | Reserved; populated incrementally |

The `0x09` address is normally the standard Ethereum `blake2f`
precompile; the chain overloads it for HDC similarity search. The
custom precompile registry documented below performs the routing.

The current node still calls a default-mainnet REVM builder that loads
only the standard `0x01`–`0x09` precompiles. The replacement registry
that adds the chain-specific entries is a small node-binary change
queued in `05-roadmap.md`.

---

## The Precompile Registry

REVM precompiles implement a simple trait:

```rust
pub trait Precompile: Send + Sync {
    fn run(
        &self,
        input: &Bytes,      // Raw bytes from the EVM CALL instruction
        gas_limit: u64,     // Gas budget
    ) -> PrecompileResult;  // (gas_used, output_bytes) or error
}
```

The chain composes a custom registry that combines the standard
Ethereum precompiles with chain-specific ones:

```rust
pub struct KoraPrecompiles {
    standard: EthPrecompiles,
    custom: Vec<(Address, Box<dyn Precompile>)>,
}

impl PrecompileSet for KoraPrecompiles {
    fn run(&self, address: &Address, input: &[u8], gas_limit: u64)
        -> Option<PrecompileResult>
    {
        for (addr, precompile) in &self.custom {
            if addr == address {
                return Some(precompile.run(input, gas_limit));
            }
        }
        self.standard.run(address, input, gas_limit)
    }
}
```

Precompiles are stateful (they hold shared references to the knowledge
index, the QMDB state, the VRF store) but are only invoked read-side
during EVM execution — they never directly mutate state.

---

## Precompile 0x09: HDC Similarity Search

### What it solves

Autonomous agents generate operational knowledge during task execution
(insights, heuristics, warnings, causal links, strategy fragments,
anti-knowledge — covered in `03-agent-systems.md`). Each entry is
encoded as a **10,240-bit binary hypervector** (1,280 bytes, stored
as `[u64; 160]` in Rust). Finding knowledge relevant to a new task is
a similarity search over all known entries.

Similarity is measured by **Hamming distance**: the number of bit
positions where two vectors differ. Modern CPUs have a hardware
instruction (`POPCNT`) that counts the 1-bits in a 64-bit word in a
single clock cycle. With AVX-512 SIMD, a compiler vectorizes
Hamming-distance computation over 160-word vectors and runs all
comparisons in microseconds.

In Solidity, this operation is computationally infeasible at scale.
Each 256-bit XOR plus a popcount emulation costs on the order of
hundreds of gas; scanning 100,000 entries works out to roughly 400
million gas — exceeding any block's gas limit by more than an order
of magnitude.

In native Rust with `.count_ones()` compiling to POPCNT, scanning
100,000 entries takes approximately **170 microseconds**. The
precompile charges a flat **50,000 gas** regardless of entry count
because the actual computation is dominated by cache-friendly memory
access, not arithmetic.

### ABI

```
Input:  [query_vector: 1280 bytes][top_k: uint8][filters: bytes]
Output: [(similarity: uint16, entry_id: bytes32, weight: uint256, proof: bytes)][]
```

Input fields:

- `query_vector` (1,280 bytes): the 10,240-bit query hypervector in
  little-endian `[u64; 160]` serialization.
- `top_k` (1 byte): number of results to return (1–255).
- `filters` (variable, optional): ABI-encoded filter criteria
  (entry-type bitmask, minimum weight, maximum age in blocks).

Output fields, sorted by descending similarity:

- `similarity` (uint16): `10,240 − Hamming_distance` (higher = more
  similar). The inversion makes "higher is better" intuitive.
- `entry_id` (bytes32): the knowledge entry identifier in the
  `InsightBoard` ledger contract.
- `weight` (uint256): the entry's current decayed weight (decay
  formula and confirmation boost are applied).
- `proof` (variable bytes): Merkle inclusion proof for the entry
  against the current block's state root, so third parties can verify
  results without trusting the responding node.

### Gas: 50,000 (fixed)

Cost does not scale with entry count. For comparison, a single
Solidity `SLOAD` costs 2,100 gas; 50,000 gas is roughly 24 storage
reads — a bargain for searching an entire knowledge base.

### How the index is maintained

The precompile holds an in-memory `HdcIndex` rebuilt at block
boundaries from the latest `InsightBoard` state and `InsightPosted`
event content. At 100,000 entries × 1,280 bytes the total raw vector
data is ~128 MB. Initial implementation rebuilds the index per block
(cheap at devnet scale); incremental updates that invalidate only
changed entries are an obvious optimization path.

The HDC vector itself is computed off-chain by each agent's runtime
from the entry's text using a deterministic encoder (FNV-1a seeding
plus splitmix64 PRNG expansion, combined via XOR-bind, permute, and
majority-vote bundle operations). The chain stores and searches
vectors; it does not generate them.

### Solidity-only fallback

Without the precompile, similarity search can be approximated by
tag-based retrieval in pure Solidity. This is sufficient for small
knowledge bases where entries fit into predefined categories, and is
the fallback shape today. It fails as soon as queries are "find
things conceptually similar to X" rather than "find things tagged Y",
or as soon as the entry count grows past a few thousand. Migration
recommendation: deploy tag-based contract first, migrate to the
precompile when entry count or query patterns demand it.

---

## Precompile 0x0B: QMDB Historical State Proofs

### What it solves

Agents and external verifiers need to answer: "what was the value of
key K at block N?". An EVM contract reads storage through the `SLOAD`
opcode, which only exposes a flat key-value view of its own current
state. It cannot access:

- The Merkle-tree structure underlying its storage (invisible to the
  EVM).
- State from previous blocks (only the current block's state is
  available).
- Proofs that any specific key-value pair was or was not present at
  block N.

QMDB's historical roots and its in-memory Merkle tree are maintained
by the node implementation, not by the EVM. A precompile that runs
inside the node can access QMDB's internal APIs and produce the
proofs.

### ABI

```
Input:  [block_number: uint64][key: bytes32][proof_type: uint8]
Output: [exists: bool][value: bytes32][proof: bytes]
```

Input fields:

- `block_number` (uint64): the historical block to prove state at.
  Must be finalized.
- `key` (bytes32): the storage key to prove. Standard EVM storage
  layout: `keccak256(abi.encode(slot_number))` for simple slots or
  the mapping-derived key for mappings.
- `proof_type` (uint8): `0` = inclusion proof (key exists, return
  value plus Merkle path); `1` = exclusion proof (key does not exist,
  return proof of absence).

Output fields:

- `exists` (bool).
- `value` (bytes32): the value at the key (zero for exclusion proofs).
- `proof` (variable bytes): serialized Merkle path verifiable against
  the `state_root` in the block header for `block_number`. Block
  headers are public data, so verification can be performed by any
  party — on-chain, off-chain, or on a different chain.

### Gas: 30,000 (fixed)

Merkle proofs require traversing the tree from leaf to root. The tree
depth is logarithmic in the number of state entries (about 20–25
levels for millions of entries). Each level is one hash lookup; with
QMDB's in-memory Merkle tree these complete in microseconds. The
cost is lower than HDC search because the computation is simpler.

### Why it complements the state-root model

The current state root is a transition hash, not a Merkle Patricia
Trie root, so `eth_getProof` cannot generate standard Ethereum
inclusion proofs. The `0x0B` precompile is the chain-internal answer:
it generates proofs against any historical block's state root using
QMDB's internal Merkle structure, exposing the proof capability
inside EVM execution without changing the state-root format.

### Off-chain alternative

Without the precompile, proofs can be generated off-chain by running
a node with QMDB historical retention enabled and exposing an RPC
method. This works but introduces a trust assumption: the requesting
contract trusts the RPC endpoint to return a genuine proof. With the
precompile, proof generation happens inside EVM execution
(trustless, deterministic, part of the state transition).

---

## Precompile 0x0C: BTLE Encryption and Decryption

### What it solves

Some agent operations require commitment without early revelation.
The two canonical examples are sealed-bid auctions and time-delayed
knowledge reveals (covered in `03-agent-systems.md`). Standard
commit-reveal schemes have a fatal flaw: a participant can refuse to
reveal. **Binding Timelock Encryption (BTLE)** eliminates this flaw —
decryption happens automatically as a byproduct of normal consensus,
not through any participant's action.

### Why a precompile

BTLE uses **Identity-Based Encryption (IBE)** over BLS12-381
pairings. Each encrypt and decrypt requires at least one pairing
computation. Pure-Solidity BLS12-381 pairings cost on the order of
500,000–2,000,000 gas (381-bit modular arithmetic emulated on 256-bit
EVM words, Miller-loop iterations, final exponentiation). Ethereum's
EIP-2537 proposes adding BLS12-381 precompiles to mainnet but has not
been deployed there as of the time of writing. At those gas costs,
using BTLE for routine operations is prohibitively expensive.

In native Rust with the `blst` library, a single pairing completes in
1–2 milliseconds. The precompile charges **80,000 gas** for either
encrypt or decrypt — cheap enough for routine sealed bids and
encrypted votes.

### ABI

**Encrypt** (callable at any time):

```
Input:  [operation: uint8 = 0x00][target_view: uint64][plaintext: bytes]
Output: [ciphertext: bytes]
```

The encryption key is derived from the chain's group public key (read
from chain state) and the target view number via a hash-to-G1
operation followed by a pairing computation.

**Decrypt** (callable only after `target_view` is finalized):

```
Input:  [operation: uint8 = 0x01][target_view: uint64][ciphertext: bytes]
Output: [plaintext: bytes]
```

The precompile looks up the threshold-VRF output for `target_view`.
If the view is not yet finalized, decrypt reverts. The VRF output
serves as the IBE decryption key.

### Ciphertext format

```
[C1: 48 bytes][nonce: 12 bytes][encrypted_data: variable][auth_tag: 16 bytes]
```

`C1` is a BLS12-381 G1 point (the IBE ephemeral public value); the
encrypted body uses ChaCha20-Poly1305 (the scheme is hybrid — IBE for
key exchange, symmetric cipher for bulk data).

### Gas: 80,000 (encrypt and decrypt)

Higher than HDC search or QMDB proofs because of the pairing
computation. Low enough for routine sealed bids and encrypted votes.

### Companion contract: `BtleVault`

A small Solidity contract stores the ciphertexts and manages the
commit-reveal lifecycle:

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

Anyone can call `revealCommitment` after the target view finalizes;
the precompile call succeeds automatically once the VRF output is
available, and reverts before that.

---

## The Agent Namespace `0xA10`–`0xA1F`

This page is reserved for agent-specific precompiles. The design
principle: keep the namespace cohesive so agent-specific operations
can be discovered by scanning a single small address range, and so
the gas-cost model can be documented uniformly.

The first registrations populate the operations contracts most need
to delegate to the chain rather than re-implement in Solidity:

- `0xA10`: ERC-8004 passport lookup by Ethereum address.
- `0xA11`: Capability-bit check (does the agent at address X hold
  capability Y?).
- `0xA12`: Tier check (is the agent at address X at tier Y or above?).
- `0xA13`: Reputation-minimum check (is the agent's domain-Y score at
  least Z?).

Each new entry is small in code and large in commitment; the page
grows incrementally as products demonstrate the need. The expansion
plan is in `05-roadmap.md`.

---

## The Index Page `0xA0_`

The validator-aggregated index publication page. The first
registration is the ISFR oracle at `0xA01`, covered in detail in
`04-defi-and-operations.md`. Reads are fixed-gas, equivalent to
reading the block number — there is no contract-call overhead and no
per-source variable cost.

The same precompile shape is intended to be reused for additional
benchmark indices over time (agent task success, knowledge quality,
security-detection rates, research-output quality), each at its own
address inside this page.

---

## Implementation Priority Across Precompiles

| Priority | Component | Type | Chain change | Prerequisite |
|---|---|---|---|---|
| 1 | `AgentRegistry.sol` | Contract | None | — |
| 2 | `InsightBoard.sol` (tag-based search) | Contract | None | — |
| 3 | HDC precompile (`0x09`) | Precompile | New node build | `InsightBoard` deployed |
| 4 | QMDB proof precompile (`0x0B`) | Precompile | New node build + archive config | QMDB historical retention |
| 5 | BTLE precompile (`0x0C`) | Precompile | New node build | VRF store access + `BtleVault` |

Steps 1–2 are pure Solidity and deployable immediately. Steps 3–5
require modifying the node binary (adding Rust code to the REVM
executor) and shipping an updated chain node to all validators — a
coordinated upgrade.

---

## The Solidity Contract Suite

Ten production-intent Solidity contracts make up the on-chain
application surface, plus several companion contracts specified for
later phases. All are standard Solidity (pragma `^0.8.20` or
`^0.8.26`) — no chain-specific language extensions. They compile with
any Solidity toolchain (Foundry, Hardhat) and deploy via standard
`eth_sendRawTransaction`. Most have been exercised against local
Anvil and against the in-process EVM fork simulator that lives
alongside the agent runtime. Live deployment to the Daeji devnet is
gated on the two prerequisite node-level fixes documented at the end
of this file.

### Contract inventory

| Contract | Purpose | Uses `block.timestamp`? |
|---|---|---|
| `MockERC20` | `DAEJI` test token (plain ERC-20, no demurrage) | No |
| `AgentRegistry` | Minimal agent identity (name, capabilities, heartbeat, liveness) | Yes |
| `IdentityRegistry` | Full ERC-8004 identity passport (4 tiers, staking, timelocks, TEE attestation, system-prompt hash) | Yes |
| `ReputationRegistry` | 7-domain EMA reputation scores with decay | Yes |
| `ValidationRegistry` | Work proofs and validator attestations | Yes |
| `InsightBoard` | Knowledge ledger (post, confirm, pheromone counter) | Yes |
| `BountyMarket` | Job marketplace with escrow and three hiring models | Yes |
| `WorkerRegistry` | Worker staking + liveness | Yes |
| `ConsortiumValidator` | Consortium-based validation with weighted voting | Yes |
| `FeeDistributor` | Fee distribution across registered recipients | No |

Planned but not yet deployed:

- `ClearingHouse` — the yield-perpetual settlement contract
  (covered in `04-defi-and-operations.md`).
- `BtleVault` — sealed-ciphertext storage and reveal lifecycle.
- `PredictionRegistry` — on-chain predictive-foraging surface.

The full feature set of each contract is covered in
`03-agent-systems.md` (identity, reputation, knowledge ledger) and
`04-defi-and-operations.md` (clearing, marketplace, fees). This file
catalogues what each contract stores and how the suite fits together.

### `MockERC20` — the `DAEJI` test token

A standard ERC-20 with `transfer`, `approve`, `transferFrom`,
`balanceOf`, `allowance`, `totalSupply`. No demurrage, no minting
schedule, no special features. Used as the gas/fee/reward asset
inside the marketplace and `InsightBoard` rewards while the eventual
native token's economics remain an open question (covered in
`05-roadmap.md`).

The eventual native token's design target is documented in
in-memory Rust reference code: lazy demurrage with a 1% annual
decay, five earning pathways (task completion, knowledge
contribution, validation participation, reputation staking,
marketplace fees), five spending mechanisms (compute purchase,
knowledge access, stake deposit, market bid, slashing bond), and
ERC-3009 `transferWithAuthorization` for gasless micropayments.
Whether and when this native token actually ships is on the roadmap.

### `AgentRegistry` — minimal agent identity

The currently deployed agent-identity contract. Stores agent ID,
owner address, name, capabilities array, and a `lastSeen` timestamp
updated by periodic heartbeats. Functions: `register`, `heartbeat`,
`update`, `getAgent`, `isActive`. The minimal precursor to the full
ERC-8004 `IdentityRegistry`.

### `IdentityRegistry` — full ERC-8004 passport

The production-target agent-identity contract. Implements the full
ERC-8004 standard: soulbound ERC-721 passport per agent,
four-tier system with staking thresholds, 64-bit capability bitmask,
SHA-256 system-prompt hash, TEE attestation hash with expiry, Agent
Card URI, prompt-update timelock, and withdrawal cooldown.

Every timelock and cooldown uses `block.timestamp`; the current
devnet's `block.timestamp` set to block height (a current limitation
documented at the end of this file) is the reason live deployment
waits.

### `ReputationRegistry` — 7-domain EMA scoring

Stores per-agent per-domain `(score, jobCount, lastUpdate)`, the set
of authorized feedback sources, and decay parameters. Authorized
sources (the marketplace contract, the clearing contract, peer-review
contracts) emit feedback events; the contract applies the EMA
update. Scores decay toward neutral with a 30-day half-life after a
7-day grace period of inactivity. The 7 domain tracks and the EMA
formula are covered in `03-agent-systems.md`.

### `ValidationRegistry` — work proofs

Records proofs of completed work. Each `WorkProof` carries the agent
ID, a job hash, a Merkle root over the deliverables, per-rung gate
results (compile, lint, test, symbol checks, generated tests,
property tests, LLM-judge), and an optional clearing certificate for
DeFi work. Verification can come from any of four validator types
(reputation-based, stake-secured re-execution, zkML, TEE oracle); the
registry stores the proofs, and consumer contracts (marketplace,
clearing house) reference them.

### `InsightBoard` — the knowledge ledger

The on-chain anchor for the agent knowledge layer. Posts (content
hash + URI), confirmations (incrementing a pheromone counter), and a
token reward per confirmation. Full contract surface and the
`NeuroChainSync` protocol that feeds it are in `03-agent-systems.md`.

### `BountyMarket` — the job marketplace

Implements ERC-8183 (a proposed standard for on-chain agent task
coordination). The job lifecycle:

```
POSTED -> BIDDING -> ASSIGNED -> IN_PROGRESS -> SUBMITTED -> VERIFIED -> SETTLED
                                       |              |
                                ABANDONED       DISPUTED -> RESOLVED -> SETTLED
```

Each transition is enforced on-chain. Three hiring models cover
different job sizes and trust levels:

- **Random VRF.** For commodity work under a cost threshold. The
  block-level VRF picks an agent from the eligible pool using
  power-of-two-choices load balancing (two random agents are
  selected, the less-loaded one is assigned). Minimal fees.
- **Blind Vickrey auction (reputation-adjusted, second-price).** The
  default for standard jobs. Bidders submit encrypted bids scored by
  `s_i = p_i * (1 + (1 - R_i))`, where `p_i` is the price bid and
  `R_i` is the agent's domain reputation. The winner pays
  second-price `payment = s_second / (1 + (1 - R_winner))`,
  preserving the Vickrey truthfulness property (bidding your true
  cost is the dominant strategy) while naturally favoring
  higher-reputation agents.
- **Direct hire.** When the requester knows which agent they want.
  Escalating fee premiums apply when one agent's volume share
  approaches monopoly territory.

### `WorkerRegistry` — worker staking

Heavier staking model for workers participating in specific economic
mechanisms (clearing solving, oracle submission). Uses
`block.timestamp` for liveness windows.

### `ConsortiumValidator` — consortium voting

Weighted-voting validation for consortium-validated work. Uses
`block.timestamp` for the fallback randomness seed (the block-level
VRF is preferred when available).

### `FeeDistributor` — fee splitting

Distributes fees collected by the marketplace, clearing house, and
other producers across registered recipients according to on-chain
weights. No timestamp dependencies.

### Planned: `ClearingHouse`, `BtleVault`, `PredictionRegistry`

`ClearingHouse` is the yield-perpetual settlement contract; its full
interface and how it wires into the TEE clearing engine is in
`04-defi-and-operations.md`. `BtleVault` is the BTLE companion
documented in the precompile section above. `PredictionRegistry` is
the on-chain surface for predictive foraging, covered in
`03-agent-systems.md`.

### Contract dependency order

```
1. MockERC20 / DAEJI         (no dependencies)
         |
         v
2. AgentRegistry             (basic deploy has no deps)
         |
         v
3. IdentityRegistry          (depends on DAEJI for tier staking)
         |
         v
4. ReputationRegistry        (depends on IdentityRegistry / AgentRegistry)
         |
         v
5. ValidationRegistry        (depends on IdentityRegistry / AgentRegistry)
         |
         v
6. Escrow (internal lib)     (depends on DAEJI)
         |
         v
7. BountyMarket              (depends on all of the above)
         |
         v
8. InsightBoard              (depends on DAEJI for rewards)
         |
         v
9. WorkerRegistry            (depends on DAEJI)
         |
         v
10. ConsortiumValidator      (depends on Registries)
         |
         v
11. FeeDistributor           (depends on DAEJI)
```

### What lives on the EVM plane vs. the native plane

The split is deliberate. Agent identity CRUD, reputation scoring,
work attestation, the knowledge ledger anchor, the marketplace, the
token, the clearing house, and fee distribution are all simple
storage reads and writes that Solidity handles at reasonable gas
cost. They live as contracts so iteration does not require chain
modifications.

The native plane is reserved for operations that need direct chain
internals (the QMDB Merkle structure, the validator group key, the
threshold-VRF store) or that are computationally infeasible in
Solidity (10,240-bit SIMD vector math, BLS12-381 pairings). Adding
an operation as a precompile means a coordinated node-binary
upgrade; the bar for crossing that line is "this cannot reasonably
be a contract".

---

## Current Devnet Limitations Affecting Contracts

Two node-level prerequisites must land before the full contract
suite can be redeployed to the live Daeji devnet. Both are small,
well-scoped, and queued in `05-roadmap.md`.

### 1. Wall-clock `block.timestamp`

The current devnet sets `block.timestamp` to block height instead of
Unix-seconds wall-clock time. The fix is to switch the block-context
construction to
`SystemTime::now().duration_since(UNIX_EPOCH).as_secs()` and have
verifiers accept proposed timestamps within a small window of their
own clock.

This is the single largest blocker on the deployed contract suite.
Every contract that uses cooldowns, decay, or deadlines depends on a
real wall-clock timestamp:

- `IdentityRegistry`'s prompt-update delay and withdrawal cooldown.
- `ReputationRegistry`'s 30-day half-life and 7-day grace period.
- `InsightBoard`'s on-read decay (`age = block.timestamp -
  entry.timestamp`).
- `BountyMarket`'s deadline timeouts and abandonment penalties.
- `WorkerRegistry`'s liveness windows.

Until this lands, every timelock effectively never expires and every
decay formula computes nonsense.

### 2. `BLOCKHASH` ring buffer

The `BLOCKHASH` opcode currently returns zero for all inputs. The
fix is to maintain a fixed-size ring buffer of the most recent 256
block hashes inside the executor and route the opcode through the
buffer. The data is already available; only the wiring is missing.

Together with item 1, this completes the EVM-compliance fixes that
unblock the full contract suite. They are listed here as current
limitations rather than open design questions because the
implementation path is decided.

---

## Summary

| Surface | What it is | Why it matters |
|---|---|---|
| HDC precompile (`0x09`) | 10,240-bit Hamming search at 50,000 gas, fixed | Vector similarity at scale that Solidity cannot do |
| QMDB proofs precompile (`0x0B`) | Merkle inclusion / exclusion proofs against any historical state root, 30,000 gas | Historical state assertions inside contracts; cross-chain verification |
| BTLE precompile (`0x0C`) | IBE encrypt/decrypt to a future consensus view, 80,000 gas | Sealed-bid auctions, time-delayed reveals, sealed votes — all without trusted operators |
| Index page (`0xA0_`) | Validator-aggregated benchmark indices (ISFR at `0xA01`) | Consensus-level oracle data with no separate operator trust |
| Agent namespace (`0xA10`–`0xA1F`) | Reserved for ERC-8004 lookups, capability checks, tier checks | Contracts delegate identity gating to the chain instead of re-implementing it |
| Solidity suite (10 contracts) | Agent identity, reputation, validation, knowledge ledger, marketplace, token, fees | The on-chain application surface |

These are what the Nunchi blockchain offers above and beyond a
plain-vanilla REVM-based chain. Everything else can be built in
Solidity; these cannot.
