# Profiling Guide

> How to measure what Roko is actually doing: CPU flamegraphs, heap profiles, latency
> histograms, and production sampling.

**Status**: Shipping
**Crate**: cross-crate
**Depends on**: [00-overview.md](00-overview.md), [05-hot-paths.md](05-hot-paths.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

For a quick flamegraph: `cargo flamegraph -p roko-cli -- plan run plans/`. For heap:
use `heaptrack`. For production latency: `ROKO_LOG=roko=debug` and parse `*_ms` fields
from structured logs.

---

## Step 1: Identify the Bottleneck First

Before profiling, use structured logs to pinpoint which stage is slow:

```bash
ROKO_LOG=roko=debug roko plan run plans/ 2>&1 | jq 'select(.fields.stage != null) | {stage: .fields.stage, ms: .fields.duration_ms}'
```

Log fields to look for:

| Field | Stage | What it measures |
|-------|-------|----------------|
| `context_assembly_ms` | Context | Time to assemble the LLM prompt |
| `llm_first_token_ms` | LLM | Time to first streaming token |
| `llm_full_response_ms` | LLM | Time for full LLM response |
| `gate_duration_ms` + `gate_name` | Gate | Per-gate execution time |
| `substrate_append_ms` | Substrate | Engram persist time |
| `episode_flush_ms` | Learning | Episode batch flush time |
| `hdc_search_ms` | Learning | HDC similarity search time |

If `gate_duration_ms` for `test` is > 30 seconds, the bottleneck is the test suite.
If `llm_full_response_ms` is > 20 seconds, the bottleneck is the LLM API.
If `context_assembly_ms` is > 1 second, the symbol index is cold or the workspace is large.

---

## Step 2: CPU Flamegraph

For CPU-bound slowness (e.g. slow symbol index build, slow context assembly, unexpected
hot-path allocation):

### Using `cargo-flamegraph`

```bash
cargo install flamegraph
cargo flamegraph -p roko-cli -- plan run plans/ --concurrency 1
# opens flamegraph.svg in the browser
```

### Using `perf` + `inferno` (Linux)

```bash
cargo build --release -p roko-cli
perf record --call-graph dwarf target/release/roko plan run plans/
perf script | inferno-collapse-perf | inferno-flamegraph > flamegraph.svg
```

### Reading the Flamegraph

Focus on:
- `roko_core::engram` — if this is > 5% of CPU, Engram construction is unexpectedly hot.
- `roko_fs::substrate` — if > 2%, the write buffer is flushing too often (reduce
  batch size or increase buffer).
- `serde_json` — if > 10%, JSON serialisation is a bottleneck (expected for high
  Engram throughput; switch to `rkyv` if this matters).
- `regex::` — if present, a regex is being compiled on the hot path (should not happen).
- `alloc::` or `jemalloc::` — if > 5%, unexpected heap allocation.

---

## Step 3: Heap Profile

For memory growth or unexpected allocation:

### Using `heaptrack` (Linux)

```bash
heaptrack target/release/roko plan run plans/
heaptrack_gui heaptrack.roko.<pid>.gz
```

Look for:
- `roko_core::engram::new` — should allocate only into the arena, not the global heap.
- `Vec::with_capacity` calls that are outside the startup path.
- Any `String` allocation inside the core hot loop.

### Using `dhat` (portable, slower)

```bash
cargo add dhat --dev
# Add dhat profiling harness to the binary (see dhat docs)
DHAT_OPTS=heap cargo run --features dhat-heap -p roko-cli -- plan run plans/
```

---

## Step 4: Latency Histogram

For p99 tail latency investigation:

```bash
# Run 100 tasks and collect latency data
ROKO_LOG=roko=info roko plan run plans/ 2>&1 | \
  jq -r 'select(.fields.gate_duration_ms) | [.fields.gate_name, .fields.gate_duration_ms] | @csv' | \
  sort -t, -k2 -n | tail -20
```

This shows the slowest 20 gate executions, sorted by duration. Identify outliers (e.g.
a single `cargo test` that took 120 s while others took 8 s).

---

## Step 5: Continuous Sampling in Production

For low-overhead continuous profiling in production (Linux only):

```bash
# Run Roko with async-profiler sampling (1% overhead)
ASYNC_PROFILER_EVENT=cpu ASYNC_PROFILER_INTERVAL=10ms \
  java -agentpath:/path/to/libasyncProfiler.so=start,file=profile.jfr \
  ... # (or use similar for Rust: perf with low sample rate)
```

For Rust, `cargo-pprof` with `pprof-rs` provides a low-overhead in-process sampler:

```toml
# Cargo.toml (dev profile)
[profile.dev]
debug = true

[dependencies]
pprof = { version = "0.12", features = ["flamegraph"] }
```

Then query the profile endpoint:

```bash
curl http://localhost:6060/debug/pprof/profile?seconds=30 -o profile.pb
go tool pprof -http=:8080 profile.pb
```

(The `/debug/pprof` endpoint is planned for `roko serve` — not yet implemented.)

---

## Quick Reference: Profiling Decision Tree

```
Roko is slow
  ├─ Which stage? → ROKO_LOG=roko=debug, look at *_ms fields
  │
  ├─ LLM latency high?
  │   → Check network / API status. Use gateway caching. Use faster model.
  │
  ├─ Gate latency high?
  │   → Identify which gate. For test: use nextest, reduce test scope.
  │     For compile: use sccache, reduce cold rebuild.
  │
  ├─ Context assembly slow (> 500 ms)?
  │   → Symbol index is cold. Run once to warm it. Check workspace size.
  │
  ├─ High CPU (not LLM/gate)?
  │   → CPU flamegraph (cargo flamegraph). Look for unexpected hot paths.
  │
  └─ Memory growing?
      → heaptrack. Check for arena misuse or unconstrained Vec growth.
```

---

## See Also

- [01-latency-budgets.md](01-latency-budgets.md) — reference numbers to compare against
- [07-benchmarks-reference.md](07-benchmarks-reference.md) — running the benchmark suite
- [08-regression-detection.md](08-regression-detection.md) — automated regression detection

## Open Questions

- `roko serve` does not yet expose a `/debug/pprof` profiling endpoint.
- A built-in `roko profile` subcommand (automates flamegraph generation) is planned.
