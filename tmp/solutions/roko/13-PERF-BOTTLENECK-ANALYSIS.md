# Performance Bottleneck Analysis

Date: 2026-04-29 (expanded from 2026-04-28)
Target: <500ms wall-clock for `roko run` with fast API models, no gates
Target: <2s wall-clock for `roko run` with standard workflow + fast model + express gates

---

## Executive Summary

Roko's runtime performance is bounded by five layers: CLI bootstrap, agent dispatch,
model routing, gate verification, and persistence. After fixing the shared HTTP client
(B01), the remaining bottlenecks are dominated by network latency (inference) and
subprocess costs (gates). The optimization strategy is: eliminate redundant work
in layers 1/4/5, parallelize layer 3, and provide fast-path bypasses for layer 3.

### Bottleneck Priority Matrix

| Rank | ID | Bottleneck | Cost (ms) | Fix Effort | Status |
|------|-----|-----------|-----------|------------|--------|
| 1 | B01 | HTTP client per agent | 50-200 | Low | **FIXED** |
| 2 | B02 | Config loaded 4+ times per run | 10-50 | Low | Open |
| 3 | B03 | LearningRuntime opened twice | 100-200 | Medium | Open |
| 4 | B04 | Claude CLI subprocess spawn | 200-500 | High (arch) | Open |
| 5 | B05 | Safety contract file I/O per dispatch | 10-50 | Low | Open |
| 6 | B06 | Cascade router sync read | 150-300 | Medium | Open |
| 7 | B07 | Enrichment pipeline sequential | 200-500 | Medium | Open |
| 8 | B08 | Cargo compile gate | 500-2000 | High | Partial |
| 9 | B09 | Git diff computed per rung | 100-300 | Low | Open |
| 10 | B10 | Sequential substrate writes | 50-100 | Medium | Open |
| 11 | B11 | Feedback flush synchronous | 30-50 | Medium | Open |
| 12 | B12 | Prompt assembly I/O | 50-200 | Medium | Open |
| 13 | B13 | Event bus per-event serialization | 20-40 | Low | Open |
| 14 | B14 | Workspace convention detection | 30-100 | Low | Open |
| 15 | B15 | Agent warm pool missing for API | 20-50 | Medium | Partial |

**Total addressable overhead**: 1020-4330ms across all bottlenecks.
**After B01-B03 + B05-B06 + B10-B11**: saves 370-850ms.
**After all fixes**: saves 1020-4330ms, hitting 300-500ms for fast models.

---

## Layer 1: CLI Entry and Configuration

### B02 -- Config Loaded 4+ Times (10-50ms)

**Files**:
- `crates/roko-cli/src/run.rs:487` -- `load_layered(workdir)` first load
- `crates/roko-cli/src/run.rs:490` -- `load_config(workdir)` second load
- `crates/roko-cli/src/run.rs:1272` -- `load_config(workdir)` in `dispatch_agent()`
- `crates/roko-cli/src/run.rs:1908` -- `load_config()` in `append_episode_log()`
- `crates/roko-cli/src/run.rs:1831` -- `load_roko_config_models(workdir)` reads same TOML

**Problem**: The same `roko.toml` file is parsed from disk 4+ times per `roko run`.
Each parse involves `std::fs::read_to_string()` + TOML deserialization. The config
file is ~2KB, so I/O is fast, but TOML parsing plus validation adds ~10ms per load.

**Root cause**: Functions were written independently and each loads its own copy.
No config caching or threading pattern was established early.

**Fix**: Load once at CLI entry, store in `Arc<RokoConfig>`, pass through call chain.

```rust
// In main.rs or run.rs entry point:
let config = Arc::new(load_layered(&workdir)?);

// Pass to all consumers:
fn dispatch_agent(config: &Arc<RokoConfig>, ...) { /* use config directly */ }
fn append_episode_log(config: &Arc<RokoConfig>, ...) { /* same */ }
```

**Measurement**: Before/after with `tracing::instrument` on `load_config`.

---

### B03 -- LearningRuntime Opened Twice (100-200ms)

**Files**:
- `crates/roko-cli/src/run.rs:1839-1847` -- opened in main dispatch path
- `crates/roko-cli/src/run.rs:1052-1061` -- `append_episode_log()` opens again

