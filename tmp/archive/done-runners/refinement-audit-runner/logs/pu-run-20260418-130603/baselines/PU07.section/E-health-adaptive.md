# E — Health + Adaptive Timeouts + Yerkes-Dodson + Process Supervision (Doc 07/06 + 10 + 12 + 13)

Parity of four Doc 07 chapters against the shipping conductor, runtime, and
orchestrator wiring:

- `docs/07-conductor/06-health-monitors.md` — four system-level health checks
  over a `SystemSnapshot` producing a `HealthStatus`.
- `docs/07-conductor/10-adaptive-timeouts-state-machine.md` — `PhaseKind`
  timeouts scaled by `ComplexityBand`, `PhaseTransition` audit records,
  layered provider timeouts, graceful shutdown + atomic checkpoints.
- `docs/07-conductor/12-yerkes-dodson-pressure.md` — 919 lines of
  arousal/performance theory, per-model `PressureBandit`, Yerkes-Dodson
  estimator, cognitive-load split, flow state detection. Almost entirely
  design-only narrative.
- `docs/07-conductor/13-process-supervision-wiring.md` — `ProcessSupervisor`
  in `roko-runtime` owning spawn, PID tracking, orphan cleanup, SIGTERM →
  SIGKILL escalation, and shutdown sequence.

