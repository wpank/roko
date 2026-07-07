# Decay — Examples

> Worked examples for every decay variant, the freeze-thaw cycle, and mixed-model scenarios.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Overview](00-overview.md)  
**Last reviewed**: 2026-04-19

---

## TL;DR

This page collects twelve standalone examples covering Demurrage, Exponential, Step, Linear,
Custom, reinforcement, cold-tier freeze-thaw, and GC boundary cases. Each example is
self-contained: it names the scenario, shows the relevant parameters, shows the computation,
and states the expected output.

---

## Example 1 — Demurrage, No Retrieval

**Scenario**: A `KnowledgeEntry` created 90 days ago, never retrieved. Default params.

```rust
<!-- source: crates/roko-core/tests/decay_examples.rs -->

let p = DemurrageParams::default();
// idle_tax_per_day = 0.01, balance = 1.0
let now_ms = 90 * 86_400_000_i64;
let created_at_ms = 0_i64;
let w = p.weight_at(now_ms, created_at_ms);
// (1 - 0.01)^90 = 0.99^90 ≈ 0.4047
assert!((w - 0.4047).abs() < 0.001);
```

**Result**: `weight ≈ 0.405`. Still above `GC_FLOOR = 0.001`; still above
`COLD_TIER_THRESHOLD = 0.1`. Remains warm.

---

## Example 2 — Demurrage, Retrieved Weekly

**Scenario**: A `KnowledgeEntry` created 90 days ago, retrieved once per week.

```rust
<!-- source: crates/roko-core/tests/decay_examples.rs -->

let mut p = DemurrageParams::default();
// Simulate 90 days = 12.86 weeks ≈ 12 retrievals
for _ in 0..12 {
    // Apply 7 days of idle tax
    p.apply_idle_tax(7.0);
    // Reinforce
    p.reinforce();
}
// Apply remaining ~7 days
p.apply_idle_tax(7.0);
// balance > 0.9 — frequent retrieval keeps it warm
assert!(p.balance > 0.9);
```

**Result**: `balance ≈ 0.93`. The equilibrium with 1 retrieval/week holds well above
the cold-tier threshold.

---

## Example 3 — Demurrage Reaches Cold Tier

**Scenario**: A `ToolTrace` with aggressive decay (`idle_tax = 0.05`) not retrieved for
30 days.

```rust
<!-- source: crates/roko-core/tests/decay_examples.rs -->

let p = DemurrageParams {
    balance: 1.0,
    idle_tax_per_day: 0.05,
    reinforcement_per_use: 0.02,
};
let w = p.weight_at(30 * 86_400_000, 0);
// (1 - 0.05)^30 = 0.95^30 ≈ 0.215
assert!((w - 0.215).abs() < 0.005);
// Still above COLD_TIER_THRESHOLD = 0.1
```

After 50 days:
```
0.95^50 ≈ 0.077 < 0.1 → cold tier
```

**Result**: Engram reaches cold tier somewhere between day 43 and day 44.

---

## Example 4 — Exponential, One Half-Life

**Scenario**: An `Observation` with `half_life_secs = 86_400` (1 day), queried exactly
24 hours after creation.

```rust
<!-- source: crates/roko-core/tests/decay_examples.rs -->

let p = ExponentialDecayParams { half_life_secs: 86_400 };
let w = p.weight_at(86_400_000, 0);  // exactly 1 day in ms
assert!((w - 0.5).abs() < 1e-9);
```

**Result**: `weight = 0.5` (by definition of half-life).

---

## Example 5 — Exponential GC Boundary

**Scenario**: When does an Observation with `half_life_secs = 3600` (1 hour) cross
`GC_FLOOR = 0.001`?

```
GC time = half_life × log2(1/GC_FLOOR)
        = 3600 × log2(1000)
        = 3600 × 9.966
        ≈ 35_878 seconds ≈ 9.97 hours
```

```rust
<!-- source: crates/roko-core/tests/decay_examples.rs -->

let p = ExponentialDecayParams { half_life_secs: 3600 };
let gc_ms = p.time_to_gc_ms(0, GC_FLOOR);
// Should be approximately 35_878_000 ms
assert!((gc_ms - 35_878_000).abs() < 1_000);
```

---

## Example 6 — Step Decay, Sprint-Scoped Plan

**Scenario**: A `Plan` Engram created at the start of a sprint. Query it mid-sprint, at
end of sprint (epoch 1), and two sprints later.

```rust
<!-- source: crates/roko-core/tests/decay_examples.rs -->

let p = StepDecayParams {
    balance: 1.0,
    epoch_secs: 604_800,   // 1 week
    step_multiplier: 0.5,
};

// Mid-sprint (day 3 = 259_200 seconds = < 1 epoch)
let w_mid = p.weight_at(259_200_000, 0);
assert_eq!(w_mid, 1.0);  // still epoch 0

// End of sprint (1 week + 1 second = epoch 1)
let w_end = p.weight_at(604_801_000, 0);
assert!((w_end - 0.5).abs() < 0.001);  // one step applied

// Two sprints later (epoch 2)
let w_two = p.weight_at(2 * 604_800_000 + 1_000, 0);
assert!((w_two - 0.25).abs() < 0.001);
```

---

## Example 7 — Linear Decay, Session Context

**Scenario**: A `ContextAssembly` Engram with a 30-minute lifetime. Check weight at
10 min, 30 min, and 31 min.

