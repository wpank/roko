# Implementation Plan 10: Performance Optimization

> Concrete task list for all 14 playbook optimizations, warm pool implementation,
> HAL benchmark integration, and regression testing infrastructure. Each task
> includes exact file paths, acceptance criteria verifiable by the gate pipeline,
> and dependency ordering. Line numbers are approximate and should be confirmed
> at execution time.
>
> Reference documents:
> - `tmp/solutions/roko/13-PERF-BENCHMARK-RESULTS.md` -- measured baselines
> - `tmp/solutions/roko/13-PERF-BOTTLENECK-ANALYSIS.md` -- 15 bottlenecks ranked
> - `tmp/solutions/roko/13-PERF-WARM-POOL-DESIGN.md` -- three-tier pool architecture
> - `tmp/solutions/roko/13-PERF-OPTIMIZATION-PLAYBOOK.md` -- 14 optimization recipes
> - `tmp/solutions/roko/13-PERF-HAL-AND-AGENT-BENCHMARKS.md` -- benchmark landscape
> - `tmp/solutions/roko/13-PERF-HAL-BENCHMARK-INTEGRATION.md` -- HAL integration plan

---

## Phase 0: Instrumentation Baseline (prerequisite for all other phases)

### Task 10.1: Add Tracing Spans to Hot-Path Functions

**File**: `crates/roko-cli/src/run.rs`
**Also**: `crates/roko-compose/src/prompt_assembly_service.rs`,
`crates/roko-runtime/src/effect_driver.rs`
**What**: Instrument the critical path with `tracing::instrument` spans so every
optimization can be measured before and after.

**Steps**:
1. Add `#[tracing::instrument(skip_all, fields(phase = "config_load"))]` to every
   `load_config()` / `load_layered()` call site in `run.rs`
2. Add `#[tracing::instrument(skip_all, fields(phase = "learning_open"))]` to
   `LearningRuntime::open_under()` in `crates/roko-learn/src/runtime_feedback.rs`
3. Add `#[tracing::instrument(skip_all, fields(phase = "agent_construct"))]` to
   `create_agent_for_model()` in `crates/roko-agent/src/model_call_service.rs`
4. Add `#[tracing::instrument(skip_all, fields(phase = "prompt_assemble"))]` to
   `PromptAssemblyService::assemble()` in `crates/roko-compose/src/prompt_assembly_service.rs`
5. Add `#[tracing::instrument(skip_all, fields(phase = "substrate_write"))]` to
   `FileSubstrate::put()` in `crates/roko-fs/src/`
6. Add `#[tracing::instrument(skip_all, fields(phase = "gate_run"))]` to
   `run_gates()` in `crates/roko-gate/src/gate_service.rs`
7. Add `#[tracing::instrument(skip_all, fields(phase = "feedback_flush"))]` to
   `JsonlLogger::write_event()` in `crates/roko-runtime/src/jsonl_logger.rs`

**Acceptance criteria**:
- Run `RUST_LOG=roko=trace cargo run --release -p roko-cli -- run --model gpt-4.1-nano --gates none "echo hello"`
- Verify log output contains spans for all 7 phases with durations
- `cargo clippy --workspace --no-deps -- -D warnings` passes clean

---

### Task 10.2: Create Performance Benchmark Harness

**File**: `crates/roko-cli/benches/runtime_overhead.rs` (NEW)
**Also**: `crates/roko-cli/Cargo.toml`
**What**: Create a criterion-based benchmark that measures roko's non-inference
overhead by mocking the model caller and timing everything else.

**Steps**:
1. Add `criterion` to `[dev-dependencies]` in `crates/roko-cli/Cargo.toml`
2. Add `[[bench]]` entry: `name = "runtime_overhead"`, `harness = false`
3. Create `benches/runtime_overhead.rs` with benchmarks:
   - `bench_config_load`: Time `load_layered()` on the workspace `roko.toml`
   - `bench_learning_open`: Time `LearningRuntime::open_under()` with existing state
   - `bench_prompt_assembly`: Time `PromptAssemblyService::assemble()` with cached conventions
   - `bench_substrate_write`: Time 10 sequential `substrate.put()` calls
   - `bench_jsonl_flush`: Time 30 `JsonlLogger::write_event()` calls
4. Each benchmark group should report both single-iteration and throughput

**Acceptance criteria**:
- `cargo bench --bench runtime_overhead -p roko-cli` runs and produces timing output
- Benchmark reports include mean, median, and standard deviation
- Results can be compared across runs via criterion's built-in regression detection

---

## Phase 1: Config and Init Caching (B02, B03, B05, B13)

### Task 10.3: Shared Config Cache -- Load Once, Arc Through

**File**: `crates/roko-cli/src/run.rs`
**Also**: `crates/roko-cli/src/model_selection.rs`, `crates/roko-cli/src/orchestrate.rs`
**What**: Eliminate 4+ redundant `roko.toml` loads per `roko run` by loading once
at CLI entry and passing `Arc<RokoConfig>` through the call chain.
**Bottleneck**: B02 (10-50ms saved)

**Steps**:
1. In the top-level `run_once()` (or equivalent entry in `run.rs`), call
   `load_layered(&workdir)` exactly once, wrap in `Arc::new()`
2. Change `dispatch_agent()` signature to accept `config: &Arc<RokoConfig>` instead
   of calling `load_config()` internally
3. Change `append_episode_log()` signature to accept `config: &Arc<RokoConfig>`
4. Change `load_roko_config_models()` callers to use the shared `config` reference
5. Remove all internal `load_config()` / `load_layered()` calls from functions
   that now receive config as a parameter
6. Add a `debug!("config loaded, {} providers configured", ...)` log line at the
   single load site for verification

**Acceptance criteria**:
- Run `RUST_LOG=roko_cli=debug roko run "echo hello"` and verify exactly ONE
  "config loaded" log line (not 4)
- `cargo test --workspace` passes
- `roko run --model kimi-k2-6 "echo hello"` still correctly routes to moonshot
  (model override not clobbered by caching)

---

### Task 10.4: LearningRuntime Single-Open

**File**: `crates/roko-cli/src/run.rs`
**What**: Thread the `LearningRuntime` instance through to `append_episode_log()`
instead of opening it twice per run.
**Bottleneck**: B03 (70-100ms saved)

**Steps**:
1. In the main dispatch path of `run.rs`, open `LearningRuntime::open_under()` once
2. Change `append_episode_log()` to accept `lr: &LearningRuntime` parameter
3. Remove the internal `LearningRuntime::open_under()` call inside `append_episode_log()`
4. Ensure the same `LearningRuntime` instance is used for both dispatch-time
   recording and post-dispatch episode logging
5. At run end, call `lr.flush()` once (if such a method exists) to ensure persistence

**Acceptance criteria**:
- Run `RUST_LOG=roko_learn=debug roko run "echo hello"` and verify exactly ONE
  "opening learning runtime" (or equivalent) log line
- `.roko/episodes.jsonl` still receives entries after a run
- `.roko/learn/cascade-router.json` still updates after a run
- `cargo bench --bench runtime_overhead` shows reduced `bench_learning_open` time

**Dependencies**: Task 10.3 (config caching simplifies the LearningRuntime init path)

---

### Task 10.5: Safety Contract Caching

