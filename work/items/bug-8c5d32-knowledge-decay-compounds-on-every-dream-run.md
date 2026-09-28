+++
id = "bug-8c5d32"
kind = "bug"
title = "Knowledge decay compounds on every dream run"
status = "open"
triage = "verified"
severity = "p2"
subsystem = ["roko-neuro/knowledge", "roko-dreams"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-neuro/src/knowledge_store/gc.rs::decay"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'cargo test -p roko-neuro knowledge_store'
+++

`KnowledgeStore::decay` (`knowledge_store/gc.rs:18`) multiplies each entry's stored confidence by an age factor and writes the result back (`:26-30`); the dream runner calls it on every cycle.
Because the factor is computed from the full age while the already-decayed value is stored, each run charges the same age again and confidence collapses with run frequency, not elapsed time.
Fix: make decay time-idempotent (derive effective confidence from base confidence and age at read time, or apply only the interval since the last decay) and test that two runs an hour apart decay by one hour.
