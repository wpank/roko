+++
id = "gap-9b30ba"
kind = "gap"
title = "Batch Controller (Pause After N Plans)"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/commands"]
created = 2026-09-07
updated = 2026-09-28
source = "tmp/backlog/archive/179-batch-controller.md#179 — Batch Controller (Pause After N Plans)"
discovered_from = "audit:tmp/backlog/archive/179-batch-controller.md#179 — Batch Controller (Pause After N Plans)"
anchors = ["crates/roko-cli/src/commands/plan.rs", "event_loop.rs", "crates/roko-cli/src/runner/types.rs", "crates/roko-cli/src/runner/event_loop.rs", "crates/roko-cli/src/tui/app.rs", "RunConfig", "TuiBridge", "RunnerEvent::BatchResume"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
operator quality-of-life for long multi-plan runs; not blocking any other work. Mori's `BatchController` was a 35-line component: track a `completed_since_pause` counter, pause the event loop when it reaches `batch_size`, and resume on operator signal. This gave operators natural checkpoints…

Imported without verification from:
- `tmp/backlog/archive/179-batch-controller.md#179 — Batch Controller (Pause After N Plans)`
- `tmp/backlog/_archive/_mori-old-gaps.md#MO-09`
- `tmp/archive/CONSOLIDATED-BACKLOG-2026-09-23.md#UXP-05 (UX/TUI Parity: Partial Items (13 items f)`
- `tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#4.9 Parity items requiring runner/infras PX.3`

Some cited files are gone: `crates/roko-cli/src/runner/event_loop.rs`, `crates/roko-cli/src/tui/app.rs`.

How to verify: Check: `roko plan run plans/ --batch-size 5` pauses after every 5 plan completions.; TUI shows a batch-pause modal with progress counts.; Headless mode prints status and waits for stdin. [evidence: CONSOLIDATED UXP-05: Flag+events exist; event loop stub never triggers; 00-STATUS-SUMMARY 3. Open / P3 -- Low (Open): XS | 7 |]
