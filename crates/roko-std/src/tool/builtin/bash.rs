//! `bash` — execute a shell command.
//!
//! Category: [`ToolCategory::Exec`]. Permission: read + exec.
//! Concurrency: [`ToolConcurrency::Serial`] (shared shell state).
//! Idempotent: no.

use async_trait::async_trait;
use roko_core::child_env;
use roko_core::defaults::DEFAULT_REQUEST_TIMEOUT_MS;
use roko_core::tool::{
    ToolCall, ToolCategory, ToolConcurrency, ToolContext, ToolDef, ToolError, ToolHandler,
    ToolPermission, ToolResult, ToolSchema,
};
use std::ffi::OsString;
use std::time::Duration;

use super::sandbox::{refuse_key_file_in_command, require_string};

/// Canonical `snake_case` name.
pub const NAME: &str = "bash";

/// Human-readable description sent to the LLM.
pub const DESCRIPTION: &str = "Execute a shell command via `bash -c` and return its output.";

/// Build the [`ToolDef`] for `bash`.
#[must_use]
pub fn tool_def() -> ToolDef {
    ToolDef::new(
        NAME,
        DESCRIPTION,
        ToolCategory::Exec,
        ToolPermission::executes(),
    )
    .with_parameters(ToolSchema::from_value(serde_json::json!({
        "type": "object",
        "properties": {
            "command": {
                "type": "string",
                "description": "The shell command to execute via `bash -c`."
            },
            "timeout_ms": {
                "type": "integer",
                "description": "Optional wall-clock timeout in milliseconds (default: 120000)."
            }
        },
        "required": ["command"],
        "additionalProperties": false
    })))
    .with_concurrency(ToolConcurrency::Serial)
    .with_idempotent(false)
    .with_timeout_ms(DEFAULT_REQUEST_TIMEOUT_MS)
}

// Command-level safety (denylist, path confinement) is enforced by the
// SafetyLayer's `BashPolicy` before this handler is invoked. Provider key
// files and roko configs holding a secret are the exception: the handler
// refuses a command that names one, or searches or lists a tree that holds
// one, itself, with the same `refuse_key_file_in_command` SafetyLayer runs,
// so the block holds whichever dispatcher runs it, as for the file tools.

/// Handler for `bash` (§36.20).
///
/// Spawns `bash -c <command>` in the worktree with
/// [`ToolContext::timeout`] as the wall-clock budget. The command's
/// combined stdout+stderr is returned as the tool result's content.
///
/// The command gets the environment verify steps get
/// ([`child_env::apply_gate_env`]): system, locale and toolchain variables,
/// never a provider key, a secret-looking name or a name roko loaded from a
/// `.env` file, unless [`ToolContext::env_passthrough`] names it.
///
/// Per the sandbox contract, the handler only runs when
/// [`ToolPermission::exec`] is granted; safety policies layer additional
/// allowlist/blocklist checks on top.
#[derive(Debug, Clone, Copy, Default)]
pub struct Handler;

#[async_trait]
impl ToolHandler for Handler {
    fn name(&self) -> &str {
        NAME
    }

    async fn execute(&self, call: ToolCall, ctx: &ToolContext) -> ToolResult {
        run(call, ctx, None).await
    }
}

