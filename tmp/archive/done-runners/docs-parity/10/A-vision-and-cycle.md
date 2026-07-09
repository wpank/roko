# A - Vision, Cycle, Scheduling (Docs 00, 01, 13)

This section is mostly doc-honesty work. The core runtime already ships
the runner, cycle, budgeting, trigger policy, heartbeat policy, CLI
entry points, and persisted reports.

Generated: 2026-04-18

---

## Shipping Now

| Item | Status | Evidence |
|------|--------|----------|
| idle / scheduled / manual trigger model | DONE | `DreamTrigger` in `runner.rs:221-228`; `trigger_delay()` in `runner.rs:329-343` |
| schedule policy with cron and manual gating | DONE | `DreamSchedulePolicy` in `runner.rs:244-343` |
| budget accounting | DONE | `DreamBudget` in `runner.rs:156-216` |
| heartbeat / delta-loop polling | DONE | `DreamHeartbeatPolicy` and `DreamHeartbeatReport` in `runner.rs:347-500` |
| public runner facade | DONE | `DreamRunner` in `runner.rs:504-566` |
| core cycle and persisted reports | DONE | `DreamCycle` in `cycle.rs:333-540`; report write in `cycle.rs:681-689`; latest-report loader in `runner.rs:558-566, 829-848` |
| CLI surfaces | DONE | `cmd_dream` in `crates/roko-cli/src/main.rs:5609-5704` |
| daemon / orchestrator entry points | DONE | `daemon.rs:239-268`; `orchestrate.rs:5890-5969` |

---

## Main Corrections

### A.01 - Dreams are idle-triggered, not mortality-triggered

**Status**: DONE

The runtime only exposes `Idle`, `Scheduled`, and `Manual` triggers. The
older death-style framing is not present in the crate.

### A.02 - The three-stage runtime is real

**Status**: DONE

`DreamCycle` is the shipping consolidation engine, and `DreamCycleReport`
is the persisted output contract. The phase boundaries are lighter-weight
than the prose docs imply, but the runtime already has replay-ish input,
REM imagination, and integrated knowledge/report output.

### A.03 - Scheduled and manual triggers already ship

**Status**: DONE

This is the biggest status drift in the scheduling docs:

- scheduled cron support ships through `scheduled_cron` plus `cron_delay()`,
- manual triggering ships through `DreamTrigger::Manual`,
- and the CLI already exposes `dream run`, `dream report`, and `dream schedule`.

### A.04 - Budgeting and heartbeat already ship

**Status**: DONE

`DreamBudget` tracks token, cost, and duration ceilings. `DreamHeartbeatPolicy`
tracks daemon-friendly polling and delta-loop readiness. The docs should treat
both as current runtime, not as roadmap items.

### A.05 - Per-phase budget splits are still target-state

**Status**: TARGET-STATE

The runtime has one cycle budget, not a formal `NREM/REM/Integration`
allocation split. Any `40/40/20` style breakdown belongs in future-work
language.

### A.06 - Intensive consolidation mode is still target-state

**Status**: TARGET-STATE

There is no backlog high/low watermark mode in `roko-dreams`. Keep that
idea future-facing.

---

## What To Carry Into The Live Docs

- Doc 13 should present idle, scheduled, and manual triggers as current runtime.
- Doc 16 should stop marking scheduled/manual trigger support as absent.
- Doc 01 can keep the three-phase framing, but per-phase budget partitioning must be labeled as design-only.
- Any intensive-consolidation language should move to explicit future work.
