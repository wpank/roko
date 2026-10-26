+++
id = "gap-c96097"
kind = "gap"
title = "The red-on-base pre-check runs before the rung probes and its cache key includes roko's build: it hit its cache in 2 of 15 round-v7 runs, and run 16fc88ae pre-checked for 134 s, then died in 7 s on a refusal the probe also got"
status = "done"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/spec_gate"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
last_verified_rev = "bb0af5a61"
source = "round-v7 run-analyst (tmp/hermes-roko-v7/findings/run-analyst.md), the lead's ask"
discovered_from = "agent:run-analyst (round v7)"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan_body", "crates/roko-cli/src/spec_red_on_base.rs::CACHE_VERSION", "crates/roko-cli/src/spec_red_on_base.rs::cache_key"]
links = { depends_on = [], blocks = [], related = ["gap-e02ec4", "gap-618a75", "bug-29014c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn the_rung_probes_run_before_the_red_on_base_pre_check' crates/roko-cli/src && grep -rqw 'fn a_new_roko_build_reuses_the_pre_check_cache' crates/roko-cli/src && cargo test -p roko-cli --lib the_rung_probes_run_before_the_red_on_base_pre_check && cargo test -p roko-cli --lib a_new_roko_build_reuses_the_pre_check_cache"

[closed]
at = 2026-10-26
at_ts = "2026-10-26T12:04:45Z"
commit = "bb0af5a61"
forced = false
evidence = "bb0af5a61: unrunnable_ladder (plan_runner.rs) probes the routing ladder before the spec gate with plan start's probes and 24 h cache, under their conditions; when every rung's model failed its probe and the ladder routes every task (none pinned by --model, model_hint or preferred_model) the run stops there in the probes' own words, before the red-on-base pre-check runs a step. spec_red_on_base: CACHE_VERSION = red-on-base/2 (the check's semantics, bumped by hand; roko's build left the key) and the base keyed by its tree (<sha>^{tree}). Verify: the_rung_probes_run_before_the_red_on_base_pre_check (every rung's cached probe refused: the run errs with each rung's reason and the pre-check's marker is never written; a pinned task, --model or --no-budget lets it go on) and a_new_roko_build_reuses_the_pre_check_cache (no build in the key; an empty commit, same tree, reuses the decided check) pass; spec_red_on_base:: and spec_gate:: 15/15; nightly fmt and clippy -p roko-cli -D warnings clean."
+++

## Problem

There are two problems with the red-on-base pre-check (`[spec_quality] red_on_base`).

1. **It runs before the rung probes.** `run_graph_plan_body` runs the pre-check at about line 1311 and probes the
   ladder's rungs (`rung_probe::probe_ladder`) at about line 1473.
   - Phase 4's first run, 16fc88ae, spent 134 s building and running 14 plugin steps on the base. It then died 7 s
     later: OpenAI refused every call (HTTP 400, reasoning_effort beside tools, bug-29014c).
   - The probe had got the same 400.
2. **Its cache key includes roko's build.** The key (gap-e02ec4) starts with `CACHE_VERSION`, which is the entry
   format plus `CARGO_PKG_VERSION` and `ROKO_GIT_HASH`, so every new roko build misses.
   - Round v7 used 8 binaries over 15 run segments, and the cache hit only twice: 82b7e069, after a
     `validate --dynamic` on the same binary, and 6059db44's resume.
   - Phase 4's two runs had the same base (6089b2a) and the same plan, so its second run (c4fa953f) ran the same
     14 plugin steps again, for 107.6 s.

In all, the pre-check took 8.2 min of the round's 481 run-minutes. A plugin plan pays 108–134 s each time.

Expected:
- A run whose start rungs can't run stops in seconds, before the pre-check.
- A new roko build reuses a decided pre-check when nothing the check depends on has changed.

## Why it matters

Golden path, speed. On a C++/JUCE plan the pre-check builds the plugin in a base checkout, and it ran again after
nearly every roko change, which in a dogfood round is every run. A doomed run should fail in the time a probe takes.

## Where

- `graph_execution/plan_runner.rs::run_graph_plan_body`: the order. The spec gate (`spec_gate::gate_plans_with`) runs
  first, at about 1311. Run manifests are captured at about 1424, and `rung_probe::probe_ladder` runs at about 1473.
- `spec_red_on_base.rs::CACHE_VERSION` and `spec_red_on_base.rs::cache_key`: the key.

## Current state

- gap-e02ec4 (merged 97d9b1819) added the cache: decided results are kept 12 h, keyed by the base, the steps, the
  cargo mode, the step environment, the plan directory's files and roko's build. It also gives
  `validate --dynamic --fresh`.
- The base commit changes with every config-only commit too (Undertow's roko.toml commits cfb8a73, c9dc249, 6089b2a),
  and those also miss.
- The rung probes are cached for 24 h in `.roko/learn/rung-probes.json` and take seconds. gap-618a75 is about making
  them stream like an attempt.

## Plan

1. Probe the rungs before the pre-check. Stop the run with the probe's own words when no rung that a task's tier
   may start on can run.
2. Replace roko's build in `CACHE_VERSION` with a pre-check logic version. Bump it by hand when the pre-check's
   semantics change, as the entry format already is.
3. Optional and harder: key on the trees of the paths the steps read, not the base commit, so config-only commits
   hit. Leave it out if it can't be made safe.

## Design

roko-exact, 2026-10-26, with the lead's call on the key.

1. **Probe first.** Before the plan-load spec gate, whose red-on-base pre-check builds and runs the tasks' steps on
   the base, `run_graph_plan_body` probes the routing ladder with plan start's own probes
   (`rung_probe::probe_ladder`, under the same conditions: `[routing.ladder] probe`, no `--no-budget`, not FAST).
   Their verdicts stand for 24 h, so the probe at plan start reads them again and probes nothing twice.
   - When every rung's model failed its probe and the ladder routes every task of the run, the run stops there,
     in the probes' own words: "no rung of the routing ladder can run, so no task of this run can start: mid-1:
     request_refused: …".
   - A task pinned to a model (`--model`, `model_hint`, `preferred_model`), or one the ladder has no start rung
     for, can still run, so then the run goes on. gap-20301d (roko-ladder) refuses the ladder's own tasks at
     dispatch.
   - The probe at plan start is unchanged; gap-20301d edits it.
2. **The cache key.** `CACHE_VERSION` becomes `red-on-base/2`: the entry format and what the check does with a
   step. It is bumped by hand when how the check runs a step or reads its result changes. roko's build leaves the
   key, so a new build reuses what an earlier one decided.
   - The base enters as its tree (`<sha>^{tree}`), not the commit, so a commit that changes no file still hits.
   - The rest stays: the steps (command, timeout, expect), the cargo mode, the step environment, the plan
     directory's other files, the 12 h time-to-live and `--fresh`.
3. **Left out:** keying on the trees of the paths the steps read (Plan point 3). A config-only commit still misses.

## Done when

- A run whose every start rung is refused stops before the pre-check runs a step. Test:
  `the_rung_probes_run_before_the_red_on_base_pre_check`.
- A pre-check decided by one roko build is reused by another build on the same base, steps and plan. Test:
  `a_new_roko_build_reuses_the_pre_check_cache`.
- The `[[verify]]` passes.

## Notes

- Keep `--fresh` and the 12 h time-to-live.
- Evidence: `tmp/hermes-roko-v7/findings/run-analyst.md`, section 3 and lever L5. The per-run pre-check times are in
  `run-analyst/data/runs.csv`.
