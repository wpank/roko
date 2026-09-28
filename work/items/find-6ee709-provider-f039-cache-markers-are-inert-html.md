+++
id = "find-6ee709"
kind = "finding"
title = "[provider F039] Cache markers are inert HTML comments, not translated to provider API cache_control"
status = "open"
triage = "verified"
severity = "p3"
subsystem = ["roko-compose/system_prompt_builder"]
created = 2026-09-01
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F039"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F039"
anchors = ["crates/roko-compose/src/system_prompt_builder.rs:1474", "crates/roko-cli/src/dispatch/prompt_builder.rs:1740", "crates/roko-agent/src/translate/claude.rs::inject_cache_markers_into_content", "crates/roko-agent/src/provider/anthropic_api/tool_loop.rs:466"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
The `SystemPromptBuilder` emits `<!-- cache:system -->` and `<!-- cache:session -->` HTML comment markers at stability tier boundaries. These are raw strings in the concatenated system prompt. The Anthropic API requires `cache_control: { type: "ephemeral" }` JSON fields in the content blocks. Tra...

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F039`
- `tmp/archive/provider-audit/15-prompt-composition.md`

How to verify: Confirm in crates/roko-compose/src/system_prompt_builder.rs, crates/roko-agent/src/translate/claude.rs whether still true: Cache markers are inert HTML comments, not translated to provider API `cache_control`

Verified 2026-09-28: partly fixed, and narrower than the title says. Both Anthropic API paths now turn the markers into cache_control blocks through translate/claude.rs::inject_cache_markers_into_content: ClaudeAgent (crates/roko-agent/src/claude_agent.rs:357, :375) and the anthropic_api tool loop (provider/anthropic_api/tool_loop.rs:466, :485, :526, :544). Still true: the markers are switched on for every provider (dispatch/prompt_builder.rs:1740, prompting.rs:59, prompt_assembly_service.rs:395) and are neither stripped nor mapped on the default claude_cli path or the OpenAI-compatible and Gemini paths, where they stay as inert text. Severity lowered p1 to p3: the cost is a missed caching optimisation plus a few bytes of prompt noise, not broken behaviour.
