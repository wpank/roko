+++
id = "reg-d49ed1"
kind = "regression"
title = "Plan-run playbooks write tool-call targets unscrubbed, so a secret on a command line reaches .roko/learn/playbooks (secret_canary fails)"
status = "open"
triage = "unverified"
severity = "p1"
subsystem = ["roko-learn/playbook", "roko-cli/runtime_feedback"]
created = 2026-10-17
updated = 2026-10-17
source = "release PR check 2026-10-17 at 31964059d (workspace tests)"
anchors = ["crates/roko-cli/src/runtime_feedback/verified_playbooks.rs::playbook_steps", "crates/roko-learn/src/playbook.rs::save", "crates/roko-cli/tests/secret_canary.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --test secret_canary canary_absent_from_every_file_after_plan_run"
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
