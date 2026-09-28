+++
id = "spec-afe4aa"
kind = "spec"
title = "Deep Hermes / Nous Research Integration"
status = "open"
triage = "verified"
severity = "p2"
goal = "hermes"
subsystem = ["roko-agent/hermes"]
created = 2026-09-07
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/172-deep-hermes-nous-integration.md#172 — Deep Hermes / Nous Research Integration"
discovered_from = "audit:tmp/backlog/172-deep-hermes-nous-integration.md#172 — Deep Hermes / Nous Research Integration"
anchors = ["crates/roko-agent/src/translate/hermes.rs::HermesXmlTranslator", "crates/roko-agent/src/hermes/", "crates/roko-agent/src/provider/hermes.rs"]
links = { depends_on = [], blocks = [], related = ["bug-7567eb", "bug-b14145", "bug-0a1729", "bug-d0b8b8", "gap-914f8c", "spec-aa8f5b"], supersedes = [], duplicate_of = "" }
+++
completes the only provider with native tool format, self-hosted inference,. Hermes is Nous Research's flagship open-weight model family (8B–405B parameters, 128K–512K context). Roko already has a `ProviderKind::Hermes` variant, three transport tiers (HTTP, CLI one-shot, ACP over stdio), a gateway…

Imported without verification from:
- `tmp/backlog/172-deep-hermes-nous-integration.md#172 — Deep Hermes / Nous Research Integration`

How to verify: Check: `HermesXmlTranslator` renders tools as `<tools>` XML and parses `<tool_call>` responses; Multi-tool parsing works (2+ tool calls in one response); Hybrid reasoning mode (`<think>` + `<tool_call>`) is handled correctly [evidence: 00-STATUS-SUMMARY 3. Open / P2 -- Medium (Open): XL | 7 |]

Verified 2026-09-28: still open. Changed p1 -> p2 to match its 00-STATUS-SUMMARY source. Phase 1 has landed: `HermesXmlTranslator` (crates/roko-agent/src/translate/hermes.rs:51) renders `<tools>`, parses multiple `<tool_call>` blocks, and documents skipping `<think>`; see bug-7567eb and bug-b14145 for parser defects. Phases 2-5 are absent: there is no `is_open_weight`, `SamplingDefaults`, `tool_format_override` or `LocalInferenceManager` in crates/*/src. The acceptance list in tmp/backlog/172-deep-hermes-nous-integration.md still has 58 unchecked boxes. The cited hermes/config.rs, hermes/gateway_service.rs and provider/hermes.rs exist under crates/roko-agent/src/.
