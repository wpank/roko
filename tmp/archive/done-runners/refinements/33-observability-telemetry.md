# Observability & Telemetry

> **TL;DR**: The most distinctive claim Roko makes — every agent
> turn, every gate, every heuristic, every cost in one
> content-addressed, queryable substrate — lives or dies by its
> observability. This doc consolidates the metrics, logs, traces,
> events, replay primitives, and cost visibility scattered across
> 22, 23, 24, 26, 27, 30, 32 into a single instrumentation spec.
> What ships with the binary, what's pluggable, what's Roko-specific
> (c-factor, demurrage balance, calibration drift), and what
> integrates with existing monitoring stacks.

> **For first-time readers**: "Observability" here covers five
> things: structured logs, Prometheus-compatible metrics, OpenTelemetry
> traces, Bus events (internally), and episode replay (time-travel).
> Read 24 §5 first for the deployment-level overview; this doc is
> the depth. The key distinction: *generic* observability (every
> framework has metrics) vs *Roko-specific* (c-factor, demurrage
> balance, heuristic calibration drift). The Roko-specific metrics
> are the reason the system is worth running; they deserve first-class
> treatment.

## 1. The four telemetry surfaces

Roko produces telemetry in four modes, each with a different consumer:

1. **Logs** (stderr, JSON by default) — for humans inspecting a
   session, for `docker logs`/`journalctl`/log aggregators.
2. **Metrics** (`/metrics` Prometheus exposition) — for scraping by
   Prometheus, VictoriaMetrics, Datadog, etc.
3. **Traces** (OpenTelemetry spans over OTLP) — for distributed
   tracing: which operator took how long, how did the call tree
   branch, which agent owns which span.
4. **Events** (Bus pulses + StateHub projections) — internal-first,
   but externally consumable via `27-realtime-event-surface.md`.

Each is pluggable; defaults ship. Operators can redirect any of the
four without changing kernel code.

## 2. Structured log format

Every log line is a single JSON object:

```json
{
  "ts": "2026-04-16T13:42:15.312Z",
  "level": "info",
  "target": "roko_orchestrator::plan",
  "fields": {
    "plan_id": "p_abc123",
    "task_id": "t_def456",
    "agent_id": "ag_gh78",
    "message": "task dispatched",
    "elapsed_ms": 12
  },
  "span": {
    "name": "op.route",
    "id": "sp_1a2b3c",
    "parent_id": "sp_0a0b0c"
  },
  "trace_id": "4a1e9b..."
}
```

Every Bus publish emits a corresponding log line if log level is
`info` or below. Large bodies (>1 KB) are replaced with `{hash: ...,
len: N}` to keep logs parseable.

Plain-text mode (`--log-format human`) for interactive sessions;
default is JSON.

## 3. Generic metrics (every runtime has these)

| Metric | Type | Labels | Purpose |
|---|---|---|---|
| `roko.http.requests_total` | counter | method, path, status | HTTP control plane traffic |
| `roko.http.request_duration_seconds` | histogram | method, path | Latency |
| `roko.process.cpu_seconds_total` | counter | — | CPU usage |
| `roko.process.memory_bytes` | gauge | — | RSS |
| `roko.process.open_files` | gauge | — | fd count |
| `roko.tokio.tasks_active` | gauge | — | async task count |
| `roko.tokio.blocking_tasks` | gauge | — | blocking pool usage |

These are table stakes; standard exporters handle them.

## 4. Safety-relevant metrics (from 32)

| Metric | Type | Labels | Purpose |
|---|---|---|---|
| `roko.safety.authz_total` | counter | role, action, decision | Who's getting denied |
| `roko.safety.confirms_pending` | gauge | role | Unanswered confirms |
| `roko.safety.escalations_total` | counter | reason | Escalation rate |
| `roko.safety.taint_propagations` | counter | from, to | Taint fan-out |
| `roko.safety.plugin_violations_total` | counter | plugin, kind | Sandbox breaches |
| `roko.network.egress_total` | counter | host, status | External calls |

An operator dashboard for security builds entirely on these.

## 5. Roko-specific metrics (the interesting ones)

Metrics that *only* make sense in Roko. These are the ones to
surface loudly:

