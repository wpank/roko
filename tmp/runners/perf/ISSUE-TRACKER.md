# Perf Runner — Issue Tracker

Master checklist for the 21 perf optimization batches under
`tmp/runners/perf/prompts/`. Each batch links back to its source plan
in `tmp/solutions/perf/implementation/` and to its prompt under
`prompts/`.

**Update this file** whenever a batch lands. Convention:

```
- [ ] PERF_NN — title             (← unchecked = pending)
- [x] PERF_NN — title — <commit-sha>   (← checked = done)
- [~] PERF_NN — title — <reason>      (← in-progress / blocked / partial)
```

For each batch, sub-items are the acceptance criteria copied from the
source plan's "Status check" section. Tick those individually when
verified.

**Progress at a glance:** 0 / 21 batches complete (0 %).

---

## Phase 0 — Low-hanging fruit (5 batches, ~7 h, ~245 ms wall-time)

### perf_01 — Shared config cache (B02)

- [ ] **PERF_01** — Shared config cache (B02) — [prompt](./prompts/PERF_01.prompt.md) — [plan](../../solutions/perf/implementation/01-shared-config-cache.md)
  - [ ] `ConfigBundle` struct exists in `crates/roko-cli/src/config.rs` with `legacy: Arc<Config>`, `roko: Arc<RokoConfig>`, `workdir: PathBuf`.
  - [ ] `ConfigBundle::load(workdir)` is the only place that calls `load_layered` + `load_config` + `apply_process_env` + `merge_global_providers`.
  - [ ] `main.rs` builds the bundle once at CLI entry; subcommand handlers accept `&ConfigBundle`.
  - [ ] `run_once`, `dispatch_agent`, `append_episode_log`, `resolved_model` no longer call `load_config` or `load_layered` themselves.
  - [ ] `rg "load_config|load_layered" crates/roko-cli/src/` shows references only inside `ConfigBundle::load`.
  - [ ] Test asserts `roko run` parses `roko.toml` exactly once.
  - [ ] `cargo clippy -p roko-cli --release -- -D warnings` clean (validated post-merge).
  - [ ] Commit message trailer: `tracker: PERF_01 done <sha>`.

### perf_02 — LearningRuntime single-open (B03)

- [ ] **PERF_02** — LearningRuntime single-open (B03) — [prompt](./prompts/PERF_02.prompt.md) — [plan](../../solutions/perf/implementation/02-learning-runtime-single-open.md)
  - [ ] `LearningRuntime::open_under{,_with_models}` is called exactly once inside `run_once`.
  - [ ] `append_episode_log` accepts `&mut LearningRuntime` instead of opening its own.
  - [ ] `set_episode_completion_hook` registration preserved at the single open site.
  - [ ] `tracing::info!(target = "roko_perf", ..., "learning_runtime_opened")` emitted at the open site.
  - [ ] `load_roko_config_models(workdir)` no longer called twice in the run path.
  - [ ] `cargo test -p roko-cli` and `cargo test -p roko-learn` pass.
  - [ ] Commit message trailer: `tracker: PERF_02 done <sha>`.

### perf_03 — Contract cache audit (B05)

- [ ] **PERF_03** — Contract cache audit (B05) — [prompt](./prompts/PERF_03.prompt.md) — [plan](../../solutions/perf/implementation/03-contract-cache-audit.md)
  - [ ] No production caller bypasses `AgentContract::load_for_role` (audited via `rg`).
  - [ ] Doc comment on `CONTRACT_CACHE` explicitly states immutability for the process lifetime.
  - [ ] Regression test `load_for_role_reads_disk_only_once_per_role` passes.
  - [ ] Optional `prewarm_contracts()` helper exists and is called from CLI startup OR the optional step is documented as deferred.
  - [ ] Commit message trailer: `tracker: PERF_03 done <sha>`.

### perf_04 — Buffered JSONL event logger (B11+B13)

- [ ] **PERF_04** — Buffered JSONL event logger (B11+B13) — [prompt](./prompts/PERF_04.prompt.md) — [plan](../../solutions/perf/implementation/04-buffered-jsonl-logger.md)
  - [ ] `JsonlLogger::write_event` no longer calls `flush()` per event.
  - [ ] `JsonlLogger::flush()` is `pub` and called from workflow completion in `WorkflowEngine::run`.
  - [ ] `Drop for JsonlLogger` flushes best-effort.
  - [ ] `BufWriter` capacity raised to 8 KiB (`with_capacity(8 * 1024, file)`).
  - [ ] Thread-local serialization scratch buffer (`SCRATCH`) used in `write_event`.
  - [ ] Test `buffered_writes_persist_after_explicit_flush` passes (1 000 events).
  - [ ] Test `dropped_logger_persists_buffered_events` passes.
  - [ ] Test `reader_tolerates_partial_last_line` passes.
  - [ ] No reader regression discovered (audited per plan §Step 4).
  - [ ] Commit message trailer: `tracker: PERF_04 done <sha>`.

