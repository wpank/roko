+++
id = "bug-48dc41"
kind = "bug"
title = "roko prd list and prd status rewrite tracked index files"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
subsystem = ["roko-cli/prd"]
created = 2026-09-28
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "tmp/work-management/03-roko-native-capabilities.md"
discovered_from = "doc:tmp/work-management/03-roko-native-capabilities.md"
anchors = ["crates/roko-cli/src/main.rs::finish_with_index_rebuild", "crates/roko-cli/src/main.rs:3829", "crates/roko-cli/src/index.rs::rebuild_all"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! sed -n '/Command::Prd { cmd } => {/,/^        }/p' crates/roko-cli/src/main.rs | grep -q 'finish_with_index_rebuild(result, &wd, true)'"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:15Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:13:21Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

Every `roko prd ...` invocation, including read-only `list` and `status`, ends in `finish_with_index_rebuild(result, &wd, true)` (`main.rs:3802`), which rebuilds all indexes and rewrites `.roko/prd/INDEX.md`, the git-tracked `plans/INDEX.md` and others.
Inspecting PRDs therefore dirties the working tree.
Fix: rebuild only after mutating subcommands (as other groups do via `should_rebuild`) and test that `prd list` leaves files untouched.

Rechecked 2026-09-29 at d9e79e9d8: still open. The unconditional rebuild call has moved to main.rs:3829. The Plan group's gating (PlanCmd::should_rebuild_indexes, main.rs:2302) is the pattern to copy for PrdCmd.

## Notes

- 2026-10-01 (wk-climain): implemented on work/bug-28c193; cargo verification deferred to the batch check.
  New `PrdCmd::should_rebuild_indexes` (false for `list`, `status`, `draft list` and `plan --dry-run`), and the
  `Command::Prd` arm passes it to `finish_with_index_rebuild`. The PRD index header now says it is rebuilt after
  mutating `roko prd` commands. Tests: `read_only_prd_commands_do_not_rebuild_indexes`,
  `mutating_prd_commands_rebuild_indexes`, `prd_list_leaves_the_index_files_untouched`. The static `[[verify]]` passes.
  CLAUDE.md's CLI table still says every `roko prd` subcommand rebuilds the indexes; left for the coordinator.
