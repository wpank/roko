# Gate Pipeline — Deep Second-Pass Execution Trace (Runner v2)

> Status-quo trace · verified 2026-07-08 · HEAD `5852c93c05` · branch `main`
> Sources read directly this pass: `crates/roko-cli/src/runner/event_loop.rs` (6681 ln), `runner/gate_dispatch.rs` (539 ln), `runner/types.rs`, `runner/persist.rs`, `crates/roko-gate/src/{rung_dispatch,rung_selector,registry,adaptive_threshold,lib}.rs`, `crates/roko-cli/src/orchestrate.rs` (gate regions 17600–18720), `crates/roko-runtime/src/effect_driver.rs`, `crates/roko-core/src/{foundation,dashboard_snapshot}.rs`, `crates/roko-learn/src/episode_logger.rs`, `crates/roko-chain/src/identity_economy_identity.rs`, `crates/roko-fs/src/layout.rs`.
>
> **Scope:** This doc supersedes the *runtime-path* portions of `35-GATES-VERIFICATION` and `100-...` for gate execution. Those docs trace `orchestrate.rs::PlanRunner` (`run_gate_rung`/`enrich_rung_config`/`gate_rung_config`) as if it were live. **It is not.** `roko plan run` executes `runner::event_loop::run` (Runner v2), a *different* gate path that never touches `enrich_rung_config`. See §0 correction.

---

## 0. TL;DR — the correction that reframes everything

**There are two complete, divergent gate engines in the tree. Only one runs when you type `roko plan run`.**

| | Legacy engine (docs 35/100 traced this) | **LIVE engine (Runner v2)** |
|---|---|---|
| Entry | `PlanRunner::run` / `run_task_plans` — orchestrate.rs:8365 / 8427 | `runner::event_loop::run` — event_loop.rs |
| Reached from CLI? | **No** — no `commands/*` constructs/runs `PlanRunner` for plan-run; `run.rs:837` says it "Skips the 21K-line PlanRunner orchestration path" | **Yes** — `commands/plan.rs:654` and `commands/do_cmd.rs:616` both call `event_loop::run(...)` |
| Rung config enrichment | `enrich_rung_config` (18492) + `gate_rung_config` (18430) wire Perplexity oracle, `AgentJudgeOracle`, `SymbolManifest`, `source_roots`, integration pattern | **None.** `gate_dispatch::run_gate_once` builds the pipeline with `RungExecutionInputs::default()` (gate_dispatch.rs:104) — no oracles, no manifests |
| Threshold type | `roko_gate::AdaptiveThresholds` (CUSUM+EWMA+BOCPD+Hotelling+role-override) | `runner::persist::GateThresholds` — plain EMA only (persist.rs:186) |
| Threshold persistence | `adaptive_thresholds.save(&thresholds_path)` at teardown (orchestrate.rs:5953) | **`GateThresholds::save` is never called** — thresholds only ride inside the executor snapshot JSON |
| Ratchet / VerdictPublisher / artifact store | wired | **absent from the gate path** |

Consequence: every "oracle 4–6 wired", "SPC drains alerts", "ratcheting", "VerdictPublisher" claim in docs 35/100 describes **dead code** relative to `roko plan run`. The rungs 3–6 that *do* fire in the live path hit `stub_verdict → Verdict::pass` unconditionally (rung_dispatch.rs:290) because their inputs are `default()`.

---

## 1. The live hop-by-hop trace (`roko plan run plans/`)

### 1.1 Command → runner
- `commands/plan.rs:654` → `roko_cli::runner::event_loop::run(plans, &run_config, &state_hub, cancel)`.
- `RunConfig::max_gate_rung` is set at construction: `if gates.skip_tests { clippy_enabled as u32 } else { 2 }` — types.rs:1427-1431; `Default` = **2** (types.rs:1482). **Default ceiling = rung 2 (Test).**

