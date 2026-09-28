+++
id = "gap-ed42f3"
kind = "gap"
title = "DOCS-06 B3 / TD-03,TD-17: Blanket and scattered lint suppressions"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-serve"]
created = 2026-09-15
updated = 2026-09-28
source = "tmp/docs-audit/06-POTENTIAL-BACKLOG.md#B3. `roko-serve` blanket `#![allow(dead_code)]` suppression"
discovered_from = "audit:tmp/docs-audit/06-POTENTIAL-BACKLOG.md#B3. `roko-serve` blanket `#![allow(dead_code)]` suppression"
anchors = ["crates/roko-serve/src/lib.rs", "#![allow(dead_code)]"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
roko-serve has crate-wide #![allow(dead_code)]; roko-agent carries 40+ and roko-cli 50+ clippy suppressions (roko-cli blanket allow removed 2026-09-04). Decision: remove suppression and fix exposed dead code. roko-serve suppression removed 2026-09-15 (MISC-03).

Imported without verification from:
- `tmp/docs-audit/06-POTENTIAL-BACKLOG.md#B3. `roko-serve` blanket `#![allow(dead_code)]` suppression`
- `tmp/docs-audit/07-TECH-DEBT.md#TD-03: Blanket Dead Code Suppressions`
- `tmp/docs-audit/07-TECH-DEBT.md#TD-17: Various `#[allow(...)]` Annotations`
- `tmp/dogfood/2026-09-18-session.md#Final Summary`
- `tmp/archive/dogfood-audit-2026-09-03/06-status-update-2026-09-03.md#Remaining Open Work`

How to verify: grep -rn 'allow(dead_code)' crates/roko-serve/src/lib.rs; count allow attributes.
