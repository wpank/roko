# 00-ARCH -- Compositional Kinds

> **Parent**: [00-ARCHITECTURE](../../00-ARCHITECTURE.md)
>
> The Signal type system uses `Kind` to classify signals. This depth file specifies
> `Kind::Compound(Vec<Kind>)` -- a compositional extension that lets a Signal carry
> multiple kinds simultaneously -- and defines the algebraic structure (join-semilattice),
> dispatch semantics, scoring algebra, serialization format, migration path, and limits.
>
> Canonical source: `crates/roko-core/src/kind.rs`

---

## 1. The Problem with Flat Kinds

The current `Kind` enum is flat: a Signal is a `GateVerdict` or a `PromptSection` or a
`Task`, but it cannot be both. Some signals naturally span multiple domains. Three
concrete problems motivate the Compound extension:

### 1.1 Verdict + Test

A test gate produces a verdict that contains detailed test results. The Signal must be
`Kind::GateVerdict` for the gate pipeline and `Kind::TestResult` for the test analysis
system. Today the orchestrator emits two separate Signals with shared lineage, doubling
storage and requiring consumers to join on lineage.

### 1.2 Episode + Skill

When the skill library extracts a skill from an episode, the resulting Signal is both a
Skill (for the skill library) and an Episode derivative (for the learning system).
Today it is `Kind::Skill` with a tag `source=episode`, which the episode logger cannot
query by Kind alone.

### 1.3 Routing Feedback + Cost

A routing feedback Signal carries cost information. The cost normalization system needs
`Kind::Metric`-like access, but the Signal is `Kind::RouterFeedback`. Tags bridge the
gap, but tag-based dispatch is stringly typed and fragile.

The same composition model gives learning-layer artifacts a clean type-level home when
a record needs to carry both heuristic identity and worldview membership. That stays in
the kind system rather than being represented as tags or ad hoc fields.

---

## 2. The Compound Variant

### 2.1 Definition

```rust
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    // ... all existing variants unchanged ...

    /// A signal that participates in multiple domains simultaneously.
    /// The inner Vec is sorted and deduplicated on construction.
    /// Dispatch matches if ANY inner kind matches the filter.
    Compound(Vec<Kind>),
}
```

### 2.2 Construction Algebra

Compound kinds are built via a helper that enforces structural invariants:

```rust
impl Kind {
    const MAX_COMPOUND_SIZE: usize = 4;

    /// Create a compound kind from multiple kinds.
    /// Flattens nested Compounds, deduplicates, and sorts.
    /// Returns the single inner kind if only one remains after dedup.
    pub fn compound(kinds: impl IntoIterator<Item = Kind>) -> Kind {
        let mut flat: Vec<Kind> = Vec::new();
        for k in kinds {
            match k {
                Kind::Compound(inner) => flat.extend(inner),
                other => flat.push(other),
            }
        }
        flat.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        flat.dedup();
        match flat.len() {
            0 => Kind::Custom("empty_compound".into()),
            1 => flat.into_iter().next().unwrap(),
            _ => Kind::Compound(flat),
        }
    }
}
```

### 2.3 Structural Invariants

The construction algebra satisfies four invariants that together make `Kind::compound`
an associative, commutative, idempotent operation on the set of Kinds, forming a
**join-semilattice** under the subset ordering:

1. **Idempotency of flattening**: `compound(compound([A, B]), C) = compound(A, B, C)`.
   No nesting. A Compound never contains another Compound.

2. **Canonical ordering**: The inner Vec is sorted by `as_str()` and deduplicated.
   Equality comparison and hashing are deterministic regardless of insertion order.

3. **Minimum cardinality**: A Compound with one element unwraps to that element
   (identity under singleton). A Compound with zero elements becomes
   `Kind::Custom("empty_compound")` (an error state).

4. **Bounded size**: Cap at 4 kinds per Compound. A Signal that participates in 5+
   domains is a design smell -- it should be split into separate Signals with shared
   lineage.

### 2.4 Algebraic Properties

The `compound` operation satisfies the following algebraic laws:

```
Associativity:  compound(compound(A, B), C) = compound(A, compound(B, C))
Commutativity:  compound(A, B) = compound(B, A)
Idempotency:    compound(A, A) = A
Identity:       compound(A) = A  (singleton unwrap)
```

