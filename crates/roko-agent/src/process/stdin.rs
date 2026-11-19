//! Prompt delivery: write a prompt to an agent's stdin and close it.

use std::io;

use tokio::io::AsyncWriteExt as _;
use tokio::process::ChildStdin;

/// Write `prompt` to an agent's `stdin`, then close it.
///
/// An agent that exited, or closed its input, before it read the whole
/// prompt makes the write fail with a broken pipe. That is no failure of the
/// run in itself (bug-7d13f0): the agent's exit status and output say what
/// happened, as they do when the write wins the race and the prompt waits in
/// the pipe unread. So a broken pipe counts as written, and the caller goes
/// on to wait for the agent.
///
/// # Errors
///
/// A write error other than a broken pipe.
pub async fn write_prompt(mut stdin: ChildStdin, prompt: &[u8]) -> io::Result<()> {
    match stdin.write_all(prompt).await {
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => {
            tracing::debug!(
                "the agent stopped reading its input before the whole prompt; its exit decides the run"
            );
            Ok(())
        }
        written => written,
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::process::Stdio;

    use super::*;

    /// A child that exits without reading its input takes no prompt, and that
    /// is no error; one that reads it gets it whole.
    #[tokio::test]
    async fn write_prompt_to_a_child_that_stopped_reading_is_no_error() {
        let mut child = tokio::process::Command::new("sh")
            .args(["-c", "exit 0"])
            .stdin(Stdio::piped())
            .spawn()
            .expect("spawn sh");
        let stdin = child.stdin.take().expect("its stdin");
        // 4 MiB: more than any pipe holds, so the write ends at the exit.
        write_prompt(stdin, &vec![b'x'; 4 << 20])
            .await
            .expect("a broken pipe is no error");
        assert!(child.wait().await.expect("wait for sh").success());

        let mut child = tokio::process::Command::new("wc")
            .arg("-c")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("spawn wc");
        let stdin = child.stdin.take().expect("its stdin");
        write_prompt(stdin, b"twelve bytes")
            .await
            .expect("the prompt is written");
        let output = child.wait_with_output().await.expect("wait for wc");
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "12");
    }
}
