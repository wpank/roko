+++
id = "gap-56f761"
kind = "gap"
title = "A try is judged only on the tree it ends with, so its own checks' better tree is lost within the try: phase 3's T1#1 saw 140/149 twice, then the in-flight call's write after the 2700 s bound broke the build and the gate judged only that tree (run 7b8cb121)"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-21
updated = 2026-10-21
last_verified = 2026-10-21
source = "round-v7-hermes-pilot#M-p3-7b8cb121-T1-1"
discovered_from = "agent:hermes-pilot (round v7 phase 3 run 7b8cb121, findings section M); the lead's decision (option a) 2026-10-21"
anchors = ["crates/roko-std/src/tool/builtin/task_checks.rs", "crates/roko-cli/src/graph_task_dispatch/verification.rs::run_verify_steps", "crates/roko-cli/src/graph_task_dispatch/best_try.rs", "crates/roko-cli/src/graph_task_dispatch/hand_off.rs"]
links = { depends_on = [], blocks = [], related = ["gap-54089d", "gap-fefdf5", "bug-cbbaa8", "gap-770cb5"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_try_is_judged_on_its_best_checked_tree_when_its_last_edit_is_worse' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_try_is_judged_on_its_best_checked_tree_when_its_last_edit_is_worse"
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
