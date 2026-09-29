# Built-in Tools

> Depth file for [19-TOOLS-PLUGINS](../../19-TOOLS-PLUGINS.md) -- the 16 local
> tools and 19 GitHub MCP tools shipped with `roko-std`.

---

## 1. Overview

Roko ships 35 builtin tool definitions by default: 16 local executables and
19 GitHub MCP catalog entries. With the `chain` cargo feature, 17 chain-domain
placeholder tools bring the total to 52. The 17 chain tools are feature-gated
and typed placeholders -- the chain domain is a separate plugin, not part of
the core framework.

**Note:** The original v1 specification described 423+ chain domain tools
inline. Those 17 retained chain placeholders are now opt-in behind the `chain`
feature flag. The core tool catalog is domain-agnostic.

---

## 2. The 16 Local Tools

Defined in `crates/roko-std/src/tool/builtin/`. Registration order follows
`roko_core::tool::aliases::ALIASES`.

| # | Name | Category | Permission | Description |
|---|---|---|---|---|
| 1 | `read_file` | Read | read | Read file contents with optional offset/limit |
| 2 | `write_file` | Write | write | Write content to a file, creating if necessary |
| 3 | `edit_file` | Write | write | Replace exact string matches in a file |
| 4 | `multi_edit` | Write | write | Apply multiple edits atomically |
| 5 | `glob` | Read | read | Find files matching glob patterns |
| 6 | `grep` | Read | read | Search file contents with regex (ripgrep) |
| 7 | `bash` | Exec | execute | Execute shell commands and return output |
| 8 | `ls` | Read | read | List directory contents |
| 9 | `web_fetch` | Web | network | Fetch content from a URL |
| 10 | `web_search` | Web | network | Search the web for information |
| 11 | `notebook_edit` | Write | write | Edit Jupyter notebook cells |
| 12 | `todo_write` | Planning | write | Write todo items for task tracking |
| 13 | `task` | Agent | execute | Spawn a sub-agent for a delegated task |
| 14 | `exit_plan_mode` | Planning | write | Signal plan completion |
| 15 | `apply_patch` | Write | write | Apply a unified diff patch |
| 16 | `run_tests` | Exec | execute | Run the project's test suite |

Each module (`crates/roko-std/src/tool/builtin/<name>.rs`) exports:

- `pub const NAME: &str` -- canonical `snake_case` name
- `pub const DESCRIPTION: &str` -- LLM-facing help text
- `pub fn tool_def() -> ToolDef` -- full definition constructor

---

## 3. Tool Details

### File I/O (6 tools)

**`read_file`** -- reads a file and returns contents with line numbers in
`cat -n` format. Supports `offset` and `limit` for large files. Used in the
SENSE step when an agent needs to understand existing code.

**`write_file`** -- creates or overwrites a file. The agent must have read
the file first (if it exists) to prevent blind overwrites.

**`edit_file`** -- exact string replacement. The `old_string` must be unique
or `replace_all` must be set. Preferred over `write_file` for targeted
modifications (smaller token cost, clearer intent).

**`multi_edit`** -- batched `edit_file`. Applies multiple replacements across
one or more files in a single operation. Reduces round-trips.

**`notebook_edit`** -- replaces, inserts, or deletes cells in `.ipynb` files.
Handles the JSON structure of Jupyter notebooks transparently.

**`apply_patch`** -- applies a unified diff patch to one or more files.
Useful when the agent has generated a diff.

### Search (3 tools)

**`glob`** -- fast file pattern matching. Patterns like `**/*.rs`. Returns
paths sorted by modification time.

**`grep`** -- content search built on ripgrep. Supports regex, file type
filtering, glob filtering, and output modes (`content`, `files_with_matches`,
`count`). The primary codebase exploration tool.

**`ls`** -- lists files and directories at a path. Quick exploration before
deeper investigation with `glob` or `grep`.

### Execution (2 tools)

**`bash`** -- executes a shell command. Working directory persists between
calls but shell state does not. Used for builds, git, and system interaction.

