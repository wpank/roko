#![cfg(unix)]

//! Canary C2 (assessment W8, gate G2; epic spec-ba7bea): in one real plan
//! run, no provider key reaches the agent, a verify command or any file roko
//! writes, and the git guard roko hands the agent denies destructive
//! commands.
//!
//! There are two kinds of canary: provider keys in the home directory's
//! `~/.roko/.env`, which roko loads at startup, and a provider key exported
//! in roko's own environment. The shared scripted provider logs each call's
//! argv, prompt and environment outside the repository; the verify step
//! records its environment there too. Probe variables that `[agent]` and
//! `[gates]` pass through show that both environments came from roko's, so
//! a missing canary means roko kept it out.
//!
//! `agent_tool_shells_exclude_provider_keys` covers the shells of roko's own
//! agent tools (`run_tests`, `bash`), which OpenAI-compatible models reach
//! through roko's tool loop, and `agent_tool_loop_refuses_git_stash` covers
//! the git guard of that tool loop.

mod common;

use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::{Arc, Mutex};

use common::scripted_provider::{Script, ScriptedProvider, Turn};
use common::{ScriptedPlanWorkspace, describe_leak, files_containing, mask_secret};
use serde_json::Value;

const PLAN: &str = "c2-canary";

/// Provider keys in `~/.roko/.env`, which roko loads at startup.
const HOME_ENV_KEYS: [(&str, &str); 2] = [
    ("ANTHROPIC_API_KEY", "c2-canary-home-anthropic-7f3a91"),
    ("OPENAI_API_KEY", "c2-canary-home-openai-19be42"),
];

/// A provider key exported in roko's own environment. It is not a Claude
/// key: the Claude CLI keeps an exported key of its own by design
/// (`roko_core::child_env::CredentialScrub`).
const EXPORTED_KEY: (&str, &str) = ("GEMINI_API_KEY", "c2-canary-exported-gemini-5d2e07");

/// Passed through to the agent and to verify commands by `roko.toml`: one
/// from `~/.roko/.env`, one exported.
const HOME_PROBE: (&str, &str) = ("C2_PROBE_HOME", "c2-probe-home-value");
const EXPORTED_PROBE: (&str, &str) = ("C2_PROBE_EXPORTED", "c2-probe-exported-value");

/// Every canary value.
fn canaries() -> Vec<&'static str> {
    HOME_ENV_KEYS
        .iter()
        .map(|(_, value)| *value)
        .chain([EXPORTED_KEY.1])
        .collect()
}

/// The agent appends to `NOTES.md` so the attempt has a diff, and reports
/// success. Its reply quotes a home key, as an agent's would that found the
/// key some other way: roko must redact it from everything it writes
/// (gap-5f4852).
fn script() -> Script {
    let leaked = HOME_ENV_KEYS[0].1;
    Script::new().task(
        "T1",
        [Turn::reply()
            .append("NOTES.md", "attempt\n")
            .text(&format!("Updated NOTES.md; the key is {leaked}."))],
    )
}

/// One task; its verify step records the environment it runs with.
const TASKS: &str = r#"[meta]
plan = "c2-canary"
iteration = 1
total = 1
done = 0
status = "ready"
max_parallel = 1
estimated_total_minutes = 1
skip_enrichment = true

[[task]]
id = "T1"
title = "Update the notes"
description = "Append a line to NOTES.md."
role = "implementer"
status = "ready"
tier = "focused"
model_hint = "scripted"
files = ["NOTES.md"]
allowed_tools = []
denied_tools = []
mcp_servers = []
depends_on = []
depends_on_plan = []
acceptance = []
verify = [{ phase = "structural", command = "env >> {fixtures}/gate-env.txt", fail_msg = "the environment was not recorded" }]
timeout_secs = 30
max_retries = 0
"#;

