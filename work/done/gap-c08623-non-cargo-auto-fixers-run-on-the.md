+++
id = "gap-c08623"
kind = "gap"
title = "Non-Cargo auto-fixers run on the whole tree instead of the task's files"
status = "done"
triage = "verified"
severity = "p3"
goal = "core"
subsystem = ["roko-cli/gate_dispatch"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e4-perf"
anchors = ["crates/roko-cli/src/runner/gate_dispatch.rs::attempt_auto_fix"]
links = { depends_on = [], blocks = [], related = ["bug-c7174d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = '! grep -q "eslint --fix \.\"" crates/roko-cli/src/runner/gate_dispatch.rs'

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:49Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:11:58Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

bug-c7174d scoped the Cargo auto-fix with `-p` and bounded it in time. The JS/TS, Go and Python fixers in `attempt_auto_fix` are time-bounded too, but still run on the whole tree (`npx eslint --fix .`, `gofmt -w .`, `ruff --fix .` falling back to `black .`). A gate failure in one task can therefore reformat files that belong to other tasks, or to other plans running beside it.

Fix: pass the task's changed or declared files to these fixers.

## Notes

2026-10-01 (wk-gates): implemented on work/bug-951930; cargo verification deferred to the batch check.
`attempt_auto_fix` now passes the eslint, gofmt, ruff and black fixers the task's own files in their language
(`task_fix_targets`: regular files inside the workdir, with no absolute or `..` paths and none starting with `-`).
When the task has no such file, the fix is skipped, as the Cargo path does when no package owns the task files.
Tests: `non_cargo_fix_targets_are_the_task_files_in_the_language` and
`non_cargo_auto_fix_skips_a_task_without_files_in_its_language`.
