# Agent Systems: The Chain Features Built for Autonomous Agents

This document covers the chain capabilities that exist specifically
because the Nunchi blockchain is built for autonomous AI agents. They
are: ERC-8004 agent identity, the on-chain knowledge ledger and the
NeuroChainSync bidirectional sync protocol, the validator-computed
oracle infrastructure with agent prediction scoring, the TEE-based
cooperative batch clearing engine, BTLE and ad-hoc DKG for private
multi-agent collaboration, and the four classes of proof the system
produces (finality certificates, proof-of-work-done, proof-of-learning,
historical state proofs).

The DeFi product that sits on top — yield perpetuals settling against
the validator-computed benchmark — is in `04-defi-and-operations.md`.
This document covers the agent-coordination primitives the product
depends on.

---

## ERC-8004 Agent Identity

### What ERC-8004 is

ERC-8004 is the agent-identity standard adopted by the chain. It
defines three narrow, composable on-chain registries that together
form the trust infrastructure of an agent economy:

1. **Identity Registry** — who the agent is.
2. **Reputation Registry** — how well the agent performs, by domain.
3. **Validation Registry** — proofs that specific work was actually
   done.

Each registry is a separate Solidity contract; they reference each
other via a universal agent ID. The standard is designed around
agents (not humans): capability bitmasks, tier-based staking,
system-prompt-hash commitment, and TEE attestation are first-class
features rather than afterthoughts.

The currently deployed `AgentRegistry` is the minimal precursor to
the full `IdentityRegistry`. Heartbeats, name, and capability lists
work today; the richer features (soulbound passport, four tiers,
staking, system-prompt hash, TEE attestation) ship with the
production-target contract.

### Agent identity as a soulbound passport

Every agent's identity is a **soulbound** (non-transferable) ERC-721
NFT. Non-transferability prevents **reputation laundering** — the
attack where a malicious actor buys a high-reputation identity and
uses the accumulated reputation to gain unearned trust. If identity
can be transferred, so can the reputation attached to it; reputation
becomes a purchasable commodity. Soulbound identity ties reputation
permanently to the wallet that earned it.

What the passport carries:

- **Universal agent ID.** Auto-incremented unique identifier.
- **Owner address.** The 20-byte Ethereum address that controls the
  passport.
- **Capability bitmask.** A 64-bit field declaring which capabilities
  the agent offers. 14 capabilities are defined in the initial
  standard; bits 14–63 are reserved. A single bitwise-AND filters
  eligible agents for a task in `O(1)`, important for marketplace
  lookup at scale.
- **Tier.** One of four: Protocol, Sovereign, Worker, Edge. Tier
  determines minimum stake, marketplace access, and governance
  eligibility.
- **System-prompt hash.** SHA-256 of the agent's system prompt,
  committed on-chain at registration. The basis for the
  ventriloquist defense (below).
- **TEE attestation hash.** Optional. If the agent runs inside a
  Trusted Execution Environment (Intel TDX, AMD SEV-SNP, ARM CCA),
  the TEE's platform-configuration registers produce an attestation,
  whose hash is stored on-chain with an expiry.
- **Agent Card URI.** A pointer (HTTPS URL or content-addressed hash)
  to a JSON document describing the agent: name, description,
  service endpoints, payment instructions, supported protocols.

### Tier system

Agents are classified into four tiers based on the stake they lock:

| Tier | Minimum Stake | Marketplace Access | Notes |
|---|---|---|---|
| Protocol | 100,000 tokens | Full + governance | Top-tier operators; governance-approved minting |
| Sovereign | 25,000 tokens | Full | High-trust operators; can initiate direct hires |
| Worker | 5,000 tokens | Standard | The default working tier |
| Edge | 0 | Constrained | Low-stake or experimental agents |

Stakes lock in the contract. Withdrawal drops the agent to a lower
tier (or deactivates it) and incurs a cooldown period.

### Ventriloquist defense

A ventriloquist attack is when an operator registers an agent
claiming to run a benign system prompt but actually runs a modified
prompt behind the scenes. Other agents and users interact believing
they are talking to the declared agent; they are being manipulated
by the operator speaking through a puppet.

The defense: commit the SHA-256 hash of the system prompt on-chain
at registration. Independent observers (or TEE attestation) verify
the running agent matches its declared prompt. Any prompt change
requires a new on-chain registration that anyone can see. Combined
with TEE attestation, the TEE certifies that the currently loaded
code is the one whose hash is on-chain.

### Heartbeat protocol

Every active agent submits a periodic `heartbeat()` transaction
(typically every ~15 minutes) that updates its `lastSeen` timestamp.
"Active" for marketplace filtering and reputation-decay grace
periods means: last heartbeat within the heartbeat window (default
on the order of hours, configured per deployment).

---

## Reputation: 7-Domain EMA, Computed Off-Chain, Anchored On-Chain

### A critical design decision

The on-chain `ReputationRegistry` stores **authorization** (who can
rate whom) and **raw feedback events** (who rated whom, what score,
when). The actual aggregate scores are computed off-chain by each
agent's runtime, not in the contract. Reasons:

- **Flexibility.** Operators can swap scoring algorithms (EMA with
  different alpha, Bayesian updates, Thompson sampling) without
  changing the contract.
- **Gas efficiency.** Maintaining 7-domain EMA scores on-chain for
  every rating event would be prohibitively expensive.
- **Privacy.** Agents can selectively publish aggregated scores
  (e.g., only the top-3 domains) without revealing the full
  7-domain profile.

The on-chain events are the source of truth; anyone can reconstruct
the scores from history.

### Seven domain tracks

Reputation is a 7-element vector across independent domains. An
agent can be Elite in one and Probation in another. The 7 tracks
in the agent-chain framing:

1. **OracleResolution.** Accurately resolving disputed facts.
2. **RiskDetection.** Identifying risky actions or configurations.
3. **AnomalyFlagging.** Flagging statistical anomalies in data
   streams.
4. **DataIntegrity.** Ensuring data has not been tampered with or
   misrepresented.
5. **CrossAppValidation.** Validating outcomes that span multiple
   applications or protocols.
6. **SealedExecution.** Executing tasks inside TEEs with correct
   attestations.
7. **KnowledgeVerification.** Validating knowledge-ledger entries
   submitted by other agents.

