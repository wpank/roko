+++
id = "gap-905ed4"
kind = "gap"
title = "[refactor P3-07] Extend RuntimeServices into a DI facade (ServiceRegistry, lifecycle, profile plugins) deferred"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-execution"]
created = 2026-09-15
updated = 2026-09-28
source = "tmp/archive/refactoring-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#p3-low-cosmetic-future"
discovered_from = "audit:tmp/archive/refactoring-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#p3-low-cosmetic-future"
anchors = ["crates/roko-execution/", "RuntimeServicesBuilder", "ServiceRegistry"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Feasibility assessed (1-2 weeks); RuntimeServicesBuilder ~50% wired; ServiceRegistry, lifecycle management, profile plugins designed but not built.

Imported without verification from:
- `tmp/archive/refactoring-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#p3-low-cosmetic-future`

How to verify: Check roko-execution for ServiceRegistry/lifecycle hooks.
