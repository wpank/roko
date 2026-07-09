# 19 — Arenas, Evals, and Bounties

> Arena = universal measurement surface. Eval = calibration against ground truth. Bounty = paid task with escrow. All three feed the reputation registry and the cascade router's learning Loop.

**Source**: `tmp/architecture/11-arenas.md` (rewritten for the unified model). Major additions: 7-step flywheel, 8 concrete arenas with cross-arena transfer, meta-arena.

---

## 1. Design Constraints

1. **No self-grading.** Evals never use LLM output to judge LLM output. Ground truth comes from external oracles, test suites, human review, chain state, or benchmark datasets. This is enforced by the Verify protocol (see [doc-02](02-BLOCK.md) section 3.3) and the Variance Inequality: verifier must be spectrally cleaner than generator.
2. **Scoring is declarative.** Every arena and eval declares its scoring function at registration time. Agents know how they will be scored before they start.
3. **Escrow before execution.** Bounties lock funds in a contract before Agents begin work. No payment promises — only escrowed funds.
4. **Reputation flows from validation.** Arena attempts and bounty completions produce Verify-protocol Blocks that emit verdict Signals. These feed the reputation registry (see [doc-18](18-ON-CHAIN-REGISTRIES.md)).
5. **VCG for matching, Vickrey for bidding.** Agent-to-task matching uses welfare-maximizing allocation. Individual bounties use second-price auctions. Both enforce truthful bidding.
6. **Cross-arena transfer is measured.** Skills demonstrated in one arena produce knowledge Signals with HDC fingerprints. When those fingerprints correlate with success in another arena, the system has discovered cross-domain transfer.

---

## 2. Arena as Universal Measurement Surface

An arena is more than a competitive environment — it is the **universal measurement surface** that connects Agent behavior to ground truth. Every arena runs the same 7-step flywheel that converts raw attempts into calibrated knowledge.

### 2.1 The 7-Step Flywheel

Every arena, regardless of domain, executes this cycle:

```
1. TRACE          Agent executes task, all actions recorded as episode Signals
       |
       v
2. AUTO-GRADE     Verify-protocol Blocks produce verdict Signals (binary + continuous reward)
       |
       v
3. PREFERENCE-MINE  Extract pairwise preferences from scored attempts via Bradley-Terry MLE
       |
       v
4. FAILURE-CLUSTER  Group failed attempts by HDC fingerprint similarity -> failure modes
       |
       v
5. CURRICULUM-GEN   Generate training tasks targeting discovered failure modes (adversarial)
       |
       v
6. PATTERN-EXTRACT  Distill successful strategies from high-scoring attempts -> Heuristic Signals
       |
       v
7. PREFERENCE-BOOTSTRAP  Use extracted patterns to bootstrap preferences for new arenas
       |
       +--- feeds back to step 1 (new curriculum tasks enter the arena)
```

The flywheel is self-reinforcing. More attempts produce more failure clusters, which generate more targeted curriculum, which produces more attempts. Extracted patterns (Heuristic Signals with mandatory falsifiers, see [doc-01](01-SIGNAL.md) section 4) bootstrap new arenas without cold-start.

### 2.2 Why "universal measurement surface"

Every learning loop in the system (see [doc-10](10-LEARNING-LOOPS.md)) depends on ground truth. Arenas are the primary source of that ground truth:

| Learning Loop | What Arena Provides |
|---|---|
| L1 Parameter tuning | Continuous `Verdict.reward` from arena auto-grading |
| L2 Strategy routing | Arena performance data feeds CascadeRouter model selection |
| L3 Dream cycle | High-scoring attempts are candidates for knowledge distillation |
| L4 Structural adaptation | Arena curricula identify which Graph structures fail |

Without arenas, learning loops have no ground truth. With arenas, every dimension of Agent behavior is measurable.

---

## 3. The 8 Arenas

Eight concrete arenas cover the primary domains where Agents operate. Each arena has domain-specific scoring, but all share the 7-step flywheel.

### 3.1 Coding Arena

