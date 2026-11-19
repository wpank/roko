+++
id = "bug-c1649d"
kind = "bug"
title = "Dream cycle writes its report before consolidation runs"
status = "open"
triage = "verified"
severity = "p3"
hold = "Set aside per tldr/05 §3; Will chose hold over park on 2026-09-29 (dec-e70592). Remove this line to revive."
subsystem = ["roko-dreams/cycle"]
created = 2026-09-28
updated = 2026-10-27
last_verified = 2026-10-27
last_verified_rev = "15335e2f9"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-dreams/src/cycle.rs:910", "crates/roko-dreams/src/cycle.rs::write_report", "crates/roko-dreams/src/runner.rs:1206"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'self.write_report(&report)' crates/roko-dreams/src/cycle.rs && cargo test -p roko-dreams report_written_after_consolidation"
+++

`DreamCycle` calls `self.write_report(&report)` inside the cycle (`cycle.rs:879`); merge, decay and GC run afterwards in the dream runner (`runner.rs`).
The persisted report never reflects what consolidation changed, and a crash during consolidation leaves a report claiming a finished cycle.
Fix: write the report after consolidation and include its outcome.

Checked 2026-10-27: This item has no goal field set.
