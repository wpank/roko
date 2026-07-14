# E09 — Observability

> Executable backlog epic · verified against HEAD 5852c93c05 · sources: `53-OBSERVABILITY`, `97-TRACE-SERVE-LIFECYCLE`
> Native task schema: `crates/roko-cli/src/task_parser.rs::TaskDef` · exemplars: `plans/P11-runner-v2-default/tasks.toml`
> **Depends on: E01** (runner-v2 must be the default plan engine — metric hooks live in the v2 event loop).

## Why this epic

Unattended self-hosting means nobody is watching the terminal. The operator's only honest window into a
run is metrics, logs, and traces. Today that window is **boarded up**: the runner builds a metric registry
and immediately drops it, three logs grow unbounded (`chain-watcher.log` 23 MB, `roko.log` 12 MB, no
rotation anywhere), and the "run event log" (`.roko/events.jsonl`, 44 MB) is **97.3% feed-agent UI noise**.
An autonomous run today emits almost nothing an operator can trust.

## The one fix that matters

**Thread the already-built `MetricRegistry` into `RunConfig.metrics` (E09-T01).**
`commands/plan.rs:379-380` constructs a registry and calls `register_standard_metrics`, then builds
`RunConfig` with `metrics: None` at `:569` — the registry is dropped on the floor. The gate-verdict counter
at `runner/event_loop.rs:1024-1037` is guarded by `if let Some(ref metrics) = config.metrics`, so it **never
fires**. `grep 'metrics: Some' crates/` → 0. Changing `None` → `Some(metrics.clone())` is a one-line change
that turns the entire runner-v2 metric pipe from dead to live. Everything else in this epic (disk dump, serve
wiring, rotation, firehose trim) is downstream of that single edit.

## Obs-signal map (signal → emitted → written → read → gap)

| Signal | Emitted at | Written to | Read by | Gap |
|---|---|---|---|---|
| Gate-verdict counter (`ROKO_GATE_VERDICTS_TOTAL`) | `runner/event_loop.rs:1024-1037` (guarded by `config.metrics`) | nowhere — registry dropped | nobody | **DEAD**: `RunConfig.metrics=None` at every site (plan.rs:569, serve_runtime.rs:628, types.rs:1458/1499) → E09-T01/T02/T03 |
| LLM-call metrics (per provider/model) | `model_call_service.rs:100-102,282` | `MetricRegistry` (serve only) | `GET /metrics`, `/api/metrics` | 🟡 serve-only; CLI runs blind → E09-T02 |
| Tool traces | `JsonlTraceSink` (roko-fs/trace_sink.rs) | `.roko/traces/<date>/<id>.jsonl` | nobody | 🔌 dirs bootstrapped (main.rs:3098-3110), sink attached only in legacy orchestrate.rs → **dir empty** → E09-T08 |
| Tool metrics | `JsonlMetricsSink` (tool_metrics_sink.rs) | `.roko/metrics/tool_metrics.jsonl` | nobody | 🔌 same attach gap → **dir empty** → E09-T08 |
| CLI tracing spans/logs | `main.rs:2074-2145` file layer | `.roko/roko.log` (plain append, **no rotation**, 12 MB) | humans; serve tails 50 lines | ❌ unbounded growth → E09-T05 |
| chain-watcher subprocess | serve redirect `lib.rs:440-444` | `.roko/chain-watcher.log` (**23 MB, no rotation**) | nobody | ❌ largest log, write-only → E09-T05 |
| StateHub run/gate/task events | runner persist.rs:282 + StateHub publish (state_hub.rs:59,75,107-125) | `.roko/events.jsonl` | TUI @render, SSE/WS, resume/replay | 🟡 buried under feed noise (1:200 SNR) → E09-T04/T07 |
| `feed_tick` firehose | 15 serve feed agents `feed_agents/mod.rs:90` → dashboard_snapshot.rs:201 | `.roko/events.jsonl` | TUI feed pane, relay | 🟡 **152,965/157,264 lines (97.3%)** — ephemeral UI pulse persisted to disk → E09-T04 |
| Log verbosity knob | `main.rs:2293` reads `RUST_LOG`+`ROKO_LOG`; chain-watcher reads only `ROKO_LOG` (main.rs:44); agent-relay its own default | — | env | 🟡 one binary's `RUST_LOG` ≠ another's → E09-T06 |
| OTLP export | `init_otlp_tracing` roko-serve/lib.rs:2879-2909 | — (logs "deferred" and returns) | — | ❌ stub, feature not in default → out of scope (delete-or-build, tracked in 53-OBSERVABILITY §checklist) |
| v2 Lens pipeline (Collector/Transform/Export) | — | — | — | ❌ **0% built** (`grep '\bLens\b' crates/` → 0) → E09-T09 (design) |

