# Agent Matchmaking — Backend Implementation Checklist

**Date:** 2026-04-22
**Source PRD:** `nunchi-dashboard/docs/agent-matchmaking.md` (branch `wp-demo-dashboard`)
**Target crate:** `roko-serve` (`crates/roko-serve/`)

## Context

The Nunchi dashboard's `/coding` flow shows a **quote** to the user before posting a bounty:
"N agents matched, total fee X KORAI, ETA Yh — accept/deny". The dashboard UI is built
(`agent_quote` bubble, `resolveAgentQuote` bridge, `useMatchAgents` hook). The backend
needs to support this flow.

## What already exists (correcting the PRD)

The PRD states that `jobs.rs` is absent and `/api/jobs` returns 404. **This is incorrect.**
The jobs module is fully wired:

| Endpoint | File | Status |
|---|---|---|
| `GET /api/jobs` | `crates/roko-serve/src/routes/jobs.rs:20` | **Wired** |
| `POST /api/jobs` | `crates/roko-serve/src/routes/jobs.rs:20` | **Wired** |
| `GET /api/jobs/stats` | `crates/roko-serve/src/routes/jobs.rs:21` | **Wired** |
| `GET /api/jobs/{id}` | `crates/roko-serve/src/routes/jobs.rs:22` | **Wired** |
| `PATCH /api/jobs/{id}` | `crates/roko-serve/src/routes/jobs.rs:23` | **Wired** |
| `DELETE /api/jobs/{id}` | `crates/roko-serve/src/routes/jobs.rs:24` | **Wired** |
| `POST /api/jobs/{id}/assign` | `crates/roko-serve/src/routes/jobs.rs:26` | **Wired** |
| `POST /api/jobs/{id}/start` | `crates/roko-serve/src/routes/jobs.rs:27` | **Wired** |
| `POST /api/jobs/{id}/submit` | `crates/roko-serve/src/routes/jobs.rs:28` | **Wired** |
| `POST /api/jobs/{id}/evaluate` | `crates/roko-serve/src/routes/jobs.rs:29` | **Wired** |
| `POST /api/jobs/{id}/execute` | `crates/roko-serve/src/routes/jobs.rs:30` | **Wired** |
| `POST /api/jobs/{id}/cancel` | `crates/roko-serve/src/routes/jobs.rs:31` | **Wired** |
| `POST /api/jobs/match` | — | **MISSING** |

Routes are mounted in `crates/roko-serve/src/routes/mod.rs:52` via `.merge(jobs::routes())`.
Jobs are persisted as individual JSON files in `.roko/jobs/{id}.json`.
State machine transitions are enforced. Events are emitted via `ServerEvent::JobCreated`,
`JobUpdated`, `JobTransitioned`.

## What's actually missing

1. **Agent enrichment fields** — `DiscoveredAgent` lacks `tier`, `reputation`, `skills`,
   `past_jobs_completed`, and load tracking needed for matchmaking ranking.
2. **Matchmaking endpoint** — `POST /api/jobs/match` does not exist.
3. **`committed_candidates` on job creation** — `CreateJobRequest` and `JobRecord` lack this
   field, which the dashboard needs to pass accepted candidates when posting.
4. **In-flight job tracking** — no mechanism to count how many jobs an agent currently has
   active, needed for the `1 - jobsInFlight/maxLoad` ranking signal.

## Task files

| # | File | What |
|---|---|---|
| 1 | [01-enrich-discovered-agent.md](01-enrich-discovered-agent.md) | Add tier/reputation/skills/load fields to DiscoveredAgent |
| 2 | [02-agent-load-tracking.md](02-agent-load-tracking.md) | Track in-flight jobs per agent |
| 3 | [03-matchmaking-endpoint.md](03-matchmaking-endpoint.md) | Implement `POST /api/jobs/match` |
| 4 | [04-committed-candidates.md](04-committed-candidates.md) | Add `committed_candidates` to job creation |
| 5 | [05-integration-test.md](05-integration-test.md) | End-to-end test: match → create → assign → complete |

**Dependency order:** Task 1 → Task 2 → Task 3 → Task 4 → Task 5
(Tasks 1 and 2 can be done in parallel; Task 3 depends on both; Task 4 is independent of 3;
Task 5 depends on all.)
