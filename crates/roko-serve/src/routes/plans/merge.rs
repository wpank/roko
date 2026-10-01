//! Task reviews and diffs: list and record human reviews, and show and merge a
//! task's changes.

use super::*;

// ── Review workflow ──────────────────────────────────────────────────

/// `GET /api/plans/:id/reviews` — list tasks pending review.
///
/// Scans plan tasks that are completed (by an agent) and checks for
/// corresponding agent branches.  Returns gate results from the snapshot
/// and diff summaries from git.
pub(super) async fn list_reviews(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let plan = resolve_plan(&state, &id).await?;

    // Pull gate results for this plan from the snapshot.
    let snapshot = state.state_hub.current_snapshot();
    let plan_gates: Vec<_> = snapshot.gates.iter().filter(|g| g.plan_id == id).collect();

    let mut reviews = Vec::new();

    for task in &plan.tasks {
        // A Graph attempt held for approval (gap-0d64d5).
        if let Some(hold) = graph_review_hold(&state.workdir, &id, &task.id) {
            let files = parse_diff_output(
                hold["numstat"].as_str().unwrap_or_default(),
                hold["patch"].as_str().unwrap_or_default(),
            );
            reviews.push(json!({
                "task_id": task.id,
                "description": task.description,
                "status": "awaiting_approval",
                "attempt_key": hold["attempt_key"],
                "branch": hold["branch"],
                "diff_summary": format!(
                    "{} files changed, {} insertions(+), {} deletions(-)",
                    files.len(),
                    files.iter().map(|f| f.additions).sum::<u32>(),
                    files.iter().map(|f| f.deletions).sum::<u32>()
                ),
                "gate_results": Vec::<Value>::new(),
                "files_changed": files.iter().map(|f| f.path.clone()).collect::<Vec<_>>(),
            }));
            continue;
        }

        // Detect agent branches: convention is `agent/<name>/<task_id>`.
        let branch = find_agent_branch(&state.workdir, &task.id).await;

        // Gather gate results for this task.
        let task_gates: Vec<Value> = plan_gates
            .iter()
            .filter(|g| g.task_id == task.id)
            .map(|g| {
                json!({
                    "gate": g.gate,
                    "passed": g.passed,
                })
            })
            .collect();

        // Compute diff summary if branch exists.
        let (diff_summary, files_changed) = if let Some(ref branch_name) = branch {
            diff_summary(&state.workdir, branch_name).await
        } else {
            (String::new(), Vec::new())
        };

        // Determine review status.
        let status = if branch.is_some() && !files_changed.is_empty() {
            "pending_review"
        } else if task.completed {
            "completed"
        } else {
            "pending"
        };

        reviews.push(json!({
            "task_id": task.id,
            "description": task.description,
            "status": status,
            "branch": branch,
            "diff_summary": diff_summary,
            "gate_results": task_gates,
            "files_changed": files_changed,
        }));
    }

    Ok(Json(json!({
        "plan_id": id,
        "reviews": reviews,
    })))
}

#[derive(Deserialize, Validate)]
pub(super) struct ReviewDecision {
    #[validate(custom(function = "validate_decision"))]
    pub(super) decision: String,
    #[serde(default)]
    pub(super) comment: String,
}

pub(super) fn validate_decision(decision: &str) -> Result<(), validator::ValidationError> {
    match decision {
        "approve" | "reject" | "skip" => Ok(()),
        _ => {
            let mut err = validator::ValidationError::new("invalid_decision");
            err.message = Some("decision must be approve, reject, or skip".into());
            Err(err)
        }
    }
}

impl RequestPayload for ReviewDecision {
    fn validate_payload(&self) -> Result<(), ApiError> {
        validate_with_validator(self)
    }
}

