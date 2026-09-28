+++
id = "bug-fade34"
kind = "bug"
title = "`knowledge query` results dump full hypothesis text"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/knowledge"]
created = 2026-09-18
updated = 2026-09-28
source = "tmp/dogfood/2026-09-18-session.md#ISSUE-18: `roko knowledge query` results are verbose/noisy"
discovered_from = "audit:tmp/dogfood/2026-09-18-session.md#ISSUE-18: `roko knowledge query` results are verbose/noisy"
anchors = ["roko knowledge query"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Results print full strategy hypothesis text; proposal: truncate to ~2 lines with --verbose for full output.

Imported without verification from:
- `tmp/dogfood/2026-09-18-session.md#ISSUE-18: `roko knowledge query` results are verbose/noisy`

How to verify: Run a knowledge query and inspect output width.
