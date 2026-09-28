+++
id = "find-41176a"
kind = "finding"
title = "[provider F152] Seed playbooks dominate matching due to generic keywords"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-learn/playbook"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F152"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F152"
anchors = ["crates/roko-learn/src/playbook.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
The three seed playbooks (`minimal-edit`, `test-first`, `grep-before-write`) use common keywords that match nearly every implementation task. Learned specific playbooks may never be selected because seed playbooks consistently score higher on keyword overlap.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F152`
- `tmp/archive/provider-audit/17-learning-loop.md`

How to verify: Confirm in crates/roko-learn/src/playbook.rs whether still true: Seed playbooks dominate matching due to generic keywords
