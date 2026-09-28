+++
id = "gap-a7a7b7"
kind = "gap"
title = "[provider F188] No heuristic-coverage validation anywhere — doctor, config validate, and --model accept any slug silently"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/doctor"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F188"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F188"
anchors = ["crates/roko-cli/src/doctor.rs:611-661", "crates/roko-cli/src/config_cmd.rs:1198-1264", "crates/roko-core/src/config/schema.rs:1532-1592", "crates/roko-cli/src/main.rs:314-316"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
No surface checks whether a configured slug is covered by the heuristic tables or the builtin registry. `roko doctor` (doctor.rs:611-661) validates only the legacy `agent.model` field against `config.models` and `builtin_model()`, never `[models.*]` entries or heuristic coverage. `roko config val...

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F188`
- `tmp/archive/provider-audit/06-model-config-resolution.md`
- `tmp/archive/provider-audit/22-config-surface.md`
- `tmp/archive/provider-audit/31-slug-heuristic-map.md`

How to verify: Roadmap SH-5 (doctor/config validate heuristic coverage) deferred. Confirm in crates/roko-cli/src/doctor.rs:611-661, crates/roko-cli/src/config_cmd.rs:1198-1264, crates/roko-core/src/config/schema.rs:1532-1592 whether still true: No heuristic-coverage validation anywhere — doctor, config validate, and --model accept any slug silently
