+++
id = "bug-415db3"
kind = "bug"
title = "DF-0925 P1-1: TUI exits after the first plan of a multi-plan run and the terminal goes silent"
status = "done"
triage = "verified"
severity = "p1"
subsystem = ["roko-cli/tui"]
created = 2026-09-25
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P1-1. The TUI exits after the first plan, then the terminal goes silent"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P1-1. The TUI exits after the first plan, then the terminal goes silent"
anchors = ["crates/roko-cli/src/tui/app/channels.rs::update_plan_completion_exit", "crates/roko-cli/src/tui/app/mod.rs::App::with_exit_on_plan_completion"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'plan_set_complete()' crates/roko-cli/src/tui/app/channels.rs"

[closed]
at = 2026-09-28
commit = "91b4745f8"
evidence = "(working tree, uncommitted; not in HEAD 91b4745f8) crates/roko-cli/src/tui/app/channels.rs::update_plan_completion_exit no longer exits when plans_active == 0 between plans; it exits only once snapshot.plan_set_complete() or a run_outcome is published (tests in tui/app/tests.rs seed PlanSetLoaded)."
+++
with_exit_on_plan_completion shuts the TUI when plans_active==0, which happens between plans; inline progress is disabled and stderr is dup2'd to a log, so remaining plans run for hours with no output.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P1-1. The TUI exits after the first plan, then the terminal goes silent`

How to verify: Run plan run --tui on a two-plan directory; observe TUI exit.

Verified 2026-09-28: closed as done; see [closed].evidence.
