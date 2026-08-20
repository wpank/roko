# 98 — GitHub Workflow: Idempotent Comments, 401 Detection, and Pre-Merge Check

**Priority**: P2 — reliability; duplicate PR comments and silent auth failures degrade the GitHub integration in production
**Size**: M (2-3 days)
**Crates**: `crates/roko-cli` (github_ops, github_workflow), `crates/roko-mcp-github`
**Depends on**: None

---

## Background

The roko runner posts GitHub comments and attempts PR merges as part of the GitHub workflow integration wired in E46. This integration currently has three independent correctness gaps that each require distinct changes.

**Gap 1 — Duplicate comments on plan restart.** Every task completion and plan summary is posted as a new comment without checking whether a comment for the same task already exists. If a plan is interrupted mid-run and resumed, or if a task is retried, the same gate result gets posted as a second (or third) comment. For long plans with many tasks this creates dozens of duplicate comments on the PR.

**Gap 2 — Silent 401 token expiry.** `GitHubClient` reads `GITHUB_TOKEN` once at construction time and stores it as a `String` that is never updated. Fine-grained PATs expire after a configurable window (default 30 days) and GitHub App installation tokens expire every hour. When the token expires mid-plan, `parse_response` (line 492) returns the same generic `"GitHub API returned 401"` error used for every other 4xx, with no hint that token expiry is the cause and no suggestion to set `GITHUB_TOKEN` again.

**Gap 3 — No pre-merge mergeability check.** `check_ci_then_merge` (line 356 in `github_workflow.rs`) calls `ops.merge_pr()` immediately after CI passes without first checking whether the PR has merge conflicts. GitHub returns 405 Method Not Allowed when the PR is not mergeable (not 409 as previously thought). This error is logged as a generic non-fatal warning with no specific indication that a rebase is required.

## Current State

1. **No idempotency in `comment_pr`.** `crates/roko-cli/src/runner/github_workflow.rs`, line 300 — `ops.comment_pr(plan.pr_number, &body).await` is called directly. Line 323 posts the plan summary the same way. `crates/roko-cli/src/github_ops_impl.rs`, lines 161-173 — the `LiveGitHubOps::comment_pr` implementation calls `client.comment_pr()` with no preceding list-then-match logic. `crates/roko-cli/src/github_ops.rs`, line 48 — the `GitHubOps` trait exposes only `comment_pr(pr_number, body)` with no `upsert` or `list` variant.

2. **`GitHubClient` has no `list_pr_comments` or `update_comment` methods.** `crates/roko-mcp-github/src/lib.rs` — the full set of `pub fn` methods is: `new`, `from_env`, `with_api_base`, `create_branch`, `create_pr`, `merge_pr`, `merge_pr_with_title`, `comment_pr`, `review_pr`, `list_prs`, `list_prs_with_filters`, `authenticated_user`, `create_issue`, `create_issue_with_assignees`, `close_issue`, `close_issue_with_reason`, `list_issues`, `add_labels`, `remove_label`, `get_actions_status`. No comment listing or editing methods exist.

3. **`parse_response` treats 401 identically to all other 4xx.** `crates/roko-mcp-github/src/lib.rs`, lines 487-500 — the `if !status.is_success()` branch at line 492 formats a single generic error string regardless of which error code was returned. Rate-limit retries are handled separately in `send_request` (line 452) only for 429 responses.

4. **`GitHubClient` stores token as immutable `String`.** Lines 54-59:
   ```rust
   pub struct GitHubClient {
       client: Client,
       token: String,    // ← never updated
       api_base: String,
   }
   ```
   No refresh mechanism exists and none is planned in scope (GitHub App token auto-refresh requires an App credential flow that is out of scope here).

5. **Merge is attempted without a mergeability check.** `crates/roko-cli/src/runner/github_workflow.rs`, lines 386-393:
   ```rust
   Ok(CiStatus::Success) => {
       if let Err(error) = ops
           .merge_pr(plan.pr_number, config.merge_method.as_str())
           .await
       {
           warn!(%plan_id, pr_number = plan.pr_number, %error,
               "GitHub PR merge failed after CI passed (non-fatal)");
       }
   ```
   `GitHubClient::merge_pr` calls `PUT /repos/{owner}/{repo}/pulls/{number}/merge` immediately. `GitHubClient` has no `get_pr` method to query the `mergeable` or `mergeable_state` fields first.

6. **`GitHubOps` trait must be extended without breaking the no-op adapter.** `crates/roko-cli/src/github_ops.rs`, lines 22-90 — `GitHubOps` is an `async_trait`. Adding new methods requires providing default implementations in `NoOpGitHubOps` at lines 56-90, otherwise the no-op adapter fails to compile.

## Implementation Plan

