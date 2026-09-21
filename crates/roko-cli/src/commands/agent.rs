//! agent command handlers.

use crate::*;
use roko_cli::resolved_overrides::{AgentChatInput, AgentServeInput, ResolvedExecutionOverrides};

pub(crate) async fn cmd_agent(cli: &Cli, cmd: AgentCmd) -> Result<i32> {
    let workdir = resolve_workdir(cli);
    prepare_runtime_hooks(&workdir, cli.quiet);

    // Managed agent lifecycle operations write manifests, process registry,
    // and runtime state. Serialize those mutations with plan/run/serve.
    let _workspace_lock = match &cmd {
        AgentCmd::Create { workdir, .. }
        | AgentCmd::Delete { workdir, .. }
        | AgentCmd::Start { workdir, .. }
        | AgentCmd::Stop { workdir, .. } => {
            let workdir = workdir.clone().unwrap_or_else(|| resolve_workdir(cli));
            Some(roko_cli::workspace_lock::acquire_workspace_lock(
                &workdir.join(".roko"),
            )?)
        }
        _ => None,
    };

    // Build resolved overrides for serve/chat surfaces (#305).
    let flags = global_cli_flags(cli);
    let overrides = match &cmd {
        AgentCmd::Serve(args) => Some(ResolvedExecutionOverrides::for_agent_serve(
            &flags,
            &AgentServeInput {
                allow_stub_cognitive_loop: args.allow_stub_cognitive_loop,
            },
        )),
        AgentCmd::Chat { provider, text, .. } => Some(ResolvedExecutionOverrides::for_agent_chat(
            &flags,
            &AgentChatInput {
                provider: provider.clone(),
                text: *text,
            },
        )),
        _ => None,
    };

    // P2-FLG-1: Propagate the global --json flag into per-subcommand json fields
    // so that `roko --json agent list` is equivalent to `roko agent list --json`.
    let cmd = if cli.json {
        match cmd {
            AgentCmd::List { workdir, name, .. } => AgentCmd::List {
                workdir,
                json: true,
                name,
            },
            AgentCmd::Status { name, workdir, .. } => AgentCmd::Status {
                name,
                workdir,
                json: true,
            },
            other => other,
        }
    } else {
        cmd
    };

    agent_serve::run(cmd, overrides.as_ref()).await?;
    Ok(EXIT_SUCCESS)
}