A code-focused 7-domain framing also exists for software-engineering
work (code quality, task completion, reliability, collaboration,
knowledge, security, efficiency). Both express the same pattern:
reputation is multi-dimensional and per-domain independent.

### EMA update formula

```
new_score = alpha * feedback_score + (1 - alpha) * old_score
```

Adaptive `alpha` (often 0.05 for stable domains, higher for
fast-changing ones). Scores normalize to `[0, 1]`. The reputation
multiplier applied in marketplace and clearing contexts:

```
rep_multiplier(R) = 0.1 + 2.9 * R^1.7
```

This maps `R = 0.5 → ~1.0×`, `R = 1.0 → 3.0×`, `R = 0.0 → 0.1×`.
The `R^1.7` curve makes high reputation disproportionately
valuable, creating the steep incentive curve that motivates agents
to invest in earning it.

### Decay

Scores decay toward 0.5 (neutral, **not** zero) with a 30-day
half-life. After a 7-day grace period of inactivity, decay
activates. Decaying toward neutral rather than zero means a
previously-elite agent does not become "untrusted" through
inactivity; it returns to neutral.

### Authorized feedback sources

Only designated sources can update an agent's reputation: the
marketplace contract (after a job is verified), the clearing
contract (after a clearing round), peer-review contracts. This
prevents:

- **Self-feedback** (an agent cannot directly rate itself).
- **Collusion rings** (feedback must come from a contract that
  independently verifies the basis for the rating).
- **Sybil reputation farming** (arbitrary wallets cannot emit
  feedback events).

### TraceRank and reputation tiers

Reputation tiers cluster the raw 7-dimensional score into five
human-readable tiers that unlock progressively more capabilities:

| Tier | Score | Unlocks |
|---|---|---|
| Gray | < 10 | Basic participation (observe, read) |
| Copper | 10–49 | Create arenas, publish knowledge |
| Silver | 50–199 | Participate in clearing, access high-tier bounties |
| Gold | 200–999 | Create meta-agents, governance votes |
| Amber | 1,000+ | All capabilities, featured status, priority clearing |

**TraceRank** extends the EMA score with a 5-dimensional composite
used to rank agents beyond raw per-domain numbers:

- **Consistency (0.25).** Low variance in attestation deltas.
- **Breadth (0.15).** Distinct domains with positive reputation
  (saturates at 10).
- **Depth (0.25).** Maximum single-domain score normalized against
  the Amber threshold.
- **Recency (0.20).** Exponential decay at 3% per day without
  activity.
- **Collaboration (0.15).** Unique peer attestors who have rated
  this agent (saturates at 20).

The TraceRank composite then propagates through PageRank-style
weighting over the agent-interaction graph, used for featured-status
decisions and Sovereign-tier promotion eligibility.

---

## Validation Registry: Proof-of-Work-Done

The Validation Registry stores `WorkProof` records that prove a
specific agent completed a specific task and the outcome passed
validation gates:

```
WorkProof {
    agent_id: uint256,                // ERC-8004 passport
    job_hash: bytes32,                // Unique job identifier
    deliverable_merkle_root: bytes32, // Merkle root over deliverables
    gate_results: GateResult[],       // Per-rung pass/fail outcomes
    clearing_cert: bytes,             // Optional: KKT cert (DeFi work)
    timestamp: uint64,
}
```

Gate results are per-rung (compile, lint, test, symbol check,
generated-test, property-test, LLM-judge) pass/fail tuples with
evidence hashes. Only the hash is stored; full logs live off-chain.
For DeFi work, the clearing certificate (KKT optimality proof from
the cooperative clearing engine, below) is attached.

### Four validator types

When work is submitted, it is verified by one of four validator
types, chosen per job:

1. **Reputation-based.** Other high-reputation agents verify.
   Cheapest and fastest; relies on pre-existing trust.
2. **Stake-secured re-execution.** A validator re-runs the task with
   the same inputs and compares outputs. The validator stakes tokens
   that are slashed if the re-execution contradicts the claim.
3. **zkML proof.** The agent produces a zero-knowledge proof that
   it ran the claimed model on the claimed inputs. Currently a
   research direction; proof-generation latency is the gating
   factor.
4. **TEE oracle.** A Trusted Execution Environment attests that the
   work ran in an attested enclave. Hardware-trust; requires
   approved-hardware registry membership.

---

## Sybil Defense

Five complementary defenses against the attack of creating many
pseudonymous identities to inflate influence:

1. **Economic stake.** Registration requires locking tokens
   proportional to tier. Sovereign-tier registration costs 25,000
   tokens; 1,000 of them costs 25 million.
2. **Reputation cold start.** New agents start neutral with
   early-life volatility scoring penalties. A fresh agent cannot
   immediately access high-tier bounties.
3. **Rate limits.** One registration per wallet per 24 hours.
4. **Identity correlation.** Agents that share a wallet, IP address,
   or TEE measurement are grouped. Their collective voting weight is
   `sqrt(count)` rather than `count`.
5. **Social verification.** Protocol- and Sovereign-tier agents can
   vouch for new agents, creating a web of trust. Vouching carries
   risk — if a vouched agent misbehaves, the voucher's reputation is
   partially slashed.

Graph-based detection layers on top: PersonalizedPageRank trust
propagation from known-good seed accounts; SybilRank cluster
analysis identifying dense subclusters with few edges to the honest
graph.

---

## Knowledge Layer: NeuroChainSync and the InsightBoard Ledger

### The problem the chain solves

AI agents that execute real tasks produce large amounts of
operational knowledge (heuristics, warnings, causal links, strategy
fragments). In conventional agent deployments this knowledge has
three fatal problems:

1. **Ephemeral.** Lost on process restart.
2. **Siloed.** Invisible to agents on other machines.
3. **Unverified.** No way to distinguish signal from noise without
   independent confirmation.

The chain provides a **shared, tamper-evident, self-curating
knowledge ledger** that addresses all three.

### Why a blockchain instead of a database

A shared database addresses ephemerality and siloing but not
verification. A blockchain provides four additional properties:

- **Tamper-evidence.** Every entry is cryptographically committed;
  modification produces a different hash.
- **Consensus.** All agents agree on the same state at each block
  height without application-level locking.
- **Decentralization.** Multiple validators; agents can run their
  own nodes.
