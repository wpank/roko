+++
id = "gap-20301d"
kind = "gap"
title = "A rung skipped at plan start for a failed probe fires no event and is missing from the run summary, and with every rung skipped a task the ladder routes goes to a model outside it without a word (phase 4: cheap's Cerebras 402; the pilot's fenced check sent T1 to glm-4.7 on z.ai)"
status = "open"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/dispatch", "roko-serve/routes"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "round-v7-lead#rung-skipped"
discovered_from = "agent:team-lead (round v7, 2026-10-21: phase 4's plan drops cheap on Cerebras' 402); agent:hermes-pilot's fenced check tmp/hermes-roko-v7/pilot/gpt54-proof/checks/20261021-101949"
anchors = ["crates/roko-cli/src/dispatch/rung_probe.rs", "crates/roko-cli/src/dispatch/factory.rs::skip_failed_rungs", "crates/roko-cli/src/dispatch/model_routing.rs::without_models", "crates/roko-cli/src/graph_execution/plan_runner.rs", "crates/roko-core/src/dashboard_snapshot.rs", "crates/roko-serve/src/routes/runs.rs::run_summary"]
links = { depends_on = [], blocks = [], related = ["gap-3217a0", "bug-390e4b", "bug-f9ca2f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_failed_probe_skips_its_rung_with_an_event' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_failed_probe_skips_its_rung_with_an_event a_task_the_ladder_routes_is_refused_when_every_rung_failed_its_probe a_skipped_rung_says_why_and_until_when"

[[verify]]
command = "grep -rqw 'fn a_runs_summary_lists_its_skipped_rungs' crates/roko-serve/src && cargo test -p roko-serve --lib -- a_runs_summary_lists_its_skipped_rungs"
+++

## Problem

At plan start, roko probes each API rung of the routing ladder with one tool-use call (decision 1119, backlog 1121).
It keeps the verdicts in `.roko/learn/rung-probes.json` for 24 h, and `skip_failed_rungs` drops a rung whose model
has a fresh failed probe. Two things go wrong after that.

**A skipped rung is silent.** roko only logs a tracing line:
- No event fires, so a client still draws the rung as live.
- The run summary doesn't list it.
- Right now this is the normal case, not an edge case. Phase 4.2 and phase 5 run on OpenAI only (mid gpt-5.4-mini,
  strong gpt-5.4): z.ai is out of balance (1113), and Cerebras refuses with 402. So every run skips cheap, for 24 h
  after each probe. Phase 4's verdict was fresh until 20:33 on 2026-10-21.

**With every rung skipped, a task goes outside the ladder.** roko drops the ladder, and "the router picks" a model for
each task the ladder would have routed. Nothing says so.
- The pilot's fenced check on the broken effort binary (`tmp/hermes-roko-v7/pilot/gpt54-proof/checks/20261021-101949`)
  shows it. All three probes failed: gpt-5.4 and gpt-5.4-mini on OpenAI's 400 (reasoning_effort beside function
  tools), and gpt-oss-120b on the fence.
- T1 then ran on glm-4.7 on z.ai ("Rung (no rung) (glm-4.7, implementer)") and ended `provider_error`.
- In a real run, z.ai is out of balance, so the task would fail on a provider the plan's ladder never named.

## Where

- `crates/roko-cli/src/dispatch/rung_probe.rs`: `probe_ladder`, `RungProbes`, `PROBE_TTL_MS`.
- `crates/roko-cli/src/dispatch/factory.rs`: `skip_failed_rungs`.
- `crates/roko-cli/src/dispatch/model_routing.rs`: `RoutingLadder::without_models`.
- `crates/roko-cli/src/graph_execution/plan_runner.rs`: the probe at plan start.
- `crates/roko-cli/src/graph_task_dispatch.rs`: `plan_dispatch`, where an attempt is refused before its agent runs.
- `crates/roko-core/src/dashboard_snapshot.rs`: `DashboardEvent`.
- `crates/roko-serve/src/routes/runs.rs`: `run_summary`.

## Plan

1. **The event.** Once the plan set is announced, publish a `rung_skipped` event for each plan of the run and each
   skipped rung. Its fields:
   - `plan_id`, `rung`, `model` and `provider`;
   - `reason`: the probe's own words;
   - `until_ms`: the probe's `probed_at_ms` plus `PROBE_TTL_MS`;
   - `detail`: the finding in plain words, e.g. "rung cheap (gpt-oss-120b on cerebras) is skipped until 20:33: its
     tool-use probe failed: …". The CLI's progress lines and the hub's event log show the detail.
2. **The summary.** The run summary gets `rungs_skipped`: one entry per plan and rung, from the latest event of each.
3. **Every rung skipped.** roko refuses a task the ladder would route, before any model call, rather than sending it
   to a model outside the ladder.
   - The refusal is `RokoError::Rejected`, so it isn't retried, and a resume runs the task again.
   - Its error names each rung and why: "no rung of the routing ladder can run: cheap (gpt-oss-120b on cerebras):
     …; …".
   - These tasks still run as before: a pinned task (`--model`, express routing, `model_hint`, `preferred_model`),
     and a task whose role the ladder doesn't route.
   - **Why refuse rather than route outside the ladder with an event:** the ladder is the operator's statement of
     which models a task may use. A model outside it may be unfunded (z.ai) or unsuited, and an event after the
     fact doesn't stop the spend. The refusal says what to fix: the providers, or a pinned model.

4. **The run's readout.** The end-of-run lines list each skipped rung after the rungs that ran: "Rung cheap
   (gpt-oss-120b on cerebras): skipped until 20:33, its tool-use probe failed: …". `--json` output carries
   `rungs_skipped`. Right now a run that skips cheap is the normal case, so the readout says so plainly.

## Done when

- A fixture probe cache with one fresh failed verdict and one fresh passing verdict gives one `rung_skipped` event
  and one `rungs_skipped` entry. The passing verdict gives none.
- With every rung's probe failed, roko refuses a task the ladder routes, with no model call, and its error names
  each rung's reason. A pinned task still runs.