**`run_tests`** -- runs the project's test suite. Invokes the appropriate
runner based on project type (Cargo for Rust, npm for TypeScript).

### Web (2 tools)

**`web_fetch`** -- HTTP GET for web resources. Used by research agents.

**`web_search`** -- web search via configured provider. Returns titles,
snippets, and URLs.

### Planning / Orchestration (3 tools)

**`todo_write`** -- creates and manages todo items within a session.

**`task`** -- spawns a sub-agent with a specific prompt and returns the
result. Used for task delegation in multi-agent orchestration.

**`exit_plan_mode`** -- signals plan completion and requests user approval.

---

## 4. The 19 GitHub MCP Tools

Defined in `crates/roko-std/src/tool/builtin/github.rs`. All carry
`ToolSource::Mcp { server: "roko-mcp-github" }` and `ToolCategory::Mcp`.

**Read-only (9 tools):**

| Name | Description |
|---|---|
| `github.list_prs` | List pull requests with filters |
| `github.get_pr` | Get pull request details |
| `github.list_issues` | List issues with filters |
| `github.get_file` | Get file contents from a repository |
| `github.search_code` | Search code across repositories |
| `github.list_commits` | List commits on a branch |
| `github.get_branch` | Get branch details |
| `github.compare_branches` | Compare two branches |
| `github.get_actions_status` | Get CI/CD workflow status |

**Write (10 tools):**

| Name | Description |
|---|---|
| `github.create_pr` | Create a pull request |
| `github.comment_pr` | Comment on a pull request |
| `github.review_pr` | Submit a PR review |
| `github.merge_pr` | Merge a pull request |
| `github.create_issue` | Create an issue |
| `github.comment_issue` | Comment on an issue |
| `github.close_issue` | Close an issue |
| `github.add_labels` | Add labels to an issue or PR |
| `github.create_label` | Create a repository label |
| `github.create_branch` | Create a branch |

---

## 5. Chain Domain Tools (Feature-Gated)

The `chain` cargo feature adds 17 chain-domain placeholder tools from
`roko-chain`. These are typed placeholders covering optional on-chain
operations. The chain domain is a domain plugin -- not part of the core
framework. The original v1 specification described 423+ DeFi tools across
17 chain categories; these have been consolidated into typed placeholders
that fail closed when the chain feature is not enabled.

---

## 6. Registration

Tools are registered via `LazyLock` in `builtin/mod.rs`:

```rust
pub static ROKO_BUILTIN_TOOLS: LazyLock<Vec<ToolDef>> = LazyLock::new(|| {
    // 16 local tools + 19 GitHub MCP tools
    // (+ 17 chain tools when feature = "chain")
});

#[cfg(not(feature = "chain"))]
pub const TOOL_COUNT: usize = 35;

#[cfg(feature = "chain")]
pub const TOOL_COUNT: usize = 52;
```

---

## 7. Module Structure

```
crates/roko-std/src/tool/
  mod.rs              -- module structure, re-exports
  builtin/
    mod.rs            -- tool definitions + LazyLock registration
    github.rs         -- 19 GitHub MCP tool definitions
  registry.rs         -- StaticToolRegistry + role-based filtering
  handlers.rs         -- handler dispatch
  sandbox_config.rs   -- SandboxConfig per tier
  expand_pointer.rs   -- JSON pointer expansion
  mock_dispatcher.rs  -- MockToolDispatcher for testing
```

---

## 8. Key Tests

- `builtin_count_matches` -- `ROKO_BUILTIN_TOOLS.len() == TOOL_COUNT`
- `no_duplicate_names` -- all tool names are unique
- `github_tool_catalog_entries` -- all 19 GitHub MCP tools present
- `for_role_implementer_is_nonempty` -- Implementer sees tools
- `for_role_auditor_is_read_only_subset` -- Auditor restricted

---

*Derived from: v1/18-tools/01-builtin-tools.md. Chain-domain tools (423+) removed
from inline documentation; 17 retained as feature-gated placeholders.*
