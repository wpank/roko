# M157 — Add Prometheus Metrics Endpoint

## Objective
Add a Prometheus-compatible metrics endpoint to `roko-serve` at `GET /metrics`. Expose key operational metrics: agent ticks by tier, gate verdicts by result, inference cost as histogram, knowledge signal counts, and bus pulse totals. Wire metric recording into the orchestrate.rs event loop so metrics are populated during execution.

## Scope
- Crates: `roko-serve`, `roko-cli`
- Files:
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/` (add metrics route)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/state.rs` (add metrics state)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs` (record metrics)
- Depth doc: `tmp/unified-depth/14-deployment/` (observability)

## Steps
1. Check if prometheus crate is already a dependency:
   ```bash
   grep -rn 'prometheus\|metrics' /Users/will/dev/nunchi/roko/roko/crates/roko-serve/Cargo.toml | head -5
   grep -rn 'prometheus' /Users/will/dev/nunchi/roko/roko/Cargo.toml | head -5
   ```

2. Read existing metrics routes:
   ```bash
   grep -rn 'metrics\|/metrics' /Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/ -r --include='*.rs' | head -10
   ls /Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/status/ 2>/dev/null
   ```

3. Read the serve state to understand how to add shared state:
   ```bash
   grep -n 'pub struct.*State\|AppState\|SharedState' /Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/state.rs | head -10
   ```

4. Add `prometheus` crate dependency to roko-serve Cargo.toml (if not present):
   ```toml
   [dependencies]
   prometheus = { version = "0.13", features = ["process"] }
   ```

5. Define metrics registry in state.rs or a new `metrics.rs`:
   ```rust
   use prometheus::{Registry, IntCounterVec, HistogramVec, IntGauge, Opts, HistogramOpts};

   pub struct PrometheusMetrics {
       pub registry: Registry,
       pub agent_ticks_total: IntCounterVec,      // labels: tier
       pub gate_verdicts_total: IntCounterVec,     // labels: result (pass/fail/skip)
       pub inference_cost_usd: HistogramVec,       // labels: model
       pub knowledge_signals_total: IntGauge,
       pub bus_pulses_total: IntCounterVec,        // labels: topic
   }

   impl PrometheusMetrics {
       pub fn new() -> Self {
           let registry = Registry::new();

           let agent_ticks = IntCounterVec::new(
               Opts::new("roko_agent_ticks_total", "Total agent ticks by tier"),
               &["tier"],
           ).unwrap();

           let gate_verdicts = IntCounterVec::new(
               Opts::new("roko_gate_verdicts_total", "Gate verdicts by result"),
               &["result"],
           ).unwrap();

           let inference_cost = HistogramVec::new(
               HistogramOpts::new("roko_inference_cost_usd", "Inference cost in USD")
                   .buckets(vec![0.001, 0.01, 0.05, 0.10, 0.50, 1.0, 5.0]),
               &["model"],
           ).unwrap();

           let knowledge_signals = IntGauge::new(
               "roko_knowledge_signals_total", "Total knowledge signals in store"
           ).unwrap();

           let bus_pulses = IntCounterVec::new(
               Opts::new("roko_bus_pulses_total", "Total bus pulses by topic"),
               &["topic"],
           ).unwrap();

           registry.register(Box::new(agent_ticks.clone())).unwrap();
           registry.register(Box::new(gate_verdicts.clone())).unwrap();
           registry.register(Box::new(inference_cost.clone())).unwrap();
           registry.register(Box::new(knowledge_signals.clone())).unwrap();
           registry.register(Box::new(bus_pulses.clone())).unwrap();

           Self { registry, agent_ticks_total: agent_ticks, gate_verdicts_total: gate_verdicts,
                  inference_cost_usd: inference_cost, knowledge_signals_total: knowledge_signals,
                  bus_pulses_total: bus_pulses }
       }
   }
   ```

6. Add `GET /metrics` route handler:
   ```rust
   pub async fn metrics_handler(State(state): State<AppState>) -> impl IntoResponse {
       use prometheus::Encoder;
       let encoder = prometheus::TextEncoder::new();
       let mut buffer = Vec::new();
       encoder.encode(&state.metrics.registry.gather(), &mut buffer).unwrap();
       (
           [(axum::http::header::CONTENT_TYPE, "text/plain; version=0.0.4")],
           String::from_utf8(buffer).unwrap(),
       )
   }
   ```

7. Wire metric recording into orchestrate.rs:
   - After each agent dispatch: `metrics.agent_ticks_total.with_label_values(&[tier]).inc()`
   - After each gate verdict: `metrics.gate_verdicts_total.with_label_values(&[result]).inc()`
   - After each inference: `metrics.inference_cost_usd.with_label_values(&[model]).observe(cost)`

8. Write tests:
   - `GET /metrics` returns 200 with `text/plain` content type
   - Output contains `roko_agent_ticks_total`
   - Counter increments correctly after recording

## Verification
```bash
cargo check -p roko-serve
cargo clippy -p roko-serve --no-deps -- -D warnings
cargo test -p roko-serve -- metrics
cargo check -p roko-cli
```

## What NOT to do
- Do NOT add grafana dashboards — only the metrics endpoint
- Do NOT add tracing/OpenTelemetry in this batch — only Prometheus counters/histograms
- Do NOT expose /metrics without the /api prefix if existing routes use /api — check convention
- Do NOT add high-cardinality labels (agent_id, task_id) — only tier, result, model, topic
- Do NOT block the request path to compute metrics — use atomic counters
