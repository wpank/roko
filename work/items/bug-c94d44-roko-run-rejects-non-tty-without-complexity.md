+++
id = "bug-c94d44"
kind = "bug"
title = "`roko run` rejects non-TTY without --complexity; e2e signal tests ignored"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/run"]
created = 2026-09-21
updated = 2026-09-28
source = "crates/roko-cli/tests/e2e.rs:88"
discovered_from = "audit:crates/roko-cli/tests/e2e.rs:88"
anchors = ["crates/roko-cli/tests/e2e.rs::init_run_produces_expected_signals", "crates/roko-cli/tests/e2e.rs::run_fails_when_gate_fails", "crates/roko-cli/tests/e2e.rs::prompt_files_are_injected_as_sections"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Three e2e tests are ignored because `roko run` now dispatches through do_cmd, which rejects non-TTY input without --complexity. Scripted/CI use of `roko run` may be broken and the init→run→signals path lacks e2e coverage.

Imported without verification from:
- `crates/roko-cli/tests/e2e.rs:88`
- `crates/roko-cli/tests/e2e.rs:319`
- `crates/roko-cli/tests/e2e.rs:372`

How to verify: echo prompt | roko run 'x' in a non-TTY shell; check do_cmd complexity prompt; add an e2e adapter or non-TTY default.
