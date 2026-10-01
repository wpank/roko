+++
id = "bug-d1dea8"
kind = "bug"
title = "tools/http_route_inventory.py snapshot is stale and compares counts only; no CI job runs it"
status = "open"
triage = "unverified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["tools"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-c50b85"
anchors = ["tools/http_route_inventory.py", "tools/http_route_inventory.snapshot.json"]
lane = "tracker"
links = { depends_on = [], blocks = [], related = ["gap-c50b85"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 tools/http_route_inventory.py --check-snapshot"
+++

## Problem

`python3 tools/http_route_inventory.py --check-snapshot` fails at BASE: the snapshot has 421 registrations and 376 routes, the tree 453 and 402. The check compares counts only, and no CI job runs it.

## Plan

Refresh the snapshot, compare route lists rather than counts, and add the check to CI (or drop the snapshot).

## Done when

- The verify passes.

## Notes

- Reported on 2026-10-01 by wk-filer4, working on gap-c50b85, during the evening close-out round.
