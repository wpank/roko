# Perf Runner — File Inventory

A condensed crate map + edit-site cheat-sheet so each prompt does not
have to repeat the same orientation. Read this before any batch.

---

## Crates touched by this runner

| Crate | LOC | Owns |
|---|---|---|
| `roko-cli` | ~50 K | CLI entry, `run.rs` (3.6 K LOC), `orchestrate.rs` (22.8 K LOC), plan execution, dispatch |
| `roko-runtime` | ~10 K | `WorkflowEngine`, `EffectDriver`, `pipeline_state.rs`, event bus, `jsonl_logger`, soon `warm_dispatch_pool` |
| `roko-agent` | ~30 K | provider HTTP, `ModelCallService`, `MultiAgentPool`, safety contracts |
| `roko-gate` | ~10 K | `GateService`, `GatePipeline`, all rung-N gate impls, composition |
| `roko-compose` | ~8 K | `PromptAssemblyService`, enrichment pipeline (13 steps), conventions |
| `roko-fs` | ~3 K | `FileSubstrate` (engrams.jsonl), atomic writes |
| `roko-learn` | ~30 K | cascade router, episode logger, costs DB, efficiency signals |
| `roko-core` | ~12 K | `RokoConfig` schema, foundation traits (`ModelCaller`, `PromptAssembler`, `GateRunner`) |
| `roko-serve` | ~15 K | HTTP server, routes, dispatch endpoint |

---

## Hot-path edit sites for the perf runner

### Config loading (PERF_01)

```text
crates/roko-cli/src/run.rs
   381  crate::config::load_layered(workdir)
   392  roko_core::config::load_config(workdir)
  1827  roko_core::config::load_config(workdir)        ← inside dispatch_agent
  2397  roko_core::config::load_config(workdir)
  2715  roko_core::config::load_config(Path::new("."))  ← resolved_model fallback
crates/roko-cli/src/config.rs
        load_layered(...) -> Result<ResolvedConfig>
crates/roko-core/src/config/mod.rs
        load_config(workdir) -> Result<RokoConfig>
        RokoConfig::apply_process_env(&mut self)
crates/roko-core/src/config/provider.rs
        merge_global_providers(&mut RokoConfig)
```

### Learning runtime (PERF_02)

```text
crates/roko-cli/src/run.rs
  2630  let learn_root = workdir.join(".roko").join("learn");
  2638  LearningRuntime::open_under(...)              ← episode log path
  2643  LearningRuntime::open_under_with_models(...)  ← episode log path
        (also opens occur inside orchestrator dispatch path)
crates/roko-learn/src/runtime_feedback.rs
        LearningRuntime::open_under(path)
        LearningRuntime::open_under_with_models(path, models)
        LearningRuntime::record_completed_run(input)
        LearningRuntime::set_episode_completion_hook(closure)
```

### Safety contracts (PERF_03 — already cached)

```text
crates/roko-agent/src/safety/contract.rs
   34  static CONTRACT_CACHE: LazyLock<RwLock<HashMap<String, AgentContract>>>
  120  pub fn load_for_role(role) -> Result<Self, ContractLoadError>
  176  pub fn load_for_role_with_mode(role, mode) -> ...
crates/roko-agent/src/safety/contracts/*.yaml
        bundled per-role policies
```

### JSONL event logger (PERF_04)

```text
crates/roko-runtime/src/jsonl_logger.rs    ← 127 LOC, primary edit
   62  fn write_event(&self, event: &RuntimeEvent)
   72  let json = serde_json::to_string(&envelope)?;
   82  writeln!(w, "{json}")?;
   83  w.flush()?;                                    ← REMOVE
crates/roko-runtime/src/workflow_engine.rs
        WorkflowEngine::run(config) ← add explicit logger.flush() here
crates/roko-fs/src/file_substrate.rs::replay_log
        reference impl for "tolerate partial last line" pattern
```

### Substrate writes (PERF_05)

```text
crates/roko-fs/src/file_substrate.rs
  137  pub async fn put_batch(&self, signals: Vec<Engram>) -> Result<Vec<ContentHash>>
        (already optimised: serialize → single write → flush, dedup against index)
crates/roko-cli/src/run.rs
        existing put_batch sites: 1156, 1180, 1188, 1245, 2872, 2901
        single-put sites: rg "substrate\.put\(" crates/roko-cli/src/
```