### 5.1 Collective intelligence

| Metric | Type | Labels | From |
|---|---|---|---|
| `roko.c_factor` | gauge | cohort | 13 §2.3 |
| `roko.turn_taking_entropy` | gauge | cohort | 13 §2.2 |
| `roko.peer_prediction_accuracy` | gauge | cohort | 13 §2.2 |
| `roko.citation_reciprocity` | gauge | cohort | 13 §2.2 |
| `roko.hdc_diversity` | gauge | cohort | 13 §2.2 |
| `roko.cohort_delivery_rate` | gauge | cohort | 13 §2.2 |

### 5.2 Memory economy

| Metric | Type | Labels | From |
|---|---|---|---|
| `roko.demurrage.balance_p50` | histogram | kind | 12 §9 |
| `roko.demurrage.balance_p95` | histogram | kind | 12 §9 |
| `roko.demurrage.thaw_total` | counter | kind | 12 §9 |
| `roko.demurrage.reinforce_total` | counter | kind, reinforce_kind | 12 §9 |
| `roko.substrate.engrams_warm` | gauge | kind | 12 |
| `roko.substrate.engrams_cold` | gauge | kind | 12 |
| `roko.substrate.query_latency_ms` | histogram | query_kind | existing |
| `roko.substrate.query_similar_latency_ms` | histogram | — | 11 §4.1 |

### 5.3 Learning

| Metric | Type | Labels | From |
|---|---|---|---|
| `roko.heuristic.total` | gauge | calibration_bucket | 14 |
| `roko.heuristic.calibration_brier` | histogram | heuristic_id | 14 §2 |
| `roko.heuristic.trials_total` | counter | heuristic_id | 14 §3.3 |
| `roko.replication.ledger_total` | gauge | status | 16 §5 |
| `roko.prediction.ema_error` | gauge | operator | 10 §5 |
| `roko.prediction.rmse` | gauge | operator | 10 §10 |

### 5.4 Gate pipeline

| Metric | Type | Labels | From |
|---|---|---|---|
| `roko.gate.verdicts_total` | counter | gate, passed | existing |
| `roko.gate.failure_rate` | gauge | gate | 10 §7.1 |
| `roko.gate.latency_ms` | histogram | gate | existing |
| `roko.gate.pipeline_duration_ms` | histogram | — | existing |

### 5.5 Bus

| Metric | Type | Labels | From |
|---|---|---|---|
| `roko.bus.pulses_total` | counter | topic | 03 |
| `roko.bus.ring_occupancy` | gauge | bus_name | 03 §8 |
| `roko.bus.ring_capacity` | gauge | bus_name | 03 §2 |
| `roko.bus.subscribers_active` | gauge | topic_pattern | 03 |
| `roko.bus.lagging_subscribers_total` | counter | topic_pattern | 03 §8 |

### 5.6 Cost

| Metric | Type | Labels | From |
|---|---|---|---|
| `roko.cost.tokens_total` | counter | model, role | 24 §10 |
| `roko.cost.usd_total` | counter | model, role | 24 §10 |
| `roko.cost.budget_remaining_usd` | gauge | budget_scope | 24 §10 |
| `roko.cost.cascade_router_decisions_total` | counter | model_selected | existing |

All metrics follow Prometheus naming conventions (lowercase,
underscore-separated, `_total` suffix for counters).

## 6. Traces — OpenTelemetry spans

Every operator boundary emits a span:

```
op.sense        (step 1 of the loop)
  ├─ substrate.query
  └─ bus.receive
op.assess       (step 2)
  └─ router.select
     └─ cascade_router.decide
op.compose      (step 3)
  └─ composer.compose
     └─ substrate.query_similar      (HDC retrieval)
op.act          (step 4)
  └─ agent.llm_call
     ├─ tool.call               (multiple)
     └─ bus.publish             (token chunks)
op.verify       (step 5)
  └─ gate.pipeline
     ├─ gate.compile
     ├─ gate.test
     └─ gate.clippy
op.persist      (step 6a)
  └─ substrate.put
op.broadcast    (step 6b)
  └─ bus.publish
op.react        (step 7)
  └─ policy.decide            (per-policy child span)
```

