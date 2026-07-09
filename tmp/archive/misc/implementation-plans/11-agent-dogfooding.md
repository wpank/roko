# 11 — Agent Dogfooding: Multi-Repo Autonomous Agents

> **Priority**: 🔴 P0 — Core value proposition: roko agents running autonomously across repos
> **Depends on**: Learning wiring (05, done), TUI dashboard (09, independent)
> **Status**: Not started
> **Scope**: 9 phases, 5 new crates, 16 agent templates, 21+ subscriptions, ~235 checklist items

---

## Problem Statement

Roko has a working plan-execute-gate-persist loop but agents only run when a human types
`roko plan run`. The system can't yet:

1. **React to external events** (GitHub pushes, Slack messages, cron schedules)
2. **Interact with external services** (post PR reviews, send Slack messages)
3. **Run persistently** as a daemon/service
4. **Manage multiple repos** from a single roko instance
5. **Wrap existing scripts** (30+ Python automations across collaboration/knowledge-base repos)
6. **Autonomously implement PRDs** (the full PRD → plan → implement → PR workflow)

This plan turns roko from a CLI tool into an **agent platform**.

---

## Design Philosophy

### Generality Over Specificity

The collaboration/knowledge-base agents are **one instantiation**. The architecture
equally supports coding agents, blockchain agents, DevOps agents, research agents,
and any custom domain. Every design decision is evaluated against: "Could a developer
create a new agent type in under 30 minutes?"

### Cybernetic Principles

```
Event → Agent → Action → Outcome → Feedback Signal → Learning → Better Agent
  ↑                                                                    │
  └────────────────────────────────────────────────────────────────────┘
```

### Modularity

```
roko-core          ← pure types + traits
  ↓
roko-plugin        ← Integration/EventSource/FeedbackCollector traits
  ↓
roko-serve         ← extracted from roko-cli, reusable server library
  ↓
roko-mcp-*         ← standalone MCP server binaries
  ↓
roko-cli           ← thin CLI wrapper
```

---

## Phase Map

```
Phase 0 (Refactoring)
  0.1 Extract roko-serve ─────┐
  0.2 Create roko-plugin ─────┤
  0.3 Kind constants ─────────┘
         │
         ▼
Phase 1 (Event Ingress)          Phase 2 (MCP Servers)
  1.1 Webhook routes ─────┐      2.1 GitHub MCP ──────┐
  1.2 WebhookReceived ────┤      2.2 Slack MCP ───────┤
  1.3 Subscriptions ──────┤      2.3 Script wrapper ──┤
  1.4 Dispatch loop ──────┘      2.4 MCP discovery ───┘
         │                              │
         └──────────┬───────────────────┘
                    ▼
Phase 3 (Agent Templates)          Phase 4 (Scheduler + FS Watch)
  3.0 Enhanced schema ─────┐       4.1 Cron scheduler ────┐
  3.1 Collab templates ────┤       4.2 File watcher ──────┤
  3.2 KB templates ────────┤       4.3 Wire EventSources ─┘
  3.3 Cross-repo templates ┤
  3.4 Subscription configs ┤
  3.5 Validation ──────────┘
                    │
                    ▼
Phase 5 (Daemon + Deploy)          Phase 6 (Multi-repo Config)
  5.1 Daemon wiring ───────┐       6.1 Subscription format ──┐
  5.2 Launchd plist ───────┤       6.2 Multi-repo loading ───┤
  5.3 Cloud deploy ────────┤       6.3 Per-repo init ────────┤
  5.4 Remote orchestrator ─┘       6.4 Secret management ────┘
                    │
                    ▼
Phase 7 (Learning)                 Phase 8 (PRD Workflow)
  7.1 Episode logging ─────┐       8.1 End-to-end flow ──────┐
  7.2 Prompt experiments ──┤       8.2 PRD ingestion agent ──┤
  7.3 Feedback collection ─┤       8.3 Auto-plan agent ──────┤
  7.4 HDC integration ─────┤       8.4 Code implementer ─────┤
  7.5 Metrics dashboard ───┘       8.5 Review response ──────┤
                                   8.6 PM integration ───────┤
                                   8.7 Cross-repo coord ─────┤
                                   8.8 Safety ────────────────┤
                                   8.9 E2E verification ─────┘
```

