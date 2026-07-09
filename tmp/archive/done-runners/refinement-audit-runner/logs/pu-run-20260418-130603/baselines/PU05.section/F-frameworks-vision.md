# F — Frameworks + Vision (Docs 12, 17)

Parity analysis of `docs/05-learning/12-self-improvement-frameworks.md` and
`docs/05-learning/17-adas-and-autocatalytic.md` vs the actual codebase.

Both docs are primarily literature/thesis: doc 12 is a framework survey mapping
published papers to existing modules; doc 17 is a falsifiability thesis about
autocatalytic improvement. Items below check whether each mapped module exists,
not whether the framework is a literal code artifact. For "mapping" items the
framework name is not expected in the code — only the pattern it describes.

---

## F.01 — Reflexion (Shinn et al. 2023) → playbook confidence dynamics

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 12 — Reflexion maps to the playbook rule system with confidence tracking: validate +0.05, contradict −0.10.
**Reality**: `crates/roko-learn/src/playbook_rules.rs:341-350` implements `record_outcome(rule_id, validated)` with the exact bounds the doc cites: `confidence = (confidence + 0.05).min(0.95)` on validation, `confidence = (confidence - 0.10).max(0.0)` on contradiction. Module header comment at `playbook_rules.rs:9-11` documents the same constants. 1355 LOC.
**Notes**: The code is the implementation of the Reflexion pattern; the paper name is not referenced in source and doesn't need to be.

---

## F.02 — ExpeL (Zhao et al. 2023) → skill library + playbook rules

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 12 — ExpeL-style experience extraction: successful episodes produce skills (positive experiences), failure patterns produce playbook rules (negative experiences). Both persist across sessions and grow monotonically.
**Reality**: `crates/roko-learn/src/skill_library.rs` — 2495 LOC. `Skill` struct at `skill_library.rs:63` with source-episode provenance (`source_episodes` field, line 86). `extract_skill` at `skill_library.rs:1246` generates skills from episode inputs; `register` at `skill_library.rs:1045` persists them. Paired with `playbook_rules.rs` for the negative-experience side. Both are on-disk JSON stores that accumulate across sessions.
**Notes**: Pattern matches ExpeL's dual positive/negative extraction. Confidence bounds on playbook rules are a Roko addition beyond ExpeL's natural-language insight format.

---

## F.03 — DSPy (Khattab et al. 2023) → ExperimentStore / prompt experiments

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 12 — DSPy-style prompt optimization implemented via `ExperimentStore` with UCB1 bandit variant selection and gate-pass-rate evaluation.
**Reality**: `crates/roko-learn/src/prompt_experiment.rs` — 678 LOC. `PromptVariant` at line 22, `VariantStats` at line 53 with a UCB1-style score field at line 71, `PromptExperiment::assign_variant` at `prompt_experiment.rs:167` performs UCB1-based variant selection. `ExperimentStore` (same module) is wired into orchestrate.rs as referenced by the current-state table in `CLAUDE.md`.
**Notes**: Doc is explicit that Roko's online bandit approach differs from DSPy's static compile-then-select flow. Code matches the adapted description.

---

## F.04 — RouteLLM (Ong et al. ICLR 2025) → cascade_router 3-stage gating

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 12 — The cascade router's confidence stage implements empirical pass rates with confidence intervals; the LinUCB stage provides context-dependent routing similar to RouteLLM's classifier.
**Reality**: `crates/roko-learn/src/cascade_router.rs:63-70` defines `CascadeStage::{Static, Confidence, UCB}`. Module header at `cascade_router.rs:6-10` lists the three stages with the exact observation thresholds doc 12 describes (`<50`, `50–200`, `>200`). LinUCB stage uses `LinUCBRouter` imported at `cascade_router.rs:45`. 4766 LOC total.
**Notes**: Roko uses linear contextual bandits in place of RouteLLM's neural classifier — the doc flags this intentional substitution.

---

## F.05 — FrugalGPT (Chen 2305.05176) → CascadeModel primary + fallback

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 12 — The `CascadeModel` includes both a primary and a fallback model; on primary failure the orchestrator retries with the fallback.
**Reality**: `crates/roko-learn/src/cascade_router.rs:107-118` defines `CascadeModel { primary, fallback_chain, context_overflow_fallback, stage }`. `CascadeModel::model_for_attempt` at line 126 walks primary → fallback chain by attempt index. `fallback_for_error` at line 135 picks a cross-backend fallback for provider-specific errors.
**Notes**: The doc caveats that Roko's three-stage cascade is a different dimension from FrugalGPT's cost-cascade: strategy complexity rather than model cost. The primary+fallback mechanism itself is the FrugalGPT-shaped escalation.

