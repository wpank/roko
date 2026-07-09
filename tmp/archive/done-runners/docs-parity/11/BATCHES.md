# Batch Execution Contract

Batch 11 is a **status-calibration pass**, not a new safety-architecture program.

The working baseline is fixed:

- safety already ships across **two crates / 7,183 LOC**
- `AgentContract`, `AgentWarrant`, and `Capability` already work
- `Capability<K>`, `AuditChain`, `TaintTracker`, `LoopGuard`, and `SandboxEnforcer` already ship
- the near-term gaps are documentation accuracy, partial coverage language, a threat-model doc, and future extensions to attestation/taint

Generated: 2026-04-18

---

## Batch Order

`M1 -> M2 -> M3 -> M4 -> M5`

| Batch | Purpose | Primary Files | Verify Focus |
|---|---|---|---|
| M1 | Rewrite the shipped core safety story | `00-INDEX.md`, `A-*.md`, `B-*.md`, `C-*.md`, `SOURCE-INDEX.md` | shipping module names and anchors appear in parity docs |
| M2 | Narrow threat/risk and recast Doc 16 as coverage status | `D-*.md`, `F-*.md` | threat-model doc called out; Doc 16 treated as coverage, not generic failure |
| M3 | Defer chain-domain safety cleanly | `E-*.md` | MEV / LTL / witness DAG / formal verification labeled deferred |
| M4 | Refresh context pack metadata | `context-pack/*` | context notes match the narrowed scope |
| M5 | Final sweep | all parity files | language, scope, and verification consistency |

---

## Batch Details

### M1 — Reframe The Shipping Core

**Owns**

- `00-INDEX.md`
- `A-defense-and-capabilities.md`
- `B-audit-taint-provenance.md`
- `C-runtime-guards.md`
- `SOURCE-INDEX.md`

**Goal**

Make the parity pack impossible to read without seeing the live safety stack.

**Required outcomes**

1. State the **7,183 LOC / two-crate** baseline.
2. Explicitly mark `AgentContract`, `AgentWarrant`, agent-layer `Capability`, and orchestrator `Capability<K>` as shipping.
3. Explicitly mark `AuditChain`, `TaintTracker`, `LoopGuard`, and `SandboxEnforcer` as shipping.
4. Separate shipped runtime behavior from planned deepening.

**Out of scope**

- editing Rust code
- designing new authz models
- treating speculative custody/compliance structures as current

**Verify**

```bash
rg -n "7,183|AgentContract|AgentWarrant|Capability<K>|AuditChain|TaintTracker|LoopGuard|SandboxEnforcer" tmp/docs-parity/11
```

### M2 — Threat / Risk / Coverage Status

**Owns**

- `D-threat-risk-adaptive.md`
- `F-kernel-forensics-gap.md`

**Goal**

Narrow the threat/risk story to what matters now: the missing threat-model doc, partial dispatcher coverage, and explicit deferral of advanced risk math and forensic packaging.

**Required outcomes**

1. Treat the threat-model doc as a concrete **ship-soon** item.
2. Mark compliance mappings as informational, not missing runtime.
3. Reframe Doc 16 from "critical integration gap" to **coverage status**.
4. Keep subprocess and specialty execution paths as the remaining bounded gap.

**Out of scope**

- implementing risk-budget systems
- inventing compliance frameworks in code
- treating cognitive kernel work as near-term

**Verify**

```bash
rg -n "threat model|ship soon|coverage status|subprocess|specialty|deferred|informational" tmp/docs-parity/11/D-threat-risk-adaptive.md tmp/docs-parity/11/F-kernel-forensics-gap.md
```

### M3 — Chain-Safety Defer Pass

**Owns**

- `E-chain-safety.md`

**Goal**

Keep chain-domain safety honest: useful design material, but deferred until the chain layer is active.

**Required outcomes**

1. Mark MEV, temporal logic, witness DAG expansion, and formal verification as deferred.
2. Preserve only minimal cross-links to live precursors such as `WalletGate`, `TxSimGate`, `ChainWitnessEngine`, and content-addressed lineage.
3. Remove any implication that this batch should build or fully document chain-runtime safety now.

**Verify**

```bash
rg -n "deferred|Tier 6|Phase 2|WalletGate|TxSimGate|ChainWitnessEngine" tmp/docs-parity/11/E-chain-safety.md
```

### M4 — Context-Pack Refresh

**Owns**

- `context-pack/safety-summary.md`
- `context-pack/gaps-summary.md`
- `context-pack/carry-forward-map.md`
- `context-pack/repo-map.md`
- `context-pack/agent-runbook.md`

**Goal**

Make the supporting brief match the narrowed execution contract.

**Required outcomes**

1. Keep the 7,183 LOC / two-crate baseline front and center.
2. Reduce the worklist to realistic parity refresh tasks.
3. Keep the context pack aligned to the current batch contract.

### M5 — Final Sweep

**Owns**

- all files under `tmp/docs-parity/11/`
- `run-docs-parity.sh`

**Goal**

Catch drift between the rewritten detail files, context pack, and runner.

**Verify**

```bash
rg -n "7,183|two crates|coverage status|deferred|M1|M2|M3|M4|M5" tmp/docs-parity/11/*.md tmp/docs-parity/11/context-pack/*.md tmp/docs-parity/11/run-docs-parity.sh
bash -n tmp/docs-parity/11/run-docs-parity.sh
```

---

## Working Rule

If a task starts requiring new Rust, new protocol design, or new compliance machinery to make the docs true, it does not belong in PU11. Record the seam and defer it.
