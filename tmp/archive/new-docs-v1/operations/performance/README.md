# Performance

> Concrete numbers for what Roko should achieve, where the bottlenecks are, and how to
> profile and tune a running instance. Performance is measured and documented — not assumed.

**Status**: Shipping (core targets and hot paths) / Built (regression detection) / Specified (cluster scaling)
**Crate**: `roko-core`, `roko-orchestrator`, `roko-agent`, `roko-gate`, `roko-fs`
**Depends on**: [operations/README.md](../README.md)
**Last reviewed**: 2026-04-19

---

## Contents

| # | Page | What it covers | Status |
|---|------|----------------|--------|
| 00 | [Overview](00-overview.md) | Philosophy: measured, not assumed | Shipping |
| 01 | [Latency Budgets](01-latency-budgets.md) | Per-stage targets (p50/p95/p99) | Shipping |
| 02 | [Throughput Targets](02-throughput-targets.md) | Engrams/sec, tasks/sec | Shipping |
| 03 | [Memory Model](03-memory-model.md) | Allocation patterns, arenas, pooling | Shipping |
| 04 | [Numerical Stability](04-numerical-stability.md) | Floating-point, score arithmetic, decay computation | Shipping |
| 05 | [Hot Paths](05-hot-paths.md) | Critical paths; allocation rules | Shipping |
| 06 | [Profiling Guide](06-profiling-guide.md) | How to profile Roko | Shipping |
| 07 | [Benchmarks Reference](07-benchmarks-reference.md) | Per-subsystem benchmark suites | Shipping |
| 08 | [Regression Detection](08-regression-detection.md) | How perf regressions are caught | Built |
| 09 | [Scaling Patterns](09-scaling-patterns.md) | Horizontal vs vertical; sharding | Specified |
| 10 | [Resource Limits](10-resource-limits.md) | Memory caps, disk quotas, rate limits | Shipping |

## Suggested reading order

Diagnosing a slow system: `00` → `01` → `05` → `06`.
Setting up performance monitoring: `07` → `08`.
Planning a large deployment: `09` → `10`.

## See also

- [`status/benchmarks.md`](../../status/benchmarks.md) — raw benchmark numbers and CI history
- [`operations/configuration/04-learn-config.md`](../configuration/04-learn-config.md) — CascadeRouter cost reduction
- [`operations/error-handling/08-observability.md`](../error-handling/08-observability.md) — metrics that surface performance issues
