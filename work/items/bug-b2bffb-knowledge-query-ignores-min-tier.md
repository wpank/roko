+++
id = "bug-b2bffb"
kind = "bug"
title = "Knowledge query ignores min_tier"
status = "open"
triage = "verified"
severity = "p2"
subsystem = ["roko-serve/neuro"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-serve/src/routes/neuro.rs:35"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[repro]]
command = 'test "$(grep -c min_tier crates/roko-serve/src/routes/neuro.rs)" -gt 1'

[[verify]]
command = "test \"$(grep -c min_tier crates/roko-serve/src/routes/neuro.rs)\" -gt 1 && cargo test -p roko-serve routes::neuro::tests::query_filters_by_min_tier"
+++

The knowledge query request declares `min_tier: Option<String>` (`routes/neuro.rs:35`), but it is the field's only reference in the file: it is never read.
Callers asking for working or consolidated knowledge also get transient entries.
Fix: parse `min_tier`, filter by tier (400 on unknown tier names) and add a route test.
