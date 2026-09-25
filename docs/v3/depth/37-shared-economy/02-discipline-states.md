# Depth: Discipline States

> Four-state discipline machine with threshold-driven transitions, 90-day
> rolling slash windows, and governance override.

**Parent chapter:** [37-SHARED-ECONOMY](../../37-SHARED-ECONOMY.md) Section 2
**Source:** `crates/roko-chain/src/reputation_registry.rs`

---

## State Machine

```
GOOD_STANDING --> PROBATION --> SUSPENSION --> BANNED
      ^               ^            ^
      '---------------'------------'  (recovery)
```

All forward transitions are automatic based on score thresholds and slash
history. Backward transitions (recovery) require explicit conditions to be met.
The `Banned` state can only be entered via admin/governance action and exited
via governance amnesty after 365 days.

---

## Transition Conditions

### Good Standing to Probation

Triggered when any participated domain's effective (decay-adjusted) score
drops below 0.4. The check uses effective scores, so decay alone can trigger
probation if an agent has been inactive long enough.

```rust
for domain in self.domains.values() {
    let effective = domain.effective_score(now);
    if effective < 0.4 {
        return DisciplineState::Probation;
    }
}
```

### Probation to Suspension

Triggered by either condition:

1. **Score threshold:** Any domain's effective score drops below 0.2.
2. **Slash accumulation:** 3 or more slashing events across all domains within
   a rolling 90-day window.

```rust
let total_slashes: u32 = self.domains
    .values()
    .map(|d| d.slash_count_in_window(now))
    .sum();
if total_slashes >= 3 {
    return DisciplineState::Suspended;
}
```

The 90-day window uses a rolling count of timestamped slash events. Old slashes
age out automatically.

### Suspension to Banned

Only via explicit admin/governance action:

```rust
pub fn ban_agent(&mut self, passport_id: AgentId, now: u64) {
    if let Some(agent) = self.records.get_mut(&passport_id) {
        agent.discipline_override = Some(DisciplineState::Banned);
        agent.recovery.state_entered_at = now;
    }
}
```

---

## Restrictions by State

| State | Job Participation | Group Leadership | Direct Hire | Pricing Tier |
|---|---|---|---|---|
| Good Standing | Full | Yes | Yes | Score-based |
| Probation | Limited | No | No | Free (0.0x) |
| Suspension | None in domain | No | No | Free (0.0x) |
| Banned | None | No | No | Free (0.0x) |

Any state other than GoodStanding results in the Free pricing tier (0.0x
multiplier), which prevents selling paid access.

---

## Override Mechanism

The discipline state is normally computed from scores and slash history. An
explicit override can be set for governance actions (ban, amnesty):

```rust
pub fn discipline_state(&self, now: u64) -> DisciplineState {
    if let Some(state) = self.discipline_override {
        return state;
    }
    // ... compute from scores and slashes
}
```

The override takes absolute precedence. Setting `discipline_override = Some(GoodStanding)`
after amnesty restores full participation regardless of current scores.
