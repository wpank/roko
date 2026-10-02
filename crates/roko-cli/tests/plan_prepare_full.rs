#![cfg(unix)]

//! gap-c56341: `roko plan prepare --full` has the planner model write a plan's
//! `decomposition.md` and then its `rubric.md`, through the built `roko`
//! binary. The shared scripted provider stands in for the model; no live model
//! runs. It plays a turn per document: the prompts name `Task: decomposition:`
//! and `Task: rubric:`.

mod common;

use assert_cmd::cargo::cargo_bin;
use common::scripted_provider::{Call, Script, ScriptedProvider, Turn};
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

/// The decomposition the scripted model writes.
const DECOMPOSITION: &str = "# Decomposition: demo\n\n## Steps\n\n\
    ### Step 1: Write the widget (task T1)\n\n**Checkpoint**: `test -f widget.txt`\n";

/// The rubric the scripted model writes, inside a fence the document drops.
const RUBRIC: &str = "# Review rubric: demo\n\n## Blocking checklist\n\n\
    - [ ] `test -f widget.txt` exits 0.\n";

/// A `roko.toml` whose planner model is `provider`, and the plan
/// `plans/demo`.
fn setup_workspace(workdir: &Path, provider: &ScriptedProvider) {
    let provider = provider.command();
    fs::write(
        workdir.join("roko.toml"),
        format!(
            r#"
[agent]
default_model = "planner-model"
command = {provider:?}
bare_mode = false

[authoring]
planner_model = "planner-model"

[providers.scripted]
kind = "claude_cli"
command = {provider:?}

[models.planner-model]
provider = "scripted"
slug = "claude-sonnet-4-6"
context_window = 200000
"#,
            provider = provider.display().to_string()
        ),
    )
    .expect("write roko.toml");

    let plan_dir = workdir.join("plans/demo");
    fs::create_dir_all(&plan_dir).expect("create the plan directory");
    fs::write(plan_dir.join("plan.md"), "# Plan: demo\n").expect("plan.md");
    fs::write(
        plan_dir.join("tasks.toml"),
        "[meta]\nplan = \"demo\"\n\n[[task]]\nid = \"T1\"\ntitle = \"Write the widget\"\n\
         files = [\"widget.txt\"]\n\
         verify = [{ phase = \"structural\", command = \"test -f widget.txt\" }]\n",
    )
    .expect("tasks.toml");
}

fn prepare_full(workdir: &Path) -> Output {
    Command::new(cargo_bin("roko"))
        .current_dir(workdir)
        .args(["plan", "prepare", "plans/demo", "--full", "--workdir"])
        .arg(workdir)
        .output()
        .expect("run roko plan prepare --full")
}

/// The calls the provider took for a document, in order.
fn document_calls(provider: &ScriptedProvider) -> Vec<Call> {
    provider
        .calls()
        .into_iter()
        .filter(|call| call.task.is_some())
        .collect()
}

#[test]
fn plan_prepare_full_writes_the_decomposition_and_the_rubric() {
    let temp = tempfile::tempdir().expect("tempdir");
    let workdir = temp.path().join("work");
    fs::create_dir_all(&workdir).expect("create the workdir");
    let rubric_reply = format!("```markdown\n{RUBRIC}```\n");
    let script = Script::new()
        .task("decomposition", [Turn::reply().text(DECOMPOSITION)])
        .task("rubric", [Turn::reply().text(&rubric_reply)]);
    let provider = ScriptedProvider::install(&temp.path().join("provider"), &script);
    setup_workspace(&workdir, &provider);

    let output = prepare_full(&workdir);
    assert!(
        output.status.success(),
        "plan prepare --full failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let plan_dir = workdir.join("plans/demo");
    let read = |file: &str| fs::read_to_string(plan_dir.join(file)).expect(file);
    let decomposition = read("decomposition.md");
    assert!(
        decomposition.starts_with("<!-- Written by model `claude-sonnet-4-6`"),
        "{decomposition}"
    );
    assert!(decomposition.ends_with(DECOMPOSITION), "{decomposition}");
    let rubric = read("rubric.md");
    assert!(rubric.ends_with(RUBRIC), "the fence goes: {rubric}");
    let brief = read("brief.md");
    assert!(
        brief.contains("| Decomposition | `decomposition.md` |"),
        "{brief}"
    );
    assert!(brief.contains("| Review rubric | `rubric.md` |"), "{brief}");

    // One call per document; the rubric's comes second, with the
    // decomposition in its prompt.
    let calls = document_calls(&provider);
    let tasks: Vec<Option<&str>> = calls.iter().map(|call| call.task.as_deref()).collect();
    assert_eq!(tasks, [Some("decomposition"), Some("rubric")]);
    let (decomposition_prompt, rubric_prompt) = (&calls[0].prompt, &calls[1].prompt);
    assert!(
        decomposition_prompt.contains("## tasks.toml"),
        "{decomposition_prompt}"
    );
    assert!(
        rubric_prompt.contains("### Step 1: Write the widget"),
        "{rubric_prompt}"
    );

    // A second run keeps both documents and calls no model.
    let again = prepare_full(&workdir);
    assert!(again.status.success());
    assert_eq!(document_calls(&provider).len(), 2);
}