/// `POST /api/plans/:id/tasks/:task_id/review` — approve, reject, or skip.
///
/// - **approve**: merge the agent branch into the base branch, mark approved.
/// - **reject**: record rejection with comment, keep branch for rework.
/// - **skip**: mark task as skipped, no merge.
pub(super) async fn submit_review(
    State(state): State<Arc<AppState>>,
    Path((id, task_id)): Path<(String, String)>,
    ValidJson(body): ValidJson<ReviewDecision>,
) -> Result<Json<Value>, ApiError> {
    validate_path_segment(&id, "plan id")?;
    validate_path_segment(&task_id, "task id")?;

    // Verify plan and task exist.
    let plan = resolve_plan(&state, &id).await?;
    if !plan.tasks.iter().any(|t| t.id == task_id) {
        return Err(ApiError::not_found(format!(
            "task '{task_id}' not found in plan '{id}'"
        )));
    }

    // A held Graph attempt (gap-0d64d5): the plan run waiting on it reads
    // the decision and merges or fails the attempt itself.
    if let Some(hold) = graph_review_hold(&state.workdir, &id, &task_id) {
        let decision = match body.decision.as_str() {
            "approve" => "approved",
            "reject" => "rejected",
            _ => "skipped",
        };
        let attempt_key = hold["attempt_key"].as_str().unwrap_or_default();
        record_review(
            &state.workdir,
            &id,
            &task_id,
            decision,
            &body.comment,
            Some(attempt_key),
        )
        .await;
        return Ok(Json(json!({
            "task_id": task_id,
            "status": decision,
            "attempt_key": attempt_key,
            "held": true,
        })));
    }

    let branch = find_agent_branch(&state.workdir, &task_id).await;

    let result = match body.decision.as_str() {
        "approve" => {
            let merged = if let Some(ref branch_name) = branch {
                merge_branch(&state.workdir, branch_name).await
            } else {
                false
            };

            // Record the review in the state directory.
            record_review(
                &state.workdir,
                &id,
                &task_id,
                "approved",
                &body.comment,
                None,
            )
            .await;

            json!({
                "task_id": task_id,
                "status": "approved",
                "merged": merged,
                "branch": branch,
            })
        }
        "reject" => {
            // Send feedback to the agent if possible.
            if let Some(ref branch_name) = branch {
                // Extract agent name from branch: `agent/<name>/<task_id>`
                let agent_name = branch_name
                    .strip_prefix("agent/")
                    .and_then(|s| s.split('/').next())
                    .unwrap_or("");

                if !agent_name.is_empty() {
                    let feedback = format!(
                        "Review rejected for task {task_id}: {}",
                        if body.comment.is_empty() {
                            "No comment provided"
                        } else {
                            &body.comment
                        }
                    );
                    // Publish as an event so agents can pick it up.
                    state.event_bus.publish(ServerEvent::Error {
                        message: format!(
                            "review:reject agent={agent_name} task={task_id}: {feedback}"
                        ),
                    });
                }
            }

            record_review(
                &state.workdir,
                &id,
                &task_id,
                "rejected",
                &body.comment,
                None,
            )
            .await;

            json!({
                "task_id": task_id,
                "status": "needs_rework",
                "branch": branch,
                "comment": body.comment,
            })
        }
        "skip" => {
            record_review(
                &state.workdir,
                &id,
                &task_id,
                "skipped",
                &body.comment,
                None,
            )
            .await;
            json!({ "task_id": task_id, "status": "skipped" })
        }
        _ => unreachable!("validated above"),
    };

    Ok(Json(result))
}

/// `GET /api/plans/:id/tasks/:task_id/diff` — structured diff for a task.
///
/// A Graph attempt held for review (gap-0d64d5) returns the change it waits
/// with; otherwise a Graph run's recorded result: the task's accepted
/// attempt commit against its parent. Without either, the legacy agent
/// branch against main. Returns per-file diff entries with path, status,
/// additions, deletions, and unified patch.
pub(super) async fn task_diff(
    State(state): State<Arc<AppState>>,
    Path((id, task_id)): Path<(String, String)>,
) -> Result<Json<Value>, ApiError> {
    validate_path_segment(&id, "plan id")?;
    validate_path_segment(&task_id, "task id")?;

    // Verify plan and task exist.
    let plan = resolve_plan(&state, &id).await?;
    if !plan.tasks.iter().any(|t| t.id == task_id) {
        return Err(ApiError::not_found(format!(
            "task '{task_id}' not found in plan '{id}'"
        )));
    }

    if let Some(hold) = graph_review_hold(&state.workdir, &id, &task_id) {
        let files = parse_diff_output(
            hold["numstat"].as_str().unwrap_or_default(),
            hold["patch"].as_str().unwrap_or_default(),
        );
        return Ok(Json(diff_response(
            &task_id,
            &files,
            json!({
                "source": "review_hold",
                "status": "awaiting_approval",
                "attempt_key": hold["attempt_key"],
                "branch": hold["branch"],
                "base": hold["base"],
            }),
        )));
    }

    if let Some((commit, plan_branch)) = graph_task_commit(&state.workdir, &id, &task_id) {
        let base = format!("{commit}^");
        let files = git_range_diff(&state.workdir, &[&base, &commit]).await?;
        return Ok(Json(diff_response(
            &task_id,
            &files,
            json!({
                "source": "graph",
                "branch": plan_branch,
                "base": base,
                "commit": commit,
            }),
        )));
    }

    let branch = find_agent_branch(&state.workdir, &task_id)
        .await
        .ok_or_else(|| {
            ApiError::not_found(format!(
                "no recorded result or agent branch found for task '{task_id}'"
            ))
        })?;

    let files = parse_git_diff(&state.workdir, &branch).await?;

    Ok(Json(diff_response(
        &task_id,
        &files,
        json!({ "branch": branch, "base": "main" }),
    )))
}

