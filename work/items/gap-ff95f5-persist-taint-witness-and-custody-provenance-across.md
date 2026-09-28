+++
id = "gap-ff95f5"
kind = "gap"
title = "Persist Taint, Witness, and Custody Provenance Across Restart"
status = "open"
triage = "verified"
severity = "p1"
goal = "features"
subsystem = ["roko-agent/safety"]
created = 2026-09-01
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/351-durable-taint-witness-and-custody-provenance.md#351 — Persist Taint, Witness, and Custody Provenance Across Restart"
discovered_from = "audit:tmp/backlog/archive/351-durable-taint-witness-and-custody-provenance.md#351 — Persist Taint, Witness, and Custody Provenance Across Restart"
anchors = ["crates/roko-agent/src/safety/taint_propagation.rs::TaintTracker", "crates/roko-agent/src/safety/witness.rs::WitnessLogger", "crates/roko-agent/src/safety/provenance.rs::CustodyLogger", "crates/roko-cli/src/custody.rs"]
links = { depends_on = [], blocks = [], related = ["gap-1151bf", "gap-bf8d20"], supersedes = [], duplicate_of = "" }
+++
[blocked] Blocked on #208 and #251; #282's registry contract is already frozen and aggregate #282 completion follows… — safety provenance is lost on restart and forensic components are not populated by live dispatch. `crates/roko-agent/src/safety/taint_propagation.rs::TaintTracker` maintains a…

Imported without verification from:
- `tmp/backlog/archive/351-durable-taint-witness-and-custody-provenance.md#351 — Persist Taint, Witness, and Custody Provenance Across Restart`

How to verify: Check: Every live tool/dispatch effect has an acknowledged pre-effect custody/witness record and a terminal result/denial record.; Signal-level taint ancestry and policy fingerprints survive process restart.; Restore occurs before new scheduling… [evidence: own status: Blocked on #208 and #251; #282's registry contract is already frozen and aggregate #282 completion follows…]

Verified 2026-09-28: still true. The cited files exist under crates/roko-agent/src/safety/. `TaintTracker` has atomic save/load (taint_propagation.rs:277-338), but no production code instantiates or restores it. `WitnessDag` and `WitnessLogger` have no users outside witness.rs (only re-exported at safety/mod.rs:110). `CustodyLogger` is used only by the read-side `roko knowledge custody list/show/verify` commands (crates/roko-cli/src/custody.rs:75, :150; commands/knowledge.rs:282-290). Live dispatch neither writes pre-effect witness/custody records nor persists taint ancestry.
