# Operations

> Everything you need to run Roko in production: configuration reference, performance
> targets, and failure recovery playbooks. Written for the operator — the person who
> deploys, tunes, and keeps Roko running.

**Status**: Shipping (configuration, core performance, core error handling) / Specified (cluster scaling, deployment automation)
**Crate**: cross-crate — `roko-cli`, `roko-orchestrator`, `roko-runtime`, `roko-learn`
**Depends on**: [reference/README.md](../reference/README.md), [guides/quickstart.md](../guides/quickstart.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

An operator running Roko needs three things:

1. **A valid `roko.toml`** — the unified configuration surface for all subsystems.
2. **An understanding of the performance envelope** — what is fast, what is bounded, and what to do when things slow down.
3. **A recovery playbook** — what Roko does automatically, what you must do manually, and how to replay a failure forensically.

This section covers all three.

---

## Who This Is For

This section is written for the **operator persona**: someone responsible for deploying and
running Roko in a production or semi-production environment. That might be:

- A developer running Roko as a self-hosting coding assistant on a laptop or dev server.
- An SRE running a Roko cluster for a team, with persistent storage and a shared model router.
- A platform engineer integrating Roko into a larger CI/CD or agent pipeline.

The docs assume you understand Rust build basics (`cargo build`, `cargo run`) and are
comfortable with TOML configuration files. No deep knowledge of Roko's internals is assumed —
the relevant concepts are linked where needed.

---

## What Is and Is Not in This Section

**In scope:**

- All `roko.toml` configuration keys, types, defaults, and examples.
- Environment-variable overrides and CLI flag precedence.
- Concrete performance targets (latency budgets, throughput, memory).
- Error taxonomy: what can fail, how Roko classifies failures, and what recovery looks like.
- Operational playbooks: crash recovery, partial failure, forensic replay.
- Observability: where errors surface, what metrics are emitted.

**Out of scope:**

- Deployment infrastructure (containers, Kubernetes, systemd) — see [`operations/deployment/`](deployment/) when that section is written.
- Monitoring dashboards and alerting setup — see [`operations/monitoring/`](monitoring/) when that section is written.
- The internal architecture of each subsystem — see [`reference/`](../reference/) for that.
- Benchmarking methodology and raw benchmark suites — see [`status/benchmarks.md`](../status/benchmarks.md).

---

## Contents

### Configuration

| # | Page | What it covers | Status |
|---|------|----------------|--------|
| — | [README](configuration/README.md) | Configuration folder index | Shipping |
| 00 | [Overview](configuration/00-overview.md) | `roko.toml` as the unified surface; init workflow | Shipping |
| 01 | [Schema Reference](configuration/01-roko-toml-schema.md) | Every table and key with type, default, range, example | Shipping |
| 02 | [Agent Config](configuration/02-agent-config.md) | `[agent]` table — model, turns, timeout, backends | Shipping |
| 03 | [Gate Config](configuration/03-gate-config.md) | `[gate]` table — pipeline, thresholds, adaptive gates | Shipping |
| 04 | [Learn Config](configuration/04-learn-config.md) | `[learn]` table — router, experiments, distillation | Shipping |
| 05 | [Substrate Config](configuration/05-substrate-config.md) | `[substrate]` table — storage backend selection | Shipping |
| 06 | [Bus Config](configuration/06-bus-config.md) | `[bus]` table — transport when shipping | Specified |
| 07 | [MCP Config](configuration/07-mcp-config.md) | `.mcp.json` discovery and tool-server layout | Shipping |
| 08 | [Environment Variables](configuration/08-environment-variables.md) | Env-var overrides and precedence rules | Shipping |
| 09 | [CLI Flag Precedence](configuration/09-cli-flag-precedence.md) | CLI flag > env > file > default chain | Shipping |
| 10 | [Validation](configuration/10-config-validation.md) | How configs are validated; what errors look like | Shipping |
| 11 | [Migration](configuration/11-config-migration.md) | Moving from old config formats | Shipping |
| 12 | [Examples](configuration/12-examples.md) | Laptop, server, cluster, coding-agent, research-agent profiles | Shipping |
| 13 | [Security](configuration/13-security-considerations.md) | Secrets handling, `.roko/` directory layout | Shipping |

### Performance

| # | Page | What it covers | Status |
|---|------|----------------|--------|
| — | [README](performance/README.md) | Performance folder index | Shipping |
| 00 | [Overview](performance/00-overview.md) | Philosophy: measured, not assumed | Shipping |
| 01 | [Latency Budgets](performance/01-latency-budgets.md) | Per-stage targets (p50/p95/p99) | Shipping |
| 02 | [Throughput Targets](performance/02-throughput-targets.md) | Engrams/sec, tasks/sec | Shipping |
| 03 | [Memory Model](performance/03-memory-model.md) | Allocation patterns, arenas, pooling | Shipping |
| 04 | [Numerical Stability](performance/04-numerical-stability.md) | Floating-point, score arithmetic, decay computation | Shipping |
| 05 | [Hot Paths](performance/05-hot-paths.md) | Critical paths; what not to allocate on | Shipping |
| 06 | [Profiling Guide](performance/06-profiling-guide.md) | How to profile a running Roko instance | Shipping |
| 07 | [Benchmarks Reference](performance/07-benchmarks-reference.md) | Per-subsystem benchmark suites | Shipping |
| 08 | [Regression Detection](performance/08-regression-detection.md) | How perf regressions are caught | Built |
| 09 | [Scaling Patterns](performance/09-scaling-patterns.md) | Horizontal vs vertical; Substrate sharding | Specified |
| 10 | [Resource Limits](performance/10-resource-limits.md) | Memory caps, disk quotas, rate limits | Shipping |

### Error Handling

| # | Page | What it covers | Status |
|---|------|----------------|--------|
| — | [README](error-handling/README.md) | Error-handling folder index | Shipping |
| 00 | [Overview](error-handling/00-overview.md) | Error philosophy: verdicts not errors | Shipping |
| 01 | [Error Taxonomy](error-handling/01-error-taxonomy.md) | Classes: gate-verdict, infra, user, LLM, safety | Shipping |
| 02 | [Recovery Strategies](error-handling/02-recovery-strategies.md) | Retry, circuit-break, escalate, fail | Shipping |
| 03 | [Event-Log Replay](error-handling/03-event-log-replay.md) | Hash-chained event-log-replay recovery | Built |
| 04 | [Crash Recovery](error-handling/04-crash-recovery.md) | ParallelExecutor restart semantics | Shipping |
| 05 | [Partial Failure](error-handling/05-partial-failure.md) | When a subset of agents fail | Shipping |
| 06 | [Cascade Failure](error-handling/06-cascade-failure.md) | Preventing amplification; circuit breakers | Built |
| 07 | [Forensic Replay](error-handling/07-forensic-replay.md) | Reproducing a failure post-hoc | Built |
| 08 | [Observability](error-handling/08-observability.md) | Where errors surface: logs, Pulses, metrics | Shipping |
| 09 | [Failure Drills](error-handling/09-failure-drill-examples.md) | 5+ concrete failure scenarios walked through | Shipping |

---

## Suggested Reading Order

**New operator (first deployment):**
`configuration/00-overview.md` → `configuration/12-examples.md` → `configuration/08-environment-variables.md` → `error-handling/04-crash-recovery.md`

**Diagnosing a slow system:**
`performance/00-overview.md` → `performance/01-latency-budgets.md` → `performance/05-hot-paths.md` → `performance/06-profiling-guide.md`

**Diagnosing a failure:**
`error-handling/00-overview.md` → `error-handling/01-error-taxonomy.md` → `error-handling/04-crash-recovery.md` → `error-handling/09-failure-drill-examples.md`

**Security review:**
`configuration/13-security-considerations.md` → `configuration/08-environment-variables.md` → `error-handling/01-error-taxonomy.md` (safety class)

---

## See Also

- [`guides/quickstart.md`](../guides/quickstart.md) — getting started end-to-end
- [`reference/11-crate-map.md`](../reference/11-crate-map.md) — which crate ships which concept
- [`status/benchmarks.md`](../status/benchmarks.md) — full benchmark suite and raw numbers
- [`status/status.md`](../status/status.md) — implementation tier for each component

## Open Questions

- Deployment section (`operations/deployment/`) is not yet written — will cover systemd, Docker, Fly.io.
- Monitoring section (`operations/monitoring/`) is not yet written — will cover Prometheus metrics, alerting rules.
