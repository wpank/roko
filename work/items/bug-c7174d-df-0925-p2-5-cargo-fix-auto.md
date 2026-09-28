+++
id = "bug-c7174d"
kind = "bug"
title = "DF-0925 P2-5: `cargo fix` auto-fix is unscoped and has no timeout"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/gate_dispatch"]
created = 2026-09-25
updated = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P2-5. `cargo fix` is unscoped and untimed"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P2-5. `cargo fix` is unscoped and untimed"
anchors = ["gate_dispatch.rs:557 attempt_auto_fix", "gate_dispatch.rs:590", "gate_dispatch.rs:566"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
attempt_auto_fix runs cargo fix --allow-dirty in the workdir without -p scoping or timeout, so one task can rewrite untouched crates or hang; its semaphore uses literals 1/300s instead of gates.compile_concurrency.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P2-5. `cargo fix` is unscoped and untimed`

How to verify: Read attempt_auto_fix command construction.
