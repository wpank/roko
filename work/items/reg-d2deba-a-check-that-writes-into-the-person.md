+++
id = "reg-d2deba"
kind = "regression"
title = "A check that writes into the person's checkout during the try's own run_tests or hand-off check goes unseen: bug-b7d9ad watches the checkout only around the gate's verify steps, and since gap-6df2bf the gate no longer re-runs a step the try's own checks passed on its final tree"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "round-v7 roko-speed, found when bug-b7d9ad merged over gap-6df2bf; the lead's direction and priority"
discovered_from = "agent:roko-speed (round v7)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/checkout_writes.rs::checkout_before_checks", "crates/roko-cli/src/graph_task_dispatch/verification.rs::screen_and_verify", "crates/roko-std/src/tool/builtin/task_checks.rs::run_checks"]
links = { depends_on = [], blocks = [], related = ["bug-b7d9ad", "gap-6df2bf", "gap-184cba"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_try_s_own_checks_that_write_into_the_checkout_say_so' crates/roko-cli/src && cargo test -p roko-cli --lib a_try_s_own_checks_that_write_into_the_checkout_say_so"
+++

## Problem

bug-b7d9ad (merged f9ba45559) reports a run that changes the person's own checkout while an attempt runs in its own
worktree. A typical cause is a build path compiled in from the person's checkout. It snapshots the checkout's `git
status` right before the gate's verify steps and again right after. Writes at any other time during the try go
unseen:
- **The try's own checks.** gap-6df2bf (merged 635de7f86) passes a gate step without running it again when the try's
  own checks passed it on the tree the try ends with. Those checks are `run_tests` with `build: "checks"` or the
  hand-off check. For a try that changed its tree, the check that writes into the person's checkout now runs only
  in the try's own checks. So the gate's window sees nothing, and neither the hub nor the verdict says a word.
  - Before 6df2bf the gate re-ran it, and the rewrite showed up as a modified path. Detection regressed.
- **Other builds the agent runs.** These are `run_tests` with another build, or `bash`, in the try's worktree. The
  same compiled-in path writes into the person's checkout. Nothing watches the checkout then.

The case rehearsal p41 found is still caught: a try that changed nothing has its checks probed at the gate, and
6df2bf never skips that probe.

## Why it matters

Golden path, truth. The person's checkout changing behind their back is the harm bug-b7d9ad set out to report. A
check whose output lands in the person's checkout also means the try's own tree lacks it (p41's PNGs), and its
delivery too.

## Where

- `roko-cli/src/graph_task_dispatch/checkout_writes.rs`: `CheckoutState`, `checkout_before_checks`,
  `checkout_written` and `report_checkout_written`.
- `roko-cli/src/graph_task_dispatch/verification.rs::screen_and_verify`: the window around the gate's steps.
- `roko-std/src/tool/builtin/task_checks.rs::run_checks`: every run of the try's own checks, `run_tests` with
  `build: "checks"` and the hand-off check. It records a `CheckRun` beside the attempt's checks. Its
  `ctx.immune_root_path` is the person's checkout, and `ctx.worktree()` is the attempt's.
- `roko-cli/src/graph_task_dispatch.rs`: the dispatch, where a try starts and settles.

## Plan

The lead's direction: watch the person's checkout over the whole try, with the same exclusions and naming of
overlapping tasks.
1. **A snapshot at dispatch, compared at settle.** Take it when an attempt in its own worktree starts. Compare it in
   every settle path, verified or not, and report what changed as b7d9ad does.
2. **Around every check roko runs.**
   - `run_checks` snapshots the checkout before and after the try's own checks, both `run_tests` and the hand-off
     check. It records the paths that changed in its `CheckRun`, as `checkout_written`.
   - The gate's window stays as it is.
   - Move `CheckoutState` into roko-std so both crates use one snapshot.
3. **Attribution.** A path a check run or a gate step changed is "while its checks ran". Any other path the try's
   window shows is "while the try ran, outside its checks: a build or command its agent ran". The verdict lists
   both as `checkout_written` findings, and the run summary, `roko diagnose` and the end of `roko plan run` name them
   as now.
4. The unverified rule stays as it is: an unchanged tree whose probe at the gate passes while the checkout changes.

## Done when

- A try whose own `run_tests` (`build: "checks"`) writes into the person's checkout, and which then passes at the
  gate without the step running again, is reported: the hub diagnosis, the verdict's `checkout_written` findings,
  and the run summary. Test: `a_try_s_own_checks_that_write_into_the_checkout_say_so`.
- A write by a command the agent runs itself is reported as outside its checks.
- The `[[verify]]` passes.

## Notes

- Found when bug-b7d9ad merged over gap-6df2bf (round v7 roko-speed). The lead set the direction and the priority.
- Prevention, a warm tree with no paths into the person's checkout, is gap-184cba.
