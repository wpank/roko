+++
id = "bug-59ed90"
kind = "bug"
title = "Planner-written checks discard the output that explains a failure, and the retry prompt drops it too, so neither the retrying agent nor the person sees why a check failed"
status = "done"
triage = "verified"
severity = "p1"
goal = "core"
size = "M"
subsystem = ["roko-cli/plan_generate", "roko-gate/spec_quality", "roko-cli/plan_validate", "roko-cli/dispatch"]
created = 2026-10-15
updated = 2026-10-15
last_verified_rev = "b6581dc62"
last_verified = 2026-10-15
source = "tmp/hermes-roko-v5/findings/fresh-guide.md item 28 (2026-10-15, roko main 7b6a5af44)"
anchors = ["crates/roko-cli/src/plan_generate.rs::PLAN_GENERATOR_SYSTEM_PROMPT", "crates/roko-gate/src/spec_quality/shell.rs::hidden_runner_output", "crates/roko-cli/src/plan_validate.rs::validate_tasks_file", "crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan", "crates/roko-cli/src/dispatch/prompt_builder.rs::render_gate_feedback"]
links = { depends_on = [], blocks = [], related = ["bug-11901c", "bug-6fa025", "spec-e57870", "bug-46c04b", "gap-5420f3", "gap-1f08f4"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_test_runners_hidden_output_is_flagged' crates/roko-gate/src && grep -rqw 'fn plan_validate_warns_on_a_check_that_hides_its_runner_output' crates/roko-cli/src && grep -rqw 'fn generator_prompt_runs_test_runners_plainly' crates/roko-cli/src && grep -rqw 'fn generation_asks_once_more_when_a_check_hides_its_runner_output' crates/roko-cli/src && grep -rqw 'fn retry_prompt_keeps_a_failed_tests_output' crates/roko-cli/src && grep -rqw 'fn a_check_that_hides_its_runner_names_the_runner' crates/roko-cli/src && grep -rqw 'fn a_failed_planner_call_says_what_the_planner_printed' crates/roko-cli/src && cargo test -p roko-gate -p roko-cli --lib -- a_test_runners_hidden_output_is_flagged plan_validate_warns_on_a_check_that_hides_its_runner_output generator_prompt_runs_test_runners_plainly generation_asks_once_more_when_a_check_hides_its_runner_output retry_prompt_keeps_a_failed_tests_output a_check_that_hides_its_runner_names_the_runner a_failed_planner_call_says_what_the_planner_printed"

[closed]
at = 2026-10-15
at_ts = "2026-10-15T21:36:13Z"
commit = "b6581dc62"
by = "roko-planner"
executor = "claude-agent"
via = "manual"
forced = false
evidence = "b6581dc62: roko-gate spec_quality::hidden_runner_output finds a test runner whose output a step pipes, redirects or captures; roko plan validate warns PLAN_050 (verify steps, [task.accept] runners, [[meta.verify]]); plan generation re-asks the planner once and keeps the plan that hides fewer; the planner prompt teaches a grep guard or a named test instead of the pass-count pipe; the retry prompt keeps a failed test's raw output unless compile errors explain it, and a check that hid its runner's output names the runner to run; a failing planner call quotes what it printed. The active plans are PLAN_050-free. Checked: the item's [[verify]] tests, roko-gate 690 and roko-cli 3638 lib tests, clippy -D warnings, fmt; live: plan validate flags item 28's plan, the scripted retry demo shows the AssertionError and the runner note in the retry prompt, gpt-5.4-mini now writes a grep guard plus the plain runner (findings: tmp/hermes-roko-v5/findings/roko-planner.md)."
+++

## Problem

When a check fails, the output that explains why can be lost before it reaches the retrying agent or the person.

Seen by a newcomer on 2026-10-15 (roko `main` 7b6a5af44, tmp/hermes-roko-v5/findings/fresh-guide.md, item 28). In
a tiny Python project the planner wrote this check for task T2 of run `4363bc38`:

```
python3 -m unittest tests.test_cli 2>&1 | grep -E '^Ran [0-9]+ test' | awk '{if ($2 >= 11) exit 0; else exit 1}'
```

- It pipes every line the tests print into `grep` and `awk`, so the step prints nothing. The inspect view said
  only "✗ exit status 1", and each of the four retries got no failure text either; the agent kept answering that
  it had "implemented additional unittest coverage".
- It guesses a test count (at least 11). An earlier attempt failed only because it wrote 7 tests.
- The real cause, found by running the tests by hand in a copy of the attempt's worktree: one test failed
  (`test_json_flag_for_stdin`: `{'count': 1} != {'count': 3}`).

Two things lose the output:

1. **The planner writes such checks, and its system prompt teaches them.** Principle 6 and both worked examples
   check `cargo test -p x new_test 2>&1 | grep -q 'ok. 1 passed'`: a pass count read from piped output, used so a
   `fail_on_base` step fails while the new test is missing. Nothing flags a check that pipes, redirects or
   captures a test runner's output, in generation or in `roko plan validate`.
2. **The retry prompt drops a plain runner's output too.** `render_gate_feedback` shows the "Raw gate output"
   excerpt only when `GateFeedback::from_raw` found no compile error, test failure or warning line. A line with
   "test" and "failed" counts as a test failure, so a step whose authored `fail_msg` says, as the planner's own
   example does, "The exact health endpoint integration test failed or was not found", hides the output it ran
   with (the assertion that failed), and so does a pytest summary line. The retry sees a one-line headline.

## Why it matters

A check that fails without saying why wastes every retry (the agent cannot fix what it cannot see), spends the
budget, and leaves the person unable to tell broken work from a broken check, often on their first plan (goal
`core`). bug-11901c (the pipefail race of `| grep -q`) is another defect of the same checks; its fix drains the
reader so such checks stop failing at random, but the output is still discarded.

## Where

- `crates/roko-cli/src/plan_generate.rs::PLAN_GENERATOR_SYSTEM_PROMPT`: principle 6, the T2 example, the complete
  health-check example and the self-check list teach the pass-count pipe.
- `crates/roko-gate/src/spec_quality/shell.rs`: the verify-step shell analysis that plan lint and generation share
  (`parse_shell`, `classify`); it drops redirections today.
- `crates/roko-cli/src/plan_validate.rs::validate_tasks_file`: `roko plan validate`'s per-step lints (PLAN_042 is
  the nearest neighbour).
- `crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan`: generation-time validation and the one
  regeneration a weak plan gets.
- `crates/roko-cli/src/dispatch/prompt_builder.rs::render_gate_feedback`: what of a failure the retry prompt shows.
- `crates/roko-cli/src/graph_task_dispatch/verification.rs`: where a failed step's output becomes the retry's
  `GateFeedback`.

## Current state

Open at a8e27e4b7. Nothing flags a check that hides its runner's output; the planner prompt recommends it. No
mechanism re-runs or recovers the output of a check that failed silently.

## Plan

1. Planner prompt: a test runner's exit status is the check, so run the project's test command plainly. To make it
   fail while a new test is missing, name the test (`python3 -m unittest tests.test_x.XTests.test_new`,
   `pytest tests/test_x.py::test_new`) or put a static check first
   (`grep -qw 'fn new_test' crates/x/src/lib.rs && cargo test -p x new_test`). No test-count thresholds; never pipe
   or redirect a runner's output away; extra assertions are their own `&&` commands, not filters on the output.
2. One detector in `spec_quality::shell` for a step that pipes a test runner's output into another program (other
   than `tee`/`cat`), sends it to a file or `/dev/null`, or captures it in `$(…)`. `roko plan validate` warns
   (PLAN_050, so `--strict` rejects it), and generation asks the planner once more to rewrite such steps, keeping
   the plan that hides less.
