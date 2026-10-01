+++
id = "bug-951930"
kind = "bug"
title = "Post-auto-fix verify re-run does not take the compile lock"
status = "done"
triage = "verified"
severity = "p3"
goal = "core"
subsystem = ["roko-cli/graph-dispatch"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:w4a-reverify"
anchors = ["crates/roko-cli/src/graph_task_dispatch/verification.rs::GraphTaskDispatcher::settle_task_verification", "crates/roko-cli/src/graph_task_dispatch/verification.rs::verify_compile_permit", "crates/roko-cli/src/runner/gate_dispatch.rs::acquire_compile_ownership"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'grep -rq "fn post_fix_rerun_takes_compile_lock" crates/roko-cli/src && cargo test -p roko-cli --lib post_fix_rerun_takes_compile_lock'

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:23Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:11:58Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

Graph verify steps normally acquire compile ownership (`acquire_compile_ownership`, called at graph_task_dispatch.rs:3985) so concurrent tasks and plans do not build at once. After a successful auto-fix, the dispatcher re-runs the task's verify steps inline (graph_task_dispatch.rs:2069-2135, `retry_gate.verify`) without taking that lock, so the re-run can overlap other builds and contend for the target directory. This predates the sibling-settle work.

Fix: route the re-run through the same locked verify helper. Add a test named `post_fix_rerun_takes_compile_lock`.

2026-09-29: re-verified at d9e79e9d8. Still open. The locked helper is now verify_compile_permit (graph_task_dispatch.rs:3968), used at lines 1895 and 1923; the post-fix re-run ShellGate at line 2104 still skips it.

## Notes

2026-10-01 (wk-gates): implemented on work/bug-951930; cargo verification deferred to the batch check.
The post-auto-fix re-run in `GraphTaskDispatcher::run_verify_steps` now runs each step through the new
`verify_step_locked` (verification.rs), which holds `verify_compile_permit`'s lock while the gate runs and releases it
before the baseline judge takes its own. The test `post_fix_rerun_takes_compile_lock` holds the lock and checks that
the step waits for it.
