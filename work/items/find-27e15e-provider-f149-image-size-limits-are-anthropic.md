+++
id = "find-27e15e"
kind = "finding"
title = "[provider F149] Image size limits are Anthropic-tuned; too restrictive for Gemini"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-core/foundation"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F149"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F149"
anchors = ["crates/roko-core/src/foundation.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
`MAX_MODEL_IMAGE_BYTES` (5 MiB) and `MAX_MODEL_IMAGE_TOTAL_BYTES` (20 MiB) match Anthropic's limits. Gemini supports up to 20 MB inline per image. Roko may reject valid Gemini images.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F149`
- `tmp/archive/provider-audit/17-image-vision.md`

How to verify: Confirm in crates/roko-core/src/foundation.rs whether still true: Image size limits are Anthropic-tuned; too restrictive for Gemini