**Problem**: `LearningRuntime::open_under()` reads three JSON files from disk:
1. `.roko/learn/cascade-router.json` (~5KB, routing weights)
2. `.roko/learn/experiments.json` (~2KB, A/B state)
3. `.roko/learn/gate-thresholds.json` (~1KB, EMA thresholds)

It also spawns a background distillation task. All of this happens twice per run.

**Impact breakdown**:
```
File reads:       3 files x 10ms = 30ms
JSON parsing:     3 parses x 5ms = 15ms
Distillation:     1 spawn x 20ms = 20ms
Mutex setup:      ~5ms
─────────────────────────────────────
Per open:         ~70ms
Two opens:        ~140ms
Savings:          ~70ms
```

**Fix**: Thread the `LearningRuntime` instance through to `append_episode_log()`.

```rust
// Currently:
fn append_episode_log(workdir: &Path, ...) {
    let lr = LearningRuntime::open_under(workdir)?;  // REDUNDANT
    lr.record_episode(...);
}

// Fixed:
fn append_episode_log(lr: &LearningRuntime, ...) {
    lr.record_episode(...);  // reuse existing
}
```

---

## Layer 2: Agent Dispatch

### B01 -- HTTP Client Per Agent (50-200ms) [FIXED]

**Files**:
- `crates/roko-agent/src/provider/mod.rs:88-110` -- `SHARED_HTTP_CLIENT` + `shared_http_client()`
- `crates/roko-agent/src/http.rs:126-128` -- `ReqwestPoster::new()` uses shared client

**Status**: FIXED. The `SHARED_HTTP_CLIENT` static provides process-wide connection
pooling with 10 idle connections per host, 90s idle timeout, and 30s TCP keep-alive.
All production HTTP posters now use this client.

**Configuration**:
```rust
static SHARED_HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .pool_max_idle_per_host(10)
        .pool_idle_timeout(Duration::from_secs(90))
        .tcp_keepalive(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .unwrap()
});
```

**Remaining gap**: The `OpenAiCompatLlmBackend` at `crates/roko-agent/src/openai_compat_backend.rs:80`
still creates its own `ReqwestPoster::new()` per backend instance. This is fine because
`ReqwestPoster::new()` now delegates to `shared_http_client()`, but it means each backend
holds an `Arc` clone to the same underlying client -- not a separate connection pool.

---

### B04 -- Claude CLI Subprocess Spawn (200-500ms)

**Files**:
- `crates/roko-agent/src/claude_cli_agent.rs:302-430` -- `tokio::process::Command` spawn
- Process group setup, PID registration, stdout/stderr capture tasks

**Problem**: Every Claude CLI agent call forks a new process. The fork cost includes:
1. Process creation: ~50ms (memory copy-on-write setup)
2. `claude` binary startup: ~100ms (Node.js / native runtime init)
3. TLS context initialization: ~50ms
4. Environment variable clone: ~10ms
5. File descriptor duplication: ~5ms
6. Stdout/stderr capture task spawn: ~5ms

**Cost formula**: `spawn_cost = 50 + node_init + tls_init + env_clone = 200-500ms`

For API-backed models, the spawn cost is zero because the dispatch goes through
`ModelCallService` -> `create_agent_for_model()` -> HTTP request.

**Architectural analysis**:
- The Claude CLI is designed for one-shot execution. There is no long-running daemon mode.
- `--resume <session-id>` reuses context but still spawns a new process.
- The `interactive` mode could theoretically be used with stdin piping, but this is
  fragile and breaks on multi-turn tool use.

**Fix options**:
1. **Bypass CLI for perf-critical paths**: Route through Anthropic API directly (preferred).
   The `ClaudeAgent` at `crates/roko-agent/src/claude_agent.rs` already does this.
2. **Warm process pool**: Keep Claude CLI processes alive (see WARM-POOL-DESIGN.md).
   Hard to implement correctly due to CLI's one-shot model.
3. **Session caching**: Use `--resume` to reuse context, saving the prompt assembly
   cost but not the spawn cost.

**Recommendation**: For the workflow engine path (`EffectDriver::spawn_agent()`), always
use the API path via `ModelCallService`. Reserve Claude CLI for interactive chat only.

---

### B05 -- Safety Contract File I/O (10-50ms)

**Files**:
- `crates/roko-agent/src/safety/` -- contract loading and enforcement
- Contract YAML loaded from disk on every `ToolDispatcher::dispatch()`

