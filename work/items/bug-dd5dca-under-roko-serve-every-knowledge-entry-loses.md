+++
id = "bug-dd5dca"
kind = "bug"
title = "Under roko serve every knowledge entry loses confidence every ~2.8 hours"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
hold = "Set aside per tldr/05 §3; Will chose hold over park on 2026-09-29 (dec-e70592). Remove this line to revive."
subsystem = ["roko-serve/neuro", "roko-neuro/knowledge"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-serve/src/lib.rs::start_demurrage_timer", "crates/roko-serve/src/lib.rs:2373", "crates/roko-neuro/src/knowledge_store/crud.rs::record_access"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[repro]]
command = '! grep -q "last_validated_at: 0" crates/roko-serve/src/lib.rs'

[[verify]]
command = "! grep -q 'validated_since_last: false' crates/roko-serve/src/lib.rs && grep -rqE '\\.record_access\\(' crates/roko-serve/src crates/roko-neuro/src --include='*.rs'"
+++

`start_demurrage_timer` (`roko-serve/src/lib.rs:2212`) runs the demurrage consumer about every 250 x 40 s (~2.78 h) and hard-codes `last_validated_at: 0` / `validated_since_last: false` (`:2257-2258`) before `store.apply_demurrage()` (`:2307`).
Every entry is treated as never validated and loses confidence on each pass, and retrieval does not count as use, so a long-running server erodes all knowledge regardless of how it is used.
Fix: pass real validation and access state to the consumer and credit retrieval (`record_access`) as reinforcement.

Re-checked 2026-09-29: unchanged. The hardcoded fields moved to crates/roko-serve/src/lib.rs:2373-2374 (function now at :2328, apply_demurrage at :2423). KnowledgeStore::record_access (crates/roko-neuro/src/knowledge_store/crud.rs:559) is defined but never called.