### perf_05 — Adopt FileSubstrate::put_batch everywhere (B10)

- [ ] **PERF_05** — Adopt `FileSubstrate::put_batch` everywhere (B10) — [prompt](./prompts/PERF_05.prompt.md) — [plan](../../solutions/perf/implementation/05-batch-substrate-writes.md)
  - [ ] No production code path issues ≥3 sequential `substrate.put` calls without batching (verified by `rg`).
  - [ ] Sequential `put` call sites that genuinely need ordering are annotated with a `// SAFETY-ORDER:` comment.
  - [ ] Partial-write replay test `replay_skips_partial_last_line` exists and passes.
  - [ ] `put_batch([])` is a no-op (guarded with `if !signals.is_empty()`).
  - [ ] (Optional) `Store` trait grew a default `put_batch` impl — OR documented as deferred.
  - [ ] Commit message trailer: `tracker: PERF_05 done <sha>`.

---

## Phase 1 — Prompt assembly cache (1 batch, ~3 h, ~50-150 ms wall-time)

### perf_06 — PromptAssemblyService convention cache (B12+B14)

- [ ] **PERF_06** — PromptAssemblyService convention cache (B12+B14) — [prompt](./prompts/PERF_06.prompt.md) — [plan](../../solutions/perf/implementation/06-prompt-assembly-cache.md)
  - [ ] `lru` dependency added to `crates/roko-compose/Cargo.toml`.
  - [ ] `ConventionCacheEntry` struct + `compute_convention_entry` helper added.
  - [ ] `PromptAssemblyService` carries `convention_cache: Mutex<lru::LruCache<PathBuf, ConventionCacheEntry>>` (cap 8).
  - [ ] `cached_conventions` and `cached_file_listing` accessors exist.
  - [ ] All callers of `detect_workdir_conventions` / `collect_source_context` route through the cache.
  - [ ] Async path uses `tokio::task::spawn_blocking` for the cache miss branch.
  - [ ] No `std::sync::Mutex` held across `.await`.
  - [ ] Test `cached_conventions_avoid_disk_on_second_call` passes.
  - [ ] Test `cache_invalidates_on_cargo_toml_mtime_change` passes.
  - [ ] Test `lru_evicts_oldest_workdir` passes.
  - [ ] Commit message trailer: `tracker: PERF_06 done <sha>`.

---

## Phase 2 — Routing + warm pool (5 batches, ~21 h)

### perf_07 — Routing decision cache (B06)

- [ ] **PERF_07** — Routing decision cache (B06) — [prompt](./prompts/PERF_07.prompt.md) — [plan](../../solutions/perf/implementation/07-routing-cache.md)
  - [ ] New module `crates/roko-learn/src/cascade/routing_cache.rs` exists with `RoutingCache`, `CachedSignals`, `CachedDecision`.
  - [ ] `lru` dep present in `roko-learn`'s `Cargo.toml` (or already exists).
  - [ ] `RoutingCache::signals()` honours both 10 s TTL and `efficiency.jsonl` mtime invalidation.
  - [ ] `RoutingCache::lookup_decision`/`record_decision` honour 5 min TTL with LRU cap 1024.
  - [ ] `routing_cache_key` buckets quality into deciles (no float hashing).
  - [ ] `Orchestrator` owns one `Arc<RoutingCache>` per workdir; replaces all `load_efficiency_signals_sync` call sites in the dispatch hot path.
  - [ ] CLI `--no-routing-cache` flag exists and disables both signal and decision caches.
  - [ ] Test `signals_are_cached_within_ttl` passes.
  - [ ] Test `signals_invalidate_on_mtime_change` passes.
  - [ ] Test `decision_cache_hits_for_same_key` + `_misses_after_ttl` pass.
  - [ ] Commit message trailer: `tracker: PERF_07 done <sha>`.

### perf_08 — Parallel per-dispatch enrichment (B07)