**Problem**: The safety contract (YAML file defining tool permissions per role) is
loaded from disk on every tool invocation. For a typical agent turn with 5-10 tool
calls, this adds 50-500ms of redundant file I/O.

**Fix**: Cache the contract per role at agent startup using `OnceLock<AgentContract>`.

```rust
// In ToolDispatcher:
struct ToolDispatcher {
    contract_cache: OnceLock<AgentContract>,
    // ...
}

impl ToolDispatcher {
    fn contract(&self, role: &str) -> &AgentContract {
        self.contract_cache.get_or_init(|| {
            load_contract_for_role(role).unwrap_or_default()
        })
    }
}
```

---

### B15 -- Agent Warm Pool Missing for API (20-50ms)

**Files**:
- `crates/roko-agent/src/multi_pool.rs` -- `MultiAgentPool` with warm entries
- `crates/roko-agent/src/pool.rs` -- `AgentPool` with instance management
- `crates/roko-agent/src/session.rs` -- `WarmReusePolicy` and session management

**Problem**: The `MultiAgentPool` exists and supports warm entries, but the workflow
engine does not use it. Each `EffectDriver::spawn_agent()` call constructs a new
agent via `ModelCallService::call()`, which calls `create_agent_for_model()` fresh.

**Status**: The warm pool infrastructure is built (`WarmEntry`, `WarmReusePolicy`,
`MultiAgentPool`), but it is not wired into the `WorkflowEngine` -> `EffectDriver`
dispatch path.

**Fix**: Wire `MultiAgentPool` into `EffectServices` so `spawn_agent()` acquires
from the pool instead of constructing fresh.

---

## Layer 3: Model Routing and Prompt Assembly

### B06 -- Cascade Router Sync Read (150-300ms)

**Files**:
- `crates/roko-cli/src/orchestrate.rs:14113-14157` -- cascade routing with scoring
- `crates/roko-cli/src/orchestrate.rs:14123` -- `load_efficiency_signals_sync()` reads entire JSONL

**Problem**: Every dispatch in the orchestrator reads the full `efficiency.jsonl` file
(one line per past model call), loops over all candidate models for scoring, and queries
the neuro store per candidate. All synchronous.

**Size scaling**: After 100 runs, `efficiency.jsonl` is ~50KB (500 lines x 100 bytes).
After 1000 runs, ~500KB. Read + parse scales linearly.

**Fix (three parts)**:
1. **Memoize efficiency signals**: Cache parsed signals with 10s TTL.
   ```rust
   static EFFICIENCY_CACHE: LazyLock<Mutex<CachedSignals>> = ...;
   struct CachedSignals {
       signals: Vec<EfficiencySignal>,
       loaded_at: Instant,
   }
   ```
2. **Cache routing decisions**: Key = hash(task_type, complexity_tier, recent_quality_score).
   TTL = 5 minutes. Invalidate on new efficiency signal writes.
3. **Batch knowledge queries**: Instead of N queries (one per candidate model), do one
   query with all model names as keywords.

---

### B07 -- Enrichment Pipeline Sequential (200-500ms)

**Files**:
- `crates/roko-cli/src/orchestrate.rs:8422` -- `EnrichmentPipeline::new()`
- Steps execute serially via `ALL_ORDERED`

**Problem**: The enrichment pipeline runs multiple independent steps sequentially:
1. File intelligence (reads source files for context)
2. Knowledge context (queries neuro store)
3. Wave context (recent execution history)
4. Research context (if available)

Steps 1-4 are independent -- no step depends on the output of another.

**Fix**: Parallelize with `tokio::join!` or `futures::join_all()`:

```rust
let (file_intel, knowledge_ctx, wave_ctx, research_ctx) = tokio::join!(
    enrich_file_intel(&task),
    enrich_knowledge(&task, &neuro_store),
    enrich_wave(&task, &wave_store),
    enrich_research(&task, &research_store),
);
```

Expected savings: max(step_times) instead of sum(step_times) = 50-300ms saved.

---

### B12 -- Prompt Assembly I/O (50-200ms)

**Files**:
- `crates/roko-compose/src/prompt_assembly_service.rs:319-479` -- `assemble()` method
- `crates/roko-compose/src/prompt_assembly_service.rs:669-711` -- `collect_source_context()`

