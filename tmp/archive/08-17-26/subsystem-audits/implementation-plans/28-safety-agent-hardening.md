# 28 — Safety Agent Hardening

The safety contract framework is architecturally sound. T1-15 made the
default fail-closed (`restricted` instead of `permissive`). Remaining
work:

1. Recovery actions defined but never invoked end-to-end.
2. Safety contract enforcement gap on `roko-acp` direct dispatch
   (R3_F03 partly addressed; verify).
3. Permission request bridge (`session/request_permission`) needs
   graduated trust (R3_F04 partly).
4. Tool dispatch 10-stage pipeline needs auditing for "fail open at
   stage N" bugs.

Source: subsystem-audits/safety-agent/AUDIT.md, subsystem-audits/safety-agent/PLAN.md,
doc 36 § safety fail-open defaults, doc 41 priority 9.

---

## Today's State (verified 2026-05-01)

- `SafetyLayer::with_defaults()` uses `AgentContract::restricted("default")` (T1-15).
- `contract_for_role()` returns `restricted` on missing/failed YAML, with `tracing::warn!`.
- `AgentContract::permissive(...)` is `#[cfg(test)]`-only in usage.
- Recovery actions are defined in YAML; the dispatch path doesn't invoke them.
- `R3_F03` wired safety contract enforcement into ACP dispatch; verify scope.
- `R3_F04` added permission request bridge with graduated trust.

---

## Anti-Patterns

1. **No `permissive()` outside test code.**
2. **No "skip safety check if dispatch fast-path."**
3. **No silent acceptance of "tool not in allowlist."** Either deny or
   request permission.
4. **No global mutable safety state.** Per-session + per-role.
5. **No bypass via env var without typed override.**

---

## Plan

### [ ] S-1: Audit all `permissive()` call sites

```bash
rg 'permissive\(' crates/ -g '*.rs'
```

For each match outside test code, replace with `restricted` or document
why permissive is required (e.g. internal benchmark fixture). Mark all
test-only sites with `#[cfg(test)]` if not already.

**Estimated effort**: 1-2 hours.

### [ ] S-2: Wire recovery actions into the dispatch failure path

**File**: `crates/roko-agent/src/safety/mod.rs`, dispatch failure handlers.

**Why**: When a tool call is denied or fails, the YAML defines a
recovery action (e.g. "ask user," "fall back to read-only"). Today the
recovery action is parsed but never invoked.

#### Implementation

```rust
impl SafetyLayer {
    pub async fn handle_violation(&self, violation: &SafetyViolation) -> RecoveryOutcome {
        let action = self.contract.recovery_for(&violation.tool);
        match action {
            RecoveryAction::AskUser => RecoveryOutcome::RequestPermission(violation.clone()),
            RecoveryAction::FallbackReadOnly => RecoveryOutcome::RetryWith(SafetyOverlay::read_only()),
            RecoveryAction::Abort => RecoveryOutcome::Abort,
            RecoveryAction::Log => {
                tracing::warn!(?violation, "safety violation logged but not blocked");
                RecoveryOutcome::Continue
            }
        }
    }
}
```

Call from the dispatch error path in `roko-agent` and `roko-acp`.

### [ ] S-3: Add per-session safety overlay

**Why**: Today the contract is per-role. A specific session may need to
narrow further (e.g. "this session may only read files in /tmp"). Add a
per-session overlay that intersects with the contract.

```rust
pub struct SafetyOverlay {
    pub additional_denies: Vec<ToolPolicy>,
    // ...
}

let effective_contract = base_contract.intersect(&session_overlay);
```

Sessions inherit the overlay from the agent / role.

### [ ] S-4: Verify ACP dispatch enforces the safety contract

**File**: `crates/roko-acp/src/session.rs`, `crates/roko-acp/src/bridge_events.rs`.

```bash
rg 'AgentContract|SafetyLayer|safety_layer' crates/roko-acp/
```

For each tool dispatch in ACP, confirm the contract check happens before
the dispatch. Add a regression test:

```rust
#[tokio::test]
async fn acp_denies_blacklisted_tool() {
    let contract = AgentContract::restricted("test").deny_tool("bash");
    let session = AcpSession::test_with_contract(contract).await;
    let resp = session.dispatch_tool("bash", json!({"cmd": "ls"})).await;
    assert!(matches!(resp, Err(SafetyError::ToolDenied { tool, .. }) if tool == "bash"));
}
```

### [ ] S-5: Permission request bridge graduated trust

**Reference**: R3_F04 already shipped a permission request bridge. Verify:

- First request → ask user, default deny.
- Second request for same `(tool, resource)` within session → reuse decision.
- Cross-session: trust is per-session unless explicitly elevated to
  workspace level.

Add tests for graduation behavior.

### [ ] S-6: Audit 10-stage tool dispatch pipeline

The audit identified a 10-stage pipeline; some stages may "fail open"
(allow on internal error). Review each stage:

1. Bind input
2. Resolve tool
3. Permission check (must fail closed)
4. Rate limit check
5. ... etc.

For each stage that hit an error, the response is **deny + log**, not
**allow + log**.

**Estimated effort**: 4-6 hours per stage.

---

## Combined Verification

```bash
cargo test -p roko-agent safety --lib
cargo test -p roko-acp safety_enforcement --lib
rg 'permissive\(' crates/ -g '*.rs'   # only #[cfg(test)] usage
rg 'RecoveryAction' crates/roko-agent/   # invoked from dispatch path
```

---

## Status

- [ ] S-1 — Audit `permissive()` call sites
- [ ] S-2 — Wire recovery actions
- [ ] S-3 — Per-session safety overlay
- [ ] S-4 — Verify ACP enforces contract
- [ ] S-5 — Graduated trust verification
- [ ] S-6 — 10-stage pipeline fail-closed audit

**Estimated effort**: 16-30 hours.
