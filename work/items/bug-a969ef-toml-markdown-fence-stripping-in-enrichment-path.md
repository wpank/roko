+++
id = "bug-a969ef"
kind = "bug"
title = "TOML markdown fence stripping in enrichment path"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-compose"]
created = 2026-08-13
updated = 2026-09-28
source = "tmp/archive/08-15-26/MASTER-TASKS.md#1. Demo & Pitch (P1-2)"
discovered_from = "audit:tmp/archive/08-15-26/MASTER-TASKS.md#1. Demo & Pitch (P1-2)"
anchors = []
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Enrichment path does not strip ```toml markdown fences from agent output before parsing (P1-2, 2026-08 demo list). May be subsumed by consolidated P2-PLN-3 (plan-generation TOML reliability, done).

Imported without verification from:
- `tmp/archive/08-15-26/MASTER-TASKS.md#1. Demo & Pitch (P1-2)`

How to verify: Find the enrichment TOML parse site (roko-compose enrichment / plan generation) and check for fence stripping before toml::from_str.
