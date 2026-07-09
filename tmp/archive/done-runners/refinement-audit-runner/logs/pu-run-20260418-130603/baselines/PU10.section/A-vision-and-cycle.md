# A — Vision, Three-Phase Cycle, Scheduling (Docs 00, 01, 13)

Parity of the foundational framing chapters: what dreams are (idle-triggered
not mortality-triggered), the three-phase cycle (NREM → REM → Integration),
and scheduling / triggers (idle / scheduled / manual).

Most of this section is DONE or PARTIAL-approaching-DONE. The surprises
are: the scheduled (cron) trigger, the budget accounting, the heartbeat
policy, and the quality-adaptive idle-delay scheduler all ship — none
of which Doc 16 §"G1 Episode Replay Scheduler" acknowledges.

Generated 2026-04-16.

---

## A.01 — "Dream as death reframe" is honored across the stack (Doc 00 §"Vision")

**Status**: DONE
**Severity**: —
**Doc claim**: Dreams are idle-triggered offline consolidation, not a mortality or termination process. Vitality phases (Thriving / Stable / Conservation / Declining / Terminal) are removed.
**Reality**: `DreamTrigger` at `crates/roko-dreams/src/runner.rs:221-228` has exactly three variants: `Idle`, `Scheduled`, `Manual` — no `Dying` / `Terminal` / `Death` trigger. Cross-linked with batch 09 A.01 (mortality framing removed from daimon). `Grep 'mortality|dying|terminal_state' crates/roko-dreams --include=*.rs` returns zero matches.

---

## A.02 — Three cognitive cross-cuts framing is structurally honored (Doc 00 §"What Dreams Are")

**Status**: DONE
**Severity**: —
**Doc claim**: Dreams are one of three cognitive cross-cuts: Neuro (memory), Daimon (affect), Dreams (offline consolidation).
**Reality**: `crates/roko-dreams/Cargo.toml:16-18` declares `roko-core`, `roko-neuro`, `roko-learn`, `roko-agent`, `roko-primitives` as path deps. Dreams reads from Neuro (KnowledgeStore), receives affect from Daimon (`EmotionalTag` / PAD per batch 09 D.04), and writes consolidated insights back to Neuro. The three-way cross-cut structure is real.

---

## A.03 — Three-phase dream cycle (NREM → REM → Integration) ships as `DreamCycle` (Doc 01 §"Three-Phase Cycle")

**Status**: DONE
**Severity**: —
**Doc claim**: Dream cycle has three phases: NREM replay → REM imagination → Integration staging. Each phase has specific resource allocation and output format.
**Reality**: `DreamCycle` at `crates/roko-dreams/src/cycle.rs:333` is the core engine (2,910 LOC file). `DreamCycleReport` at `cycle.rs:67` carries timestamps + insights + processing metadata + cluster reports (`DreamClusterReport` at `:300`, `DreamClusterKey` at `:259`, `DreamOutcome` at `:282`). `AgentDispatcher` trait at `cycle.rs:50` is the pluggable backend for dream inference. The cycle runs NREM-like episode replay → REM-like imagination (see B.04) → integration into KnowledgeStore. Phase boundaries are implicit rather than explicit phase-enum types, but the three-stage pipeline is real.

---

## A.04 — Phase state machine and resource allocation (Doc 01 §"State Machine", §"Resource Allocation")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 01 describes a formal state machine across the three phases with resource allocation per phase (e.g., NREM 40% / REM 40% / Integration 20%).
**Reality**: The shipping `DreamBudget` at `runner.rs:156-216` tracks `max_tokens`, `max_cost_usd`, `max_duration_secs` plus consumed counters and provides `remaining_fraction()` + `exhausted()` — an honest budget surface. But there is **no phase-specific allocation split** inside the budget. The cycle in `cycle.rs` processes episodes end-to-end without budget partitioning per phase. Doc 01's 40/40/20 allocation is design-only.
**Fix sketch**: Doc 01 should mark the per-phase allocation split as `Design — Phase 2+`. Or the code could add a `PhaseBudget { nrem_fraction, rem_fraction, integration_fraction }` struct — but that's deepening work, not doc-parity scope.

