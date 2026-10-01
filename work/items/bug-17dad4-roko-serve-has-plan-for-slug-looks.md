+++
id = "bug-17dad4"
kind = "bug"
title = "roko-serve has_plan_for_slug looks only in .roko/plans, but generated plans now land in the workspace plans directory"
status = "open"
triage = "unverified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-serve/routes"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-e3df7d"
anchors = ["crates/roko-serve/src/routes/prds.rs::has_plan_for_slug"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-e3df7d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-serve --lib has_plan_for_slug"
+++

## Problem

`has_plan_for_slug` (`routes/prds.rs`) decides whether a PRD already has a plan by looking under `.roko/plans`. Since bug-e3df7d, `roko plan generate` and the job runner write plans to the workspace plans directory (`roko_fs::workspace_plans::workspace_plans_dir`), so the PRD routes report "no plan" for plans that exist.

## Plan

1. Look in the workspace plans directory (the same resolver `plan run` uses), and keep `.roko/plans` as a legacy fallback.
2. Add a test named `has_plan_for_slug_*` that finds a plan written under `plans/`.

## Done when

- `cargo test -p roko-serve --lib has_plan_for_slug` passes, and the test covers a plan under the workspace plans directory.

## Notes

- Reported on 2026-10-01 by the worker on bug-e3df7d, during the evening close-out round.