**Problem**: The `PromptAssemblyService::assemble()` method performs multiple I/O operations:
1. `detect_workdir_conventions()` -- reads `Cargo.toml` + up to 12 source files
2. `collect_source_context()` -- walks `src/` directory recursively
3. `EpisodeLogger::read_all()` -- reads entire `episodes.jsonl`
4. `PlaybookStore::query()` -- reads playbook files
5. `KnowledgeStore::query()` -- searches knowledge entries

The source context collection at `collect_source_context_from()` (lines 681-711) does
synchronous `std::fs::read_dir()` and `std::fs::read_to_string()` on up to 12 files.
This blocks the Tokio runtime.

**Fix (two parts)**:
1. **Cache conventions**: Detect once per workdir, cache with file-modification-time
   invalidation. Conventions rarely change during a run.
   ```rust
   static CONVENTIONS_CACHE: LazyLock<Mutex<HashMap<PathBuf, (SystemTime, String)>>> = ...;
   ```
2. **Async file I/O**: Use `tokio::fs` instead of `std::fs` in the hot path.
   The `collect_source_context_from()` function should use `tokio::fs::read_dir()`.

---

### B14 -- Workspace Convention Detection (30-100ms)

**Files**:
- `crates/roko-compose/src/prompt_assembly_service.rs:650-667` -- `detect_workdir_conventions()`
- `crates/roko-compose/src/prompt_assembly_service.rs:669-679` -- `collect_source_context()`

**Problem**: Every prompt assembly call walks the `src/` directory, reads up to 12
source files, and re-detects project conventions. For large projects, the directory
walk alone takes 30-100ms.

**Fix**: Cache the result with mtime-based invalidation. The workspace structure
changes only when files are added/removed/modified, and even then, convention
detection results are stable (language, build system, naming style).

---

## Layer 4: Gate Pipeline

### B08 -- Cargo Compile Gate (500-2000ms)

**Files**:
- `crates/roko-gate/src/gate_service.rs:234-366` -- `run_gates()` main loop
- `crates/roko-gate/src/compile.rs` -- `CompileGate` implementation
- `crates/roko-gate/src/clippy_gate.rs` -- `ClippyGate` implementation

**Problem**: The compile gate runs `cargo check` as a subprocess. Even with
incremental compilation, this takes 500ms+ for the roko workspace. Full cold
builds take 8-15 seconds.

**Cost breakdown**:
```
cargo process spawn:    50ms
Dependency resolution:  100ms
Incremental check:      300-1500ms
Output parsing:         10ms
────────────────────────────────────
Total:                  460-1660ms
```

**Fix (three approaches)**:

1. **Express gate mode**: Define a `GateConfig.fast_path` flag that skips rungs 1-6
   for tasks marked as non-code-modifying (documentation, config changes).
   ```rust
   // In pipeline_state.rs or workflow config:
   pub struct WorkflowConfig {
       pub gate_fast_path: bool,  // skip compile/clippy/test for non-code tasks
   }
   ```

2. **Source hash gate guard**: Before running compile, hash the modified source files.
   If the hash matches the last successful compile, skip the gate entirely.
   ```rust
   fn should_skip_compile(workdir: &Path) -> bool {
       let current_hash = hash_modified_sources(workdir);
       let last_hash = read_last_compile_hash(workdir);
       current_hash == last_hash
   }
   ```

3. **Parallel gate rungs**: Run compile + fmt simultaneously (they are independent).
   Currently, `run_gates()` iterates sequentially over `ordered_gate_names()`.
   Rungs 0 (compile) and 4 (fmt) can run in parallel:
   ```rust
   let (compile_result, fmt_result) = tokio::join!(
       compile_gate.verify(&signal, &ctx),
       fmt_gate.verify(&signal, &ctx),
   );
   ```

**Adaptive threshold bypass**: Already implemented at `gate_service.rs:120-136`.
The system tracks consecutive passes per rung and skips gates with high pass rates.
Rung 0 (compile) is never skipped regardless of thresholds.

---

### B09 -- Git Diff Computed Per Rung (100-300ms)

**Files**:
- `crates/roko-cli/src/orchestrate.rs:16955-16970` -- spawns TWO git commands per rung

**Problem**: For rungs that need a code diff (e.g., diff gate, LLM judge context),
the git diff is computed fresh each time. The orchestrator spawns two git processes:
`git diff --stat` and `git diff HEAD`.

**Fix**: Compute the diff once at the start of the gate phase, store in memory,
and pass through to all gates that need it.

