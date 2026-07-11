# Duplicate and Conflicting Type Definitions

## Critical — Active runtime conflicts

### GateVerdict — 4 definitions
| Location | Fields |
|----------|--------|
| `roko-core/foundation.rs:368` | `gate_name, passed, skipped, output, duration_ms` |
| `roko-learn/episode_logger.rs:90` | `gate, passed, signature` |
| `roko-core/dashboard_snapshot.rs:290` | `plan_id, task_id, gate, passed, ts_millis` |
| `roko-chain/identity_economy_identity.rs:1600` | `gate: GateType, passed, score: f64, detail` |

Three incompatible types share the name, resolved only by import path. No conversion traits.

### Usage / TokenUsage — 4 definitions
| Location | Types |
|----------|-------|
| `roko-core/foundation.rs:159` — TokenUsage | `u64, u64, u64, f64` |
| `roko-core/chat_types.rs:111` — Usage | `u32, u32, u32, u32, f32, u64` |
| `roko-learn/episode_logger.rs:123` — Usage | `u64, u64, u64, u64, f64, f64, u64` |
| `roko-cli/runner/output_sink.rs:38` — TokenUsage | `u64, u64, u64, u64` (no cost) |

Field types differ (`u32` vs `u64`, `f32` vs `f64`). Silent precision loss at conversion boundaries.

### AgentEvent — 3 definitions
- `roko-agent/task_runner.rs:75`: 4 runtime feedback variants
- `roko-learn/events.rs:15`: 9 learning pipeline variants (superset)
- `roko-cli/inline/agent_events.rs:16`: 9 UI streaming variants (disjoint)

## High — Structural duplicates that will drift

### AgentConfig — 3 definitions
- `roko-core/config/agent.rs:29`: TOML parsing struct
- `roko-cli/config.rs:140`: Subprocess runner config
- `roko-agent/lifecycle.rs:680`: Full deployment manifest

### BudgetConfig — 3 definitions
- `roko-core/config/budget.rs:10`: `f32` monetary values
- `roko-cli/config.rs:335`: `f64` monetary values (precision mismatch)
- `roko-agent/lifecycle.rs:337`: Daily/hourly limits

### RetentionPolicy — 3 definitions
- `roko-learn/episode_logger.rs:1229`: Episode compaction
- `roko-fs/gc.rs:32`: Filesystem GC
- `roko-serve/retention.rs:20`: Artifact rotation

### Plan + PlanTask — 2+2 definitions
- `roko-cli/plan.rs:152` + `roko-serve/plan_types.rs:8`: Acknowledged copy, drift risk.

### TaskStatus — 3 definitions
- `roko-core/task.rs:66`: `Pending, Active, Done, Blocked`
- `roko-cli/tui/state.rs:101`: Adds `Failed`
- `roko-runtime/task_scheduler.rs:25`: `Blocked, Ready, Running, Completed, Failed, Skipped`

### DashboardSnapshot — 3 definitions (completely different shapes)
- `roko-core/dashboard_snapshot.rs:759`: Canonical push-based state
- `roko-cli/runner/projection.rs:124`: Event window ring
- `roko-cli/tui/dashboard.rs:3308`: Learning metrics (TUI only)

## Medium — Naming inconsistencies

- `Signal` is a compat re-export of `Engram` (`signal.rs:6`). Rename not completed.
- `Engram` exists in both `roko-core` and `roko-chain` with incompatible fields.
- `AgentId` defined 3 times as `String` + once as `u256` in chain.
- `GateVerdictSummary`, `GateResultSummary`, `GateResult`, `GateStatus` — all 2+ definitions.
