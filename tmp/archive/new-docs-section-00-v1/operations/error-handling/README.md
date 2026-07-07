# Error Handling

> Roko's approach to failure: how errors are classified, what recovery is applied
> automatically, and what operators must do manually. Written for someone debugging
> a failure at 2 AM.

**Status**: Shipping (core recovery, crash recovery, observability) / Built (forensic replay, cascade protection)
**Crate**: `roko-orchestrator`, `roko-agent`, `roko-gate`, `roko-runtime`
**Depends on**: [operations/README.md](../README.md)
**Last reviewed**: 2026-04-19

---

## Contents

| # | Page | What it covers | Status |
|---|------|----------------|--------|
| 00 | [Overview](00-overview.md) | Error philosophy: verdicts not errors | Shipping |
| 01 | [Error Taxonomy](01-error-taxonomy.md) | Classes: gate-verdict, infra, user, LLM, safety | Shipping |
| 02 | [Recovery Strategies](02-recovery-strategies.md) | Retry, circuit-break, escalate, fail | Shipping |
| 03 | [Event-Log Replay](03-event-log-replay.md) | Hash-chained event-log recovery | Built |
| 04 | [Crash Recovery](04-crash-recovery.md) | ParallelExecutor restart semantics | Shipping |
| 05 | [Partial Failure](05-partial-failure.md) | When a subset of agents fail | Shipping |
| 06 | [Cascade Failure](06-cascade-failure.md) | Preventing amplification; circuit breakers | Built |
| 07 | [Forensic Replay](07-forensic-replay.md) | Reproducing a failure post-hoc | Built |
| 08 | [Observability](08-observability.md) | Where errors surface: logs, Pulses, metrics | Shipping |
| 09 | [Failure Drills](09-failure-drill-examples.md) | 5+ concrete failure scenarios | Shipping |

## Suggested reading order

Debugging a current failure: `00` → `01` (classify the error) → `04` (if crashed) → `09` (find the scenario).
Setting up monitoring: `08` → `01`.
Understanding the recovery model: `00` → `02` → `05` → `06`.

## See also

- [`operations/configuration/03-gate-config.md`](../configuration/03-gate-config.md) — gate pipeline and retry configuration
- [`reference/05-operators/gate.md`](../../reference/05-operators/gate.md) — gate verdicts
- [`status/status.md`](../../status/status.md) — which error recovery components are Shipping vs Built
