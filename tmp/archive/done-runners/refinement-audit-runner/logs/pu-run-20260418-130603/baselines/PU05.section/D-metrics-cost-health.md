# D — Metrics + Cost + Health (Docs 06, 07, 08, 09)

Parity analysis of `docs/05-learning/06-task-metrics-and-baselines.md`,
`07-regression-detection.md`, `08-cost-normalization.md`, and
`09-provider-health-circuit-breaker.md` vs the actual codebase.

---

## D.01 — `AgentEfficiencyEvent` with 20+ fields (identity, tokens, cost, prompt, tools, timing)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 06 §Efficiency Events — struct has six identity fields, five token fields, two cost fields, three prompt fields, three tool fields, three timing fields (20+ total).
**Reality**: `crates/roko-learn/src/efficiency.rs:80` declares the struct with every banner group from the doc plus extras: identity 6 (`agent_id`, `role`, `backend`, `model`, `plan_id`, `task_id`), tokens 5 (`input`, `output`, `reasoning`, `cache_read`, `cache_write`), cost 2 (`cost_usd`, `cost_usd_without_cache`), prompt 3 (`prompt_sections`, `total_prompt_tokens`, `system_prompt_tokens`), tools 3 (`tools_available`, `tools_used`, `tool_calls`), timing 4 incl. `duration_ms` alias (`wall_time_ms`, `duration_ms`, `time_to_first_token_ms`, `was_warm_start`), plus outcome block (`iteration`, `gate_passed`, `outcome`, `gate_errors`, `model_used`, `frequency`, `strategy_attempted`) that doc doesn't list. 29 tests in the file.
**Notes**: Real struct is wider than doc's 20 fields — no harm, just undercount in the prose.

---

## D.02 — `Grade A-D` prompt efficiency grading

**Status**: PARTIAL (LOW severity)
**Doc claim**: Doc 06 §Prompt Efficiency Grading — thresholds `A ≥ 0.8`, `B ≥ 0.6`, `C ≥ 0.4`, `D < 0.4`; composite is `0.30 × section_utilization + 0.30 × token_efficiency + 0.20 × cache_hit_rate + 0.20 × tool_utilization`.
**Reality**: `efficiency.rs:251` declares `pub enum Grade { A, B, C, D }`. Thresholds at `efficiency.rs:338-349` are **`A ≥ 0.75`, `B ≥ 0.50`, `C ≥ 0.25`, `D < 0.25`** — all four cutoffs are 5 basis points lower than doc. Composite weights at `efficiency.rs:329-334` are **`0.4 × signal_ratio + 0.2 × (1 − budget_utilization) + 0.2 × cache_efficiency + 0.2 × outcome`**, not the four-way doc formula.
**Fix sketch**: Update doc 06 §Prompt Efficiency Grading to quote the actual 0.75/0.50/0.25 cutoffs and replace the four-weight table with the `signal_ratio / budget_headroom / cache / outcome` weights from `PromptEfficiencyScore::composite()`.

---

## D.03 — `MetricsWriter` / `MetricsReader` / `MetricFilter` for `task-metrics.jsonl`

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 06 §MetricsWriter and MetricsReader — thread-safe append-only JSONL writer with `parking_lot::Mutex`; reader tolerant of corrupt lines; `MetricFilter` with 11 AND-combined predicates.
**Reality**: `task_metric.rs:130-154` defines `MetricsWriter { buffer: Mutex<Vec<String>> }` (parking_lot) with `append()` that serializes via `serde_json::to_string` and pushes newline-terminated lines. `MetricFilter` at `task_metric.rs:35-58` declares exactly the 11 fields the doc lists (`roles`, `complexity_bands`, `plan_ids`, `gates`, `models`, `backends`, `gate_passed`, `iteration_range`, `min_cost_usd`, `max_cost_usd`, `config_hashes`). Persistence wired at `crates/roko-cli/src/main.rs:1250` and `:7059` reading `.roko/learn/task-metrics.jsonl`; layout constant at `crates/roko-learn/src/runtime_feedback.rs:128`. 24 tests.

---

## D.04 — `TaskMetric` canonical schema

