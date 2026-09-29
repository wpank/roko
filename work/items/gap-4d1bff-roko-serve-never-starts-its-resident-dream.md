+++
id = "gap-4d1bff"
kind = "gap"
title = "roko serve never starts its resident dream scheduler"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
hold = "Set aside per tldr/05 §3; Will chose hold over park on 2026-09-29 (dec-e70592). Remove this line to revive."
subsystem = ["roko-serve/dreams"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-serve/src/dreams.rs::start_dream_loop", "crates/roko-serve/src/dreams.rs::ResidentDreamScheduler", "crates/roko-serve/src/dreams.rs::run_scheduled_dream_cycle", "crates/roko-cli/src/daemon.rs:471"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[repro]]
command = 'grep -rn "dreams::" crates/roko-serve/src --include="*.rs" | grep -v "roko_dreams::" | grep -v "crates/roko-serve/src/dreams.rs" | grep -q .'

[[verify]]
command = "grep -qn 'start_dream_loop' crates/roko-serve/src/lib.rs crates/roko-cli/src/commands/server.rs (plus a test the fix must add, e.g. cargo test -p roko-serve serve_starts_resident_dream_loop_when_enabled)"
+++

`crates/roko-serve/src/dreams.rs` defines `ResidentDreamScheduler` (`:38`) and `run_scheduled_dream_cycle` (`:301`), but nothing outside the module references `dreams::`, so a running server never schedules consolidation; only manual `POST /api/dream/run` works.
Even when invoked, the scheduled path calls a bare `cycle.run()` (`:312`): no merge, decay, GC or journal line.
Fix: start the scheduler at serve startup behind config and route scheduled and manual runs through one consolidation pipeline.

Re-checked 2026-09-29: still true for roko serve. Correction: the resident loop is not dead code; roko daemon start calls roko_serve::dreams::start_dream_loop (crates/roko-cli/src/daemon.rs:471), but the roko serve startup path (ServerBuilder, crates/roko-serve/src/lib.rs around :446-468) does not. run_scheduled_dream_cycle still calls a bare cycle.run() (crates/roko-serve/src/dreams.rs:312). The [[repro]] only greps crates/roko-serve/src, so it would keep failing if the fix were wired from crates/roko-cli/src/commands/server.rs.
