+++
id = "gap-1151bf"
kind = "gap"
title = "[provider F020] TaintTracker audit log is ephemeral (memory-only)"
status = "superseded"
triage = "verified"
severity = "p1"
subsystem = ["roko-agent/safety"]
created = 2026-09-01
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F020"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F020"
anchors = ["crates/roko-agent/src/safety/taint_propagation.rs::TaintTracker::save"]
links = { depends_on = [], blocks = [], related = ["gap-bf8d20"], supersedes = [], duplicate_of = "gap-ff95f5" }

[closed]
at = 2026-09-28
evidence = "duplicate of gap-ff95f5, which covers persisting TaintTracker state across restart along with witness and custody provenance. Partly addressed at HEAD 91b4745f8: TaintTracker now has atomic save/load (crates/roko-agent/src/safety/taint_propagation.rs:277-338), but no production code instantiates, saves or restores it; outside its own file it is only re-exported from safety/mod.rs. Taint history is still lost on restart."
+++
`TaintTracker` records taint propagation events in memory only. These records are not persisted to disk. After a process restart, the taint history is lost. Post-incident reconstruction of how taint propagated through a session is impossible.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F020`

How to verify: Check whether TaintTracker audit log persists to disk. Confirm in crates/roko-agent/src/safety/taint_propagation.rs whether still true: TaintTracker audit log is ephemeral (memory-only)

Verified 2026-09-28: superseded as a duplicate; see `[closed].evidence`.
