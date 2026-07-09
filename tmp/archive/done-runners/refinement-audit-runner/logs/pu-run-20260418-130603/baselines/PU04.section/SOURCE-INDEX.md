# SOURCE-INDEX — Code Anchors for 04-Verification Parity

Verified code references for batch `04`, organized by crate and focused on the runtime seams an agent is likely to touch.

Generated: 2026-04-16

---

## Important Corrections First

Use these before trusting the docs literally:

- `EngramBuilder::new(...)` defaults to `Decay::None` in `crates/roko-core/src/engram.rs:165-169`. The doc-15 24h verdict half-life is **not** implicit.
- `Engram::derive(...)` at `crates/roko-core/src/engram.rs:131-135` carries lineage only. It does **not** inherit tags from the parent engram.
- `run_gate_rung(...)` at `crates/roko-cli/src/orchestrate.rs:11423-11461` still uses ad-hoc numeric semantics (`1 => Test`, `2 => Clippy`) rather than the canonical `Rung` enum ordering.
- `select_rungs(...)` is fully implemented in `roko-gate`, but has **no production caller** today.
- `feedback_for_agent(...)` is fully implemented in `roko-gate`, but has **no production caller** today.
- `GeneratedTestGate` uses its own trait named `ArtifactStore` inside `generated_test_gate.rs`; that is separate from `crates/roko-gate/src/artifact_store.rs`.

---

## crates/roko-core/src/

### Gate trait + engram contract

| File | What | Section |
|------|------|---------|
| `traits.rs:102-108` | canonical `Gate` trait: `verify(&self, signal: &Engram, ctx: &Context) -> Verdict` plus `name()` | A.01-A.02 |
| `engram.rs:131-135` | `Engram::derive(kind, body)` — lineage helper, no automatic tag inheritance | G.08, G.10 |
| `engram.rs:165-169` | `EngramBuilder::new` defaults (`Decay::None`, neutral score, empty tags) | G.10 |
| `engram.rs:183-186` | explicit `.decay(...)` setter on builder | G.10 |
| `kind.rs:42` | `Kind::GateVerdict` enum variant | G.09-G.10 |
| `decay.rs:21-29` | `Decay::HalfLife { half_life_ms }` contract | G.10 |
| `decay.rs:104-106` | `Decay::WISDOM` constant = 24h half-life | G.10 |

### Secondary verdict representation

| File | What | Section |
|------|------|---------|
| `dashboard_snapshot.rs:123-129` | dashboard-facing `GateVerdict` struct distinct from the signal / learning types | G.10 |

---

## crates/roko-gate/src/

### Public entry + module inventory

| File | What | Section |
|------|------|---------|
| `lib.rs:17-39` | gate module declarations; current crate inventory | A.04 |
| `lib.rs:41-55` | public re-exports including `ArtifactStore`, `GateFeedback`, `GateRatchet`, gate types, and helpers | A.04 |

### Gate implementations

| File | What | Section |
|------|------|---------|
| `shell.rs:57-118` | `ShellGate::verify()` timeout / spawn / exit handling with `kill_on_drop(true)` | A.05, A.12 |
| `compile.rs:134-151` | `summarize_errors()` | A.06, A.13 |
| `clippy_gate.rs:70-93` | cargo `--` splicing logic | A.07 |
| `clippy_gate.rs:146-163` | lint summarization helper | A.13 |
| `test_gate.rs:166-241` | `parse_test_counts()` by `BuildSystem` | A.08 |
| `test_gate.rs:244-267` | test-failure summarization helper | A.13 |
| `symbol_gate.rs:177-202` | `SymbolGate` type + constructor | A.09 |
| `diff_gate.rs:98-119` | `DiffGate` type + default + `Gate` impl start | A.10 |
| `generated_test_gate.rs:69-123` | inner `ArtifactStore` trait + `InMemoryArtifactStore` | A.04, F.01, F.05 |
| `generated_test_gate.rs:173-239` | `GeneratedTestGate` type + constructor path | A.04, F.01 |
| `property_test_gate.rs:46-161` | `PropertyTestGate` constructors + `Gate` impl start | A.04 |
| `integration_gate.rs:143-250` | `IntegrationGate` constructors + `Gate` impl start | A.04 |
| `llm_judge_gate.rs` | scaffold gate; no production backend | A.11 |
| `verify_chain_gate.rs` | scaffold gate; no production caller | A.11 |
| `fact_check.rs` | scaffold gate + `SearchOracle` abstraction | A.11 |
| `code_exec.rs` | scaffold gate + backend abstraction | A.11 |

