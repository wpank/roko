+++
id = "bug-b7d9ad"
kind = "bug"
title = "A warm build tree keeps compiled-in source paths: relocation rewrites flags.make but keeps its mtime, so an object built with a definition naming the person's checkout (Undertow's UNDERTOW_SOURCE_DIR) is not rebuilt, and a check writes into the person's checkout (rehearsal p41: T2's PNGs landed in the base, T2 settled already_satisfied, the delivery has none)"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/worktree"]
created = 2026-10-21
updated = 2026-10-26
last_verified = 2026-10-26
last_verified_rev = "1df36e624"
source = "round-v7 rehearsal p41 (roko-ladder), the lead's ask"
discovered_from = "agent:roko-ladder (round v7 rehearsal p41)"
anchors = ["crates/roko-cli/src/orchestrator/worktree/build_trees.rs::relocate", "crates/roko-cli/src/orchestrator/worktree/build_trees.rs::write_keeping_times", "crates/roko-cli/src/graph_task_dispatch/checkout_writes.rs::CheckoutState", "crates/roko-cli/src/graph_task_dispatch/verification.rs::screen_and_verify"]
links = { depends_on = [], blocks = [], related = ["gap-6754ab", "bug-40bf59"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn checks_that_write_into_the_persons_checkout_say_so' crates/roko-cli/src && cargo test -p roko-cli --lib -- checks_that_write_into_the_persons_checkout_say_so"

[[verify]]
command = "grep -rqw 'fn a_checkout_state_names_what_changed' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_checkout_state_names_what_changed"

[[verify]]
command = "grep -rqw 'fn the_checkout_warning_says_how_to_drop_or_keep_the_changes' crates/roko-cli/src && cargo test -p roko-cli --lib -- the_checkout_warning_says_how_to_drop_or_keep_the_changes"

[[verify]]
command = "grep -rqw 'fn rokos_own_writes_in_the_checkout_raise_no_checkout_flag' crates/roko-cli/src && cargo test -p roko-cli --lib -- rokos_own_writes_in_the_checkout_raise_no_checkout_flag"

[[verify]]
command = "grep -rqw 'fn graph_report_names_the_checkout_a_tasks_checks_changed' crates/roko-cli/src && cargo test -p roko-cli --lib -- graph_report_names_the_checkout_a_tasks_checks_changed"

[[verify]]
command = "grep -rqw 'fn a_runs_summary_names_the_checks_that_changed_the_checkout' crates/roko-serve/src && cargo test -p roko-serve --lib -- a_runs_summary_names_the_checks_that_changed_the_checkout"

[closed]
at = 2026-10-26
at_ts = "2026-10-26T14:23:53Z"
commit = "1df36e624"
by = "roko-speed"
executor = "claude-agent"
via = "manual"
size = "M"
claimed_at = "2026-10-26T10:40:25Z"
forced = false
evidence = "chain44 on main 7d47479ce plus 1df36e624: fmt and clippy clean; the verify list passed 14/14 in roko-cli (checks_that_write_into_the_persons_checkout_say_so, a_checkout_state_names_what_changed, the_checkout_warning_says_how_to_drop_or_keep_the_changes, rokos_own_writes_in_the_checkout_raise_no_checkout_flag, graph_report_names_the_checkout_a_tasks_checks_changed, a_worktree_run_delivers_each_plan_into_its_batch_branch, sibling_files_since_names_every_attempt_that_overlapped_the_window, the unchanged-tree, scope and gap-20301d tests) and 3/3 in roko-serve (a_runs_summary_names_the_checks_that_changed_the_checkout, a_runs_summary_lists_its_skipped_rungs, run_summary_lists_its_rungs); lib suites green: roko-agent 2400, roko-cli 4008, roko-core 2048, roko-learn 1395, roko-runtime 213, roko-serve 1083. A check that writes into the person's checkout while an attempt runs in its own worktree raises the hub diagnosis checkout_written:<plan>/<task> (the paths, the likely cause, how to drop or keep them, the tasks in flight beside it), lists the paths as checkout_written scope findings, settles an attempt that changed nothing unverified, and names it in the run summary, roko diagnose and the end of roko plan run. Prevention is gap-184cba."
+++

## Problem

A warm build tree that roko clones from the person's checkout into an attempt's worktree (gap-6754ab) keeps the
compile-time source paths of what it already built. `relocate` rewrites every absolute path in the tree's text
files to the worktree's, but gives each rewritten file back its times (`write_keeping_times`), so make rebuilds
nothing. That covers a target's `flags.make` too. An object compiled with a definition that names the source
checkout therefore keeps naming it, and a check that writes files through that path writes into the **person's
checkout**, not the attempt's worktree.

Undertow's spec target has `UNDERTOW_SOURCE_DIR="${PROJECT_SOURCE_DIR}"` (`tests/spec/CMakeLists.txt:67`), and its
review PNGs are written under it.

Round v7 rehearsal p41 (`tmp/hermes-roko-v7/roko-ladder/rehearsals/p41/`, run `20261021-120220`):
- T2, "Refresh and commit the phase 4.1 review PNGs": try 1 ran with 2 warm trees. Its check
  `tools/spec.sh 4 budget` passed in 131.9 s.
- The PNGs went to the base checkout. `ws/` shows `M artifacts/ui/editor-{idle,playing,clipping}.png`, written at
  12:11 during the run.
- The attempt's worktree changed nothing, so T2 settled `already_satisfied` and the delivery has no PNGs.
- In a live run the base is the person's real checkout. The person's tree gets dirtied, and the task commits
  nothing.

## Where

- `crates/roko-cli/src/orchestrator/worktree/build_trees.rs::relocate` and `write_keeping_times`: every rewritten
  file keeps its times.
- What runs an attempt's checks in its worktree: nothing looks at the person's checkout afterwards.

## Plan

Undertow has since moved its checks to run-time paths (Undertow ceac70e), so its spec no longer bakes the checkout
into the binary. roko's side is detection (the lead's design call, 2026-10-26):
- Snapshot the person's checkout (`git status --porcelain -z --untracked-files=all`, each path with its size and
  mtime) before and after an attempt's checks, when the attempt runs outside it. Leave out what roko itself writes
  during a run: `.roko/`, the plan's own directory, and nested checkouts such as worktrees.
- Anything that changed while the checks ran is a red flag:
  - a warning diagnosis on the hub saying the run changed the person's checkout, which paths, how to drop them
    (`git -C <checkout> checkout -- <paths>`, `git -C <checkout> clean -f -- <new files>`) or keep them, and the other
    tasks that ran meanwhile, any of which may have written them;
  - the paths as `checkout_written` scope findings in the attempt's verdict;
  - an attempt that changed nothing settles `unverified`, not `already_satisfied`. Its checks may have tested the
    person's checkout, and unverified costs no retry: a path compiled into the build is not the agent's to fix.
- Plan level: the run summary (`checkout_written`), `roko diagnose` and the end of `roko plan run` (and its plan's
  `graph.plan_unverified` event) name the task as unverified with the paths.
- Prevention, making the objects compiled with the checkout's path rebuild after the clone, is gap-184cba.

## Done when

An attempt whose checks change the person's checkout instead of its own worktree says so, on the hub, in its records
and at the plan level, and is not settled `already_satisfied`; a clean run of plans that live in the checkout raises
no flag. Checked by the tests in `[[verify]]`.
