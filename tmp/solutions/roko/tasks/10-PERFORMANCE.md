# Task File 10: Performance Optimization

> Concrete, gate-verifiable tasks for the 14-optimization playbook, warm pool
> wiring, HAL benchmark integration, and regression CI. Each task targets a
> measured bottleneck (B01-B15) from the bottleneck analysis.
>
> Source plans:
> - `tmp/solutions/roko/impl/10-PERFORMANCE.md`
> - `tmp/solutions/roko/13-PERF-OPTIMIZATION-PLAYBOOK.md`
> - `tmp/solutions/roko/13-PERF-BOTTLENECK-ANALYSIS.md`
> - `tmp/solutions/roko/13-PERF-WARM-POOL-DESIGN.md`

---

## Overview

Roko's non-inference overhead is ~710ms per `roko run` (fast model, no gates).
With gates, it balloons to 3700-5200ms. Fifteen measured bottlenecks (B01-B15)
account for 1020-4330ms of addressable waste. The optimization strategy is:

1. Eliminate redundant work (config re-parse, LearningRuntime re-open, contract
   re-load, duplicate git diffs)
2. Batch I/O (substrate writes, feedback flush)
3. Cache stable data (conventions, routing decisions, source context)
4. Parallelize independent work (enrichment steps, gate rungs)
5. Pool reusable resources (warm dispatch slots)
6. Add fast-paths (express gate mode, source-hash guard)
7. Establish measurement infrastructure (tracing spans, criterion benchmarks,
   HAL integration, per-PR regression CI)

**Target after all phases:** <345ms overhead (no gates), <2000ms full pipeline.

---

## Anti-Patterns to Remove

### AP-1: Config parsed 4+ times per run
**Where:** `crates/roko-cli/src/run.rs` lines 423, 434, 1855, 2422, 2735
**What:** `load_layered()` / `load_config()` / `load_roko_config_models()` each
independently parse `roko.toml` from disk. Same file, same result, 4-5 times.
**Fix:** Load once at CLI entry, wrap in `Arc<RokoConfig>`, pass through.

### AP-2: LearningRuntime opened multiple times
**Where:** `crates/roko-cli/src/run.rs` line 2664 (`append_episode_log` opens
its own), `crates/roko-cli/src/orchestrate.rs` lines 4311, 4536, 4748
**What:** Each `open_under()` reads 3 JSON files + spawns distillation task.
Multiple opens per run waste 70-140ms.
**Fix:** Thread `LearningRuntime` as a parameter; never re-open within a run.

### AP-3: Safety contract loaded from embedded YAML on every tool dispatch
**Where:** `crates/roko-agent/src/safety/mod.rs` lines 864-887
(`contract_for_role()` calls `AgentContract::load_for_role()`)
**What:** A 10-tool turn = 10 redundant YAML asset loads for the same role.
**Fix:** `LazyLock<Mutex<HashMap<String, Arc<AgentContract>>>>` process-scoped cache.

### AP-4: Per-event flush in JsonlLogger
**Where:** `crates/roko-runtime/src/jsonl_logger.rs` line 81 (`w.flush()`)
**What:** Every `write_event()` call flushes the BufWriter. 20-30 events/run =
60-150ms of synchronous disk I/O. The BufWriter is already allocated but never
gets a chance to buffer because of the immediate flush.
**Fix:** Remove per-event `flush()`. Add explicit `flush()` method called at run end.

### AP-5: No tracing instrumentation on hot path
**Where:** `crates/roko-cli/src/run.rs` (no `#[instrument]` attributes found)
**What:** No spans on config load, agent construct, prompt assembly, gate run,
substrate write, or feedback flush. Cannot measure before/after for any
optimization without manual timing.
**Fix:** Add `#[tracing::instrument(skip_all, fields(phase = "..."))]` to 7 hot functions.

### AP-6: Git diff computed per gate rung, not cached
**Where:** `crates/roko-cli/src/orchestrate.rs` lines 17484-17537 (gate phase
spawns multiple git processes)
**What:** Each rung that needs diff data spawns its own `git diff` subprocess.
Multiple rungs = multiple redundant git processes.
**Fix:** Compute diff once per gate phase, store in `GatePhaseContext`, pass through.

### AP-7: BufWriter created with default 8-byte capacity
**Where:** `crates/roko-runtime/src/jsonl_logger.rs` line 55
(`BufWriter::new(file)`)
**What:** Default `BufWriter::new()` uses 8KB capacity, but the per-event flush
(AP-4) makes this irrelevant. Even after removing the flush, the default is fine
but explicitly sizing shows intent.
**Fix:** `BufWriter::with_capacity(8192, file)` after removing per-event flush.

### AP-8: No GateMode enum -- all tasks run full gate pipeline
**Where:** `crates/roko-runtime/src/pipeline_state.rs` -- `WorkflowConfig` has
`express()` template but no gate-level mode selection
**What:** Documentation changes, config updates, and research queries all run
the full compile/clippy/test pipeline (500-2000ms) even though they modify no
code.
**Fix:** Add `GateMode` enum (Full/Express/None/Auto), wire into gate service.

### AP-9: Enrichment steps run sequentially despite being independent
**Where:** `crates/roko-cli/src/orchestrate.rs` -- `EnrichmentPipeline` runs
steps from `ALL_ORDERED` array serially
**What:** File intel, knowledge, wave, and research enrichments have no
dependencies on each other but execute in series: sum(times) instead of
max(times).
**Fix:** `tokio::join!` for independent steps.

### AP-10: Warm pool infrastructure built but not wired
**Where:** `crates/roko-agent/src/multi_pool.rs` (`MultiAgentPool`, `WarmEntry`,
`WarmReusePolicy`), `crates/roko-agent/src/session.rs`
**What:** Full warm pool data structures exist with pre-spawned warm entries,
reuse policies, and concurrency limits. But `EffectDriver.spawn_agent()` always
constructs cold. `EffectServices` has no `warm_pool` field.
**Fix:** Create `WarmDispatchPool` in roko-runtime, wire into EffectServices.