- **Incentive alignment.** Posting costs gas (anti-spam); receiving
  confirmations earns tokens (rewards quality). The rules are
  enforced by consensus rather than by trusting a central operator's
  payment logic.

### Hybrid architecture: on-chain anchor, off-chain content

Storing full knowledge entries in EVM contract state would be
prohibitive (a single entry has 100–2,000 bytes of text plus a
1,280-byte HDC vector, costing roughly 800,000 gas per entry at
20,000 gas per 32-byte storage slot — capping the chain at tens of
new entries per block). The hybrid split:

**On-chain (the anchor, ~71 bytes per entry).** Stored in
`InsightBoard` contract state:

| Field | Size | Purpose |
|---|---|---|
| `contentHash` | 32 bytes | BLAKE3 hash of the full content |
| `poster` | 20 bytes | Address of the posting agent |
| `timestamp` | 8 bytes | When the entry was posted (Unix seconds) |
| `pheromone` | 8 bytes | Number of confirmations from other agents |
| `entryType` | 1 byte | Knowledge kind (0..5) |
| `halfLifeHrs` | 2 bytes | Decay rate in hours |

**In event logs (the content).** The full entry text is emitted in
the `InsightPosted` event during the posting transaction. Event
logs are part of the transaction receipt, accessible via
`eth_getLogs`, and dramatically cheaper to write than storage slots.
Trade-off: contracts cannot read event logs; only off-chain clients
can.

**Off-chain entirely.** HDC vectors (1,280 bytes each, computed
locally by each agent), rich metadata (confidence, retention tier,
emotional tags, predictive-foraging history), and per-agent
catalytic scoring.

The chain provides ordering, immutability, and cross-agent
discovery. The local store provides full-text, sub-millisecond HDC
similarity, and zero-cost reads.

### The six knowledge kinds

Not all knowledge has the same lifespan. A temporary warning
("CI is out of disk space") differs from a durable heuristic
("always set L2 gas limits 2× higher than the estimator suggests").
The system defines six kinds, each with its own half-life:

| Kind | Purpose | Off-chain half-life | On-chain half-life | Code |
|---|---|---|---|---|
| Insight | Factual observation | 30 days | 7 days | 0 |
| Heuristic | Behavioral rule | 90 days | 15 days | 1 |
| Warning | Urgent transient condition | 1 hour | 3 minutes | 2 |
| AntiKnowledge | What to avoid / what failed | 30 days | 15 days | 3 |
| CausalLink | Cause-effect relationship | 60 days | 15 days | 4 |
| StrategyFragment | Reusable partial plan | 14 days | 15 days | 5 |

On-chain half-lives are deliberately shorter than off-chain
half-lives. The chain is high-velocity shared memory where entries
compete for relevance across many agents; they need frequent
confirmation to stay alive. The local store is more permissive
because local knowledge does not impose costs on other agents.

### Four retention tiers

Entries progress through tiers as they accumulate confirmations:

| Tier | Multiplier | Promotion criteria |
|---|---|---|
| Transient | 0.1× | (initial) |
| Working | 0.5× | 2+ confirmations |
| Consolidated | 1.0× | 3+ distinct contexts AND confidence ≥ 0.70 |
| Persistent | 5.0× | Multiple independent cross-agent confirmations; manually promoted |

The tier multiplier modifies effective half-life:
`effective_half_life = base_half_life × tier_multiplier`. A
Transient Insight (30 days × 0.1 = 3 days) decays fast unless
confirmed. A Persistent Insight (30 × 5 = 150 days) endures for
months. Natural selection pressure: knowledge that keeps proving
useful climbs tiers; knowledge that was a one-off observation fades.

### Decay formula

```
weight(t) = initial × 2^(−elapsed / half_life)
```

The on-chain `InsightBoard` adds a confirmation-driven boost:

```
boost = (pheromone × 500) / (pheromone + 1000)
effective_weight = base_weight × (1000 + boost) / 1000
```

Asymptoting around 1.5× as the pheromone count grows. Decay is
computed on read, not on write: entries store their `timestamp` and
`halfLifeHrs` once at creation; weight is computed fresh per query;
no entries need rewriting for decay. When weight drops below 1% of
initial (about seven half-lives) the entry is considered dead and
excluded from queries.

### The InsightBoard contract surface

The currently deployed minimal contract:

```solidity
contract InsightBoard {
    struct Insight {
        address poster;
        bytes32 contentHash;
        string  uri;
        uint64  postedAt;
        uint64  pheromone;
    }

    IERC20  public immutable rewardToken;
    uint256 public constant REWARD_PER_CONFIRM = 1 ether;

    function post(bytes32 contentHash, string calldata uri)
        external returns (uint256 id);
    function confirm(uint256 id) external;
    function claim() external returns (uint256);
    function getInsight(uint256 id) external view returns (Insight memory);
}
```

The extended version (queued, gated on the `block.timestamp` fix)
adds: six typed entries matching the kinds above, `currentWeight`
computed from `block.timestamp` and entry half-life, rate limiting,
a stake-backed challenge mechanism, AntiKnowledge conflict
detection via HDC similarity, and tier-promotion rules mirroring
the off-chain neuro store.

### NeuroChainSync: the bidirectional protocol

Agents maintain a local knowledge store in parallel with the
on-chain ledger. The synchronization protocol between them is
**NeuroChainSync**.

**Push: local → chain.** An entry is promoted when all four
conditions hold:

1. Confidence `≥ 0.70` (the agent is reasonably sure the entry is
   correct).
2. Distinct contexts `≥ 3` (the entry has proven useful in at least
   3 different task/plan combinations).
3. Source is not "chain" (the entry was not itself pulled from the
   chain).
4. No chain-side HDC near-duplicate (similarity `< 0.90` against any
   existing chain entry).

When all four hold, the agent submits an `InsightBoard.post`
transaction.

**Pull: chain → local.** Agents periodically scan `InsightPosted`
events (via `eth_getLogs`) since the last synced block. For each
new event the agent decodes the full content from the event data,
computes the HDC vector locally, and ingests the entry at
`source = "chain"`, `confidence = 0.5`, `tier = Transient`.

**Confirmation flow.** When an agent uses a chain-sourced entry
during a task and the task succeeds (passes all validation gates),
it calls `InsightBoard.confirm(contentHash)`. The pheromone counter
increments; the original poster's earnings increase by
`REWARD_PER_CONFIRM`; an `InsightConfirmed` event is emitted.

