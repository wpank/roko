+++
id = "bug-dee062"
kind = "bug"
title = "Dream consolidation merges entries that share two tags by concatenating their text"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
subsystem = ["roko-dreams/consolidation"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-dreams/src/runner.rs::shared_tag_count", "crates/roko-dreams/src/runner.rs:991", "crates/roko-dreams/src/runner.rs::merge_knowledge_group"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'shared_tag_count(&entries\\[index\\], &entries\\[candidate\\]) < 2' crates/roko-dreams/src/runner.rs && cargo test -p roko-dreams consolidation_does_not_merge_unrelated_entries_sharing_tags  (current `cargo test -p roko-dreams` passes while the bug exists)"
+++

The merge step links same-kind, non-frozen entries when they share at least two tags (`runner.rs:990`, `shared_tag_count` at `:1368`) and fuses each connected component of three or more into one `dream_consolidated_*` entry: member texts joined with blank lines, mean confidence, promoted tier.
Tags alone (often provenance tags such as a source name) decide the merge, with no semantic similarity or contradiction check, so unrelated rules become one promoted blob.
Fix: merge on a provenance-free semantic key, keep the originals with provenance links, and summarise instead of concatenating.
