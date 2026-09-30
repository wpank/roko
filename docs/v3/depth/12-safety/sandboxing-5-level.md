# Five-Level Sandboxing

> **v3 depth file** -- `/docs/v3/depth/12-safety/sandboxing-5-level.md`
> Canonical source: v1 `docs/v1/11-safety/06-sandboxing.md`
> Status: **Complete** (E34 8/8). Five `SandboxLevel` variants with `PluginTier`
> mapping, `SandboxPolicy` enforcement, and path-level sandbox checks are live.

---

## 1. Sandboxing vs. Authorization

Sandboxing answers a different question than authorization:

- Authorization decides whether the principal may attempt an action.
- Sandboxing decides what the runtime prevents even if code is buggy, malicious, or
  compromised.

Both are required. A permission grant without a sandbox is trust. A sandbox without
permission checks is a crude jail.

---

## 2. The Five Sandbox Levels

Defined in `roko-agent/src/safety/sandbox.rs`:

```rust
pub enum SandboxLevel {
    /// No sandbox. Reserved for trusted in-tree execution.
    None,
    /// Audit policy decisions without blocking.
    Observe,
    /// Enforce worktree paths, secret-path denials, and network policy.
    #[default]
    Restrict,
    /// Deny network and constrain filesystem access to the worktree.
    Isolate,
    /// Memory-only execution: no filesystem, network, subprocess, or git.
    Quarantine,
}
```

### Level detail matrix

| Level | Filesystem | Network | Subprocess | Git | Environment | Audit |
|---|---|---|---|---|---|---|
| None | Full | Full | Full | Full | Inherited | No |
| Observe | Full | Full | Full | Full | Inherited | Yes (non-blocking) |
| Restrict | Worktree only | Policy-controlled | Allowed | Allowed | Inherited | Yes (blocking) |
| Isolate | Worktree only | Denied | Allowed | Denied | Stripped | Yes (blocking) |
| Quarantine | Denied | Denied | Denied | Denied | Stripped | Yes (blocking) |

### Default level

`Restrict` is the default (`#[default]`). This enforces worktree paths, secret-path
denials, and network policy while allowing normal development operations.

---

## 3. ActPlane Alignment

The five-level sandbox model aligns with the ActPlane framework (ActPlane, 2025,
arXiv:2606.25189), which proposes layered execution planes for LLM agents:

| ActPlane concept | Roko sandbox level | Enforcement |
|---|---|---|
| Unrestricted plane | None | Trusted kernel code |
| Monitored plane | Observe | Audit without blocking |
| Restricted plane | Restrict | Path/network enforcement |
| Isolated plane | Isolate | No network, stripped env |
| Quarantine plane | Quarantine | Memory-only, no I/O |

The key insight from ActPlane: each execution plane should be a complete, self-consistent
environment with explicit boundaries, not a partial restriction layered on top of full
access. Roko implements this: each `SandboxLevel` produces a complete `SandboxPolicy`
with all enforcement decisions pre-computed.

---

## 4. Plugin Tier Mapping

Each `PluginTier` maps monotonically to a `SandboxLevel`:

```rust
impl From<PluginTier> for SandboxLevel {
    fn from(tier: PluginTier) -> Self {
        match tier {
            PluginTier::Kernel    => Self::None,
            PluginTier::Trusted   => Self::Observe,
            PluginTier::Standard  => Self::Restrict,
            PluginTier::Sandboxed => Self::Isolate,
            PluginTier::Untrusted => Self::Quarantine,
        }
    }
}
```

This ensures that more trusted plugins receive more permissive sandboxes and that
the mapping is a monotone function: upgrading trust always widens (or maintains)
access, and downgrading always narrows it.

---

## 5. SandboxPolicy Enforcement

Each level produces a `SandboxPolicy`:

```rust
pub struct SandboxPolicy {
    pub level: SandboxLevel,
    pub config: SandboxConfig,
    pub audit_only: bool,
    pub allow_environment: bool,
}
```

### Effective policies

| Level | SandboxConfig | audit_only | allow_environment |
|---|---|---|---|
| None | unrestricted() | false | true |
| Observe | unrestricted() | true | true |
| Restrict | for_tier_level(3) | false | true |
| Isolate | for_tier_level(2) | false | false |
| Quarantine | most_restricted() | false | false |

### Permission bypass