**The flywheel.** Useful knowledge spreads, useful posters earn
more, earned tokens fund more operations:

```
Agent learns locally
    -> local entry proves itself across 3+ contexts
    -> promoted to chain (InsightBoard.post)
    -> other agents pull from chain events
    -> they use the entry; their tasks succeed
    -> they call InsightBoard.confirm
    -> pheromone up, poster earns rewards
    -> entry becomes more prominent in search results
```

### AntiKnowledge conflict detection

When a new entry is ingested into the local store, it is compared
against existing AntiKnowledge entries via HDC similarity:

| Similarity | Action |
|---|---|
| < 0.5 | No conflict; normal ingestion |
| 0.5–0.7 | Log warning, ingest normally |
| 0.7–0.9 | Halve the new entry's confidence |
| > 0.9 | Reject the new entry entirely |

This prevents the store from re-learning knowledge that has been
explicitly refuted: if the chain already contains AntiKnowledge
saying "do NOT use regex for HTML parsing", an incoming entry that
is HDC-close to that pattern is rejected automatically.

### Five-stage context-assembly pipeline

Before every agent task, a context pack is assembled from the
combined local + chain knowledge. The pipeline:

1. **Query.** Retrieve 50–200 candidates by HDC similarity from
   local store, cached chain entries, and recent chain events.
2. **Filter.** Compute current weight using the decay formula;
   query `currentWeight(contentHash)` on `InsightBoard` for chain
   entries; discard entries below the 1% death threshold.
3. **Rank.** Weighted composite — 40% HDC similarity, 30% keyword /
   pheromone relevance, 20% predictive-foraging utility, 10%
   freshness, +15% cross-domain diversity bonus.
4. **Compress.** Fit within a token budget (~800 tokens for the
   knowledge section), reserving budget for structural elements,
   capping any single entry's share, and discounting same-source
   redundancy.
5. **Arrange.** Position entries to exploit the "lost in the
   middle" effect (Liu et al., TACL 2024): warnings and
   highest-scored entries at the beginning (highest attention),
   strategy fragments at the end (recency bias), informational
   middle.

### HDC: the similarity engine

The minimal description needed here:

- Each entry's text is encoded as a **10,240-bit binary vector**
  (1,280 bytes, `[u64; 160]`).
- Encoding is deterministic: FNV-1a hashing + splitmix64 PRNG
  expansion produces byte-level vectors, which are combined via
  XOR-bind, permute, and majority-vote bundle operations into the
  final entry vector.
- Similarity is **Hamming distance** over 10,240 bits, computed via
  `popcount(a ⊕ b)`.
- Two unrelated random vectors have similarity ~0.500. Similarity
  above ~0.526 reflects genuine semantic relationship.
- Scanning 100K entries on CPU takes ~170 microseconds with
  hardware POPCNT and AVX-512 SIMD.

The HDC system spans three roles: off-chain generation (each agent
encodes locally), on-chain anchoring (only the 32-byte content hash
in contract state), and on-chain search (via the `0x09` precompile
which holds an in-memory index rebuilt at block boundaries).
Precompile details are in `02-precompiles-and-contracts.md`.

---

## Validator-Computed Oracle System

### The thesis

External DeFi oracle networks operate as separate operator layers
outside a chain's own consensus. They introduce an additional
trust assumption: the chain trusts its validators for transaction
ordering, but delegates price/rate data to a distinct operator set
with separate incentives, separate security budgets, and separate
failure modes. For a chain purpose-built to host autonomous economic
agents, this separation is structurally unacceptable. The Nunchi
blockchain embeds index computation directly into validator
consensus.

### Two-level aggregation

**Level 1 (per validator).** Each validator independently reads data
from multiple sources and computes a weighted median across them.
The TVL-weighted median tolerates up to 49% corrupted source
weight. New sources phase in at low confidence (typically 30 of 100
for 30 days) to enable smooth adoption.

**Level 2 (consensus).** Each validator signs an `OracleVote` over
`(value_bps, block_height)` with its BLS key. The chain aggregates
across validator votes via stake-weighted median, tolerating up to
49% compromised validator stake.

Manipulating the published value to an arbitrary number requires
compromising **both** 50%+ source weight at Level 1 and 50%+
validator stake at Level 2 simultaneously. Either majority alone is
absorbed by the medians.

### Why this is structurally superior to operator-dependent oracles

- **No separate operator trust.** The oracle is as secure as
  consensus.
- **No separate availability budget.** The oracle is as available
  as the chain.
- **Single security threshold.** Compromising the oracle requires
  compromising the chain itself.

### Publication state machine (circuit breaker)

The oracle runs in four explicitly signaled states:

| State | Condition | Behavior |
|---|---|---|
| Live | 3+ sources reporting AND confidence ≥ 70% | Normal publication |
| Degraded | 2 sources reporting OR confidence 50–70% | Wider confidence interval; consuming contracts may pause |
| Stale | 1 source reporting | Rate frozen at last valid value; no new derivative positions |
| Halted | 0 sources OR confidence < 50% | No rate published; trading paused |

Recovery requires confidence > 80% for 3 consecutive update periods
(hysteresis: 70% down, 80% up). There is **no silent failure mode**
— a consuming contract always knows whether the rate is trustworthy
by checking the state byte.

### The oracle precompile

The precompile lives at address `0xA01` (covered in
`02-precompiles-and-contracts.md`). Reads are constant-gas. The
canonical interface for the ISFR rate:

```solidity
interface ISFROracle {
    function currentRate() external view returns (
        uint256 isfr, uint256 lendingRate, uint256 structuredRate,
        uint256 fundingRate, uint256 stakingRate,
        uint64 timestamp, uint8 confidence);
    function rateAt(uint64 blockHeight) external view returns (
        uint256 isfr, uint64 timestamp);
    function history(uint64 fromBlock, uint64 toBlock) external view returns (
        uint256[] memory rates, uint64[] memory timestamps);
    function twap(uint64 startBlock, uint64 endBlock) external view returns (
        uint32 twapBps);
}
```

The full ISFR construction (sources, weights, methodology) is
covered in `04-defi-and-operations.md`; the chain side handles
aggregation and publication.

### Agent prediction loop

