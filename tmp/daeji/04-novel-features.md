# Novel Features Enabled by Commonware Primitives

This document describes eight features that become uniquely possible because daeji
is built on commonware rather than a standard Ethereum execution stack. Commonware
is a Rust library of composable blockchain primitives: consensus algorithms, threshold
cryptography, authenticated storage, and peer-to-peer networking. Daeji assembles
these primitives into a custom application-specific chain for AI agent coordination.

Each feature below is explained from first principles, then applied concretely to
roko -- a self-developing Rust toolkit (18 crates, ~177,000 lines of code) that
orchestrates AI coding assistants to execute implementation tasks. Roko's core loop:
read a PRD (Product Requirements Document) -> generate a plan of tasks -> dispatch
agents to execute each task -> verify output through a 7-rung gate pipeline -> persist
results -> learn from outcomes. Every piece of this loop that a commonware feature
could touch is identified below.

---

## 1. Binding Timelock Encryption (BTLE) for Agent Commitments

### Background: Threshold Cryptography

In standard public-key cryptography, a single private key controls all signing and
decryption. If that key is compromised, security is entirely lost. If the key holder
is unavailable, the system is entirely stuck.

Threshold cryptography distributes this power across a group. A (T, N) threshold
scheme works as follows:

- **N participants** each hold a **share** of a private key. No single participant
  holds the complete private key. The full key is never assembled in one place.
- **Any T of those N participants** can cooperate to produce a valid signature or
  decrypt a ciphertext. This cooperation involves each participant computing a
  partial result from their share, then combining the partial results mathematically.
- **Fewer than T participants learn nothing.** Even if T-1 participants collude and
  pool their shares, they gain zero information about the private key or any
  signature/decryption it could produce.

The group also has a single **group public key**. Anyone can verify signatures
produced by the threshold group using only this public key, without knowing which
specific T participants cooperated, or even that threshold cryptography was used at
all. From the verifier's perspective, the signature looks identical to one produced
by a single signer.

Daeji uses **BLS12-381**, an elliptic curve specifically designed for threshold
signatures. BLS signatures have a special property: partial signatures from different
participants can be mathematically combined (aggregated) into a single short
signature. A BLS group public key is 48 bytes. A BLS signature is 96 bytes.

### Background: Verifiable Random Functions (VRFs)

A Verifiable Random Function is a cryptographic primitive that takes two inputs (a
private key and a message) and produces two outputs:

1. A **pseudorandom value** that is deterministic (the same key and message always
   produce the same value) but appears random to anyone without the key.
2. A **proof** that the value was correctly derived from that specific message using
   the key corresponding to a known public key.

The critical properties:

- **Deterministic**: Given the same inputs, the output is always the same. The key
  holder cannot "try different outputs" and pick a favorable one.
- **Unpredictable**: Without the private key, the output is computationally
  indistinguishable from random, even if you know the message.
- **Verifiable**: Anyone with the public key can check that the output and proof are
  valid for the given message.

When threshold cryptography is combined with a VRF, the result is a **threshold VRF**:
the group collectively produces a random value that no single member could have
predicted or influenced, and anyone can verify it. In daeji's consensus, validators
hold BLS12-381 key shares. When a view (consensus round) finalizes, each validator
produces a partial signature over the view number. Once T partial signatures are
collected and aggregated, the resulting threshold signature over the view number
serves as the VRF output: deterministic, unpredictable, unbiasable, and verifiable
against the 48-byte group public key.

### How BTLE Works

BTLE (Binding Timelock Encryption) allows data to be encrypted such that it can only
be decrypted after a specific future point in the blockchain's consensus progress.
No trusted third party is involved. The decryption happens automatically as a
byproduct of consensus.

The mechanism uses Identity-Based Encryption (IBE) over BLS12-381 pairings:

**Step 1 -- Encrypt.** Take a plaintext message and a target future view number V.
Using the group public key and the view number V, perform a pairing-based encryption
operation. Specifically: hash V to a point on the BLS12-381 G1 curve, then use a
bilinear pairing `e(H(V), group_pubkey)` as ephemeral key material. The resulting
ciphertext can only be decrypted by someone who possesses the threshold signature
over V. That signature does not exist yet because view V has not been finalized.

**Step 2 -- Post.** Publish the ciphertext on-chain. It is visible to everyone but
unreadable. The poster cannot revoke or modify it.

**Step 3 -- Automatic reveal.** When the blockchain reaches view V, validators
finalize it as part of normal consensus. Finalization produces the threshold signature
over V (this happens automatically -- it is how the consensus protocol works, not an
extra step). This threshold signature is the decryption key. It becomes public as
soon as the view is finalized.

**Step 4 -- Decrypt.** Anyone can now decrypt the ciphertext using the threshold
signature from view V. No "reveal" transaction is needed. No participant needs to
take any deliberate action.

This is **binding**: the ciphertext cannot be changed after posting. It is
**timelock**: decryption is impossible before view V finalizes. It is
**automatic**: decryption keys are produced by the consensus process itself.

The key advantage over traditional commit-reveal schemes: in commit-reveal, a
participant commits a hash of their value, then must later reveal the value in a
second transaction. A participant can simply refuse to reveal if the outcome would
be unfavorable (they pay a small penalty but break the protocol). In BTLE, there is
no reveal step. Decryption is automatic and cannot be withheld.

### Concrete Roko Use Cases

**Sealed-bid model selection.** Roko's `CascadeRouter` (in `roko-learn/src/cascade_router.rs`)
selects which LLM model handles each task. Today, the router progresses through three
stages: Static (hardcoded role-to-model table), Confidence (empirical pass rates), and
UCB1 (LinUCB contextual bandit). When multiple operators run agents against the same
codebase, model selection becomes a competitive advantage -- operators do not want others
to see which models they route to (it reveals their cost/quality strategy). BTLE enables
sealed-bid model selection: each operator encrypts their model choice to a future view.
At reveal, all choices decrypt simultaneously. A smart contract selects the winning bid
(e.g., the operator offering the lowest cost for a given quality tier). No operator can
see another's model choice before committing.

Concrete example: Three operators compete for a task ("implement the rate limiter"). Each
encrypts their bid -- Operator A bids Claude Sonnet at $0.15/task, Operator B bids GPT-4o
at $0.12/task, Operator C bids Claude Haiku at $0.04/task. During the ~100-view bidding
window (about 40 seconds at 400ms blocks), no operator sees another's bid. At reveal,
the contract selects Operator C (lowest cost), and verifiable logs prove no front-running
occurred.