/// A workspace whose home holds `~/.roko/.env` with the home canaries and
/// probe, and whose `roko.toml` passes the probes through to the agent, which
/// plays [`script`].
fn workspace(
    plan: &str,
    tasks: &str,
    extra_config: &str,
) -> (ScriptedPlanWorkspace, ScriptedProvider) {
    let config = format!(
        "agent.env_passthrough = [\"C2_PROBE_*\"]\ngates.env_passthrough = [\"C2_PROBE_*\"]\n{extra_config}"
    );
    let (workspace, provider) =
        ScriptedPlanWorkspace::with_provider(plan, tasks, &script(), &config);
    let home_env = workspace.home.join(".roko").join(".env");
    fs::create_dir_all(home_env.parent().expect("~/.roko")).expect("create ~/.roko");
    let mut text = String::new();
    for (name, value) in HOME_ENV_KEYS.iter().chain([&HOME_PROBE]) {
        text.push_str(&format!("{name}={value}\n"));
    }
    fs::write(&home_env, text).expect("write ~/.roko/.env");
    (workspace, provider)
}

/// Run the plan with the exported canary and probe in roko's environment.
fn run(workspace: &ScriptedPlanWorkspace, plan: &str, extra_env: &[(&str, &str)]) -> Output {
    let mut env = vec![EXPORTED_KEY, EXPORTED_PROBE, ("ROKO_LOG", "roko=debug")];
    env.extend_from_slice(extra_env);
    workspace.run_plan(plan, &env)
}

/// `output`'s stdout and stderr with every canary masked, for failure
/// messages.
fn context(output: &Output) -> String {
    let mut text = format!(
        "exit: {}\nstdout:\n{}\nstderr (last 4000 bytes):\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        {
            let stderr = String::from_utf8_lossy(&output.stderr);
            stderr[stderr.len().saturating_sub(4000)..].to_string()
        }
    );
    for canary in canaries() {
        text = mask_secret(&text, canary);
    }
    text
}

fn read(path: &Path, output: &Output) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("read {}: {error}\n{}", path.display(), context(output)))
}

/// Assert that no canary occurs in `text`, which `what` describes.
fn assert_no_canary(text: &str, what: &str) {
    for canary in canaries() {
        assert!(!text.contains(canary), "a provider key reached {what}");
    }
}

