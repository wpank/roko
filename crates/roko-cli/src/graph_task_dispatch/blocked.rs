//! A try that ends blocked by another task's file goes back to that task's
//! files and checks (bug-3b3bb6).
//!
//! An implementer that cannot fix a failure inside its task's files says so
//! with a `Blocked: <path> — <why>` line ([`stated_blocker`]). Phase 5's T4
//! could not: its renders were too quiet because of T3's renderer, its plan
//! forbade changing the renderer, both of its tries said so, and roko retried
//! it, then climbed, and the stronger model boosted the renders past the
//! check. Now a red try whose blocker is another task's file hands the failure
//! back: the task's next try may change that task's files too, that task's
//! checks judge it as well, and the blocked try does not count on its rung. A
//! blocker in a file no task of the plan changes stops the task for the
//! person.

use roko_agent::tool_loop::{StatedBlocker, stated_blocker};

use crate::graph_execution::plan_handback::{declared_owners, named_files};
use crate::runner::plan_loader::Plan;
use crate::task_parser::{TaskDef, TasksFile, VerifyStep};

/// The most other tasks one task's tries are handed back to for blockers; a
/// blocker past them stops the task for the person.
pub(super) const MAX_BLOCKED_HAND_BACKS: usize = 2;

/// What a red try's stated blocker means for its task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum BlockedTry {
    /// The try states no blocker.
    None,
    /// The blocker is a file the task's tries may change already: the next
    /// try is told so.
    InScope(StatedBlocker),
    /// The blocker is `path`, task `owner`'s file: the task's next tries may
    /// change that task's files too, and its checks judge them.
    HandBack {
        /// What the try said.
        blocker: StatedBlocker,
        /// The plan's file it names.
        path: String,
        /// The task that changes it.
        owner: String,
    },
    /// No task of the plan changes the blocker's file, or the task was
    /// handed back [`MAX_BLOCKED_HAND_BACKS`] times: the task stops, and
    /// `reason` tells the person why.
    Stop {
        /// What the try said.
        blocker: StatedBlocker,
        /// What the task's failure says.
        reason: String,
    },
}

/// The hand-backs one task's tries got for blockers: each owner's task id,
/// the file and what the try said, in the order they came.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct BlockedHandBacks {
    /// `(owner, path, blocker)` for each hand-back.
    pub(super) owners: Vec<(String, String, StatedBlocker)>,
}

/// What `output`, the final message of a red try of `task` in plan
/// `plan_id`, whose tasks are `plan`, means for the task, given the
/// hand-backs its tries already got, `handed`. `task` is the try's definition,
/// widened by those hand-backs. Without the plan's tasks a blocker outside the
/// task's files cannot be placed, and the try counts as any red try.
pub(super) fn classify(
    plan_id: &str,
    plan: Option<&TasksFile>,
    task: &TaskDef,
    output: &str,
    handed: &BlockedHandBacks,
) -> BlockedTry {
    let Some(blocker) = stated_blocker(output) else {
        return BlockedTry::None;
    };
    let in_scope = |path: &str| {
        task.files
            .iter()
            .any(|file| file.trim().trim_start_matches("./") == path)
    };
    if in_scope(blocker.path.trim_start_matches("./")) {
        return BlockedTry::InScope(blocker);
    }
    let Some(tasks) = plan else {
        return BlockedTry::None;
    };
    let owners = declared_owners(&Plan {
        id: plan_id.to_string(),
        dir: std::path::PathBuf::new(),
        tasks: tasks.clone(),
    });
    let named = named_files(&blocker.path, &owners);
    // The longest path the line names is the file it means.
    let Some((path, owner)) = named.into_iter().max_by_key(|(path, _)| path.len()) else {
        let reason = format!(
            "task {} is blocked: its try says the cause of its failure is in `{}`, which no task \
             of plan {plan_id} changes{}. Change that file, or replan the task with it, and \
             resume the plan",
            task.id,
            blocker.path,
            because(&blocker),
        );
        return BlockedTry::Stop { blocker, reason };
    };
    if owner == task.id || in_scope(&path) {
        return BlockedTry::InScope(blocker);
    }
    if handed.owners.len() >= MAX_BLOCKED_HAND_BACKS {
        let reason = format!(
            "task {} is blocked: its try says the cause of its failure is in `{path}`, task \
             {owner}'s file{}, and its tries were handed back to other tasks' files {} times \
             already. Fix the cause, or replan the tasks, and resume the plan",
            task.id,
            because(&blocker),
            handed.owners.len(),
        );
        return BlockedTry::Stop { blocker, reason };
    }
    BlockedTry::HandBack {
        blocker,
        path,
        owner,
    }
}

/// `: <why>` of `blocker`, or nothing when it gave no reason.
fn because(blocker: &StatedBlocker) -> String {
    if blocker.why.is_empty() {
        String::new()
    } else {
        format!(": {}", blocker.why)
    }
}

