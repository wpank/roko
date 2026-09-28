+++
id = "bug-1d250a"
kind = "bug"
title = "[cli-audit F188] Model slug validation is referential only; unregistered slugs validate clean"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-core/config"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/cli-audit-2026-09-21/08-config.md#Validation coverage note (2026-09-01, provider-audit F188/F189)"
discovered_from = "audit:tmp/archive/cli-audit-2026-09-21/08-config.md#Validation coverage note (2026-09-01, provider-audit F188/F189)"
anchors = ["roko-core/src/config/schema.rs validate_references", "config_cmd.rs phase 3 validation", "BUILTIN_MODELS", "backlog #370"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
config validate and validate_references only catch dangling provider/model refs; a configured-but-unregistered slug (e.g. kimi-k3) or --model/--force-model value is not checked against BUILTIN_MODELS. #370 semantic validation claimed done.

Imported without verification from:
- `tmp/archive/cli-audit-2026-09-21/08-config.md#Validation coverage note (2026-09-01, provider-audit F188/F189)`
- `tmp/archive/cli-audit-2026-09-21/11-doctor.md`

A source claims this was fixed; confirm against current code before closing.

How to verify: Configure an unknown slug and run `roko config validate`; expect a warning/error.