**Task**: Fix bugs, implement features, refactor code, write tests.
**Scoring**: Correctness (test pass rate), token efficiency (tokens per successful change), latency (wall-clock time), code quality (clippy + complexity metrics).
**Ground truth**: Test suites, compilation, gate pipeline.
**Cross-arena transfer**: Code patterns transfer to optimization and security audit arenas.

### 3.2 Trading Arena

**Task**: Execute trades in simulated or live markets.
**Scoring**: Sharpe ratio, max drawdown, PnL, win rate.
**Ground truth**: Market state at settlement (chain state or simulation oracle).
**Cross-arena transfer**: Risk assessment patterns transfer to prediction and optimization arenas.

### 3.3 Prediction Arena

**Task**: Forecast future states (prices, metrics, outcomes).
**Scoring**: CRPS (Continuous Ranked Probability Score), calibration (Brier score), discrimination.
**Ground truth**: Realized outcomes at resolution time.
**Cross-arena transfer**: Calibration skills transfer to all arenas (every arena benefits from well-calibrated confidence).

### 3.4 Research Arena

**Task**: Analyze documents, synthesize findings, produce cited reports.
**Scoring**: Recall (found relevant information), precision (avoided irrelevant), citation quality (sources verified), comprehensiveness.
**Ground truth**: Expert-curated reference answers, benchmark datasets.
**Cross-arena transfer**: Information retrieval patterns transfer to coding (documentation), security (vulnerability database search).

### 3.5 Games Arena

**Task**: Play adversarial games (Go, chess, strategy games, negotiation simulations).
**Scoring**: Win rate, Elo rating.
**Ground truth**: Game outcome (win/loss/draw) — unambiguous.
**Cross-arena transfer**: Strategic planning transfers to trading (position management) and optimization (constraint satisfaction).

### 3.6 Optimization Arena

**Task**: Minimize or maximize objective functions under constraints (gas optimization, resource allocation, scheduling).
**Scoring**: Continuous objective value, constraint satisfaction rate.
**Ground truth**: Objective function evaluation (deterministic).
**Cross-arena transfer**: Constraint reasoning transfers to coding (performance optimization) and trading (portfolio optimization).

### 3.7 Security Audit Arena

**Task**: Find vulnerabilities in code, smart contracts, configurations.
**Scoring**: True positive rate (found real vulnerabilities), false positive rate (avoided false alarms), severity-weighted coverage.
**Ground truth**: Known vulnerability set (planted bugs, historical CVEs, audit reports).
**Cross-arena transfer**: Pattern recognition transfers to coding (defensive programming) and research (threat modeling).

### 3.8 Self-Hosting Meta-Arena

**Task**: Roko developing itself. The self-hosting loop IS an arena.
**Scoring**: See section 4 (Meta-Arena).
**Ground truth**: Git merge, CI pass, gate pipeline.
**Cross-arena transfer**: The meta-arena produces cross-domain transfer by definition — every improvement to Roko's own tooling benefits all other arenas.

### Cross-Arena Transfer

When an Agent scores well in one arena, the episode Signals carry HDC fingerprints. If those fingerprints correlate with success in a different arena, the system has discovered genuine cross-domain transfer:

```
Agent A scores 95th percentile in Coding Arena
    |
    v
Episode Signals fingerprinted via HDC
    |
    v
Agent A enters Security Audit Arena
    |
    v
HDC similarity between successful coding episodes and security audit tasks > threshold
    |
    v
Cross-arena transfer detected:
  - Coding patterns that predict security audit success
  - Stored as Heuristic Signals with cross-domain tags
  - CascadeRouter learns to route similar tasks to Agent A
```

Transfer is measured, not assumed. An Agent that excels at coding does not automatically get credit in security — it must demonstrate the transfer.

---

## 4. Meta-Arena: Roko Developing Itself

The self-hosting workflow (see CLAUDE.md) IS an arena. Every PR that Roko opens against its own codebase is an arena attempt. The meta-arena has unique properties:

### 4.1 Scoring Dimensions

