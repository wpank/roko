//! Test support for bug-7b62ff: a check step that leaves a process running,
//! and whether a stop killed it.

use std::path::Path;
use std::time::Duration;

/// A shell step that starts `sleep 60` in the background, writes its own PID
/// and the sleep's to `pids`, and waits for the sleep: a step that runs until
/// it is stopped, with a child as cargo has its rustc.
pub fn sleeping_step(pids: &Path) -> String {
    format!("sleep 60 & echo $$ $! > '{}'; wait", pids.display())
}

/// The two PIDs [`sleeping_step`] wrote to `pids`, once it wrote them. Waits
/// up to a minute, for a step that runs only after a plan's tasks.
pub async fn wait_for_pids(pids: &Path) -> Vec<u32> {
    for _ in 0..2400 {
        if let Ok(text) = std::fs::read_to_string(pids) {
            let found: Vec<u32> = text
                .split_whitespace()
                .filter_map(|pid| pid.parse().ok())
                .collect();
            if found.len() == 2 {
                return found;
            }
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("the step wrote no PIDs to {}", pids.display());
}

/// Wait until process `pid` is gone; panic if it is still there after 10 s.
pub async fn wait_for_exit(pid: u32) {
    for _ in 0..400 {
        if !alive(pid) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("process {pid} outlived its stopped check");
}

/// Whether process `pid` exists, a zombie included.
fn alive(pid: u32) -> bool {
    let Some(pid) = i32::try_from(pid)
        .ok()
        .and_then(rustix::process::Pid::from_raw)
    else {
        return false;
    };
    !matches!(
        rustix::process::test_kill_process(pid),
        Err(rustix::io::Errno::SRCH)
    )
}
