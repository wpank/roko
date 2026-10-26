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

## What was built (roko-speed, 2026-10-26)

- **roko-std `checkout_state`:** `CheckoutState`, moved from roko-cli's `checkout_writes`. It is a checkout's dirty
  paths from `git status --porcelain=v1 -z --untracked-files=all`, run with git's env scrubbed and a timeout, each
  with its size and mtime. `changes_since` returns `CheckoutChanges` (changed, added, touched).
- **`run_checks`, the try's own checks:** this covers both `run_tests` with `build: "checks"` and the hand-off
  check. When the attempt runs in a checkout other than the workspace root (`ctx.immune_root_path`), it snapshots
  that root before and after the checks, leaving `.roko/` out. Its `CheckRun` then records the paths that changed
  as `checkout_written`.
- **The try's window (roko-cli):**
  - The dispatcher snapshots the person's checkout when a try in its own checkout starts its dispatch, in both the
    batch and the streaming path.
  - It compares as the dispatch ends: `try_checkout_written`. A path some `CheckRun` recorded is put down to the
    checks. Any other path is put down to a build or command the agent ran itself (`outside_checks`).
  - The result is reported at once (log, hub diagnosis, end-of-run registry), since the try may end before its
    gate. Its `checkout_written` findings go on the attempt's verdict whatever the try's end.
- **The gate's window** stays as it was. When it finds a change, it reports the try's changes and its own together,
  and the hub's diagnosis is replaced by id. The unverified rule is unchanged.
- **Wording:**
  - The diagnosis says "while T's checks ran" when all of it came from checks, else "while T ran". It names the
    paths changed outside the checks, "unless you changed them yourself".
  - `roko diagnose`, the end of `roko plan run` and the API doc say "your checkout changed while it ran".
- **Tests:**
  - `a_try_s_own_checks_that_write_into_the_checkout_say_so` (roko-cli). The agent writes `a.txt`, runs its checks
    (which write `artifacts/shot.png` into the person's checkout), then writes `notes/agent.txt` there with `bash`.
    The gate takes the self-check and runs nothing. Both paths reach the hub, the verdict and the registry, with
    the note put down to the agent.
  - `a_check_run_records_what_it_changed_in_the_persons_checkout` and `a_checkout_state_comes_from_git_status`
    (roko-std).
  - The b7d9ad tests, updated.

## Done when

- A try whose own `run_tests` (`build: "checks"`) writes into the person's checkout, and which then passes at the
  gate without the step running again, is reported: the hub diagnosis, the verdict's `checkout_written` findings,
  and the run summary. Test: `a_try_s_own_checks_that_write_into_the_checkout_say_so`.
- A write by a command the agent runs itself is reported as outside its checks.
- The `[[verify]]` passes.

## Notes

- Found when bug-b7d9ad merged over gap-6df2bf (round v7 roko-speed). The lead set the direction and the priority.
- Prevention, a warm tree with no paths into the person's checkout, is gap-184cba.
