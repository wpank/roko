# 11-Safety Parity Analysis

Gap analysis of `docs/11-safety/` (17 PRDs + INDEX, ~10,683 lines
describing defense-in-depth, capability tokens, audit chain, taint
tracking, permits, loop detection, sandboxing, prompt security,
threat model, adaptive risk, chain-domain safety, cognitive kernel,
forensic AI, critical integration gap) against the shipping safety
stack across two crates:

- `crates/roko-agent/src/safety/` — 3,870 LOC, 9 modules (bash, capabilities, contract, git, network, path, rate_limit, scrub, mod)
- `crates/roko-orchestrator/src/safety/` — 3,313 LOC, 7 modules (audit_chain, capability_tokens, loop_guard, permit, sandboxing, taint_propagation, mod)

Plus cross-crate consumers: `roko-agent/src/dispatcher/mod.rs`
(ToolDispatcher), `roko-conductor` (circuit breaker + ghost turn +
stuck detection), `roko-core/src/{provenance, engram}.rs`
(Engram lineage), `roko-chain/src/gate/*` (WalletGate + TxSimGate),
`roko-compose` (XML-delimited prompts).

Generated: 2026-04-16

---

## How To Use This Batch

**Topic 11 is a shipping-more-than-documented problem, with a large
frontier research halo.**

The agent-layer `SafetyLayer` (6 guards: bash, git, network, path,
scrub, rate_limit) is what Doc 16 §"What Is Built" acknowledges. What
Doc 16 does **NOT** acknowledge is that `crates/roko-orchestrator/
src/safety/` ships the "advanced" surfaces that Doc 01 / 02 / 03
frame as "target design":

- `Capability<K>` with PhantomData (860 LOC) — Doc 01's target design, **shipping**
- `AuditChain` with hash-chain (565 LOC) — Doc 02's Merkle-chain, **shipping**
- `TaintTracker` (409 LOC) — Doc 03's taint system, **shipping (simpler than full Denning)**
- `LoopGuard` (364 LOC) — orchestrator-layer loop detection, **shipping**
- `SandboxEnforcer` (651 LOC) — Doc 06's "future container sandbox", **shipping**
- `Permit` (452 LOC) — permit scoping, **shipping**

Two crates totalling **7,183 LOC of safety infrastructure**. Combined
with conductor watchers (batch 07) + chain gates (batch 08) + daimon
affect-aware routing (batch 09) + dream threat simulation (batch 10
D.08), Roko has **substantial shipping safety**.

The frontier in topic 11 is:

- compliance-framework mappings (NIST AI RMF, MITRE ATLAS, STRIDE-AI, OWASP Agentic Top 10, CSA MAESTRO) — informational only
- advanced risk math (Kelly sizing, Beta-Binomial, 5D safety budgets, hierarchical delegation)
- chain-domain safety (MEV detection, LTL Büchi automata, Witness DAG with 5 vertex types, Heimdall/Slither/Echidna/hevm/Certora/Kontrol pipeline) — Tier 6 deferred
- academic prompt-security patterns (CaMeL dual-LLM, Ventriloquist on-chain)
- cognitive kernel primitives (Namespaces, Cognitive Scheduling, Engram Syscalls)
- forensic-AI compliance packaging (regulator-facing export generators)
- advanced taint algebra (Denning lattice, FIDES, RTBAS, PFI, PCAS Datalog)

The work in this batch is therefore:

1. **Acknowledge the orchestrator-safety crate** (M1) — the biggest single doc-honesty win.
2. **Reframe Doc 01 `Capability<T>`** from "target" to "shipping" (M2).
3. **Reframe Doc 02 `AuditChain` + Doc 03 `TaintTracker`** as shipping, with scope caveats (M3, M4).
4. **Reframe Doc 16** from "Critical Integration Gap" to "SafetyLayer Coverage Status" (M5).
5. **Frontier banner pass** on Docs 08, 09, 10, 11, 12, 13, 14, 15 (M6).
6. **Housekeeping** (M7).