// ── Review helpers ──────────────────────────────────────────────────

/// Find an agent branch matching the task ID.
///
/// Convention: `agent/<agent-name>/<task_id>`.
pub(super) async fn find_agent_branch(workdir: &std::path::Path, task_id: &str) -> Option<String> {
    let output = tokio::process::Command::new("git")
        .args(["branch", "--list", &format!("agent/*/{task_id}")])
        .current_dir(workdir)
        .output()
        .await
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    // git branch --list returns "  branch-name\n" or "* branch-name\n"
    stdout
        .lines()
        .map(|l| l.trim_start_matches(['*', ' '].as_ref()).trim().to_string())
        .find(|l| !l.is_empty())
}

/// Compute a short diff summary ("+N -M across K files") for a branch.
pub(super) async fn diff_summary(workdir: &std::path::Path, branch: &str) -> (String, Vec<String>) {
    let output = tokio::process::Command::new("git")
        .args(["diff", "--stat", &format!("main...{branch}")])
        .current_dir(workdir)
        .output()
        .await;

    let output = match output {
        Ok(o) => o,
        Err(_) => return (String::new(), Vec::new()),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    // Last line is summary: " N files changed, M insertions(+), K deletions(-)"
    let summary = lines
        .last()
        .map(|l| l.trim().to_string())
        .unwrap_or_default();

    // Each preceding line is " path/to/file | N ++--"
    let files: Vec<String> = lines
        .iter()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.contains('|') {
                Some(trimmed.split('|').next().unwrap_or("").trim().to_string())
            } else {
                None
            }
        })
        .collect();

    (summary, files)
}

pub(super) struct DiffFile {
    pub(super) path: String,
    pub(super) status: String,
    pub(super) additions: u32,
    pub(super) deletions: u32,
    pub(super) patch: String,
}

/// Parse `git diff --numstat` + `git diff` of `main...{branch}` into
/// structured per-file entries.
pub(super) async fn parse_git_diff(
    workdir: &std::path::Path,
    branch: &str,
) -> Result<Vec<DiffFile>, ApiError> {
    git_range_diff(workdir, &[&format!("main...{branch}")]).await
}

/// `git diff --numstat` and `git diff` of `range` (read-only) in `workdir`,
/// as structured per-file entries.
pub(super) async fn git_range_diff(
    workdir: &std::path::Path,
    range: &[&str],
) -> Result<Vec<DiffFile>, ApiError> {
    // Get numstat for additions/deletions counts.
    let numstat = tokio::process::Command::new("git")
        .args(["diff", "--no-color", "--no-ext-diff", "--numstat"])
        .args(range)
        .current_dir(workdir)
        .output()
        .await
        .map_err(|e| ApiError::internal(format!("git diff --numstat: {e}")))?;

    // Get the full diff for patches.
    let full_diff = tokio::process::Command::new("git")
        .args(["diff", "--no-color", "--no-ext-diff"])
        .args(range)
        .current_dir(workdir)
        .output()
        .await
        .map_err(|e| ApiError::internal(format!("git diff: {e}")))?;

    Ok(parse_diff_output(
        &String::from_utf8_lossy(&numstat.stdout),
        &String::from_utf8_lossy(&full_diff.stdout),
    ))
}

/// Per-file entries from `git diff --numstat` output and the matching
/// unified diff.
pub(super) fn parse_diff_output(numstat_str: &str, full_diff_str: &str) -> Vec<DiffFile> {
    // Parse per-file patches from the full diff.
    let mut file_patches: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    let mut current_file = String::new();
    let mut current_patch = String::new();

    for line in full_diff_str.lines() {
        if line.starts_with("diff --git") {
            if !current_file.is_empty() {
                file_patches.insert(current_file.clone(), current_patch.clone());
            }
            // Extract filename from "diff --git a/path b/path"
            current_file = line.split(" b/").nth(1).unwrap_or("").to_string();
            current_patch = String::new();
        }
        current_patch.push_str(line);
        current_patch.push('\n');
    }
    if !current_file.is_empty() {
        file_patches.insert(current_file, current_patch);
    }

    // Parse numstat lines: "additions\tdeletions\tpath"
    let mut files = Vec::new();
    for line in numstat_str.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 3 {
            continue;
        }
        let additions = parts[0].parse::<u32>().unwrap_or(0);
        let deletions = parts[1].parse::<u32>().unwrap_or(0);
        let path = parts[2].to_string();

        let status = if additions > 0 && deletions == 0 {
            "added"
        } else if additions == 0 && deletions > 0 {
            "deleted"
        } else {
            "modified"
        };

        let patch = file_patches.get(&path).cloned().unwrap_or_default();

        files.push(DiffFile {
            path,
            status: status.to_string(),
            additions,
            deletions,
            patch,
        });
    }

    files
}

