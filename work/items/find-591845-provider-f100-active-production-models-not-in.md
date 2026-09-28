+++
id = "find-591845"
kind = "finding"
title = "[provider F100] Active production models not in built-in registry"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-core/config"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F100"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F100"
anchors = ["crates/roko-core/src/config/registry.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
`sonar-deep-research`, `glm-5.1`, `kimi-k2.5`, and several Cerebras models are actively dispatched but not in the `BUILTIN_MODELS` registry. These models use `resolve_model()` fallback paths with incomplete profiles.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F100`

How to verify: Confirm in crates/roko-core/src/config/registry.rs whether still true: Active production models not in built-in registry
