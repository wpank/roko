# Provenance and Attestation

> **v3 depth file** -- `/docs/v3/depth/00-architecture/provenance-and-attestation.md`
> Canonical source: v1 `docs/v1/00-architecture/05-provenance-and-attestation.md`
> Implementation: `crates/roko-core/src/engram.rs` (Provenance struct), `crates/roko-agent/src/safety/`
> Status: **Shipping** -- base Provenance (author, trust, session, taint) is on every Signal.
> Trust-origin IFC lattice with TaintTracker, five-layer immune Graph, and mandatory audited
> production hooks are live (E34 8/8). Typed taint enum and formal Custody records are
> target-state extensions.

---

## 1. Why Provenance Matters

Roko runs in adversarial and regulated environments. It ingests user input, tool output,
external fetches, plugin results, and its own prior knowledge. That means the system must
preserve more than content. It must preserve audit context.

At the architecture level, provenance answers four questions for every durable record:

1. **Who produced this Signal?** A user, agent, gate, plugin, chain source, or system role.
2. **How trusted was that producer at emission time?** Trust is a snapshot, not a live lookup.
3. **Is the record tainted, and why?** Taint is a safety signal, not a generic warning bit.
4. **What higher-assurance proof exists around this record?** A Custody chain, an
   Attestation, or both.

This is load-bearing for three reasons:

- **Safety:** tainted inputs must not silently flow into high-risk actions.
- **Auditability:** operators must be able to answer who did what, why, with what approval,
  and with what consequence.
- **Composability:** Score, Verify, Route, Compose, and React need a common,
  architecture-level vocabulary for trust and safety.

---

## 2. The Provenance Contract

Every Signal carries provenance. The architecture contract is stable: provenance is the
minimum durable audit context required to interpret the record safely after the fact.

```rust
pub struct Provenance {
    pub author: String,
    pub trust: f32,
    pub session: Option<String>,
    pub taint: Taint,
}
```

| Field | Meaning |
|---|---|
| `author` | Durable producer identity: user, agent, gate, plugin, system role, or external source |
| `trust` | Snapshot trust score at time of emission; later reputation changes do not rewrite history |
| `session` | Optional run/session grouping for replay, audits, and scoped queries |
| `taint` | Safety classification for whether the record originated from or depends on untrusted input |

Two architecture rules:

1. Provenance is part of the Signal's durable meaning. Two identical bodies from different
   authors or taint states are not the same audit record.
2. Provenance is not the whole safety story. It is the base layer that Custody, Attestation,
   and safety-focused Pulse streams build on top of.

---

## 3. Provenance as Durable Audit Context

The durable medium in Roko is the Signal (Engram struct), so the durable audit trail lives
there:

- A retrieved Signal from the Store is self-describing enough to evaluate safety without
  consulting the original runtime process.
- A reviewer can inspect lineage and see whether a record came from trusted gates, user input,
  plugin output, or external fetches.
- The runtime can attach stronger evidence, such as a Custody record or cryptographic
  Attestation, without mutating the historical meaning of the original record.

Three layers of audit evidence:

| Layer | Stored on | Purpose |
|---|---|---|
| `Provenance` | every Signal | minimum durable audit context |
| `Custody` | auditable-action Signals | why a privileged or externally visible action happened |
| `Attestation` | opt-in on selected Signal kinds | cryptographic proof of signer and integrity |

Architectural split:
- **Bus** is for live delivery, approvals, and safety telemetry.
- **Store** (Substrate) is for durable audit truth.

---

## 4. Taint Analysis

The architecture-level taint model is typed:

```rust
#[non_exhaustive]
enum Taint {
    Clean,
    LlmHallucination { detail: String },
    ToolFailure { detail: String },
    UserFlagged { detail: String },
    StaleData { threshold_ms: i64 },
    UnverifiedSource { detail: String },
    Propagated { detail: String, inherited_from: Option<ContentHash> },
    UserInput { detail: String },
    Custom(String),
}
```

| Variant | Meaning |
|---|---|
| `Clean` | No taint -- data from a trusted, verified source |
| `LlmHallucination` | LLM-generated content that may contain hallucinated facts |
| `ToolFailure` | A tool call failed or returned suspect data |
| `UserFlagged` | A human operator explicitly flagged this data |
| `StaleData` | Data has exceeded its freshness window |
| `UnverifiedSource` | Data from an unverified external source (API, webhook, chain) |
| `Propagated` | Taint inherited from upstream (tracks `inherited_from` hash) |
| `UserInput` | User-provided prompt, paste, file upload, or inline instruction |
| `Custom` | Application-specific taint reason |

### 4.1 Propagation Rules

Taint is one-way and conservative:

1. If a Composer reads any tainted Signal, the composed prompt Signal is tainted.
2. If an LLM turn or tool action consumes tainted input, its derived output stays tainted
   until explicitly reviewed and signed off.
3. If multiple taint sources contribute to one result, the output records the strongest
   relevant taint classification rather than silently collapsing to `None`.
4. Gate verdicts can validate claims about tainted inputs, but they do not erase the
   historical fact that tainted material participated in the decision path.
5. Clearing taint requires explicit human action recorded in the audit trail; it is not an
   automatic side effect of normal execution.

### 4.2 Taint and the Cognitive Immune System

The cognitive immune system consumes taint as an architectural input. Taint marks the
first-order source of concern; the immune system layers quarantine, anomaly detection,
re-verification, and attack-pattern memory on top of that base. The shipped safety layer
(E34 8/8 strict) implements:

