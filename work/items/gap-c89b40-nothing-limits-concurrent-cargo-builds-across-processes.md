+++
id = "gap-c89b40"
kind = "gap"
title = "Nothing limits concurrent cargo builds across processes that share a target dir"
status = "open"
triage = "verified"
severity = "p2"
goal = "tooling"
subsystem = ["roko-cli/gates"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "tmp/cybernetic-harness/assessment-2026-09-28/process-capacity.md"
discovered_from = "audit:tmp/cybernetic-harness/assessment-2026-09-28/process-capacity.md"
anchors = ["crates/roko-cli/src/runner/gate_dispatch.rs::compile_coordinator", "crates/roko-cli/src/runner/gate_dispatch.rs::acquire_compile_ownership", "crates/roko-cli/src/graph_task_dispatch/verification.rs::verify_compile_permit"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "sed -n '/fn acquire_compile_ownership/,/^}/p' crates/roko-cli/src/runner/gate_dispatch.rs | grep -qE 'lock_exclusive|FileExt|build_slot'"
+++
roko's compile permits (`gates.compile_concurrency`, `gate_dispatch.rs:538`) only coordinate tasks inside one roko process. Parallel plan runs, other sessions and developer builds that share the cargo target dir queue on cargo's build lock instead. Those waits count against verify timeouts, which produces false task failures, retries and wasted spend.

The portal session alone ran 6–7 Rust agents at once, against an advised maximum of 5 per target dir. Free disk dropped to about 41 GiB when a second target dir was created.

Fix: a cross-process build slot (a lock file with a counter, or a small daemon), with verify timeouts that start only once the slot is acquired.
