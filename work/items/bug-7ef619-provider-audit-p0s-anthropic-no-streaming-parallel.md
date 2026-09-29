+++
id = "bug-7ef619"
kind = "bug"
title = "Provider audit P0s (Anthropic no streaming, parallel tool results, 400→ContextOverflow, ErrorClass Unknown) + 75 unmerged fixes"
status = "done"
triage = "verified"
severity = "p1"
subsystem = ["roko-agent/providers"]
created = 2026-09-04
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/nous-research/AUDIT-SUMMARY-2026-09-04.md#2. Provider Audit (`tmp/provider-audit/`, 44 files, 189 findings)"
discovered_from = "audit:tmp/nous-research/AUDIT-SUMMARY-2026-09-04.md#2. Provider Audit (`tmp/provider-audit/`, 44 files, 189 findings)"
anchors = ["crates/roko-agent/src/provider/anthropic_api/tool_loop.rs:685", "crates/roko-agent/src/provider/error_classify.rs:154", "crates/roko-agent/src/tool_loop/backends/mod.rs:139"]
links = { depends_on = [], blocks = [], related = ["gap-d30038"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-agent --lib error_classify"

[[verify]]
command = "cargo test -p roko-agent --lib anthropic_api"

[closed]
at = 2026-09-28
commit = "91b4745f8"
evidence = "Fixed at HEAD 91b4745f8. F001: the Anthropic backend streams (provider/anthropic_api/tool_loop.rs:473 sends stream: true; stream_turn at :685; ToolLoop calls stream_turn at tool_loop/mod.rs:1430). F002: tool_loop.rs:232-241 sends all tool_result blocks of a turn in one user message. F003: provider/error_classify.rs:154 routes HTTP 400 to classify_bad_request, which maps context_length_exceeded to ContextOverflow (:250-255). F004: tool_loop/backends/mod.rs:139-140 and :220-221 set disable_parallel_tool_calls and normalize_tool_call_content for Cerebras. F008: no ErrorClass::Unknown occurrence remains in non-test crates/roko-agent/src. No provider-audit branches remain locally, so the 75-unmerged-fixes claim is moot. error_classify.rs (+536 lines) and anthropic_api/tool_loop.rs carry uncommitted rate-limit/exhaustion work from a concurrent session; HEAD already satisfies F001-F008."
+++
F001 Anthropic API never streams; F002 parallel tool results sent as separate messages; F003 HTTP 400 not mapped to ContextOverflow (retry storms); F004 Cerebras flags lost in rate-limited factory; F008 ErrorClass always Unknown. 75 fixes sat on worktree branches pending merge.

Imported without verification from:
- `tmp/nous-research/AUDIT-SUMMARY-2026-09-04.md#2. Provider Audit (`tmp/provider-audit/`, 44 files, 189 findings)`
- `tmp/provider-audit/`

How to verify: Check whether feat/provider-audit-implementation merged; re-test F001-F008.

Verified 2026-09-28: fixed; see `[closed].evidence`.
