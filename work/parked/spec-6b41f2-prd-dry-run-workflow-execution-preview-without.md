+++
id = "spec-6b41f2"
kind = "spec"
title = "PRD: `--dry-run` workflow execution preview without LLM dispatch (published, 0% coverage)"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/run"]
created = 2026-05-02
updated = 2026-09-28
source = ".roko/prd/published/dry-run-flag.md"
discovered_from = "audit:.roko/prd/published/dry-run-flag.md"
anchors = ["roko run --dry-run"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Published PRD: `roko run --dry-run` resolves config, workflow/graph template, prompts and gates without dispatching to an LLM. No plans generated; coverage 0.

Imported without verification from:
- `.roko/prd/published/dry-run-flag.md`

How to verify: Run `roko run --help` for a dry-run flag; check plan run dry-run support.
