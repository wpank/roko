//! A tool command that leads a process group of its own (bug-c83c84).
//!
//! An agent's command runs apart from roko's process group, so a signal it
//! sends its own group, such as `kill -INT 0`, never reaches roko: a
//! `roko serve` would take that SIGINT as Ctrl-C and shut down. A call that
//! ends before its command does, because a cancelled attempt dropped it or
//! it ran past its timeout, kills the whole group. Nothing the command
//! started, such as cargo, rustc or a test binary, outlives it.

use std::process::{Output, Stdio};
use std::time::Duration;

use tokio::process::Command;
use tokio::time::error::Elapsed;

/// Run `cmd` in a process group of its own and collect its output, as
/// [`Command::output`] does, for at most `timeout`. A spawn or wait error is
/// the inner error.
pub(super) async fn output_within(
    mut cmd: Command,
    timeout: Duration,
) -> Result<std::io::Result<Output>, Elapsed> {
    cmd.kill_on_drop(true)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    cmd.process_group(0);
    let child = match cmd.spawn() {
        Ok(child) => child,
        Err(error) => return Ok(Err(error)),
    };
    let mut group = KillGroupOnDrop(child.id());
    let output = tokio::time::timeout(timeout, child.wait_with_output()).await?;
    if output.is_ok() {
        // The command ended on its own: what it left running in the
        // background is its own business.
        group.0 = None;
    }
    Ok(output)
}

/// Kills the process group a command leads when its call ends before the
/// command does.
struct KillGroupOnDrop(Option<u32>);

impl Drop for KillGroupOnDrop {
    fn drop(&mut self) {
        let leader = self.0.take();
        #[cfg(unix)]
        if let Some(leader) = leader
            .and_then(|pid| i32::try_from(pid).ok())
            .and_then(rustix::process::Pid::from_raw)
        {
            let _ = rustix::process::kill_process_group(leader, rustix::process::Signal::KILL);
        }
        #[cfg(not(unix))]
        let _ = leader;
    }
}