### 1.2 Runner setup (event_loop.rs `run`)
- `gate_thresholds = persist::load_gate_thresholds(&paths).unwrap_or_default()` — event_loop.rs:281. Reads `.roko/learn/gate-thresholds.json` **once, read-only**.
- Prefers thresholds embedded in the unified executor snapshot if present — :306-372.
- `gate_sem = Semaphore::new(config.gate_concurrency)` — :473. Bounds concurrent rungs.
- `gate_buffer = (max_concurrent_tasks * 7).clamp(32,256)`; `(gate_tx, gate_rx) = mpsc::channel` — :503-504. The `*7` is the only place "7 rungs" is honored as a sizing constant.

### 1.3 Executor emits `ExecutorAction::RunGate { plan_id, rung }`
Handled in `dispatch_action` — event_loop.rs:4686. Key facts:
- `pipeline_rung = ctx.config.max_gate_rung` — :4689. **The executor's per-rung `rung` field is discarded; the runner always uses `max_gate_rung`.**
- Guard: if `!has_custom_rungs() && !Cargo.toml present` → `record_skipped_gate_rung`, return Handled — :4691-4703. (Non-Rust workspaces get no default gates.)
- Read-only roles (`researcher|strategist|quick-reviewer`, event_loop.rs:3865-3871) → synthesize an **auto-pass** `GateCompletion{passed:true}` and skip gates entirely — :4747-4787.
- Otherwise: `complexity = gate_plan_complexity_for_task(task_def)` — :4790, then `gate_dispatch::spawn_gate(plan_id, task_id, pipeline_rung, workdir, gates_config, complexity, verify_steps, timeout, gate_tx, gate_sem, target_crates)` — :4792.

`gate_plan_complexity_for_task` (event_loop.rs:3855) maps **task tier → PlanComplexity**:

| task `tier` | PlanComplexity | (default when absent) |
|---|---|---|
| `mechanical` / `fast` | Trivial | |
| `focused` | Simple | ← **default** (`unwrap_or("focused")`) |
| `integrative` | Standard | |
| `architectural` / `complex` / `premium` | Complex | |
| anything else | Simple | |

**This is the real rung selector at runtime** — not `max_gate_rung`, not `select_rungs(prior_failures)`. The rungs that execute are a pure function of the task's `tier` string.

### 1.4 `spawn_gate` → `run_gate_once` (gate_dispatch.rs:28 / 77)
- Acquires a semaphore permit (:43), then `run_gate_once`.
- Builds a `GatePayload` signal (`gate_signal`, :278) tagged `plan_id/task_id/rung`, env `ROKO_GATE_*`, and an attempt-sentinel path.
- Pipeline construction — gate_dispatch.rs:110-119:
  ```
  let pipeline = if gates_config.has_custom_rungs() {
      GatePipelineBuilder::from_config(&gates_config, complexity)               // (A) custom rungs
  } else {
      GatePipelineBuilder::from_config_with_execution(
          &gates_config, complexity, inputs /* =default */, config /* source_roots+timeout */) // (B)
  };
  ```
  **Both branches pass `RungExecutionInputs::default()`** — no symbol/factcheck/judge signal is ever attached in Runner v2. `config` only carries `source_roots` + `timeout_ms`.
- `verdicts = [pipeline.verify(signal)] ++ run_verify_steps(...)` — :121-123. Task `verify` steps from `tasks.toml` run as `ShellGate("bash -o pipefail -c …")` (:359-371) and are appended.
- Whole thing wrapped in `timeout(timeout_secs)`; on elapse → single `Verdict::fail("gate-timeout:rung-N")` — :126-137.
- `passed = verdicts.iter().all(|v| v.passed)` — :140. **Stub passes count.**
- Emits one `GateCompletion { rung: <max_gate_rung>, passed, verdicts: Vec<GateVerdictSummary>, failure_kind, output, duration_ms }` — :171-182.

### 1.5 Pipeline build → rung selection (`GatePipelineBuilder`, rung_dispatch.rs:105)
For the default (non-custom) branch:
```
caps = RungCaps { has_lint_tool: config.clippy_enabled, ..RungCaps::all() }   // rung_dispatch.rs:119
rungs = select_rungs(complexity, &caps, 0)                                     // prior_failures HARD-CODED 0
          .filter(|r| !(config.skip_tests && r == Test))                       // :123-125
```
`select_rungs` (rung_selector.rs:267) = `base_rungs(complexity.escalate_by(0))` filtered by caps. **`prior_failures` is always 0 here** → the "escalation ladder" (Trivial→Simple on repeat failure) documented in rung_selector.rs is *never exercised in Runner v2*; escalation only existed in the legacy `PlanRunner`.