### AP-11: BenchmarkRegressionGate is a stub
**Where:** `crates/roko-gate/src/benchmark_gate.rs` line 74 ("Stub: no baseline
infrastructure yet. Pass through.")
**What:** Gate always returns `Verdict::pass`. No baseline capture, no
comparison logic. Cannot detect performance regressions during gate verification.
**Fix:** Implement baseline capture/storage/comparison with configurable threshold.

### AP-12: No `--output json` on `roko run`
**Where:** `crates/roko-cli/src/run.rs` -- no OutputMode or JSON output support
**What:** HAL wrapper and automation scripts need structured output. Currently
only human-readable text output.
**Fix:** Add `--output json` flag with structured `RunOutputJson`.

### AP-13: No criterion benchmarks for runtime overhead
**Where:** `crates/roko-cli/benches/` -- directory does not exist
**What:** Cannot track regressions in config load, prompt assembly, substrate
write, or feedback flush times. No automated baseline comparison.
**Fix:** Create `benches/runtime_overhead.rs` with criterion benchmarks.

### AP-14: Efficiency cache exists but routing decisions are not cached
**Where:** `crates/roko-cli/src/orchestrate.rs` -- `EfficiencyCache` at line
2445 caches raw signals with 10s TTL, but each dispatch still re-scores all
candidate models and re-queries neuro store
**What:** For a 10-task plan, the cascade router computes identical routing
decisions 10 times because only the raw signals are cached, not the final
routing decision.
**Fix:** Add routing decision memoization keyed on (task_type, complexity, quality).

---

## Phase 0: Instrumentation Baseline

### Task 10.1 -- Add Tracing Spans to Hot-Path Functions

**Files:**
- `crates/roko-cli/src/run.rs`
- `crates/roko-compose/src/prompt_assembly_service.rs`
- `crates/roko-runtime/src/effect_driver.rs`
- `crates/roko-learn/src/runtime_feedback.rs`
- `crates/roko-fs/src/file_substrate.rs`
- `crates/roko-gate/src/gate_service.rs`
- `crates/roko-runtime/src/jsonl_logger.rs`

**What:** Add `#[tracing::instrument(skip_all, fields(phase = "..."))]` to seven
hot-path functions so every optimization can be measured before and after.

**Steps:**
1. `run.rs`: Instrument `resolve_workflow_model_selection()` with
   `fields(phase = "config_load")`
2. `runtime_feedback.rs`: Instrument `LearningRuntime::open_under()` with
   `fields(phase = "learning_open")`
3. `effect_driver.rs`: Instrument `EffectDriver::spawn_agent()` with
   `fields(phase = "agent_construct")`
4. `prompt_assembly_service.rs`: Instrument the `assemble()` impl on
   `PromptAssemblyService` with `fields(phase = "prompt_assemble")`
5. `file_substrate.rs`: Instrument `FileSubstrate::put()` (the `Store` impl at
   line 271) with `fields(phase = "substrate_write")`
6. `gate_service.rs`: Instrument `GateService::run_gates()` (the `GateRunner`
   impl at line 235) with `fields(phase = "gate_run")`
7. `jsonl_logger.rs`: Instrument `JsonlLogger::write_event()` (line 62) with
   `fields(phase = "feedback_flush")`

**Acceptance:**
- `RUST_LOG=roko=trace cargo run --release -p roko-cli -- config show` produces
  log output with phase spans and durations
- `cargo clippy --workspace --no-deps -- -D warnings` clean

**Depends on:** Nothing

---

### Task 10.2 -- Create Criterion Benchmark Harness

**Files:**
- `crates/roko-cli/benches/runtime_overhead.rs` (NEW)
- `crates/roko-cli/Cargo.toml`

**What:** Criterion benchmarks for the five heaviest non-inference functions:
config load, learning runtime open, prompt assembly, substrate write, feedback
flush.

**Steps:**
1. Add `criterion = { version = "0.5", features = ["html_reports"] }` to
   `[dev-dependencies]` in `crates/roko-cli/Cargo.toml`
2. Add `[[bench]] name = "runtime_overhead" harness = false` entry
3. Create `benches/runtime_overhead.rs` with benchmark groups:
   - `bench_config_load`: time `crate::config::load_layered()` on workspace
     `roko.toml`
   - `bench_learning_open`: time `LearningRuntime::open_under()` with existing
     `.roko/learn/` state (create temp dir with fixture data)
   - `bench_prompt_assembly`: time `PromptAssemblyService::assemble()` with a
     fixture prompt and workspace
   - `bench_substrate_write_10`: time 10 sequential `FileSubstrate::put()` calls
   - `bench_jsonl_flush_30`: time 30 `JsonlLogger::write_event()` calls
4. Each group reports mean, median, std deviation via criterion defaults

**Acceptance:**
- `cargo bench --bench runtime_overhead -p roko-cli` runs and produces timing
- Criterion's HTML report generates at `target/criterion/`
- Comparison with prior runs via `cargo bench` regression detection

**Depends on:** Nothing

---

## Phase 1: Config and Init Caching (B02, B03, B05, B13)

### Task 10.3 -- Shared Config Cache: Load Once, Arc Through

**Files:**
- `crates/roko-cli/src/run.rs` (primary)
- `crates/roko-cli/src/model_selection.rs`

**What:** Eliminate 4+ redundant `roko.toml` loads per `roko run`. Load once at
CLI entry, wrap in `Arc`, pass through the call chain.

**Steps:**
1. In `resolve_workflow_model_selection()` (line 420), this already loads both
   `load_layered()` and `load_config()`. Refactor so the caller loads once and
   passes the result in.
2. Change `dispatch_agent()` (line 1846) to accept config as a parameter instead
   of calling `load_config(workdir)` internally at line 1855
3. Change `append_episode_log()` (line 2545) to accept config -- remove its
   internal `load_roko_config_models(workdir)` call at line 2656
4. Remove standalone `load_config()` / `load_roko_config_models()` calls from
   functions that now receive config as parameter
5. Add `debug!("config loaded once, {} providers", ...)` at the single load site

**Acceptance:**
- `RUST_LOG=roko_cli=debug roko run "echo hello"` shows exactly ONE "config
  loaded" log line
- `cargo test -p roko-cli` passes
- `roko run --model <override> "test"` still correctly uses the override

**Depends on:** Task 10.1

---

### Task 10.4 -- LearningRuntime Single-Open

**Files:**
- `crates/roko-cli/src/run.rs`

**What:** Thread `LearningRuntime` through to `append_episode_log()` instead of
re-opening it. Saves ~70ms per run (3 file reads + JSON parse + distillation
spawn).

**Steps:**
1. In the main dispatch path, open `LearningRuntime::open_under()` once
2. Change `append_episode_log()` signature to accept `lr: &mut LearningRuntime`
3. Remove the `LearningRuntime::open_under()` call inside `append_episode_log()`
   at line 2663-2670
4. Thread the same instance from the dispatch caller (around line 1301) to
   `append_episode_log()`
5. Call `lr.flush()` / drop at run end to ensure persistence

**Acceptance:**
- `RUST_LOG=roko_learn=debug roko run "echo hello"` shows exactly ONE "opening
  learning runtime" log line
- `.roko/episodes.jsonl` still receives entries
- `.roko/learn/cascade-router.json` still updates

**Depends on:** Task 10.3 (config caching simplifies init path)

---

### Task 10.5 -- Safety Contract Caching

**Files:**
- `crates/roko-agent/src/safety/mod.rs`
- `crates/roko-agent/src/safety/contract.rs`

**What:** Cache `AgentContract` per role using a process-scoped
`LazyLock<Mutex<HashMap<String, Arc<AgentContract>>>>` so that repeated tool
dispatches within a turn do not re-load from embedded YAML assets.

**Steps:**
1. In `safety/mod.rs`, add a `static CONTRACT_CACHE: LazyLock<Mutex<HashMap<String, Arc<AgentContract>>>>`
2. Create `fn cached_contract_for_role(role: &str) -> Arc<AgentContract>` that
   checks the cache first, falls back to `AgentContract::load_for_role()` on miss
3. In `contract_for_role()` (line 864), replace the direct
   `AgentContract::load_for_role(role)` call with `cached_contract_for_role(role)`
4. Cache is process-scoped and never invalidated (contracts are immutable during
   a process lifetime; restarts clear the static)
5. Preserve the existing fallback logic: role overrides still checked first,
   `RestrictedFallback` mode preserved for unknown roles

**Acceptance:**
- Unit test: two `contract_for_role("implementer")` calls -- second is instant
  (no YAML parse)
- Unit test: different roles return different contracts
- `cargo test -p roko-agent` passes
- No change in external safety behavior

**Depends on:** Nothing

---

### Task 10.6 -- Lazy Event Serialization + Buffer Reuse

**Files:**
- `crates/roko-runtime/src/jsonl_logger.rs`

**What:** Use a thread-local buffer to avoid per-event `String` allocation in
`write_event()`. Serialize directly into a reusable `Vec<u8>` instead of
allocating a new `String` per event.

**Steps:**
1. Add `thread_local! { static BUF: RefCell<Vec<u8>> = RefCell::new(Vec::with_capacity(512)); }`
2. In `write_event()`, replace `serde_json::to_string(&envelope)` (line 72)
   with:
   ```
   BUF.with(|buf| {
       let mut buf = buf.borrow_mut();
       buf.clear();
       serde_json::to_writer(&mut *buf, &envelope)?;
       buf.push(b'\n');
       // write buf to file
   })
   ```
3. Remove the separate `writeln!(w, "{json}")` -- the buffer already has the
   newline
4. Verify `event_bus.rs` does not eagerly serialize before passing to consumers

**Acceptance:**
- `cargo test -p roko-runtime` passes (all existing logger tests)
- Events round-trip correctly (envelope schema unchanged)
- Benchmark: 30 sequential `write_event()` calls complete in <20ms

**Depends on:** Nothing

---

## Phase 2: Persistence Optimization (B10, B11)

### Task 10.7 -- Batch Substrate Writes

**Files:**
- `crates/roko-fs/src/file_substrate.rs`
- `crates/roko-cli/src/run.rs`

**What:** The `put_batch()` method already exists on `FileSubstrate` (line 137).
The task is to ensure the CLI `run` path uses it instead of sequential `put()`
calls, and to add crash-safety tests.

**Steps:**
1. Verify `put_batch()` at `file_substrate.rs` line 137 does single-write I/O
   (it currently collects lines into a String and does one write)
2. In `run.rs`, identify the post-dispatch signal persistence (around line
   924-1050 per the bottleneck analysis) and replace individual `substrate.put()`
   calls with `substrate.put_batch(signals)`
3. Add a crash-safety test: write valid signals then a truncated last line,
   verify the reader returns only complete lines
4. If `put_batch` already coalesces I/O, confirm with tracing that a single
   `substrate_write` span covers all signals

**Acceptance:**
- `cargo test -p roko-fs` passes including new crash-safety test
- Test: 10 signals via `put_batch()`, read back, all 10 present
- Test: partial last line is ignored by reader
- Tracing shows single `substrate_write` span for batch

**Depends on:** Task 10.1 (tracing spans)

---

### Task 10.8 -- Async Feedback Flush

**Files:**
- `crates/roko-runtime/src/jsonl_logger.rs`
- `crates/roko-runtime/src/workflow_engine.rs`

**What:** Remove per-event `w.flush()` from `write_event()`. Add explicit
`flush()` method. Call it at workflow completion.

**Steps:**
1. In `jsonl_logger.rs` line 81, remove `w.flush()?;` after `writeln!(w, "{json}")?;`
2. Change `BufWriter::new(file)` at line 55 to `BufWriter::with_capacity(8192, file)`
   for explicit sizing
3. Add public method:
   ```rust
   pub fn flush(&self) -> std::io::Result<()> {
       let mut writer = self.writer.lock().unwrap_or_else(|e| e.into_inner());
       if let Some(ref mut w) = *writer {
           w.flush()?;
       }
       Ok(())
   }
   ```
4. In `WorkflowEngine::run()` (at `workflow_engine.rs`), call the logger's
   `flush()` after the workflow completes, before returning the result
5. Verify `BufWriter` flushes on `Drop` as a safety net

**Acceptance:**
- `cargo test -p roko-runtime` passes
- Test: write 100 events, call `flush()`, read back all 100
- No data loss: `roko run` still produces events in `runtime-events.jsonl`
- Per-event write latency drops (no sync I/O per event)

**Depends on:** Task 10.6 (lazy serialization applied first)

---

## Phase 3: Prompt Assembly and Routing Caches (B06, B12, B14)

### Task 10.9 -- Workspace Convention Cache

**Files:**
- `crates/roko-compose/src/prompt_assembly_service.rs`

**What:** Cache `detect_workdir_conventions()` result with `Cargo.toml` mtime
invalidation. Conventions (language, build system, naming style) are stable
within a run.

**Steps:**
1. Add field `convention_cache: Mutex<Option<ConventionCacheEntry>>` to
   `PromptAssemblyService` struct (line 47)
2. Define `ConventionCacheEntry { workdir: PathBuf, conventions: String, mtime: SystemTime }`
3. In `assemble()`, before `detect_workdir_conventions()` at line 507:
   - Lock cache, check if entry exists for same workdir + matching
     `Cargo.toml` mtime
   - On hit: return cached conventions string
   - On miss: call `detect_workdir_conventions()`, store result
4. Use `std::fs::metadata(workdir.join("Cargo.toml")).and_then(|m| m.modified())`
   for mtime comparison
5. For non-Rust workspaces: try `package.json`, `go.mod`, `pyproject.toml` as
   cache key file

**Acceptance:**
- Unit test: two `assemble()` for same workdir with unchanged Cargo.toml --
  second returns in <5ms
- Unit test: touch Cargo.toml between calls -- cache invalidated, fresh detect
- `cargo test -p roko-compose` passes

**Depends on:** Nothing

---

### Task 10.10 -- Source Context Collection Cache

**Files:**
- `crates/roko-compose/src/prompt_assembly_service.rs`

**What:** Cache `collect_source_context()` / `collect_source_context_from()`
with `src/` directory mtime invalidation. Convert blocking `std::fs` to
`tokio::fs` in the hot path.

**Steps:**
1. Add `source_context_cache: Mutex<Option<SourceContextCacheEntry>>` to
   `PromptAssemblyService`
2. Define `SourceContextCacheEntry { workdir: PathBuf, context: (Vec<String>, Vec<String>), dir_mtime: SystemTime }`
3. Before calling `collect_source_context()` at line 513, check cache using
   `src/` directory mtime
4. Convert `std::fs::read_dir()` and `std::fs::read_to_string()` in
   `collect_source_context_from()` (line 681) to `tokio::fs` equivalents.
   Note: `collect_source_context_from` is called synchronously from
   `detect_workdir_conventions`, so this may require making it async or using
   `spawn_blocking` for the recursive walk
5. Cap source file reads at 12 files / 64KB total (preserve existing limits)

**Acceptance:**
- Unit test: repeated `assemble()` uses cached source context
- `cargo test -p roko-compose` passes
- No blocking `std::fs` in the hot prompt assembly path (or wrapped in
  `spawn_blocking`)

**Depends on:** Task 10.9 (convention cache pattern established)

---

### Task 10.11 -- Routing Decision Cache

**Files:**
- `crates/roko-cli/src/orchestrate.rs`

**What:** The `EfficiencyCache` at line 2445 already caches raw efficiency
signals with 10s TTL. Extend with routing decision memoization so sequential
plan tasks skip re-scoring all model candidates.

**Steps:**
1. Add a `routing_decisions: HashMap<u64, (String, Instant)>` field to
   `PlanRunner` (or adjacent to `EfficiencyCache`)
2. Compute routing cache key as `hash(task_type, complexity_tier, (recent_quality * 100) as u32)`
3. Before the cascade routing logic (around line 5971 and 14663 where
   `efficiency_cache.get()` is called): check if a routing decision exists for
   the computed key within TTL
4. On cache hit: use cached model, skip scoring loop
5. On cache miss: run normal cascade routing, store decision
6. Invalidate on new efficiency event write (set cached_at to the past)
7. TTL: 30 seconds (shorter than efficiency cache because routing decisions
   incorporate external state like neuro store queries)

**Acceptance:**
- 5-task plan: `efficiency.jsonl` read at most twice (initial + one TTL expiry)
- Routing decisions for identical task profiles are consistent within TTL window
- `cargo test --workspace` passes

**Depends on:** Nothing (efficiency cache already exists)

---

## Phase 4: Parallel Enrichment (B07)

### Task 10.12 -- Parallelize Enrichment Pipeline

**Files:**
- `crates/roko-cli/src/orchestrate.rs`

**What:** Run independent enrichment steps concurrently using `tokio::join!`
instead of the sequential `ALL_ORDERED` iteration.

**Steps:**
1. Locate the `EnrichmentPipeline::new()` usage at orchestrate.rs line 8888 and
   the `selected_enrichment_steps()` call at line 1829
2. Identify which steps are independent (file intel, knowledge, wave, research
   are all independent; tasks step may depend on file intel)
3. Group independent steps and run them with `tokio::join!` or
   `futures::future::join_all()`
4. Steps that are CPU-bound (not I/O-bound): wrap in
   `tokio::task::spawn_blocking()`
5. Handle errors individually -- each enrichment step's failure should not abort
   the others; collect results and report which steps failed
6. Preserve the existing `StepSelector` complexity-based filtering

**Acceptance:**
- Tracing shows enrichment time = `max(step_times)` not `sum(step_times)`
- `cargo test --workspace` passes
- Enriched prompt content is unchanged (diff before/after parallelization)

**Depends on:** Nothing

---

## Phase 5: Gate Pipeline Optimization (B08, B09)

### Task 10.13 -- Define GateMode Enum

**Files:**
- `crates/roko-runtime/src/pipeline_state.rs`

**What:** Add a `GateMode` enum to `WorkflowConfig` for gate-level mode
selection (Full/Express/None/Auto).

**Steps:**
1. Define:
   ```rust
   #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
   pub enum GateMode {
       #[default]
       Full,     // all configured gates
       Express,  // lightweight only (diff, fmt)
       None,     // skip all gates
       Auto,     // detect from changed file types
   }
   ```
2. Add `pub gate_mode: GateMode` to `WorkflowConfig` with `Default::default()`
   (Full -- backwards compatible)
3. Add `pub fn with_gate_mode(mut self, mode: GateMode) -> Self`
4. Derive `clap::ValueEnum` so it can be used as CLI argument
5. Implement `Display` for log output

**Acceptance:**
- `cargo test -p roko-runtime` passes
- `GateMode::default()` returns `Full`
- Serializes/deserializes in JSON and TOML correctly

**Depends on:** Nothing

---

### Task 10.14 -- Wire Express Gate Mode Into Gate Service

**Files:**
- `crates/roko-gate/src/gate_service.rs`
- `crates/roko-runtime/src/effect_driver.rs`

**What:** Implement gate filtering based on `GateMode`. Express mode skips
compile, clippy, and test. Auto mode detects from changed file types.

**Steps:**
1. Add `fn filter_gates_for_mode(gates: &[GateEntry], mode: GateMode, workdir: &Path) -> Vec<GateEntry>` or equivalent filtering in `run_gates()`
2. For `Express`: retain only rungs 3 (diff) and 4 (fmt) per the existing
   `rung_for_name()` mapping at line 392
3. For `None`: return empty -- all gates skipped
4. For `Auto`:
   - Run `git diff --stat HEAD`
   - If any `.rs`/`.ts`/`.py` files modified -> Full
   - If only `.toml`/`.json`/`.yaml` -> Express
   - If only `.md`/`.txt` -> None
5. In `run_gates()` (line 235), apply filter before iterating over gates
6. Log which gates are skipped and the resolved mode

**Acceptance:**
- `GateMode::Express` skips compile/clippy/test (verify via trace log)
- `GateMode::Auto` correctly classifies code vs config vs docs changes
- `GateMode::Full` unchanged from current behavior
- `cargo test -p roko-gate` passes

**Depends on:** Task 10.13

---

### Task 10.15 -- Add `--gates` CLI Flag

**Files:**
- `crates/roko-cli/src/main.rs`
- `crates/roko-cli/src/run.rs`

**What:** Expose `GateMode` as `--gates <MODE>` on `roko run`.

**Steps:**
1. Add `--gates <MODE>` argument to the `run` subcommand, using `GateMode`'s
   `ValueEnum` derive
2. Default to `auto` for `roko run` (interactive runs benefit from detection)
3. Default to `full` for `roko plan run` (plan execution is thorough)
4. Pass resolved `GateMode` through to `WorkflowConfig::with_gate_mode()`
5. Log resolved gate mode at run start

**Acceptance:**
- `roko run --gates none "echo hello"` skips all gates
- `roko run --gates express "echo hello"` runs only diff + fmt
- `roko run "echo hello"` defaults to auto-detection
- `roko --help` shows `--gates` with value options
- `cargo test -p roko-cli` passes

**Depends on:** Task 10.14

---

### Task 10.16 -- Git Diff Cache Per Gate Phase

**Files:**
- `crates/roko-cli/src/orchestrate.rs`

**What:** Compute git diff once per gate phase, store in `GatePhaseContext`,
pass to all gates that need it.

**Steps:**
1. Define:
   ```rust
   struct GatePhaseContext {
       diff_stat: String,
       diff_full: String,
       modified_files: Vec<String>,
       computed_at: Instant,
   }
   ```
2. Add `async fn compute(workdir: &Path) -> Self` that runs `git diff --stat HEAD`
   and `git diff HEAD` in parallel via `tokio::join!`
3. Parse modified file paths from diff stat output
4. Replace the gate-phase git subprocess spawns (around lines 17484-17537,
   17666, 18700) with reads from `GatePhaseContext`
5. Pass `GatePhaseContext` through the gate dispatch path

**Acceptance:**
- During gate phase, at most ONE `git diff --stat` and ONE `git diff HEAD`
  subprocess spawned (verify via tracing)
- Gate verdicts unchanged from before
- `cargo test --workspace` passes

**Depends on:** Nothing

---

### Task 10.17 -- Source Hash Gate Guard

**Files:**
- `crates/roko-gate/src/gate_service.rs`

**What:** Skip the compile gate if modified source files have not changed since
the last successful compile (hash-based guard).

**Steps:**
1. Add `fn hash_modified_sources(modified_files: &[String]) -> u64` that hashes
   concatenated mtimes + sizes of all `.rs` files using `DefaultHasher`
2. Store last successful compile hash in `.roko/state/last-compile-hash` (single
   line file with hex-encoded u64)
3. In `run_gates()`, before running the compile rung:
   - Compute current hash from modified files list
   - If matches stored hash: skip compile, log "compile skipped (unchanged)"
   - If differs: run compile, update stored hash on pass
4. Never skip if last compile failed (always re-check after failure)

**Acceptance:**
- Two runs on unchanged codebase: second skips compile gate
- Modify a `.rs` file: compile gate runs again
- `cargo test -p roko-gate` passes

**Depends on:** Task 10.16 (GatePhaseContext provides modified_files)

---

### Task 10.18 -- Parallel Gate Rungs

**Files:**
- `crates/roko-gate/src/gate_service.rs`

**What:** Run independent gate rungs concurrently. Compile (0) + diff (3) +
fmt (4) are independent and can run in parallel. Clippy (1) and test (2) depend
on compile passing.

**Steps:**
1. In `run_gates()`, group rungs by dependency:
   - Parallel set 1: {0 compile, 3 diff, 4 fmt}
   - Sequential set 2 (if compile passed): {1 clippy}
   - Sequential set 3 (if compile passed): {2 test}
   - Sequential set 4: {5 custom/shell, 6 judge}
2. Execute parallel sets with `futures::future::join_all()` or `tokio::join!`
3. If any gate in set 1 fails, still report all set 1 results but skip sets 2-4
4. Preserve existing short-circuit and adaptive threshold skip logic

**Acceptance:**
- Wall-clock gate phase time reduced when compile and fmt run in parallel
- Gate verdicts identical to sequential execution
- If compile fails, clippy and test are still skipped
- `cargo test -p roko-gate` passes

**Depends on:** Nothing (can be done independently)

---

## Phase 6: Warm Dispatch Pool (B04, B15)

### Task 10.19 -- Create WarmDispatchPool

**Files:**
- `crates/roko-runtime/src/warm_dispatch_pool.rs` (NEW)

**What:** Three-tier warm dispatch pool: hot (in-flight), warm (pre-built idle),
cold (on-demand construct). RAII slot guards. Pool metrics.

**Steps:**
1. Create `warm_dispatch_pool.rs` with:
   - `WarmPoolConfig`: `max_warm_slots`, `max_active`, `idle_timeout`,
     `pre_warm`, `pre_warm_targets`
   - `WarmSlot`: `provider`, `model`, `caller: Arc<dyn ModelCaller>`,
     `created_at`, `last_used`, `dispatches_served`, `state: SlotState`
   - `SlotState`: `Idle`, `Active { run_id, since }`, `Draining`
   - `WarmPoolMetrics`: `total_dispatches`, `warm_hits`, `cold_misses`,
     `evictions`, `peak_active`, `avg_acquire_us`
   - `WarmDispatchPool`: `config`, `slots: Mutex<Vec<WarmSlot>>`,
     `metrics: Mutex<WarmPoolMetrics>`, `factory`
   - `WarmSlotGuard<'a>`: `pool`, `slot_idx`, `caller`
2. `acquire()`: tier 1 (exact match) -> tier 2 (same provider) -> tier 3 (cold)
3. `pre_warm()`: create idle slots for configured targets
4. `evict_idle()`: remove slots past `idle_timeout`
5. `release()`: return slot to idle state
6. Document: `Drop` for `WarmSlotGuard` cannot be async; callers must call
   `pool.release(idx)` explicitly

**Acceptance:**
- Unit test: acquire from empty -> cold miss, slot created
- Unit test: acquire, release, acquire again -> warm hit
- Unit test: same provider different model -> warm hit (provider reuse)
- Unit test: evict_idle removes expired slots
- Unit test: metrics track hits/misses/evictions accurately
- `cargo test -p roko-runtime` passes

**Depends on:** Nothing

---

### Task 10.20 -- Export WarmDispatchPool Module

**Files:**
- `crates/roko-runtime/src/lib.rs`

**What:** Add `warm_dispatch_pool` module to crate, export key types.

**Steps:**
1. Add `pub mod warm_dispatch_pool;` to `lib.rs`
2. Add re-exports:
   `pub use warm_dispatch_pool::{WarmDispatchPool, WarmPoolConfig, WarmPoolMetrics, WarmSlotGuard};`

**Acceptance:**
- `cargo build -p roko-runtime` succeeds
- `cargo doc -p roko-runtime` generates docs for new module

**Depends on:** Task 10.19

---

### Task 10.21 -- Wire WarmDispatchPool Into EffectDriver

**Files:**
- `crates/roko-runtime/src/effect_driver.rs`

**What:** Add `warm_pool: Option<Arc<WarmDispatchPool>>` to `EffectServices`.
`spawn_agent()` tries pool first, falls back to cold construction.

**Steps:**
1. Add `pub warm_pool: Option<Arc<WarmDispatchPool>>` to `EffectServices` (after
   `affect_policy` at line 49)
2. In `spawn_agent()` (line 87), before constructing the model call:
   ```rust
   let caller = if let Some(ref pool) = self.services.warm_pool {
       if let Some(guard) = pool.acquire(&provider, &model).await {
           guard.caller
       } else {
           Arc::clone(&self.services.model_caller)
       }
   } else {
       Arc::clone(&self.services.model_caller)
   };
   ```
3. After dispatch completes, release slot back to pool
4. Default `warm_pool` to `None` -- update all `EffectServices` construction
   sites to include `warm_pool: None`

**Acceptance:**
- With `warm_pool = None`: identical to current behavior
- With `warm_pool = Some(pool)`: second dispatch reuses warm slot
- `cargo test -p roko-runtime` passes (existing tests use `warm_pool: None`)

**Depends on:** Task 10.20

---

### Task 10.22 -- Wire WarmDispatchPool Into WorkflowEngine

**Files:**
- `crates/roko-runtime/src/workflow_engine.rs`

**What:** Pool lifecycle: pre-warm on workflow start, evict on completion, log
metrics.

**Steps:**
1. In `WorkflowEngine::run()`, if `self.driver.services.warm_pool.is_some()`:
   - Call `pool.pre_warm().await` before main workflow loop
   - Call `pool.evict_idle().await` after workflow completes
2. Log pool metrics at workflow end:
   ```rust
   info!(warm_hits = m.warm_hits, cold_misses = m.cold_misses,
         avg_acquire_us = m.avg_acquire_us, "warm pool stats");
   ```

**Acceptance:**
- Pool metrics logged after workflow run
- Pre-warm creates slots for configured targets
- Evict removes idle slots past timeout
- `cargo test -p roko-runtime` passes

**Depends on:** Task 10.21

---

### Task 10.23 -- Wire WarmDispatchPool Into `roko run`

**Files:**
- `crates/roko-cli/src/run.rs`

**What:** Construct pool in CLI `run_once()` path, pass to `EffectServices`.
For CLI one-shot: no pre-warm (first request warms HTTP client; second reuses).

**Steps:**
1. Build `WarmPoolConfig::default()` (no pre-warm for CLI)
2. Create `model_caller_factory` closure from existing `create_agent_for_model`
   or `ModelCallService` constructor
3. Construct `WarmDispatchPool::new(config, Arc::new(factory))`
4. Set `effect_services.warm_pool = Some(Arc::new(pool))`
5. Pool lives for run duration, dropped on completion

**Acceptance:**
- Standard workflow (2 agent calls): warm slot reused for second call (verify
  via pool metrics in trace log)
- Express workflow (1 call): works correctly
- `cargo test -p roko-cli` passes

**Depends on:** Task 10.22

---

### Task 10.24 -- Wire WarmDispatchPool Into `roko serve`

**Files:**
- `crates/roko-serve/src/embedded.rs`

**What:** Long-running server benefits most. Pre-warm on startup, periodic
eviction via background task.

**Steps:**
1. Read `WarmPoolConfig` from `roko.toml` `[conductor.warm_pool]` section
   (or defaults)
2. Construct `WarmDispatchPool` with config, pre-warm on startup
3. Spawn periodic eviction task: `tokio::spawn` with 60s interval calling
   `pool.evict_idle().await`
4. Share pool with route handlers that dispatch agent work

**Acceptance:**
- `roko serve` starts with warm slots pre-created (startup log)
- After 5+ minutes idle, warm slots evicted
- Multiple concurrent API requests reuse warm slots
- `cargo test -p roko-serve` passes

**Depends on:** Task 10.23 (pool construction pattern established)

---

### Task 10.25 -- WarmPoolConfig in `roko.toml` Schema

**Files:**
- `crates/roko-core/src/config/mod.rs`
- `roko.toml`

**What:** Add `[conductor.warm_pool]` config section to the TOML schema.

**Steps:**
1. Add `WarmPoolConfig` struct to config:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct WarmPoolTomlConfig {
       pub enabled: bool,
       pub max_warm_slots: usize,
       pub max_active: usize,
       pub idle_timeout_secs: u64,
       pub pre_warm_on_serve: bool,
       pub pre_warm_providers: Vec<String>,
       pub pre_warm_models: Vec<String>,
   }
   ```
2. Add `pub warm_pool: Option<WarmPoolTomlConfig>` to `[conductor]` section
3. Implement `Default`: enabled=true, max_warm_slots=4, max_active=8,
   idle_timeout_secs=300, pre_warm_on_serve=true
4. Validate: max_warm_slots <= 16, idle_timeout_secs >= 30
5. Wire so `roko config show` displays warm pool config

**Acceptance:**
- `roko config show` includes warm pool config
- `roko config validate` accepts valid config
- Missing `[conductor.warm_pool]` uses defaults (backwards compatible)
- `cargo test -p roko-core` passes

**Depends on:** Nothing (schema can be added independently)

---

## Phase 7: Speculative Execution

### Task 10.26 -- Speculative Pre-Warming in Workflow Engine

**Files:**
- `crates/roko-runtime/src/workflow_engine.rs`
- `crates/roko-runtime/src/warm_dispatch_pool.rs`

**What:** While implementer runs, speculatively pre-warm reviewer's model caller
for instant acquisition on implementation completion.

**Steps:**
1. After spawning implementer in workflow loop:
   - Check if template includes review phase (standard/full)
   - If yes and pool has idle capacity: `tokio::spawn(pool.pre_warm_for(...))`
2. Add `pub async fn pre_warm_for(&self, provider: &str, model: &str)` to
   `WarmDispatchPool` -- creates a single warm slot for the given pair
3. When reviewer dispatched, `pool.acquire()` finds pre-warmed slot (warm hit)
4. If implementation fails (no review): slot sits idle, evicted after timeout

**Acceptance:**
- Standard workflow: reviewer acquisition <5ms (warm hit from speculation)
- Express workflow: no speculation (no reviewer phase)
- Failed implementation: pre-warmed slot eventually evicted, not leaked
- Pool metrics show speculative warm hits

**Depends on:** Task 10.22

---

## Phase 8: Profile-Guided Optimization

### Task 10.27 -- PGO Build Script

**Files:**
- `scripts/pgo-build.sh` (NEW)

**What:** Script that builds instrumented binary, runs workloads, merges
profiles, rebuilds with PGO data.

**Steps:**
1. Create `scripts/pgo-build.sh`:
   ```bash
   #!/usr/bin/env bash
   set -euo pipefail
   PGO_DIR="${1:-/tmp/roko-pgo-data}"
   rm -rf "$PGO_DIR" && mkdir -p "$PGO_DIR"
   RUSTFLAGS="-Cprofile-generate=$PGO_DIR" cargo build --release -p roko-cli
   ./target/release/roko config show 2>/dev/null || true
   ./target/release/roko plan validate plans/ 2>/dev/null || true
   ./target/release/roko status 2>/dev/null || true
   llvm-profdata merge -o "$PGO_DIR/merged.profdata" "$PGO_DIR"
   RUSTFLAGS="-Cprofile-use=$PGO_DIR/merged.profdata" cargo build --release -p roko-cli
   echo "PGO build complete: target/release/roko"
   ```
2. `chmod +x scripts/pgo-build.sh`
3. Document prerequisite: `rustup component add llvm-tools-preview`

**Acceptance:**
- `./scripts/pgo-build.sh` completes without error
- Resulting binary comparable in size to non-PGO release build
- Benchmark: 5-15% improvement on `bench_config_load` and `bench_prompt_assembly`

**Depends on:** Task 10.2 (benchmark harness for measurement)

---

### Task 10.28 -- PGO CI Integration

**Files:**
- `.github/workflows/pgo-build.yml` (NEW)

**What:** Add PGO build step to release workflow. Published binaries are
profile-optimized.

**Steps:**
1. Create `.github/workflows/pgo-build.yml` triggered on release tags:
   - Install `llvm-tools-preview`
   - Build instrumented binary
   - Run representative workloads (config show, plan validate, status)
   - Merge profiles
   - Rebuild with PGO data
   - Upload PGO binary as release artifact
2. Keep standard non-PGO build as fallback
3. Compare PGO vs non-PGO binary sizes

**Acceptance:**
- CI produces PGO binary on release tags
- PGO failure does not block release (fallback to standard build)
- Release notes indicate whether binary is PGO-optimized

**Depends on:** Task 10.27

---

## Phase 9: HAL Benchmark Integration

### Task 10.29 -- Create HAL Agent Wrapper

**Files:**
- `hal/roko_agent/main.py` (NEW)
- `hal/roko_agent/requirements.txt` (NEW)

**What:** Python wrapper exposing roko as HAL-compatible agent for standardized
benchmark evaluation (SWE-bench, etc.).

**Steps:**
1. Create `hal/roko_agent/main.py` with `run(task, **kwargs)` function per HAL's
   agent protocol
2. Wrapper logic:
   - Accept task dict: `instance_id`, `prompt`/`problem_statement`, `repo`,
     `base_commit`, `hints`
   - Clone/checkout task repo into temp dir
   - Run `roko init` + `roko run --model <model> --output json "<prompt>"`
   - Capture `git diff HEAD` as `model_patch`
   - Return: `model_patch`, `cost`, `tokens`, `duration_s`, `model`, `exit_code`
3. Create `hal/roko_agent/requirements.txt` (stdlib only, no deps)
4. Support kwargs: `model_name`, `workflow`, `gates`, `timeout`, `roko_binary`

**Acceptance:**
- `hal-eval` can invoke the wrapper and get a valid result dict
- `model_patch` is a valid unified diff
- Timeout respected (process killed after limit)

**Depends on:** Task 10.30 (`--output json` required)

---

### Task 10.30 -- Add `--output json` to `roko run`

**Files:**
- `crates/roko-cli/src/run.rs`
- `crates/roko-cli/src/main.rs`

**What:** Structured JSON output for automation and HAL wrapper.

**Steps:**
1. Add `--output <FORMAT>` to `run` subcommand: `text` (default) or `json`
2. Define:
   ```rust
   #[derive(Serialize)]
   struct RunOutputJson {
       success: bool,
       model: String,
       cost_usd: f64,
       total_tokens: u64,
       input_tokens: u64,
       output_tokens: u64,
       duration_ms: u64,
       gate_results: Vec<GateResultJson>,
       files_changed: Vec<String>,
       error: Option<String>,
   }
   ```
3. At end of `run_once()`, if `--output json`: serialize and print to stdout
4. Suppress non-JSON output (progress bars, status) when `--output json`

**Acceptance:**
- `roko run --output json "echo hello" | jq .success` outputs `true`/`false`
- JSON includes all fields with correct types
- `--output text` behavior unchanged
- `cargo test -p roko-cli` passes

**Depends on:** Nothing

---

### Task 10.31 -- Performance Benchmark Suite Definition

**Files:**
- `.roko/bench/suites/perf.json` (NEW)

**What:** Benchmark suite with 5 tasks measuring non-inference overhead across
workflow templates.

**Steps:**
1. Create `perf.json` with 5 benchmark tasks:
   - `perf-001`: minimal prompt, express workflow, no gates (baseline)
   - `perf-002`: single tool call (file write), express, no gates
   - `perf-003`: code edit, express workflow, express gates
   - `perf-004`: code gen, standard workflow, full gates
   - `perf-005`: multi-step, full workflow, express gates
2. All use fast models (gpt-4.1-nano) to isolate framework overhead
3. Each specifies model, workflow template, gate mode

**Acceptance:**
- Suite definition validates against `BenchSuite` schema
- Results include per-task wall-clock, inference, and overhead time

**Depends on:** Task 10.15 (`--gates` flag for gate mode), Task 10.30 (`--output json`)

---

### Task 10.32 -- Quality Benchmark Suite Definition

**Files:**
- `.roko/bench/suites/quality.json` (NEW)

**What:** Quality regression suite with 5 tasks testing code generation, bug
fixing, and refactoring quality.

**Steps:**
1. Create `quality.json` with 5 tasks:
   - `qual-001`: fix compilation error (type mismatch)
   - `qual-002`: reverse string with Unicode handling
   - `qual-003`: refactor loops to iterators
   - `qual-004`: add error handling
   - `qual-005`: implement a trait
2. Each includes `expected_gates` for automated scoring
3. Self-contained (no external deps)

**Acceptance:**
- Suite validates against `BenchSuite` schema
- Each task has clear pass/fail via gate verdicts
- Tasks cover different agent capabilities

**Depends on:** Nothing

---

## Phase 10: Regression Testing and CI

### Task 10.33 -- Benchmark Comparison Command

**Files:**
- `crates/roko-cli/src/bench.rs`
- `crates/roko-cli/src/main.rs` (add subcommand)

**What:** `roko bench compare <baseline> <current>` subcommand that compares
two benchmark result files and reports regressions.

**Steps:**
1. Add `bench compare` subcommand to CLI
2. Load two JSON result files
3. Per-task comparison:
   - Wall-clock: flag if current > baseline * 1.2 (20% regression)
   - Overhead: flag if non-inference time increased
   - Gate pass rate: flag if any passing gate now fails
4. Output comparison table to stdout
5. Exit code 1 if any regression exceeds threshold
6. `--threshold <percent>` flag for custom tolerance

**Acceptance:**
- `roko bench compare a.json b.json` outputs comparison table
- Exit 0 if no regressions, 1 if threshold exceeded
- `--threshold 50` allows up to 50% before failing
- `cargo test -p roko-cli` passes

**Depends on:** Nothing

---

### Task 10.34 -- Per-PR Performance Check CI Workflow

**Files:**
- `.github/workflows/perf-check.yml` (NEW)

**What:** CI workflow running perf benchmark on every PR, comparing against
main branch baseline.

**Steps:**
1. Create workflow triggered on `pull_request`:
   - Build release binary
   - Run perf benchmark suite
   - Download main branch baseline from artifact cache
   - Run `roko bench compare`
   - Post comparison as PR comment or fail check
2. Cache main branch results as GitHub Actions artifact
3. On merge to main: update cached baseline

**Acceptance:**
- PRs get perf regression check reporting overhead changes
- 20%+ regression fails check (configurable)
- Main baseline updates on merge

**Depends on:** Task 10.33

---

### Task 10.35 -- Nightly HAL Benchmark CI Workflow

**Files:**
- `.github/workflows/hal-bench.yml` (NEW)

**What:** Nightly workflow running roko through HAL's SWE-bench mini evaluation
and tracking quality over time.

**Steps:**
1. Create workflow on `schedule` (daily 2AM UTC) + `workflow_dispatch`:
   - Build release binary
   - Install `hal-harness` via pip
   - Run `hal-eval` on SWE-bench mini (50 tasks)
   - Upload results as artifacts
   - Compare with previous nightly
2. `max_concurrent: 5` for cost control
3. Default model: `gpt-4.1-mini`

**Acceptance:**
- Nightly runs and produces HAL results
- Results include per-task pass/fail, cost, duration
- Budget does not exceed ~$20/night

**Depends on:** Task 10.29 (HAL wrapper)

---

### Task 10.36 -- Multi-Run Consistency Mode for Bench Harness

**Files:**
- `crates/roko-cli/src/bench.rs`

**What:** Add `trials: usize` to benchmark options. When >1, run each task K
times and compute consistency metrics (pass rate, K-trial consistency, token
variance).

**Steps:**
1. Add `pub trials: usize` to `SweBenchOptions`, default 1
2. When `trials > 1`:
   - Run each task K times with different seeds
   - Collect pass/fail per trial
   - Compute: pass rate, K-trial consistency, token usage CoV
3. Include metrics in result output
4. `trials = 1`: unchanged behavior

**Acceptance:**
- `trials = 1`: identical behavior
- `trials = 5`: each task runs 5x, results include per-trial outcomes
- A task passing 3/5 gets 60% pass rate

**Depends on:** Nothing

---

### Task 10.37 -- Wire Cost Tracking Into Bench Results

**Files:**
- `crates/roko-cli/src/bench.rs`

**What:** The `cost_usd` field in `BenchResult` is currently always 0.0.
Connect to the learning subsystem's cost tracking.

**Steps:**
1. After each bench task, read cost from `ModelCallResponse` or efficiency event
2. Sum costs across all model calls for total `cost_usd`
3. For local models (Ollama): estimate from token counts, or leave 0.0
4. Set `result.cost_usd = total_cost`

**Acceptance:**
- API-backed models: non-zero `cost_usd` in results
- Local models: `cost_usd = 0.0`
- `cargo test --workspace` passes

**Depends on:** Nothing

---

### Task 10.38 -- Pareto Frontier Analysis in Bench Results

**Files:**
- `crates/roko-cli/src/bench.rs` (or `crates/roko-serve/src/bench.rs`)

**What:** Compute cost-quality Pareto frontier across models.

**Steps:**
1. Add `pub fn pareto_frontier(results: &[BenchRunResult]) -> Vec<ParetoPoint>`
2. `ParetoPoint { model, pass_rate, avg_cost, avg_latency_ms }`
3. Keep only non-dominated points (no other point has both higher pass rate AND
   lower cost)
4. Sort by cost ascending
5. Include in benchmark suite output

**Acceptance:**
- 5 models: frontier contains 2-4 points (not all 5)
- Dominated models excluded
- Sorted by cost ascending
- `cargo test --workspace` passes

**Depends on:** Task 10.37

---

## Phase 11: Batch Inference and Connection Reuse

### Task 10.39 -- Verify SHARED_HTTP_CLIENT Connection Reuse

**Files:**
- `crates/roko-agent/src/provider/mod.rs`

**What:** Add logging and tests confirming connection reuse. The
`SHARED_HTTP_CLIENT` static at line 93 already exists with `pool_max_idle_per_host(10)`,
`pool_idle_timeout(90s)`, `tcp_keepalive(30s)`.

**Steps:**
1. Add `tracing::debug!("SHARED_HTTP_CLIENT initialized")` in the `LazyLock`
   init closure
2. Add test: two requests to same mock server, verify single TCP connection
3. Verify current config is optimal (10 idle, 90s timeout, 30s keepalive)
4. Consider: for `roko serve`, increase `pool_idle_timeout` to 300s

**Acceptance:**
- Test confirms connection reuse within idle timeout
- No unnecessary TLS handshakes for sequential same-provider requests
- Log shows SHARED_HTTP_CLIENT initialized exactly once per process

**Depends on:** Nothing

---

### Task 10.40 -- Parallel Inference for Independent Plan Tasks

**Files:**
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-orchestrator/src/dag.rs`

**What:** Ensure DAG executor dispatches independent tasks concurrently, using
the warm pool for connection reuse.

**Steps:**
1. Verify existing DAG executor identifies independent tasks (no unmet deps)
2. If independent tasks dispatched sequentially: change to `tokio::spawn` per
   ready task + `futures::future::join_all()`
3. Limit concurrency to `config.conductor.max_concurrent_tasks` (default 3)
4. Each concurrent task acquires from warm pool independently
5. Verify concurrent substrate writes don't corrupt data (mutex in FileSubstrate)

**Acceptance:**
- 3 independent tasks dispatch concurrently (trace log)
- Wall clock < sum of individual task times
- No data corruption from concurrent substrate writes
- `cargo test --workspace` passes

**Depends on:** Task 10.23 (warm pool in CLI)

---

### Task 10.41 -- Batch Inference Collector for Plan Execution

**Files:**
- `crates/roko-agent/src/batch.rs` (NEW)
- `crates/roko-agent/src/lib.rs`

**What:** `BatchCollector` that accumulates inference requests from concurrent
plan tasks and dispatches them in parallel, sharing connection resources.

**Steps:**
1. Create `batch.rs` with:
   - `BatchCollector { pending, batch_window: Duration, max_batch_size: usize }`
   - `submit(request) -> Result<ModelCallResponse>`: queue + wait for batch flush
   - `flush()`: dispatch all pending via `futures::future::join_all()`
2. Auto-flush when `pending.len() >= max_batch_size` or `batch_window` elapses
3. Each request gets individual response via `oneshot::channel`
4. Default: `batch_window = 50ms`, `max_batch_size = 10`
5. Export from `lib.rs`

**Acceptance:**
- 5 requests submitted within batch window: dispatched concurrently
- Each request gets individual response
- Partial batch flushes on timeout
- `cargo test -p roko-agent` passes

**Depends on:** Nothing

---

## Phase 12: Final Integration and Validation

### Task 10.42 -- End-to-End Performance Validation Script

**Files:**
- `scripts/perf-validate.sh` (NEW)

**What:** Script running before/after measurements across model/template/gate
combinations.

**Steps:**
1. Create `scripts/perf-validate.sh`:
   - Iterate models (gpt-4.1-nano, gpt-4.1-mini)
   - Iterate templates (express, standard)
   - Iterate gates (none, express, full)
   - Run `roko run --model M --workflow-template T --gates G --output json "Reply with only hello"`
   - Capture `/usr/bin/time -l` output for wall clock + peak RSS
   - Store results in `.roko/bench/perf-YYYYMMDD/`
2. `chmod +x scripts/perf-validate.sh`
3. Parse timing output for wall clock, peak RSS, syscall count

**Acceptance:**
- Script runs all 12 combinations (2 x 2 x 3)
- Each produces timing data and JSON output
- Results directory contains 12+ result files

**Depends on:** Task 10.15 (`--gates` flag), Task 10.30 (`--output json`)

---

### Task 10.43 -- Warm Pool Metrics Endpoint in `roko serve`

**Files:**
- `crates/roko-serve/src/routes/status/mod.rs`

**What:** `/api/status/warm-pool` endpoint exposing pool metrics for monitoring.

**Steps:**
1. Add route handler:
   `async fn warm_pool_status(State(state)) -> Json<WarmPoolMetrics>`
2. Read from `state.warm_pool.metrics().await`
3. Return: `total_dispatches`, `warm_hits`, `cold_misses`, `evictions`,
   `peak_active`, `avg_acquire_us`, `current_slots`, `idle_slots`
4. Register in status router

**Acceptance:**
- `curl localhost:6677/api/status/warm-pool` returns pool metrics JSON
- Metrics update after agent dispatches
- Returns 200 with default/empty metrics if pool not configured

**Depends on:** Task 10.24

---

### Task 10.44 -- Fill BenchmarkRegressionGate Implementation

**Files:**
- `crates/roko-gate/src/benchmark_gate.rs`

**What:** Replace stub with baseline capture, storage, and comparison logic.
Currently `verify()` always returns `Verdict::pass` (line 74: "Stub: no baseline
infrastructure yet").

**Steps:**
1. Implement baseline capture:
   - After successful benchmark, store timing in
     `.roko/state/bench-baselines/<gate-name>.json`
   - Format: `{ task_id, wall_ms, overhead_ms, tokens, timestamp }`
2. Comparison logic in `verify()`:
   - Load baseline for current task
   - If current > baseline * (1 + threshold_pct/100): `Verdict::fail`
   - If no baseline: pass and capture baseline (first run)
3. Never skip re-check after previous failure

**Acceptance:**
- First run: passes, creates baseline file
- Second run (same perf): passes
- Third run (injected 30% slowdown): fails with regression message
- `cargo test -p roko-gate` passes

**Depends on:** Nothing

---

### Task 10.45 -- Perf Metrics Panel in TUI Dashboard

**Files:**
- `crates/roko-cli/src/tui/` (relevant tab module)

**What:** Performance metrics panel in TUI showing warm pool stats, recent run
overhead, optimization state.

**Steps:**
1. Identify appropriate TUI tab (status or metrics)
2. Add "Performance" section:
   - Warm pool: hits/misses/evictions
   - Recent run overhead: config load, agent construct, gate, persistence times
   - Optimization state: which caches active (config, convention, routing)
3. Read from:
   - `.roko/bench/perf-latest.json`
   - Warm pool metrics (if serve mode)
   - Runtime event log for per-phase timing
4. Refresh on file change via existing `notify::RecommendedWatcher`

**Acceptance:**
- `roko dashboard` shows performance section with timing data
- Metrics update when benchmark completes
- No crash if metrics files missing (graceful "no data")

**Depends on:** Task 10.24 (pool wired), Task 10.1 (tracing spans)

---

## Dependency Graph

```
10.1 (tracing spans) ────────── all phases use tracing for validation
10.2 (benchmark harness)
10.3 (config cache) ──────────── 10.4 (learning single-open)
10.5 (contract cache)
10.6 (lazy serialization) ────── 10.8 (async flush, apply serialization first)
10.7 (batch substrate)
10.9 (convention cache) ──────── 10.10 (source context cache)
10.11 (routing cache)
10.12 (parallel enrichment)
10.13 (GateMode enum) ────────── 10.14 (gate filtering) ── 10.15 (--gates CLI)
10.16 (diff cache) ───────────── 10.17 (source hash guard)
10.18 (parallel rungs)
10.19 (pool struct) ──────────── 10.20 (export) ── 10.21 (EffectDriver) ── 10.22 (WorkflowEngine)
10.22 ─────────────────────────── 10.23 (CLI) ── 10.24 (serve) ── 10.26 (speculation)
10.25 (pool config)
10.27 (PGO script) ──────────── 10.28 (PGO CI)
10.29 (HAL wrapper) ← 10.30 (--output json)
10.30 (--output json)
10.15 + 10.30 ────────────────── 10.31 (perf suite) + 10.42 (validation script)
10.33 (bench compare) ────────── 10.34 (per-PR CI)
10.29 ────────────────────────── 10.35 (nightly HAL CI)
10.37 (cost tracking) ────────── 10.38 (Pareto analysis)
10.23 ────────────────────────── 10.40 (parallel plan tasks)
10.24 ────────────────────────── 10.43 (warm pool metrics endpoint)
10.1 + 10.24 ─────────────────── 10.45 (TUI perf panel)
```

---

## Summary

| Phase | Tasks | Effort | Cumulative Savings |
|-------|-------|--------|--------------------|
| 0: Instrumentation | 10.1-10.2 | 3h | Baseline for measurement |
| 1: Config/Init | 10.3-10.6 | 5h | ~135ms |
| 2: Persistence | 10.7-10.8 | 4h | ~230ms |
| 3: Caches | 10.9-10.11 | 7h | ~480ms |
| 4: Enrichment | 10.12 | 2h | ~600ms |
| 5: Gates | 10.13-10.18 | 10h | ~1600-3400ms |
| 6: Warm Pool | 10.19-10.25 | 12h | ~1650-3450ms |
| 7: Speculation | 10.26 | 3h | ~1680-3480ms |
| 8: PGO | 10.27-10.28 | 4h | +5-15% on CPU-bound |
| 9: HAL | 10.29-10.32 | 8h | External validation |
| 10: Regression CI | 10.33-10.38 | 10h | Regression detection |
| 11: Batch/Parallel | 10.39-10.41 | 6h | Plan execution speedup |
| 12: Integration | 10.42-10.45 | 6h | Observability + monitoring |
| **Total** | **45 tasks** | **~80h** | **1680-3480ms + 5-15% PGO** |

### Targets

| Scenario | Current | Target | Reduction |
|----------|---------|--------|-----------|
| Express + fast US model, no gates | ~710ms overhead | ~345ms | 51% |
| Standard + fast model, express gates | ~4700-6200ms | ~2000ms | 57-68% |
| 10-task plan, 3-wide parallel | ~100s | ~55s | 45% |