/// The review hold of `task_id` in `plan_id`: a verified Graph attempt
/// waiting for approval before it is accepted, with what it changed
/// (gap-0d64d5). The plan run writes it and removes it once decided.
pub(super) fn graph_review_hold(
    workdir: &std::path::Path,
    plan_id: &str,
    task_id: &str,
) -> Option<Value> {
    let path = roko_fs::RokoLayout::for_project(workdir).review_hold(plan_id, task_id);
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

/// The commit a Graph run recorded for `task_id`'s last accepted attempt
/// in `plan_id`, and the plan branch it went onto: the
/// `workspace.attempt_commit` and `workspace.plan_branch` tags of the task's
/// last output in the plan's activity log
/// (`.roko/state/graph/<plan>/activities.jsonl`).
pub(super) fn graph_task_commit(
    workdir: &std::path::Path,
    plan_id: &str,
    task_id: &str,
) -> Option<(String, Option<String>)> {
    let safe_plan: String = plan_id
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect();
    let activities = roko_fs::RokoLayout::for_project(workdir)
        .state_dir()
        .join("graph")
        .join(safe_plan)
        .join("activities.jsonl");
    let log = std::fs::read_to_string(activities).ok()?;
    log.lines()
        .rev()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .flat_map(|entry| match entry.get("signals") {
            Some(Value::Array(signals)) => signals.clone(),
            _ => Vec::new(),
        })
        .find_map(|signal| {
            let tags = signal.get("tags")?;
            if tags.get("task_id")?.as_str()? != task_id {
                return None;
            }
            let commit = tags.get("workspace.attempt_commit")?.as_str()?;
            let is_commit = commit.len() >= 40 && commit.bytes().all(|b| b.is_ascii_hexdigit());
            is_commit.then(|| {
                let branch = tags
                    .get("workspace.plan_branch")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned);
                (commit.to_string(), branch)
            })
        })
}

/// A task diff response for `files`.
pub(super) fn diff_response(task_id: &str, files: &[DiffFile], extra: Value) -> Value {
    let mut response = json!({
        "task_id": task_id,
        "file_count": files.len(),
        "total_additions": files.iter().map(|f| f.additions).sum::<u32>(),
        "total_deletions": files.iter().map(|f| f.deletions).sum::<u32>(),
        "files": files.iter().map(|f| json!({
            "path": f.path,
            "status": f.status,
            "additions": f.additions,
            "deletions": f.deletions,
            "patch": f.patch,
        })).collect::<Vec<_>>(),
    });
    if let (Some(response), Value::Object(extra)) = (response.as_object_mut(), extra) {
        response.extend(extra);
    }
    response
}

/// Merge an agent branch into the current branch.
pub(super) async fn merge_branch(workdir: &std::path::Path, branch: &str) -> bool {
    let output = tokio::process::Command::new("git")
        .args([
            "merge",
            "--no-ff",
            "-m",
            &format!("Merge {branch} (approved via review)"),
            branch,
        ])
        .current_dir(workdir)
        .output()
        .await;

    matches!(output, Ok(o) if o.status.success())
}

/// Record a review decision to `.roko/state/reviews.jsonl`, naming the held
/// attempt it decides when there is one (gap-0d64d5).
pub(super) async fn record_review(
    workdir: &std::path::Path,
    plan_id: &str,
    task_id: &str,
    decision: &str,
    comment: &str,
    attempt_key: Option<&str>,
) {
    let reviews_path = roko_fs::RokoLayout::for_project(workdir).reviews_log();
    if let Err(err) = tokio::fs::create_dir_all(reviews_path.parent().unwrap_or(workdir)).await {
        tracing::warn!(path = %reviews_path.display(), error = %err, "failed to create reviews state directory");
        return;
    }

    let mut entry = serde_json::json!({
        "plan_id": plan_id,
        "task_id": task_id,
        "decision": decision,
        "comment": comment,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });
    if let Some(attempt_key) = attempt_key {
        entry["attempt_key"] = Value::from(attempt_key);
    }

    let mut line = serde_json::to_string(&entry).unwrap_or_default();
    line.push('\n');

    // Append atomically.
    match tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&reviews_path)
        .await
    {
        Ok(mut f) => {
            use tokio::io::AsyncWriteExt;
            if let Err(err) = f.write_all(line.as_bytes()).await {
                tracing::warn!(path = %reviews_path.display(), error = %err, "failed to write review entry");
            } else if let Err(err) = f.flush().await {
                tracing::warn!(path = %reviews_path.display(), error = %err, "failed to flush review entry");
            }
        }
        Err(err) => {
            tracing::warn!(path = %reviews_path.display(), error = %err, "failed to open reviews file for append");
        }
    }
}
