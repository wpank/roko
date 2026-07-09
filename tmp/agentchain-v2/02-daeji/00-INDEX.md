# Index — The Nunchi Blockchain (Daeji Testnet)

The Nunchi blockchain is a sovereign, application-specific Layer-1
blockchain built from composable Rust primitives supplied by the
Commonware library. It is **not** a fork of go-ethereum, Reth, or any
existing Ethereum client; it is assembled from three independent
components — Commonware's Simplex Byzantine-Fault-Tolerant consensus
under BLS12-381 threshold signatures, REVM (the Rust Ethereum Virtual
Machine also used by Foundry and Reth) for contract execution, and
Commonware's QMDB authenticated key-value storage — and exposes the
standard Ethereum JSON-RPC interface so any Ethereum tooling works
against it unchanged. On top of the base EVM it adds a small set of
custom precompiles for operations infeasible at scale in Solidity
(fast Hamming-distance search over hyperdimensional knowledge vectors,
historical state proofs, pairing-based encryption to future consensus
views), embeds composite market-data indices directly into validator
consensus, and surfaces compact ~240-byte finality certificates as a
natural byproduct of threshold consensus. This folder describes the
chain side of the agentchain stack from first principles.

---

## Names

- **Nunchi** — the brand. The project and company that builds the
  blockchain, the agent runtime, and the financial products that run
  on top of both.
- **Nunchi blockchain** — the chain this folder describes.
- **Daeji** — the testnet name for the current devnet deployment of
  the Nunchi blockchain. The chain itself is "the Nunchi blockchain";
  the testnet is "Daeji".
- **Roko** — the agent runtime, a separate Rust toolkit that runs
  autonomous coding and DeFi agents. Roko *consumes* the chain; it is
  covered in its own documentation set. This folder mentions Roko
  only when explaining how it interacts with the chain.
- **agentchain** — the umbrella term for the full stack (chain + agent
  runtime + market products).
- **Korai** — legacy internal codename for the chain. Older sources
  sometimes say "Korai" or "Kora"; the current name is "the Nunchi
  blockchain". References to Korai should be read as historical.

---

## Doc Map (5 content + this index)

| # | Doc | One-line summary |
|---|---|---|
| 01 | `01-foundations.md` | How the chain is built: Commonware composable primitives, Simplex BFT consensus with BLS12-381 threshold signatures, REVM execution, QMDB state, block time and validator topology, JSON-RPC and the `kora_*` namespace. |
| 02 | `02-precompiles-and-contracts.md` | The on-chain extension points and the contract surface: HDC similarity at `0x09`, QMDB historical proofs at `0x0B`, BTLE encryption at `0x0C`, the reserved `0xA10`–`0xA1F` agent namespace; the ten production-intent Solidity contracts; the two prerequisite node-level fixes (`block.timestamp`, `BLOCKHASH`). |
| 03 | `03-agent-systems.md` | Chain features built specifically for autonomous agents: ERC-8004 identity / reputation / validation registries, the on-chain knowledge ledger and NeuroChainSync, the validator-computed oracle system with agent prediction scoring, the TEE-based cooperative batch clearing engine, BTLE and ad-hoc DKG for private collaboration, and the four classes of proof. |
| 04 | `04-defi-and-operations.md` | DeFi integration (source-protocol reads from Aave V3, Compound V3, Hyperliquid, Ethena, ETH staking; the AAVE backstop use case; the HyperEVM / HIP-3 bridge target) and how to operate the chain (devnet topology, the Docker and native bring-up paths, RPC endpoints, observability, the in-process E2E harness, load testing, coexistence with Roko and fork simulators). |
| 05 | `05-roadmap.md` | Recent changes and the three-phase plan. Phase 1 unblocks the deployed contract suite. Phase 2 ships the differentiated chain surface (precompiles, validator-aggregated oracle, TEE clearing, knowledge-layer redesign, finality-certificate export). Phase 3 prepares the chain for multiple operators. The mainnet path. |