- [ ] **PERF_08** — Parallel per-dispatch enrichment (B07) — [prompt](./prompts/PERF_08.prompt.md) — [plan](../../solutions/perf/implementation/08-parallel-enrichment.md)
  - [ ] At least one dispatch-time enricher join site uses `tokio::join!` instead of serial awaits.
  - [ ] Error semantics preserved or explicitly documented (best-effort vs short-circuit).
  - [ ] Long-running enrichers wrapped in `tokio::time::timeout(...)`.
  - [ ] `EnrichmentPipeline::run_steps` body is **unchanged**.
  - [ ] Doc comment block added to `EnrichmentPipeline::run_steps` explaining the sequential invariant ("Sequential by design ...").
  - [ ] Existing test `run_steps_executes_only_requested_steps_in_explicit_order` still green.
  - [ ] Commit message trailer: `tracker: PERF_08 done <sha>`.

### perf_09 — WarmDispatchPool module (B15 part 1)

- [ ] **PERF_09** — WarmDispatchPool module (B15 part 1) — [prompt](./prompts/PERF_09.prompt.md) — [plan](../../solutions/perf/implementation/09-warm-dispatch-pool.md)
  - [ ] New file `crates/roko-runtime/src/warm_dispatch_pool.rs` exists.
  - [ ] `WarmDispatchPool`, `WarmPoolConfig`, `WarmPoolMetrics`, `WarmSlotGuard`, `ModelCallerFactory` exported.
  - [ ] `acquire(provider, model) -> Option<WarmSlotGuard>` records hit/miss metrics.
  - [ ] `pre_warm()` populates configured targets.
  - [ ] `evict_idle()` removes slots past `idle_timeout`.
  - [ ] Pool uses `tokio::sync::Mutex` (acquire crosses awaits).
  - [ ] `metrics()` returns a snapshot by value.
  - [ ] `crates/roko-runtime/src/lib.rs` re-exports the module's public types.
  - [ ] Unit tests cover acquire/release/eviction/pre_warm/metrics.
  - [ ] Commit message trailer: `tracker: PERF_09 done <sha>`.

### perf_10 — Wire warm pool into EffectDriver + run.rs (B15 part 2)

- [ ] **PERF_10** — Wire warm pool into EffectDriver + run.rs (B15 part 2) — [prompt](./prompts/PERF_10.prompt.md) — [plan](../../solutions/perf/implementation/09-warm-dispatch-pool.md)
  - [ ] `EffectServices` carries `warm_pool: Option<Arc<WarmDispatchPool>>` (default `None`).
  - [ ] `EffectDriver::spawn_agent` consults the pool when present, falls back to `services.model_caller` otherwise.
  - [ ] Released slot via explicit `pool.release(idx)` (not `Drop`).
  - [ ] `infer_provider_from_model` helper added to `roko_agent::provider` (single source of truth).
  - [ ] `build_workflow_effect_services` constructs the pool empty when `[conductor.warm_pool] enabled = true`.
  - [ ] `build_caller_factory(&model_config)` returns the `ModelCallerFactory`.
  - [ ] `FeedbackEvent::ModelCall` carries optional `warm_hit: bool` (serde `default false`).
  - [ ] Test `second_dispatch_in_standard_workflow_uses_warm_slot` passes.
  - [ ] Commit message trailer: `tracker: PERF_10 done <sha>`.

### perf_11 — Warm pool config + serve startup + metrics route (B15 part 3)

- [ ] **PERF_11** — Warm pool config + serve startup + metrics route (B15 part 3) — [prompt](./prompts/PERF_11.prompt.md) — [plan](../../solutions/perf/implementation/09-warm-dispatch-pool.md)
  - [ ] `WarmPoolConfigSchema` added to `roko-core::config::schema` with sane defaults.
  - [ ] Default `roko.toml` documents `[conductor.warm_pool]`.
  - [ ] `roko serve` startup constructs the pool with `pre_warm: true` + configured targets.
  - [ ] Periodic eviction task spawned with 60 s tick.
  - [ ] `/v1/perf/warm-pool` route returns `WarmPoolMetrics` JSON.
  - [ ] Pool injected into per-request `EffectServices`.
  - [ ] Commit message trailer: `tracker: PERF_11 done <sha>`.

---

## Phase 3 — Gate pipeline (4 batches, ~11 h, 800-2000 ms savings on applicable runs)

### perf_12 — Express gate mode + auto-detect (B08-a)

