# Tool Categories

> Depth file for [19-TOOLS-PLUGINS](../../19-TOOLS-PLUGINS.md) -- category
> taxonomy, prefix conventions, risk tier scale, and profile-to-category mapping.

---

## 1. Overview

Every `ToolDef` has a `category: ToolCategory` field that drives profile
filtering. The built-in tools use seven categories (Read, Write, Exec, Web,
Planning, Agent, Mcp). Domain plugins define their own category sets.

The original v1 specification documented 17 chain-domain categories with 423+
tools. Those chain categories are now part of the chain domain plugin -- a
separate crate, not the core framework. This document covers the
domain-agnostic category system.

---

## 2. The 7 Core Categories

| Category | Prefix Convention | Permission | Examples |
|---|---|---|---|
| `Read` | file/search verbs | read_only | `read_file`, `glob`, `grep`, `ls` |
| `Write` | file mutation verbs | writes | `write_file`, `edit_file`, `multi_edit`, `apply_patch`, `notebook_edit` |
| `Exec` | process verbs | executes | `bash`, `run_tests` |
| `Web` | `web_` | network | `web_fetch`, `web_search` |
| `Planning` | task/plan verbs | writes | `todo_write`, `exit_plan_mode` |
| `Agent` | orchestration verbs | executes | `task` |
| `Mcp` | `<server>.` | varies | `github.get_pr`, `github.create_pr` |

---

## 3. Risk Tier Scale

Tools use a three-layer risk classification. The core framework applies this
for built-in tools; domain plugins extend it with domain-specific escalation.

### Layer 1 (Low Risk)

Read-only operations. No state mutation. No capability token required.

**Examples:** `read_file`, `glob`, `grep`, `ls`, `web_search`

### Layer 2 (Medium Risk)

Bounded write operations. Subject to role authorization and safety checks.

**Examples:** `write_file`, `edit_file`, `bash`, `notebook_edit`

### Layer 3 (High Risk)

Unbounded writes or privileged operations. Subject to additional verification.

**Examples:** Domain-specific admin operations, unrestricted process execution

### Risk Classification Table (Built-in Tools)

| Category | Default Risk | Notes |
|---|---|---|
| Read | Layer 1 | Always safe, no mutation |
| Write | Layer 2 | File modifications, bounded |
| Exec | Layer 2 | Process execution, bounded by timeout |
| Web | Layer 1 | Read-only network access |
| Planning | Layer 2 | State management writes |
| Agent | Layer 2 | Sub-agent delegation |
| Mcp | Layer 2 (conservative default) | MCP tools default to write-level |

---

## 4. Prefix Convention

All tool names follow `<prefix>_<action>_<subject>` or `<namespace>.<action>`.

### Built-in Prefixes

| Prefix | Subsystem | Example |
|---|---|---|
| *(none)* | Core file/search tools | `read_file`, `glob`, `grep` |
| `web_` | Web access | `web_fetch`, `web_search` |
| `github.` | GitHub MCP | `github.get_pr`, `github.create_pr` |

### Domain Plugin Prefixes

Domain plugins register their own prefixes. The `validate_tool_catalog()`
function flags deprecated prefixes (`legacy.`, `deprecated.`, `old_`).

---

## 5. Profile-to-Category Mapping

Profiles compose categories. A profile defines which categories are loaded at
boot. The built-in tools use role-based access (see Section 6) rather than
profile-based filtering, since all 16 local tools are available regardless
of domain.

Domain plugins ship their own profile bundles. A profile is a named selection
of categories with companion extensions. An agent with a read-only profile is
**structurally unable** to write -- the write tool handlers do not exist in
its registry. This is not a runtime policy check; it is a structural absence.

### Profile Composition

Profiles compose -- `TOOL_PROFILE=trader,vault` activates both sets:

```rust
let allowed = resolve_profile_categories(profile);
let tools: Vec<&ToolDef> = ALL_TOOL_DEFS
    .iter()
    .filter(|t| allowed.contains(&t.category))
    .collect();
```

### Fine-Grained Overrides

Per-tool enable/disable overrides take precedence over profiles:

```toml
[tools]
profile = "active"
enable = ["intel_compute_vpin"]
disable = ["uniswap_submit_uniswapx_order"]
```

---

## 6. Role-Based Access Control

The `StaticToolRegistry` filters tools per agent role via `for_role()`:

| Role | Available Tools | Rationale |
|---|---|---|
| **Implementer** | read + write + exec | Full access for code writing |
| **Reviewer** | read-only (no write, no exec) | Can inspect but not modify |
| **Researcher** | read + web | Can search and read, no write |
| **Architect** | read + search | Can inspect and plan |
| **Scribe** | read + write | Can read and write docs |
| **Auditor** | read-only | Strictest: read-only inspection |

---

## 7. Category Interaction with Cognitive Subsystems

Categories influence how cognitive subsystems process tool results:

| Category | Neuro Storage | Daimon Influence | Dream Replay |
|---|---|---|---|
| Read | Store as transient knowledge | No affect change | Rarely replayed |
| Write | Store as working-tier episodes | Success/failure affect | Standard replay |
| Exec | Store as working-tier episodes | Arousal adjustment | Standard replay |
| Web | Store as transient knowledge | Novelty detection | Selective replay |

This interaction is automatic. The cognitive loop processes tool results
through the VERIFY -> PERSIST -> ADAPT -> META-COGNIZE pipeline regardless of
category. The category metadata informs how each subsystem weights the result.

---

*Derived from: v1/18-tools/02-tool-categories.md. Chain-domain 17 categories
(data, trading, LP, vault, lending, staking, restaking, derivatives, yield,
safety, intelligence, memory, identity, wallet, streaming, testnet, bootstrap)
removed -- now part of the chain domain plugin. Core categories retained.*
