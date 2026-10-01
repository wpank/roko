+++
id = "bug-e7c0be"
kind = "bug"
title = "Crash reports ignore --workdir and land in ./.roko unless ROKO_WORKDIR is set"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
subsystem = ["roko-cli/main"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e7-pid-registry"
anchors = ["crates/roko-cli/src/main.rs::main", "crates/roko-cli/src/main.rs::invoked_subcommand_workdir"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn crash_report_dir_uses_subcommand_workdir' crates/roko-cli/ && cargo test -p roko-cli --bin roko crash_report_dir_uses_subcommand_workdir"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:29Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:13:21Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

The panic hook installed at the top of `main` (main.rs:3240-3285) writes `.roko/crash-report.json` under `ROKO_WORKDIR`, falling back to `.`. It does not use the subcommand's `--workdir`/`--repo`, which the log file and the PID registry now honour (`invoked_subcommand_workdir`). A crash in `roko plan run --workdir X` started from another directory writes its report into that directory, creating `.roko/` there if needed.

Fix: resolve the workdir before installing the hook (or store it in a shared cell after parsing) and write the report there.

## Notes

- 2026-10-01 (wk-climain): implemented on work/bug-28c193; cargo verification deferred to the batch check.
  `main` records the workdir it computes for the log file and the PID registry in a `CRASH_REPORT_WORKDIR` cell, and
  the panic hook writes to `crash_report_dir(...)`: that workdir's `.roko/`, else `ROKO_WORKDIR`'s (a panic before
  parsing), else `./.roko`. Test: `crash_report_dir_uses_subcommand_workdir`.
