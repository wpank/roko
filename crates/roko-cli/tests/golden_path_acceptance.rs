#![cfg(unix)]

//! The golden path's acceptance test (gap-f30b8e; backlog 3115, 3116): a
//! seed repository and a spec-first plan of nine tasks over it, each with a
//! planner-written `[task.accept]` test, in `tests/fixtures/golden_path/`
//! (`seed/`, `plan/`, and the scripted provider's `replay/`).
//!
//! - `golden_path_fixture_is_red_on_the_seed`: the seed passes the plan's
//!   whole-plan check, and every task's acceptance test fails on it, so a
//!   pass credits only the task's work.
//! - `golden_path_fixture_plan_merges_green`: the plan runs through the
//!   ladder on the scripted provider, escalates once, is delivered into the
//!   run's batch branch, and the merged seed is green.
//! - `golden_path_live` (backlog 3117, ignored): the same run on the real
//!   models of an operator's ladder, with a report of what it cost.
//!
//! The TypeScript checks need Node 22.6 or later, which runs TypeScript by
//! stripping its types; without it they are skipped, and the test says so.

mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Instant;

use assert_cmd::cargo::cargo_bin;
use common::scripted_provider::{Script, ScriptedProvider, Turn};
use roko_cli::task_accept::{AcceptStore, pin_task};
use roko_cli::task_parser::TasksFile;
use serde_json::Value;

/// Wire slugs of the ladder's rungs, cheapest first.
const CHEAP: &str = "claude-haiku-4-5";
const MID: &str = "claude-sonnet-4-6";
const TOP: &str = "claude-opus-4-1";

/// The task whose start rung fails it twice (`replay/T3-wrong/`), so that it
/// passes one rung up.
const ESCALATED: &str = "T3";

/// A part of the fixture: `seed`, `plan` or `replay`.
fn fixture(part: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/golden_path")
        .join(part)
}

/// Copy the directory `from` to `to`.
fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("create a directory");
    for entry in fs::read_dir(from).expect("read a fixture directory") {
        let entry = entry.expect("a directory entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("a file type").is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).expect("copy a fixture file");
        }
    }
}

/// Whether `node` runs TypeScript by stripping its types (Node 22.6 or
/// later).
fn node_runs_typescript() -> bool {
    Command::new("node")
        .args(["--experimental-strip-types", "-e", "const n: number = 1;"])
        .output()
        .is_ok_and(|output| output.status.success())
}

/// Whether `command` runs Node.
fn needs_node(command: &str) -> bool {
    command.contains("node ")
}

/// Run `command` with `sh -c` in `dir`, with cargo building into `target`:
/// whether it passed, and what it printed.
fn sh(dir: &Path, target: &Path, command: &str) -> (bool, String) {
    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(dir)
        .env("CARGO_TARGET_DIR", target)
        .output()
        .expect("run sh");
    let printed = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    (output.status.success(), printed)
}

/// backlog 3115: the seed is green, and every task's checks are red on it.
/// The seed passes the plan's `[meta] verify` (fmt, clippy, the Rust, Python
/// and TypeScript suites). Each task's pinned acceptance test, run as `plan
/// run` runs it, and any verify step of its own then fail on the seed.
#[test]
fn golden_path_fixture_is_red_on_the_seed() {
    let temp = tempfile::tempdir().expect("tempdir");
    let seed = temp.path().join("seed");
    copy_dir(&fixture("seed"), &seed);
    let target = temp.path().join("target");
    let typescript = node_runs_typescript();
    if !typescript {
        eprintln!("node 22.6 or later is not on PATH: the TypeScript checks are skipped");
    }
    let plan_dir = fixture("plan");
    let plan = TasksFile::parse(&plan_dir.join("tasks.toml")).expect("the fixture plan parses");
    let count = plan.tasks.len();
    assert!((8..=10).contains(&count), "{count} tasks");

    for step in &plan.meta.verify {
        if needs_node(&step.command) && !typescript {
            continue;
        }
        let (passed, printed) = sh(&seed, &target, &step.command);
        assert!(passed, "the seed fails `{}`:\n{printed}", step.command);
    }

    // The acceptance tests are pinned outside any home directory.
    let store = AcceptStore::at(temp.path().join("accept"));
    let plan_id = plan.meta.plan.clone();
    for mut task in plan.tasks {
        let id = task.id.clone();
        assert!(task.has_accept_tests(), "{id} has no [task.accept] test");
        assert!(task.model_hint.is_none(), "{id} pins a model");
        pin_task(&store, &seed, &plan_id, &plan_dir, &mut task).expect("pin the accept tests");
        for step in &task.verify {
            if needs_node(&step.command) && !typescript {
                continue;
            }
            let (passed, printed) = sh(&seed, &target, &step.command);
            assert!(
                !passed,
                "{id}: a check passes on the seed, so a pass would credit nothing:\n{}\n{printed}",
                step.command
            );
        }
    }
}

