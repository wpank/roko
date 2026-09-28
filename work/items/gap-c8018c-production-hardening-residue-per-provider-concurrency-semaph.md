+++
id = "gap-c8018c"
kind = "gap"
title = "Production hardening residue: per-provider concurrency semaphores scaffolded, graceful shutdown partial, hedged requests"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/resilience"]
created = 2026-09-15
updated = 2026-09-28
source = "docs/v3/depth/32-deployment/production-hardening.md:279"
discovered_from = "audit:docs/v3/depth/32-deployment/production-hardening.md:279"
anchors = ["roko-agent retry/backoff", "ProcessSupervisor"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Production-hardening doc: per-provider concurrency semaphores are scaffolded only, graceful shutdown is partial (ProcessSupervisor handles agents only), and hedged requests are designed but not implemented.

Imported without verification from:
- `docs/v3/depth/32-deployment/production-hardening.md:279`
- `docs/v3/depth/32-deployment/port-allocation.md:201`

How to verify: grep for Semaphore in roko-agent provider factory; test SIGTERM during `roko serve` and a plan run.