Cross-cuts with A-architecture (`A.08 health monitor`, `A.09 phase timeout
matrix`) and with the F-status audit on wiring-vs-built drift (CLAUDE.md
says "ProcessSupervisor ... Wired — `PlanRunner` tracks + shuts down
agents"). This file drills into the actual call-site wiring, the
Yerkes-Dodson grep-negative surface, and the ProcessSupervisor vs
`roko-agent/src/process/*` dual stack.

Generated 2026-04-16.

---

## E.01 — `SystemSnapshot` struct matches doc shape modulo field renames (Doc 06 §"SystemSnapshot")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: `SystemSnapshot` carries `active_agents: usize`, `expected_agents: usize`, `last_agent_heartbeat_ms: Option<u64>`, `chain_connected: bool`, `chain_expected: bool`, `spec_hash_expected: Option<String>`, `spec_hash_actual: Option<String>`, `coverage_history: Vec<f64>`.
**Reality**: The struct ships at `roko-conductor/src/health.rs:92-114` with nine fields. Field-by-field: `active_agents: u32` (`:95`, not `usize`), `expected_agents: u32` (`:97`, not `usize`), `last_agent_heartbeat_ms: i64` (`:99`, not `Option<u64>` — `0 = never`), `chain_connected: bool` (`:101`), `chain_expected: bool` (`:103`), `spec_hash_at_start: String` (`:105`, not `spec_hash_expected: Option<String>`), `spec_hash_current: String` (`:107`, not `spec_hash_actual: Option<String>`), `coverage_history: Vec<f64>` (`:109`), plus two extra fields the doc omits: `now_ms: i64` (`:111`) and `heartbeat_timeout_ms: i64` (`:113`). Semantically the fields line up but three names and four types differ from the doc rendering.
**Fix sketch**: Update doc 06 lines 17-27 to use the shipping types (`u32`, `i64` + sentinel zero, `String` with `is_empty()` sentinel) and rename `spec_hash_expected` → `spec_hash_at_start` and `spec_hash_actual` → `spec_hash_current`. Add the two missing fields `now_ms` and `heartbeat_timeout_ms` to the struct block.

---

## E.02 — `HealthStatus` enum matches doc (Doc 06 §"HealthStatus")

**Status**: DONE
**Severity**: —
**Doc claim**: `HealthStatus { Healthy, Degraded, Critical }` with worst-status-wins aggregation.
**Reality**: Exactly three variants at `roko-conductor/src/health.rs:26-33` with explicit discriminants `Healthy = 0`, `Degraded = 1`, `Critical = 2`. `#[derive(...PartialOrd, Ord...)]` supports `.max()` aggregation — `overall_status()` at `:188-194` calls `.iter().map(|c| c.status).max().unwrap_or(HealthStatus::Healthy)`. Test `health_status_ordering` at `:545-548` pins `Healthy < Degraded < Critical`. The `overall_status_picks_worst` test at `:525-540` confirms behavior end-to-end. Doc is accurate.

---

## E.03 — Four built-in checks ship, but second is named `golem_status` (Doc 06 §"The Four Checks")

**Status**: PARTIAL
**Severity**: MEDIUM
**Doc claim**: Four checks — `terminal_liveness`, `agent_status`, `spec_drift`, `coverage_trend`.
**Reality**: Four checks registered in `HealthMonitor::new()` at `roko-conductor/src/health.rs:152-171`: `terminal_liveness` (`:155`), `golem_status` (`:159`, **not** `agent_status`), `spec_drift` (`:163`), `coverage_trend` (`:167`). Second check is `check_golem_status()` at `:258-270` and inspects `chain_connected` / `chain_expected` — it is chain-status, not agent-status. The doc's "Agent Status" sub-section §2 describes comparing `active_agents` vs `expected_agents`, which is actually folded into `check_terminal_liveness()` at `:201-255` (see lines `:218-227` — "active agents match expected"), not a separate check. So there are four checks but only three of them map cleanly onto the doc's four labels: `terminal_liveness` covers both liveness + agent_status; `golem_status` covers chain; the doc's "agent_status" is not a standalone check. This post-rename drift is flagged in `tmp/docs-parity/07/A-architecture.md` A.08.
**Fix sketch**: Rename `check_golem_status` → `check_chain_status` at `health.rs:159, 258` and update doc 06 §"The Four Checks" to distinguish chain-status (check 2) from the agent-count logic now living inside `terminal_liveness`. The "golem" naming is a stale pre-dissolution holdover (see `tmp/docs-parity/06/F-status-frontier.md` F.05).

---

## E.04 — `HealthMonitor::check()` signature matches doc modulo method name (Doc 06 §"HealthMonitor API")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: `HealthMonitor::check(&self, snapshot: &SystemSnapshot) -> HealthStatus` running four checks and taking the worst via `.max()`.
**Reality**: The entrypoint is named `overall_status()` at `roko-conductor/src/health.rs:188-194`, not `check()`. `check_all()` at `:182-184` returns `Vec<HealthCheckResult>` per-check, and `overall_status()` calls `check_all().iter().map(|c| c.status).max()` — the "worst wins" aggregation is correct. `check_count()` at `:176-178` exposes the registered-check count. The doc's pseudocode sketch is directionally accurate but the shipping API splits the per-check and aggregate paths into two methods instead of one.
**Fix sketch**: Update doc 06 lines 170-183 to show the actual two-method API: `check_all(&self, snapshot) -> Vec<HealthCheckResult>` plus `overall_status(&self, snapshot) -> HealthStatus`. The snippet as written implies a single `check()` method that does not exist.

---

## E.05 — `HealthMonitor` is BUILT BUT NOT WIRED into the orchestrator (Doc 06 §"Snapshot Collection" + A.08 follow-up)

**Status**: NOT DONE (wiring claim; the crate-level implementation is complete)
**Severity**: HIGH
**Doc claim**: Doc 06 §"Snapshot Collection" says "The orchestrator constructs the snapshot periodically (every 10 seconds in the default configuration) and passes it to the health monitor"; Doc 06 opening badge reads "**Implementation**: Built". Doc 13 §"Orchestrator Architecture" implies health monitoring is part of the plan runner loop.
**Reality**: No caller of `HealthMonitor` exists outside the health module's own tests. `Grep 'HealthMonitor'` across the tree returns matches only in `crates/roko-conductor/src/health.rs` (struct + tests), `docs/07-conductor/06-health-monitors.md`, and an unrelated `PluginHealthMonitor` discussion in `docs/18-tools/14-plugin-sdk.md`. `Grep 'HealthMonitor|SystemSnapshot|check_terminal_liveness|check_golem_status|check_spec_drift|check_coverage_trend'` on `crates/roko-cli/` returns zero matches. The orchestrator `PlanRunner` at `crates/roko-cli/src/orchestrate.rs:2135-2172` never constructs a `HealthMonitor`, never populates a `SystemSnapshot`, and never runs the periodic 10-second check loop. The A.08 followup in `tmp/docs-parity/07/A-architecture.md` reached the same conclusion: "the health monitor is built but not consumed by the orchestrator runtime, only tested inside the crate."
**Fix sketch**: Either (a) wire `HealthMonitor::overall_status()` into the `PlanRunner` main loop at `orchestrate.rs:~4780` (the place where `tokio::select!` drives phase transitions) alongside `self.conductor.check_all(&signals)` at `:1474`, populating `SystemSnapshot { active_agents: supervisor.count(), expected_agents: executor.active_plans.len(), coverage_history: gate_results.coverage_pct_history, ... }` on a 10 s interval, OR (b) strike "the orchestrator constructs the snapshot periodically" from Doc 06 line 230 and replace "**Implementation**: Built" with "**Implementation**: Built (not yet wired into the plan runner)". Current state violates WIRE-don't-build rule.

---

## E.06 — `check_coverage_trend` threshold differs from doc prose (Doc 06 §"Coverage Trend")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 06 line 157-158: "If the slope is negative and the recent average is below the earlier average by more than a threshold (e.g., 2 percentage points), the status is Degraded." Doc 06 line 141-142 says Coverage Trend maps only to Healthy or Degraded, "Critical" is "(Not used — coverage decline is always Degraded.)".
**Reality**: The shipping thresholds differ. `check_coverage_trend()` at `roko-conductor/src/health.rs:292-327` examines only the last **two** samples (`recent` = `history[len-1]`, `previous` = `history[len-2]`, `delta = recent - previous`) — it is not a slope over a window, it is a one-step delta. The band thresholds are `-5.0` for **Critical** (`:308`), `-1.0` for **Degraded** (`:314`), everything else Healthy. Unit test `coverage_drop_is_critical` at `:465-473` pins `80.0 → 72.0 = Critical` (doc 06 says this should never happen since Critical is "not used"). Unit test `coverage_slight_decline_is_degraded` at `:476-485` pins `80.0 → 77.5 = Degraded` (-2.5pp triggers the -1.0 threshold, not the doc-stated 2.0pp threshold). The code diverges on two points: (1) Critical is in fact emitted for sharp drops, contradicting doc 06 line 142; (2) the Degraded threshold is 1.0pp not 2.0pp; (3) the "slope over window" framing in the doc is not implemented — only last-pair delta.
**Fix sketch**: Update doc 06 lines 130-158 to reflect the actual three-band logic: Critical when `delta < -5.0`, Degraded when `delta < -1.0`, Healthy otherwise. Drop the "slope regression" language and the `Critical: Not used` line. Alternatively, re-implement with a true windowed regression if that was the intent.

---

## E.07 — VSM System 3* mapping is doc-only (Doc 06 §"VSM Mapping")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 06 §"VSM Mapping" lines 237-257 maps `HealthMonitor` → Beer's VSM System 3* (audit channel, independent check of System 3's model of reality).
**Reality**: `Grep 'VSM|System 3|Beer'` on `crates/roko-conductor/` returns zero matches. The mapping exists only as doc prose — there are no code comments, module docs, or type-level tags linking `HealthMonitor` to VSM. This is acceptable narrative framing (VSM terminology is not load-bearing), but the doc implies a deeper structural mapping than the code provides. Treating as "design-only narrative" rather than a missing feature.
**Fix sketch**: Either add a one-line module doc to `health.rs:1-18` referencing the VSM framing, or annotate Doc 06 §"VSM Mapping" as "conceptual framing, not reflected in code." Prefer the former since Doc 04 and Doc 07/08 both use VSM.

---

## E.08 — Phase timeout matrix is fully implemented (Doc 10 §"Phase Timeouts")

**Status**: DONE
**Severity**: —
**Doc claim**: `phase_timeout(phase: PlanPhase, complexity: Complexity) -> Duration` returning the 3-band matrix for `Implementing` (Complex 600s / Standard 300s / Fast 120s) plus fixed `Gating 300s`, `Reviewing 300s`, `Merging 60s`.
**Reality**: The function lives at `roko-conductor/src/state_machine.rs:37-55` as `pub const fn phase_timeout(phase: PhaseKind, complexity: TaskComplexityBand) -> Option<u64>`. Types differ from doc (`PhaseKind` not `PlanPhase`, `TaskComplexityBand` not `Complexity`, `Option<u64>` seconds not `Duration`), but the matrix is complete. Shipping constants at `:13-31`: `TIMEOUT_IMPLEMENTING_COMPLEX=600`, `TIMEOUT_IMPLEMENTING_STANDARD=300`, `TIMEOUT_IMPLEMENTING_SIMPLE=120` (note: doc calls the tier "Fast", code maps `TaskComplexityBand::Fast` to `TIMEOUT_IMPLEMENTING_SIMPLE` at `:41`), `TIMEOUT_GATING=300`, `TIMEOUT_REVIEWING=300`, `TIMEOUT_MERGING=60`. Bonus phases: `TIMEOUT_VERIFYING=300` (`:27`), `TIMEOUT_ENRICHING=120` (`:25`), `TIMEOUT_AUTO_FIXING=300` (`:29`), `TIMEOUT_DOC_REVISION=120` (`:31`). Terminal phases (`Queued`, `Complete`, `Failed`, `Skipped`, `Done`, `RegeneratingVerify`) return `None` (`:53`). Tests `implementing_timeout_varies_by_complexity` (`:115-129`), `non_implementing_phases_ignore_complexity` (`:131-142`), `all_active_phases_have_timeouts` (`:168-186`). Doc's three-band table is a subset; Doc 10 line 92-96 could be extended to list the eight total timed phases.

---

## E.09 — `PhaseTransition` audit struct exists but is not emitted as a typed record (Doc 10 §"PhaseTransition Records")

**Status**: PARTIAL
**Severity**: MEDIUM
**Doc claim**: `PhaseTransition { plan_id, from: PlanPhase, to: PlanPhase, timestamp: String (ISO 8601), reason: String }` with "every phase transition produces an audit record." Doc claims the audit trail "enables post-mortem analysis, performance optimization, anomaly detection, learning system input."
**Reality**: The struct ships at `roko-conductor/src/state_machine.rs:61-73`: `plan_id: String`, `from: PhaseKind`, `to: PhaseKind`, `at_ms: i64`, `reason: Option<String>`. Shape differs from doc (`PhaseKind` not `PlanPhase`, `at_ms: i64` not `timestamp: String`, `reason: Option<String>` not `String`), but the four fields are present. Helper methods `with_reason()` at `:89-92`, `elapsed_ms()` at `:96-99`, `elapsed_secs()` at `:103-108`. BUT: `Grep 'PhaseTransition::new'` returns matches only inside the crate's own tests (`state_machine.rs:190, 197, 205, 212`). The orchestrator's phase-change path at `crates/roko-cli/src/orchestrate.rs:5682-5707, 9424-9430` emits the transition through `EventKind::PhaseTransition` (an `EventKind` variant from `roko-fs`, not the `PhaseTransition` struct) into `event_log.append(...)` as a raw `serde_json::Value`. The typed `PhaseTransition` record is never instantiated at runtime, so none of the four audit-trail downstreams (post-mortem, optimization, anomaly, learning) actually consume it.
**Fix sketch**: Replace the ad-hoc `serde_json::json!({...})` payload at `orchestrate.rs:5682-5684, 5704-5707, 9424-9430` with `state_machine::PhaseTransition::new(plan_id, from_kind, to_kind, now_ms).with_reason(...)` serialized to JSON. That wires the typed record into the event log and makes the four listed downstream consumers actually achievable. As-is, the doc overstates what ships.

---

## E.10 — Adaptive P95-based timeout computation is built but not applied (Doc 10 §"Adaptive Timeout Computation")

**Status**: PARTIAL
**Severity**: MEDIUM
**Doc claim**: Doc 10 §"P95-Based Adaptive Timeout" shows `LatencyStats::adaptive_timeout_ms()` returning `2 × p95` clamped to `[5s, 300s]`. Doc 10 §"Cold Start Behavior" says the adaptive timeout takes over after 10 observations. Doc 10 §"Per-Phase Adaptive Timeouts" notes "the infrastructure exists in the latency registry (`roko-learn/src/latency.rs`)".
**Reality**: `LatencyStats::adaptive_timeout_ms()` ships exactly as documented at `roko-learn/src/latency.rs:70-77`: `if observations < 10 { 120_000 } else { (p95 * 2.0).clamp(5_000, 300_000) }`. Unit tests at `:481-514` pin the cold-start and clamp behavior. BUT: `Grep 'adaptive_timeout_ms|\.adaptive_timeout'` of `crates/` returns **only** the `latency.rs` definition + its own tests. No downstream caller applies the computed timeout. The static `ProviderConfig { timeout_ms, ttft_timeout_ms, connect_timeout_ms }` at `crates/roko-core/src/config/schema.rs:981-1020` uses fixed defaults (`120_000`, `15_000`, `5_000`) set at `main.rs:3635-3636, 3872-3873` — the adaptive value from `LatencyStats` never flows into `ProviderConfig.timeout_ms`. The doc's "Per-Phase Adaptive Timeouts" section is aspirational; Doc 10 line 200-202 correctly says "infrastructure exists" but the surrounding prose reads as if adaptive timeouts are active, which they are not.
**Fix sketch**: Either (a) in the dispatcher at `crates/roko-agent/src/dispatcher/mod.rs`, look up `latency_registry.get(model, provider)?.adaptive_timeout_ms()` and override `ProviderConfig.timeout_ms` on each request, OR (b) downgrade Doc 10 §"Adaptive Timeout Computation" from implementation to "design — infrastructure exists, consumer not wired."

---

## E.11 — TTFT / connect / hard timeout layering ships as config (Doc 10 §"TTFT Timeout")

**Status**: DONE
**Severity**: —
**Doc claim**: `ProviderConfig { timeout_ms: Option<u64> (120s), ttft_timeout_ms: Option<u64> (15s), connect_timeout_ms: Option<u64> (5s) }` three-layer timeout stack.
**Reality**: `ProviderConfig` at `roko-core/src/config/schema.rs:981-1020` carries `timeout_ms: Option<u64>`, `ttft_timeout_ms: Option<u64>`, `connect_timeout_ms: Option<u64>` as three separate serde-defaulted fields. Defaults at `:1022-1032`: `default_provider_timeout_ms() -> Some(120_000)`, `default_provider_ttft_timeout_ms() -> Some(15_000)`, `default_provider_connect_timeout_ms() -> Some(5_000)`. Values flow through `effective_providers()` at `:452-496` to every registered provider. Doc 10 lines 212-217 match the shipping struct.

---

## E.12 — Graceful shutdown ships the four-phase sequence via tokio-select (Doc 10 §"Graceful Shutdown Sequence")

**Status**: DONE
**Severity**: —
**Doc claim**: Four-phase shutdown: (1) stop accepting new tasks, (2) drain with 30 s grace period, (2b) kill remaining agents if drain times out, (3) checkpoint, (4) flush logs.
**Reality**: The shutdown path is at `orchestrate.rs:5084-5148`. Phase 1 is implicit via `self.cancel.cancel()` at `:5104` propagating through `roko_runtime::cancel::CancelToken`; the run loop observes `is_cancelled()` at `:4749` ("shutdown requested; stopping new dispatches") and `:4841`. Phase 2 drain with `SHUTDOWN_DRAIN_GRACE_SECS` (30 s by convention) at `:5106-5110`: `tokio::time::timeout(Duration::from_secs(SHUTDOWN_DRAIN_GRACE_SECS), &mut run)`. Phase 2b force-kill at `:5137-5138`: `RunExit::SignalTimedOut => self.force_shutdown().await`, which calls `supervisor.kill_all()` at `:3754-3761`. Phase 3 checkpoint at `:5129, 5140`: `self.save_state_to(&snapshot_path)` invoking `save_snapshot_atomic` at `:646-658` (temp-file-then-rename — see E.13). Phase 4 flush at `:5134, 5145`: `self.flush_logs().await` at `:3766`. `wait_for_shutdown_signal()` at `:660-681` handles SIGINT + SIGTERM on Unix, `ctrl_c` on Windows. The four phases are present; the doc's prose matches the code flow.

---

## E.13 — Atomic checkpoint write uses temp-then-rename (Doc 10 §"Atomic Checkpoint Writes")

**Status**: DONE
**Severity**: —
**Doc claim**: `save_snapshot_atomic` writes to `path.with_extension("json.tmp")` then `std::fs::rename` for POSIX-atomic replace; kill-mid-write leaves previous snapshot intact.
**Reality**: `save_snapshot_atomic()` at `orchestrate.rs:646-658` implements exactly this pattern: `create_dir_all(parent)` at `:647-649`, `tmp_path = path.with_extension("json.tmp")` at `:651`, `std::fs::write(&tmp_path, &json)` at `:653-654`, `std::fs::rename(&tmp_path, path)` at `:655-656`. Unit tests at `:15042, 15049` confirm save + rename-failure paths. Doc's snippet at Doc 10 lines 287-293 is a direct transcription of the shipping code.

---

## E.14 — `ProcessSupervisor` ships in `roko-runtime::process` but does NOT drive agent spawn (Doc 13 §"ProcessSupervisor Architecture")

**Status**: PARTIAL
**Severity**: HIGH
**Doc claim**: Doc 13 §"ProcessSupervisor Architecture" lines 41-42: "The `ProcessSupervisor` lives in `bardo-runtime` and is wired into the plan execution pipeline through `PlanRunner`." Doc 13 ASCII diagram at lines 45-71 shows `PlanRunner → ProcessSupervisor → {Agent 1, Agent 2, Agent 3}` with PID registry tracking `4201, 4202, 4203, 4205, 4206, 4209, 4210`. Doc's §"Core Responsibilities" enumerates five guarantees: PID tracking, descendant discovery, lifecycle management, orphan prevention, attempt isolation. CLAUDE.md row: "ProcessSupervisor (lifecycle mgmt) ... Wired — `PlanRunner` tracks + shuts down agents".
**Reality**: `ProcessSupervisor` ships at `roko-runtime/src/process.rs:280-591` (crate renamed from `bardo-runtime` per F.05 — Doc 13 line 41 still uses the old name). `PlanRunner` at `roko-cli/src/orchestrate.rs:2135-2172` holds `supervisor: Arc<ProcessSupervisor>` at `:2172`, constructed at `:3254, 3373, 3496` with `ProcessSupervisor::new(cancel.clone())`. BUT: `supervisor.spawn(SpawnConfig)` is **never called** outside the supervisor's own tests. `Grep 'supervisor\.spawn'` on `crates/` returns matches only at `roko-runtime/src/process.rs:312, 603, 628, 651` (definition + unit tests). Agent subprocesses are actually spawned by `crates/roko-agent/src/exec.rs:178` and `crates/roko-agent/src/claude_cli_agent.rs:420`, each calling `register_spawned_pid(pid)` against the separate `roko-agent/src/process/registry.rs` global registry (disk-persisted at `.roko/runtime/agent-pids.json`). Two parallel PID-tracking systems exist: (1) `roko-runtime::ProcessSupervisor` holding `handles: Mutex<HashMap<ProcessId, ProcessHandle>>` never populated from agent dispatch; (2) `roko-agent::process::registry` with a static `OnceLock<Mutex<HashSet<u32>>>` actually populated by every agent spawn. The supervisor's `count()` at `:483-485` returns zero during real agent runs, which is why `orchestrate.rs:3945, 4157` derives `active_agents` from `supervisor.count().await` but the value is consistently wrong. The shutdown path `supervisor.shutdown_all()` at `:3721` finds zero handles because nothing was ever registered with the runtime supervisor.
**Fix sketch**: Pick one of: (A) wire the real agent spawn path through `ProcessSupervisor::spawn(SpawnConfig)` at `roko-agent/src/exec.rs:178` and `claude_cli_agent.rs:420`, letting the runtime supervisor own the `Child` handle instead of the static registry — this matches Doc 13 architecture; (B) delete `supervisor: Arc<ProcessSupervisor>` from `PlanRunner` and replace Doc 13 prose with the real `roko-agent/process/{group.rs, kill.rs, registry.rs}` trio that actually ships; (C) document the two-layer design (runtime supervisor for future use + agent registry for today's dispatch) and fix the `active_agents` accounting so it reads from `registered_pids()` not `supervisor.count()`. CLAUDE.md line 39 should be corrected to "ProcessSupervisor ... Built but not driving agent spawn — spawn is handled by `roko-agent/src/process/registry.rs`."

---

## E.15 — Attempt tracking / stale-exit race fix is NOT DONE (Doc 13 §"Attempt Tracking")

**Status**: NOT DONE
**Severity**: MEDIUM
**Doc claim**: Doc 13 §"Attempt Tracking" lines 374-399: every spawn gets a monotonically increasing `attempt_id`; exit events carry `attempt_id`; supervisor compares `current_attempt_id` vs `event_attempt_id` and discards stale events. This is the structural fix for Issue #6 (spawn races).
**Reality**: `Grep 'attempt_id|spawn_backoff'` on `crates/` returns matches only in `roko-mcp-github/src/main.rs` (exponential-backoff-delay helper, unrelated to supervision). The `ProcessEntry { pid, parent_pid, plan_id, task_id, attempt_id, spawned_at, status }` struct documented at Doc 13 lines 102-118 does not exist in the tree. The shipping `ProcessHandle` at `roko-runtime/src/process.rs:159-170` has `id: ProcessId` (a monotonic counter, but per-process not per-attempt), `label`, `child`, `os_pid`, `grace_period`, `cancel`, `spawn_config`, `started_at` — no `attempt_id`, no `parent_pid`, no `plan_id`/`task_id` association. The restart logic at `:510-542` (`restart_process`) calls `handle.shutdown()` then `self.spawn(config)` returning a new `ProcessId`; there is no attempt-id carried through exit events for the stale-race detection. Spawn backoff table at Doc 13 lines 404-411 (2 s, 4 s, 30 s, 60 s) is not implemented for the supervisor.
**Fix sketch**: Add `attempt_id: u64` + `plan_id: String` + `task_id: String` to `ProcessHandle` (`process.rs:159-170`) and to `ProcessOutcome` (`:147-156`); carry the attempt id through exit event emission; compare on receive. Either implement Doc 13 or strip §"Attempt Tracking" from the doc as future work.

---

## E.16 — Orphan reaper + cross-restart cleanup ships in `roko-agent`, not `roko-runtime` (Doc 13 §"Orphan Reaper")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 13 §"Orphan Reaper" lines 324-370: every 30 seconds the supervisor scans the PID registry, updates dead entries to `Exited`, kills alive-but-parent-complete entries as orphans, logs unregistered processes. Doc 13 §"Orphan Detection Heuristics" lists four signals (parent PID 1, task completed, plan aborted, no registry entry).
**Reality**: An orphan reaper ships but in the parallel `roko-agent` stack (see E.14). `crates/roko-agent/src/process/registry.rs:89-148` has `cleanup_orphaned_agents()` — reads `.roko/runtime/agent-pids.json` on startup, sends SIGTERM + SIGKILL to surviving PIDs, cleans descendants via `collect_descendants()`. `reap_orphaned_children()` at `:166-229` uses `parent PID == 1` detection (Doc 13 line 362 — "Parent PID is 1 (init)") as the orphan signal; macOS goes to `launchd`, Linux to PID 1 or subreaper (Doc 13 line 368-370). The `roko-runtime::ProcessSupervisor` has no equivalent background task; `reap_exited()` at `:451-480` is a one-shot non-blocking check, not a 30-second loop. Shipping behavior matches Doc 13's intent (cross-restart orphan cleanup exists) but lives in the wrong crate per the doc; the 30-second periodic scan is not implemented anywhere.
**Fix sketch**: Either move `cleanup_orphaned_agents`/`reap_orphaned_children` into `roko-runtime::process` as documented, or update Doc 13 §"Orphan Reaper" to point at `roko-agent/src/process/registry.rs:89-229`. Add a `tokio::time::interval(Duration::from_secs(30))` loop around `reap_exited` + `reap_orphaned_children` if the periodic scan is still desired.

---

## E.17 — SIGTERM → SIGKILL escalation ships with the documented grace profile (Doc 13 §"SIGTERM → SIGKILL Escalation")

**Status**: DONE
**Severity**: —
**Doc claim**: Two-phase kill: SIGTERM, 5 s grace, then SIGKILL. Grace period configurable per process type.
**Reality**: `kill_tree()` at `roko-agent/src/process/kill.rs:33-71` implements the escalation: (1) `drop(child.stdin.take())` sends EOF (`:36`), (2) wait `grace` for natural exit (`:39`), (3) `kill_process_group(child, libc::SIGTERM)` (`:49`), (4) 800 ms grace via `GRACE_SIGTERM_MS` (`:21, 53`), (5) `kill_process_group(child, libc::SIGKILL)` (`:62`). The runtime-side `ProcessHandle::shutdown()` at `roko-runtime/src/process.rs:244-251` also does graceful-then-force via `wait_for_graceful_exit()` (`:182-197`) → `force_kill()` (`:199-204`). Default grace at `SpawnConfig::default()` is `Duration::from_secs(5)` at `:139`. Doc 13 lines 246-259 per-process-type grace table (Agent CLI 5 s, cargo 2 s, gate 2 s, rustc 1 s) is **not** implemented — the supervisor applies a single `grace_period` from `SpawnConfig`, no look-up table. Test `kill_tree_escalates_to_sigkill` at `kill.rs:91-116` spawns `bash -c "trap '' TERM; sleep 30"` and asserts the process is dead — escalation works end-to-end.

---

## E.18 — `setsid` / `setpgid` process-group isolation ships but with a slight shape difference (Doc 13 §"setsid for Isolation")

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 13 lines 282-294 shows `pre_exec(|| setsid())` to create a new session + process group; group kill via `kill(-pgid, signal)` at lines 307-313.
**Reality**: `set_process_group()` at `roko-agent/src/process/group.rs:15-26` uses `pre_exec` with `libc::setpgid(0, 0)` — this creates a new **process group** (sharing the existing session), not a new session. Doc 13 uses `setsid` (creates session + group); code uses `setpgid(0, 0)` (group only). Semantically both achieve signal isolation for the documented use case (`kill(-pgid, signal)` works with either). `kill_process_group()` at `group.rs:87-118` sends `libc::kill(-(pid as i32), signal)` exactly as Doc 13 line 310 shows, then iterates descendants from `collect_descendants()` (`:38-65`, uses `pgrep -P`) and signals each individually — Doc 13 line 215 lists this as the macOS approach. Shipping tree discovery depth is capped at 8 (`:41, 43`). The `setpgid` vs `setsid` distinction is cosmetic for signaling; the doc snippet could be updated to reflect the code.
**Fix sketch**: Update Doc 13 lines 282-294 from `libc::setsid()` to `libc::setpgid(0, 0)`. Semantically equivalent for the group-kill flow.

---

## E.19 — Cross-platform process discovery uses `pgrep -P` on all Unix, not cgroups on Linux (Doc 13 §"Platform-Specific Discovery")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 13 lines 205-211 distinguishes Linux cgroups (strongest, kernel-tracked), Linux `/proc/{pid}/task/*/children` (fallback), macOS `pgrep -P` recursive (macOS primary), macOS `ps -o pid,ppid` (fallback).
**Reality**: Only the `pgrep -P` approach ships. `collect_descendants()` at `roko-agent/src/process/group.rs:38-65` shells out to `std::process::Command::new("pgrep").args(["-P", &parent.to_string()])` on every Unix target — no Linux-specific cgroup branch, no `/proc` fallback. The code compiles under `#[cfg(unix)]`. On Linux the less-reliable `pgrep -P` approach is used instead of cgroups, which means processes that escape their PGID via `setsid()` (Doc 13 line 316-319) can be missed. Doc overstates the shipped discovery richness.
**Fix sketch**: Either implement the Linux cgroups branch (`/sys/fs/cgroup/roko/agent-{plan_id}/`) as Doc 13 lines 466-504 describes, or shrink the Doc 13 §"Platform-Specific Discovery" table to a single row — "All Unix: `pgrep -P {pid}` recursive, depth-capped at 8."

---

## E.20 — Resource limits via cgroups are NOT DONE (Doc 13 §"Resource Limits")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 13 §"Resource Limits" lines 460-504 prescribe per-agent CPU limits via `cgroup/roko/agent-{plan_id}/cpu.max = "50000 100000"`, memory limits via `memory.max = "2147483648"`, disk I/O via `io.max`. macOS fallback via SIGSTOP/SIGCONT throttling.
**Reality**: `Grep 'cgroup|cpu\.max|memory\.max|SIGSTOP|SIGCONT'` across `crates/` returns **zero matches**. No cgroup integration anywhere in the tree. `ResourceAccount` at `roko-runtime/src/resource.rs:11-148` tracks **budget** accounting (tokens, USD cost, wall time) but is not an OS-level resource limiter — it just checks `any_exceeded()` after the fact and reports via `ResourceAccount::any_exceeded()` (`:86-88`). Doc 13 §"Resource Limits" is design-only narrative; the `ResourceAccount` budget surface ships, the kernel-level OOM/CPU/IO limits do not.
**Fix sketch**: Mark Doc 13 §"Resource Limits" as "Design — not implemented" at the section header; either keep the text as a forward plan or defer until a Linux deployment actually needs enforcement. The Rust `cgroups-rs` crate exists if this becomes necessary.

---

## E.21 — Budget-tier `ResourceAccount` helpers ship (Doc 13-adjacent; complements §"Resource Limits")

**Status**: DONE
**Severity**: —
**Doc claim**: Implicit in Doc 13 §"Resource Limits" framing — each plan has a budget by complexity tier (no explicit Doc 13 claim, but it is the in-process counterpart to cgroups).
**Reality**: `ResourceAccount` at `roko-runtime/src/resource.rs:11-148` ships the in-process budget side with four tier constructors: `trivial(label)` → 50 000 tokens / $0.50 / 5 min (`:129-132`), `simple(label)` → 200 000 / $2.00 / 15 min (`:134-137`), `standard(label)` → 500 000 / $5.00 / 30 min (`:139-142`), `complex(label)` → 2 000 000 / $20.00 / 60 min (`:144-147`). `token_utilisation()`, `cost_utilisation()`, `time_utilisation()` at `:92-114` expose 0.0-1.0 ratios used by the conductor's `context-window-pressure` and `cost-overrun` watchers. Not strictly part of Doc 13 but belongs to the process-supervision surface. Units match doc 12's `cost_budget_usd` design in `PressureConfig` (see E.26).

---

## E.22 — Yerkes-Dodson curve theory (Zones 1/2/3, collapse window) has NO implementation (Doc 12 §"The Inverted-U Curve" + §"Yerkes-Dodson in LLM Agent Systems")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 12 §"The Inverted-U Curve" lines 12-49 describes three-regime arousal-performance curve (under/optimal/over-arousal) with a 5-12 turn collapse window under excessive pressure. Doc 12 §"Yerkes-Dodson in LLM Agent Systems" extends this to autonomous agents.
**Reality**: No code anywhere implements a Yerkes-Dodson curve. `Grep 'Yerkes|YerkesDodson'` on `crates/` returns **zero matches**. `Arousal` appears only in the PAD-affect surface (`roko-core/src/affect.rs:10`, `operating_frequency.rs:97, 232`, `roko-neuro/src/context.rs:151`) as the "A" dimension of the pleasure-arousal-dominance vector — unrelated to the cooperation-pressure curve. This is one of the heaviest theoretical sections in the conductor docs; the whole chapter is labelled "Implementation: Built" at Doc 12 line 8, which materially overstates the tree.
**Fix sketch**: Change Doc 12's top-of-file badge from "**Implementation**: Built" to "**Implementation**: Design (narrative-only)". The 919-line chapter is valuable design prose but claims implementation for structures that do not exist.

---

## E.23 — `PressureDial` / pressure envelope is NOT implemented (Doc 12 §"Conductor Thresholds as Pressure Parameters")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 12 §"The Pressure Envelope" lines 110-132 describes a 5-axis envelope (iteration, cost, time, progress, output pressure) around the agent operating space, with a dial mapping threshold settings to Yerkes-Dodson curve positions.
**Reality**: `Grep 'PressureDial|PressureEnvelope|pressure_dial'` on `crates/` returns zero matches. The individual thresholds exist as constants — `MAX_PLAN_FAILURES=2` at `roko-conductor/src/circuit_breaker.rs:11`, `MAX_GHOST_TURNS=3` at `watchers/ghost_turn.rs:11`, `DEFAULT_MAX_ITERATIONS=25` at `roko-agent/src/tool_loop/max_iter.rs:7`, `MAX_CONTEXT_USAGE_RATIO=0.80` at `watchers/context_window_pressure.rs:10` — but there is no struct that combines them into a single pressure envelope, no `fn pressure_index(...)` as Doc 12 lines 483-503 documents, no mapping from threshold tuple to a Yerkes-Dodson x-axis position. The thresholds are isolated.
**Fix sketch**: Either implement a `PressureEnvelope` struct that reads all five threshold sources and emits a scalar pressure index (Doc 12 line 483-503 provides the formula), or tag the §"Pressure Envelope" section in Doc 12 as "conceptual framing — the five thresholds ship as standalone watcher constants, no aggregated envelope struct exists."

---

## E.24 — `ModelPressureProfile` + per-model Yerkes-Dodson curves is NOT DONE (Doc 12 §"Pressure Calibration Per Agent Type")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 12 lines 469-480 specifies `ModelPressureProfile { model, optimal_pressure, collapse_threshold, observations, history: Vec<(f64, f64)> }`. Table at lines 441-445 maps Opus/Sonnet/Haiku to different peak locations and collapse thresholds.
**Reality**: `Grep 'ModelPressureProfile'` on `crates/` returns **zero matches** (only Doc 12 itself). The struct, its curve-fitting methods (Doc 12 lines 577-603, parametric logistic), the binned `YerkesDodsonEstimator` (lines 614-669), the CUSUM regime-shift detector (lines 689-696), and the Bayesian confidence bounds (lines 698-712) are all grep-negative. No `roko-learn` module ships this. Entirely design narrative.
**Fix sketch**: Tag Doc 12 §"Pressure Calibration Per Agent Type" and §"Pressure-Performance Curve Fitting from Historical Data" as "Design — future work." Nothing to fix in code; just keep the doc honest.

---

## E.25 — Thompson-sampling `PressureBandit` is NOT DONE (Doc 12 §"Thompson sampling for pressure optimization")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 12 lines 519-567 specifies `PressureBandit { arms: Vec<PressureArm>, discount: f64 }` with 5 discrete arms (very-loose, loose, moderate, tight, very-tight) mapped to concrete `PressureConfig { max_iterations, cost_budget_usd, phase_timeout_secs, stuck_threshold, ghost_turn_max }`. Reward = `pass_rate / cost_usd`, Beta posterior with 0.995 discount for non-stationarity.
**Reality**: `Grep 'PressureBandit|PressureArm|PressureConfig|PRESSURE_CONFIGS'` returns **zero matches** in `crates/`. The `roko-learn` crate has a Thompson-sampling bandit used for model routing (`Cascade Router`), but no pressure bandit. The five documented `PressureConfig` tuples are design-only. The existing conductor constants happen to match the "moderate" arm (`max_iterations=3` — but this is in the gate/conductor spec, not a tunable arm).
**Fix sketch**: Either implement on top of the existing bandit infrastructure in `roko-learn` (the machinery for discounted Beta posteriors already exists for model routing) and persist to `.roko/learn/pressure-bandit.json`, or mark Doc 12 §"Thompson sampling for pressure optimization" as design-only. Prefer doc-only for now — no observable need.

---

## E.26 — `pressure_index` multi-dimensional scalar is NOT DONE (Doc 12 §"Pressure Calibration Per Agent Type" snippet)

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 12 lines 483-503 defines `pressure_index(iteration, max_iterations, cost_usd, cost_budget_usd, elapsed_ms, timeout_ms, stuck_count, stuck_threshold) -> f64` as a weighted combination `0.30*iter + 0.25*cost + 0.25*time + 0.20*stuck` with weights summing to 1.0.
**Reality**: `Grep 'pressure_index'` on `crates/` returns **zero matches**. The function does not exist. Individual ratios are computable from `ResourceAccount::token_utilisation() / cost_utilisation() / time_utilisation()` at `roko-runtime/src/resource.rs:92-114` but they are not combined. The `conductor.rs` routing-bias flow at `:277-285` does treat `cost-overrun | context-window-pressure | time-overrun` as a binary "load pressure" signal but with no weighted scoring.
**Fix sketch**: Adding a 15-line `pressure_index` helper would be trivial and would enable E.23 / E.24 / E.25 to land incrementally; defer until at least one consumer wants it. For now leave as design.

---

## E.27 — `FlowDetector` + `TurnMetrics::is_productive` is NOT DONE (Doc 12 §"Flow State Detection")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 12 §"Flow State Detection" lines 794-902 (Csikszentmihalyi framing) describes `FlowDetector { min_flow_turns: 3, flow_threshold_multiplier: 1.5, agent_flow: HashMap<String, FlowState> }` with `update(agent_id, turn: &TurnMetrics)`. `TurnMetrics { files_changed, gate_score_improved, tool_calls_diverse, context_usage_ratio }` with `is_productive()` = nonzero file changes + 20-85% context usage. Flow preservation bumps watcher thresholds by 50%.
**Reality**: `Grep 'FlowDetector|FlowState|TurnMetrics|is_productive'` on `crates/` returns **zero matches**. The struct, its update loop, and the 50% threshold bump are all absent. No code path in the conductor reduces watcher sensitivity when an agent shows productive-turn patterns. Doc 12 line 897-901 explicitly says the flow detector "does not override the circuit breaker" — but there is nothing to override in either direction.
**Fix sketch**: Tag Doc 12 §"Flow State Detection" as design-only. Implementing this requires per-turn `TurnMetrics` emission from the agent dispatcher, which is a cross-cutting surface change; defer until a concrete motivating failure mode appears.

---

## E.28 — Cognitive-load three-load partition (intrinsic/extraneous/germane) is design-only (Doc 12 §"Cognitive Load Theory Mapping")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 12 §"Cognitive Load Theory Mapping" lines 716-790 (Sweller framing) partitions context-window usage into intrinsic (task complexity), extraneous (irrelevant context), germane (productive scaffolding). "Reducing extraneous load creates room for germane load" is framed as an active conductor policy. Context-window pressure watcher at 80% is "the mechanism for enforcing this constraint."
**Reality**: The 80% threshold ships — `MAX_CONTEXT_USAGE_RATIO = 0.80` at `roko-conductor/src/watchers/context_window_pressure.rs:10`, firing when `tokens_used / tokens_total > 0.80`. But there is no load-partition accounting: no struct that classifies prompt sections into intrinsic/extraneous/germane, no `signal_ratio` field on prompt sections as Doc 12 lines 728-730 claims, no policy that preferentially drops extraneous content when the watcher fires. The `ContextAssembler` auction surface (`roko-compose/src/prompt.rs:77-260` — bidder enum, VCG payments, auction scoring) ranks content by bidder weight but not by cognitive-load category. Doc 12 line 759-761 names `SystemPromptBuilder` as the mechanism; that builder lives at `roko-compose/src/system_prompt_builder.rs` but does not implement the three-way partition.
**Fix sketch**: Either add an `explicit CognitiveLoad::{Intrinsic, Extraneous, Germane}` tag to every `PromptSection` consumed by the assembler + wire the context-pressure watcher to evict Extraneous first, or mark Doc 12 §"Cognitive Load Theory Mapping" as design-only narrative that informed the 80% threshold without shipping the partition itself.

---

## E.29 — `CooperationMetrics` signals are design-only; no roll-up exists (Doc 12 §"Cooperation Metrics")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 12 §"Cooperation Metrics" lines 232-243 tables six signals (merge-conflict rate, gate-pass-on-first-attempt rate, conductor-intervention rate, review approval rate, cost per task, token waste ratio) distinguishing cooperative (low rates) from collapsed (high rates) states. Doc 12 §"The Feedback Loop" lines 247-273 frames these as closing the learning loop for pressure re-calibration.
**Reality**: Individual metrics exist as efficiency events (`roko-learn/src/efficiency.rs`), gate results (`roko-gate`), and event-bus signals (`EventKind::PhaseTransition` in the orchestrator's event log), but no `CooperationMetrics` roll-up ships. `Grep 'CooperationMetric|cooperation_metrics'` returns zero matches. The cascade router logs outcome data (A.10 cross-check) but does not aggregate by cooperation signal. Doc 12 line 276 says "Extending it to track pressure-cooperation relationships enables automated Yerkes-Dodson tuning" — this extension is unimplemented.
**Fix sketch**: Defer unless there's a concrete downstream that needs the roll-up. Doc should be marked design-only.

---

## E.30 — Stigmergy framing (Grassé 1959) is conceptual only (Doc 12 §"Stigmergy and Pressure")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 12 §"Stigmergy and Pressure" lines 334-364 frames git commits as stigmergic traces and argues the conductor must preserve high-quality traces by calibrating pressure below the collapse point.
**Reality**: The only stigmergy-tagged code is `Kind::Pheromone` at `roko-core/src/kind.rs:91` with doc comment "A stigmergic pheromone (threat/opportunity/wisdom)" and the `SystemPromptBuilder` comment at `roko-compose/src/system_prompt_builder.rs:11` mentioning "Pheromone / stigmergic guidance". These are the pheromone Engram variant surfaced in F.15, unrelated to the git-commit-as-trace framing. No code inspects commit quality or adjusts pressure based on stigmergic quality. Purely conceptual framing for the pheromone context chunker.
**Fix sketch**: No fix needed; Doc 12 §"Stigmergy and Pressure" is explanatory narrative for the O(1)-per-agent coordination model, not a spec.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 8 (E.02, E.08, E.11, E.12, E.13, E.17, E.18, E.21) |
| PARTIAL | 9 (E.01, E.03, E.04, E.06, E.09, E.10, E.14, E.16, E.19) |
| NOT DONE | 13 (E.05, E.07, E.15, E.20, E.22, E.23, E.24, E.25, E.26, E.27, E.28, E.29, E.30) |
| SCAFFOLD | 0 |

**HIGH severity (2)**: E.05 (`HealthMonitor` built but not wired into orchestrator — contradicts Doc 06 "Implementation: Built" claim and the 10-second periodic snapshot collection narrative), E.14 (`ProcessSupervisor` has a `supervisor` field on `PlanRunner` but `supervisor.spawn(SpawnConfig)` is never called for real agent dispatch — agent spawning is handled by the parallel `roko-agent/src/process/{group,kill,registry}.rs` stack, so `supervisor.count()` returns zero during real runs and the doc/CLAUDE.md wiring claim is structurally misleading).

**MEDIUM severity (3)**: E.03 (four built-in checks ship but the second is named `golem_status`, post-dissolution naming drift flagged in A.08 and F.05), E.09 (`PhaseTransition` struct ships in `state_machine.rs` but the orchestrator emits raw `serde_json::json!` at the `EventKind::PhaseTransition` call sites instead of constructing the typed record — the four downstream consumers doc 10 lists are unreachable as-is), E.15 (Doc 13 §"Attempt Tracking" structural-fix for Issue #6 spawn races is completely absent; `ProcessHandle` has no `attempt_id` or `plan_id`/`task_id` association).

**LOW severity (14)**: E.01 field-name/type drift on `SystemSnapshot`, E.04 method-naming drift (`overall_status` not `check`), E.06 coverage-trend band thresholds diverge from doc prose (Critical does fire), E.07 VSM mapping is narrative-only, E.10 `adaptive_timeout_ms` ships but no consumer wires it into `ProviderConfig`, E.16 orphan reaper lives in `roko-agent` not `roko-runtime` and lacks the 30 s periodic loop, E.19 Unix discovery is `pgrep -P` everywhere (no cgroup branch), E.20 cgroup resource limits absent, E.22-E.30 (9 items) cover the Yerkes-Dodson theoretical surface which ships as zero code — `PressureDial`, `ModelPressureProfile`, `PressureBandit`, `pressure_index`, `FlowDetector`, cognitive-load partition, cooperation-metrics roll-up, and the stigmergy framing are all design-only per the `Grep 'Yerkes|PressureDial|FlowDetector|ModelPressureProfile|PressureBandit|CooperationMetric' crates/` sweep returning zero matches.

**Cross-cuts with earlier audits**:

- A.08 (Doc 00) independently flagged the `HealthMonitor` build-not-wired drift (E.05).
- A.09 (Doc 00) noted the 8-vs-4 phase timeout extension — E.08 confirms all 8 ship.
- F.05 (Doc 16) confirmed `roko-golem` dissolution — E.03 shows the `check_golem_status` naming is a lingering holdover.

**Recommended immediate fixes (prioritized)**:

1. E.05 HIGH — either wire `HealthMonitor` into `PlanRunner`'s event loop or strip the "Implementation: Built" badge from Doc 06.
2. E.14 HIGH — decide between unifying on `ProcessSupervisor` or stripping it from `PlanRunner`; the current dual-stack silently breaks `active_agents` accounting at `orchestrate.rs:3945, 4157`.
3. E.03 MEDIUM — rename `check_golem_status` → `check_chain_status` at `health.rs:159, 258` to complete the roko-golem dissolution.
4. E.09 MEDIUM — instantiate `state_machine::PhaseTransition` at the orchestrator's phase-change emission sites so the documented audit trail is actually reachable.
5. Doc 12 — add a top-of-file "Design — narrative-only" note so readers of the 919-line chapter don't mistake `PressureBandit` / `FlowDetector` / `YerkesDodsonEstimator` Rust snippets for shipping code (addresses E.22-E.30 in bulk).

## Agent Execution Notes

### E.05 / E.14 — Core Runtime Ownership Batches

This section contains two of the most important runtime tasks in batch `07`:

1. making `HealthMonitor` real in the orchestrator,
2. resolving who actually owns process and agent accounting.

Treat both as ownership problems, not just doc drift.

### E.09 / E.10 / E.15 — Bounded State-Machine Contract

Good outcome:

- typed `PhaseTransition` is either used on a real path or clearly treated as passive,
- adaptive timeout computation is either consumed or clearly advisory,
- attempt tracking is either minimally real or explicitly deferred.

### E.22-E.30 — Keep Pressure Theory Deferred

Doc 12 should default to truth-in-advertising work, not implementation work, unless a later learning pass explicitly owns it.

Acceptance criteria for this section:

- later agents can say which health/process/timeouts surfaces are live,
- ownership of process accounting is no longer ambiguous,
- pressure-theory chapters stop reading like current runtime dependencies.