3. The retry prompt shows the raw output excerpt unless compile errors already explain the failure.
4. A failed step that printed nothing and hides a runner's output says so, and names the runner command that shows
   why, for the retry and the person. Actually re-running that command is a separate item.
5. Investigate the planner model for a newcomer without the `claude` CLI (CLI `plan generate`, `/mcp`
   `plan_generate`); fix if small, otherwise file. Found (2026-10-15): both paths already fall back to a runnable
   API model; a failing planner call hid the provider's message ("Agent crashed for an unknown reason"), which
   this item fixes (`pipeline.rs::agent_said`); the fallback takes the first runnable model rather than the
   strongest, filed as bug-46c04b. Re-running a hidden runner to recover its failure text is gap-5420f3.

## Done when

- The planner prompt carries no pass-count pipe and teaches the plain runner with a named test or a static guard.
- `roko plan validate` warns on the item-28 check, the old prompt idiom and a `[task.accept]` runner piped into
  `tail`, and stays quiet on plain runners, `&&`-guarded runners and `| tee`.
- Generation re-asks the planner when a written check hides its runner's output.
- A failed plain-runner check's output reaches the retry prompt, even with a `fail_msg` that mentions a failed test.
- A failed silent check's retry prompt says where its output went and which command shows it.
- A plan generation whose planner call keeps failing quotes what the planner printed.
- `[[verify]]` passes.

## Notes

- roko-checks is changing the verifier for bug-11901c (`verification.rs`, `verify_depth.rs`, `baseline_verify.rs`,
  `runner/gate_dispatch.rs`, `spec_red_on_base.rs`, `task_accept.rs`, roko-gate `shell.rs`/`lib.rs`/
  `verify_shell.rs`) and adds a test-runner line to the planner prompt. Coordinated by message (2026-10-15): build
  on its merge, do not edit those files before it lands.
- `[task.accept]` entries pin an exact pass count on purpose: roko tees the runner's output and counts the passing
  tests itself (`task_accept::pinned_verify_step`). So the runner should be the plain test command; item 28's
  plan piped it into `tail -5`, which keeps the count lines and drops the traceback.
