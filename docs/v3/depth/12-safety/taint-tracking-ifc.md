# Taint Tracking and Information Flow Control

> **v3 depth file** -- `/docs/v3/depth/12-safety/taint-tracking-ifc.md`
> Canonical source: v1 `docs/v1/11-safety/03-taint-tracking.md`
> Status: **Complete** (E34 8/8). Trust-origin taint lattice is live via `TaintTracker`
> and `TrustOriginTaintLevel`. Monotonic propagation, durable persistence, and Signal
> observation are wired. Provider-owned internal trace Signals and broad adaptive immune
> memory remain product scope.

---

## 1. The Foundational Question

Taint tracking answers a basic question: which inputs are still untrusted?

If a prompt, fetch result, plugin output, or imported Signal influenced the current
action, that fact must remain visible until a reviewer explicitly signs off. Taint is:

- Attached when data crosses a trust boundary.
- Propagated through composition and action.
- Consulted by gates before high-risk effects.
- Persisted in provenance and the audit chain.

The model is intentionally one-way. Inputs can accumulate taint automatically; they
cannot become clean automatically. This asymmetry is not a conservative over-approximation
that could be relaxed later -- it is a structural invariant that prevents taint laundering
through summarization, multi-hop transformation, or time-based decay.

---

## 2. Information Flow Control: Denning's Lattice

Roko's taint system implements a classic information flow control (IFC) lattice as
described by Denning (1976) in "A Lattice Model of Secure Information Flow."

### 2.1 The Denning lattice model

Denning's formulation defines a lattice `(SC, <=, join)` where:

- `SC` is a finite set of security classes.
- `<=` is a partial order on `SC` (the "can flow to" relation).
- `join` is the least upper bound operator.

The key property: information can flow from class `A` to class `B` if and only if
`A <= B`. If information from classes `A` and `B` are combined, the result has class
`join(A, B)`.

The lattice structure provides three guarantees that a flat tag set cannot:

1. **Composability.** The join of any two classes is well-defined and unique. There is
   no ambiguity about the taint level of composed data.
2. **Monotonicity.** Information can only flow upward. A computation cannot produce an
   output cleaner than its dirtiest input.
3. **Decidability.** Every flow decision reduces to a comparison in the partial order.
   No heuristic judgment is needed.

### 2.2 Roko's trust-origin lattice

The `TrustOriginTaintLevel` enum in `roko-core/src/provenance.rs` instantiates a
four-element totally ordered lattice:

```rust
pub enum TrustOriginTaintLevel {
    /// Produced by trusted, verified internal computation.
    Trusted,
    /// Produced locally but from user-supplied or unverified input.
    Local,
    /// Received from an external source (API, webhook, fetch).
    External,
    /// From an adversarial, unverified, or actively suspicious source.
    Untrusted,
}
```

The lattice ordering is:

```
Trusted            (bottom / clean)
    |
Local
    |
External
    |
Untrusted          (top / most tainted)
```

### 2.3 The join operator

The join operation computes the least upper bound:

```rust
impl TrustOriginTaintLevel {
    pub fn join(self, other: Self) -> Self {
        match (self, other) {
            (Self::Untrusted, _) | (_, Self::Untrusted) => Self::Untrusted,
            (Self::External, _) | (_, Self::External) => Self::External,
            (Self::Local, _) | (_, Self::Local) => Self::Local,
            _ => Self::Trusted,
        }
    }
}
```

This is exactly Denning's join: combining Trusted data with External data produces
External data. Combining anything with Untrusted produces Untrusted.

### 2.4 Lattice properties (formal)

The lattice satisfies four properties that the implementation relies on:

- **Commutativity:** `join(a, b) = join(b, a)` -- the order of inputs does not
  matter. Verified by the match arm symmetry.
- **Associativity:** `join(join(a, b), c) = join(a, join(b, c))` -- multi-input
  composition is order-independent. This permits `fold`-based propagation.
