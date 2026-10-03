#![cfg(unix)]

//! Prompt-relevance canary (backlog 4220): over two scripted runs, each
//! prompt holds only its own task's learned context. It is the acceptance
//! test of slice 42 (prompt composition without noise, R2 #8).
//!
//! The fixture is R2's: a small `calc/` package with a test and
//! `docs/notes.md`, and seeded learned state that the runs must keep apart
//! by topic:
//!
//! - three knowledge entries: one about calc power, one another plan's `T01`
//!   left (tagged `crates/b/src/x.rs`), and a runtime success note;
//! - four playbooks: one about calc, and three with net successes and no
//!   word in common with any task;
//! - one error pattern, of a verify command no task runs.
//!
//! Run 1: `T01` edits `calc/power.py`, fails its verify step once and passes
//! on its retry, stating a lesson; `T02` edits `docs/notes.md` with another
//! verify command. Run 2: `T03`, a new calc task, runs `T01`'s verify
//! command among its own.
//!
//! The runs are in maximize mode, so no withhold arm or section draw hides
//! learned context: the prompts show what relevance selects. The helper
//! calls a failed verify step makes go to a provider of their own, so the
//! agent's provider logs the attempts alone. No model runs.

mod common;

use std::fs;
use std::path::Path;
use std::process::Output;

use common::ScriptedPlanWorkspace;
use common::scripted_provider::{Call, Script, ScriptedProvider, Turn};
use roko_learn::error_pattern_store::{ErrorPatternStore, GateFailureObservation, GateFailureSource};
use roko_learn::playbook::Playbook;
use serde_json::json;

/// Run 1's plan: `T01` and `T02`.
const RUN_ONE: &str = "calc-one";
/// Run 2's plan: `T03`.
const RUN_TWO: &str = "calc-cube";

/// `T01`'s verify command, which `T03` runs too: its failure's pattern is
/// keyed to it.
const POWER_CHECK: &str = "grep -qF 'base ** exponent' calc/power.py";

/// The lesson `T01`'s passing attempt states.
const LESSON: &str = "Exponentiation in calc/power.py uses Python's ** operator, not a loop.";

/// `roko.toml` additions: maximize mode, and no wait for siblings before a
/// verify step.
const CONFIG: &str = "experiments.maximize = true
gates.sibling_settle_secs = 0
";

/// Run 1: `T01` writes `calc/power.py` and may retry once; `T02` writes the
/// release notes. Their topics share no word.
const RUN_ONE_TASKS: &str = r#"[meta]
plan = "calc-one"
max_parallel = 1
skip_enrichment = true

[[task]]
id = "T01"
title = "Add power to calc"
description = "Write calc/power.py so that power(base, exponent) returns base raised to exponent."
role = "implementer"
status = "ready"
tier = "focused"
model_hint = "scripted"
files = ["calc/power.py"]
timeout_secs = 60
max_retries = 1
verify = [{ phase = "test", command = "grep -qF 'base ** exponent' calc/power.py" }]

[[task]]
id = "T02"
title = "Write the release notes"
description = "Add a Usage heading to docs/notes.md that explains how to install the release."
role = "implementer"
status = "ready"
tier = "focused"
model_hint = "scripted"
files = ["docs/notes.md"]
timeout_secs = 60
max_retries = 0
verify = [{ phase = "test", command = "grep -q '^## Usage' docs/notes.md" }]
"#;

/// Run 2: `T03` writes `calc/cube.py` on top of `T01`'s `power`, and checks
/// `calc/power.py` as `T01` did.
const RUN_TWO_TASKS: &str = r#"[meta]
plan = "calc-cube"
max_parallel = 1
skip_enrichment = true

[[task]]
id = "T03"
title = "Add a cube helper to calc"
description = "Write calc/cube.py so that cube(x) returns power(x, 3) from calc/power.py."
role = "implementer"
status = "ready"
tier = "focused"
model_hint = "scripted"
files = ["calc/cube.py"]
timeout_secs = 60
max_retries = 0
verify = [
  { phase = "test", command = "grep -qF 'base ** exponent' calc/power.py" },
  { phase = "test", command = "grep -q 'def cube' calc/cube.py" },
]
"#;