Span attributes include `operator_id`, `principal_id`,
`content_hash`, `pulse_seq` where relevant. Trace id flows through
the ctx argument of every operator.

Exporters: OTLP, Jaeger, Zipkin. `OTEL_EXPORTER_OTLP_ENDPOINT` env
configures.

## 7. Events — StateHub projections as a telemetry target

Consumers that want *typed, filterable, queryable* telemetry subscribe
to projections via `26-statehub-rearchitecture.md`. This is a
first-party alternative to scraping Prometheus:

- `cohort_health` → live c-factor snapshot + roster.
- `gate_pipeline` → current rung status, pass/fail counts.
- `bus_stats` → pulses/sec by topic.
- `substrate_stats` → balance histogram, tier sizes.
- `cost_meter` → per-model spend.
- `safety_events` → recent authz denials, confirms, escalations.
- `replication_ledger` → claim status table.
- `calibration_curves` → per-operator error trends.

A Grafana data source plugin can consume any projection as a stream
or as a point-in-time snapshot. Operators get live, typed
observability without rolling their own scraping.

## 8. Alerts

Default alert rules shipping with Roko (Prometheus/Alertmanager
format):

```yaml
# alerts/roko.yml
groups:
- name: roko_critical
  rules:
    - alert: RokoCFactorDropping
      expr: rate(roko.c_factor[10m]) < -0.05
      for: 15m
    - alert: RokoGatePipelineStalled
      expr: rate(roko.gate.verdicts_total[5m]) == 0
      for: 10m
    - alert: RokoSafetyEscalationSurge
      expr: rate(roko.safety.escalations_total[5m]) > 1
    - alert: RokoDemurrageSubstrateBloat
      expr: roko.substrate.engrams_warm > 10_000_000
    - alert: RokoCalibrationDriftSpike
      expr: rate(roko.prediction.ema_error[30m]) > 0.1
    - alert: RokoBusRingSaturation
      expr: roko.bus.ring_occupancy / roko.bus.ring_capacity > 0.9
    - alert: RokoCostSurge
      expr: rate(roko.cost.usd_total[1h]) > 5
    - alert: RokoPluginViolationsSpike
      expr: rate(roko.safety.plugin_violations_total[5m]) > 0.1
```

Each alert has a runbook URL in its `annotations` pointing at
`docs/runbooks/<name>.md`.

## 9. Cost dashboard as a first-class page

Given Roko's cost visibility claim (24 §10), cost gets its own
telemetry surface beyond raw metrics:

- **Per-session spend**: live counter in the CLI prompt and web UI.
- **Per-task breakdown**: after a `plan run`, a table like
  `task_id | model | tokens_in | tokens_out | usd | seconds`.
- **Per-role historical**: `roko cost report --period 7d --by role`.
- **Per-model**: which models are earning their keep.
- **Budget vs burn**: visualizes budget consumption rate over time.

Tiles on the web UI Home (29 §3.1) and the TUI Cost tab read these
from the `cost_meter` projection.

## 10. Replay as observability

Time-travel through any decision:

```bash
# Replay an episode inspecting what the agent saw
roko replay ep_12345 --trace
# Replay with alternate config to test sensitivity
roko replay ep_12345 --override "demurrage.flat_tax=0.02"
# Replay to generate an audit report
roko replay ep_12345 --audit > report.md
```

Replay consumes the Engram + Pulse history of the episode (if the
ring hadn't wrapped on Pulses, a fresh subscribe — if it had,
reconstructed from graduated Engrams). Operators walking through a
postmortem use replay to answer "what did the agent know at this
moment?"

## 11. Self-observability of the observability surface

The observability layer itself is instrumented:

