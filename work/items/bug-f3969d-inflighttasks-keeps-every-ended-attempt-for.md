+++
id = "bug-f3969d"
kind = "bug"
title = "InFlightTasks keeps every ended attempt for the life of the process"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-tamper's report, branch work/gap-b72761 at 7531304ca)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/sibling_settle.rs"]
lane = "rust-cold"
parent = "spec-9a3131"
links = { depends_on = ["gap-b72761"], blocks = [], related = ["gap-b72761"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn in_flight_tasks_forget_settled_attempts' crates/roko-cli/src/ && cargo test -p roko-cli --lib in_flight_tasks_forget_settled_attempts"
+++

## Problem

`InFlightTasks` (`graph_task_dispatch/sibling_settle.rs:25` on gap-b72761's branch) moves each attempt into `ended: Mutex<Vec<(u64, InFlightAttempt)>>` when the attempt's guard drops (:71-72), and nothing prunes that list. Each entry is small, but in a long-running `roko serve`, or a large plan, the list grows with every attempt ever run.

## Why it matters

Hygiene (epic spec-9a3131): unbounded growth in a long-lived process. p3, because each entry is small.

## Where

`InFlightTasks` and its drop guard in `sibling_settle.rs`.

## Plan

1. Drop ended attempts once no sibling can still need them, for example when every attempt that overlapped them has ended. Or keep only a bounded window.
2. Add `in_flight_tasks_forget_settled_attempts`.

## Done when

- [ ] The ended list stays bounded however many attempts run.
- [ ] The `[[verify]]` command passes.

## Notes

- **wk-tamper (2026-09-30):** Implemented on `work/gap-6d172d` at `1db85d74a`; cargo verification deferred to the
  batch check. `InFlightTasks` tracks the windows `mark()` opens and `release()` closes. Each attempt end forgets the
  ended attempts no open window overlaps, then keeps at most `MAX_ENDED` (4096), the last to end. `diff_snapshot`
  releases a task's window when its base is forgotten (the task passed) or replaced, or when the snapshot fails.
  The title overstates it: production builds one dispatcher per `run_graph_plan` call, so the list lived for a run,
  not the process. A task that never passes keeps its window open, which is what the cap is for.
