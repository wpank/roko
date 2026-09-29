//! Red flags: rejecting an attempt before its verify steps run.
//!
//! An attempt the provider reports as successful goes through this screen
//! before any verify step (C1 implication 5, `meyerson2025solving`). It is
//! rejected when:
//!
//! - its output ran away: it reported more output tokens than `[gates]
//!   max_output_tokens` allows its role;
//! - its role's output is the product (a reviewer's verdict, a planner's
//!   plan), and the output is empty, or a fenced TOML or JSON block in it is
//!   cut off or does not parse. The provider adapters already fail a
//!   non-zero exit, a turn cap hit and, for the Claude CLI, an empty
//!   response;
//! - it is an implementer attempt at a task that names `files`, and the task
//!   has changed nothing, or added only stub lines
//!   ([`roko_gate::analyze_diff`]): "no changes". Other roles, refactors
//!   (role `refactorer`) and tasks without `files` are exempt.
//!
//! A rejection costs no compile or test run. It settles like a failed verify
//! step, `gate_failed` and blamed on the agent, so no learner credits it; the
//! verdict's `failure_class.rung` names the check (`pre_verify:<check>`), and
//! its message is the next attempt's feedback.

use roko_gate::{DiffPayload, analyze_diff};

use super::diff_snapshot::AttemptDiff;
use super::sibling_settle::declares;
use super::*;

/// Prefix of the gate name, and so of the verdict's `failure_class.rung`,
/// of an attempt the screen rejected: `pre_verify:<check>`.
pub(super) const PRE_VERIFY_GATE_PREFIX: &str = "pre_verify:";

/// Most `files` entries a "no changes" message names.
const NAMED_FILES: usize = 5;

/// Why the screen rejected an attempt.
#[derive(Debug)]
struct Rejection {
    /// The check, e.g. `no_changes`.
    check: &'static str,
    /// What was wrong and what to do instead.
    message: String,
}

impl GraphTaskDispatcher {
    /// Screen an attempt the provider reported as successful, before its
    /// verify steps run. A rejection is an `Err`, a verify failure of gate
    /// `pre_verify:<check>`, and its message is left as the next attempt's
    /// feedback.
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn screen_attempt(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        dispatch: &crate::dispatch_v2::AgentResultDispatch,
        workdir: &Path,
        attempt_key: &str,
        attempt_number: u32,
        progress_tx: Option<&tokio::sync::mpsc::Sender<GraphTaskEvent>>,
    ) -> Result<()> {
        let role = task.role.as_deref().unwrap_or("implementer");
        let mut rejection = output_red_flag(&self.config, role, dispatch);
        if rejection.is_none()
            && let Some(diff) = self.attempt_diff(spec, task, attempt_key, workdir).await
        {
            rejection = no_changes_red_flag(task, role, &diff, attempt_number).await;
        }
        let Some(Rejection { check, message }) = rejection else {
            return Ok(());
        };
        let gate = format!("{PRE_VERIFY_GATE_PREFIX}{check}");
        let message = format!("Rejected before its verify steps ran ({gate}). {message}");
        tracing::warn!(
            plan_id = %spec.plan_id,
            task_id = %task.id,
            attempt = attempt_number,
            %gate,
            reason = %message,
            "attempt rejected before its verify steps"
        );
        if let Some(tui) = &self.tui_bridge {
            tui.gate_result_with_output(&spec.plan_id, &task.id, &gate, false, Some(&message));
        }
        if let Some(progress_tx) = progress_tx {
            let _ = progress_tx
                .send(GraphTaskEvent::Progress {
                    message: format!("rejected before verify: {gate}"),
                    completed: None,
                    total: None,
                })
                .await;
        }
        if let Some(feedback) = GateFeedback::from_raw(&message) {
            self.gate_retry_context.record(
                &spec.plan_id,
                &task.id,
                feedback,
                attempt_number.saturating_add(1),
            );
        }
        Err(RokoError::Verify { gate, message })
    }
}

