# Batch Execution Contract

This batch set is now **docs-only**. It exists to keep `tmp/docs-parity/04/` honest after the verification audit.

Do not treat these batches as a mandate to implement reward models, autonomous evaluator agents, or forensic replay systems.

---

## Batch Posture

- Default strategy: **refresh the parity materials to match shipped verification code**.
- Only edit files under `tmp/docs-parity/04/`.
- Use current code anchors from [SOURCE-INDEX.md](SOURCE-INDEX.md) instead of stale line numbers in older parity notes.
- Prefer `shipped`, `partial`, and `deferred` language over large future implementation plans.
- If a section depends on research-grade systems, move it to an explicit deferred posture instead of expanding the work.

Required reads for every batch:

- [00-INDEX.md](00-INDEX.md)
- [SOURCE-INDEX.md](SOURCE-INDEX.md)
- [context-pack/agent-runbook.md](context-pack/agent-runbook.md)

---

## Recommended Serial Order

For a single agent run:

`V1 -> V2 -> V3 -> V4 -> V5`

This order first fixes the overall posture, then refreshes the shipped runtime sections, then marks the research tail as deferred, then refreshes anchors and runbooks.

---

## Batch Overview

| Batch | Purpose | Primary Files | Verify |
|---|---|---|---|
| `V1` | Reset the overall posture and gate-foundation story | `00-INDEX.md`, `A-gate-foundation.md`, `B-pipeline-rungs.md` | `rg -n "substantially shipped|7-rung|rung_dispatch|Gate trait" tmp/docs-parity/04` |
| `V2` | Refresh partial foundations without overscoping | `C-artifacts-ratcheting.md`, `D-feedback-thresholds.md`, context summaries | `rg -n "ArtifactStore|GateRatchet|gate-thresholds.json|EMA" tmp/docs-parity/04` |
| `V3` | Mark the back half as deferred research | `E-process-rewards-lifecycle.md`, `F-autonomous-evoskills.md` | `rg -n "DEFERRED|research|target-state" tmp/docs-parity/04/E-process-rewards-lifecycle.md tmp/docs-parity/04/F-autonomous-evoskills.md` |
| `V4` | Split shipped verdict signals from deferred forensic replay | `G-forensic-verdict-signals.md`, `SOURCE-INDEX.md`, `context-pack/carry-forward-map.md` | `rg -n "GateVerdict|forensic|deferred|rung_dispatch" tmp/docs-parity/04` |
| `V5` | Final consistency sweep for runbooks and runner metadata | `context-pack/agent-runbook.md`, `context-pack/repo-map.md`, `run-docs-parity.sh` | `bash -n tmp/docs-parity/04/run-docs-parity.sh` |

---

## Dependency Graph

| Batch | Depends on |
|---|---|
| `V1` | — |
| `V2` | `V1` |
| `V3` | `V1` |
| `V4` | `V1`, `V2`, `V3` |
| `V5` | `V4` |

---

## Batch Details

### V1 — Reset The Shipped Story

**Owns**:

- overall posture in [00-INDEX.md](00-INDEX.md)
- gate-foundation truth in [A-gate-foundation.md](A-gate-foundation.md)
- runtime 7-rung truth in [B-pipeline-rungs.md](B-pipeline-rungs.md)

**Scope**:

1. Replace backlog framing with shipped/runtime framing.
2. Keep the `Gate` trait and gate inventory grounded in current code.
3. Document the live 7-rung executor/plan path through `run_gate_pipeline(...)` and `rung_dispatch.rs`.
4. Stop implying that the primary runtime gap is “activation of missing verification.”

**Out of scope**:

- artifact persistence details
- reward-model design
- forensic replay design

---

### V2 — Refresh Partial Foundations

**Owns**:

- [C-artifacts-ratcheting.md](C-artifacts-ratcheting.md)
- [D-feedback-thresholds.md](D-feedback-thresholds.md)
- [context-pack/verification-summary.md](context-pack/verification-summary.md)
- [context-pack/gaps-summary.md](context-pack/gaps-summary.md)

**Scope**:

1. Present `ArtifactStore` and `GateRatchet` as real foundations with limited runtime scope.
2. Present adaptive thresholds as wired EMA persistence, not speculative design.
3. Keep `GateFeedback` truthful: shipped classifier, limited evidence of full retry-path ownership.
4. Remove long research backlogs from these sections.

**Out of scope**:

- new persistence schemes
- SPC detector implementation plans
- new gate families

---

### V3 — Defer The Research Tail

**Owns**:

- [E-process-rewards-lifecycle.md](E-process-rewards-lifecycle.md)
- [F-autonomous-evoskills.md](F-autonomous-evoskills.md)

**Scope**:

1. Mark these sections `DEFERRED`.
2. Preserve only the small, current truths: efficiency-event capture, episode/skill plumbing, generated-test consumer-side gate.
3. Move the rest to target-state / future-work language.

**Out of scope**:

- converting research concepts into implementation batches
- adding speculative milestones

---

### V4 — Split Verdict Signals From Forensic Replay

**Owns**:

- [G-forensic-verdict-signals.md](G-forensic-verdict-signals.md)
- [SOURCE-INDEX.md](SOURCE-INDEX.md)
- [context-pack/carry-forward-map.md](context-pack/carry-forward-map.md)

**Scope**:

1. Keep the live `GateVerdict` path in the shipped story.
2. Mark forensic replay and analytics as deferred.
3. Refresh stale line anchors and point readers at current runtime files.

**Out of scope**:

- replay-algorithm design
- predictive gate-selection planning

---

### V5 — Final Consistency Sweep

**Owns**:

- [context-pack/agent-runbook.md](context-pack/agent-runbook.md)
- [context-pack/repo-map.md](context-pack/repo-map.md)
- [run-docs-parity.sh](run-docs-parity.sh)

**Scope**:

1. Align runbook language with the narrowed docs-only mission.
2. Refresh repo-map notes to emphasize the real runtime path.
3. Update runner descriptions, dependencies, and verify commands so they match the new batches.

**Out of scope**:

- code changes outside `tmp/docs-parity/04/`
- cargo-based verification plans

---

## Completion Standard

A successful refresh leaves:

- no large implementation backlog language in `A-D`
- `DEFERRED` labels in `E-F`
- an explicit split between live verdict signals and deferred forensic replay in `G`
- fresh source anchors
- a shell-syntax-clean [run-docs-parity.sh](run-docs-parity.sh)
