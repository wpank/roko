# ux/, ux-followup/, ux-followup-runner/, ux-refactoring/ — UX System

---

## ux/ — Architecture Specification

**Directory**: `tmp/ux/`
**Status**: DONE — spec complete, ~90% of Phase 1 implemented
**Files**: 7 docs (00-architecture-overview through 06-open-questions)

5-layer agent-native architecture replacing monolithic backend:
- L0: On-chain contracts (ERC-8004)
- L1: mirage-rs (pure EVM fork)
- L2: Per-agent HTTP servers (roko-agent-server)
- L2.5: Aggregator on roko-serve
- L3: roko-serve orchestration
- L4: Kauri Dashboard

### Implementation Status

| Component | Status | Source |
|-----------|--------|--------|
| roko-agent-server crate | DONE | `crates/roko-agent-server/` (~3,364 LOC) |
| Aggregator routes | DONE | `crates/roko-serve/src/routes/aggregator.rs` (~1,435 LOC) |
| Bearer auth | DONE | `crates/roko-agent-server/src/auth/` |
| Agent registration | DONE | `crates/roko-agent-server/src/registration.rs` |
| Dashboard URL migration | READY | Awaiting Sam to flip `NEXT_PUBLIC_API_URL` |
| Knowledge/pheromone chain-backing | NOT STARTED | Phase 2 |
| mirage-rs REST deletion | DEFERRED | Phase 3 (feature-gated, not deleted) |

No remaining action on spec docs.

---

## ux-followup/ — Follow-up Catalog (112 Items)

**Directory**: `tmp/ux-followup/`
**Status**: ACTIVE — 72 done, 40 open (0 P0, 31 P1, 9 P2)
**Files**: 16 files (00-INDEX through 15-safety-and-learning-closure)

### Per-File Status

| File | Total | Done | Open |
|------|-------|------|------|
| 01-verified-p0-bugs.md | 4 | 4 | 0 |
| 02-high-impact-quick-wins.md | 10 | 10 | 0 |
| 03-non-batch-followups.md | 6 | 5 | 1 |
| 04-t9-t19-residuals.md | 10 | 7 | 3 |
| 05-partially-wired-subsystems.md | 12 | 8 | 4 |
| 06-advanced-agent-backends.md | 6 | 0 | 6 |
| 07-spec-code-drift.md | 11 | 8 | 3 |
| 08-phase-2-vision.md | 6 | 0 | 6 |
| 09-hygiene-and-test-coverage.md | 11 | 8 | 3 |
| 10-stale-docs.md | 8 | 3 | 5 |
| 12-tui-event-parity.md | 11 | 4 | 7 |
| 13-session-state-mgmt.md | 4 | 3 | 1 |
| 14-observability-gaps.md | 6 | 5 | 1 |
| 15-safety-and-learning-closure.md | 7 | 7 | 0 |
| **TOTAL** | **112** | **72** | **40** |

### Self-Hosting Loop: FULLY CLOSED

- CLAUDE.md item 10 (auto-plan on PRD publish): DONE
- CLAUDE.md item 11 (gate failure -> replan): DONE

### Open Items Checklist (P1, by priority)

- [ ] Item 35a: Wire remaining 4 gate rungs (FactCheck, Symbol, GeneratedTest, PropertyTest) — 2 days
- [ ] Items 70-76: TUI event-parity polling→push migration (7 items) — Phase E, ~2 weeks
- [ ] Item 87: Per-gate pass/fail timeline widget — 2 days
- [ ] Items 64-67a: Stale docs (banners, terminology, paths) — 5 items, ~3 hours total
- [ ] Items 27-28a: Runner hardening + log retention + env knobs — 3 items, 2 days
- [ ] Item 56: clippy::missing_errors_doc fixes — 2-3 days
- [ ] Item 58: Flaky timeout-based tests — 1 day
- [ ] Item 60c: Cascade router e2e test — 2 days
- [ ] Items 45-47: Spec drift (integration tests, MORI checklist, stale banners) — 2-3 days
- [ ] Item 34: MCP server audit (4 crates) — 1 day
- [ ] Item 19: SystemPromptBuilder snapshot tests — 1 day
- [ ] Item 81: Snapshot migration framework — 1-2 days
- [ ] Item 32: Dreams feature gate — 2 hours
- [ ] Advanced backends (6 items): Codex/Cursor/streaming/test parity — not re-audited

