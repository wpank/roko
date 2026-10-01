+++
id = "gap-c56341"
kind = "gap"
title = "plan prepare --full is not built: no LLM-written decomposition.md or rubric.md"
status = "open"
triage = "unverified"
severity = "p3"
goal = "features"
size = "M"
subsystem = ["roko-cli/plan"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-d6fd85"
anchors = ["crates/roko-cli/src/plan_brief.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-d6fd85"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -n 'full' crates/roko-cli/src/plan_brief.rs"
+++

## Problem

gap-d6fd85 added `roko plan prepare` with deterministic companion documents. Its phase 3 is not built: `--full`, an LLM-written `decomposition.md` and `rubric.md`.

## Plan

Build phase 3 as gap-d6fd85's notes describe, behind `--full`.

## Done when

- `roko plan prepare --full` writes both documents, and a test with the scripted provider covers it.

## Notes

- Reported on 2026-10-01 by the worker on gap-d6fd85, during the evening close-out round.
