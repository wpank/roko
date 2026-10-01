+++
id = "bug-fade34"
kind = "bug"
title = "`knowledge query` results dump full hypothesis text"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
subsystem = ["roko-cli/knowledge"]
created = 2026-09-18
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "tmp/dogfood/2026-09-18-session.md#ISSUE-18: `roko knowledge query` results are verbose/noisy"
discovered_from = "audit:tmp/dogfood/2026-09-18-session.md#ISSUE-18: `roko knowledge query` results are verbose/noisy"
anchors = ["crates/roko-cli/src/commands/knowledge.rs::cmd_neuro", "crates/roko-cli/src/main.rs:1171"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -n -A12 'Query the durable knowledge store' crates/roko-cli/src/main.rs | grep -q verbose && grep -qw 'fn knowledge_query_previews_two_lines_unless_verbose' crates/roko-cli/src/commands/knowledge.rs && cargo test -p roko-cli --bin roko knowledge_query_previews_two_lines_unless_verbose"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:30Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:13:21Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++
Results print full strategy hypothesis text; proposal: truncate to ~2 lines with --verbose for full output.

Imported without verification from:
- `tmp/dogfood/2026-09-18-session.md#ISSUE-18: `roko knowledge query` results are verbose/noisy`

How to verify: Run a knowledge query and inspect output width.

Verified 2026-09-29 at d9e79e9d8. The text-mode output of roko knowledge query (crates/roko-cli/src/commands/knowledge.rs, cmd_neuro, NeuroCmd::Query arm) prints the full entry.content for every result, and KnowledgeCmd::Query in main.rs has no --verbose flag. On the same path, the declared --limit flag is ignored: dispatch_knowledge hardcodes limit: 10 and cmd_neuro calls store.query(&topic, 10).

## Notes

- 2026-10-01 (wk-climain): implemented on work/bug-28c193; cargo verification deferred to the batch check.
  `roko knowledge query` prints each match's first two non-empty lines, each cut at 160 characters, and a count of
  the lines left out (`query_entry_text` in `commands/knowledge.rs`); `knowledge query --verbose` prints entries in
  full. The declared `--limit` now reaches `store.query` (it was hardcoded to 10). Test:
  `knowledge_query_previews_two_lines_unless_verbose`.
- The `[[verify]]` command changed: its second grep looked for `NeuroCmd::Query { topic, workdir` on one line, but
  with four bound fields rustfmt puts the pattern on several lines (struct patterns over 18 columns go vertical).
  It now checks for the test and runs it. Like `roko diagnose --verbose`, the subcommand's `--verbose` shadows the
  global one, so the global `-v` short form is not accepted after `query`.
