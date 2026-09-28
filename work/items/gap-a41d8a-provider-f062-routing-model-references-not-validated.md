+++
id = "gap-a41d8a"
kind = "gap"
title = "[provider F062] Routing model references not validated against model registry"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-core/config"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F062"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F062"
anchors = ["crates/roko-core/src/config/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
`[routing] fast_task_model`, `balanced_task_model`, etc. in `roko.toml` accept any string. If these reference a model slug not in the registry or provider config, `resolve_model()` silently returns an incomplete profile.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F062`
- `tmp/archive/provider-audit/06-model-config-resolution.md`

How to verify: SH-5 text notes config validate has referential checks; confirm routing model refs validated. Confirm in crates/roko-core/src/config/ whether still true: Routing model references not validated against model registry
