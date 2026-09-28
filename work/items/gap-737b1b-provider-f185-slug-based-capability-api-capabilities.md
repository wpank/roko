+++
id = "gap-737b1b"
kind = "gap"
title = "[provider F185] Slug-based capability API (capabilities_for/translator_for) has zero production callers — latent version-locked trap"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/translate"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F185"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F185"
anchors = ["crates/roko-agent/src/translate/capability.rs:63-107", "crates/roko-agent/src/translate/mod.rs:48-50", "capabilities_for", "translator_for"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
`capabilities_for` and the version-locked slug fast-paths behind it (`glm-5`/`glm-5.1` at capability.rs:64, `kimi-k2` at capability.rs:79) are reachable only through `translator_for`/`translator_name_for`, which have no production callers — all 36 workspace references are internal to capability.r...

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F185`
- `tmp/archive/provider-audit/09-model-registry.md`
- `tmp/archive/provider-audit/24-abstraction-quality.md`
- `tmp/archive/provider-audit/31-slug-heuristic-map.md`

How to verify: Roadmap SH-1 deferred (delete or reroute zero-caller capabilities_for/translator_for). Confirm in crates/roko-agent/src/translate/capability.rs:63-107, crates/roko-agent/src/translate/mod.rs:48-50 whether still true: Slug-based capability API (`capabilities_for`/`translator_for`) has zero production callers — latent version-locked trap
