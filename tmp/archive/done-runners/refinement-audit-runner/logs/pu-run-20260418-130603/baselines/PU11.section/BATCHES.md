# Batch Execution Contract

7 batches ordered for unattended execution. Topic 11 is **substantially
shipping** (7,183 LOC across two safety crates) but several docs
undercount what exists (Capability<T> framed as "target"; AuditChain,
TaintTracker, LoopGuard, SandboxEnforcer not cited in status docs).

The default work here is **regenerating Doc 16 and the status portions
of Doc 01 / 02 / 03 + frontier banners on Docs 08 / 09 / 10-13 / 14**,
not new subsystem construction.

---

## Batch Posture

- Default strategy: **cite the shipping two-crate safety stack; frontier-tag compliance + research surfaces**.
- Treat `docs/11-safety/16-critical-integration-gap.md` as the primary status hotspot (its headline is stale).
- Treat `roko-orchestrator/src/safety/` as the invisible shipping surface — docs 01 / 02 / 03 don't acknowledge it.
- If a task starts requiring actual LTL/Büchi, Heimdall/Slither/Echidna pipeline, ZK circuits, Kelly sizing, Beta-Binomial math, CaMeL dual-LLM, or cognitive-kernel namespaces, record the seam and stop.

## Required Reads

- `tmp/docs-parity/11/00-INDEX.md`
- `tmp/docs-parity/11/BATCHES.md`
- `tmp/docs-parity/11/SOURCE-INDEX.md`
- `tmp/docs-parity/11/context-pack/agent-runbook.md`
- `tmp/docs-parity/11/context-pack/carry-forward-map.md`
- `tmp/docs-parity/11/context-pack/safety-summary.md`
- `tmp/docs-parity/11/context-pack/gaps-summary.md`
- `tmp/docs-parity/11/context-pack/repo-map.md`

---

## Recommended Serial Order

`M1 -> M2 -> M3 -> M4 -> M5 -> M6 -> M7`

- M1 acknowledges `roko-orchestrator/src/safety/` — the invisible shipping surface.
- M2-M4 calibrate the three most-drifted narrative docs (01, 02, 03).
- M5 reframes Doc 16 (integration gap → coverage matrix).
- M6 frontier-banner pass for compliance + chain + kernel + risk math.
- M7 housekeeping.

---

## Batch Overview

