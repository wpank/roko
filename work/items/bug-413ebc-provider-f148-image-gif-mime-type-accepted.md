+++
id = "bug-413ebc"
kind = "bug"
title = "[provider F148] image/gif MIME type accepted but may fail at OpenAI endpoint"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-core/foundation"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F148"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F148"
anchors = ["crates/roko-core/src/foundation.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
`validate_model_input_images` allows `image/gif`. Anthropic supports GIF (first frame only). OpenAI does not support GIF in `image_url` data URIs. No per-provider MIME type filtering exists.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F148`
- `tmp/archive/provider-audit/17-image-vision.md`

How to verify: Confirm in crates/roko-core/src/foundation.rs whether still true: `image/gif` MIME type accepted but may fail at OpenAI endpoint