/// `calc/power.py` before the runs.
const POWER_STUB: &str = "def power(base, exponent):\n    raise NotImplementedError\n";
/// `T01`'s first `calc/power.py`, which its verify step rejects.
const POWER_LOOP: &str = "def power(base, exponent):
    result = 1
    for _ in range(exponent):
        result *= base
    return result
";
/// `T01`'s second `calc/power.py`, which passes.
const POWER_OPERATOR: &str = "def power(base, exponent):\n    return base ** exponent\n";
/// The package's test.
const POWER_TEST: &str = "from calc.power import power


def test_power():
    assert power(2, 10) == 1024
";

/// The agent: `T01` fails once and passes with a lesson; `T02` and `T03`
/// pass at once and state none.
fn agent() -> Script {
    let passed = format!("Wrote calc/power.py with base ** exponent.\nLesson: {LESSON}");
    Script::new()
        .task(
            "T01",
            [
                Turn::reply()
                    .write("calc/power.py", POWER_LOOP)
                    .text("Wrote calc/power.py with a loop."),
                Turn::reply()
                    .write("calc/power.py", POWER_OPERATOR)
                    .text(&passed),
            ],
        )
        .task(
            "T02",
            [Turn::reply()
                .write("docs/notes.md", "# Notes\n\n## Usage\n\nInstall the release.\n")
                .text("Wrote the release notes.\nLesson: none")],
        )
        .task(
            "T03",
            [Turn::reply()
                .write(
                    "calc/cube.py",
                    "from calc.power import power\n\n\ndef cube(x):\n    return power(x, 3)\n",
                )
                .text("Wrote calc/cube.py.\nLesson: none")],
        )
}

/// The provider of `routing.fast_task_model`, which takes the helper calls
/// after a failed verify step.
fn helper_script() -> Script {
    Script::new().otherwise(Turn::reply().text("helper").model("claude-helper"))
}

/// Install the helper calls' provider beside the workspace and route them to
/// it in `roko.toml`.
fn add_helper_provider(workspace: &ScriptedPlanWorkspace) -> ScriptedProvider {
    let helper = ScriptedProvider::install(&workspace.root.join("helper"), &helper_script());
    let config = workspace.repo.join("roko.toml");
    let mut toml = fs::read_to_string(&config).expect("read roko.toml");
    toml.push_str(&format!(
        "providers.helper-cli.kind = \"claude_cli\"
providers.helper-cli.command = {command:?}
models.helper-model.provider = \"helper-cli\"
models.helper-model.slug = \"claude-helper\"
models.helper-model.context_window = 200000
routing.fast_task_model = \"helper-model\"
",
        command = helper.command().display().to_string()
    ));
    fs::write(&config, toml).expect("write roko.toml");
    helper
}

/// The `calc/` package, its test, and `docs/notes.md`.
fn write_calc_package(repo: &Path) {
    let files = [
        ("calc/__init__.py", ""),
        ("calc/power.py", POWER_STUB),
        ("tests/test_power.py", POWER_TEST),
        ("docs/notes.md", "# Notes\n"),
    ];
    for (path, text) in files {
        let path = repo.join(path);
        fs::create_dir_all(path.parent().expect("a parent directory"))
            .expect("create a fixture directory");
        fs::write(&path, text).expect("write a fixture file");
    }
}