/// Assert that no file under `root`, except `skip`, holds a canary.
fn assert_no_canary_in_files(root: &Path, skip: &[PathBuf]) {
    for canary in canaries() {
        let leaks = files_containing(root, canary, skip);
        assert!(
            leaks.is_empty(),
            "a provider key reached {} file(s):\n{}",
            leaks.len(),
            leaks
                .iter()
                .map(|path| describe_leak(path, canary))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

/// Run a Claude Code hook command the way Claude Code does (`sh -c`, the
/// tool input on stdin) and return its exit code; 2 blocks the tool call.
fn run_hook(hook: &str, cwd: &Path, command: &str, env: &[(&str, &Path)]) -> Option<i32> {
    let payload = serde_json::json!({ "cwd": cwd, "tool_input": { "command": command } });
    // An absolute path, so PATH can be replaced.
    let mut shell = Command::new("/bin/sh");
    shell
        .arg("-c")
        .arg(hook)
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    for (name, value) in env {
        shell.env(name, value);
    }
    let mut child = shell.spawn().expect("spawn hook");
    // The hook may exit before it reads its input (no python3).
    let _ = child
        .stdin
        .take()
        .expect("hook stdin")
        .write_all(payload.to_string().as_bytes());
    child.wait().expect("wait for hook").code()
}

#[test]
fn secrets_and_git_guard_canary() {
    let (workspace, provider) = workspace(PLAN, TASKS, "");
    let output = run(&workspace, PLAN, &[]);
    let fixtures = &workspace.fixtures;

    // Not vacuous: the agent and the verify step ran, and each saw the
    // probes, so their environments came from roko's own.
    let calls = provider.calls_for("T1");
    let attempt = calls
        .first()
        .unwrap_or_else(|| panic!("the agent was not called for T1\n{}", context(&output)));
    let agent_env: String = calls.iter().map(|call| call.env.as_str()).collect();
    let gate_env = read(&fixtures.join("gate-env.txt"), &output);
    for (env, what) in [(&agent_env, "agent"), (&gate_env, "verify step")] {
        for (name, value) in [HOME_PROBE, EXPORTED_PROBE] {
            assert!(
                env.lines().any(|line| line == format!("{name}={value}")),
                "the {what} did not get the passed-through {name}\n{}",
                context(&output)
            );
        }
    }

    // No key reached the agent (environment, prompt, argv) or the verify
    // step: every call the provider logged and every file the verify step
    // wrote is checked. The provider's turns, which hold the key it prints,
    // are not.
    assert_no_canary(&agent_env, "the agent's environment");
    assert_no_canary(&gate_env, "the verify step's environment");
    assert_no_canary_in_files(&provider.calls_dir(), &[]);
    assert_no_canary_in_files(fixtures, &[]);

    // Nor any file roko wrote, in the repository (`.roko/` included) or in
    // its home apart from the `.env` itself, nor its output.
    assert_no_canary_in_files(&workspace.repo, &[]);
    assert_no_canary_in_files(
        &workspace.home,
        &[workspace.home.join(".roko").join(".env")],
    );
    assert_no_canary(&String::from_utf8_lossy(&output.stdout), "stdout");
    assert_no_canary(&String::from_utf8_lossy(&output.stderr), "stderr");

    // The agent's argv: isolated setting sources (gap-8be530) and the
    // guard hooks in `--settings`.
    let argv = &attempt.argv;
    assert!(
        argv.iter().any(|arg| arg == "--setting-sources"),
        "the agent was not started with --setting-sources: {argv:?}"
    );
    let settings = argv
        .iter()
        .position(|arg| arg == "--settings")
        .and_then(|index| argv.get(index + 1))
        .expect("the agent was not started with --settings");
    let settings: Value = serde_json::from_str(settings).expect("--settings is JSON");
    let bash_hook = settings["hooks"]["PreToolUse"]
        .as_array()
        .expect("PreToolUse hooks")
        .iter()
        .find(|entry| entry["matcher"] == "Bash")
        .and_then(|entry| entry["hooks"][0]["command"].as_str())
        .expect("a Bash hook command");

    let repo = &workspace.repo;
    let home = [("HOME", workspace.home.as_path())];
    for denied in [
        "git reset --hard",
        "git clean -fdx",
        "git stash",
        "cd x && git checkout main",
        "cat ~/.roko/.env",
    ] {
        assert_eq!(
            run_hook(bash_hook, repo, denied, &home),
            Some(2),
            "the guard let `{denied}` through"
        );
    }
    // The guard is not a blanket refusal...
    for allowed in ["git status", "cargo test"] {
        assert_eq!(
            run_hook(bash_hook, repo, allowed, &home),
            Some(0),
            "the guard refused `{allowed}`"
        );
    }
    // ...and it fails closed without python3.
    let no_python = workspace.root.join("no-python");
    fs::create_dir_all(&no_python).expect("empty PATH directory");
    assert_eq!(
        run_hook(
            bash_hook,
            repo,
            "echo ok",
            &[("HOME", workspace.home.as_path()), ("PATH", &no_python)]
        ),
        Some(2),
        "the guard let a command through without python3 on PATH"
    );
}

// ── roko's own agent tools ───────────────────────────────────────────────────

/// The key the fake OpenAI-compatible provider is configured with. The
/// provider may receive it; a tool shell may not.
const PROVIDER_KEY: (&str, &str) = ("C2_FAKE_OPENAI_KEY", "c2-canary-provider-key-0b8c3d");

/// A tool call a fake model makes: its id, the tool's name and the JSON
/// arguments.
type FakeToolCall = (&'static str, &'static str, String);

/// A fake model's `bash` call that runs `command`.
fn bash_call(id: &'static str, command: &str) -> FakeToolCall {
    (
        id,
        "bash",
        serde_json::json!({ "command": command }).to_string(),
    )
}

/// An OpenAI-compatible chat server on a free local port. A request without
/// a tool result gets calls to `run_tests` (a Makefile target) and `bash`;
/// any other gets a final answer.
fn spawn_tool_calling_server(fixtures: &Path) -> String {
    let bash_command = format!(
        "env > {}/bash-env.txt; printf 'attempt\\n' >> NOTES.md",
        fixtures.display()
    );
    let calls = vec![
        (
            "call_run_tests",
            "run_tests",
            r#"{"build":"make"}"#.to_string(),
        ),
        bash_call("call_bash", &bash_command),
    ];
    spawn_chat_server(vec![calls], Arc::default())
}

/// An OpenAI-compatible chat server on a free local port that plays `turns`,
/// one per model reply: a request gets the calls of the first turn that its
/// tool results do not answer yet, and a final answer once they answer every
/// turn. A streaming request is answered with server-sent events, one tool
/// call per chunk as the providers send them. Each request is appended to
/// `requests`. Returns the base URL; the server thread lives as long as the
/// test.
fn spawn_chat_server(turns: Vec<Vec<FakeToolCall>>, requests: Arc<Mutex<Vec<String>>>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake provider");
    let base_url = format!("http://{}", listener.local_addr().expect("address"));
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let request = read_http_request(&mut stream);
            let streaming = request.contains("\"stream\":true");
            // The tool results not yet matched to a turn.
            let mut results = request.matches("\"role\":\"tool\"").count();
            let tool_calls: &[FakeToolCall] = turns
                .iter()
                .find(|calls| {
                    let unanswered = results == 0;
                    results = results.saturating_sub(calls.len());
                    unanswered
                })
                .map(Vec::as_slice)
                .unwrap_or_default();
            requests.lock().expect("request log").push(request);
            let answered = tool_calls.is_empty();
            let finish_reason = if answered { "stop" } else { "tool_calls" };
            let usage = serde_json::json!({ "prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15 });
            let (content_type, body) = if streaming {
                let mut chunks = Vec::new();
                if answered {
                    chunks.push(serde_json::json!({ "choices": [{ "index": 0, "delta": { "role": "assistant", "content": "done" } }] }));
                } else {
                    for (index, (id, name, arguments)) in tool_calls.iter().enumerate() {
                        chunks.push(serde_json::json!({ "choices": [{ "index": 0, "delta": { "tool_calls": [{
                            "index": index, "id": id, "type": "function",
                            "function": { "name": name, "arguments": arguments }
                        }] } }] }));
                    }
                }
                chunks.push(serde_json::json!({ "choices": [{ "index": 0, "delta": {}, "finish_reason": finish_reason }] }));
                chunks.push(serde_json::json!({ "choices": [], "usage": usage }));
                let mut body: String = chunks
                    .iter()
                    .map(|chunk| format!("data: {chunk}\n\n"))
                    .collect();
                body.push_str("data: [DONE]\n\n");
                ("text/event-stream", body)
            } else {
                let message = if answered {
                    serde_json::json!({ "role": "assistant", "content": "done" })
                } else {
                    let calls: Vec<Value> = tool_calls
                        .iter()
                        .map(|(id, name, arguments)| {
                            serde_json::json!({ "id": id, "type": "function",
                                "function": { "name": name, "arguments": arguments } })
                        })
                        .collect();
                    serde_json::json!({ "role": "assistant", "content": null, "tool_calls": calls })
                };
                let body = serde_json::json!({
                    "id": "chatcmpl-c2",
                    "object": "chat.completion",
                    "choices": [{ "index": 0, "message": message, "finish_reason": finish_reason }],
                    "usage": usage,
                });
                ("application/json", body.to_string())
            };
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
        }
    });
    base_url
}

/// One HTTP request's head and body, read up to its `Content-Length`.
fn read_http_request(stream: &mut std::net::TcpStream) -> String {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 8192];
    loop {
        let Ok(read) = stream.read(&mut chunk) else {
            break;
        };
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);
        let text = String::from_utf8_lossy(&buffer);
        if let Some(head_end) = text.find("\r\n\r\n") {
            let length = text[..head_end]
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())?
                })
                .unwrap_or(0);
            if buffer.len() >= head_end + 4 + length {
                break;
            }
        }
    }
    String::from_utf8_lossy(&buffer).into_owned()
}

