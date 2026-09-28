+++
id = "bug-f090fc"
kind = "bug"
title = "DFA-01: `roko run` intent routing can silently execute a plan matching a one-word prompt"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/run"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/dogfood-audit-2026-09-03/01-findings-register.md#Implemented findings with additional context from the audits"
discovered_from = "audit:tmp/archive/dogfood-audit-2026-09-03/01-findings-register.md#Implemented findings with additional context from the audits"
anchors = ["commands/do_cmd.rs", "cmd_do"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
CLI audit: run delegated to do by flag presence and do classified intent by keywords; single-word prompts matching a plan slug on disk silently executed that plan. The workflow-audit CLI consolidation may have changed this path.

Imported without verification from:
- `tmp/archive/dogfood-audit-2026-09-03/01-findings-register.md#Implemented findings with additional context from the audits`

How to verify: Run `roko run <existing-plan-slug> --dry-run` and check routing.