- Trust-origin IFC lattice with TaintTracker
- Five-layer immune Graph
- Five-head corrigibility ordering
- Five-level sandbox policy
- Exact capability wrappers
- Persistent quarantine
- Mandatory audited production hooks
- Universal host-visible tool-result screening
- Provider isolation and tool cooldown/isolation

---

## 5. Custody Records for Auditable Actions

Not every Signal needs a full chain-of-custody record. But any action that changes external
state, performs a privileged operation, or needs compliance-grade review must emit a durable
Custody Signal.

```rust
Custody {
    action: ActionHash,
    principal: PrincipalId,
    when: Timestamp,
    authorized: AuthzEvidence,
    why_heuristics: Vec<HeuristicId>,
    why_claims: Vec<ClaimId>,
    simulation: Option<SimHash>,
    gates_passed: Vec<GateVerdict>,
    result: Option<ResultHash>,
    witness: Option<ChainWitness>,
}
```

| Field | Why it exists |
|---|---|
| `action` | Canonical identity for what was attempted or executed |
| `principal` | Who initiated: user, agent, plugin, or delegated role |
| `when` | Timestamp for replay and audit sequencing |
| `authorized` | Which role grant, confirmation, escalation, or session approval allowed it |
| `why_heuristics` | Which heuristics shaped the choice |
| `why_claims` | Which research-backed claims justified it |
| `simulation` | Optional dry-run or preflight evidence |
| `gates_passed` | Which verification steps approved the action |
| `result` | Durable pointer to the outcome |
| `witness` | Optional external witness, including chain witness |

Architecture rules for Custody:

- Custody is itself durable. It lives in the Store, not only in logs.
- Domain profiles decide which actions require custody, but destructive and externally
  visible actions are the default high-priority cases.
- Custody does not replace ordinary provenance; it augments it for actions that need deeper
  accountability.
- When present, Custody should be queryable independently of the original runtime.

---

## 6. Attestation

Attestation is the cryptographic layer on top of provenance. It proves that a specific
signer committed to a specific durable record.

```rust
Attestation {
    signer: PublicKey,
    signature: Ed25519Signature,
    signed_hash: ContentHash,
    timestamp: i64,
    level: AttestationLevel,
}

enum AttestationLevel {
    LocalAgent,
    OrgRole,
    ChainWitness,
}
```

### 6.1 Attestation Levels

| Level | Meaning | Typical use |
|---|---|---|
| `LocalAgent` | Signed by the current agent session key | Low-friction auditability for gate verdicts and local outputs |
| `OrgRole` | Signed by a human-owned organizational key | Destructive or externally visible actions requiring human sign-off |
| `ChainWitness` | Independently witnessed on-chain | Cross-deployment trust, later verification |

### 6.2 Attestation Rules

1. An attestation signs the `ContentHash`; it does not replace content addressing.
2. Attestation strengthens integrity and signer identity, but it does not erase taint.
3. Attestation and custody compose: a Custody Signal can itself be attested.

---

## 7. Provenance in the Seven-Step Loop

### Step 1: SENSE

- `Store.query()` returns provenance-bearing Signals.
- `Bus.subscribe()` returns live Pulses that may later graduate into Signals.
- External I/O enters as potential taint sources.

### Step 2: ASSESS

- Scorers and Routers may consult provenance and taint, not just semantic content.
- Tainted or weakly trusted records can be down-ranked, routed to a stronger gate path, or
  sent toward confirmation workflows.

### Step 3: COMPOSE

- Composer preserves taint lineage when assembling prompt Signals.
- Prompt assembly is where untrusted context can become dangerous, so provenance-aware
  composition is a core safety obligation.

### Step 4: ACT

- Actions consume the taint state of their inputs.
- High-risk actions use that state to require confirmation, deny execution, or escalate.
- If the action is auditable, this step allocates the Custody record.

### Step 5: VERIFY

- Verify implementations record which verdicts passed and feed those verdicts into Custody.
- Verification can validate an action without deleting the historical taint that fed it.
- Attestation commonly starts here for GateVerdict and approval-bearing outputs.

### Step 6: PERSIST and BROADCAST

- `Store.put()` persists the action result, verdict Signals, and any Custody record.
- `Bus.publish()` emits live Pulses such as `safety.*`, `network.egress.*`, or
  `gate.verdict.emitted`.
- Architectural invariant: Store holds durable audit truth while Bus delivers real-time
  visibility and reaction triggers.

### Step 7: REACT

- React implementations read the new safety evidence and decide follow-up actions:
  quarantine, approval request, replay, or escalation.

---

## 8. Provenance Across the Two Fabrics

### 8.1 Store Responsibilities

Store is where durable audit context lives:
- persisted Signal provenance
- Custody records
- attested records
- lineage needed for replay and incident response

### 8.2 Bus Responsibilities

Bus is where live safety coordination happens:
- approval prompts and confirmations
- `safety.*` notifications
- `network.egress.*` telemetry
- sandbox violations and gate verdict notifications

### 8.3 Why the Split Matters

The split prevents a common failure mode in agent systems: safety evidence exists only in
live logs or UI traces and disappears once the process exits. In Roko, live observation and
durable proof are separate but connected:

- Pulse enables immediate intervention.
- Signal enables later audit.
- Custody and Attestation lift selected actions to stronger guarantees.

---

## Cross-References

- `naming-and-glossary.md` -- Canonical terminology
- `substrate-trait.md` -- Durable storage fabric
- `bus-transport-fabric.md` -- Transport fabric
- `synapse-traits-12.md` -- Kernel trait overview
