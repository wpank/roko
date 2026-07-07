---
title: "Readiness Audit: Deployment (§19)"
section: analysis
subsection: readiness-audit
id: ra-subsystem-19
source: 31-implementation-readiness-audit.md (§19)
score: 25/30
tags: [deployment, Docker, daemon, roko-serve, fly-toml, infrastructure]
---

# Readiness Audit: Deployment (§19)

**Score**: 25/30 | **Crate**: Native builds only. All deployment infrastructure exists as documentation only.

## Criterion Scores

| Criterion | Score | Notes |
|---|---|---|
| rust_structs | 4 | Native build config complete |
| pseudocode | 5 | Docker/fly.toml specs ready |
| config_params | 5 | Subscription config schema elegant |
| error_handling | 4 | Retry with full jitter partially implemented |
| integration_wiring | 3 | Zero actual deployment files created |
| test_criteria | 4 | Deployment test scenarios specified |

## Strengths

- Status doc: "exceptionally honest" (source file 31)
- Production hardening (adaptive timeouts, retry with full jitter) partially implemented in roko-agent
- Subscription config schema elegant

## Gaps

All deployment infrastructure exists as documentation only — zero actual files:
- No Dockerfiles
- No fly.toml
- No daemon mode
- No roko-serve implementation

## The Opportunity (G14)

Docker + deploy scripts could be created in **2-3 days** (Tier 1). This is the highest-ROI deployment gap.

## Cross-References

- [subsystem-lifecycle.md](./subsystem-lifecycle.md) — Daemon mode is a lifecycle dependency