/// Run git in `dir`; its trimmed stdout.
fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// Every file under `dir`, as its path relative to `dir` and its text, in
/// path order.
fn files_under(dir: &Path) -> Vec<(String, String)> {
    let mut files = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        for entry in fs::read_dir(&next).expect("read a replay directory") {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            let relative = path.strip_prefix(dir).expect("a path under the replay");
            let text = fs::read_to_string(&path).expect("read a replay file");
            files.push((relative.to_string_lossy().into_owned(), text));
        }
    }
    files.sort();
    files
}

/// A turn that writes the files under `dir` into the agent's working tree.
fn replay_turn(dir: &Path) -> Turn {
    let mut turn = Turn::reply();
    for (path, text) in files_under(dir) {
        turn = turn.write(&path, &text);
    }
    turn
}

/// The ladder's provider: each task of `plan` writes its solution from
/// `replay/`, except [`ESCALATED`], which writes the wrong one twice first.
fn replay_script(plan: &TasksFile) -> Script {
    let replay = fixture("replay");
    plan.tasks.iter().fold(Script::new(), |script, task| {
        let solution = replay_turn(&replay.join(&task.id));
        let turns = if task.id == ESCALATED {
            let wrong = replay_turn(&replay.join(format!("{ESCALATED}-wrong")));
            vec![wrong.clone(), wrong, solution]
        } else {
            vec![solution]
        };
        script.task(&task.id, turns)
    })
}

/// The provider of the helper calls after a failed check, so the ladder's
/// provider sees attempts only.
fn helper_script() -> Script {
    Script::new().otherwise(
        Turn::reply()
            .text("helper")
            .model("claude-helper")
            .usage(3, 2, 0.001),
    )
}

/// The run's `roko.toml`: three ladder rungs on `provider`, `helper` taking
/// the helper calls and the default model, a plan budget with a turn cap, and
/// no red-on-base check (`golden_path_fixture_is_red_on_the_seed` makes it).
fn config(provider: &Path, helper: &Path) -> String {
    format!(
        r#"[agent]
default_model = "helper-model"
command = {helper:?}
bare_mode = false

[providers.ladder-cli]
kind = "claude_cli"
command = {provider:?}

[providers.helper-cli]
kind = "claude_cli"
command = {helper:?}

[models.cheap-model]
provider = "ladder-cli"
slug = "{CHEAP}"
context_window = 200000

[models.mid-model]
provider = "ladder-cli"
slug = "{MID}"
context_window = 200000

[models.top-model]
provider = "ladder-cli"
slug = "{TOP}"
context_window = 200000

[models.helper-model]
provider = "helper-cli"
slug = "claude-helper"
context_window = 200000

[routing]
fast_task_model = "helper-model"

[routing.ladder]
rungs = [
  {{ name = "cheap", model = "cheap-model" }},
  {{ name = "mid", model = "mid-model" }},
  {{ name = "top", model = "top-model" }},
]

[budget]
max_plan_usd = 5.0
max_turn_usd = 0.5

[spec_quality]
red_on_base = false

[gates]
sibling_settle_secs = 0
"#,
        provider = provider.display().to_string(),
        helper = helper.display().to_string(),
    )
}

