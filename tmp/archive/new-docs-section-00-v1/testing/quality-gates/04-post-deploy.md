# Post-Deploy Gate

> Observability checks and smoke tests that run after a deployment. Alerting (not blocking).

**Status**: Specified
**Depends on**: deployment observability infrastructure (Phase 2)
**Last reviewed**: 2026-04-19

---

## TL;DR

Post-deploy checks verify that a deployed Roko instance is healthy and behaving as expected. They are observability-driven rather than assertion-driven: they check metrics and health endpoints rather than running tests.

---

## Smoke Tests

Smoke tests run against the deployed instance immediately after deployment:

```bash
# Health check
curl -f https://<instance>/health

# Substrate ping (read a known Engram)
roko status --check-substrate

# Gate pipeline ping (run a no-op gate evaluation)
roko gate ping --gate compile --input tests/fixtures/rust_projects/clean_project

# LLM backend ping (send a minimal prompt, check response format)
roko agent ping --model claude-3-5-sonnet
```

All smoke tests must pass within 60 seconds of deployment.

---

## Observability Metrics

After deployment, the following metrics are monitored for 30 minutes:

| Metric | Alert threshold | Notes |
|---|---|---|
| Gate pipeline error rate | > 5% | Errors (not verdicts) |
| Agent LLM error rate | > 10% | Backend failures |
| Substrate write latency p99 | > 500ms | |
| Orchestrator plan start rate | < expected baseline | Indicates scheduler issue |
| Memory RSS | > 2× baseline | Potential memory leak |

Alerts go to the on-call channel. They do not roll back the deployment automatically.

---

## Rollback Criteria

A rollback is triggered if any of the following occur within 30 minutes of deployment:
- Any smoke test fails.
- Gate error rate > 20%.
- The self-hosting loop produces 0 successful task completions in 15 minutes (if running).

---

## Status

Post-deploy observability requires the `roko-serve` metrics endpoint and the deployment monitoring infrastructure (planned for Phase 2). Currently, smoke tests are run manually after each release.

---

## See also

- [03-pre-release.md](03-pre-release.md) — pre-release checks
- [operations/error-handling/](../../operations/error-handling/) — error taxonomy used in monitoring