### Prompt assembly (PERF_06)

```text
crates/roko-compose/src/prompt_assembly_service.rs   ← 1 050 LOC
   504  conventions_for_spec(spec, default)
   650  detect_workdir_conventions(workdir)
   669  collect_source_context(workdir)
   681  collect_source_context_from(dir, root, samples, listing)  ← std::fs::read_dir
crates/roko-compose/src/conventions.rs
        detect_conventions(cargo_toml, source_refs, file_refs) -> Conventions
crates/roko-runtime/src/effect_driver.rs
        EffectDriver::spawn_agent calls services.prompt_assembler.assemble(spec)
```

### Routing (PERF_07)

```text
crates/roko-cli/src/orchestrate.rs
   198  use roko_learn::cascade::load_efficiency_signals_sync
  6073  load_efficiency_signals_sync(...)
 14705  Some(self.learning.cascade_router())     ← dispatch entry
 14808  load_efficiency_signals_sync(...)         ← dispatch hot path
crates/roko-learn/src/cascade/mod.rs
        load_efficiency_signals_sync(path) -> Result<Vec<EfficiencySignal>>
crates/roko-learn/src/cascade_router.rs
        CascadeRouter::observe / apply_bias / explain_route / select_for_frequency_among
```

### Enrichment (PERF_08 — sequential by design)

```text
crates/roko-compose/src/enrichment/pipeline.rs
   385  pub async fn run_all(plan_base) -> Vec<StepOutcome>
   397  pub async fn run_steps(plan_base, steps) -> Vec<StepOutcome>
        ← INTENTIONALLY sequential; later steps consume earlier outputs
crates/roko-compose/src/enrichment/step.rs
   62  ALL_ORDERED: &[EnrichStep] = &[...]   (13 steps, dependency-ordered)
crates/roko-cli/src/orchestrate.rs
  2343  enrich_task_context_with_search(...)  ← PER-DISPATCH enrichment (parallelisable)
  8985  EnrichmentPipeline::new(...)          ← PLAN enrichment (must stay sequential)
```

### Warm pool (PERF_09 / PERF_10 / PERF_11)

```text
crates/roko-agent/src/multi_pool.rs           ← existing pool primitives, NOT wired
   17  WarmEntry { agent, spawned_at, reuse_policy }
   51  MultiAgentPool { active, warm, fallbacks, ... }
  110  pre_spawn_warm(role, count, factory)
  167  promote_warm(role) -> Option<AgentInstanceId>
crates/roko-agent/src/session.rs
        WarmReusePolicy { scope, max_reuses, ttl }
crates/roko-agent/src/provider/mod.rs
   88  static SHARED_HTTP_CLIENT: LazyLock<reqwest::Client>
  108  pub fn shared_http_client() -> reqwest::Client
crates/roko-agent/src/model_call_service.rs
  107  impl ModelCallService { /* the thing we cache */ }
 1280  let agent = create_agent_for_model(&self.config, attempt_model, opts)?
crates/roko-runtime/src/effect_driver.rs
   38  pub struct EffectServices { /* ADD warm_pool: Option<Arc<WarmDispatchPool>> */ }
   88  pub async fn spawn_agent(&self, role, prompt, ctx) -> PipelineInput
crates/roko-cli/src/run.rs
   426  fn build_workflow_effect_services(workdir, config, model_config, selection)
crates/roko-serve/src/lib.rs / runtime.rs
        serve startup; periodic eviction task lives here
```

### Gate pipeline (PERF_12 / PERF_13 / PERF_14)

```text
crates/roko-gate/src/gate_service.rs   ← 679 LOC; primary edit for all 3 gate batches
   88  pub fn ordered_gate_names(config: &GateConfig) -> Vec<String>
  120  fn should_skip_rung_adaptively(rung)
  234  async fn run_gates(&self, config: GateConfig) -> Result<GateReport>
crates/roko-gate/src/gate_pipeline.rs
        GatePipeline (sequential, short-circuit)
crates/roko-gate/src/composition.rs
        ParallelGate, VotingGate, FallbackGate
crates/roko-gate/src/adaptive_threshold.rs
        AdaptiveThresholds (existing skip mechanism — orthogonal to mode/source-hash)
crates/roko-gate/src/{compile,clippy_gate,test_gate,format_check_gate}.rs
        rung-N gate implementations (read-only context)
crates/roko-runtime/src/pipeline_state.rs
   97  pub struct WorkflowConfig { /* ADD gate_mode: GateMode */ }
crates/roko-core/src/foundation.rs
        GateConfig (the trait input; ADD gate_mode field there too)
```