If a task starts requiring new LTL / Büchi / CaMeL / Kelly / 5D-budget / cognitive-namespace / Engram-syscall code, stop and record the seam.

Recommended single-agent serial order: `M1 -> M2 -> M3 -> M4 -> M5 -> M6 -> M7`

---

## Document Index

| File | Docs Covered | Items | Status |
|------|--------------|-------|--------|
| [A-defense-and-capabilities.md](A-defense-and-capabilities.md) | 00, 01, 04 | A.01-A.11 | 8 DONE / 2 PARTIAL / 1 NOT DONE |
| [B-audit-taint-provenance.md](B-audit-taint-provenance.md) | 02, 03 | B.01-B.09 | 3 DONE / 4 PARTIAL / 2 NOT DONE |
| [C-runtime-guards.md](C-runtime-guards.md) | 05, 06, 07 | C.01-C.14 | 11 DONE / 1 PARTIAL / 2 NOT DONE |
| [D-threat-risk-adaptive.md](D-threat-risk-adaptive.md) | 08, 09 | D.01-D.15 | 2 DONE / 1 PARTIAL / 12 NOT DONE |
| [E-chain-safety.md](E-chain-safety.md) | 10, 11, 12, 13 | E.01-E.12 | 0 DONE / 2 PARTIAL / 10 NOT DONE |
| [F-kernel-forensics-gap.md](F-kernel-forensics-gap.md) | 14, 15, 16 | F.01-F.12 | 3 DONE / 3 PARTIAL / 6 NOT DONE |
| [BATCHES.md](BATCHES.md) | — | 7 batches (M1-M7) | Execution contract |
| [SOURCE-INDEX.md](SOURCE-INDEX.md) | — | Verified code anchors | Reference |
| [run-docs-parity.sh](run-docs-parity.sh) | — | Batch runner | Overnight execution scaffold |
| `context-pack/agent-runbook.md` | — | Execution posture | Agent brief |
| `context-pack/carry-forward-map.md` | — | Deferral map | Scope control |
| `context-pack/safety-summary.md` | — | Shipping safety summary | Quick context |
| `context-pack/gaps-summary.md` | — | Main doc-honesty hotspots | Quick context |
| `context-pack/repo-map.md` | — | High-value paths + searches | Fast verification |

Doc `INDEX.md` is absorbed into this file.

---

## Overall Parity: 27/73 items DONE (37%)

The 36% number substantially undersells what actually ships because
**~9 of the 13 PARTIAL entries are shipping code with doc drift** —
Doc 01 frames shipping `Capability<K>` as "target", Doc 16 frames
5-of-N integration-wiring as "Critical Integration Gap", Doc 03
describes full Denning lattice while the shipping `TaintTracker` is
a simpler-but-functional subset, etc.

If PARTIAL-due-to-doc-drift entries are counted as DONE, parity
jumps to ~54%.

### Tier 1 — Should Exist Now (runtime-critical)

None. Safety is shipping at the level required to run agents. The
integration-gap residual (subprocess paths) is narrow and the
conductor-layer watchers + per-turn rate limiter + path canonicaliser
cover the immediate exposure.

### Tier 2 — Should Exist Soon (doc honesty / status clarity)