## Task breakdown (E09-Txx)

| Task | Tier | Summary | Depends |
|---|---|---|---|
| **E09-T01** | mechanical | Thread the built `MetricRegistry` into `RunConfig.metrics` at `plan.rs:569` (`None`→`Some(metrics.clone())`) so the gate-verdict counter fires | E01 |
| **E09-T02** | small | Dump `metrics.render_prometheus()` → `.roko/metrics/prometheus.txt` at end of every runner-v2 plan run (CLI runs have no `/metrics` HTTP endpoint) | E09-T01 |
| **E09-T03** | mechanical | Thread the serve `AppState` registry into `RunConfig.metrics` at `serve_runtime.rs:628` so serve-launched runs show gate counters on the **live** `GET /metrics` | E09-T01 |
| **E09-T04** | small | Stop persisting `feed_tick` (and `chain_block`) to `.roko/events.jsonl` — filter `DashboardEvent::FeedTick` at the StateHub `EventLogWriter` append (`state_hub.rs`) so only resume-critical events hit disk | E01 |
| **E09-T05** | small | Add day-based rotation via `tracing_appender::rolling::daily` for `.roko/roko.log` (main.rs:2100-2120) and `.roko/chain-watcher.log` (serve redirect lib.rs:440-444) | — |
| **E09-T06** | mechanical | Make `ROKO_LOG` authoritative across all binaries — `roko-chain-watcher` (main.rs:42-49) and `agent-relay` (main.rs:40-43) honor the same env var as CLI/serve; document in `roko doctor` | — |
| **E09-T07** | small | GC/cap `.roko/events.jsonl` (size-based truncation or split into `run-events.jsonl` vs disk-less feed channel) so the file tracks run volume, not uptime | E09-T04 |
| **E09-T08** | medium | Attach `FsObservabilitySinks` in the runner-v2 tool loop (or delete the sinks + dir bootstrap at main.rs:3098-3110) — stop shipping empty `.roko/traces/` + `.roko/metrics/` on every run | E01 |
| **E09-T09** | design | Design the v2 telemetry-as-Lens pipeline (CollectorLens → TransformLens → ExportLens) reconciling the hand-rolled `MetricRegistry` (obs/metrics.rs:263) with `docs/v2-depth/09-telemetry` — longer-horizon, deliverable is a design doc + trait sketch, not code | E09-T01..T08 |

## First 3 tasks (executable TOML)

```toml
[meta]
plan = "E09-OBSERVABILITY"
total = 9
done = 0
status = "ready"
max_parallel = 1

# ─────────────────────────────────────────────────────────────────────────────
# E09-T01: Thread the built MetricRegistry into RunConfig.metrics (THE fix)
# ─────────────────────────────────────────────────────────────────────────────
#
# commands/plan.rs:379-380 already builds a MetricRegistry and calls
# register_standard_metrics(&metrics) — then throws it away by passing
# `metrics: None` when constructing RunConfig at :569. The gate-verdict counter
# at runner/event_loop.rs:1024-1037 is guarded by `if let Some(ref metrics) =
# config.metrics`, so it never records. `grep 'metrics: Some' crates/` → 0.
#
# Change EXACTLY the one field at plan.rs:569:
#   metrics: None,
# to:
#   metrics: Some(metrics.clone()),
#
# The `metrics` binding is already in scope (declared :379). Do NOT rebuild it.
#
[[task]]
id = "E09-T01"
title = "Thread MetricRegistry into RunConfig.metrics (plan.rs)"
status = "ready"
tier = "mechanical"
model_hint = "claude-haiku-4-5"
max_loc = 5
files = ["crates/roko-cli/src/commands/plan.rs"]
role = "implementer"
depends_on = []

