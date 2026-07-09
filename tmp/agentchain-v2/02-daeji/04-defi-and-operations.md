# DeFi Integration and Operating the Chain

This document covers two related concerns: the chain-side facilities that
DeFi applications consume (source-protocol reads, the validator-computed
benchmark, the clearing engine, the AAVE backstop use case, the
HyperEVM bridge target) and the practical mechanics of running the
chain (devnet topology, the bring-up paths, RPC endpoints,
observability, the in-process test harness, load testing). The closing
section draws the seam between the chain and the rest of the agentchain
stack — what runs on-chain versus off-chain, what the chain replaces
versus complements.

The full ISFR index methodology lives in a sibling spec; this document
covers only what the chain itself contributes to its computation,
publication, and consumption.

---

## What "DeFi Integration" Means Here

Three things have to fit together for an agent-mediated DeFi product to
work on the Nunchi blockchain:

1. **Source-protocol data has to land on-chain in a tamper-resistant
   form.** Aave V3, Compound V3, Hyperliquid, Ethena, and the Ethereum
   beacon chain all live somewhere else. Their rates and prices have
   to be brought into the chain's view in a way that no single party
   can rewrite.
2. **Agents have to be able to act on that data without a custodian.**
   Agents are not human users; they sign their own transactions,
   manage their own keys, and route trades through normalized
   adapters. The chain has to expose primitives that make this safe —
   risk caps, deterministic ordering, slashing for misbehavior.
3. **Settlement has to produce a verifiable record.** Every cleared
   batch must be reconstructible from on-chain data, every liquidation
   must be witnessable, and every payoff must be derivable from a
   published formula.

The chain owns layers 1 and 3. Layer 2 (the agent execution path)
lives in the Roko agent runtime and is described in its own
documentation set.

---

## Source Protocols and the Read Path

Four classes of external protocol contribute the underlying data the
benchmark indices and yield perpetuals settle against. The chain reads
them from the source — never from a centralized aggregator — by
giving each validator a direct connection to the source chain.

| Class | Source | What is read | Source chain |
|---|---|---|---|
| LENDING | Aave V3 | USDC supply APY | Ethereum mainnet |
| LENDING | Compound V3 | USDC supply APY | Ethereum mainnet |
| STRUCTURED | Ethena (sUSDe) | 7-day rolling yield | Ethereum mainnet |
| FUNDING | Hyperliquid | ETH perpetual funding rate | HyperEVM |
| STAKING | Ethereum beacon chain | Consensus rewards + MEV tips | Ethereum |

Each Nunchi validator runs (or has a direct websocket connection to) a
full node of every source chain. The validator queries the source
contract or beacon API directly — there is no shared endpoint, no
external oracle dependency, no off-chain aggregator the chain has to
trust. If a validator's source connection fails or returns stale data,
that validator's vote is excluded from the round; the chain does not
silently substitute a fallback.

### Liveness timeouts

Each source has a tolerance for how long it can be unavailable before
its contribution is dropped from the round.

| Source | Liveness timeout |
|---|---|
| Aave V3, Compound V3 | 120 seconds (~10 Ethereum blocks) |
| Ethena sUSDe | 24 hours |
| Hyperliquid funding | a few minutes |
| ETH staking | 30 minutes (~5 epochs) |

These timeouts feed the publication-state machine (Live → Degraded →
Stale → Halted) described in `03-agent-systems.md`, which the
settlement contracts read to decide whether to clear, widen spreads,
freeze liquidations, or fall back to an emergency continuous order
book.

---

## The ISFR Composite

**ISFR (Internet Secured Funding Rate)** is the chain's flagship
composite benchmark — the DeFi analogue of SOFR (the Secured
Overnight Financing Rate, the traditional-finance benchmark for
overnight secured lending in dollars). Full methodology lives in a
sibling spec; here is the chain-side flow.

### Source-class taxonomy and weights

| Class | Weight | What it measures | Sources |
|---|---|---|---|
| LENDING | 0.60 | Collateralized lending yield | Aave V3, Compound V3 |
| STRUCTURED | 0.25 | Multi-instrument strategy yield | Ethena sUSDe |
| FUNDING | 0.10 | Perpetual-futures funding rate | Hyperliquid ETH perp |
| STAKING | 0.05 | Proof-of-stake validator yield | ETH staking |

Composite formula:

```
ISFR = 0.60 × LENDING + 0.25 × STRUCTURED + 0.10 × FUNDING + 0.05 × STAKING
```

### Cadence

Updates happen every 25 blocks (approximately 10 seconds at the
current ~400 ms block time). The 10-second cadence produces ~8,640
updates per day — vs. one daily publication for SOFR. This update
density is what makes the prediction-scoring feedback loop in the
oracle system possible (covered in `03-agent-systems.md`).

### Worked example

Sources reporting:

| Class | Rate | Weight |
|---|---|---|
| LENDING | 6.20% | 0.60 |
| STRUCTURED | 7.10% | 0.25 |
| FUNDING | 12.40% | 0.10 |
| STAKING | 3.20% | 0.05 |