Each selected rung → `gate_from_known_rung` (rung_dispatch.rs:182): Compile/Lint/Test become real `CompileGate/ClippyGate/TestGate::cargo()`; Symbol/GeneratedTest/PropertyTest/Integration become a `CanonicalRungGate` wrapper (:193-200) whose `verify` calls `run_canonical_rung` (:522-533) with the **default** inputs → stubs.

### 1.6 Completion handling (event_loop.rs:1000-1210, the `gate_rx.recv()` arm)
1. `state.clear_gate_active(effect_key)`, store `state.gate_output` — :1007-1008.
2. Per-verdict: `tui.gate_result(...)`, emit `verify_verdicts` metric (:1024-1035), push into `plan_state.gate_results` (:1059).
3. `record_daimon_gate_result` — :1082 (affect engine).
4. `ledger.record_gate_run(...)` → `.roko/state/run-ledger.jsonl` — :1087.
5. `retry_budget = max_retries.min(gate_thresholds.suggested_max_retries(completion.rung))` — :1124.
6. **`update_gate_thresholds(&mut gate_thresholds, completion.rung, completion.passed)`** — :1128 → `thresholds.observe(rung, passed)` (event_loop.rs:5023 → persist.rs:197). **`completion.rung` is always `max_gate_rung` (2).** So the per-rung EMA map only ever gains observations at key `2` — rungs 0,1,3–6 EMA are never updated even though those gates ran. **Per-rung adaptive thresholds are fiction in the live path.**
7. `learning_event_bus.publish(GateResult{ gate_name: "rung-N", ... })` — :1138 (N = max_gate_rung).
8. **Verdict written to `signals.jsonl`** — :1147-1168: `OpenOptions::append` on `config.layout.signals_path()` = `.roko/signals.jsonl` (layout.rs:219). A hand-rolled JSON blob `{kind:"GateVerdict", plan_id, task_id, rung, passed, gate_kind, duration_ms, timestamp}` — *not* a real `Engram`/`Signal`, no lineage, no attestation, no per-gate breakdown (aggregate only).
9. `fire_on_gate_hook` — :1171 (extensions).
10. Branch on `completion.kind` (Merge / PlanVerify / Gate).
11. `if completion.passed && completion.rung < config.max_gate_rung { advance }` — :1206. **Dead branch:** `completion.rung == max_gate_rung` always, so `rung < max_gate_rung` is never true → there is **no incremental rung climb**; the entire complexity-selected pipeline runs in a single dispatch, then the task is done or retried.

### 1.7 Snapshot persistence (thresholds never hit the learn file)
- `save_snapshot(... &gate_thresholds ...)` called at :919, :995, :1739, :1826, :1857 → `build_unified_snapshot` serializes `gate_thresholds_json = to_string_pretty(gate_thresholds)` (event_loop.rs:3401) into the **executor snapshot** (`.roko/state/…`).
- `GateThresholds::save(&path)` (persist.rs:230, targets `.roko/learn/gate-thresholds.json`) **has zero callers in `roko-cli/src`** (only `orchestrate.rs:5953` saves the *other* type, `AdaptiveThresholds`, at legacy teardown).
- **Net:** in the live path `.roko/learn/gate-thresholds.json` is read at startup and never rewritten; live threshold state lives only inside the executor snapshot. Prior docs' "save-at-teardown (5953)" describes the dead legacy engine.

---

## 2. Rung-selection decision table (LIVE Runner v2)

Inputs: task `tier` → `complexity`; `caps.has_lint_tool = clippy_enabled`; `prior_failures ≡ 0`; `skip_tests` drops Test. Rungs that fire and their real status:

