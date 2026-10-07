+++
id = "bug-f090fc"
kind = "bug"
title = "DFA-01: `roko run` intent routing can silently execute a plan matching a one-word prompt"
status = "done"
triage = "verified"
severity = "p2"
subsystem = ["roko-cli/run"]
created = 2026-09-01
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "bfd36512f"
source = "tmp/archive/dogfood-audit-2026-09-03/01-findings-register.md#Implemented findings with additional context from the audits"
discovered_from = "audit:tmp/archive/dogfood-audit-2026-09-03/01-findings-register.md#Implemented findings with additional context from the audits"
anchors = ["commands/do_cmd.rs", "cmd_do"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-10-02
at_ts = "2026-10-02T20:40:15Z"
commit = "bfd36512f"
by = "roko-7d"
executor = "claude-session"
via = "manual"
forced = false
evidence = "Done in merge bfd36512f: `roko run` no longer classifies intent, so a prompt never selects an existing plan; a plan runs only when its directory is named (`roko run plans/<slug>`, `run_cmd.rs::plan_argument`; main_tests.rs `cli_parses_run_of_a_plan_directory`)."
+++
CLI audit: run delegated to do by flag presence and do classified intent by keywords; single-word prompts matching a plan slug on disk silently executed that plan. The workflow-audit CLI consolidation may have changed this path.

Imported without verification from:
- `tmp/archive/dogfood-audit-2026-09-03/01-findings-register.md#Implemented findings with additional context from the audits`

How to verify: Run `roko run <existing-plan-slug> --dry-run` and check routing.
