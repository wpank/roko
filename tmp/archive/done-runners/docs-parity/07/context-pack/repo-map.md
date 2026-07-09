# Repo Map - Shared Conductor Context

Quick reference for the docs-only parity refresh in `tmp/docs-parity/07/`.

## Workspace Root

`/Users/will/dev/nunchi/roko/roko/`

## Repo Snapshot

- workspace members: 36
- total Rust LOC: 322,088
- `docs/07-conductor/`: 17 files, 8,718 lines
- `crates/roko-conductor/src/`: 19 files, 6,122 lines
- `crates/roko-conductor/src/watchers/`: 11 files, 2,246 lines
  (`mod.rs` + 10 watcher implementations)
- `tmp/docs-parity/07/context-pack/`: 5 docs-refresh context files

## High-Value Paths

| What | Path | Why It Matters In This Refresh |
|------|------|--------------------------------|
| Conductor architecture docs | `docs/07-conductor/` | source material whose status language needs refresh |
| Conductor crate root | `crates/roko-conductor/src/lib.rs` | confirms exported surfaces and current module set |
| Conductor core | `crates/roko-conductor/src/conductor.rs` | proves the 10-watcher loop, `CircuitBreaker`, and `RoutingBias` |
| Breaker | `crates/roko-conductor/src/circuit_breaker.rs` | confirms plan-level failure tracking and current threshold |
| Diagnosis engine | `crates/roko-conductor/src/diagnosis.rs` | confirms the live diagnosis surface described in docs |
| Watcher implementations | `crates/roko-conductor/src/watchers/` | confirms there are 10 watcher implementations in the default set |
| Orchestrator call sites | `crates/roko-cli/src/orchestrate.rs` | proves breaker checks, conductor checks, and diagnosis usage |
| Retry-path bandit | `crates/roko-learn/src/conductor.rs` | confirms `ConductorBandit` is a live persisted surface |
| Process ownership surfaces | `crates/roko-runtime/src/process.rs`, `crates/roko-agent/src/process/` | background only; document honestly, do not resolve here |
| Health / stuck / state-machine library surfaces | `crates/roko-conductor/src/{health,stuck_detection,state_machine}.rs` | background for careful status wording, not implementation scope |
| Owned refresh docs | `tmp/docs-parity/07/` | only editable package in this task |

## Important Corrections

Use these instead of the older execution-oriented posture:

- the conductor core is already live: 10 default watchers, breaker checks,
  `WorstSeverityPolicy`, and orchestrator call sites are all present,
- `DiagnosisEngine` is already wired into live orchestrator code,
- `ConductorBandit` is already live in the retry path,
- `RoutingBias` is a real conductor surface and should be named as such,
- theory, federation, and self-healing items should be treated as deferred,
  not current batch objectives,
- implementation work outside the owned docs files should be handed off.

## Search Priorities

Before editing, verify these first:

```bash
rg -n "Conductor::new|watchers.len|WorstSeverityPolicy|RoutingBias|evaluate\\(" \
  crates/roko-conductor/src/conductor.rs
rg -n "CircuitBreaker|MAX_PLAN_FAILURES|record_failure|is_tripped|is_broken" \
  crates/roko-conductor/src/circuit_breaker.rs \
  crates/roko-cli/src/orchestrate.rs
rg -n "DiagnosisEngine|diagnose\\(" \
  crates/roko-conductor/src/diagnosis.rs \
  crates/roko-cli/src/orchestrate.rs
rg -n "ConductorBandit|load_or_new|record_outcome|select_action" \
  crates/roko-learn/src/conductor.rs \
  crates/roko-cli/src/orchestrate.rs
rg -n "HealthMonitor|StuckDetector|MetaCognitionHook|PhaseTransition|adaptive_timeout_ms|ProcessSupervisor" \
  crates/roko-conductor/src \
  crates/roko-cli/src/orchestrate.rs \
  crates/roko-runtime/src/process.rs \
  crates/roko-agent/src/process
```

## Verification Commands

```bash
rg --files docs/07-conductor | wc -l
find docs/07-conductor -type f | xargs wc -l | tail -n 1
rg --files crates/roko-conductor/src | wc -l
find crates/roko-conductor/src -type f | xargs wc -l | tail -n 1
find crates/roko-conductor/src/watchers -type f | xargs wc -l | tail -n 1
```

## Practical Rules

1. Refresh docs from source-backed reality, not from older batch language.
2. Treat Rust implementation work as a handoff unless the task scope changes.
3. Keep theory and federation visible, but explicitly deferred.
