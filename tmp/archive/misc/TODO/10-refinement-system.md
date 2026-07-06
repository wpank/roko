# refinement-audit-runner/, refinements/, refinements-audit/, refinements-runner/ — Refinement System

**Status**: ACTIVE — Phase 1 done, Phase 2 stuck, refinements branch pending merge

These four directories form a connected pipeline: 35 refinement proposals -> audit of proposals -> batch execution -> audit-informed re-execution.

---

## refinements/ — 35 Architectural Proposals

**Directory**: `tmp/refinements/`
**Files**: 35 markdown docs (REF01–REF35)
**Status**: DESIGN COMPLETE, IMPLEMENTATION BLOCKED on kernel refactor decision

### Arc Structure

| Arc | Docs | Scope | Status |
|-----|------|-------|--------|
| Foundation (01-09) | Pulse/Bus/Datum/Loop reframing | Kernel redesign | Blocked on stakeholder commit |
| Learning (10-16) | HDC, demurrage, c-factor, heuristics | Memory & intelligence | Awaiting kernel |
| Moat (17-21) | Plugins, modularity, redesigns | Ecosystem scaling | Deferred |
| UX (22-30) | SDK, CLI, TUI, StateHub, Web UI | Developer/user experience | Pick 3 of 9 |
| Integrators (31-35) | Synergy, safety, observability, glossary, roadmap | Cross-cutting | Reference + selective implementation |

### Key Concepts Proposed

- **Pulse**: Ephemeral event type (complement to durable Engram)
- **Bus trait**: First-class transport in kernel (fixes conductor->learn layer violation)
- **Datum enum**: Generalized operator input (Engram | Pulse)
- **7-step loop**: SENSE, ASSESS, COMPOSE, ACT, VERIFY, PERSIST+BROADCAST, REACT
- **Plugin SPI**: 5-tier extension architecture
- **StateHub**: Kernel-tier projection layer (currently TUI-only)

### What's Already In Code

| Concept | Codebase Location | Status |
|---------|-------------------|--------|
| Engram type | `crates/roko-core/src/engram.rs` | Wired |
| Six Synapse traits | `crates/roko-core/src/traits.rs` | Wired |
| EventBus<E> | `crates/roko-runtime/src/event_bus.rs` | Exists (not promoted to Bus trait) |
| HDC vectors | `crates/roko-primitives/src/hdc.rs` | Scaffolded (no Engram fingerprint) |
| CascadeRouter | `crates/roko-learn/src/cascade_router.rs` | Wired |
| SafetyLayer | `crates/roko-agent/src/safety/` | Wired |

### What's Absent

- Pulse type (0 code)
- Bus trait in roko-core (0 code)
- Datum enum (0 code)
- Demurrage balance model (0 code)
- Heuristic falsifiers (0 code)
- Plugin tiers 4-5 / WASM host (0 code)
- StateHub extraction from TUI (0 code)
- Web UI (0 code)

**Source files**: `tmp/refinements/REF01.md` through `REF35.md`

---

## refinements-audit/ — Audit of Proposals

**Directory**: `tmp/refinements-audit/`
**Files**: 8 audit documents + matrix + reality check
**Status**: DONE — highly relevant guidance
**Generated**: 2026-04-17

### Core Finding

> The refinements are **directionally right but 5-10x overscoped**. Correctly diagnose real problems but propose solutions calibrated for 5-7 engineers over 12 months, not 1 developer + AI agents.

### The "Ship Now" List (1 week total)

- [ ] Add `fingerprint: Option<HdcVector>` to Engram — `crates/roko-core/src/engram.rs`
- [ ] Unify 4 incompatible event enums into `RokoEvent` — across 4 crates
- [ ] Add generic `Bus<E>` trait to roko-core — `crates/roko-core/src/traits.rs` (~100 lines)
- [ ] Clean up ~40 stale "Signal" references — traits.rs, README, kind.rs, CLAUDE.md
- [ ] Fix architecture INDEX status (says serve/TUI "not wired" — both are) — `docs/00-architecture/INDEX.md`

### The "Ship Soon" List (next month)