### Phase 2 Items (9, Parked)

- Golem chain-witness, chain primitives, dreams consolidation
- TUI beyond-parity editors, HTTP auth/multi-tenant, roko-plugin fate

---

## ux-followup-runner/ — 47-Batch Automation System

**Directory**: `tmp/ux-followup-runner/`
**Status**: DONE — 47/47 batches executed (97.9% first-pass rate)
**Size**: 122 MB (mostly logs)

### Execution Results

| Metric | Value |
|--------|-------|
| Batches | 47/47 SUCCESS |
| Duration | 16h 53m (Apr 16-17) |
| Model | gpt-5.4, high reasoning |
| First-pass rate | 46/47 (97.9%) |
| Retries needed | 3 batches (UX42 clippy, UX43 tool, UX47 runner) |

### Batch Groups

| Group | Batches | Focus |
|-------|---------|-------|
| selfhost | UX01-04 | Self-hosting closure (P0) |
| tui-stream | UX05-11 | Replace TUI polling with streaming |
| state | UX12-14 | Snapshot versioning, resume |
| observ | UX15-22 | Dashboard widgets |
| wired | UX23-29 | Wire subsystems |
| backends | UX30-34 | Codex/Cursor harness |
| hygiene | UX35-42 | Thresholds, cleanup, validation |
| docs | UX43-47 | MORI parity, terminology, runners |

### Infrastructure

Reusable runner:
```bash
bash tmp/ux-followup-runner/run-ux-followup.sh --list
bash tmp/ux-followup-runner/run-ux-followup.sh --group selfhost
bash tmp/ux-followup-runner/run-ux-followup.sh --continue last
```

No remaining action on runner itself.

---

## ux-refactoring/ — 12-Batch Implementation

**Directory**: `tmp/ux-refactoring/`
**Status**: DONE — all 12 batches merged to main (`0102883e`)
**Executed**: 2026-04-15, ~6 hours

### Batch Results

| Batch | Scope | Status |
|-------|-------|--------|
| A1 | Dashboard foundations (owner/skills/artifacts) | DONE |
| A2 | ISFR/predictions/chat/research intent | DONE |
| B1 | Demo contracts/providers/yield routing | DONE |
| B2 | Demo modes (benchmark/tournament/autonomous/TUI) | DONE |
| C1 | roko-agent-server crate + builder | DONE |
| C2 | Aggregator + WS mux + mirage cleanup | DONE |
| D1 | Core attestation/lineage/tiering | DONE |
| D2 | DAG optimization/mutation/dreams | DONE |
| D3 | Heartbeat/daemon/pheromone/playbooks | DONE |
| E1 | Feedback loops (health/conductor/cost/skill/experiment) | DONE |
| F1 | Interactive TUI + serve routes | DONE |
| F2 | Tracing/daemon/playbooks/code MCP | DONE |

### Sections Covered

- **A**: Dashboard backend (10 tasks) — all SDB specs
- **B**: Demo features (18 tasks) — yield routing, benchmarks, autonomous
- **C**: Architecture migration (8 tasks) — agent-server, aggregator, feature gates
- **D**: Architectural gaps (44 tasks) — attestation, DAG, dreams, heartbeat
- **E**: Feedback loops (8 tasks) — cybernetic routing feedback
- **F**: TUI & interfaces (12 tasks) — ratatui, daemon, playbooks

No remaining action — all merged.
