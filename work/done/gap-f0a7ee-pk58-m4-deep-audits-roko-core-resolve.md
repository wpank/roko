+++
id = "gap-f0a7ee"
kind = "gap"
title = "PK58 M4 deep audits: roko-core: resolve the audit vault outside every workdir, and define the shared audit… (+9 more)"
status = "done"
triage = "verified"
severity = "p2"
goal = "cybernetic"
rank = 58
size = "L"
subsystem = ["roko-gate/audit"]
created = 2026-10-02
updated = 2026-10-03
last_verified = 2026-10-03
last_verified_rev = "2347ad858"
source = "tmp/backlog/2026-10-02-complete-and-wire PK58"
anchors = ["crates/roko-agent/src/claude_cli_agent.rs", "crates/roko-agent/src/claude_cli_guard.py", "crates/roko-agent/src/safety/path.rs", "crates/roko-agent/src/safety/sandbox.rs", "crates/roko-cli/src/graph_task_dispatch/attempt.rs", "crates/roko-cli/src/graph_task_dispatch/red_flags.rs", "crates/roko-core/src/config/config_fingerprint_golden.json", "crates/roko-core/src/config/env_registry.rs", "crates/roko-core/src/config/mod.rs", "crates/roko-core/src/config/schema.rs", "crates/roko-core/src/lib.rs", "crates/roko-gate/Cargo.toml", "crates/roko-gate/src/attempt_diff.rs", "crates/roko-gate/src/lib.rs", "crates/roko-std/src/tool/builtin/sandbox/reads.rs", "crates/roko-std/src/tool/builtin/sandbox/secret_read_cases.txt"]
lane = "rust-hot"
parent = "spec-c3abc8"
links = { depends_on = ["gap-c06ff3", "gap-dff960"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'ROKO_AUDIT_HOME' crates/roko-core/src/audit_home.rs && grep -qw 'fn audit_vault_refuses_a_home_inside_the_workdir' crates/roko-core/src/audit_home.rs && cargo test -p roko-core --lib audit_vault_refuses_a_home_inside_the_workdir"

[[verify]]
command = "grep -qw 'fn audit_config_locks_the_floor' crates/roko-core/src/config/audit.rs && cargo test -p roko-core --lib audit_config_locks_the_floor"

[[verify]]
command = "grep -rqw 'fn lottery_matches_the_python_fixtures' crates/roko-gate/src/audit/ && cargo test -p roko-gate --lib lottery_matches_the_python_fixtures"

[[verify]]
command = "grep -rqw 'fn ledger_chain_verifies_after_10k_appends_and_catches_a_flipped_byte' crates/roko-gate/src/audit/ && cargo test -p roko-gate --lib ledger_chain_verifies_after_10k_appends_and_catches_a_flipped_byte"

[[verify]]
command = "grep -rqw 'fn hidden_suite_lifecycle_rejects_illegal_transitions' crates/roko-gate/src/audit/ && cargo test -p roko-gate --lib hidden_suite_lifecycle_rejects_illegal_transitions"

[[verify]]
command = "grep -rqw 'fn a_canary_in_an_episode_line_exposes_its_suite' crates/roko-gate/src/audit/ && cargo test -p roko-gate --lib a_canary_in_an_episode_line_exposes_its_suite"

[[verify]]
command = "grep -qw 'fn file_tools_refuse_the_audit_vault' crates/roko-agent/src/safety/sandbox.rs && cargo test -p roko-agent --lib file_tools_refuse_the_audit_vault"

[[verify]]
command = "grep -rqw 'fn bash_reads_of_the_audit_vault_are_refused' crates/roko-std/src/ && grep -qw 'fn claude_settings_deny_the_audit_vault' crates/roko-agent/src/claude_cli_agent.rs && cargo test -p roko-std --lib bash_reads_of_the_audit_vault_are_refused && cargo test -p roko-agent --lib claude_settings_deny_the_audit_vault"

[[verify]]
command = "grep -qw 'fn audit_only_kinds_find_test_detection_and_success_strings' crates/roko-gate/src/attempt_diff.rs && cargo test -p roko-gate --lib audit_only_kinds_find_test_detection_and_success_strings"

[[verify]]
command = "grep -rqw 'fn every_green_attempt_draws_one_audit_selection' crates/roko-cli/src/ && cargo test -p roko-cli --lib every_green_attempt_draws_one_audit_selection"

[closed]
at = 2026-10-03
at_ts = "2026-10-03T01:24:15Z"
commit = "2347ad858"
executor = "claude-agent"
via = "work-batch"
size = "L"
claimed_at = "2026-10-02T22:19:30Z"
forced = false
evidence = "Gate 5b (merged into main as 2347ad858, tree identical to work/backlog-batch-5b apart from work/): cargo check --workspace --tests, cargo clippy --workspace -D warnings, nextest --lib 11,533 passed over 10 crates (after the [audit] schema fix, roko-core 1,999/1,999), roko-cli bin + golden-path canaries + learning_wiring_census + plan_spec_gate 442/442, ViabilityBench suite 582 passed with the analysis lock; every [[verify]] passes."
+++

## Problem

This package delivers 10 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK58, slice 71xx, phase 7), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 7112 | S | p2 | roko-core: resolve the audit vault outside every workdir, and define the shared audit types | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7112-roko-core-audit-vault-resolver-and-types.md` |
| 2 | 7113 | S | p2 | roko-core: the [audit] config section | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7113-roko-core-audit-config-section.md` |
| 3 | 7114 | M | p2 | roko-gate audit module: the lottery policy, the HMAC draw, key commit and reveal, and the estimators (S05 task 4) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7114-roko-gate-audit-policy-draw-and-estimators.md` |
| 4 | 7115 | S | p2 | roko-gate audit ledger: a SHA-256 hash chain in the vault and a redacted workspace mirror | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7115-roko-gate-audit-ledger-hash-chain.md` |
| 5 | 7116 | M | p2 | roko-gate: the hidden-suite store and its lifecycle in the vault (S05 task 5, store half) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7116-roko-gate-hidden-suite-store-and-lifecycle.md` |
| 6 | 7117 | S | p2 | roko-gate: a canary scanner that exposes a suite whose canary appears in a prompt, output, episode, knowledge entry or diff (S05 task 5, scanner half) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7117-roko-gate-canary-scanner.md` |
| 7 | 7118 | S | p2 | Roko's own file tools refuse the audit vault by its canonical path (S05 task 6, first half) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7118-roko-file-tools-refuse-the-audit-vault.md` |
| 8 | 7119 | S | p2 | Shell reads and Claude CLI tools refuse the audit vault (S05 task 6, second half) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7119-shell-and-claude-cli-refuse-the-audit-vault.md` |
| 9 | 7120 | S | p2 | Audit-only A1 kinds: test detection in product code, printed success strings and vacuous diffs | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7120-audit-only-a1-kinds.md` |
| 10 | 7121 | M | p2 | DP1: draw the audit lottery for every green attempt at settle, with the run key committed at open and revealed at close (S05 task 7) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7121-dp1-audit-lottery-draw-at-settle.md` |

## Why it matters

Phase 7: M4 random deep audits (S05). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7100-m4-random-deep-audits.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-agent/src/claude_cli_agent.rs`, `crates/roko-agent/src/claude_cli_guard.py`, `crates/roko-agent/src/safety/path.rs`, `crates/roko-agent/src/safety/sandbox.rs`, `crates/roko-cli/src/graph_task_dispatch.rs`, `crates/roko-cli/src/graph_task_dispatch/attempt.rs`, `crates/roko-cli/src/graph_task_dispatch/audit_select.rs`, `crates/roko-cli/src/graph_task_dispatch/red_flags.rs`, `crates/roko-core/src/audit_home.rs`, `crates/roko-core/src/audit_types.rs`, `crates/roko-core/src/config/audit.rs`, `crates/roko-core/src/config/config_fingerprint_golden.json`, `crates/roko-core/src/config/env_registry.rs`, `crates/roko-core/src/config/mod.rs`, `crates/roko-core/src/config/schema.rs`, `crates/roko-core/src/lib.rs`, `crates/roko-gate/Cargo.toml`, `crates/roko-gate/src/attempt_diff.rs`, `crates/roko-gate/src/audit/canary.rs`, `crates/roko-gate/src/audit/estimate.rs`, `crates/roko-gate/src/audit/hidden.rs`, `crates/roko-gate/src/audit/ledger.rs`, `crates/roko-gate/src/audit/mod.rs`, `crates/roko-gate/src/audit/policy.rs`, `crates/roko-gate/src/lib.rs`, `crates/roko-std/src/tool/builtin/sandbox/reads.rs`, `crates/roko-std/src/tool/builtin/sandbox/secret_read_cases.txt`.

It also edits the hot file(s) `crates/roko-cli/src/graph_task_dispatch.rs`, which are left out of this item's anchors so that two hot packages can run at once; the coordinator resolves any merge conflict.

## Current state

The tasks were checked against `2c3ea9f73` on 2026-10-02. Re-check each task's anchors and premise at your base commit before implementing it, and report a task that is already done instead of redoing it.

## Plan

1. Work through the tasks in the order above. For each: read its file, implement its Plan, write the test it names, and make one commit per task whose message ends with `Backlog-Task: <task id>`, `Work-Item: <this item's id>` and `Executor: claude-agent`.
2. Follow `BUILD-RULES.md` in the backlog folder. Workers run no cargo: Rust is checked by the coordinator's batched gate. Python and doc checks you may run.
3. If a task cannot be done (a premise is false, a decision is missing, or its verify cannot pass), stop at that task, keep the earlier commits, and report it; do not skip ahead to tasks that depend on it.

## Done when

- Every task's verify command passes (this item's `[[verify]]` list, one entry per task), after the coordinator's batched gate.
- Each task's own "Done when" holds (see its file).

## Notes

- Waits on: PK28 (gap-c06ff3), PK53 (gap-dff960).
- Suggested model: opus.

## Progress

All on `work/gap-f0a7ee`; cargo verification deferred to the batch gate.

- 7112: implemented at 1a6362709
- 7113: implemented at cff1dd068 (the fingerprint golden vectors hash fixed inputs, so they did not change)
- 7114: implemented at 8aa1370fa (Wilson in every cell, per gap-a499aa; Cargo.lock's roko-gate entry gains hmac and sha2 by hand)
- 7115: implemented at 149cbd885
- 7116: implemented at 13bb70c1c
- 7117: implemented at e7dcf8ee9
- 7118: implemented at 6b5cd93c3
- 7119: implemented at 56d1b8f4f, except plan step 4 (a note on what Codex's broker lets an agent read outside the workspace), which is not written; the Python guard passes all 185 table rows run by hand
- 7120: implemented at 4dc29e730
- 7121: implemented at b165ea489 and 4cee12746; the settle-time canary scan covers the agent's output only, since the composed prompt and the diff's added lines do not reach settle
