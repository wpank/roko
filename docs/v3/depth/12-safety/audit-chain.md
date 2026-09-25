# Audit Chain

> **v3 depth file** -- `/docs/v3/depth/12-safety/audit-chain.md`
> Canonical source: v1 `docs/v1/11-safety/02-audit-chain.md`
> Status: **Wired**. Signal content-addressing (BLAKE3), lineage tracking,
> ToolDispatcher `emit_audit()`, episode logging, and gate verdict persistence are live.
> Dedicated Custody type and replay engine remain target-state.

---

## 1. Beyond Generic Audit Logs

The safety spine needs more than a generic log. For any auditable action, Roko must
reconstruct:

- Who initiated it (principal).
- What was authorized (evidence).
- Which heuristics and claims influenced it (lineage).
- Which gates and reviews approved it (verdicts).
- What result was produced (content hash).
- Which tainted inputs were still in play (taint state).

The canonical durable record is a content-addressed Signal with provenance metadata.
The target-state Custody record extends this into an explicit chain-of-custody story.

---

## 2. SafetyHook Architecture

The `ToolDispatcher` implements a seven-stage pipeline where each stage emits an
audit Signal via `emit_audit()`:

```
dispatch() pipeline:
  1. validate     -- is this a valid tool call?
  2. tool_filter  -- is this tool allowed for this role?
  3. permission   -- does the agent have permission?
  4. safety.check_pre_execution -- SafetyLayer pre-checks
  5. handler      -- execute the tool
  6. truncate     -- enforce output size limits
  7. safety.scrub_output -- ScrubPolicy post-processing
```

Each stage produces an audit record containing:

- Tool name and call parameters.
- Phase identifier (validate, filter, permission, pre_execution, execute, truncate, scrub).
- Status (allowed, denied, error).
- Timestamp and principal.
- Content hash linking to the parent Signal.

---

## 3. Signal Lineage as Causal Structure

Every Signal carries lineage:

```rust
pub struct Signal {
    pub id: ContentHash,           // BLAKE3(kind + body + author + tags)
    pub lineage: Vec<ContentHash>, // Parent Signal hashes
    pub provenance: Provenance,    // Author, model, taint
    // ...
}
```

Lineage answers causal questions:

- Which prompt or plan step led to this action?
- Which tool output or external fetch influenced the decision?
- Which gate verdicts were emitted before the action persisted?

---

## 4. Custody: The Auditable Action Record

Custody closes the gap between lineage (ancestry) and authorization (who approved):

```rust
pub struct Custody {
    pub action: ActionHash,
    pub principal: PrincipalId,
    pub when: Timestamp,
    pub authorized: AuthzEvidence,
    pub why_heuristics: Vec<HeuristicId>,
    pub why_claims: Vec<ClaimId>,
    pub simulation: Option<SimHash>,
    pub gates_passed: Vec<GateVerdict>,
    pub taint: Option<Taint>,
    pub result: Option<ResultHash>,
    pub witness: Option<ChainWitness>,
}
```

The important addition: not just "what happened" but "why the runtime believed it was
acceptable at the time."

---

## 5. What Requires Custody

If an action is destructive, externally visible, compliance-relevant, or hard to reverse,
it should emit Custody:

- File deletion or overwrite outside trivial workspace edits.
- Shell execution with side effects.
- Dependency installation.
- Network egress carrying user or external data.
- Pull request creation or publication.
- External fact claims promoted into durable knowledge.
- Signing or broadcasting chain transactions.

Lower-risk actions emit lightweight audit Signals.

---

## 6. Authorization Evidence

Custody records not just the decision but the evidence behind it:

| Evidence | Meaning |
|---|---|
| Role grant | Standing permission under the active profile |
| Session approval | Previously granted confirmation still in scope |
| One-shot approval | Single-use checkpoint approval |
| Review confirmation | Explicit approval for destructive action |
| Escalation outcome | Human or system-level override |

This makes it possible to distinguish:

- An action the system was always allowed to perform.
- An action the user approved once.
- An action that only happened after escalation.

That distinction is often more important to auditors than the raw action.

---

## 7. Attestation Levels

Some Signals need stronger guarantees than "we persisted them":

| Level | Use case |
|---|---|
| LocalAgent | Gate verdicts, routine audit, provisional custody |
| OrgRole | Reviewed destructive actions, production writes |
| ChainWitness | Cross-deployment verifiable evidence |

Attestation attaches a cryptographic statement to the content hash and records who
stands behind it. It strengthens the evidentiary weight of specific Signals.

---

## 8. Pre-Call, Post-Call, and Replay

Custody brackets the action lifecycle:

1. **Pre-call:** record principal, target, context, requested permission, confirmation.
2. **Execution:** capture result hash, simulation output, violations.
3. **Post-call:** persist gate verdicts, secret scrubbing outcomes, taint metadata.

This enables faithful replay:

- Start from the action hash.
- Walk lineage backward to inputs.
- Inspect the exact heuristics and claims cited at the time.
- Re-run verification logic with the recorded taint and attestation state.
- Compare the reproduced result to the stored outcome.

Replay matters because incident response should rely on recorded historical state,
not today's recalibrated system.

---

## 9. Content Addressing and Tamper Evidence

Every Signal's `id` is `BLAKE3(kind + body + author + tags)`. If any field has been
modified, the hash does not match. The lineage vector creates a DAG of content-addressed
references. Modifying any intermediate Signal invalidates all downstream references.

The FileSubstrate persists Signals as JSONL with content hashes. On-chain anchoring
of DAG root hashes (target-state) provides non-repudiable timestamps.

---

## 10. Incident Response

Postmortems begin with the audit chain:

1. Identify the affected action or result hash.
2. Load the associated audit Signals and lineage.
3. Confirm the authorization source and review scope.
4. Inspect taint sources, plugin tier, and egress logs.
5. Verify attestation and replayability.
6. Contain by tightening permissions or disabling plugins.
7. Publish a postmortem Signal linked into the same lineage.

Transitive quarantine incidents persist across restarts and carry reciprocal links
back to the causal evidence chain.

---

## 11. Audit Tooling

```bash
roko knowledge custody list --after 7d --principal user:alice
roko knowledge custody show <action-hash>
roko knowledge custody verify <action-hash>
```

---

## Implementation References

| Component | Location |
|---|---|
| Signal content-addressing | `crates/roko-core/src/signal.rs` |
| Signal lineage | `crates/roko-core/src/signal.rs` (lineage field) |
| ToolDispatcher emit_audit | `crates/roko-agent/src/dispatcher/mod.rs` |
| FileSubstrate (JSONL) | `crates/roko-fs/` |
| Episode logging | `.roko/episodes.jsonl` |
| Gate verdict persistence | `.roko/learn/gate-thresholds.json` |
