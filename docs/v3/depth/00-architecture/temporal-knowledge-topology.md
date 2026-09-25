# Temporal Knowledge Topology

> **v3 depth file** -- `/docs/v3/depth/00-architecture/temporal-knowledge-topology.md`
> Canonical source: v1 `docs/v1/00-architecture/27-temporal-knowledge-topology.md`
> Status: **Specified** -- Core temporal types and Allen's algebra are specified; HDC
> fingerprints are wired per-episode. Full temporal knowledge graph, event calculus, and
> constraint propagation remain target-state.

---

## 1. The Problem: Timeless Knowledge in a Temporal World

Roko's Neuro knowledge store currently treats knowledge as effectively atemporal. A Signal has
a `created_at` timestamp and a `Decay` variant that controls how its weight diminishes over
time. But the knowledge itself has no explicit temporal structure. There is no way to express:

- **Validity windows**: "This API endpoint was active from March to June 2025"
- **Temporal ordering**: "The migration happened before the schema change"
- **Temporal overlap**: "While we were on version 2.x, the bug was present"
- **Causal chains**: "Because the CI pipeline broke, the release was delayed"

---

## 2. Allen's Interval Algebra

### 2.1 The 13 Relations (Allen 1983)

Allen (1983) defined 13 mutually exclusive relations between time intervals. Every pair of
temporal intervals satisfies exactly one of these relations:

```rust
/// Allen's 13 temporal interval relations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AllenRelation {
    Before,        // x_end < y_start
    Meets,         // x_end == y_start
    Overlaps,      // x_start < y_start < x_end < y_end
    Starts,        // x_start == y_start, x_end < y_end
    During,        // y_start < x_start, x_end < y_end
    Finishes,      // x_end == y_end, x_start > y_start
    Equals,        // x_start == y_start, x_end == y_end
    After,         // inverse of Before
    MetBy,         // inverse of Meets
    OverlappedBy,  // inverse of Overlaps
    StartedBy,     // inverse of Starts
    Contains,      // inverse of During
    FinishedBy,    // inverse of Finishes
}

/// A time interval with nanosecond precision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemporalInterval {
    pub start: i64,  // Unix timestamp (nanoseconds)
    pub end: i64,    // Unix timestamp (nanoseconds), or i64::MAX for "ongoing"
}

impl TemporalInterval {
    pub const ONGOING: i64 = i64::MAX;

    pub fn new(start: i64, end: i64) -> Self {
        debug_assert!(start <= end, "interval start must not exceed end");
        Self { start, end }
    }

    /// Determine the Allen relation between self and other.
    pub fn relation_to(&self, other: &Self) -> AllenRelation {
        // Full 13-relation classification (see v1 for exhaustive implementation)
        if self.end < other.start { AllenRelation::Before }
        else if self.end == other.start { AllenRelation::Meets }
        else if self.start == other.start && self.end == other.end { AllenRelation::Equals }
        else if self.start > other.end { AllenRelation::After }
        // ... remaining 9 cases follow the same pattern
        else { AllenRelation::Overlaps } // simplified
    }

    pub fn overlaps_with(&self, other: &Self) -> bool {
        self.start < other.end && other.start < self.end
    }

    pub fn duration_ns(&self) -> Option<i64> {
        if self.end == Self::ONGOING { None } else { Some(self.end - self.start) }
    }
}
```

### 2.2 Temporal Constraint Network

Allen's algebra supports constraint propagation -- if A is before B and B overlaps C, we can
infer the possible relations between A and C.

```rust
pub struct TemporalConstraintNetwork {
    constraints: HashMap<(ContentHash, ContentHash), AllenRelationSet>,
    intervals: HashMap<ContentHash, TemporalInterval>,
}

/// Compact set of Allen relations (13 bits, one per relation).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllenRelationSet(u16);

impl AllenRelationSet {
    pub const ALL: Self = Self(0x1FFF);  // 13 bits set
    pub const EMPTY: Self = Self(0);
}
```

### 2.3 Constraint Propagation Algorithm

```
ALGORITHM: AllenConstraintPropagation(network, new_constraint(A, B, R))

1. Add R to constraints[(A, B)]
2. Initialize worklist = [(A, B)]
3. While worklist is not empty:
   a. Pop (X, Y) from worklist
   b. For each Z != X, Y with known constraints:
      - Compute transitive: R_xz_new = compose(constraints[(X, Y)], constraints[(Y, Z)])
      - Compute intersection: R_xz = constraints[(X, Z)] intersect R_xz_new
      - If R_xz is stricter than stored:
        - Update constraints[(X, Z)] = R_xz
        - Add (X, Z) to worklist
      - If R_xz is empty:
        - INCONSISTENCY DETECTED -- temporal contradiction

COMPLEXITY: O(N^3) worst case, but sparse networks are much faster.
COMPOSITION TABLE: 13x13 table of Allen relation compositions (Allen 1983, Table 2).
```

---

## 3. Event Calculus (Kowalski & Sergot 1986)

### 3.1 Fluents and Events

```rust
/// A fluent is a time-varying property of the system.
pub struct Fluent {
    pub id: FluentId,
    pub name: String,
    pub value: serde_json::Value,
    pub valid: TemporalInterval,
    pub initiated_by: Option<EventId>,
    pub terminated_by: Option<EventId>,
}

/// An event is a point-in-time occurrence that initiates or terminates fluents.
pub struct TemporalEvent {
    pub id: EventId,
    pub timestamp: i64,
    pub description: String,
    pub signal_hash: Option<ContentHash>,
    pub initiates: Vec<FluentId>,
    pub terminates: Vec<FluentId>,
    pub caused_by: Vec<EventId>,
}
```

### 3.2 Core Axioms