/// A red flag on the attempt's output: past its role's output-token cap, or
/// a malformed product.
fn output_red_flag(
    config: &RokoConfig,
    role: &str,
    dispatch: &crate::dispatch_v2::AgentResultDispatch,
) -> Option<Rejection> {
    let output_tokens = u64::from(dispatch.result.usage.output_tokens);
    if let Some(cap) = config.gates.max_output_tokens_for(role)
        && output_tokens > cap
    {
        return Some(Rejection {
            check: "overlong_output",
            message: format!(
                "Red flag, overlong output: the attempt reported {output_tokens} output tokens, \
                 more than the {cap} allowed for role `{role}` ([gates] max_output_tokens). \
                 Runaway output is discarded; do the task in fewer, shorter steps."
            ),
        });
    }
    if !output_is_the_product(role) {
        return None;
    }
    let output = dispatch.result.output.body.as_text().unwrap_or_default();
    malformed_product(output).map(|problem| Rejection {
        check: "malformed_output",
        message: format!(
            "Red flag, malformed output: the {role}'s output is its product, and {problem}. \
             Answer with the complete, well-formed result."
        ),
    })
}

/// Roles whose output, rather than a diff, is the product: a verdict or a
/// plan.
fn output_is_the_product(role: &str) -> bool {
    matches!(
        role.trim().to_ascii_lowercase().replace('_', "-").as_str(),
        "reviewer"
            | "quick-reviewer"
            | "architect"
            | "auditor"
            | "critic"
            | "planner"
            | "strategist"
    )
}

/// Why `output`, a product such as a verdict or a plan, is malformed: it is
/// empty, or one of its fenced TOML or JSON blocks is never closed or does
/// not parse. Free text is not judged: the Graph path asks for no particular
/// format yet (gap-f4b935).
fn malformed_product(output: &str) -> Option<String> {
    if output.trim().is_empty() {
        return Some("it is empty".to_string());
    }
    let mut lines = output.lines();
    while let Some(line) = lines.next() {
        let Some(language) = line.trim().strip_prefix("```") else {
            continue;
        };
        let language = language.trim().to_ascii_lowercase();
        let mut block = String::new();
        let mut closed = false;
        for inner in lines.by_ref() {
            if inner.trim_start().starts_with("```") {
                closed = true;
                break;
            }
            block.push_str(inner);
            block.push('\n');
        }
        if language != "toml" && language != "json" {
            continue;
        }
        if !closed {
            return Some(format!("its ```{language} block is cut off"));
        }
        let error = if language == "toml" {
            toml::from_str::<toml::Value>(&block)
                .err()
                .map(|error| error.to_string())
        } else {
            serde_json::from_str::<serde_json::Value>(&block)
                .err()
                .map(|error| error.to_string())
        };
        if let Some(error) = error {
            let error: String = error
                .lines()
                .find(|line| !line.trim().is_empty())
                .unwrap_or_default()
                .chars()
                .take(200)
                .collect();
            return Some(format!("its ```{language} block does not parse: {error}"));
        }
    }
    None
}

/// Whether an attempt must leave a change: an implementer's, at a task that
/// names `files`.
fn expects_changes(role: &str, task: &TaskDef) -> bool {
    role.trim().eq_ignore_ascii_case("implementer")
        && task.files.iter().any(|file| !file.trim().is_empty())
}

/// The "no changes" red flag: an implementer attempt at a task that names
/// `files` whose task has changed nothing, or added only stub lines.
async fn no_changes_red_flag(
    task: &TaskDef,
    role: &str,
    diff: &AttemptDiff,
    attempt_number: u32,
) -> Option<Rejection> {
    if !expects_changes(role, task) {
        return None;
    }
    // A change to a path both this task and a sibling declare may be this
    // task's: it cannot be called unchanged, nor judged a stub.
    let shared = diff.sibling_paths.iter().any(|path| {
        task.files
            .iter()
            .any(|file| declares(file, Path::new(path)))
    });
    if shared {
        return None;
    }
    let files = named_files(&task.files);
    if diff.changes.is_empty() {
        // A resumed run: its earlier attempts ran in another process, and
        // whatever they changed is part of this base.
        if diff.base_is_this_attempts && attempt_number > 0 {
            return None;
        }
        return Some(Rejection {
            check: "no_changes",
            message: format!(
                "Red flag, no changes: the task names files to change ({files}), and its \
                 attempts have left the working tree as they found it. Make the change the \
                 task asks for; its verify steps run only on a changed tree."
            ),
        });
    }
    let patch = diff.patch().await?;
    analyze_diff(&DiffPayload::new(patch))
        .all_added_are_forbidden
        .then(|| Rejection {
            check: "no_changes",
            message: format!(
                "Red flag, no changes: every line the task added to {files} is a stub, such as \
                 `todo!()`, `unimplemented!()` or a bare `Ok(())`. Replace the stubs with the \
                 implementation."
            ),
        })
}