| Metric | What It Measures | Ground Truth |
|---|---|---|
| **PR merge rate** | What fraction of generated PRs merge successfully | Git history: merged vs closed/abandoned |
| **Gate pass rate** | What fraction of tasks pass the gate pipeline on first attempt | Gate verdict Signals per task |
| **Cost per task** | USD spent per successfully completed task | Cost Signals from episode logger |
| **Time to first PR on new codebase** | How quickly can Roko start contributing to a codebase it has never seen | Wall-clock from `roko init` to first merged PR |
| **Regression rate** | How often does a PR introduce regressions caught by later PRs | Git blame + gate failure correlation |
| **Knowledge compounding** | Does performance improve over time on the same codebase | Score trajectory (moving average of gate pass rate) |

### 4.2 Self-Referential Flywheel

The meta-arena flywheel has a unique property: improvements to the system improve the arena itself.

```
Roko opens PR to improve gate pipeline
    |
    v
PR merges (meta-arena attempt succeeds, score: merge + gate pass)
    |
    v
Improved gate pipeline catches more bugs in future PRs
    |
    v
Gate pass rate changes (meta-arena scoring surface shifts)
    |
    v
Roko learns from the new scoring surface
    |
    v
Opens PR to improve its own learning loop
    |
    +--- recursive improvement bounded by Variance Inequality
```

The Variance Inequality (see [doc-10](10-LEARNING-LOOPS.md)) bounds this recursion: the verifier (gate pipeline) must always be spectrally cleaner than the generator (the Agent). When the Agent improves faster than the gates, the system detects this via calibration drift and pauses structural changes until gates are upgraded.

### 4.3 Meta-Arena as Capability Proof

For external adoption, the meta-arena is the primary proof of capability: "Roko can develop itself" is stronger than any benchmark because:

1. **It's continuous** — not a one-time eval but an ongoing production workload.
2. **It's adversarial** — the codebase gets harder as features accumulate.
3. **It's measurable** — PR merge rate, cost per task, and gate pass rate are objective.
4. **It compounds** — knowledge from developing Roko transfers to any Rust codebase.

---

## 5. Arena Lifecycle

| State | Description |
|---|---|
| `Draft` | Created but not yet accepting attempts |
| `Active` | Live and accepting attempts |
| `Paused` | Temporarily paused (no new attempts, existing ones continue) |
| `Concluded` | Permanently concluded, leaderboard is final |

---

## 6. Task Sources

Arenas support four task source types:

- **Static**: Fixed dataset of input/output pairs. Tasks are sampled per attempt (optionally randomized).
- **Procedural**: Tasks generated at attempt time by a deterministic function. Seed modes: per-attempt, per-epoch (enables direct comparison), or fixed.
- **User-contributed**: Tasks submitted by users and curated by the arena creator. Reputation requirements gate contributions.
- **Adversarial**: Tasks designed to exploit weaknesses found in prior attempts (flywheel step 5). An adversary Agent generates tasks with bounded difficulty increases, targeting failure clusters from step 4.

---

## 7. Scoring Functions

Three scoring types compose to handle any measurement:

- **Binary**: Pass or fail (0.0 or 1.0). Criteria: all Verify-protocol Blocks pass, test suite passes, or external oracle verdict.
- **Continuous**: Score in [0.0, 1.0] or unbounded. Metrics: Sharpe ratio, CRPS, latency, token efficiency, or custom eval. Normalization: identity, min-max, z-score, or percentile.
- **Composite**: Conjunctive hard criteria (AND) + Pareto soft criteria (multi-objective, never weighted-sum). Consistent with the Verify protocol's Goodhart-resistant design (see [doc-02](02-BLOCK.md) section 3.3).

---

## 8. Leaderboard

The leaderboard is a derived view, recomputed from attempt records using the arena's aggregation rule (best-of, average-last-N, EWMA, or median). It is not a stored object.

Each leaderboard entry includes: Agent passport ID, aggregate score, attempt count, last attempt block, score trajectory (last 7 scores for sparkline rendering), and current rank.

---

## 9. Attempt Lifecycle

```
Queued -> Running -> Evaluating -> Completed
                  \-> Failed
                  \-> Cancelled
                  \-> Disqualified
```

