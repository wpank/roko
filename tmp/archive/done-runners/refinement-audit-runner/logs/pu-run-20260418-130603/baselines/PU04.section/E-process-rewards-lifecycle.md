# E — Process Rewards & Evaluation Lifecycle (Docs 07, 09)

Parity analysis of `docs/04-verification/07-process-reward-models.md` and
`docs/04-verification/09-evaluation-lifecycle.md` vs the actual codebase.

---

## E.01 — Promise score function (doc 07 §2.1, §4)

**Status**: NOT DONE (MEDIUM severity)
**Doc claim**: Doc 07 §4 defines `Promise(attempt) = w₁·rung_fraction + w₂·test_pass_rate + w₃·error_trend + w₄·tool_efficiency` with thresholds for early termination.
**Reality**: `grep -rn 'PromiseScore\|promise_score\|promise(' crates/` returns zero matches. No struct, no function, no field.

---

## E.02 — Progress score function (doc 07 §2.2, §5)

**Status**: NOT DONE (MEDIUM severity)
**Doc claim**: Doc 07 §5 defines `Progress(attempt_n) = Δrung + Δtest_rate + Δerror_count` with thresholds for re-planning.
**Reality**: `grep -rn 'ProgressScore\|progress_score' crates/` returns zero matches. No per-attempt delta tracking of any kind.

---

## E.03 — ToolCallMeta with `advanced_task` / `was_redundant` / `error_category` (doc 07 §3.1)

**Status**: NOT DONE (LOW severity)
**Doc claim**: Doc 07 §3.1 shows `ToolCallMeta { tool_name, duration_ms, result_tokens, succeeded, advanced_task, was_redundant, error_category }`.
**Reality**: Per-tool-call timing is partially captured in efficiency events (see E.13) but the fields `advanced_task`, `was_redundant`, `error_category` do not exist in any crate. `grep -rn 'advanced_task\|was_redundant' crates/` returns zero matches.

---

## E.04 — Three-timescale feedback architecture (doc 07 §6)

**Status**: PARTIAL (MEDIUM severity)
**Doc claim**: Doc 07 §6 describes three interleaved control loops: per-turn (Promise → continue/terminate), per-attempt (Gate verdict → retry with adjusted prompt), across-attempts (escalation).
**Reality**:
- Per-turn intervention: absent (no Promise/Progress implementation).
- Per-attempt retry: exists at `orchestrate.rs:5261` (AutoFixer role dispatch) and `handle_autofix` at `orchestrate.rs:8897` — but uses raw verdict details, no Promise gating.
- Cross-attempt escalation: `PlanComplexity::escalate_by` exists at `rung_selector.rs:47-55` but has no caller (see B.05).

Only the middle loop is functional, and partially (feedback is not classified — see D.10).

---

## E.05 — Self-supervised PRM from gate verdicts (doc 07 §11)

**Status**: NOT DONE (LOW severity)
**Doc claim**: Doc 07 §11 describes per-step labeling from gate verdicts; §11.2 `MonteCarloStepLabeler` with 8 rollouts per step.
**Reality**: `grep -rn 'MonteCarloStepLabeler\|StepLabel\|StepQuality\|IntermediateState' crates/` returns zero matches. No rollout-based step scoring infrastructure.

---

## E.06 — ThinkPRM / FoVer generative verifiers (doc 07 §14, §11.3)

**Status**: NOT DONE (LOW severity)
**Doc claim**: Doc 07 §11.3 describes `FormalStepLabeler` with Z3/Isabelle/Prusti; doc §14 `ThinkPrm` generative reasoner.
**Reality**: `grep -rn 'ThinkPrm\|FormalStepLabeler\|FormalVerifier' crates/` returns zero matches.

---

## E.07 — RLHF alternatives: DPO, RLAIF, Constitutional (doc 07 §12)

**Status**: NOT DONE (LOW severity)
**Doc claim**: Doc 07 §12.1–12.3 — `DpoTrainingPair`, `RlaifConfig`, `SAFETY_CONSTITUTION`, dpo-pairs.jsonl persistence.
**Reality**: `grep -rn 'DpoTrainingPair\|RlaifConfig\|SAFETY_CONSTITUTION\|dpo-pairs' crates/` returns zero matches.

---

## E.08 — Potential-based reward shaping (doc 07 §13)

**Status**: NOT DONE (LOW severity)
**Doc claim**: Doc 07 §13 `GatePotential` with PotentialWeights; §13.3 `StepRewardComputer` combining sparse gate + dense shaped.
**Reality**: `grep -rn 'GatePotential\|StepRewardComputer\|PotentialWeights' crates/` returns zero matches.

---

## E.09 — 14-loop evaluation lifecycle (doc 09 §2)

**Status**: PARTIAL (LOW severity)
**Doc claim**: Doc 09 §2 lists 14 loops across 5 speed tiers: Machine (5), Cognitive (3), Consolidation (3), Retrospective (2), Meta (1).
**Reality**: From doc 09 §8's own status table, only ~7 are "Wired"; cross-checked against code:

