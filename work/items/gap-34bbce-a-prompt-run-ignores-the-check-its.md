+++
id = "gap-34bbce"
kind = "gap"
title = "A prompt run ignores the check its request names: only the workspace rungs run, so a named test such as python3 -m unittest tests.test_env never runs on its own"
status = "open"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/run"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "round-v6-roko-gates#bug-5089fb-plan-step-2"
discovered_from = "agent:roko-gates (round v6, bug-5089fb, 2026-10-17)"
anchors = ["crates/roko-cli/src/run.rs::prompt_verify_steps", "crates/roko-cli/src/run.rs::prompt_tasks_file", "crates/roko-serve/src/routes/mcp.rs::tools", "crates/roko-cli/src/plan_generate/runnable.rs::problems"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-5089fb", "gap-161be1", "gap-ecafde"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn prompt_run_takes_the_check_its_request_names' crates/roko-cli/ && cargo test -p roko-cli --lib prompt_run_takes_the_check_its_request_names"
+++

## Problem

A prompt run ignores the check its request names. The round v6 run 4 request asked to "implement synth/env.py …
so that `python3 -m unittest tests.test_env` passes". The run's only checks were still the workspace's rungs: the
whole suite, `expect = "pass_on_base"`. Since bug-5089fb (merged a9f7fdf3f) a red base no longer blocks the run.
The step passes when the change adds no failure. But the check the person named never runs on its own. Expected:
the named check becomes the task's own step and fails on the base. The workspace rungs run after it as regression
checks. The run's plan names that step, so a host can show it.

## Why it matters

Goal `golden-path`. From chat, people ask for one module at a time and say which test proves it. The named test is
the sharpest acceptance check there is. "No new failures" over the whole suite is weaker: a change that adds no
failure passes even if the named test never ran. Run 4 showed the gap: the same task as a one-task plan whose own
verify was `python3 -m unittest tests.test_env` passed first try for $0.0854 (run `6a0a4818`). Related:
bug-5089fb, gap-161be1, gap-ecafde.

## Where

- `crates/roko-cli/src/run.rs::prompt_verify_steps` builds a prompt run's checks, today only the domain pack's or
  the workspace's required rungs.
- `crates/roko-cli/src/run.rs::prompt_tasks_file` writes the one-task plan; the request is fenced for a chat host
  (9117).
- `crates/roko-serve/src/routes/mcp.rs::tools` declares MCP `run_prompt`. It takes `prompt`, `title`, `domain` and
  `max_usd`, so a chat host cannot pass a check explicitly.
- `crates/roko-cli/src/plan_generate/runnable.rs::problems` already checks that a generated step can run: the
  program is installed, and the files and test ids it names exist (gap-ecafde). A check taken from a request needs
  the same test.

## Current state

- There is no extraction of a check from the request text, and no `--check` flag on `roko run`.
- Since bug-5089fb, `prompt_base_checks` runs Python rungs on the base first and keeps the run per commit. A named
  check could join that same pre-dispatch run to learn whether it fails on the base.

## Plan

1. Find a check in the request: a backticked command, or "so that <command> passes", whose runner the baseline
   judgement knows (`python3 -m unittest <ids>`, `pytest <paths>`, `cargo test -p <crate> <filter>`). A chat
   host's request is untrusted data: take only commands that `runnable::problems` accepts and that match an allow
   list of runners, never arbitrary shell.
2. Run it on the base with the rungs (`prompt_base_checks`). If it fails there, it becomes the task's first step
   with `expect = "fail_on_base"`. If it passes there, it becomes a regression step (`pass_on_base`). The rungs
   follow either way.
3. Add an explicit form: `roko run --check "<command>"`, and an optional `check` argument to MCP `run_prompt`.
4. Add the test `prompt_run_takes_the_check_its_request_names`. It uses a pocket-synth-shaped repo, a request that
   names `python3 -m unittest tests.test_env`, and a scripted agent. The task's first step must be that command,
   with `fail_on_base`; a change that passes it but breaks another test that passed before must still fail.

## Done when

- The `[[verify]]` passes.
- `roko run` on the pocket-synth seed, asked for env.py "so that python3 -m unittest tests.test_env passes", shows
  that command as the task's first step in its `tasks.toml`, and passes on it.

## Notes

- Never run a command the request names unless it passes the runnable check and the runner allow list: the request
  can come from a chat host.
- Keep the workspace rungs as the regression checks after the named step.