- [ ] CLI parity (REF28): interactive entry, slash commands, diff-first output
- [ ] StateHub hardening (REF26): cursor tracking, reconnect-with-replay
- [ ] Heuristic calibration struct: extend HeuristicRule with trials/confirmations/Brier score
- [ ] Safety: extend Attestation + expand taint from bool to enum
- [ ] Standalone threat model doc (from REF32)

### Explicitly Deferred

- Pulse type (use unified RokoEvent instead)
- Datum enum (premature abstraction)
- Demurrage economy (add last_used + access_count first)
- Plugin tiers 4-5 (zero plugin authors)
- 3 new kernel crates (only roko-bus justified)
- All 5 rewrite candidates
- SvelteKit web UI, gRPC wire protocol
- 12-month roadmap timeline

### Per-Arc Verdicts

| Arc | Verdict |
|-----|---------|
| Foundation (01-09) | Diagnosis correct; prescription overcomplicated. Use RokoEvent + Bus trait, skip Pulse/Datum |
| Learning (10-16) | Underestimates existing code (35K LOC). HDC fingerprint is highest-value single change |
| Moat (17-21) | Aspirational fiction. Of 10 "moat components", 2 exist fully, 2 partially, 6 not at all |
| UX (22-30) | Pick 3: CLI parity (28), StateHub (26), chat subset (23). Skip the other 6 |
| Integrators (31-35) | Extend existing SafetyLayer, don't replace. Glossary 60% accurate. Roadmap 5x overstaffed |

**Source files**: `tmp/refinements-audit/00-INDEX.md` through `08-simpler-target-architecture.md`

---

## refinements-runner/ — Batch Execution System

**Directory**: `tmp/refinements-runner/`
**Files**: Runner script + lib/ + context-pack/ + prompts/ + logs/
**Status**: DONE — all 35 batches executed successfully

### Execution Results

| Metric | Value |
|--------|-------|
| Batches | 35/35 SUCCESS |
| Duration | ~14 hours (2026-04-16 22:15 to 2026-04-17 12:29) |
| Model | GPT-5.4, high reasoning |
| Files changed | 368 |
| Lines added | +55,020 |
| Lines removed | -23,099 |

### Output Branch

**Branch**: `codex/refinements-run-20260416-221511`
**Status**: Exists but NOT merged to main. Needs rebase + conflict resolution.

- [ ] Review refinements branch for quality
- [ ] Rebase onto current main (significant divergence — 861 commits ahead)
- [ ] Resolve conflicts and merge

### Infrastructure

Reusable runner with CLI:
```bash
bash tmp/refinements-runner/run-refinements.sh --list          # List batches
bash tmp/refinements-runner/run-refinements.sh --group foundation  # Run by group
bash tmp/refinements-runner/run-refinements.sh --continue last     # Resume
```

**Source files**: `tmp/refinements-runner/run-refinements.sh`, `lib/`, `context-pack/`, `prompts/REF*.prompt.md`

---

## refinement-audit-runner/ — Audit-Informed Re-Execution

**Directory**: `tmp/refinement-audit-runner/`
**Files**: Runner scripts + context-pack/ + lib/ + prompts/ + logs/
**Status**: ACTIVE — Phase 1 done, Phase 2 stuck on verify gate failures

### Three-Phase Pipeline

| Phase | Batches | Purpose | Status |
|-------|---------|---------|--------|
| Phase 1 (AUD01-08) | 8 | Apply audit verdicts to docs (narrow/defer/rewrite) | DONE |
| Phase 2 (PU00-12) | 13 | Refresh parity content with audit-refined docs | STUCK (PU00-01 verify_failed) |
| Phase 3 (PE00-12) | 13 | Execute code changes from parity docs | NOT STARTED |

### Phase 2 Blockers

- [ ] Investigate PU00 & PU01 verify gate failures (diff gate rejected)
- [ ] Check `pu-run-20260418-*` retry logs for resolution
- [ ] Unblock Phase 2 to enable Phase 3 code execution

### Source Files

- Runners: `tmp/refinement-audit-runner/run-*.sh`
- Context pack: `tmp/refinement-audit-runner/context-pack/`
- Phase 1 logs: `tmp/refinement-audit-runner/logs/run-20260417-214125/`
- Phase 2 retries: `tmp/refinement-audit-runner/logs/pu-run-20260418-*/`
