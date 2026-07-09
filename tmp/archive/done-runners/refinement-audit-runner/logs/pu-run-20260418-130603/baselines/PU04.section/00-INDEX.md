# 04-Verification Parity Analysis

Gap analysis of `docs/04-verification/` (13 documents) vs the actual Roko verification stack: `crates/roko-gate/`, the orchestrator callsites in `crates/roko-cli/src/orchestrate.rs`, and the downstream learning / signal consumers that are supposed to benefit from gate verdicts.

Generated: 2026-04-16

---

## How To Use This Batch

This batch should be treated as **verification runtime activation + contract hardening**, not as a mandate to implement every verification-adjacent research idea from docs `07`, `09`, `10`, `11`, `12`, and `15`.

- Prefer wiring already-shipped gate code over inventing new gate families.
- Prefer one runtime seam per batch: dispatch, selection, thresholds, feedback, ratchet, artifacts, signals.
- If a task starts depending on reward-model training, autonomous test-writer agents, or replay analytics, record the seam and defer it.
- For unattended runs, every batch should be able to stop with a clear `PASS`, `FAIL`, or `BLOCKED` result and leave behind evidence: files changed, commands run, test output, and explicit deferrals.

Recommended single-agent serial order inside batch `04`:

`V1 -> V2 -> V3 -> V4 -> V5 -> V6 -> V7 -> V8`

Reasoning:

- `V1` makes the current runtime contract honest before broader activation work starts.
- `V2` turns selector / pipeline code into a live production path.
- `V3` and `V4` close the two biggest “learns but does not act” seams: adaptive thresholds and autofix feedback.
- `V5` and `V6` make long-running verification state survive process boundaries.
- `V7` activates the higher-rung gates only after capability discovery and artifacts are clearer.
- `V8` hardens the verdict-signal contract after the runtime path is less misleading.

---

## Document Index

| File | Docs Covered | Items | Status |
|------|--------------|-------|--------|
| [A-gate-foundation.md](A-gate-foundation.md) | 00, 01 (Gate trait + gate implementations) | A.01-A.14 | 12 DONE / 1 PARTIAL / 1 SCAFFOLD |
| [B-pipeline-rungs.md](B-pipeline-rungs.md) | 02, 03 (selector + pipeline) | B.01-B.12 | 9 DONE / 2 PARTIAL / 1 NOT DONE |
| [C-artifacts-ratcheting.md](C-artifacts-ratcheting.md) | 04, 05 (ArtifactStore + GateRatchet) | C.01-C.10 | 6 DONE / 2 PARTIAL / 2 NOT DONE |
| [D-feedback-thresholds.md](D-feedback-thresholds.md) | 06, 08 (AdaptiveThresholds + GateFeedback) | D.01-D.14 | 7 DONE / 3 PARTIAL / 4 NOT DONE |
| [E-process-rewards-lifecycle.md](E-process-rewards-lifecycle.md) | 07, 09 (Process rewards + evaluation lifecycle) | E.01-E.14 | 2 DONE / 3 PARTIAL / 9 NOT DONE |
| [F-autonomous-evoskills.md](F-autonomous-evoskills.md) | 10, 11 (Autonomous eval + EvoSkills) | F.01-F.18 | 2 DONE / 3 PARTIAL / 13 NOT DONE |
| [G-forensic-verdict-signals.md](G-forensic-verdict-signals.md) | 12, 15 (Forensic replay + verdict signals) | G.01-G.14 | 5 DONE / 2 PARTIAL / 7 NOT DONE |
| [BATCHES.md](BATCHES.md) | — | 8 batches | Execution contract |
| [SOURCE-INDEX.md](SOURCE-INDEX.md) | — | Verified code anchors | Reference |

Doc `INDEX.md` is absorbed into this file.

---

## Overall Parity: 43/96 items DONE (45%)

The verification stack is not in the same shape as composition or agent core:

- the **gate implementations themselves are mostly real and well-tested**,
- but the **runtime only exercises a small subset of that surface**,
- and much of the “adaptive” or “learning from verification” story is **half-wired**,
- while the advanced reward / replay / EvoSkills material is still **design-level**.

### Tier 1 — Should Exist Now (self-hosting relevant)

| ID | Title | Status | Severity |
|----|-------|--------|----------|
| B.04 | `run_gate_rung()` is hardcoded and numerically inconsistent with `Rung` | PARTIAL | HIGH |
| B.05 | `select_rungs(..., prior_failures)` has no production caller | PARTIAL | HIGH |
| D.03 | `suggested_max_retries(rung)` trains but does not affect orchestrator retries | PARTIAL | HIGH |
| D.04 | `should_skip_rung(rung)` is displayed but never enforced at runtime | PARTIAL | HIGH |
| D.10 | `feedback_for_agent` has zero orchestrator callers | PARTIAL | HIGH |
| C.08 | `GateRatchet` is fully built but unused in runtime decisions | PARTIAL | HIGH |

### Tier 2 — Should Exist Soon (operational quality)

| ID | Title | Status | Severity |
|----|-------|--------|----------|
| C.05 | `ArtifactStore` is in-memory only; `.roko/artifacts/` layout is absent | NOT DONE | MEDIUM |
| C.09 | `GateRatchet` has no persistence / resume path | NOT DONE | MEDIUM |
| G.08 | lineage exists, but no verification pass checks signal-chain integrity | PARTIAL | MEDIUM |
| G.10 | verdict engrams lack the doc's explicit 24h half-life and explicit tag propagation | PARTIAL | MEDIUM |
| A.04 | docs undercount the actual gate inventory and blur concrete vs scaffold gates | PARTIAL | MEDIUM |
| F.07 | `SkillLibrary` extraction is wired, but the 5+ validated-use promotion rule does not exist | PARTIAL | MEDIUM |

