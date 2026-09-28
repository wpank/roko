+++
id = "find-7a69c2"
kind = "finding"
title = "[cli-audit architecture] Three separate doctor implementations with divergent checks"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/doctor"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/cli-audit-2026-09-21/SUMMARY.md#Architectural Issues"
discovered_from = "audit:tmp/archive/cli-audit-2026-09-21/SUMMARY.md#Architectural Issues"
anchors = ["crates/roko-cli/src/doctor.rs", "crates/roko-cli/src/config_cmd.rs", "crates/roko-cli/src/chat_inline.rs", "backlog #279/#320"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
doctor.rs, config_cmd.rs, and chat_inline.rs each implement doctor checks with no shared code (plus runner/preflight.rs overlap). #320 fixed exit status only; consolidation owned by ENGINE #279 (fold #104).

Imported without verification from:
- `tmp/archive/cli-audit-2026-09-21/SUMMARY.md#Architectural Issues`
- `tmp/archive/cli-audit-2026-09-21/11-doctor.md`
- `tmp/archive/cli-audit-2026-09-21/FINDINGS-COVERAGE-MATRIX.md#Primary CLI audit (00–31)`

Some cited files are gone: `crates/roko-cli/src/chat_inline.rs`.

How to verify: Check whether doctor/config doctor/chat doctor share one check registry and severity->exit contract.