/// The first few `files`, in backticks.
fn named_files(files: &[String]) -> String {
    let mut named: Vec<String> = files
        .iter()
        .filter(|file| !file.trim().is_empty())
        .take(NAMED_FILES)
        .map(|file| format!("`{}`", file.trim()))
        .collect();
    if files.len() > NAMED_FILES {
        named.push(format!("and {} more", files.len() - NAMED_FILES));
    }
    named.join(", ")
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;
    use crate::graph_task_dispatch::diff_snapshot::tests::commit_repo;
    use crate::graph_task_dispatch::tests::{
        VERIFY_PROVIDER, make_spec, make_test_dispatcher, no_auto_fix, verify_step,
    };

    /// A fake Claude CLI that answers with `text` (JSON-escaped) and reports
    /// `output_tokens`, after running `edit` in its working directory.
    fn provider(edit: &str, text: &str, output_tokens: u32) -> String {
        format!(
            r#"#!/bin/sh
set -eu
cat >/dev/null
{edit}
printf '%s\n' '{{"type":"content_block_delta","delta":{{"text":"{text}"}}}}'
printf '%s\n' '{{"type":"result","session_id":"s","model":"claude-sonnet-4-6","total_cost_usd":0.01,"usage":{{"input_tokens":5,"output_tokens":{output_tokens}}}}}'
"#
        )
    }

    /// Dispatch `task` once, expecting a pre-verify rejection; returns its
    /// gate and message, and checks that the verify step's `marker` was
    /// never created.
    async fn rejected(
        dispatcher: &GraphTaskDispatcher,
        task: &TaskDef,
        marker: &Path,
    ) -> (String, String) {
        let error = dispatcher
            .dispatch(&make_spec(task), Vec::new(), &CellContext::new())
            .await
            .expect_err("the attempt is rejected");
        let RokoError::Verify { gate, message } = error else {
            panic!("expected a verify failure, got {error}");
        };
        assert!(!marker.exists(), "the verify steps must not run: {message}");
        (gate, message)
    }

    #[tokio::test]
    async fn empty_diff_fails_before_verify_runs() {
        let temp = tempdir().expect("tempdir");
        commit_repo(
            temp.path(),
            &[("src/lib.rs", "pub fn one() -> u8 {\n    1\n}\n")],
        );
        let (dispatcher, mut task) = make_test_dispatcher(
            &temp,
            VERIFY_PROVIDER,
            no_auto_fix,
            GraphFeedbackContext::default(),
        )
        .await;
        let marker = temp.path().join("verify-ran");
        task.files = vec!["src/lib.rs".to_string()];
        task.verify = vec![verify_step(
            "structural",
            &format!("touch {}", marker.display()),
        )];

        let (gate, message) = rejected(&dispatcher, &task, &marker).await;
        assert_eq!(gate, "pre_verify:no_changes");
        assert!(message.contains("no changes"), "{message}");
        assert!(message.contains("`src/lib.rs`"), "{message}");

        // The message is the next attempt's feedback.
        let next = dispatcher.next_retry_attempt(&make_spec(&task).plan_id, &task.id);
        let feedback = next.feedback.expect("feedback for the next attempt");
        assert!(feedback.raw_output.contains("no changes"), "{feedback:?}");

        // A task without `files`, like a non-implementer role, is exempt.
        task.files.clear();
        let outputs = dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &CellContext::new())
            .await
            .expect("a task without files may change nothing");
        assert_eq!(
            TaskGateVerdict::from_signals(&outputs),
            Some(TaskGateVerdict::Passed)
        );
        assert!(marker.exists(), "its verify step ran");
    }

    #[tokio::test]
    async fn a_real_change_passes_and_stub_only_changes_do_not() {
        let temp = tempdir().expect("tempdir");
        commit_repo(
            temp.path(),
            &[("src/lib.rs", "pub fn one() -> u8 {\n    1\n}\n")],
        );
        let stub = provider(
            "printf 'pub fn two() -> u8 { todo!() }\\n' >> src/lib.rs",
            "done",
            10,
        );
        let (dispatcher, mut task) =
            make_test_dispatcher(&temp, &stub, no_auto_fix, GraphFeedbackContext::default()).await;
        let marker = temp.path().join("verify-ran");
        task.files = vec!["src/lib.rs".to_string()];
        task.verify = vec![verify_step(
            "structural",
            &format!("touch {}", marker.display()),
        )];
        let (gate, message) = rejected(&dispatcher, &task, &marker).await;
        assert_eq!(gate, "pre_verify:no_changes");
        assert!(message.contains("stub"), "{message}");

        // The same task, once a later attempt writes the body, passes.
        std::fs::write(
            temp.path().join("src/lib.rs"),
            "pub fn one() -> u8 {\n    1\n}\npub fn two() -> u8 {\n    one() + one()\n}\n",
        )
        .expect("implement");
        std::fs::write(
            temp.path().join("fake-claude-stream.sh"),
            provider(":", "done", 10),
        )
        .expect("rewrite provider");
        dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &CellContext::new())
            .await
            .expect("the change is real now");
        assert!(marker.exists());
    }

    #[tokio::test]
    async fn overlong_output_fails_before_verify_runs() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, mut task) = make_test_dispatcher(
            &temp,
            &provider(":", "a very long answer", 5_000),
            |config| {
                no_auto_fix(config);
                config
                    .gates
                    .max_output_tokens
                    .insert("implementer".to_string(), 1_000);
            },
            GraphFeedbackContext::default(),
        )
        .await;
        let marker = temp.path().join("verify-ran");
        task.verify = vec![verify_step(
            "structural",
            &format!("touch {}", marker.display()),
        )];

        let (gate, message) = rejected(&dispatcher, &task, &marker).await;
        assert_eq!(gate, "pre_verify:overlong_output");
        assert!(message.contains("5000 output tokens"), "{message}");
        assert!(message.contains("1000"), "{message}");
    }

    #[tokio::test]
    async fn malformed_reviewer_output_fails_before_verify_runs() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, mut task) = make_test_dispatcher(
            &temp,
            &provider(":", r"Review done.\n```toml\n[verdict\noverall = \n```", 40),
            no_auto_fix,
            GraphFeedbackContext::default(),
        )
        .await;
        let marker = temp.path().join("verify-ran");
        task.role = Some("reviewer".to_string());
        task.verify = vec![verify_step(
            "structural",
            &format!("touch {}", marker.display()),
        )];

        let (gate, message) = rejected(&dispatcher, &task, &marker).await;
        assert_eq!(gate, "pre_verify:malformed_output");
        assert!(message.contains("does not parse"), "{message}");
    }

    #[test]
    fn products_are_judged_by_their_fenced_blocks() {
        assert_eq!(malformed_product("  \n").as_deref(), Some("it is empty"));
        assert_eq!(malformed_product("APPROVE: looks right"), None);
        assert_eq!(
            malformed_product("```toml\n[verdict]\noverall = \"approve\"\n```\n"),
            None
        );
        assert_eq!(
            malformed_product("```json\n{\"status\": \"pass\"}\n```"),
            None
        );
        // Another language's block is skipped whole, even when it holds
        // text that looks like a fence.
        assert_eq!(malformed_product("```rust\nlet x = 1;\n```\ndone"), None);
        assert_eq!(
            malformed_product("```toml\n[verdict]\noverall = \"approve\"\n").as_deref(),
            Some("its ```toml block is cut off")
        );
        assert!(
            malformed_product("```json\n{\"status\": }\n```")
                .is_some_and(|problem| problem.starts_with("its ```json block does not parse"))
        );
        assert!(output_is_the_product("quick_reviewer"));
        assert!(!output_is_the_product("implementer"));
    }
}
