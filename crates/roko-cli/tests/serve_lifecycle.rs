#![cfg(unix)]

//! `roko serve --detach`, `status`, `stop` and `restart`, the signals and
//! `--idle-exit`, driven through the real binary (gap-fb1594).
//!
//! Each project is a git checkout set up with `roko init`, with `[server]
//! port` pinned to one port in 6800–6849, and every `roko` (and every serve it
//! starts) runs with a temporary `HOME`, so the machine's serve registry is the
//! test's own. No model runs.

mod common;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

use common::Isolated;
use serde_json::Value;

/// The serves this test started, killed when it ends however it ends: a
/// detached serve outlives the test process otherwise.
#[derive(Default)]
struct Started(Vec<u32>);

impl Drop for Started {
    fn drop(&mut self) {
        for pid in &self.0 {
            if alive(*pid) {
                // SAFETY: kill(2) on a serve this test started.
                #[allow(unsafe_code)]
                unsafe {
                    libc::kill(*pid as libc::pid_t, libc::SIGKILL);
                }
            }
        }
    }
}

fn alive(pid: u32) -> bool {
    // SAFETY: kill(2) with signal 0 only checks that the process exists.
    #[allow(unsafe_code)]
    let found = unsafe { libc::kill(pid as libc::pid_t, 0) } == 0;
    found
}

/// `roko <args>` in `cwd`, isolated from the machine, with `home` as `HOME`.
fn roko(home: &Path, cwd: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_roko"))
        .isolated()
        .env("HOME", home)
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("run roko")
}

/// The one JSON object a `--json` command printed.
fn json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "not one JSON object ({error}):\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn describe(output: &Output) -> String {
    format!(
        "status {:?}\nstdout: {}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// A git checkout set up for roko as `roko init` does, with `[server] port`
/// pinned to `port`.
fn project(root: &Path, name: &str, home: &Path, port: u16) -> PathBuf {
    let dir = root.join(name);
    std::fs::create_dir_all(&dir).expect("project dir");
    let git = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&dir)
        .status()
        .expect("git init");
    assert!(git.success());
    let init = roko(home, &dir, &["init"]);
    assert!(init.status.success(), "roko init: {}", describe(&init));
    let config = dir.join("roko.toml");
    let mut table: toml::Table = std::fs::read_to_string(&config)
        .expect("roko.toml")
        .parse()
        .expect("parse roko.toml");
    table
        .entry("server")
        .or_insert_with(|| toml::Value::Table(toml::Table::new()))
        .as_table_mut()
        .expect("[server] is a table")
        .insert("port".to_string(), toml::Value::Integer(i64::from(port)));
    std::fs::write(
        &config,
        toml::to_string(&table).expect("serialize roko.toml"),
    )
    .expect("pin the port");
    dir.canonicalize().expect("canonical project dir")
}

/// The first port of this worker's range that is free now.
fn free_port() -> u16 {
    (6800..6850)
        .find(|port| std::net::TcpListener::bind(("127.0.0.1", *port)).is_ok())
        .expect("a free port in 6800-6849")
}