- [ ] **PERF_12** — Express gate mode + auto-detect (B08-a) — [prompt](./prompts/PERF_12.prompt.md) — [plan](../../solutions/perf/implementation/10-express-gate-mode.md)
  - [ ] `GateMode` enum (`Full`, `Express`, `None`, `Auto`) added to `pipeline_state.rs` (with serde + `Default = Full`).
  - [ ] `GateConfig` (in `roko_core::foundation`) carries `gate_mode: GateMode`.
  - [ ] `WorkflowConfig.gate_mode` exists; presets default appropriately (`express()` -> Express, others -> Full).
  - [ ] `EXPRESS_GATE_NAMES = ["diff", "fmt", "format-check"]` constant in `gate_service.rs`.
  - [ ] `GateService::run_gates` filters by mode; emits `skipped: true` verdicts for filtered gates with `skip_reason = "gate_mode=..."`.
  - [ ] `detect_gate_mode(workdir) -> GateMode` exists and covers `.rs`, `.ts`, `.py`, `.go`, `.md`, `.toml`, `.yaml`, `.json`.
  - [ ] CLI flag `--gates {full|express|none|auto}` added (clap).
  - [ ] Tests: `express_mode_skips_compile_and_test`, `detect_gate_mode_*` (md-only, rust-change, no-diff) pass.
  - [ ] Commit message trailer: `tracker: PERF_12 done <sha>`.

### perf_13 — Source-hash gate skip (B08-b)

- [ ] **PERF_13** — Source-hash gate skip (B08-b) — [prompt](./prompts/PERF_13.prompt.md) — [plan](../../solutions/perf/implementation/11-source-hash-gate-skip.md)
  - [ ] `crates/roko-gate/src/source_hash.rs` exists with `GateHashCache` + `compute_source_hash`.
  - [ ] `sha2 = "0.10"` and `ignore = "0.4"` added to `crates/roko-gate/Cargo.toml`.
  - [ ] `gate_input_set(workdir, gate_name)` covers `compile`, `clippy`, `fmt`, `format-check`, `test`.
  - [ ] `GateService` carries `hash_cache: Mutex<GateHashCache>` + `hash_cache_path: PathBuf`.
  - [ ] Skip emits `GateVerdict { skipped: true, skip_reason: Some("source-hash:<12>") }`.
  - [ ] Cache persisted via `roko_fs::atomic::atomic_write_bytes`.
  - [ ] CLI flag `--no-gate-cache` exists.
  - [ ] Tests: `second_run_with_unchanged_source_is_skipped`, `modifying_a_file_invalidates_skip`, `input_set_excludes_target_dir` pass.
  - [ ] Commit message trailer: `tracker: PERF_13 done <sha>`.

### perf_14 — Parallel gate rungs (B08-c)

- [ ] **PERF_14** — Parallel gate rungs (B08-c) — [prompt](./prompts/PERF_14.prompt.md) — [plan](../../solutions/perf/implementation/12-parallel-gate-rungs.md)
  - [ ] `parallel_safe_pair` whitelist covers only `(compile, fmt)`, `(compile, format-check)`, `(clippy, fmt)`, `(clippy, format-check)`.
  - [ ] `build_parallel_groups(names) -> Vec<Vec<String>>` greedy grouping implemented.
  - [ ] `run_one_gate(name, config)` extracted as helper.
  - [ ] `run_gates` loops over groups; uses `futures::future::join_all` for groups of size > 1.
  - [ ] Compile-failure short-circuit preserved (downstream gates emit `skip_reason = "compile-failed-dependency"`).
  - [ ] Verdict order in output matches rung order (re-sort after parallel join).
  - [ ] `RuntimeEvent::Gate{Started,Passed,Failed}` carry `group_id: u32` (additive serde field).
  - [ ] Tests: `compile_and_fmt_run_in_parallel`, `compile_failure_skips_dependents_in_next_group`, `build_parallel_groups_coalesces_safe_pairs` pass.
  - [ ] Commit message trailer: `tracker: PERF_14 done <sha>`.

### perf_15 — Git diff cache for gate phase (B09)