/// The learned state the runs start from: the knowledge entries, playbooks
/// and error pattern of the module documentation.
fn seed_learned_state(repo: &Path) {
    let roko = repo.join(".roko");
    let neuro = roko.join("neuro");
    fs::create_dir_all(&neuro).expect("create the knowledge store's directory");
    let now = chrono::Utc::now();
    let entries = [
        json!({
            "id": "kn-calc-power",
            "content": "calc power must reject negative exponents with a ValueError",
            "confidence": 0.8,
            "created_at": now,
        }),
        json!({
            "id": "kn-other-t01",
            "content": "T01 renamed the parser module and updated its imports",
            "confidence": 0.8,
            "tags": ["crates/b/src/x.rs", "plan:other-plan"],
            "created_at": now,
        }),
        json!({
            "id": "kn-success-note",
            "content": "Successful runtime episode for calc-one/T01: calc power passed",
            "source": "runtime:gate_verdict",
            "confidence": 0.9,
            "created_at": now,
        }),
    ];
    let lines: String = entries.iter().map(|entry| format!("{entry}\n")).collect();
    fs::write(neuro.join("knowledge.jsonl"), lines).expect("seed the knowledge store");

    let learn = roko.join("learn");
    let playbook_dir = learn.join("playbooks");
    fs::create_dir_all(&playbook_dir).expect("create the playbook directory");
    let calc = Playbook::new("pb-calc-power", "Implement calc power functions and test them");
    let mut playbooks = vec![calc];
    for (id, goal) in [
        ("pb-db-index", "Tune the database index cache"),
        ("pb-tls-rotate", "Rotate the TLS certificates before expiry"),
        ("pb-queue-batch", "Migrate the queue consumer to batches"),
    ] {
        let mut playbook = Playbook::new(id, goal);
        playbook.success_count = 6;
        playbook.failure_count = 1;
        playbooks.push(playbook);
    }
    for playbook in &playbooks {
        let json = serde_json::to_string(playbook).expect("serialize a playbook");
        fs::write(playbook_dir.join(format!("{}.json", playbook.id)), json)
            .expect("seed a playbook");
    }

    let mut patterns = ErrorPatternStore::empty();
    let _ = patterns.observe_gate_failure(GateFailureObservation::new(
        "verify::markdown lint found trailing spaces",
        "docs-plan",
        Some("T09".to_string()),
        "make lint-docs",
        "verify",
        "markdown lint found trailing spaces",
        GateFailureSource::GateClassification,
    ));
    patterns
        .save(&learn.join("error-patterns.json"))
        .expect("seed the error patterns");
}

/// The fixture repository, committed, with its learned state seeded: a
/// workspace whose plan is run 1's, the agent's provider and the helper
/// calls' provider.
fn fixture() -> (ScriptedPlanWorkspace, ScriptedProvider, ScriptedProvider) {
    let (workspace, provider) =
        ScriptedPlanWorkspace::with_provider(RUN_ONE, RUN_ONE_TASKS, &agent(), CONFIG);
    let helper = add_helper_provider(&workspace);
    write_calc_package(&workspace.repo);
    seed_learned_state(&workspace.repo);
    workspace.git(&["add", "--all"]);
    workspace.git(&["commit", "--quiet", "-m", "calc fixture"]);
    (workspace, provider, helper)
}

/// Add plan `plan` with `tasks_toml`, and commit it with what the last run
/// left in the working tree.
fn add_plan(workspace: &ScriptedPlanWorkspace, plan: &str, tasks_toml: &str) {
    let dir = workspace.repo.join("plans").join(plan);
    fs::create_dir_all(&dir).expect("create the plan directory");
    fs::write(dir.join("plan.md"), format!("# Plan: {plan}\n")).expect("write plan.md");
    fs::write(dir.join("tasks.toml"), tasks_toml).expect("write tasks.toml");
    workspace.git(&["add", "--all"]);
    workspace.git(&["commit", "--quiet", "-m", plan]);
}

/// What a call showed its agent: its prompt and its arguments, which carry
/// the system prompt.
fn shown(call: &Call) -> String {
    format!("{}\n{}", call.prompt, call.argv.join("\n"))
}

