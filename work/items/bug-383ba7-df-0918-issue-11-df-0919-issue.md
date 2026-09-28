+++
id = "bug-383ba7"
kind = "bug"
title = "DF-0918 ISSUE-11 / DF-0919 ISSUE-26: `config show` output hard to use (truncation, no section filter)"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/config"]
created = 2026-09-19
updated = 2026-09-28
source = "tmp/dogfood/2026-09-18-session.md#ISSUE-11: `config show` truncates after providers section"
discovered_from = "audit:tmp/dogfood/2026-09-18-session.md#ISSUE-11: `config show` truncates after providers section"
anchors = ["roko config show"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
ISSUE-11: output stopped after the first provider; ISSUE-26: full dump is 272 lines and needs `config show <section>` filtering (nice-to-have).

Imported without verification from:
- `tmp/dogfood/2026-09-18-session.md#ISSUE-11: `config show` truncates after providers section`
- `tmp/dogfood/2026-09-19-session.md#ISSUE-26: Config show outputs 272 lines — hard to find specific settings`

How to verify: Run roko config show [section].