**File**: `crates/roko-agent/src/safety/` (contract loading module)
**What**: Cache safety contracts per role using `LazyLock<Mutex<HashMap<String, Arc<AgentContract>>>>`
so that repeated tool dispatches within a turn do not re-read YAML from disk.
**Bottleneck**: B05 (10-50ms saved per multi-tool turn)

**Steps**:
1. Create a `ContractCache` struct with `get_or_load(role: &str) -> Arc<AgentContract>`
2. Use `LazyLock<Mutex<HashMap<String, Arc<AgentContract>>>>` as the backing store
3. In `get_or_load()`, check the map first; on miss, call `load_contract_for_role()`
   and insert into the map
4. Replace all direct `load_contract_for_role()` calls in `ToolDispatcher` with
   `CONTRACT_CACHE.get_or_load(role)`
5. The cache is process-scoped (static). Stale entries are acceptable because
   contracts are immutable during a process lifetime; restarts clear the cache

**Acceptance criteria**:
- Unit test: two `get_or_load("implementer")` calls -- verify file I/O happens once
  (mock filesystem or count calls)
- Unit test: `get_or_load("implementer")` and `get_or_load("reviewer")` return
  different contracts
- `cargo test -p roko-agent` passes
- No change in external behavior (agents still respect contracts)

---

### Task 10.6: Lazy Event Serialization

**File**: `crates/roko-runtime/src/jsonl_logger.rs`
**Also**: `crates/roko-runtime/src/event_bus.rs` (if emit path serializes eagerly)
**What**: Use a thread-local buffer to avoid per-event String allocation during
serialization, and ensure serialization only happens in the logger consumer.
**Bottleneck**: B13 (15-25ms saved)

**Steps**:
1. In `JsonlLogger::write_event()`, replace the current `serde_json::to_string()`
   with `serde_json::to_writer()` writing directly into a thread-local `Vec<u8>`
2. Use `thread_local! { static BUF: RefCell<Vec<u8>> = RefCell::new(Vec::with_capacity(512)); }`
3. Clear the buffer at the start of each `write_event()` call
4. After serialization, push a `\n` byte and write the entire buffer to the file
5. Verify that `event_bus.rs` does not eagerly serialize events before passing to
   consumers -- if it does, remove the eager serialization

**Acceptance criteria**:
- `cargo test -p roko-runtime` passes (including the existing `writes_events_to_file` test)
- `runtime_event_envelopes_round_trip_as_json` contract test still passes
- `jsonl_logger_does_not_serialize_events_as_debug_strings` contract test still passes
- Benchmark: 30 sequential `write_event()` calls complete in <20ms (down from ~30-40ms)

---

## Phase 2: Persistence Optimization (B10, B11)

### Task 10.7: Batch Substrate Writes

**File**: `crates/roko-fs/src/` (FileSubstrate implementation)
**Also**: `crates/roko-cli/src/run.rs`
**What**: Add a `batch_put()` method to `FileSubstrate` that serializes and writes
multiple signals in a single I/O operation.
**Bottleneck**: B10 (60ms saved)

**Steps**:
1. Add method `pub fn batch_put(&self, signals: &[Engram]) -> Result<()>` to `FileSubstrate`
2. Implementation: pre-allocate `String::with_capacity(signals.len() * 256)`,
   serialize each signal, append `\n`, then do a single `write_all()` + `flush()`
3. Add private helper `fn append_raw(&self, data: &str) -> Result<()>` that opens
   the file in append mode, writes, and flushes once
4. In `run.rs`, collect all signals produced during a run into a `Vec<Engram>` and
   call `substrate.batch_put(&signals)` instead of 10+ individual `substrate.put()` calls
5. Add a test that verifies partial-line crash safety: write a truncated line and
   confirm the reader ignores it

**Acceptance criteria**:
- `cargo test -p roko-fs` passes, including new batch_put test
- Test: write 10 signals via `batch_put()`, read back, verify all 10 are present
- Test: write 10 signals then a truncated 11th line, reader returns exactly 10
- Benchmark: 10 signals via `batch_put()` completes in <10ms (down from ~80ms)

**Dependencies**: Task 10.1 (tracing spans for substrate writes)

---

### Task 10.8: Async Feedback Flush

**File**: `crates/roko-runtime/src/jsonl_logger.rs`
**What**: Remove per-event `w.flush()` from `JsonlLogger::write_event()`. Add an
explicit `flush()` method called at run completion.
**Bottleneck**: B11 (30-50ms saved)

**Steps**:
1. In `JsonlLogger::write_event()` at line ~81, remove the `w.flush()?;` call
   after `writeln!(w, "{json}")?;`. The `BufWriter` (already used) will buffer
   writes and flush when its internal 8KB buffer fills or on drop.
2. Increase `BufWriter` capacity: change `BufWriter::new(file)` to
   `BufWriter::with_capacity(8192, file)` in `ensure_writer()`
3. Add a public `pub fn flush(&self) -> std::io::Result<()>` method that acquires
   the mutex and calls `w.flush()` on the inner writer
4. In `WorkflowEngine::run()` (or equivalent run-completion path), call
   `jsonl_logger.flush()` to ensure all events are persisted before exit
5. Verify that `BufWriter`'s `Drop` implementation flushes on process exit as a
   safety net

**Acceptance criteria**:
- `cargo test -p roko-runtime` passes
- Test: write 100 events without explicit flush, call `flush()`, read back all 100
- Test: verify data survives normal process exit (BufWriter flushes on Drop)
- No data loss in `roko run` (events still appear in `runtime-events.jsonl`)

---

## Phase 3: Prompt Assembly and Routing Caches (B06, B12, B14)

### Task 10.9: Workspace Convention Cache

**File**: `crates/roko-compose/src/prompt_assembly_service.rs`
**What**: Cache the result of `detect_workdir_conventions()` with `Cargo.toml`
mtime-based invalidation so that repeated prompt assemblies within a run skip
directory walking and file reads.
**Bottleneck**: B14 (30-100ms saved)

**Steps**:
1. Add a `Mutex<Option<ConventionCache>>` field to `PromptAssemblyService`
2. Define `ConventionCache { workdir: PathBuf, conventions: String, mtime: SystemTime }`
3. In `assemble()`, before calling `detect_workdir_conventions()`:
   a. Lock the cache mutex
   b. If cache entry exists for same `workdir` and `Cargo.toml` mtime matches, return cached
   c. Otherwise, call `detect_workdir_conventions()`, store result in cache
4. Use `std::fs::metadata(workdir.join("Cargo.toml")).and_then(|m| m.modified())`
   for mtime comparison
5. If no `Cargo.toml` exists (non-Rust project), key on `package.json`, `go.mod`,
   or `pyproject.toml` mtime instead

**Acceptance criteria**:
- Unit test: two `assemble()` calls for same workdir with unchanged Cargo.toml --
  verify second call returns in <5ms
- Unit test: modify Cargo.toml mtime between calls -- verify cache is invalidated
- `cargo test -p roko-compose` passes

---

### Task 10.10: Source Context Collection Cache

**File**: `crates/roko-compose/src/prompt_assembly_service.rs`
**What**: Cache the output of `collect_source_context()` / `collect_source_context_from()`
with directory mtime invalidation, and convert blocking `std::fs` calls to `tokio::fs`.
**Bottleneck**: B12 (50-200ms saved)

**Steps**:
1. Add a `Mutex<Option<SourceContextCache>>` field to `PromptAssemblyService`
2. Define `SourceContextCache { workdir: PathBuf, context: String, dir_mtime: SystemTime }`
3. Before calling `collect_source_context_from()`, check cache validity using
   the `src/` directory's mtime
