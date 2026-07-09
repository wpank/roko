# 06 — Process Management

> **Priority**: 🟡 P1 — Required for multi-agent stability
> **Parity sections**: §8 (Process management)
> **Checklist ref**: `MORI-PARITY-CHECKLIST.md` §8

## Problem statement

Mori has robust process management for spawned Claude subprocesses:
- PID tracking and orphan reaping (`register_spawned_pid`)
- `kill_all_descendants()` for process tree cleanup
- SIGTERM → SIGKILL escalation
- Stderr monitoring with known-warning classification

Roko's `ExecAgent` has basic timeout and kill, but no:
- PID registry
- Process tree cleanup
- Descendant reaping
- Orphan protection

## Checklist

- [ ] **6.1** PID registry — global registry of spawned subprocess PIDs
  - Mori ref: `/Users/will/dev/uniswap/bardo/apps/mori/src/agent/connection.rs:2624-2626`
- [ ] **6.2** `kill_all_descendants(pid)` — walk process tree, kill children first
  - Checklist: §8.6 (currently marked `[ ]` — was false `[x]`)
- [ ] **6.3** Orphan reaper — background task that kills registered PIDs on parent exit
- [ ] **6.4** SIGTERM → SIGKILL escalation with configurable grace period
- [ ] **6.5** Stderr monitoring with classification
  - Known warnings to suppress (codex state DB migration, etc.)
  - Mori ref: `connection.rs:2630-2655`, `classify_known_warning()` at line 826
- [ ] **6.6** Process group management — `setsid` for subprocess isolation
- [ ] **6.7** Resource limits — per-agent CPU/memory caps
- [ ] **6.8** Graceful shutdown — drain active agents before exit

> Maps to checklist: §8.1-8.8
