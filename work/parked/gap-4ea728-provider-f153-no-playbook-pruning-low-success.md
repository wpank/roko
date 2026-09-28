+++
id = "gap-4ea728"
kind = "gap"
title = "[provider F153] No playbook pruning — low-success playbooks persist indefinitely"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-learn/playbook"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F153"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F153"
anchors = ["crates/roko-learn/src/playbook.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Playbooks are never removed regardless of how many times they fail. A playbook with a 10% success rate continues to be injected into system prompts, ranked lower but never eliminated.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F153`
- `tmp/archive/provider-audit/17-learning-loop.md`

How to verify: Confirm in crates/roko-learn/src/playbook.rs whether still true: No playbook pruning — low-success playbooks persist indefinitely
