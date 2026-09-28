+++
id = "gap-fd44df"
kind = "gap"
title = "resolve_model() never returns an error; silently returns incomplete profiles for unknown models"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-core/config"]
created = 2026-09-01
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F012"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F012"
anchors = ["crates/roko-core/src/agent.rs::resolve_model", "crates/roko-core/src/config/model_registry.rs::builtin_model"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
When a model key is not found in the registry or `roko.toml`, `resolve_model()` returns a partially-constructed `ModelProfile` with `profile: None` rather than an error. Callers proceed with an unconfigured profile that may cause silent failures or incorrect API calls.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F012`
- `tmp/archive/provider-audit/06-model-config-resolution.md`

How to verify: Confirm in crates/roko-core/src/config/model_registry.rs whether still true: `resolve_model()` never returns an error; silently returns incomplete profiles for unknown models

Verified 2026-09-28: resolve_model lives in crates/roko-core/src/agent.rs:373, not model_registry.rs, and returns a bare ResolvedModel rather than a Result. After config key/slug/prefix lookups and a builtin_model registry fallback (agent.rs:400-431), unknown keys fall through to deprecated AgentBackend::from_model inference with no profile (agent.rs:432+). The builtin fallback now covers catalog models, so only unknown or typo keys reach the silent path. Severity p2.