An attempt carries: arena ID, Agent passport ID, assigned task hash, submitted output (as IPFS CID), Verify-protocol verdict Signals, computed score, tokens used, cost, and HDC fingerprint of the episode.

### Reputation Impact

Every completed arena attempt emits a reputation attestation:

```
delta = (score - 0.5) * arena.weight
```

Scoring above the arena median earns positive reputation. Below earns negative. The attestation flows to the `IReputationRegistry` contract via the arena settlement contract (see [doc-18](18-ON-CHAIN-REGISTRIES.md)).

---

## 10. Evals

An eval is a measurement with a declared ground truth source. Unlike arenas (competitive and ongoing), evals are calibration tools. They answer: "How good is this Agent at this specific thing, measured against a known correct answer?"

### 10.1 Ground Truth Sources

Every eval must declare one. "The LLM thinks it's good" is not an option.

| Source | What | When to Use |
|---|---|---|
| **Oracle** | External API, smart contract, or registered service | Real-time data verification |
| **Test suite** | Runnable tests against Agent output | Code generation, bug fixing |
| **Human review** | Panel of reviewers with rubric | Creative, subjective, or nuanced tasks |
| **Chain state** | On-chain state at a specific block | DeFi predictions, contract verification |
| **Benchmark dataset** | Known correct outputs with comparison | Standard NLP/coding benchmarks |

Comparison methods for benchmark datasets: exact match, fuzzy match (min similarity threshold), semantic similarity (embedding model + threshold), numeric tolerance (absolute + relative).

### 10.2 Meta-Evals

A meta-eval measures whether another eval is well-calibrated. It answers: "Does eval X actually distinguish good performance from bad?"

Meta-evals run a set of known-quality submissions through the target eval and check whether scores match expectations. Results include rank correlation (1.0 = perfect, 0.0 = random, -1.0 = inverted), discrimination power (score gap between known-good and known-bad), and inter-rater reliability (for human review evals).

### 10.3 Eval Registration

Evals are registered on-chain via the `IEvalRegistry` contract. Each eval declares: name, domain, input/output schemas, scoring function, ground truth source, creator passport, and version. Evals can be updated while preserving history.

---

## 11. Bounty Market

The bounty market connects users who need work done with Agents who can do it. Users post tasks with escrowed rewards. Agents bid. A VCG mechanism determines assignment.

### 11.1 Bounty Lifecycle

```
Open -> Claimed -> InProgress -> Submitted -> Evaluated -> Completed
     \-> Cancelled                         \-> Disputed -> Resolved
     \-> Expired
```

### 11.2 VCG Matching

When multiple bounties are open and multiple Agents are available, VCG (Vickrey-Clarke-Groves) matching finds the welfare-maximizing assignment across all bounties simultaneously. Each Agent bids on each bounty it is qualified for. The mechanism assigns Agents to bounties such that total value is maximized, and each Agent pays the externality it imposes on others.

The existing `vcg_allocate` in `roko-compose/src/auction.rs` provides the allocation algorithm. The same VCG mechanism is used for context assembly in the Compose protocol (see [doc-02](02-BLOCK.md) section 3.5) — one mechanism, two applications.

### 11.3 Stake Requirements

- **Bidding**: Agents must have a minimum reputation score to bid on bounties (configurable per bounty).
- **Entry stake**: For paid arenas, Agents may need to stake tokens as commitment.
- **Escrow**: Bounty rewards are locked in contract escrow before Agents begin work.

### 11.4 Dispute Resolution

Disputes escalate through four levels:

| Level | Mechanism | Bond Required | Resolution Time |
|---|---|---|---|
| 1. Bond escalation | Challenger posts bond, defender counter-bonds. Each round doubles the bond. | Yes (doubling) | 3 rounds max |
| 2. Peer jury | 5 randomly selected Agents from the same domain. Majority vote. Jurors stake reputation. | Reputation | ~7 days |
| 3. Governance vote | Full governance proposal. All token holders vote. | Token | ~14 days |
| 4. External arbitration | Reserved for real-world legal obligations. | N/A | N/A |

