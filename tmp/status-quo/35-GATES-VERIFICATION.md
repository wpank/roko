# roko-gate — Gates, Rungs, Verification Pipeline

> Status-quo audit · verified 2026-07-08 (re-verified against HEAD 5852c93c05) · sources: 42 src files + 5 test files in `crates/roko-gate/` (~550 test fns), orchestrate.rs gate regions read directly, rung_dispatch/gate_service/service_factory/state_machine/run.rs call sites, `.roko/` data files, docs/v1/04-verification (15 docs) + docs/v2-depth/02-block (7 verify docs)
>
> **Re-verification note (2026-07-08):** Every file:line claim below was spot-checked against current HEAD and holds, with one drift: `task_tier_to_plan_complexity` is at **orchestrate.rs:3197** (was cited 18126); the complexity→rung path is unchanged. Confirmed live: 42 gate src files, no `gates/` subdir, `Rung` enum 0–6 (rung_selector.rs:96-117), `CANONICAL_ORDER` len 7 (:120-128), `select_rungs(complexity,caps,prior_failures)` (:267-274), `escalate_by` (:49), `enable_advanced_rungs` dead-toggle both branches `skipped_count += 1` (18259-18270), `gate_results.len()` scheduling (state_machine.rs:245-247), 467 `GateVerdict` in `.roko/signals.jsonl`, `gate-thresholds.json` still **ABSENT on disk**, save-at-teardown (orchestrate.rs:5947-5953), GateService dual-dialect (diff=3/fmt=4/shell=5/judge=6, gate_service.rs:51-59), `set_verdict_publisher` **no real callers**, `stub_verdict → Verdict::pass` (rung_dispatch.rs:290-292), serve gates route reads `engrams.jsonl`+`events.jsonl` not `signals.jsonl` (routes/status/gates.rs:84-92 — open Q1 confirmed).
>
> **tmp-feedback/2/23 drift:** The feedback doc "23-GATE-RUNGS-3-6-NEVER-SELECTED.md" correctly identifies the *symptom* (rungs 3–6 don't execute inside the rung-0 pipeline) but its root-cause code is **fabricated**: it invents a `select_rung(task, config)` keyed on `task.priority` (Critical/High/Normal) — no such function exists; real selection is complexity-band-based (`select_rungs`, above). It also cites `crates/roko-gate/src/gates/verify_chain.rs` and a `Gate`/`GateResult`/`Artifact` API that do not exist (the file is `verify_chain_gate.rs`, the trait is `Cell`, the return type is `Verdict`). Its proposed "Fix 1" would introduce a priority→rung mapping that contradicts the actual architecture. **Treat 23 as a lead, not a spec.** The real fix is the `enable_advanced_rungs` dead-toggle at orchestrate.rs:18259-18270 (see checklist P1).

## Summary

roko-gate is one of the **most genuinely wired** subsystems. The canonical pipeline is **7 rungs** (the v1 doc *filename* `02-6-rung-selector.md` is a vestige — its content is titled "7-Rung Gate Selector" and lists rungs 0–6; CLAUDE.md's "7-rung" is correct). Rung selection is a pure function of 4-tier `PlanComplexity` × capability caps × prior-failure escalation (`rung_selector.rs:267`). The executor's Gating phase schedules rungs progressively (`rung = gate_results.len()`, `roko-orchestrator/src/executor/state_machine.rs:245-247`); rung 0 runs a complexity-selected `GatePipeline`, rungs 1–6 dispatch individually via `run_rung`.

**Verdicts ARE Signals** (v2-aligned): every gate verdict is derived from the payload engram as `Kind::GateVerdict` with lineage, `gate/passed/rung/artifact_hash` tags, `Decay::GATE_VERDICT`, optional attestation, and a chain-verification check before `substrate.put()` (orchestrate.rs:17702-17747, 18401-18428). **467 GateVerdict signals exist in `.roko/signals.jsonl`** — this is exercised, not aspirational; the v2 doc's claim that verdicts are "not emitted as first-class Signals" (verdicts-as-signals.md:485) is **stale**. Adaptive EMA thresholds, CUSUM/EWMA/BOCPD SPC, Hotelling T² joint anomaly, ratcheting, content-addressed artifacts, structured failure feedback, and the gate-failure→replan loop are all wired in orchestrate.rs. However: `.roko/learn/gate-thresholds.json` **does not exist on disk** despite 467 recorded verdicts (save happens only at graceful run teardown, orchestrate.rs:5947-5959), the Pulse-based `VerdictPublisher` is never attached (no caller of `set_verdict_publisher`), advanced rungs (Symbol/PropertyTest/Integration) are skipped inside the rung-0 pipeline regardless of config, missing-input rungs emit **passing** stub verdicts, and PRM / eval generation / forensic replay / PELT / evoskills are built-not-wired or absent. The roko-graph cognitive-loop `gate-pipeline` cell is still a `PassthroughCell` stub even though `GatePipeline` itself implements `roko_core::Cell`.

## Current state table

| Component | Design source | Code | Status | Evidence |
|---|---|---|---|---|
| Gate trait (`Verify` → `Verdict`) | v1 00-gate-trait | `roko-core/src/verdict.rs:51-70`; 18 `impl roko_core::Cell` in roko-gate | ✅ wired | Verdict-total-function invariant honored; gates return Verdict, never Result |
| 7-rung selector | v1 02 (filename "6-rung", content 7-rung) | `rung_selector.rs:96-128, 234-274` | ✅ wired | `select_rungs(complexity, caps, prior_failures)` w/ escalation ladder; called at orchestrate.rs:18206 |
| Gate pipeline (sequential, short-circuit) | v1 03 / v2 verify-cells-and-pipeline | `gate_pipeline.rs:208-292`; run at orchestrate.rs:18348-18367 | ✅ wired | Short-circuit + skip transcript + aggregate test counts; is a Cell (`cell_id "gate-pipeline"`, gate_pipeline.rs:195-205) |
| Rung dispatch (rungs 1–6 oracles) | v2 verify-cells | `rung_dispatch.rs:219-238` (`run_rung`), orchestrate.rs:18527-18635 (`run_gate_rung`) | ✅ wired | Executor emits `RunGate{rung=gate_results.len()}` (state_machine.rs:245-247) |
| Rung oracles (Symbol 3, GenTest 4, FactCheck 5, Judge 6) | CLAUDE.md "oracles 4-6" | `gate_rung_config` orchestrate.rs:18430-18487; `enrich_rung_config` 18492-18525 | ✅ wired* | SymbolManifest from task ctx (18548-18573), Perplexity oracle iff `PERPLEXITY_API_KEY` (18463-18466), `AgentJudgeOracle` (18468-18485); *stub-pass when inputs absent |
| Adaptive EMA thresholds | v1 06 / v2 ratcheting-and-adaptive | `adaptive_threshold.rs:322-373` (`observe`, α=0.1) | 🟡 partial | Loaded ×3 (orchestrate.rs:4676-4680 etc.), observed per verdict (17789-17790), saved at teardown (5953) — **but `.roko/learn/gate-thresholds.json` absent on disk** despite 467 verdict signals |
| SPC (CUSUM/EWMA/BOCPD) + Hotelling T² | GATE-01/GATE-08 | `spc.rs`, `hotelling.rs`; fed via `observe`/`observe_pipeline` | ✅ wired | orchestrate.rs:17793-17806 drains alerts → `tracing::warn` only (no reactive policy) |
| Ratcheting | v1 05 | `ratchet.rs:21-…`; orchestrate.rs:17663-17678 | ✅ wired | `can_regress`/`record_pass` per plan; persisted via `gate_ratchet_path` (5960) |
| Artifact store (BLAKE3 content-addressed) | v1 04 / v2 process-reward-and-artifacts | `artifact_store.rs:27-80`; orchestrate.rs:18369-18399 | ✅ wired | `persist_gate_artifact` → `artifact_hash` tag on verdict signal (17717-17718) |
| Verdicts-as-Signals | v1 15 / v2 verdicts-as-signals | orchestrate.rs:17702-17747 + `verify_gate_signal_chain` 18401-18428 | ✅ wired | `Kind::GateVerdict`, lineage, `Decay::GATE_VERDICT`, attestation; **467 entries in `.roko/signals.jsonl`** |
| Verdict Pulse reentry (bus) | GATE-05 | `verdict_publisher.rs:58-101`; attach point orchestrate.rs:18457 | 🔌 built-not-wired | `set_verdict_publisher` (orchestrate.rs:5353) has **no callers** → field always `None` |
| Gate feedback → agent retry | v1 08 / v2 gate-feedback-and-retry | `feedback.rs` (`feedback_for_agent`); orchestrate.rs:14347 | ✅ wired | Output truncated to 8K, rung-specific structured feedback into retry prompt |
| Failure classification + error patterns | v2 gate-feedback | `compile_errors.rs` (`classify_gate_failure` @ orchestrate.rs:13620); `error_patterns.rs` (@17904) | ✅ wired | Feeds anti-knowledge entries (8920-8985) and failure-pattern store |
| Gate-failure replan | CLAUDE.md #11 | orchestrate.rs:5543-5576 (`build_gate_failure_plan_revision`), 5657-5759, 10282-10286 | ✅ wired | Fail → retry budget from EMA (`suggested_max_retries`) → `RokoEvent::PlanRevision` → `handle_runtime_event` (5600-…) → regenerate; caps: `replan_gate_attempts`=3, `replan_max_per_plan`=2 (21490-21491) |
| GateService (workflow-engine runner) | — | `gate_service.rs:26-…`; `impl GateRunner` @234 | ✅ wired | Instantiated in `service_factory.rs:234`; consumed by `roko run` (roko-cli/src/run.rs:52), roko-serve (state.rs:951, shared_runs.rs:471), roko-acp (runner.rs:458) |
| Process Reward Model (promise/progress) | v1 07 / v2 process-reward-and-artifacts | `process_reward.rs:51-…` | 🔌 built-not-wired | Zero callers outside roko-gate |
| Eval generation (3 strategies) | v1 10 / v2 eval-lifecycle-and-generation | `eval_generator.rs` | 🔌 built-not-wired | Zero callers outside roko-gate |
| Forensic replay | v1 12 | `forensic.rs` (`ForensicReplayBuilder`) | 🔌 built-not-wired | Zero external callers; **duplicate** impl in `roko-core/src/forensic.rs:143` |
| PELT change-point | P1-13 | `pelt.rs:94-…` | 🔌 built-not-wired | Only self-tests |
| Evoskills | v1 11-evoskills | — | ❌ missing | No `evoskill` symbol anywhere in `crates/` |
| Verify as roko-graph Cell | v2 verify-cells | `roko-graph/src/cells/stubs.rs:14-77` | ❌ missing (stub) | `"gate-pipeline"` in `COGNITIVE_LOOP_STUBS` (stubs.rs:74); GAPS.md:16 |
| HTTP exposure | — | roko-serve `routes/status/gates.rs:22,44,61`; `routes/learning/mod.rs:47-48,102-120` | ✅ wired | `/gates/summary`, `/gates/history`, `/gates/{name}/history`, `/learning/gate-thresholds`, `/learn/adaptive-thresholds` |
| CLI tuning | — | `roko learn tune gates` (`commands/learn.rs:58-64`) | ✅ wired | Displays/adjusts adaptive thresholds |

## Gate census (every gate)

Rung-dispatched tier — 12 concrete gates across 7 rungs (lib.rs:12-25, rung_selector.rs:96-117):

| # | Gate | File | Checks | Rung | Wired where | Status |
|---|---|---|---|---|---|---|
| 1 | CompileGate | `compile.rs` (impl Cell @71) | build/type-check per BuildSystem | 0 | pipeline (orchestrate.rs:18228), `run_rung`, GateService, run.rs:47 | ✅ |
| 2 | ClippyGate | `clippy_gate.rs:69` | lint | 1 | pipeline 18232, GateService, run.rs | ✅ |
| 3 | TestGate | `test_gate.rs:102` | existing test suite + `parse_test_counts` | 2 | pipeline 18242, GateService, run.rs | ✅ |
| 4 | SymbolGate | `symbol_gate.rs:201` | expected symbols exist (SymbolManifest) | 3 | `run_gate_rung` only; manifest from task `context.symbols` (18548-18573); `source_roots` @18459 | 🟡 stub-pass w/o manifest |
| 5 | GeneratedTestGate | `generated_test_gate.rs:246` | agent-generated behavioural tests from `ArtifactStore` trait | 4 | pipeline iff `FsGeneratedArtifactStore` present (18244-18253); enrich @18499-18506 | 🟡 |
| 6 | VerifyChainGate | `verify_chain_gate.rs:176` | verify-script chain (`VERIFY_SCRIPT_TAG`) | 4 | `run_canonical_rung` companion | 🟡 |
| 7 | PropertyTestGate | `property_test_gate.rs:174` | property-based tests | 5 | `run_gate_rung` only | 🟡 |
| 8 | FactCheckGate | `fact_check.rs:160` | acceptance-criteria claims vs `SearchOracle` | 5 | oracle = Perplexity iff env key (18463-18466); claims from `task.acceptance` (18576-18582) | 🟡 |
| 9 | LlmJudgeGate | `llm_judge_gate.rs:195` | LLM judges task-description × git diff | 6 | `AgentJudgeOracle` (18468-18485); JudgePayload (18584-18607); adaptive `llm_judge_min_score` (18450-18454) | ✅ |
| 10 | IntegrationGate | `integration_gate.rs:249` | integration scenario command | 6 | `integration_test_pattern` from task verify phase=integration (18507-18523) | 🟡 |

Standalone tier — invoked outside rungs (lib.rs:27-36):

| # | Gate | File | Checks | Status |
|---|---|---|---|---|
| 11 | ShellGate | `shell.rs:58` | arbitrary command | ✅ wired — domain gates + task verify steps (orchestrate.rs:18296-18345), GateService shell/diff, run.rs |
| 12 | DiffGate | `diff_gate.rs:119` | diff analysis (`analyze_diff`) | 🔌 no callers outside crate |
| 13 | CodeExecutionGate | `code_exec.rs` | sandboxed code execution | 🔌 |
| 14 | BenchmarkRegressionGate | `benchmark_gate.rs:60` | perf regression | 🔌 |
| 15 | FormatCheckGate | `format_check_gate.rs:28` | `cargo fmt --check` | 🟡 only via GateService "fmt" mapping (gate_service.rs:74; **second impl** @ gate_service.rs:168) |
| 16 | SecurityScanGate | `security_scan_gate.rs:27` | security scan | 🔌 |
| 17 | GateGenerator / GeneratedCheck | `generated.rs` | ad-hoc generated checks | 🔌 |
| 18 | StubJudgeGate | `gate_service.rs:199` | placeholder judge in GateService | 🕰️ stub in wired path |

Composition wrappers: `ParallelGate` / `VotingGate` / `FallbackGate` (`composition.rs`, GATE-04) — 🔌 no external callers; `ComposedGatePipeline` + `GateComposition::{Sequential,Parallel,Voting,Fallback}` (gate_pipeline.rs:302-…) used by `GatePipelineBuilder` (rung_dispatch.rs:88-164) with Sequential only at runtime.

Support modules: `payload.rs` (GatePayload/BuildSystem, ✅), `env_builder.rs` (per-rung env, 🔌 internal-only), `registry.rs` (GateRegistry alias/rung metadata, 🔌 internal-only), `acceptance_contract.rs` (✅ task_parser.rs:15,97,160 + plan_validate.rs:9,411 + evidence types in orchestrate), `review_verdict.rs` (✅ orchestrate.rs:14613, 20768, 22492), `gate_service.rs` (✅ via ServiceFactory).

## Rung pipeline reality

- **7 rungs is real**: `Rung` enum 0–6 (`rung_selector.rs:96-117`), `CANONICAL_ORDER` len 7 (:120-128). The "6-rung" name survives only in the v1 doc filename `docs/v1/04-verification/02-6-rung-selector.md`, whose content says 7. **Resolution: 7.**
- **Selection**: `select_rungs(complexity, caps, prior_failures)` — Trivial→[Compile], Simple→+Lint, Standard→+Test+Symbol, Complex→all 7 (:234-249); caps only narrow (:210-219); each prior failure escalates one tier (:267-274). Complexity comes from task tier (orchestrate.rs:3197 `task_tier_to_plan_complexity`; mechanical/fast→Trivial … architectural/complex/premium→Complex). Caps detected from filesystem (symbols.json, proptest-regressions, tests/integration — orchestrate.rs:18153-18170).
- **Scheduling**: executor Gating phase emits `RunGate { rung: plan_state.gate_results.len() }` (state_machine.rs:245-247) → orchestrate handles at 8688-8691 → `run_gate_pipeline` (17620): rung 0 → complexity-selected `GatePipeline` of `RecordingGate`-wrapped steps (18348-18367); rung 1–6 → `run_gate_rung` (18527-18635); rung >6 → all rungs sequentially (rung_dispatch.rs:226-232).
- **Inside the rung-0 pipeline, Symbol/PropertyTest/Integration never execute** — both branches of the `enable_advanced_rungs` check just increment `skipped_count` (orchestrate.rs:18255-18270); the config flag (default `false`, roko-core/src/config/gates.rs:51,73) is a dead toggle at pipeline level. They only run via later `RunGate` actions.
- **Stub verdicts pass**: when a rich rung lacks inputs, `stub_verdict` returns `Verdict::pass` labeled "stub gate" (rung_dispatch.rs:290-296) — honest in text, but counts as a pass.
- **Per-rung skip**: EMA streak ≥20 consecutive passes advises skip; Compile/Test never skipped (orchestrate.rs:18172-18175, adaptive_threshold.rs:409-413); temperament variants exist (adaptive_threshold.rs:566-612).
- **Second, incompatible rung scheme**: GateService maps names→rungs as compile=0, clippy=1, test=2, **diff=3, fmt=4, shell=5, judge=6** (gate_service.rs:51-59, asserted in tests :392-398) — the `roko run`/workflow-engine path speaks a different 7-rung dialect than the canonical enum.

### "Rungs 3–6 never selected" — reconciliation (tmp-feedback/2/23)

The feedback claim is **half-true and must be re-scoped**:

- **Can rungs 3–6 be selected?** *Yes, in principle.* `select_rungs` returns Symbol(3) for `Standard` complexity and GeneratedTest/PropertyTest/Integration(4-6) for `Complex` (base_rungs, rung_selector.rs:234-249). A Complex-tier task (tier `architectural`/`complex`/`premium` → `task_tier_to_plan_complexity`, orchestrate.rs:3197) with the right caps selects all 7. The escalation ladder (`escalate_by`, rung_selector.rs:49) also lifts a failing plan toward Complex.
- **Do they actually execute?** *Two paths, both attenuated:*
  1. **Inside the rung-0 pipeline** (`selected_gate_steps`): Symbol/PropertyTest/Integration are **hard-skipped regardless of `enable_advanced_rungs`** — both branches only `skipped_count += 1` (orchestrate.rs:18255-18270). This is the real dead-toggle bug. GeneratedTest runs *only if* an `FsGeneratedArtifactStore` is present (18244-18253), which nothing populates today.
  2. **Via later `RunGate{rung=N}` actions** dispatched by the executor as `gate_results.len()` advances (state_machine.rs:245-247 → `run_gate_rung`, orchestrate.rs:18527): here rungs 3/5 **do** build real inputs (SymbolManifest from `task.context.symbols` at 18548-18573; fact-check from `task.acceptance` at 18576+). But when those inputs are absent the rung returns `stub_verdict → Verdict::pass` (rung_dispatch.rs:290-292), so in practice most real plans (no symbol manifest, no proptest regressions, no integration scenario) get **passing stubs** for rungs 3–6, which is observationally close to "never really verified."
- **Verdict:** the *symptom* ("critical tasks get the same weak validation as doc updates") is real for the pipeline path and for input-less plans; the feedback doc's *code* and *priority-based fix* are fabricated (no `select_rung`/`task.priority`; the file is `verify_chain_gate.rs` not `gates/verify_chain.rs`). Correct fixes: (a) make `enable_advanced_rungs` actually push Symbol/PropertyTest/Integration steps (P1), (b) make `stub_verdict` neutral/inconclusive and exclude from EMA (P1), (c) populate a generated-test store so rung 4 has artifacts (P2). `VerifyChainGate` stub-pass is genuinely Phase-2-blocked (chain runtime), correctly deferred.

## V2-aligned

- **Verdict as total function** — every gate returns `Verdict`, never `Result`; timeouts become fail verdicts (v2 verify-cells invariant 2 holds).
- **Verdict→Signal with lineage/decay/tags** — `payload_sig.derive_verdict(...)` + `Kind::GateVerdict` + `Decay::GATE_VERDICT` + lineage + attestation + tamper check (`verify_gate_signal_chain`, orchestrate.rs:18401-18428) → `substrate.put` (17722). Evidence: 467 GateVerdict lines in `.roko/signals.jsonl`. The v2 doc's "not yet wired" note (verdicts-as-signals.md:485) is stale.
- **Gates are Cells** — 18 `impl roko_core::Cell` across gates; `GatePipeline` is itself a Cell exposing protocol `["Verify"]` (gate_pipeline.rs:195-205), enabling fractal composition; proven by `tests/cell_execute.rs` (execute → Verdict → GateVerdict Signal).
- **Ratcheting + adaptive thresholds + SPC** — matches v2 ratcheting-and-adaptive-thresholds: EMA α=0.1, CUSUM k=0.25/h=4.0, SPC ensemble, Hotelling T², neuro hints (`apply_neuro_hints`, INT-15, called via `apply_neuro_gate_hints` orchestrate.rs:8580), residual tightening (TA-15, adaptive_threshold.rs:505-520), role floors (`override_for_role` used at 18430-18445).
- **Verdict-driven replanning** — gate failures produce structured classifications, failure-pattern records, anti-knowledge entries, and `PlanRevision` events consumed in-process (v2 gate-feedback-and-retry shape).
- **Content-addressed evidence** — artifact hash on each verdict signal links substrate to `ArtifactStore` (forensic chain raw material exists even though the replayer isn't wired).

## Old paradigm & tech debt

- 🕰️ **Dual rung dialects**: canonical `Rung` enum vs GateService's name→rung map (diff/fmt/shell/judge at 3–6, gate_service.rs:51-59; asserted at :392-398). Same word, two meanings; thresholds keyed by rung index would mix populations if GateService attached the same `AdaptiveThresholds` file. **The GateService dialect leaks outside roko-gate**: `roko-runtime/src/effect_driver.rs` calls `GateService::rung_for_name(name)` to assign rungs to effect-driven gates — so the *runtime effect path* also speaks the non-canonical dialect, not just `roko run`. Unifying the vocabulary must cover this call site too.
- 🕰️ **Duplicate implementations** (the classic roko disease): complexity mapping in orchestrate.rs:18126 **and** runner/event_loop.rs:3855-3861 (Runner v2); forensic in roko-gate `forensic.rs` **and** roko-core `forensic.rs:143-…`; `gate_artifact_store_path`/`gate_ratchet_path` in gate_runner.rs:20,25 **and** config_helpers.rs:61,65; FormatCheckGate in format_check_gate.rs:28 **and** gate_service.rs:168.
- 🕰️ **`GateRunner`/`GateReport` foundation types** (roko-core::foundation) are a bespoke pre-Signal verdict shape used by the workflow-engine path; the orchestrate path uses real Verdict→Signal. Two verification result vocabularies coexist.
- 🕰️ Doc drift: v1 filename `02-6-rung-selector.md` (content says 7); v2 verdicts-as-signals.md:485 says signals not wired (they are).
- Stub-pass verdicts for unwired rungs (rung_dispatch.rs:290-296) and `StubJudgeGate` in GateService inflate pass rates fed to the EMA.
- SPC/Hotelling alerts terminate in `tracing::warn!` (orchestrate.rs:17804-17806) — detection without reaction.
- `.roko/learn/gate-thresholds.json.tmp.*` style orphan tmp files pattern (seen for cascade-router in `.roko/learn/`) suggests save-at-teardown is fragile; thresholds file never made it to disk at all.

## Not implemented

- ❌ roko-graph `gate-pipeline` cognitive-loop cell — `PassthroughCell` stub (`roko-graph/src/cells/stubs.rs:14-77`, listed at :74; GAPS.md:16).
- ❌ Evoskills (v1 doc 11) — no code anywhere.
- 🔌 ProcessRewardModel (promise/progress per turn) — zero runtime callers.
- 🔌 EvalGenerator (example/property/mutation strategies) — zero runtime callers; no eval lifecycle CLI (only a benchmark-eval flag in main.rs:596).
- 🔌 Forensic replay builder — zero callers (and duplicated in roko-core).
- 🔌 PELT offline change-point detection — zero callers.
- 🔌 VerdictPublisher Pulse reentry — attach point exists (orchestrate.rs:18457) but `set_verdict_publisher` (5353) is never called.
- 🔌 DiffGate, CodeExecutionGate, BenchmarkRegressionGate, SecurityScanGate, GateGenerator, ParallelGate/VotingGate/FallbackGate — exported, tested, unused.
- ❌ Predictive gate selection from verdict history (v2 verdicts-as-signals bullet 5) — selection is static complexity bands only.
- ❌ Verify-as-universal-oracle beyond code — FactCheckGate is the sole non-code oracle; no doc/research/data verification cells.

## Drift ledger (fast reference)

| Drift | Kind | Evidence | Severity |
|---|---|---|---|
| `gate-thresholds.json` never on disk despite 467 verdicts | Persistence gap | file ABSENT; save only at teardown (orchestrate.rs:5947-5953) | **P0** |
| roko-graph `gate-pipeline` cell is `PassthroughCell` stub | Not wired | roko-graph/src/cells/stubs.rs:74 | **P0** |
| `enable_advanced_rungs` dead toggle (both branches skip) | Bug | orchestrate.rs:18259-18270 | **P1** |
| Stub verdicts count as `Verdict::pass` in EMA | Correctness | rung_dispatch.rs:290-292 | **P1** |
| GateService dual rung dialect (diff/fmt/shell/judge=3-6), leaks to roko-runtime | Duplication | gate_service.rs:51-59 vs Rung enum; `effect_driver.rs` calls `rung_for_name` | **P1** |
| `VerdictPublisher` attach point but no `set_verdict_publisher` caller | Not wired | orchestrate.rs:5353 (0 callers) | **P1** |
| serve `/gates/*` reads engrams.jsonl+events.jsonl, verdicts in signals.jsonl | Substrate split | routes/status/gates.rs:84-92 | **P1** |
| `StubJudgeGate` in wired GateService path | Stub-in-prod | gate_service.rs:80,199 | **P2** |
| Forensic dup (roko-gate vs roko-core), path-helper dup, complexity-map dup | Duplication | see "Old paradigm" | **P2** |
| SPC/Hotelling alerts warn-only, no reaction | Detection≠action | orchestrate.rs:17804-17806 | **P2** |
| Doc drift: v1 `02-6-rung-selector.md` (content=7); verdicts-as-signals.md:485 stale | Doc | filenames | **P3** |
| tmp-feedback/2/23 fabricated code (`select_rung`/`task.priority`, `gates/verify_chain.rs`) | Stale intel | no such symbols | note-only |

## Migration checklist

- [ ] **[P0]** Implement the real roko-graph `gate-pipeline` cell by delegating to `roko_gate::GatePipeline` (it already implements `Cell`) — verify: `grep -n 'PassthroughCell' crates/roko-graph/src/cells/stubs.rs` no longer lists gate-pipeline; graph loop test executes real gates
- [ ] **[P0]** Fix adaptive-threshold persistence: save incrementally after each `observe` batch (not only run teardown) — verify: run `cargo run -p roko-cli -- plan run plans/` then `cat .roko/learn/gate-thresholds.json` shows non-empty `rungs`
- [ ] **[P1]** Unify the two rung dialects: make GateService use the canonical `Rung` enum or namespace its thresholds — verify: `grep -n 'rung_for_name' crates/roko-gate/src/gate_service.rs` maps to `Rung::from_index`
- [ ] **[P1]** Wire `VerdictPublisher` into PlanRunner startup (call `set_verdict_publisher` with the runtime event bus) — verify: `grep -rn 'set_verdict_publisher' crates/roko-cli/src/ | grep -v 'pub fn'` shows ≥1 caller; `gate.verdict.emitted` pulses observable via `roko serve` SSE
- [ ] **[P1]** Make stub verdicts neutral (skip/inconclusive) instead of `Verdict::pass`, and exclude them from `AdaptiveThresholds::observe` — verify: unit test in `rung_dispatch.rs` asserts stub verdicts don't raise EMA
- [ ] **[P1]** Resolve `enable_advanced_rungs` dead toggle in `selected_gate_steps` (orchestrate.rs:18255-18270): either push Symbol/PropertyTest/Integration steps when enabled or delete the flag — verify: with `[gates] enable_advanced_rungs = true`, `roko plan run` transcript shows symbol/prop-test/integration executed in the pipeline
- [ ] **[P2]** Wire ProcessRewardModel per agent turn (promise/progress → efficiency events / replan triggers) — verify: `grep -rn 'ProcessRewardModel' crates/roko-cli/src/orchestrate.rs`
- [ ] **[P2]** Wire EvalGenerator into task dispatch (generate tests before implementation, store in `FsGeneratedArtifactStore` so rung 4 has real artifacts) — verify: `.roko/` generated-tests dir populated after a plan run
- [ ] **[P2]** Deduplicate forensic (roko-gate vs roko-core), path helpers (gate_runner.rs vs config_helpers.rs), and complexity mapping (orchestrate vs runner/event_loop) — verify: `grep -rn 'fn gate_ratchet_path' crates/roko-cli/src/ | wc -l` == 1
- [ ] **[P2]** React to SPC/Hotelling alerts (escalate complexity, freeze merges, or emit PlanRevision) instead of warn-only — verify: integration test injecting failure streak asserts an executor action
- [ ] **[P2]** Rename `docs/v1/04-verification/02-6-rung-selector.md` → `02-7-rung-selector.md`; fix verdicts-as-signals.md:485 stale claim — verify: `ls docs/v1/04-verification/ | grep 7-rung`
- [ ] **[P3]** Wire or delete: DiffGate, CodeExecutionGate, BenchmarkRegressionGate, SecurityScanGate, GateGenerator, composition wrappers, PELT — verify: each has ≥1 non-test caller or is removed from lib.rs exports
- [ ] **[P3]** Replace `StubJudgeGate` in GateService with the real `LlmJudgeGate`+`AgentJudgeOracle` used by orchestrate — verify: `grep -n 'StubJudgeGate' crates/roko-gate/src/gate_service.rs` gone
- [ ] **[P3]** Predictive gate selection from GateVerdict signal history (v2) — verify: selector consults substrate query before falling back to complexity bands

## Open questions

1. **Substrate file split**: gate verdicts land in `.roko/signals.jsonl` (467 entries) but roko-serve's `/gates/*` handlers read `.roko/engrams.jsonl` + `events.jsonl` (routes/status/gates.rs:84) — engrams.jsonl contains **zero** GateVerdict entries. Are the dashboard gate views silently empty, or is there a migration/dual-write I didn't find?
2. Why did 467 verdict-producing runs (last write 2026-05-08) never produce `gate-thresholds.json`? Crash-before-teardown, different workdir, or save-path regression? The `roko-acp` THRESHOLDS_PATH (runner.rs:1873) and `/learning/gate-thresholds` route both read a file that has never existed.
3. Executor schedules `rung = gate_results.len()` — with multi-verdict rungs (rung 4/5/6 each emit 2 verdicts) does the rung counter skip rungs? `gate_results.push` happens per verdict (orchestrate.rs:17750-17756), so one pipeline run can advance the counter by >1.
4. Is `GateService`'s adaptive-thresholds attachment (`with_adaptive_thresholds`, gate_service.rs:45-48) ever used by ServiceFactory, and if so does it share the same JSON file as orchestrate's canonical-rung EMA (dialect-mixing risk)?
5. Should `FactCheckGate`'s Perplexity dependency degrade to the LLM-judge oracle instead of stub-pass when `PERPLEXITY_API_KEY` is absent?