---

## A.05 — Idle trigger with quality-adaptive delay ships (Doc 13 §"Three Trigger Types", §"Frequency Adaptation")

**Status**: DONE
**Severity**: —
**Doc claim**: Idle trigger fires after `idle_threshold_mins` of no task activity. Adaptive frequency: high dream quality → longer idle delays; low quality → shorter delays.
**Reality**: `DreamSchedulePolicy` at `runner.rs:244-276` ships with `enabled: bool, idle_threshold_mins: u64, scheduled_cron: Option<String>, manual_enabled: bool, quality_gain: f64 (0.75 default), quality_penalty: f64 (1.25 default)`. `idle_delay(report, budget)` at `:281-304` dynamically scales the idle threshold: `quality >= 1.5` → multiplies by `quality_gain` (0.75 = longer delays); `quality <= 0.5` → multiplies by `quality_penalty` (1.25 = shorter); if `budget.remaining_fraction() < 0.20` → also applies penalty. `dream_quality_score(report)` drives the decision. This is the frequency-adaptation logic Doc 13 describes, fully wired.

---

## A.06 — Scheduled (cron) trigger ships (Doc 13 §"Scheduled Trigger", Doc 16 §"G1 — Not implemented")

**Status**: DONE (Doc 16 drift — says NOT IMPLEMENTED)
**Severity**: MEDIUM
**Doc claim**: Doc 13 §"Three Trigger Types" lists scheduled (cron-like) triggers. Doc 16 §"G1 Episode Replay Scheduler" says: "`scheduled_interval_hours` config exists in spec but not in code".
**Reality**: Scheduled cron triggers **ship today**. `DreamSchedulePolicy.scheduled_cron: Option<String>` at `runner.rs:253`. `cron_delay(now)` at `runner.rs:308-316` parses the cron expression via `cron::Schedule::from_str()` (Cargo.toml:23 declares `cron = "0.12"` dep), computes the next fire time, returns `Duration` until then. `trigger_delay(trigger, report, budget, now)` at `runner.rs:329-343` dispatches on `DreamTrigger::{Idle, Scheduled, Manual}` — all three are live. Doc 16 is stale.
**Fix sketch**: Update Doc 16 §"G1" to mark Scheduled trigger as Done with anchor `runner.rs:308-316`. Update Doc 13 to reference the cron dependency.

---

## A.07 — Manual trigger / CLI dream run command status (Doc 13 §"Manual Trigger", Doc 16 §"G1 — Not implemented")

**Status**: DONE (older parity assumption stale)
**Severity**: LOW
**Doc claim**: Doc 16: "Manual trigger (CLI) — Not implemented — `roko dream run` CLI command not yet wired". Doc 13 §"Manual Trigger" describes the CLI surface.
**Reality**: Both halves now ship. At the crate level, `DreamTrigger::Manual` is a valid trigger variant at `runner.rs:227`, `trigger_delay(..., DreamTrigger::Manual, ...)` returns `Duration::ZERO` at `runner.rs:342`, and `DreamSchedulePolicy.manual_enabled: bool` at `runner.rs:255` gates it. At the CLI level, `cmd_dream` in `crates/roko-cli/src/main.rs:5585-5678` ships `dream run`, `dream report`, and `dream schedule`. Doc 16 is stale.
**Fix sketch**: Update Docs 13 and 16 to treat manual triggering as implemented, with CLI anchors.

---

## A.08 — Dream budget (token + cost + duration) ships (Doc 13 §"Scheduling", Doc 12 §"Budget Allocation")

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 12 §"Budget Allocation" describes per-cycle resource caps. Doc 13 mentions budget-aware scheduling.
**Reality**: `DreamBudget { max_tokens, max_cost_usd, max_duration_secs, consumed_tokens, consumed_cost_usd, consumed_duration_secs }` at `runner.rs:156-169`. `consume_episode(episode)` at `runner.rs:186-197` tallies tokens + cost + duration from an `Episode`. `remaining_fraction()` at `:201-207` returns the min of the three axes. `exhausted()` at `:211-215` returns true when any axis is capped. The default budget at `:171-182` is unbounded (`u64::MAX`, `f64::MAX`) — production deployments set real limits. This is wired into the schedule policy (see A.05): low budget → schedule penalty.