**Commit-reveal for task claims.** When a plan has multiple tasks that agents could claim,
BTLE prevents "claim sniping" -- where an agent watches which tasks others are claiming
and strategically picks easier complementary tasks. With BTLE, agents encrypt their task
claims to a future view. At reveal, claims decrypt simultaneously. If two agents claim the
same task, the contract arbitrates (e.g., by reputation score or random tiebreak using the
VRF). This is relevant to roko's orchestrator (`orchestrate.rs`), which today assigns tasks
sequentially -- BTLE would enable fair parallel claiming when multiple agents are active.

**Time-delayed knowledge reveals.** An agent discovers a valuable heuristic during task
execution (e.g., "this library's async API requires Tokio 1.38+"). Rather than posting
it immediately to the InsightBoard (where competitors could read and exploit it before the
discovering agent has used it fully), the agent encrypts the knowledge entry targeting a
future view (say, 500 views later -- about 200 seconds). The entry appears on-chain
immediately as opaque ciphertext, timestamped and binding. At the reveal view, it decrypts
automatically and becomes available to all agents. This creates a first-mover advantage
window: the discovering agent gets a head start using the knowledge, while still
guaranteeing eventual sharing.

**Independent multi-agent verification.** When running A/B experiments across agents
(roko's `ExperimentStore` at `.roko/learn/experiments.json` already supports this), each
agent commits its output via BTLE before seeing others' results. At the reveal view, all
outputs surface simultaneously. No agent could have been influenced by another's output.
This property cannot be achieved with a trusted coordinator -- the coordinator can
always peek.

**Where BTLE touches roko's universal loop.** BTLE is primarily a **Route** operation: it
mediates how tasks and models are selected among competing agents/operators. It also
touches **Store** (encrypted knowledge entries posted to chain before reveal) and
**Verify** (the reveal mechanism is a cryptographic verification that the committed
value matches what was encrypted).

**What roko code already exists that could use BTLE.**
- `CascadeRouter` in `roko-learn`: model selection logic that could use BTLE for
  sealed-bid routing when multiple operators participate
- `orchestrate.rs`: task dispatch path where BTLE would mediate task claims
- `roko-neuro` `KnowledgeStore`: knowledge ingestion path where BTLE would gate
  time-delayed knowledge sharing

### Implementation Path

| Step | Work Required | Status |
|------|--------------|--------|
| VRF output at every view | `SeedReporter` fires on finalization | Exists in daeji |
| BTLE crypto library | Rust crate: BLS12-381 IBE encrypt/decrypt | Build |
| Commitment contract | Solidity: post commitment, auto-reveal at target view | Build |
| BTLE precompile 0x0C | Native precompile in daeji's REVM executor (see doc 05) | Build |
| Agent integration | Wire sealed commitment into CascadeRouter bid/vote flow | Build |

---

## 2. On-Chain VRF as a Verifiable Randomness Source

### Background: What prevrandao Is

In the Ethereum Virtual Machine (EVM), every block contains a field that smart
contracts can access via the `PREVRANDAO` opcode. Historically this field was called
`DIFFICULTY` (it held the proof-of-work difficulty). Since Ethereum's transition to
proof-of-stake (the "merge"), this field was repurposed to hold a pseudorandom value
called `prevrandao`.

On Ethereum mainnet, `prevrandao` comes from the RANDAO mechanism: each validator
mixes a random contribution into a running accumulator over the course of an epoch
(32 blocks). Smart contracts access it via `block.prevrandao` in Solidity.

The RANDAO mechanism has a known weakness: the last validator to contribute in an
epoch can choose to withhold their block. By withholding, they can bias the final
accumulator value at the cost of forfeiting their block reward. For high-value
applications (large lottery payouts), this bias is a meaningful attack vector.

### How prevrandao Is Different on Daeji

On daeji, `prevrandao` is not a RANDAO accumulator. It is the output of a threshold
VRF (as described in section 1). At each finalized view, the threshold signature
over the view number is hashed and placed in the `mixHash` field of the block
header (the EVM-compatible name for prevrandao).

This provides stronger properties than Ethereum's RANDAO:

- **Bias-resistant**: No single validator controls the output. The threshold
  signature is deterministic for a given view number. A validator cannot "try
  different values" -- the input (view number) is fixed, and the output is uniquely
  determined by the group key and that input.
- **Unpredictable**: Until T validators produce their signature shares and the
  threshold signature is assembled, nobody can compute the VRF output.
- **Verifiable**: Anyone can verify the VRF output against the 48-byte group
  public key.
- **Free and per-block**: No oracle call, no additional transaction, no external
  service, no fee. The randomness is a natural byproduct of consensus and is
  available in every single block.

### Concrete Roko Use Cases

**Agent selection from pools.** Roko's agent system supports 28 roles (Implementer,
Reviewer, Planner, Debugger, Architect, etc.) and can run multiple agents
simultaneously. When a plan has a task that multiple agents are qualified for -- say,
three Implementer agents are idle -- VRF-based selection is provably fair:
`selected = hash(prevrandao, task_id) % qualified_agents.len()`. No operator can
claim favoritism. The selection is deterministic (reproducible from public block data)
and unpredictable (no agent knows in advance which task it will receive).

In roko's orchestrator (`orchestrate.rs`), the `ProcessSupervisor` in `roko-runtime`
tracks all active agents. Today, task assignment is sequential. VRF-based assignment
would make the dispatch path verifiably random when multiple qualified agents exist.

**Experiment assignment.** Roko's `ExperimentStore` (in
`.roko/learn/experiments.json`) supports A/B experiments across prompt variants, model
choices, and gate thresholds. Today, variant assignment uses local randomness.
VRF-based assignment would make every assignment auditable:
`variant = hash(agent_id, experiment_id, prevrandao) % num_variants`.
Because `prevrandao` is on-chain, anyone can independently recompute the assignment
for any task and verify it was correct. This matters when experiment results affect
model routing (the CascadeRouter uses experiment outcomes to adjust model weights) --
verifiable assignment proves the routing optimization was not rigged.

Concrete example: The CascadeRouter is testing whether Claude Sonnet or GPT-4o
performs better on Rust async tasks. Over 200 tasks, VRF assigns each task to a
variant. After the experiment, anyone can verify that each assignment was determined
by the VRF seed and was not cherry-picked to favor one model.

