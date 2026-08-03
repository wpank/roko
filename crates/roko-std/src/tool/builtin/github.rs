//! GitHub MCP tool catalog entries.
//!
//! These tools are MCP-backed (dispatched by the `roko-mcp-github` server),
//! **not** handler-backed. They are registered in the builtin tool catalog so
//! that:
//!
//! - Agents know GitHub tools exist even before the MCP server is running.
//! - Permission checks work correctly (write tools vs. read tools).
//! - Tool-listing endpoints return a complete catalog.
//!
//! The `source` field on every [`ToolDef`] is set to
//! [`ToolSource::Mcp`](roko_core::tool::ToolSource::Mcp) with
//! `server = "roko-mcp-github"`, which signals to the dispatcher that the
//! tool must be routed through the MCP client, not through
//! [`handler_for`](crate::tool::handlers::handler_for). Consequently, the
//! advertised-handler parity test (E14-T07) correctly excludes these tools.

use roko_core::tool::{
    ToolCategory, ToolConcurrency, ToolDef, ToolPermission, ToolSchema, ToolSource,
};

// ─── MCP server identifier ────────────────────────────────────────────────────

const MCP_SERVER: &str = "roko-mcp-github";

// ─── Tool names (match roko-mcp-github handle_tools_list exactly) ─────────────

/// Canonical name: list pull requests.
pub const GITHUB_LIST_PRS: &str = "github.list_prs";
/// Canonical name: get a pull request.
pub const GITHUB_GET_PR: &str = "github.get_pr";
/// Canonical name: create a pull request.
pub const GITHUB_CREATE_PR: &str = "github.create_pr";
/// Canonical name: comment on a pull request.
pub const GITHUB_COMMENT_PR: &str = "github.comment_pr";
/// Canonical name: review a pull request.
pub const GITHUB_REVIEW_PR: &str = "github.review_pr";
/// Canonical name: merge a pull request.
pub const GITHUB_MERGE_PR: &str = "github.merge_pr";
/// Canonical name: list issues.
pub const GITHUB_LIST_ISSUES: &str = "github.list_issues";
/// Canonical name: create an issue.
pub const GITHUB_CREATE_ISSUE: &str = "github.create_issue";
/// Canonical name: comment on an issue.
pub const GITHUB_COMMENT_ISSUE: &str = "github.comment_issue";
/// Canonical name: close an issue.
pub const GITHUB_CLOSE_ISSUE: &str = "github.close_issue";
/// Canonical name: add labels to an issue.
pub const GITHUB_ADD_LABELS: &str = "github.add_labels";
/// Canonical name: create a repository label.
pub const GITHUB_CREATE_LABEL: &str = "github.create_label";
/// Canonical name: fetch a file from a repository.
pub const GITHUB_GET_FILE: &str = "github.get_file";
/// Canonical name: search code in GitHub repositories.
pub const GITHUB_SEARCH_CODE: &str = "github.search_code";
/// Canonical name: list commits.
pub const GITHUB_LIST_COMMITS: &str = "github.list_commits";
/// Canonical name: create a branch.
pub const GITHUB_CREATE_BRANCH: &str = "github.create_branch";
/// Canonical name: get branch metadata.
pub const GITHUB_GET_BRANCH: &str = "github.get_branch";
/// Canonical name: compare two branches.
pub const GITHUB_COMPARE_BRANCHES: &str = "github.compare_branches";
/// Canonical name: get combined GitHub Actions status for a ref.
pub const GITHUB_GET_ACTIONS_STATUS: &str = "github.get_actions_status";

// ─── Tool count ───────────────────────────────────────────────────────────────

/// Total number of GitHub MCP tools.
pub const GITHUB_TOOL_COUNT: usize = 19;

