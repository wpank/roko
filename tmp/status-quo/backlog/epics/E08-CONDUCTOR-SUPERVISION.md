# E08 — Conductor Supervision For The Live Engine

**Status:** GAP — no existing plan. Adapter sketched in doc `88-CONDUCTOR` but never built.
**Source doc:** `tmp/status-quo/88-CONDUCTOR.md`
**Depends on:** E01 (runner-v2 baseline / shared setup this backlog assumes)
**Owning subsystem:** `roko-conductor` ↔ `roko-cli/src/runner/` (runner-v2 event loop)
**Task count:** 7 (E08-T01 … E08-T07)

---

## Why this epic matters

For unattended / long-running plan execution, the *default* binary today has **no anomaly
supervision beyond static timeouts**. If an agent falls into a ghost-turn loop, a compile-fail
loop, a review-loop, a cost blowout, or a spec-drift spiral, nothing intervenes until the coarse
per-task or plan wall-clock timeout fires (`event_loop.rs` Branch 5, `plan_timeout`). The whole
reactive-intelligence layer that was built to catch exactly these cases is dark at runtime.

## Wiring-gap summary (verified 2026-07-09)

`roko-conductor/` (~10K LOC: 10 watchers, circuit breaker w/ Holt forecaster, 37-pattern /
20-category diagnosis, pattern detector, threshold learner, ~300 tests) is **built and passing**
but has effectively **zero live consumers**:

- **Runner-v2 imports zero conductor symbols.** Verified: `rg 'roko_conductor|roko-conductor'
  crates/roko-cli/src/runner` → 0 matches. The entire `crates/roko-cli/src/runner/` tree
  (`event_loop.rs`, `types.rs`, `runtime_feedback/`, …) never references the conductor.
- **`Conductor::from_config` has zero callers** → every `[conductor.watchers.*]` TOML threshold
  is dead config (`conductor.rs:204`, `configured_watchers` unreachable at runtime).
- **`conductor_load` is hardcoded `0.0`** in the live path: `event_loop.rs:4258` (runner-v2
  routing context) and `runtime_feedback/routing.rs:132`. Model routing therefore never sees
  real system load. (The *dead* `orchestrate.rs` path at line 3016 computes it for real via
  `routing_load_pressure` — proof the value is meaningful, just not wired into the live engine.)
- The only place the conductor is actually driven is dead `orchestrate.rs`
  (`self.conductor.routing_bias()`, `evaluate_full`), which the runner-v2 engine replaced.
- **`RunConfig`'s own doc comment already lies** (`types.rs:1318-1321`): it claims the
  `FeedbackFacade` "fans out to the registered learning / knowledge / **conductor** / dream
  sinks" — but `runtime_feedback/` has no conductor sink. The facade + `FeedbackSink` trait
  (`runtime_feedback/mod.rs:130`) is the ready-made decoration point.

### The minimal adapter (from doc 88)

The conductor's watchers are pure functions over `&[Engram]` (roko-core signals), keyed on
`Kind::PlanPhase`, `Kind::GateVerdict`, `Kind::Metric`, `Kind::AgentOutput`, and ghost-turn
custom kinds, with tags like `plan_id`, `model`, `provider`, `severity`. Runner-v2 speaks a
different vocabulary (`RunnerEvent` / `AgentEvent` enums in `runner/types.rs`). Bridging the two
is small and mechanical:

1. **RuntimeEvent→Engram adapter** — map `RunnerEvent` / `AgentEvent` variants to the `Engram`
   kinds+tags watchers expect (E08-T01).
2. **`conductor_ring` + `EventSink` decorator** — a bounded `VecDeque<Engram>` fed by a
   `FeedbackSink` (or `RunOutputSink`) decorator so every live event lands in the ring (E08-T02).
3. **`Conductor::from_config` wiring** — actually construct the conductor from
   `roko_config.conductor` and thread it (as `Arc<Conductor>` + shared ring) into the event
   loop (E08-T03).