[task.context]
read_files = [
    { path = "crates/roko-cli/src/commands/plan.rs", lines = "376-570", why = "Registry built at :379-380 but RunConfig.metrics is None at :569 — the drop site" },
    { path = "crates/roko-cli/src/runner/event_loop.rs", lines = "1015-1040", why = "Gate-verdict counter is guarded by `if let Some(ref metrics) = config.metrics` — dead until this is Some" },
    { path = "crates/roko-cli/src/runner/types.rs", lines = "1336-1339", why = "RunConfig.metrics field type: Option<Arc<MetricRegistry>>" },
]
symbols = ["MetricRegistry", "RunConfig", "register_standard_metrics"]
anti_patterns = [
    "Do NOT construct a new MetricRegistry — reuse the `metrics` binding already declared at plan.rs:379.",
    "Do NOT change the field type in types.rs; it is already Option<Arc<MetricRegistry>>.",
    "Do NOT touch serve_runtime.rs here — that is E09-T03.",
]

[[task.verify]]
phase = "structural"
command = "grep -q 'metrics: Some(metrics.clone())' crates/roko-cli/src/commands/plan.rs"
fail_msg = "RunConfig.metrics must be Some(metrics.clone()) in plan.rs"

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-cli 2>&1"
fail_msg = "roko-cli must compile after wiring the metric registry"

acceptance = "After a runner-v2 plan run, the gate-verdict counter increments; grep 'metrics: Some' crates/ is no longer empty."


# ─────────────────────────────────────────────────────────────────────────────
# E09-T02: Dump the runner metric registry to disk at end of run
# ─────────────────────────────────────────────────────────────────────────────
#
# CLI plan runs have no /metrics HTTP endpoint, so even with E09-T01 the
# counters live only in memory and vanish when the process exits. Render the
# registry to a Prometheus text file at the end of the runner-v2 plan run, so a
# post-mortem operator (or a scrape sidecar) can read it. Legacy orchestrate.rs
# already does this at :5937-5944 (`.roko/metrics/prometheus.txt`) — mirror that
# path so both engines write the same file.
#
# After the runner returns its PlanRunSummary in commands/plan.rs (post-run,
# after the run_config was consumed), write:
#   std::fs::write(layout.metrics_dir().join("prometheus.txt"),
#                  metrics.render_prometheus())
# Best-effort (`let _ =`); a failed write must not fail the run.
#
[[task]]
id = "E09-T02"
title = "Write runner metrics to .roko/metrics/prometheus.txt post-run"
status = "ready"
tier = "small"
model_hint = "claude-sonnet-4-5"
max_loc = 25
files = ["crates/roko-cli/src/commands/plan.rs"]
role = "implementer"
depends_on = ["E09-T01"]

[task.context]
read_files = [
    { path = "crates/roko-cli/src/commands/plan.rs", lines = "376-600", why = "Registry in scope; find the post-run point after the runner returns to add the dump" },
    { path = "crates/roko-core/src/obs/metrics.rs", lines = "263-300", why = "MetricRegistry::render_prometheus — the renderer to call" },
]
symbols = ["MetricRegistry", "render_prometheus"]
anti_patterns = [
    "Do NOT panic or `?`-propagate on write failure — use best-effort `let _ =`; observability must never fail a run.",
    "Do NOT invent a new path — reuse the layout metrics dir (same file legacy orchestrate.rs:5937-5944 writes).",
    "Do NOT move the registry — clone the Arc if you need it after the runner consumed run_config.",
]

[[task.verify]]
phase = "structural"
command = "grep -q 'prometheus.txt' crates/roko-cli/src/commands/plan.rs"
fail_msg = "post-run Prometheus dump path must be present in plan.rs"

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-cli 2>&1"
fail_msg = "roko-cli must compile after adding the metrics dump"

acceptance = "After `roko plan run <dir>` on the runner-v2 path, `.roko/metrics/prometheus.txt` exists and contains a `roko_gate_verdicts_total` line."