**Task shuffling for fairness.** When a plan contains 20 tasks and the orchestrator
must choose an execution order (within DAG constraints), VRF-seeded shuffling
prevents systematic bias. Without shuffling, earlier tasks in the plan file might
always get assigned to the fastest model, while later tasks get the remainder. With
VRF shuffling, the execution order is unpredictable but reproducible:
`shuffled_order = sort_by(hash(prevrandao, task_id))`.

**Anti-gaming gate thresholds.** Roko's gate pipeline (7 rungs: Compile -> Lint ->
Test -> Symbol -> GeneratedTest -> PropertyTest -> Integration) uses adaptive
thresholds stored as exponential moving averages in
`.roko/learn/gate-thresholds.json`. VRF-derived noise can jitter these thresholds:
`threshold = base_threshold + noise(prevrandao, task_id) * jitter_range`.
The jitter range is small (plus or minus 5%). This prevents an agent from learning
to produce work that just barely clears the gate. The noise is unpredictable to the
agent at execution time but verifiable by auditors after the fact.

**Where VRF touches roko's universal loop.** VRF is primarily a **Route** input:
it provides the randomness seed for model selection, experiment assignment, and task
ordering. It also touches **Verify** (gate threshold jittering) and **Score** (when
VRF seeds affect how candidate models are scored in the CascadeRouter's UCB1 stage).

**What roko code already exists that could use VRF.**
- `CascadeRouter` in `roko-learn`: model selection that already uses randomness
  (could switch from local PRNG to VRF seed)
- `ExperimentStore` in `roko-learn`: experiment variant assignment
- `orchestrate.rs`: task scheduling and dispatch ordering
- Gate threshold adaptation in `.roko/learn/gate-thresholds.json`

### Implementation Path

| Step | Work Required | Status |
|------|--------------|--------|
| VRF in block headers | `mixHash` = threshold VRF output | Exists in daeji |
| Read from Solidity | `block.prevrandao` opcode | Works today |
| Read off-chain | `eth_getBlockByNumber` JSON-RPC, read `mixHash` field | Works today |
| Agent framework integration | Read VRF seed in CascadeRouter and ExperimentStore | Build |

No chain changes are required. The randomness is already produced and available.

---

## 3. 240-Byte Cross-Chain Certificates

### Background: What Finality Means

"Finality" on a blockchain means a block (and all transactions within it) is
permanently committed and will never be reverted. Different blockchains achieve
finality differently:

- **Probabilistic finality** (Bitcoin, pre-merge Ethereum): A block becomes
  progressively less likely to be reverted as more blocks are built on top of it.
  After 6 blocks on Bitcoin (~60 minutes), a reversal is considered impractical.
  But it is never mathematically impossible -- just economically irrational.
- **Deterministic finality** (Tendermint/CometBFT, commonware's threshold_simplex):
  Once a supermajority of validators sign off on a block, it is final. Period. No
  amount of subsequent computation or economic incentive can revert it (assuming
  the fault-tolerance threshold is not exceeded).

To trust a blockchain's state from the outside (from a different blockchain, an
off-chain application, or a mobile wallet), you need a **finality proof**: evidence
that the block was actually committed by the chain's validator set.

### Background: Light Clients

A full blockchain node downloads and verifies every block, every transaction, and
every state transition from genesis. This can require terabytes of storage and days
of synchronization.

A **light client** is software that verifies blockchain state without storing the
full history. It tracks only:

- The current validator set (or a representative committee).
- Block headers (which contain the state root -- a cryptographic digest of all
  state).
- Validator signatures attesting to each block.

To verify a specific piece of state (e.g., "account X has balance Y"), a light
client checks: (1) the state is included in a block header via a Merkle proof,
and (2) the block header is signed by a sufficient set of validators.

Light clients are used by bridges (systems connecting two blockchains), mobile
wallets, and any application that needs to verify chain state without running a
full node.

### Background: IBC and Ethereum Finality Proofs

**IBC (Inter-Blockchain Communication)** is a protocol originating in the Cosmos
ecosystem for verified communication between independent blockchains. Each chain
runs a light client for the other chain. Verification requires tracking validator
set changes, storing headers, and processing signature sets. It works but is
heavyweight.

**Ethereum finality proofs** are even larger. Ethereum's beacon chain rotates a
sync committee of 512 validators every ~27 hours. A finality proof includes:

- The sync committee's aggregate BLS signature (~96 bytes).
- A participation bitmap showing which of the 512 validators signed (64 bytes).
- The sync committee's public keys (512 validators x 48 bytes = 24,576 bytes).
- Block header data and Merkle branches connecting the beacon state to the execution
  payload.

Total proof size: approximately **100KB**. Verifying this on another chain is
expensive in both gas and implementation complexity.

### How Commonware Compresses This to 240 Bytes

A commonware chain using `threshold_simplex` consensus produces finality certificates
that contain:

- **48 bytes**: the group public key (a single BLS12-381 G1 point representing all
  validators collectively).
- **96 bytes**: the threshold signature over the block (a single BLS12-381 G2 point
  -- the aggregation of T partial signatures).
- **~96 bytes**: metadata (view number, state root, block hash).

Total: approximately **240 bytes**.

This dramatic compression is possible because threshold cryptography collapses all
validator signatures into a single group signature. There is no participation bitmap
(every valid certificate has exactly one signature). There is no list of individual
public keys (the group is represented by one 48-byte key). There is no validator set
tracking needed by the verifier (the group public key is stable across resharing
events, as described in section 5).

Verification is a single BLS12-381 pairing check: `e(signature, G2_generator) == e(H(message), group_public_key)`. This is one of the most well-optimized operations
in blockchain cryptography.

### Concrete Roko Use Cases

**Gate verdict certification.** Roko's 7-rung gate pipeline (Compile -> Lint -> Test
-> Symbol -> GeneratedTest -> PropertyTest -> Integration, implemented in
`roko-gate`) produces a `Verdict` for each task: passed/failed, evidence, logs. Today,
gate verdicts are recorded locally in `.roko/episodes.jsonl` as `GateVerdict` records
(gate name, passed boolean, optional diagnostic signature). The `ChainWitnessEngine`
in `roko-chain` can hash episode data and submit it to daeji.

With cross-chain certificates, a gate verdict recorded on daeji can be proven to
any external system. The proof is: a 240-byte finality certificate + a Merkle
inclusion proof against the certified state root. Total: ~500 bytes. An external CI
system, a code review platform, or another blockchain can independently verify that
"task X passed all 7 gate rungs at block N" without trusting roko's operator or
running a daeji node.