---

## Glossary

Every chain-side term that appears in this folder, defined in one
place.

### Consensus, cryptography, networking

- **Simplex BFT** — the Byzantine-Fault-Tolerant consensus protocol
  the chain uses (Chan and Pass, IACR ePrint 2023/463). Three
  message hops to finality (propose, vote, finalize) with single-slot
  finality once a block is finalized.
- **Byzantine Fault Tolerant (BFT)** — a class of consensus protocols
  that produce correct results even when up to `f` of `N` validators
  misbehave (typically `N >= 3f + 1`).
- **Single-slot finality** — once a block is finalized in its slot,
  it cannot be reverted by any subsequent computation. Distinct from
  probabilistic finality (Bitcoin) where reversal becomes
  progressively unlikely.
- **BLS12-381** — the pairing-friendly elliptic curve used for
  threshold signatures, the threshold VRF, and BTLE. 48-byte group
  public keys, 96-byte signatures, single-pairing verification.
- **Threshold signature** — a cryptographic signature where any T of
  N participants combine their shares to produce a valid signature
  verifiable against a single group public key. The full private
  key is never assembled in one place.
- **DKG (Distributed Key Generation)** — the multi-party protocol
  that produces threshold key shares without any participant ever
  holding the full private key. Two modes: trusted-dealer (fast,
  for development) and interactive Joint-Feldman (no single party
  sees the full secret; for production-like environments).
- **VRF (Verifiable Random Function)** — a function whose output is
  deterministic, unpredictable without the key, and verifiable by
  anyone with the public key. On the Nunchi blockchain, the
  threshold signature over each view number serves as the VRF
  output and is exposed as the block's `prevrandao`/`mixHash`
  field.
- **Ed25519** — the elliptic-curve signature scheme used for P2P
  identity. Distinct from secp256k1 (Ethereum transaction signing)
  and BLS12-381 (consensus).
- **Authenticated P2P** — Commonware's mutually authenticated,
  encrypted overlay network. Every peer is identified by an Ed25519
  public key; unauthenticated peers cannot connect.
- **Secondary peer** — a read-only follower node. Replicates all
  blocks via the P2P overlay but does not participate in consensus
  voting. Useful as a local push-event source and as the
  integration point for downstream consumers wanting low-latency
  block notifications without polling.
- **Resharing** — the cryptographic protocol that redistributes
  threshold key shares to a new validator set while keeping the
  48-byte group public key constant. Preserves the verifiability of
  every historical finality certificate across membership changes.

### Execution and storage

- **REVM** — the Rust Ethereum Virtual Machine. The same EVM engine
  used by Foundry and Reth. Executes standard EVM bytecode without
  modification.
- **EVM plane** — the execution environment Solidity contracts see.
  Standard Ethereum semantics plus custom precompiles.
- **Native plane** — the Rust execution environment the node
  implementation maintains. Consensus, cryptography, P2P, and
  direct state access live here.
- **QMDB (Quick Merkle Database)** — Commonware's authenticated
  key-value store. Sub-millisecond updates with in-memory
  Merkleization. Every commit produces a state root included in
  the block header.
- **State root** — the cryptographic commitment to the chain's
  state at a given block. On Daeji, the state root is a QMDB
  transition hash, not a Merkle Patricia Trie root.
- **Transition hash** — the deterministic formula
  `keccak256("_KORA_STATE_TRANSITION_ROOT" + parent_root +
  serialized_changes)` used as the state root. Deterministic but
  not a trie root, so Merkle inclusion proofs against it are not
  directly available via `eth_getProof`.
- **MMR (Merkle Mountain Range)** — Commonware's append-only
  authenticated log. Used for entry-level inclusion proofs without
  modifying the state-root model.
- **EIP-1559** — the standard Ethereum transaction format with a
  base-fee market. The chain accepts EIP-1559 transactions; the
  base-fee adjustment itself is queued for a later phase.