/// The seed as a git repository on `main`, with the plan in
/// `plans/golden-path/` and `config` as its `roko.toml`, committed.
fn seed_repo(repo: &Path, config: &str) {
    copy_dir(&fixture("seed"), repo);
    copy_dir(&fixture("plan"), &repo.join("plans/golden-path"));
    fs::write(repo.join("roko.toml"), config).expect("write roko.toml");
    git(repo, &["init", "--quiet", "--initial-branch=main"]);
    for (key, value) in [
        ("user.name", "Operator"),
        ("user.email", "operator@example.test"),
        ("commit.gpgsign", "false"),
    ] {
        git(repo, &["config", key, value]);
    }
    git(repo, &["add", "-A"]);
    git(repo, &["commit", "--quiet", "-m", "seed"]);
}

/// `roko --json plan run plans/golden-path` in `repo` through the built
/// binary, as a user whose home is `home`, without the invoking
/// environment's `ROKO_*` variables, log and config variables. Cargo builds
/// into each checkout's own `target/` (a relative `CARGO_TARGET_DIR`), so no
/// task's worktree reuses crates another one built from older sources, and
/// finds its toolchains where the real home keeps them.
fn plan_command(repo: &Path, home: &Path) -> Command {
    let mut command = Command::new(cargo_bin("roko"));
    command
        .current_dir(repo)
        .args(["--json", "plan", "run", "plans/golden-path", "--workdir"])
        .arg(repo)
        .env("HOME", home)
        .env("CARGO_TARGET_DIR", "target");
    // Without these, rustup and cargo look for their homes under HOME.
    if let Some(real_home) = std::env::var_os("HOME").map(PathBuf::from) {
        for (name, dir) in [("CARGO_HOME", ".cargo"), ("RUSTUP_HOME", ".rustup")] {
            if std::env::var_os(name).is_none() {
                command.env(name, real_home.join(dir));
            }
        }
    }
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("ROKO_") {
            command.env_remove(name);
        }
    }
    for name in ["RUST_LOG", "XDG_CONFIG_HOME", "CLAUDECODE"] {
        command.env_remove(name);
    }
    command
}

/// The run's JSON summary: the last stdout line that opens an object,
/// through the end of stdout; `None` when that is not JSON.
fn run_summary(stdout: &[u8]) -> Option<Value> {
    let stdout = String::from_utf8_lossy(stdout);
    let start = stdout.rfind("\n{").map_or(0, |index| index + 1);
    serde_json::from_str(&stdout[start..]).ok()
}

/// The run's directory, `.roko/runs/<run>`: there is one run in `repo`.
fn run_dir(repo: &Path) -> PathBuf {
    let mut runs: Vec<PathBuf> = fs::read_dir(repo.join(".roko/runs"))
        .expect("the run wrote .roko/runs")
        .map(|entry| entry.expect("a run directory").path())
        .filter(|path| path.is_dir())
        .collect();
    assert_eq!(runs.len(), 1, "{runs:?}");
    runs.remove(0)
}

/// The verdict rows of the attempt log in `run_dir`, `attempts.jsonl`.
fn verdicts(run_dir: &Path) -> Vec<Value> {
    fs::read_to_string(run_dir.join("attempts.jsonl"))
        .expect("read the run's attempts.jsonl")
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str::<Value>(line).expect("a JSON row"))
        .filter(|row| row["schema_version"] == "roko.verdict/1")
        .collect()
}

/// Where an attempt ran and how it ended: `<rung> <reason> <outcome>`, with
/// `-` for a pinned attempt's rung.
fn place(verdict: &Value) -> String {
    let ladder = &verdict["ladder"];
    format!(
        "{} {} {}",
        ladder["rung"].as_str().unwrap_or("-"),
        ladder["reason"].as_str().unwrap_or("?"),
        verdict["outcome"].as_str().unwrap_or("?")
    )
}

