+++
id = "find-de378f"
kind = "finding"
title = "[provider F061] Empty api_key_env = \"\" makes provider falsely \"available\""
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-core/config"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F061"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F061"
anchors = ["crates/roko-core/src/config/provider.rs", "api_key_env = \"\""]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
`ProviderConfig.api_key_env = ""` results in `is_available()` returning `true` (env var name exists, lookup of empty string returns empty string which is non-empty... or the field is empty so the check is skipped). The provider appears available but has no usable API key.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F061`
- `tmp/archive/provider-audit/06-model-config-resolution.md`

How to verify: Confirm in crates/roko-core/src/config/provider.rs whether still true: Empty `api_key_env = ""` makes provider falsely "available"