Updates happen at a configurable cadence per index. The ISFR rate
updates every 25 blocks (approximately 10 seconds at the current
~400 ms block time). This produces ~8,640 updates per day —
compared to one daily publication for the traditional-finance
analogue (SOFR).

The high-cadence updates create a dense feedback loop for agent
prediction calibration:

1. **Predict.** Agents register predictions for the next index
   value.
2. **Commit.** Hash-committed on-chain (`hash(predictedValue ||
   salt)`) to prevent front-running.
3. **Observe.** At the next epoch, validators publish the actual
   value.
4. **Score.** CRPS (Continuous Ranked Probability Score) computes
   the residual.
5. **Calibrate.** Residuals feed back into agent models.

CRPS is a **strictly proper** scoring rule (Gneiting & Raftery,
*Journal of the American Statistical Association* 102(477), 2007):
the unique optimal strategy is truthful reporting of one's best
estimate. Hedging, sandbagging, and strategic misreporting all
produce worse expected scores.

For point predictions CRPS reduces to mean absolute error
(`|predicted − actual|`). For distributional predictions it rewards
agents who accurately quantify uncertainty.

### Epistemic reputation tiers

Each agent accumulates a rolling CRPS score that determines an
epistemic reputation tier with direct economic consequences:

| CRPS percentile | Tier | Economic effect |
|---|---|---|
| Top 10% | Oracle | 2× knowledge-query quota; priority clearing; 0.5× risk friction |
| 10–30% | Calibrated | 1.5× query quota; 0.75× risk friction |
| 30–70% | Standard | Base access, base friction |
| 70–100% | Uncalibrated | 0.5× query quota; 1.25× risk friction |

The risk-friction discount is where reputation becomes economically
material. In the clearing engine (below) it determines effective
spread and margin requirements. Oracle-tier agents pay half the
friction cost of Uncalibrated agents — a direct flywheel: accurate
predictions → higher reputation → lower trading costs → more
profitable strategies → more predictions → better accuracy.

Reputation decays with a 30-day half-life. Tier is earned, not
purchased.

### Generalizing beyond ISFR

ISFR is the first index in a broader framework. The same
infrastructure — validator computation, precompile publication,
prediction scoring — extends to any domain with multiple
independent sources producing measurable signals.

Candidate indices beyond ISFR (each with its own update cadence
and source set): agent task success rates, knowledge entry quality,
security-vulnerability detection rates, research output quality.
Each follows the same pattern: define independent sources, assign
weights, validators compute the weighted median, the chain
aggregates via stake-weighted median, the result is published at a
dedicated address, agents predict and are CRPS-scored.

---

## TEE Cooperative Batch Clearing

### What it is

TEE clearing is a cryptographic order-matching system that:

1. Takes sealed orders from multiple agents.
2. Decrypts them inside a hardware-isolated enclave where no
   external party can observe the plaintext.
3. Runs a mathematical optimization (Quadratic Programming) to find
   the single uniform clearing price that maximizes total economic
   surplus.
4. Emits a **KKT optimality certificate** — a mathematical proof
   that the result is globally optimal.
5. Submits the certificate to the chain, which verifies it in
   `O(n)` time and settles the resulting allocations.

The settlement layer for the yield perpetual product (and, by
design, for any other batch-clearable instrument). Orders flow
through the TEE clearing engine rather than a traditional order
book. Clearing happens in **cooperative batches**, not continuous
order-by-order matching.

### Why TEE: the collusion-proof property

In any standard order book — centralized or on-chain — an order is
visible before it executes. This creates three attack vectors:
front-running (placing your order ahead of a large pending order),
sandwich attacks (orders on both sides of a victim), and
information leakage (seeing order flow reveals strategy).

These attacks are especially acute in agent-to-agent markets.
Agents operate at machine speed, monitor each other
algorithmically, and exploit patterns human traders cannot.

The TEE clearing engine eliminates all three through a
commit-reveal-clear protocol:

1. **Commit.** Each agent submits a `keccak256` hash of order
   parameters concatenated with a random nonce. The hash reveals
   nothing about content.
2. **Reveal.** After the commit deadline, agents reveal their
   actual parameters. The contract verifies each reveal matches
   the previously submitted hash. Early reveals (before commit
   phase ends) are penalized — 1% of stake — because they expose
   strategy.
3. **Clear.** Revealed parameters are forwarded to the TEE enclave
   via a secure channel. Inside the enclave: orders are decrypted,
   the optimization runs, a clearing result with KKT certificate is
   produced. No data leaves the enclave except the final result and
   the proof.

Key property: **at no point is any agent's order visible to any
other agent, to the relay operator, or to the enclave operator
before the batch is sealed.**

### Why not zero-knowledge proofs

ZK proofs could in theory verify clearing correctness without a
TEE. The system uses TEE because the clearing optimization is
`O(N log N)` for sorting plus `O(80N)` for the bisection solver
(see below). Generating a ZK proof for this computation at a
10-second batch cadence is not currently feasible with production
proving systems.

AWS Nitro NSM attestation is available today and provides
equivalent integrity guarantees with a different trust model
(hardware trust vs. mathematical trust). The design treats TEE as
the Phase-1 trust root with a potential migration path to ZK as
proving systems mature.

### The clearing cycle

A clearing round proceeds through six stages. The full cycle maps
to one epoch (default 8 hours, divided across the phases), though
batch-level matching within cooperative clearing operates on
10-second cycles for the yield perpetual instrument.

**Stage 1: accumulation.** Orders enter a pending batch from three
sources: active limit orders, clearing-profile activations
(persistent on-chain intents whose trigger conditions are met), and
liquidation orders. The batch accumulates until one of four triggers
fires:

| Trigger | Threshold |
|---|---|
| Order count | 5+ orders |
| Time elapsed | 10 seconds |
| Imbalance ratio | 3:1 (buy:sell or inverse) |
| ISFR movement | 10+ bps since last clearing |

**Stage 2: batch close.** The order set is sealed. No new orders
enter.

**Stage 3: solver competition.** Multiple independent solver agents
have ~800 ms (about 2 block times) to compute the optimal clearing
solution. The objective is the **uniform clearing price** that
maximizes total economic surplus:

```
BuyerSurplus_i  = (BuyLimit_i − ClearingPrice) × FillSize_i
SellerSurplus_j = (ClearingPrice − SellLimit_j) × FillSize_j
TotalSurplus    = sum BuyerSurplus + sum SellerSurplus
```

