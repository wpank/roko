# Roko Performance Implementation Plans -- INDEX

This folder contains **self-contained, fully-detailed implementation plans** for
every optimization called out in the parent `tmp/solutions/perf/` research
documents (`BENCHMARK-RESULTS.md`, `BOTTLENECK-ANALYSIS.md`,
`OPTIMIZATION-PLAYBOOK.md`, `WARM-POOL-DESIGN.md`,
`HAL-AND-AGENT-BENCHMARKS.md`, `HAL-BENCHMARK-INTEGRATION.md`).

Each document is written so that **a fresh agent with no prior context** can
read it, find the right files, follow the steps, and ship the change. Every
plan ends with a verification checklist and a list of anti-patterns to avoid.

---

## How to use this folder

1. Read this INDEX top-to-bottom to understand sequencing and dependencies.
2. Pick the next plan that is **status: PENDING**.
3. Open the corresponding `NN-<name>.md` file. Each plan is structured as:
   - **Goal & success criteria** (what "done" means).
   - **Background** (links to research docs and the current code paths).
   - **Files to read first** (orient before editing).
   - **Concrete code-level plan** (what to change, with line-anchored snippets).
   - **Step-by-step execution** (numbered, copy-pasteable).
   - **Anti-patterns / things NOT to do** (avoid common pitfalls).
   - **Test plan** (unit, integration, benchmark).
   - **Rollback plan** (how to revert safely).
   - **Status checks** (acceptance criteria).
4. Tick the plan off in this INDEX when merged.

> **Important.** Many of the items in `OPTIMIZATION-PLAYBOOK.md` and
> `BOTTLENECK-ANALYSIS.md` were written from older snapshots of the codebase.
> A handful are already implemented (B01 shared HTTP client, B05 contract
> cache, `FileSubstrate::put_batch`). The plans below explicitly call out
> what is already shipped and what remains so you do not waste time on a
> fix that is already merged.

---

## Phase ordering (read in this order)

| # | Plan | Bottleneck IDs | Effort | Savings | Status |
|---|------|----------------|--------|---------|--------|
| 01 | [Shared config cache](./01-shared-config-cache.md) | B02 | 2 h | 30 ms / run | Pending |
| 02 | [LearningRuntime single-open](./02-learning-runtime-single-open.md) | B03 | 1 h | 70-100 ms / run | Pending |
| 03 | [Contract cache audit & ContractLoadMode plumbing](./03-contract-cache-audit.md) | B05 | 1 h | 30-50 ms / multi-tool turn | Audit (already 90% done) |
| 04 | [Buffered JSONL event logger](./04-buffered-jsonl-logger.md) | B11, B13 | 2 h | 35-60 ms / run | Pending |
| 05 | [Adopt `FileSubstrate::put_batch` everywhere](./05-batch-substrate-writes.md) | B10 | 2 h | 60 ms / run | Audit (mostly done) |
| 06 | [PromptAssemblyService convention cache](./06-prompt-assembly-cache.md) | B12, B14 | 3 h | 50-150 ms / run | Pending |
| 07 | [Routing decision cache](./07-routing-cache.md) | B06 | 4 h | 100-200 ms / dispatch | Pending |
| 08 | [Parallel **independent** enrichment phases](./08-parallel-enrichment.md) | B07 | 3 h | 100-300 ms / plan | Pending (with caveats) |
| 09 | [WarmDispatchPool wired into `EffectDriver`](./09-warm-dispatch-pool.md) | B15, B04 | 10-12 h | 20-50 ms / dispatch, 200-500 ms / claude-cli | Pending |
| 10 | [Express gate mode](./10-express-gate-mode.md) | B08 | 4 h | 500-2000 ms / non-code task | Pending |
| 11 | [Source-hash gate skip](./11-source-hash-gate-skip.md) | B08 | 3 h | 500-1500 ms / unchanged-source run | Pending |
| 12 | [Parallel gate rungs (compile + fmt)](./12-parallel-gate-rungs.md) | B08 | 3 h | 200-500 ms / standard run | Pending |
| 13 | [Git diff cache for gate phase](./13-git-diff-cache.md) | B09 | 1 h | 50-200 ms / run | Pending |
| 14 | [Speculative reviewer pre-warm](./14-speculative-execution.md) | novel | 6 h | 20-50 ms / standard run | Pending |
| 15 | [Batch inference for plan execution](./15-batch-inference.md) | novel | 8 h | 30-50 % on 10+ task plans | Pending |
| 16 | [PGO release build](./16-pgo-build.md) | compiler | 4 h | 5-10 % overall | Pending |
| 17 | [HAL agent wrapper integration](./17-hal-integration.md) | external | 17-20 h | quality eval | Pending |
| 18 | [Bench suite extensions (consistency, cost, Pareto)](./18-bench-suite-extension.md) | external | 8-12 h | quality eval | Pending |

**Cumulative target:** sub-500 ms wall-clock for `roko run` with a fast US API
model and `--gates none`; sub-2 s for the standard workflow with express gates.
See `BENCHMARK-RESULTS.md` §8 for the full projection.

---

## Cross-cutting principles

These rules apply to every plan in this folder.

### Performance hygiene

1. **Measure first, change second.** Before any optimization, capture a
   baseline via the macro-benchmark in `BENCHMARK-RESULTS.md` §11.1.
   Re-run after the change and record the delta in your PR description.
2. **Add `#[tracing::instrument(skip_all)]` spans** at every newly-cached
   boundary so future runs can spot regressions immediately
   (`BOTTLENECK-ANALYSIS.md` §"Measurement methodology").