4. Convert `std::fs::read_dir()` and `std::fs::read_to_string()` in
   `collect_source_context_from()` to `tokio::fs::read_dir()` and
   `tokio::fs::read_to_string()` to avoid blocking the Tokio runtime
5. Cap source file reads at 12 files (existing behavior) and 64KB total to prevent
   bloat on large workspaces

**Acceptance criteria**:
- Unit test: repeated `assemble()` calls use cached source context (measure timing)
- `cargo test -p roko-compose` passes
- No blocking `std::fs` calls remain in the hot prompt assembly path

**Dependencies**: Task 10.9 (convention cache pattern established first)

---

### Task 10.11: Routing Decision Cache

**File**: `crates/roko-cli/src/orchestrate.rs`
**What**: Memoize parsed efficiency signals and routing decisions with TTL-based
invalidation so that sequential plan tasks do not re-read `efficiency.jsonl` on
every dispatch.
**Bottleneck**: B06 (150-300ms saved)

**Steps**:
1. Define `RoutingCache` struct with:
   - `efficiency_signals: Option<(Vec<EfficiencySignal>, Instant)>` -- cached signals + timestamp
   - `routing_decisions: HashMap<u64, (String, Instant)>` -- hash(task_type, complexity) -> (model, cached_at)
   - `ttl: Duration` -- default 10 seconds
2. Add `fn get_or_load_efficiency_signals(&mut self) -> &[EfficiencySignal]` that
   checks TTL and returns cached or freshly loaded signals
3. Add `fn get_routing_decision(&self, key: u64) -> Option<&str>` that returns
   cached model choice if within TTL
4. Add `fn put_routing_decision(&mut self, key: u64, model: String)` to store
   new routing decisions
5. Compute cache key as `hash(task_type, complexity_tier, (recent_quality * 100) as u32)`
6. Wire into the cascade routing path in `orchestrate.rs` where
   `load_efficiency_signals_sync()` is currently called on every dispatch
7. Invalidate the signal cache when a new efficiency event is written (set
   `loaded_at` to the past)

**Acceptance criteria**:
- Run a 5-task plan and verify `efficiency.jsonl` is read at most twice
  (once for initial load, once if TTL expires)
- `cargo test --workspace` passes
- Routing decisions for identical task profiles are consistent within the TTL window

---

## Phase 4: Parallel Enrichment (B07)

### Task 10.12: Parallelize Enrichment Pipeline

**File**: `crates/roko-cli/src/orchestrate.rs`
**What**: Run independent enrichment steps concurrently using `tokio::join!`
instead of sequentially.
**Bottleneck**: B07 (100-300ms saved)

**Steps**:
1. Identify the enrichment steps in `orchestrate.rs` near the `EnrichmentPipeline` usage:
   - File intelligence (reads source files for context)
   - Knowledge context (queries neuro store)
   - Wave context (recent execution history)
   - Research context (if available)
2. Verify that none of these steps depend on the output of another
3. Replace sequential execution:
   ```rust
   // Before:
   let file_intel = enrich_file_intel(&task).await?;
   let knowledge = enrich_knowledge(&task, &store).await?;
   let wave = enrich_wave(&task).await?;
   let research = enrich_research(&task).await?;
   ```
   With parallel execution:
   ```rust
   let (file_intel, knowledge, wave, research) = tokio::join!(
       enrich_file_intel(&task),
       enrich_knowledge(&task, &store),
       enrich_wave(&task),
       enrich_research(&task),
   );
   ```
4. Handle errors: each result should be unwrapped individually with context about
   which enrichment step failed
5. If any step is CPU-bound (not I/O-bound), wrap it in `tokio::task::spawn_blocking()`

**Acceptance criteria**:
- Enrichment time is `max(step_times)` not `sum(step_times)` -- verify with tracing
- `cargo test --workspace` passes
- No change in enrichment output quality (diff the enriched prompts before/after)

---

## Phase 5: Gate Pipeline Optimization (B08, B09)

### Task 10.13: Define GateMode Enum

**File**: `crates/roko-runtime/src/pipeline_state.rs`
**What**: Add a `GateMode` enum to `WorkflowConfig` so workflows can select between
full, express, auto, and none gate modes.
**Bottleneck**: B08 (prerequisite for express gate mode)

**Steps**:
1. Define the enum:
   ```rust
   #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
   pub enum GateMode {
       #[default]
       Full,     // Run all configured gates
       Express,  // Run only lightweight gates (diff, fmt). Skip compile/test/clippy
       None,     // Skip all gates
       Auto,     // Auto-detect based on changed files
   }
   ```
2. Add `pub gate_mode: GateMode` to `WorkflowConfig` with `Default::default()` (Full)
3. Add builder method `pub fn with_gate_mode(mut self, mode: GateMode) -> Self`
4. Add `impl std::fmt::Display for GateMode` for logging
5. Make `GateMode` derive `clap::ValueEnum` so it can be used as a CLI argument

**Acceptance criteria**:
- `cargo test -p roko-runtime` passes
- `GateMode::default()` returns `Full` (backwards compatible)
- `GateMode` serializes/deserializes correctly in JSON and TOML

---

### Task 10.14: Wire Express Gate Mode Into Gate Service

**File**: `crates/roko-gate/src/gate_service.rs`
**Also**: `crates/roko-runtime/src/effect_driver.rs`
**What**: Implement gate filtering based on `GateMode`: express mode skips
compile, clippy, and test rungs; auto mode detects based on changed file types.
**Bottleneck**: B08 (500-2000ms saved for applicable tasks)

**Steps**:
1. Add a `fn filter_gates_for_mode(gates: &[GateEntry], mode: GateMode, workdir: &Path) -> Vec<GateEntry>` function
2. For `GateMode::Express`: retain only rungs 3 (diff) and 4 (fmt)
3. For `GateMode::None`: return empty vec
4. For `GateMode::Auto`: check `git diff --stat HEAD` output:
   - If any `.rs`, `.ts`, `.py` files modified -> `Full`
   - If only `.toml`, `.json`, `.yaml` files modified -> `Express`
   - If only `.md`, `.txt` files modified -> `None`
5. In `run_gates()`, apply `filter_gates_for_mode()` before iterating over gates
6. Log which gates are being skipped and why

**Acceptance criteria**:
- `GateMode::Express` skips compile, clippy, and test gates (verify via trace log)
- `GateMode::Auto` correctly detects code vs. config vs. docs changes
- `GateMode::Full` behavior is unchanged from current (regression safe)
- `cargo test -p roko-gate` passes

**Dependencies**: Task 10.13 (GateMode enum defined)

---

### Task 10.15: Add --gates CLI Flag

**File**: `crates/roko-cli/src/main.rs`
**Also**: `crates/roko-cli/src/run.rs`
**What**: Expose `GateMode` as a `--gates` CLI flag on `roko run` so users can
choose full/express/none/auto from the command line.

**Steps**:
1. Add `--gates <MODE>` argument to the `run` subcommand using `clap::ValueEnum`
2. Default to `auto` (not `full`) for `roko run` -- most interactive runs benefit
   from auto-detection
3. Default to `full` for `roko plan run` -- plan execution should be thorough
4. Pass the `GateMode` through to `WorkflowConfig::with_gate_mode()`
5. Log the resolved gate mode at the start of each run

