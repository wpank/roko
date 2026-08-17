//! String constants for externally sourced signal kinds.

pub const GITHUB_PUSH: &str = "github:push";
pub const GITHUB_PR_OPENED: &str = "github:pull_request:opened";
pub const GITHUB_PR_REVIEW: &str = "github:pull_request_review";
pub const GITHUB_ISSUE_OPENED: &str = "github:issues:opened";
/// A GitHub `issues` event with `action=closed`.
pub const GITHUB_ISSUE_CLOSED: &str = "github:issues:closed";
/// A GitHub `issues` event with `action=reopened`.
pub const GITHUB_ISSUE_REOPENED: &str = "github:issues:reopened";
/// A GitHub `issues` event with `action=labeled`.
pub const GITHUB_ISSUE_LABELED: &str = "github:issues:labeled";
/// A GitHub `issues` event with `action=assigned`.
pub const GITHUB_ISSUE_ASSIGNED: &str = "github:issues:assigned";
/// A GitHub `issue_comment` event (new comment on an issue or PR).
pub const GITHUB_ISSUE_COMMENTED: &str = "github:issue_comment:created";
/// A GitHub Actions `workflow_run` event (any action).
pub const GITHUB_WORKFLOW_RUN: &str = "github:workflow_run";
/// A GitHub Actions `workflow_run` event with `action=completed`.
pub const GITHUB_WORKFLOW_RUN_COMPLETED: &str = "github:workflow_run:completed";
/// A GitHub `check_suite` event (any action).
pub const GITHUB_CHECK_SUITE: &str = "github:check_suite";
/// A completed GitHub check event. GitHub currently delivers this through the
/// `check_suite` webhook, but consumers should depend on this canonical name.
pub const GITHUB_CHECK_COMPLETED: &str = "github:check_suite:completed";
/// Backward-compatible alias for consumers that named the GitHub transport.
pub const GITHUB_CHECK_SUITE_COMPLETED: &str = GITHUB_CHECK_COMPLETED;
/// A validated GitHub issue requested execution of a referenced plan.
pub const GITHUB_PLAN_EXECUTION_REQUESTED: &str = "github:plan:execution_requested";
/// A validated GitHub PR review requested replanning of its plan branch.
pub const GITHUB_REPLAN_REQUESTED: &str = "github:plan:replan_requested";
/// A GitHub `create` event (branch or tag created).
pub const GITHUB_CREATE: &str = "github:create";
/// A GitHub `delete` event (branch or tag deleted).
pub const GITHUB_DELETE: &str = "github:delete";
/// A GitHub `release` event (any action).
pub const GITHUB_RELEASE: &str = "github:release";
/// A GitHub `release` event with `action=published`.
pub const GITHUB_RELEASE_PUBLISHED: &str = "github:release:published";
/// A GitHub `deployment` event (any action).
pub const GITHUB_DEPLOYMENT: &str = "github:deployment";
/// A GitHub `deployment_status` event for a deployment that is pending.
pub const GITHUB_DEPLOYMENT_PENDING: &str = "github:deployment_status:pending";
/// A GitHub `deployment_status` event for a deployment that is in progress.
pub const GITHUB_DEPLOYMENT_IN_PROGRESS: &str = "github:deployment_status:in_progress";
/// A GitHub `deployment_status` event for a deployment that succeeded.
pub const GITHUB_DEPLOYMENT_SUCCESS: &str = "github:deployment_status:success";
/// A GitHub `deployment_status` event for a deployment that failed.
pub const GITHUB_DEPLOYMENT_FAILURE: &str = "github:deployment_status:failure";
/// A GitHub `deployment_status` event for a deployment that encountered an error.
pub const GITHUB_DEPLOYMENT_ERROR: &str = "github:deployment_status:error";
/// A GitHub `deployment_status` event for a deployment that became inactive.
pub const GITHUB_DEPLOYMENT_INACTIVE: &str = "github:deployment_status:inactive";
pub const GITHUB_PR_MERGED: &str = "github:pull_request:merged";
pub const GITHUB_PR_CLOSED: &str = "github:pull_request:closed";
pub const GITHUB_CI_FAILED: &str = "github:ci:failed";
pub const PRD_PLAN_APPROVED: &str = "prd.plan_approved";
pub const SLACK_MESSAGE: &str = "slack:message";
pub const SLACK_REACTION: &str = "slack:reaction_added";
pub const CRON_TICK: &str = "scheduler:cron";
pub const FS_CHANGED: &str = "fswatcher:changed";
pub const FS_CREATED: &str = "fswatcher:created";
pub const FS_MODIFIED: &str = "fswatcher:modified";
pub const FS_DELETED: &str = "fswatcher:deleted";
pub const FS_RENAMED: &str = "fswatcher:renamed";
pub const MANUAL_TRIGGER: &str = "manual:trigger";
