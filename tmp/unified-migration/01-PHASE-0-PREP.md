# Phase 0 — Pre-Migration Prep & Cleanup

> Wire dead code, fix known gaps, and prepare the codebase for the kernel rename. Nothing in this phase changes public API — it just ensures the starting point is clean.

**Spec source**: `tmp/unified/21-ROADMAP.md` §1 (Phase 0 — Current State)
**Audit source**: `tmp/roko-trustworthy/AUDIT.md`

---

## 0.1 Wire Dead Code (Category A from AUDIT)

Items that are built but never called. Wire them before migrating, or delete them if the unified spec supersedes them.

- [ ] **Wire ExtensionChain into orchestrate.rs** — `crates/roko-agent/src/extensions/` (539 LOC) is built but never invoked from the dispatch path in `crates/roko-cli/src/orchestrate.rs`. Wire pre/post-inference hooks into `dispatch_agent_with()`. **Verify**: add a no-op extension, confirm hooks fire during `roko plan run`.
  - Source: `tmp/roko-trustworthy/AUDIT.md` §A1
  - Code: `crates/roko-cli/src/orchestrate.rs` (dispatch_agent_with function)

- [ ] **Wire KnowledgeAdmissionController** — `crates/roko-neuro/src/admission.rs` (1,285 LOC) gates knowledge writes but is never called. Wire it into the knowledge store's `put()` path. **Verify**: post a low-quality knowledge entry, confirm it's rejected.
  - Source: `tmp/roko-trustworthy/AUDIT.md` §A2
  - Code: `crates/roko-neuro/src/store.rs`

- [ ] **Wire ContextualBanditPolicy for routing** — `crates/roko-learn/src/bandits/` (1,372 LOC) is built but never called for model routing decisions. Wire into CascadeRouter's selection path. **Verify**: confirm bandit feedback updates after agent dispatch.
  - Source: `tmp/roko-trustworthy/AUDIT.md` §A4
  - Code: `crates/roko-learn/src/routing/cascade.rs`

- [ ] **Audit ConnectorRegistry + FeedRegistry** — `crates/roko-runtime/src/` (493 LOC) has empty registries. Either wire them or delete if unified Connect protocol supersedes. **Verify**: `cargo clippy` clean, no unused imports.
  - Source: `tmp/roko-trustworthy/AUDIT.md` §A3
  - Code: `crates/roko-runtime/src/`

## 0.2 Fix Implementation Gaps (Category B from AUDIT)

- [ ] **Fix token accounting in gateway.rs** — Token tracking uses `len/4` heuristic instead of real token counts. Accumulate actual token counts from LLM responses. **Verify**: gateway route returns accurate token counts matching LLM response headers.
  - Source: `tmp/roko-trustworthy/AUDIT.md` §B1
  - Code: `crates/roko-serve/src/routes/gateway.rs:269-270`

- [ ] **Parallelize batch requests in gateway.rs** — Batch submissions are sequential. Use `JoinSet` for parallelism. **Verify**: batch of 5 requests completes faster than 5x single-request time.
  - Source: `tmp/roko-trustworthy/AUDIT.md` §B2
  - Code: `crates/roko-serve/src/routes/gateway.rs:437-487`

- [ ] **Fix routing context in gateway.rs** — Routing context is hardcoded. Derive from request metadata (model hint, domain, task type). **Verify**: different request metadata produces different routing decisions.
  - Source: `tmp/roko-trustworthy/AUDIT.md` §B4
  - Code: `crates/roko-serve/src/routes/gateway.rs:615-632`

## 0.3 Rename Preparation

These create the file structure for the new names without changing any logic yet. The actual renames happen in Phase 1.

- [ ] **Create `crates/roko-core/src/signal.rs`** — Empty module that will hold the renamed `Engram` struct. Add `mod signal;` to lib.rs (feature-gated or empty for now). **Verify**: `cargo check -p roko-core`.
  - Target: `crates/roko-core/src/signal.rs`

- [ ] **Create `crates/roko-core/src/pulse.rs`** — Empty module for the new `Pulse` struct. **Verify**: `cargo check -p roko-core`.
  - Target: `crates/roko-core/src/pulse.rs`

- [ ] **Create `crates/roko-core/src/cell.rs`** — Empty module for the new `Cell` trait. **Verify**: `cargo check -p roko-core`.
  - Target: `crates/roko-core/src/cell.rs`

- [ ] **Create `crates/roko-core/src/bus.rs`** — Empty module for the `Bus` trait (currently `EventBus` lives in roko-runtime). **Verify**: `cargo check -p roko-core`.
  - Target: `crates/roko-core/src/bus.rs`

## 0.4 Baseline Verification

- [ ] **Full workspace builds and passes** — Run `cargo build --workspace && cargo test --workspace && cargo clippy --workspace --no-deps -- -D warnings`. Record baseline test count and pass rate. This is the regression baseline for all subsequent phases. **Verify**: all three commands pass clean.
  - Code: workspace root