**Acceptance criteria**:
- `roko run --gates none "echo hello"` skips all gates
- `roko run --gates express "echo hello"` runs only diff + fmt
- `roko run "echo hello"` defaults to auto-detection
- `roko --help` shows the `--gates` flag with value options
- `cargo test -p roko-cli` passes

**Dependencies**: Task 10.14 (gate filtering implemented)

---

### Task 10.16: Git Diff Cache Per Gate Phase

**File**: `crates/roko-cli/src/orchestrate.rs`
**What**: Compute git diff once at the start of the gate phase and pass the cached
result through to all gates that need it, eliminating redundant git subprocess spawns.
**Bottleneck**: B09 (100-300ms saved)

**Steps**:
1. Define `GatePhaseContext` struct:
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
3. Parse modified file paths from the diff stat output
4. Pass `GatePhaseContext` to the gate dispatch path so gates can read
   `ctx.diff_stat` / `ctx.diff_full` instead of spawning their own git processes
5. Remove the two `git diff` subprocess spawns at orchestrate.rs ~line 16955-16970
   and replace with reads from `GatePhaseContext`

**Acceptance criteria**:
- During a gate phase, at most ONE `git diff --stat` and ONE `git diff HEAD`
  process is spawned (verify via `tracing` or process count)
- Gate verdicts are unchanged (diff data is identical to what gates computed before)
- `cargo test --workspace` passes

---

### Task 10.17: Source Hash Gate Guard

**File**: `crates/roko-gate/src/gate_service.rs`
**What**: Before running the compile gate, hash the modified source files. If the
hash matches the last successful compile, skip the gate entirely.

**Steps**:
1. Define `fn hash_modified_sources(workdir: &Path, modified_files: &[String]) -> u64`
   that computes a fast hash (FxHash or xxhash) of the concatenated mtimes and sizes
   of all modified `.rs` files
2. Store the last successful compile hash in `.roko/state/last-compile-hash`
3. In `run_gates()`, before running the compile gate:
   - Compute current hash from `GatePhaseContext.modified_files`
   - If hash matches stored hash, skip compile and log "compile skipped (unchanged)"
   - If hash differs, run compile and update stored hash on pass
4. Never skip if the compile gate failed last time (always re-check after failure)

**Acceptance criteria**:
- Run `roko run` twice on the same unchanged codebase -- second run skips compile gate
- Modify a `.rs` file -- compile gate runs again
- `cargo test -p roko-gate` passes

**Dependencies**: Task 10.16 (GatePhaseContext provides modified_files list)

---

### Task 10.18: Parallel Gate Rungs (Compile + Fmt)

**File**: `crates/roko-gate/src/gate_service.rs`
**What**: Run independent gate rungs concurrently. Specifically, compile (rung 0)
and fmt (rung 4) are independent and can execute in parallel.

**Steps**:
1. In `run_gates()`, identify which rungs are independent:
   - Rung 0 (compile) and Rung 4 (fmt) are independent
   - Rung 1 (clippy) depends on Rung 0 (compile must pass first)
   - Rung 2 (test) depends on Rung 0
   - Rung 3 (diff) is independent of all others
2. Group rungs into parallel sets: `{0, 3, 4}` then `{1}` then `{2}`
3. Execute each parallel set with `futures::future::join_all()` or `tokio::join!`
4. Short-circuit: if any gate in a parallel set fails, cancel pending gates in
   later sets (but let the current parallel set finish)
5. Preserve the existing ordering for sequential dependencies

**Acceptance criteria**:
- Wall-clock time for the gate phase is reduced when compile and fmt run in parallel
- Gate verdicts are identical to sequential execution
- If compile fails, clippy and test are still skipped (dependency respected)
- `cargo test -p roko-gate` passes

---

## Phase 6: Warm Dispatch Pool (B04, B15)

### Task 10.19: Create WarmDispatchPool

**File**: `crates/roko-runtime/src/warm_dispatch_pool.rs` (NEW)
**What**: Implement the three-tier warm dispatch pool with hot/warm/cold slots,
RAII slot guards, and pool metrics.

**Steps**:
1. Create the file `crates/roko-runtime/src/warm_dispatch_pool.rs`
2. Implement structs per the warm pool design document:
   - `WarmPoolConfig` with `max_warm_slots`, `max_active`, `idle_timeout`,
     `pre_warm`, `pre_warm_targets`
   - `WarmSlot` with `provider`, `model`, `caller: Arc<dyn ModelCaller>`,
     `created_at`, `last_used`, `dispatches_served`, `state: SlotState`
   - `SlotState` enum: `Idle`, `Active { run_id, since }`, `Draining`
   - `WarmPoolMetrics` with `total_dispatches`, `warm_hits`, `cold_misses`,
     `evictions`, `peak_active`, `avg_acquire_us`
   - `WarmDispatchPool` with `config`, `slots: Mutex<Vec<WarmSlot>>`,
     `metrics: Mutex<WarmPoolMetrics>`, `factory`
   - `WarmSlotGuard<'a>` with `pool`, `slot_idx`, `caller`
3. Implement `acquire()` with three-tier lookup: exact match -> same provider -> cold construct
4. Implement `pre_warm()` for configured targets
5. Implement `evict_idle()` for timeout-based cleanup
6. Implement `release()` for slot return
7. Note: `Drop` for `WarmSlotGuard` cannot be async -- document that callers must
   call `pool.release(idx)` explicitly, or use a background task

**Acceptance criteria**:
- Unit test: acquire from empty pool -> cold miss, slot created
- Unit test: acquire, release, acquire again -> warm hit on second acquire
- Unit test: acquire with same provider but different model -> warm hit (provider reuse)
- Unit test: evict_idle removes slots past timeout
- Unit test: metrics accurately track hits/misses/evictions
- `cargo test -p roko-runtime` passes

---

### Task 10.20: Export WarmDispatchPool Module

**File**: `crates/roko-runtime/src/lib.rs`
**What**: Add the `warm_dispatch_pool` module to the crate and export key types.

**Steps**:
1. Add `pub mod warm_dispatch_pool;` to `lib.rs`
2. Add re-exports: `pub use warm_dispatch_pool::{WarmDispatchPool, WarmPoolConfig, WarmPoolMetrics, WarmSlotGuard};`
3. Add the module to the `#![allow(...)]` attribute if any new lints fire

**Acceptance criteria**:
- `cargo build -p roko-runtime` succeeds
- `cargo doc -p roko-runtime` generates docs for the new module

**Dependencies**: Task 10.19 (module must exist)

---

### Task 10.21: Wire WarmDispatchPool Into EffectDriver

**File**: `crates/roko-runtime/src/effect_driver.rs`
**What**: Add an optional `WarmDispatchPool` to `EffectServices` so that
`spawn_agent()` acquires from the pool before constructing cold.

**Steps**:
1. Add `pub warm_pool: Option<Arc<WarmDispatchPool>>` to `EffectServices`
2. In `spawn_agent()`, try the pool first:
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
3. After agent dispatch completes, release the slot back to the pool
4. Default `warm_pool` to `None` so all existing code paths are unchanged

**Acceptance criteria**:
- With `warm_pool = None`: behavior is identical to current (no regression)
- With `warm_pool = Some(pool)`: second dispatch reuses warm slot
- `cargo test -p roko-runtime` passes

**Dependencies**: Task 10.20 (WarmDispatchPool exported)

---

### Task 10.22: Wire WarmDispatchPool Into WorkflowEngine