/// Canonical names of all GitHub MCP tools, in [`all_tool_defs`] order.
pub const GITHUB_TOOL_NAMES: [&str; GITHUB_TOOL_COUNT] = [
    GITHUB_LIST_PRS,
    GITHUB_GET_PR,
    GITHUB_CREATE_PR,
    GITHUB_COMMENT_PR,
    GITHUB_REVIEW_PR,
    GITHUB_MERGE_PR,
    GITHUB_LIST_ISSUES,
    GITHUB_CREATE_ISSUE,
    GITHUB_COMMENT_ISSUE,
    GITHUB_CLOSE_ISSUE,
    GITHUB_ADD_LABELS,
    GITHUB_CREATE_LABEL,
    GITHUB_GET_FILE,
    GITHUB_SEARCH_CODE,
    GITHUB_LIST_COMMITS,
    GITHUB_CREATE_BRANCH,
    GITHUB_GET_BRANCH,
    GITHUB_COMPARE_BRANCHES,
    GITHUB_GET_ACTIONS_STATUS,
];

// ─── Internal helpers ─────────────────────────────────────────────────────────

/// Build the MCP source tag for every GitHub tool.
fn mcp_source() -> ToolSource {
    ToolSource::Mcp {
        server: MCP_SERVER.to_string(),
    }
}

/// Base builder for a read-only GitHub MCP tool.
fn github_read_tool(
    name: &'static str,
    description: &'static str,
    schema: serde_json::Value,
) -> ToolDef {
    let mut def = ToolDef::new(
        name,
        description,
        ToolCategory::Mcp,
        ToolPermission::read_only(),
    )
    .with_parameters(ToolSchema::from_value(schema))
    .with_concurrency(ToolConcurrency::Parallel)
    .with_idempotent(true)
    .with_timeout_ms(30_000);
    def.source = mcp_source();
    def
}

/// Base builder for a write GitHub MCP tool (read + write permissions).
fn github_write_tool(
    name: &'static str,
    description: &'static str,
    schema: serde_json::Value,
) -> ToolDef {
    let mut def = ToolDef::new(
        name,
        description,
        ToolCategory::Mcp,
        ToolPermission::writes(),
    )
    .with_parameters(ToolSchema::from_value(schema))
    .with_concurrency(ToolConcurrency::Serial)
    .with_idempotent(false)
    .with_timeout_ms(60_000);
    def.source = mcp_source();
    def
}

// ─── Tool definitions ─────────────────────────────────────────────────────────

/// `github.list_prs` — list pull requests in a repository.
pub fn tool_def_list_prs() -> ToolDef {
    github_read_tool(
        GITHUB_LIST_PRS,
        "List pull requests in a repository.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "owner": {"type": "string"},
                "repo": {"type": "string"},
                "state": {"type": "string", "enum": ["open", "closed", "all"]},
                "head": {"type": "string"},
                "base": {"type": "string"},
                "per_page": {"type": "integer", "minimum": 1}
            },
            "required": ["owner", "repo"],
            "additionalProperties": false
        }),
    )
}

/// `github.get_pr` — get a pull request with diff stats and review summary.
pub fn tool_def_get_pr() -> ToolDef {
    github_read_tool(
        GITHUB_GET_PR,
        "Get a pull request with diff stats and review summary.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "owner": {"type": "string"},
                "repo": {"type": "string"},
                "number": {"type": "integer", "minimum": 1}
            },
            "required": ["owner", "repo", "number"],
            "additionalProperties": false
        }),
    )
}

/// `github.create_pr` — create a pull request.
pub fn tool_def_create_pr() -> ToolDef {
    github_write_tool(
        GITHUB_CREATE_PR,
        "Create a pull request.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "owner": {"type": "string"},
                "repo": {"type": "string"},
                "title": {"type": "string"},
                "body": {"type": "string"},
                "head": {"type": "string"},
                "base": {"type": "string"},
                "draft": {"type": "boolean"}
            },
            "required": ["owner", "repo", "title", "body", "head", "base"],
            "additionalProperties": false
        }),
    )
}

/// `github.comment_pr` — comment on a pull request.
pub fn tool_def_comment_pr() -> ToolDef {
    github_write_tool(
        GITHUB_COMMENT_PR,
        "Comment on a pull request.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "owner": {"type": "string"},
                "repo": {"type": "string"},
                "number": {"type": "integer", "minimum": 1},
                "body": {"type": "string"}
            },
            "required": ["owner", "repo", "number", "body"],
            "additionalProperties": false
        }),
    )
}