| task tier | complexity | rungs selected (rung_selector base_rungs) | which actually verify vs stub-pass |
|---|---|---|---|
| mechanical/fast | Trivial | `[Compile]` | Compile real |
| **focused (default)** | Simple | `[Compile, Lint]` | Compile+Lint real (Lint dropped if `!clippy_enabled`) |
| integrative | Standard | `[Compile, Lint, Test, Symbol]` | Compile/Lint/Test real; **Symbol → stub-pass** |
| architectural/complex/premium | Complex | `[Compile, Lint, Test, Symbol, GeneratedTest, PropertyTest, Integration]` | Compile/Lint/Test real; **PropertyTest real** (runs `cargo test`); Symbol/GeneratedTest+VerifyChain/FactCheck/LlmJudge/Integration → **stub-pass** |

Notes:
- `skip_tests=true` removes `Test` and pins `max_gate_rung` to `clippy_enabled` (0 or 1).
- `has_custom_rungs()` (any `[[gates.rungs]]` in roko.toml) overrides the whole table with the configured shell/known rungs (rung_dispatch.rs:130-145).
- The `select_rungs` escalation-on-failure and `RungCaps` for symbol/gentest/etc. are inert here (prior_failures=0, caps default to `all()` except lint).

---

## 3. All-gates inventory (gate → what it runs → LIVE status)

Real = executes a subprocess/logic and can fail. Stub = returns `Verdict::pass` in Runner v2 because inputs are `default()`. Standalone = not in the rung pipeline.

| # | Gate (file) | Rung | What it actually runs | Runner v2 status |
|---|---|---|---|---|
| 1 | `CompileGate` (compile.rs) | 0 Compile | `cargo check` (build system detected) | ✅ real |
| 2 | `ClippyGate` (clippy_gate.rs) | 1 Lint | `cargo clippy -- -D warnings`; skipped if `!clippy_enabled` | ✅ real (cap-gated) |
| 3 | `TestGate` (test_gate.rs) | 2 Test | `cargo test`, parses pass/fail counts | ✅ real (dropped if `skip_tests`) |
| 4 | `SymbolGate` (symbol_gate.rs) | 3 Symbol | Would check a `SymbolManifest` vs source_roots | 🟡 **stub-pass** — `inputs.symbol_signal=None` (rung_dispatch.rs:303-304) |
| 5 | `GeneratedTestGate` (generated_test_gate.rs) | 4 GenTest | Would run generated behavioural tests from an artifact store | 🟡 **stub-pass** — `config.generated_test_artifacts=None` (:330-331) |
| 6 | `VerifyChainGate` (verify_chain_gate.rs) | 4 GenTest | Would run a task-attached verify script chain | 🟡 **stub-pass** — no `VERIFY_SCRIPT_TAG`, no fallback (:345-347) |
| 7 | `PropertyTestGate` (property_test_gate.rs) | 5 PropTest | `cargo test` for proptest targets — **runs unconditionally** | ✅ real (only advanced rung that fires for real) |
| 8 | `FactCheckGate` (fact_check.rs) | 5 PropTest | Would check claims via a `SearchOracle` (Perplexity) | 🟡 **stub-pass** — no signal + no oracle (:365-368) |
| 9 | `LlmJudgeGate` (llm_judge_gate.rs) | 6 Integration | Would score a diff via `JudgeOracle` | 🟡 **stub-pass** — no judge signal + no oracle (:396-399) |
| 10 | `IntegrationGate` (integration_gate.rs) | 6 Integration | Would run a build-system integration test pattern | 🟡 **stub-pass** — `integration_test_pattern=None` (:413-414) |
| 11 | `ShellGate` (shell.rs) | — | `bash -o pipefail -c <cmd>` for each task `verify` step + custom rungs | ✅ real (task verify + `[[gates.rungs]]`) |
| S1 | `DiffGate` (diff_gate.rs) | standalone | git-diff analysis / review | 🔌 only in legacy `run_gate_rung(...,3)` (orchestrate.rs:18019) — not in Runner v2 |
| S2 | `CodeExecutionGate` (code_exec.rs) | standalone | sandboxed code exec | 🔌 built, not invoked at runtime |
| S3 | `BenchmarkRegressionGate` (benchmark_gate.rs) | standalone | perf-regression bench | 🔌 built, not invoked |
| S4 | `FormatCheckGate` (format_check_gate.rs) | standalone | `cargo fmt --check` | 🔌 built, not invoked |
| S5 | `SecurityScanGate` (security_scan_gate.rs) | standalone | security scan | 🔌 built, not invoked |