/// Point `workspace`'s agent at the fake OpenAI-compatible server at
/// `base_url`, whose key is [`PROVIDER_KEY`]. The caller commits the change.
fn use_fake_openai(workspace: &ScriptedPlanWorkspace, base_url: &str) {
    let config_path = workspace.repo.join("roko.toml");
    let mut config = fs::read_to_string(&config_path).expect("read roko.toml");
    config = config.replace(
        "agent.default_model = \"scripted\"",
        "agent.default_model = \"fake-openai\"",
    );
    config.push_str(&format!(
        "providers.fake-openai.kind = \"openai_compat\"\n\
         providers.fake-openai.base_url = \"{base_url}/v1\"\n\
         providers.fake-openai.api_key_env = \"{}\"\n\
         models.fake-openai.provider = \"fake-openai\"\n\
         models.fake-openai.slug = \"fake-model\"\n\
         models.fake-openai.context_window = 128000\n\
         models.fake-openai.supports_tools = true\n",
        PROVIDER_KEY.0
    ));
    fs::write(&config_path, config).expect("write roko.toml");
}

/// roko's own tool shells (`run_tests` and `bash`, which OpenAI-compatible
/// models reach through roko's tool loop) see no provider key either, the
/// provider's own included.
#[test]
fn agent_tool_shells_exclude_provider_keys() {
    const TOOLS_PLAN: &str = "c2-tools";
    let tasks = TASKS
        .replace("c2-canary", TOOLS_PLAN)
        .replace("model_hint = \"scripted\"", "model_hint = \"fake-openai\"")
        .replace(
            "allowed_tools = []",
            "allowed_tools = [\"run_tests\", \"bash\"]",
        )
        .replace("env >> {fixtures}/gate-env.txt", "test -d .");
    // The fake provider's config is set once its port is known.
    let (workspace, _provider) = workspace(TOOLS_PLAN, &tasks, "");
    let base_url = spawn_tool_calling_server(&workspace.fixtures);
    use_fake_openai(&workspace, &base_url);
    fs::write(
        workspace.repo.join("Makefile"),
        format!(
            "test:\n\tenv > {}/run-tests-env.txt\n",
            workspace.fixtures.display()
        ),
    )
    .expect("write Makefile");
    workspace.git(&["add", "--all"]);
    workspace.git(&["commit", "--quiet", "-m", "fake OpenAI-compatible provider"]);

    let output = run(&workspace, TOOLS_PLAN, &[PROVIDER_KEY]);
    let fixtures = &workspace.fixtures;
    let mut leaky_tools = Vec::new();
    for (file, tool) in [("run-tests-env.txt", "run_tests"), ("bash-env.txt", "bash")] {
        let env = read(&fixtures.join(file), &output);
        assert!(
            env.lines().any(|line| line.starts_with("PATH=")),
            "the {tool} shell recorded no environment\n{}",
            context(&output)
        );
        if canaries()
            .into_iter()
            .chain([PROVIDER_KEY.1])
            .any(|key| env.contains(key))
        {
            leaky_tools.push(tool);
        }
    }
    assert!(
        leaky_tools.is_empty(),
        "a provider key reached the shells of {leaky_tools:?}"
    );
}

