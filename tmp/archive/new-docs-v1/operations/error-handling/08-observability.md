# Observability

> The structured logging, metrics, and alerting surface that lets an operator know
> what Roko is doing, how fast it is doing it, and when something is wrong.

**Status**: Shipping (structured logs, ROKO_LOG) / Built (metrics, circuit state, regression pulse)
**Crate**: `roko-runtime`, `roko-orchestrator`, `roko-gate`, `roko-learn`
**Depends on**: [Error Taxonomy](01-error-taxonomy.md), [Forensic Replay](07-forensic-replay.md)
**Used by**: [Failure Drill Examples](09-failure-drill-examples.md)
**Last reviewed**: 2026-04-19

---

## Three Observability Pillars

| Pillar | Roko implementation | Status |
|---|---|---|
| **Logs** | Structured JSON via `tracing` crate; `ROKO_LOG` controls level/filter | Shipping |
| **Metrics** | `roko-runtime` emits counters/histograms; in-process aggregation | Built |
| **Traces** | OpenTelemetry spans (OTLP export); per-task and per-gate spans | Specified |

This page covers all three as they exist today, with clear markers for what is Shipping
vs Built vs Specified.

---

## Structured Logs (Shipping)

Roko writes structured JSON logs to stderr by default. Each line is a valid JSON object:

```json
{
  "timestamp": "2026-04-19T14:03:17.441Z",
  "level":     "ERROR",
  "target":    "roko_gate",
  "span":      { "task_id": "task-7f3a", "rung": "test" },
  "message":   "gate failed",
  "error_code": "ROKO-G-002",
  "exit_code":  1,
  "duration_ms": 8134,
  "cascade_depth": 0
}
```

### Log level control

```bash
# Set log level globally
export ROKO_LOG=info

# Enable debug logs for specific crates only
export ROKO_LOG="roko_gate=debug,roko_agent=debug,roko_learn=warn"

# Trace-level (very verbose; includes every LLM token stream event)
export ROKO_LOG=trace
```

Recommended production setting:

```bash
export ROKO_LOG="roko=info,roko_gate=debug"
```

This captures gate-level detail (useful for failure triage) without the volume of
agent turn traces.

### Key log fields reference

| Field | Type | Present in | Meaning |
|---|---|---|---|
| `timestamp` | ISO-8601 | All | UTC timestamp |
| `level` | string | All | TRACE / DEBUG / INFO / WARN / ERROR |
| `target` | string | All | Rust crate/module path |
| `task_id` | string | All task logs | Active task identifier |
| `rung` | string | Gate logs | Gate rung name |
| `error_code` | string | ERROR logs | `ROKO-<CLASS>-<NUM>` |
| `cascade_depth` | int | ERROR logs | 0=root cause, >0=secondary |
| `caused_by` | string | ERROR logs | Parent error_code |
| `duration_ms` | float | Timing logs | Wall-clock duration |
| `model` | string | LLM logs | Model identifier |
| `tokens_in` | int | LLM logs | Prompt token count |
| `tokens_out` | int | LLM logs | Completion token count |
| `latency_ms` | float | LLM logs | Time to first token |
| `subtask_id` | string | Executor logs | Subtask identifier |
| `circuit_state` | string | Circuit logs | CLOSED / HALF-OPEN / OPEN |

### Collecting logs

```bash
# Pipe to jq for live filtering
roko run --task "fix bug" 2>&1 | jq 'select(.level == "ERROR")'

# Write to file with tee
roko run --task "fix bug" 2> >(tee roko.log) 1>/dev/null

# Ship to a log aggregator (e.g., Loki, Datadog, CloudWatch)
roko run --task "fix bug" 2>&1 | your-log-shipper-agent
```

---

## Metrics (Built)

The `roko-runtime` crate maintains in-process metric counters and histograms. They are
accessible via:

```bash
roko metrics show         # print current snapshot to stdout
roko metrics show --json  # JSON format for parsing
```

### Full metrics reference

#### Task metrics

| Metric | Type | Description |
|---|---|---|
| `tasks.started_total` | Counter | Tasks started since process start |
| `tasks.completed_total` | Counter | Tasks completed successfully |
| `tasks.failed_total` | Counter | Tasks that ended in terminal failure |
| `tasks.duration_ms` | Histogram | Per-task wall-clock duration |
| `tasks.turns_used` | Histogram | Agent turns consumed per task |

#### Gate metrics

| Metric | Type | Description |
|---|---|---|
| `gate.runs_total{rung}` | Counter | Gate runs per rung |
| `gate.pass_total{rung}` | Counter | Gate passes per rung |
| `gate.fail_total{rung}` | Counter | Gate failures per rung |
| `gate.retry_total{rung}` | Counter | Retries per rung |
| `gate.escalation_total{rung}` | Counter | Escalations per rung |
| `gate.duration_ms{rung}` | Histogram | Per-rung wall-clock duration |
| `gate.circuit_open{service}` | Gauge | 1 if circuit is OPEN, else 0 |

#### LLM / model metrics

| Metric | Type | Description |
|---|---|---|
| `llm.requests_total{model}` | Counter | LLM API calls per model |
| `llm.errors_total{model,code}` | Counter | LLM errors by model and error code |
| `llm.tokens_in_total{model}` | Counter | Prompt tokens consumed |
| `llm.tokens_out_total{model}` | Counter | Completion tokens generated |
| `llm.latency_ms{model}` | Histogram | Time to first token per model |
| `llm.rate_limit_total{model}` | Counter | HTTP 429s received |