These properties mean that the order in which kinds are combined does not matter, and
combining the same kind multiple times has no additional effect. The resulting
structure is a bounded join-semilattice on the set of Kinds.

---

## 3. Dispatch Semantics

### 3.1 Filter Matching

Compound kinds match if **any** inner kind matches the filter:

```rust
impl Kind {
    pub fn matches(&self, filter: &Kind) -> bool {
        match (self, filter) {
            (Kind::Compound(kinds), _) => kinds.iter().any(|k| k.matches(filter)),
            (_, Kind::Compound(filters)) => filters.iter().any(|f| self.matches(f)),
            (a, b) => a == b,
        }
    }
}
```

Example:

```
Signal { kind: Compound([GateVerdict, TestResult]) }

  matches(Kind::GateVerdict)  => true
  matches(Kind::TestResult)   => true
  matches(Kind::Task)         => false
  matches(Compound([GateVerdict, TestResult]))  => true
```

A Compound signal appears in every query that any of its component kinds would appear
in. This is the lattice join: it is a supertype of each component kind for the purpose
of retrieval.

### 3.2 Trait Dispatch

Each consumer processes the kind(s) it recognizes and ignores the rest. Consumers must
not fail on unrecognized kinds within a Compound:

```
Gate "compile":
  Input: Compound([GateVerdict, CompileDiagnostic])
  Behavior: verify the CompileDiagnostic aspect, ignore the GateVerdict aspect
  (the GateVerdict is the output of verification, not the input)
```

The dispatch rule: each consumer processes the kind(s) it recognizes and ignores the
rest.

### 3.3 Scoring Algebra

The Scorer evaluates each component kind independently and takes the **maximum** score
across dimensions:

```rust
fn score_compound(signal: &Signal, kinds: &[Kind]) -> Score {
    let scores: Vec<Score> = kinds.iter()
        .filter_map(|k| score_for_kind(signal, k))
        .collect();

    Score {
        relevance:  scores.iter().map(|s| s.relevance).max_by(f32_cmp).unwrap_or(0.0),
        confidence: scores.iter().map(|s| s.confidence).max_by(f32_cmp).unwrap_or(0.0),
        urgency:    scores.iter().map(|s| s.urgency).max_by(f32_cmp).unwrap_or(0.0),
        novelty:    scores.iter().map(|s| s.novelty).max_by(f32_cmp).unwrap_or(0.0),
        salience:   scores.iter().map(|s| s.salience).max_by(f32_cmp).unwrap_or(0.0),
        coherence:  scores.iter().map(|s| s.coherence).max_by(f32_cmp).unwrap_or(0.0),
        surprise:   scores.iter().map(|s| s.surprise).max_by(f32_cmp).unwrap_or(0.0),
    }
}
```

The max-aggregation makes Compound signals at least as relevant as their most relevant
component. Alternative strategies are configurable:

| Strategy | Behavior |
|---|---|
| `"max"` (default) | Maximum across component scores per axis |
| `"mean"` | Average across component scores per axis |
| `"first"` | Use the score from the first (alphabetically sorted) component |

---

## 4. Serialization

Compound kinds serialize as a JSON object with an array value:

```json
{ "kind": { "compound": ["gate_verdict", "test_result"] } }
```

Single kinds serialize as before:

```json
{ "kind": "gate_verdict" }
```

The `as_str()` representation for display uses a joined form:
`"gate_verdict+test_result"` (components joined by `+` in sorted order).

Backwards compatibility: existing Signals with flat kinds deserialize without change.
The `#[serde(rename_all = "snake_case")]` and `#[non_exhaustive]` attributes handle
both forms.

---

## 5. Migration Path

### Phase 1: Add the Variant (non-breaking)

Add `Kind::Compound(Vec<Kind>)` to the `#[non_exhaustive]` enum. All existing match
expressions fall through to the wildcard arm. No code changes required in consumers.

### Phase 2: Add matches() and compound() Helpers

Update Substrate query filtering to use `Kind::matches()` instead of `==`. This is the
only breaking change -- any code that does `signal.kind == Kind::GateVerdict` must
switch to `signal.kind.matches(&Kind::GateVerdict)`.