4. **Supervision `select!` branch** — a periodic tick that calls `evaluate_full` over a ring
   snapshot and enforces the decision: `Fail`→cancel the run (`CancellationToken`),
   `Restart`→re-queue the task via the executor (E08-T04).
5. **Real `conductor_load`** — replace the two hardcoded `0.0`s with load derived from the
   conductor's routing bias / circuit-breaker pressure + queue depth (E08-T05).
6. **Snapshot breaker state** — persist `CircuitBreakerState` into the executor snapshot and
   restore via `Conductor::from_circuit_breaker_state` on `--resume` (E08-T06).
7. **Close the loop into routing** — feed `routing_bias()` into cascade routing + a smoke test
   and config docs (E08-T07).

### Relevant conductor public API (already exists, `roko-conductor/src/`)

| Symbol | Signature | File |
|---|---|---|
| `Conductor::from_config` | `fn(&ConductorConfig) -> Conductor` | `conductor.rs:204` |
| `Conductor::from_circuit_breaker_state` | `fn(CircuitBreakerState) -> Conductor` | `conductor.rs:254` |
| `Conductor::evaluate_full` | `fn(&[Engram], &Context) -> ConductorEvaluation` | `conductor.rs:311` |
| `Conductor::circuit_breaker` | `fn(&self) -> &CircuitBreaker` | `conductor.rs:248` |
| `Conductor::routing_bias` | `fn(&self) -> RoutingBias` | `conductor.rs:287` |
| `ConductorDecision` | `Continue` / `Restart` / `Fail{watcher,reason}` | `roko-core` |
| `CircuitBreakerState` | serde-serializable breaker snapshot | `circuit_breaker.rs` |

---

## Task breakdown

### E08-T01 — RuntimeEvent→Engram adapter
- **tier:** focused
- **files:** `crates/roko-cli/src/runner/conductor_adapter.rs` (new), `crates/roko-cli/src/runner/mod.rs`
- **depends_on:** [] (E01)
- Map `RunnerEvent` and `AgentEvent` variants to `Engram`s with the `Kind` + tags each watcher
  reads (`plan_id`, `task`, `model`, `provider`, `severity`, gate pass/fail, cost/budget metrics,
  ghost-turn payload). Return `Option<Engram>` so uninteresting events are dropped.
- **acceptance:** pure `fn(&RunnerEvent) -> Option<Engram>` + `fn(&AgentEvent) -> Option<Engram>`;
  unit tests assert a synthetic ghost-turn / gate-fail stream produces engrams that make
  `Conductor::default().evaluate(...)` return a non-`Continue` decision.

### E08-T02 — conductor_ring + EventSink decorator
- **tier:** focused
- **files:** `crates/roko-cli/src/runner/conductor_adapter.rs`, `crates/roko-cli/src/runtime_feedback/mod.rs`
- **depends_on:** [E08-T01]
- Add a bounded `ConductorRing` (`Arc<Mutex<VecDeque<Engram>>>`, cap ~512, drop-oldest) and a
  `ConductorRingSink` implementing `FeedbackSink` that runs each event through the T01 adapter and
  pushes into the ring. Register it on the `FeedbackFacade` when a conductor is present.
- **acceptance:** `FeedbackFacade` gains a conductor sink; a fanned-out event stream leaves the
  ring non-empty and bounded at cap; sink is best-effort (never blocks the facade).

### E08-T03 — Conductor::from_config wiring
- **tier:** integrative
- **files:** `crates/roko-cli/src/runner/event_loop.rs`, `crates/roko-cli/src/runner/types.rs`
- **depends_on:** [E08-T02]
- Construct `Conductor::from_config(&roko_config.conductor)` (falling back to
  `from_circuit_breaker_state` on resume — see T06) at run setup; store `Arc<Conductor>` + the
  shared `ConductorRing` on the run context; give `ConductorRingSink` the ring handle. This is the
  call that revives all `[conductor.watchers.*]` config.