/// What a run of the fixture plan in `repo` must show, scripted or live: it
/// exited 0 with one delivered batch, every task's last attempt passed and
/// none ran pinned, and the seed's `main` takes the batch branch by
/// fast-forward, after which the seed, built into `target`, passes the
/// plan's whole-plan check. Returns each task's attempts, in order, as
/// [`place`]s.
fn assert_merges_green(
    repo: &Path,
    plan: &TasksFile,
    run: &Output,
    target: &Path,
) -> BTreeMap<String, Vec<String>> {
    let log = format!(
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(run.status.success(), "{log}");
    let summary = run_summary(&run.stdout).unwrap_or_else(|| panic!("no JSON summary: {log}"));
    let batch = &summary["batch"];
    let deliveries = batch["deliveries"].as_array().map_or(0, Vec::len);
    assert_eq!(deliveries, 1, "{summary:#}");
    let delivered = batch["deliveries"][0]["state"].as_str();
    assert_eq!(delivered, Some("delivered"), "{summary:#}");
    let branch = batch["branch"].as_str().expect("the run's batch branch");

    let mut attempts: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for verdict in verdicts(&run_dir(repo)) {
        let task = verdict["task_id"].as_str().unwrap_or("?").to_string();
        attempts.entry(task).or_default().push(place(&verdict));
    }
    for task in &plan.tasks {
        let tried = attempts.get(&task.id).cloned().unwrap_or_default();
        let id = &task.id;
        let last = tried.last().map_or("none", String::as_str);
        assert!(
            last.ends_with(" passed"),
            "{id} did not pass: {tried:?}\n{log}"
        );
        let pinned = tried.iter().any(|place| place.contains(" pinned "));
        assert!(!pinned, "{id} ran pinned: {tried:?}");
    }

    git(repo, &["merge", "--ff-only", branch]);
    for step in &plan.meta.verify {
        let (passed, printed) = sh(repo, target, &step.command);
        assert!(
            passed,
            "the merged seed fails `{}`:\n{printed}",
            step.command
        );
    }
    attempts
}

/// backlog 3116: the fixture plan runs end to end on the scripted provider,
/// with per-task worktrees (the default), a plan budget and a turn cap. Every
/// task passes, on the rung its tier starts on except T3, which fails twice
/// there and passes one rung up, and no attempt is pinned. The plan is
/// delivered once into the run's batch branch, the seed's `main` takes it by
/// fast-forward, and the merged seed passes the plan's whole-plan check.
#[test]
fn golden_path_fixture_plan_merges_green() {
    if !node_runs_typescript() {
        eprintln!("node 22.6 or later is not on PATH: the plan's TypeScript checks cannot run");
        return;
    }
    let plan = TasksFile::parse(&fixture("plan").join("tasks.toml")).expect("the plan parses");
    let temp = tempfile::tempdir().expect("tempdir");
    let provider = ScriptedProvider::install(&temp.path().join("ladder"), &replay_script(&plan));
    let helper = ScriptedProvider::install(&temp.path().join("helper"), &helper_script());
    let repo = temp.path().join("repo");
    seed_repo(&repo, &config(&provider.command(), &helper.command()));
    let home = temp.path().join("home");
    fs::create_dir_all(&home).expect("create the home directory");

    // The scripted run has no use for the invoking environment's keys.
    let mut command = plan_command(&repo, &home);
    for name in roko_core::child_env::PROVIDER_KEY_VARS {
        command.env_remove(name);
    }
    let run = command.output().expect("run roko plan run");

    let attempts = assert_merges_green(&repo, &plan, &run, &temp.path().join("target"));
    assert_eq!(
        attempts[ESCALATED],
        [
            "cheap start gate_failed",
            "cheap start gate_failed",
            "mid escalated passed",
        ],
        "{attempts:#?}"
    );
}

/// The live run's `roko.toml`: the operator's file at `path`, which names
/// the providers, `[models.*]` and `[routing.ladder]` to run on, with its
/// plan budget set to `max_usd` and a turn cap of at most $0.50 unless it
/// sets its own.
fn live_config(path: &Path, max_usd: f64) -> String {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    let mut config: toml::Table = toml::from_str(&text)
        .unwrap_or_else(|error| panic!("{} is not TOML: {error}", path.display()));
    for table in ["providers", "models"] {
        assert!(
            config.contains_key(table),
            "{} has no [{table}]",
            path.display()
        );
    }
    let routing = config.get("routing");
    let has_ladder = routing.is_some_and(|routing| routing.get("ladder").is_some());
    assert!(has_ladder, "{} has no [routing.ladder]", path.display());
    let budget = config
        .entry("budget")
        .or_insert(toml::Value::Table(toml::Table::new()))
        .as_table_mut()
        .expect("[budget] is a table");
    budget.insert("max_plan_usd".to_string(), toml::Value::Float(max_usd));
    budget
        .entry("max_turn_usd")
        .or_insert(toml::Value::Float(max_usd.min(0.5)));
    toml::to_string(&config).expect("serialize the live config")
}

/// The live run's report: its id, wall time and budget, each task's attempts
/// (rung, why it ran there, model, outcome, cost), the escalations, and the
/// cost per verified task.
fn live_report(
    run_path: &Path,
    summary: &Value,
    verdicts: &[Value],
    wall_secs: f64,
    max_usd: f64,
) -> Value {
    let mut tasks: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    let mut last_passed: BTreeMap<String, bool> = BTreeMap::new();
    let mut escalations = Vec::new();
    let (mut billed, mut api_equiv) = (0.0_f64, 0.0_f64);
    for verdict in verdicts {
        let task = verdict["task_id"].as_str().unwrap_or("?").to_string();
        let (ladder, cost) = (&verdict["ladder"], &verdict["cost"]);
        let attempt = serde_json::json!({
            "rung": ladder["rung"],
            "reason": ladder["reason"],
            "model": verdict["executed"]["model_dispatched"],
            "outcome": verdict["outcome"],
            "billed_usd": cost["billed_usd"],
            "api_equiv_usd": cost["api_equiv_usd"],
        });
        if ladder["reason"] == "escalated" {
            escalations.push(serde_json::json!({ "task": task, "attempt": attempt }));
        }
        billed += cost["billed_usd"].as_f64().unwrap_or(0.0);
        api_equiv += cost["api_equiv_usd"].as_f64().unwrap_or(0.0);
        last_passed.insert(task.clone(), verdict["outcome"] == "passed");
        tasks.entry(task).or_default().push(attempt);
    }
    let verified = last_passed.values().filter(|&&passed| passed).count();
    let per_verified = |total: f64| (verified > 0).then_some(total / verified as f64);
    let run_id = run_path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned());
    serde_json::json!({
        "schema": "roko.golden_path_live/1",
        "run_id": run_id,
        "succeeded": summary["succeeded"],
        "wall_secs": wall_secs,
        "max_plan_usd": max_usd,
        "tasks": tasks,
        "escalations": escalations,
        "verified_tasks": verified,
        "cost_usd": { "billed": billed, "api_equiv": api_equiv },
        "cost_per_verified_task_usd": {
            "billed": per_verified(billed),
            "api_equiv": per_verified(api_equiv),
        },
        "run_total_cost_usd": summary["total_cost_usd"],
    })
}

