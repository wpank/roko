+++
id = "gap-3cbd8d"
kind = "gap"
title = "tests/endpoint_smoke.py has never been live-checked against /api/run's gated one-task plan path, and isn't wired into CI"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["tests"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "wave-3 follow-up reports 2026-10-02 (w3-pk73 gap-5e9292)"
discovered_from = "gap-5e9292 (PK73, domains-and-assistant runner work)"
anchors = ["tests/endpoint_smoke.py", "crates/roko-serve/src/routes/run.rs::start_run"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rq 'endpoint_smoke.py' .github/workflows/ && grep -q 'runs/.*summary' tests/endpoint_smoke.py"
+++

## Problem

`tests/endpoint_smoke.py` starts a real `roko serve`, fires one `POST /api/run`, and checks the read surfaces
(`health`, `plans`, `executor/state`, `signals`, `gates/summary`, `gates/history`, `episodes`, `knowledge`,
`statehub/snapshot`). Its fixture workspace (`write_workspace`, lines ~77-98) configures `[agent] command = "cat"`
(an echo stub, not a real model) and a single always-passing gate (`kind = "shell"`, `program = "true"`).

Nothing runs this script automatically: `grep -rl endpoint_smoke .github/ Makefile*` finds no reference anywhere.
It is a manual, human-run smoke test only.

`POST /api/run` has changed meaningfully since the script's assumptions were written: it now runs the prompt as a
full gated one-task plan through the Graph engine (`crates/roko-serve/src/routes/run.rs::start_run`, commit
`f0c15452d`, "POST /api/run runs a gated one-task plan under the id it returns"), reports an ungated answer as
`unverified` rather than `succeeded` (commit `01755a555`, which also touched this script's `wait_for_run` comment),
and gained a `GET /api/runs/{run_id}/summary` route (commit `f4ade3ffc`) that `verify_endpoints` does not call at
all. Nobody has actually run the script against a live server since these changes landed to confirm its `cat`+
`true`-gate fixture still produces the "succeeded"/"unverified" outcome `wait_for_run` expects under the current
Graph-engine-backed path, or that its loose accept-either check (`status in ("succeeded", "unverified")`,
line ~168) is still the right assertion rather than papering over a regression.

## Why it matters

Goal: tooling (test hygiene / dev-loop confidence). A smoke test nobody runs and nobody wires into CI gives false
confidence that the HTTP surfaces it lists are healthy; three behavior changes have landed underneath it unchecked.
If `/api/run`'s current gated-plan path no longer produces a clean "succeeded" or "unverified" for this trivial
fixture (e.g. because the Graph engine now expects something `write_workspace`'s minimal `roko.toml` does not
provide), nothing would notice until a person happens to run this file by hand.

## Where

- `tests/endpoint_smoke.py` (the whole file; `write_workspace`, `wait_for_run`, `verify_endpoints`).
- The route it exercises: `crates/roko-serve/src/routes/run.rs::start_run` and `GET /api/run/{id}/status`.
- The route it does not exercise: `GET /api/runs/{run_id}/summary` (`crates/roko-serve/src/routes/run.rs`, added by
  `f4ade3ffc`, mentioned in its own doc comment as "a run summary a host can post").

## Current state

Unverified against current `main`. No CI workflow or Makefile target runs it; its last real update
(`01755a555`) only adjusted the `wait_for_run` comment/acceptance for the ungated-vs-unverified change, not for the
later gated-one-task-plan change (`f0c15452d`) or the summary route (`f4ade3ffc`).

## Plan

1. Run it for real against a current build: `python3 tests/endpoint_smoke.py` (builds or reuses `target/debug/roko`
   via `cargo run` if no `--roko-bin`/`ROKO_BIN` is given). Fix whatever it reports.
2. Add a check for `GET /api/runs/{run_id}/summary` to `verify_endpoints`.
3. Wire it into some CI workflow (even a manual-trigger one) or, at minimum, document it in a developer-facing
   README section, so "nobody runs this" stops being true.
4. Reconsider the loose `status in ("succeeded", "unverified")` acceptance in `wait_for_run`: either assert the
   fixture's trivial gate always yields `succeeded` (since `program = "true"` always passes), or document precisely
   why `unverified` is also an acceptable outcome for this specific fixture.

## Done when

- The script is referenced from a CI workflow (or equivalent wired check) and covers `GET
  /api/runs/{run_id}/summary`.
- The `[[verify]]` command passes.

## Notes

- This is a Python smoke script, not a Rust crate; no `cargo test` guard applies. The verify below is a static
  check for CI wiring and summary-route coverage, not a live run — running it live is part of the Plan, for
  whoever picks this item up.