- **acceptance:** `rg 'roko_conductor|Conductor::from_config' crates/roko-cli/src/runner` > 0;
  a `[conductor.watchers.ghost_turn]` threshold set in `roko.toml` measurably changes when the
  ghost-turn watcher fires in an integration run.

### E08-T04 — Supervision select! branch
- **tier:** architectural
- **files:** `crates/roko-cli/src/runner/event_loop.rs`
- **depends_on:** [E08-T03]
- Add a `conductor_tick` interval branch to the main `tokio::select!` (alongside Branch 3
  `tick_interval` @ `event_loop.rs:1743`; keep it cancel-safe). Each tick: snapshot the ring,
  call `conductor.evaluate_full(&snapshot, &Context::now())`, then enforce:
  `Fail` → trigger the run `CancellationToken` + emit a terminal `RunnerEvent` with the watcher
  reason; `Restart` → re-queue the offending task via the executor (mirror the retry/re-dispatch
  path); `Continue` → apply sub-critical `CognitiveSignal`s (cooldown/escalate) best-effort.
- **acceptance:** an integration run fed a ghost-turn / compile-fail-loop plan is aborted or
  restarted by the conductor *before* the plan wall-clock timeout; a test asserts the terminal
  event carries the watcher name (not `plan-timeout`).

### E08-T05 — Real conductor_load (kill the 0.0)
- **tier:** focused
- **files:** `crates/roko-cli/src/runner/event_loop.rs`, `crates/roko-cli/src/runtime_feedback/routing.rs`
- **depends_on:** [E08-T03]
- Replace `conductor_load: 0.0` (`event_loop.rs:4258`, `routing.rs:132`) with a real value derived
  from `conductor.routing_bias().prefer_cheaper`, circuit-breaker pressure, and ready-queue depth
  (reuse the `routing_load_pressure` logic proven in `orchestrate.rs:3016`).
- **acceptance:** `rg 'conductor_load: 0\.0' crates/roko-cli/src/runner crates/roko-cli/src/runtime_feedback`
  → 0 matches; under synthetic load the routing context reports `conductor_load > 0.0`.

### E08-T06 — Snapshot / restore circuit-breaker state
- **tier:** integrative
- **files:** `crates/roko-cli/src/runner/snapshot_writer.rs`, `crates/roko-cli/src/runner/resume.rs`, `crates/roko-cli/src/runner/event_loop.rs`
- **depends_on:** [E08-T03]
- Persist `conductor.circuit_breaker().state()` into the executor snapshot on the periodic flush
  (Branch 4, `event_loop.rs:1825`); on `--resume`, rebuild via
  `Conductor::from_circuit_breaker_state(state)` so failure budgets survive restarts.
- **acceptance:** a resumed run re-hydrates a tripped breaker (a plan tripped pre-restart stays
  tripped post-resume); round-trip serde test on `CircuitBreakerState`.

### E08-T07 — Close the loop into routing + docs/smoke
- **tier:** focused
- **files:** `crates/roko-cli/src/runner/event_loop.rs`, `crates/roko-cli/src/dispatch/model_routing.rs`, `crates/roko-cli/CLAUDE.md`
- **depends_on:** [E08-T04, E08-T05]
- Feed `conductor.routing_bias()` (deprioritize models / prefer_cheaper) into the runner-v2
  cascade-routing decision (mirror `orchestrate.rs:15436`); document the now-live
  `[conductor.watchers.*]` config surface; add a smoke assertion to an existing runner-v2
  integration test.
- **acceptance:** routing bias observably shifts model selection after repeated failures on a
  model in an integration run; `[conductor]` config section documented as live.

---

## First three tasks as executable native TOML

