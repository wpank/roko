+++
id = "bug-8c5d32"
kind = "bug"
title = "Knowledge decay compounds on every dream run"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
hold = "Set aside per tldr/05 §3; Will chose hold over park on 2026-09-29 (dec-e70592). Remove this line to revive."
subsystem = ["roko-neuro/knowledge", "roko-dreams"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-neuro/src/knowledge_store/gc.rs::decay", "crates/roko-neuro/src/knowledge_store/scoring.rs::recency_factor", "crates/roko-dreams/src/runner.rs:1032"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! sed -n '/pub fn decay(&self)/,/^    }/p' crates/roko-neuro/src/knowledge_store/gc.rs | grep -q 'entry.confidence.max(0.0) \\* factor' && cargo test -p roko-neuro decay_is_time_idempotent  (current `cargo test -p roko-neuro knowledge_store` passes while the bug exists)"
+++

`KnowledgeStore::decay` (`knowledge_store/gc.rs:18`) multiplies each entry's stored confidence by an age factor and writes the result back (`:26-30`); the dream runner calls it on every cycle.
Because the factor is computed from the full age while the already-decayed value is stored, each run charges the same age again and confidence collapses with run frequency, not elapsed time.
Fix: make decay time-idempotent (derive effective confidence from base confidence and age at read time, or apply only the interval since the last decay) and test that two runs an hour apart decay by one hour.
