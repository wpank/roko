+++
id = "bug-11901c"
kind = "bug"
title = "Planner-written checks pipe into grep -q, which under the verifier's pipefail fails at random when the producer gets a broken pipe (exit 120), so passing work is retried until the cap"
status = "done"
triage = "verified"
severity = "p1"
goal = "core"
size = "M"
subsystem = ["roko-gate/verify_shell", "roko-cli/graph_task_dispatch", "roko-cli/plan_generate"]
created = 2026-10-13
updated = 2026-10-15
last_verified = 2026-10-15
last_verified_rev = "a96491532"
source = "hermes-plugin-roko live end-to-end, 2026-10-13 (roko serve 1e3419a51)"
anchors = ["crates/roko-gate/src/verify_shell.rs::drain_early_readers", "crates/roko-gate/src/shell.rs::verify_step", "crates/roko-cli/src/graph_task_dispatch/verification.rs", "crates/roko-cli/src/graph_task_dispatch/verify_depth.rs", "crates/roko-cli/src/graph_task_dispatch/baseline_verify.rs", "crates/roko-cli/src/runner/gate_dispatch.rs::verify_step_gate", "crates/roko-cli/src/spec_red_on_base.rs::run_step", "crates/roko-cli/src/plan_generate.rs::PLAN_GENERATOR_SYSTEM_PROMPT"]
links = { depends_on = [], blocks = [], related = ["bug-6fa025", "gap-1f08f4"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_reader_that_stops_early_no_longer_fails_the_step' crates/roko-gate/src && grep -rqw 'fn graph_verify_passes_when_a_reader_stops_early' crates/roko-cli/src && grep -rqw 'fn generator_prompt_makes_a_test_runner_its_own_check' crates/roko-cli/src && cargo test -p roko-gate -p roko-cli --lib -- verify_shell:: graph_verify_passes_when_a_reader_stops_early generator_prompt_makes_a_test_runner_its_own_check"

[closed]
at = 2026-10-15
at_ts = "2026-10-15T16:20:03Z"
commit = "a96491532"
by = "roko-checks"
executor = "claude-agent"
via = "manual"
size = "M"
claimed_at = "2026-10-15T14:00:30Z"
forced = false
evidence = "a96491532: verify steps drain early readers (roko_gate::verify_shell, ShellGate::verify_step) on every pipefail verify path; the planner prompt says a test runner's exit status is its check. CLI: the planner-style 'python3 -m unittest ... -v 2>&1 | grep -q OK' plan passed 8 of 8 runs on the first attempt with the fix (main 7b6a5af44: 8 of 15 attempts failed with exit 120, 1 run failed outright). Tests: verify_shell:: (7), graph_verify_passes_when_a_reader_stops_early, generator_prompt_makes_a_test_runner_its_own_check; roko-cli lib 3630 passed, roko-gate lib 687 and its 6 integration test binaries passed; clippy -D warnings and nightly fmt clean on roko-gate and roko-cli."
+++

## Problem

Graph verification runs every verify step as `bash -o pipefail -c <command>`. A step that pipes a command into a
reader that stops reading early (`grep -q` at its first match, `head` after its lines, `grep -m1`, `sed q`) closes
the pipe while the writer may still be writing. The writer then dies of SIGPIPE (141) or fails its write (Python
exits 120 or 1, cargo 101), `pipefail` fails the step, and the check fails although what it looked for was there.
Whether it does depends on timing, so passing work is failed and retried until the cap.

Seen in the Hermes plugin's live run (2026-10-13, plan `project-tally-a-small-python-cli-expense-ledger-2`): T3's
check `python3 -m unittest tests.test_budgets -v 2>&1 | grep -q 'OK'` failed five times, spent $0.89 of the run's
$1.00 cap, and the run was cancelled at 39 minutes.

Reproduced 2026-10-15 at 7b6a5af44 with `/bin/bash` 3.2 and bash 5.3 under `-o pipefail`:

- that Python check, on a test that prints 2000 lines: 29 of 40 runs fail with exit 120;
- the planner prompt's own pass-count idiom, `cargo test new_test 2>&1 | grep -q 'ok. 1 passed'`, on a crate with
  a later test binary: 19 of 20 runs fail with exit 101;
- `{ echo OK; seq 1 200000; } | grep -q OK`: 100 of 100 runs fail with 141;
- `plans/demos/demo-full-stack` checks `cargo build --quiet 2>&1 | head -20`, which fails the same way once the
  build prints more than 20 lines.

## Why it matters

A check that fails at random blames the agent for passing work, wastes the retry budget and can sink a plan,
often a person's first plan (goal `core`). The planner writes the pattern by default, and its own system prompt
teaches it (`cargo test -p x new_test 2>&1 | grep -q 'ok. 1 passed'`).

## Where

- `crates/roko-gate/src/verify_shell.rs`: `drain_early_readers` and `bash_args`, how a verify step's command runs.
- `crates/roko-gate/src/shell.rs::verify_step`: the `ShellGate` for one verify step.
- Every place that ran `bash -o pipefail -c <command>` for a plan's verify step: `graph_task_dispatch/verification.rs`
  (the first run and the post-auto-fix re-run), `verify_depth.rs`, `baseline_verify.rs`,
  `runner/gate_dispatch.rs::verify_step_gate`, `spec_red_on_base.rs::run_step`.
- `crates/roko-cli/src/plan_generate.rs::PLAN_GENERATOR_SYSTEM_PROMPT`: the planner's verify guidance.

## Current state

Fixed on `work/bug-11901c`. Each reader a pipe feeds (`grep`, `egrep`, `fgrep`, `rg`, `ag`, `head`, `sed`, the awks)
runs through a shell function defined in front of the command. Once the reader exits, the function leaves a
background `head -c` that reads and discards what the writer still sends, up to 64 MiB, and returns the reader's
status. The writer runs to its end, so the step fails only when the writer fails or the check does. Because the
drain runs in the background, a process the writer leaves running with the pipe open does not hold the step up;
because it is bounded, a writer that never ends (`yes | head -1`) fails at once, as before. The scan that finds
the readers follows quoting, `$(…)`, `${…}`, arithmetic, comments, here-documents, subshells and `[[ ]]`; a command
with syntax it does not follow (`case`, backquotes, extended globs, arrays, function definitions) runs unchanged.
Commands with no piped reader run byte for byte as before.

## Plan

1. Drain early readers at the one place every verify step runs (done): this covers planner-written,
   hand-written and existing plans alike.
2. Tell the planner that a test runner's exit status is its check (done): `| grep -q OK` after a runner adds
   nothing and hides the runner's output from the retry prompt.
3. Not done, on purpose: a `roko plan validate` warning on `| grep -q`. Once the verifier drains the reader the
   pattern no longer fails, so the warning would be noise.

## Done when

- A step that pipes a long writer into `grep -q` passes through the Graph verifier, every run, and a writer that
  fails after the match still fails the step (`graph_verify_passes_when_a_reader_stops_early`).
- The scan's unit tests in `roko-gate` (`verify_shell::`) pass: which readers it drains, what it leaves alone,
  the bash behaviour (the undrained command fails, the drained one passes 20 runs in 20, endless writers and
  leftover processes do not hang).
- The planner prompt carries the test-runner rule (`generator_prompt_makes_a_test_runner_its_own_check`).
- `[[verify]]` passes.

## Notes

- Only the `pipefail` verify paths change. `roko-gate`'s production service and `[[gates.rungs]]` run
  `bash -c` / `sh -c` without `pipefail`, where a cut-off writer does not fail the command.
- Related: bug-6fa025 (exit 127 from a missing tool reported as the agent's failure), gap-1f08f4.

## Original notes

## Problem

In the Hermes plugin's live end-to-end run (2026-10-13, roko serve built from `1e3419a51`, plan
`project-tally-a-small-python-cli-expense-ledger-2`, run `b7bca0f3…` started from the desktop UI with a $1.00 cap),
the planner-written check of T3 was a pipeline into `grep -q`:

```
python3 -m unittest tests.test_budgets -v 2>&1 | grep -q 'OK'
```

Graph verification runs every step as `bash -o pipefail -c <command>`. `grep -q` exits at its first match and
closes the pipe; when Python still has output to flush it dies writing to the closed pipe and exits 120, and with
`pipefail` that exit fails the step even though the text it looked for was there. Whether the check passes depends
on timing.

T3 retried five times on this check (each attempt told the agent its work failed a check), spent $0.89 of the run's
$1.00 cap, and stalled until the person cancelled the run at 39 minutes. The planner wrote the same pattern into the
dogfood plan earlier the same day (`… -v 2>&1 | grep -q 'OK'`, and `grep -q … && python3 …` chains).

## Why it matters

A check that fails at random wastes money on retries, blames the agent for passing work, and can sink a plan. The
planner writes this pattern by default, so most generated plans for Python projects carry it (goal `core`).

## Where

- `crates/roko-cli/src/graph_task_dispatch/verification.rs`: runs steps with `bash -o pipefail -c`.
- `crates/roko-cli/src/plan_generate/`: the planner's guidance for verify commands.
- `roko plan validate` (plan lint): could flag the pattern before a run.

## Current state

Nothing warns about a pipeline whose consumer exits early under `pipefail`; the planner is not told to avoid it.

## Plan

1. Tell the planner to write checks without early-exiting consumers in a pipeline: run the test command on its
   own (its exit status is the check), or capture output first (`out=$(…); grep -q … <<<"$out"`).
2. Make `roko plan validate` (and the spec gate) flag `| grep -q`, `| head` and similar after a producer, with the
   rewrite.
3. Optionally, when a step fails with exit 120 or 141 from a producer while the consumer succeeded, report it as a
   check defect rather than the agent's failure (see bug-6fa025 for exit 127).

## Done when

`[[verify]]` passes: plan lint warns on a `… | grep -q …` verify command, and the planner's prompt carries the rule.

2026-10-15 (status-quo assessment, finding 9): still open at b42578bab. No commit since 2026-10-13 touches `graph_task_dispatch/verification.rs` or `plan_generate/`, and no other item links here. E2a's rows AU2, EX6, QA1 and RC5 (`tmp/status-quo/2026-10-15/programme/E2a-matrix-retag-AU-EX-IS-QA-RC.md`) still list it: there is no lint for `| grep -q` under pipefail, and run `b7bca0f3` sat at $0.89 of its $1.00 cap until it was cancelled. Decision 5 (2026-10-15) funds the golden-path live reruns (gap-c013c5, gap-f30b8e). Planner-written checks that fail at random would make that spend unreadable, so this should land first.
