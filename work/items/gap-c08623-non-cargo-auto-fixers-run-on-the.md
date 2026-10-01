+++
id = "gap-c08623"
kind = "gap"
title = "Non-Cargo auto-fixers run on the whole tree instead of the task's files"
status = "open"
triage = "verified"
severity = "p3"
goal = "core"
subsystem = ["roko-cli/gate_dispatch"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e4-perf"
anchors = ["crates/roko-cli/src/runner/gate_dispatch.rs::attempt_auto_fix"]
links = { depends_on = [], blocks = [], related = ["bug-c7174d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = '! grep -q "eslint --fix \.\"" crates/roko-cli/src/runner/gate_dispatch.rs'
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
