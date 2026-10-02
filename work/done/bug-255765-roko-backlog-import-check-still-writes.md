+++
id = "bug-255765"
kind = "bug"
title = "roko backlog import --check still writes"
status = "done"
triage = "verified"
severity = "p2"
goal = "tooling"
subsystem = ["roko-cli/backlog"]
created = 2026-09-28
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "tmp/work-management/03-roko-native-capabilities.md"
discovered_from = "doc:tmp/work-management/03-roko-native-capabilities.md"
anchors = ["crates/roko-cli/src/commands/backlog.rs::cmd_backlog", "crates/roko-cli/src/commands/backlog.rs::cmd_backlog_import", "crates/roko-cli/src/main.rs:1739"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn backlog_import_check_writes_nothing' crates/roko-cli/ && cargo test -p roko-cli --bin roko backlog_import_check_writes_nothing"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:10Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:12:51Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

`--check` is declared as a dry run without side effects (`main.rs:1731-1733`), but the dispatcher destructures the import arguments with `..` (`commands/backlog.rs:28`) and never passes the flag on, so `backlog import --check` performs the import.
Fix: thread `check` into the importer, print the planned changes without writing, and test that no files change.

Re-verified 2026-09-29: unchanged. The --check flag declaration moved to main.rs:1739-1741.

## Notes

- 2026-10-01 (wk-taskdef): implemented on work/bug-e3df7d; cargo verification deferred to the batch check.
- The dispatcher passes `check` to `cmd_backlog_import`, which then prints each idea it would record (marking specs
  already imported) and returns before writing. `backlog_import_check_writes_nothing` snapshots the workspace before
  and after.
