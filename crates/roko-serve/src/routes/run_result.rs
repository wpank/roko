//! Where a finished run's verified work is (gap-8409d0).
//!
//! roko never commits into the operator's checkout. A run whose tasks each ran
//! in their own git worktree has a batch branch, `roko/batch/<run-id>`, started
//! at the operator's `HEAD`, which the run's manifest records as
//! `workspace.base_commit`. Each plan whose tasks all passed is delivered into
//! the batch, and the plan's checkpoint records that delivery under
//! `roko.batch@1`. A promoted run's commit carries the tag `roko/run/<run-id>`.
//! roko-cli (`graph_execution/batch.rs`) writes all of these; this module only
//! reads them, with read-only git plumbing, so that the run summary can tell a
//! chat host what the run changed and how to take it.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use roko_fs::RokoLayout;
use serde_json::{Value, json};

/// roko-cli's `BATCH_BRANCH_PREFIX`.
const BATCH_BRANCH_PREFIX: &str = "roko/batch/";

/// roko-cli's `RUN_TAG_PREFIX`.
const RUN_TAG_PREFIX: &str = "roko/run/";

/// roko-cli's `BATCH_EXTENSION`: a plan checkpoint's record of its delivery
/// into its run's batch.
const BATCH_EXTENSION: &str = "roko.batch@1";

/// How many changed files a result names; its `diffstat` counts them all.
const MAX_FILES: usize = 50;

/// The largest run manifest or plan checkpoint read for a run's base.
const MAX_RECORD_BYTES: u64 = 8 * 1024 * 1024;

/// `.roko/state/graph/<plan>/`, where a Graph run keeps plan `plan_id`'s
/// checkpoint and activity log, named as roko-cli's `graph_checkpoint` names
/// it: characters outside `[A-Za-z0-9._-]` become `_`, and an empty, `.` or
/// `..` id becomes `plan`.
pub(super) fn graph_plan_state_dir(workdir: &Path, plan_id: &str) -> PathBuf {
    let safe: String = plan_id
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect();
    let component = if safe.is_empty() || safe == "." || safe == ".." {
        "plan".to_string()
    } else {
        safe
    };
    RokoLayout::for_project(workdir)
        .state_dir()
        .join("graph")
        .join(component)
}

/// Where run `run_id`'s verified work is in the repository at `workdir`, or
/// `None` when it has no batch branch there: its tasks ran in the shared
/// working tree, it ran in another repository, or the branch was deleted.
///
/// The result names the batch `branch` and its tip `commit`, the `base` the
/// batch started from, and what the batch changes from there: `files_changed`
/// (the first [`MAX_FILES`]) and `diffstat` (`files`, `insertions`,
/// `deletions`). `merged` says whether the checkout's `HEAD` already has that
/// work. A promoted run adds its `tag` and the branch it was `promoted_into`;
/// otherwise, while the checkout lacks the work, `merge_command` is the
/// command that takes it, as `roko plan run` prints it. `plan_ids` are the
/// run's plans, whose checkpoints may record the base.
pub(super) fn run_result(workdir: &Path, run_id: &str, plan_ids: &[String]) -> Option<Value> {
    if run_id.is_empty() {
        return None;
    }
    let branch = format!("{BATCH_BRANCH_PREFIX}{run_id}");
    let commit = rev_parse(workdir, &format!("refs/heads/{branch}"))?;
    let head = rev_parse(workdir, "HEAD");
    let base = recorded_base(workdir, run_id, plan_ids, &commit).or_else(|| {
        let head = head.as_deref()?;
        git_text(workdir, &["merge-base", head, &commit]).filter(|base| is_object_id(base))
    });

    let mut result = json!({ "branch": branch, "commit": commit });
    if let Some(base) = &base {
        result["base"] = json!(base);
        if let Some((files, insertions, deletions)) = changed_files(workdir, base, &commit) {
            result["diffstat"] = json!({
                "files": files.len(),
                "insertions": insertions,
                "deletions": deletions,
            });
            result["files_changed"] = json!(files.into_iter().take(MAX_FILES).collect::<Vec<_>>());
        }
    }
    // A batch still at its base delivered nothing, so there is nothing to take.
    let delivered = base.as_deref() != Some(commit.as_str());
    let head_has_tip = head.is_some() && is_ancestor(workdir, &commit, "HEAD");
    result["merged"] = json!(delivered && head_has_tip);

    let tag = format!("{RUN_TAG_PREFIX}{run_id}");
    if rev_parse(workdir, &format!("refs/tags/{tag}")).is_some() {
        // roko-cli tags a promotion "roko run <id>: <branch> promoted into <target>".
        let subject = git_text(
            workdir,
            &[
                "for-each-ref",
                "--format=%(contents:subject)",
                &format!("refs/tags/{tag}"),
            ],
        );
        if let Some(target) = subject
            .as_deref()
            .and_then(|subject| subject.rsplit_once(" promoted into "))
            .map(|(_, target)| target.trim())
            .filter(|target| !target.is_empty())
        {
            result["promoted_into"] = json!(target);
        }
        result["tag"] = json!(tag);
    } else if delivered && head.is_some() && !head_has_tip {
        // roko-cli's `merge_command`: a fast-forward while the checkout's
        // `HEAD` is behind the batch, a merge once it has moved on.
        let command = if is_ancestor(workdir, "HEAD", &commit) {
            format!("git merge --ff-only {branch}")
        } else {
            format!("git merge {branch}")
        };
        result["merge_command"] = json!(command);
    }
    Some(result)
}

