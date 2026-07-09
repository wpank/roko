# Batch Execution Contract

8 batches ordered for unattended execution. The goal is not just to “cover the conductor docs”, but to let an agent turn conductor-parity findings into bounded work that can run overnight without guessing which conductor surfaces are already real.

---

## Batch Posture

- Default strategy: **wire already-shipped conductor subsystems into the CLI runtime before inventing new control theory**.
- Treat `crates/roko-cli/src/orchestrate.rs` as the primary production conflict hotspot.
- Treat `crates/roko-conductor/src/{health,stuck_detection,circuit_breaker,state_machine,diagnosis}.rs` as the main contract modules.
- Treat the `ProcessSupervisor` vs `roko-agent/src/process/registry.rs` split as a first-class runtime ownership problem, not a naming nit.
- If a task starts requiring Yerkes-Dodson primitives, Good Regulator metrics, typed cognitive-signal redesign, watcher CEP composition, or conductor federation, record the seam and stop.
- Every completed batch should leave behind:
  - code changes,
  - verification command output,
  - explicit deferrals,
  - and any newly clarified runtime contract.

Required reads for every batch:

- [00-INDEX.md](00-INDEX.md)
- the owning section file(s) named below
- [SOURCE-INDEX.md](SOURCE-INDEX.md)
- [context-pack/agent-runbook.md](context-pack/agent-runbook.md)
- [context-pack/carry-forward-map.md](context-pack/carry-forward-map.md)

---

## Recommended Serial Order

For a single long-running agent run, prefer:

`C1 -> C2 -> C3 -> C4 -> C5 -> C6 -> C7 -> C8`

This order first activates the dark runtime seams, then fixes ownership and persistence failures, then clarifies the state-machine and diagnosis contracts, and only after that resolves conductor-doc honesty and frontier tagging.

---

## Batch Overview

| Batch | Tasks | Purpose | Primary Write Scope | Verify Focus | Est. LOC |
|-------|-------|---------|---------------------|--------------|----------|
| C1 | E.05, A.08, E.03 | Wire `HealthMonitor` into the orchestrator and resolve the `golem_status` holdover if touched | `roko-cli`, `roko-conductor` | `cargo test -p roko-cli -p roko-conductor` | 220 |
| C2 | D.11, D.14 | Wire `StuckDetector` + `MetaCognitionHook` into a real runtime cadence | `roko-cli`, `roko-conductor` | `cargo test -p roko-cli -p roko-conductor` | 240 |
| C3 | E.14 | Pick one canonical owner for agent/process accounting | `roko-cli`, `roko-runtime`, `roko-agent` | `cargo test -p roko-cli -p roko-runtime -p roko-agent` | 180 |
| C4 | C.09 | Persist circuit-breaker state across snapshots and restarts | `roko-orchestrator`, `roko-conductor`, `roko-cli` | `cargo test -p roko-orchestrator -p roko-conductor -p roko-cli` | 160 |
| C5 | E.09, E.10, E.15 | Make the state-machine / timeout / attempt-tracking contract honest and bounded | `roko-cli`, `roko-conductor`, `roko-learn`, docs if needed | `cargo test -p roko-cli -p roko-conductor -p roko-learn` | 180 |
| C6 | D.09, D.17, C.13, C.16, B.19 | Clean up diagnosis / watcher / signal contract drift without expanding into theory work | `roko-conductor`, `roko-cli`, docs | `cargo test -p roko-conductor -p roko-cli` | 140 |
| C7 | A.10, A.11, F.21 | Conductor status and meta-doc honesty pass (`RoutingBias`, real signal kinds, retry-path bandit status) | docs, `CLAUDE.md`, small code/docs fixes if needed | `rg -n "RoutingBias|conductor\\.intervention|conductor:alert|ConductorBandit|golem_status" docs CLAUDE.md tmp/docs-parity/07 crates/roko-conductor crates/roko-cli` | 100 |
| C8 | B.17-B.18, E.22-E.30, F.05-F.14, F.22, F.24-F.26 | Mark design-only conductor theory sections explicitly and leave clean handoffs | docs only | `rg -n "Design — not yet implemented|Planned extension|Scaffold|PressureBandit|FlowDetector|CognitiveSignal|SelfHealingConductor" docs/07-conductor tmp/docs-parity/07` | 60 |

---

## Dependency Graph

| Batch | Depends on |
|-------|------------|
| C1 | — |
| C2 | C1 |
| C3 | — |
| C4 | — |
| C5 | C1 |
| C6 | C2 |
| C7 | C1, C3, C5, C6 |
| C8 | C7 |