/// `task` as its next try runs it after `handed`, the hand-backs its tries got
/// for blockers, in plan `plan`: each owner's files join its own, each
/// owner's verify steps judge it after its own (what a borrowed step covers
/// and expects is its own task's, so those are cleared; each command runs
/// once), and its description leads with why it may change them.
pub(super) fn blocked_task(task: TaskDef, plan: &TasksFile, handed: &BlockedHandBacks) -> TaskDef {
    let mut task = task;
    let mut lines = Vec::new();
    for (owner, path, blocker) in &handed.owners {
        let Some(owner_task) = plan.tasks.iter().find(|candidate| &candidate.id == owner) else {
            continue;
        };
        for file in &owner_task.files {
            let file = file.trim().trim_start_matches("./");
            if !task
                .files
                .iter()
                .any(|own| own.trim().trim_start_matches("./") == file)
            {
                task.files.push(file.to_string());
            }
        }
        for step in &owner_task.verify {
            let command = step.command.trim();
            if !task.verify.iter().any(|own| own.command.trim() == command) {
                task.verify.push(VerifyStep {
                    covers: Vec::new(),
                    expect: None,
                    ..step.clone()
                });
            }
        }
        lines.push(format!(
            "A try of this task ended with `Blocked: {path}`: the cause of its failure is in \
             `{path}`, task {owner}'s file{}. This try may change task {owner}'s files too ({}), \
             and task {owner}'s checks judge it with this task's own.",
            because(blocker),
            owner_task.files.join(", "),
        ));
    }
    if lines.is_empty() {
        return task;
    }
    let mut description = lines.join(" ");
    description.push_str(
        " Fix the cause where it is. Never change what a check measures to make it pass: do not \
         scale, pad, synthesize or overwrite an output, or special-case a check's inputs.",
    );
    if let Some(own) = task
        .description
        .as_deref()
        .filter(|own| !own.trim().is_empty())
    {
        description.push_str("\n\nThe task's own description: ");
        description.push_str(own);
    }
    task.description = Some(description);
    task
}