/// Run `call`. The command inherits `parent_env` in place of roko's own
/// environment when given, filtered by the same policy.
async fn run(
    call: ToolCall,
    ctx: &ToolContext,
    parent_env: Option<Vec<(String, OsString)>>,
) -> ToolResult {
    if !ctx.capabilities.exec {
        return ToolResult::Err(ToolError::PermissionDenied(
            "bash requires exec capability".into(),
        ));
    }
    let command = match require_string(&call.arguments, "command") {
        Ok(c) => c,
        Err(e) => return ToolResult::Err(e),
    };
    if let Err(e) = refuse_key_file_in_command(&command, ctx.worktree()) {
        return ToolResult::Err(e);
    }
    let effective_timeout = if ctx.timeout.is_zero() {
        Duration::from_mins(2)
    } else {
        ctx.timeout
    };
    let mut cmd = tokio::process::Command::new("bash");
    cmd.arg("-c").arg(&command);
    cmd.current_dir(ctx.worktree());
    cmd.kill_on_drop(true);
    child_env::apply_gate_env(
        cmd.as_std_mut(),
        parent_env.unwrap_or_else(child_env::process_env),
        &ctx.env_passthrough,
    );

    let output = match tokio::time::timeout(effective_timeout, cmd.output()).await {
        Ok(Ok(o)) => o,
        Ok(Err(e)) => {
            return ToolResult::Err(ToolError::Other(format!("bash: spawn failed: {e}")));
        }
        Err(_) => {
            return ToolResult::Err(ToolError::Timeout {
                after_ms: u64::try_from(effective_timeout.as_millis()).unwrap_or(u64::MAX),
            });
        }
    };
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let combined = if stderr.is_empty() {
        stdout
    } else {
        format!("{stdout}{stderr}")
    };
    if output.status.success() {
        ToolResult::text(combined)
    } else {
        ToolResult::Err(ToolError::Other(format!(
            "bash: exited with status {:?}: {combined}",
            output.status.code()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::tool::ToolContext;

    /// `items` as an inherited environment, after this process's `PATH`.
    #[cfg(unix)]
    fn parent_env(items: &[(&str, &str)]) -> Vec<(String, OsString)> {
        let path = std::env::var_os("PATH")
            .filter(|path| !path.is_empty())
            .unwrap_or_else(|| "/usr/bin:/bin".into());
        let mut env = vec![("PATH".to_string(), path)];
        for (name, value) in items {
            env.push(((*name).to_string(), OsString::from(*value)));
        }
        env
    }

    fn env_output(result: ToolResult) -> String {
        match result {
            ToolResult::Ok { .. } => result.text_content(),
            ToolResult::Err(e) => panic!("bash handler failed unexpectedly: {e}"),
        }
    }

    /// With roko's own environment, the command sees no variable that looks
    /// like a credential, whatever this process was started with, and still
    /// sees `PATH`.
    #[tokio::test]
    async fn env_scrubbing_hides_secrets_preserves_safe_keys() {
        let call = ToolCall::new("c", NAME, serde_json::json!({"command": "env"}));
        let ctx = ToolContext::testing(std::env::temp_dir());

        let output = env_output(Handler.execute(call, &ctx).await);

        assert!(
            output.lines().any(|line| line.starts_with("PATH=")),
            "PATH should be inherited by the child process:\n{output}"
        );
        for line in output.lines() {
            let Some((name, _)) = line.split_once('=') else {
                continue;
            };
            assert!(
                !child_env::is_secret_env_name(name) && !child_env::is_provider_key_var(name),
                "{name} leaked into the child env"
            );
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn bash_tool_keeps_toolchain_env() {
        let parent = parent_env(&[
            ("HOME", "/tmp/tool-home"),
            ("CARGO_HOME", "/tmp/tool-cargo"),
            ("RUSTUP_HOME", "/tmp/tool-rustup"),
            ("CARGO_TARGET_DIR", "/tmp/tool-target"),
            ("DATABASE_URL", "postgres://tool-db"),
            ("OPENAI_API_KEY", "sk-test-not-real"),
            ("MY_SECRET_TOKEN", "tok-test-not-real"),
            ("UNRELATED_SETTING", "not-inherited"),
        ]);
        let call = ToolCall::new("c", NAME, serde_json::json!({"command": "env"}));
        let ctx = ToolContext::testing(std::env::temp_dir())
            .with_env_passthrough(vec!["DATABASE_URL".to_string()]);

        let output = env_output(run(call, &ctx, Some(parent)).await);

        for leaked in ["sk-test-not-real", "tok-test-not-real", "not-inherited"] {
            assert!(!output.contains(leaked), "{leaked} leaked:\n{output}");
        }
        for kept in [
            "HOME=/tmp/tool-home",
            "CARGO_HOME=/tmp/tool-cargo",
            "RUSTUP_HOME=/tmp/tool-rustup",
            "CARGO_TARGET_DIR=/tmp/tool-target",
            "DATABASE_URL=postgres://tool-db",
        ] {
            assert!(output.contains(kept), "{kept} missing:\n{output}");
        }
    }
}
