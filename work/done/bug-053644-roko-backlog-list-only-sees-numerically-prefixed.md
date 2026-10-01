+++
id = "bug-053644"
kind = "bug"
title = "roko backlog list only sees numerically prefixed files in a hard-coded directory"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
subsystem = ["roko-cli/backlog"]
created = 2026-09-28
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "tmp/work-management/03-roko-native-capabilities.md"
discovered_from = "doc:tmp/work-management/03-roko-native-capabilities.md"
anchors = ["crates/roko-cli/src/plan_generate.rs::DEFAULT_BACKLOG_DIR", "crates/roko-cli/src/commands/backlog.rs::cmd_backlog_list", "crates/roko-cli/src/commands/backlog.rs::has_imported_idea"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqE 'fn backlog_list_[A-Za-z0-9_]*' crates/roko-cli/ && cargo test -p roko-cli --bin roko backlog_list_"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:08Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:12:51Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

`backlog list` reads `DEFAULT_BACKLOG_DIR = "tmp/backlog"` (`plan_generate.rs:597`); per a local audit it accepts only files whose prefix parses as a `u32`, lists a status summary as item `#0`, ignores archive directories, never shows the `**Status**:` line that `mark-done` writes, and checks for imports in a path the CLI does not write.
Fix: configurable source directories, a tolerant id grammar, and status/import detection that matches what the other backlog commands write.

Verified 2026-09-28 (static check against 3d0ee4d02): cmd_backlog_list (crates/roko-cli/src/commands/backlog.rs:57-104) hard-codes workdir.join("tmp/backlog") (:58; DEFAULT_BACKLOG_DIR itself now at plan_generate.rs:651), reads only top-level *.md whose first '-' segment parses as u32 (:65-80), skips only 00-INDEX so 00-STATUS-SUMMARY.md lists as #0, never descends into archive/, and prints only 'imported'/'-' (never the **Status**: line mark-done writes at :819). has_imported_idea reads .roko/prd/ideas/ideas.md (:85,:107-113) while import writes through prd::cmd_idea to .roko/prd/ideas.md (workspace_paths.rs:29-30), so imports are never detected. No commit or uncommitted edit touches backlog.rs.

Re-verified 2026-09-29: unchanged. The anchor plan_generate.rs:597 is stale: DEFAULT_BACKLOG_DIR is at plan_generate.rs:651, and cmd_backlog_list does not use it (it hard-codes tmp/backlog at backlog.rs:58).

## Notes

- 2026-10-01 (wk-taskdef): implemented on work/bug-e3df7d; cargo verification deferred to the batch check.
- `backlog list [<path>]` reads `DEFAULT_BACKLOG_DIR` (or the given directory) and its `archive/`. An id may have leading
  zeros and a `-`, `_` or `.` before the slug; `00-` index and summary files are not specs. Import and `mark-done`'s
  lookup share that grammar. Each row shows the spec's `**Status**:` line, and imports are read from
  `.roko/prd/ideas.md`, where `backlog import` writes them. main.rs gains one optional `path` on `BacklogCmd::List`.
