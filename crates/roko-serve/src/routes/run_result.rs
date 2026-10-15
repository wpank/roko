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
//! chat host what the run changed and how to take it, and the run's diff can
//! show it (gap-0d6d21).
//!
//! A plan that was not delivered keeps the work its run accepted on its plan
//! branch, `roko/plan/<plan>`, and its batch record says which part of it no
//! check ran on (`work.unverified`, bug-db89d4). That work is never
//! delivered; the summary names it apart from the batch, so that a host can
//! show it as unverified.

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
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

/// Where a run's batch stands in the repository: its branch, the branch's tip
/// and, when it can be told, the commit the batch started from. The summary's
/// `result` and the run's diff both read it, so they agree (gap-0d6d21).
pub(super) struct ResultRange {
    /// `roko/batch/<run-id>`.
    pub(super) branch: String,
    /// The branch's tip.
    pub(super) commit: String,
    /// The commit the batch started from, from the run's records, else the
    /// merge base of the tip and `HEAD`.
    pub(super) base: Option<String>,
    /// The checkout's `HEAD` commit, when it has one.
    head: Option<String>,
}

/// Run `run_id`'s [`ResultRange`] in the repository at `workdir`, or `None`
/// when it has no batch branch there. `plan_ids` are the run's plans, whose
/// checkpoints may record the base.
pub(super) fn result_range(
    workdir: &Path,
    run_id: &str,
    plan_ids: &[String],
) -> Option<ResultRange> {
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
    Some(ResultRange {
        branch,
        commit,
        base,
        head,
    })
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
    let ResultRange {
        branch,
        commit,
        base,
        head,
    } = result_range(workdir, run_id, plan_ids)?;

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
    let unverified = batch_records(workdir, run_id, plan_ids)
        .iter()
        .filter_map(|(plan_id, record)| unverified_work(workdir, plan_id, record))
        .collect::<Vec<_>>();
    if !unverified.is_empty() {
        result["unverified"] = json!(unverified);
    }
    Some(result)
}

/// The work plan `plan_id` left on its plan branch that no check ran on, as
/// its batch `record` names it (bug-db89d4): the `commit` it ends at, the
/// `base` it starts from, its `tasks` with their verdicts, the files it
/// changes, the plan `branch` while it still points there, and the command
/// that shows it. It is never delivered, so it has no merge command. `None`
/// when the record names none, or its commits are gone.
fn unverified_work(workdir: &Path, plan_id: &str, record: &Value) -> Option<Value> {
    let work = &record["work"]["unverified"];
    let commit = rev_parse(
        workdir,
        work["commit"].as_str().filter(|oid| is_object_id(oid))?,
    )?;
    let base = rev_parse(
        workdir,
        work["base"].as_str().filter(|oid| is_object_id(oid))?,
    )?;
    let branch = record["plan_branch"].as_str().filter(|branch| {
        rev_parse(workdir, &format!("refs/heads/{branch}")).as_ref() == Some(&commit)
    });
    let shown = branch.unwrap_or(&commit);
    let mut entry = json!({
        "plan_id": plan_id,
        "commit": commit,
        "base": base,
        "tasks": work["tasks"],
        "review_command": format!("git diff {}..{shown}", &base[..12]),
    });
    if let Some(branch) = branch {
        entry["branch"] = json!(branch);
    }
    if let Some((files, insertions, deletions)) = changed_files(workdir, &base, &commit) {
        entry["diffstat"] = json!({
            "files": files.len(),
            "insertions": insertions,
            "deletions": deletions,
        });
        entry["files_changed"] = json!(files.into_iter().take(MAX_FILES).collect::<Vec<_>>());
    }
    Some(entry)
}

/// The plans of run `run_id` whose checkpoint keeps a batch record of the
/// run (`roko.batch@1`), each with its record: `plan_ids`, then the plans
/// the run's manifest names.
fn batch_records(workdir: &Path, run_id: &str, plan_ids: &[String]) -> Vec<(String, Value)> {
    run_plan_ids(workdir, run_id, plan_ids)
        .into_iter()
        .filter_map(|plan_id| {
            let checkpoint =
                read_record(&graph_plan_state_dir(workdir, &plan_id).join("checkpoint.json"))?;
            let record = checkpoint["extensions"][BATCH_EXTENSION]["value"].clone();
            (record["run_id"].as_str() == Some(run_id)).then_some((plan_id, record))
        })
        .collect()
}

