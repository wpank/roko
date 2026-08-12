//! String constants for externally sourced signal kinds.

pub const GITHUB_PUSH: &str = "github:push";
pub const GITHUB_PR_OPENED: &str = "github:pull_request:opened";
pub const GITHUB_PR_REVIEW: &str = "github:pull_request_review";
pub const GITHUB_ISSUE_OPENED: &str = "github:issues:opened";
/// A GitHub Actions `workflow_run` event (any action).
pub const GITHUB_WORKFLOW_RUN: &str = "github:workflow_run";
/// A GitHub Actions `workflow_run` event with `action=completed`.
pub const GITHUB_WORKFLOW_RUN_COMPLETED: &str = "github:workflow_run:completed";
/// A GitHub `check_suite` event (any action).
pub const GITHUB_CHECK_SUITE: &str = "github:check_suite";
/// A GitHub `check_suite` event with `action=completed`.
pub const GITHUB_CHECK_SUITE_COMPLETED: &str = "github:check_suite:completed";
/// A GitHub `create` event (branch or tag created).
pub const GITHUB_CREATE: &str = "github:create";
/// A GitHub `delete` event (branch or tag deleted).
pub const GITHUB_DELETE: &str = "github:delete";
/// A GitHub `release` event (any action).
pub const GITHUB_RELEASE: &str = "github:release";
/// A GitHub `release` event with `action=published`.
pub const GITHUB_RELEASE_PUBLISHED: &str = "github:release:published";
pub const PRD_PLAN_APPROVED: &str = "prd.plan_approved";
pub const SLACK_MESSAGE: &str = "slack:message";
pub const SLACK_REACTION: &str = "slack:reaction_added";
pub const CRON_TICK: &str = "scheduler:cron";
pub const FS_CHANGED: &str = "fswatcher:changed";
pub const FS_CREATED: &str = "fswatcher:created";
pub const FS_MODIFIED: &str = "fswatcher:modified";
pub const FS_DELETED: &str = "fswatcher:deleted";
pub const MANUAL_TRIGGER: &str = "manual:trigger";
