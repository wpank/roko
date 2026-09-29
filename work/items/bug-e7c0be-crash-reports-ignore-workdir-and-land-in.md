+++
id = "bug-e7c0be"
kind = "bug"
title = "Crash reports ignore --workdir and land in ./.roko unless ROKO_WORKDIR is set"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
subsystem = ["roko-cli/main"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e7-pid-registry"
anchors = ["crates/roko-cli/src/main.rs::main", "crates/roko-cli/src/main.rs::invoked_subcommand_workdir"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn crash_report_dir_uses_subcommand_workdir' crates/roko-cli/ && cargo test -p roko-cli --bin roko crash_report_dir_uses_subcommand_workdir"
+++

The panic hook installed at the top of `main` (main.rs:3240-3285) writes `.roko/crash-report.json` under `ROKO_WORKDIR`, falling back to `.`. It does not use the subcommand's `--workdir`/`--repo`, which the log file and the PID registry now honour (`invoked_subcommand_workdir`). A crash in `roko plan run --workdir X` started from another directory writes its report into that directory, creating `.roko/` there if needed.

Fix: resolve the workdir before installing the hook (or store it in a shared cell after parsing) and write the report there.