/// The commit run `run_id`'s batch started from, as the run recorded it: its
/// manifest's `workspace.base_commit`, else the `base_oid` of a plan's batch
/// record that names the run. Only a commit the batch's tip `tip` descends
/// from counts.
fn recorded_base(workdir: &Path, run_id: &str, plan_ids: &[String], tip: &str) -> Option<String> {
    let manifest = read_record(
        &RokoLayout::for_project(workdir)
            .run_dir(run_id)
            .join("manifest.json"),
    );
    let mut plans = plan_ids.to_vec();
    if let Some(recorded) = manifest
        .as_ref()
        .and_then(|manifest| manifest["plan_ids"].as_array())
    {
        for plan_id in recorded.iter().filter_map(Value::as_str) {
            if !plans.iter().any(|known| known == plan_id) {
                plans.push(plan_id.to_string());
            }
        }
    }
    let from_manifest = manifest
        .as_ref()
        .and_then(|manifest| manifest["workspace"]["base_commit"].as_str())
        .map(str::to_owned);
    let from_checkpoints = plans.iter().filter_map(|plan_id| {
        let checkpoint =
            read_record(&graph_plan_state_dir(workdir, plan_id).join("checkpoint.json"))?;
        let record = &checkpoint["extensions"][BATCH_EXTENSION]["value"];
        if record["run_id"].as_str() != Some(run_id) {
            return None;
        }
        record["base_oid"].as_str().map(str::to_owned)
    });
    from_manifest
        .into_iter()
        .chain(from_checkpoints)
        .find(|base| is_object_id(base) && is_ancestor(workdir, base, tip))
}

/// The files `tip` changes from `base`, with the lines it inserts and
/// deletes (a binary file counts none), or `None` when git cannot tell.
fn changed_files(workdir: &Path, base: &str, tip: &str) -> Option<(Vec<String>, u64, u64)> {
    let numstat = git(
        workdir,
        &[
            "diff",
            "--no-color",
            "--no-ext-diff",
            "--no-renames",
            "--numstat",
            "-z",
            base,
            tip,
        ],
    )?;
    let mut files = Vec::new();
    let (mut insertions, mut deletions) = (0_u64, 0_u64);
    for record in numstat.split(|byte| *byte == 0) {
        let record = String::from_utf8_lossy(record);
        let mut fields = record.splitn(3, '\t');
        let (Some(added), Some(removed), Some(path)) =
            (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        insertions = insertions.saturating_add(added.parse().unwrap_or(0));
        deletions = deletions.saturating_add(removed.parse().unwrap_or(0));
        files.push(path.to_string());
    }
    Some((files, insertions, deletions))
}

/// A JSON record roko wrote under `.roko/`, unless it is missing, too large
/// or not JSON.
fn read_record(path: &Path) -> Option<Value> {
    let metadata = std::fs::metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_RECORD_BYTES {
        return None;
    }
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

/// The commit `rev` names, when it names one.
fn rev_parse(workdir: &Path, rev: &str) -> Option<String> {
    git_text(
        workdir,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{rev}^{{commit}}"),
        ],
    )
    .filter(|oid| is_object_id(oid))
}

/// Whether commit `ancestor` is `descendant` or one of its ancestors.
fn is_ancestor(workdir: &Path, ancestor: &str, descendant: &str) -> bool {
    git(
        workdir,
        &["merge-base", "--is-ancestor", ancestor, descendant],
    )
    .is_some()
}

fn is_object_id(text: &str) -> bool {
    matches!(text.len(), 40 | 64) && text.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// The trimmed stdout of a git command that succeeded, unless it is empty.
fn git_text(workdir: &Path, args: &[&str]) -> Option<String> {
    let stdout = git(workdir, args)?;
    let text = String::from_utf8_lossy(&stdout).trim().to_string();
    (!text.is_empty()).then_some(text)
}

/// The stdout of a read-only git command in `workdir`, when it succeeded. It
/// takes no optional locks, never prompts, and ignores a `GIT_DIR` or
/// `GIT_WORK_TREE` the server inherited.
fn git(workdir: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(workdir)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    output.status.success().then_some(output.stdout)
}