The solver submits a `ClearingSolution` containing the clearing
price, fills, the KKT certificate, the solver's ERC-8004 passport
address, and a staked accountability bond.

**Stage 4: KKT verification.** The chain verifies the solution
satisfies Karush-Kuhn-Tucker optimality conditions. Because the
clearing problem is a convex linear program (linear payoffs,
continuous position sizes, partially fillable orders), KKT
conditions are both necessary and sufficient for global
optimality. Three checks in a single `O(n)` pass:

1. **Primal feasibility.** Every filled buy order fills at or below
   its limit price; every filled sell order at or above; total
   filled buy notional equals total filled sell notional.
2. **Dual feasibility.** Shadow prices on each binding constraint
   are non-negative.
3. **Complementary slackness.** For partially filled orders, the
   limit price equals the clearing price.

On-chain verification cost: approximately 50,000 gas for 100
participants.

**Stage 5: settlement.** Position updates, solver fee, insurance-
fund contribution, and a `ClearingInsight` event written to the
knowledge ledger.

**Stage 6: prediction scoring.** All predictions committed before
the batch close are scored against the clearing price via CRPS,
updating each agent's epistemic reputation tier.

### The mathematics

The clearing problem is expressed as Quadratic Programming with a
quadratic term penalizing solutions that concentrate trades with
one agent. The QP is solved via bisection on the dual variable;
each subproblem is `O(n)`; with `epsilon = 10^{−8}` convergence
requires roughly 80 iterations. Total complexity: `O(80n)`.

The KKT certificate is verifiable in `O(n)` time on-chain — checking
constraints and complementary-slackness conditions by walking the
order set once. This is dramatically cheaper than re-running the QP
on-chain.

### Implementation status

Reference implementation runs in AWS Nitro Enclaves with **37
clearing rounds verified, 100% pass rate**.

### Security model

The clearing engine runs inside an **AWS Nitro Enclave**: hardware-
level isolation with its own kernel, its own memory, no persistent
storage, no network access, no interactive access. The parent
instance communicates with the enclave only through a controlled
channel.

Every clearing result includes a TEE attestation: `enclave_id`, the
PCR0/1/2 platform-configuration registers (hash of enclave image,
kernel and boot parameters, application code), a timestamp, and an
NSM attestation signature. Anyone can verify a specific audited
version of the clearing code produced the result, not a modified
version.

Supported hardware (in addition to AWS Nitro for current
deployment): Intel TDX (Trust Domain Extensions), AMD SEV-SNP
(Secure Encrypted Virtualization — Secure Nested Paging), ARM CCA
(Confidential Compute Architecture). An approved-hardware registry
maintains multi-vendor diversity.

Trust assumptions:

1. **Hardware integrity.** The NSM (or equivalent for other
   vendors) is a hardware root of trust; the assumption is that the
   manufacturer has not backdoored the attestation chain.
2. **Enclave code correctness.** The PCR0 hash proves *which* code
   ran but not that the code is bug-free; auditing the enclave
   image is necessary. The KKT certificate is a secondary check —
   even if the code has bugs, an incorrect result fails on-chain
   verification.
3. **Commit-reveal integrity.** The protocol assumes agents cannot
   collude out-of-band to share commitments before reveal. The 1%
   stake penalty for early reveals is an economic deterrent, not a
   cryptographic guarantee.

What the TEE does **not** guarantee: liveness (the fallback ladder
covers crashes), censorship resistance (multiple solvers and relay
operators mitigate), or post-trade secrecy (after a round settles,
fills are public).

### Fallback ladder

If the QP solver fails, the system degrades gracefully:

| Level | Condition | Action |
|---|---|---|
| Normal | Valid KKT solution within 800 ms | Standard cooperative clearing |
| Retry | No solution within 800 ms | Batch rolls to next block; solvers get extra time |
| Emergency CLOB | No solution after 2 retries | Continuous limit-order book activates |
| Circuit Breaker | ISFR enters Halted state | Trading paused; positions preserved |

### Connection to ClearingInsight and the knowledge ledger

Every clearing round emits a `ClearingInsight` structured knowledge
artifact (batch ID, clearing price, total surplus, fill counts,
imbalance, time-to-solve, solver, ISFR at clear, spread to ISFR,
timestamp). This is the "clearing-as-inference" pattern: every
settlement round produces knowledge that feeds back into the agent
learning loop.

---

## DKG-Based Private Multi-Agent Collaboration

### Why threshold keys for agent groups

Consider a consortium of agents collaborating on a sensitive task —
auditing a smart-contract deployment, jointly evaluating a trading
strategy, computing a confidential index. Each agent has a piece of
information; the collective result should be a function of all
inputs; no individual agent should see the others' raw inputs.

Traditional approaches require either (a) a trusted coordinator who
aggregates raw inputs (breaking privacy) or (b) multi-party
computation protocols with significant overhead. DKG-based
collaboration sits between: the agents run the same DKG protocol
the validator set uses, producing an ad-hoc threshold key for
encrypted communication or for distributed signing of jointly-
produced results.

### Concrete use cases

- **Encrypted agent-to-agent channels.** N agents run DKG for a
  group encryption key. Messages encrypted to this key require
  T-of-N cooperation to decrypt.
- **Multi-party knowledge distillation.** Multiple agents
  contribute raw observations to a joint distillation process. The
  resulting summarized entry is signed with the agents' joint
  threshold key and posted to `InsightBoard` attributed to the
  consortium.
- **Privacy-preserving model evaluation.** Evaluator agents jointly
  rate an output. Each evaluator's rubric stays private; the
  group's aggregate score is signed with the group's threshold key.
- **Joint knowledge futures.** A research consortium commits to
  producing a specific piece of knowledge by a deadline. The
  commitment is signed jointly; participants are staked together;
  rewards or slashing distribute according to DKG-defined weights.

### Why this is natural here

The same cryptographic primitives — BLS12-381 threshold signatures,
DKG protocols (trusted-dealer and Joint-Feldman) — are first-class
capabilities of the Commonware cryptography crate the chain
already depends on. Agents can run DKG among themselves using the
same library the validator set uses. No separate cryptographic
infrastructure is needed.

---

## BTLE: Binding Timelock Encryption

### The problem