### Step 1: Add `list_pr_comments` and `update_comment` to `GitHubClient`

In `crates/roko-mcp-github/src/lib.rs`, add two new methods to `impl GitHubClient` after `comment_pr` (line 196):

```rust
/// List all issue/PR comments for a given issue or PR number.
/// GitHub's issues comment API is shared with PRs.
pub fn list_pr_comments(&self, owner: &str, repo: &str, number: u64) -> Result<Value> {
    let url = format!(
        "{}/repos/{owner}/{repo}/issues/{number}/comments",
        self.api_base
    );
    self.get_json(&url, &[("per_page", "100".to_string())], "list PR comments")
}

/// Edit an existing issue/PR comment body.
pub fn update_comment(&self, owner: &str, repo: &str, comment_id: u64, body: &str) -> Result<Value> {
    let url = format!(
        "{}/repos/{owner}/{repo}/issues/comments/{comment_id}",
        self.api_base
    );
    let payload = serde_json::json!({ "body": body });
    self.patch_json(&url, &payload, "update PR comment")
}

/// Get a pull request by number.
pub fn get_pr(&self, owner: &str, repo: &str, number: u64) -> Result<Value> {
    let url = format!(
        "{}/repos/{owner}/{repo}/pulls/{number}",
        self.api_base
    );
    self.get_json(&url, &[], "get pull request")
}
```

### Step 2: Add `upsert_pr_comment` and `merge_pr_if_mergeable` to `GitHubOps` trait

In `crates/roko-cli/src/github_ops.rs`, add two new methods to the `GitHubOps` trait with no-op default implementations so `NoOpGitHubOps` does not need to be changed:

```rust
/// Post or update a PR comment that starts with `marker`.
/// If a comment from this run already starts with `marker`, update it in-place.
/// Otherwise, create a new comment.
async fn upsert_pr_comment(&self, pr_number: u64, marker: &str, body: &str) -> Result<(), String> {
    // Default: just post a new comment (no idempotency). Callers that want
    // idempotency should use the live adapter which overrides this.
    self.comment_pr(pr_number, body).await
}

/// Check PR mergeability before merging.
async fn merge_pr_if_mergeable(&self, pr_number: u64, method: &str) -> Result<(), String> {
    // Default: attempt merge unconditionally (existing behaviour).
    self.merge_pr(pr_number, method).await
}
```

### Step 3: Implement `upsert_pr_comment` in `LiveGitHubOps`

In `crates/roko-cli/src/github_ops_impl.rs`, add the implementation after `comment_pr` (line 173):

```rust
async fn upsert_pr_comment(
    &self,
    pr_number: u64,
    marker: &str,
    body: &str,
) -> Result<(), String> {
    let client = Arc::clone(&self.client);
    let owner = self.owner.clone();
    let repo = self.repo.clone();
    let marker = marker.to_string();
    let body = body.to_string();

    // List existing comments and find one that starts with our marker.
    let existing_id = Self::blocking("list PR comments for upsert", {
        let client = Arc::clone(&client);
        let owner = owner.clone();
        let repo = repo.clone();
        let marker = marker.clone();
        move || -> Result<Option<u64>, String> {
            let comments = client
                .list_pr_comments(&owner, &repo, pr_number)
                .map_err(|e| e.to_string())?;
            let id = comments
                .as_array()
                .and_then(|arr| {
                    arr.iter().find(|c| {
                        c["body"].as_str().map(|b| b.starts_with(&marker)).unwrap_or(false)
                    })
                })
                .and_then(|c| c["id"].as_u64());
            Ok(id)
        }
    })
    .await?;

    if let Some(comment_id) = existing_id {
        Self::blocking("update PR comment", move || {
            client
                .update_comment(&owner, &repo, comment_id, &body)
                .map(|_| ())
                .map_err(|e| e.to_string())
        })
        .await
    } else {
        Self::blocking("post PR comment", move || {
            client
                .comment_pr(&owner, &repo, pr_number, &body)
                .map(|_| ())
                .map_err(|e| e.to_string())
        })
        .await
    }
}
```

### Step 4: Implement `merge_pr_if_mergeable` in `LiveGitHubOps`

