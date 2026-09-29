+++
id = "find-7b5ba5"
kind = "finding"
title = "[provider F106] Playbook matching is lexically shallow — keyword overlap without semantic understanding"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-learn/playbook"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F106"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F106"
anchors = ["crates/roko-learn/src/playbook.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
`match_playbooks()` uses keyword overlap (case-insensitive word overlap ratio). Seed playbooks with 4 generic words (`"test"`, `"implement"`, `"verify"`, `"requirement"`) match nearly every implementation task. HDC semantic similarity, which is used for episode lookup, is not used for playbook ma...

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F106`
- `tmp/archive/provider-audit/17-learning-loop.md`

How to verify: Confirm in crates/roko-learn/src/playbook.rs whether still true: Playbook matching is lexically shallow — keyword overlap without semantic understanding