**File**: `crates/roko-runtime/src/workflow_engine.rs`
**What**: Add pool lifecycle management: pre-warm on workflow start, evict idle
on workflow completion.

**Steps**:
1. In `WorkflowEngine::run()`, if `self.driver.services.warm_pool` is `Some`:
   - Call `pool.pre_warm().await` before entering the main workflow loop
   - After the workflow completes, call `pool.evict_idle().await`
2. Log pool metrics at the end of each workflow run:
   ```rust
   if let Some(ref pool) = self.driver.services.warm_pool {
       let m = pool.metrics().await;
       info!(warm_hits = m.warm_hits, cold_misses = m.cold_misses,
             avg_acquire_us = m.avg_acquire_us, "warm pool stats");
   }
   ```

**Acceptance criteria**:
- Pool metrics appear in logs after a workflow run
- Pre-warm creates slots for configured targets
- Evict removes idle slots past timeout
- `cargo test -p roko-runtime` passes

**Dependencies**: Task 10.21 (pool wired into EffectDriver)

---

### Task 10.23: Wire WarmDispatchPool Into roko run

**File**: `crates/roko-cli/src/run.rs`
**What**: Construct the pool in the CLI `run_once()` path and pass it to
`EffectServices`. For CLI one-shot usage, skip pre-warming (first request warms
via SHARED_HTTP_CLIENT; second request reuses the warm connection).

**Steps**:
1. In `run_once()`, construct `WarmPoolConfig::default()` (no pre-warm for CLI)
2. Build a `model_caller_factory` closure that creates `ModelCallService` instances
   from provider+model pairs
3. Construct `WarmDispatchPool::new(config, Arc::new(factory))`
4. Set `effect_services.warm_pool = Some(Arc::new(pool))`
5. The pool lives for the duration of the run and is dropped on completion

**Acceptance criteria**:
- `roko run` with standard workflow (2 agent calls) reuses the warm slot for the
  second agent call (verify via pool metrics in trace log)
- `roko run` with express workflow (1 agent call) still works correctly
- `cargo test -p roko-cli` passes

**Dependencies**: Task 10.22 (pool lifecycle in WorkflowEngine)

---

### Task 10.24: Wire WarmDispatchPool Into roko serve

**File**: `crates/roko-serve/src/runtime.rs`
**What**: For the HTTP server, pre-warm on startup and run periodic eviction.
Long-running server benefits most from warm slots.

**Steps**:
1. Read `WarmPoolConfig` from `roko.toml` under `[conductor.warm_pool]`:
   - `enabled: bool` (default true)
   - `max_warm_slots: usize` (default 4)
   - `idle_timeout_secs: u64` (default 300)
   - `pre_warm_providers: Vec<String>` (default: configured providers)
   - `pre_warm_models: Vec<String>` (default: default model)
2. Construct `WarmDispatchPool` with the config
3. Call `pool.pre_warm().await` on server startup
4. Spawn a periodic eviction task: `tokio::spawn` with 60-second interval calling
   `pool.evict_idle().await`
5. Pass pool to all route handlers that dispatch agent work

**Acceptance criteria**:
- `roko serve` starts with warm slots pre-created (verify via startup log)
- After 5+ minutes of idle, warm slots are evicted (verify via metrics endpoint)
- Multiple concurrent API requests reuse warm slots
- `cargo test -p roko-serve` passes

**Dependencies**: Task 10.23 (pool construction pattern established)

---

### Task 10.25: WarmPoolConfig in roko.toml Schema

**File**: `crates/roko-core/src/config/mod.rs`
**Also**: `roko.toml` (add default section)
**What**: Add the `[conductor.warm_pool]` config section to the TOML schema.

**Steps**:
1. Add `WarmPoolConfig` struct to the config module:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct WarmPoolConfig {
       pub enabled: bool,
       pub max_warm_slots: usize,
       pub max_active: usize,
       pub idle_timeout_secs: u64,
       pub pre_warm_on_serve: bool,
       pub pre_warm_providers: Vec<String>,
       pub pre_warm_models: Vec<String>,
   }
   ```
2. Add `pub warm_pool: Option<WarmPoolConfig>` to the `[conductor]` config section
3. Implement `Default` with sensible values:
   - `enabled: true`, `max_warm_slots: 4`, `max_active: 8`
   - `idle_timeout_secs: 300`, `pre_warm_on_serve: true`
4. Wire deserialization so `roko config show` displays the warm pool config
5. Add validation: `max_warm_slots <= 16`, `idle_timeout_secs >= 30`

**Acceptance criteria**:
- `roko config show` includes warm pool configuration
- `roko config validate` accepts valid warm pool config
- Missing `[conductor.warm_pool]` section uses defaults (backwards compatible)
- `cargo test -p roko-core` passes

---

## Phase 7: Speculative Execution

### Task 10.26: Speculative Pre-Warming in Workflow Engine

**File**: `crates/roko-runtime/src/workflow_engine.rs`
**What**: While the implementer agent is running, speculatively pre-warm the
reviewer's model caller so it is ready instantly when the implementation completes.

**Steps**:
1. In the workflow engine run loop, after spawning the implementer agent:
   - Check if the workflow template includes a review phase
   - If yes and the warm pool has idle capacity, call
     `pool.pre_warm_for(reviewer_provider, reviewer_model)` in a `tokio::spawn`
   - This runs concurrently with the implementer's inference
2. Add `pub async fn pre_warm_for(&self, provider: &str, model: &str)` to
   `WarmDispatchPool` that creates a single warm slot for the given provider/model
3. When the implementer completes and the reviewer is dispatched,
   `pool.acquire()` should find the speculatively pre-warmed slot (warm hit)
4. If the implementation fails (no review needed), the pre-warmed slot sits idle
   and is evicted after `idle_timeout`

**Acceptance criteria**:
- Standard workflow: reviewer acquisition takes <5ms (warm hit from speculation)
- Express workflow: no speculation occurs (no reviewer phase)
- Failed implementation: pre-warmed slot is eventually evicted, not leaked
- Pool metrics show speculative warm hits

**Dependencies**: Task 10.22 (pool lifecycle in WorkflowEngine)

---

## Phase 8: Profile-Guided Optimization

### Task 10.27: PGO Build Script

**File**: `scripts/pgo-build.sh` (NEW)
**What**: Create a script that builds an instrumented binary, runs representative
workloads, merges profiles, and rebuilds with PGO data.

**Steps**:
1. Create `scripts/pgo-build.sh`:
   ```bash
   #!/usr/bin/env bash
   set -euo pipefail
   PGO_DIR="${1:-/tmp/roko-pgo-data}"
   rm -rf "$PGO_DIR" && mkdir -p "$PGO_DIR"
   # Step 1: Build instrumented binary
   RUSTFLAGS="-Cprofile-generate=$PGO_DIR" cargo build --release -p roko-cli
   # Step 2: Run representative workloads
   ./target/release/roko config show 2>/dev/null || true
   ./target/release/roko plan validate plans/ 2>/dev/null || true
   ./target/release/roko status 2>/dev/null || true
   # Step 3: Merge profiles
   llvm-profdata merge -o "$PGO_DIR/merged.profdata" "$PGO_DIR"
   # Step 4: Rebuild with PGO data
   RUSTFLAGS="-Cprofile-use=$PGO_DIR/merged.profdata" cargo build --release -p roko-cli
   echo "PGO build complete: target/release/roko"
   ```
2. Make the script executable: `chmod +x scripts/pgo-build.sh`
3. Document that `llvm-profdata` must be installed (`rustup component add llvm-tools-preview`)

**Acceptance criteria**:
- `./scripts/pgo-build.sh` completes without error
- The resulting binary is smaller or comparable in size to a non-PGO release build
- Benchmark: PGO binary shows 5-15% improvement on `bench_config_load` and
  `bench_prompt_assembly` (the CPU-bound portions)

---

### Task 10.28: PGO CI Integration

**File**: `.github/workflows/release.yml` (or create `.github/workflows/pgo-build.yml`)
**What**: Add a PGO build step to the release workflow so that published binaries
are profile-optimized.

**Steps**:
1. Add a `pgo-build` job to the release workflow:
   - Install `llvm-tools-preview` via `rustup component add`
   - Build instrumented binary
   - Run representative workloads (config show, plan validate, status)
   - Merge profiles with `llvm-profdata merge`
   - Rebuild with PGO data
   - Upload the PGO-optimized binary as the release artifact
2. Keep the standard non-PGO build as a fallback in case PGO build fails
3. Add a step that compares PGO vs non-PGO binary sizes

**Acceptance criteria**:
- CI produces a PGO-optimized binary on release tags
- PGO build failure does not block the release (falls back to standard build)
- Release notes include whether the binary is PGO-optimized

**Dependencies**: Task 10.27 (PGO script tested locally)

---

## Phase 9: HAL Benchmark Integration

### Task 10.29: Create HAL Agent Wrapper

**File**: `hal/roko_agent/main.py` (NEW)
**Also**: `hal/roko_agent/requirements.txt` (NEW)
**What**: Implement the Python wrapper that exposes roko as a HAL-compatible agent
for standardized benchmark evaluation.

**Steps**:
1. Create `hal/roko_agent/main.py` with the `run(task, **kwargs)` function
   conforming to HAL's agent protocol
2. The wrapper should:
   - Accept task dict with `instance_id`, `prompt` / `problem_statement`, `repo`,
     `base_commit`, `hints`
   - Clone/checkout the task repository into a temp directory
   - Run `roko init` in the workspace
   - Execute `roko run --model <model> --workflow-template <template> --gates <gates>
     --output json "<prompt>"` via `subprocess.run()`
   - Capture `git diff HEAD` as the `model_patch`
   - Return dict with `model_patch`, `cost`, `tokens`, `duration_s`, `model`, `exit_code`
3. Create `hal/roko_agent/requirements.txt` with no dependencies (only stdlib used)
4. Support kwargs: `model_name`, `workflow`, `gates`, `timeout`, `roko_binary`

**Acceptance criteria**:
- `hal-eval --benchmark swe_bench_verified_mini --agent_dir hal/roko_agent/
  --agent_function main.run --agent_name "roko-test" -A model_name=gpt-4.1-nano
  -A roko_binary=./target/release/roko --max_concurrent 1` executes at least one task
- The returned `model_patch` is a valid unified diff
- Timeout is respected (process killed after `timeout` seconds)

---

### Task 10.30: Add JSON Output Mode to roko run

**File**: `crates/roko-cli/src/run.rs`
**Also**: `crates/roko-cli/src/main.rs`
**What**: Add `--output json` flag to `roko run` that outputs structured JSON
with cost, tokens, duration, and result summary -- required by the HAL wrapper.

**Steps**:
1. Add `--output <FORMAT>` argument to the `run` subcommand:
   - `text` (default): current behavior
   - `json`: structured JSON to stdout
2. Define `RunOutputJson` struct:
   ```rust
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
3. At the end of `run_once()`, if `--output json`, serialize `RunOutputJson` and
   print to stdout
