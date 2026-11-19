+++
id = "gap-56f761"
kind = "gap"
title = "A try is judged only on the tree it ends with, so its own checks' better tree is lost within the try: phase 3's T1#1 saw 140/149 twice, then the in-flight call's write after the 2700 s bound broke the build and the gate judged only that tree (run 7b8cb121)"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-21
updated = 2026-10-21
last_verified = 2026-10-21
last_verified_rev = "33a0b7859"
source = "round-v7-hermes-pilot#M-p3-7b8cb121-T1-1"
discovered_from = "agent:hermes-pilot (round v7 phase 3 run 7b8cb121, findings section M); the lead's decision (option a) 2026-10-21"
anchors = ["crates/roko-std/src/tool/builtin/task_checks.rs", "crates/roko-cli/src/graph_task_dispatch/verification.rs::run_verify_steps", "crates/roko-cli/src/graph_task_dispatch/best_try.rs", "crates/roko-cli/src/graph_task_dispatch/hand_off.rs"]
links = { depends_on = [], blocks = [], related = ["gap-54089d", "gap-fefdf5", "bug-cbbaa8", "gap-770cb5"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_try_is_judged_on_its_best_checked_tree_when_its_last_edit_is_worse' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_try_is_judged_on_its_best_checked_tree_when_its_last_edit_is_worse"

[closed]
at = 2026-10-21
at_ts = "2026-10-21T10:42:30Z"
commit = "33a0b7859"
via = "manual"
size = "M"
forced = false
evidence = "33a0b7859: each run of an attempt's own checks, the agent's run_tests with build \"checks\" and the hand-off check alike (both roko_std task_checks::run_checks), is recorded beside the attempt's checks file (task_check_runs_path, one CheckRun per line): the git tree it judged (roko_std checkout_tree::snapshot_tree, a copy of the index, git add --all, git write-tree; about 0.1-0.18 s on Undertow's tree), the checks that passed, and the failing one's label and output tail. The checks file and the record stay until the try is judged. After the gate judged the final tree, auto-fix included, restore_checked_best ranks it (Reach::of_gate) against the best recorded run (Reach::of_run) by gap-54089d's order: steps, then the first failing step's assertions, then its test cases; ties go to the latest. A final tree behind it is replaced in the task's own checkout by that tree (gap-fefdf5's restore_tree, roko state left out; a restore that puts back no file is skipped), and the verify steps run again (rerun_verify_steps, the auto-fix re-run made a method) and judge it. The next try's feedback carries 'Kept by roko: this try's own checks saw a better tree at HH:MM:SS than the one it ended with: ...', and the hub gets checked_tree_restored. Test a_try_is_judged_on_its_best_checked_tree_when_its_last_edit_is_worse, in phase 3's shape: run_tests sees doctest 140 of 149 assertions, then the last write breaks the build. Green: try 2's prompt carries the note and '140 assertions', and try 2 reads 'good' back. Red on main's code (2026-10-21 11:01) failed its first assertion, but the request it read was the error-enrichment helper's, because the test then let the task fall back to the API model. Dispatch reads a task's model from task_def_json, so the test now pins it there (pinned()), and helpers answer on a fake CLI. On the branch merged with main 3d3d00d57 (3e9fd8e94): roko-cli lib 3991 passed, 0 failed, 7 ignored (share_run_scrubs_output_text_and_prompt skipped); before the merge, roko-std lib 244/0 (main changes no roko-std file). On 3e9fd8e94: clippy -D warnings (roko-std, roko-cli) clean; nightly rustfmt clean. The held-then-slow test in 822e06461 (a_held_call_that_then_waits_long_keeps_its_queue_and_its_providers_timeout) passes on both."
+++

## Problem

roko judges a try only on the tree it ends with, so the try's best tree can be lost within the try. The try's own
checks can see a good tree, through the agent's `run_tests {"build":"checks"}` (gap-770cb5) and, since bug-cbbaa8,
the hand-off check. A later edit can then make it worse.

gap-54089d ranks tries against each other, so the next try starts from the best try's tree. That doesn't help when
one try loses its own best state: the try's best is then its broken final tree.

Round v7, Undertow phase 3, run `7b8cb121` (roko 5a9884c95 + bug-5e597a), T1#1 on mid (glm-4.7). Local times, from
hermes-pilot's findings, section M:

| Time | What happened |
|---|---|
| 05:28:02 | The try's own checks: P3.01 131/131. |
| 05:28:12 | P3.01–02: 140/149. |
| 05:32:43–05:32:54 | It rewrote sequencer.cpp; 140/149 again. |
| 05:33:50 | The try's 2700 s bound: "the try ran its rung's time; it makes no further model call". |
| 05:38:35 | The call in flight since 05:32:54 ended (11,116 tokens), and its `write_file core/include/undertow/sequencer.h` landed after the bound. |
| 05:38:39 | The gate found the build broken ("no member named 'load' in 'undertow::Pattern'"), and all 4 steps failed. |

- The tree at the bound held 140/149.
- T1#2 started from the broken tree, and spent 16 min from the compile errors to its first passing check.
- With bug-cbbaa8 sending red hand-offs back, tries run longer and have more chances to make a good tree worse.

## Where

- `crates/roko-std/src/tool/builtin/task_checks.rs`: `run_tests {"build":"checks"}` runs the attempt's checks and
  returns their result to the agent. Nothing keeps the tree they judged.
- `crates/roko-cli/src/graph_task_dispatch/hand_off.rs` (`SendBack`): the hand-off check runs the same checks.
- `crates/roko-cli/src/graph_task_dispatch/verification.rs` (`run_verify_steps`): the gate judges the final tree
  only.
- `crates/roko-cli/src/graph_task_dispatch/best_try.rs`: gap-54089d's ranking (`BestTry::behind`) and gap-fefdf5's
  tree restore.

## Plan

Option (a), the lead's decision on 2026-10-21. Option (b), settling on the tree at the bound, is a special case of
it, so it isn't built separately.

1. Whenever the try's own checks report a result, record the tree they judged. That covers the agent's `run_tests`
   with `build: "checks"` and the hand-off check.
   - The tree is a git tree object of the checkout. Ignored files such as `build/` are left out.
   - Each run is appended beside the attempt's checks file: the tree, the checks that passed, and the failing check's
     label and output tail.
   - Cost: about 0.1–0.2 s on Undertow's tree (a scratch clone with warm build dirs), and a few KB of objects per
     snapshot.
2. At settle, the gate judges the final tree as before, auto-fix included. Then, if the final tree ranks behind the
   best recorded tree, roko puts that tree back (gap-fefdf5's restore) and runs the verify steps again, so the gate
   judges it.
   - Ranking is gap-54089d's: steps passed, then the first failing step's passed assertions, then test cases. Ties
     go to the latest.
   - The next try's feedback says so ("this try's own checks saw a better tree at 05:32:54 than the one it ended
     with ..."), and the hub gets a `checked_tree_restored` event.

## Done when

A try whose last edit leaves its tree worse than one its own checks saw is judged on that better tree, and its
feedback says so. Checked by the test in `[[verify]]`, in phase 3's shape: the try's checks see 140/149, then its
last edit breaks the build.
