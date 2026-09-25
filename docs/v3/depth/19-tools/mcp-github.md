# MCP GitHub Integration

> Depth file for [19-TOOLS-PLUGINS](../../19-TOOLS-PLUGINS.md) -- the 19 GitHub
> MCP tool catalog entries shipped with `roko-std`, their read/write
> classification, and how they integrate with the `roko-mcp-github` server.

---

## 1. Overview

`roko-std` ships 19 GitHub MCP tool catalog entries alongside the 16 standard
built-in tools. These entries are defined in
`crates/roko-std/src/tool/builtin/github.rs` and describe tools that are
dispatched through a `roko-mcp-github` MCP server process rather than executed
in-process. The catalog entries carry `ToolSource::Mcp { server:
"roko-mcp-github" }` so the `validate_tool_catalog()` check exempts them from
the local handler requirement.

---

## 2. The 19 GitHub Tools

### 2.1 Read-Only Tools (9)

| Tool | Description |
|---|---|
| `github.list_prs` | List pull requests with state/author/label filters |
| `github.get_pr` | Get pull request details including diff and comments |
| `github.list_issues` | List issues with state/label/assignee filters |
| `github.get_file` | Get file contents at a branch, tag, or commit |
| `github.search_code` | Search code across repositories |
| `github.list_commits` | List commits on a branch |
| `github.get_branch` | Get branch details and protection status |
| `github.compare_branches` | Compare two branches (commits and diff) |
| `github.get_actions_status` | Get CI/CD check status for a ref |

All read-only tools have `permission.write = false`. They are available to every
role including Auditor and Researcher.

### 2.2 Write Tools (10)

| Tool | Description |
|---|---|
| `github.create_pr` | Create a new pull request |
| `github.comment_pr` | Add a comment to a pull request |
| `github.review_pr` | Submit a PR review (APPROVE/COMMENT/REQUEST_CHANGES) |
| `github.merge_pr` | Merge a pull request (squash/merge/rebase) |
| `github.create_issue` | Create a new issue with labels and assignees |
| `github.comment_issue` | Add a comment to an issue |
| `github.close_issue` | Close an issue |
| `github.add_labels` | Add labels to an issue or PR |
| `github.create_label` | Create a new repository label |
| `github.create_branch` | Create a new branch from a ref |

All write tools have `permission.write = true`. They are filtered out for
read-only roles (Auditor, Researcher).

---

## 3. Catalog Registration

GitHub tools are registered alongside standard tools in the `ROKO_BUILTIN_TOOLS`
LazyLock:

```rust
// crates/roko-std/src/tool/builtin/mod.rs
pub static ROKO_BUILTIN_TOOLS: LazyLock<Vec<ToolDef>> = LazyLock::new(|| {
    let mut tools = vec![
        // ... 16 standard tools ...
    ];
    tools.extend(github::all_tool_defs());
    tools
});
```

The `github` module exposes:
- `GITHUB_TOOL_COUNT: usize = 19` -- compile-time constant
- `GITHUB_TOOL_NAMES: [&str; 19]` -- canonical name list
- `all_tool_defs() -> Vec<ToolDef>` -- constructs all 19 definitions

Every GitHub tool has `category: ToolCategory::Mcp` and
`source: ToolSource::Mcp { server: "roko-mcp-github".to_string() }`.

---

## 4. Authentication

The MCP server authenticates via environment variable:

```toml
# roko.toml
[[agent.mcp_servers]]
name = "github"
command = "roko-mcp-github"
args = ["--repo", "nunchi/roko"]
env = { GITHUB_TOKEN = "${GITHUB_TOKEN}" }
```

Two authentication methods are supported:
- **GitHub App installation token** -- recommended for organizations
- **Personal Access Token (PAT)** -- simpler setup for individual use

The token is resolved from the host environment and passed to the MCP server
process. It never traverses the stdio channel.

---

## 5. Rate Limiting

GitHub API rate limits are handled by the MCP server:

| Auth Method | Limit | Reset |
|---|---|---|
| GitHub App | 5,000 requests/hour | Per installation |
| PAT | 5,000 requests/hour | Per token |
| Search API | 30 requests/minute | Per token |

The server implements exponential backoff with full jitter for 429 responses.

---

## 6. Integration with `roko github status`

The `roko github status` CLI command (E46) inspects GitHub configuration,
authentication state, plan PR/CI status, and failure issues. It operates through
the same MCP server configuration used by agents.

---

## 7. Test Coverage

Registry tests in `crates/roko-std/src/tool/registry.rs` verify:

- All 19 GitHub tools appear in the catalog
- Each tool has `category: Mcp` and `source: Mcp { server: "roko-mcp-github" }`
- Write tools have `permission.write = true`
- Read-only tools have `permission.write = false`
- `validate_tool_catalog()` does not flag MCP tools as unhandled

---

## 8. Source Locations

| Component | Path |
|---|---|
| GitHub tool definitions | `crates/roko-std/src/tool/builtin/github.rs` |
| Tool registry tests | `crates/roko-std/src/tool/registry.rs` |
| GitHub workflow integration | `crates/roko-cli/src/commands/github.rs` |

---

*Derived from: v1/18-tools/10-mcp-github.md. Bounty program removed (internal
project). Tool count corrected from 17 to 19 per actual implementation.*