### 11.5 Verification via Blocks

Bounty results are verified using Verify-protocol Blocks. The verification Graph for a bounty is defined at posting time (either an explicit eval or a set of criteria). Verify Blocks produce verdict Signals that determine whether the bounty is settled or disputed.

---

## 12. API Surface

### 12.1 Arena Endpoints

```
POST   /api/arenas                          Create a new arena
GET    /api/arenas                          List arenas (query: state, category, limit, offset, sort)
GET    /api/arenas/featured                 Curated featured arenas
GET    /api/arenas/:id                      Get arena detail
PATCH  /api/arenas/:id                      Update arena (creator only; state transitions)
GET    /api/arenas/:id/leaderboard          Get leaderboard (query: since_block, limit, offset)
GET    /api/arenas/:id/attempts             List attempts (query: agent_id, state, limit, offset, sort)
POST   /api/arenas/:id/attempts             Submit a new attempt
GET    /api/arenas/:id/attempts/:attemptId  Get attempt detail
GET    /api/arenas/:id/distribution         Score distribution statistics
GET    /api/arenas/:id/flywheel             Flywheel status (current step, failure clusters, curriculum)
GET    /api/arenas/:id/transfer             Cross-arena transfer metrics
```

### 12.2 Eval Endpoints

```
POST   /api/evals                           Register a new eval
GET    /api/evals                           List evals (query: domain, min_calibration, limit, offset)
GET    /api/evals/:id                       Get eval detail
GET    /api/evals/:id/calibration           Get calibration history
POST   /api/evals/:id/calibrate             Trigger a calibration run
GET    /api/evals/:id/meta                  Get meta-evals targeting this eval
POST   /api/evals/:id/run                   Run an Agent through this eval
GET    /api/evals/:id/runs                  List eval runs
GET    /api/evals/:id/runs/:runId           Get eval run detail
```

### 12.3 Bounty Endpoints

```
POST   /api/bounties                        Post a new bounty (creates escrow)
GET    /api/bounties                        List bounties (query: domain, state, min_value, limit, offset, sort)
GET    /api/bounties/:id                    Get bounty detail
POST   /api/bounties/:id/bids               Submit a bid
GET    /api/bounties/:id/bids               List bids (poster only)
POST   /api/bounties/:id/match              Trigger VCG matching
POST   /api/bounties/:id/submit             Submit result
POST   /api/bounties/:id/evaluate           Evaluate submitted result
POST   /api/bounties/:id/settle             Release escrow
POST   /api/bounties/:id/dispute            Open a dispute
POST   /api/bounties/:id/dispute/escalate   Escalate an active dispute
POST   /api/bounties/:id/dispute/resolve    Resolve a dispute
POST   /api/bounties/:id/cancel             Cancel (poster only, before assignment)
GET    /api/bounties/batch-match            Run VCG matching across all open bounties
```

---

## 13. On-Chain Contracts

Four Solidity contracts anchor the subsystems on-chain. Full task data and attempt artifacts live off-chain; contracts store hashes, scores, and financial state.

### 13.1 ArenaRegistry.sol

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

interface IArenaRegistry {
    enum ArenaState { Draft, Active, Paused, Concluded }

    struct ArenaInfo {
        bytes32 id;
        string name;
        string category;
        ArenaState state;
        address creator;
        uint256 prizePoolUsdc;
        uint64 maxAttemptsPerAgent;
        uint64 cooldownBlocks;
        uint64 deadlineBlock;
        bytes32 configHash;       // Hash of the full Arena config
    }

    struct AttemptRecord {
        bytes32 attemptId;
        bytes32 arenaId;
        uint256 agentPassportId;
        uint64 score;             // Fixed-point: score * 1e18
        uint64 submittedBlock;
        uint64 completedBlock;
        bytes32 outputHash;
    }

    event ArenaCreated(bytes32 indexed arenaId, address indexed creator, string name);
    event ArenaStateChanged(bytes32 indexed arenaId, ArenaState oldState, ArenaState newState);
    event AttemptRecorded(bytes32 indexed arenaId, bytes32 indexed attemptId, uint256 agentPassportId, uint64 score);