- **Idempotence:** `join(a, a) = a` -- marking a datum with its own taint is a no-op.
- **Monotonicity:** if `a <= b` then `join(a, c) <= join(b, c)` -- substituting a
  dirtier input never produces a cleaner output.

The bottom element is `Trusted` (identity for join). The top element is `Untrusted`
(absorbing element for join).

### 2.5 Why four levels, not more

The four-level lattice is chosen for cognitive and operational simplicity:

| Level | Operational meaning | Source examples |
|---|---|---|
| Trusted | System-generated, internally verified | Gate verdicts, kernel computations |
| Local | Locally authored, not independently verified | User prompts, manual edits |
| External | Crossed the network boundary | API responses, web fetches |
| Untrusted | Adversarial or unknown origin | Error messages, third-party plugins |

A finer lattice (e.g., per-provider or per-tenant levels) could express more nuance but
would increase the cost of every join decision and make operator reasoning harder.
Domain-specific refinement is handled by `TaintReason` labels, not by lattice expansion.

---

## 3. The TaintTracker

The `TaintTracker` in `roko-agent/src/safety/taint_propagation.rs` is the runtime
enforcement point for the lattice. It is a thread-safe, content-addressed map from
signal hashes to `(TaintLevel, TaintReason, Vec<ContentHash>)` triples.

```rust
pub struct TaintTracker {
    state: Mutex<TrackerState>,
}

struct TrackerState {
    taints: HashMap<ContentHash, TaintEntry>,
    audit: Vec<PropagationAudit>,
}

struct TaintEntry {
    level: TaintLevel,
    reason: TaintReason,
    derived_from: Vec<ContentHash>,
}
```

### 3.1 Monotonic marking

`mark_tainted()` never lowers an existing level -- it uses the lattice join:

```rust
pub fn mark_tainted(&self, hash: ContentHash, reason: TaintReason, level: TaintLevel) {
    let mut state = self.state.lock();
    state.taints
        .entry(hash)
        .and_modify(|entry| {
            entry.level = entry.level.join(level);  // Monotonic: only goes up
            entry.reason = reason.clone();
        })
        .or_insert(TaintEntry { level, reason, derived_from: Vec::new() });
}
```

If a hash is already at `External` and a new mark arrives at `Local`, the level stays
`External`. This preserves the lattice monotonicity invariant.

### 3.2 Propagation

`propagate()` computes the join of all tracked parent levels:

```rust
pub fn propagate(&self, parents: &[ContentHash], child: ContentHash) -> bool {
    // ...
    let inherited = tainted_parents.iter()
        .map(|(_, level)| *level)
        .fold(TaintLevel::Trusted, TaintLevel::join);
    // ...
    entry.level = entry.level.join(inherited);
    // ...
}
```

Clean parents (not in the tracker) do not create taint. This prevents phantom taint
from materializing through propagation alone. The fold over `TaintLevel::join` is
correct because `join` is associative and commutative, and `Trusted` is the identity.

### 3.3 Signal observation

`observe_signal()` registers a Signal from its provenance trust decision:

```rust
pub fn observe_signal(&self, signal: &Signal) -> bool {
    let level = signal.effective_taint();
    // Map Taint variants to TaintReason:
    // Taint::UserInput -> TaintReason::user_input
    // Taint::UnverifiedSource -> TaintReason::external
    // Taint::ToolFailure -> TaintReason::tool_failure
    // Taint::StaleData -> TaintReason::stale
}
```

This bridges the Signal-level provenance model (which carries `Taint` variants) to the
tracker-level lattice model (which carries `TaintLevel` values).

### 3.4 Querying taint

`get_taint()` returns the current level for a hash, or `None` if the hash is not
tracked (implicitly `Trusted`):

```rust
pub fn get_taint(&self, hash: &ContentHash) -> Option<TaintLevel> {
    self.state.lock().taints.get(hash).map(|entry| entry.level)
}
```

The `None` case is important: absence means "not tainted," not "unknown." This
simplifies gate logic that needs to distinguish between "clean" and "tracked but
at Trusted level."

---

## 4. NeuroTaint: Neural Taint Analysis

