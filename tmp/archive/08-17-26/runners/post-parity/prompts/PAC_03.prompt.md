# PAC_03: Add post-execution contract verification

## Task
Add post-execution contract checks to the tool dispatcher to verify that completed tool calls satisfy contract invariants.

## Runner Context
Runner PAC (Safety Completeness), batch 3 of 4. No dependencies.

## Problem
ISS-8 safety gap: Contract enforcement (`AgentContract`) is pre-dispatch only via `GovernanceRule::check()`. There is no post-execution verification. A tool call that passes pre-checks but produces unexpected output (excessive file writes, unexpected network calls) goes undetected.

## Current Code

**Pre-dispatch checks** — `crates/roko-agent/src/safety/contract.rs:400-467`:
```rust
impl GovernanceRule {
    pub fn check(&self, ...) -> GovernanceCheckResult { ... }
}
```

**Post-execution** — ZERO matches for `post_check`, `post_execution`, `PostCheck` in roko-agent.

**RecoveryAction** — `contract.rs:225-234`:
Per-tool-call only: `applicable_recovery(&self, result: &ToolResult)`. Called from `safety/mod.rs:605`.

## Exact Changes

### Step 1: Add post-execution check method to AgentContract

```rust
// In contract.rs:
impl AgentContract {
    /// Post-execution verification of tool call results against contract invariants.
    pub fn post_check(&self, tool_name: &str, result: &ToolResult) -> PostCheckResult {
        let mut violations = Vec::new();

        // Check tool call count invariant
        for invariant in &self.invariants {
            if let InvariantKind::MaxToolCallsPerTurn(max) = &invariant.kind {
                // This is checked elsewhere via budget; skip
                continue;
            }
            if let InvariantKind::MaxOutputLength(max) = &invariant.kind {
                if result.output.len() > *max {
                    violations.push(PostCheckViolation {
                        invariant: invariant.name.clone(),
                        tool: tool_name.to_string(),
                        detail: format!("output length {} exceeds max {}", result.output.len(), max),
                    });
                }
            }
        }

        // Check governance rules post-hoc
        for rule in &self.governance_rules {
            if let GovernanceRule::NoSideEffects = rule {
                if result.has_side_effects {
                    violations.push(PostCheckViolation {
                        invariant: "no_side_effects".to_string(),
                        tool: tool_name.to_string(),
                        detail: "tool produced side effects in no-side-effect context".to_string(),
                    });
                }
            }
        }

        if violations.is_empty() {
            PostCheckResult::Pass
        } else {
            PostCheckResult::Violations(violations)
        }
    }
}

pub enum PostCheckResult {
    Pass,
    Violations(Vec<PostCheckViolation>),
}

pub struct PostCheckViolation {
    pub invariant: String,
    pub tool: String,
    pub detail: String,
}
```

### Step 2: Call post-check after tool execution in dispatcher

In `safety/mod.rs`, after tool execution completes:

```rust
// After tool result is returned:
match self.contract.post_check(&tool_name, &result) {
    PostCheckResult::Pass => {}
    PostCheckResult::Violations(violations) => {
        for v in &violations {
            warn!(
                tool = %v.tool,
                invariant = %v.invariant,
                detail = %v.detail,
                "post-execution contract violation"
            );
        }
        // Log but don't block (first iteration — make observable, then enforce)
        // Future: match self.contract.post_check_mode { Warn => log, Enforce => block }
    }
}
```

### Step 3: Add cumulative turn metrics tracking

Track across the turn for invariants that need cumulative state:

```rust
struct TurnMetrics {
    tool_call_count: u32,
    total_output_bytes: usize,
    files_modified: Vec<String>,
}
```

## Write Scope
- `crates/roko-agent/src/safety/contract.rs` (add post_check, PostCheckResult, PostCheckViolation)
- `crates/roko-agent/src/safety/mod.rs` (call post_check after tool execution)

## Read-Only Context
- `crates/roko-agent/src/safety/contract.rs` (GovernanceRule, InvariantKind existing types)


## Verify
```bash
cargo build -p roko-agent 2>&1 | head -30
cargo test -p roko-agent 2>&1 | tail -20
```
## Acceptance Criteria
- `post_check()` runs after every tool call
- Violations logged as warnings (observe-first mode, not enforce-first)
- No behavioral change to existing tool dispatch (warnings only)
- PostCheckResult type available for future enforcement mode

## Do NOT
- Block tool calls on post-check violations in this iteration (warn-only)
- Change existing pre-dispatch check behavior
- Add post-checks to the recovery action path (those are separate)
