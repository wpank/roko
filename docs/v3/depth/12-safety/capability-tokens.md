# Capability Tokens

> **v3 depth file** -- `/docs/v3/depth/12-safety/capability-tokens.md`
> Canonical source: v1 `docs/v1/11-safety/01-capability-tokens.md`
> Status: **Shipping runtime foundation** -- `AgentWarrant`, `Capability` enum, and
> `ToolPermission` enforce capability checks at dispatch time. The compile-time
> generic `Capability<T>` is a target-state extension.

---

## 1. The Problem: Runtime Checks Alone Are Not Sufficient

The standard agent safety pattern: the LLM proposes a tool call, a safety hook checks
it, blocks if it violates policy. Roko does this with dispatcher permissions, warrants,
and `SafetyLayer` checks. The limitation is that this is a runtime guard. If the hook
has a bug, if the hook chain is bypassed by an unexpected code path, if a race condition
opens a window between check and execution, the tool may still execute.

Dennis and Van Horn (1966) established capability-based security: access rights should be
unforgeable tokens verified at the type level, not runtime guards. The WASM Component
Model (Haas et al. 2017) instantiates this for sandboxed execution.

---

## 2. Current Implementation: Runtime Permissions and Warrants

Two shipping layers compose to form the live capability system:

### AgentWarrant and Capability enum

`roko-agent/src/safety/capabilities.rs` provides:

- `AgentWarrant` -- a runtime capability bundle for an agent session.
- `Capability` enum with variants: `Tool`, `ReadPath`, `WritePath`, `Exec`, `Network`.
- `check_capability()` -- verify a warrant covers the requested capability.
- `delegate()` -- narrow a warrant to produce a strictly weaker sub-warrant.

`SafetyLayer::check_pre_execution()` checks warrants before tool dispatch.

### ToolPermission and ToolPermissions

`roko-core` provides `ToolPermission` (what a tool requires) and `ToolPermissions`
(what a role grants). Five flags: read, write, exec, git, network.

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

When a check fails, the dispatcher emits an audit Signal with `phase=permission` and
`status=denied`, including both required and granted sets for debugging.

---

## 3. Three-Layer Capability Intersection

E34 introduced exact intersection across three capability scopes:

| Layer | Source | Example |
|---|---|---|
| Cell | The Cell's declared type contract | `WriterCell` permits writes |
| Graph | The Graph topology's edge authorization | Edge allows write propagation |
| Space | The runtime execution space | Workspace-rooted authority |

An action proceeds only at the intersection. This means:

- A Cell with write permission in a read-only Graph: denied.
- A Graph with network edges in an offline Space: denied.
- A Space with full capability but a Cell lacking exec: denied.

No single layer can widen what another restricts. The intersection is conjunctive.

---

## 4. Role Permission Matrix

| Role | read | write | exec | git | network | Rationale |
|------|------|-------|------|-----|---------|-----------|
| Implementer | yes | yes | yes | yes | no | Write code, run builds, commit |
| Auditor | yes | no | yes | no | no | Review code, run analysis |
| Researcher | yes | no | no | no | yes | Read code, query external |
| Planner | yes | no | no | no | no | Read-only plan inspection |
| Reviewer | yes | no | yes | no | no | Read code, run verification |

---

## 5. Task-Level Tool Filters

Beyond role permissions, individual tasks can restrict which tools are available:

```rust
pub struct ToolContext {
    pub allowed_tools: Option<Vec<String>>,  // If set, only these tools
    pub denied_tools: Option<Vec<String>>,   // If set, these are blocked
}
```

The deny list is evaluated before the allow list. A tool on both lists is blocked. The
full evaluation order in the dispatcher:

1. Schema validation against the registry's JSON schema.
2. Tool existence in the registry.
3. Deny list check.
4. Allow list check.
5. Permission check via `satisfied_by()`.
6. Safety layer pre-execution checks.

---

## 6. Target-State Design: Compile-Time Capability<T>

The target-state extension uses Rust's ownership system for four properties:

**Unforgeability.** The `Capability<T>` constructor is `pub(crate)`. No code outside the
safety module can create one. No `Default`, `Clone`, or `Copy` implementation.

**Single-use.** When passed to `execute_write()`, Rust's move semantics transfer
ownership. The caller no longer has the capability. Double-spending is a compile error.

**Type-safety.** `Capability<SwapTool>` cannot be used with `execute_write()` on a
`DepositTool`. `PhantomData<T>` binds the token to a specific tool type.

**Temporal validity.** The `expires_at` field provides runtime temporal bounds.

```rust
pub struct Capability<T> {
    pub value_limit: f64,
    pub expires_at: u64,
    pub policy_hash: [u8; 32],
    pub permit_id: String,
    _marker: PhantomData<T>,
}
```

### Three tool tiers in the target design

| Tier | Trait | Capability requirement |
|------|-------|----------------------|
| 1 (Read) | `ReadTool` | None -- cannot modify state |
| 2 (Write) | `WriteTool` | `Capability<Self>` consumed on execution |
| 3 (Privileged) | `PrivilegedTool` | `Capability<Self>` plus `OwnerApproval` |

---

## 7. Capability Lifecycle Events

Every lifecycle event is emitted as a Signal through the audit sink:

| Event | Description |
|---|---|
| `PermitCreated` | New capability minted by safety layer |
| `PermitConsumed` | Capability consumed by a write tool |
| `PermitExpired` | Capability expired without use |
| `PermitDenied` | Capability request rejected |

These events form part of the content-addressed audit DAG, enabling forensic
reconstruction of every safety decision.

---

## 8. Interaction with Speculative Execution

The dual-process cognition system can only speculate on Tier 1 (read-only) tools.
Speculating on a Tier 2 tool is impossible to write in the target-state design:

```rust
// This compiles -- read tools need no capability:
async fn speculate_read(tool: &dyn ReadTool) {
    let _ = tool.execute_read(serde_json::Value::Null).await;
}

// This does NOT compile -- no way to construct the Capability:
// async fn speculate_write(tool: &dyn WriteTool) {
//     tool.execute_write(serde_json::Value::Null, ???).await;
//     //                                          ^^^ no capability
// }
```

---

## 9. Delegation and Narrowing

Capabilities can be delegated but never widened:

- `delegate()` produces a strictly weaker sub-warrant.
- A child warrant cannot exceed its parent's scope.
- Delegation is logged to the audit chain.

This supports the non-widening invariant: no action in the safety pipeline can increase
an agent's capability beyond what was initially granted.

---

## Academic References

| Paper | Contribution |
|---|---|
| Dennis & Van Horn (1966) | Capability-based security -- unforgeable tokens |
| Haas et al. (2017) | WASM Component Model -- sandboxed capabilities |
| Lampson (1974) | Protection -- formal access control model |
| Levy (1984) | Capability-Based Computer Systems |
| Miller et al. (2003) | Robust Composition -- object-capability patterns |
| Watson et al. (2015) | CHERI -- hardware-enforced capabilities |

---

## Implementation References

| Component | Location |
|---|---|
| AgentWarrant + Capability enum | `crates/roko-agent/src/safety/capabilities.rs` |
| SafetyLayer | `crates/roko-agent/src/safety/mod.rs` |
| ToolPermission / ToolPermissions | `crates/roko-core/src/tool.rs` |
| ToolDispatcher | `crates/roko-agent/src/dispatcher/mod.rs` |
| SandboxPolicy | `crates/roko-agent/src/safety/sandbox.rs` |
