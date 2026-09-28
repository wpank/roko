+++
id = "spec-82f519"
kind = "spec"
title = "HuggingFace Provider Integration"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-hf"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/59-huggingface-provider.md#59 — HuggingFace Provider Integration"
discovered_from = "audit:tmp/backlog/archive/59-huggingface-provider.md#59 — HuggingFace Provider Integration"
anchors = ["crates/roko-hf/", "crates/roko-core/src/config/provider.rs:275-295", "crates/roko-agent/src/provider/openai_compat.rs:193-208", "openai_compat.rs:221-226", "demo/demo-resources/provider-routing/roko.toml:37-41", "roko.toml", "owner/model-id", "crates/roko-hf/src/hub.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
[todo] CONSOLIDATED P3-PRV-1: deferred — useful capability addition, no blockers on other work. HuggingFace hosts thousands of open-source models (Llama, Mistral, Qwen, DeepSeek, Phi, Gemma, and others) through an OpenAI-compatible Inference Providers API at `https://router.huggingface.co/v1`. The…

Imported without verification from:
- `tmp/backlog/archive/59-huggingface-provider.md#59 — HuggingFace Provider Integration`
- `tmp/archive/CONSOLIDATED-BACKLOG-2026-09-23.md#P3-PRV-1 (Subsystem: Providers)`

Some cited files are gone: `crates/roko-hf/`, `crates/roko-hf/src/hub.rs`, `owner/model-id`.

How to verify: Check: A `roko.toml` with `[providers.huggingface]` (kind=openai_compat, base_url=https://router.huggingface.co/v1, api_key_env=HF_TOKEN) causes `roko config providers list` to show the provider.; `roko run "<prompt>" --model llama-3-3-70b` (or any… [evidence: CONSOLIDATED P3-PRV-1: deferred; 00-STATUS-SUMMARY 3. Open / P3 -- Low (Open): S | 7 |]