```rust
<!-- source: crates/roko-core/tests/decay_examples.rs -->

let p = LinearDecayParams {
    balance: 1.0,
    rate_per_sec: 1.0 / 1_800.0,  // 30-minute lifetime
};

// 10 minutes
let w_10 = p.weight_at(600_000, 0);  // 600_000 ms = 600 s
// 1.0 - (1/1800 × 600) = 1.0 - 0.333 = 0.667
assert!((w_10 - 0.667).abs() < 0.001);

// Exactly 30 minutes
let w_30 = p.weight_at(1_800_000, 0);
assert!((w_30).abs() < 0.001);  // ≈ 0.0

// 31 minutes (past expiry)
let w_31 = p.weight_at(1_860_000, 0);
assert_eq!(w_31, 0.0);  // clamped at floor
```

---

## Example 8 — Reinforcement Equilibrium Check

**Scenario**: Verify that with default Demurrage params, any retrieval rate ≥ 1/day
keeps the Engram at full weight.

```rust
<!-- source: crates/roko-core/tests/decay_examples.rs -->

let mut p = DemurrageParams::default();  // tax=0.01, reinforce=0.05
// Simulate 365 daily retrievals
for _ in 0..365 {
    p.apply_idle_tax(1.0);
    p.reinforce();
}
// After 1 year of daily retrieval, balance should be at 1.0
assert!((p.balance - 1.0).abs() < 0.001);
```

---

## Example 9 — Cold Tier Freeze and Thaw

**Scenario**: An Engram drops below `COLD_TIER_THRESHOLD`, gets frozen. After 6 months
(below `MAX_COLD_DWELL_SECS`), a query retrieves it. Verify thaw restores balance.

```rust
<!-- source: crates/roko-fs/tests/substrate_decay.rs -->

// 1. Create Engram with low balance
let mut p = DemurrageParams { balance: 0.05, ..Default::default() };
assert!(p.balance < COLD_TIER_THRESHOLD);

// 2. Substrate freezes it
let frozen_at = 0_i64;
substrate.freeze(&id, frozen_at).unwrap();

// 3. Six months later, query retrieves it
let now = 15_552_000_000_i64;  // 180 days in ms
let engram = substrate.thaw(&id, now).unwrap();

// 4. Balance should be THAW_RESTORE_BALANCE
match &engram.decay {
    Decay::Demurrage(p) => assert!((p.balance - THAW_RESTORE_BALANCE).abs() < 0.001),
    _ => panic!("unexpected variant"),
}
```

---

## Example 10 — Cold Tier GC after Max Dwell

**Scenario**: An Engram is frozen and never retrieved. After `MAX_COLD_DWELL_SECS`,
the GC job deletes it.

```rust
<!-- source: crates/roko-fs/tests/substrate_decay.rs -->

let frozen_at = 0_i64;
substrate.freeze(&id, frozen_at).unwrap();

// Time advances past max cold dwell
let after_dwell = (MAX_COLD_DWELL_SECS as i64 + 1) * 1_000;
let report = substrate.gc_cold_tier(after_dwell);

assert_eq!(report.cold_gc_count, 1);
assert!(substrate.get_warm(&id).is_err());  // gone
assert!(substrate.cold_store.get(&id).is_err());  // gone from cold too
```

---

## Example 11 — Custom Handler (Ebbinghaus)

**Scenario**: An Engram using the Ebbinghaus custom handler, retrieved three times over
30 days.

```
Retrieval 1: day 0  — stability = 1.0
Retrieval 2: day 2  — stability doubles to 2.0
Retrieval 3: day 6  — stability doubles to 4.0
Query at day 14 (8 days since last retrieval):
  weight = e^(-8/4) = e^(-2) ≈ 0.135
```

```rust
<!-- source: crates/roko-neuro/tests/ebbinghaus_decay.rs -->

let handler = EbbinghausHandler;
let mut params = json!({"stability": 4.0});  // after 3 retrievals
let last_retrieved = 6 * 86_400_000_i64;
let now = 14 * 86_400_000_i64;
let w = handler.weight_at(&params, now, 0, last_retrieved);
assert!((w - 0.135).abs() < 0.01);
```

---

## Example 12 — Builder Integration

**Scenario**: Create an Engram with an explicit non-default decay.

```rust
<!-- source: crates/roko-core/tests/decay_examples.rs -->

let engram = Engram::builder()
    .kind(Kind::KnowledgeEntry)
    .body(Body::Text("Rust's borrow checker enforces ownership".into()))
    .provenance(Provenance::local_agent("agent-001"))
    .decay(Decay::Demurrage(DemurrageParams {
        balance: 1.0,
        idle_tax_per_day: 0.002,   // very slow decay
        reinforcement_per_use: 0.05,
    }))
    .build()
    .expect("valid engram");

// ContentHash is deterministic regardless of decay parameters
assert_eq!(engram.id, expected_hash);

// Decay is present
match engram.decay {
    Decay::Demurrage(p) => assert_eq!(p.idle_tax_per_day, 0.002),
    _ => panic!("wrong variant"),
}
```

---

## Open Questions

None at this time.

## See Also

- [`00-overview.md`](00-overview.md) — all decay variants
- [`10-api-reference.md`](10-api-reference.md) — complete method signatures
- [`09-invariants.md`](09-invariants.md) — invariants verified by these examples
