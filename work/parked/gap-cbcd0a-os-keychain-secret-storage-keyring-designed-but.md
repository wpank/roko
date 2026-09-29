+++
id = "gap-cbcd0a"
kind = "gap"
title = "OS keychain secret storage (keyring) designed but not wired"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-core/config-secrets"]
created = 2026-09-15
updated = 2026-09-28
source = "docs/v3/depth/32-deployment/secret-management.md:276"
discovered_from = "audit:docs/v3/depth/32-deployment/secret-management.md:276"
anchors = ["roko config secrets set"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Secret management: env vars, dotenvy, ${VAR} interpolation and `roko config secrets` work, but OS keychain integration via the keyring crate is designed and not wired.

Imported without verification from:
- `docs/v3/depth/32-deployment/secret-management.md:276`

How to verify: grep for keyring crate usage in roko-core/roko-cli.
