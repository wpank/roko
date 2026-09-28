+++
id = "gap-db0dff"
kind = "gap"
title = "[provider F063] Two parallel secret systems not unified"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-core/config"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F063"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F063"
anchors = ["crates/roko-core/src/config/provider.rs", "crates/roko-core/src/secrets.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Provider `api_key_env` (environment variable lookup) and `SecretResolver` (5-layer resolution) are independent systems. Some code paths use `api_key_env`; others use `SecretResolver`. A key configured in one system may not be visible to the other.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F063`
- `tmp/archive/provider-audit/06-model-config-resolution.md`

Some cited files are gone: `crates/roko-core/src/secrets.rs`.

How to verify: Confirm in crates/roko-core/src/config/provider.rs, crates/roko-core/src/secrets.rs whether still true: Two parallel secret systems not unified
