+++
id = "bug-3d2059"
kind = "bug"
title = "docs/v2/GITHUB-INTEGRATION.md claims PRs, comments, CI polling and merges that the Graph runner does not do"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["docs"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d27cfae01"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-cd51b7"
anchors = ["docs/v2/GITHUB-INTEGRATION.md"]
lane = "docs"
links = { depends_on = [], blocks = [], related = ["gap-cd51b7"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -n 'IMPLEMENTED' docs/v2/GITHUB-INTEGRATION.md"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:27:12Z"
commit = "d27cfae01"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T18:14:48Z"
forced = false
evidence = "docs/v2/GITHUB-INTEGRATION.md now says PARTIAL and marks each runner step with its real status; the verify (! grep -n 'IMPLEMENTED' docs/v2/GITHUB-INTEGRATION.md) passes and check_markdown_links.py passes"
+++

## Problem

The doc says IMPLEMENTED and describes draft PRs, PR comments, closing issues, CI polling and merges. After gap-cd51b7, the Graph runner only files failure issues.

## Plan

Mark each feature with its real status.

## Done when

- The verify passes.

## Notes

- Reported on 2026-10-01 by wk-filer4, working on gap-cd51b7, during the evening close-out round.
- 2026-10-01 (wk-filer4): implemented on work/gap-cd51b7. The status line now says PARTIAL. "Runner workflow" describes the one live step, task-failure issues with `auto_pr = true` (gap-cd51b7), and lists the rest in a table: GitHub plan branch and draft PR, PR comments, closing issues, pushing, and CI polling and merges, each marked "not wired", "not built" or "off". The push note, the branch convention, the CI plan-validation commands (the workflow file runs plain `plan validate` per tracked plan plus `plan index --check`, not `--strict` on `tmp/status-quo/backlog/plans/`) and two troubleshooting bullets were corrected to match. `tools/docs_integrity/check_markdown_links.py` passes.