/// `github.review_pr` — create a pull request review.
pub fn tool_def_review_pr() -> ToolDef {
    github_write_tool(
        GITHUB_REVIEW_PR,
        "Create a pull request review.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "owner": {"type": "string"},
                "repo": {"type": "string"},
                "number": {"type": "integer", "minimum": 1},
                "event": {"type": "string", "enum": ["APPROVE", "REQUEST_CHANGES", "COMMENT"]},
                "body": {"type": "string"},
                "comments": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "path": {"type": "string"},
                            "line": {"type": "integer", "minimum": 1},
                            "body": {"type": "string"}
                        },
                        "required": ["path", "line", "body"],
                        "additionalProperties": false
                    }
                }
            },
            "required": ["owner", "repo", "number", "event", "body"],
            "additionalProperties": false
        }),
    )
}

/// `github.merge_pr` — merge a pull request.
pub fn tool_def_merge_pr() -> ToolDef {
    github_write_tool(
        GITHUB_MERGE_PR,
        "Merge a pull request.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "owner": {"type": "string"},
                "repo": {"type": "string"},
                "number": {"type": "integer", "minimum": 1},
                "merge_method": {"type": "string", "enum": ["merge", "squash", "rebase"]},
                "commit_title": {"type": "string"}
            },
            "required": ["owner", "repo", "number"],
            "additionalProperties": false
        }),
    )
}

/// `github.list_issues` — list issues in a repository.
pub fn tool_def_list_issues() -> ToolDef {
    github_read_tool(
        GITHUB_LIST_ISSUES,
        "List issues in a repository.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "owner": {"type": "string"},
                "repo": {"type": "string"},
                "state": {"type": "string"},
                "labels": {
                    "type": "array",
                    "items": {"type": "string"}
                },
                "assignee": {"type": "string"},
                "per_page": {"type": "integer", "minimum": 1}
            },
            "required": ["owner", "repo"],
            "additionalProperties": false
        }),
    )
}

/// `github.create_issue` — create an issue.
pub fn tool_def_create_issue() -> ToolDef {
    github_write_tool(
        GITHUB_CREATE_ISSUE,
        "Create an issue.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "owner": {"type": "string"},
                "repo": {"type": "string"},
                "title": {"type": "string"},
                "body": {"type": "string"},
                "labels": {
                    "type": "array",
                    "items": {"type": "string"}
                },
                "assignees": {
                    "type": "array",
                    "items": {"type": "string"}
                }
            },
            "required": ["owner", "repo", "title", "body"],
            "additionalProperties": false
        }),
    )
}

/// `github.comment_issue` — comment on an issue.
pub fn tool_def_comment_issue() -> ToolDef {
    github_write_tool(
        GITHUB_COMMENT_ISSUE,
        "Comment on an issue.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "owner": {"type": "string"},
                "repo": {"type": "string"},
                "number": {"type": "integer", "minimum": 1},
                "body": {"type": "string"}
            },
            "required": ["owner", "repo", "number", "body"],
            "additionalProperties": false
        }),
    )
}

/// `github.close_issue` — close an issue.
pub fn tool_def_close_issue() -> ToolDef {
    github_write_tool(
        GITHUB_CLOSE_ISSUE,
        "Close an issue.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "owner": {"type": "string"},
                "repo": {"type": "string"},
                "number": {"type": "integer", "minimum": 1},
                "reason": {"type": "string", "enum": ["completed", "not_planned"]}
            },
            "required": ["owner", "repo", "number"],
            "additionalProperties": false
        }),
    )
}

/// `github.add_labels` — add labels to an issue.
pub fn tool_def_add_labels() -> ToolDef {
    github_write_tool(
        GITHUB_ADD_LABELS,
        "Add labels to an issue.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "owner": {"type": "string"},
                "repo": {"type": "string"},
                "number": {"type": "integer", "minimum": 1},
                "labels": {
                    "type": "array",
                    "items": {"type": "string"}
                }
            },
            "required": ["owner", "repo", "number", "labels"],
            "additionalProperties": false
        }),
    )
}