/// Wait up to `limit` for `done`.
fn wait_for(limit: Duration, mut done: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + limit;
    while Instant::now() < deadline {
        if done() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    done()
}

fn serve_json(workdir: &Path) -> PathBuf {
    workdir.join(".roko/runtime/serve.json")
}

fn serve_token(workdir: &Path) -> PathBuf {
    workdir.join(".roko/runtime/serve.token")
}

/// The project's entry in the machine's serve registry under `home`.
fn registry_entry(home: &Path, workdir: &Path) -> PathBuf {
    home.join(".roko/runtime/serves").join(format!(
        "{}.json",
        roko_serve::endpoint::registry_key(workdir)
    ))
}

/// Whether some process holds the project's workspace lock.
fn lock_held(workdir: &Path) -> bool {
    roko_cli::workspace_lock::workspace_lock_is_held(&workdir.join(".roko"))
}

/// What a serve leaves while it runs, and removes when it stops: serve.json,
/// serve.token and its registry entry.
fn advertised(home: &Path, workdir: &Path) -> [bool; 3] {
    [
        serve_json(workdir).exists(),
        serve_token(workdir).exists(),
        registry_entry(home, workdir).exists(),
    ]
}

/// `roko serve --detach --json` for `workdir`: the A1 object.
fn detach(home: &Path, workdir: &Path, extra: &[&str], started: &mut Started) -> Value {
    let workdir_arg = workdir.to_str().expect("utf-8 path");
    let mut args = vec!["serve", "--detach", "--json", "--workdir", workdir_arg];
    args.extend_from_slice(extra);
    let output = roko(home, workdir, &args);
    assert!(output.status.success(), "detach: {}", describe(&output));
    let value = json(&output);
    assert_eq!(value["ok"], true, "{value}");
    let pid = value["pid"].as_u64().expect("pid") as u32;
    if !started.0.contains(&pid) {
        started.0.push(pid);
    }
    value
}

/// `roko serve <args> --json`: the status and the object.
fn serve_cmd(home: &Path, cwd: &Path, args: &[&str]) -> (Option<i32>, Value) {
    let mut all = vec!["serve"];
    all.extend_from_slice(args);
    all.push("--json");
    let output = roko(home, cwd, &all);
    (output.status.code(), json(&output))
}

#[test]
fn serve_detach_lifecycle_end_to_end() {
    let temp = tempfile::tempdir().expect("tempdir");
    let home = temp.path().join("home");
    std::fs::create_dir_all(&home).expect("home");
    let pinned = free_port();
    let alpha = project(temp.path(), "alpha", &home, pinned);
    let beta = project(temp.path(), "beta", &home, pinned);
    let mut started = Started::default();

    // A folder that is not a directory is refused, and not created.
    let missing = temp.path().join("missing");
    let output = roko(
        &home,
        temp.path(),
        &[
            "serve",
            "--detach",
            "--json",
            "--workdir",
            missing.to_str().expect("utf-8"),
        ],
    );
    assert_eq!(output.status.code(), Some(2), "{}", describe(&output));
    assert_eq!(json(&output)["error"]["code"], "not_a_workspace");
    assert!(!missing.exists(), "the missing workdir was created");

    // A folder roko is not set up in (no roko.toml, no provider): roko serve
    // would refuse it, so --detach fails fast with the fix.
    let bare = temp.path().join("bare");
    std::fs::create_dir_all(&bare).expect("bare dir");
    let begun = Instant::now();
    let output = roko(
        &home,
        &bare,
        &[
            "serve",
            "--detach",
            "--json",
            "--workdir",
            bare.to_str().expect("utf-8"),
        ],
    );
    assert_eq!(output.status.code(), Some(2), "{}", describe(&output));
    let refused = json(&output);
    assert_eq!(refused["ok"], false, "{refused}");
    assert_eq!(refused["error"]["code"], "not_initialized", "{refused}");
    assert!(
        refused["error"]["message"]
            .as_str()
            .is_some_and(|message| message.contains("roko init")),
        "{refused}"
    );
    assert!(
        begun.elapsed() < Duration::from_secs(30),
        "it waited out the timeout"
    );

    // Detach starts alpha's serve on its configured port and returns once it
    // answers as itself.
    let first = detach(&home, &alpha, &[], &mut started);
    assert_eq!(first["started"], true, "{first}");
    assert_eq!(first["port"], u64::from(pinned), "{first}");
    assert_eq!(first["workdir"], alpha.to_str().expect("utf-8"), "{first}");
    assert_eq!(
        first["url"],
        format!("http://127.0.0.1:{pinned}"),
        "{first}"
    );
    let alpha_instance = first["instance_id"]
        .as_str()
        .expect("instance id")
        .to_string();
    assert_eq!(alpha_instance.len(), 36, "a UUID: {first}");
    assert!(first["git_sha"].is_string(), "{first}");
    assert!(
        first["exe"]
            .as_str()
            .is_some_and(|exe| exe.ends_with("roko")),
        "{first}"
    );
    assert_eq!(first["idle_exit_secs"], Value::Null, "{first}");
    assert_eq!(
        first["args"]["port_requested"],
        u64::from(pinned),
        "{first}"
    );
    assert_eq!(
        first["log"],
        alpha.join(".roko/logs/serve.log").to_str().expect("utf-8"),
        "{first}"
    );
    let alpha_pid = first["pid"].as_u64().expect("pid") as u32;
    assert_eq!(advertised(&home, &alpha), [true, true, true]);

    // Twice: the same serve, not started again.
    let again = detach(&home, &alpha, &[], &mut started);
    assert_eq!(again["started"], false, "{again}");
    assert_eq!(again["instance_id"], alpha_instance.as_str(), "{again}");
    assert_eq!(again["pid"], u64::from(alpha_pid), "{again}");

    // Status says it runs, with its build and runs.
    let (code, status) = serve_cmd(
        &home,
        &alpha,
        &["status", "--workdir", alpha.to_str().expect("utf-8")],
    );
    assert_eq!(code, Some(0), "{status}");
    let serve = &status["serves"][0];
    assert_eq!(serve["state"], "running", "{status}");
    assert_eq!(serve["instance_id"], alpha_instance.as_str(), "{status}");
    assert_eq!(serve["active_runs"], 0, "{status}");
    assert_eq!(serve["outdated"], false, "{status}");

    // Beta pins the same port, which alpha holds: its serve gets one the OS
    // picks, and serve.json records it.
    let second = detach(&home, &beta, &[], &mut started);
    assert_eq!(second["started"], true, "{second}");
    let beta_port = second["port"].as_u64().expect("port");
    assert!(beta_port != 0 && beta_port != u64::from(pinned), "{second}");
    assert_eq!(second["args"]["port_requested"], 0, "{second}");
    let beta_pid = second["pid"].as_u64().expect("pid") as u32;

    // --all finds both through the registry.
    let (code, all) = serve_cmd(&home, temp.path(), &["status", "--all"]);
    assert_eq!(code, Some(0), "{all}");
    let mut workdirs: Vec<&str> = all["serves"]
        .as_array()
        .expect("serves")
        .iter()
        .map(|serve| serve["workdir"].as_str().expect("workdir"))
        .collect();
    workdirs.sort_unstable();
    let mut expected = vec![
        alpha.to_str().expect("utf-8"),
        beta.to_str().expect("utf-8"),
    ];
    expected.sort_unstable();
    assert_eq!(workdirs, expected, "{all}");

    // Restart: a new process with a new instance id, on its old port.
    let (code, restarted) = serve_cmd(
        &home,
        &alpha,
        &["restart", "--workdir", alpha.to_str().expect("utf-8")],
    );
    assert_eq!(code, Some(0), "{restarted}");
    let renewed = &restarted["restarted"][0];
    let renewed_pid = renewed["pid"].as_u64().expect("pid") as u32;
    started.0.push(renewed_pid);
    assert_ne!(
        renewed["instance_id"],
        alpha_instance.as_str(),
        "{restarted}"
    );
    assert_ne!(renewed_pid, alpha_pid, "{restarted}");
    assert_eq!(renewed["port"], u64::from(pinned), "{restarted}");
    assert!(
        wait_for(Duration::from_secs(10), || !alive(alpha_pid)),
        "the old serve is still running"
    );

    // Stop: the serve, its files and its registry entry are gone.
    let (code, stopped) = serve_cmd(
        &home,
        &alpha,
        &["stop", "--workdir", alpha.to_str().expect("utf-8")],
    );
    assert_eq!(code, Some(0), "{stopped}");
    assert_eq!(
        stopped["stopped"][0]["pid"],
        u64::from(renewed_pid),
        "{stopped}"
    );
    assert!(
        wait_for(Duration::from_secs(10), || !alive(renewed_pid)),
        "the stopped serve is still running"
    );
    assert_eq!(advertised(&home, &alpha), [false, false, false]);
    assert!(!lock_held(&alpha));
    let (code, status) = serve_cmd(
        &home,
        &alpha,
        &["status", "--workdir", alpha.to_str().expect("utf-8")],
    );
    assert_eq!(code, Some(1), "{status}");
    assert_eq!(status["serves"][0]["state"], "stopped", "{status}");
    let (code, stopped) = serve_cmd(
        &home,
        &alpha,
        &["stop", "--workdir", alpha.to_str().expect("utf-8")],
    );
    assert_eq!(code, Some(0), "{stopped}");
    assert_eq!(stopped["not_running"][0], alpha.to_str().expect("utf-8"));

    // SIGTERM: beta's serve shuts down gracefully and removes its own files.
    assert_eq!(advertised(&home, &beta), [true, true, true]);
    // SAFETY: kill(2) on the serve this test started.
    #[allow(unsafe_code)]
    unsafe {
        libc::kill(beta_pid as libc::pid_t, libc::SIGTERM);
    }
    assert!(
        wait_for(Duration::from_secs(20), || !alive(beta_pid)),
        "SIGTERM did not stop the serve"
    );
    assert_eq!(advertised(&home, &beta), [false, false, false]);
    assert!(!lock_held(&beta));
    let lock_pid = std::fs::read_to_string(beta.join(".roko/runtime/roko.lock")).expect("lock");
    assert!(
        lock_pid.trim().is_empty(),
        "the lock still names {lock_pid}"
    );
    let log = std::fs::read_to_string(beta.join(".roko/logs/serve.log")).expect("serve.log");
    assert!(log.contains("SIGTERM: shutting down"), "{log}");

    // A crash leaves the files; status reports the serve stale and removes
    // them.
    let crashed = detach(&home, &beta, &[], &mut started);
    let crashed_pid = crashed["pid"].as_u64().expect("pid") as u32;
    // SAFETY: kill(2) on the serve this test started.
    #[allow(unsafe_code)]
    unsafe {
        libc::kill(crashed_pid as libc::pid_t, libc::SIGKILL);
    }
    assert!(wait_for(Duration::from_secs(10), || !alive(crashed_pid)));
    assert_eq!(advertised(&home, &beta), [true, true, true]);
    let (code, status) = serve_cmd(
        &home,
        &beta,
        &["status", "--workdir", beta.to_str().expect("utf-8")],
    );
    assert_eq!(code, Some(1), "{status}");
    assert_eq!(status["serves"][0]["state"], "stale", "{status}");
    assert_eq!(status["serves"][0]["cleaned"], true, "{status}");
    assert_eq!(advertised(&home, &beta), [false, false, false]);

    // --idle-exit: a serve nobody uses exits by itself, cleanly.
    let idle = detach(&home, &alpha, &["--idle-exit", "2"], &mut started);
    assert_eq!(idle["idle_exit_secs"], 2, "{idle}");
    let idle_pid = idle["pid"].as_u64().expect("pid") as u32;
    assert!(
        wait_for(Duration::from_secs(30), || !alive(idle_pid)),
        "the idle serve did not exit"
    );
    assert_eq!(advertised(&home, &alpha), [false, false, false]);
    assert!(!lock_held(&alpha));
    let (code, all) = serve_cmd(&home, temp.path(), &["status", "--all"]);
    assert_eq!(code, Some(0), "{all}");
    assert_eq!(all["serves"], serde_json::json!([]), "{all}");
}
