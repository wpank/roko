+++
id = "gap-d8382b"
kind = "gap"
title = "build_bundle.py doesn't copy econ-report.json into bundles; GET /api/showcase/economics 404s on real bundles"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-serve/showcase"]
created = 2026-10-04
updated = 2026-10-04
last_verified = 2026-10-04
source = "gate-13c follow-up reports 2026-10-04 (PK84 gap-59ebfd, task 6134)"
discovered_from = "gap-59ebfd (closed; own done-note already flagged this, never tracked)"
anchors = ["benchmarks/viabilitybench/showcase/build_bundle.py::build"]
lane = "bench"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_build_copies_the_econ_report_into_the_bundle' benchmarks/viabilitybench/showcase/test_bundle.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/showcase/test_bundle.py -q -k test_build_copies_the_econ_report_into_the_bundle"
+++

## Problem

`build_bundle.py` never copies `.roko/econ/<id>/econ-report.json` into a bundle as
`econ/<id>/econ-report.json`, so `GET /api/showcase/economics?experiment_id=<id>` 404s on every
real bundle. `economics::economics` (`crates/roko-serve/src/routes/showcase/economics.rs:42-64`)
serves `econ/{experiment}/econ-report.json` (`ECON_REPORT`, line 31) "from the newest verified
bundle that lists it" (line 53). `build_bundle.py::build` (`benchmarks/viabilitybench/showcase/build_bundle.py:340-422`)
only ever writes `data/records.jsonl`, `data/metrics.jsonl`, optional `data/mechanism/*.jsonl`
and optional `timeline/events.jsonl` into a bundle's `files` dict and `manifest["files"]`; there
is no `econ` entry anywhere in the script (confirmed: zero matches for "econ" in the file).
PK84's own done-note already flagged this at closure
(`work/done/gap-59ebfd-pk84-m3-self-model-serve-the-economics.md:77`: "The bundle builder does
not copy reports into `econ/<id>/` yet."), but no open item tracks fixing the builder side.

## Why it matters

Goal: cybernetic, M3 self-model / showcase economics (backlog 6134, PK84). The serve-side route
is implemented, tested and shipped, but it is unreachable in practice: every bundle that
`build_bundle.py` produces is missing the one file the route needs, so the showcase's
economics view 404s for any real (non-hand-crafted-fixture) bundle.

## Where

- `benchmarks/viabilitybench/showcase/build_bundle.py::build` (the fix: copy the econ report
  in, alongside `data/`/`timeline/`).
- `crates/roko-serve/src/routes/showcase/economics.rs::economics`, `ECON_REPORT` (the consumer;
  unchanged, read-only reference).
- `.roko/econ/<experiment_id>/econ-report.json` (the source M3 already writes, per PK84's note).

## Current state

The route and its tests pass against hand-built fixture bundles that already place a file at
`econ/<id>/econ-report.json`. No real bundle `build_bundle.py` produces has that path, so the
route's happy path is untested against the actual builder's output.

## Plan

1. In `build()`, if `.roko/econ/<experiment_id>/econ-report.json` exists for the experiment
   being bundled, copy it into the bundle at `econ/<experiment_id>/econ-report.json` and add it
   to `files`/`manifest["files"]` the same way `data/mechanism/*.jsonl` is included
   conditionally.
2. Add a regression test that builds a bundle from an experiment with an econ report present
   and asserts the bundle contains `econ/<id>/econ-report.json`.
3. Land this after `gap-dbe8e7` (currently claimed and in progress, `work/gap-dbe8e7`, same
   file's `project_views`/overview logic) merges, to avoid a collision on `build_bundle.py`.

## Done when

- A bundle built by `build_bundle.py` from an experiment with an econ report includes
  `econ/<id>/econ-report.json`, and `GET /api/showcase/economics` serves it from that bundle.
- The `[[verify]]` command passes.

## Notes

- 2026-10-04 (gate-13c follow-up, PK84 gap-59ebfd, PK85 gap-fcb44c-adjacent): confirmed at main
  HEAD `908f7ec40`. `gap-59ebfd` is closed; this is a genuine follow-up the done-note already
  anticipated but nothing tracked. `gap-dbe8e7` is live-claimed (`.roko/work-claims/gap-dbe8e7.json`,
  session `coordinator-session-roko-90`, branch `work/gap-dbe8e7`) and also edits
  `build_bundle.py`; this item does not touch that file and should be picked up only after
  `gap-dbe8e7` merges.

## Progress

- gap-d8382b: implemented at b59d1a150. The builder copies M3's report from `--econ` or `.roko/econ/<id>/econ-report.json`, byte for byte, to `econ/<id>/econ-report.json`, listed in `files` and SHA256SUMS; verify_bundle.py has an `econ` rule. Bench venv pytest showcase/ 29/29 (the verify included); a bundle built with a report and served by a copy of the batch binary answers `GET /api/showcase/economics?experiment_id=FIXTURE-P1` 200 byte for byte, and 404 for another experiment.