**Knowledge entry provenance.** The neuro store (`roko-neuro`) tracks knowledge
entries with confidence scores, tier levels, and confirmation counts. When knowledge
is posted to the InsightBoard contract on daeji, a cross-chain certificate can prove
to any external system that "knowledge entry K existed at block N with confidence C
and was confirmed by 5 agents." This is ~500 bytes of proof, compared to ~100KB
for proving the same fact on Ethereum.

**Episode summary attestation.** Each episode in `.roko/episodes.jsonl` contains:
agent ID, task ID, prompt, outcome, tool calls, gate verdicts, HDC fingerprint,
timestamps, and model used. The `blake3` hash of this data can be anchored to daeji
via the `ChainWitnessEngine`. A cross-chain certificate proves the episode summary
hash was committed at a specific block, creating a tamper-evident audit trail that
any external system can verify with only daeji's 48-byte group public key.

**Portable agent reputation.** Roko tracks agent performance through multiple
learning subsystems: per-model pass rates in the CascadeRouter, efficiency events
(C-Factor), playbook success counts, and episode outcomes. If this reputation data
is stored on daeji (via the AgentRegistry contract), a 240-byte certificate lets an
agent prove its track record to any system that knows daeji's group public key.

**Where certificates touch roko's universal loop.** Certificates are a **Verify**
output: they prove that a verification (gate verdict, knowledge confirmation) happened.
They also touch **Store** (proving what was stored at a given time) and **React**
(external systems can react to certified events -- e.g., a CI system auto-merging
a PR when it sees a certified gate-pass from daeji).

**What roko code already exists that could use certificates.**
- `ChainWitnessEngine` in `roko-chain`: already anchors episode hashes to daeji
- Gate pipeline in `roko-gate`: produces verdicts that could be certified
- `KnowledgeStore` in `roko-neuro`: entries that could have provenance certificates
- `AgentRegistry.sol` in `contracts/`: agent identity that could be portably proven

### Implementation Path

| Step | Work Required | Status |
|------|--------------|--------|
| Certificate production | Simplex consensus produces certificates | Exists in daeji |
| Certificate export API | New RPC method or P2P extraction | Build |
| Verifier contract | Solidity contract on target chain (~200 LOC), BLS pairing check | Build |
| Certificate relay | Fetch certificate from daeji, submit to target chain verifier | Build |

---

## 4. Deterministic Simulation for Agent Testing

### Background: What "Deterministic" Means Here

A deterministic system produces identical output given identical input. For a single
function, this is straightforward. For a distributed system with multiple nodes
communicating over a network, determinism is extremely difficult because:

- **Network message ordering** is non-deterministic: messages between nodes arrive
  in different orders on different runs, depending on network conditions.
- **Thread scheduling** varies: the operating system's scheduler makes different
  decisions on each run.
- **System clocks** drift and differ between machines.
- **Random number generators** are typically seeded from OS entropy, which differs
  between runs.

Any of these sources of non-determinism means that running the same distributed
system twice produces different behavior, making bugs difficult to reproduce and
testing unreliable.

Commonware provides a `runtime::deterministic` module that replaces all sources of
non-determinism:

- A **seeded pseudo-random number generator** replaces OS entropy. Given the same
  seed, the same sequence of "random" numbers is produced.
- A **simulated clock** replaces wall-clock time. Time advances only when the
  simulation advances it.
- **Simulated task scheduling** replaces OS thread scheduling. Tasks execute in a
  deterministic order controlled by the simulation.

Code written against commonware's runtime abstraction runs identically whether using
the real runtime (production) or the deterministic runtime (testing). No code changes
are needed to switch between them.

### Background: Simulated P2P Networking

Commonware's `p2p::simulated` module provides a fake network within a single
operating system process. Instead of opening TCP connections between nodes on
different machines:

- All "nodes" exist as tasks within one process.
- Messages between nodes are passed through in-memory channels.
- The simulated network can model: configurable latency per link, message loss
  (configurable drop probability), network partitions (arbitrary subsets of nodes
  isolated from each other), and Byzantine behavior (nodes programmed to send
  conflicting or malicious messages).

Combined with the deterministic runtime: an entire multi-validator blockchain
network (consensus, P2P, state storage, EVM execution) runs in a single process,
with a fixed random seed, producing identical behavior on every run with that seed.

### Concrete Roko Use Cases

**Agent execution replay for debugging.** Roko records every agent turn in
`.roko/episodes.jsonl` (agent ID, task ID, prompt, outcome, tool calls, gate verdicts,
HDC fingerprint). When a task fails, the episode log shows what happened but not
_why_ the chain behaved as it did during the agent's execution. Deterministic
simulation allows replaying the exact chain state the agent interacted with: the same
blocks, the same VRF outputs, the same contract state -- all from a single seed. If
the agent submitted a transaction that failed, the replay shows exactly which state
condition caused the failure.

This is particularly valuable for debugging the `ChainWitnessEngine` (which anchors
episode hashes) and the neuro-chain sync path (which would push/pull knowledge
entries to/from the InsightBoard). Both involve multi-step agent-chain interactions
where timing and state dependencies are hard to reproduce with real networks.

**Plan re-runs for regression testing.** Roko's orchestrator (`orchestrate.rs`)
executes plans: DAGs of tasks dispatched to agents, verified by gates, with state
persisted to `.roko/state/executor.json`. When the orchestrator code changes, a
regression test should verify that the same plan produces the same gate verdicts.
With deterministic simulation, the test spins up a simulated daeji network, replays
a known plan, and asserts that every gate verdict matches the baseline. No Docker.
No network setup. The test runs as a single Rust unit test in milliseconds.

**Pre-deployment contract simulation.** Before deploying the InsightBoard or
AgentRegistry contracts to the real daeji devnet, the deterministic simulator can
execute the full deployment sequence: deploy contract, call register(), post a
knowledge entry, confirm it, read it back, verify the gas costs and storage layout.
If the simulation passes, proceed to real deployment with confidence.

**Gate rung simulation.** Roko's gate pipeline verifies agent output against ground
truth. A new gate rung (e.g., the `VerifyChainGate` at rung 4) could include a
simulation step: spin up a simulated daeji network, replay the agent's chain
interactions in simulation, and verify they produce the expected state changes. This
is analogous to fork-mode testing but includes full consensus simulation rather than
just EVM execution.

**Where simulation touches roko's universal loop.** Simulation is primarily a
**Verify** tool: it provides deterministic environments for verifying agent behavior.
It also touches **Store** (replaying historical state) and **React** (simulation
results trigger replanning or debugging workflows).

