#![allow(dead_code)]

use assert_cmd::Command;
use assert_cmd::assert::Assert;
use assert_cmd::cargo::cargo_bin;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command as ProcessCommand, Output, Stdio};
use std::time::{Duration, Instant};

pub mod scripted_provider;

use scripted_provider::{Script, ScriptedProvider};

pub const MOCK_FIXTURE: &str = "mock-self-host-fixture";
pub const SAMPLE_PLAN_ID: &str = "test-wire-xyz";

const SAMPLE_PLAN_MARKDOWN: &str =
    "# Plan: test-wire-xyz\n\nA single-task smoke-test plan generated from a prompt.\n";

const SAMPLE_TASKS_TOML: &str = r#"[meta]
plan = "test-wire-xyz"
iteration = 1
total = 1
done = 0
status = "ready"
max_parallel = 1
estimated_total_minutes = 1
skip_enrichment = true

[[task]]
id = "T1"
title = "Run the offline smoke-test task"
description = "Exercise the mock-backed plan runner with a single task."
role = "implementer"
status = "ready"
tier = "focused"
model_hint = "claude-opus-4-6"
max_loc = 10
files = ["Cargo.toml", "Cargo.lock", "src/main.rs", ".roko"]
allowed_tools = ["read_file"]
denied_tools = []
mcp_servers = []
depends_on = []
depends_on_plan = []
acceptance = ["cargo check"]
verify = [{ command = "cargo check", phase = "compile" }]
timeout_secs = 60
max_retries = 1

[task.context]
read_files = [{ path = "Cargo.toml", why = "confirm the workspace is initialized" }, { path = "src/main.rs", why = "confirm the sample binary compiles" }]
symbols = []
anti_patterns = []
prior_failures = []
"#;

pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

pub fn rustc_version_ge(version_text: &str, target: &str) -> bool {
    fn parse(version_text: &str) -> Option<(u64, u64, u64)> {
        let version = version_text
            .split_whitespace()
            .find(|part| part.chars().next().is_some_and(|ch| ch.is_ascii_digit()))?;
        let mut parts = version.split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts
            .next()
            .and_then(|part| part.split('-').next())
            .unwrap_or("0")
            .parse()
            .ok()?;
        Some((major, minor, patch))
    }

    parse(version_text)
        .zip(parse(target))
        .is_some_and(|(actual, wanted)| actual >= wanted)
}

pub fn run_process(workdir: &Path, args: &[&str]) {
    let (program, rest) = args.split_first().expect("process command");
    let status = ProcessCommand::new(program)
        .current_dir(workdir)
        .args(rest)
        .status()
        .unwrap_or_else(|err| panic!("spawn {program}: {err}"));
    assert!(
        status.success(),
        "{program} {:?} failed with {status}",
        rest
    );
}

pub fn process_stdout(workdir: &Path, args: &[&str]) -> String {
    let (program, rest) = args.split_first().expect("process command");
    let output = ProcessCommand::new(program)
        .current_dir(workdir)
        .args(rest)
        .output()
        .unwrap_or_else(|err| panic!("spawn {program}: {err}"));
    assert!(
        output.status.success(),
        "{program} {:?} failed with {}",
        rest,
        output.status
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

pub fn init_workspace(workdir: &Path) {
    Command::cargo_bin("roko")
        .expect("roko binary")
        .current_dir(workdir)
        .arg("init")
        .arg(workdir)
        .assert()
        .success();
}

pub fn seed_minimal_rust_project(workdir: &Path) {
    fs::create_dir_all(workdir.join("src")).expect("create src");
    fs::write(
        workdir.join("Cargo.toml"),
        r#"[package]
name = "ux44-smoke"
version = "0.1.0"
edition = "2024"

[dependencies]
"#,
    )
    .expect("write Cargo.toml");
    fs::write(
        workdir.join("src").join("main.rs"),
        "fn main() {\n    println!(\"ux44 smoke\");\n}\n",
    )
    .expect("write src/main.rs");
}

pub fn seed_git_repo(workdir: &Path) {
    run_process(workdir, &["git", "init"]);
    run_process(workdir, &["git", "config", "user.name", "UX44 Smoke"]);
    run_process(
        workdir,
        &["git", "config", "user.email", "ux44-smoke@example.com"],
    );
    run_process(workdir, &["git", "add", "."]);
    run_process(workdir, &["git", "commit", "-m", "seed"]);
}

pub fn write_executable(path: &Path, body: &str) {
    fs::write(path, body).unwrap_or_else(|err| panic!("write {}: {err}", path.display()));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut perms = fs::metadata(path)
            .unwrap_or_else(|err| panic!("metadata {}: {err}", path.display()))
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms)
            .unwrap_or_else(|err| panic!("chmod {}: {err}", path.display()));
    }
}

