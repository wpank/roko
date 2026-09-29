# Forensic AI: Causal Replay and Regulatory Pre-Compliance

> **v3 depth file** -- `/docs/v3/depth/12-safety/forensic-ai.md`
> Canonical source: v1 `docs/v1/11-safety/15-forensic-ai.md`
> Status: **Partial**. Signal content-addressing (BLAKE3), lineage tracking,
> FileSubstrate (JSONL persistence), ToolDispatcher audit emissions, gate verdict
> persistence, and episode logging are built. Temporal query, dedicated replay engine,
> Witness DAG integration, and ZK proof generation remain design targets.

---

## 1. The Core Question

When an agent takes an action that causes harm -- a broken deployment, a data breach,
a costly error -- Roko can answer the question that no other agent framework can:

> "Why did the agent do this, what information led to this decision, and who is
> accountable?"

This capability is **Forensic AI**: content-addressed causal replay that reconstructs
the complete decision context for any past agent action. It is not a debugging feature --
it is a regulatory pre-compliance capability that transforms agent governance from
reactive (investigate after harm) to proactive (prove compliance continuously).

---

## 2. Structural Foundation

The capability exploits a structural property of the Roko architecture: every piece of
information is a Signal with a content-addressed hash and lineage chain. The entire
processing loop is auditable by construction.

### 2.1 Content-addressed Signals

Every Signal's `id` is `BLAKE3(kind + body + author + tags)`. If any field is modified,
the hash does not match. This provides tamper evidence at the individual Signal level.

### 2.2 Lineage

The `lineage: Vec<ContentHash>` field on each Signal records its parent Signals. This
creates a causal DAG (or more precisely, the Witness DAG described in `witness-dag.md`)
that enables backward traversal from any action to its inputs.

### 2.3 Provenance

The `provenance: Provenance` field records the author, model fingerprint, and taint
chain. Attribution is non-repudiable within the deployment.

---

## 3. The Replay Process

Take any agent action and replay the exact decision context:

### Step 1: Identify the action Signal

Every action produces a Signal stored in the Substrate. The Signal's `id` (ContentHash)
uniquely identifies the action.

### Step 2: Reconstruct state at the time

Query all Signals with `created_at_ms` before the action's timestamp. This
reconstructs what the agent knew at the time of the decision.

### Step 3: Reconstruct Scorer outputs

Which Scorer implementations were active? What scores did they compute? Scores are
persisted as metadata on the Signals (confidence, novelty, utility, reputation,
precision, salience, coherence).

### Step 4: Reconstruct Router selection

Which Router selected which candidate Signal, with what confidence? Router decisions
are logged as Signals in the audit chain, including rejected alternatives and their
scores.

### Step 5: Reconstruct Composer output

Which Composer assembled the context window? Under what budget constraints? Which
Signals were included, which were excluded, and why?

### Step 6: Reconstruct Gate verdict

Which Gate verified the output? What was the Verdict (Pass, Fail, Skip)? What was
the confidence score? Gate verdicts are persisted in `.roko/learn/gate-thresholds.json`
and as Signals in the audit chain.

### Step 7: Reconstruct Policy decisions

Which Policy implementations fired? What Signals did they emit? Policy decisions
(permit, deny, modify, log) are recorded by the ToolDispatcher's `emit_audit()`.

---

## 4. Cryptographic Verifiability

Every step in the replay is cryptographically verifiable:

- Each Signal's `id` is computed from its content -- modification changes the hash.
- The `lineage` field records parent Signals -- the audit DAG is tamper-evident.
- The `provenance` field records author and model -- attribution is non-repudiable.
- Optional `attestation` carries cryptographic proofs of origin.

If the Witness DAG is enabled, the replay becomes richer: five vertex types provide
fine-grained cognitive provenance, and BLAKE3 commitment hashes verify the entire
reasoning chain.

---

## 5. Replay Data Structure

```rust
pub struct ForensicReplay {
    pub action: ContentHash,
    pub action_timestamp_ms: i64,
    pub substrate_state: Vec<ContentHash>,
    pub scorer_outputs: Vec<(ContentHash, Score)>,
    pub router_selection: RouterDecision,
    pub composer_output: ComposerContext,
    pub gate_verdict: Verdict,
    pub policy_decisions: Vec<PolicyDecision>,
    pub replay_hash: ContentHash,
}
```

The replay itself is content-addressed: the `replay_hash` is computed from the replay
body, and the replay Signal has lineage pointing to all reconstructed Signals. The
replay of the replay is also verifiable.

---

## 6. Regulatory Pre-Compliance

