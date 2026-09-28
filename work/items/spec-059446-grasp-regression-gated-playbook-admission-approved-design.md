+++
id = "spec-059446"
kind = "spec"
title = "GRASP regression-gated playbook admission (approved design upgrade)"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-learn/playbooks"]
created = 2026-09-15
updated = 2026-09-28
source = "docs/v3/39-ROADMAP.md#1.1 GRASP Regression-Gated Playbook Admission"
discovered_from = "audit:docs/v3/39-ROADMAP.md#1.1 GRASP Regression-Gated Playbook Admission"
anchors = ["crates/roko-learn/src/playbook_rules.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Approved: test each candidate when/then playbook entry against a balanced held-out probe under a hard regression budget before admission; SiriuS failed-episode augmentation and SkillZip MDL compression related. GRASP gate is 'Target design, not yet implemented'.

Imported without verification from:
- `docs/v3/39-ROADMAP.md#1.1 GRASP Regression-Gated Playbook Admission`
- `docs/v3/depth/31-self-hosting/03-grasp-admission.md:245`

How to verify: grep playbook admission for regression probe logic.
