# Performance Optimization Playbook

Date: 2026-04-29
Scope: Concrete implementation plans for each optimization, with file paths, code
patterns, testing strategy, and rollback plans.

---

## Table of Contents

1. [Shared Config Cache](#1-shared-config-cache)
2. [LearningRuntime Single-Open](#2-learningruntime-single-open)
3. [Contract Caching](#3-contract-caching)
4. [Batch Substrate Writes](#4-batch-substrate-writes)
5. [Async Feedback Flush](#5-async-feedback-flush)
6. [Prompt Assembly Cache](#6-prompt-assembly-cache)
7. [Routing Cache](#7-routing-cache)
8. [Parallel Enrichment](#8-parallel-enrichment)
9. [Express Gate Mode](#9-express-gate-mode)
10. [Git Diff Cache](#10-git-diff-cache)
11. [Lazy Event Serialization](#11-lazy-event-serialization)
12. [Speculative Execution](#12-speculative-execution)
13. [Batch Inference](#13-batch-inference)
14. [Profile-Guided Optimization](#14-profile-guided-optimization)

---

## 1. Shared Config Cache

**Bottleneck**: B02 (10-50ms saved)
**Effort**: 2h
**Risk**: Low

### Problem

Config is loaded 4+ times per `roko run`. Each load parses `roko.toml` from disk.

### Files to modify

| File | Change |
|------|--------|
| `crates/roko-cli/src/run.rs` | Load once, pass `Arc<RokoConfig>` |
| `crates/roko-cli/src/orchestrate.rs` | Accept `Arc<RokoConfig>` parameter |
| `crates/roko-cli/src/model_selection.rs` | Accept reference instead of loading |

### Implementation

```rust
// crates/roko-cli/src/run.rs -- entry point
pub async fn run_once(opts: &RunOpts) -> Result<()> {
    let config = Arc::new(load_layered(&opts.workdir)?);
    // Pass config to all consumers
    let result = dispatch_agent(&config, &opts.prompt, &opts.model).await?;
    append_episode_log(&config, &result).await?;
    Ok(())
}
```

### Test plan

1. Unit test: Load config once, verify `Arc::strong_count()` == 1 after all consumers drop
2. Integration test: `roko run` with `RUST_LOG=roko_cli=trace` -- verify single "loading config" log line
3. Regression test: Verify `--model` override still works after caching

### Rollback

If config caching causes stale-config bugs (e.g., hot-reload in `roko serve`), add
a `config.reload()` method that re-reads from disk and updates the `Arc` via a
`watch::Sender`.

---

## 2. LearningRuntime Single-Open

**Bottleneck**: B03 (70-100ms saved)
**Effort**: 1h
**Risk**: Low

### Problem

`LearningRuntime::open_under()` is called twice -- once in dispatch and once in
episode logging. Each open reads 3 JSON files and spawns a distillation task.

### Files to modify

| File | Change |
|------|--------|
| `crates/roko-cli/src/run.rs` | Pass `LearningRuntime` to `append_episode_log()` |
| `crates/roko-learn/src/lib.rs` | No change needed |

### Implementation

```rust
// Before:
let lr = LearningRuntime::open_under(&roko_dir)?;
// ... dispatch ...
append_episode_log(&workdir, episode)?;  // opens lr again inside

// After:
let lr = LearningRuntime::open_under(&roko_dir)?;
// ... dispatch ...
append_episode_log_with(&lr, episode)?;  // reuses existing lr
```

### Test plan

1. Unit test: Mock LearningRuntime, verify `open_under` called exactly once
2. Benchmark: Measure wall-clock time before/after with `criterion`

---

## 3. Contract Caching

**Bottleneck**: B05 (10-50ms saved per multi-tool turn)
**Effort**: 1h
**Risk**: Low

### Problem

Safety contracts are loaded from YAML on every tool dispatch. A 10-tool turn
means 10 redundant file reads.

### Files to modify

| File | Change |
|------|--------|
| `crates/roko-agent/src/safety/` | Add `OnceLock` contract cache per role |

### Implementation

```rust
use std::sync::OnceLock;

struct ContractCache {
    contracts: Mutex<HashMap<String, Arc<AgentContract>>>,
}

impl ContractCache {
    fn get_or_load(&self, role: &str) -> Arc<AgentContract> {
        let mut cache = self.contracts.lock().unwrap();
        cache
            .entry(role.to_string())
            .or_insert_with(|| {
                Arc::new(load_contract_for_role(role).unwrap_or_default())
            })
            .clone()
    }
}

// In ToolDispatcher:
static CONTRACT_CACHE: LazyLock<ContractCache> = LazyLock::new(|| ContractCache::new());
```

### Test plan

1. Unit test: Two dispatches for same role -> contract loaded once
2. Unit test: Different roles -> different contracts loaded
3. File modification test: Verify stale contract does not cause security issue
   (contracts are immutable during a run; restarting roko clears the cache)

---

## 4. Batch Substrate Writes

**Bottleneck**: B10 (60ms saved)
**Effort**: 2h
**Risk**: Medium (data integrity on crash)

### Problem

10+ `substrate.put()` calls each serialize + append + flush individually.

### Files to modify

| File | Change |
|------|--------|
| `crates/roko-fs/src/` | Add `batch_put()` method to FileSubstrate |
| `crates/roko-cli/src/run.rs` | Use `batch_put()` instead of individual puts |

### Implementation

```rust
// crates/roko-fs/src/lib.rs
impl FileSubstrate {
    /// Write multiple signals in a single I/O operation.
    pub fn batch_put(&self, signals: &[Engram]) -> Result<()> {
        let mut buffer = String::with_capacity(signals.len() * 256);
        for signal in signals {
            let json = serde_json::to_string(signal)?;
            buffer.push_str(&json);
            buffer.push('\n');
        }
        self.append_raw(&buffer)
    }

    fn append_raw(&self, data: &str) -> Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        file.write_all(data.as_bytes())?;
        file.flush()?;
        Ok(())
    }
}
```

### Crash safety

If the process crashes mid-write, the JSONL file may have a partial last line.
The reader already handles this by ignoring lines that fail JSON parsing.
Add a test to verify this:

```rust
#[test]
fn reader_ignores_partial_last_line() {
    let mut file = File::create(&path).unwrap();
    writeln!(file, r#"{{"valid":"line"}}"#).unwrap();
    write!(file, r#"{{"partial":"no newl"#).unwrap(); // truncated
    let entries = read_jsonl(&path).unwrap();
    assert_eq!(entries.len(), 1); // only the complete line
}
```

---

## 5. Async Feedback Flush

**Bottleneck**: B11 (35ms saved)
**Effort**: 2h
**Risk**: Low (events may be lost on crash, but they are advisory)

### Problem

`JsonlLogger` flushes on every single event. For 20-30 events per run, this
is 60-150ms of synchronous disk I/O.

### Files to modify

| File | Change |
|------|--------|
| `crates/roko-runtime/src/jsonl_logger.rs` | Remove per-event flush, add periodic flush |

### Implementation

```rust
// Option A: Increase BufWriter capacity, flush on drop
impl JsonlLogger {
    fn ensure_writer(&self) -> std::io::Result<()> {
        let mut writer = self.writer.lock().unwrap_or_else(|e| e.into_inner());
        if writer.is_none() {
            let file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)?;
            // 8KB buffer instead of default 8B
            *writer = Some(std::io::BufWriter::with_capacity(8192, file));
        }
        Ok(())
    }

    fn write_event(&self, event: &RuntimeEvent) -> std::io::Result<()> {
        self.ensure_writer()?;
        let envelope = RuntimeEventEnvelope::new(/* ... */);
        let json = serde_json::to_string(&envelope)?;
        let mut writer = self.writer.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(ref mut w) = *writer {
            writeln!(w, "{json}")?;
            // NO flush() here -- let BufWriter handle it
        }
        Ok(())
    }
}

// Explicit flush on run completion
impl JsonlLogger {
    pub fn flush(&self) -> std::io::Result<()> {
        let mut writer = self.writer.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(ref mut w) = *writer {
            w.flush()?;
        }
        Ok(())
    }
}
```

### Test plan

1. Write 100 events without explicit flush
2. Verify all 100 are readable after `flush()` call
3. Verify data survives process exit (BufWriter flushes on Drop)

---

## 6. Prompt Assembly Cache

**Bottleneck**: B12 + B14 (50-150ms saved)
**Effort**: 3h
**Risk**: Medium (stale conventions after file edits)

### Problem

`PromptAssemblyService::assemble()` reads Cargo.toml, walks src/, reads 12 source
files, and detects conventions on every call. For the same workdir, these results
are stable within a run.

### Files to modify

| File | Change |
|------|--------|
| `crates/roko-compose/src/prompt_assembly_service.rs` | Add convention cache |

### Implementation

```rust
use std::time::SystemTime;

struct ConventionCache {
    workdir: PathBuf,
    conventions: String,
    mtime: SystemTime,
}

impl PromptAssemblyService {
    fn cached_conventions(&self, workdir: &Path) -> Option<String> {
        let mut cache = self.convention_cache.lock().unwrap();

        // Check if cache is valid (same workdir, Cargo.toml not modified)
        if let Some(ref entry) = *cache {
            if entry.workdir == workdir {
                let current_mtime = std::fs::metadata(workdir.join("Cargo.toml"))
                    .and_then(|m| m.modified())
                    .ok();
                if current_mtime == Some(entry.mtime) {
                    return Some(entry.conventions.clone());
                }
            }
        }

        // Cache miss: detect fresh and store
        let conventions = detect_workdir_conventions(workdir)?;
        let mtime = std::fs::metadata(workdir.join("Cargo.toml"))
            .and_then(|m| m.modified())
            .ok()?;
        *cache = Some(ConventionCache {
            workdir: workdir.to_path_buf(),
            conventions: conventions.clone(),
            mtime,
        });
        Some(conventions)
    }
}
```

### Invalidation strategy

- Cache key: `(workdir, Cargo.toml mtime)`
- Cache invalidated when: Cargo.toml is modified, or workdir changes
- TTL: No timeout -- mtime comparison is sufficient
- Scope: Per `PromptAssemblyService` instance (one per run or per serve session)

---

## 7. Routing Cache

**Bottleneck**: B06 (100-200ms saved)
**Effort**: 3h
**Risk**: Medium (stale routing decisions)

### Problem

Cascade router reads efficiency.jsonl, scores all candidates, and queries neuro
store on every dispatch. For sequential tasks in a plan, results are identical.

### Files to modify

| File | Change |
|------|--------|
| `crates/roko-cli/src/orchestrate.rs` | Memoize efficiency signals + routing decisions |

### Implementation

```rust
struct RoutingCache {
    efficiency_signals: Option<(Vec<EfficiencySignal>, Instant)>,
    routing_decisions: HashMap<u64, (String, Instant)>,  // hash -> (model, cached_at)
    ttl: Duration,
}

impl RoutingCache {
    fn get_efficiency_signals(&mut self) -> &[EfficiencySignal] {
        if self.efficiency_signals.as_ref().map_or(true, |(_, t)| t.elapsed() > self.ttl) {
            let signals = load_efficiency_signals_sync();
            self.efficiency_signals = Some((signals, Instant::now()));
        }
        &self.efficiency_signals.as_ref().unwrap().0
    }

    fn get_routing_decision(&mut self, key: u64) -> Option<&str> {
        self.routing_decisions.get(&key).and_then(|(model, t)| {
            if t.elapsed() < self.ttl {
                Some(model.as_str())
            } else {
                None
            }
        })
    }
}
```

### Cache key computation

```rust
fn routing_cache_key(task_type: &str, complexity: u8, recent_quality: f64) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    task_type.hash(&mut hasher);
    complexity.hash(&mut hasher);
    (recent_quality * 100.0) as u32).hash(&mut hasher);
    hasher.finish()
}
```

---

## 8. Parallel Enrichment

**Bottleneck**: B07 (100-300ms saved)
**Effort**: 2h
**Risk**: Low

### Problem

Enrichment steps run sequentially even though they are independent.

### Files to modify

| File | Change |
|------|--------|
| `crates/roko-cli/src/orchestrate.rs` | Use `tokio::join!` for independent steps |

### Implementation

```rust
// Before:
let file_intel = enrich_file_intel(&task).await?;
let knowledge = enrich_knowledge(&task, &store).await?;
let wave = enrich_wave(&task).await?;
let research = enrich_research(&task).await?;

// After:
let (file_intel, knowledge, wave, research) = tokio::join!(
    enrich_file_intel(&task),
    enrich_knowledge(&task, &store),
    enrich_wave(&task),
    enrich_research(&task),
);
let file_intel = file_intel?;
let knowledge = knowledge?;
let wave = wave?;
let research = research?;
```

### Expected timing

```
Sequential: 50 + 80 + 30 + 40 = 200ms
Parallel:   max(50, 80, 30, 40) = 80ms
Savings:    120ms
```

---

## 9. Express Gate Mode

**Bottleneck**: B08 (500-2000ms saved for applicable tasks)
**Effort**: 4h
**Risk**: Medium (skipping gates reduces safety)

### Problem

All tasks run through the full gate pipeline, even when the task does not modify
code (e.g., documentation changes, config updates, research queries).

### Files to modify

| File | Change |
|------|--------|
| `crates/roko-runtime/src/pipeline_state.rs` | Add `gate_mode` to WorkflowConfig |
| `crates/roko-runtime/src/effect_driver.rs` | Respect gate_mode in `run_gates()` |
| `crates/roko-gate/src/gate_service.rs` | Add express mode support |
| `crates/roko-cli/src/main.rs` | Add `--gates` CLI flag |

### Implementation

```rust
// crates/roko-runtime/src/pipeline_state.rs
pub enum GateMode {
    /// Run all configured gates.
    Full,
    /// Run only lightweight gates (diff, fmt). Skip compile/test/clippy.
    Express,
    /// Skip all gates.
    None,
    /// Auto-detect based on changed files.
    Auto,
}

impl WorkflowConfig {
    pub fn with_gate_mode(mut self, mode: GateMode) -> Self {
        self.gate_mode = mode;
        self
    }
}
```

Auto-detection logic:
```rust
fn detect_gate_mode(workdir: &Path) -> GateMode {
    let diff = git_diff_stat(workdir);
    let modified_files = parse_modified_files(&diff);

    let has_code = modified_files.iter().any(|f|
        f.ends_with(".rs") || f.ends_with(".ts") || f.ends_with(".py")
    );
    let has_config = modified_files.iter().any(|f|
        f.ends_with(".toml") || f.ends_with(".json") || f.ends_with(".yaml")
    );
    let has_docs = modified_files.iter().any(|f|
        f.ends_with(".md") || f.ends_with(".txt")
    );

    if has_code { GateMode::Full }
    else if has_config { GateMode::Express }
    else { GateMode::None }
}
```

---

## 10. Git Diff Cache

**Bottleneck**: B09 (50-200ms saved)
**Effort**: 1h
**Risk**: Low

### Problem

Git diff is computed multiple times during the gate phase.

### Files to modify

| File | Change |
|------|--------|
| `crates/roko-cli/src/orchestrate.rs` | Compute diff once, pass through |

### Implementation

```rust
struct GateContext {
    diff_stat: String,
    diff_full: String,
    modified_files: Vec<String>,
    computed_at: Instant,
}

impl GateContext {
    async fn compute(workdir: &Path) -> Self {
        let (stat_out, full_out) = tokio::join!(
            tokio::process::Command::new("git")
                .args(["diff", "--stat", "HEAD"])
                .current_dir(workdir)
                .output(),
            tokio::process::Command::new("git")
                .args(["diff", "HEAD"])
                .current_dir(workdir)
                .output(),
        );
        // ... parse outputs ...
        Self { diff_stat, diff_full, modified_files, computed_at: Instant::now() }
    }
}
```

---

## 11. Lazy Event Serialization

**Bottleneck**: B13 (15-25ms saved)
**Effort**: 1h
**Risk**: Low

### Problem

Runtime events are serialized to JSON on every `emit_runtime_event()`, even when
no consumer is listening.

### Files to modify

| File | Change |
|------|--------|
| `crates/roko-runtime/src/event_bus.rs` | Lazy serialization |
| `crates/roko-runtime/src/jsonl_logger.rs` | Accept pre-serialized or raw events |

### Implementation

Events are cheap to clone (they are small enums). Serialization should happen only
in the logger consumer, not at emit time.

```rust
// event_bus.rs already passes the event by reference to consumers.
// The logger should serialize on demand, not eagerly.
// This is already the case -- the JsonlLogger receives &RuntimeEvent
// and serializes in write_event().
//
// The actual cost is in RuntimeEventEnvelope::new() which allocates
// timestamp + source strings. Optimize by using static strings:

fn write_event(&self, event: &RuntimeEvent) -> std::io::Result<()> {
    // Use a thread-local buffer to avoid allocation
    thread_local! {
        static BUFFER: RefCell<String> = RefCell::new(String::with_capacity(512));
    }

    BUFFER.with(|buf| {
        let mut buf = buf.borrow_mut();
        buf.clear();
        // Direct serialization into buffer
        serde_json::to_writer(unsafe { buf.as_mut_vec() }, &envelope)?;
        buf.push('\n');
        // Write to file
        writer.write_all(buf.as_bytes())?;
        Ok(())
    })
}
```

---

## 12. Speculative Execution

**Bottleneck**: Novel optimization (saves 20-50ms for standard workflow)
**Effort**: 6h
**Risk**: Medium (wasted work if speculation fails)

### Concept

In the standard workflow (implement -> gate -> review), the reviewer agent will
always be needed after gates pass. While the implementer is running, speculatively
pre-warm the reviewer's model caller so it is ready instantly.

### Where to implement

**File**: `crates/roko-runtime/src/workflow_engine.rs`

### Trigger conditions

1. Standard or full workflow template
2. Implementer currently running
3. Warm pool has idle capacity

### Implementation sketch

```rust
// In the workflow engine run loop:
match output {
    PipelineOutput::SpawnImplementer { .. } => {
        // Start implementer
        let impl_fut = driver.spawn_agent("implementer", prompt, context);

        // Speculatively pre-warm reviewer (if standard/full workflow)
        if config.workflow.has_review {
            if let Some(ref pool) = driver.services.warm_pool {
                let _ = pool.pre_warm_for("reviewer_provider", "reviewer_model").await;
            }
        }

        let result = impl_fut.await;
        // ...
    }
}
```

### Cost/benefit analysis

- **Cost**: One warm slot occupied during implementation (~1-30s)
- **Benefit**: 20-50ms saved on reviewer acquisition
- **Waste case**: If implementation fails, the pre-warmed reviewer is unused (evicted after idle timeout)

---

## 13. Batch Inference

**Bottleneck**: Novel optimization (saves 30-50% on multi-task plans)
**Effort**: 8h
**Risk**: High (requires provider-specific batch APIs)

### Concept

For plan execution with multiple independent tasks, batch inference requests to
the same provider into a single API call.

### Provider support

| Provider | Batch API | Max Batch Size | Latency Benefit |
|---|---|---|---|
| OpenAI | /v1/chat/completions (N=1 only) | No batch | None |
| OpenAI Batch | /v1/batches | 50,000 | 50% cost, async (24h) |
| Anthropic | /v1/messages/batches | 10,000 | 50% cost, async |
| Gemini | BatchPredict | 300 | ~30% latency |
| Cerebras | No batch API | N/A | None |

### Implementation strategy

For **real-time** batch inference (not async batch APIs):

```rust
/// Collect pending inference requests and dispatch together.
struct BatchCollector {
    pending: Vec<(ModelCallRequest, oneshot::Sender<Result<ModelCallResponse>>)>,
    provider: String,
    batch_window: Duration,  // e.g., 50ms
}

impl BatchCollector {
    /// Submit a request for batching.
    async fn submit(&self, request: ModelCallRequest) -> Result<ModelCallResponse> {
        let (tx, rx) = oneshot::channel();
        self.pending.push((request, tx));

        // If batch window elapsed or batch full, flush
        if self.should_flush() {
            self.flush().await;
        }

        rx.await?
    }

    async fn flush(&mut self) {
        let batch = std::mem::take(&mut self.pending);
        // Send all requests concurrently (not a true batch API, but parallel dispatch)
        let futures = batch.into_iter().map(|(req, tx)| async move {
            let result = self.caller.call(req).await;
            let _ = tx.send(result);
        });
        futures::future::join_all(futures).await;
    }
}
```

For **async** batch APIs (OpenAI/Anthropic batch endpoints):

```rust
// Submit batch, poll for completion
async fn batch_inference(
    requests: Vec<ModelCallRequest>,
    provider: &str,
) -> Vec<Result<ModelCallResponse>> {
    match provider {
        "openai" => openai_batch_submit_and_poll(requests).await,
        "anthropic" => anthropic_batch_submit_and_poll(requests).await,
        _ => {
            // Fallback to parallel individual requests
            let futures = requests.into_iter().map(|req| caller.call(req));
            futures::future::join_all(futures).await
        }
    }
}
```

### Use case

Best suited for `roko plan run` with many independent tasks. Not useful for
single `roko run` (only 1-2 inference calls).

---

## 14. Profile-Guided Optimization (PGO)

**Bottleneck**: Compiler optimization (~5-15% total improvement)
**Effort**: 4h
**Risk**: Low

### Concept

Use Rust's profile-guided optimization to build a `roko` binary that is optimized
for the actual execution profile.

### Steps

```bash
# 1. Build instrumented binary
RUSTFLAGS="-Cprofile-generate=/tmp/pgo-data" \
  cargo build --release -p roko-cli

# 2. Run representative workloads
for i in $(seq 1 10); do
  ./target/release/roko run --model gpt-4.1-nano --gates none "echo hello"
  ./target/release/roko plan validate plans/
  ./target/release/roko config show
done

# 3. Merge profiles
llvm-profdata merge -o /tmp/pgo-data/merged.profdata /tmp/pgo-data

# 4. Rebuild with PGO data
RUSTFLAGS="-Cprofile-use=/tmp/pgo-data/merged.profdata" \
  cargo build --release -p roko-cli

# 5. Benchmark
time ./target/release/roko run --model gpt-4.1-nano --gates none "echo hello"
```

### Expected impact

PGO typically improves:
- Branch prediction accuracy -> fewer pipeline stalls
- Function inlining decisions -> less call overhead
- Cache layout -> fewer L1/L2 misses

For I/O-bound workloads like roko, expect 5-10% improvement on the non-network
portions of the code (config parsing, JSON serialization, prompt assembly).

### CI integration

Add a PGO build target to the release workflow:

```yaml
# .github/workflows/release.yml
- name: PGO build
  run: |
    RUSTFLAGS="-Cprofile-generate=$RUNNER_TEMP/pgo" cargo build --release -p roko-cli
    ./target/release/roko config show
    ./target/release/roko plan validate plans/test/
    llvm-profdata merge -o $RUNNER_TEMP/pgo/merged.profdata $RUNNER_TEMP/pgo
    RUSTFLAGS="-Cprofile-use=$RUNNER_TEMP/pgo/merged.profdata" cargo build --release -p roko-cli
```

---

## Implementation Timeline

### Week 1: Low-hanging fruit (Phase 0)

| Day | Task | Bottleneck | Savings |
|-----|------|-----------|---------|
| Mon | Config cache | B02 | 30ms |
| Mon | LearningRuntime single-open | B03 | 70ms |
| Tue | Contract caching | B05 | 30ms |
| Tue | Lazy event serialization | B13 | 20ms |
| Wed | Test + benchmark | -- | -- |
| **Total** | | | **150ms** |

### Week 2: Persistence (Phase 1)

| Day | Task | Bottleneck | Savings |
|-----|------|-----------|---------|
| Thu | Batch substrate writes | B10 | 60ms |
| Thu | Async feedback flush | B11 | 35ms |
| Fri | Prompt assembly cache | B12+B14 | 100ms |
| **Total** | | | **195ms** |

### Week 3: Routing + Gates (Phase 2-3)

| Day | Task | Bottleneck | Savings |
|-----|------|-----------|---------|
| Mon | Routing cache | B06 | 150ms |
| Tue | Parallel enrichment | B07 | 120ms |
| Wed | Express gate mode | B08 | 500-2000ms |
| Thu | Git diff cache | B09 | 100ms |
| Fri | Wire warm pool | B15 | 30ms |
| **Total** | | | **900-2400ms** |

### Week 4: Advanced (Phase 3-4)

| Day | Task | Bottleneck | Savings |
|-----|------|-----------|---------|
| Mon-Tue | Warm dispatch pool | B04/B15 | 40ms |
| Wed | Speculative execution | Novel | 30ms |
| Thu | PGO build | Novel | 5-10% |
| Fri | Benchmark + document | -- | -- |
| **Total** | | | **70ms + 5-10%** |

### Cumulative savings

```
After Week 1:  150ms saved  (710ms -> 560ms, no gates)
After Week 2:  345ms saved  (710ms -> 365ms, no gates)
After Week 3:  1245-2745ms  (includes gate savings)
After Week 4:  1315-2815ms  (+ 5-10% PGO)
```
