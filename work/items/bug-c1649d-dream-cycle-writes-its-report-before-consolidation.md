+++
id = "bug-c1649d"
kind = "bug"
title = "Dream cycle writes its report before consolidation runs"
status = "open"
triage = "verified"
severity = "p3"
subsystem = ["roko-dreams/cycle"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-dreams/src/cycle.rs::write_report", "crates/roko-dreams/src/cycle.rs:879"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

`DreamCycle` calls `self.write_report(&report)` inside the cycle (`cycle.rs:879`); merge, decay and GC run afterwards in the dream runner (`runner.rs`).
The persisted report never reflects what consolidation changed, and a crash during consolidation leaves a report claiming a finished cycle.
Fix: write the report after consolidation and include its outcome.
