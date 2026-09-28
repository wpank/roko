+++
id = "bug-7567eb"
kind = "bug"
title = "Hermes tool-call parser executes <tool_call> blocks inside <think> reasoning"
status = "open"
triage = "verified"
severity = "p1"
subsystem = ["roko-agent/translate"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-agent/src/translate/hermes.rs::parse_calls"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'cargo test -p roko-agent --lib translate::hermes'
+++

The module doc says "`<think>...</think>` reasoning blocks are skipped" (`translate/hermes.rs:34`), but `parse_calls` (`:92`) scans the whole response with `find("<tool_call>")` and has no reasoning-block handling.
The only think test (`parse_skips_think_blocks`, `:440`) has no tool call inside the think block. A call the model merely drafts while reasoning is executed, and an unclosed `<tool_call>` parses to end of text.
Fix: strip reasoning blocks (think/thinking/reasoning, including unterminated ones) before scanning; add tests for a call inside `<think>` (must not execute) and after it (must execute).