| Doc Loop | Doc Status | Code Reality |
|---|---|---|
| 1. Confidence Calibration | — | Partial — efficiency events capture data at `.roko/learn/efficiency.jsonl`; no ECE computation |
| 2. Context Attribution | — | Partial — `SectionEffectivenessRegistry` exists in roko-learn (see 03-composition parity) |
| 3. Cost-Effectiveness | — | Partial — efficiency events track tokens/cost |
| 4. Tool Selection | — | Partial — tool calls logged, no efficiency analysis |
| 5. Adversarial Awareness | — | Not present |
| 6. Gate Pipeline | Wired | Present but hardcoded (see B.04) |
| 7. Error Diagnosis | Wired | `feedback.rs` exists but unwired (see D.10) |
| 8. Retry Logic | — | Partial — AutoFix exists at `orchestrate.rs:8897`, no Promise/Progress |
| 9. Skill Extraction | Scaffold | Wired narrowly — see F.07 |
| 10. Pattern Discovery | — | Not present |
| 11. Model Calibration | Wired | Cascade router persists to `.roko/learn/cascade-router.json` |
| 12. Shadow Testing | Design | Not present (zero references to `ShadowTest`) |
| 13. Reasoning Quality Review | — | Not present |
| 14. Meta-Learning Evaluation | Design | Not present |

Realistically, only ~2-3 loops are truly functional (gate pipeline, skill extraction narrow, model calibration). The rest are data-capture-only (loops 1-4) or absent.

---

## E.10 — Four-phase lifecycle (Trace/Backtest/Paper/Canary, doc 09 §5)

**Status**: PARTIAL (LOW severity)
**Doc claim**: Doc 09 §5 — four phases with distinct data sources.
**Reality**:
- Phase 1 Trace Inspection: data layer exists (`.roko/episodes.jsonl`, `.roko/learn/efficiency.jsonl`); no dedicated trace-inspection tool.
- Phase 2 Backtesting: not present; no replay-with-different-parameters.
- Phase 3 Paper Trading: not present.
- Phase 4 Canary Deployment: `ExperimentStore` at `.roko/learn/experiments.json` exists (prompt A/B). Partial.

---

## E.11 — Gauntlet benchmark (doc 09 §6)

**Status**: NOT DONE (LOW severity)
**Doc claim**: Doc 09 §6 — `Smoke/Nightly/Full` benchmark suite with 5min/2-4hr/24-48hr durations.
**Reality**: `grep -rn 'Gauntlet\|gauntlet' crates/` returns zero matches. No benchmark harness of any kind.

---

## E.12 — Karpathy property as a design constraint (doc 09 §4)

**Status**: NOT DONE (LOW severity, mostly philosophical)
**Doc claim**: Doc 09 §4 claims every loop is designed such that improving the metric improves end-to-end performance.
**Reality**: Not verifiable without the 14 loops being real. For the loops that exist (gate pass rate, model calibration), the property holds trivially. Marking this as not-done in the sense that it is not actively measured.

---

## E.13 — Efficiency events wired (doc 09 §8)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 09 §8 table — "Efficiency events (Loops 1–5): Wired".
**Reality**: 25 crate files reference `efficiency.jsonl` or `EfficiencyEvent`: `crates/roko-learn/src/efficiency.rs`, `crates/roko-learn/src/event_subscriber.rs`, `crates/roko-cli/src/orchestrate.rs` (emission sites), `crates/roko-serve/src/routes/learning.rs`, TUI pages. The data-capture side of Loops 1–5 is real; what's missing is downstream consumers that turn that data into decisions (E.09).

---

## E.14 — Single realized "consolidation loop" in orchestrate (doc 09 §7)

**Status**: DONE (narrowly)
**Severity**: —
**Doc claim**: Doc 09 §7 — "every loop is ultimately grounded in gate verdicts".
**Reality**: The only consolidation loop that fully closes today is at `orchestrate.rs:5284-5356`: gate verdict → episode logger append → runtime_feedback `enrich_completed_run` → skill library request (see F.07). This is one of the 14 designed loops, and it's a narrow one (skill extraction on full-gate-pass + merge). The remaining 13 loops are either data-capture-only or absent.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 2 (E.13 efficiency data capture, E.14 one consolidation loop) |
| PARTIAL | 3 (E.04 three-timescale partial, E.09 loop inventory partial, E.10 four-phase partial) |
| NOT DONE | 9 (E.01 Promise, E.02 Progress, E.03 rich ToolCallMeta, E.05–E.08 PRM components, E.11 Gauntlet, E.12 Karpathy enforcement) |

Docs 07 and 09 are almost **entirely aspirational** with respect to the current code. Doc 07 describes Promise + Progress as if they were a working system with interventions at three timescales — but `grep -rn 'PromiseScore\|ProcessReward\|GatePotential' crates/` returns nothing. Doc 09's "14 feedback loops" are mostly sketches: data is captured for loops 1–5 (efficiency events), one consolidation loop is closed (skill extraction, narrowly), and model calibration is real. The rest are design.

**Recommendation**: Mark docs 07 and 09 explicitly as "Design — not started" in their own headers so a reader doesn't mistake them for current architecture. Neither blocks self-hosting.

## Agent Execution Notes

### E.01-E.12 — Usually Defer From Batch 04

These sections are mostly here to prevent accidental overreach.

Default action:

1. verify absence,
2. record the deferral cleanly,
3. hand the work to `tmp/docs-parity/05` unless a later batch only needs one narrow contract stub.

Acceptance criteria:

- later agents are not misled into thinking reward-model code already exists,
- deferrals point to a better owning batch,
- batch `04` stays focused on verification runtime activation.

### E.13 / E.14 — Useful Boundary Markers

Use the existing efficiency-event and consolidation-loop code as evidence that data capture exists, but do not treat that as proof that the broader process-reward architecture is live.
