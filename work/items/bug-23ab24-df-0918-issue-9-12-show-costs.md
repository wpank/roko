+++
id = "bug-23ab24"
kind = "bug"
title = "DF-0918 ISSUE-9/12: `show costs` aggregates historical broken runs (7.8% pass rate, $120 total)"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/show"]
created = 2026-09-18
updated = 2026-09-28
source = "tmp/dogfood/2026-09-18-session.md#ISSUE-9: `roko show costs` — pass_rate 7.8% is misleadingly low"
discovered_from = "audit:tmp/dogfood/2026-09-18-session.md#ISSUE-9: `roko show costs` — pass_rate 7.8% is misleadingly low"
anchors = ["roko show costs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Cost and pass-rate reports include early runs from broken provider configs with no windowing or decay, making current performance look far worse.

Imported without verification from:
- `tmp/dogfood/2026-09-18-session.md#ISSUE-9: `roko show costs` — pass_rate 7.8% is misleadingly low`
- `tmp/dogfood/2026-09-18-session.md#ISSUE-12: `show costs` reports $120 total but most is from old broken runs`

How to verify: Run roko show costs; check for time window/decay options.
