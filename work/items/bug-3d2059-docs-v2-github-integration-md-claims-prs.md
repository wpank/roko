+++
id = "bug-3d2059"
kind = "bug"
title = "docs/v2/GITHUB-INTEGRATION.md claims PRs, comments, CI polling and merges that the Graph runner does not do"
status = "open"
triage = "unverified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["docs"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-cd51b7"
anchors = ["docs/v2/GITHUB-INTEGRATION.md"]
lane = "docs"
links = { depends_on = [], blocks = [], related = ["gap-cd51b7"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -n 'IMPLEMENTED' docs/v2/GITHUB-INTEGRATION.md"
+++

## Problem

The doc says IMPLEMENTED and describes draft PRs, PR comments, closing issues, CI polling and merges. After gap-cd51b7, the Graph runner only files failure issues.

## Plan

Mark each feature with its real status.

## Done when

- The verify passes.

## Notes

- Reported on 2026-10-01 by wk-filer4, working on gap-cd51b7, during the evening close-out round.
