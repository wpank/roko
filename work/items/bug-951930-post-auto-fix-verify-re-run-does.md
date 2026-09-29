+++
id = "bug-951930"
kind = "bug"
title = "Post-auto-fix verify re-run does not take the compile lock"
status = "open"
triage = "verified"
severity = "p3"
goal = "core"
subsystem = ["roko-cli/graph-dispatch"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:w4a-reverify"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs:2104", "crates/roko-cli/src/graph_task_dispatch.rs::verify_compile_permit", "crates/roko-cli/src/runner/gate_dispatch.rs::acquire_compile_ownership"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'grep -rq "fn post_fix_rerun_takes_compile_lock" crates/roko-cli/src && cargo test -p roko-cli --lib post_fix_rerun_takes_compile_lock'
+++

Graph verify steps normally acquire compile ownership (`acquire_compile_ownership`, called at graph_task_dispatch.rs:3985) so concurrent tasks and plans do not build at once. After a successful auto-fix, the dispatcher re-runs the task's verify steps inline (graph_task_dispatch.rs:2069-2135, `retry_gate.verify`) without taking that lock, so the re-run can overlap other builds and contend for the target directory. This predates the sibling-settle work.

Fix: route the re-run through the same locked verify helper. Add a test named `post_fix_rerun_takes_compile_lock`.

2026-09-29: re-verified at d9e79e9d8. Still open. The locked helper is now verify_compile_permit (graph_task_dispatch.rs:3968), used at lines 1895 and 1923; the post-fix re-run ShellGate at line 2104 still skips it.
