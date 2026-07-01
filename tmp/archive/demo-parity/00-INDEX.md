# Demo Parity: Self-Contained Task Checklists

> **This directory supersedes `PRDs/IMPL-10-DEMO.md`.** The monolithic IMPL-10-DEMO file has been broken into 25 standalone checklists here. Each is more detailed, has correct APIs, and can be executed in isolation. Do not use IMPL-10-DEMO.md for agent execution.

**Audit update (2026-04-22):** completed backend/TUI task files were moved to `tmp/archive/04-21-26/demo-parity/`. Files remaining in this directory should be treated as active or blocked work. See `tmp/04-21-26/AUDIT-BOARD.md` for the file-by-file audit and proof.

Archived as complete:
- `B1-job-types.md`
- `B2-job-routes.md`
- `B3-incremental-watchers.md`
- `B4-server-persistence.md`
- `B5-research-jobs.md`
- `B7-heartbeats.md`
- `B8-ws-enrichment.md`
- `B9-auth-middleware.md`
- `B10-integration-test.md`
- `C1-marketplace-tab.md`
- `C2-atelier-tab.md`
- `C3-inspect-subviews.md`
- `C4-config-subviews.md`
- `C6-header-stats.md`

**Deadline:** Thursday April 24, 2026
**Total:** 25 tasks, 18,400+ lines, 81 agent-hours across 3 parallel streams

Each file is **fully self-contained** — give it to a Claude agent with no other context and it can execute the task. Every file includes:
- Context section explaining the repo, tech stack, and architecture
- Absolute file paths for everything
- Complete code (not pseudocode) with all imports/types
- `// MOCK:` tags on every mock value with wire-up instructions
- `- [ ]` checkboxes for every step
- Verification section with exact commands

## How to run

Start all 3 streams in parallel. Within each stream, tasks are sequential unless marked parallel-safe.

```
Stream A (Dashboard — nunchi-dashboard repo):
  A1 → A2 → A3 → A4 → A5 → A6 → A7 → A8 → A9
       ↑ parallel    (A3-A7 can parallelize after A1+A2 done)

Stream B (Backend — roko repo):
  B1 → B2 → B5 → B6
  B3 (parallel with B1-B2)
  B4 (parallel with B1-B2)
  B7 → B8 → B10
  B9 (parallel with B7-B8)

Stream C (TUI — roko repo):
  C1 → C2 (sequential — both modify tabs.rs)
  C3, C4 (parallel with each other, after C1)
  C5 (parallel with anything)
  C6 (after B7)
```

Cross-stream: A8 benefits from B2+B8. A9 needs B10. C6 needs B7.

## Stream A: Dashboard Complete Rewrite

| Task | File | Lines | Hours | What |
|------|------|-------|-------|------|
| A1 | [A1-project-setup.md](A1-project-setup.md) | 1,408 | 4 | Design system (20 colors, 14 components), 3 stores, router (23 routes), layouts, App.tsx rewrite |
| A2 | [A2-api-layer.md](A2-api-layer.md) | 836 | 3 | 27 TypeScript types, 18 query hooks, 10 mutations, WebSocket client with reconnect |
| A3 | [A3-landing-page.md](A3-landing-page.md) | 647 | 4 | Interactive landing: hero, architecture explorer, VCG auction, stigmergy canvas, stats |
| A4 | [A4-observatory-pages.md](A4-observatory-pages.md) | 716 | 5 | LiveAgents, Plans, Learning, Conductor, Costs (5 pages) |
| A5 | [A5-network-pages.md](A5-network-pages.md) | 577 | 3 | AgentNetwork (force graph), PheromoneField (heatmap), KnowledgeGraph (search) |
| A6 | [A6-marketplace-pages.md](A6-marketplace-pages.md) | 729 | 5 | JobBoard, CreateJob, JobDetail + shared components (StatusTimeline, DeliverableViewer) |
| A7 | [A7-remaining-pages.md](A7-remaining-pages.md) | 1,162 | 5 | Agent Studio (4), Command (2), Atelier (3), Settings (1) — 10 pages |
| A8 | [A8-integration-polish.md](A8-integration-polish.md) | 442 | 4 | WS→query invalidation, right panel, responsive, contrast, loading/error/empty states |
| A9 | [A9-demo-rehearsal.md](A9-demo-rehearsal.md) | 287 | 2 | 3 end-to-end demo flows, full navigation audit, bug triage |
| | | **6,804** | **35** | |

