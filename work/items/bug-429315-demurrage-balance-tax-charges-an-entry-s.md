+++
id = "bug-429315"
kind = "bug"
title = "Demurrage balance tax charges an entry's whole age on every pass"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
subsystem = ["roko-neuro/knowledge"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-neuro/src/knowledge_store/gc.rs::apply_demurrage", "crates/roko-neuro/src/lib.rs:703", "crates/roko-serve/src/lib.rs:2423"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! sed -n '/pub fn apply_demurrage(&self)/,/^    }/p' crates/roko-neuro/src/knowledge_store/gc.rs | grep -q 'signed_duration_since(entry.created_at)' && cargo test -p roko-neuro apply_demurrage_charges_only_elapsed_interval  (current `cargo test -p roko-neuro knowledge_store` passes while the bug exists)"
+++

`apply_demurrage` (`knowledge_store/gc.rs:203`) computes the tax from `now - created_at` (`:208-210`) on every pass instead of the time since the previous pass.
An entry is charged for its full lifetime again on each run, so old entries deplete (and freeze after 7 days) far faster than the configured rate.
Fix: store a last-taxed timestamp and tax only the elapsed interval; test that two passes an hour apart charge one hour.
