+++
id = "bug-aa8893"
kind = "bug"
title = "roko_hdc_queries_total has no production emitter"
status = "open"
triage = "unverified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-neuro"]
created = 2026-10-02
updated = 2026-10-02
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-a95898"
anchors = ["crates/roko-neuro/src/"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-a95898"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rn 'roko_hdc_queries_total' crates/ --include=*.rs"
+++

## Problem

`roko_hdc_queries_total` is reached only from `query_hdc`, which only the uncalled `query_by_role_filler` calls, so the metric never moves.

## Plan

Emit it where HDC queries actually run, or drop the metric.

## Done when

- The metric moves in a test, or it is gone.

## Notes

- Reported on 2026-10-02 by wk-streams, working on gap-a95898, during the overnight close-out round.
