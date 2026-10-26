//! End-to-end coverage for `roko safety controls` and `roko safety release`.

mod common;
use common::Isolated;

use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

/// `roko` as the person these tests play: with none of the variables roko
/// sets on its agents and checks, whatever runs the tests (find-d05b5e).
/// `isolated` drops the `ROKO_*` ones, and this the rest.
fn roko(workdir: &Path) -> Command {
    let mut command = Command::cargo_bin("roko").expect("roko binary");
    command.isolated();
    let env = roko_core::child_env::process_env();
    for marker in roko_cli::commands::safety::roko_started_markers(&env) {
        for name in marker.variables {
            command.env_remove(name);
        }
    }
    command.current_dir(workdir);
    command
}

/// backlog 1105: an operator sees an immune isolation and lifts it in one
/// command, and the release leaves an audit line.
#[test]
fn safety_release_clears_isolation_control() {
    let workdir = TempDir::new().expect("tempdir");
    roko_agent::isolate_agent(workdir.path(), "live-a/cli", "operator_isolation")
        .expect("seed an isolation control");

    let listed = predicate::str::contains("live-a/cli")
        .and(predicate::str::contains("reason=operator_isolation"));
    roko(workdir.path())
        .args(["safety", "controls", "--workdir"])
        .arg(workdir.path())
        .assert()
        .success()
        .stdout(listed);

    roko(workdir.path())
        .args([
            "safety",
            "release",
            "live-a/cli",
            "--reason",
            "blank answer, not tamper",
            "--by",
            "release-test",
            "--workdir",
        ])
        .arg(workdir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("released isolation control"));

    let controls = roko_agent::list_agent_controls(workdir.path()).expect("read controls");
    assert!(controls.is_empty(), "{controls:?}");
    let audit_path = roko_agent::agent_control_releases_path(workdir.path());
    let audit = std::fs::read_to_string(&audit_path).expect("audit file");
    let lines = audit.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 1, "{audit}");
    let record: roko_agent::ReleasedControl = serde_json::from_str(lines[0]).expect("audit line");
    assert_eq!(record.agent_id, "live-a/cli");
    assert_eq!(record.by, "release-test");
    assert_eq!(record.reason, "blank answer, not tamper");
    // find-d05b5e: the line says the release came through the command, with
    // the name from --by, and no terminal on its standard input.
    assert_eq!(
        record.via,
        Some(roko_agent::ReleaseOrigin {
            channel: roko_agent::ReleaseChannel::Cli,
            by_source: roko_agent::PrincipalSource::Flag,
            terminal: Some(false),
        })
    );

    // Nothing is left to release: the command says so and fails.
    roko(workdir.path())
        .args(["safety", "release", "live-a/cli", "--reason", "again"])
        .arg("--workdir")
        .arg(workdir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("no isolation control"));
    let audit_after = std::fs::read_to_string(&audit_path).expect("audit file");
    assert_eq!(audit_after, audit, "a failed release writes no audit line");
}

/// A tool whose first result is structured but not JSON, which the
/// tool-result immune boundary withholds and answers by isolating the tool.
struct Lookup;

#[async_trait::async_trait]
impl roko_core::tool::ToolHandler for Lookup {
    fn name(&self) -> &str {
        "lookup"
    }

    async fn execute(
        &self,
        _call: roko_core::tool::ToolCall,
        _ctx: &roko_core::tool::ToolContext,
    ) -> roko_core::tool::ToolResult {
        roko_core::tool::ToolResult::Ok {
            content: vec![roko_core::tool::ToolResultContent::text("{not-json")],
            is_structured: true,
            artifacts: Vec::new(),
        }
    }
}