3. **No micro-optimizations that complicate code without a measured win.**
   PGO and PRM-style tweaks come last.
4. **Never block the Tokio runtime.** Replace `std::fs::*` in hot paths with
   `tokio::fs::*` or wrap in `tokio::task::spawn_blocking`. Synchronous
   directory walks in async contexts are an anti-pattern.

### Caching hygiene

1. **Pick a clear invalidation strategy** per cache: TTL, mtime, content
   hash, or scope-bound (run / plan). Never ship a cache without an
   explicit eviction story.
2. **Bound caches.** Use `lru::LruCache` or a hard cap. An unbounded
   `HashMap` is a memory leak in disguise.
3. **Document the cache key inputs.** A future change to one of those
   inputs that doesn't update the key is a heisenbug factory.
4. **Cache per-process, not per-request.** Use `LazyLock` /
   `OnceLock<...>` for static caches; per-instance caches go on the
   service struct that owns the data.

### Concurrency hygiene

1. **Use `tokio::join!` for independent IO**, not `std::thread`.
2. **Never hold a `parking_lot::Mutex` across `.await`.** Use
   `tokio::sync::Mutex` or restructure to drop the guard first.
3. **Avoid `Arc<Mutex<HashMap<_, _>>>` patterns.** Prefer
   `RwLock<HashMap<_, _>>` for read-heavy caches, or `dashmap::DashMap`
   for write-heavy ones.
4. **Prefer `tokio::sync::RwLock` for config-style caches** (frequent
   reads, rare writes).

### Anti-pattern: "while we're here" refactors

Each plan in this folder is **scoped to one concrete optimization**. Do
not bundle unrelated cleanups into the same change. Reviewer attention is
finite, regressions hide easily inside large diffs, and rollback becomes
painful. If you spot something else worth fixing, open a follow-up issue.

### Compatibility constraints

- The `roko serve` HTTP daemon must continue to work after every change.
  It re-uses many of the same call sites as `roko run`; do not write code
  that assumes one-shot semantics (process exit cleans up everything).
- The Cursor IDE plugin and ACP bridge consume `runtime-events.jsonl`
  and `events.jsonl`. Any change to event formatting must remain
  forward-compatible.
- Tests that touch `LearningRuntime` may share `.roko/learn/` state. Do
  not introduce process-wide singletons that carry test data across
  cargo test runs (use `tempfile::tempdir` instead).

---

## File map (where things live)

When in doubt, the plans below cite specific files. The high-level map:

| Concern | Crate / file |
|---|---|
| CLI entry, dispatch, episode logging | `crates/roko-cli/src/run.rs` (3.6 k LOC) |
| Plan / multi-task orchestrator | `crates/roko-cli/src/orchestrate.rs` (22.8 k LOC) |
| Workflow state machine + driver | `crates/roko-runtime/src/workflow_engine.rs`, `effect_driver.rs`, `pipeline_state.rs` |
| Runtime event bus + JSONL log | `crates/roko-runtime/src/event_bus.rs`, `jsonl_logger.rs` |
| Provider HTTP client + agent factory | `crates/roko-agent/src/provider/mod.rs`, `model_call_service.rs` |
| Multi-agent / warm pool primitives | `crates/roko-agent/src/multi_pool.rs`, `pool.rs`, `session.rs` |
| Safety contracts | `crates/roko-agent/src/safety/contract.rs` |
| Prompt assembly | `crates/roko-compose/src/prompt_assembly_service.rs`, `enrichment/pipeline.rs` |
| Substrate (engrams.jsonl) | `crates/roko-fs/src/file_substrate.rs` |
| Learning subsystem | `crates/roko-learn/src/runtime_feedback.rs`, `feedback_service.rs` |
| Gate runner + composition | `crates/roko-gate/src/gate_service.rs`, `gate_pipeline.rs`, `composition.rs` |
| Config schema | `crates/roko-core/src/config/schema.rs`, `mod.rs` |
| Built-in bench harness | `crates/roko-cli/src/bench.rs`, `crates/roko-serve/src/bench.rs` |

---

## Verification harness

Every plan in this folder ends with a `make verify` ritual. Define it in
your shell profile or use directly:

```bash
# Fast (≈30 s): unit + clippy on changed crate.
cargo test -p <crate> --lib --release
cargo clippy -p <crate> --release -- -D warnings

# Slow (≈3-8 m): full workspace.
cargo test --workspace --release
cargo clippy --workspace --release --all-targets -- -D warnings

# Macro-benchmark: end-to-end timing (requires API keys).
RUST_LOG=roko=info /usr/bin/time -l ./target/release/roko run \
  --model gpt-4.1-nano --workflow-template express --gates none \
  "Reply with only the word hello"
```

Compare the macro-benchmark output to the baselines in
`BENCHMARK-RESULTS.md` §3. The goal is a measurable improvement, not just
"feels faster".

---

## Out-of-scope

The following are explicitly **not** addressed by these plans:

- **Provider-side latency.** Inference TTFT is dominated by the model
  vendor; we route to faster providers (Cerebras, Groq, OpenAI) but do
  not optimize their TTFT.
- **Network-bound benchmarks.** All numbers above assume a US datacenter
  network. China-based endpoints (Z.AI, Moonshot, Qwen) are not the
  baseline.
- **Browser / TUI rendering performance.** Out of scope for this
  initiative -- see `crates/roko-cli/src/tui/` for that work.
- **Cold-disk benchmarks.** Numbers assume a warm OS page cache. First
  runs after a reboot will be slower; that's expected.