/// `github.create_label` — create a repository label.
pub fn tool_def_create_label() -> ToolDef {
    github_write_tool(
        GITHUB_CREATE_LABEL,
        "Create a repository label.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "owner": {"type": "string"},
                "repo": {"type": "string"},
                "name": {"type": "string"},
                "color": {"type": "string"},
                "description": {"type": "string"}
            },
            "required": ["owner", "repo", "name", "color"],
            "additionalProperties": false
        }),
    )
}

/// `github.get_file` — fetch a file from a repository.
pub fn tool_def_get_file() -> ToolDef {
    github_read_tool(
        GITHUB_GET_FILE,
        "Fetch a file from a repository.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "owner": {"type": "string"},
                "repo": {"type": "string"},
                "path": {"type": "string"},
                "ref": {"type": "string"}
            },
            "required": ["owner", "repo", "path"],
            "additionalProperties": false
        }),
    )
}

/// `github.search_code` — search code in GitHub repositories.
pub fn tool_def_search_code() -> ToolDef {
    github_read_tool(
        GITHUB_SEARCH_CODE,
        "Search code in GitHub repositories.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {"type": "string"},
                "owner": {"type": "string"},
                "repo": {"type": "string"},
                "per_page": {"type": "integer", "minimum": 1}
            },
            "required": ["query", "owner", "repo"],
            "additionalProperties": false
        }),
    )
}

/// `github.list_commits` — list commits in a repository.
pub fn tool_def_list_commits() -> ToolDef {
    github_read_tool(
        GITHUB_LIST_COMMITS,
        "List commits in a repository.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "owner": {"type": "string"},
                "repo": {"type": "string"},
                "sha": {"type": "string"},
                "path": {"type": "string"},
                "since": {"type": "string"},
                "until": {"type": "string"},
                "per_page": {"type": "integer", "minimum": 1}
            },
            "required": ["owner", "repo"],
            "additionalProperties": false
        }),
    )
}

/// `github.create_branch` — create a branch from a commit SHA.
pub fn tool_def_create_branch() -> ToolDef {
    github_write_tool(
        GITHUB_CREATE_BRANCH,
        "Create a branch from a commit SHA.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "owner": {"type": "string"},
                "repo": {"type": "string"},
                "branch": {"type": "string"},
                "from_sha": {"type": "string"}
            },
            "required": ["owner", "repo", "branch", "from_sha"],
            "additionalProperties": false
        }),
    )
}

/// `github.get_branch` — get branch metadata.
pub fn tool_def_get_branch() -> ToolDef {
    github_read_tool(
        GITHUB_GET_BRANCH,
        "Get branch metadata.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "owner": {"type": "string"},
                "repo": {"type": "string"},
                "branch": {"type": "string"}
            },
            "required": ["owner", "repo", "branch"],
            "additionalProperties": false
        }),
    )
}

/// `github.compare_branches` — compare two branches.
pub fn tool_def_compare_branches() -> ToolDef {
    github_read_tool(
        GITHUB_COMPARE_BRANCHES,
        "Compare two branches.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "owner": {"type": "string"},
                "repo": {"type": "string"},
                "base": {"type": "string"},
                "head": {"type": "string"}
            },
            "required": ["owner", "repo", "base", "head"],
            "additionalProperties": false
        }),
    )
}

/// `github.get_actions_status` — get the combined GitHub Actions status for a ref.
pub fn tool_def_get_actions_status() -> ToolDef {
    github_read_tool(
        GITHUB_GET_ACTIONS_STATUS,
        "Get the combined GitHub Actions status for a ref.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "owner": {"type": "string"},
                "repo": {"type": "string"},
                "ref": {"type": "string"}
            },
            "required": ["owner", "repo", "ref"],
            "additionalProperties": false
        }),
    )
}

// ─── Collection helpers ───────────────────────────────────────────────────────