Composition wrappers `ParallelGate`/`VotingGate`/`FallbackGate` (composition.rs) and `GateGenerator` (generated.rs) exist but no Runner v2 call site. "11 gates" ≈ the rung-pipeline set (1–11 above); the standalone S1–S5 push the true concrete-gate count higher.

`stub_verdict` (rung_dispatch.rs:290-296):
```
let mut verdict = Verdict::pass(gate);   // ← PASS, not skip
verdict.reason = "stub gate; <detail>";
```
`CanonicalRungGate` aggregates inner verdicts with `all(passed)` (rung_dispatch.rs:535) → stubs make the aggregate pass.

---

## 4. The three rung dialects

Rung index `N` means three different gate-sets depending on which module you read. All three coexist; the runner mixes #1 (execution) with #3-shaped labels.

| Rung | **#1 roko-gate `rung_selector` `CANONICAL_ORDER`** (rung_selector.rs:96-128) — the real pipeline | **#2 roko-gate `registry` `GATE_SPECS`** (registry.rs:134-184) — consumed by roko-runtime | **#3 roko-runtime `effect_driver` `rung_for_gate_name`** (effect_driver.rs:704) — thin wrapper over #2 |
|---|---|---|---|
| 0 | Compile | compile (`compile:cargo`) | compile |
| 1 | Lint (clippy) | clippy (`clippy:cargo`) | clippy |
| 2 | Test | test (`test:cargo`) | test |
| 3 | **Symbol** | **diff** (`diff:git`) | **diff** |
| 4 | **GeneratedTest** (+VerifyChain) | **fmt** (`fmt:cargo`/`format`) | **fmt** |
| 5 | **PropertyTest** (+FactCheck) | **custom/shell** | **custom/shell** (heuristic) |
| 6 | **Integration** (LlmJudge+Integration) | **judge** (`llm-judge`) | **judge** |

- Rungs 0–2 agree across all three. **Rungs 3–6 diverge completely.**
- `effect_driver.rs:336`: `confidence = if rung <= 4 { 1.0 } else { 0.5 }` — bakes dialect #2's numbering (fmt=4 deterministic, shell=5 heuristic) into the affect/feedback policy. When a #1-dialect verdict named e.g. `rung:symbol` flows through, `GateRegistry::rung_for_name` returns `None → u8::MAX` (registry.rs:128-130, effect_driver.rs:705), so it is treated as an unknown max-rung gate.
- roko-cli adds two sentinels on top of #1: `RUNG_PLAN_VERIFY = 1000`, `RUNG_MERGE = 1001` (gate_dispatch.rs:23-25), plus `rung > 6 ⇒ run every rung` semantics (rung_dispatch.rs:226-231).

---

## 5. `GateVerdict` — four incompatible struct definitions (plus the real one)

The Verify trait actually returns `roko_core::Verdict`. Everything below is a *parallel* representation; none share fields, none are convertible without hand-mapping.

| # | Path:line | Fields | Consumer |
|---|---|---|---|
| 1 | `roko-core/src/foundation.rs:368` | `gate_name, passed, skipped, skip_reason, output, duration_ms` | `GateReport`, `GateRunner` trait, roko-runtime `effect_driver` |
| 2 | `roko-learn/src/episode_logger.rs:90` | `gate, passed, signature` (hashed) | episode records `.roko/episodes.jsonl` |
| 3 | `roko-core/src/dashboard_snapshot.rs:290` | `plan_id, task_id, gate, passed, ts_millis` | dashboard snapshot (see §6) |
| 4 | `roko-chain/src/identity_economy_identity.rs:1600` | `gate: GateType, passed, score, detail` | chain witness / futures |

Related but distinct: `GateVerdictSummary` (runner/types.rs:141 **and** roko-runtime/event_bus.rs:75 — itself duplicated) and `GateVerdictRecord` (roko-core/forensic.rs:124). The runner emits `GateVerdictSummary` internally, then serializes an *ad-hoc anonymous JSON object* (§1.6 step 8) to `signals.jsonl` that matches **none** of the four structs.

