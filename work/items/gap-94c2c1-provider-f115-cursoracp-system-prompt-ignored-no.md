+++
id = "gap-94c2c1"
kind = "gap"
title = "[provider F115] CursorAcp system prompt ignored; no tool support"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/provider"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F115"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F115"
anchors = ["crates/roko-agent/src/provider/cursor_acp.rs", "CursorAcp", "crates/roko-agent/src/provider/gemini_cli.rs", "GeminiCli"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
The Cursor ACP adapter ignores the system prompt from `AgentOptions` and provides no tool dispatch. Agents dispatched via CursorAcp cannot use tools.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F115`
- `tmp/archive/provider-audit/01-provider-adapters.md`
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F116`

How to verify: Roadmap P3-3 forwarded system prompts; tool support for CursorAcp was not addressed. Check tool support only. Confirm in crates/roko-agent/src/provider/cursor_acp.rs whether still true: `CursorAcp` system prompt ignored; no tool support / Roadmap P3-3 forwarded system prompts; GeminiCli tool support not addressed. Check tool support only. Confirm in crates/roko-agent/src/provider/gemini_cli.rs whether still true: `GeminiCli` system prompt and tool support absent

Merged 2 mined candidates: m2-081, m2-082.
