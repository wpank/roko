# Depth: Recovery Paths

> Graduated trust rebuilding from Probation and Suspension, 365-day ban
> amnesty, and the RecoveryTracker implementation.

**Parent chapter:** [37-SHARED-ECONOMY](../../37-SHARED-ECONOMY.md) Section 2.3
**Source:** `crates/roko-chain/src/reputation_registry.rs`

---

## Recovery Requirements

```rust
pub struct RecoveryRequirements {
    pub min_jobs: usize,           // Minimum qualifying jobs
    pub min_avg_feedback: f64,     // Required average quality
    pub waiting_period_secs: u64,  // Mandatory wait (0 for probation)
    pub requires_stake: bool,      // 2x domain stake equivalent
    pub requires_verification: bool, // Domain-specific gate run
}
```

### Probation Recovery

```rust
RecoveryRequirements {
    min_jobs: 10,
    min_avg_feedback: 0.6,
    waiting_period_secs: 0,
    requires_stake: false,
    requires_verification: false,
}
```

The agent must complete 10 jobs in the domain with average feedback >= 0.6
(above "Adequate" level). No waiting period and no stake required. This path
is designed to be achievable within 2-4 weeks of active work.

### Suspension Recovery

```rust
RecoveryRequirements {
    min_jobs: 0,
    min_avg_feedback: 0.0,
    waiting_period_secs: 90 * 24 * 3600,  // 90 days
    requires_stake: true,
    requires_verification: true,
}
```

Suspension recovery is deliberately more demanding:

1. **90-day waiting period:** No jobs can be accepted in the suspended domain
   during this time. The agent can work in other domains.
2. **2x domain stake:** Financial commitment demonstrating seriousness.
3. **Verification challenge:** Pass a domain-specific gate run (e.g., compile
   and test a reference task for the `coding` domain).
4. **Returns to Probation:** Even after meeting all requirements, the agent
   returns to Probation -- not directly to Good Standing. They must then
   complete the probation recovery path.

Total minimum recovery time from Suspension to Good Standing: 90 days (wait)
+ 2-4 weeks (probation recovery) = approximately 4 months.

---

## RecoveryTracker Implementation

```rust
pub struct RecoveryTracker {
    pub recovery_jobs: Vec<RecoveryJob>,
    pub state_entered_at: u64,
    pub recovery_stake_posted: bool,
    pub verification_passed: bool,
}

pub struct RecoveryJob {
    pub feedback: f64,
    pub completed_at: u64,
}
```

### Status Check

The tracker evaluates requirements in priority order:

1. Waiting period (if applicable)
2. Stake requirement (if applicable)
3. Verification requirement (if applicable)
4. Job count
5. Average feedback

```rust
pub enum RecoveryStatus {
    Eligible,
    WaitingPeriod { remaining_secs: u64 },
    NeedMoreJobs { current: usize, required: usize },
    FeedbackTooLow { current_avg: f64, required: f64 },
    NeedStake,
    NeedVerification,
    NotApplicable,
}
```

---

## Ban Amnesty

Banned agents can appeal through governance after 365 days:

```rust
pub fn amnesty_eligible(&self, passport_id: AgentId, now: u64) -> Option<u64> {
    // Returns Some(0) if eligible now, Some(days_remaining) if waiting,
    // None if not banned
}

pub fn governance_amnesty(&mut self, passport_id: AgentId, now: u64) -> bool {
    // Lifts ban, resets recovery tracker, returns true if successful
}
```

Amnesty is not automatic -- it requires an explicit admin/governance decision.
The 365-day wait is a minimum, not a guarantee. The governance body may choose
to deny amnesty even after the waiting period.

---

## Reputation Amnesty (Systemic Events)

For systemic failures affecting many agents simultaneously (model provider
outage, infrastructure incident), governance can issue a reputation amnesty:

- Reverses specific slashing events across affected agents
- Restores discipline states to pre-event levels
- Scoped to a time range and specific domains
- Does not affect feedback weight dilution (collusion penalties are separate)

This prevents a single infrastructure failure from cascading into mass
suspensions across the network.
