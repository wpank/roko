# 11-Safety Parity Refresh

Parity refresh for `docs/11-safety/` after the integrator audit.

Generated: 2026-04-18

---

## Topic Position

Topic 11 is not missing a safety system. It is missing disciplined status language.

The shipping baseline for this topic is:

- two safety crates, **7,183 LOC total**
- `crates/roko-agent/src/safety/` as the live runtime guard and authorization layer
- `crates/roko-orchestrator/src/safety/` as the shipped orchestrator safety module set
- `ToolDispatcher` integration for routed provider-backed paths

The main doc problem is that several chapters still describe shipping code as aspirational while other chapters describe speculative work as if it were current.

---

## What This Batch Fixes

This parity pack is now scoped to five realistic outcomes:

1. Reframe the core safety docs around the **existing** `AgentContract` / `AgentWarrant` / `Capability` system.
2. Mark `Capability<K>`, `AuditChain`, `TaintTracker`, `LoopGuard`, and `SandboxEnforcer` as **shipping**.
3. Recast Doc 16 as a **coverage-status** problem, not a generic "critical gap" story.
4. Call out the two practical ship-soon items:
   - extend `Attestation` and expand taint
   - write the missing threat-model doc
5. Move compliance, chain-safety, cognitive-kernel, and forensic-packaging work into explicit defer buckets.

---

## Shipping Reality

### Shipping now

- `AgentContract`, `GovernanceRule`, and `Invariant` already ship in `roko-agent`.
- `AgentWarrant` and the agent-layer `Capability` enum already ship in `roko-agent`.
- `Capability<K>` with typed marker kinds ships in `roko-orchestrator` at 860 LOC.
- `AuditChain` ships in `roko-orchestrator` at 565 LOC.
- `TaintTracker` ships in `roko-orchestrator` at 409 LOC.
- `LoopGuard` ships in `roko-orchestrator` at 364 LOC.
- `SandboxEnforcer` ships in `roko-orchestrator` at 651 LOC.
- `ToolDispatcher` already applies safety checks on routed provider-backed execution paths.

Module existence and coverage are separate questions. This pack treats those orchestrator modules as shipped code while reserving "coverage status" for the execution paths that actually invoke the shared safety pipeline today.

### Ship soon

- extend the existing `Attestation` surface rather than inventing a replacement
- expand taint beyond the current minimal tracker
- write the standalone threat-model doc called for by the audit

### Deferred

- NIST / MITRE / STRIDE / OWASP / CSA mapping depth
- advanced adaptive-risk math
- MEV / LTL / witness-DAG expansion / formal-verification pipeline
- cognitive-kernel namespaces / scheduling / Engram syscalls
- regulator-facing forensic export packaging

---

## Parity Files

| File | Purpose | Refresh posture |
|---|---|---|
| [A-defense-and-capabilities.md](A-defense-and-capabilities.md) | Docs 00, 01, 04 | Rewrite around the shipped authorization stack |
| [B-audit-taint-provenance.md](B-audit-taint-provenance.md) | Docs 02, 03 | Split shipped audit/taint from planned deepening |
| [C-runtime-guards.md](C-runtime-guards.md) | Docs 05, 06, 07 | Confirm loop/sandbox/runtime guards as shipping |
| [D-threat-risk-adaptive.md](D-threat-risk-adaptive.md) | Docs 08, 09 | Narrow to ship-soon threat-model doc plus deferred risk math |
| [E-chain-safety.md](E-chain-safety.md) | Docs 10, 11, 12, 13 | Mark chain safety as deferred |
| [F-kernel-forensics-gap.md](F-kernel-forensics-gap.md) | Docs 14, 15, 16 | Defer kernel/forensics; recast Doc 16 as coverage status |
| [SOURCE-INDEX.md](SOURCE-INDEX.md) | Code anchors | Point directly at the live safety modules and wiring |
| [BATCHES.md](BATCHES.md) | Execution contract | Keep batch scope realistic for one agent |

---

## Current Gap Picture

### High signal

- Doc 01 is materially wrong if it still calls `Capability<K>` a target design.
- Docs 02 and 03 need to acknowledge the shipping `AuditChain` and `TaintTracker`.
- Docs 05 and 06 need to cite the shipping `LoopGuard` and `SandboxEnforcer`.
- Doc 16's headline is stale; the remaining issue is partial coverage, especially subprocess and specialty paths.

### Not a current implementation gap

- compliance taxonomy depth
- chain-domain formal methods
- cognitive-kernel redesign
- forensic compliance packaging

---

## Success Definition

Batch 11 is in good shape when:

- the parity pack consistently uses the **7,183 LOC / two-crate** baseline
- the core safety system is described as shipping, not planned
- the threat-model doc is named as a concrete ship-soon documentation task
- deferred material is labeled clearly enough that later batches do not treat it as current architecture