4. Suppress all non-JSON output (progress bars, status messages) when `--output json`

**Acceptance criteria**:
- `roko run --output json "echo hello" | jq .success` outputs `true` or `false`
- JSON output includes all fields with correct types
- `roko run --output text "echo hello"` behavior is unchanged
- `cargo test -p roko-cli` passes

---

### Task 10.31: Create Performance Benchmark Suite Definition

**File**: `.roko/bench/suites/perf.json` (NEW)
**What**: Define a performance benchmark suite with 5+ tasks that measure roko's
non-inference overhead across workflow templates.

**Steps**:
1. Create `.roko/bench/suites/perf.json` with the BenchSuite format
2. Include tasks:
   - `perf-001`: Minimal prompt, express workflow, no gates (baseline overhead)
   - `perf-002`: Single tool call (file write), express workflow, no gates
   - `perf-003`: Code edit task, express workflow, express gates
   - `perf-004`: Code generation task, standard workflow, full gates
   - `perf-005`: Multi-step task, full workflow, express gates
3. Each task specifies `model`, `workflow`, `gates` configuration
4. All tasks use fast models (gpt-4.1-nano or gpt-4.1-mini) to minimize
   inference time and isolate framework overhead

**Acceptance criteria**:
- `roko bench run .roko/bench/suites/perf.json` completes without error
  (or equivalent command from existing bench infrastructure)
- Suite definition validates against the `BenchSuite` schema
- Results include per-task wall-clock time, inference time, and overhead time

---

### Task 10.32: Create Quality Benchmark Suite Definition

**File**: `.roko/bench/suites/quality.json` (NEW)
**What**: Define a quality regression suite with 5+ tasks that test code generation,
bug fixing, and refactoring quality across models.

**Steps**:
1. Create `.roko/bench/suites/quality.json` with the BenchSuite format
2. Include tasks:
   - `qual-001`: Fix compilation error (easy, type mismatch)
   - `qual-002`: Reverse a string with Unicode handling (medium, correctness)
   - `qual-003`: Refactor loops to iterators (medium, code quality)
   - `qual-004`: Add error handling to an existing function (medium, robustness)
   - `qual-005`: Implement a simple trait (medium, API compliance)
3. Each task includes `expected_gates` for automated scoring
4. All tasks are self-contained (no external dependencies)

**Acceptance criteria**:
- Suite definition validates against the `BenchSuite` schema
- Each task has clear pass/fail criteria via gate verdicts
- Tasks cover different agent capabilities (fix, generate, refactor)

---

## Phase 10: Regression Testing and CI

### Task 10.33: Benchmark Comparison Command

**File**: `crates/roko-cli/src/commands/mod.rs`
**Also**: `crates/roko-serve/src/bench.rs`
**What**: Add `roko bench compare <baseline> <current>` subcommand that compares
two benchmark result files and reports regressions.

**Steps**:
1. Add `bench compare` subcommand to the CLI
2. Load two JSON result files (baseline and current)
3. For each task present in both:
   - Compare wall-clock time: flag if current > baseline * 1.2 (20% regression)
   - Compare overhead time: flag if non-inference time increased
   - Compare gate pass rate: flag if any gate that passed now fails
4. Output a comparison table to stdout
5. Exit with code 1 if any regression threshold is exceeded
6. Accept `--threshold <percent>` flag for custom regression tolerance

**Acceptance criteria**:
- `roko bench compare a.json b.json` outputs a table of per-task comparisons
- Exit code 0 if no regressions, 1 if any metric exceeds threshold
- `--threshold 50` allows up to 50% regression before failing
- `cargo test -p roko-cli` passes

---

### Task 10.34: Per-PR Performance Check CI Workflow

**File**: `.github/workflows/perf-check.yml` (NEW)
**What**: Create a CI workflow that runs the performance benchmark suite on every
PR and compares against main branch results.