- [ ] **PERF_15** — Git diff cache for gate phase (B09) — [prompt](./prompts/PERF_15.prompt.md) — [plan](../../solutions/perf/implementation/13-git-diff-cache.md)
  - [ ] `crates/roko-cli/src/git_diff_snapshot.rs` exists with `GitDiffSnapshot` + `compute()`.
  - [ ] `Orchestrator` owns a `RwLock<HashMap<plan_id, Arc<GitDiffSnapshot>>>` cache.
  - [ ] Snapshot computed at gate-phase start, cleared at phase end (and at start of each iteration).
  - [ ] `gate_diff_for_plan` reads from snapshot only.
  - [ ] `build_review_prompt`'s `files_changed` reads from snapshot only.
  - [ ] `run_plan_verify_steps` retains its own `--cached` spawn with a documenting `// SEMANTICS:` comment.
  - [ ] (If PERF_12 already merged) `detect_gate_mode` refactored to take `&GitDiffSnapshot`.
  - [ ] Tests: `snapshot_includes_full_diff_and_modified_files`, `snapshot_is_empty_on_clean_repo`, `two_consumers_share_one_snapshot` pass.
  - [ ] Commit message trailer: `tracker: PERF_15 done <sha>`.

---

## Phase 4 — Advanced (3 batches)

### perf_16 — Speculative reviewer pre-warm (deps PERF_11)

- [ ] **PERF_16** — Speculative reviewer pre-warm — [prompt](./prompts/PERF_16.prompt.md) — [plan](../../solutions/perf/implementation/14-speculative-execution.md)
  - [ ] `WarmDispatchPool::pre_warm_for(self: Arc<Self>, provider: String, model: String)` exists; spawns and dedupes (`try_lock` fast-path + post-lock check).
  - [ ] `WorkflowMetadata { reviewer_target, strategist_target }` plumbed through `EffectDriver`.
  - [ ] Engine populates metadata for `standard`/`full` workflows from resolved selection.
  - [ ] Speculation budget `AtomicU32` (default 3 per run) caps pre-warms; exhausted budget logs once at `debug!`.
  - [ ] `[conductor.workflow.speculation]` config flag (default `true`) controls behaviour.
  - [ ] Test `standard_workflow_reviewer_hits_warm_slot` passes.
  - [ ] Test `pre_warm_does_not_duplicate_slots` passes.
  - [ ] Test `speculation_budget_caps_pre_warms` passes.
  - [ ] Commit message trailer: `tracker: PERF_16 done <sha>`.

### perf_17 — Plan executor parallel dispatch (Feature A of plan 15)

- [ ] **PERF_17** — Plan executor parallel dispatch (Feature A) — [prompt](./prompts/PERF_17.prompt.md) — [plan](../../solutions/perf/implementation/15-batch-inference.md)
  - [ ] `crates/roko-cli/src/dispatch/parallel.rs` exists with `ConcurrencyPolicy` + `dispatch_group`.
  - [ ] `default_provider_limits()` covers OpenAI, Anthropic, Gemini, Cerebras, Moonshot, Ollama.
  - [ ] `group_dispatchable(tasks) -> HashMap<(provider, model), Vec<&ReadyTask>>` exists.
  - [ ] Plan executor groups ready tasks by provider/model and dispatches with per-provider semaphores.
  - [ ] `[conductor.plan.parallel]` config flag (default `true`) toggles feature.
  - [ ] Test `dispatch_group_respects_concurrency_cap` passes.
  - [ ] Feature B (`--batch-async`) explicitly **out of scope** for this batch (deferred to future runner).
  - [ ] Commit message trailer: `tracker: PERF_17 done <sha>`.

### perf_18 — PGO release build pipeline

- [ ] **PERF_18** — PGO release build pipeline — [prompt](./prompts/PERF_18.prompt.md) — [plan](../../solutions/perf/implementation/16-pgo-build.md)
  - [ ] `scripts/pgo-train.sh` and `scripts/pgo-build.sh` exist and are `chmod +x`.
  - [ ] `fixtures/pgo-workdir/` contains `roko.toml` (mock provider) + `plans/test/plan.md` + a minimal `src/` tree.
  - [ ] `crates/roko-cli/benches/cli_overhead.rs` benches `config_show`, `plan_validate` via `criterion`.
  - [ ] `criterion` added as dev-dep in `crates/roko-cli/Cargo.toml`.
  - [ ] `.github/workflows/release.yml` has `pgo-build` job (uploads artifact `roko-pgo-${{ github.sha }}`).
  - [ ] PGO job triggers only on `push: branches: [main]` (NOT on pull_request).
  - [ ] Commit message trailer: `tracker: PERF_18 done <sha>`.

---

## External — Eval (3 batches, ~30 h)

### perf_19 — HAL agent wrapper + nightly CI

