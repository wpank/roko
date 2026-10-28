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
//! Only a person releases a control, after reviewing it (find-d05b5e). The
//! default bash policy refuses `roko safety release` in an agent's shell
//! before it runs (`roko_agent::safety::bash`), and so does the Claude CLI
//! guard. The command itself refuses to run in a process roko started for an
//! agent or a check, which it tells by the variables roko sets on them
//! ([`roko_started_markers`]), `ROKO_AGENT_ID` on every agent process and
//! every command roko's own tools run for one among them (gap-31e8a7): a
//! script an agent writes, which no bash rule reads, inherits that. And its
//! audit line says how the release came in: through this command, where
//! the name in `by` came from (`--by`, `$USER`), and whether its standard
//! input was a terminal ([`roko_agent::ReleaseOrigin`]).
//!
//! The commands live in the library, as `diagnose` does, so its tests run
//! them; the `roko` binary re-exports the module.

use std::ffi::OsString;
use std::io::{IsTerminal as _, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use clap::Subcommand;
use roko_agent::claude_cli_agent::ISOLATION_ENV;
use roko_agent::{PrincipalSource, ReleaseChannel, ReleaseOrigin, ToolControl, ToolControlState};
use roko_core::child_env::AGENT_ID_ENV;
use roko_graph::cells::shell_exec::CELL_INPUT_ENV;

use crate::effects_apply::EFFECT_FILE_ENV;
use crate::exit_codes::{EXIT_FAILURE, EXIT_SUCCESS};
use crate::runner::agent_stream::PROVIDER_CONFIG_DIR_PREFIX;
use crate::runner::extension_loader::TOOL_ARGS_ENV;
use crate::runner::gate_dispatch::GATE_RUNG_ENV;

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
    /// and --scope), and record who did it, how and why. Only a person
    /// releases a control: the command refuses to run in a process roko
    /// started for an agent or a check.
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
        /// Who releases it (default: $USER, else "operator"; the audit line
        /// records which).
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
        &Caller::current(),
        &mut std::io::stdout().lock(),
        &mut std::io::stderr().lock(),
    )
}

/// The process a `roko safety` command runs in, as much of it as a release
/// looks at: its environment, and whether its standard input is a terminal.
#[derive(Clone, Debug, Default)]
struct Caller {
    env: Vec<(String, OsString)>,
    terminal: bool,
}

impl Caller {
    /// This process.
    fn current() -> Self {
        Self {
            env: roko_core::child_env::process_env(),
            terminal: std::io::stdin().is_terminal(),
        }
    }

    /// The releasing principal: `--by` when given, else `$USER`, else
    /// `operator`, and which of them it is.
    fn principal(&self, by: Option<String>) -> (String, PrincipalSource) {
        if let Some(by) = by {
            return (by, PrincipalSource::Flag);
        }
        let user = self
            .env
            .iter()
            .rev()
            .find(|(name, _)| name == "USER")
            .and_then(|(_, value)| value.to_str())
            .filter(|user| !user.trim().is_empty());
        match user {
            Some(user) => (user.to_string(), PrincipalSource::UserEnv),
            None => ("operator".to_string(), PrincipalSource::Fallback),
        }
    }
}

