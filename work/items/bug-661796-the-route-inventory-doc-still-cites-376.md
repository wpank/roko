+++
id = "bug-661796"
kind = "bug"
title = "The route-inventory doc still cites 376 canonical routes; the tree has 402"
status = "open"
triage = "unverified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["docs"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-d1dea8"
anchors = ["docs/v3/depth/26-http/01-route-inventory-376.md"]
lane = "docs"
links = { depends_on = [], blocks = [], related = ["bug-d1dea8"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -n '376 canonical' docs/v3/depth/26-http/01-route-inventory-376.md"
+++

## Problem

bug-d1dea8 refreshed `tools/http_route_inventory.snapshot.json` (453 registrations, 402 canonical) and added a CI check. `docs/v3/depth/26-http/01-route-inventory-376.md` still cites about 376 canonical routes.

## Plan

Regenerate the doc's numbers from the tool, or point the doc at the snapshot. Rename the file if its name carries the count, and fix the links.

## Done when

- The verify passes, and docs link checks pass.

## Notes

- Reported on 2026-10-01 by wk-filer4, working on bug-d1dea8, during the evening close-out round.
