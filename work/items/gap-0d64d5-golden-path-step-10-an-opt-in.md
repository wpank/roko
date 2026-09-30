+++
id = "gap-0d64d5"
kind = "gap"
title = "Golden-path step 10: an opt-in hold that shows each task's diff and waits for approval before it merges"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "L"
subsystem = ["roko-serve/plans", "roko-cli/graph_execution"]
created = 2026-09-29
updated = 2026-09-30
last_verified = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (15:04); docs/whitepaper/data/mechanisms.toml (row SS6)"
anchors = ["crates/roko-serve/src/routes/plans.rs::find_agent_branch", "crates/roko-serve/src/routes/plans.rs::list_reviews", "crates/roko-serve/src/routes/plans.rs::task_diff", "crates/roko-cli/src/graph_execution/control_adapter.rs::GraphExecutionControlAdapter", "crates/roko-cli/src/graph_execution/delivery.rs::GitDeliveryBackend"]
lane = "rust-hot"
parent = "spec-a0e40a"
links = { depends_on = ["gap-3b5361", "spec-f830c4"], blocks = [], related = ["gap-23fa38", "gap-c3add8", "bug-619253", "gap-a6de8d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn approval_hold_blocks_merge_until_approved' crates/roko-cli/ && cargo test -p roko-cli approval_hold_blocks_merge_until_approved"

[[verify]]
command = "grep -rqw 'fn task_diff_reads_the_graph_task_result' crates/roko-serve/ && cargo test -p roko-serve task_diff_reads_the_graph_task_result"
+++

## Problem

Step 10 of the golden path (tldr/04; whitepaper §4.10 "Review") is an optional hold: a person sees each verified
task's diff and approves it before it merges. The status matrix tags it MISSING with the verdict "build" (row SS6),
and no item or epic covers it:

- **No diff to show.** The review and diff routes (`list_reviews`, `submit_review` and `task_diff` in
  `crates/roko-serve/src/routes/plans.rs`, about `:1313-1530`) find a task's work with `find_agent_branch` (`:1535`),
  which lists `agent/*/<task_id>` branches. Graph runs don't create those branches, so the routes find nothing.
- **Nothing to hold.** Nothing merges on the Graph path yet: `accept_attempt` has no production caller (gap-3b5361),
  and per-task commits on a plan branch are spec-f830c4's work.
- **No hold.** A task goes straight from verified to done; there is no awaiting-approval state and no
  approve-or-reject control in the Graph control adapter.

Row SS6 names gap-25065c as its item, but no row of the research checklist covers approval or a per-task diff, so that
import will not create one.

## Why it matters

The whitepaper's golden path promises this step, and tldr/05 §6 decision 8 proposes approve-before-merge as a per-plan
opt-in by default. Cheap executors make it more important: a person can inspect exactly what each attempt changed
before it lands. The portal's review view depends on the diff routes. Goal `golden-path`, epic spec-a0e40a.

## Where

- `crates/roko-serve/src/routes/plans.rs`: `find_agent_branch`, `diff_summary`, `list_reviews`, `submit_review`,
  `task_diff`. gap-a6de8d plans to split this file.
- `crates/roko-cli/src/graph_execution/control_adapter.rs::GraphExecutionControlAdapter`: pause, resume and retry
  controls; an approval control would sit here.
- `crates/roko-cli/src/graph_execution/delivery.rs::GitDeliveryBackend` and the worktree `accept_attempt`: where a
  verified task's work is merged, once gap-3b5361 and spec-f830c4 land.

## Current state

Checked at `4c0326dfc`: `find_agent_branch` looks only for `agent/*/<task_id>`. The matrix row (checked at
`a17d4dadd`) says Graph runs never create such branches. Confirm the branch names the Graph worktree manager uses
before relying on this. Older approval items are parked: gap-23fa38 (the approval flow is not connected end to end),
gap-c3add8 (`--approval` on the Graph engine) and bug-619253.

## Plan

1. **The diff:** once spec-f830c4 records each task's result as a commit on the plan branch (or keeps the attempt's
   diff), the review routes read that record instead of guessing branch names.
2. **The hold:** an opt-in plan setting (for example `approval = "per_task"`) parks a verified task in
   `awaiting_approval` before its merge. Approve merges it; reject fails the attempt, with the reviewer's note fed to
   the retry.
3. **Controls:** approve and reject through REST (`submit_review`), the CLI and the portal, using the same control
   path as pause and resume.
4. **Tests:** `approval_hold_blocks_merge_until_approved` (roko-cli) and `task_diff_reads_the_graph_task_result`
   (roko-serve).

## Done when

- [ ] With the hold on, no verified task merges until someone approves it, and a rejection fails the attempt with the
      reviewer's note.
- [ ] The diff route returns a Graph task's real diff.
- [ ] Both `[[verify]]` commands pass.

## Notes

- Hot files: `routes/plans.rs` and `graph_execution/`. Start after the integration work (gap-3b5361, spec-f830c4) and
  the serve-routes split (gap-a6de8d).
- When this lands, supersede or close the parked approval items (gap-23fa38, gap-c3add8, bug-619253) with a pointer
  here.
- 2026-09-30 (wk-integrate): Implemented on `work/bug-4862cf` at `d2c7c86a6`; cargo verification deferred to the batch check.
  - `[meta] approval = "per_task"` (`TaskMeta.approval`, `ApprovalMode`) holds each verified attempt before `accept_attempt` folds it into the plan branch (`GraphTaskDispatcher::await_review`). The attempt's change goes to `.roko/state/review-holds/<plan>/<task>.json` (`RokoLayout::review_hold`): its base commit, numstat and patch, built in a temporary index, so the checkout is untouched. The attempt then waits for a decision naming it in `.roko/state/reviews.jsonl` (`RokoLayout::reviews_log`).
  - On a decision: approval accepts the attempt. A rejection or a skip fails it with `gate: "review"`, and the reviewer's note becomes the next attempt's retry feedback. A cancelled run ends the hold. The hold file is removed in every case.
  - The run refuses a plan with a hold unless it runs with `--worktree-per-task` and the default topology: the rich topology's plan gate accepts on its own.
  - roko serve: `task_diff` returns a held attempt's change, else the Graph task's recorded result (its `workspace.attempt_commit` in the plan's `activities.jsonl`, diffed against its parent), else the legacy `agent/*` branch. `list_reviews` shows held tasks as `awaiting_approval`, and `submit_review` on a held task records the decision with the attempt's key instead of merging.
  - CLI: `roko plan review <plan> <task> --approve|--reject [--note]` records the same decision (`docs/v3/28-CLI.md`).
  - Tests: `approval_hold_blocks_merge_until_approved`, `a_rejected_attempt_fails_with_the_reviewers_note`, `approval_needs_worktrees_and_the_default_topology` (roko-cli), and `task_diff_reads_the_graph_task_result` (roko-serve). Clippy with -D warnings on roko-cli, roko-serve and roko-fs is clean.
  - Not done:
    - A held task keeps its agent slot while it waits.
    - The TUI shows the hold as a `review` gate result but has no approve or reject keys. `control_adapter` has an in-process approval API that is not wired to this.
    - The portal's review view is unchanged.
    - The legacy `submit_review` approve path still runs `git merge` in the server's checkout for `agent/*` branches.
    - The parked items gap-23fa38, gap-c3add8 and bug-619253 are left for the coordinator to supersede.
