+++
id = "gap-e891a5"
kind = "gap"
title = "Wire Attention Bidder Load/Save Persistence Loop"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-compose"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/380-wire-attention-bidder-persistence-loop.md#380 — Wire Attention Bidder Load/Save Persistence Loop"
discovered_from = "audit:tmp/backlog/archive/380-wire-attention-bidder-persistence-loop.md#380 — Wire Attention Bidder Load/Save Persistence Loop"
anchors = ["crates/roko-compose/src/context_provider.rs", "crates/roko-cli/src/dispatch/prompt_builder.rs", ".roko/learn/attention-bidders.json", "load_attention_bidders()", "set_learning_bidders()"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
attention bidder learning infrastructure exists but never initializes or persists. RAG-02 verification found that the attention-bidder learning loop is incomplete. `load_attention_bidders()` and `save_attention_bidders()` functions exist but are only used in tests. `set_learning_bidders()` setter…

Imported without verification from:
- `tmp/backlog/archive/380-wire-attention-bidder-persistence-loop.md#380 — Wire Attention Bidder Load/Save Persistence Loop`

How to verify: Check whether the gap described in tmp/backlog/archive/380-wire-attention-bidder-persistence-loop.md still exists at the anchored paths. [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Audit Sweep Items (#376-#395)]
