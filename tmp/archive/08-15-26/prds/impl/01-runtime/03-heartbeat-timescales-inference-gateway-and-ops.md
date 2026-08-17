# Heartbeat Timescales, Inference Gateway, And Runtime Ops

## Scope

Use this file for the PRD-02 material that sits below the extraction work but above product migration: multi-timescale loops, concurrent mechanisms, process supervision details, performance targets, and the inference gateway.

## Implementation checklist

- [ ] Implement the three timescales explicitly.
  - gamma loop for fast observe/analyze/gate/action;
  - theta loop for reflective consolidation and plan-state refresh;
  - delta loop for dream/consolidation/cleanup work.
- [ ] Define timescale configuration and persistence.
  - default intervals;
  - domain overrides;
  - jitter/backoff strategy;
  - persisted last-run timestamps if needed for restart recovery.
- [ ] Add explicit runtime tasks for the six concurrent cognitive mechanisms.
  - attention salience updates;
  - habituation mask refresh;
  - sleep-pressure accumulation;
  - event-driven wakeup triggers;
  - homeostasis metrics;
  - compensation/rollback behavior.
- [ ] Specify compensation and rollback semantics.
  - when simulation/validation/execution diverge;
  - what state rolls back;
  - what events are emitted;
  - what remains as durable audit trail.
- [ ] Define `CorticalState` implementation details if the runtime takes ownership.
  - lock-free vs snapshot-based access;
  - fixed-point encoding or other compact representation where PRD-02 requires it;
  - snapshot/export API for surfaces and learning code.
- [ ] Expand event-fabric detail.
  - typed payloads;
  - filtered subscription semantics;
  - backlog/ring buffer policy;
  - lagged subscriber behavior.
- [ ] Flesh out process supervision and actor-model details.
  - mailbox contract;
  - PID registry;
  - supervision strategy types;
  - kill sequence and graceful shutdown path.
- [ ] Implement the inference gateway as a real runtime-adjacent subsystem.
  - L3 exact-match cache;
  - L2 semantic cache;
  - L1 prefix/provider cache alignment;
  - intent-based routing;
  - translator pattern between provider response/tool formats;
  - cost/latency instrumentation on every gateway decision.
- [ ] Connect gateway behavior to current `roko-agent` reality.
  - unify translator work with existing backend/translate code;
  - avoid inventing a second provider abstraction that competes with current adapters.
- [ ] Add performance targets as tests or benchmarks where practical.
  - per-operation latency;
  - per-tier latency;
  - cache-hit improvement;
  - overhead as percent of full LLM dispatch.

## Additional gap-closure tasks

- [ ] Add an explicit runtime backlog item for heartbeat-step observability.
  - one event type or metric per pipeline step;
  - per-step duration recording;
  - failure attribution when a tick aborts before completion.
- [ ] Add a runtime task for homeostasis-policy persistence.
  - persisted target ranges;
  - violation counters;
  - recovery actions when an agent remains outside target ranges for N ticks.
- [ ] Add a runtime task for degraded-mode operation.
  - provider outage mode;
  - disk or substrate outage mode;
  - event-bus backlog saturation mode;
  - behavior when only T0 is permitted.
- [ ] Add a runtime task for extension-crash isolation.
  - one bad extension cannot wedge the full heartbeat loop;
  - extension timeout and quarantine policy;
  - surfaced diagnostics for disabled/quarantined extensions.
- [ ] Add an explicit task for backward-compatible `DecisionCycleRecord` evolution.
  - schema versioning;
  - readers for older persisted records;
  - forward-safe event consumers.

## Agent-ready task sequence

1. `RT-GAP-01` Heartbeat step instrumentation
   - Scope: emit one structured event/metric per heartbeat step and record per-step duration.
   - Touches: `crates/roko-runtime/src/heartbeat.rs`, `event_bus.rs`, runtime metrics types.
   - Deliverable: a step-level telemetry surface that lets later agents see where ticks spend time and where they fail.
   - Done when: one integration test can assert the full ordered step sequence for a non-T0 tick.

2. `RT-GAP-02` Homeostasis policy persistence
   - Scope: define persisted target ranges, deviation counters, and recovery policy storage.
   - Touches: `crates/roko-runtime/src/lifecycle.rs`, `metrics.rs`, `.roko` persistence wiring.
   - Deliverable: load/save behavior for homeostasis policy and drift counters.
   - Depends on: `RT-GAP-01`.
   - Done when: runtime restart preserves homeostasis state and emits recovery actions deterministically.

3. `RT-GAP-03` Degraded-mode execution model
   - Scope: specify and implement runtime behavior when providers, substrate, or the event bus are impaired.
   - Touches: `heartbeat.rs`, `process.rs`, possibly `orchestrate.rs` fallback wiring.
   - Deliverable: explicit degraded-mode states with guarded T0-only or reduced-capability behavior.
   - Depends on: `RT-GAP-01`.
   - Done when: tests prove the runtime can keep operating in at least one degraded mode without undefined behavior.

4. `RT-GAP-04` Extension crash isolation and quarantine
   - Scope: stop one bad extension from wedging the full loop.
   - Touches: extension-chain execution path, supervision logic, runtime diagnostics.
   - Deliverable: timeout/quarantine policy plus operator-visible diagnostics for disabled extensions.
   - Depends on: `RT-GAP-03`.
   - Done when: a faulting extension is isolated and the remaining chain still progresses.

5. `RT-GAP-05` `DecisionCycleRecord` schema evolution
   - Scope: version the record format and add backward-compatible readers.
   - Touches: record type definitions, persistence readers/writers, event consumers.
   - Deliverable: schema version field, compatibility readers, and one migration/regression fixture.
   - Depends on: `RT-GAP-01`.
   - Done when: old persisted records remain readable after adding the new telemetry fields.

## Concrete file touchpoints

- `crates/roko-runtime/src/heartbeat.rs`
- `crates/roko-runtime/src/heartbeat_attention.rs`
- `crates/roko-runtime/src/theta_consumer.rs`
- `crates/roko-runtime/src/delta_consumer.rs`
- `crates/roko-runtime/src/process.rs`
- `crates/roko-runtime/src/event_bus.rs`
- `crates/roko-agent/src/`
- `crates/roko-cli/src/orchestrate.rs`

## Verification checklist

- [ ] Gamma/theta/delta work can be observed independently in tests or runtime logs.
- [ ] Event-driven wakeups do not starve periodic loops.
- [ ] Supervisor restart/kill behavior is deterministic in at least one test.
- [ ] Gateway cache/routing decisions are logged with enough detail to debug misses and fallbacks.
- [ ] Translator behavior is covered by provider-format tests, not only by prose.

## Acceptance criteria

- Runtime timing behavior is explicit instead of hidden in one flat loop.
- Operational and supervision behavior is testable.
- The inference gateway is integrated with current agent infrastructure and measurable against latency/cost targets.
