# ⚠️ SUPERSEDED — See [MASTER-PLAN.md](../MASTER-PLAN.md) Tier 1D
>
> Content absorbed into MASTER-PLAN.md. This file retained for historical reference.

---

# 08 — Observability Wiring

> **Priority**: 🟡 P2 — System works without this but is opaque
> **Parity sections**: §40 (Observability), I.4 (Observability wiring)
> **Checklist ref**: `MORI-PARITY-CHECKLIST.md` §40, I.4

## Problem statement

Roko has tracing infrastructure (`TraceSink`, `MetricsSink`, `ToolTrace`,
`FailureTrace`, `MetricRegistry`) all built but never initialized or called
from the runtime.

## What exists (built but unused)

| Component | Path | Wired? |
|-----------|------|--------|
| TraceSink trait | `roko-core/src/trace.rs` | ❌ |
| MetricsSink trait | `roko-core/src/metrics.rs` | ❌ |
| ToolTrace struct | `roko-core/src/tool/trace.rs` | ❌ |
| FailureTrace struct | `roko-core/src/failure.rs` | ❌ |
| MetricRegistry | `roko-agent/src/metrics/` | ❌ |
| Prometheus exporter | `roko-agent/src/metrics/prometheus.rs` | ❌ |

## Checklist

- [ ] **8.1** Initialize TraceSink at CLI startup
- [ ] **8.2** Initialize MetricsSink at CLI startup
- [ ] **8.3** Agent dispatch emits ToolTrace for each tool invocation
- [ ] **8.4** Failed operations emit FailureTrace
- [ ] **8.5** MetricRegistry tracks: agent_runs_total, agent_duration_seconds, gate_pass_rate, tokens_used_total, cost_usd_total
- [ ] **8.6** Prometheus /metrics endpoint when --web-port is active
- [ ] **8.7** Structured JSON logs (tracing-subscriber with JSON formatter)
- [ ] **8.8** OpenTelemetry trace export (optional, behind feature flag)
- [ ] **8.9** Cost attribution per agent/role/task in trace spans
- [ ] **8.10** ToolTrace/FailureTrace serialization to episode artifacts

> Maps to checklist: I.4.1 through I.4.6, §40.1-40.22
