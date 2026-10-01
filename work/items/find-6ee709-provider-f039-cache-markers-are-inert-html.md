+++
id = "find-6ee709"
kind = "finding"
title = "Cache markers are inert HTML comments, not translated to provider API cache_control"
status = "open"
triage = "verified"
severity = "p3"
goal = "core"
subsystem = ["roko-compose/system_prompt_builder"]
created = 2026-09-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d9e79e9d8"
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F039"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F039"
anchors = ["crates/roko-compose/src/system_prompt_builder.rs:1474", "crates/roko-cli/src/dispatch/prompt_builder.rs:1740", "crates/roko-agent/src/translate/claude.rs::inject_cache_markers_into_content", "crates/roko-agent/src/provider/anthropic_api/tool_loop.rs:466"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn cache_markers_are_stripped_for_non_anthropic_providers' crates/roko-agent/ && cargo test -p roko-agent cache_markers_are_stripped_for_non_anthropic_providers"
+++
The `SystemPromptBuilder` emits `<!-- cache:system -->` and `<!-- cache:session -->` HTML comment markers at stability tier boundaries. These are raw strings in the concatenated system prompt. The Anthropic API requires `cache_control: { type: "ephemeral" }` JSON fields in the content blocks. Tra...

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F039`
- `tmp/archive/provider-audit/15-prompt-composition.md`

How to verify: Confirm in crates/roko-compose/src/system_prompt_builder.rs, crates/roko-agent/src/translate/claude.rs whether still true: Cache markers are inert HTML comments, not translated to provider API `cache_control`

Verified 2026-09-28: partly fixed, and narrower than the title says. Both Anthropic API paths now turn the markers into cache_control blocks through translate/claude.rs::inject_cache_markers_into_content: ClaudeAgent (crates/roko-agent/src/claude_agent.rs:357, :375) and the anthropic_api tool loop (provider/anthropic_api/tool_loop.rs:466, :485, :526, :544). Still true: the markers are switched on for every provider (dispatch/prompt_builder.rs:1740, prompting.rs:59, prompt_assembly_service.rs:395) and are neither stripped nor mapped on the default claude_cli path or the OpenAI-compatible and Gemini paths, where they stay as inert text. Severity lowered p1 to p3: the cost is a missed caching optimisation plus a few bytes of prompt noise, not broken behaviour.

Rechecked 2026-09-29: unchanged. Remaining: the <!-- cache:system --> and <!-- cache:session --> markers still reach the claude_cli, OpenAI-compatible and Gemini paths as inert text. Either strip them outside the Anthropic API translators or call with_cache_markers only for Anthropic API targets.

## Notes

- 2026-10-01 (wk-model-truth): implemented on work/bug-3aa61f; cargo verification deferred to the batch check.
  `create_agent_for_model` strips the `<!-- cache:system -->` and `<!-- cache:session -->` markers from the system
  prompt for every provider kind but `AnthropicApi` (`translate::claude::strip_cache_markers` joins the segments
  between them), so the factory-built Claude CLI, OpenAI-compatible and Gemini agents get no inert markers; the
  Anthropic API translators still turn them into `cache_control` blocks. Not covered: roko-cli's own CLI invocations
  (`dispatch_v2::CliDispatchProvider::build_invocation`, which `roko chat` uses) build the command line without the
  factory. Tests: `cache_markers_are_stripped_for_non_anthropic_providers`,
  `strip_cache_markers_joins_the_segments_between_them`.