Roko's taint tracking can be compared with the NeuroTaint approach (Chen et al. 2025,
arXiv:2604.23374), which introduces neural-network-assisted taint analysis for code
understanding. Where traditional taint analysis requires explicit source/sink annotations,
NeuroTaint leverages LLM understanding of code semantics to identify implicit information
flows that static analysis misses.

### 4.1 Classical vs. neural taint

| Dimension | Classical taint analysis | NeuroTaint | Roko cognitive taint |
|---|---|---|---|
| Domain | Program variables | Code semantics | Agent reasoning chain |
| Granularity | Per-variable | Per-expression | Per-Signal |
| Propagation | Data flow graph | Neural inference | Lineage DAG |
| Implicit flows | Missed | Detected via LLM | Conservative (join) |
| Overhead | Compile-time | Inference-time | Runtime (O(1) per join) |

### 4.2 Adapting the insight

Roko adapts the NeuroTaint insight at the agent level: instead of analyzing code taint,
Roko tracks cognitive taint -- the flow of trust-relevant information through the agent's
reasoning chain. The `TaintTracker` serves as a neural-aware (in the sense of understanding
the LLM's information mixing) taint propagation engine:

- Classical taint analysis: marks variables, propagates through data flow.
- Roko cognitive taint: marks Signals, propagates through lineage DAG.

The key parallel: both systems must handle implicit flows (information leakage through
control flow, summarization, or cross-reference) that a naive source/sink model would
miss. Roko's solution is conservative -- the join lattice assumes any composition with
tainted input produces tainted output, even if the specific information path is benign.

### 4.3 CaMeL taint integration

The CaMeL framework (Debenedetti et al. 2025; enterprise hardening arXiv:2505.22852)
defines a capability-aware taint model with explicit data/control flow separation. Roko's
`CamelTaintLevel` type references this work, and the dispatcher carries CaMeL tags through
delegation chains. The correspondence is:

| CaMeL concept | Roko implementation |
|---|---|
| Data taint (untrusted input) | `TrustOriginTaintLevel::External` or `Untrusted` |
| Control taint (influenced decision) | `TaintReason::Propagated` via lineage |
| Taint sanitizer | Explicit human review (Section 8) |
| Capability check | `SandboxPolicy` + `AgentWarrant` |

---

## 5. Taint Reason Taxonomy

The `TaintReason` enum records why a hash is tracked, independent of its lattice level:

```rust
pub enum TaintReason {
    ExternalSource { detail: String },
    UserInput { detail: String },
    ToolFailure { detail: String },
    Propagated,
    Stale { detail: String },
    Custom { category: String, detail: String },
}
```

The taxonomy is about origin, not moral judgment. Taint means "treat carefully," not
"discard."

| Reason | When assigned | Lattice level |
|---|---|---|
| `ExternalSource` | Data crosses the network boundary | External |
| `UserInput` | Human provides unvalidated content | Local |
| `ToolFailure` | Tool exits with an error | Local or External |
| `Propagated` | Inherited from tainted parent(s) | join of parents |
| `Stale` | Data older than configured threshold | Local |
| `Custom` | Domain-specific taint (e.g., chain data) | Varies |

---

## 6. TaintedString: Labeled Information Flow

The `TaintedString` type (`roko-agent/src/safety/hooks.rs`) carries sensitivity labels
that control which sinks may receive the value:

```rust
pub struct TaintedString {
    value: Vec<u8>,          // Zeroed on drop
    labels: HashSet<TaintLabel>,
}
```

Five labels map to flow rules:

| Label | LlmContext | EventBus | CollectiveMesh |
|---|---|---|---|
| `WalletSecret` | BLOCKED | BLOCKED | BLOCKED |
| `OwnerSecret` | BLOCKED | Allowed | BLOCKED |
| `StrategyConfidential` | Allowed | Allowed | BLOCKED |
| `UserPII` | Allowed | Allowed | BLOCKED |
| `UntrustedExternal` | Allowed | Allowed | Allowed |

`TaintedString` bytes are overwritten with zeroes when the value is dropped, preventing
secret material from lingering in freed memory. This implements the zeroize-on-drop
pattern from Vaucher et al. (2018).

The label system is orthogonal to the trust-origin lattice: labels control where data
may flow (sink policy), while the lattice controls how much to trust it (decision policy).
A datum can be `WalletSecret` (blocked from all sinks) at `Trusted` level (produced
internally), or `UntrustedExternal` (flows freely) at `Untrusted` level (do not act on
it without review).

---

## 7. Taint Through the Processing Loop

| Step | Taint behavior |
|---|---|
| SENSE | Attach taint via `observe_signal()` from provenance |
| ASSESS | Use taint level as routing and scoring input |
| COMPOSE | Propagate taint through composition via `propagate()` |
| ACT | Read taint before tool use, egress, signing |
| VERIFY | Gates can deny or escalate based on taint at high-risk destinations |
| PERSIST | Persist taint in Signal provenance |
| REACT | Open quarantine incidents when taint reaches blocked destinations |

The propagation at COMPOSE is particularly important because it is where the lattice
join does most of its work. A prompt assembled from five context sections, one of which
is External, produces a tainted prompt at External level. The LLM completion derived
from that prompt inherits the taint. A summary of the completion inherits it again.
Taint does not attenuate through the composition chain.

---

## 8. Review as the Only Cleaning Action

Taint does not disappear because time passed or because another model summarized the
input. The only legitimate route from tainted to trusted is explicit review that records:

- Who reviewed it.
- What scope they approved.
- When they approved it.
- Which resulting Signal or action the approval covers.

Without that durable record, "cleaning" taint is indistinguishable from ignoring it.

This is more conservative than most IFC systems, which allow declassification by
authorized principals. Roko's position is that in the agent context, the cost of a
false declassification (acting on unreviewed adversarial input) far exceeds the cost
of a false alarm (requiring review of benign input). The asymmetry is deliberate.

---

## 9. High-Risk Destinations

Taint matters most at costly-to-undo or hard-to-audit destinations:

- Signing a chain transaction.
- Sending data to an external API.
- Writing to production infrastructure.
- Publishing a pull request.
- Persisting a heuristic as trusted knowledge.

Expected policy behavior:

| Taint level | Low-risk action | Medium-risk action | High-risk action |
|---|---|---|---|
| Trusted | Allow | Allow | Allow |
| Local | Allow (record) | Confirm checkpoint | Review checkpoint |
| External | Allow (record) | Review checkpoint | Escalate |
| Untrusted | Record + alert | Escalate | Fail closed |

A good example is a chain action with a recipient address extracted from `ExternalFetch`.
The correct default is not "try and log it"; it is "escalate until reviewed."

---

## 10. Taint vs. Secret Handling

Taint and secret handling overlap but are not the same:

| Concern | Taint | Secrets |
|---|---|---|
| Purpose | Track trust and origin | Track sensitivity and redaction |
| Example: tainted, not secret | Web page content | -- |
| Example: secret, not tainted | Locally stored credential | -- |
| Example: both | Pasted credential from user | -- |

The safety spine uses both: taint to decide whether an action may proceed, scrubbing
and secret types to prevent disclosure. This distinction avoids a common failure mode
where secret redaction exists but the system still takes high-risk actions on unreviewed
external data.

---

## 11. Durable Persistence

The `TaintTracker` supports atomic save/load for restart durability:

```rust
pub fn save(&self, path: impl AsRef<Path>) -> io::Result<()> {
    // Atomic write via temporary file + rename
    let snapshot = self.state.lock().to_snapshot();
    // ...
    fs::rename(&temporary, path)?;
}

pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
    // Returns empty tracker if file does not exist
    let snapshot: TaintTrackerSnapshot = serde_json::from_slice(&bytes)?;
    Ok(Self { state: Mutex::new(TrackerState::from_snapshot(snapshot)) })
}
```

The snapshot captures all taint entries and the propagation audit log. A 4 MiB size
limit prevents unbounded growth. If the snapshot exceeds the limit, older audit entries
are evicted first, preserving the taint entries themselves.

---

## 12. Propagation Audit Trail

Every successful propagation produces a `PropagationAudit`:

```rust
pub struct PropagationAudit {
    pub hash: ContentHash,
    pub parents: Vec<ContentHash>,
    pub resulting_level: TaintLevel,
}
```

The audit log is queryable via `audit_log()` and persisted as part of the tracker
snapshot. This enables forensic reconstruction of exactly how taint reached any
given Signal -- a capability that the v3 chapter's immune system and the forensic-ai
depth file depend on.

---

## 13. The Distinction from Data Classification

Trust-origin taint is explicitly distinct from the established data-classification
lattice. Data classification (Confidential, Internal, Public) answers "who may see
this data." Trust-origin taint answers "how much should I trust this data when making
decisions." A datum can be Public (anyone may see it) but Untrusted (do not act on it
without review). Conversely, a datum can be Confidential (restricted audience) but
Trusted (produced by verified internal computation).

