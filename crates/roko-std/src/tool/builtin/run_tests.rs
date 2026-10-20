//! `run_tests` — run the project's tests and return structured results.
//!
//! Category: [`ToolCategory::Exec`]. Permission: read + exec.
//! Concurrency: [`ToolConcurrency::Serial`] (tests often share fixtures).
//! Idempotent: no.

use async_trait::async_trait;
use roko_core::child_env;
use roko_core::tool::{
    ToolCall, ToolCategory, ToolConcurrency, ToolContext, ToolDef, ToolError, ToolHandler,
    ToolPermission, ToolResult, ToolSchema,
};
use std::ffi::OsString;

use super::child_group;

/// Canonical `snake_case` name.
pub const NAME: &str = "run_tests";

/// Human-readable description sent to the LLM.
pub const DESCRIPTION: &str = "Run the project's test suite (or a filtered subset) \
    and return structured pass/fail results. With build \"checks\", run this task's own \
    checks instead: the verify steps roko runs on your work when you hand off, in order, \
    stopping at the first that fails and showing the end of its output. Run them after a \
    change rather than handing off to find out.";

/// Build the [`ToolDef`] for `run_tests`.
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
            "build": {
                "type": "string",
                "enum": ["cargo", "npm", "go", "pytest", "forge", "make", "checks"],
                "description": "Build system to use for running tests (default: 'cargo'), or \
                    'checks' for this task's own checks (any language: CMake, ctest, scripts)."
            },
            "filter": {
                "type": "string",
                "description": "Test name filter / pattern to run a subset of tests."
            },
            "timeout_ms": {
                "type": "integer",
                "description": "Optional wall-clock timeout in milliseconds. Default 120000 \
                    (`[timeouts] tool_execution_ms`); a longer one is honoured up to 600000 \
                    (`[timeouts] tool_execution_max_ms`)."
            }
        },
        "additionalProperties": false
    })))
    .with_concurrency(ToolConcurrency::Serial)
    .with_idempotent(false)
    .with_timeout_ms(600_000)
}

/// Handler for `run_tests` (§36.29).
///
/// Wraps `bash` by invoking the build-system-appropriate test command
/// (`cargo test`, `npm test`, `go test ./...`, …) selected by the
/// `build` argument. Falls back to `cargo test` when unspecified.
///
/// The result structure is:
///
/// ```json
/// { "build": "...", "status": "ok"|"failed", "passed": N, "failed": N, "ignored": N }
/// ```
///
/// Full stdout is included when the tests fail.
///
/// The tests run with the environment verify steps get
/// ([`child_env::apply_gate_env`]): agent-written tests see no provider key,
/// secret-looking name or `.env`-loaded name unless
/// [`ToolContext::env_passthrough`] names it.
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

