# 09-telemetry — Depth Index

Depth for [09-TELEMETRY.md](../../unified/09-TELEMETRY.md)

> **Implementation status (2026-09-10):** COMPLETE (E33 9/9; 39/39 ingress). All 11 built-in Lens executors, bounded queued delivery, breaker controls, typed StateHub aggregation, REST/SSE, restart-durable history, resolution queries, configurable 7-day retention, and all 39 production event variants are live. `roko-serve` samples shared metrics every 30s through `PeriodicObserver` and writes rotation-bounded JSONL. Direct native Agent publication into E33 observation ingress remains separate product scope.

---

## Source docs (1)

### Production observability

| Source doc | Status |
|---|---|
| `docs/19-deployment/14-observability-and-telemetry.md` | Absorbed |

---

## Depth docs

| Depth doc | Source | What it adds |
|---|---|---|
| [01-observability-as-lens-pipeline.md](01-observability-as-lens-pipeline.md) | `14-observability-and-telemetry.md` | Observability as Pipeline of Lens Cells. Logs as Bus Pulses (telemetry.log.* topics). Metrics as numeric Lens outputs with /metrics endpoint. Traces as lineage-annotated Signals. StateHub projections as named Lens compositions. Cost visibility as a Lens Pipeline. Replay as Store traversal. |