```
ISFR = 0.60 × 6.20 + 0.25 × 7.10 + 0.10 × 12.40 + 0.05 × 3.20
     = 3.720 + 1.775 + 1.240 + 0.160
     = 6.895%   (approximately 690 basis points)
```

The elevated FUNDING rate (12.40%) contributes only 124 bps because
its 10% class weight limits its influence. Under a flat equal-weight
average the speculative signal would pull the composite ~33 bps
higher. Two-level aggregation with class weights anchors the
benchmark to lending fundamentals where hedging demand concentrates.

### Median robustness vs. mean

A flash-loan attack spiking Aave's rate to 50%:

| Aggregation | Input | Result |
|---|---|---|
| Median | `[5.80, 6.20, 7.10, 50.00]` | 6.65% (65 bps transient distortion) |
| Mean | Same | 17.28% (1,128 bps spike) |

The median absorbs outliers; the mean amplifies them. Every credible
benchmark — SOFR, EURIBOR, ISFR — uses a median for exactly this
reason.

### Published values

Every round publishes five values via the `0xA01` precompile:

- ISFR (primary composite, also included in the block header).
- ISFR.LENDING, ISFR.STRUCTURED, ISFR.FUNDING, ISFR.STAKING.

Sub-indices are byproducts of computing the composite — zero
marginal cost.

### Hybrid rate: oracle plus market

Once a market exists, ISFR has two sources of truth: the oracle
layer (external DeFi rate measurement) and the clearing engine
(endogenous market discovery). The canonical ISFR combines them:

```
ISFR = ISFR_oracle + EMA(ISFR_market - ISFR_oracle)
```

At launch with thin clearing liquidity, `ISFR ≈ ISFR_oracle`. As
the yield-perpetual market deepens, `ISFR_market` becomes
progressively more informative and the benchmark transitions
smoothly toward endogenous discovery. No binary cutover.

---

## What the Chain Exposes to DeFi Contracts

### The index precompile (`0xA01`, on the `0xA0_` page)

The benchmark output is exposed as a precompile, not as a contract
that the oracle has to be "pushed" to by a keeper. Any Solidity
contract can read the current value with a fixed-gas call:

```solidity
interface IRateOracle {
    function current() external view returns (uint32 valueBps, uint8 state);
    function at(uint64 blockHeight) external view returns (Snapshot memory);
    function twap(uint64 startBlock, uint64 endBlock) external view returns (uint32);
}
```

Snapshots include the value in basis points, the block height and
timestamp, the publication state (Live / Degraded / Stale /
Halted), the validator confidence, and the number of active
sources. Historical snapshots are retained on-chain for 90 days, so
contracts can settle against the value as it existed at the moment
a round closed without trusting any external feed.

The precompile path is what makes per-batch settlement against a
benchmark practical. Reading is constant-time and constant-gas
regardless of how many sources contribute.

### The agent precompile namespace (`0xA10`–`0xA1F`)

Agent identity, reputation, and capability checks are precompiled
at the `0xA10`–`0xA1F` page. DeFi contracts use this page to gate
sensitive operations: "only an agent with reputation ≥ Silver and
the `clearing-solver` capability bit may submit a clearing
solution"; "this profile may only be activated by an agent that
holds an ERC-8004 passport". The DeFi contracts never hand-roll
identity checks — they delegate to the precompile.

### Historical state proofs (`0x0B`)

The QMDB historical-proofs precompile lets a contract verify that a
key had a specific value at a specific past block. Yield perpetuals
use this to settle disputes about the index value at the moment a
round closed: a counterparty challenging a settlement supplies a
Merkle proof against the snapshot's storage slot, and the chain
verifies the proof natively without re-execution.

### Block-level VRF

Because consensus is BLS12-381 threshold signatures, every block
carries a verifiable random value in the `prevrandao` field. DeFi
contracts use it for unbiased solver-rotation, liquidation-keeper
selection, and any other decision that needs randomness no
participant can grind. There is no external RNG dependency.

### BTLE for sealed clearing intents

Binding Timelock Encryption (covered in `03-agent-systems.md`) lets
a trader encrypt a clearing intent so that it can only be decrypted
at a specified future consensus view. This is the chain-side
primitive that prevents front-running of large profile
activations: the intent is committed in block N, becomes visible to
solvers only at block N+k, and the auction settles at the price
that prevailed at the unsealing.

---

## On-Chain Settlement Guarantees

The chain provides four guarantees during settlement:

1. **Per-block phase ordering.** Every block executes its
   operations in a fixed sequence: `ORACLE → ACCRUAL → LIQUIDATION
   → MATCHING`. Liquidations always see a fresh oracle tick;
   matched orders never settle against a stale mark. The ordering
   is enforced by the block builder, not a contract convention.
