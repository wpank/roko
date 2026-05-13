# Roadmap: Recent Changes, Phased Plan, and the Path to Mainnet

This document is the forward-looking inventory: what has recently
shipped on the Daeji testnet, what is queued for the next round of
changes, and what the longer milestones look like.

The roadmap is organized into three tiers:

- **Phase 1 — Foundation.** Changes that unblock the rest of the
  contract suite and the agent integration. Mostly small, mechanical,
  dependency-shaped.
- **Phase 2 — Differentiated Surface.** New chain capabilities that
  exercise primitives no standard EVM chain provides. Includes
  precompile wiring, the knowledge-layer redesign, and the
  agent-namespace expansion.
- **Phase 3 — Multi-Operator and Beyond.** Changes that depend on the
  chain being run by more than one party. Validator-set evolution,
  real fee markets, full economic instruments.

The phasing is by dependency, not by calendar. Phase 1 items
genuinely unblock everything else; Phase 3 items genuinely require
Phase 1 and 2 to have settled.

---

## Recent Changes

The Daeji testnet has reached the point where the foundational
architecture described elsewhere in this folder runs end-to-end. The
following pieces are stable and in regular use on the current devnet:

- **Simplex BFT consensus** under threshold BLS12-381, with
  single-slot finality at ~400 ms cadence on a 4-validator devnet.
- **REVM execution** with the standard Ethereum precompile set
  (`0x01`–`0x09`).
- **QMDB state storage** with deterministic transition-hash state
  roots.
- **Authenticated P2P overlay** with Ed25519 identity for both
  validators and secondaries.
- **Standard JSON-RPC surface** (`eth_*`, `net_*`, `web3_*`) plus the
  initial `kora_*` namespace (`kora_nodeStatus`).
- **Trusted-dealer and interactive Joint-Feldman DKG** ceremonies,
  both exposed through the `keygen` binary and integrated with the
  Docker and native bring-up paths.
- **In-process E2E test harness** using the deterministic runtime
  and simulated P2P, with fault injection and fully reproducible
  multi-validator runs.
- **EIP-1559 transaction acceptance** (with gas effectively free
  pending the base-fee market).
- **Per-validator Prometheus metrics** with optional Grafana
  dashboards via the observability Docker Compose profile.
- **Transaction load generator** (`loadgen`) for throughput
  measurement.
- **Ten production-intent Solidity contracts** compile and have
  been exercised against local Anvil and the in-process EVM fork
  simulator. Live deployment to the Daeji devnet is gated on the
  two prerequisite fixes below.

---

## Phase 1 — Foundation

These items are queued in roughly the order they unblock the rest of
the roadmap. Each is small and well-scoped; the gating factor is
sequencing rather than design.

### 1. Wall-clock `block.timestamp` (PREREQUISITE)

Switch the block-context construction to use
`SystemTime::now().duration_since(UNIX_EPOCH).as_secs()` instead of
block height. Verifiers should accept proposed timestamps within a
small window (e.g., ±30 seconds) of their own clock.

This is the single largest blocker on the production-intent contract
suite — every contract that uses cooldowns, decay, or deadlines
depends on it.

### 2. `BLOCKHASH` ring buffer (PREREQUISITE)

Maintain a fixed-size ring buffer of the most recent 256 block
hashes inside the executor; route the `BLOCKHASH` opcode lookup
through the buffer instead of returning zero. The data is already
available — only the wiring is missing.

Together with item 1, this completes the EVM-compliance fixes that
unblock contract deployment.

### 3. Custom precompile registry

Replace REVM's default `build_mainnet()` precompile preset with a
custom builder that registers chain-specific precompile addresses
alongside the standard Ethereum set. The mechanism is the gate that
lets the next several roadmap items land.

The first registrations land at the chain-specific HDC search
address (`0x09` overload), the QMDB historical-proofs precompile
(`0x0B`), and the agent-namespace identity-lookup at `0xA10`. BTLE
encryption (`0x0C`) is registered alongside these.

### 4. Coinbase derivation from validator identity

Set the EVM `beneficiary` field to a deterministic Ethereum address
derived from the proposing validator's Ed25519 public key
(`Address::from_slice(&keccak256(pubkey)[12..])`). Required before
any contract can attribute fees or rewards to validators.

### 5. WebSocket subscription endpoints

Wire `eth_subscribe` for `newHeads`, `logs`, and
`newPendingTransactions` through the existing JSON-RPC server. The
underlying library supports this natively; the change is wiring
rather than research. Reduces client polling load and is a
prerequisite for any real-time dashboard or indexer.

### 6. `kora_*` RPC extensions

