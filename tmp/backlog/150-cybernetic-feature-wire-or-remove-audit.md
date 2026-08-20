# 150 — Cybernetic Feature Wire-or-Remove Audit

**Priority**: P3 — Several accepted epics have built subsystems that are not wired into the main dispatch path; each is either a wiring task or a documentation task, but the ambiguity creates maintenance debt and misleads contributors about what is actually live.
**Size**: M (2-3 days)
**Crates**: `crates/roko-daimon/`, `crates/roko-agent/`, `crates/roko-graph/`, `crates/roko-runtime/`
**Depends on**: None (audit first; wiring tasks become separate backlog items)
**Sources**: `tmp/backlog/_checklist-gaps.md` §5.2, `tmp/backlog/_mori-old-gaps.md` MO-21, MO-22, MO-23, MO-24

---

## Background

The CLAUDE.md and GAPS.md mark several subsystems as "complete" per their epic manifests, but the mori-old cybernetic features audit found that some are built but not wired into the main dispatch path. The distinction matters: a built-but-unwired subsystem has zero runtime impact despite appearing in acceptance counts.

Four subsystems need an explicit decision:

1. **`CorticalState` / Cognitive Autonomy (E23)**: `CorticalState` and `heartbeat.rs` (2,717 LOC) are built but no production code path instantiates `CorticalState`. The epic is 10/10 accepted. Decision needed: wire into runner dispatch loop OR document as "built, awaiting runner integration, not yet live."

2. **`EnrichedCell` / Cross-Cut Functors (E44)**: `EnrichedCell` is the cross-cut functor wrapper that should intercept every agent dispatch. CLAUDE.md says "live non-blocking gate-failure cascade are wired" but the mori-old audit found `dispatch_agent_with()` routes through `SharedAgentFactory` directly, not through `EnrichedCell`. Decision: confirm `EnrichedCell` is in the dispatch path OR open a wiring task.

3. **`roko-gateway` / Inference Gateway (E26)**: Marked "Complete (E26 12/12)" in CLAUDE.md. The mori-old audit found runner-v2 dispatches directly through provider adapters, bypassing the gateway. Decision: wire runner-v2 through `roko-gateway` OR document that the gateway is HTTP-only (not used by the runner).

4. **Agent Groups coordination modes (E28)**: Accepted at 8/8. The coordination modes enum has variants (Collaborative, Competitive, Sequential, Broadcast) but the actual inter-agent coordination behavior may be stub implementations. Decision: verify each mode is implemented OR document which modes are stubs.

For each subsystem, the audit output should be:
- **WIRE**: it is not wired, here is the 3-step plan to wire it.
- **CONFIRM-LIVE**: it is actually wired (the mori-old audit was wrong); here is the evidence.
- **DOCUMENT**: the subsystem is intentionally separate scope (HTTP-only, manual-only, etc.); update CLAUDE.md and GAPS.md to clarify.

## Current State

- `CorticalState`: `crates/roko-agent/src/` or a standalone crate; not instantiated in runner dispatch.
- `EnrichedCell`: `crates/roko-graph/src/` or `crates/roko-agent/src/`; dispatch path unclear.
- `roko-gateway`: `crates/roko-gateway/` (if it exists as a separate crate) or embedded in `roko-agent`.
- Agent Groups: `crates/roko-runtime/src/` or `crates/roko-agent/src/`.

## Implementation Plan

1. **CorticalState audit**:
   - Grep for `CorticalState::new` across all crates. Count call sites.
   - If zero call sites: write a stub integration plan (3 steps: create at runner startup, wire heartbeat tick, feed energy from efficiency events).
   - Update GAPS.md: "CorticalState not instantiated in production runner; integration is a separate task."

2. **EnrichedCell audit**:
   - Read `dispatch_agent_with()` in `event_loop.rs` and trace the call chain.
   - If `EnrichedCell` is not in the chain: write the wiring plan (change one call site in `event_loop.rs`).
   - If it is in the chain: write "confirmed live, evidence: <code path>".

3. **roko-gateway audit**:
   - Read the runner dispatch path for any reference to gateway types.
   - If absent: update CLAUDE.md to say "roko-gateway is HTTP-serve path only; runner-v2 dispatches directly through provider adapters."
   - If present: document the exact code path.

4. **Agent Groups coordination modes audit**:
   - For each mode enum variant, find the `match` arm and check if the arm body is a `todo!()` or a stub.
   - For each stub: create a sub-issue in GAPS.md.
   - For implemented modes: document with a test name that covers them.

5. **Output**: Produce `tmp/backlog/150-audit-results.md` with the verdict for each subsystem and the recommended action. New wiring tasks from this audit should become separate backlog items (151+).

6. **Update CLAUDE.md and GAPS.md**: Revise the "Wired" / "Complete" claims to accurately reflect the audit results.

## Acceptance Criteria

1. Each of the four subsystems has a written verdict: WIRE, CONFIRM-LIVE, or DOCUMENT.
2. CLAUDE.md and GAPS.md are updated to accurately reflect wiring status.
3. Any WIRE verdict generates a new backlog item (151+) with a concrete implementation plan.
4. Any CONFIRM-LIVE verdict includes a code path citation and a test that exercises it.
5. `tmp/backlog/150-audit-results.md` exists with verdicts for all four subsystems.

## Verification Checklist

- [ ] `grep -rn 'CorticalState::new\|CorticalState {' crates/ --include='*.rs' | grep -v target/` — confirm zero or non-zero call sites; update verdict accordingly.
- [ ] Trace `dispatch_agent_with()` in `event_loop.rs`; confirm whether `EnrichedCell` is in the call chain.
- [ ] Check runner dispatch for `roko_gateway::` imports; confirm gateway presence or absence.
- [ ] For each coordination mode, find the match arm body; confirm stub vs implementation.
- [ ] `150-audit-results.md` exists with all four verdicts documented.

## Files to Modify

| File | Change |
|---|---|
| `.roko/GAPS.md` | Update with accurate wiring status for each subsystem |
| `CLAUDE.md` | Revise "Wired"/"Complete" claims that the audit shows are overstated |
| `tmp/backlog/150-audit-results.md` | New file: audit verdicts for all four subsystems |
| `tmp/backlog/151+` | New backlog items for any WIRE verdicts |
