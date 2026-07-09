# 08 — Tech Debt Inventory

**TODOs, gaps, known issues, stubs, and structural debt.**

> Current correction, 2026-07-07: this file is retained as a first-pass debt list. The current P0 is not `orchestrate.rs` decomposition by itself; it is the misleading default Graph plan execution path and broken resume routing. Use `24-OPEN-ISSUE-LEDGER.md` and `12-ROADMAP.md` for current ordering.

---

## Critical Debt

### 1. orchestrate.rs Monolith (23K+ LOC)
- **What**: All execution logic in one file — plan loading, DAG scheduling, prompt building, agent dispatch, gate execution, learning hooks, state persistence, replanning
- **Impact**: Hard to test, hard to modify, hard to reason about
- **Fix**: Decompose into modules: `plan_loader.rs`, `task_scheduler.rs`, `prompt_enricher.rs`, `gate_runner.rs`, `state_manager.rs`, `replan.rs`
- **Effort**: High (refactor, not rewrite — preserve behavior)

### 2. V1/V2 Paradigm Split
- **What**: Codebase runs V1 architecture while V2 is specified in 184+ documents. `roko-graph` built for V2 but unwired.
- **Impact**: Confusion about what the "real" architecture is; wasted effort if V2 is never adopted
- **Fix**: Make a decision — V2 migration timeline or formalize V1 as the architecture
- **Effort**: Decision, not code

### 3. Two Execution Engines
- **What**: `PlanRunner` (orchestrate.rs, 23K LOC) and `WorkflowEngine` (roko-runtime, 2K LOC)
- **Impact**: Maintenance overhead, confusion about which to use
- **Fix**: Remove `WorkflowEngine` or extract `PlanRunner` logic into `roko-orchestrator` and merge
- **Effort**: Medium

### 4. Circular Dependencies (5 cycles)
- **What**: `roko-agent` ↔ `roko-learn` ↔ `roko-compose` ↔ `roko-neuro` form cycles
- **Impact**: Layer violations, harder to reason about build order and separation of concerns
- **Fix**: Extract shared traits into `roko-core` or a new `roko-traits` crate; invert dependencies via trait objects
- **Effort**: Medium-High

### 5. roko-runtime Layer Violation
- **What**: Layer 1-2 crate depends on layer 3 crates (`roko-learn`, `roko-compose`, `roko-gate`)
- **Impact**: Lower layers know about higher layers — inverts the architecture
- **Fix**: Move the offending code up to `roko-orchestrator` or use trait bounds that live in core
- **Effort**: Medium

---

## Moderate Debt

### 6. Safety Contract Completion
- **What**: `AgentContract` enforcement is fail-closed for bundled roles, but advanced warrant/taint/budget/witness hooks are not proven end-to-end
- **Impact**: The default stance is safer than older docs describe, but safety semantics are still fragmented across contract defaults, operator role choices, and runtime dispatch paths
- **Fix**: Keep missing-contract behavior restrictive, document role exceptions, and add integration proof for advanced safety hooks
- **Effort**: Low

### 7. No Unified Persistence Layer
- **What**: Each subsystem serializes differently — JSONL, JSON, TOML, custom formats
- **Impact**: No transactional consistency, harder to backup/restore, scattered state files
- **Fix**: Either accept the current approach (it works) or introduce a unified state store
- **Effort**: High if changing; None if accepting status quo

### 8. Stale tmp/ Documentation
- **What**: 60+ directories in `tmp/` with designs, audits, plans — many reference outdated state
- **Impact**: Misleading for anyone reading project docs; hard to know what's current
- **Fix**: This status-quo audit partially addresses it. Consider archiving stale docs.
- **Effort**: Medium (organizational, not code)

### 9. HTTP Route Count (278)
- **What**: `roko-serve` has ~278 routes, many in separate handler files
- **Impact**: Unclear which routes are actively used vs stubs; maintenance overhead
- **Fix**: Audit route usage, consolidate or remove unused routes
- **Effort**: Medium

### 10. Demo App Drift
- **What**: `demo/demo-app/` React frontend may not reflect current API surface
- **Impact**: Demo doesn't showcase current capabilities
- **Fix**: Update demo to use current routes, or document as example-only
- **Effort**: Low-Medium

---

## Low Debt / Cleanup Items

### 11. Phase 2 Dead Code
- **What**: `roko-dreams/phase2/`, `roko-daimon/phase2_stubs.rs`, `roko-chain/phase2/` compile but do nothing
- **Impact**: Code that will never be called until Phase 2 starts
- **Fix**: Feature-gate behind `#[cfg(feature = "phase2")]`
- **Effort**: Low

### 12. Unused / Minimal Crates
- **What**: Some crates have minimal functionality or overlap with others
- **Impact**: Workspace bloat
- **Fix**: Audit and merge where appropriate
- **Effort**: Low-Medium

### 13. Test Coverage Imbalance
- **What**: Current all-workspace static census found 9,968 test attribute hits, but coverage is uneven and several integration-heavy surfaces are still under-proven.
- **Impact**: Key components may have regressions that aren't caught
- **Fix**: Add integration tests for undertested crates
- **Effort**: Medium

### 14. TODOs in Code
- **What**: Various `TODO` and `FIXME` comments scattered across crates
- **Typical patterns**: `// TODO: implement proper error handling`, `// FIXME: this is a temporary workaround`
- **Fix**: Triage and create issues, or fix inline
- **Effort**: Varies

### 15. loop_tick() Dead Code
- **What**: The V1 spec's universal loop function exists but is never called
- **Impact**: Misleading — appears to be a core function but isn't part of the runtime
- **Fix**: Either wire into the runtime or document as "reference implementation"
- **Effort**: Low (decision) or High (wiring)

---

## Tech Debt by Crate

| Crate | Debt Items | Severity |
|-------|-----------|----------|
| roko-cli | orchestrate.rs monolith (23K LOC) | Critical |
| roko-runtime | Layer violations, unused WorkflowEngine | Medium |
| roko-graph | Built and reachable, but plan execution dry-runs and parity is incomplete | Medium |
| roko-core | loop_tick() dead code, alias indirection | Low |
| roko-agent | Circular deps with learn/compose | Medium |
| roko-compose | VCG built but unused | Low |
| roko-serve | 278 routes, unclear usage | Medium |
| roko-dreams | Runtime triggers exist, but v2 cron/delta/BusPulse scheduling and advice consumption are incomplete | Low |
| roko-daimon | Phase 2 stubs | Low |
| roko-chain | Phase 2 stubs | Low |
| roko-agent-server | Undertested (9 tests) | Low |

---

## Debt Prioritization Matrix

| Priority | Item | Effort | Impact |
|----------|------|--------|--------|
| P0 | Decompose orchestrate.rs | High | High — unblocks all other work |
| P0 | Decide V1/V2 direction | Decision | High — determines roadmap |
| P1 | Fix circular dependencies | Medium | Medium — improves architecture |
| P1 | Fix runtime layer violations | Medium | Medium — corrects hierarchy |
| P1 | Remove WorkflowEngine | Low | Low — reduces confusion |
| P2 | Ship safety contract defaults | Low | Medium — improves security posture |
| P2 | Wire dream auto-trigger | Low | Low — improves knowledge consolidation |
| P2 | Feature-gate Phase 2 code | Low | Low — reduces noise |
| P3 | Unify persistence | High | Low — current approach works |
| P3 | Route audit for roko-serve | Medium | Low — doesn't affect functionality |