**What roko code already exists that could use simulation.**
- `roko-gate` gate pipeline: could add a simulation gate rung
- `roko-chain` `ChainWitnessEngine`: witness anchoring needs integration testing
- `orchestrate.rs`: plan execution needs regression testing with chain interactions
- `roko-learn` `forensic_replay` module: forensic replay of failed tasks could use
  deterministic chain simulation

### Implementation Path

| Step | Work Required | Status |
|------|--------------|--------|
| Deterministic runtime | `runtime::deterministic` module | Exists in commonware |
| Simulated P2P | `p2p::simulated` module | Exists in commonware |
| End-to-end test harness | Daeji's `crates/e2e/` test infrastructure | Exists in daeji |
| Library exposure | Add `[lib]` target so external code can instantiate harness | Build |
| Simulation gate | Gate rung that spins up simulated network per task | Build |

---

## 5. Validator Set Resharing (Stable Chain Identity Across Membership Changes)

### The Problem: Changing Validators Breaks External Verifiers

Blockchains need to change their validator set over time: validators retire, new
validators join, misbehaving validators are removed. In most systems, changing the
validator set means changing the cryptographic keys that sign blocks. This creates
a cascade of problems:

- **Light clients must track every validator set change.** Each epoch transition
  requires the light client to verify a "handoff" from the old validator set to the
  new one.
- **Cross-chain verifiers must update their knowledge.** A bridge contract that
  verifies finality proofs from another chain must store and update the signing keys.
- **Old certificates become harder to verify.** To verify a certificate from era 3,
  you need the validator keys from era 3, which means tracking the full history of
  key changes.
- **Each transition is an attack surface.** An attacker who compromises the handoff
  between validator sets can forge certificates.

### How Resharing Solves This

Resharing is a cryptographic protocol that redistributes threshold key shares to a
new set of participants while preserving the same group public key. The process:

1. The current validator set (call them Set A) holds shares of a private key
   corresponding to group public key PK.
2. A new validator set is determined (call them Set B). Some members may overlap
   between A and B, some may be entirely new, some from A may be leaving.
3. Set A runs a resharing protocol that produces new shares for every member of
   Set B. This protocol is interactive (A members cooperate with B members) and
   uses zero-knowledge techniques to ensure no participant learns the underlying
   secret.
4. After resharing, Set B holds shares corresponding to the **same** group public
   key PK. Set A's old shares are discarded (they are no longer valid for producing
   threshold signatures).

The result:

- The group public key **never changes** (same 48 bytes, forever).
- All certificates from all past eras remain verifiable against this one key.
- External systems that know PK need **zero updates** when the validator set
  changes.
- Light clients, bridges, and verification contracts never need to track key
  rotations.

### Concrete Roko Use Cases

**Agent fleet membership changes.** A roko fleet is a group of agents under one
operator sharing a `roko.toml` config. In a multi-operator deployment where agents
participate as daeji validators, the fleet composition changes over time: new agents
are provisioned, old agents are decommissioned, agents are upgraded to new model
backends. If validator membership is tied to agent identity, resharing allows the
validator set to evolve without changing the chain's cryptographic identity.

This matters for every external system that verifies daeji certificates: CI systems,
code review platforms, cross-chain bridges, audit tools. They all embed daeji's
48-byte group public key. Without resharing, every validator change requires updating
every external verifier. With resharing, the key is stable forever.

**Key rotation for long-running fleets.** Even in a single-operator deployment (the
current 4-validator devnet), periodic key rotation is security hygiene. Resharing
enables rotation without disrupting certificate verification: rotate validator keys
monthly, maintain the same group public key, and all historical certificates remain
valid.

Roko's `ProcessSupervisor` manages agent lifecycles (spawn, heartbeat, shutdown).
When a supervisor rotates agents (e.g., upgrading from Claude Sonnet to a new model),
resharing ensures the chain identity survives the rotation. The supervisor already
tracks which agents are active -- wiring resharing triggers into the
lifecycle-transition events (`LifecycleTransition` in `roko-runtime/src/lifecycle.rs`)
would automate validator-set updates.

**Ephemeral task chains.** For computationally expensive multi-agent tasks (large
codebase refactors, comprehensive test generation across many crates), a purpose-built
chain could be spun up with the participating agents as validators. The 48-byte group
public key permanently identifies this task chain. Certificates from it remain
verifiable indefinitely, serving as a permanent audit trail for the task.