/// A run's exit status, stdout and the tail of its stderr.
fn context(output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let tail = stderr
        .get(stderr.len().saturating_sub(4000)..)
        .unwrap_or(&stderr);
    format!(
        "exit: {}\nstdout:\n{}\nstderr (tail):\n{tail}",
        output.status,
        String::from_utf8_lossy(&output.stdout)
    )
}

/// Run 1, which must pass: `T01` on its retry, `T02` at once.
fn run_one(workspace: &ScriptedPlanWorkspace) {
    let output = workspace.run_plan(RUN_ONE, &[]);
    assert!(output.status.success(), "run 1 failed\n{}", context(&output));
}

/// R2 #8: a task gets no learned section that is not about its topic. `T02`
/// sees no knowledge, playbooks or error patterns, though the stores hold
/// entries with net successes and another plan's `T01` note; `T01` sees no
/// pattern on its first attempt and its own failure's on its retry. No
/// prompt shows another plan's `T01` entry, a success note, the pattern of a
/// command no task runs, or a collective calibration block.
#[test]
fn unrelated_tasks_get_no_learned_sections() {
    let (workspace, provider, _helper) = fixture();
    run_one(&workspace);

    let notes = provider.calls_for("T02");
    assert_eq!(notes.len(), 1, "T02 passes at once");
    let notes = shown(&notes[0]);
    for section in [
        "# Neuro knowledge",
        "# Relevant playbooks",
        "Prior Verify Failure Patterns",
    ] {
        assert!(!notes.contains(section), "T02 shows {section}:\n{notes}");
    }

    let power = provider.calls_for("T01");
    assert_eq!(power.len(), 2, "T01 fails once and passes on its retry");
    let first = shown(&power[0]);
    assert!(!first.contains("Prior Verify Failure Patterns"), "{first}");
    let retry = shown(&power[1]);
    assert!(retry.contains("Prior Verify Failure Patterns"), "{retry}");
    assert!(retry.contains(&format!("Verify: {POWER_CHECK}")), "{retry}");

    for call in provider.calls() {
        let prompt = shown(&call);
        for noise in [
            "kn-other-t01",
            "kn-success-note",
            "make lint-docs",
            "# Collective calibration",
        ] {
            assert!(
                !prompt.contains(noise),
                "call {} shows {noise}:\n{prompt}",
                call.number
            );
        }
    }
}

/// A later task on the same topic sees what the last run learned about it:
/// `T03`, in run 2, sees `T01`'s failure pattern (keyed to the verify
/// command they share) with the fix `T01`'s pass recorded (4125), `T01`'s
/// lesson as durable knowledge (4216), and the calc playbook, and none of
/// the unrelated playbooks or the success note.
#[test]
fn a_later_task_sees_its_topics_lessons() {
    let (workspace, provider, _helper) = fixture();
    run_one(&workspace);
    add_plan(&workspace, RUN_TWO, RUN_TWO_TASKS);
    let output = workspace.run_plan(RUN_TWO, &[]);
    assert!(output.status.success(), "run 2 failed\n{}", context(&output));

    let cube = provider.calls_for("T03");
    assert_eq!(cube.len(), 1, "T03 passes at once");
    let cube = shown(&cube[0]);
    assert!(cube.contains("Prior Verify Failure Patterns"), "{cube}");
    assert!(cube.contains(&format!("Verify: {POWER_CHECK}")), "{cube}");
    assert!(
        cube.contains("Fix: Wrote calc/power.py with base ** exponent."),
        "{cube}"
    );
    assert!(cube.contains("# Neuro knowledge"), "{cube}");
    assert!(cube.contains(LESSON), "{cube}");
    assert!(cube.contains("# Relevant playbooks"), "{cube}");
    assert!(cube.contains("pb-calc-power"), "{cube}");
    for noise in [
        "pb-db-index",
        "pb-tls-rotate",
        "pb-queue-batch",
        "kn-success-note",
        "kn-other-t01",
        "make lint-docs",
    ] {
        assert!(!cube.contains(noise), "T03 shows {noise}:\n{cube}");
    }
}