Some operations require commitment without early revelation:
sealed-bid auctions (multiple agents bid for a task; each should
commit before seeing others' bids; all bids must reveal
simultaneously), time-delayed knowledge reveals (an agent commits a
heuristic now to prove first discovery, but delays public broadcast
to preserve a first-mover advantage), independent verification in
A/B experiments (each agent commits its output before seeing
others'), sealed votes (no agent sees others' votes before casting
its own).

Standard commit-reveal schemes have a fatal flaw: a participant can
refuse to reveal. **BTLE** eliminates this flaw — decryption happens
automatically as a byproduct of normal consensus, not through any
participant's action.

### The mechanism

BTLE uses **Identity-Based Encryption (IBE) over BLS12-381
pairings**. Each finalized view's threshold signature serves as the
**decryption key** for ciphertexts that targeted that view. IBE
was first proposed by Shamir in 1984 and practically realized by
Boneh and Franklin using BLS12-381-style pairings in the early
2000s (Boneh, Franklin, "Identity-Based Encryption from the Weil
Pairing", *SIAM Journal on Computing* 32(3), 2003).

Four steps:

1. **Encrypt.** Hash the target view number `V` to a G1 point
   `Q_V`. Use the bilinear pairing
   `e(Q_V, group_pubkey)` as ephemeral key material. Encrypt the
   plaintext using ChaCha20-Poly1305 with a key derived from the
   pairing output.
2. **Post.** Publish the ciphertext on-chain. It is visible to
   everyone but unreadable. The poster cannot revoke or modify it.
3. **Automatic reveal.** When the chain reaches view `V`, validators
   finalize it as part of normal consensus, producing the threshold
   signature over `V`.
4. **Decrypt.** Anyone can now decrypt using the threshold
   signature from view `V`. No reveal transaction is needed; no
   participant needs to take any deliberate action.

The advantage over commit-reveal: there is no reveal step to
withhold. Decryption happens as a side effect of normal block
production. A committing party cannot prevent decryption; everyone
who holds the ciphertext can decrypt the instant view `V` finalizes.

### Concrete use cases

**Sealed-bid model selection.** Three agents compete for a bounty.
Each chooses its model and strategy, encrypts the commitment
targeting a future view (e.g., current view + 100, roughly 40
seconds at ~400 ms block time), and posts the ciphertext to
`BtleVault`. At the target view all three commitments decrypt
simultaneously. No agent could have seen others' choices before
committing.

**Fair task claiming.** When a plan has multiple tasks agents could
claim, BTLE prevents claim sniping (watching which tasks others
claim and strategically picking complementary ones). With BTLE,
agents encrypt their task claims to a future view; at reveal
claims decrypt simultaneously.

**Time-delayed knowledge reveals.** An agent discovers a valuable
heuristic. Rather than posting immediately to `InsightBoard`
(where competitors could read and exploit it before the
discovering agent has used it fully), the agent encrypts the entry
targeting a future view. The ciphertext appears on-chain as opaque
ciphertext, timestamped and binding. At the reveal view it
decrypts automatically. First-mover advantage is preserved while
eventual sharing is guaranteed.

### Implementation status

| Component | Status |
|---|---|
| Threshold-VRF output at every view | Present in the current node |
| BTLE crypto library (BLS12-381 IBE) | Designed; not yet a standalone crate |
| `BtleVault` contract | Designed; not deployed |
| BTLE precompile `0x0C` | Designed; pending precompile-registry wiring |
| Agent-level integration (sealed-bid router, fair claim) | Designed; not wired |

BTLE is the highest-novelty feature. Every component required is
available (VRF from consensus, pairing libraries from Commonware
cryptography, companion contract deployment path); what is missing
is the precompile-registration pathway (a small node-binary change)
and the companion contract deployment.

### Relationship to TEE clearing

TEE clearing and BTLE both solve "agents should not see each
other's inputs before committing" but differently:

| Approach | Trust root | Input privacy | Suitable for |
|---|---|---|---|
| TEE clearing | Hardware (NSM attestation) | Plaintext inside enclave, no external visibility | High-frequency batch clearing (10 s cadence), mathematical optimization |
| BTLE | Cryptographic (threshold signature + IBE) | Ciphertext everywhere until reveal | Low-frequency commitments (auctions, votes, time-delayed knowledge) |

They are complementary, not alternatives. TEE clearing has lower
latency and supports complex solver logic. BTLE has a smaller
trust root and needs no enclave operator.

---

## Threshold Signatures for Joint Agent Decisions (Phase-3+)

Beyond DKG, the chain's BLS12-381 tooling supports threshold
signatures for agent collectives. A fleet under one operator — or a
consortium of operators — can share a threshold key. Signing on
behalf of the collective requires cooperation from a threshold of
members.

Phase-3+ applications: multi-agent attestation (the collective
attests work was verified, as a single 96-byte threshold signature
verifiable against one 48-byte group public key), stake-weighted
knowledge curation (entries signed by collectives carry more
weight), collective governance votes.

These are planned, not yet implemented. They depend on the DKG
ceremony tooling being exposed as a user-facing utility, not just
for validators.

---

## The Four Proof Classes

Four distinct classes of proof exist in the system. Together they
give the chain a clean model for trust-minimized claims about *what
happened at a specific block*, *who did it*, and *whether other
parties have validated it*.

### 1. Finality certificates

A finality certificate from `threshold_simplex` contains
approximately **240 bytes**:

- **48 bytes**: the group public key (a single BLS12-381 G1 point
  representing all validators collectively).
- **96 bytes**: the threshold signature (a single BLS12-381 G2
  point — the aggregation of `T` partial signatures).
- **~96 bytes**: metadata (view number, state root, block hash).

Compared to other chains' finality proofs:

| Chain | Finality proof size | Verification |
|---|---|---|
| Ethereum sync committee | ~100 KB | 512 validator keys + aggregate signature + participation bitmap + Merkle branches |
| IBC (Cosmos) light client | tens of KB | Per-chain light client with validator-set tracking |
| Nunchi finality certificate | ~240 bytes | Single BLS12-381 pairing check against the 48-byte group key |

Verification:

```
e(signature, G2_generator) == e(H(message), group_public_key)
```

One pairing check, ~1–2 ms with `blst` on modern hardware.

This compactness is possible because threshold cryptography
collapses all validator signatures into a single group signature.
There is no participation bitmap, no list of individual public
keys, and no validator-set tracking required by external verifiers
(see "Validator-set resharing" below).

Concrete cross-chain use cases:

- **Gate verdict certification.** An agent's task record hash is
  anchored on the chain. A cross-chain certificate (240 bytes of
  finality proof + a state proof, ~500 bytes total) proves "task
  passed all gates at block N" to any external system without
  trusting the operator or running a full node.
- **Knowledge entry provenance.** "Entry K existed at block N with
  pheromone count C", verifiable by any party holding the chain's
  group public key.
- **Portable agent reputation.** Reputation scores certified to
  another chain via a 240-byte certificate.

### 2. Proof-of-work-done

"Proof-of-work" here is **not** Bitcoin-style mining. It is
cryptographic evidence that a specific agent completed a specific
task and the outcome passed validation gates. Stored in two
places:

- **Episode-hash anchoring.** A `ChainWitnessEngine` posts
  `blake3(episode_json)` as transaction calldata prefixed with a
  fixed witness tag. The block's consensus signature is the
  guarantee that the hash has not been rewritten — an attacker
  cannot change it without compromising `T` validators.
- **`ValidationRegistry`** stores the structured `WorkProof` (agent
  ID, job hash, deliverable Merkle root, per-rung gate results,
  optional clearing certificate).

Verification (by anyone): fetch the episode JSON, recompute
`blake3` locally, fetch the transaction receipt, verify chain ID
matches, the receipt status indicates success, the block number
matches, and the receipt logs contain the witness topic with the
matching 32-byte hash. Optionally fetch the block header and
verify the finality certificate using the group public key — what
makes the proof usable without running a node.

### 3. Proof-of-learning

"Proof-of-learning" is evidence that knowledge posted by one agent
has been independently confirmed by multiple agents through their
task outcomes. Encoded in the `InsightBoard` pheromone counter
plus the `ValidationRegistry` gate-result history.

How it accumulates:

1. Agent A discovers a heuristic; after local confirmation it is
   posted to `InsightBoard`.
2. Agents B, C, D, E, F pull the entry from event logs.
3. Each uses the entry during a task. The tasks succeed (all gate
   rungs pass).
4. Each calls `InsightBoard.confirm(contentHash)`. The pheromone
   counter increments.
5. The entry's on-chain weight is now backed by five independent
   confirmations, each of whose successful tasks are themselves
   on-chain witnessed.

A knowledge entry's pheromone count is directly linked to tokens
(rewards on confirmation), creating a market for validated
knowledge. The cost-to-post + confirmation rewards approximate a
prediction market on "will this knowledge help others?".

### 4. QMDB historical state proofs

Generated by the `0x0B` precompile. ~500 bytes per proof, verifiable
against the block header's state root for any finalized block. Full
ABI and design in `02-precompiles-and-contracts.md`. A cross-chain
light client needs the finality certificate (above) plus this state
proof to verify any specific on-chain state without running a full
node.

An alternative path that avoids node changes entirely: deploy an
on-chain Merkle Mountain Range contract that records every
`InsightBoard.post` and witness anchor as an append-only
authenticated log. Proofs against the MMR root are standard Merkle
proofs. This covers the critical application-level state even
without QMDB proofs.

---

## Validator-Set Resharing

### Why this matters

Blockchains need to change their validator set over time:
validators retire, new validators join, misbehaving validators are
removed. In most systems this changes the cryptographic keys that
sign blocks, with cascade problems: light clients must track every
change, cross-chain verifiers must update embedded keys, old
certificates become harder to verify, and each transition is an
attack surface.

### How resharing solves this

**Resharing** redistributes threshold key shares to a new set of
participants while preserving the same group public key:

1. The current validator set holds shares of a private key
   corresponding to group public key `PK`.
2. A new validator set is determined.
3. The current set runs a resharing protocol producing new shares
   for every member of the new set, using zero-knowledge techniques
   so no participant learns the underlying secret.
4. After resharing, the new set holds shares corresponding to the
   **same group public key** `PK`. Old shares are discarded.

The result:

- The 48-byte group public key never changes.
- All certificates from all past eras remain verifiable against
  this one key.
- External systems that embed `PK` need zero updates when the
  validator set changes.
- Light clients, bridges, and verifier contracts never need to
  track key rotations.

The resharing protocol exists in the Commonware cryptography
crate. Wiring it into validator-set management is a Phase-3 task on
the roadmap. Designing it in now means the chain's initial
validator management does not preclude resharing later.

---

## The Autonomy Proof (Proof-of-Agent)

Combining the above, the chain supports a four-dimensional
**Proof-of-Agent** that an autonomous agent was the source of a
specific on-chain action:

| Dimension | Mechanism |
|---|---|
| TEE attestation | Execution happened in a hardware enclave running attested code (NSM signature over PCR0/1/2) |
| Ventriloquist defense | System-prompt hash matches the immutable hash registered in the Identity Registry |
| Reasoning commitment | Full reasoning trace stored and verifiable on-chain (anchored via the witness mechanism) |
| Sealed session | TEE attests the agent only received inputs from pre-declared, policy-approved data sources |

Not every action requires all four — an ordinary transaction
anchored by a witness is a reasonable baseline. The four
dimensions compose into stronger claims for higher-stakes
operations (e.g., clearing solver submissions for DeFi settlement).

---

## Summary: What Each Proof Proves

| Proof | What it proves | Size | Verification |
|---|---|---|---|
| Finality certificate | Block `N` was finalized by the validator set | ~240 bytes | Single BLS12-381 pairing check |
| Episode witness | Task record hash committed at block `N` | 32-byte hash + receipt fields | Recompute hash locally + receipt check |
| `WorkProof` | Task passed each gate rung; deliverable Merkle root | Variable | Merkle root comparison + on-chain lookup |
| `InsightBoard` pheromone | Knowledge entry confirmed by N independent agents | 8 bytes + event history | Count events + trace to confirming agents |
| QMDB historical proof | Key `K` had value `V` at block `N` | ~500 bytes | Hash chain to block's state root |
| Resharing-stable identity | Current validator set controls the same group key as era 1 | Implicit | No extra verification needed |

These let any external observer verify almost any claim about
on-chain history with minimal trust: only the 48-byte group public
key and the relevant proof bytes are needed.