/// The contents of the tool results in `requests`, the HTTP requests a fake
/// chat server received.
fn tool_results(requests: &[String]) -> Vec<String> {
    let mut results = Vec::new();
    for request in requests {
        let Some((_, body)) = request.split_once("\r\n\r\n") else {
            continue;
        };
        let Ok(body) = serde_json::from_str::<Value>(body) else {
            continue;
        };
        for message in body["messages"].as_array().into_iter().flatten() {
            if message["role"] == "tool" {
                results.push(match &message["content"] {
                    Value::String(text) => text.clone(),
                    other => other.to_string(),
                });
            }
        }
    }
    results
}

/// `git <args>`'s standard output in `workspace`'s repository, run with the
/// workspace's home as [`ScriptedPlanWorkspace::git`] runs git.
fn git_stdout(workspace: &ScriptedPlanWorkspace, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(&workspace.repo)
        .env("HOME", &workspace.home)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .expect("run git");
    assert!(output.status.success(), "git {args:?} failed");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// roko's own tool loop refuses `git stash` and `git clean -fdx` from an
/// OpenAI-compatible model in the operator's checkout (the shared working
/// tree): the model is told each command was refused, the operator's
/// uncommitted edit and untracked file survive the run, and nothing is
/// stashed (G08).
#[test]
fn agent_tool_loop_refuses_git_stash() {
    const GUARD_PLAN: &str = "c2-git-guard";
    let tasks = TASKS
        .replace("c2-canary", GUARD_PLAN)
        .replace("model_hint = \"scripted\"", "model_hint = \"fake-openai\"")
        .replace("allowed_tools = []", "allowed_tools = [\"bash\"]")
        .replace("env >> {fixtures}/gate-env.txt", "test -d .");
    let (workspace, _provider) = workspace(GUARD_PLAN, &tasks, "");
    // The model runs `git stash`, then `git clean -fdx`, then ends its turn.
    let requests = Arc::new(Mutex::new(Vec::new()));
    let base_url = spawn_chat_server(
        vec![
            vec![bash_call("call_stash", "git stash")],
            vec![bash_call("call_clean", "git clean -fdx")],
        ],
        Arc::clone(&requests),
    );
    use_fake_openai(&workspace, &base_url);
    let edited = workspace.repo.join("operator.txt");
    fs::write(&edited, "committed\n").expect("write operator.txt");
    workspace.git(&["add", "--all"]);
    workspace.git(&["commit", "--quiet", "-m", "fake OpenAI-compatible provider"]);
    // The operator's work in progress in the checkout the agent runs in: an
    // edit to a tracked file and an untracked file.
    fs::write(&edited, "committed\noperator edit\n").expect("edit operator.txt");
    let untracked = workspace.repo.join("untracked.txt");
    fs::write(&untracked, "operator notes\n").expect("write untracked.txt");

    let output = run(&workspace, GUARD_PLAN, &[PROVIDER_KEY]);

    let results = tool_results(&requests.lock().expect("request log"));
    for command in ["git stash", "git clean -fdx"] {
        assert!(
            results
                .iter()
                .any(|result| result.contains("command not allowed") && result.contains(command)),
            "the tool loop did not refuse `{command}`; the model got {results:?}\n{}",
            context(&output)
        );
    }
    assert_eq!(
        read(&edited, &output),
        "committed\noperator edit\n",
        "the operator's uncommitted edit was lost\n{}",
        context(&output)
    );
    assert_eq!(
        read(&untracked, &output),
        "operator notes\n",
        "the operator's untracked file changed\n{}",
        context(&output)
    );
    let stashes = git_stdout(&workspace, &["stash", "list"]);
    assert!(
        stashes.trim().is_empty(),
        "the operator's work was stashed: {stashes}"
    );
}
