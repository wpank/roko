+++
id = "bug-9fc3fc"
kind = "bug"
title = "roko backlog audit reads the removed engine's state and ignores Graph checkpoints"
status = "superseded"
triage = "verified"
severity = "p2"
goal = "tooling"
subsystem = ["roko-cli/backlog"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "tmp/work-management/03-roko-native-capabilities.md"
discovered_from = "doc:tmp/work-management/03-roko-native-capabilities.md"
anchors = ["crates/roko-cli/src/commands/backlog.rs::read_toml_plan_statuses", "crates/roko-cli/src/commands/backlog.rs::read_executor_plan_phases"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "gap-759041" }

[[verify]]
command = "grep -rqw 'fn audit_reports_graph_completed_plan_left_ready' crates/roko-cli/ && cargo test -p roko-cli --bin roko audit_reports_graph_completed_plan_left_ready"

[closed]
at = 2026-09-29
by = "work sweep 2026-09-29 (static check against HEAD; cargo verify not re-run while the portal plan run held the build lock)"
evidence = "Duplicate of gap-759041 (older item; keep that ID). Still true at HEAD: backlog.rs (unchanged since 244f564e1) reads only state-snapshot.json/executor.json (backlog.rs:314-380), never the Graph checkpoint, and read_toml_plan_statuses (backlog.rs:278-308) scans one level and silently `continue`s on parse failures. This is the same defect as the older gap-759041 (created 2026-09-07), whose 2026-09-28 verification narrowed it to exactly this. Plan 02-plan-closes T04 closes both."
+++

`backlog audit` compares plans on disk with the legacy `state-snapshot.json` (`commands/backlog.rs`), which the Graph engine no longer writes; it never reads `.roko/state/graph/<plan>/checkpoint.json`.
It also scans one directory level (nested `tasks.toml` files are invisible) and silently skips plans without `[meta] plan`; a local run reported 0 plans in the executor and 0 drift for 13 plans.
Fix: read Graph checkpoints, recurse like `plan validate`, and report skipped plans.

2026-09-29: this is the same defect as gap-759041 (backlog audit ignores Graph checkpoints). Its extra criteria are recorded on gap-759041: recurse into nested tasks.toml, and report plans skipped for parse failure or a missing [meta] plan.
