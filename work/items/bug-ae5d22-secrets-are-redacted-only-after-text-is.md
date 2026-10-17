+++
id = "bug-ae5d22"
kind = "bug"
title = "Secrets are redacted only after text is cut, collapsed or lowercased, so a live tool-step target or a playbook's topic terms can keep part of one"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-agent/live-output"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "session:reg-d49ed1 worker 2026-10-17 (review of the fix)"
discovered_from = "reg-d49ed1"
anchors = ["crates/roko-agent/src/live_output.rs::tool_step_target", "crates/roko-cli/src/graph_task_dispatch/feedback.rs", "crates/roko-cli/src/dispatch/prompt_builder.rs::task_topic_terms"]
parent = "spec-ba7bea"
links = { depends_on = [], blocks = [], related = ["reg-d49ed1", "gap-b35a57"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn tool_step_target_redacts_registered_secrets' crates/roko-agent/ && cargo test -p roko-agent --lib tool_step_target_redacts_registered_secrets"

[[verify]]
command = "grep -rqw 'fn verified_topic_terms_carry_no_registered_secret' crates/roko-cli/ && cargo test -p roko-cli --lib verified_topic_terms_carry_no_registered_secret"
+++

## Problem

roko's persistence writers remove the process's registered secrets (the values loaded from `.env` files, provider
keys and config secrets) by exact match: `roko_core::obs::scrub_secrets` and its JSON forms replace each literal
value. Two places transform text before that exact match can run, so part or a changed form of a secret survives:

1. **Live tool-step targets.** `roko_agent::live_output::tool_step_target` scrubs a target only with the heuristic
   patterns of `roko_agent::safety::scrub::scrub_secrets` (vendor-prefixed keys, JWTs, PEM blocks, `NAME=value`),
   never with the process's registered values. It then collapses whitespace and cuts the target at 120 characters.
   A registered value with no heuristic shape (a `.env` password, a credential inside a connection URL that a
   command is given) passes through in full, and once cut, a value that straddles character 120 leaves its first
   characters followed by `…`. Every consumer of `LiveAgentEvent::ToolStep` sees that target. `ToolStep` is the screened half of the live feed (its
   doc calls the target "scrubbed"; an untrusted consumer is told to treat `Unscreened` events as sensitive, not
   these), and the attempt's tool calls become a verified attempt's playbook steps (gap-89f985). reg-d49ed1
   redacts the whole value where the playbook is built and where it is stored, but no exact match can find a value
   that was cut or whose internal whitespace was collapsed.
2. **A verified attempt's topic terms.** `graph_task_dispatch/feedback.rs` builds `VerifiedAttempt::topic_terms`
   with `dispatch::prompt_builder::task_topic_terms(task)`, which lowercases the task's title, description,
   acceptance and file names into words. A mixed-case secret quoted in a task's text (a plan written from a
   prompt that pasted a key) becomes a lowercased term. The playbook sink joins the terms into the playbook's
   `when_pattern`; reg-d49ed1 redacts that string, but the exact, case-sensitive match misses the lowercased form,
   so the playbook file keeps it.

Seen while fixing reg-d49ed1 (2026-10-17, by reading the code). Not yet reproduced with a running roko.

## Why it matters

Goal `release`, secrets and git guard epic (spec-ba7bea): no secret roko loaded may reach a file it writes, and
`.roko/learn/playbooks/` is read back into later prompts. A secret's prefix or its lowercased form is most of the
secret.

## Where

- `crates/roko-agent/src/live_output.rs::tool_step_target`: builds the `ToolStep` target (heuristic scrub, then
  whitespace collapse, then the 120-character cut).
- `crates/roko-cli/src/graph_task_dispatch/feedback.rs`: builds `VerifiedAttempt` (`topic_terms` from
  `task_topic_terms`) for `FeedbackEvent::TaskVerified`.
- `crates/roko-cli/src/runtime_feedback/verified_playbooks.rs::candidate_playbook`: turns both into a playbook.

## Current state

reg-d49ed1 redacts registered secrets in `verified_playbooks.rs` (goal, steps, `when_pattern`, the
`playbook_written` goal) and in `roko_learn::playbook::PlaybookStore` (every file it writes), so a whole,
unchanged secret no longer reaches a playbook. The two transformations above happen before either of them.

## Plan

1. In `tool_step_target`, pass the raw value through `roko_core::obs::scrub_secrets` before the heuristic scrub,
   the whitespace collapse and the cut, so the cut can only fall after a redaction.
2. Build a verified attempt's topic terms from the task's text with the registered secrets redacted (for example
   `task_topic_terms` over a copy of the task whose title, description and acceptance went through
   `scrub_secrets`), so a lowercased secret never becomes a term.
3. Tests: a `Bash` step whose command holds a registered value that straddles character 120, and one that holds
   it with a whitespace run inside, leave no 8-character piece of it in the target
   (`tool_step_target_redacts_registered_secrets`); a task whose description quotes a mixed-case registered value
   yields topic terms without its lowercased form (`verified_topic_terms_carry_no_registered_secret`).

## Done when

- Both named tests pass, and `secret_canary` still passes.

## Notes

- A test that installs the process's scrubber should put the previous one back when it ends; see
  `PlantedKeyScrubber` in `crates/roko-serve/src/routes/run.rs`.