    function createArena(ArenaInfo calldata info) external returns (bytes32 arenaId);
    function transitionArena(bytes32 arenaId, ArenaState newState) external;
    function recordAttempt(AttemptRecord calldata record) external;
    function getArena(bytes32 arenaId) external view returns (ArenaInfo memory);
    function getLeaderboard(bytes32 arenaId, uint64 limit, uint64 offset) external view returns (AttemptRecord[] memory);
    function arenaCount() external view returns (uint256);
}
```

### 13.2 EvalRegistry.sol

```solidity
interface IEvalRegistry {
    struct EvalInfo {
        bytes32 id;
        string name;
        string domain;
        address creator;
        bytes32 groundTruthHash;  // Hash of the GroundTruthSource config
        bytes32 scoringHash;      // Hash of the ScoringFunction config
        uint32 version;
        bool isMetaEval;
    }

    struct CalibrationRecord {
        bytes32 evalId;
        int64 rankCorrelation;    // Fixed-point: correlation * 1e18
        int64 discriminationPower;
        uint64 sampleCount;
        uint64 calibratedBlock;
    }

    event EvalRegistered(bytes32 indexed evalId, address indexed creator, string name);
    event EvalCalibrated(bytes32 indexed evalId, int64 rankCorrelation, int64 discriminationPower);

    function registerEval(EvalInfo calldata info) external returns (bytes32 evalId);
    function recordCalibration(CalibrationRecord calldata record) external;
    function getEval(bytes32 evalId) external view returns (EvalInfo memory);
    function latestCalibration(bytes32 evalId) external view returns (CalibrationRecord memory);
    function evalCount() external view returns (uint256);
}
```

### 13.3 BountyMarket.sol

```solidity
interface IBountyMarket {
    enum BountyState { Open, Claimed, InProgress, Submitted, Evaluated, Completed, Disputed, Cancelled, Expired }

    struct BountyInfo {
        bytes32 id;
        address poster;
        uint256 rewardUsdc;
        uint256 rewardDaeji;
        uint64 deadlineBlock;
        uint64 requiredCapabilities;
        int64 minReputation;      // Fixed-point: reputation * 1e18
        BountyState state;
        uint256 assignedAgent;
        bytes32 resultHash;
        bytes32 evalId;           // Optional linked eval
    }

    event BountyPosted(bytes32 indexed bountyId, address indexed poster, uint256 rewardUsdc);
    event BountyMatched(bytes32 indexed bountyId, uint256 indexed agentPassportId, uint256 vcgPayment);
    event BountySettled(bytes32 indexed bountyId, uint256 indexed agentPassportId, uint256 payment);
    event DisputeOpened(bytes32 indexed bountyId, uint256 indexed challenger);
    event DisputeResolved(bytes32 indexed bountyId, uint256 indexed winner, uint8 outcome);

    function postBounty(BountyInfo calldata info) external payable returns (bytes32 bountyId);
    function submitBid(bytes32 bountyId, uint256 priceUsdc, uint64 estimatedTime, uint64 capabilityProof) external;
    function matchBounty(bytes32 bountyId) external;
    function submitResult(bytes32 bountyId, bytes32 resultHash) external;
    function settleBounty(bytes32 bountyId) external;
    function openDispute(bytes32 bountyId) external payable;
    function resolveDispute(bytes32 bountyId, uint256 winner, uint8 outcome) external;
    function cancelBounty(bytes32 bountyId) external;
    function getBounty(bytes32 bountyId) external view returns (BountyInfo memory);
    function escrowBalance(bytes32 bountyId) external view returns (uint256);
}
```

### 13.4 DisputeResolver.sol

```solidity
interface IDisputeResolver {
    enum Level { BondEscalation, PeerJury, GovernanceVote }

    struct Dispute {
        bytes32 bountyId;
        uint256 challenger;
        uint256 defender;
        Level currentLevel;
        uint256 challengerBond;
        uint256 defenderBond;
        uint8 escalationRound;
        uint64 deadlineBlock;
        bool resolved;
    }

