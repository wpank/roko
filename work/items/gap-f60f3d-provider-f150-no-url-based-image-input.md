+++
id = "gap-f60f3d"
kind = "gap"
title = "[provider F150] No URL-based image input"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/multimodal"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F150"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F150"
anchors = ["crates/roko-agent/src/multimodal.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Both Anthropic and OpenAI support HTTP URL image references. Roko only supports base64 inline data. Users must pre-fetch and encode images locally.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F150`
- `tmp/archive/provider-audit/17-image-vision.md`

How to verify: Roadmap P4-8 (URL image input) deferred. Confirm in crates/roko-agent/src/multimodal.rs whether still true: No URL-based image input