```rust
async fn merge_pr_if_mergeable(&self, pr_number: u64, method: &str) -> Result<(), String> {
    let client = Arc::clone(&self.client);
    let owner = self.owner.clone();
    let repo = self.repo.clone();
    let method = if method.trim().is_empty() {
        self.config.merge_method.as_str().to_string()
    } else {
        method.to_string()
    };

    // Check mergeability before attempting.
    let mergeable_state = Self::blocking("check PR mergeability", {
        let client = Arc::clone(&client);
        let owner = owner.clone();
        let repo = repo.clone();
        move || -> Result<Option<String>, String> {
            let pr = client.get_pr(&owner, &repo, pr_number).map_err(|e| e.to_string())?;
            Ok(pr["mergeable_state"].as_str().map(str::to_string))
        }
    })
    .await?;

    match mergeable_state.as_deref() {
        Some("clean") | None => {
            // Either clean, or GitHub hasn't computed it yet — attempt merge.
            Self::blocking("merge GitHub pull request", move || {
                client
                    .merge_pr(&owner, &repo, pr_number, &method)
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            })
            .await
        }
        Some("dirty") => Err(format!(
            "PR #{pr_number} has merge conflicts; rebase the branch before merging"
        )),
        Some("blocked") => Err(format!(
            "PR #{pr_number} is blocked by branch protection rules"
        )),
        Some(state) => Err(format!(
            "PR #{pr_number} is not mergeable (state: {state})"
        )),
    }
}
```

### Step 5: Add 401-specific error in `parse_response`

In `crates/roko-mcp-github/src/lib.rs`, modify `parse_response` (lines 487-500) to detect 401 before the generic error:

```rust
fn parse_response(&self, resp: reqwest::blocking::Response, context: &str) -> Result<Value> {
    let status = resp.status();
    let text = resp
        .text()
        .map_err(|e| api_err(format!("read GitHub response ({context}): {e}")))?;
    if status == StatusCode::UNAUTHORIZED {
        return Err(api_err(format!(
            "GitHub API returned 401 Unauthorized ({context}): token may have expired \
             or been revoked. Re-set GITHUB_TOKEN and restart. ({})",
            text.trim()
        )));
    }
    if !status.is_success() {
        return Err(api_err(format!(
            "GitHub API returned {status} ({context}): {}",
            text.trim()
        )));
    }
    serde_json::from_str(&text)
        .map_err(|e| api_err(format!("parse GitHub response ({context}): {e}")))
}
```

### Step 6: Switch callers to use upsert and pre-merge check

In `crates/roko-cli/src/runner/github_workflow.rs`:

- Line 300: replace `ops.comment_pr(plan.pr_number, &body).await` with `ops.upsert_pr_comment(plan.pr_number, &format!("### Task `{}`", result.task_id), &body).await`
- Line 323: replace `ops.comment_pr(plan.pr_number, &body).await` with `ops.upsert_pr_comment(plan.pr_number, "## Roko plan", &body).await`
- Line 388: replace `ops.merge_pr(plan.pr_number, ...).await` with `ops.merge_pr_if_mergeable(plan.pr_number, ...).await` and update the error handling to use `error!()` instead of `warn!()` since merge conflicts are actionable:

```rust
Ok(CiStatus::Success) => {
    if let Err(error) = ops
        .merge_pr_if_mergeable(plan.pr_number, config.merge_method.as_str())
        .await
    {
        error!(%plan_id, pr_number = plan.pr_number, %error,
            "GitHub PR merge failed after CI passed");
    }
    return;
}
```

## Acceptance Criteria

1. When a task gate comment is posted twice for the same `task_id`, the PR ends up with exactly one comment (the second call updates the first).
2. When `GITHUB_TOKEN` has expired, the error message contains the string "token may have expired" or "401 Unauthorized".
3. When a PR has `mergeable_state = "dirty"`, `merge_pr_if_mergeable` returns an error mentioning "merge conflicts" without making a merge API call.
4. `GitHubClient` exposes `list_pr_comments`, `update_comment`, and `get_pr` as public methods.
5. `GitHubOps` trait compiles with `NoOpGitHubOps` using default implementations (no changes to `NoOpGitHubOps`).
6. All existing tests in `crates/roko-cli/tests/` and `crates/roko-mcp-github/` pass.
7. New unit test: `GitHubClient::list_pr_comments` and `update_comment` round-trip against a mock HTTP server (use `mockito` or `wiremock`).
8. New unit test: `parse_response` with a 401 response → error string contains "token may have expired".
9. New integration test: post two `upsert_pr_comment` calls with the same marker → `list_pr_comments` returns one comment.

### Not in Scope

- GitHub App installation token auto-refresh (requires app credential flow)
- Automatic merge conflict resolution via rebase
- PR review requirement checking or branch protection validation

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-mcp-github/src/lib.rs` | Add `list_pr_comments`, `update_comment`, `get_pr` methods; add 401-specific branch in `parse_response` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/github_ops.rs` | Add `upsert_pr_comment` and `merge_pr_if_mergeable` to trait with no-op defaults |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/github_ops_impl.rs` | Implement `upsert_pr_comment` and `merge_pr_if_mergeable` in `LiveGitHubOps` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/github_workflow.rs` | Replace `comment_pr` calls with `upsert_pr_comment`; replace `merge_pr` with `merge_pr_if_mergeable` at line 388 |
