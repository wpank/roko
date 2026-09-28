+++
id = "bug-cf9670"
kind = "bug"
title = "DOCS-03/06 A5,E4,E5,TD-15: Silent or broken CLI flags (--json, --role, config set --global, status daemon check)"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli"]
created = 2026-09-15
updated = 2026-09-28
source = "tmp/docs-audit/03-SUPERSEDED-AND-DEAD.md#Silent/Broken Flags"
discovered_from = "audit:tmp/docs-audit/03-SUPERSEDED-AND-DEAD.md#Silent/Broken Flags"
anchors = ["--json", "roko config set --global", "roko status", "roko research --role"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
--json accepted but ignored on ~15+ commands; --role hardcoded in research (PRD) commands; `config set --global` silently ignored; `status` daemon check unreachable. Decision: FIX ALL. 09-20 says all flag P2s done (P2-FLG-5/6); per-flag status unconfirmed.

Imported without verification from:
- `tmp/docs-audit/03-SUPERSEDED-AND-DEAD.md#Silent/Broken Flags`
- `tmp/docs-audit/06-POTENTIAL-BACKLOG.md#A5. `--json` flag silently ignored on ~15+ commands`
- `tmp/docs-audit/06-POTENTIAL-BACKLOG.md#E4. `config set --global` flag`
- `tmp/docs-audit/07-TECH-DEBT.md#TD-15: `--role` Hardcoded in Research/PRD Commands`
- `tmp/dogfood/2026-09-18-session.md#Final Summary`
- `tmp/dogfood/2026-09-20-final-session.md#P2 (Medium)`
- `tmp/archive/dogfood-audit-2026-09-03/06-status-update-2026-09-03.md#Cross-Audit Findings (from 2026-09-01 register)`

How to verify: Run each flag and diff output/behavior.
