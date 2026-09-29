+++
id = "gap-4d835d"
kind = "gap"
title = "Graph engine runs a plan wave by wave, so a ready task waits for its whole wave"
status = "open"
triage = "verified"
severity = "p1"
goal = "core"
subsystem = ["roko-graph/engine"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md#10-05"
discovered_from = "plan:portal-programme/08f-final-polish#T04"
anchors = ["crates/roko-graph/src/engine.rs::topological_waves", "crates/roko-graph/src/engine.rs:983", "crates/roko-graph/src/engine.rs:2152"]
links = { depends_on = [], blocks = [], related = ["find-43768e"], supersedes = [], duplicate_of = "" }

[[repro]]
command = "! grep -q 'for wave in &waves' crates/roko-graph/src/engine.rs"

[[verify]]
command = "grep -q 'fn a_ready_node_does_not_wait_for_its_wave' crates/roko-graph/src/engine.rs && cargo test -p roko-graph --lib a_ready_node_does_not_wait_for_its_wave"
+++

The engine computes `topological_waves` and runs them in order (`for wave in &waves` at
`engine.rs:983`, and the `'wave_loop` at `:2152`): each wave's `JoinSet` is drained before the
next wave starts. A node whose own dependencies are done still waits for every other node in the
previous wave, so a plan's wall-clock time is the sum of each wave's slowest node, not its
longest dependency chain.

Seen on 2026-09-29 in `08f-final-polish` (max_parallel 4). T01, T02, T03 and T05 formed wave 1,
and T04 (depends on T02 and T03) formed wave 2. T02 and T03 finished by about 09:24. T05 then
spent about 30 minutes in two 600-second agent timeouts plus a failing attempt. T04 sat ready
and idle the whole time. When T05 exhausted its retries, the run failed and T04 never ran at
all, though it did not depend on T05.

A fix must dispatch each node as soon as all its dependencies have completed, still within the
semaphore and admission limits. It must keep conditional routing, failure propagation to
dependants only, checkpoint/resume, and the per-node events. A failed node should block only
its own dependants, and the rest of the plan should run to completion before the plan reports
failure.
