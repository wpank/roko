+++
id = "bug-6c11d1"
kind = "bug"
title = "DF-0918 ISSUE-19: `show agents` lists long-gone agents"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/show"]
created = 2026-09-18
updated = 2026-09-28
source = "tmp/dogfood/2026-09-18-session.md#ISSUE-19: `roko show agents` shows stale agents from months ago"
discovered_from = "audit:tmp/dogfood/2026-09-18-session.md#ISSUE-19: `roko show agents` shows stale agents from months ago"
anchors = ["roko show agents"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Agents from May/August (H11:12, DEMO-T01:1) still appear; needs age filtering or stale marking.

Imported without verification from:
- `tmp/dogfood/2026-09-18-session.md#ISSUE-19: `roko show agents` shows stale agents from months ago`

How to verify: Run roko show agents; check timestamps.
