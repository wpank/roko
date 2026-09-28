+++
id = "find-170e77"
kind = "finding"
title = "[provider F099] gpt-5.5, gpt-5.4-mini are speculative model slugs not yet released"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-core/config"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F099"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F099"
anchors = ["crates/roko-core/src/config/registry.rs", "gpt-5.5", "gpt-5.4-mini"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
The built-in model registry includes `gpt-5.5` and `gpt-5.4-mini` as entries. These models do not exist in the OpenAI API at audit time. Routing to these models will fail with `ModelNotFound`.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F099`

How to verify: Confirm in crates/roko-core/src/config/registry.rs whether still true: `gpt-5.5`, `gpt-5.4-mini` are speculative model slugs not yet released
