+++
id = "find-868aa8"
kind = "finding"
title = "[provider F183] Generation-locked slug patterns across all model families"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-agent/translate"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F183"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F183"
anchors = ["crates/roko-agent/src/translate/capability.rs:64", "crates/roko-core/src/tool/format.rs:232-338", "crates/roko-agent/src/token_estimator.rs:222-236", "crates/roko-gateway/src/provider.rs:99-111"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
The Kimi case is not unique: roughly 25 heuristic surfaces across the workspace pin slug patterns to current model generations — `glm-5`/`glm-5.1` in capability.rs:64, generation-specific arms for gemma-4, qwen3.5/3, llama4/llama3, and mistral-7b in tool/format.rs:232-338, `gemini-2.5`/`gemini-3`...

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F183`
- `tmp/archive/provider-audit/08-cascade-router.md`
- `tmp/archive/provider-audit/09-model-registry.md`
- `tmp/archive/provider-audit/24-abstraction-quality.md`

How to verify: SH-2 widened version-locked arms (done); SH-1 central ModelFamily classifier deferred. Check remaining generation locks. Confirm in crates/roko-agent/src/translate/capability.rs:64, crates/roko-core/src/tool/format.rs:232-338, crates/roko-agent/src/token_estimator.rs:222-236 whether still true: Generation-locked slug patterns across all model families
