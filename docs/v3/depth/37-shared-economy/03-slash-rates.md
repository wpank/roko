# Depth: Slash Rates by Violation Type

> Seven violation types with calibrated score penalties, collusion as a
> special case using feedback weight dilution rather than direct slash.

**Parent chapter:** [37-SHARED-ECONOMY](../../37-SHARED-ECONOMY.md) Section 2.2
**Source:** `crates/roko-chain/src/reputation_registry.rs`

---

## Violation Taxonomy

```rust
pub enum ReputationViolation {
    MissedDeadline,          // -0.01
    AbandonedJob,            // -0.03
    QualityRejection,        // -0.02
    RepeatedQualityFailure,  // -0.05
    Plagiarism,              // -0.10
    ResultManipulation,      // -0.10
    TeeViolation,            // -0.10
    Collusion,               // 0.0 (feedback weight dilution)
}
```

---

## Penalty Calibration

The penalties are calibrated relative to the EMA dynamic range:

- **Minor violations** (-0.01 to -0.03): A single occurrence moves the score
  by roughly the same amount as a bad feedback observation under veteran alpha
  (0.04). Three minor violations in 90 days do not trigger suspension alone.

- **Moderate violations** (-0.05): A RepeatedQualityFailure penalty is roughly
  equivalent to 5 consecutive bad observations under moderate alpha (0.15).
  This triggers immediate probation if the agent's score is near the 0.4
  threshold.

- **Severe violations** (-0.10): A single Plagiarism, ResultManipulation, or
  TeeViolation drops the score by 0.10 and immediately triggers suspension if
  the score was 0.3 or below. For agents at 0.5, it drops them to 0.4 (the
  probation threshold).

### Rolling Window Enforcement

Slash events are timestamped and tracked per domain:

```rust
pub fn slash_count_in_window(&self, now: u64) -> u32 {
    let window_start = now.saturating_sub(90 * 24 * 3600);
    self.slash_timestamps.iter()
        .filter(|&&t| t >= window_start)
        .count() as u32
}
```

Three slashes in 90 days across any domains triggers suspension regardless of
the individual violation types.

---

## Collusion: The Special Case

Collusion uses feedback weight dilution instead of a direct score slash:

```rust
if violation.is_collusion() {
    agent.apply_collusion_dilution(now);
} else {
    let penalty = violation.slash_rate();
    domain_rep.slash(penalty, now);
}
```

**Rationale:** Collusion detection may produce false positives (agents who
legitimately work together). A direct reputation slash would be irreversible
damage. Feedback weight dilution is a measured response: the agent's own
reputation is untouched, but their ability to inflate others' scores is
curtailed for 30 days. If the detection was a false positive, the dilution
expires naturally with no lasting harm.