Roko's Forensic AI maps directly to specific regulatory requirements:

### 6.1 EU AI Act

| Article | Requirement | Roko capability |
|---|---|---|
| Article 14 | Human oversight mechanisms | Cognitive Signals + Gate architecture |
| Article 11 | Technical documentation and logging | Signal lineage with content-addressed provenance |
| Article 12 | Record keeping | FileSubstrate JSONL + Witness DAG |
| Article 13 | Transparency and information to users | Forensic replay: reconstructable context |
| FRIA | Fundamental rights impact assessment | Pre-deployment simulation via Dreams engine |

### 6.2 SEC/CFTC (Financial Regulation)

| Requirement | Roko capability |
|---|---|
| Trading decision reconstruction (MiFID II) | Signal lineage from data to decision |
| Order audit trail (Rule 17a-4) | Content-addressed provenance chain |
| Best execution documentation | Router selection logs |
| Risk management documentation | Adaptive risk verdicts as Signals |
| Market manipulation detection | Temporal logic monitoring + MEV detection |

### 6.3 HIPAA (Healthcare)

| Requirement | Roko capability |
|---|---|
| Audit trail for clinical decisions | Content-addressed provenance chain |
| Access controls | Cognitive Namespaces with ACL |
| Integrity controls | BLAKE3 content hashes + commitment hashes |
| Breach notification evidence | Forensic replay reconstructs data access |

### 6.4 GDPR

| Requirement | Roko capability |
|---|---|
| Right to explanation (Article 22) | Forensic replay: reconstructable decision context |
| Purpose limitation (Article 5) | Namespace channels with kind filtering |
| Data minimization (Article 5) | Composer budget constraints |
| Right to erasure (Article 17) | Signal decay (HalfLife, TTL) |
| Processing records (Article 30) | FileSubstrate JSONL = complete processing log |

---

## 7. Incident Response

Postmortems should begin with custody records, not with grep:

1. Identify the affected action or result hash.
2. Load the associated custody record and its lineage.
3. Confirm the authorization source and human checkpoint scope.
4. Inspect taint sources, plugin tier, egress logs.
5. Verify attestation and replayability.
6. Contain by tightening permissions, disabling the plugin, or reducing egress scope.
7. Publish a postmortem Signal linked into the same lineage.

When the safety spine works, incidents turn into learnable evidence instead of
irrecoverable ambiguity.

---

## 8. Comparison with Existing Observability

| Feature | Typical agent framework | Roko Forensic AI |
|---|---|---|
| Logging | Text logs, unstructured | Content-addressed Signals with lineage |
| Audit trail | Optional, bolt-on | Mandatory, structural |
| Decision reconstruction | Not possible | Full replay of processing loop |
| Tamper evidence | None | BLAKE3 hashes + Merkle chain |
| Regulatory mapping | Custom, expensive | Pre-built for EU AI Act, SEC, HIPAA, GDPR |
| Cross-agent accountability | Not supported | Witness DAG + group trust |

---

## 9. Evidence Integrity

### 9.1 Attestation levels

| Level | Signer | Use case |
|---|---|---|
| LocalAgent | Current agent session | Routine verdicts, provisional custody |
| OrgRole | Human-owned signing authority | Destructive actions, regulated workflows |
| ChainWitness | External chain anchor | Cross-deployment verifiability |

### 9.2 Export and third-party review

Forensic evidence can be exported as a self-contained package:

```bash
roko custody export --signed --from <hash> --depth 10
```

The exported package includes all Signals in the provenance chain, their commitment
hashes, attestation records, and verification keys. A third party can verify the
package without trusting the deployment's runtime.

---

## Academic References

| Paper | Contribution |
|---|---|
| Sumers et al. (2023, arXiv:2309.02427) | CoALA cognitive architecture |
| Merkle (1987) | Merkle tree: tamper-evident audit chains |
| O'Connor & Aumasson (2020) | BLAKE3: hash function for content-addressing |
| Saltzer & Schroeder (1975) | Complete mediation principle |

---

## Implementation References

| Component | Location |
|---|---|
| Signal content-addressing | `crates/roko-core/src/signal.rs` |
| Signal lineage | `crates/roko-core/src/signal.rs` |
| FileSubstrate | `crates/roko-fs/` |
| ToolDispatcher audit | `crates/roko-agent/src/dispatcher/mod.rs` |
| Gate verdict persistence | `.roko/learn/gate-thresholds.json` |
| Episode logging | `.roko/episodes.jsonl` |
| Witness DAG | See `witness-dag.md` |