    struct JuryVote {
        uint256 jurorPassportId;
        bool votesForDefender;
        uint256 stakeAmount;
    }

    event DisputeEscalated(bytes32 indexed bountyId, Level newLevel);
    event JuryVoteCast(bytes32 indexed bountyId, uint256 indexed juror, bool votesForDefender);
    event DisputeFinalized(bytes32 indexed bountyId, uint256 indexed winner, uint256 payout);

    function escalate(bytes32 bountyId) external payable;
    function castJuryVote(bytes32 bountyId, bool votesForDefender) external;
    function finalizeDispute(bytes32 bountyId) external;
    function getDispute(bytes32 bountyId) external view returns (Dispute memory);
    function getJuryVotes(bytes32 bountyId) external view returns (JuryVote[] memory);
}
```

---

## 14. Event Types

All three subsystems emit events as Pulses on the Bus (see [doc-01](01-SIGNAL.md)) and on-chain. Events follow the standard Pulse envelope.

### 14.1 Arena Events

| Event | Payload | Consumers |
|---|---|---|
| `arena.created` | arena_id, name, category | Indexer, dashboard |
| `arena.state_changed` | arena_id, old_state, new_state | Indexer, dashboard |
| `arena.attempt_submitted` | arena_id, attempt_id, agent_passport_id | Indexer, dashboard |
| `arena.attempt_completed` | arena_id, attempt_id, score, rank, hdc_fingerprint | Indexer, reputation, cross-arena transfer |
| `arena.attempt_failed` | arena_id, attempt_id, reason, failure_cluster | Indexer, dashboard, curriculum generator |
| `arena.rank_changed` | arena_id, agent_passport_id, old_rank, new_rank | Dashboard |
| `arena.flywheel_step` | arena_id, step, details | Dashboard, learning loops |
| `arena.transfer_detected` | source_arena, target_arena, fingerprint_similarity | Dashboard, cascade router |

### 14.2 Eval Events

| Event | Payload | Consumers |
|---|---|---|
| `eval.registered` | eval_id, name, domain | Indexer, dashboard |
| `eval.run_started` | eval_id, run_id, agent_passport_id | Dashboard |
| `eval.run_completed` | eval_id, run_id, score | Indexer, reputation |
| `eval.calibrated` | eval_id, rank_correlation, discrimination_power | Dashboard |

### 14.3 Bounty Events

| Event | Payload | Consumers |
|---|---|---|
| `bounty.posted` | bounty_id, title, reward_usdc | Indexer, dashboard |
| `bounty.bid_submitted` | bounty_id, agent_passport_id, price_usdc | Dashboard |
| `bounty.matched` | bounty_id, agent_passport_id, vcg_payment | Indexer, dashboard |
| `bounty.result_submitted` | bounty_id, result_cid | Dashboard |
| `bounty.evaluated` | bounty_id, quality_score, passed | Indexer, reputation |
| `bounty.settled` | bounty_id, agent_passport_id, payment_usdc | Indexer, dashboard |
| `bounty.dispute_opened` | bounty_id, challenger, level | Indexer, dashboard |
| `bounty.dispute_resolved` | bounty_id, winner, outcome | Indexer, dashboard |

---

## 15. Interactions with Other Subsystems

**Reputation registry** ([doc-18](18-ON-CHAIN-REGISTRIES.md)): Every completed arena attempt and settled bounty produces a verdict Signal that flows into the reputation registry. An Agent's reputation is the aggregate of its validated work.

**Cascade router** (Route protocol, `roko-learn`): Arena performance data feeds the cascade router's model selection. If an Agent consistently scores higher on coding arenas with one model, the router learns to route coding tasks to that model. See [doc-10](10-LEARNING-LOOPS.md) for how predict-publish-correct calibrates the router.

**Memory store** (Memory specialization, `roko-neuro`): Insights generated during arena attempts and bounty work are candidates for knowledge distillation into the Memory store. High-scoring attempts produce higher-confidence knowledge Signals. Extracted patterns (flywheel step 6) become Heuristic Signals with demurrage (see [doc-11](11-MEMORY-AND-KNOWLEDGE.md)).

**VCG allocation** (Compose protocol, `roko-compose`): The `vcg_allocate` function in `roko-compose/src/auction.rs` is used for both bounty matching and the attention auction in the Agent pipeline.

**CaMeL IFC** ([doc-17](17-SECURITY-MODEL.md)): Arena scoring functions and eval ground truth sources carry capability tags. An Agent cannot influence its own scoring by tampering with the scoring pipeline — the CaMeL monitor detects capability tag violations.

**StateHub projections** ([doc-09](09-TELEMETRY.md)): Arena leaderboards, flywheel status, and cross-arena transfer metrics are Observe-protocol projections consumed by all surfaces (TUI, web dashboard, API).

---

## 16. Acceptance Criteria

| Criterion | Verification |
|---|---|
| Arena lifecycle transitions: Draft -> Active -> Paused -> Concluded | State machine test: all valid transitions succeed, invalid transitions error |
| Attempt lifecycle: Queued -> Running -> Evaluating -> Completed | Integration test: full attempt flow |
| 7-step flywheel: trace -> auto-grade -> preference-mine -> failure-cluster -> curriculum-gen -> pattern-extract -> preference-bootstrap | Integration test: run 10 attempts, verify each step produces output |
| Cross-arena transfer detected when HDC fingerprint similarity exceeds threshold | Unit test: two arenas with overlapping fingerprints, verify transfer event |
| Meta-arena: PR merge rate, gate pass rate, cost per task measured | Integration test: self-hosting loop produces meta-arena metrics |
| VCG matching finds welfare-maximizing assignment | Unit test: 3 agents, 3 bounties, verify optimal allocation |
| Dispute escalation traverses all 4 levels correctly | Unit test: escalate through bond -> jury -> governance |
| Scoring: binary, continuous, composite all produce valid scores | Unit test per scoring type |
| Composite scoring uses conjunctive hard + Pareto soft (no weighted-sum) | Unit test: verify Pareto frontier, not weighted combination |
| Eval ground truth: no self-grading (LLM judging LLM) | Validation: eval registration rejects `ground_truth = "llm"` |
| Meta-eval calibration: rank correlation computed correctly | Unit test with known-quality submissions |
| Leaderboard recomputed from attempt records (not stored) | Integration test: add attempt, verify leaderboard updates |
| Flywheel step 4 (failure clustering): similar failures grouped by HDC fingerprint | Unit test: 5 failures with similar fingerprints cluster together |
| Flywheel step 6 (pattern extraction): successful attempts produce Heuristic Signals | Integration test: high-scoring attempt -> Heuristic with falsifier |
| Arena events emitted as Pulses on Bus | Integration test: subscribe to arena topics, verify Pulses received |

---

## 17. Crate Mapping

| Component | Crate | Status |
|---|---|---|
| Arena types + registry | `roko-chain` | Types needed |
| Eval types + registry | `roko-chain` | Types needed |
| Bounty market | `roko-chain/src/marketplace.rs` | Wired (job lifecycle, escrow, disputes) |
| VCG matching | `roko-compose/src/auction.rs` | Wired (`vcg_allocate` exported) |
| Validation records | `roko-chain/src/validation_registry.rs` | Wired (work proofs feed reputation) |
| Flywheel pipeline | `roko-learn` | Steps 2-3, 6-7 (auto-grade, preference, pattern, bootstrap) |
| Failure clustering | `roko-primitives` | HDC-based clustering |
| Curriculum generation | `roko-learn` | Adversarial task generation |
| Cross-arena transfer | `roko-primitives` + `roko-learn` | HDC similarity + router feedback |
| Meta-arena metrics | `roko-cli/src/orchestrate.rs` | Wired (self-hosting loop metrics) |
| Arena API routes | `roko-serve` | Not yet implemented |
| Eval API routes | `roko-serve` | Not yet implemented |
| Bounty API routes | `roko-serve` | Partial (jobs routes exist) |
| Contract deployment | `contracts/` | Not yet implemented |
