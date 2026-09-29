+++
id = "bug-d0b8b8"
kind = "bug"
title = "tool_format hermes_json is ignored for openai_compat providers"
status = "open"
triage = "verified"
severity = "p2"
goal = "hermes"
subsystem = ["roko-agent/providers"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-agent/src/provider/openai_compat.rs:509", "crates/roko-agent/src/translate/capability.rs::translator_for_profile"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'Arc::new(OpenAiTranslator)' crates/roko-agent/src/provider/openai_compat.rs && grep -q 'fn hermes_json_profile_uses_hermes_translator' crates/roko-agent/src/provider/openai_compat.rs && cargo test -p roko-agent --lib provider::openai_compat"
+++

The openai_compat provider always builds its tool loop with `Arc::new(OpenAiTranslator)` (`provider/openai_compat.rs:509`), whatever the model profile's `tool_format` says.
Models configured for `hermes_json` (text `<tool_call>` blocks) get OpenAI-native tool requests and their calls are never parsed; per a local audit `build_body` also rejects the system-prompt block the Hermes translator emits.
Fix: pick the translator from the resolved profile's `tool_format`, splice its system block into the system message, omit empty `tools`, and test each format.

Re-checked 2026-09-29: still open. The translator selector already exists as translate/capability.rs::translator_for_profile (and translator_for_capabilities); the fix should call it at openai_compat.rs:509 instead of hard-coding OpenAiTranslator. The existing openai_compat tests all use tool_format = "openai_json", so the current verify command passes while the bug is present.