**Where resharing touches roko's universal loop.** Resharing is infrastructure that
underlies **Store** (the chain's identity is how external systems trust stored data)
and **Verify** (certificates verified against the stable group key). It does not
directly touch agent-level traits but enables all certificate-dependent features
(section 3) to work across validator set changes.

**What roko code already exists that could use resharing.**
- `ProcessSupervisor` in `roko-runtime`: lifecycle management for agents that could
  trigger resharing when fleet membership changes
- `roko-chain` `AlloyChainClient`: the RPC client would need to discover and connect
  to new validators after resharing
- Cross-chain certificate verification (section 3): depends on stable group key

### Implementation Path

| Step | Work Required | Status |
|------|--------------|--------|
| Resharing protocol | `commonware-cryptography` supports resharing | Exists in commonware |
| Daeji integration | Wire resharing into validator management | Not yet built |
| Reputation-gated membership | Smart contract controlling validator eligibility | Design phase |

This is a longer-term feature. The architecture should be designed now so the chain's
initial validator management does not preclude resharing later.

---

## 6. QMDB Historical State Proofs

### Background: What QMDB Is

QMDB (Quick Merkle Database) is a purpose-built database for blockchain state
storage, developed in collaboration with LayerZero. It addresses two requirements
that conflict in standard databases:

**Fast updates.** Blockchain state changes with every block. Each block may modify
thousands of storage slots (account balances, contract variables, nonces). Updates
must complete in milliseconds.

**Cryptographic commitment.** The entire state must be summarized as a single hash
(the "state root") included in each block header. Computing this root requires
"Merkleization": building a hash tree over all state entries. In a standard Merkle
tree, updating one leaf requires recomputing hashes along a path from that leaf to
the root -- O(log N) hash computations per update, each potentially requiring a
random disk read.

At scale (millions of storage slots), the combination of random I/O for Merkle
updates and sequential I/O for state writes creates a bottleneck. Ethereum's state
database (LevelDB or PebbleDB with a Merkle Patricia Trie) is one of the primary
performance limiters for node operation.

QMDB solves this with two key design decisions:

- **O(1) SSD I/O per state update**: State changes are appended to a write-ahead
  log and batched. The on-disk data structure is organized for sequential writes,
  avoiding the random I/O pattern that makes Merkle updates slow.
- **In-memory Merkleization**: The Merkle tree is maintained entirely in RAM.
  Computing the state root after a batch of updates requires no disk reads. This
  is possible because the Merkle tree structure (internal nodes and their hashes)
  is much smaller than the state data itself.

The result: a state database handling millions of entries with sub-millisecond
update latency and instant state root computation.

### Background: Merkle Proofs

A Merkle tree is a binary tree of cryptographic hashes:

- Each **leaf** is the hash of one data item (e.g., a storage slot's key-value pair).
- Each **internal node** is the hash of its two children.
- The **root** (at the top) is a single hash that cryptographically commits to every
  piece of data in the tree.

A **Merkle inclusion proof** proves that a specific data item is part of the tree
without revealing the entire tree. The proof consists of the sibling hashes along the
path from the leaf to the root. The verifier:

1. Hashes the claimed data to get the leaf hash.
2. Combines the leaf hash with the first sibling hash (provided in the proof) to
   compute the parent hash.
3. Repeats up the tree, combining each computed hash with the next sibling from the
   proof.
4. Checks that the final computed hash matches the known root.

Proof size is logarithmic: for a tree with N leaves, the proof contains log2(N)
hashes. For 1 million leaves with 32-byte hashes, a proof is about 20 hashes = 640
bytes.

A **Merkle exclusion proof** proves that a key does NOT exist in the tree. The
mechanism varies by tree type but typically involves showing the two adjacent keys
that bound the absent key, proving there is no space between them for the missing
key.

### What "Historical" Means Here

Most blockchain state databases store only the **current** state. If you need to
know what value key K had at block 500 (and you are now at block 10,000), you have
two options, both expensive:

1. Maintain a full archive node that stores snapshots at every block. This requires
   enormous disk space.
2. Replay all transactions from block 500 to reconstruct the state. This requires
   the full transaction history and significant computation.

QMDB retains historical state roots: the Merkle tree root at each past block. A
**historical state proof** proves "at block N, key K had value V" by providing a
Merkle inclusion proof against block N's state root, which is committed in block N's
header.

### Concrete Roko Use Cases

**Knowledge entry existence proofs.** The neuro store (`roko-neuro`) tracks knowledge
entries locally in `.roko/neuro/knowledge.jsonl`. When entries are posted to the
InsightBoard contract on daeji, QMDB can prove that "knowledge entry with content
hash H existed at block N with confidence C, confirmation count 5, and kind
Heuristic." This is a ~500-byte Merkle proof against block N's state root.

Roko use case: an agent claims its approach was informed by a specific heuristic.
The QMDB proof shows that the heuristic was indeed in the InsightBoard at the block
before the agent's task transaction. This creates a cryptographic chain of evidence
from knowledge existence to agent decision to task outcome -- something that log
files alone cannot provide (logs can be tampered with; Merkle proofs cannot).

**Agent registration verification.** The AgentRegistry contract stores each agent's
public key, capabilities, heartbeat timestamp, and reputation stake. A QMDB proof
can show "at block N, agent X was registered with capability Y and had a reputation
score of Z." This enables retroactive auditing: after an incident, prove exactly what
capabilities an agent had at the time of the problematic action.

In roko, this connects to the `roko-runtime` lifecycle system: the `AgentState`
tracks each agent's lifecycle (Unvalidated -> ResourcesAllocated -> ToolsLoaded ->
Ready -> Running). QMDB proofs would add cryptographic timestamps to these transitions.

**Reputation score history.** Roko tracks agent performance across multiple
dimensions: per-model pass rates in the `CascadeRouter`, efficiency events (C-Factor
in `.roko/learn/efficiency.jsonl`), playbook success counts, and episode outcomes.
If reputation is derived from these metrics and stored on-chain, QMDB proves the
reputation at any historical block. This prevents retroactive reputation inflation
(changing historical scores to make an agent look better than it was).

**Where QMDB proofs touch roko's universal loop.** QMDB proofs are a **Store**
extension: they prove what was in the store at a given time. They also enable
**Verify** (proving that verification inputs -- knowledge entries, agent capabilities
-- existed when a decision was made).

**What roko code already exists that could use QMDB proofs.**
- `KnowledgeStore` in `roko-neuro`: entries posted to InsightBoard could have
  existence proofs
- `AgentRegistry.sol` in `contracts/`: agent identity and capabilities could be
  historically proven
- `ChainWitnessEngine` in `roko-chain`: episode hashes anchored to chain could
  reference QMDB proofs for the state they read
- `roko-learn` episode logger: episode records reference chain state that QMDB
  could prove

### Implementation Path

| Step | Work Required | Status |
|------|--------------|--------|
| QMDB as state backend | State storage and Merkleization | Exists in daeji |
| Historical proof RPC | RPC method accepting (block_number, key) and returning proof | Build |
| Client-side verification | Proof verification in agent framework (pure computation) | Build |

---

## 7. Agent Mesh via commonware-p2p

### Background: Ed25519 Digital Signatures

Ed25519 is a widely-used digital signature algorithm based on the Curve25519
elliptic curve. Each participant has:

- A **private key** (32 bytes): kept secret, used to sign messages. Signing proves
  the message came from the key holder.
- A **public key** (32 bytes): shared openly, used by others to verify signatures.
  Verification confirms the message was signed by the holder of the corresponding
  private key and has not been modified.

Ed25519 is chosen for networking applications because it is fast (signing and
verification take microseconds on modern hardware), produces compact signatures
(64 bytes), and is used extensively in production systems: SSH, TLS 1.3, WireGuard
VPN, and numerous blockchain protocols.

### Background: Authenticated P2P Networking

In an **unauthenticated** network (like raw TCP), anyone can connect to anyone, and
there is no built-in way to verify who you are talking to. Man-in-the-middle attacks
are possible.

In an **authenticated** peer-to-peer network, every participant is identified by
their Ed25519 public key. When two peers connect:

1. They perform a cryptographic **handshake**: each side proves they hold the private
   key corresponding to their claimed public key, typically by signing a challenge
   value.
2. The connection is **encrypted** using a session key derived from the handshake
   (similar to how TLS works).
3. Every subsequent message is authenticated -- the recipient can verify it came from
   the claimed sender and was not tampered with in transit.

No peer can impersonate another. Messages cannot be modified in transit. Peers can
selectively accept connections only from known, trusted public keys.

### Background: Hub-Spoke vs. Mesh Architectures

In a **hub-spoke** architecture, all communication flows through a central
coordinator (the "hub"). Agent A sends a message to the hub, the hub forwards it
to Agent B. The hub sees all messages and controls all routing.

- Benefits: Simple to implement, easy to monitor and log, straightforward access
  control.
- Drawbacks: The hub is a single point of failure (if it goes down, no agents can
  communicate), a throughput bottleneck (all traffic flows through one node), and
  adds latency (every message traverses two network hops instead of one).

In a **mesh** architecture, agents communicate directly with each other. Agent A
sends a message directly to Agent B, without any intermediary.

- Benefits: No single point of failure, lower latency (one hop instead of two),
  throughput scales with participant count (each pair communicates independently).
- Drawbacks: More complex connection management, harder to monitor centrally.

### Concrete Roko Use Cases

**Knowledge gossip between agents.** Today, roko agents share knowledge only through
the local neuro store -- each agent runs in its own process with its own copy of
`.roko/neuro/knowledge.jsonl`. There is no real-time knowledge sharing between agents
during plan execution. P2P gossip would allow agents working on related tasks to
share intermediate findings immediately, without waiting for the full distillation ->
chain post -> sync cycle.

Concrete example: Agent A is implementing a rate limiter and discovers that
`tokio::time::Interval` has a subtle behavior with `MissedTickBehavior`. It creates
a Warning-type knowledge entry. Via P2P gossip, Agent B (working on a related async
task in the same plan) receives this warning within milliseconds. Without gossip,
Agent B would only discover this after the plan completes, the distiller runs, and
the knowledge is ingested.

In roko's architecture, this connects to the `roko-neuro` store: gossipped entries
would be ingested as `source: "gossip"` with `Transient` tier and low initial
confidence (0.4), requiring local confirmation before climbing tiers. The neuro
store's existing anti-knowledge conflict detection (HDC similarity > 0.9 = reject)
would prevent gossipped entries from contradicting established local knowledge.

**Task coordination for parallel execution.** Roko's orchestrator dispatches tasks
from a plan DAG. Today, the `ProcessSupervisor` manages agents centrally -- it
tracks which agents are active, which tasks are assigned, and coordinates
completion. With a P2P mesh, agents could coordinate task handoffs directly: when
Agent A finishes task T1 that unblocks tasks T2 and T3, it broadcasts a completion
signal over the mesh. Agents waiting for T1 can pull T2 and T3 without routing
through the orchestrator.

This makes the system more resilient: if the orchestrator process crashes mid-plan,
agents with mesh connectivity can continue working on unblocked tasks based on the
last known plan state. The orchestrator's `--resume` flag
(`.roko/state/executor.json`) would reconstruct what happened from the mesh's message
history when it restarts.

**Fleet discovery.** Today, fleets are configured statically via `roko.toml`. With
P2P, agents could discover each other dynamically: each agent broadcasts its
capabilities (role, available models, current load) over the mesh. The
`CascadeRouter` could use this information for routing decisions: if another
operator's agent has a high pass rate on Rust async tasks (learned from gossipped
episode summaries), route the next async task to that agent.