- **`block.timestamp`** — the EVM block context's timestamp field.
  On the current devnet, set to block height (a known limitation
  with a queued one-line fix).
- **`BLOCKHASH`** — the EVM opcode that returns recent block
  hashes. Currently returns zero; ring-buffer fix is queued.
- **`prevrandao` / `mixHash`** — the block-header field carrying
  the threshold-VRF output. Solidity reads it as
  `block.prevrandao`.
- **Coinbase / beneficiary** — the EVM block context's proposer
  address. Currently zero; derivation from the proposing
  validator's Ed25519 key is on the roadmap.

### Precompiles and the agent namespace

- **Precompile** — a piece of native Rust code at a fixed EVM
  address. Called like a contract but executes in native code with
  full access to chain internals. Standard Ethereum has 9
  precompiles (`0x01`–`0x09`); the Nunchi blockchain adds
  chain-specific ones at reserved addresses.
- **HDC similarity-search precompile** — at the chain-specific
  overload of `0x09`. Performs Hamming-distance similarity search
  across hyperdimensional knowledge vectors. Native SIMD POPCNT;
  ~170 microseconds for 100K entries; flat 50,000 gas per call.
- **QMDB historical-proofs precompile** — at `0x0B`. Returns
  Merkle inclusion or exclusion proofs for any key at any
  finalized block. 30,000 gas.
- **BTLE precompile** — at `0x0C`. Pairing-based encryption to a
  future consensus view; decryption becomes possible automatically
  when that view's threshold signature is finalized. 80,000 gas
  for either encrypt or decrypt.
- **Agent namespace (`0xA10`–`0xA1F`)** — reserved page for
  agent-related precompiles (passport lookup, capability check,
  tier check, reputation-min check). Populated incrementally.
- **Index precompile page (`0xA0_`)** — reserved page for
  validator-aggregated indices. The ISFR rate publishes at
  `0xA01`. Reads are constant-gas.

### Identity, reputation, knowledge

- **ERC-8004** — the agent-identity standard adopted by the chain.
  Three composable registries: Identity, Reputation, Validation.
- **Identity Registry** — soulbound (non-transferable) ERC-721
  passport per agent. Carries capability bitmask, tier,
  system-prompt hash, TEE attestation hash, and Agent Card URI.
- **Reputation Registry** — on-chain authorization plus raw
  feedback events. The 7-domain EMA score itself is computed
  off-chain.
- **Validation Registry** — work proofs and validator
  attestations. Four validator types (reputation-based,
  stake-secured re-execution, zkML, TEE oracle).
- **Capability bitmask** — 64-bit field on each passport. 14
  capabilities currently defined; bits 14–63 reserved.
- **Tier** — Protocol, Sovereign, Worker, Edge. Tier-based
  staking thresholds gate participation in higher-stakes
  operations.
- **System-prompt hash** — committed on-chain to enable
  ventriloquist-defense (proving the running agent uses the
  prompt its operator claims).
- **TEE attestation** — a hash committed to the agent's Trusted
  Execution Environment attestation, proving the runtime is the
  expected hardware-isolated environment.
- **Agent Card** — a JSON document with an agent's name,
  description, endpoints, and payment info. URI-referenced from
  the passport.
- **`InsightBoard`** — the on-chain knowledge-ledger contract.
  Posts carry a content hash; full content lives in event logs.
- **Knowledge kinds** — Insight, Heuristic, Warning,
  AntiKnowledge, CausalLink, StrategyFragment. Each has a
  type-specific half-life.
- **Knowledge tiers** — Transient → Working → Consolidated →
  Persistent. Promotion is driven by independent confirmations
  across distinct task contexts.
- **AntiKnowledge** — explicitly negative knowledge entries used
  to reject contradicting future entries via HDC similarity.
- **HDC (Hyperdimensional Computing)** — a representation that
  encodes text and other data as 10,240-bit binary hypervectors.
  Similarity is measured by Hamming distance, computable in
  microseconds via hardware POPCNT.