/// [`cmd_safety`] for `caller`, printing to `out` and `err`.
fn run(
    cmd: SafetyCmd,
    json: bool,
    default_workdir: impl FnOnce() -> PathBuf,
    caller: &Caller,
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
            // find-d05b5e: refuse before anything is read or written.
            if let Some(marker) = roko_started_markers(&caller.env).into_iter().next() {
                refuse_release(&marker, json, out, err)?;
                return Ok(EXIT_FAILURE);
            }
            let workdir = workdir.unwrap_or_else(default_workdir);
            let (by, by_source) = caller.principal(by);
            let via = ReleaseOrigin {
                channel: ReleaseChannel::Cli,
                by_source,
                terminal: Some(caller.terminal),
            };
            let released = match (tool, scope) {
                (Some(tool), Some(scope)) => {
                    release_tool(&workdir, &tool, &scope, &by, &reason, via, json, out)?
                }
                // clap requires an agent id when --tool is absent.
                _ => release_agent(
                    &workdir,
                    &agent_id.unwrap_or_default(),
                    &by,
                    &reason,
                    via,
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
        "a person releases one after review: roko safety release <agent> --reason <why>, or roko \
         safety release --tool <tool> --scope <scope> --reason <why>"
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
    via: ReleaseOrigin,
    json: bool,
    out: &mut impl Write,
) -> Result<std::result::Result<(), String>> {
    let released = roko_agent::release_tool_control(workdir, tool, scope, by, reason, via)
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
    via: ReleaseOrigin,
    json: bool,
    out: &mut impl Write,
) -> Result<std::result::Result<(), String>> {
    let released = roko_agent::release_agent_control(workdir, agent_id, by, reason, via)
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

/// A sign, in a process's environment, that roko started the process for
/// an agent or a check ([`roko_started_markers`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RokoMarker {
    /// The variables that make up the sign, as the environment names them.
    pub variables: Vec<String>,
    /// What roko sets them on.
    pub set_on: &'static str,
}

/// Variables roko sets on one kind of process it starts, beside that kind.
const ROKO_PROCESS_VARIABLES: [(&[&str], &str); 5] = [
    (
        &[AGENT_ID_ENV],
        "every agent roko runs and every command its own tools run for one",
    ),
    (
        &[TOOL_ARGS_ENV],
        "the command of an extension tool an agent calls",
    ),
    (
        &GATE_RUNG_ENV,
        "a gate rung's commands and the tests they run",
    ),
    (&[EFFECT_FILE_ENV], "an applied effect's receipt rungs"),
    (&[CELL_INPUT_ENV], "a Graph shell cell's command"),
];

/// The signs in `env` that roko started the process for an agent or a
/// check, which `roko safety release` refuses to run under (find-d05b5e):
/// no such process is a person. Each is a variable roko sets on those
/// processes, and their children inherit:
///
/// - every pair of [`ISOLATION_ENV`] (`CLAUDE_CODE_DISABLE_AUTO_MEMORY=1`,
///   `CLAUDE_CODE_ADDITIONAL_DIRECTORIES_CLAUDE_MD=1`), on the Claude Code
///   agents roko runs (plan runs, `roko run`, `roko chat`), and so on each
///   command their Bash tool runs; one of the two alone, as a person may set
///   in their own shell, is no sign;
/// - a variable naming a file in a `roko-cli-provider-*` directory, the
///   provider config roko writes for one CLI agent's run (Gemini's
///   `GEMINI_CLI_SYSTEM_SETTINGS_PATH`);
/// - [`AGENT_ID_ENV`] (`ROKO_AGENT_ID`), whatever its value, on every agent
///   roko runs (the Claude, Codex, Gemini, Cursor, Hermes and OpenClaw CLI
///   agents) and on every command its own tools run for one (`bash`,
///   `run_tests`, ACP's `bash`), which the API-model agents' shells are, and
///   so on a script such an agent writes and runs (gap-31e8a7);
/// - `ROKO_TOOL_ARGS`, on the command of an extension tool an agent calls;
/// - `ROKO_GATE_PLAN_ID`, `ROKO_GATE_TASK_ID`, `ROKO_GATE_RUNG` and
///   `ROKO_GATE_ATTEMPT_SENTINEL`, on a gate rung's commands and the tests
///   they run, agent-written tests among them;
/// - [`EFFECT_FILE_ENV`] (`ROKO_EFFECT_FILE`), on an applied effect's
///   receipt rungs;
/// - [`CELL_INPUT_ENV`] (`ROKO_CELL_INPUT`), on a Graph shell cell's
///   command.
///
/// An agent can unset a variable, so a sign is no boundary against an
/// agent that hides: it keeps roko from offering the release to its agents
/// and checks, and from recording one of theirs under a person's name. The
/// bash policy (`roko_agent::safety::bash`) and the Claude CLI guard refuse
/// a command whose words run the release before it runs.
#[must_use]
pub fn roko_started_markers(env: &[(String, OsString)]) -> Vec<RokoMarker> {
    let value = |name: &str| {
        env.iter()
            .rev()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value)
    };
    let mut markers = Vec::new();
    if ISOLATION_ENV
        .iter()
        .all(|(name, expected)| value(name).is_some_and(|found| found.to_str() == Some(*expected)))
    {
        markers.push(RokoMarker {
            variables: ISOLATION_ENV
                .iter()
                .map(|(name, _)| (*name).to_string())
                .collect(),
            set_on: "the Claude Code agents roko runs",
        });
    }
    for (name, path) in env {
        let in_provider_config_dir = Path::new(path).components().any(|component| {
            component
                .as_os_str()
                .to_string_lossy()
                .starts_with(PROVIDER_CONFIG_DIR_PREFIX)
        });
        if in_provider_config_dir {
            markers.push(RokoMarker {
                variables: vec![name.clone()],
                set_on: "a CLI agent whose provider config roko writes for its run",
            });
        }
    }
    for (names, set_on) in ROKO_PROCESS_VARIABLES {
        let present: Vec<String> = names
            .iter()
            .filter(|name| value(name).is_some())
            .map(|name| (*name).to_string())
            .collect();
        if !present.is_empty() {
            markers.push(RokoMarker {
                variables: present,
                set_on,
            });
        }
    }
    markers
}

/// Say why `roko safety release` does not run here: `marker` says roko
/// started this process for an agent or a check, and only a person
/// releases a control.
fn refuse_release(
    marker: &RokoMarker,
    json: bool,
    out: &mut impl Write,
    err: &mut impl Write,
) -> Result<()> {
    let message = format!(
        "refused: roko safety release does not run in a process roko started for an agent or a \
         check, and this one has {} (set on {}). Only a person releases an immune control, after \
         reviewing it: from their own terminal, or for an agent's control through roko serve \
         (POST /api/safety/controls/<agent>/release, which needs config-edit rights). An agent \
         leaves the control in force and says in its hand-off which one stopped it.",
        marker.variables.join(" and "),
        marker.set_on
    );
    if json {
        let report = serde_json::json!({ "refused": message, "variables": marker.variables });
        writeln!(out, "{}", serde_json::to_string_pretty(&report)?)?;
    } else {
        writeln!(err, "{message}")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use clap::Parser;
    use roko_agent::dispatcher::{HandlerResolver, ToolDispatcher};
    use roko_agent::{ReleasedControl, ReleasedToolControl, SafetyLayer, ToolPermissionPolicy};
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

    /// A caller whose environment holds `vars`, with a terminal or not.
    fn caller(vars: &[(&str, &str)], terminal: bool) -> Caller {
        Caller {
            env: vars
                .iter()
                .map(|(name, value)| ((*name).to_string(), OsString::from(value)))
                .collect(),
            terminal,
        }
    }

    /// A person at their terminal, logged in as `will`.
    fn person() -> Caller {
        caller(&[("USER", "will")], true)
    }

    /// Run `roko safety <args>` as [`person`]: its exit code, stdout and
    /// stderr.
    fn safety(args: &[&str], json: bool) -> (i32, String, String) {
        safety_as(&person(), args, json)
    }

    /// Run `roko safety <args>` as `caller`: its exit code, stdout and
    /// stderr.
    fn safety_as(caller: &Caller, args: &[&str], json: bool) -> (i32, String, String) {
        let cmd = Safety::try_parse_from(std::iter::once("safety").chain(args.iter().copied()))
            .unwrap_or_else(|error| panic!("{args:?}: {error}"))
            .cmd;
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = run(
            cmd,
            json,
            || panic!("the test passes --workdir"),
            caller,
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
        assert_eq!(
            lines[0].via,
            Some(ReleaseOrigin {
                channel: ReleaseChannel::Cli,
                by_source: PrincipalSource::Flag,
                terminal: Some(true),
            })
        );
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

    /// A `bash` handler that keeps each command it is handed instead of
    /// running it.
    struct RecordingBash {
        ran: Arc<std::sync::Mutex<Vec<String>>>,
    }

    #[async_trait::async_trait]
    impl ToolHandler for RecordingBash {
        fn name(&self) -> &str {
            "bash"
        }

        async fn execute(&self, call: ToolCall, _ctx: &ToolContext) -> ToolResult {
            let command = call.arguments["command"].as_str().unwrap_or_default();
            self.ran.lock().expect("lock").push(command.to_string());
            ToolResult::text("ran")
        }
    }

    /// find-d05b5e: an agent's own shell does not lift an immune control. The
    /// shell lists the controls, but a `bash` call that runs
    /// `roko safety release`, however it is spelled, is refused by the
    /// dispatcher's safety layer before the command runs, and says why; the
    /// control stays in force. (Refusals count as failures, and the default
    /// contract stops an agent after three in a row, so the listing goes
    /// first.)
    #[tokio::test]
    async fn an_agents_bash_call_that_runs_the_release_is_refused() {
        let workspace = tempfile::tempdir().expect("workspace");
        let workdir = workspace.path().to_str().expect("utf-8 path");
        roko_agent::isolate_agent(workspace.path(), "live-a/cli", "operator_isolation")
            .expect("seed an isolation control");
        let ran = Arc::new(std::sync::Mutex::new(Vec::new()));
        let handler: Arc<dyn ToolHandler> = Arc::new(RecordingBash {
            ran: Arc::clone(&ran),
        });
        let resolver: Arc<dyn HandlerResolver> =
            Arc::new(move |name: &str| (name == "bash").then(|| Arc::clone(&handler)));
        let def = ToolDef::new(
            "bash",
            "Run a shell command",
            ToolCategory::Exec,
            ToolPermission::executes(),
        );
        let registry: Arc<dyn ToolRegistry> = Arc::new(VecToolRegistry::from_tools(vec![def]));
        let safety_layer = SafetyLayer::with_defaults()
            .with_tool_permission_policy(ToolPermissionPolicy::DenyExplicit, Vec::new());
        let dispatcher = ToolDispatcher::new(registry, resolver).with_safety(safety_layer);
        let context = ToolContext::testing(workspace.path());
        let bash = |id: &str, command: &str| {
            ToolCall::new(id, "bash", serde_json::json!({ "command": command }))
        };
        let listed = format!("roko safety controls --workdir {workdir}");
        let result = dispatcher.dispatch(bash("list", &listed), &context).await;
        assert!(matches!(result, ToolResult::Ok { .. }), "{result:?}");

        for command in [
            format!("roko safety release live-a/cli --reason 'looks fine' --workdir {workdir}"),
            format!("cd {workdir} && ../../target/debug/roko --json safety release live-a/cli"),
            "cargo run -q -p roko-cli -- safety release --tool t --scope builtin:local".to_string(),
            "bash -c \"$(printf x); roko safety release live-a/cli --reason ok\"".to_string(),
        ] {
            let refused = dispatcher
                .dispatch(bash("release", &command), &context)
                .await;
            assert!(
                matches!(&refused, ToolResult::Err(ToolError::CommandNotAllowed(why))
                    if why.contains("`roko safety release`")
                        && why.contains("only a person releases an immune control")),
                "{command}: {refused:?}"
            );
        }
        assert_eq!(
            *ran.lock().expect("lock"),
            vec![listed],
            "only the listing ran"
        );
        assert_eq!(
            roko_agent::list_agent_controls(workspace.path())
                .expect("controls")
                .len(),
            1,
            "the control stays in force"
        );
    }

    /// gap-31e8a7: an API-model agent's shell is roko's own `bash` tool, and
    /// a script the agent writes there and runs is out of the bash policy's
    /// sight. The tool marks its command with `ROKO_AGENT_ID`, the script
    /// inherits the mark, and the release the script attempts refuses under
    /// it: it reads and writes nothing, and the control stays in force.
    ///
    /// The library's tests have no `roko` binary, so the `roko` the script
    /// finds is a stand-in that records the arguments and environment it
    /// was started with, and the release command runs here with exactly
    /// those (`tests/safety_cli.rs` runs the binary under the mark).
    #[cfg(unix)]
    #[tokio::test]
    async fn a_release_from_a_tool_command_is_refused() {
        use std::os::unix::fs::PermissionsExt as _;

        use roko_core::tool::CorrelationEnvelope;

        let workspace = tempfile::tempdir().expect("workspace");
        let workdir = workspace.path().to_str().expect("utf-8 path");
        roko_agent::isolate_agent(workspace.path(), "live-a/cli", "operator_isolation")
            .expect("seed an isolation control");
        let bin = workspace.path().join("bin");
        std::fs::create_dir(&bin).expect("bin directory");
        let stand_in = bin.join("roko");
        std::fs::write(
            &stand_in,
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$(dirname \"$0\")/args\"\n\
             env > \"$(dirname \"$0\")/env\"\n",
        )
        .expect("write the stand-in");
        std::fs::set_permissions(&stand_in, std::fs::Permissions::from_mode(0o755))
            .expect("make the stand-in executable");

        // roko's own `bash` tool behind the dispatcher's default safety
        // layer, called with the agent id a plan run's dispatch gives an
        // API-model agent's tool calls.
        let handler: Arc<dyn ToolHandler> = Arc::new(roko_std::tool::builtin::bash::Handler);
        let resolver: Arc<dyn HandlerResolver> =
            Arc::new(move |name: &str| (name == "bash").then(|| Arc::clone(&handler)));
        let registry: Arc<dyn ToolRegistry> = Arc::new(VecToolRegistry::from_tools(vec![
            roko_std::tool::builtin::bash::tool_def(),
        ]));
        let safety_layer = SafetyLayer::with_defaults()
            .with_tool_permission_policy(ToolPermissionPolicy::DenyExplicit, Vec::new());
        let dispatcher = ToolDispatcher::new(registry, resolver).with_safety(safety_layer);
        let context =
            ToolContext::testing(workspace.path()).with_correlation(CorrelationEnvelope {
                agent_id: "live-b/glm".to_string(),
                ..CorrelationEnvelope::empty()
            });
        let bash = |id: &str, command: &str| {
            ToolCall::new(id, "bash", serde_json::json!({ "command": command }))
        };

        // The agent writes a script that runs the release by a name no shell
        // word spells, with its own `roko` first on the PATH, and runs it.
        let write = format!(
            "printf '%s\\n' 'r=ro' 'PATH=\"$PWD/bin:$PATH\"' '\"${{r}}ko\" safety release \
             live-a/cli --reason fine --workdir {workdir}' > release.sh"
        );
        for (id, command) in [("write", write.as_str()), ("run", "sh release.sh")] {
            let ran = dispatcher.dispatch(bash(id, command), &context).await;
            assert!(matches!(ran, ToolResult::Ok { .. }), "{command}: {ran:?}");
        }

        let recorded = |name: &str| std::fs::read_to_string(bin.join(name)).expect("it ran");
        let args = recorded("args");
        let args: Vec<&str> = args.lines().collect();
        assert_eq!(args.get(..2), Some(&["safety", "release"][..]), "{args:?}");
        let env: Vec<(String, OsString)> = recorded("env")
            .lines()
            .filter_map(|line| line.split_once('='))
            .map(|(name, value)| (name.to_string(), OsString::from(value)))
            .collect();
        assert!(
            env.iter()
                .any(|(name, value)| name == AGENT_ID_ENV && value == "live-b/glm"),
            "{env:?}"
        );
        let script = Caller {
            env,
            terminal: false,
        };

        let (code, out, err) = safety_as(&script, &args[1..], false);

        assert_eq!(code, EXIT_FAILURE, "{out}{err}");
        assert!(out.is_empty(), "{out}");
        assert!(
            err.contains(
                "this one has ROKO_AGENT_ID (set on every agent roko runs and every command its \
                 own tools run for one)"
            ) && err.contains("Only a person releases an immune control"),
            "{err}"
        );
        assert_eq!(
            roko_agent::list_agent_controls(workspace.path())
                .expect("controls")
                .len(),
            1,
            "the control stays in force"
        );
        assert!(!roko_agent::agent_control_releases_path(workspace.path()).exists());
    }

    /// find-d05b5e: `roko safety release` does not run in a process roko
    /// started for an agent or a check. Under each variable roko sets on one
    /// it refuses, names the variable and what a person does instead, and
    /// reads and writes nothing, whatever `--by` says and with a terminal
    /// attached. One of the Claude pair alone, as a person may set in their
    /// own shell, does not stop them.
    #[test]
    fn release_refuses_in_an_agent_environment() {
        let workspace = tempfile::tempdir().expect("workspace");
        let workdir = workspace.path().to_str().expect("utf-8 path");
        roko_agent::isolate_agent(workspace.path(), "live-a/cli", "operator_isolation")
            .expect("seed an isolation control");
        let provider_config = std::env::temp_dir()
            .join(format!("{PROVIDER_CONFIG_DIR_PREFIX}a1b2c3"))
            .join("settings.json");
        let provider_config = provider_config.to_str().expect("utf-8 path");
        let cases: [(&[(&str, &str)], &str); 7] = [
            (
                ISOLATION_ENV,
                "CLAUDE_CODE_DISABLE_AUTO_MEMORY and CLAUDE_CODE_ADDITIONAL_DIRECTORIES_CLAUDE_MD \
                 (set on the Claude Code agents roko runs)",
            ),
            (
                &[("GEMINI_CLI_SYSTEM_SETTINGS_PATH", provider_config)],
                "GEMINI_CLI_SYSTEM_SETTINGS_PATH (set on a CLI agent whose provider config roko \
                 writes for its run)",
            ),
            (
                &[("ROKO_AGENT_ID", "live-b/codex")],
                "ROKO_AGENT_ID (set on every agent roko runs and every command its own tools run \
                 for one)",
            ),
            (
                &[("ROKO_TOOL_ARGS", "{}")],
                "ROKO_TOOL_ARGS (set on the command of an extension tool an agent calls)",
            ),
            (
                &[("ROKO_GATE_RUNG", "2"), ("ROKO_GATE_TASK_ID", "T1")],
                "ROKO_GATE_TASK_ID and ROKO_GATE_RUNG (set on a gate rung's commands and the \
                 tests they run)",
            ),
            (
                &[("ROKO_EFFECT_FILE", "/tmp/effect.json")],
                "ROKO_EFFECT_FILE (set on an applied effect's receipt rungs)",
            ),
            (
                &[("ROKO_CELL_INPUT", "/tmp/input.json")],
                "ROKO_CELL_INPUT (set on a Graph shell cell's command)",
            ),
        ];
        let agent_release = [
            "release",
            "live-a/cli",
            "--reason",
            "fine",
            "--by",
            "will",
            "--workdir",
            workdir,
        ];
        let tool_release = [
            "release",
            "--tool",
            "lookup",
            "--scope",
            "builtin:local",
            "--reason",
            "fine",
            "--workdir",
            workdir,
        ];
        for (marker, named) in cases {
            let mut vars = vec![("USER", "will")];
            vars.extend_from_slice(marker);
            let agent = caller(&vars, true);
            for args in [&agent_release[..], &tool_release[..]] {
                let (code, out, err) = safety_as(&agent, args, false);
                assert_eq!(code, EXIT_FAILURE, "{marker:?} {args:?}: {out}{err}");
                assert!(out.is_empty(), "{out}");
                assert!(
                    err.starts_with(
                        "refused: roko safety release does not run in a process roko started \
                         for an agent or a check, and this one has "
                    ) && err.contains(named)
                        && err.contains("Only a person releases an immune control"),
                    "{marker:?}: {err}"
                );
            }
            let (code, out, _) = safety_as(&agent, &agent_release, true);
            assert_eq!(code, EXIT_FAILURE, "{out}");
            let report: serde_json::Value = serde_json::from_str(&out).expect("json");
            assert!(
                report["refused"]
                    .as_str()
                    .is_some_and(|why| why.contains(named)),
                "{out}"
            );
        }
        assert_eq!(
            roko_agent::list_agent_controls(workspace.path())
                .expect("controls")
                .len(),
            1,
            "the control stays in force"
        );
        assert!(!roko_agent::agent_control_releases_path(workspace.path()).exists());
        assert!(!roko_agent::tool_control_releases_path(workspace.path()).exists());

        let (name, value) = ISOLATION_ENV[0];
        let half = caller(&[("USER", "will"), (name, value)], true);
        let (code, out, err) = safety_as(&half, &agent_release, false);
        assert_eq!(code, EXIT_SUCCESS, "{out}{err}");
    }

    /// find-d05b5e: a person's release still works, and its audit line says
    /// how it came in: through the command, with the name from `$USER`,
    /// `--by` or neither, and with or without a terminal.
    #[test]
    fn a_persons_release_still_works_and_records_how() {
        let workspace = tempfile::tempdir().expect("workspace");
        let workdir = workspace.path().to_str().expect("utf-8 path");
        for agent in ["live-a/cli", "live-b/cli", "live-c/cli"] {
            roko_agent::isolate_agent(workspace.path(), agent, "operator_isolation")
                .expect("seed an isolation control");
        }
        let release = |agent: &str, by: Option<&str>, caller: &Caller| {
            let mut args = vec![
                "release",
                agent,
                "--reason",
                "reviewed",
                "--workdir",
                workdir,
            ];
            if let Some(by) = by {
                args.extend(["--by", by]);
            }
            let (code, out, err) = safety_as(caller, &args, false);
            assert_eq!(code, EXIT_SUCCESS, "{out}{err}");
            assert!(out.contains("released isolation control"), "{out}");
        };

        release("live-a/cli", None, &person());
        release(
            "live-b/cli",
            Some("reviewer"),
            &caller(&[("USER", "will")], false),
        );
        release("live-c/cli", None, &caller(&[], true));

        let audit =
            std::fs::read_to_string(roko_agent::agent_control_releases_path(workspace.path()))
                .expect("audit file");
        let lines: Vec<ReleasedControl> = audit
            .lines()
            .map(|line| serde_json::from_str(line).expect("audit line"))
            .collect();
        let how: Vec<_> = lines
            .iter()
            .map(|line| (line.agent_id.as_str(), line.by.as_str(), line.via))
            .collect();
        let cli = |by_source, terminal| {
            Some(ReleaseOrigin {
                channel: ReleaseChannel::Cli,
                by_source,
                terminal: Some(terminal),
            })
        };
        assert_eq!(
            how,
            [
                ("live-a/cli", "will", cli(PrincipalSource::UserEnv, true)),
                ("live-b/cli", "reviewer", cli(PrincipalSource::Flag, false)),
                (
                    "live-c/cli",
                    "operator",
                    cli(PrincipalSource::Fallback, true)
                ),
            ]
        );
        assert!(
            roko_agent::list_agent_controls(workspace.path())
                .expect("controls")
                .is_empty()
        );
    }
}
