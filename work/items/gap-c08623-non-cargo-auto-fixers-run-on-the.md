+++
id = "gap-c08623"
kind = "gap"
title = "Non-Cargo auto-fixers run on the whole tree instead of the task's files"
status = "open"
triage = "verified"
severity = "p3"
subsystem = ["roko-cli/gate_dispatch"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e4-perf"
anchors = ["crates/roko-cli/src/runner/gate_dispatch.rs::attempt_auto_fix"]
links = { depends_on = [], blocks = [], related = ["bug-c7174d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = '! grep -q "eslint --fix \.\"" crates/roko-cli/src/runner/gate_dispatch.rs'
+++

bug-c7174d scoped the Cargo auto-fix with `-p` and bounded it in time. The JS/TS, Go and Python fixers in `attempt_auto_fix` are time-bounded too, but still run on the whole tree (`npx eslint --fix .`, `gofmt -w .`, `ruff --fix .` falling back to `black .`). A gate failure in one task can therefore reformat files that belong to other tasks, or to other plans running beside it.

Fix: pass the task's changed or declared files to these fixers.
