# F — Autonomous Evaluation & EvoSkills (Docs 10, 11)

Parity analysis of `docs/04-verification/10-autonomous-eval-generation.md` and
`docs/04-verification/11-evoskills.md` vs the actual codebase.

---

## F.01 — Three-stage test generation pipeline (doc 10 §2)

**Status**: NOT DONE (LOW severity)
**Doc claim**: Doc 10 §2 — Stage 1 test generation by dedicated agent, Stage 2 validation (should fail pre-impl, pass post-impl), Stage 3 registration with `GeneratedTestGate`.
**Reality**: `GeneratedTestGate` exists (820 LOC at `crates/roko-gate/src/generated_test_gate.rs`) and can execute a test suite, **but**: no test-generation agent role exists. `grep -rn 'TestGeneratorAgent\|TestGenerator\|AgentRole::TestGenerator' crates/` returns nothing. The `AgentRole` enum has Implementer/AutoFixer/Reviewer/Scribe/etc., but no TestWriter.
**Notes**: The gate is built but the generator side is absent, so "Autonomous evaluation generation" (the doc's title) does not run end-to-end. See B.04 — even if a generator existed, `GeneratedTestGate` is unreachable from `run_gate_rung`.

---

## F.02 — Adversarial separation of generation from implementation (doc 10 §4)

**Status**: NOT DONE (LOW severity)
**Doc claim**: Doc 10 §4 — "Test generation and implementation are performed by different agents" as the key architectural commitment.
**Reality**: There is only one implementation role (`AgentRole::Implementer`). No parallel test-generation agent. The adversarial setup doc describes does not exist.

---

## F.03 — Generation-Verification Gap enforcement (doc 10 §5)

**Status**: NOT DONE (LOW severity)
**Doc claim**: Doc 10 §5 — ensure test generation is at least as sophisticated as implementation (capable model for tests, include edge cases, validate tests fail pre-impl).
**Reality**: Not applicable; no generator exists. Doc §5 is philosophy.

---

## F.04 — Cheap-model convergence loop (doc 10 §6)

**Status**: NOT DONE (LOW severity)
**Doc claim**: Doc 10 §6 describes a within-attempt loop: cheap model generates → generated tests run → feedback → iterate → submit to full gate pipeline if converged.
**Reality**: Cascade router exists with model tiering (`roko-learn/src/cascade_router.rs`) but has no "inner convergence loop with generated tests" semantics. Model selection is single-pass per task.

---

## F.05 — Immutable verification artifacts before implementation (doc 10 §7)

**Status**: PARTIAL (LOW severity)
**Doc claim**: Doc 10 §7 — generated tests stored as immutable BLAKE3-hashed artifacts in `ArtifactStore` before implementation starts, so impl agent cannot tamper.
**Reality**: `ArtifactStore` is content-addressed and append-only (C.01–C.04). But since no generator produces pre-impl tests today, this capability is unused. Infrastructure exists; workflow does not.

---

## F.06 — Tier 1: Episodes + Tier 2: Patterns (doc 11 §2)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 11 §2.1 — episodes in `.roko/episodes.jsonl`. §2.2 — patterns extracted when 5+ similar episodes agree.
**Reality**:
- **Tier 1 (Episodes)**: `EpisodeLogger` at `crates/roko-learn/src/episode_logger.rs:789` with `append` / `read_all` / `read_all_lossy`. `Episode` struct at `episode_logger.rs:169`. Embedded `GateVerdict` struct at `episode_logger.rs:90-100` (with `gate: String`, `passed: bool`, `signature: Option<String>`). Runtime wiring at `orchestrate.rs` via `runtime_feedback.rs:325+` — `EpisodeLogger::new(&paths.episodes_jsonl)` instantiates the logger.
- **Tier 2 (Patterns)**: `PlaybookRules` at `crates/roko-learn/src/playbook_rules.rs:173-803`, including `Rule`, `Triggers`, `upsert`, `select`, `save`. Persisted to `.roko/learn/playbook-rules.toml` (runtime_feedback.rs:127). Upserted from successful outcomes (runtime_feedback.rs:829-831).

Both tiers exist and are wired.

---

## F.07 — Tier 3: Playbook / SkillLibrary (doc 11 §2.3)

**Status**: PARTIAL (MEDIUM severity)
**Doc claim**: Doc 11 §2.3 — when a pattern is used 5+ times, promote to playbook. Playbook injected into prompts via "skills" section with confidence/usage telemetry.
**Reality**: `SkillLibrary` at `crates/roko-learn/src/skill_library.rs:404-803` with `Skill` struct, `new(path)`, `extract_skill(request)`. Persisted to `.roko/learn/skills.json`. Skill extraction is wired narrowly:
- `orchestrate.rs:11204-11219` — on full-gate-pass, populate a pending `SkillRequest` (with gate verdicts).
- `orchestrate.rs:11361-11381` — on successful merge, call `self.skill_library.extract_skill(request)`.
- `orchestrate.rs:5556-5566` — implementer tracks `last_skill_request`.

**Gap**: the "5+ applications before promotion" rule from doc §2 is not enforced. Extraction happens on single successful plan (all-gates-pass + merge). There's no running count of "times this skill was applied successfully" before marking it Tier 3.
**Fix sketch**: Add an `apply_count` field to `Skill`, track applications via section-effectiveness style hooks, promote to playbook only when threshold met.

---

## F.08 — Adversarial Surrogate Verification (doc 11 §3)

**Status**: NOT DONE (LOW severity)
**Doc claim**: Doc 11 §3.1 — generate surrogate test suite to break skills; §3.2 cross-model testing; §3.3 confidence scoring `= validations / (validations + failures) × cross_model_factor`.
**Reality**: `grep -rn 'surrogate\|adversarial_verification\|cross_model_factor' crates/` returns zero matches. No adversarial testing infrastructure.

---

## F.09 — Skill confidence + telemetry fields (doc 11 §4)

**Status**: PARTIAL (LOW severity)
**Doc claim**: Doc 11 §4 `Skill { id, name, precondition, procedure, postcondition, confidence, source_episodes, validations, failures, task_categories, created_at, last_validated_at }`.
**Reality**: Actual `Skill` struct in `crates/roko-learn/src/skill_library.rs` contains a subset (id, name, procedure text, created_at). The richer fields (confidence, source_episodes, validations, failures, task_categories, last_validated_at) are partial or absent — skills are single-event extractions rather than accumulated-over-time records.
**Notes**: Exact field set requires reading the full skill_library.rs; this item is marked PARTIAL pending that confirmation.

---

## F.10 — Skill injection into agent context (doc 11 §5)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 11 §5 — validated skills injected into system prompt as a dedicated "Relevant Skills" section.
**Reality**: `SystemPromptBuilder` at `crates/roko-compose/src/system_prompt_builder.rs` has a `with_skills(Vec<Skill>)` method and a Layer 6/7 that emits skills (from 03-composition parity B.07). Wired via `RoleSystemPromptSpec` in `orchestrate.rs`. Confirmed as DONE in docs-parity/03/B-system-prompt-builder.md.

---

## F.11 — Cross-model transfer (doc 11 §6)

**Status**: NOT DONE (LOW severity)
**Doc claim**: Doc 11 §6 — +35-44pp cross-model transfer improvement; skills encode task-completion knowledge, not model-specific behaviors.
**Reality**: No cross-model validation code. Skills are extracted once and injected into prompts without per-model testing.

---

## F.12 — Skill evolution: refinement / specialization / retirement (doc 11 §7)

**Status**: NOT DONE (LOW severity)
**Doc claim**: Doc 11 §7.1–§7.3 — on failure, refine/specialize; on confidence drop, retire to archive.
**Reality**: No evolution logic — skills don't refine or specialize. Retirement is ad-hoc at best.

---

## F.13 — Skill Genome representation (doc 11 §12)

**Status**: NOT DONE (LOW severity, Phase 2+)
**Doc claim**: Doc 11 §12.1 `SkillGenome` with prompt_template, tool_preferences, retry_config, temperature, token_budget, gate_weights, behavioral descriptor, fitness.
**Reality**: `grep -rn 'SkillGenome\|BehavioralDescriptor\|RetryGenome' crates/` returns zero matches.

---

## F.14 — MAP-Elites archive (doc 11 §13)

**Status**: NOT DONE (LOW severity, Phase 2+)
**Doc claim**: Doc 11 §13.1 `SkillArchive` with cells/resolution/dimensions/insert/coverage/qd_score.
**Reality**: `grep -rn 'SkillArchive\|MapElites\|InsertResult\|qd_score' crates/` returns zero matches.

---

## F.15 — Mutation / crossover operators (doc 11 §13.3)

**Status**: NOT DONE (LOW severity, Phase 2+)
**Doc claim**: Doc 11 §13.3 — `mutate`, `crossover`, uniform recombination, Gaussian perturbation.
**Reality**: Absent. No evolutionary operators.

---

## F.16 — Fitness landscape analysis (doc 11 §15)

**Status**: NOT DONE (LOW severity, Phase 2+)
**Doc claim**: Doc 11 §15.1 `LandscapeAnalysis` with local_optima/ruggedness/neutrality/FDC/evolvability.
**Reality**: `grep -rn 'LandscapeAnalysis\|ruggedness\|evolvability' crates/` returns zero matches.

---

## F.17 — Speciation via NEAT compatibility distance (doc 11 §16)

**Status**: NOT DONE (LOW severity, Phase 2+)
**Doc claim**: Doc 11 §16.1 `CompatibilityMetric`; §16.2 `SpeciesManager` with fitness sharing and stagnation detection.
**Reality**: `grep -rn 'Speciation\|SpeciesManager\|CompatibilityMetric' crates/` returns zero matches.

---

## F.18 — AURORA learned descriptors + CMA-ES (doc 11 §17, §18)

**Status**: NOT DONE (LOW severity, Phase 2+)
**Doc claim**: Doc 11 §17 `AuroraDescriptor` with VAE encoder; §18 `SkillCmaEs` for continuous parameters.
**Reality**: `grep -rn 'AuroraDescriptor\|TraceEncoder\|SkillCmaEs' crates/` returns zero matches.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 2 (F.06 episodes+patterns wired, F.10 skill injection) |
| PARTIAL | 3 (F.05 artifact store exists but workflow absent, F.07 narrow extraction, F.09 skill fields subset) |
| NOT DONE | 13 (F.01–F.04 autonomous gen, F.08 adversarial, F.11–F.12 cross-model / evolution, F.13–F.18 MAP-Elites / speciation / AURORA / CMA-ES) |

**Doc 10 (Autonomous Eval Generation)**: The `GeneratedTestGate` exists (820 LOC) and the `ArtifactStore` provides content-addressed immutability (C.01). **But** the test-generation **agent** does not exist. No `TestGenerator` role, no pre-impl validation workflow, no cheap-model convergence loop. The gate is the *consumer* of an autonomous evaluation system that has no producer.

**Doc 11 (EvoSkills)**: Tier 1 (episodes) and Tier 2 (patterns) are fully wired (F.06). Tier 3 (playbook / skill library) is narrowly wired — single-event extraction on all-gates-pass + merge (F.07). Skill injection into prompts works (F.10). Everything from doc §12 onwards (SkillGenome, MAP-Elites, mutation operators, landscape analysis, speciation, AURORA, CMA-ES) is design-only. The headline metric (32% → 75% with EvoSkills) is a reference-system result, not a measurement of the current code.

**Recommendation**: Doc 11 §12–§18 should be prefixed with "Design — not started; current implementation is §1–§11 minus §3, §6, §7". Doc 10's three-stage pipeline should state explicitly that only the gate side (Stage 3 consumer) is implemented; Stages 1–2 are design.

## Agent Execution Notes

### F.01 / F.05 — Verification-Owned Consumer Side

The only batch-`04` work here that may be worth doing is making the consumer-side verification gates reachable when their inputs exist.

Recommended slice:

1. keep focus on `GeneratedTestGate` reachability and artifact availability,
2. make missing generator-side infrastructure explicit,
3. do not claim autonomous eval generation exists end-to-end.

Acceptance criteria:

- consumer-side gate activation is honest,
- missing producer-side agent architecture is called out clearly,
- batch `04` does not widen into a new agent-role design.

### F.07 — Skill Extraction Boundary

Treat the current `SkillLibrary` path as a downstream consumer boundary, not as ownership for EvoSkills policy. The missing 5+ validated-use promotion rule is better handled in `tmp/docs-parity/05`.

### F.08-F.18 — Defer By Default

Adversarial verification, cross-model transfer, skill evolution, MAP-Elites, speciation, AURORA, and CMA-ES are not batch-`04` work unless a later pass explicitly re-scopes them.
