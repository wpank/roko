# The Nine Protocol IDs

> Every Cell may declare zero or more of nine canonical `ProtocolId` values.
> These are the verbs of the system -- the nine kinds of work a Cell can
> perform.

**Source**: `crates/roko-core/src/cell.rs` (ProtocolId enum),
`crates/roko-core/src/traits.rs` (protocol traits)

---

## 1. The Enum

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProtocolId {
    Store,     // Persisted signal storage
    Score,     // Signal scoring
    Verify,    // Gate / verification
    Route,     // Signal routing
    Compose,   // Prompt / signal composition
    React,     // Reactive policy
    Observe,   // Passive observation emitter
    Connect,   // External connection lifecycle
    Trigger,   // Event-driven trigger
}
```

Cells declare their protocols via `Cell::protocols() -> Vec<ProtocolId>`.
The engine and introspection tools use this for dispatch, validation,
and dashboard rendering.

---

## 2. Store (Substrate)

**Purpose**: Persisted storage of Signals.

**Trait**: `roko_core::traits::Store` (alias of `Substrate`)

**Contract**:
```rust
#[async_trait]
pub trait Store: Send + Sync {
    async fn put(&self, signal: Signal) -> Result<ContentHash>;
    async fn get(&self, id: &ContentHash) -> Result<Option<Signal>>;
    async fn query(&self, q: &Query, ctx: &Context) -> Result<Vec<Signal>>;
    async fn query_similar(&self, fp: &HdcVector, radius: f32, limit: usize, ctx: &Context)
        -> Result<Vec<(ContentHash, f32)>>;
    async fn prune(&self, threshold: f32, ctx: &Context) -> Result<usize>;
    async fn len(&self) -> Result<usize>;
    async fn is_empty(&self) -> Result<bool>;
    fn name(&self) -> &'static str;
}
```

**Key properties**: `put` is idempotent on content hash. Stores are
`Send + Sync` and handle concurrent access internally.

**Implementations**:

| Crate | Struct | Storage |
|---|---|---|
| `roko-fs` | `FileSubstrate` | JSONL files (`.roko/signals.jsonl`) |
| `roko-core` | `MemorySubstrate` | In-memory `HashMap` (testing) |

**Cold variant**: `ColdStore` trait for archival. `FileSubstrate` archives
aged signals before pruning hot data.

---

## 3. Score

**Purpose**: Rate a Signal along multiple quality dimensions.

**Trait**: `roko_core::traits::Score`

**Contract**:
```rust
#[async_trait]
pub trait Score: Cell {
    async fn score(&self, signal: &Signal, ctx: &Context) -> Result<ScoreValue>;
}
```

Score Cells produce a `ScoreValue` (the 7-axis `Score` struct) for a
given Signal. They are learners -- they predict quality, publish
predictions, and receive corrections via the predict-correct lifecycle.

**Implementations**:

| Crate | Struct | Scoring method |
|---|---|---|
| `roko-learn` | `QualityJudge` | 5-dimensional assessment |
| `roko-learn` | `BayesianConfidence` | Bayesian evidence accumulation |
| `roko-std` | `DefaultScorer` | Confidence-from-provenance heuristic |

---

## 4. Verify

**Purpose**: The load-bearing protocol. Verification gates that determine
whether work products meet quality standards.

**Trait**: `roko_core::traits::Verify`

**Contract**:
```rust
#[async_trait]
pub trait Verify: Cell {
    async fn verify_pre(&self, input: &Signal, ctx: &Context) -> Result<Verdict>;
    async fn verify_post(&self, output: &Signal, ctx: &Context) -> Result<Verdict>;
}
```

Verify Cells serve four simultaneous roles:

1. **Reward function**: continuous `Verdict.reward` feeds routing decisions
2. **Relabeling oracle**: hindsight relabeling of failed trajectories
3. **Safety boundary**: `verify_pre` can veto execution before it begins
4. **Economic attestation**: reputation flows from verified work

**Implementations**:

| Crate | Struct(s) | Purpose |
|---|---|---|
| `roko-gate` | 19 gate rungs | Compile, test, clippy, diff, etc. |
| `roko-graph` | `PlanGateCell` | Runs gate rungs via `SharedGateEvaluator` |
| `roko-graph` | 5 corrigibility cells | Deference, Switch, Truth, Impact, Task |
| `roko-graph` | 5 immune cells | Perception, Assessment, Containment, Validation, Escalation |

---

## 5. Route

**Purpose**: Choose among candidates or next actions.

**Trait**: `roko_core::traits::Route`

**Contract**:
```rust
#[async_trait]
pub trait Route: Cell {
    async fn route(
        &self,
        candidates: &[Signal],
        selection: &Selection,
        ctx: &Context,
    ) -> Result<Vec<Signal>>;
}
```

Route Cells select which model to call, which provider to use, which
tool to run, or which plan branch to pursue.

**Implementations**:

| Crate | Struct | Routing method |
|---|---|---|
| `roko-learn` | `CascadeRouter` | Persisted bandit statistics + EFE |
| `roko-core` | `DefaultRouter` | First-match from candidates |

---

## 6. Compose

**Purpose**: Assemble bounded artifacts from multiple ingredients.

**Trait**: `roko_core::traits::Compose`

**Contract**:
```rust
#[async_trait]
pub trait Compose: Cell {
    async fn compose(
        &self,
        ingredients: &[Signal],
        budget: &Budget,
        ctx: &Context,
    ) -> Result<Signal>;
}
```

The canonical example is prompt construction: retrieve relevant Signals,
rank and filter, assemble under a token budget.

**Implementations**:

| Crate | Struct | Purpose |
|---|---|---|
| `roko-graph` | `ComposeCell` | Template variable substitution |
| `roko-graph` | `PlanComposeCell` | Fan-in of 6 enrichers + task context |
| `roko-graph` | `CognitiveComposeCell` | Cognitive loop prompt assembly |

---

## 7. React

**Purpose**: Watch activity and decide whether to intervene.

**Trait**: `roko_core::traits::React`

**Contract**:
```rust
#[async_trait]
pub trait React: Cell {
    fn subscription(&self) -> TopicFilter;
    async fn decide(&self, pulses: &[Pulse], ctx: &Context) -> Result<PolicyOutputs>;
}
```

React Cells consume Pulse streams and produce `PolicyOutputs` with new
Pulses (immediate reactions) and graduated Signals (durable decisions).

**Implementations**:

| Crate | Struct | Purpose |
|---|---|---|
| `roko-graph` | `GraduationCell` | Promote qualifying Pulses to Signals |
| `roko-graph` | `ReactCell` | Cognitive loop maintenance and events |

---

## 8. Observe

**Purpose**: Passive observation without modification.

**Trait**: `roko_core::traits::Observe`

**Contract**:
```rust
#[async_trait]
pub trait Observe: Cell {
    async fn observe(&self, datum: &Datum, ctx: &Context) -> Result<Outcome>;
}
```

Observe Cells read Signals from the Store and Pulses from the Bus
without modifying them, emitting observation records for telemetry.

**Implementations**:

| Crate | Struct | Purpose |
|---|---|---|
| `roko-graph` | `SenseCell` | Cognitive loop: detect new Signals/Pulses |

---

## 9. Connect

**Purpose**: External connection lifecycle management.

**Trait**: `roko_core::traits::Connect`

**Contract**:
```rust
#[async_trait]
pub trait Connect: Cell {
    async fn connect(&self, ctx: &Context) -> Result<()>;
    async fn disconnect(&self, ctx: &Context) -> Result<()>;
    async fn is_connected(&self) -> bool;
}
```

Connect Cells handle establishing, maintaining, and tearing down
connections to external systems (HTTP endpoints, WebSockets, chain
RPCs, peer agents).

The `Connect` trait coexists with the transport-neutral connector API
in `roko-core::connector`.

---

## 10. Trigger

**Purpose**: Event-driven activation.

**Trait**: `roko_core::traits::Trigger`

**Contract**:
```rust
#[async_trait]
pub trait Trigger: Cell {
    async fn should_fire(&self, ctx: &Context) -> Result<bool>;
    async fn fire(&self, ctx: &Context) -> Result<Vec<Signal>>;
}
```

Trigger Cells fire when specific conditions are met: cron schedules,
filesystem changes, chain events, webhook arrivals, Bus Pulses, or
manual invocation.

The `Trigger` trait coexists with the declarative trigger runtime
(E31, 7 source types) which binds conditions to Graph execution.

---

## 11. Protocol Composition

A single Cell can declare multiple protocols. For example, a gate cell
that also emits metrics might declare `[ProtocolId::Verify, ProtocolId::Observe]`.

The engine does not enforce that a Cell declaring `ProtocolId::Verify`
actually implements the `Verify` trait. Protocol IDs are metadata for
introspection and routing, not compile-time constraints (Rust does not
support trait-level runtime dispatch from an enum value).

---

## 12. Verification Commands

```bash
# Confirm exactly 9 ProtocolId variants
grep -c '^\s\+[A-Z][a-z]' crates/roko-core/src/cell.rs | head -1

# List all protocol trait definitions
grep 'pub trait' crates/roko-core/src/traits.rs

# Find all cells declaring specific protocols
grep -rn 'ProtocolId::Verify' crates/ --include='*.rs' | grep -v target/ | grep -v test
```

---

## 13. Source Files

| File | Contents |
|---|---|
| `crates/roko-core/src/cell.rs` | ProtocolId enum |
| `crates/roko-core/src/traits.rs` | Store, Score, Verify, Route, Compose, React, Observe, Connect, Trigger traits |
| `crates/roko-graph/src/cells/` | Concrete Cell implementations declaring ProtocolIds |
| `crates/roko-gate/src/` | 19 gate rung implementations (Verify protocol) |
| `crates/roko-learn/src/` | CascadeRouter (Route), QualityJudge (Score) |