- **Hypervector** — a 10,240-bit binary vector (`[u64; 160]`,
  1,280 bytes). The atomic unit of HDC representation.
- **Pheromone counter** — the on-chain confirmation count for a
  knowledge entry. Stigmergic coordination signal.
- **NeuroChainSync** — the bidirectional protocol between an
  off-chain knowledge store and the on-chain `InsightBoard`.
  Local entries meeting promotion criteria push to chain; chain
  entries from other parties pull into the local store at
  Transient tier.
- **Predictive foraging** — committing falsifiable predictions
  before task execution and scoring the residual against the
  actual outcome. Calibrates future knowledge-retrieval
  weighting.
- **TraceRank** — the multi-dimensional reputation composite over
  consistency, breadth, depth, recency, and collaboration,
  propagated PageRank-style over the agent-interaction graph.

### Oracle and settlement

- **ISFR (Internet Secured Funding Rate)** — the chain's flagship
  composite benchmark, computed inside consensus from four
  source-protocol classes (LENDING, STRUCTURED, FUNDING, STAKING).
  Methodology lives in a sibling spec; the chain side describes
  aggregation, publication, and consumption.
- **Source-protocol class** — LENDING (Aave V3, Compound V3),
  STRUCTURED (Ethena), FUNDING (Hyperliquid perp funding rate),
  STAKING (Ethereum beacon chain).
- **Dual-median aggregation** — each validator computes a
  weighted median across sources; the chain computes a
  stake-weighted median across validators. Resists both
  source-side and validator-side manipulation.
- **Publication state machine** — Live → Degraded → Stale →
  Halted, with hysteresis on validator confidence (drop below
  70% to leave Live, climb above 80% for three consecutive
  periods to return).
- **TEE clearing** — cooperative batch matching inside a Trusted
  Execution Environment. Decrypts sealed orders inside the
  enclave, computes the surplus-maximizing uniform clearing
  price, emits a KKT certificate.
- **KKT certificate** — a mathematical proof that a clearing
  solution is globally optimal under the stated constraints. The
  chain verifies the certificate in O(n) by walking the order
  set once.
- **Per-block phase ordering** — the fixed sequence of operations
  in every block: ORACLE → ACCRUAL → LIQUIDATION → MATCHING.
  Eliminates stale-mark liquidation attacks.
- **`ClearingHouse`** — the on-chain settlement contract suite
  for yield perpetuals. Manages positions, batches, settlement
  rounds, and liquidations.
- **`ClearingProfile`** — a single-signature on-chain intent
  declaring direction, trigger condition, max notional, max fee,
  and expiry.
- **`ClearingInsight`** — the structured event emitted per
  cleared batch (price, surplus, fill rates, solver identity).
- **Solver bond** — capital posted by clearing solvers; slashed
  if the chain's KKT verification rejects their submission.
- **Insurance fund** — the on-chain reserve that backs cascade
  liquidations. Accrues a small fixed fraction of every cleared
  trade.

### Private collaboration

- **BTLE (Binding Timelock Encryption)** — encryption to a
  future consensus view. Decryption becomes possible
  automatically when that view's threshold signature is
  finalized; no participant can refuse to reveal.
- **Sealed-bid auction** — an auction in which all bids are
  encrypted to the same future view. All bids decrypt
  simultaneously at the reveal view; no participant can see
  another's bid before committing.
- **Sealed knowledge reveal** — a knowledge entry committed
  on-chain as ciphertext that decrypts automatically at a
  specified future view. Establishes priority-of-discovery
  without immediate disclosure.
- **Ad-hoc DKG group** — a private group of agents that runs
  its own DKG ceremony for a private threshold key, separate
  from validator consensus.

### Proofs and certificates

- **Finality certificate** — a ~240-byte aggregate (48-byte
  group key, 96-byte threshold signature, ~96 bytes of
  metadata) verifiable in a single BLS12-381 pairing check.
  Lets external systems verify that a Daeji block was
  finalized.