```toml
[meta]
plan = "E08-conductor-supervision"
total = 7
done = 0
status = "ready"
max_parallel = 1

# ═══════════════════════════════════════════════════════════════════════════════
# E08-T01: RuntimeEvent -> Engram adapter (bridge runner-v2 events to watchers)
# ═══════════════════════════════════════════════════════════════════════════════

[[task]]
id = "E08-T01"
title = "Add RunnerEvent/AgentEvent -> Engram adapter for conductor watchers"
status = "ready"
tier = "focused"
model_hint = "claude-sonnet-4-6"
max_loc = 220
files = [
    "crates/roko-cli/src/runner/conductor_adapter.rs",
    "crates/roko-cli/src/runner/mod.rs",
]
role = "implementer"
depends_on = []
acceptance = "Pure fns map RunnerEvent/AgentEvent to Option<Engram>; a synthetic ghost-turn + gate-fail stream, once adapted, drives Conductor::default().evaluate() to a non-Continue decision."

[task.context]
read_files = [
    { path = "crates/roko-cli/src/runner/types.rs", lines = "561-765", why = "RunnerEvent enum variants + payload fields to map (plan_id, task, model, provider, gate pass/fail, cost)" },
    { path = "crates/roko-cli/src/runner/types.rs", lines = "309-321", why = "AgentEvent (AgentRuntimeEvent) variants: TurnCompleted carries cost; ToolCall/MessageDelta for activity" },
    { path = "crates/roko-conductor/src/conductor.rs", lines = "455-476", why = "extract_provider / extract_plan_id — the exact tag keys and Kinds watchers read from the stream" },
    { path = "crates/roko-conductor/src/conductor.rs", lines = "691-740", why = "Test fixtures show the ghost-turn Engram body schema + PlanPhase/GateVerdict/Metric tag layout watchers expect" },
]
symbols = [
    "RunnerEvent — enum, runner/types.rs:564 (GateCompleted, TaskAttemptCompleted, PlanStarted, AgentCompleted ...)",
    "AgentEvent — alias for roko_agent::AgentRuntimeEvent, runner/types.rs:41",
    "roko_core::Engram — target signal type; ::builder(Kind).body(Body).tag(k,v).build()",
    "roko_core::Kind — PlanPhase | GateVerdict | Metric | AgentOutput | Custom(String)",
    "roko_conductor::PLAN_ID_TAG — the 'plan_id' tag key watchers key on",
]
anti_patterns = [
    "Do NOT add roko-conductor as a dep just for this file — map to roko_core::Engram only; keep the adapter dependency-light.",
    "Do NOT emit an Engram for every event — return Option and drop events no watcher consumes (avoids ring churn).",
    "Do NOT invent new tag keys — reuse exactly plan_id/task/model/provider/severity as the watchers read them.",
    "Do NOT mutate runner state or perform IO in the adapter — it must stay a pure mapping function.",
]

[[task.verify]]
phase = "structural"
command = "test -f crates/roko-cli/src/runner/conductor_adapter.rs"
fail_msg = "conductor_adapter.rs must exist"

[[task.verify]]
phase = "structural"
command = "rg -q 'fn .*runner_event.*Engram|fn .*to_engram' crates/roko-cli/src/runner/conductor_adapter.rs"
fail_msg = "adapter must expose a RunnerEvent -> Engram mapping fn"

[[task.verify]]
phase = "compile"
command = "cargo build -p roko-cli"
fail_msg = "crate must compile with the new adapter module"

[[task.verify]]
phase = "test"
command = "cargo test -p roko-cli conductor_adapter"
fail_msg = "adapter unit tests (ghost-turn/gate-fail stream -> non-Continue decision) must pass"

# ═══════════════════════════════════════════════════════════════════════════════
# E08-T02: conductor_ring + FeedbackSink decorator
# ═══════════════════════════════════════════════════════════════════════════════

[[task]]
id = "E08-T02"
title = "Add bounded conductor_ring fed by a ConductorRingSink FeedbackSink decorator"
status = "ready"
tier = "focused"
model_hint = "claude-sonnet-4-6"
max_loc = 200
files = [
    "crates/roko-cli/src/runner/conductor_adapter.rs",
    "crates/roko-cli/src/runtime_feedback/mod.rs",
]
role = "implementer"
depends_on = ["E08-T01"]
acceptance = "A ConductorRingSink implementing FeedbackSink converts events via the T01 adapter and pushes into a bounded ring; fanning many events leaves the ring non-empty and capped; a failing push never aborts the facade fan-out."

[task.context]
read_files = [
    { path = "crates/roko-cli/src/runtime_feedback/mod.rs", lines = "121-210", why = "FeedbackSink trait (best-effort, per-sink error containment) + FeedbackFacade::with_sink registration pattern to mirror" },
    { path = "crates/roko-cli/src/runtime_feedback/mod.rs", lines = "48-120", why = "FeedbackEvent vocabulary the sink receives — what to convert via the T01 adapter" },
    { path = "crates/roko-cli/src/runner/conductor_adapter.rs", lines = "1-40", why = "The T01 adapter fns this sink calls before pushing into the ring" },
]
symbols = [
    "FeedbackSink — trait, runtime_feedback/mod.rs:130 (on_event async, wants(&event)->bool)",
    "FeedbackFacade::with_sink — runtime_feedback/mod.rs:191 builder registration",
    "FeedbackEvent — runtime_feedback/mod.rs:48 provider-neutral event enum",
    "std::collections::VecDeque + Arc<Mutex<..>> — ConductorRing backing store",
]
anti_patterns = [
    "Do NOT let the ring grow unbounded — enforce a cap (~512) with drop-oldest (pop_front) semantics.",
    "Do NOT block or .await under the ring Mutex — lock, push/trim, drop the guard immediately.",
    "Do NOT panic on a poisoned lock or full ring — the sink is best-effort per the FeedbackSink contract.",
    "Do NOT register the sink unconditionally — only when a conductor is present (added in E08-T03).",
]

[[task.verify]]
phase = "structural"
command = "rg -q 'ConductorRing|conductor_ring' crates/roko-cli/src/runner/conductor_adapter.rs"
fail_msg = "a bounded conductor ring type must exist"

[[task.verify]]
phase = "structural"
command = "rg -q 'impl FeedbackSink for .*Ring|ConductorRingSink' crates/roko-cli/src/runtime_feedback/mod.rs crates/roko-cli/src/runner/conductor_adapter.rs"
fail_msg = "a FeedbackSink decorator feeding the ring must exist"

[[task.verify]]
phase = "compile"
command = "cargo build -p roko-cli"
fail_msg = "crate must compile with the ring + sink"

[[task.verify]]
phase = "test"
command = "cargo test -p roko-cli conductor_ring"
fail_msg = "ring bound + best-effort push tests must pass"

# ═══════════════════════════════════════════════════════════════════════════════
# E08-T03: Conductor::from_config wiring into the runner-v2 event loop
# ═══════════════════════════════════════════════════════════════════════════════

[[task]]
id = "E08-T03"
title = "Construct Conductor::from_config and thread it + ring into the event loop"
status = "ready"
tier = "integrative"
model_hint = "claude-opus-4-6"
max_loc = 260
files = [
    "crates/roko-cli/src/runner/event_loop.rs",
    "crates/roko-cli/src/runner/types.rs",
]
role = "implementer"
depends_on = ["E08-T02"]
acceptance = "Conductor::from_config(&roko_config.conductor) is built once at run setup, stored as Arc on the run context, and shares the ConductorRing with the sink; a [conductor.watchers.ghost_turn] threshold in roko.toml is now honored at runtime (config no longer dead)."

[task.context]
read_files = [
    { path = "crates/roko-cli/src/runner/types.rs", lines = "1258-1460", why = "RunConfig fields + from_roko_config builder — where roko_config.conductor is available and facades are constructed" },
    { path = "crates/roko-cli/src/runner/event_loop.rs", lines = "744-790", why = "Run setup: facades/subscribers spawned here; the conductor + ring must be built alongside and shared into the sink" },
    { path = "crates/roko-conductor/src/conductor.rs", lines = "194-256", why = "Conductor::from_config(&ConductorConfig) and from_circuit_breaker_state — the constructors to call" },
    { path = "crates/roko-cli/src/orchestrate.rs", lines = "15416-15440", why = "Dead reference: how orchestrate built routing context from conductor.routing_bias()/signals — the pattern to port" },
]
symbols = [
    "Conductor::from_config — roko_conductor, conductor.rs:204 (materializes [conductor.watchers.*])",
    "RokoConfig.conductor — roko_core::config::schema::ConductorConfig, already parsed",
    "RunConfig — runner/types.rs:1261; add Option<Arc<Conductor>> + ConductorRing handle",
    "FeedbackFacade::with_sink — register the ConductorRingSink only when conductor present",
]
anti_patterns = [
    "Do NOT default the conductor on when config is absent in a way that breaks smoke tests — gate on Option, preserve None-means-off.",
    "Do NOT rebuild the conductor per tick — construct once at run start, share via Arc.",
    "Do NOT bypass from_config with Conductor::new() — that would keep [conductor.watchers.*] dead (the whole point is to revive config).",
    "Do NOT hold the conductor across an .await while also holding the ring lock.",
]

[[task.verify]]
phase = "structural"
command = "rg -q 'roko_conductor|Conductor::from_config' crates/roko-cli/src/runner/event_loop.rs"
fail_msg = "the event loop must construct the conductor from config (was zero conductor imports)"

[[task.verify]]
phase = "structural"
command = "rg -c 'roko_conductor' crates/roko-cli/src/runner"
fail_msg = "runner tree must now reference roko_conductor (>0)"

[[task.verify]]
phase = "compile"
command = "cargo build -p roko-cli"
fail_msg = "crate must compile with the conductor wired into RunConfig/event loop"

[[task.verify]]
phase = "test"
command = "cargo test -p roko-cli runner::"
fail_msg = "existing runner tests must still pass with the conductor threaded through"
```