---

## 6. Sink/source mismatch: verdicts land in `signals.jsonl`, dashboard reads `engrams.jsonl`

- **Write** (Runner v2): `config.layout.signals_path()` → `.roko/signals.jsonl` (event_loop.rs:1159; layout.rs:219).
- **Read** (dashboard): `dashboard_snapshot.rs:1276` `read_signal_gates(&engrams_path)` where `engrams_path = ws.engrams_path()` = `.roko/engrams.jsonl` (dashboard_snapshot.rs:1257-1276, 2891; layout.rs:204).
- layout.rs distinguishes them: `engrams_path()` = `engrams.jsonl` (new canonical, :204); `engrams_path_legacy()` = `signals.jsonl` (:213); `signals_path()` = `signals.jsonl` (:219).
- **The dashboard reads the file the runner does not write.** Live gate verdicts (the ad-hoc JSON) accumulate in `signals.jsonl`; the dashboard's `read_signal_gates` scans `engrams.jsonl` for `Kind::GateVerdict` engrams — which only the legacy `PlanRunner` emitted (via `substrate.put`). So a `roko plan run` produces verdicts the `roko dashboard` gate panel cannot see.

---

## 7. Gate-failure → replan (prompt enrichment only)

On `!completion.passed` the runner does **not** rewrite `tasks.toml`. It:
1. Classifies via `classify_gate_failure("runner", text)` → `GateFailureAction` (Blocked/NeedsHuman/NeedsReplan/Retry) → `RunnerFailureKind` (gate_dispatch.rs:410-449).
2. Records a `PostGateReflection` (roko-learn `post_gate_reflection`, imported event_loop.rs:47) and folds the truncated gate `output` + structured failure classification into the **next agent dispatch prompt** as `GateFeedback` (`render_failure_classification`, event_loop.rs:25).
3. Re-dispatches the same task (bounded by `retry_budget`, §1.6 step 5). The plan DAG, task list, and rung selection are unchanged.

The legacy `build_gate_failure_plan_revision` (an actual plan/tasks revision, cited in CLAUDE.md as "wired") lives on `PlanRunner` and is **not** reached from `event_loop::run`. So "gate-failure replan" in the live path = retry-with-enriched-prompt, not a structural plan rewrite.

---

## 8. Two EMA implementations, and what each actually does

| | `roko_gate::AdaptiveThresholds::observe` (adaptive_threshold.rs:322) | `runner::persist::GateThresholds::observe` (persist.rs:197) |
|---|---|---|
| Used by | legacy `PlanRunner` (dead) | **Runner v2 (live)** |
| EMA | α=0.1, seeds on first obs | α=0.1, seeds on first obs (identical formula) |
| CUSUM shift detection | yes (:340-359) | **no** |
| SPC ensemble (CUSUM+EWMA+BOCPD) | yes (:361-372, drains alerts) | **no** |
| Hotelling T² joint anomaly | yes (`observe_pipeline`) | **no** |
| role/temperament override | `override_for_role` (:300), `threshold_for_temperament` (:586) | **no** |
| `should_skip_rung` (20-streak) | yes (:409) | **no** |
| persisted where | `.roko/learn/gate-thresholds.json` at teardown (orchestrate.rs:5953) | executor snapshot only; **standalone file never written** |
| per-rung keys populated | per real rung (legacy passed true rung) | **only key `max_gate_rung` (=2)** — §1.6 |

So the entire statistical-process-control apparatus in `roko-gate` is invoked only by the engine `roko plan run` does not use. The live engine keeps a single-scalar EMA at rung 2.

---

## 9. Checklist / findings