/// What the next try's feedback says of `blocked`, what the red try's
/// blocker meant; `None` for a try with no blocker, or one that stops.
pub(super) fn feedback_note(blocked: &BlockedTry) -> Option<String> {
    match blocked {
        BlockedTry::InScope(blocker) => Some(format!(
            "Your last try ended with `Blocked: {}`, but this try may change that file: fix the \
             cause there.",
            blocker.path
        )),
        BlockedTry::HandBack {
            blocker,
            path,
            owner,
        } => Some(format!(
            "Your last try ended with `Blocked: {}`{}. That is task {owner}'s file `{path}`: this \
             try may change task {owner}'s files, which the task's description lists, and task \
             {owner}'s checks judge it too.",
            blocker.path,
            because(blocker),
        )),
        BlockedTry::None | BlockedTry::Stop { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use roko_core::error::RokoError;
    use roko_graph::cell::CellContext;
    use roko_graph::cells::TaskDispatcher;
    use tempfile::tempdir;

    use super::*;
    use crate::graph_task_dispatch::GraphFeedbackContext;
    use crate::graph_task_dispatch::tests::{
        FIXTURE_HANG_GUARD_SECS, make_spec, make_test_dispatcher, no_auto_fix,
    };

    /// A fake Claude CLI that keeps each call's arguments and prompt as
    /// `call-<n>` beside itself. With a `vendor-mode` file beside it, a try is
    /// blocked by a vendored file; a try that may change task T1's files makes
    /// `src/render.txt` loud; any other try says T1's renderer blocks it.
    const BLOCKED_PROVIDER: &str = r#"#!/bin/sh
dir=$(dirname -- "$0")
n=$(( $(cat "$dir/calls" 2>/dev/null || echo 0) + 1 ))
echo "$n" > "$dir/calls"
{ printf '%s\n' "$@"; cat; } > "$dir/call-$n"
if [ -f "$dir/vendor-mode" ]; then
  text='The bug is in the vendored code.\nBlocked: vendor/lib.txt — it drops the last frame'
elif grep -q 'may change task T1' "$dir/call-$n"; then
  printf 'loud\n' > "$dir/src/render.txt"
  text='Fixed the level in the renderer.'
else
  text='The renders are too quiet.\nBlocked: src/render.txt — T1 renders at -33 dBFS\nLesson: none'
fi
printf '{"type":"content_block_delta","delta":{"text":"%s"}}\n' "$text"
printf '%s\n' '{"type":"result","session_id":"sess-v1","model":"claude-sonnet-4-6","total_cost_usd":0.01,"usage":{"input_tokens":5,"output_tokens":10}}'
"#;

    /// T1 writes the renderer, T2's check needs its output loud, and T3's
    /// failure lies in a vendored file no task changes.
    const PLAN: &str = r#"[meta]
plan = "demo"

[[task]]
id = "T1"
title = "Write the renderer"
description = "Write the renderer."
role = "implementer"
status = "ready"
tier = "focused"
files = ["src/render.txt"]

[[task.verify]]
phase = "test"
command = "test -f src/render.txt"

[[task]]
id = "T2"
title = "Render the demos"
description = "Render the demos."
role = "implementer"
status = "ready"
tier = "focused"
files = ["tools/demo.sh"]
depends_on = ["T1"]

[[task.verify]]
phase = "test"
command = "grep -q loud src/render.txt"

[[task]]
id = "T3"
title = "Use the vendored code"
description = "Use the vendored code."
role = "implementer"
status = "ready"
tier = "focused"
files = ["tools/use.sh"]

[[task.verify]]
phase = "test"
command = "false"
"#;

    /// bug-3b3bb6 (Undertow phase 5, run 1f677933): T4's tries said their
    /// renders were too quiet because of T3's renderer, outside T4's files,
    /// and roko retried T4, then climbed, until a stronger model boosted the
    /// renders past the check. A red try that ends `Blocked: <path> — <why>`
    /// on another task's file goes back with that task's files and checks:
    /// here T2's next try may change T1's `src/render.txt`, is told why, fixes
    /// the level there and passes, its first try not counted on its rung. A
    /// blocker in a file no task changes stops the task for the person.
    #[tokio::test]
    async fn a_try_blocked_by_another_tasks_file_is_handed_back() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, template) = make_test_dispatcher(
            &temp,
            BLOCKED_PROVIDER,
            no_auto_fix,
            GraphFeedbackContext::default(),
        )
        .await;
        let plan_dir = temp.path().join("plans/demo");
        std::fs::create_dir_all(&plan_dir).expect("the plan dir");
        std::fs::write(plan_dir.join("tasks.toml"), PLAN).expect("tasks.toml");
        std::fs::create_dir_all(temp.path().join("src")).expect("src");
        std::fs::write(temp.path().join("src/render.txt"), "quiet\n").expect("T1's work");
        let plan = TasksFile::parse(&plan_dir.join("tasks.toml")).expect("the plan");
        let spec_of = |id: &str| {
            let task = TaskDef {
                model_hint: template.model_hint.clone(),
                timeout_secs: FIXTURE_HANG_GUARD_SECS,
                ..plan
                    .tasks
                    .iter()
                    .find(|task| task.id == id)
                    .expect("the task")
                    .clone()
            };
            let mut spec = make_spec(&task);
            spec.plan_id = "demo".to_string();
            spec.plan_dir = plan_dir.display().to_string();
            spec
        };
        let ctx = CellContext::new().with_run_id("run-blocked".to_string());
        let spec = spec_of("T2");

        let first = dispatcher.dispatch(&spec, Vec::new(), &ctx).await;
        assert!(
            matches!(&first, Err(error) if !matches!(error, RokoError::Rejected(_))),
            "{first:?}"
        );
        assert!(
            dispatcher.uncounted_tries.lock().is_empty(),
            "the ladder took the blocked try off its count"
        );
        let second = dispatcher.dispatch(&spec, Vec::new(), &ctx).await;

        assert!(second.is_ok(), "{second:?}");
        assert_eq!(
            std::fs::read_to_string(temp.path().join("src/render.txt")).expect("the render"),
            "loud\n"
        );
        let handed = dispatcher
            .blocked
            .lock()
            .get("demo/T2")
            .cloned()
            .expect("the hand-back");
        assert_eq!(handed.owners.len(), 1);
        assert_eq!(
            (handed.owners[0].0.as_str(), handed.owners[0].1.as_str()),
            ("T1", "src/render.txt")
        );
        let sent = std::fs::read_to_string(temp.path().join("call-2")).expect("the second call");
        for said in [
            "A try of this task ended with `Blocked: src/render.txt`",
            "T1 renders at -33 dBFS",
            "This try may change task T1's files too (src/render.txt)",
            "Never change what a check measures",
            "Your last try ended with `Blocked: src/render.txt`",
            "test -f src/render.txt",
        ] {
            assert!(sent.contains(said), "{said}: {sent}");
        }

        // A blocker in a file no task of the plan changes stops the task.
        std::fs::write(temp.path().join("vendor-mode"), "").expect("the mode");
        let reason = match dispatcher.dispatch(&spec_of("T3"), Vec::new(), &ctx).await {
            Err(RokoError::Rejected(reason)) => reason,
            other => panic!("the task stops: {other:?}"),
        };
        assert!(
            reason.starts_with(
                "task T3 is blocked: its try says the cause of its failure is in \
                 `vendor/lib.txt`, which no task of plan demo changes: it drops the last frame"
            ),
            "{reason}"
        );
    }
}
