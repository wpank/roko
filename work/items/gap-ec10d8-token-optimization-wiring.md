+++
id = "gap-ec10d8"
kind = "gap"
title = "Token Optimization Wiring"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-agent"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/402-token-optimization-wiring.md#402 — Token Optimization Wiring"
discovered_from = "audit:tmp/backlog/archive/402-token-optimization-wiring.md#402 — Token Optimization Wiring"
anchors = ["auction.rs", "compaction.rs", "BudgetPredictor", "SectionCompressor"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Roko pays full token cost on every agent dispatch. The codebase contains four non-trivial optimization systems — `BudgetPredictor`, `SectionCompressor`, VCG auction allocation (`auction.rs`), and conversation compaction (`compaction.rs`) — none of which are wired into the runtime. Additionally…

Imported without verification from:
- `tmp/backlog/archive/402-token-optimization-wiring.md#402 — Token Optimization Wiring`

How to verify: Check: Agent worktrees do not contain CLAUDE.md or AGENTS.md at dispatch time; Gate feedback sections are limited to ~500 characters in the assembled prompt; `BudgetPredictor` loads from `.roko/learn/` at dispatch and records outcomes [evidence: own status: Backlog]
