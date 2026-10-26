+++
id = "bug-fa22cc"
kind = "bug"
title = "A retry dies in pre-validation on a declared line range its own last try made stale: phase 5.1's T2#2-#4 each ended in about 100 ms on `1-121` of tools/render-demos.sh, which T2#1 had rewritten to 43 lines, and T2 was exhausted with no model call"
status = "open"
triage = "verified"
severity = "p1"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "round-v7 lead (Undertow phase 5.1 live run 814a7340)"
discovered_from = "agent:team-lead (round v7, phase 5.1)"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs::fit_declared_ranges", "crates/roko-cli/src/plan_policy.rs::render_declared_context"]
links = { depends_on = [], blocks = [], related = ["bug-5e361a", "bug-c5f406", "bug-7ff055"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_try_fits_declared_ranges_to_its_checkout' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_try_fits_declared_ranges_to_its_checkout"
+++

## Problem

A try dies in pre-validation when a declared line range runs past the end of its file as that try's checkout holds
it. It fails before any model call, and every later try of the task fails the same way. Only the ranges in files an
earlier task changed are fitted (bug-7ff055), and only for a task that has dependencies. A range in the task's own
file, which its own last try rewrote, is not fitted.

Undertow phase 5.1's live run `814a7340` (plan `p5-1-the-renders-as-they-play`) shows it. T2, "Make the offline
renderer play like the engine", declares `tools/render-demos.sh` both in its `files` and in its
`context.read_files` with `lines = "1-121"`. In Undertow's `.roko/events.jsonl`, around 17:14:44Z:

- **T2#1** rewrote `tools/render-demos.sh` from 121 lines to 43, removing the boost as the spec asks. It failed its
  gate on `render_test.cpp:130` (gate_failed, 203 s).
- **T2#2, T2#3 and T2#4** reused T2#1's checkout (`workspace.reused = true`). Each ended in about 100 ms, with no
  model call: "harness: planning error: pre-validation failed: line range `1-121` is outside
  `tools/render-demos.sh` (43 lines)". The verdicts were harness_error with blame harness, at $0.
- **T2 was exhausted**, and the run failed with T1 and T3 passed: "plan 'p5-1-the-renders-as-they-play': task 'T2'
  failed: planning error: pre-validation failed: …".

**Expected:** every try (the first, a retry, a hand-back) reads its declared ranges against the file as its own
checkout holds it:
- a range from line 1 that runs past the end becomes the whole file;
- any other range that runs past the end is cut at the file's last line;
- a range that starts past the end becomes the whole file.

A range past the end never fails pre-validation.

## Why it matters

- It failed phase 5.1's live run, with no fault of the agent. A try that shortens its own declared file, which a
  task that removes code does, can never be retried.
- Each dead try counts toward the task's retries, so the task is exhausted without another model call.
- The same defect hit a plan-check hand-back's try in phase 5's rehearsal (bug-5e361a, fixed for hand-backs alone).

## Where

- `crates/roko-cli/src/graph_task_dispatch.rs::fit_declared_ranges`. It runs per try in `plan_dispatch`, but returns
  early for a task without `depends_on` and fits only files an earlier task changes
  (`plan_policy::fit_ranges_to_earlier_changes`, replaced by `fit_ranges_to_checkout`).
- `crates/roko-cli/src/plan_policy.rs::render_declared_context` and `render_declared_files` check each range against
  the file at prompt assembly. Their "line range … is outside …" error becomes
  `RunnerDispatchError::PreValidationFailed` (`dispatch/prompt_builder.rs::declared_files`).
- `plan_policy::fit_range` already cuts a range at the file's end, and makes one that starts past it the whole
  file.

## Plan

1. **Fit on every try.** `fit_declared_ranges` fits every declared range of the try's task to the file in its
   checkout (`plan_policy::fit_ranges_to_checkout`), whatever task changed the file and whether or not the task has
   dependencies. It logs each range it changes, as now.
2. **A whole-file range.** `fit_range` makes a range from line 1 that runs past the end the whole file (unranged),
   so the prompt shows the file as it is now, within the policy's line limit.
3. **Never fail on a range past the end.** `render_declared_context` and `render_declared_files` fit such a range as
   `fit_range` does, rather than fail the try. Only a malformed range (line 0, reversed, unparsable) still fails.
   This covers every path that assembles a prompt, not only the Graph dispatcher.
4. The plan check at load (`validate_plan_context`) is unchanged. A range past the end of a file no earlier task
   changes is still a planning error before anything runs. Plan generation fits those ranges first (gap-b95913).

## Done when

- The test in `[[verify]]` passes: a try whose checkout holds a 43-line `tools/render-demos.sh`, declared as `1-121`,
  runs, and its prompt shows the whole file.
- A range past the end of a file no earlier task changed renders fitted instead of failing.

## Notes

- **Related:**
  - bug-5e361a: the hand-back's case, fixed by reading a plan-changed file whole;
  - bug-c5f406: roko-ladder's prompt-side note on declared ranges;
  - bug-7ff055: fitting for files an earlier task changed.
- roko-exact takes the pre-validation part; roko-ladder keeps the prompt-side range notes (bug-c5f406).
