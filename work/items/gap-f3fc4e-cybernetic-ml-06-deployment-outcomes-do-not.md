+++
id = "gap-f3fc4e"
kind = "gap"
title = "[cybernetic ML-06] Deployment outcomes do not feed the learning system"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-learn"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/cybernetic-audit/28-feedback-loops.md#ML-06: Deployment Outcome -> Learning System"
discovered_from = "audit:tmp/archive/cybernetic-audit/28-feedback-loops.md#ML-06: Deployment Outcome -> Learning System"
anchors = ["DeploymentEvent", "DeploymentState"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
DeploymentEvent/DeploymentState are parsed and signal-converted but post-merge deployment success/failure never reaches provider health, cascade routing, or episode completion as late evidence.

Imported without verification from:
- `tmp/archive/cybernetic-audit/28-feedback-loops.md#ML-06: Deployment Outcome -> Learning System`

How to verify: Trace DeploymentEvent consumers; look for learning/health updates.