Add the next set of `kora_*` methods alongside the existing
`kora_nodeStatus`:

- `kora_vrfSeed(blockNumber)` — explicit VRF-seed retrieval for the
  given block.
- `kora_recentBlocks(count)` — recent finalized blocks with VRF
  seed, transaction count, gas used, and timestamp.
- `kora_consensusHealth()` — superset of `kora_nodeStatus` with
  per-validator participation rate and rolling block-time average.

These are read-only convenience wrappers over data that already
exists.

### 7. Contract redeployment to the live devnet

Once items 1 and 2 land, redeploy the existing contract suite to
the live Daeji devnet. No Solidity changes are needed. After this,
the contract surface described in `02-precompiles-and-contracts.md`
is exercisable end-to-end on the devnet for the first time.

---

## Phase 2 — Differentiated Surface

These items ship the chain features that make Daeji distinct from a
standard EVM L1.

### 8. HDC similarity-search precompile

Wire the HDC similarity-search precompile at the chain-specific
overload of address `0x09`. The precompile maintains its own
in-memory index of HDC vectors keyed by content hash, rebuilt from
`InsightPosted` events at each finalized block. Search is constant
gas regardless of entry count.

This is the load-bearing primitive for the cross-fleet knowledge
layer. Without it, on-chain similarity search across more than a
few thousand entries is gas-prohibitive.

### 9. QMDB historical-proofs precompile

Wire the historical-proofs precompile at `0x0B`. Given a block
number and a key, it returns a Merkle inclusion or exclusion proof
verifiable against that block's state root.

