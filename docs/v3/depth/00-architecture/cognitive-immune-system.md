# Cognitive Immune System

> **v3 depth file** -- `/docs/v3/depth/00-architecture/cognitive-immune-system.md`
> Canonical source: v1 `docs/v1/00-architecture/26-cognitive-immune-system.md`
> Status: **Complete (E34 8/8 strict)** -- Trust-origin IFC, 5-stage immune Graph, five-head
> corrigibility, sandbox/process policy, mandatory audited hooks, provider isolation, tool
> cooldown/isolation, and linked incidents are live. Provider-owned internals, trace Signals,
> adaptive semantic immune memory, and externally anchored whole-ledger authenticity remain
> product residuals.

---

## 1. Threat Model

### 1.1 Attack Surfaces

The CIS exists because knowledge corruption is usually indirect: hostile input enters through
one surface, mutates a durable Signal somewhere else, and only becomes visible when a later
action is about to cross a real-world boundary.

| Vector | Example | Why CIS Cares | Primary Response |
|---|---|---|---|
| Prompt injection | Tool output includes instructions that alter later tool use | Can taint composed prompts and derived outputs | Mark taint at ingestion, require stronger gates at ACT |
| Memory poisoning | False claim survives into durable Neuro state | Contaminates later retrieval and planning | Quarantine, replay, reviewer release only |
| Adversarial retrieval | Crafted Signals score high despite low integrity | Pulls the Composer toward bad context | Contradiction and score-distribution checks |
| Plugin abuse | Third-party plugin output exceeds declared capability intent | Untrusted output can steer later actions | `ThirdPartyPlugin` taint, sandbox violation handling |
| Legacy import corruption | Imported archives contain stale or forged lineage | Pollutes trusted local state | `LegacyImport` taint plus quarantine-on-ingest |
| Cross-tenant contamination | Shared deployment leaks a tenant's data into another | Breaks isolation and audit boundaries | Immediate quarantine and escalation |

### 1.2 Scope Boundary: Safety Spine vs. CIS

The defensive story is clearer when the responsibilities are split explicitly:

| Concern | Primary owner | CIS role |
|---|---|---|
| Who may act | Safety spine authorization | Consume role and approval outcomes as evidence |
| Where untrusted code may run | Safety spine sandbox tiers | Treat violations as high-severity findings |
| What entered the system | CIS taint model | Label, propagate, and expose risk to gates |
| What must be isolated | CIS quarantine + Substrate filtering | Remove suspect Signals from default query paths |
| What happened and why | Custody and attestation | Attach findings, taint sources, and replay evidence |
| How the system learns after failure | CIS immune memory + policy updates | Turn incidents into future defenses |

---

## 2. Architecture: Five Defense Layers

The E34 implementation realized the five-layer architecture with the 5-stage immune decision
Graph in `roko-graph`:

```
+-----------------------------------------------------------+
| Layer 5: IMMUNE MEMORY + DELTA PROBES                     |
| Remember prior attacks; replay and probe weak points      |
+-----------------------------------------------------------+
| Layer 4: CUSTODY-LINKED INCIDENT RESPONSE                 |
| Tie findings to Custody, replay, and postmortems          |
+-----------------------------------------------------------+
| Layer 3: QUARANTINE                                       |
| Remove suspect Signals from default retrieval paths       |
+-----------------------------------------------------------+
| Layer 2: ANOMALY DETECTION                                |
| Detect contradiction clusters, fan-out, and drift         |
+-----------------------------------------------------------+
| Layer 1: TAINT PROPAGATION                                |
| Track untrusted lineage through Signals and Pulses        |
+-----------------------------------------------------------+
```

Layer 1 is the fast path. Layers 2 and 3 contain suspect knowledge before it reaches action.
Layers 4 and 5 make the system auditable and self-improving.

### 2.1 Implementation in roko-graph

The immune decision Graph (`crates/roko-graph/src/cells/immune.rs`) implements the five
layers as Graph cells:

- **Taint Cell**: Evaluates trust-origin lattice for incoming Signals
- **Anomaly Cell**: Detects contradiction bursts and score distribution anomalies
- **Quarantine Cell**: Manages the quarantine partition in Substrate
- **Incident Cell**: Links findings to custody records
- **Memory Cell**: Stores HDC fingerprints of known attack patterns

The `tool_immune.rs` and `immune_boundary.rs` modules in `roko-agent` provide the
host-visible tool result screening that traverses the five-stage immune Graph.

### 2.2 Biological Analogy

The CIS design draws from Artificial Immune Systems (AIS) research:

