# M085 — Bounty System

**[BLOCKED:depth]** -- This item depends on `tmp/unified-depth/19-arenas/` depth docs and M076/M078 (on-chain contracts and clients). The depth docs specify escrow mechanics, VCG matching parameters, second-price auction rules, and 4-level dispute resolution procedures.

## Objective
Implement the bounty system: escrow before execution, VCG task matching, second-price auctions for competitive bounties, reputation settlement after completion, and 4-level dispute resolution (arbiter -> court -> council -> DAO vote). Bounties are the economic mechanism that connects Agents to paid work.

## Scope
- Crates: `roko-learn`
- Files: `crates/roko-learn/src/arena/bounty.rs` (new)
- Phase ref: `tmp/unified-migration/04-PHASE-3-ECONOMY.md` SS3.5
- Spec ref: `tmp/unified/19-ARENAS-EVALS-BOUNTIES.md` SS6-7
- Depth docs: `tmp/unified-depth/19-arenas/` (pending)

## Steps
1. Define bounty types:
   ```rust
   pub struct Bounty {
       pub id: String,
       pub title: String,
       pub description: String,
       pub escrow_amount: f64,
       pub deadline: DateTime<Utc>,
       pub arena_id: Option<String>,
       pub scoring: ScoringFunction,
       pub status: BountyStatus,
   }

   pub enum BountyStatus {
       Open,
       Claimed { agent_id: String },
       InProgress { agent_id: String },
       Completed { agent_id: String, score: f64 },
       Disputed { level: DisputeLevel },
       Cancelled,
   }

   pub enum DisputeLevel {
       Arbiter,
       Court,
       Council,
       DaoVote,
   }
   ```

2. Implement bounty lifecycle:
   - Post: escrow funds, set deadline
   - Claim: agent claims via VCG/auction
   - Complete: eval scores result, escrow releases
   - Dispute: escalate through 4 levels

3. Write tests: post bounty -> agent claims -> completes -> escrow releases -> reputation updated.

## Verification
```bash
cargo check -p roko-learn
cargo clippy -p roko-learn --no-deps -- -D warnings
cargo test -p roko-learn -- arena::bounty
```

## What NOT to do
- Do NOT implement real escrow without on-chain contracts -- use local mock
- Do NOT proceed without depth docs
- Do NOT skip VCG matching -- simple FIFO claiming is not sufficient
- Do NOT implement DAO voting mechanism -- stub the highest dispute level