/// Run `call`. The tests inherit `parent_env` in place of roko's own
/// environment when given, filtered by the same policy.
async fn run(
    call: ToolCall,
    ctx: &ToolContext,
    parent_env: Option<Vec<(String, OsString)>>,
) -> ToolResult {
    if !ctx.capabilities.exec {
        return ToolResult::Err(ToolError::PermissionDenied(
            "run_tests requires exec capability".into(),
        ));
    }
    let build = call
        .arguments
        .get("build")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("cargo");
    // gap-770cb5: the task's own checks, within the call's time.
    if build == "checks" {
        return super::task_checks::run(ctx, ctx.call_timeout(&call).budget, parent_env).await;
    }
    let filter = call
        .arguments
        .get("filter")
        .and_then(serde_json::Value::as_str);
    let (program, args) = match build {
        "cargo" => ("cargo", vec!["test", "--workspace"]),
        "npm" => ("npm", vec!["test"]),
        "go" => ("go", vec!["test", "./..."]),
        "pytest" => ("python3", vec!["-m", "pytest"]),
        "forge" => ("forge", vec!["test"]),
        "make" => ("make", vec!["test"]),
        other => {
            return ToolResult::Err(ToolError::Other(format!(
                "run_tests: unknown build system `{other}`"
            )));
        }
    };
    let mut cmd = tokio::process::Command::new(program);
    for arg in &args {
        cmd.arg(arg);
    }
    if let Some(f) = filter {
        cmd.arg(f);
    }
    cmd.current_dir(ctx.worktree());
    child_env::apply_gate_env(
        cmd.as_std_mut(),
        parent_env.unwrap_or_else(child_env::process_env),
        &ctx.env_passthrough,
    );
    // The call's own `timeout_ms`, up to the cap, else the context's
    // (bug-c22bd9).
    let timeout = ctx.call_timeout(&call);
    // The run leads a process group of its own, which a cancel or the timeout
    // kills whole (bug-c83c84).
    let output = match child_group::output_within(cmd, timeout.budget).await {
        Ok(Ok(o)) => o,
        Ok(Err(e)) => {
            return ToolResult::Err(ToolError::Other(format!("run_tests: spawn failed: {e}")));
        }
        Err(_) => {
            return ToolResult::Err(
                timeout.timed_out(u64::try_from(timeout.budget.as_millis()).unwrap_or(u64::MAX)),
            );
        }
    };
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let combined = format!("{stdout}\n{stderr}");
    let counts = parse_counts(&combined);
    let payload = serde_json::json!({
        "build": build,
        "status": if output.status.success() { "ok" } else { "failed" },
        "passed": counts.passed,
        "failed": counts.failed,
        "ignored": counts.ignored,
        "output": if output.status.success() { serde_json::Value::Null } else { serde_json::Value::String(combined.clone()) },
    });
    if output.status.success() {
        ToolResult::structured(payload.to_string())
    } else {
        ToolResult::Err(ToolError::Other(format!(
            "run_tests: {} tests failed — {}",
            counts.failed,
            truncate(&combined, 400)
        )))
    }
}

#[derive(Default)]
struct Counts {
    passed: u32,
    failed: u32,
    ignored: u32,
}

fn parse_counts(output: &str) -> Counts {
    let mut c = Counts::default();
    for line in output.lines() {
        let t = line.trim();
        if t.starts_with("test result:") {
            c.passed += extract(t, "passed");
            c.failed += extract(t, "failed");
            c.ignored += extract(t, "ignored");
        }
    }
    c
}