### Git diff cache (PERF_15)

```text
crates/roko-cli/src/orchestrate.rs
 17767  gate_diff_for_plan(plan_id) -> Option<String>      (full diff)
 17895  build_review_prompt → "git diff --name-only HEAD"  (file list)
 18922  run_plan_verify_steps → "git diff --cached"        (different semantics; KEEP)
crates/roko-cli/src/lib.rs
        re-exports CLI modules; add git_diff_snapshot
```

### Speculative + parallel dispatch (PERF_16 / PERF_17)

```text
crates/roko-runtime/src/warm_dispatch_pool.rs   ← created by PERF_09
crates/roko-runtime/src/effect_driver.rs        ← PERF_16 hooks here
crates/roko-cli/src/dispatch/                   ← exists; PERF_17 adds parallel.rs
crates/roko-cli/src/dispatch/mod.rs
crates/roko-cli/src/dispatch/model_routing.rs
crates/roko-cli/src/orchestrate.rs              ← plan executor; PERF_17 wiring
```

### PGO (PERF_18)

```text
scripts/pgo-train.sh        ← NEW
scripts/pgo-build.sh        ← NEW
fixtures/pgo-workdir/       ← NEW (tiny rust workspace + roko.toml + plans/)
crates/roko-cli/benches/cli_overhead.rs ← NEW (criterion harness)
.github/workflows/release.yml ← MODIFY (add pgo-build job)
```

### HAL + bench (PERF_19 / PERF_20 / PERF_21)

```text
hal/roko_agent/{main.py,requirements.txt,tests/test_main.py}  ← NEW
hal/README.md                                                 ← NEW
.github/workflows/hal-bench.yml                               ← NEW
crates/roko-cli/src/output_format.rs    ← --output json producer
crates/roko-cli/src/bench.rs            ← SweBenchOptions, run_task_real
crates/roko-cli/src/commands/{mod.rs,bench_compare.rs (NEW)}
crates/roko-serve/src/bench.rs          ← BenchSuite / BenchTask / BenchResult
crates/roko-learn/src/costs_db.rs       ← cost wiring source
crates/roko-fs/src/atomic.rs            ← atomic_write_json
```

---

## Concurrency primitives in use

| Primitive | When to use |
|---|---|
| `std::sync::Mutex<T>` | Sync-only state; **NEVER hold across `.await`** |
| `parking_lot::Mutex<T>` | Same as above; faster, no poisoning. Same `.await` rule |
| `tokio::sync::Mutex<T>` | When the lock must be held across awaits (e.g., warm-pool slots) |
| `parking_lot::RwLock<T>` | Read-heavy caches with rare writes (contract cache, convention cache) |
| `tokio::sync::RwLock<T>` | Same as above but across awaits |
| `LazyLock<T>` | Process-wide single-init (shared HTTP client, contract cache) |
| `OnceLock<T>` | Same; preferred over `LazyLock` when init is fallible |
| `Arc<T>` | Sharing immutable or interior-mut state across tasks/threads |
| `lru::LruCache<K, V>` | Bounded caches with LRU eviction |
| `dashmap::DashMap<K, V>` | Concurrent map; reach for only after measuring contention |

---

## Where the existing perf bottleneck research lives

```text
tmp/solutions/perf/
  BENCHMARK-RESULTS.md         ← baselines; macro-bench recipe in §11.1
  BOTTLENECK-ANALYSIS.md       ← B01..B15 full catalogue
  OPTIMIZATION-PLAYBOOK.md     ← higher-level descriptions of fixes
  WARM-POOL-DESIGN.md          ← required reading for PERF_09..PERF_11
  HAL-AND-AGENT-BENCHMARKS.md  ← required reading for PERF_19
  HAL-BENCHMARK-INTEGRATION.md ← required reading for PERF_19
  implementation/              ← the 18 detailed plans
    00-INDEX.md
    01-shared-config-cache.md   ← source for PERF_01
    ...                         ← (see prompts for the per-batch mapping)
    18-bench-suite-extension.md ← source for PERF_20 + PERF_21
```

Each prompt cites its source plan; treat the plan as the canonical
reference if a prompt and a plan disagree.