2. **Atomic batch settlement.** Once a clearing batch closes, the
   entire batch either settles in one transaction or none of it
   does. There is no partial commit visible to other contracts
   mid-batch.
3. **Verifiable optimality.** The TEE clearing engine produces a
   KKT certificate alongside every solution. The chain verifies the
   certificate in `O(n)` by walking the order set once. Any contract
   can replay the verification offline and reach the same verdict.
4. **Tamper-evident insight emission.** Each cleared batch emits a
   `ClearingInsight` record (clearing price, surplus, fill rates,
   solver identity, oracle value at close) that is anchored on-chain
   and forms the basis for solver and prediction-agent scoring.

What the chain does **not** provide: the order book itself
(continuous matching, market-maker quoting, and the emergency CLOB
live in application contracts), the clearing solver (off-chain
processes that compete to compute the surplus-maximizing price; the
chain only verifies their solutions), position management (margin
top-ups, profile expiry handling, the actual decision to enter or
exit a position live in agent runtimes and in the contracts that
hold positions), or the published interest-rate methodology (the
chain runs a generic consensus-aggregated-median oracle pipeline;
the choice of sources, weights, and circuit-breaker thresholds is
the index designer's responsibility).

---

## Agent-Mediated Trading: the Chain's Surface

From the chain's perspective, agent-mediated trading reduces to
four observable on-chain events and one ambient guarantee.

### The four events

1. **Profile creation.** A user submits one transaction to the
   `ClearingProfile` contract declaring direction, trigger rate,
   max notional, max acceptable fee, and expiry. Stored on-chain.
   Costs gas at posting time and zero gas thereafter while
   dormant.
2. **Profile activation.** The clearing engine detects the trigger
   condition and pulls the profile into a pending batch. No
   off-chain keeper is required — the consensus layer evaluates
   triggers as part of the per-block `ORACLE` phase.
3. **Solver submission.** A solver agent posts a `ClearingSolution`
   with a KKT certificate before the round-deadline window
   expires. Solvers must be registered in the agent registry, hold
   the relevant capability bit, and have posted the operator-set
   bond.
4. **Settlement emission.** The matched fills, the realized
   clearing price, the funding payments for the elapsed interval,
   and the resulting position adjustments are emitted as a single
   atomic block of events.

### The one ambient guarantee

The agent registry, reputation registry, and capability bitmask
checks are evaluated on every chain-touching action. An agent
whose reputation has dropped below tier, whose passport has been
revoked, or whose capability bits no longer authorize the
operation cannot submit the transaction in the first place — the
precompile reverts. DeFi contracts do not have to defensively
re-check identity; the chain has already done it.

---

## On-Chain Risk Management

The chain enforces three categories of risk constraint at the
protocol level. Application-level risk policies live in the agent
runtime and are out of scope here.

### Settlement-layer circuit breakers

The benchmark publication state machine gates settlement contract
behavior:

| Index state | Settlement contract behavior |
|---|---|
| Live | Normal clearing. New profiles activate. Liquidations proceed against fresh marks. |
| Degraded | Clearing continues with widened spreads. Tight-trigger profiles may pause. Liquidations still allowed. |
| Stale | Clearing continues against the last Live value. New profile activations blocked. New positions blocked. |
| Halted | Clearing pauses entirely. Liquidations frozen. Existing positions remain open at last Live mark. |

Hysteresis: drop below 70% to leave Live, climb above 80% for
three consecutive periods to return.

### Solver-bond slashing

Every solver posts a bond before being eligible to submit
solutions. If the chain's KKT verification rejects a solution, the
bond is slashed. If a solver wins by submitting a solution
provably worse than another competing submission within the same
window, the bond is partially slashed and the surplus share is
forfeited. Bond accounting is enforced by the clearing-house
contract, not by an off-chain trust assumption.

### Insurance-fund accrual

Every cleared trade contributes a small fixed fraction of notional
(currently 0.5 basis points) to an on-chain insurance fund. The
fund is the first source of capital for backstopping bad debt from
cascade liquidations. It accrues automatically as part of the
settlement transaction; no off-chain process has to remember to
top it up.

### Per-block phase ordering as a risk primitive

Because the per-block phase order is `ORACLE → ACCRUAL →
LIQUIDATION → MATCHING`, a position can never be liquidated
against a mark older than the block's own oracle update. This
eliminates the entire class of attacks where a counterparty uses a
stale price to liquidate a healthy position during an oracle
outage.

---

## The Clearing Profile: Consumer-Facing Intent

A clearing profile is what makes "set it and forget it" rate
hedging possible — a persistent on-chain intent that sits dormant
until market conditions activate it.

```solidity
struct ClearingProfile {
    address account;
    bytes32 market;          // Market ID (e.g., keccak256("ISFR-PERP-V1"))
    Direction direction;     // 0 = LONG, 1 = SHORT
    uint256 trigger;         // ISFR threshold in bps that activates
    uint256 maxNotional;     // Maximum USD exposure (1e18 scaled)
    uint16  maxFeeBps;       // Maximum acceptable clearing fee
    uint64  expiry;          // 0 = no expiry
    uint256 minFillNotional; // Minimum fill per round (anti-dust)
    uint32  maxRounds;       // 0 = unlimited rounds
}
```

Lifecycle:

1. **Creation.** User submits one transaction. Profile stored
   on-chain. Cost: ~50,000 gas (one storage write).
2. **Dormancy.** Profile sits on-chain. No keeper, no monitoring.
   The consensus layer checks trigger conditions during each ISFR
   update.
3. **Activation.** When ISFR crosses the trigger, the clearing
   engine includes the profile's order in the next batch. Order
   sized as `min(maxNotional − filledSoFar, availableCounterparty)`.
   If `maxFeeBps` is exceeded, the profile skips that round and
   retries.
4. **Filling.** Profile participates in clearing rounds until
   `maxNotional` is filled, `maxRounds` reached, `expiry` passed,
   or the user cancels.
5. **Completion.** The resulting position is a standard yield-
   perpetual position.

---

## The AAVE Backstop Use Case

The clearest concrete example of how an external protocol can use
the chain is the AAVE backstop. AAVE suppliers earn variable USDC
supply APY. There is no on-chain instrument anywhere that lets a
supplier hedge against the rate falling. An $X-million position
earning a variable rate has unbounded downside exposure to rate
compression; in TradFi the equivalent exposure would be hedged with
SOFR futures or interest-rate swaps.

On the Nunchi blockchain, the supplier creates a single clearing
profile declaring "go SHORT on the LENDING benchmark if it drops
below X basis points, up to $Y notional, max fee Z bps". The
chain's responsibilities:

1. Read Aave V3's supply APY directly from Ethereum mainnet on
   every validator, every round.
2. Aggregate it into the LENDING benchmark via the dual-median
   pipeline.
3. Watch the trigger condition every `ORACLE` phase, atomically as
   part of block production, with no keeper.
4. When the trigger fires, pull the profile into the next clearing
   batch.
5. Run the auction, verify the KKT certificate, settle the batch,
   and emit the resulting position event.

After that, the position behaves like any other on-chain holding.
The chain keeps publishing the benchmark, applying funding every
interval, and exposing position state. From the supplier's
perspective, the entire interaction was a single signature.

The AAVE source-protocol read path is what makes the example
possible at all. Without first-class source connections at the
validator layer, the same product would have to depend on whichever
oracle service was willing to publish AAVE rates with low enough
latency and sufficient cryptoeconomic security — a dependency the
chain explicitly avoids by computing the rate inside consensus.

---

## Cross-Chain Settlement: the HyperEVM Bridge

The settlement contracts can also be deployed on **HyperEVM**
(Hyperliquid's EVM-compatible execution environment) as a
builder-operated market under **HIP-3** (the Hyperliquid Improvement
Proposal that allows builder-operated perpetual markets), with the
benchmark value bridged from the Nunchi blockchain.

The bridge is a publisher process that polls the index precompile
on the Nunchi side and submits the snapshots — including the
consensus finality certificate — to a counterpart oracle precompile
on the target chain. Because the certificate is a single 240-byte
BLS aggregate verifiable in one pairing check, the target chain
can validate the published value without trusting the publisher.

This is the chain's only export-side concern. The matching engine,
liquidity, and order-book primitives on the destination chain are
not part of the Nunchi blockchain. The Nunchi side simply
guarantees that the value the destination chain settles against
was the value finalized on Nunchi at the stated block.

EIP-2537 affects the gas cost of verifying Nunchi finality
certificates from Ethereum mainnet (without it, BLS verification
in pure Solidity is ~2 M gas per pairing). HyperEVM and
mirage-style fork simulators that support custom precompiles
sidestep this constraint.

---

## What Runs On-Chain vs. Off-Chain (DeFi)

A compact summary of the DeFi-stack division of labor.

| Concern | On-chain (this folder) | Off-chain |
|---|---|---|
| Source-protocol rate / price ingestion | Validator-level reads + median aggregation | — |
| Benchmark publication | Precompile at `0xA0_` | — |
| Index methodology design | — | Index designer |
| Profile storage and trigger evaluation | `ClearingProfile` contract + per-block ORACLE phase | — |
| Order book (continuous matching) | Application contract | — |
| Clearing-batch construction | Block builder | — |
| Solver competition | Verification | Solver computation |
| KKT optimality check | `O(n)` chain verification | — |
| Settlement, funding accrual, position state | `ClearingHouse` contract | — |
| Position management (margin top-up, exit logic) | — | Agent runtime |
| Risk policy (per-agent loss caps, sizing) | — | Agent runtime |
| Liquidation triggering and matching | Permissionless on-chain | Liquidation-keeper agents (off-chain) |
| Insurance-fund accrual | Automatic in settlement | — |
| Insight emission per batch | `ClearingInsight` event + on-chain anchor | — |
| Solver bond / slashing | `ClearingHouse` contract | — |

The pattern is consistent: the chain owns ingestion, verification,
settlement, and tamper-evident emission. The chain never owns
quoting, position management, or the agent's own risk policy.
Those layers are explicitly off-chain so they can iterate
independently of consensus and so the chain stays minimal.

---

## Operating the Chain

The runnable artifact is a Nunchi blockchain node. A node is a
single Rust process that combines four things:

- **Simplex BFT consensus** (`threshold_simplex`) — proposes
  blocks, votes, contributes a BLS12-381 signature share toward a
  threshold-finalized block.
- **REVM execution** — applies transactions to local state.
- **QMDB state** — authenticated key-value storage with a Merkle
  root in every block header.
- **Authenticated P2P** — Ed25519-authenticated, encrypted overlay
  between peers.

A node runs in one of three modes selected at startup:

| Mode | What it does |
|---|---|
| `validator` | Full consensus participant: proposes blocks, votes, contributes signature shares. Holds a BLS12-381 DKG share and an Ed25519 P2P identity key. |
| `secondary` | Read-only follower: authenticates to the P2P overlay, replicates all finalized blocks, never votes or proposes. Useful as a non-consensus RPC endpoint. |
| `dkg` | Participates in an interactive Joint-Feldman key-generation ceremony. Used only during initial validator setup or resharing. |

A devnet is the smallest interesting deployment of these processes:
enough nodes to reach the threshold and produce blocks.

---

## Current Devnet Topology

The standard devnet is **4 validators + 1 secondary**, with a
**3-of-4 threshold**. Deliberately minimal-but-useful:

- 4 validators is the smallest set that exercises BFT correctly.
  With threshold 3, one validator can be offline or malicious
  without halting finality. Lose two and the network stalls but
  does not produce incorrect results.
- 1 secondary peer is enough to demonstrate the read-only follower
  path, which is the integration point for any downstream system
  that wants to verify blocks locally without participating in
  consensus.

A single instance of the devnet runs entirely on one host. The
Docker layout gives each node its own container; the native layout
gives each node its own process. Both produce the same chain.

### Current measured state

| Property | Current measured value |
|---|---|
| Validators | 4 |
| Secondary peers | 1 |
| Threshold | 3 of 4 |
| Block time (observed) | ~400 ms |
| Finality | Single-slot |
| Chain ID (default devnet) | 1337 |
| Gas limit per block | 30,000,000 |
| Transaction format | EIP-1559 |
| Solidity contracts deployed | 10 (production-intent) |
| RPC namespaces exposed | `eth_*`, `net_*`, `web3_*`, `kora_*` |

The headline product target — sub-50 ms blocks via co-located
Tokyo-region validators — is a stated objective, not a measured
devnet number. The current ~400 ms figure reflects single-host
devnet operation with default Simplex round timing.

---

## The Three Binaries

| Binary | Purpose |
|---|---|
| `kora` | The node binary. Runs in `validator`, `secondary`, or `dkg` mode. |
| `keygen` | Key-generation utility. Runs the DKG ceremony, generates Ed25519 P2P keys, and writes `peers.json` and `genesis.json`. |
| `loadgen` | EIP-1559 transaction load generator for throughput testing. |

The toolchain is Rust nightly because some BLS12-381 SIMD
optimizations require nightly-only features.

---

## Bringing Up a Devnet

Two paths are supported. Both produce the same network — the only
difference is process isolation and how the DKG ceremony is run.

### Path A: Docker Compose

Recommended for CI, staging, and any environment where
reproducibility across hosts matters.

Two top-level recipes:

- **Trusted-dealer DKG.** A single process generates the full
  secret, splits it into 4 shares, distributes them, and exits.
  Sub-second; acceptable for any environment where the host
  running setup is itself trusted.
- **Interactive Joint-Feldman DKG.** No party ever holds the full
  secret; each validator independently generates a polynomial,
  exchanges commitments and encrypted shares with the others, and
  ends the ceremony with its own share of a collective secret.
  10–30 seconds.

Both recipes run a fixed three-stage sequence:

1. **Init-config** runs `keygen setup --validators 4
   --secondary-peers 1 --threshold 3 --chain-id 1337`. Produces
   per-node Ed25519 P2P keys, `peers.json`, and `genesis.json`.
2. **Init-DKG** performs the chosen DKG ceremony. Output: one
   BLS12-381 share per validator and the shared 48-byte group
   public key.
3. **4 validator containers + 1 secondary container** start as
   long-running services. Each validator loads its share and joins
   consensus; the secondary connects to the P2P overlay and
   replicates blocks.

Lifecycle helpers in the workspace recipe runner stop containers
without losing state, reset and rerun DKG, tail logs from all
containers, print a live `kora_nodeStatus` dashboard across
validators, and print a container-status summary.

### Path B: Native Cargo

Recommended for active development of the chain itself, where
edit-compile-test cycles dominate. You manage each node process
directly.

Three steps:

1. **Generate keys and genesis.**

   ```
   keygen setup --validators 4 --secondary-peers 1 \
     --threshold 3 --chain-id 1337 --output-dir <devnet-dir>
   ```

   Writes one directory per node (each with that node's Ed25519
   P2P key), `peers.json` (the bootstrap list), and
   `genesis.json`.

2. **Run the DKG ceremony.**

   ```
   keygen dkg-deal --validators 4 --threshold 3 --output-dir <devnet-dir>
   ```

   Trusted-dealer mode. The interactive variant is `keygen
   dkg-interactive` (requires the validators to be running and
   reachable on P2P).

3. **Start the nodes.** One process per validator plus one for the
   secondary:

   ```
   kora validator --data-dir <devnet-dir>/node0 --peers <devnet-dir>/peers.json --chain-id 1337
   ...
   kora secondary --data-dir <devnet-dir>/secondary0 --peers <devnet-dir>/peers.json --chain-id 1337
   ```

   Once at least 3 validators are running, blocks begin to
   finalize.

---

## Endpoint Table

Default ports for the Docker layout (chosen to avoid collision
with Anvil on 8545):

| Service | Address |
|---|---|
| validator-node0 JSON-RPC | `http://localhost:8550` |
| validator-node1 JSON-RPC | `http://localhost:8551` |
| validator-node2 JSON-RPC | `http://localhost:8552` |
| validator-node3 JSON-RPC | `http://localhost:8553` |
| secondary-node0 JSON-RPC | `http://localhost:8554` |
| validator P2P (node0–3) | `localhost:30400`–`localhost:30403` |
| secondary P2P | `localhost:30500` |
| Per-validator Prometheus metrics | `localhost:9000`–`localhost:9003` |
| Prometheus UI (with observability profile) | `http://localhost:9090` |
| Grafana (with observability profile) | `http://localhost:3000` |

For a native devnet, ports are assigned through each node's
`config.toml`. The conventional choice is the standard Ethereum
8545.

---

## RPC Surface

Standard `eth_*`, `net_*`, and `web3_*` are covered in
`01-foundations.md`. The chain-specific `kora_*` namespace ships
`kora_nodeStatus` today and queues additional methods (explicit
VRF-seed retrieval, recent-blocks summary, finality-certificate
export, consensus health, active-agent enumeration) tracked in
`05-roadmap.md`.

Field meanings of `kora_nodeStatus`:

- `currentView` — the consensus view number, increments every
  Simplex round.
- `finalizedCount` — total finalized blocks. Incrementing means the
  network is live.
- `nullifiedCount` — number of views that ended without finalizing
  a block. Should be near zero in a healthy 4-validator devnet.
- `peerCount` — number of authenticated P2P connections.
- `isLeader` — whether this node is the proposer for the current
  view.

### What is not yet wired

- WebSocket subscriptions (`eth_subscribe`) are not implemented.
  Clients must poll. The underlying RPC library natively supports
  subscriptions; the wiring is a queued change.
- `eth_getProof` is not implemented. The QMDB transition-hash
  state-root model does not directly support Ethereum-style
  inclusion proofs. Mitigated by the QMDB-proofs precompile at
  `0x0B`.

---

## Configuration

### Genesis (`genesis.json`)

Generated by `keygen setup`. Defines the initial state of the
chain.

```json
{
  "chain_id": 1337,
  "timestamp": 1714000000,
  "allocations": [
    {"address": "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266", "balance": "1000000000000000000000000"},
    {"address": "0x70997970C51812dc3A010C7d01b50e0d17dc79C8", "balance": "1000000000000000000000000"}
  ]
}
```

By default, allocations include the standard Foundry / Hardhat
development addresses, so any tutorial keypair has gas out of the
box. After the DKG ceremony, the genesis must not be modified —
every validator must agree on the same starting state.

### Peers (`peers.json`)

Authoritative bootstrap list of every node authorized to
participate in the P2P overlay. Each entry has an Ed25519 hex
pubkey, a P2P address, and an RPC address. The P2P layer rejects
any incoming connection whose handshake is not signed by one of
these keys. This is independent of the BLS12-381 consensus key —
Ed25519 is for P2P identity, BLS12-381 is for consensus signatures.

### Per-node `config.toml`

Each node reads a `config.toml` from its data directory. The
relevant sections:

```toml
chain_id = 1337
data_dir = "/var/lib/kora"

[consensus]
validator_key = "/data/validator.key"
threshold = 3

[network]
listen_addr = "0.0.0.0:30303"
bootstrap_peers = ["<hex-pubkey>@host1:30303", "<hex-pubkey>@host2:30303"]

[execution]
gas_limit = 30_000_000
block_time = 2

[rpc]
http_addr = "0.0.0.0:8545"
ws_addr   = "0.0.0.0:8546"

[metrics]
addr = "0.0.0.0:9000"
```

`block_time` is a hint to the consensus loop; the actual cadence
is dictated by Simplex round timing (~400 ms in the current
devnet).

---

## Observability

Each validator exposes a Prometheus-compatible metrics endpoint.
The metrics cover:

- Block finalization rate.
- Consensus view progression (current view, finalized view, gap).
- Nullification counts (views without a finalized block).
- Peer connectivity.
- REVM execution timings.
- QMDB read/write latencies and root computation.
- RPC request volume by method.

Enabling the observability Compose profile starts a Prometheus
container that scrapes the validator endpoints every 15 seconds
and a Grafana container with pre-built dashboards. The Grafana
default login should be changed before exposing the instance.

For lightweight probing without Prometheus, `kora_nodeStatus` is a
single RPC call that returns the most-watched fields. The
companion live-dashboard recipe polls this on every validator and
prints a terminal table.

### Verifying a network is live

The fastest single check after bring-up:

```
curl -s -X POST http://localhost:8550 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"kora_nodeStatus","params":[],"id":1}'
```

A healthy response:

```json
{
  "currentView": 42,
  "finalizedCount": 41,
  "nullifiedCount": 0,
  "peerCount": 3,
  "isLeader": false
}
```

If `finalizedCount` increments between calls, the network is live
and producing blocks. If `currentView` increments but
`finalizedCount` does not, the network is failing to reach the
threshold (likely fewer than 3 validators reachable). If neither
increments, the consensus loop is not running on this node.

---

## In-Process E2E Test Harness

The repository ships an in-process end-to-end harness that runs a
complete multi-validator consensus network inside a single Rust
process — no Docker, no real sockets, no separate keygen step. It
is built on two Commonware substitutions:

- The **deterministic runtime** replaces tokio with a
  single-threaded simulator where time advances only when the test
  asks it to and every operation is reproducible from a seed.
- The **simulated P2P** replaces real TCP with an in-process
  message bus. Latency, packet loss, and reordering are
  configurable for fault injection.

The same consensus and execution code that runs in production runs
under the harness; only the I/O substrate is swapped. A typical
full cycle (spin up network, deploy contract, submit transactions,
read state, tear down) takes well under 30 seconds and is fully
deterministic from its seed.

This is the intended substrate for any external system that wants
to validate chain interactions without committing to a long-running
devnet — the harness can be imported as a Rust dev-dependency and
driven from external test code.

---

## Load Testing

The `loadgen` binary generates EIP-1559 transactions, signs them
with pre-funded test accounts, and submits them at a target
concurrency. Two preset recipes ship: a small load test (1,000
transactions submitted to node 0) and a stress test (10,000
transactions from 50 funded accounts at higher concurrency).
Custom invocation:

```
loadgen \
  --rpc-url http://127.0.0.1:8550 \
  --total-txs 5000 \
  --accounts 20 \
  --concurrency 50 \
  --chain-id 1337
```

Spreading across multiple sender accounts avoids per-sender nonce
contention (every Ethereum account has a strictly increasing nonce;
a single sender forces strict ordering). The reported metrics are
TPS, median and p99 finalization latency, and failure count.

---

## DKG Ceremony Modes (Operator Recap)

DKG produces two artifacts: one BLS12-381 share per validator
(held privately by that validator) and one shared 48-byte group
public key (used by anyone verifying a finality signature). After
DKG, no individual validator knows the full private key — only any
3 of 4 combining their shares can produce a valid threshold
signature.

| Mode | Speed | Trust requirement | Use case |
|---|---|---|---|
| Trusted dealer | Sub-second | The dealer process briefly holds the full secret in memory before splitting and exiting. The host running setup must be trusted. | Local development, CI, programmatic bootstrap from a trusted controller. |
| Interactive Joint-Feldman | 10–30 seconds; requires all validators running and reachable | No single party ever sees the full secret. | Production-like staging, or any environment where the operator's host should not be a trust anchor. |

Persisted DKG output survives node restarts; you only re-run DKG
if the validator set changes or you wipe data deliberately.

---

## Coexistence: Where the Chain Ends

The chain shares territory with two systems: the **agent runtime**
(autonomous agents that consume the chain) and **EVM fork
simulators** (in-process clones of public chain state used for
testing). Both have their own documentation sets; this section
draws the seam.

### Two-sentence summary

The Nunchi blockchain owns **identity, finalized state, settlement,
and tamper-evident anchoring**. The agent runtime owns **agent
execution, model selection, prompt assembly, and local learning**.
Fork simulators own **counterfactual simulation against external
chain state**.

### What the chain replaces

| Replaced thing | Why a single machine does not need it | Why a multi-party deployment does |
|---|---|---|
| Cross-machine knowledge sharing | Local agents see one another's files. | Agents on different operators' machines have no shared memory; the chain's `InsightBoard` is the only neutral substrate. |
| Tamper-evident task records | Trusting the local filesystem suffices. | Anyone with file access can rewrite a JSONL file. The chain's witness anchoring binds an episode hash to a finality-signed block. |
| Verifiable randomness | A local PRNG is fine for a process you trust. | Outside parties cannot verify a model-selection or auction outcome unless the seed itself is verifiable. The block-level VRF is that seed. |
| Permissionless agent identity | A local registry is just a config file. | Identity that any party can verify without contacting an authority — the role of ERC-8004. |
| Cross-organization settlement | Internal accounting suffices. | A contract that any party can read, compute against, and dispute on identical terms — the role of the clearing-house contracts. |

### What the chain complements

**The agent runtime.** The agent runtime is the primary client of
the chain. Its job is to dispatch LLM-powered agents through a
verify-execute-learn loop, persist what worked, and route future
work to the best-fitting model. The runtime keeps everything
locally; high-confidence subsets are promoted to the chain so
other operators' fleets can read them. The chain's role from the
runtime's perspective: a shared finality-signed knowledge ledger,
a verifiable substrate for randomness, an anchor for episode
hashes, and a registry for agent identity that survives process
restarts and is visible across fleets.

The runtime does not stop functioning without the chain — most of
its loop runs against local files. The chain is what turns local
agent learning into a substrate that compounds across the
network.

**EVM fork simulators.** In-process REVM instances that lazily
fetch storage slots from a public source chain (Ethereum mainnet,
Arbitrum, Base) and let the caller execute transactions against
that cloned state. They are simulators, not chains: no consensus,
no validators, no P2P network, no permanence.

| Concern | Fork simulator | Nunchi blockchain |
|---|---|---|
| Counterfactual against mainnet state | Yes — its sole job. | No — has its own independent state. |
| Cryptographic finality | No. | Yes — single-slot Simplex BFT. |
| Cross-party visibility | No (in-process). | Yes (every node sees every block). |
| Persistence across restarts | No. | Yes (QMDB). |
| Verifiable randomness | No. | Yes (block-level VRF). |
| Tamper-evident records | No. | Yes (state root + finality signature). |

The fork simulator cannot replace the chain because nothing in it
is trustworthy to a third party — a single operator controls
everything. The chain cannot replace the fork simulator because it
has no mechanism to load external mainnet state. They serve
different questions ("what *would* happen" vs. "what *did* happen,
and who agrees?") and are expected to coexist indefinitely.

The canonical pattern: **simulate on a fork, commit on the
chain.**

### On-chain vs. off-chain decision rules

Five rules of thumb:

1. **Cross-party agreement → on-chain.** If two or more independent
   parties have to agree about whether something happened, it goes
   on the chain.
2. **Single-party state with no audit need → off-chain.** If only
   one operator cares about the state and no one else will ever
   audit it, it stays off-chain. Putting these on-chain costs gas
   with no benefit.
3. **Cryptographic finality required → on-chain.** If the question
   is "can you prove this happened, to anyone, at any later time?",
   the chain's finality signature is the only mechanism that
   delivers the answer.
4. **Heavy local computation that no one else needs to verify →
   off-chain.** LLM inference, gate pipelines, prompt assembly are
   all expensive in compute and small in agreement value. The
   chain only sees the artifact.
5. **External-state simulation → fork simulator.** If the question
   requires the live state of a public chain, only a fork
   simulator can answer it. The chain has no mechanism to clone
   external state.

### Why the chain does not try to be a general-purpose L1

The chain is deliberately narrow. Every component (the
precompiles, the agent registry, the oracle pipeline, the
clearing engine) exists because some product in the agentchain
stack needs it. Components that would be standard on a
general-purpose L1 — programmable token standards, complex
governance modules, broad ecosystem subsidies — are intentionally
absent.

The trade-off is that the chain stays small (and therefore
auditable, fast to finalize, and easy to operate as a 4-validator
devnet) at the cost of not being the right substrate for arbitrary
applications. The corollary: anything not on the agentchain
critical path should not be on this chain.

---

## Summary

The chain provides:

- **Direct source-protocol reads from each validator**, with no
  shared aggregator.
- **A consensus-aggregated-median benchmark** published every few
  seconds via a fixed-gas precompile.
- **Native agent identity and capability checks** at the precompile
  layer.
- **Per-block phase ordering** that makes liquidations safe against
  oracle staleness.
- **Atomic, KKT-verified batch settlement** with on-chain insight
  emission.
- **Block-level VRF, BTLE, and historical state proofs** as
  building blocks for solver rotation, sealed intents, and dispute
  resolution.
- **Compact finality certificates** that let other chains settle
  against the benchmark without trusting a bridge operator.

A single-host devnet is one recipe away. It produces blocks at
~400 ms cadence with single-slot finality, exposes the standard
Ethereum RPC surface plus the `kora_*` extension, and ships with
deterministic in-process testing, load generation, and Prometheus
+ Grafana observability. The process-level layout (4 validators +
1 secondary, 3-of-4 threshold) is small enough to run on a
developer laptop and faithful enough to represent the shape of a
production deployment.

Everything beyond that — quoting, market making, position
management, agent risk policy — is intentionally application-level
and lives outside this folder.