```rust
struct GatePhaseContext {
    diff_stat: String,
    diff_full: String,
    modified_files: Vec<PathBuf>,
}

impl GatePhaseContext {
    async fn compute(workdir: &Path) -> Self {
        let (stat, full) = tokio::join!(
            git_diff_stat(workdir),
            git_diff_full(workdir),
        );
        Self {
            diff_stat: stat,
            diff_full: full,
            modified_files: parse_modified_files(&stat),
        }
    }
}
```

---

## Layer 5: Persistence and Feedback

### B10 -- Sequential Substrate Writes (50-100ms)

**Files**:
- `crates/roko-cli/src/run.rs:924-1050` -- 10+ `substrate.put()` calls in series
- `crates/roko-fs/src/` -- `FileSubstrate` JSONL implementation

**Problem**: After each agent run, 10+ signals are written to the substrate one at a
time. Each `put()` call: serialize JSON -> append to JSONL file -> flush.

**Breakdown**:
```
Per put():
  serde_json::to_string():  1ms
  File append:              3ms
  flush():                  4ms
  ────────────────────
  Total:                    8ms x 10 = 80ms
```

**Fix**: Batch writes -- collect all signals, serialize once, single append + flush.

```rust
fn batch_put(substrate: &FileSubstrate, signals: Vec<Engram>) -> Result<()> {
    let mut buffer = String::new();
    for signal in &signals {
        let json = serde_json::to_string(signal)?;
        buffer.push_str(&json);
        buffer.push('\n');
    }
    substrate.append_raw(&buffer)?;  // single write + flush
    Ok(())
}
```

---

### B11 -- Feedback Flush Synchronous (30-50ms)

**Files**:
- `crates/roko-learn/src/feedback_service.rs:92-170` -- `flush()` with Mutex lock
- `crates/roko-runtime/src/jsonl_logger.rs:62-86` -- `write_event()` flushes per event

**Problem**: The `JsonlLogger` at `crates/roko-runtime/src/jsonl_logger.rs:62-86`
calls `w.flush()` on every single event write. For a typical run with 20-30 runtime
events, this adds 60-150ms of synchronous disk I/O.

```rust
// crates/roko-runtime/src/jsonl_logger.rs:82-84
if let Some(ref mut w) = *writer {
    writeln!(w, "{json}")?;
    w.flush()?;  // FLUSH EVERY EVENT
}
```

**Fix**: Buffer events and flush periodically or at run completion.

```rust
// Option 1: BufWriter with larger buffer (defers flush to OS)
let file = std::fs::OpenOptions::new().append(true).open(&self.path)?;
let writer = std::io::BufWriter::with_capacity(8192, file);
// Remove explicit flush() -- let BufWriter handle it

// Option 2: Periodic flush (every 100 events or 5 seconds)
struct BatchingLogger {
    buffer: Vec<RuntimeEventEnvelope>,
    last_flush: Instant,
}
```

---

### B13 -- Event Bus Per-Event Serialization (20-40ms)

**Files**:
- `crates/roko-runtime/src/event_bus.rs` -- `emit_runtime_event()`
- `crates/roko-runtime/src/jsonl_logger.rs:72` -- `serde_json::to_string(&envelope)`

**Problem**: Every runtime event is serialized to JSON immediately upon emission,
even if no consumer is listening. The serialization cost is ~1ms per event, and a
typical run emits 20-40 events.

**Fix**: Lazy serialization -- serialize only when a consumer requests it.

```rust
// Instead of:
let json = serde_json::to_string(&envelope)?;
writeln!(w, "{json}")?;

// Use:
enum EventPayload {
    Raw(RuntimeEventEnvelope),
    Serialized(String),
}

impl EventPayload {
    fn to_json(&self) -> String {
        match self {
            Self::Raw(env) => serde_json::to_string(env).unwrap(),
            Self::Serialized(s) => s.clone(),
        }
    }
}
```

---

## Optimization Impact Matrix

### Fast API Path (no gates, express workflow)