> "The immune system is a distributed, adaptive system that provides a natural intrusion
> detection mechanism." -- de Castro & Timmis, *Artificial Immune Systems: A New
> Computational Intelligence Approach* (2002), p. 57.

| Biological concept | CIS analogue | de Castro & Timmis reference |
|---|---|---|
| Innate immunity | Immediate tainting and quarantine | Ch. 2: pattern recognition receptors |
| Adaptive immunity | HDC-backed immune memory and replay | Ch. 4: clonal selection |
| Danger signals | Contradiction and lineage-integrity indicators | Ch. 3: co-stimulation |
| Memory cells | Stored postmortems, false positives, proven defenses | Ch. 5: immune memory |

The "danger model" perspective from Matzinger (2002) is particularly relevant:

> "The immune system does not care about self and nonself; it cares about danger."
> -- Polly Matzinger, "The Danger Model: A Renewed Sense of Self," *Science* 296:301-305 (2002).

This insight shapes the CIS design: the system responds when it detects signs of damage
(contradiction bursts, lineage gaps, sandbox violations), not merely when content is foreign.
A well-formed input from an untrusted source that passes all integrity checks is not
automatically quarantined. A trusted source that suddenly produces contradictory outputs is.

### 2.3 Core Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Taint {
    None,
    UserInput,
    ExternalFetch(Source),
    ThirdPartyPlugin(PluginId),
    LegacyImport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThreatClass {
    PromptInjection,
    MemoryPoisoning,
    TaintCascade,
    AdversarialRetrieval,
    SandboxViolation,
    CrossTenantLeakage,
    LineageMismatch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatFinding {
    pub id: Uuid,
    pub class: ThreatClass,
    pub affected_signals: Vec<ContentHash>,
    pub taint_sources: Vec<ContentHash>,
    pub confidence: f64,
    pub severity: f64,
    pub recommended_action: ContainmentAction,
    pub custody: Option<ContentHash>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ContainmentAction {
    Monitor,
    Quarantine,
    Reverify,
    Escalate,
    DisablePlugin,
}
```

---

## 3. Layer 1: Taint Propagation Tracking

### 3.1 Propagation Law

The propagation rule is intentionally simple:

1. If any Signal or Pulse consumed during composition is tainted, the derived output is tainted.
2. Pulses carry taint metadata while in motion on the Bus; when a Pulse graduates to a Signal,
   that taint becomes durable provenance.
3. Taint is monotonic. It propagates forward through lineage and does not silently disappear.
4. Human review can approve later use, but approval is recorded through Custody or attestation;
   it does not rewrite ancestor provenance.

```rust
pub fn derive_taint(inputs: &[Taint]) -> Taint {
    inputs.iter()
        .find(|t| !matches!(t, Taint::None))
        .cloned()
        .unwrap_or(Taint::None)
}
```

### 3.2 E34 Implementation

The E34 implementation extends taint tracking with:
- **Trust-origin lattice**: `TaintTracker` maintains transitive taint across lineage
- **Workspace-rooted authority**: Controls persist outside disposable attempt worktrees
- **Provider isolation**: Each provider session runs in bounded isolation
- **Tool cooldown/isolation**: Prevents rapid tool abuse patterns
- **Reciprocal incident links**: Evidence and incidents are cross-referenced

---

## 4. Layer 2: Anomaly Detection

### 4.1 Monitored Indicators

| Indicator | Example | Likely class |
|---|---|---|
| Contradiction burst | Many new claims suddenly conflict with established Signals | `MemoryPoisoning` |
| Score spike without support | Retrieval rank rises but gates and lineage do not justify it | `AdversarialRetrieval` |
| Taint fan-out burst | One import suddenly contaminates a large lineage region | `TaintCascade` |
| Sandbox violation cluster | One plugin repeatedly exceeds permission envelope | `SandboxViolation` |
| Tenant-boundary mismatch | Query path mixes two tenant prefixes | `CrossTenantLeakage` |
| Lineage gap | Durable record cites missing or unverifiable ancestors | `LineageMismatch` |

---

## 5. Layer 3: Quarantine

### 5.1 Quarantine Partition

Quarantine is a first-class containment boundary inside the Substrate. Suspect Signals stay
durable and queryable for reviewers, but they disappear from default retrieval and composition.

```rust
pub struct QuarantineEntry {
    pub signal_hash: ContentHash,
    pub taint: Taint,
    pub reason: ThreatClass,
    pub placed_at: SystemTime,
    pub custody: Option<ContentHash>,
    pub review_required: bool,
    pub reviewer_release: Option<PrincipalId>,
}
```

### 5.2 Resolution Workflow

1. Detect and place the Signal in quarantine.
2. Run full reverification against current gates and domain-specific checks.
3. Open review if the Signal could influence visible, destructive, or cross-tenant actions.
4. Record the reviewer decision in Custody; require attestation for high-risk release.
5. Either: keep quarantined and produce a reviewed successor, or quarantine permanently
   and publish a falsifier or postmortem.

---

## 6. Layer 4: Custody-Linked Incident Response

```rust
pub struct IncidentLink {
    pub custody: ContentHash,
    pub findings: Vec<Uuid>,
    pub affected_signals: Vec<ContentHash>,
    pub taint_sources: Vec<ContentHash>,
    pub replay_snapshot: Option<ContentHash>,
    pub postmortem: Option<ContentHash>,
}
```

---

## 7. Layer 5: Immune Memory

### 7.1 Stored Artifacts

| Artifact | Purpose |
|---|---|
| HDC fingerprint of known poisoning pattern | Fast similarity lookup during future intake |
| Taint source and fan-out shape | Recognize repeated propagation geometry |
| Best containment action | Reuse the defense that worked last time |
| False-positive record | Avoid over-quarantining benign material |
| Postmortem or custody link | Keep learning grounded in auditable history |

### 7.2 Delta Probes and Replay

Delta-speed work exercises the immune memory:
1. Replay prior poisoning cases against updated gates.
2. Probe known weak spots with synthetic hostile inputs.
3. Check whether quarantined lineage still leaks into Composer assembly.
4. Verify that plugin sandbox violations still force containment.

---

## 8. E34 Implementation State

### 8.1 What Is Live

| Component | Status | Location |
|---|---|---|
| Trust-origin IFC | Live | `roko-agent/src/safety/` |
| Five-stage immune Graph | Live | `roko-graph/src/cells/immune.rs` |
| Five-head corrigibility ordering | Live | `roko-agent/src/safety/` |
| Five-level sandbox policy | Live | `roko-agent/src/safety/` |
| Exact Cell x Graph x Space capabilities | Live | `roko-graph/src/cells/` |
| Mandatory audited production hooks | Live | All production paths |
| Host-visible tool result screening | Live | `roko-agent/src/tool_immune.rs` |
| Canonical-workspace controls | Live | `roko-agent/src/immune_boundary.rs` |
| Provider isolation | Live | Per-provider session boundary |
| Tool cooldown/isolation | Live | `roko-agent/src/safety/` |
| Linked incidents | Live | Evidence + incident cross-references |

### 8.2 Product Residuals

| Component | Scope |
|---|---|
| Provider-owned internal calls/results | Provider internals not traversed by immune Graph |
| Provider trace Signals | Internal provider telemetry not yet taint-tracked |
| Broad semantic/adaptive immune memory | Full learning from past attacks |
| Externally anchored whole-ledger authenticity | Chain-backed integrity verification |

---

## 9. Configuration

```toml
[immune]
enabled = true

[immune.taint]
propagate_through_pulses = true
require_human_signoff_for_release = true

[immune.anomaly]
z_threshold = 3.0
fanout_alert_threshold = 50
lineage_gap_alert = true

[immune.quarantine]
default_partition = "quarantine"
hide_from_query = true
hide_from_compose = true
require_org_attestation_for_high_risk_release = true

[immune.incident]
link_to_custody = true
publish_topics = ["safety.taint.detected", "safety.quarantine.entered", "safety.incident.opened"]

[immune.memory]
recognition_threshold = 0.85
store_false_positives = true
replay_on_delta = true
```

---

## 10. Test Criteria

| Test | What it validates | Type |
|---|---|---|
| `test_taint_propagates_from_pulse_to_signal` | Graduated durable records keep the Pulse taint | Unit |
| `test_taint_never_clears_without_review` | Normal derivation cannot erase taint | Unit |
| `test_quarantine_hidden_from_default_query` | Suspect Signals stay out of normal retrieval | Integration |
| `test_compose_refuses_quarantine_without_scope` | Composer cannot silently use quarantined lineage | Integration |
| `test_plugin_violation_opens_containment` | Sandbox violation produces containment and review work | Integration |
| `test_incident_links_back_to_custody` | High-risk incident reconstructable from custody + replay | Integration |
| `test_high_risk_release_requires_attestation` | Reviewer release for risky material requires attestation | Integration |
| `test_cross_tenant_mix_forces_escalation` | Tenant-boundary mismatch never degrades to warning only | Integration |
| `test_delta_replay_reopens_regression` | A broken defense in replay raises a fresh finding | Integration |

---

## Cross-References

- [Cross-Section Integration Map](./cross-section-integration-map.md) -- subsystem wiring
- v3 Section 0 -- Architecture, Provenance and Attestation
- v3 Section 11 -- Safety, broader safety spine