/// `plan_ids`, then the plans run `run_id`'s manifest names that they leave
/// out.
fn run_plan_ids(workdir: &Path, run_id: &str, plan_ids: &[String]) -> Vec<String> {
    let mut plans = plan_ids.to_vec();
    if let Some(manifest) = read_record(&run_manifest_path(workdir, run_id))
        && let Some(recorded) = manifest["plan_ids"].as_array()
    {
        for plan_id in recorded.iter().filter_map(Value::as_str) {
            if !plans.iter().any(|known| known == plan_id) {
                plans.push(plan_id.to_string());
            }
        }
    }
    plans
}

/// Run `run_id`'s `manifest.json`.
fn run_manifest_path(workdir: &Path, run_id: &str) -> PathBuf {
    RokoLayout::for_project(workdir)
        .run_dir(run_id)
        .join("manifest.json")
}

/// The commit run `run_id`'s batch started from, as the run recorded it: its
/// manifest's `workspace.base_commit`, else the `base_oid` of a plan's batch
/// record that names the run. Only a commit the batch's tip `tip` descends
/// from counts.
fn recorded_base(workdir: &Path, run_id: &str, plan_ids: &[String], tip: &str) -> Option<String> {
    let from_manifest = read_record(&run_manifest_path(workdir, run_id)).and_then(|manifest| {
        manifest["workspace"]["base_commit"]
            .as_str()
            .map(str::to_owned)
    });
    let from_checkpoints = batch_records(workdir, run_id, plan_ids)
        .into_iter()
        .filter_map(|(_, record)| record["base_oid"].as_str().map(str::to_owned));
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

/// What [`run_diff`] returns at most.
#[derive(Debug, Clone, Copy)]
pub(super) struct DiffLimits {
    /// The most files listed; the totals count every file.
    pub(super) max_files: usize,
    /// The most bytes of one file's patch, cut on a line boundary.
    pub(super) max_patch_bytes: usize,
}

/// One file a run's batch changes from its base.
struct DiffEntry {
    path: String,
    status: &'static str,
    additions: u64,
    deletions: u64,
    binary: bool,
}

/// The diff of run `run_id`'s verified work (gap-0d6d21): each file its batch
/// changes from its base, with its status, line counts, whether it is binary
/// and its unified patch (`null` for a binary file), within `limits`, plus
/// totals over every file. `None` when the run has no batch branch in the
/// repository at `workdir`, no base to diff from, or git cannot tell.
pub(super) fn run_diff(
    workdir: &Path,
    run_id: &str,
    plan_ids: &[String],
    limits: DiffLimits,
) -> Option<Value> {
    let range = result_range(workdir, run_id, plan_ids)?;
    let base = range.base.as_deref()?;
    let mut diff = range_diff(workdir, base, &range.commit, limits)?;
    diff["run_id"] = json!(run_id);
    diff["branch"] = json!(range.branch);
    Some(diff)
}

/// The diff of the work run `run_id` left on a plan branch that no check
/// ran on (bug-db89d4), in [`run_diff`]'s shape with `work: "unverified"`
/// and its `plan_id`: plan `plan_id`'s, or the first such plan's. Its
/// `branch` is the plan branch while that still points at the work. `None`
/// when the run left no such work, or git cannot tell.
pub(super) fn unverified_diff(
    workdir: &Path,
    run_id: &str,
    plan_ids: &[String],
    plan_id: Option<&str>,
    limits: DiffLimits,
) -> Option<Value> {
    let entry = batch_records(workdir, run_id, plan_ids)
        .into_iter()
        .filter(|(id, _)| plan_id.is_none_or(|wanted| wanted == id))
        .find_map(|(id, record)| unverified_work(workdir, &id, &record))?;
    let (base, commit) = (entry["base"].as_str()?, entry["commit"].as_str()?);
    let mut diff = range_diff(workdir, base, commit, limits)?;
    diff["run_id"] = json!(run_id);
    diff["work"] = json!("unverified");
    diff["plan_id"] = entry["plan_id"].clone();
    diff["branch"] = entry.get("branch").cloned().unwrap_or(Value::Null);
    Some(diff)
}

/// Each file `commit` changes from `base`, with its status, line counts,
/// whether it is binary and its unified patch (`null` for a binary file),
/// within `limits`, plus totals over every file. `None` when git cannot
/// tell.
fn range_diff(workdir: &Path, base: &str, commit: &str, limits: DiffLimits) -> Option<Value> {
    let entries = diff_entries(workdir, base, commit)?;
    let kept = &entries[..entries.len().min(limits.max_files)];
    let paths = kept
        .iter()
        .filter(|entry| !entry.binary)
        .map(|entry| entry.path.as_str())
        .collect::<Vec<_>>();
    let mut patches = if paths.is_empty() {
        HashMap::new()
    } else {
        file_patches(workdir, base, commit, &paths, limits.max_patch_bytes)?
    };
    let files = kept
        .iter()
        .map(|entry| {
            let (patch, truncated) = if entry.binary {
                (Value::Null, false)
            } else {
                patches
                    .remove(&entry.path)
                    .map_or((json!(""), false), |(patch, cut)| (json!(patch), cut))
            };
            json!({
                "path": entry.path,
                "status": entry.status,
                "additions": entry.additions,
                "deletions": entry.deletions,
                "binary": entry.binary,
                "patch": patch,
                "truncated": truncated,
            })
        })
        .collect::<Vec<_>>();
    let additions = entries
        .iter()
        .fold(0_u64, |sum, entry| sum.saturating_add(entry.additions));
    let deletions = entries
        .iter()
        .fold(0_u64, |sum, entry| sum.saturating_add(entry.deletions));
    Some(json!({
        "base": base,
        "commit": commit,
        "files": files,
        "totals": {
            "files": entries.len(),
            "additions": additions,
            "deletions": deletions,
        },
        "truncated": kept.len() < entries.len(),
    }))
}

/// Each file `tip` changes from `base`, in git's path order: its status from
/// `--name-status` and its line counts from `--numstat` (a binary file counts
/// none). `None` when git cannot tell.
fn diff_entries(workdir: &Path, base: &str, tip: &str) -> Option<Vec<DiffEntry>> {
    let options = ["diff", "--no-color", "--no-ext-diff", "--no-renames", "-z"];
    let numstat = git(workdir, &[&options[..], &["--numstat", base, tip]].concat())?;
    let mut counts = HashMap::new();
    for record in numstat.split(|byte| *byte == 0) {
        let record = String::from_utf8_lossy(record);
        let mut fields = record.splitn(3, '\t');
        let (Some(added), Some(removed), Some(path)) =
            (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        let binary = added == "-" && removed == "-";
        counts.insert(
            path.to_string(),
            (
                added.parse::<u64>().unwrap_or(0),
                removed.parse::<u64>().unwrap_or(0),
                binary,
            ),
        );
    }
    let statuses = git(
        workdir,
        &[&options[..], &["--name-status", base, tip]].concat(),
    )?;
    let mut fields = statuses
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty());
    let mut entries = Vec::new();
    while let (Some(status), Some(path)) = (fields.next(), fields.next()) {
        let path = String::from_utf8_lossy(path).into_owned();
        let (additions, deletions, binary) = counts.get(&path).copied().unwrap_or((0, 0, false));
        entries.push(DiffEntry {
            status: match status.first() {
                Some(b'A') => "added",
                Some(b'D') => "deleted",
                _ => "modified",
            },
            path,
            additions,
            deletions,
            binary,
        });
    }
    Some(entries)
}

/// The unified patch of each of `paths` from `base` to `tip`, by path, each
/// cut on a line boundary at `max_patch_bytes` (and then marked truncated).
/// git writes one `diff --git` section per path, in the order `--name-status`
/// lists them, so sections pair with `paths` in turn; should the counts ever
/// differ, each section's header names its path instead. The output is read as
/// a stream, so no patch takes more memory than its own limit. `None` when git
/// fails.
fn file_patches(
    workdir: &Path,
    base: &str,
    tip: &str,
    paths: &[&str],
    max_patch_bytes: usize,
) -> Option<HashMap<String, (String, bool)>> {
    let mut child = git_command(workdir)
        .args([
            "--literal-pathspecs",
            "diff",
            "--no-color",
            "--no-ext-diff",
            "--no-renames",
            base,
            tip,
            "--",
        ])
        .args(paths)
        .stdout(Stdio::piped())
        .spawn()
        .ok()?;
    let read = child
        .stdout
        .take()
        .map(|stdout| read_sections(&mut BufReader::new(stdout), max_patch_bytes));
    let status = child.wait().ok()?;
    let sections = read?.ok()?;
    if !status.success() {
        return None;
    }
    let text = |patch: Vec<u8>| String::from_utf8_lossy(&patch).into_owned();
    if sections.len() == paths.len() {
        return Some(
            paths
                .iter()
                .zip(sections)
                .map(|(path, (patch, truncated))| ((*path).to_string(), (text(patch), truncated)))
                .collect(),
        );
    }
    Some(
        sections
            .into_iter()
            .filter_map(|(patch, truncated)| {
                let path = header_path(&patch)?;
                Some((path, (text(patch), truncated)))
            })
            .collect(),
    )
}

/// The `diff --git` sections of a patch stream, each with the lines that fit
/// in `max_patch_bytes` and whether any did not.
fn read_sections(
    reader: &mut impl BufRead,
    max_patch_bytes: usize,
) -> std::io::Result<Vec<(Vec<u8>, bool)>> {
    // Each line keeps at most this much: enough to see a section header, and
    // to tell whether the line fits what is left of its patch.
    let line_cap = max_patch_bytes.saturating_add(1).max(64);
    let mut sections: Vec<(Vec<u8>, bool)> = Vec::new();
    let mut line = Vec::new();
    loop {
        line.clear();
        let length = read_line_capped(reader, &mut line, line_cap)?;
        if length == 0 {
            return Ok(sections);
        }
        if line.starts_with(b"diff --git ") {
            sections.push((Vec::new(), false));
        }
        let Some((patch, truncated)) = sections.last_mut() else {
            continue;
        };
        if *truncated {
            continue;
        }
        if length == line.len() && patch.len() + length <= max_patch_bytes {
            patch.extend_from_slice(&line);
        } else {
            *truncated = true;
        }
    }
}

/// The path a `diff --git a/<path> b/<path>` section header names. Without
/// renames both sides name the same path; a quoted header names none here.
fn header_path(section: &[u8]) -> Option<String> {
    let first = section.split(|byte| *byte == b'\n').next()?;
    let rest = std::str::from_utf8(first.strip_prefix(b"diff --git ")?).ok()?;
    // "a/" + path + " b/" + path: five bytes besides the path twice.
    let both = rest.len().checked_sub(5)?;
    if both % 2 != 0 {
        return None;
    }
    let length = both / 2;
    let path = rest.get(2..2 + length)?;
    (rest.starts_with("a/") && rest.get(2 + length..)? == format!(" b/{path}"))
        .then(|| path.to_string())
}

/// Read one line from `reader` into `line`, keeping at most `cap` bytes of
/// it, and return the line's whole length (0 at the end of the stream), so a
/// very long line costs no more memory than `cap`.
fn read_line_capped(
    reader: &mut impl BufRead,
    line: &mut Vec<u8>,
    cap: usize,
) -> std::io::Result<usize> {
    let mut length = 0;
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return Ok(length);
        }
        let end = available.iter().position(|byte| *byte == b'\n');
        let chunk = &available[..end.map_or(available.len(), |at| at + 1)];
        let room = cap.saturating_sub(line.len());
        line.extend_from_slice(&chunk[..chunk.len().min(room)]);
        let used = chunk.len();
        length += used;
        reader.consume(used);
        if end.is_some() {
            return Ok(length);
        }
    }
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

/// The stdout of a read-only git command in `workdir`, when it succeeded.
fn git(workdir: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let output = git_command(workdir).args(args).output().ok()?;
    output.status.success().then_some(output.stdout)
}

/// A git command in `workdir` that takes no optional locks, never prompts,
/// and ignores a `GIT_DIR` or `GIT_WORK_TREE` the server inherited.
fn git_command(workdir: &Path) -> Command {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(workdir)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .stdin(Stdio::null())
        .stderr(Stdio::null());
    command
}
