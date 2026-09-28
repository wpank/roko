+++
id = "gap-594c89"
kind = "gap"
title = "DOCS-06 D1: Exclude roko-demo (zero callers) from default workspace builds"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-demo"]
created = 2026-09-15
updated = 2026-09-28
source = "tmp/docs-audit/06-POTENTIAL-BACKLOG.md#D1. roko-demo crate (5,838 LOC)"
discovered_from = "audit:tmp/docs-audit/06-POTENTIAL-BACKLOG.md#D1. roko-demo crate (5,838 LOC)"
anchors = ["Cargo.toml default-members", "crates/roko-demo/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
roko-demo (5,838 LOC) has zero callers; decision is KEEP but exclude from default builds.

Imported without verification from:
- `tmp/docs-audit/06-POTENTIAL-BACKLOG.md#D1. roko-demo crate (5,838 LOC)`
- `tmp/docs-audit/00-INDEX.md#Key Decisions (ALL FINALIZED 2026-09-15)`

How to verify: Check workspace default-members.
