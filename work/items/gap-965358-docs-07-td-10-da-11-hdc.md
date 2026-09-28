+++
id = "gap-965358"
kind = "gap"
title = "DOCS-07 TD-10 / DA-11: HDC feature flag not propagated to roko-compose, roko-fs, roko-serve"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/features"]
created = 2026-09-15
updated = 2026-09-28
source = "tmp/docs-audit/07-TECH-DEBT.md#TD-10: HDC Feature Propagation"
discovered_from = "audit:tmp/docs-audit/07-TECH-DEBT.md#TD-10: HDC Feature Propagation"
anchors = ["crates/roko-cli/Cargo.toml", "hdc feature"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
roko-cli enables roko-neuro/hdc but does not forward `hdc` to compose, fs or serve, so HDC prompt assembly and substrate paths may be compiled out (CLI audit critical #4).

Imported without verification from:
- `tmp/docs-audit/07-TECH-DEBT.md#TD-10: HDC Feature Propagation`
- `tmp/dev-audit/11-implementation-status.md#New risks surfaced by the audits that affect dev-audit scope`
- `tmp/docs-audit/03-SUPERSEDED-AND-DEAD.md#Half-Implemented Features — FINISH ALL`
- `tmp/dogfood/2026-09-19-session.md#Fixes Applied This Session`

A source claims this was fixed; confirm against current code before closing.

How to verify: cargo tree -e features -p roko-cli | grep hdc.
