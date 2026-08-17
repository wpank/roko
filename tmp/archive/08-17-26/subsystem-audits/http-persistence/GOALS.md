# HTTP & Persistence: Goals

## End State

HTTP control plane serves as the unified API surface for dashboards, external integrations, and inter-agent communication. Persistence layer is consistent, deduplicated, and supports real-time streaming via StateHub.

## Key Properties

- **Single API surface**: All features accessible via REST + SSE + WebSocket (not just CLI).
- **StateHub push pattern**: `DashboardEvent` → `state_hub.publish()` → SSE/TUI consumers. Separate `EventBus<ServerEvent>` feeds WebSocket clients.
- **Deduplicated persistence**: One canonical location per data type (no .roko/learn/ vs .roko/memory/ duplication).
- **Real-time streaming**: Agent progress, gate results, plan execution all stream via SSE/WS.
- **Cost tracking API**: Cumulative per-session and per-task cost exposed via endpoints.
- **Multi-agent dashboard**: Parallel agent progress visible in real-time (ties to parallel agent feature).

## What Exists Today

- 208 `.route()` registrations across 30 modules (roko-serve); effective HTTP endpoint count is higher due to multi-method routes
- 39-field AppState
- SSE (`/api/events`, `/api/sse`) and WebSocket (`/ws`, `/roko-ws`) streaming via two separate buses (StateHub for SSE/TUI, EventBus for WS)
- 50+ persistence files across .roko/
- Atomic JSON (write-tmp-rename) + append-only JSONL patterns
- `experiments.json` is NOT created by default; it only exists after the first experiment write

## From v2 UX Showcase (9 Scenarios)

Every right-rail panel and every streaming primitive needs a corresponding SSE/WS event and/or REST endpoint for the web dashboard equivalent:

- **CostPanel data** — REST: GET /session/:id/cost (turn_cost, session_cost, budget, tokens, sparkline). SSE: cost_update event per turn.
- **RouterPanel data** — REST: GET /session/:id/router (tier_confidence, recent_decisions). SSE: router_decision event per model call.
- **KnowledgePanel data** — REST: GET /session/:id/knowledge (hits array). SSE: knowledge_injected event.
- **MCPPanel data** — REST: GET /session/:id/mcp (server list with call counts). SSE: mcp_call event.
- **PermissionScope data** — REST: GET /session/:id/permissions (scope array). SSE: permission_request / permission_granted events.
- **EpisodeScrubber data** — REST: GET /episodes/:id/timeline (events array with timestamps). REST: POST /episodes/:id/branch (fork from position).
- **SwarmGrid data** — SSE: swarm_update event with per-agent progress, gates, metrics. REST: GET /session/:id/swarm.
- **GateRow data** — SSE: gate_update event per gate. REST: GET /session/:id/gates.
- **Plan data** — SSE: plan_update event (entries, replan flag). REST: GET /session/:id/plan.
- **PhaseStrip data** — SSE: phase_change event. REST: GET /session/:id/phase.
- **AgentChat data** — SSE: agent_chat event (from_role, to_role, text). REST: GET /session/:id/chat.

### Data Persistence Required
- `sessions/{id}.json` — full session state including cost, router decisions, knowledge hits
- `episodes/{id}/timeline.json` — event timeline for replay scrubber
- `episodes/{id}/learnings.json` — extracted learnings from replay analysis
- `swarm/{id}/` — per-agent state files for parallel execution
- `permissions/{id}.json` — permission scope state and audit log

## Gap

- Persistence duplication between .roko/learn/ and .roko/memory/ (episodes.jsonl, knowledge-seeds.jsonl, and several other files exist in both)
- No cumulative cost tracking API (gateway_model_counters tracks per-model but no per-session endpoint)
- No multi-agent real-time dashboard (single-agent focus)
- AppState is monolithic (39 fields) — no decomposition yet
- SSE streams exist but not all events wired to them

---

## Sources

| File | What was verified |
|---|---|
| `crates/roko-serve/src/routes/mod.rs` | Total route count, WebSocket mount paths |
| `crates/roko-serve/src/state.rs` | AppState field count (39), field types |
| `crates/roko-serve/src/routes/sse.rs` | SSE route paths |
| `crates/roko-serve/src/routes/ws.rs` | WebSocket route paths |
| `.roko/learn/` (live) | Actual persistence files present |
| `.roko/memory/` (live) | Actual persistence files present |
