//! `roko safety`: inspect and release immune controls.
//!
//! Two immune boundaries write them, and the workspace keeps them until
//! someone acts. The provider boundary writes an isolation control to
//! `<workdir>/.roko/immune/agent-controls.json` when it contains a provider
//! result, and denies that agent id before every later dispatch. The
//! tool-result boundary writes a control to
//! `<workdir>/.roko/immune/tool-controls.json` when it withholds a tool's
//! result, and refuses that tool from that source in the workspace: an
//! isolation until it is released, a rate limit for a minute. These commands
//! show the controls in force and lift one, with an audit line in
//! `.roko/immune/agent-control-releases.jsonl` or
//! `.roko/immune/tool-control-releases.jsonl` (gap-a2787f).
//!
//! The commands live in the library, as `diagnose` does, so its tests run
//! them; the `roko` binary re-exports the module.

use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use clap::Subcommand;
use roko_agent::{ToolControl, ToolControlState};

use crate::exit_codes::{EXIT_FAILURE, EXIT_SUCCESS};

/// Immune isolation controls.
#[derive(Debug, Subcommand)]
pub enum SafetyCmd {
    /// List the immune controls in force: agents denied before dispatch, and
    /// tools refused in this workspace.
    Controls {
        /// Workspace root (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Release one agent's isolation control, or one tool's control (--tool
    /// and --scope), and record who did it and why.
    Release {
        /// Agent id the control denies, as `roko safety controls` lists it.
        #[arg(required_unless_present = "tool", conflicts_with = "tool")]
        agent_id: Option<String>,
        /// Tool whose control to release, as `roko safety controls` lists it.
        #[arg(long, requires = "scope")]
        tool: Option<String>,
        /// The source scope of that tool's control, as listed
        /// (`builtin:local`, `mcp:<server>`, …).
        #[arg(long, requires = "tool")]
        scope: Option<String>,
        /// Why the control is released (recorded in the audit file).
        #[arg(long)]
        reason: String,
        /// Who releases it (default: $USER, else "operator").
        #[arg(long)]
        by: Option<String>,
        /// Workspace root (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
}

/// Run `roko safety <cmd>`, in `--workdir` when given, else in
/// `default_workdir()`; `json` prints JSON.
///
/// # Errors
///
/// Fails on a control ledger that cannot be read or written (it fails
/// closed), or on an invalid agent id, tool, scope, `--by` or `--reason`.
pub fn cmd_safety(
    cmd: SafetyCmd,
    json: bool,
    default_workdir: impl FnOnce() -> PathBuf,
) -> Result<i32> {
    run(
        cmd,
        json,
        default_workdir,
        &mut std::io::stdout().lock(),
        &mut std::io::stderr().lock(),
    )
}

/// [`cmd_safety`], printing to `out` and `err`.
fn run(
    cmd: SafetyCmd,
    json: bool,
    default_workdir: impl FnOnce() -> PathBuf,
    out: &mut impl Write,
    err: &mut impl Write,
) -> Result<i32> {
    match cmd {
        SafetyCmd::Controls { workdir } => {
            controls(&workdir.unwrap_or_else(default_workdir), json, out)
        }
        SafetyCmd::Release {
            agent_id,
            tool,
            scope,
            reason,
            by,
            workdir,
        } => {
            let workdir = workdir.unwrap_or_else(default_workdir);
            let by = by.unwrap_or_else(default_principal);
            let released = match (tool, scope) {
                (Some(tool), Some(scope)) => {
                    release_tool(&workdir, &tool, &scope, &by, &reason, json, out)?
                }
                // clap requires an agent id when --tool is absent.
                _ => release_agent(
                    &workdir,
                    &agent_id.unwrap_or_default(),
                    &by,
                    &reason,
                    json,
                    out,
                )?,
            };
            if let Err(none) = released {
                if !json {
                    writeln!(err, "{none}")?;
                }
                return Ok(EXIT_FAILURE);
            }
            Ok(EXIT_SUCCESS)
        }
    }
}

/// `roko safety controls`: the agent and tool controls in force under
/// `workdir`.
fn controls(workdir: &Path, json: bool, out: &mut impl Write) -> Result<i32> {
    let agents = roko_agent::list_agent_controls(workdir)
        .with_context(|| format!("read isolation controls under {}", workdir.display()))?;
    let tools = roko_agent::list_tool_controls(workdir)
        .with_context(|| format!("read tool controls under {}", workdir.display()))?;
    if json {
        let report = serde_json::json!({
            "agent_controls": agents,
            "tool_controls": tools,
        });
        writeln!(out, "{}", serde_json::to_string_pretty(&report)?)?;
        return Ok(EXIT_SUCCESS);
    }
    if agents.is_empty() && tools.is_empty() {
        writeln!(out, "no isolation controls")?;
        return Ok(EXIT_SUCCESS);
    }
    for control in &agents {
        writeln!(
            out,
            "agent {}  state={}  reason={}  expires={}  control={}",
            control.agent_id,
            control.state,
            control.reason,
            time_label(control.expires_at_ms.and_then(|ms| i64::try_from(ms).ok())),
            control.control_id
        )?;
    }
    for control in &tools {
        writeln!(out, "{}", tool_control_line(control))?;
    }
    writeln!(
        out,
        "release one after review: roko safety release <agent> --reason <why>, or roko safety \
         release --tool <tool> --scope <scope> --reason <why>"
    )?;
    Ok(EXIT_SUCCESS)
}

/// Release the control of `tool` from `scope` under `workdir`, printing what
/// was released; `Ok(Err(why))` when there is none to release.
fn release_tool(
    workdir: &Path,
    tool: &str,
    scope: &str,
    by: &str,
    reason: &str,
    json: bool,
    out: &mut impl Write,
) -> Result<std::result::Result<(), String>> {
    let released = roko_agent::release_tool_control(workdir, tool, scope, by, reason)
        .with_context(|| format!("release the control of tool {tool} ({scope})"))?;
    let audit_path = roko_agent::tool_control_releases_path(workdir);
    if json {
        let report = serde_json::json!({ "released": released, "audit_path": audit_path });
        writeln!(out, "{}", serde_json::to_string_pretty(&report)?)?;
    }
    let Some(released) = released else {
        return Ok(Err(format!("no control for tool {tool} ({scope})")));
    };
    if !json {
        writeln!(
            out,
            "released the {} control of tool {} ({}), issued {} (by {})",
            released.state,
            released.tool_name,
            released.source_scope,
            time_label(Some(released.issued_at_ms)),
            released.by
        )?;
        writeln!(out, "audit record: {}", audit_path.display())?;
    }
    Ok(Ok(()))
}

/// Release the isolation control of `agent_id` under `workdir`, printing
/// what was released; `Ok(Err(why))` when there is none to release.
fn release_agent(
    workdir: &Path,
    agent_id: &str,
    by: &str,
    reason: &str,
    json: bool,
    out: &mut impl Write,
) -> Result<std::result::Result<(), String>> {
    let released = roko_agent::release_agent_control(workdir, agent_id, by, reason)
        .with_context(|| format!("release the isolation control of {agent_id}"))?;
    let audit_path = roko_agent::agent_control_releases_path(workdir);
    if json {
        let report = serde_json::json!({ "released": released, "audit_path": audit_path });
        writeln!(out, "{}", serde_json::to_string_pretty(&report)?)?;
    }
    let Some(released) = released else {
        return Ok(Err(format!("no isolation control for agent {agent_id}")));
    };
    if !json {
        writeln!(
            out,
            "released isolation control {} of {} (by {})",
            released.control_id, released.agent_id, released.by
        )?;
        writeln!(out, "audit record: {}", audit_path.display())?;
    }
    Ok(Ok(()))
}

/// One tool control as `roko safety controls` lists it.
fn tool_control_line(control: &ToolControl) -> String {
    let state = match control.control {
        ToolControlState::Isolated => "state=isolated".to_string(),
        ToolControlState::RateLimited { until_ms } => {
            format!("state=rate_limited  until={}", time_label(Some(until_ms)))
        }
    };
    format!(
        "tool {}  scope={}  {state}  issued={}  output={}",
        control.tool_name,
        control.source_scope,
        time_label(Some(control.issued_at_ms)),
        control.triggering_output.to_hex()
    )
}

/// A Unix time in milliseconds as RFC 3339, or `never` for none: a control
/// written before agent controls expired stays until released.
fn time_label(at_ms: Option<i64>) -> String {
    let Some(at_ms) = at_ms else {
        return "never".to_string();
    };
    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(at_ms)
        .map_or_else(|| format!("{at_ms}ms"), |at| at.to_rfc3339())
}

/// The releasing principal when `--by` is absent: `$USER`, else `operator`.
fn default_principal() -> String {
    std::env::var("USER")
        .ok()
        .filter(|user| !user.trim().is_empty())
        .unwrap_or_else(|| "operator".to_string())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use clap::Parser;
    use roko_agent::dispatcher::{HandlerResolver, ToolDispatcher};
    use roko_agent::{ReleasedToolControl, SafetyLayer, ToolPermissionPolicy};
    use roko_core::tool::{
        ToolCall, ToolCategory, ToolContext, ToolDef, ToolError, ToolHandler, ToolPermission,
        ToolRegistry, ToolResult, ToolResultContent, VecToolRegistry,
    };

    use super::*;

    /// `roko safety …` as clap parses it.
    #[derive(Debug, Parser)]
    struct Safety {
        #[command(subcommand)]
        cmd: SafetyCmd,
    }

    /// Run `roko safety <args>`: its exit code, stdout and stderr.
    fn safety(args: &[&str], json: bool) -> (i32, String, String) {
        let cmd = Safety::try_parse_from(std::iter::once("safety").chain(args.iter().copied()))
            .unwrap_or_else(|error| panic!("{args:?}: {error}"))
            .cmd;
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = run(
            cmd,
            json,
            || panic!("the test passes --workdir"),
            &mut out,
            &mut err,
        )
        .expect("roko safety");
        let text = |bytes: Vec<u8>| String::from_utf8(bytes).expect("utf-8");
        (code, text(out), text(err))
    }

    /// A tool whose first result is structured but not JSON, which the
    /// tool-result immune boundary withholds and answers by isolating the
    /// tool; later calls find what they look for.
    struct Lookup {
        calls: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl ToolHandler for Lookup {
        fn name(&self) -> &str {
            "lookup"
        }

        async fn execute(&self, _call: ToolCall, _ctx: &ToolContext) -> ToolResult {
            if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
                ToolResult::Ok {
                    content: vec![ToolResultContent::text("{not-json")],
                    is_structured: true,
                    artifacts: Vec::new(),
                }
            } else {
                ToolResult::text("found it")
            }
        }
    }

    /// gap-a2787f: the immune boundary isolates a tool in a workspace for
    /// good, and nothing listed or released it. `roko safety controls`
    /// lists the isolated tool, `roko safety release --tool --scope`
    /// releases it with an audit line naming the result that triggered it,
    /// and the tool's next call runs.
    #[tokio::test]
    async fn safety_lists_and_releases_a_tool_control() {
        let workspace = tempfile::tempdir().expect("workspace");
        let workdir = workspace.path().to_str().expect("utf-8 path");
        let calls = Arc::new(AtomicUsize::new(0));
        let handler: Arc<dyn ToolHandler> = Arc::new(Lookup {
            calls: Arc::clone(&calls),
        });
        let resolver: Arc<dyn HandlerResolver> =
            Arc::new(move |name: &str| (name == "lookup").then(|| Arc::clone(&handler)));
        let def = ToolDef::new(
            "lookup",
            "Look something up",
            ToolCategory::Read,
            ToolPermission::read_only(),
        );
        let registry: Arc<dyn ToolRegistry> = Arc::new(VecToolRegistry::from_tools(vec![def]));
        let safety_layer = SafetyLayer::with_defaults()
            .with_tool_permission_policy(ToolPermissionPolicy::DenyExplicit, Vec::new());
        let dispatcher = ToolDispatcher::new(registry, resolver).with_safety(safety_layer);
        let context = ToolContext::testing(workspace.path());
        let call = |id: &str| ToolCall::new(id, "lookup", serde_json::json!({}));

        let withheld = dispatcher.dispatch(call("first"), &context).await;
        let refused = dispatcher.dispatch(call("second"), &context).await;

        assert!(
            matches!(&withheld, ToolResult::Err(ToolError::PermissionDenied(why))
                if why.contains("denied by immune boundary")),
            "{withheld:?}"
        );
        let ToolResult::Err(ToolError::PermissionDenied(why)) = &refused else {
            panic!("the isolated tool is refused: {refused:?}");
        };
        assert!(
            why.contains(
                "`roko safety controls` lists it, and `roko safety release --tool lookup \
                 --scope builtin:local --reason <why>` releases it"
            ),
            "{why}"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        let (code, listed, _) = safety(&["controls", "--workdir", workdir], false);
        assert_eq!(code, EXIT_SUCCESS);
        assert!(
            listed.contains("tool lookup  scope=builtin:local  state=isolated  issued="),
            "{listed}"
        );
        let (_, listed_json, _) = safety(&["controls", "--workdir", workdir], true);
        let report: serde_json::Value = serde_json::from_str(&listed_json).expect("json");
        let tool_controls: Vec<ToolControl> =
            serde_json::from_value(report["tool_controls"].clone()).expect("tool controls");
        assert_eq!(tool_controls.len(), 1, "{listed_json}");
        let isolated = &tool_controls[0];
        assert_eq!(isolated.control, ToolControlState::Isolated);
        assert!(
            listed.contains(&format!("output={}", isolated.triggering_output.to_hex())),
            "{listed}"
        );

        let release = [
            "release",
            "--tool",
            "lookup",
            "--scope",
            "builtin:local",
            "--reason",
            "a false positive",
            "--by",
            "reviewer",
            "--workdir",
            workdir,
        ];
        let (code, released, _) = safety(&release, false);

        assert_eq!(code, EXIT_SUCCESS, "{released}");
        assert!(
            released.contains("released the isolated control of tool lookup (builtin:local)"),
            "{released}"
        );
        let audit_path = roko_agent::tool_control_releases_path(workspace.path());
        let audit = std::fs::read_to_string(&audit_path).expect("audit file");
        let lines: Vec<ReleasedToolControl> = audit
            .lines()
            .map(|line| serde_json::from_str(line).expect("audit line"))
            .collect();
        assert_eq!(lines.len(), 1, "{audit}");
        assert_eq!(lines[0].tool_name, "lookup");
        assert_eq!(lines[0].source_scope, "builtin:local");
        assert_eq!(lines[0].state, "isolated");
        assert_eq!(lines[0].triggering_output, isolated.triggering_output);
        assert_eq!(lines[0].issued_at_ms, isolated.issued_at_ms);
        assert_eq!(lines[0].by, "reviewer");
        assert_eq!(lines[0].reason, "a false positive");
        let (_, listed_after, _) = safety(&["controls", "--workdir", workdir], false);
        assert_eq!(listed_after, "no isolation controls\n");

        let next = dispatcher.dispatch(call("after-release"), &context).await;

        assert!(
            matches!(&next, ToolResult::Ok { .. }),
            "the released tool's next call runs: {next:?}"
        );
        assert!(next.text_content().contains("found it"), "{next:?}");
        assert_eq!(calls.load(Ordering::SeqCst), 2);

        // Nothing is left to release: it says so, fails and writes no line.
        let (code, _, err) = safety(&release, false);
        assert_eq!(code, EXIT_FAILURE);
        assert!(
            err.contains("no control for tool lookup (builtin:local)"),
            "{err}"
        );
        assert_eq!(
            std::fs::read_to_string(&audit_path).expect("audit file"),
            audit
        );
        // A tool release names its scope, and is not an agent release too.
        for args in [
            &["release", "--tool", "lookup", "--reason", "why"][..],
            &[
                "release",
                "live-a/cli",
                "--tool",
                "lookup",
                "--scope",
                "builtin:local",
                "--reason",
                "why",
            ][..],
        ] {
            let parsed =
                Safety::try_parse_from(std::iter::once("safety").chain(args.iter().copied()));
            assert!(parsed.is_err(), "{args:?} parsed");
        }
    }

    /// An agent's isolation control is still listed and released as before,
    /// now beside the tool controls.
    #[test]
    fn safety_still_lists_and_releases_an_agent_control() {
        let workspace = tempfile::tempdir().expect("workspace");
        let workdir = workspace.path().to_str().expect("utf-8 path");
        roko_agent::isolate_agent(Path::new(workdir), "live-a/cli", "operator_isolation")
            .expect("seed an isolation control");

        let (code, listed, _) = safety(&["controls", "--workdir", workdir], false);
        assert_eq!(code, EXIT_SUCCESS);
        assert!(
            listed.contains("agent live-a/cli  state=isolated  reason=operator_isolation"),
            "{listed}"
        );

        let (code, released, _) = safety(
            &[
                "release",
                "live-a/cli",
                "--reason",
                "blank answer",
                "--by",
                "reviewer",
                "--workdir",
                workdir,
            ],
            false,
        );
        assert_eq!(code, EXIT_SUCCESS, "{released}");
        assert!(
            released.contains("released isolation control"),
            "{released}"
        );
        assert!(
            roko_agent::list_agent_controls(workspace.path())
                .expect("controls")
                .is_empty()
        );
    }
}