This is the chain-internal alternative to `eth_getProof` (which the
QMDB transition-hash model does not support). It enables historical
state assertions inside contracts (proving "key K had value V at
block N" as part of a settlement, dispute, or audit flow).

### 10. BTLE encryption precompile

Wire the BTLE precompile at `0x0C`. Encrypts a payload to a future
consensus view; decrypts when that view's threshold signature
becomes available. Allows sealed-bid intents, sealed knowledge
reveals, and sealed solver submissions inside contracts without
requiring an external commit-reveal round.

The cryptographic primitives exist as Commonware library code; the
wiring is what is missing.

### 11. Agent-namespace precompile page

Begin populating the `0xA10`–`0xA1F` page with the operations that
contracts most need to delegate to the chain rather than
re-implement in Solidity:

- `0xA10`: ERC-8004 passport lookup by Ethereum address.
- `0xA11`: Capability-bit check (does the agent at address X hold
  capability Y?).
- `0xA12`: Tier check (is the agent at address X at tier Y or
  above?).
- `0xA13`: Reputation-minimum check (is the agent's domain-Y score
  at least Z?).

Each new entry is small in code and large in commitment; the page
grows incrementally as products demonstrate need.

### 12. Knowledge-layer redesign rollout

Extend `InsightBoard` from its current minimal shape (post a
content hash, mark confirmations) to the full design:

- Six knowledge kinds with type-specific half-lives (Insight,
  Heuristic, Warning, AntiKnowledge, CausalLink, StrategyFragment).
- On-read decay computed from `block.timestamp`, half-life, and
  confirmation count (depends on Phase 1 item 1).
- Challenge mechanism with stake and slashing.
- AntiKnowledge conflict detection via HDC similarity (depends on
  Phase 2 item 8).
- Tier-promotion rules (Transient → Working → Consolidated →
  Persistent) reflecting the off-chain knowledge store's tier
  system.

This rollout is staged: kinds and on-read decay first, then
challenges, then HDC-gated AntiKnowledge handling once the
precompile is live.

### 13. Validator-aggregated oracle wiring

Bring the ISFR-class oracle pipeline online end-to-end on the
devnet. Each validator gains a configured connection to a full node
of every source chain (Ethereum mainnet, HyperEVM, beacon chain).
Per-validator submissions, dual-median aggregation in consensus,
the publication state machine (Live / Degraded / Stale / Halted),
and snapshot history all become live.

The aggregation logic exists as Rust library code; the missing
piece is the validator-host configuration and the consensus-loop
integration.

### 14. TEE clearing wiring

Deploy the `ClearingHouse` contract suite, the BTLE-backed
sealed-intent flow, the solver-bond contracts, and the KKT
verifier. Solver code is initially a reference implementation;
over time the solver market opens to operators.

This is the first end-to-end exercise of the settlement pattern
that the chain was designed for and unblocks the yield-perpetual
product.

### 15. Finality-certificate export

Add `kora_finalityCertificate(blockNumber)` to the RPC, returning
the 240-byte BLS aggregate alongside enough metadata for an
external verifier to bind the certificate to a specific block.
Ship a reference verifier contract for an initial target chain
(HyperEVM is the leading candidate, given the yield-perpetual
settlement path).

### 16. Local episode-witness MMR

Deploy a Merkle Mountain Range contract for entry-level inclusion
proofs, accepting `blake3` hashes of episode records and emitting
MMR peaks. External chains can verify entry-level proofs against
the MMR without depending on the full state root being a trie
root. This is the partial answer to the "no `eth_getProof`" gap;
full state proofs are the QMDB-precompile path (item 9).

---

## Phase 3 — Multi-Operator and Beyond

These items only become meaningful once the chain is operated by
parties beyond the original deployers. They are sketched here so
that earlier phases do not preclude them.

### 17. Variable validator-set size

Replace the hardcoded `4` in leader election with a configurable
constant read from genesis. This is a one-line change that does
not add dynamic membership but allows deploying with different
fixed set sizes (3, 5, 7, etc.) without rebuilding the binary.

### 18. Validator-registry contract and staking

Deploy a validator-registry contract that manages admission, stake
posting, and exit. Tie validator-set membership to entries in this
contract. The registry exists at the application layer; the chain
only reads it during epoch transitions.

### 19. Resharing protocol

Wire the resharing primitive into validator-set management. This
is the load-bearing piece that lets the validator set change while
keeping the 48-byte group public key constant — preserving the
verifiability of every historical finality certificate. Without
resharing, every membership change invalidates external verifiers.

### 20. Slashing surface

Define and enforce slashing for the conduct that matters at this
phase: double-signing, extended downtime, signing an invalid state
root. The slashing contract is application-level; the chain
reports the conduct evidence.

### 21. Real EIP-1559 base-fee market

Implement the EIP-1559 base-fee adjustment in the executor and
emit it from block construction. Required for genuine economic
spam-resistance once the chain is operated by parties whose
interests are not aligned with each other.

### 22. Token deployment with demurrage

If the open token-economics decision lands on a real token, deploy
it with the demurrage and emission model decided there. Until
then, the `MockERC20` (`DAEJI` test token) is the deployed
surface.

### 23. PredictionRegistry contract

Deploy the on-chain predictive-foraging surface: predictions
registered before tasks, outcomes recorded after, residuals fed
back into knowledge-retrieval weights. Connects to the off-chain
prediction and calibration loop in the agent runtime.

### 24. BountyMarket and KnowledgeFutures activation

Activate the cross-agent bounty marketplace and the
knowledge-futures primitive (pre-sell knowledge before producing
it, with stake slashable on non-delivery). Both contracts exist at
the application layer; activation depends on the token decision.

### 25. EIP-4844 blob-sidecar storage

Add storage and serving for EIP-4844 blob sidecars. Not needed
until the chain provides data availability for external systems.
Listed here so that the surface is part of the long-term plan
rather than an afterthought.

### 26. Block-STM parallel execution (deferred)

Original specifications proposed parallel EVM execution via
software-transactional-memory techniques. This is premature
optimization at the current scale — the existing sequential
executor handles devnet workloads with substantial headroom — and
is listed here only to mark that it is not on the active path.

### 27. Extended block-header fields (deferred)

Original specifications proposed adding chain-specific fields
(active-agent count, knowledge-entry count, separate knowledge
state root) to the block header. The current posture is to track
these values in contract state instead, preserving header
compatibility with standard Ethereum tooling. Header extensions
remain an option for future revisiting, but are explicitly
deferred.

---

## Novel Features in the Pipeline

Several capabilities described in earlier docs become possible only
after the corresponding roadmap items above land. The shipping
condition for each is summarized so no one mistakes a
designed-but-deferred feature for a current capability.

### Block-level VRF as a public surface

Already produced by consensus on every block (the
`prevrandao`/`mixHash` field). Already accessible from Solidity
(`block.prevrandao`) and from JSON-RPC (`eth_getBlockByNumber`,
read `mixHash`). Roadmap item 6 (`kora_vrfSeed`) adds an explicit
getter.

### Sealed-bid intents and time-locked reveals (BTLE)

Cryptographic primitives exist as library code. Surfacing requires
the BTLE precompile (Phase 2 item 10) and the BTLE-backed
contract surface in the clearing-house (Phase 2 item 14).

### Cross-chain finality certificates

Already produced internally; need the export RPC (Phase 2 item
15) and a reference verifier on a target chain. The export shape
is deliberately small (240 bytes per certificate) so that external
chains without EIP-2537 BLS precompiles can still verify them,
albeit at higher gas cost than chains that have them.

### Deterministic full-network simulation

Already shipping in the in-process E2E harness. The next steps
are exposing the harness as a reusable library (so external
systems can import it) and adding library-driven contract
deployment and state-assertion helpers. This unlocks
"spin up the entire network in a single test" patterns for any
downstream consumer.

### Validator-aggregated source-protocol benchmarks

Designed end-to-end (see `03-agent-systems.md` and
`04-defi-and-operations.md`). Phase 2 item 13 is the wiring step.
After it ships, the indices described for DeFi integration are
live for the first time on the devnet.

### TEE-cooperative-clearing engine

Designed end-to-end. Phase 2 item 14 is the wiring step. The
KKT-verifiable settlement pattern works against the chain only
once both BTLE and the contract suite are in place.

### Knowledge-layer with on-read decay and challenges

Designed end-to-end. Phase 2 item 12 rolls it out incrementally
on top of the existing `InsightBoard`. The challenge mechanism
depends on the token decision (Phase 3 item 22).

### Agent-namespace expansion

Designed. Phase 2 item 11 begins populating the page. Future
entries land as products demonstrate need; the chain's preference
is to keep the page small until each addition is justified by an
actual contract.

### Resharing-stable chain identity

Designed. Phase 3 item 19 is the wiring step. Once it lands, the
validator set can evolve without breaking historical certificate
verification.

---

## Mainnet Path

There is no committed mainnet date. The honest dependency graph:

1. **Phase 1 items must land.** The two prerequisite fixes
   (`block.timestamp`, `BLOCKHASH`) and the contract-redeployment
   to the live devnet are the minimum credible production
   foundation.
2. **Phase 2 items 8–14 must land for the chain's distinguishing
   features to be exercisable.** Without them, the chain is
   "another EVM L1 with Simplex consensus", which is not the value
   proposition.
3. **The token-economics decision must be resolved.** Mainnet
   without a decided token is operable but not credible — the
   spam-resistance and validator-economy stories both need an
   answer.
4. **The validator-set decisions must be resolved well enough to
   allow at least one independent operator.** "Mainnet" operated by
   one party is hard to distinguish from an advanced testnet.
5. **A specific product needs to be running with real
   counterparties.** The leading candidate is the yield-perpetual
   settlement path (Phase 2 item 14 plus Phase 2 item 15 to export
   to a target chain like HyperEVM). Without a concrete product,
   "mainnet" is a label without a referent.

The roadmap above is sequenced to make each of these conditions
addressable in turn. Phase 1 is mechanical and small. Phase 2 is
the substantial work and is gated on Phase 1. Phase 3 is gated on
those decisions and on at least one second operator showing up to
use the chain.

The intermediate stations — a public testnet (Phase 1 + a subset
of Phase 2, operated by the original team across multiple regions)
and a multi-operator testnet (Phase 1 + Phase 2 + early Phase 3
items 17–19, operated by a small set of named operators) — are
explicit waypoints rather than separate brands. Each is a state
the network passes through on the way to mainnet rather than a
permanent destination.

---

## What Will Not Ship

Some items from earlier specifications are deliberately not on the
roadmap. Listed here to make the absence visible rather than
implicit:

- **Sentinel agents** that adversarially probe knowledge entries
  for inconsistencies. Valuable in a public network with untrusted
  participants; not justified for the current trust model. Revisit
  if and when the network opens to external participants who would
  need this defense.
- **Cross-chain bridges to non-EVM chains** (Cosmos, Solana, etc.).
  The 240-byte certificate is in principle verifiable from any
  chain with BLS12-381 pairing support, but no specific non-EVM
  target has the demand to justify a verifier.
- **Native generic NFT or social-graph standards.** These belong on
  general-purpose L1s; deploying them here would dilute the
  application-specific posture.
- **Custom block header extensions** beyond what the standard
  Ethereum shape provides. Tracked in contract state instead. The
  cost of breaking compatibility with standard tooling is not
  justified at the current scale.
- **Protocol-level token minting in block rewards.** Token
  economics, if any, belong in a contract; baking minting into the
  protocol layer would lock in choices the chain prefers to leave
  to governance.

---

## Summary

Phase 1 is a small set of mechanical fixes and small additions
that unblock the deployed contract suite. Phase 2 ships the
differentiated chain surface (precompile pages, validator-
aggregated oracle, TEE clearing, knowledge-layer redesign,
finality-certificate export). Phase 3 prepares the chain for
operation by parties beyond the original deployers (variable
validator set, staking, resharing, real fee market, real token).
Mainnet is a function of these phases plus the open economic and
operator decisions resolving — there is no useful date for it
independent of those resolutions.
