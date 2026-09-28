+++
id = "bug-dbbe78"
kind = "bug"
title = "[cli-audit stubs] 3 `todo!()` panics in HdcPrecompile (roko-chain phase2)"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-chain"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/cli-audit-2026-09-21/SUMMARY.md#Stubs & Unimplemented"
discovered_from = "audit:tmp/archive/cli-audit-2026-09-21/SUMMARY.md#Stubs & Unimplemented"
anchors = ["crates/roko-chain/src/phase2.rs HdcPrecompile", "backlog #341"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
HdcPrecompile in roko-chain/phase2.rs contained 3 todo!() panics (unreachable at audit time). Checklist marks #341 (non-panicking boundary) done.

Imported without verification from:
- `tmp/archive/cli-audit-2026-09-21/SUMMARY.md#Stubs & Unimplemented`
- `tmp/archive/cli-audit-2026-09-21/22-stubs-unimplemented.md`

A source claims this was fixed; confirm against current code before closing.

How to verify: grep -n 'todo!' crates/roko-chain/src/
