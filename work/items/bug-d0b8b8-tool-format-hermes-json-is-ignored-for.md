+++
id = "bug-d0b8b8"
kind = "bug"
title = "tool_format hermes_json is ignored for openai_compat providers"
status = "open"
triage = "verified"
severity = "p2"
subsystem = ["roko-agent/providers"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-agent/src/provider/openai_compat.rs:509"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'cargo test -p roko-agent --lib provider::openai_compat'
+++

The openai_compat provider always builds its tool loop with `Arc::new(OpenAiTranslator)` (`provider/openai_compat.rs:509`), whatever the model profile's `tool_format` says.
Models configured for `hermes_json` (text `<tool_call>` blocks) get OpenAI-native tool requests and their calls are never parsed; per a local audit `build_body` also rejects the system-prompt block the Hermes translator emits.
Fix: pick the translator from the resolved profile's `tool_format`, splice its system block into the system message, omit empty `tools`, and test each format.
