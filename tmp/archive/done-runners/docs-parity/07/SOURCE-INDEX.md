# SOURCE-INDEX — Key Anchors For 07-Conductor Refresh

Refreshed source anchors for the narrowed conductor parity pass.

Generated: 2026-04-18

---

## Conductor Core

| Anchor | Why it matters |
|--------|----------------|
| `crates/roko-core/src/traits.rs:166-171` | Live `Policy` trait used by watchers and `Conductor` |
| `crates/roko-conductor/src/conductor.rs:53-61` | `Conductor` fields: watchers, policy, breaker, `RoutingBias` |
| `crates/roko-conductor/src/conductor.rs:82-99` | `Conductor::new()` wires all 10 watchers |
| `crates/roko-conductor/src/conductor.rs:156-186` | Main breaker -> watcher -> decision evaluation path |
| `crates/roko-conductor/src/conductor.rs:226-249` | `Conductor` emits `conductor.decision` |
| `crates/roko-conductor/src/conductor.rs:258-315` | `RoutingBias` derivation from live watcher output |
| `crates/roko-conductor/src/interventions.rs:22-44` | `Severity` and severity -> decision mapping |
| `crates/roko-conductor/src/interventions.rs:99-121` | `InterventionPolicy` and `WorstSeverityPolicy` |
| `crates/roko-conductor/src/interventions.rs:123-144` | `conductor:alert:<watcher>` emission |

## Watchers, Diagnosis, And Support Modules

| Anchor | Why it matters |
|--------|----------------|
| `crates/roko-conductor/src/watchers/mod.rs:8-28` | All 10 watcher modules and re-exports |
| `crates/roko-conductor/src/diagnosis.rs:22-67` | 20 `ErrorCategory` variants |
| `crates/roko-conductor/src/diagnosis.rs:71-94` | 9 `SuggestedIntervention` variants |
| `crates/roko-conductor/src/diagnosis.rs:147-153` | `DiagnosisEngine::new()` |
| `crates/roko-conductor/src/diagnosis.rs:277-531` | Built-in pattern registry |
| `crates/roko-conductor/src/health.rs:87-114` | `SystemSnapshot` |
| `crates/roko-conductor/src/health.rs:148-172` | `HealthMonitor::new()` and four built-in checks |
| `crates/roko-conductor/src/health.rs:180-194` | `check_all()` and `overall_status()` |
| `crates/roko-conductor/src/stuck_detection.rs:30-47` | 6 `StuckKind` variants |
| `crates/roko-conductor/src/stuck_detection.rs:171-233` | `check_stuck()` and `check_all()` |
| `crates/roko-conductor/src/stuck_detection.rs:235-273` | `meta_cognition()` / theta-frequency assessment |

## Breaker, Snapshots, And Runtime Integration

| Anchor | Why it matters |
|--------|----------------|
| `crates/roko-conductor/src/circuit_breaker.rs:11` | `MAX_PLAN_FAILURES = 2` |
| `crates/roko-conductor/src/circuit_breaker.rs:28-44` | Live plan-level `CircuitBreaker` type |
| `crates/roko-cli/src/orchestrate.rs:4718-4727` | Dispatch refusal for tripped plans |
| `crates/roko-cli/src/orchestrate.rs:4729-4775` | Orchestrator conductor check path |
| `crates/roko-cli/src/orchestrate.rs:4706-4708` | `conductor.circuit_breaker` emission |
| `crates/roko-orchestrator/src/executor/snapshot.rs:27-44` | Current snapshot fields; no breaker-state field |

## Process Ownership And Learning Integration

| Anchor | Why it matters |
|--------|----------------|
| `crates/roko-runtime/src/process.rs:13-15` | `ProcessSupervisor` is a real runtime surface |
| `crates/roko-cli/src/orchestrate.rs:2604` | `PlanRunner` stores a `ProcessSupervisor` |
| `crates/roko-cli/src/orchestrate.rs:4855-4856` | Runtime still reads active-agent count from supervisor |
| `crates/roko-agent/src/process/registry.rs:46` | Agent-process registry is also live |
| `crates/roko-conductor/Cargo.toml:13-21` | `roko-conductor -> roko-learn` dependency is real |
| `crates/roko-runtime/src/event_bus.rs:101-130` | `RokoEvent` currently has exactly 2 variants |
| `crates/roko-runtime/src/event_bus.rs:167-219` | Live generic `EventBus<E>` surface |
| `crates/roko-learn/src/conductor.rs:108-223` | `ConductorBandit` implementation |
| `crates/roko-cli/src/orchestrate.rs:3797-3799` | Retry-path bandit load wiring |
| `crates/roko-cli/src/orchestrate.rs:7182-7183` | Retry-path bandit success recording |
| `crates/roko-cli/src/orchestrate.rs:7294-7295` | Retry-path bandit failure recording |
| `crates/roko-cli/src/orchestrate.rs:7311-7331` | Retry-path bandit action selection |

## Parity Guidance

Use the anchors above to keep the docs aligned with the current codebase:

- shipped conductor core in present tense,
- support modules called implemented,
- retry-path learning called live but narrow,
- theory and federation material marked informational or Phase 2+.