### Rung selection + pipeline

| File | What | Section |
|------|------|---------|
| `rung_selector.rs:24-34` | `PlanComplexity` enum | B.01 |
| `rung_selector.rs:36-55` | `escalate()` and `escalate_by(n)` | B.01, B.05 |
| `rung_selector.rs:62-80` | `Rung` enum with canonical discriminants | B.02 |
| `rung_selector.rs:83-107` | `CANONICAL_ORDER` and `Rung::label()` | B.02 |
| `rung_selector.rs:117-154` | `RungCaps` plus `all()` / `allows()` | B.03 |
| `rung_selector.rs:168-189` | `base_rungs()` complexity mapping | B.10 |
| `rung_selector.rs:207-214` | `select_rungs(complexity, caps, prior_failures)` | B.03, B.05 |
| `gate_pipeline.rs:36-96` | `GatePipeline` struct + builders | B.06 |
| `gate_pipeline.rs:115-126` | merged `TestCount` logic | B.09 |
| `gate_pipeline.rs:129-142` | per-step detail renderer | B.08 |
| `gate_pipeline.rs:145-180` | `impl Gate for GatePipeline` with short-circuit + skip accounting | B.06-B.08 |

### Adaptive thresholds + feedback

| File | What | Section |
|------|------|---------|
| `adaptive_threshold.rs:11-19` | constants: `EMA_ALPHA`, retry bounds, skip threshold | D.01 |
| `adaptive_threshold.rs:22-40` | `RungStats` + default neutral prior | D.01 |
| `adaptive_threshold.rs:58-76` | `load_or_new(path)` and atomic `save(path)` | D.06 |
| `adaptive_threshold.rs:79-96` | EMA update rule | D.02 |
| `adaptive_threshold.rs:102-119` | `suggested_max_retries(rung)` | D.03 |
| `adaptive_threshold.rs:127-131` | `should_skip_rung(rung)` | D.04 |
| `feedback.rs:13-21` | `Severity` ordering | D.08 |
| `feedback.rs:26-64` | `FeedbackItem` + `GateFeedback` structs | D.08, D.11 |
| `feedback.rs:66-94` | `item_count`, `is_empty`, `items` | D.08 |
| `feedback.rs:99-131` | `classify_line()` priority chain | D.09 |
| `feedback.rs:134-192` | noise / error / warning / suggestion helpers | D.09 |
| `feedback.rs:196-237` | `feedback_for_agent(gate_output, rung)` | D.10 |

### Artifacts + ratchet

| File | What | Section |
|------|------|---------|
| `artifact_store.rs:21-23` | in-memory `ArtifactStore` shape | C.01 |
| `artifact_store.rs:38-42` | `store()` with content-addressed dedup | C.02-C.03 |
| `artifact_store.rs:46-66` | `retrieve`, `exists`, `len`, `is_empty` | C.02 |
| `ratchet.rs:16-19` | `GateRatchet { passes: HashMap<String, u8> }` | C.07 |
| `ratchet.rs:34-40` | `record_pass(plan_id, rung)` | C.07, C.08 |
| `ratchet.rs:45-47` | `highest_pass(plan_id)` | C.07 |
| `ratchet.rs:57-62` | `can_regress(plan_id, rung)` | C.07, C.08 |
| `ratchet.rs:78-205` | ratchet test suite | C.07 |

### Payload + build-system helpers

| File | What | Section |
|------|------|---------|
| `payload.rs:86-189` | `BuildSystem` command / args dispatch helpers | A.06, A.08, D.14 |
| `payload.rs` | `GatePayload` and builder helpers | A.14 |
| `env_builder.rs` | `GateEnv`, `GateEnvBuilder` | A.14 |

---

## crates/roko-cli/src/

### Orchestrator hot spots