---

## Verification performed for this epic

- `rg 'roko_conductor|roko-conductor' crates/roko-cli/src/runner` → **0 matches** (confirms zero
  conductor imports in runner-v2).
- `rg 'conductor_load' …` → hardcoded `0.0` at `runner/event_loop.rs:4258` and
  `runtime_feedback/routing.rs:132`; real value only in dead `orchestrate.rs:3016`.
- Read `roko-conductor/src/lib.rs` + `conductor.rs` → confirmed `from_config`, `evaluate_full`,
  `from_circuit_breaker_state`, `circuit_breaker()`, `routing_bias()` public API and watcher
  `&[Engram]` contract.
- Read `runner/types.rs` (`RunnerEvent`/`AgentEvent`, `RunConfig`, `FeedbackFacade` comment) and
  `runner/event_loop.rs` select! branches (1–6) → confirmed the ring/sink/tick insertion points.
- `runtime_feedback/mod.rs` → `FeedbackSink` trait + `FeedbackFacade::with_sink` is the
  ready-made decoration point; no conductor sink exists today.

## CTRL-08 ownership reconciliation

The seven core conductor tasks remain distinct. T08 is an acceptance roll-up for
E47-T08's `React`-based `DiskPressureWatcher`, intervention Engrams, and canonical
`[resources]` thresholds; the retired `Watcher`/`WatcherOutput` and
`[conductor.watchers.disk_space]` shapes are not parallel contracts. T09 is the
distinct `React::decide` consumer of E47-T09's exact `Kind::Metric` Engrams tagged
`name=worktree_count` and `value=WorktreeManager::active_count`. E47 emits the metric
on both runner paths; E08 owns only the configurable warning watcher and must not
define another disk-accounting, worktree-count producer, or serialization mechanism. See
[`17-OPERATIONAL-OWNERSHIP.md`](../17-OPERATIONAL-OWNERSHIP.md).