**Status**: PARTIAL (LOW severity)
**Doc claim**: Doc 06 §TaskMetric Schema — 14 fields incl. `cost_usd`, `duration_ms`, `input_tokens`, `output_tokens`, `config_hash: ConfigHash`, `timestamp: DateTime<Utc>`.
**Reality**: `crates/roko-core/src/metric.rs:145` — struct is actually wider. Timing is `wall_time_ms: u64` (not `duration_ms`). Token bookkeeping has a single `cached_tokens` field (not the split `cache_read`/`cache_write` implied by the efficiency event). `timestamp` is `String` (ISO-8601), not `DateTime<Utc>`. Doc omits `run_id` (git SHA) which is a first-class field at `metric.rs:150`.
**Fix sketch**: Update doc 06 code block to use `wall_time_ms`, `cached_tokens`, `timestamp: String`, and add the `run_id` field. Note that `MetricFilter` still matches its struct faithfully.

---

## D.05 — `Baseline` + `SliceBaseline` + `compute_baseline()` per (role, complexity_band)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 06 §Baseline Computation — per-slice struct with 9 fields; overall aggregate; `compute_baseline()` groups records by `(role, complexity_band)`.
**Reality**: `baseline.rs:23-44` defines `SliceBaseline` with exactly the 9 doc fields. `baseline.rs:48-61` defines `Baseline` with `slices: Vec<SliceBaseline>` + `overall_pass_rate` + `overall_avg_cost` + `overall_avg_duration_ms` + `total_records` + `min_records_for_confidence`. Doc uses `overall_n_records` and lists `overall_avg_iterations`; code uses `total_records` and has no `overall_avg_iterations`. `compute_baseline()` at `baseline.rs:128` groups via `HashMap<SliceKey, SliceAccum>` keyed on `(role, complexity_band)`, including the first-attempt-only pass-rate bookkeeping (line 155). 7 tests.
**Notes**: Field naming drift is cosmetic: `total_records` vs `overall_n_records`, plus doc lists an `overall_avg_iterations` that doesn't exist. Per-slice `avg_iterations` does exist.

---

## D.06 — `RegressionThresholds` defaults: 15% / 20% / 30% / 25%

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 07 §Threshold Configuration — `pass_rate_drop=0.15`, `cost_increase=0.20`, `duration_increase=0.30`, `iterations_increase=0.25`, `min_records=5`.
**Reality**: `regression.rs:29-52` — struct declared with those five fields; `impl Default` at line 42 sets exactly those values (`0.15`, `0.20`, `0.30`, `0.25`, `5`). Module header comment at `regression.rs:10-15` enumerates the same defaults and tags pass-rate/cost as alert, duration/iterations as warning.

---

## D.07 — `detect_regressions()` + `RegressionReport` + `RegressionAlert` + `AlertSeverity`

**Status**: PARTIAL (MEDIUM severity)
**Doc claim**: Doc 07 §Detection Algorithm — detection runs for pass rate, cost, duration, iterations; each check fires `Alert`/`Warning`/`Improvement`.
**Reality**: `regression.rs:140` declares `pub fn detect_regressions(baseline, current, thresholds) -> RegressionReport`. `AlertSeverity` at `regression.rs:58-65` has exactly three variants (`Alert`, `Warning`, `Improvement`). `RegressionAlert` at `:69-89` matches doc byte-for-byte including `slice: Option<(String, String)>`. `RegressionReport` at `:93-104` + `regressions()` / `warnings()` filters at `:108-118`. Detection body covers **only three metrics**: pass rate (`:161-200`), cost (`:202-240`), duration (`:242-265`). The fourth metric, `iterations_increase`, is defined in the threshold struct but never checked anywhere in `detect_regressions()`. 9 tests.
**Fix sketch**: Add an iterations block to `detect_regressions()` matching the existing cost/duration pattern, or remove `iterations_increase` from the public threshold struct. Doc 07 text currently promises this check.

---

## D.08 — Per-slice regression analysis

**Status**: NOT DONE (HIGH severity)
**Doc claim**: Doc 07 §Per-Slice Analysis — "Regressions are detected per `(role, complexity_band)` slice, not just in aggregate." Doc's own worked example at §Practical Example emits slice-scoped alerts like `ALERT: pass_rate regression in (Implementer, standard)`.
**Reality**: `regression.rs:140-276` — `detect_regressions()` never iterates `baseline.slices` or `current_baseline.slices`. Every emitted alert sets `slice: None` (lines 180, 197, 221, 237, 262). There is no per-(role, complexity_band) loop in the function body. `RegressionAlert::slice` field is declared but always populated with `None` in production.
**Fix sketch**: Add an outer loop over `baseline.slices` that looks up the matching `current_baseline.slices` entry and re-runs pass-rate/cost/duration checks with `slice: Some((role, complexity))`. Gate on `n_records >= thresholds.min_records`. Without this, the doc's §Practical Example is unreachable and the field exists in name only.