| File | What | Section |
|------|------|---------|
| `orchestrate.rs:3292,3411,3534` | `AdaptiveThresholds::load_or_new(...gate-thresholds.json)` | D.05-D.06 |
| `orchestrate.rs:3740-3741` | adaptive-threshold save path | D.05-D.06 |
| `orchestrate.rs:5273` | `ExecutorAction::RunGate` production caller into `run_gate_pipeline` | B.04-B.06 |
| `orchestrate.rs:5329` | `self.adaptive_thresholds.update(rung, passed)` | D.05 |
| `orchestrate.rs:5284-5356` | one narrow consolidation loop: gate -> episode -> enrichment -> skill | E.14, F.07 |
| `orchestrate.rs:8897-9000` | `handle_autofix(plan_id)` raw gate-context retry path | D.03, D.10, E.04 |
| `orchestrate.rs:11144-11272` | `run_gate_pipeline(plan_id, rung)` main verification path | B.04-B.06, G.09 |
| `orchestrate.rs:11175-11185` | derived `Kind::GateVerdict` engram builder with only `gate` / `passed` tags | G.09-G.10 |
| `orchestrate.rs:11246` | conductor-side `Kind::GateVerdict` emission with plan / rung / duration fields | G.09 |
| `orchestrate.rs:11339` | post-merge follow-up calls `run_gate_rung(..., 3)` using the current ad-hoc numeric contract | B.04 |
| `orchestrate.rs:11423-11461` | `run_gate_rung(payload_sig, rung)` hardcoded dispatch | B.04 |

### CLI / TUI / HTTP readers of threshold state

| File | What | Section |
|------|------|---------|
| `main.rs:5439-5441` | CLI status reads `suggested_max_retries` and `should_skip_rung` | D.03-D.04 |
| `tui/dashboard.rs:2708,3991` | dashboard reads `suggested_max_retries` | D.03 |
| `tui/dashboard.rs:3687,4015` | dashboard reads `should_skip_rung` | D.04 |
| `tui/dashboard.rs:47` | `GATE_THRESHOLDS_FILE` constant | D.06 |

---

## crates/roko-serve/src/

| File | What | Section |
|------|------|---------|
| `routes/learning.rs:82-95` | HTTP load path for adaptive thresholds | D.06 |
| `routes/learning.rs:237-238` | HTTP exposure of retry / skip advisories | D.03-D.04 |
| `routes/learning.rs:1253` | `gate-thresholds.json` path construction | D.06 |

---

## crates/roko-learn/src/

### Downstream verdict / skill consumers

| File | What | Section |
|------|------|---------|
| `episode_logger.rs:90-100` | learning-layer `GateVerdict` struct | F.06, G.02 |
| `episode_logger.rs:169` | `Episode` struct | F.06, G.02 |
| `runtime_feedback.rs:123,127` | episodes and playbook file paths | F.06, G.02 |
| `runtime_feedback.rs:325-462` | runtime feedback state holding logger / skill library / playbook rules | F.06 |
| `runtime_feedback.rs:829-831` | playbook upsert + save path | F.06 |
| `skill_library.rs:404-803` | `Skill` + `SkillLibrary` core type surface | F.07, F.09 |
| `skill_library.rs:1246` | `extract_skill(request)` | F.07 |
| `efficiency.rs` | `EfficiencyEvent` persistence | E.13, G.04 |
| `drift.rs` | efficiency-event consumer for drift detection | G.04 |

---

## Missing / Absent (code-search negatives)

These doc features have no matching production code in `crates/`:

### Reward-model and lifecycle design

| Absent Feature | Search | Section |
|----------------|--------|---------|
| `PromiseScore`, `promise_score` | `rg -n "PromiseScore|promise_score" crates/` | E.01 |
| `ProgressScore`, `progress_score` | `rg -n "ProgressScore|progress_score" crates/` | E.02 |
| `ProcessReward`, `ProcessRewardModel` | `rg -n "ProcessReward|ProcessRewardModel" crates/` | E.01-E.02 |
| `GatePotential`, `StepRewardComputer` | `rg -n "GatePotential|StepRewardComputer" crates/` | E.08 |
| `MonteCarloStepLabeler`, `StepLabel`, `StepQuality` | `rg -n "MonteCarloStepLabeler|StepLabel|StepQuality" crates/` | E.05 |
| `ThinkPrm`, `FormalStepLabeler`, `FormalVerifier` | `rg -n "ThinkPrm|FormalStepLabeler|FormalVerifier" crates/` | E.06 |
| `DpoTrainingPair`, `RlaifConfig`, `SAFETY_CONSTITUTION` | `rg -n "DpoTrainingPair|RlaifConfig|SAFETY_CONSTITUTION" crates/` | E.07 |
| `Gauntlet`, `gauntlet` | `rg -n "Gauntlet|gauntlet" crates/` | E.11 |

