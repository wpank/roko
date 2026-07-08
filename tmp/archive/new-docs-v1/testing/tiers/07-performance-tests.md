# Performance Tests

> Benchmark harness using `criterion.rs` for hot-path latency tracking with flakiness control.

**Status**: Built (benchmarks defined; CI integration is partial)
**Crate**: `roko-bench` (benchmark harness), individual crate `benches/` directories
**Depends on**: [01-unit-tests.md](01-unit-tests.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

Performance tests use `criterion.rs` to benchmark hot paths (scoring, gate pipeline, HDC similarity search, content hashing, substrate reads/writes). Flakiness is controlled via statistical noise thresholds. Regressions are reported as percentage changes with confidence intervals. Performance benchmarks run in pre-release CI.

---

## Benchmark Harness

Roko uses `criterion.rs` with the following configuration:
- **Measurement time**: 5s per benchmark (default).
- **Noise threshold**: 5% — changes smaller than 5% are not reported as regressions.
- **Sample size**: 100 samples minimum (criterion default).
- **Baseline tracking**: baselines stored in `target/criterion/` (gitignored) and in CI as named baseline artifacts.

---

## Benchmark Catalogue

### `roko-core` — Core Type Operations

| Benchmark | What it measures | Target latency |
|---|---|---|
| `bench_content_hash_1kb` | BLAKE3 hashing of 1KB | < 5µs |
| `bench_content_hash_1mb` | BLAKE3 hashing of 1MB | < 200µs |
| `bench_score_aggregate` | Weighted score aggregation over 7 axes | < 100ns |
| `bench_engram_serialize` | `serde_json` serialization of a full Engram | < 10µs |
| `bench_engram_deserialize` | `serde_json` deserialization of a full Engram | < 15µs |
| `bench_decay_step_1000` | Decay computation for 1000 Engrams | < 1ms |

### `roko-gate` — Gate Pipeline

| Benchmark | What it measures | Target latency |
|---|---|---|
| `bench_gate_compile_cold` | First compile gate evaluation (cold cache) | < 5s |
| `bench_gate_compile_warm` | Compile gate evaluation (warm cache) | < 500ms |
| `bench_gate_pipeline_7_rungs` | Full 7-rung pipeline (mocked compilers) | < 100ms |
| `bench_gate_threshold_eval` | EMA threshold evaluation for 1 gate | < 1µs |

### `roko-neuro` — HDC Operations

| Benchmark | What it measures | Target latency |
|---|---|---|
| `bench_hdc_bundle_10` | Bundling 10 hypervectors (10,240-bit) | < 1µs |
| `bench_hdc_bind` | Binding two 10,240-bit hypervectors | < 500ns |
| `bench_hdc_similarity_1000` | Similarity search over 1000 hypervectors | < 5ms |
| `bench_hdc_encode_engram` | Encoding an Engram to a hypervector | < 10µs |

### `roko-fs` — Substrate I/O

| Benchmark | What it measures | Target latency |
|---|---|---|
| `bench_substrate_write_engram` | Single Engram write to JSONL substrate | < 100µs |
| `bench_substrate_read_by_id` | Single Engram read by content hash | < 50µs |
| `bench_substrate_list_1000` | List 1000 Engrams by query | < 10ms |
| `bench_substrate_gc` | GC pass over 10,000 Engrams | < 1s |

---

## Running Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run a specific benchmark group
cargo bench -p roko-core

# Run a specific benchmark
cargo bench -p roko-core -- bench_content_hash_1kb

# Compare against a saved baseline
cargo bench -- --baseline main_branch

# Save as a new baseline
cargo bench -- --save-baseline my_feature
```

---

## Flakiness Control

Criterion's statistical model computes a confidence interval for each measurement. A benchmark is only flagged as a regression if the performance change exceeds both:
1. The 5% noise threshold.
2. The 95% confidence interval lower bound.

Environment flakiness (background load, thermal throttling) is controlled in CI by:
- Running benchmarks on dedicated bare-metal CI runners.
- Discarding runs where system load > 50% at measurement start.
- Using 200 samples (instead of 100) for high-variance benchmarks.

---

## Regression Policy

A performance regression in CI is not a blocking failure. It generates a warning annotation on the PR with:
- The affected benchmark.
- The percentage change.
- The confidence interval.
- A link to the previous run's report.

The PR author decides whether the regression is acceptable. For regressions > 20%, a maintainer review is required.

---

## Invariants

- Benchmarks must not use `#[test]` — they live in `benches/` directories and run via `cargo bench`.
- Benchmarks must use `criterion::black_box()` on all inputs to prevent dead-code elimination.
- Benchmarks may not make real LLM calls; use in-process mock responses.

---

## Roadmap

- [ ] Add `roko-orchestrator` benchmarks for plan scheduling throughput.
- [ ] Add `roko-learn` benchmarks for bandit algorithm update latency.
- [ ] Publish benchmark results to a public dashboard (Phase 2).

## Open Questions

- Should performance benchmarks be part of pre-release CI or a separate nightly job?
- How should HDC benchmarks handle the 10,240-bit vector size when hardware SIMD width varies?

## See also

- [../quality-gates/03-pre-release.md](../quality-gates/03-pre-release.md) — benchmarks in the release gate
- [reference/21-performance-numerical-stability.md](../../reference/21-performance-numerical-stability.md) — performance architecture
