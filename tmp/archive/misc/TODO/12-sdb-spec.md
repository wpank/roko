# sdb-spec/ — Dashboard Backend Specifications

**Directory**: `tmp/sdb-spec/`
**Status**: DONE — all 10 specs fully implemented
**SDB** = "Sam's Dashboard Backend"

## What This Is

10-item implementation checklist for wiring the Nunchi dashboard backend (mirage-rs + roko-serve) to support real agent operations, predictions, messaging, and learning metrics. Derived from GitHub #45.

## Specification Status

| # | Spec | Target | Priority | Status | Key Source File |
|---|------|--------|----------|--------|-----------------|
| 01 | Agent `owner` field | mirage-rs | P0 | DONE | `apps/mirage-rs/src/chain/agent.rs` |
| 02 | Agent skills endpoints | mirage-rs | P0 | DONE | `apps/mirage-rs/src/http_api/skills.rs` |
| 03 | C-Factor + cost tier endpoints | roko-serve + mirage-rs | P1 | DONE | `crates/roko-serve/src/routes/learning.rs` |
| 04 | Task artifacts & metadata | mirage-rs | P1 | DONE | `apps/mirage-rs/src/chain/task.rs` |
| 05 | Agent messaging (POST + WS) | roko-serve + mirage-rs | P0 | DONE | `crates/roko-serve/src/routes/agents.rs:231` |
| 06 | ISFR proxy endpoints | mirage-rs | P1 | DONE | `apps/mirage-rs/src/http_api/isfr.rs` |
| 07 | Prediction endpoints (Mirofish) | mirage-rs | P1 | DONE | `apps/mirage-rs/src/chain/prediction.rs`, `http_api/prediction.rs` |
| 08 | `roko chat` CLI REPL | roko-cli | P1 | DONE | `crates/roko-cli/src/chat.rs` |
| 09 | Research intent field | roko-serve | P2 | DONE | `crates/roko-serve/src/routes/research.rs:70` |
| 10 | Task improve/feedback | mirage-rs | P2 | DONE | `apps/mirage-rs/src/http_api/task.rs:406` |

## Implementation Details

### Prediction Engine (Spec 07 — largest)

Full Mirofish prediction engine with:
- `PredictionSession` lifecycle: Dispatching -> Collecting -> Registered -> Pending -> Resolved
- `PredictionClaim` with confidence, difficulty_weight, residuals
- Difficulty weight: `domain_variance * novelty * tightness`
- `CalibrationSummary` per agent/category (mean_bias, coverage_rate)
- 7 REST endpoints + WebSocket broadcasting

### Agent Messaging (Spec 05)

- `POST /api/agents/{id}/message` replaces browser-side OpenRouter calls
- Returns `run_id` for polling/streaming
- Wired with event bus: `ServerEvent::RunStarted/RunCompleted/Error`

### Skills Config (Spec 02)

8 configurable skills per agent: ISFR Observer, DeFi Router, Risk Sentinel, Knowledge Curator, Prediction Agent, Market Maker, Hedge Agent, Self-Tuner

## Post-Demo Items (Future Work)

These are noted in the specs but not yet implemented:

- [ ] Stream CRUD infrastructure (`POST /api/streams`)
- [ ] Data provider registry (`/api/data/feeds`)
- [ ] Chat persistence (SQLite on roko-serve)
- [ ] Multi-agent conversations
- [ ] IPFS artifact storage
- [ ] Valhalla privacy tier enforcement
- [ ] Full Mirofish dispatch (claims -> EVM grading)
- [ ] Subscription tier enforcement (Privy + points)

## No Remaining Core Action

All 10 core specs are implemented. Post-demo items are tracked above for future reference.

**Source files**: `tmp/sdb-spec/01-agent-owner-field.md` through `10-task-improve-feedback.md`
