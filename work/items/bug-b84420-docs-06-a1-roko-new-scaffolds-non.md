+++
id = "bug-b84420"
kind = "bug"
title = "DOCS-06 A1: `roko new` scaffolds non-compiling code for 6 of 9 types"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/commands/new"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/docs-audit/06-POTENTIAL-BACKLOG.md#A1. `roko new` generates non-compiling code (6/9 types)"
discovered_from = "audit:tmp/docs-audit/06-POTENTIAL-BACKLOG.md#A1. `roko new` generates non-compiling code (6/9 types)"
anchors = ["roko new", "crates/roko-cli/src/commands/", "crates/roko-cli/src/scaffold.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Scaffold templates still use pre-rename types, so e.g. `roko new signal foo` generates broken code (decision: FIX templates).

Imported without verification from:
- `tmp/docs-audit/06-POTENTIAL-BACKLOG.md#A1. `roko new` generates non-compiling code (6/9 types)`
- `tmp/docs-audit/03-SUPERSEDED-AND-DEAD.md#CLI Stub Commands`
- `tmp/dogfood/2026-09-20-final-session.md#P0 (Critical)`
- `tmp/dogfood/2026-09-19-session.md#Issues Found`
- `tmp/archive/dogfood-audit-2026-09-03/06-status-update-2026-09-03.md#Cross-Audit Findings (from 2026-09-01 register)`
- `crates/roko-cli/src/scaffold.rs:119`
- `crates/roko-cli/src/scaffold.rs:768`
- `docs/v3/39-ROADMAP.md#3.4 CLI Surface Fixes`
- `tmp/nous-research/AUDIT-SUMMARY-2026-09-04.md#6. CLI Audit`

A source claims this was fixed; confirm against current code before closing.

How to verify: Generate each `roko new` type into a temp crate and cargo check. / For each `roko new <type> <name>` generate into a temp crate and cargo check it.

Merged 2 mined candidates: m4-031, m5-016.