| ID | Title | Status | Severity |
|----|-------|--------|----------|
| A.04 | `Capability<K>` shipping at 860 LOC; Doc 01 says "target design" | DONE (doc drift) | HIGH |
| B.01 | `AuditChain` shipping at 565 LOC; Doc 02 implies future work | DONE (doc drift) | HIGH |
| B.05 | `TaintTracker` shipping at 409 LOC; Doc 03 implies full Denning required | DONE (doc drift) | HIGH |
| C.04 | `LoopGuard` shipping at 364 LOC; Doc 05 doesn't cite it | DONE (doc drift) | MEDIUM |
| C.08 | `SandboxEnforcer` shipping at 651 LOC; Doc 06 says "future container sandbox" | DONE (doc drift) | MEDIUM |
| F.08 | SafetyLayer wired to ToolDispatcher for 5 HTTP providers; Doc 16 title says "Critical Integration Gap" | PARTIAL (doc stale) | MEDIUM |
| F.11 | Resolution path phase 1 substantially complete (5 of N providers); Doc 16 doesn't status-code it | PARTIAL | MEDIUM |
| C.14 | MCP is embraced with safety gating, not avoided; Doc 07 still frames "MCP avoidance" | PARTIAL | LOW |
| B.03 / B.04 | AuditSink / FileSubstrate persistence + on-chain anchoring wiring unverified | PARTIAL | LOW |
| B.09 | `is_tainted` shipping but call-site coverage at git / network sinks unverified | PARTIAL | LOW |

### Tier 3 — Future / Phase 2+ Frontier

| ID | Title | Status | Severity |
|----|-------|--------|----------|
| D.01-D.08 | Attack trees, NIST/MITRE/STRIDE/OWASP/CSA-MAESTRO compliance mappings | NOT DONE | LOW |
| D.11-D.15 | Kelly sizing, Beta-Binomial, 5D safety budgets, hierarchical delegation | NOT DONE | LOW |
| E.01-E.12 | MEV detection, LTL Büchi, Witness DAG, formal-verification pipeline — all Tier 6 chain deferred | NOT DONE / PARTIAL | LOW |
| F.01 / F.03 / F.04 | Cognitive kernel namespaces, cognitive scheduling, Engram syscalls | NOT DONE | LOW |
| F.06 / F.07 | Forensic-AI regulatory pre-compliance (EU AI Act / HIPAA / SOX / GDPR / SEC/CFTC) | NOT DONE | LOW |
| C.12 | CaMeL dual-LLM (Debenedetti 2025) | NOT DONE | LOW |
| C.13 | Ventriloquist on-chain (Korai chain Tier 6) | NOT DONE | LOW |
| B.07 / B.08 | Bloom Oracle, FIDES, RTBAS, PFI, PCAS Datalog | NOT DONE | LOW |

### Already Shipped

| ID | Title | Status |
|----|-------|--------|
| A.01 | SafetyLayer composite with 6 runtime guards | DONE |
| A.02 | Three defense categories (structural / behavioral / cognitive) | DONE |
| A.03 | Safety as `Gate` + `Policy` via Synapse traits | DONE |
| A.04 | `Capability<K>` with PhantomData + 6 kinds + CapabilityIssuer | DONE |
| A.05 | AgentWarrant OCaps surface | DONE |
| A.07 | ToolPermission flags | DONE |
| A.08 | Role-based permission matrix via SafetyLayer.role | DONE |
| A.09 | Task-level tool filters | DONE |
| B.01 | AuditChain append-only hash-chain | DONE |
| B.02 | Engram lineage DAG via ContentHash | DONE |
| B.05 | TaintTracker with mark / propagate / is_tainted | DONE |
| C.01 | RateLimiter sliding window | DONE |
| C.02 | Circuit breaker via conductor | DONE |
| C.03 | Ghost-turn detection + stuck-pattern watcher | DONE |
| C.04 | LoopGuard | DONE |
| C.05 | ScrubPolicy secret scrubbing (9 default patterns) | DONE |
| C.06 | Adaptive gate thresholds | DONE |
| C.07 | PathPolicy canonicalization | DONE |
| C.08 | SandboxEnforcer | DONE |
| C.09 | ProcessSupervisor lifecycle | DONE |
| C.10 | WorktreeManager isolation | DONE |
| C.11 | XML-delimited prompt architecture | DONE |
| D.10 | Hard shields via SafetyLayer deny-patterns | DONE |
| D.13 | Daimon affect-aware risk modulation | DONE |
| F.05 | Content-addressed causal replay (`roko replay`) | DONE |
| F.09 | 6-guard SafetyLayer composite (Doc 16 "What Is Built") | DONE |
| F.10 | 7-stage ToolDispatcher pipeline | DONE |