### Tier 3 — Future / Design-Heavy

| ID | Title | Status | Severity |
|----|-------|--------|----------|
| D.07 | SPC detector stack (CUSUM / EWMA / BOCPD / PELT / Hotelling T²) | NOT DONE | LOW |
| E.01-E.08 | Promise / Progress / PRM / DPO / reward-shaping systems | NOT DONE | LOW |
| E.11 | Gauntlet benchmark harness | NOT DONE | LOW |
| F.01-F.04 | autonomous test-writer / adversarial generation workflow | NOT DONE | LOW |
| F.08, F.11-F.18 | adversarial EvoSkills + cross-model + evolutionary search | NOT DONE | LOW |
| G.05-G.07 | replay algorithm, what-if analysis, pre-certified templates | NOT DONE | LOW |
| G.12-G.14 | verdict trend detection, clustering, replanning, predictive selection | NOT DONE | LOW |

### Already Shipped

| ID | Title | Status |
|----|-------|--------|
| A.01-A.03 | `Gate` trait + `name()` + `Verdict` helpers | DONE |
| A.05-A.10 | Shell / Compile / Clippy / Test / Symbol / Diff gates | DONE |
| A.12-A.14 | timeouts, `kill_on_drop`, `GatePayload` | DONE |
| B.01-B.03 | `PlanComplexity`, `Rung`, `select_rungs`, `RungCaps` | DONE |
| B.06-B.09 | `GatePipeline`, short-circuiting, verdict aggregation, merged `TestCount` | DONE |
| C.01-C.04 | in-memory content-addressed `ArtifactStore` | DONE |
| C.07 | `GateRatchet` API + tests | DONE |
| D.01-D.02, D.05-D.06 | `AdaptiveThresholds` update + persistence half | DONE |
| D.08-D.09, D.11 | `GateFeedback` classifier + serde | DONE |
| E.13-E.14 | efficiency-event capture + one narrow consolidation loop | DONE |
| F.06 | episodes + playbook-rules tiers | DONE |
| F.10 | skill injection into prompt composition | DONE |
| G.01-G.04, G.09 | signal / episode / efficiency foundations + verdict emission | DONE |

---

## Execution Boundaries

These are real gaps, but they should usually be handled outside batch `04`:

| Item | Better Home | Why |
|------|-------------|-----|
| Promise / Progress / PRM / reward-shaping work | `tmp/docs-parity/05` | learning owns reward-model semantics |
| autonomous test-writer agents and adversarial generation | `tmp/docs-parity/05` or later evaluation pass | verification owns the consumer gates, not the generator-agent architecture |
| advanced EvoSkills (`SkillGenome`, MAP-Elites, speciation, AURORA, CMA-ES) | `tmp/docs-parity/05` or post-parity research pass | this is learning / search infrastructure, not harness activation |
| replay analytics, what-if analysis, replanning, predictive gate selection | later retrospective / learning pass | depends on a stable runtime signal contract first |
| SPC detector stack from doc 06 §11-§16 | later analytics hardening pass | not needed to make current verification adaptive |

Batch `04` should usually produce:

- one honest canonical runtime path for verification,
- adaptive behavior that actually affects runtime decisions,
- persisted long-running verification state,
- and explicit handoffs for the learning-heavy or retrospective-heavy designs.

---

## Critical Verification Issues

1. **The runtime verification surface is much smaller than the gate crate surface.** Most shipped gate code is not actually reachable from the orchestrator.
2. **Adaptive thresholds currently learn without acting.** The system updates and persists EMAs, but runtime retry and skip behavior does not consume them.
3. **Autofix still sees raw gate output instead of structured gate feedback.** The best classifier in the stack does not currently shape the retry prompt.
4. **State meant for long-running convergence is ephemeral or dormant.** `ArtifactStore` is memory-only and `GateRatchet` has no runtime or persistence wiring.
5. **Verdict signals exist, but their contract is weaker than the docs imply.** There is no explicit 24h decay on the engram builder path, no explicit tag propagation, and no chain-integrity verifier.

---

## Key Insight

Batch `04` does **not** primarily suffer from missing gate implementations.

It suffers from a narrower problem:

**verification code ships, but the runtime still behaves like only three gates, one retry style, and one raw-error prompt path exist.**

That means the highest-value work here is usually:

1. make the orchestrator use the canonical rung / gate / pipeline abstractions,
2. make thresholds and feedback influence real runtime behavior,
3. persist verification state so long-running runs can benefit from prior work,
4. keep reward-model, autonomous-eval, and replay-research work deferred unless the batch only needs a narrow contract cleanup.

---

## Batch 04 Success Definition

Batch `04` is successful when:

- the production verification path is driven by canonical rung selection rather than ad-hoc numeric matches,
- adaptive thresholds and feedback affect real retry / skip / autofix behavior,
- ratchet and artifact state can survive long-running or resumed runs,
- verdict signals have an explicit, inspectable contract,
- and the design-heavy reward / EvoSkills / replay sections are cleanly deferred instead of being half-implemented.
