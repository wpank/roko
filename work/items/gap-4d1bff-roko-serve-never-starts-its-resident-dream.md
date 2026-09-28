+++
id = "gap-4d1bff"
kind = "gap"
title = "roko serve never starts its resident dream scheduler"
status = "open"
triage = "verified"
severity = "p2"
subsystem = ["roko-serve/dreams"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-serve/src/dreams.rs::ResidentDreamScheduler", "crates/roko-serve/src/dreams.rs::run_scheduled_dream_cycle"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[repro]]
command = 'grep -rn "dreams::" crates/roko-serve/src --include="*.rs" | grep -v "roko_dreams::" | grep -v "crates/roko-serve/src/dreams.rs" | grep -q .'
+++

`crates/roko-serve/src/dreams.rs` defines `ResidentDreamScheduler` (`:38`) and `run_scheduled_dream_cycle` (`:301`), but nothing outside the module references `dreams::`, so a running server never schedules consolidation; only manual `POST /api/dream/run` works.
Even when invoked, the scheduled path calls a bare `cycle.run()` (`:312`): no merge, decay, GC or journal line.
Fix: start the scheduler at serve startup behind config and route scheduled and manual runs through one consolidation pipeline.