---

## Execution Boundaries

| Item | Better Home | Why |
|------|-------------|-----|
| NIST / MITRE / STRIDE-AI / OWASP compliance mappings | later compliance certification pass | no shipping code impact |
| Kelly sizing / Beta-Binomial / 5D safety budgets | later adaptive-risk pass | Phase 2+ |
| MEV / LTL / Witness DAG / formal-verification pipeline | Tier 6 chain activation | blocked on chain layer |
| Cognitive kernel namespaces / Engram syscalls | later kernel redesign pass | fundamental redesign |
| Forensic-AI regulatory export templates | later compliance packaging pass | positioning only |
| CaMeL dual-LLM | later prompt-security pass | academic frontier |
| Advanced Denning / FIDES / PFI / PCAS | later taint-deepening pass | shipping tracker suffices |

Batch 11 should produce:

- Doc 01 acknowledging `Capability<K>` shipping,
- Docs 02 / 03 acknowledging AuditChain / TaintTracker shipping,
- Doc 16 renamed / reframed with provider × dispatcher matrix,
- frontier banners on Docs 08 / 09 / 10 / 11 / 12 / 13 / 14 / 15,
- cross-reference to the two shipping safety crates throughout.

---

## Critical Safety-Layer Issues

1. **Orchestrator-safety crate is invisible to the safety PRDs.** `roko-orchestrator/src/safety/` ships 7 modules at 3,313 LOC that implement exactly what Docs 01 / 02 / 03 / 05 / 06 describe as "target" or "future" designs.
2. **Doc 01 `Capability<T>` framing is the single biggest undercount.** 860 LOC of the full advanced PhantomData-based type-safe token system ships today.
3. **Doc 16 headline "Critical Integration Gap" is stale.** Its own body acknowledges 5 HTTP provider paths are wired; the residual is subprocess / specialty endpoints.
4. **Compliance framework chapters are informational, not code-gap.** NIST / MITRE / STRIDE-AI / OWASP mappings in Doc 08 have no shipping counterpart because they are meta-compliance frameworks — not subsystems to implement.
5. **Chain-domain safety is Tier 6 deferred.** Docs 10-13 are 4,239 lines of specification with almost no shipping counterpart, and that is consistent with the broader chain-layer deferral (batch 08).

---

## Key Insight

Roko's safety story is **stronger than the PRDs document**. The
`roko-orchestrator/src/safety/` crate — which ships `Capability<K>`,
`AuditChain`, `TaintTracker`, `LoopGuard`, `SandboxEnforcer`, `Permit`,
and a shell for tool contracts — is essentially invisible to Docs
00-07. Fixing that single class of drift (M1-M4) would flip topic 11
from "36% done" self-reporting to "~54% done" reporting without
writing any Rust.

The remaining frontier is genuinely frontier: compliance frameworks
are mappings not subsystems, chain-domain safety waits for chain
activation, cognitive kernel is a redesign pass, and advanced risk
math is Phase 2+.

---

## Batch 11 Success Definition

Batch `11` is successful when:

- Docs 00-07 cite `crates/roko-orchestrator/src/safety/` as a shipping surface,
- Doc 01 `Capability<T>` is "shipping" not "target",
- Doc 02 acknowledges shipping AuditChain with honest hash-algorithm note,
- Doc 03 separates shipping minimal TaintTracker from frontier Denning / FIDES,
- Doc 16 title + body consistent with 5-of-N provider wiring reality,
- Docs 08-15 carry frontier / compliance-framework / Tier-6 banners uniformly,
- later agents can pick up `BATCHES.md` and execute M1 without further context.