- Log producer queue depth.
- Log dropped lines (when queue overflows).
- Metric scrape duration.
- Trace span drops (when the exporter can't keep up).
- StateHub projection delta latency.

`/readyz` returns `not ready` if any observability sink is
unavailable beyond a threshold, so rollouts don't black-hole
observability.

## 12. Integration with existing stacks

Typical deployment stacks and how Roko plugs in:

| Stack | Roko surface used | Integration |
|---|---|---|
| Prometheus + Grafana | `/metrics`, alerts | `helm install prometheus-community/...`; Grafana dashboards shipped |
| Loki / Elastic / Datadog Logs | stdout JSON | Standard container log shipping |
| Jaeger / Zipkin / Tempo / Honeycomb | OTLP | `OTEL_EXPORTER_OTLP_ENDPOINT` env |
| Sentry / Bugsnag | stderr + crash handler | Crash reporter plugin (tier-3) |
| Slack / PagerDuty | Alert routes | Alertmanager receiver |
| Custom dashboards | StateHub + realtime | `@roko/client` subscription |

Roko provides stock Grafana dashboards in
`deployment/grafana/roko-overview.json`,
`roko-safety.json`, `roko-cognitive.json` (the last one is the
unique surface no other framework has).

## 13. Structured-log heuristics

For humans grepping logs:

- Every log line that crosses a safety boundary includes
  `safety_decision=<decision>`.
- Every cost-bearing action includes `usd=<amount>`.
- Every Engram write includes `engram_kind` and `engram_hash`.
- Every Pulse publish includes `topic`.
- Every gate verdict includes `gate`, `passed`.

Shared labels enable aggregations across log lines from different
subsystems. An operator grepping
`safety_decision=escalate` finds every escalation in a session.

## 14. Debug mode

`roko --debug` enables:

- All tracing spans logged as events (even ones the exporter
  discards).
- Every Bus publish also logged at `info`.
- Every Substrate put logged at `debug` with body length.
- Profiling data dumped to `.roko/debug/profile-<ts>.json`.
- `RUST_BACKTRACE=1` implicit.

`--debug` is for postmortems and development. Production always
runs without it.

## 15. Retention and sampling

Not all telemetry retains forever:

- Metrics: whatever the downstream stack retains (Prometheus default
  15 days).
- Logs: whatever the log shipping stack retains.
- Traces: sampling rate configurable; default 10% sample; 100% sample
  on error.
- Bus Pulses: ring-buffer retention (default 4096 per bus); graduated
  Engrams persist per demurrage.
- Engrams: demurrage-managed; see 12.
- Custody records: retained long-term (compliance-tier storage);
  optionally chain-witnessed (Phase 2+).

Each has a CLI for local inspection:
`roko logs tail`, `roko metrics show`, `roko traces find`,
`roko bus replay`, `roko substrate stats`, `roko custody list`.

## 16. The observability pitch

"Every agent turn has a trace. Every heuristic has a calibration
curve. Every gate has a latency histogram. Every cohort has a
c-factor you can watch in real time. Every cost is attributed.
Every safety decision is audited. Every decision is replayable."

Few frameworks make any of these claims. None make all of them. Roko
can, and should, because the substrate was designed for it. The
instrumentation in this doc is the line between that claim being
marketing and being reality.

## 17. Staging

Most of the generic surface already exists in `roko-runtime`. The
gap list:

1. **Roko-specific metrics** (§5) — two weeks to wire the dozens of
   new gauges/counters to the actual subsystems. Most of the data
   already exists on the Bus; this is plumbing.
2. **Default Grafana dashboards** — one week.
3. **StateHub projections for telemetry** — two weeks, depends on 26
   landing.
4. **Alert rules and runbooks** — one week.
5. **Cost report CLI** — three days.
6. **Replay-with-override CLI** — one week.
7. **Grafana data-source plugin for StateHub** — two weeks.

Total: two months of focused observability work. After, Roko is
legible.

## 18. Cross-references

- Bus-related metrics home: `03-bus-as-first-class.md` §5, §8.
- StateHub surface that feeds most live telemetry:
  `26-statehub-rearchitecture.md`.
- Realtime surface for external consumers:
  `27-realtime-event-surface.md`.
- Cost story: `24-deployment-ux.md` §10, `28-cli-parity-familiar-workflows.md` §9.
- Safety events: `32-safety-sandbox-provenance.md` §14.
- c-factor dashboard tile: `13-collective-intelligence-c-factor.md` §7.
- Demurrage balance histogram: `12-knowledge-demurrage.md` §9.
- Rich UX primitives that render telemetry:
  `30-rich-ux-primitives.md`.
