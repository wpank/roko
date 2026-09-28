//! Regression: a Roko process's startup cleanup must never kill agents that a
//! live Roko process in the same workspace registered, whatever directory it
//! runs from, and must still reap them once that process is gone.
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use roko_agent::process::{
    process_identity, register_spawned_pid, set_registry_root, unregister_pid,
};

/// Turns this test binary into "process A" for the given workspace.
const OWNER_WORKSPACE: &str = "ROKO_TEST_REGISTRY_OWNER_WORKSPACE";
const AGENT_PID_FILE: &str = "agent.pid";

/// Process A: register a live agent stand-in in the workspace's PID registry,
/// report its PID, and stay alive while it runs (reaping it if it dies).
#[test]
#[ignore = "helper process for startup_cleanup_spares_a_live_owners_agents"]
fn registry_owner_process() {
    let Some(workspace) = std::env::var_os(OWNER_WORKSPACE).map(PathBuf::from) else {
        return;
    };
    set_registry_root(&workspace);
    let mut agent = Command::new("sleep")
        .arg("60")
        .spawn()
        .expect("spawn agent stand-in");
    register_spawned_pid(agent.id());
    std::fs::write(workspace.join(AGENT_PID_FILE), agent.id().to_string())
        .expect("report agent pid");
    agent.wait().expect("wait for agent");
    unregister_pid(agent.id());
}

#[test]
fn startup_cleanup_spares_a_live_owners_agents() {
    let workspace = tempfile::tempdir().expect("workspace");
    let workspace = workspace.path();
    std::fs::create_dir_all(workspace.join(".roko")).expect("create .roko");
    let elsewhere = tempfile::tempdir().expect("unrelated cwd");

    let mut owner = OwnerProcess(
        Command::new(std::env::current_exe().expect("test binary"))
            .args([
                "registry_owner_process",
                "--exact",
                "--ignored",
                "--nocapture",
            ])
            .env(OWNER_WORKSPACE, workspace)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn owner process"),
    );
    let agent = Agent::await_reported(&workspace.join(AGENT_PID_FILE));
    let record = owner_record(workspace, owner.0.id()).expect("owner record in the workspace");

    // Process B runs its startup hook in the same workspace, from another cwd.
    run_startup_hook(workspace, elsewhere.path());
    std::thread::sleep(Duration::from_millis(200));
    assert!(
        agent.is_running(),
        "startup cleanup killed a live process's agent"
    );
    assert!(owner.0.try_wait().expect("poll owner").is_none());
    assert!(record.exists(), "a live owner's record was removed");

    // Process A dies without cleaning up: the next startup hook reaps its agent.
    owner.0.kill().expect("kill owner");
    owner.0.wait().expect("reap owner");
    run_startup_hook(workspace, elsewhere.path());
    assert!(
        agent.exits_within(Duration::from_secs(10)),
        "an orphaned agent survived startup cleanup"
    );
    assert!(!record.exists(), "a dead owner's record was kept");
}

/// `roko agent list` runs the process lifecycle hooks before listing.
fn run_startup_hook(workspace: &Path, cwd: &Path) {
    let output = Command::new(env!("CARGO_BIN_EXE_roko"))
        .arg("--repo")
        .arg(workspace)
        .args(["agent", "list"])
        .current_dir(cwd)
        .output()
        .expect("run roko");
    assert!(
        output.status.success(),
        "roko agent list failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn owner_record(workspace: &Path, owner_pid: u32) -> Option<PathBuf> {
    let prefix = format!("{owner_pid}-");
    std::fs::read_dir(workspace.join(".roko/runtime/agent-pids"))
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(&prefix))
        })
}

/// Process A; killed if the test ends early.
struct OwnerProcess(Child);

impl Drop for OwnerProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// The agent stand-in process A registered, pinned by its start fingerprint.
struct Agent {
    pid: u32,
    start: u64,
}

impl Agent {
    fn await_reported(pid_file: &Path) -> Self {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            if let Some(pid) = std::fs::read_to_string(pid_file)
                .ok()
                .and_then(|pid| pid.trim().parse().ok())
                && let Some(identity) = process_identity(pid)
            {
                return Self {
                    pid,
                    start: identity.start,
                };
            }
            assert!(
                Instant::now() < deadline,
                "owner process never reported its agent"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn is_running(&self) -> bool {
        process_identity(self.pid).is_some_and(|identity| identity.start == self.start)
    }

    fn exits_within(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        while self.is_running() {
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        true
    }
}

impl Drop for Agent {
    fn drop(&mut self) {
        if self.is_running() {
            let _ = Command::new("kill")
                .args(["-9", &self.pid.to_string()])
                .status();
        }
    }
}