/// All GitHub MCP tool definitions in [`GITHUB_TOOL_NAMES`] order.
pub fn all_tool_defs() -> Vec<ToolDef> {
    vec![
        tool_def_list_prs(),
        tool_def_get_pr(),
        tool_def_create_pr(),
        tool_def_comment_pr(),
        tool_def_review_pr(),
        tool_def_merge_pr(),
        tool_def_list_issues(),
        tool_def_create_issue(),
        tool_def_comment_issue(),
        tool_def_close_issue(),
        tool_def_add_labels(),
        tool_def_create_label(),
        tool_def_get_file(),
        tool_def_search_code(),
        tool_def_list_commits(),
        tool_def_create_branch(),
        tool_def_get_branch(),
        tool_def_compare_branches(),
        tool_def_get_actions_status(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::tool::ToolSource;

    #[test]
    fn github_tool_count_matches_constant() {
        assert_eq!(all_tool_defs().len(), GITHUB_TOOL_COUNT);
        assert_eq!(GITHUB_TOOL_NAMES.len(), GITHUB_TOOL_COUNT);
    }

    #[test]
    fn github_tool_names_match_definitions() {
        for (def, name) in all_tool_defs().iter().zip(GITHUB_TOOL_NAMES.iter()) {
            assert_eq!(def.name, *name, "name mismatch");
        }
    }

    #[test]
    fn all_github_tools_are_mcp_backed() {
        for def in all_tool_defs() {
            assert!(
                matches!(&def.source, ToolSource::Mcp { server } if server == MCP_SERVER),
                "tool {} must have source = Mcp {{ server: {:?} }}, got {:?}",
                def.name,
                MCP_SERVER,
                def.source
            );
        }
    }

    #[test]
    fn all_github_tools_have_mcp_category() {
        for def in all_tool_defs() {
            assert_eq!(
                def.category,
                ToolCategory::Mcp,
                "tool {} must be ToolCategory::Mcp",
                def.name
            );
        }
    }

    #[test]
    fn github_tool_write_permission_classification() {
        // Write tools must require write permission.
        let write_tools = [
            GITHUB_CREATE_PR,
            GITHUB_COMMENT_PR,
            GITHUB_REVIEW_PR,
            GITHUB_MERGE_PR,
            GITHUB_CREATE_ISSUE,
            GITHUB_COMMENT_ISSUE,
            GITHUB_CLOSE_ISSUE,
            GITHUB_ADD_LABELS,
            GITHUB_CREATE_LABEL,
            GITHUB_CREATE_BRANCH,
        ];
        // Read-only tools must NOT require write permission.
        let read_tools = [
            GITHUB_LIST_PRS,
            GITHUB_GET_PR,
            GITHUB_LIST_ISSUES,
            GITHUB_GET_FILE,
            GITHUB_SEARCH_CODE,
            GITHUB_LIST_COMMITS,
            GITHUB_GET_BRANCH,
            GITHUB_COMPARE_BRANCHES,
            GITHUB_GET_ACTIONS_STATUS,
        ];

        let defs: std::collections::HashMap<&str, ToolDef> = all_tool_defs()
            .into_iter()
            .map(|d| (Box::leak(d.name.clone().into_boxed_str()) as &str, d))
            .collect();

        for name in write_tools {
            let def = defs
                .get(name)
                .unwrap_or_else(|| panic!("missing def for {name}"));
            assert!(
                def.permission.write,
                "write tool {name} must have permission.write = true"
            );
        }
        for name in read_tools {
            let def = defs
                .get(name)
                .unwrap_or_else(|| panic!("missing def for {name}"));
            assert!(
                !def.permission.write,
                "read tool {name} must have permission.write = false"
            );
        }
    }

    #[test]
    fn github_tool_catalog_no_duplicates() {
        let names: Vec<_> = all_tool_defs().iter().map(|d| d.name.clone()).collect();
        let mut seen = std::collections::HashSet::new();
        for name in &names {
            assert!(seen.insert(name.as_str()), "duplicate tool name: {name}");
        }
    }
}