This distinction matters because many IFC failures come from conflating the two: a
system that treats "public" as "safe to act on" or "confidential" as "untrusted" will
make systematically wrong decisions in both directions.

---

## 14. Relation to the Immune System

The five-layer immune decision Graph (see `defense-in-depth.md` Section 7) uses taint
as its primary input signal:

- **Layer 1 (Taint Propagation)**: computes derived taint via the join lattice.
- **Layer 2 (Anomaly Detection)**: uses taint fanout bursts as a danger signal.
- **Layer 3 (Quarantine Gate)**: blocks actions based on taint level at destination.
- **Layer 4 (Incident Response)**: creates durable quarantine incidents.
- **Layer 5 (Immune Memory)**: feeds back learned patterns to taint recognition.

The taint tracker is therefore the first stage of the immune pipeline, not an
independent subsystem.

---

## 15. Goguen-Meseguer Non-Interference

The lattice model satisfies the non-interference property from Goguen and Meseguer
(1982). Informally: an observer at level L cannot distinguish between two system states
that differ only in information at levels above L. In Roko's context: a gate checking
at `Local` level cannot observe a difference caused by `External` or `Untrusted` inputs
alone -- it sees only the taint level, not the content. The content is opaque; the taint
metadata is the decision input.

This property is important because it means the taint system cannot be used to
exfiltrate information about the content of tainted data. The gate knows "this is
External" but not "this is a specific external URL." That distinction prevents the
taint layer itself from becoming an information leak.