**Steps**:
1. Create `.github/workflows/perf-check.yml`:
   - Trigger on `pull_request`
   - Build release binary
   - Run perf benchmark suite
   - Download main branch baseline results (from artifact cache)
   - Run `roko bench compare` between baseline and current
   - Post comparison as PR comment (or fail the check)
2. Cache the main branch results as a GitHub Actions artifact
3. On pushes to main, update the cached baseline results

**Acceptance criteria**:
- PRs get a perf regression check that reports overhead changes
- 20%+ regression fails the check (configurable via workflow input)
- Main branch baseline is updated on each merge to main

**Dependencies**: Task 10.33 (bench compare command exists)

---

### Task 10.35: Nightly HAL Benchmark CI Workflow

**File**: `.github/workflows/hal-bench.yml` (NEW)
**What**: Create a nightly CI workflow that runs roko through HAL's SWE-bench mini
evaluation and tracks quality over time.

**Steps**:
1. Create `.github/workflows/hal-bench.yml`:
   - Trigger on `schedule` (daily at 2 AM UTC) and `workflow_dispatch`
   - Build release binary
   - Install `hal-harness` via pip
   - Run `hal-eval` on SWE-bench mini (50 tasks) with roko agent wrapper
   - Upload results as artifacts
   - Compare with previous nightly results if available
2. Configure `max_concurrent: 5` to limit API costs
3. Default model: `gpt-4.1-mini` (cost-effective for nightly runs)
4. Upload results to `.roko/bench/results/nightly-YYYYMMDD.json`

**Acceptance criteria**:
- Nightly workflow runs and produces HAL results
- Results include per-task pass/fail, cost, and duration
- Workflow does not exceed a reasonable budget cap (~$20/night)
- Results are comparable across nightly runs (same model/config)

**Dependencies**: Task 10.29 (HAL agent wrapper exists)

---

### Task 10.36: Multi-Run Consistency Mode for Bench Harness

**File**: `crates/roko-cli/src/bench.rs` (if it exists as SWE-bench proxy)
**Also**: `crates/roko-serve/src/bench.rs`
**What**: Add a `trials: usize` field to benchmark options so each task runs K
times and consistency metrics are computed across runs.

**Steps**:
1. Add `pub trials: usize` to `SweBenchOptions` (or equivalent bench config struct),
   defaulting to 1 for backwards compatibility
2. When `trials > 1`:
   - Run each benchmark task K times with different seeds
   - Collect pass/fail results per trial
   - Compute consistency metrics:
     - Pass rate: fraction of trials that pass
     - K-trial consistency: fraction of tasks where ALL K trials pass
     - Token usage variance: coefficient of variation across trials
3. Include consistency metrics in the benchmark result output
4. For `trials = 1`: behavior is unchanged

**Acceptance criteria**:
- `trials = 1`: identical behavior to current (no regression)
- `trials = 5`: each task runs 5 times, results include per-trial outcomes
- Consistency metrics are computed and included in results JSON
- A task that passes 3/5 trials gets a 60% pass rate, not a binary pass/fail

---

### Task 10.37: Wire Cost Tracking Into Bench Results

**File**: `crates/roko-cli/src/bench.rs` (or `crates/roko-serve/src/bench.rs`)
**What**: Fill in the `cost_usd` field in `BenchResult` which is currently always
0.0. Connect to the learning subsystem's cost tracking.

**Steps**:
1. After each benchmark task completes, read the cost from the `ModelCallResponse`
   or from the efficiency event written during the run
2. Sum costs across all model calls within a task to get total `cost_usd`
3. If cost data is unavailable (e.g., Ollama local models), estimate from token
   counts using the model's published pricing
4. Include cost in the benchmark result: `result.cost_usd = total_cost`
5. Add cost to the Pareto analysis: plot accuracy vs. cost across models

**Acceptance criteria**:
- Benchmark results have non-zero `cost_usd` for API-backed models
- Benchmark results have `cost_usd = 0.0` for local models (Ollama)
- `cargo test --workspace` passes

---

### Task 10.38: Pareto Frontier Analysis in Bench Results

**File**: `crates/roko-serve/src/bench.rs`
**What**: Compute the cost-quality Pareto frontier across benchmark results for
different models and routing configurations.

**Steps**:
1. Add `pub fn pareto_frontier(results: &[BenchRunResult]) -> Vec<ParetoPoint>` function
2. `ParetoPoint`: `{ model: String, pass_rate: f64, avg_cost: f64, avg_latency_ms: u64 }`
3. Compute the Pareto frontier: keep only points where no other point has both
   higher pass rate AND lower cost
4. Sort frontier points by cost (ascending)
5. Integrate with existing `roko_learn::pareto` module if one exists
6. Include Pareto analysis in benchmark suite output

**Acceptance criteria**:
- Given 5 models' results, Pareto frontier contains 2-4 points (not all 5)
- Dominated models (higher cost AND lower quality) are excluded
- Frontier is correctly sorted by cost ascending
- `cargo test --workspace` passes

**Dependencies**: Task 10.37 (cost tracking populated)

---

## Phase 11: Batch Inference and Connection Reuse

### Task 10.39: Verify SHARED_HTTP_CLIENT Connection Reuse

**File**: `crates/roko-agent/src/provider/mod.rs`
**What**: Add logging and tests to confirm that the existing SHARED_HTTP_CLIENT
correctly reuses connections across multiple requests to the same provider.

**Steps**:
1. Add a `tracing::debug!` log in `shared_http_client()` that logs when the client
   is first initialized (via `LazyLock`)
