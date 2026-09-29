# Permits and Allowlists

> **v3 depth file** -- `/docs/v3/depth/12-safety/permits-allowlists.md`
> Canonical source: v1 `docs/v1/11-safety/04-permits-allowlists.md`
> Status: **Shipping**. Role-based permissions, task-level tool filters, and
> `AgentContract` tool policy are live. Unknown roles deny all tools.

---

## 1. Three-Level Permission Model

The Roko tool permission system operates at three levels that compose conjunctively:

1. **Role-based permissions** -- each agent role has capability flags.
2. **Tool-level requirements** -- each tool declares what permissions it needs.
3. **Task-level filters** -- individual tasks restrict which tools are available.

A tool call succeeds only if the role grants the required permissions AND the tool is
not on the task's deny list AND (if an allow list exists) the tool is on the allow list.

---

## 2. ToolPermissionPolicy

The `ToolPermission` struct declares what a tool requires:

```rust
pub struct ToolPermission {
    pub read: bool,
    pub write: bool,
    pub exec: bool,
    pub git: bool,
    pub network: bool,
}
```

Convenience constructors:

```rust
impl ToolPermission {
    pub fn read_only() -> Self {
        Self { read: true, write: false, exec: false, git: false, network: false }
    }
    pub fn writes() -> Self {
        Self { read: true, write: true, exec: false, git: false, network: false }
    }
    pub fn full() -> Self {
        Self { read: true, write: true, exec: true, git: true, network: true }
    }
}
```

### Satisfaction check

```rust
impl ToolPermission {
    pub fn satisfied_by(&self, granted: &ToolPermissions) -> bool {
        (!self.read || granted.read)
            && (!self.write || granted.write)
            && (!self.exec || granted.exec)
            && (!self.git || granted.git)
            && (!self.network || granted.network)
    }
}
```

When permission check fails, the dispatcher emits an audit Signal with phase
`permission` and status `denied`.

---

## 3. Role Permission Matrix

| Role | read | write | exec | git | network | Rationale |
|------|------|-------|------|-----|---------|-----------|
| Implementer | yes | yes | yes | yes | no | Write code, run builds, commit |
| Auditor | yes | no | yes | no | no | Review code, run analysis |
| Researcher | yes | no | no | no | yes | Read code, query external |
| Planner | yes | no | no | no | no | Read plans and state only |
| Reviewer | yes | no | yes | no | no | Read code, run verification |

The default policy is deny-by-default. Unknown roles deny all tools.

---

## 4. Task-Level Tool Filters

### Allowed tools

When `allowed_tools` is set on the `ToolContext`, only tools in this list can execute.
All others return `ToolError::PermissionDenied`.

Use cases:
- Read-only audit: restrict Implementer to `read_file`, `glob`, `grep`.
- Git-only: limit to `git_status`, `git_diff`, `git_log`.
- Test-only: allow only `bash` and `read_file`.

### Denied tools

When `denied_tools` is set, tools in this list are blocked regardless of role permissions.

Use cases:
- Block `bash` during review.
- Block `write_file` during analysis.
- Block network tools during offline tasks.

### Evaluation order

The deny list is evaluated before the allow list:

1. **Schema validation** -- check arguments against JSON schema.
2. **Tool existence** -- verify the tool exists in the registry.
3. **Deny list** -- if in `denied_tools`, reject.
4. **Allow list** -- if `allowed_tools` is non-empty and tool not in it, reject.
5. **Permission check** -- verify `satisfied_by()`.
6. **Safety layer** -- run `SafetyLayer::check_pre_execution()`.

---

## 5. AgentContract Tool Policy

Beyond role and task filters, `AgentContract` provides a fourth policy layer:

```rust
pub struct AgentContract {
    pub role: AgentRole,
    pub allowed_tools: Option<Vec<String>>,
    pub denied_tools: Option<Vec<String>>,
    pub governance_rules: Vec<GovernanceRule>,
}
```

The contract intersects with role and task filters:

- Role allowlists intersect with contract allowlists.
- Denials from any source win.
- Unknown roles deny all.
- Unsupported policy-bearing dispatches are rejected.

---

## 6. Tool Registry and Definitions

Every tool is registered via a `ToolDef`:

```rust
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub category: ToolCategory,
    pub permission: ToolPermission,
    pub concurrency: ToolConcurrency,
    pub schema: Option<serde_json::Value>,
}
```

### Tool categories

```rust
pub enum ToolCategory {
    Read,       // read_file, glob, grep
    Write,      // write_file, edit_file
    Shell,      // bash
    Git,        // git operations
    Network,    // web_fetch, web_search
    Meta,       // list_tools, show_plan
    Custom,     // user-defined tools
}
```

### Concurrency policy

```rust
pub enum ToolConcurrency {
    Parallel,  // Safe to run concurrently (read_file, glob)
    Serial,    // Must run sequentially (bash, write_file)
}
```

`ToolDispatcher::dispatch_batch()` groups calls by concurrency: `Parallel` tools
run via `join_all`, `Serial` tools run sequentially.

---

## 7. Configuration

Tool permissions and filters are configurable in `roko.toml`:

```toml
[agent]
default_role = "Implementer"
mcp_config = ".roko/mcp-config.json"

[safety]
bash_deny_patterns = ["npm run deploy", "cargo publish"]
network_allow_hosts = [".github.com", ".crates.io"]
rate_limit_calls = 120
rate_limit_window_secs = 60
```

---

## 8. Integration with Plan Execution

The runner assigns roles and permissions when dispatching agents:

1. Each task in a plan has an assigned role.
2. The role determines `ToolPermissions`.
3. The task may specify additional tool filters.
4. `RoleSystemPromptSpec` generates a system prompt including available tools.

---

## 9. Fail-Closed Behavior

The permission system fails closed at every decision point:

- Unknown tool names: denied.
- Missing permission flags: the unflagged capability is treated as denied.
- Empty allow list with non-empty deny list: deny list applies, all others allowed.
- Both lists empty: all tools available within role permissions.
- Role not recognized: all tools denied.

---

## Academic References

| Paper | Contribution |
|---|---|
| Dennis & Van Horn (1966) | Capability-based security |
| Saltzer & Schroeder (1975) | Principle of least privilege |
| Anderson (2008) | Security Engineering -- access control |

---

## Implementation References

| Component | Location |
|---|---|
| ToolPermission | `crates/roko-core/src/tool.rs` |
| ToolDef | `crates/roko-core/src/tool.rs` |
| ToolDispatcher | `crates/roko-agent/src/dispatcher/mod.rs` |
| SafetyLayer | `crates/roko-agent/src/safety/mod.rs` |
| AgentContract | `crates/roko-core/src/agent_contract.rs` |
