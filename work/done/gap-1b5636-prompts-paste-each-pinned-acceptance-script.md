+++
id = "gap-1b5636"
kind = "gap"
title = "Prompts paste each pinned acceptance script verbatim; show pinned steps by their header line only"
status = "done"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/dispatch"]
created = 2026-09-29
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "a8159e1ec"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-accept's report on gap-d14a43, branch work/gap-d14a43 at 37b6d7c95)"
anchors = ["crates/roko-cli/src/dispatch/prompt_builder.rs", "crates/roko-cli/src/task_parser.rs::TaskDef"]
lane = "rust-cold"
parent = "spec-e57870"
links = { depends_on = ["gap-d14a43"], blocks = [], related = ["gap-ba4d01", "bug-b0fd73"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn pinned_accept_steps_show_only_their_header' crates/roko-cli/src/ && cargo test -p roko-cli --lib pinned_accept_steps_show_only_their_header"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in a8159e1ec. Batch 12a gate on the merged tree (MAIN a8159e1ec has the same tree as gated 265acb18b): cargo check --workspace --tests, nightly fmt --check (after the coordinator's rustfmt commits a3509fc46 and 6144df24b) and clippy -p roko-cli -p roko-learn -p roko-core -p roko-agent -p roko-gate -p roko-serve --no-deps -D warnings clean; lib tests pass: roko-cli 3133, roko-core 1938, roko-learn 1196, roko-serve 958, roko-gate 689, roko-agent 2257 (its one failure, a_timed_out_attempt_reports_the_usage_it_streamed, is a load flake at load 77 that passes alone). Verify: pinned_accept_steps_show_only_their_header passes (roko-cli lib)."
+++

## Problem

On `work/gap-d14a43` (37b6d7c95, not merged yet), `[task.accept]` compiles each pinned acceptance test into a verify step. The step is a generated script of about 25 lines. It begins with the header `# roko accept:` (`task_accept.rs`, `STEP_HEADER`), checks the pinned copy against its sha256, copies it in and runs it.

Both prompt renderers print every verify step's full command:

- `dispatch/prompt_builder.rs`, under "## Verification Commands" (:1958-1964 on the branch);
- `TaskDef::build_prompt` (task_parser.rs:528 onward).

So every pinned test pastes its whole generated script into the implementer's prompt. It costs tokens, shows the store path and the hash plumbing, and invites the agent to imitate the script or work around it.

## Why it matters

Specs a cheap model can execute (epic spec-e57870): the prompt should state what must pass, not the harness's mechanics. A cheap model reading 25 lines of plumbing per test gets noise where it needs the requirement.

## Where

The two renderers above, and `task_accept.rs` (the step's shape and its header) on the branch.

## Current state

The behaviour exists only on the unmerged branch. This item depends on gap-d14a43.

## Plan

1. In both renderers, print a pinned accept step as its header line only (`# roko accept: <test path>`), plus one line saying the harness runs it. Keep authored verify commands verbatim.
2. Add `pinned_accept_steps_show_only_their_header`, covering both renderers.

## Done when

- [ ] A prompt for a task with pinned tests lists each one by its header line, and no generated script body reaches the prompt.
- [ ] The `[[verify]]` command passes.

## Notes

- Merge gap-d14a43 first. The anchors are files that exist at BASE; the accept code arrives with that merge.
- Implemented on `work/bug-019f02` at `97c383cab`; cargo verification deferred to the batch check.
- 2026-09-30: `prompt_builder.rs` renders verify steps in two places, the runner context's `# Verify` (through
  `PromptContext::verify_commands`) and the user prompt's `## Verification Commands`, and both now use
  `task_accept::prompt_command`. `TaskDef::build_prompt` has no callers at BASE, so no prompt goes through it,
  and it was left alone (wk-taskdef owns `task_parser.rs`). Still open, not in this item:
  `graph_task_dispatch/verification.rs` quotes `step.command` in full in skipped-step lists, progress events
  and published gate output, so a failing pinned step still repeats its script in retry feedback.