fn extract(line: &str, label: &str) -> u32 {
    for part in line.split(';') {
        let p = part.trim();
        if let Some(rest) = p.strip_suffix(label).map(str::trim_end)
            && let Some(num) = rest.split_whitespace().last()
            && let Ok(n) = num.parse::<u32>()
        {
            return n;
        }
    }
    0
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &s[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_counts_reads_summary_line() {
        let out = "test result: ok. 5 passed; 2 failed; 1 ignored; 0 measured";
        let c = parse_counts(out);
        assert_eq!(c.passed, 5);
        assert_eq!(c.failed, 2);
        assert_eq!(c.ignored, 1);
    }

    #[test]
    fn parse_counts_aggregates_multiple_lines() {
        let out = "test result: ok. 3 passed; 0 failed; 0 ignored\n\
                   test result: FAILED. 2 passed; 1 failed; 0 ignored";
        let c = parse_counts(out);
        assert_eq!(c.passed, 5);
        assert_eq!(c.failed, 1);
    }

    #[test]
    fn parse_counts_zero_on_empty() {
        let c = parse_counts("");
        assert_eq!(c.passed, 0);
        assert_eq!(c.failed, 0);
        assert_eq!(c.ignored, 0);
    }

    #[test]
    fn truncate_adds_ellipsis_when_long() {
        let long = "a".repeat(500);
        let short = truncate(&long, 10);
        assert!(short.ends_with('…'));
        assert!(short.chars().count() <= 12);
    }

    #[test]
    fn truncate_passthrough_when_short() {
        assert_eq!(truncate("short", 400), "short");
    }

    /// gap-770cb5: `build: "checks"` runs the checks dispatch left for the
    /// call's attempt, in the checkout, and stops at the first that fails
    /// with the end of its output; once the work is fixed they pass. A call
    /// of no attempt, or of one with no checks, says so.
    #[cfg(unix)]
    #[tokio::test]
    async fn run_tests_runs_the_task_checks() {
        use super::super::task_checks::{TaskCheck, task_checks_path};

        let dir = tempfile::tempdir().expect("tempdir");
        let mut ctx = ToolContext::testing(dir.path());
        let call = || ToolCall::new("c", NAME, serde_json::json!({"build": "checks"}));
        let result = run(call(), &ctx, None).await;
        assert!(
            matches!(&result, ToolResult::Err(ToolError::Other(why)) if why.contains("no task")),
            "{result:?}"
        );
        ctx.correlation.attempt_id = "run-1:plan-a:T1:1".to_string();
        let result = run(call(), &ctx, None).await;
        assert!(
            matches!(&result, ToolResult::Err(ToolError::Other(why)) if why.contains("no checks")),
            "{result:?}"
        );

        let checks = vec![
            TaskCheck {
                label: "verify[0:build]".to_string(),
                command: "echo building; test -f fixed.txt || { echo 'error: not fixed' >&2; \
                          exit 2; }"
                    .to_string(),
                timeout_ms: Some(30_000),
            },
            TaskCheck {
                label: "verify[1:test]".to_string(),
                command: "echo tested".to_string(),
                timeout_ms: None,
            },
        ];
        let path = task_checks_path(&ctx.immune_root_path, &ctx.correlation.attempt_id);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("dir");
        std::fs::write(&path, serde_json::to_vec(&checks).expect("json")).expect("checks");

        let result = run(call(), &ctx, None).await;
        let ToolResult::Err(ToolError::Other(why)) = &result else {
            panic!("the unfixed work fails its first check: {result:?}");
        };
        assert!(
            why.contains("check 1 of 2 (verify[0:build]) failed (exit 2)"),
            "{why}"
        );
        assert!(why.contains("building\nerror: not fixed"), "{why}");

        std::fs::write(dir.path().join("fixed.txt"), "ok\n").expect("fix");
        let result = run(call(), &ctx, None).await;
        let ToolResult::Ok { .. } = &result else {
            panic!("the fixed work passes: {result:?}");
        };
        let text = format!("{result:?}");
        assert!(text.contains("verify[1:test]"), "{text}");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn run_tests_env_excludes_provider_keys() {
        let dir = tempfile::tempdir().expect("tempdir");
        // The test target records the environment it runs with.
        std::fs::write(
            dir.path().join("Makefile"),
            "test:\n\tenv > child-env.txt\n",
        )
        .expect("write Makefile");
        let path = std::env::var_os("PATH")
            .filter(|path| !path.is_empty())
            .unwrap_or_else(|| "/usr/bin:/bin".into());
        let mut parent = vec![("PATH".to_string(), path)];
        for (name, value) in [
            ("CARGO_HOME", "/tmp/tests-cargo"),
            ("OPENAI_API_KEY", "sk-test-not-real"),
            ("ANTHROPIC_API_KEY", "sk-ant-test-not-real"),
            ("MY_SECRET_TOKEN", "tok-test-not-real"),
        ] {
            parent.push((name.to_string(), OsString::from(value)));
        }
        let call = ToolCall::new("c", NAME, serde_json::json!({"build": "make"}));
        let ctx = ToolContext::testing(dir.path());

        let result = run(call, &ctx, Some(parent)).await;

        assert!(
            matches!(result, ToolResult::Ok { .. }),
            "make test failed: {result:?}"
        );
        let env = std::fs::read_to_string(dir.path().join("child-env.txt"))
            .expect("make test records its env");
        for leaked in [
            "sk-test-not-real",
            "sk-ant-test-not-real",
            "tok-test-not-real",
        ] {
            assert!(!env.contains(leaked), "{leaked} leaked:\n{env}");
        }
        assert!(
            env.contains("CARGO_HOME=/tmp/tests-cargo"),
            "toolchain env missing:\n{env}"
        );
    }
}
