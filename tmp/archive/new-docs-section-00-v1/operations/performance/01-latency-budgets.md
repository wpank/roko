# Latency Budgets

> Per-stage latency targets for a healthy Roko deployment, expressed as p50/p95/p99
> percentiles. These numbers come from the benchmark suite and the self-hosting loop.

**Status**: Shipping
**Crate**: `roko-core`, `roko-orchestrator`, `roko-agent`, `roko-gate`, `roko-fs`
**Depends on**: [00-overview.md](00-overview.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

Non-LLM stages should complete in < 100 ms at p99. The gate pipeline dominates
non-LLM latency. LLM calls dominate total task latency.

---

## Core Primitive Latencies (Tier 1 — Hot Path)

Measured on CI (x86-64 Linux, no GPU) with `cargo bench`:

| Operation | p50 | p95 | p99 | Benchmark |
|-----------|-----|-----|-----|-----------|
| `Engram::new()` | 180 ns | 220 ns | 280 ns | `bench_engram_new` |
| `Score::compute(7 axes)` | 45 ns | 60 ns | 80 ns | `bench_score_compute` |
| `HdcVector::hamming_distance()` | 48 ns | 52 ns | 65 ns | `bench_hdc_hamming` |
| `HdcVector::bind()` (XOR) | 12 ns | 15 ns | 18 ns | `bench_hdc_bind` |
| `HdcVector::bundle()` (majority) | 38 ns | 42 ns | 50 ns | `bench_hdc_bundle` |
| Substrate JSONL append (1 Engram) | 8 µs | 25 µs | 60 µs | `bench_substrate_append` |
| Substrate JSONL append (batch 100) | 85 µs | 120 µs | 180 µs | `bench_substrate_batch` |
| EventBus publish (1 subscriber) | 200 ns | 350 ns | 500 ns | `bench_bus_publish` |
| EventBus publish (10 subscribers) | 800 ns | 1.2 µs | 2.0 µs | `bench_bus_publish_10` |
| Score decay step (exponential) | 22 ns | 28 ns | 35 ns | `bench_decay_step` |
| CascadeRouter T0 rule check | < 1 µs | 2 µs | 5 µs | `bench_cascade_t0` |
| HDC similarity search (10K entries) | 50 µs | 80 µs | 120 µs | `bench_hdc_search_10k` |
| HDC similarity search (1M entries) | 5 ms | 8 ms | 12 ms | `bench_hdc_search_1m` |

All p99 values are measured over 10,000 iterations with warm caches.

---

## Context Assembly Latency (Tier 1 — Index Operations)

| Operation | p50 | p95 | p99 | Notes |
|-----------|-----|-----|-----|-------|
| Symbol index warm lookup | 2 ms | 8 ms | 20 ms | Index already built, in memory |
| Symbol index cold lookup | 200 ms | 500 ms | 1 s | First lookup; triggers index build |
| Context assembly (small task, 2K tokens) | 30 ms | 80 ms | 150 ms | |
| Context assembly (large task, 30K tokens) | 150 ms | 300 ms | 600 ms | Multiple symbol + semantic lookups |
| Semantic search (HDC + ripgrep hybrid) | 15 ms | 40 ms | 80 ms | Typical workspace size (< 500 files) |

**When context assembly is slow:** The symbol index is cold (just built, pages evicted)
or the workspace is very large (> 5,000 files). The index uses `salsa` incremental
computation: only changed files are re-parsed on the second and subsequent runs.

---

## LLM Call Latency (Tier 2 — External Bound)

These are **typical observed values** from the Anthropic API, not benchmarks of Roko's
own code. They depend on model, payload size, network latency, and API load.

| Model | Time-to-first-token (p50) | Full response (p50) | Full response (p95) |
|-------|--------------------------|---------------------|---------------------|
| `claude-haiku-4-5` | 0.5 s | 2 s | 5 s |
| `claude-sonnet-4-5` | 0.8 s | 4 s | 12 s |
| `claude-opus-4-6` | 1.2 s | 8 s | 25 s |

These numbers are illustrative. Measure your own with `ROKO_LOG=roko_agent=debug` and
look for `llm_call_ms` log fields.

---

## Gate Latency (Tier 2 — Local Execution)

Gate times depend heavily on project size. These are targets for a medium-sized Rust
project (~50 crates, ~50K LOC):

| Gate | p50 | p95 | p99 | Scales with |
|------|-----|-----|-----|-------------|
| `compile` (`cargo check`) | 3 s | 8 s | 20 s | Dependency count, incremental cache hit rate |
| `test` (nextest, 200 tests) | 8 s | 20 s | 60 s | Test count, parallelism |
| `clippy` | 2 s | 5 s | 15 s | Crate count |
| `format` | 0.5 s | 1 s | 3 s | LOC count |
| `diff` (analysis, no LLM) | 50 ms | 200 ms | 500 ms | Diff size |
| `semantic` (LLM-judge) | 2 s | 5 s | 15 s | Response size; LLM latency |
| `security` (`cargo audit`) | 3 s | 8 s | 20 s | Dependency count |
| `coverage` (`cargo-llvm-cov`) | 20 s | 60 s | 120 s | Test count |

For large projects (> 200 crates), `compile` and `clippy` times may be significantly
higher on cold builds. Use `sccache` to maintain cross-invocation compilation caches.

---

## Aggregate Per-Task Latency

For a typical coding task with the default `["compile", "test", "clippy", "diff"]` pipeline:

| Scenario | p50 | p95 | p99 |
|----------|-----|-----|-----|
| First attempt passes all gates | ~20 s | ~45 s | ~80 s |
| One retry (1 gate failure) | ~35 s | ~80 s | ~160 s |
| Two retries | ~50 s | ~120 s | ~240 s |

The dominant factor is LLM + gate. Roko's internal overhead contributes < 1% of
total task time in typical deployments.

---

## Regression Budget

An operation is considered regressed if its p99 latency increases by > 20% across
benchmark runs with statistical significance (p < 0.05, 1000+ samples). See
[08-regression-detection.md](08-regression-detection.md).

---

## See Also

- [05-hot-paths.md](05-hot-paths.md) — which operations are on the hot path and why
- [07-benchmarks-reference.md](07-benchmarks-reference.md) — running the benchmarks
- [06-profiling-guide.md](06-profiling-guide.md) — measuring latency in production

## Open Questions

- p99 targets for `substrate.data_dir` on NFS or S3-backed storage are not yet measured.
- Latency budgets for multi-agent parallel execution (10+ concurrent agents) are not yet documented — they are dominated by resource contention for `sccache` and the gate filesystem.