2. Add a test that makes two requests to the same mock server and verifies only
   one TCP connection is established (use `hyper`'s test server or mock)
3. Verify the existing config: `pool_max_idle_per_host(10)`,
   `pool_idle_timeout(90s)`, `tcp_keepalive(30s)` are optimal
4. Consider increasing `pool_idle_timeout` to 300s for `roko serve` (long-running)
   while keeping 90s for CLI

**Acceptance criteria**:
- Test confirms connection reuse within the idle timeout window
- No unnecessary TLS handshakes for sequential requests to the same provider
- Log shows SHARED_HTTP_CLIENT initialized exactly once per process

---

### Task 10.40: Parallel Inference for Independent Plan Tasks

**File**: `crates/roko-cli/src/orchestrate.rs`
**Also**: `crates/roko-orchestrator/src/dag.rs`
**What**: Ensure that the DAG executor dispatches independent tasks (no
dependencies between them) concurrently, using the warm pool for connection reuse.

**Steps**:
1. Verify that the existing DAG executor in `orchestrate.rs` / `dag.rs` already
   identifies independent tasks (tasks with no unmet dependencies)
2. If independent tasks are currently dispatched sequentially, change the dispatch
   loop to use `tokio::spawn` for each ready task and `futures::future::join_all`
   to await them
3. Limit concurrency to `config.conductor.max_concurrent_tasks` (default: 3)
4. Each concurrent task should acquire from the warm pool independently
5. Verify that concurrent tasks writing to the same substrate do not corrupt data
   (mutex protection in FileSubstrate)

**Acceptance criteria**:
- A plan with 3 independent tasks dispatches all 3 concurrently (verify via trace log)
- Concurrent tasks complete faster than sequential (wall clock < sum of individual times)
- No data corruption in substrate writes from concurrent tasks
- `cargo test --workspace` passes

---

### Task 10.41: Batch Inference Collector for Plan Execution

**File**: `crates/roko-agent/src/batch.rs` (NEW)
**What**: Implement a `BatchCollector` that accumulates inference requests from
concurrent plan tasks and dispatches them in parallel, sharing connection resources.

**Steps**:
1. Create `crates/roko-agent/src/batch.rs` with `BatchCollector` struct:
   - `pending: Vec<(ModelCallRequest, oneshot::Sender<Result<ModelCallResponse>>)>`
   - `batch_window: Duration` (default 50ms)
   - `max_batch_size: usize` (default 10)
2. `pub async fn submit(&mut self, request: ModelCallRequest) -> Result<ModelCallResponse>`
   queues the request and waits for the batch to flush
3. `async fn flush(&mut self)` dispatches all pending requests concurrently via
   `futures::future::join_all()` -- these are NOT provider-level batch API calls,
   just concurrent individual requests sharing the connection pool
4. Auto-flush when `pending.len() >= max_batch_size` or `batch_window` elapses
5. Export from `crates/roko-agent/src/lib.rs`

**Acceptance criteria**:
- 5 requests submitted within the batch window are dispatched concurrently
- Each request receives its individual response (not a batch response)
- Timeout: if batch window elapses with <max_batch_size requests, flush partial batch
- `cargo test -p roko-agent` passes

---

## Phase 12: Final Integration and Validation

### Task 10.42: End-to-End Performance Validation Script

**File**: `scripts/perf-validate.sh` (NEW)
**What**: Create a script that runs before/after measurements for all optimizations
and produces a summary report.

**Steps**:
1. Create `scripts/perf-validate.sh`:
   ```bash
   #!/usr/bin/env bash
   set -euo pipefail
   MODELS=("gpt-4.1-nano" "gpt-4.1-mini")
   TEMPLATES=("express" "standard")
   GATES=("none" "express" "full")
   RESULTS_DIR=".roko/bench/perf-$(date +%Y%m%d)"
   mkdir -p "$RESULTS_DIR"
   for model in "${MODELS[@]}"; do
     for template in "${TEMPLATES[@]}"; do
       for gate in "${GATES[@]}"; do
         echo ">>> $model / $template / $gate"
         /usr/bin/time -l ./target/release/roko run \
           --model "$model" --workflow-template "$template" \
           --gates "$gate" --output json \
           "Reply with only the word hello" \
           2>"$RESULTS_DIR/${model}_${template}_${gate}_time.txt" \
           1>"$RESULTS_DIR/${model}_${template}_${gate}_output.json" \
           || true
       done
     done
   done
   echo "Results in $RESULTS_DIR"
   ```
2. Make executable: `chmod +x scripts/perf-validate.sh`
3. Parse `/usr/bin/time` output to extract wall clock, peak RSS, syscall count

**Acceptance criteria**:
- Script runs all model/template/gate combinations
- Each run produces both timing data and JSON output
- Results directory contains 12+ result files (2 models x 2 templates x 3 gates)

---

### Task 10.43: Warm Pool Metrics Endpoint in roko serve

**File**: `crates/roko-serve/src/routes/status/mod.rs`
**What**: Add a `/api/status/warm-pool` endpoint that exposes pool metrics for
monitoring and dashboards.

**Steps**:
1. Add route handler `async fn warm_pool_status(State(state): State<AppState>) -> Json<WarmPoolMetrics>`
2. Read metrics from `state.warm_pool.metrics().await`
3. Return JSON with `total_dispatches`, `warm_hits`, `cold_misses`, `evictions`,
   `peak_active`, `avg_acquire_us`, `current_slots`, `idle_slots`
4. Register the route in the status router

**Acceptance criteria**:
- `curl localhost:6677/api/status/warm-pool` returns pool metrics JSON
- Metrics update after agent dispatches
- Returns 200 with empty/default metrics if pool is not configured
- `cargo test -p roko-serve` passes

**Dependencies**: Task 10.24 (pool wired into serve)

---

### Task 10.44: BenchmarkRegressionGate Baseline Infrastructure

**File**: `crates/roko-gate/src/benchmark_gate.rs`
**What**: Fill in the currently-stub `BenchmarkRegressionGate` with baseline
capture, storage, and comparison logic so it can detect performance regressions
during gate verification.

**Steps**:
1. Read the current stub implementation and identify the `verify()` method
2. Implement baseline capture:
   - After a successful benchmark run, store timing results in
     `.roko/state/bench-baselines/<gate-name>.json`
   - Baseline format: `{ task_id, wall_ms, overhead_ms, tokens, timestamp }`
3. Implement comparison logic:
   - In `verify()`, load the baseline for the current task
   - Compare current timing against baseline
   - If current > baseline * (1 + threshold), return `GateVerdict::Fail`
   - Default threshold: 20%
4. First run (no baseline exists): always pass and capture baseline

**Acceptance criteria**:
- First run: passes and creates baseline file
- Second run (same perf): passes
- Third run (injected 30% slowdown): fails with regression message
- `cargo test -p roko-gate` passes

---

### Task 10.45: Integrate Perf Metrics Into TUI Dashboard

**File**: `crates/roko-cli/src/tui/` (relevant tab module)
**What**: Add a performance metrics panel to the TUI dashboard showing warm pool
stats, recent run overhead, and optimization effectiveness.

**Steps**:
1. Identify the appropriate TUI tab (likely the status or metrics tab)
2. Add a "Performance" section with:
   - Warm pool status: hits/misses/evictions
   - Recent run overhead: config load, agent construct, gate, persistence times
   - Optimization state: which caches are active (config, convention, routing)
3. Read metrics from:
   - `.roko/bench/perf-latest.json` for recent benchmark results
   - Warm pool metrics (if running in serve mode with shared state)
   - Runtime event log for per-phase timing data
4. Refresh on file change using the existing `notify::RecommendedWatcher`

**Acceptance criteria**:
- `roko dashboard` shows a performance section with recent timing data
- Metrics update when a new benchmark run completes
- No crash if metrics files are missing (graceful degradation to "no data")

---

## Summary

### Phase Execution Order

| Phase | Tasks | Estimated Effort | Cumulative Savings |
|-------|-------|-----------------|-------------------|
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

### Target Performance After All Phases

| Scenario | Current | Target | Reduction |
|----------|---------|--------|-----------|
| Express + fast US model, no gates | ~710ms overhead | ~345ms | 51% |
| Standard + fast model, express gates | ~4700-6200ms | ~2000ms | 57-68% |
| 10-task plan, 3-wide parallel | ~100s | ~55s | 45% |

### Critical Dependencies

```
10.1 (instrumentation) ── all other tasks use tracing for validation
10.3 (config cache) ──── 10.4 (learning single-open)
10.13 (GateMode enum) ── 10.14 (gate filtering) ── 10.15 (--gates CLI flag)
10.16 (diff cache) ───── 10.17 (source hash guard)
10.19 (pool struct) ──── 10.20 (export) ── 10.21 (EffectDriver) ── 10.22 (WorkflowEngine)
10.22 (WorkflowEngine) ─ 10.23 (CLI) ── 10.24 (serve) ── 10.26 (speculation)
10.27 (PGO script) ───── 10.28 (PGO CI)
10.29 (HAL wrapper) ──── 10.35 (nightly CI)
10.33 (bench compare) ── 10.34 (per-PR CI)
10.37 (cost tracking) ── 10.38 (Pareto analysis)
```
