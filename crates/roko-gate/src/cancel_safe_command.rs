use std::io;
use std::process::{Output, Stdio};

use tokio::process::Command;

/// Run a command whose whole process group is terminated if this future is dropped.
pub(crate) async fn output(mut command: Command) -> io::Result<Output> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    configure_process_group(&mut command);

    let child = command.spawn()?;
    let mut guard = ProcessGroupGuard::new(child.id());
    let result = child.wait_with_output().await;
    guard.disarm();
    result
}

struct ProcessGroupGuard {
    pid: Option<u32>,
}

impl ProcessGroupGuard {
    const fn new(pid: Option<u32>) -> Self {
        Self { pid }
    }

    fn disarm(&mut self) {
        self.pid = None;
    }
}

impl Drop for ProcessGroupGuard {
    fn drop(&mut self) {
        terminate_process_group(self.pid);
    }
}

#[cfg(unix)]
#[allow(unsafe_code, clippy::cast_possible_wrap)]
fn configure_process_group(command: &mut Command) {
    // SAFETY: setpgid is async-signal-safe and runs in the child before exec.
    unsafe {
        command.pre_exec(|| {
            if libc::setpgid(0, 0) == 0 {
                Ok(())
            } else {
                Err(io::Error::last_os_error())
            }
        });
    }
}

#[cfg(not(unix))]
fn configure_process_group(_command: &mut Command) {}

#[cfg(unix)]
#[allow(unsafe_code, clippy::cast_possible_wrap)]
fn terminate_process_group(pid: Option<u32>) {
    if let Some(pid) = pid {
        // SAFETY: a negative PID targets the process group created above.
        unsafe {
            libc::kill(-(pid as i32), libc::SIGKILL);
        }
    }
}

#[cfg(not(unix))]
fn terminate_process_group(_pid: Option<u32>) {}

#[cfg(all(test, unix))]
mod tests {
    use std::path::Path;
    use std::time::Duration;

    use tempfile::TempDir;

    use super::*;

    #[tokio::test]
    async fn abort_kills_root_and_descendant_pids() {
        let temp = TempDir::new().expect("tempdir");
        let root_path = temp.path().join("root.pid");
        let child_path = temp.path().join("child.pid");
        let script = format!(
            "echo $$ > '{}'; sleep 30 & echo $! > '{}'; wait",
            root_path.display(),
            child_path.display()
        );
        let mut command = Command::new("sh");
        command.args(["-c", &script]);

        let task = tokio::spawn(output(command));
        wait_for_file(&root_path).await;
        wait_for_file(&child_path).await;
        let root = read_pid(&root_path);
        let child = read_pid(&child_path);

        task.abort();
        let error = task.await.expect_err("command task should be cancelled");
        assert!(error.is_cancelled());

        wait_for_exit(root).await;
        wait_for_exit(child).await;
    }

    async fn wait_for_file(path: &Path) {
        for _ in 0..100 {
            if path.is_file() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("timed out waiting for {}", path.display());
    }

    fn read_pid(path: &Path) -> u32 {
        std::fs::read_to_string(path)
            .expect("read pid")
            .trim()
            .parse()
            .expect("parse pid")
    }

    async fn wait_for_exit(pid: u32) {
        for _ in 0..100 {
            if !pid_exists(pid) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("process {pid} survived cancellation");
    }

    #[allow(unsafe_code, clippy::cast_possible_wrap)]
    fn pid_exists(pid: u32) -> bool {
        // SAFETY: signal 0 only probes process existence.
        unsafe {
            libc::kill(pid as i32, 0) == 0
                || io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
        }
    }
}
