# 04-Verification Parity Refresh

Audit-aligned refresh of `docs/04-verification/` parity materials.

Generated: 2026-04-18

---

## Post-Audit Posture

The old parity pack treated verification as a large runtime-activation backlog. That is no longer an honest reading of the code.

Current posture for this section:

- `A-D` are **substantially shipped** and should be documented as current runtime/foundation behavior.
- `C` still has **real partials** around artifact persistence and ratchet runtime use.
- `D` has **real EMA wiring** and persistence, with some advisory/read-side behavior still partial.
- `E-F` are **DEFERRED** research/design material, not current implementation work.
- `G` must be **split**: the `GateVerdict` path is live, the forensic replay system is not.

---

## Section Index

| File | Docs Covered | Audit Posture | What Changed |
|---|---|---|---|
| [A-gate-foundation.md](A-gate-foundation.md) | 00, 01 | `keep` + `rewrite` | Gate trait works; treat the verification foundation as present, not hypothetical |
| [B-pipeline-rungs.md](B-pipeline-rungs.md) | 02, 03 | `rewrite` | The live 7-rung runtime path is in the executor/plan flow, but the production path is `orchestrate.rs -> rung_dispatch.rs`, not the full selector-first story |
| [C-artifacts-ratcheting.md](C-artifacts-ratcheting.md) | 04, 05 | `narrow` | Artifact and ratchet foundations are real; persistence/runtime use are still partial |
| [D-feedback-thresholds.md](D-feedback-thresholds.md) | 06, 08 | `narrow` | EMA thresholds, persistence, and gate-result learning hooks are wired; advanced analytics remain future work |
| [E-process-rewards-lifecycle.md](E-process-rewards-lifecycle.md) | 07, 09 | `defer` | Keep only the small data-foundation truth; move the reward-model architecture out of the shipped story |
| [F-autonomous-evoskills.md](F-autonomous-evoskills.md) | 10, 11 | `defer` | Distinguish real episode/skill plumbing from unbuilt autonomous eval and EvoSkills research |
| [G-forensic-verdict-signals.md](G-forensic-verdict-signals.md) | 12, 15 | `rewrite` | Separate the shipped verdict-signal path from the deferred forensic replay system |
| [SOURCE-INDEX.md](SOURCE-INDEX.md) | — | `rewrite` | Fresh source anchors; remove stale `orchestrate.rs` line references |

---

## Gap Picture After The Audit

### What Is Shipped

- `roko-core`'s `Gate` trait is live and unchanged.
- `roko-gate` contains a real verification surface, not just scaffolding.
- Gate execution is called from `ExecutorAction::RunGate` in `orchestrate.rs`.
- The live runtime executes a 7-rung dispatch path through `roko-gate/src/rung_dispatch.rs`.
- Gate runs produce episodes, executor-state updates, adaptive-threshold updates, and `Kind::GateVerdict` signals.
- Adaptive thresholds persist to `.roko/learn/gate-thresholds.json`.

### What Is Still Partial

- `rung_selector.rs` and `gate_pipeline.rs` are real library abstractions, but they are not the primary production entrypoint today.
- `ArtifactStore` is still an in-memory foundation, not a persisted `.roko/artifacts/` catalog.
- `GateRatchet` exists as a tested module, but this pack should not describe it as an active runtime guardrail.
- Threshold advisories and `GateFeedback` exist and should be documented honestly, but not overstated as fully closed-loop orchestration policy.

### What Must Move Out Of The Shipped Story

- Process reward models
- Promise / Progress scoring
- 14-loop lifecycle as current runtime fact
- Autonomous eval generation
- EvoSkills and evolutionary-search layers
- Forensic replay, root-cause analysis, what-if analysis, verdict clustering, predictive gate selection

Those are valid future directions. They are not current parity gaps for `04`.

---

## Working Rules For This Pack

1. Describe runtime behavior in present tense only when the codepath is live.
2. Keep `rung_dispatch.rs` and `run_gate_pipeline(...)` central in the narrative.
3. Treat `rung_selector.rs`, `GatePipeline`, `GateRatchet`, and `GateFeedback` as shipped foundations unless the runtime hook is explicitly missing.
4. Mark research-heavy material as `DEFERRED`, not `NOT DONE`.
5. Prefer small, truthful future-work notes over new execution backlogs.

---

## Success Definition

This parity pack is correct when:

- A reader understands that verification core (`A-D`) is mostly shipped.
- A reader understands that the live runtime path is 7-rung executor/plan verification, but narrower than the docs' canonical selector model.
- E and F are clearly labeled `DEFERRED`.
- G clearly distinguishes live verdict signals from unbuilt forensic replay.
- `SOURCE-INDEX.md` points at current source locations instead of stale pre-refresh anchors.
