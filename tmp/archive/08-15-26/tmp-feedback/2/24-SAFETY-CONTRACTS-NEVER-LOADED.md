# Safety Contracts Never Loaded — Permissive Default Always Used

## Problem

The `AgentContract` system defines per-role safety constraints (allowed tools, forbidden
patterns, resource limits). But the orchestrator never loads contract YAML files, so every
agent runs with the permissive default contract. Additionally, Claude CLI bash denylist
can't be enforced because the safety layer is outside the opaque subprocess.

## Root Cause

### A. Contract YAML never loaded

**File:** `crates/roko-agent/src/safety/contract.rs`

```rust
pub struct AgentContract {
    pub role: String,
    pub allowed_tools: Vec<String>,
    pub forbidden_patterns: Vec<String>,
    pub max_turns: u32,
    pub max_file_writes: u32,
    // ...
}

impl AgentContract {
    pub fn load(path: &Path) -> Result<Self> { /* reads YAML */ }
    pub fn permissive() -> Self { /* allows everything */ }
}
```

**File:** `crates/roko-cli/src/orchestrate.rs`

```rust
let contract = AgentContract::permissive();  // ← always permissive
// Never calls:
// let contract = AgentContract::load(&contract_path)?;
// or:
// let contract = AgentContract::for_role(&task.role)?;
```

The orchestrator creates contracts but always uses `permissive()`. The `.with_role()` method
that would look up a role-specific contract is never called.

### B. No contract YAML files exist

**Directory:** `.roko/contracts/` — does not exist

Even if the code loaded contracts, there are no YAML files defining role-specific constraints.
The system needs both:
1. Default contracts for builtin roles (researcher, implementer, reviewer, planner)
2. A way to override via `.roko/contracts/<role>.yaml`

### C. Claude CLI bash safety can't be enforced

When using the Claude CLI backend, agent tool calls happen inside an opaque subprocess:
```
roko → spawn claude CLI → claude calls bash → bash executes command
```

The safety layer in roko-agent sits outside the subprocess. It can restrict which tools
roko passes to Claude CLI via `--allowed-tools`, but once Claude CLI has `Bash`, roko
can't inspect or block individual bash commands. The bash denylist in
`crates/roko-agent/src/safety/bash_filter.rs` is never consulted.

## Fix

### Fix 1: Load contracts from role (~15 min)

**File:** `crates/roko-cli/src/orchestrate.rs`

```rust
let contract = if let Some(contract_path) = find_contract(&task.role, workdir) {
    AgentContract::load(&contract_path)?
} else {
    AgentContract::for_builtin_role(&task.role)
};
```

### Fix 2: Create default contracts for builtin roles (~10 min)

**File:** `crates/roko-agent/src/safety/contract.rs`

```rust
pub fn for_builtin_role(role: &str) -> Self {
    match role {
        "researcher" => AgentContract {
            allowed_tools: vec!["read_file", "glob", "grep", "web_search", "web_fetch"],
            forbidden_patterns: vec!["rm -rf", "git push", "curl.*POST"],
            max_turns: 20,
            max_file_writes: 5,
            ..
        },
        "implementer" => AgentContract {
            allowed_tools: vec!["read_file", "write_file", "edit_file", "bash", "glob", "grep"],
            forbidden_patterns: vec!["rm -rf", "git push --force"],
            max_turns: 50,
            max_file_writes: 20,
            ..
        },
        "reviewer" => AgentContract {
            allowed_tools: vec!["read_file", "glob", "grep"],
            forbidden_patterns: vec![],
            max_turns: 10,
            max_file_writes: 1,  // only the review report
            ..
        },
        _ => AgentContract::permissive(),
    }
}
```

### Fix 3: Wire bash filter for non-CLI backends (~15 min)

For the ExecAgent and OpenAI-compat backends (where tool calls go through roko's tool loop),
wire the bash filter:

**File:** `crates/roko-agent/src/tool_loop.rs`

```rust
if tool_call.name == "bash" {
    if let Some(filter) = &self.bash_filter {
        if filter.is_denied(&tool_call.arguments["command"]) {
            return Err(ToolError::Denied(format!(
                "Command blocked by safety filter: {}",
                tool_call.arguments["command"]
            )));
        }
    }
}
```

## Files to Modify

| File | Change |
|------|--------|
| `crates/roko-cli/src/orchestrate.rs` | Load contracts instead of permissive default |
| `crates/roko-agent/src/safety/contract.rs` | Add `for_builtin_role()` |
| `crates/roko-agent/src/tool_loop.rs` | Wire bash filter for non-CLI backends |

## Priority

**P1** — The safety system is the last line of defense against agent misuse. Having it always
permissive means every agent can do anything. The builtin role contracts are a 10-minute fix
that provides meaningful default safety.