---

## D.09 — Advanced drift detectors (Page-Hinkley / ADWIN / CUSUM / BOCPD / HotellingT2)

**Status**: NOT DONE (LOW severity)
**Doc claim**: Doc 07 does not explicitly enumerate these advanced detectors, but §False Positive Management and §C-Factor Regression discuss statistical techniques that the research docs tie to SPC-style detectors.
**Reality**: `rg 'PageHinkleyDetector|AdwinDetector|CusumDetector|CusumDrift|HotellingT2|BOCPD' crates/` returns zero matches. Only detectors in the repo are the simple EWMA + z-score in `AnomalyDetector` (anomaly.rs) and threshold-based change detection in `regression.rs`.
**Fix sketch**: Either add a §Out of Scope section to doc 07 that explicitly disclaims advanced drift detectors, or land them incrementally. Page-Hinkley is the cheapest entry point (single-sum tracker) and composes cleanly with the existing `RegressionReport` flow.

---

## D.10 — `CostTable` + `ModelPricing` with 3:1 blended cost formula

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 08 §Blended Cost Formula — `blended_cost_per_m = (3 × input_price_per_m + 1 × output_price_per_m) / 4`. Doc 08 §CostTable Design lists `ModelPricing` with six fields including `cache_read_price_per_m: Option<f64>` and a pre-computed `blended_cost_per_m`.
**Reality**: `cost_table.rs:11-22` declares `ModelPricing { input_per_m, output_per_m, cache_read_per_m, cache_write_per_m, tokenizer_ratio }` — five fields, not the six in doc. Cache fields are plain `f64` not `Option<f64>`. `blended_cost_per_m` is **not a stored field**; it's a method at `cost_table.rs:63-70` computing `((3.0 * input + output) / 4.0) * tokenizer_ratio` (note the extra tokenizer normalization the doc omits). Test at `cost_table.rs:199-215` confirms the formula. 5 tests.
**Notes**: Formula is the doc's 3:1 blend; adding tokenizer ratio multiplier is substance beyond the doc. Field naming `input_per_m` (code) vs `input_price_per_m` (doc) is cosmetic.

---

## D.11 — `CostsLog` JSONL append-only persistence

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 08 §CostsLog — path + fsync fields; `at()`, `open_creating()`, `append()`, `append_all()`, `read_all()`; `without_fsync()` builder for tests.
**Reality**: `costs_log.rs:20-23` declares `CostsLog { path: PathBuf, fsync: bool }`. Constructors at `:25-46` (`at()` line 28, `open_creating()` line 40). Complete API present in the 395 LOC file. Path plumbed via `crates/roko-cli/src/main.rs:5315` as `learn_dir.join("costs.jsonl")` and layout constant at `runtime_feedback.rs:124`.

---

## D.12 — `CostsDb` in-memory database with summary aggregations

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 08 §Relationship to CostsDb — `CostsDb` is the in-memory companion to `CostsLog`, replayable on startup.
**Reality**: `costs_db.rs:472` — `pub struct CostsDb { records: RwLock<Vec<CostRecord>> }` with `insert()`/`insert_batch()` (`:485-491`), `len()`/`is_empty()` (`:494-501`). `summary()` / `summary_by_model()` / `summary_by_role()` / `summary_by_plan()` at `:570-602` produce `CostSummary` aggregates. `CostSummary` struct at `:61-78` exposes 8 fields; doc 08 §CostSummary lists 7 fields including `avg_cost_per_success` which is **not** present in the real struct (real has `success_rate` instead). 27 tests.
**Notes**: `CostSummary` field drift: doc names `avg_cost_per_success`/`success_count`; code has `success_rate` + `record_count`. Cosmetic, but worth updating.

---

## D.13 — `BudgetGuardrail` multi-level: per-task / per-session / per-day