- **Proof-of-work-done** — cryptographic evidence that an agent
  completed a specific task and passed validation gates.
  Anchored as episode-hash witnesses on chain.
- **Proof-of-learning** — evidence that knowledge posted by one
  agent has been independently confirmed by other agents
  through their task outcomes.
- **Witness anchoring** — the operation of committing
  `blake3(episode_data)` as transaction calldata, producing a
  finality-signed record of the episode hash.
- **Proof-of-Agent** — the four-dimensional autonomy proof
  combining TEE attestation, ventriloquist-defense
  prompt-hash check, on-chain reasoning commitment, and
  sealed-session input attestation.

### Other

- **Commonware** — the Rust library suite of independent
  blockchain primitives the chain is assembled from.
  Anti-framework: independent crates rather than a monolithic
  node.
- **`kora` / `keygen` / `loadgen`** — the three release
  binaries shipped with the chain. `kora` is the node
  (validator, secondary, or DKG mode); `keygen` runs the DKG
  ceremony and genesis setup; `loadgen` generates EIP-1559
  transactions for throughput testing.
- **`kora_*` namespace** — the chain-specific JSON-RPC
  namespace alongside the standard `eth_*`, `net_*`,
  `web3_*`. Currently ships `kora_nodeStatus`; additional
  methods are queued.
- **HyperEVM** — Hyperliquid's EVM-compatible execution
  environment. A target chain for cross-chain certificate
  verification and the intended HIP-3 deployment surface for
  yield perpetuals.
- **HIP-3** — Hyperliquid Improvement Proposal allowing
  builder-operated perpetual markets on HyperEVM.
- **EIP-2537** — Ethereum proposal adding BLS12-381
  precompiles. Affects the gas cost of verifying Daeji
  finality certificates from Ethereum mainnet.
- **ERC-8183** — proposed standard for on-chain agent task
  coordination, implemented by the `BountyMarket` contract.

---

## Reading Paths

### "I'm a chain engineer evaluating the architecture"

`01-foundations.md` → `02-precompiles-and-contracts.md` →
`03-agent-systems.md` (the proofs section in particular) →
`05-roadmap.md`.

### "I'm a smart-contract developer building on the chain"

`01-foundations.md` (the EVM/native plane sections) →
`02-precompiles-and-contracts.md` (the precompile ABIs and the
contract suite) → `03-agent-systems.md` (identity, reputation,
knowledge ledger). Pay attention to the current-devnet limitations
section in `02-precompiles-and-contracts.md`.

### "I'm building DeFi on top of the chain"

`01-foundations.md` (block model and JSON-RPC) →
`03-agent-systems.md` (the oracle system and the TEE clearing
engine) → `04-defi-and-operations.md` (the source-protocol read
path, settlement guarantees, the AAVE backstop, the HyperEVM
bridge target) → `05-roadmap.md` (which precompiles unblock what).

### "I'm an operator running a node"

`01-foundations.md` (consensus, DKG modes, validator topology) →
`04-defi-and-operations.md` (the operator sections — bring-up
paths, endpoints, configuration, observability, load testing) →
`05-roadmap.md`.

### "I'm a researcher interested in agentchain primitives"

`01-foundations.md` → `03-agent-systems.md` (entire doc — ERC-8004,
the knowledge layer, the validator-computed oracle, TEE clearing,
BTLE and DKG, the four proof classes) → `04-defi-and-operations.md`
(the coexistence section).

---

## Current State at a Glance

4-node devnet, ~400 ms blocks, ten production-intent contracts,
mainnet pending. The dependency graph is described in
`05-roadmap.md`; the honest summary is that mainnet depends on the
two prerequisite EVM-compliance fixes shipping, the differentiated
precompile surface landing, the open token-economics decision
resolving, and at least one second operator showing up to use the
chain. There is no useful date for it independent of those
resolutions.

For everything else, the doc map above is the authoritative entry
point.
