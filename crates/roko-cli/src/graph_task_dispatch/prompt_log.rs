//! `[runner] log_prompts`: the prompts each Graph attempt sends its agent, kept for debugging.
//!
//! With the key on, every attempt writes its system and user prompt to
//! `.roko/prompt-logs/<plan>-<task>-<unix ms>.txt` (gitignored) just before its agent runs, with
//! known secret patterns redacted. `[runner] prompt_log_retention` caps how many files stay; the
//! oldest by modification time go first. Writing is best effort: a failure is logged and the
//! attempt carries on.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::{SystemTime, UNIX_EPOCH};

use roko_core::obs::LogScrubber;

use super::*;

static SCRUBBER: LazyLock<LogScrubber> = LazyLock::new(LogScrubber::new);

impl GraphTaskDispatcher {
    /// Keep the attempt's prompts when `[runner] log_prompts` is on.
    pub(super) fn log_prompts(
        &self,
        spec: &TaskExecutionSpec,
        task_id: &str,
        system_prompt: &str,
        user_prompt: &str,
    ) {
        let runner = &self.config.runner;
        if runner.log_prompts {
            write_prompt_log(
                &self.workdir,
                &format!("{}-{task_id}", spec.plan_id),
                system_prompt,
                user_prompt,
                runner.prompt_log_retention,
            );
        }
    }
}

/// Where the prompt logs live under a workspace root.
fn prompt_log_dir(workspace: &Path) -> PathBuf {
    workspace.join(".roko").join("prompt-logs")
}

/// Write one attempt's prompts, then prune the directory to `retention` files.
fn write_prompt_log(
    workspace: &Path,
    name: &str,
    system_prompt: &str,
    user_prompt: &str,
    retention: usize,
) {
    let dir = prompt_log_dir(workspace);
    if let Err(error) = std::fs::create_dir_all(&dir) {
        tracing::warn!(%error, path = %dir.display(), "prompt log: cannot create its directory");
        return;
    }
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis());
    let name: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '-'
            }
        })
        .collect();
    let path = dir.join(format!("{name}-{stamp}.txt"));
    let contents = format!(
        "=== SYSTEM PROMPT ===\n{}\n\n=== USER PROMPT ===\n{}\n",
        SCRUBBER.scrub(system_prompt),
        SCRUBBER.scrub(user_prompt)
    );
    if let Err(error) = std::fs::write(&path, contents) {
        tracing::warn!(%error, path = %path.display(), "prompt log: write failed");
        return;
    }
    prune(&dir, retention);
}

/// Remove the oldest `.txt` files until at most `retention` remain.
fn prune(dir: &Path, retention: usize) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut logs: Vec<(SystemTime, PathBuf)> = entries
        .flatten()
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "txt"))
        .filter_map(|entry| Some((entry.metadata().ok()?.modified().ok()?, entry.path())))
        .collect();
    if logs.len() <= retention {
        return;
    }
    logs.sort();
    for (_, path) in &logs[..logs.len() - retention] {
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn logs(workspace: &Path) -> Vec<PathBuf> {
        let mut paths: Vec<PathBuf> = std::fs::read_dir(prompt_log_dir(workspace))
            .expect("prompt-log dir")
            .flatten()
            .map(|entry| entry.path())
            .collect();
        paths.sort();
        paths
    }

    #[test]
    fn writes_both_prompts_with_secrets_redacted() {
        let workspace = tempfile::tempdir().expect("tempdir");
        let key = "sk-ant-api03-abcdefghijklmnopqrstuvwxyz0123456789";
        write_prompt_log(
            workspace.path(),
            "plan/a-task:1",
            "You are the implementer.",
            &format!("Use the key {key} to call the API."),
            10,
        );

        let files = logs(workspace.path());
        assert_eq!(files.len(), 1);
        let name = files[0]
            .file_name()
            .and_then(|name| name.to_str())
            .expect("file name");
        assert!(name.starts_with("plan-a-task-1-"), "{name}");
        let text = std::fs::read_to_string(&files[0]).expect("log text");
        assert!(text.starts_with("=== SYSTEM PROMPT ===\nYou are the implementer.\n"));
        assert!(text.contains("=== USER PROMPT ===\nUse the key "));
        assert!(!text.contains(key), "{text}");
    }

    #[test]
    fn keeps_only_the_newest_retention_files() {
        let workspace = tempfile::tempdir().expect("tempdir");
        for task in ["t1", "t2", "t3"] {
            write_prompt_log(
                workspace.path(),
                &format!("plan-{task}"),
                "system",
                "user",
                2,
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        }

        let names: Vec<String> = logs(workspace.path())
            .iter()
            .filter_map(|path| path.file_name()?.to_str().map(str::to_owned))
            .collect();
        assert_eq!(names.len(), 2, "{names:?}");
        assert!(
            names.iter().all(|name| !name.starts_with("plan-t1-")),
            "{names:?}"
        );
    }
}