**Status**: PARTIAL (MEDIUM severity)
**Doc claim**: Doc 08 §Budget Guardrails — `BudgetGuardrail { per_task_limit, per_session_limit, per_day_limit }` with `BudgetAction { Continue, Downgrade, Block, HardStop }` and doc §Escalation Thresholds table lists 80% → Downgrade, 95% → Block, 100% → HardStop.
**Reality**: `budget.rs:8-20` — struct is `BudgetGuardrail { per_task_limit_usd, per_session_limit_usd, per_day_limit_usd, warn_at_percent, task_spent, session_spent, day_spent }`. `BudgetAction` at `:24-40` has **five variants**: `Ok`, `Warn { percent_used, level }`, `RouteToCheaper`, `BlockNewSessions`, `Block`. **No `HardStop` variant.** Threshold mapping at `:82-102`: `>= 1.0 → Block`, `>= 0.95 → BlockNewSessions`, `>= 0.80 → RouteToCheaper`, `>= warn_at_percent → Warn`. Test at `:132-159` confirms. No `MultiLevelBudget` type (`rg 'MultiLevelBudget' crates/` empty).
**Fix sketch**: Align doc 08 naming to the actual variants: `Continue→Ok`, `Downgrade→RouteToCheaper`, `Block→BlockNewSessions`, `HardStop→Block`. Escalation thresholds match in spirit but the terminal state at 100% is called `Block`, not `HardStop`. Also document the 4th `warn_at_percent` soft threshold that the doc hides.

---

## D.14 — `ProviderHealthRegistry` + `CircuitState` three-state circuit breaker

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 09 §Three-State Circuit Breaker — `Closed`/`Open`/`HalfOpen`. Registry exposes `record_success`, `record_failure`, `is_available`, `available_providers`.
**Reality**: `provider_health.rs:43-50` — `CircuitState { Closed, Open, HalfOpen }`. `ProviderHealthRegistry` at `:182-186` wraps `Arc<Mutex<HashMap<String, ProviderHealth>>>` plus an async save worker. `record_success()` at `:208-221`, `record_failure()` call sites at `:959-963`, `:994-999`, etc. Debounced persistence via `HEALTH_SAVE_DEBOUNCE` (line 188) and `PersistCommand::{Dirty, FlushAndStop}` (line 191). 17 tests.

---

## D.15 — `ErrorClass` taxonomy with cooldown matrix

**Status**: PARTIAL (LOW severity)
**Doc claim**: Doc 09 §Error Classification — 7 variants (`RateLimit`, `AuthFailure`, `Timeout`, `ServerError`, `ContentPolicy`, `ContextOverflow`, `Unknown`). Doc §Error-Specific Cooldowns table lists cooldowns in seconds: RateLimit 60s, AuthFailure 300s, Timeout 30s, ServerError 120s, ContentPolicy 0s, ContextOverflow 0s, Unknown 60s.
**Reality**: `provider_health.rs:54-69` — all 7 variants match the doc. Cooldown mapping at `:162-165` is in **milliseconds, not seconds**, and covers only 4 of the 7 classes: `RateLimit => 5_000`, `Timeout => 10_000`, `ServerError => 30_000`, `AuthFailure => 300_000`. `ContentPolicy`, `ContextOverflow`, `Unknown` have no explicit arm. Doc's 60s/30s/120s schedule vs code's 5s/10s/30s is an order-of-magnitude mismatch.
**Fix sketch**: Either update the code to match the doc cooldown schedule (60/300/30/120/0/0/60) or update the doc to reflect the actual 5s/10s/30s/300s values. Also add arms for the three unhandled variants so the intent is explicit.

---

## D.16 — `LatencyRegistry` + `LatencyStats` with EMA

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 09 is light on `LatencyRegistry` but §ProviderHealthTracker mentions per-provider latency rollups; latency is the companion metric to failure rate.
**Reality**: `latency.rs:20-35` — `LatencyStats` with `model_slug`, `provider_id`, `ttft_ema_ms`, `total_latency_ema_ms`, `tokens_per_second_ema`, `observations`, `recent_latencies: VecDeque<f64>`. EMA with α = 0.1 at `:40`. `LatencyRegistry` at `:123-127` wraps `Arc<Mutex<HashMap<(String, String), LatencyStats>>>` plus the same debounced persistence pattern as health registry. Imported and used in orchestrate.rs line 84. 8 tests.

---

