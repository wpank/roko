+++
id = "bug-5089fb"
kind = "bug"
title = "A prompt run's only check is the workspace's whole test suite, marked pass_on_base and never run on the base, so on a project whose suite is red before the change the run cannot pass and spends its cap"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/run"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "round-v6-dogfood-will#R4-1"
discovered_from = "agent:dogfood-will (round v6 run 4, 2026-10-17)"
anchors = ["crates/roko-cli/src/run.rs::prompt_verify_steps", "crates/roko-cli/src/spec_red_on_base.rs::check_plans"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-e2d2e5"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn prompt_run_on_a_red_base_judges_only_new_failures' crates/roko-cli/ && cargo test -p roko-cli --lib prompt_run_on_a_red_base_judges_only_new_failures"
+++

## Problem

A prompt run (`roko run "<prompt>"`, serve's `run_prompt`, which Hermes' `roko_run` calls) gets exactly one kind of
check: the workspace's required gate rungs, each marked `expect = "pass_on_base"` ("the workspace passes them before
the change too"). Nothing checks that assumption. On a project whose suite is red before the change, the run can't
pass however good its work is, and it spends its cap finding that out.

Round v6 dogfood run 4 (2026-10-17, roko b2970cb12) showed this. The project was the pocket-synth showcase seed:
`synth/osc.py`, `svf.py`, `env.py` and `render.py` are not written yet, so `tests/test_osc.py` and the other module
tests fail to import.

From Hermes chat: "use roko to implement synth/env.py as section 3 of specs/pocket-synth.md specifies … so that
python3 -m unittest tests.test_env passes. … Cap it at $0.20." This was run `b92daddc`.
- The generated task's only check was `verify[0:test]`, `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s
  tests -q`, with `expect = "pass_on_base"`. That is the python profile's workspace rung. The test the request named
  was ignored.
- kimi-k2.6 wrote an `env.py` that passes `tests.test_env` (4/4, checked by hand on the kept attempt worktree). The
  check still failed: `ImportError: Failed to import test module: test_osc`, 3 errors in 7 tests.
- The attempt cost $0.1067 of the $0.20 cap, so the budget stopped the retry. Hermes told the person the run "stopped
  at the $0.20 cap … last failed check verify[0:test] … ImportError … test_osc", correctly but uselessly.

The same task as a one-task plan, whose own verify is `python3 -m unittest tests.test_env`, passed first try on the
same model for $0.0854 (run `6a0a4818`).

## Why it matters

Goal `golden-path`. Half-built projects are exactly where people ask for one module at a time from chat, and their
suites are often red before the change. A prompt run there is wasted money with a misleading verdict ("the agent's
check failed"). The red-on-base check that guards plan tasks skips these steps on purpose (3231).

## Where

- `crates/roko-cli/src/run.rs::prompt_verify_steps` builds a prompt run's checks: the domain pack's or the workspace's
  required rungs, every one `expect = "pass_on_base"`. No base check, and nothing from the request.
- `crates/roko-cli/src/spec_red_on_base.rs::check_plans` runs a task's steps on the base, but treats an
  all-`pass_on_base` task as "not run: every step is a regression check".
- The prompt run's task file is written by `crates/roko-cli/src/run.rs::prompt_tasks_file`.

## Current state

- No base check runs for prompt runs.
- `run_prompt` (MCP) takes only `prompt`, `title`, `domain` and `max_usd`, so a chat host cannot name the check.
- Evidence:
  - the run: `tmp/hermes-roko-v6/dogfood-will/evidence/run4/` (the run's `tasks.toml` and `attempts.jsonl`);
  - the verdict summary: `tmp/hermes-roko-v6/dogfood-will/logs/run4-summary-b92daddc-7fca-459c-9.json`;
  - the chat: `logs/g5b-chat.txt`;
  - the finding: `tmp/hermes-roko-v6/findings/dogfood-will.md`, Run 4.

## Plan

1. Before the first attempt, run the prompt run's rungs once on the base (as the red-on-base check does, in a scratch
   checkout). If one fails there, do not spend. Two possible responses:
   - stop with a clear refusal: "the workspace's test rung already fails before any change: <first error>";
   - or keep the rung but judge it per test: only tests that passed on the base must pass, plus a check scoped to
     what the request names.
2. When the request names a check command (`python3 -m unittest tests.test_env`, `cargo test -p x name`, `pytest
   path`), take it as the task's own `fail_on_base` step, with the workspace rungs as regression checks after it. Name
   it in the run's plan so a host can show it.
3. Add the test `prompt_run_on_a_red_base_stops_before_spending`: a workspace whose test rung fails on the base gets a
   refusal (or a scoped check), and no agent dispatch.

## Done when

- The `[[verify]]` passes.
- A prompt run against the pocket-synth seed either refuses before spending or passes on its scoped check.

## Notes

- 2026-10-17, the fix (roko-gates, per the round v6 lead): no refusal, since the change may be what turns the base
  green. A prompt run runs its Python test checks (unittest, pytest) on the base commit first, in a scratch checkout,
  and keeps the run per base commit (`.roko/state/baselines/<commit>.json`), where the gap-161be1 judgement of a
  failed step finds it. A green base keeps today's rule. On a red base the step passes when the change adds no
  failure: tests that passed on the base still pass, tests the change lets run (a module that now imports) or edits
  pass, a test module that failed to load and that the change touches loads, and some test passes; the settlement
  names the rest ("3 tests already failed before the change: test_osc, test_render, test_svf") and the attempt
  settles as `passed_with_preexisting_failures`. A base whose suite names no failure (pytest stopped at collection)
  is judged by the test files the change adds or touches, run alone; with none, the step fails. The agent is told
  which tests already fail (a `[task.context]` note), and the run publishes a `run.base_check` event. Cargo suites
  are left to the lazy judgement (a cold base build costs more than it saves). Plan step 2 (a check the request
  names becomes the task's own step) is not done.
- Keep the workspace rungs as regression checks where the base is green: that is what they are for.
- Related: bug-0a7ec1 (lessons), gap-e2d2e5 (phases).