| Batch | Tasks | Purpose | Primary Write Scope | Verify Focus |
|-------|-------|---------|---------------------|--------------|
| M1 | A.04, A.05, B.01, B.05, C.04, C.08, E.11 | Make the orchestrator-layer safety surface (capability_tokens / audit_chain / taint_propagation / loop_guard / sandboxing / permit / contract) visible | Docs 00, 01, 02, 03, 05, 06 | `rg -n "roko-orchestrator/src/safety" docs/11-safety` |
| M2 | A.04, A.06, A.07, A.08, A.09 | Doc 01 + Doc 04 calibration — Capability<K> shipping + tool tiers + role matrix | Docs 01, 04 | `rg -n "Capability<|CapabilityKind|Capability<K>|target design" docs/11-safety/01-*.md docs/11-safety/04-*.md` |
| M3 | B.01, B.02, B.03, B.04 | Doc 02 calibration — AuditChain + ContentHash lineage + on-chain anchoring cross-link | Doc 02 | `rg -n "AuditChain|AuditEntry|ContentHash|ChainWitnessEngine" docs/11-safety/02-*.md` |
| M4 | B.05, B.06, B.07, B.08, B.09 | Doc 03 calibration — TaintTracker shipping + mark full Denning / FIDES / PCAS frontier | Doc 03 | `rg -n "TaintTracker|is_tainted|Denning|FIDES|PCAS|Design — Phase 2" docs/11-safety/03-*.md` |
| M5 | F.08, F.11, F.12 | Doc 16 reframe — from "Critical Integration Gap" to "SafetyLayer Coverage Status" with provider × dispatcher matrix | Doc 16 | `rg -n "Critical Integration Gap|SafetyLayer Coverage|provider matrix" docs/11-safety/16-*.md` |
| M6 | D.01-D.08, D.11-D.15, E.01-E.12, F.01-F.04, F.06-F.07, A.10, C.12, C.13 | Frontier banner pass: compliance frameworks (NIST/MITRE/STRIDE/OWASP/CSA), chain-safety (Docs 10-13), cognitive kernel (Doc 14), forensic compliance (Doc 15), advanced risk math (Doc 09), CaMeL, Ventriloquist | Docs 00, 08, 09, 10, 11, 12, 13, 14, 15 | `rg -n "Design — Phase 2\\+|compliance framework|Tier 6" docs/11-safety/*.md` |
| M7 | global banner + INDEX | Final banner sweep + INDEX.md parity pointer | All docs/11-safety/*.md + tmp/docs-parity/11/* | `rg -n "^> \\*\\*Implementation\\*\\*:" docs/11-safety/*.md` |

---

## Dependency Graph

| Batch | Depends on |
|-------|------------|
| M1 | — |
| M2 | M1 |
| M3 | M1 |
| M4 | M1 |
| M5 | M1 |
| M6 | M1 |
| M7 | M1-M6 |

M1 is the gate — acknowledging the orchestrator-safety crate makes every other doc calibration's cross-links land cleanly.

---

## Batch Details

### M1 — Acknowledge Orchestrator-Safety Crate

**Owns**: A.04, A.05, B.01, B.05, C.04, C.08, E.11

**Problem**: Docs 00-06 describe the agent-layer `SafetyLayer` (6 guards) and imply that advanced concepts (`Capability<T>`, `AuditChain`, taint tracking, loop guard, sandbox enforcer, tool contracts) are "target design" or frontier. In reality, `crates/roko-orchestrator/src/safety/` ships 7 modules totalling 3,313 LOC implementing exactly those advanced concepts.

**Scope**:

1. Doc 00 §"Three Defense Categories" — add crate layout diagram showing agent-layer (`roko-agent/src/safety/`) + orchestrator-layer (`roko-orchestrator/src/safety/`).
2. Doc 01 — cite `Capability<K>` shipping at `capability_tokens.rs:1-860` (not "target").
3. Doc 02 — cite `AuditChain` shipping at `audit_chain.rs:1-565`.
4. Doc 03 — cite `TaintTracker` shipping at `taint_propagation.rs:1-409`.
5. Doc 05 — cite `LoopGuard` shipping at `loop_guard.rs:1-364`.
6. Doc 06 — cite `SandboxEnforcer` shipping at `sandboxing.rs:1-651`.
7. Doc 13 — cite `contract.rs:1-173` as tool-contract shell.

**Out of scope**: Implementing new safety primitives; migrating code between the two crates.

**Files**: all `docs/11-safety/*.md`, `tmp/docs-parity/11/*`

**Verify**:

```bash
rg -n "roko-orchestrator/src/safety|roko-agent/src/safety" docs/11-safety
rg -n "capability_tokens.rs|audit_chain.rs|taint_propagation.rs|loop_guard.rs|sandboxing.rs" docs/11-safety
```

**Acceptance criteria**: all major docs cite the two-crate shipping layout; "target design" language removed where it contradicts shipping code.

---

### M2 — Capability<K> Reframe (Doc 01 / Doc 04)

**Owns**: A.04, A.06, A.07, A.08, A.09

**Problem**: Doc 01 §"Target Capability<T> Design" is the biggest undercount. The shipping `Capability<K>` with PhantomData is a full 860 LOC implementation.

**Scope**:

1. Doc 01 — move `Capability<T>` from "target design" to "shipping"; cite 6 marker kinds (FileWrite/FileRead/NetworkEgress/SubprocessSpawn/GitMutate/SignalEmit).
2. Doc 01 — clarify the two tiers: lightweight `ToolPermission` flags (agent layer) + type-safe `Capability<K>` (orchestrator layer).
3. Doc 04 — cite role matrix via `SafetyLayer.role` + `RateLimitKey` keying.
4. Doc 04 — tool tiers T1/T2/T3 → map onto shipping `CapabilityKind` markers or mark informational.

**Files**: `docs/11-safety/01-capability-tokens.md`, `docs/11-safety/04-permits-allowlists.md`, `tmp/docs-parity/11/*`

**Verify**:

```bash
rg -n "Capability<K>|CapabilityKind|target design|FileWrite|NetworkEgress" docs/11-safety/01-*.md docs/11-safety/04-*.md
```

**Acceptance criteria**: Doc 01 cites the 860-LOC shipping implementation; tier story consistent between Docs 01 and 04.

---

### M3 — AuditChain and Engram Lineage (Doc 02)

**Owns**: B.01, B.02, B.03, B.04

**Problem**: Doc 02 describes SHA-256/BLAKE3 Merkle chain + AuditSink trait + FileSubstrate persistence + on-chain anchoring. The shipping `AuditChain` uses custom canonical encoding (different hash path) but is functionally equivalent.

**Scope**:

1. Doc 02 — cite `AuditChain` + `AuditEntry` at `audit_chain.rs:37-565`.
2. Doc 02 — clarify shipping hash algorithm is custom canonical encoding (for serde-version stability), BLAKE3-length output.
3. Doc 02 — note AuditSink trait + FileSubstrate persistence is partial (plumbing exists; verification pending).
4. Doc 02 — cross-link `ChainWitnessEngine` (batch 08 F.08) as anchoring primitive; note audit-chain → witness anchoring is a wiring step.

**Files**: `docs/11-safety/02-audit-chain.md`, `tmp/docs-parity/11/*`

**Verify**:

```bash
rg -n "AuditChain|AuditEntry|ContentHash|ChainWitnessEngine|content_hash" docs/11-safety/02-*.md
```

**Acceptance criteria**: Doc 02 acknowledges shipping AuditChain with honest hash-algorithm note.

---

### M4 — TaintTracker Reframe (Doc 03)

**Owns**: B.05, B.06, B.07, B.08, B.09

**Problem**: Doc 03 (804 lines) is the most over-specified safety chapter — Denning lattice, SecurityLabel with confidentiality/integrity, FIDES, RTBAS, PFI, PCAS Datalog. The shipping `TaintTracker` is a simpler boolean-with-reason tracker.

**Scope**:

1. Doc 03 — cite `TaintTracker` + `TaintReason` at `taint_propagation.rs:1-409` as shipping minimal taint surface.
2. Doc 03 — mark full Denning lattice + SecurityLabel + FIDES / RTBAS / PFI / PCAS Datalog as `Design — Phase 2+`.
3. Doc 03 — note `TaintedString with zeroize` is separate; `ScrubPolicy` (roko-agent/src/safety/scrub.rs) handles secret zeroization.
4. Doc 03 — verify (via grep in K4) whether `is_tainted` is called from git / network sinks; cite call sites.

**Files**: `docs/11-safety/03-taint-tracking.md`, `tmp/docs-parity/11/*`

**Verify**:

```bash
rg -n "TaintTracker|is_tainted|Denning|FIDES|Design — Phase 2|ScrubPolicy" docs/11-safety/03-*.md
rg -n "is_tainted" crates/roko-agent/src/safety crates/roko-orchestrator --include=*.rs
```

**Acceptance criteria**: Doc 03 clearly separates shipping minimal TaintTracker from frontier Denning / PFI / PCAS design.

---

### M5 — Doc 16 Integration Gap Reframe

**Owns**: F.08, F.11, F.12

**Problem**: Doc 16's top paragraph already acknowledges SafetyLayer is wired to ToolDispatcher for 5 HTTP provider paths. But the doc title is still "Critical Integration Gap" and the 4-phase resolution path is not status-coded.

**Scope**:

1. Rename Doc 16 from "Critical Integration Gap" to "SafetyLayer Coverage Status" (or similar).
2. Add a coverage matrix: provider path × ToolDispatcher-reached Y/N/Partial.
3. Update §"Resolution Path" with per-phase status: Phase 1 (5 of N providers wired), Phase 2-4 open.
4. Clarify the remaining gap is subprocess paths (Claude CLI) + specialty endpoints — architecture question, not pure wiring.

**Files**: `docs/11-safety/16-critical-integration-gap.md`, `tmp/docs-parity/11/*`

**Verify**:

```bash
rg -n "Critical Integration Gap|SafetyLayer Coverage|provider matrix|Phase 1" docs/11-safety/16-*.md
```

**Acceptance criteria**: Doc 16 title + body consistent with partial-closure reality.

---

### M6 — Frontier Banner Pass

**Owns**: D.01-D.08, D.11-D.15, E.01-E.12, F.01-F.04, F.06-F.07, A.10, C.12, C.13

**Problem**: Docs 08 (968 LOC threat model), 09 (1,101 LOC adaptive risk), 10 (228 LOC MEV), 11 (1,157 LOC temporal logic), 12 (1,544 LOC witness DAG), 13 (1,310 LOC formal verification), 14 (801 LOC cognitive kernel), 15 (366 LOC forensic AI) — total 7,475 lines of design content largely without shipping code.

**Scope**:

1. Apply `Design — compliance framework` to Doc 08 §"NIST AI RMF / MITRE ATLAS / STRIDE-AI / OWASP Agentic".
2. Apply `Design — Phase 2+` to Doc 09 §"Kelly Sizing / Beta-Binomial / 5D Safety Budgets".
3. Apply `Design — Phase 2+ Tier 6` to Docs 10-13 (MEV / LTL / Witness DAG / formal verification pipeline).
4. Apply `Design — Phase 2+` to Doc 14 §"Namespaces / Cognitive Scheduling / Engram Syscalls".
5. Doc 15 — mark `Implementation: Positioning — technical foundation ships; compliance-specific exports frontier`. Cross-link F.05 (`roko replay`).
6. Doc 00 §"CSA MAESTRO mapping" — mark informational.
7. Doc 07 §"CaMeL dual-LLM" — frontier.
8. Doc 07 §"Ventriloquist" — cross-link batch 08 B.08 (Tier 6 chain deferred).

**Files**: `docs/11-safety/00-*.md, 07-*.md, 08-*.md, 09-*.md, 10-*.md, 11-*.md, 12-*.md, 13-*.md, 14-*.md, 15-*.md`, `tmp/docs-parity/11/*`

**Verify**:

```bash
rg -n "Design — Phase 2\\+|compliance framework|Tier 6|Positioning" docs/11-safety
```

**Acceptance criteria**: every frontier doc carries a banner; informational-only sections marked as such.

---

### M7 — Global Banner + Housekeeping

**Owns**: final topic-11 cleanup

**Scope**:

1. Sweep `docs/11-safety/*.md` for stale `Implementation:` banners.
2. Add pointer from `docs/11-safety/INDEX.md` to `tmp/docs-parity/11/00-INDEX.md`.
3. Ensure the parity pack reflects settled structure.

**Files**: `docs/11-safety/*.md`, `tmp/docs-parity/11/*`

**Verify**:

```bash
rg -n "^> \\*\\*Implementation\\*\\*:" docs/11-safety/*.md
```

**Acceptance criteria**: banners consistent; parity audit discoverable; batch 11 closed.