- [ ] **PERF_19** — HAL agent wrapper + nightly CI — [prompt](./prompts/PERF_19.prompt.md) — [plan](../../solutions/perf/implementation/17-hal-integration.md)
  - [ ] `roko run --output json` produces a single canonical JSON object on stdout (logs go to stderr).
  - [ ] `hal/roko_agent/main.py` exposes `run(task, **kwargs) -> dict` with `model_patch`, `cost`, `tokens`, `duration_s`, `model`, `workflow`, `exit_code` keys.
  - [ ] `hal/roko_agent/requirements.txt` exists (`hal-harness>=0.4.0`).
  - [ ] `hal/roko_agent/tests/test_main.py` covers `_build_prompt` shape variations + `_parse_roko_output` tolerance.
  - [ ] `hal/README.md` documents the quick-start invocation.
  - [ ] `.github/workflows/hal-bench.yml` exists with `schedule` + `workflow_dispatch` triggers.
  - [ ] `_setup_workspace` skips `roko init` if `roko.toml` already exists.
  - [ ] `_setup_workspace` clones with `--depth 50`.
  - [ ] No secrets committed; `OPENAI_API_KEY` referenced as `secrets.OPENAI_API_KEY`.
  - [ ] Commit message trailer: `tracker: PERF_19 done <sha>`.

### perf_20 — Bench K-trial trials + cost wiring

- [ ] **PERF_20** — Bench K-trial trials + cost wiring — [prompt](./prompts/PERF_20.prompt.md) — [plan](../../solutions/perf/implementation/18-bench-suite-extension.md)
  - [ ] `SweBenchOptions.trials: usize` field exists (default 1).
  - [ ] `run_instance_with_trials` runs each instance K times, returns `InstanceTrialReport`.
  - [ ] `ConsistencyMetrics` struct has `trials`, `passes`, `k_pass_rate`, `all_pass`, `distribution_consistency`, `sequence_consistency`, `cost_cv`, `duration_cv`.
  - [ ] `Option<f64>` (NOT `f64::NAN`) used for any metric that may be undefined.
  - [ ] `BenchResult.cost_usd` reads from `roko_learn::costs_db::CostsDb::cost_for_run`.
  - [ ] Missing-cost fallback emits a `bench_cost_missing_total` metric counter.
  - [ ] Test `multi_trial_consistency_computes_k_pass_rate` passes.
  - [ ] Test `cost_is_wired_from_costs_db` passes.
  - [ ] Commit message trailer: `tracker: PERF_20 done <sha>`.

### perf_21 — Bench compare subcommand + result layout (deps PERF_20)

- [ ] **PERF_21** — Bench compare subcommand + result layout — [prompt](./prompts/PERF_21.prompt.md) — [plan](../../solutions/perf/implementation/18-bench-suite-extension.md)
  - [ ] `crates/roko-cli/src/commands/bench_compare.rs` exists with `BenchCompareArgs`.
  - [ ] `roko bench compare BASELINE CANDIDATE [--threshold 20] [--pareto]` exits non-zero if any task regresses by more than `--threshold` percent.
  - [ ] Sanity check: `baseline.config_hash != candidate.config_hash` warns and requires `--force`.
  - [ ] Stable result layout: `.roko/bench/perf/YYYYMMDD-HHMMSS/{suite-results,consistency,pareto,summary}` + `latest` symlink (atomic temp-link-rename).
  - [ ] Result files written via `roko_fs::atomic::atomic_write_json`.
  - [ ] Test `bench_compare_fails_on_regression` passes.
  - [ ] Commit message trailer: `tracker: PERF_21 done <sha>`.

---

## Cross-cutting hygiene checks (every batch)

These are not batch-specific but apply to the whole runner. A wave is
not "complete" until these are also true on `main`.

- [ ] No new `std::fs::*` call inside an async fn unless wrapped in
      `tokio::task::spawn_blocking`.
- [ ] No `std::sync::Mutex` held across `.await`.
- [ ] No `Arc::new(reqwest::Client::new())` outside the existing
      `SHARED_HTTP_CLIENT` static.
- [ ] No new `unwrap()` / `expect()` in non-test code (use `?` or
      `let-else`).
- [ ] All caches have a documented invalidation strategy (TTL, mtime,
      content-hash, or scope).
- [ ] All caches have a bounded size (`LruCache` capacity, hard cap, or
      scope-bound).

---

## How to mark items done

```bash
# After PERF_03 lands as commit 9f1c8a2:
sed -i '' 's|- \[ \] \*\*PERF_03\*\*|- [x] **PERF_03**|' tmp/runners/perf/ISSUE-TRACKER.md
git commit -m "tracker: PERF_03 done 9f1c8a2"

# Or just open the file and tick the box manually + bump the
# "Progress at a glance" header.
```
