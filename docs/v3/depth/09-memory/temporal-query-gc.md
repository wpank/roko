# Temporal Query and GC

> **v3 depth -- 09-memory** | Allen's 13 intervals, GC.

`roko-neuro` provides an optional in-memory `TemporalIndex` using Allen's
13 interval relations (Allen 1983). Each knowledge entry can be associated
with a temporal interval, and the index supports temporal queries alongside
the primary HDC similarity and keyword paths.

---

## Allen's 13 Interval Relations

Allen (1983) defined 13 fundamental relations between temporal intervals.
These relations form a JEPD (jointly exhaustive, pairwise disjoint)
partition: every pair of intervals satisfies exactly one relation.

```rust
pub enum AllenRelation {
    Before,       // A entirely before B (gap between)
    After,        // A entirely after B
    Meets,        // A ends exactly where B starts (no gap)
    MetBy,        // B ends exactly where A starts
    Overlaps,     // A starts before B, A ends during B
    OverlappedBy, // B starts before A, B ends during A
    Contains,     // A starts before B, A ends after B
    During,       // A starts after B starts, A ends before B ends
    Starts,       // A and B start at same time, A ends first
    StartedBy,    // A and B start at same time, B ends first
    Finishes,     // A and B end at same time, A starts later
    FinishedBy,   // A and B end at same time, B starts later
    Equal,        // A and B have same start and end
}
```

### Interval representation

```rust
pub struct TemporalInterval {
    pub start: i64,   // epoch milliseconds
    pub end: i64,
}
```

### Relation computation

For intervals A = [a1, a2] and B = [b1, b2]:

| Relation | Condition |
|----------|-----------|
| Before | a2 < b1 |
| After | a1 > b2 |
| Meets | a2 == b1 |
| MetBy | a1 == b2 |
| Overlaps | a1 < b1 AND a2 > b1 AND a2 < b2 |
| OverlappedBy | b1 < a1 AND b2 > a1 AND b2 < a2 |
| Contains | a1 < b1 AND a2 > b2 |
| During | a1 > b1 AND a2 < b2 |
| Starts | a1 == b1 AND a2 < b2 |
| StartedBy | a1 == b1 AND a2 > b2 |
| Finishes | a2 == b2 AND a1 > b1 |
| FinishedBy | a2 == b2 AND a1 < b1 |
| Equal | a1 == b1 AND a2 == b2 |

### Inverse relations

Each relation has an inverse: Before/After, Meets/MetBy, Overlaps/OverlappedBy,
Contains/During, Starts/StartedBy, Finishes/FinishedBy. Equal is its own
inverse.

---

## TemporalIndex

The in-memory index maintains epoch-ordered records:

```rust
pub struct TemporalIndex {
    entries: Vec<(String, TemporalInterval)>,  // entry_id, interval
}

impl TemporalIndex {
    /// Find all entries with the given relation to a query interval.
    pub fn query(&self, interval: &TemporalInterval, relation: AllenRelation)
        -> Vec<&str>;

    /// Find all entries overlapping a time range.
    pub fn overlapping(&self, start: i64, end: i64) -> Vec<&str>;

    /// Find entries from the last N hours.
    pub fn recent(&self, hours: u64) -> Vec<&str>;
}
```

---

## Garbage Collection

### Confidence-based GC

Entries below `DEFAULT_GC_MIN_CONFIDENCE` (0.05) are removed.
AntiKnowledge entries are exempt below their 0.3 floor.

### Balance-based GC

Entries below `BALANCE_GC_FLOOR` (0.05) are frozen rather than deleted:

```rust
pub const BALANCE_GC_FLOOR: f64 = 0.05;
pub const THAW_STARTER_BALANCE: f64 = 0.3;
```

### Seven-day depleted-balance freezing

Entries at zero balance for 7 days are automatically frozen into cold
storage. The `balance_depleted_at` timestamp tracks when depletion began.

### Worldview-aware GC

Worldview clustering (union-find by tag overlap) prevents GC from removing
the last representative of a conceptual cluster. If an entry is the sole
member of its worldview cluster, it is preserved even if its confidence or
balance would otherwise trigger GC.

### GC schedule

```
1. Compute confidence and balance for all entries
2. Identify entries below GC thresholds
3. Check worldview cluster membership
4. Freeze (not delete) entries below balance floor
5. Delete entries below confidence floor (except AntiKnowledge)
6. Preserve sole cluster representatives
```

---

## Cold Storage: Freeze and Thaw

### Freeze

Entry keeps content address and lineage; body moves off hot query path.
`frozen = true`, `frozen_at = now`.

### Thaw

When a future query needs the entry:
1. Restore from cold storage
2. Set balance to `THAW_STARTER_BALANCE` (0.3)
3. Set `frozen = false`
4. Entry competes again on the hot path

If it keeps failing after thaw, it cools back down. This prevents cold
entries from getting infinite second chances.

---

## Academic Foundations

- Allen, J. F. (1983). "Maintaining knowledge about temporal intervals."
  *Communications of the ACM*, 26(11), 832--843.
- Allen, J. F. & Hayes, P. J. (1985). "A common-sense theory of time."
  *IJCAI*.

---

## Cross-References

- `ebbinghaus-decay-with-tier.md` -- decay formulas driving GC timing
- `four-validation-tiers.md` -- tier affects effective half-life
- `knowledge-query-api.md` -- temporal queries in the query API