/// gap-a2787f: a tool the immune boundary isolated in a workspace is listed
/// by `roko safety controls` and released by `roko safety release --tool
/// --scope`, which leaves an audit line.
#[tokio::test]
async fn safety_release_clears_a_tool_control() {
    use std::sync::Arc;

    use roko_agent::dispatcher::{HandlerResolver, ToolDispatcher};
    use roko_core::tool::{
        ToolCall, ToolCategory, ToolContext, ToolDef, ToolHandler, ToolPermission, ToolRegistry,
        VecToolRegistry,
    };

    let workdir = TempDir::new().expect("tempdir");
    let handler: Arc<dyn ToolHandler> = Arc::new(Lookup);
    let resolver: Arc<dyn HandlerResolver> =
        Arc::new(move |name: &str| (name == "lookup").then(|| Arc::clone(&handler)));
    let def = ToolDef::new(
        "lookup",
        "Look something up",
        ToolCategory::Read,
        ToolPermission::read_only(),
    );
    let registry: Arc<dyn ToolRegistry> = Arc::new(VecToolRegistry::from_tools(vec![def]));
    let safety = roko_agent::SafetyLayer::with_defaults()
        .with_tool_permission_policy(roko_agent::ToolPermissionPolicy::DenyExplicit, Vec::new());
    let dispatcher = ToolDispatcher::new(registry, resolver).with_safety(safety);
    let context = ToolContext::testing(workdir.path());
    for id in ["withheld", "refused"] {
        let result = dispatcher
            .dispatch(ToolCall::new(id, "lookup", serde_json::json!({})), &context)
            .await;
        assert!(result.is_err(), "{id}: {result:?}");
    }

    roko(workdir.path())
        .args(["safety", "controls", "--workdir"])
        .arg(workdir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "tool lookup  scope=builtin:local  state=isolated",
        ));

    roko(workdir.path())
        .args([
            "safety",
            "release",
            "--tool",
            "lookup",
            "--scope",
            "builtin:local",
            "--reason",
            "a false positive",
            "--by",
            "release-test",
            "--workdir",
        ])
        .arg(workdir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "released the isolated control of tool lookup (builtin:local)",
        ));

    let audit_path = roko_agent::tool_control_releases_path(workdir.path());
    let audit = std::fs::read_to_string(&audit_path).expect("audit file");
    let lines = audit.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 1, "{audit}");
    let record: roko_agent::ReleasedToolControl =
        serde_json::from_str(lines[0]).expect("audit line");
    assert_eq!(
        (record.tool_name.as_str(), record.by.as_str()),
        ("lookup", "release-test")
    );
    roko(workdir.path())
        .args(["safety", "controls", "--workdir"])
        .arg(workdir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("no isolation controls"));
}

/// find-d05b5e: in the shell of a Claude Code agent roko runs, which has
/// the variables roko sets on every such agent, `roko safety release`
/// refuses, says why, and leaves the control and the audit file alone; the
/// controls are still listed there.
#[test]
fn safety_release_refuses_in_a_claude_agents_shell() {
    let workdir = TempDir::new().expect("tempdir");
    roko_agent::isolate_agent(workdir.path(), "live-a/cli", "operator_isolation")
        .expect("seed an isolation control");
    let agent_shell = |command: &mut Command| {
        for (name, value) in roko_agent::claude_cli_agent::ISOLATION_ENV {
            command.env(name, value);
        }
    };

    let mut release = roko(workdir.path());
    agent_shell(&mut release);
    release
        .args([
            "safety",
            "release",
            "live-a/cli",
            "--reason",
            "looks fine",
            "--by",
            "will",
            "--workdir",
        ])
        .arg(workdir.path())
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(
            predicate::str::contains(
                "refused: roko safety release does not run in a process roko started for an \
                 agent or a check",
            )
            .and(predicate::str::contains(
                "(set on the Claude Code agents roko runs)",
            ))
            .and(predicate::str::contains(
                "Only a person releases an immune control",
            )),
        );

    let controls = roko_agent::list_agent_controls(workdir.path()).expect("read controls");
    assert_eq!(
        controls.len(),
        1,
        "the control stays in force: {controls:?}"
    );
    assert!(!roko_agent::agent_control_releases_path(workdir.path()).exists());
    let mut listed = roko(workdir.path());
    agent_shell(&mut listed);
    listed
        .args(["safety", "controls", "--workdir"])
        .arg(workdir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("agent live-a/cli  state=isolated"));
}

#[test]
fn safety_controls_on_an_empty_workspace_says_so() {
    let workdir = TempDir::new().expect("tempdir");

    roko(workdir.path())
        .args(["safety", "controls", "--workdir"])
        .arg(workdir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("no isolation controls"));
}