---

## A.09 — Heartbeat policy ships for daemon-friendly polling (Doc 13 §"Orchestrator Coordination")

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 13 §"Orchestrator Coordination" describes how dreams integrate with the running orchestrator — heartbeat-style polling for daemon mode.
**Reality**: `DreamHeartbeatPolicy { tick_interval_secs, delta_interval_mins, idle_grace_mins }` at `runner.rs:348-359`, with defaults. `DreamHeartbeatReport` at `runner.rs:373+` tracks `processed_through: Option<DateTime<Utc>>, recent_episode_count, ...`. This is daemon-polling infrastructure ready for a long-running dream service.

---

## A.10 — Intensive consolidation mode (backlog high/low water marks) is not implemented (Doc 13 §"Intensive Mode", Doc 16 §"G1")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: When the episode backlog exceeds a high watermark, the scheduler enters "intensive mode" — faster dreams, shorter idle gaps, until the backlog drops below a low watermark.
**Reality**: `Grep 'intensive|high_watermark|low_watermark|backlog_intensive' crates/roko-dreams --include=*.rs` returns zero matches. The shipping `quality_penalty` at `DreamSchedulePolicy` (see A.05) is the closest adjacent mechanism — it shortens idle delays when budget is low — but it does not cover the "backlog → intensive" semantics.
**Fix sketch**: Doc 13 §"Intensive Mode" should carry a `Design — Phase 2+` banner. If implemented, it would extend `DreamSchedulePolicy` with `{backlog_high: usize, backlog_low: usize, intensive_multiplier: f64}`.

---

## A.11 — Dream report persistence to `.roko/dreams/` ships (Doc 01 §"Integration Staging", Doc 16 §"G8")

**Status**: DONE
**Severity**: —
**Doc claim**: Dream reports are serialized to JSON and persisted in `.roko/dreams/` for inspection / replay / re-use.
**Reality**: `load_latest_dream_report(report_dir) -> Result<Option<DreamReport>>` at `runner.rs:796` scans the directory and returns the most recent report. Reports named `dream-{timestamp_ms}.json` per Doc 16 §"G8". Doc 16 correctly marks G8 as Implemented.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 8 (A.01, A.02, A.03, A.05, A.06, A.08, A.09, A.11) |
| PARTIAL | 2 (A.04 per-phase allocation, A.07 CLI dream run) |
| NOT DONE | 1 (A.10 intensive consolidation mode) |

Section A confirms that **most of the scheduling and cycle
infrastructure ships** — the idle-triggered cycle with quality-adaptive
delay, the cron scheduler, the budget accounting, the heartbeat
policy, and report persistence are all live. The gap is narrow:
per-phase budget allocation, backlog-intensive mode, and possibly the
CLI-level `roko dream run` command.

## Agent Execution Notes

### A.06 / A.11 — Doc 16 undercount fixes

Doc 16 §"G1 Episode Replay Scheduler" says the scheduled trigger is
"not implemented" — but `DreamSchedulePolicy.scheduled_cron` + `cron`
dep + `cron_delay()` ship today. Update Doc 16 to match.

### A.07 / A.09 — Runtime ownership is broader than the crate alone

Manual triggering is not merely a crate capability anymore. `roko-cli`
ships `dream run/report/schedule`, and daemon mode starts the dream
loop automatically. The docs should treat dreams as a real runtime
surface, not just a library.

### A.10 — Frontier

Intensive consolidation is a well-scoped future extension, not
blocking anything today.

Acceptance criteria:

- Doc 16 §"G1" no longer claims Scheduled trigger is unimplemented,
- Doc 16 no longer claims manual trigger / CLI support is unimplemented,
- Doc 01 per-phase budget allocation is banner-tagged,
- Doc 13 intensive mode is banner-tagged.