#### Substrate metrics

| Metric | Type | Description |
|---|---|---|
| `substrate.writes_total` | Counter | Successful substrate writes |
| `substrate.write_errors_total` | Counter | Failed substrate writes |
| `substrate.bytes_written_total` | Counter | Total bytes appended |
| `substrate.size_bytes` | Gauge | Current substrate size on disk |

#### Learning metrics

| Metric | Type | Description |
|---|---|---|
| `learn.episodes_queued` | Gauge | Episodes awaiting processing |
| `learn.episodes_processed_total` | Counter | Episodes processed by CascadeRouter |
| `learn.episodes_failed_total` | Counter | Episodes that failed routing |
| `learn.episodes_dropped_total` | Counter | Episodes dropped (substrate error) |
| `learn.router_tier{tier}` | Counter | Routing decisions by tier (T0/T1/T2) |

#### Memory / resource metrics

| Metric | Type | Description |
|---|---|---|
| `memory.engrams_total` | Gauge | Current engrams in HDC index |
| `memory.hdc_index_bytes` | Gauge | HDC index memory (1,280 bytes × engrams) |
| `memory.agent_rss_bytes` | Gauge | Agent subprocess resident set size |
| `memory.gc_runs_total` | Counter | Substrate GC cycles run |

---

## Alerting Thresholds (Operator-Defined)

Roko does not ship a built-in alerting engine (Specified). Connect the metrics endpoint
to your existing monitoring stack. Recommended alert thresholds:

### Critical alerts (page immediately)

| Condition | Threshold | Meaning |
|---|---|---|
| `gate.circuit_open{service="llm/openai"}` | > 0 for > 5 min | LLM provider outage |
| `substrate.write_errors_total` rate | > 0.1/min sustained | Disk problem |
| `tasks.failed_total` rate | > 5/min | Systemic failure |
| `memory.agent_rss_bytes` | > 2 GB | Agent memory runaway |
| `substrate.size_bytes` | > `substrate.max_size_gb` × 0.9 | Near disk cap |

### Warning alerts (investigate within 1 hour)

| Condition | Threshold | Meaning |
|---|---|---|
| `gate.fail_total{rung="compile"}` rate | > 2/task avg | Code quality regression |
| `llm.rate_limit_total` rate | > 0.5/min | Approaching API rate limit |
| `learn.episodes_queued` | > 100 for > 10 min | Learning subsystem lagging |
| `tasks.turns_used` p99 | > 0.8 × `agent.max_turns` | Agents near turn limit |
| `llm.latency_ms{model}` p95 | > 30,000 ms | LLM latency degradation |

### Info alerts (track trends)

| Condition | Threshold | Meaning |
|---|---|---|
| `learn.router_tier{tier="T0"}` fraction | < 0.3 | T0 rules under-utilized |
| `gate.escalation_total` rate | Rising week-over-week | Gate quality degrading |
| `memory.gc_runs_total` rate | > 4/hour | Substrate GC running frequently |

---

## PerformanceRegressionPulse (Built)

`roko-orchestrator` emits a `PerformanceRegressionPulse` event after each task completes.
This event contains:

```json
{
  "event":       "PerformanceRegressionPulse",
  "task_id":     "task-7f3a",
  "gate_p50_ms": 14230,
  "gate_p99_ms": 58100,
  "llm_p50_ms":  4800,
  "llm_p99_ms":  18200,
  "regression":  false,
  "welch_p":     0.12,
  "cohens_d":    0.31
}
```

If `regression = true`, the task's timing was statistically worse than the rolling
baseline (Welch's t-test p < 0.05 and Cohen's d > 0.5). Log this event and investigate
the cause.

```bash
# Show regression pulses for recent tasks
roko events dump --all --type PerformanceRegressionPulse | jq 'select(.regression == true)'
```

---

## OpenTelemetry Integration (Specified)

When OTLP export is configured, Roko emits spans compatible with any OpenTelemetry backend
(Jaeger, Tempo, Honeycomb, etc.).

```toml
[observability]
tracing_enabled = true
otlp_endpoint   = "http://otel-collector:4317"
service_name    = "roko"
```

Span hierarchy (Specified design):

```
task: <task-id>
  └── agent: turn=1
  └── agent: turn=2
  └── gate: rung=compile
  └── gate: rung=test
        └── llm: model=claude-opus-4-5
```

Each span carries `task_id`, `error_code` (if applicable), and standard OTEL attributes.

---

## Dashboard Quick-Start

For operators using Grafana + Prometheus (or compatible):

1. Configure Prometheus scrape:
   ```yaml
   scrape_configs:
     - job_name: roko
       static_configs:
         - targets: ['localhost:9090']  # roko-serve metrics endpoint
   ```

2. Import the Roko Grafana dashboard (when available; track
   [`github.com/nunchi/roko`](https://github.com/nunchi/roko) for release).

3. Set up alerts using the thresholds in the Alerting Thresholds section above.

---

## See also

- [07-forensic-replay.md](07-forensic-replay.md) — using event log for deep debugging
- [06-cascade-failure.md](06-cascade-failure.md) — cascade detection via metrics
- [../performance/06-profiling-guide.md](../performance/06-profiling-guide.md) — performance-specific instrumentation
- [09-failure-drill-examples.md](09-failure-drill-examples.md) — using observability in drills
