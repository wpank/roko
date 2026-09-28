+++
id = "bug-7257cb"
kind = "bug"
title = "Dream journal records hard-coded and fabricated fields"
status = "open"
triage = "verified"
severity = "p2"
subsystem = ["roko-dreams/journal", "roko-serve/dream"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-dreams/src/runner.rs::persist_journal_entry", "crates/roko-serve/src/routes/dream.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'cargo test -p roko-serve routes::dream'
+++

`persist_journal_entry` (`roko-dreams/src/runner.rs:1221`) hard-codes `trigger: DreamTrigger::Manual` (`:1228`); per a local audit it also records `total_tokens` as 0 and ignores the append result.
`GET /api/dream/journal` reads the whole file, looks for a `timestamp` key the journal never writes, and fabricates `episodes_total / 4` for a count it does not have (`routes/dream.rs`, `dream_journal`).
Fix: record the real trigger, spend and counts; have the route read the fields the journal writes, bounded.