# ─────────────────────────────────────────────────────────────────────────────
# E09-T03: Thread the serve AppState registry into the serve-launched runner
# ─────────────────────────────────────────────────────────────────────────────
#
# serve_runtime.rs:628 builds RunConfig with `metrics: None` too, so runs
# launched from `roko serve` also never populate the registry that backs the
# live `GET /metrics` scrape. AppState already owns a MetricRegistry
# (roko-serve/src/state.rs:947-950); pass that same Arc down so serve-launched
# runs increment counters visible on the running server's /metrics endpoint.
#
# Locate the RunConfig construction in serve_runtime.rs and set:
#   metrics: Some(app_state.metrics.clone()),
# using whatever the in-scope AppState/registry handle is named.
#
[[task]]
id = "E09-T03"
title = "Thread serve AppState MetricRegistry into serve_runtime RunConfig"
status = "ready"
tier = "mechanical"
model_hint = "claude-sonnet-4-5"
max_loc = 10
files = ["crates/roko-cli/src/serve_runtime.rs"]
role = "implementer"
depends_on = ["E09-T01"]

[task.context]
read_files = [
    { path = "crates/roko-cli/src/serve_runtime.rs", lines = "600-640", why = "RunConfig built with metrics: None at :628 — the serve-side drop site" },
    { path = "crates/roko-serve/src/state.rs", lines = "940-960", why = "AppState owns the MetricRegistry that backs GET /metrics — reuse this exact Arc" },
]
symbols = ["MetricRegistry", "RunConfig", "AppState"]
anti_patterns = [
    "Do NOT build a fresh registry — the whole point is to share AppState's registry so counters appear on the LIVE /metrics scrape.",
    "Do NOT change the /metrics route or state.rs; only wire the existing Arc into RunConfig.",
]

[[task.verify]]
phase = "structural"
command = "! grep -n 'metrics: None' crates/roko-cli/src/serve_runtime.rs"
fail_msg = "serve_runtime RunConfig must no longer pass metrics: None"

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-cli 2>&1"
fail_msg = "roko-cli must compile after sharing the serve registry"

acceptance = "With `roko serve` running, launching a plan and then `curl -s localhost:6677/metrics | grep roko_gate_verdicts_total` shows non-zero gate-verdict counters."
```

## Remaining tasks (E09-T04 .. E09-T09)

Authored in the same schema when scheduled; key parameters:

- **E09-T04** (small, `state_hub.rs` + `dashboard_snapshot.rs`): filter `DashboardEvent::FeedTick`/`chain_block` before the `EventLogWriter` append. Verify: after a serve session with feeds running, `grep -c feed_tick .roko/events.jsonl` → 0.
- **E09-T05** (small, `main.rs:2100-2120`, serve `lib.rs:440-444`): swap plain-append file layers for `tracing_appender::rolling::daily`. Verify: `.roko/roko.log.<date>` files appear; old ones age out.
- **E09-T06** (mechanical, `apps/roko-chain-watcher/src/main.rs:42-49`, `apps/agent-relay/src/main.rs:40-43`): honor one authoritative `ROKO_LOG`. Verify: one env var toggles verbosity in all three binaries; `roko doctor` documents it.
- **E09-T07** (small, `roko-fs` layout + StateHub): size-cap/GC `events.jsonl` or split `run-events.jsonl`. Verify: file size tracks run volume, not uptime.
- **E09-T08** (medium, runner-v2 tool loop): attach `FsObservabilitySinks` or delete the dir bootstrap. Verify: after `roko run "…"`, `ls .roko/traces/$(date +%F)/` is non-empty (or dirs no longer created).
- **E09-T09** (design, longer-horizon): Lens-pipeline design doc reconciling `MetricRegistry` with `docs/v2-depth/09-telemetry`; deliverable is trait sketch + migration plan, no runtime code. `grep '\bLens\b' crates/` is the 0% baseline.

## CTRL-08 ownership reconciliation

E09 owns metrics, event persistence policy, and observability exports. T10 is an
acceptance roll-up for E47-T07 JSONL rotation: the sole threshold is
`ResourcesConfig.log_rotation_max_mb` (default 100 MB), appends are serialized, and
readers/GC discover complete timestamped JSONL generations. T11 remains a distinct Prometheus
consumer, but must use the E47-T05 target scanner rather than add another filesystem
walk. See [`17-OPERATIONAL-OWNERSHIP.md`](../17-OPERATIONAL-OWNERSHIP.md).
