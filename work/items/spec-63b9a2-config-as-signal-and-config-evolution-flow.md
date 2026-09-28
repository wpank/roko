+++
id = "spec-63b9a2"
kind = "spec"
title = "Config-as-Signal and config-evolution flow remain aspirational (v2 19-CONFIG)"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-core/config"]
created = 2026-08-16
updated = 2026-09-28
source = "docs/v2/19-CONFIG.md:13"
discovered_from = "audit:docs/v2/19-CONFIG.md:13"
anchors = ["roko_core::config::ConfigSource::Evolved", "ConfigWatchCallback"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
No Kind::Config; ConfigComposeCell/ConfigVerifyCell lineage aspirational; ConfigWatchTrigger/automatic FS-watch reload aspirational; L4 evolved-config proposal/approval/persistence flow not wired; automatic profile selection not applied.

Imported without verification from:
- `docs/v2/19-CONFIG.md:13`
- `docs/v2/19-CONFIG.md:249`
- `docs/v2/19-CONFIG.md:841`
- `docs/v2/19-CONFIG.md:902`

How to verify: Check v3 21-CONFIG for whether these are still intended; grep ConfigSource::Evolved producers.
