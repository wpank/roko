+++
id = "gap-afc773"
kind = "gap"
title = "[provider F082] Gemini thought content (reasoning text) not extracted; only token count captured"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-agent/gemini"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F082"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F082"
anchors = ["crates/roko-agent/src/gemini/types.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
The Gemini `Part` enum has no `Thought` variant. Gemini's thought text (parts with `thought: true`) is not parsed or stored. Only `thinking_token_count` from `usageMetadata` is captured.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F082`
- `tmp/archive/provider-audit/18-thinking-reasoning.md`

How to verify: Confirm in crates/roko-agent/src/gemini/types.rs whether still true: Gemini thought content (reasoning text) not extracted; only token count captured
