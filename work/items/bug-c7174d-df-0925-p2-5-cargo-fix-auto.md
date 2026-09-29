+++
id = "bug-c7174d"
kind = "bug"
title = "DF-0925 P2-5: `cargo fix` auto-fix is unscoped and has no timeout"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-cli/gate_dispatch"]
created = 2026-09-25
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P2-5. `cargo fix` is unscoped and untimed"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P2-5. `cargo fix` is unscoped and untimed"
anchors = ["gate_dispatch.rs:557 attempt_auto_fix", "gate_dispatch.rs:590", "gate_dispatch.rs:566", "crates/roko-cli/src/runner/gate_dispatch.rs::attempt_auto_fix", "crates/roko-cli/src/runner/gate_dispatch.rs::AutoFixBounds"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-09-28
commit = "725f21e05"
by = "triage check 2026-09-28"
evidence = "Fixed in 725f21e05: attempt_auto_fix (runner/gate_dispatch.rs:629) now takes AutoFixBounds (:530-555). It scopes cargo fix/clippy --fix with -p to the packages owning the task files and skips the fix when there are none (:651-667). It kills the command after bounds.timeout, which comes from timeouts.gate_compile_secs (run_fix_command :594, used at :694/:734). It takes the compile permit with gates.compile_concurrency instead of the literals 1/300s (:669-677). Remaining nit outside the cargo claim: the npm/go fallbacks still run over the whole workdir (eslint --fix ., gofmt -w .), though they are now timed. (Static check against 3d0ee4d02; tests not re-run.)"
+++
attempt_auto_fix runs cargo fix --allow-dirty in the workdir without -p scoping or timeout, so one task can rewrite untouched crates or hang; its semaphore uses literals 1/300s instead of gates.compile_concurrency.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P2-5. `cargo fix` is unscoped and untimed`

How to verify: Read attempt_auto_fix command construction.

Fixed in 725f21e05 (checked 2026-09-28).