## Stream B: Roko Backend Stabilization + Jobs

| Task | File | Lines | Hours | What |
|------|------|-------|-------|------|
| B1 | [B1-job-types.md](B1-job-types.md) | 740 | 3 | Job/JobState/FileJobStore in roko-core, state machine, unit tests |
| B2 | [B2-job-routes.md](B2-job-routes.md) | 494 | 3 | 9 REST routes in roko-serve, ServerEvent variants, CORS fix |
| B3 | [B3-incremental-watchers.md](B3-incremental-watchers.md) | 326 | 2 | IncrementalTailer for JSONL, replaces O(N) re-reads |
| B4 | [B4-server-persistence.md](B4-server-persistence.md) | 305 | 1 | ServerStateSnapshot save/restore, 30s auto-save |
| B5 | [B5-research-jobs.md](B5-research-jobs.md) | 487 | 4 | JobRunner, polls for open jobs, spawns research subprocess |
| B6 | [B6-coding-jobs.md](B6-coding-jobs.md) | 458 | 4 | Coding job execution: PRD → plan → run → gate → submit |
| B7 | [B7-heartbeats.md](B7-heartbeats.md) | 341 | 2 | HeartbeatPayload in roko-core, server routes, orchestrator emission |
| B8 | [B8-ws-enrichment.md](B8-ws-enrichment.md) | 188 | 2 | Verify job+heartbeat events flow through existing WS handler |
| B9 | [B9-auth-middleware.md](B9-auth-middleware.md) | 331 | 2 | Bearer JWT support alongside existing API key auth |
| B10 | [B10-integration-test.md](../../archive/04-21-26/demo-parity/B10-integration-test.md) | 583 | 3 | Full lifecycle curl tests + Rust integration test |
| | | **4,253** | **26** | |

## Stream C: TUI Enhancements

| Task | File | Lines | Hours | What |
|------|------|-------|-------|------|
| C1 | [C1-marketplace-tab.md](C1-marketplace-tab.md) | 913 | 4 | F8 tab + Marketplace view (job browser from .roko/jobs/) |
| C2 | [C2-atelier-tab.md](C2-atelier-tab.md) | 792 | 4 | F9 tab + Atelier view (PRD workshop, plan progress) |
| C3 | [C3-inspect-subviews.md](../../archive/04-21-26/demo-parity/C3-inspect-subviews.md) | 693 | 4 | F7: EngramDag, EpisodeReplay, KnowledgeBrowse rendering |
| C4 | [C4-config-subviews.md](../../archive/04-21-26/demo-parity/C4-config-subviews.md) | 672 | 3 | F6: ProviderHealth, ModelComparison rendering |
| C5 | [C5-bug-fixes.md](C5-bug-fixes.md) | 425 | 3 | 5 fixes: vfy column, log cache, wave collapse, git parser, status hints |
| C6 | [C6-header-stats.md](../../archive/04-21-26/demo-parity/C6-header-stats.md) | 236 | 2 | Network stats in header bar (agents online, ISFR) |
| | | **3,731** | **20** | |

## TUI Data Flow (for reference)

The TUI does NOT poll files for everything. It has a dual data path:

1. **StateHub push** (16ms latency): `orchestrate.rs` → `StateHub.publish(DashboardEvent)` → `watch<DashboardSnapshot>` → `App.drain_snapshot_channel()` → `TuiState.update_from_dashboard_snapshot()`
2. **File poll** (200ms debounce): `.roko/` file change → `notify` watcher → `FsRefresh::Coalesced` → `DashboardData::tick()` → `TuiState.update_from_snapshot()`
3. **WebSocket** (real-time): `AgentStreamClient` → `roko-serve /ws` → `StreamChunk` → `TuiState.agent_streams`

New TUI views (C1-C4) read from `TuiState` fields populated by these paths. Views that need local-only data (`.roko/jobs/`, `.roko/memory/`) read files directly — this is correct because the TUI must work without `roko-serve`.
