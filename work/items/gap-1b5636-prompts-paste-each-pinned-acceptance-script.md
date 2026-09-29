+++
id = "gap-1b5636"
kind = "gap"
title = "Prompts paste each pinned acceptance script verbatim; show pinned steps by their header line only"
status = "open"
triage = "unverified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/dispatch"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-accept's report on gap-d14a43, branch work/gap-d14a43 at 37b6d7c95)"
anchors = ["crates/roko-cli/src/dispatch/prompt_builder.rs", "crates/roko-cli/src/task_parser.rs::TaskDef"]
lane = "rust-cold"
parent = "spec-e57870"
links = { depends_on = ["gap-d14a43"], blocks = [], related = ["gap-ba4d01", "bug-b0fd73"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn pinned_accept_steps_show_only_their_header' crates/roko-cli/src/ && cargo test -p roko-cli --lib pinned_accept_steps_show_only_their_header"
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
