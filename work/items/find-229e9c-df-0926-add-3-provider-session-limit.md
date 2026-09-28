+++
id = "find-229e9c"
kind = "finding"
title = "DF-0926 add-3: Provider session-limit refusals retried as ordinary task failures"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/provider"]
created = 2026-09-26
updated = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#Addendum 3 — 2026-09-26: plan 02, and a hard stop"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#Addendum 3 — 2026-09-26: plan 02, and a hard stop"
anchors = ["error_classify.rs", "claude_cli"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Claude CLI 'session limit' exits (exit 1 in ~2s) were retried three times, then FailFast skipped dependents; refusal/limit errors should be classified and paused rather than consuming retries.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#Addendum 3 — 2026-09-26: plan 02, and a hard stop`

How to verify: Check error classification for session/rate-limit messages.