/// Write `report` as `golden-path-live-<run>.json` to
/// `ROKO_GOLDEN_PATH_REPORT_DIR`, else to a directory in the system's temp
/// dir; never inside the repository, nor in `scratch`, which the test
/// removes. Returns the file's path.
fn write_report(report: &Value, scratch: &Path) -> PathBuf {
    let dir = match std::env::var_os("ROKO_GOLDEN_PATH_REPORT_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => std::env::temp_dir().join("roko-golden-path-live"),
    };
    fs::create_dir_all(&dir).expect("create the report directory");
    let dir = dir.canonicalize().expect("resolve the report directory");
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for outside in [repository, scratch.to_path_buf()] {
        let outside = outside.canonicalize().expect("resolve a directory");
        assert!(
            !dir.starts_with(&outside),
            "the report directory {} is inside {}",
            dir.display(),
            outside.display()
        );
    }
    let run = report["run_id"].as_str().unwrap_or("unknown");
    let path = dir.join(format!("golden-path-live-{run}.json"));
    let text = serde_json::to_string_pretty(report).expect("serialize the report");
    fs::write(&path, text + "\n").expect("write the report");
    path
}

/// backlog 3117: the fixture plan on real models, through an operator's
/// ladder. Ignored by default, since it spends money (decision 3114 caps
/// it); an operator runs it on request:
///
/// ```text
/// ROKO_GOLDEN_PATH_CONFIG=live.toml cargo test -p roko-cli \
///     --test golden_path_acceptance golden_path_live -- --ignored --nocapture
/// ```
///
/// `ROKO_GOLDEN_PATH_CONFIG` names the `roko.toml` to run on (see
/// [`live_config`]); the plan budget is `ROKO_GOLDEN_PATH_MAX_USD`, $4 by
/// default. Keys come from the operator's environment: the test sets none,
/// and with HOME a temp dir roko reads no `~/.roko/.env`. The run starts
/// once and nothing touches it. Besides the scripted run's checks, it is not
/// interrupted, and every attempt names the model it dispatched and has a
/// cost amount. The report ([`live_report`]) goes to
/// `ROKO_GOLDEN_PATH_REPORT_DIR`, else to a temp dir the test prints.
#[test]
#[ignore = "live models: it spends money"]
fn golden_path_live() {
    assert!(
        node_runs_typescript(),
        "node 22.6 or later must be on PATH: the plan has TypeScript checks"
    );
    let config_path = std::env::var_os("ROKO_GOLDEN_PATH_CONFIG")
        .map(PathBuf::from)
        .expect("ROKO_GOLDEN_PATH_CONFIG names the roko.toml to run on");
    let max_usd = std::env::var("ROKO_GOLDEN_PATH_MAX_USD")
        .map_or(Ok(4.0), |value| value.trim().parse::<f64>())
        .expect("ROKO_GOLDEN_PATH_MAX_USD is an amount in dollars");
    assert!(max_usd.is_finite() && max_usd > 0.0, "{max_usd}");
    let plan = TasksFile::parse(&fixture("plan").join("tasks.toml")).expect("the plan parses");
    let temp = tempfile::tempdir().expect("tempdir");
    let repo = temp.path().join("repo");
    seed_repo(&repo, &live_config(&config_path, max_usd));
    let home = temp.path().join("home");
    fs::create_dir_all(&home).expect("create the home directory");

    // One start, and no resume or edit to `.roko/` until the run ends.
    let started = Instant::now();
    let run = plan_command(&repo, &home)
        .output()
        .expect("run roko plan run");
    let wall_secs = started.elapsed().as_secs_f64();
    let run_path = run_dir(&repo);
    let verdicts = verdicts(&run_path);
    let summary = run_summary(&run.stdout).unwrap_or_default();
    let report = live_report(&run_path, &summary, &verdicts, wall_secs, max_usd);
    let written = write_report(&report, temp.path());
    println!("golden-path live report: {}", written.display());

    assert_merges_green(&repo, &plan, &run, &temp.path().join("target"));
    assert!(summary["interrupted_by"].is_null(), "{summary:#}");
    for verdict in &verdicts {
        let dispatched = verdict["executed"]["model_dispatched"].is_string();
        assert!(dispatched, "no dispatched model: {verdict:#}");
        let cost = &verdict["cost"];
        let amount = cost["billed_usd"].is_number() || cost["api_equiv_usd"].is_number();
        assert!(amount, "no cost amount: {verdict:#}");
    }
}