---

## Academic References

| Paper | Contribution |
|---|---|
| Denning (1976), "A Lattice Model of Secure Information Flow" | Foundational IFC lattice |
| Chen et al. (2025, arXiv:2604.23374), "NeuroTaint" | Neural-assisted taint analysis |
| Goguen & Meseguer (1982), "Security Policies and Security Models" | Non-interference |
| Myers & Liskov (1997), "A Decentralized Model for Information Flow Control" | Decentralized IFC (JFlow/Jif) |
| Sabelfeld & Myers (2003), "Language-Based Information-Flow Security" | Comprehensive IFC survey |
| Debenedetti et al. (2025), "CaMeL" | Capability-aware taint for LLM agents |
| Vaucher et al. (2018), "Zeroization patterns" | Secure memory clearing |

---

## Implementation References

| Component | Location |
|---|---|
| TrustOriginTaintLevel | `crates/roko-core/src/provenance.rs` |
| TaintTracker | `crates/roko-agent/src/safety/taint_propagation.rs` |
| TaintReason | `crates/roko-agent/src/safety/taint_propagation.rs` |
| PropagationAudit | `crates/roko-agent/src/safety/taint_propagation.rs` |
| TaintedString | `crates/roko-agent/src/safety/hooks.rs` |
| Signal.provenance.taint | `crates/roko-core/src/provenance.rs` |
| Taint enum | `crates/roko-core/src/provenance.rs` |
| CamelTaintLevel | `crates/roko-agent/src/safety/hooks.rs` |