Search for affected call sites:

```bash
grep -rn '\.kind ==' crates/ --include='*.rs' | grep -v target/
```

### Phase 3: Emit Compound Signals

| Current | Compound replacement |
|---|---|
| Two Signals: GateVerdict + TestResult | One Signal: Compound([GateVerdict, TestResult]) |
| Skill with tag `source=episode` | Compound([Skill, Episode]) |
| RouterFeedback with cost tags | Compound([RouterFeedback, Metric]) |

### Phase 4: Update Consumers

| Consumer | Change |
|---|---|
| Gate pipeline | Use `matches()` for verdict filtering |
| Cascade router | Use `matches()` for feedback filtering |
| Episode logger | Use `matches()` for episode filtering |
| Skill library | Use `matches()` for skill filtering |
| Dashboard | Display primary kind (first in sorted order) |

---

## 6. Limits and Anti-Patterns

### 6.1 Maximum Compound Size

```rust
impl Kind {
    const MAX_COMPOUND_SIZE: usize = 4;

    pub fn compound(kinds: impl IntoIterator<Item = Kind>) -> Result<Kind, KindError> {
        // ... flatten, dedup, sort ...
        if flat.len() > Self::MAX_COMPOUND_SIZE {
            return Err(KindError::CompoundTooLarge {
                size: flat.len(),
                max: Self::MAX_COMPOUND_SIZE,
            });
        }
        // ...
    }
}
```

### 6.2 Anti-Patterns

1. **Compound as a grab bag**: Do not create `Compound([Task, Plan, PlanPhase,
   Episode])`. If everything is everything, the type system provides no value.

2. **Compound for versioning**: Do not use `Compound([GateVerdict,
   Custom("gate_verdict_v2")])`. Use the existing variant with different body formats.

3. **Compound for metadata**: Do not use `Compound([Task, Metric])` just because the
   task carries a metric. Tags and body fields handle metadata.

4. **Custom inside Compound**: Compound kinds should use only the predefined variants.
   Custom kinds inside Compounds make dispatch unpredictable.

---

## 7. Configuration Parameters

| Parameter | Default | Range | Description |
|---|---|---|---|
| `max_compound_size` | 4 | 2 - 8 | Maximum kinds in a Compound |
| `compound_scoring_strategy` | "max" | "max", "mean", "first" | How to combine scores across kinds |
| `compound_display_strategy` | "first" | "first", "all", "primary" | How to display Compound kinds in UI |

---

## 8. Error Handling

| Condition | Response |
|---|---|
| Empty kinds iterator | Return `Kind::Custom("empty_compound")` |
| Single kind after dedup | Unwrap to that single kind (not a Compound) |
| Nested Compound | Flatten automatically |
| Exceeds max size | Return `Err(KindError::CompoundTooLarge)` |
| Custom kind inside Compound | Allow but log warning |

---

## 9. Test Criteria

1. `Kind::compound([GateVerdict, TestResult])` produces a sorted, deduplicated Compound
2. `Kind::compound([GateVerdict])` returns `Kind::GateVerdict` (unwrapped)
3. `Kind::compound([])` returns `Kind::Custom("empty_compound")`
4. Nested Compounds flatten: `compound([compound([A, B]), C])` equals `compound([A, B, C])`
5. `Compound([GateVerdict, TestResult]).matches(&Kind::GateVerdict)` returns true
6. `Compound([GateVerdict, TestResult]).matches(&Kind::Task)` returns false
7. Serde round-trip preserves Compound kinds
8. `as_str()` for Compound returns joined form: "gate_verdict+test_result"
9. Hash equality: same kinds in different insertion order produce same hash
10. Exceeding MAX_COMPOUND_SIZE returns an error

---

## Cross-References

- [00-ARCHITECTURE](../../00-ARCHITECTURE.md) -- Parent chapter
- `crates/roko-core/src/kind.rs` -- Current Kind enum
- `crates/roko-core/src/engram.rs` -- Signal struct with Kind field
- [design-principles-frontier-summary.md](design-principles-frontier-summary.md) -- P1 composition