```
CURRENT (measured, gpt-4.1-nano baseline):
  Config load (4x):    40ms
  Learning init (2x):  140ms
  Agent construct:     30ms
  Prompt assembly:     50ms  (convention detection + knowledge query)
  HTTP request:        300ms (network-bound, US endpoint)
  Persistence:         80ms  (10 writes)
  Feedback flush:      40ms
  Event serialization: 30ms
  ─────────────────────────
  TOTAL:               ~710ms  (excluding inference)
  WITH inference:      ~1010ms

AFTER PHASE 0 (B02, B03, B05, B13):
  Config load (1x):    10ms
  Learning init (1x):  70ms
  Agent construct:     10ms  (cached contract)
  Prompt assembly:     50ms
  HTTP request:        300ms
  Persistence:         80ms
  Feedback:            40ms
  Events:              15ms  (lazy)
  ─────────────────────────
  TOTAL:               ~575ms
  Savings:             135ms

AFTER PHASE 1 (+ B10, B11, B12, B14):
  Config:              10ms
  Learning:            70ms
  Agent:               10ms
  Prompt assembly:     20ms  (cached conventions)
  HTTP request:        300ms
  Persistence:         20ms  (batched)
  Feedback:            5ms   (async)
  Events:              10ms
  ─────────────────────────
  TOTAL:               ~445ms
  Savings:             565ms

AFTER PHASE 2 (+ B06, B07, B15):
  Config:              10ms
  Learning:            70ms
  Agent:               5ms   (warm pool)
  Routing:             10ms  (cached)
  Prompt:              15ms  (cached + parallel enrichment)
  HTTP request:        300ms
  Persistence:         20ms
  Feedback:            5ms
  Events:              10ms
  ─────────────────────────
  TOTAL:               ~445ms  (routing/prompt overlap)
  Additional savings:  ~100ms from parallelization
```

### Full Pipeline Path (standard workflow, with gates)

```
CURRENT:              ~710ms overhead + 2x inference + 500-2000ms gates
                      = ~3710-5210ms (fast model)

AFTER ALL PHASES:     ~345ms overhead + 2x inference + 200ms express gate
                      = ~1845ms (fast model)

SAVINGS:              1865-3365ms (50-65% reduction)
```

---

## Priority Implementation Order

### Phase 0 -- Immediate wins (2-3h, saves ~135ms)
1. **B02**: Config caching (load once, `Arc<RokoConfig>` through call chain)
2. **B03**: LearningRuntime pass-through (stop re-opening)
3. **B05**: Contract caching (`OnceLock<AgentContract>` per role)
4. **B13**: Lazy event serialization

### Phase 1 -- Persistence optimization (3-4h, saves ~430ms)
5. **B10**: Batch substrate writes (single append + flush)
6. **B11**: Async feedback flush (remove per-event flush)
7. **B12**: Prompt assembly caching (conventions + source context)
8. **B14**: Workspace convention cache (mtime invalidation)

### Phase 2 -- Routing and dispatch (4-6h, saves ~200ms)
9. **B06**: Memoize efficiency signals + routing cache
10. **B07**: Parallelize enrichment pipeline
11. **B15**: Wire `MultiAgentPool` into `EffectDriver`

### Phase 3 -- Gate pipeline (6-8h, saves 300-1800ms for applicable tasks)
12. **B08**: Express gate mode (skip compile for non-code tasks)
13. **B08**: Source hash gate guard (skip if unchanged)
14. **B08**: Parallel gate rungs (compile + fmt concurrent)
15. **B09**: Git diff caching (compute once per gate phase)

### Phase 4 -- Architecture (8-16h, saves 200-500ms for CLI path)
16. **B04**: Agent warm pool for Claude CLI (or route through API)

---

## Measurement Methodology

### Instrumentation points

Add `tracing::instrument` spans at each bottleneck location:

```rust
#[tracing::instrument(skip_all, fields(phase = "config_load"))]
fn load_config(workdir: &Path) -> Result<Config> { ... }

#[tracing::instrument(skip_all, fields(phase = "agent_construct"))]
fn create_agent_for_model(config: &ModelConfig) -> Result<Box<dyn Agent>> { ... }
```

### Benchmark harness

```bash
# Micro-benchmark: config load only
cargo bench --bench config_load

# Macro-benchmark: full run, no gates
RUST_LOG=roko=trace cargo run --release -p roko-cli -- run \
  --model gpt-4.1-nano --workflow-template express --gates none \
  "echo hello" 2>&1 | grep -E 'phase|duration'

# Profiling with flamegraph
cargo flamegraph --bin roko -- run --model gpt-4.1-nano "echo hello"
```

### Regression detection

Store timing results in `.roko/bench/perf/` after each commit. Compare P50 and P99
latencies. Alert if any phase regresses by >20%.