`None`, `Observe`, and `Restrict` permit bypass when the runner config explicitly
enables `dangerously_skip_permissions`. `Isolate` and `Quarantine` never allow bypass.

```rust
pub const fn allows_permission_bypass(self) -> bool {
    matches!(self, Self::None | Self::Observe | Self::Restrict)
}
```

---

## 6. Path-Level Enforcement

`SandboxPolicy::check_call()` and `check_tool()` enforce path restrictions:

1. Identify the tool's permission requirements from its name or `ToolDef`.
2. For filesystem tools, resolve the path argument against the worktree.
3. Check whether the resolved path matches the sandbox's allowed/denied path patterns.
4. For network tools, check `config.network_access`.
5. For Quarantine, block all capability-bearing tools.

```rust
fn check_permissions(&self, tool_name: &str, permission: ToolPermission,
                     params: &Value, ctx: &ToolContext, path_policy: &PathPolicy)
                     -> Result<(), ToolError> {
    if matches!(self.level, SandboxLevel::None | SandboxLevel::Observe) {
        return Ok(());
    }
    if self.level == SandboxLevel::Quarantine
        && (permission.read || permission.write || permission.exec
            || permission.git || permission.network) {
        return Err(ToolError::PermissionDenied(..));
    }
    if permission.network && !self.config.network_access {
        return Err(ToolError::PermissionDenied(..));
    }
    // Path resolution and pattern matching for filesystem tools...
}
```

---

## 7. Path Pattern Matching

The sandbox uses glob-style path pattern matching:

- `**` matches everything.
- `**/name` matches any path ending with `name`.
- `prefix/**` matches any path starting with `prefix/`.
- `**/segment/**` matches any path containing `segment`.
- `prefix*` matches paths starting with `prefix`.

The deny list is evaluated after the allow list: a path allowed by one pattern but
denied by another is blocked.

---

## 8. Subprocess Enforcement

`SandboxPolicy::check_exec()` handles opaque subprocess launches:

```rust
pub fn check_exec(&self, program: &str) -> Result<(), ToolError> {
    if self.level == SandboxLevel::Quarantine {
        return Err(ToolError::PermissionDenied(
            format!("sandbox quarantine blocks subprocess `{program}`")
        ));
    }
    Ok(())
}
```

Quarantine blocks all subprocesses. Other levels defer to `BashPolicy` for
command-level filtering (force-push, protected branches, dangerous patterns).

---

## 9. Network Egress as a Sandbox Boundary

Network control is a sandbox boundary because outbound connectivity is one of the
main escape routes for compromised logic:

- Host allowlists from profile defaults, plugin manifests, and session approvals.
- Private-network blocking unless explicitly required.
- Principal-aware authorization.
- Durable logging of principal, URL, status.

For `Isolate` and `Quarantine` levels, "no network" means no network capability,
not merely "the docs told the plugin not to call out."

---

## 10. Secrets in Sandboxed Execution

Secrets are a special-case capability:

- Not inherited by default into subprocess environments.
- Manifests must request them explicitly.
- Lower-trust tiers do not receive them without operator approval.
- Outputs derived from secret use pass scrubbing before persistence.

---

## 11. Workspace Authority Rooting

E34 introduced workspace-rooted authority: safety configuration is anchored at the
canonical workspace, not at disposable attempt worktrees. This prevents:

- A temporary worktree from inheriting overly broad write scope.
- Safety bypass by creating a new worktree with different configuration.
- Provider isolation violations through worktree-level configuration injection.

---

## Academic References

| Paper | Contribution |
|---|---|
| ActPlane (Zheng et al. 2025, arXiv:2606.25189) | Layered execution planes for LLM agents |
| Saltzer & Schroeder (1975) | Principle of least privilege |
| Provos (2003) | Preventing Privilege Escalation (systrace) |
| Watson et al. (2010) | Capsicum -- capability-based sandboxing |

---

## Implementation References

| Component | Location |
|---|---|
| SandboxLevel | `crates/roko-agent/src/safety/sandbox.rs` |
| SandboxPolicy | `crates/roko-agent/src/safety/sandbox.rs` |
| SandboxConfig | `crates/roko-std/src/tool.rs` |
| PathPolicy | `crates/roko-agent/src/safety/path.rs` |
| PluginTier | `crates/roko-core/src/plugin.rs` |
| RunnerSandboxLevel | `crates/roko-core/src/config/schema.rs` |