```rust
pub struct EventCalculus {
    pub events: BTreeMap<i64, Vec<TemporalEvent>>,
    pub fluents: HashMap<FluentId, Vec<Fluent>>,
}

impl EventCalculus {
    /// HoldsAt(fluent, time) -- is the fluent true at the given time?
    pub fn holds_at(&self, fluent_id: FluentId, time: i64) -> bool {
        self.fluents.get(&fluent_id)
            .map(|history| {
                history.iter().any(|f| {
                    f.valid.start <= time
                        && (f.valid.end == TemporalInterval::ONGOING || f.valid.end > time)
                })
            })
            .unwrap_or(false)
    }

    /// CausedBy(event_a, event_b) -- transitive causal chain.
    pub fn caused_by(&self, effect: EventId, cause: EventId) -> bool {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(effect);
        while let Some(current) = queue.pop_front() {
            if current == cause { return true; }
            if !visited.insert(current) { continue; }
            if let Some(event) = self.find_event(current) {
                queue.extend(event.caused_by.iter().copied());
            }
        }
        false
    }
}
```

---

## 4. Temporal Knowledge Graph

### 4.1 Three-Tier Architecture (Rasmussen et al. 2025)

```rust
/// Temporal Knowledge Graph with three tiers.
///
/// Tier 1: Episode Layer -- raw Signal sequences with bundled fingerprints
/// Tier 2: Entity Layer -- extracted entities with temporal properties
/// Tier 3: Community Layer -- HDC-backed clusters of related entities
pub struct TemporalKnowledgeGraph {
    pub episodes: Vec<TemporalEpisode>,
    pub entities: HashMap<EntityId, TemporalEntity>,
    pub communities: Vec<TemporalCommunity>,
    pub temporal_constraints: TemporalConstraintNetwork,
    pub event_calculus: EventCalculus,
}
```

### 4.2 HDC Fingerprints Across the Temporal Tiers

Allen relations explain *when* two records relate. HDC fingerprints explain *how close in
meaning* those records are. The temporal topology uses fingerprints at each tier:

- **Tier 1 episodes** bundle the fingerprints of their member Signals, producing an episode
  centroid that can be compared with other episodes in constant time.
- **Tier 2 entities** maintain a running centroid over the Signals that mention or update the
  entity, so "the same thing evolving through time" becomes a similarity query.
- **Tier 3 communities** are formed when temporal overlap and HDC similarity both exceed
  threshold. This avoids clustering two co-temporal but semantically unrelated episodes.

### 4.3 HDC-Guided Tier Progression

1. Episode Signals land with their own fingerprints.
2. Delta consolidation groups temporally overlapping episodes whose fingerprints fall within
   a similarity radius.
3. The cluster center becomes a candidate semantic Signal: less specific than any single
   episode, but still anchored to the contributing lineage.
4. As older episodes accumulate noise through decay, the centroid remains close to the broad
   pattern while drifting away from one-off details.

---

## 5. Temporal Queries

```rust
pub enum TemporalQuery {
    PointQuery { fluent_pattern: String, at_time: i64 },
    IntervalQuery { fluent_pattern: String, during: TemporalInterval },
    DiffQuery { entity_pattern: String, from: i64, to: i64 },
    CausalQuery { effect_event: EventId, max_depth: usize },
    AllenQuery { reference: ContentHash, relation: AllenRelation },
    PredictionQuery { fluent_pattern: String, at_future_time: i64, confidence_threshold: f64 },
}
```

---

## 6. Integration with Decay Variants

| Decay Variant | Temporal Enhancement |
|---|---|
| **HalfLife** | Half-life scaled by temporal community stability |
| **TTL** | TTL extended if fluent is still valid (re-observation resets timer) |
| **Ebbinghaus** | Spaced repetition intervals derived from temporal access pattern |
| **None** | No change (axioms and definitions do not decay) |

---

## 7. Configuration

```toml
[temporal]
enabled = true

[temporal.intervals]
default_validity_secs = 86400
comparison_epsilon_ns = 1_000_000

[temporal.constraint_network]
max_entities = 10_000
max_propagation_iterations = 1_000

[temporal.event_calculus]
max_causal_depth = 20
min_causal_confidence = 0.01

[temporal.tkg]
max_episodes = 5_000
community_detection_interval = 3
min_community_size = 3
stability_threshold = 0.7

[temporal.prediction]
min_data_points = 5
max_horizon_secs = 604800
```

---

## 8. Test Criteria

| Test | What It Validates | Type |
|---|---|---|
| `test_allen_all_13_relations` | Each of 13 relations correctly computed | Unit |
| `test_allen_exhaustive_coverage` | Every pair maps to exactly one relation | Property |
| `test_constraint_propagation_transitive` | A before B, B before C -> A before C | Unit |
| `test_constraint_inconsistency_detected` | A before B and B before A -> error | Unit |
| `test_holds_at_basic` | Fluent initiated at T1, queried at T2 > T1: holds | Unit |
| `test_holds_at_terminated` | Fluent terminated at T2, queried at T3 > T2: not holds | Unit |
| `test_causal_chain_transitive` | A caused B, B caused C -> A caused C | Unit |
| `test_tkg_community_detection` | Co-temporal entities clustered by overlap + fingerprint | Integration |
| `test_diff_query_detects_changes` | DiffQuery between T1 and T2 finds changes | Unit |
| `test_modulated_decay_stability` | Stable community -> slower decay | Unit |

---

## Cross-References

- [Emergent Goal Structures](./emergent-goal-structures.md) -- temporal patterns trigger goals
- [Cognitive Immune System](./cognitive-immune-system.md) -- temporal consistency as immune check
- v3 Section 6 -- Neuro, knowledge store wrapped in TKG
