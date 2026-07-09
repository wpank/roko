# 08 — Per-Role Tool Allowlists

## The Problem

Mori enforces strict per-role tool restrictions (e.g., Conductor is read-only).
Roko has `claude_tool_allowlist()` but it's only used in one dispatch path.

---

## Mori's Per-Role Tool Allowlists
**File**: `connection.rs:2483-2536`

| Role | Tools | Permission Mode |
|------|-------|----------------|
| **Conductor** | Read, Glob, Grep, WebFetch, WebSearch | `plan` |
| **Scribe** | Read, Glob, Grep, Write, Edit, WebFetch, WebSearch | `dangerously-skip-permissions` |
| **QuickReviewer, Auditor, Critic, Architect** | Read, Glob, Grep, Bash, WebFetch, WebSearch + `--json-schema` | `dangerously-skip-permissions` |
| **Researcher** | Read, Glob, Grep, Bash, WebFetch, WebSearch | `dangerously-skip-permissions` |
| **Implementer/AutoFixer (low/medium effort)** | Read, Glob, Grep, Edit, Write, Bash | `dangerously-skip-permissions` |
| **Implementer/AutoFixer (high/max effort)** | Read, Glob, Grep, Edit, Write, Bash, WebFetch, WebSearch | `dangerously-skip-permissions` |
| **Other roles** | Read, Glob, Grep, Edit, Write, Bash, WebFetch, WebSearch | `dangerously-skip-permissions` |

Key observations:
- **Conductor is read-only** — no Edit, Write, Bash. Uses `plan` permission mode.
- **Implementer gets web tools only at high/max effort** — saves tokens by omitting Agent, NotebookEdit, WebFetch, WebSearch at low effort
- **Reviewers/Auditors get Bash but not Edit/Write** — can inspect but not modify
- **Agent tool excluded from all roles** — prevents recursive spawning

### MCP Skipped for Some Roles
MCP config is NOT passed to:
- **AutoFixer** — needs speed, MCP startup adds 60s latency
- **Conductor** — doesn't need code search tools

### Safety Hooks
**File**: `connection.rs:647-678`

Git operations blocked via hooks:
- `git checkout *` → BLOCKED
- `git switch *` → BLOCKED
- `git branch -m *` → BLOCKED
- `git push *` → BLOCKED

---

## Roko's Tool Handling

### claude_tool_allowlist()
**File**: `crates/roko-cli/src/run.rs`

```rust
fn claude_tool_allowlist(role: &str) -> Vec<String> {
    // Returns CSV of tools based on role
}
```

Only used in dispatch Path 3 (Claude CLI subprocess in run.rs).
Not used in:
- dispatch_direct.rs (chat) — no tool allowlist at all
- orchestrate.rs — has its own tool handling via RoleSystemPromptSpec
- HTTP dispatch — tools determined by sidecar/serve config

### Inconsistencies

| Feature | Mori | Roko |
|---------|------|------|
| Conductor read-only | Enforced | Not enforced — all tools available |
| Permission mode per role | `plan` for conductor, `dsp` for others | `dangerously_skip_permissions` everywhere or nothing |
| Effort-based tool filtering | Yes (high effort → web tools) | No |
| MCP skipped for speed-critical roles | Yes (AutoFixer, Conductor) | No — MCP always or never |
| Safety hooks (git protection) | Yes — blocked via hooks.json | No |
| Agent tool excluded | Yes — prevents recursive spawn | Not checked |

---

## What Needs to Change

1. **Apply tool allowlists in all dispatch paths** — not just Claude CLI subprocess
2. **Conductor must be read-only** — critical for safety
3. **Add git safety hooks** — prevent agents from pushing, checking out, etc.
4. **Effort-based filtering** — reduce token cost at low/medium effort
5. **Permission mode per role** — conductor gets `plan`, others get `dsp`
6. **Skip MCP for speed-critical roles** — AutoFixer and Conductor
