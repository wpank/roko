# M166 — Wire Observability as Lens Pipeline

## Objective
Wire observability as a Lens Pipeline in `roko-serve`. The `/metrics` route stub and prometheus integration already exist (see `routes/status/metrics.rs`), but structured log emission as Bus Pulses and StateHub projections (`system_health`, `cost_meter`, `error_rate`) are not yet wired. Complete the observability pipeline so that telemetry flows from structured logs through Bus Pulses (topic: `telemetry.log.*`) into queryable StateHub projections that feed the `/metrics` endpoint and TUI dashboard.

## Scope
- Crates: `roko-serve`, `roko-cli`
- Files:
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/status/metrics.rs` (extend Prometheus endpoint)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/state.rs` (add StateHub projections)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/lib.rs` (wire telemetry bus subscription)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs` (emit telemetry Pulses)
- Depth doc: `tmp/unified-depth/09-telemetry/01-observability-as-lens-pipeline.md`

## Steps
1. Read existing metrics route and state:
   ```bash
   grep -n 'pub async fn\|pub fn\|metrics\|prometheus' /Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/status/metrics.rs | head -20
   grep -n 'AppState\|pub struct\|projections\|health\|cost\|error_rate' /Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/state.rs | head -20
   ```

2. Check what telemetry infrastructure already exists:
   ```bash
   grep -rn 'telemetry\|Pulse\|BusSender\|event_bus' /Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/ --include='*.rs' | head -15
   grep -rn 'system_health\|cost_meter\|error_rate' /Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/ --include='*.rs' | head -10
   ```

3. Add StateHub projection structs to `state.rs`:
   ```rust
   /// Lens Pipeline projection: system health summary.
   #[derive(Debug, Default, Serialize)]
   pub struct SystemHealthProjection {
       pub uptime_seconds: u64,
       pub active_agents: u32,
       pub pending_tasks: u32,
       pub gate_pass_rate: f64,
       pub last_heartbeat: Option<Instant>,
   }

   /// Lens Pipeline projection: cost meter.
   #[derive(Debug, Default, Serialize)]
   pub struct CostMeterProjection {
       pub total_tokens_in: u64,
       pub total_tokens_out: u64,
       pub total_cost_usd: f64,
       pub cost_per_task_avg: f64,
       pub budget_remaining: f64,
   }

   /// Lens Pipeline projection: error rate tracker.
   #[derive(Debug, Default, Serialize)]
   pub struct ErrorRateProjection {
       pub errors_1m: u32,
       pub errors_5m: u32,
       pub errors_1h: u32,
       pub error_rate_per_minute: f64,
       pub top_error_kinds: Vec<(String, u32)>,
   }
   ```

4. Wire structured log emission in orchestrate.rs:
   ```bash
   grep -n 'tracing::info\|tracing::warn\|tracing::error\|emit\|bus.*send' /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs | head -15
   ```
   At key points (task start, gate result, agent dispatch, errors), emit Bus Pulses with topic `telemetry.log.{level}`.

5. Create telemetry Bus subscription in roko-serve that updates projections:
   ```rust
   /// Subscribe to telemetry Bus Pulses and update StateHub projections.
   pub fn spawn_telemetry_subscriber(state: Arc<AppState>) -> JoinHandle<()> {
       tokio::spawn(async move {
           // Subscribe to telemetry.log.* topics
           // Update system_health, cost_meter, error_rate projections
       })
   }
   ```

6. Extend `/metrics` endpoint to expose projections as Prometheus gauges/counters:
   - `roko_agents_active` (gauge)
   - `roko_tasks_pending` (gauge)
   - `roko_gate_pass_rate` (gauge)
   - `roko_tokens_total{direction="in|out"}` (counter)
   - `roko_cost_usd_total` (counter)
   - `roko_errors_total{kind="..."}` (counter)

7. Wire telemetry subscriber startup in serve `lib.rs` alongside existing spawns.

## Verification
```bash
cargo check -p roko-serve
cargo clippy -p roko-serve --no-deps -- -D warnings
cargo test -p roko-serve -- metrics
cargo check -p roko-cli
```

## What NOT to do
- Do NOT add prometheus crate if it's already a dependency — check Cargo.toml first
- Do NOT replace existing tracing infrastructure — add Bus Pulse emission alongside tracing
- Do NOT make projections persist to disk — they are in-memory views rebuilt on startup
- Do NOT implement full OpenTelemetry — this is a lightweight Bus→projection pipeline
- Do NOT modify the TUI in this batch — dashboard integration is separate
