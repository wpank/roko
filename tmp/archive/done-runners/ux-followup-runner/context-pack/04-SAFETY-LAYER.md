# Safety Layer — SafetyLayer / AgentContract / role context

Required reading for UX26 (safety contract enforcement) and UX27 (role-based
tool whitelist). Relevant background for UX14 (process supervision) and
UX36 (roko.toml keys).

## Overview

The safety layer sits between the dispatcher and tool execution. It receives
the tool invocation + the calling role, runs pre-execution checks, executes
the tool, and runs post-execution scrubbing. Today the wiring is partial:

- `SafetyLayer::check_pre_execution` receives the role but does not enforce
  role → tool ACLs (UX27 fixes).
- `AgentContract` is declared in `safety/contract.rs` but never constructed
  outside its own tests (UX26 fixes).
- Rate limits, path allow-lists, and network allow-lists are enforced.
- Git safety + bash safety + scrub paths are wired.

## Module layout

`crates/roko-agent/src/safety/`

| File | LOC | Role |
|------|-----|------|
| `mod.rs` | 15 768 | `SafetyLayer` struct + `check_pre_execution` / `check_post_execution` |
| `contract.rs` | 5 978 | **Unenforced**: `AgentContract` + declarative invariants |
| `capabilities.rs` | 5 592 | Capability flags |
| `bash.rs` | 14 039 | Bash command allow-list + injection hardening |
| `git.rs` | 27 285 | Git command allow-list (tallest file — broad coverage) |
| `network.rs` | 16 703 | Domain / port allow-list |
| `path.rs` | 19 446 | Path escape / allow-list |
| `rate_limit.rs` | 18 291 | Per-role / per-tool rate limits |
| `scrub.rs` | 19 672 | Output scrubbing (secrets, PII) |

## AgentContract (UX26 target)

```rust
pub struct AgentContract {
    pub role: String,
    pub invariants: Vec<Invariant>,
    pub governance: GovernanceRules,
    pub recovery: RecoveryRules,
}

pub enum Invariant {
    MaxTokensPerTurn(u32),
    RequireGateBeforeCommit,
    NoNetworkAccess,
    // …
}
```

Today: constructed only in `#[cfg(test)]` blocks.
Target (UX26): constructed at `Dispatcher::new` from the agent's `roko.toml`
block, stored on `SafetyLayer`, checked via a new
`SafetyLayer::check_contract(invocation) -> Result<(), ContractViolation>`.

## Role → tool whitelist (UX27 target)

Add to `roko.toml`:

```toml
[agent.implementer]
model = "claude-opus-4-6"
tools = ["read", "edit", "bash", "git-*"]

[agent.reviewer]
model = "claude-sonnet-4-6"
tools = ["read"]  # read-only
```

`SafetyLayer::check_pre_execution` should:
1. Look up the calling role in its role→tools map (built at boot from roko.toml).
2. Match the tool name against the whitelist (glob match supported).
3. If not allowed: return `Err(SafetyViolation::ToolNotInRoleWhitelist { role, tool })`.

## Role propagation (how the role arrives)

Role flows from:
- `roko.toml` `[agent.<role>]` block → `AgentConfig`
- → `ToolDispatcher::new(config, role)` via `roko-cli/src/orchestrate.rs`
- → `SafetyLayer::new(role_config)` inside `ToolDispatcher`
- → `SafetyLayer::check_pre_execution(invocation, role)` per tool call

## Current wiring seams

These already call `SafetyLayer`:

- `ToolDispatcher::dispatch` — primary path
- `ToolDispatcher::preflight` — pre-prompt capability check
- Bash + git + network helpers inside `roko-std` tools

These do **not** yet call role-based ACL checks:
- `SafetyLayer::check_pre_execution` body (UX27 extends)
- Contract governance / recovery loops (UX26 extends)

## Interaction with ProcessSupervisor (UX14)

`ProcessSupervisor` in `roko-runtime/src/process.rs` spawns child processes
on behalf of tools. It does not consult `SafetyLayer` — pre-execution checks
must happen in the dispatcher before the supervisor is called.

UX14 adds SIGTERM-then-SIGKILL escalation + `Drop` for ProcessSupervisor to
guarantee children are reaped even on parent panic. It does **not** change
the dispatcher → safety → supervisor ordering.

## Expected pattern for UX26

```rust
impl ToolDispatcher {
    pub async fn dispatch(&self, req: ChatRequest) -> Result<ChatResponse> {
        self.safety.check_pre_execution(&req, self.role).await?;
        self.safety.check_contract(&req).await?;                     // NEW
        let resp = self.backend.send_turn(req).await?;
        self.safety.check_post_execution(&resp, self.role).await?;
        self.contract.recovery.apply_if_applicable(&resp).await?;   // NEW
        Ok(resp)
    }
}
```

## Expected pattern for UX27

```rust
impl SafetyLayer {
    pub async fn check_pre_execution(&self, inv: &Invocation, role: &str) -> Result<()> {
        let allowed = self.role_tool_whitelist
            .get(role)
            .map(|whitelist| matches_any(&inv.tool, whitelist))
            .unwrap_or(false);
        if !allowed {
            return Err(SafetyError::ToolNotInRoleWhitelist {
                role: role.to_string(),
                tool: inv.tool.clone(),
            });
        }
        // existing per-tool checks
    }
}
```

## Safety test pattern

Unit tests for safety sit in `crates/roko-agent/src/safety/` (per-file
`mod tests`). Integration tests for the dispatcher + safety end-to-end belong
in `crates/roko-agent/tests/` (new location; UX26 may need to create this
directory).

Test helper for UX26 + UX27:

```rust
#[tokio::test]
async fn reviewer_cannot_invoke_bash() {
    let config = AgentConfig::from_toml_str(r#"
        [agent.reviewer]
        model = "mock"
        tools = ["read"]
    "#).unwrap();

    let dispatcher = ToolDispatcher::new(config, "reviewer").unwrap();
    let err = dispatcher.dispatch(mock_bash_req()).await.unwrap_err();
    assert!(matches!(err, DispatchError::Safety(SafetyError::ToolNotInRoleWhitelist { .. })));
}
```

## Non-goals for UX26 / UX27

- Do **not** rewrite `bash.rs`, `git.rs`, `network.rs`, or `path.rs`. Those
  layers already work; UX26/UX27 sit in front of them.
- Do **not** touch `scrub.rs` — output scrubbing is a separate concern.
- Do **not** add network auth. That is `roko-serve` Phase-2 work (item 53 in
  file 08, parked).