---

## F.06 — Voyager (Wang 2023) → skill_library monotonic accumulation

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 12 comparison matrix — Voyager input/output maps to the skill library with monotonic growth. Cross-referenced to `02-skill-library-voyager.md`.
**Reality**: `SkillLibrary::register` at `skill_library.rs:1045` inserts without deletion; `len` at line 1068 counts accumulated skills; `is_empty` at line 1073 reports zero state. There is no delete or prune API in the public surface — the library is write-and-read. `source_episodes` on each `Skill` (line 86) preserves provenance across sessions.
**Notes**: Monotonic accumulation is the architectural property — `register` never overwrites to zero, no garbage-collect path. Matches Voyager's skill-library semantics.

---

## F.07 — ImprovementScoreCard + PeriodMetrics + SignificanceTests

**Status**: NOT DONE
**Severity**: HIGH (doc prescribes types that do not exist)
**Doc claim**: Doc 12 §"Improvement Measurement" — `ImprovementScoreCard`, `PeriodMetrics`, `SignificanceTests`, `Confound` structs with fields for z-tests, Welch's t-tests, Mann-Whitney U, KL divergence, and statistical-significance gating at α=0.05. Plus `ImprovementExperiment` holdout design at 80/20.
**Reality**: `rg 'ImprovementScoreCard|PeriodMetrics|SignificanceTests' crates/` returns zero hits. No holdout-experiment runner exists. The four key metrics (first-attempt pass rate, iterations per plan, cost per plan, prompt tokens per spawn) are implicitly derivable from existing logs but no struct consolidates them with significance testing.
**Fix sketch**: Either (a) add a measurement module in `roko-learn` that materialises these structs and computes significance tests from `.roko/episodes.jsonl`, or (b) demote doc 12 §"Improvement Measurement" from `> **Implementation**: Shipping` to `Planned`.

---

## F.08 — GateGamingDetector + SafetyInvariants + ConstitutionalConstraints

**Status**: NOT DONE
**Severity**: HIGH (safety-critical types missing)
**Doc claim**: Doc 12 §"Improvement Safety" — `SafetyInvariants`, `SafetyViolation`, `GateGamingDetector`, and `ImprovementVelocityLimits` structs. Constitutional rules in `roko.toml [safety.constitution]` (gates_immutable, self_modification_forbidden_crates, min_quality_model_tier, quality_floor, self_mod_requires_review).
**Reality**: `rg 'GateGamingDetector|SafetyInvariants|ConstitutionalConstraints' crates/` returns zero hits. `rg 'ImprovementVelocityLimits|QualityFloor|max_downgrade' crates/` returns zero hits. No `[safety.constitution]` section in any shipped `roko.toml` template. Safety layer in `crates/roko-agent/src/safety/` handles per-tool role auth and pre/post checks (noted in CLAUDE.md) but does not implement the gate-gaming or constitutional-constraint surface doc 12 prescribes.
**Fix sketch**: Doc is describing a safety layer that has not been built. Either implement the detection pipeline or move the §"Improvement Safety" block to a "Planned" section with explicit status.

---

## F.09 — ADAS meta-agent (Hu et al. ICLR 2025)

**Status**: NOT DONE
**Severity**: — (doc itself labels ADAS "planned but not implemented")
**Doc claim**: Doc 17 §"ADAS" — Hu et al. meta-agent that searches agent architectures. Doc admits ADAS is "planned but not implemented" and describes a pathway using existing Roko components.
**Reality**: `rg 'AdasRunner|MetaAgent|Autocatalytic' crates/` returns zero hits. No meta-agent driver exists. The doc's "Roko's ADAS Pathway" table mapping ADAS requirements to existing components (roko.toml, gate pipeline, ExperimentStore, cascade router) is accurate — those components exist — but there is no orchestrator that uses them to search architecture space.
**Notes**: Doc epistemic framing is correct ("Both are speculative"). No fix needed unless the doc's `> **Implementation**: Shipping` header is meant to imply ADAS is shipped; if so, the header should be `Planned` or `Tier 3`.

---