**Where P2P mesh touches roko's universal loop.** P2P is primarily a **Store** and
**React** mechanism: agents store and retrieve knowledge through the mesh (gossip),
and agents react to mesh events (task completions, capability announcements). It also
touches **Route** (mesh-aware routing that considers remote agent capabilities).

**What roko code already exists that could use P2P mesh.**
- `roko-neuro` `KnowledgeStore`: knowledge ingestion path could accept gossipped
  entries
- `roko-runtime` `ProcessSupervisor`: agent lifecycle tracking could use mesh
  heartbeats
- `roko-runtime` `event_bus`: the typed broadcast channel could be backed by P2P
  for cross-process events
- `orchestrate.rs`: task dispatch could use mesh-based agent discovery
- `roko-learn` `provider_health`: provider health registry could aggregate health
  from mesh peers

### Implementation Path

| Step | Work Required | Status |
|------|--------------|--------|
| P2P library | `commonware-p2p` standalone crate | Exists in commonware |
| Agent key management | Ed25519 keypair per agent at creation time | Build |
| P2P manager per agent | Start manager, connect to known peers | Build |
| Message protocol | Define types: knowledge sharing, task handoff, heartbeat | Build |

This feature is independent of the daeji chain. It is a pure networking layer that
agents can use alongside (not instead of) on-chain interactions.

---

## 8. Ordered Broadcast (DSMR) for High-Throughput Agent Messaging

### Background: The Single-Leader Bottleneck

In most blockchain consensus protocols (PBFT, Tendermint/CometBFT, HotStuff, and
commonware's `threshold_simplex`), one validator is designated as the "leader" for
each round. The leader:

1. Collects pending transactions from the network.
2. Assembles them into a proposed block.
3. Broadcasts the proposed block to other validators.
4. Other validators verify and vote on the proposal.
5. If a supermajority votes to accept, the block is finalized.

The throughput of the entire chain is limited by a single node: the leader. Even
with 100 validators, only one is producing blocks at any given time. The others
wait until it is their turn to lead. This is the **single-leader bottleneck**.

### Background: How Ordered Broadcast (DSMR) Works

DSMR (Decoupled State Machine Replication) is an alternative architecture,
implemented in commonware's `broadcast` crate, that separates two concerns:

**Message dissemination** (getting data from producers to the network) is decoupled
from **message ordering** (agreeing on the sequence of messages for state machine
execution).

The process:

1. **Broadcast phase**: Multiple participants (called "sequencers") independently
   broadcast messages to the network. Each sequencer maintains its own append-only
   log. Each new message references the previous one (forming a per-sequencer chain).
   Validators acknowledge each message, producing a certificate of receipt.

2. **Ordering phase**: The consensus protocol (still one leader per round) does NOT
   include the messages themselves in its proposal. Instead, it references the latest
   certified message from each sequencer (the "tip"). The consensus proposal is just
   a set of tip references -- a small, fixed-size data structure regardless of how
   many messages were broadcast.

3. **Execution phase**: Once consensus finalizes the tip references, all validators
   process all messages from all sequencers up to the referenced tips, in a
   deterministic order derived from the tip references.

The throughput gain: in the single-leader model, one node is the bottleneck. In
DSMR, every sequencer contributes data in parallel. If 100 sequencers each broadcast
10 messages per second, the chain processes 1,000 messages per second. The consensus
leader handles only the small tip-reference block, not the messages themselves.

### Concrete Roko Use Cases

**Agent-as-sequencer for knowledge proposals.** In roko's architecture, each active
agent discovers knowledge during task execution: insights about the codebase,
heuristics for tool usage, warnings about API limitations. Today, these are collected
after task completion by the `Distiller` (which batches episodes and extracts knowledge
candidates via Claude Haiku). With DSMR, each agent runs as its own broadcast
sequencer, continuously proposing knowledge entries as it discovers them.

The flow: Agent A discovers a heuristic -> broadcasts it to its sequencer channel ->
validators acknowledge -> at the next consensus round, the leader includes Agent A's
tip reference -> all validators process the knowledge entry -> it appears in the
InsightBoard contract state.

This eliminates the batch-and-distill delay: knowledge enters the chain in near
real-time (within one consensus round, ~400ms) rather than after the full
distillation pipeline (which runs post-task and requires an LLM call).

In roko's universal loop, DSMR primarily touches **Store** (knowledge entries are
stored as sequencer messages) and **React** (other agents react to newly proposed
knowledge in real-time via their own sequencer subscriptions).