## D.17 — `AnomalyDetector` per-session wiring in orchestrate

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 09 §Anomaly Detection Integration — `AnomalyDetector` carries `prompt_hash_window`, EWMA cost baseline, quality history, session cost; 3 anomaly types (prompt loop, cost spike, quality degradation); thresholds `PROMPT_LOOP_THRESHOLD = 5`, `COST_SPIKE_Z_THRESHOLD = 3.0`, EWMA α = 0.2.
**Reality**: `anomaly.rs:20-26` — struct matches exactly. Constants at `:9-12`: `PROMPT_LOOP_WINDOW = 20`, `PROMPT_LOOP_THRESHOLD = 5`, `COST_SPIKE_Z_THRESHOLD = 3.0`, `QUALITY_WINDOW = 50` (doc says 10 prior + 5 recent; code uses a 50-sample rolling window). `Anomaly` enum at `:206` has `PromptLoop`, `CostSpike`, `QualityDegradation`, plus a fourth `BudgetExhausted` variant the doc doesn't mention. Wired per-session in orchestrate at `:3279`, `:3398`, `:3521` (`AnomalyDetector::new(now_unix_ms_i64())`). `session_start_ms()` accessor at `anomaly.rs:43-44` is consumed by runner at `orchestrate.rs:10590` and `:10701`. 4 tests.
**Notes**: `QUALITY_WINDOW = 50` in code vs "recent 5 + prior 10" in doc prose is a constant-value drift worth flagging in a future pass but not a functional gap.

---

## D.18 — Regression alerts logged via tracing in orchestrate

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 07 §LearningRuntime Integration — regression report is consumed and alerts are surfaced.
**Reality**: `crates/roko-cli/src/orchestrate.rs:7479-7495` — `handle_learning_update()` checks `update.regression_report` and, when `report.has_regressions`, iterates `report.regressions()` emitting `tracing::warn!` with `plan_id`, `metric`, `severity`, `description`. Extracted skill IDs are logged at `:7493-7494` with `tracing::info!`. Wired through `LearningRuntime::record_completed_run()` in `runtime_feedback.rs:1356` which calls `detect_regressions` with `cfg: &RegressionConfig { current_window: 20 }` default.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 11 |
| PARTIAL | 5 (D.02, D.04, D.07, D.13, D.15) |
| NOT DONE | 2 (D.08, D.09) |

The metrics, cost, and health layer is substantively wired. `AgentEfficiencyEvent`,
`MetricsWriter`, `Baseline`, `CostTable`, `CostsLog`, `BudgetGuardrail`,
`ProviderHealthRegistry`, `LatencyRegistry`, and `AnomalyDetector` all exist with
real persistence, real thresholds, and real call sites in `orchestrate.rs`.

The main drift is in doc accuracy, not missing code:

1. **Regression detector is overall-only** (D.08). Per-slice analysis is the
   doc's headline feature and its worked example, but `detect_regressions()`
   never loops `baseline.slices`. Every emitted alert has `slice: None`. Highest
   severity in this batch.
2. **Iterations threshold is dead code** (D.07). `iterations_increase` lives in
   the public struct and Default, but no detection block reads it.
3. **Budget variant names don't match** (D.13). Code has 5 variants
   (`Ok/Warn/RouteToCheaper/BlockNewSessions/Block`), doc lists 4
   (`Continue/Downgrade/Block/HardStop`).
4. **Cooldown unit mismatch** (D.15). Doc lists seconds (60/300/30/120); code
   uses milliseconds (5_000/10_000/30_000/300_000). Order of magnitude apart
   for RateLimit, Timeout, ServerError.
5. **Grade thresholds quoted wrong** (D.02). Doc says 0.8/0.6/0.4; code uses
   0.75/0.50/0.25. Composite formula weights also differ from doc prose.
6. **TaskMetric field drift** (D.04). `wall_time_ms` not `duration_ms`;
   `cached_tokens` not split cache read/write; `timestamp: String` not
   `DateTime<Utc>`; undocumented `run_id` field.

Advanced drift detectors (Page-Hinkley, ADWIN, CUSUM, BOCPD) are entirely
absent from the repo and are marked NOT DONE (D.09). They are low severity
because no doc explicitly lists them as required; the gap is mostly
prospective and the existing EWMA + threshold path is functional.

## Agent Execution Notes

### D.07 / D.08 — Regression Activation

This is one of the best bounded runtime batches in `05`.

Recommended slice:

1. iterate `baseline.slices`,
2. emit slice-aware alerts,
3. activate `iterations_increase`,
4. keep overall alerts intact.

Acceptance criteria:

- slice-aware regressions are real,
- iteration regressions are no longer dead code,
- alert consumers do not lose the existing overall view.

### D.13 — Budget Pressure Contract

If a batch touches budget pressure, prefer making it a clearer routing input over renaming enum variants in docs only. Keep hard blocking as a backstop.

### D.09 — Defer By Default

Do not default to building advanced drift detectors in batch `05` unless they become necessary to support an already-shipped runtime contract.
