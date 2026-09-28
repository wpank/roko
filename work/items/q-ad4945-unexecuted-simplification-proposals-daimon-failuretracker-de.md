+++
id = "q-ad4945"
kind = "question"
title = "Unexecuted simplification proposals: daimon->FailureTracker, delete pheromones and VCG, CascadeRouter 17->6 features"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["workspace"]
created = 2026-04-28
updated = 2026-09-28
source = "tmp/archive/08-17-26/subsystem-audits/MASTER-IMPLEMENTATION-PLAN.md#Phase 6: Retirement"
discovered_from = "audit:tmp/archive/08-17-26/subsystem-audits/MASTER-IMPLEMENTATION-PLAN.md#Phase 6: Retirement"
anchors = ["roko-daimon", "vcg_allocate", "CascadeRouter"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
WorkflowEngine plan Phase 6.4-6.7 proposed replacing the daimon (~40K) with a 2-rule FailureTracker, deleting the pheromone system (~68K), deleting VCG auction and shrinking CascadeRouter features. Later epics (E23/E25/E44) instead wired these systems; decide whether any simplification is still…

Imported without verification from:
- `tmp/archive/08-17-26/subsystem-audits/MASTER-IMPLEMENTATION-PLAN.md#Phase 6: Retirement`

How to verify: Confirm the subsystems are live (then close as rejected) or record an explicit simplification decision.