- [ ] **P0 — Two gate engines; live one skips enrichment.** `roko plan run` → `event_loop::run` → `gate_dispatch::run_gate_once` uses `RungExecutionInputs::default()` (gate_dispatch.rs:104). `enrich_rung_config`/`gate_rung_config` (orchestrate.rs:18430-18525) are only reachable from the dead `PlanRunner`. Docs 35/100 trace the dead path.
- [ ] **P0 — Advanced rungs stub-pass.** Symbol/GeneratedTest/VerifyChain/FactCheck/LlmJudge/Integration all return `Verdict::pass` in Runner v2 (rung_dispatch.rs:290, 303/330/345/365/396/413). Only PropertyTest fires for real among rungs 3–6.
- [ ] **P1 — Per-rung EMA is fiction.** `completion.rung ≡ max_gate_rung` (event_loop.rs:4689, spawn label) ⇒ `GateThresholds::observe` only ever writes rung `2` (event_loop.rs:1128). Rungs 0/1/3–6 never accrue stats.
- [ ] **P1 — Dead advance branch.** `completion.passed && completion.rung < config.max_gate_rung` (event_loop.rs:1206) can never be true → no incremental rung climb; whole pipeline in one dispatch.
- [ ] **P1 — Threshold file never written by runner.** `GateThresholds::save` (persist.rs:230) has no caller in the live path; `.roko/learn/gate-thresholds.json` is read-only-at-startup. Live state hides in the executor snapshot.
- [ ] **P1 — Stub verdicts inflate the EMA.** `passed = all(verdicts.passed)` (gate_dispatch.rs:140) counts stub passes; the single rung-2 EMA trends toward 1.0 regardless of advanced-gate reality.
- [ ] **P2 — Sink/source split.** Runner writes verdicts to `signals.jsonl` (event_loop.rs:1159); dashboard reads `engrams.jsonl` (dashboard_snapshot.rs:1276). Gate panel is blind to live runs.
- [ ] **P2 — Three rung dialects.** rungs 3–6 mean Symbol/GenTest/PropTest/Integration (rung_selector) vs diff/fmt/shell/judge (registry + effect_driver). `effect_driver.rs:336` hardcodes the registry numbering into affect confidence.
- [ ] **P2 — Four `GateVerdict` structs** (foundation/episode_logger/dashboard_snapshot/identity_economy_identity) + duplicated `GateVerdictSummary`; the on-disk verdict matches none of them (ad-hoc JSON).
- [ ] **P2 — `enable_advanced_rungs` dead toggle** (orchestrate.rs:18259-18270): both branches `skipped_count += 1`. Also legacy-path-only, so doubly inert.
- [ ] **P3 — Escalation ladder inert.** `select_rungs(..., prior_failures=0)` hard-codes 0 (rung_dispatch.rs:123); the repeat-failure escalation in rung_selector.rs:267 never runs in Runner v2.
- [ ] **P3 — Standalone gates (Diff/CodeExec/Benchmark/Format/SecurityScan) built, no runtime call site.**

## 10. Roadmap to make the pipeline honest

1. **Wire enrichment into Runner v2** — port `enrich_rung_config`/`gate_rung_config` (or equivalent `RungExecutionInputs`/`RungExecutionConfig` construction) into `gate_dispatch::run_gate_once`, so Symbol/FactCheck/LlmJudge/Integration receive real signals+oracles. Kill the dead `PlanRunner` gate methods once ported.
2. **Label verdicts with the real inner rung**, not `max_gate_rung`, so per-rung EMA is meaningful; iterate `spawn_gate` per selected rung or emit one `GateCompletion` per inner verdict.
3. **Make stubs report `Skipped`/`NotWired`** (registry.rs:`GateStatus::NotWired` already exists) instead of `Verdict::pass`, and exclude them from the EMA `all(passed)`.
4. **Persist thresholds to the learn file** — call `GateThresholds::save(&paths.gate_thresholds_json)` on each `save_snapshot` (or on teardown) in Runner v2.
5. **Unify the write/read path** — either write verdicts as real `Kind::GateVerdict` engrams to `engrams.jsonl`, or point `read_signal_gates` at `signals_path()`.
6. **Collapse the three rung dialects** — make `effect_driver`/`registry` consume `rung_selector::Rung` or add an explicit dialect-translation layer; delete the `confidence = rung<=4` heuristic.
7. **Collapse the four `GateVerdict` structs** to one canonical type (or newtype views over `roko_core::Verdict`).