## F.10 — Autocatalytic thesis falsification oracle → C-Factor trend

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 17 §"Empirical Validation" — the autocatalytic thesis is falsified if the C-Factor shows a linear or sub-linear trend after 500 episodes with all 8 loops wired. C-Factor is the measurement instrument.
**Reality**: `crates/roko-learn/src/cfactor.rs` — 1847 LOC. `CFactor` struct at line 16 with `components` (`CFactorComponents` at line 95). `trend_arrow` at `cfactor.rs:465` computes direction over a time window, `CFactorRegression` at line 127 + regression detection at line 501 produce the before/after comparison that a falsifiability check requires. History-walking logic at lines 471 and 510 supports windowed trend analysis.
**Notes**: The *instrument* exists. The doc's claim is that C-Factor trend is the falsification oracle — and the oracle (trend + regression tooling) is wired. No separate "autocatalytic thesis runner" is needed because the thesis is a statement about what the oracle should show, not a code artifact.

---

## F.11 — EvoSkills evolutionary skill optimization (Chen 2023)

**Status**: NOT DONE
**Severity**: — (doc explicitly labels "Tier 3, not implemented")
**Doc claim**: Doc 17 §"EvoSkills" — selection, crossover, mutation, fitness evaluation over the skill population. Doc states: "Not implemented. The current skill library only accumulates and tracks, it does not evolve skills. EvoSkills is a Tier 3 innovation."
**Reality**: `rg 'EvoSkills|EvoSkillsEvolution' crates/` returns zero hits. No crossover or mutation operator exists on `Skill` or `SkillLibrary`. `model_experiment.rs` (754 LOC) provides A/B selection over model variants but does not perform genetic recombination of skills.
**Notes**: Doc's own status line matches reality. No action required.

---

## F.12 — Pólya urn + flywheel mechanisms + network effects

**Status**: NOT DONE
**Severity**: LOW (theoretical scaffolding, no code claim)
**Doc claim**: Doc 17 §"Network Effects" and §"Flywheel Mechanisms" — invokes Metcalfe's Law (N²), Reed's Law (2^N), Loreto & Tria Pólya urn model (2014), and lists ten flywheel mechanisms.
**Reality**: `rg 'PolyaUrn|FlywheelMechanism|NetworkEffect' crates/` returns zero hits. The ten mechanisms in the doc's table (skill accumulation, pattern extraction, playbook rules, model routing, cache optimization, prompt optimization, calibration, crate familiarity, cross-project transfer, meta-optimization) map to real components — but "flywheel" as a data-modelled construct does not exist. The scaling laws are theoretical framing, not code.
**Notes**: Doc prose treats these as analytical frames, not as artifact claims. Only flag if the doc intends to prescribe structs (it does not). No fix needed.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 7 |
| NOT DONE | 5 |

| Doc | DONE | NOT DONE |
|-----|------|----------|
| 12 (frameworks survey) | 6 (F.01–F.06) | 2 (F.07, F.08) |
| 17 (ADAS/autocatalytic thesis) | 1 (F.10) | 3 (F.09, F.11, F.12) |

The framework-to-module mappings in doc 12 are accurate: every framework with a "Roko implementation" callout (Reflexion, ExpeL, DSPy, RouteLLM, FrugalGPT, Voyager) points at code that implements the pattern. The two high-severity gaps in doc 12 are both **prescriptive** blocks: §"Improvement Measurement" (F.07) invents scorecard/significance-test types that do not exist, and §"Improvement Safety" (F.08) prescribes a constitutional-constraint layer that is not built — both are followed by `> **Implementation**: Shipping` at the top of the doc, which is misleading.

Doc 17 is more honest: it explicitly labels ADAS and EvoSkills as planned/Tier 3, so F.09 and F.11 are not drift — they are self-consistent. F.10 (C-Factor as falsification oracle) is the only real DONE item in doc 17 because C-Factor is a genuine measurement artifact, not a theoretical claim. F.12 is theoretical framing with no code contract, so "NOT DONE" is accurate but low-severity.

## Agent Execution Notes

### F.07 / F.08 — Docs-Honesty And Handoff

This is mostly a truth-in-advertising batch.

Recommended slice:

1. mark improvement-measurement and safety blocks as deferred,
2. add explicit handoff notes to a later eval / governance pass,
3. keep the framework mappings useful without overstating what ships.

Acceptance criteria:

- later agents are not misled into thinking scorecards or constitutional safety layers already exist,
- the docs still preserve the conceptual mapping value,
- batch `05` does not widen into a governance-system implementation.

### F.09-F.12 — Usually Defer

ADAS, EvoSkills optimization, and broader autocatalytic/evolutionary framing belong to later research passes unless a future batch explicitly narrows them into concrete runtime work.
