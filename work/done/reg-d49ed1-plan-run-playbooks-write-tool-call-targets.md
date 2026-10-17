+++
id = "reg-d49ed1"
kind = "regression"
title = "Plan-run playbooks write tool-call targets unscrubbed, so a secret on a command line reaches .roko/learn/playbooks (secret_canary fails)"
status = "done"
triage = "verified"
severity = "p1"
subsystem = ["roko-learn/playbook", "roko-cli/runtime_feedback"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
last_verified_rev = "e6577bc80"
source = "release PR check 2026-10-17 at 31964059d (workspace tests)"
anchors = ["crates/roko-cli/src/runtime_feedback/verified_playbooks.rs::playbook_steps", "crates/roko-learn/src/playbook.rs::save", "crates/roko-cli/tests/secret_canary.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --test secret_canary canary_absent_from_every_file_after_plan_run"

[closed]
at = 2026-10-17
at_ts = "2026-10-17T08:02:21Z"
commit = "e6577bc80"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
claimed_at = "2026-10-17T07:14:36Z"
forced = false
evidence = "e6577bc80: roko_learn PlaybookStore::save passes the serialized playbook through roko_core::obs::scrub_secrets_in_json before writing it, so every producer (plan runs, roko bench via save_or_merge, dream cycles) is covered, and the locked rewrite behind record_outcome and relabel_success_as_failure redacts the playbook it rewrites, so a file an older roko wrote loses its secret when next credited. VerifiedPlaybookSink builds each candidate from redacted text (roko_core::obs::scrub_secrets: each step's tool and target, the goal before its whitespace is collapsed, the plan-task source of its id, the when_pattern of topic terms) and redacts the playbook_written goal, so the in-memory playbook, the event and the dedupe keys carry no registered secret and a repeat credits the stored playbook. Checked at e6577bc80 in a CI-like env (env -i, scratch HOME and TMPDIR outside the repo, CARGO_INCREMENTAL=0, SKIP_FRONTEND_BUILD=1, CI=true), each cargo command through the heavy.sh limiter: the verify (cargo test -p roko-cli --test secret_canary canary_absent_from_every_file_after_plan_run) passes; the whole secret_canary file 12/12, with the new playbooks_from_verified_attempts_do_not_contain_canary; roko-cli lib verified_playbooks and verified_plan_task_writes_a_playbook 4/4; roko-learn playbook:: 35/35, with the new saved_playbook_carries_no_registered_secret; cargo +nightly fmt --all and cargo clippy -p roko-learn -p roko-cli --no-deps -D warnings clean. Mutation runs: without the save scrub, or without the rewrite's redaction, the roko-learn test fails; without the sink's redaction (store kept) the canary sink test fails, the repeat adding a second playbook. Filed bug-ae5d22: a live tool-step target is cut, whitespace-collapsed and heuristic-scrubbed before any exact redaction, and topic terms are lowercased, so part or a lowercased form of a secret can still reach a playbook."
+++

## Problem

Plan runs now write playbooks from verified attempts (gap-89f985, merged 7f3f346e5). Each step's description is the
tool call's target as its tool step showed it, for example a shell command's first line. Neither
`runtime_feedback/verified_playbooks.rs::playbook_steps` nor `roko_learn::playbook::PlaybookStore::save` runs the
process's secret scrubber (`roko_core::scrub_secrets` / `scrub_secrets_in_json`), so a secret on a command line is
written to `.roko/learn/playbooks/<id>.json` verbatim.

The workspace test run for the release PR (2026-10-17, frozen commit 31964059d, CI-like env) caught it:
`roko-cli --test secret_canary canary_absent_from_every_file_after_plan_run` fails with "the canary reached 1
file(s): …/.roko/learn/playbooks/ep-canary-plan-T1-….json: \"description\": \"echo <SECRET>\"". The same test passed
in the 2026-10-15 baseline run; gap-89f985's worker ran only targeted tests, and its post-merge verify is a single test.

## Why it matters

Goal `release`, under the secrets and git guard epic (spec-ba7bea, p0): no secret roko loaded may reach a file it
writes. Learned state under `.roko/learn/` is read back into later prompts, so a leaked key would also reach models.
This blocks the release PR (decision 11).

## Where

- `crates/roko-cli/src/runtime_feedback/verified_playbooks.rs::playbook_steps` and the goal built from the task.
- `crates/roko-learn/src/playbook.rs::PlaybookStore::save`, the one writer of playbook files, so every producer
  (plan runs and `roko bench`) is covered.
- `crates/roko-cli/tests/secret_canary.rs`.

## Plan

1. Scrub at the store: `PlaybookStore::save` serializes, then redacts with the process scrubber before writing,
   the way the other learned-state writers do. Also scrub the step text where `playbook_steps` builds it, so the
   in-memory playbook (events, dedupe keys) never carries the secret either.
2. Unit test: a playbook whose step holds a registered secret is saved without it.
3. The canary test passes.

## Done when

- [ ] `canary_absent_from_every_file_after_plan_run` passes.
- [ ] No playbook file, `playbook_written` event or dedupe key carries a registered secret.