/// Mock `claude` script that emits valid stream-json output so runner v2
/// can process it without a real LLM. Writes a trivial code change + completes.
const MOCK_CLAUDE_SCRIPT: &str = r#"#!/bin/sh
# Mock claude CLI for runner v2 smoke tests.
# Reads prompt from stdin (ignored), emits stream-json to stdout.
cat <<'STREAM'
{"type":"system","subtype":"init","session_id":"smoke-sess","model":"claude-sonnet-4-6","tools":[]}
{"type":"assistant","subtype":"message","message":{"content":[{"type":"text","text":"I will verify the project compiles."}],"usage":{"input_tokens":100,"output_tokens":50,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}}
{"type":"result","session_id":"smoke-sess","total_cost_usd":0.001,"num_turns":1,"is_error":false}
STREAM
"#;

pub fn setup_sample_plan_workspace(workdir: &Path) {
    init_workspace(workdir);
    seed_minimal_rust_project(workdir);
    seed_git_repo(workdir);

    // Create a mock claude script that outputs valid stream-json.
    let mock_claude = workdir.join("mock-claude.sh");
    write_executable(&mock_claude, MOCK_CLAUDE_SCRIPT);

    // Configure roko.toml to use the mock claude script.
    // Replace the existing `command = "..."` line under [agent] with our mock path.
    let roko_toml = workdir.join("roko.toml");
    let existing = fs::read_to_string(&roko_toml).unwrap_or_default();
    let mock_cmd_line = format!("command = {:?}", mock_claude.display());
    let updated = if existing.contains("command = ") {
        // Replace the existing command line.
        let mut result = String::new();
        for line in existing.lines() {
            if line.trim_start().starts_with("command = ") {
                result.push_str(&mock_cmd_line);
            } else {
                result.push_str(line);
            }
            result.push('\n');
        }
        result
    } else if existing.contains("[agent]") {
        existing.replace("[agent]", &format!("[agent]\n{mock_cmd_line}"))
    } else {
        format!("{existing}\n[agent]\n{mock_cmd_line}\n")
    };
    // The smoke plan runs in the shared working tree, as it did before
    // per-task worktrees became the default (gap-4ec59f).
    let updated = if updated.contains("\nworktree_per_task = ") {
        updated.replace("\nworktree_per_task = true", "\nworktree_per_task = false")
    } else if updated.contains("\n[runner]\n") {
        updated.replace("\n[runner]\n", "\n[runner]\nworktree_per_task = false\n")
    } else {
        format!("{updated}\n[runner]\nworktree_per_task = false\n")
    };
    // gap-0ee70b: these smoke plans are about the run, not the spec gate, so
    // their checks are not proven red on the base first.
    let updated = if updated.contains("\nred_on_base = ") {
        updated.replace("\nred_on_base = true", "\nred_on_base = false")
    } else if updated.contains("\n[spec_quality]\n") {
        updated.replace(
            "\n[spec_quality]\n",
            "\n[spec_quality]\nred_on_base = false\n",
        )
    } else {
        format!("{updated}\n[spec_quality]\nred_on_base = false\n")
    };
    fs::write(&roko_toml, updated).expect("write roko.toml with mock claude");

    let plan_dir = workdir.join("plans").join(SAMPLE_PLAN_ID);
    fs::create_dir_all(&plan_dir).expect("create sample plan dir");
    fs::write(plan_dir.join("plan.md"), SAMPLE_PLAN_MARKDOWN).expect("write plan.md");
    fs::write(plan_dir.join("tasks.toml"), SAMPLE_TASKS_TOML).expect("write tasks.toml");

    let index_path = workdir.join("plans").join("INDEX.md");
    if index_path.exists() {
        fs::remove_file(&index_path).expect("remove plans/INDEX.md");
    }

    run_process(workdir, &["git", "add", "."]);
    run_process(workdir, &["git", "commit", "-m", "prepare smoke plan"]);
    let branch = format!("roko/plan/{SAMPLE_PLAN_ID}");
    let main_branch = process_stdout(workdir, &["git", "branch", "--show-current"]);
    run_process(workdir, &["git", "checkout", "-b", &branch]);
    run_process(workdir, &["git", "checkout", &main_branch]);

    fs::create_dir_all(workdir.join(".roko").join("state")).expect("create state dir");
    fs::write(mock_state_path(workdir), "3").expect("prime mock cursor");
}

pub fn mock_state_path(workdir: &Path) -> PathBuf {
    workdir
        .join(".roko")
        .join("state")
        .join("mock-dispatcher-turn.txt")
}

pub fn run_sample_plan(workdir: &Path) -> Value {
    // Use ProcessCommand (std) so we can capture output without asserting
    // exit code — the plan run may exit non-zero when it goes through the
    // full gate/verify/review pipeline with mock responses but still
    // produces valid JSON output with meaningful metrics.
    //
    // The Graph engine spawns `claude` as a child process. The mock script is
    // configured via `agent.command` in `roko.toml` (set up by
    // `setup_sample_plan_workspace`).
    //
    // The plan run includes gate compilation which can be slow, so we
    // enforce a timeout. If it doesn't finish, we kill and read partial output.
    let mut command = ProcessCommand::new(cargo_bin("roko"));
    command
        .current_dir(workdir)
        .arg("--json")
        .arg("plan")
        .arg("run")
        .arg("plans");
    let mut child = command
        // Isolate from user's global config / API keys so the mock
        // agent command from roko.toml is used.
        .env("HOME", workdir)
        .env("ROKO_LOG", "error")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("XDG_CONFIG_HOME")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn roko plan run");

    let timeout = Duration::from_secs(120);
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    break;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(_) => break,
        }
    }
    let output = child.wait_with_output().expect("read plan run output");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json_start = stdout
        .lines()
        .scan(0usize, |offset, line| {
            let start = *offset;
            *offset += line.len() + 1;
            Some((start, line))
        })
        .find_map(|(start, line)| line.starts_with('{').then_some(start))
        .unwrap_or(0);
    serde_json::from_str(&stdout[json_start..]).unwrap_or_else(|err| {
        panic!(
            "parse sample plan JSON stdout: {err}\nstdout: {}\nstderr: {}",
            stdout,
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

/// A throwaway Graph plan workspace whose agent is a scripted fake Claude
/// CLI, isolated from the invoking user. Under one canonical temp root it
/// holds `repo` (the git repository roko runs in), `home` (roko's `HOME`,
/// so no `~/.roko/.env` or global config of the user's is read) and
/// `fixtures` (the agent script and whatever it or the test records there),
/// so what lives outside the repository never shows up in a scan of it.
pub struct ScriptedPlanWorkspace {
    _temp: tempfile::TempDir,
    pub root: PathBuf,
    pub repo: PathBuf,
    pub home: PathBuf,
    pub fixtures: PathBuf,
}

impl ScriptedPlanWorkspace {
    /// A repository holding a minimal crate and the plan `plan`, whose
    /// `tasks.toml` is `tasks_toml` with `{fixtures}` standing for the
    /// fixtures directory, committed with `.roko/` ignored. `roko.toml`
    /// routes every task to `fixtures/fake-claude.sh`, which runs
    /// `agent_script`, through a `claude_cli` provider on model `scripted`,
    /// and runs the tasks in the shared working tree, as these tests were
    /// written for before per-task worktrees became the default (gap-4ec59f;
    /// `--worktree-per-task` still turns them on). It is written as
    /// root-level dotted keys, so `extra_config`, appended to it, can add keys
    /// such as `agent.env_passthrough = [...]`. `roko init` is not used: its
    /// template depends on this machine's PATH and keys.
    pub fn new(plan: &str, tasks_toml: &str, agent_script: &str, extra_config: &str) -> Self {
        Self::create(None, plan, tasks_toml, extra_config, |root| {
            let agent = root.join("fixtures").join("fake-claude.sh");
            write_executable(&agent, agent_script);
            agent
        })
    }

    /// [`Self::new`] with the shared [`ScriptedProvider`] playing `script` as
    /// the agent. It lives in `root/provider`, outside the repository and the
    /// fixtures, so its turns, which may print a canary, are not scanned with
    /// them.
    pub fn with_provider(
        plan: &str,
        tasks_toml: &str,
        script: &Script,
        extra_config: &str,
    ) -> (Self, ScriptedProvider) {
        Self::with_provider_under(None, plan, tasks_toml, script, extra_config)
    }

    /// [`Self::with_provider`], with the workspace under `/tmp` rather than
    /// the platform's temp dir. On macOS that dir is deep enough that a run's
    /// inject socket (`.roko/runtime/inject/<pid>.sock`) overflows the 104
    /// bytes of a Unix socket path, and `roko plan pause` or `roko inject`
    /// then finds no run listening (1224).
    pub fn with_provider_at_short_path(
        plan: &str,
        tasks_toml: &str,
        script: &Script,
        extra_config: &str,
    ) -> (Self, ScriptedProvider) {
        Self::with_provider_under(
            Some(Path::new("/tmp")),
            plan,
            tasks_toml,
            script,
            extra_config,
        )
    }

    /// [`Self::with_provider`], with the workspace under `base` when given.
    fn with_provider_under(
        base: Option<&Path>,
        plan: &str,
        tasks_toml: &str,
        script: &Script,
        extra_config: &str,
    ) -> (Self, ScriptedProvider) {
        let mut provider = None;
        let workspace = Self::create(base, plan, tasks_toml, extra_config, |root| {
            let installed = ScriptedProvider::install(&root.join("provider"), script);
            let command = installed.command();
            provider = Some(installed);
            command
        });
        (workspace, provider.expect("the provider is installed"))
    }

    /// The workspace, under `base` when given, else the platform's temp dir,
    /// with `install_agent(root)` putting the agent in place and returning
    /// its command.
    fn create(
        base: Option<&Path>,
        plan: &str,
        tasks_toml: &str,
        extra_config: &str,
        install_agent: impl FnOnce(&Path) -> PathBuf,
    ) -> Self {
        let temp = match base {
            Some(base) => tempfile::tempdir_in(base),
            None => tempfile::tempdir(),
        }
        .expect("tempdir");
        // Canonical, so paths roko prints and paths the test builds agree.
        let root = temp.path().canonicalize().expect("canonical tempdir");
        let repo = root.join("repo");
        let home = root.join("home");
        let fixtures = root.join("fixtures");
        for dir in [&repo, &home, &fixtures] {
            fs::create_dir_all(dir).expect("create workspace directory");
        }
        let agent = install_agent(&root);

        seed_minimal_rust_project(&repo);
        fs::write(repo.join(".gitignore"), ".roko/\ntarget/\n").expect("write .gitignore");
        fs::write(
            repo.join("roko.toml"),
            format!(
                r#"agent.default_model = "scripted"
agent.command = {agent:?}
agent.bare_mode = false
providers.scripted-cli.kind = "claude_cli"
providers.scripted-cli.command = {agent:?}
models.scripted.provider = "scripted-cli"
models.scripted.slug = "claude-sonnet-4-6"
models.scripted.context_window = 200000
gates.cargo_fix_enabled = false
runner.worktree_per_task = false
spec_quality.red_on_base = false
{extra_config}"#,
                agent = agent.display().to_string()
            ),
        )
        .expect("write roko.toml");
        let plan_dir = repo.join("plans").join(plan);
        fs::create_dir_all(&plan_dir).expect("create plan directory");
        fs::write(plan_dir.join("plan.md"), format!("# Plan: {plan}\n")).expect("write plan.md");
        fs::write(
            plan_dir.join("tasks.toml"),
            tasks_toml.replace("{fixtures}", &fixtures.display().to_string()),
        )
        .expect("write tasks.toml");

        let workspace = Self {
            _temp: temp,
            root,
            repo,
            home,
            fixtures,
        };
        workspace.git(&["init", "--quiet"]);
        workspace.git(&["config", "user.name", "Scripted Plan"]);
        workspace.git(&["config", "user.email", "scripted-plan@example.invalid"]);
        workspace.git(&["add", "--all"]);
        workspace.git(&["commit", "--quiet", "-m", "seed"]);
        workspace
    }

    /// Run git in the repository with the workspace's home, so the user's
    /// global git configuration (signing, hooks) stays out.
    pub fn git(&self, args: &[&str]) {
        let status = ProcessCommand::new("git")
            .args(["-c", "commit.gpgsign=false"])
            .args(args)
            .current_dir(&self.repo)
            .env("HOME", &self.home)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .status()
            .expect("run git");
        assert!(status.success(), "git {args:?} failed");
    }

    /// `roko --json plan run plans/<plan>` in the repository, as a user whose
    /// home is the workspace's: provider keys, `ROKO_*` variables and the
    /// log and config variables of the invoking environment are removed,
    /// then `env` is added. The exit status is not checked.
    pub fn run_plan(&self, plan: &str, env: &[(&str, &str)]) -> Output {
        let mut command = ProcessCommand::new(cargo_bin("roko"));
        command
            .current_dir(&self.repo)
            .arg("--json")
            .args(["plan", "run", &format!("plans/{plan}"), "--workdir"])
            .arg(&self.repo)
            .env("HOME", &self.home);
        for name in roko_core::child_env::PROVIDER_KEY_VARS {
            command.env_remove(name);
        }
        for (name, _) in std::env::vars_os() {
            if name.to_string_lossy().starts_with("ROKO_") {
                command.env_remove(name);
            }
        }
        for name in ["RUST_LOG", "XDG_CONFIG_HOME", "CLAUDECODE"] {
            command.env_remove(name);
        }
        command.envs(env.iter().copied());
        command.output().expect("run roko plan run")
    }
}

/// The files under `root` whose bytes contain `needle`, except `skip`,
/// sorted. Symlinks are not followed.
pub fn files_containing(root: &Path, needle: &str, skip: &[PathBuf]) -> Vec<PathBuf> {
    let needle = needle.as_bytes();
    let mut found = Vec::new();
    let mut dirs = vec![root.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                dirs.push(path);
            } else if file_type.is_file()
                && !skip.contains(&path)
                && fs::read(&path)
                    .is_ok_and(|bytes| bytes.windows(needle.len()).any(|window| window == needle))
            {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// `text` with every `secret` replaced by `<SECRET>`, for failure messages
/// that must not repeat a secret.
pub fn mask_secret(text: &str, secret: &str) -> String {
    text.replace(secret, "<SECRET>")
}

/// Each line of `path` that contains `secret`, masked, under the path: what a
/// failure message shows about a file a secret leaked into.
pub fn describe_leak(path: &Path, secret: &str) -> String {
    let text = String::from_utf8_lossy(&fs::read(path).unwrap_or_default()).into_owned();
    let lines: Vec<String> = text
        .lines()
        .filter(|line| line.contains(secret))
        .take(3)
        .map(|line| {
            let line = mask_secret(line, secret);
            let shown: String = line.chars().take(300).collect();
            format!("    {shown}")
        })
        .collect();
    format!("  {}\n{}", path.display(), lines.join("\n"))
}

pub fn run_roko(workdir: &Path, args: &[&str]) -> Assert {
    Command::cargo_bin("roko")
        .expect("roko binary")
        .current_dir(workdir)
        .args(args)
        .assert()
}

/// Like `run_roko` but isolates from the user's global config and API keys.
///
/// Sets `HOME` to the workdir so `~/.roko/config.toml` is not found, and
/// removes `ANTHROPIC_API_KEY` / `XDG_CONFIG_HOME` to prevent provider
/// auto-synthesis that would override the test's `roko.toml`.
pub fn run_roko_isolated(workdir: &Path, args: &[&str]) -> Assert {
    Command::cargo_bin("roko")
        .expect("roko binary")
        .current_dir(workdir)
        .args(args)
        .env("HOME", workdir)
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("XDG_CONFIG_HOME")
        .assert()
}

pub fn run_cargo(args: &[&str], workdir: &Path) -> Output {
    ProcessCommand::new("cargo")
        .current_dir(workdir)
        .args(args)
        .output()
        .unwrap_or_else(|err| panic!("spawn cargo {:?}: {err}", args))
}

pub struct ServeHandle {
    pub base_url: String,
    child: Child,
}

pub struct AgentServeConfig<'a> {
    pub agent_id: &'a str,
    pub relay_url: Option<&'a str>,
    pub chain_rpc_url: Option<&'a str>,
    pub identity_registry: Option<&'a str>,
    pub passport_id: Option<&'a str>,
    pub wallet_key: Option<&'a str>,
    pub roko_config: Option<&'a Path>,
}

impl<'a> AgentServeConfig<'a> {
    pub fn new(agent_id: &'a str) -> Self {
        Self {
            agent_id,
            relay_url: None,
            chain_rpc_url: None,
            identity_registry: None,
            passport_id: None,
            wallet_key: None,
            roko_config: None,
        }
    }
}

impl Drop for ServeHandle {
    fn drop(&mut self) {
        if let Ok(None) = self.child.try_wait() {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
    }
}

pub fn spawn_roko_serve_on_random_port(workdir: &Path) -> ServeHandle {
    let port = pick_unused_port();
    let base_url = format!("http://127.0.0.1:{port}");
    let bin = cargo_bin("roko");
    let child = ProcessCommand::new(bin)
        .current_dir(workdir)
        .arg("serve")
        .arg("--bind")
        .arg("127.0.0.1")
        .arg("--port")
        .arg(port.to_string())
        .arg("--workdir")
        .arg(workdir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|err| panic!("spawn roko serve: {err}"));

    ServeHandle { base_url, child }
}

pub fn spawn_roko_agent_serve_on_random_port(
    workdir: &Path,
    config: AgentServeConfig<'_>,
) -> ServeHandle {
    let port = pick_unused_port();
    let bind = format!("127.0.0.1:{port}");
    let base_url = format!("http://{bind}");
    let bin = cargo_bin("roko");
    let mut command = ProcessCommand::new(bin);
    command
        .current_dir(workdir)
        .arg("agent")
        .arg("serve")
        .arg("--agent-id")
        .arg(config.agent_id)
        .arg("--bind")
        .arg(&bind);

    if let Some(relay_url) = config.relay_url {
        command.arg("--relay-url").arg(relay_url);
    }
    if let Some(chain_rpc_url) = config.chain_rpc_url {
        command.arg("--chain-rpc-url").arg(chain_rpc_url);
    }
    if let Some(identity_registry) = config.identity_registry {
        command.arg("--identity-registry").arg(identity_registry);
    }
    if let Some(passport_id) = config.passport_id {
        command.arg("--passport-id").arg(passport_id);
    }
    if let Some(wallet_key) = config.wallet_key {
        command.arg("--wallet-key").arg(wallet_key);
    }
    if let Some(roko_config) = config.roko_config {
        command.env("ROKO_CONFIG", roko_config);
    }

    let child = command
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|err| panic!("spawn roko agent serve: {err}"));

    ServeHandle { base_url, child }
}

/// Wait for `roko serve` to write its launch token to
/// `<workdir>/.roko/runtime/serve.token` (mode 0600) and return it.
pub async fn wait_for_serve_token(workdir: &Path, timeout: Duration) -> String {
    let path = workdir.join(".roko").join("runtime").join("serve.token");
    let deadline = Instant::now() + timeout;
    loop {
        if let Ok(token) = fs::read_to_string(&path) {
            let token = token.trim().to_string();
            if !token.is_empty() {
                return token;
            }
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for the launch token at {}",
            path.display()
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// Like [`wait_for_http_ok`], but authenticates with the server's launch token
/// as a bearer credential (a fresh `roko init` workspace keeps serve auth on).
pub async fn wait_for_http_ok_with_bearer(
    url: &str,
    token: &str,
    timeout: Duration,
) -> reqwest::Response {
    let deadline = Instant::now() + timeout;
    let client = reqwest::Client::builder()
        .connect_timeout(timeout.min(Duration::from_millis(250)))
        .timeout(timeout.min(Duration::from_secs(1)))
        .build()
        .expect("build smoke-test HTTP client");

    loop {
        let last_error = match client.get(url).bearer_auth(token).send().await {
            Ok(response) if response.status().is_success() => return response,
            Ok(response) => format!("unexpected status {}", response.status()),
            Err(err) => err.to_string(),
        };

        assert!(
            Instant::now() < deadline,
            "timed out waiting for {url}: {last_error}"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

pub async fn wait_for_http_ok(url: &str, timeout: Duration) -> reqwest::Response {
    let deadline = Instant::now() + timeout;
    let client = reqwest::Client::builder()
        .connect_timeout(timeout.min(Duration::from_millis(250)))
        .timeout(timeout.min(Duration::from_secs(1)))
        .build()
        .expect("build smoke-test HTTP client");

    loop {
        let last_error = match client.get(url).send().await {
            Ok(response) if response.status().is_success() => return response,
            Ok(response) => format!("unexpected status {}", response.status()),
            Err(err) => err.to_string(),
        };

        assert!(
            Instant::now() < deadline,
            "timed out waiting for {url}: {last_error}"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

pub fn pick_unused_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind random port");
    let port = listener.local_addr().expect("listener addr").port();
    drop(listener);
    port
}
