# Performance Overview

> Roko's performance philosophy: every target is a measured number, not a marketing claim.
> No component ships without a benchmark. Every regression is caught before it merges.

**Status**: Shipping
**Crate**: cross-crate
**Depends on**: [operations/performance/README.md](README.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

Roko's performance model has two tiers: the sub-millisecond **synchronous hot path**
(Engram construction, scoring, HDC operations, Substrate writes) and the multi-second
**async agent path** (LLM calls, gate execution). The hot path must not allocate on the
heap in steady state. The async path is bounded by external latency (LLM APIs) and is
optimised for throughput (parallel agent dispatch) rather than per-call latency.

---

## Philosophy

**Measured, not assumed.** Every performance claim in this documentation has a backing
benchmark. Claims without benchmarks are marked as `[estimated]` or `[no-benchmark]`.
If you observe behaviour that contradicts a number in this documentation, file a bug
with your measurement — the number or the code is wrong.

**The LLM is the bottleneck; everything else must be invisible.** An LLM call takes
1–30 seconds. Roko's internal work — scoring, HDC fingerprinting, substrate writes,
gate execution — must complete in under 100 ms aggregate in the common case. If Roko's
overhead is visible relative to LLM latency, something is wrong.

**Allocate at setup, not at runtime.** The hot paths (Score arithmetic, HDC operations,
Engram construction, Substrate writes) use arena allocators and pre-allocated pools. No
heap allocation in steady state for core types.

**Parallelism, not serialisation.** Where tasks are independent, Roko runs them in
parallel. The plan executor uses a DAG-based scheduler. Gate rungs run in parallel
where possible. HDC fingerprinting runs on a background thread pool, not blocking
the main task loop.

---

## The Two Performance Tiers

### Tier 1: Synchronous Hot Path (< 1 ms target)

Operations that happen on every event, on every Engram, on every agent turn:

- Engram construction and field assignment.
- Score arithmetic (7-axis, f32 math).
- HDC fingerprint XOR / Hamming distance operations.
- Substrate JSONL append (fast path: buffer flush, no fsync).
- EventBus<E> publish/subscribe dispatch.
- Gate pre-screening (T0 rule check in CascadeRouter).

These operations are benchmarked individually. The combined overhead per Engram creation
must be < 1 ms at p99 on the CI benchmark machine.

### Tier 2: Async Agent Path (LLM-bounded)

Operations that involve I/O or external services:

- LLM API calls (1–30 seconds typical; dominated by model response time).
- Gate execution (compile: 1–10 s; test: 10–120 s; clippy: 1–5 s).
- MCP tool calls (variable; dominated by tool execution time).
- Substrate GC (background; does not block the main loop).

Targets for Tier 2 are per-stage latency budgets (see [01-latency-budgets.md](01-latency-budgets.md)).

---

## Budget Model for a Single Task

A complete task (from LLM invocation to Substrate commit) has this budget:

| Stage | Budget (p50) | Budget (p99) | Bottleneck |
|-------|-------------|-------------|-----------|
| Context assembly | 50 ms | 200 ms | Symbol index lookup |
| LLM first token | 0.5 s | 3 s | Model response time / cold start |
| LLM full response | 2 s | 20 s | Model response size / streaming |
| Gate pipeline | 5 s | 60 s | Test suite size |
| Engram persist | 1 ms | 5 ms | JSONL append (buffered) |
| Learning update | 10 ms | 50 ms | Episode write + bandit update |
| **Total (ex-LLM)** | **~55 ms** | **~315 ms** | Context + gates dominate |
| **Total (inc-LLM)** | **~7 s** | **~80 s** | LLM dominates |

The non-LLM overhead (context assembly + persist + learning) must stay under 500 ms
at p99. Everything beyond that is LLM or gate execution.

---

## What to Check First When Roko Is Slow

1. **Is it the LLM?** Check the `llm.latency_ms` metric. If LLM calls are taking > 10s
   at p50, the bottleneck is the LLM API (network, rate limits, model size). Use
   CascadeRouter to route cheap tasks to cheaper/faster models.

2. **Is it the gates?** Check the `gate.duration_ms` metric per gate name. A slow test
   suite is the most common culprit. Consider parallelising tests with nextest, or
   reducing the gate pipeline for fast iteration.

3. **Is it context assembly?** Check the `context.assembly_ms` metric. If context
   assembly is taking > 200 ms, the workspace symbol index may be cold (first run after
   a clean build) or stale.

4. **Is it the Substrate?** Check `substrate.write_ms` and `substrate.file_size_mb`.
   If the JSONL files are large (> 500 MB), consider running `roko substrate gc` and
   reducing `gc_interval_hours`.

5. **Is it allocation?** Run with `MALLOC_CONF=stats_print:true jemalloc` or use
   the profiling guide ([06-profiling-guide.md](06-profiling-guide.md)) to identify
   unexpected heap allocation on hot paths.

---

## See Also

- [01-latency-budgets.md](01-latency-budgets.md) — per-stage latency targets
- [05-hot-paths.md](05-hot-paths.md) — what the hot paths are and allocation rules
- [06-profiling-guide.md](06-profiling-guide.md) — how to measure

## Open Questions

- A unified performance dashboard (visualising all tier-1 and tier-2 metrics in one view) is not yet built.
- The budget model above is based on empirical observations from the self-hosting loop; it should be validated with formal benchmarks.