**Reputation update proposals.** Each agent continuously publishes its own
performance metrics as sequencer messages: gate pass rates, token efficiency,
C-Factor scores (computed in `roko-learn/src/cfactor.rs`), model usage statistics
from the CascadeRouter. Validators aggregate these into reputation scores at
finalization boundaries. This creates a streaming reputation system rather than
periodic snapshots.

**Task claim proposals.** When a plan has multiple ready tasks and multiple agents,
each agent proposes its task claims through its sequencer channel. The consensus
ordering determines which agent claimed which task first -- a provably fair
ordering that no single agent can manipulate.

In roko's orchestrator, task claims are currently sequential (the orchestrator
assigns tasks one at a time). DSMR would enable concurrent claiming with
deterministic ordering: multiple agents broadcast claims simultaneously, and the
consensus-ordered sequence determines assignment priority.

**Streaming telemetry.** Roko produces rich per-turn telemetry: `AgentEfficiencyEvent`
records (token counts, cost estimates, prompt section attributions, tool call
metadata) written to `.roko/learn/efficiency.jsonl`, episode records in
`.roko/episodes.jsonl`, and routing decisions logged by `RoutingLogger` in
`roko-learn/src/routing_log.rs`. With DSMR, agents broadcast this telemetry as
sequencer messages. The chain captures snapshots at finalization boundaries.
Real-time observability (agents broadcast immediately, observers subscribe) with
eventual on-chain consistency (finalized periodically).

**Where DSMR touches roko's universal loop.** DSMR is a **Store** mechanism (data
enters the chain via broadcast) and a **React** mechanism (agents subscribe to each
other's broadcasts for real-time coordination). It also enables a new form of
**Route**: the consensus ordering of sequencer tips provides a canonical,
tamper-proof ordering for competing proposals (task claims, knowledge entries,
reputation updates).

**What roko code already exists that could use DSMR.**
- `roko-neuro` `Distiller`: knowledge extraction currently batch -- DSMR enables
  streaming
- `roko-learn` `efficiency.rs`: efficiency events currently file-based -- could
  be sequencer messages
- `roko-learn` `episode_logger.rs`: episodes could be broadcast as sequencer
  messages
- `orchestrate.rs`: task dispatch ordering could use DSMR consensus ordering
- `roko-runtime` `event_bus`: the in-process broadcast channel could be backed
  by DSMR for cross-process, cross-machine event dissemination

### Implementation Path

| Step | Work Required | Status |
|------|--------------|--------|
| Broadcast crate | `commonware-broadcast` with ordered broadcast | Exists in commonware |
| Daeji integration | Replace or augment block-building with DSMR architecture | Major change |
| Agent sequencer runtime | Per-agent sequencer process, broadcasting actions | Build |

This is a longer-term feature requiring significant architectural changes to daeji's
consensus integration. It should be planned for but not built until the foundational
features (VRF, certificates, simulation) are stable and proven.

---

## Priority Matrix

| # | Feature | Novelty | Feasibility | Phase | Roko Integration Points | Rationale |
|---|---------|---------|-------------|-------|------------------------|-----------|
| 2 | On-chain VRF | Medium | High | **1** | CascadeRouter (model selection), ExperimentStore (variant assignment), gate thresholds (jittering), orchestrate.rs (task ordering) | Already produced by consensus. Requires only reading an existing block field. Zero chain changes. |
| 4 | Deterministic simulation | High | High | **1** | roko-gate (simulation gate rung), roko-chain (witness testing), orchestrate.rs (plan regression tests), forensic_replay (deterministic debugging) | E2E test harness exists. Needs library exposure and gate integration. |
| 1 | BTLE commitments | Very High | Medium | **2** | CascadeRouter (sealed-bid model routing), orchestrate.rs (fair task claims), roko-neuro (time-delayed knowledge reveals), ExperimentStore (independent verification) | Cryptographic primitives exist. Needs encryption library, smart contract, and orchestration wiring. |
| 3 | Cross-chain certificates | Very High | Medium | **2** | ChainWitnessEngine (episode hash attestation), roko-gate (verdict certification), roko-neuro (knowledge provenance), AgentRegistry (portable reputation) | Certificates already produced. Needs export API, verifier contract on target chain, and relay. |
| 6 | QMDB historical proofs | High | Medium | **2** | KnowledgeStore (entry existence proofs), AgentRegistry (capability history), ChainWitnessEngine (state context for episodes) | QMDB is the state backend. Needs RPC method for proof requests and client-side verification code. |
| 7 | Agent P2P mesh | Medium | Medium | **2** | roko-neuro (knowledge gossip), ProcessSupervisor (mesh heartbeats), event_bus (cross-process events), CascadeRouter (mesh-aware routing) | Standalone crate exists. Needs key management, message types, and agent lifecycle integration. |
| 5 | Resharing | High | Low | **3** | ProcessSupervisor (lifecycle-triggered resharing), AlloyChainClient (post-reshare reconnection), all certificate-dependent features | Cryptographic protocol exists. Daeji integration not yet built. |
| 8 | DSMR / ordered broadcast | Very High | Low | **3** | Distiller (streaming knowledge), episode_logger (broadcast episodes), efficiency events (streaming telemetry), orchestrate.rs (consensus-ordered task claims) | Requires major consensus architecture changes. Broadcast crate exists but chain integration is substantial. |

**Phase 1** features require no chain modifications and can be adopted immediately.
**Phase 2** features require new components (contracts, RPC methods, libraries) but
build on existing infrastructure. **Phase 3** features require architectural changes
to consensus or validator management.