---

## Detailed Plans (by phase)

| Phase | File | Lines | Summary |
|---|---|---|---|
| 0-1 | [phase-0-1.md](11-sections/phase-0-1.md) | 1,237 | Extract roko-serve, create roko-plugin SDK, signal constants, webhook endpoints, subscriptions, dispatch loop |
| 2 | [phase-2.md](11-sections/phase-2.md) | 917 | roko-mcp-github (17 tools), roko-mcp-slack (8 tools), roko-mcp-scripts (wraps 30+ Python scripts) |
| 3-4 | [phase-3-4.md](11-sections/phase-3-4.md) | 1,200 | 16 agent templates with full system prompts, 21 subscriptions, cron scheduler, file watcher |
| 5-6 | [phase-5-6.md](11-sections/phase-5-6.md) | 756 | Daemon lifecycle, launchd, cloud deploy, remote orchestrator, multi-repo config, secrets |
| 7-8 | [phase-7-8.md](11-sections/phase-7-8.md) | 1,027 | Learning loops, feedback collection, HDC, metrics, PRD-driven autonomous development workflow |

---

## New Crates

| Crate | Type | Purpose |
|---|---|---|
| `roko-plugin` | lib | Integration/EventSource/FeedbackCollector traits (the developer SDK) |
| `roko-serve` | lib | HTTP server, dispatch, scheduler, watchers (extracted from roko-cli) |
| `roko-mcp-github` | bin | GitHub API as MCP tools (17 tools) |
| `roko-mcp-slack` | bin | Slack Web API as MCP tools (8 tools) |
| `roko-mcp-scripts` | bin | Generic script wrapper (config-driven, any language) |

---

## Agent Templates (16 total)

| Template | Repo | Trigger | Tools |
|---|---|---|---|
| doc-lifecycle-agent | collaboration | push (docs/) | github, scripts |
| digest-agent | collaboration | weekly cron | slack, scripts, github |
| meeting-agent | collaboration | push (call-notes/) | github, slack, scripts |
| sync-agent | collaboration | 6h cron, /sync | slack, scripts |
| conflict-detector-agent | collaboration | push (docs/) | github, scripts |
| freshness-agent | collaboration | daily cron | slack, scripts |
| pm-board-agent | knowledge-base | 2h cron, push (pm/) | scripts, github |
| enrich-agent | knowledge-base | push (docs/) | scripts, github |
| triage-agent | knowledge-base | issue/PR opened | github, scripts |
| pm-health-agent | knowledge-base | daily cron | slack, scripts, github |
| action-tracker-agent | knowledge-base | daily cron | scripts, github, slack |
| pr-review-agent | roko | PR opened | github |
| slack-notify-agent | roko | agent events | slack |
| auto-plan-agent | roko | PRD push | github, slack |
| code-implementer-agent | roko | plan approved | github |
| gate-fixer-agent | roko | gate failed | github |

---

## End-to-End Verification

After all phases, this sequence works:

1. `roko daemon start` — loads all repo subscriptions, starts scheduler + watchers
2. Open a PR on any configured repo → `pr-review-agent` fires, posts review
3. Push canonical PRD to collaboration → `prd-ingestion-agent` syncs to roko
4. auto-plan-agent generates implementation plan, creates PR
5. Merge plan PR → `code-implementer-agent` implements, pushes PR
6. Review comments → `review-response-agent` handles them
7. Monday 9am → `digest-agent` generates weekly summary to Slack
8. Every 2h → `pm-board-agent` syncs GitHub state to TOML tasks
9. All episodes logged, cascade router optimizes model selection
10. Feedback signals track engagement, experiments optimize prompts

Full step-by-step test script in [phase-7-8.md § 8.9](11-sections/phase-7-8.md).
