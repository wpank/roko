+++
id = "bug-2866df"
kind = "bug"
title = "A resumed run's summary has no attempts: the serve mints a new run id (dfba45e8) while the Graph run's attempts continue under the old one (bdc4461e), so rungs[] is empty for a run whose four tries cost $0.52"
status = "open"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-serve/routes"]
created = 2026-10-21
updated = 2026-10-21
last_verified = 2026-10-21
source = "round-v7-hermes-pilot#M-p3-resume-dfba45e8"
discovered_from = "agent:hermes-pilot (round v7 phase 3 resume dfba45e8)"
anchors = ["crates/roko-serve/src/routes/runs.rs::get_run_summary"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_resumed_runs_summary_shows_its_tries' crates/roko-serve/src && cargo test -p roko-serve --lib -- a_resumed_runs_summary_shows_its_tries"
+++

## Problem

A resumed run's summary has no attempts. The serve gives the resume a new run id, but the Graph run, its attempts and
its kept checkouts continue under the original id. So `GET /api/runs/<new id>/summary` reads the run's events
without the attempt records, and its per-rung readout is empty.

Round v7, Undertow phase 3 (roko e4643dafd). The chat resumed `p3-undertow-hypnotic-sequencer` with
`roko_plan_run {resume: true}`:

- The serve's run id was `dfba45e8-5e7a-4b9c-a1f2-8c389ec81703`, and the run's events carry it.
- `.roko/runs/bdc4461e…/attempts.jsonl` gained attempts 5–8 under `run_id: bdc4461e…`. roko.log: "re-attached the
  attempt checkout an earlier process of the run kept … run_id=bdc4461e…". There is no `.roko/runs/dfba45e8…/`.
- `GET /api/runs/dfba45e8…/summary`: `state failed`, `cost_usd 0.5166`, `tasks {failed 1, other 1}`, but `rungs: []`
  and `misordered: null`. Four strong tries ran ($0.5165), and none show per rung.

## Where

- `crates/roko-serve/src/routes/runs.rs::get_run_summary` (~line 309) and the rung readout built from the run's
  attempt records (gap-3217a0).
- The resume path: the serve mints a run id for the request while the Graph checkpoint's run id
  (`.roko/state/graph/<plan>/checkpoint.json` `run_id`) keeps the attempts.

## Plan

Have the summary of a serve run that resumed a Graph run also read the attempts of the run it continued, for this
process's attempts at least. Or record the continued run's id on the serve run, so readers can join the two.

## Done when

A resumed run's summary shows its own tries in `rungs[]`. Checked by the test in `[[verify]]`.