### Autonomous eval + advanced EvoSkills

| Absent Feature | Search | Section |
|----------------|--------|---------|
| `AgentRole::TestGenerator`, `TestGeneratorAgent` | `rg -n "TestGeneratorAgent|TestGenerator|AgentRole::TestGenerator" crates/` | F.01-F.02 |
| `surrogate`, `adversarial_verification` | `rg -n "surrogate|adversarial_verification|cross_model_factor" crates/` | F.08 |
| `SkillGenome`, `BehavioralDescriptor`, `RetryGenome` | `rg -n "SkillGenome|BehavioralDescriptor|RetryGenome" crates/` | F.13 |
| `SkillArchive`, `MapElites`, `qd_score` | `rg -n "SkillArchive|MapElites|qd_score" crates/` | F.14 |
| `LandscapeAnalysis`, `ruggedness`, `evolvability` | `rg -n "LandscapeAnalysis|ruggedness|evolvability" crates/` | F.16 |
| `SpeciesManager`, `CompatibilityMetric` | `rg -n "SpeciesManager|CompatibilityMetric" crates/` | F.17 |
| `AuroraDescriptor`, `TraceEncoder`, `SkillCmaEs` | `rg -n "AuroraDescriptor|TraceEncoder|SkillCmaEs" crates/` | F.18 |

### Replay / verdict analytics

| Absent Feature | Search | Section |
|----------------|--------|---------|
| `replay_task`, `reconstruct_chain`, `replay_algorithm` | `rg -n "replay_task|reconstruct_chain|replay_algorithm" crates/` | G.05 |
| `RootCauseAnalysis`, `GapAnalysis`, `what_if` | `rg -n "RootCauseAnalysis|GapAnalysis|what_if" crates/` | G.06 |
| `VerdictTimeSeries`, `VerdictTrend` | `rg -n "VerdictTimeSeries|VerdictTrend|classify_trend" crates/` | G.12 |
| `CoFailureDetector`, `SignatureCluster` | `rg -n "CoFailureDetector|SignatureCluster|CoFailurePair" crates/` | G.13 |
| `ReplanEngine`, `PredictiveGateSelector`, `VerdictPatternMemory` | `rg -n "ReplanEngine|PredictiveGateSelector|VerdictPatternMemory|FailurePredictor" crates/` | G.14 |

---

## Runtime Negatives That Matter For Batch 04

These are especially important because the code exists, but runtime does not use it:

| Runtime-negative | Evidence | Section |
|------------------|----------|---------|
| `select_rungs(...)` has no orchestrator caller | only `rung_selector.rs` tests reference it | B.05 |
| `GatePipeline` has no production caller | only `gate_pipeline.rs` tests reference it | B.06 |
| `feedback_for_agent(...)` has no production caller | only `feedback.rs` and `lib.rs` reference it | D.10 |
| `GateRatchet` has no production caller | only `ratchet.rs` and `lib.rs` reference it | C.08 |
| `.roko/artifacts` persistence path does not exist | no workspace references | C.05 |

---

## File Size Reality Check

The important runtime gaps here are mostly **integration gaps, not missing-library gaps**:

- `symbol_gate.rs` is ~1000 LOC,
- `generated_test_gate.rs` is ~800 LOC,
- `integration_gate.rs` is ~800 LOC,
- `property_test_gate.rs` is ~700 LOC,
- `gate_pipeline.rs` is ~600 LOC,
- `feedback.rs` is ~370 LOC,
- `ratchet.rs` is ~200 LOC,
- yet the orchestrator still behaves like a much smaller verification system.

That is the main reason batch `04` should default to runtime activation and contract cleanup rather than new verification theory.