Why `C2 -> C1`:

- both touch the same `PlanRunner` cadence seam in `orchestrate.rs`, and it is cleaner to establish the health tick before adding meta-cognition to it.

Why `C5 -> C1`:

- typed phase-transition and timeout cleanup is easier once the runtime already has one explicit periodic/control seam.

Why `C6 -> C2`:

- diagnosis/catalog drift is easier to evaluate after `MetaCognitionHook` and stuck semantics are no longer hypothetical runtime surfaces.

Why `C7` comes after `C1/C3/C5/C6`:

- the doc/status pass should reflect the runtime decisions made by the earlier batches rather than race ahead of them.

Why `C8` is last:

- frontier banners should reflect the final post-runtime-handoff truth, not the pre-fix state.

Parallel-safe groups:

- `{C1, C3, C4}` can start immediately.
- `C2` waits for `C1`.
- `C5` waits for `C1`.
- `C6` waits for `C2`.
- `C7` waits for `C1, C3, C5, C6`.
- `C8` should be last.

Conflict groups:

| Group | Crates / Files | Batches |
|-------|----------------|---------|
| orchestrate-core | `crates/roko-cli/src/orchestrate.rs` | C1, C2, C3, C4, C5, C6 |
| conductor-runtime | `crates/roko-conductor/src/{health,stuck_detection,circuit_breaker,state_machine,diagnosis}.rs` | C1, C2, C4, C5, C6 |
| agent-process | `crates/roko-runtime/src/process.rs`, `crates/roko-agent/src/process/*` | C3 |
| snapshot | `crates/roko-orchestrator/src/executor/snapshot.rs` | C4 |
| docs-contract | `docs/07-conductor/*`, `CLAUDE.md`, `tmp/docs-parity/07/*` | C6, C7, C8 |

---

## Batch Details

### C1 — HealthMonitor Runtime Activation

**Owns**: `E.05`, `A.08`, `E.03`

**Read first**:

- [E-health-adaptive.md](E-health-adaptive.md)
- [A-architecture.md](A-architecture.md)

**Problem**: `HealthMonitor` ships with four checks and tests, but the orchestrator never constructs `SystemSnapshot` or calls `overall_status()`. The `golem_status` naming holdover also makes the health story harder to trust.

**Scope**:

1. Add one real orchestrator path that constructs `SystemSnapshot`.
2. Run the monitor on a real cadence or other explicit control point.
3. Emit or otherwise surface degraded/critical status in a way the conductor can observe.
4. If the code is touched anyway, prefer renaming `golem_status` to `chain_status`.

**Out of scope**:

- adding disk-pressure checks,
- redesigning the health API,
- implementing VSM mapping,
- Yerkes-Dodson pressure work.

**Files**:

- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-conductor/src/health.rs`
- docs only if the runtime contract remains intentionally narrow

**Verify**:

```bash
cargo test -p roko-cli -p roko-conductor
rg -n "HealthMonitor|SystemSnapshot|overall_status|check_(golem|chain)_status" crates/roko-cli crates/roko-conductor
```

**Acceptance criteria**:

- `HealthMonitor` is no longer library-only,
- a real `SystemSnapshot` is constructed from runtime state,
- the post-dissolution check naming is either fixed or explicitly documented.

---

### C2 — MetaCognition Runtime Activation

**Owns**: `D.11`, `D.14`

**Read first**:

- [D-diagnosis-stuck.md](D-diagnosis-stuck.md)
- [B-watchers-signals.md](B-watchers-signals.md)

**Problem**: `StuckDetector` and `MetaCognitionHook` are substantial, tested, and still dark. Only the simpler `StuckPatternWatcher` is live today.

**Scope**:

1. Add one real runtime cadence for `MetaCognitionHook::assess(...)`.
2. Build `ActivityEntry` values from existing runtime or signal-stream data.
3. Make at least one previously dark stuck heuristic observable in production.
4. Keep the bridge to the conductor small and explicit.

**Out of scope**:

- creating a second watcher subsystem,
- redesigning stuck heuristics,
- adding new `StuckKind` variants,
- learning-driven stuck tuning.

**Files**:

- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-conductor/src/stuck_detection.rs`
- `crates/roko-conductor/src/conductor.rs` only if a tiny bridge helper is needed

**Verify**:

```bash
cargo test -p roko-cli -p roko-conductor
rg -n "MetaCognitionHook|StuckDetector|ActivityEntry|assess\\(" crates/roko-cli crates/roko-conductor
```

**Acceptance criteria**:

- `MetaCognitionHook` has at least one production caller,
- runtime `ActivityEntry` values are derived from live data rather than invented test scaffolding,
- the batch leaves at least one dark stuck heuristic meaningfully reachable.

---

### C3 — Process Ownership Unification

**Owns**: `E.14`

**Read first**:

- [E-health-adaptive.md](E-health-adaptive.md)
- [SOURCE-INDEX.md](SOURCE-INDEX.md)

**Problem**: the runtime currently has two owners for agent/process accounting, and the one on `PlanRunner` does not own real spawns.

**Scope**:

1. Choose one canonical owner: `ProcessSupervisor`, agent registry, or a clearly documented split.
2. Fix `active_agents` and related runtime accounting to read from the chosen source.
3. Resolve the shutdown story consistently with that owner.
4. Update claims in docs or `CLAUDE.md` that depend on the old assumption.

**Out of scope**:

- full attempt tracking,
- Linux cgroup limits,
- moving every orphan-reaper behavior across crates unless required by the chosen ownership model.

**Files**:

- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-runtime/src/process.rs`
- `crates/roko-agent/src/process/*`
- `CLAUDE.md` if the status text changes

**Verify**:

```bash
cargo test -p roko-cli -p roko-runtime -p roko-agent
rg -n "ProcessSupervisor|supervisor\\.spawn|supervisor\\.count|registered_pids|cleanup_orphaned" crates/roko-cli crates/roko-runtime crates/roko-agent
```

**Acceptance criteria**:

- later agents can name one authoritative source for active-agent accounting,
- shutdown behavior aligns with the same source,
- the runtime no longer reports agent counts from a structurally dark path.

---

### C4 — CircuitBreaker Snapshot Persistence

**Owns**: `C.09`

**Read first**:

- [C-decision-space.md](C-decision-space.md)

**Problem**: doc `02` promises breaker persistence across snapshots, but `ExecutorSnapshot` currently has no place to store it and recovery reconstructs a fresh breaker.

**Scope**:

1. Add the smallest durable representation of breaker failure records.
2. Snapshot and restore that state through the real executor path.
3. Keep serialization explicit and testable.
4. Avoid widening into provider-health or unrelated retry breakers.

**Out of scope**:

- changing `MAX_PLAN_FAILURES`,
- redesigning breaker semantics,
- merging provider-health and plan-level breaker models.

**Files**:

- `crates/roko-orchestrator/src/executor/snapshot.rs`
- `crates/roko-orchestrator/src/executor/mod.rs`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-conductor/src/circuit_breaker.rs`

**Verify**:

```bash
cargo test -p roko-orchestrator -p roko-conductor -p roko-cli
rg -n "failure_records|ExecutorSnapshot|CircuitBreaker" crates/roko-orchestrator crates/roko-conductor crates/roko-cli
```

**Acceptance criteria**:

- breaker records survive snapshot round-trips,
- restart behavior no longer silently resets the plan-level failure budget,
- docs can honestly claim persistence.

---

### C5 — State-Machine And Timeout Contract

**Owns**: `E.09`, `E.10`, `E.15`

**Read first**:

- [E-health-adaptive.md](E-health-adaptive.md)
- [C-decision-space.md](C-decision-space.md)

**Problem**: typed `PhaseTransition` and `adaptive_timeout_ms` are both real surfaces, but production still underuses them, and attempt-tracking remains absent.

**Scope**:

1. Decide whether the real fix is code activation or docs demotion for typed `PhaseTransition`.
2. Decide whether `adaptive_timeout_ms` should actually drive a provider timeout or remain an advisory metric.
3. Keep attempt-tracking honest: either add the smallest real contract or explicitly demote the doc claim.
4. Make the state-machine/timeout story easier for later agents to execute without guessing.

**Out of scope**:

- new timeout theory,
- new process race architectures,
- Yerkes-Dodson pressure or flow modeling.

**Files**:

- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-conductor/src/state_machine.rs`
- `crates/roko-learn/src/latency.rs`
- docs `10` and `13` if needed

**Verify**:

```bash
cargo test -p roko-cli -p roko-conductor -p roko-learn
rg -n "PhaseTransition|adaptive_timeout_ms|timeout_ms|attempt_id" crates/roko-cli crates/roko-conductor crates/roko-learn crates/roko-agent crates/roko-runtime
```

**Acceptance criteria**:

- later agents can tell whether `PhaseTransition` is a live runtime contract or a passive type,
- adaptive timeout behavior has one honest interpretation,
- attempt-tracking is either minimally real or explicitly deferred.

---

### C6 — Diagnosis And Signal Contract Cleanup

**Owns**: `D.09`, `D.17`, `C.13`, `C.16`, `B.19`

**Read first**:

- [D-diagnosis-stuck.md](D-diagnosis-stuck.md)
- [C-decision-space.md](C-decision-space.md)
- [B-watchers-signals.md](B-watchers-signals.md)

**Problem**: the diagnosis tables, restart semantics, cooldown promises, and cognitive-signal wording still drift across docs and code.

**Scope**:

1. Decide whether category/intervention drift should be fixed in code or docs.
2. Decide whether missing doc-14 variants should be added or explicitly removed from the docs.
3. Make the restart-vs-breaker distinction and per-watcher cooldown story explicit.
4. Make doc `09` honest about `CognitiveSignal` without widening into a signal redesign.

**Out of scope**:

- building typed `CognitiveSignal`,
- CEP composition,
- learning-driven diagnosis confidence updates,
- new cooldown algorithms beyond the documented 120 s promise.

**Files**:

- `crates/roko-conductor/src/diagnosis.rs`
- `crates/roko-conductor/src/conductor.rs`
- `docs/07-conductor/*`
- `tmp/docs-parity/07/*`

**Verify**:

```bash
cargo test -p roko-conductor -p roko-cli
rg -n "ImportError|ImportNotFound|RetryWithContext|RetryWithFix|cooldown|CognitiveSignal|conductor\\.intervention" crates docs/07-conductor tmp/docs-parity/07
```

**Acceptance criteria**:

- later agents can tell which diagnosis and signal contracts are canonical,
- doc `09` no longer overclaims `CognitiveSignal`,
- the cooldown and restart semantics are no longer ambiguous.

---

### C7 — Status And Meta-Doc Honesty

**Owns**: `A.10`, `A.11`, `F.21`

**Read first**:

- [A-architecture.md](A-architecture.md)
- [F-theory-learning.md](F-theory-learning.md)

**Problem**: several docs still underspecify or misstate real conductor surfaces such as `RoutingBias`, emitted signal kinds, and retry-path bandit wiring.

**Scope**:

1. Document `RoutingBias` where the architecture currently omits it.
2. Make the real signal-kind story explicit instead of repeating the stale `conductor.intervention` literal.
3. Correct `ConductorBandit` status in the theory/learning docs.
4. Keep changes focused on accurate status, not new subsystem design.

**Out of scope**:

- replacing `WorstSeverityPolicy` with learned policy,
- redesigning signal names across the codebase unless a tiny cleanup is justified,
- rewriting the entire conductor docs set.

**Files**:

- `docs/07-conductor/*`
- `CLAUDE.md`
- `tmp/docs-parity/07/*`
- tiny code/docs alignment only if needed

**Verify**:

```bash
rg -n "RoutingBias|conductor\\.intervention|conductor:alert|ConductorBandit|golem_status" docs CLAUDE.md tmp/docs-parity/07 crates/roko-conductor crates/roko-cli
```

**Acceptance criteria**:

- meta-docs no longer understate live conductor surfaces,
- signal-kind docs match the actual emitted kinds,
- later agents can trust the conductor status docs as execution context.

---

### C8 — Frontier Demotion And Handoff Pass

**Owns**: `B.17-B.18`, `E.22-E.30`, `F.05-F.14`, `F.22`, `F.24-F.26`

**Read first**:

- [E-health-adaptive.md](E-health-adaptive.md)
- [F-theory-learning.md](F-theory-learning.md)
- [B-watchers-signals.md](B-watchers-signals.md)

**Problem**: several theory-heavy chapters still read more “implemented” than the code supports.

**Scope**:

1. Mark clearly design-only sections as design-only.
2. Preserve the distinction between “adjacent runtime pieces exist” and “the described subsystem exists”.
3. Leave explicit handoffs for pressure tuning, self-model work, signal redesign, and conductor federation.

**Out of scope**:

- implementing the frontier systems themselves,
- collapsing every theory doc into a stub,
- changing shipped runtime behavior to satisfy theoretical sections.

**Files**:

- `docs/07-conductor/*`
- `tmp/docs-parity/07/*`

**Verify**:

```bash
rg -n "Design — not yet implemented|Planned extension|Scaffold|PressureBandit|FlowDetector|CognitiveSignal|SelfHealingConductor" docs/07-conductor tmp/docs-parity/07
```

**Acceptance criteria**:

- later agents can immediately tell which conductor-theory sections are runtime, partial, or frontier,
- doc banners stop contradicting the body,
- handoffs to later batches are explicit instead of implied.
